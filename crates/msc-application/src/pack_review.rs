//! Update review observes current bytes before comparing them with the old and
//! newly retained upstream trees. It stages no active-server mutations.
use crate::{content_management as management, pack_activation, pack_retention, pack_updates};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::content_collection::{ContentAudience, ContentCollection, ContentEntry};
use msc_infrastructure::addon_provider::{AddonTransport, fingerprint::identify};
use msc_infrastructure::secret_store::SecretStore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    #[serde(default)]
    pub replacement: bool,
    pub id: String,
    pub owner: String,
    pub created_unix: u64,
    pub minecraft_version: Option<String>,
    pub loader_version: Option<String>,
    pub flavor: String,
    pub source_collection: Option<ContentCollection>,
    pub inventory_baseline: BTreeMap<String, String>,
    pub observed_paths: BTreeMap<String, Option<String>>,
    pub activation: pack_activation::Activation,
    pub retained_pack: pack_retention::RetainedPack,
    pub server_changes: Vec<pack_updates::PathChange>,
    pub player_changes: Vec<pack_updates::PathChange>,
    pub missing: Vec<String>,
    pub warnings: Vec<String>,
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn path(server: &ConfigServer, id: &str) -> Result<std::path::PathBuf, String> {
    uuid::Uuid::parse_str(id).map_err(|_| "Invalid pack review identity")?;
    crate::content_install::managed_path(
        &server.server_dir,
        &format!(".msc2-content/pack-reviews/{id}.json"),
    )
}
fn observed(server: &ConfigServer, path: &str) -> Result<Option<String>, String> {
    management::content_path(path)?;
    let actual = crate::content_install::managed_path(&server.server_dir, path)?;
    match std::fs::symlink_metadata(&actual) {
        Ok(_) => identify(&actual).map(|identity| Some(identity.sha512)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}
fn supported(
    files: &BTreeMap<String, ContentEntry>,
    warnings: &mut Vec<String>,
) -> BTreeMap<String, ContentEntry> {
    files
        .iter()
        .filter_map(|(path, entry)| {
            if management::content_path(path).is_ok() {
                Some((path.clone(), entry.clone()))
            } else {
                warnings.push(format!(
                    "Outside reviewed content folders; left untouched: {path}"
                ));
                None
            }
        })
        .collect()
}
fn apply_audiences(
    collection: &mut ContentCollection,
    choices: &BTreeMap<String, ContentAudience>,
) -> Result<(), String> {
    for (path, audience) in choices {
        management::content_path(path)?;
        let mut entry = collection
            .upstream_files
            .get(path)
            .or_else(|| collection.upstream_player_files.get(path))
            .ok_or("Classification choice does not belong to this pack")?
            .clone();
        entry.audience = *audience;
        entry.classification_evidence = vec!["Owner reviewed this pack file's audience".into()];
        entry
            .unresolved_reasons
            .retain(|reason| !reason.contains("classification"));
        if *audience == ContentAudience::Players {
            collection.upstream_files.remove(path);
        } else {
            collection
                .upstream_files
                .insert(path.clone(), entry.clone());
        }
        if *audience == ContentAudience::Server {
            collection.upstream_player_files.remove(path);
        } else {
            collection.upstream_player_files.insert(path.clone(), entry);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn prepare(
    server: &ConfigServer,
    owner: &str,
    revision: u64,
    inspection: &crate::modpacks::ModpackInspection,
    retained: Option<pack_retention::RetainedPack>,
    restore_server: &[String],
    restore_player: &[String],
    audiences: &BTreeMap<String, ContentAudience>,
    confirm_unverified_origin: bool,
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    cancelled: &dyn Fn() -> bool,
) -> Result<Review, String> {
    prepare_inner(
        server,
        owner,
        revision,
        inspection,
        retained,
        restore_server,
        restore_player,
        audiences,
        confirm_unverified_origin,
        None,
        transport,
        secrets,
        cancelled,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplacementChoices {
    pub confirm_world_compatibility: bool,
    pub preserve_additions: BTreeMap<String, String>,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_replacement(
    server: &ConfigServer,
    owner: &str,
    revision: u64,
    inspection: &crate::modpacks::ModpackInspection,
    retained: Option<pack_retention::RetainedPack>,
    restore_server: &[String],
    restore_player: &[String],
    audiences: &BTreeMap<String, ContentAudience>,
    confirm_unverified_origin: bool,
    choices: &ReplacementChoices,
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    cancelled: &dyn Fn() -> bool,
) -> Result<Review, String> {
    if !choices.confirm_world_compatibility {
        return Err("Confirm that changing packs may make the existing world incompatible; MSC does not convert or replace the world".into());
    }
    prepare_inner(
        server,
        owner,
        revision,
        inspection,
        retained,
        restore_server,
        restore_player,
        audiences,
        confirm_unverified_origin,
        Some(choices),
        transport,
        secrets,
        cancelled,
    )
}

#[allow(clippy::too_many_arguments)]
fn prepare_inner(
    server: &ConfigServer,
    owner: &str,
    revision: u64,
    inspection: &crate::modpacks::ModpackInspection,
    retained: Option<pack_retention::RetainedPack>,
    restore_server: &[String],
    restore_player: &[String],
    audiences: &BTreeMap<String, ContentAudience>,
    confirm_unverified_origin: bool,
    replacement: Option<&ReplacementChoices>,
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    cancelled: &dyn Fn() -> bool,
) -> Result<Review, String> {
    if restore_server.len() + restore_player.len() + audiences.len() > 2048 {
        return Err("Too many explicit pack review choices".into());
    }
    let mut setup = server.clone();
    if replacement.is_some()
        && let Some(collection) = setup.content_collection.as_mut()
    {
        collection.upstream = None;
    }
    crate::content_recovery::compatible(&setup, inspection)?;
    let fallback = ContentCollection::default();
    let previous = match server.content_collection.as_ref() {
        Some(collection) => collection,
        None if replacement.is_some() => &fallback,
        None => return Err("Recover the original pack baseline before updating".into()),
    };
    if previous.revision != revision {
        return Err("Content changed; refresh pack review".into());
    }
    if replacement.is_none()
        && previous.upstream_files.is_empty()
        && previous.upstream_player_files.is_empty()
    {
        return Err("There is no original pack baseline to compare".into());
    }
    let mut warnings = Vec::new();
    if replacement.is_some() {
        warnings.push("Replacing the base pack may make existing worlds incompatible. Worlds are left untouched; MSC does not convert them.".into());
    }
    if inspection.origin.is_none() || (replacement.is_none() && previous.upstream.is_none()) {
        if !confirm_unverified_origin {
            return Err("Explicitly confirm that this unverified archive is an update of the current base pack; its title is not identity evidence".into());
        }
        warnings.push(
            "Owner confirmed an unverified local pack association; provider origin is not proven"
                .into(),
        );
    }
    let snapshot = management::snapshot(server)?;
    let mut pack = match retained {
        Some(pack) => pack,
        None => {
            pack_retention::capture_original(inspection, Path::new(&server.server_dir), cancelled)?
                .ok_or("Choose a complete pack archive")?
        }
    };
    pack_retention::resolve_content(
        Path::new(&server.server_dir),
        inspection,
        &mut pack,
        transport,
        secrets,
        cancelled,
    )?;
    if cancelled() {
        return Err("Pack review cancelled".into());
    }
    let mut incoming =
        pack_retention::collection(Path::new(&server.server_dir), &pack, Some(previous))?;
    apply_audiences(&mut incoming, audiences)?;
    let old_server = supported(&previous.upstream_files, &mut warnings);
    let new_server = supported(&incoming.upstream_files, &mut warnings);
    let mut current = supported(&snapshot.collection.current_files, &mut warnings);
    let mut observed_paths = BTreeMap::new();
    for logical in old_server
        .keys()
        .chain(new_server.keys())
        .chain(current.keys())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
    {
        if cancelled() {
            return Err("Pack observation cancelled".into());
        }
        let hash = observed(server, &logical)?;
        observed_paths.insert(logical.clone(), hash.clone());
        if let Some(hash) = hash {
            let actual = identify(&crate::content_install::managed_path(
                &server.server_dir,
                &logical,
            )?)?;
            let mut entry = current
                .get(&logical)
                .or_else(|| old_server.get(&logical))
                .or_else(|| new_server.get(&logical))
                .ok_or("Observed path has no content metadata")?
                .clone();
            if entry.sha512 != hash {
                entry.sha512 = hash;
                entry.byte_size = actual.byte_size;
                entry.source = None;
                entry.redistribution_evidence = None;
                entry.extra.remove("retainedPath");
                entry
                    .unresolved_reasons
                    .push("Owner-edited server bytes preserved by pack review".into());
            }
            current.insert(logical, entry);
        } else {
            current.remove(&logical);
        }
    }
    let server_merge = pack_updates::compare(
        &old_server,
        &current,
        &new_server,
        &previous
            .excluded_paths
            .iter()
            .filter(|path| management::content_path(path).is_ok())
            .cloned()
            .collect::<Vec<_>>(),
        restore_server,
    )?;
    let old_player = supported(&previous.upstream_player_files, &mut warnings);
    let current_player = supported(&previous.player_files, &mut warnings);
    let new_player = supported(&incoming.upstream_player_files, &mut warnings);
    let player_merge = pack_updates::compare(
        &old_player,
        &current_player,
        &new_player,
        &previous
            .player_excluded_paths
            .iter()
            .filter(|path| management::content_path(path).is_ok())
            .cloned()
            .collect::<Vec<_>>(),
        restore_player,
    )?;
    let mut collection = incoming;
    collection.current_files = snapshot.collection.current_files;
    for logical in old_server
        .keys()
        .chain(current.keys())
        .chain(new_server.keys())
    {
        collection.current_files.remove(logical);
    }
    collection.current_files.extend(server_merge.files.clone());
    collection.player_files = previous.player_files.clone();
    for logical in old_player
        .keys()
        .chain(current_player.keys())
        .chain(new_player.keys())
    {
        collection.player_files.remove(logical);
    }
    collection.player_files.extend(player_merge.files);
    collection.excluded_paths = server_merge.excluded_paths;
    collection.excluded_paths.extend(
        previous
            .excluded_paths
            .iter()
            .filter(|path| management::content_path(path).is_err())
            .cloned(),
    );
    collection.player_excluded_paths = player_merge.excluded_paths;
    collection.player_excluded_paths.extend(
        previous
            .player_excluded_paths
            .iter()
            .filter(|path| management::content_path(path).is_err())
            .cloned(),
    );
    collection.revision = revision.checked_add(1).ok_or("Content revision overflow")?;
    if inspection.origin.is_none() {
        collection.upstream = None;
        collection.extra.insert(
            "updateOriginUnverified".into(),
            serde_json::Value::Bool(true),
        );
    }
    let mut missing: Vec<String> = pack
        .unresolved
        .iter()
        .map(|(path, reason)| format!("{path}: {reason}"))
        .collect();
    let mut changes = Vec::new();
    for change in &server_merge.changes {
        if change.action == "keep" {
            continue;
        }
        let after = server_merge.files.get(&change.path).cloned();
        if after
            .as_ref()
            .is_some_and(|entry| entry.classification_evidence.is_empty())
        {
            missing.push(format!(
                "Choose server/player audience before activating: {}",
                change.path
            ));
        }
        changes.push(pack_activation::FileChange {
            path: change.path.clone(),
            before_sha512: change.current_sha512.clone(),
            after,
        });
    }
    for entry in collection.player_files.values() {
        if let Err(error) = management::player_source(server, entry) {
            missing.push(format!("{}: {error}", entry.path));
        }
    }
    if let Some(choices) = replacement {
        for (side, files, upstream) in [
            ("server", &collection.current_files, &new_server),
            ("players", &collection.player_files, &new_player),
        ] {
            for (path, entry) in files {
                if !path.starts_with("mods/")
                    || upstream
                        .get(path)
                        .is_some_and(|incoming| incoming.sha512 == entry.sha512)
                {
                    continue;
                }
                let key = format!("{side}/{path}");
                if choices.preserve_additions.get(&key) != Some(&entry.sha512) {
                    missing.push(format!(
                        "Review compatibility of preserved owner mod: {key}"
                    ));
                }
            }
            let mut projects: BTreeMap<(String, String), std::collections::BTreeSet<String>> =
                BTreeMap::new();
            for entry in files.values() {
                if let Some(source) = &entry.source {
                    projects
                        .entry((source.provider.clone(), source.project_id.clone()))
                        .or_default()
                        .insert(source.version_id.clone());
                }
            }
            for ((provider, project), versions) in &projects {
                if versions.len() > 1 {
                    missing.push(format!(
                        "Conflicting versions of {provider} project {project} in {side}"
                    ));
                }
            }
            for (path, entry) in files {
                let Some(source) = &entry.source else {
                    continue;
                };
                for project in entry
                    .extra
                    .get("incompatibleProjects")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                {
                    if projects.contains_key(&(source.provider.clone(), project.to_string())) {
                        missing.push(format!("Known incompatible content must be resolved in {side}: {path} conflicts with {} project {project}", source.provider));
                    }
                }
            }
        }
        collection.extra.insert(
            "replacementCompatibilityConfirmations".into(),
            serde_json::to_value(&choices.preserve_additions).map_err(|error| error.to_string())?,
        );
        collection.extra.insert(
            "worldCompatibilityAcknowledged".into(),
            serde_json::Value::Bool(true),
        );
    }
    collection.validate()?;
    let id = uuid::Uuid::new_v4().to_string();
    let review = Review {
        replacement: replacement.is_some(),
        id: id.clone(),
        owner: owner.into(),
        created_unix: now(),
        minecraft_version: server.minecraft_version.clone(),
        loader_version: server.loader_version.clone(),
        flavor: format!("{:?}", server.java_flavor),
        source_collection: server.content_collection.clone(),
        inventory_baseline: snapshot.server_baseline,
        observed_paths,
        activation: pack_activation::Activation {
            id,
            server_id: server.id.clone(),
            server_dir: server.server_dir.clone(),
            collection_revision: revision,
            changes,
            collection,
        },
        retained_pack: pack,
        server_changes: server_merge.changes,
        player_changes: player_merge.changes,
        missing,
        warnings,
    };
    let bytes = serde_json::to_vec(&review).map_err(|error| error.to_string())?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Pack review exceeds its size limit".into());
    }
    msc_infrastructure::atomic_write::atomic_write(
        &msc_infrastructure::fs::StdFileSystem,
        &path(server, &review.id)?,
        &bytes,
    )
    .map_err(|error| error.to_string())?;
    Ok(review)
}

pub fn load_reference(server: &ConfigServer, id: &str, owner: &str) -> Result<Review, String> {
    use msc_infrastructure::fs::FileSystem;
    let bytes = msc_infrastructure::fs::StdFileSystem
        .read_bounded(&path(server, id)?, 32 * 1024 * 1024)
        .map_err(|error| error.to_string())?;
    let review: Review = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if review.id != id
        || review.activation.id != id
        || review.owner != owner
        || review.activation.server_id != server.id
        || review.activation.server_dir != server.server_dir
        || review.minecraft_version != server.minecraft_version
        || review.loader_version != server.loader_version
        || review.flavor != format!("{:?}", server.java_flavor)
        || now().saturating_sub(review.created_unix) > 1800
    {
        return Err("Pack review expired or server changed".into());
    }
    Ok(review)
}

pub fn load(server: &ConfigServer, id: &str, owner: &str) -> Result<Review, String> {
    let review = load_reference(server, id, owner)?;
    if review.source_collection != server.content_collection {
        return Err("Pack collection changed; prepare a fresh review".into());
    }
    if management::snapshot(server)?.server_baseline != review.inventory_baseline {
        return Err("Server inventory changed after pack review".into());
    }
    for (path, before) in &review.observed_paths {
        if observed(server, path)? != *before {
            return Err(format!("Server file changed after pack review: {path}"));
        }
    }
    review.activation.collection.validate()?;
    for entry in review.activation.collection.player_files.values() {
        management::player_source(server, entry)?;
    }
    Ok(review)
}

/// The review itself supplies an owner-bound reference to a staged new pack.
/// This does not grant access to arbitrary retained roots or historical packs.
pub fn retained_for_review(
    server: &ConfigServer,
    review: &Review,
) -> Result<pack_retention::RetainedPack, String> {
    pack_retention::root(Path::new(&server.server_dir), &review.retained_pack)?;
    let mut scoped = server.clone();
    let collection = scoped
        .content_collection
        .get_or_insert_with(Default::default);
    if !collection
        .retained_roots
        .contains(&review.retained_pack.relative_root)
    {
        collection
            .retained_roots
            .push(review.retained_pack.relative_root.clone());
    }
    let pack = pack_retention::load(&scoped, &review.retained_pack.id)?;
    if pack.original.sha512 != review.retained_pack.original.sha512
        || pack.original.byte_size != review.retained_pack.original.byte_size
    {
        return Err("Retained original identity changed after pack review".into());
    }
    if serde_json::to_value(&pack.origin).map_err(|error| error.to_string())?
        != serde_json::to_value(&review.retained_pack.origin).map_err(|error| error.to_string())?
    {
        return Err("Retained pack provenance changed after review".into());
    }
    for (path, pinned) in &review.retained_pack.expected_files {
        let current = pack
            .expected_files
            .get(path)
            .ok_or("Reviewed manifest file disappeared")?;
        if current.path != pinned.path
            || current.byte_size != pinned.byte_size
            || current.sha1 != pinned.sha1
            || current.sha512 != pinned.sha512
            || current.source != pinned.source
        {
            return Err("Retained manifest identity changed after pack review".into());
        }
    }
    Ok(pack)
}
