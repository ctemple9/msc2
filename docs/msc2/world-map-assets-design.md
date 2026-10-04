# Java map resources — renderer and repair contract

P18.14 · 2026-10-04 · **Feasibility blocked; production unchanged.**

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
| Archive entries / inventory source objects | 200,000 / 10,000 |
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
