# MSC 2 — Rolling Plan

> ## STATUS: Phase 16 step records P16.1–P16.34 and Phase 17 step records P17.1–P17.27 and all Phase 18 steps/substeps are Done at Cameron's direction and archived. [v0.1.18](https://github.com/ctemple9/msc2/releases/tag/v0.1.18) published all nine artifacts from `ededaf33632bbbdcc518ae8928a54bb3ba073cc6`. Outstanding physical acceptance, deferred checks and independent phase reviews remain separate from Done step status.
> **Next move:** Cameron records Phase 17 physical acceptance, then the other agent reviews its gate. Phase 16 still needs Cameron's exact-artifact results in `docs/msc2/release/phase16-acceptance.md` and an independent gate review. Phase 18 awaits independent review against its consolidated map acceptance record; named checks remain deferred. Done step statuses do not assert that pending gate evidence exists.

## How this document works

The vision documents describe where MSC 2 is going; the port plan defines the phase sequence and exit gates; this file records the active priorities and current phase state. Completed phase plans and historical step records live in `rolling-plan-archive.md`.

Each implementation step is planned, read, executed, verified by Cameron, reviewed against its phase gate, and then archived. A step's status records the completion state Cameron directs; phase acceptance evidence and independent review are tracked separately. Marking steps Done does not by itself close a phase gate.

## Current phase

| Phase | Name | State |
|---|---|---|
| Setup | Repo, docs, agent instructions, CI, editor config | complete |
| 0 | Freeze the baseline and build the harness | complete |
| 1 | Domain types and pure rules | complete |
| 2 | API contract and operation model | complete |
| 3 | Safety substrate | complete |
| 4 | Java lifecycle vertical slice | complete |
| 5 | Configuration and migration | complete |
| 6 | Worlds and backups | complete |
| 7 | Server families and provisioning | complete |
| 8 | Mods, plugins, modpacks | complete |
| 9 | Networking and helpers | complete |
| 10 | Bedrock runtimes | complete |
| 11 | Desktop and web clients | complete |
| 12 | Client redesign and post-phase corrections | complete |
| 13 | Full-screen terminal client | retired by D-034 |
| 14 | Operational refinements | complete |
| 15 | Maintenance follow-ups | complete |
| 16 | Release safety and codebase readiness | complete |
| 17 | Local CLI refinement | step records done; physical acceptance and independent review pending |
| 18 | Integrated 3D world map | all step records done by owner direction; named checks deferred, independent review pending |

## Active Phase 16 acceptance

The [v0.1.18 release](https://github.com/ctemple9/msc2/releases/tag/v0.1.18)
completed its build and publish workflow with all nine platform artifacts.
Cameron's exact-artifact physical observations and the independent Phase 16
review remain outstanding. See `docs/msc2/release/phase16-acceptance.md` for
the release identity, published asset metadata, and pending acceptance rows.

P16.1–P16.34 and the September 17 and September 28 audits are preserved in
`docs/msc2/rolling-plan-archive.md`. Phase 16 remains in progress until its
exit gate holds.

## Active Phase 17 acceptance

P17.1–P17.27 are Done at Cameron's direction. Their complete records are in
[the archive](rolling-plan-archive.md#phase-17--local-cli-refinement), including
[completion steps](rolling-plan-archive.md#phase-17-completion-steps).
Physical acceptance and independent gate review remain outstanding. The
local CLI contract and acceptance guidance remain in
[Phase 17 CLI](clients/phase17-cli.md) and the [port plan](msc2-port-plan.md).

## Active Phase 18 acceptance

All Phase 18 steps and corrective substeps, including P18.11, are Done at
Cameron's direction. Their complete records are in
[the archive](rolling-plan-archive.md#phase-18--3d-world-viewer).
The [map acceptance record](phase18-map-acceptance.md) preserves owner
observations, measured costs and limits. Nether absent-source troubleshooting,
ATM10 custom-dimension visual proof and further runtime/platform checks remain
deferred. Independent gate review remains outstanding. The map work is merged
into main; no release publication or exact-artifact acceptance is implied.

## Owner-requested Phase 12 follow-up

### P12.194 — Redesign agent home and remove signal/status dots

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/App.svelte`, shared status components and their callers, player-presence/chat/setup-progress surfaces, `clients/desktop-web/tests/agent-install/agent-install.test.ts`, `clients/desktop-web/tests/screens/first-launch-reset.test.ts`, `clients/desktop-web/tests/components/base.test.ts`, `clients/desktop-web/tests/archive/agent-home/`, `docs/msc2/antiAIslop.md`, this plan.
**What:** Implement Cameron's reviewed HTML home design inside the existing desktop shell. Show the current agent, its actual server list and a permanently visible explanation of the app/agent/Minecraft relationship. Missing local service offers Install; installed local service offers one Start/Stop agent button plus Repair. Those native actions await the existing connection refresh; remove separate reconnect/disconnect buttons. Service controls remain scoped to the selected local host; a remote host's service remains managed on that computer. The remote dropdown offers Connect new host / View saved hosts and populates the chosen content below, retaining SSH/tunnel review, route selection, secure credentials, editing, pairing replacement and removal confirmation. The root route opens agent home; server Overview and explicit deep links remain available. Apply Cameron's 2026-10-02 app-wide prohibition on signal/status dots and record it in the design law. Preserve state information as text. Archive obsolete exact-source assertions that required the replaced page's wording/layout; retain the controlled behavioral checks for automatic installation/startup. No tests added or run; Rust formatting/lint are not applicable because no Rust is changed.
**Verify:** `npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build`
**Batch:** P12.194 only.
**Commit:** `P12.194: redesign agent home and remove status dots`
**Owner visual verification:** Open agent home with a missing, stopped and running local service; confirm Install or the single Start/Stop button plus Repair, and automatic connection refresh after each action. Open both remote-dropdown choices, edit/switch a saved host, and open a server's Overview. Confirm signal/status dots are absent in setup, service status, player lists and chat. These interactions remain Cameron's verification; builds do not assert physical service or remote-host acceptance.

**Agent checks:** Frontend type-check and production build passed after restoring the checkout’s dependencies with `npm ci --ignore-scripts`; 10 existing type-check warnings remain outside this change. Targeted frontend formatting and `git diff --check` passed. Tests were not run. Physical app/service/remote verification remains pending.


## Owner-requested release

### P16.35 — Prepare and publish v0.1.19

**Status:** Prepared; publication requested by Cameron, exact-artifact verification pending.
**Files:** `Cargo.lock`, `crates/msc-agent/Cargo.toml`, client package manifests/lockfile, Tauri package manifests/lockfile/configuration, bundle identity (source and static asset) and its existing assertion, `tools/release/stage-windows-agent.ps1`, `README.md`, `docs/msc2/release/v0.1.19.md`, this plan.
**What:** Increment the latest release tag from v0.1.18 to v0.1.19, preserving the current main-branch agent-home redesign, Cameron's subsequent adjustment and merged world-map work. Keep agent, desktop, frontend and locked package versions consistent. Include the two terrain helper executables and their existing license in Windows desktop staging, matching the other desktop/headless packaging paths. Push main and the new immutable tag once to trigger the existing build-only beta workflow, preserving all nine artifacts, checksums and signed update metadata. No workflow gates or tests added. The pre-existing Tauri Cargo.lock dependency edit remains uncommitted; stage only the release-version change from that file.
**Verify:** `gh release view v0.1.19 --json tagName,isPrerelease,assets,url`
**Batch:** P16.35 only.
**Commit:** `P16.35: prepare v0.1.19 release`

**Release preparation checks:** Locked Cargo metadata resolved for the agent and desktop packages; all release version fields agree at 0.1.19. Frontend production build and Windows staging PowerShell syntax inspection passed. Existing workflow signing-key variable/secret names are configured. No tests were run.


### P12.195 — Remove local service details disclosure

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Remove the local “Service details & pairing another desktop” disclosure and its contents from the Agents screen at Cameron's request. Retain the local Install/Start/Stop/Repair controls and remote host/pairing workflows. Preserve Cameron's uncommitted heading edit without including it in this step's commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.195 only.
**Commit:** `P12.195: remove local service details disclosure`

**Agent checks:** Frontend type-check passed with zero errors and 10 existing warnings; `git diff --check` passed. Tests were not run.


### P12.196 — Explain closing MSC and stopping services

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Replace the connection/pairing explanation with the approved “What happens when you close MSC?” guidance: running servers continue after the app closes; use the server’s Stop button for Minecraft and Stop agent for the agent. Remove the duplicated closing-window note from the left panel and its unused styling. Preserve Cameron's uncommitted heading edit without including it in this step's commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.196 only.
**Commit:** `P12.196: clarify closing the app and stopping the agent`

### P12.197 — Collapse the agent server list and open rows directly

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/lib/sections/performance/PerformanceSection.svelte`, `clients/desktop-web/src/lib/sections/shared/server-uptime.ts`, `clients/desktop-web/src/App.svelte`, this plan.
**What:** Collapse the agent server list by default with a labeled disclosure and server count. Open Overview by clicking a keyboard-accessible server row with a quiet arrow. Place the selected server first and distinguish its running/stopped state using existing status colors without dots. Show server type/port and selected-server live players/RAM, refreshing only while the list is expanded and the screen is active. Share the existing Performance uptime semantics across tabs and the shell: count from an observed stopped-to-running transition; show Running when the start time is unknown. Forget host observations when the connection is reset, reject late list responses after host/server changes, and never present the selected server's stats on other rows. Preserve Cameron's uncommitted heading and Tauri lockfile edits outside this commit. No Rust changed; no tests added or run.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.197 only.
**Commit:** `P12.197: make agent server rows collapsible and show live stats`

**Manual verification:** Expand On this agent; confirm the selected server is first, click another row to open its Overview, and compare Players/RAM with that server's existing screens. Start a previously stopped server while connected and compare uptime across Agents and Performance. Reconnecting to an already-running server should show Running rather than inventing elapsed time.

### P12.198 — Match the selected server row to the rest of the list

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Remove the selected row's permanent gray background. Place its extra stats beside the existing type/port details rather than adding a third line, so rows share the same spacing and height when room permits. Allow details to wrap on narrow screens. Retain selected-first ordering, colored status, and direct Overview navigation. Preserve Cameron's uncommitted heading and lockfile edits outside this commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.198 only.
**Commit:** `P12.198: match selected server row styling to the list`

## Phase 19 — Complete local uninstall (owner-requested 2026-10-02)

**Planning state:** Cameron authorized implementation on 2026-10-02. P19.1 is implemented pending owner verification; later steps remain planned.

**Owner-approved intent:** Add **Uninstall MSC 2…** beside Reset in app settings and a local `msc uninstall --danger` command. Permanently remove this computer's MSC 2 agent services, managed servers/worlds/backups, MSC-owned helpers and runtimes, host/client data and credentials, caches/logs, installed command, and desktop app. Never contact or uninstall a saved remote agent. Running this flow on a remote computer means running its local MSC desktop or CLI there. Preserve MSC 1, source checkouts, separately installed Java/Tailscale/Docker, unrelated files, and OS-owned package caches.

**Confirmation contract:** Both interfaces must show the actual computer, server root(s), installation(s), and exact deletion list before execution. Desktop: review sheet, acknowledgement of permanent world/backup loss, exact typed `UNINSTALL MSC 2`, then a final destructive confirmation dialog. CLI: `--danger` enables the destructive flow but does not bypass review; print the same inventory and require the exact phrase interactively. `--confirm "UNINSTALL MSC 2"` is the explicit non-interactive equivalent, used only after a separate `--dry-run` inventory review; refuse redirected input without it. `--dry-run` performs no writes, elevation, service changes, or cleanup. A native command must enforce the confirmation independently of UI state. Local cleanup must still work while the selected desktop host is remote and while the local agent is missing/offline; no HTTP uninstall route.

**Downloaded installers:** MSC cannot prove ownership of every renamed/moved installer or its original download location. Include verified MSC release installers in known download/update locations and let the operator explicitly select additional installers. Show every selected file before confirmation, validate its package/bundle identity or signed release checksum, and remove only those files. No filename-only recursive disk search. Never promise that an unknown original DMG has been found. A mounted disk image needs explicit unmount handling; if another application holds it open, report the remaining file instead of claiming full removal. Do not delete MSI/OS package-manager caches directly.

**Completion contract:** Gracefully stop Minecraft and MSC-managed helpers before removing services/data. If shutdown or service removal fails, stop and report what remains. Remove the app through its OS installation mechanism: verified macOS bundle removal, Windows registered MSI uninstall, Linux owning package removal (or a verified standalone AppImage). Use a narrowly scoped detached continuation where the running app/command cannot remove itself. The continuation must validate its inventory again, propagate failures, and leave a readable result outside the deleted MSC trees; let the owner choose whether to retain that report. “Scheduled” is not “Uninstalled.” Reject unsupported/dev installations rather than deleting a source checkout. Clean up continuation files when finished.

### P19.1 — Inventory local installations and define the deletion boundary

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-infrastructure/src/uninstall.rs` (new), `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/tests/uninstall.rs` (new only for essential boundary cases), `docs/msc2/clients/local-uninstall.md` (new), this plan.
**What:** Build one serializable local inventory for CLI and desktop, with canonical paths, ownership evidence, missing/unavailable states, and exclusions. Discover service definitions and their actual MSC2_DATA_DIR/MSC2_APP_CONFIG_PATH/MSC2_AGENT_SERVERS_ROOT overrides rather than guessing from the current shell. Cover desktop and headless data layouts (macOS MSC 2 vs MSC2; Windows roaming/local data; Linux desktop and system headless roots), registered server roots and external managed server/backups with explicit ownership boundaries, client WebView state, credential stores, helper installs, old marked headless versions, command links/PATH entries, application/package identity, and verified installer files. Inventory is read-only; corrupt configuration or ambiguous ownership blocks affected deletion and is visible. Reject root/home/shared-parent targets, MSC 1 paths, symlink escapes, traversal, and unsupported developer checkout removal. Record essential controlled tests for path escapes, ambiguous ownership, and remote exclusion; do not run them without a specific owner instruction. Avoid brittle timing or real-machine installation fixtures.
**Verify:** `cargo check -p msc-infrastructure`
**Batch:** P19.1 only.
**Commit:** `P19.1: inventory local msc installations for uninstall`

**Implementation boundary:** The shared layer consumes local OS service/package inspection from platform adapters; P19.2 wires those native inspections and execution. Missing service/package inspection produces a blocking entry, never an assumption that an installation is absent. No CLI command or Settings uninstall action is available in this step. The preview function itself has no network, elevation, process control, or filesystem mutation.

**Essential tests:** Added six controlled in-memory deletion-boundary cases for protected/symlink/source targets, corrupt-config parent removal, ambiguous custom data roots, service override/secret separation, unverified/changed installers, and preview mutation. These protect permanent data loss or secret disclosure rather than structure/prose. Expected runtime is under one second after compilation. No tests were run.

**Agent checks:** Package type-check, compilation of the focused test target without execution, formatting, and package-library Clippy with warnings denied passed. No Rust test executables were run. Owner verification remains open.

### P19.2 — Remove services, data, credentials, and OS installation locally

**Status:** Implemented; awaiting Cameron's verification. Cameron authorized completion of P19.2–P19.4 together on 2026-10-02.
**Files:** `crates/msc-infrastructure/src/uninstall.rs`, platform-specific uninstall modules under `crates/msc-platform-macos/src/`, `crates/msc-platform-windows/src/`, `crates/msc-platform-linux/src/`, existing service/secret-store adapters where necessary, essential controlled boundary tests if current coverage misses a concrete risk, `docs/msc2/clients/local-uninstall.md`, this plan.
**What:** Execute the inventory through existing platform privilege boundaries, with no remote service API. Authenticate to the local agent if available, request graceful server/helper shutdown and verify it; handle an offline/stopped installation through its inspected service definition without guessing process ownership. Stop/unregister all verified MSC-owned service/helper definitions, remove approved data and credential records, remove verified links/PATH entries, uninstall the desktop/package and marked headless artifacts, then delete the approved installer files. Distinguish Windows MSI uninstall from raw file deletion; use Linux package ownership and macOS bundle identifier checks. Secure detached continuation state against tampering; revalidate filesystem boundaries/ownership immediately before deletion. Never execute a user-writable elevated cleanup script blindly. Provide partial-failure/result reporting and retry inventory for leftovers, preserving failures rather than suppressing them. Do not launch the real uninstaller while implementing or verifying this step.
**Verify:** `cargo check -p msc-platform-macos -p msc-platform-windows -p msc-platform-linux`
**Batch:** P19.2 only.
**Commit:** `P19.2: implement complete local uninstall execution`

**Agent checks:** Host-platform checks and package-library Clippy passed without running tests or uninstall. Linux cleanup code also compiles on Unix for inspection. A Windows-target check was attempted but blocked by missing Windows C headers in ring (assert.h); Windows native compilation/acceptance remains required on Windows. The copied worker and CLI handoff are wired in P19.3; native execution is not invoked here.

### P19.3 — Expose the confirmed uninstall command

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/uninstall.rs` (new), CLI documentation/help, `docs/msc2/clients/local-uninstall.md`, this plan.
**What:** Add `msc uninstall --danger`, `--dry-run`, exact `--confirm` for explicit automation, and validated additional-installer selection. Share inventory/execution with desktop rather than duplicating deletion logic. Ignore/refuse remote target overrides and obtain all destructive targets locally. Print warnings and inventory before the typed interactive confirmation. Return nonzero on blocked/partial cleanup and distinguish a detached scheduled action from actual completion. Keep ordinary `msc service uninstall` and host-reset behavior unchanged. Review CLI parsing/help through non-destructive checks; do not run a destructive invocation on the developer machine.
**Verify:** `cargo check -p msc-agent`
**Batch:** P19.3 only.
**Commit:** `P19.3: add confirmed local uninstall command`

**Agent checks:** Agent compilation and Clippy passed (one pre-existing auth.rs dead-code warning); no tests or uninstall commands ran. A private copied worker waits for the parent/desktop to exit, rediscovers and compares the inventory, authenticates only to the local agent to stop Minecraft, and records partial/failure outcomes. Explicit optional reports survive outside the deleted directories; otherwise successful worker files are removed. Native Windows acceptance remains pending as noted in P19.2.

### P19.4 — Add Uninstall MSC 2 to app settings

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`, `clients/desktop-web/src/lib/sections/app-settings/UninstallSheet.svelte` (new), `clients/desktop-web/src/App.svelte`, platform adapter types/implementation, `clients/desktop-web/src-tauri/src/lib.rs` and a native uninstall module if needed, `docs/msc2/clients/local-uninstall.md`, this plan.
**What:** Read antiAIslop.md before frontend work. Add a separate destructive Uninstall action beside Reset. Show native local inventory independent of selected remote host; support verified additional-installer selection. Require the loss acknowledgement, typed phrase, and final dialog before invoking native execution. Enforce typed confirmation and inventory identity natively, disable duplicate submissions, and show OS elevation and failures clearly. Clear all local saved remote credentials/connections without contacting those agents. Exit only after native handoff is established; communicate scheduled continuation and its result location accurately. Existing Reset remains unchanged. Preserve unrelated owner edits and commit only this step's work.
**Verify:** `npm --prefix clients/desktop-web run check && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Batch:** P19.4 only.
**Commit:** `P19.4: add confirmed complete uninstall to settings`

**Agent checks:** Svelte check passed with only ten pre-existing warnings; desktop native compilation passed. No tests or destructive uninstall ran. Settings uses only the packaged local command, previews verified targets, enforces the loss acknowledgement/typed phrase/final dialog, and closes after private worker handoff. Reports default to the home folder. Installer discovery is bounded by retained signed metadata, and Windows native acceptance remains pending. JSON CLI worker output is detached so the scheduled response stays parseable; Linux elevated tools use fixed absolute paths; marked Windows archives remove their exact User/Machine PATH entry. Agent and desktop Clippy completed with only existing warnings.

**Phase 19 acceptance gate:** Cameron verifies both entry points on disposable installed MSC 2 environments for macOS, Windows MSI, and Linux desktop/headless packaging. Observe server shutdown, service/helper removal, data/credential/cache cleanup, OS package deregistration, self-removal, verified installer deletion, and readable partial-failure results. Check cancellation at each confirmation, no-write dry run, stale/tampered inventory rejection, missing/offline agent behavior, protected symlink/root paths, and that saved remote agents plus MSC 1 remain unchanged. Developer source trees are never used for destructive acceptance. The other agent independently reviews the deletion boundary and phase gate. No release workflow gates or release runs are added by this work.

### P12.199 — Show remote host actions directly

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Replace Connect remote agent and its dropdown with two visible buttons, Connect new host and View saved hosts (including the saved count). Retain the existing form/list underneath. Remove unused dropdown state, focus/Escape handling, and styles. Preserve owner heading and lockfile edits outside this commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.199 only.
**Commit:** `P12.199: show remote host actions as separate buttons`

### P12.200 — Update server-computer instructions for the Phase 17 CLI

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Rename Remote service commands to Commands on your server computer. Replace Linux service-name discovery and lower-level service commands with msc status/start/stop/enable/disable agent. Explain local terminal or SSH use as the installing account, support across all three platforms while the agent is stopped, separate Minecraft server controls, and the distinction between stopping and disabling boot startup. Remove the unsupported repair wording; preserve desktop pairing. Keep owner heading and lockfile edits outside this commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.200 only.
**Commit:** `P12.200: update agent instructions for the local cli`


## Phase 6 import correction — owner-requested 2026-10-02

### P6.52 — Accept external world archive folder layouts

**Status:** Implemented; awaiting Cameron's verification. Cameron explicitly authorized implementation in this conversation.
**Files:** `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/tests/world_archive.rs`, `crates/msc-application/src/worlds.rs`, `clients/desktop-web/src/lib/sections/worlds/ImportWorldZipSheet.svelte`, this plan.
**What:** Recognize Bedrock world files at the archive root, in a named folder, or inside enclosing folders; retain existing MSC layouts. Accept `.mcworld` in the Worlds import picker. Normalize only the new slot archive, removing known macOS packaging metadata while preserving file contents, compression and permissions. Store the detected single Bedrock world folder name so activation opens that imported world. Also accept loose Java worlds and enclosing folders around Java worlds with their sibling dimensions. Keep generic source safety/CRC/size checks and strict final world-layout checks; reject unrelated files and ambiguous external multi-world bundles with an explanatory error. Existing MSC-format multiple Bedrock worlds remain supported. Original archives are untouched; failed normalization removes the partial archive/slot. MSC 1's `WorldSlotManager.createSlotFromZIP` copies ZIPs unchanged without structural checks; this correction preserves P16.4's stronger safety boundary while broadening input packaging. No API contract or release workflow changed.
**Verify:** `cargo test -p msc-infrastructure --test world_archive world_import_`
**Batch:** P6.52 only.
**Commit:** `P6.52: normalize external world archive layouts on import`

**Essential test rationale:** Three focused cases protect previously rejected external layouts, correct Bedrock folder identity, original archive preservation, retention of Java dimensions, and refusal of traversal, links, executable permissions, server configuration, unrelated entries and ambiguous bundles. Existing strict-layout tests do not exercise normalization. All inputs are tiny local ZIPs with controlled contents and independent temporary directories; no network, Minecraft runtime or timing assumptions. Expected combined runtime: under one second. Tests were added and compiled with Clippy but not run.

**Agent checks:** Rust formatting and Clippy for the affected libraries and archive test target; Svelte static check (zero errors, ten existing warnings). No test commands run.

**Manual acceptance:** Rebuild/restart the local app and agent, then import `/Users/camerontemple/msc2-servers/XqKXS4++O7k=.zip` unchanged through the Bedrock Worlds tab. With the server stopped, activate the imported slot and confirm the existing terrain loads. Also import a `.mcworld` file without renaming it. External bundles containing several worlds require individual imports; a selection screen remains a separate UI/API change.


### P6.53 — Let API imports reach external archive normalization

**Status:** Implemented; awaiting Cameron's verification. Follow-up to Cameron's report that the unchanged ZIP still receives the original error after P6.52.
**Files:** `crates/msc-agent/src/routes/worlds/import_activation.rs`, `crates/msc-agent/src/routes/worlds.rs`, this plan.
**What:** Remove the import route's premature activation-layout check. All desktop and CLI imports now reach P6.52's source safety checks, normalization, and strict final layout validation. Preserve invalid-archive HTTP 400 responses and consumed staging cleanup; failures are recorded on the exclusive import operation. Diagnosis confirmed that the running local agent contains the new normalization code, so the repeated rejection was a missed route-level check, not an outdated binary. Preserve activation/restore validation unchanged.
**Verify:** `cargo test -p msc-agent --bin msc world_backup_routes_staged_upload_import_round_trip`
**Batch:** P6.53 only.
**Commit:** `P6.53: normalize world imports before checking stored layout`

**Essential coverage:** Strengthen the existing staged-upload/import round-trip with a loose `level.dat` ZIP and assert that the stored archive contains `world/level.dat`. This directly catches a route precheck preventing normalization, the gap missed by P6.52's library cases. Reuses the existing controlled local ZIP and fake process/journal setup; no additional test count, network, live Minecraft or timing assumptions. Expected runtime remains under one second. Test execution is deferred.

**Agent checks:** Rust formatting and production agent Clippy passed (one existing unused `auth::forbidden` warning). Broader test-target compilation was blocked by pre-existing `crates/msc-agent/tests/cli_service.rs` references to removed `CommonArgs` fields (`base_url`, `host`, `port`, `token`); no tests ran and that unrelated file was not changed. Rebuild/restart the development app/agent and retry the original Bedrock ZIP unchanged for manual acceptance.


### P6.54 — Preserve directory types when normalizing ZIPs

**Status:** Implemented; awaiting Cameron's verification. Follow-up to Cameron's `unsafe archive entry: worlds/XqKXS4++O7k=/db/` import failure.
**Files:** `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/tests/world_archive.rs`, this plan.
**What:** Write normalized directory markers with ZIP `add_directory`, retaining their permission bits. Raw copying remains only for file contents. The ZIP library's `raw_copy_file_rename` reconstructs options via `unix_permissions`, which strips entry type bits; directory markers therefore became regular-file entries and failed strict validation. Inspect original entry modes before copying to reject executables and non-regular types rather than let option reconstruction disguise them. Source archives and strict activation validation remain unchanged.
**Verify:** `cargo test -p msc-infrastructure --test world_archive world_import_`
**Batch:** P6.54 only.
**Commit:** `P6.54: preserve directory entry types during world import`

**Essential coverage:** Extend the existing Bedrock packaging case with an explicit `db/` directory marker (the actual failed entry) for every supported layout, and assert the stored directory type and original 0700 permissions. No new test count; existing tiny local fixtures, expected combined runtime under one second. No tests run.

**Agent checks:** Rust formatting and Clippy for the affected libraries and archive test target. Manual acceptance requires rebuilding/restarting the agent and importing the unchanged original ZIP.


### P6.55 — Show real world activation progress and status age

**Status:** Implemented; awaiting Cameron's verification. Cameron explicitly requested progress reporting after successfully importing the original Bedrock ZIP.
**Files:** `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/tests/world_archive.rs`, `crates/msc-application/src/backups.rs`, `crates/msc-application/src/worlds.rs`, `crates/msc-application/src/worlds/activation.rs`, `crates/msc-application/tests/world_activation.rs`, `crates/msc-agent/src/routes/worlds/import_activation.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Report activation stages through the existing operation DTO: checking imported world, creating/checking safety backup, saving the outgoing slot, checking archive before extraction, extracting imported world, installing world, and applying settings. Count actual verified/copied bytes; measure compression source totals only for progress-aware archive creation. Show a flat inline Worlds progress panel with the stage, per-stage bar/percentage and byte counts when a total is known, elapsed time, and time since the stage/count last changed. Connection/read failures explicitly say status is unavailable and retry reads without reissuing activation or declaring it failed. Stop monitoring on destruction or host/server changes and ignore late responses. Existing ordinary backup/archive entry points remain available. Progress callbacks preserve source safety/CRC checks, cancellation boundaries, world-swap ordering and recovery behavior. Throttle agent publication to four updates per second, plus stage starts/completions, to bound journal writes. No API contract or release workflow changed. Read the anti-slop design law; the panel uses existing neutral tokens, one flat group, a functional bar, and text for state.
**Verify:** `cargo test -p msc-infrastructure --test world_archive world_activation_archive_progress_counts_real_bytes`
**Batch:** P6.55 only.
**Commit:** `P6.55: report world activation stages and byte progress`

**Essential coverage:** One new small local archive round-trip checks measured compression/verification/extraction totals, intermediate byte counts and monotonic counts, plus preserved extracted contents. Existing archive safety and cancellation coverage exercises the shared primitives. Extend the existing activation/profile case to ensure stage reporting accompanies the unchanged resulting settings. Both use controlled local files, with no network, live Minecraft, sleeps, or timing assertions; expected combined runtime under one second. Tests were compiled by Clippy but not run.

**Agent checks:** Rust formatting; Clippy for the affected libraries, archive/activation test targets, and production agent (one existing unused `auth::forbidden` warning); Svelte static check (zero errors, ten existing warnings). No tests or live world mutations run.

**Manual acceptance:** Rebuild/restart the development app and agent. Activate a sizable saved world while the server is stopped; verify the checking/backup/save/extraction stages and real byte bar, followed by success. During a long stage, observe elapsed time and last-progress age. A status read/connection failure must show retrying and last-agent-contact age, then resume tracking the same operation after recovery. Switching tabs retains tracking; switching host/server or destroying the view must stop its timer/polling and prevent late old-operation notices. Percentages describe the current named stage and can reset at the next stage; periods with no countable byte work show the stage/time rather than a simulated percentage. Progress age reveals inactivity but does not by itself prove a stall.


### P12.201 — Open Modrinth project links in the default browser

**Status:** Implemented; awaiting Cameron's verification. Cameron reported that View on Modrinth does nothing in the Paper datapack project sheet.
**Files:** `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte`, this plan.
**What:** Route the shared Modrinth project sheet's external anchors through the existing platform `openExternal` function, which invokes Tauri's validated OS browser opener. Ordinary `target="_blank"` anchors did not invoke that desktop command. Apply the same handling to About-description links and Source/Issues/Wiki/Discord anchors in this sheet. Preserve link destinations and styling; show browser-opening errors inline instead of silently failing. The shared sheet covers datapacks, mods and plugins. No native code, URL policy, API contract or release workflow changes; no new tests needed for this small wiring correction and no tests run.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.201 only.
**Commit:** `P12.201: open project detail links in the default browser`

**Manual acceptance:** In the Paper world's datapack browser, open Terratonic and click View on Modrinth; confirm the default browser opens the project page. Check the About wiki link and a project's Source/Issues/Wiki links through the same shared sheet. The existing desktop opener reports failed/unsupported URL launches inline. Similar plain anchors in the separate CurseForge Bedrock pack sheet are outside this step.


### P12.202 — Install datapacks with overlays and remember installed versions

**Status:** Implemented; awaiting Cameron's verification. Cameron clarified that the Terratonic install actually failed, and also requested recognition of previously installed datapacks.
**Files:** `crates/msc-application/src/addons.rs`, `crates/msc-application/tests/addons.rs`, `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, this plan.
**What:** Select the outermost `pack.mcmeta` rather than the first member in ZIP order. Inspection of the actual Modrinth Terratonic 3.0.27 download (version cT2AsHrJ) showed seven metadata members: six overlay copies before the root copy; the old installer selected an overlay and rejected legitimate sibling files. Preserve all overlay contents and keep the common-root, path, symlink, size, compatibility and checksum checks. Read the selected world's saved Modrinth datapack records when opening the browser, pass their version IDs into project details, and label successful/pre-existing recorded installations Installed (staging remains Added). Record successful new versions immediately. Shared project details no longer mark a failed/cancelled background install as installed. The browser-link correction remains its own P12.201 commit. Manual/imported packs without saved provider/version identity cannot be matched to a Modrinth release by these labels; no identity is guessed. No live install performed and no release workflow changes.
**Verify:** `cargo test -p msc-application --test addons java_datapack_install_uses_outer_metadata_and_preserves_overlays`
**Batch:** P12.202 only.
**Commit:** `P12.202: install overlay datapacks and restore installed labels`

**Essential test rationale:** One focused regression case reproduces overlay metadata preceding main metadata, at archive root and inside an enclosing folder; checks preserved overlays/main metadata, prior-world backup, and refusal of ambiguous separate packs without changing the world. Existing addon tests did not cover datapack metadata selection. Uses tiny local ZIPs and a unique directory with automatic cleanup, no network, live Minecraft, sleeps or timing assertions. Expected runtime under one second. Test compiled but not run.

**Manual acceptance:** Rebuild/restart the app and agent; on a stopped Paper server, install Terratonic 3.0.27 from the datapack sheet and confirm success/Installed. Close and reopen the browser and project details; the same saved Modrinth version should remain Installed. Installation errors must remain errors rather than create Installed labels. World-generation effects and external datapack dependencies remain Minecraft/pack behavior, not proved by installation alone.


### P12.203 — Limit world datapack choices to datapack releases

**Status:** Implemented; awaiting Cameron's verification. Cameron reported Fabric/NeoForge mod releases labeled Compatible in the Paper datapack browser and requested a correction.
**Files:** `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-application/src/addons.rs`, `crates/msc-application/tests/addons.rs`, this plan.
**What:** Require the provider's datapack loader/type marker in the world datapack detail sheet before collapsing/filtering versions or computing the compatible-version summary. Exclude mod builds even if they match the server Minecraft version; do not use the general add-on browser's fallback to show them when no datapack remains. Base Stable-only fallback on datapack releases. Automatic search-result installation selects a matching datapack release, not the first same-Minecraft mod release. Reject non-datapack releases before download in the agent and before mutation in the application installer. Datapacks for other Minecraft versions remain visible with Other version; the existing installer still refuses versions that do not list the server version. Shared mod/plugin browsing retains its existing behavior. No API contract or release workflow changes.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.203 only.
**Commit:** `P12.203: exclude mod builds from world datapack installs`

**Essential coverage:** Extend the existing overlay-install regression with Fabric and NeoForge releases advertising the same Minecraft version and containing otherwise valid datapack metadata. Assert refusal and unchanged world bytes. Reuse controlled tiny local ZIPs, no additional test count or runtime assumptions; expected under one second. Tests compiled but not run.

**Manual acceptance:** Rebuild/restart app and agent. Browse the mixed Tectonic project from the Paper world's datapack browser: Fabric/NeoForge builds must be absent; only datapack builds can receive the Compatible/Other version badge and install controls. A mod-only release list must remain empty rather than fall back to incompatible builds. Install from the search result must select a matching datapack release. Compatible here means the provider lists the server Minecraft version, not proof of runtime or dependency behavior.


**Agent checks for P12.203:** Rust formatting, application regression test compilation with Clippy, and production agent Clippy passed (one existing unused `auth::forbidden` warning). Svelte static check passed with zero errors and ten existing warnings. No tests or live installs run.

### P12.204 — Manage installed world datapacks

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-application/src/addons.rs`, `crates/msc-application/tests/addons.rs`, this plan.
**What:** Clicking an installed Java datapack opens its status and Delete action, with confirmation. View datapack opens its saved Modrinth project inside MSC, including Installed labels; packs without a recorded catalog identity explain why no page is available. Extend existing pack management to delete Java datapack files, retain safety backup and profile rollback, and refresh/reapply stopped active worlds using their actual edition and configured level name. Java enable/disable is not offered because it requires a separate Minecraft activation-list change. Keep existing Bedrock controls.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.204 only.
**Commit:** `P12.204: add installed datapack deletion and project navigation`

**Essential coverage:** Extend the existing small local overlay fixture to delete all installed files while preserving level.dat, verify backup bytes, and refuse unsupported Java disable without mutation. No network or timing assumptions; expected under one second. Compiled with Clippy, not run.
**Manual acceptance:** Rebuild/restart app and agent. Click an installed datapack on a stopped Paper world. View datapack must open its in-app project page if a saved Modrinth source exists. Delete requires confirmation, removes the row and its files from the selected world, and stays removed after reopening and activating the world. Cancel leaves it installed. Active running servers refuse deletion.

### P12.205 — Match datapack actions to Components menus

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Replace the datapack action sheet with the same shared popup Menu used by Components. Show View and destructive Uninstall only, with matching selected-row styling and a chevron. View opens the saved in-app Modrinth project; disable View when no supported catalog identity exists. Uninstall switches the row to inline Uninstall?/Cancel/Uninstall confirmation, matching Components. Keep stopped-server protection and existing removal/backup behavior. Anchor keyboard-triggered menus to the row.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.205 only.
**Commit:** `P12.205: match datapack actions to components menus`

**Manual acceptance:** Click a datapack row: the popup must look like Components with View and Uninstall. View opens its in-app catalog page where available. Uninstall shows inline confirmation; Cancel preserves the pack, confirmed Uninstall removes it. Clicking away or Escape closes the popup. Svelte check passed with zero errors and ten existing warnings; no tests added or run.

### P12.206 — Check Chunker updates and quiet conversion guidance

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldConversionWizard.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-infrastructure/src/chunker.rs`, `docs/msc2/worlds/phase6-api.md`, this plan.
**What:** Remove the orange side rail and inset from conversion guidance. Add an explicit Check for Chunker updates action on preflight when installed. Report the official latest release, up-to-date status, or lookup failure; offer Update Chunker when the release differs or installed provenance is unknown. Share official release/JAR selection validation between metadata lookup and acquisition. New permission-checked/audited GET reads metadata only on a blocking worker. Updating remains user-selected via existing operation/progress/download validation; reload supported formats and installed version on success. Preserve ready conversion controls if an update attempt fails. No automatic update, tests, or release workflow changes.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.206 only.
**Commit:** `P12.206: add optional chunker update checks`

**Manual acceptance:** Rebuild/restart app and agent. Convert World preflight has plain conversion guidance without an orange rail. Check for updates displays latest/current status; lookup failure does not prevent conversion with the installed version. Choose Update Chunker if offered, observe acquisition progress, then confirm installed version/formats refresh. No converter update occurs from checking alone. Svelte check passed with zero errors and ten existing warnings; Rust formatting and production Clippy passed with the existing unused auth helper warning. No tests or live downloads run.

### P12.207 — Remove decorative Manage Servers icons

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/fleet/ManageSheet.svelte`, this plan.
**What:** Remove the decorative server glyph and its boxed surface from each Manage Servers row, including unused styling. Server names and paths now begin at the existing row inset; preserve badges, activation and menu actions.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.207 only.
**Commit:** `P12.207: remove decorative server row icons`

**Manual acceptance:** Open Manage Servers: every row starts with the server name, without a left icon or reserved icon gap. No tests added or run.

### P12.208 — Open server actions from the whole row

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/fleet/ManageSheet.svelte`, this plan.
**What:** Replace per-server Set Active and three-dot buttons with a full-row button and trailing chevron. Use the existing shared Menu, Components selection treatment, and Set Active/Edit/Remove labels. Keep removal confirmation and editor behavior; disable Set Active for the already active server or missing control permission. Keyboard activation anchors the menu to the row.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.208 only.
**Commit:** `P12.208: open server actions from row clicks`

**Manual acceptance:** Manage Servers has a chevron instead of action buttons on each row. Clicking or keyboard-activating the row opens the Components-style three-action menu; Set Active updates the selected server, Edit opens its editor, Remove requires existing confirmation. Escape/click-away dismisses the menu. No tests added or run.

### P12.209 — Expand activation progress and simplify elapsed text

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Remove the Last progress age from activation display and replace its 520px cap with full available width. Preserve elapsed time, percentage, byte counts and lost-contact reporting. Display-only change; no activation logic, backend changes, restarts or live operations performed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.209 only.
**Commit:** `P12.209: expand world activation progress display`

**Manual acceptance:** Activation progress spans the world section width and shows elapsed time without Last progress text. Lost-agent-contact reporting remains available. No tests added or run.

### P12.210 — Fill stretched world slot cards with selection outline

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldSlotCard.svelte`, this plan.
**What:** Make the inner slot wrapper fill the grid-stretched Card. The selected border and background now cover the entire card height, which follows the tallest card in each grid row even when a slot has fewer metadata lines.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.210 only.
**Commit:** `P12.210: fill world slot selection cards`

**Manual acceptance:** Select slots with and without seed/profile details in the same row. Their selected border should run around the full, equal-height card including the blank space below shorter metadata. Svelte check passed; no tests added or run.

### P12.211 — Keep long active world names inside Overview cards

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/home/HomeSection.svelte`, `clients/desktop-web/src/lib/sections/home/ActiveWorldCard.svelte`, this plan.
**What:** Allow the Activity grid tracks and Active World column to shrink below their contents' intrinsic width. Let the world title metadata column take only available space and clip its existing single-line ellipsis inside the card. Long slot names no longer push into or overlap Chat.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.211 only.
**Commit:** `P12.211: contain long active world names`

**Manual acceptance:** Open Overview with an unusually long active-world slot name. The title should truncate within Active World and Chat should keep its own column with no overlap. Svelte check passed; no tests added or run.

### P12.212 — Freeze activation elapsed time at completion

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Stop the one-second elapsed-time clock as soon as the agent reports succeeded, failed, or cancelled. Keep the terminal progress and elapsed value steady while worlds/backups refresh; existing cleanup then closes the progress display.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.212 only.
**Commit:** `P12.212: freeze world activation elapsed timer`

**Manual acceptance:** Activate a world. On terminal status, elapsed time must stop changing immediately, including while the refreshed world list is loading. No tests added or run.

### P12.213 — Keep sidebar gameplay defaults in the active world profile

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `docs/msc2/clients/world-settings.md`, this plan.
**What:** Read sidebar Difficulty and Gamemode from the active slot's `WorldProfile` and save changes back through its profile endpoint. Refresh the sidebar after profile saves, world activation, and world creation. Running Bedrock servers apply these fields with runtime commands and report pending restart if a command cannot be sent. Java uses the shared profile application path for Paper and all other Java flavors.
**Verify:** `npm --prefix clients/desktop-web run check` and `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.213 only.
**Commit:** `P12.213: sync sidebar world gameplay settings`

**Manual acceptance:** On Bedrock and Java servers, confirm the sidebar and active slot's World Settings show the same difficulty and default game mode. Change each value from either surface and confirm the other reflects it. Check Creative confirmation behavior on Bedrock and Java; confirm Paper, Fabric, Forge, and NeoForge share the Java behavior. No tests added or run.

### P12.214 — Keep latest Java selection stable through server creation

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, this plan.
**What:** Pin the resolved latest Java version when the wizard first asks which runtime to use. This keeps the selected Java runtime associated with the same release through the World step and Create action, so the wizard doesn't ask twice. Changing the selected version or Java flavor still invalidates the selection.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.214 only.
**Commit:** `P12.214: keep latest java selection through creation`

**Manual acceptance:** Create a fresh Java server with Download latest selected. Choose Java at the first prompt; the final Create Server action should not prompt again. If you go back and change Minecraft version or Java flavor, the runtime picker should appear again before proceeding. No tests added or run.

### P12.215 — Apply Bedrock default game mode to online players

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/routes/worlds.rs`, `docs/msc2/clients/world-settings.md`, this plan.
**What:** When the active, running Bedrock world's default game mode changes, keep applying `defaultgamemode` for the saved world default and also run `gamemode <mode> @a` so currently connected players switch immediately. Difficulty continues to apply through Bedrock's live difficulty command. Keep Creative's existing achievements confirmation.
**Verify:** `cargo fmt --all` and `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.215 only.
**Commit:** `P12.215: apply bedrock gamemode to online players`

**Manual acceptance:** With multiple players online, change the active Bedrock world's default game mode from the sidebar or World Settings. Every connected player should switch immediately; new players should also receive the saved default. Confirm Bedrock Creative still requires the existing achievement warning. No tests added or run.

### P12.216 — Replace sidebar whitelist with Enforce Gamemode

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, `clients/desktop-web/src/lib/sections/settings/SettingsSection.svelte`, `crates/msc-domain/src/settings_schema.rs`, `docs/msc2/clients/world-settings.md`, this plan.
**What:** Replace the sidebar Whitelist control with Enforce Gamemode. Read and save the server-wide `force-gamemode` property through `/v1/settings`, preserve its existing confirmation when enabled, and refresh the sidebar when Server Settings changes it. Mark this property restart-required for Java and Bedrock because the running server reads it at startup; explain that it enforces the server default when players join.
**Verify:** `npm --prefix clients/desktop-web run check` and `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.216 only.
**Commit:** `P12.216: replace sidebar whitelist with enforce gamemode`

**Manual acceptance:** Confirm the sidebar shows Force Gamemode's saved value on Java and Bedrock, including values changed in Server Settings. Enable it and accept the server-wide confirmation; the setting should persist and indicate restart when the server is running. After restart, players joining should receive the server default. Disabling should persist without confirmation. No tests added or run.

### P17.28 — Resolve the CLI socket from the installed agent service

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/cli/transport.rs`, `crates/msc-agent/src/cli/service.rs`, `docs/msc2/clients/phase17-cli.md`, this plan.
**What:** On macOS and Linux, locate the local CLI socket using an explicit process `MSC2_DATA_DIR` when set, otherwise read `MSC2_DATA_DIR` from the installed agent service definition, and use the existing platform default when the service has no override. This fixes the macOS headless installer path mismatch and also supports Linux custom data roots. Windows uses a fixed named pipe and is unaffected. Add focused path-precedence tests without running them.
**Verify:** `cargo check -p msc-agent`
**Batch:** P17.28 only.
**Commit:** `P17.28: resolve cli socket from installed service`

**Essential tests:** Two pure path-selection cases protect the concrete CLI authorization failure caused by a service/CLI data-directory mismatch and retain the documented explicit environment override. They use fixed paths, do not touch the host, and should complete in under one second after compilation. Tests were not run.

**Manual acceptance:** On macOS, run `msc capabilities` without setting `MSC2_DATA_DIR`; confirm it uses the installed service path. On Linux, repeat with the default installer path and with an explicitly configured service data root. Confirm an explicit shell `MSC2_DATA_DIR` still takes precedence. Windows needs no path-specific change because its CLI connects to the fixed local named pipe.

### P12.217 — Remove Enforce Gamemode helper text

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, this plan.
**What:** Remove the explanatory sentence beneath the sidebar Enforce Gamemode toggle.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.217 only.
**Commit:** `P12.217: remove enforce gamemode helper text`

### P12.218 — Allow older Java runtimes in first-run setup

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src-tauri/tauri.conf.json`, this plan.
**What:** Let first-run setup select Java 8 or later instead of incorrectly requiring Java 21 before a Minecraft version is known. Keep version-specific compatibility checks at server creation, and set the default Tauri window to 1740 × 1080 logical pixels to match Cameron's current window.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.218 only.
**Commit:** `P12.218: allow older java runtimes during setup`

### P12.219 — Match the default window to the resized app

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src-tauri/tauri.conf.json`, this plan.
**What:** Set the initial Tauri content area to 1600 × 900 logical pixels to match Cameron's manually resized app window.
**Verify:** Rebuild and launch the desktop app; confirm the initial window opens at the resized dimensions instead of filling the display.
**Batch:** P12.219 only.
**Commit:** `P12.219: match default window to resized app`

### P12.220 — Remove Tailscale from first-run setup

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, this plan.
**What:** Remove the Tailscale page from first-run setup and remove its mention from the intro page. Keep the remaining setup steps, optional skips, and completion navigation in order.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.220 only.
**Commit:** `P12.220: remove tailscale from first-run setup`
