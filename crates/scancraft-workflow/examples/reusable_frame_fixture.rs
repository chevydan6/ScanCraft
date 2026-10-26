//! Reproducible synthetic frame document, unrelated to scanner calibration.
use scancraft_geometry::{coordinate_frame::Frame, work::WorkControl};
use scancraft_workflow::frame_document::{Document, save};
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let target = std::env::args_os()
        .nth(1)
        .ok_or("usage: reusable_frame_fixture NEW_FOLDER")?;
    let document = Document::new(
        "Synthetic reference frame".into(),
        Frame::from_points([10., 20., 30.], [10., 22., 30.], [9., 21., 30.])?,
    )?;
    save(
        &document,
        std::path::Path::new(&target),
        &WorkControl::default(),
    )?;
    Ok(())
}
