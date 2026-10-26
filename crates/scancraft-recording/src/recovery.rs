//! Recover only the validated contiguous prefix; never resynchronize past a bad frame.
use super::{CaptureContext, Reader, StreamProfile, Writer, invalid};
use std::io::{self, Read, Write};
pub const MAX_RECOVERY_FRAMES: u64 = 1_000_000;
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    SourceComplete,
    InvalidOrTruncatedSuffix,
    FrameBudget,
}
#[derive(Debug, serde::Serialize)]
pub struct Report {
    pub source_version: u8,
    pub source_header_crc_verified: bool,
    pub source_context_available: bool,
    pub profile: StreamProfile,
    pub recovered_frames: u64,
    pub validated_prefix_end_bytes: u64,
    pub source_bytes_read: u64,
    pub source_bytes_read_crc32: u32,
    pub source_complete: bool,
    pub stop_reason: StopReason,
    pub source_issue_kind: Option<String>,
    pub source_issue: Option<String>,
}
struct Counted<'a, R, C> {
    input: R,
    bytes: u64,
    crc: crc32fast::Hasher,
    check: &'a C,
    source_io_error: bool,
}
fn controlled(error: io::Error) -> io::Error {
    io::Error::other(format!("recovery stopped by caller: {error}"))
}
impl<R: Read, C: Fn(u64) -> io::Result<()>> Read for Counted<'_, R, C> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        (self.check)(self.bytes).map_err(controlled)?;
        self.source_io_error = false;
        let n = match self.input.read(out) {
            Ok(n) => n,
            Err(error) => {
                self.source_io_error = true;
                return Err(error);
            }
        };
        self.bytes = self
            .bytes
            .checked_add(n as u64)
            .ok_or_else(|| invalid("recovery byte count overflow"))?;
        self.crc.update(&out[..n]);
        (self.check)(self.bytes).map_err(controlled)?;
        Ok(n)
    }
}
/// Rewrites validated frames into v2 with a new completion footer. Caller owns
/// publication: errors may leave output incomplete and must never publish it.
/// v1 header CRC/context were unavailable and remain explicitly unverified.
pub fn recover_prefix<R: Read, W: Write, C: Fn(u64) -> io::Result<()>>(
    input: R,
    output: W,
    maximum_frames: u64,
    check: &C,
) -> io::Result<Report> {
    if !(1..=MAX_RECOVERY_FRAMES).contains(&maximum_frames) {
        return Err(invalid("recovery frame budget must be 1..1000000"));
    }
    check(0).map_err(controlled)?;
    let counted = Counted {
        input,
        bytes: 0,
        crc: crc32fast::Hasher::new(),
        check,
        source_io_error: false,
    };
    let mut reader = Reader::new(counted)?;
    let profile = reader.profile.clone();
    let version = reader.version;
    let context_present = reader.context.is_some();
    let mut writer = Writer::with_context(
        output,
        profile.clone(),
        reader
            .context
            .clone()
            .unwrap_or_else(CaptureContext::default),
    )?;
    let mut recovered = 0;
    let mut prefix_end = reader.input.bytes;
    let (reason, issue) = loop {
        check(reader.input.bytes).map_err(controlled)?;
        if recovered == maximum_frames {
            break (StopReason::FrameBudget, None);
        }
        match reader.read_frame() {
            Ok(Some(frame)) => {
                writer.push(&frame)?;
                recovered += 1;
                prefix_end = reader.input.bytes;
            }
            Ok(None) => break (StopReason::SourceComplete, None),
            Err(error) if reader.input.source_io_error => return Err(error),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::UnexpectedEof | io::ErrorKind::InvalidData
                ) =>
            {
                break (StopReason::InvalidOrTruncatedSuffix, Some(error));
            }
            Err(error) => return Err(error),
        }
    };
    if recovered == 0 {
        return Err(invalid("no validated frames available for recovery"));
    }
    check(reader.input.bytes).map_err(controlled)?;
    writer.finish()?;
    check(reader.input.bytes).map_err(controlled)?;
    Ok(Report {
        source_version: version,
        source_header_crc_verified: version == 2,
        source_context_available: context_present,
        profile,
        recovered_frames: recovered,
        validated_prefix_end_bytes: prefix_end,
        source_bytes_read: reader.input.bytes,
        source_bytes_read_crc32: reader.input.crc.finalize(),
        source_complete: matches!(reason, StopReason::SourceComplete),
        stop_reason: reason,
        source_issue_kind: issue.as_ref().map(|e| format!("{:?}", e.kind())),
        source_issue: issue.map(|e| e.to_string()),
    })
}
