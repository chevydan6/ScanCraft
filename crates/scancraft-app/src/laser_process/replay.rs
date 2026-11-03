//! Explicit recording replay diagnostic; never opens scanner hardware.
use scancraft_core::{Frame, StreamProfile};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::CaptureContext;
use scancraft_workflow::{
    laser_stream::{
        StopToken,
        bundle::{Plan, PreviewInput, PreviewStatus},
    },
    live_stereo::runner::Source,
    recording_playback::Playback,
};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{Arc, atomic::AtomicBool},
};
struct Replay {
    source: Playback,
    index: usize,
    work: Arc<WorkControl>,
    stop: StopToken,
}
impl Source for Replay {
    fn context(&self) -> Result<CaptureContext, String> {
        Ok(CaptureContext {
            calibration_status: "recording_replay_unverified".into(),
            controls: serde_json::json!({"source_kind":"recording_replay"}),
            ..CaptureContext::default()
        })
    }
    fn next(&mut self) -> Result<Option<Frame>, String> {
        if self.stop.stop_requested() || self.index == self.source.len() {
            return Ok(None);
        }
        let frame = self
            .source
            .select_raw(self.index, &self.work)
            .map_err(|e| e.to_string())?
            .frame;
        self.index += 1;
        Ok(Some(frame))
    }
    fn statistics(&self) -> Value {
        serde_json::json!({"replayed":self.index,"source_kind":"recording_replay"})
    }
    fn close(self) -> Result<(), String> {
        self.source.check_source().map_err(|e| e.to_string())
    }
}
pub(super) fn run(
    paths: &[PathBuf],
    work: &Arc<WorkControl>,
    stop: &Arc<AtomicBool>,
    preview: impl FnMut(PreviewInput<'_>) -> PreviewStatus,
) -> Result<Value, String> {
    let plan = Plan::open(&paths[0], &paths[1], work)?;
    let recordings = [paths[2].clone(), paths[3].clone()];
    let w = work.clone();
    plan.capture(
        &paths[4],
        work,
        stop,
        move |c, p: StreamProfile, token| {
            let source = Playback::open(&recordings[c.index()], &w).map_err(|e| e.to_string())?;
            if source.profile() != &p {
                return Err("replay profile differs from supplied laser settings".into());
            }
            Ok(Replay {
                source,
                index: 0,
                work: w.clone(),
                stop: token,
            })
        },
        || Ok(()),
        preview,
    )
}
