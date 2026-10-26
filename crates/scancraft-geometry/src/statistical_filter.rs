//! Exact distance statistics and candidate selection; never infers noise labels.
use crate::{
    Cloud,
    spatial::SpatialIndex,
    work::{Stage, WorkControl},
};
#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub neighbors: usize,
    pub sigma_multiplier: f64,
    pub mean_neighbor_distance_mm: Vec<f64>,
    pub global_mean_mm: f64,
    pub sample_standard_deviation_mm: f64,
    pub threshold_mm: f64,
    pub outlier_indices: Vec<usize>,
}
/// Self is excluded by index; coincident other samples count. Equality is kept.
/// Global dispersion uses sample variance (N-1), not population variance.
pub fn analyze(
    cloud: &Cloud,
    neighbors: usize,
    sigma_multiplier: f64,
    work: &WorkControl,
) -> Result<Report, String> {
    cloud.validate_with(work)?;
    let count = cloud.points.len();
    if !(2..=1_000_000).contains(&count)
        || !(1..=1024).contains(&neighbors)
        || neighbors >= count
        || !sigma_multiplier.is_finite()
        || sigma_multiplier < 0.
    {
        return Err("statistical filtering requires 2..1000000 points, 1..1024 neighbors below point count and finite nonnegative sigma multiplier".into());
    }
    let points: Vec<_> = cloud.points.iter().map(|p| p.position).collect();
    let index = SpatialIndex::build(&points, work)?;
    let mut distances = Vec::with_capacity(count);
    let mut mean = 0.;
    let mut m2 = 0.;
    for (i, point) in points.iter().enumerate() {
        work.update(Stage::Processing, i, count)?;
        let nearest = index.k_nearest(*point, neighbors, Some(i), work)?;
        let distance = nearest
            .iter()
            .map(|n| n.distance_mm / (neighbors as f64))
            .sum::<f64>();
        let delta = distance - mean;
        mean += delta / ((i + 1) as f64);
        m2 += delta * (distance - mean);
        if !distance.is_finite() || !mean.is_finite() || !m2.is_finite() {
            return Err("statistical distance moments overflow".into());
        }
        distances.push(distance);
    }
    let deviation = (m2.max(0.) / ((count - 1) as f64)).sqrt();
    let threshold = mean + sigma_multiplier * deviation;
    if !threshold.is_finite() {
        return Err("statistical distance threshold overflow".into());
    }
    let mut outliers = Vec::new();
    for (i, distance) in distances.iter().enumerate() {
        if i.is_multiple_of(1024) {
            work.check()?;
        }
        if *distance > threshold {
            outliers.push(i);
        }
    }
    work.check()?;
    Ok(Report {
        neighbors,
        sigma_multiplier,
        mean_neighbor_distance_mm: distances,
        global_mean_mm: mean,
        sample_standard_deviation_mm: deviation,
        threshold_mm: threshold,
        outlier_indices: outliers,
    })
}
pub fn filter(
    cloud: &Cloud,
    neighbors: usize,
    sigma_multiplier: f64,
    work: &WorkControl,
) -> Result<(Cloud, Report), String> {
    let report = analyze(cloud, neighbors, sigma_multiplier, work)?;
    let filtered = cloud.trimmed_with(&report.outlier_indices, false, work)?;
    Ok((filtered, report))
}
