//! Read-only timestamp association diagnostics, never automatic exposure pairing.
//! Raw clocks may be unrelated. A close host timestamp or equal PTS is not proof
//! that the sensors exposed together; no clock offset/frequency is estimated.
use crate::{Error, recording_playback::Playback};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::indexed::FrameTiming;
use serde::Serialize;
use std::path::Path;

const MAX_SAMPLES: usize = 100;
#[derive(Debug, Serialize)]
pub struct Candidate {
    pub left_index: usize,
    pub right_index: usize,
    pub left: FrameTiming,
    pub right: FrameTiming,
    pub host_absolute_delta_us: u64,
    /// False means right preceded left in the recorded host timestamp values.
    pub right_host_at_or_after_left: bool,
    pub raw_sensor_pts_equal: bool,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub scope: &'static str,
    pub left_frames: usize,
    pub right_frames: usize,
    pub max_host_delta_us: u64,
    pub unique_nearest_within_limit: usize,
    pub equidistant_nearest_within_limit: usize,
    pub outside_limit: usize,
    pub right_frames_used_by_multiple_left_candidates: usize,
    pub raw_pts_equal_unique_candidates: usize,
    pub candidate_samples: Vec<Candidate>,
    pub samples_truncated: bool,
    pub clock_domains_verified: bool,
    pub synchronization_verified: bool,
    pub physical_accuracy_qualified: bool,
}

/// Fully validates both recordings before inspecting historical timestamp values.
/// Requires expected Raptor IR interfaces and matching packed-GREY profiles. This
/// does not establish device identity, common clocks, exposure or calibration.
/// The frame limit is the existing million-entry index bound; output samples are
/// capped at 100. Inputs are guarded using the normal playback identity checks.
pub fn inspect(
    left_path: &Path,
    right_path: &Path,
    max_host_delta_us: u64,
    work: &WorkControl,
) -> Result<Report, Error> {
    let left = Playback::open(left_path, work)?;
    let right = Playback::open(right_path, work)?;
    let lp = left.profile();
    let rp = right.profile();
    if lp.interface != 0
        || rp.interface != 2
        || lp.fourcc != *b"GREY"
        || rp.fourcc != *b"GREY"
        || lp.width != rp.width
        || lp.height != rp.height
        || lp.fps != rp.fps
    {
        return Err("timing diagnostics require matching left interface 0 / right interface 2 GREY profiles".into());
    }
    let right_times: Vec<_> = right.timings().collect();
    let report = associate(left.timings(), &right_times, max_host_delta_us, work)?;
    left.check_source()?;
    right.check_source()?;
    work.check()?;
    Ok(report)
}

fn associate(
    left: impl ExactSizeIterator<Item = FrameTiming>,
    right: &[FrameTiming],
    limit: u64,
    work: &WorkControl,
) -> Result<Report, Error> {
    let mut report = Report {
        schema_version: 1,
        scope: "nearest recorded host values only; clocks/exposures unverified; no automatic reconstruction pairing",
        left_frames: left.len(),
        right_frames: right.len(),
        max_host_delta_us: limit,
        unique_nearest_within_limit: 0,
        equidistant_nearest_within_limit: 0,
        outside_limit: 0,
        right_frames_used_by_multiple_left_candidates: 0,
        raw_pts_equal_unique_candidates: 0,
        candidate_samples: Vec::new(),
        samples_truncated: false,
        clock_domains_verified: false,
        synchronization_verified: false,
        physical_accuracy_qualified: false,
    };
    // Recording validation requires strictly increasing host timestamps. Nearest
    // values can only be the insertion point or its predecessor. No quadratic
    // all-pairs scan and no one-to-one/greedy exposure assignment is performed.
    let mut usage = vec![0_u32; right.len()];
    for (left_index, l) in left.enumerate() {
        work.check()?;
        let at = right.partition_point(|r| r.host_timestamp_us < l.host_timestamp_us);
        let before = at.checked_sub(1);
        let after = (at < right.len()).then_some(at);
        let distance = |i: usize| right[i].host_timestamp_us.abs_diff(l.host_timestamp_us);
        let selected = match (before, after) {
            (Some(a), Some(b)) if distance(a) == distance(b) => {
                if distance(a) <= limit {
                    report.equidistant_nearest_within_limit += 1;
                } else {
                    report.outside_limit += 1;
                }
                continue;
            }
            (Some(a), Some(b)) => Some(if distance(a) < distance(b) { a } else { b }),
            (Some(a), None) | (None, Some(a)) => Some(a),
            (None, None) => None,
        };
        let Some(right_index) = selected.filter(|&i| distance(i) <= limit) else {
            report.outside_limit += 1;
            continue;
        };
        usage[right_index] += 1;
        report.unique_nearest_within_limit += 1;
        let r = right[right_index];
        let equal = l.sensor_pts == r.sensor_pts;
        report.raw_pts_equal_unique_candidates += usize::from(equal);
        if report.candidate_samples.len() < MAX_SAMPLES {
            report.candidate_samples.push(Candidate {
                left_index,
                right_index,
                left: l,
                right: r,
                host_absolute_delta_us: distance(right_index),
                right_host_at_or_after_left: r.host_timestamp_us >= l.host_timestamp_us,
                raw_sensor_pts_equal: equal,
            });
        }
    }
    report.right_frames_used_by_multiple_left_candidates = usage.iter().filter(|&&n| n > 1).count();
    report.samples_truncated = report.unique_nearest_within_limit > report.candidate_samples.len();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn times(values: &[u64]) -> Vec<FrameTiming> {
        values
            .iter()
            .enumerate()
            .map(|(i, &t)| FrameTiming {
                sequence: i as u32,
                host_timestamp_us: t,
                sensor_pts: i as u32,
            })
            .collect()
    }
    #[test]
    fn nearest_candidates_keep_ties_reuse_and_limits_explicit() {
        let left = times(&[1, 10, 11, 15, 20, 100]);
        let right = times(&[10, 20]);
        let report = associate(left.into_iter(), &right, 2, &WorkControl::default()).unwrap();
        assert_eq!(report.unique_nearest_within_limit, 3);
        assert_eq!(report.outside_limit, 3);
        assert_eq!(report.equidistant_nearest_within_limit, 0);
        assert_eq!(report.right_frames_used_by_multiple_left_candidates, 1);
        let report =
            associate(times(&[15]).into_iter(), &right, 5, &WorkControl::default()).unwrap();
        assert_eq!(report.equidistant_nearest_within_limit, 1);
        assert!(report.candidate_samples.is_empty());
        assert!(
            !report.clock_domains_verified
                && !report.synchronization_verified
                && !report.physical_accuracy_qualified
        );
    }
    #[test]
    fn endpoints_large_timestamps_and_sample_bound_do_not_wrap_or_invent_pairs() {
        let report = associate(
            times(&[0, u64::MAX]).into_iter(),
            &times(&[1]),
            u64::MAX,
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(
            report.candidate_samples[1].host_absolute_delta_us,
            u64::MAX - 1
        );
        assert!(!report.candidate_samples[1].right_host_at_or_after_left);
        let values: Vec<_> = (0..105).collect();
        let report = associate(
            times(&values).into_iter(),
            &times(&values),
            0,
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(report.unique_nearest_within_limit, 105);
        assert_eq!(report.raw_pts_equal_unique_candidates, 105);
        assert_eq!(report.candidate_samples.len(), 100);
        assert!(report.samples_truncated);
        assert_eq!(
            associate(times(&[1]).into_iter(), &[], 10, &WorkControl::default())
                .unwrap()
                .outside_limit,
            1
        );
        let work = WorkControl::default();
        work.cancel();
        assert!(associate(times(&[1]).into_iter(), &times(&[1]), 0, &work).is_err());
    }
    #[test]
    fn binary_search_association_matches_independent_exhaustive_distance_reference() {
        let left = times(&(0..500).map(|i| i * 7).collect::<Vec<_>>());
        let right = times(&(0..180).map(|i| i * 19 + i % 3).collect::<Vec<_>>());
        for limit in [0, 3, 20, u64::MAX] {
            let report =
                associate(left.iter().copied(), &right, limit, &WorkControl::default()).unwrap();
            let (mut unique, mut ties, mut outside) = (0, 0, 0);
            let mut usage = vec![0; right.len()];
            for (li, l) in left.iter().enumerate() {
                let distances: Vec<_> = right
                    .iter()
                    .map(|r| r.host_timestamp_us.abs_diff(l.host_timestamp_us))
                    .collect();
                let minimum = *distances.iter().min().unwrap();
                let winners: Vec<_> = distances
                    .iter()
                    .enumerate()
                    .filter(|&(_, &d)| d == minimum)
                    .map(|(i, _)| i)
                    .collect();
                if minimum > limit {
                    outside += 1;
                } else if winners.len() != 1 {
                    ties += 1;
                } else {
                    unique += 1;
                    usage[winners[0]] += 1;
                    if unique <= MAX_SAMPLES {
                        let sample = &report.candidate_samples[unique - 1];
                        assert_eq!(
                            (
                                sample.left_index,
                                sample.right_index,
                                sample.host_absolute_delta_us
                            ),
                            (li, winners[0], minimum)
                        );
                    }
                }
            }
            assert_eq!(
                (
                    report.unique_nearest_within_limit,
                    report.equidistant_nearest_within_limit,
                    report.outside_limit
                ),
                (unique, ties, outside)
            );
            assert_eq!(
                report.right_frames_used_by_multiple_left_candidates,
                usage.iter().filter(|&&n| n > 1).count()
            );
        }
    }
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "scancraft-timing-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn recording(&self, name: &str, interface: u8, host: &[u64]) -> std::path::PathBuf {
            let path = self.0.join(name);
            let profile = scancraft_core::StreamProfile {
                interface,
                width: 2,
                height: 2,
                fourcc: *b"GREY",
                fps: 50,
            };
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .unwrap();
            let mut writer = scancraft_recording::Writer::new(file, profile).unwrap();
            for (i, &timestamp) in host.iter().enumerate() {
                let mut metadata = vec![0; 16];
                metadata[10] = 6;
                metadata[11] = 4;
                metadata[12..16].copy_from_slice(&(100_u32 + i as u32).to_le_bytes());
                writer
                    .push(&scancraft_core::Frame {
                        sequence: i as u32,
                        host_timestamp_us: timestamp,
                        flags: 0,
                        metadata,
                        data: vec![30; 4],
                    })
                    .unwrap();
            }
            writer.finish().unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn full_recording_inspection_preserves_inputs_and_rejects_bad_suffix_and_profiles() {
        let fixture = Fixture::new();
        let l = fixture.recording("left.raptor", 0, &[100, 120, 140]);
        let r = fixture.recording("right.raptor", 2, &[101, 121, 141]);
        let original = [std::fs::read(&l).unwrap(), std::fs::read(&r).unwrap()];
        let report = inspect(&l, &r, 1, &WorkControl::default()).unwrap();
        assert_eq!(report.unique_nearest_within_limit, 3);
        assert_eq!(report.raw_pts_equal_unique_candidates, 3);
        assert_eq!(std::fs::read(&l).unwrap(), original[0]);
        assert_eq!(std::fs::read(&r).unwrap(), original[1]);
        let wrong = fixture.recording("wrong.raptor", 0, &[101, 121, 141]);
        assert!(inspect(&l, &wrong, 1, &WorkControl::default()).is_err());
        let work = WorkControl::default();
        work.cancel();
        assert!(inspect(&l, &r, 1, &work).is_err());
        let mut corrupt = original[1].clone();
        corrupt.pop();
        std::fs::write(&r, corrupt).unwrap();
        assert!(inspect(&l, &r, 1, &WorkControl::default()).is_err());
        assert_eq!(std::fs::read(&l).unwrap(), original[0]);
    }
}
