//! Local/private host classification, used to decide whether plain HTTP
//! (rather than HTTPS) is safe to allow for a given host.
//!
//! Ported from `NetworkSafety.swift` (iOS) — but pinned to the verbatim copy
//! `NetworkSafetyTests.swift` carries as its own oracle (see that file's
//! header comment: "this suite pins a VERBATIM PORT of the iOS NetworkSafety
//! rule set... as the oracle instead"), not the iOS source's `IPv4Address`
//! struct directly. The two diverge on Tailscale's CGNAT range
//! (100.64.0.0/10): the iOS struct folds it into `isPrivateOrLocal`, but the
//! test suite's copy deliberately does not, since a separate function
//! (`preferredPairingHost`) handles `100.*` as a first-choice pairing host
//! instead. The 13 extracted fixtures (`fixtures/network-safety/`, from
//! P0.14) encode the test suite's behavior, so that's what this ports.

/// Allow HTTP only when the host looks local/private (LAN/VPN ranges).
///
/// `URL.host` always strips brackets from IPv6 literals (e.g. `[::1]` →
/// `::1`), so callers are expected to pass bare IPv6 forms.
pub fn is_local_or_private_host(host: &str) -> bool {
    let h = host.trim().to_lowercase();
    if h.is_empty() {
        return false;
    }
    if h == "localhost" || h == "127.0.0.1" {
        return true;
    }
    if h.ends_with(".local") {
        return true;
    }
    if h.ends_with(".ts.net") {
        return true;
    }

    // IPv6: colons present and no dots (excludes IPv4-mapped ::ffff:a.b.c.d forms).
    if h.contains(':') && !h.contains('.') {
        if h == "::1" {
            return true; // loopback (RFC 4291)
        }
        if h.starts_with("fe80:") {
            return true; // link-local (fe80::/10, RFC 4291)
        }
        if h.starts_with("fd") {
            return true; // ULA (fd00::/8, RFC 4193)
        }
        return false;
    }

    // IPv4 — verbatim port of the test suite's prefix-based checks.
    if h.starts_with("10.") {
        return true;
    }
    if h.starts_with("192.168.") {
        return true;
    }
    if h.starts_with("172.") {
        let parts: Vec<&str> = h.split('.').collect();
        if parts.len() >= 2
            && let Ok(second) = parts[1].parse::<i64>()
            && (16..=31).contains(&second)
        {
            return true;
        }
    }
    if h.starts_with("169.254.") {
        return true;
    }
    false
}
