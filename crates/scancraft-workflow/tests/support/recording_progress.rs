//! Test-only progress observation for the real file-backed recording workers.
//! No production callback, queue, admission or publication behavior is changed.
use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::{CaptureContext, Writer};
use std::{
    fs,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
#[derive(Clone, Default)]
struct Count(Arc<AtomicUsize>);
impl Write for Count {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.fetch_add(bytes.len(), Ordering::Relaxed);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct Layout {
    header: u64,
    frame: u64,
}
pub(super) struct RecordingProgress {
    target: PathBuf,
    paths: Mutex<Option<[PathBuf; 2]>>,
    layouts: [Layout; 2],
    pub(super) window: usize,
    // Observability for the controlled-stall regression, not a pacing signal.
    blocked: [AtomicUsize; 2],
}
impl RecordingProgress {
    pub(super) fn for_target(
        target: &Path,
        profiles: [StreamProfile; 2],
        context: CaptureContext,
        prototypes: &[Frame; 2],
    ) -> Self {
        let layouts = std::array::from_fn(|side| {
            let count = Count::default();
            let mut writer =
                Writer::with_context(count.clone(), profiles[side].clone(), context.clone())
                    .unwrap();
            let header = count.0.load(Ordering::Relaxed) as u64;
            writer.push(&prototypes[side]).unwrap();
            let frame = count.0.load(Ordering::Relaxed) as u64 - header;
            Layout { header, frame }
        });
        // runner/recording::Sink and CappedFile both use BufWriter::new. Permit
        // that default buffer plus one complete frame to force observable writes.
        // The fixed-size synthetic frames have the exact serialized lengths above.
        let buffer = BufWriter::new(Vec::<u8>::new()).capacity() as u64;
        let window = layouts
            .iter()
            .map(|l| buffer.div_ceil(l.frame) as usize + 1)
            .max()
            .unwrap();
        assert!(
            window < 8,
            "fixture must leave headroom in the unchanged eight-frame queue"
        );
        Self {
            target: target.to_owned(),
            paths: Mutex::new(None),
            layouts,
            window,
            blocked: Default::default(),
        }
    }
    fn paths(&self) -> io::Result<Option<[PathBuf; 2]>> {
        let mut paths = self.paths.lock().unwrap();
        if paths.is_none() {
            let prefix = format!(
                "{}.partial-",
                self.target.file_name().unwrap().to_str().unwrap()
            );
            let candidates = fs::read_dir(self.target.parent().unwrap())?
                .map(|e| e.map(|e| e.path()))
                .collect::<io::Result<Vec<_>>>()?
                .into_iter()
                .filter(|p| {
                    p.is_dir()
                        && p.file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with(&prefix)
                })
                .collect::<Vec<_>>();
            if candidates.len() > 1 {
                return Err(io::Error::other("ambiguous fixture staging directory"));
            }
            if let Some(root) = candidates.first() {
                *paths = Some([root.join("left.raptor"), root.join("right.raptor")]);
            }
        }
        Ok(paths.clone())
    }
    pub(super) fn ready(&self, emitted: usize, side: usize) -> io::Result<bool> {
        // After N observed complete serialized frames, at most `window` further
        // frames may be admitted. Even a stopped consumer cannot fill eight slots.
        let ready = if emitted < self.window {
            true
        } else if let Some(paths) = self.paths()? {
            let required = emitted + 1 - self.window;
            paths
                .iter()
                .zip(&self.layouts)
                .try_fold(true, |ready, (path, layout)| {
                    let length = match fs::metadata(path) {
                        Ok(m) => m.len(),
                        Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
                        Err(e) => return Err(e),
                    };
                    Ok::<_, io::Error>(
                        ready && length >= layout.header + required as u64 * layout.frame,
                    )
                })?
        } else {
            false
        };
        self.blocked[side].store(if ready { 0 } else { emitted }, Ordering::Release);
        Ok(ready)
    }
}

#[cfg(test)]
#[path = "recording_progress_tests.rs"]
mod tests;
