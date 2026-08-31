//! Pure, fixture-backed rules for player networking. These types describe safe
//! display and classification only; sockets, helper processes, and secrets
//! belong to later infrastructure/application steps.

use crate::identity::ServerType;
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

pub const PLAYIT_JAVA_TUNNEL_NAME: &str = "MSC Java";
pub const PLAYIT_BEDROCK_TUNNEL_NAME: &str = "MSC Bedrock";
pub const PLAYIT_VOICE_TUNNEL_NAME: &str = "MSC Voice";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayitTunnelKind {
    Java,
    Bedrock,
    Voice,
}

impl PlayitTunnelKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Java => PLAYIT_JAVA_TUNNEL_NAME,
            Self::Bedrock => PLAYIT_BEDROCK_TUNNEL_NAME,
            Self::Voice => PLAYIT_VOICE_TUNNEL_NAME,
        }
    }

    pub const fn tunnel_type(self) -> Option<&'static str> {
        match self {
            Self::Java => Some("minecraft-java"),
            Self::Bedrock => Some("minecraft-bedrock"),
            Self::Voice => None,
        }
    }

    pub const fn port_type(self) -> &'static str {
        match self {
            Self::Java => "tcp",
            Self::Bedrock | Self::Voice => "udp",
        }
    }

    pub const fn uses_static_ip_fallback(self) -> bool {
        matches!(self, Self::Bedrock | Self::Voice)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayitTunnelSpec {
    pub kind: PlayitTunnelKind,
    pub local_port: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayitTunnelAddresses {
    pub java: Option<String>,
    pub bedrock: Option<String>,
    pub voice: Option<String>,
}

/// The tunnel inventory follows MSC 1's server rules: Java servers always
/// expose Java, a configured Bedrock/Geyser port adds Bedrock, and voice chat
/// owns the fixed Simple Voice Chat UDP port. Bedrock servers expose only
/// their Bedrock tunnel, plus voice when enabled.
pub fn playit_tunnel_specs(
    server_type: ServerType,
    java_port: Option<u16>,
    bedrock_enabled: bool,
    bedrock_port: Option<u16>,
    voice_enabled: bool,
) -> Vec<PlayitTunnelSpec> {
    let mut specs = Vec::new();
    match server_type {
        ServerType::Java => {
            specs.push(PlayitTunnelSpec {
                kind: PlayitTunnelKind::Java,
                local_port: java_port.unwrap_or(25565),
            });
            if bedrock_enabled || bedrock_port.is_some() {
                specs.push(PlayitTunnelSpec {
                    kind: PlayitTunnelKind::Bedrock,
                    local_port: bedrock_port.unwrap_or(19132),
                });
            }
        }
        ServerType::Bedrock => specs.push(PlayitTunnelSpec {
            kind: PlayitTunnelKind::Bedrock,
            local_port: bedrock_port.unwrap_or(19132),
        }),
    }
    if voice_enabled {
        specs.push(PlayitTunnelSpec {
            kind: PlayitTunnelKind::Voice,
            local_port: 24454,
        });
    }
    specs
}

/// Builds a player-facing address from one inventory record. Java must have
/// the provider's assigned domain; UDP tunnels prefer the static IPv4 because
/// RakNet and Simple Voice Chat need a stable numeric endpoint, with the
/// assigned domain retained as the provider's fallback.
pub fn playit_public_address(
    kind: PlayitTunnelKind,
    assigned_domain: Option<&str>,
    static_ip4: Option<&str>,
    port: Option<u16>,
) -> Option<String> {
    let port = port.filter(|port| *port > 0)?;
    let host = if kind.uses_static_ip_fallback() {
        static_ip4.or(assigned_domain)
    } else {
        assigned_domain
    }?;
    safe_player_address(host, Some(port))
}

/// Patch only the three Simple Voice Chat values MSC owns. Existing comments,
/// blank lines, and unrelated properties remain untouched.
pub fn patch_voice_chat_properties(existing: &str, voice_host: &str) -> String {
    let desired = [
        ("voice_host", voice_host),
        ("bind_address", "*"),
        ("port", "24454"),
    ];
    let mut replaced = [false; 3];
    let mut lines: Vec<String> = existing.lines().map(str::to_owned).collect();
    for line in &mut lines {
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };
        let Some(index) = desired.iter().position(|(wanted, _)| key.trim() == *wanted) else {
            continue;
        };
        *line = format!("{}={}", desired[index].0, desired[index].1);
        replaced[index] = true;
    }
    for (index, (key, value)) in desired.iter().enumerate() {
        if !replaced[index] {
            lines.push(format!("{key}={value}"));
        }
    }
    let mut output = lines.join("\n");
    if !output.is_empty() {
        output.push('\n');
    }
    output
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

/// MCXboxBroadcast reports the account it authenticated immediately before
/// creating the Xbox LIVE session. Keep only the display name; the XUID and
/// all credential material stay out of the client-facing status.
pub fn parse_broadcast_gamertag(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let marker = "successfully authenticated as ";
    let start = lower.find(marker)? + marker.len();
    let gamertag = line[start..]
        .split_whitespace()
        .next()?
        .trim_matches(['[', ']', '(', ')', ',']);
    (!gamertag.is_empty()).then(|| gamertag.to_owned())
}

pub fn broadcast_is_ready(line: &str) -> bool {
    line.to_ascii_lowercase()
        .contains("creation of xbox live session was successful")
}
