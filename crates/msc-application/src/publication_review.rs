//! Eligibility review binds owner approvals to exact bytes. Download permission
//! alone is never used as evidence that a file may be redistributed in a ZIP.
use crate::content_management as management;
use crate::content_publications::{PreparedPublicationFile, PublicationIssue, PublishedFile};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::content_collection::{ContentAudience, ContentCollection, ContentEntry};
use msc_infrastructure::addon_provider::fingerprint::identify;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Approval {
    pub audience: String,
    pub path: String,
    pub sha512: String,
    pub redistribution_evidence: String,
    #[serde(default)]
    pub reviewed_configuration: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Exclusion {
    pub audience: String,
    pub path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    #[serde(default)]
    pub pack_export: bool,
    #[serde(default = "default_audiences")]
    pub audiences: Vec<String>,
    pub collection_revision: u64,
    pub version_name: Option<String>,
    pub player_setup: Option<crate::content_environment::PlayerSetup>,
    #[serde(default)]
    pub approvals: Vec<Approval>,
    #[serde(default)]
    pub excluded: Vec<Exclusion>,
    #[serde(default)]
    pub server_paths: Vec<String>,
    #[serde(default)]
    pub datapacks: Vec<crate::publication_datapacks::Selection>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub audience: String,
    pub path: String,
    pub sha512: String,
    pub byte_size: u64,
    pub optional: bool,
    pub configuration: bool,
    pub restricted: bool,
    pub redistribution_evidence: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub id: String,
    pub owner: String,
    pub server_id: String,
    pub server_dir: String,
    pub minecraft_version: Option<String>,
    pub loader_version: Option<String>,
    pub flavor: String,
    pub created_unix: u64,
    pub source_collection: Option<ContentCollection>,
    pub inventory_baseline: BTreeMap<String, String>,
    pub content_fingerprint: String,
    pub player_setup: Option<crate::content_environment::PlayerSetup>,
    pub request: Request,
    pub files: Vec<PreparedPublicationFile>,
    #[serde(default)]
    pub candidates: Vec<Candidate>,
    pub issues: Vec<PublicationIssue>,
    pub exclusions: Vec<PublicationIssue>,
}
fn default_audiences() -> Vec<String> {
    vec!["players".into(), "server".into()]
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn path(server: &ConfigServer, id: &str) -> Result<std::path::PathBuf, String> {
    uuid::Uuid::parse_str(id).map_err(|_| "Invalid publication review identity")?;
    crate::content_install::managed_path(
        &server.server_dir,
        &format!(".msc2-content/publication-reviews/{id}.json"),
    )
}
fn scoped(audience: &str, path: &str) -> Result<String, String> {
    if !matches!(audience, "server" | "players") {
        return Err("Choose server or players".into());
    }
    management::content_path(path)?;
    Ok(format!("{audience}/{path}"))
}
fn issue(
    audience: &str,
    path: Option<&str>,
    required: bool,
    reason: impl Into<String>,
) -> PublicationIssue {
    PublicationIssue {
        audience: audience.into(),
        path: path.map(str::to_string),
        required,
        reason: reason.into(),
    }
}
fn config(path: &str) -> bool {
    matches!(
        path.split('/').next(),
        Some("config" | "scripts" | "kubejs" | "defaultconfigs")
    )
}

pub fn fingerprint(
    server: &ConfigServer,
    server_paths: &[String],
    datapacks: &[crate::publication_datapacks::Selection],
) -> Result<String, String> {
    let snapshot = management::snapshot(server)?;
    fingerprint_snapshot(server, &snapshot, server_paths, datapacks)
}

fn fingerprint_snapshot(
    server: &ConfigServer,
    snapshot: &management::Snapshot,
    server_paths: &[String],
    datapacks: &[crate::publication_datapacks::Selection],
) -> Result<String, String> {
    let datapack_files = selected_datapacks(server, datapacks, false, &|| false)?;
    let datapack_hashes: BTreeMap<_, _> = datapack_files
        .into_iter()
        .map(|file| (format!("{}/{}", file.slot_id, file.path), file.sha512))
        .collect();
    let mut selected = BTreeMap::new();
    for path in server_paths {
        management::content_path(path)?;
        let observed = crate::content_install::managed_path(&server.server_dir, path)
            .and_then(|path| identify(&path));
        selected.insert(
            path,
            observed
                .map(|file| file.sha512)
                .unwrap_or_else(|_| "unavailable".into()),
        );
    }
    let value = serde_json::json!({"serverName":server.display_name,"minecraft":server.minecraft_version,"loader":server.java_flavor.raw_value(),"loaderVersion":server.loader_version,"serverFiles":snapshot.collection.current_files,"playerFiles":snapshot.collection.player_files,"requirements":snapshot.requirements,"observed":snapshot.server_baseline,"selected":selected,"datapacks":datapack_hashes});
    let bytes = serde_json::to_vec(&value).map_err(|error| error.to_string())?;
    Ok(format!("{:x}", Sha512::digest(bytes)))
}

pub fn pack_fingerprint(
    server: &ConfigServer,
    server_paths: &[String],
    datapacks: &[crate::publication_datapacks::Selection],
) -> Result<String, String> {
    let mut snapshot = management::snapshot(server)?;
    crate::pack_export_selection::project(server, &mut snapshot, &|| false)?;
    Ok(format!(
        "pack-v1:{}",
        fingerprint_snapshot(server, &snapshot, server_paths, datapacks)?
    ))
}

pub fn prepare(
    server: &ConfigServer,
    owner: &str,
    request: Request,
    cancelled: &dyn Fn() -> bool,
) -> Result<Review, String> {
    prepare_with_progress(server, owner, request, cancelled, &|_, _, _| {})
}

pub fn prepare_with_progress(
    server: &ConfigServer,
    owner: &str,
    request: Request,
    cancelled: &dyn Fn() -> bool,
    progress: &dyn Fn(u64, u64, &str),
) -> Result<Review, String> {
    if request
        .version_name
        .as_ref()
        .is_some_and(|name| name.len() > 120 || name.chars().any(char::is_control))
    {
        return Err(
            "Publication version name must be at most 120 bytes with no control characters".into(),
        );
    }
    if request.approvals.len()
        + request.excluded.len()
        + request.server_paths.len()
        + request.datapacks.len()
        > 50_000
    {
        return Err("Too many publication selections".into());
    }
    let audiences: BTreeSet<_> = request.audiences.iter().map(String::as_str).collect();
    if audiences.is_empty()
        || audiences.len() != request.audiences.len()
        || !audiences
            .iter()
            .all(|audience| matches!(*audience, "players" | "server"))
    {
        return Err("Choose players, server, or both publication audiences".into());
    }
    if !request.pack_export
        && !audiences.contains("server")
        && (!request.server_paths.is_empty() || !request.datapacks.is_empty())
    {
        return Err("Server paths/datapacks require the server publication audience".into());
    }
    if let Some(setup) = &request.player_setup {
        setup.validate()?;
        if !audiences.contains("players") {
            return Err("Player setup requires the player publication audience".into());
        }
    }
    progress(0, 100, "Checking server content");
    let mut snapshot = management::snapshot_with_progress(
        server,
        &|done, total, label| progress(done * 35 / total.max(1), 100, label),
        cancelled,
    )?;

    progress(35, 100, "Finding retained client content and owner changes");
    if request.pack_export {
        crate::pack_export_selection::project_with_progress(
            server,
            &mut snapshot,
            cancelled,
            &|done, total, label| progress(35 + done * 5 / total.max(1), 100, label),
        )?;
    }
    if request.collection_revision != snapshot.collection.revision {
        return Err("Content changed; refresh publication review".into());
    }
    let mut approvals = BTreeMap::new();
    for approval in &request.approvals {
        let key = scoped(&approval.audience, &approval.path)?;
        if !audiences.contains(approval.audience.as_str()) {
            return Err("Approval belongs to an unselected audience".into());
        }
        if approval.sha512.len() != 128
            || !approval.sha512.bytes().all(|byte| byte.is_ascii_hexdigit())
            || approval.redistribution_evidence.trim().is_empty()
            || approval.redistribution_evidence.len() > 2048
            || approval
                .redistribution_evidence
                .chars()
                .any(char::is_control)
        {
            return Err(
                "Approval needs an exact SHA512 and concise redistribution evidence".into(),
            );
        }
        if approvals.insert(key, approval).is_some() {
            return Err("Duplicate publication approval".into());
        }
    }
    let mut excluded = BTreeSet::new();
    for entry in &request.excluded {
        if !excluded.insert(scoped(&entry.audience, &entry.path)?) {
            return Err("Duplicate publication exclusion".into());
        }
    }
    let mut server_files = snapshot.collection.current_files.clone();
    for path in &request.server_paths {
        management::content_path(path)?;
        if !config(path) {
            return Err("Additional server paths must be explicitly reviewed configuration or scripts; mods come from inventory".into());
        }
        let source = crate::content_install::managed_path(&server.server_dir, path)?;
        let identity = identify(&source)?;
        server_files
            .entry(path.clone())
            .or_insert_with(|| ContentEntry {
                path: path.clone(),
                sha512: identity.sha512,
                byte_size: identity.byte_size,
                audience: ContentAudience::Server,
                optional: false,
                source: None,
                redistribution_evidence: None,
                classification_evidence: vec![
                    "Owner selected this server configuration file".into(),
                ],
                unresolved_reasons: Vec::new(),
                extra: BTreeMap::new(),
            });
    }
    let has_player_mods = snapshot.collection.player_files.values().any(|entry| {
        entry.path.starts_with("mods/")
            && !entry.path.ends_with(".disabled")
            && entry
                .extra
                .get("enabled")
                .and_then(serde_json::Value::as_bool)
                != Some(false)
    });
    let player_setup = request.player_setup.clone().or_else(|| {
        crate::content_environment::infer(
            server.java_flavor.raw_value(),
            server.loader_version.as_deref(),
            has_player_mods,
        )
    });
    let mut files = Vec::new();
    let mut candidates = Vec::new();
    let mut issues = Vec::new();
    let mut exclusions = Vec::new();
    if audiences.contains("players") {
        match &player_setup {
            None=>issues.push(issue("players",None,true,"Exact player loader/version needs review; a server platform alone does not identify a loader for independent client mods")),
            Some(setup) if setup.loader=="vanilla" && has_player_mods=>issues.push(issue("players",None,true,"Vanilla player setup cannot load the selected client mod JARs")),
            Some(setup) if matches!(server.java_flavor.raw_value(),"fabric"|"forge"|"neoforge"|"quilt") && setup.loader!=server.java_flavor.raw_value() && snapshot.collection.current_files.values().any(|entry|entry.audience==ContentAudience::Both && entry.extra.get("serverEnabled").and_then(serde_json::Value::as_bool)!=Some(false))=>issues.push(issue("players",None,true,"Player loader differs from the server while shared server mods are selected")),
            _=>{},
        }
    }
    if !server
        .minecraft_version
        .as_deref()
        .is_some_and(crate::content_environment::exact_version)
    {
        issues.push(issue(
            "all",
            None,
            true,
            "Exact Minecraft version is unknown",
        ));
    }
    if snapshot
        .collection
        .extra
        .get("playerRecoveryRequired")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
        && audiences.contains("players")
    {
        issues.push(issue(
            "players",
            None,
            true,
            "Missing original client content has not been recovered",
        ));
    }
    for requirement in snapshot
        .requirements
        .iter()
        .filter(|_| audiences.contains("players"))
    {
        if management::content_path(&requirement.path).is_err()
            || (requirement.state == "different" && !requirement.path.starts_with("mods/"))
        {
            continue;
        }
        if !matches!(
            requirement.state.as_str(),
            "matched" | "serverOnly" | "serverDisabled" | "optionalMissing"
        ) {
            issues.push(issue(
                "players",
                Some(&requirement.path),
                true,
                requirement.reason.clone(),
            ));
        }
    }
    if let Some(missing) = snapshot
        .collection
        .extra
        .get("retainedPackUnresolved")
        .and_then(serde_json::Value::as_object)
    {
        for (path, reason) in missing.iter().filter(|_| audiences.contains("players")) {
            issues.push(issue(
                "players",
                Some(path),
                true,
                reason
                    .as_str()
                    .unwrap_or("Retained pack file is unresolved"),
            ));
        }
    }
    let entry_total = (if audiences.contains("server") {
        server_files.len()
    } else {
        0
    } + if audiences.contains("players") {
        snapshot.collection.player_files.len()
    } else {
        0
    })
    .max(1) as u64;
    let mut inspected = 0u64;
    for (audience, entries) in [
        ("server", &server_files),
        ("players", &snapshot.collection.player_files),
    ] {
        if !audiences.contains(audience) {
            continue;
        }
        for (path, entry) in entries {
            progress(
                40 + inspected * 50 / entry_total,
                100,
                &format!(
                    "Verifying export file {}/{}: {path}",
                    inspected + 1,
                    entry_total
                ),
            );
            inspected += 1;
            if cancelled() {
                return Err("Publication review cancelled".into());
            }
            if management::content_path(path).is_err() {
                exclusions.push(issue(
                    audience,
                    Some(path),
                    false,
                    "Outside approved content folders; runtime/world/private data is excluded",
                ));
                continue;
            }
            if (audience == "players" && entry.audience == ContentAudience::Server)
                || (audience == "server" && entry.audience == ContentAudience::Players)
            {
                exclusions.push(issue(
                    audience,
                    Some(path),
                    false,
                    "Content belongs to the other audience",
                ));
                continue;
            }
            candidates.push(Candidate {
                audience: audience.into(),
                path: path.clone(),
                sha512: entry.sha512.clone(),
                byte_size: entry.byte_size,
                optional: entry.optional,
                configuration: config(path),
                restricted: entry
                    .extra
                    .get("providerDistributionAllowed")
                    .and_then(serde_json::Value::as_bool)
                    == Some(false),
                redistribution_evidence: entry.redistribution_evidence.clone(),
            });
            let key = scoped(audience, path)?;
            if !request.pack_export
                && audience == "players"
                && path.starts_with("mods/")
                && !entry.optional
                && entry
                    .extra
                    .get("enabled")
                    .and_then(serde_json::Value::as_bool)
                    == Some(false)
            {
                issues.push(issue(
                    audience,
                    Some(path),
                    true,
                    "Required player mod is disabled",
                ));
            }
            if excluded.contains(&key) {
                exclusions.push(issue(
                    audience,
                    Some(path),
                    false,
                    "Owner excluded this file",
                ));
                if !entry.optional {
                    issues.push(issue(
                        audience,
                        Some(path),
                        true,
                        "Required content was explicitly excluded",
                    ));
                }
                continue;
            }
            if entry.classification_evidence.is_empty() {
                issues.push(issue(
                    audience,
                    Some(path),
                    !entry.optional,
                    "Review this file's server/player audience before publication",
                ));
                continue;
            }
            if !entry.unresolved_reasons.is_empty() {
                issues.push(issue(
                    audience,
                    Some(path),
                    !entry.optional,
                    format!(
                        "Content review remains unresolved: {}",
                        entry.unresolved_reasons.join("; ")
                    ),
                ));
                continue;
            }
            if audience == "server" && path.starts_with("datapacks/") {
                exclusions.push(issue(
                    audience,
                    Some(path),
                    false,
                    "Datapacks are published only through explicit world-slot selections",
                ));
                continue;
            }
            let approval = approvals.get(&key).copied();
            if approval.is_some_and(|approval| approval.sha512 != entry.sha512) {
                return Err(format!("Approval bytes changed: {key}"));
            }
            if config(path)
                && !approval.is_some_and(|approval| approval.reviewed_configuration)
                && !(request.pack_export
                    && (entry
                        .extra
                        .get("originalPackContent")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)))
            {
                issues.push(issue(
                    audience,
                    Some(path),
                    !entry.optional,
                    "Explicitly review this configuration/script file before including it",
                ));
                continue;
            }
            if entry
                .extra
                .get("providerDistributionAllowed")
                .and_then(serde_json::Value::as_bool)
                == Some(false)
            {
                issues.push(issue(
                    audience,
                    Some(path),
                    !entry.optional,
                    "Known provider distribution restriction; bytes will not be embedded",
                ));
                continue;
            }
            let evidence = approval
                .map(|approval| approval.redistribution_evidence.clone())
                .or_else(|| entry.redistribution_evidence.clone())
                .filter(|evidence| !evidence.trim().is_empty());
            let Some(evidence) = evidence else {
                issues.push(issue(audience,Some(path),!entry.optional,"Redistribution eligibility needs review; download authorization alone is insufficient"));
                continue;
            };
            let server_disabled = audience == "server"
                && entry
                    .extra
                    .get("serverEnabled")
                    .and_then(serde_json::Value::as_bool)
                    == Some(false);
            let relative = if audience == "players" {
                entry
                    .extra
                    .get("retainedPath")
                    .and_then(serde_json::Value::as_str)
                    .ok_or("Player source path disappeared")?
                    .to_string()
            } else if server_disabled && !path.ends_with(".jar.disabled") {
                format!("{path}.disabled")
            } else {
                path.clone()
            };
            let source = match crate::content_publications::checked_source(server, &relative) {
                Ok(source) => source,
                Err(error) => {
                    issues.push(issue(audience, Some(path), !entry.optional, error));
                    continue;
                }
            };
            let actual = match identify(&source) {
                Ok(identity) => identity,
                Err(error) => {
                    issues.push(issue(audience, Some(path), !entry.optional, error));
                    continue;
                }
            };
            if actual.sha512 != entry.sha512 || actual.byte_size != entry.byte_size {
                return Err(format!("Content changed during publication review: {key}"));
            }
            files.push(PreparedPublicationFile {
                audience: audience.into(),
                source_relative: relative,
                file: PublishedFile {
                    path: path.clone(),
                    sha512: actual.sha512,
                    sha1: actual.sha1,
                    byte_size: actual.byte_size,
                    optional: entry.optional,
                    enabled: !server_disabled
                        && !path.ends_with(".jar.disabled")
                        && entry
                            .extra
                            .get("enabled")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(
                                !path.starts_with("resourcepacks/")
                                    && !path.starts_with("shaderpacks/"),
                            ),
                    source: entry.source.clone(),
                    classification_evidence: entry.classification_evidence.clone(),
                    redistribution_evidence: evidence,
                    world_slot: entry
                        .extra
                        .get("worldSlot")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string),
                },
            });
        }
    }
    progress(90, 100, "Verifying selected world datapacks");
    let datapack_files = selected_datapacks(server, &request.datapacks, true, cancelled)?;
    let datapack_grants: BTreeMap<_, _> = request
        .datapacks
        .iter()
        .map(|selection| {
            (
                (selection.slot_id.as_str(), selection.path.as_str()),
                selection,
            )
        })
        .collect();
    for datapack in datapack_files {
        for audience in &request.audiences {
            let path = format!("datapacks/{}/{}", datapack.slot_id, datapack.path);
            management::content_path(&path)?;
            let selection = datapack_grants
                .get(&(datapack.slot_id.as_str(), datapack.path.as_str()))
                .ok_or("Datapack selection disappeared")?;
            if excluded.contains(&format!("{audience}/{path}")) {
                exclusions.push(issue(
                    audience,
                    Some(&path),
                    false,
                    "Owner excluded selected world datapack content",
                ));
                issues.push(issue(
                    audience,
                    Some(&path),
                    true,
                    "Selected world datapack content was excluded",
                ));
                continue;
            }
            candidates.push(Candidate {
                audience: audience.clone(),
                path: path.clone(),
                sha512: datapack.sha512.clone(),
                byte_size: datapack.byte_size,
                optional: false,
                configuration: false,
                restricted: false,
                redistribution_evidence: None,
            });
            if selection.redistribution_evidence.trim().is_empty() {
                issues.push(issue(
                    audience,
                    Some(&path),
                    true,
                    "Redistribution permission is needed for this world datapack",
                ));
                continue;
            }
            files.push(PreparedPublicationFile {
                audience: audience.clone(),
                source_relative: datapack
                    .source_relative
                    .clone()
                    .ok_or("Selected datapack was not retained")?,
                file: PublishedFile {
                    path,
                    sha512: datapack.sha512.clone(),
                    sha1: datapack.sha1.clone(),
                    byte_size: datapack.byte_size,
                    optional: false,
                    enabled: true,
                    source: None,
                    classification_evidence: vec![
                        "Owner selected content from this world slot".into(),
                    ],
                    redistribution_evidence: selection.redistribution_evidence.clone(),
                    world_slot: Some(datapack.slot_id.clone()),
                },
            });
        }
    }
    progress(95, 100, "Recording export contents");
    let fingerprint =
        fingerprint_snapshot(server, &snapshot, &request.server_paths, &request.datapacks)?;
    let content_fingerprint = if request.pack_export {
        format!("pack-v1:{fingerprint}")
    } else {
        fingerprint
    };
    let review = Review {
        id: uuid::Uuid::new_v4().to_string(),
        owner: owner.into(),
        server_id: server.id.clone(),
        server_dir: server.server_dir.clone(),
        minecraft_version: server.minecraft_version.clone(),
        loader_version: server.loader_version.clone(),
        flavor: server.java_flavor.raw_value().into(),
        created_unix: now(),
        source_collection: server.content_collection.clone(),
        inventory_baseline: snapshot.server_baseline,
        content_fingerprint,
        player_setup,
        request,
        files,
        candidates,
        issues,
        exclusions,
    };
    let bytes = serde_json::to_vec(&review).map_err(|error| error.to_string())?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Publication review exceeds its size limit".into());
    }
    msc_infrastructure::atomic_write::atomic_write(
        &msc_infrastructure::fs::StdFileSystem,
        &path(server, &review.id)?,
        &bytes,
    )
    .map_err(|error| error.to_string())?;
    progress(100, 100, "Export contents ready");
    Ok(review)
}
pub fn load(server: &ConfigServer, id: &str, owner: &str) -> Result<Review, String> {
    read_review(server, id, owner, true)
}

/// Preview reads keep identity/revision/expiry checks. Exact current bytes are
/// revalidated by load at publication commit, not on every display fetch.
pub fn preview(server: &ConfigServer, id: &str, owner: &str) -> Result<Review, String> {
    read_review(server, id, owner, false)
}
fn read_review(
    server: &ConfigServer,
    id: &str,
    owner: &str,
    verify_bytes: bool,
) -> Result<Review, String> {
    use msc_infrastructure::fs::FileSystem;
    let bytes = msc_infrastructure::fs::StdFileSystem
        .read_bounded(&path(server, id)?, 32 * 1024 * 1024)
        .map_err(|error| error.to_string())?;
    let review: Review = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if review.id != id
        || review.owner != owner
        || review.server_id != server.id
        || review.server_dir != server.server_dir
        || review.source_collection != server.content_collection
        || review.minecraft_version != server.minecraft_version
        || review.loader_version != server.loader_version
        || review.flavor != server.java_flavor.raw_value()
        || now().saturating_sub(review.created_unix) > 1800
    {
        return Err("Publication review expired or server changed".into());
    }
    if !verify_bytes {
        return Ok(review);
    }
    let current_fingerprint = if review.request.pack_export {
        pack_fingerprint(
            server,
            &review.request.server_paths,
            &review.request.datapacks,
        )?
    } else {
        fingerprint(
            server,
            &review.request.server_paths,
            &review.request.datapacks,
        )?
    };
    if current_fingerprint != review.content_fingerprint {
        return Err("Content bytes or options changed after publication review".into());
    }
    for prepared in &review.files {
        let source =
            crate::content_publications::checked_source(server, &prepared.source_relative)?;
        let actual = identify(&source)?;
        if actual.sha512 != prepared.file.sha512
            || actual.sha1 != prepared.file.sha1
            || actual.byte_size != prepared.file.byte_size
        {
            return Err(format!("Reviewed file changed: {}", prepared.file.path));
        }
    }
    Ok(review)
}

fn selected_datapacks(
    server: &ConfigServer,
    selections: &[crate::publication_datapacks::Selection],
    retain: bool,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<crate::publication_datapacks::File>, String> {
    let mut grouped: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for selection in selections {
        msc_domain::content_collection::validate_relative_path(&selection.path)?;
        if selection.sha512.len() != 128
            || !selection
                .sha512
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || selection.redistribution_evidence.len() > 2048
            || selection
                .redistribution_evidence
                .chars()
                .any(char::is_control)
        {
            return Err("Selected datapack needs exact bytes and redistribution evidence".into());
        }
        if !grouped
            .entry(selection.slot_id.clone())
            .or_default()
            .insert(selection.path.clone())
        {
            return Err("Duplicate world datapack selection".into());
        }
    }
    let grants: BTreeMap<_, _> = selections
        .iter()
        .map(|selection| {
            (
                (selection.slot_id.as_str(), selection.path.as_str()),
                selection,
            )
        })
        .collect();
    let mut files = Vec::new();
    for (slot, paths) in grouped {
        let inspected =
            crate::publication_datapacks::inspect(server, &slot, Some(&paths), retain, cancelled)?;
        let inspected: BTreeMap<_, _> = inspected
            .into_iter()
            .map(|file| (file.path.clone(), file))
            .collect();
        for path in paths {
            let file = inspected
                .get(&path)
                .ok_or_else(|| format!("Selected datapack is missing: {slot}/{path}"))?;
            let selection = grants
                .get(&(slot.as_str(), path.as_str()))
                .ok_or("Datapack selection disappeared")?;
            if file.sha512 != selection.sha512 {
                return Err(format!("Selected datapack changed: {slot}/{path}"));
            }
            files.push(file.clone());
        }
    }
    Ok(files)
}
