use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::{Reader, Writer};
use std::{
    cell::RefCell,
    io::{self, Cursor, Read, Write},
    rc::Rc,
};

#[derive(Default)]
struct State {
    bytes: Vec<u8>,
    calls: usize,
    fail_at: Option<usize>,
}
#[derive(Clone)]
struct Sink(Rc<RefCell<State>>);
impl Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut state = self.0.borrow_mut();
        state.calls += 1;
        let count = if let Some(limit) = state.fail_at {
            if state.bytes.len() >= limit {
                state.fail_at = None;
                return Err(io::Error::other("injected output failure"));
            }
            bytes.len().min(limit - state.bytes.len())
        } else {
            bytes.len()
        };
        state.bytes.extend_from_slice(&bytes[..count]);
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.borrow_mut().calls += 1;
        Ok(())
    }
}
struct Source {
    input: Cursor<Vec<u8>>,
    state: Rc<RefCell<State>>,
}
impl Read for Source {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let mut state = self.state.borrow_mut();
        state.calls += 1;
        let count = if let Some(limit) = state.fail_at {
            let position = self.input.position() as usize;
            if position >= limit {
                state.fail_at = None;
                return Err(io::Error::other("injected input failure"));
            }
            bytes.len().min(limit - position)
        } else {
            bytes.len()
        };
        self.input.read(&mut bytes[..count])
    }
}
fn profile() -> StreamProfile {
    StreamProfile {
        interface: 0,
        width: 2,
        height: 2,
        fourcc: *b"GREY",
        fps: 30,
    }
}
fn frame(index: u32) -> Frame {
    let mut metadata = vec![0; 16];
    metadata[10] = 6;
    metadata[11] = 4;
    metadata[12..16].copy_from_slice(&(100 + index).to_le_bytes());
    Frame {
        sequence: index,
        host_timestamp_us: 1000 + u64::from(index),
        flags: 0,
        metadata,
        data: vec![index as u8; 4],
    }
}
fn recording() -> Vec<u8> {
    let mut writer = Writer::new(Vec::new(), profile()).unwrap();
    writer.push(&frame(0)).unwrap();
    writer.push(&frame(1)).unwrap();
    writer.finish().unwrap()
}

#[test]
fn partial_output_failure_refuses_all_future_writes_and_footer_even_if_sink_recovers() {
    for offset in [0, 2, 6, 10, 40, 51] {
        let state = Rc::new(RefCell::new(State::default()));
        let mut writer = Writer::new(Sink(state.clone()), profile()).unwrap();
        let start = state.borrow().bytes.len();
        state.borrow_mut().fail_at = Some(start + offset);
        let error = writer.push(&frame(0)).unwrap_err();
        assert_eq!(error.to_string(), "injected output failure");
        let calls = state.borrow().calls;
        let bytes = state.borrow().bytes.clone();
        assert!(
            writer
                .push(&frame(1))
                .unwrap_err()
                .to_string()
                .contains("unusable")
        );
        assert!(
            writer
                .finish()
                .err()
                .unwrap()
                .to_string()
                .contains("unusable")
        );
        assert_eq!(state.borrow().calls, calls);
        assert_eq!(state.borrow().bytes, bytes);
    }
}

#[test]
fn rejected_unwritten_frame_can_be_corrected_without_losing_order_state() {
    let state = Rc::new(RefCell::new(State::default()));
    let mut writer = Writer::new(Sink(state.clone()), profile()).unwrap();
    writer.push(&frame(0)).unwrap();
    let before = state.borrow().bytes.clone();
    let calls = state.borrow().calls;
    assert!(writer.push(&frame(0)).is_err()); // Duplicate sequence/PTS.
    let mut invalid = frame(1);
    invalid.data.pop();
    assert!(writer.push(&invalid).is_err());
    assert_eq!(state.borrow().calls, calls);
    assert_eq!(state.borrow().bytes, before);
    writer.push(&frame(1)).unwrap();
    writer.finish().unwrap();
    let bytes = state.borrow().bytes.clone();
    let mut reader = Reader::new(bytes.as_slice()).unwrap();
    assert_eq!(reader.read_frame().unwrap().unwrap().sequence, 0);
    assert_eq!(reader.read_frame().unwrap().unwrap().sequence, 1);
    assert!(reader.read_frame().unwrap().is_none());
    assert!(reader.read_frame().unwrap().is_none());
}

#[test]
fn partial_input_failure_is_terminal_even_when_source_would_recover() {
    let bytes = recording();
    let header = 16 + u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    for offset in [0, 2, 6, 10, 40, 51] {
        let state = Rc::new(RefCell::new(State::default()));
        let input = Source {
            input: Cursor::new(bytes.clone()),
            state: state.clone(),
        };
        let mut reader = Reader::new(input).unwrap();
        state.borrow_mut().fail_at = Some(header + offset);
        assert_eq!(
            reader.read_frame().unwrap_err().to_string(),
            "injected input failure"
        );
        let calls = state.borrow().calls;
        assert!(
            reader
                .read_frame()
                .unwrap_err()
                .to_string()
                .contains("unusable")
        );
        assert_eq!(state.borrow().calls, calls);
    }
}

#[test]
fn crc_and_footer_failures_remain_errors_on_every_subsequent_read() {
    let original = recording();
    let header = 16 + u32::from_le_bytes(original[8..12].try_into().unwrap()) as usize;
    let mut crc = original.clone();
    crc[header + 8 + 24] ^= 1;
    let mut footer = original.clone();
    *footer.last_mut().unwrap() ^= 1;
    for bytes in [crc, footer, original[..original.len() - 1].to_vec()] {
        let mut reader = Reader::new(bytes.as_slice()).unwrap();
        let mut failed = false;
        for _ in 0..4 {
            if reader.read_frame().is_err() {
                failed = true;
                break;
            }
        }
        assert!(failed, "fixture must fail within its two frames and footer");
        for _ in 0..3 {
            assert!(
                reader
                    .read_frame()
                    .unwrap_err()
                    .to_string()
                    .contains("unusable")
            );
        }
    }
}
