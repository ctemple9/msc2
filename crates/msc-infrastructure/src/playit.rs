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
use std::path::{Path, PathBuf};

pub const PLAYIT_SECRET_KEY: &str = "playit.secret-key";
pub const PLAYIT_HELPER_NAME: &str = "playitd";
pub const PLAYIT_HELPER_VERSION: &str = "playitd-v1.0.10";
pub const PLAYIT_RELEASE_METADATA_URL: &str = "https://api.github.com/repos/ctemple9/minecraft-server-controller/releases/tags/playitd-v1.0.10";
pub const PLAYIT_HELPER_SHA256: &str =
    "91ae745a35aad7a058a9bfb3320d7dc27a54f66a8bb81831360966dd69acc791";

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
