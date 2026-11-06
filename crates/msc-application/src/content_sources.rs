//! Provider-qualified discovery never treats a fuzzy fingerprint as byte identity.
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::content_collection::{ContentSource, validate_relative_path};
use msc_infrastructure::addon_provider::{AddonTransport, fingerprint, java_catalog as cf};
use msc_infrastructure::secret_store::SecretStore;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentifiedContent {
    pub path: String,
    pub sha512: Option<String>,
    pub byte_size: u64,
    pub source: Option<ContentSource>,
    pub candidates: Vec<ContentSource>,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentIdentification {
    pub items: Vec<IdentifiedContent>,
    pub note: Option<String>,
}

pub fn identify(
    server: &ConfigServer,
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    cancelled: &dyn Fn() -> bool,
) -> Result<ContentIdentification, String> {
    let kind = server
        .java_flavor
        .add_on_kind()
        .ok_or("Server has no mod/plugin content")?;
    let directory = crate::content_install::managed_path(&server.server_dir, kind.folder_name())?;
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ContentIdentification {
                items: Vec::new(),
                note: None,
            });
        }
        Err(error) => return Err(error.to_string()),
    };
    let mut items = Vec::new();
    let mut identities = BTreeMap::new();
    for entry in entries {
        if cancelled() {
            return Err("Identification cancelled".into());
        }
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Content filename is not Unicode")?;
        let lower = name.to_ascii_lowercase();
        if !lower.ends_with(".jar") && !lower.ends_with(".jar.disabled") {
            continue;
        }
        if items.len() >= 10_000 {
            return Err("Content identification exceeds its item limit".into());
        }
        let path = format!("{}/{name}", kind.folder_name());
        let mut item = IdentifiedContent {
            path: path.clone(),
            sha512: None,
            byte_size: 0,
            source: None,
            candidates: Vec::new(),
            status: "unidentified".into(),
            note: None,
        };
        let result = validate_relative_path(&path).and_then(|()| {
            let resolved = crate::content_install::managed_path(&server.server_dir, &path)?;
            if std::fs::symlink_metadata(entry.path())
                .map_err(|error| error.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err("Identification does not follow content symlinks".into());
            }
            fingerprint::identify(&resolved)
        });
        match result {
            Ok(identity) => {
                item.sha512 = Some(identity.sha512.clone());
                item.byte_size = identity.byte_size;
                if let Some(previous) = server
                    .content_collection
                    .as_ref()
                    .and_then(|collection| collection.current_files.get(&path))
                {
                    if previous.sha512 == identity.sha512 {
                        item.source = previous.source.clone();
                        if item.source.is_some() {
                            item.status = "identified".into();
                        }
                    } else {
                        item.status = "modified".into();
                        item.note =
                            Some("Installed bytes changed since source identification".into());
                    }
                }
                identities.insert(path, identity);
            }
            Err(error) => {
                item.status = "unreadable".into();
                item.note = Some(error);
            }
        }
        items.push(item);
    }
    let fingerprints: Vec<_> = identities
        .values()
        .map(|identity| identity.fingerprint)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let matches = match cf::fingerprint_matches(transport, secrets, &fingerprints) {
        Ok(matches) => matches,
        Err(error) => {
            return Ok(ContentIdentification {
                items,
                note: Some(error.to_string()),
            });
        }
    };
    for item in &mut items {
        let Some(identity) = identities.get(&item.path) else {
            continue;
        };
        let mut candidates = BTreeSet::new();
        for candidate in &matches {
            let file = &candidate.file;
            if file.mod_id > 0
                && file.id > 0
                && file.file_length == identity.byte_size
                && file
                    .hashes
                    .iter()
                    .any(|hash| hash.algo == 1 && hash.value.eq_ignore_ascii_case(&identity.sha1))
            {
                candidates.insert((file.mod_id.to_string(), file.id.to_string()));
            }
        }
        item.candidates = candidates
            .into_iter()
            .map(|(project_id, version_id)| ContentSource {
                provider: "curseforge".into(),
                project_id,
                version_id,
            })
            .collect();
        if item.source.is_none() {
            match item.candidates.as_slice() {
                [source] => {
                    item.source = Some(source.clone());
                    item.status = "identified".into();
                }
                [] => {}
                _ => {
                    item.status = "ambiguous".into();
                    item.note = Some(
                        "Several provider projects publish these exact bytes; review the source"
                            .into(),
                    );
                }
            }
        }
    }
    items.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(ContentIdentification { items, note: None })
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentUpdate {
    pub path: String,
    pub provider: String,
    pub project_id: String,
    pub installed_version_id: Option<String>,
    pub available_version_id: Option<String>,
    pub label: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentUpdates {
    pub items: Vec<ContentUpdate>,
    pub note: Option<String>,
}

pub fn updates(
    server: &ConfigServer,
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    cancelled: &dyn Fn() -> bool,
) -> Result<ContentUpdates, String> {
    let inventory = identify(server, transport, secrets, cancelled)?;
    let game_version = server
        .minecraft_version
        .as_deref()
        .ok_or("Server Minecraft version is unknown")?;
    let mut output = Vec::new();
    for item in inventory.items {
        let hint = server
            .content_collection
            .as_ref()
            .and_then(|collection| collection.current_files.get(&item.path))
            .filter(|entry| item.sha512.as_deref() == Some(entry.sha512.as_str()))
            .and_then(|entry| entry.extra.get("sourceHint"))
            .filter(|hint| hint["provider"] == "curseforge")
            .and_then(|hint| hint["projectId"].as_str())
            .map(|project_id| ContentSource {
                provider: "curseforge".into(),
                project_id: project_id.into(),
                version_id: String::new(),
            });
        let Some(source) = item
            .source
            .filter(|source| source.provider == "curseforge")
            .or(hint)
        else {
            continue;
        };
        if cancelled() {
            return Err("Update lookup cancelled".into());
        }
        let mut result = ContentUpdate {
            path: item.path,
            provider: source.provider,
            project_id: source.project_id.clone(),
            installed_version_id: (!source.version_id.is_empty())
                .then(|| source.version_id.clone()),
            available_version_id: None,
            label: None,
            note: inventory.note.clone(),
        };
        let project = source
            .project_id
            .parse()
            .map_err(|_| "Invalid recorded project ID")?;
        let release = crate::content_install::dependency_release(
            transport,
            secrets,
            project,
            server.java_flavor,
            game_version,
            cancelled,
        );
        match release {
            Ok(release) if source.version_id.is_empty() => {
                result.available_version_id = Some(release.id.to_string());
                result.label = Some(release.display_name);
                result.note = Some("Manual project link; installed release is unknown and replacement requires review".into());
            }
            Ok(release) if release.id.to_string() != source.version_id => {
                let current = cf::release(
                    transport,
                    secrets,
                    project,
                    source
                        .version_id
                        .parse()
                        .map_err(|_| "Invalid installed release ID")?,
                )
                .map_err(|error| error.to_string());
                match current {
                    Ok(current) if release.file_date > current.file_date => {
                        result.available_version_id = Some(release.id.to_string());
                        result.label = Some(release.display_name);
                    }
                    Ok(_) => {}
                    Err(error) => result.note = Some(error),
                }
            }
            Ok(_) => {}
            Err(error) => result.note = Some(error.to_string()),
        }
        output.push(result);
    }
    Ok(ContentUpdates {
        items: output,
        note: inventory.note,
    })
}

/// A manually chosen project is only a hint until a specific release checksum
/// matches. Never manufacture an installed version from a project selection.
#[derive(Clone, Copy)]
pub struct SourceSelection<'a> {
    pub path: &'a str,
    pub expected_sha512: &'a str,
    pub project_id: Option<u64>,
    pub version_id: Option<u64>,
}

pub fn link_source(
    server: &ConfigServer,
    selection: SourceSelection<'_>,
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
) -> Result<msc_domain::content_collection::ContentCollection, String> {
    let SourceSelection {
        path,
        expected_sha512,
        project_id,
        version_id,
    } = selection;
    use msc_domain::content_collection::{ContentAudience, ContentEntry};
    validate_relative_path(path)?;
    let folder = server
        .java_flavor
        .add_on_kind()
        .ok_or("Server has no mod/plugin content")?
        .folder_name();
    let name = path
        .strip_prefix(&format!("{folder}/"))
        .ok_or("Choose content in the server's add-on directory")?;
    if name.contains('/')
        || !(name.to_ascii_lowercase().ends_with(".jar")
            || name.to_ascii_lowercase().ends_with(".jar.disabled"))
    {
        return Err("Choose one installed JAR file".into());
    }
    let local = fingerprint::identify(&crate::content_install::managed_path(
        &server.server_dir,
        path,
    )?)?;
    if !local.sha512.eq_ignore_ascii_case(expected_sha512) {
        return Err("File changed before source linking".into());
    }
    let mut source = None;
    let mut hint = None;
    if let Some(project_id) = project_id {
        let project =
            cf::project(transport, secrets, project_id).map_err(|error| error.to_string())?;
        let class = if folder == "plugins" { 5 } else { 6 };
        if project.class_id != Some(class) {
            return Err("Project is not the installed content kind".into());
        }
        if let Some(version_id) = version_id {
            let release = cf::release(transport, secrets, project_id, version_id)
                .map_err(|error| error.to_string())?;
            if release.file_length != local.byte_size
                || !release
                    .hashes
                    .iter()
                    .any(|hash| hash.algo == 1 && hash.value.eq_ignore_ascii_case(&local.sha1))
            {
                return Err("Selected release does not match installed bytes".into());
            }
            source = Some(ContentSource {
                provider: "curseforge".into(),
                project_id: project_id.to_string(),
                version_id: version_id.to_string(),
            });
        } else {
            hint = Some(
                serde_json::json!({"provider": "curseforge", "projectId": project_id.to_string(), "provenance": "manual"}),
            );
        }
    } else if version_id.is_some() {
        return Err("A release requires its provider project".into());
    }
    let mut collection = server.content_collection.clone().unwrap_or_default();
    collection.revision = collection
        .revision
        .checked_add(1)
        .ok_or("Content revision overflow")?;
    let entry = collection
        .current_files
        .entry(path.to_owned())
        .or_insert_with(|| ContentEntry {
            path: path.to_owned(),
            sha512: local.sha512.clone(),
            byte_size: local.byte_size,
            audience: ContentAudience::Server,
            optional: false,
            source: None,
            redistribution_evidence: None,
            classification_evidence: Vec::new(),
            unresolved_reasons: vec!["Player requirements have not been reviewed".into()],
            extra: BTreeMap::new(),
        });
    if entry.sha512 != local.sha512 {
        entry.classification_evidence.clear();
        entry.redistribution_evidence = None;
        entry.unresolved_reasons =
            vec!["Content changed; player requirements must be reviewed".into()];
    }
    entry.sha512 = local.sha512;
    entry.byte_size = local.byte_size;
    entry.source = source;
    entry.extra.remove("sourceHint");
    if let Some(hint) = hint {
        entry.extra.insert("sourceHint".into(), hint);
    }
    collection.validate()?;
    Ok(collection)
}

pub fn verified_pins(server: &ConfigServer) -> Result<BTreeMap<u64, u64>, String> {
    let mut pins = BTreeMap::new();
    if let Some(collection) = &server.content_collection {
        for entry in collection.current_files.values().filter(|entry| {
            entry.path.ends_with(".jar")
                && entry.audience != msc_domain::content_collection::ContentAudience::Players
        }) {
            let Some(source) = entry
                .source
                .as_ref()
                .filter(|source| source.provider == "curseforge")
            else {
                continue;
            };
            let path = crate::content_install::managed_path(&server.server_dir, &entry.path)?;
            let actual = fingerprint::identify(&path)?;
            if actual.sha512 != entry.sha512 {
                return Err(format!(
                    "{} changed; identify it before reviewing dependencies",
                    entry.path
                ));
            }
            let project = source
                .project_id
                .parse()
                .map_err(|_| "Invalid installed project ID")?;
            let version = source
                .version_id
                .parse()
                .map_err(|_| "Invalid installed release ID")?;
            if pins
                .insert(project, version)
                .is_some_and(|previous| previous != version)
            {
                return Err("Several installed versions claim the same provider project".into());
            }
        }
    }
    Ok(pins)
}

pub struct UpdateSelection<'a> {
    pub path: &'a str,
    pub sha512: &'a str,
    pub project_id: u64,
    pub version_id: u64,
}

pub fn review_update(
    server: &ConfigServer,
    owner: &str,
    selection: UpdateSelection<'_>,
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    cancelled: &dyn Fn() -> bool,
) -> Result<crate::content_install::StoredInstallReview, String> {
    use crate::content_install::{self, ReviewContext, StoredInstallReview};
    validate_relative_path(selection.path)?;
    let folder = server
        .java_flavor
        .add_on_kind()
        .ok_or("Server has no add-ons")?
        .folder_name();
    let original = selection
        .path
        .strip_prefix(&format!("{folder}/"))
        .ok_or("Choose an installed add-on")?;
    if original.contains('/')
        || !(original.ends_with(".jar") || original.ends_with(".jar.disabled"))
    {
        return Err("Choose one installed add-on JAR".into());
    }
    let actual = fingerprint::identify(&content_install::managed_path(
        &server.server_dir,
        selection.path,
    )?)?;
    if actual.sha512 != selection.sha512 {
        return Err("File changed before update review".into());
    }
    let previous = server
        .content_collection
        .as_ref()
        .and_then(|collection| collection.current_files.get(selection.path));
    let recorded = previous.filter(|entry| entry.sha512 == actual.sha512);
    let linked = recorded
        .and_then(|entry| entry.source.as_ref())
        .is_some_and(|source| {
            source.provider == "curseforge" && source.project_id == selection.project_id.to_string()
        });
    let manual = recorded
        .and_then(|entry| entry.extra.get("sourceHint"))
        .is_some_and(|hint| {
            hint["provider"] == "curseforge"
                && hint["projectId"]
                    .as_str()
                    .and_then(|id| id.parse::<u64>().ok())
                    == Some(selection.project_id)
        });
    if !linked && !manual {
        let matches = cf::fingerprint_matches(transport, secrets, &[actual.fingerprint])
            .map_err(|error| error.to_string())?;
        if !matches.iter().any(|candidate| {
            candidate.file.mod_id.to_string() == selection.project_id.to_string()
                && candidate.file.file_length == actual.byte_size
                && candidate
                    .file
                    .hashes
                    .iter()
                    .any(|hash| hash.algo == 1 && hash.value.eq_ignore_ascii_case(&actual.sha1))
        }) {
            return Err("Installed bytes do not identify the selected project; link the source explicitly first".into());
        }
    }
    let baseline = content_install::content_baseline(server)?;
    let pins = verified_pins(server)?;
    let version = server
        .minecraft_version
        .clone()
        .ok_or("Minecraft version is unknown")?;
    let mut review = content_install::review_curseforge_with_pins(
        transport,
        secrets,
        selection.project_id,
        selection.version_id,
        ReviewContext {
            flavor: server.java_flavor,
            game_version: &version,
            installed: &pins,
        },
        cancelled,
    )
    .map_err(|error| error.to_string())?;
    let root = review
        .files
        .iter_mut()
        .find(|file| file.project_id == selection.project_id.to_string())
        .ok_or("Update review has no root release")?;
    if original.ends_with(".disabled") {
        root.file_name.push_str(".disabled");
    }
    let mut replacements = BTreeMap::from([(original.to_owned(), root.file_name.clone())]);
    if let Some(collection) = &server.content_collection {
        for file in &review.files {
            if file.project_id == selection.project_id.to_string() {
                continue;
            }
            let prior: Vec<_> = collection
                .current_files
                .values()
                .filter(|entry| {
                    entry.source.as_ref().is_some_and(|source| {
                        source.provider == "curseforge" && source.project_id == file.project_id
                    })
                })
                .collect();
            if prior.len() > 1 {
                return Err(
                    "Required dependency has several installed files; resolve duplicates first"
                        .into(),
                );
            }
            if let Some(entry) = prior.first() {
                if entry.path.ends_with(".disabled") {
                    return Err(
                        "A required dependency is disabled; review its enabled state first".into(),
                    );
                }
                let source = entry.source.as_ref().ok_or("Missing dependency identity")?;
                if source.version_id != file.version_id {
                    return Err(format!(
                        "Required dependency {} needs a separate update review",
                        file.title
                    ));
                }
            }
            for entry in collection
                .current_files
                .values()
                .filter_map(|entry| entry.source.as_ref())
                .filter(|source| source.provider == "curseforge")
            {
                if file.incompatible_projects.contains(&entry.project_id) {
                    return Err(format!(
                        "{} conflicts with installed project {}",
                        file.title, entry.project_id
                    ));
                }
            }
        }
    }
    if let Some(collection) = &server.content_collection {
        for file in &review.files {
            for installed in collection
                .current_files
                .values()
                .filter_map(|entry| entry.source.as_ref())
                .filter(|source| source.provider == "curseforge")
            {
                if file.incompatible_projects.contains(&installed.project_id) {
                    return Err(format!(
                        "{} conflicts with installed project {}",
                        file.title, installed.project_id
                    ));
                }
            }
        }
    }
    // Detect target collisions before owner approval, not halfway through apply.
    for file in &review.files {
        if baseline.contains_key(&file.file_name) && file.file_name != original {
            let identity = fingerprint::identify(&content_install::managed_path(
                &server.server_dir,
                &format!("{folder}/{}", file.file_name),
            )?)?;
            if identity.sha1 != file.sha1 {
                return Err("Update target collides with another installed file".into());
            }
        }
    }
    replacements.retain(|_, target| review.files.iter().any(|file| file.file_name == *target));
    review.warnings.push(format!(
        "Replace {original}; preserve the selected file's enabled/disabled state"
    ));
    let record = StoredInstallReview {
        allow_incompatible: false,
        id: uuid::Uuid::new_v4().to_string(),
        owner: owner.into(),
        server_id: server.id.clone(),
        server_dir: server.server_dir.clone(),
        minecraft_version: version,
        flavor: server.java_flavor.raw_value().into(),
        created_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_secs(),
        collection_revision: server
            .content_collection
            .as_ref()
            .map_or(0, |collection| collection.revision),
        baseline,
        replacements,
        review,
    };
    if content_install::content_baseline(server)? != record.baseline {
        return Err("Files changed during update review".into());
    }
    content_install::store_review(&record)?;
    Ok(record)
}
