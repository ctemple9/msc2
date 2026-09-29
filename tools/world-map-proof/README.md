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
