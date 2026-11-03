use super::*;
use scancraft_geometry::work::WorkControl;
use serde_json::{Value, json};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
pub(super) struct Temp(pub(super) PathBuf);
impl Temp {
    pub(super) fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let p = std::env::temp_dir().join(format!(
            "scancraft-segments-{}-{}",
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
fn limits() -> Limits {
    Limits {
        segment_bytes: 4096,
        maximum_record_bytes: 2048,
        maximum_total_bytes: 100000,
    }
}
pub(super) fn record(index: u64, padding: usize) -> Value {
    json!({"schema_version":1,"frames":[{"recording_role":"first.raptor","frame_index":index,"sequence":index as u32,"host_timestamp_us":index+1,"flags":0,"payload_crc32":0,"metadata_crc32":0},{"recording_role":"second.raptor","frame_index":index,"sequence":index as u32,"host_timestamp_us":index+1,"flags":0,"payload_crc32":0,"metadata_crc32":0}],"raw_pts":index as u32,"result":{"Err":"x".repeat(padding)}})
}
#[test]
fn rollover_streams_exact_records_and_index_spans_with_final_integrity() {
    let t = Temp::new();
    let work = WorkControl::default();
    let mut out = Journal::new(&t.0, limits()).unwrap();
    let mut expected = Vec::new();
    for i in 0..13 {
        let r = record(i, 900);
        let mut bytes = serde_json::to_vec(&r).unwrap();
        bytes.push(b'\n');
        expected.push(bytes);
        out.push(&r, &work).unwrap();
    }
    let done = out.finish().unwrap();
    assert!(done.segments > 1);
    assert_eq!(done.pairs, 13);
    assert_eq!(
        done.bytes,
        expected.iter().map(|b| b.len() as u64).sum::<u64>()
    );
    let mut actual = Vec::new();
    let report = visit(&t.0, Mode::Complete, &work, |index, bytes, origin| {
        assert_eq!(origin, RecordOrigin::Indexed);
        assert_eq!(index, actual.len() as u64);
        actual.push(bytes.to_vec());
        Ok(())
    })
    .unwrap();
    assert_eq!(actual, expected);
    assert!(report.journal_complete);
    assert_eq!(report.pairs, 13);
    assert!(!report.physical_accuracy_qualified);
    assert!(!report.source_restoration_verified);
    for d in done.artifacts {
        let bytes = fs::read(t.0.join(d.role)).unwrap();
        assert_eq!(d.bytes, bytes.len() as u64);
        assert_eq!(d.crc32, crc32fast::hash(&bytes));
    }
}
#[test]
fn recovery_keeps_indexed_and_unindexed_complete_prefix_and_marks_torn_tail() {
    let t = Temp::new();
    let work = WorkControl::default();
    let mut out = Journal::new(&t.0, limits()).unwrap();
    for i in 0..4 {
        out.push(&record(i, 900), &work).unwrap();
    }
    drop(out);
    let mut files = fs::read_dir(&t.0)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("pairs-")
        })
        .collect::<Vec<_>>();
    files.sort();
    use std::io::Write;
    let mut tail = fs::OpenOptions::new()
        .append(true)
        .open(files.last().unwrap())
        .unwrap();
    tail.write_all(b"{\"torn\":").unwrap();
    drop(tail);
    assert!(visit(&t.0, Mode::Complete, &work, |_, _, _| Ok(())).is_err());
    let mut origins = Vec::new();
    let report = visit(&t.0, Mode::Recovery, &work, |_, _, origin| {
        origins.push(origin);
        Ok(())
    })
    .unwrap();
    assert!(!report.journal_complete);
    assert_eq!(report.pairs, 4);
    assert!(report.indexed_pairs > 0);
    assert!(report.unindexed_complete_pairs > 0);
    assert_eq!(report.trailing_incomplete_bytes, 8);
    assert!(origins.contains(&RecordOrigin::Unindexed));
    assert!(!t.0.join("manifest.json").exists());
}
#[test]
fn quotas_oversized_records_cancellation_and_segment_creation_failure_never_complete() {
    for kind in 0..4 {
        let t = Temp::new();
        let work = WorkControl::default();
        let mut l = limits();
        if kind == 0 {
            l.maximum_total_bytes = 4096;
        }
        let mut out = Journal::new(&t.0, l).unwrap();
        out.push(&record(0, 900), &work).unwrap();
        let failed = match kind {
            0 => (1..10)
                .find_map(|i| out.push(&record(i, 900), &work).err())
                .unwrap(),
            1 => out.push(&record(1, 3000), &work).unwrap_err(),
            2 => {
                work.cancel();
                out.push(&record(1, 900), &work).unwrap_err()
            }
            _ => {
                fs::create_dir(t.0.join(role(1))).unwrap();
                (1..10)
                    .find_map(|i| out.push(&record(i, 900), &work).err())
                    .unwrap()
            }
        };
        assert!(!failed.to_string().is_empty());
        assert!(out.finish().is_err());
        assert!(
            visit(&t.0, Mode::Complete, &WorkControl::default(), |_, _, _| Ok(
                ()
            ))
            .is_err()
        );
    }
}
#[test]
fn corrupt_sealed_bytes_footer_paths_and_record_order_fail_closed() {
    for mode in 0..4 {
        let t = Temp::new();
        let work = WorkControl::default();
        let mut out = Journal::new(&t.0, limits()).unwrap();
        out.push(&record(0, 900), &work).unwrap();
        out.push(&record(1, 900), &work).unwrap();
        let _done = out.finish().unwrap();
        match mode {
            0 => {
                let file = t.0.join(role(0));
                let mut bytes = fs::read(&file).unwrap();
                bytes[20] ^= 1;
                fs::write(file, bytes).unwrap()
            }
            1 => {
                let file = t.0.join(INDEX_NAME);
                let mut bytes = fs::read(&file).unwrap();
                bytes.extend_from_slice(b"{}\n");
                fs::write(file, bytes).unwrap()
            }
            2 => {
                fs::rename(t.0.join(role(0)), t.0.join("saved")).unwrap();
                std::os::unix::fs::symlink("saved", t.0.join(role(0))).unwrap()
            }
            _ => fs::write(t.0.join(role(1)), b"{}\n").unwrap(),
        }
        assert!(visit(&t.0, Mode::Complete, &work, |_, _, _| Ok(())).is_err());
    }
    let t = Temp::new();
    let work = WorkControl::default();
    let mut out = Journal::new(&t.0, limits()).unwrap();
    out.push(&record(0, 0), &work).unwrap();
    out.push(&record(0, 0), &work).unwrap();
    out.finish().unwrap();
    assert!(
        visit(&t.0, Mode::Complete, &work, |_, _, _| Ok(()))
            .unwrap_err()
            .contains("regress")
    );
}
#[test]
fn limits_bound_segment_slots_metadata_and_legacy_digest_compatibility() {
    for l in [
        Limits {
            segment_bytes: 4096,
            maximum_record_bytes: 4096,
            maximum_total_bytes: 4096,
        },
        Limits {
            segment_bytes: 1 << 31,
            ..limits()
        },
        Limits {
            maximum_record_bytes: MAX_RECORD_BYTES + 1,
            ..limits()
        },
        Limits {
            maximum_total_bytes: u64::MAX,
            ..limits()
        },
    ] {
        assert!(l.validate().is_err());
    }
    let t = Temp::new();
    let work = WorkControl::default();
    let mut bytes = serde_json::to_vec(&record(0, 0)).unwrap();
    bytes.push(b'\n');
    fs::write(t.0.join("pairs.ndjson"), &bytes).unwrap();
    assert!(visit(&t.0, Mode::Complete, &work, |_, _, _| Ok(())).is_err());
    let prefix = visit(&t.0, Mode::Recovery, &work, |_, _, origin| {
        assert_eq!(origin, RecordOrigin::LegacyPrefix);
        Ok(())
    })
    .unwrap();
    assert_eq!(prefix.pairs, 1);
    assert!(!prefix.journal_complete);
    fs::write(t.0.join("manifest.json"),serde_json::to_vec(&json!({"schema_version":1,"kind":"laser_candidate_capture_artifacts","artifacts":[{"role":"pairs.ndjson","bytes":bytes.len(),"crc32":crc32fast::hash(&bytes)}]})).unwrap()).unwrap();
    assert!(
        visit(&t.0, Mode::Complete, &work, |_, _, _| Ok(()))
            .unwrap()
            .journal_complete
    );
    work.cancel();
    assert!(visit(&t.0, Mode::Complete, &work, |_, _, _| Ok(())).is_err());
}
#[test]
#[ignore = "writes and verifies more than1GiB; run explicitly for long-session storage changes"]
fn segmented_journal_crosses_old_one_gib_limit_with_bounded_live_state() {
    let t = Temp::new();
    let work = WorkControl::default();
    let l = Limits {
        segment_bytes: 16 * 1024 * 1024,
        maximum_record_bytes: 256 * 1024,
        maximum_total_bytes: 2 * (1 << 30),
    };
    let mut out = Journal::new(&t.0, l).unwrap();
    for i in 0..9000 {
        out.push(&record(i, 128 * 1024), &work).unwrap();
    }
    let done = out.finish().unwrap();
    assert!(done.bytes > 1 << 30);
    let report = visit(&t.0, Mode::Complete, &work, |_, _, _| Ok(())).unwrap();
    assert_eq!(report.pairs, 9000);
    assert_eq!(report.bytes, done.bytes);
    assert!(report.journal_complete);
    println!(
        "pairs={} bytes={} segments={}",
        done.pairs, done.bytes, done.segments
    );
}

#[test]
fn recovery_ignores_torn_bundle_manifest_but_complete_mode_and_midread_mutation_fail() {
    let t = Temp::new();
    let work = WorkControl::default();
    let mut out = Journal::new(&t.0, limits()).unwrap();
    out.push(&record(0, 0), &work).unwrap();
    out.push(&record(1, 0), &work).unwrap();
    out.finish().unwrap();
    fs::write(t.0.join("manifest.json"), b"{\"torn\":").unwrap();
    assert!(visit(&t.0, Mode::Complete, &work, |_, _, _| Ok(())).is_err());
    let recovered = visit(&t.0, Mode::Recovery, &work, |_, _, _| Ok(())).unwrap();
    assert_eq!(recovered.pairs, 2);
    assert!(recovered.recovery_inspection);
    assert!(!recovered.source_restoration_verified);
    fs::remove_file(t.0.join("manifest.json")).unwrap();
    let error = visit(&t.0, Mode::Complete, &work, |i, _, _| {
        if i == 0 {
            use std::io::Write;
            fs::OpenOptions::new()
                .append(true)
                .open(t.0.join(INDEX_NAME))
                .unwrap()
                .write_all(b" ")
                .unwrap();
        }
        Ok(())
    })
    .unwrap_err();
    assert!(error.contains("changed"));
}
