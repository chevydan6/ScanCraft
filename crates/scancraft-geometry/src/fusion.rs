//! Deterministic bounded voxel-centroid fusion, not a TSDF or surface reconstruction.
use crate::{
    Cloud, Point,
    registration::Rigid,
    tracking::{TrackingState, TrackingUpdate},
    work::{Stage, WorkControl},
};
use nalgebra::Vector3;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
struct Voxel {
    count: u64,
    position: [f64; 3],
    color_count: u64,
    color: [f64; 3],
    normal_count: u64,
    normal: [f64; 3],
}
fn average(mean: &mut [f64; 3], count: &mut u64, value: [f64; 3]) -> Result<(), String> {
    let next = count
        .checked_add(1)
        .ok_or("voxel observation count overflow")?;
    let old_weight = *count as f64 / next as f64;
    for i in 0..3 {
        mean[i] = mean[i] * old_weight + value[i] / next as f64;
    }
    if !mean.iter().all(|v| v.is_finite()) {
        return Err("voxel mean overflow".into());
    }
    *count = next;
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FusionUpdate {
    pub frame_id: u64,
    pub input_points: usize,
    pub touched_voxels: usize,
    pub new_voxels: usize,
    pub total_voxels: usize,
}
pub struct VoxelFusion {
    voxel_mm: f64,
    maximum_voxels: usize,
    voxels: BTreeMap<[i64; 3], Voxel>,
    last_frame: Option<u64>,
}
impl VoxelFusion {
    pub fn new(voxel_mm: f64, maximum_voxels: usize) -> Result<Self, String> {
        if !voxel_mm.is_finite() || voxel_mm <= 0. || !(1..=1_000_000).contains(&maximum_voxels) {
            return Err("invalid fusion voxel size or capacity".into());
        }
        Ok(Self {
            voxel_mm,
            maximum_voxels,
            voxels: BTreeMap::new(),
            last_frame: None,
        })
    }
    pub fn len(&self) -> usize {
        self.voxels.len()
    }
    pub fn is_empty(&self) -> bool {
        self.voxels.is_empty()
    }
    pub fn last_frame(&self) -> Option<u64> {
        self.last_frame
    }
    /// Reject lost/rejected tracking observations; this is a consistency guard,
    /// not independent validation of a caller-supplied TrackingUpdate.
    pub fn integrate_tracked(
        &mut self,
        cloud: &Cloud,
        update: &TrackingUpdate,
        work: &WorkControl,
    ) -> Result<FusionUpdate, String> {
        if !update.accepted || update.state != TrackingState::Tracking {
            return Err("cannot fuse an unaccepted tracking frame".into());
        }
        let pose = update
            .world_from_frame
            .as_ref()
            .ok_or("accepted tracking update has no pose")?;
        self.integrate(update.frame_id, cloud, pose, work)
    }
    /// Merge a complete frame or nothing. Staging is limited to affected voxels.
    /// Caller must supply an accepted alignment in this fusion session's world.
    pub fn integrate(
        &mut self,
        frame_id: u64,
        cloud: &Cloud,
        pose: &Rigid,
        work: &WorkControl,
    ) -> Result<FusionUpdate, String> {
        work.check()?;
        if cloud.points.is_empty()
            || cloud.points.len() > 100_000
            || self.last_frame.is_some_and(|id| frame_id <= id)
        {
            return Err("fusion requires 1..=100000 points and increasing frame IDs".into());
        }
        pose.validate()?;
        cloud.validate_with(work)?;
        let mut staged: BTreeMap<[i64; 3], Voxel> = BTreeMap::new();
        let mut new_voxels = 0;
        for (i, point) in cloud.points.iter().enumerate() {
            work.update(Stage::Processing, i, cloud.points.len())?;
            let position = pose.apply(point.position);
            let mut key = [0i64; 3];
            for axis in 0..3 {
                let cell = (position[axis] / self.voxel_mm).floor();
                if !cell.is_finite() || cell < i64::MIN as f64 || cell >= -(i64::MIN as f64) {
                    return Err("fusion position exceeds voxel index range".into());
                }
                key[axis] = cell as i64;
            }
            if let std::collections::btree_map::Entry::Vacant(entry) = staged.entry(key) {
                let voxel = if let Some(existing) = self.voxels.get(&key) {
                    existing.clone()
                } else {
                    new_voxels += 1;
                    if self.voxels.len() + new_voxels > self.maximum_voxels {
                        return Err("fusion voxel capacity exceeded".into());
                    }
                    Voxel::default()
                };
                entry.insert(voxel);
            }
            let voxel = staged.get_mut(&key).unwrap();
            average(&mut voxel.position, &mut voxel.count, position)?;
            if let Some(color) = point.color {
                average(
                    &mut voxel.color,
                    &mut voxel.color_count,
                    color.map(f64::from),
                )?;
            }
            if let Some(normal) = point.normal {
                let n = Vector3::from(normal);
                let length = n.norm();
                if !length.is_finite() || length <= 1e-12 {
                    return Err("fusion requires finite nonzero normals".into());
                }
                let rotated = pose.rotation * (n / length);
                average(&mut voxel.normal, &mut voxel.normal_count, rotated.into())?;
            }
        }
        work.check()?;
        let report = FusionUpdate {
            frame_id,
            input_points: cloud.points.len(),
            touched_voxels: staged.len(),
            new_voxels,
            total_voxels: self.voxels.len() + new_voxels,
        };
        // No fallible operations after this boundary. Cancellation before it is atomic.
        self.voxels.extend(staged);
        self.last_frame = Some(frame_id);
        Ok(report)
    }
    /// Deterministic lexicographic voxel order; one centroid per voxel. Missing
    /// attributes do not dilute present colors/normals. Opposing normals can cancel.
    pub fn cloud(&self, work: &WorkControl) -> Result<Cloud, String> {
        let mut points = Vec::with_capacity(self.voxels.len());
        for (i, voxel) in self.voxels.values().enumerate() {
            work.update(Stage::Processing, i, self.voxels.len())?;
            let n = Vector3::from(voxel.normal);
            let length = n.norm();
            points.push(Point {
                position: voxel.position,
                color: (voxel.color_count > 0)
                    .then(|| voxel.color.map(|v| v.round().clamp(0., 255.) as u8)),
                normal: (voxel.normal_count > 0 && length > 1e-12).then(|| (n / length).into()),
            });
        }
        work.check()?;
        Ok(Cloud { points })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tracking::{Tracker, TrackingConfig};
    use nalgebra::Matrix3;
    fn point(position: [f64; 3]) -> Point {
        Point {
            position,
            color: None,
            normal: None,
        }
    }
    #[test]
    fn weighted_centroids_attributes_and_signed_voxel_order_are_analytic() {
        let mut fusion = VoxelFusion::new(1., 10).unwrap();
        let work = WorkControl::default();
        let mut a = point([0.1, 0.2, 0.3]);
        a.color = Some([10, 20, 30]);
        a.normal = Some([1., 0., 0.]);
        let mut b = point([0.3, 0.4, 0.5]);
        b.color = Some([30, 40, 50]);
        b.normal = Some([-1., 0., 0.]);
        fusion
            .integrate(
                0,
                &Cloud {
                    points: vec![a, b, point([-0.1, 0., 0.])],
                },
                &Rigid::identity(),
                &work,
            )
            .unwrap();
        fusion
            .integrate(
                1,
                &Cloud {
                    points: vec![point([0.5, 0.6, 0.7])],
                },
                &Rigid::identity(),
                &work,
            )
            .unwrap();
        let cloud = fusion.cloud(&work).unwrap();
        assert_eq!(cloud.points.len(), 2);
        assert_eq!(cloud.points[0].position, [-0.1, 0., 0.]);
        for (actual, expected) in cloud.points[1].position.into_iter().zip([0.3, 0.4, 0.5]) {
            assert!((actual - expected).abs() < 1e-14);
        }
        assert_eq!(cloud.points[1].color, Some([20, 30, 40]));
        assert_eq!(cloud.points[1].normal, None);
    }
    #[test]
    fn normals_rotate_without_translation_and_invalid_frames_are_atomic() {
        let work = WorkControl::default();
        let mut fusion = VoxelFusion::new(1., 2).unwrap();
        let pose = Rigid {
            rotation: Matrix3::new(0., -1., 0., 1., 0., 0., 0., 0., 1.),
            translation: Vector3::new(10., 0., 0.),
        };
        let mut p = point([1.1, 2.1, 3.1]);
        p.normal = Some([2., 0., 0.]);
        fusion
            .integrate(0, &Cloud { points: vec![p] }, &pose, &work)
            .unwrap();
        let before = fusion.cloud(&work).unwrap();
        assert_eq!(before.points[0].normal, Some([0., 1., 0.]));
        assert!((before.points[0].position[0] - 7.9).abs() < 1e-14);
        let mut bad = point([7.9, 1.1, 3.1]);
        bad.normal = Some([0.; 3]);
        let invalid = Cloud {
            points: vec![point([7.8, 1.2, 3.2]), bad],
        };
        assert!(
            fusion
                .integrate(1, &invalid, &Rigid::identity(), &work)
                .is_err()
        );
        assert_eq!(fusion.cloud(&work).unwrap(), before);
        assert_eq!(fusion.last_frame(), Some(0));
        let too_many = Cloud {
            points: vec![point([10.; 3]), point([20.; 3])],
        };
        assert!(
            fusion
                .integrate(1, &too_many, &Rigid::identity(), &work)
                .is_err()
        );
        assert_eq!(fusion.cloud(&work).unwrap(), before);
        let mut bad_pose = Rigid::identity();
        bad_pose.rotation[(0, 0)] = 2.;
        assert!(
            fusion
                .integrate(
                    1,
                    &Cloud {
                        points: vec![point([0.; 3])]
                    },
                    &bad_pose,
                    &work
                )
                .is_err()
        );
        let huge = Cloud {
            points: vec![point([f64::MAX, 0., 0.])],
        };
        assert!(
            fusion
                .integrate(1, &huge, &Rigid::identity(), &work)
                .is_err()
        );
        let cancelled = WorkControl::default();
        cancelled.cancel();
        assert!(
            fusion
                .integrate(
                    1,
                    &Cloud {
                        points: vec![point([0.; 3])]
                    },
                    &Rigid::identity(),
                    &cancelled
                )
                .is_err()
        );
        assert_eq!(fusion.cloud(&work).unwrap(), before);
        assert_eq!(fusion.last_frame(), Some(0));
        assert!(
            fusion
                .integrate(
                    0,
                    &Cloud {
                        points: vec![point([0.; 3])]
                    },
                    &Rigid::identity(),
                    &work
                )
                .is_err()
        );
    }
    #[test]
    fn tracked_sequence_fuses_to_world_and_never_fuses_lost_frames() {
        let world = [
            [0.25, 0.25, 0.25],
            [10.25, 0.25, 0.25],
            [0.25, 20.25, 0.25],
            [0.25, 0.25, 30.25],
            [4.25, 5.25, 8.25],
            [-6.75, 2.25, 4.25],
        ];
        let mut tracker = Tracker::new(TrackingConfig {
            max_correspondence_mm: 2.,
            max_rms_mm: 0.01,
            minimum_overlap: 0.8,
            max_step_translation_mm: 1.,
            max_step_rotation_rad: 0.1,
            iterations: 20,
            tolerance_mm: 1e-8,
        })
        .unwrap();
        let mut fusion = VoxelFusion::new(1., 100).unwrap();
        let work = WorkControl::default();
        for frame in 0..10 {
            let local: Vec<_> = world
                .iter()
                .map(|p| [p[0] - frame as f64 * 0.2, p[1], p[2]])
                .collect();
            let update = tracker.track(frame, &local, &work).unwrap();
            assert!(update.accepted);
            fusion
                .integrate_tracked(
                    &Cloud {
                        points: local.into_iter().map(point).collect(),
                    },
                    &update,
                    &work,
                )
                .unwrap();
        }
        assert_eq!(fusion.len(), world.len());
        let before = fusion.cloud(&work).unwrap();
        for p in &before.points {
            assert!(
                world
                    .iter()
                    .any(|q| (Vector3::from(p.position) - Vector3::from(*q)).norm() < 1e-10)
            );
        }
        let bad: Vec<_> = world.iter().map(|p| [p[0] + 1000., p[1], p[2]]).collect();
        let lost = tracker.track(10, &bad, &work).unwrap();
        assert!(!lost.accepted);
        assert!(
            fusion
                .integrate_tracked(
                    &Cloud {
                        points: bad.into_iter().map(point).collect()
                    },
                    &lost,
                    &work
                )
                .is_err()
        );
        assert_eq!(fusion.last_frame(), Some(9));
        assert_eq!(fusion.cloud(&work).unwrap(), before);
        let mut bytes = Vec::new();
        crate::write_ply(&before, &mut bytes).unwrap();
        assert_eq!(
            crate::formats::read_ply(std::io::Cursor::new(bytes), None).unwrap(),
            before
        );
    }
}
