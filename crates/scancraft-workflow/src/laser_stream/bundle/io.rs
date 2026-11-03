use crate::{Digest, scan_project_load::Guard};
use scancraft_core::StreamProfile;
use scancraft_geometry::work::WorkControl;
use scancraft_recording::{Reader, Writer};
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader, BufWriter, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
    sync::{Arc, Mutex},
};
pub(super) type RawDigests = Arc<Mutex<[Option<Digest>; 2]>>;
#[cfg(test)]
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Fault {
    Write,
    Sync,
}
pub(crate) struct CappedFile {
    #[cfg(test)]
    fault: Option<Fault>,
    out: BufWriter<File>,
    role: String,
    maximum: u64,
    bytes: u64,
    hash: crc32fast::Hasher,
}
impl CappedFile {
    pub(crate) fn new(directory: &Path, role: &str, maximum: u64) -> io::Result<Self> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(directory.join(role))?;
        Ok(Self {
            out: BufWriter::new(file),
            #[cfg(test)]
            fault: None,
            role: role.into(),
            maximum,
            bytes: 0,
            hash: crc32fast::Hasher::new(),
        })
    }
    #[cfg(test)]
    pub(crate) fn inject(&mut self, fault: Fault) {
        self.fault = Some(fault);
    }
    pub(crate) fn sync(&mut self) -> io::Result<()> {
        self.out.flush()?;
        #[cfg(test)]
        if self.fault == Some(Fault::Sync) {
            self.fault = None;
            return Err(io::Error::other("injected sync failure"));
        }
        self.out.get_ref().sync_all()
    }
    pub(crate) fn snapshot(&self) -> Digest {
        Digest {
            role: self.role.clone(),
            bytes: self.bytes,
            crc32: self.hash.clone().finalize(),
        }
    }
    pub(crate) fn discard(self) {
        let (_file, _pending) = self.out.into_parts();
    }
    pub(crate) fn finish(mut self) -> io::Result<Digest> {
        if let Err(e) = self.sync() {
            self.discard();
            return Err(e);
        }
        Ok(self.snapshot())
    }
}
impl Write for CappedFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        #[cfg(test)]
        if self.fault == Some(Fault::Write) {
            self.fault = None;
            return Err(io::Error::other("injected write failure"));
        }
        if self
            .bytes
            .checked_add(bytes.len() as u64)
            .is_none_or(|n| n > self.maximum)
        {
            return Err(io::Error::other(
                "laser capture artifact exceeds configured byte budget",
            ));
        }
        let n = self.out.write(bytes)?;
        self.bytes += n as u64;
        self.hash.update(&bytes[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.out.flush()
    }
}
/// Batch small serializer writes before CappedFile's checksum/write path. The
/// fixed buffer is discarded explicitly on error: Drop must not retry rejected
/// bytes. A successful complete record is flushed before optional preview.
pub(crate) fn pair_record(out: &mut impl Write, value: &impl Serialize) -> io::Result<()> {
    let mut buffered = BufWriter::with_capacity(64 * 1024, out);
    let result = (|| {
        serde_json::to_writer(&mut buffered, value)?;
        buffered.write_all(b"\n")?;
        buffered.flush()
    })();
    let (_out, _pending) = buffered.into_parts();
    result
}
pub(super) struct RawJournal {
    pub writer: Writer<CappedFile>,
    pub digests: RawDigests,
    pub camera: usize,
}
impl super::super::Journal for RawJournal {
    fn push(&mut self, frame: &scancraft_core::Frame) -> Result<(), String> {
        self.writer.push(frame).map_err(|e| e.to_string())
    }
    fn finish(self) -> Result<(), String> {
        let digest = self
            .writer
            .finish()
            .and_then(CappedFile::finish)
            .map_err(|e| e.to_string())?;
        let mut slots = self
            .digests
            .lock()
            .map_err(|_| "raw artifact digest state poisoned")?;
        if slots[self.camera].is_some() {
            return Err("raw artifact finalized twice".into());
        }
        slots[self.camera] = Some(digest);
        Ok(())
    }
}
pub(super) fn bytes(directory: &Path, role: &str, bytes: &[u8]) -> io::Result<Digest> {
    let mut out = CappedFile::new(directory, role, bytes.len() as u64)?;
    out.write_all(bytes)?;
    out.finish()
}
pub(super) fn json(
    directory: &Path,
    role: &str,
    value: &impl Serialize,
    maximum: u64,
) -> io::Result<Digest> {
    let mut out = CappedFile::new(directory, role, maximum)?;
    serde_json::to_writer_pretty(&mut out, value)?;
    out.finish()
}
struct Hashed<R> {
    input: R,
    bytes: u64,
    maximum: u64,
    hash: crc32fast::Hasher,
}
impl<R: Read> Read for Hashed<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let n = self.input.read(buffer)?;
        self.bytes = self
            .bytes
            .checked_add(n as u64)
            .filter(|v| *v <= self.maximum)
            .ok_or_else(|| io::Error::other("readback exceeds written byte count"))?;
        self.hash.update(&buffer[..n]);
        Ok(n)
    }
}
fn equal(actual: (u64, u32), expected: &Digest) -> Result<(), String> {
    if actual != (expected.bytes, expected.crc32) {
        return Err(format!(
            "written artifact {} differs from readback",
            expected.role
        ));
    }
    Ok(())
}
pub(super) fn verify_raw(
    directory: &Path,
    expected: &Digest,
    profile: &StreamProfile,
    count: u64,
    guard: &mut Guard,
    work: &WorkControl,
) -> Result<(), String> {
    let input = guard
        .open(&directory.join(&expected.role), false, expected.bytes)
        .map_err(|e| e.to_string())?;
    let mut hashed = Hashed {
        input,
        bytes: 0,
        maximum: expected.bytes,
        hash: crc32fast::Hasher::new(),
    };
    {
        let mut reader = Reader::new(BufReader::new(&mut hashed)).map_err(|e| e.to_string())?;
        if reader.version != 2 || &reader.profile != profile {
            return Err(
                "laser raw recording header differs from configured profile/version".into(),
            );
        }
        let mut frames = 0u64;
        while reader.read_frame().map_err(|e| e.to_string())?.is_some() {
            work.check()?;
            frames += 1;
            if frames > count {
                return Err("laser raw recording exceeds captured frame count".into());
            }
        }
        if frames != count {
            return Err("laser raw replay count differs from capture count".into());
        }
    }
    equal((hashed.bytes, hashed.hash.finalize()), expected)
}
pub(super) fn verify(
    directory: &Path,
    expected: &Digest,
    guard: &mut Guard,
    work: &WorkControl,
) -> Result<(), String> {
    let (_, bytes, crc) = guard
        .read(&directory.join(&expected.role), expected.bytes, false, work)
        .map_err(|e| e.to_string())?;
    equal((bytes, crc), expected)
}
