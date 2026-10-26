//! Generate a synthetic offline dataset. Does not access a scanner.
#[path = "../tests/common/mod.rs"]
mod common;
fn main() {
    let path = std::path::PathBuf::from(
        std::env::args_os()
            .nth(1)
            .expect("supply a new output directory"),
    );
    std::fs::create_dir(&path).expect("dataset directory must not already exist");
    common::dataset(&path, false);
    println!(
        "Synthetic fixture written to {}. Not Raptor calibration.",
        path.display()
    );
}
