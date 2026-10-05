# Java mod assets — acceptance contract and fixture availability

**P18.23 update (2026-10-05):** Production capture validation, private position
replacement, guarded terrain/capture adoption, generation-bound delivery and
desktop display code are implemented. Build/type checks are recorded in the
rolling plan. No production capture, Minecraft launch, visual comparison,
native-platform result or test execution is inferred from that code. The earlier
NeoForge owner confirmation remains isolated feasibility evidence. No new
acceptance row is closed.

The three new controlled regressions in map_capture.rs protect mismatched
binding/snapshot/client inputs, malformed portable geometry/images/archives,
previous-capture preservation and private palette repacking including waterlogged
blocks. Expected runtime is below two seconds after compilation; compilation is
not test execution. Exact candidate owner command:
`cargo test -p msc-infrastructure --test map_capture`.

Required P18.23 appearance check: once P18.24 supplies the real helper command,
issue the agent request from the named adopted saved fixture, capture matching
inputs, import through production staging, and compare the difficult-model/context
objects inside MSC with Minecraft. Check same-state uncaptured objects, adjacent
faces, water/transparency, cutaway, camera/player continuity and saved-frame
label. Wrong source context, changed selection, cancelled import and malformed
files must leave the existing map intact. A reader without captureFormat must
receive capture_viewer_required. Preserve original all-platform/repair gates;
legacy unbound proof data cannot be accepted by copying a request onto it.

P18.14 · 2026-10-04 · **Open; no new live rendering passes.**

The [renderer design](world-map-assets-design.md) records the inspected pin,
namespace route, difficult-model blocker and proposed capture alternative.
Compilation, model resolution, downloads and reports do not establish the
required successful rendering/repair. Historical observations remain in
`phase18-map-acceptance.md` and `tools/world-map-proof/README.md`.

## Required private receipt

Each fixture needs: fixture ID; supplying owner; game version and client/server
artifact hashes; loader version/hash; exact mod/pack provider release/file IDs
and hashes; enabled pack order/configuration hash; immutable source snapshot
hash/time; dimension; saved chunk bounds; named block coordinates and full
states; expected Minecraft appearance; target repair; measured compressed,
expanded and largest-image bytes; renderer/adapter/format pins; operation and
before/after report IDs; map/Minecraft capture references; observer/platform/date.

Unknown fields are explicitly pending. A filename, namespace, pack display name
or older runtime observation cannot fill a missing release/hash/coordinate.
Private worlds, game assets and capture data stay outside Git. Only redacted
receipts and acceptance dispositions belong here. Cameron is the private-world
supplier for historical cases; availability of a matching client and exact
versions must be confirmed rather than assumed.

## Catalogue

All new acceptance cells below are pending. Historical versions describe what
was previously inspected; they are not complete fixture pins. Exact hashes,
loader/mod versions and coordinates absent from the historical record remain
required inputs. Synthetic fixtures use MSC-authored data only.

| ID | Version/source availability | Named target appearance | Required action and unresolved input |
|---|---|---|---|
| vanilla-control | Historical Vanilla 26.3; Cameron private world. Artifact hash/coordinates pending. | Upright logs, deepslate, stairs, leaves, glass, water; match Minecraft orientations/transparency. | Existing vanilla assets path; no mod generation selected. Capture named blocks and current exact receipt. |
| paper-tectonic-control | Historical Paper 26.2; Cameron supplies exact Tectonic release/hash and area. | Existing natural terrain and vanilla palette retained. | No block-adding mod path merely because world generation differs; retain visible scene through candidate failure. |
| purpur-control | Historical Purpur 1.21.11; hashes/coordinates pending from Cameron. | Vanilla palette, Nether/End controls and player navigation. | Existing assets; no loader-flavor inference. |
| fabric-blocks | Cameron supplies saved world + matching block-adding Fabric client, exact game/loader/mod pins pending. | A named non-vanilla block with distinct shape, texture and state orientation. | Automatic installed/exact-provider closure, then matching-client import if exact resources unavailable. |
| forge-blocks | Historical Forge 26.3 is runtime evidence only; block-adding fixture/pins pending from Cameron. | Named non-vanilla block and variant. | Exact resource acquisition/import; no newest-release substitution. |
| neoforge-blocks | Historical NeoForge 26.2 is runtime evidence only; block-adding fixture/pins pending from Cameron. | Named non-vanilla block and variant. | Exact resource acquisition/import; verify custom-loader classification separately. |
| mixed-pack | ATM10 Lite 1.1.0, Minecraft 1.21.1, historical Prism client proof. Cameron supplies exact manifest/client/server hashes and loader/mod releases. | Named ordinary blocks across multiple contributing namespaces, with dependency textures. | Client manifest/overrides/order retained; missing exact files identified and imported without installing client-only JARs into the server. |
| namespace-collision | MSC-authored synthetic `alpha:panel`, `beta:panel` with different textures; no third-party assets. No prototype run yet. | Both visibly different; parent `beta:block/base` and texture variable resolving to `alpha:block/panel` remain distinct. | Namespace-aware resolver; order override affects only the selected resource. Full states/coordinates assigned in private scratch fixture. |
| dependency-order | MSC-authored two ordered packs overriding a shared parent texture; files/hashes to be created with isolated prototype. | Last selected winner consistently changes dependent child appearance; disabled pack contributes nothing. | Select/reorder then rebuild affected dependencies; record exact selection/digests. |
| empty-versus-broken | MSC-authored intentional invisible model plus a missing model and missing texture. Fixture bytes pending. | Invisible technical block stays invisible; missing resources have counted visible fallback without hiding neighbors. | Actual resolver diagnosis distinguishes supported empty, unsupported custom loader and missing bytes. |
| custom-loader | Proposed NeoForge 1.21.1 `neoforge:composite` fixture from the documented loader contract; exact NeoForge build/hash and saved fixture pending. | Both selected child model parts render according to visibility, rather than empty standard elements. | Matching-client capture or explicit pinned loader adapter; no viable implementation/proof yet. |
| code-rendered-context | Historical `lootr:lootr_chest` is unresolved placeholder evidence. Add a context-dependent visible-content object from a pinned mod; exact release/state/coordinates pending. | Real chest geometry/materials, plus two same-state objects with visibly different contents or neighbor context. | Client capture/internal adapter with actual context. Lootr alone does not prove visible-content capture. This is the P18.14 feasibility blocker. |
| saved-custom-dimension | ATM10 custom folders historically lacked saved region chunks. Cameron supplies a named generated/saved dimension with exact ID/region coordinates/hash. | Saved nonstandard-dimension terrain including a mod block. | Private dimension normalization/resolution; missing chunks remain missing. Existing discovery alone cannot pass. |
| bedrock-control | Existing mature BDS fixture from Cameron; exact current host/version/hash/coordinates pending. | Stairs, foliage, glass/water and player roster/Fly/Follow retain existing behavior. | Shared viewer/helper delivery regression acceptance; supported BDS host on every desktop. |

No particular loader version is invented to make a row look pinned. Missing
private fixtures block final acceptance. The capture-route row additionally
blocks the next production step; ordinary source inventory would not resolve
it.

## Required proof to resume Batch A

1. Pin one actual matching client/mod/loader and saved contextual fixture, with
   coordinates, entity/neighbor context and source bytes documented privately.
2. Build an isolated MSC-owned client-capture or explicit mod-adapter prototype.
   No production renderer, game save, server JAR or MSC 1 changes. A source-level
   hook description is insufficient; record the actual prototype revision.
3. Supply Cameron a runnable command for that prototype, bound to a private
   source/output and named area. The command must exist before asking him to
   run it. It must produce validated mesh/material/context data and an MSC
   proof-viewer result, not a texture audit or checker cube.
4. Record successful visual comparison for the custom loader and contextual
   object, including two equal palette states with different visible context,
   unvisited-chunk refusal and saved-frame semantics. Establish which materials
   need the supplemental format and retain baseline Java/Bedrock decoding.
5. Pin the demonstrated route/schema and remove the feasibility blocker only
   on that evidence. Then P18.15 can proceed under the already authorized batch.

No live command is supplied now: no capture executable/adapter exists in the
inspected repo. The prior `vantage meshtex` and `audit_java_terrain_assets.py`
commands exercise the failing pipeline and cannot satisfy this proof. No tests,
rendering smoke suites, game launch or release workflow were run in P18.14.

## Final rendering and repair gate

Record actual evidence for Windows x86_64, Fedora desktop/Linux x86_64,
Ubuntu headless/Linux x86_64, Intel macOS and Apple Silicon macOS. All are
pending for the new feature. Exercise all six Java flavors; a loader running
only vanilla terrain is not the block-adding fixture. Apple Silicon Bedrock
viewer acceptance uses a supported remote BDS host and does not add local BDS
support. Headless imports must work without Minecraft, GPU, launcher or login
on the agent host.

For every repaired case record before/after failures for the same dimension,
area and snapshot; adopted resource/geometry generations; operation outcome;
named visual appearance and Cameron's confirmation. Missing exact resources,
wrong source version, wrong order, corrupt candidate, stale geometry and
context/custom-loader capture must each have a successful applicable remedy.
Download/import completion alone remains pending repair. Capture is a frozen
saved appearance with explicit time/context; never label it exact/live.

Preservation evidence must cover camera/dimension/depth/exit, roster/Fly/Follow,
health/player polling during work, cancellation/offline failure retaining the
scene, host/slot/restore isolation, stale-job refusal, bounded disk/memory/work,
retry reuse, no source-file mutation and compatible rollback. Counts identify
their scope and separate intentional empty geometry, absent saved chunks,
missing resources, unsupported features and material/culling errors.

P18.21's evidence collector may fill receipts supplied by actual platforms;
visual rows stay pending until Cameron confirms the named blocks. No automated
CI/release gate, release/tag change or universal present/future-mod claim is
introduced. Any required unresolved row keeps the feature gate open.

## P18.19 repair acceptance — pending owner verification

No live rendering or native-platform repair pass is claimed by this implementation.
Use a disposable, exactly pinned saved fixture and record the before/after report
and operation IDs, snapshot/area, resource generations, captures and observer.

- Remove a required standard model/texture. Desktop and CLI identify the same
  block and missing source; matching import/acquisition removes that failure and
  adopts validated terrain. `repaired` requires the same saved scope; check the
  actual block appearance against Minecraft and an unchanged good block.
- Import the wrong game/loader/mod release. Preview or host validation rejects
  it, identifies the mismatch and leaves the previous scene usable. Choose the
  matching instance; a repeated wrong import does not claim success.
- Reverse two conflicting selected packs, check the named conflicting block,
  then correct low-to-high order. Confirm both expected texture and unaffected
  blocks. For unknown mod overlap, supply only proven client priority.
- Make an affected terrain artifact unavailable/corrupt. Scoped checking fails
  clearly; rebuild produces validated dependent terrain/atlas without deleting
  game files or all caches. Check camera/depth/dimension/follow continuity.
- Cancel transfer and cancel renderer preparation; switch hosts/servers while
  inspecting/transferring. No candidate reaches the wrong binding. Previous
  visible scene survives; scratch/staged resources are cleaned or expire.
- Restore the named compatible prior selection; reject an incompatible prior
  world/game/mod/config receipt. Export a redacted report and resource-only bundle
  to new local filenames and consume the bundle on the headless agent.
- Preserve Paper/Tectonic, pure vanilla and Bedrock automatic maps. Read-only
  checking uses a consistent saved snapshot while a Java server runs. Missing
  saved terrain recommends generation/save/refresh, and unsupported runtime
  models/context offer no fake working exporter.
- Repeat supported Fedora-to-Ubuntu remote import and native Windows/macOS path,
  cancellation and export checks. Compilation on Linux is not those passes.

## P18.20 public Forge fixture selection — build passed, acceptance pending

Cameron authorized selecting the public fixture on 2026-10-05. The exact input
receipt is [pins.json](../../tools/java-map-export/forge-1.20.1/pins.json):
Minecraft 1.20.1 / Forge 47.4.10; Supplementaries `1.20-3.1.43`, Modrinth release
`S0TIJ1hU`, file `5cdXldaM`; Moonlight `1.20-2.16.35`, release `W0ZWjZib`, file
`sKZHESzj`. Downloaded mod bytes matched their published SHA-512 values; the
receipt also records exact local SHA-256/size for game, loader and mods.

Intended new isolated fixture: custom-loader goblet at `9,65,3`, plus same-state
pedestals displaying diamond/emerald at `3,65,3` and `6,65,3`. This version has
no barnacles block model; source inspection selected its real
`supplementaries:goblet` loader. No world, snapshot, live block states, capture,
operation, corrected MSC appearance or owner confirmation exists for this
fixture yet. Input identity alone fills none of those missing fields.

Draft Forge capture compilation failed on 13 material/lighting member-access
errors; details and the identified mapping correction are in the
[helper README](../../tools/java-map-export/forge-1.20.1/README.md).
The authorized SRG-name correction now passes Java compilation and JAR
reobfuscation on Linux, with three deprecated-constructor warnings. The private
build receipt records the helper digest; no capture or rendering pass is
claimed. Cameron authorized proceeding with P18.21 evidence preparation.
The catalogue/platform rows and feature gate remain pending. No production
export command or supported Forge rendering range is advertised by this draft.

## P18.21 evidence preparation — owner acceptance pending

The [collector instructions](../../tools/world-map-assets/README.md) define the
exact owner invocation, private session format, raw report exports, capture
checksums and owner-only visual confirmation. The collector reads existing
receipts; it performs no repair, game launch, package installation or test.
Python compilation and collection of the actual Forge build receipt passed.
The JAR is 27,171 bytes with SHA-256
`7f9d58e2bdc65a1fe49ea256101393b48abf7eb13cd5c0f2269fefdda9a48e45`;
the private build receipt SHA-256 is
`0507c9210e3ab3684b0eeded35f18b09bd71581e8bd40e653f0545e3337de817`.
This records one build and zero production/platform observations.

| Desktop → agent | macOS | Windows | Linux |
|---|---|---|---|
| macOS | Pending | Pending | Pending |
| Windows | Pending | Pending | Pending |
| Linux | Pending | Pending | Pending |

The collector expands these nine OS-family pairs into 16 transport cells with
both native macOS architectures, Fedora desktop and Ubuntu agent, plus local
Fedora and Ubuntu cells: 18 total. All remain pending. Its 222 coverage cells
cover named fixtures, repairs, controls and scenarios across the five desktop
profiles, with Windows-specific controls limited to Windows. All remain pending.
Cameron’s previously recorded NeoForge isolated proof remains scoped feasibility
evidence; it does not fill production transport or repair cells.

Read-only inspection of Linux, both macOS headless packages, Windows headless
and desktop staging found the existing agent, Bedrock helper and Vantage/license
payloads; macOS Intel also stages its existing Sidecar payload. Tauri packages
the staged agent resources. The release workflow retains its nine required
artifacts, checksums and signed update metadata. No workflow was changed or run.
The draft Forge JAR is not staged in those packages. Installation, update,
rollback and helper launch acceptance on actual platforms remain pending.

Actual corrected appearances, all-platform transport, performance measurements
and budget review still require owner observations. The P18.20 production
exporter/context/import/adoption path remains incomplete. The collector always
keeps the feature gate open for owner verification and subsequent independent
review; successful receipt collection cannot close it.
