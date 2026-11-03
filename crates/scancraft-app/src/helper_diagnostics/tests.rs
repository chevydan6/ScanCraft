use super::*;
use std::{
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "scancraft-diagnostics-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[test]
fn state_paths_are_explicit_absolute_and_do_not_fall_back_to_tmp() {
    assert_eq!(
        state_root(Some("/example/state".into()), None).unwrap(),
        PathBuf::from("/example/state")
    );
    assert_eq!(
        state_root(None, Some("/example/home".into())).unwrap(),
        PathBuf::from("/example/home/.local/state")
    );
    assert_eq!(
        state_root(Some("".into()), Some("/example/home".into())).unwrap(),
        PathBuf::from("/example/home/.local/state")
    );
    assert!(state_root(Some("relative".into()), Some("/example/home".into())).is_err());
    assert!(state_root(None, None).is_err());
}
#[test]
fn private_log_is_synced_regular_private_and_survives_writer_close() {
    let temp = Temp::new();
    let (path, file) = create(&temp.0).unwrap();
    assert_eq!(file.metadata().unwrap().mode() & 0o777, 0o600);
    assert_eq!(
        fs::metadata(path.parent().unwrap()).unwrap().mode() & 0o777,
        0o700
    );
    assert!(path.starts_with(temp.0.join("scancraft/diagnostics")));
    let header: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(header["schema_version"], 1);
    assert_eq!(header["kind"], "scancraft_helper_diagnostic");
    drop(file);
    assert!(path.is_file());
}
#[test]
fn state_symlinks_shared_permissions_and_non_directories_fail_without_following() {
    for which in 0..4 {
        let temp = Temp::new();
        let outside = temp.0.join("outside");
        DirBuilder::new().mode(0o700).create(&outside).unwrap();
        match which {
            0 => std::os::unix::fs::symlink(&outside, temp.0.join("scancraft")).unwrap(),
            1 => {
                fs::create_dir(temp.0.join("scancraft")).unwrap();
                std::os::unix::fs::symlink(&outside, temp.0.join("scancraft/diagnostics")).unwrap();
            }
            2 => fs::set_permissions(&temp.0, fs::Permissions::from_mode(0o777)).unwrap(),
            _ => {
                fs::create_dir(temp.0.join("scancraft")).unwrap();
                fs::write(temp.0.join("scancraft/diagnostics"), b"keep").unwrap();
            }
        }
        assert!(create(&temp.0).is_err());
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    }
}
#[test]
fn retention_preserves_active_inherited_logs_and_unrecognized_files() {
    let temp = Temp::new();
    let (active_path, active) = create(&temp.0).unwrap();
    let directory = active_path.parent().unwrap();
    fs::write(directory.join("user-notes.txt"), b"keep").unwrap();
    let unknown = directory.join("scancraft-capture-1-2-3.log");
    fs::write(&unknown, b"not our header\n").unwrap();
    fs::set_permissions(&unknown, fs::Permissions::from_mode(0o600)).unwrap();
    let symlink = directory.join("scancraft-capture-4-5-6.log");
    std::os::unix::fs::symlink("user-notes.txt", &symlink).unwrap();
    // A child inherits the same open description and lock even after its parent
    // closes its duplicate. This child is entirely device-free.
    let mut child = Command::new("sh")
        .args(["-c", "cat >/dev/null"])
        .stdin(Stdio::piped())
        .stderr(Stdio::from(active))
        .spawn()
        .unwrap();
    for _ in 0..KEEP_INACTIVE + 4 {
        let (_, file) = create(&temp.0).unwrap();
        drop(file);
    }
    assert!(active_path.is_file());
    assert_eq!(fs::read(&unknown).unwrap(), b"not our header\n");
    assert!(symlink.is_symlink());
    assert_eq!(fs::read(directory.join("user-notes.txt")).unwrap(), b"keep");
    let managed = fs::read_dir(directory)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| {
            e.file_type().unwrap().is_file() && managed_name(&e.file_name().to_string_lossy())
        })
        .count();
    assert!(managed <= KEEP_INACTIVE + 3); // active + newest writer + unrecognized header.
    child.stdin.take();
    assert!(child.wait().unwrap().success());
}
#[test]
fn failure_context_is_nonblocking_and_does_not_claim_live_child_cleanup() {
    let temp = Temp::new();
    let (path, log) = create(&temp.0).unwrap();
    let mut child = Command::new("sh")
        .args(["-c", "cat >/dev/null; exit 42"])
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let text = failure(&mut child, "test helper", &path, &log, "missing final IPC");
    assert!(text.contains("still running"));
    assert!(text.contains("NOT confirmed"));
    assert!(text.contains("file present"));
    child.stdin.take();
    assert_eq!(child.wait().unwrap().code(), Some(42));
    let text = failure(&mut child, "test helper", &path, &log, "missing final IPC");
    assert!(text.contains("exit status: 42"));
    let stored = fs::read_to_string(&path).unwrap();
    assert!(
        stored
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .any(|record| record["kind"] == "supervisor_failure"
                && record["status"]
                    .as_str()
                    .unwrap()
                    .contains("exit status: 42"))
    );
    fs::remove_file(&path).unwrap();
    assert!(
        failure(&mut child, "test helper", &path, &log, "missing final IPC")
            .contains("unavailable")
    );
}

#[test]
fn diagnostic_inventory_is_bounded_without_removing_unrecognized_entries() {
    let temp = Temp::new();
    let (path, file) = create(&temp.0).unwrap();
    drop(file);
    for i in 0..MAX_ENTRIES {
        fs::write(path.parent().unwrap().join(format!("user-{i}.txt")), []).unwrap();
    }
    assert!(
        create(&temp.0)
            .err()
            .unwrap()
            .to_string()
            .contains("inventory bound")
    );
    assert!(path.is_file());
    assert!(path.parent().unwrap().join("user-0.txt").is_file());
}

#[test]
fn best_effort_notes_accept_oversized_json_formatters_without_panicking() {
    note(format_args!(
        "{}",
        serde_json::json!({"diagnostic_test":"é".repeat(8192)})
    ));
}
