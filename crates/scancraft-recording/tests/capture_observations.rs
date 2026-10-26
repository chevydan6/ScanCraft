use scancraft_core::{Frame, StreamProfile};
use scancraft_recording::{CaptureContext, Reader, Writer};
use serde_json::json;
#[test]
fn observations_preserve_trials_identity_calibration_and_header_roundtrip() {
    let mut context = CaptureContext {
        device_identity: Some("explicit synthetic identity".into()),
        calibration_bytes: Some(vec![1, 2, 3]),
        calibration_status: "synthetic_not_verified".into(),
        controls: json!({"gain_trial":{"requested_raw":17},"line_trial":{"physical_emission_verified":false}}),
    };
    let observed = json!({"schema_version":1,"source_device_binding_verified":false,"per_frame_settings_verified":false,"usb_reported_attributes":[{"name":"serial","observation":{"value":"synthetic","error":null}}],"standard_controls":[{"value":null,"error":"unsupported"}]});
    context.add_capture_observations(&observed).unwrap();
    assert_eq!(context.controls["gain_trial"]["requested_raw"], 17);
    let expected = serde_json::to_value(&context).unwrap();
    let mut writer = Writer::with_context(
        Vec::new(),
        StreamProfile {
            interface: 0,
            width: 2,
            height: 2,
            fourcc: *b"GREY",
            fps: 30,
        },
        context,
    )
    .unwrap();
    let mut metadata = vec![0; 16];
    metadata[10] = 6;
    metadata[11] = 4;
    writer
        .push(&Frame {
            sequence: 0,
            host_timestamp_us: 1,
            flags: 0,
            metadata,
            data: vec![7; 4],
        })
        .unwrap();
    let bytes = writer.finish().unwrap();
    let mut reader = Reader::new(bytes.as_slice()).unwrap();
    assert_eq!(
        serde_json::to_value(reader.context.as_ref().unwrap()).unwrap(),
        expected
    );
    assert_eq!(reader.read_frame().unwrap().unwrap().data, [7; 4]);
    assert!(reader.read_frame().unwrap().is_none());
}
#[test]
fn duplicate_invalid_controls_and_serialization_errors_leave_context_unchanged() {
    let mut context = CaptureContext {
        controls: json!(["opaque previous metadata"]),
        ..Default::default()
    };
    let before = serde_json::to_value(&context).unwrap();
    assert!(context.add_capture_observations(&json!({})).is_err());
    assert_eq!(serde_json::to_value(&context).unwrap(), before);
    context.controls = json!({});
    context
        .add_capture_observations(&json!({"original":true}))
        .unwrap();
    let before = serde_json::to_value(&context).unwrap();
    assert!(
        context
            .add_capture_observations(&json!({"replacement":true}))
            .is_err()
    );
    assert_eq!(serde_json::to_value(&context).unwrap(), before);
    struct Fails;
    impl serde::Serialize for Fails {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("injected"))
        }
    }
    let mut context = CaptureContext::default();
    let before = serde_json::to_value(&context).unwrap();
    assert!(context.add_capture_observations(&Fails).is_err());
    assert_eq!(serde_json::to_value(&context).unwrap(), before);
}
