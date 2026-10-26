//! Deterministic, attribute-preserving point filters in millimeters.
use crate::{
    Cloud,
    work::{Stage, WorkControl},
};
use std::collections::{HashMap, HashSet};
fn cell(position: [f64; 3], size: f64) -> Result<[i64; 3], String> {
    let mut key = [0; 3];
    for a in 0..3 {
        let value = (position[a] / size).floor();
        if !value.is_finite() || value <= i64::MIN as f64 || value >= i64::MAX as f64 {
            return Err("coordinate exceeds spatial index range".into());
        }
        key[a] = value as i64;
    }
    Ok(key)
}
/// Retain the first point's attributes for exactly coincident coordinates; -0 equals +0.
pub fn remove_duplicates(cloud: &Cloud) -> Result<Cloud, String> {
    remove_duplicates_with(cloud, &WorkControl::default())
}
pub fn remove_duplicates_with(cloud: &Cloud, work: &WorkControl) -> Result<Cloud, String> {
    cloud.validate_with(work)?;
    let mut seen = HashSet::new();
    let mut points = Vec::new();
    for (i, p) in cloud.points.iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Processing, i, cloud.points.len())?;
        }
        if seen.insert(p.position.map(|v| if v == 0. { 0 } else { v.to_bits() })) {
            points.push(p.clone());
        }
    }
    work.check()?;
    Ok(Cloud { points })
}
/// Excludes the point itself from its neighbor count. Coincident other points count.
/// Dense neighborhoods stop searching once the required count is met.
pub fn radius_filter(cloud: &Cloud, radius_mm: f64, min_neighbors: usize) -> Result<Cloud, String> {
    radius_filter_with(cloud, radius_mm, min_neighbors, &WorkControl::default())
}
pub fn radius_filter_with(
    cloud: &Cloud,
    radius_mm: f64,
    min_neighbors: usize,
    work: &WorkControl,
) -> Result<Cloud, String> {
    cloud.validate_with(work)?;
    if !radius_mm.is_finite() || radius_mm <= 0. {
        return Err("radius must be positive and finite".into());
    }
    if min_neighbors == 0 {
        return Ok(cloud.clone());
    }
    let mut keys = Vec::with_capacity(cloud.points.len());
    for (i, p) in cloud.points.iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Indexing, i, cloud.points.len())?;
        }
        keys.push(cell(p.position, radius_mm)?);
    }
    let mut grid: HashMap<[i64; 3], Vec<usize>> = HashMap::new();
    for (i, k) in keys.iter().enumerate() {
        if i % 1024 == 0 {
            work.check()?;
        }
        grid.entry(*k).or_default().push(i);
    }
    let mut points = Vec::new();
    for (i, p) in cloud.points.iter().enumerate() {
        work.update(Stage::Processing, i, cloud.points.len())?;
        let key = keys[i];
        let mut neighbors = 0;
        'search: for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let Some(kx) = key[0].checked_add(x) else {
                        continue;
                    };
                    let Some(ky) = key[1].checked_add(y) else {
                        continue;
                    };
                    let Some(kz) = key[2].checked_add(z) else {
                        continue;
                    };
                    if let Some(candidates) = grid.get(&[kx, ky, kz]) {
                        for (candidate, j) in candidates.iter().enumerate() {
                            if candidate % 1024 == 0 {
                                work.check()?;
                            }
                            if *j == i {
                                continue;
                            }
                            let q = &cloud.points[*j];
                            let distance = (p.position[0] - q.position[0])
                                .hypot(p.position[1] - q.position[1])
                                .hypot(p.position[2] - q.position[2]);
                            if distance <= radius_mm {
                                neighbors += 1;
                                if neighbors >= min_neighbors {
                                    break 'search;
                                }
                            }
                        }
                    }
                }
            }
        }
        if neighbors >= min_neighbors {
            points.push(p.clone());
        }
    }
    Ok(Cloud { points })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;
    fn point(position: [f64; 3]) -> Point {
        Point {
            position,
            color: Some([1, 2, 3]),
            normal: None,
        }
    }
    #[test]
    fn labeled_grid_and_outliers() {
        let mut points = Vec::new();
        for x in 0..10 {
            for y in 0..10 {
                points.push(point([x as f64, y as f64, 0.]));
            }
        }
        for i in 0..20 {
            points.push(point([100. + i as f64 * 10., 100., 100.]));
        }
        let c = Cloud { points };
        let filtered = radius_filter(&c, 1.5, 3).unwrap();
        assert_eq!(filtered.points.len(), 100);
        assert_eq!(&filtered.points[..], &c.points[..100]);
    }
    #[test]
    fn compare_against_brute_force() {
        let c = Cloud {
            points: (0..300)
                .map(|i| {
                    point([
                        ((i * 73) % 127) as f64 / 10.,
                        ((i * 29) % 173) as f64 / 10.,
                        ((i * 47) % 67) as f64 / 10.,
                    ])
                })
                .collect(),
        };
        let radius = 1.7;
        let min = 3;
        let expected: Vec<_> = c
            .points
            .iter()
            .enumerate()
            .filter(|(i, p)| {
                c.points
                    .iter()
                    .enumerate()
                    .filter(|(j, q)| {
                        i != j && crate::distance_mm(p.position, q.position).unwrap() <= radius
                    })
                    .count()
                    >= min
            })
            .map(|(_, p)| p.clone())
            .collect();
        assert_eq!(radius_filter(&c, radius, min).unwrap().points, expected);
    }
    #[test]
    fn duplicates_and_signed_zero() {
        let c = Cloud {
            points: vec![
                point([0., 1., 2.]),
                point([-0., 1., 2.]),
                point([1., 1., 2.]),
            ],
        };
        assert_eq!(remove_duplicates(&c).unwrap().points.len(), 2);
        assert_eq!(radius_filter(&c, 0.01, 1).unwrap().points.len(), 2);
        assert!(radius_filter(&c, f64::NAN, 2).is_err());
    }
}
