//! Storage invokes transfer owners; it never adopts a transfer path as authority.
use super::*;
use msc_infrastructure::storage;

#[derive(Clone)]
pub(crate) struct Entry {
    pub volume_path: std::path::PathBuf,
    pub id: String,
    pub label: String,
    pub bytes: Option<u64>,
    pub removable: bool,
    pub reason: String,
    pub(crate) fingerprint: String,
}
impl ComponentsRoutesState {
    pub(crate) async fn storage_candidates(&self) -> Vec<Entry> {
        let mut items = native_upload::storage_candidates(self).await;
        let sources = {
            let prepared = self.staging.downloads.lock().unwrap();
            let active = self.staging.active_downloads.lock().unwrap();
            prepared
                .iter()
                .map(|(id, e)| (id.clone(), e.path.clone(), now_unix() > e.expires_at_unix))
                .chain(
                    active
                        .iter()
                        .map(|(id, e)| (id.clone(), e.path.clone(), *e.done.borrow())),
                )
                .take(256)
                .collect::<Vec<_>>()
        };
        for (id, path, inactive) in sources {
            let volume_path = path.parent().unwrap_or(&path).to_path_buf();
            let measured = if inactive {
                tokio::task::spawn_blocking(move || storage::observe_tree(&path, &|| false))
                    .await
                    .ok()
                    .and_then(Result::ok)
            } else {
                None
            };
            items.push(Entry {
                volume_path,
                id: format!("download:{id}"),
                label: "Prepared download".into(),
                bytes: measured.as_ref().map(|m| m.0),
                removable: inactive && measured.is_some(),
                reason: if inactive {
                    "Expired or completed transfer; checked again on removal"
                } else {
                    "Active transfer is protected"
                }
                .into(),
                fingerprint: measured.map(|m| m.1).unwrap_or_default(),
            });
        }
        items
    }
    pub(crate) async fn storage_remove(&self, entry: &Entry) -> Result<u64, String> {
        let Some(id) = entry.id.strip_prefix("download:") else {
            return native_upload::storage_remove(self, entry).await;
        };
        // Same lock order as the streaming handoff. Removal is synchronous while
        // held so no new reader can acquire a file between validation and unlink.
        let mut prepared = self.staging.downloads.lock().unwrap();
        let mut active = self.staging.active_downloads.lock().unwrap();
        let (path, owner) = if let Some(e) = prepared.get(id) {
            if now_unix() <= e.expires_at_unix {
                return Err("Download is no longer expired".into());
            }
            (e.path.clone(), e.owner.clone())
        } else if let Some(e) = active.get(id) {
            if !*e.done.borrow() {
                return Err("Download has an active reader".into());
            }
            (e.path.clone(), e.owner.clone())
        } else {
            return Err("Transfer ownership no longer exists; refresh".into());
        };
        let observed = storage::observe_tree(&path, &|| false).map_err(|e| e.to_string())?;
        if Some(observed.0) != entry.bytes || observed.1 != entry.fingerprint {
            return Err("Download changed since review".into());
        }
        std::fs::remove_file(path).map_err(|e| format!("Removal unconfirmed: {e}"))?;
        prepared.remove(id);
        active.remove(id);
        record_download_cleanup(&self.staging, id, &owner);
        Ok(observed.0)
    }
}
