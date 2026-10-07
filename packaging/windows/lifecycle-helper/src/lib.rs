//! MSI calls only these fixed operations from its embedded native DLL.
#![cfg(target_os = "windows")]

use std::{
    os::windows::{ffi::OsStringExt, process::CommandExt},
    process::Command,
};
use windows_sys::Win32::System::ApplicationInstallationAndServicing::{
    MsiCloseHandle, MsiCreateRecord, MsiGetPropertyW, MsiProcessMessage, MsiRecordSetStringW,
    INSTALLMESSAGE_ERROR, MSIHANDLE,
};

include!(concat!(env!("OUT_DIR"), "/payload.rs"));

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn property(session: MSIHANDLE, name: &str) -> Result<String, String> {
    let mut buffer = vec![0u16; 32_768];
    let mut length = (buffer.len() - 1) as u32;
    // SAFETY: MSI supplies the session handle; the property and writable buffer
    // remain alive for the call and the length bounds the output.
    let result = unsafe {
        MsiGetPropertyW(
            session,
            wide(name).as_ptr(),
            buffer.as_mut_ptr(),
            &mut length,
        )
    };
    if result != 0 {
        return Err("Could not read the bounded native lifecycle request.".into());
    }
    String::from_utf16(&buffer[..length as usize])
        .map_err(|_| "Invalid lifecycle request encoding.".into())
}

fn encoded(script: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<_> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut output = String::new();
    for block in bytes.chunks(3) {
        let a = block[0] as usize;
        let b = block.get(1).copied().unwrap_or(0) as usize;
        let c = block.get(2).copied().unwrap_or(0) as usize;
        output.push(ALPHABET[a >> 2] as char);
        output.push(ALPHABET[((a & 3) << 4) | (b >> 4)] as char);
        output.push(if block.len() > 1 {
            ALPHABET[((b & 15) << 2) | (c >> 6)] as char
        } else {
            '='
        });
        output.push(if block.len() > 2 {
            ALPHABET[c & 63] as char
        } else {
            '='
        });
    }
    output
}

fn invoke(session: MSIHANDLE, operation: &str) -> Result<(), String> {
    let data = property(session, "CustomActionData")?;
    let fields: Vec<_> = data.split('|').collect();
    if fields.len() != 5 {
        return Err("Invalid native lifecycle request.".into());
    }
    if !fields[3].is_empty() && !fields[3].eq_ignore_ascii_case("ALL") {
        return Err("MSC Setup supports complete package removal only.".into());
    }
    let request = serde_json::json!({
        "operation": operation, "transaction": fields[0], "packageRoot": fields[1],
        "previousRoot": fields[2], "remove": !fields[3].is_empty(),
        "installed": !fields[4].is_empty(),
    })
    .to_string()
    .replace('\'', "''");
    // The command carries fixed embedded code and literal JSON, never an
    // external script or registry command. MSI data selects no executable.
    let script = format!("{}\ntry {{ Invoke-MscDesktopLifecycle ('{request}' | ConvertFrom-Json) ('{PAYLOAD_HASHES}' | ConvertFrom-Json); exit 0 }} catch {{ [Console]::Error.WriteLine($_.Exception.Message); exit 1 }}", include_str!("../../desktop-service-lifecycle.ps1"));
    let script = encoded(&script);
    // Windows' command line is bounded. Feed the fixed script through stdin
    // instead, with no temporary elevated script path for another user to edit.
    let mut directory = [0u16; 32_768];
    // SAFETY: the writable buffer is valid for its stated capacity. Windows
    // supplies the system directory independently of caller environment data.
    let length = unsafe {
        windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW(
            directory.as_mut_ptr(),
            directory.len() as u32,
        )
    } as usize;
    if length == 0 || length >= directory.len() {
        return Err("Windows system directory unavailable.".into());
    }
    let powershell = std::path::PathBuf::from(std::ffi::OsString::from_wide(&directory[..length]))
        .join("WindowsPowerShell/v1.0/powershell.exe");
    let mut child = Command::new(powershell)
        .args(["-NoProfile", "-NonInteractive", "-Command", "-"])
        .creation_flags(0x0800_0000)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|_| "Could not open the native service coordinator.")?;
    use std::io::Write;
    // Decode in-memory so stdin receives a single fixed statement. -Command -
    // otherwise parses a multiline function interactively on older PowerShell.
    let command = format!("Invoke-Expression ([Text.Encoding]::Unicode.GetString([Convert]::FromBase64String('{script}')))\n");
    child
        .stdin
        .take()
        .ok_or("Could not open the coordinator input.")?
        .write_all(command.as_bytes())
        .map_err(|_| "Could not send the coordinator request.")?;
    let output = child
        .wait_with_output()
        .map_err(|_| "Could not wait for local service coordination.")?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr)
            .trim()
            .chars()
            .take(1600)
            .collect())
    }
}

fn action(session: MSIHANDLE, operation: &str) -> u32 {
    match std::panic::catch_unwind(|| invoke(session, operation)) {
        Ok(Ok(())) => 0,
        result => {
            let message = match result {
                Ok(Err(message)) => message,
                _ => "Native local service coordination failed.".into(),
            };
            // SAFETY: the new record is owned here and closed after MSI consumes
            // the localized error message. No credential enters this record.
            unsafe {
                let record = MsiCreateRecord(0);
                if record != 0 {
                    MsiRecordSetStringW(record, 0, wide(&message).as_ptr());
                    MsiProcessMessage(session, INSTALLMESSAGE_ERROR, record);
                    MsiCloseHandle(record);
                }
            }
            1603
        }
    }
}

#[no_mangle]
pub extern "system" fn MscPrepareService(session: MSIHANDLE) -> u32 {
    action(session, "prepare")
}
#[no_mangle]
pub extern "system" fn MscApplyService(session: MSIHANDLE) -> u32 {
    action(session, "apply")
}
#[no_mangle]
pub extern "system" fn MscResumeService(session: MSIHANDLE) -> u32 {
    action(session, "resume")
}
#[no_mangle]
pub extern "system" fn MscRollbackService(session: MSIHANDLE) -> u32 {
    action(session, "rollback")
}
#[no_mangle]
pub extern "system" fn MscCommitService(session: MSIHANDLE) -> u32 {
    action(session, "commit")
}
