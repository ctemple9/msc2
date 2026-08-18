//! The family launch shape: which argv a Java server actually starts with,
//! and the pure pieces of the headless launch script MSC 1 generates from
//! the same shape.
//!
//! Ported from `JavaServerLaunchHelper.swift` (`resolve`, `megabytes(fromGB:)`)
//! and `HeadlessScriptGenerator.swift` (`javaScript`'s command-line
//! assembly and `shellQuote`), plus `NeoForgeInstaller.swift`'s/
//! `ForgeInstaller.swift`'s `findArgsFile` *selection* rule (the directory
//! listing that feeds it is I/O and stays in the caller -- the same split
//! [`crate::nbt::first_level_dat_path`] already uses against its own
//! caller in `msc-application`).
//!
//! `HeadlessScriptGenerator.bedrockScript` is not ported: Bedrock stays
//! Phase 10, and MSC 1's own doc comment on it says it "is kept for
//! reference but is no longer called from the UI." The `includeXboxBroadcast`
//! block in `javaScript` is not ported either -- Xbox Broadcast stays
//! Phase 9 per this phase's own "Not in this phase" list.

use std::path::Path;

/// `HeadlessScriptGenerator.shellQuote` (`HeadlessScriptGenerator.swift:227-232`).
const SHELL_SPECIAL_CHARS: &str = " \t\n\"'\\$`!#&;|<>(){}";

pub fn shell_quote(s: &str) -> String {
    if s.chars().any(|c| SHELL_SPECIAL_CHARS.contains(c)) {
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    } else {
        s.to_string()
    }
}

/// `resolve`'s `effectiveRaw` step (`JavaServerLaunchHelper.swift:40-41`):
/// trim, and an empty string defaults to the bare `java` command (distinct
/// from a non-empty bare command or an absolute path, both of which pass
/// through as-is). The directory-vs-executable normalization that follows
/// in MSC 1 is I/O (`msc_infrastructure::java_runtime_detection::normalized_java_executable_path`)
/// and is the caller's job; `resolve` itself falls back to this effective
/// raw string whenever that normalization fails (`.path ?? effectiveRaw`),
/// so a caller composing the two need not treat normalization failure as
/// fatal.
pub fn effective_java_command(raw_java_path: &str) -> String {
    let trimmed = raw_java_path.trim();
    if trimmed.is_empty() {
        "java".to_string()
    } else {
        trimmed.to_string()
    }
}

/// `resolve`'s `jarName` step (`JavaServerLaunchHelper.swift:70-77`): an
/// empty `paperJarPath` (or one whose last path component is empty) falls
/// back to `"paper.jar"`; otherwise the basename. No fixture in
/// `fixtures/headless-script/`/`fixtures/args-file-resolution/` exercises
/// the empty-path fallback branch specifically (P7.5's own characterization
/// flagged this gap) -- ported directly from source instead, the same as
/// MSC 1 itself has no test for it. Tilde-expansion (`expandingTildeInPath`)
/// is not reproduced: no fixture path uses one, and resolving `~` needs a
/// real home-directory lookup, which is I/O this pure function doesn't do.
pub fn jar_basename(paper_jar_path: &str) -> String {
    if paper_jar_path.is_empty() {
        return "paper.jar".to_string();
    }
    let name = Path::new(paper_jar_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    if name.is_empty() {
        "paper.jar".to_string()
    } else {
        name.to_string()
    }
}

/// `NeoForgeInstaller.findArgsFile(in:specificVersion:)`'s selection rule
/// (`NeoForgeInstaller.swift:186-195`): a non-empty configured version wins
/// if it's among the installed versions; otherwise the first installed
/// version (in whatever order the caller's directory listing produced),
/// or `None` if nothing is installed. `installed_versions` are the
/// directory names under `libraries/net/neoforged/neoforge/` that were
/// found to contain `unix_args.txt` -- obtaining that listing is the
/// caller's job.
pub fn neoforge_select_args_file(
    installed_versions: &[String],
    specific_version: Option<&str>,
) -> Option<String> {
    if let Some(v) = specific_version.map(str::trim).filter(|v| !v.is_empty())
        && installed_versions.iter().any(|iv| iv == v)
    {
        return Some(format!(
            "libraries/net/neoforged/neoforge/{v}/unix_args.txt"
        ));
    }
    installed_versions
        .first()
        .map(|v| format!("libraries/net/neoforged/neoforge/{v}/unix_args.txt"))
}

/// `ForgeInstaller.findArgsFile(in:mcVersion:forgeVersion:)`'s selection
/// rule (`NeoForgeInstaller.swift:478-489`): the configured `{mc}-{forge}`
/// pair wins only if BOTH halves are present (a nil/empty half falls
/// straight through to the first-match fallback, it never tries a partial
/// match); otherwise the first installed pair. `installed_pairs` are the
/// directory names under `libraries/net/minecraftforge/forge/`.
pub fn forge_select_args_file(
    installed_pairs: &[String],
    mc_version: Option<&str>,
    forge_version: Option<&str>,
) -> Option<String> {
    let mc = mc_version.map(str::trim).filter(|s| !s.is_empty());
    let forge = forge_version.map(str::trim).filter(|s| !s.is_empty());
    if let (Some(mc), Some(forge)) = (mc, forge) {
        let pair = format!("{mc}-{forge}");
        if installed_pairs.iter().any(|p| p == &pair) {
            return Some(format!(
                "libraries/net/minecraftforge/forge/{pair}/unix_args.txt"
            ));
        }
    }
    installed_pairs
        .first()
        .map(|p| format!("libraries/net/minecraftforge/forge/{p}/unix_args.txt"))
}

/// `javaScript`'s per-flavor invocation line (`HeadlessScriptGenerator.swift:62-75`):
/// `@<args-file> nogui` when an args file was resolved (Forge/NeoForge);
/// an early `exit 1` naming the flavor when it's a Forge-family flavor
/// with no args file found; `-jar <jar> --nogui` otherwise.
pub fn build_java_invocation(
    java_path: &str,
    jvm_flags: &[String],
    args_file: Option<&str>,
    jar_name: &str,
    is_forge_family: bool,
    flavor_display_name: &str,
) -> String {
    let quoted_java = shell_quote(java_path);
    let flags_str = jvm_flags.join(" ");
    if let Some(af) = args_file {
        format!("{quoted_java} {flags_str} @{af} nogui")
    } else if is_forge_family {
        format!(
            "echo \"[MSC] Error: {flavor_display_name} args file not found.\"\necho \"       Run the server once inside MSC to complete installation.\"\nexit 1"
        )
    } else {
        format!(
            "{quoted_java} {flags_str} -jar {} --nogui",
            shell_quote(jar_name)
        )
    }
}

/// `HeadlessWrapMode` (`HeadlessScriptGenerator.swift:8-14`). `Screen` is
/// carried for completeness (the `wrapMode` switch, lines 106-132) even
/// though no P7.5/P7.11 fixture exercises it directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    None,
    AutoRestart,
    Screen,
}

/// The `wrapMode` switch's line assembly (`HeadlessScriptGenerator.swift:106-132`).
pub fn wrap_command_lines(java_cmd: &str, mode: WrapMode) -> Vec<String> {
    match mode {
        WrapMode::None => vec![java_cmd.to_string()],
        WrapMode::AutoRestart => vec![
            "# Auto-restart: re-launches the server if it exits unexpectedly.".to_string(),
            "while true; do".to_string(),
            format!("    {java_cmd}"),
            "    echo \"[MSC] Server exited. Restarting in 5 seconds... (Ctrl+C to stop)\""
                .to_string(),
            "    sleep 5".to_string(),
            "done".to_string(),
        ],
        WrapMode::Screen => vec![
            "# Runs in a detached screen session.".to_string(),
            "# To reattach: screen -r minecraft".to_string(),
            "# To detach:   Ctrl+A then D".to_string(),
            "if ! command -v screen &>/dev/null; then".to_string(),
            "    echo \"[MSC] Error: 'screen' is not installed. Install it with: brew install screen\""
                .to_string(),
            "    exit 1".to_string(),
            "fi".to_string(),
            format!("screen -S minecraft {java_cmd}"),
        ],
    }
}
