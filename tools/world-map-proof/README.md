# P18.2 — offline BDS terrain proof

## Fabric live player feed proof (P18.3m)

This temporary Fabric mod targets the managed **Fabric 26.2 / Loader 0.19.5**
server. It emits one complete roster to the server console every 20 server
ticks, including an empty roster when nobody is online. Samples contain the
same sequence, tick, sample time, UUID, name, dimension, XYZ, yaw, and pitch
fields as the Paper and KubeJS probes. Player names and coordinates are
private; do not post watcher output publicly.

The mod uses Fabric API's end-of-server-tick event. If the selected server
doesn't already have a Fabric API build for Minecraft 26.2, the installer adds
the official Fabric API `0.161.0+26.2` aggregate jar and records its checksum
so removal can delete only that exact proof dependency. Existing mods and
world data are not edited.

Stop **Fabric** in MSC before installing:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/install_fabric_player_feed.py \
  --server-dir "$HOME/Library/Application Support/MSC 2/servers/java/fabric" \
  --server-stopped
```

Start Fabric through MSC. From another terminal, watch its authenticated MSC
console:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/watch_java_player_feed.py --server Fabric
```

Join with a **Minecraft 26.2** Java client, walk and turn, and confirm fresh
samples track XYZ and yaw/pitch. If convenient, change dimension and confirm
that field changes too. Disconnect and confirm the next sample has
`"players": []`. The watcher reports `fresh: false` if samples stop. This
proves the selected Fabric Loader/API combination and the shared console
sample format; it does not prove another Fabric version or Java terrain
rendering.

After the proof, stop Fabric in MSC and remove the temporary files:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/remove_fabric_player_feed.py \
  --server-dir "$HOME/Library/Application Support/MSC 2/servers/java/fabric" \
  --server-stopped
```

The remover refuses to delete changed files. It removes the Fabric API jar
only when this installer added it; pre-existing Fabric API and other mods are
left in place.

## Forge live player feed proof (P18.3n)

This temporary Forge mod targets the managed **Minecraft 26.3 / Forge 66.0.8**
server. The installer compiles it against that server's own Forge and Minecraft
libraries with Java 25, then adds one jar to `mods/`. It emits a complete
roster every 20 server ticks using the same `MSC_MAP_PLAYERS_V1` envelope read
by the authenticated Java watcher, including UUID, name, dimension, XYZ,
yaw/pitch, tick, and sample time. Empty rosters are emitted after disconnect.
Do not post watcher output publicly; names and coordinates are private.

Stop **Forge** in MSC before installing:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/install_forge_player_feed.py \
  --server-dir "$HOME/Library/Application Support/MSC 2/servers/java/forge" \
  --server-stopped
```

Start Forge through MSC. From another terminal, watch its authenticated
console:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/watch_java_player_feed.py --server Forge
```

Join with a **Minecraft 26.3** Java client, move and turn, and confirm fresh
samples track XYZ and yaw/pitch. Disconnect and confirm the next sample has
`"players": []`. Stop Forge in MSC and remove the probe:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/remove_forge_player_feed.py \
  --server-dir "$HOME/Library/Application Support/MSC 2/servers/java/forge" \
  --server-stopped
```

Both helpers require the server to be stopped, refuse to overwrite existing
files, and check that no Java process is using the selected server directory.
Removal also checks the installed jar checksum. This proves the selected Forge
build and shared sample contract; it does not prove another Forge version or
Java terrain rendering.

## Vanilla live player feed proof (P18.3o)

Vanilla has no plugin or mod API, so this proof uses a temporary datapack and a
dedicated watcher that turns vanilla command-feedback records into the shared
player-sample envelope. It targets the managed **official Minecraft 26.3**
server and pack format 121. The datapack reads player UUID, position, and
rotation once per second across the Overworld, Nether, and End. It does not
broadcast the values to in-game chat or change server properties or terrain.
The watcher needs access to MSC's authenticated server console. Player names
and coordinates are private; do not post output publicly.

If the managed Vanilla server has never been started, start it once in MSC so
its selected `Vanilla/` world directory is created, then stop it. With Vanilla
stopped, install the datapack:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/install_vanilla_player_feed.py \
  --server-dir "$HOME/Library/Application Support/MSC 2/servers/java/vanilla" \
  --server-stopped
```

Start Vanilla through MSC. In another terminal, watch the authenticated
console feedback:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/watch_vanilla_player_feed.py --server Vanilla
```

Join with a **Minecraft 26.3** Java client, move and turn, and confirm fresh
samples track XYZ and yaw/pitch. Disconnect and confirm an empty `players`
array. This route depends on Vanilla exposing datapack command feedback in
MSC's console with the expected English text; confirm that live before relying
on it. Stop Vanilla in MSC and remove the datapack:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/remove_vanilla_player_feed.py \
  --server-dir "$HOME/Library/Application Support/MSC 2/servers/java/vanilla" \
  --server-stopped
```

The helpers refuse to run while a Java process uses the selected server.
Removal checks both the recorded install hash and the current source pack hash,
and removes only this proof datapack and its install record. This proves the
selected Vanilla version and console-feedback route; it does not prove another
version or Java terrain rendering.

## Paper live player feed proof (P18.3l)

This temporary plugin targets the managed **Paper 26.2 build 121** server. The
installer compiles it against that server's own Paper API jar using Java 25,
then adds only `plugins/msc-map-player-feed-proof.jar`. It writes one complete
roster to Paper's local console every 20 ticks, including an empty roster when
nobody is online. The sample fields match the ATM10 Lite probe: `sequence`,
`tick`, `sampledAtMs`, and each player's UUID, name, dimension, XYZ, yaw, and
pitch. Names and coordinates are private; do not publish watcher output.

Stop **Paper** in MSC before installing the plugin:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/install_paper_player_feed.py \
  --server-dir "$HOME/Library/Application Support/MSC 2/servers/java/paper" \
  --server-stopped
```

The installer refuses to overwrite an existing file and checks that no Java
process is using this server directory. Start Paper through MSC. From a second
terminal, watch its authenticated console:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/watch_java_player_feed.py --server Paper
```

Join with a **Minecraft 26.2** Java client, walk and turn, and confirm fresh
samples track XYZ and yaw/pitch. If convenient, change dimension and confirm
that field changes too. Disconnect and confirm the next sample has
`"players": []`. The watcher reports `fresh: false` if samples stop. This
proves the selected Paper version and the shared console sample format; it
does not prove other Paper versions or Java terrain rendering.

After the proof, stop Paper in MSC and remove the plugin:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/remove_paper_player_feed.py \
  --server-dir "$HOME/Library/Application Support/MSC 2/servers/java/paper" \
  --server-stopped
```

Removal recompiles the expected jar and refuses to delete an altered file. It
does not change the world, other plugins, or Paper's configuration. The probe
uses the [Paper plugin descriptor](https://docs.papermc.io/paper/dev/plugin-yml/)
and a [main-thread repeating task](https://docs.papermc.io/paper/dev/scheduler/).

## Modded Java live player feed proof (P18.3j)

This probe checks live positions on the managed **All The Mods 10 LITE** server
(Minecraft 1.21.1, NeoForge, KubeJS 2101.7.2). It is one separate file in
`kubejs/server_scripts`; it does not edit existing KubeJS scripts, world data,
or terrain exports. Every 20 server ticks it writes one complete roster to the
server log, including an empty roster when nobody is online. Samples contain
UUID, name, dimension, XYZ, yaw, pitch, tick, and sample time. Player names and
coordinates are private; do not post the watcher output publicly.
KubeJS lists `ServerEvents.tick`, but its detailed event page is unwritten, so
the startup and console proof also checks this event against the installed
runtime: [event list](https://kubejs.com/wiki/events),
[tick event](https://kubejs.com/wiki/events/ServerEvents/tick).

Stop **All The Mods 10 LITE** in MSC, then install the probe:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/install_java_player_feed.py \
  --server-dir "$HOME/Library/Application Support/MSC 2/servers/java/all_the_mods_10_lite" \
  --server-stopped
```

The installer refuses to overwrite the probe file and checks for a Java
process whose working directory is this server. Start the server in MSC, then
watch its authenticated MSC console from another terminal:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/watch_java_player_feed.py \
  --server "All The Mods 10 LITE"
```

Join with a Java client, walk and turn, and check that fresh samples update
XYZ and yaw/pitch about once per second. If convenient, cross a dimension
boundary and check the dimension field. Disconnect and confirm the next sample
has an empty `players` array. If no new sample arrives for five seconds, the
watcher reports `fresh: false`; it keeps one authenticated MSC console stream
open, not a connection to the server log file.

To remove the probe, stop the server in MSC and run:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/remove_java_player_feed.py \
  --server-dir "$HOME/Library/Application Support/MSC 2/servers/java/all_the_mods_10_lite" \
  --server-stopped
```

Removal checks the script still matches the installed proof file and removes
only that file. The installer and remover both refuse to run while a Java
process is using this server directory. This proof establishes only the
selected NeoForge/KubeJS combination; Paper has a separate probe above, and
this does not establish modded terrain fidelity or player rendering.

## Bedrock live player feed proof (P18.3a)

The `bedrock-player-feed` behavior pack uses stable `@minecraft/server`
functions to sample active players every 20 game ticks (normally about one
second). It logs one JSON roster with name, session-scoped entity ID,
dimension, XYZ, pitch, yaw, game tick, and wall-clock sample time. An empty
roster is an explicit sample. This pack has no client component, so the proof
can include a console-connected player. It does not require the Beta APIs
experiment, network APIs, or a client map mod. This is a **server console
proof**, not yet an authenticated MSC player-position endpoint.

To try it on the imported `theboyslatest` server, **stop that server in MSC**
first. Then run:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/install_bedrock_player_feed.py \
  --server-dir "$HOME/Library/Application Support/MSC 2/servers/bedrock/theboyslatest" \
  --server-stopped
```

The installer copies the pack into that server's `behavior_packs`, adds its
pack ID to the active world's `world_behavior_packs.json`, and preserves any
existing pack list as `.before-msc-map-proof`. It refuses duplicate installs.
Restart BDS in MSC, then in another terminal run the polling watcher. It uses
the authenticated `msc console tail` endpoint and works with the currently
installed agent:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/watch_bedrock_player_feed.py --poll \
  --server theboyslatest
```

After a development agent containing P18.3b is loaded, the streaming path can
also be checked with:

```sh
cd /Users/camerontemple/msc2-world-map
MSC2_DATA_DIR="$HOME/Library/Application Support/MSC 2" \
  target/debug/msc console follow --server theboyslatest \
  | python3 tools/world-map-proof/watch_bedrock_player_feed.py
```

The watcher prints `fresh: true` with the latest full roster; if samples stop
for five seconds while the console stream remains open, it prints
`fresh: false`. Join from a Bedrock or console client, walk and turn, change
dimension if convenient, then disconnect. Confirm XYZ/yaw change and the
player disappears from the next roster. Do not share full output publicly:
names and coordinates are private. The watcher reports sequence gaps, and a
pack error appears as `MSC_MAP_PLAYERS_ERROR` in the server log.

This proof does not yet join the script's session ID with MSC's XUID roster,
authenticate a position API, animate a 3D model, or prove Java players.
Those are separate checkpoints; the script entity ID is not assumed to be a
permanent account identifier. If the pack does not load or its warnings do not
reach MSC's console, record that failure before building the bridge.

Sources: [Microsoft's stable World API](https://learn.microsoft.com/en-us/minecraft/creator/scriptapi/minecraft/server/world?view=minecraft-bedrock-stable),
[Entity location and rotation](https://learn.microsoft.com/en-us/minecraft/creator/scriptapi/minecraft/server/entity?view=minecraft-bedrock-stable),
[BDS pack folders](https://learn.microsoft.com/en-us/minecraft/creator/documents/bedrockserver/getting-started?view=minecraft-bedrock-stable),
and [script logging](https://learn.microsoft.com/en-us/minecraft/creator/documents/scripting/debugging-scripts?view=minecraft-bedrock-stable).

## Changed tile export proof (P18.2r)

The exporter can compare two **consistent, private offline snapshots** by
their raw Bedrock terrain, biome, and subchunk records. It checks a bounded
grid of 4×4-chunk tiles and exports a tile from the newer snapshot only when
one of its 16 chunks changed. Player/entity records are excluded, so player
movement alone does not mark terrain dirty. Keep both snapshot paths from
separate `msc --json world map-snapshot` results; do not compare a live BDS
LevelDB directory. Use a new output directory for each comparison:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/export_changed_tiles.py \
  --before '/path/from/first/snapshot/result/worldPath' \
  --after '/path/from/second/snapshot/result/worldPath' \
  --resource-pack /private/tmp/msc-bedrock-samples/resource_pack \
  --output-root /private/tmp/msc-bds-changed-tiles-proof \
  --chunk-x -6 --chunk-z 2 --tiles-x 2 --tiles-z 2
```

The JSON report lists changed tile origins, output directories, comparison
time, and total ready time. With no terrain changes, `changedTiles` and
`exported` should both be empty. A block edit in the first 4×4 area should
export `tile_-6_2` alone. This proof still copies the full BDS save for each
snapshot and rebuilds an entire dirty 4×4 tile. It does not yet stream multiple
tiles into the viewer or set an automatic refresh cadence. Keep all snapshot
and generated paths outside Git.

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
The source and exact-version gap are recorded in
`docs/msc2/bedrock-biome-registry.md`. The exporter labels its current mapping
as provisional in `summary.txt`.

### Optional exact-version biome registry (P18.2k)

Pass a JSON file after the chunk origin to validate a registry before export:

```json
{
  "version": [1, 26, 31, 1, 0],
  "source": "BDS registry exporter provenance",
  "source_sha256": "64 hexadecimal characters",
  "ids": { "frozen_peaks": 183, "grove": 185, "stony_peaks": 189 }
}
```

The file must also contain the current birch forest IDs, and all IDs must
be unique. The exporter rejects a version mismatch before rendering. The
example shows the schema only; it is **not** a generated exact-version map.
The available private BedrockData `1.26.30` wrapper at
`/private/tmp/msc-biome-registry-1.26.30.json` is deliberately incompatible
with this save's `1.26.31` metadata. The exact registry still needs extraction
from the corresponding BDS executable on Linux. See
`docs/msc2/bedrock-biome-registry.md` for provenance and upgrade limits.

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

P18.2l adds a running-BDS snapshot operation, awaiting Cameron's physical
verification below. Live tile replacement, in-window Worlds navigation and
player movement are still absent. The later user-facing slice brings the viewer into MSC
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

## Running BDS snapshot proof (P18.2l)

Start a disposable Bedrock world through MSC and place or remove one easily
recognized block. Load this worktree's development agent, then run from the
worktree root:

```sh
target/debug/msc --json world map-snapshot
```

The operation waits for BDS `save query` to confirm the world is ready before
copying. Its JSON result contains `worldPath`, `bytesCopied` and `holdMillis`.
The world copy is in an owner-private directory under the host's temporary
directory, never in Git or the normal backup list. The command refuses a
world over 2 GiB or a copy that takes more than 30 seconds, and releases the
save hold on errors while the same server run is available. Do not use a
failed or incomplete copy for a map.

Pass the returned `worldPath` as the exporter's first argument, with the
resource pack, a new private output directory, and a 4×4 chunk origin that
contains the changed block. For block coordinates `(x,z)`, use an origin of
`(floor(x/16)-1,floor(z/16)-1)` to put it near the center of the 4×4 area.
Then launch the proof viewer with
`MSC_WORLD_MAP_PROOF_OUTPUT` set to that output. Confirm the change appears
and BDS continues accepting world writes. This is a *saved terrain* refresh
proof; walking players require a separate position feed. A completed snapshot
is a temporary private copy and should be removed after inspection.

### Replace saved terrain without reopening the proof viewer (P18.2n)

Keep the viewer open on the first export, with
`MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-live-proof/output`. Make a
second visible block change in BDS and capture another `msc world
map-snapshot`. Export its returned `worldPath` to a new revision directory:

```sh
tools/world-map-proof/target/release/msc-world-map-proof \
  '<new-worldPath>' \
  /private/tmp/msc-bedrock-samples/resource_pack \
  /private/tmp/msc-bds-live-proof/output/revisions/after \
  -2,-1
```

In the still-open proof viewer, enter `after` under **Saved terrain revision**
and select **Replace terrain**. Both new files load before the viewer swaps
the tile; the camera position is restored. Export to a new revision name for
each subsequent capture rather than overwriting the current tile. This is a
manual saved-terrain refresh proof, with no live players or automatic polling.

### Measure one manual refresh (P18.2o)

Keep the proof viewer open, make another visible block change near the pillar,
then run from the worktree root:

```sh
python3 tools/world-map-proof/measure_refresh.py \
  --resource-pack /private/tmp/msc-bedrock-samples/resource_pack \
  --output-root /private/tmp/msc-bds-live-proof/output \
  --chunk-x -2 --chunk-z -1
```

The command captures a new BDS snapshot and exports it to a unique revision.
It prints `snapshotMs` (CLI request through confirmed operation completion),
`holdMillis` (BDS save hold), `exportMs`, and `readyMs` (snapshot plus export).
In the still-open viewer, enter the printed `revision` and select **Replace
terrain**. Its status shows the additional browser load time. Confirm the new
block appears and BDS remains writable. These numbers measure a manual local
refresh; they exclude the time between placing the block and starting the
command, and do not establish a safe automatic polling interval by themselves.

### Repeat on the mature running BDS world (P18.2p)

After Cameron imports the mature `theboyslatest` world into MSC and starts it,
capture three samples around the developed base at `(-50, 87, 65)`. The 4×4
origin for that base is `(-6, 2)`:

```sh
cd /Users/camerontemple/msc2-world-map
python3 tools/world-map-proof/measure_refresh_series.py \
  --resource-pack /private/tmp/msc-bedrock-samples/resource_pack \
  --output-root /private/tmp/msc-bds-base-proof/output \
  --chunk-x -6 --chunk-z 2 --samples 3
```

The script waits for Enter before each capture, prints each result, then reports
minimum, median, and maximum snapshot, hold, export, and ready times. It stops
on the first failure. Each capture copies the entire selected world, subject
to the proof's 2 GiB and 30-second limits, even though the exporter renders
only 4×4 chunks. Confirm MSC still reports the server running and BDS accepts
block changes after each sample. The output measures one developed tile; it is
not a full-world streaming benchmark or an automatic refresh loop.

### Live Bedrock player overlay (P18.3c)

Build and load this worktree's development agent before using the new route.
Leave the Bedrock player behavior pack installed in the active world and start
that world through MSC. From this worktree, inspect the authenticated feed:

```sh
cargo build -p msc-agent --bin msc
MSC2_DATA_DIR="$HOME/Library/Application Support/MSC 2" \
  target/debug/msc --json world map-players
```

The result reports `fresh: true` only while the active BDS server's pack has
sent a sample in the last five seconds. It carries current XYZ, yaw, pitch,
dimension, and names. When the feed is absent or stale, `players` is empty.
The server must run under this newly built development agent; building the CLI
alone does not update an already running agent.

Open the existing private base tile in the proof viewer:

```sh
cd tools/world-map-proof/viewer
MSC2_DATA_DIR="$HOME/Library/Application Support/MSC 2" \
MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-base-proof/output npm run dev
```

The local Vite proof server calls the authorized CLI on behalf of the browser;
the browser never receives an agent token. Visit the local address Vite prints.
Keep that Vite process running: it now uses one CLI session for live updates
and renews its credential before expiry, instead of authorizing every second.
Join the Bedrock server near the exported base at `(-50, 87, 65)`, walk and
turn, and compare the 3D model with your client. The label should show one
live Bedrock player. Leave the server or stop the agent and confirm the model
disappears and the label reports an unavailable feed within roughly five
seconds. This is a single-tile development proof; it does not yet provide the
full Worlds-tab shell, roster, skins, all dimensions, or Java player feeds.

If blocks placed after the initial export are missing, the player feed is
working independently of the saved terrain. From the worktree root, create a
new base-area revision with `measure_refresh.py --resource-pack
/private/tmp/msc-bedrock-samples/resource_pack --output-root
/private/tmp/msc-bds-base-proof/output --chunk-x -6 --chunk-z 2`. Enter the
printed `revision` in the viewer and select **Replace terrain**.

The **Players** roster lists current Bedrock players in the map's dimension.
Select **Fly** to move the camera to a player. Select **Follow** to keep the
camera centered on their model as they move; select **Following** again or
begin navigating the map to release the camera. These actions use map
navigation and aim the view at the model midpoint, including when the viewer
was left in free-flight mode. While locked to a player, terrain-height
adjustment is suspended so it cannot push the target off center; ordinary
terrain camera behavior returns when the lock ends. A player who leaves or
whose feed becomes stale is removed from the roster and follow ends. This proof
roster covers the active BDS feed only; it does not establish Java players,
skins, or the final MSC Worlds-tab layout.
