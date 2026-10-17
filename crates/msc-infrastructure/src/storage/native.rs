use std::{
    fs::File,
    io,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Identity(pub u64, pub u128);

pub(super) struct Measurement {
    pub identity: Identity,
    pub mount: u64,
    pub logical: u64,
    pub allocated: Option<u64>,
}

// Resolve each component relative to its retained parent, never a canonicalize-
// then-open pair. A substituted ancestor or final link cannot escape the root.
pub(super) fn open(path: &Path) -> io::Result<File> {
    if !path.is_absolute() || path.as_os_str().len() > 4096 {
        return Err(io::Error::other(
            "storage root must be a bounded absolute path",
        ));
    }
    let mut prefix = PathBuf::new();
    let mut handle = None;
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        match component {
            Component::Prefix(_) => prefix.push(component.as_os_str()),
            Component::RootDir => {
                prefix.push(component.as_os_str());
                handle = Some(crate::source_tree::open(&prefix, true)?);
            }
            Component::Normal(name) => {
                let text = name
                    .to_str()
                    .ok_or_else(|| io::Error::other("non-text storage root"))?;
                prefix.push(name);
                let directory =
                    components.peek().is_some() || std::fs::symlink_metadata(&prefix)?.is_dir();
                handle = Some(crate::source_tree::storage_child(
                    handle
                        .as_ref()
                        .ok_or_else(|| io::Error::other("missing volume root"))?,
                    text,
                    directory,
                )?);
            }
            _ => {
                return Err(io::Error::other(
                    "storage root contains relative components",
                ));
            }
        }
    }
    handle.ok_or_else(|| io::Error::other("missing storage root"))
}

pub(super) fn measure(file: &File) -> io::Result<Measurement> {
    let metadata = file.metadata()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(Measurement {
            identity: Identity(metadata.dev(), u128::from(metadata.ino())),
            mount: mount_id(file, metadata.dev())?,
            logical: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            allocated: metadata.blocks().checked_mul(512),
        })
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_ID_INFO, FILE_STANDARD_INFO, FileIdInfo, FileStandardInfo,
            GetFileInformationByHandleEx,
        };
        let mut info: FILE_ID_INFO = unsafe { std::mem::zeroed() };
        if unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle().cast(),
                FileIdInfo,
                (&mut info as *mut FILE_ID_INFO).cast(),
                std::mem::size_of::<FILE_ID_INFO>() as u32,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        let mut standard: FILE_STANDARD_INFO = unsafe { std::mem::zeroed() };
        let allocated = if unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle().cast(),
                FileStandardInfo,
                (&mut standard as *mut FILE_STANDARD_INFO).cast(),
                std::mem::size_of::<FILE_STANDARD_INFO>() as u32,
            )
        } != 0
        {
            u64::try_from(standard.AllocationSize).ok()
        } else {
            None
        };
        Ok(Measurement {
            identity: Identity(
                info.VolumeSerialNumber,
                u128::from_le_bytes(info.FileId.Identifier),
            ),
            logical: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            mount: info.VolumeSerialNumber,
            allocated,
        })
    }
}

#[cfg(unix)]
fn mount_id(file: &File, device: u64) -> io::Result<u64> {
    #[cfg(target_os = "linux")]
    {
        use std::os::fd::AsRawFd;
        let _ = device;
        let mut info: libc::statx = unsafe { std::mem::zeroed() };
        // SAFETY: AT_EMPTY_PATH queries this retained file descriptor only.
        if unsafe {
            libc::statx(
                file.as_raw_fd(),
                c"".as_ptr(),
                libc::AT_EMPTY_PATH,
                libc::STATX_MNT_ID,
                &mut info,
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        if info.stx_mask & libc::STATX_MNT_ID == 0 {
            return Err(io::Error::other("filesystem mount identity unavailable"));
        }
        Ok(info.stx_mnt_id)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = file;
        Ok(device)
    }
}
