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
spawn. This save yields real terrain, foliage and water states; the first export
produced 8,971 solid faces, 610 water faces, 16 texture layers, no texture
fallback faces and no missing-shape blocks. This proves a Bedrock chunk reader
can feed the Vantage format and renderer with supplied Bedrock assets. It does
**not** prove a finished Bedrock geometry path: this mesher retains only the top
visible block per X/Z column, so caves, overhangs, structures under canopy,
exact neighbor culling, exact biome tint and much water volume geometry are
absent. The grey grass in Cameron's first visual check came from writing white
vertex tint for Bedrock's greyscale grass texture. P18.2a reads grass/foliage
colormaps from the supplied pack at a **representative temperate climate**;
actual biome ID-to-climate mapping is still needed before this can be called
biome-correct. It also omits buried/down surface faces and adds water sides.
Block face UV/material slots
are not yet fully mapped. Stairs and glass do not occur in this 4×4 area, so
their shape, transparency and texture behavior need a separate representative
Bedrock fixture before any coverage claim. The tool currently emits fallback
checker texture for missing assets and counts affected faces.

No running-world snapshot, live terrain refresh, authenticated MSC route,
in-window Worlds navigation or player movement is present. The next proof is
safe reads and refreshes from a running BDS save, alongside a shape fixture
for stairs and glass; the later user-facing slice brings the viewer into MSC
with the Vantage-style bottom toolbar and biome panel Cameron requested.
Repeated reads of this offline copy varied by three buried grass-block counts;
the exported face counts stayed stable. Resolve that reader inconsistency
before using counts or changes as running-world evidence.

For Cameron's visual verification, record whether the tile loads and camera
movement works, whether land and water look plausible, and any holes or
wrong-looking blocks. `summary.txt` in the private output reports geometry
counts. First-visible time, CPU use and memory use are still unmeasured.
