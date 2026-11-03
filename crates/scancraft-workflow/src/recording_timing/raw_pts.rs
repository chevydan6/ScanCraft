//! Exact raw-value intersections, not clock unwrapping or exposure association.
use crate::Error;
use scancraft_geometry::work::WorkControl;
use scancraft_recording::indexed::FrameTiming;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Comparison {
    pub scope: &'static str,
    /// Only values occurring exactly once in each complete recording qualify.
    pub unique_equal_value_pairs: usize,
    pub left_frames_without_equal_right_value: usize,
    /// A repeated value on either side with cross-recording matches is ambiguous.
    pub left_frames_with_ambiguous_equal_value: usize,
    pub unique_pairs_within_host_limit: usize,
    pub minimum_absolute_host_delta_us: Option<u64>,
    pub maximum_absolute_host_delta_us: Option<u64>,
}

pub(super) fn compare(
    left: impl ExactSizeIterator<Item = FrameTiming> + Clone,
    right: &[FrameTiming],
    limit: u64,
    work: &WorkControl,
) -> Result<Comparison, Error> {
    work.check()?;
    let mut left_values = Vec::with_capacity(left.len());
    for frame in left.clone() {
        work.check()?;
        left_values.push(frame.sensor_pts);
    }
    let mut right_values = Vec::with_capacity(right.len());
    for (index, frame) in right.iter().enumerate() {
        work.check()?;
        right_values.push((frame.sensor_pts, index));
    }
    // Raw values can wrap/repeat. Sorting is for exact-value lookup, never a
    // claim of chronological ordering or alignment across different epochs.
    left_values.sort_unstable();
    work.check()?;
    right_values.sort_unstable();
    work.check()?;
    let mut result = Comparison {
        scope: "unique equal raw PTS values only; wrap epochs, clocks and exposures unverified",
        unique_equal_value_pairs: 0,
        left_frames_without_equal_right_value: 0,
        left_frames_with_ambiguous_equal_value: 0,
        unique_pairs_within_host_limit: 0,
        minimum_absolute_host_delta_us: None,
        maximum_absolute_host_delta_us: None,
    };
    for frame in left {
        work.check()?;
        let value = frame.sensor_pts;
        let start = right_values.partition_point(|&(pts, _)| pts < value);
        let end = right_values.partition_point(|&(pts, _)| pts <= value);
        if start == end {
            result.left_frames_without_equal_right_value += 1;
            continue;
        }
        let left_count = left_values.partition_point(|&pts| pts <= value)
            - left_values.partition_point(|&pts| pts < value);
        if left_count != 1 || end - start != 1 {
            result.left_frames_with_ambiguous_equal_value += 1;
            continue;
        }
        let right_frame = right[right_values[start].1];
        let delta = frame
            .host_timestamp_us
            .abs_diff(right_frame.host_timestamp_us);
        result.unique_equal_value_pairs += 1;
        result.unique_pairs_within_host_limit += usize::from(delta <= limit);
        result.minimum_absolute_host_delta_us = Some(
            result
                .minimum_absolute_host_delta_us
                .map_or(delta, |old| old.min(delta)),
        );
        result.maximum_absolute_host_delta_us = Some(
            result
                .maximum_absolute_host_delta_us
                .map_or(delta, |old| old.max(delta)),
        );
    }
    work.check()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frames(values: &[(u64, u32)]) -> Vec<FrameTiming> {
        values
            .iter()
            .enumerate()
            .map(|(i, &(host, pts))| FrameTiming {
                sequence: i as u32,
                host_timestamp_us: host,
                sensor_pts: pts,
            })
            .collect()
    }
    #[test]
    fn raw_value_matches_are_independent_of_host_nearest_and_limits() {
        let left = frames(&[(100, 9), (200, 10), (300, 11)]);
        let right = frames(&[(101, 10), (201, 11), (301, 12)]);
        let result = compare(left.into_iter(), &right, 99, &WorkControl::default()).unwrap();
        assert_eq!(result.unique_equal_value_pairs, 2);
        assert_eq!(result.left_frames_without_equal_right_value, 1);
        assert_eq!(result.unique_pairs_within_host_limit, 2);
        assert_eq!(result.minimum_absolute_host_delta_us, Some(99));
        assert_eq!(result.maximum_absolute_host_delta_us, Some(99));
    }
    #[test]
    fn wrap_and_repeated_values_are_not_assigned_to_arbitrary_epochs() {
        let left = frames(&[(1, u32::MAX), (2, 0), (3, 1), (4, 0), (5, 2)]);
        let right = frames(&[(10, u32::MAX), (20, 0), (30, 1), (40, 2), (50, 2)]);
        let result = compare(left.into_iter(), &right, 100, &WorkControl::default()).unwrap();
        assert_eq!(result.unique_equal_value_pairs, 2);
        assert_eq!(result.left_frames_with_ambiguous_equal_value, 3);
        assert_eq!(result.left_frames_without_equal_right_value, 0);
        assert_eq!(result.minimum_absolute_host_delta_us, Some(9));
        assert_eq!(result.maximum_absolute_host_delta_us, Some(27));
    }
    #[test]
    fn sorted_lookup_matches_exhaustive_raw_value_reference_with_collisions() {
        let left = frames(
            &(0..120)
                .map(|i| (i * 23, (i * 37 % 127) as u32))
                .collect::<Vec<_>>(),
        );
        let right = frames(
            &(0..100)
                .map(|i| (i * 29, (i * 19 % 89) as u32))
                .collect::<Vec<_>>(),
        );
        for limit in [0, 100, u64::MAX] {
            let report =
                compare(left.iter().copied(), &right, limit, &WorkControl::default()).unwrap();
            let (mut unique, mut ambiguous, mut missing, mut within) = (0, 0, 0, 0);
            let mut deltas = Vec::new();
            for l in &left {
                let matches: Vec<_> = right
                    .iter()
                    .filter(|r| r.sensor_pts == l.sensor_pts)
                    .collect();
                let left_count = left
                    .iter()
                    .filter(|other| other.sensor_pts == l.sensor_pts)
                    .count();
                if matches.is_empty() {
                    missing += 1;
                } else if matches.len() != 1 || left_count != 1 {
                    ambiguous += 1;
                } else {
                    unique += 1;
                    let delta = l.host_timestamp_us.abs_diff(matches[0].host_timestamp_us);
                    within += usize::from(delta <= limit);
                    deltas.push(delta);
                }
            }
            assert_eq!(
                (
                    report.unique_equal_value_pairs,
                    report.left_frames_with_ambiguous_equal_value,
                    report.left_frames_without_equal_right_value,
                    report.unique_pairs_within_host_limit
                ),
                (unique, ambiguous, missing, within)
            );
            assert_eq!(
                report.minimum_absolute_host_delta_us,
                deltas.iter().min().copied()
            );
            assert_eq!(
                report.maximum_absolute_host_delta_us,
                deltas.iter().max().copied()
            );
            assert!(unique > 0 && ambiguous > 0 && missing > 0);
        }
    }
    #[test]
    fn absent_matches_and_extreme_host_values_keep_optional_bounds_and_cancel() {
        let work = WorkControl::default();
        let result = compare(
            frames(&[(0, 1)]).into_iter(),
            &frames(&[(u64::MAX, 1)]),
            u64::MAX,
            &work,
        )
        .unwrap();
        assert_eq!(result.maximum_absolute_host_delta_us, Some(u64::MAX));
        let result = compare(frames(&[(1, 1)]).into_iter(), &[], 0, &work).unwrap();
        assert_eq!(result.unique_equal_value_pairs, 0);
        assert_eq!(result.minimum_absolute_host_delta_us, None);
        assert_eq!(result.maximum_absolute_host_delta_us, None);
        work.cancel();
        assert!(compare(Vec::<FrameTiming>::new().into_iter(), &[], 0, &work).is_err());
    }
}
