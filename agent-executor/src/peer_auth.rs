use std::os::fd::AsRawFd;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeerCredentials {
    pub uid: u32,
    pub gid: u32,
    pub pid: Option<i32>,
}

#[derive(Clone, Copy, Debug)]
pub struct PeerPolicy {
    pub allowed_uid: u32,
    pub allowed_gid: u32,
}

impl PeerPolicy {
    pub fn authorizes(self, credentials: PeerCredentials) -> bool {
        credentials.uid == self.allowed_uid && credentials.gid == self.allowed_gid
    }
}

#[cfg(any(target_os = "macos", target_os = "freebsd"))]
pub fn credentials(stream: &tokio::net::UnixStream) -> std::io::Result<PeerCredentials> {
    let mut uid = 0;
    let mut gid = 0;
    // SAFETY: getpeereid only writes the supplied uid/gid values for a valid socket fd.
    let result = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(PeerCredentials {
        uid,
        gid,
        pid: None,
    })
}

#[cfg(target_os = "linux")]
pub fn credentials(stream: &tokio::net::UnixStream) -> std::io::Result<PeerCredentials> {
    let mut value: libc::ucred = unsafe { std::mem::zeroed() };
    let mut length = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: getsockopt writes at most `length` bytes to a correctly sized ucred value.
    let result = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut value as *mut libc::ucred).cast(),
            &mut length,
        )
    };
    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(PeerCredentials {
        uid: value.uid,
        gid: value.gid,
        pid: Some(value.pid),
    })
}

#[cfg(target_os = "linux")]
pub fn executable_is(peer: PeerCredentials, expected: &std::path::Path) -> bool {
    peer.pid.is_some_and(|pid| {
        std::fs::read_link(format!("/proc/{pid}/exe"))
            .ok()
            .is_some_and(|path| path == expected)
    })
}

#[cfg(not(target_os = "linux"))]
pub fn executable_is(_peer: PeerCredentials, _expected: &std::path::Path) -> bool {
    true
}
