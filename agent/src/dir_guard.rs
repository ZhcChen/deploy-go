use std::{
    fs,
    io::{self, ErrorKind},
    path::Path,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// 确保目录存在且权限属于允许集合。
///
/// 若目录已经以允许的 setgid 权限存在（例如由父目录继承），不再执行
/// chmod；systemd 的 RestrictSUIDSGID 会阻止对已有 setgid 位重复设置。
pub fn ensure_directory_mode(
    path: &Path,
    desired_mode: u32,
    allowed_modes: &[u32],
) -> io::Result<()> {
    if !path.exists() {
        create_directory(path, desired_mode)?;
    }
    #[cfg(unix)]
    {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "unsafe journal directory",
            ));
        }
        let mut mode = metadata.permissions().mode() & 0o7777;
        if !allowed_modes.contains(&mode) {
            fs::set_permissions(path, fs::Permissions::from_mode(desired_mode))?;
            mode = fs::symlink_metadata(path)?.permissions().mode() & 0o7777;
        }
        if !allowed_modes.contains(&mode) {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "unsafe task journal directory",
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn create_directory(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }
    }
    let mut builder = fs::DirBuilder::new();
    // 直接以目标权限创建，避免 systemd RestrictSUIDSGID 下对 setgid
    // 位重复 chmod 被 seccomp 拒绝。RestrictSUIDSGID 同样会拒绝显式
    // 带 S_ISGID 的 mkdir，因此创建时去掉 setgid 位，由 setgid 父目录
    // 自动继承；若未继承再由后续 chmod 修正。
    builder.mode(mode & !0o2000);
    builder.create(path)
}

#[cfg(not(unix))]
fn create_directory(path: &Path, _mode: u32) -> io::Result<()> {
    fs::create_dir_all(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[cfg(unix)]
    #[test]
    fn accepts_already_allowed_mode() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("task");
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o3700)).unwrap();

        ensure_directory_mode(&path, 0o3700, &[0o3700, 0o3770]).unwrap();

        let mode = fs::symlink_metadata(&path).unwrap().permissions().mode() & 0o7777;
        assert_eq!(mode, 0o3700);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn allowed_setgid_mode_does_not_require_chmod_under_foreign_owner() {
        use std::ffi::CString;

        if unsafe { libc::geteuid() } != 0 {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("task");
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o3700)).unwrap();

        let cpath = CString::new(path.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::chown(cpath.as_ptr(), 12345, 12345) }, 0);

        // 若实现仍尝试 chmod，foreign owner 会返回 EPERM；这里必须直接通过。
        ensure_directory_mode(&path, 0o3700, &[0o3700, 0o3770]).unwrap();

        assert_eq!(
            unsafe { libc::chown(cpath.as_ptr(), libc::geteuid(), libc::getegid()) },
            0
        );
    }

    #[cfg(unix)]
    #[test]
    fn creates_new_directory_with_requested_mode() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("task");

        ensure_directory_mode(&path, 0o3700, &[0o3700, 0o3770]).unwrap();

        let mode = fs::symlink_metadata(&path).unwrap().permissions().mode() & 0o7777;
        assert!(mode == 0o3700 || mode == 0o3770, "mode was {mode:o}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn setgid_parent_inherits_task_mode_without_explicit_sgid_mkdir() {
        let directory = tempfile::tempdir().unwrap();
        let parent = directory.path().join("tasks");
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o3710)).unwrap();
        let path = parent.join("task");

        ensure_directory_mode(&path, 0o3700, &[0o3700, 0o3770]).unwrap();

        let mode = fs::symlink_metadata(&path).unwrap().permissions().mode() & 0o7777;
        assert!(mode == 0o3700 || mode == 0o3770, "mode was {mode:o}");
    }
}
