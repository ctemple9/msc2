# Java map resources — renderer and repair contract

## P18.23 production capture intake — implementation, acceptance pending

The production importer now accepts `msc-contextual-mesh-1` data bundles and
the actual desktop map consumes their checked geometry/materials/textures with
a matching private terrain generation. Older partial-stage descriptions below
are historical. Helpers, guided capture repair and native/visual observations
remain P18.24–P18.26 work; this implementation alone proves no appearance.

`POST /v1/worlds/{slot_id}/map-assets/capture-request` accepts the scoped check
body. It reads an adopted non-stale scene with known exact inputs and a loader
version, plus a one-block context halo. The combined bounds must fit the existing
16-chunk/262,144-block inspection budget. Missing fully saved context refuses
issuance. A request retains its bounded scene for the 30-minute upload lifetime;
it neither starts Minecraft nor changes the snapshot.

The request binds host/server/slot/incarnation/revision, base geometry/resource
generations, complete resource-manifest fingerprint, dimension, game/loader
versions and both areas. Snapshot identity is independently recomputed from
the agent-owned level.dat and decoded chunk bytes, including entity contents.
Exporters must record independently observed snapshot/input fingerprints after
checking their actual sources before and after rendering. Attaching a request
to an unrelated historical proof is insufficient. Import compares those
identities and every original block ID/state/position with the saved snapshot.
Input correspondence is distinct from proving that submitted triangles look
correct: the MSC-owned helper and owner visual comparison remain required.

The existing authenticated map-client-assets staging/import operation accepts
a ZIP containing capture.json. Original resource ZIPs retain their old path and
limits. Supplemental imports publish no server mod or resource selection.
Bounds are 64 MiB encoded, 4,096 files/blocks, 1,024 materials, one million total
vertices, six million total indices and 16 MiB per file. PNGs must be eight-bit RGB/RGBA, at most 4,096 pixels
per side, with at most 64 MiB decoded memory per image and 128 MiB
for the complete capture texture set. Checksums, paths, case
collisions, links/special files, duplicate keys/references, unused payloads,
indices, attribute lengths, finite values, unit-range UV/colors and context-local
bounds are checked before adoption. Unknown versions and executable/custom
shader payloads are refused. Opaque/cutout/translucent/additive modes carry alpha,
culling, depth-write and captured directional-light/tint data. Every capture is
an explicitly dated saved frame. Unsupported materials remain acceptance gaps.

Only captured positions change in the renderer's private copy: palettes are
repacked across bit-width changes, neighboring faces become visible and vanilla
water inside waterlogged blocks remains in the fluid pass. Captured meshes must
exclude fluid geometry. Uncaptured instances of identical states stay intact.
The existing tile apron invalidates affected neighbors. Terrain, capture and
scoped report publish as one guarded generation; invalid/stale/cancelled
candidates retain the previous scene. Reports use captured_appearance for only
the validated positions, with visual acceptance pending.

Terrain manifests expose mscCapture and generation-bound capture/capture.json,
meshes and textures. Readers must send captureFormat=msc-contextual-mesh-1;
older readers receive capture_viewer_required rather than a scene missing its
replaced blocks. Desktop candidates validate hashes, identities, bounds and
geometry before adoption, dispose failed/retired GPU objects, share the animated
terrain cutaway plane, and label saved time/tick. Capture files currently live
with their runtime scene leases. Durable exporter output reuse and complete
helper delivery remain explicit P18.24 work; historical diagnostics alone never
restore a captured scene after restart.

**P18.20 partial update, 2026-10-05:** Public Forge input selection and isolated
draft sources are recorded under `tools/java-map-export/forge-1.20.1/`.
The initial compilation failed on Forge-specific access rules; the authorized
SRG correction now compiles/packages successfully. Capture verification is
pending. Cameron authorized P18.21 evidence preparation; no
production supplemental-format negotiation, snapshot correspondence,
import/adoption or difficult-model remedy is implemented by this update.
See the helper README and acceptance record for exact pins/failure. The existing
NeoForge visual proof remains scoped to its original isolated fixture.

P18.14 findings · 2026-10-04 · **Original disposition: feasibility blocked; production unchanged.**

**Current update:** Cameron confirmed the isolated matching-client proof. P18.15 resource inspection is implemented, awaiting owner verification. The original findings below are historical; see the P18.15 implementation record at the end. Production supplemental geometry and broader acceptance remain open.

This is the engineering result of Batch A's first step, including its stop
rule. It does not establish successful modded rendering. P18.15 has not begun.
The required code-rendered/contextual example has no demonstrated export or
adapter path. See [acceptance](world-map-assets-acceptance.md) for the missing
inputs and proof needed to remove that blocker. No supported requirement is
waived, and the Phase 18/release acceptance records remain separate.

## Inspected implementation

Vantage v0.15.1 resolves to upstream commit
`953f0ac78d62de751a935b8e8d5be2d711d6fd56`. The source was inspected at that
commit, without running its tests or launching Minecraft:

- [`src/model.zig`](https://github.com/thoughts-on-things/vantage-mc/blob/953f0ac78d62de751a935b8e8d5be2d711d6fd56/src/model.zig):
  `resolveBlockUncached` strips the block namespace, `gather` strips parent
  namespaces, and `resolveTexture` strips final texture namespaces. The root
  is one `assets/minecraft` directory. Variants, multipart conditions, model
  parents, texture variables, element rotations, cull faces and tint indices
  are represented. Weighted variant arrays select their first entry.
- [`src/mesh.zig`](https://github.com/thoughts-on-things/vantage-mc/blob/953f0ac78d62de751a935b8e8d5be2d711d6fd56/src/mesh.zig):
  `bakeModel` uses name/state-based cached shapes. Resolver failures can create
  opaque fallback cubes, which can hide neighboring faces. Empty models can
  enter hardcoded special-block handling. That handling is a static
  approximation, not execution of an arbitrary mod's block-entity renderer.
- [`src/grid.zig`](https://github.com/thoughts-on-things/vantage-mc/blob/953f0ac78d62de751a935b8e8d5be2d711d6fd56/src/grid.zig):
  the interner keys geometry inputs by block name/state. That key cannot select
  two different appearances for identical states with different container
  contents, neighbors or client-only context.
- [`src/tile.zig`](https://github.com/thoughts-on-things/vantage-mc/blob/953f0ac78d62de751a935b8e8d5be2d711d6fd56/src/tile.zig),
  [`web/src/core/tile.ts`](https://github.com/thoughts-on-things/vantage-mc/blob/953f0ac78d62de751a935b8e8d5be2d711d6fd56/web/src/core/tile.ts)
  and [`web/src/three/materials.ts`](https://github.com/thoughts-on-things/vantage-mc/blob/953f0ac78d62de751a935b8e8d5be2d711d6fd56/web/src/three/materials.ts):
  packed terrain has positions, UVs, normals, texture layers, tint, saved light,
  biome/surface information and solid/fluid passes. Compact formats derive
  canonical quad indices. They do not carry arbitrary index topology,
  per-draw mod render types/shaders, or a block-entity context manifest.

The repository pins the same viewer package version and official helper
archive digests in `tools/release/vantage-release.json`. A changed executable
must have its own identity and digests. No official digest may authenticate a
patched build.

Local evidence: `stage_java_mod_assets.py` rewrites/merges resources for a
private ATM10 Lite proof; the README explicitly records a placeholder for
`lootr:lootr_chest`. `audit_java_terrain_assets.py`'s historical `exact` category
means files/model elements resolved. It does not prove visual correctness.
Historical reports and script behavior remain intact. New reports must use
`model_resolved`, with a separate visual acceptance field.

The production Java compatibility helper returns the original world for a
custom dimension. Its modern-layout normalization covers only the three
standard dimensions. The existing terrain bridge owns renderer/snapshot
mutexes; new preparation must run outside them and adopt a complete compatible
generation. Neither behavior changes in this step.

## Ordinary-resource route

Choose a namespace-aware private extension of the pinned resolver, rather
than flattening identifiers or changing the game save. Its root becomes a
composed `assets` tree. Resource keys are `(kind, namespace, path)` and paths
are validated Minecraft identifiers. An omitted namespace defaults to
`minecraft`; it must not be inferred from a filesystem directory. Preserve
original IDs through all reporting and palette handling.

Example: `alpha:block/panel` and `beta:block/panel` select
`assets/alpha/models/block/panel.json` and
`assets/beta/models/block/panel.json`. A parent `beta:block/base` retains beta's
identity; a binding `"all": "alpha:block/panel"` retains alpha's identity;
`#all` remains a texture variable. Resource selection resolves the ordered
stack first, then parent/texture dependencies. Unknown mod JSON fields are
preserved and classified; no blanket recursive string replacement is allowed.
This represents collisions and dependency chains by construction, but no
patched-renderer runtime proof is recorded yet.

The extension must also bound reads/references, distinguish custom `loader`
fields from intentional empty models, retain fallback diagnostics and prevent
unknown fallback geometry from occluding neighbors. Weighted selection and
mod-specific tint/material semantics need explicit support classification;
namespace resolution alone does not make them visually correct.

Build direction: keep baseline Vantage unchanged for no-mod/no-custom-pack
worlds and Bedrock. Pin the extension's source revision, patch digest, Zig
0.16 toolchain and build options. Produce native Windows x86_64, Linux x86_64
and macOS x86_64/aarch64 helpers with distinct SHA-256 receipts and version
output. Stage through the existing build-only packaging path. Reproducibility
requires recording exact compiler distribution hashes and comparing outputs;
these hashes/build results are not available and must not be invented here.

## Difficult-model blocker and alternatives

NeoForge 1.21.1's [custom-loader contract](https://docs.neoforged.net/docs/1.21.1/resources/client/models/modelloaders/)
allows a loader to ignore standard element fields. Its
[block-entity renderer contract](https://docs.neoforged.net/docs/1.21.1/blockentities/ber/)
passes the entity, partial tick, pose stack, vertex buffers, light and overlay
on the physical client. Therefore interpreting ordinary JSON cannot recover
all required appearances. This is an inference from those contracts and the
inspected Vantage input/format, not a failed live exporter experiment.

**Failing required class:** two blocks with the same palette state but different
visible contents/context cannot be faithfully selected by Vantage's existing
name/state cache. A custom loader producing non-cuboid geometry or a block
entity using another render type has no general ingestion path. The historical
Lootr placeholder is a concrete unresolved example, not proof of a remedy.

| Alternative | Compatibility and cost | Evidence still required |
|---|---|---|
| Matching-client capture plus MSC-owned supplemental mesh | Recommended investigation. Keep baseline tiles/decoder; add a separately negotiated mesh/material sidecar. Requires loader/version-specific client code and viewer ingestion. Headless host only imports data. | Capture a real custom loader and a real contextual block entity, transfer only bounded output, then show both in MSC. |
| Explicit internal adapter for a pinned mod | Smaller initial implementation for known behavior; manually reproduces geometry, material and context semantics. Must refuse unrecognized versions. Does not establish general mod coverage. | Read exact mod source, identify all required saved/client context, implement it privately and compare the named appearance. |
| Replace the terrain pipeline with a client-render export | Larger renderer/packaging change and greater baseline regression risk; still requires matching-client context and headless import. | Demonstrate export completeness and baseline navigation/material performance before choosing it. |

Proposed minimal supplemental format: a versioned manifest binding snapshot,
dimension, area, client game/loader/mod/pack/config hashes and capture time to
content-addressed buffers/images. Meshes carry float positions/UVs/normals,
explicit triangle indices, baked vertex tint/light, bounds and material IDs.
Materials declare opaque/cutout/translucent/additive mode, alpha threshold,
culling, depth-write and emissive behavior. Arbitrary shaders are not
transferable as trusted executable code; unrepresentable shader output remains
a blocker for its acceptance case. Unknown sidecar versions are refused while
the existing map remains visible. Baseline VTL decoding remains unchanged.

Client capture would intercept supported baked-model and block-entity buffer
output in the user's explicitly launched matching client. Fabric, Forge and
NeoForge need separately pinned adapters; NeoForge's documented method is not
evidence that the other two hooks work. Capture actual client-visible entity
data and a neighbor halo with hashes. Do not assume private inventories are
sent to every client. Do not present capture of current online state as a
capture of an older server snapshot. If correspondence cannot be established,
require a matching isolated snapshot/client context or refuse that area.

Unvisited/unloaded chunks are pending, never air or successful captures. Capture
must not generate missing terrain silently. Exported animation is one named
frame/time; it is a saved appearance, not live animation. Missing context,
shader behavior or animation phase cannot be hidden under `model_resolved`.
No passive scan starts mod code, a launcher or a login session on the agent.

**Stop disposition:** neither client capture nor a pinned contextual adapter
exists in the inspected implementation. A source-level entry point is not a
working export path. P18.14's feasibility gate remains blocked and P18.15 is
not implemented. There is no honest live-proof command for a nonexistent
exporter. The next engineering work is an isolated capture/adapter prototype
under `tools/world-map-proof/`, followed by a specific owner-run proof command;
it must precede the production foundation and acquisition/UI batches. Existing
`meshtex`/asset-audit commands cannot close this blocker.

## Frozen application contract, conditional on feasibility

These names and semantics are the implementation contract for later steps;
they are not currently available API routes, capabilities or CLI commands.
The supplemental capture schema remains provisional until the proof succeeds.

Binding: `(agent_host_id, server_id, slot_id, world_incarnation)`; restored or
replaced slot contents change the incarnation even if the slot ID survives.
Dimension/area belongs to geometry/report context. Display names and absolute
paths are never identities. The authenticated agent supplies host identity and
resolves approved roots; clients cannot select another host through a body.

Source evidence carries content SHA-256/size, game and loader versions,
declared mod IDs/versions, provider project/release/file IDs and published
digests when known, manifest/override receipts, namespace contributions and
evidence kind (`provider_verified`, `local_hashed`, `unknown`). Local hashing
is not publisher authentication. A namespace is not a unique mod identity.

Resource generation ID is SHA-256 of canonical schema-versioned inputs:
ordered source/content digests, selected pack order, applicable overlays,
filters/configuration, game/loader versions, adapter/renderer/format versions.
Geometry generation additionally binds snapshot, dimension, area/neighbor
context and resource generation. Imported advanced capture adds its context
and capture digest. Changing any dependency invalidates its reverse dependents;
unrelated reusable content remains cached. Server updates, restores, slot
replacement or unknown context refuse incompatible previous generations.

Generation lifecycle: `candidate -> validated -> current -> previous`.
Incomplete/cancelled/failed candidates never publish. Reader leases retain
their immutable objects. Prepare outside global rendering locks; compare the
binding/input revision again at atomic adoption. Retain a compatible current
scene on failure; explicitly label it stale after an incompatible input change.
Host switching discards stale client responses but does not implicitly cancel
host-owned operations (D-013 and the Phase 14 operation boundary).

Operation progress uses the existing operation ID/journal/cancellation model,
with stage, processed/total units (unknown totals stay absent), source/target
generation, binding, scoped diagnostics and retry/input requirements. Outcomes
are `ready`, `repaired`, `needs_input`, `partially_repaired`, `unsupported`,
`cancelled`, `failed`. `repaired` requires adopted artifacts plus reinspection
of the same affected area with the original classified failures removed.
Visual acceptance remains separately pending until Cameron confirms it.

| HTTP surface under `/v1/worlds/{slot_id}/map-assets` | Semantics |
|---|---|
| `GET /status`, `GET /report` | Bound status and latest scoped redacted report; server ID required. |
| `POST /check` | Read-only scan, operation-backed when needed; explicit dimension/area and expected binding revision. No downloads. |
| `POST /prepare`, `POST /repair` | Operation-backed acquisition/preparation or diagnosed remedy; expected input/current generation. |
| `POST /import` | Consume a purpose-bound completed staged upload; no arbitrary client/world path. |
| `PUT /selection` | Ordered known source/pack IDs, expected selection revision; no game configuration changes. |
| `POST /restore` | Adopt a named compatible previous generation after validation. |

All require Worlds permission, live server/slot authorization and expected
binding checks. Upload begin/chunk/complete/apply and report/artifact access
must enforce their own credential/binding checks, including revocation.
Stale requests receive a conflict with the current revision. Versioned
capabilities are `worlds.map_assets.status.v1`, `.check.v1`, `.prepare.v1`,
`.import.v1`, `.repair.v1`, `.selection.v1`, `.restore.v1`, `.report.v1` with the
common prefix. Advertise only implemented actions and supported resource/capture
formats; unavailable advanced capture must never activate an Export button.

CLI grammar: `msc world map-assets status|report|check|prepare|repair|restore`,
`import <bundle>` and `select <ordered-source-id>...`. Reuse active-server
selection, explicit `--slot`, `--dimension` and area parameters for inspections,
`--generation` for restore, expected revision options and established operation
waiting/JSON conventions. The CLI is host-local, including an SSH login shell;
it gains no direct-remote URL/token mode. A portable bundle can come from another
machine. JSON shares the API result model and identifies pending operations;
input-required, unsupported, partial, cancelled and failed completed actions
must not return a repaired/success result.

Reports carry schema version, binding, snapshot/resource/geometry IDs,
dimension and checked bounds, source evidence, before/after classified counts,
sample original IDs/states/coordinates, truncation totals and operation IDs.
Count distinct states, block occurrences and visible faces separately. Include
`missing_model`, `missing_texture`, `unsupported_loader`, `unsupported_material`,
`missing_context`, `missing_saved_chunk`, `intentional_empty`, `model_resolved`
and visual acceptance status. Empty geometry is intentional only with positive
semantic evidence; it cannot be inferred just from absence of elements.
Export omits paths, credentials, player identities, inventory contents and raw
world/config files by default. No world-wide claim from a bounded-area report.

## Proposed initial bounds for P18.15

These are explicit design choices, not measured pack suitability claims.
Reject oversize input with the named limit and required remedy; never truncate
silently or repeat the same failed scan on tile requests.

| Resource | Initial hard limit |
|---|---|
| Individual archive / uploaded bundle | 2 GiB / 8 GiB compressed |
| Decompressed entry / whole scan | 256 MiB / 16 GiB |
| Archive entries / inventory source objects | 200,000 per archive; 2,000,000 cumulative / 10,000 sources |
| Nested JAR depth / count | 4 / 1,024 per scan, shared aggregate byte budget |
| JSON metadata/model bytes | 8 MiB per document |
| Parent / texture reference depth | 32 / 16; cycle detection required |
| Image dimensions / decoded bytes | 8,192 per axis / 256 MiB per image |
| Diagnostic samples | 5 coordinates per issue, 1,000 issues; count omitted items |
| Scan/render-preparation workers | 2 per host; bounded queues and cancellation |
| Content store / candidate reservation | 32 GiB per host / 16 GiB maximum; free-space check first |
| Terrain artifact | Existing 32 MiB response limit retained |

Reserve before extraction; cumulative limits include nested inputs and duplicate
entries. Reject links, traversal, special files, duplicate/case-colliding and
Windows-reserved paths on every platform. Hash source bytes consistently;
compare file identity/size/timestamps around reads and discard changed scans.
Only declared nested resources may enter the closure. Immutable completed
objects use atomic publication; current/previous and leased objects are never
evicted. If protected bytes exhaust quota, fail admission instead of deleting
them. Clean abandoned candidates on cancellation/recovery with bounded work.

Representative pack byte/entry/image measurements are **unavailable** on this
host. ATM10 Lite 1.1.0 is historical proof context, not a measured quota fixture.
The acceptance manifest must record actual sizes before these limits can be
called sufficient for that pack. Production bounds/store code have not been
added while the renderer feasibility stop remains in effect.

## P18.15 implementation record — 2026-10-04

The historical P18.14 blocker above was cleared for the pinned isolated proof by Cameron’s saved visual confirmation. Broader acceptance criteria remain unchanged. P18.15 implements resource inspection, not production supplemental-mesh adoption.

Read-only measurement of the installed ATM10 top-level 494 JARs found 1,438,919,961 compressed bytes, 823,443 ZIP entries and 2,381,796,095 declared decompressed bytes. Largest archive: 78,290,532 bytes; largest entry: 27,666,127 bytes. This justifies changing the proposed cumulative entry ceiling to 2,000,000 while retaining 200,000 per archive. Nested closure and decoded image usage were not measured; this is not evidence that a full scan or general mod rendering succeeds.

Additional implemented ceilings: 256 MiB aggregate resource JSON, 64 selected model references per state, 4,096 elements per model, 16 chunks / 262,144 requested blocks, 256 MiB per saved region / 512 MiB aggregate, 8 MiB decoded chunk NBT / depth 64 / 100,000 nodes. Config inspection is limited to 10,000 entries / 256 MiB aggregate / 8 MiB per file. Saved archives use the 2 GiB individual archive ceiling. Workers refuse excess requests rather than waiting in an unbounded queue.

Publication rechecks source stamps, configuration hashes and the host/server/slot/incarnation/revision binding. Immutable reader records remain available across pointer changes. Initial quota policy retains published and reusable content; it does not evict any object. Cancellation cleans candidate temporary files but can retain reusable hashed blobs. Quota exhaustion therefore refuses further preparation; automatic garbage collection is deferred. Incarnation uses conservative local source/slot evidence and can invalidate on metadata changes; it is not a portable restore identity. Live active-world inspection requires a stopped server. Unknown client inputs and selected order stay unknown; P18.16 supplies acquisition/staging. No provider or mod identity is inferred from filenames or namespaces.

## P18.16 acquisition/composition — 2026-10-04

Exact manifest file hashes and published sizes select client downloads; already
matching installed/disabled bytes precede reusable map-cache bytes and provider
acquisition. Requests use the existing provider credential boundary. Map download
transport refuses redirects, permits only Modrinth/CurseForge HTTPS content hosts,
and never consumes a server's arbitrary resource-pack URL. Author-blocked,
offline, missing-identity and checksum failures retain specific source requirements
and previously verified downloads. A different server/client file is **not**
selected by its filename: without explicit publisher relationship evidence the
counterpart remains an import requirement. Hash identification annotates loose
installed JARs only when the exact release lists the same file, game and loader.

Future imports retain manifest bytes and relevant override JARs, assets, metadata
and pack ZIPs in an immutable MSC-owned `.msc-map-source` receipt tree, separate
from server mods. This is source evidence, not a new inferred modpack identity.
Older imports without retained evidence remain explicitly incomplete. No launcher
credentials, arbitrary client configuration or world files are retained.

The stack starts with vanilla, then the set of exact/installed mod resources.
Conflicting same-priority mod bytes stay unresolved; sorted filesystem discovery
is not priority. An approved server pack is used only when local bytes match the
configured SHA-1; its URL is never fetched by a scan. Enabled client pack order
must come from the later importer; downloaded or uploaded pack ZIPs are not all
enabled automatically. Loose client override assets follow selected packs;
`client-overrides` follow common `overrides`. Parent/texture references and all
namespaces remain intact. Each selected resource records winners and overridden
sources. Applicable pack overlays require an evidenced game pack format; unknown
format is an input requirement. Filters use bounded anchored regular expressions;
unsupported expressions fail explicitly. Animation metadata is retained, not a
claim that Vantage animates its output. Custom-loader/generated geometry retains
its matching-client remedy.

Candidates own bounded scratch/resource objects and include a manifest, provenance
index, missing exact sources and prerequisites. Dropping one removes its scratch;
verified source downloads remain reusable. No candidate switches a renderer in
this step. No third-party assets enter Git or release artifacts.

**Renderer-route adjustment for P18.17:** use a reversible, recognized-field private
resource/palette adapter with the existing helper, instead of a separately built
Zig resolver extension. Namespace/path bytes map bijectively into a reserved
adapter subtree; collision with real resources is refused. Vanilla IDs retain
special handling. Adapt only snapshot palette names and known JSON reference
fields; arbitrary mod payload stays untouched. Unsupported models use explicitly
reported non-occluding fallback geometry. This avoids a second executable and its
four-platform toolchain while preserving the pinned baseline helper and explicit
override. It does not remove the difficult-model capture acceptance requirement.

## P18.17 implementation record — 2026-10-04

The normal Java manifest entry point now starts exact resource preparation when
there is a retained pack source, an installed mod JAR or a configured server
resource pack. Ordinary vanilla servers retain their existing helper path;
Bedrock retains its existing renderer. An explicit Worlds-authorized `prepare`
action also adopts resources for an active Java slot. `check` remains scoped,
read-only inspection. Renderer recovery, explicit executable/assets overrides
and release helper pins are preserved. The adapter ships inside the Rust agent;
there is no additional native executable, packaging input or release gate.

Preparation uses the existing consistent Java saved-world capture while running
(`save-off`, confirmed flush, bounded copy, `save-on`) or a checked private copy
while stopped. It never edits source saves, mods or launcher state. Captures are
limited to 2 GiB, 100,000 entries and 64 directory levels; individual regions are
limited to 256 MiB and decoded chunks to 32 MiB with bounded NBT validation. The
adapter changes only recognized reference fields and private palette names,
retaining unfamiliar compound fields. Modern implicit vanilla palette defaults
are shared between adaptation and diagnostic inspection. A saved custom
namespace dimension is staged through the helper's private overworld layout;
public dimension IDs and player feeds retain their original IDs.

Namespace and full path bytes are encoded separately with hexadecimal strings.
Mod block aliases have a `_glass` suffix because the pinned helper recognizes
that name as non-occluding. This conservative rule can retain internal faces; it
avoids inventing opaque culling for unknown mod behavior. Missing, ambiguous,
unsupported or contextual mod appearances use explicit inset checkerboard
models, without cull faces. These are never counted as `model_resolved`. Client
color providers for namespaced tint indices, custom loaders/materials and
block-entity context remain unresolved, rather than being inferred from the
encoded name. Vanilla biome data, language, textures and metadata accompany the
private resource tree. Supported model dependencies resolve separately from
visual acceptance; weighted alternatives retain the pinned helper's first-choice
policy, not an assertion of Minecraft's random weighting.

At most two asset workers, eight concurrent binding records and four retained
renderer generations are admitted. A binding includes host, server, slot, source
location and original dimension. Each preparation has a cancellable operation
and a monotonically distinct ticket; same-input manifest requests deduplicate,
and tile traffic does not initiate preparation. Work, helper startup and disk
staging occur outside renderer coordination. The final short guard rechecks the
ticket, snapshot epoch and active-server binding, then atomically changes the
report pointer and in-memory scene. Snapshot refresh invalidates outstanding
tickets through that same guard. Changed inputs, failures or cancellation retain
the prior scene; explicit retry reuses verified source downloads.

Candidate validation requests saved tiles overlapping the named inspection
area and their texture array. Default first-open scope is one saved chunk from
Y -64 through 319, not a whole-world repair claim. HTTP artifacts are bounded to
32 MiB, inflation to 256 MiB; tile magic and texture dimensions, pixels and
animation tables are checked before publication. Later tile failures remain
local and do not retire the usable scene. Scoped original-ID reports retain
missing saved chunks as a separate classification. Their geometry generation is
recorded, while visual acceptance remains `pending`.

Java manifests carry `mscGenerationId`; subsequent texture/tile requests pin it.
Generation leases keep old helpers and snapshots alive until 90 seconds idle.
Tile fingerprints include original chunk bytes, selected state dependencies,
parent/texture/animation metadata, vanilla biome data and a neighboring-chunk
apron. Atlas identity is also context: a different layer layout invalidates
references into that atlas. The viewer retains up to 64 MiB/128 verified compact
tile payloads and reuses a tile only when its fingerprint and initial atlas
identity match and all its layer indices fit that atlas. Other tiles render
through the candidate helper. This preserves safe selective reuse, not a promise
that a helper never recomputes a tile. Dynamic atlas growth cannot make an
out-of-range old tile eligible for reuse.

The old viewer remains visible while a hidden replacement loads. Successful
same-dimension adoption retains camera position/orientation, flight mode, depth
and a valid follow target; player polling continues. A failed replacement leaves
the old viewer alive. Closing/switching dimensions requests operation cancellation;
idle generations retire their helpers/private caches. Provider/helper HTTP waits
remain bounded by their existing timeouts, so cancellation can wait for an
in-flight request. Status exposes retained snapshot/resource times, stale state,
required exact source identities, input prerequisites and explicit retry. Staleness
uses the conservative live source/slot binding: ordinary metadata changes can
label a retained saved scene stale. Fresh terrain requires explicit refresh;
this is not live terrain streaming or a portable restore-identity guarantee.

Essential new regression source covers late completion, failure, cancellation,
deduplication and host isolation in the renderer coordinator, and the private
namespace/non-occlusion/parent-texture mapping. The existing store regression does
not cover the visible scene swap, and the inspection regression does not exercise
the production adapter. Both use controlled data without helpers, networks,
clocks or sleeps and are expected to take under two seconds combined locally.
They were type-checked, not run. No release workflow was changed. Linux compile
and viewer type checks do not establish live modded, Paper/Tectonic, Bedrock or
Windows/macOS acceptance; those owner checks remain open.

## P18.18 portable client imports — 2026-10-05

The native inspector accepts an explicitly selected Prism instance, an
unambiguous official game directory, a Modrinth/CurseForge archive, a portable
MSC bundle or an individual JAR/pack with the selected map's game context.
It reads metadata and archives without executing them. Portable bundles contain
version/loader identity, ordered resource layers, source hashes/mod declarations,
enabled-pack selection when actually available, and checksum-named resource
objects. Bytecode, launcher credentials, options unrelated to pack selection,
logs, screenshots and saves are excluded. Nested declared JAR resources use the
existing bounded inventory. Archive client overrides outrank ordinary overrides
independently of ZIP entry order. Exact manifest downloads use the existing
provider boundary and agent-held credentials; unavailable/manual files remain
specific requirements. Locally supplied metadata/hashes are local evidence,
not publisher verification. Unknown mod versions require matching original
JAR hashes; known overlapping declarations must match the installed release.

Native inspection yields an opaque 30-minute handle and 1–8 MiB reads, plus
summary/portable-export commands. Remote transport carries only resource bytes,
never a client filesystem path. Map staging uses `map-client-assets` with
`fileId` = slot UUID, `operationId` = current binding revision, and exact
`expectedBytes`; the agent stores its own binding and credential. Worlds
permission/ownership and the live binding are checked at begin, each chunk,
completion and redemption. Owner-only cancellation can remove an old upload
after a server switch. Expired map uploads are pruned at the next admission;
two concurrent map uploads are allowed, each at most 8 GiB. Existing staging
is best-effort across agent restarts. A completed bundle is rehashed and
reinspected on the agent before entering the same cancellable tile/atlas
validation and guarded scene adoption as automatic resources. Importing bytes
alone does not prove repaired rendering.

Static resource receipts live under MSC's map store, keyed by agent/server/slot.
Their compatibility fingerprint includes slot metadata, game/loader/mod/config
inputs and saved world generation/version identity. It excludes ordinary save
timestamps so a consistent snapshot flush does not invalidate a matching client
import. Snapshot-bound geometry still uses the full live binding and saved
snapshot checks from P18.17. Restores/imports/updates with changed compatible
inputs require a new client import. The displayed scene remains available if
validation/adoption fails; validated source receipts can remain for an explicit
retry. Imported client mod layers replace server-side resource copies as one
unordered group unless an explicit mod resource order is supplied. Optional
packs are never all enabled by the collector. Their known selection/order is
preserved; unknown selection becomes an input requirement.

Bundle admission checks expanded object plus materialized-layer bytes against
the existing store quota/free-space policy before extraction, and reuses all
path/link/case/image/document/reference limits. Existing reusable receipts are
checked against captured layer hashes on every preparation. Correct appearance,
remote Fedora-to-Ubuntu imports, Windows/macOS paths and baseline regressions
remain owner acceptance, not results inferred from Linux compilation.

## P18.19 implementation record — 2026-10-05

The repair sheet and headless commands share the existing cancellable preparation
operation. Additive selection/restore routes validate current binding, a hashed
selection revision and the prior receipt's game/mod/config/world compatibility.
Import and selection updates preserve one previous resource receipt atomically;
its immutable extracted layers remain available. A failed renderer adoption
retains the displayed scene and validated source receipts for an explicit retry.
This is a map-resource rollback, not a world restore or mod enablement change.

Scoped checks re-resolve the adopted inventory against the retained private
snapshot and validate overlapping terrain artifacts plus their atlas. They
publish a new diagnostic report without replacing geometry. Stored reports are
hash-validated and used only for the matching geometry generation. Publication
rechecks live binding and geometry identity so an old inspection cannot replace
a newer repair report. Baseline checks reuse a compatible private map snapshot,
otherwise a live directory check requires a stopped server.

Repair candidates inherit the latest report's exact area and reuse the saved
scene snapshot until explicit terrain refresh. The success classifier requires
matching saved scope/snapshot/world binding, zero remaining classified failures,
validated geometry and an otherwise ready result before reporting `repaired`.
The final publication and scene adoption share the existing guarded transaction.
Counts cannot establish appearance, and new snapshots do not retroactively prove
a repair of the previous saved area. Partial resolution and unsupported loaders,
materials, renderer namespaces or missing context remain explicit outcomes.

Rebuild aliases the same fresh renderer-candidate path for the affected area;
tile dependency fingerprints identify unchanged tiles. It does not delete all
caches. Generation adoption is still a whole coherent dimension/atlas transaction
rather than mixing individually rebuilt tiles from incompatible atlases.

The desktop captures its selected host transport before a multi-request upload,
checks the native preview's checksum, cancels uploads/operations on dismissal,
and discards native scratch handles after close or replacement. Read-only native
inspection completes before transfer; closing during inspection discards the
result rather than applying it. Report/bundle exports use explicitly chosen new
local filenames. No user filesystem path goes to the remote agent. The sheet
uses existing neutral tier/type/button tokens, flat grouping, bounded details
and one primary remedy. It never opens automatically; dismissal cannot produce
repeated unchanged warnings. Live players and map controls retain their existing
behavior, with camera state preserved by the existing generation reload path.

One essential synthetic report regression protects the success boundary while
missing/unsupported issues remain, artifacts are absent, or saved scope changes.
It also checks partial resolution and successful resource-generation evidence.
No network, renderer, timers or visual snapshots are used; expected execution is
below two seconds. Its source was compiled only; execution requires Cameron's
specific authorization. Linux static checks do not close platform/visual gates.

## P18.21 acceptance evidence preparation

The read-only [collector](../../tools/world-map-assets/README.md) validates
private exported reports, package identities and owner-confirmed captures,
then emits a redacted matrix. Build-only receipts never populate rendering
cells. Actual measurements must be supplied and compared with the budgets above;
missing measurements are not zero. No production acceptance observations were
collected in this step. The corrected Forge draft compiles, but its production
integration and distribution remain open. Owner verification and independent
phase review remain required.
