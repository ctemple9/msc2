//! Explicit server IDs feed the established deletion services, never paths from clients.
use super::{
    review::{Candidate, Owner},
    *,
};
use msc_application::{backups, worlds};
use msc_infrastructure::{backup_store, fs::StdFileSystem, storage, world_store};
use std::path::{Path, PathBuf};
#[derive(Clone)]
pub(super) struct Data {
    pub server_id: String,
    pub server_dir: PathBuf,
    pub root_id: String,
    pub domain_id: String,
    pub kind: String,
    pub bytes: Option<u64>,
    pub fingerprint: String,
}
fn ready(lifecycle: &LifecycleRoutesState, id: &str) -> Result<(), String> {
    if !matches!(
        lifecycle.reconciliation_status(id),
        super::super::lifecycle::ReconciliationStatus::Ready
    ) {
        return Err("Server reconciliation must complete before removal".into());
    }
    Ok(())
}
fn observe(path: &Path, kind: &str) -> Result<(u64, String), String> {
    let first = storage::observe_tree(path, &|| false).map_err(|e| e.to_string())?;
    if kind != "delete-backups" {
        return Ok(first);
    }
    let sidecar = backup_store::sidecar_path(path);
    match std::fs::symlink_metadata(&sidecar) {
        Ok(_) => {
            let second = storage::observe_tree(&sidecar, &|| false).map_err(|e| e.to_string())?;
            Ok((
                first.0.saturating_add(second.0),
                storage::identity(&[&first.1, &second.1]),
            ))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Ok((first.0, storage::identity(&[&first.1, "no-sidecar"])))
        }
        Err(e) => Err(e.to_string()),
    }
}
pub(super) fn candidates(
    lifecycle: &LifecycleRoutesState,
    host: &str,
    action: &str,
) -> Result<Vec<Candidate>, String> {
    let mut result = vec![];
    let start = std::time::Instant::now();
    for server in lifecycle.app_config_servers().into_iter().take(512) {
        if start.elapsed() > Duration::from_secs(300) || result.len() >= 512 {
            return Err(
                "Server-data review budget reached; narrow the inventory before removal".into(),
            );
        }
        let directory = PathBuf::from(&server.server_dir);
        let root_id = storage::verified_root_id(host, &directory).map_err(|e| e.to_string())?;
        let state = ready(lifecycle, &server.id).and_then(|()| {
            if action == "remove-server"
                && lifecycle.app_config_servers().iter().any(|other| {
                    other.id != server.id
                        && (Path::new(&other.server_dir).starts_with(&directory)
                            || directory.starts_with(&other.server_dir))
                })
            {
                Err("Overlapping registered server roots are protected".into())
            } else {
                Ok(())
            }
        });
        let mut sources: Vec<(String, String, PathBuf, Result<(), String>)> = vec![];
        match action {
            "delete-backups" => {
                let entries = backups::list_backups(&StdFileSystem, &directory);
                let verified = entries.iter().filter(|e| e.verified).count();
                for entry in entries.into_iter().take(512) {
                    sources.push((
                        entry.filename,
                        entry.display_name,
                        entry.zip_path,
                        if entry.verified && verified <= 1 {
                            Err("Last verified backup is protected".into())
                        } else {
                            state.clone()
                        },
                    ));
                }
            }
            "remove-world" => {
                let active = active_slot(&directory);
                for slot in world_store::inspect_slots(&StdFileSystem, &directory)
                    .into_iter()
                    .take(512)
                {
                    let eligible = match &active {
                        Err(error) => Err(error.clone()),
                        Ok(active) if active.as_deref() == Some(&slot.id) => {
                            Err("Active world slot is protected".into())
                        }
                        _ => state.clone(),
                    };
                    sources.push((
                        slot.id.clone(),
                        slot.name.clone(),
                        world_store::slot_directory(&directory, &slot.id),
                        eligible,
                    ));
                }
            }
            "remove-server" => {
                let running = lifecycle.active_server_id().as_deref() == Some(&server.id)
                    && lifecycle.status_snapshot().running;
                sources.push((
                    server.id.clone(),
                    server.display_name.clone(),
                    directory.clone(),
                    if running {
                        Err("Stop the server explicitly before reviewing removal".into())
                    } else {
                        state.clone()
                    },
                ));
            }
            _ => return Err("Unsupported server-data action".into()),
        }
        for (domain_id, label, path, eligibility) in sources {
            let measurement = match &eligibility {
                Ok(()) => observe(&path, action),
                Err(e) => Err(e.clone()),
            };
            let bytes = measurement.as_ref().ok().map(|m| m.0);
            let fingerprint = measurement
                .as_ref()
                .ok()
                .map(|m| m.1.clone())
                .unwrap_or_default();
            let reason=measurement.as_ref().err().cloned().unwrap_or_else(||if action=="remove-server"{format!("Permanently removes this server directory, its live worlds, saved world slots, backups, components and logs: {}",directory.display())}else if action=="remove-world"{"Permanently removes the saved slot and its source archive; active world is unchanged".into()}else{"Permanently removes this backup ZIP and paired metadata; retention floor checked again on removal".into()});
            let id = storage::identity(&[host, &root_id, action, &server.id, &domain_id]);
            let mut consequences = vec![];
            if action == "remove-server" {
                consequences.push(format!("Server directory: {}", directory.display()));
                consequences.extend(
                    world_store::inspect_slots(&StdFileSystem, &directory)
                        .into_iter()
                        .take(512)
                        .map(|s| format!("Saved world: {}", s.name)),
                );
                consequences.extend(
                    backups::list_backups(&StdFileSystem, &directory)
                        .into_iter()
                        .take(512)
                        .map(|b| format!("Backup: {}", b.display_name)),
                );
                consequences.push("All live world folders, components, configuration and logs inside this directory are also removed. External files are not included.".into());
            }
            result.push(Candidate {
                item: msc_api::dto::StorageCandidateDto {
                    server_id: Some(server.id.clone()),
                    id,
                    label: format!("{} · {label}", server.display_name),
                    display_path: Some(path.display().to_string()),
                    logical_bytes: bytes,
                    consequences,
                    removable: eligibility.is_ok() && measurement.is_ok(),
                    reason,
                },
                owner: Owner::Data(Data {
                    server_id: server.id.clone(),
                    server_dir: directory.clone(),
                    root_id: root_id.clone(),
                    domain_id,
                    kind: action.into(),
                    bytes,
                    fingerprint,
                }),
            });
        }
    }
    Ok(result)
}
pub(super) fn remove(
    lifecycle: &LifecycleRoutesState,
    host: &str,
    data: &Data,
) -> Result<u64, String> {
    let server = lifecycle
        .app_config_servers()
        .into_iter()
        .find(|s| s.id == data.server_id)
        .ok_or("Server no longer exists")?;
    if Path::new(&server.server_dir) != data.server_dir
        || storage::verified_root_id(host, &data.server_dir).map_err(|e| e.to_string())?
            != data.root_id
    {
        return Err("Server directory or ownership changed".into());
    }
    ready(lifecycle, &server.id)?;
    if data.kind == "remove-server"
        && lifecycle.app_config_servers().iter().any(|other| {
            other.id != server.id
                && (Path::new(&other.server_dir).starts_with(&data.server_dir)
                    || data.server_dir.starts_with(&other.server_dir))
        })
    {
        return Err("Overlapping registered server roots are protected".into());
    }
    let path = match data.kind.as_str() {
        "delete-backups" => {
            let entries = backups::list_backups(&StdFileSystem, &data.server_dir);
            let entry = entries
                .iter()
                .find(|e| e.filename == data.domain_id)
                .ok_or("Backup identity no longer exists")?;
            if entry.verified && entries.iter().filter(|e| e.verified).count() <= 1 {
                return Err("Last verified backup is protected".into());
            }
            entry.zip_path.clone()
        }
        "remove-world" => {
            let slot = find_slot(&data.server_dir, &data.domain_id)
                .ok_or("World slot no longer exists")?;
            if active_slot(&data.server_dir)?.as_deref() == Some(&slot.id) {
                return Err("Active world slot is protected".into());
            }
            world_store::slot_directory(&data.server_dir, &slot.id)
        }
        "remove-server" => {
            if lifecycle.active_server_id().as_deref() == Some(&server.id)
                && lifecycle.status_snapshot().running
            {
                return Err("Server is running; stop it explicitly before another review".into());
            }
            data.server_dir.clone()
        }
        _ => return Err("Unsupported server-data action".into()),
    };
    let measurement = observe(&path, &data.kind)?;
    if Some(measurement.0) != data.bytes || measurement.1 != data.fingerprint {
        return Err("Server data changed since review; refresh".into());
    }
    match data.kind.as_str() {
        "delete-backups" => {
            backups::delete_backup(&StdFileSystem, &path)
                .map_err(|e| format!("Removal unconfirmed: {e}"))?;
            if path.exists() || backup_store::sidecar_path(&path).exists() {
                return Err("Removal unconfirmed: backup metadata remains".into());
            }
        }
        "remove-world" => {
            let slot =
                find_slot(&data.server_dir, &data.domain_id).ok_or("World identity changed")?;
            let active = active_slot(&data.server_dir)?;
            worlds::delete_slot(&StdFileSystem, &data.server_dir, &slot, active.as_deref())
                .map_err(|e| format!("Removal unconfirmed: {e}"))?;
        }
        "remove-server" => {
            lifecycle
                .delete_fleet_server(&server.id)
                .map_err(|e| format!("Removal unconfirmed: {e:?}"))?;
        }
        _ => unreachable!(),
    }
    Ok(measurement.0)
}

fn find_slot(dir: &Path, id: &str) -> Option<msc_domain::world::WorldSlot> {
    world_store::inspect_slots(&StdFileSystem, dir)
        .into_iter()
        .find(|slot| slot.id == id)
}
fn active_slot(dir: &Path) -> Result<Option<String>, String> {
    let slots = world_store::inspect_slots(&StdFileSystem, dir);
    let marker = match storage::read_small(&world_store::active_marker_path(dir), 4096) {
        Ok(bytes) => Some(
            String::from_utf8(bytes)
                .map_err(|_| "Active world marker is unreadable; world removal is protected")?
                .trim()
                .to_string(),
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => {
            return Err(
                "Active world marker cannot be verified; world removal is protected".into(),
            );
        }
    };
    Ok(msc_domain::world::resolve_active_slot_id(
        &slots,
        marker.as_deref(),
    ))
}
