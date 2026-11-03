//! Keep full IR observations on disk; extended sessions retain only compact lineage and latest tracking.
use super::{Pair, Processor, Result, json_file};
use crate::laser_stream::pair_journal::Journal;
use scancraft_geometry::{sequence_references::Frame, work::WorkControl};
use serde_json::{Value, json};
use std::path::Path;
pub(super) struct PairLog {
    journal: Option<Journal>,
    summaries: Vec<Value>,
    frames: Vec<Frame>,
    last_tracking: Value,
}
pub(super) struct Closed {
    pub summaries: Vec<Value>,
    pub frames: Vec<Frame>,
    pub names: Vec<String>,
    pub details: Option<Value>,
    pub last_tracking: Value,
}
impl PairLog {
    pub fn new(processor: &Processor, root: &Path) -> Result<Self> {
        Ok(Self {
            journal: processor
                .config()
                .storage
                .as_ref()
                .map(|s| Journal::new(root, s.pair_journal))
                .transpose()
                .map_err(|e| e.to_string())?,
            summaries: Vec::new(),
            frames: Vec::new(),
            last_tracking: Value::Null,
        })
    }
    pub fn len(&self) -> usize {
        self.frames.len()
    }
    pub fn push(
        &mut self,
        root: &Path,
        pair: &Pair,
        result: &Value,
        work: &WorkControl,
    ) -> Result<()> {
        let frame = Frame {
            id: result["pair_id"].as_u64().ok_or("missing pair id")?,
            left_index: pair.left.index,
            right_index: pair.right.index,
            integrated: result["tracking"]["integrated"]
                .as_bool()
                .ok_or("missing tracking decision")?,
        };
        if frame.id != self.frames.len() as u64 {
            return Err("pair log ordinal differs".into());
        }
        if let Some(journal) = &mut self.journal {
            journal
                .push(&super::super::pair_journal::record(pair, result), work)
                .map_err(|e| e.to_string())?;
        } else {
            let name = format!("pair-{:04}.json", self.frames.len());
            json_file(&root.join(&name), result)?;
            self.summaries.push(json!({"pair_id":result["pair_id"],"left_index":result["left_index"],"right_index":result["right_index"],
                "raw_pts":result["raw_pts"],"tracking":result["tracking"],"report":name}));
        }
        self.frames.push(frame);
        self.last_tracking = result["tracking"].clone();
        Ok(())
    }
    pub fn finish(self, root: &Path, work: &WorkControl) -> Result<Closed> {
        let (names, details) = if let Some(journal) = self.journal {
            let done = journal.finish().map_err(|e| e.to_string())?;
            let checked = super::super::pair_journal::visit(
                root,
                super::super::pair_journal::Mode::Complete,
                work,
                |_, _, _| Ok(()),
            )?;
            if checked.pairs != self.frames.len() as u64 || done.pairs != checked.pairs {
                return Err("live journal count differs from processed pairs".into());
            }
            (
                done.artifacts.into_iter().map(|a| a.role).collect(),
                Some(json!({
                    "format":"segmented_ndjson_v1", "index":"pairs.index.ndjson",
                    "segments":done.segments,"pairs":done.pairs,"bytes":done.bytes,
                })),
            )
        } else {
            (
                (0..self.frames.len())
                    .map(|i| format!("pair-{i:04}.json"))
                    .collect(),
                None,
            )
        };
        Ok(Closed {
            summaries: self.summaries,
            frames: self.frames,
            names,
            details,
            last_tracking: self.last_tracking,
        })
    }
}
