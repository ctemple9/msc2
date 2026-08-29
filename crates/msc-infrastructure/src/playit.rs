//! Playit-specific process inputs.
//!
//! The secret itself never enters this type.  A platform secret bridge gives
//! `playitd` an opaque, access-restricted path; the process supervisor only
//! receives that path as its `--secret-path` argument.

use crate::fs::FileSystem;
use crate::helper_acquisition::{
    AcquiredHelper, HelperAcquisitionError, HelperPlatform, PinnedHelperAsset, PinnedHelperRelease,
    acquire_pinned_helper,
};
use crate::jar_provider::Transport;
use crate::process::ProcessSpawnRequest;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

pub const PLAYIT_SECRET_KEY: &str = "playit.secret-key";
pub const PLAYIT_HELPER_NAME: &str = "playitd";
pub const PLAYIT_HELPER_VERSION: &str = "playitd-v1.0.10";
pub const PLAYIT_RELEASE_METADATA_URL: &str = "https://api.github.com/repos/ctemple9/minecraft-server-controller/releases/tags/playitd-v1.0.10";
pub const PLAYIT_HELPER_SHA256: &str =
    "91ae745a35aad7a058a9bfb3320d7dc27a54f66a8bb81831360966dd69acc791";

static NEXT_SECRET_BRIDGE_TEMP: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayitLaunch {
    pub working_directory: PathBuf,
    pub secret_path: PathBuf,
}

impl PlayitLaunch {
    pub fn process_request(&self, executable_path: &Path) -> ProcessSpawnRequest {
        ProcessSpawnRequest::new(executable_path, &self.working_directory).args([
            "--secret-path".to_string(),
            self.secret_path.to_string_lossy().into_owned(),
        ])
    }

    pub fn write_secret_bridge(&self, secret: &str) -> io::Result<PlayitSecretBridge> {
        PlayitSecretBridge::create(&self.secret_path, secret)
    }
}

/// A short-lived file that gives `playitd` access to the host-scoped key.
///
/// The key is written to a new file in the same directory and promoted with a
/// rename, so a helper can never observe a partially-written secret. The
/// bridge owns cleanup as well: callers keep it alive for the helper's whole
/// process lifetime and explicitly remove it when the process exits or is
/// reset. Its debug representation contains only the path.
#[derive(Debug)]
pub struct PlayitSecretBridge {
    path: PathBuf,
}

impl PlayitSecretBridge {
    pub fn create(path: &Path, secret: &str) -> io::Result<Self> {
        let secret = secret.trim();
        if secret.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Playit secret cannot be empty",
            ));
        }
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Playit secret bridge path has no parent directory",
            )
        })?;
        std::fs::create_dir_all(parent)?;

        let mut temporary_path = None;
        let mut temporary_file = None;
        for _ in 0..8 {
            let suffix = NEXT_SECRET_BRIDGE_TEMP.fetch_add(1, Ordering::Relaxed);
            let candidate = parent.join(format!(
                ".{}.tmp-{}-{}",
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("secret"),
                std::process::id(),
                suffix
            ));
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(restrictive_mode());
            match options.open(&candidate) {
                Ok(file) => {
                    temporary_path = Some(candidate);
                    temporary_file = Some(file);
                    break;
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
        let temporary_path = temporary_path.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AlreadyExists,
                "could not allocate a unique Playit secret bridge temporary file",
            )
        })?;
        let mut temporary_file = temporary_file.expect("temporary path has a file");
        let result = (|| {
            temporary_file.write_all(secret.as_bytes())?;
            temporary_file.write_all(b"\n")?;
            temporary_file.sync_all()?;
            drop(temporary_file);

            // Unix rename atomically replaces an existing file. Windows does
            // not, so only remove an existing regular bridge at this exact
            // path before the final promotion.
            if cfg!(windows) && path.exists() {
                if std::fs::symlink_metadata(path)?.is_dir() {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "Playit secret bridge path is a directory",
                    ));
                }
                std::fs::remove_file(path)?;
            }
            std::fs::rename(&temporary_path, path)?;
            set_restrictive_permissions(path)
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary_path);
            return Err(result.expect_err("checked bridge creation error"));
        }
        Ok(Self {
            path: path.to_owned(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn remove(self) -> io::Result<()> {
        let path = self.path.clone();
        std::mem::forget(self);
        remove_bridge_file(&path)
    }

    pub fn remove_path(path: &Path) -> io::Result<()> {
        remove_bridge_file(path)
    }
}

impl Drop for PlayitSecretBridge {
    fn drop(&mut self) {
        let _ = remove_bridge_file(&self.path);
    }
}

#[cfg(unix)]
fn restrictive_mode() -> u32 {
    0o600
}

#[cfg(not(unix))]
fn restrictive_mode() -> u32 {
    0
}

#[cfg(unix)]
fn set_restrictive_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_restrictive_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn remove_bridge_file(path: &Path) -> io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() || metadata.file_type().is_symlink() => {
            std::fs::remove_file(path)
        }
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Playit secret bridge path is not a file",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// The release pin used by Playit. The one published MSC 1 asset is a
/// universal macOS binary; unsupported platforms fail explicitly until their
/// own repository-owned artifact and checksum are published.
pub fn pinned_playit_release(
    platform: HelperPlatform,
) -> Result<PinnedHelperRelease, HelperAcquisitionError> {
    match platform {
        HelperPlatform::MacosX86_64 | HelperPlatform::MacosAarch64 => Ok(PinnedHelperRelease {
            helper: PLAYIT_HELPER_NAME.into(),
            version: PLAYIT_HELPER_VERSION.into(),
            release_metadata_url: PLAYIT_RELEASE_METADATA_URL.into(),
            assets: vec![PinnedHelperAsset {
                platform,
                asset_name: PLAYIT_HELPER_NAME.into(),
                sha256: PLAYIT_HELPER_SHA256.into(),
            }],
        }),
        _ => Err(HelperAcquisitionError::ReleaseResolution(format!(
            "no pinned {PLAYIT_HELPER_NAME} artifact is published for {platform}"
        ))),
    }
}

/// Testable inputs for Playit's binary acquisition. Keeping the transport and
/// filesystem injected means lifecycle tests never contact a public provider.
pub struct PlayitBinaryAcquisition<'a> {
    transport: &'a dyn Transport,
    fs: &'a dyn FileSystem,
    cache_directory: &'a Path,
    pin: PinnedHelperRelease,
    platform: HelperPlatform,
}

impl<'a> PlayitBinaryAcquisition<'a> {
    pub fn new(
        transport: &'a dyn Transport,
        fs: &'a dyn FileSystem,
        cache_directory: &'a Path,
        pin: PinnedHelperRelease,
        platform: HelperPlatform,
    ) -> Self {
        Self {
            transport,
            fs,
            cache_directory,
            pin,
            platform,
        }
    }

    pub fn for_current_platform(
        transport: &'a dyn Transport,
        fs: &'a dyn FileSystem,
        cache_directory: &'a Path,
    ) -> Result<Self, HelperAcquisitionError> {
        let platform = HelperPlatform::current()?;
        let pin = pinned_playit_release(platform)?;
        Ok(Self::new(transport, fs, cache_directory, pin, platform))
    }

    pub fn acquire(&self) -> Result<AcquiredHelper, HelperAcquisitionError> {
        acquire_pinned_helper(
            self.transport,
            self.fs,
            self.cache_directory,
            &self.pin,
            self.platform,
        )
    }
}
