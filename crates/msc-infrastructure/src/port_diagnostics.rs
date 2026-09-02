//! Bounded local TCP probing for Java-server reachability diagnostics.
use msc_domain::networking::{DiagnosticResult, classify_tcp_connection};
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::time::Duration;

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
