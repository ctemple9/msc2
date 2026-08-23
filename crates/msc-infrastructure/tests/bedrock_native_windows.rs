use msc_infrastructure::bedrock_native::{
    NativeBedrockError, WINDOWS_BEDROCK_EXECUTABLE_NAME, preflight_udp_bind,
    windows_bedrock_spawn_request,
};
use std::net::UdpSocket;
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::fs::{self, File, OpenOptions};
#[cfg(target_os = "windows")]
use std::io::ErrorKind;
#[cfg(target_os = "windows")]
use std::os::windows::fs::OpenOptionsExt;

#[test]
fn windows_bedrock_request_uses_exe_and_preserves_server_directory() {
    let server_dir = PathBuf::from(r"C:\MSC\servers\bedrock");
    let request = windows_bedrock_spawn_request(&server_dir);

    assert_eq!(
        request.executable_path,
        server_dir.join(WINDOWS_BEDROCK_EXECUTABLE_NAME)
    );
    assert_eq!(request.working_directory, server_dir);
}

#[test]
fn windows_direct_udp_preflight_reports_an_occupied_port() {
    let listener = UdpSocket::bind("0.0.0.0:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    assert!(matches!(
        preflight_udp_bind("0.0.0.0".parse().unwrap(), port),
        Err(NativeBedrockError::UdpPortInUse { port: actual, .. }) if actual == port
    ));
}

#[test]
#[cfg(target_os = "windows")]
fn windows_server_directory_file_lock_is_observable_before_start() {
    let path = std::env::temp_dir().join(format!(
        "msc2-bedrock-windows-lock-{}.dat",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);
    let locked = OpenOptions::new()
        .create_new(true)
        .write(true)
        .share_mode(0)
        .open(&path)
        .unwrap();

    let second_open = OpenOptions::new().write(true).open(&path);
    assert!(matches!(
        second_open,
        Err(error) if matches!(error.kind(), ErrorKind::PermissionDenied | ErrorKind::Other)
    ));

    drop(locked);
    let _reopened: File = OpenOptions::new().write(true).open(&path).unwrap();
    fs::remove_file(path).unwrap();
}
