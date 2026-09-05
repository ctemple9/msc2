//! Small, bounded public-address lookup used by connectivity diagnostics.
use std::net::IpAddr;
use std::time::Duration;

const PUBLIC_IP_URL: &str = "https://api.ipify.org";
const MAX_RESPONSE_BYTES: u64 = 64;

/// Ask a public IP service which address this host presents to the internet.
///
/// This identifies the address players should try after router port forwarding;
/// it does not claim that the forwarded port is reachable. The caller keeps
/// that distinction in the connectivity diagnostics.
pub fn detect(timeout: Duration) -> Option<String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut response = agent
        .get(PUBLIC_IP_URL)
        .header("Accept", "text/plain")
        .call()
        .ok()?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit(MAX_RESPONSE_BYTES)
        .read_to_vec()
        .ok()?;
    let body = String::from_utf8(bytes).ok()?;
    body.trim().parse::<IpAddr>().ok().map(|ip| ip.to_string())
}
