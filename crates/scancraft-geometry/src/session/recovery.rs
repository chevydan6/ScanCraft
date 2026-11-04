//! Bounded local recovery against session-owned accepted references.
//! This is not global registration or physical feature identification.
use super::{ScanSession, SessionFrame, SessionState};
use crate::{
    Cloud,
    tracking::{Tracker, TrackingUpdate},
    work::WorkControl,
};

pub const MAX_REFERENCES: usize = 16;
pub const REFERENCE_CADENCE: usize = 8;

#[derive(Clone)]
struct SavedReference {
    id: u64,
    tracker: Tracker,
}

#[derive(Clone, Default)]
pub(super) struct RecoveryState {
    references: Vec<SavedReference>,
    accepted: usize,
    attempted: usize,
    last_attempt: Option<u64>,
}

/// Borrowed reference for selecting this frame's observations. A callback cannot
/// mutate the stored tracker or supply an arbitrary world pose.
#[derive(Clone, Copy)]
pub struct Reference<'a> {
    pub frame_id: u64,
    pub positions: &'a [[f64; 3]],
}

#[derive(Clone, Debug)]
pub struct Attempt {
    pub reference_frame: Option<u64>,
    pub retained: bool,
    pub observations: Vec<[f64; 3]>,
    pub tracking: Result<TrackingUpdate, String>,
}

#[derive(Debug)]
pub struct Outcome {
    /// None when no valid tracking update exists, e.g. fewer than three points.
    pub frame: Option<SessionFrame>,
    pub attempts: Vec<Attempt>,
    pub selected_attempt: usize,
    pub recovered_from: Option<u64>,
    pub conflicting_references: bool,
    pub maximum_pose_disagreement: f64,
}

fn attempt(
    tracker: &Tracker,
    frame_id: u64,
    retained: bool,
    select: &mut impl FnMut(Option<Reference<'_>>, &WorkControl) -> Result<Vec<[f64; 3]>, String>,
    work: &WorkControl,
) -> Result<(Tracker, Attempt), String> {
    work.check()?;
    let reference_frame = tracker.last_accepted();
    let observations = select(
        reference_frame.map(|frame_id| Reference {
            frame_id,
            positions: tracker.reference_points(),
        }),
        work,
    )?;
    work.check()?;
    if observations.len() > 512 || observations.iter().flatten().any(|v| !v.is_finite()) {
        return Err("recovery selector requires at most512 finite observations".into());
    }
    let mut candidate = tracker.clone();
    let tracking = candidate.track(frame_id, &observations, work);
    // Cancellation is a transaction failure, never a recoverable tracking error.
    work.check()?;
    Ok((
        candidate,
        Attempt {
            reference_frame,
            retained,
            observations,
            tracking,
        },
    ))
}

fn disagreement(
    attempts: &[Attempt],
    accepted: &[(usize, Tracker)],
    work: &WorkControl,
) -> Result<f64, String> {
    let mut maximum = 0.0_f64;
    for (i, (a, _)) in accepted.iter().enumerate() {
        for (b, _) in &accepted[i + 1..] {
            let a = &attempts[*a];
            let b = &attempts[*b];
            let pa = a
                .tracking
                .as_ref()
                .unwrap()
                .world_from_frame
                .as_ref()
                .unwrap();
            let pb = b
                .tracking
                .as_ref()
                .unwrap()
                .world_from_frame
                .as_ref()
                .unwrap();
            for point in a.observations.iter().chain(&b.observations) {
                work.check()?;
                let x = pa.apply(*point);
                let y = pb.apply(*point);
                let distance = (x[0] - y[0]).hypot(x[1] - y[1]).hypot(x[2] - y[2]);
                if !distance.is_finite() {
                    return Err("recovery pose comparison overflow".into());
                }
                maximum = maximum.max(distance);
            }
        }
    }
    Ok(maximum)
}

impl ScanSession {
    pub fn retained_reference_ids(&self) -> Vec<u64> {
        self.recovery
            .as_ref()
            .map_or_else(Vec::new, |s| s.references.iter().map(|r| r.id).collect())
    }

    /// Start recovery mode on an empty running session, then use this method for
    /// every subsequent frame. Point-to-point only; no normals are inferred.
    ///
    /// The selector supplies current-frame raw positions conditional on a borrowed
    /// prior. It must preserve coordinate/observation provenance. It may be called
    /// up to17 times; external callback side effects cannot be rolled back here.
    /// All attempts remain in the returned report, including insufficient points.
    ///
    /// Evaluate local tracking first, then every retained snapshot on local failure.
    /// Passing references must agree within the configured RMS distance over their
    /// selected points; choose the oldest passing consistent reference. No reset.
    /// Tracker, keyframes, attempted-frame counter, fusion and history commit only
    /// after all fallible work succeeds. Even unavailable frames consume a monotonic
    /// ID/capacity slot; errors/cancellation consume neither. Original APIs cannot
    /// be mixed into this mode. No physical identity or accuracy is certified.
    pub fn process_with_reference_recovery(
        &mut self,
        frame_id: u64,
        surface: &Cloud,
        mut select: impl FnMut(Option<Reference<'_>>, &WorkControl) -> Result<Vec<[f64; 3]>, String>,
        work: &WorkControl,
    ) -> Result<Outcome, String> {
        work.check()?;
        if self.state != SessionState::Running
            || self.tracker.method().requires_normals()
            || (self.recovery.is_none() && !self.history.is_empty())
        {
            return Err(
                "recovery requires an empty or ongoing point-to-point recovery session".into(),
            );
        }
        let mut state = self.recovery.clone().unwrap_or_default();
        if state.attempted >= self.maximum_frames
            || state.last_attempt.is_some_and(|id| frame_id <= id)
        {
            return Err("recovery frame capacity or monotonic ID requirement violated".into());
        }
        if surface.points.len() > 100_000 {
            return Err("recovery surface exceeds100000 points".into());
        }
        surface.validate_with(work)?;
        let (mut candidate, local) = attempt(&self.tracker, frame_id, false, &mut select, work)?;
        let mut attempts = vec![local];
        let mut selected_attempt = 0;
        let mut recovered_from = None;
        let mut maximum_pose_disagreement = 0.;
        let mut conflicting_references = false;
        if !attempts[0].tracking.as_ref().is_ok_and(|u| u.accepted) {
            let mut accepted = Vec::new();
            for saved in &state.references {
                let (trial, outcome) = attempt(&saved.tracker, frame_id, true, &mut select, work)?;
                if outcome.tracking.as_ref().is_ok_and(|u| u.accepted) {
                    accepted.push((attempts.len(), trial));
                }
                attempts.push(outcome);
            }
            maximum_pose_disagreement = disagreement(&attempts, &accepted, work)?;
            conflicting_references = maximum_pose_disagreement > self.tracker.config().max_rms_mm;
            if !conflicting_references && !accepted.is_empty() {
                (selected_attempt, candidate) = accepted.remove(0);
                recovered_from = attempts[selected_attempt].reference_frame;
            }
        }
        let selected = &attempts[selected_attempt];
        if selected.tracking.as_ref().is_ok_and(|u| u.accepted) {
            state.accepted += 1;
            if state.accepted == 1 || state.accepted.is_multiple_of(REFERENCE_CADENCE) {
                if state.references.len() == MAX_REFERENCES {
                    state.references.remove(1);
                }
                state.references.push(SavedReference {
                    id: frame_id,
                    tracker: candidate.clone(),
                });
            }
        }
        state.attempted += 1;
        state.last_attempt = Some(frame_id);
        // Reserve history storage before fusion's commit; no fallible operation
        // follows successful integration. Bounded temporary state is already staged.
        if selected.tracking.is_ok() {
            self.history.try_reserve(1).map_err(|e| e.to_string())?;
        }
        work.check()?;
        let frame = match &selected.tracking {
            Ok(tracking) => {
                let fusion = if tracking.accepted && !surface.points.is_empty() {
                    Some(self.fusion.integrate_tracked(surface, tracking, work)?)
                } else {
                    work.check()?;
                    None
                };
                Some(SessionFrame {
                    tracking: tracking.clone(),
                    fusion,
                })
            }
            Err(_) => {
                work.check()?;
                None
            }
        };
        self.tracker = candidate;
        self.recovery = Some(state);
        if let Some(frame) = &frame {
            self.history.push(frame.clone());
        }
        Ok(Outcome {
            frame,
            attempts,
            selected_attempt,
            recovered_from,
            conflicting_references,
            maximum_pose_disagreement,
        })
    }
}

#[cfg(test)]
mod tests;
