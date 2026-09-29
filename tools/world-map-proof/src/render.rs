use bedrock_block_model::{
    BlockFace, BlockStateQuery, BlockStateValue, ObjTextureResolver, model_shape_for_block_state,
};
use bedrock_world::{BedrockWorld, BlockState, ChunkPos, Dimension, NbtTag};
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const SIDE: usize = 64;

#[derive(Clone, Default)]
struct Column {
    solid: Option<(i32, BlockState)>,
    water: Option<i32>,
    plant: Option<(i32, BlockState)>,
}

#[derive(Default)]
struct Mesh {
    positions: Vec<f32>,
    uv: Vec<f32>,
    layer: Vec<f32>,
    colors: Vec<u8>,
    normals: Vec<u8>,
    biome: Vec<f32>,
    indices: Vec<u32>,
}

impl Mesh {
    fn quad(&mut self, corners: [[f32; 3]; 4], normal: [i32; 3], uv: [[f32; 2]; 4], layer: u16) {
        let start = (self.positions.len() / 3) as u32;
        for i in 0..4 {
            self.positions.extend(corners[i]);
            self.uv.extend(uv[i]);
            self.layer.push(f32::from(layer));
            self.colors.extend([255, 255, 255, 255]);
            self.normals.extend([
                normal[0] as i8 as u8,
                normal[1] as i8 as u8,
                normal[2] as i8 as u8,
                255,
            ]);
            self.biome.push(0.0);
        }
        self.indices
            .extend([start, start + 1, start + 2, start, start + 2, start + 3]);
    }

    fn write(&self, out: &mut Vec<u8>) {
        out.extend((self.positions.len() as u32 / 3).to_le_bytes());
        out.extend((self.indices.len() as u32).to_le_bytes());
        for value in &self.positions {
            out.extend(value.to_le_bytes());
        }
        for value in &self.uv {
            out.extend(value.to_le_bytes());
        }
        for value in &self.layer {
            out.extend(value.to_le_bytes());
        }
        out.extend(&self.colors);
        out.extend(&self.normals);
        for value in &self.biome {
            out.extend(value.to_le_bytes());
        }
        for value in &self.indices {
            out.extend(value.to_le_bytes());
        }
    }
}

struct Textures {
    resolver: ObjTextureResolver,
    layers: BTreeMap<PathBuf, u16>,
    pixels: Vec<u8>,
    fallback: usize,
}

impl Textures {
    fn new(pack: &Path) -> Self {
        let mut pixels = Vec::with_capacity(16 * 16 * 4);
        for z in 0..16 {
            for x in 0..16 {
                let bright = (x / 4 + z / 4) % 2 == 0;
                pixels.extend(if bright {
                    [230, 60, 190, 255]
                } else {
                    [35, 25, 35, 255]
                });
            }
        }
        Self {
            resolver: ObjTextureResolver::with_pack_roots([pack], "textures"),
            layers: BTreeMap::new(),
            pixels,
            fallback: 0,
        }
    }

    fn layer(&mut self, block: &str, normal: [i32; 3]) -> u16 {
        let Some(texture) = self.resolver.texture_for(block, normal) else {
            self.fallback += 1;
            return 0;
        };
        if let Some(index) = self.layers.get(&texture.source_path) {
            return *index;
        }
        let Ok(image) = image::open(&texture.source_path) else {
            self.fallback += 1;
            return 0;
        };
        let index = (self.layers.len() + 1) as u16;
        let (w, h) = image.dimensions();
        let frame = if h > w {
            image.crop_imm(0, 0, w, w)
        } else {
            image
        };
        let rgba =
            DynamicImage::ImageRgba8(frame.resize_exact(16, 16, FilterType::Nearest).to_rgba8())
                .to_rgba8();
        self.pixels.extend(rgba.as_raw());
        self.layers.insert(texture.source_path, index);
        index
    }

    fn write(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let mut out = Vec::new();
        out.extend(b"VTA1");
        for value in [1, 16, 16, (self.layers.len() + 1) as u32] {
            out.extend(value.to_le_bytes());
        }
        out.extend(&self.pixels);
        fs::write(path, out)?;
        Ok(())
    }
}

fn query(state: &BlockState) -> BlockStateQuery {
    let mut result = BlockStateQuery::new(&state.name);
    for (key, value) in &state.states {
        let value = match value {
            NbtTag::Byte(n) => BlockStateValue::Int(i64::from(*n)),
            NbtTag::Short(n) => BlockStateValue::Int(i64::from(*n)),
            NbtTag::Int(n) => BlockStateValue::Int(i64::from(*n)),
            NbtTag::String(s) => BlockStateValue::String(s.clone()),
            _ => continue,
        };
        result.states.insert(key.clone(), value);
    }
    result
}

fn is_plant(name: &str) -> bool {
    name.contains("short_grass")
        || name.contains("tall_grass")
        || name.contains("flower")
        || name.contains("seagrass")
        || name.contains("fern")
        || name.contains("sapling")
}

fn face(
    mesh: &mut Mesh,
    textures: &mut Textures,
    block: &str,
    origin: [f32; 3],
    min: [f32; 3],
    max: [f32; 3],
    side: BlockFace,
) {
    let x = origin[0] + min[0];
    let xx = origin[0] + max[0];
    let y = origin[1] + min[1];
    let yy = origin[1] + max[1];
    let z = origin[2] + min[2];
    let zz = origin[2] + max[2];
    let (corners, normal) = match side {
        BlockFace::Up => (
            [[x, yy, z], [x, yy, zz], [xx, yy, zz], [xx, yy, z]],
            [0, 1, 0],
        ),
        BlockFace::Down => ([[x, y, zz], [x, y, z], [xx, y, z], [xx, y, zz]], [0, -1, 0]),
        BlockFace::North => ([[x, y, z], [x, yy, z], [xx, yy, z], [xx, y, z]], [0, 0, -1]),
        BlockFace::South => (
            [[xx, y, zz], [xx, yy, zz], [x, yy, zz], [x, y, zz]],
            [0, 0, 1],
        ),
        BlockFace::East => (
            [[xx, y, z], [xx, yy, z], [xx, yy, zz], [xx, y, zz]],
            [1, 0, 0],
        ),
        BlockFace::West => ([[x, y, zz], [x, yy, zz], [x, yy, z], [x, y, z]], [-1, 0, 0]),
        _ => return,
    };
    let layer = textures.layer(block, normal);
    mesh.quad(
        corners,
        normal,
        [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
        layer,
    );
}

fn block_mesh(
    mesh: &mut Mesh,
    textures: &mut Textures,
    state: &BlockState,
    x: f32,
    y: f32,
    z: f32,
) -> bool {
    let shape = model_shape_for_block_state(&query(state));
    let Some(shape) = shape else {
        return false;
    };
    if shape.is_empty() {
        return false;
    }
    for cuboid in shape.cuboids {
        for side in [
            BlockFace::Up,
            BlockFace::Down,
            BlockFace::North,
            BlockFace::South,
            BlockFace::East,
            BlockFace::West,
        ] {
            face(
                mesh,
                textures,
                &state.name,
                [x, y, z],
                cuboid.min,
                cuboid.max,
                side,
            );
        }
    }
    for plane in shape.planes {
        let corners = plane.corners.map(|p| [x + p[0], y + p[1], z + p[2]]);
        let layer = textures.layer(&state.name, plane.normal);
        mesh.quad(
            corners,
            plane.normal,
            plane
                .uv
                .unwrap_or([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
            layer,
        );
    }
    true
}

pub fn render(
    world: &BedrockWorld,
    anchor: (i32, i32),
    pack: &Path,
    output: &Path,
) -> Result<(), Box<dyn Error>> {
    let mut columns = vec![Column::default(); SIDE * SIDE];
    for cz in 0..4 {
        for cx in 0..4 {
            let pos = ChunkPos {
                x: anchor.0 + cx,
                z: anchor.1 + cz,
                dimension: Dimension::Overworld,
            };
            let chunk = world.get_chunk_blocking(pos)?;
            for sy in -4i8..=19i8 {
                let Some(subchunk) = chunk.get_subchunk(sy)? else {
                    continue;
                };
                for ly in 0..16u8 {
                    for lz in 0..16u8 {
                        for lx in 0..16u8 {
                            let Some(state) = subchunk.visible_block_state_at(lx, ly, lz) else {
                                continue;
                            };
                            if state.name.ends_with(":air") {
                                continue;
                            }
                            let x = cx as usize * 16 + lx as usize;
                            let z = cz as usize * 16 + lz as usize;
                            let y = i32::from(sy) * 16 + i32::from(ly);
                            let column = &mut columns[z * SIDE + x];
                            if state.name.contains("water") {
                                column.water = Some(y);
                            } else if is_plant(&state.name) {
                                column.plant = Some((y, state.clone()));
                            } else {
                                column.solid = Some((y, state.clone()));
                            }
                        }
                    }
                }
            }
        }
    }
    let mut solid = Mesh::default();
    let mut fluid = Mesh::default();
    let mut textures = Textures::new(pack);
    let mut missing_shapes = BTreeMap::<String, usize>::new();
    for z in 0..SIDE {
        for x in 0..SIDE {
            let column = &columns[z * SIDE + x];
            let wx = (anchor.0 * 16 + x as i32) as f32;
            let wz = (anchor.1 * 16 + z as i32) as f32;
            if let Some((y, state)) = &column.solid
                && !block_mesh(&mut solid, &mut textures, state, wx, *y as f32, wz)
            {
                *missing_shapes.entry(state.name.clone()).or_default() += 1;
            }
            if let Some((y, state)) = &column.plant
                && !block_mesh(&mut solid, &mut textures, state, wx, *y as f32, wz)
            {
                *missing_shapes.entry(state.name.clone()).or_default() += 1;
            }
            if let Some(y) = column.water
                && column.solid.as_ref().is_none_or(|(sy, _)| y > *sy)
            {
                face(
                    &mut fluid,
                    &mut textures,
                    "minecraft:water",
                    [wx, y as f32, wz],
                    [0.0, 0.0, 0.0],
                    [1.0, 0.9, 1.0],
                    BlockFace::Up,
                );
            }
        }
    }
    fs::create_dir_all(output)?;
    let mut tile = Vec::new();
    tile.extend(b"VTL4");
    tile.extend(4u32.to_le_bytes());
    solid.write(&mut tile);
    fluid.write(&mut tile);
    tile.extend(1u32.to_le_bytes());
    tile.extend(4u16.to_le_bytes());
    tile.extend(b"none");
    fs::write(output.join("terrain.vtile"), tile)?;
    textures.write(&output.join("terrain.vtexarr"))?;
    let mut summary = fs::File::create(output.join("summary.txt"))?;
    writeln!(
        summary,
        "4x4 BDS Overworld chunks at {}, {}",
        anchor.0, anchor.1
    )?;
    writeln!(
        summary,
        "solid faces: {}, water faces: {}, textures: {}, fallback faces: {}",
        solid.indices.len() / 6,
        fluid.indices.len() / 6,
        textures.layers.len(),
        textures.fallback
    )?;
    writeln!(summary, "missing shape blocks: {missing_shapes:?}")?;
    println!(
        "render: {} solid faces, {} water faces, {} textures, {} fallback faces",
        solid.indices.len() / 6,
        fluid.indices.len() / 6,
        textures.layers.len(),
        textures.fallback
    );
    println!("missing shape blocks: {missing_shapes:?}");
    Ok(())
}
