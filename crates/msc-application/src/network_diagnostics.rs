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
    let join_address = duckdns_hostname.map(|host| format!("{host}:{port}"));
    ConnectivitySummary {
        join_address_source: if join_address.is_some() {
            "duckdns"
        } else {
            "unavailable"
        },
        join_address,
        local,
        public,
    }
}
