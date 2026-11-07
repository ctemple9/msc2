//! Stream a single audience from registry-verified immutable publication files.
//! No live server directory or provider API is used to reconstruct the bundle.
use crate::content_publications::{self as publications, PublishedFile};
use msc_domain::app_config_schema::ConfigServer;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use sha2::Sha512;
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    path::Path,
};
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveResult {
    pub mrpack_file_name: Option<String>,
    pub file_name: String,
    pub byte_size: u64,
    pub content_bytes: u64,
    pub file_count: usize,
    pub complete: bool,
}
pub(crate) fn options() -> zip::write::SimpleFileOptions {
    zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644)
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
// Keep zip-v1 path and README behavior stable for historical publications.
// A different layout needs a new snapshot layout version, not a silent rewrite.
pub fn export_path_v1(audience: &str, file: &PublishedFile) -> String {
    if audience == "players"
        && (file.path.starts_with("resourcepacks/") || file.path.starts_with("shaderpacks/"))
        && (file.optional || !file.enabled)
    {
        format!("optional-content/{}", file.path)
    } else if !file.enabled
        && (file.path.starts_with("mods/") || file.path.starts_with("plugins/"))
        && file.path.ends_with(".jar")
    {
        format!("{}.disabled", file.path)
    } else {
        file.path.clone()
    }
}
pub(crate) fn text_entry<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    path: &str,
    bytes: &[u8],
) -> Result<(), String> {
    zip.start_file(path, options())
        .map_err(|error| error.to_string())?;
    zip.write_all(bytes).map_err(|error| error.to_string())
}
pub(crate) fn copy_entry<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    source: &Path,
    path: &str,
    file: &PublishedFile,
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(u64),
) -> Result<(), String> {
    zip.start_file(path, options())
        .map_err(|error| error.to_string())?;
    let mut input = std::fs::File::open(source).map_err(|error| error.to_string())?;
    let mut buffer = [0u8; 128 * 1024];
    let mut total = 0u64;
    let mut sha1 = Sha1::new();
    let mut sha512 = Sha512::new();
    loop {
        if cancelled() {
            return Err("Content export cancelled".into());
        }
        let count = input.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > file.byte_size {
            return Err("Frozen file grew during export".into());
        }
        sha1.update(&buffer[..count]);
        sha512.update(&buffer[..count]);
        zip.write_all(&buffer[..count])
            .map_err(|error| error.to_string())?;
        progress(count as u64);
    }
    if total != file.byte_size
        || format!("{:x}", sha1.finalize()) != file.sha1
        || format!("{:x}", sha512.finalize()) != file.sha512
    {
        return Err(format!(
            "Frozen content changed or is unavailable: {}",
            file.path
        ));
    }
    Ok(())
}

pub fn build(
    server: &ConfigServer,
    publication_id: &str,
    audience: &str,
    allow_incomplete: bool,
    destination: &Path,
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<ArchiveResult, String> {
    build_with_options(
        server,
        publication_id,
        audience,
        allow_incomplete,
        false,
        destination,
        cancelled,
        progress,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn build_with_options(
    server: &ConfigServer,
    publication_id: &str,
    audience: &str,
    allow_incomplete: bool,
    include_mrpack: bool,
    destination: &Path,
    cancelled: &dyn Fn() -> bool,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<ArchiveResult, String> {
    use msc_infrastructure::fs::FileSystem;
    if !matches!(audience, "players" | "server") {
        return Err("Choose players or server export".into());
    }
    if cancelled() {
        return Err("Content export cancelled".into());
    }
    let publication = publications::load(server, publication_id)?;
    if publication.archive_layout_version != "zip-v1" {
        return Err("This publication needs an unsupported archive layout version".into());
    }
    if !publication
        .audiences
        .iter()
        .any(|published| published == audience)
    {
        return Err("This publication does not contain the selected audience".into());
    }
    let date = publication
        .created_at
        .get(..10)
        .ok_or("Publication date is invalid")?;
    let files = if audience == "players" {
        &publication.player_files
    } else {
        &publication.server_files
    };
    let issues = crate::content_environment::issues(&publication, audience);
    let exportable = publications::exportable(
        publication.content_fingerprint.starts_with("pack-v1:"),
        &issues,
    );
    if !exportable && !allow_incomplete {
        return Err("Explicit incomplete-export consent is required for this audience".into());
    }
    let mrpack = crate::content_mrpack::eligibility(&publication, audience);
    if include_mrpack && !mrpack.supported {
        return Err(mrpack.reasons.join("; "));
    }
    let mrpack_file_name = include_mrpack.then(|| {
        format!(
            "{}{}-pack.mrpack",
            if issues.is_empty() { "" } else { "incomplete-" },
            audience
        )
    });
    let exclusions: Vec<_> = publication
        .exclusions
        .iter()
        .filter(|issue| issue.audience == audience || issue.audience == "all")
        .collect();
    let content_bytes = files
        .values()
        .try_fold(0u64, |sum, file| sum.checked_add(file.byte_size))
        .ok_or("Export size overflow")?;
    let work_bytes = content_bytes
        .checked_mul(if include_mrpack { 2 } else { 1 })
        .ok_or("Export size overflow")?;
    let mut exported = BTreeMap::new();
    let mut names = std::collections::BTreeSet::new();
    for file in files.values() {
        let path = export_path_v1(audience, file);
        msc_domain::content_collection::validate_relative_path(&path)?;
        if !names.insert(path.to_lowercase()) {
            return Err("Export paths collide across platforms".into());
        }
        exported.insert(path, file);
    }
    let guide_name = if audience == "players" {
        "PLAYER.txt"
    } else {
        "SERVER.txt"
    };
    let guide_path = publications::frozen_guide(server, &publication, guide_name)?;
    let guide = msc_infrastructure::fs::StdFileSystem
        .read_bounded(&guide_path, 16 * 1024 * 1024)
        .map_err(|error| error.to_string())?;
    let guide = String::from_utf8(guide).map_err(|_| "Frozen text guide is not UTF-8")?;
    let guide = if audience == "players"
        && publication.guide_version == "text-v1"
        && matches!(
            publication.loader.as_str(),
            "paper" | "purpur" | "pufferfish" | "spigot"
        ) {
        let player = crate::content_environment::for_publication(&publication);
        format!(
            "PLAYER SETUP CORRECTION FOR ARCHIVED TEXT-V1 GUIDE\nThe archived header below records the server platform. Player loader: {}. Player loader version: {}. Use this player setup when creating a client instance.\n\n{guide}",
            player
                .as_ref()
                .map_or("UNKNOWN — resolve before playing", |setup| setup
                    .loader
                    .as_str()),
            player
                .as_ref()
                .and_then(|setup| setup.loader_version.as_deref())
                .unwrap_or("not required or not recorded")
        )
    } else {
        guide
    };
    let mut text = format!(
        "{}\nPublication {} — revision {}\nAudience: {audience}\nStatus: {}\n\n{guide}\nINACTIVE OPTIONAL CONTENT\nFiles under optional-content/resourcepacks and optional-content/shaderpacks are included but inactive. Copy a chosen pack into the matching resourcepacks or shaderpacks folder only when you want to use it. Files ending in .jar.disabled remain disabled until you deliberately remove .disabled.\n",
        publication.server_name,
        publication.id,
        publication.publication_revision,
        if issues.is_empty() {
            "complete selected content"
        } else if exportable {
            "Missing files are listed in the README; add them before playing"
        } else {
            "INCOMPLETE — read missing-content reasons before joining"
        }
    );
    if !issues.is_empty() && exportable {
        text.push_str("\nFILES TO DOWNLOAD\nThese files were not included because they are not stored locally. Download each exact filename for the Minecraft and loader versions above, then place it at its listed path in the launcher instance (or server folder for a server export) before playing. MSC has not downloaded or substituted these files.\n");
        for issue in issues
            .iter()
            .filter(|issue| publications::is_missing_local_file(issue))
        {
            if let Some(path) = &issue.path {
                text.push_str(&format!("- {path}\n"));
            }
        }
    }
    if let Some(name) = &mrpack_file_name {
        text.push_str(&format!(
            "\nMRPACK IMPORT\n{name} contains the same frozen {audience} content. {}\n{}\n",
            if audience == "players" {
                "Import it into a compatible launcher supporting client-overrides."
            } else {
                "Use a dedicated-server installer supporting server-overrides."
            },
            mrpack.warnings.join("\n")
        ));
    }
    let html = format!(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\"><title>MSC content instructions</title><style>body{{font-family:system-ui,sans-serif;margin:2rem auto;padding:0 1rem;max-width:72ch;line-height:1.5}}h1{{font-size:1.5rem;font-weight:600}}pre{{font:inherit;white-space:pre-wrap;overflow-wrap:anywhere}}</style><main><h1>MSC content instructions</h1><pre>{}</pre></main></html>",
        escape(&text)
    );
    let manifest=serde_json::to_vec_pretty(&serde_json::json!({"mrpackFileName":mrpack_file_name,"playerSetup":crate::content_environment::for_publication(&publication),"schemaVersion":1,"publicationId":publication.id,"publicationRevision":publication.publication_revision,"collectionRevision":publication.collection_revision,"createdAt":publication.created_at,"versionName":publication.version_name,"serverName":publication.server_name,"audience":audience,"minecraftVersion":publication.minecraft_version,"loader":publication.loader,"loaderVersion":publication.loader_version,"guideVersion":publication.guide_version,"complete":issues.is_empty(),"incompleteConsent":allow_incomplete,"issues":issues,"exclusions":exclusions,"files":exported})).map_err(|error|error.to_string())?;
    let parent = destination
        .parent()
        .ok_or("Export destination has no parent")?;
    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let guides_size: u64 = publication
        .guide_assets
        .values()
        .map(|guide| guide.byte_size)
        .sum();
    let required = work_bytes
        .checked_add(work_bytes / 100)
        .and_then(|n| n.checked_add(guides_size))
        .and_then(|n| {
            n.checked_add((files.len() as u64 + 10) * if include_mrpack { 4096 } else { 1024 })
        })
        .and_then(|n| {
            n.checked_add(
                text.len() as u64
                    + html.len() as u64
                    + manifest.len() as u64 * if include_mrpack { 3 } else { 1 },
            )
        })
        .ok_or("Export size overflow")?;
    if fs2::available_space(parent).map_err(|error| error.to_string())? < required {
        return Err("Insufficient space for selected content export".into());
    }
    if destination
        .try_exists()
        .map_err(|error| error.to_string())?
    {
        return Err("Export destination already exists".into());
    }
    let temporary = parent.join(format!(".{}.part", uuid::Uuid::new_v4()));
    let mut open = std::fs::OpenOptions::new();
    open.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open.mode(0o600);
    }
    let output = open.open(&temporary).map_err(|error| error.to_string())?;
    let result = (|| {
        let mut zip = zip::ZipWriter::new(output);
        text_entry(&mut zip, "README.txt", text.as_bytes())?;
        text_entry(&mut zip, "README.html", html.as_bytes())?;
        text_entry(&mut zip, "manifest.json", &manifest)?;
        let mut written = 0u64;
        progress(0, work_bytes);
        for (path, file) in &exported {
            let source = publications::frozen_file_location(server, &publication, file)?;
            copy_entry(&mut zip, &source, path, file, cancelled, &mut |delta| {
                written += delta;
                progress(written, work_bytes);
            })?;
        }
        if let Some(name) = &mrpack_file_name {
            zip.start_file(
                name,
                options()
                    .compression_method(zip::CompressionMethod::Stored)
                    .large_file(true),
            )
            .map_err(|error| error.to_string())?;
            crate::content_mrpack::write(
                &mut zip,
                server,
                &publication,
                audience,
                allow_incomplete,
                cancelled,
                &mut |delta| {
                    written += delta;
                    progress(written, work_bytes);
                },
            )?;
        }
        for path in publication.guide_assets.keys() {
            if path == "PLAYER.txt" && audience != "players"
                || path == "SERVER.txt" && audience != "server"
            {
                continue;
            }
            if cancelled() {
                return Err("Content export cancelled".into());
            }
            let source = publications::frozen_guide(server, &publication, path)?;
            let bytes = msc_infrastructure::fs::StdFileSystem
                .read_bounded(&source, 16 * 1024 * 1024)
                .map_err(|error| error.to_string())?;
            text_entry(&mut zip, &format!("instructions/{path}"), &bytes)?;
        }
        let output = zip.finish().map_err(|error| error.to_string())?;
        output.sync_all().map_err(|error| error.to_string())?;
        drop(output);
        if cancelled() {
            return Err("Content export cancelled before completion".into());
        }
        std::fs::hard_link(&temporary, destination).map_err(|error| error.to_string())?;
        #[cfg(unix)]
        {
            std::fs::File::open(parent)
                .and_then(|file| file.sync_all())
                .map_err(|error| {
                    format!("Export file completed but directory sync failed: {error}")
                })?;
        }
        let byte_size = std::fs::metadata(destination)
            .map_err(|error| error.to_string())?
            .len();
        let label: String = publication
            .server_name
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                    ch
                } else {
                    '-'
                }
            })
            .take(60)
            .collect();
        let label = if label.trim_matches('-').is_empty() {
            "server"
        } else {
            label.trim_matches('-')
        };
        Ok(ArchiveResult {
            mrpack_file_name,
            file_name: format!(
                "msc-{label}-{}-r{}-{audience}.zip",
                date, publication.publication_revision
            ),
            byte_size,
            content_bytes,
            file_count: files.len(),
            complete: issues.is_empty(),
        })
    })();
    if let Err(cleanup) = std::fs::remove_file(&temporary)
        && cleanup.kind() != std::io::ErrorKind::NotFound
    {
        return Err(format!(
            "{}; partial ZIP cleanup could not be confirmed: {cleanup}",
            result
                .err()
                .unwrap_or_else(|| "ZIP generation completed".into())
        ));
    }
    result
}
