#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use msc_infrastructure::secret_store::SecretStore;
use reqwest::{header, Method, Url};
use serde::{Deserialize, Serialize};

const DESKTOP_CREDENTIAL_KEY_PREFIX: &str = "msc.desktop.host-token.";
const DESKTOP_SECRET_SERVICE: &str = "com.ctemple.msc2.desktop";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopPairingRequest {
    base_url: String,
    pairing_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopPairingResult {
    agent_host_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopRequest {
    agent_host_id: String,
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredDesktopCredential {
    base_url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopCredentialResult {
    agent_host_id: String,
    credential_id: String,
    token: String,
    expires_at: Option<String>,
}

/// Redeems a pairing code entirely in the native process. The webview receives
/// only the agent host ID; the bearer value goes directly to the platform
/// credential store and is never a Tauri command result.
#[tauri::command]
async fn desktop_exchange_pairing(
    request: DesktopPairingRequest,
) -> Result<DesktopPairingResult, String> {
    let base_url = canonical_base_url(&request.base_url)?;
    if request.pairing_code.trim().is_empty() {
        return Err("A desktop pairing code is required.".to_string());
    }
    let response = reqwest::Client::new()
        .post(format!("{base_url}/v1/auth/desktop-pairings"))
        .json(&serde_json::json!({ "pairingCode": request.pairing_code }))
        .send()
        .await
        .map_err(|error| format!("Desktop pairing request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Desktop pairing was refused (HTTP {}).",
            response.status()
        ));
    }
    let result: DesktopCredentialResult = response
        .json()
        .await
        .map_err(|error| format!("Desktop pairing returned an invalid response: {error}"))?;
    if result.agent_host_id.trim().is_empty()
        || result.credential_id.trim().is_empty()
        || !result.token.starts_with("msc2_")
    {
        return Err("Desktop pairing returned an invalid credential.".to_string());
    }
    let record = StoredDesktopCredential {
        base_url,
        token: result.token,
    };
    desktop_secret_store()?
        .set(
            &credential_key(&result.agent_host_id),
            &serde_json::to_string(&record).expect("desktop credential serializes"),
        )
        .map_err(|error| error.to_string())?;
    let _ = result.expires_at;
    Ok(DesktopPairingResult {
        agent_host_id: result.agent_host_id,
    })
}

/// Proxies one request to the origin stored with this host's credential. The
/// caller supplies a relative API path, so a compromised webview cannot turn
/// the shell into a bearer-token relay to a different origin.
#[tauri::command]
async fn desktop_authorized_request(request: DesktopRequest) -> Result<DesktopResponse, String> {
    let key = credential_key(&request.agent_host_id);
    let store = desktop_secret_store()?;
    let Some(record) = store.get(&key).map_err(|error| error.to_string())? else {
        return Err("This desktop has no credential for the selected host.".to_string());
    };
    let record: StoredDesktopCredential = serde_json::from_str(&record)
        .map_err(|error| format!("Stored desktop credential is invalid: {error}"))?;
    let method = Method::from_bytes(request.method.as_bytes())
        .map_err(|_| "The requested HTTP method is not supported.".to_string())?;
    let url = relative_request_url(&record.base_url, &request.path)?;
    let mut builder = reqwest::Client::new()
        .request(method, url)
        .header(header::AUTHORIZATION, format!("Bearer {}", record.token));
    for (name, value) in request.headers {
        let lower = name.to_ascii_lowercase();
        if matches!(lower.as_str(), "authorization" | "cookie" | "host") {
            continue;
        }
        builder = builder.header(name, value);
    }
    if let Some(body) = request.body {
        builder = builder.body(body);
    }
    let response = builder
        .send()
        .await
        .map_err(|error| format!("Desktop request failed: {error}"))?;
    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_string(), value.to_string()))
        })
        .collect();
    let body = response
        .bytes()
        .await
        .map_err(|error| format!("Desktop response could not be read: {error}"))?
        .to_vec();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        // A revoked or expired credential must not linger locally after the
        // agent has authoritatively rejected it.
        store.delete(&key).map_err(|error| error.to_string())?;
    }
    Ok(DesktopResponse {
        status: status.as_u16(),
        headers,
        body,
    })
}

fn credential_key(agent_host_id: &str) -> String {
    format!("{DESKTOP_CREDENTIAL_KEY_PREFIX}{agent_host_id}")
}

fn canonical_base_url(value: &str) -> Result<String, String> {
    let mut url =
        Url::parse(value).map_err(|_| "The agent address is not a valid URL.".to_string())?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("The agent address must be an HTTP(S) origin without credentials.".to_string());
    }
    url.set_path("");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn relative_request_url(base_url: &str, path: &str) -> Result<Url, String> {
    if !path.starts_with('/') || path.starts_with("//") || path.contains("..") {
        return Err("Desktop requests must use a safe relative API path.".to_string());
    }
    Url::parse(&format!("{base_url}{path}"))
        .map_err(|_| "The desktop request path is invalid.".to_string())
}

fn desktop_secret_store() -> Result<Box<dyn SecretStore>, String> {
    #[cfg(target_os = "macos")]
    {
        return msc_platform_macos::secret_store::MacosSecretStore::default_keychain_for_service(
            DESKTOP_SECRET_SERVICE,
        )
        .map(|store| Box::new(store) as Box<dyn SecretStore>)
        .map_err(|error| error.to_string());
    }
    #[cfg(target_os = "windows")]
    {
        return Ok(Box::new(
            msc_platform_windows::secret_store::WindowsSecretStore::new(),
        ));
    }
    #[cfg(target_os = "linux")]
    {
        return Ok(Box::new(
            msc_platform_linux::secret_store::LinuxSecretStore::new(),
        ));
    }
    #[allow(unreachable_code)]
    Err("No desktop credential store is available for this platform.".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            desktop_exchange_pairing,
            desktop_authorized_request
        ])
        .run(tauri::generate_context!())
        .expect("error while running the MSC 2 desktop shell");
}
