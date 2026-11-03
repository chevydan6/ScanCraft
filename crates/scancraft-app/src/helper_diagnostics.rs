//! Private helper logs and best-effort diagnostics, independent of required IPC.
pub(crate) use scancraft_core::diagnostics::note;
use std::{
    ffi::{CString, OsString},
    fs::{self, DirBuilder, File, OpenOptions},
    io::{self, BufRead, BufReader, Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    },
    path::{Component, Path, PathBuf},
    process::Child,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
const KEEP_INACTIVE: usize = 64;
const MAX_ENTRIES: usize = 1024;
static NEXT: AtomicU64 = AtomicU64::new(0);
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn state_root(xdg: Option<OsString>, home: Option<OsString>) -> io::Result<PathBuf> {
    let root =
        match xdg.filter(|s| !s.is_empty()) {
            Some(root) => PathBuf::from(root),
            None => PathBuf::from(home.ok_or_else(|| {
                invalid("HOME or XDG_STATE_HOME is required for helper diagnostics")
            })?)
            .join(".local/state"),
        };
    if !root.is_absolute() {
        return Err(invalid(
            "helper diagnostic state directory must be absolute",
        ));
    }
    Ok(root)
}
fn leaf(name: &str) -> io::Result<CString> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        return Err(invalid("diagnostic name must be one path component"));
    }
    CString::new(name).map_err(|_| invalid("diagnostic name contains NUL"))
}
fn open_at(directory: &File, name: &str, flags: i32) -> io::Result<File> {
    let name = leaf(name)?;
    // SAFETY: directory is live, name is a live NUL-terminated leaf, and mode is
    // provided for O_CREAT. A successful descriptor is exclusively transferred below.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            0o600 as libc::mode_t,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: successful openat returned a newly owned descriptor, not an alias.
    Ok(unsafe { File::from_raw_fd(fd) })
}
fn check_directory(directory: &File, uid: u32, private: bool) -> io::Result<()> {
    let metadata = directory.metadata()?;
    let forbidden = if private { 0o077 } else { 0o022 };
    if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & forbidden != 0 {
        return Err(invalid(
            "helper state directories must be owned by this user, non-writable by others, and diagnostics must be private",
        ));
    }
    Ok(())
}
fn child_directory(parent: &File, name: &str, uid: u32, private: bool) -> io::Result<File> {
    let component = leaf(name)?;
    // SAFETY: live directory, valid C leaf, ordinary directory mode.
    let created = unsafe { libc::mkdirat(parent.as_raw_fd(), component.as_ptr(), 0o700) };
    if created != 0 {
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::AlreadyExists {
            return Err(error);
        }
    }
    let directory = open_at(parent, name, libc::O_RDONLY | libc::O_DIRECTORY)?;
    check_directory(&directory, uid, private)?;
    if created == 0 {
        parent.sync_all()?;
    }
    Ok(directory)
}
fn lock(file: &File) -> io::Result<()> {
    // SAFETY: flock borrows a live owned file; the lock lives with this open description.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
fn managed_name(name: &str) -> bool {
    let Some(name) = name
        .strip_prefix("scancraft-capture-")
        .and_then(|s| s.strip_suffix(".log"))
    else {
        return false;
    };
    let parts = name.split('-').collect::<Vec<_>>();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}
fn prune(directory: &File, path: &Path, uid: u32) -> io::Result<()> {
    let mut candidates = Vec::new();
    for (i, entry) in fs::read_dir(path)?.enumerate() {
        if i == MAX_ENTRIES {
            return Err(invalid("diagnostic directory exceeds its inventory bound"));
        }
        let entry = entry?;
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if !managed_name(&name) || !entry.file_type()?.is_file() {
            continue;
        }
        let metadata = entry.metadata()?;
        if !metadata.is_file()
            || metadata.uid() != uid
            || metadata.mode() & 0o777 != 0o600
            || metadata.nlink() != 1
        {
            continue;
        }
        candidates.push(((metadata.mtime(), metadata.mtime_nsec()), name));
    }
    candidates.sort_unstable_by(|a, b| b.cmp(a));
    let mut inactive = 0;
    for (_, name) in candidates {
        let mut file = match open_at(directory, &name, libc::O_RDWR) {
            Ok(file) => file,
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::InvalidInput
                ) =>
            {
                continue;
            }
            Err(e) if e.raw_os_error() == Some(libc::ELOOP) => continue,
            Err(e) => return Err(e),
        };
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.uid() != uid
            || metadata.mode() & 0o777 != 0o600
            || metadata.nlink() != 1
        {
            continue;
        }
        match lock(&file) {
            Ok(()) => (),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
            Err(e) => return Err(e),
        }
        let mut header = Vec::new();
        BufReader::new(&mut file)
            .take(513)
            .read_until(b'\n', &mut header)?;
        if header.len() > 512 {
            continue;
        }
        let Ok(header) = serde_json::from_slice::<serde_json::Value>(&header) else {
            continue;
        };
        if header["kind"] != "scancraft_helper_diagnostic" || header["schema_version"] != 1 {
            continue;
        }
        inactive += 1;
        if inactive <= KEEP_INACTIVE {
            continue;
        }
        let name = leaf(&name)?;
        // SAFETY: live retained directory and validated managed leaf; symlinks,
        // active locked files and unrecognized headers were rejected above.
        if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
            let e = io::Error::last_os_error();
            if e.kind() != io::ErrorKind::NotFound {
                return Err(e);
            }
        }
    }
    Ok(())
}
fn create(root: &Path) -> io::Result<(PathBuf, File)> {
    if !root.is_absolute() {
        return Err(invalid("helper state directory must be absolute"));
    }
    DirBuilder::new().recursive(true).mode(0o700).create(root)?;
    let state = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root)?;
    // SAFETY: geteuid has no preconditions.
    let uid = unsafe { libc::geteuid() };
    check_directory(&state, uid, false)?;
    let app = child_directory(&state, "scancraft", uid, false)?;
    let directory = child_directory(&app, "diagnostics", uid, true)?;
    let path = root.join("scancraft/diagnostics");
    prune(&directory, &path, uid)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let name = format!(
        "scancraft-capture-{}-{stamp}-{}.log",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut file = open_at(
        &directory,
        &name,
        libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_APPEND,
    )?;
    lock(&file)?;
    writeln!(
        file,
        "{}",
        serde_json::json!({"schema_version":1,"kind":"scancraft_helper_diagnostic","supervisor_pid":std::process::id()})
    )?;
    file.sync_all()?;
    directory.sync_all()?;
    let current = fs::symlink_metadata(&path)?;
    let held = directory.metadata()?;
    if !current.is_dir() || (current.dev(), current.ino()) != (held.dev(), held.ino()) {
        return Err(invalid("diagnostic directory changed during setup"));
    }
    Ok((path.join(name), file))
}
pub(crate) fn private_log() -> io::Result<(PathBuf, File)> {
    let root = state_root(std::env::var_os("XDG_STATE_HOME"), std::env::var_os("HOME"))?;
    create(&root).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "cannot prepare private helper diagnostics under {}: {e}; helper not launched",
                root.display()
            ),
        )
    })
}
#[cfg(test)]
pub(crate) fn test_log(root: &Path) -> io::Result<(PathBuf, File)> {
    create(root)
}
pub(crate) fn failure(
    child: &mut Child,
    label: &str,
    log: &Path,
    mut observation: &File,
    reason: &str,
) -> String {
    let status = match child.try_wait() {
        Ok(Some(status)) => format!("exited {status}"),
        Ok(None) => "still running; cleanup/restoration is NOT confirmed".into(),
        Err(e) => format!("status unavailable: {e}; cleanup/restoration is NOT confirmed"),
    };
    let available = match fs::symlink_metadata(log) {
        Ok(m) if m.is_file() => "file present".to_owned(),
        Ok(_) => "unavailable: not a regular file".to_owned(),
        Err(e) => format!("unavailable: {e}"),
    };
    // Keep the observed status even after the GUI closes. The inherited append
    // description avoids reopening/replacing a diagnostic path on this error path.
    // This is best effort; its I/O failure cannot replace the required error.
    let prefix = |text: &str, maximum: usize| {
        let mut end = text.len().min(maximum);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text[..end].to_owned()
    };
    let record = serde_json::json!({"schema_version":1,"kind":"supervisor_failure",
        "helper_pid":child.id(),"label":prefix(label,32),"status":prefix(&status,128),
        "reason":prefix(reason,384),"reason_truncated":reason.len()>384});
    let recorded = writeln!(observation, "{record}").err();
    let note = recorded
        .map(|e| format!("; supervisor observation could not be written: {e}"))
        .unwrap_or_default();
    format!(
        "{reason}; {label} PID {} status now: {status}; diagnostics: {} ({available}){note}",
        child.id(),
        log.display()
    )
}
#[cfg(test)]
mod tests;
