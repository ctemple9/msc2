//! Recovery constructs player metadata and private retained bytes only. It
//! never invokes the server importer or installs an archive's server scripts.
use crate::{content_management as management, pack_retention};
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::content_collection::ContentAudience;
use msc_infrastructure::addon_provider::AddonTransport;
use msc_infrastructure::secret_store::SecretStore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryResult {
    pub review: management::Review,
    pub conflicts: Vec<String>,
    pub missing: Vec<String>,
    pub pack_id: Option<String>,
}
pub(crate) fn compatible(
    server: &ConfigServer,
    inspection: &crate::modpacks::ModpackInspection,
) -> Result<(), String> {
    use msc_domain::modpack_manifest::{self, LoaderFlavor};
    let (game, flavor, loader) = match &inspection.format {
        crate::modpacks::InspectedFormat::Mrpack(manifest) => {
            let metadata = modpack_manifest::mrpack_metadata(manifest);
            (
                metadata.minecraft_version,
                metadata.loader_flavor,
                metadata.loader_version,
            )
        }
        crate::modpacks::InspectedFormat::CurseForge(metadata) => (
            Some(metadata.minecraft_version.clone()),
            metadata.loader_flavor,
            metadata.loader_version.clone(),
        ),
        _ => return Err("Choose the original .mrpack or CurseForge client pack".into()),
    };
    let expected = match server.java_flavor {
        msc_domain::identity::JavaServerFlavor::Fabric => Some(LoaderFlavor::Fabric),
        msc_domain::identity::JavaServerFlavor::Forge => Some(LoaderFlavor::Forge),
        msc_domain::identity::JavaServerFlavor::NeoForge => Some(LoaderFlavor::NeoForge),
        _ => None,
    };
    if game.is_none() || game != server.minecraft_version || flavor.is_none() || flavor != expected
    {
        return Err(
            "Original pack must match the existing server's Minecraft version and loader".into(),
        );
    }
    if loader.is_some() && server.loader_version.is_some() && loader != server.loader_version {
        return Err(
            "Original pack uses another loader version; choose the matching original release"
                .into(),
        );
    }
    if let (Some(current), Some(origin)) = (
        server
            .content_collection
            .as_ref()
            .and_then(|collection| collection.upstream.as_ref()),
        inspection.origin.as_ref(),
    ) && (current.provider != origin.source.provider
        || current.project_id != origin.source.project_id)
    {
        return Err(
            "This is a different base pack; use replacement review instead of recovery".into(),
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn review(
    server: &ConfigServer,
    owner: &str,
    revision: u64,
    selected_paths: &[String],
    restore_paths: &[String],
    inspection: Option<&crate::modpacks::ModpackInspection>,
    retained_pack: Option<pack_retention::RetainedPack>,
    transport: &dyn AddonTransport,
    secrets: &dyn SecretStore,
    cancelled: &dyn Fn() -> bool,
) -> Result<RecoveryResult, String> {
    if selected_paths.len() + restore_paths.len() > 512 {
        return Err("Select at most 512 explicit recovery choices".into());
    }
    let snapshot = management::snapshot(server)?;
    if snapshot.collection.revision != revision {
        return Err("Content changed; refresh recovery review".into());
    }
    let mut result = snapshot.collection.clone();
    let mut warnings=vec!["Recovery does not reinstall server files. An installed mod list cannot reconstruct absent client-only content.".into()];
    let mut conflicts = Vec::new();
    let mut missing = Vec::new();
    let mut pack_id = None;
    if let Some(inspection) = inspection {
        compatible(server, inspection)?;
        let mut pack = match retained_pack {
            Some(pack) => pack,
            None => pack_retention::capture_original(
                inspection,
                Path::new(&server.server_dir),
                cancelled,
            )?
            .ok_or("No original pack archive was supplied")?,
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
            return Err("Recovery cancelled".into());
        }
        let mut recovered = pack_retention::collection(
            Path::new(&server.server_dir),
            &pack,
            Some(&snapshot.collection),
        )?;
        // Retain the observed running-server state. Exact matches may gain
        // provenance, but a differing archive file is never written or claimed.
        recovered.current_files = snapshot.collection.current_files.clone();
        for (path, baseline) in &recovered.upstream_files {
            if let Some(current) = recovered.current_files.get_mut(path) {
                if current.sha512 == baseline.sha512 {
                    if current.source.is_none() {
                        current.source = baseline.source.clone();
                    }
                    if current.classification_evidence.is_empty()
                        && !baseline.classification_evidence.is_empty()
                    {
                        current.audience = baseline.audience;
                        current.classification_evidence = baseline.classification_evidence.clone();
                        current.unresolved_reasons.retain(|reason| {
                            !reason.starts_with("Review client/server classification")
                        });
                    }
                } else {
                    conflicts.push(format!("Server customization preserved: {path}"));
                }
            } else {
                missing.push(format!("Server differs from original pack: {path}"));
            }
        }
        for (path, current) in &snapshot.collection.player_files {
            if recovered
                .upstream_player_files
                .get(path)
                .is_some_and(|upstream| upstream.sha512 != current.sha512)
            {
                recovered.player_files.insert(path.clone(), current.clone());
                conflicts.push(format!("Player customization preserved: {path}"));
            }
        }
        for path in &snapshot.collection.player_excluded_paths {
            if !restore_paths.contains(path) {
                recovered.player_files.remove(path);
            }
        }
        for (path, reason) in &pack.unresolved {
            missing.push(format!("{path}: {reason}"));
        }
        if pack
            .origin
            .as_ref()
            .is_some_and(|origin| origin.server_pack == Some(true))
        {
            missing.push("The supplied archive is a server pack; its missing client content is not proven recovered".into());
        }
        if pack.origin.is_none() {
            if let Some(previous) = recovered.upstream.take() {
                recovered.extra.insert(
                    "previousUpstreamBeforeRecovery".into(),
                    serde_json::to_value(previous).map_err(|error| error.to_string())?,
                );
            }
            recovered.extra.insert(
                "recoveryOriginUnverified".into(),
                serde_json::Value::Bool(true),
            );
        }
        warnings.push("A local archive's title alone does not prove it is the original pack; inspect matched files and preserved conflicts before applying".into());
        pack_id = Some(pack.id);
        result = recovered;
    } else if selected_paths.is_empty() && restore_paths.is_empty() {
        return Err("Select installed files to retain or supply the original client pack".into());
    }
    let mut selected = BTreeSet::new();
    for path in selected_paths {
        if cancelled() {
            return Err("Recovery cancelled".into());
        }
        if !selected.insert(path) {
            return Err("Duplicate recovery path".into());
        }
        management::content_path(path)?;
        let server_entry = snapshot
            .collection
            .current_files
            .get(path)
            .ok_or("Selected server content is absent")?;
        if server_entry.audience == ContentAudience::Server
            && !server_entry.classification_evidence.is_empty()
        {
            return Err(format!(
                "{path} is confirmed server-only; classify it explicitly before adding it to players"
            ));
        }
        if result
            .player_files
            .get(path)
            .is_some_and(|current| current.sha512 != server_entry.sha512)
        {
            conflicts.push(format!("Player customization preserved: {path}"));
            continue;
        }
        let source = crate::content_install::managed_path(&server.server_dir, path)?;
        let mut retained = management::retain_player(server, path, &source)?;
        retained.source = server_entry.source.clone();
        retained.optional = server_entry.optional;
        retained.redistribution_evidence = server_entry.redistribution_evidence.clone();
        retained.extra.extend(
            server_entry
                .extra
                .iter()
                .filter(|(key, _)| key.as_str() != "retainedPath")
                .map(|(key, value)| (key.clone(), value.clone())),
        );
        if server_entry.classification_evidence.is_empty() {
            retained.unresolved_reasons.push("Original client/server requirements are unknown; owner classification review remains required".into());
            retained.classification_evidence.clear();
        }
        result.player_files.insert(path.clone(), retained);
        result
            .player_excluded_paths
            .retain(|excluded| excluded != path);
    }
    for path in restore_paths {
        let upstream = result
            .upstream_player_files
            .get(path)
            .ok_or("No retained upstream file for restoration")?
            .clone();
        management::player_source(server, &upstream)?;
        result.player_files.insert(path.clone(), upstream);
        result
            .player_excluded_paths
            .retain(|excluded| excluded != path);
    }
    if !selected_paths.is_empty()
        && !result
            .retained_roots
            .iter()
            .any(|root| root == ".msc2-content/player")
    {
        result.retained_roots.push(".msc2-content/player".into());
    }
    if inspection.is_none() && result.upstream.is_none() {
        result.extra.insert(
            "playerRecoveryRequired".into(),
            serde_json::Value::Bool(true),
        );
    }
    result.extra.insert(
        "recoverySummary".into(),
        serde_json::json!({"conflicts":conflicts,"missing":missing,"packId":pack_id}),
    );
    warnings.extend(conflicts.iter().cloned());
    warnings.extend(missing.iter().cloned());
    let review = management::create_review(
        server,
        owner,
        snapshot.server_baseline,
        result,
        Vec::new(),
        warnings,
    )?;
    Ok(RecoveryResult {
        review,
        conflicts,
        missing,
        pack_id,
    })
}
