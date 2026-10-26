//! Read-only non-atomic V4L2 format/rate snapshots; no streaming/control writes.
use super::{VIDEO, discover, invalid, ioctl, open};
use serde::Serialize;
use std::io;
use v4l2_sys_mit as v;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct State {
    pub interface: u8,
    pub width: u32,
    pub height: u32,
    pub fourcc: [u8; 4],
    pub field: u32,
    pub bytes_per_line: u32,
    pub image_size_bytes: u32,
    pub colorspace: u32,
    pub frame_interval_numerator: u32,
    pub frame_interval_denominator: u32,
}
pub fn read(interface: u8) -> io::Result<State> {
    let nodes = discover()?;
    let matches: Vec<_> = nodes
        .iter()
        .filter(|n| n.interface == interface && !n.metadata)
        .collect();
    if matches.len() != 1 {
        return Err(invalid(
            "requires exactly one Raptor video node for stream-state snapshot",
        ));
    }
    let fd = open(&matches[0].path)?;
    from_fd(&fd, interface)
}
/// GET-only snapshot on the actual selected stream descriptor.
pub(crate) fn from_fd(fd: &std::fs::File, interface: u8) -> io::Result<State> {
    // SAFETY: zero-valid Linux UAPI POD initialized before the GET calls.
    let mut format: v::v4l2_format = unsafe { std::mem::zeroed() };
    format.type_ = VIDEO;
    ioctl(fd, 4, 3, &mut format)?;
    // SAFETY: the requested VIDEO type selects the initialized pix union member.
    let pix = unsafe { format.fmt.pix };
    // SAFETY: zero-valid Linux UAPI POD, type VIDEO selects capture timeperframe.
    let mut rate: v::v4l2_streamparm = unsafe { std::mem::zeroed() };
    rate.type_ = VIDEO;
    ioctl(fd, 21, 3, &mut rate)?;
    // SAFETY: successful GET_PARM with VIDEO type initializes capture parameters.
    let interval = unsafe { rate.parm.capture.timeperframe };
    Ok(State {
        interface,
        width: pix.width,
        height: pix.height,
        fourcc: pix.pixelformat.to_le_bytes(),
        field: pix.field,
        bytes_per_line: pix.bytesperline,
        image_size_bytes: pix.sizeimage,
        colorspace: pix.colorspace,
        frame_interval_numerator: interval.numerator,
        frame_interval_denominator: interval.denominator,
    })
}
