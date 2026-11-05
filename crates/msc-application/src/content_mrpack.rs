//! .mrpack v1 uses approved embedded overrides from the frozen publication.
//! No credentials, mutable provider URLs or cross-provider substitutions are
//! necessary to reconstruct a historical package. Native downloads[] is empty
//! because all included content is carried by the side-specific override layer.
use crate::{
    content_archives as archives, content_environment as environment,
    content_publications::{self as publications, Publication},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::Write};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Eligibility {
    pub supported: bool,
    pub reasons: Vec<String>,
    pub warnings: Vec<String>,
    pub dependencies: BTreeMap<String, String>,
    pub embedded_file_count: usize,
    pub complete: bool,
}
pub fn eligibility(publication: &Publication, audience: &str) -> Eligibility {
    let mut reasons = Vec::new();
    let mut warnings = Vec::new();
    let mut dependencies = BTreeMap::new();
    if !matches!(audience, "players" | "server")
        || !publication.audiences.iter().any(|scope| scope == audience)
    {
        reasons.push("Publication does not contain the selected audience".into());
    }
    if publication.archive_layout_version != "zip-v1" {
        reasons.push("Publication archive layout is not supported by this .mrpack writer".into());
    }
    if let Some(minecraft) = publication
        .minecraft_version
        .as_deref()
        .filter(|version| environment::exact_version(version))
    {
        dependencies.insert("minecraft".into(), minecraft.into());
    } else {
        reasons.push("An exact Minecraft version is required for .mrpack".into());
    }
    let files = if audience == "players" {
        &publication.player_files
    } else {
        &publication.server_files
    };
    let setup = if audience == "players" {
        environment::for_publication(publication)
    } else {
        let setup = environment::PlayerSetup {
            loader: publication.loader.clone(),
            loader_version: publication.loader_version.clone(),
        };
        setup.validate().ok().map(|()| setup)
    };
    if let Some(setup) = setup {
        let key = match setup.loader.as_str() {
            "fabric" => Some("fabric-loader"),
            "quilt" => Some("quilt-loader"),
            "forge" => Some("forge"),
            "neoforge" => Some("neoforge"),
            "vanilla" => None,
            _ => None,
        };
        if let Some(key) = key {
            if let Some(version) = setup.loader_version {
                dependencies.insert(key.into(), version);
            }
        } else if files.values().any(|file| {
            file.path.starts_with("mods/") && file.enabled && !file.path.ends_with(".disabled")
        }) {
            reasons.push(
                "Vanilla setup cannot load active mod JARs; publish a reviewed mod-loader setup"
                    .into(),
            );
        }
    } else {
        reasons.push(if audience=="server" {"The selected server platform or unknown loader version cannot be faithfully represented by .mrpack; use the content ZIP"} else {"Exact player loader/version is unknown; publish a reviewed player setup"}.into());
    }
    if files.keys().any(|path| path.starts_with("plugins/")) {
        reasons.push(
            "Plugin bundles are content ZIPs; .mrpack does not describe a plugin server platform"
                .into(),
        );
    }
    if files.keys().any(|path| path.starts_with("datapacks/")) {
        reasons.push("Selected world-slot datapacks need explicit target-world placement; use the ZIP instructions".into());
    }
    if files
        .values()
        .any(|file| file.optional && file.path.starts_with("mods/"))
    {
        warnings.push("Owner-selected optional mods are included; enabled ones load by default; review the manifest before disabling any of them".into());
    }
    if audience == "server" {
        warnings
            .push("Use a dedicated-server .mrpack installer that supports server-overrides".into());
    } else {
        warnings.push("Use a compatible .mrpack launcher that supports client-overrides; launcher-specific acceptance remains separate".into());
    }
    warnings.push(
        "Game and loader installation may still require the launcher's normal downloads".into(),
    );
    Eligibility {
        supported: reasons.is_empty(),
        reasons,
        warnings,
        dependencies,
        embedded_file_count: files.len(),
        complete: environment::issues(publication, audience).is_empty(),
    }
}

/// Write directly into the outer ZIP's stored .mrpack entry. The nested writer
/// uses ZIP data descriptors, avoiding both an inner temporary file and a second
/// full-pack memory buffer. Every payload is hashed again while streaming.
pub fn write<W: Write>(
    target: W,
    server: &msc_domain::app_config_schema::ConfigServer,
    publication: &Publication,
    audience: &str,
    allow_incomplete: bool,
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(u64),
) -> Result<(), String> {
    let eligibility = eligibility(publication, audience);
    if !eligibility.supported {
        return Err(eligibility.reasons.join("; "));
    }
    if !eligibility.complete && !allow_incomplete {
        return Err("Explicit incomplete consent is required for the nested .mrpack".into());
    }
    let files = if audience == "players" {
        &publication.player_files
    } else {
        &publication.server_files
    };
    let layer = if audience == "players" {
        "client-overrides"
    } else {
        "server-overrides"
    };
    let mut records = BTreeMap::new();
    let mut paths = std::collections::BTreeSet::new();
    for file in files.values() {
        let path = archives::export_path_v1(audience, file);
        msc_domain::content_collection::validate_relative_path(&path)?;
        if !paths.insert(path.to_lowercase()) {
            return Err("Nested .mrpack override paths collide".into());
        }
        records.insert(path,serde_json::json!({"sourcePath":file.path,"hashes":{"sha1":file.sha1,"sha512":file.sha512},"fileSize":file.byte_size,"optional":file.optional,"enabled":file.enabled,"source":file.source,"env":{"client":if audience=="players" {if file.optional {"optional"}else{"required"}}else{"unsupported"},"server":if audience=="server" {if file.optional {"optional"}else{"required"}}else{"unsupported"}}}));
    }
    let issues = environment::issues(publication, audience);
    let index=serde_json::to_vec_pretty(&serde_json::json!({"formatVersion":1,"game":"minecraft","versionId":format!("{}-{audience}",publication.id),"name":if eligibility.complete {format!("{} — {} content",publication.server_name,audience)}else{format!("INCOMPLETE — {} — {} content",publication.server_name,audience)},"summary":if eligibility.complete {"MSC frozen content publication"}else{"Incomplete MSC publication; read msc-manifest.json before playing"},"files":[],"dependencies":eligibility.dependencies})).map_err(|error|error.to_string())?;
    let manifest=serde_json::to_vec_pretty(&serde_json::json!({"schemaVersion":1,"publicationId":publication.id,"publicationRevision":publication.publication_revision,"collectionRevision":publication.collection_revision,"audience":audience,"complete":eligibility.complete,"issues":issues,"distributionMode":"embeddedOverrides","warnings":eligibility.warnings,"files":records})).map_err(|error|error.to_string())?;
    if manifest.len() > 64 * 1024 * 1024 {
        return Err("Nested .mrpack metadata exceeds its size limit".into());
    }
    let readme=format!("MSC {} CONTENT\\nPublication {}\\n\\n{}\\n\\nThe game/loader dependencies are pinned in modrinth.index.json. Content bytes are embedded in this package. Optional shader/resource packs remain under optional-content and require a deliberate move into the matching game folder. Files ending in .jar.disabled remain inactive; confirm the compatible loader before enabling them. Enabled optional mods load by default. Read msc-manifest.json for exact paths, SHA1/SHA512, side/optional flags and missing-content reasons.\\n",audience.to_uppercase(),publication.id,if eligibility.complete {"Selected content is complete."}else{"INCOMPLETE: resolve the missing content listed in msc-manifest.json before playing."}).replace("\\n","\n");
    let mut zip = zip::ZipWriter::new_stream(target);
    archives::text_entry(&mut zip, "modrinth.index.json", &index)?;
    archives::text_entry(&mut zip, "msc-manifest.json", &manifest)?;
    archives::text_entry(&mut zip, "README-MSC.txt", readme.as_bytes())?;
    archives::text_entry(
        &mut zip,
        &format!("{layer}/msc-instructions/README.txt"),
        readme.as_bytes(),
    )?;
    for file in files.values() {
        if cancelled() {
            return Err("Nested .mrpack export cancelled".into());
        }
        let source = publications::frozen_file_location(server, publication, file)?;
        archives::copy_entry(
            &mut zip,
            &source,
            &format!("{layer}/{}", archives::export_path_v1(audience, file)),
            file,
            cancelled,
            progress,
        )?;
    }
    if cancelled() {
        return Err("Nested .mrpack export cancelled before completion".into());
    }
    drop(zip.finish().map_err(|error| error.to_string())?);
    Ok(())
}
