//! Publish only after the caller has validated the complete recording.
use crate::output::{publish, staging};
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};

pub fn publish_png(
    image: &image::DynamicImage,
    target: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let partial = staging(target)?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let staged = partial.join("image.png");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged)?;
        image.write_to(&mut file, image::ImageFormat::Png)?;
        file.sync_all()?;
        drop(file);
        publish(&staged, target)?;
        Ok(())
    })();
    result.map_err(|e| {
        format!(
            "{e}; unpublished image staging retained at {}",
            partial.display()
        )
    })?;
    fs::remove_dir(&partial).map_err(|e| {
        format!(
            "image published at {}, staging cleanup failed at {}: {e}",
            target.display(),
            partial.display()
        )
    })?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    File::open(parent)?.sync_all().map_err(|e| {
        format!(
            "image published at {}, parent sync failed: {e}",
            target.display()
        )
    })?;
    Ok(())
}
