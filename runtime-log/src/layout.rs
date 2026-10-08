use std::{
    fs, io,
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::Path,
};

use crate::{COMPONENTS, NODE_LOG_ROOT};

/// 由安装器或 root Broker 创建固定新命名空间，不遍历或清理历史任务日志。
pub fn prepare_node_layout() -> io::Result<()> {
    if unsafe { libc::geteuid() } != 0 {
        return Err(io::Error::other("runtime_log_root_required"));
    }
    let user = unsafe { libc::getpwnam(c"deploy-go-agent".as_ptr()) };
    if user.is_null() {
        return Err(io::Error::other("runtime_log_identity_missing"));
    }
    let (uid, gid) = unsafe { ((*user).pw_uid, (*user).pw_gid) };
    let root = Path::new(NODE_LOG_ROOT);
    for parent in root.ancestors().skip(1) {
        let metadata = fs::symlink_metadata(parent)?;
        if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o022 != 0 {
            return Err(io::Error::other("runtime_log_parent_invalid"));
        }
    }
    prepare(root, 0, gid, 0o750)?;
    for component in COMPONENTS {
        let (owner, mode) = if component == "agent" {
            (uid, 0o700)
        } else {
            (0, 0o750)
        };
        prepare(&root.join(component), owner, gid, mode)?;
    }
    Ok(())
}

fn prepare(path: &Path, uid: u32, gid: u32, mode: u32) -> io::Result<()> {
    match fs::DirBuilder::new().mode(mode).create(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    let directory = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open(path)?;
    let metadata = directory.metadata()?;
    if metadata.uid() != 0 && metadata.uid() != uid {
        return Err(io::Error::other("runtime_log_owner_invalid"));
    }
    if (metadata.uid() != uid || metadata.gid() != gid)
        && unsafe { libc::fchown(directory.as_raw_fd(), uid, gid) } != 0
    {
        return Err(io::Error::last_os_error());
    }
    if metadata.mode() & 0o7777 != mode {
        directory.set_permissions(fs::Permissions::from_mode(mode))?;
    }
    Ok(())
}
