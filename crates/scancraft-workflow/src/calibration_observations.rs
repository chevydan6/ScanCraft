//! Recorded bright-target observations with explicit units and retained raw lineage.
//! Connects existing native detection/shape/association primitives; never installs calibration.
use crate::{
    Error, artifact, bounded,
    calibration_bundle::{check, ensure_new},
    output::{publish, staging},
    recording_playback::Playback,
    recording_reconstruction::lineage,
};
use scancraft_calibration::target::{self, Point};
use scancraft_geometry::work::WorkControl;
use scancraft_image::{GrayImage, components, shape};
use scancraft_recording::{Writer, selection::SelectedFrame};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetUnits {
    Unverified,
    DeclaredMillimeters,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetPoint {
    pub pattern: [f64; 2],
    pub anchor: bool,
    /// Exact supplied coordinates for regular marks; anchors must use null.
    pub coordinates: Option<[f64; 2]>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub recording: PathBuf,
    pub index: usize,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Detection {
    pub threshold: u8,
    pub minimum_area: u32,
    pub maximum_area: u32,
    pub maximum_components: usize,
    pub anchor_area_above: u32,
    pub maximum_shape_rms_px: f64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub schema_version: u32,
    pub target_units: TargetUnits,
    pub target: Vec<TargetPoint>,
    pub left: Selection,
    pub right: Selection,
    pub maximum_host_delta_us: u64,
    pub detection: Detection,
    pub association: target::Config,
}
struct Analysis {
    report: Value,
    /// One entry per target index, retaining raw centroid only when its shape passes.
    accepted: Vec<Option<[f64; 2]>>,
}
fn validate(r: &Request) -> Result<(), Error> {
    if r.schema_version != 1
        || !(8..=128).contains(&r.target.len())
        || r.maximum_host_delta_us > 1_000_000
        || !r.detection.maximum_shape_rms_px.is_finite()
        || !(0. ..=1e5).contains(&r.detection.maximum_shape_rms_px)
        || r.detection.maximum_shape_rms_px == 0.
        || r.detection.anchor_area_above < r.detection.minimum_area
        || r.detection.anchor_area_above >= r.detection.maximum_area
    {
        return Err("invalid recorded-target request limits".into());
    }
    for (i, point) in r.target.iter().enumerate() {
        if point.anchor != point.coordinates.is_none() {
            return Err(
                "regular targets require coordinates; anchors require null coordinates".into(),
            );
        }
        if let Some(xy) = point.coordinates
            && (xy.iter().any(|v| !v.is_finite() || v.abs() > 1e9)
                || r.target[..i].iter().any(|p| p.coordinates == Some(xy)))
        {
            return Err("invalid or duplicate supplied target coordinates".into());
        }
    }
    Ok(())
}
fn analyze(
    frame: &SelectedFrame,
    request: &Request,
    work: &WorkControl,
    cancel: &dyn Fn() -> bool,
) -> Result<Analysis, Error> {
    check(work, cancel)?;
    let image = GrayImage::from_frame(&frame.profile, &frame.frame)?;
    let c = &request.detection;
    let found = components::detect(
        image,
        components::Config {
            threshold: c.threshold,
            minimum_area: c.minimum_area,
            maximum_area: c.maximum_area,
            maximum_components: c.maximum_components,
        },
        work,
    )?;
    if found.observations.len() > 256 {
        return Err("target association limited to 256 components per image".into());
    }
    let mut details = Vec::new();
    let mut passed = Vec::new();
    for o in &found.observations {
        check(work, cancel)?;
        let measured = shape::measure(image, c.threshold, o, work);
        let ok = measured
            .as_ref()
            .is_ok_and(|s| s.rms_sampson <= c.maximum_shape_rms_px);
        passed.push(ok);
        let measurement = match measured {
            Ok(fit) => json!({"fit":fit, "rms_gate_passed":ok}),
            Err(e) => json!({"error":e, "rms_gate_passed":false}),
        };
        details.push(
            json!({"pixel":o.pixel,"area":o.area,"bounds_inclusive":o.bounds,
            "peak":o.peak,"saturated_pixels":o.saturated_pixels,"shape":measurement}),
        );
    }
    let model: Vec<_> = request
        .target
        .iter()
        .map(|t| Point {
            position: t.pattern,
            anchor: t.anchor,
        })
        .collect();
    let observed: Vec<_> = found
        .observations
        .iter()
        .map(|o| Point {
            position: o.pixel,
            anchor: o.area > c.anchor_area_above,
        })
        .collect();
    let associations = target::associate(&model, &observed, request.association, || {
        work.check().is_err() || cancel()
    })?;
    check(work, cancel)?;
    let mut accepted = vec![None; request.target.len()];
    for m in &associations.matches {
        if !request.target[m.target_index].anchor && passed[m.observation_index] {
            accepted[m.target_index] = Some(found.observations[m.observation_index].pixel);
        }
    }
    Ok(Analysis {
        accepted,
        report: json!({"raw_observations":details,"association":associations,
        "components_examined":found.components_examined,"foreground_pixels":found.foreground_pixels,
        "area_rejections":found.rejected_area,"border_rejections":found.rejected_border,
        "centers":"binary area centroids; ellipse centers are diagnostics only",
        "physical_centers_verified":false}),
    })
}
/// Request-relative recording paths, complete recording integrity validation and
/// source guards. Equal nonmissing sensor PTS and bounded host delta are required,
/// but do not attest exposure synchronization. No existing output is overwritten.
pub fn run(
    input: &Path,
    target: &Path,
    work: &WorkControl,
    cancel: &dyn Fn() -> bool,
) -> Result<Value, Error> {
    check(work, cancel)?;
    ensure_new(target)?;
    let bytes = bounded(input, 1024 * 1024, work)?;
    let request: Request = serde_json::from_slice(&bytes)?;
    validate(&request)?;
    let parent = input
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let paths = [
        parent.join(&request.left.recording),
        parent.join(&request.right.recording),
    ];
    let mut sources = [
        Playback::open(&paths[0], work)?,
        Playback::open(&paths[1], work)?,
    ];
    let selected = [
        sources[0].select_raw(request.left.index, work)?,
        sources[1].select_raw(request.right.index, work)?,
    ];
    if selected[0].profile.interface != 0 || selected[1].profile.interface != 2 {
        return Err("recorded target pair requires left interface0 and right interface2".into());
    }
    let pts = [
        scancraft_core::uvc_pts(&selected[0].frame.metadata)?,
        scancraft_core::uvc_pts(&selected[1].frame.metadata)?,
    ];
    let delta = selected[0]
        .frame
        .host_timestamp_us
        .abs_diff(selected[1].frame.host_timestamp_us);
    if pts[0].is_none() || pts[0] != pts[1] || delta > request.maximum_host_delta_us {
        return Err(
            "recorded target pair requires equal sensor PTS and permitted host delta".into(),
        );
    }
    let analyses = [
        analyze(&selected[0], &request, work, cancel)?,
        analyze(&selected[1], &request, work, cancel)?,
    ];
    let mut views = [Vec::new(), Vec::new()];
    let mut stereo = Vec::new();
    for (i, t) in request.target.iter().enumerate() {
        if let Some(xy) = t.coordinates {
            for side in 0..2 {
                if let Some(pixel) = analyses[side].accepted[i] {
                    views[side].push(json!({"target_index":i,"coordinates":xy,"pixel":pixel}));
                }
            }
            if let (Some(left), Some(right)) = (analyses[0].accepted[i], analyses[1].accepted[i]) {
                stereo.push(json!({"target_index":i,"coordinates":xy,"left_pixel":left,"right_pixel":right}));
            }
        }
    }
    let lineage = [
        lineage(&paths[0], sources[0].version(), &selected[0])?,
        lineage(&paths[1], sources[1].version(), &selected[1])?,
    ];
    let report = json!({"schema_version":1,"target_units":request.target_units,
        "units_verified":false,"physical_accuracy_qualified":false,"calibration_activated":false,
        "source_device_binding_verified":false,"synchronization_verified":false,
        "sensor_pts":pts[0],"host_delta_us":delta,"source_recordings":lineage,
        "cameras":[analyses[0].report,analyses[1].report],"views":views,"stereo_observations":stereo,
        "minimum_fit_point_count_met":views.iter().all(|v|v.len()>=4)&&stereo.len()>=4});
    for s in &sources {
        s.check_source()?;
    }
    check(work, cancel)?;
    let partial = staging(target)?;
    let result = (|| -> Result<Value, Error> {
        let mut artifacts = vec![
            artifact(&partial, "request.json", work, |o| {
                o.write_all(&bytes)?;
                Ok(())
            })?,
            artifact(&partial, "measurements.json", work, |o| {
                serde_json::to_writer_pretty(o, &report)?;
                Ok(())
            })?,
        ];
        for (name, frame) in ["left-frame.raptor", "right-frame.raptor"].into_iter().zip(&selected) {
            check(work, cancel)?;
            artifacts.push(artifact(&partial, name, work, |o| {
                let mut writer = Writer::with_context(o, frame.profile.clone(), frame.context.clone().unwrap_or_default())?;
                writer.push(&frame.frame)?;
                writer.finish()?;
                Ok(())
            })?);
        }
        let fit_ready = request.target_units == TargetUnits::DeclaredMillimeters
            && views.iter().all(|v| v.len() >= 4) && stereo.len() >= 4;
        if fit_ready {
            for (name, view) in ["left-view.json", "right-view.json"].into_iter().zip(&views) {
                let observations: Vec<_> = view.iter().map(|o| json!({"target_xy_mm":o["coordinates"],"pixel":o["pixel"]})).collect();
                artifacts.push(artifact(&partial, name, work, |o| {
                    serde_json::to_writer_pretty(o, &json!({"observations":observations}))?;
                    Ok(())
                })?);
            }
            let observations: Vec<_> = stereo.iter().map(|o| json!({"target_xy_mm":o["coordinates"],"left_pixel":o["left_pixel"],"right_pixel":o["right_pixel"]})).collect();
            artifacts.push(artifact(&partial, "stereo-view.json", work, |o| {
                serde_json::to_writer_pretty(o, &json!({"observations":observations}))?;
                Ok(())
            })?);
        }
        let manifest = json!({"schema_version":1,"kind":"recorded_target_observations","artifacts":artifacts,
            "target_units":request.target_units,"units_verified":false,"fit_views_exported":fit_ready,
            "synchronization_verified":false,"physical_accuracy_qualified":false});
        artifact(&partial, "manifest.json", work, |o| {
            serde_json::to_writer_pretty(o, &manifest)?;
            Ok(())
        })?;
        File::open(&partial)?.sync_all()?;
        check(work, cancel)?;
        for s in &sources { s.check_source()?; }
        publish(&partial, target)?;
        Ok(manifest)
    })().map_err(|e| format!("{e}; incomplete target observations retained at {}", partial.display()))?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent).and_then(|f| f.sync_all()).map_err(|e| {
        format!(
            "observations published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(result)
}

#[cfg(test)]
#[path = "calibration_observations/tests.rs"]
mod tests;
