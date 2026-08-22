//! Pure, fixture-backed rules for player networking. These types describe safe
//! display and classification only; sockets, helper processes, and secrets
//! belong to later infrastructure/application steps.

use crate::network_safety::is_local_or_private_host;
use sha1::{Digest, Sha1};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticResult {
    NotAttempted,
    Open,
    Closed,
    Unreachable,
    Unavailable,
    NotApplicable,
}

impl DiagnosticResult {
    pub fn summary(self) -> &'static str {
        match self {
            Self::NotAttempted => "unknown",
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Unreachable => "unreachable",
            Self::Unavailable => "provider_unavailable",
            Self::NotApplicable => "not_applicable",
        }
    }

    /// The API folds a never-run probe into `not_applicable`; the domain keeps
    /// `NotAttempted` so missing evidence cannot be confused with a result.
    pub fn api_outcome(self) -> &'static str {
        match self {
            Self::NotAttempted | Self::NotApplicable => "not_applicable",
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Unreachable => "unreachable",
            Self::Unavailable => "unavailable",
        }
    }
}

pub fn diagnostic_for_server_has_run(server_has_run: bool) -> DiagnosticResult {
    if server_has_run {
        DiagnosticResult::Unavailable
    } else {
        DiagnosticResult::NotAttempted
    }
}

pub fn classify_tcp_connection(state: &str) -> DiagnosticResult {
    match state {
        "ready" => DiagnosticResult::Open,
        "refused" => DiagnosticResult::Closed,
        "unreachable" => DiagnosticResult::Unreachable,
        _ => DiagnosticResult::Unavailable,
    }
}

pub fn classify_provider_outcome(outcome: &str) -> DiagnosticResult {
    match outcome {
        "open" => DiagnosticResult::Open,
        "closed" => DiagnosticResult::Closed,
        "unreachable" => DiagnosticResult::Unreachable,
        "network_error" | "parse_error" | "timeout" => DiagnosticResult::Unavailable,
        _ => DiagnosticResult::Unavailable,
    }
}

/// A player-facing address, never an accidental local management address.
pub fn safe_player_address(host: &str, port: Option<u16>) -> Option<String> {
    let host = host
        .trim()
        .trim_matches(|c| matches!(c, '(' | ')' | '[' | ']' | '\'' | '"'));
    if host.is_empty() || is_local_or_private_host(host) {
        return None;
    }
    let host = host.split_whitespace().next()?;
    Some(match port {
        Some(port) => format!("{host}:{port}"),
        None => host.to_owned(),
    })
}

pub fn parse_playit_address(line: &str, expecting_address: bool) -> Option<String> {
    const PROVIDER_DOMAINS: [&str; 3] = ["joinmc.link", "auto.playit.gg", "ply.gg"];
    for token in line.split_whitespace() {
        let clean = token.trim_matches(|c| matches!(c, '(' | ')' | '[' | ']' | '\'' | '"'));
        let host = clean.split(':').next().unwrap_or_default();
        let provider_address = PROVIDER_DOMAINS.iter().any(|domain| host.contains(domain));
        let has_port = clean
            .rsplit_once(':')
            .is_some_and(|(_, port)| port.parse::<u16>().is_ok());
        if provider_address || (expecting_address && has_port) {
            return safe_player_address(clean, None);
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourcePackError {
    JavaPackMustBeZip,
    UnsafeFilename,
}

pub fn validate_java_pack_filename(filename: &str) -> Result<(), ResourcePackError> {
    if filename.is_empty() || filename.contains(['/', '\\']) || filename == "." || filename == ".."
    {
        return Err(ResourcePackError::UnsafeFilename);
    }
    if !filename.to_ascii_lowercase().ends_with(".zip") {
        return Err(ResourcePackError::JavaPackMustBeZip);
    }
    Ok(())
}

pub fn resource_pack_sha1(bytes: &[u8]) -> String {
    format!("{:x}", Sha1::digest(bytes))
}

pub fn hosted_resource_pack_url(
    host: &str,
    port: u16,
    filename: &str,
) -> Result<String, ResourcePackError> {
    validate_java_pack_filename(filename)?;
    let host = host.trim();
    if host.is_empty() || host.contains(['/', '?', '#', '@']) {
        return Err(ResourcePackError::UnsafeFilename);
    }
    Ok(format!(
        "http://{host}:{port}/{}",
        percent_encode_path_segment(filename)
    ))
}

fn percent_encode_path_segment(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                vec![byte as char]
            }
            byte => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossPlayStatus {
    None,
    GeyserOnly,
    FloodgateOnly,
    Both,
}

pub fn classify_cross_play(files: impl IntoIterator<Item = impl AsRef<str>>) -> CrossPlayStatus {
    let mut geyser = false;
    let mut floodgate = false;
    for file in files {
        let name = file.as_ref().to_ascii_lowercase();
        geyser |= name.contains("geyser") && name.ends_with(".jar");
        floodgate |= name.contains("floodgate") && name.ends_with(".jar");
    }
    match (geyser, floodgate) {
        (false, false) => CrossPlayStatus::None,
        (true, false) => CrossPlayStatus::GeyserOnly,
        (false, true) => CrossPlayStatus::FloodgateOnly,
        (true, true) => CrossPlayStatus::Both,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BroadcastAuthPrompt {
    pub code: String,
    pub url: String,
}

pub fn parse_broadcast_auth_prompt(line: &str) -> Option<BroadcastAuthPrompt> {
    let marker = "https://www.microsoft.com/link";
    let url_start = line.find(marker)?;
    let url = line[url_start..].split_whitespace().next()?.to_owned();
    let code_start = line.find("enter the code ")? + "enter the code ".len();
    let code = line[code_start..].split_whitespace().next()?.trim();
    (!code.is_empty()).then(|| BroadcastAuthPrompt {
        code: code.to_owned(),
        url,
    })
}

pub fn broadcast_is_ready(line: &str) -> bool {
    line.to_ascii_lowercase()
        .contains("creation of xbox live session was successful")
}
