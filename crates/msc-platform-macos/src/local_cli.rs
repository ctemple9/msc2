//! Kernel-provided identity for the peer of an accepted local CLI socket.

use std::os::fd::{AsRawFd, RawFd};

pub const SOCKET_NAME: &str = "local-cli.sock";

pub fn peer_uid(stream: &impl AsRawFd) -> Result<u32, String> {
    peer_uid_from_fd(stream.as_raw_fd())
}

fn peer_uid_from_fd(fd: RawFd) -> Result<u32, String> {
    let mut uid = 0 as libc::uid_t;
    let mut gid = 0 as libc::gid_t;
    // getpeereid reads the credentials attached by the kernel to this socket.
    let result = unsafe { libc::getpeereid(fd, &mut uid, &mut gid) };
    if result != 0 {
        return Err(format!(
            "reading local CLI peer credentials: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(uid as u32)
}

pub fn service_uid() -> u32 {
    // The LaunchDaemon runs as the installing user, not root.
    unsafe { libc::geteuid() as u32 }
}
