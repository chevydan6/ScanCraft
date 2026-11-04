//! Bounded recovery journal validation. Checks state/selection consistency and
//! pose disagreement, not optical correspondence identity or ICP optimality.
use super::{
    Error, Request,
    rows::{self, FrameOutcome, FrameStatus, Pose},
};
use crate::{bounded_json::Items, tracked_candidates::recovery::JOURNAL_LINE_BYTES};
use scancraft_calibration::stereo::RigidPose;
use scancraft_geometry::{
    session::recovery::{MAX_REFERENCES, REFERENCE_CADENCE},
    work::WorkControl,
};
use serde::Deserialize;
use std::fs::File;

#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    pub attempts: usize,
    pub selected_attempt: usize,
    pub recovered_from: Option<u64>,
    pub conflicting_references: bool,
    pub maximum_pose_disagreement_model_units: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Tracking {
    accepted: bool,
    reason: String,
    pose: Option<Pose>,
    rms_model_units: Option<f64>,
    source_overlap: f64,
    target_overlap: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Attempt {
    reference_frame: Option<u64>,
    retained: bool,
    positions_model_units: Items<[f64; 3], 512>,
    tracking: Result<Tracking, String>,
}
impl Attempt {
    fn accepted(&self) -> bool {
        self.tracking.as_ref().is_ok_and(|t| t.accepted)
    }
    fn validate(&self, request: &Request) -> Result<(), Error> {
        if self
            .positions_model_units
            .0
            .iter()
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err("nonfinite recovery observations".into());
        }
        match &self.tracking {
            Err(e) if e.is_empty() || e.len() > 4096 => return Err("invalid recovery error".into()),
            Ok(t) => {
                if self.positions_model_units.0.len() < 3
                    || t.reason.is_empty()
                    || t.reason.len() > 4096
                    || t.pose.is_some() != t.accepted
                    || t.rms_model_units.is_some_and(|v| !v.is_finite() || v < 0.)
                    || [t.source_overlap, t.target_overlap]
                        .into_iter()
                        .any(|v| !v.is_finite() || !(0. ..=1.).contains(&v))
                    || (t.accepted
                        && (t
                            .rms_model_units
                            .is_none_or(|v| v > request.tracking.max_rms_model_units)
                            || t.source_overlap < request.tracking.minimum_overlap
                            || t.target_overlap < request.tracking.minimum_overlap))
                {
                    return Err("invalid recovery tracking result".into());
                }
                if let Some(p) = &t.pose {
                    pose(p).validate()?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
fn pose(p: &Pose) -> RigidPose {
    RigidPose {
        rotation: p.rotation,
        translation_mm: p.translation_model_units,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    schema_version: u32,
    frame_id: u64,
    summary: Summary,
    attempts: Items<Attempt, 17>,
    retained_references: Items<u64, 16>,
}

pub(super) fn validate(
    file: &mut File,
    frames: &[FrameOutcome],
    request: &Request,
    work: &WorkControl,
) -> Result<(), Error> {
    let mut next = 0;
    let mut last_accepted = None;
    let mut bank = Vec::new();
    let mut accepted_count = 0usize;
    rows::read(file, 256, JOURNAL_LINE_BYTES, work, |record: Record| {
        let frame = frames.get(next).ok_or("extra recovery journal frame")?;
        let attempts = &record.attempts.0;
        let summary = &record.summary;
        if record.schema_version != 1
            || record.frame_id != frame.frame_id
            || frame.recovery.as_ref() != Some(summary)
            || attempts.is_empty()
            || summary.attempts != attempts.len()
            || summary.selected_attempt >= attempts.len()
            || !summary.maximum_pose_disagreement_model_units.is_finite()
            || summary.maximum_pose_disagreement_model_units < 0.
            || attempts[0].retained
            || attempts[0].reference_frame != last_accepted
        {
            return Err("invalid recovery journal frame or local reference".into());
        }
        for a in attempts {
            work.check()?;
            a.validate(request)?;
        }
        if attempts.len()
            != if attempts[0].accepted() {
                1
            } else {
                bank.len() + 1
            }
        {
            return Err("incomplete recovery trials".into());
        }
        for (a, id) in attempts.iter().skip(1).zip(&bank) {
            if !a.retained || a.reference_frame != Some(*id) {
                return Err("recovery reference bank mismatch".into());
            }
        }
        let passing: Vec<_> = attempts
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, a)| a.accepted())
            .collect();
        let mut maximum = 0.0_f64;
        for (i, (_, a)) in passing.iter().enumerate() {
            for (_, b) in &passing[i + 1..] {
                let pa = pose(a.tracking.as_ref().unwrap().pose.as_ref().unwrap());
                let pb = pose(b.tracking.as_ref().unwrap().pose.as_ref().unwrap());
                for p in a
                    .positions_model_units
                    .0
                    .iter()
                    .chain(&b.positions_model_units.0)
                {
                    work.check()?;
                    let x = pa.apply(*p);
                    let y = pb.apply(*p);
                    let d = (x[0] - y[0]).hypot(x[1] - y[1]).hypot(x[2] - y[2]);
                    if !d.is_finite() {
                        return Err("recovery pose disagreement overflow".into());
                    }
                    maximum = maximum.max(d);
                }
            }
        }
        let conflict = maximum > request.tracking.max_rms_model_units;
        let expected_selection = if conflict {
            0
        } else {
            passing.first().map_or(0, |(i, _)| *i)
        };
        if maximum != summary.maximum_pose_disagreement_model_units
            || conflict != summary.conflicting_references
            || expected_selection != summary.selected_attempt
            || summary.recovered_from
                != if expected_selection == 0 {
                    None
                } else {
                    attempts[expected_selection].reference_frame
                }
        {
            return Err("inconsistent recovery consensus or selection".into());
        }
        let selected = &attempts[summary.selected_attempt];
        if frame.tracking_points != selected.positions_model_units.0.len() {
            return Err("recovery selected observation count mismatch".into());
        }
        match &selected.tracking {
            Ok(t) => {
                if frame.accepted != Some(t.accepted)
                    || frame.reason.as_ref() != Some(&t.reason)
                    || frame.pose != t.pose
                    || frame.rms_model_units != t.rms_model_units
                    || frame.source_overlap != Some(t.source_overlap)
                    || frame.target_overlap != Some(t.target_overlap)
                {
                    return Err("selected recovery tracking and frame disagree".into());
                }
            }
            Err(e) => {
                let status = if frame.tracking_points < 3 {
                    FrameStatus::InsufficientTrackingObservations
                } else {
                    FrameStatus::TrackingError
                };
                if frame.status != status
                    || (status == FrameStatus::TrackingError && frame.reason.as_ref() != Some(e))
                {
                    return Err("selected recovery error and frame disagree".into());
                }
            }
        }
        if selected.accepted() {
            last_accepted = Some(frame.frame_id);
            accepted_count += 1;
            if accepted_count == 1 || accepted_count.is_multiple_of(REFERENCE_CADENCE) {
                if bank.len() == MAX_REFERENCES {
                    bank.remove(1);
                }
                bank.push(frame.frame_id);
            }
        }
        if record.retained_references.0 != bank {
            return Err("recovery retention history mismatch".into());
        }
        next += 1;
        Ok(())
    })?;
    if next != frames.len() {
        return Err("missing recovery journal frames".into());
    }
    Ok(())
}
