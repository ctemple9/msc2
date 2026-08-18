//! The jar archive and template store: the filesystem-backed half of
//! `AppViewModel+Templates.swift`'s Paper/plugin template directories
//! (`paperTemplateDir`/`pluginTemplateDir`, already carried by
//! `AppConfig` per `msc_domain::app_config_schema`) and
//! `archiveServerJar` (`AppViewModel+ServerCreation.swift:622-660`).
//!
//! Characterized by `fixtures/jar-templates/` (P7.6). Seven of its ten
//! cases are this store's job and are exercised directly by
//! `tests/template_store.rs`: the four `archive-jar-*` cases, both
//! `latest-template-*` cases, and `template-listing-sorted-*`. The
//! other three — `jar-summary-geyser-floodgate-*`,
//! `export-server-as-template-*`, and `create-server-from-template-*` —
//! need a `ConfigServer`, a running-server check, and (for export) a
//! second directory (`plugins/`) this store has no opinion about; they
//! are `msc-application`'s job (P7.21), composing this module's
//! primitives rather than duplicating them.
//!
//! Every path this module resolves against a template directory goes
//! through [`crate::path_safety::safe_path`] — "over approved roots" per
//! this step's own plan text — even though no `fixtures/jar-templates`
//! case exercises an escape attempt (every version/build string in the
//! oracle's own `archiveServerJar` comes from an already-parsed provider
//! response, never from free user text). Symlinks resolved and `..`
//! collapsed for free, at the cost of one extra parameter
//! (`home_dir`) every function here already threads the same way
//! `msc-application::import::import_raw_server` does.

use crate::fs::FileSystem;
use crate::path_safety::{PathSafetyError, safe_path};
use msc_domain::identity::JavaServerFlavor;
use msc_domain::version::parse_paper_jar_filename;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug)]
pub enum TemplateStoreError {
    Io(io::Error),
    PathSafety(PathSafetyError),
}

impl fmt::Display for TemplateStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateStoreError::Io(e) => write!(f, "{e}"),
            TemplateStoreError::PathSafety(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for TemplateStoreError {}

impl From<io::Error> for TemplateStoreError {
    fn from(e: io::Error) -> Self {
        TemplateStoreError::Io(e)
    }
}

impl From<PathSafetyError> for TemplateStoreError {
    fn from(e: PathSafetyError) -> Self {
        TemplateStoreError::PathSafety(e)
    }
}

/// One `.jar` in a template directory, as `loadPaperTemplates`/
/// `loadPluginTemplates` see it, plus the version/build
/// [`parse_paper_jar_filename`] can read back out of a Paper-shaped
/// filename — `None` for any other naming pattern, including every
/// plugin template (`ComponentVersionParsing` has no Purpur/Vanilla/
/// Fabric reader, and no `fixtures/jar-templates` case asks for one).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateFile {
    pub filename: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified: SystemTime,
    pub version: Option<String>,
    pub build: Option<i64>,
}

/// `!fm.fileExists(atPath:isDirectory:) || !isDir` (source lines 34-43 /
/// 214-223): a missing or non-directory template dir is created, not an
/// error — the ordinary state the first time MSC 2 ever lists templates.
pub fn ensure_template_dir(fs: &dyn FileSystem, dir: &Path) -> io::Result<()> {
    match fs.stat(dir) {
        Ok(meta) if meta.is_dir => Ok(()),
        _ => fs.create_dir_all(dir),
    }
}

/// `.filter { $0.pathExtension.lowercased() == "jar" }` plus
/// `.skipsHiddenFiles` (both `loadPaperTemplates` and
/// `loadPluginTemplates` pass this option to `contentsOfDirectory`),
/// unsorted — callers pick the ordering that matches their own oracle
/// function (`list_templates`'s natural sort vs `latest_template`'s raw
/// one; see this module's doc for why those two genuinely differ).
fn scan_jar_files(
    fs: &dyn FileSystem,
    dir: &Path,
    home_dir: &Path,
) -> Result<Vec<TemplateFile>, TemplateStoreError> {
    let resolved_dir = safe_path(fs, dir, None, home_dir)?;
    ensure_template_dir(fs, &resolved_dir)?;

    let mut entries = Vec::new();
    for path in fs.list(&resolved_dir)? {
        let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if filename.starts_with('.') {
            continue;
        }
        if !filename.to_lowercase().ends_with(".jar") {
            continue;
        }
        let meta = fs.stat(&path)?;
        if !meta.is_file {
            continue;
        }
        let parsed = parse_paper_jar_filename(filename);
        entries.push(TemplateFile {
            filename: filename.to_string(),
            path,
            size_bytes: meta.size,
            modified: meta.modified,
            version: parsed.as_ref().map(|p| p.mc_version.clone()),
            build: parsed.map(|p| p.build),
        });
    }
    Ok(entries)
}

/// `loadPaperTemplates`/`loadPluginTemplates`'s listing order
/// (`fixtures/jar-templates/template-listing-sorted-localized-case-
/// insensitive-ascending-not-lexicographic.json`): case-insensitive
/// *and* digit-run-aware, so `paper-1.21.4-...` sorts before
/// `paper-1.21.10-...` the way a person reading the list would expect —
/// genuinely different from [`latest_template`]'s raw compare below.
pub fn list_templates(
    fs: &dyn FileSystem,
    dir: &Path,
    home_dir: &Path,
) -> Result<Vec<TemplateFile>, TemplateStoreError> {
    let mut entries = scan_jar_files(fs, dir, home_dir)?;
    entries.sort_by(|a, b| natural_case_insensitive_compare(&a.filename, &b.filename));
    Ok(entries)
}

/// `latestTemplate(in:prefixLowercased:)` (source line 764-779):
/// `jars.filter { base.hasPrefix(prefix) }.sorted { $0 < $1 }.last` — a
/// **raw** string compare, not [`list_templates`]'s natural one. Matches
/// `fixtures/jar-templates/latest-template-picks-lexicographically-last-
/// matching-prefix.json`'s own point exactly: `paper-1.21.4-...` can
/// lose to `paper-1.21.10-...` here even though the digit-aware listing
/// sort would rank them the other way. `prefix_lowercased` is matched
/// against the filename's extension-stripped stem, lowercased, the same
/// as source's `base`.
pub fn latest_template(
    fs: &dyn FileSystem,
    dir: &Path,
    home_dir: &Path,
    prefix_lowercased: &str,
) -> Result<Option<TemplateFile>, TemplateStoreError> {
    let mut candidates: Vec<TemplateFile> = scan_jar_files(fs, dir, home_dir)?
        .into_iter()
        .filter(|entry| jar_stem_lower(&entry.filename).starts_with(prefix_lowercased))
        .collect();
    candidates.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(candidates.into_iter().next_back())
}

fn jar_stem_lower(filename: &str) -> String {
    filename
        .strip_suffix(".jar")
        .or_else(|| filename.strip_suffix(".JAR"))
        .unwrap_or(filename)
        .to_lowercase()
}

/// What [`archive_jar`] did — mirrors `archiveServerJar`'s three
/// possible endings: copied, silently skipped because it's already
/// there, or silently skipped because this flavor has no archive
/// filename pattern at all (source's `default: return`, or Paper's own
/// `guard let buildInt = Int(result.build) else { return }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArchiveOutcome {
    Archived { filename: String },
    AlreadyArchived { filename: String },
    Skipped(ArchiveSkipReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveSkipReason {
    /// `default: return` (source line 640-641) — every flavor this
    /// switch doesn't name: NeoForge, Forge, Spigot, Quilt, Pufferfish.
    UnsupportedFlavor,
    /// Paper's own guard (source line 632) — `result.build` didn't parse
    /// as an `Int`.
    NonNumericPaperBuild,
}

/// `archiveServerJar(flavor:result:from:)` (source line 622-660):
/// derives the archive filename per flavor, skips silently (no error,
/// matching source's silent `return`s) if the flavor has none or Paper's
/// build string isn't numeric, skips (also silently, but logged in
/// source) if that filename is already archived, and otherwise copies
/// `source_path` into `archive_dir` under it. `archive_dir` is always
/// the Paper template directory in source — Purpur/Vanilla/Fabric land
/// in the same bucket as Paper, there is no separate one per flavor
/// (confirmed by reading `archiveServerJar` itself, which builds
/// `archiveDir` once from `configManager.config.paperTemplateDir`
/// regardless of `flavor`).
pub fn archive_jar(
    fs: &dyn FileSystem,
    archive_dir: &Path,
    home_dir: &Path,
    flavor: JavaServerFlavor,
    version: &str,
    build: &str,
    source_path: &Path,
) -> Result<ArchiveOutcome, TemplateStoreError> {
    let filename = match flavor {
        JavaServerFlavor::Paper => match build.parse::<i64>() {
            Ok(build_int) => format!("paper-{version}-build{build_int}.jar"),
            Err(_) => {
                return Ok(ArchiveOutcome::Skipped(
                    ArchiveSkipReason::NonNumericPaperBuild,
                ));
            }
        },
        JavaServerFlavor::Purpur => format!("purpur-{version}-build{build}.jar"),
        JavaServerFlavor::Vanilla => format!("minecraft_server-{version}.jar"),
        JavaServerFlavor::Fabric => format!("fabric-server-launch-{version}.jar"),
        _ => {
            return Ok(ArchiveOutcome::Skipped(
                ArchiveSkipReason::UnsupportedFlavor,
            ));
        }
    };

    let archive_path = safe_path(fs, archive_dir, Some(&filename), home_dir)?;
    if fs.stat(&archive_path).is_ok() {
        return Ok(ArchiveOutcome::AlreadyArchived { filename });
    }

    ensure_template_dir(fs, archive_dir)?;
    let bytes = fs.read(source_path)?;
    fs.write(&archive_path, &bytes)?;
    Ok(ArchiveOutcome::Archived { filename })
}

/// Copies a template jar into a server directory under `dest_filename` —
/// the shared shape behind `applyPaperTemplateToSelectedServer`,
/// `updatePaperFromLatestTemplate`, and `updatePluginTemplate`'s own
/// "remove what's there, then `copyItem`" (their running-server refusal
/// and same-prefix-replacement scan are `msc-application`'s job, per
/// this module's own doc — this is just the copy primitive all three
/// share). `server_dir` is validated the same way `archive_dir` is
/// above: the caller passes an already-provisioned server directory, but
/// `dest_filename` is checked against it regardless.
pub fn copy_into_server_dir(
    fs: &dyn FileSystem,
    template_path: &Path,
    server_dir: &Path,
    home_dir: &Path,
    dest_filename: &str,
) -> Result<PathBuf, TemplateStoreError> {
    let dest = safe_path(fs, server_dir, Some(dest_filename), home_dir)?;
    let bytes = fs.read(template_path)?;
    if let Some(parent) = dest.parent() {
        fs.create_dir_all(parent)?;
    }
    fs.write(&dest, &bytes)?;
    Ok(dest)
}

/// `localizedCaseInsensitiveCompare`'s observed behavior on this
/// module's one sort-order fixture: case-folded (ASCII — ties on non-
/// ASCII casing are not exercised by anything in this corpus) and
/// digit-run-aware, so a run of digits compares by numeric magnitude
/// (via length-then-lexicographic on the run with leading zeros
/// stripped, which agrees with numeric order for any digit string) while
/// every other run compares as lowercase text.
fn natural_case_insensitive_compare(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    let mut ai = a.chars().peekable();
    let mut bi = b.chars().peekable();
    loop {
        match (ai.peek().copied(), bi.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ac), Some(bc)) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let a_num = take_digits(&mut ai);
                    let b_num = take_digits(&mut bi);
                    let a_trimmed = a_num.trim_start_matches('0');
                    let b_trimmed = b_num.trim_start_matches('0');
                    let ord = a_trimmed
                        .len()
                        .cmp(&b_trimmed.len())
                        .then_with(|| a_trimmed.cmp(b_trimmed))
                        .then_with(|| a_num.cmp(&b_num));
                    if ord != Ordering::Equal {
                        return ord;
                    }
                } else {
                    let al = ac.to_ascii_lowercase();
                    let bl = bc.to_ascii_lowercase();
                    match al.cmp(&bl) {
                        Ordering::Equal => {
                            ai.next();
                            bi.next();
                        }
                        other => return other,
                    }
                }
            }
        }
    }
}

fn take_digits(it: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut s = String::new();
    while let Some(&c) = it.peek() {
        if !c.is_ascii_digit() {
            break;
        }
        s.push(c);
        it.next();
    }
    s
}
