//! P7.21: the template workflows over P7.15's `template_store` — list
//! Paper/plugin templates in the shape `TemplatesResponseDTO` needs,
//! export the active server as a template, and create a new server from
//! a template. Ports `AppViewModel+APIWiringServerMgmt.swift`'s
//! `buildTemplatesResponse` (line 287-331) and
//! `templateMutationProvider`'s `"exportServer"`/`"createServer"` cases
//! (line 339-424), plus the small pure display-title helpers
//! `PaperTemplateItem.displayTitle`/`PluginTemplateItem.displayTitle`
//! (`TemplateItemDisplay.swift`) neither of which any earlier phase
//! ported, against the three `fixtures/jar-templates/` cases P7.15's own
//! module doc left here: `jar-summary-geyser-floodgate-*`,
//! `export-server-as-template-*`, `create-server-from-template-*`.
//!
//! **`jar_summary`'s date is not formatted here.** The fixture's own
//! `expected` shows a fully formatted English label
//! (`"... — Mar 1, 2026 at 12:00 AM"`), but that's
//! `DateFormatter.localizedString`'s locale-dependent rendering baked
//! into MSC 1's own Swift view code — this crate has no locale
//! infrastructure, and `TemplateItemDTO.modifiedAt` (the frozen contract
//! this same data ultimately feeds) already carries a raw timestamp for
//! each client to format itself, not a pre-rendered string. This module
//! returns the raw `SystemTime` (or `None`, ported as the
//! `SystemTime::UNIX_EPOCH` sentinel `msc_infrastructure::fs` already
//! uses for "no readable modification date" — see `fs.rs`'s own
//! `unwrap_or(SystemTime::UNIX_EPOCH)`) and leaves formatting to the
//! caller. The *selection* logic the fixture actually characterizes —
//! newest-by-modification-date wins, an undated candidate is only a
//! last resort — is ported exactly.
//!
//! **Corrections to this step's own plan text**, already flagged by
//! P7.6/P7.15 and reconfirmed here by reading `templateMutationProvider`
//! end to end: export-as-template has **no running-server refusal** in
//! source (`case "exportServer":`, line 339-391) — `applyPaperTemplate
//! ToSelectedServer`'s refusal is a different function for a different
//! action. `includePlugins` defaults to `true` when omitted, but that
//! default is the DTO/route layer's job (P7.23), not this module's —
//! [`export_server_as_template`] takes an already-resolved `bool`.

use crate::provisioning::{self, CreateServerError, CreatedServer, NewServerRequest, WorldSource};
use msc_domain::identity::JavaServerFlavor;
use msc_domain::provisioning::ImportedWorldMetadata;
use msc_domain::version::parse_paper_jar_filename;
use msc_infrastructure::fs::{FileSystem, join_forward_slash};
use msc_infrastructure::path_safety;
use msc_infrastructure::template_store::{self, TemplateStoreError};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// One entry in the shape `TemplatesResponseDTO.paperTemplates`/
/// `.pluginTemplates` need — `templateIdFor`/`buildTemplatesResponse`'s
/// per-item mapping (source line 236-238, 298-322) composed directly
/// onto `template_store::TemplateFile`, so nothing here re-derives what
/// that module already parsed.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateListItem {
    /// `"<kind>:<filename>"` (`templateIdFor`, line 236-238).
    pub id: String,
    pub kind: &'static str,
    pub filename: String,
    pub display_name: String,
    pub size_bytes: u64,
    pub modified: SystemTime,
    pub version: Option<String>,
    pub build: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ServerTemplates {
    pub paper: Vec<TemplateListItem>,
    pub plugin: Vec<TemplateListItem>,
}

/// `buildTemplatesResponse`'s two listing loops (source line 298-322),
/// minus `serverName`/`serverRunning` — those come from the caller's own
/// fleet/lifecycle state (P7.23's job), not this store.
pub fn list_server_templates(
    fs: &dyn FileSystem,
    paper_template_dir: &Path,
    plugin_template_dir: &Path,
    home_dir: &Path,
) -> Result<ServerTemplates, TemplateStoreError> {
    let paper = template_store::list_templates(fs, paper_template_dir, home_dir)?
        .into_iter()
        .map(|t| TemplateListItem {
            id: format!("paper:{}", t.filename),
            kind: "paper",
            display_name: paper_template_display_title(&t.filename),
            version: t.version.clone(),
            build: t.build,
            filename: t.filename,
            size_bytes: t.size_bytes,
            modified: t.modified,
        })
        .collect();
    let plugin = template_store::list_templates(fs, plugin_template_dir, home_dir)?
        .into_iter()
        .map(|t| TemplateListItem {
            id: format!("plugin:{}", t.filename),
            kind: "plugin",
            display_name: plugin_template_display_title(&t.filename),
            version: None,
            build: None,
            filename: t.filename,
            size_bytes: t.size_bytes,
            modified: t.modified,
        })
        .collect();
    Ok(ServerTemplates { paper, plugin })
}

/// `PaperTemplateItem.displayTitle` (`TemplateItemDisplay.swift:42-83`).
/// No `fixtures/jar-templates` case names this function directly; ported
/// straight from source (three patterns: `paper-<v>-build<n>`,
/// `paper-<v>-<n>`, and a non-`paper-` fallback to the bare stem) since
/// `TemplateItemDTO.displayName` is a required field this phase's
/// contract needs filled honestly, not left blank.
fn paper_template_display_title(filename: &str) -> String {
    let base = strip_jar_extension(filename);
    let Some(rest) = base.strip_prefix("paper-") else {
        return base.to_string();
    };
    let components: Vec<&str> = rest.split('-').collect();
    if components.len() == 2 {
        let version = components[0];
        let second = components[1];
        let second_lower = second.to_lowercase();
        if let Some(build_str) = second_lower.strip_prefix("build")
            && !build_str.is_empty()
        {
            let real_build = &second[second.len() - build_str.len()..];
            return format!("Paper {version} (build {real_build})");
        }
        return format!("Paper {version} (build {second})");
    }
    format!("Paper {rest}")
}

/// `PluginTemplateItem.displayTitle` (`TemplateItemDisplay.swift:9-40`):
/// hides a `-latest` suffix, then treats a trailing digit-bearing `-`
/// segment as a version. Same "no fixture, ported for contract
/// completeness" rationale as [`paper_template_display_title`].
fn plugin_template_display_title(filename: &str) -> String {
    let raw_base = strip_jar_extension(filename);
    let base = replace_ignore_ascii_case(raw_base, "-latest", "");
    let parts: Vec<&str> = base.split('-').collect();
    if parts.len() < 2 {
        return base;
    }
    let last = parts[parts.len() - 1];
    let has_digits = last.chars().any(|c| c.is_ascii_digit());
    if has_digits {
        let name = parts[..parts.len() - 1].join("-");
        format!("{name} ({last})")
    } else {
        base
    }
}

fn strip_jar_extension(filename: &str) -> &str {
    filename
        .strip_suffix(".jar")
        .or_else(|| filename.strip_suffix(".JAR"))
        .unwrap_or(filename)
}

fn replace_ignore_ascii_case(haystack: &str, needle: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(haystack.len());
    let mut rest = haystack;
    while let Some(idx) = rest.to_lowercase().find(&needle.to_lowercase()) {
        result.push_str(&rest[..idx]);
        result.push_str(replacement);
        rest = &rest[idx + needle.len()..];
    }
    result.push_str(rest);
    result
}

/// `PaperVersionSidecarManager.read` (`PaperVersionSidecar.swift:24-34`):
/// `None` on a missing file, unreadable bytes, or malformed/incomplete
/// JSON — `Codable`'s `PaperVersionSidecar` requires all three keys
/// (`mcVersion`, `build`, `timestamp`) to decode at all, so a sidecar
/// missing `timestamp` is exactly as invalid as one missing `mcVersion`;
/// `timestamp`'s own value is checked for presence only, never read,
/// since no caller here needs it.
fn read_paper_version_sidecar(fs: &dyn FileSystem, server_dir: &Path) -> Option<(String, i64)> {
    let bytes = fs.read(&server_dir.join(".msc_paper_version.json")).ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let mc_version = value.get("mcVersion")?.as_str()?.to_string();
    let build = value.get("build")?.as_i64()?;
    value.get("timestamp")?.as_str()?;
    Some((mc_version, build))
}

/// One `jarSummary(for:)` result (`AppViewModel+Templates.swift:518-
/// 568`): the server's own Paper jar plus the newest Geyser/Floodgate
/// template already sitting in its `plugins/` folder — display-only
/// data, no on-disk change.
#[derive(Debug, Clone, PartialEq)]
pub struct JarSummary {
    pub paper_filename: String,
    pub geyser: Option<JarSummaryEntry>,
    pub floodgate: Option<JarSummaryEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JarSummaryEntry {
    pub filename: String,
    pub modified: Option<SystemTime>,
}

/// `jarSummary(for:)`. `paper_jar_path` mirrors `cfgServer.paperJarPath`
/// exactly (`launch_shape::jar_basename`'s same empty-path fallback to
/// `"paper.jar"`, line 525-530). The geyser/floodgate pick (line 546-
/// 568) is order-independent by construction: a dated candidate always
/// beats an undated one and the newest dated candidate always wins; an
/// undated candidate is only kept if nothing — dated or undated — has
/// been picked yet, matching source's `else if xURLForSummary == nil`
/// exactly (not "if the current best is undated", which would let a
/// *later* undated candidate replace an *earlier* one, something source
/// never does).
pub fn jar_summary(
    fs: &dyn FileSystem,
    plugins_dir: &Path,
    home_dir: &Path,
    paper_jar_path: &str,
) -> Result<JarSummary, TemplateStoreError> {
    let paper_filename = msc_domain::launch_shape::jar_basename(paper_jar_path);
    let entries = template_store::list_templates(fs, plugins_dir, home_dir)?;
    Ok(JarSummary {
        paper_filename,
        geyser: pick_summary_jar(&entries, "geyser"),
        floodgate: pick_summary_jar(&entries, "floodgate"),
    })
}

fn pick_summary_jar(
    entries: &[template_store::TemplateFile],
    prefix_lower: &str,
) -> Option<JarSummaryEntry> {
    let mut best: Option<&template_store::TemplateFile> = None;
    for entry in entries {
        let stem_lower = strip_jar_extension(&entry.filename).to_lowercase();
        if !stem_lower.starts_with(prefix_lower) {
            continue;
        }
        let has_date = entry.modified != SystemTime::UNIX_EPOCH;
        match best {
            None => best = Some(entry),
            Some(cur) => {
                let cur_has_date = cur.modified != SystemTime::UNIX_EPOCH;
                if has_date && (!cur_has_date || entry.modified > cur.modified) {
                    best = Some(entry);
                }
            }
        }
    }
    best.map(|entry| JarSummaryEntry {
        filename: entry.filename.clone(),
        modified: (entry.modified != SystemTime::UNIX_EPOCH).then_some(entry.modified),
    })
}

/// What [`export_server_as_template`] did — always "succeeds" (there is
/// no failure return in source; every copy failure is caught, logged,
/// and skipped, per this module's own doc note on the running-server
/// wording correction).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExportedTemplate {
    pub exported_count: usize,
}

/// `templateMutationProvider`'s `"exportServer"` case (source line 339-
/// 391). The Paper jar's destination filename comes from the server's
/// own `.msc_paper_version.json` sidecar when one exists
/// (`paper-<mc>-build<build>.jar`), falling back to the source jar's own
/// filename otherwise (line 353-361) — genuinely different from
/// `template_store::archive_jar`'s flavor-driven naming, which this
/// function deliberately does not call. Every individual copy is
/// best-effort: a missing Paper jar, a missing `plugins/` directory, or
/// a single plugin copy failure only lowers `exported_count`, never
/// returns an error (source's own per-item `do { } catch { log }`).
#[allow(clippy::too_many_arguments)]
pub fn export_server_as_template(
    fs: &dyn FileSystem,
    home_dir: &Path,
    paper_template_dir: &Path,
    plugin_template_dir: &Path,
    server_dir: &Path,
    paper_jar_path: &str,
    is_java: bool,
    include_plugins: bool,
) -> ExportedTemplate {
    let mut exported = 0usize;

    if is_java {
        let source = server_dir.join(msc_domain::launch_shape::jar_basename(paper_jar_path));
        let is_file = fs.stat(&source).map(|m| m.is_file).unwrap_or(false);
        if is_file && template_store::ensure_template_dir(fs, paper_template_dir).is_ok() {
            let dest_name = read_paper_version_sidecar(fs, server_dir)
                .map(|(mc_version, build)| format!("paper-{mc_version}-build{build}.jar"))
                .unwrap_or_else(|| {
                    source
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("paper.jar")
                        .to_string()
                });
            if let Ok(bytes) = fs.read(&source)
                && let Ok(dest) =
                    path_safety::safe_path(fs, paper_template_dir, Some(&dest_name), home_dir)
                && fs.write(&dest, &bytes).is_ok()
            {
                exported += 1;
            }
        }
    }

    if include_plugins {
        let plugins_dir = server_dir.join("plugins");
        if template_store::ensure_template_dir(fs, plugin_template_dir).is_ok() {
            for jar in list_jar_files(fs, &plugins_dir) {
                let Some(filename) = jar.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if template_store::copy_into_server_dir(
                    fs,
                    &jar,
                    plugin_template_dir,
                    home_dir,
                    filename,
                )
                .is_ok()
                {
                    exported += 1;
                }
            }
        }
    }

    ExportedTemplate {
        exported_count: exported,
    }
}

/// `.jar`, non-hidden files directly inside `dir` — `contentsOfDirectory
/// (..., options: .skipsHiddenFiles).filter { $0.pathExtension...==
/// "jar" }` (source line 372), a plain directory scan with no template-
/// store natural-sort/version-parse overhead this call site doesn't
/// need. `Vec::new()` (not an error) when `dir` doesn't exist, matching
/// source's `try?`.
fn list_jar_files(fs: &dyn FileSystem, dir: &Path) -> Vec<PathBuf> {
    fs.list(dir)
        .unwrap_or_default()
        .into_iter()
        .filter(|path| {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                return false;
            };
            !name.starts_with('.') && name.to_lowercase().ends_with(".jar")
        })
        .collect()
}

/// `templateFlavorForFilename` (`AppViewModel+APIWiringServerMgmt.swift:
/// 244-251`): a plain filename-prefix sniff, defaulting to Paper for
/// everything it doesn't otherwise recognize — including a literal
/// `paper-...` name, but also anything unrecognized, matching source's
/// own `return .paper` fallthrough exactly rather than making it look
/// like a deliberate Paper match.
pub fn template_flavor_for_filename(filename: &str) -> JavaServerFlavor {
    let lower = filename.to_lowercase();
    if lower.starts_with("purpur-") {
        JavaServerFlavor::Purpur
    } else if lower.starts_with("pufferfish") {
        JavaServerFlavor::Pufferfish
    } else if lower.starts_with("minecraft_server-") {
        JavaServerFlavor::Vanilla
    } else if lower.starts_with("fabric-server-launch") {
        JavaServerFlavor::Fabric
    } else {
        JavaServerFlavor::Paper
    }
}

/// `templateMutationProvider`'s `"createServer"` case (source line 393-
/// 421), reusing [`crate::provisioning::finish_server_creation`] for
/// everything after the jar is in place — the same shared tail P7.17/
/// P7.18 already compose. World source is always `Fresh`: source's own
/// caller hardcodes `worldSource: .fresh` (line 420) and this action has
/// no way to request anything else, so [`CreateFromTemplateRequest`] has
/// no `world_source` field to be silently ignored.
#[derive(Debug, Clone)]
pub struct CreateFromTemplateRequest<'a> {
    pub name: &'a str,
    pub initial_world_name: Option<&'a str>,
    pub port: u16,
    pub enable_cross_play: bool,
    pub cross_play_bedrock_port: Option<u16>,
    pub enable_playit: bool,
    pub difficulty: &'a str,
    pub gamemode: &'a str,
    pub world_seed: Option<&'a str>,
    pub default_banner_color_hex: &'a str,
}

#[allow(clippy::too_many_arguments)]
pub fn create_server_from_template(
    fs: &dyn FileSystem,
    home_dir: &Path,
    servers_root: &Path,
    plugin_template_dir: &Path,
    template_path: &Path,
    template_filename: &str,
    request: &CreateFromTemplateRequest,
    now: &str,
) -> Result<CreatedServer, CreateServerError> {
    let flavor = template_flavor_for_filename(template_filename);

    let safe_name = msc_domain::provisioning::trimmed_server_name(request.name)
        .ok_or(CreateServerError::EmptyName)?;
    let initial_slot_name =
        provisioning::initial_world_slot_name(&safe_name, request.initial_world_name);
    let imported_metadata = ImportedWorldMetadata::default();
    let normalized_world_seed =
        provisioning::normalized_initial_world_seed(request.world_seed, &WorldSource::Fresh);
    let effective = msc_domain::provisioning::effective_world_settings(
        request.difficulty,
        request.gamemode,
        normalized_world_seed.as_deref(),
        &imported_metadata,
    );
    let initial_level_name =
        msc_domain::world::sanitized_world_level_name(&initial_slot_name, "world");

    let folder_name = msc_domain::provisioning::folder_name_from_safe_name(&safe_name);
    // See the matching comment in `provisioning.rs::create_new_server` --
    // same Windows mixed-separator gap in the same `new_dir`/`jar_dest`
    // shape, found by P7.29's own Windows CI leg.
    let new_dir = join_forward_slash(
        &join_forward_slash(servers_root, std::ffi::OsStr::new("java")),
        folder_name.as_ref(),
    );

    if fs.stat(&new_dir).is_ok() {
        return Err(CreateServerError::FolderAlreadyExists {
            folder_name,
            path: new_dir,
        });
    }
    fs.create_dir_all(&new_dir)?;

    let outcome = (|| -> Result<CreatedServer, CreateServerError> {
        let primary_jar_filename = msc_domain::provisioning::primary_jar_filename(flavor);
        let jar_dest = join_forward_slash(&new_dir, std::ffi::OsStr::new(primary_jar_filename));
        let bytes = fs.read(template_path)?;
        let dest = path_safety::safe_path(fs, &new_dir, Some(primary_jar_filename), home_dir)?;
        fs.write(&dest, &bytes)?;

        let parsed = parse_paper_jar_filename(template_filename);
        if let Some(p) = &parsed {
            provisioning::write_paper_version_sidecar(fs, &new_dir, &p.mc_version, p.build, now);
        }
        let resolved_version = parsed.as_ref().map(|p| p.mc_version.clone());
        let resolved_build = parsed.as_ref().map(|p| p.build.to_string());

        let inner_request = NewServerRequest {
            name: request.name,
            initial_world_name: request.initial_world_name,
            flavor,
            port: request.port,
            enable_cross_play: request.enable_cross_play,
            cross_play_bedrock_port: request.cross_play_bedrock_port,
            enable_playit: request.enable_playit,
            // Not exposed by `templateMutationProvider`'s `"createServer"`
            // case (source line 408-421 passes no `enableXboxBroadcast`
            // argument at all), so `createNewServer`'s own default (`=
            // false`, line 139) applies.
            enable_xbox_broadcast: false,
            difficulty: request.difficulty,
            gamemode: request.gamemode,
            world_seed: request.world_seed,
            initial_world_profile: None,
            world_source: WorldSource::Fresh,
            // `save_downloaded_jars` only gates [`provisioning::acquire_jar`]'s
            // own archive step, which this jar-source (a local template
            // copy, not a download) never calls.
            save_downloaded_jars: false,
            default_banner_color_hex: request.default_banner_color_hex,
        };

        provisioning::finish_server_creation(
            fs,
            home_dir,
            plugin_template_dir,
            &new_dir,
            &inner_request,
            &safe_name,
            &initial_slot_name,
            &initial_level_name,
            &effective,
            &imported_metadata,
            &jar_dest.to_string_lossy(),
            resolved_version.as_deref(),
            resolved_build.as_deref(),
            None,
            now,
            |_, _| false,
            |_, _, _| false,
        )
    })();

    if outcome.is_err() {
        let _ = fs.remove(&new_dir);
    }
    outcome
}
