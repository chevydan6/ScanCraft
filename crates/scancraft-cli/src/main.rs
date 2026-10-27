mod extract_frame;
mod factory_dump;
mod image_export;
mod laser_diagnostics;
mod live_scan;
mod output;
mod reconstruct_stereo;
mod recording_scr;
mod stereo_probe;
mod termination;
use scancraft_core::{StreamProfile, uvc_pts};
use scancraft_device::{Capture, SessionLock, discover};
use scancraft_recording::{Reader, Writer};
use std::{
    env,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter},
    path::PathBuf,
    time::Instant,
};
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
fn stereo_fps(args: &[String]) -> Result<u32, Box<dyn std::error::Error>> {
    match args {
        [] => Ok(50),
        [flag, value] if flag == "--fps" => Ok(value.parse()?),
        _ => Err("expected optional --fps FPS".into()),
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("refine-target-corners") => {
            if args.len() != 5 {
                return Err(
                    "refine-target-corners requires IMAGE.PNG SEEDS_JSON NEW_OUTPUT_DIRECTORY"
                        .into(),
                );
            }
            let _signals = termination::Guard::install()?;
            let report = scancraft_workflow::target_corners::run(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                std::path::Path::new(&args[4]),
                &scancraft_geometry::work::WorkControl::default(),
                &|| termination::check().is_err(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("calibrate-stereo-extrinsics") => {
            if args.len() != 4 {
                return Err(
                    "calibrate-stereo-extrinsics requires OBSERVATIONS_JSON NEW_OUTPUT_DIRECTORY"
                        .into(),
                );
            }
            let _signals = termination::Guard::install()?;
            let report = scancraft_workflow::stereo_calibration::run(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                &scancraft_geometry::work::WorkControl::default(),
                &|| termination::check().is_err(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("calibrate-planar") => {
            if args.len() != 4 {
                return Err(
                    "calibrate-planar requires OBSERVATIONS_JSON NEW_OUTPUT_DIRECTORY".into(),
                );
            }
            let _signals = termination::Guard::install()?;
            let report = scancraft_workflow::planar_calibration::run(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                &scancraft_geometry::work::WorkControl::default(),
                &|| termination::check().is_err(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("scan-live-stereo") => {
            if args.len() != 6 {
                return Err("scan-live-stereo requires CALIBRATION RECONSTRUCTION_SETTINGS SCAN_SETTINGS NEW_DIRECTORY".into());
            }
            let report = live_scan::run(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                std::path::Path::new(&args[4]),
                std::path::Path::new(&args[5]),
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("reconstruct-recorded-sequence") => {
            if args.len() != 8 {
                return Err("reconstruct-recorded-sequence requires CALIBRATION LEFT_RECORDING RIGHT_RECORDING RECONSTRUCTION_SETTINGS SEQUENCE_SETTINGS NEW_DIRECTORY".into());
            }
            let inputs = scancraft_workflow::recording_reconstruction::Inputs {
                calibration: args[2].clone().into(),
                left: args[3].clone().into(),
                right: args[4].clone().into(),
                settings: args[5].clone().into(),
                left_index: 0,
                right_index: 0,
            };
            let report = scancraft_workflow::recorded_sequence::run(
                &inputs,
                std::path::Path::new(&args[6]),
                std::path::Path::new(&args[7]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("recording-scr") => {
            if args.len() != 3 {
                return Err("recording-scr requires RECORDING".into());
            }
            let mut reader = Reader::new(BufReader::new(File::open(&args[2])?))?;
            let report = recording_scr::inspect(&mut reader)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("recording-timing") => {
            if args.len() != 5 {
                return Err(
                    "recording-timing requires LEFT_RECORDING RIGHT_RECORDING MAX_HOST_DELTA_US"
                        .into(),
                );
            }
            let report = scancraft_workflow::recording_timing::inspect(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                args[4].parse()?,
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("recover-recording") => {
            if args.len() != 5 {
                return Err(
                    "recover-recording requires INPUT MAX_FRAMES NEW_OUTPUT_DIRECTORY".into(),
                );
            }
            let _termination = termination::Guard::install()?;
            let summary = scancraft_workflow::recording_recovery::run(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[4]),
                args[3].parse()?,
                &scancraft_geometry::work::WorkControl::default(),
                &|| termination::check().map_err(Into::into),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Some("stream-state") => {
            if args.len() != 3 {
                return Err("stream-state requires Raptor interface 0, 2 or 4".into());
            }
            let state = scancraft_device::stream_state::read(args[2].parse()?)?;
            println!("{}", serde_json::to_string_pretty(&state)?);
        }
        Some("import-mesh") => {
            if args.len() != 7 {
                return Err("import-mesh requires FILE obj|stl mm|in|m separate|weld-exact NEW_OUTPUT_DIRECTORY".into());
            }
            let config =
                scancraft_geometry::mesh_import::Config::from_labels(&args[3], &args[4], &args[5])?;
            let summary = scancraft_workflow::mesh_trim::import_mesh(
                std::path::Path::new(&args[2]),
                config,
                std::path::Path::new(&args[6]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Some("smooth-mesh") => {
            if args.len() != 9 {
                return Err("smooth-mesh requires MESH_OR_PROJECT ITERATIONS RELAXATION MAX_DISPLACEMENT_MM MAX_NORMAL_DEGREES CREASE_DEGREES NEW_OUTPUT_DIRECTORY".into());
            }
            let config = scancraft_geometry::mesh_smooth::Config {
                iterations: args[3].parse()?,
                relaxation: args[4].parse()?,
                maximum_displacement_mm: args[5].parse()?,
                maximum_normal_change_degrees: args[6].parse()?,
                preserve_crease_degrees: args[7].parse()?,
            };
            let work = scancraft_geometry::work::WorkControl::default();
            let prepared = scancraft_workflow::mesh_trim::prepare_smooth(
                std::path::Path::new(&args[2]),
                config,
                &work,
            )
            .map_err(|e| e.to_string())?;
            let publication = scancraft_workflow::mesh_trim::publish_prepared(
                &prepared,
                std::path::Path::new(&args[8]),
                &work,
            )
            .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"publication":publication,"smoothing":prepared.report()["parameters"]["smoothing"]})
                )?
            );
        }

        Some("simplify-mesh") => {
            if args.len() != 9 {
                return Err("simplify-mesh requires MESH_OR_PROJECT TARGET_TRIANGLES MAX_DISPLACEMENT_MM MAX_NORMAL_DEGREES CREASE_DEGREES MAX_COLLAPSES NEW_OUTPUT_DIRECTORY".into());
            }
            let config = scancraft_geometry::mesh_simplify::Config {
                target_triangles: args[3].parse()?,
                maximum_displacement_mm: args[4].parse()?,
                maximum_normal_change_degrees: args[5].parse()?,
                preserve_crease_degrees: args[6].parse()?,
                maximum_collapses: args[7].parse()?,
            };
            let work = scancraft_geometry::work::WorkControl::default();
            let prepared = scancraft_workflow::mesh_trim::prepare_simplify(
                std::path::Path::new(&args[2]),
                config,
                &work,
            )
            .map_err(|e| e.to_string())?;
            let publication = scancraft_workflow::mesh_trim::publish_prepared(
                &prepared,
                std::path::Path::new(&args[8]),
                &work,
            )
            .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"publication":publication,"simplification":prepared.report()["parameters"]["decimation"]})
                )?
            );
        }

        Some("save-mesh-section") => {
            if args.len() != 11 {
                return Err("save-mesh-section requires MESH_OR_PROJECT OX OY OZ NX NY NZ TOLERANCE_MM NEW_OUTPUT_DIRECTORY".into());
            }
            let origin = [args[3].parse()?, args[4].parse()?, args[5].parse()?];
            let normal = [args[6].parse()?, args[7].parse()?, args[8].parse()?];
            let tolerance = args[9].parse()?;
            let work = scancraft_geometry::work::WorkControl::default();
            let (editor, _) =
                scancraft_workflow::mesh_history::load_input(std::path::Path::new(&args[2]), &work)
                    .map_err(|e| e.to_string())?;
            scancraft_workflow::section_document::save(
                editor.mesh(),
                origin,
                normal,
                tolerance,
                std::path::Path::new(&args[10]),
                &work,
            )
            .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"section_document":std::path::Path::new(&args[10]).join("section.secproj"),"source_snapshot":"source.meshproj","physical_accuracy_qualified":false})
                )?
            );
        }
        Some("split-mesh-vertex-fans") => {
            if args.len() != 5 {
                return Err("split-mesh-vertex-fans requires MESH_OR_PROJECT COMMA_SEPARATED_VERTEX_IDS NEW_RESULT_FOLDER".into());
            }
            let ids = args[3]
                .split(',')
                .map(|s| s.trim().parse::<u32>())
                .collect::<Result<Vec<_>, _>>()?;
            let work = scancraft_geometry::work::WorkControl::default();
            let prepared = scancraft_workflow::mesh_trim::prepare_split_fans(
                std::path::Path::new(&args[2]),
                &ids,
                &work,
            )
            .map_err(|e| e.to_string())?;
            let report = scancraft_workflow::mesh_trim::publish_prepared(
                &prepared,
                std::path::Path::new(&args[4]),
                &work,
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("mesh-topology") => {
            if args.len() != 3 {
                return Err("mesh-topology requires MESH_OR_PROJECT".into());
            }
            let work = scancraft_geometry::work::WorkControl::default();
            let (editor, _) =
                scancraft_workflow::mesh_history::load_input(std::path::Path::new(&args[2]), &work)
                    .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"schema_version":1,"units":"millimeters","topology":editor.mesh().topology(&work)?,"watertightness_qualified":false,"physical_accuracy_qualified":false})
                )?
            );
        }
        Some("orient-mesh") => {
            if args.len() != 5 {
                return Err("orient-mesh requires MESH_OR_PROJECT REVERSE_COMPONENT_IDS_OR_none NEW_RESULT_FOLDER".into());
            }
            let reverse = if args[3] == "none" {
                Vec::new()
            } else {
                args[3]
                    .split(',')
                    .map(|s| s.trim().parse::<usize>())
                    .collect::<Result<Vec<_>, _>>()?
            };
            let work = scancraft_geometry::work::WorkControl::default();
            let prepared = scancraft_workflow::mesh_trim::prepare_orientation(
                std::path::Path::new(&args[2]),
                &reverse,
                &work,
            )
            .map_err(|e| e.to_string())?;
            let result = scancraft_workflow::mesh_trim::publish_prepared(
                &prepared,
                std::path::Path::new(&args[4]),
                &work,
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Some("fill-mesh-boundary") => {
            if args.len() != 6 {
                return Err("fill-mesh-boundary requires MESH_OR_PROJECT BOUNDARY_ID MAX_PLANARITY_MM NEW_OUTPUT_DIRECTORY".into());
            }
            let work = scancraft_geometry::work::WorkControl::default();
            let prepared = scancraft_workflow::mesh_trim::prepare_fill(
                std::path::Path::new(&args[2]),
                args[3].parse()?,
                args[4].parse()?,
                &work,
            )
            .map_err(|e| e.to_string())?;
            let report = scancraft_workflow::mesh_trim::publish_prepared(
                &prepared,
                std::path::Path::new(&args[5]),
                &work,
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("mesh-boundaries") => {
            if args.len() != 3 {
                return Err("mesh-boundaries requires MESH_OR_PROJECT".into());
            }
            let work = scancraft_geometry::work::WorkControl::default();
            let (editor, _) =
                scancraft_workflow::mesh_history::load_input(std::path::Path::new(&args[2]), &work)
                    .map_err(|e| e.to_string())?;
            let analysis = scancraft_geometry::mesh_boundaries::analyze(editor.mesh(), &work)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"schema_version":1,"units":"millimeters","interpretation":"indexed boundary contours; not hole classification or watertightness proof","physical_accuracy_qualified":false,"analysis":analysis})
                )?
            );
        }
        Some("mesh-section") => {
            if args.len() != 10 {
                return Err(
                    "mesh-section requires MESH_OR_PROJECT OX OY OZ NX NY NZ TOLERANCE_MM".into(),
                );
            }
            let origin = [args[3].parse()?, args[4].parse()?, args[5].parse()?];
            let normal = [args[6].parse()?, args[7].parse()?, args[8].parse()?];
            let tolerance = args[9].parse()?;
            let work = scancraft_geometry::work::WorkControl::default();
            let (editor, _) =
                scancraft_workflow::mesh_history::load_input(std::path::Path::new(&args[2]), &work)
                    .map_err(|e| e.to_string())?;
            let section = scancraft_geometry::mesh_section::intersect(
                editor.mesh(),
                origin,
                normal,
                tolerance,
                &work,
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"schema_version":1,"units":"millimeters","physical_accuracy_qualified":false,"interpretation":"indexed plane section; coplanar faces omitted, branches retained as ambiguous graphs; not area or watertightness qualification","section":section})
                )?
            );
        }
        Some("mesh-components") => {
            if args.len() != 3 {
                return Err("mesh-components requires MESH_OR_PROJECT".into());
            }
            let work = scancraft_geometry::work::WorkControl::default();
            let (editor, _) =
                scancraft_workflow::mesh_history::load_input(std::path::Path::new(&args[2]), &work)
                    .map_err(|e| e.to_string())?;
            let analysis = scancraft_geometry::mesh_components::analyze(editor.mesh(), &work)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"schema_version":1,"definition":"faces sharing an indexed edge; IDs in first-triangle order","units":"millimeters","component_count":analysis.components.len(),"components":analysis.components,"physical_accuracy_qualified":false})
                )?
            );
        }
        Some("select-mesh-components") => {
            if args.len() != 6 || !["keep", "remove"].contains(&args[3].as_str()) {
                return Err("select-mesh-components requires MESH_OR_PROJECT [keep|remove] COMMA_SEPARATED_IDS NEW_OUTPUT_DIRECTORY".into());
            }
            let ids: Vec<usize> = args[4]
                .split(',')
                .map(str::parse)
                .collect::<Result<_, _>>()?;
            let work = scancraft_geometry::work::WorkControl::default();
            let target = std::path::Path::new(&args[5]);
            if std::fs::symlink_metadata(target).is_ok() {
                return Err("component output already exists".into());
            }
            let p = scancraft_workflow::mesh_trim::prepare_components(
                std::path::Path::new(&args[2]),
                &ids,
                args[3] == "keep",
                &work,
            )
            .map_err(|e| e.to_string())?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &scancraft_workflow::mesh_trim::publish_prepared(&p, target, &work)
                        .map_err(|e| e.to_string())?
                )?
            );
        }
        Some("cleanup-mesh") => {
            if args.len() != 5 || !["duplicates", "unused", "both"].contains(&args[3].as_str()) {
                return Err("cleanup-mesh requires MESH_OR_PROJECT [duplicates|unused|both] NEW_OUTPUT_DIRECTORY".into());
            }
            let options = scancraft_geometry::mesh_edit::CleanupOptions {
                duplicate_faces: args[3] != "unused",
                unused_vertices: args[3] != "duplicates",
            };
            let work = scancraft_geometry::work::WorkControl::default();
            let target = std::path::Path::new(&args[4]);
            if std::fs::symlink_metadata(target).is_ok() {
                return Err("cleanup output already exists".into());
            }
            let prepared = scancraft_workflow::mesh_trim::prepare_cleanup(
                std::path::Path::new(&args[2]),
                options,
                &work,
            )
            .map_err(|e| e.to_string())?;
            let summary = scancraft_workflow::mesh_trim::publish_prepared(&prepared, target, &work)
                .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Some("mesh-history") => {
            if args.len() != 5 || !["undo", "redo"].contains(&args[3].as_str()) {
                return Err(
                    "mesh-history requires PROJECT [undo|redo] NEW_OUTPUT_DIRECTORY".into(),
                );
            }
            let summary = scancraft_workflow::mesh_history::step(
                std::path::Path::new(&args[2]),
                args[3] == "undo",
                std::path::Path::new(&args[4]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Some("trim-mesh") => {
            if args.len() != 8 {
                return Err(
                    "trim-mesh requires MESH_JSON NX NY NZ OFFSET NEW_OUTPUT_DIRECTORY".into(),
                );
            }
            let summary = scancraft_workflow::mesh_trim::run(
                std::path::Path::new(&args[2]),
                [args[3].parse()?, args[4].parse()?, args[5].parse()?],
                args[6].parse()?,
                std::path::Path::new(&args[7]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Some("mesh-grid") => {
            if args.len() != 4 {
                return Err("mesh-grid requires ORGANIZED_GRID_JSON NEW_OUTPUT_DIRECTORY".into());
            }
            let summary = scancraft_workflow::meshing::run(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Some("filter-statistical") => {
            if args.len() != 6 {
                return Err(
                    "filter-statistical requires PROJECT_OR_PLY K SIGMA_MULTIPLIER NEW_DIRECTORY"
                        .into(),
                );
            }
            let report = scancraft_workflow::statistical::run(
                std::path::Path::new(&args[2]),
                args[3].parse()?,
                args[4].parse()?,
                std::path::Path::new(&args[5]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("fuse-sequence") => {
            if args.len() != 4 {
                return Err("fuse-sequence requires SETTINGS_JSON NEW_OUTPUT_DIRECTORY".into());
            }
            let report = scancraft_workflow::sequence::run(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("verify-project-recordings") => {
            if args.len() != 4 {
                return Err(
                    "verify-project-recordings requires PROJECT RECORDINGS_DIRECTORY".into(),
                );
            }
            let report = scancraft_workflow::scan_project_load::verify_project_recordings(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("verify-scan-project") => {
            if args.len() != 3 {
                return Err("verify-scan-project requires SCAN_FOLDER".into());
            }
            let loaded = scancraft_workflow::scan_project_load::load(
                std::path::Path::new(&args[2]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&loaded.report)?);
        }
        Some("export-scan-project") => {
            if args.len() != 7 {
                return Err("export-scan-project requires PROJECT SOURCE_BUNDLE LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY".into());
            }
            let report = scancraft_workflow::scan_project::export_file(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                std::path::Path::new(&args[4]),
                std::path::Path::new(&args[5]),
                std::path::Path::new(&args[6]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("archive-recordings") => {
            if args.len() != 5 {
                return Err(
                    "archive-recordings requires LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY"
                        .into(),
                );
            }
            let report = scancraft_workflow::recording_archive::archive(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                std::path::Path::new(&args[4]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("export-project-bundle") => {
            if args.len() != 5 {
                return Err(
                    "export-project-bundle requires PROJECT SOURCE_BUNDLE NEW_OUTPUT_DIRECTORY"
                        .into(),
                );
            }
            let summary = scancraft_workflow::project_bundle::export_file(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                std::path::Path::new(&args[4]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Some("verify-source-bundle") => {
            if args.len() != 4 {
                return Err("verify-source-bundle requires PROJECT BUNDLE_DIRECTORY".into());
            }
            let result = scancraft_workflow::source_verification::verify_project(
                std::path::Path::new(&args[2]),
                std::path::Path::new(&args[3]),
                &scancraft_geometry::work::WorkControl::default(),
            )
            .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&result)?);
            if !result.passed {
                return Err("source bundle does not match saved project references".into());
            }
        }
        Some("reconstruct-recordings") => {
            if args.len() != 9 {
                return Err("reconstruct-recordings requires CALIBRATION LEFT_RECORDING LEFT_INDEX RIGHT_RECORDING RIGHT_INDEX SETTINGS NEW_OUTPUT_DIRECTORY".into());
            }
            let inputs = scancraft_workflow::recording_reconstruction::Inputs {
                calibration: args[2].clone().into(),
                left: args[3].clone().into(),
                left_index: args[4].parse()?,
                right: args[5].clone().into(),
                right_index: args[6].parse()?,
                settings: args[7].clone().into(),
            };
            let target = PathBuf::from(&args[8]);
            if std::fs::symlink_metadata(&target).is_ok() {
                return Err("output directory already exists".into());
            }
            let work = scancraft_geometry::work::WorkControl::default();
            let prepared = scancraft_workflow::recording_reconstruction::prepare(&inputs, &work)
                .map_err(|e| e.to_string())?;
            let summary = scancraft_workflow::publish_prepared(&prepared, &target, &work)
                .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Some("reconstruct-stereo") => {
            if args.len() != 7 {
                return Err("reconstruct-stereo requires CALIBRATION LEFT.PNG RIGHT.PNG REQUEST.JSON OUTPUT_DIRECTORY".into());
            }
            let summary = reconstruct_stereo::run(args[2..].iter().map(PathBuf::from).collect())
                .map_err(|e| e.to_string())?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Some("detect-stripes") => {
            if args.len() != 9 && !(args.len() == 11 && args[9] == "--opening-radius") {
                return Err("detect-stripes requires RECORDING INDEX BACKGROUND MIN_PEAK MAX_WIDTH SATURATION MAX_OBSERVATIONS [--opening-radius RADIUS]".into());
            }
            let opening_radius = if args.len() == 11 {
                Some(args[10].parse()?)
            } else {
                None
            };
            let config = scancraft_image::laser::LineConfig {
                background: args[4].parse()?,
                minimum_peak: args[5].parse()?,
                maximum_width: args[6].parse()?,
                saturation: args[7].parse()?,
                maximum_observations: args[8].parse()?,
            };
            let report = laser_diagnostics::run(
                std::path::Path::new(&args[2]),
                args[3].parse()?,
                config,
                opening_radius,
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("extract-frame") => {
            if args.len() != 5 {
                return Err(
                    "extract-frame requires RECORDING ZERO_BASED_INDEX NEW_OUTPUT_DIRECTORY".into(),
                );
            }
            let report = extract_frame::run(
                std::path::Path::new(&args[2]),
                args[3].parse()?,
                std::path::Path::new(&args[4]),
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("stereo-line-capture") => {
            if args.len() < 4 {
                return Err(
                    "stereo-line-capture requires FRAME_COUNT NEW_OUTPUT_DIRECTORY [--fps FPS]"
                        .into(),
                );
            }
            let report = stereo_probe::record_line(
                args[2].parse()?,
                stereo_fps(&args[4..])?,
                std::path::Path::new(&args[3]),
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("stereo-flood-capture") => {
            if args.len() < 4 {
                return Err(
                    "stereo-flood-capture requires FRAME_COUNT NEW_OUTPUT_DIRECTORY [--fps FPS]"
                        .into(),
                );
            }
            let report = stereo_probe::record_flood(
                args[2].parse()?,
                stereo_fps(&args[4..])?,
                std::path::Path::new(&args[3]),
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("stereo-gain-capture") => {
            if args.len() < 5 {
                return Err("stereo-gain-capture requires FRAME_COUNT RAW_GAIN NEW_OUTPUT_DIRECTORY [--fps FPS]".into());
            }
            let report = stereo_probe::record_gain(
                args[2].parse()?,
                stereo_fps(&args[5..])?,
                std::path::Path::new(&args[4]),
                Some(args[3].parse()?),
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("stereo-capture") => {
            if args.len() < 4 {
                return Err(
                    "stereo-capture requires FRAME_COUNT NEW_OUTPUT_DIRECTORY [--fps FPS]".into(),
                );
            }
            let report = stereo_probe::record(
                args[2].parse()?,
                stereo_fps(&args[4..])?,
                std::path::Path::new(&args[3]),
            )?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Some("stereo-probe") => {
            let count = args
                .get(2)
                .ok_or("stereo-probe requires a per-camera frame count")?
                .parse()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&stereo_probe::run(count, stereo_fps(&args[3..])?)?)?
            );
        }
        Some("illumination-status") => {
            if args.len() != 2 {
                return Err("illumination-status takes no parameters".into());
            }
            let lock = SessionLock::acquire()?;
            let properties = scancraft_device::control::illumination::snapshot(&lock)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "read_only_opcode":1,"properties":properties,"physical_emitter_state_verified":false,
                    "interpretation":"Raw reported SDK properties; no emitter/mode setting changed or physical illumination inferred"
                }))?
            );
        }
        Some("mirror-status") => {
            let lock = SessionLock::acquire()?;
            let properties = scancraft_device::control::mirror_status::snapshot(&lock)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "read_only_opcode":1, "properties":properties,
                "sdk_accessor_equivalence_verified":false,
                    "image_orientation_verified":false, "calibration_binding_verified":false,
                    "interpretation":"Raw firmware property responses only; SDK mirror controls may be host-side; no state changed or physical camera convention inferred"
                }))?
            );
        }
        Some("controls") => {
            let _lock = SessionLock::acquire()?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"read_only":true,"units":"raw V4L2 values; absolute exposure interpreted using standard 100 microsecond units","cameras":scancraft_device::standard_controls::snapshot()?})
                )?
            );
        }
        Some("factory-inspect")
        | Some("factory-verify")
        | Some("factory-lenses")
        | Some("factory-laser-fields") => {
            if args.len() != 3 {
                return Err(
                    "factory-inspect/factory-verify/factory-lenses/factory-laser-fields requires FACTORY_REGION_FILE"
                        .into(),
                );
            }
            use std::{io::Read, os::unix::fs::OpenOptionsExt};
            let file = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&args[2])?;
            if !file.metadata()?.is_file() {
                return Err("factory inspection requires a regular file".into());
            }
            let mut bytes = Vec::new();
            file.take(8193).read_to_end(&mut bytes)?;
            if args[1] == "factory-laser-fields" {
                let records = scancraft_calibration::factory_laser::inspect(&bytes)?;
                let epipolar_comparisons = records
                    .iter()
                    .map(|r| r.compare_epipolar_fields())
                    .collect::<Result<Vec<_>, _>>()?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "checksum_verified": true,
                        "physical_camera_assignment_verified": false,
                        "image_mode_binding_verified": false,
                        "translation_units_verified": false,
                        "laser_coefficient_formula_verified": false,
                        "epipolar_numeric_formula_verified": true,
                        "epipolar_field_comparisons": epipolar_comparisons,
                        "records": records
                    }))?
                );
                return Ok(());
            } else if args[1] == "factory-lenses" {
                let lenses = scancraft_calibration::factory_named::inspect(&bytes)?;
                let pose_matrices = lenses
                    .iter()
                    .map(|lens| {
                        scancraft_calibration::factory_pose::matrix_raw_units(
                            lens.rotation_fields,
                            lens.translation_fields,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "checksum_verified": true, "stereo_calibration_decoded": false,
                        "brown_forward_formula_verified": true, "lenses": lenses,
                        "brown_coefficient_order": ["k1", "k2", "k3", "k4", "k5", "k6", "p1", "p2"],
                        "brown5_reduction_supported": lenses.iter().map(|lens| scancraft_calibration::factory_brown::brown5_if_supported(lens.brown_values).is_ok()).collect::<Vec<_>>(),
                        "calibration_coverage_verified": false,
                        "pose_matrices_raw_units": pose_matrices,
                        "second_times_inverse_first_raw_units": scancraft_calibration::factory_pose::relative_matrix_raw_units(
                            lenses[0].rotation_fields, lenses[0].translation_fields,
                            lenses[1].rotation_fields, lenses[1].translation_fields)?,
                        "pose_math": "row-major for column vectors; Rz(RZ)*Rx(RX)*Ry(RY), radians; translation unscaled",
                        "translation_units_verified": false, "transform_direction_verified": false
                    }))?
                );
                return Ok(());
            }
            if args[1] == "factory-verify" {
                let envelope = scancraft_calibration::factory::verify(&bytes)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "checksum_verified": true, "optical_calibration_decoded": false,
                        "envelope": envelope
                    }))?
                );
                return Ok(());
            }
            let index = scancraft_calibration::factory::inspect(&bytes)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "checksum_verified": true, "optical_calibration_decoded": false,
                    "index": index
                }))?
            );
        }
        Some("factory-read") => {
            if args.len() != 4 {
                return Err("factory-read requires REGION NEW_OUTPUT_DIRECTORY".into());
            }
            let region = scancraft_device::control::FactoryRegion::parse(&args[2])?;
            println!(
                "{}",
                serde_json::to_string_pretty(&factory_dump::run(
                    region,
                    std::path::Path::new(&args[3])
                )?)?
            );
        }
        Some("factory-header") => {
            if args.len() != 3 {
                return Err("factory-header requires REGION (200000, 202000, 20a000)".into());
            }
            let region = scancraft_device::control::FactoryRegion::parse(&args[2])?;
            let _lock = SessionLock::acquire()?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"region_start":region.base(),"read_only_opcode":25,"header":scancraft_device::control::factory_header(region)?,"optical_calibration_decoded":false})
                )?
            );
        }
        Some("protocol-version") => {
            let _lock = SessionLock::acquire()?;
            let payload = scancraft_device::control::protocol_version()?;
            println!(
                "{}",
                serde_json::json!({"read_only":true,"opcode":254,"payload":payload})
            );
        }
        Some("discover") => println!("{}", serde_json::to_string_pretty(&discover()?)?),
        Some("capture") | Some("validate") => {
            let interface: u8 = args.get(2).ok_or("missing INTERFACE")?.parse()?;
            if ![0, 2, 4].contains(&interface) {
                return Err("interface must be 0, 2, or 4".into());
            }
            let verify_only = args[1] == "validate";
            let path = if verify_only {
                None
            } else {
                Some(PathBuf::from(args.get(3).ok_or("missing output path")?))
            };
            let count: u64 = args
                .get(if verify_only { 3 } else { 4 })
                .ok_or("missing frame count")?
                .parse()?;
            if count == 0 {
                return Err("frame count must be positive".into());
            }
            let _termination = termination::Guard::install()?;
            termination::check()?;
            let _lock = SessionLock::acquire()?;
            let fps = if let Some(i) = args.iter().position(|a| a == "--fps") {
                args.get(i + 1)
                    .ok_or("--fps needs a value")?
                    .parse::<u32>()?
            } else if interface == 4 {
                10
            } else {
                50
            };
            let supported = if interface == 4 {
                [10, 15, 20, 25, 30].contains(&fps)
            } else {
                [5, 10, 15, 20, 25, 30, 40, 50].contains(&fps)
            };
            if !supported {
                return Err("frame rate not in the measured profile allowlist".into());
            }
            let profile = StreamProfile {
                interface,
                width: if interface == 4 { 1280 } else { 1920 },
                height: if interface == 4 { 800 } else { 1200 },
                fourcc: if interface == 4 { *b"MJPG" } else { *b"GREY" },
                fps,
            };
            let partial = path.as_ref().map(|p| p.with_extension("raptor.partial"));
            termination::check()?;
            let recording_file = if let Some(p) = &path {
                if p.exists() {
                    return Err("output already exists".into());
                }
                Some(BufWriter::new(
                    OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(partial.as_ref().unwrap())?,
                ))
            } else {
                None
            };
            let check_interrupted = || {
                termination::check().map_err(|error| match &partial {
                    Some(path) => format!(
                        "{error}; unfinished recording retained at {}",
                        path.display()
                    ),
                    None => error,
                })
            };
            let opened = if recording_file.is_some() {
                Capture::open_recording(profile.clone())
            } else {
                Capture::open(profile.clone())
            };
            check_interrupted()?;
            let mut capture = opened?;
            let mut writer = recording_file
                .map(|file| -> Result<_, Box<dyn std::error::Error>> {
                    let mut context = scancraft_recording::CaptureContext::default();
                    if let Some(observed) = capture.recording_observations() {
                        context.add_capture_observations(observed)?;
                    }
                    Ok(Writer::with_context(file, profile, context)?)
                })
                .transpose()?;
            check_interrupted()?;
            let start = Instant::now();
            let mut first = None;
            let mut last = None;
            let mut first_host = None;
            let mut last_host = None;
            for n in 0..count {
                check_interrupted()?;
                let acquired = capture.next_frame();
                check_interrupted()?;
                let frame = acquired?;
                let pts = uvc_pts(&frame.metadata)?.ok_or("missing PTS")?;
                if let Some(previous) = last {
                    let delta = pts.wrapping_sub(previous);
                    if delta == 0 || delta >= 0x8000_0000 {
                        return Err("sensor PTS is not increasing".into());
                    }
                }
                if first.is_none() {
                    first = Some(pts);
                }
                last = Some(pts);
                first_host.get_or_insert(frame.host_timestamp_us);
                last_host = Some(frame.host_timestamp_us);
                if let Some(w) = &mut writer {
                    w.push(&frame)?;
                }
                if n % 500 == 0 {
                    eprintln!(
                        "validated frame {}/{} sequence={} PTS={} rejected={} missing={} gaps={}",
                        n + 1,
                        count,
                        frame.sequence,
                        pts,
                        capture.rejected,
                        capture.metadata_missing,
                        capture.sequence_drops
                    );
                }
            }
            check_interrupted()?;
            let host_span = last_host.unwrap().saturating_sub(first_host.unwrap());
            let summary = serde_json::json!({"profile":&capture.profile,"first_host_timestamp_us":first_host,"last_host_timestamp_us":last_host,"accepted_host_span_seconds":host_span as f64/1e6,"accepted_buffer_rate_hz":if host_span>0 {Some((count-1) as f64*1e6/host_span as f64)} else {None},"frames":count,"elapsed_seconds":start.elapsed().as_secs_f64(),"rejected_frames":capture.rejected,"rejected_buffers":capture.rejected,"rejected_before_first_valid":capture.rejected_before_first_valid,"rejected_after_first_valid":capture.rejected_after_first_valid,"duplicate_sensor_pts":capture.duplicate_sensor_pts,"rejection_samples":capture.rejection_samples,"ongoing_rejection_samples":capture.ongoing_rejection_samples,"rejection_counter_unit":"V4L2 buffers; not necessarily physical exposures","metadata_missing":capture.metadata_missing,"sequence_drops":capture.sequence_drops,"first_sensor_pts":first,"last_sensor_pts":last,"native_rust":true,"wine_used":false,"output":path,"saved":!verify_only});
            let closed = capture.close();
            if let Err(interruption) = check_interrupted() {
                return Err(match closed {
                    Ok(()) => interruption,
                    Err(cleanup) => format!("{interruption}; stream cleanup failed: {cleanup}"),
                }
                .into());
            }
            closed?;
            if let Some(w) = writer {
                let output = w.finish()?;
                output.get_ref().sync_all()?;
                drop(output);
                check_interrupted()?;
                std::fs::hard_link(partial.as_ref().unwrap(), path.as_ref().unwrap())?;
                std::fs::remove_file(partial.as_ref().unwrap())?;
            }
            println!("{}", serde_json::to_string_pretty(&summary)?);
        }
        Some("replay") | Some("export-image") => {
            let export_target = if args[1] == "export-image" {
                let target = args.get(3).ok_or("export-image needs output PNG")?;
                if std::fs::symlink_metadata(target).is_ok() {
                    return Err("snapshot already exists".into());
                }
                Some(std::path::Path::new(target))
            } else {
                None
            };
            let mut first_image = None;
            let path = args.get(2).ok_or("replay requires a recording path")?;
            let mut reader = Reader::new(BufReader::new(File::open(path)?))?;
            let mut n = 0;
            let mut bytes = 0;
            let decode = args.iter().any(|a| a == "--decode") || args[1] == "export-image";
            let mut decoded = 0;
            while let Some(frame) = reader.read_frame()? {
                if decode {
                    let im = match &reader.profile.fourcc {
                        b"MJPG" => image::load_from_memory_with_format(
                            &frame.data,
                            image::ImageFormat::Jpeg,
                        )?,
                        b"GREY" => image::DynamicImage::ImageLuma8(
                            image::GrayImage::from_raw(
                                reader.profile.width,
                                reader.profile.height,
                                frame.data.clone(),
                            )
                            .ok_or("invalid image size")?,
                        ),
                        _ => return Err("decoder unavailable for this format".into()),
                    };
                    if im.width() != reader.profile.width || im.height() != reader.profile.height {
                        return Err("decoded dimensions mismatch".into());
                    }
                    if args[1] == "export-image" && n == 0 {
                        first_image = Some(im);
                    }
                    decoded += 1;
                }
                n += 1;
                bytes += frame.data.len();
            }
            if let Some(target) = export_target {
                image_export::publish_png(
                    &first_image.ok_or("recording contains no image")?,
                    target,
                )?;
            }
            println!(
                "{}",
                serde_json::json!({"frames":n,"payload_bytes":bytes,"decoded_frames":decoded,"integrity":"PASS","recording_version":reader.version,"context":reader.context,"profile":reader.profile})
            );
        }
        _ => {
            return Err(
                "Usage: scancraft-cli refine-target-corners IMAGE.PNG SEEDS_JSON NEW_OUTPUT_DIRECTORY | recording-scr RECORDING | calibrate-stereo-extrinsics OBSERVATIONS_JSON NEW_OUTPUT_DIRECTORY | calibrate-planar OBSERVATIONS_JSON NEW_OUTPUT_DIRECTORY | scan-live-stereo CALIBRATION RECONSTRUCTION_SETTINGS SCAN_SETTINGS NEW_DIRECTORY | reconstruct-recorded-sequence CALIBRATION LEFT_RECORDING RIGHT_RECORDING RECONSTRUCTION_SETTINGS SEQUENCE_SETTINGS NEW_DIRECTORY | recording-timing LEFT_RECORDING RIGHT_RECORDING MAX_HOST_DELTA_US | filter-statistical PROJECT_OR_PLY K SIGMA_MULTIPLIER NEW_DIRECTORY | verify-project-recordings PROJECT RECORDINGS_DIRECTORY | verify-scan-project SCAN_FOLDER | export-scan-project PROJECT SOURCE_BUNDLE LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY | archive-recordings LEFT_RECORDING RIGHT_RECORDING NEW_DIRECTORY | export-project-bundle PROJECT SOURCE_BUNDLE NEW_OUTPUT_DIRECTORY | verify-source-bundle PROJECT BUNDLE_DIRECTORY | reconstruct-recordings CALIBRATION LEFT_RECORDING LEFT_INDEX RIGHT_RECORDING RIGHT_INDEX SETTINGS NEW_OUTPUT_DIRECTORY | recover-recording INPUT MAX_FRAMES NEW_OUTPUT_DIRECTORY | stream-state INTERFACE | import-mesh FILE obj|stl mm|in|m separate|weld-exact NEW_OUTPUT_DIRECTORY | smooth-mesh MESH_OR_PROJECT ITERATIONS RELAXATION MAX_DISPLACEMENT_MM MAX_NORMAL_DEGREES CREASE_DEGREES NEW_OUTPUT_DIRECTORY | stereo-line-capture FRAME_COUNT NEW_OUTPUT_DIRECTORY [--fps FPS] | orient-mesh MESH_OR_PROJECT REVERSE_COMPONENT_IDS_OR_none NEW_RESULT_FOLDER | fill-mesh-boundary MESH_OR_PROJECT BOUNDARY_ID MAX_PLANARITY_MM NEW_OUTPUT_DIRECTORY | split-mesh-vertex-fans MESH_OR_PROJECT COMMA_SEPARATED_VERTEX_IDS NEW_RESULT_FOLDER | simplify-mesh MESH_OR_PROJECT TARGET_TRIANGLES MAX_DISPLACEMENT_MM MAX_NORMAL_DEGREES CREASE_DEGREES MAX_COLLAPSES NEW_OUTPUT_DIRECTORY | mesh-topology MESH_OR_PROJECT | mesh-boundaries MESH_OR_PROJECT | save-mesh-section MESH_OR_PROJECT OX OY OZ NX NY NZ TOLERANCE_MM NEW_OUTPUT_DIRECTORY | mesh-section MESH_OR_PROJECT OX OY OZ NX NY NZ TOLERANCE_MM | mesh-components MESH_OR_PROJECT | select-mesh-components MESH_OR_PROJECT [keep|remove] IDS NEW_OUTPUT_DIRECTORY | cleanup-mesh MESH_OR_PROJECT [duplicates|unused|both] NEW_OUTPUT_DIRECTORY | mesh-history PROJECT [undo|redo] NEW_OUTPUT_DIRECTORY | trim-mesh MESH_JSON NX NY NZ OFFSET NEW_OUTPUT_DIRECTORY | mesh-grid ORGANIZED_GRID_JSON NEW_OUTPUT_DIRECTORY | fuse-sequence SETTINGS_JSON NEW_OUTPUT_DIRECTORY | detect-stripes RECORDING INDEX BACKGROUND MIN_PEAK MAX_WIDTH SATURATION MAX_OBSERVATIONS [--opening-radius RADIUS] | discover | controls | mirror-status | illumination-status | stereo-flood-capture FRAME_COUNT NEW_OUTPUT_DIRECTORY [--fps FPS] | stereo-gain-capture FRAME_COUNT RAW_GAIN NEW_OUTPUT_DIRECTORY [--fps FPS] | factory-laser-fields FACTORY_REGION_FILE | factory-lenses FACTORY_REGION_FILE | factory-verify FACTORY_REGION_FILE | factory-inspect FACTORY_REGION_FILE | factory-header REGION | factory-read REGION NEW_OUTPUT_DIRECTORY | protocol-version | validate INTERFACE FRAME_COUNT [--fps FPS] | capture INTERFACE OUTPUT FRAME_COUNT [--fps FPS] | replay FILE [--decode] | export-image FILE OUTPUT.PNG | stereo-probe FRAME_COUNT [--fps FPS] | extract-frame RECORDING INDEX NEW_OUTPUT_DIRECTORY | stereo-capture FRAME_COUNT NEW_OUTPUT_DIRECTORY [--fps FPS] | reconstruct-stereo CALIBRATION LEFT.PNG RIGHT.PNG REQUEST.JSON OUTPUT_DIRECTORY"
                    .into(),
            );
        }
    }
    Ok(())
}
