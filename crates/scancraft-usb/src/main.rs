//! Experimental pure-Rust direct USB acquisition. Never changes firmware or calibration.
use nusb::{
    Device, Interface, MaybeFuture,
    transfer::{Bulk, ControlIn, ControlOut, ControlType, In, Recipient},
};
use scancraft_device::SessionLock;
use std::{
    io::{self, Write},
    path::PathBuf,
    time::{Duration, Instant},
};
const TIMEOUT: Duration = Duration::from_secs(2);
const FRAME_BYTES: usize = 1920 * 1200;
fn err(s: impl Into<String>) -> io::Error {
    io::Error::other(s.into())
}
fn get(device: &Device, value: u16) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    eprintln!("UVC GET_CUR value={value:#x} interface=3 length=34");
    let bytes = device
        .control_in(
            ControlIn {
                control_type: ControlType::Class,
                recipient: Recipient::Interface,
                request: 0x81,
                value,
                index: 3,
                length: 34,
            },
            TIMEOUT,
        )
        .wait()?;
    if bytes.len() != 34 {
        return Err(err("unexpected UVC control length").into());
    }
    Ok(bytes)
}
fn set(device: &Device, value: u16, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    device
        .control_out(
            ControlOut {
                control_type: ControlType::Class,
                recipient: Recipient::Interface,
                request: 1,
                value,
                index: 3,
                data,
            },
            TIMEOUT,
        )
        .wait()?;
    Ok(())
}
struct Session {
    device: Device,
    control: Option<Interface>,
    stream: Option<Interface>,
    probe: Option<Vec<u8>>,
    commit: Option<Vec<u8>>,
    changed: bool,
    closed: bool,
}
impl Session {
    fn open(device: Device) -> Result<Self, Box<dyn std::error::Error>> {
        let mut s = Self {
            device,
            control: None,
            stream: None,
            probe: None,
            commit: None,
            changed: false,
            closed: false,
        };
        s.control = Some(s.device.detach_and_claim_interface(2).wait()?);
        s.stream = Some(s.device.claim_interface(3).wait()?);
        s.probe = Some(get(&s.device, 0x100)?);
        s.commit = Some(get(&s.device, 0x200)?);
        Ok(s)
    }
    fn configure(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut probe = self.probe.clone().ok_or("missing original probe")?;
        probe[2] = 2;
        probe[3] = 5;
        probe[4..8].copy_from_slice(&200_000u32.to_le_bytes());
        self.changed = true;
        set(&self.device, 0x100, &probe)?;
        let p = get(&self.device, 0x100)?;
        if p[2] != 2
            || p[3] != 5
            || u32::from_le_bytes(p[4..8].try_into()?) != 200000
            || u32::from_le_bytes(p[18..22].try_into()?) as usize != FRAME_BYTES
        {
            return Err(err("unsupported negotiated UVC profile").into());
        }
        let payload = u32::from_le_bytes(p[22..26].try_into()?) as usize;
        if payload != 12288 {
            return Err(err(format!("unqualified USB payload size {payload}")).into());
        }
        set(&self.device, 0x200, &p)?;
        Ok(payload)
    }
    fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.closed {
            return Ok(());
        }
        let mut errors = Vec::new();
        if self.changed {
            if let Some(p) = &self.probe
                && let Err(e) = set(&self.device, 0x100, p)
            {
                errors.push(e.to_string());
            }
            if let Some(p) = &self.commit
                && let Err(e) = set(&self.device, 0x200, p)
            {
                errors.push(e.to_string());
            }
            self.changed = false;
        }
        self.stream.take();
        self.control.take();
        self.closed = true;
        // nusb reattaches the original control-interface kernel driver on drop.
        // Verify rebinding explicitly rather than relying only on a Drop side effect.
        for interface in [2, 3] {
            if let Err(e) = self.device.attach_kernel_driver(interface) {
                // Already bound is acceptable only when sysfs confirms both UVC nodes below.
                eprintln!("attach interface {interface}: {e}");
            }
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let restored = scancraft_device::discover()
                .map(|n| {
                    n.iter().any(|n| n.interface == 2 && !n.metadata)
                        && n.iter().any(|n| n.interface == 2 && n.metadata)
                })
                .unwrap_or(false);
            if restored {
                break;
            }
            let _ = self.device.attach_kernel_driver(2);
            let _ = self.device.attach_kernel_driver(3);
            if Instant::now() > deadline {
                errors.push("right-IR kernel nodes were not restored".into());
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(err(errors.join("; ")).into())
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            eprintln!("USB session restoration failed: {e}");
        }
    }
}
#[derive(Default)]
struct Assembler {
    fid: Option<u8>,
    pts: Option<u32>,
    data: Vec<u8>,
    bad: bool,
    valid: u64,
    rejected: u64,
    header_errors: u64,
    last_pts: Option<u32>,
    first: Option<Vec<u8>>,
}
impl Assembler {
    fn finish(&mut self) {
        if self.fid.take().is_none() {
            return;
        }
        let pts = self.pts.take();
        let increasing = match (self.last_pts, pts) {
            (Some(old), Some(new)) => {
                let d = new.wrapping_sub(old);
                d != 0 && d < 0x8000_0000
            }
            (None, Some(_)) => true,
            _ => false,
        };
        if self.data.len() == FRAME_BYTES && !self.bad && increasing {
            self.valid += 1;
            self.last_pts = pts;
            if self.first.is_none() {
                self.first = Some(self.data.clone());
            }
        } else {
            self.rejected += 1;
        }
        self.data.clear();
        self.bad = false;
    }
    fn push(&mut self, packet: &[u8]) -> io::Result<()> {
        if packet.is_empty() {
            return Ok(());
        }
        if packet.len() < 12 {
            return Err(err("short UVC USB packet"));
        }
        let n = packet[0] as usize;
        let flags = packet[1];
        if n < 12 || n > packet.len() || n > 64 || flags & 0x0c != 0x0c {
            return Err(err("unexpected UVC header"));
        }
        let fid = flags & 1;
        let pts = u32::from_le_bytes(packet[2..6].try_into().unwrap());
        if self.fid.is_some() && (self.fid != Some(fid) || self.pts != Some(pts)) {
            self.finish();
        }
        if self.fid.is_none() {
            self.fid = Some(fid);
            self.pts = Some(pts);
        }
        if flags & 0x40 != 0 {
            self.bad = true;
            self.header_errors += 1;
        }
        if self.data.len() + packet.len() - n > FRAME_BYTES {
            self.bad = true;
        } else {
            self.data.extend_from_slice(&packet[n..]);
        }
        if flags & 2 != 0 {
            self.finish();
        }
        Ok(())
    }
}
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if !cfg!(feature = "hardware-experiment") {
        return Err(err(
            "direct USB hardware mode disabled after failed driver restoration; use the V4L2 app",
        )
        .into());
    }
    if !args.iter().any(|a| a == "--allow-hardware") {
        return Err(err("experimental diagnostic requires --allow-hardware").into());
    }
    let count = args
        .iter()
        .position(|a| a == "--frames")
        .map(|i| {
            args.get(i + 1)
                .ok_or("missing count")?
                .parse::<u64>()
                .map_err(|_| "invalid count")
        })
        .transpose()?
        .unwrap_or(500);
    let depth = args
        .iter()
        .position(|a| a == "--queue")
        .map(|i| {
            args.get(i + 1)
                .ok_or("missing queue depth")?
                .parse::<usize>()
                .map_err(|_| "invalid queue depth")
        })
        .transpose()?
        .unwrap_or(32);
    if count == 0 || !(1..=128).contains(&depth) {
        return Err(err("invalid frame count or queue depth").into());
    }
    let _lock = SessionLock::acquire()?;
    let devices: Vec<_> = nusb::list_devices()
        .wait()?
        .filter(|d| d.vendor_id() == 0x2bc5 && d.product_id() == 0x06a8)
        .collect();
    if devices.len() != 1 {
        return Err(err("requires exactly one Raptor").into());
    }
    let device = devices[0].open().wait()?;
    let mut session = Session::open(device)?;
    let payload = session.configure()?;
    let mut assembly = Assembler::default();
    let start = Instant::now();
    let mut transfers = 0;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut endpoint = session
            .stream
            .as_ref()
            .ok_or("no streaming interface")?
            .endpoint::<Bulk, In>(0x82)?;
        for _ in 0..depth {
            endpoint.submit(endpoint.allocate(payload));
        }
        while assembly.valid < count {
            if start.elapsed().as_secs() > count / 10 + 20 {
                return Err(err("native USB capture deadline exceeded").into());
            }
            let completion = endpoint
                .wait_next_complete(TIMEOUT)
                .ok_or("USB transfer timed out")?;
            let mut buffer = completion.into_result()?;
            transfers += 1;
            assembly.push(&buffer)?;
            buffer.clear();
            buffer.set_requested_len(payload);
            endpoint.submit(buffer);
        }
        endpoint.cancel_all();
        let drain_deadline = Instant::now() + TIMEOUT;
        while endpoint.pending() > 0 {
            let remaining = drain_deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() || endpoint.wait_next_complete(remaining).is_none() {
                return Err(err("USB cancellation did not drain in time").into());
            }
        }
        drop(endpoint);
        Ok(())
    })();
    session.close()?;
    result?;
    if let Some(i) = args.iter().position(|a| a == "--save-first") {
        let path = PathBuf::from(args.get(i + 1).ok_or("missing snapshot path")?);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        file.write_all(b"P5\n1920 1200\n255\n")?;
        file.write_all(assembly.first.as_ref().ok_or("no image")?)?;
        file.sync_all()?;
    }
    println!(
        "{}",
        serde_json::json!({"native_rust":true,"transport":"nusb usbfs","interface":3,"queue_depth":depth,"valid_frames":assembly.valid,"rejected_frames":assembly.rejected,"uvc_header_errors":assembly.header_errors,"usb_transfers":transfers,"payload_size":payload,"elapsed_seconds":start.elapsed().as_secs_f64(),"kernel_driver_restored":true})
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn packet(fid: u8, pts: u32, payload: usize, eof: bool) -> Vec<u8> {
        let mut p = vec![0; 12 + payload];
        p[0] = 12;
        p[1] = 0x8c | fid | if eof { 2 } else { 0 };
        p[2..6].copy_from_slice(&pts.to_le_bytes());
        p
    }
    #[test]
    fn complete_frame_and_truncated_frame() {
        let mut a = Assembler::default();
        a.push(&packet(0, 1, FRAME_BYTES, true)).unwrap();
        assert_eq!(a.valid, 1);
        a.push(&packet(1, 2, 10, true)).unwrap();
        assert_eq!(a.rejected, 1);
    }
    #[test]
    fn fid_change_rejects_partial_and_recovers() {
        let mut a = Assembler::default();
        a.push(&packet(0, 1, 10, false)).unwrap();
        a.push(&packet(1, 2, FRAME_BYTES, true)).unwrap();
        assert_eq!((a.valid, a.rejected), (1, 1));
    }
    #[test]
    fn malformed_headers_are_errors() {
        for n in 1..12 {
            assert!(Assembler::default().push(&vec![0; n]).is_err());
        }
    }
}
