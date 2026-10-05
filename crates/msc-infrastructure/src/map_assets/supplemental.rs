//! Portable data only. A failed candidate never publishes or alters game files.
use super::{saved_terrain::Terrain, *};
use msc_domain::map_assets::{
    CAPTURE_FORMAT, CaptureManifest, CaptureMeshData, CaptureRequest, valid_resource_id,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use zip::ZipArchive;

pub const MAX_CAPTURE: u64 = 64 * 1024 * 1024;
const MAX_DECODED_TEXTURES: u64 = 128 * 1024 * 1024;
const MAX_FILES: usize = 4096;
const MAX_VERTICES: usize = 1_000_000;

pub struct Capture {
    pub manifest: CaptureManifest,
    pub digest: String,
    root: PathBuf,
}
impl Drop for Capture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Capture {
    pub fn read_artifact(&self, path: &str) -> io::Result<Vec<u8>> {
        if path == "capture.json" {
            return serde_json::to_vec(&self.manifest).map_err(|_| error("serialization_failed"));
        }
        let receipt = self
            .manifest
            .files
            .get(path)
            .ok_or_else(|| error("capture_artifact_missing"))?;
        let raw = read(&self.root.join(path), receipt.bytes)?;
        if raw.len() as u64 != receipt.bytes || hash(&raw) != receipt.sha256 {
            return Err(error("capture_artifact_changed"));
        }
        Ok(raw)
    }
    pub fn positions(&self) -> BTreeSet<[i32; 3]> {
        self.manifest.blocks.iter().map(|b| b.position).collect()
    }
    pub fn compatible(&self, request: &CaptureRequest) -> bool {
        // The base geometry generation changes after adoption. All saved input
        // identities remain mandatory for subsequent rebuilds of that scene.
        let mut original = self.manifest.request.clone();
        original.geometry_generation_id = request.geometry_generation_id.clone();
        original == *request
    }
}
/// Cache portable bytes, never an adopted-scene pointer. Every hit goes through
/// the complete validator again against the current saved frame and inputs.
pub struct Cache {
    root: PathBuf,
}
fn cache_gate() -> &'static std::sync::Mutex<()> {
    static GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());
    &GATE
}
impl Cache {
    pub fn new(store: &Path) -> Self {
        Self {
            root: store.join("captures"),
        }
    }
    fn key(request: &CaptureRequest) -> io::Result<String> {
        let mut identity = request.clone();
        identity.geometry_generation_id.clear();
        hash_json(&identity)
    }
    pub fn retain(
        &self,
        path: &Path,
        sha: &str,
        request: &CaptureRequest,
        cancel: &dyn Fn() -> bool,
    ) -> io::Result<()> {
        let _lock = cache_gate()
            .lock()
            .map_err(|_| error("capture_cache_unavailable"))?;
        fs::create_dir_all(&self.root)?;
        safe_path(&self.root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))?;
        }
        let key = Self::key(request)?;
        let destination = self.root.join(format!("{key}.zip"));
        if destination.exists() && file_hash(&destination, MAX_CAPTURE, cancel)? == sha {
            return Ok(());
        }
        let candidate = self
            .root
            .join(format!("candidate-{}", uuid::Uuid::new_v4()));
        let result = (|| {
            let mut input = open(path)?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)?;
            let mut copied = 0;
            let mut bytes = [0u8; 65536];
            loop {
                poll(cancel)?;
                let n = input.read(&mut bytes)?;
                if n == 0 {
                    break;
                }
                copied += n as u64;
                if copied > MAX_CAPTURE {
                    return Err(error("capture_byte_limit"));
                }
                std::io::Write::write_all(&mut output, &bytes[..n])?;
            }
            output.sync_all()?;
            if file_hash(&candidate, MAX_CAPTURE, cancel)? != sha {
                return Err(error("capture_archive_changed"));
            }
            poll(cancel)?;
            // A changed output for the same context is a new frozen frame. Replace
            // cached data only; currently leased scenes own separate immutable copies.
            if destination.exists() {
                fs::remove_file(&destination)?;
            }
            fs::rename(&candidate, &destination)?;
            let mut entries = fs::read_dir(&self.root)?
                .take(33)
                .collect::<Result<Vec<_>, _>>()?;
            if entries.len() > 32 {
                return Err(error("capture_cache_entry_limit"));
            }
            entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
            let mut total = entries
                .iter()
                .map(|e| e.metadata().map(|m| m.len()))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .sum::<u64>();
            let mut count = entries.len();
            for entry in entries {
                safe_path(&entry.path())?;
                if (count > 8 || total > 512 * 1024 * 1024) && entry.path() != destination {
                    total = total.saturating_sub(entry.metadata()?.len());
                    fs::remove_file(entry.path())?;
                    count -= 1;
                }
            }
            Ok(())
        })();
        let _ = fs::remove_file(candidate);
        result
    }
    pub fn load(
        &self,
        request: &CaptureRequest,
        terrain: &Terrain,
        cancel: &dyn Fn() -> bool,
    ) -> io::Result<Option<Capture>> {
        let _lock = cache_gate()
            .lock()
            .map_err(|_| error("capture_cache_unavailable"))?;
        let path = self.root.join(format!("{}.zip", Self::key(request)?));
        if !path.exists() {
            return Ok(None);
        }
        let sha = file_hash(&path, MAX_CAPTURE, cancel)?;
        // Only the base terrain generation is rebased. No binding, frame, scope
        // or resource identity may change on restart/rebuild.
        let mut zip =
            ZipArchive::new(open(&path)?).map_err(|_| error("invalid_capture_archive"))?;
        let mut raw = Vec::new();
        zip.by_name("capture.json")
            .map_err(|_| error("capture_manifest_required"))?
            .take(MAX_JSON + 1)
            .read_to_end(&mut raw)?;
        if raw.len() as u64 > MAX_JSON {
            return Err(error("capture_manifest_limit"));
        }
        let mut manifest: CaptureManifest =
            serde_json::from_slice(&raw).map_err(|_| error("invalid_capture_manifest"))?;
        let original = manifest.request.clone();
        manifest.request.geometry_generation_id = request.geometry_generation_id.clone();
        validate_manifest(&manifest, request, terrain)?;
        let mut capture = import_bundle(&path, &sha, &original, terrain, cancel)?;
        capture.manifest = manifest;
        // Keep the digest of immutable output bytes: the base-generation rebase
        // is transport metadata, not a claim that geometry changed.
        Ok(Some(capture))
    }
}
pub fn is_bundle(path: &Path) -> io::Result<bool> {
    let mut zip = ZipArchive::new(open(path)?).map_err(|_| error("invalid_client_bundle"))?;
    Ok(zip.by_name("capture.json").is_ok())
}
/// Bind auxiliary saved context without transferring unrelated server config.
pub fn context_data_fingerprint(root: &Path, cancel: &dyn Fn() -> bool) -> io::Result<String> {
    let mut folders = BTreeMap::new();
    for name in ["serverconfig", "datapacks", "data"] {
        folders.insert(name, store::fingerprint_configs(&root.join(name), cancel)?);
    }
    hash_json(&folders)
}
pub fn context_area(
    area: msc_domain::map_assets::Area,
) -> io::Result<msc_domain::map_assets::Area> {
    area.validate().map_err(error)?;
    let context = msc_domain::map_assets::Area {
        min: area.min.map(|n| n - 1),
        max: area.max.map(|n| n + 1),
    };
    context
        .validate()
        .map_err(|_| error("capture_context_area_limit"))?;
    Ok(context)
}
fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn member(s: &str) -> bool {
    safe_member(s)
        // Exporters generate artifact identifiers; ASCII avoids Unicode
        // normalization collisions on native macOS filesystems.
        && s.is_ascii()
        && s.len() <= 128
        && (s.starts_with("meshes/") && s.ends_with(".json")
            || s.starts_with("textures/") && s.ends_with(".png"))
}
pub fn validate_manifest(
    m: &CaptureManifest,
    expected: &CaptureRequest,
    terrain: &Terrain,
) -> io::Result<()> {
    if expected.format != CAPTURE_FORMAT
        || m.request != *expected
        || m.observed_snapshot_id != terrain.snapshot_id
        || m.observed_input_fingerprint != expected.input_fingerprint
        || m.request.snapshot_id != terrain.snapshot_id
        || m.request
            .context_data_fingerprint
            .as_ref()
            .is_some_and(|value| !digest(value))
        || !terrain.missing.is_empty()
        || context_area(expected.area)? != expected.context_area
        || !m.saved_frame
        || m.captured_at_unix == 0
        || m.game_tick < 0
        || m.blocks.is_empty()
        || m.blocks.len() > 4096
        || m.files.len() > MAX_FILES
        || m.materials.len() > 1024
        || !m
            .directional_lights
            .iter()
            .flatten()
            .all(|v| v.is_finite() && v.abs() <= 1.0)
    {
        return Err(error("capture_context_mismatch"));
    }
    let samples = terrain
        .blocks
        .iter()
        .map(|b| (b.position, b))
        .collect::<BTreeMap<_, _>>();
    let mut positions = BTreeSet::new();
    let mut meshes = BTreeSet::new();
    let mut refs = BTreeSet::new();
    for b in &m.blocks {
        let sample = samples
            .get(&b.position)
            .ok_or_else(|| error("capture_block_missing"))?;
        if !expected.area.contains(b.position)
            || !positions.insert(b.position)
            || !valid_resource_id(&b.id)
            || !b.id.contains(':')
            || b.id != sample.id
            || b.state != sample.state
            || b.meshes.len() > 128
            || matches!(
                b.id.as_str(),
                "minecraft:air"
                    | "minecraft:cave_air"
                    | "minecraft:void_air"
                    | "minecraft:water"
                    | "minecraft:lava"
            )
        {
            return Err(error("capture_block_mismatch"));
        }
        for mesh in &b.meshes {
            if !mesh.file.starts_with("meshes/")
                || !member(&mesh.file)
                || !meshes.insert(mesh.file.clone())
                || mesh.material >= m.materials.len()
            {
                return Err(error("invalid_capture_mesh_reference"));
            }
            refs.insert(mesh.file.clone());
        }
    }
    for material in &m.materials {
        if !member(&material.texture)
            || !material.texture.starts_with("textures/")
            || !["opaque", "cutout", "translucent", "additive"].contains(&material.mode.as_str())
            || !material.alpha_threshold.is_finite()
            || !(0.0..=1.0).contains(&material.alpha_threshold)
            || (matches!(material.mode.as_str(), "translucent" | "additive")
                && material.depth_write)
        {
            return Err(error("unsupported_capture_material"));
        }
        refs.insert(material.texture.clone());
    }
    let mut names = BTreeSet::new();
    let mut total = 0;
    for (name, file) in &m.files {
        if !member(name)
            || !names.insert(name.to_ascii_lowercase())
            || !digest(&file.sha256)
            || file.bytes == 0
            || file.bytes > 16 * 1024 * 1024
        {
            return Err(error("unsafe_capture_file"));
        }
        total += file.bytes;
    }
    if total > MAX_CAPTURE || refs != m.files.keys().cloned().collect() {
        return Err(error("capture_file_limit_or_unused_payload"));
    }
    Ok(())
}
fn validate_mesh(
    raw: &[u8],
    position: [i32; 3],
    context: msc_domain::map_assets::Area,
) -> io::Result<(usize, usize)> {
    let m: CaptureMeshData =
        serde_json::from_slice(raw).map_err(|_| error("invalid_capture_mesh"))?;
    let n = m.positions.len() / 3;
    if n == 0
        || n > MAX_VERTICES
        || m.positions.len() != n * 3
        || m.normals.len() != n * 3
        || m.uv.len() != n * 2
        || m.colors.len() != n * 4
        || m.indices.is_empty()
        || m.indices.len() > MAX_VERTICES * 6
        || !m.indices.len().is_multiple_of(3)
        || m.indices.iter().any(|i| *i as usize >= n)
        || !m
            .positions
            .iter()
            .chain(&m.normals)
            .chain(&m.uv)
            .chain(&m.colors)
            .all(|v| v.is_finite())
        || m.colors.iter().any(|v| !(0.0..=1.0).contains(v))
        || m.uv.iter().any(|v| !(0.0..=1.0).contains(v))
        || m.normals.iter().any(|v| v.abs() > 1.001)
        || m.positions.chunks_exact(3).any(|p| {
            (0..3).any(|a| {
                (p[a] - position[a] as f32).abs() > 16.0
                    || f64::from(p[a]) < f64::from(context.min[a])
                    || f64::from(p[a]) > f64::from(context.max[a]) + 1.0
            })
        })
    {
        return Err(error("invalid_capture_geometry"));
    }
    Ok((n, m.indices.len()))
}
fn validate_texture(raw: &[u8]) -> io::Result<u64> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(raw));
    decoder.set_limits(png::Limits {
        bytes: 64 * 1024 * 1024,
    });
    let mut reader = decoder
        .read_info()
        .map_err(|_| error("invalid_capture_texture"))?;
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || info.width > 4096
        || info.height > 4096
        || info.animation_control.is_some()
        || info.bit_depth != png::BitDepth::Eight
        || !matches!(info.color_type, png::ColorType::Rgb | png::ColorType::Rgba)
    {
        return Err(error("invalid_capture_texture"));
    }
    let decoded = u64::from(info.width) * u64::from(info.height) * 4;
    let size = reader
        .output_buffer_size()
        .filter(|s| *s <= 64 * 1024 * 1024)
        .ok_or_else(|| error("capture_texture_limit"))?;
    reader
        .next_frame(&mut vec![0; size])
        .map_err(|_| error("invalid_capture_texture"))?;
    Ok(decoded)
}
pub fn import_bundle(
    path: &Path,
    sha: &str,
    expected: &CaptureRequest,
    terrain: &Terrain,
    cancel: &dyn Fn() -> bool,
) -> io::Result<Capture> {
    if !digest(sha) || file_hash(path, MAX_CAPTURE, cancel)? != sha {
        return Err(error("capture_bundle_checksum"));
    }
    let mut zip = ZipArchive::new(open(path)?).map_err(|_| error("invalid_capture_bundle"))?;
    if zip.len() > MAX_FILES + 1 {
        return Err(error("capture_file_limit"));
    }
    let mut members = BTreeSet::new();
    for i in 0..zip.len() {
        poll(cancel)?;
        let entry = zip
            .by_index(i)
            .map_err(|_| error("invalid_capture_bundle"))?;
        let name = entry.name();
        if (name != "capture.json" && !member(name))
            || !members.insert(name.to_ascii_lowercase())
            || entry.is_dir()
            || entry
                .unix_mode()
                .is_some_and(|m| ![0, 0o100000].contains(&(m & 0o170000)))
            || entry.size() > 16 * 1024 * 1024
        {
            return Err(error("unsafe_capture_archive"));
        }
    }
    let load = |zip: &mut ZipArchive<std::fs::File>, name: &str, max: u64| -> io::Result<Vec<u8>> {
        poll(cancel)?;
        let mut entry = zip
            .by_name(name)
            .map_err(|_| error("capture_file_missing"))?;
        if entry.size() > max {
            return Err(error("capture_file_limit"));
        }
        let mut raw = Vec::new();
        entry.by_ref().take(max + 1).read_to_end(&mut raw)?;
        if raw.len() as u64 != entry.size() || raw.len() as u64 > max {
            return Err(error("capture_file_limit"));
        }
        Ok(raw)
    };
    let raw = load(&mut zip, "capture.json", MAX_JSON)?;
    let manifest: CaptureManifest =
        serde_json::from_slice(&raw).map_err(|_| error("invalid_capture_manifest"))?;
    validate_manifest(&manifest, expected, terrain)?;
    if members
        != manifest
            .files
            .keys()
            .map(|s| s.to_ascii_lowercase())
            .chain(std::iter::once("capture.json".into()))
            .collect()
    {
        return Err(error("capture_archive_manifest_mismatch"));
    }
    let root = fs::canonicalize(std::env::temp_dir())?
        .join(format!("msc-contextual-map-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&root)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(error) = fs::set_permissions(&root, fs::Permissions::from_mode(0o700)) {
            let _ = fs::remove_dir(&root);
            return Err(error);
        }
    }
    let capture = Capture {
        digest: hash_json(&manifest)?,
        manifest,
        root,
    };
    let mesh_positions = capture
        .manifest
        .blocks
        .iter()
        .flat_map(|b| b.meshes.iter().map(move |m| (&m.file, b.position)))
        .collect::<BTreeMap<_, _>>();
    let mut vertices = 0;
    let mut indices = 0;
    let mut decoded_textures = 0;
    for (name, file) in &capture.manifest.files {
        let raw = load(&mut zip, name, file.bytes)?;
        if hash(&raw) != file.sha256 {
            return Err(error("capture_file_checksum"));
        }
        if let Some(position) = mesh_positions.get(name) {
            let (vertex_count, index_count) =
                validate_mesh(&raw, *position, expected.context_area)?;
            vertices += vertex_count;
            indices += index_count;
            if vertices > MAX_VERTICES || indices > MAX_VERTICES * 6 {
                return Err(error("capture_vertex_limit"));
            }
        } else {
            decoded_textures += validate_texture(&raw)?;
            if decoded_textures > MAX_DECODED_TEXTURES {
                return Err(error("capture_decoded_texture_limit"));
            }
        }
        let target = capture.root.join(name);
        fs::create_dir_all(
            target
                .parent()
                .ok_or_else(|| error("unsafe_capture_file"))?,
        )?;
        fs::write(target, raw)?;
    }
    poll(cancel)?;
    if file_hash(path, MAX_CAPTURE, cancel)? != sha {
        return Err(error("capture_bundle_changed"));
    }
    Ok(capture)
}

/// Repack only touched private sections. Air removes the old shape and exposes
/// adjoining faces; state-wide substitution would erase uncaptured instances.
pub fn suppress_section(
    section: &mut std::collections::HashMap<String, fastnbt::Value>,
    cx: i32,
    cz: i32,
    captured: &std::collections::BTreeSet<[i32; 3]>,
) -> io::Result<()> {
    use super::saved_terrain::palette_at;
    use fastnbt::Value;
    use std::collections::HashMap;
    let sy = match section.get("Y") {
        Some(Value::Byte(v)) => i32::from(*v),
        Some(Value::Int(v)) => *v,
        _ => return Ok(()),
    };
    let positions = captured
        .iter()
        .filter(|p| {
            p[0].div_euclid(16) == cx && p[1].div_euclid(16) == sy && p[2].div_euclid(16) == cz
        })
        .collect::<Vec<_>>();
    if positions.is_empty() {
        return Ok(());
    }
    let mut old = Vec::with_capacity(4096);
    for i in 0..4096 {
        old.push(
            palette_at(
                section,
                cx * 16 + i % 16,
                sy * 16 + i / 256,
                cz * 16 + i / 16 % 16,
            )?
            .cloned()
            .ok_or_else(|| error("capture_section_missing"))?,
        );
    }
    let air = Value::Compound(HashMap::from([(
        "Name".into(),
        Value::String("minecraft:air".into()),
    )]));
    for p in positions {
        let i =
            (p[1].rem_euclid(16) * 256 + p[2].rem_euclid(16) * 16 + p[0].rem_euclid(16)) as usize;
        // Captured block meshes do not replace the fluid pass. Retain the
        // vanilla water source inside a waterlogged block in this private copy.
        old[i] = if matches!(&old[i], Value::Compound(fields)
            if matches!(fields.get("Properties"), Some(Value::Compound(props))
                if props.get("waterlogged") == Some(&Value::String("true".into()))))
        {
            Value::Compound(HashMap::from([
                ("Name".into(), Value::String("minecraft:water".into())),
                (
                    "Properties".into(),
                    Value::Compound(HashMap::from([("level".into(), Value::String("0".into()))])),
                ),
            ]))
        } else {
            air.clone()
        };
    }
    let mut palette = Vec::new();
    let mut keys = BTreeMap::new();
    let mut indices = Vec::with_capacity(4096);
    for value in old {
        let key = serde_json::to_string(&value).map_err(|_| error("invalid_palette_state"))?;
        let index = *keys.entry(key).or_insert_with(|| {
            palette.push(value);
            palette.len() - 1
        });
        indices.push(index);
    }
    let mut states = HashMap::new();
    if palette.len() > 1 {
        let bits = (usize::BITS - (palette.len() - 1).leading_zeros()).max(4) as usize;
        let per = 64 / bits;
        let mut data = vec![0i64; 4096usize.div_ceil(per)];
        for (i, value) in indices.into_iter().enumerate() {
            data[i / per] |= ((value as u64) << ((i % per) * bits)) as i64;
        }
        states.insert(
            "data".into(),
            Value::LongArray(fastnbt::LongArray::new(data)),
        );
    }
    states.insert("palette".into(), Value::List(palette));
    section.insert("block_states".into(), Value::Compound(states));
    Ok(())
}
