//! Bounded local reachability probes for Java and Bedrock servers.
use msc_domain::networking::{DiagnosticResult, classify_tcp_connection};
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const RAKNET_MAGIC: [u8; 16] = [
    0x00, 0xff, 0xff, 0x00, 0xfe, 0xfe, 0xfe, 0xfe, 0xfd, 0xfd, 0xfd, 0xfd, 0x12, 0x34, 0x56, 0x78,
];

pub fn probe_tcp(host: &str, port: u16, timeout: Duration) -> DiagnosticResult {
    let Ok(addresses) = (host, port).to_socket_addrs() else {
        return DiagnosticResult::Unavailable;
    };
    let Some(address) = addresses.into_iter().next() else {
        return DiagnosticResult::Unavailable;
    };
    match TcpStream::connect_timeout(&address, timeout) {
        Ok(_) => DiagnosticResult::Open,
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
            classify_tcp_connection("refused")
        }
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::AddrNotAvailable
                    | std::io::ErrorKind::NetworkUnreachable
                    | std::io::ErrorKind::HostUnreachable
            ) =>
        {
            classify_tcp_connection("unreachable")
        }
        Err(_) => DiagnosticResult::Unavailable,
    }
}

/// Best-effort local UDP probe for Bedrock and Geyser ports.
///
/// UDP has no connection handshake. A successful send therefore proves only
/// that the operating system accepted the datagram; a refused receive is the
/// useful negative signal. A timeout is treated as open to match Minecraft's
/// UDP behavior: a valid listener may ignore an empty probe packet.
pub fn probe_udp(host: &str, port: u16, timeout: Duration) -> DiagnosticResult {
    let Ok(addresses) = (host, port).to_socket_addrs() else {
        return DiagnosticResult::Unavailable;
    };
    let Some(address) = addresses.into_iter().next() else {
        return DiagnosticResult::Unavailable;
    };
    let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else {
        return DiagnosticResult::Unavailable;
    };
    if socket.set_read_timeout(Some(timeout)).is_err() || socket.connect(address).is_err() {
        return DiagnosticResult::Unavailable;
    }
    if let Err(error) = socket.send(&[0]) {
        return if error.kind() == std::io::ErrorKind::ConnectionRefused {
            DiagnosticResult::Closed
        } else {
            DiagnosticResult::Unavailable
        };
    }

    let mut response = [0_u8; 1];
    match socket.recv(&mut response) {
        Ok(_) => DiagnosticResult::Open,
        Err(error) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
            DiagnosticResult::Closed
        }
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            ) =>
        {
            DiagnosticResult::Open
        }
        Err(_) => DiagnosticResult::Unavailable,
    }
}

/// Send Bedrock's RakNet unconnected ping and require a valid pong.
///
/// Unlike an empty UDP send, this proves that the complete path reached a
/// Bedrock-compatible endpoint and that its reply returned to the caller.
pub fn probe_bedrock(host: &str, port: u16, timeout: Duration) -> DiagnosticResult {
    let Ok(addresses) = (host, port).to_socket_addrs() else {
        return DiagnosticResult::Unavailable;
    };
    let Some(address) = addresses.into_iter().next() else {
        return DiagnosticResult::Unavailable;
    };
    let bind_address = if address.is_ipv6() {
        "[::]:0"
    } else {
        "0.0.0.0:0"
    };
    let Ok(socket) = UdpSocket::bind(bind_address) else {
        return DiagnosticResult::Unavailable;
    };
    if socket.set_read_timeout(Some(timeout)).is_err()
        || socket.set_write_timeout(Some(timeout)).is_err()
        || socket.connect(address).is_err()
    {
        return DiagnosticResult::Unavailable;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let mut ping = Vec::with_capacity(33);
    ping.push(0x01);
    ping.extend_from_slice(&timestamp.to_be_bytes());
    ping.extend_from_slice(&RAKNET_MAGIC);
    ping.extend_from_slice(&timestamp.rotate_left(17).to_be_bytes());
    if socket.send(&ping).is_err() {
        return DiagnosticResult::Unavailable;
    }

    let mut response = [0_u8; 2048];
    match socket.recv(&mut response) {
        Ok(length) if length >= 35 && response[0] == 0x1c && response[17..33] == RAKNET_MAGIC => {
            DiagnosticResult::Open
        }
        Ok(_) => DiagnosticResult::Unavailable,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::WouldBlock
            ) =>
        {
            DiagnosticResult::Closed
        }
        Err(_) => DiagnosticResult::Unavailable,
    }
}
