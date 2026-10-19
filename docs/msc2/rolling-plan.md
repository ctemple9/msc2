# MSC 2 — Rolling Plan

**Current priority:** P20.12 corrects imported Java JAR presence/version reporting;
awaiting owner verification. Earlier Phase 20 steps remain accepted and archived.
Phase 21 P21.1–P21.37 implementation steps were closed at Cameron’s October 17
request and moved to [the archive](rolling-plan-archive.md#phase-21--storage-inventory-and-management).
Physical acceptance and the independent Phase 21 gate review remain in
[pending acceptance](pending-acceptance.md).
Independent Phase 20 gate review remains in [pending acceptance](pending-acceptance.md).

**Add Server follow-up:** P12.190 keeps the Add Server wizard at one height
across steps and scrolls longer step content; awaiting owner verification.

**Bedrock map follow-up:** P18.56's 16,384-file limit did not resolve the
large converted world's Intel Mac failure; P18.57 supersedes that limit.
P18.57 confirmed the installed exporter received 65,536 files, but its catalog
still failed without a new file error. P18.58's sixty-minute catalog deadline
also expired on the Intel Mac. P18.59 moves first map opening to a bounded
spawn-area catalog and builds wider coverage separately. P18.60 reuses one
read-only world handle across tile requests after the local 7.5 GB copy showed
roughly one minute per tile. P18.61 corrects its reply channel after owner
verification found no tiles appearing; awaiting owner verification.
P18.62 adds saved-chunk discovery around the moving Bedrock map camera;
awaiting owner verification before the next release.
P18.63 adds a fast surface preview and batches detailed tile reads. It keeps
map discovery near the camera instead of running a competing full-world scan.
Owner verification found its tile revision values rejected by the viewer;
P18.64 corrects that manifest format and awaits owner verification on the
imported 7.5 GB world.
P18.65 addresses the owner's follow-up that nearby Bedrock terrain remains
blank when the camera stays near a discovery-region boundary; awaiting owner
verification.
P18.66 corrects stretched cliff textures in the fast Bedrock surface preview;
awaiting owner verification.
P18.67 removes the preview's tall leaf walls while keeping the quick surface
sampling path; awaiting owner verification.

**Services follow-up:** P21.39 makes host-wide Playit account setup available
from MSC Settings even when the selected server does not use Playit. P21.40
moves Xbox Broadcast setup to the selected server’s Services tab. Both await
Cameron’s manual verification.
P21.41 reorders those server controls; it also awaits manual verification.
P21.42 shows the shared Playit agent and saved tunnel addresses in MSC Settings;
it awaits manual verification.
P21.43 shows those saved addresses in the active server’s Services tab;
it awaits manual verification.
P21.44 limits that server view to tunnels its own configuration uses;
it awaits manual verification.
P21.45 hides server Playit addresses until the owner reveals them;
it awaits manual verification.

Historical implementation and published release records were reconciled and
moved to [the archive](rolling-plan-archive.md) at Cameron's October 16 request.
Outstanding physical checks and independent reviews live in
[pending acceptance](pending-acceptance.md). Unimplemented connected-host server
transfers remain in [deferred work](deferred-work.md). Neither link is an active
step queue, and no phase gate was closed by cleanup.

**Recent owner acceptance:** Phase 20 P20.1–P20.10 closed and archived by owner
direction after local native-import acceptance; see [acceptance](uploads/acceptance.md).
P16.89 XboxBroadcast works on the affected
Intel Mac with RakNet retained. Publication records live in the archive and
[release notes](release/v0.1.30.md); live publication status belongs on GitHub,
not in this implementation queue.

## Phase 12 — Add Server wizard follow-up

### P12.190 — Keep the Add Server wizard at one size

**Status:** Implemented; awaiting owner verification.
**Files:** `clients/desktop-web/src/lib/components/base/Sheet.svelte`; `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`; this plan.
**What:** Give the Add Server sheet a steady 820 × 740 px frame, capped to the viewport. Keep its progress row and navigation footer visible while each step scrolls inside the body. Hide the body scrollbar and return to the top on step changes. Leave other sheets at their existing natural heights.
**Verify:** Run `npx tauri dev`; open Manage Servers → Add Server and move through Fresh and Import steps. The sheet should keep the Configure step’s size, longer steps should scroll without a visible scrollbar, and Back/Continue should stay visible. Check at a shorter window height that the sheet fits and its content remains reachable.
**Batch:** P12.190 only.
**Commit:** `P12.190: keep add server wizard at one size`

## Phase 18 — Bedrock map follow-up

### P18.56 — Allow large Bedrock maps to open more LevelDB tables

**Status:** Implemented; owner verification failed on the large Intel Bedrock world; superseded by P18.57.
**Files:** `tools/world-map-proof/Cargo.toml`; `tools/world-map-proof/Cargo.lock`; `tools/world-map-proof/src/main.rs`; this plan.
**What:** The Intel Mac’s agent log shows the Bedrock map exporter failing on a converted 7.3 GB world with “Too many open files (os error 24)” while reading LevelDB tables. The reader retains open table handles during catalog generation, while launchd starts the agent with a 256-file soft limit. On macOS, raise only the short-lived exporter’s soft limit to at most 16,384, bounded by the inherited hard limit, before it opens the world. Keep a log diagnostic if the OS rejects the increase. Do not change world data or the agent service’s limits. No new test: this requires the owner’s large-world Intel acceptance and an artificial small-file-limit unit test would not exercise the real table scan.
**Verify:** Install an Intel build containing this step, then open the converted Bedrock world’s map. Confirm the catalog loads and the agent log no longer reports “Too many open files”; if it still fails, provide the new exporter error and the service’s hard file limit.
**Batch:** P18.56 only.
**Commit:** `P18.56: raise bedrock map exporter file limit`

### P18.57 — Match the successful large-world file limit

**Status:** Implemented; owner verification confirmed the 65,536-file limit but the catalog still failed; superseded by P18.58.
**Files:** `tools/world-map-proof/src/main.rs`; this plan.
**What:** Cameron's 7.5 GB converted Bedrock snapshot completed the same exporter catalog command from Terminal with a 65,536-file soft limit. The installed exporter requested only 16,384 and its agent-run catalog still failed with “Too many open files.” On macOS, request up to 65,536 for the short-lived exporter, bounded by the inherited hard limit. Log the effective soft and hard limits for catalog runs so the Intel check can distinguish an insufficient limit from a launchd inheritance problem without flooding per-tile logs. No new test: the decisive case is the owner's physical scan of this large world, and a synthetic small-limit test would not exercise its LevelDB table load.
**Verify:** Run `open "$HOME/Downloads/msc2-bedrock-map-intel-p18.57.dmg"` on the Intel Mac, install and repair the agent, then open Bedtonic's map. Confirm it loads and the agent log reports a 65,536 soft limit; if it fails, provide that limit line and the new exporter error.
**Batch:** P18.57 only.
**Commit:** `P18.57: raise bedrock map exporter limit to 65536`

### P18.58 — Allow the large Bedrock catalog to finish

**Status:** Implemented; owner verification failed on the large Intel Bedrock world; superseded by P18.59.
**Files:** `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`; this plan.
**What:** The same 7.5 GB Bedrock snapshot's direct catalog scan was still using a full CPU core after fifteen minutes and then completed. The agent stops catalog exporters after ten minutes, which fits the installed build's generic catalog failure even though the exporter logged an effective 65,536-file limit and no new LevelDB error. Allow initial and refreshed Bedrock catalogs up to sixty minutes while preserving the ten-minute limit for individual tile and comparison exports. Keep shutdown cancellation and log a distinct timeout if the longer limit is reached. No new test: a short synthetic timing case would not represent the large-world scan, and Cameron will run the physical acceptance.
**Verify:** Run `open "$HOME/Downloads/msc2-bedrock-map-intel-p18.58.dmg"` on the Intel Mac, install and repair the agent, then open Bedtonic's map and allow the catalog to finish. Confirm the overview loads; if it still fails, provide the recent agent log and elapsed time.
**Batch:** P18.58 only.
**Commit:** `P18.58: allow large bedrock catalogs to finish`

### P18.59 — Open nearby Bedrock terrain before whole-world indexing

**Status:** Implemented; awaiting owner verification.
**Files:** `tools/world-map-proof/src/main.rs`; `tools/world-map-proof/src/render.rs`; `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`; `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`; this plan.
**What:** The Intel Mac's exporter received 65,536 open files but reached the full catalog's sixty-minute deadline. For an Overworld's first map request, query only saved chunks within eight chunks of the configured spawn and write a valid nearby manifest without a whole-world overview. Return that map first, then index the complete saved snapshot in a cancellable background exporter. Hold the snapshot until it finishes, atomically replace the in-memory catalog only if that snapshot is still selected, and let an open viewer reload its full manifest while retaining its camera position. Remove the background job's arbitrary deadline; cancel it on snapshot release or replacement. Log snapshot, index, and overview durations and periodic large-scan progress. Keep Nether and End catalog behavior unchanged. No new test: a tiny fixture cannot represent this multi-gigabyte LevelDB workload, and the owner's Intel Mac is the acceptance machine.
**Verify:** Run `open "$HOME/Downloads/msc2-bedrock-map-intel-p18.59.dmg"` on the Intel Mac, install and repair the agent, then open Bedtonic's Overworld map. Nearby terrain should load without waiting for whole-world indexing. The agent log should report `catalog-nearby indexed`, `nearby ... catalog ready`, and later full-index/overview progress; when the full catalog completes, the viewer should expand coverage without changing the selected world or camera position. If nearby loading fails, provide the new `Bedrock map:` log lines.
**Batch:** P18.59 only.
**Commit:** `P18.59: open nearby bedrock terrain before full catalog`

### P18.60 — Reuse the Bedrock world handle across terrain tiles

**Status:** Implemented; owner verification found no tiles appearing; superseded by P18.61.
**Files:** `tools/world-map-proof/src/main.rs`; `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`; this plan.
**What:** On Cameron's local copy of the 7.5 GB converted world, the nearby catalog took 50.9 seconds, of which 15.4 seconds indexed 289 nearby chunks. Each subsequent tile took about 50–66 seconds because the agent started a new exporter that reopened the large LevelDB before rendering. Add a tile helper that opens the read-only snapshot once and answers successive tile requests over pipes. Reuse it only while its snapshot and catalog remain selected; stop it on refresh, selection change, or release, and let it exit after two idle minutes. Preserve the ten-minute per-tile timeout and existing full-catalog background behavior. No new test: a small fixture cannot represent the large-world open cost; Cameron's imported copy provides the decisive physical check. Do not run a test without his specific instruction.
**Verify:** Install `~/Desktop/msc2-bedrock-map-intel-p18.60.dmg`, repair the agent, and open the imported Bedtonic-world map. After the first tile, later nearby tiles should no longer each take about a minute. Compare the `[world-map] Bedrock ... tile ... ready after` lines in `~/Library/Application Support/MSC 2/logs/agent.log` and confirm terrain fills progressively without a tile error.
**Batch:** P18.60 only.
**Commit:** `P18.60: reuse bedrock world handle across map tiles`

### P18.61 — Keep Bedrock tile helper replies separate from diagnostics

**Status:** Implemented; awaiting owner verification.
**Files:** `tools/world-map-proof/src/render.rs`; this plan.
**What:** The persistent tile helper finished rendering a tile but printed render diagnostics to standard output before its `ok` reply. The agent read the diagnostic as an invalid reply, discarded the rendered tile, and restarted the helper. Send render diagnostics to standard error so standard output carries only tile-session replies. No new test: the owner's imported 7.5 GB world is the decisive check, and tests are not authorized to run.
**Verify:** Install `~/Desktop/msc2-bedrock-map-intel-p18.61.dmg`, repair the agent, then reopen the imported Bedtonic-world map. Terrain tiles should appear, and subsequent tiles should avoid reopening the world. Check `~/Library/Application Support/MSC 2/logs/agent.log` for `[world-map] Bedrock ... tile ... ready after` lines without new tile failures.
**Batch:** P18.61 only.
**Commit:** `P18.61: reserve bedrock tile helper stdout for replies`

### P18.62 — Load saved Bedrock terrain near the camera

**Status:** Implemented; awaiting owner verification.
**Files:** `tools/world-map-proof/src/main.rs`; `crates/msc-agent/src/routes/worlds/map_terrain.rs`; `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`; `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`; this plan.
**What:** The first Bedrock catalog advertises only 25 spawn-area tiles while the full-world overview is still building. Let the existing read-only tile helper query saved chunks in a bounded 16 × 16-chunk region around the camera, cache inspected regions for the selected snapshot, and add only discovered tiles to the dynamic manifest. Poll the camera position while that nearby catalog is active; Vantage streams the new tiles into the open viewer. Keep the camera and flight state when the full catalog eventually replaces the nearby one. No new test: a small fixture would not represent this converted 7.5 GB world, and Cameron's imported copy is the physical acceptance case. Do not run a test without specific owner instruction.
**Verify:** Install the local P18.62 candidate, repair the agent, open the imported Bedtonic-world map, then fly beyond the original 25 tiles to another area known to contain saved terrain. New tiles should appear around the camera without reopening the map; an unsaved area should remain empty. Check `~/Library/Application Support/MSC 2/logs/agent.log` for `discovered ... saved tiles near block` and tile-ready lines. Confirm that flight stays active if the full catalog completes.
**Batch:** P18.62 only.
**Commit:** `P18.62: load saved bedrock tiles near the camera`

### P18.63 — Stream Bedrock surface previews before detailed tiles

**Status:** Implemented; awaiting owner verification.
**Files:** `tools/world-map-proof/src/render.rs`; `tools/world-map-proof/src/main.rs`; `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`; `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`; this plan.
**What:** On the converted 7.5 GB Bedrock world, a detailed tile took 8–27 seconds and the full-world overview used the same disk and CPU in the background. Query each tile's 16 chunks in one exact-key batch, cache block visibility classification, and skip fully enclosed solid blocks before modeling. Return a surface-only preview first; after preview requests settle, refine cached preview tiles nearest the camera and replace them in the viewer using tile revisions. Keep the shared texture atlas count current as materials are added. Discover saved terrain around the moving camera without an automatic full-world scan; retain rendered previews and details for the selected snapshot so revisiting them does not rerender. Let the viewer remember explored Bedrock tiles after they leave its memory budget, since the full-world overview no longer supplies that context. Log preview time and detailed read/mesh time separately. The five-second target is a physical performance goal, not a claim established by static checks. No new test: the owner’s large converted world is the acceptance case, and tests were not authorized to run.
**Verify:** Open `~/Desktop/msc2-bedrock-map-intel-p18.63.dmg`, install the app, repair the agent, then open the imported Bedtonic-world map. Surface tiles should appear before full 3D detail and refine near the camera without resetting flight. Fly to another saved area and back; returning tiles should load from the current snapshot cache. In `~/Library/Application Support/MSC 2/logs/agent.log`, compare `surface preview` and `detail ... read ..., mesh ...` times with the former 8–27 seconds per tile. Confirm normal viewing no longer starts a full-world catalog. Report preview and detail times from the Intel Mac to assess the under-five-second goal.
**Batch:** P18.63 only.
**Commit:** `P18.63: stream bedrock map surface previews`

### P18.64 — Send valid Bedrock tile revisions to the viewer

**Status:** Implemented; awaiting owner verification.
**Files:** `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`; this plan.
**What:** Vantage rejects numeric manifest tile revisions before it loads any terrain. Send nonempty string revisions for both initially listed and newly discovered tiles, changing each tile from `preview` to `detail` when its detailed mesh replaces the surface preview. The imported 7.5 GB world remains the physical acceptance case; no test was added or run without owner authorization.
**Verify:** Install `~/Desktop/msc2-bedrock-map-intel-p18.64.dmg`, repair the agent, and reopen the imported Bedtonic-world map. The `invalid revision` error should be gone; surface previews should appear and refine into detail near the camera. Fly to another saved area and back, then compare preview and detail times in `~/Library/Application Support/MSC 2/logs/agent.log` with the earlier 8–27 seconds per tile.
**Batch:** P18.64 only.
**Commit:** `P18.64: send valid bedrock tile revisions`

### P18.65 — Discover saved Bedrock terrain around the visible camera

**Status:** Implemented; awaiting owner verification.
**Files:** `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`; `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`; this plan.
**What:** The Bedrock map previously checked only the 256-block region containing the camera. A stationary camera near a boundary could see blank adjacent regions indefinitely. Inspect one unvisited region from the surrounding 3 × 3 neighborhood on each camera poll, nearest first, and poll every two seconds instead of ten. Let discovery and surface previews take precedence over detailed mesh refinement; only camera-aware polls refine details, and preview requests no longer replace the actual camera position with a tile center. Region results remain cached for the selected snapshot. This improves nearby coverage without repeating the earlier hour-long full-world catalog scan. No test was added or run without owner authorization.
**Verify:** Install `~/Desktop/msc2-bedrock-map-intel-p18.65.dmg`, repair the agent, then reopen the imported Bedtonic-world map. Hold the camera near a previously blank region edge for at least a minute. Saved terrain across the edge should appear without moving the camera into it. Fly to another saved area and stop; nearby terrain should continue filling before its detailed meshes. The agent log should show `discovered ... in region` lines around each stationary camera position and preview timings; report any persistent blank area with the camera XYZ and recent discovery/preview log lines.
**Batch:** P18.65 only.
**Commit:** `P18.65: discover bedrock terrain around camera`

### P18.66 — Keep Bedrock preview cliff textures at block scale

**Status:** Implemented; awaiting owner verification.
**Files:** `tools/world-map-proof/src/render.rs`; this plan.
**What:** The surface preview uses one tall quad per exposed cliff face but previously applied one block's texture coordinates to its full height. Repeat side textures once per vertical block and keep grass sides to the uppermost block, with dirt beneath in the provisional mesh. Detailed tiles still replace the preview with actual saved blocks. No test was added or run; the imported converted world is the physical acceptance case.
**Verify:** Install the local P18.66 candidate, repair the agent, then reopen or refresh the imported Bedtonic-world map. Fly near the cliff shown in the owner's screenshot while the status still shows preview tiles. Cliff textures should have block-sized detail rather than stretched vertical bands; grass should be confined to the surface layer. Confirm detailed tiles still replace previews as they finish.
**Batch:** P18.66 only.
**Commit:** `P18.66: repeat bedrock preview cliff textures`

### P18.67 — Keep Bedrock preview foliage to sampled leaves

**Status:** Implemented; awaiting owner verification.
**Files:** `tools/world-map-proof/src/render.rs`; this plan.
**What:** The fast preview knows only the highest leaf block in each column, so its former height-difference side faces invented leaf walls down to nearby lower terrain. Limit preview leaf geometry to that one sampled block and add its underside so a camera beneath the canopy sees it. Detailed tiles continue to supply actual trunks and full foliage. No test was added or run; the imported converted world is the physical acceptance case.
**Verify:** Install the local P18.67 Intel candidate, repair the agent, reopen or refresh the imported Bedtonic-world map, and fly near trees while preview tiles are still loading. Tree foliage should no longer form tall square curtains reaching the ground. As detailed tiles replace the previews, their complete trunks and canopies should appear.
**Batch:** P18.67 only.
**Commit:** `P18.67: bound bedrock preview foliage to sampled leaves`

## Phase 20 — Imported server component follow-up

P20.1–P20.10 remain archived and owner-accepted. This later reported correction
is separate from that closure; no earlier unobserved check is promoted to Pass.

### P20.12 — Detect installed imported Java server JAR versions

**Status:** Implemented; awaiting owner verification.
**Files:** `crates/msc-agent/src/routes/components.rs`; `crates/msc-application/src/import.rs`; `crates/msc-infrastructure/src/java_jar_metadata.rs` and module registration; candidate evidence under `uploads/`; this plan.
**What:** Check configured JAR presence independently of version fields. Read small known JAR metadata entries offline, recovering the reported Paperclip Minecraft version from root `version.json`, plus explicit server-manifest version/build when available. Persist recovered metadata on future raw imports; existing imports detect it when Components loads without re-import or data mutation. Existing unreadable/unknown metadata shows installed/version unknown; an absent JAR stays missing even with stale config metadata. Do not invent an online update verdict, run Java or download/replace the JAR. Inspect off the async API worker. Add three essential regression cases covering the reported Paperclip layout, dependency-version confusion/oversize metadata, and real component presence versus stale version fields. Tiny memory ZIPs/isolated files, no timing/network dependencies, expected under one second each excluding compilation; compile only, do not run tests. Prepare a matching optimized Intel candidate.
**Verify:** `cargo build --locked -p msc-agent --bin msc`
**Batch:** P20.12 only.
**Commit:** `P20.12: detect versions of imported server jars`

## Phase 21 — Services settings follow-up

### P21.39 — Decouple service account setup from the selected server

**Status:** Implemented; awaiting owner verification.
**Files:** `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`; `clients/desktop-web/src/lib/sections/server-editor/PlayitSetupSheet.svelte`; `clients/desktop-web/src/lib/sections/server-editor/BroadcastAuthSheet.svelte`; `clients/desktop-web/src/App.svelte`; `crates/msc-api/src/dto/networking.rs`; `crates/msc-agent/src/routes/networking.rs`; this plan.
**What:** Allow Playit setup from MSC Settings to claim or reuse the host account without creating tunnels or changing any server. Let Xbox Broadcast account sign-in use an eligible server on the host without selecting it in the app. Both Settings buttons remain available when the selected server does not use the service. Xbox Broadcast still needs an eligible server because its helper and sign-in state are server-scoped.
**Verify:** Run `npx tauri dev`; with a server selected that uses neither service, open MSC Settings → Services and confirm both setup buttons respond. Set up Playit and confirm the selected server’s service settings remain unchanged. If another server has Xbox Broadcast enabled, use Xbox sign-in and confirm the selected server remains unchanged.
**Batch:** P21.39 only.
**Commit:** `P21.39: decouple service account setup from selected server`

### P21.40 — Put Xbox Broadcast setup with the server

**Status:** Implemented; awaiting owner verification.
**Files:** `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`; `clients/desktop-web/src/lib/sections/server-editor/BroadcastTab.svelte`; `clients/desktop-web/src/lib/sections/server-editor/BroadcastAuthSheet.svelte`; `crates/msc-agent/src/routes/networking.rs`; this plan.
**What:** Remove Xbox Broadcast from global MSC Settings. Put its Microsoft device-code setup, helper JAR management, and autostart control in the active server’s Services tab beside its enable and runtime controls. Remove the email, gamertag, and password fields because the helper’s device-code sign-in does not consume them. Retire the now-unused cross-server Settings setup routes. Keep Playit account setup in global Settings.
**Verify:** Run `npx tauri dev`; confirm MSC Settings → Services has no Xbox Broadcast section, then open a server’s Services tab and use Xbox Broadcast → Set up. The device-code prompt must appear for that server, and selecting another server must not be required.
**Batch:** P21.40 only.
**Commit:** `P21.40: move xbox broadcast setup to server services`

### P21.41 — Reorder Xbox Broadcast server controls

**Status:** Implemented; awaiting owner verification.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/BroadcastTab.svelte`; this plan.
**What:** Show Xbox Broadcast runtime status and Start/Stop first, followed by the server enable and automatic-start toggles, Microsoft sign-in, then Helper JAR download. Remove the Join address row from this card and update the enable hint for the new order.
**Verify:** Run `npx tauri dev`; open a server’s Services tab and confirm the Xbox Broadcast rows appear in that order and each control still works as before.
**Batch:** P21.41 only.
**Commit:** `P21.41: reorder xbox broadcast service controls`

### P21.42 — Show shared Playit agent and tunnels in MSC Settings

**Status:** Implemented; awaiting owner verification.
**Files:** `crates/msc-api/src/dto/networking.rs`; `crates/msc-agent/src/routes/networking.rs`; `crates/msc-agent/tests/playit_routes.rs`; `docs/msc2/api-contract/openapi.json`; `clients/desktop-web/src/lib/api/generated.ts`; `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`; this plan.
**What:** Include the saved host Playit agent ID and Java, Bedrock, and voice tunnel addresses in status even when no server is active. Show each configured tunnel on its own labeled row in global Services settings, preserving the full provider address and port. This is read-only and does not create tunnels or change server participation.
**Verify:** Run `npx tauri dev`; open MSC Settings → Services after Playit setup and confirm the agent ID and each configured Java, Bedrock, and voice address appear on separate rows, including with no server selected. Missing tunnels should have no address row.
**Batch:** P21.42 only.
**Commit:** `P21.42: show shared playit agent and tunnels`

### P21.43 — Show Playit tunnel addresses in server Services

**Status:** Implemented; awaiting owner verification.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/BroadcastTab.svelte`; this plan.
**What:** When Playit is enabled for the active server, show the saved shared Java, Bedrock, and voice addresses as separate labeled rows in its Services tab. Preserve the Simple Voice Chat restart hint when a voice address exists and the server is running. Do not create or change tunnels.
**Verify:** Run `npx tauri dev`; open a Playit-enabled server’s Manage → Services tab and confirm every configured shared tunnel address appears on its own row. Disable Playit for that server and confirm the address rows disappear without changing MSC Settings’ shared addresses.
**Batch:** P21.43 only.
**Commit:** `P21.43: show playit tunnel addresses in server services`

### P21.44 — Filter Playit addresses to the selected server’s tunnels

**Status:** Implemented; awaiting owner verification.
**Files:** `crates/msc-api/src/dto/networking.rs`; `crates/msc-agent/src/routes/networking.rs`; `crates/msc-agent/tests/playit_routes.rs`; `docs/msc2/api-contract/openapi.json`; `clients/desktop-web/src/lib/api/generated.ts`; `clients/desktop-web/src/lib/sections/server-editor/BroadcastTab.svelte`; this plan.
**What:** Report which of the shared Java, Bedrock, and voice tunnel kinds the active server actually uses, following the existing backend tunnel rules. Display only those configured addresses in that server’s Services tab; MSC Settings continues to show the full shared host inventory.
**Verify:** Run `npx tauri dev`; open a Playit-enabled Bedrock-only server and confirm only its Bedrock tunnel appears. Check a Java server, a Java server with Bedrock cross-play, and a server with Simple Voice Chat to confirm each sees only its applicable tunnel addresses.
**Batch:** P21.44 only.
**Commit:** `P21.44: filter server playit tunnel addresses`

### P21.45 — Hide Playit addresses in server Services until revealed

**Status:** Implemented; awaiting owner verification.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/BroadcastTab.svelte`; this plan.
**What:** Add an eye control immediately left of Playit Manage setup. Tunnel rows remain labeled but show “Hidden” until the eye is selected. Re-hide on server change or when the server is no longer active. This changes only on-screen visibility, not Playit networking.
**Verify:** Run `npx tauri dev`; open a Playit-enabled server’s Services tab and confirm its tunnel addresses start hidden, the eye reveals and hides them, and selecting another server starts hidden again.
**Batch:** P21.45 only.
**Commit:** `P21.45: hide server playit addresses by default`

### P21.46 — Hide shared Playit details in MSC Settings until revealed

**Status:** Implemented; awaiting owner verification.
**Files:** `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`; this plan.
**What:** Add an eye control immediately left of Playit Manage setup in MSC Settings. Keep the agent ID and configured tunnel rows labeled, but show “Hidden” until the eye is selected. Re-hide when the settings sheet is reopened. This changes only on-screen visibility.
**Verify:** Run `npx tauri dev`; open MSC Settings → Services and confirm the Playit agent ID and tunnel addresses start hidden, the eye reveals and hides them, and reopening MSC Settings starts hidden again.
**Batch:** P21.46 only.
**Commit:** `P21.46: hide shared playit details by default`
