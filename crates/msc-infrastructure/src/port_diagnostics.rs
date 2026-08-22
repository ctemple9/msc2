//! Bounded local TCP probing for Java-server reachability diagnostics.
use msc_domain::networking::{DiagnosticResult, classify_tcp_connection};
use std::net::{TcpStream, ToSocketAddrs};
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
