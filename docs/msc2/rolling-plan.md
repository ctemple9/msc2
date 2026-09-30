# MSC 2 — Rolling Plan

> ## STATUS: Phase 16 step records P16.1–P16.34 and Phase 17 step records P17.1–P17.27 are Done at Cameron's direction and archived. [v0.1.18](https://github.com/ctemple9/msc2/releases/tag/v0.1.18) published all nine artifacts from `ededaf33632bbbdcc518ae8928a54bb3ba073cc6`. Both phase exit gates remain open for their separate physical results and independent reviews.
> **Next move:** Cameron records Phase 17 physical acceptance, then the other agent reviews its gate. Phase 16 still needs Cameron's exact-artifact results in `docs/msc2/release/phase16-acceptance.md` and an independent gate review. Done step statuses do not assert that pending gate evidence exists.

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

## Active Phase 16 acceptance

The [v0.1.18 release](https://github.com/ctemple9/msc2/releases/tag/v0.1.18)
completed its build and publish workflow with all nine platform artifacts.
Cameron's exact-artifact physical observations and the independent Phase 16
review remain outstanding. See `docs/msc2/release/phase16-acceptance.md` for
the release identity, published asset metadata, and pending acceptance rows.

P16.1–P16.34 and the September 17 and September 28 audits are preserved in
`docs/msc2/rolling-plan-archive.md`. Phase 16 remains in progress until its
exit gate holds.

## Phase 17 — Local CLI refinement

**Owner direction received 2026-09-29:** Cameron directed that P17.1–P17.27 be recorded Done. The Phase 17 exit gate remains open until physical acceptance and the independent review are recorded.

**Scope under D-040 and D-041.** After installing either the desktop app or a
headless agent, `msc` works from that host's terminal or an SSH shell on that
host, without a manually exported token, visible pairing code, or repeated
setup after reboot. The CLI does not control another host directly; the desktop
retains its separate remote-host pairing. `msc start agent`, `msc stop agent`,
and `msc status agent` control the local OS service even when the agent is
down. Starting and stopping do not silently change boot enablement. The
installer owns the command path and boot policy on every supported OS.
`msc command <Minecraft command>` remains the sole raw game-command path;
Phase 17 adds no Minecraft command vocabulary or full-screen terminal client.

The CLI should cover normal server, player, world, backup, configuration,
networking, installed-content, recovery, file, log, and help tasks. Supported
catalog operations use search → inspect compatibility and dependencies →
confirm → install, with useful noninteractive JSON output. CurseForge modpacks
still begin with a user-supplied archive; the agent downloads allowed
manifest-listed files and identifies any author-blocked file the user must
supply. Do not invent CurseForge modpack browsing.

**Working gate to record in `msc2-port-plan.md` during P17.1:** locally
packaged desktop and headless installs on macOS, Windows, and Linux put `msc`
on PATH; local and SSH-host commands need no token entry before or after an
agent restart or host reboot; service controls work while the agent is down;
boot behavior matches the documented policy; the API-to-CLI task inventory
has no unreviewed user-facing gaps; provider limits, permissions, safety
confirmations, and edition limits remain accurate. Cameron records physical
results before the other agent reviews the gate. Phase 16's open acceptance
remains separate. No release publication or CI gate is implied.

### P17.1 — Record the local CLI contract and phase gate

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `docs/msc2/msc2-decisions.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/msc2-product.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/clients/phase17-cli.md`
- **What:** Record Cameron's local-or-SSH-host-only CLI rule, invisible local authentication, service and boot semantics, desktop pairing separation, supported catalog boundary, no new Minecraft commands, and the Phase 17 gate. Reconcile old direct-remote CLI claims without changing desktop multi-host support.
- **Verify:** `rg -n 'Phase 17|SSH|boot|pairing|CurseForge' docs/msc2/msc2-port-plan.md docs/msc2/clients/phase17-cli.md`
- **Batch:** A (P17.1–P17.2) — contract and inventory

### P17.2 — Map every API route to a CLI task or explicit exclusion

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `docs/msc2/clients/phase17-cli.md`, `docs/msc2/api-contract/openapi.json`
- **What:** Compare the live router and OpenAPI with the CLI. Record every user task, existing command, missing command, permission, edition limit, active-server context, and output rule. Mark staging, stream tickets, and desktop bootstrap as internal. Identify any missing API capability before implementing commands.
- **Verify:** `rg -n 'servers|players|worlds|backups|catalog|operations|help|internal' docs/msc2/clients/phase17-cli.md`
- **Batch:** A (P17.1–P17.2) — contract and inventory

### P17.3 — Define host-local CLI authentication

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `crates/msc-agent/src/auth.rs`, `crates/msc-agent/src/auth/local_cli.rs`, `docs/msc2/clients/phase17-cli.md`, `docs/msc2/rolling-plan.md`
- **What:** Define the shared, compile-safe local-authentication interface and in-memory short-lived API credential policy. Document the trusted OS peer and service-account inputs, authorization, route permissions, audit identity, failure behavior, and restart behavior. P17.4–P17.6 add the actual platform IPC listeners and peer verification; P17.7 connects the CLI and removes its existing remote/token options. No local exchange is available to operators in this step.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** B (P17.3–P17.7) — local authentication

### P17.4 — Implement Linux local authentication

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `crates/msc-agent/src/auth/`, `crates/msc-agent/src/main.rs`, `crates/msc-platform-linux/src/`, `docs/msc2/clients/phase17-cli.md`, `docs/msc2/rolling-plan.md`
- **What:** Use a Unix socket with peer credentials and installation-user ownership. Authorize the same account at the keyboard or through SSH after agent and host restarts, while preserving the privileged credential helper boundary.
- **Verify:** `cargo check -p msc-agent -p msc-platform-linux`
- **Batch:** B (P17.3–P17.7) — local authentication

### P17.5 — Implement macOS local authentication

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `crates/msc-agent/src/auth/`, `crates/msc-platform-macos/src/`, `packaging/macos/`
- **What:** Verify the local or SSH-shell OS peer identity without a GUI login. Keep CLI access distinct from the desktop's signed-package bootstrap key and preserve service-account and macOS consent boundaries.
- **Verify:** `cargo check -p msc-agent -p msc-platform-macos --target x86_64-apple-darwin`
- **Batch:** B (P17.3–P17.7) — local authentication

### P17.6 — Implement Windows local authentication

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `crates/msc-agent/src/auth/`, `crates/msc-platform-windows/src/`, `packaging/windows/`
- **What:** Use a local named pipe with an explicit access-control list and verified caller identity. Permit the installing account after reboot or remote Windows login, reject unrelated users, and preserve the Service Control Manager privilege boundary.
- **Verify:** `cargo check -p msc-agent -p msc-platform-windows --target x86_64-pc-windows-msvc`
- **Batch:** B (P17.3–P17.7) — local authentication

### P17.7 — Make CLI API calls local and automatic

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/transport.rs`, `crates/msc-agent/src/cli/session.rs`, `crates/msc-agent/src/main.rs`
- **What:** Obtain local authorization on each invocation and use the existing authenticated API without visible tokens. Remove public direct-remote host/URL/port/token options and `token print`; explain the migration. Report a stopped or uninstalled agent and unauthorized OS account clearly. Preserve JSON scripting and route permission checks.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** B (P17.3–P17.7) — local authentication

### P17.8 — Add plain local agent-service commands

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.8: add local agent service commands
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/service.rs`, `crates/msc-platform-linux/src/service.rs`, `crates/msc-platform-macos/src/service.rs`, `crates/msc-platform-windows/src/service.rs`
- **What:** Add `msc start agent`, `msc stop agent`, and `msc status agent` around the installed local service, without an API credential or internal service-name flags. Show installed, running, stopped, and boot-enabled states. Keep `msc server start/stop` distinct and require only OS-level privilege where needed.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** C (P17.8–P17.10) — installation and service

### P17.9 — Put the CLI on PATH for both installation types

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.9: register desktop cli command paths
- **Files:** `packaging/linux/`, `packaging/macos/`, `packaging/windows/`, `clients/desktop-web/src-tauri/`, `docs/msc2/clients/headless-installation.md`
- **What:** Audit desktop packages and headless installers on all three OSes. Install `msc` in a standard command location or installer-owned PATH entry; preserve conflict, upgrade, and uninstall ownership rules. Tell users when only a new shell can see a PATH change.
- **Verify:** `git diff --check`
- **Batch:** C (P17.8–P17.10) — installation and service

### P17.10 — Make boot and stop behavior consistent

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.10: align agent boot and stop behavior
- **Files:** `crates/msc-platform-linux/src/service.rs`, `crates/msc-platform-macos/src/service.rs`, `crates/msc-platform-windows/src/service.rs`, `packaging/linux/`, `packaging/macos/`, `packaging/windows/`
- **What:** Start and enable the agent during approved installation on every OS, correcting macOS's current `RunAtLoad=false`. A routine stop lasts until an explicit start or next boot; a separate explicit disable action controls future boot startup. Preserve server-process shutdown guarantees.
- **Verify:** `git diff --check`
- **Batch:** C (P17.8–P17.10) — installation and service

### P17.11 — Add CLI access administration

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/users.rs`
- **What:** Expose named-token list/create/update/revoke for intentional delegation and inspect the current local authorization. Show a new delegated secret once through a safe terminal path; never use it to authenticate the local CLI. Keep admin permissions and expiry visible.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** D (P17.11–P17.15) — core administration

### P17.12 — Complete server discovery and transfer

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/session.rs`, `crates/msc-agent/src/routes/lifecycle.rs`
- **What:** Add list, detail, active selection, size, notes, and export/transfer commands over existing APIs. Make active-server context obvious before mutations; accept name or ID with clear ambiguity errors. Preserve create, import, start, stop, and EULA behavior.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** D (P17.11–P17.15) — core administration

### P17.13 — Complete status, metrics, sessions, and console reading

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `crates/msc-agent/Cargo.toml`, `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/transport.rs`
- **What:** Surface performance, host resources, player session log, and bounded console history; add cancellable live console follow if the stream contract supports it. Label unavailable edition-specific data honestly. Distinguish agent-service status from server status.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** D (P17.11–P17.15) — core administration

### P17.14 — Add player inspection and moderation

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/players.rs`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/clients/phase17-cli.md`
- **What:** Show online and known players, then expose message, kick, ban/pardon, operator, Java whitelist, and Bedrock allowlist tasks using existing permissions and capabilities. Leave other Minecraft commands to raw `msc command`.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** D (P17.11–P17.15) — core administration

### P17.15 — Add saved player-data operations

- **Status:** Done — Cameron direction, 2026-09-29
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/players.rs`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/clients/phase17-cli.md`
- **What:** Inspect available stats and inventory; duplicate/delete data and perform Java offline/custom UUID migration. Explain Bedrock's stopped-server requirement and Java-only migration; explicitly confirm destructive actions.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** D (P17.11–P17.15) — core administration

### P17.16 — Complete world and backup maintenance

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.16: complete world maintenance commands
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/backups.rs`
- **What:** Add profile inspection, save-current, repair, safe live-world rename, supported conversion formats, and Chunker acquisition to the existing world/backup commands. Preserve mandatory safety backups, stopped-server guards, confirmation tokens, and operation progress; do not duplicate existing verbs.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** E (P17.16–P17.20) — content and configuration

### P17.17 — Manage packs attached to worlds

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.17: add world pack management commands
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/components.rs`
- **What:** Inspect installed Java data packs and Bedrock behavior packs by slot; expose supported enable/disable/remove actions. For provider installs, show search, detail, version compatibility, dependencies, and target world before confirmation. Keep packs world-scoped.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** E (P17.16–P17.20) — content and configuration

### P17.18 — Expose host, server, and network settings

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.18: expose host and network settings
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/settings.rs`, `crates/msc-agent/src/routes/networking.rs`
- **What:** Cover host setup, server root, memory, Geyser/cross-play, CurseForge key, Playit setup/reset, Xbox Broadcast state, Java resource-pack URL/required/remove, and watchdog controls through task-oriented CLI actions. Read secrets by prompt or protected input, never positional arguments or shell history. Report unsupported combinations.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** E (P17.16–P17.20) — content and configuration

### P17.19 — Make supported catalog installs inspectable

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.19: inspect catalog versions before install
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/components.rs`
- **What:** Improve Modrinth-backed add-on search/install to search → project/version detail → compatibility and dependencies → explicit confirmation → progress/result. Keep local-JAR install and installed add-on update/enable/disable/remove usable; preserve JSON automation and provider errors. Do not claim CurseForge modpack browsing.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** E (P17.16–P17.20) — content and configuration

### P17.20 — Make archive modpack import and recovery clear

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.20: clarify modpack import recovery
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/components.rs`
- **What:** Keep Modrinth `.mrpack` and CurseForge `.zip` as user-supplied archives. Show inspection, API-key setup, exact target, pack-managed consequences, progress, and author-blocked files with links and expected names. Resume with a matching local file through the existing operation-bound upload; make cancel/retry clear.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** E (P17.16–P17.20) — content and configuration

### P17.21 — Add operation and recovery controls

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.21: add operation and host reset controls
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/operations.rs`, `crates/msc-agent/src/routes/host_reset.rs`
- **What:** Inspect and cancel owned long-running operations and expose host reset with existing guards and exact confirmation. Keep local client reset distinct from host reset; preserve role permissions and no remote service-control route.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** F (P17.21–P17.24) — information and acceptance

### P17.22 — Read permitted files and built-in help

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.22: add file and help commands
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/files.rs`, `crates/msc-agent/src/routes/help.rs`
- **What:** Browse/read permitted server files without escaping API path limits. Search and read handbook topics, onboarding guidance, and router guides as terminal text. Preserve permissions and avoid a full-screen interface.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** F (P17.21–P17.24) — information and acceptance

### P17.23 — Document and audit the finished CLI

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.23: document cli usage and route audit
- **Files:** `README.md`, `docs/msc2/clients/phase17-cli.md`, `docs/msc2/clients/headless-installation.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/api-contract/openapi.json`
- **What:** Replace obsolete remote-CLI/token instructions, publish task-first local and SSH examples with edition limits, and close every row of the API-to-CLI inventory or record an owner-approved exception. Preserve desktop remote-host docs. Do not add release gates.
- **Verify:** `git diff --check`
- **Batch:** F (P17.21–P17.24) — information and acceptance

### P17.24 — Prepare physical acceptance without publishing

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.24: prepare cli physical acceptance
- **Files:** `docs/msc2/clients/phase17-cli-acceptance.md`, `docs/msc2/clients/phase17-cli.md`
- **What:** Record reproducible desktop/headless checks on macOS, Windows, and Linux: PATH, start/stop/status, reboot, local and SSH-shell authorization, wrong-user and forwarded-port refusal, representative server/player/world/backup/content commands, JSON scripting, provider failures, and uninstall ownership. Cameron runs and records results; the other agent reviews the gate. No test suite, CI gate, release tag, or publication run is implied.
- **Verify:** `rg -n 'macOS|Windows|Linux|SSH|reboot|stopped|PATH|wrong user|catalog|CurseForge' docs/msc2/clients/phase17-cli-acceptance.md`
- **Batch:** F (P17.21–P17.24) — information and acceptance

## Phase 17 completion steps

**Owner direction received 2026-09-29:** Cameron approved adding CLI commands
for the server-setup and player/pack-maintenance groups recommended in the
P17.23 audit. He approved keeping built-in gamerule lookup and the router
symptom-analysis action out of the CLI; the existing raw game-command path and
router-guide reading remain available. P17.27 records these two deliberate
exceptions and the resulting inventory in the decision register and gate.

### P17.25 — Complete server setup and inspection commands

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.25: complete server setup cli commands
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `docs/msc2/clients/phase17-cli.md`, `docs/msc2/rolling-plan.md`
- **What:** Add task commands for active-server RAM read/write, registered-server Bedrock transport, per-server Playit and Xbox Broadcast enablement, and installed system-component inspection. Use the existing routes and permissions; keep target server identity explicit, active-server effects visible, JSON output machine-readable, and provider or edition limits clear.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** G (P17.25–P17.27) — close the inventory and record approved exceptions

### P17.26 — Complete player and pack maintenance commands

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.26: add player and pack maintenance commands
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `docs/msc2/clients/phase17-cli.md`, `docs/msc2/rolling-plan.md`
- **What:** Add task commands for player skin overrides, profile hiding, unresolved Bedrock-player identification, session-history clearing, Geyser resource-pack toggles, and clearing Xbox Broadcast credentials. Preserve existing role and edition checks, operation/confirmation behavior, and JSON results; use server IDs where required and never put account secrets in arguments.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** G (P17.25–P17.27) — close the inventory and record approved exceptions

### P17.27 — Record CLI task exceptions and close the route audit

- **Status:** Done — Cameron direction, 2026-09-29
- **Commit:** P17.27: record cli task exceptions
- **Files:** `docs/msc2/msc2-decisions.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/clients/phase17-cli.md`, `docs/msc2/clients/phase17-cli-acceptance.md`, `docs/msc2/rolling-plan.md`
- **What:** Record Cameron's 2026-09-29 approval to exclude gamerule-catalog lookup and router symptom analysis from CLI task coverage while retaining their API/desktop use. Update the API-to-CLI inventory for P17.25–P17.26, decision index/history, ordered phase-step range, working gate, and physical acceptance notes. Keep desktop remote pairing, raw `msc command`, and build-only beta publishing unchanged.
- **Verify:** `git diff --check`
- **Batch:** G (P17.25–P17.27) — close the inventory and record approved exceptions

## Proposed Phase 18 — 3D world viewer

**Owner direction received 2026-09-29:** Record a 3D view for the selected
active world. The path is **Worlds → active world → 3D**; the viewer fills the
existing MSC window and Worlds tab, and exiting returns to the world list.
The map should be navigable in 3D, with terrain cutaway/depth controls, biome
and lighting views, quality controls, and a player roster. Selecting a player
flies the camera to them; following a player keeps the camera with them. The
target experience includes visible player models walking and turning as their
positions update.

**Feasibility recorded from the Vantage review:** Vantage demonstrates an
embeddable 3D viewer and Java Anvil terrain pipeline. Its viewer supports
terrain streaming, camera navigation, cave/depth views, biome and lighting
controls, player models, roster selection, click-to-fly, follow, and smooth
movement between player updates. Live movement needs a current player-position
feed; last-known positions read from saved player data can be stale. The viewer
and player UI are reusable concepts, but MSC still needs to integrate server
access, authentication, and the feed into its own API and desktop window.

**Edition scope clarified by the owner 2026-09-29:** The end-state targets
standard and modded Java servers, plus Bedrock Dedicated Server (BDS),
including players connecting from consoles. Players in the supported world
must appear as 3D models regardless of whether they joined from Java or
Bedrock. Bedrock players do not install a client-side map mod. Terrain
rendering and live player tracking are separate data paths. Bedrock needs a
LevelDB chunk-to-geometry reader/adapter; the existing community
`bedrock-render` work is a top-down tile renderer, not Vantage-style 3D
geometry. A Bedrock server-side position feed is also needed for live players.
Custom mod/add-on block-model fidelity, historical world formats, and the
first-release dimension set need explicit acceptance criteria. Missing custom
models must not prevent the map or player overlay from opening. Any Bedrock
snapshot use of experimental server APIs needs a compatibility proof before
it becomes a dependency.

**Rough proof-point guide — deliberately not a full phase plan:**

1. **Render Bedrock geometry in the intended viewer.** Read a real BDS save
   and display a small textured 3D area, including representative shapes such
   as stairs, foliage, glass, and water. This checks the chunk parser, block
   model resolver, texture source, and viewer tile format together.
2. **Read a running world safely.** Confirm BDS and modded Java terrain can be
   refreshed as saved chunks change, with acceptable CPU and memory use.
3. **Show live players across server types.** Supply current positions for
   standard Java, modded Java, and BDS; render 3D models that walk and turn;
   verify the roster, click-to-fly, follow, and console-connected players.
4. **Put the proven viewer in MSC.** Open it from the selected active world,
   fill the Worlds tab, return cleanly, stream terrain through the authenticated
   agent, and provide the agreed terrain and player controls.
5. **Measure real-world coverage.** Check representative vanilla, modded, and
   BDS worlds; document block-model fallbacks, dimensions, freshness, and
   resource costs before setting release acceptance limits.

Each checkpoint should produce a concrete result and a stop/go decision. The
next checkpoint is chosen from that result, so implementation can stop or
change direction if a core assumption fails.
At each handoff, recommend the next in-scope step based on the result and name
the UX promise it advances; do not leave Cameron to infer the route forward.

**UX check required in every Phase 18 verification:** Recheck the result
against the agreed path: Worlds → active world → 3D opens inside the MSC window
and fills the Worlds tab; the viewer supports the agreed terrain exploration;
online Java and BDS players appear as moving 3D models; roster selection flies
to a player and follow keeps the camera with them; exit returns to Worlds.
The in-window view should retain the Vantage reference's bottom navigation
toolbar, lighting and quality controls, biome legend, and available terrain
view controls, adapted to MSC's shell. Cameron's 2026-09-29 comparison images
are the visual reference for the Bedrock view and control placement.
Every step's `Verify:` must say which visible promise it advances and what
observable result supports that claim. An early technical checkpoint may prove
a named dependency rather than show the complete flow, but it must identify
the next checkpoint that will demonstrate the missing experience. A passing
parser or build check alone is not a UX pass. If a result hides a supported
player, presents stale coordinates as live, opens outside MSC, or drops a
target server type, stop and revise the step, add the required dependency, or
reject that approach before continuing.

This phase remains an isolated experiment in `feature/world-map-3d`; each
implementation checkpoint is chosen after Cameron reviews the preceding proof.

### P18.0 — Prepare isolated world-map worktree

- **Status:** Done — Cameron verified, 2026-09-29
- **Files:** `docs/msc2/rolling-plan.md`
- **What:** Create branch `feature/world-map-3d` and linked worktree
  `/Users/camerontemple/msc2-world-map` from `main` at
  `1a82357e300e3a8f70d3fae5cf19219d45c536c9`. Keep the experiment's planning
  changes and later implementation commits on this branch.
- **Verify:** `git worktree list --porcelain`
- **Batch:** setup — isolate the experiment before feasibility work
- **Commit:** P18.0: prepare isolated world map worktree

### P18.1 — Scope the 3D world viewer vertical slice

- **Status:** Done — Cameron verified section 21, 2026-09-29
- **Files:** `docs/msc2/rolling-plan.md`, `docs/msc2/msc2-engineering.md`
- **What:** Compare embedding Vantage's MIT viewer/protocol with an MSC-owned
  integration; define terrain access for standard and modded Java saves, live
  player position feeds, and the Bedrock LevelDB-to-geometry path for BDS.
  Specify the full-tab Worlds navigation, terrain controls, player roster,
  click-to-fly/follow, data freshness, authentication boundary, resource
  budget, and acceptance criteria for vanilla, modded, and console-connected
  Bedrock players. Identify whether a shared geometry format is practical,
  what custom block assets can be resolved, and what the Bedrock parser can
  safely supply. Define a user-experience check for every later checkpoint:
  name the visible promise advanced, the observable proof, and the next
  checkpoint for any gap. Do not promise that the existing raster renderer
  provides 3D geometry or that saved player records are live.
- **Result:** Section 21 of the engineering specification records the reuse
  boundaries, authenticated MSC path, separate terrain/player freshness,
  support matrix and experiment guardrails. The next candidate checkpoint is
  a real BDS save rendered as a textured 4×4-chunk 3D area in the intended
  viewer. This advances the Bedrock terrain part of the UX; running-world
  freshness and live players remain named follow-on proofs. Choose the formal
  next step after reviewing this scope.
- **Verify:** `rg -n '## 21\.|first implementation proof|4×4-chunk|UX check for every later|Worlds → selected active world' docs/msc2/msc2-engineering.md`
  then read section 21: confirm the first proof visibly advances Bedrock 3D
  exploration, names what is still missing from the agreed full-tab/live-player
  UX, and identifies safe running-world reads as the next proof.
- **Batch:** solo — scope and feasibility before implementation
- **Commit:** P18.1: scope the 3d world viewer vertical slice

### P18.2 — Prove offline BDS terrain in the Vantage viewer

- **Status:** Visual verification failed — Cameron reported grey grass and
  jagged terrain, 2026-09-29; corrective checkpoint P18.2a follows
- **Files:** `tools/world-map-proof/`, `docs/msc2/rolling-plan.md`
- **What:** Read an offline copy of Cameron's BDS save with a pinned Bedrock
  chunk reader. Resolve block shapes and textures from a separately supplied
  Bedrock resource pack, encode one 4×4-chunk Vantage tile, and open it in the
  Vantage Three.js engine with camera controls. Keep the original save and all
  generated world and texture data outside Git.
- **Result:** The selected complete area near spawn begins at chunk `(-3, 2)`.
  The exporter generated 27,146 solid faces, 529 water faces, 15 image layers,
  zero texture fallback faces and zero missing-shape blocks from actual BDS
  block states. The tool and local viewer are in
  `tools/world-map-proof/README.md`. Cameron's visual check showed that the
  **visible BDS 3D exploration** promise was not met: grass appeared grey and
  buried cube faces made the terrain jagged. The first mesher uses the top
  visible block per column;
  it omits caves, overhangs, biome tint and much water geometry. This area has
  no stairs or glass, so a separate representative Bedrock fixture is needed
  before claiming those shapes. BDS itself lacks terrain images; the proof
  uses a local copy of Mojang's sample resource pack. Running-world safety,
  changed terrain, in-window MSC navigation and live players remain open.
- **Verify:** Cameron opened the local viewer, orbited the actual BDS terrain,
  and supplied comparison screenshots on 2026-09-29. Grey grass and jagged
  terrain failed the visible BDS exploration check; P18.2a addresses these
  defects before the running-world proof.
- **Batch:** solo — offline geometry dependency before live integration
- **Commit:** P18.2: prove offline bds terrain in vantage

### P18.2a — Correct the visible BDS terrain proof

- **Status:** Visual verification failed — Cameron's 2026-09-29 screenshots
  showed grass fringe below dirt, upside-looking plants, holes through the
  terrain, and trees without visible logs; P18.2b follows
- **Files:** `tools/world-map-proof/src/render.rs`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Respond to Cameron's P18.2 screenshots. Apply the Bedrock resource
  pack's grass and foliage colormaps at a clearly labeled representative
  climate, omit buried/down faces of sampled surface cubes, and draw exposed
  water sides. Regenerate the same private 4×4 tile. Do not imply this is exact
  biome tint, full 3D terrain, or the final in-app controls.
- **Result:** The regenerated tile has 8,971 solid faces, 610 water faces,
  16 texture layers, zero texture fallbacks and zero missing shapes. The
  rendered appearance is awaiting Cameron's inspection. Exact Bedrock biome
  IDs/climate mapping, cave/overhang meshing, stairs/glass coverage, and the
  Vantage-style in-window toolbar and biome panel remain open.
  Repeated reads of the same offline copy varied by three buried grass-block
  counts while exported face counts stayed stable; investigate before claiming
  reliable running-world scans.
- **Verify:** `cd /Users/camerontemple/msc2-world-map/tools/world-map-proof/viewer && MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-world-map-output npm run dev` — reload the viewer and compare with Cameron's P18.2 screenshot: grass/foliage should no longer be grey, buried-face spikes should be reduced, and shore water should be more continuous. Orbit and zoom to find remaining holes. This advances the visible Bedrock exploration promise; the next candidate is actual biome tint and full surface fidelity with a representative stairs/glass area before live-world reads.
- **Batch:** solo — correct the failed visual proof
- **Commit:** P18.2a: correct the visible bds terrain proof

### P18.2b — Restore Bedrock block depth and texture orientation

- **Status:** Done — Cameron visually verified upright textures, tree trunks
  and full terrain depth in screenshots, 2026-09-29
- **Files:** `tools/world-map-proof/src/render.rs`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Address Cameron's second visual check. Read every block in the
  bounded 4×4 area instead of keeping only the top block of each column;
  emit exposed faces of solid blocks, tree logs, foliage, plants, and water.
  Flip decoded texture rows for Vantage's WebGL texture array so grass-side
  fringe and plant artwork appear upright. Keep this an offline proof.
- **Result:** The regenerated private tile has 67,266 solid faces, 1,596 water
  faces, 53 texture layers, zero texture fallback faces, 25 log blocks and
  zero missing-shape blocks. Export took 2.24 seconds with a 57 MB peak RSS
  on the local copy. These counts show that logs and below-canopy geometry
  reach the tile; Cameron's visual check is still needed to confirm appearance.
  Exact biome tint, complex block behavior, safe running-world snapshots,
  in-window controls and live player movement remain open.
- **Verify:** `cd /Users/camerontemple/msc2-world-map/tools/world-map-proof/viewer && MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-world-map-output npm run dev` — reload the viewer and orbit above and below the same terrain. Confirm the grass-side green fringe sits at the *top* of dirt sides, plants stand upright, logs are visible beneath leaves, and the large sky holes from P18.2a are filled by actual lower blocks. This advances the visible Bedrock exploration promise. If it passes, the next candidate proof is a representative area containing stairs and glass plus actual biome tint; if it fails, correct the observed geometry or texture defect first.
- **Batch:** solo — correct the failed visual proof
- **Commit:** P18.2b: restore bds block depth and texture orientation

### P18.2c — Prove mature BDS shapes and surface biome tint

- **Status:** Visual verification failed — Cameron's 2026-09-29 screenshots
  showed stair facing, unconnected fences, lantern UV, and crossed glass-pane
  defects; P18.2d follows
- **Files:** `tools/world-map-proof/src/main.rs`,
  `tools/world-map-proof/src/render.rs`, `tools/world-map-proof/README.md`,
  `docs/msc2/rolling-plan.md`
- **What:** Use a private offline copy of Cameron's mature MSC1 BDS world,
  centered near base coordinates `(-50, 87, 65)`. Allow a selected 4×4 chunk
  origin, convert resolver face UVs to the tile's corner and V orientation, render actual stairs
  and glass panes, and read surface biome IDs to select grass/foliage colormap
  tint using the supplied Bedrock climate definitions. Count shapes, fallbacks
  and biome IDs; keep source and output outside Git.
- **Result:** The chosen area begins at chunk `(-6, 2)` and contains 786
  placed stairs (including 413 oak and 354 stone) and 124 glass panes. The
  export has 119,399 solid faces, 2,039 water faces, 149 texture layers,
  zero texture fallbacks and zero missing-shape blocks. Surface biome IDs are
  `27` in 1,922 columns and `155` in 2,174 columns. This bounded proof uses
  the legacy birch-forest mapping for those IDs; the local Bedrock definitions
  for both variants give temperature and downfall of `0.6`. They therefore
  produce the same sampled grass `[136,186,103]` and
  foliage `[107,169,65]` tints. This establishes the biome-ID-to-colormap
  plumbing for these two IDs, not a visible boundary between different biome
  climates or general ID mapping. The selected area has panes but no full
  glass blocks; cutout pane appearance and stair orientation await Cameron's
  inspection. Cameron's first screenshots caught sideways grass and wood
  textures from unconverted face UVs; the tile was regenerated with corrected
  UV corner order and V origin and awaits reinspection. Export took 4.4
  seconds and 88 MB peak RSS. Running-world reads,
  in-window controls and live players remain open.
- **Verify:** `cd /Users/camerontemple/msc2-world-map/tools/world-map-proof/viewer && MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-base-proof/output npm run dev` — reload the standalone viewer, move near the base at `(-50, 87, 65)`, and check that grass-side fringe is horizontal at the top of dirt and wood grain is upright. Inspect oak/stone stair direction and corners, glass-pane connections and see-through cutouts, and grass/foliage color. Inspect `/private/tmp/msc-bds-base-proof/output/summary.txt` for biome IDs and fallback counts. This advances the agreed Bedrock 3D exploration promise. The next proof should address any visible shape defects; if sound, select an area with clearly different climate values and full glass blocks before claiming broad biome/glass fidelity, then move to safe running-world refresh.
- **Batch:** solo — mature offline BDS geometry proof
- **Commit:** P18.2c: prove mature bds shapes and surface biome tint

### P18.2d — Correct mature BDS state-dependent shapes

- **Status:** Partial visual verification — Cameron confirmed stairs, fences
  and lanterns on 2026-09-29, but window panes regressed to opaque stripes;
  P18.2e follows
- **Files:** `tools/world-map-proof/src/render.rs`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Correct the four defects Cameron found in P18.2c's base view.
  Translate saved stair direction before model resolution; derive fence and
  glass-pane connections from adjacent blocks when the save omits connection
  state; map the Bedrock lantern texture atlas onto the lantern's body, cap and
  hook. Regenerate the same private tile without changing the source world.
- **Result:** The regenerated area contains 121,426 solid faces, 2,039 water
  faces, 149 texture layers, zero texture fallbacks and zero missing-shape
  blocks. Export took 4.84 seconds and 88 MB peak RSS. These are structural
  checks only; stair facing, fence joins, lantern appearance and pane shape
  need Cameron's visual comparison. The pane model still includes its narrow
  center post; a perfectly flat sheet is not claimed. Lantern UV regions are
  a bounded approximation of the supplied 16×16 first animation frame, not
  full animated Bedrock model fidelity.
- **Verify:** `cd /Users/camerontemple/msc2-world-map/tools/world-map-proof/viewer && MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-base-proof/output npm run dev` — reload the viewer and compare the same base stairs, fence line, lanterns and windows against Cameron's P18.2c screenshots and his in-game knowledge. Stairs should face the intended side, fences should join, lantern bodies should show one coherent light/metal texture, and panes should connect across the opening without the old crossed shape. This advances Bedrock 3D exploration; if a shape still differs, correct that exact state/model path before running-world refresh.
- **Batch:** solo — correct failed shape verification
- **Commit:** P18.2d: correct mature bds state-dependent shapes

### P18.2e — Restore see-through Bedrock glass panes

- **Status:** Cameron visual check found one pane orientation still opaque;
  P18.2f follows
- **Files:** `tools/world-map-proof/src/render.rs`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Replace the connected pane cuboids that cropped the full glass
  texture into opaque stripes. Use neighboring saved blocks to choose a thin
  north-south or east-west plane, both for corners and isolated panes, with
  the supplied full glass texture and two visible sides. Keep the same private
  base world and 4×4 tile.
- **Result:** The regenerated tile has 119,876 solid faces, 2,039 water faces,
  149 texture layers, zero texture fallbacks and zero missing-shape blocks.
  The correction changes pane geometry only; the in-view transparency and
  connections await Cameron's comparison screenshots. It is a visual proof of
  vanilla glass panes, not all stained/tinted glass or full glass blocks.
- **Verify:** `cd /Users/camerontemple/msc2-world-map/tools/world-map-proof/viewer && MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-base-proof/output npm run dev` — reload the viewer and view the same house windows from outside and inside. The panes should form a flat sheet along each window, connect across adjacent blocks, and show the house behind the clear center with narrow visible edges. Compare with Cameron's before/after screenshots. This advances the Bedrock 3D exploration promise; if it passes, the next proof can use the gold farm's 211 full glass blocks and the complete ice-mountain 4×4 area at chunk `(-35,-17)` to assess full glass and distinct climates.
- **Batch:** solo — repair visual regression before new area proof
- **Commit:** P18.2e: restore see-through bds glass panes

### P18.2f — Use transparent texture on both pane orientations

- **Status:** Done — Cameron confirmed both pane directions clear, 2026-09-29
- **Files:** `tools/world-map-proof/src/render.rs`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** The Bedrock resource pack maps the pane's east face to the opaque
  narrow edge texture and its north/south faces to the transparent broad
  glass texture. The flat east-west pane was resolving the east face. Resolve
  all flat pane surfaces through the broad glass face while retaining their
  actual geometric orientation and two visible sides.
- **Result:** Regenerated the same complete base tile with 119,876 solid
  faces, 2,039 water faces, 148 texture layers, zero texture fallbacks and
  zero missing-shape blocks. One fewer texture layer is expected because the
  edge-only pane texture is unused. Cameron confirmed the previously opaque
  pane direction is clear.
- **Verify:** Reload
  `MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-base-proof/output npm run dev`
  from `tools/world-map-proof/viewer`; view the two perpendicular house window
  directions shown in Cameron's screenshots. Both should have thin borders
  and a clear center showing the room behind them. This advances the Bedrock
  explorable 3D world promise; the next proof should inspect the gold farm's
  full glass blocks and the ice mountain's different climate tint.
- **Batch:** solo — repair remaining pane visual defect
- **Commit:** P18.2f: fix bds pane texture orientation

### P18.2g — Inspect full glass at the BDS gold farm

- **Status:** Done — Cameron confirmed full glass looks good, 2026-09-29
- **Files:** `tools/world-map-proof/src/render.rs`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Export a separate complete 4×4 tile around Cameron's gold farm
  at `(-11, 113, -53)` from the same private offline copy. Add fallback block
  names to the summary so texture misses can be distinguished from glass
  defects. Keep the output outside Git.
- **Result:** The tile starting at chunk `(-3, -5)` has all 16 chunks and 208
  full glass blocks. It has 103,160 solid faces, 382 water faces, 159 texture
  layers and no missing shapes. Its 42 fallback faces all belong to
  `minecraft:sticky_piston_arm_collision`; they do not indicate missing glass
  textures. Cameron confirmed the full glass looks good. The `4294967295` biome value in 311
  columns is unresolved and should not be treated as a mapped climate.
- **Verify:** From `tools/world-map-proof/viewer`, run
  `MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-gold-proof/output npm run dev`.
  Navigate to the gold farm near `(-11, 113, -53)` and inspect full glass
  blocks from outside and through multiple adjoining blocks. Their interior
  must stay clear and their borders should connect without opaque faces.
  Check `/private/tmp/msc-bds-gold-proof/output/summary.txt` for counts and
  the named fallback. This advances the Bedrock 3D world fidelity promise;
  the next proof should sample the ice mountain for a different biome tint.
- **Batch:** solo — one visual proof before expanding climate mapping
- **Commit:** P18.2g: export bds gold farm glass proof

### P18.2h — Inspect Bedrock ice mountain biome tint

- **Status:** Done — Cameron confirmed the ice mountain looks good, 2026-09-29
- **Files:** `tools/world-map-proof/src/render.rs`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Export a complete 4×4 tile around Cameron's ice mountain at
  `(-516, 188, -225)` using the same offline BDS copy. For this bounded proof,
  map surface biome IDs `183`, `185`, and `189` to frozen peaks, grove, and
  stony peaks using the saved world's Bedrock biome definitions for climate
  values. Clamp freezing temperatures before sampling the supplied grass and
  foliage colormaps. Keep unmapped IDs on the existing representative tint
  and identify them in the summary.
- **Result:** Tile origin `(-35, -17)` contains all 16 chunks, 182,193 solid
  faces, 2,576 water faces, 79 texture layers, and no texture or shape
  fallbacks. It contains 493 surface columns with ID `183`, 109 with `185`,
  and 1,751 with `189`; the mapped grass colors are `[128, 180, 150]` for
  freezing biomes and `[154, 189, 74]` for stony peaks. Nine columns have
  unresolved ID `4294967295`; IDs `4` and `188` still use the representative
  tint. The numeric ID mapping is provisional until version-aware Bedrock
  registry handling is proven. Cameron confirmed the visible mountain result.
- **Verify:** From `tools/world-map-proof/viewer`, run
  `MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-ice-proof/output npm run dev`.
  Look around `(-516, 188, -225)` for actual ice/snow geometry and compare
  grass/foliage near the frozen and stony areas; the latter should have a
  distinct warmer tint. Inspect `summary.txt` for source biome counts and
  mapped colors. This advances the Bedrock 3D world's biome-color promise.
  Next proof: version-aware biome ID mapping and 3D biome sampling, or a
  focused visual correction if this tile shows a defect.
- **Batch:** solo — distinct climate proof
- **Commit:** P18.2h: render bds ice mountain biome tint

### P18.2i — Read Bedrock biome at rendered block height

- **Status:** Done — Cameron confirmed the mountain view looks good, 2026-09-29
- **Files:** `tools/world-map-proof/src/render.rs`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Use the BDS reader's decoded 3D biome storages to assign a biome
  ID to each block position rather than repeating the column's surface ID
  throughout its depth. Preserve 2D save compatibility by repeating legacy
  column values vertically. Report save version, surface ID counts, and IDs
  at non-air block positions for the same complete ice-mountain tile.
- **Result:** The regenerated tile still has 182,193 solid faces, 2,576 water
  faces, 79 textures and no fallback or missing-shape blocks. Surface IDs are
  unchanged, but block-height sampling finds ID `190` in 324,336 non-air
  blocks, plus ID `188` in 398,332. The earlier surface-only approach would
  have hidden `190` and assigned mountain surface IDs throughout cave depth.
  The save reports `lastOpenedWithVersion` `[1,26,31,1,0]` and storage version
  `10`. Those fields are evidence of the save format, not an ID-to-name
  registry. The bounded tint mapping remains provisional.
- **Verify:** Read
  `/private/tmp/msc-bds-ice-3d-biome-proof/output/summary.txt` and compare
  surface versus non-air-block ID counts. Open with
  `MSC_WORLD_MAP_PROOF_OUTPUT=/private/tmp/msc-bds-ice-3d-biome-proof/output npm run dev`
  from `tools/world-map-proof/viewer`; inspect the mountain surface and cut
  sides for sudden tint or geometry regressions. This advances the Bedrock
  biome fidelity promise at depth. Next proof: establish a version-aware
  numeric biome ID registry before treating these names as generally valid.
- **Batch:** solo — height sampling proof before broader ID mapping
- **Commit:** P18.2i: sample bds biomes at block height

### P18.2j — Audit Bedrock biome ID registry source

- **Status:** Done — Cameron directed the registry proof, 2026-09-29
- **Files:** `docs/msc2/bedrock-biome-registry.md`,
  `tools/world-map-proof/src/render.rs`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Trace whether the saved world, supplied BDS packs, or a generated
  BDS ID map can name numeric biome IDs for the exact world version. Record
  the source, version mismatch, and the rule for unknown or custom IDs.
  Label the existing export summary's mapping as provisional.
- **Result:** The save reports `1.26.31`; the available CC0 BedrockData map,
  generated by a BDS mod, reports `1.26.30`. It corroborates the mountain's
  sampled IDs but is not an exact-version registry. The chunk reader exposes
  numeric biome IDs and no names. The copied server has a BDS binary, but no
  exact-version ID artifact has yet been extracted from it. The current
  mountain tint remains a bounded visual proof, not a general biome legend.
- **Verify:** Read `docs/msc2/bedrock-biome-registry.md` and the regenerated
  `/private/tmp/msc-bds-ice-3d-biome-proof/output/summary.txt`. Check that
  the version mismatch and provisional status are explicit. This protects
  the Worlds-tab biome controls from showing guessed names. Next proof:
  obtain an exact-version registry artifact from BDS and reject mismatched
  registry versions in the exporter.
- **Batch:** solo — establish registry boundary before generalizing tint
- **Commit:** P18.2j: audit bds biome registry source

### P18.2k — Reject mismatched Bedrock biome registries

- **Status:** Done — Cameron confirmed the expected version rejection,
  2026-09-29
- **Files:** `tools/world-map-proof/Cargo.toml`, `tools/world-map-proof/Cargo.lock`,
  `tools/world-map-proof/src/main.rs`, `tools/world-map-proof/src/render.rs`,
  `tools/world-map-proof/README.md`, `docs/msc2/bedrock-biome-registry.md`,
  `docs/msc2/rolling-plan.md`
- **What:** Accept an optional private JSON biome registry carrying a five-part
  BDS version, source, hash, and unique name-to-ID map. Compare it with
  `lastOpenedWithVersion` before export; reject version mismatch or conflict
  with the five IDs currently driving bounded tint. Keep exports without a
  registry labeled provisional.
- **Result:** The supplied save reports `[1,26,31,1,0]`; the available
  BedrockData registry is for `[1,26,30,31,0]`. The latter is wrapped in a
  private file outside Git for a rejection check. The supplied BDS executable
  is version `1.26.31.1`. Docker was started, but the copied binary lacks
  exported biome symbols, and the official archive download stalled before
  any bytes transferred. An exact registry remains uncollected.
  The new gate checks version and shape; it does not prove source authenticity
  or mixed-version chunk compatibility. Cameron's run rejected
  `[1,26,30,31,0]` against `[1,26,31,1,0]` as intended.
- **Verify:** Run the exporter with
  `/private/tmp/msc-biome-registry-1.26.30.json` as the fifth argument after
  `-35,-17`; it should stop with a `biome registry version ... does not match
  save version ...` error and write no new tile. Run without that argument and
  confirm `summary.txt` still says `provisional`. This protects the Bedrock
  Worlds-tab biome legend from silent nearby-version assumptions. The exact
  map still requires a BDS package or Linux host suitable for the mapping
  mod. While that source is unavailable, the next independent UX proof should
  establish safe refresh from a running BDS world using consistent snapshots.
  Named biome legends remain gated; upgrades and custom IDs remain open.
- **Batch:** solo — enforce registry provenance boundary
- **Commit:** P18.2k: guard bds biome registry version

### P18.2l — Capture one consistent running-BDS map snapshot

- **Status:** Awaiting Cameron verification
- **Files:** `crates/msc-agent/src/backup_operations.rs`,
  `crates/msc-agent/src/main.rs`,
  `crates/msc-agent/src/routes/lifecycle.rs`,
  `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/cli/mod.rs`,
  `docs/msc2/api-contract/openapi.json`, `tools/world-map-proof/README.md`,
  `docs/msc2/rolling-plan.md`
- **What:** Add a proof-only, Worlds-authorized `msc world map-snapshot`
  operation for the currently running, MSC-managed BDS server. Reuse its
  console boundary to send `save hold`, require `save query` readiness, copy
  only the configured active world into an owner-private temporary directory,
  then dispatch `save resume` before reporting the path and measured copy size
  and hold time. Refuse symlinks, special files, an unsafe level name, more
  than 2 GiB, or a copy lasting over 30 seconds. A timeout or copy error
  leaves no published map artifact and reports whether resume was dispatched.
  Cameron checks that the server is writable again. This deliberately does
  not change MSC's existing best-effort backup contract. Disable Clap's
  automatic help subcommand, which collides with MSC's existing `help` command
  in debug builds and otherwise blocks the new CLI proof command.
- **Verify:** Start a disposable Bedrock world through MSC and make a visible
  block change. After loading the new development agent, run
  `cd /Users/camerontemple/msc2-world-map && target/debug/msc --json world map-snapshot`.
  The completed operation must report `worldPath`, `bytesCopied`, and
  `holdMillis`; BDS stays running and writable. Run
  `tools/world-map-proof/target/release/msc-world-map-proof <worldPath>
  /private/tmp/msc-bedrock-samples/resource_pack <new-private-output> <4x4-origin>`
  and open that output in the proof viewer to find the changed block.
  This advances fresh Bedrock terrain in the eventual Worlds-tab 3D view;
  it does not show live player movement. If the snapshot cannot be opened or
  the server cannot resume, stop here. If it passes, the next proof is
  versioned tile replacement in the open viewer, then measure refresh cost.
- **Batch:** solo — prove the BDS hold/copy/resume boundary before auto refresh
- **Commit:** P18.2l: capture running bds map snapshot

### P18.2m — Repair snapshot copy stack overflow

- **Status:** Awaiting Cameron verification
- **Files:** `crates/msc-agent/src/backup_operations.rs`,
  `docs/msc2/rolling-plan.md`
- **What:** The first live snapshot attempt ended with an agent stack overflow,
  disconnecting the MSC-managed BDS server. The recursive world copy reserved
  a 1 MiB buffer in every call frame. Allocate that buffer on the heap and
  reject directory trees deeper than 32 levels. The live proof remains open
  until Cameron confirms the agent and BDS both survive a new capture.
- **Verify:** After loading the repaired development agent with an MSC-managed
  BDS server running, run
  `cd /Users/camerontemple/msc2-world-map && MSC2_DATA_DIR="$HOME/Library/Application Support/MSC 2" target/debug/msc --json world map-snapshot`.
  Confirm the operation reports a `worldPath`, the agent and BDS stay running,
  and a new block can still be placed. Then export the snapshot near the
  diamond pillar and confirm that its shape appears in the 3D proof viewer.
  This advances fresh Bedrock terrain in the eventual Worlds-tab 3D view;
  it does not yet provide automatic refresh or live player movement.
- **Batch:** solo — repair live snapshot safety before further refresh work
- **Commit:** P18.2m: repair bds snapshot copy stack overflow

### P18.2n — Replace saved terrain in the open proof viewer

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/viewer/main.ts`,
  `tools/world-map-proof/viewer/index.html`,
  `tools/world-map-proof/viewer/style.css`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Let the standalone proof viewer load a second, versioned BDS tile
  export on demand without reopening the page. Fetch the terrain and texture
  files before replacing the scene, and restore the viewer camera afterward.
  This keeps the proof explicitly labeled as saved terrain, not live players.
- **Verify:** While viewing the first diamond pillar export, change blocks near
  `-5, 88, 0` on the running BDS server. Capture a new `msc world map-snapshot`
  and export its `worldPath` to
  `/private/tmp/msc-bds-live-proof/output/revisions/after` with origin `-2,-1`
  as described in `tools/world-map-proof/README.md`. Enter `after` in the open
  viewer and select **Replace terrain**. The change appears without a page
  reload, the camera stays near the pillar, and BDS remains writable. This
  advances fresh Bedrock terrain within the future Worlds-tab 3D view;
  automatic refresh, full-world streaming, and live players remain separate
  proofs. If the new revision cannot load, the existing view remains visible.
- **Batch:** solo — prove in-place tile replacement before timing refresh
- **Commit:** P18.2n: replace saved terrain in open viewer

### P18.2o — Measure one saved-terrain refresh

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/measure_refresh.py`,
  `tools/world-map-proof/viewer/main.ts`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Time a single manual BDS snapshot and 4×4 tile export, publish the
  result under a unique revision, and show browser load time when the open
  viewer replaces it. Report server save hold separately from total ready
  time. The tool does not schedule captures or infer a safe cadence from one
  measurement.
- **Verify:** Make a visible change near the diamond pillar and run the
  `measure_refresh.py` command in `tools/world-map-proof/README.md` while BDS
  and the proof viewer remain open. Enter its printed revision in the viewer
  and select **Replace terrain**. Record `snapshotMs`, `holdMillis`,
  `exportMs`, `readyMs`, and browser load time; confirm the block change appears
  and the server remains writable. This quantifies one saved-terrain update
  toward the Worlds-tab 3D promise. It does not measure player movement,
  automatic refresh load, or full-world tile streaming.
- **Batch:** solo — measure one local refresh before cadence decisions
- **Commit:** P18.2o: measure saved terrain refresh cost

### P18.2p — Repeat refresh measurements on the mature BDS world

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/measure_refresh_series.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Collect 2–5 owner-triggered snapshot and export measurements on
  one 4×4 developed area, stopping after any failure. Report per-sample values
  and minimum, median, and maximum rather than treating one quick capture as
  a safe automatic interval. Cameron is importing the mature Bedrock world;
  this tool does not change server selection or automate the captures.
- **Verify:** With the mature `theboyslatest` world imported and running through
  MSC, use the three-sample command in `tools/world-map-proof/README.md` for
  the base at 4×4 origin `(-6, 2)`. Press Enter separately for each capture,
  confirm BDS remains running and writable after each, and record the printed
  timing range. This measures variation for one developed Bedrock tile toward
  the Worlds-tab 3D map. It does not yet measure whole-world streaming or
  player motion, and the 2 GiB or 30-second snapshot limits may stop the proof.
- **Batch:** solo — characterize developed-world cost before cadence choice
- **Commit:** P18.2p: collect manual bds refresh series

### P18.2q — Correct import progress server type

- **Status:** Awaiting Cameron verification
- **Files:** `crates/msc-agent/src/routes/lifecycle.rs`,
  `crates/msc-agent/src/routes/servers/import.rs`,
  `crates/msc-agent/src/routes/servers.rs`, `docs/msc2/rolling-plan.md`
- **What:** Cameron's mature Bedrock import exposed a progress line that said
  “Importing Paper server” while the review correctly identified Bedrock.
  Start normal imports with a Java or Bedrock progress label from the selected
  type. Use a neutral label for recovery rescans and transfer packages, whose
  contents may span server types. Existing in-flight operations retain their
  original status line; the correction applies to later imports.
- **Verify:** After loading the updated agent, start a new Bedrock folder import
  through MSC. Its in-progress line says “Importing Bedrock server.” A Java
  folder import says “Importing Java server.” This keeps the import feedback
  accurate while preparing the mature world used for the Worlds-tab 3D map
  measurements; it does not change the snapshot or rendering path.
- **Batch:** solo — correct import feedback found during mature BDS setup
- **Commit:** P18.2q: label import progress by server type

### P18.2r — Export only changed Bedrock tiles

- **Status:** Done — Cameron verified unchanged and changed tile selection,
  visible block change, and continuing BDS writes, 2026-09-29
- **Files:** `tools/world-map-proof/src/main.rs`,
  `tools/world-map-proof/export_changed_tiles.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Compare the terrain, biome, and subchunk records of two consistent
  BDS snapshots across a bounded grid of 4×4-chunk tiles. Export only tiles
  containing changed chunks. Ignore entity and player records so player motion
  does not trigger a terrain rebuild. The dirty tile is still rebuilt in full;
  full-save snapshot copying and multi-tile viewer loading remain separate
  work. Do not read a running LevelDB save directly.
- **Verify:** Capture two snapshots of the running mature Bedrock world through
  `MSC2_DATA_DIR="$HOME/Library/Application Support/MSC 2" target/debug/msc --json world map-snapshot`,
  recording each result's `worldPath`. First make no terrain edit, then run the
  command in `tools/world-map-proof/README.md` with a fresh output directory;
  it should export no tiles. Capture a third snapshot after changing one block
  within the base tile beginning at `(-6, 2)`, and run the same command using
  the second and third paths and a new output directory. It should export
  `tile_-6_2` and skip its unchanged neighboring tiles. Confirm BDS remains
  writable and the block appears when that tile is viewed. This makes saved
  terrain refresh work proportional to dirty tiles toward the Worlds-tab 3D
  view. It does not establish live player motion or whole-world streaming.
- **Batch:** solo — prove selective tile rebuild before automatic refresh
- **Commit:** P18.2r: export only changed bedrock tiles

### P18.3a — Prove current BDS player samples, including console clients

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/bedrock-player-feed/manifest.json`,
  `tools/world-map-proof/bedrock-player-feed/scripts/main.js`,
  `tools/world-map-proof/install_bedrock_player_feed.py`,
  `tools/world-map-proof/watch_bedrock_player_feed.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Package a minimal stable Bedrock behavior pack that emits a complete
  current player roster once per second to the server log. Each sample carries
  dimension, XYZ, pitch, yaw, game tick, and sample time; empty rosters and
  missing updates are explicit. Install into one stopped BDS world with a pack
  list backup, then inspect the stream through MSC's existing authenticated
  console. This isolates the biggest Bedrock player-feed uncertainty without
  presenting saved coordinates as live or changing the product UI yet.
- **Verify:** Follow the stop, install, restart, and `msc console follow`
  commands in `tools/world-map-proof/README.md`. Join with a console-connected
  Bedrock client, walk and turn, and verify fresh samples change XYZ and yaw
  about once per second; the player must disappear from the sampled roster
  after disconnect. Confirm no Beta APIs experiment or client-side map mod was
  required. This advances the visible promise of moving Bedrock player models
  in the Worlds-tab 3D map by proving their current server-side positions. If
  the feed fails to load or omits console players, stop and investigate before
  building the authenticated player bridge or 3D overlay. The next proof is
  consuming this feed in MSC and animating a model with stale-state handling.
- **Batch:** solo — prove BDS player positions before MSC viewer integration
- **Commit:** P18.3a: probe live bds player positions

### P18.3b — Restore authenticated BDS player-feed observation

- **Status:** Awaiting Cameron verification
- **Files:** `crates/msc-agent/src/auth.rs`,
  `tools/world-map-proof/watch_bedrock_player_feed.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** P18.3a's pack installed, but `msc console follow` obtained a stream
  ticket and then received 401 on WebSocket upgrade. Accept the console stream
  path both before and after Axum's `/v1` nesting in the ticket gate. Provide
  a polling watcher over the already authenticated console-tail route so the
  current installed agent can expose the pack samples without a service reload.
- **Verify:** Start the BDS server with the P18.3a pack and run
  `cd /Users/camerontemple/msc2-world-map && python3 tools/world-map-proof/watch_bedrock_player_feed.py --poll --server theboyslatest`.
  Join from a console client; current XYZ and yaw should change as the player
  moves and turns, and the next roster should be empty after disconnect.
  Once the corrected agent is loaded, `msc console follow` should also connect
  without 401. This unblocks observation of current Bedrock players toward
  moving 3D models; it does not itself draw models or verify Java players.
- **Batch:** solo — unblock the BDS player feed proof before model integration
- **Commit:** P18.3b: restore console ticket path and poll fallback

### P18.3c — Draw a fresh Bedrock player in the proof viewer

- **Status:** Awaiting Cameron verification
- **Files:** `crates/msc-agent/src/routes/lifecycle.rs`,
  `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/cli/mod.rs`,
  `tools/world-map-proof/viewer/main.ts`,
  `tools/world-map-proof/viewer/vite.config.ts`,
  `tools/world-map-proof/viewer/index.html`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Accept the installed BDS pack's current player samples into
  memory for the selected server run. Expose a Worlds-permission authenticated
  endpoint and CLI command. Clear the roster when the server changes/stops or
  the feed is over five seconds old. Poll the authorized CLI from the local
  proof viewer and interpolate one or more 3D player models over the saved
  Bedrock terrain. Keep player positions out of the exported terrain files.
- **Verify:** Follow **Live Bedrock player overlay (P18.3c)** in
  `tools/world-map-proof/README.md`. Near the exported base, a Bedrock player
  walking and turning should move and turn a 3D model in the viewer; leaving
  or stopping the feed should remove the model and show an unavailable status.
  This advances the agreed live-moving-player UX promise for BDS. It does not
  prove Java, console clients specifically, roster click-to-fly/follow, or the
  final Worlds-tab shell. Next choose a proof for the missing player interaction
  or Java feed based on Cameron's visual result.
- **Batch:** solo — first authenticated live model bridge
- **Commit:** P18.3c: draw fresh bedrock players in proof viewer

### P18.3d — Keep the proof player poll within CLI authorization limits

- **Status:** Awaiting Cameron verification
- **Files:** `crates/msc-agent/src/cli/mod.rs`,
  `tools/world-map-proof/viewer/vite.config.ts`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Cameron observed the accurate moving BDS model, but the proof
  viewer's one-second CLI process loop minted a fresh five-minute credential
  each time. The agent caps live CLI credentials at 128, so it refused a later
  terrain snapshot. Keep one polling CLI process, renew its credential before
  expiry, and let the local viewer serve only a fresh cached result. Retain
  explicit stale status if the CLI or agent goes away.
- **Verify:** Restart the local proof viewer with the new CLI binary. Confirm
  the BDS model keeps moving, then run the `measure_refresh.py` command in
  **Live Bedrock player overlay (P18.3c)** while the viewer remains open.
  It should return a revision without a CLI credential-limit error; entering
  that revision and selecting **Replace terrain** should reveal newly placed
  blocks while the player continues moving. This preserves the live-player UX
  and the saved-terrain refresh path together. Next prove roster click-to-fly
  and follow, then expand the live feed to Java.
- **Batch:** solo — correct the proof bridge resource use
- **Commit:** P18.3d: reuse local cli authorization for player polling

### P18.3e — Add live-player fly and follow controls

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/viewer/main.ts`,
  `tools/world-map-proof/viewer/index.html`,
  `tools/world-map-proof/viewer/style.css`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Add a compact live Bedrock roster to the existing terrain proof.
  Each player has a Fly action that moves the camera to their current model
  and a Follow toggle that tracks their interpolated position. Manual camera
  navigation releases follow. Leaving the roster, disconnecting, or losing
  feed freshness clears the player and ends follow. Keep the roster in the
  proof viewer only; this does not imply the full MSC Worlds-tab shell.
- **Verify:** With a live Bedrock player near the exported base, click **Fly**
  and confirm the camera moves to the visible model. Click **Follow**, move in
  game, and confirm the camera tracks; orbit manually and confirm follow ends.
  Disconnect and confirm the roster clears and the camera releases. This
  advances player roster selection, click-to-fly, and follow from the agreed
  UX. Next prove Java player feeds and whether the same roster behavior works
  across server types before integrating the full shell.
- **Batch:** solo — validate BDS roster and camera interaction
- **Commit:** P18.3e: add live bedrock player fly and follow

### P18.3f — Center the live-player camera actions

- **Status:** Superseded after Cameron reported no visible change; the player
  remained at the bottom-right of the viewport.
- **Files:** `tools/world-map-proof/viewer/main.ts`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Correct the camera targeting reported during P18.3e verification:
  Fly and Follow could move the view while leaving the live model near the
  viewport edge. Center the model's vertical midpoint, let Follow update the
  map pivot without resetting camera controls every frame, and suspend the
  terrain-height pivot while targeting a player. Restore ordinary terrain
  camera behavior when the user navigates away, stops following, or the player
  feed becomes unavailable.
- **Verify:** With one live Bedrock player visible, click **Fly** and confirm
  the player model is centered in the viewport. Click **Follow**, move and jump
  in game, and confirm the model stays centered while the terrain moves under
  it. Orbit or pan manually and confirm follow releases and ordinary terrain
  framing resumes. Toggle follow off, then disconnect or stop the feed and
  confirm the roster clears. This advances click-to-fly and follow toward the
  agreed in-app UX; it does not prove Java feeds or final Worlds-tab
  integration. Choose the next proof from this visual result.
- **Batch:** solo — correct live Bedrock camera framing
- **Commit:** P18.3f: center live player camera actions

### P18.3g — Aim roster navigation at the rendered player

- **Status:** Superseded after Cameron's screenshot showed the view aimed upward
  into the roof; camera pitch used the wrong vertical sign.
- **Files:** `tools/world-map-proof/viewer/main.ts`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** P18.3f did not change the visible framing. Reconcile Vantage's two
  camera modes before roster navigation: in map mode `controls.position` is
  the look-at pivot, while in free-flight mode it is the camera eye. Switch
  Fly/Follow to map navigation and calculate the view direction from the
  current camera eye to the rendered model midpoint. Preserve the chosen zoom,
  track the model's interpolated position, and suspend terrain-height
  adjustment while locked.
- **Verify:** Rebuild/restart the proof viewer and confirm the new camera
  actions center on the model rather than the former bottom-right point. Click
  **Fly**, then **Follow** and walk/jump; the model should remain at viewport
  center while terrain moves. Pan manually and confirm follow releases and
  terrain framing resumes. This advances the visible click-to-fly/follow UX.
  If centering still fails, capture the viewer with the roster visible and
  record the camera mode and player's feed coordinates before choosing another
  camera change. Do not move to Java integration until this BDS interaction is
  visually verified.
- **Batch:** solo — correct and recheck live Bedrock camera targeting
- **Commit:** P18.3g: aim player camera actions at rendered model

### P18.3h — Correct the player camera pitch direction

- **Status:** Superseded after Cameron's screenshots showed the player still
  pinned near the lower-right corner. A camera projection readout then showed
  the player at normalized screen center (0, 0), revealing a canvas sizing
  issue rather than another camera direction error.
- **Files:** `tools/world-map-proof/viewer/main.ts`,
  `docs/msc2/rolling-plan.md`
- **What:** Fix the vertical sign in the conversion from camera-to-player
  direction to Vantage's map angle. The prior conversion aimed upward and
  placed the camera inside the roof. Keep the current view aimed down toward
  the player, then center the model midpoint for Fly/Follow.
- **Verify:** Rebuild/restart the proof viewer. Click **Fly** and confirm the
  camera moves toward the player from above and centers the model without
  entering the roof. Click **Follow**, walk/jump, and confirm the model stays
  centered. Pan manually and confirm follow releases. This directly advances
  the requested click-to-fly/follow UX; defer Java feed work until the BDS
  camera behavior is visually verified.
- **Batch:** solo — correct camera pitch sign
- **Commit:** P18.3h: correct player camera pitch direction

### P18.3i — Size the proof canvas to its visible viewport

- **Status:** Done — Cameron confirmed Fly and Follow are centered, 2026-09-30
- **Files:** `tools/world-map-proof/viewer/style.css`,
  `docs/msc2/rolling-plan.md`
- **What:** The Vantage renderer calls `setSize(..., false)` and uses the device
  pixel ratio for its drawing buffer. Without an explicit CSS size, the canvas
  retains that larger intrinsic size and the viewer clips it. Cameron's live
  screenshot showed the camera projecting the player to (0, 0), while the
  visible model sat at the lower-right edge. Set the canvas CSS width and
  height to the viewer dimensions so its buffer scales into the viewport.
  Remove the temporary camera readout used to isolate the mismatch.
- **Verify:** Reload the proof viewer on the same Retina display. Click
  **Follow** and confirm the player appears near the center and stays there
  while moving. Click **Fly** and confirm it approaches the player. Pan or
  orbit manually and confirm Follow releases. Cameron confirmed both Fly and
  Follow work.
- **Batch:** solo — correct proof viewer canvas sizing
- **Commit:** P18.3i: size proof canvas to viewport

### P18.3j — Prove modded Java player samples on ATM10 Lite

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/java-player-feed/scripts/server.js`,
  `tools/world-map-proof/install_java_player_feed.py`,
  `tools/world-map-proof/remove_java_player_feed.py`,
  `tools/world-map-proof/watch_java_player_feed.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Add a separate, removable KubeJS server script that writes a
  complete current player roster to the Java server log about once per second.
  Each sample includes name, UUID, dimension, XYZ, yaw, pitch, game tick, and
  sample time; empty rosters and a stopped or stale feed are observable. The
  installer must refuse a running server, preserve the exact prior state, and
  avoid edits to ATM10 Lite's existing KubeJS scripts or world data. Prove the
  1.21.1 NeoForge/KubeJS runtime on the selected modded server before claiming
  broader Java support. Keep the probe output local in MSC's authenticated
  console; do not add player positions to terrain exports.
- **Verify:** Follow **Modded Java live player feed proof (P18.3j)** in
  `tools/world-map-proof/README.md`. With ATM10 Lite stopped in MSC, install
  the isolated probe and start the server. Follow its authenticated console
  and join with a Java client. Walk, turn, and change dimension if convenient;
  verify fresh JSON
  rosters update XYZ/yaw/pitch/dimension about once per second and become empty
  after disconnect. Confirm the server remains healthy, then remove the probe
  while stopped and confirm the original KubeJS files are unchanged. This
  advances live-player proof for one modded Java runtime; it does not establish
  Paper or vanilla Java support or terrain fidelity for modded blocks. Next
  prove the Paper feed path and compare the feed contract before adding Java
  players to the viewer.
- **Batch:** solo — establish the modded Java live-position path before
  Paper or viewer integration
- **Commit:** P18.3j: prove modded java player samples

### P18.3k — Use the configured Paper world for downgrade backups

- **Status:** Awaiting Cameron verification
- **Files:** `crates/msc-agent/src/routes/versions.rs`,
  `docs/msc2/rolling-plan.md`
- **What:** The Paper server has `level-name=Paper` and a real `Paper/` world,
  but the downgrade backup route passed no level name and searched for the
  default `world/` folder. Read the configured Java level name through the
  existing backup helper before creating the required safety archive. Keep the
  downgrade blocked if that archive still cannot be created.
- **Verify:** Rebuild and reload the local agent, then retry the Paper version
  change while the server is stopped. Confirm the pre-downgrade backup is
  created for `Paper/` and the version change progresses past the backup step.
  The current world was last opened on 26.2; use a separate world or restored
  backup before starting that world under an older Minecraft version.
- **Batch:** solo — fix Paper downgrade backup world selection
- **Commit:** P18.3k: use configured world for downgrade backup

### P18.3l — Prove live player samples on Paper

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/paper-player-feed/`,
  `tools/world-map-proof/install_paper_player_feed.py`,
  `tools/world-map-proof/remove_paper_player_feed.py`,
  `tools/world-map-proof/watch_java_player_feed.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Use the managed Paper server for the next Java player-feed proof.
  Build a removable Paper plugin that emits the same local player-sample
  envelope as the ATM10 Lite probe, using Paper's API for UUID, name,
  dimension, position, yaw, pitch, tick, and sample time. Reuse the
  authenticated console watcher. Keep world data and other plugins intact;
  install and remove the probe only while Paper is stopped. Check the API and
  Java toolchain against the installed Paper version before implementation.
- **Verify:** Start Paper through MSC with the probe installed, join, move,
  turn, and disconnect. Confirm fresh samples track the player and then show
  an empty roster. Confirm the existing world and plugins are unchanged after
  stopping Paper and removing the probe. Compare the sample contract with
  ATM10 Lite before adding Java players to the viewer.
- **Batch:** solo — establish the Paper live-position path
- **Commit:** P18.3l: prove paper player samples

### P18.3m — Prove live player samples on Fabric

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/fabric-player-feed/`,
  `tools/world-map-proof/install_fabric_player_feed.py`,
  `tools/world-map-proof/remove_fabric_player_feed.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Add a temporary Fabric server mod that uses Fabric API's end-tick
  event to emit a complete roster every 20 ticks. Keep the sample schema aligned
  with Paper and KubeJS, install only into a stopped Minecraft 26.2 Fabric
  server, and checksum any official Fabric API dependency the installer adds.
  Preserve pre-existing mods and make removal refuse changed proof artifacts.
- **Verify:** Follow **Fabric live player feed proof (P18.3m)** in
  `tools/world-map-proof/README.md`. Start the selected Fabric server through
  MSC; join with a Minecraft 26.2 Java client; walk and turn; confirm fresh
  samples update XYZ and yaw/pitch, and disconnect to confirm an empty roster.
  Stop Fabric and remove only the probe and any Fabric API jar installed by the
  probe. This proves the selected Fabric Loader/API combination, not other
  Fabric versions or terrain rendering.
- **Batch:** solo — prove Fabric player samples before testing the other Java
  server flavors
- **Commit:** P18.3m: prove fabric player samples

### P18.3n — Prove live player samples on Forge

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/forge-player-feed/`,
  `tools/world-map-proof/install_forge_player_feed.py`,
  `tools/world-map-proof/remove_forge_player_feed.py`,
  `tools/world-map-proof/watch_java_player_feed.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Add a temporary server-side Forge mod using Forge's post-server
  tick event. Compile it against the managed Minecraft 26.3 / Forge 66.0.8
  runtime, emit the shared player roster once per second, and safely install
  and remove it only while Forge is stopped. Preserve the world and existing
  mods.
- **Verify:** Follow **Forge live player feed proof (P18.3n)** in
  `tools/world-map-proof/README.md`. Join with a Minecraft 26.3 client, move,
  turn, and disconnect; confirm the authenticated watcher reports fresh
  positions/look and then an empty roster. Stop Forge and remove the probe.
- **Batch:** solo — establish Forge player-feed compatibility
- **Commit:** P18.3n: prove forge player samples

### P18.3o — Prove live player samples on Vanilla

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/vanilla-player-feed/`,
  `tools/world-map-proof/install_vanilla_player_feed.py`,
  `tools/world-map-proof/remove_vanilla_player_feed.py`,
  `tools/world-map-proof/watch_vanilla_player_feed.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Use a temporary Vanilla datapack to sample UUID, position, and
  rotation across the three standard dimensions, then normalize its command
  feedback through an authenticated console watcher. Target the managed
  official Minecraft 26.3 bundle and pack format 121. Install and remove only
  while the server is stopped; preserve the world and server configuration.
- **Verify:** Follow **Vanilla live player feed proof (P18.3o)** in
  `tools/world-map-proof/README.md`. Start the selected Vanilla server through
  MSC, join with a 26.3 client, move, turn, and disconnect. Confirm fresh
  positions/look and then an empty roster. Confirm datapack command feedback
  reaches the watcher in the expected form, stop the server, and remove the
  probe. This console-feedback route has not yet been live verified.
- **Batch:** solo — establish the vanilla datapack and console-feedback path
- **Commit:** P18.3o: prove vanilla player samples

### P18.3p — Prove live player samples on Purpur

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/purpur-player-feed/`,
  `tools/world-map-proof/install_purpur_player_feed.py`,
  `tools/world-map-proof/remove_purpur_player_feed.py`,
  `tools/world-map-proof/watch_java_player_feed.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Build a temporary plugin against the API bundled with the selected
  Purpur jar. The currently managed instance is Minecraft 1.21.11 in the
  `pupur` directory; the helper detects the selected bundle and compiles
  against its API. Install and remove only while stopped, preserving existing
  plugins and world data.
- **Verify:** Follow **Purpur live player feed proof (P18.3p)** in
  `tools/world-map-proof/README.md`. Start through MSC, join with a matching
  client, move and turn, then disconnect. Confirm fresh samples and an empty
  roster in the authenticated watcher, stop Purpur, and remove the plugin.
  Compilation has been checked on the existing 1.21.11 bundle; live behavior
  and any newly created Purpur version remain to be verified.
- **Batch:** solo — establish Purpur player-feed compatibility
- **Commit:** P18.3p: prove purpur player samples

### P18.3q — Prove live player samples on NeoForge without KubeJS

- **Status:** Awaiting Cameron verification
- **Files:** `tools/world-map-proof/neoforge-player-feed/`,
  `tools/world-map-proof/install_neoforge_player_feed.py`,
  `tools/world-map-proof/remove_neoforge_player_feed.py`,
  `tools/world-map-proof/watch_java_player_feed.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Add a temporary NeoForge mod to the managed **Minecraft 26.2 /
  NeoForge 26.2.0.88** instance named `Neoforge`, which has no KubeJS. Compile
  against that server's installed NeoForge and patched Minecraft APIs, emit
  the shared roster once per second, and install/remove only while the server
  is stopped. Preserve world data and existing mods.
- **Verify:** Follow **NeoForge without KubeJS live player feed proof
  (P18.3q)** in `tools/world-map-proof/README.md`. Join with a 26.2 client,
  move and turn, then disconnect. Confirm fresh positions/look and an empty
  roster through the authenticated watcher. Stop NeoForge and remove the
  probe. This checks the selected NeoForge build, not all NeoForge versions or
  terrain rendering.
- **Batch:** solo — verify NeoForge's native mod event path without KubeJS
- **Commit:** P18.3q: prove neoforge player samples without kubejs
