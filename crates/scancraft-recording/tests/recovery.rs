use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::{
    CaptureContext, Reader, Writer,
    recovery::{StopReason, recover_prefix},
};
use std::io::{self, Cursor, Read};
fn profile() -> StreamProfile {
    StreamProfile {
        interface: 0,
        width: 2,
        height: 2,
        fourcc: *b"GREY",
        fps: 30,
    }
}
fn frame(i: u32) -> Frame {
    let mut metadata = vec![0; 16];
    metadata[10] = 6;
    metadata[11] = 4;
    metadata[12..16].copy_from_slice(&(1000 + i * 33337).to_le_bytes());
    Frame {
        sequence: i,
        host_timestamp_us: 100 + i as u64 * 33337,
        flags: 0,
        metadata,
        data: vec![i as u8; 4],
    }
}
fn fixture() -> (Vec<u8>, Vec<usize>) {
    let mut bytes = Vec::new();
    let mut ends = Vec::new();
    let context = CaptureContext {
        device_identity: Some("synthetic".into()),
        calibration_bytes: Some(vec![1, 2, 3]),
        calibration_status: "synthetic_unqualified".into(),
        controls: serde_json::json!({"gain":3}),
    };
    {
        let mut w = Writer::with_context(&mut bytes, profile(), context).unwrap();
        for i in 0..3 {
            w.push(&frame(i)).unwrap();
        }
        w.finish().unwrap();
    }
    let h = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let mut offset = 16 + h;
    for _ in 0..3 {
        let n = u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 8 + n;
        ends.push(offset);
    }
    (bytes, ends)
}
fn recovered_frames(bytes: &[u8]) -> Vec<Frame> {
    let mut r = Reader::new(bytes).unwrap();
    let mut frames = Vec::new();
    while let Some(f) = r.read_frame().unwrap() {
        frames.push(f);
    }
    frames
}
#[test]
fn every_truncation_after_first_frame_preserves_exact_validated_prefix_and_context() {
    let (source, ends) = fixture();
    let context = serde_json::to_value(Reader::new(source.as_slice()).unwrap().context).unwrap();
    for end in ends[0]..=source.len() {
        let mut output = Vec::new();
        let report = recover_prefix(&source[..end], &mut output, 100, &|_| Ok(())).unwrap();
        let expected = ends.iter().filter(|&&n| n <= end).count();
        assert_eq!(report.recovered_frames, expected as u64);
        assert_eq!(report.validated_prefix_end_bytes, ends[expected - 1] as u64);
        assert_eq!(
            recovered_frames(&output),
            (0..expected as u32).map(frame).collect::<Vec<_>>()
        );
        let reader = Reader::new(output.as_slice()).unwrap();
        assert_eq!(serde_json::to_value(reader.context).unwrap(), context);
        assert_eq!(report.source_complete, end == source.len());
        assert_eq!(
            report.source_bytes_read_crc32,
            crc32fast::hash(&source[..end])
        );
    }
}
#[test]
fn corrupt_middle_frame_never_resynchronizes_to_later_valid_frame() {
    let (mut source, ends) = fixture();
    source[ends[0] + 8] ^= 1;
    let mut output = Vec::new();
    let report = recover_prefix(source.as_slice(), &mut output, 100, &|_| Ok(())).unwrap();
    assert_eq!(report.recovered_frames, 1);
    assert!(report.source_issue.unwrap().contains("CRC"));
    assert_eq!(recovered_frames(&output), vec![frame(0)]);
}
#[test]
fn crc_valid_semantic_error_and_bad_footer_stop_without_relaxing_validation() {
    let (mut source, ends) = fixture();
    let n = u32::from_le_bytes(source[ends[0]..ends[0] + 4].try_into().unwrap()) as usize;
    source[ends[0] + 8..ends[0] + 12].copy_from_slice(&0_u32.to_le_bytes());
    let crc = crc32fast::hash(&source[ends[0] + 8..ends[0] + 8 + n]);
    source[ends[0] + 4..ends[0] + 8].copy_from_slice(&crc.to_le_bytes());
    let mut out = Vec::new();
    let report = recover_prefix(source.as_slice(), &mut out, 100, &|_| Ok(())).unwrap();
    assert_eq!(report.recovered_frames, 1);
    assert!(report.source_issue.unwrap().contains("non-monotonic"));
    let (mut malformed, offsets) = fixture();
    let start = offsets[0];
    let body = u32::from_le_bytes(malformed[start..start + 4].try_into().unwrap()) as usize;
    malformed[start + 8 + 20..start + 8 + 24].copy_from_slice(&u32::MAX.to_le_bytes());
    let crc = crc32fast::hash(&malformed[start + 8..start + 8 + body]);
    malformed[start + 4..start + 8].copy_from_slice(&crc.to_le_bytes());
    let report = recover_prefix(malformed.as_slice(), Vec::new(), 100, &|_| Ok(())).unwrap();
    assert_eq!(report.recovered_frames, 1);
    assert!(
        report
            .source_issue
            .unwrap()
            .contains("invalid frame lengths")
    );
    let (mut source, _) = fixture();
    let length = source.len();
    source[length - 8..].copy_from_slice(&99_u64.to_le_bytes());
    let mut out = Vec::new();
    let report = recover_prefix(source.as_slice(), &mut out, 100, &|_| Ok(())).unwrap();
    assert_eq!(report.recovered_frames, 3);
    assert!(report.source_issue.unwrap().contains("footer"));
    assert_eq!(recovered_frames(&out).len(), 3);
}
#[test]
fn frame_budget_is_explicit_and_bad_header_or_zero_frames_never_finish() {
    let (mut source, ends) = fixture();
    let mut out = Vec::new();
    let report = recover_prefix(source.as_slice(), &mut out, 1, &|_| Ok(())).unwrap();
    assert!(matches!(report.stop_reason, StopReason::FrameBudget));
    assert!(!report.source_complete);
    assert_eq!(report.source_bytes_read, ends[0] as u64);
    assert_eq!(recovered_frames(&out), vec![frame(0)]);
    source[16] ^= 1;
    assert!(recover_prefix(source.as_slice(), Vec::new(), 100, &|_| Ok(())).is_err());
    let (source, ends) = fixture();
    for end in 0..ends[0] {
        assert!(recover_prefix(&source[..end], Vec::new(), 100, &|_| Ok(())).is_err());
    }
    assert!(recover_prefix(source.as_slice(), Vec::new(), 0, &|_| Ok(())).is_err());
}
#[test]
fn callback_error_cannot_be_misreported_as_recoverable_corruption() {
    let (source, ends) = fixture();
    let mut out = Vec::new();
    let error = recover_prefix(source.as_slice(), &mut out, 100, &|offset| {
        if offset > ends[0] as u64 {
            Err(io::Error::new(io::ErrorKind::InvalidData, "cancelled"))
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);
    assert!(error.to_string().contains("caller"));
    let mut reader = Reader::new(out.as_slice()).unwrap();
    assert!(reader.read_frame().unwrap().is_some());
    assert!(reader.read_frame().is_err());
}
struct Fault {
    source: Cursor<Vec<u8>>,
    stop: u64,
    kind: io::ErrorKind,
}
impl Read for Fault {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.source.position() >= self.stop {
            return Err(io::Error::new(self.kind, "simulated I/O failure"));
        }
        let n = out.len().min((self.stop - self.source.position()) as usize);
        self.source.read(&mut out[..n])
    }
}
#[test]
fn io_failure_is_fatal_and_legacy_header_integrity_is_not_invented() {
    let (source, ends) = fixture();
    for kind in [
        io::ErrorKind::PermissionDenied,
        io::ErrorKind::InvalidData,
        io::ErrorKind::UnexpectedEof,
    ] {
        assert_eq!(
            recover_prefix(
                Fault {
                    source: Cursor::new(source.clone()),
                    stop: ends[0] as u64,
                    kind
                },
                Vec::new(),
                100,
                &|_| Ok(())
            )
            .unwrap_err()
            .kind(),
            kind
        );
    }
    let (v2, _) = fixture();
    let h = u32::from_le_bytes(v2[8..12].try_into().unwrap()) as usize;
    let p = serde_json::to_vec(&profile()).unwrap();
    let mut legacy = b"RAPTOR01".to_vec();
    legacy.extend_from_slice(&(p.len() as u32).to_le_bytes());
    legacy.extend(p);
    legacy.extend_from_slice(&v2[16 + h..]);
    let mut out = Vec::new();
    let report = recover_prefix(legacy.as_slice(), &mut out, 100, &|_| Ok(())).unwrap();
    assert_eq!(report.source_version, 1);
    assert!(!report.source_header_crc_verified && !report.source_context_available);
    assert!(report.source_complete);
    assert_eq!(recovered_frames(&out).len(), 3);
    assert_eq!(Reader::new(out.as_slice()).unwrap().version, 2);
}
