//! Windows named-pipe ACLs and kernel-authenticated client identities.

use std::ffi::c_void;
use std::mem::size_of;
use std::os::windows::io::AsRawHandle;
use std::ptr;

use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::System::Pipes::ImpersonateNamedPipeClient;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, GetCurrentThread, OpenProcessToken, OpenThreadToken,
};
use windows_sys::core::PCWSTR;

pub const PIPE_NAME: &str = r"\\.\pipe\msc2-local-cli-v1";

struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: this wrapper owns a live token handle returned by Open*Token.
        unsafe { CloseHandle(self.0) };
    }
}

/// Returns the account SID from the agent process token, not an environment
/// variable or installer-supplied name.
pub fn service_sid() -> Result<String, String> {
    let mut token = ptr::null_mut();
    // SAFETY: token is a valid output pointer and GetCurrentProcess is a valid
    // pseudo-handle for this process.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(last_error("opening agent process token"));
    }
    token_user_sid(OwnedHandle(token))
}

/// Impersonates the connected named-pipe client long enough to read its token
/// SID. The caller must not await between connection and this call.
pub fn peer_sid(pipe: &impl AsRawHandle) -> Result<String, String> {
    let handle = pipe.as_raw_handle().cast::<c_void>();
    // SAFETY: handle is the live server endpoint of an accepted named pipe.
    if unsafe { ImpersonateNamedPipeClient(handle) } == 0 {
        return Err(last_error("impersonating local CLI pipe client"));
    }

    let mut token = ptr::null_mut();
    // SAFETY: impersonation established a thread token; token is an output
    // pointer, and TRUE asks Windows to check the process security context.
    let token_result =
        if unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, &mut token) } == 0 {
            Err(last_error("opening local CLI client token"))
        } else {
            token_user_sid(OwnedHandle(token))
        };
    // Always drop impersonation before returning to async code on this worker.
    // SAFETY: this thread successfully impersonated the pipe client above.
    if unsafe { windows_sys::Win32::Security::RevertToSelf() } == 0 {
        // A Tokio worker must never be reused while it still carries a client token.
        std::process::abort();
    }
    token_result
}

/// Creates a local-only pipe instance whose protected DACL grants full pipe
/// access only to the account running this service.
pub fn create_server_pipe(
    service_sid: &str,
    first_instance: bool,
) -> Result<NamedPipeServer, String> {
    let sddl = format!("D:P(A;;GA;;;{service_sid})");
    let sddl_wide: Vec<u16> = sddl.encode_utf16().chain(std::iter::once(0)).collect();
    let mut descriptor = ptr::null_mut();
    // SAFETY: sddl_wide is NUL-terminated and descriptor is a valid output
    // pointer. Windows allocates the descriptor, released below with LocalFree.
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_wide.as_ptr() as PCWSTR,
            SDDL_REVISION_1,
            &mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(last_error("building local CLI named-pipe ACL"));
    }

    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let result = unsafe {
        ServerOptions::new()
            .first_pipe_instance(first_instance)
            .reject_remote_clients(true)
            .access_inbound(true)
            .access_outbound(true)
            .create_with_security_attributes_raw(
                PIPE_NAME,
                (&mut attributes as *mut SECURITY_ATTRIBUTES).cast(),
            )
    };
    // CreateNamedPipe copies the descriptor into the new pipe object.
    unsafe { LocalFree(descriptor) };
    result.map_err(|error| format!("creating local CLI named pipe: {error}"))
}

fn token_user_sid(token: OwnedHandle) -> Result<String, String> {
    let mut required = 0u32;
    // SAFETY: a null data buffer with zero length is the documented size query.
    unsafe {
        GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut required);
    }
    if required == 0 {
        return Err(last_error("sizing Windows user token information"));
    }
    let word_count = (required as usize).div_ceil(size_of::<usize>());
    let mut storage = vec![0usize; word_count];
    // SAFETY: storage is suitably aligned and large enough per the size query.
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            storage.as_mut_ptr().cast(),
            required,
            &mut required,
        )
    } == 0
    {
        return Err(last_error("reading Windows user token information"));
    }
    // SAFETY: GetTokenInformation filled the buffer with TOKEN_USER.
    let token_user = unsafe { &*storage.as_ptr().cast::<TOKEN_USER>() };
    sid_to_string(token_user.User.Sid)
}

fn sid_to_string(sid: *mut c_void) -> Result<String, String> {
    let mut wide = ptr::null_mut();
    // SAFETY: sid points into a live TOKEN_USER buffer; Windows allocates wide.
    if unsafe { ConvertSidToStringSidW(sid, &mut wide) } == 0 {
        return Err(last_error("converting Windows account SID"));
    }
    let mut length = 0usize;
    // SAFETY: ConvertSidToStringSidW returns a NUL-terminated string allocation.
    unsafe {
        while *wide.add(length) != 0 {
            length += 1;
        }
    }
    // SAFETY: `wide` has at least `length` readable UTF-16 elements.
    let value = unsafe { String::from_utf16(std::slice::from_raw_parts(wide, length)) }
        .map_err(|error| format!("Windows account SID is not valid UTF-16: {error}"));
    // SAFETY: Windows allocated this string; LocalFree releases it.
    unsafe { LocalFree(wide.cast()) };
    value
}

fn last_error(action: &str) -> String {
    // SAFETY: GetLastError reads the current thread's Win32 error value.
    let error = unsafe { GetLastError() };
    format!("{action}: Windows error {error}")
}
