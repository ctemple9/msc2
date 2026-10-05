//! Essential: forged context must not replace a map, malformed portable data
//! must not reach the GPU, and replacing one state instance must preserve its
//! neighbors. Controlled local bytes only; expected runtime below two seconds.
use fastnbt::Value;
use msc_domain::map_assets::*;
use msc_infrastructure::map_assets::{
    hash,
    saved_terrain::{BlockSample, Terrain, palette_at},
    supplemental,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::Write;
use std::path::PathBuf;

struct Temporary(PathBuf);
impl Temporary {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("msc-map-capture-check-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> (CaptureManifest, Terrain, BTreeMap<String, Vec<u8>>) {
    let area = Area {
        min: [2, 64, 2],
        max: [7, 66, 4],
    };
    let request = CaptureRequest {
        format: CAPTURE_FORMAT.into(),
        binding: Binding {
            agent_host_id: "host".into(),
            server_id: "server".into(),
            slot_id: "slot".into(),
            world_incarnation: "world".into(),
            revision: hash(b"revision"),
        },
        geometry_generation_id: hash(b"geometry"),
        resource_generation_id: hash(b"resources"),
        input_fingerprint: hash(b"inputs"),
        snapshot_id: hash(b"snapshot"),
        dimension: "minecraft:overworld".into(),
        area,
        context_area: supplemental::context_area(area).unwrap(),
        minecraft_version: "1.21.1".into(),
        loader: "neoforge".into(),
        loader_version: "21.1.251".into(),
    };
    let positions = [[3, 65, 3], [6, 65, 3]];
    let blocks = positions
        .iter()
        .enumerate()
        .map(|(i, position)| CapturedBlock {
            position: *position,
            id: "fixture:pedestal".into(),
            state: BTreeMap::new(),
            meshes: vec![CaptureMesh {
                file: format!("meshes/{i}.json"),
                material: 0,
            }],
        })
        .collect::<Vec<_>>();
    let mut png = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&[255, 255, 255, 255])
            .unwrap();
    }
    let mut data = BTreeMap::from([("textures/atlas.png".into(), png)]);
    for (i, [x, y, z]) in positions.iter().enumerate() {
        data.insert(
            format!("meshes/{i}.json"),
            serde_json::to_vec(&CaptureMeshData {
                positions: vec![
                    *x as f32,
                    *y as f32,
                    *z as f32,
                    *x as f32 + 0.5,
                    *y as f32,
                    *z as f32,
                    *x as f32,
                    *y as f32 + 0.5,
                    *z as f32,
                ],
                normals: vec![0., 0., 1., 0., 0., 1., 0., 0., 1.],
                uv: vec![0., 0., 1., 0., 0., 1.],
                colors: if i == 0 { vec![1.; 12] } else { vec![0.5; 12] },
                indices: vec![0, 1, 2],
            })
            .unwrap(),
        );
    }
    let files = data
        .iter()
        .map(|(name, bytes)| {
            (
                name.clone(),
                CaptureFile {
                    sha256: hash(bytes),
                    bytes: bytes.len() as u64,
                },
            )
        })
        .collect();
    let terrain = Terrain {
        snapshot_id: request.snapshot_id.clone(),
        chunks: 1,
        missing: vec![],
        blocks: blocks
            .iter()
            .map(|b| BlockSample {
                position: b.position,
                id: b.id.clone(),
                state: b.state.clone(),
                entity: true,
            })
            .collect(),
    };
    (
        CaptureManifest {
            observed_snapshot_id: request.snapshot_id.clone(),
            observed_input_fingerprint: request.input_fingerprint.clone(),
            request,
            captured_at_unix: 1,
            game_tick: 1,
            saved_frame: true,
            directional_lights: [[0., 1., 0.], [0., 1., 0.]],
            materials: vec![CaptureMaterial {
                texture: "textures/atlas.png".into(),
                mode: "cutout".into(),
                alpha_threshold: 0.1,
                cull: false,
                depth_write: true,
                directional_lighting: false,
            }],
            blocks,
            files,
        },
        terrain,
        data,
    )
}
fn zip(
    path: &std::path::Path,
    manifest: &CaptureManifest,
    data: &BTreeMap<String, Vec<u8>>,
) -> String {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    let opts =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("capture.json", opts).unwrap();
    zip.write_all(&serde_json::to_vec(manifest).unwrap())
        .unwrap();
    for (name, raw) in data {
        zip.start_file(name, opts).unwrap();
        zip.write_all(raw).unwrap();
    }
    zip.finish().unwrap();
    hash(&std::fs::read(path).unwrap())
}

#[test]
fn wrong_binding_snapshot_generation_state_and_duplicate_positions_are_refused() {
    let (m, terrain, _) = fixture();
    supplemental::validate_manifest(&m, &m.request, &terrain).unwrap();
    for mutation in 0..8 {
        let mut bad = m.clone();
        match mutation {
            0 => bad.request.binding.agent_host_id = "other-host".into(),
            1 => bad.request.snapshot_id = hash(b"other-snapshot"),
            2 => bad.request.resource_generation_id = hash(b"other-resources"),
            3 => {
                bad.blocks[0].state.insert("facing".into(), "north".into());
            }
            4 => bad.blocks[1].position = bad.blocks[0].position,
            5 => bad.request.input_fingerprint = hash(b"other-client"),
            6 => bad.observed_snapshot_id = hash(b"same-state-different-entity-contents"),
            _ => bad.observed_input_fingerprint = hash(b"wrong-client-resource-order"),
        }
        assert!(supplemental::validate_manifest(&bad, &m.request, &terrain).is_err());
    }
    let mut absent = terrain;
    absent.missing.push([0, 64, 0]);
    assert!(supplemental::validate_manifest(&m, &m.request, &absent).is_err());
}

#[test]
fn malformed_mesh_texture_archive_and_cancel_leave_the_valid_capture_readable() {
    let temp = Temporary::new();
    let (m, terrain, data) = fixture();
    let path = temp.0.join("capture.zip");
    let sha = zip(&path, &m, &data);
    let retained =
        supplemental::import_bundle(&path, &sha, &m.request, &terrain, &|| false).unwrap();
    assert_eq!(retained.positions().len(), 2);
    let good = retained.read_artifact("meshes/0.json").unwrap();
    assert_ne!(good, retained.read_artifact("meshes/1.json").unwrap());
    for mutation in 0..6 {
        let mut bad = m.clone();
        let mut bytes = data.clone();
        match mutation {
            0 => {
                let mut mesh: CaptureMeshData =
                    serde_json::from_slice(&bytes["meshes/0.json"]).unwrap();
                mesh.indices[2] = 1000;
                bytes.insert("meshes/0.json".into(), serde_json::to_vec(&mesh).unwrap());
            }
            1 => {
                let mut mesh: CaptureMeshData =
                    serde_json::from_slice(&bytes["meshes/0.json"]).unwrap();
                mesh.positions[0] = 1000.;
                bytes.insert("meshes/0.json".into(), serde_json::to_vec(&mesh).unwrap());
            }
            2 => {
                bytes.insert("textures/atlas.png".into(), b"not a png".to_vec());
            }
            3 => {
                bytes.insert(
                    "textures/ATLAS.png".into(),
                    bytes["textures/atlas.png"].clone(),
                );
            }
            4 => {
                bytes.insert("../escape.json".into(), b"{}".to_vec());
            }
            _ => {
                bad.materials[0].mode = "arbitrary-shader".into();
            }
        }
        bad.files = bytes
            .iter()
            .map(|(name, raw)| {
                (
                    name.clone(),
                    CaptureFile {
                        sha256: hash(raw),
                        bytes: raw.len() as u64,
                    },
                )
            })
            .collect();
        let sha = zip(&path, &bad, &bytes);
        assert!(supplemental::import_bundle(&path, &sha, &m.request, &terrain, &|| false).is_err());
        assert_eq!(retained.read_artifact("meshes/0.json").unwrap(), good);
    }
    let sha = zip(&path, &m, &data);
    assert!(supplemental::import_bundle(&path, &sha, &m.request, &terrain, &|| true).is_err());
    assert_eq!(retained.read_artifact("meshes/0.json").unwrap(), good);
    let raw = serde_json::to_string(&m).unwrap().replace("\"files\":{", "\"files\":{\"duplicate\":{\"sha256\":\"x\",\"bytes\":1},\"duplicate\":{\"sha256\":\"y\",\"bytes\":1},");
    assert!(serde_json::from_str::<CaptureManifest>(&raw).is_err());
}

#[test]
fn position_replacement_repacks_palette_width_without_erasing_other_instances() {
    let palette = (0..16)
        .map(|i| {
            Value::Compound(HashMap::from([(
                "Name".into(),
                Value::String(format!("fixture:block_{i}")),
            )]))
        })
        .collect();
    let mut data = vec![0i64; 256];
    for i in 0..4096 {
        data[i / 16] |= ((i % 16) as i64) << ((i % 16) * 4);
    }
    let mut section = HashMap::from([
        ("Y".into(), Value::Byte(4)),
        (
            "block_states".into(),
            Value::Compound(HashMap::from([
                ("palette".into(), Value::List(palette)),
                (
                    "data".into(),
                    Value::LongArray(fastnbt::LongArray::new(data)),
                ),
            ])),
        ),
    ]);
    let original = section.clone();
    let captured = BTreeSet::from([[-16, 64, -15], [-1, 79, -1]]);
    supplemental::suppress_section(&mut section, -1, -1, &captured).unwrap();
    for i in 0..4096 {
        let p = [-16 + i % 16, 64 + i / 256, -16 + i / 16 % 16];
        let actual = palette_at(&section, p[0], p[1], p[2]).unwrap().unwrap();
        if captured.contains(&p) {
            assert_eq!(
                actual,
                &Value::Compound(HashMap::from([(
                    "Name".into(),
                    Value::String("minecraft:air".into())
                )]))
            );
        } else {
            assert_eq!(
                Some(actual),
                palette_at(&original, p[0], p[1], p[2]).unwrap()
            );
        }
    }
    // The actual source section remains unchanged, including its old packed data.
    assert_ne!(section, original);
    let wet = Value::Compound(HashMap::from([
        ("Name".into(), Value::String("fixture:wet_block".into())),
        (
            "Properties".into(),
            Value::Compound(HashMap::from([(
                "waterlogged".into(),
                Value::String("true".into()),
            )])),
        ),
    ]));
    let mut section = HashMap::from([
        ("Y".into(), Value::Byte(4)),
        (
            "block_states".into(),
            Value::Compound(HashMap::from([(
                "palette".into(),
                Value::List(vec![wet.clone()]),
            )])),
        ),
    ]);
    supplemental::suppress_section(&mut section, -1, -1, &captured).unwrap();
    let water = Value::Compound(HashMap::from([
        ("Name".into(), Value::String("minecraft:water".into())),
        (
            "Properties".into(),
            Value::Compound(HashMap::from([("level".into(), Value::String("0".into()))])),
        ),
    ]));
    assert_eq!(palette_at(&section, -16, 64, -15).unwrap(), Some(&water));
    assert_eq!(palette_at(&section, -15, 64, -15).unwrap(), Some(&wet));
}
