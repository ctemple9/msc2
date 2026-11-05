//! Original pack retention lives outside the running server's content tree.
//! Original bytes and extracted overrides are evidence, not an assertion that
//! client dependencies have already been resolved or are ready to distribute.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetainedFile {
    pub path: String,
    pub sha512: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetainedPack {
    pub schema_version: u32,
    #[serde(default)]
    pub origin: Option<crate::modpacks::PackOrigin>,
    pub id: String,
    pub relative_root: String,
    pub format: String,
    pub name: String,
    pub version: String,
    pub original: RetainedFile,
    pub override_files: BTreeMap<String, RetainedFile>,
    pub resolved_files: BTreeMap<String, RetainedFile>,
    pub unresolved: BTreeMap<String, String>,
    #[serde(default)]
    pub expected_files: BTreeMap<String, ExpectedPackFile>,
    #[serde(default)]
    pub server_excluded_file_ids: Vec<String>,
}

fn copy_exact(
    source: &Path,
    destination: &Path,
    limit: u64,
    cancelled: &dyn Fn() -> bool,
) -> Result<RetainedFile, String> {
    let metadata = std::fs::symlink_metadata(source).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err("Pack source is not a bounded regular file".into());
    }
    let parent = destination
        .parent()
        .ok_or("Missing retained pack directory")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    if fs2::available_space(parent).map_err(|error| error.to_string())? < metadata.len() {
        return Err("Insufficient space to retain the original pack".into());
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut output = options
        .open(destination)
        .map_err(|error| error.to_string())?;
    let result = (|| {
        let mut input = std::fs::File::open(source).map_err(|error| error.to_string())?;
        let mut buffer = [0u8; 128 * 1024];
        let mut hash = Sha512::new();
        let mut total = 0u64;
        loop {
            if cancelled() {
                return Err("Pack retention cancelled".into());
            }
            let count = input.read(&mut buffer).map_err(|error| error.to_string())?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > metadata.len() || total > limit {
                return Err("Pack source grew during retention".into());
            }
            hash.update(&buffer[..count]);
            output
                .write_all(&buffer[..count])
                .map_err(|error| error.to_string())?;
        }
        if total != metadata.len() {
            return Err("Pack source changed during retention".into());
        }
        output.sync_all().map_err(|error| error.to_string())?;
        Ok(RetainedFile {
            path: destination
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("Invalid retained filename")?
                .into(),
            sha512: format!("{:x}", hash.finalize()),
            byte_size: total,
        })
    })();
    drop(output);
    if result.is_err() {
        let _ = std::fs::remove_file(destination);
    }
    result
}

/// Keep each override layer intact. Reconstructing the player tree applies
/// shared overrides then client-overrides; the server importer keeps its own
/// shared/server precedence. Neither side destroys the other side's source.
fn retain_tree(
    source: &Path,
    target: &Path,
    prefix: &str,
    files: &mut BTreeMap<String, RetainedFile>,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    let entries = match std::fs::read_dir(source) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.to_string()),
    };
    for entry in entries {
        if cancelled() {
            return Err("Pack retention cancelled".into());
        }
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Pack filename is not Unicode")?;
        let relative = format!("{prefix}/{name}");
        msc_domain::content_collection::validate_relative_path(&relative)?;
        if relative.split('/').count() > 64 || files.len() >= 50_000 {
            return Err("Pack override tree exceeds retention limits".into());
        }
        let metadata =
            std::fs::symlink_metadata(entry.path()).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() {
            return Err("Pack retention does not follow symbolic links".into());
        }
        if metadata.is_dir() {
            retain_tree(
                &entry.path(),
                &target.join(&name),
                &relative,
                files,
                cancelled,
            )?;
        } else if metadata.is_file() {
            let mut retained = copy_exact(
                &entry.path(),
                &target.join(&name),
                512 * 1024 * 1024,
                cancelled,
            )?;
            retained.path = format!("overrides/{relative}");
            if files
                .keys()
                .any(|path| path.eq_ignore_ascii_case(&relative))
            {
                return Err("Pack paths collide across platforms".into());
            }
            files.insert(relative, retained);
        }
    }
    Ok(())
}

pub fn capture_original(
    inspection: &crate::modpacks::ModpackInspection,
    server_dir: &Path,
    cancelled: &dyn Fn() -> bool,
) -> Result<Option<RetainedPack>, String> {
    let Some(original) = inspection.archive_path.as_deref() else {
        return Ok(None);
    };
    let (format, name, version) = match &inspection.format {
        crate::modpacks::InspectedFormat::Mrpack(pack) => {
            ("mrpack", pack.name.clone(), pack.version_id.clone())
        }
        crate::modpacks::InspectedFormat::CurseForge(pack) => {
            ("curseforge", pack.name.clone(), pack.version_id.clone())
        }
        crate::modpacks::InspectedFormat::PlainJarZip { .. } => return Ok(None),
    };
    let id = uuid::Uuid::new_v4().to_string();
    let relative_root = format!(".msc2-content/packs/{id}");
    let directory = crate::content_install::managed_path(
        server_dir.to_str().ok_or("Server path is not Unicode")?,
        &relative_root,
    )?;
    let original = copy_exact(
        original,
        &directory.join("original.zip"),
        10 * 1024 * 1024 * 1024,
        cancelled,
    )?;
    let mut pack = RetainedPack {
        schema_version: 1,
        origin: inspection.origin.clone(),
        id,
        relative_root,
        format: format.into(),
        name,
        version,
        original,
        override_files: BTreeMap::new(),
        resolved_files: BTreeMap::new(),
        unresolved: BTreeMap::new(),
        expected_files: BTreeMap::new(),
        server_excluded_file_ids: Vec::new(),
    };
    let mut layers = Vec::new();
    match &inspection.format {
        crate::modpacks::InspectedFormat::Mrpack(manifest) => {
            for file in &manifest.files {
                msc_domain::content_collection::validate_relative_path(&file.path)?;
                pack.unresolved.insert(
                    file.path.clone(),
                    "Manifest content has not been retained yet".into(),
                );
            }
            for layer in ["overrides", "client-overrides", "server-overrides"] {
                layers.push((layer, inspection.staged_dir.join(layer)));
            }
        }
        crate::modpacks::InspectedFormat::CurseForge(manifest) => {
            msc_domain::content_collection::validate_relative_path(&manifest.overrides_folder)?;
            for file in &manifest.files {
                pack.unresolved.insert(
                    format!("curseforge:{}", file.file_id),
                    "Manifest content has not been retained yet".into(),
                );
            }
            layers.push((
                "overrides",
                inspection.staged_dir.join(&manifest.overrides_folder),
            ));
        }
        _ => {}
    }
    pack.unresolved.insert(
        "override-capture".into(),
        "Override retention has not completed".into(),
    );
    save(server_dir, &pack)?;
    for (layer, source) in layers {
        retain_tree(
            &source,
            &directory.join("overrides").join(layer),
            layer,
            &mut pack.override_files,
            cancelled,
        )?;
    }
    pack.unresolved.remove("override-capture");
    save(server_dir, &pack)?;
    Ok(Some(pack))
}

pub fn root(server_dir: &Path, pack: &RetainedPack) -> Result<PathBuf, String> {
    uuid::Uuid::parse_str(&pack.id).map_err(|_| "Invalid retained pack identity")?;
    if pack.relative_root != format!(".msc2-content/packs/{}", pack.id) {
        return Err("Invalid retained pack root".into());
    }
    crate::content_install::managed_path(
        server_dir.to_str().ok_or("Server path is not Unicode")?,
        &pack.relative_root,
    )
}

fn retained_path(
    server_dir: &Path,
    pack: &RetainedPack,
    relative: &str,
) -> Result<PathBuf, String> {
    root(server_dir, pack)?;
    msc_domain::content_collection::validate_relative_path(relative)?;
    crate::content_install::managed_path(
        server_dir.to_str().ok_or("Server path is not Unicode")?,
        &format!("{}/{relative}", pack.relative_root),
    )
}

pub fn save(server_dir: &Path, pack: &RetainedPack) -> Result<(), String> {
    let bytes = serde_json::to_vec(pack).map_err(|error| error.to_string())?;
    msc_infrastructure::atomic_write::atomic_write(
        &msc_infrastructure::fs::StdFileSystem,
        &retained_path(server_dir, pack, "index.json")?,
        &bytes,
    )
    .map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedPackFile {
    pub path: String,
    pub byte_size: u64,
    pub sha1: Option<String>,
    pub sha512: Option<String>,
    pub mirrors: Vec<String>,
    pub source: Option<msc_domain::content_collection::ContentSource>,
    pub audience: Option<msc_domain::content_collection::ContentAudience>,
    pub optional: bool,
}

fn matches_expected(path: &Path, expected: &ExpectedPackFile) -> Result<(), String> {
    let identity = msc_infrastructure::addon_provider::fingerprint::identify(path)?;
    if identity.byte_size != expected.byte_size
        || expected
            .sha1
            .as_ref()
            .is_some_and(|hash| !hash.eq_ignore_ascii_case(&identity.sha1))
        || expected
            .sha512
            .as_ref()
            .is_some_and(|hash| !hash.eq_ignore_ascii_case(&identity.sha512))
        || (expected.sha1.is_none() && expected.sha512.is_none())
    {
        return Err("File does not match the pack manifest's size and checksums".into());
    }
    Ok(())
}

fn resolve_one(
    server_dir: &Path,
    pack: &RetainedPack,
    expected: &ExpectedPackFile,
    secrets: &dyn msc_infrastructure::secret_store::SecretStore,
    cancelled: &dyn Fn() -> bool,
) -> Result<RetainedFile, String> {
    msc_domain::content_collection::validate_relative_path(&expected.path)?;
    if expected.byte_size == 0 || expected.byte_size > 512 * 1024 * 1024 {
        return Err("Manifest file has no supported bounded size".into());
    }
    let directory = root(server_dir, pack)?;
    let destination = retained_path(server_dir, pack, &format!("files/{}", expected.path))?;
    if destination
        .try_exists()
        .map_err(|error| error.to_string())?
    {
        matches_expected(&destination, expected)?;
        let identity = msc_infrastructure::addon_provider::fingerprint::identify(&destination)?;
        return Ok(RetainedFile {
            path: format!("files/{}", expected.path),
            sha512: identity.sha512,
            byte_size: identity.byte_size,
        });
    }
    let mut candidates = Vec::new();
    let server_file = crate::content_install::managed_path(
        server_dir.to_str().ok_or("Server path is not Unicode")?,
        &expected.path,
    )?;
    candidates.push(server_file.clone());
    candidates.push(server_file.with_extension(format!(
            "{}.disabled",
            server_file
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or("")
        )));
    for layer in ["client-overrides", "overrides", "server-overrides"] {
        candidates.push(directory.join("overrides").join(layer).join(&expected.path));
    }
    for source in candidates {
        if cancelled() {
            return Err("Pack retention cancelled".into());
        }
        if matches_expected(&source, expected).is_ok() {
            let mut retained = copy_exact(&source, &destination, expected.byte_size, cancelled)?;
            if let Err(error) = matches_expected(&destination, expected) {
                let _ = std::fs::remove_file(&destination);
                return Err(error);
            }
            retained.path = format!("files/{}", expected.path);
            return Ok(retained);
        }
    }
    for url in &expected.mirrors {
        if cancelled() {
            return Err("Pack retention cancelled".into());
        }
        let provider = if url.starts_with("https://cdn.modrinth.com/") {
            "modrinth"
        } else if url.starts_with("https://edge.forgecdn.net/")
            || url.starts_with("https://mediafilez.forgecdn.net/")
        {
            "curseforge"
        } else {
            continue;
        };
        let artifact = msc_infrastructure::addon_provider::artifact::Artifact {
            provider,
            project_id: expected
                .source
                .as_ref()
                .map_or_else(String::new, |source| source.project_id.clone()),
            version_id: expected
                .source
                .as_ref()
                .map_or_else(String::new, |source| source.version_id.clone()),
            file_name: Path::new(&expected.path)
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or("Invalid manifest filename")?
                .into(),
            url: url.clone(),
            size: expected.byte_size,
            sha1: expected.sha1.clone(),
            sha512: expected.sha512.clone(),
        };
        if msc_infrastructure::addon_provider::artifact::acquire(
            &artifact,
            secrets,
            &destination,
            512 * 1024 * 1024,
            cancelled,
        )
        .is_ok()
        {
            matches_expected(&destination, expected)?;
            let identity = msc_infrastructure::addon_provider::fingerprint::identify(&destination)?;
            return Ok(RetainedFile {
                path: format!("files/{}", expected.path),
                sha512: identity.sha512,
                byte_size: identity.byte_size,
            });
        }
    }
    Err("Exact client file needs manual recovery or a supported provider download".into())
}

/// Run after the server import so matching server files can be reused without
/// another download; the original archive/override layers were captured first.
pub fn resolve_content(
    server_dir: &Path,
    inspection: &crate::modpacks::ModpackInspection,
    pack: &mut RetainedPack,
    transport: &dyn msc_infrastructure::addon_provider::AddonTransport,
    secrets: &dyn msc_infrastructure::secret_store::SecretStore,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    use msc_domain::content_collection::{ContentAudience, ContentSource};
    let pinned_expected = pack.expected_files.clone();
    match &inspection.format {
        crate::modpacks::InspectedFormat::Mrpack(manifest) => {
            for file in &manifest.files {
                let source = file.downloads.iter().find_map(|url| {
                    let rest = url.strip_prefix("https://cdn.modrinth.com/data/")?;
                    let parts: Vec<_> = rest.split('/').collect();
                    (parts.len() >= 4 && parts[1] == "versions").then(|| ContentSource {
                        provider: "modrinth".into(),
                        project_id: parts[0].into(),
                        version_id: parts[2].into(),
                    })
                });
                let client = file.env.as_ref().and_then(|env| env.client.as_deref());
                let server = file.env.as_ref().and_then(|env| env.server.as_deref());
                let audience = if client == Some("unsupported") {
                    ContentAudience::Server
                } else if server == Some("unsupported") {
                    ContentAudience::Players
                } else {
                    ContentAudience::Both
                };
                pack.expected_files.insert(
                    file.path.clone(),
                    ExpectedPackFile {
                        path: file.path.clone(),
                        byte_size: file.file_size,
                        sha1: file.hashes.sha1.clone(),
                        sha512: file.hashes.sha512.clone(),
                        mirrors: file.downloads.clone(),
                        source,
                        audience: Some(audience),
                        optional: client == Some("optional"),
                    },
                );
            }
        }
        crate::modpacks::InspectedFormat::CurseForge(manifest) => {
            let ids: Vec<_> = manifest.files.iter().map(|file| file.file_id).collect();
            let files = match msc_infrastructure::addon_provider::curseforge_files(
                transport, secrets, &ids,
            ) {
                Ok(files) => files,
                Err(error) => {
                    pack.unresolved
                        .insert("provider-resolution".into(), error.to_string());
                    save(server_dir, pack)?;
                    return Ok(());
                }
            };
            pack.unresolved.remove("provider-resolution");
            let project_ids: Vec<_> = manifest.files.iter().map(|file| file.project_id).collect();
            let projects = match msc_infrastructure::addon_provider::java_catalog::projects(
                transport,
                secrets,
                &project_ids,
            ) {
                Ok(projects) => projects,
                Err(error) => {
                    pack.unresolved
                        .insert("provider-resolution".into(), error.to_string());
                    save(server_dir, pack)?;
                    return Ok(());
                }
            };

            for declared in &manifest.files {
                let Some(file) = files
                    .iter()
                    .find(|file| file.id == declared.file_id && file.mod_id == declared.project_id)
                else {
                    continue;
                };
                msc_domain::content_collection::validate_relative_path(&file.file_name)?;
                if file.file_name.contains('/') {
                    return Err("Invalid CurseForge filename".into());
                }
                let class = projects
                    .iter()
                    .find(|project| project.id == file.mod_id)
                    .and_then(|project| project.class_id);
                let (folder, audience) = match class {
                    Some(6) => ("mods", None),
                    Some(5) => ("plugins", Some(ContentAudience::Server)),
                    Some(12) => ("resourcepacks", Some(ContentAudience::Players)),
                    Some(6552) => ("shaderpacks", Some(ContentAudience::Players)),
                    _ => {
                        pack.unresolved.insert(
                            format!("curseforge:{}", file.id),
                            "Unknown CurseForge content class; folder placement needs review"
                                .into(),
                        );
                        continue;
                    }
                };
                let path = format!("{folder}/{}", file.file_name);
                pack.unresolved.remove(&format!("curseforge:{}", file.id));
                pack.expected_files.insert(
                    path.clone(),
                    ExpectedPackFile {
                        path,
                        byte_size: file.file_length,
                        sha1: file
                            .hashes
                            .iter()
                            .find(|hash| hash.algo == 1)
                            .map(|hash| hash.value.clone()),
                        sha512: None,
                        mirrors: file.download_url.clone().into_iter().collect(),
                        source: Some(ContentSource {
                            provider: "curseforge".into(),
                            project_id: file.mod_id.to_string(),
                            version_id: file.id.to_string(),
                        }),
                        audience,
                        optional: !declared.required,
                    },
                );
            }
        }
        _ => return Ok(()),
    }
    for (path, pinned) in pinned_expected {
        if let Some(resolved) = pack.expected_files.get(&path)
            && (resolved.byte_size != pinned.byte_size
                || resolved.sha1 != pinned.sha1
                || resolved.sha512 != pinned.sha512
                || resolved.source != pinned.source)
        {
            pack.expected_files.insert(path.clone(), pinned);
            pack.unresolved.insert(
                path,
                "Provider identity changed; original pinned metadata preserved".into(),
            );
            save(server_dir, pack)?;
            return Err("Provider changed a pinned file identity".into());
        }
    }
    let mut folded = std::collections::BTreeSet::new();
    for path in pack.expected_files.keys() {
        msc_domain::content_collection::validate_relative_path(path)?;
        if !folded.insert(path.to_lowercase()) {
            return Err("Manifest paths collide across platforms".into());
        }
    }
    for expected in pack.expected_files.values_mut() {
        if expected.source.as_ref().is_some_and(|source| {
            source.provider == "curseforge"
                && pack.server_excluded_file_ids.contains(&source.version_id)
        }) {
            expected.audience = Some(ContentAudience::Players);
        }
    }
    save(server_dir, pack)?;
    for (path, expected) in pack.expected_files.clone() {
        if cancelled() {
            return Err("Pack retention cancelled".into());
        }
        match resolve_one(server_dir, pack, &expected, secrets, cancelled) {
            Ok(file) => {
                if expected.audience.is_none() {
                    let classified =
                        local_audience(&path, &root(server_dir, pack)?.join(&file.path));
                    if let Some(audience) = classified {
                        pack.expected_files
                            .get_mut(&path)
                            .ok_or("Expected file disappeared")?
                            .audience = Some(audience);
                    }
                }
                pack.resolved_files.insert(path.clone(), file);
                pack.unresolved.remove(&path);
            }
            Err(error) => {
                pack.unresolved.insert(path, error);
            }
        }
        save(server_dir, pack)?;
    }
    Ok(())
}

fn collection_entry(
    pack: &RetainedPack,
    path: &str,
    file: &RetainedFile,
    source: Option<msc_domain::content_collection::ContentSource>,
    audience: msc_domain::content_collection::ContentAudience,
    optional: bool,
    uncertain: bool,
) -> msc_domain::content_collection::ContentEntry {
    msc_domain::content_collection::ContentEntry {
        path: path.into(),
        sha512: file.sha512.clone(),
        byte_size: file.byte_size,
        audience,
        optional,
        source,
        redistribution_evidence: None,
        classification_evidence: if uncertain {
            Vec::new()
        } else {
            vec!["Pack environment or override layer".into()]
        },
        unresolved_reasons: if uncertain {
            vec!["Review client/server classification for this imported file".into()]
        } else {
            Vec::new()
        },
        extra: BTreeMap::from([
            (
                "retainedPath".into(),
                serde_json::Value::String(format!("{}/{}", pack.relative_root, file.path)),
            ),
            ("packId".into(), serde_json::Value::String(pack.id.clone())),
        ]),
    }
}

/// Authored baselines and editable collections are separate. Each audience has
/// its own map, so client/server overrides never overwrite one another's bytes.
pub fn collection(
    server_dir: &Path,
    pack: &RetainedPack,
    previous: Option<&msc_domain::content_collection::ContentCollection>,
) -> Result<msc_domain::content_collection::ContentCollection, String> {
    use msc_domain::content_collection::ContentAudience;
    let mut server_files = BTreeMap::new();
    let mut player_files = BTreeMap::new();
    for (path, file) in &pack.resolved_files {
        let expected = pack
            .expected_files
            .get(path)
            .ok_or("Retained file has no manifest entry")?;
        let audience = expected.audience.unwrap_or(ContentAudience::Both);
        let entry = collection_entry(
            pack,
            path,
            file,
            expected.source.clone(),
            audience,
            expected.optional,
            expected.audience.is_none(),
        );
        if audience != ContentAudience::Players {
            server_files.insert(path.clone(), entry.clone());
        }
        if audience != ContentAudience::Server {
            player_files.insert(path.clone(), entry);
        }
    }
    for layer in ["overrides", "server-overrides", "client-overrides"] {
        for (path, file) in &pack.override_files {
            let Some(path) = path.strip_prefix(&format!("{layer}/")) else {
                continue;
            };
            let asset = path.starts_with("shaderpacks/")
                || path.starts_with("resourcepacks/")
                || path == "options.txt";
            let local = local_audience(path, &root(server_dir, pack)?.join(&file.path));
            let audience = if layer == "client-overrides" || asset {
                ContentAudience::Players
            } else if layer == "server-overrides" {
                ContentAudience::Server
            } else {
                local.unwrap_or(ContentAudience::Both)
            };
            let entry = collection_entry(
                pack,
                path,
                file,
                None,
                audience,
                false,
                path.starts_with("mods/") && local.is_none(),
            );
            if audience != ContentAudience::Players {
                server_files.insert(path.into(), entry.clone());
            }
            if audience != ContentAudience::Server {
                player_files.insert(path.into(), entry);
            }
        }
    }
    let mut collection = previous.cloned().unwrap_or_default();
    collection.revision = collection
        .revision
        .checked_add(1)
        .ok_or("Content revision overflow")?;
    collection.upstream_files = server_files.clone();
    collection.upstream_player_files = player_files.clone();
    for (path, mut entry) in server_files {
        let actual_path = crate::content_install::managed_path(
            server_dir.to_str().ok_or("Server path is not Unicode")?,
            &path,
        )?;
        match msc_infrastructure::addon_provider::fingerprint::identify(&actual_path) {
            Ok(actual) if actual.sha512 == entry.sha512 => {}
            Ok(actual) => {
                entry.sha512 = actual.sha512;
                entry.byte_size = actual.byte_size;
                entry.source = None;
                entry.extra.remove("retainedPath");
                entry
                    .unresolved_reasons
                    .push("Server content differs from the retained pack baseline".into());
            }
            Err(_) => entry.unresolved_reasons.push(
                "Server content is missing or was excluded; review its audience before publication"
                    .into(),
            ),
        }
        collection.current_files.insert(path, entry);
    }
    for (path, entry) in player_files {
        let owner_edit = previous
            .and_then(|previous| previous.player_files.get(&path))
            .filter(|current| {
                previous.and_then(|previous| previous.upstream_player_files.get(&path))
                    != Some(*current)
            });
        if let Some(owner_edit) = owner_edit.filter(|current| current.sha512 != entry.sha512) {
            let mut kept = owner_edit.clone();
            kept.unresolved_reasons
                .push("Imported pack conflicts with this player customization".into());
            collection.player_files.insert(path, kept);
        } else {
            collection.player_files.insert(path, entry);
        }
    }
    if !collection.retained_roots.contains(&pack.relative_root) {
        collection.retained_roots.push(pack.relative_root.clone());
    }
    if let Some(origin) = &pack.origin {
        collection.upstream = Some(origin.source.clone());
        collection.extra.insert(
            "sourceServerPack".into(),
            serde_json::to_value(origin.server_pack).map_err(|error| error.to_string())?,
        );
    } else {
        collection
            .extra
            .insert("sourceServerPack".into(), serde_json::Value::Null);
    }
    collection.extra.insert(
        "playerRecoveryRequired".into(),
        serde_json::Value::Bool(
            pack.origin
                .as_ref()
                .is_some_and(|origin| origin.server_pack == Some(true)),
        ),
    );
    collection.extra.remove("pendingPackRetention");
    collection.extra.insert(
        "basePackId".into(),
        serde_json::Value::String(pack.id.clone()),
    );
    collection.extra.insert(
        "retainedPackUnresolved".into(),
        serde_json::to_value(&pack.unresolved).map_err(|error| error.to_string())?,
    );
    collection.validate()?;
    Ok(collection)
}

fn local_audience(
    path: &str,
    retained: &Path,
) -> Option<msc_domain::content_collection::ContentAudience> {
    use msc_domain::content_collection::ContentAudience;
    if path.starts_with("resourcepacks/")
        || path.starts_with("shaderpacks/")
        || path == "options.txt"
    {
        return Some(ContentAudience::Players);
    }
    if path.starts_with("mods/") && path.ends_with(".jar") {
        let stem = Path::new(path).file_stem()?.to_str()?;
        if msc_domain::modpack::known_client_only_reason(stem).is_some() {
            return Some(ContentAudience::Players);
        }
        let input = std::fs::File::open(retained).ok()?;
        let mut archive = zip::ZipArchive::new(input).ok()?;
        let entry = archive.by_name("fabric.mod.json").ok()?;
        if entry.size() > 64 * 1024 {
            return None;
        }
        let mut bytes = Vec::new();
        entry.take(64 * 1024 + 1).read_to_end(&mut bytes).ok()?;
        if bytes.len() > 64 * 1024 {
            return None;
        }
        let manifest: serde_json::Value = serde_json::from_slice(&bytes).ok()?;

        return match manifest["environment"].as_str() {
            Some("client") => Some(ContentAudience::Players),
            Some("server") => Some(ContentAudience::Server),
            Some("*") => Some(ContentAudience::Both),
            _ => None,
        };
    }
    None
}

pub fn load(
    server: &msc_domain::app_config_schema::ConfigServer,
    pack_id: &str,
) -> Result<RetainedPack, String> {
    use msc_infrastructure::fs::FileSystem;
    uuid::Uuid::parse_str(pack_id).map_err(|_| "Invalid retained pack identity")?;
    let relative = format!(".msc2-content/packs/{pack_id}");
    if !server
        .content_collection
        .as_ref()
        .is_some_and(|collection| collection.retained_roots.contains(&relative))
    {
        return Err("Retained pack does not belong to this server".into());
    }
    let directory = crate::content_install::managed_path(&server.server_dir, &relative)?;
    let bytes = msc_infrastructure::fs::StdFileSystem
        .read_bounded(&directory.join("index.json"), 32 * 1024 * 1024)
        .map_err(|error| error.to_string())?;
    let pack: RetainedPack = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if pack.schema_version != 1 || pack.id != pack_id || pack.relative_root != relative {
        return Err("Retained pack identity or schema changed".into());
    }
    if pack.original.path != "original.zip" || pack.original.sha512.len() != 128 {
        return Err("Invalid original archive identity".into());
    }
    for (path, file) in &pack.resolved_files {
        msc_domain::content_collection::validate_relative_path(path)?;
        if file.path != format!("files/{path}") {
            return Err("Invalid retained content path".into());
        }
    }
    for (path, file) in &pack.override_files {
        msc_domain::content_collection::validate_relative_path(path)?;
        if file.path != format!("overrides/{path}") {
            return Err("Invalid retained override path".into());
        }
    }
    for (path, expected) in &pack.expected_files {
        msc_domain::content_collection::validate_relative_path(path)?;
        if path != &expected.path {
            return Err("Retained manifest path differs from its key".into());
        }
    }
    Ok(pack)
}

/// Exact raw-file recovery also supports non-JAR manifest assets. The caller
/// has already redeemed an owner/file-bound staged upload; no host path is an
/// API input and no recovered file is copied into the running server here.
pub fn accept_manual_file(
    server_dir: &Path,
    pack: &mut RetainedPack,
    logical_path: &str,
    uploaded: &Path,
) -> Result<(), String> {
    let expected = pack
        .expected_files
        .get(logical_path)
        .ok_or("File does not belong to the retained manifest")?
        .clone();
    matches_expected(uploaded, &expected)?;
    let destination = retained_path(server_dir, pack, &format!("files/{logical_path}"))?;
    let existed = destination
        .try_exists()
        .map_err(|error| error.to_string())?;
    let mut retained = if existed {
        matches_expected(&destination, &expected)?;
        let identity = msc_infrastructure::addon_provider::fingerprint::identify(&destination)?;
        RetainedFile {
            path: String::new(),
            sha512: identity.sha512,
            byte_size: identity.byte_size,
        }
    } else {
        copy_exact(uploaded, &destination, expected.byte_size, &|| false)?
    };
    if let Err(error) = matches_expected(&destination, &expected) {
        if !existed {
            let _ = std::fs::remove_file(&destination);
        }
        return Err(error);
    }
    retained.path = format!("files/{logical_path}");
    if expected.audience.is_none()
        && let Some(audience) = local_audience(logical_path, &destination)
    {
        pack.expected_files
            .get_mut(logical_path)
            .ok_or("Expected file disappeared")?
            .audience = Some(audience);
    }
    pack.resolved_files.insert(logical_path.into(), retained);
    pack.unresolved.remove(logical_path);
    save(server_dir, pack)
}

/// Retry from the privately retained archive; never substitute a new upstream pack.
pub fn retry_resolution(
    server_dir: &Path,
    pack: &mut RetainedPack,
    transport: &dyn msc_infrastructure::addon_provider::AddonTransport,
    secrets: &dyn msc_infrastructure::secret_store::SecretStore,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), String> {
    let original = retained_path(server_dir, pack, "original.zip")?;
    let metadata = std::fs::symlink_metadata(&original).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() != pack.original.byte_size {
        return Err("Retained original changed; recovery requires the original archive".into());
    }
    let mut input = std::fs::File::open(&original).map_err(|error| error.to_string())?;
    let mut hasher = Sha512::new();
    let mut buffer = [0u8; 128 * 1024];
    let mut total = 0u64;
    loop {
        if cancelled() {
            return Err("Pack recovery cancelled".into());
        }
        let count = input.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > pack.original.byte_size {
            return Err("Retained original grew during recovery".into());
        }
        hasher.update(&buffer[..count]);
    }
    if format!("{:x}", hasher.finalize()) != pack.original.sha512 {
        return Err("Retained original checksum changed".into());
    }
    let recovery_id = uuid::Uuid::new_v4().to_string();
    let staging = retained_path(server_dir, pack, "recovery")?;
    let inspection = crate::modpacks::inspect_staged_archive(
        &msc_infrastructure::fs::StdFileSystem,
        transport,
        secrets,
        &original,
        &staging,
        &recovery_id,
    )
    .map_err(|error| error.to_string())?;
    let result = resolve_content(server_dir, &inspection, pack, transport, secrets, cancelled);
    let _ = std::fs::remove_dir_all(&inspection.staged_dir);
    result
}

/// Share an already accepted server file with the retained player copy only
/// when the pinned manifest identifies those exact bytes.
pub fn reconcile_server_file(
    server: &msc_domain::app_config_schema::ConfigServer,
    accepted: &Path,
) -> Result<Option<msc_domain::content_collection::ContentCollection>, String> {
    let Some(pack_id) = server
        .content_collection
        .as_ref()
        .and_then(|collection| collection.extra.get("basePackId"))
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(None);
    };
    let mut pack = load(server, pack_id)?;
    let candidates: Vec<_> = pack
        .expected_files
        .iter()
        .filter(|(path, expected)| {
            pack.unresolved.contains_key(*path) && matches_expected(accepted, expected).is_ok()
        })
        .map(|(path, _)| path.clone())
        .collect();
    if candidates.is_empty() {
        return Ok(None);
    }
    for path in candidates {
        accept_manual_file(Path::new(&server.server_dir), &mut pack, &path, accepted)?;
    }
    collection(
        Path::new(&server.server_dir),
        &pack,
        server.content_collection.as_ref(),
    )
    .map(Some)
}
