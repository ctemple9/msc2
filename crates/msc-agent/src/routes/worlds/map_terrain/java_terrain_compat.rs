//! Private, read-only compatibility copy for Java's 26.x dimension layout.
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Condvar, Mutex};
use std::time::Instant;

use fastnbt::Value;
use flate2::{Compression, read::GzDecoder, read::ZlibDecoder, write::ZlibEncoder};

const SECTOR: usize = 4096;
const MAX_REGION_BYTES: u64 = 256 * 1024 * 1024;
const MAX_CHUNK_BYTES: usize = 32 * 1024 * 1024;

pub(super) fn staging_estimate(world: &Path, dimension: &str) -> io::Result<u64> {
    let relative = match dimension {
        "minecraft:overworld" => "dimensions/minecraft/overworld/region",
        "minecraft:the_nether" => "dimensions/minecraft/the_nether/region",
        "minecraft:the_end" => "dimensions/minecraft/the_end/region",
        _ => return Ok(0),
    };
    let mut source = world.to_path_buf();
    for segment in Path::new(relative).components() {
        source.push(segment);
        match fs::symlink_metadata(&source) {
            Ok(meta) if meta.file_type().is_dir() => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(0),
            Err(error) => return Err(error),
            _ => return Err(invalid("Java dimension path is not a regular directory")),
        }
    }
    // Reserve one working copy. Recompression growth is checked using the
    // actual normalized size before every write, rather than doubling all
    // regions and refusing worlds that already fit on disk.
    crate::map_staging::estimate_tree(&source)
}

pub(super) fn prepare(
    world: &Path,
    dimension: &str,
    cache: &Path,
    progress: &(impl Fn(usize, usize) + Sync),
    server_running: &(impl Fn() -> bool + Sync),
) -> io::Result<PathBuf> {
    let (modern, legacy) = match dimension {
        "minecraft:overworld" => ("dimensions/minecraft/overworld/region", "region"),
        "minecraft:the_nether" => ("dimensions/minecraft/the_nether/region", "DIM-1/region"),
        "minecraft:the_end" => ("dimensions/minecraft/the_end/region", "DIM1/region"),
        _ => return Ok(world.to_path_buf()),
    };
    let mut source = world.to_path_buf();
    for segment in Path::new(modern).components() {
        source.push(segment);
        match fs::symlink_metadata(&source) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(world.to_path_buf());
            }
            Ok(_) => return Err(invalid("Java dimension path is not a regular directory")),
            Err(error) => return Err(error),
        }
    }
    let level = world.join("level.dat");
    if !fs::symlink_metadata(&level)?.file_type().is_file() {
        return Err(invalid("Java level.dat is not a regular file"));
    }
    let estimate = crate::map_staging::estimate_tree(&source)?;
    crate::map_staging::ensure_space(cache, estimate.saturating_add(64 * 1024 * 1024))?;
    let staged = cache.join("world");
    let target = staged.join(legacy);
    fs::create_dir_all(&target)?;
    fs::copy(level, staged.join("level.dat"))?;
    let entries = fs::read_dir(&source)?.collect::<io::Result<Vec<_>>>()?;
    let mut regions = Vec::new();
    for entry in entries {
        if entry.file_name().to_string_lossy().ends_with(".mca") && entry.file_type()?.is_file() {
            regions.push(entry);
        }
    }
    let total = regions.len();
    let conversion_started = Instant::now();
    let source_bytes = regions
        .iter()
        .filter_map(|entry| entry.metadata().ok().map(|metadata| metadata.len()))
        .sum::<u64>();
    progress(0, total);
    let available_cores = std::thread::available_parallelism().map_or(1, usize::from);
    let memory_budget_mib =
        msc_infrastructure::conversion_resources::available_heap_mib().unwrap_or(1024);
    let available_disk = fs2::available_space(cache).unwrap_or(0);
    let disk_worker_budget = if available_disk == 0 {
        1
    } else {
        (available_disk.saturating_sub(msc_infrastructure::conversion_resources::DISK_RESERVE)
            / (512 * 1024 * 1024))
            .max(1) as usize
    };
    let max_workers = available_cores
        .min((memory_budget_mib / 1024).max(1) as usize)
        .min(disk_worker_budget)
        .min(4);
    let next_region = AtomicUsize::new(0);
    let completed = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let output_bytes = AtomicU64::new(0);
    let first_error = Mutex::new(None);
    let active_workers = Mutex::new(0usize);
    let worker_changed = Condvar::new();
    let server_throttle_logged = AtomicBool::new(false);
    let target = target.as_path();

    eprintln!(
        "Java map: converting {total} regions with pool capacity {max_workers} ({available_cores} logical cores; {memory_budget_mib} MiB memory budget; {available_disk} bytes available on staging volume; concurrency limited to one while server runs)"
    );
    std::thread::scope(|scope| {
        for _ in 0..max_workers {
            scope.spawn(|| loop {
                if failed.load(Ordering::Acquire) {
                    break;
                }
                let index = next_region.fetch_add(1, Ordering::Relaxed);
                let Some(entry) = regions.get(index) else {
                    break;
                };
                let mut active = active_workers.lock().expect("worker gate poisoned");
                loop {
                    if failed.load(Ordering::Acquire) {
                        return;
                    }
                    let running = server_running();
                    if running && max_workers > 1 {
                        if !server_throttle_logged.swap(true, Ordering::AcqRel) {
                            eprintln!(
                                "Java map: server started during conversion; limiting new region work to one worker"
                            );
                        }
                    } else if !running {
                        server_throttle_logged.store(false, Ordering::Release);
                    }
                    let limit = if running { 1 } else { max_workers };
                    if *active < limit {
                        *active += 1;
                        break;
                    }
                    let (next, _) = worker_changed
                        .wait_timeout(active, std::time::Duration::from_millis(250))
                        .expect("worker gate poisoned");
                    active = next;
                }
                drop(active);

                let result = convert_region(entry, target, cache, index);
                let mut active = active_workers.lock().expect("worker gate poisoned");
                *active -= 1;
                worker_changed.notify_all();
                drop(active);
                match result {
                    Ok(bytes) => {
                        output_bytes.fetch_add(bytes, Ordering::Relaxed);
                        let done = completed.fetch_add(1, Ordering::AcqRel) + 1;
                        progress(done, total);
                    }
                    Err(error) => {
                        failed.store(true, Ordering::Release);
                        if let Ok(mut first) = first_error.lock()
                            && first.is_none()
                        {
                            *first = Some(error);
                        }
                        worker_changed.notify_all();
                        break;
                    }
                }
            });
        }
    });
    if let Some(error) = first_error
        .into_inner()
        .map_err(|_| io::Error::other("Java region conversion error state was poisoned"))?
    {
        return Err(error);
    }
    eprintln!(
        "Java map: converted {total} compatible region files ({source_bytes} source bytes; {} staged bytes) in {:.1}s",
        output_bytes.load(Ordering::Relaxed),
        conversion_started.elapsed().as_secs_f64()
    );
    Ok(staged)
}

fn convert_region(
    entry: &fs::DirEntry,
    target: &Path,
    cache: &Path,
    index: usize,
) -> io::Result<u64> {
    let name = entry.file_name();
    if !name.to_string_lossy().ends_with(".mca") || !entry.file_type()?.is_file() {
        return Ok(0);
    }
    if entry.metadata()?.len() > MAX_REGION_BYTES {
        return Err(io::Error::other(
            "Java region exceeds the terrain renderer limit",
        ));
    }
    let raw = fs::read(entry.path())?;
    let normalized = normalize_region(&raw)?;
    crate::map_staging::ensure_space(cache, normalized.len() as u64 + 64 * 1024 * 1024)?;
    let output = target.join(name);
    let temporary = target.join(format!(".region-{}-{index}.tmp", std::process::id()));
    let result = (|| {
        fs::write(&temporary, &normalized)?;
        fs::rename(&temporary, output)?;
        Ok(normalized.len() as u64)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn normalize_region(region: &[u8]) -> io::Result<Vec<u8>> {
    if region.len() < 2 * SECTOR {
        return Err(invalid("Java region header is incomplete"));
    }
    let mut output = vec![0u8; 2 * SECTOR];
    output[SECTOR..2 * SECTOR].copy_from_slice(&region[SECTOR..2 * SECTOR]);
    for index in 0..1024 {
        let entry = index * 4;
        let offset = (usize::from(region[entry]) << 16)
            | (usize::from(region[entry + 1]) << 8)
            | usize::from(region[entry + 2]);
        let sectors = usize::from(region[entry + 3]);
        if offset == 0 || sectors == 0 {
            continue;
        }
        let start = offset
            .checked_mul(SECTOR)
            .ok_or_else(|| invalid("Region offset overflow"))?;
        start
            .checked_add(sectors * SECTOR)
            .ok_or_else(|| invalid("Region length overflow"))?;
        let header = region
            .get(start..start + 5)
            .ok_or_else(|| invalid("Region chunk header is incomplete"))?;
        let length = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
        if length < 1 || length + 4 > sectors * SECTOR || start + 4 + length > region.len() {
            return Err(invalid("Region chunk length is invalid"));
        }
        let compression = header[4];
        if compression & 0x80 != 0 {
            return Err(invalid("External Java chunk data is unsupported"));
        }
        let payload = &region[start + 5..start + 4 + length];
        let mut decoded = Vec::new();
        match compression {
            1 => GzDecoder::new(payload)
                .take((MAX_CHUNK_BYTES + 1) as u64)
                .read_to_end(&mut decoded)?,
            2 => ZlibDecoder::new(payload)
                .take((MAX_CHUNK_BYTES + 1) as u64)
                .read_to_end(&mut decoded)?,
            3 => {
                decoded.extend_from_slice(payload);
                decoded.len()
            }
            _ => return Err(invalid("Unsupported Java chunk compression")),
        };
        if decoded.len() > MAX_CHUNK_BYTES {
            return Err(invalid("Java chunk exceeds the terrain renderer limit"));
        }
        let mut chunk: Value =
            fastnbt::from_bytes(&decoded).map_err(|_| invalid("Java chunk NBT is invalid"))?;
        let (compressed, output_compression) = if normalize_chunk(&mut chunk) {
            decoded = fastnbt::to_bytes(&chunk)
                .map_err(|_| invalid("Could not encode Java chunk NBT"))?;
            let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&decoded)?;
            (encoder.finish()?, 2)
        } else {
            (payload.to_vec(), compression)
        };
        let record_len = compressed.len() + 5;
        let needed = record_len.div_ceil(SECTOR);
        if needed > 255 {
            return Err(invalid("Normalized Java chunk exceeds region sector limit"));
        }
        let new_offset = output.len() / SECTOR;
        if new_offset > 0x00ff_ffff {
            return Err(invalid("Normalized Java region exceeds offset limit"));
        }
        output[entry] = (new_offset >> 16) as u8;
        output[entry + 1] = (new_offset >> 8) as u8;
        output[entry + 2] = new_offset as u8;
        output[entry + 3] = needed as u8;
        output.extend_from_slice(&((compressed.len() + 1) as u32).to_be_bytes());
        output.push(output_compression);
        output.extend_from_slice(&compressed);
        output.resize(output.len().div_ceil(SECTOR) * SECTOR, 0);
    }
    Ok(output)
}

fn normalize_chunk(chunk: &mut Value) -> bool {
    let Value::Compound(root) = chunk else {
        return false;
    };
    let Some(Value::List(sections)) = root.get_mut("sections") else {
        return false;
    };
    let mut changed = false;
    for section in sections {
        let Value::Compound(section) = section else {
            continue;
        };
        let Some(Value::Compound(states)) = section.get_mut("block_states") else {
            continue;
        };
        let Some(Value::List(palette)) = states.get_mut("palette") else {
            continue;
        };
        for entry in palette {
            let (name, properties) = match entry {
                Value::String(name) => (name.clone(), None),
                Value::Compound(fields) if !fields.contains_key("Name") => {
                    let name = fields.get("id").or_else(|| fields.get(""));
                    let Some(Value::String(name)) = name else {
                        continue;
                    };
                    (name.clone(), fields.get("properties").cloned())
                }
                _ => continue,
            };
            let mut legacy = HashMap::from([("Name".to_string(), Value::String(name.clone()))]);
            if let Some(properties) = properties.or_else(|| default_properties(&name)) {
                legacy.insert("Properties".to_string(), properties);
            }
            *entry = Value::Compound(legacy);
            changed = true;
        }
    }
    changed
}

fn default_properties(name: &str) -> Option<Value> {
    let block = name.strip_prefix("minecraft:")?;
    let (key, value) = if ["_log", "_wood", "_stem", "_hyphae"]
        .iter()
        .any(|suffix| block.ends_with(suffix))
        || block == "deepslate"
    {
        ("axis", "y")
    } else if block == "grass_block" {
        ("snowy", "false")
    } else if matches!(block, "tall_grass" | "tall_seagrass") {
        ("half", "lower")
    } else if block.ends_with("_amethyst_bud") {
        ("facing", "up")
    } else {
        return None;
    };
    Some(Value::Compound(HashMap::from([(
        key.to_string(),
        Value::String(value.to_string()),
    )])))
}
