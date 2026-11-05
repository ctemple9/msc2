//! Read only datapack entries from an explicitly selected world slot. Saved
//! world archives are never extracted wholesale and no terrain/player data is
//! copied into publication staging.
use msc_domain::app_config_schema::ConfigServer;
use msc_domain::content_collection::validate_relative_path;
use msc_infrastructure::{fs::StdFileSystem, world_store};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use sha2::Sha512;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Selection {
    pub slot_id: String,
    pub path: String,
    pub sha512: String,
    pub redistribution_evidence: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub slot_id: String,
    pub path: String,
    pub sha512: String,
    pub sha1: String,
    pub byte_size: u64,
    pub source_relative: Option<String>,
}
fn single(value: &str) -> Result<(), String> {
    validate_relative_path(value)?;
    if value.contains('/') {
        return Err("World identity must be a single directory name".into());
    }
    Ok(())
}
fn selected(path: &str, paths: Option<&std::collections::BTreeSet<String>>) -> bool {
    paths.is_none_or(|paths| paths.contains(path))
}
fn active_files(
    server: &ConfigServer,
    root: &str,
    slot_id: &str,
    paths: Option<&std::collections::BTreeSet<String>>,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<File>, String> {
    let mut files = Vec::new();
    let mut pending = vec![(root.to_string(), String::new())];
    let mut count = 0usize;
    while let Some((directory, prefix)) = pending.pop() {
        if cancelled() {
            return Err("Datapack inspection cancelled".into());
        }
        let absolute = crate::content_publications::checked_source(server, &directory)?;
        let meta = std::fs::symlink_metadata(&absolute).map_err(|error| error.to_string())?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err("Datapack inspection does not follow directory symlinks".into());
        }
        for entry in std::fs::read_dir(absolute).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            count += 1;
            if count > 50_000 {
                return Err("Datapack folder exceeds its entry limit".into());
            }
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "Datapack name is not Unicode")?;
            let relative = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            validate_relative_path(&relative)?;
            if relative.split('/').count() > 64 {
                return Err("Datapack directory is too deep".into());
            }
            let metadata =
                std::fs::symlink_metadata(entry.path()).map_err(|error| error.to_string())?;
            if metadata.file_type().is_symlink() {
                return Err("Datapack inspection does not follow symbolic links".into());
            }
            if metadata.is_dir() {
                pending.push((format!("{root}/{relative}"), relative));
            } else if metadata.is_file() && selected(&relative, paths) {
                let source_relative = format!("{root}/{relative}");
                let actual = msc_infrastructure::addon_provider::fingerprint::identify(
                    &crate::content_publications::checked_source(server, &source_relative)?,
                )?;
                files.push(File {
                    slot_id: slot_id.into(),
                    path: relative,
                    sha512: actual.sha512,
                    sha1: actual.sha1,
                    byte_size: actual.byte_size,
                    source_relative: Some(source_relative),
                });
            }
        }
    }
    Ok(files)
}
fn archive_entry(
    mut input: impl Read,
    size: u64,
    destination: Option<&Path>,
    cancelled: &dyn Fn() -> bool,
) -> Result<(String, String), String> {
    if size > 512 * 1024 * 1024 {
        return Err("Datapack file exceeds 512 MiB".into());
    }
    let mut output = if let Some(path) = destination {
        let parent = path.parent().ok_or("Missing datapack staging parent")?;
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        if fs2::available_space(parent).map_err(|error| error.to_string())? < size {
            return Err("Insufficient space to stage selected datapack".into());
        }
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        Some(options.open(path).map_err(|error| error.to_string())?)
    } else {
        None
    };
    let result = (|| {
        let mut sha1 = Sha1::new();
        let mut sha512 = Sha512::new();
        let mut count = 0u64;
        let mut buffer = [0u8; 128 * 1024];
        loop {
            if cancelled() {
                return Err("Datapack inspection cancelled".into());
            }
            let read = input.read(&mut buffer).map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            count += read as u64;
            if count > size {
                return Err("Datapack entry grew while reading".into());
            }
            sha1.update(&buffer[..read]);
            sha512.update(&buffer[..read]);
            if let Some(output) = output.as_mut() {
                output
                    .write_all(&buffer[..read])
                    .map_err(|error| error.to_string())?;
            }
        }
        if count != size {
            return Err("Datapack archive entry size changed".into());
        }
        if let Some(output) = output.as_mut() {
            output.sync_all().map_err(|error| error.to_string())?;
        }
        Ok((
            format!("{:x}", sha1.finalize()),
            format!("{:x}", sha512.finalize()),
        ))
    })();
    drop(output);
    if result.is_err()
        && let Some(path) = destination
    {
        let _ = std::fs::remove_file(path);
    }
    result
}

pub fn inspect(
    server: &ConfigServer,
    slot_id: &str,
    paths: Option<&std::collections::BTreeSet<String>>,
    retain: bool,
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<File>, String> {
    single(slot_id)?;
    if server.server_type != msc_domain::identity::ServerType::Java {
        return Err("Datapack publication requires a Java world".into());
    }
    if let Some(paths) = paths {
        for path in paths {
            validate_relative_path(path)?;
        }
    }
    let slots = world_store::inspect_slots(&StdFileSystem, Path::new(&server.server_dir));
    let slot = slots
        .iter()
        .find(|slot| slot.id == slot_id)
        .ok_or("World slot does not belong to this server")?;
    let marker =
        world_store::load_explicit_active_slot_id(&StdFileSystem, Path::new(&server.server_dir));
    let active = msc_domain::world::resolve_active_slot_id(&slots, marker.as_deref()).as_deref()
        == Some(slot_id);
    crate::content_publications::checked_source(server, &format!("world_slots/{slot_id}"))?;
    let profile = world_store::load_profile(&StdFileSystem, Path::new(&server.server_dir), slot);
    let level = if active {
        crate::worlds::read_configured_level_name(&StdFileSystem, Path::new(&server.server_dir))
            .or(profile.identity.level_name.clone())
            .or(slot.world_level_name.clone())
    } else {
        profile
            .identity
            .level_name
            .clone()
            .or(slot.world_level_name.clone())
    }
    .ok_or("World slot has no known level folder")?;
    single(&level)?;
    let live_root = format!("{level}/datapacks");
    let live = crate::content_publications::checked_source(server, &live_root)?;
    let live_world = crate::content_publications::checked_source(server, &level)?;
    let live_world_exists = active && live_world.try_exists().map_err(|error| error.to_string())?;
    let mut files = if live_world_exists {
        if std::fs::symlink_metadata(&live_world)
            .map_err(|error| error.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("World datapack inspection does not follow directory symlinks".into());
        }
        if live.try_exists().map_err(|error| error.to_string())? {
            active_files(server, &live_root, slot_id, paths, cancelled)?
        } else {
            Vec::new()
        }
    } else {
        let world_relative = format!("world_slots/{slot_id}/world.zip");
        let world = crate::content_publications::checked_source(server, &world_relative)?;
        let archive_path = if world.try_exists().map_err(|error| error.to_string())? {
            world
        } else {
            crate::content_install::managed_path(
                &server.server_dir,
                &format!("world_slots/{slot_id}/packs.zip"),
            )?
        };
        let metadata =
            std::fs::symlink_metadata(&archive_path).map_err(|error| error.to_string())?;
        if !metadata.is_file() {
            return Err("World archive must be a regular file".into());
        }
        let input = std::fs::File::open(archive_path).map_err(|error| error.to_string())?;
        let mut archive = zip::ZipArchive::new(input).map_err(|error| error.to_string())?;
        if archive.len() > 500_000 {
            return Err("World archive exceeds inspection entry limit".into());
        }
        let prefix = format!("{level}/datapacks/");
        let staging_id = uuid::Uuid::new_v4();
        let mut files = Vec::new();
        let mut total = 0u64;
        for index in 0..archive.len() {
            if cancelled() {
                return Err("Datapack inspection cancelled".into());
            }
            let entry = archive.by_index(index).map_err(|error| error.to_string())?;
            if entry.is_dir() {
                continue;
            }
            let Some(relative) = entry.name().strip_prefix(&prefix).map(str::to_string) else {
                continue;
            };
            validate_relative_path(&relative)?;
            if !selected(&relative, paths) {
                continue;
            }
            if entry
                .unix_mode()
                .is_some_and(|mode| mode & 0o170000 == 0o120000)
            {
                return Err("Selected datapack entry is a symbolic link".into());
            }
            if files.len() >= 50_000 {
                return Err("Too many datapack entries".into());
            }
            let size = entry.size();
            total = total.checked_add(size).ok_or("Datapack size overflow")?;
            if total > 10 * 1024 * 1024 * 1024 {
                return Err("Selected datapack content exceeds 10 GiB".into());
            }
            let source_relative =
                retain.then(|| format!(".msc2-content/datapack-staging/{staging_id}/{index}"));
            let destination: Option<PathBuf> = source_relative
                .as_ref()
                .map(|relative| crate::content_publications::checked_source(server, relative))
                .transpose()?;
            let (sha1, sha512) = archive_entry(entry, size, destination.as_deref(), cancelled)?;
            files.push(File {
                slot_id: slot_id.into(),
                path: relative,
                sha512,
                sha1,
                byte_size: size,
                source_relative,
            });
        }
        files
    };
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let total = files
        .iter()
        .try_fold(0u64, |total, file| total.checked_add(file.byte_size))
        .ok_or("Datapack size overflow")?;
    if total > 10 * 1024 * 1024 * 1024 {
        return Err("Selected datapack content exceeds 10 GiB".into());
    }
    let mut folded = std::collections::BTreeSet::new();
    for file in &files {
        if !folded.insert(file.path.to_lowercase()) {
            return Err("Datapack paths collide across platforms".into());
        }
    }
    Ok(files)
}
