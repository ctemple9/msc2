//! Publication metadata contains only reviewed content facts. Private staging
//! paths, arbitrary collection extras, credentials and runtime settings never
//! become export metadata. Frozen copies are independent of active/retained files.
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::content_collection::{ContentCollection, ContentSource};
use msc_infrastructure::addon_provider::fingerprint::identify;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedFile {
    pub path: String,
    pub sha512: String,
    pub sha1: String,
    pub byte_size: u64,
    pub optional: bool,
    pub enabled: bool,
    pub source: Option<ContentSource>,
    pub classification_evidence: Vec<String>,
    pub redistribution_evidence: String,
    pub world_slot: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicationIssue {
    pub audience: String,
    pub path: Option<String>,
    pub required: bool,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideFile {
    pub sha512: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Publication {
    pub schema_version: u32,
    pub id: String,
    pub server_id: String,
    pub server_name: String,
    pub created_at: String,
    pub audiences: Vec<String>,
    pub collection_revision: u64,
    pub publication_revision: u64,
    pub version_name: Option<String>,
    pub minecraft_version: Option<String>,
    pub loader: String,
    pub loader_version: Option<String>,
    #[serde(default)]
    pub player_setup: Option<crate::content_environment::PlayerSetup>,
    #[serde(default)]
    pub player_setup_reviewed: bool,
    pub guide_version: String,
    #[serde(default = "archive_layout_version")]
    pub archive_layout_version: String,
    pub guide_assets: BTreeMap<String, GuideFile>,
    pub incomplete_consent: bool,
    pub issues: Vec<PublicationIssue>,
    pub exclusions: Vec<PublicationIssue>,
    pub content_fingerprint: String,
    pub reviewed_server_paths: Vec<String>,
    pub reviewed_datapacks: Vec<crate::publication_datapacks::Selection>,
    pub player_files: BTreeMap<String, PublishedFile>,
    pub server_files: BTreeMap<String, PublishedFile>,
}
/// The general managed-path helper permits links that stay inside a registered
/// root. Publication reads deliberately reject every child link as well.
pub(crate) fn checked_source(server: &ConfigServer, relative: &str) -> Result<PathBuf, String> {
    msc_domain::content_collection::validate_relative_path(relative)?;
    let mut path = crate::content_install::managed_path(&server.server_dir, "")?;
    for component in relative.split('/') {
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err("Publication does not follow symbolic links".into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.to_string()),
        }
    }
    crate::content_install::managed_path(&server.server_dir, relative)
}

fn archive_layout_version() -> String {
    "zip-v1".into()
}
fn directory(server: &ConfigServer, id: &str) -> Result<PathBuf, String> {
    msc_domain::content_collection::validate_relative_path(id)?;
    if id.contains('/') || id.len() > 100 {
        return Err("Invalid publication identity".into());
    }
    checked_source(server, &format!(".msc2-content/publications/{id}"))
}
fn blob(server: &ConfigServer, id: &str, sha512: &str) -> Result<PathBuf, String> {
    if sha512.len() != 128 || !sha512.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Invalid publication checksum".into());
    }
    directory(server, id)?;
    checked_source(
        server,
        &format!(".msc2-content/publications/{id}/files/{sha512}"),
    )
}
fn validate(publication: &Publication, server: &ConfigServer, id: &str) -> Result<(), String> {
    if publication.schema_version != 1 || publication.id != id || publication.server_id != server.id
    {
        return Err("Publication identity or schema changed".into());
    }
    let audiences: std::collections::BTreeSet<_> =
        publication.audiences.iter().map(String::as_str).collect();
    if audiences.is_empty()
        || audiences.len() != publication.audiences.len()
        || !audiences
            .iter()
            .all(|audience| matches!(*audience, "players" | "server"))
        || (!audiences.contains("players") && !publication.player_files.is_empty())
        || (!audiences.contains("server") && !publication.server_files.is_empty())
    {
        return Err("Publication audience metadata is invalid".into());
    }
    if publication.guide_version.is_empty()
        || publication.guide_assets.is_empty()
        || publication.guide_assets.len() > 1000
    {
        return Err("Publication has no valid pinned guide assets".into());
    }
    let mut guide_total = 0u64;
    for (path, guide) in &publication.guide_assets {
        msc_domain::content_collection::validate_relative_path(path)?;
        if guide.sha512.len() != 128
            || !guide.sha512.bytes().all(|byte| byte.is_ascii_hexdigit())
            || guide.byte_size > 16 * 1024 * 1024
        {
            return Err("Invalid frozen guide identity".into());
        }
        guide_total = guide_total
            .checked_add(guide.byte_size)
            .ok_or("Guide size overflow")?;
    }
    if guide_total > 64 * 1024 * 1024 {
        return Err("Frozen guide assets exceed size limit".into());
    }
    for files in [&publication.player_files, &publication.server_files] {
        let mut folded = std::collections::BTreeSet::new();
        for (path, file) in files {
            crate::content_management::content_path(path)?;
            if file.path != *path || !folded.insert(path.to_lowercase()) {
                return Err("Publication paths collide or differ from metadata".into());
            }
            blob(server, id, &file.sha512)?;
            if file.sha1.len() != 40
                || !file.sha1.bytes().all(|byte| byte.is_ascii_hexdigit())
                || file.byte_size > 512 * 1024 * 1024
                || file.redistribution_evidence.trim().is_empty()
            {
                return Err("Publication file identity or eligibility is invalid".into());
            }
        }
    }
    Ok(())
}

/// Source paths belong to the private, owner-bound publication review. Only
/// PublishedFile is retained in the public manifest; callers never submit paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedPublicationFile {
    pub audience: String,
    pub source_relative: String,
    pub file: PublishedFile,
}

/// The caller owns the operation lock and has revalidated the publication
/// review. Copy every file before committing a manifest and registry reference.
#[allow(clippy::too_many_arguments)]
pub fn freeze(
    server: &ConfigServer,
    revision: u64,
    version_name: Option<String>,
    audiences: Vec<String>,
    player_setup: Option<crate::content_environment::PlayerSetup>,
    guide_version: &str,
    prepared: &[PreparedPublicationFile],
    mut issues: Vec<PublicationIssue>,
    exclusions: Vec<PublicationIssue>,
    content_fingerprint: String,
    reviewed_server_paths: Vec<String>,
    reviewed_datapacks: Vec<crate::publication_datapacks::Selection>,
    incomplete_consent: bool,
    cancelled: &dyn Fn() -> bool,
    persist: &mut dyn FnMut(ContentCollection) -> Result<(), String>,
) -> Result<Publication, String> {
    if server
        .content_collection
        .as_ref()
        .map_or(0, |collection| collection.revision)
        != revision
    {
        return Err("Content collection changed before publication".into());
    }
    if prepared.len() > 50_000 || issues.len() > 50_000 {
        return Err("Publication exceeds its file/issue limit".into());
    }
    if audiences.iter().any(|audience| audience == "players") && player_setup.is_none() {
        issues.push(PublicationIssue {
            audience: "players".into(),
            path: None,
            required: true,
            reason: "Exact player loader/version needs review".into(),
        });
    }
    if !issues.is_empty() && !incomplete_consent {
        return Err(
            "Unresolved selected content needs explicit incomplete-publication consent".into(),
        );
    }
    if let Some(setup) = &player_setup {
        setup.validate()?;
    }
    if version_name
        .as_ref()
        .is_some_and(|name| name.len() > 120 || name.chars().any(char::is_control))
        || guide_version.is_empty()
        || guide_version.len() > 120
    {
        return Err("Publication name or guide version is invalid".into());
    }
    let created_at = crate::notifications::iso8601_now();
    let previous_sequence = match server
        .content_collection
        .as_ref()
        .and_then(|collection| collection.extra.get("publicationSequence"))
    {
        Some(value) => value
            .as_u64()
            .ok_or("Publication revision registry is invalid")?,
        None => server
            .content_collection
            .as_ref()
            .map_or(0, |collection| collection.publication_ids.len() as u64),
    };
    let publication_revision = previous_sequence
        .checked_add(1)
        .ok_or("Publication revision overflow")?;
    let id = format!(
        "{}-r{publication_revision}-{}",
        &created_at[..10],
        uuid::Uuid::new_v4()
    );
    let mut publication = Publication {
        schema_version: 1,
        id: id.clone(),
        server_id: server.id.clone(),
        server_name: server.display_name.clone(),
        created_at,
        audiences,
        collection_revision: revision,
        publication_revision,
        version_name,
        minecraft_version: server.minecraft_version.clone(),
        loader: server.java_flavor.raw_value().into(),
        loader_version: server.loader_version.clone(),
        player_setup,
        player_setup_reviewed: true,
        guide_version: guide_version.into(),
        archive_layout_version: archive_layout_version(),
        guide_assets: BTreeMap::new(),
        incomplete_consent,
        issues,
        exclusions,
        content_fingerprint,
        reviewed_server_paths,
        reviewed_datapacks,
        player_files: BTreeMap::new(),
        server_files: BTreeMap::new(),
    };
    for prepared in prepared {
        let files = match prepared.audience.as_str() {
            "players" => &mut publication.player_files,
            "server" => &mut publication.server_files,
            _ => return Err("Unknown publication audience".into()),
        };
        if files
            .insert(prepared.file.path.clone(), prepared.file.clone())
            .is_some()
        {
            return Err("Duplicate publication path".into());
        }
    }
    if guide_version != crate::content_guides::VERSION {
        return Err("Unknown guide version; choose the installed version".into());
    }
    let guide_assets = crate::content_guides::assets(&publication);
    let mut guide_bytes = 0u64;
    use sha2::{Digest, Sha512};
    for asset in &guide_assets {
        msc_domain::content_collection::validate_relative_path(&asset.path)?;
        guide_bytes = guide_bytes
            .checked_add(asset.bytes.len() as u64)
            .ok_or("Guide size overflow")?;
        if asset.bytes.len() > 16 * 1024 * 1024 || guide_bytes > 64 * 1024 * 1024 {
            return Err("Guide assets exceed their size limit".into());
        }
        if publication
            .guide_assets
            .insert(
                asset.path.clone(),
                GuideFile {
                    sha512: format!("{:x}", Sha512::digest(&asset.bytes)),
                    byte_size: asset.bytes.len() as u64,
                },
            )
            .is_some()
        {
            return Err("Duplicate guide asset path".into());
        }
    }
    validate(&publication, server, &id)?;
    let root = directory(server, &id)?;
    std::fs::create_dir_all(root.parent().ok_or("Missing publication parent")?)
        .map_err(|error| error.to_string())?;
    let mut sizes = BTreeMap::new();
    for file in publication
        .player_files
        .values()
        .chain(publication.server_files.values())
    {
        if sizes
            .insert(&file.sha512, file.byte_size)
            .is_some_and(|previous| previous != file.byte_size)
        {
            return Err("One checksum has conflicting file sizes".into());
        }
    }
    let metadata_size = serde_json::to_vec(&publication)
        .map_err(|error| error.to_string())?
        .len() as u64;
    let required_space = sizes
        .values()
        .try_fold(
            metadata_size
                .checked_add(guide_bytes)
                .ok_or("Publication size overflow")?,
            |total, size| total.checked_add(*size),
        )
        .ok_or("Publication size overflow")?;
    if fs2::available_space(root.parent().ok_or("Missing publication parent")?)
        .map_err(|error| error.to_string())?
        < required_space
    {
        return Err("Insufficient space for the reviewed publication snapshot".into());
    }
    std::fs::create_dir(&root).map_err(|error| error.to_string())?;
    let mut copied = std::collections::BTreeSet::new();
    for prepared in prepared {
        if cancelled() {
            return Err("Publication cancelled; unfinished retained data remains protected".into());
        }
        msc_domain::content_collection::validate_relative_path(&prepared.source_relative)?;
        let source = checked_source(server, &prepared.source_relative)?;
        let expected = &prepared.file;
        let actual = identify(&source)?;
        if actual.sha512 != expected.sha512
            || actual.sha1 != expected.sha1
            || actual.byte_size != expected.byte_size
        {
            return Err(format!(
                "Content changed during publication: {}",
                expected.path
            ));
        }
        if copied.insert(expected.sha512.clone()) {
            let destination = blob(server, &id, &expected.sha512)?;
            crate::pack_retention::copy_exact(
                &source,
                &destination,
                expected.byte_size,
                cancelled,
            )?;
            let copied = identify(&destination)?;
            if copied.sha512 != expected.sha512
                || copied.sha1 != expected.sha1
                || copied.byte_size != expected.byte_size
            {
                return Err("Copied publication bytes changed".into());
            }
        }
    }
    if cancelled() {
        return Err("Publication cancelled before manifest commit".into());
    }
    for asset in guide_assets {
        if cancelled() {
            return Err("Publication cancelled while freezing guides".into());
        }
        let target = guide_path(server, &id, &asset.path)?;
        std::fs::create_dir_all(target.parent().ok_or("Missing guide parent")?)
            .map_err(|error| error.to_string())?;
        use std::io::Write;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(target).map_err(|error| error.to_string())?;
        output
            .write_all(&asset.bytes)
            .and_then(|()| output.sync_all())
            .map_err(|error| error.to_string())?;
    }
    #[cfg(unix)]
    {
        std::fs::File::open(root.join("guides"))
            .and_then(|file| file.sync_all())
            .map_err(|error| error.to_string())?;
        if !copied.is_empty() {
            std::fs::File::open(root.join("files"))
                .and_then(|file| file.sync_all())
                .map_err(|error| error.to_string())?;
        }
        std::fs::File::open(&root)
            .and_then(|file| file.sync_all())
            .map_err(|error| error.to_string())?;
    }
    let bytes = serde_json::to_vec(&publication).map_err(|error| error.to_string())?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Publication metadata exceeds size limit".into());
    }
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let manifest = checked_source(
        server,
        &format!(".msc2-content/publications/{id}/manifest.json"),
    )?;
    let mut output = options.open(manifest).map_err(|error| error.to_string())?;
    output
        .write_all(&bytes)
        .and_then(|()| output.sync_all())
        .map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        std::fs::File::open(&root)
            .and_then(|file| file.sync_all())
            .map_err(|error| error.to_string())?;
        std::fs::File::open(root.parent().ok_or("Missing publication parent")?)
            .and_then(|file| file.sync_all())
            .map_err(|error| error.to_string())?;
    }
    if cancelled() {
        return Err(
            "Publication cancelled before registry commit; frozen bytes remain protected".into(),
        );
    }
    let mut collection = server.content_collection.clone().unwrap_or_default();
    collection.extra.insert(
        "publicationSequence".into(),
        serde_json::json!(publication_revision),
    );
    collection.publication_ids.push(id.clone());
    let manifest_hashes = collection
        .extra
        .entry("publicationManifestHashes".into())
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or("Publication checksum registry is invalid")?;
    manifest_hashes.insert(
        id.clone(),
        serde_json::Value::String(format!("{:x}", Sha512::digest(&bytes))),
    );
    let relative = format!(".msc2-content/publications/{id}");
    collection.retained_roots.push(relative);
    collection.extra.insert(
        "lastPublishedCollectionRevision".into(),
        serde_json::json!(revision),
    );
    collection.validate()?;
    persist(collection).map_err(|error| {
        format!("Publication bytes retained but registry commit failed: {error}")
    })?;
    Ok(publication)
}

pub fn load(server: &ConfigServer, id: &str) -> Result<Publication, String> {
    use msc_infrastructure::fs::FileSystem;
    if !server
        .content_collection
        .as_ref()
        .is_some_and(|collection| {
            collection
                .publication_ids
                .iter()
                .any(|published| published == id)
        })
    {
        return Err("Publication does not belong to this server".into());
    }
    directory(server, id)?;
    let manifest = checked_source(
        server,
        &format!(".msc2-content/publications/{id}/manifest.json"),
    )?;
    let bytes = msc_infrastructure::fs::StdFileSystem
        .read_bounded(&manifest, 32 * 1024 * 1024)
        .map_err(|error| error.to_string())?;
    let expected = server
        .content_collection
        .as_ref()
        .and_then(|collection| collection.extra.get("publicationManifestHashes"))
        .and_then(serde_json::Value::as_object)
        .and_then(|hashes| hashes.get(id))
        .and_then(serde_json::Value::as_str)
        .ok_or("Publication manifest has no pinned registry checksum")?;
    use sha2::{Digest, Sha512};
    if format!("{:x}", Sha512::digest(&bytes)) != expected {
        return Err("Frozen publication manifest changed".into());
    }
    let publication: Publication =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    validate(&publication, server, id)?;
    Ok(publication)
}
pub(crate) fn frozen_file_location(
    server: &ConfigServer,
    publication: &Publication,
    file: &PublishedFile,
) -> Result<PathBuf, String> {
    let belongs = publication
        .player_files
        .get(&file.path)
        .into_iter()
        .chain(publication.server_files.get(&file.path))
        .any(|published| {
            published.sha512 == file.sha512
                && published.sha1 == file.sha1
                && published.byte_size == file.byte_size
        });
    if publication.server_id != server.id || !belongs {
        return Err("File does not belong to this publication".into());
    }
    blob(server, &publication.id, &file.sha512)
}

pub fn frozen_file(
    server: &ConfigServer,
    publication: &Publication,
    file: &PublishedFile,
) -> Result<PathBuf, String> {
    let path = frozen_file_location(server, publication, file)?;
    let actual = identify(&path)?;
    if actual.sha512 != file.sha512
        || actual.sha1 != file.sha1
        || actual.byte_size != file.byte_size
    {
        return Err("Frozen publication bytes changed or are unavailable".into());
    }
    Ok(path)
}

fn guide_path(server: &ConfigServer, id: &str, path: &str) -> Result<PathBuf, String> {
    directory(server, id)?;
    msc_domain::content_collection::validate_relative_path(path)?;
    checked_source(
        server,
        &format!(".msc2-content/publications/{id}/guides/{path}"),
    )
}
pub fn frozen_guide(
    server: &ConfigServer,
    publication: &Publication,
    path: &str,
) -> Result<PathBuf, String> {
    use msc_infrastructure::fs::FileSystem;
    if publication.server_id != server.id {
        return Err("Guide belongs to another server".into());
    }
    let guide = publication
        .guide_assets
        .get(path)
        .ok_or("Guide is not in the frozen publication")?;
    let target = guide_path(server, &publication.id, path)?;
    let bytes = msc_infrastructure::fs::StdFileSystem
        .read_bounded(&target, 16 * 1024 * 1024)
        .map_err(|error| error.to_string())?;
    use sha2::{Digest, Sha512};
    if bytes.len() as u64 != guide.byte_size
        || format!("{:x}", Sha512::digest(&bytes)) != guide.sha512
    {
        return Err("Frozen guide bytes changed".into());
    }
    Ok(target)
}
