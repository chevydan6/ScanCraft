//! Historical measurement evidence, independent of subsequent cloud edits.
use crate::{
    measurement::{self, Circle, Sphere},
    work::WorkControl,
};
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResultValue {
    Cylinder {
        fit: crate::cylinder::Cylinder,
        maximum_rms_mm: f64,
    },
    PointPlane {
        fit: crate::plane_measurement::PointPlane,
        probe: usize,
        maximum_plane_rms_mm: f64,
    },
    Distance {
        distance_mm: f64,
    },
    Angle {
        degrees: f64,
        vertex: usize,
    },
    Sphere {
        fit: Sphere,
    },
    Circle {
        fit: Circle,
        maximum_plane_rms_mm: f64,
    },
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema_version: u32,
    pub label: String,
    pub source_points_mm: Vec<[f64; 3]>,
    pub result: ResultValue,
    pub physical_accuracy_qualified: bool,
}
impl Snapshot {
    pub fn cylinder(
        label: String,
        points: Vec<[f64; 3]>,
        maximum_rms_mm: f64,
        work: &WorkControl,
    ) -> Result<Self, String> {
        let fit = crate::cylinder::fit(&points, maximum_rms_mm, work)?;
        let snapshot = Self {
            schema_version: 4,
            label,
            source_points_mm: points,
            result: ResultValue::Cylinder {
                fit,
                maximum_rms_mm,
            },
            physical_accuracy_qualified: false,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
    pub fn point_plane(
        label: String,
        points: Vec<[f64; 3]>,
        probe: usize,
        maximum_plane_rms_mm: f64,
        work: &WorkControl,
    ) -> Result<Self, String> {
        let fit = crate::plane_measurement::measure(&points, probe, maximum_plane_rms_mm, work)?;
        let snapshot = Self {
            schema_version: 3,
            label,
            source_points_mm: points,
            result: ResultValue::PointPlane {
                fit,
                probe,
                maximum_plane_rms_mm,
            },
            physical_accuracy_qualified: false,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
    pub fn distance(
        label: String,
        points: Vec<[f64; 3]>,
        work: &WorkControl,
    ) -> Result<Self, String> {
        work.check()?;
        if points.len() != 2 {
            return Err("distance snapshot requires exactly two points".into());
        }
        let distance_mm = crate::distance_mm(points[0], points[1])?;
        let snapshot = Self {
            schema_version: 2,
            label,
            source_points_mm: points,
            result: ResultValue::Distance { distance_mm },
            physical_accuracy_qualified: false,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
    /// `vertex` indexes the retained source coordinate list, not the edited cloud.
    pub fn angle(
        label: String,
        points: Vec<[f64; 3]>,
        vertex: usize,
        work: &WorkControl,
    ) -> Result<Self, String> {
        work.check()?;
        if points.len() != 3 || vertex >= 3 {
            return Err(
                "angle snapshot requires three points and a valid local vertex index".into(),
            );
        }
        let arms: Vec<_> = (0..3).filter(|&i| i != vertex).collect();
        let degrees = measurement::angle_degrees(points[arms[0]], points[vertex], points[arms[1]])?;
        let snapshot = Self {
            schema_version: 2,
            label,
            source_points_mm: points,
            result: ResultValue::Angle { degrees, vertex },
            physical_accuracy_qualified: false,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
    pub fn sphere(
        label: String,
        points: Vec<[f64; 3]>,
        work: &WorkControl,
    ) -> Result<Self, String> {
        let fit = measurement::fit_sphere(&points, work)?;
        let snapshot = Self {
            schema_version: 1,
            label,
            source_points_mm: points,
            result: ResultValue::Sphere { fit },
            physical_accuracy_qualified: false,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
    pub fn circle(
        label: String,
        points: Vec<[f64; 3]>,
        maximum_plane_rms_mm: f64,
        work: &WorkControl,
    ) -> Result<Self, String> {
        let fit = measurement::fit_circle(&points, maximum_plane_rms_mm, work)?;
        let snapshot = Self {
            schema_version: 1,
            label,
            source_points_mm: points,
            result: ResultValue::Circle {
                fit,
                maximum_plane_rms_mm,
            },
            physical_accuracy_qualified: false,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }
    pub fn validate(&self) -> Result<(), String> {
        let source_limit = match self.result {
            ResultValue::PointPlane { .. } => 100_001,
            ResultValue::Cylinder { .. } => 20_000,
            _ => 100_000,
        };
        if ![1, 2, 3, 4].contains(&self.schema_version)
            || self.label.trim().is_empty()
            || self.label.len() > 128
            || self.label.chars().any(char::is_control)
            || self.physical_accuracy_qualified
            || self.source_points_mm.len() > source_limit
            || !self
                .source_points_mm
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        {
            return Err("invalid measurement snapshot metadata or source points".into());
        }
        if let ResultValue::Cylinder {
            fit,
            maximum_rms_mm,
        } = &self.result
        {
            let length = fit.axis_direction[0]
                .hypot(fit.axis_direction[1])
                .hypot(fit.axis_direction[2]);
            if self.schema_version != 4
                || !(6..=20_000).contains(&self.source_points_mm.len())
                || fit.points != self.source_points_mm.len()
                || !fit
                    .axis_point_mm
                    .iter()
                    .chain(fit.axis_direction.iter())
                    .chain(fit.axial_range_mm.iter())
                    .all(|v| v.is_finite())
                || ![
                    length,
                    fit.radius_mm,
                    fit.diameter_mm,
                    fit.geometric_rms_mm,
                    fit.maximum_absolute_residual_mm,
                    fit.jacobian_condition_number,
                    *maximum_rms_mm,
                ]
                .iter()
                .all(|v| v.is_finite())
                || (length - 1.).abs() > 1e-8
                || fit.radius_mm <= 0.
                || fit.diameter_mm != 2. * fit.radius_mm
                || fit.geometric_rms_mm < 0.
                || fit.geometric_rms_mm
                    > fit.maximum_absolute_residual_mm
                        + 1e-12 * fit.maximum_absolute_residual_mm.max(f64::MIN_POSITIVE)
                || *maximum_rms_mm < 0.
                || fit.geometric_rms_mm > *maximum_rms_mm
                || fit.axial_range_mm[0] >= fit.axial_range_mm[1]
                || fit.jacobian_condition_number < 1.
                || fit.jacobian_condition_number >= 1e8
                || !(1..=80).contains(&fit.iterations)
                || !(1..=3).contains(&fit.starts_converged)
            {
                return Err("invalid saved cylinder fit or RMS gate".into());
            }
            return Ok(());
        }
        if let ResultValue::PointPlane {
            fit,
            probe,
            maximum_plane_rms_mm,
        } = &self.result
        {
            let length = fit.normal[0].hypot(fit.normal[1]).hypot(fit.normal[2]);
            if self.schema_version != 3
                || !(4..=100_001).contains(&self.source_points_mm.len())
                || *probe >= self.source_points_mm.len()
                || fit.plane_points != self.source_points_mm.len() - 1
                || !fit
                    .origin_mm
                    .iter()
                    .chain(fit.foot_mm.iter())
                    .chain(fit.normal.iter())
                    .all(|v| v.is_finite())
                || ![
                    fit.distance_mm,
                    fit.signed_distance_mm,
                    fit.plane_rms_mm,
                    *maximum_plane_rms_mm,
                    length,
                ]
                .iter()
                .all(|v| v.is_finite())
                || (length - 1.).abs() > 1e-8
                || fit.distance_mm != fit.signed_distance_mm.abs()
                || fit.plane_rms_mm < 0.
                || fit.plane_rms_mm > *maximum_plane_rms_mm
                || *maximum_plane_rms_mm < 0.
            {
                return Err("invalid saved point-plane measurement".into());
            }
            let p = self.source_points_mm[*probe];
            for (a, component) in p.iter().enumerate() {
                let expected = component - fit.signed_distance_mm * fit.normal[a];
                let magnitude = component
                    .abs()
                    .max(fit.signed_distance_mm.abs())
                    .max(fit.foot_mm[a].abs());
                let tolerance = (magnitude * f64::EPSILON * 16.).max(f64::MIN_POSITIVE);
                if !expected.is_finite() || (expected - fit.foot_mm[a]).abs() > tolerance {
                    return Err("saved point-plane foot disagrees with its probe and normal".into());
                }
            }
            return Ok(());
        }
        match self.result {
            ResultValue::Distance { distance_mm } => {
                if self.schema_version != 2
                    || self.source_points_mm.len() != 2
                    || !distance_mm.is_finite()
                    || distance_mm < 0.
                {
                    return Err("invalid saved distance".into());
                }
                let expected =
                    crate::distance_mm(self.source_points_mm[0], self.source_points_mm[1])?;
                if (expected - distance_mm).abs() > 1e-12 * expected.max(f64::MIN_POSITIVE) {
                    return Err("saved distance disagrees with source evidence".into());
                }
                return Ok(());
            }
            ResultValue::Angle { degrees, vertex } => {
                if self.schema_version != 2
                    || self.source_points_mm.len() != 3
                    || vertex >= 3
                    || !degrees.is_finite()
                    || !(0. ..=180.).contains(&degrees)
                {
                    return Err("invalid saved angle".into());
                }
                let arms: Vec<_> = (0..3).filter(|&i| i != vertex).collect();
                let expected = measurement::angle_degrees(
                    self.source_points_mm[arms[0]],
                    self.source_points_mm[vertex],
                    self.source_points_mm[arms[1]],
                )?;
                if (expected - degrees).abs() > 1e-10 {
                    return Err("saved angle disagrees with source evidence".into());
                }
                return Ok(());
            }
            _ => {}
        }
        let (center, radius, diameter, rms, maximum, condition, count, minimum) = match &self.result
        {
            ResultValue::Distance { .. }
            | ResultValue::Angle { .. }
            | ResultValue::PointPlane { .. }
            | ResultValue::Cylinder { .. } => unreachable!(),
            ResultValue::Sphere { fit } => (
                fit.center_mm,
                fit.radius_mm,
                fit.diameter_mm,
                fit.geometric_rms_mm,
                fit.maximum_absolute_residual_mm,
                fit.design_condition_number,
                fit.points,
                4,
            ),
            ResultValue::Circle {
                fit,
                maximum_plane_rms_mm,
            } => {
                let length = fit.normal[0].hypot(fit.normal[1]).hypot(fit.normal[2]);
                if !maximum_plane_rms_mm.is_finite()
                    || *maximum_plane_rms_mm < 0.
                    || !fit.plane_rms_mm.is_finite()
                    || fit.plane_rms_mm < 0.
                    || fit.plane_rms_mm > *maximum_plane_rms_mm
                    || !length.is_finite()
                    || (length - 1.).abs() > 1e-8
                {
                    return Err("invalid saved circle plane settings or normal".into());
                }
                (
                    fit.center_mm,
                    fit.radius_mm,
                    fit.diameter_mm,
                    fit.geometric_rms_mm,
                    fit.maximum_absolute_residual_mm,
                    fit.design_condition_number,
                    fit.points,
                    3,
                )
            }
        };
        if !center.iter().all(|v| v.is_finite())
            || ![radius, diameter, rms, maximum, condition]
                .iter()
                .all(|v| v.is_finite())
            || radius <= 0.
            || diameter != 2. * radius
            || rms < 0.
            || rms > maximum + 1e-12 * maximum.max(f64::MIN_POSITIVE)
            || condition < 1.
            || condition >= 1e8
            || count != self.source_points_mm.len()
            || count < minimum
        {
            return Err("invalid measurement snapshot result".into());
        }
        Ok(())
    }
}
