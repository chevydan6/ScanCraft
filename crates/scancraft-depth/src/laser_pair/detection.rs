//! Independent camera detection with at most one additional scoped worker.
use super::{Observations, Settings, detect};
use scancraft_geometry::work::WorkControl;
use scancraft_image::GrayImage;

// Conservative scheduling threshold, not an accuracy parameter. Saved Raptor
// pairs and >=640x480 synthetic images benefit; tiny images incur spawn overhead.
const MIN_PARALLEL_PIXELS: u64 = 256 * 1024;

pub(super) fn pair(
    images: [GrayImage<'_>; 2],
    settings: &Settings,
    work: &WorkControl,
    parallel: bool,
) -> Result<[Observations; 2], String> {
    work.check()?;
    let run = |side| {
        detect(
            images[side],
            &settings.detection,
            &settings.continuity,
            work,
        )
    };
    let serial = || Ok([run(0)?, run(1)?]);
    if !parallel
        || images.iter().any(|image| {
            u64::from(image.size()[0]) * u64::from(image.size()[1]) < MIN_PARALLEL_PIXELS
        })
    {
        return serial();
    }
    std::thread::scope(|scope| {
        let first = std::thread::Builder::new()
            .name("scancraft-laser-detect".into())
            .spawn_scoped(scope, || run(0));
        let Ok(first) = first else {
            // Resource pressure must not disable otherwise valid processing.
            return serial();
        };
        let second = run(1);
        // Join before returning either result. Preserve first-camera error
        // precedence and propagate panics; never detach borrowed frame work.
        let first = first
            .join()
            .unwrap_or_else(|p| std::panic::resume_unwind(p));
        Ok([first?, second?])
    })
}
