# P18.2 — offline BDS terrain proof

This isolated tool reads an **offline copy** of a BDS LevelDB world and writes
one 4×4-chunk Vantage tile plus a texture array. The Vantage Three.js engine
opens it in a local proof viewer. It does not touch the installed server save,
and its generated files stay outside Git.

The tool pins the MIT/Apache-2.0
[`bedrock-world`](https://github.com/BE-Community-Dev/bedrock-world) chunk reader
and [`bedrock-block-model`](https://github.com/BE-Community-Dev/bedrock-block-model)
shape and texture resolver. The viewer pins the MIT
[`@thoughts-on-things/vantage-mc`](https://github.com/thoughts-on-things/vantage-mc)
package. Minecraft image assets are loaded from a user-supplied Bedrock resource
pack at export time and are never checked into this repository.

## Current local proof

The private world copy is `/private/tmp/msc-bds-world-proof`, made from
`~/msc2-servers/bedrock/juice/worlds/Juice!` while BDS was stopped. The resource
pack is `/private/tmp/msc-bedrock-samples/resource_pack`, fetched from Mojang's
[`bedrock-samples`](https://github.com/Mojang/bedrock-samples). The installed BDS
`resource_packs/vanilla` has block metadata but no PNG/TGA terrain images, so
the sample pack is needed for this textured proof. The generated tile is in
`/private/tmp/msc-world-map-output`.

To regenerate after temporary files are cleared, make a consistent copy while
BDS is stopped, obtain a Bedrock resource pack containing
`textures/terrain_texture.json` and block images, then run:

```sh
cd /Users/camerontemple/msc2-world-map
cargo run --release --manifest-path tools/world-map-proof/Cargo.toml -- \
  /path/to/offline-world-copy /path/to/resource_pack \
  /path/to/private-output-directory
```

To view the existing artifact:

```sh
cd /Users/camerontemple/msc2-world-map/tools/world-map-proof/viewer
MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-world-map-output npm run dev
```

Open the localhost address Vite prints. Drag to orbit, scroll to zoom, and
inspect terrain, foliage and water. This is a standalone technical viewer, not
the final MSC in-window flow.

### Mature BDS base proof (P18.2c)

Cameron's transferred MSC1 BDS world is copied read-only to
`/private/tmp/msc-bds-base-proof/world`. The copy stays outside Git. Its base at
`(-50, 87, 65)` is inside the selected 4×4 chunks beginning at `(-6, 2)`.
The generated files are `/private/tmp/msc-bds-base-proof/output`.

```sh
cd /Users/camerontemple/msc2-world-map/tools/world-map-proof/viewer
MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-base-proof/output npm run dev
```

To regenerate from the offline copy, run:

```sh
cd /Users/camerontemple/msc2-world-map
cargo run --release --manifest-path tools/world-map-proof/Cargo.toml -- \
  /private/tmp/msc-bds-base-proof/world \
  /private/tmp/msc-bedrock-samples/resource_pack \
  /private/tmp/msc-bds-base-proof/output -6,2
```

This area contains 786 placed stairs and 124 glass panes. The exporter now
converts the block resolver's per-face UV coordinates to the tile's face corner
order and bottom-origin texture coordinates. Cameron's first screenshots of
this area showed vertical grass-side fringe and sideways wood before that
conversion; reload the regenerated tile to inspect the correction.
`glass.png` has transparent and opaque texels, which the Vantage
terrain shader cuts out; pane geometry and visual correctness still require
Cameron's inspection. There are no full glass blocks here, so this cannot
establish all glass behavior.

P18.2c's first visual pass found wrong-facing stairs, disconnected fences,
crossed panes and lantern texture artifacts. P18.2d regenerates the same tile
after interpreting the saved stair direction, deriving fence/pane neighbor
connections, and assigning regions of the lantern atlas to its body, cap and
hook. Cameron confirmed the stair, fence and lantern changes, then found that
the connected pane cuboids sampled opaque parts of the glass texture. P18.2e
uses flat, two-sided panes along the connected axis with the full transparent
glass texture. Cameron's screenshots then showed one pane direction still
opaque: Bedrock's east pane face resolves to its narrow edge texture. P18.2f
resolves both flat pane directions through the broad transparent face. Cameron
confirmed both directions now show clear centers. The lantern treatment
remains a bounded approximation of its first texture frame.

### Gold farm full glass proof (P18.2g)

The same private offline world copy contains Cameron's gold farm near
`(-11, 113, -53)`. Its complete 4×4 tile starts at chunk `(-3, -5)` and is
generated outside Git at `/private/tmp/msc-bds-gold-proof/output`:

```sh
cd /Users/camerontemple/msc2-world-map
tools/world-map-proof/target/release/msc-world-map-proof \
  /private/tmp/msc-bds-base-proof/world \
  /private/tmp/msc-bedrock-samples/resource_pack \
  /private/tmp/msc-bds-gold-proof/output -3,-5
```

To inspect the full glass blocks, run the viewer with
`MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-gold-proof/output`. This tile
contains 208 full glass blocks. Its 42 texture fallback faces are all from
`minecraft:sticky_piston_arm_collision`, not glass. Cameron confirmed the full
glass looks good; this area does not prove stained glass or distinct
biome climate tint.

### Ice mountain biome tint proof (P18.2h)

Cameron's ice mountain at `(-516, 188, -225)` falls inside a complete 4×4
tile starting at chunk `(-35, -17)`. Its private generated files are at
`/private/tmp/msc-bds-ice-proof/output`:

```sh
cd /Users/camerontemple/msc2-world-map
tools/world-map-proof/target/release/msc-world-map-proof \
  /private/tmp/msc-bds-base-proof/world \
  /private/tmp/msc-bedrock-samples/resource_pack \
  /private/tmp/msc-bds-ice-proof/output -35,-17
```

Run the viewer with
`MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-ice-proof/output`.
The local Bedrock biome definitions give frozen peaks temperature `-0.7`,
grove `-0.2`, and stony peaks `1.0`. Their downfall values are `0.9`, `0.8`,
and `0.3`. This bounded proof provisionally maps biome IDs `183`, `185`, and
`189` to those names, then samples the supplied grass and foliage colormaps.
Frozen temperatures clamp to `0` for colormap lookup, so frozen peaks and
grove yield the same cold grass tint here. IDs `4`, `188`, and the unresolved
`4294967295` remain on representative tints. Biome ID mapping varies with
Bedrock versions. Cameron confirmed the visible mountain result.

### Block-height biome sampling (P18.2i)

The exporter now reads each chunk's decoded 3D biome storages once and
applies the biome ID at each rendered block's Y position. Legacy 2D biome
data is repeated vertically. The ice mountain's surface ID counts remain
the same, while block-height sampling finds ID `190` deep in 324,336 non-air
blocks and ID `188` in 398,332. The private regenerated tile is at
`/private/tmp/msc-bds-ice-3d-biome-proof/output`; run the viewer with
`MSC_WORLD_MAP_PROOF_OUTPUT` set to that directory. Its `summary.txt` also
records `lastOpenedWithVersion` `[1,26,31,1,0]` and storage version `10`.
Neither field resolves a numeric biome ID to a name. The bounded color
mapping remains provisional until a version-aware registry is established.

Surface biome IDs `27` and `155` occur in 1,922 and 2,174 columns. This
bounded proof assumes their legacy mapping to birch forest and its mutated
variant; general version-specific ID mapping is still unproved. The local
Bedrock biome definitions give both temperature and downfall `0.6`, so both
sample the supplied grass and foliage colormaps at the same colors. The
result demonstrates reading the saved IDs and applying their climate-based
tint to the generated geometry; this area cannot prove a visible boundary
between different climate colors. Other biome IDs still use the earlier
representative tint. That earlier export sampled each column's surface
height; P18.2i samples each rendered block's height. General version-aware
biome ID mapping remains future work.

## What this proves and what it leaves open

The selected complete 4×4 area begins at chunk `(-3, 2)` near the save's
spawn. This save yields real terrain, foliage and water states. P18.2b exports
all blocks in the bounded area and emits their exposed faces: 67,266 solid
faces, 1,596 water faces, 53 texture layers, 25 log blocks, no texture
fallback faces and no missing-shape blocks. It flips decoded texture rows
before writing Vantage's WebGL texture array, correcting the grass-side fringe
that Cameron saw below the dirt and the upside-looking plant artwork. These
counts show that a Bedrock chunk reader can feed the Vantage format and
renderer with supplied Bedrock assets; Cameron's visual check of this revision
is pending. The mesher conservatively culls faces next to full opaque blocks
and between neighboring leaves or water; its behavior for complex shapes and
waterlogged blocks is not yet established. P18.2a reads grass/foliage
colormaps from the supplied pack at a **representative temperate climate**;
actual biome ID-to-climate mapping is still needed before this can be called
biome-correct. Block face UV/material slots are not yet fully mapped. Stairs
and glass do not occur in this 4×4 area, so
their shape, transparency and texture behavior need a separate representative
Bedrock fixture before any coverage claim. The tool currently emits fallback
checker texture for missing assets and counts affected faces.

No running-world snapshot, live terrain refresh, authenticated MSC route,
in-window Worlds navigation or player movement is present. If Cameron's visual
check passes, the next proof is a representative area containing stairs and
glass plus actual biome tint, followed by safe running-world reads. The later
user-facing slice brings the viewer into MSC
with the Vantage-style bottom toolbar and biome panel Cameron requested.
Repeated reads of this offline copy varied by three buried grass-block counts;
the exported face counts stayed stable. Resolve that reader inconsistency
before using counts or changes as running-world evidence.

For Cameron's visual verification, reload the viewer and orbit above and below
the same area. Check that the grass-side fringe sits above the dirt, plants
stand upright, logs appear under leaf canopies, and the large sky holes are
filled by real lower terrain. `summary.txt` in the private output reports
geometry counts. The local export took 2.24 seconds and peaked at 57 MB RSS;
first-visible viewer time and viewer CPU/memory use are still unmeasured.
