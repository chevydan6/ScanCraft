//! Cooperative cancellation and bounded progress shared by geometry workers.
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Stage {
    Validating,
    Indexing,
    Processing,
    Preview,
    Reading,
    Writing,
    Finishing,
    ImageMapping,
    Resampling,
    Matching,
    Triangulating,
}
#[derive(Default)]
pub struct WorkControl {
    cancelled: AtomicBool,
    stage: AtomicU8,
    done: AtomicU64,
    total: AtomicU64,
}
impl WorkControl {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
    pub fn check(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Relaxed) {
            Err("Operation cancelled".into())
        } else {
            Ok(())
        }
    }
    pub fn update(&self, stage: Stage, done: usize, total: usize) -> Result<(), String> {
        self.check()?;
        self.total.store(total as u64, Ordering::Relaxed);
        self.done.store(done as u64, Ordering::Relaxed);
        self.stage.store(stage as u8, Ordering::Relaxed);
        Ok(())
    }
    pub fn progress(&self) -> (&'static str, f32) {
        let name = match self.stage.load(Ordering::Relaxed) {
            0 => "Validating",
            1 => "Building index",
            2 => "Processing",
            3 => "Preparing view",
            4 => "Reading",
            5 => "Writing",
            7 => "Building image maps",
            8 => "Resampling images",
            9 => "Matching stereo",
            10 => "Triangulating",
            _ => "Finishing",
        };
        let total = self.total.load(Ordering::Relaxed);
        let done = self.done.load(Ordering::Relaxed);
        (
            name,
            if total == 0 {
                0.
            } else {
                (done as f64 / total as f64).clamp(0., 1.) as f32
            },
        )
    }
}
