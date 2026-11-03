use super::*;
use crate::{
    Digest,
    laser_stream::bundle::io::{CappedFile, pair_record},
};
use scancraft_geometry::work::WorkControl;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
struct Encoded {
    bytes: Vec<u8>,
    maximum: usize,
}
impl Write for Encoded {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.bytes.len().saturating_add(b.len()) > self.maximum {
            return Err(invalid("pair record exceeds configured byte limit"));
        }
        self.bytes.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
struct Active {
    file: CappedFile,
    pairs: u64,
}
pub(crate) struct Journal {
    root: PathBuf,
    directory: File,
    limits: Limits,
    index: Option<CappedFile>,
    active: Option<Active>,
    artifacts: Vec<Digest>,
    pairs: u64,
    bytes: u64,
    failed: bool,
}
pub(crate) struct Finished {
    pub artifacts: Vec<Digest>,
    pub pairs: u64,
    pub bytes: u64,
    pub segments: usize,
}
impl Journal {
    pub(crate) fn new(root: &Path, limits: Limits) -> io::Result<Self> {
        limits.validate()?;
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(root)?;
        let mut index = CappedFile::new(root, INDEX_NAME, INDEX_LIMIT)?;
        pair_record(
            &mut index,
            &Line::Header {
                schema_version: 1,
                limits,
            },
        )?;
        index.sync()?;
        directory.sync_all()?;
        Ok(Self {
            root: root.into(),
            directory,
            limits,
            index: Some(index),
            active: None,
            artifacts: Vec::new(),
            pairs: 0,
            bytes: 0,
            failed: false,
        })
    }
    fn seal(&mut self) -> io::Result<()> {
        let Some(active) = self.active.take() else {
            return Ok(());
        };
        let digest = active.file.finish()?;
        self.directory.sync_all()?;
        let number = self.artifacts.len();
        let first_pair = self.pairs - active.pairs;
        let index = self
            .index
            .as_mut()
            .ok_or_else(|| invalid("missing pair index writer"))?;
        pair_record(
            index,
            &Line::Segment {
                number,
                first_pair,
                pairs: active.pairs,
                bytes: digest.bytes,
                crc32: digest.crc32,
            },
        )?;
        index.sync()?;
        self.artifacts.push(digest);
        Ok(())
    }
    pub(crate) fn push(&mut self, value: &impl Serialize, work: &WorkControl) -> io::Result<()> {
        if self.failed {
            return Err(invalid("pair journal unusable after an error"));
        }
        let result = self.append(value, work);
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn append(&mut self, value: &impl Serialize, work: &WorkControl) -> io::Result<()> {
        work.check().map_err(|e| invalid(&e))?;
        let mut encoded = Encoded {
            bytes: Vec::new(),
            maximum: self.limits.maximum_record_bytes as usize,
        };
        // Coalesce small serializer fragments with the existing fixed 64 KiB
        // helper. Check cancellation before every resulting bounded-sink write.
        // Encoding must finish before touching the required file/index prefix.
        struct Controlled<'a> {
            encoded: &'a mut Encoded,
            work: &'a WorkControl,
        }
        impl Write for Controlled<'_> {
            fn write(&mut self, b: &[u8]) -> io::Result<usize> {
                self.work.check().map_err(|e| invalid(&e))?;
                self.encoded.write(b)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let mut out = Controlled {
            encoded: &mut encoded,
            work,
        };
        pair_record(&mut out, value)?;
        work.check().map_err(|e| invalid(&e))?;
        let n = encoded.bytes.len() as u64;
        let total = self
            .bytes
            .checked_add(n)
            .filter(|&v| v <= self.limits.maximum_total_bytes)
            .ok_or_else(|| invalid("pair journal exceeds total byte quota"))?;
        if self
            .active
            .as_ref()
            .is_some_and(|a| a.file.snapshot().bytes + n > self.limits.segment_bytes)
        {
            self.seal()?;
        }
        if self.active.is_none() {
            if self.artifacts.len() == MAX_SEGMENTS {
                return Err(invalid("pair journal segment limit exceeded"));
            }
            self.active = Some(Active {
                file: CappedFile::new(
                    &self.root,
                    &role(self.artifacts.len()),
                    self.limits.segment_bytes,
                )?,
                pairs: 0,
            });
        }
        let active = self.active.as_mut().unwrap();
        active.file.write_all(&encoded.bytes)?;
        active.file.flush()?;
        active.pairs += 1;
        self.pairs = self
            .pairs
            .checked_add(1)
            .ok_or_else(|| invalid("pair count overflow"))?;
        self.bytes = total;
        Ok(())
    }
    pub(crate) fn finish(mut self) -> io::Result<Finished> {
        if self.failed {
            return Err(invalid(
                "cannot complete failed pair journal; inspect retained prefix",
            ));
        }
        self.seal()?;
        let mut index = self
            .index
            .take()
            .ok_or_else(|| invalid("missing pair index writer"))?;
        let prefix = index.snapshot();
        let footer = Line::Complete {
            segments: self.artifacts.len(),
            pairs: self.pairs,
            bytes: self.bytes,
            prefix_bytes: prefix.bytes,
            prefix_crc32: prefix.crc32,
        };
        if let Err(e) = pair_record(&mut index, &footer) {
            index.discard();
            return Err(e);
        }
        let digest = index.finish()?;
        let segments = self.artifacts.len();
        self.artifacts.push(digest);
        self.directory.sync_all()?;
        Ok(Finished {
            artifacts: std::mem::take(&mut self.artifacts),
            pairs: self.pairs,
            bytes: self.bytes,
            segments,
        })
    }
}
impl Drop for Journal {
    fn drop(&mut self) {
        if let Some(active) = self.active.take() {
            active.file.discard();
        }
        if let Some(index) = self.index.take() {
            index.discard();
        }
    }
}
use std::os::unix::fs::OpenOptionsExt;

#[cfg(test)]
mod failure_tests {
    use super::*;
    use crate::laser_stream::{
        bundle::io::Fault,
        pair_journal::{
            Mode,
            tests::{Temp, record},
            visit,
        },
    };
    #[test]
    fn failed_rollover_write_or_sync_retains_prefix_without_completion_footer() {
        for case in 0..4 {
            let root = Temp::new();
            let work = WorkControl::default();
            let limits = Limits {
                segment_bytes: 4096,
                maximum_record_bytes: 2048,
                maximum_total_bytes: 100000,
            };
            let mut out = Journal::new(&root.0, limits).unwrap();
            out.push(&record(0, 1400), &work).unwrap();
            if case == 0 {
                out.active.as_mut().unwrap().file.inject(Fault::Write);
                assert!(out.push(&record(1, 1400), &work).is_err());
            } else {
                out.push(&record(1, 1400), &work).unwrap();
                match case {
                    1 => out.active.as_mut().unwrap().file.inject(Fault::Sync),
                    2 => out.index.as_mut().unwrap().inject(Fault::Write),
                    _ => out.index.as_mut().unwrap().inject(Fault::Sync),
                }
                assert!(out.push(&record(2, 1400), &work).is_err());
            }
            assert!(out.finish().is_err());
            assert!(visit(&root.0, Mode::Complete, &work, |_, _, _| Ok(())).is_err());
            let prefix = visit(&root.0, Mode::Recovery, &work, |_, _, _| Ok(())).unwrap();
            assert!(!prefix.journal_complete);
            assert_eq!(prefix.pairs, if case == 0 { 1 } else { 2 });
        }
    }

    #[test]
    fn final_index_sync_error_can_leave_visible_footer_but_never_returns_success() {
        let root = Temp::new();
        let work = WorkControl::default();
        let limits = Limits {
            segment_bytes: 4096,
            maximum_record_bytes: 2048,
            maximum_total_bytes: 100000,
        };
        let mut out = Journal::new(&root.0, limits).unwrap();
        out.push(&record(0, 0), &work).unwrap();
        out.seal().unwrap();
        // The footer flush succeeds; only the subsequent fsync fails.
        out.index.as_mut().unwrap().inject(Fault::Sync);
        assert!(out.finish().is_err());
        for mode in [Mode::Complete, Mode::Recovery] {
            let observed = visit(&root.0, mode, &work, |_, _, _| Ok(())).unwrap();
            assert!(observed.journal_complete);
            assert_eq!(observed.pairs, 1);
            assert!(!observed.source_restoration_verified);
            assert!(!observed.physical_accuracy_qualified);
        }
        assert!(!root.0.join("manifest.json").exists());
        assert!(!root.0.join("report.json").exists());
    }
}
