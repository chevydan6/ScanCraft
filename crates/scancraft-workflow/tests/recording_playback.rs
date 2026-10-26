use scancraft_core::{Frame, StreamProfile};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::Writer;
use scancraft_workflow::recording_playback::Playback;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "scancraft-playback-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn recording(fourcc: [u8; 4], payload: Vec<u8>) -> Vec<u8> {
    let mut writer = Writer::new(
        Vec::new(),
        StreamProfile {
            interface: 0,
            width: 2,
            height: 2,
            fourcc,
            fps: 30,
        },
    )
    .unwrap();
    for i in 0..3 {
        let mut metadata = vec![0; 16];
        metadata[10] = 6;
        metadata[11] = 4;
        metadata[12..16].copy_from_slice(&(100 + i * 10u32).to_le_bytes());
        writer
            .push(&Frame {
                sequence: i,
                host_timestamp_us: 1000 + u64::from(i) * 10,
                flags: 0,
                metadata,
                data: payload.clone(),
            })
            .unwrap();
    }
    writer.finish().unwrap()
}
#[test]
fn gray_navigation_preserves_raw_source_and_context() {
    let t = Temp::new();
    let p = t.0.join("source");
    let bytes = recording(*b"GREY", vec![0, 64, 128, 255]);
    fs::write(&p, &bytes).unwrap();
    let work = WorkControl::default();
    let mut playback = Playback::open(&p, &work).unwrap();
    for index in [2, 0, 1, 2] {
        let view = playback.read(index, &work).unwrap();
        assert_eq!(view.index, index);
        assert_eq!(view.total, 3);
        assert_eq!(
            view.image.as_raw(),
            &vec![0, 0, 0, 64, 64, 64, 128, 128, 128, 255, 255, 255]
        );
        assert!(view.description.contains("not_loaded_or_verified"));
        assert_eq!(view.sensor_pts, Some(100 + index as u32 * 10));
    }
    assert!(playback.read(3, &work).is_err());
    assert!(playback.read(0, &work).is_ok());
    assert_eq!(fs::read(p).unwrap(), bytes);
}
#[test]
fn detects_in_place_changes_and_path_replacement() {
    let t = Temp::new();
    let p = t.0.join("source");
    let bytes = recording(*b"GREY", vec![1; 4]);
    fs::write(&p, &bytes).unwrap();
    let work = WorkControl::default();
    let mut playback = Playback::open(&p, &work).unwrap();
    let replacement = t.0.join("other");
    fs::write(&replacement, &bytes).unwrap();
    fs::rename(replacement, &p).unwrap();
    assert!(
        playback
            .read(0, &work)
            .err()
            .unwrap()
            .to_string()
            .contains("source changed")
    );
    let mut playback = Playback::open(&p, &work).unwrap();
    let mut changed = bytes;
    changed.push(0);
    fs::write(&p, changed).unwrap();
    assert!(
        playback
            .read(0, &work)
            .err()
            .unwrap()
            .to_string()
            .contains("source changed")
    );
}
#[test]
fn refuses_symlinks_fifos_bad_footer_and_cancellation() {
    let t = Temp::new();
    let p = t.0.join("source");
    let mut bytes = recording(*b"GREY", vec![1; 4]);
    fs::write(&p, &bytes).unwrap();
    let work = WorkControl::default();
    let link = t.0.join("link");
    std::os::unix::fs::symlink(&p, &link).unwrap();
    assert!(Playback::open(&link, &work).is_err());
    let fifo = t.0.join("fifo");
    let name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert!(Playback::open(&fifo, &work).is_err());
    let mut playback = Playback::open(&p, &work).unwrap();
    work.cancel();
    assert!(playback.read(0, &work).is_err());
    assert!(Playback::open(&p, &work).is_err());
    bytes.pop();
    fs::write(&p, bytes).unwrap();
    assert!(Playback::open(&p, &WorkControl::default()).is_err());
}
fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut bytes)
        .encode(
            &vec![128; (width * height * 3) as usize],
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )
        .unwrap();
    bytes
}
#[test]
fn jpeg_dimensions_and_decode_errors_cannot_hide_behind_raw_integrity() {
    let t = Temp::new();
    let p = t.0.join("source");
    let work = WorkControl::default();
    fs::write(&p, recording(*b"MJPG", jpeg(2, 2))).unwrap();
    let mut playback = Playback::open(&p, &work).unwrap();
    assert_eq!(playback.read(0, &work).unwrap().image.dimensions(), (2, 2));
    fs::write(&p, recording(*b"MJPG", jpeg(3, 2))).unwrap();
    let mut playback = Playback::open(&p, &work).unwrap();
    assert!(playback.read(0, &work).is_err());
    fs::write(&p, recording(*b"MJPG", vec![0xff, 0xd8, 0, 0xff, 0xd9])).unwrap();
    let mut playback = Playback::open(&p, &work).unwrap();
    assert!(playback.read(0, &work).is_err());
}
#[test]
fn y14_display_mapping_is_explicit_and_raw_bytes_are_unchanged() {
    let t = Temp::new();
    let p = t.0.join("source");
    let bytes = recording(*b"Y14 ", vec![0, 0, 0, 1, 255, 63, 255, 255]);
    fs::write(&p, &bytes).unwrap();
    let work = WorkControl::default();
    let mut playback = Playback::open(&p, &work).unwrap();
    let view = playback.read(0, &work).unwrap();
    assert!(view.description.contains("16-bit container range"));
    assert_eq!(
        view.image.as_raw(),
        &vec![0, 0, 0, 1, 1, 1, 63, 63, 63, 255, 255, 255]
    );
    assert_eq!(fs::read(p).unwrap(), bytes);
}
