//! Compose bounded probe results into player-facing connectivity facts.
use msc_domain::networking::DiagnosticResult;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectivitySummary {
    pub join_address: Option<String>,
    pub join_address_source: &'static str,
    pub local: DiagnosticResult,
    pub public: DiagnosticResult,
}
pub fn connectivity_summary(
    duckdns_hostname: Option<&str>,
    port: u16,
    local: DiagnosticResult,
    public: DiagnosticResult,
) -> ConnectivitySummary {
    connectivity_summary_with_public_ip(duckdns_hostname, None, port, local, public)
}

pub fn connectivity_summary_with_public_ip(
    duckdns_hostname: Option<&str>,
    public_ip: Option<&str>,
    port: u16,
    local: DiagnosticResult,
    public: DiagnosticResult,
) -> ConnectivitySummary {
    let (join_address, join_address_source) = if let Some(host) = duckdns_hostname {
        (Some(endpoint(host, port)), "duckdns")
    } else if let Some(host) = public_ip {
        (Some(endpoint(host, port)), "public_ip")
    } else {
        (None, "unavailable")
    };
    ConnectivitySummary {
        join_address_source,
        join_address,
        local,
        public,
    }
}

fn endpoint(host: &str, port: u16) -> String {
    let host = host.trim();
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}
