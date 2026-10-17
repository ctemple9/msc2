//! Unix descriptor handoff. No source path is accepted or reconstructed.
use std::fs::File;
use std::io;
use std::mem::{size_of, zeroed};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::time::SystemTime;

pub const SOCKET_NAME: &str = "local-source-v1.sock";

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Identity {
    device: u64,
    inode: u64,
    pub size: u64,
    modified: SystemTime,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}
impl Identity {
    pub fn read(file: &File) -> io::Result<Self> {
        let meta = file.metadata()?;
        // The receiver must never accept a writable capability or a device.
        let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFL) };
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        if !meta.is_file() || flags & libc::O_ACCMODE != libc::O_RDONLY {
            return Err(io::Error::other(
                "handoff requires a read-only regular file",
            ));
        }
        Ok(Self {
            device: meta.dev(),
            inode: meta.ino(),
            size: meta.len(),
            modified: meta.modified()?,
            changed_seconds: meta.ctime(),
            changed_nanoseconds: meta.ctime_nsec(),
        })
    }
    pub fn validate(&self, file: &File) -> io::Result<()> {
        if &Self::read(file)? != self {
            return Err(io::Error::other("selected source changed"));
        }
        Ok(())
    }
}

/// A one-byte marker carries exactly one descriptor. Call with socket readiness
/// and retry WouldBlock; metadata is framed separately before this marker.
pub fn send_file(socket: &impl AsRawFd, file: &File) -> io::Result<()> {
    let mut marker = [1u8];
    let mut vector = libc::iovec {
        iov_base: marker.as_mut_ptr().cast(),
        iov_len: 1,
    };
    // usize storage provides the alignment required by cmsghdr.
    let space = unsafe { libc::CMSG_SPACE(size_of::<libc::c_int>() as _) } as usize;
    let mut control = vec![0usize; space.div_ceil(size_of::<usize>())];
    // SAFETY: zero is the empty msghdr representation; all referenced buffers
    // below live through sendmsg and their lengths describe those allocations.
    let mut message: libc::msghdr = unsafe { zeroed() };
    message.msg_iov = &mut vector;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = space as _;
    unsafe {
        let header = libc::CMSG_FIRSTHDR(&message);
        (*header).cmsg_level = libc::SOL_SOCKET;
        (*header).cmsg_type = libc::SCM_RIGHTS;
        (*header).cmsg_len = libc::CMSG_LEN(size_of::<libc::c_int>() as _) as _;
        std::ptr::write_unaligned(
            libc::CMSG_DATA(header).cast::<libc::c_int>(),
            file.as_raw_fd(),
        );
    }
    #[cfg(target_os = "linux")]
    let flags = libc::MSG_NOSIGNAL;
    #[cfg(not(target_os = "linux"))]
    let flags = 0; // Darwin's UnixStream enables SO_NOSIGPIPE.
    let sent = unsafe { libc::sendmsg(socket.as_raw_fd(), &message, flags) };
    if sent == 1 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

pub fn receive_file(socket: &impl AsRawFd) -> io::Result<File> {
    let mut marker = [0u8];
    let mut vector = libc::iovec {
        iov_base: marker.as_mut_ptr().cast(),
        iov_len: 1,
    };
    let space = unsafe { libc::CMSG_SPACE(size_of::<[libc::c_int; 8]>() as _) } as usize;
    let mut control = vec![0usize; space.div_ceil(size_of::<usize>())];
    let mut message: libc::msghdr = unsafe { zeroed() };
    message.msg_iov = &mut vector;
    message.msg_iovlen = 1;
    message.msg_control = control.as_mut_ptr().cast();
    message.msg_controllen = space as _;
    #[cfg(target_os = "linux")]
    let flags = libc::MSG_CMSG_CLOEXEC;
    #[cfg(not(target_os = "linux"))]
    let flags = 0;
    // SAFETY: recvmsg writes only within the aligned buffers described above.
    let received = unsafe { libc::recvmsg(socket.as_raw_fd(), &mut message, flags) };
    if received < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut files = Vec::new();
    unsafe {
        let mut header = libc::CMSG_FIRSTHDR(&message);
        while !header.is_null() {
            if (*header).cmsg_level == libc::SOL_SOCKET && (*header).cmsg_type == libc::SCM_RIGHTS {
                let bytes =
                    ((*header).cmsg_len as usize).saturating_sub(libc::CMSG_LEN(0) as usize);
                for index in 0..bytes / size_of::<libc::c_int>() {
                    let fd = std::ptr::read_unaligned(
                        libc::CMSG_DATA(header).cast::<libc::c_int>().add(index),
                    );
                    // Every received descriptor is owned here, including on rejection.
                    files.push(File::from_raw_fd(fd));
                }
            }
            header = libc::CMSG_NXTHDR(&message, header);
        }
    }
    if received != 1
        || marker[0] != 1
        || files.len() != 1
        || message.msg_flags & libc::MSG_CTRUNC != 0
    {
        return Err(io::Error::other("invalid source descriptor handoff"));
    }
    let file = files.pop().expect("one descriptor checked");
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Identity::read(&file)?;
    Ok(file)
}
