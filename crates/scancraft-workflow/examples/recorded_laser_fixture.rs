//! Redistributable synthetic observations for the read-only candidate viewport.
use scancraft_core::{Frame, StreamProfile};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::{CaptureContext, Writer};
use scancraft_workflow::recorded_traces::{Inputs, run_automatic};
use serde_json::json;
use std::{fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("usage: recorded_laser_fixture NEW_DIRECTORY")?,
    );
    fs::create_dir(&root)?;
    let camera = json!({"width":96,"height":64,"fx":100.,"fy":100.,"cx":47.5,"cy":31.5,
        "skew":0.,"valid_radius":1.,"distortion":{"k1":0.,"k2":0.,"p1":0.,"p2":0.,"k3":0.}});
    let mut second = camera.clone();
    second["cy"] = json!(31.);
    let geometry = json!({"left":camera,"right":second,"right_from_left":{"rotation":[[1.,0.,0.],[0.,1.,0.],[0.,0.,1.]],"translation_mm":[-10.,0.,0.]}});
    fs::write(
        root.join("geometry.json"),
        serde_json::to_vec_pretty(&geometry)?,
    )?;
    let mut expected = Vec::new();
    for (name, interface, stripes) in [
        ("first.raptor", 0, [60, 70]),
        ("second.raptor", 2, [50, 55]),
    ] {
        let mut data = vec![0u8; 96 * 64];
        for y in 0..64 {
            for x in stripes {
                data[y * 96 + x] = 80;
            }
        }
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&100u32.to_le_bytes());
        expected.push(json!({"interface":interface,"payload_crc32":crc32fast::hash(&data),"metadata_crc32":crc32fast::hash(&metadata)}));
        let context = CaptureContext {
            controls: json!({"source":"synthetic_fixture","physical_capture":false}),
            ..CaptureContext::default()
        };
        let mut writer = Writer::with_context(
            fs::File::create(root.join(name))?,
            StreamProfile {
                interface,
                width: 96,
                height: 64,
                fourcc: *b"GREY",
                fps: 30,
            },
            context,
        )?;
        writer.push(&Frame {
            sequence: 0,
            host_timestamp_us: 1000,
            flags: 0,
            metadata,
            data,
        })?;
        writer.finish()?;
    }
    let request = json!({"schema_version":1,"expected_frames":expected,"reflect_x":[false,false],"maximum_host_delta_us":1000,
        "detection":{"background":10,"minimum_peak":30,"maximum_width":3,"saturation":255,"maximum_observations":1000,"opening_radius":null},
        "continuity":{"maximum_row_gap":2,"maximum_horizontal_step":1.,"maximum_slope":0.,"minimum_span_rows":5,"minimum_observations":4},
        "sheets":[{"quadratic_y":0.,"linear_y":0.,"linear_z":0.,"offset":12.5},{"quadratic_y":0.,"linear_y":0.,"linear_z":0.,"offset":15.}],
        "triangulation":{"min_depth_z_mm":40.,"max_depth_z_mm":200.,"min_ray_angle_rad":0.0001,"max_ray_gap_mm":0.01,"max_reprojection_error_px":0.01},
        "matching":{"intersection":{"maximum_segment_length_px":3.,"normalized_tolerance":1e-10,"minimum_plane_sine":1e-8},"maximum_sheet_distance_px":0.25,"maximum_work":10000},
        "maximum_queries":1000,"maximum_total_work":1000000,"maximum_report_bytes":1000000});
    fs::write(
        root.join("request.json"),
        serde_json::to_vec_pretty(&request)?,
    )?;
    let report = run_automatic(
        &Inputs {
            geometry: root.join("geometry.json"),
            first_recording: root.join("first.raptor"),
            first_index: 0,
            second_recording: root.join("second.raptor"),
            second_index: 0,
            request: root.join("request.json"),
        },
        &root.join("result"),
        &WorkControl::default(),
    )
    .map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
