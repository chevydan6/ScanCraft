//! Linux UVC acquisition. No proprietary runtime or USB reset is used.
use scancraft_core::{Frame, StreamProfile, validate_frame};
use serde::Serialize;
use std::{
    fs::{self, File, OpenOptions},
    io,
    mem::{size_of, zeroed},
    os::fd::AsRawFd,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    ptr::NonNull,
    time::{Duration, Instant},
};
mod cleanup_errors;
use cleanup_errors::CleanupErrors;
use v4l2_sys_mit as v;
fn invalid(s: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s.into())
}
const VIDEO: u32 = 1;
const META: u32 = 13;
const MMAP: u32 = 1;
const fn request<T>(nr: u32, dir: u32) -> libc::c_ulong {
    ((dir << 30) | ((size_of::<T>() as u32) << 16) | (b'V' as u32) << 8 | nr) as libc::c_ulong
}
// All callers pass the matching Linux UAPI POD type and an open owned fd.
fn ioctl<T>(fd: &File, nr: u32, dir: u32, value: &mut T) -> io::Result<()> {
    loop {
        // SAFETY: value points to writable, initialized storage of the ioctl's exact ABI type.
        let result = unsafe { libc::ioctl(fd.as_raw_fd(), request::<T>(nr, dir), value as *mut T) };
        if result >= 0 {
            return Ok(());
        }
        let e = io::Error::last_os_error();
        if e.kind() != io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
}
fn open(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(path)
}

#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub path: PathBuf,
    pub interface: u8,
    pub metadata: bool,
    pub usb_path: PathBuf,
}
pub fn discover() -> io::Result<Vec<Node>> {
    let mut out = Vec::new();
    for e in fs::read_dir("/sys/class/video4linux")? {
        let e = e?;
        let interface = fs::canonicalize(e.path().join("device"))?;
        let Some(usb) = interface.parent() else {
            continue;
        };
        if fs::read_to_string(usb.join("idVendor"))
            .unwrap_or_default()
            .trim()
            != "2bc5"
            || fs::read_to_string(usb.join("idProduct"))
                .unwrap_or_default()
                .trim()
                != "06a8"
        {
            continue;
        }
        let number = u8::from_str_radix(
            fs::read_to_string(interface.join("bInterfaceNumber"))?.trim(),
            16,
        )
        .map_err(|e| invalid(e.to_string()))?;
        let path = PathBuf::from("/dev").join(e.file_name());
        let fd = open(&path)?;
        // SAFETY: Linux capability structure is a C integer/byte-array POD; zero is valid.
        let mut cap: v::v4l2_capability = unsafe { zeroed() };
        ioctl(&fd, 0, 2, &mut cap)?;
        let caps = if cap.capabilities & 0x8000_0000 != 0 {
            cap.device_caps
        } else {
            cap.capabilities
        };
        if caps & (1 | 0x0080_0000) == 0 {
            continue;
        }
        out.push(Node {
            path,
            interface: number,
            metadata: caps & 0x0080_0000 != 0,
            usb_path: usb.to_owned(),
        });
    }
    out.sort_by_key(|n| (n.interface, n.metadata));
    Ok(out)
}

struct Mapping {
    pointer: NonNull<libc::c_void>,
    length: usize,
}
impl Drop for Mapping {
    fn drop(&mut self) {
        // SAFETY: this mapping is exclusively owned and no frame borrows survive dequeue().
        unsafe {
            libc::munmap(self.pointer.as_ptr(), self.length);
        }
    }
}
struct Stream {
    fd: File,
    kind: u32,
    maps: Vec<Mapping>,
    streaming: bool,
    allocated: bool,
    old_format: Option<v::v4l2_format>,
    old_rate: Option<v::v4l2_streamparm>,
    observations: Option<capture_observations::Snapshot>,
}
struct Packet {
    sequence: u32,
    timestamp: u64,
    flags: u32,
    data: Vec<u8>,
}
impl Stream {
    fn new(node: &Node, profile: Option<&StreamProfile>, observe: bool) -> io::Result<Self> {
        let mut s = Self {
            fd: open(&node.path)?,
            kind: if node.metadata { META } else { VIDEO },
            maps: Vec::new(),
            streaming: false,
            allocated: false,
            old_format: None,
            old_rate: None,
            observations: None,
        };
        // SAFETY: these UAPI structs consist of C POD fields and unions with zero-valid representations.
        let mut fmt: v::v4l2_format = unsafe { zeroed() };
        fmt.type_ = s.kind;
        ioctl(&s.fd, 4, 3, &mut fmt)?;
        if let Some(p) = profile {
            s.old_format = Some(fmt);
            let mut rate: v::v4l2_streamparm = unsafe { zeroed() };
            rate.type_ = VIDEO;
            ioctl(&s.fd, 21, 3, &mut rate)?;
            s.old_rate = Some(rate);
            let mut f: v::v4l2_format = unsafe { zeroed() };
            f.type_ = VIDEO;
            f.fmt.pix.width = p.width;
            f.fmt.pix.height = p.height;
            f.fmt.pix.pixelformat = u32::from_le_bytes(p.fourcc);
            f.fmt.pix.field = 1;
            ioctl(&s.fd, 5, 3, &mut f)?;
            // SAFETY: format type VIDEO selects the pix member written by the kernel.
            let actual = unsafe { f.fmt.pix };
            if actual.width != p.width
                || actual.height != p.height
                || actual.pixelformat != u32::from_le_bytes(p.fourcc)
            {
                return Err(invalid("camera changed the requested format"));
            }
            if actual.sizeimage as usize > 64 * 1024 * 1024 {
                return Err(invalid("camera buffer too large"));
            }
            let mut r: v::v4l2_streamparm = unsafe { zeroed() };
            r.type_ = VIDEO;
            r.parm.capture.timeperframe.numerator = 1;
            r.parm.capture.timeperframe.denominator = p.fps;
            ioctl(&s.fd, 22, 3, &mut r)?;
            // SAFETY: VIDEO selects capture rate member.
            let rate = unsafe { r.parm.capture.timeperframe };
            if rate.numerator != 1 || rate.denominator != p.fps {
                return Err(invalid("camera changed requested frame rate"));
            }
        } else {
            // SAFETY: META type selects the metadata format member.
            if unsafe { fmt.fmt.meta.dataformat } != u32::from_le_bytes(*b"UVCH") {
                return Err(invalid("metadata is not UVC"));
            }
        }
        let mut req: v::v4l2_requestbuffers = unsafe { zeroed() };
        req.count = 4;
        req.type_ = s.kind;
        req.memory = MMAP;
        ioctl(&s.fd, 8, 3, &mut req)?;
        s.allocated = true;
        if req.count == 0 || req.count > 32 {
            return Err(invalid("unexpected buffer count"));
        }
        for index in 0..req.count {
            let mut b = s.buffer(index);
            ioctl(&s.fd, 9, 3, &mut b)?;
            if b.length == 0 || b.length > 64 * 1024 * 1024 {
                return Err(invalid("invalid mmap buffer length"));
            }
            // SAFETY: kernel supplies the offset/length for this fd; mapping lifetime is owned by Stream.
            let ptr = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    b.length as usize,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    s.fd.as_raw_fd(),
                    b.m.offset as libc::off_t,
                )
            };
            if ptr == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }
            let pointer = NonNull::new(ptr).ok_or_else(|| invalid("null mmap address"))?;
            s.maps.push(Mapping {
                pointer,
                length: b.length as usize,
            });
            ioctl(&s.fd, 15, 3, &mut b)?;
        }
        if observe && let Some(profile) = profile {
            s.observations = Some(capture_observations::observe(&s.fd, node, profile));
        }
        let mut kind = s.kind;
        ioctl(&s.fd, 18, 1, &mut kind)?;
        s.streaming = true;
        Ok(s)
    }
    fn buffer(&self, index: u32) -> v::v4l2_buffer {
        // SAFETY: zero is valid for all C POD members; type and memory select the mmap union.
        let mut b: v::v4l2_buffer = unsafe { zeroed() };
        b.index = index;
        b.type_ = self.kind;
        b.memory = MMAP;
        b
    }
    fn dequeue(&mut self, timeout: Duration) -> io::Result<Packet> {
        let mut p = libc::pollfd {
            fd: self.fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: pointer describes exactly one live pollfd for the owned stream fd.
        let result =
            unsafe { libc::poll(&mut p, 1, timeout.as_millis().min(i32::MAX as u128) as i32) };
        if result < 0 {
            return Err(io::Error::last_os_error());
        }
        if result == 0 {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "camera poll timed out",
            ));
        }
        if p.revents & libc::POLLIN == 0 {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "camera disconnected or poll error",
            ));
        }
        let mut b = self.buffer(0);
        ioctl(&self.fd, 17, 3, &mut b)?;
        let packet = (|| {
            let map = self
                .maps
                .get(b.index as usize)
                .ok_or_else(|| invalid("kernel buffer index out of range"))?;
            if b.bytesused as usize > map.length {
                return Err(invalid("kernel bytesused exceeds mapping"));
            }
            if b.timestamp.tv_sec < 0 || b.timestamp.tv_usec < 0 || b.timestamp.tv_usec >= 1_000_000
            {
                return Err(invalid("invalid kernel timestamp"));
            }
            // SAFETY: after DQBUF kernel relinquishes buffer; bounds checked above; copy ends before QBUF.
            let data = unsafe {
                std::slice::from_raw_parts(map.pointer.as_ptr().cast::<u8>(), b.bytesused as usize)
            }
            .to_vec();
            Ok(Packet {
                sequence: b.sequence,
                timestamp: b.timestamp.tv_sec as u64 * 1_000_000 + b.timestamp.tv_usec as u64,
                flags: b.flags,
                data,
            })
        })();
        ioctl(&self.fd, 15, 3, &mut b)?;
        packet
    }
    fn close(&mut self) -> io::Result<()> {
        let mut errors = CleanupErrors::default();
        if self.streaming {
            let mut kind = self.kind;
            errors.record("stop streaming", ioctl(&self.fd, 19, 1, &mut kind));
            self.streaming = false;
        }
        self.maps.clear();
        if self.allocated {
            // SAFETY: zero-valid UAPI POD; count zero releases all requested buffers.
            let mut r: v::v4l2_requestbuffers = unsafe { zeroed() };
            r.type_ = self.kind;
            r.memory = MMAP;
            errors.record("release buffers", ioctl(&self.fd, 8, 3, &mut r));
            self.allocated = false;
        }
        if let Some(mut f) = self.old_format.take() {
            errors.record("restore format", ioctl(&self.fd, 5, 3, &mut f));
        }
        if let Some(mut r) = self.old_rate.take() {
            errors.record("restore frame rate", ioctl(&self.fd, 22, 3, &mut r));
        }
        errors.finish()
    }
}
impl Drop for Stream {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            eprintln!("stream cleanup failed (buffer type {}): {e}", self.kind);
        }
    }
}

mod rejection;
pub use rejection::RejectedBufferInfo;

/// Owns both native nodes. Metadata is paired only by exact kernel sequence.
pub struct Capture {
    video: Stream,
    metadata: Stream,
    pending: Option<Packet>,
    pub profile: StreamProfile,
    pub rejected: u64,
    pub rejected_before_first_valid: u64,
    pub rejected_after_first_valid: u64,
    pub metadata_missing: u64,
    pub sequence_drops: u64,
    last: Option<(u32, u64, u32)>,
    pub duplicate_sensor_pts: u64,
    pub rejection_samples: Vec<RejectedBufferInfo>,
    pub ongoing_rejection_samples: Vec<RejectedBufferInfo>,
}
impl Capture {
    pub fn open(profile: StreamProfile) -> io::Result<Self> {
        Self::open_inner(profile, false)
    }
    /// Capture setup observations before video STREAMON, using GET-only queries.
    pub fn open_recording(profile: StreamProfile) -> io::Result<Self> {
        Self::open_inner(profile, true)
    }
    pub fn recording_observations(&self) -> Option<&capture_observations::Snapshot> {
        self.video.observations.as_ref()
    }
    fn open_inner(profile: StreamProfile, observe: bool) -> io::Result<Self> {
        let nodes = discover()?;
        let videos: Vec<_> = nodes
            .iter()
            .filter(|n| n.interface == profile.interface && !n.metadata)
            .collect();
        if videos.len() != 1 {
            return Err(invalid(
                "requires exactly one Raptor capture node for selected interface",
            ));
        }
        let node = videos[0];
        let meta = nodes
            .iter()
            .find(|n| n.metadata && n.interface == node.interface && n.usb_path == node.usb_path)
            .ok_or_else(|| invalid("no matching metadata node"))?;
        let metadata = Stream::new(meta, None, false)?;
        let video = Stream::new(node, Some(&profile), observe)?;
        Ok(Self {
            video,
            metadata,
            pending: None,
            profile,
            rejected: 0,
            rejected_before_first_valid: 0,
            rejected_after_first_valid: 0,
            metadata_missing: 0,
            sequence_drops: 0,
            last: None,
            duplicate_sensor_pts: 0,
            rejection_samples: Vec::new(),
            ongoing_rejection_samples: Vec::new(),
        })
    }
    pub fn next_frame(&mut self) -> io::Result<Frame> {
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            let v = self.video.dequeue(Duration::from_secs(2))?;
            let mut meta = None;
            for _ in 0..32 {
                let m = if let Some(m) = self.pending.take() {
                    m
                } else {
                    self.metadata.dequeue(Duration::from_millis(200))?
                };
                if m.sequence == v.sequence {
                    meta = Some(m);
                    break;
                }
                if m.sequence.wrapping_sub(v.sequence) < 0x8000_0000 {
                    self.pending = Some(m);
                    break;
                }
            }
            let Some(m) = meta else {
                self.metadata_missing += 1;
                continue;
            };
            let metadata_flags = m.flags;
            let frame = Frame {
                sequence: v.sequence,
                host_timestamp_us: v.timestamp,
                flags: v.flags,
                metadata: m.data,
                data: v.data,
            };
            if metadata_flags & 0x40 != 0 {
                self.reject(&frame, metadata_flags, "metadata buffer error flag");
                continue;
            }
            if let Err(reason) = validate_frame(&self.profile, &frame) {
                self.reject(&frame, metadata_flags, reason);
                continue;
            }
            let pts = scancraft_core::uvc_pts(&frame.metadata)
                .map_err(invalid)?
                .ok_or_else(|| invalid("missing sensor timestamp"))?;
            if let Some((seq, stamp, old_pts)) = self.last {
                let delta = frame.sequence.wrapping_sub(seq);
                if delta == 0 || delta >= 0x8000_0000 || frame.host_timestamp_us <= stamp {
                    return Err(invalid("non-monotonic frame sequence/timestamp"));
                }
                let ticks = pts.wrapping_sub(old_pts);
                if ticks == 0 {
                    self.duplicate_sensor_pts += 1;
                    self.reject(&frame, metadata_flags, "duplicate sensor PTS");
                    continue;
                }
                if ticks >= 0x8000_0000 {
                    return Err(invalid("sensor PTS moved backwards or reset"));
                }
                self.sequence_drops += u64::from(delta - 1);
            }
            self.last = Some((frame.sequence, frame.host_timestamp_us, pts));
            return Ok(frame);
        }
        Err(invalid("no validated frame within eight seconds"))
    }
    fn reject(&mut self, frame: &Frame, metadata_flags: u32, reason: &str) {
        self.rejected += 1;
        if self.last.is_some() {
            self.rejected_after_first_valid += 1;
        } else {
            self.rejected_before_first_valid += 1;
        }
        rejection::retain(
            &mut self.rejection_samples,
            &mut self.ongoing_rejection_samples,
            self.last.is_some(),
            || RejectedBufferInfo::new(&self.profile, frame, metadata_flags, reason),
        );
    }
    pub fn close(mut self) -> io::Result<()> {
        let mut errors = CleanupErrors::default();
        errors.record("video stream", self.video.close());
        errors.record("metadata stream", self.metadata.close());
        errors.finish()
    }
}

/// Process-wide cooperation with native clients and the development Wine bridge.
/// This is an advisory lock, not a security boundary against other camera apps.
pub struct SessionLock {
    _files: Vec<File>,
}
impl SessionLock {
    pub fn acquire() -> io::Result<Self> {
        use std::os::unix::fs::MetadataExt;
        // SAFETY: getuid has no preconditions.
        let uid = unsafe { libc::getuid() };
        let runtime = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let mut paths = vec![
            runtime.join(format!("scancraft-{uid}.lock")),
            runtime.join(format!("open-raptor-{uid}.lock")),
        ];
        let bridge = std::env::var_os("SCANCRAFT_BRIDGE_LOCK")
            .or_else(|| std::env::var_os("OPEN_RAPTOR_BRIDGE_LOCK"))
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|p| {
                    PathBuf::from(p).join("raptor-driver-research/wine-bridge/session.lock")
                })
            });
        if let Some(path) = bridge
            && path.exists()
        {
            paths.push(path);
        }
        let mut files = Vec::new();
        for path in paths {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .mode(0o600)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
                .open(path)?;
            if file.metadata()?.uid() != uid {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "session lock has another owner",
                ));
            }
            // SAFETY: flock receives a valid descriptor owned for the lifetime of SessionLock.
            if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "close Creality Scan or another scanner session first",
                ));
            }
            files.push(file);
        }
        Ok(Self { _files: files })
    }
}
pub mod control;
pub mod ir_gain;
pub mod standard_controls;

pub mod stream_state;

pub mod capture_observations;

pub mod stream_group;
