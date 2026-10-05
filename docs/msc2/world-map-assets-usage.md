# P18.15 resource inspection

This inspects saved Java blocks and resource evidence. It does not adopt captured meshes or repair rendering. Existing maps keep their renderer. The isolated P18.14 proof remains separate.

Build the updated executable:

```sh
cargo build -p msc-agent --bin msc
```

Run the updated agent through your normal development startup before using the commands below. Do not start another agent against the same application data while an installed agent is running. Select the intended Java server and stop it before inspecting its active world; archived slots can be inspected without stopping the server. Worlds permission is required.

List slots and obtain the selected slot’s binding revision:

```sh
target/debug/msc --json world list
target/debug/msc --json world map-assets status --slot SLOT_UUID
```

Replace `SLOT_UUID` with the listed UUID and `REVISION` with `binding.revision` from status. Bounds are inclusive, in block coordinates. For a small overworld region:

```sh
target/debug/msc --json world map-assets check --slot SLOT_UUID --expected-revision REVISION --dimension minecraft:overworld --min 0 64 0 --max 15 80 15
target/debug/msc --json world map-assets report --slot SLOT_UUID
```

Check waits for the asynchronous operation and prints its report. `--no-wait` instead returns its operation ID. Ctrl-C uses existing operation cancellation. `needs_input` returns a nonzero CLI exit code while retaining the diagnostic report; it is not a repaired/ready result. A stale revision refuses publication: reread status rather than reusing the old binding.

Inspect original namespaced IDs, source hashes and declared mod identities, requested area, snapshot/resource generation, sample coordinates, classifications and omitted counts. `missing_saved_chunk` counts chunks rather than blocks. An explicit empty model is distinct from a missing model. A block entity’s presence does not prove its rendered contents are available. Unknown client selection and unsupported loader/material/renderer features stay explicit. A saved-world version mismatch refuses geometry adoption. `visibleFaces` and geometry generation remain absent because this stage does not render geometry.

The inventory reads installed JARs, declared nested JARs, retained manifest receipts and existing server `resource-packs` archives. It reads only an existing exact-version vanilla helper cache; it does not download missing assets, execute mods, infer pack priority or collect a client installation automatically. Explicit client acquisition/staging is the next step. Diagnostics are a bounded independent resolver inspection, not evidence that the production Vantage renderer can display a modded model.

Acceptance remains manual: check a known broken-model region and an intentional empty model; inspect a second slot/server and confirm bindings are distinct; open existing Java and Bedrock maps and confirm their baseline rendering remains intact. Broader mod/platform acceptance is unchanged. Numeric limits and conservative generation/quota behavior are recorded in [the design](world-map-assets-design.md#p1815-implementation-record--2026-10-04).

## P18.17 saved renderer preparation

Opening a modded Java map now queues exact resource preparation automatically.
The first usable generation appears after saved tile/atlas validation. Later
preparation preserves the displayed scene, including its camera and player
feed. A retained scene shows its saved snapshot/resource times; input-required
states retain marked fallbacks. Use **Retry resources** after addressing the
reported requirement. **Refresh terrain** captures a new saved snapshot.
Neither a usable candidate nor `model_resolved` closes visual acceptance.

For the selected active Java slot, fetch a fresh revision with `status`, then:

```sh
target/debug/msc --json world map-assets prepare --slot SLOT_UUID --expected-revision REVISION --dimension minecraft:overworld
target/debug/msc --json world map-assets rendering --slot SLOT_UUID --dimension minecraft:overworld
target/debug/msc --json world map-assets report --slot SLOT_UUID
```

`prepare --no-wait` returns its cancellable operation ID. With waiting enabled,
an unresolved scoped report returns a nonzero exit code even when marked
fallbacks keep the map usable. Reports from retained renderer scenes keep their
original snapshot and binding; `rendering` distinguishes their age from current
live inputs. Required source records expose provider/project/release/file IDs,
logical filenames and published hashes, without download URLs, credentials or
host paths. Archive-slot inspection remains available through `check`; renderer
adoption requires an active slot.

HTTP clients use `POST /v1/worlds/{slot_id}/map-assets/prepare` with `serverId`,
`expectedRevision`, original `dimension` and optional `area`. Area bounds retain
the existing 16-chunk/262,144-block limit. Rendering status is
`GET /v1/worlds/{slot_id}/map-assets/rendering?serverId=...&dimension=...`.
Java terrain manifests return `mscGenerationId`; pass `generation` and
`serverId` on every subsequent atlas/tile request to avoid mixing generations.
A first-open `map_preparing` response includes status/binding in error `details`.
Poll manifests only while preparation is active; failures need explicit retry.

Owner acceptance remains pending: compare a standard-model modded saved region
and custom dimension against Minecraft, check the original-ID scoped report,
change a parent texture and retry, cancel/fail a replacement while observing
camera/player continuity, and confirm Paper/Tectonic and Bedrock baseline maps.
Perform the supported Windows/macOS checks separately. The implementation and
bounded limitations are recorded in the P18.17 design record.

## P18.18 portable imports

The desktop repair controls are P18.19. Native inspection/export commands now
produce a resource-only portable MSC bundle from a selected instance/archive.
An ambiguous official launcher profile requires a specific matching instance;
Prism's `mmc-pack.json` supplies its exact game/loader identity. The bundle
never includes launcher credentials or a private world.

On the selected headless agent, after obtaining its current `status` revision:

```sh
target/debug/msc --json world map-assets import /path/to/client-resources.zip --slot SLOT_UUID --expected-revision REVISION --dimension minecraft:overworld
```

The CLI streams bounded chunks, verifies the transferred hash, waits for the
same preparation/adoption operation and reports its scoped diagnostics. Ctrl-C
cancels transfer or the existing operation. `--no-wait` returns the operation
ID. Unresolved input/rendering issues return a nonzero exit code; an upload
cannot be reported as a complete repair. No local Minecraft installation or
GPU is needed on the agent. Native desktop import/export uses the authenticated
selected-host transport; the next step exposes these controls in the map.

## P18.19 affected-area repairs

In a Java map, **Check rendering** opens the repair sheet without interrupting
player tracking. **Check camera area** inspects one saved chunk column and 32
vertical blocks around the current depth. The report names its exact bounds,
snapshot, block states, missing sources and unsupported model/context cases.
A repair uses that reported area; it does not claim the entire world is fixed.
The sheet stays closed after dismissal and never repeatedly opens for an
unchanged issue. The quiet map resource status still reveals changed inputs.

Select **Import client instance** for an explicit game/Prism directory, or
**Import client assets** for an archive, individual pack/JAR or an exported MSC
bundle. Review game/loader identity and the bounded mod/version evidence before
applying. The agent additionally checks installed mod release/hash agreement.
Wrong releases require selecting the matching client, not another download of
the same wrong files. Unknown overlapping mod priority stays unresolved until
the actual client order is supplied. Local hashes are not publisher verification.
**Export resource bundle** writes a new portable file for a headless host.

**Select resource packs** lists imported packs and applies low-to-high priority.
Its optional mod ordering requires an explicit acknowledgement of known client
priority; filenames never determine that order. **Repair map assets** appears
for a retryable acquisition/helper failure or a resolvable resource diagnosis.
**Rebuild affected terrain** starts a new validated renderer candidate for stale
or invalid terrain artifacts using the saved snapshot and reported area. It
keeps other saved data and map resource caches. To include newly generated or
saved chunks, use the map's separate **Refresh terrain** action. Unsupported
code-driven models, materials and missing world context need the later exporter
or adapter capability; there is no active runtime exporter control yet.

A compatible previous import/selection enables **Restore previous map resources**.
This swaps only MSC-owned resource selection, not game files, enabled server mods
or the world. Every mutating action validates the current binding and selection
revision. Transfer and operation cancellation retain the displayed scene. Closed
maps and host/server switches cancel work through the original captured transport.
**Export report** writes bounded diagnostic JSON to a new local file; existing
files are preserved. Reports have no host paths, credentials or private world data.

The same actions are available on a selected headless agent:

```sh
# Obtain binding revision, selection revision, layer IDs and compatible rollback ID.
msc --json world map-assets status --slot SLOT_UUID
msc --json world map-assets selection --slot SLOT_UUID
msc --json world map-assets check --slot SLOT_UUID --expected-revision REVISION --dimension minecraft:overworld --min 0 64 0 --max 15 95 15
msc --json world map-assets repair --slot SLOT_UUID --expected-revision REVISION --dimension minecraft:overworld
msc --json world map-assets select --slot SLOT_UUID --expected-revision REVISION --expected-selection-revision SELECTION --dimension minecraft:overworld --packs LOWER_PACK_ID HIGHER_PACK_ID
msc --json world map-assets rebuild --slot SLOT_UUID --expected-revision REVISION --dimension minecraft:overworld
msc --json world map-assets restore --slot SLOT_UUID --expected-revision REVISION --dimension minecraft:overworld --generation PREVIOUS_SELECTION_ID
msc --json world map-assets report --slot SLOT_UUID --output map-rendering-report.json
```

Omit `--packs` to explicitly select none. Supply `--mod-order LOW_MOD_ID HIGH_MOD_ID`
only when the full low-to-high mod layer priority is known. Repair/import/select/
restore/rebuild use the previous scoped report's area, or the existing bounded
first-area default if there is no report. HTTP callers can supply explicit area
bounds. `--no-wait` returns the operation ID; Ctrl-C cancels the existing operation.
CLI completion checks the returned report's operation ID and returns nonzero for
unresolved, unsupported, partial or failed work. A valid report export is a
successful export, not evidence that rendering was repaired.

`POST .../map-assets/repair` and `POST .../map-assets/rebuild` reuse preparation,
artifact validation and guarded adoption. `GET .../map-assets/selection` returns
selection metadata; `PUT` applies it, with equivalent `POST` supported by the
existing desktop adapter. `POST .../map-assets/restore` names a compatible prior
selection generation. All routes require Worlds permission. A scoped check
uses adopted resource inventory and artifacts against their retained snapshot,
or the baseline map's private snapshot; it never downloads resources or reads a
running server's live mutable world as if it were a consistent snapshot.

A report becomes `repaired` only after its original classified failures disappear
in the same saved area/snapshot and compatible world binding, validated geometry
is ready, and the candidate is adopted. Otherwise it distinguishes `ready`,
`needs_input`, `partially_repaired` and `unsupported`; operations separately expose
`cancelled` and `failed`. Before/after evidence records source and target resource
generations and classification counts. Visual acceptance always remains pending
until Cameron checks the named blocks against Minecraft.
