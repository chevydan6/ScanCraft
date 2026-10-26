//! Exact raw grayscale intensity summaries and display-only tone mapping.
//! These values do not establish focus, optical validity or depth accuracy.
use crate::GrayImage;
#[derive(Debug, Clone, PartialEq)]
pub struct GrayStatistics {
    pub pixels: usize,
    pub minimum: u8,
    pub p01: u8,
    pub median: u8,
    pub p99: u8,
    pub maximum: u8,
    pub mean: f64,
    pub black_clipped: usize,
    pub white_clipped: usize,
}
pub fn statistics(image: GrayImage<'_>) -> GrayStatistics {
    let mut histogram = [0usize; 256];
    for value in image.data {
        histogram[usize::from(*value)] += 1;
    }
    let pixels = image.data.len();
    let quantile = |percent: usize| {
        let rank = (pixels * percent).div_ceil(100).max(1);
        let mut sum = 0;
        for (i, count) in histogram.iter().enumerate() {
            sum += count;
            if sum >= rank {
                return i as u8;
            }
        }
        unreachable!("nonempty bounded GrayImage")
    };
    GrayStatistics {
        pixels,
        minimum: histogram.iter().position(|n| *n > 0).unwrap() as u8,
        p01: quantile(1),
        median: quantile(50),
        p99: quantile(99),
        maximum: histogram.iter().rposition(|n| *n > 0).unwrap() as u8,
        mean: histogram
            .iter()
            .enumerate()
            .map(|(v, n)| v as f64 * *n as f64)
            .sum::<f64>()
            / pixels as f64,
        black_clipped: histogram[0],
        white_clipped: histogram[255],
    }
}
impl GrayStatistics {
    /// Map p01..p99 to 0..255, clipping display tails. A flat percentile range
    /// uses identity. Apply only to a display buffer, never sensor/reconstruction data.
    pub fn contrast_lut(&self) -> [u8; 256] {
        std::array::from_fn(|i| {
            let span = u32::from(self.p99.saturating_sub(self.p01));
            if span == 0 {
                return i as u8;
            }
            let level = u32::from((i as u8).saturating_sub(self.p01)).min(span);
            ((level * 255 + span / 2) / span) as u8
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quantiles_match_sorted_nearest_rank_and_count_exact_clipping() {
        let data: Vec<u8> = (0..1000).map(|i| ((i * 73 + 19) % 256) as u8).collect();
        let mut sorted = data.clone();
        sorted.sort_unstable();
        let stats = statistics(GrayImage::new(100, 10, &data).unwrap());
        for (p, actual) in [(1, stats.p01), (50, stats.median), (99, stats.p99)] {
            assert_eq!(actual, sorted[(data.len() * p).div_ceil(100) - 1]);
        }
        assert_eq!(
            stats.black_clipped,
            data.iter().filter(|v| **v == 0).count()
        );
        assert_eq!(
            stats.white_clipped,
            data.iter().filter(|v| **v == 255).count()
        );
        assert!(
            (stats.mean - data.iter().map(|v| f64::from(*v)).sum::<f64>() / 1000.).abs() < 1e-12
        );
    }
    #[test]
    fn display_lut_is_monotonic_and_leaves_source_unchanged() {
        let data: Vec<u8> = (0..100).map(|i| 20 + (i % 21) as u8).collect();
        let original = data.clone();
        let s = statistics(GrayImage::new(10, 10, &data).unwrap());
        let lut = s.contrast_lut();
        assert_eq!(lut[usize::from(s.p01)], 0);
        assert_eq!(lut[usize::from(s.p99)], 255);
        assert!(lut.windows(2).all(|w| w[0] <= w[1]));
        let display: Vec<_> = data.iter().map(|v| lut[usize::from(*v)]).collect();
        assert_ne!(display, data);
        assert_eq!(data, original);
        let flat = [17u8; 16];
        let lut = statistics(GrayImage::new(4, 4, &flat).unwrap()).contrast_lut();
        assert_eq!(lut[17], 17);
        let one = [255u8];
        let s = statistics(GrayImage::new(1, 1, &one).unwrap());
        assert_eq!(s.white_clipped, 1);
        assert_eq!(s.p01, 255);
    }
}
