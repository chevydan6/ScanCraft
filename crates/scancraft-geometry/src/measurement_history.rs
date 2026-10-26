//! Compact inverse changes keep removed source evidence without cloud copies.
use crate::measurement_snapshot::Snapshot;
use std::collections::VecDeque;
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Change {
    Insert { index: usize, snapshot: Snapshot },
    Remove { index: usize, label: String },
    Rename { index: usize, label: String },
}
fn valid_label(label: &str) -> bool {
    !label.trim().is_empty() && label.len() <= 128 && !label.chars().any(char::is_control)
}
fn apply_labels(labels: &mut Vec<String>, change: &Change) -> Result<(), String> {
    match change {
        Change::Insert { index, snapshot } => {
            snapshot.validate()?;
            if *index > labels.len() || labels.len() >= 16 || labels.contains(&snapshot.label) {
                return Err("invalid measurement insertion history".into());
            }
            labels.insert(*index, snapshot.label.clone());
        }
        Change::Remove { index, label } => {
            if labels.get(*index) != Some(label) {
                return Err("measurement removal history does not match label/index".into());
            }
            labels.remove(*index);
        }
        Change::Rename { index, label } => {
            if !valid_label(label) || *index >= labels.len() || labels.contains(label) {
                return Err("invalid or duplicate measurement rename".into());
            }
            labels[*index] = label.clone();
        }
    }
    Ok(())
}
pub(crate) fn apply(records: &mut Vec<Snapshot>, change: Change) -> Result<Change, String> {
    let mut labels = records.iter().map(|s| s.label.clone()).collect();
    apply_labels(&mut labels, &change)?;
    Ok(match change {
        Change::Insert { index, snapshot } => {
            let label = snapshot.label.clone();
            records.insert(index, snapshot);
            Change::Remove { index, label }
        }
        Change::Remove { index, .. } => {
            let snapshot = records.remove(index);
            Change::Insert { index, snapshot }
        }
        Change::Rename { index, label } => {
            let old = std::mem::replace(&mut records[index].label, label);
            Change::Rename { index, label: old }
        }
    })
}
pub(crate) fn validate(
    records: &[Snapshot],
    undo: &VecDeque<Change>,
    redo: &[Change],
) -> Result<(), String> {
    if undo.len() + redo.len() > 16 {
        return Err("measurement history exceeds 16 changes".into());
    }
    let labels: Vec<_> = records.iter().map(|s| s.label.clone()).collect();
    let mut previous = labels.clone();
    for change in undo.iter().rev() {
        apply_labels(&mut previous, change)?;
    }
    let mut following = labels;
    for change in redo.iter().rev() {
        apply_labels(&mut following, change)?;
    }
    Ok(())
}
