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
