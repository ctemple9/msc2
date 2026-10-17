//! Bounded, read-only traversal of a selected server tree. Every child is
//! opened without following links; directory handles stay alive during descent.
#[cfg(windows)]
#[path = "source_tree/windows.rs"]
mod windows;
use crate::archive::{MAX_ARCHIVE_ENTRIES, MAX_TOTAL_UNCOMPRESSED_BYTES};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub entries: u64,
    pub bytes: u64,
    pub archive_bytes: u64,
    digest: [u8; 32],
}

#[derive(Debug)]
pub struct UnsafeLink;
impl std::fmt::Display for UnsafeLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("source contains a symbolic link or reparse point")
    }
}
impl std::error::Error for UnsafeLink {}
fn link_error() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, UnsafeLink)
}
fn invalid(message: &str) -> io::Error {
    io::Error::other(message)
}
pub fn open(path: &Path, directory: bool) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(
            libc::O_NOFOLLOW | libc::O_NONBLOCK | if directory { libc::O_DIRECTORY } else { 0 },
        );
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ,
        };
        options
            .custom_flags(
                FILE_FLAG_OPEN_REPARSE_POINT
                    | if directory {
                        FILE_FLAG_BACKUP_SEMANTICS
                    } else {
                        0
                    },
            )
            .share_mode(FILE_SHARE_READ);
    }
    let opened = options.open(path);
    #[cfg(unix)]
    let opened = opened.map_err(|error| {
        if error.raw_os_error() == Some(libc::ELOOP) {
            link_error()
        } else {
            error
        }
    });
    let file = opened?;
    check_kind(&file, directory)?;
    Ok(file)
}
fn check_kind(file: &File, directory: bool) -> io::Result<()> {
    let meta = file.metadata()?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            return Err(link_error());
        }
    }
    if if directory {
        !meta.is_dir()
    } else {
        !meta.is_file()
    } {
        return Err(invalid(
            "source contains a link, device or unsupported entry",
        ));
    }
    Ok(())
}
pub fn fingerprint(file: &File) -> io::Result<Vec<u8>> {
    let meta = file.metadata()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            meta.dev(),
            meta.ino(),
            meta.len(),
            meta.mtime(),
            meta.mtime_nsec(),
            meta.ctime(),
            meta.ctime_nsec(),
            meta.mode()
        )
        .into_bytes())
    }
    #[cfg(windows)]
    {
        use std::os::windows::{fs::MetadataExt, io::AsRawHandle};
        use windows_sys::Win32::Storage::FileSystem::{
            BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
        };
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        if unsafe { GetFileInformationByHandle(file.as_raw_handle().cast(), &mut info) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(format!(
            "{}:{}:{}:{}:{}:{}:{}",
            info.dwVolumeSerialNumber,
            info.nFileIndexHigh,
            info.nFileIndexLow,
            meta.file_size(),
            meta.last_write_time(),
            meta.creation_time(),
            meta.file_attributes()
        )
        .into_bytes())
    }
}

#[cfg(unix)]
fn children(
    root: &File,
    mut visit: impl FnMut(&str, File, bool) -> io::Result<()>,
) -> io::Result<()> {
    use std::{
        ffi::CStr,
        os::fd::{AsRawFd, FromRawFd},
    };
    struct Directory(*mut libc::DIR);
    impl Drop for Directory {
        fn drop(&mut self) {
            unsafe {
                libc::closedir(self.0);
            }
        }
    }
    let fd = unsafe {
        libc::openat(
            root.as_raw_fd(),
            c".".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let pointer = unsafe { libc::fdopendir(fd) };
    if pointer.is_null() {
        unsafe {
            libc::close(fd);
        }
        return Err(io::Error::last_os_error());
    }
    let directory = Directory(pointer);
    loop {
        // readdir's null result is EOF only when errno is zero.
        #[cfg(target_os = "macos")]
        unsafe {
            *libc::__error() = 0;
        }
        #[cfg(target_os = "linux")]
        unsafe {
            *libc::__errno_location() = 0;
        }
        let entry = unsafe { libc::readdir(directory.0) };
        if entry.is_null() {
            let error = io::Error::last_os_error();
            return if error.raw_os_error() == Some(0) {
                Ok(())
            } else {
                Err(error)
            };
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name.to_bytes() == b"." || name.to_bytes() == b".." {
            continue;
        }
        let text = name
            .to_str()
            .map_err(|_| invalid("source names must be valid text"))?;
        portable_name(text)?;
        let fd = unsafe {
            libc::openat(
                root.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            let error = io::Error::last_os_error();
            return Err(if error.raw_os_error() == Some(libc::ELOOP) {
                link_error()
            } else {
                error
            });
        }
        let file = unsafe { File::from_raw_fd(fd) };
        let is_dir = file.metadata()?.is_dir();
        check_kind(&file, is_dir)?;
        visit(text, file, is_dir)?;
    }
}
fn portable_name(name: &str) -> io::Result<()> {
    if name.is_empty()
        || name.len() > 255
        || name.contains(['/', '\\', ':', '\0', '<', '>', '"', '|', '?', '*'])
        || name.ends_with([' ', '.'])
        || name.chars().any(|c| c.is_control())
    {
        return Err(invalid("source contains a nonportable filename"));
    }
    let base = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
    {
        return Err(invalid("source contains a reserved filename"));
    }
    Ok(())
}

pub fn walk(
    root: &File,
    root_path: &Path,
    cancelled: &dyn Fn() -> bool,
    visitor: &mut dyn FnMut(&Path, &mut File, bool) -> io::Result<()>,
) -> io::Result<Snapshot> {
    fn descend(
        file: &File,
        absolute: &Path,
        relative: &Path,
        depth: usize,
        snapshot: &mut Snapshot,
        cancelled: &dyn Fn() -> bool,
        visitor: &mut dyn FnMut(&Path, &mut File, bool) -> io::Result<()>,
    ) -> io::Result<()> {
        if depth > 64 || cancelled() {
            return Err(invalid("source traversal cancelled or too deep"));
        }
        let before = fingerprint(file)?;
        let mut visit = |name: &str, mut child: File, directory: bool| -> io::Result<()> {
            if cancelled() {
                return Err(invalid("source traversal cancelled"));
            }
            snapshot.entries += 1;
            if snapshot.entries > MAX_ARCHIVE_ENTRIES {
                return Err(invalid("source has too many entries"));
            }
            let path = relative.join(name);
            if path.as_os_str().len() > 4096 {
                return Err(invalid("source path is too long"));
            }
            let child_before = fingerprint(&child)?;
            let meta = child.metadata()?;
            if !directory {
                snapshot.bytes = snapshot
                    .bytes
                    .checked_add(meta.len())
                    .filter(|n| *n <= MAX_TOTAL_UNCOMPRESSED_BYTES)
                    .ok_or_else(|| invalid("source is too large"))?;
            }
            let text = path
                .to_str()
                .ok_or_else(|| invalid("source path is not text"))?;
            let mut header = tar::Header::new_gnu();
            let long_name = header.set_path(&path).is_err();
            let extra = if long_name {
                512 + (text.len() as u64 + 1).div_ceil(512) * 512
            } else {
                0
            };
            snapshot.archive_bytes = snapshot
                .archive_bytes
                .checked_add(
                    512 + extra
                        + if directory {
                            0
                        } else {
                            meta.len().div_ceil(512) * 512
                        },
                )
                .ok_or_else(|| invalid("archive size overflow"))?;
            let mut hash = Sha256::new();
            hash.update(
                path.to_str()
                    .ok_or_else(|| invalid("source path is not text"))?
                    .as_bytes(),
            );
            hash.update([u8::from(directory)]);
            hash.update(&child_before);
            let value = hash.finalize();
            for (dest, value) in snapshot.digest.iter_mut().zip(value) {
                *dest ^= value;
            }
            visitor(&path, &mut child, directory)?;
            if directory {
                descend(
                    &child,
                    &absolute.join(name),
                    &path,
                    depth + 1,
                    snapshot,
                    cancelled,
                    visitor,
                )?;
            }
            if fingerprint(&child)? != child_before {
                return Err(invalid("source changed during traversal"));
            }
            Ok(())
        };
        #[cfg(unix)]
        children(file, &mut visit)?;
        #[cfg(windows)]
        windows::children(file, &mut visit)?;
        if fingerprint(file)? != before {
            return Err(invalid("source directory changed"));
        }
        Ok(())
    }
    check_kind(root, true)?;
    let mut snapshot = Snapshot {
        entries: 0,
        bytes: 0,
        archive_bytes: 1024,
        digest: [0; 32],
    };
    descend(
        root,
        root_path,
        Path::new(""),
        0,
        &mut snapshot,
        cancelled,
        visitor,
    )?;
    Ok(snapshot)
}

pub fn snapshot(root: &File, path: &Path, cancelled: &dyn Fn() -> bool) -> io::Result<Snapshot> {
    walk(root, path, cancelled, &mut |path, file, directory| {
        if directory {
            Ok(())
        } else {
            stopped(file, path)
        }
    })
}

/// Probe live-world locks using the same kernel record-lock mechanism as Java
/// and LevelDB. An inaccessible or held lock is a refusal, never a guessed stop.
fn stopped(file: &File, path: &Path) -> io::Result<()> {
    if !matches!(
        path.file_name().and_then(|s| s.to_str()),
        Some("session.lock" | "LOCK")
    ) {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        let mut lock: libc::flock = unsafe { std::mem::zeroed() };
        lock.l_type = libc::F_RDLCK as _;
        lock.l_whence = libc::SEEK_SET as _;
        if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_SETLK, &lock) } < 0 {
            return Err(invalid(
                "Stop the source Minecraft server before importing its folder.",
            ));
        }
    }
    #[cfg(windows)]
    {
        let _ = file;
    } // Read-only sharing already refuses live write handles.
    Ok(())
}

pub fn copy(
    root: &File,
    path: &Path,
    destination: &Path,
    expected: &Snapshot,
    cancelled: &dyn Fn() -> bool,
    block: &mut dyn FnMut(&[u8]) -> io::Result<()>,
) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let source = root.metadata()?;
        for ancestor in destination.ancestors() {
            if let Ok(meta) = ancestor.metadata()
                && meta.dev() == source.dev()
                && meta.ino() == source.ino()
            {
                return Err(invalid("destination staging is inside the selected source"));
            }
        }
    }
    #[cfg(windows)]
    {
        if destination
            .canonicalize()?
            .starts_with(path.canonicalize()?)
        {
            return Err(invalid("destination staging is inside the selected source"));
        }
    }
    let actual = walk(root, path, cancelled, &mut |relative, file, directory| {
        let target = destination.join(relative);
        if directory {
            std::fs::create_dir(&target)?;
        } else {
            stopped(file, relative)?;
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(file.metadata()?.permissions().mode() & 0o777);
            }
            let mut output = options.open(&target)?;
            let mut bytes = vec![0; 256 * 1024];
            let mut remaining = file.metadata()?.len();
            while remaining > 0 {
                if cancelled() {
                    return Err(invalid("source copy cancelled"));
                }
                let length = remaining.min(bytes.len() as u64) as usize;
                file.read_exact(&mut bytes[..length])?;
                output.write_all(&bytes[..length])?;
                block(&bytes[..length])?;
                remaining -= length as u64;
            }
            output.sync_all()?;
            output.set_permissions(file.metadata()?.permissions())?;
        }
        Ok(())
    })?;
    if &actual != expected || &snapshot(root, path, cancelled)? != expected {
        return Err(invalid("source tree changed during copy"));
    }
    #[cfg(unix)]
    File::open(destination)?.sync_all()?;
    Ok(())
}

pub fn archive<W: Write>(
    root: &File,
    path: &Path,
    destination: W,
    expected: &Snapshot,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<W> {
    if snapshot(root, path, cancelled)? != *expected {
        return Err(invalid("source changed before archive streaming"));
    }
    struct Bounded<W>(W, u64);
    impl<W: Write> Write for Bounded<W> {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self
                .1
                .checked_add(bytes.len() as u64)
                .is_none_or(|n| n > 10 * 1024 * 1024 * 1024)
            {
                return Err(invalid("prepared server archive exceeds staging capacity"));
            }
            let count = self.0.write(bytes)?;
            self.1 += count as u64;
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.0.flush()
        }
    }
    let mut tar = tar::Builder::new(Bounded(destination, 0));
    let actual = walk(root, path, cancelled, &mut |relative, file, directory| {
        if !directory {
            stopped(file, relative)?;
        }
        let mut header = tar::Header::new_gnu();
        header.set_metadata(&file.metadata()?);
        header.set_entry_type(if directory {
            tar::EntryType::Directory
        } else {
            tar::EntryType::Regular
        });
        header.set_size(if directory { 0 } else { file.metadata()?.len() });
        header.set_cksum();
        struct Cancellable<'a> {
            file: &'a mut File,
            cancel: &'a dyn Fn() -> bool,
        }
        impl Read for Cancellable<'_> {
            fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
                if (self.cancel)() {
                    return Err(invalid("archive preparation cancelled"));
                }
                self.file.read(b)
            }
        }
        tar.append_data(
            &mut header,
            relative,
            Cancellable {
                file,
                cancel: cancelled,
            },
        )?;
        Ok(())
    })?;
    if actual != *expected || snapshot(root, path, cancelled)? != *expected {
        return Err(invalid("source tree changed during archive preparation"));
    }
    tar.finish()?;
    let Bounded(writer, count) = tar.into_inner()?;
    if count != expected.archive_bytes {
        return Err(invalid("generated archive length changed"));
    }
    Ok(writer)
}

/// Generated folder TARs admit regular files and directories only. Reuse a
/// private destination; reject links, special files, duplicates and traversal.
pub fn extract_tar(
    source: &Path,
    destination: &Path,
    cancelled: &dyn Fn() -> bool,
) -> io::Result<()> {
    let ceiling = source.metadata()?.len();
    let mut archive = tar::Archive::new(File::open(source)?);
    let mut total = 0u64;
    let mut count = 0u64;
    let mut long_name: Option<PathBuf> = None;
    for entry in archive.entries()?.raw(true) {
        if cancelled() {
            return Err(invalid("folder extraction cancelled"));
        }
        let mut entry = entry?;
        let kind = entry.header().entry_type();
        if kind.is_gnu_longname() {
            if long_name.is_some() || entry.size() > 4097 {
                return Err(invalid("invalid extended archive name"));
            }
            let mut bytes = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut bytes)?;
            if bytes.pop() != Some(0) || bytes.contains(&0) {
                return Err(invalid("invalid extended archive name"));
            }
            long_name = Some(PathBuf::from(
                String::from_utf8(bytes).map_err(|_| invalid("archive path is not text"))?,
            ));
            continue;
        }
        count += 1;
        if count > MAX_ARCHIVE_ENTRIES {
            return Err(invalid("archive has too many entries"));
        }
        let relative = match long_name.take() {
            Some(path) => path,
            None => entry.path()?.into_owned(),
        };
        if relative.as_os_str().len() > 4096 {
            return Err(invalid("archive path is too long"));
        }
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(invalid("unsafe archive path"));
        }
        for component in &relative {
            portable_name(
                component
                    .to_str()
                    .ok_or_else(|| invalid("archive path is not text"))?,
            )?;
        }
        let target = destination.join(relative);
        if kind.is_dir() {
            std::fs::create_dir(&target)?;
        } else if kind.is_file() {
            total = total
                .checked_add(entry.size())
                .filter(|n| *n <= MAX_TOTAL_UNCOMPRESSED_BYTES && *n <= ceiling)
                .ok_or_else(|| invalid("archive exceeds size limit"))?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(entry.header().mode()? & 0o777);
            }
            let mut output = options.open(&target)?;
            let mut buffer = vec![0; 256 * 1024];
            loop {
                if cancelled() {
                    return Err(invalid("folder extraction cancelled"));
                }
                let count = entry.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                output.write_all(&buffer[..count])?;
            }
            output.sync_all()?;
        } else {
            return Err(invalid("archive contains a link or unsupported entry"));
        }
    }
    if long_name.is_some() {
        return Err(invalid("dangling extended archive name"));
    }
    Ok(())
}
