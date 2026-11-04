use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
struct Device {
    actual: [i32; 2],
    events: Vec<String>,
    stream_failures: usize,
    gain_failures: [usize; 2],
}
type Shared = Rc<RefCell<Device>>;
struct Stream {
    device: Shared,
    restored: bool,
    finished: bool,
}
impl StreamRestore for Stream {
    fn restore(&mut self) -> Result<()> {
        if self.restored {
            return Ok(());
        }
        let mut d = self.device.borrow_mut();
        d.events.push("formats/rates".into());
        d.actual = [1000; 2];
        if d.stream_failures > 0 {
            d.stream_failures -= 1;
            Err("injected stream restoration failure".into())
        } else {
            self.restored = true;
            Ok(())
        }
    }
    fn finish(mut self) -> Result<()> {
        let result = self.restore();
        self.finished = true;
        result
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.restore();
        }
        self.device
            .borrow_mut()
            .events
            .push("stream dropped".into());
    }
}
struct Gain {
    device: Shared,
    restored: [bool; 2],
}
impl Gain {
    fn restore(&mut self) -> Result<()> {
        let mut errors = Vec::new();
        let mut d = self.device.borrow_mut();
        for i in (0..2).rev() {
            if self.restored[i] {
                continue;
            }
            d.events.push(format!("gain {i}"));
            if d.gain_failures[i] > 0 {
                d.gain_failures[i] -= 1;
                errors.push(format!("injected gain {i} failure"));
            } else {
                d.actual[i] = [8000, 9000][i];
                self.restored[i] = true;
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}
impl GainRestore for Gain {
    fn restore_after_settings(&mut self) -> Result<()> {
        self.restored = [false; 2];
        self.restore()
    }
}
impl Drop for Gain {
    fn drop(&mut self) {
        let _ = self.restore();
        self.device.borrow_mut().events.push("gain dropped".into());
    }
}
fn fixture(failures: usize) -> (Shared, Settings<Stream, Gain>) {
    let device = Rc::new(RefCell::new(Device {
        actual: [15000; 2],
        stream_failures: failures,
        ..Device::default()
    }));
    let settings = Settings::new(
        Stream {
            device: device.clone(),
            restored: false,
            finished: false,
        },
        Some(Gain {
            device: device.clone(),
            restored: [false; 2],
        }),
    );
    (device, settings)
}
fn assert_final_order(device: &Shared, formats: usize) {
    let d = device.borrow();
    assert_eq!(d.actual, [8000, 9000]);
    assert_eq!(
        d.events.iter().filter(|v| *v == "formats/rates").count(),
        formats
    );
    let stream = d.events.iter().position(|v| v == "stream dropped").unwrap();
    assert_eq!(d.events[stream + 1], "gain 1");
    assert_eq!(d.events[stream + 2], "gain 0");
    assert_eq!(d.events.last().unwrap(), "gain dropped");
}

#[test]
fn failed_callback_then_successful_stream_retry_reapplies_nondefault_gains() {
    let (device, mut settings) = fixture(1);
    let callback = settings.restore();
    assert!(
        callback
            .as_ref()
            .unwrap_err()
            .contains("stream restoration")
    );
    assert_eq!(device.borrow().actual, [8000, 9000]);
    let outcome = combine(callback, settings.finish());
    assert!(outcome.unwrap_err().contains("stream restoration"));
    assert_final_order(&device, 2);
}

#[test]
fn repeated_stream_failures_finish_before_final_gain_and_remain_errors() {
    let (device, mut settings) = fixture(20);
    assert!(settings.restore().is_err());
    assert!(settings.finish().is_err());
    assert_final_order(&device, 2);
}

#[test]
fn drop_and_unwind_restore_stream_before_gain_after_failed_callback() {
    for unwind in [false, true] {
        let (device, mut settings) = fixture(20);
        assert!(settings.restore().is_err());
        if unwind {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let _settings = settings;
                panic!("injected application unwind");
            }));
            assert!(result.is_err());
        } else {
            drop(settings);
        }
        assert_final_order(&device, 2);
    }
}

#[test]
fn either_camera_gain_failure_does_not_skip_peer_or_hide_stream_error() {
    for camera in 0..2 {
        let (device, settings) = fixture(1);
        device.borrow_mut().gain_failures[camera] = 1;
        let error = settings.finish().unwrap_err();
        assert!(error.contains("stream restoration"));
        assert!(error.contains(&format!("gain {camera} failure")));
        // Gain's Drop retries only its failed camera, after stream consumption.
        assert_final_order(&device, 1);
    }
}

#[test]
fn successful_callback_does_not_repeat_formats_and_absent_gain_is_supported() {
    let (device, mut settings) = fixture(0);
    settings.restore().unwrap();
    let calls = device.borrow().events.clone();
    settings.finish().unwrap();
    let mut expected = calls;
    expected.extend(["stream dropped".into(), "gain dropped".into()]);
    assert_eq!(device.borrow().events, expected);
    assert_eq!(device.borrow().actual, [8000, 9000]);
    let stream = Stream {
        device: device.clone(),
        restored: false,
        finished: false,
    };
    Settings::<_, Gain>::new(stream, None).finish().unwrap();
    assert_eq!(device.borrow().actual, [1000; 2]);
}
