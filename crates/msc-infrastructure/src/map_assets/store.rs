//! No renderer consumes these inventory generations yet. Publication cannot adopt render output.
use super::*;
use msc_domain::map_assets::{Binding, Report, ResourceManifest};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
const QUOTA: u64 = 32 * 1024 * 1024 * 1024;
const MAX_MANIFEST: u64 = 32 * 1024 * 1024;

#[derive(Clone)]
pub struct Store {
    root: PathBuf,
    gate: Arc<Mutex<()>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pointer {
    pub binding: Binding,
    pub current: String,
    pub previous: Option<String>,
}
pub struct Candidate {
    store: Store,
    path: PathBuf,
    copied: u64,
    pub objects: Vec<String>,
    pub input_stamps: BTreeMap<PathBuf, String>,
}
/// A reader owns an immutable manifest/report snapshot.
/// This initial store never evicts published objects, including current/previous/leased data.
pub struct GenerationLease {
    pub manifest: ResourceManifest,
    pub report: Option<Report>,
}

impl Store {
    pub fn open(root: PathBuf) -> io::Result<Self> {
        if root.exists() {
            safe_path(&root)?;
        }
        fs::create_dir_all(root.join("objects"))?;
        for name in ["objects", "generations", "bindings", "candidates"] {
            fs::create_dir_all(root.join(name))?;
            safe_path(&root.join(name))?;
        }
        // Called once by the host store factory, before scan workers are admitted.
        let entries = fs::read_dir(root.join("candidates"))?
            .take(10_001)
            .collect::<Result<Vec<_>, _>>()?;
        if entries.len() > 10_000 {
            return Err(error("candidate_recovery_limit"));
        }
        for entry in entries {
            safe_path(&entry.path())?;
            if !entry.file_type()?.is_dir() {
                return Err(error("invalid_candidate"));
            }
            fs::remove_dir_all(entry.path())?;
        }
        Ok(Self {
            root,
            gate: Arc::new(Mutex::new(())),
        })
    }
    pub fn begin(&self) -> io::Result<Candidate> {
        let path = self
            .root
            .join("candidates")
            .join(Uuid::new_v4().to_string());
        fs::create_dir(&path)?;
        Ok(Candidate {
            store: self.clone(),
            path,
            copied: 0,
            objects: Vec::new(),
            input_stamps: BTreeMap::new(),
        })
    }
    pub fn pointer(&self, binding: &Binding) -> io::Result<Option<Pointer>> {
        let key = hash_json(&(
            binding.agent_host_id.clone(),
            binding.server_id.clone(),
            binding.slot_id.clone(),
        ))?;
        let path = self.root.join("bindings").join(format!("{key}.json"));
        if !path.exists() {
            return Ok(None);
        }
        let pointer: Pointer = serde_json::from_slice(&read(&path, MAX_JSON)?)
            .map_err(|_| error("invalid_generation_pointer"))?;
        if &pointer.binding != binding {
            return Ok(None);
        }
        Ok(Some(pointer))
    }
    pub fn lease(&self, id: &str) -> io::Result<GenerationLease> {
        if id.len() != 64 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(error("invalid_generation_id"));
        }
        let value: serde_json::Value = serde_json::from_slice(&read(
            &self.root.join("generations").join(format!("{id}.json")),
            MAX_MANIFEST,
        )?)
        .map_err(|_| error("invalid_generation"))?;
        let manifest: ResourceManifest = serde_json::from_value(value["manifest"].clone())
            .map_err(|_| error("invalid_generation"))?;
        let report: Report = serde_json::from_value(value["report"].clone())
            .map_err(|_| error("invalid_generation"))?;
        if hash_json(&(&manifest, &report))? != id
            || manifest.generation_id != report.resource_generation_id
        {
            return Err(error("generation_checksum_mismatch"));
        }
        Ok(GenerationLease {
            manifest,
            report: Some(report),
        })
    }
    pub fn publish(
        &self,
        candidate: &Candidate,
        manifest: &ResourceManifest,
        report: &Report,
        cancel: &dyn Fn() -> bool,
    ) -> io::Result<()> {
        let _guard = self.gate.lock().map_err(|_| error("store_unavailable"))?;
        poll(cancel)?;
        if candidate.store.root != self.root
            || manifest.generation_id != report.resource_generation_id
        {
            return Err(error("generation_binding_mismatch"));
        }
        candidate.verify_inputs(cancel)?;
        let generation = hash_json(&(manifest, report))?;
        let body = serde_json::to_vec(
            &serde_json::json!({"manifest":manifest,"report":report,"objects":candidate.objects}),
        )
        .map_err(|_| error("serialization_failed"))?;
        if body.len() as u64 > MAX_MANIFEST {
            return Err(error("generation_manifest_limit"));
        }
        self.admit(body.len() as u64 + MAX_JSON)?;
        let target = self
            .root
            .join("generations")
            .join(format!("{generation}.json"));
        if !target.exists() {
            write_new(&target, &body)?;
        } else {
            // A corrupt existing immutable record must never become the new current pointer.
            self.lease(&generation)?;
        }
        poll(cancel)?;
        let old = self.pointer(&report.binding)?;
        let pointer = Pointer {
            binding: report.binding.clone(),
            current: generation,
            previous: old.map(|p| p.current),
        };
        let key = hash_json(&(
            report.binding.agent_host_id.clone(),
            report.binding.server_id.clone(),
            report.binding.slot_id.clone(),
        ))?;
        let path = self.root.join("bindings").join(format!("{key}.json"));
        let temp = candidate.path.join("pointer.json");
        let bytes = serde_json::to_vec(&pointer).map_err(|_| error("serialization_failed"))?;
        write_new(&temp, &bytes)?;
        poll(cancel)?;
        fs::rename(temp, path)?;
        Ok(())
    }
    fn admit(&self, bytes: u64) -> io::Result<()> {
        let mut total = 0;
        let mut count = 0;
        let mut stack = vec![self.root.clone()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                count += 1;
                if count > 100_000 {
                    return Err(error("store_entry_limit"));
                }
                let m = fs::symlink_metadata(entry.path())?;
                if m.is_dir() {
                    stack.push(entry.path())
                } else if m.is_file() {
                    total += m.len()
                } else {
                    return Err(error("linked_store"));
                }
            }
        }
        if total + bytes > QUOTA {
            return Err(error("content_store_quota_protected_bytes"));
        }
        if fs2::available_space(&self.root)? < bytes + 64 * 1024 * 1024 {
            return Err(error("content_store_free_space"));
        }
        Ok(())
    }
}
impl Candidate {
    pub fn verify_inputs(&self, cancel: &dyn Fn() -> bool) -> io::Result<()> {
        for (path, before) in &self.input_stamps {
            poll(cancel)?;
            safe_path(path)?;
            if before != &stamp(&fs::symlink_metadata(path)?) {
                return Err(error("input_changed"));
            }
        }
        Ok(())
    }
    pub fn copy(
        &mut self,
        path: &Path,
        cancel: &dyn Fn() -> bool,
    ) -> io::Result<(PathBuf, String, u64)> {
        let file = open(path)?;
        let bytes = file.metadata()?.len();
        if bytes > MAX_ARCHIVE {
            return Err(error("archive_byte_limit"));
        }
        if self.copied + bytes > MAX_SCAN {
            return Err(error("candidate_reservation_limit"));
        }
        let _guard = self
            .store
            .gate
            .lock()
            .map_err(|_| error("store_unavailable"))?;
        self.store.admit(bytes)?;
        let original = stamp(&file.metadata()?);
        let temp = self.path.join(Uuid::new_v4().to_string());
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let mut input = file;
        let mut digest = Sha256::new();
        let mut buffer = [0; 65536];
        let mut copied = 0;
        loop {
            poll(cancel)?;
            let n = input.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            copied += n as u64;
            if copied > bytes {
                return Err(error("input_changed"));
            }
            digest.update(&buffer[..n]);
            output.write_all(&buffer[..n])?;
        }
        output.sync_all()?;
        if copied != bytes
            || original != stamp(&input.metadata()?)
            || original != stamp(&fs::symlink_metadata(path)?)
        {
            return Err(error("input_changed"));
        }
        let id = format!("{:x}", digest.finalize());
        let dest = self.store.root.join("objects").join(&id);
        if dest.exists() {
            if file_hash(&dest, MAX_ARCHIVE, cancel)? != id {
                return Err(error("content_store_corrupt_object"));
            }
            fs::remove_file(temp)?;
        } else {
            fs::rename(temp, &dest)?;
        }
        self.copied += bytes;
        self.objects.push(id.clone());
        self.input_stamps.insert(path.to_path_buf(), original);
        Ok((dest, id, bytes))
    }
}
impl Drop for Candidate {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

pub fn fingerprint_configs(
    root: &Path,
    cancel: &dyn Fn() -> bool,
) -> io::Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    if !root.exists() {
        return Ok(values);
    }
    safe_path(root)?;
    let mut stack = vec![root.to_path_buf()];
    let mut count = 0;
    let mut bytes = 0;
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(dir)? {
            poll(cancel)?;
            let e = e?;
            count += 1;
            if count > 10_000 {
                return Err(error("config_entry_limit"));
            }
            let m = fs::symlink_metadata(e.path())?;
            if m.is_dir() {
                safe_path(&e.path())?;
                stack.push(e.path())
            } else if m.is_file() {
                bytes += m.len();
                if m.len() > MAX_JSON || bytes > MAX_DOCUMENTS {
                    return Err(error("config_byte_limit"));
                }
                let key = e
                    .path()
                    .strip_prefix(root)
                    .map_err(|_| error("config_path"))?
                    .to_string_lossy()
                    .replace('\\', "/");
                values.insert(key, file_hash(&e.path(), MAX_JSON, cancel)?);
            } else {
                return Err(error("linked_input"));
            }
        }
    }
    Ok(values)
}
