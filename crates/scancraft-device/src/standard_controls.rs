//! Read-only allowlisted standard V4L2 control snapshots. No setter or vendor RPC.
use crate::{discover, ioctl, open};
use serde::Serialize;
use std::{io, mem::zeroed, path::PathBuf};
use v4l2_sys_mit as v;
#[derive(Debug, Serialize)]
pub struct Descriptor {
    pub name: String,
    pub control_type: u32,
    pub minimum: i32,
    pub maximum: i32,
    pub step: i32,
    pub default: i32,
    pub flags: u32,
}
#[derive(Debug, Serialize)]
pub struct Reading {
    pub id: u32,
    pub symbol: &'static str,
    pub descriptor: Option<Descriptor>,
    pub value: Option<i32>,
    pub exposure_seconds_standard_interpretation: Option<f64>,
    pub error: Option<String>,
}
#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub path: PathBuf,
    pub interface: u8,
    pub controls: Vec<Reading>,
}
const ALLOWED: [(u32, &str); 8] = [
    (v::V4L2_CID_EXPOSURE_AUTO, "exposure_auto"),
    (v::V4L2_CID_EXPOSURE_ABSOLUTE, "exposure_absolute"),
    (v::V4L2_CID_EXPOSURE_AUTO_PRIORITY, "exposure_auto_priority"),
    (v::V4L2_CID_AUTOGAIN, "autogain"),
    (v::V4L2_CID_GAIN, "gain"),
    (v::V4L2_CID_BRIGHTNESS, "brightness"),
    (v::V4L2_CID_CONTRAST, "contrast"),
    (v::V4L2_CID_GAMMA, "gamma"),
];
fn exposure_seconds(id: u32, value: i32) -> Option<f64> {
    (id == v::V4L2_CID_EXPOSURE_ABSOLUTE && value > 0).then_some(f64::from(value) * 0.0001)
}
/// Snapshot only discovered Raptor image nodes. Unsupported controls retain their
/// query error rather than invented defaults. Hold SessionLock in the caller.
pub fn snapshot() -> io::Result<Vec<Snapshot>> {
    let mut result = Vec::new();
    for node in discover()?.into_iter().filter(|n| !n.metadata) {
        let fd = open(&node.path)?;
        let controls = read_controls(&fd);
        result.push(Snapshot {
            path: node.path,
            interface: node.interface,
            controls,
        });
    }
    if result.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no Raptor image nodes",
        ));
    }
    Ok(result)
}
/// GET-only observations from the already selected capture descriptor.
pub(crate) fn read_controls(fd: &std::fs::File) -> Vec<Reading> {
    let mut controls = Vec::new();
    for (id, symbol) in ALLOWED {
        let mut reading = Reading {
            id,
            symbol,
            descriptor: None,
            value: None,
            exposure_seconds_standard_interpretation: None,
            error: None,
        };
        // SAFETY: v4l2_queryctrl contains only integer fields/byte arrays;
        // zero initializes reserved fields and is valid writable UAPI storage.
        let mut query: v::v4l2_queryctrl = unsafe { zeroed() };
        query.id = id;
        match ioctl(fd, 36, 3, &mut query) {
            Err(e) => reading.error = Some(format!("QUERYCTRL: {e}")),
            Ok(()) => {
                reading.descriptor = Some(Descriptor {
                    name: String::from_utf8_lossy(query.name.split(|b| *b == 0).next().unwrap())
                        .into_owned(),
                    control_type: query.type_,
                    minimum: query.minimum,
                    maximum: query.maximum,
                    step: query.step,
                    default: query.default_value,
                    flags: query.flags,
                });
                if query.flags & (v::V4L2_CTRL_FLAG_DISABLED | v::V4L2_CTRL_FLAG_WRITE_ONLY) != 0 {
                    reading.error =
                        Some("control is disabled or write-only; value was not queried".into());
                } else {
                    let mut value = v::v4l2_control { id, value: 0 };
                    match ioctl(fd, 27, 3, &mut value) {
                        Err(e) => reading.error = Some(format!("G_CTRL: {e}")),
                        Ok(()) => {
                            reading.value = Some(value.value);
                            reading.exposure_seconds_standard_interpretation =
                                exposure_seconds(id, value.value);
                        }
                    }
                }
            }
        }
        controls.push(reading);
    }
    controls
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_control_ioctl_layouts_match_linux_uapi() {
        assert_eq!(crate::request::<v::v4l2_queryctrl>(36, 3), 0xc0445624);
        assert_eq!(crate::request::<v::v4l2_control>(27, 3), 0xc008561b);
        let ids: std::collections::HashSet<_> = ALLOWED.iter().map(|x| x.0).collect();
        assert_eq!(ids.len(), ALLOWED.len());
    }
    #[test]
    fn exposure_units_are_interpreted_only_for_positive_absolute_exposure() {
        assert_eq!(
            exposure_seconds(v::V4L2_CID_EXPOSURE_ABSOLUTE, 100),
            Some(0.01)
        );
        assert_eq!(exposure_seconds(v::V4L2_CID_EXPOSURE_ABSOLUTE, 0), None);
        assert_eq!(exposure_seconds(v::V4L2_CID_GAIN, 100), None);
    }
}
