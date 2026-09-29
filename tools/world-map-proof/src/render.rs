use bedrock_block_model::{
    BlockFace, BlockStateQuery, BlockStateValue, ModelShape, ObjTextureResolver,
    is_full_opaque_block, model_shape_for_block_state,
};
use bedrock_world::{BedrockWorld, BlockState, ChunkPos, Dimension, NbtTag, SubChunkFormat};
use image::{DynamicImage, GenericImageView, imageops::FilterType};
use std::collections::{BTreeMap, HashMap};
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const SIDE: usize = 64;
const MIN_Y: i32 = -64;
const MAX_Y: i32 = 320;

struct Grid {
    blocks: Vec<u16>,
    states: Vec<BlockState>,
    biomes: Vec<u32>,
}

impl Grid {
    fn new() -> Self {
        Self {
            blocks: vec![0; SIDE * SIDE * (MAX_Y - MIN_Y) as usize],
            states: vec![BlockState {
                name: "minecraft:air".into(),
                states: BTreeMap::new(),
                version: None,
            }],
            biomes: vec![u32::MAX; SIDE * SIDE],
        }
    }

    fn index(x: usize, y: i32, z: usize) -> usize {
        (y - MIN_Y) as usize * SIDE * SIDE + z * SIDE + x
    }

    fn get(&self, x: i32, y: i32, z: i32) -> u16 {
        if !(0..SIDE as i32).contains(&x)
            || !(MIN_Y..MAX_Y).contains(&y)
            || !(0..SIDE as i32).contains(&z)
        {
            return 0;
        }
        self.blocks[Self::index(x as usize, y, z as usize)]
    }

    fn state_id(&mut self, state: &BlockState) -> u16 {
        if state.name.ends_with(":air") {
            return 0;
        }
        if let Some(index) = self.states.iter().position(|old| old == state) {
            return index as u16;
        }
        let index = self.states.len() as u16;
        self.states.push(state.clone());
        index
    }
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
    fn quad(
        &mut self,
        corners: [[f32; 3]; 4],
        normal: [i32; 3],
        uv: [[f32; 2]; 4],
        layer: u16,
        tint: [u8; 3],
    ) {
        let start = (self.positions.len() / 3) as u32;
        for i in 0..4 {
            self.positions.extend(corners[i]);
            self.uv.extend(uv[i]);
            self.layer.push(f32::from(layer));
            self.colors.extend([tint[0], tint[1], tint[2], 255]);
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
    grass_tint: [u8; 3],
    foliage_tint: [u8; 3],
    birch_grass_tint: [u8; 3],
    birch_foliage_tint: [u8; 3],
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
            grass_tint: colormap_tint(&pack.join("textures/colormap/grass.png"), [121, 182, 91]),
            foliage_tint: colormap_tint(
                &pack.join("textures/colormap/foliage.png"),
                [110, 160, 80],
            ),
            birch_grass_tint: climate_tint(
                &pack.join("textures/colormap/grass.png"),
                0.6,
                0.6,
                [121, 182, 91],
            ),
            birch_foliage_tint: climate_tint(
                &pack.join("textures/colormap/foliage.png"),
                0.6,
                0.6,
                [110, 160, 80],
            ),
        }
    }

    fn tint(&self, block: &str, normal: [i32; 3], biome_id: u32) -> [u8; 3] {
        let name = block.strip_prefix("minecraft:").unwrap_or(block);
        let birch = matches!(biome_id, 27 | 155);
        if name == "grass_block" && normal[1] > 0
            || name.contains("short_grass")
            || name.contains("tall_grass")
            || name.contains("fern")
        {
            if birch {
                self.birch_grass_tint
            } else {
                self.grass_tint
            }
        } else if name.contains("leaves") || name.contains("leaf") || name.contains("vine") {
            if birch {
                self.birch_foliage_tint
            } else {
                self.foliage_tint
            }
        } else if name.contains("water") {
            [68, 175, 245]
        } else {
            [255, 255, 255]
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
        // Image decoders return the top row first; WebGL texture arrays consume
        // the bottom row first. This placed Bedrock grass-side fringe below dirt.
        for row in (0..16).rev() {
            let start = row * 16 * 4;
            self.pixels.extend(&rgba.as_raw()[start..start + 16 * 4]);
        }
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

// Bedrock textures are greyscale masks for grass/foliage. The proof uses the
// supplied pack's colormap at temperate default climate until biome IDs are
// mapped to climate data; this avoids pretending the grey texture is final.
fn colormap_tint(path: &Path, fallback: [u8; 3]) -> [u8; 3] {
    sample_colormap(path, 51, 173, fallback)
}

fn climate_tint(path: &Path, temperature: f32, downfall: f32, fallback: [u8; 3]) -> [u8; 3] {
    let x = ((1.0 - temperature) * 255.0).round() as u32;
    let y = ((1.0 - downfall * temperature) * 255.0).round() as u32;
    sample_colormap(path, x, y, fallback)
}

fn sample_colormap(path: &Path, x: u32, y: u32, fallback: [u8; 3]) -> [u8; 3] {
    let Ok(image) = image::open(path) else {
        return fallback;
    };
    let image = image.to_rgba8();
    if image.width() != 256 || image.height() != 256 {
        return fallback;
    }
    let pixel = image.get_pixel(x, y).0;
    [pixel[0], pixel[1], pixel[2]]
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

#[allow(clippy::too_many_arguments)]
fn face(
    mesh: &mut Mesh,
    textures: &mut Textures,
    block: &str,
    origin: [f32; 3],
    min: [f32; 3],
    max: [f32; 3],
    side: BlockFace,
    uv: Option<[[f32; 2]; 4]>,
    biome_id: u32,
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
    let tint = textures.tint(block, normal, biome_id);
    // Block-model UVs arrive as image top-left, top-right, bottom-right,
    // bottom-left. Our face vertices begin at bottom-left, and the WebGL
    // texture array uses bottom-origin V coordinates.
    let uv = uv.map(|uv| [uv[3], uv[0], uv[1], uv[2]].map(|[u, v]| [u, 1.0 - v]));
    mesh.quad(
        corners,
        normal,
        uv.unwrap_or([[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]),
        layer,
        tint,
    );
}

fn read_grid(world: &BedrockWorld, anchor: (i32, i32)) -> Result<Grid, Box<dyn Error>> {
    let mut grid = Grid::new();
    for cz in 0..4 {
        for cx in 0..4 {
            let pos = ChunkPos {
                x: anchor.0 + cx,
                z: anchor.1 + cz,
                dimension: Dimension::Overworld,
            };
            let chunk = world.get_chunk_blocking(pos)?;
            for lz in 0..16u8 {
                for lx in 0..16u8 {
                    let x = cx as usize * 16 + usize::from(lx);
                    let z = cz as usize * 16 + usize::from(lz);
                    let y = world
                        .get_height_at_blocking(pos, lx, lz)?
                        .map(i32::from)
                        .unwrap_or(87);
                    grid.biomes[z * SIDE + x] = world
                        .get_biome_id_blocking(pos, lx, lz, y)?
                        .unwrap_or(u32::MAX);
                }
            }
            for sy in -4i8..=19i8 {
                let Some(subchunk) = chunk.get_subchunk(sy)? else {
                    continue;
                };
                let SubChunkFormat::Paletted { ref storages, .. } = subchunk.format else {
                    continue;
                };
                let mut local_ids = HashMap::<usize, u16>::new();
                for storage in storages {
                    for state in &storage.states {
                        local_ids.insert(state as *const BlockState as usize, grid.state_id(state));
                    }
                }
                for ly in 0..16u8 {
                    for lz in 0..16u8 {
                        for lx in 0..16u8 {
                            let Some(state) = subchunk.visible_block_state_at(lx, ly, lz) else {
                                continue;
                            };
                            let Some(&id) = local_ids.get(&(state as *const BlockState as usize))
                            else {
                                continue;
                            };
                            let x = cx as usize * 16 + lx as usize;
                            let z = cz as usize * 16 + lz as usize;
                            let y = i32::from(sy) * 16 + i32::from(ly);
                            grid.blocks[Grid::index(x, y, z)] = id;
                        }
                    }
                }
            }
        }
    }
    Ok(grid)
}

fn neighbor(x: i32, y: i32, z: i32, side: BlockFace) -> (i32, i32, i32) {
    match side {
        BlockFace::Up => (x, y + 1, z),
        BlockFace::Down => (x, y - 1, z),
        BlockFace::North => (x, y, z - 1),
        BlockFace::South => (x, y, z + 1),
        BlockFace::East => (x + 1, y, z),
        BlockFace::West => (x - 1, y, z),
        _ => (x, y, z),
    }
}

fn is_water(name: &str) -> bool {
    name == "minecraft:water" || name == "minecraft:flowing_water"
}
fn is_leaf(name: &str) -> bool {
    name.contains("leaves") || name.contains("leaf")
}

fn face_hidden(grid: &Grid, id: u16, x: i32, y: i32, z: i32, side: BlockFace) -> bool {
    let (nx, ny, nz) = neighbor(x, y, z, side);
    let other = grid.get(nx, ny, nz);
    if other == 0 {
        return false;
    }
    let other_name = &grid.states[other as usize].name;
    let name = &grid.states[id as usize].name;
    is_full_opaque_block(other_name)
        || is_water(name) && is_water(other_name)
        || is_leaf(name) && is_leaf(other_name)
}

fn on_boundary(min: [f32; 3], max: [f32; 3], side: BlockFace) -> bool {
    match side {
        BlockFace::Up => max[1] >= 0.999,
        BlockFace::Down => min[1] <= 0.001,
        BlockFace::North => min[2] <= 0.001,
        BlockFace::South => max[2] >= 0.999,
        BlockFace::East => max[0] >= 0.999,
        BlockFace::West => min[0] <= 0.001,
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_plane(
    mesh: &mut Mesh,
    textures: &mut Textures,
    name: &str,
    origin: [f32; 3],
    corners: [[f32; 3]; 4],
    normal: [i32; 3],
    uv: [[f32; 2]; 4],
    biome_id: u32,
) {
    let points = corners.map(|p| [origin[0] + p[0], origin[1] + p[1], origin[2] + p[2]]);
    let layer = textures.layer(name, normal);
    let tint = textures.tint(name, normal, biome_id);
    mesh.quad(points, normal, uv, layer, tint);
    // Cross plants need to be visible from either side of their plane.
    let back = [points[0], points[3], points[2], points[1]];
    mesh.quad(
        back,
        [-normal[0], -normal[1], -normal[2]],
        [uv[0], uv[3], uv[2], uv[1]],
        layer,
        tint,
    );
}

pub fn render(
    world: &BedrockWorld,
    anchor: (i32, i32),
    pack: &Path,
    output: &Path,
) -> Result<(), Box<dyn Error>> {
    let grid = read_grid(world, anchor)?;
    let shapes: Vec<Option<ModelShape>> = grid
        .states
        .iter()
        .map(|state| model_shape_for_block_state(&query(state)))
        .collect();
    let mut solid = Mesh::default();
    let mut fluid = Mesh::default();
    let mut textures = Textures::new(pack);
    let mut missing_shapes = BTreeMap::<String, usize>::new();
    let mut drawn_logs = 0usize;
    let mut biome_counts = BTreeMap::<u32, usize>::new();
    for &id in &grid.biomes {
        *biome_counts.entry(id).or_default() += 1;
    }
    for y in MIN_Y..MAX_Y {
        for z in 0..SIDE as i32 {
            for x in 0..SIDE as i32 {
                let id = grid.get(x, y, z);
                if id == 0 {
                    continue;
                }
                let state = &grid.states[id as usize];
                let biome_id = grid.biomes[z as usize * SIDE + x as usize];
                let origin = [
                    (anchor.0 * 16 + x) as f32,
                    y as f32,
                    (anchor.1 * 16 + z) as f32,
                ];
                if is_water(&state.name) {
                    for side in [
                        BlockFace::Up,
                        BlockFace::North,
                        BlockFace::South,
                        BlockFace::East,
                        BlockFace::West,
                    ] {
                        if face_hidden(&grid, id, x, y, z, side) {
                            continue;
                        }
                        face(
                            &mut fluid,
                            &mut textures,
                            &state.name,
                            origin,
                            [0.0, 0.0, 0.0],
                            [1.0, 0.9, 1.0],
                            side,
                            None,
                            biome_id,
                        );
                    }
                    continue;
                }
                let Some(shape) = &shapes[id as usize] else {
                    *missing_shapes.entry(state.name.clone()).or_default() += 1;
                    continue;
                };
                if shape.is_empty() {
                    *missing_shapes.entry(state.name.clone()).or_default() += 1;
                    continue;
                }
                if state.name.contains("_log") || state.name.ends_with(":log") {
                    drawn_logs += 1;
                }
                for cuboid in &shape.cuboids {
                    for side in [
                        BlockFace::Up,
                        BlockFace::Down,
                        BlockFace::North,
                        BlockFace::South,
                        BlockFace::East,
                        BlockFace::West,
                    ] {
                        if on_boundary(cuboid.min, cuboid.max, side)
                            && face_hidden(&grid, id, x, y, z, side)
                        {
                            continue;
                        }
                        face(
                            &mut solid,
                            &mut textures,
                            &state.name,
                            origin,
                            cuboid.min,
                            cuboid.max,
                            side,
                            cuboid.face_uvs.get(&side).copied(),
                            biome_id,
                        );
                    }
                }
                for plane in &shape.planes {
                    emit_plane(
                        &mut solid,
                        &mut textures,
                        &state.name,
                        origin,
                        plane.corners,
                        plane.normal,
                        plane
                            .uv
                            .unwrap_or([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
                        biome_id,
                    );
                }
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
        "solid faces: {}, water faces: {}, textures: {}, fallback faces: {}, log blocks: {}",
        solid.indices.len() / 6,
        fluid.indices.len() / 6,
        textures.layers.len(),
        textures.fallback,
        drawn_logs
    )?;
    writeln!(summary, "missing shape blocks: {missing_shapes:?}")?;
    writeln!(summary, "surface biome IDs by column: {biome_counts:?}")?;
    writeln!(
        summary,
        "birch grass tint: {:?}, foliage tint: {:?}",
        textures.birch_grass_tint, textures.birch_foliage_tint
    )?;
    println!(
        "render: {} solid faces, {} water faces, {} textures, {} fallback faces, {} log blocks",
        solid.indices.len() / 6,
        fluid.indices.len() / 6,
        textures.layers.len(),
        textures.fallback,
        drawn_logs
    );
    println!("missing shape blocks: {missing_shapes:?}");
    println!("surface biome IDs by column: {biome_counts:?}");
    println!(
        "birch grass tint: {:?}, foliage tint: {:?}",
        textures.birch_grass_tint, textures.birch_foliage_tint
    );
    Ok(())
}
