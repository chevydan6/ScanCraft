use scancraft_core::{Frame, StreamProfile};
use scancraft_geometry::work::WorkControl;
use scancraft_recording::{Reader, Writer};
use scancraft_workflow::recording_recovery;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "scancraft-recover-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
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
fn fixture(complete: bool) -> Vec<u8> {
    let profile = StreamProfile {
        interface: 0,
        width: 2,
        height: 2,
        fourcc: *b"GREY",
        fps: 30,
    };
    let mut bytes = Vec::new();
    {
        let mut writer = Writer::new(&mut bytes, profile).unwrap();
        for i in 0..3_u32 {
            let mut metadata = vec![0; 16];
            metadata[10] = 6;
            metadata[11] = 4;
            metadata[12..16].copy_from_slice(&(1000 + i * 33337).to_le_bytes());
            writer
                .push(&Frame {
                    sequence: i,
                    host_timestamp_us: 100 + i as u64 * 33337,
                    flags: 0,
                    metadata,
                    data: vec![i as u8; 4],
                })
                .unwrap();
        }
        if complete {
            writer.finish().unwrap();
        }
    }
    bytes
}
#[test]
fn publication_replays_complete_prefix_preserves_source_and_refuses_overwrite() {
    let temp = Temp::new();
    let input = temp.0.join("source.partial");
    let bytes = fixture(false);
    fs::write(&input, &bytes).unwrap();
    let output = temp.0.join("recovered");
    let summary =
        recording_recovery::run(&input, &output, 100, &WorkControl::default(), &|| Ok(())).unwrap();
    assert_eq!(summary["recovered_frames"], 3);
    assert_eq!(summary["source_complete"], false);
    assert_eq!(summary["full_crc_footer_replay"], "PASS");
    assert_eq!(fs::read(&input).unwrap(), bytes);
    let mut reader = Reader::new(File::open(output.join("recovered.raptor")).unwrap()).unwrap();
    for i in 0..3 {
        assert_eq!(reader.read_frame().unwrap().unwrap().data, vec![i; 4]);
    }
    assert!(reader.read_frame().unwrap().is_none());
    fs::write(output.join("sentinel"), b"keep").unwrap();
    assert!(
        recording_recovery::run(&input, &output, 100, &WorkControl::default(), &|| Ok(())).is_err()
    );
    assert_eq!(fs::read(output.join("sentinel")).unwrap(), b"keep");
}
#[test]
fn cancellation_and_bad_source_never_publish() {
    let temp = Temp::new();
    let input = temp.0.join("source.partial");
    let bytes = fixture(false);
    fs::write(&input, &bytes).unwrap();
    let work = WorkControl::default();
    work.cancel();
    let output = temp.0.join("cancelled");
    assert!(recording_recovery::run(&input, &output, 100, &work, &|| Ok(())).is_err());
    assert!(!output.exists());
    assert_eq!(fs::read(&input).unwrap(), bytes);
    let output = temp.0.join("caller-cancelled");
    let calls = AtomicU64::new(0);
    assert!(
        recording_recovery::run(&input, &output, 100, &WorkControl::default(), &|| if calls
            .fetch_add(1, Ordering::Relaxed)
            > 4
        {
            Err("cancelled by test".into())
        } else {
            Ok(())
        })
        .is_err()
    );
    assert!(!output.exists());
    assert_eq!(fs::read(&input).unwrap(), bytes);
    fs::write(&input, b"bad header").unwrap();
    let output = temp.0.join("bad");
    assert!(
        recording_recovery::run(&input, &output, 100, &WorkControl::default(), &|| Ok(())).is_err()
    );
    assert!(!output.exists());
}
#[test]
fn source_change_during_recovery_blocks_publication_even_with_valid_output() {
    let temp = Temp::new();
    let input = temp.0.join("source.raptor");
    let bytes = fixture(true);
    fs::write(&input, &bytes).unwrap();
    let output = temp.0.join("result");
    let calls = AtomicU64::new(0);
    let error = recording_recovery::run(&input, &output, 100, &WorkControl::default(), &|| {
        if calls.fetch_add(1, Ordering::Relaxed) == 1 {
            OpenOptions::new()
                .append(true)
                .open(&input)?
                .write_all(&[0])?;
        }
        Ok(())
    })
    .unwrap_err();
    assert!(error.to_string().contains("source changed"));
    assert!(!output.exists());
    assert_eq!(fs::read(&input).unwrap(), [bytes, vec![0]].concat());
}
#[test]
fn symlinks_and_special_files_are_rejected_without_reading() {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;
    let temp = Temp::new();
    let input = temp.0.join("file");
    fs::write(&input, fixture(false)).unwrap();
    let link = temp.0.join("link");
    symlink(&input, &link).unwrap();
    assert!(
        recording_recovery::run(
            &link,
            &temp.0.join("out-link"),
            100,
            &WorkControl::default(),
            &|| Ok(())
        )
        .is_err()
    );
    let fifo = temp.0.join("fifo");
    let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    // SAFETY: valid owned path; create a private test FIFO that the nonblocking loader must reject.
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert!(
        recording_recovery::run(
            &fifo,
            &temp.0.join("out-fifo"),
            100,
            &WorkControl::default(),
            &|| Ok(())
        )
        .is_err()
    );
}
