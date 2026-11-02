//! Within-image row continuity of stripe observations, not projector identity.
//! Original conservative endpoint association; no upstream implementation copied.
use crate::{laser::LineObservation, pixels};
use scancraft_geometry::work::{Stage, WorkControl};

#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Largest positive difference between consecutive observed integer rows.
    pub maximum_row_gap: u32,
    /// Allowed |dx| is this intercept plus maximum_slope * dy.
    pub maximum_horizontal_step: f64,
    pub maximum_slope: f64,
    pub minimum_span_rows: u32,
    pub minimum_observations: usize,
    /// Hard endpoint/observation pair budget; exhaustion returns no partial result.
    pub maximum_comparisons: usize,
}

#[derive(Debug, PartialEq)]
pub struct Trace {
    /// Indices into the unchanged input, ordered by increasing row.
    pub observation_indices: Vec<usize>,
    pub span_rows: u32,
    /// Both requested span and sample-count criteria pass. Not a laser label.
    pub continuity_supported: bool,
}

#[derive(Debug)]
pub struct Traces {
    /// Every input observation belongs to exactly one trace, including singletons.
    pub traces: Vec<Trace>,
    /// Current-row observations having candidates but no mutually unique endpoint.
    pub ambiguous_observations: usize,
    pub comparisons: usize,
}

/// Associate only mutually unique endpoints within the configured displacement
/// bound. At a fork/join all contested old endpoints end and unmatched current
/// observations start new traces. Zero-candidate endpoints survive bounded gaps.
/// No nearest-neighbor tie-break, fitted curve, calibration or depth is used.
///
/// Positions must be finite, in-image, sorted by (integer y, strictly increasing
/// x). Other LineObservation fields are not used. At most 100,000 observations,
/// MAX_PIXELS image pixels and 100 million comparisons are accepted. Storage is
/// O(observations); work is O(observations + comparisons). Cancellation is checked
/// at most every 1,024 pair comparisons and at row/validation/output boundaries.
pub fn trace_rows(
    size: [u32; 2],
    observations: &[LineObservation],
    config: Config,
    work: &WorkControl,
) -> Result<Traces, String> {
    pixels(size[0], size[1])?;
    if observations.len() > 100_000
        || config.maximum_row_gap == 0
        || config.maximum_row_gap > size[1]
        || !config.maximum_horizontal_step.is_finite()
        || !(0. ..=f64::from(size[0])).contains(&config.maximum_horizontal_step)
        || !config.maximum_slope.is_finite()
        || !(0. ..=f64::from(size[0])).contains(&config.maximum_slope)
        || config.minimum_span_rows >= size[1]
        || !(1..=100_000).contains(&config.minimum_observations)
        || !(1..=100_000_000).contains(&config.maximum_comparisons)
    {
        return Err("invalid stripe continuity limits".into());
    }
    for (i, observation) in observations.iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Validating, i, observations.len())?;
        }
        let [x, y] = observation.pixel;
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.
            || x > f64::from(size[0] - 1)
            || y < 0.
            || y > f64::from(size[1] - 1)
            || y.fract() != 0.
            || (i > 0 && {
                let [px, py] = observations[i - 1].pixel;
                y < py || (y == py && x <= px)
            })
        {
            return Err("stripe positions must be in-image and strictly row ordered".into());
        }
    }
    work.check()?;
    let mut result = Traces {
        traces: Vec::new(),
        ambiguous_observations: 0,
        comparisons: 0,
    };
    let mut active: Vec<usize> = Vec::new();
    let mut start = 0;
    while start < observations.len() {
        work.update(Stage::Matching, start, observations.len())?;
        let row = observations[start].pixel[1];
        let mut end = start + 1;
        while end < observations.len() && observations[end].pixel[1] == row {
            end += 1;
        }
        active.retain(|&t| {
            let last = *result.traces[t].observation_indices.last().unwrap();
            row - observations[last].pixel[1] <= f64::from(config.maximum_row_gap)
        });
        // Counts saturate at two: only zero / one / ambiguous matters.
        let mut incoming = vec![0u8; end - start];
        let mut candidate = vec![0usize; end - start];
        let mut outgoing = vec![0u8; active.len()];
        for (endpoint, &t) in active.iter().enumerate() {
            let last = *result.traces[t].observation_indices.last().unwrap();
            let [x, y] = observations[last].pixel;
            let bound = config.maximum_horizontal_step + config.maximum_slope * (row - y);
            for (local, observation) in observations[start..end].iter().enumerate() {
                if result.comparisons == config.maximum_comparisons {
                    return Err("stripe continuity comparison limit exceeded".into());
                }
                if result.comparisons.is_multiple_of(1024) {
                    work.check()?;
                }
                result.comparisons += 1;
                if (observation.pixel[0] - x).abs() <= bound {
                    incoming[local] = (incoming[local] + 1).min(2);
                    outgoing[endpoint] = (outgoing[endpoint] + 1).min(2);
                    candidate[local] = endpoint;
                }
            }
        }
        let mut continued = vec![false; active.len()];
        let mut new = Vec::new();
        for (local, &count) in incoming.iter().enumerate() {
            let index = start + local;
            if count == 1 && outgoing[candidate[local]] == 1 {
                let endpoint = candidate[local];
                result.traces[active[endpoint]]
                    .observation_indices
                    .push(index);
                continued[endpoint] = true;
            } else {
                result.ambiguous_observations += usize::from(count != 0);
                new.push(result.traces.len());
                result.traces.push(Trace {
                    observation_indices: vec![index],
                    span_rows: 0,
                    continuity_supported: false,
                });
            }
        }
        active = active
            .into_iter()
            .enumerate()
            .filter_map(|(i, t)| (outgoing[i] == 0 || continued[i]).then_some(t))
            .chain(new)
            .collect();
        start = end;
    }
    for (i, trace) in result.traces.iter_mut().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        trace.span_rows = (observations[*trace.observation_indices.last().unwrap()].pixel[1]
            - observations[trace.observation_indices[0]].pixel[1]) as u32;
        trace.continuity_supported = trace.span_rows >= config.minimum_span_rows
            && trace.observation_indices.len() >= config.minimum_observations;
    }
    work.update(Stage::Matching, observations.len(), observations.len())?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation(x: f64, y: f64) -> LineObservation {
        LineObservation {
            pixel: [x, y],
            first_x: x as u32,
            last_x: x as u32,
            peak: 80,
            integrated_signal: 60,
            variance_x: 0.,
        }
    }
    fn config() -> Config {
        Config {
            maximum_row_gap: 3,
            maximum_horizontal_step: 1.,
            maximum_slope: 0.25,
            minimum_span_rows: 4,
            minimum_observations: 3,
            maximum_comparisons: 1000,
        }
    }
    #[test]
    fn sloped_parallel_traces_bridge_bounded_gaps_and_preserve_noise() {
        let input = vec![
            observation(4., 0.),
            observation(20., 0.),
            observation(4.5, 2.),
            observation(20.5, 2.),
            observation(40., 3.),
            observation(5., 4.),
            observation(21., 4.),
            observation(5., 8.),
        ];
        let original = input.clone();
        let result = trace_rows([64, 16], &input, config(), &WorkControl::default()).unwrap();
        assert_eq!(
            result
                .traces
                .iter()
                .map(|t| t.observation_indices.clone())
                .collect::<Vec<_>>(),
            vec![vec![0, 2, 5], vec![1, 3, 6], vec![4], vec![7]]
        );
        assert_eq!(
            result
                .traces
                .iter()
                .filter(|t| t.continuity_supported)
                .count(),
            2
        );
        assert_eq!(input, original);
    }
    #[test]
    fn forks_and_joins_end_contested_endpoints_without_a_nearest_choice() {
        for pixels in [
            vec![(10., 0.), (9.5, 1.), (10.5, 1.), (10., 2.)],
            vec![(9.5, 0.), (10.5, 0.), (10., 1.), (10., 2.)],
        ] {
            let input = pixels
                .into_iter()
                .map(|(x, y)| observation(x, y))
                .collect::<Vec<_>>();
            let r = trace_rows([32, 16], &input, config(), &WorkControl::default()).unwrap();
            assert!(r.traces.iter().all(|t| !t.continuity_supported));
            assert_eq!(r.traces[0].observation_indices, vec![0]);
            assert!(r.ambiguous_observations > 0);
            let mut partition = r
                .traces
                .into_iter()
                .flat_map(|t| t.observation_indices)
                .collect::<Vec<_>>();
            partition.sort();
            assert_eq!(partition, vec![0, 1, 2, 3]);
        }
    }
    #[test]
    fn exact_displacement_and_gap_boundaries_are_inclusive() {
        let input = vec![
            observation(4., 0.),
            observation(5.75, 3.),
            observation(6.75, 6.),
        ];
        let r = trace_rows([32, 16], &input, config(), &WorkControl::default()).unwrap();
        assert_eq!(r.traces.len(), 1);
        assert!(r.traces[0].continuity_supported);
        let mut outside = input;
        outside[1].pixel[0] += 0.0001;
        assert_eq!(
            trace_rows([32, 16], &outside, config(), &WorkControl::default())
                .unwrap()
                .traces
                .len(),
            2
        );
    }
    #[test]
    fn malformed_positions_order_parameters_and_budget_fail_closed() {
        let a = observation(4., 0.);
        let b = observation(4., 1.);
        for input in [
            vec![b.clone(), a.clone()],
            vec![a.clone(), a.clone()],
            vec![observation(f64::NAN, 0.)],
            vec![observation(4., 0.5)],
            vec![observation(32., 0.)],
        ] {
            assert!(trace_rows([32, 16], &input, config(), &WorkControl::default()).is_err());
        }
        for c in [
            Config {
                maximum_slope: f64::NAN,
                ..config()
            },
            Config {
                maximum_row_gap: 0,
                ..config()
            },
            Config {
                minimum_observations: 0,
                ..config()
            },
            Config {
                minimum_span_rows: 16,
                ..config()
            },
        ] {
            assert!(trace_rows([32, 16], &[], c, &WorkControl::default()).is_err());
        }
        let input = [a, b, observation(4., 2.)];
        assert!(
            trace_rows(
                [32, 16],
                &input,
                Config {
                    maximum_comparisons: 1,
                    ..config()
                },
                &WorkControl::default()
            )
            .unwrap_err()
            .contains("comparison limit")
        );
        let work = WorkControl::default();
        work.cancel();
        assert!(
            trace_rows([32, 16], &[], config(), &work)
                .unwrap_err()
                .contains("cancelled")
        );
        assert!(
            trace_rows([32, 16], &[], config(), &WorkControl::default())
                .unwrap()
                .traces
                .is_empty()
        );
    }
}
