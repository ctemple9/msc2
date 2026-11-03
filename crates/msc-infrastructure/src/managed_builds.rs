//! Cross-process exclusion for staging/repair and reviewed old-build removal.
//! This is a service-install reference, not a competing upload registry.
use fs2::FileExt;
use std::{
    fs::{File, OpenOptions},
    io,
    path::Path,
};

pub fn guard(root: &Path, create: bool, wait: bool) -> io::Result<File> {
    if create {
        std::fs::create_dir_all(root)?;
    }
    crate::storage::verified_root_id("managed-builds", root)?;
    let path = root.join("admission.lock");
    if std::fs::symlink_metadata(&path).is_ok_and(|m| !m.file_type().is_file()) {
        return Err(io::Error::other("Build admission is not a regular file"));
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW).mode(0o600);
    }
    let file = options.open(path)?;
    if wait {
        file.lock_exclusive()?;
    } else {
        file.try_lock_exclusive()?;
    }
    Ok(file)
}
pub fn current(root: &Path) -> io::Result<String> {
    let bytes = crate::storage::read_small(&root.join(".desktop-current"), 128)?;
    let value = std::str::from_utf8(&bytes)
        .map_err(io::Error::other)?
        .trim();
    if !valid_id(value) {
        return Err(io::Error::other(
            "Current desktop build reference is unknown",
        ));
    }
    Ok(value.into())
}
pub fn record_current(root: &Path, digest: &str) -> io::Result<()> {
    if !valid_id(digest) {
        return Err(io::Error::other("Invalid managed build identity"));
    }
    crate::atomic_write::atomic_write(
        &crate::fs::StdFileSystem,
        &root.join(".desktop-current"),
        digest.as_bytes(),
    )
    .map_err(io::Error::other)
}
pub fn valid_id(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
