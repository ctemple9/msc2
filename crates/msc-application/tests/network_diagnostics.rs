use msc_application::network_diagnostics::connectivity_summary;
use msc_domain::networking::DiagnosticResult;
#[test]
fn duckdns_is_the_player_address_without_a_provider_call() {
    let summary = connectivity_summary(
        Some("family.duckdns.org"),
        25565,
        DiagnosticResult::Open,
        DiagnosticResult::Unavailable,
    );
    assert_eq!(
        summary.join_address.as_deref(),
        Some("family.duckdns.org:25565")
    );
    assert_eq!(summary.join_address_source, "duckdns");
    assert_eq!(summary.public.api_outcome(), "unavailable");
}
