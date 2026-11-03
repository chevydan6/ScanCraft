use super::tests::{Temp, record};
use super::*;
use scancraft_geometry::work::WorkControl;
use std::{cell::Cell, fs};

#[test]
fn buffered_encoding_preserves_unicode_newline_and_exact_record_limit() {
    let mut value = record(0, 0);
    value["result"] = serde_json::json!({"Err":"λ\n\"".repeat(16 * 1024)});
    let mut expected = serde_json::to_vec(&value).unwrap();
    expected.push(b'\n');
    assert!(expected.len() > 65536);
    let n = expected.len() as u64;
    for short in [false, true] {
        let root = Temp::new();
        let work = WorkControl::default();
        let mut out = Journal::new(
            &root.0,
            Limits {
                segment_bytes: 2 * n,
                maximum_record_bytes: n - u64::from(short),
                maximum_total_bytes: 4 * n,
            },
        )
        .unwrap();
        let header = fs::read(root.0.join(INDEX_NAME)).unwrap();
        if short {
            assert!(
                out.push(&value, &work)
                    .unwrap_err()
                    .to_string()
                    .contains("record exceeds")
            );
            assert!(out.push(&record(0, 0), &work).is_err());
            assert!(out.finish().is_err());
            assert_eq!(fs::read(root.0.join(INDEX_NAME)).unwrap(), header);
            assert_eq!(fs::read_dir(&root.0).unwrap().count(), 1);
        } else {
            out.push(&value, &work).unwrap();
            let done = out.finish().unwrap();
            assert_eq!(done.bytes, n);
            assert_eq!((done.pairs, done.segments), (1, 1));
            assert_eq!(fs::read(root.0.join(role(0))).unwrap(), expected);
            let checked = visit(&root.0, Mode::Complete, &work, |_, bytes, _| {
                assert_eq!(bytes, expected);
                Ok(())
            })
            .unwrap();
            assert!(checked.journal_complete);
        }
    }
}

#[test]
fn buffered_serializer_error_or_mid_record_cancel_preserves_required_prefix() {
    struct Interrupted<'a> {
        work: &'a WorkControl,
        cancel: bool,
        produced: Cell<usize>,
    }
    impl Serialize for Interrupted<'_> {
        fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            use serde::ser::SerializeSeq;
            let mut seq = s.serialize_seq(None)?;
            for i in 0..65536 {
                if self.cancel && i == 16384 {
                    self.work.cancel();
                }
                self.produced.set(i + 1);
                seq.serialize_element(&(i % 10))?;
            }
            Err(serde::ser::Error::custom("injected serializer failure"))
        }
    }
    for cancel in [false, true] {
        let root = Temp::new();
        let work = WorkControl::default();
        let mut out = Journal::new(
            &root.0,
            Limits {
                segment_bytes: 2 * 1024 * 1024,
                maximum_record_bytes: 1024 * 1024,
                maximum_total_bytes: 4 * 1024 * 1024,
            },
        )
        .unwrap();
        out.push(&record(0, 0), &work).unwrap();
        let segment = fs::read(root.0.join(role(0))).unwrap();
        let index = fs::read(root.0.join(INDEX_NAME)).unwrap();
        let value = Interrupted {
            work: &work,
            cancel,
            produced: Cell::new(0),
        };
        let error = out.push(&value, &work).unwrap_err().to_string();
        if cancel {
            assert!(error.contains("cancelled"));
            assert!(value.produced.get() < 65536);
        } else {
            assert!(error.contains("injected serializer failure"));
            assert_eq!(value.produced.get(), 65536);
        }
        assert!(out.finish().is_err());
        assert_eq!(fs::read(root.0.join(role(0))).unwrap(), segment);
        assert_eq!(fs::read(root.0.join(INDEX_NAME)).unwrap(), index);
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 2);
        let checked = visit(
            &root.0,
            Mode::Recovery,
            &WorkControl::default(),
            |_, _, _| Ok(()),
        )
        .unwrap();
        assert_eq!(checked.pairs, 1);
        assert_eq!(checked.trailing_incomplete_bytes, 0);
        assert!(!checked.journal_complete);
    }
}
