//! Java launch-command construction. The Phase 4 Paper-only slice
//! (`PaperLaunchRequest`/`build_paper_launch_command`) keeps its exact,
//! byte-for-byte-proven argv shape unchanged; P7.11 adds the six-family
//! generalization (`resolve_java_launch`/`build_headless_java_script`)
//! alongside it, composing `msc_domain::launch_shape`'s pure pieces with
//! the I/O `JavaServerLaunchHelper.resolve` needs: java-path normalization
//! (`msc_infrastructure::java_runtime_detection`) and the Forge/NeoForge
//! args-file directory scan (`find_neoforge_args_file`/`find_forge_args_file`
//! below), the same domain/I/O split `msc_domain::nbt::first_level_dat_path`
//! already uses against its own caller in `worlds.rs`.

use msc_domain::launch_shape;
use msc_infrastructure::fs::FileSystem;
use msc_infrastructure::java_runtime_detection;
use std::fmt;
use std::path::{Path, PathBuf};

const SANDBOX_SUPPRESS_FLAGS: [&str; 4] = [
    "-Djna.nosys=true",
    "-Djna.nounpack=true",
    "-Djline.terminal=dumb",
    "-Dio.netty.noUnsafe=true",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedJavaLaunch {
    pub executable_path: PathBuf,
    pub prefix_arguments: Vec<String>,
}

impl ValidatedJavaLaunch {
    pub fn new(
        executable_path: impl Into<PathBuf>,
        prefix_arguments: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            executable_path: executable_path.into(),
            prefix_arguments: prefix_arguments.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaperLaunchRequest {
    pub java: ValidatedJavaLaunch,
    pub server_dir: PathBuf,
    pub paper_jar_path: PathBuf,
    pub min_ram_gb: f64,
    pub max_ram_gb: f64,
    pub extra_flags: String,
}

impl PaperLaunchRequest {
    pub fn new(
        java: ValidatedJavaLaunch,
        server_dir: impl Into<PathBuf>,
        paper_jar_path: impl Into<PathBuf>,
        min_ram_gb: f64,
        max_ram_gb: f64,
        extra_flags: impl Into<String>,
    ) -> Self {
        Self {
            java,
            server_dir: server_dir.into(),
            paper_jar_path: paper_jar_path.into(),
            min_ram_gb,
            max_ram_gb,
            extra_flags: extra_flags.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaperLaunchCommand {
    pub executable_path: PathBuf,
    pub arguments: Vec<String>,
    pub working_directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaLaunchError {
    ServerJarNotFound { path: PathBuf },
}

impl fmt::Display for JavaLaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ServerJarNotFound { path } => {
                write!(
                    f,
                    "Server JAR not found in server folder: {}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for JavaLaunchError {}

pub trait JavaLaunchFileSystem {
    fn is_file(&self, path: &Path) -> bool;
}

pub struct StdJavaLaunchFileSystem;

impl JavaLaunchFileSystem for StdJavaLaunchFileSystem {
    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }
}

pub fn build_paper_launch_command(
    fs: &dyn JavaLaunchFileSystem,
    request: &PaperLaunchRequest,
) -> Result<PaperLaunchCommand, JavaLaunchError> {
    let jar_name = launch_shape::jar_basename(&request.paper_jar_path.to_string_lossy());
    // `server_dir` comes from the same forward-slash convention every
    // fixture in this codebase uses; `Path::join` would insert a
    // backslash on Windows, leaving the error message below with a
    // mixed-separator path (`/srv/mc\paper.jar`) even though it resolves
    // to the same file. See `msc_infrastructure::fs::join_forward_slash`.
    let jar_in_working_dir =
        msc_infrastructure::fs::join_forward_slash(&request.server_dir, jar_name.as_ref());
    if !fs.is_file(&jar_in_working_dir) {
        return Err(JavaLaunchError::ServerJarNotFound {
            path: jar_in_working_dir,
        });
    }

    let mut arguments = request.java.prefix_arguments.clone();
    arguments.extend(jvm_flags(
        request.min_ram_gb,
        request.max_ram_gb,
        &request.extra_flags,
    ));
    arguments.extend(["-jar".to_string(), jar_name, "--nogui".to_string()]);

    Ok(PaperLaunchCommand {
        executable_path: request.java.executable_path.clone(),
        arguments,
        working_directory: request.server_dir.clone(),
    })
}

pub fn jvm_flags(min_ram_gb: f64, max_ram_gb: f64, extra_flags: &str) -> Vec<String> {
    let mut flags = vec![
        format!("-Xms{}M", megabytes_from_gb(min_ram_gb)),
        format!("-Xmx{}M", megabytes_from_gb(max_ram_gb)),
    ];
    flags.extend(
        SANDBOX_SUPPRESS_FLAGS
            .iter()
            .map(|flag| (*flag).to_string()),
    );
    flags.extend(extra_flags.split_whitespace().map(str::to_string));
    flags
}

pub fn megabytes_from_gb(gb: f64) -> i64 {
    (gb * 1024.0).round() as i64
}

// ---------------------------------------------------------------------
// P7.11: the six-family generalization.
// ---------------------------------------------------------------------

/// Scans `server_dir/libraries/net/neoforged/neoforge/` for version
/// directories containing `unix_args.txt`, then delegates the selection
/// itself to `msc_domain::launch_shape::neoforge_select_args_file` -- the
/// I/O half of `NeoForgeInstaller.findArgsFile(in:specificVersion:)`.
pub fn find_neoforge_args_file(
    fs: &dyn FileSystem,
    server_dir: &Path,
    specific_version: Option<&str>,
) -> Option<String> {
    let base = server_dir.join("libraries/net/neoforged/neoforge");
    let installed = installed_subdirs_containing(fs, &base, "unix_args.txt");
    launch_shape::neoforge_select_args_file(&installed, specific_version)
}

/// The Forge sibling of [`find_neoforge_args_file`]: scans
/// `server_dir/libraries/net/minecraftforge/forge/` for `{mc}-{forge}`
/// pair directories containing `unix_args.txt`.
pub fn find_forge_args_file(
    fs: &dyn FileSystem,
    server_dir: &Path,
    mc_version: Option<&str>,
    forge_version: Option<&str>,
) -> Option<String> {
    let base = server_dir.join("libraries/net/minecraftforge/forge");
    let installed = installed_subdirs_containing(fs, &base, "unix_args.txt");
    launch_shape::forge_select_args_file(&installed, mc_version, forge_version)
}

/// Lists `base`'s immediate subdirectories that contain a file named
/// `marker`, returning their basenames. An unreadable/absent `base`
/// (e.g. the server directory doesn't exist yet) yields an empty list,
/// not an error -- matching MSC 1's own `try?`-wrapped
/// `contentsOfDirectory` call.
fn installed_subdirs_containing(fs: &dyn FileSystem, base: &Path, marker: &str) -> Vec<String> {
    let Ok(entries) = fs.list(base) else {
        return Vec::new();
    };
    entries
        .into_iter()
        .filter(|entry| {
            fs.stat(&entry.join(marker))
                .map(|m| m.is_file)
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            entry
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
        })
        .collect()
}

/// The result of `JavaServerLaunchHelper.resolve`: everything a launch
/// needs, minus the flavor-specific invocation assembly itself (that's
/// [`launch_shape::build_java_invocation`], since it also needs to know
/// whether this flavor is Forge-family).
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedJavaLaunch {
    pub java_path: String,
    pub jvm_flags: Vec<String>,
    pub jar_name: String,
}

/// Composes the java-path and JVM-flags halves of `resolve`
/// (`JavaServerLaunchHelper.swift:40-57,70-77`). The args-file half is the
/// caller's own [`find_neoforge_args_file`]/[`find_forge_args_file`] call
/// (or, in a test, a stubbed value) -- kept separate here because
/// `resolve`'s own args-file lookup is flavor-dispatched on `config.javaFlavor`,
/// which this module has no reason to know about.
pub fn resolve_java_launch(
    fs: &dyn FileSystem,
    raw_java_path: &str,
    extra_flags: &str,
    min_ram_gb: f64,
    max_ram_gb: f64,
    paper_jar_path: &str,
) -> ResolvedJavaLaunch {
    let effective = launch_shape::effective_java_command(raw_java_path);
    let java_path = java_runtime_detection::normalized_java_executable_path(fs, &effective)
        .unwrap_or(effective);
    ResolvedJavaLaunch {
        java_path,
        jvm_flags: jvm_flags(min_ram_gb, max_ram_gb, extra_flags),
        jar_name: launch_shape::jar_basename(paper_jar_path),
    }
}

/// `HeadlessScriptGenerator.javaScript`, minus the `includeXboxBroadcast`
/// block -- Xbox Broadcast stays Phase 9 per this phase's own scope list.
/// `HeadlessScriptGenerator.bedrockScript` is not ported at all: Bedrock
/// stays Phase 10, and MSC 1's own doc comment on it says it's kept only
/// for reference and is no longer reachable from the UI.
#[allow(clippy::too_many_arguments)]
pub fn build_headless_java_script(
    resolved: &ResolvedJavaLaunch,
    args_file: Option<&str>,
    is_forge_family: bool,
    flavor_display_name: &str,
    server_display_name: &str,
    add_on_folder: &str,
    server_dir: &str,
    wrap_mode: launch_shape::WrapMode,
) -> String {
    let java_cmd = launch_shape::build_java_invocation(
        &resolved.java_path,
        &resolved.jvm_flags,
        args_file,
        &resolved.jar_name,
        is_forge_family,
        flavor_display_name,
    );

    let mut lines: Vec<String> = vec![
        "#!/bin/bash".to_string(),
        "# Generated by Minecraft Server Controller".to_string(),
        format!("# Server: {server_display_name}"),
        format!("# Flavor: {flavor_display_name}"),
        String::new(),
        format!("cd {}", launch_shape::shell_quote(server_dir)),
        String::new(),
        format!(
            "# {} in your {add_on_folder}/ folder will load automatically.",
            capitalize(add_on_folder)
        ),
        String::new(),
    ];
    lines.extend(launch_shape::wrap_command_lines(&java_cmd, wrap_mode));
    lines.join("\n") + "\n"
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
