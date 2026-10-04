# Matching-client supplemental mesh feasibility proof

P18.14a. **Implemented tooling; owner live rendering/visual acceptance pending.**
This is one isolated NeoForge proof, with no production integration or general
mod/platform support claim. P18.15 remains paused until Cameron runs the proof
and confirms its result. P18.14's findings commit remains intact.

The exporter calls the game's actual registered baked-model and block-entity
renderers. It does not invent a replacement shape or flatten resource names.
A `VertexConsumer` collects emitted positions, normals, UVs, vertex tint and
client lightmap values; a `MultiBufferSource` identifies actual supported
render types and texture bindings. It copies the GPU texture images at capture
time, including generated/stitched atlas content. Quads become explicit
triangles, and the MSC-owned viewer consumes those exported buffers and PNGs.
This is a supplemental format; the Vantage/Bedrock format and production viewer
are untouched.

## Selected actual fixtures

Available local input: the ATM10 Prism instance under
`~/.local/share/PrismLauncher/instances/ATM10`. Only the two explicitly pinned
mod JARs are read/copied. No accounts, authentication tokens, logs, configs,
resource packs or worlds are copied from that instance. NeoForge/Minecraft
build artifacts come from the development toolchain cache. `pins.json` records
exact versions, sizes and SHA-256 values; these are local byte identities, not
publisher signatures.

| Input | Exact pin |
|---|---|
| Minecraft | 1.21.1, client SHA-256 `499f6897d1837516680f3114072d8106e11c9adcd933fe5cf051b551089b0c99` |
| NeoForge | 21.1.251, universal JAR SHA-256 `6d2e128d8901db0522c9766a91911ea019a9e92ab1c8bc7fa20eb2bcb8ffe170` |
| Supplementaries | 1.21.1-3.9.9, SHA-256 `d6876ad959d0da4bab0ba182839a0af0e38ef3571345e5e7aea0be572f958e05` |
| Moonlight | 1.21.1-3.6.8, SHA-256 `e5a220558a2bcef67a2078b29a75030486484164d78bd2e011b007becfecb662` |
| Build | ModDevGradle 2.0.141, Gradle 9.2.1, JDK 21 toolchain |
| Viewer | Repository's Three.js 0.180.0; no existing proof viewer changed |

Chosen custom-loader block: Supplementaries barnacles at **9,65,3**. The actual
JAR's `models/block/barnacles.json` uses `supplementaries:random_rotation`, with
nested geometry that the ordinary element resolver cannot read directly. The
export records the loaded model class, loader and resource hash, and must
produce nonempty geometry. Both faces, their attached orientation, texture
alpha and placement must be compared with Minecraft.

Chosen contextual block: Supplementaries pedestals at **3,65,3** and **6,65,3**.
They have identical block states, but contain a diamond and an emerald. Source
inspection of the pinned mod's renderer and Moonlight's display container
shows that the renderer obtains its item from the block entity. The exporter
invokes that registered renderer with each actual client entity. Validation
requires equal states, the expected item IDs, nonempty entity vertices and
different emitted appearance after removing position translation. Cameron must
still confirm the two displayed items and pedestal geometry visually.

Preparation creates a new minimal client; the fixture is generated in a new,
explicitly named Creative Superflat world. This avoids needing a private saved
world or running the full ATM10 pack just to prove these rendering paths. That
choice does not waive the later mixed-pack/custom-dimension acceptance rows.

## Exact owner-run command

From `/home/camerontemple/msc2`:

```bash
python3 tools/world-map-proof/client-capture/proof.py run --workspace /home/camerontemple/.cache/msc-map-client-proof
```

This existing command prepares exact input bytes, builds the mod, checks the
prepared launch directory, starts the loopback viewer at
`http://127.0.0.1:8766`, and **launches the isolated Minecraft development
client**. It never launches an existing Prism instance or imports a save. The
dev client uses an offline development player; no launcher login/account is
read. The tools have been built/prepared here; Minecraft was not launched by
the agent. Initial asset preparation downloaded about 786 MiB into Gradle's
cache; subsequent unchanged builds reuse it. This is not a release workflow.

In the new client:

1. Create a **new Creative Superflat** world named exactly **MSC Mesh Capture
   Proof**, with commands enabled. Keep its default resource packs.
2. Use `/tp @s 6 67 11` and wait for chunk 0,0 to load. Then run
   `/mscproof setup`. It builds only the named isolated fixture, fills the
   pedestals, saves and flushes the new world, freezes simulation and records the
   snapshot/context binding. It refuses other client roots, multiplayer,
   other world names or an unloaded fixture chunk.
3. Wait until the two displayed items and barnacles are visible, then use
   `/mscproof capture`. This checks that client/server fixture context and resources
   still match setup, flushes a fresh save and binds its snapshot, then captures
   after the rendered level frame. A
   client update that does not match the saved context is refused; after
   waiting for updates, request capture again. If you edited the scene or
   changed simulation/resources, rerun setup rather than bypassing the guard.
4. In the proof viewer, click **Load capture**. Orbit/pan/zoom and compare the
   named appearances against Minecraft. Inspect the material/geometry and
   refusal details. Mark the three confirmation boxes **only if** the
   appearances match and the saved-frame/refusal evidence is satisfactory;
   **Save confirmed result** collects the viewer image and your confirmations.
   Export/validation alone leaves visual acceptance pending.

The viewer remains up while this command's Minecraft process runs. Close
Minecraft to finish. To inspect an existing export without starting Minecraft:

```bash
python3 tools/world-map-proof/client-capture/proof.py view --workspace /home/camerontemple/.cache/msc-map-client-proof
```

If required input bytes are absent elsewhere, `--source-mods DIRECTORY` accepts
only the exact hashes in `pins.json`. `--gradle /path/to/gradle-9.2.1/bin/gradle`
selects the pinned installed build tool. The viewer uses the repo's installed
desktop dependencies; no third-party game asset is committed or served from a
public address.

## Build and inspect without running the proof

```bash
python3 tools/world-map-proof/client-capture/proof.py build --workspace /home/camerontemple/.cache/msc-map-client-proof
```

This runs only `jar`, `prepareClientRun` and `writeProofLaunchReceipt`. The last
one checks the launch task's `gameDirectory` property, which ModDevGradle applies
when it executes; it does not execute that task. Builds/syntax inspection are
not proof of runtime rendering, refusal behavior or visual matching. No tests
are added or run by this substep. The live negative requests belong to the
owner-run capture, not an automatically executed test suite.

## Binding, limits and evidence

The server fixture freezes simulation and flushes its save before taking a
SHA-256 receipt of `level.dat` and saved region files. Minecraft can still save
on pause or autosave while simulation is frozen. Therefore the explicit capture
request checks the unchanged setup context/resources on client and server, then
flushes and fingerprints a fresh snapshot for that capture. Snapshot mismatch
checks remain active before and after meshing; setup-time metadata rewrites do
not silently bypass them. The context hash covers
states and canonical client-visible block-entity update data in the named
2..10 / Y64..67 / Z2..5 area plus a one-block halo. Client capture must match that
context and the on-disk snapshot both before and after meshing. Input/config
hashes and selected resource pack IDs bind the resource context; manually
selected optional packs are refused for this first proof. This is an isolated
integrated-server comparison, not a mechanism to bind an arbitrary remote
client to a production snapshot.

Every capture actually submits three invalid requests through the same guard:

- An unloaded chunk at 1,000,000 / 1,000,000 must return `unloaded_chunk`, without
  requesting terrain generation or producing an empty success tile. Chunk guards
  use the actual chunk source with `ChunkStatus.FULL` and loading disabled;
  `ClientLevel.hasChunk()` is unsuitable because it always returns true.
- A wrong snapshot hash must return `snapshot_mismatch`.
- A wrong context hash must return `context_mismatch`.

All three results are required in the validated export. A missing refusal
aborts publication. This demonstrates these specific invalid requests when
Cameron runs it; it is not a broad security or concurrency proof.

Animation is **`saved_frame`** with client tick, server snapshot tick, capture
time and partial tick **0**. The GPU atlas is copied at that frame, and the
viewer does not replay texture/block-entity animation. A snapshot-dependent
item's client-visible contents must match the saved context. Unloaded context
is not guessed. Arbitrary client-only secrets or unavailable entity contents
are not reconstructed.

Initial prototype bounds: 200,000 vertices; 144 inspected objects; 128 mesh
files; 64 material records; 16 texture images; 8,192 pixels per image axis;
256 MiB aggregate decoded texture/export budget; 32 MiB per validation artifact;
128 snapshot files/128 MiB snapshot bytes; 1,024 config entries/16 MiB config
bytes. Native downloads remain owned/bounded by Gradle's game preparation.
PNG validation checks chunk CRCs, dimensions and bounded decompression; mesh
validation checks finite attributes, indices, bounds, exact inputs and binding
format. Linked proof paths are refused. No compressed bundle importer exists
in this prototype.

Each successful export is a new immutable `exports/capture-<uuid>/` directory;
`exports/current` switches only after complete capture and guard rechecks.
Failed candidates are deleted; prior captures remain. The viewer builds a
candidate scene and retains the visible one on load failure; artifact URLs
name an immutable capture so a new pointer cannot mix images/geometry. The
HTTP server serves only explicitly allowed viewer/vendor/export artifacts,
binds to loopback and checks origin and capture digest on confirmation writes.
The private workspace contains:

- `inputs.json`, `build-receipt.json`, `launch-receipt.json`: source/mod/game,
  prototype binary and launch preparation receipts.
- `exports/capture-…/capture.json`, `mesh-*.json`, `texture-*.png` and
  `minecraft-frame.png`: frame, geometry/materials, exact artifact hashes and
  runtime refusal evidence. No raw world/config/launcher secrets are exported.
- `evidence/capture-….json` and `…-viewer.png`: owner confirmations and viewer
  screenshot, bound to the capture digest. These are created only by the
  explicit viewer confirmation action.

The separate `validate` command writes a `validation.json` receipt and returns
nonzero for invalid exports:

```bash
python3 tools/world-map-proof/client-capture/proof.py validate --workspace /home/camerontemple/.cache/msc-map-client-proof
```

No export exists before the owner run, so this command has not been run against
invented/synthetic evidence and no runtime pass is recorded.

## Honest limits

This first path supports the selected standard opaque/cutout/translucent render
states, emitted quads/triangles, tint/light and captured standard directional
lighting. Custom render states/shaders, overlay effects and other primitive
modes are refused rather than approximated. World fog, particles and other
scene effects are not included in this bounded block-geometry proof; correct
named geometry, textures, material transparency and contents remain required.
No Fabric/Forge adapter, headless importer, remote context correspondence,
production renderer adoption, custom-dimension proof or platform matrix is
implemented here. Visual acceptance can reveal a needed correction even after
a clean build; P18.15 stays paused until that acceptance is recorded.
