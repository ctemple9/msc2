# MSC 2 — Rolling Plan

**Current priority:** P20.12 corrects imported Java JAR presence/version reporting;
awaiting owner verification. Earlier Phase 20 steps remain accepted and archived.
Phase 21 P21.1–P21.37 implementation steps were closed at Cameron’s October 17
request and moved to [the archive](rolling-plan-archive.md#phase-21--storage-inventory-and-management).
Physical acceptance and the independent Phase 21 gate review remain in
[pending acceptance](pending-acceptance.md).
Independent Phase 20 gate review remains in [pending acceptance](pending-acceptance.md).

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
