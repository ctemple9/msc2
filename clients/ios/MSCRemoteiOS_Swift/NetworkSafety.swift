import Foundation

enum NetworkSafety {
    /// Allow HTTP only when the host looks local/private (LAN/VPN ranges).
    /// URL.host already strips brackets from IPv6 literals (e.g. [::1] → ::1).
    static func isLocalOrPrivateHost(_ host: String) -> Bool {
        let h = host.lowercased().trimmingCharacters(in: .whitespacesAndNewlines)
        if h == "localhost" || h == "127.0.0.1" { return true }
        if h.hasSuffix(".local") { return true }

        // Tailscale MagicDNS hostnames
        if h.hasSuffix(".ts.net") { return true }

        // IPv6 checks (before IPv4: colons in the string exclude IPv4 parsing)
        if let ipv6 = IPv6Address(h) {
            return ipv6.isPrivateOrLocal
        }

        // IPv4 checks
        if let ipv4 = IPv4Address(h) {
            return ipv4.isPrivateOrLocal
        }

        return false
    }

    static func httpIsAllowed(for url: URL) -> Bool {
        guard (url.scheme ?? "").lowercased() == "http" else { return true } // https is fine
        guard let host = url.host else { return false }
        return isLocalOrPrivateHost(host)
    }
}

private struct IPv6Address {
    let raw: String

    /// Matches IPv6 literals (contain colon, no dot — rejects IPv4-mapped ::ffff:a.b.c.d forms).
    init?(_ s: String) {
        guard s.contains(":"), !s.contains(".") else { return nil }
        self.raw = s.lowercased()
    }

    var isPrivateOrLocal: Bool {
        if raw == "::1" { return true }           // loopback (RFC 4291)
        if raw.hasPrefix("fe80:") { return true } // link-local (fe80::/10, RFC 4291)
        if raw.hasPrefix("fd") { return true }    // ULA (fd00::/8, RFC 4193)
        return false
    }
}

private struct IPv4Address {
    let a: Int
    let b: Int
    let c: Int
    let d: Int

    init?(_ s: String) {
        let parts = s.split(separator: ".").map(String.init)
        guard parts.count == 4 else { return nil }
        let nums = parts.compactMap { Int($0) }
        guard nums.count == 4 else { return nil }
        self.a = nums[0]; self.b = nums[1]; self.c = nums[2]; self.d = nums[3]
        guard (0...255).contains(a), (0...255).contains(b), (0...255).contains(c), (0...255).contains(d) else { return nil }
    }

    var isPrivateOrLocal: Bool {
        // RFC1918 private ranges
        if a == 10 { return true }
        if a == 192 && b == 168 { return true }
        if a == 172 && (16...31).contains(b) { return true }

        // Loopback
        if a == 127 { return true }

        // CGNAT range (often used by VPNs like Tailscale): 100.64.0.0/10
        if a == 100 && (64...127).contains(b) { return true }

        return false
    }
}

