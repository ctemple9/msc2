# P18.15 resource inspection

**Current continuation (P18.25):** Production intake, private matching-client
preparation, loader adapters and the guided repair action are implemented.
Real rendering, successful repair and platform acceptance remain P18.26. Earlier instructions
below describe historical stages. Isolated proof ZIPs are not production captures.

The new API issues a request through
`POST /v1/worlds/{slot_id}/map-assets/capture-request` using the scoped check body,
then consumes an authenticated completed upload through the existing import
operation with the same area. The request comes from an adopted scene, retains
it for 30 minutes, and requires matching independently observed inputs/snapshot.
No game code runs on a headless host. The explicit `capture-context` action
exports bounded saved chunks and context data for private client preparation.

## Repair from the desktop

In the world's map, open **Check rendering**. The sheet identifies its host,
server, world, dimension and saved bounds. First inspect and apply the matching
Prism client instance with its actual enabled packs/mod priority; capture needs
its exact client resources and configuration receipt.

Under **Use Minecraft rendering**, choose a small affected area and **Preview
saved capture**. The preview shows the saved snapshot and exact game/loader
version. It refuses unavailable matching helpers without selecting another
version. Select the Prism instance folder containing `mmc-pack.json`, the exact
Minecraft **client** JAR, and the required Java executable. Optional memory
allocation affects only the private copy; empty uses the source instance's
allocation. **Prepare private client** downloads the authorized context through
the existing authenticated host connection, checks/copies the inputs and creates
private files. Preparation does not start Minecraft.

Choose the installed Prism Launcher executable and click **Launch private
Minecraft**. Sign in within that separate launcher directory if required. Open
its `msc-capture` save, load the requested dimension/chunks, run
`/mscmapcapture prepare`, then `/mscmapcapture export` after restoration completes.
Return to MSC and click **Validate and apply captured rendering**. MSC checks
the same saved blocks before import, validates the complete local export,
transfers data through authenticated staging, adopts checked terrain and compares
the resulting scoped report. It also confirms the viewer displayed the adopted
generation before presenting a successful result. Camera, depth and player
navigation follow the existing retained-scene adoption path.

A successful upload is insufficient. A **repaired** result requires a same-saved-
area comparison, validated adopted geometry and resolved affected diagnostics.
Partial/unsupported results remain explicit; the owner must still compare shape,
contents, textures, transparency and neighbors with Minecraft. A saved capture
is one animation frame, not a live simulation.

**Cancel action**, **Stop private client**, sheet close and dimension/host/world
switch stop the owned client/transfer. A stopped session requires a separate
**Reopen stopped session** action before a new launch; a late launch cannot
undo cancellation. Private prepared files remain for retry. **Reopen retained
capture** lists their original host/world identities and permits checking output
without restarting Minecraft; the displayed recovery identifier also survives
app restart. **Discard private files** removes only the selected private copy,
after confirmed process termination. Unconfirmed termination retains files and
reports the problem. Expired/changed context needs a fresh preview/preparation.
No server restart or original-client/save modification is part of this flow.

### If blocks still fail

- Missing resources or unknown priority: inspect/apply the exact client instance
  and its actual pack/mod order. Re-export old bundles to add configuration
  evidence. Repeat capture only after those inputs match.
- Missing saved chunks: visit/generate and save them in Minecraft, refresh
  terrain, then preview fresh context. The private client also needs the requested
  chunks loaded and lighting complete before prepare/export.
- Wrong Java/game/loader/client JAR: use the preview's exact pins. The server JAR
  does not contain client models and textures.
- Stale binding/snapshot/configuration: discard the stale private preparation,
  refresh saved terrain and prepare fresh context for the same affected blocks.
- Client-only shader/alternate renderer refusal: use a separate matching client
  with default rendering, reimport its resource/configuration evidence and capture
  again. Preserve all required server mods. A material/custom loader still
  unsupported by the adapter needs an adapter correction; export the report and
  the private client's refusal rather than accepting fallback cubes as success.
- Oversized context/output: narrow the affected area. Removing required mods to
  pass an input budget would break correspondence to the server.

Exact available helper targets remain Fabric 1.20.1/0.16.14 (Fabric API
0.92.5+1.20.1, Java 17), Forge 1.20.1/47.4.10 (Java 17), and NeoForge
1.21.1/21.1.251 (Java 21). Unsupported versions cannot be remedied by choosing
the nearest pin. Static resource rendering remains available on other supported
Java versions; universal mod/renderer support is not established.

## Private matching-client capture

This route currently accepts selected Prism instances and the exact helper pins
in `tools/java-map-export/production/README.md`; Fabric additionally requires
API `0.92.5+1.20.1`. Other versions receive
`matching_capture_helper_unavailable`; selecting a nearby version is not a remedy.
Source instances, their saves and the remote running server remain unchanged.

1. Export the selected client's resources with
   `msc map-capture export-resources --instance <instance> --output <new-resources.zip>`.
   Import that bundle through the existing world's map-assets import action and
   adopt its checked resource generation. Re-export older bundles to add the
   client configuration receipt required by capture preparation.
2. On the host, run `msc world map-assets capture-context` for the active server with the same
   `--slot`, `--expected-revision`, `--dimension`, `--min x y z`, `--max x y z`
   and `--output <new-context.zip>`. The selected area must already have an
   adopted, consistent saved scene. Copy this data archive to the client machine.
3. Run `msc map-capture prepare --instance <instance> --context <context.zip>
   --game-jar <matching-client.jar> --java <actual-java-executable>
   --destination <new-private-directory>`. Use one line in the terminal. The
   destination's parent must exist and the destination must be outside the
   source instance. Release binaries restore their checked helpers automatically;
   development builds can pass `--helpers <built-helper-directory>` explicitly.
   Memory follows the selected instance; `--memory-mib <MiB>` explicitly overrides it.
   Preparation copies inputs and verifies them; it does not launch Minecraft.
4. Launch Prism explicitly with `--dir <private-directory> --launch msc-capture`.
   This separate launcher directory has no copied account credentials. Sign in
   there if required, open the private `msc-capture` save, and navigate to the
   requested saved dimension/area. Only this working copy may be changed.
   Run `/mscmapcapture prepare`, then `/mscmapcapture export` after the saved
   context is loaded and restoration has completed. Follow any concrete refusal
   rather than retrying with changed version or unrelated world inputs.
5. The helper writes a unique ZIP under the private game's
   `.msc-map-capture/exports` and records its filename in `current`.
   Transfer that data-only ZIP to the host. Check its original affected area
   immediately before importing so the comparison uses those same saved blocks:

   ```sh
   msc world map-assets check --slot SLOT --expected-revision REVISION --dimension DIMENSION --min X0 Y0 Z0 --max X1 Y1 Z1
   msc world map-assets import CAPTURE_ZIP --slot SLOT --expected-revision REVISION --dimension DIMENSION --min X0 Y0 Z0 --max X1 Y1 Z1
   msc --json world map-assets report --slot SLOT
   ```

   Use the original request's binding and bounds, not guessed coordinates.
   `check` can return nonzero for the very rendering issues being repaired;
   inspect its completed report rather than chaining it with `&&`. The import
   command stages/hashes the local ZIP and waits for adoption unless `--no-wait`
   is selected. Resource-only imports can omit `--min`/`--max` as before; capture
   imports should supply both. The host checks saved correspondence again before
   adoption. Expired/changed inputs require fresh context and preparation.

The desktop provides explicit preparation, launch, cancellation,
output inspection and reopening retained output as described above. Cancellation retains private files if process termination
cannot be confirmed. Never run the capture helper in the original instance.

Checked captures may be reused after restart only when binding, inputs, saved
snapshot and area still match; every cache hit is fully validated. A cache miss
or corrupt cache leaves baseline rendering available. Cached geometry is a
saved frame, not live animation. Compare the real difficult block and its
neighbors with Minecraft before recording acceptance.

After adoption, the actual map labels saved capture time/tick. Rechecks count
captured_appearance only for validated positions; other unresolved blocks remain
unresolved. Compare shape, contents, transparency and neighboring terrain with
Minecraft before confirming success. Older viewers get an upgrade explanation
and keep their prior scene. The full format/bounds are in the renderer design.

## Historical P18.15 inspection instructions

The following records the earlier inspection-only stage. Current capture and
adoption are described above; the isolated P18.14 proof remains separate.

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

For owner acceptance, export the scoped report and rendering JSON before/after
an actual repair, retain the exact package and fixture identities, and capture
the named blocks in MSC and Minecraft. Follow the private session format and
commands in [the evidence collector README](../../tools/world-map-assets/README.md).
The collector only records existing evidence. Build receipts leave rendering
pending; visual confirmation requires Cameron’s actual comparison. The draft
Forge helper has not yet provided a production export/import/adoption remedy.
