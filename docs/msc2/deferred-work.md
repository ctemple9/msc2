# Deferred work

These owner-requested plans remain available but are outside the active upload
priority. Moving them here does not cancel them, mark implementation complete,
or authorize execution. Return them to the rolling plan when Cameron selects
them. This is separate from completed history and pending acceptance.

## Complete connected-host server transfers

**Current disposition (October 16, 2026):** P18.40 planning is recorded;
P18.41–P18.45 remain unimplemented. Preserve their complete-server, stopped-source,
resume and destination-registration contract. Phase 20 improves local file
imports and uploads; it does not claim this host-to-host feature is implemented.
The original plan and its historical observations follow.

### P18.40 — Plan complete server transfers

**Status:** Plan written; awaiting Cameron's review. Cameron changed this conversation to PLAN only before implementation began (2026-10-07); the earlier BATCH EXECUTE authorization is superseded.
**Files:** `docs/msc2/rolling-plan.md` only.
**What:** Record P18.41–P18.45 for complete selected-server transfers, with remote-to-local Tectonic acceptance as the first required outcome. No application code, tests, builds, agent installation, releases or live server actions in this conversation.
**Verify:** `git show --stat --oneline HEAD`
**Batch:** P18.40 only (PLAN). Future execution requires a new owner instruction naming its range.
**Commit:** `P18.40: plan complete server transfers`

**Owner-required outcome:** From the desktop connected to the Xubuntu agent, select Tectonic and transfer the entire server to the local agent. On Fedora, macOS or Windows, switch to the local agent, select Tectonic in the normal server dropdown, repoint Java and review port allocation, then manage it through the existing MSC screens. The same transfer mechanism supports other paired destinations and the reverse direction; direct agent-to-agent network reachability is not required. Java-to-Bedrock conversion is the motivation, but conversion implementation is outside these transfer steps.

**Product flow:** Manage Servers → Transfer → choose server and destination (default: This computer for a remote source) → review size, destination space and stop requirement → Stop and transfer, or Transfer if already stopped → byte/file progress with Cancel → completion showing Open on destination and Review Java and ports. Stop is graceful and explicit; no forced stop or automatic destination start. Closing the sheet or navigating between hosts preserves the transfer/progress. The copied server receives a new destination identity and a noncolliding folder; the source remains available. Deleting the source is a separate existing action after owner verification.

**Architecture:** The source agent inventories and reads the server; the destination agent stages, validates and registers it. The native desktop relays bounded chunks through the two existing authenticated connections. File bytes never become a whole-world browser buffer or a desktop temporary archive. The source needs only small session metadata; the destination needs approximately one complete server copy plus small metadata and reserved headroom, because finalization renames staging on the same filesystem. Size means the entire server, including backups and inactive world slots, so Tectonic's 13 GB world is a lower bound. Show conversion scratch-space needs separately from transfer space where already known; do not imply that transfer capacity guarantees conversion capacity.

**Complete-server boundary:** Include all ordinary files and empty directories inside the selected server: live Java dimensions or Bedrock worlds; world slots and their profiles/pack state; backups; mods/plugins; loader libraries and launch files; resource/behavior packs; configuration, player data and existing logs; and MSC server metadata, component provenance and modpack identity. Preserve unknown files and config keys. Map only explicit server-owned references, including a configured Java server JAR outside the server directory; do not follow arbitrary external paths. The destination's Java installation, OS service, agent identity and host credentials remain destination-owned. Re-root server-owned paths and identify source-specific helper/account settings for review without copying agent pairing, keychain/DPAPI entries, or host helper credentials. Check for copied third-party native executables and absolute loader paths: preserve their files but report platform dependencies accurately. A Bedrock copy across operating systems uses the destination's supported runtime/provisioning path; copying a Linux executable alone does not make a Windows/macOS server launchable.

**Transfer contract:** Versioned capability discovery; explicit source/destination/server identity; inventory with relative paths, lengths and file digests; credential-bound sessions; bounded chunk sizes; idempotent acknowledged writes; progress, cancellation, completion and cleanup. Validate traversal, symlink escape, Windows reserved filenames, case collisions and path limits before copying. Reject unsupported entries with their paths and an actionable explanation, never silently omit them. Refuse running-server reads until graceful stop completes. Hold existing server-operation exclusivity during the source session and destination commit, including start, settings/file edits, world operations, backups and host reset. Inspect actual admission paths rather than assuming every edit already uses the journal. Session idle expiry must refresh during reads, hashing and progress, so a large server cannot expire while working normally.

**Integrity and interruption:** Hash files with bounded reads, validate completed destination files and total inventory, and verify that the stopped source has not changed before finalization. Detect same-size edits; length alone is insufficient. Retain destination checkpoints so transient disconnects and desktop/agent restarts can resume only after authenticating both hosts and confirming the same source inventory. Changed sources require a new session; do not splice two snapshots. Cancel removes only this session's unpublished staging and releases source locks. On restart, reconcile unfinished sessions and journals explicitly. A failed transfer cannot appear as a usable server in the dropdown. Rename and configuration registration need recoverable commit bookkeeping: failure between those operations must neither lose received data nor register duplicates. Report cleanup failures and reclaim interrupted staging through an explicit owned action.

**Observed gaps:** Existing `POST /v1/servers/export` stages an archive on the source and exports every configured server. `TransferSheet.svelte` downloads it as one byte array and imports from an agent-local path; it does not implement connected-host transfer. The existing transfer importer copies a selected file/subdirectory list and contains best-effort restoration paths, so it cannot be reused unchanged as proof of a complete, verified transfer. The native desktop already has per-host credentials, local bootstrap and managed SSH forwarding. Preserve those security boundaries and keep both transport sessions alive during relay. The MSC 1 source path supplied by AGENTS.md is absent on this machine; use the existing port and format documentation as evidence, and do not claim a fresh Swift-source comparison.

**Feature acceptance gate:** Cameron transfers the real stopped Tectonic server from Xubuntu to the local Fedora agent without a source archive or another 13 GB source copy; it appears in the local dropdown and its existing worlds, slots, components and settings are available. After Java selection and port review it starts through MSC. Record equivalent macOS and Windows destination acceptance before claiming those platforms physically verified. Exercise cancellation, insufficient destination space, interrupted/resumed transfer, source modification and interrupted finalization with controlled inputs. Preserve the original server. A successful build does not close this gate. The independent gate review belongs to the other agent, per the repository loop.

### P18.41 — Define transfer sessions and stream a stopped source server

**Status:** Planned.
**Files:** `docs/msc2/api-contract/openapi.json`; transfer DTOs under `crates/msc-api/src/dto/`; new `crates/msc-agent/src/routes/server_transfer.rs`; `crates/msc-agent/src/routes/mod.rs`, `main.rs`, `lifecycle.rs` and mutation admission callers as needed; `crates/msc-application/src/operations.rs`; capability discovery; generated client API types; this plan.
**What:** Freeze the complete-server contract above against current ConfigServer/world/loader behavior. Add capability/version discovery and source inventory/read/end routes scoped to the initiating credential and selected server. Stream file hashes and chunks without a source archive. Inventory external configured JARs through an explicit safe mapping. Validate portable names and reject unsupported entries. Add explicit graceful-stop consent and preserve stopped-state/source-operation guards through hashing and transfer. Persist session identity and source inventory for restart reconciliation and safe resume; audit existing settings/file/lifecycle mutation admission and close transfer races. Expose meaningful errors for older agents, permissions, changed sources and unsupported layouts. Update the supported-client capability record; retained host-local CLI session inspection/cancellation must respect D-040 rather than introducing remote CLI credentials.
**Verify:** `cargo clippy -p msc-api -p msc-agent --bin msc -- -D warnings`
**Batch:** P18.41 only; Cameron verifies before destination work unless a later explicit batch instruction authorizes P18.41–P18.45.
**Commit:** `P18.41: define complete server transfer sessions`

### P18.42 — Receive, verify and register complete servers

**Status:** Planned.
**Files:** `crates/msc-agent/src/routes/server_transfer.rs`; destination transfer persistence/adapters as needed under `crates/msc-application/src/` and `crates/msc-infrastructure/src/`; `crates/msc-agent/src/routes/lifecycle.rs` and startup recovery; API contract/DTOs as needed; this plan.
**What:** Add destination preflight, session creation, checkpoint queries, bounded idempotent chunk writes, final verification, cancel and recovery. Check full-server space plus headroom, and check writes for space exhaustion throughout. Write private staging on the destination server filesystem and reject symlink/case/path collisions. Resume only matching inventories. Preserve metadata and all server-owned files, re-root known paths, use a fresh destination ID and folder, and report Java/ports/helper/platform review items without silently reallocating ports. Commit with recoverable rename/registration bookkeeping through existing registration and world reconciliation. Make replayed completion return the same server ID. An incomplete server stays out of normal selection. Honor revocation, host reset, session expiry, restart and owned cleanup. Add only focused essential tests for concrete corruption, traversal, resume and duplicate-registration risks absent from existing coverage; record controlled inputs, purpose and expected runtime before adding them. No test execution is authorized by this plan.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings`
**Batch:** P18.42 only; depends on P18.41 verification unless included in a newly authorized batch.
**Commit:** `P18.42: receive and register verified server transfers`

### P18.43 — Relay bounded transfers through the native desktop

**Status:** Planned.
**Files:** New `clients/desktop-web/src-tauri/src/server_transfer.rs`; `clients/desktop-web/src-tauri/src/lib.rs`; dependencies only if required; desktop platform/host connection bridge; this plan.
**What:** Resolve source and destination by stored authenticated host identity, bootstrapping the local agent through the existing native path. Keep both paired connections alive without requiring source-to-destination reachability. Reject same-agent transfers even when two saved addresses refer to it. Relay bounded file chunks with connection reuse, checksum validation, bounded transient retries and acknowledged checkpoint recovery. Keep credentials and data bytes out of webview state and errors. Expose transfer status/cancel/resume independently of the sheet's lifetime. Stop new writes promptly on cancellation; finish cleanup/release reliably and report uncertain finalization by querying the persisted destination result rather than creating another copy. Agent compatibility is checked before requesting source stop. Do not install or update a remote service through the transfer API.
**Verify:** `cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib -- -D warnings`
**Batch:** P18.43 only; depends on P18.41–P18.42 verification unless included in a newly authorized batch.
**Commit:** `P18.43: relay complete server transfers through desktop`

### P18.44 — Add the local-agent transfer and dropdown flow

**Status:** Planned.
**Files:** `clients/desktop-web/src/lib/sections/fleet/TransferSheet.svelte`, `ManageSheet.svelte`; transfer state/platform bridge; `clients/desktop-web/src/App.svelte` and host-state integration as needed; this plan.
**What:** Apply `antiAIslop.md` in the existing Transfer sheet. Select one complete server and paired destination, defaulting remote sources to This computer. Show inventory size, actual destination capacity, capability failures and review items before the explicit Stop and transfer action. Display stages, file/byte progress and cancellation; preserve transfer state across sheet closure/host navigation. Offer Resume for interrupted matching sessions. On success, refresh the destination's host-scoped server list; Open on destination switches host and selects the returned new server ID through the normal dropdown/selection path. Show Java and port review without automatic start or source deletion. Keep existing transfer-file import/export available. Distinguish transfer completion from platform/runtime startup readiness, and preserve any owner-required Java/port review. Guard late responses so source-host data cannot populate destination state.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P18.44 only; depends on P18.43 verification unless included in a newly authorized batch.
**Commit:** `P18.44: add connected host server transfer flow`

### P18.45 — Prepare runnable artifacts and record transfer acceptance

**Status:** Planned.
**Files:** Transfer instructions and acceptance record under `docs/msc2/`; existing packaging instructions only if needed; this plan. Build artifacts are generated locally, not committed.
**What:** Build the updated Fedora desktop and agent using the repository's current packaging path and provide exact artifact/install commands for Cameron's Xubuntu and local hosts. Record the required minimum source/destination agent versions and complete-server storage requirement. Document normal transfer, Java/port review, resume/cancel, staging cleanup and cross-platform native-runtime limits. Give Cameron concrete acceptance instructions for Tectonic and controlled fault cases; keep their outcomes pending until he reports them. Document macOS/Windows delivery through the existing build-only release pipeline if requested separately, preserving artifacts/checksums/signed metadata and adding no workflow gates. No publishing, remote installation, live Tectonic stop/copy or test-suite execution is implied by the current PLAN request. After authorized implementation, an unavailable platform or unpublished artifact remains a delivery limitation rather than a claim that that host is ready.
**Verify:** `cargo build -p msc-agent --bin msc`
**Batch:** P18.45 only; depends on P18.41–P18.44 verification unless included in a newly authorized batch. Physical acceptance remains Cameron's separate Verify move.
**Commit:** `P18.45: document complete server transfer acceptance`
