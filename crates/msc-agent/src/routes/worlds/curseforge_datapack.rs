//! Resolve a Java datapack without confusing CurseForge mod archives with packs.
use msc_infrastructure::addon_provider::{HttpTransport, artifact, java_catalog as cf};
use std::path::Path;

pub(super) struct CancelDownload(pub std::sync::Arc<std::sync::atomic::AtomicBool>);
impl Drop for CancelDownload {
    fn drop(&mut self) {
        self.0.store(true, std::sync::atomic::Ordering::Release);
    }
}

pub(super) struct Download {
    pub bytes: Vec<u8>,
    pub project: cf::Project,
    pub release: cf::Release,
}

pub(super) fn download(
    root: &Path,
    project_id: &str,
    version_id: &str,
    minecraft_version: &str,
    cancelled: &dyn Fn() -> bool,
    manual: Option<(&Path, &str)>,
) -> Result<Download, String> {
    let project_id = project_id
        .parse::<u64>()
        .map_err(|_| "Invalid CurseForge project ID")?;
    let version_id = version_id
        .parse::<u64>()
        .map_err(|_| "Invalid CurseForge file ID")?;
    let secrets = crate::auth::production_secret_store().map_err(|error| error.to_string())?;
    let transport = HttpTransport::new();
    let project =
        cf::project(&transport, secrets.as_ref(), project_id).map_err(|error| error.to_string())?;
    let classes = cf::classes(&transport, secrets.as_ref()).map_err(|error| error.to_string())?;
    if !classes.iter().any(|class| {
        ["data-packs", "datapacks"].contains(&class.slug.as_str())
            && project.class_id == Some(i64::from(class.id))
    }) {
        return Err("This CurseForge project is not a Java datapack".into());
    }
    let release = cf::release(&transport, secrets.as_ref(), project_id, version_id)
        .map_err(|error| error.to_string())?;
    if !release.is_available
        || !release
            .game_versions
            .iter()
            .any(|value| value == minecraft_version)
    {
        return Err(
            "The selected datapack release is unavailable or does not list this Minecraft version"
                .into(),
        );
    }
    if let Some((path, name)) = manual {
        let identity = msc_infrastructure::addon_provider::fingerprint::identify(path)?;
        if name != release.file_name
            || identity.byte_size != release.file_length
            || !release
                .hashes
                .iter()
                .any(|hash| hash.algo == 1 && hash.value.eq_ignore_ascii_case(&identity.sha1))
        {
            return Err(
                "Choose the exact provider file; its name, size and SHA1 must match this release"
                    .into(),
            );
        }
        if cancelled() {
            return Err("Datapack download cancelled".into());
        }
        let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
        use sha1::Digest;
        let actual = format!("{:x}", sha1::Sha1::digest(&bytes));
        if bytes.len() as u64 != release.file_length
            || !release
                .hashes
                .iter()
                .any(|hash| hash.algo == 1 && hash.value.eq_ignore_ascii_case(&actual))
        {
            return Err("Manual datapack changed during verification".into());
        }
        return Ok(Download {
            bytes,
            project,
            release,
        });
    }
    let resolved = artifact::resolve(
        &transport,
        secrets.as_ref(),
        "curseforge",
        &project_id.to_string(),
        &version_id.to_string(),
    )
    .map_err(|error| error.to_string())?;
    if !resolved.file_name.to_ascii_lowercase().ends_with(".zip") {
        return Err("Select a datapack ZIP archive".into());
    }
    std::fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let path = root.join(format!("{}.zip", uuid::Uuid::new_v4()));
    let result = (|| {
        artifact::acquire(
            &resolved,
            secrets.as_ref(),
            &path,
            msc_infrastructure::addon_provider::DATAPACK_MAX_BYTES,
            cancelled,
        )
        .map_err(|error| error.to_string())?;
        if cancelled() {
            return Err("Datapack download cancelled".into());
        }
        let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
        Ok(Download {
            bytes,
            project,
            release,
        })
    })();
    let _ = std::fs::remove_file(path);
    result
}
