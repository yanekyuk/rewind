use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImmutabilityError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(target_os = "linux")]
    #[error("ioctl error: {0}")]
    Ioctl(String),
}

/// Make a file read-only / immutable so Steam cannot overwrite it.
pub fn lock_file(path: &Path) -> Result<(), ImmutabilityError> {
    platform_lock(path, true)
}

/// Remove the read-only / immutable attribute.
pub fn unlock_file(path: &Path) -> Result<(), ImmutabilityError> {
    platform_lock(path, false)
}

/// Returns true if the file is currently read-only.
pub fn is_locked(path: &Path) -> Result<bool, ImmutabilityError> {
    let meta = std::fs::metadata(path)?;
    Ok(meta.permissions().readonly())
}

#[cfg(target_os = "linux")]
fn platform_lock(path: &Path, lock: bool) -> Result<(), ImmutabilityError> {
    if lock {
        // Try ioctl first; fall back to chmod if it fails (e.g. no root).
        match set_linux_immutable(path, true) {
            Ok(()) => Ok(()),
            Err(_) => set_readonly_std(path, true),
        }
    } else {
        // Best-effort ioctl to clear the immutable flag; ignore errors.
        // Always follow up with chmod to restore write bits in case the lock
        // was applied via chmod rather than ioctl.
        let _ = set_linux_immutable(path, false);
        set_readonly_std(path, false)
    }
}

#[cfg(target_os = "linux")]
fn set_linux_immutable(path: &Path, immutable: bool) -> Result<(), ImmutabilityError> {
    use std::os::unix::io::AsRawFd;

    const FS_IMMUTABLE_FL: u32 = 0x0000_0010;
    const FS_IOC_GETFLAGS: u64 = 0x8008_6601;
    const FS_IOC_SETFLAGS: u64 = 0x4008_6602;

    let file = std::fs::OpenOptions::new().read(true).open(path)?;
    let fd = file.as_raw_fd();

    let mut flags: u32 = 0;
    unsafe {
        if libc::ioctl(fd, FS_IOC_GETFLAGS, &mut flags as *mut u32) != 0 {
            return Err(ImmutabilityError::Ioctl(
                std::io::Error::last_os_error().to_string(),
            ));
        }
        if immutable {
            flags |= FS_IMMUTABLE_FL;
        } else {
            flags &= !FS_IMMUTABLE_FL;
        }
        if libc::ioctl(fd, FS_IOC_SETFLAGS, &flags as *const u32) != 0 {
            return Err(ImmutabilityError::Ioctl(
                std::io::Error::last_os_error().to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn platform_lock(path: &Path, lock: bool) -> Result<(), ImmutabilityError> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    // UF_IMMUTABLE = 0x00000002
    let flag: u32 = if lock { 0x0000_0002 } else { 0 };
    let c_path = CString::new(path.as_os_str().as_bytes()).map_err(|e| {
        ImmutabilityError::Io(std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
    })?;

    let ret = unsafe { libc::chflags(c_path.as_ptr(), flag) };
    if ret != 0 {
        return Err(ImmutabilityError::Io(std::io::Error::last_os_error()));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn platform_lock(path: &Path, lock: bool) -> Result<(), ImmutabilityError> {
    set_readonly_std(path, lock)
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn set_readonly_std(path: &Path, readonly: bool) -> Result<(), ImmutabilityError> {
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_readonly(readonly);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn lock_and_unlock_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "test content").unwrap();
        let path = file.path();

        lock_file(path).expect("lock should succeed");
        let meta = std::fs::metadata(path).unwrap();
        assert!(meta.permissions().readonly(), "file should be read-only after lock");

        unlock_file(path).expect("unlock should succeed");
        let meta = std::fs::metadata(path).unwrap();
        assert!(!meta.permissions().readonly(), "file should be writable after unlock");
    }

    #[test]
    fn is_locked_reflects_state() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "test").unwrap();
        let path = file.path();

        assert!(!is_locked(path).unwrap());
        lock_file(path).unwrap();
        assert!(is_locked(path).unwrap());
        unlock_file(path).unwrap();
        assert!(!is_locked(path).unwrap());
    }
}
