# MSC 2 — Rolling Plan

**Current priority:** P20.12 corrects imported Java JAR presence/version reporting;
awaiting owner verification. Earlier Phase 20 steps remain accepted and archived.
Phase 21 P21.1–P21.37 implementation steps were closed at Cameron’s October 17
request and moved to [the archive](rolling-plan-archive.md#phase-21--storage-inventory-and-management).
Physical acceptance and the independent Phase 21 gate review remain in
[pending acceptance](pending-acceptance.md).
Independent Phase 20 gate review remains in [pending acceptance](pending-acceptance.md).

**Services follow-up:** P21.39 makes host-wide service account setup available
from MSC Settings even when the selected server does not use that service.
It awaits Cameron’s manual verification.

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
