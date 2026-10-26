//! Explicit attribute-only normal correction; no inferred orientation policy.
use crate::{
    Cloud,
    work::{Stage, WorkControl},
};
use std::collections::HashSet;
pub fn flip_selected(
    cloud: &Cloud,
    indices: &[usize],
    work: &WorkControl,
) -> Result<Cloud, String> {
    cloud.validate_with(work)?;
    if indices.is_empty() {
        return Err("select points whose normals should be flipped".into());
    }
    let mut seen = HashSet::with_capacity(indices.len().min(cloud.points.len()));
    for (i, index) in indices.iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Validating, i, indices.len())?;
        }
        if !seen.insert(*index) {
            return Err("duplicate normal selection index".into());
        }
        let point = cloud
            .points
            .get(*index)
            .ok_or("normal selection index out of bounds")?;
        let normal = point.normal.ok_or("selected point has no normal")?;
        if normal.iter().all(|v| *v == 0.) {
            return Err("selected point has a zero normal".into());
        }
    }
    work.check()?;
    let mut result = cloud.clone();
    for (i, index) in indices.iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Processing, i, indices.len())?;
        }
        result.points[*index].normal = result.points[*index].normal.map(|n| n.map(|v| -v));
    }
    work.check()?;
    Ok(result)
}
