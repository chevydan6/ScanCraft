//! Explicit recorded-frame selection feeding the same calibrated image pipeline.
//! Selection does not establish synchronized exposures or physical calibration.
use crate::{
    Digest, Error, Prepared, Request, artifact, bounded, digest, process_images,
    recording_playback::Playback,
};
use scancraft_calibration::storage;
use scancraft_depth::MatchSource;
use scancraft_geometry::work::WorkControl;
use scancraft_recording::{Writer, selection::SelectedFrame};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inputs {
    pub calibration: PathBuf,
    pub left: PathBuf,
    pub left_index: usize,
    pub right: PathBuf,
    pub right_index: usize,
    pub settings: PathBuf,
}
pub(crate) struct Sources {
    calibration_bytes: Vec<u8>,
    request_bytes: Vec<u8>,
    lineage: Value,
    left: SelectedFrame,
    right: SelectedFrame,
}
impl Sources {
    pub(crate) fn project_origin(
        &self,
        calibration: &storage::CalibrationRecord,
        bundle: &Path,
    ) -> Result<scancraft_geometry::project_origin::Origin, Error> {
        let mut artifacts = Vec::new();
        for (name, bytes) in [
            ("calibration.rapcal", &self.calibration_bytes),
            ("request.json", &self.request_bytes),
        ] {
            artifacts.push(scancraft_geometry::project_origin::Artifact {
                name: name.into(),
                bytes: bytes.len() as u64,
                crc32: crc32fast::hash(bytes),
            });
        }
        let mut sink = DigestSink::default();
        serde_json::to_writer_pretty(&mut sink, &self.lineage)?;
        artifacts.push(sink.reference("recording-sources.json"));
        for (name, selected) in [
            ("left-frame.raptor", &self.left),
            ("right-frame.raptor", &self.right),
        ] {
            let mut sink = DigestSink::default();
            let mut writer = Writer::with_context(
                &mut sink,
                selected.profile.clone(),
                selected.context.clone().unwrap_or_default(),
            )?;
            writer.push(&selected.frame)?;
            writer.finish()?;
            artifacts.push(sink.reference(name));
        }
        let origin = scancraft_geometry::project_origin::Origin {
            schema_version: 1,
            kind: "initial_recorded_stereo_reconstruction".into(),
            bundle_directory_display: std::path::absolute(bundle)?.display().to_string(),
            calibration: serde_json::to_value(calibration)?,
            request: serde_json::from_slice(&self.request_bytes)?,
            recording_sources: self.lineage.clone(),
            artifacts,
        };
        origin.validate()?;
        Ok(origin)
    }
    pub(crate) fn write_artifacts(
        &self,
        path: &Path,
        work: &WorkControl,
        artifacts: &mut Vec<Digest>,
    ) -> Result<(), Error> {
        for (name, bytes) in [
            ("calibration.rapcal", &self.calibration_bytes),
            ("request.json", &self.request_bytes),
        ] {
            artifacts.push(artifact(path, name, work, |out| {
                std::io::Write::write_all(out, bytes)?;
                Ok(())
            })?);
        }
        artifacts.push(artifact(path, "recording-sources.json", work, |out| {
            serde_json::to_writer_pretty(out, &self.lineage)?;
            Ok(())
        })?);
        for (name, selected) in [
            ("left-frame.raptor", &self.left),
            ("right-frame.raptor", &self.right),
        ] {
            artifacts.push(artifact(path, name, work, |out| {
                let mut writer = Writer::with_context(
                    out,
                    selected.profile.clone(),
                    selected.context.clone().unwrap_or_default(),
                )?;
                writer.push(&selected.frame)?;
                writer.finish()?;
                Ok(())
            })?);
        }
        Ok(())
    }
}
pub(crate) fn lineage(path: &Path, version: u8, selected: &SelectedFrame) -> Result<Value, Error> {
    Ok(
        json!({"source_path_display":path.display().to_string(),"source_path_display_is_lossy":path.to_str().is_none(),"source_version":version,"source_header_crc_verified":version>=2,
        "source_total_frames":selected.total_frames,"selected_frame_index":selected.frame_index,
        "profile":selected.profile,"context":selected.context,"sequence":selected.frame.sequence,
        "host_timestamp_us":selected.frame.host_timestamp_us,"sensor_pts":scancraft_core::uvc_pts(&selected.frame.metadata)?,
        "payload_crc32":crc32fast::hash(&selected.frame.data),"metadata_crc32":crc32fast::hash(&selected.frame.metadata),
        "source_integrity":"all frames, order, footer and EOF validated before selection","source_file_identity_checked":true}),
    )
}
pub fn prepare(inputs: &Inputs, work: &WorkControl) -> Result<Prepared, Error> {
    let calibration_bytes = bounded(&inputs.calibration, 1024 * 1024 + 16, work)?;
    let calibration = storage::read(calibration_bytes.as_slice())?;
    let request_bytes = bounded(&inputs.settings, 4 * 1024 * 1024, work)?;
    let request: Request = serde_json::from_slice(&request_bytes)?;
    if request.schema_version != 1 {
        return Err("unsupported reconstruction request schema".into());
    }
    let mut left_source = Playback::open(&inputs.left, work)?;
    let left = left_source.select_raw(inputs.left_index, work)?;
    let mut right_source = Playback::open(&inputs.right, work)?;
    let right = right_source.select_raw(inputs.right_index, work)?;
    for (selected, interface, size) in [
        (
            &left,
            0,
            [
                calibration.geometry.left.width,
                calibration.geometry.left.height,
            ],
        ),
        (
            &right,
            2,
            [
                calibration.geometry.right.width,
                calibration.geometry.right.height,
            ],
        ),
    ] {
        if selected.profile.interface != interface
            || selected.profile.fourcc != *b"GREY"
            || [selected.profile.width, selected.profile.height] != size
        {
            return Err("recorded stereo requires left interface 0 / right interface 2 packed GREY frames with exact calibration dimensions".into());
        }
        if let Some(context) = &selected.context
            && let Some(bytes) = &context.calibration_bytes
        {
            let embedded = storage::read(bytes.as_slice())?;
            if embedded != calibration {
                return Err(
                    "embedded recording calibration disagrees with supplied calibration".into(),
                );
            }
        }
        match &request.source {
            MatchSource::SyntheticFixture => {
                if selected
                    .context
                    .as_ref()
                    .and_then(|c| c.controls.get("synthetic"))
                    .and_then(Value::as_bool)
                    != Some(true)
                {
                    return Err(
                        "synthetic reconstruction requires explicitly synthetic recording contexts"
                            .into(),
                    );
                }
            }
            MatchSource::Observations(binding) => {
                calibration.validate_for_capture(binding)?;
                if let Some(context) = &selected.context {
                    if context.controls.get("synthetic").and_then(Value::as_bool) == Some(true) {
                        return Err(
                            "synthetic recording cannot be declared physical observations".into(),
                        );
                    }
                    for (key, expected) in
                        [("firmware", &binding.firmware), ("mode", &binding.mode)]
                    {
                        if let Some(value) = context.controls.get(key)
                            && value.as_str() != Some(expected.as_str())
                        {
                            return Err(format!(
                                "recording {key} disagrees with declared calibration binding"
                            )
                            .into());
                        }
                    }
                    if context
                        .device_identity
                        .as_ref()
                        .is_some_and(|identity| identity != &binding.serial)
                    {
                        return Err(
                            "recording device identity disagrees with declared calibration binding"
                                .into(),
                        );
                    }
                }
            }
        }
    }
    let sources = Sources {
        calibration_bytes: calibration_bytes.clone(),
        request_bytes: request_bytes.clone(),
        lineage: json!({"schema_version":1,"pair_selection":"explicit user-selected frame indices; no automatic exposure pairing",
        "left":lineage(&inputs.left,left_source.version(),&left)?,"right":lineage(&inputs.right,right_source.version(),&right)?,
        "binding_verification":"source declaration validated by reconstruction pipeline; recording context cross-checked where present; device binding not independently attested",
        "source_device_binding_verified":false,"synchronization_verified":false,"physical_accuracy_qualified":false,
        "recovery_provenance":"keep original recovery reports; complete recording footer does not prove original scan completeness",
        "retained_frames":["left-frame.raptor","right-frame.raptor"],"retained_frame_format":2,"source_paths_are_references_not_immutable_snapshots":true}),
        left,
        right,
    };
    let digests = vec![
        digest("calibration", &calibration_bytes),
        digest("request", &request_bytes),
        digest("left_raw_payload", &sources.left.frame.data),
        digest("left_raw_metadata", &sources.left.frame.metadata),
        digest("right_raw_payload", &sources.right.frame.data),
        digest("right_raw_metadata", &sources.right.frame.metadata),
    ];
    let mut prepared = process_images(
        calibration,
        request,
        &sources.left.frame.data,
        &sources.right.frame.data,
        digests,
        work,
    )?;
    left_source.check_source()?;
    right_source.check_source()?;
    work.check()?;
    prepared.recording_sources = Some(sources);
    Ok(prepared)
}

#[derive(Default)]
struct DigestSink {
    hash: crc32fast::Hasher,
    bytes: u64,
}
impl std::io::Write for DigestSink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.hash.update(bytes);
        self.bytes += bytes.len() as u64;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl DigestSink {
    fn reference(self, name: &str) -> scancraft_geometry::project_origin::Artifact {
        scancraft_geometry::project_origin::Artifact {
            name: name.into(),
            bytes: self.bytes,
            crc32: self.hash.finalize(),
        }
    }
}
