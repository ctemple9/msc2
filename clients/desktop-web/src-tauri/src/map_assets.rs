//! Native read-only resource inspection; opaque handles keep scratch paths out of transport.
use msc_infrastructure::map_assets::bundle;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

struct Session {
    root: PathBuf,
    file: PathBuf,
    touched: Instant,
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn sessions() -> &'static Mutex<BTreeMap<String, Session>> {
    static SESSIONS: OnceLock<Mutex<BTreeMap<String, Session>>> = OnceLock::new();
    SESSIONS.get_or_init(Mutex::default)
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspection {
    token: String,
    size: u64,
    sha256: String,
    manifest: bundle::Manifest,
}

#[tauri::command]
pub async fn inspect_map_client_resources(
    source: String,
    context: Option<bundle::ClientContext>,
) -> Result<Inspection, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let token = format!("{:032x}", rand::random::<u128>());
        let root = std::env::temp_dir().join(format!("msc-map-import-{token}"));
        let mut guard = sessions().lock().map_err(|_| "Resource inspector unavailable.")?;
        guard.retain(|_, session| session.touched.elapsed() < Duration::from_secs(1800));
        if guard.len() >= 2 { return Err("Close an earlier resource inspection before opening another.".into()); }
        drop(guard);
        std::fs::create_dir(&root).map_err(|_| "Could not prepare private resource scratch.")?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).map_err(|_| "Could not protect resource scratch.")?; }
        let file = root.join("client-resources.zip");
        let session = Session { root: root.clone(), file: file.clone(), touched: Instant::now() };
        let source = Path::new(&source);
        let manifest = if source.extension().is_some_and(|s| s == "jar") {
            bundle::export_file(source, &root, &file, context.as_ref().ok_or("Select a bound map before importing an individual file.")?, &|| false)
        } else if source.is_dir() {
            bundle::export_instance(source, &root, &file, &|| false)
        } else {
            // Existing portable bundles are reinspected locally before any transfer.
            match bundle::unpack(source, &root.join("inspection"), &|| false) {
                Ok(manifest) => std::fs::copy(source, &file).map(|_| manifest),
                Err(error) if matches!(error.to_string().as_str(), "client_bundle_manifest_required" | "invalid_client_resource_manifest") => {
                    match bundle::export_archive(source, &root, &file, &|| false) {
                        Err(error) if error.to_string() == "client_instance_version_required" => bundle::export_file(source, &root, &file, context.as_ref().ok_or("Select a bound map before importing an individual pack.")?, &|| false),
                        result => result,
                    }
                },
                Err(error) => Err(error),
            }
        }.map_err(|e| format!("Client resources: {}. Select a matching Prism instance, an unambiguous official game directory, or an exported MSC resource bundle.", e))?;
        let size = std::fs::metadata(&file).map_err(|_| "Could not inspect bundle size.")?.len();
        let sha256 = msc_infrastructure::map_assets::file_hash(&file, bundle::MAX_BUNDLE, &|| false).map_err(|_| "Could not hash bundle.")?;
        let _ = std::fs::remove_dir_all(root.join("scan"));
        sessions().lock().map_err(|_| "Resource inspector unavailable.")?.insert(token.clone(), session);
        Ok(Inspection { token, size, sha256, manifest })
    }).await.map_err(|_| "Resource inspection worker failed.".to_string())?
}

#[tauri::command]
pub fn read_map_client_resources(
    token: String,
    offset: u64,
    max_bytes: usize,
) -> Result<tauri::ipc::Response, String> {
    if max_bytes == 0 || max_bytes > 8 * 1024 * 1024 {
        return Err("Resource chunks must be between 1 byte and 8 MiB.".into());
    }
    let mut guard = sessions()
        .lock()
        .map_err(|_| "Resource inspector unavailable.")?;
    let session = guard
        .get_mut(&token)
        .ok_or("This resource inspection has expired.")?;
    if session.touched.elapsed() > Duration::from_secs(1800) {
        return Err("This resource inspection has expired.".into());
    }
    session.touched = Instant::now();
    let mut file = msc_infrastructure::map_assets::open(&session.file)
        .map_err(|_| "The resource bundle is unavailable.")?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|_| "Invalid resource offset.")?;
    let mut bytes = Vec::new();
    file.take(max_bytes as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read resource chunk.")?;
    Ok(tauri::ipc::Response::new(bytes))
}
#[tauri::command]
pub fn discard_map_client_resources(token: String) -> Result<(), String> {
    sessions()
        .lock()
        .map_err(|_| "Resource inspector unavailable.")?
        .remove(&token);
    Ok(())
}
#[tauri::command]
pub fn export_map_client_resources(token: String, destination: String) -> Result<(), String> {
    let guard = sessions()
        .lock()
        .map_err(|_| "Resource inspector unavailable.")?;
    let session = guard
        .get(&token)
        .ok_or("This resource inspection has expired.")?;
    let destination = Path::new(&destination);
    if destination.exists() {
        return Err("Choose a new bundle filename; existing files are preserved.".into());
    }
    let mut source = msc_infrastructure::map_assets::open(&session.file)
        .map_err(|_| "Resource bundle unavailable.")?;
    let mut target = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| "Could not create the exported bundle.")?;
    std::io::copy(&mut source, &mut target).map_err(|_| "Could not export the complete bundle.")?;
    target
        .sync_all()
        .map_err(|_| "Could not finish the exported bundle.".to_string())
}

#[tauri::command]
pub fn export_map_rendering_report(report: String, destination: String) -> Result<(), String> {
    use std::io::Write;
    if report.len() > 32 * 1024 * 1024 {
        return Err("Report exceeds the safe export limit.".into());
    }
    let value: msc_domain::map_assets::Report =
        serde_json::from_str(&report).map_err(|_| "Invalid rendering report.")?;
    let bytes =
        serde_json::to_vec_pretty(&value).map_err(|_| "Could not encode rendering report.")?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| "Choose a new report filename; existing files are preserved.")?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Could not finish report export.".to_string())
}
