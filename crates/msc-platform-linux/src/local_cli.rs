//! Kernel-provided identity for the peer of an accepted local CLI socket.

use std::os::fd::{AsRawFd, RawFd};

pub const SOCKET_NAME: &str = "local-cli.sock";

pub fn peer_uid(stream: &impl AsRawFd) -> Result<u32, String> {
    peer_uid_from_fd(stream.as_raw_fd())
}

fn peer_uid_from_fd(fd: RawFd) -> Result<u32, String> {
    let mut cred = std::mem::MaybeUninit::<libc::ucred>::uninit();
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // The kernel writes a complete ucred into this correctly sized buffer.
    let result = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            cred.as_mut_ptr().cast(),
            &mut len,
        )
    };
    if result != 0 || len as usize != std::mem::size_of::<libc::ucred>() {
        return Err(format!(
            "reading local CLI peer credentials: {}",
            std::io::Error::last_os_error()
        ));
    }
    // getsockopt succeeded and reported the full kernel structure.
    Ok(unsafe { cred.assume_init().uid })
}

pub fn service_uid() -> u32 {
    // The agent service runs as the installing user, not the root helper.
    unsafe { libc::geteuid() }
}
