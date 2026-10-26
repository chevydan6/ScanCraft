#[cfg(not(feature = "hardware-experiment"))]
#[test]
fn default_binary_refuses_hardware_before_opening_usb() {
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_scancraft-usb"))
        .args(["--allow-hardware", "--frames", "1"])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("hardware mode disabled"));
}
