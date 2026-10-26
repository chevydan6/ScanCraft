//! Explicit right-handed manual coordinate frames; no scanner calibration changes.
use crate::{
    Cloud, Point,
    work::{Stage, WorkControl},
};
use nalgebra::{Matrix3, Vector3};
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame {
    pub origin_mm: [f64; 3],
    pub axes: [[f64; 3]; 3],
}
fn direction(v: [f64; 3]) -> Result<Vector3<f64>, String> {
    let scale = v.iter().map(|x| x.abs()).fold(0_f64, f64::max);
    if !v.iter().all(|x| x.is_finite()) || scale == 0. {
        return Err("frame direction is zero or unrepresentable".into());
    }
    Ok(Vector3::from(v.map(|x| x / scale)).normalize())
}
impl Frame {
    pub fn at_origin(origin_mm: [f64; 3]) -> Result<Self, String> {
        let f = Self {
            origin_mm,
            axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        };
        f.validate()?;
        Ok(f)
    }
    /// +X follows x_point-origin; xy_point chooses the positive XY half-plane.
    /// Nearly collinear references (sin angle <= 1e-8) are rejected.
    pub fn from_points(
        origin: [f64; 3],
        x_point: [f64; 3],
        xy_point: [f64; 3],
    ) -> Result<Self, String> {
        if !origin
            .iter()
            .chain(x_point.iter())
            .chain(xy_point.iter())
            .all(|v| v.is_finite())
        {
            return Err("frame references must be finite".into());
        }
        let x = direction(std::array::from_fn(|a| x_point[a] - origin[a]))?;
        let reference = direction(std::array::from_fn(|a| xy_point[a] - origin[a]))?;
        let cross = x.cross(&reference);
        if cross.norm() <= 1e-8 {
            return Err(
                "frame reference points are collinear or insufficiently separated in angle".into(),
            );
        }
        let z = cross.normalize();
        let y = z.cross(&x).normalize();
        let f = Self {
            origin_mm: origin,
            axes: [x.into(), y.into(), z.into()],
        };
        f.validate()?;
        Ok(f)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self
            .origin_mm
            .iter()
            .chain(self.axes.iter().flatten())
            .all(|v| v.is_finite())
        {
            return Err("frame must be finite".into());
        }
        let r = Matrix3::from_rows(&self.axes.map(nalgebra::RowVector3::from));
        let orthogonality = (r * r.transpose() - Matrix3::identity()).norm();
        let determinant = r.determinant();
        if !orthogonality.is_finite()
            || !determinant.is_finite()
            || orthogonality > 1e-10
            || (determinant - 1.).abs() > 1e-10
        {
            return Err("frame must be orthonormal and right-handed".into());
        }
        Ok(())
    }
    fn rotate(&self, v: [f64; 3]) -> Result<[f64; 3], String> {
        if !v.iter().all(|x| x.is_finite()) {
            return Err("coordinate difference overflow".into());
        }
        if self.axes == [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]] {
            return Ok(v);
        }
        let scale = v.iter().map(|x| x.abs()).fold(0_f64, f64::max);
        if scale == 0. {
            return Ok([0.; 3]);
        }
        let output = std::array::from_fn(|a| {
            (0..3)
                .map(|b| self.axes[a][b] * (v[b] / scale))
                .sum::<f64>()
                * scale
        });
        if !output.iter().all(|x| x.is_finite()) {
            return Err("coordinate frame result is unrepresentable".into());
        }
        Ok(output)
    }
    /// All positions transform as R*(p-origin), normals as R*n. No rescaling.
    pub fn apply(&self, source: &Cloud, work: &WorkControl) -> Result<Cloud, String> {
        self.validate()?;
        source.validate_with(work)?;
        let mut points = Vec::with_capacity(source.points.len());
        for (i, p) in source.points.iter().enumerate() {
            if i % 1024 == 0 {
                work.update(Stage::Processing, i, source.points.len())?;
            }
            let position =
                self.rotate(std::array::from_fn(|a| p.position[a] - self.origin_mm[a]))?;
            let normal = p.normal.map(|n| self.rotate(n)).transpose()?;
            points.push(Point {
                position,
                normal,
                color: p.color,
            });
        }
        work.check()?;
        Ok(Cloud { points })
    }
}
