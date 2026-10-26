//! Independent point-cloud primitives. All positions are explicitly millimeters.
use serde::{Deserialize, Serialize};
pub mod coordinate_frame;
pub mod cylinder;
pub mod fusion;
pub mod live_references;
pub mod measurement;
mod measurement_history;
pub mod measurement_snapshot;
pub mod mesh;
pub mod mesh_boundaries;
pub mod mesh_components;
pub mod mesh_edit;
pub mod mesh_fill;
pub mod mesh_normals;
pub mod mesh_orientation;
pub mod mesh_project;
pub mod mesh_section;
pub mod mesh_simplify;
pub mod mesh_split_fans;
mod mesh_vertex_links;
pub mod normal_edit;
pub mod plane_measurement;
pub mod point_plane;
pub mod project_origin;
pub mod recording_references;
pub mod sequence_references;
pub mod session;
pub mod spatial;
pub mod statistical_filter;
pub mod tracking;
pub mod work;
use std::collections::{BTreeMap, VecDeque};
use work::{Stage, WorkControl};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub position: [f64; 3],
    pub color: Option<[u8; 3]>,
    pub normal: Option<[f64; 3]>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cloud {
    pub points: Vec<Point>,
}
impl Cloud {
    pub fn validate(&self) -> Result<(), String> {
        self.validate_with(&WorkControl::default())
    }
    pub fn validate_with(&self, work: &WorkControl) -> Result<(), String> {
        work.update(Stage::Validating, 0, self.points.len())?;
        for (i, p) in self.points.iter().enumerate() {
            if i % 1024 == 0 {
                work.update(Stage::Validating, i, self.points.len())?;
            }
            if !p.position.iter().all(|x| x.is_finite()) {
                return Err(format!("non-finite point at index {i}"));
            }
            if p.normal.is_some_and(|n| !n.iter().all(|x| x.is_finite())) {
                return Err(format!("invalid normal at index {i}"));
            }
        }
        Ok(())
    }
    pub fn bounds(&self) -> Option<([f64; 3], [f64; 3])> {
        let first = self.points.first()?;
        let mut min = first.position;
        let mut max = min;
        for p in &self.points {
            for axis in 0..3 {
                min[axis] = min[axis].min(p.position[axis]);
                max[axis] = max[axis].max(p.position[axis]);
            }
        }
        Some((min, max))
    }
    pub fn select_box(&self, min: [f64; 3], max: [f64; 3]) -> Result<Vec<usize>, String> {
        self.select_box_with(min, max, &WorkControl::default())
    }
    pub fn select_box_with(
        &self,
        min: [f64; 3],
        max: [f64; 3],
        work: &WorkControl,
    ) -> Result<Vec<usize>, String> {
        self.validate_with(work)?;
        if (0..3).any(|a| !min[a].is_finite() || !max[a].is_finite() || min[a] > max[a]) {
            return Err("invalid selection box".into());
        }
        let mut selected = Vec::new();
        for (i, p) in self.points.iter().enumerate() {
            if i % 1024 == 0 {
                work.update(Stage::Processing, i, self.points.len())?;
            }
            if (0..3).all(|a| p.position[a] >= min[a] && p.position[a] <= max[a]) {
                selected.push(i);
            }
        }
        work.check()?;
        Ok(selected)
    }
    pub fn trimmed_with(
        &self,
        indices: &[usize],
        keep: bool,
        work: &WorkControl,
    ) -> Result<Self, String> {
        self.validate_with(work)?;
        let mut mask = vec![false; self.points.len()];
        for (i, index) in indices.iter().enumerate() {
            if i % 1024 == 0 {
                work.update(Stage::Indexing, i, indices.len())?;
            }
            let marked = mask.get_mut(*index).ok_or("selection index out of range")?;
            if *marked {
                return Err("duplicate selection index".into());
            }
            *marked = true;
        }
        let mut points = Vec::new();
        for (i, p) in self.points.iter().enumerate() {
            if i % 1024 == 0 {
                work.update(Stage::Processing, i, self.points.len())?;
            }
            if mask[i] == keep {
                points.push(p.clone());
            }
        }
        work.check()?;
        Ok(Self { points })
    }
    pub fn voxel_downsample(&self, size_mm: f64) -> Result<Self, String> {
        self.voxel_downsample_with(size_mm, &WorkControl::default())
    }
    pub fn voxel_downsample_with(&self, size_mm: f64, work: &WorkControl) -> Result<Self, String> {
        self.validate_with(work)?;
        if !size_mm.is_finite() || size_mm <= 0. {
            return Err("voxel size must be positive and finite".into());
        }
        let mut voxels: BTreeMap<[i64; 3], Vec<&Point>> = BTreeMap::new();
        for (i, p) in self.points.iter().enumerate() {
            if i % 1024 == 0 {
                work.update(Stage::Indexing, i, self.points.len())?;
            }
            let mut key = [0; 3];
            for (a, k) in key.iter_mut().enumerate() {
                let q = (p.position[a] / size_mm).floor();
                if !q.is_finite() || q <= i64::MIN as f64 || q >= i64::MAX as f64 {
                    return Err("coordinate exceeds voxel index range".into());
                }
                *k = q as i64;
            }
            voxels.entry(key).or_default().push(p);
        }
        let mut points = Vec::with_capacity(voxels.len());
        for (i, group) in voxels.values().enumerate() {
            if i % 1024 == 0 {
                work.update(Stage::Processing, i, voxels.len())?;
            }
            let n = group.len() as f64;
            let mut position = [0.; 3];
            let mut color_sum = [0u64; 3];
            let mut colors = 0u64;
            for (j, p) in group.iter().enumerate() {
                if j % 1024 == 0 {
                    work.check()?;
                }
                for (a, x) in position.iter_mut().enumerate() {
                    *x += p.position[a] / n;
                }
                if let Some(color) = p.color {
                    for a in 0..3 {
                        color_sum[a] += u64::from(color[a]);
                    }
                    colors += 1;
                }
            }
            if !position.iter().all(|x| x.is_finite()) {
                return Err("voxel centroid overflow".into());
            }
            points.push(Point {
                position,
                color: if colors == group.len() as u64 {
                    Some(color_sum.map(|x| ((x + colors / 2) / colors) as u8))
                } else {
                    None
                },
                normal: None,
            });
        }
        Ok(Self { points })
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Editor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    live_session: Option<live_references::LiveSession>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sequence: Option<sequence_references::Sequence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    recordings: Option<recording_references::Recordings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    origin: Option<project_origin::Origin>,
    pub cloud: Cloud,
    undo: VecDeque<Cloud>,
    redo: Vec<Cloud>,
    history_limit: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    measurements: Vec<measurement_snapshot::Snapshot>,
    #[serde(default, skip_serializing_if = "VecDeque::is_empty")]
    measurement_undo: VecDeque<measurement_history::Change>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    measurement_redo: Vec<measurement_history::Change>,
}
impl Editor {
    pub fn validate_history(&self) -> Result<(), String> {
        if let Some(live) = &self.live_session {
            live.validate()?;
            if self.origin.is_some() || self.recordings.is_some() || self.sequence.is_some() {
                return Err(
                    "live-session and other reconstruction provenance cannot coexist".into(),
                );
            }
        }
        if let Some(sequence) = &self.sequence {
            sequence.validate()?;
            if self.origin.is_some() || self.recordings.is_some() {
                return Err("single-frame and sequence provenance cannot coexist".into());
            }
        }
        if let Some(origin) = &self.origin {
            origin.validate()?;
        }
        if let Some(recordings) = &self.recordings {
            recordings.validate(
                self.origin
                    .as_ref()
                    .ok_or("recording references require reconstruction provenance")?,
            )?;
        }
        measurement_history::validate(
            &self.measurements,
            &self.measurement_undo,
            &self.measurement_redo,
        )?;
        if self.measurements.len() > 16 {
            return Err("project exceeds 16 saved measurement snapshots".into());
        }
        let mut labels = std::collections::HashSet::new();
        for snapshot in &self.measurements {
            snapshot.validate()?;
            if !labels.insert(&snapshot.label) {
                return Err("duplicate measurement snapshot label".into());
            }
        }
        if self.history_limit > 32 || self.undo.len() + self.redo.len() > self.history_limit {
            return Err("invalid edit history bound".into());
        }
        self.cloud.validate()?;
        for c in self.undo.iter().chain(self.redo.iter()) {
            c.validate()?;
        }
        Ok(())
    }

    pub fn new(cloud: Cloud, history_limit: usize) -> Result<Self, String> {
        cloud.validate()?;
        if history_limit > 32 {
            return Err("history limit exceeds 32".into());
        }
        Ok(Self {
            live_session: None,
            sequence: None,
            origin: None,
            recordings: None,
            cloud,
            undo: VecDeque::new(),
            redo: Vec::new(),
            history_limit,
            measurements: Vec::new(),
            measurement_undo: VecDeque::new(),
            measurement_redo: Vec::new(),
        })
    }
    /// Copy the complete editable state with a new descriptive source location.
    /// Callers must separately verify/copy sources; this performs no filesystem I/O.
    pub fn copied_with_source_directory(&self, directory: String) -> Result<Self, String> {
        self.validate_history()?;
        let mut origin = self
            .origin
            .clone()
            .ok_or("project has no reconstruction provenance")?;
        origin.bundle_directory_display = directory;
        origin.validate()?;
        let mut copy = self.clone();
        copy.origin = Some(origin);
        Ok(copy)
    }
    pub fn recordings(&self) -> Option<&recording_references::Recordings> {
        self.recordings.as_ref()
    }
    /// Copy descriptive references after the caller explicitly verifies their bytes.
    /// This method validates metadata only and performs no filesystem access.
    pub fn copied_with_recordings(
        &self,
        references: recording_references::Recordings,
    ) -> Result<Self, String> {
        self.validate_history()?;
        references.validate(
            self.origin
                .as_ref()
                .ok_or("recording references require reconstruction provenance")?,
        )?;
        let mut copy = self.clone();
        copy.recordings = Some(references);
        Ok(copy)
    }
    pub fn origin(&self) -> Option<&project_origin::Origin> {
        self.origin.as_ref()
    }
    /// Attach once to a fresh reconstructed document. Geometry edits retain it;
    /// it describes initial inputs, not current per-point correspondence.
    pub fn attach_origin(&mut self, origin: project_origin::Origin) -> Result<(), String> {
        origin.validate()?;
        if self.origin.is_some() || self.sequence.is_some() || self.live_session.is_some() {
            return Err("initial project origin is already attached".into());
        }
        if !self.undo.is_empty()
            || !self.redo.is_empty()
            || !self.measurements.is_empty()
            || self.has_measurement_history()
        {
            return Err("attach initial provenance before editing or measuring".into());
        }
        self.origin = Some(origin);
        Ok(())
    }
    /// Initial multi-frame references retained independently of geometry history.
    pub fn sequence(&self) -> Option<&sequence_references::Sequence> {
        self.sequence.as_ref()
    }
    /// Attach once before edits/measurements. This performs metadata validation
    /// only; loading/reopening never follows the external directory reference.
    pub fn attach_sequence(
        &mut self,
        sequence: sequence_references::Sequence,
    ) -> Result<(), String> {
        sequence.validate()?;
        if self.origin.is_some()
            || self.recordings.is_some()
            || self.sequence.is_some()
            || self.live_session.is_some()
            || !self.undo.is_empty()
            || !self.redo.is_empty()
            || !self.measurements.is_empty()
            || self.has_measurement_history()
        {
            return Err("attach sequence provenance once before editing or measuring".into());
        }
        self.sequence = Some(sequence);
        Ok(())
    }
    pub fn live_session(&self) -> Option<&live_references::LiveSession> {
        self.live_session.as_ref()
    }
    /// Attach initial live-session metadata once, before edits/measurements; no filesystem I/O.
    pub fn attach_live_session(
        &mut self,
        live: live_references::LiveSession,
    ) -> Result<(), String> {
        self.validate_history()?;
        live.validate()?;
        if self.live_session.is_some()
            || self.sequence.is_some()
            || self.origin.is_some()
            || self.recordings.is_some()
            || !self.undo.is_empty()
            || !self.redo.is_empty()
            || !self.measurements.is_empty()
            || self.has_measurement_history()
        {
            return Err("attach live-session provenance once before editing or measuring".into());
        }
        self.live_session = Some(live);
        Ok(())
    }
    pub fn measurements(&self) -> &[measurement_snapshot::Snapshot] {
        &self.measurements
    }
    pub fn add_measurement(
        &mut self,
        snapshot: measurement_snapshot::Snapshot,
    ) -> Result<(), String> {
        snapshot.validate()?;
        if self.measurements.len() >= 16
            || self.measurements.iter().any(|s| s.label == snapshot.label)
        {
            return Err(
                "measurement label must be unique and at most 16 snapshots may be saved".into(),
            );
        }
        let index = self.measurements.len();
        let inverse = measurement_history::apply(
            &mut self.measurements,
            measurement_history::Change::Insert { index, snapshot },
        )?;
        self.commit_measurement_change(inverse);
        Ok(())
    }
    fn commit_measurement_change(&mut self, inverse: measurement_history::Change) {
        self.measurement_undo.push_back(inverse);
        while self.measurement_undo.len() > 16 {
            self.measurement_undo.pop_front();
        }
        self.measurement_redo.clear();
    }
    pub fn has_measurement_history(&self) -> bool {
        !self.measurement_undo.is_empty() || !self.measurement_redo.is_empty()
    }
    pub fn can_undo_measurement(&self) -> bool {
        !self.measurement_undo.is_empty()
    }
    pub fn can_redo_measurement(&self) -> bool {
        !self.measurement_redo.is_empty()
    }
    pub fn rename_measurement(&mut self, index: usize, label: String) -> Result<(), String> {
        let inverse = measurement_history::apply(
            &mut self.measurements,
            measurement_history::Change::Rename { index, label },
        )?;
        self.commit_measurement_change(inverse);
        Ok(())
    }
    pub fn remove_measurement(&mut self, index: usize) -> Result<(), String> {
        let label = self
            .measurements
            .get(index)
            .ok_or("measurement index out of bounds")?
            .label
            .clone();
        let inverse = measurement_history::apply(
            &mut self.measurements,
            measurement_history::Change::Remove { index, label },
        )?;
        self.commit_measurement_change(inverse);
        Ok(())
    }
    pub fn undo_measurement(&mut self) -> Result<bool, String> {
        let Some(change) = self.measurement_undo.back().cloned() else {
            return Ok(false);
        };
        let inverse = measurement_history::apply(&mut self.measurements, change)?;
        self.measurement_undo.pop_back();
        self.measurement_redo.push(inverse);
        Ok(true)
    }
    pub fn redo_measurement(&mut self) -> Result<bool, String> {
        let Some(change) = self.measurement_redo.last().cloned() else {
            return Ok(false);
        };
        let inverse = measurement_history::apply(&mut self.measurements, change)?;
        self.measurement_redo.pop();
        self.measurement_undo.push_back(inverse);
        Ok(true)
    }
    pub fn replace(&mut self, next: Cloud) -> Result<(), String> {
        next.validate()?;
        let previous = std::mem::replace(&mut self.cloud, next);
        if self.history_limit > 0 {
            self.undo.push_back(previous);
            while self.undo.len() > self.history_limit {
                self.undo.pop_front();
            }
        }
        self.redo.clear();
        Ok(())
    }
    pub fn trim(&mut self, indices: &[usize], keep: bool) -> Result<(), String> {
        let next = self
            .cloud
            .trimmed_with(indices, keep, &WorkControl::default())?;
        self.replace(next)
    }
    pub fn undo_cloud(&self) -> Option<&Cloud> {
        self.undo.back()
    }
    pub fn redo_cloud(&self) -> Option<&Cloud> {
        self.redo.last()
    }
    pub fn undo(&mut self) -> bool {
        if let Some(previous) = self.undo.pop_back() {
            self.redo.push(std::mem::replace(&mut self.cloud, previous));
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo.pop() {
            self.undo
                .push_back(std::mem::replace(&mut self.cloud, next));
            true
        } else {
            false
        }
    }
}

pub fn distance_mm(a: [f64; 3], b: [f64; 3]) -> Result<f64, String> {
    if !a.iter().chain(b.iter()).all(|x| x.is_finite()) {
        return Err("measurement coordinates must be finite".into());
    }
    let d = (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]);
    if !d.is_finite() {
        return Err("distance overflow".into());
    }
    Ok(d)
}

/// ASCII PLY with full f64 round-trip precision and explicit units comment.
pub fn write_ply(cloud: &Cloud, mut output: impl std::io::Write) -> std::io::Result<()> {
    cloud.validate().map_err(std::io::Error::other)?;
    // All points must carry a given attribute to export that property faithfully.
    let colors = cloud.points.iter().all(|p| p.color.is_some());
    let normals = cloud.points.iter().all(|p| p.normal.is_some());
    if (cloud.points.iter().any(|p| p.color.is_some()) != colors
        || cloud.points.iter().any(|p| p.normal.is_some()) != normals)
        && !cloud.points.is_empty()
    {
        return Err(std::io::Error::other(
            "mixed attributes cannot be preserved by this PLY exporter",
        ));
    }

    writeln!(
        output,
        "ply\nformat ascii 1.0\ncomment units millimeters\nelement vertex {}\nproperty double x\nproperty double y\nproperty double z",
        cloud.points.len()
    )?;
    if normals {
        writeln!(
            output,
            "property double nx\nproperty double ny\nproperty double nz"
        )?;
    }
    if colors {
        writeln!(
            output,
            "property uchar red\nproperty uchar green\nproperty uchar blue"
        )?;
    }
    writeln!(output, "end_header")?;
    for p in &cloud.points {
        write!(
            output,
            "{} {} {}",
            p.position[0], p.position[1], p.position[2]
        )?;
        if normals {
            let n = p.normal.unwrap();
            write!(output, " {} {} {}", n[0], n[1], n[2])?;
        }
        if colors {
            let c = p.color.unwrap();
            write!(output, " {} {} {}", c[0], c[1], c[2])?;
        }
        writeln!(output)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cloud(n: usize) -> Cloud {
        Cloud {
            points: (0..n)
                .map(|i| Point {
                    position: [i as f64, 0., 0.],
                    color: Some([1, 2, 3]),
                    normal: Some([0., 0., 1.]),
                })
                .collect(),
        }
    }
    #[test]
    fn exact_trim_undo_100_cycles() {
        let original = cloud(100);
        let mut e = Editor::new(original.clone(), 32).unwrap();
        for _ in 0..100 {
            e.trim(&[1, 3, 99], false).unwrap();
            assert_eq!(e.cloud.points.len(), 97);
            assert_eq!(e.cloud.points[1], original.points[2]);
            assert!(e.undo());
            assert_eq!(e.cloud, original);
            assert!(e.redo());
            assert_eq!(e.cloud.points.len(), 97);
            assert!(e.undo());
            assert_eq!(e.cloud, original);
        }
    }
    #[test]
    fn keep_and_invalid_selection() {
        let mut e = Editor::new(cloud(10), 32).unwrap();
        assert!(e.trim(&[11], false).is_err());
        assert!(e.trim(&[1, 1], false).is_err());
        assert_eq!(e.cloud.points.len(), 10);
        e.trim(&[2, 4], true).unwrap();
        assert_eq!(
            e.cloud
                .points
                .iter()
                .map(|p| p.position[0])
                .collect::<Vec<_>>(),
            [2., 4.]
        );
    }
    #[test]
    fn inclusive_box_boundaries() {
        let c = cloud(10);
        assert_eq!(c.select_box([2., 0., 0.], [4., 0., 0.]).unwrap(), [2, 3, 4]);
        assert!(c.select_box([5., 0., 0.], [4., 0., 0.]).is_err());
    }
    #[test]
    fn deterministic_voxels_negative_coordinates() {
        let mut c = cloud(4);
        c.points[0].position[0] = -0.1;
        c.points[1].position[0] = 0.1;
        c.points[2].position[0] = 0.2;
        c.points[3].position[0] = 1.1;
        let out = c.voxel_downsample(1.).unwrap();
        assert_eq!(out.points.len(), 3);
        assert!((out.points[1].position[0] - 0.15).abs() < 1e-15);
        assert!(out.points.iter().all(|p| p.normal.is_none()));
    }
    #[test]
    fn reject_nonfinite_geometry() {
        let mut c = cloud(1);
        c.points[0].position[0] = f64::NAN;
        assert!(c.validate().is_err());
        assert!(c.voxel_downsample(1.).is_err());
        assert!(write_ply(&c, Vec::new()).is_err());
        assert!(distance_mm([f64::INFINITY, 0., 0.], [0.; 3]).is_err());
    }
    #[test]
    fn measurement_units() {
        assert_eq!(distance_mm([0.; 3], [3., 4., 0.]).unwrap(), 5.);
    }
    #[test]
    fn ply_preserves_properties() {
        let mut out = Vec::new();
        write_ply(&cloud(2), &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("comment units millimeters"));
        assert!(s.contains("property double nx"));
        assert!(s.ends_with("1 0 0 0 0 1 1 2 3\n"));
    }
    #[test]
    fn bounded_history_and_redo_invalidation() {
        let mut e = Editor::new(cloud(10), 1).unwrap();
        e.trim(&[0], false).unwrap();
        e.trim(&[0], false).unwrap();
        assert!(e.undo());
        assert!(!e.undo());
        e.trim(&[], false).unwrap();
        assert!(!e.redo());
    }
}

pub mod filtering;
pub mod formats;
pub mod registration;
pub mod selection;

pub mod mesh_smooth;

pub mod mesh_import;
