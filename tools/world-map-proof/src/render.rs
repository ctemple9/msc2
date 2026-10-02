use bedrock_block_model::{
    BlockFace, BlockStateQuery, BlockStateValue, ModelPlane, ModelShape, ObjTextureResolver,
    is_full_opaque_block, model_shape_for_block_state,
};
use bedrock_world::chunk::parse_subchunk;
use bedrock_world::{
    BedrockWorld, BlockState, ChunkKey, ChunkPos, Dimension, NbtTag, SubChunkFormat,
};
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
    biomes_3d: Vec<u32>,
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
            biomes_3d: vec![u32::MAX; SIDE * SIDE * (MAX_Y - MIN_Y) as usize],
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

    fn write_quantized(&self, out: &mut Vec<u8>) {
        let vertex_count = self.positions.len() / 3;
        out.extend((vertex_count as u32).to_le_bytes());
        out.extend((self.indices.len() as u32).to_le_bytes());
        for value in &self.uv {
            out.extend(value.to_le_bytes());
        }
        out.extend(&self.colors);
        out.extend(&self.normals);
        for value in &self.indices {
            out.extend(value.to_le_bytes());
        }

        let (min, scale) = if vertex_count == 0 {
            ([0.0; 3], [1.0; 3])
        } else {
            let mut min = [f32::INFINITY; 3];
            let mut max = [f32::NEG_INFINITY; 3];
            for position in self.positions.chunks_exact(3) {
                for axis in 0..3 {
                    min[axis] = min[axis].min(position[axis]);
                    max[axis] = max[axis].max(position[axis]);
                }
            }
            let scale = std::array::from_fn(|axis| (max[axis] - min[axis]) / f32::from(u16::MAX));
            (min, scale)
        };
        for value in min.into_iter().chain(scale) {
            out.extend(value.to_le_bytes());
        }
        for position in self.positions.chunks_exact(3) {
            for axis in 0..3 {
                let quantized = if scale[axis] == 0.0 {
                    0
                } else {
                    ((position[axis] - min[axis]) / scale[axis])
                        .round()
                        .clamp(0.0, f32::from(u16::MAX)) as u16
                };
                out.extend(quantized.to_le_bytes());
            }
        }
        for value in &self.layer {
            out.extend((*value as u16).to_le_bytes());
        }
        for value in &self.biome {
            out.extend((*value as u16).to_le_bytes());
        }
        out.resize(out.len().next_multiple_of(4), 0);
    }
}

struct Textures {
    resolver: ObjTextureResolver,
    layers: BTreeMap<PathBuf, u16>,
    pixels: Vec<u8>,
    face_layers: HashMap<String, HashMap<[i32; 3], u16>>,
    fallback: usize,
    fallback_blocks: BTreeMap<String, usize>,
    grass_tint: [u8; 3],
    foliage_tint: [u8; 3],
    birch_grass_tint: [u8; 3],
    birch_foliage_tint: [u8; 3],
    mountain_tints: BTreeMap<u32, ([u8; 3], [u8; 3])>,
}

impl Textures {
    fn new(pack: &Path) -> Self {
        let grass_map = pack.join("textures/colormap/grass.png");
        let foliage_map = pack.join("textures/colormap/foliage.png");
        let mountain_tints = [
            (183, -0.7, 0.9), // Frozen peaks
            (185, -0.2, 0.8), // Grove
            (189, 1.0, 0.3),  // Stony peaks
        ]
        .into_iter()
        .map(|(id, temperature, downfall)| {
            (
                id,
                (
                    climate_tint(&grass_map, temperature, downfall, [121, 182, 91]),
                    climate_tint(&foliage_map, temperature, downfall, [110, 160, 80]),
                ),
            )
        })
        .collect();
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
            face_layers: HashMap::new(),
            fallback: 0,
            fallback_blocks: BTreeMap::new(),
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
            mountain_tints,
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
            if let Some((grass, _)) = self.mountain_tints.get(&biome_id) {
                *grass
            } else if birch {
                self.birch_grass_tint
            } else {
                self.grass_tint
            }
        } else if name.contains("leaves") || name.contains("leaf") || name.contains("vine") {
            if let Some((_, foliage)) = self.mountain_tints.get(&biome_id) {
                *foliage
            } else if birch {
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
        // Resolving a material builds its block model and checks texture paths.
        // Do that once per block/face, rather than for every emitted face.
        if let Some(&layer) = self
            .face_layers
            .get(block)
            .and_then(|faces| faces.get(&normal))
        {
            if layer == 0 {
                self.fallback += 1;
                *self.fallback_blocks.entry(block.to_owned()).or_default() += 1;
            }
            return layer;
        }
        let layer = self.resolve_layer(block, normal);
        self.face_layers
            .entry(block.to_owned())
            .or_default()
            .insert(normal, layer);
        layer
    }

    fn resolve_layer(&mut self, block: &str, normal: [i32; 3]) -> u16 {
        let Some(texture) = self.resolver.texture_for(block, normal) else {
            self.fallback += 1;
            *self.fallback_blocks.entry(block.to_owned()).or_default() += 1;
            return 0;
        };
        if let Some(index) = self.layers.get(&texture.source_path) {
            return *index;
        }
        let Ok(image) = image::open(&texture.source_path) else {
            self.fallback += 1;
            *self.fallback_blocks.entry(block.to_owned()).or_default() += 1;
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

    fn restore(pack: &Path, output: &Path) -> Result<Self, Box<dyn Error>> {
        let mut textures = Self::new(pack);
        textures.layers = serde_json::from_slice(&fs::read(output.join("texture-index.json"))?)?;
        let atlas = fs::read(output.join("terrain.vtexarr"))?;
        if atlas.len() < 20 || &atlas[..4] != b"VTA1" {
            return Err("Bedrock texture array is invalid".into());
        }
        let count = u32::from_le_bytes(atlas[16..20].try_into()?) as usize;
        if count < textures.layers.len() + 1 || atlas.len() != 20 + count * 1024 {
            return Err("Bedrock texture array and index disagree".into());
        }
        textures.pixels = atlas[20..20 + (textures.layers.len() + 1) * 1024].to_vec();
        Ok(textures)
    }

    fn write_shared(&self, output: &Path) -> Result<(), Box<dyn Error>> {
        let atlas = output.join("terrain.vtexarr");
        let index = output.join("texture-index.json");
        self.write(&output.join("terrain.vtexarr.tmp"))?;
        fs::rename(output.join("terrain.vtexarr.tmp"), atlas)?;
        fs::write(
            output.join("texture-index.json.tmp"),
            serde_json::to_vec(&self.layers)?,
        )?;
        fs::rename(output.join("texture-index.json.tmp"), index)?;
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
    let temperature = temperature.clamp(0.0, 1.0);
    let downfall = downfall.clamp(0.0, 1.0);
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
    if state.name.contains("stairs")
        && let Some(NbtTag::Int(value)) = state.states.get("weirdo_direction")
    {
        let direction = match value.rem_euclid(4) {
            0 => "east",
            1 => "west",
            2 => "south",
            _ => "north",
        };
        result.states.insert(
            "minecraft:cardinal_direction".into(),
            BlockStateValue::String(direction.into()),
        );
    }
    result
}

fn is_fence(name: &str) -> bool {
    name.ends_with("_fence") || name == "minecraft:fence"
}

fn is_pane(name: &str) -> bool {
    name.ends_with("_pane") || name == "minecraft:glass_pane"
}

fn connected_shape(grid: &Grid, state: &BlockState, x: i32, y: i32, z: i32) -> Option<ModelShape> {
    let fence = is_fence(&state.name);
    let pane = is_pane(&state.name);
    if !fence && !pane {
        return None;
    }
    let mut block = query(state);
    let mut connections = [false; 4];
    for (index, (key, dx, dz)) in [
        ("north", 0, -1),
        ("south", 0, 1),
        ("east", 1, 0),
        ("west", -1, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let other = grid.get(x + dx, y, z + dz);
        let connected = if other == 0 {
            false
        } else {
            let other_name = &grid.states[other as usize].name;
            is_full_opaque_block(other_name)
                || fence && (is_fence(other_name) || other_name.ends_with("_fence_gate"))
                || pane && is_pane(other_name)
        };
        connections[index] = connected;
        block
            .states
            .insert(format!("{key}_connected"), BlockStateValue::Bool(connected));
    }
    if pane {
        let along_x = connections[2] || connections[3];
        let along_z = connections[0] || connections[1];
        let uv = [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
        let mut planes = Vec::with_capacity(2);
        if along_x || !along_z {
            planes.push(
                ModelPlane::new(
                    [
                        [0.0, 0.0, 0.5],
                        [0.0, 1.0, 0.5],
                        [1.0, 1.0, 0.5],
                        [1.0, 0.0, 0.5],
                    ],
                    [0, 0, 1],
                )
                .with_uv(uv),
            );
        }
        if along_z || !along_x {
            planes.push(
                ModelPlane::new(
                    [
                        [0.5, 0.0, 1.0],
                        [0.5, 1.0, 1.0],
                        [0.5, 1.0, 0.0],
                        [0.5, 0.0, 0.0],
                    ],
                    [1, 0, 0],
                )
                .with_uv(uv),
            );
        }
        return Some(ModelShape::default().with_planes(planes));
    }
    model_shape_for_block_state(&block)
}

fn atlas_uv(u0: f32, v0: f32, u1: f32, v1: f32) -> [[f32; 2]; 4] {
    [
        [u0 / 16.0, v0 / 16.0],
        [u1 / 16.0, v0 / 16.0],
        [u1 / 16.0, v1 / 16.0],
        [u0 / 16.0, v1 / 16.0],
    ]
}

fn shape_with_lantern_uv(state: &BlockState) -> Option<ModelShape> {
    let mut shape = model_shape_for_block_state(&query(state))?;
    if (state.name.ends_with("_lantern") || state.name == "minecraft:lantern")
        && state.name != "minecraft:sea_lantern"
    {
        for (index, cuboid) in shape.cuboids.iter_mut().enumerate() {
            let uv = match index {
                0 => atlas_uv(0.0, 2.0, 6.0, 8.0),
                1 => atlas_uv(0.0, 9.0, 6.0, 15.0),
                _ => atlas_uv(1.0, 0.0, 5.0, 2.0),
            };
            for face in [
                BlockFace::Up,
                BlockFace::Down,
                BlockFace::North,
                BlockFace::South,
                BlockFace::East,
                BlockFace::West,
            ] {
                cuboid.face_uvs.insert(face, uv);
            }
        }
    }
    Some(shape)
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

fn read_grid(
    world: &BedrockWorld,
    anchor: (i32, i32),
    dimension: Dimension,
) -> Result<Grid, Box<dyn Error>> {
    let mut grid = Grid::new();
    for cz in 0..4 {
        for cx in 0..4 {
            let pos = ChunkPos {
                x: anchor.0 + cx,
                z: anchor.1 + cz,
                dimension,
            };
            let chunk = world.get_chunk_blocking(pos)?;
            if let Some(storages) = world.get_biome_storages_blocking(pos)? {
                for storage in storages {
                    let (start, end) = match storage.y {
                        Some(y) => (y.max(MIN_Y), (y + 16).min(MAX_Y)),
                        None => (MIN_Y, MAX_Y),
                    };
                    for y in start..end {
                        let ly = storage.y.map_or(0, |section_y| (y - section_y) as u8);
                        for lz in 0..16u8 {
                            for lx in 0..16u8 {
                                let x = cx as usize * 16 + lx as usize;
                                let z = cz as usize * 16 + lz as usize;
                                grid.biomes_3d[Grid::index(x, y, z)] =
                                    storage.biome_id_at(lx, ly, lz).unwrap_or(u32::MAX);
                            }
                        }
                    }
                }
            }
            // get_height_at_blocking reparses the complete biome record on
            // every call. Read its 256-column height map once per chunk.
            let heights = world.get_height_map_blocking(pos)?;
            for lz in 0..16u8 {
                for lx in 0..16u8 {
                    let x = cx as usize * 16 + usize::from(lx);
                    let z = cz as usize * 16 + usize::from(lz);
                    let y = heights
                        .as_ref()
                        .and_then(|map| map[usize::from(lz)][usize::from(lx)])
                        .map(i32::from)
                        .unwrap_or(87);
                    grid.biomes[z * SIDE + x] = if (MIN_Y..MAX_Y).contains(&y) {
                        grid.biomes_3d[Grid::index(x, y, z)]
                    } else {
                        u32::MAX
                    };
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
    // Bedrock's east pane face selects the opaque narrow edge texture. These
    // flat planes represent the broad glass surface in either orientation.
    let texture_normal = if is_pane(name) { [0, 0, 1] } else { normal };
    let layer = textures.layer(name, texture_normal);
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
    registry_status: &str,
) -> Result<(), Box<dyn Error>> {
    let mut textures = Textures::new(pack);
    render_tile(world, anchor, output, registry_status, &mut textures)?;
    textures.write(&output.join("terrain.vtexarr"))?;
    Ok(())
}

pub fn render_grid(
    world: &BedrockWorld,
    anchors: &[(i32, i32)],
    pack: &Path,
    output: &Path,
    spawn: Option<(i32, i32, i32)>,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(output.join("tiles"))?;
    let mut textures = Textures::new(pack);
    let mut tiles = Vec::new();
    for &(x, z) in anchors {
        let tile_dir = output.join(format!("tile_{x}_{z}"));
        render_tile(world, (x, z), &tile_dir, "provisional", &mut textures)?;
        let path = format!("tiles/t.{}.{}.vtile", x.div_euclid(4), z.div_euclid(4));
        let source = tile_dir.join("terrain.vtile");
        let bytes = fs::metadata(&source)?.len();
        fs::rename(source, output.join(&path))?;
        fs::remove_dir_all(tile_dir)?;
        tiles.push(serde_json::json!({"x": x.div_euclid(4), "z": z.div_euclid(4), "path": path, "bytes": bytes}));
    }
    textures.write(&output.join("terrain.vtexarr"))?;
    fs::write(
        output.join("manifest.json"),
        serde_json::to_vec(&serde_json::json!({
            "format": 1,
            "tileChunks": 4,
            "tileBlocks": 64,
            "textures": "terrain.vtexarr",
            "biomes": [],
            "tiles": tiles,
            "spawn": spawn.map(|(x, y, z)| serde_json::json!({"x": x, "y": y, "z": z})),
        }))?,
    )?;
    Ok(())
}

// One surface sample per chunk, independent of the expensive detailed meshes.
// Shared edge samples come from the same map, so adjacent overview tiles agree.
fn create_overview(
    world: &BedrockWorld,
    positions: &[ChunkPos],
    pack: &Path,
    output: &Path,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let mut textures = Textures::new(pack);
    let mut colors = HashMap::<String, [u8; 3]>::new();
    let mut samples = BTreeMap::new();
    for &pos in positions {
        let Some(map) = world.get_height_map_blocking(pos)? else {
            continue;
        };
        let Some(height) = map[8][8] else { continue };
        // BDS height records may point at the first air block above the
        // surface. Resolve the actual visible block rather than painting that
        // air as a default material. Only decode the one or two touched sections.
        let mut surface = None;
        let mut section_y = i8::MAX;
        let mut section = None;
        for y in (i32::from(height) - 16..=i32::from(height)).rev() {
            let next_section = y.div_euclid(16) as i8;
            if next_section != section_y {
                section_y = next_section;
                section = world
                    .storage()
                    .get(&ChunkKey::subchunk(pos, section_y).encode())?
                    .map(|bytes| parse_subchunk(section_y, bytes))
                    .transpose()?;
            }
            if let Some(state) = section
                .as_ref()
                .and_then(|section| section.visible_block_state_at(8, y.rem_euclid(16) as u8, 8))
            {
                surface = Some((y as i16, state.name.clone()));
                break;
            }
        }
        let Some((height, name)) = surface else {
            continue;
        };
        let name = name.as_str();
        let color = *colors.entry(name.to_owned()).or_insert_with(|| {
            let layer = textures.layer(name, [0, 1, 0]) as usize;
            let pixels = &textures.pixels[layer * 1024..(layer + 1) * 1024];
            let mut sum = [0u32; 3];
            let mut weight = 0u32;
            for pixel in pixels.chunks_exact(4) {
                weight += u32::from(pixel[3]);
                for channel in 0..3 {
                    sum[channel] += u32::from(pixel[channel]) * u32::from(pixel[3]);
                }
            }
            let tint = textures.tint(name, [0, 1, 0], u32::MAX);
            let mut color = std::array::from_fn(|channel| {
                ((sum[channel] / weight.max(1)) * u32::from(tint[channel]) / 255) as u8
            });
            if name.contains("water") {
                color = [48, 101, 185];
            }
            color
        });
        samples.insert((pos.x, pos.z), (height, color));
    }
    // Keep the whole-world overview below the viewer's 160 coarse-tile limit,
    // even for scattered worlds. Each tile uses a fixed 65 x 65 sample grid.
    let mut level = 4u32;
    let anchors = loop {
        let chunks = 4i32
            .checked_shl(level)
            .ok_or("overview coordinates exceed supported range")?;
        let anchors: std::collections::BTreeSet<_> = samples
            .keys()
            .map(|&(x, z)| (x.div_euclid(chunks), z.div_euclid(chunks)))
            .collect();
        if anchors.len() <= 128 {
            break anchors;
        }
        if level >= 20 {
            return Err("overview extent exceeds supported coordinates".into());
        }
        level += 1;
    };
    let span = 1i32 << level;
    let tile_blocks = 64 * span;
    let chunk_step = span / 16;
    // Aggregate rather than skip samples when the overview needs larger cells.
    let mut cells = BTreeMap::<(i32, i32), (i16, [u8; 3])>::new();
    for ((x, z), sample) in samples {
        let key = (x.div_euclid(chunk_step), z.div_euclid(chunk_step));
        cells
            .entry(key)
            .and_modify(|old| {
                if sample.0 > old.0 {
                    *old = sample;
                }
            })
            .or_insert(sample);
    }
    fs::create_dir_all(output.join("overview"))?;
    let mut tiles = Vec::new();
    for (x, z) in anchors {
        let mut heights = Vec::with_capacity(65 * 65);
        let mut rgb = Vec::with_capacity(65 * 65 * 3);
        for j in 0..65 {
            for i in 0..65 {
                let (height, color) = cells
                    .get(&(x * 64 + i, z * 64 + j))
                    .copied()
                    .unwrap_or((i16::MIN, [0; 3]));
                heights.push(height);
                rgb.extend(color);
            }
        }
        let mut bytes = Vec::new();
        bytes.extend(b"VLR1");
        for value in [1u32, 65, 65] {
            bytes.extend(value.to_le_bytes());
        }
        bytes.extend((x * tile_blocks).to_le_bytes());
        bytes.extend((z * tile_blocks).to_le_bytes());
        bytes.extend((span as u32).to_le_bytes());
        for height in heights {
            bytes.extend(height.to_le_bytes());
        }
        bytes.extend(rgb);
        let path = format!("overview/t.{x}.{z}.vlr");
        fs::write(output.join(&path), &bytes)?;
        tiles.push(serde_json::json!({"x": x, "z": z, "path": path, "bytes": bytes.len()}));
    }
    Ok(
        serde_json::json!({"grid": 65, "levels": [{"level": level, "tileBlocks": tile_blocks, "span": span, "tiles": tiles}]}),
    )
}

pub fn create_catalog(
    world: &BedrockWorld,
    positions: &[ChunkPos],
    anchors: &[(i32, i32)],
    pack: &Path,
    output: &Path,
    spawn: Option<(i32, i32, i32)>,
    dimension_id: &str,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(output.join("tiles"))?;
    Textures::new(pack).write_shared(output)?;
    // A roof heightfield cannot cover the Nether's hollow interior. Advertising
    // it as whole-world coverage disables the viewer's detail-frontier fog and
    // exposes unloaded cave tile edges. Keep interior views bounded by detail.
    let overview = if dimension_id == "minecraft:the_nether" {
        serde_json::Value::Null
    } else {
        create_overview(world, positions, pack, output)?
    };
    let tiles: Vec<_> = anchors
        .iter()
        .map(|&(x, z)| {
            serde_json::json!({
                "x": x.div_euclid(4), "z": z.div_euclid(4),
                "path": format!("tiles/t.{}.{}.vtile", x.div_euclid(4), z.div_euclid(4)),
                "bytes": 0,
            })
        })
        .collect();
    fs::write(
        output.join("manifest.json"),
        serde_json::to_vec(&serde_json::json!({
            "format": 2, "tileChunks": 4, "tileBlocks": 64, "lowres": overview,
            "textures": "terrain.vtexarr", "textureLayers": 1,
            "rendering": true, "dynamic": true, "caves": true,
            "yRange": match dimension_id {
                "minecraft:the_nether" => serde_json::json!({"min": 0, "max": 128}),
                "minecraft:the_end" => serde_json::json!({"min": 0, "max": 256}),
                _ => serde_json::json!({"min": MIN_Y, "max": MAX_Y}),
            },
            "biomes": [], "tiles": tiles,
            "dimension": {
                "id": dimension_id,
                "slug": dimension_id.trim_start_matches("minecraft:"),
                "label": match dimension_id {
                    "minecraft:the_nether" => "The Nether",
                    "minecraft:the_end" => "The End",
                    _ => "Overworld",
                },
                "kind": match dimension_id {
                    "minecraft:the_nether" => "nether",
                    "minecraft:the_end" => "end",
                    _ => "overworld",
                },
            },
            "atmosphere": match dimension_id {
                "minecraft:the_nether" => serde_json::json!({
                    "skyTop": [42, 9, 9], "skyHorizon": [79, 24, 19],
                    "fog": [79, 24, 19], "ambient": 0.28, "daylight": 0.0,
                }),
                "minecraft:the_end" => serde_json::json!({
                    "skyTop": [17, 15, 23], "skyHorizon": [31, 27, 38],
                    "fog": [42, 37, 50], "ambient": 0.35, "daylight": 0.0,
                }),
                _ => serde_json::Value::Null,
            },
            "spawn": spawn.map(|(x, y, z)| serde_json::json!({"x": x, "y": y, "z": z})),
        }))?,
    )?;
    Ok(())
}

pub fn render_catalog_tile(
    world: &BedrockWorld,
    anchor: (i32, i32),
    pack: &Path,
    output: &Path,
    dimension: Dimension,
) -> Result<(), Box<dyn Error>> {
    let mut textures = Textures::restore(pack, output)?;
    let working = output.join("working-tile");
    if working.exists() {
        fs::remove_dir_all(&working)?;
    }
    render_tile_in_dimension(
        world,
        anchor,
        &working,
        "provisional",
        &mut textures,
        dimension,
    )?;
    textures.write_shared(output)?;
    let tile = output.join(format!(
        "tiles/t.{}.{}.vtile",
        anchor.0.div_euclid(4),
        anchor.1.div_euclid(4)
    ));
    fs::rename(working.join("terrain.vtile"), tile)?;
    fs::remove_dir_all(working)?;
    Ok(())
}

fn render_tile(
    world: &BedrockWorld,
    anchor: (i32, i32),
    output: &Path,
    registry_status: &str,
    textures: &mut Textures,
) -> Result<(), Box<dyn Error>> {
    render_tile_in_dimension(
        world,
        anchor,
        output,
        registry_status,
        textures,
        Dimension::Overworld,
    )
}

fn render_tile_in_dimension(
    world: &BedrockWorld,
    anchor: (i32, i32),
    output: &Path,
    registry_status: &str,
    textures: &mut Textures,
    dimension: Dimension,
) -> Result<(), Box<dyn Error>> {
    let grid = read_grid(world, anchor, dimension)?;
    let shapes: Vec<Option<ModelShape>> = grid.states.iter().map(shape_with_lantern_uv).collect();
    let mut solid = Mesh::default();
    let mut fluid = Mesh::default();
    let mut missing_shapes = BTreeMap::<String, usize>::new();
    let mut drawn_logs = 0usize;
    let mut biome_counts = BTreeMap::<u32, usize>::new();
    let mut rendered_block_biome_counts = BTreeMap::<u32, usize>::new();
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
                let biome_id = grid.biomes_3d[Grid::index(x as usize, y, z as usize)];
                *rendered_block_biome_counts.entry(biome_id).or_default() += 1;
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
                            textures,
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
                let local_shape = connected_shape(&grid, state, x, y, z);
                let Some(shape) = local_shape.as_ref().or(shapes[id as usize].as_ref()) else {
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
                            textures,
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
                        textures,
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
    tile.extend(b"VTL6");
    tile.extend(6u32.to_le_bytes());
    solid.write_quantized(&mut tile);
    fluid.write_quantized(&mut tile);
    // VTL6 stores a surface map before the biome-name legend. This Bedrock
    // exporter does not yet derive those summaries, so write an empty surface.
    for value in [0u32, 0u32, 0u32, 0u32] {
        tile.extend(value.to_le_bytes());
    }
    tile.extend(1u32.to_le_bytes());
    tile.extend(4u16.to_le_bytes());
    tile.extend(b"none");
    fs::write(output.join("terrain.vtile"), tile)?;
    let mut summary = fs::File::create(output.join("summary.txt"))?;
    writeln!(
        summary,
        "4x4 BDS {:?} chunks at {}, {}",
        dimension, anchor.0, anchor.1
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
    if let Ok(document) = world.read_level_dat_blocking()
        && let NbtTag::Compound(root) = document.root
    {
        writeln!(
            summary,
            "save lastOpenedWithVersion: {:?}",
            root.get("lastOpenedWithVersion")
        )?;
        writeln!(
            summary,
            "save StorageVersion: {:?}",
            root.get("StorageVersion")
        )?;
    }
    writeln!(summary, "fallback blocks: {:?}", textures.fallback_blocks)?;
    writeln!(summary, "biome ID mapping: {registry_status}")?;
    writeln!(summary, "surface biome IDs by column: {biome_counts:?}")?;
    writeln!(
        summary,
        "biome IDs by non-air block: {rendered_block_biome_counts:?}"
    )?;
    writeln!(
        summary,
        "mapped mountain grass/foliage tints: {:?}",
        textures.mountain_tints
    )?;
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
    println!("fallback blocks: {:?}", textures.fallback_blocks);
    println!("surface biome IDs by column: {biome_counts:?}");
    println!("biome IDs by non-air block: {rendered_block_biome_counts:?}");
    println!(
        "mapped mountain grass/foliage tints: {:?}",
        textures.mountain_tints
    );
    println!(
        "birch grass tint: {:?}, foliage tint: {:?}",
        textures.birch_grass_tint, textures.birch_foliage_tint
    );
    Ok(())
}
