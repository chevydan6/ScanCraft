//! Full-resolution screen selection. Coordinates are logical pixels; world units are mm.
use crate::{
    Cloud,
    work::{Stage, WorkControl},
};
use std::collections::HashMap;
#[derive(Clone, Copy, Debug)]
pub struct OrthographicProjection {
    center: [f64; 3],
    extent: f64,
    trig: [f64; 4],
    scale: f64,
    screen_center: [f64; 2],
    viewport: [f64; 4],
}
#[derive(Clone, Copy, Debug)]
pub struct ProjectedPoint {
    pub xy: [f64; 2],
    pub depth: f64,
}
impl OrthographicProjection {
    pub fn new(
        bounds: ([f64; 3], [f64; 3]),
        angles: [f64; 2],
        zoom: f64,
        viewport: [f64; 4],
        pan: [f64; 2],
    ) -> Result<Self, String> {
        let (min, max) = bounds;
        if (0..3).any(|a| !min[a].is_finite() || !max[a].is_finite() || min[a] > max[a])
            || !angles
                .iter()
                .chain(viewport.iter())
                .chain(pan.iter())
                .all(|v| v.is_finite())
            || !zoom.is_finite()
            || zoom <= 0.
            || viewport[2] <= 0.
            || viewport[3] <= 0.
        {
            return Err("invalid projection parameters".into());
        }
        let center = std::array::from_fn(|a| min[a] / 2. + max[a] / 2.);
        let extent = (0..3)
            .map(|a| max[a] / 2. - min[a] / 2.)
            .fold(0., f64::max)
            .max(1e-12);
        let scale = viewport[2].min(viewport[3]) * 0.75 * zoom;
        let screen_center = [
            viewport[0] + viewport[2] / 2. + pan[0],
            viewport[1] + viewport[3] / 2. + pan[1],
        ];
        if !scale.is_finite()
            || !screen_center.iter().all(|v| v.is_finite())
            || !pixel(viewport[0])
            || !pixel(viewport[1])
            || !pixel(viewport[0] + viewport[2])
            || !pixel(viewport[1] + viewport[3])
        {
            return Err("projection viewport out of supported pixel range".into());
        }
        Ok(Self {
            center,
            extent,
            trig: [
                angles[0].cos(),
                angles[0].sin(),
                angles[1].cos(),
                angles[1].sin(),
            ],
            scale,
            screen_center,
            viewport,
        })
    }
    pub fn project(&self, position: [f64; 3]) -> Option<ProjectedPoint> {
        self.project_offset(position, [0.; 3])
    }
    /// Project a model offset without first adding it to a distant world origin.
    pub fn project_offset(&self, position: [f64; 3], offset: [f64; 3]) -> Option<ProjectedPoint> {
        if !position.iter().chain(offset.iter()).all(|v| v.is_finite()) {
            return None;
        }
        let [x, y, z] = std::array::from_fn(|a| {
            (position[a] / 2. - self.center[a] / 2.) / self.extent + (offset[a] / 2.) / self.extent
        });
        let [cy, sy, cp, sp] = self.trig;
        let x1 = x * cy + z * sy;
        let z1 = -x * sy + z * cy;
        let y1 = y * cp - z1 * sp;
        let depth = y * sp + z1 * cp;
        let xy = [
            self.screen_center[0] + x1 * self.scale,
            self.screen_center[1] - y1 * self.scale,
        ];
        if !xy.iter().all(|v| v.is_finite()) || !depth.is_finite() {
            return None;
        }
        Some(ProjectedPoint { xy, depth })
    }
    pub fn in_viewport(&self, p: [f64; 2]) -> bool {
        p[0] >= self.viewport[0]
            && p[1] >= self.viewport[1]
            && p[0] <= self.viewport[0] + self.viewport[2]
            && p[1] <= self.viewport[1] + self.viewport[3]
    }
}
#[derive(Clone, Debug)]
pub enum ScreenShape {
    Rectangle { min: [f64; 2], max: [f64; 2] },
    Polygon(Vec<[f64; 2]>),
    Brush { centers: Vec<[f64; 2]>, radius: f64 },
}
#[derive(Clone, Copy, Debug)]
pub enum Visibility {
    Through,
    FrontmostPerPixel { cell_size: f64 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMode {
    Replace,
    Add,
    Subtract,
    Intersect,
}
fn pixel(v: f64) -> bool {
    v.is_finite() && v.abs() <= 1e9
}
fn distance_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length2 = dx * dx + dy * dy;
    if length2 == 0. {
        return (p[0] - a[0]).hypot(p[1] - a[1]);
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length2).clamp(0., 1.);
    (p[0] - (a[0] + t * dx)).hypot(p[1] - (a[1] + t * dy))
}
impl ScreenShape {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::Rectangle { min, max } => {
                if !(0..2).all(|a| pixel(min[a]) && pixel(max[a]) && min[a] <= max[a]) {
                    return Err("invalid selection rectangle".into());
                }
            }
            Self::Polygon(vertices) => {
                if !(3..=8192).contains(&vertices.len())
                    || !vertices.iter().flatten().all(|v| pixel(*v))
                {
                    return Err("polygon needs 3..8192 finite pixel vertices".into());
                }
                let a = vertices[0];
                let b = vertices
                    .iter()
                    .copied()
                    .find(|p| *p != a)
                    .ok_or("polygon has no area")?;
                if !vertices.iter().any(|c| {
                    ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() > 1e-12
                }) {
                    return Err("polygon vertices are collinear".into());
                }
            }
            Self::Brush { centers, radius } => {
                if centers.is_empty()
                    || centers.len() > 8192
                    || !centers.iter().flatten().all(|v| pixel(*v))
                    || !radius.is_finite()
                    || *radius <= 0.
                    || *radius > 1e6
                {
                    return Err("invalid brush stroke".into());
                }
            }
        }
        Ok(())
    }
    fn contains(&self, p: [f64; 2], tolerance: f64) -> bool {
        match self {
            Self::Rectangle { min, max } => {
                (0..2).all(|a| p[a] >= min[a] - tolerance && p[a] <= max[a] + tolerance)
            }
            Self::Polygon(vertices) => {
                let mut inside = false;
                for i in 0..vertices.len() {
                    let a = vertices[i];
                    let b = vertices[(i + 1) % vertices.len()];
                    if distance_to_segment(p, a, b) <= tolerance {
                        return true;
                    }
                    if (a[1] > p[1]) != (b[1] > p[1])
                        && p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
                    {
                        inside = !inside;
                    }
                }
                inside
            }
            Self::Brush { centers, radius } => {
                let r = radius + tolerance;
                if centers.len() == 1 {
                    return (p[0] - centers[0][0]).hypot(p[1] - centers[0][1]) <= r;
                }
                centers
                    .windows(2)
                    .any(|v| distance_to_segment(p, v[0], v[1]) <= r)
            }
        }
    }
}
/// Frontmost mode is a point depth-buffer at the stated logical-pixel cell size,
/// not visibility through a reconstructed surface. Ties select the last original index.
pub fn select_screen(
    cloud: &Cloud,
    projection: &OrthographicProjection,
    shape: &ScreenShape,
    visibility: Visibility,
    tolerance: f64,
    work: &WorkControl,
) -> Result<Vec<usize>, String> {
    cloud.validate_with(work)?;
    shape.validate()?;
    if !tolerance.is_finite() || !(0. ..=100.).contains(&tolerance) {
        return Err("invalid boundary tolerance".into());
    }
    let mut front = HashMap::new();
    if let Visibility::FrontmostPerPixel { cell_size } = visibility {
        if !cell_size.is_finite() || !(0.25..=64.).contains(&cell_size) {
            return Err("depth-buffer cell size must be 0.25..64 pixels".into());
        }
        for (i, p) in cloud.points.iter().enumerate() {
            if i % 1024 == 0 {
                work.update(Stage::Indexing, i, cloud.points.len())?;
            }
            if let Some(q) = projection
                .project(p.position)
                .filter(|q| projection.in_viewport(q.xy))
            {
                let key = (
                    (q.xy[0] / cell_size).floor() as i64,
                    (q.xy[1] / cell_size).floor() as i64,
                );
                let current = front.entry(key).or_insert((q.depth, i));
                if q.depth > current.0 || (q.depth == current.0 && i > current.1) {
                    *current = (q.depth, i);
                }
            }
        }
    }
    let mut indices = Vec::new();
    for (i, p) in cloud.points.iter().enumerate() {
        if i % 128 == 0 {
            work.update(Stage::Processing, i, cloud.points.len())?;
        }
        let Some(q) = projection
            .project(p.position)
            .filter(|q| projection.in_viewport(q.xy))
        else {
            continue;
        };
        if let Visibility::FrontmostPerPixel { cell_size } = visibility {
            let key = (
                (q.xy[0] / cell_size).floor() as i64,
                (q.xy[1] / cell_size).floor() as i64,
            );
            if front.get(&key).is_none_or(|(_, index)| *index != i) {
                continue;
            }
        }
        if shape.contains(q.xy, tolerance) {
            indices.push(i);
        }
    }
    work.check()?;
    Ok(indices)
}
pub fn combine_selection(
    count: usize,
    current: &[usize],
    incoming: &[usize],
    mode: SelectionMode,
    work: &WorkControl,
) -> Result<Vec<usize>, String> {
    let mut mask = vec![0u8; count];
    for (indices, flag) in [(current, 1u8), (incoming, 2u8)] {
        for (i, index) in indices.iter().enumerate() {
            if i % 1024 == 0 {
                work.check()?;
            }
            let value = mask.get_mut(*index).ok_or("selection index out of range")?;
            if *value & flag != 0 {
                return Err("duplicate selection index".into());
            }
            *value |= flag;
        }
    }
    let mut result = Vec::new();
    for (i, flags) in mask.into_iter().enumerate() {
        if i % 1024 == 0 {
            work.update(Stage::Processing, i, count)?;
        }
        let chosen = match mode {
            SelectionMode::Replace => flags & 2 != 0,
            SelectionMode::Add => flags != 0,
            SelectionMode::Subtract => flags == 1,
            SelectionMode::Intersect => flags == 3,
        };
        if chosen {
            result.push(i);
        }
    }
    work.check()?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Point;
    fn projection() -> OrthographicProjection {
        OrthographicProjection::new(
            ([-1.; 3], [1.; 3]),
            [0.; 2],
            1.,
            [0., 0., 1000., 1000.],
            [0.; 2],
        )
        .unwrap()
    }
    #[test]
    fn offset_projection_matches_world_position_and_preserves_small_offsets() {
        let p = projection();
        assert_eq!(
            p.project_offset([0.2, 0.3, 0.4], [0.1, -0.2, 0.5])
                .unwrap()
                .xy,
            p.project([0.2 + 0.1, 0.3 - 0.2, 0.4 + 0.5]).unwrap().xy
        );
        let distant = OrthographicProjection::new(
            ([1e100; 3], [1e100; 3]),
            [0.; 2],
            1.,
            [0., 0., 1000., 1000.],
            [0.; 2],
        )
        .unwrap();
        let center = distant.project([1e100; 3]).unwrap();
        let moved = distant.project_offset([1e100; 3], [1., 0., 0.]).unwrap();
        assert!(moved.xy[0] > center.xy[0]);
        assert!(p.project_offset([0.; 3], [f64::NAN, 0., 0.]).is_none());
    }
    fn from_pixel(x: f64, y: f64, z: f64) -> Point {
        Point {
            position: [(x - 500.) / 375., (500. - y) / 375., z],
            color: None,
            normal: None,
        }
    }
    #[test]
    fn million_labeled_points_match_analytic_rectangle() {
        let cloud = Cloud {
            points: (0..1000000)
                .map(|i| from_pixel((i % 1000) as f64 + 0.5, (i / 1000) as f64 + 0.5, 0.))
                .collect(),
        };
        let selected = select_screen(
            &cloud,
            &projection(),
            &ScreenShape::Rectangle {
                min: [250., 250.],
                max: [750., 750.],
            },
            Visibility::Through,
            1e-9,
            &WorkControl::default(),
        )
        .unwrap();
        assert_eq!(selected.len(), 250000);
        assert!(selected.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            selected
                .iter()
                .all(|i| (250..750).contains(&(i % 1000)) && (250..750).contains(&(i / 1000)))
        );
    }
    #[test]
    fn polygon_boundaries_concavity_and_even_odd_crossing() {
        let shape = ScreenShape::Polygon(vec![
            [0., 0.],
            [4., 0.],
            [4., 1.],
            [1., 1.],
            [1., 4.],
            [0., 4.],
        ]);
        shape.validate().unwrap();
        for p in [[0., 0.], [4., 0.], [0.5, 3.], [3., 0.5], [1., 1.]] {
            assert!(shape.contains(p, 1e-9), "{p:?}");
        }
        assert!(!shape.contains([3., 3.], 1e-9));
        let bow = ScreenShape::Polygon(vec![[0., 0.], [4., 4.], [0., 4.], [4., 0.]]);
        bow.validate().unwrap();
        assert!(bow.contains([2., 0.5], 0.));
        assert!(bow.contains([2., 2.], 0.));
        assert!(!bow.contains([0.5, 2.], 0.));
    }
    #[test]
    fn brush_is_union_of_capsules_with_explicit_tolerance() {
        let brush = ScreenShape::Brush {
            centers: vec![[0., 0.], [10., 0.]],
            radius: 1.,
        };
        brush.validate().unwrap();
        for p in [[5., 0.5], [-0.5, 0.], [10.5, 0.]] {
            assert!(brush.contains(p, 0.));
        }
        assert!(!brush.contains([5., 1.1], 0.));
        assert!(brush.contains([5., 1.1], 0.25));
    }
    #[test]
    fn visibility_and_tie_rules_are_deterministic() {
        let cloud = Cloud {
            points: vec![
                from_pixel(500., 500., -1.),
                from_pixel(500., 500., 1.),
                from_pixel(500., 500., 1.),
            ],
        };
        let shape = ScreenShape::Rectangle {
            min: [400., 400.],
            max: [600., 600.],
        };
        assert_eq!(
            select_screen(
                &cloud,
                &projection(),
                &shape,
                Visibility::Through,
                0.,
                &WorkControl::default()
            )
            .unwrap(),
            [0, 1, 2]
        );
        assert_eq!(
            select_screen(
                &cloud,
                &projection(),
                &shape,
                Visibility::FrontmostPerPixel { cell_size: 1. },
                0.,
                &WorkControl::default()
            )
            .unwrap(),
            [2]
        );
    }
    #[test]
    fn selection_modes_and_invalid_indices() {
        let w = WorkControl::default();
        assert_eq!(
            combine_selection(5, &[0, 2], &[1, 2], SelectionMode::Add, &w).unwrap(),
            [0, 1, 2]
        );
        assert_eq!(
            combine_selection(5, &[0, 2], &[1, 2], SelectionMode::Subtract, &w).unwrap(),
            [0]
        );
        assert_eq!(
            combine_selection(5, &[0, 2], &[1, 2], SelectionMode::Intersect, &w).unwrap(),
            [2]
        );
        assert_eq!(
            combine_selection(5, &[0, 2], &[1, 2], SelectionMode::Replace, &w).unwrap(),
            [1, 2]
        );
        assert!(combine_selection(5, &[5], &[], SelectionMode::Add, &w).is_err());
    }
    #[test]
    fn invalid_shapes_projection_and_cancellation_are_controlled() {
        assert!(
            ScreenShape::Polygon(vec![[0., 0.], [1., 1.], [2., 2.]])
                .validate()
                .is_err()
        );
        assert!(
            ScreenShape::Brush {
                centers: vec![],
                radius: 1.
            }
            .validate()
            .is_err()
        );
        assert!(
            OrthographicProjection::new(
                ([-1.; 3], [1.; 3]),
                [0.; 2],
                f64::NAN,
                [0., 0., 1., 1.],
                [0.; 2]
            )
            .is_err()
        );
        let w = WorkControl::default();
        w.cancel();
        assert!(
            select_screen(
                &Cloud { points: vec![] },
                &projection(),
                &ScreenShape::Rectangle {
                    min: [0.; 2],
                    max: [1.; 2]
                },
                Visibility::Through,
                0.,
                &w
            )
            .is_err()
        );
    }
}
