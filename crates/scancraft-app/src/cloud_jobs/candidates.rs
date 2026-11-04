//! Recorded diagnostics stay outside metric editing and retain publication errors.
use super::{DisplayPoint, Preview, Report, report};
use scancraft_geometry::work::WorkControl;
use scancraft_workflow::{
    candidate_bundle,
    recorded_traces::{self, Inputs},
};
use std::path::Path;

pub(super) fn view(path: &Path, work: &WorkControl) -> Result<Report, String> {
    view_at(path, 0, work)
}
pub(super) fn view_at(path: &Path, index: u64, work: &WorkControl) -> Result<Report, String> {
    let loaded = candidate_bundle::load_at(path, index, work).map_err(|e| e.to_string())?;
    let mut preview = Preview {
        total: loaded.positions.len(),
        ..Preview::default()
    };
    for (index, position) in loaded.positions.into_iter().enumerate() {
        work.check()?;
        if let Some((min, max)) = &mut preview.bounds {
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(position[axis]);
            }
        } else {
            preview.bounds = Some((position, position));
        }
        preview.points.push(DisplayPoint {
            normal: None,
            position,
            color: [101, 190, 226],
            selected: false,
            index: loaded.first_observations[index],
        });
    }
    let mut result = report(
        format!(
            "{} candidates from {} queries · {} numerical failures · {} without a unique match. Artifact checks and retained frame bindings passed.",
            loaded.report["candidate_points"],
            loaded.report["queries"],
            loaded.report["query_failures"],
            loaded.report["queries_without_unique_match"]
        ),
        Some(preview),
    );
    if let Some(exclusions) = loaded.report.get("domain_rejections") {
        result.message.push_str(&format!(
            " Domain exclusions: {} trace vertices, {} segments, {} sheet projections. Uniqueness applies only within the retained domain; physical identity remains unverified.",
            exclusions["rejected_trace_vertices"], exclusions["rejected_trace_segments"],
            exclusions["rejected_sheet_projections"]));
    }
    if let Some(total) = loaded.report["pair_count"].as_u64() {
        result.message = format!(
            "Saved capture · pair {} of {total} · {}",
            index.saturating_add(1),
            result.message
        );
        if let Some(error) = loaded.report["stored_pair_error"].as_str() {
            result.message = format!(
                "Saved capture · pair {} of {total} · stored processing failed: {error}. Retained images loaded; no candidate geometry.",
                index.saturating_add(1)
            );
        }
        result
            .message
            .push_str(" Stored numerical results were not recomputed. ");
        result
            .message
            .push_str(if loaded.report["source_restoration_reported"] == true {
                "Source restoration is a recorded claim. "
            } else {
                "Source restoration was not recorded by this older capture. "
            });
        result
            .message
            .push_str("Tracking and fusion remain unavailable.");
    }
    result.reset_view = true;
    result.candidate_frames = Some(loaded.frames);
    Ok(result)
}

pub(super) fn process(
    inputs: &Inputs,
    target: &Path,
    work: &WorkControl,
) -> Result<Report, String> {
    process_then_view(inputs, target, work, view)
}

// Publication and display are separate outcomes. A cancellation/read failure
// after publication must not hide the successfully saved diagnostic directory.
fn process_then_view(
    inputs: &Inputs,
    target: &Path,
    work: &WorkControl,
    load: impl FnOnce(&Path, &WorkControl) -> Result<Report, String>,
) -> Result<Report, String> {
    recorded_traces::run_automatic(inputs, target, work).map_err(|e| e.to_string())?;
    let mut report=load(target,work).map_err(|e|format!("Diagnostic bundle published at {}, but display preparation failed: {e}. Reopen that folder to retry.",target.display()))?;
    report.message = format!(
        "Saved recorded-pair diagnostics to {}. {}",
        target.display(),
        report.message
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use scancraft_core::{Frame, StreamProfile};
    use scancraft_recording::Writer;
    use serde_json::{Value, json};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    struct Temp(PathBuf);
    impl Temp {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "scancraft-app-recorded-laser-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn fixture(t: &Temp) -> Inputs {
        let camera = json!({"width":10,"height":5,"fx":10.,"fy":10.,"cx":4.5,"cy":2.,"skew":0.,"valid_radius":1.,"distortion":{"k1":0.,"k2":0.,"p1":0.,"p2":0.,"k3":0.}});
        let mut second = camera.clone();
        second["cy"] = json!(1.5);
        let write = |name: &str, value: &Value| {
            fs::write(t.0.join(name), serde_json::to_vec_pretty(value).unwrap()).unwrap()
        };
        write(
            "geometry.json",
            &json!({"left":camera,"right":second,"right_from_left":{"rotation":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],"translation_mm":[-1.,0.,0.]}}),
        );
        let mut expected = Vec::new();
        for (name, interface, x) in [("first.raptor", 0, 6), ("second.raptor", 2, 5)] {
            let mut data = vec![0; 50];
            for y in 0..5 {
                data[y * 10 + x] = 80;
            }
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&100u32.to_le_bytes());
            // Bind through the same public detector workflow as real recordings.
            let mut writer = Writer::new(
                Vec::new(),
                StreamProfile {
                    interface,
                    width: 10,
                    height: 5,
                    fourcc: *b"GREY",
                    fps: 30,
                },
            )
            .unwrap();
            writer
                .push(&Frame {
                    sequence: 0,
                    host_timestamp_us: 1000,
                    flags: 0,
                    metadata,
                    data,
                })
                .unwrap();
            fs::write(t.0.join(name), writer.finish().unwrap()).unwrap();
            // Independently computed with Python zlib.crc32 for these fixed bytes.
            expected.push(json!({"interface":interface,"payload_crc32":if x==6 {1558770027u32} else {1068209696u32},"metadata_crc32":2066116316u32}));
        }
        write(
            "request.json",
            &json!({"schema_version":1,"expected_frames":expected,"reflect_x":[false,false],"maximum_host_delta_us":1000,
            "detection":{"background":10,"minimum_peak":30,"maximum_width":3,"saturation":255,"maximum_observations":100,"opening_radius":null},
            "continuity":{"maximum_row_gap":1,"maximum_horizontal_step":1.,"maximum_slope":0.,"minimum_span_rows":2,"minimum_observations":2},
            "sheets":[{"quadratic_y":0.,"linear_y":0.,"linear_z":0.,"offset":1.5}],
            "triangulation":{"min_depth_z_mm":1.,"max_depth_z_mm":100.,"min_ray_angle_rad":0.0001,"max_ray_gap_mm":0.01,"max_reprojection_error_px":0.01},
            "matching":{"intersection":{"maximum_segment_length_px":3.,"normalized_tolerance":1e-10,"minimum_plane_sine":1e-8},"maximum_sheet_distance_px":0.25,"maximum_work":1000},
            "maximum_queries":100,"maximum_total_work":10000,"maximum_report_bytes":1000000}),
        );
        Inputs {
            geometry: t.0.join("geometry.json"),
            first_recording: t.0.join("first.raptor"),
            first_index: 0,
            second_recording: t.0.join("second.raptor"),
            second_index: 0,
            request: t.0.join("request.json"),
        }
    }
    #[test]
    fn domain_diagnostics_are_visible_and_reopen_without_metric_promotion() {
        let t = Temp::new();
        let inputs = fixture(&t);
        let mut request: Value =
            serde_json::from_slice(&fs::read(&inputs.request).unwrap()).unwrap();
        request["retain_valid_domain_segments"] = json!(true);
        fs::write(&inputs.request, serde_json::to_vec(&request).unwrap()).unwrap();
        let mut geometry: Value =
            serde_json::from_slice(&fs::read(&inputs.geometry).unwrap()).unwrap();
        geometry["right"]["valid_radius"] = json!(0.16);
        fs::write(&inputs.geometry, serde_json::to_vec(&geometry).unwrap()).unwrap();
        let target = t.0.join("domain");
        let result = process(&inputs, &target, &WorkControl::default()).unwrap();
        assert_eq!(result.preview.unwrap().total, 3);
        assert!(
            result
                .message
                .contains("Domain exclusions: 1 trace vertices, 1 segments, 2 sheet projections")
        );
        assert!(
            result
                .message
                .contains("Uniqueness applies only within the retained domain")
        );
        let reopened = view(&target, &WorkControl::default()).unwrap();
        assert_eq!(reopened.preview.unwrap().total, 3);
        assert!(
            reopened
                .message
                .contains("physical identity remains unverified")
        );
        // A missing/malformed summary must not hide the partial-domain policy.
        let path = target.join("manifest.json");
        let manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for bad in [
            Value::Null,
            json!({"rejected_trace_vertices":65537,"rejected_trace_segments":1,"rejected_sheet_projections":2}),
        ] {
            let mut value = manifest.clone();
            value["domain_rejections"] = bad;
            fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(view(&target, &WorkControl::default()).is_err());
        }
        let mut value = manifest;
        value.as_object_mut().unwrap().remove("domain_rejections");
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(view(&target, &WorkControl::default()).is_err());
    }
    #[test]
    fn processing_publishes_verified_diagnostics_and_keeps_editable_state_untouched() {
        let t = Temp::new();
        let inputs = fixture(&t);
        let target = t.0.join("result");
        let original = fs::read(&inputs.first_recording).unwrap();
        let mut document = super::super::Document {
            selection: vec![19],
            ..Default::default()
        };
        let result = super::super::execute(
            &mut document,
            super::super::Operation::ProcessLaserCandidates {
                inputs: inputs.clone(),
                target: target.clone(),
            },
            &WorkControl::default(),
        )
        .unwrap();
        assert!(result.message.contains("Saved recorded-pair diagnostics"));
        assert_eq!(result.preview.unwrap().total, 4);
        assert_eq!(
            result.candidate_frames.unwrap().map(|f| f.interface),
            [0, 2]
        );
        assert!(document.editor.is_none());
        assert_eq!(document.selection, [19]);
        assert_eq!(fs::read(&inputs.first_recording).unwrap(), original);
        let manifest: Value =
            serde_json::from_slice(&fs::read(target.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["candidate_points"], 4);
        assert_eq!(manifest["physical_accuracy_qualified"], false);
    }
    #[test]
    fn cancellation_and_existing_output_do_not_overwrite_or_publish() {
        let t = Temp::new();
        let inputs = fixture(&t);
        let target = t.0.join("cancelled");
        let work = WorkControl::default();
        work.cancel();
        assert!(process(&inputs, &target, &work).is_err());
        assert!(!target.exists());
        let target = t.0.join("existing");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("sentinel"), b"original").unwrap();
        assert!(process(&inputs, &target, &WorkControl::default()).is_err());
        assert_eq!(fs::read(target.join("sentinel")).unwrap(), b"original");
        assert!(!target.join("manifest.json").exists());
    }
    #[test]
    fn display_cancellation_after_publication_reports_retained_bundle_and_allows_reopen() {
        let t = Temp::new();
        let inputs = fixture(&t);
        let target = t.0.join("result");
        let error = process_then_view(&inputs, &target, &WorkControl::default(), |path, work| {
            work.cancel();
            view(path, work)
        })
        .err()
        .unwrap();
        assert!(error.contains("bundle published at"));
        assert!(error.contains(&target.display().to_string()));
        assert!(error.contains("display preparation failed"));
        assert!(target.join("manifest.json").is_file());
        assert_eq!(
            view(&target, &WorkControl::default())
                .unwrap()
                .preview
                .unwrap()
                .total,
            4
        );
    }
}
