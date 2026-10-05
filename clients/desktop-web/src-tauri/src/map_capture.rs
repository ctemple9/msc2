//! User-invoked private capture sessions. Opening a map never starts a client.
use super::map_capture_process::Process;
use msc_infrastructure::map_assets::{self as assets, capture_client};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use tauri::Manager;

static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

struct Session {
    prepared: capture_client::Prepared,
    cancel: Arc<AtomicBool>,
    child: Option<Process>,
    termination_failed: bool,
}
fn sessions() -> &'static Mutex<BTreeMap<String, Session>> {
    static SESSIONS: OnceLock<Mutex<BTreeMap<String, Session>>> = OnceLock::new();
    SESSIONS.get_or_init(Mutex::default)
}
fn pending() -> &'static Mutex<BTreeMap<String, Arc<AtomicBool>>> {
    static PENDING: OnceLock<Mutex<BTreeMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    PENDING.get_or_init(Mutex::default)
}
fn valid_token(token: &str) -> bool {
    token.len() == 32 && token.bytes().all(|c| c.is_ascii_hexdigit())
}
fn stop(session: &mut Session) -> bool {
    session.cancel.store(true, Ordering::Release);
    if session.termination_failed {
        return false;
    }
    if let Some(mut child) = session.child.take() {
        if !child.stop() {
            session.termination_failed = true;
            let _ = std::fs::write(session.prepared.root.join("termination-unconfirmed"), b"Private client termination was not confirmed. Inspect processes before removing these files.");
            return false;
        }
    }
    true
}
impl Drop for Session {
    fn drop(&mut self) {
        stop(self);
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preparation {
    token: String,
    prepared: capture_client::Prepared,
    game_launched: bool,
}

#[tauri::command]
pub async fn prepare_map_capture(
    app: tauri::AppHandle,
    token: String,
    mut input: capture_client::Prepare,
) -> Result<Preparation, String> {
    if !valid_token(&token) {
        return Err("Invalid capture operation identifier.".into());
    }
    let cancel = Arc::new(AtomicBool::new(false));
    {
        let mut reservations = pending()
            .lock()
            .map_err(|_| "Capture sessions unavailable.")?;
        if reservations.contains_key(&token) || reservations.len() >= 2 {
            return Err("Wait for the current capture preparation.".into());
        }
        let root = app
            .path()
            .app_cache_dir()
            .map_err(|_| "Could not locate private capture storage.")?
            .join("map-captures");
        std::fs::create_dir_all(&root).map_err(|_| "Could not prepare capture storage.")?;
        let retained = std::fs::read_dir(&root)
            .map_err(|_| "Capture storage unavailable.")?
            .take(3)
            .count();
        // Reservations whose directories already exist are included in retained.
        let not_yet_created = reservations
            .keys()
            .filter(|id| !root.join(id).exists())
            .count();
        if retained + not_yet_created >= 2 {
            return Err("Two private capture instances are retained. Export or discard one before preparing another.".into());
        }
        input.destination = root.join(&token);
        reservations.insert(token.clone(), cancel.clone());
    }
    let worker_cancel = cancel.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        capture_client::prepare(&input, &|| worker_cancel.load(Ordering::Acquire))
    })
    .await;
    let mut reservations = pending()
        .lock()
        .map_err(|_| "Capture sessions unavailable.")?;
    reservations.remove(&token);
    let prepared = result
        .map_err(|_| "Capture preparation worker failed.")?
        .map_err(|e| format!("Capture preparation: {e}"))?;
    if cancel.load(Ordering::Acquire) {
        let _ = std::fs::remove_dir_all(&prepared.root);
        return Err("Capture preparation cancelled.".into());
    }
    let descriptor = serde_json::to_vec(&prepared).map_err(|_| {
        let _ = std::fs::remove_dir_all(&prepared.root);
        "Could not record capture session."
    })?;
    std::fs::write(prepared.root.join("session.json"), descriptor).map_err(|_| {
        let _ = std::fs::remove_dir_all(&prepared.root);
        "Could not record capture session."
    })?;
    let mut guard = sessions().lock().map_err(|_| {
        let _ = std::fs::remove_dir_all(&prepared.root);
        "Capture sessions unavailable."
    })?;
    guard.insert(
        token.clone(),
        Session {
            prepared: prepared.clone(),
            cancel,
            child: None,
            termination_failed: false,
        },
    );
    Ok(Preparation {
        token,
        prepared,
        game_launched: false,
    })
}

#[tauri::command]
pub fn launch_map_capture(token: String, launcher: String) -> Result<(), String> {
    let mut sessions = sessions()
        .lock()
        .map_err(|_| "Capture sessions unavailable.")?;
    if SHUTTING_DOWN.load(Ordering::Acquire) {
        return Err("The desktop is closing; no client was launched.".into());
    }
    let session = sessions
        .get_mut(&token)
        .ok_or("Capture preparation has expired.")?;
    if session.termination_failed {
        return Err("Private client termination is unconfirmed. Inspect its processes before using retained files.".into());
    }
    if session.child.is_some() {
        return Err("The dedicated client is already running.".into());
    }
    assets::safe_path(std::path::Path::new(&launcher))
        .map_err(|_| "Choose the installed Prism Launcher executable.")?;
    if session.cancel.load(Ordering::Acquire) {
        return Err(
            "This private session was stopped. Reopen its retained output before launching again."
                .into(),
        );
    }
    session.child = Some(
        Process::launch(
            std::path::Path::new(&launcher),
            &session.prepared.root,
            &session.prepared.instance_id,
        )
        .map_err(|_| {
            "Could not start the dedicated Prism client. Check its private launcher logs."
        })?,
    );
    Ok(())
}

#[tauri::command]
pub fn cancel_map_capture(token: String) -> Result<(), String> {
    if let Some(cancel) = pending()
        .lock()
        .map_err(|_| "Capture sessions unavailable.")?
        .get(&token)
    {
        cancel.store(true, Ordering::Release);
        return Ok(());
    }
    let mut sessions = sessions()
        .lock()
        .map_err(|_| "Capture sessions unavailable.")?;
    let session = sessions.get_mut(&token);
    let Some(session) = session else {
        return Ok(());
    };
    if !stop(session) {
        return Err(
            "The private client could not be confirmed stopped. Its files are retained.".into(),
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn inspect_map_capture_output(
    token: String,
) -> Result<super::map_assets::CaptureInspection, String> {
    let prepared = sessions()
        .lock()
        .map_err(|_| "Capture sessions unavailable.")?
        .get(&token)
        .ok_or("Capture session has expired.")?
        .prepared
        .clone();
    tauri::async_runtime::spawn_blocking(move || {
        let work = prepared.game.join(".msc-map-capture");
        let failure = work.join("failure.json");
        if failure.exists() {
            let receipt: serde_json::Value = serde_json::from_slice(
                &assets::read(&failure, 65536)
                    .map_err(|_| "Could not read the capture refusal.")?,
            )
            .map_err(|_| "Invalid capture refusal receipt.")?;
            let failed_request: msc_domain::map_assets::CaptureRequest =
                serde_json::from_value(receipt["request"].clone())
                    .map_err(|_| "Invalid capture refusal binding.")?;
            if failed_request != prepared.request {
                return Err("Capture refusal belongs to different inputs.".into());
            }
            let code = receipt["code"]
                .as_str()
                .filter(|v| {
                    !v.is_empty()
                        && v.len() <= 96
                        && v.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                })
                .ok_or("Invalid capture refusal code.")?;
            let detail = receipt["detail"]
                .as_str()
                .filter(|v| v.len() <= 2048 && !v.chars().any(|c| c.is_control()))
                .ok_or("Invalid capture refusal detail.")?;
            return Err(format!("Capture refused ({code}): {detail}"));
        }
        let exports = prepared.game.join(".msc-map-capture/exports");
        let name =
            String::from_utf8(assets::read(&exports.join("current"), 1024).map_err(|_| {
                "No complete capture was exported. Read the dedicated client's refusal message."
            })?)
            .map_err(|_| "Invalid export receipt.")?;
        if !assets::safe_member(&name) || name.contains('/') || !name.ends_with(".zip") {
            return Err("Invalid export receipt.".into());
        }
        let file = exports.join(name);
        let terrain = assets::saved_terrain::WorldSource::Directory(
            prepared.game.join(".msc-map-capture/snapshot"),
        )
        .inspect(
            &prepared.request.dimension,
            prepared.request.context_area,
            &|| false,
        )
        .map_err(|_| "Saved capture context is unavailable.")?;
        let sha = assets::file_hash(&file, assets::supplemental::MAX_CAPTURE, &|| false)
            .map_err(|_| "Capture exceeds the safe import limit.")?;
        let capture =
            assets::supplemental::import_bundle(&file, &sha, &prepared.request, &terrain, &|| {
                false
            })
            .map_err(|e| format!("Capture validation: {e}"))?;
        super::map_assets::retain_capture_import(&file, &sha, capture.manifest.clone())
    })
    .await
    .map_err(|_| "Capture inspection worker failed.".to_string())?
}

#[tauri::command]
pub fn discard_map_capture(token: String) -> Result<(), String> {
    let mut guard = sessions()
        .lock()
        .map_err(|_| "Capture sessions unavailable.")?;
    let session = guard
        .get_mut(&token)
        .ok_or("Capture session has expired.")?;
    if !stop(session) {
        return Err(
            "The private client could not be confirmed stopped. Its files are retained.".into(),
        );
    }
    let session = guard.remove(&token).ok_or("Capture session has expired.")?;
    std::fs::remove_dir_all(&session.prepared.root)
        .map_err(|_| "Could not remove private capture storage.".to_string())
}

#[tauri::command]
pub fn resume_map_capture(app: tauri::AppHandle, token: String) -> Result<Preparation, String> {
    if !valid_token(&token) {
        return Err("Invalid capture identifier.".into());
    }
    let reservations = pending()
        .lock()
        .map_err(|_| "Capture sessions unavailable.")?;
    if reservations.contains_key(&token) {
        return Err("Capture preparation is still running.".into());
    }
    let root = app
        .path()
        .app_cache_dir()
        .map_err(|_| "Capture storage unavailable.")?
        .join("map-captures")
        .join(&token);
    if root.join("termination-unconfirmed").exists() {
        return Err("Private client termination was not confirmed. Inspect its processes before recovering or removing retained files.".into());
    }
    let prepared: capture_client::Prepared = serde_json::from_slice(
        &assets::read(&root.join("session.json"), assets::MAX_JSON)
            .map_err(|_| "No complete preparation is retained.")?,
    )
    .map_err(|_| "Invalid retained preparation.")?;
    if prepared.root != root
        || prepared.game != root.join("instances/msc-capture/.minecraft")
        || prepared.instance_id != "msc-capture"
    {
        return Err("Invalid retained capture paths.".into());
    }
    let mut guard = sessions()
        .lock()
        .map_err(|_| "Capture sessions unavailable.")?;
    if let Some(session) = guard.get_mut(&token) {
        if session.termination_failed {
            return Err("Private client termination is unconfirmed.".into());
        }
        if session.child.is_none() {
            session.cancel.store(false, Ordering::Release);
        }
        return Ok(Preparation {
            token,
            prepared: session.prepared.clone(),
            game_launched: session.child.is_some(),
        });
    }
    guard.insert(
        token.clone(),
        Session {
            prepared: prepared.clone(),
            cancel: Arc::new(AtomicBool::new(false)),
            child: None,
            termination_failed: false,
        },
    );
    Ok(Preparation {
        token,
        prepared,
        game_launched: false,
    })
}

#[tauri::command]
pub fn map_capture_helpers(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    let packaged = app
        .path()
        .resource_dir()
        .map_err(|_| "Could not locate helper resources.")?
        .join("agent/map-capture/0.2.0");
    let root = if packaged.join("helpers.json").is_file() {
        packaged
    } else {
        capture_client::bundled_helpers(
            &app.path()
                .app_cache_dir()
                .map_err(|_| "Could not locate helper storage.")?
                .join("map-capture-helpers"),
        )
        .map_err(|e| e.to_string())?
    };
    let manifest: serde_json::Value = serde_json::from_slice(
        &assets::read(&root.join("helpers.json"), assets::MAX_JSON)
            .map_err(|_| "Helper manifest unavailable.")?,
    )
    .map_err(|_| "Invalid helper manifest.")?;
    Ok(serde_json::json!({"directory":root,"manifest":manifest}))
}

/// Polling cancellation drops the in-flight HTTP future, including during reads.
pub(super) async fn cancellable<F: std::future::Future>(
    work: F,
    cancel: Option<&AtomicBool>,
) -> Result<F::Output, String> {
    if cancel.is_none() {
        return Ok(work.await);
    }
    tokio::pin!(work);
    loop {
        if cancel.is_some_and(|flag| flag.load(Ordering::Acquire)) {
            return Err("Capture context transfer cancelled.".into());
        }
        tokio::select! {
            result = &mut work => return Ok(result),
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {}
        }
    }
}
fn context_root(app: &tauri::AppHandle, token: &str) -> Result<PathBuf, String> {
    if !valid_token(token) {
        return Err("Invalid capture identifier.".into());
    }
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|_| "Capture storage unavailable.")?
        .join("map-capture-contexts")
        .join(token))
}
#[tauri::command]
pub async fn download_map_capture_context(
    app: tauri::AppHandle,
    token: String,
    agent_host_id: String,
    slot_id: String,
    input: serde_json::Value,
) -> Result<PathBuf, String> {
    if slot_id.is_empty() || !slot_id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        return Err("Invalid world identifier.".into());
    }
    let root = context_root(&app, &token)?;
    let flag = Arc::new(AtomicBool::new(false));
    {
        let mut guard = pending()
            .lock()
            .map_err(|_| "Capture sessions unavailable.")?;
        if guard.len() >= 2 || guard.contains_key(&token) {
            return Err("Wait for the current capture preparation.".into());
        }
        guard.insert(token.clone(), flag.clone());
    }
    let result = async {
        let response = super::authorized_request(
            super::DesktopRequest {
                agent_host_id,
                method: "POST".into(),
                path: format!("/v1/worlds/{slot_id}/map-assets/capture-context"),
                headers: vec![
                    ("Content-Type".into(), "application/json".into()),
                    ("X-MSC-Client-Api-Version".into(), "1.0".into()),
                ],
                body: Some(serde_json::to_vec(&input).map_err(|_| "Invalid capture scope.")?),
            },
            Some(&flag),
        )
        .await?;
        if !(200..300).contains(&response.status) {
            let detail: serde_json::Value =
                serde_json::from_slice(&response.body).unwrap_or_default();
            return Err(format!(
                "Capture context refused ({}; {}): {}",
                response.status,
                detail["code"].as_str().unwrap_or("context_unavailable"),
                detail["message"]
                    .as_str()
                    .unwrap_or("Check the selected world's rendering report.")
            ));
        }
        if flag.load(Ordering::Acquire) {
            return Err("Capture context transfer cancelled.".into());
        }
        let parent = root.parent().ok_or("Capture storage unavailable.")?;
        std::fs::create_dir_all(parent)
            .map_err(|_| "Could not prepare capture context storage.")?;
        assets::safe_path(parent).map_err(|_| "Unsafe capture context storage.")?;
        if std::fs::read_dir(parent)
            .map_err(|_| "Capture context storage unavailable.")?
            .take(3)
            .count()
            >= 2
        {
            return Err("Discard an earlier capture context before downloading another.".into());
        }
        std::fs::create_dir(&root).map_err(|_| "Capture context already exists.")?;
        let saved = (|| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
                    .map_err(|_| "Could not protect capture context.")?;
            }
            use std::io::Write;
            let path = root.join("context.zip");
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|_| "Could not stage capture context.")?;
            for chunk in response.body.chunks(1024 * 1024) {
                if flag.load(Ordering::Acquire) {
                    return Err("Capture context transfer cancelled.");
                }
                file.write_all(chunk)
                    .map_err(|_| "Could not stage capture context.")?;
            }
            file.sync_all()
                .map_err(|_| "Could not finish capture context transfer.")?;
            Ok(path)
        })();
        if saved.is_err() {
            let _ = std::fs::remove_dir_all(&root);
        }
        saved.map_err(String::from)
    }
    .await;
    pending()
        .lock()
        .map_err(|_| "Capture sessions unavailable.")?
        .remove(&token);
    result
}
#[tauri::command]
pub fn discard_map_capture_context(app: tauri::AppHandle, token: String) -> Result<(), String> {
    let root = context_root(&app, &token)?;
    if root.exists() {
        assets::safe_path(&root).map_err(|_| "Unsafe capture context storage.")?;
        std::fs::remove_dir_all(root).map_err(|_| "Could not remove capture context.")?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_map_captures(app: tauri::AppHandle) -> Result<Vec<Preparation>, String> {
    let parent = app
        .path()
        .app_cache_dir()
        .map_err(|_| "Capture storage unavailable.")?
        .join("map-captures");
    if !parent.exists() {
        return Ok(Vec::new());
    }
    assets::safe_path(&parent).map_err(|_| "Unsafe capture storage.")?;
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&parent)
        .map_err(|_| "Capture storage unavailable.")?
        .take(3)
    {
        let entry = entry.map_err(|_| "Capture storage unavailable.")?;
        let token = entry.file_name().to_string_lossy().to_string();
        if !valid_token(&token) || assets::safe_path(&entry.path()).is_err() {
            continue;
        }
        let descriptor = entry.path().join("session.json");
        if !descriptor.exists() {
            let reservations = pending()
                .lock()
                .map_err(|_| "Capture sessions unavailable.")?;
            let guard = sessions()
                .lock()
                .map_err(|_| "Capture sessions unavailable.")?;
            if !reservations.contains_key(&token) && !guard.contains_key(&token) {
                let _ = std::fs::remove_dir_all(entry.path());
            }
            continue;
        }
        let Ok(bytes) = assets::read(&descriptor, assets::MAX_JSON) else {
            continue;
        };
        let Ok(prepared) = serde_json::from_slice::<capture_client::Prepared>(&bytes) else {
            continue;
        };
        if prepared.root != entry.path()
            || prepared.game != entry.path().join("instances/msc-capture/.minecraft")
            || prepared.instance_id != "msc-capture"
        {
            continue;
        }
        let running = sessions()
            .lock()
            .map_err(|_| "Capture sessions unavailable.")?
            .get(&token)
            .is_some_and(|session| session.child.is_some());
        found.push(Preparation {
            token,
            prepared,
            game_launched: running,
        });
    }
    Ok(found)
}

/// Window/process exit must not depend on a webview delivering its destroy hook.
pub(super) fn shutdown() {
    SHUTTING_DOWN.store(true, Ordering::Release);
    if let Ok(guard) = pending().lock() {
        for flag in guard.values() {
            flag.store(true, Ordering::Release);
        }
    }
    if let Ok(mut guard) = sessions().lock() {
        for session in guard.values_mut() {
            stop(session);
        }
    }
}
