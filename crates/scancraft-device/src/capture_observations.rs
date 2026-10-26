//! Bounded GET-only setup observations. No source binding or per-frame attestation.
use crate::{Node, standard_controls, stream_state};
use scancraft_core::StreamProfile;
use serde::Serialize;
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Debug, Serialize)]
pub struct Observation<T> {
    pub value: Option<T>,
    pub error: Option<String>,
}
impl<T> Observation<T> {
    fn from(result: io::Result<T>) -> Self {
        match result {
            Ok(value) => Self {
                value: Some(value),
                error: None,
            },
            Err(e) => Self {
                value: None,
                error: Some(e.to_string()),
            },
        }
    }
}
#[derive(Debug, Serialize)]
pub struct Attribute {
    pub name: &'static str,
    pub observation: Observation<String>,
}
#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub schema_version: u32,
    pub sampling_phase: &'static str,
    pub read_only: bool,
    pub observed_unix_us: Option<u64>,
    pub requested_profile: StreamProfile,
    pub video_path_display: String,
    pub usb_path_display: String,
    pub stream_state: Observation<stream_state::State>,
    pub standard_controls: Vec<standard_controls::Reading>,
    pub usb_reported_attributes: Vec<Attribute>,
    pub source_device_binding_verified: bool,
    pub per_frame_settings_verified: bool,
    pub firmware_version_status: &'static str,
    pub projector_mode_status: &'static str,
    pub calibration_status: &'static str,
}
const ATTRIBUTES: [&str; 9] = [
    "idVendor",
    "idProduct",
    "serial",
    "manufacturer",
    "product",
    "bcdDevice",
    "busnum",
    "devnum",
    "speed",
];
fn attribute(root: &Path, name: &str) -> io::Result<String> {
    let mut data = Vec::new();
    File::open(root.join(name))?
        .take(513)
        .read_to_end(&mut data)?;
    if data.len() > 512 {
        return Err(io::Error::other(
            "USB descriptor exceeds 512-byte observation limit",
        ));
    }
    let text = String::from_utf8(data).map_err(io::Error::other)?;
    let value = text.trim();
    if value.is_empty() {
        return Err(io::Error::other("USB descriptor is empty"));
    }
    Ok(value.into())
}
pub(crate) fn observe(fd: &File, node: &Node, profile: &StreamProfile) -> Snapshot {
    Snapshot {
        schema_version: 1,
        sampling_phase: "after_format_rate_setup_before_video_stream_on; non-atomic observations",
        read_only: true,
        observed_unix_us: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|d| u64::try_from(d.as_micros()).ok()),
        requested_profile: profile.clone(),
        video_path_display: node.path.display().to_string(),
        usb_path_display: node.usb_path.display().to_string(),
        stream_state: Observation::from(stream_state::from_fd(fd, profile.interface)),
        standard_controls: standard_controls::read_controls(fd),
        usb_reported_attributes: ATTRIBUTES
            .into_iter()
            .map(|name| Attribute {
                name,
                observation: Observation::from(attribute(&node.usb_path, name)),
            })
            .collect(),
        source_device_binding_verified: false,
        per_frame_settings_verified: false,
        firmware_version_status: "not_queried; bcdDevice is only the reported USB descriptor revision",
        projector_mode_status: "not_queried_or_inferred",
        calibration_status: "not_loaded_or_verified",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_get_queries_remain_unknown_on_a_non_device_descriptor() {
        let fd = File::open("/dev/null").unwrap();
        let node = Node {
            path: "/dev/null".into(),
            interface: 0,
            metadata: false,
            usb_path: "/missing-scancraft-usb-root".into(),
        };
        let profile = StreamProfile {
            interface: 0,
            width: 2,
            height: 2,
            fourcc: *b"GREY",
            fps: 30,
        };
        let value = observe(&fd, &node, &profile);
        assert!(value.stream_state.value.is_none());
        assert!(value.stream_state.error.is_some());
        assert!(
            value
                .standard_controls
                .iter()
                .all(|r| r.value.is_none() && r.error.is_some())
        );
        assert!(
            value
                .usb_reported_attributes
                .iter()
                .all(|a| a.observation.value.is_none() && a.observation.error.is_some())
        );
        assert!(!value.per_frame_settings_verified);
        assert!(!value.source_device_binding_verified);
        assert!(value.read_only);
    }
    #[test]
    fn attributes_are_bounded_utf8_and_empty_values_are_not_identities() {
        let path = std::env::temp_dir().join(format!(
            "scancraft-observed-attribute-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::write(path.join("serial"), b"  synthetic-serial\n").unwrap();
        assert_eq!(attribute(&path, "serial").unwrap(), "synthetic-serial");
        std::fs::write(path.join("serial"), vec![b'x'; 513]).unwrap();
        assert!(attribute(&path, "serial").is_err());
        std::fs::write(path.join("serial"), b" \n").unwrap();
        assert!(attribute(&path, "serial").is_err());
        std::fs::write(path.join("serial"), [255]).unwrap();
        assert!(attribute(&path, "serial").is_err());
        std::fs::remove_dir_all(path).unwrap();
    }
}
