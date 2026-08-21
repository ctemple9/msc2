//! `PluginSourceDetector.swift`'s URL-to-source classification and
//! per-provider URL parsing, per `docs/msc2/addons/phase8-scope.md` and
//! `fixtures/plugin-source-resolution/` (P8.4). `PluginSourceType` is a
//! closed four-case enum (`AppModels.swift:74-78`); everything here is pure
//! string manipulation, no I/O.

/// `AppModels.swift:74-78`'s closed enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginSourceType {
    Github,
    Modrinth,
    Hangar,
    Direct,
}

impl PluginSourceType {
    pub fn as_str(self) -> &'static str {
        match self {
            PluginSourceType::Github => "github",
            PluginSourceType::Modrinth => "modrinth",
            PluginSourceType::Hangar => "hangar",
            PluginSourceType::Direct => "direct",
        }
    }
}

/// `PluginSourceDetector.stripScheme(_:)` (line 77-86): drops a matching
/// `https://`/`http://` prefix (checked in that order, case-insensitively)
/// from the ORIGINAL, non-lowercased string -- so scheme casing doesn't
/// leak into the result, and a schemeless string passes through unchanged.
pub fn strip_scheme(s: &str) -> String {
    for prefix in ["https://", "http://"] {
        if s.len() >= prefix.len() && s[..prefix.len()].to_lowercase() == prefix {
            return s[prefix.len()..].to_string();
        }
    }
    s.to_string()
}

/// `PluginSourceDetector.detect(url:)` (line 18-27): trims and lowercases
/// first, then checks github.com / modrinth.com / hangar.papermc.io (in
/// that order) as substring containment, then falls back to `.direct` for
/// any remaining http(s)-scheme or `.jar`-suffixed string. A blank input or
/// a genuinely unrecognized string (no named host, no scheme, no `.jar`
/// suffix) returns `None`.
pub fn detect(url: &str) -> Option<PluginSourceType> {
    let trimmed = url.trim().to_lowercase();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.contains("github.com") {
        return Some(PluginSourceType::Github);
    }
    if trimmed.contains("modrinth.com") {
        return Some(PluginSourceType::Modrinth);
    }
    if trimmed.contains("hangar.papermc.io") {
        return Some(PluginSourceType::Hangar);
    }
    if trimmed.ends_with(".jar") || trimmed.starts_with("http") {
        return Some(PluginSourceType::Direct);
    }
    None
}

/// Splits a stripped URL into its trimmed `/`-separated path segments, the
/// shared shape `parseGitHub`/`parseModrinth`/`parseHangar` all build on.
fn path_segments(url: &str) -> Vec<String> {
    strip_scheme(url)
        .trim_matches('/')
        .split('/')
        .map(str::to_string)
        .collect()
}

/// `PluginSourceDetector.parseGitHub(url:)` (line 32-43): requires at
/// least 3 path segments (`github.com` host, owner, repo) -- any path
/// beyond owner/repo is simply unused, not rejected as malformed. Host is
/// compared case-insensitively; owner/repo preserve their original casing.
pub fn parse_github(url: &str) -> Option<(String, String)> {
    let parts = path_segments(url);
    if parts.len() < 3 || parts[0].to_lowercase() != "github.com" {
        return None;
    }
    let (owner, repo) = (parts[1].clone(), parts[2].clone());
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some((owner, repo))
}

/// `PluginSourceDetector.parseHangar(url:)` (line 63-73): structurally
/// identical to [`parse_github`] against `hangar.papermc.io`, with
/// author/slug in place of owner/repo.
pub fn parse_hangar(url: &str) -> Option<(String, String)> {
    let parts = path_segments(url);
    if parts.len() < 3 || parts[0].to_lowercase() != "hangar.papermc.io" {
        return None;
    }
    let (author, slug) = (parts[1].clone(), parts[2].clone());
    if author.is_empty() || slug.is_empty() {
        return None;
    }
    Some((author, slug))
}

/// `PluginSourceDetector.parseModrinth(url:)` (line 48-58): the slug is
/// always `parts[2]` -- despite the doc comment implying `parts[1]`
/// ("plugin" vs "mod") is validated, the guard clause never actually reads
/// it. Preserved as-is: a faithful port keeps this permissiveness rather
/// than adding a stricter check MSC 1 never had.
pub fn parse_modrinth(url: &str) -> Option<String> {
    let parts = path_segments(url);
    if parts.len() < 3 || parts[0].to_lowercase() != "modrinth.com" {
        return None;
    }
    let slug = parts[2].clone();
    if slug.is_empty() { None } else { Some(slug) }
}
