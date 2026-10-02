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

- **Status:** Complete — live movement, rotation, and disconnect verified on
  ATM10 Lite; temporary KubeJS probe removed
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
  terrain fidelity for modded blocks.
- **Evidence:** The ATM10 Lite server log has player samples with 9 distinct
  XYZ positions and 8 distinct yaw/pitch pairs, followed by an empty roster.
  The temporary probe is absent from the stopped server.
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

- **Status:** Complete — live movement, rotation, and disconnect verified on
  Paper; temporary plugin removed
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
- **Evidence:** The Paper server log has player samples with 29 distinct XYZ
  positions and 26 distinct yaw/pitch pairs, followed by an empty roster. The
  temporary plugin is absent from the stopped server.
- **Batch:** solo — establish the Paper live-position path
- **Commit:** P18.3l: prove paper player samples

### P18.3m — Prove live player samples on Fabric

- **Status:** Complete — live movement, rotation, and disconnect verified on
  the selected Fabric server; temporary mod removed
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
- **Evidence:** The Fabric server log has player samples with 82 distinct XYZ
  positions and 76 distinct yaw/pitch pairs, followed by an empty roster. The
  temporary mod is absent from the stopped server.
- **Batch:** solo — prove Fabric player samples before testing the other Java
  server flavors
- **Commit:** P18.3m: prove fabric player samples

### P18.3n — Prove live player samples on Forge

- **Status:** Complete — live movement, rotation, and disconnect verified on
  Forge 66.0.8; temporary mod removed
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
- **Implementation note:** The first probe used the NeoForge-style dependency
  key `type="required"`; Forge 66.0.8 requires `mandatory=true`. The failed
  jar was removed by its recorded checksum and the corrected probe reinstalled.
  The corrected mod emitted live samples.
- **Evidence:** The Forge server log has player samples with 4 distinct XYZ
  positions and 3 distinct yaw/pitch pairs, followed by an empty roster. The
  temporary mod is absent from the stopped server.
- **Batch:** solo — establish Forge player-feed compatibility
- **Commit:** P18.3n: prove forge player samples

### P18.3o — Prove live player samples on Vanilla

- **Status:** Complete — live movement, rotation, and disconnect verified on
  Vanilla 26.3; temporary settings and datapack removed
- **Files:** `tools/world-map-proof/vanilla-player-feed/`,
  `tools/world-map-proof/install_vanilla_player_feed.py`,
  `tools/world-map-proof/remove_vanilla_player_feed.py`,
  `tools/world-map-proof/watch_vanilla_player_feed.py`,
  `tools/world-map-proof/enable_vanilla_rcon_player_feed.py`,
  `tools/world-map-proof/disable_vanilla_rcon_player_feed.py`,
  `tools/world-map-proof/watch_vanilla_rcon_player_feed.py`,
  `tools/world-map-proof/README.md`, `docs/msc2/rolling-plan.md`
- **What:** Use temporary loopback-only RCON on the managed official Minecraft
  26.3 bundle to sample player UUID, position, and rotation across the three
  standard dimensions. Vanilla suppresses the output of commands inside a
  datapack function, so the initial datapack console-feedback route could not
  produce live samples. The RCON helper saves and restores only the original
  `server-ip`, `enable-rcon`, `rcon.port`, and `rcon.password` values; it binds
  the game server and RCON to `127.0.0.1` during the proof. Install and remove
  only while the server is stopped; preserve the world and other settings.
- **Verify:** Follow **Vanilla live player feed proof (P18.3o)** in
  `tools/world-map-proof/README.md`. Stop Vanilla, enable the local-only RCON
  probe, start through MSC, and connect with a 26.3 client from the same Mac.
  Move, turn, and disconnect; confirm fresh positions/look and then an empty
  roster. Stop Vanilla, restore the original server properties, and remove the
  earlier datapack by its recorded checksum.
- **Evidence:** Loopback RCON returned one Overworld player with a stable UUID;
  four consecutive samples showed changing XYZ, yaw, and pitch. After Cameron
  disconnected, the roster became empty. A dimension-filtered query returned
  the player only in the Overworld. Vanilla was stopped through MSC, original
  server properties restored, and the old datapack removed by its checksum.
- **Implementation note:** Minecraft 26.3 requires data pack `min_format` and
  `max_format`; its startup log rejected the first pack's legacy `pack_format`
  metadata. After correcting the metadata, the server listed the pack as
  available; it had to be enabled explicitly. The function then ran, but its
  internal command output did not appear in the console, so the watcher could
  not consume it. The datapack was removed after the RCON proof.
- **Batch:** solo — establish the vanilla position-feed path
- **Commit:** P18.3o: prove vanilla player samples

### P18.3p — Prove live player samples on Purpur

- **Status:** Complete — live movement, rotation, and disconnect verified on
  Purpur 1.21.11; temporary plugin removed
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
- **Evidence:** The server log recorded 32 player samples with one stable UUID,
  10 distinct XYZ positions, and 9 distinct yaw/pitch pairs. The next sample
  after Cameron left had an empty roster. MSC stopped the server, and the
  checksum-checked removal helper removed the proof plugin. A different
  Purpur version still needs its own live check.
- **Batch:** solo — establish Purpur player-feed compatibility
- **Commit:** P18.3p: prove purpur player samples

### P18.3q — Prove live player samples on NeoForge without KubeJS

- **Status:** Complete — live movement, rotation, and disconnect verified on
  Minecraft 26.2 / NeoForge 26.2.0.88 without KubeJS; temporary mod removed
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
- **Evidence:** The server log recorded 23 player samples with one stable UUID,
  17 distinct XYZ positions, and 15 distinct yaw/pitch pairs. The next sample
  after Cameron left had an empty roster. MSC stopped the server, and the
  checksum-checked removal helper removed the proof mod.
- **Batch:** solo — verify NeoForge's native mod event path without KubeJS
- **Commit:** P18.3q: prove neoforge player samples without kubejs

### P18.4 — Prove Java terrain rendering before MSC integration

- **Status:** Representative Java Overworld, Nether, and End rendering accepted
  across the old and new save layouts. Include discovery/rendering of saved
  custom dimensions in the planned map; validate the ATM10 case later when its
  dimension terrain has been generated.
- **What:** Use one read-only Java Anvil terrain pipeline and the existing
  standalone 3D viewer. Test distinct Minecraft data versions, save layouts,
  and block asset sets rather than treating each server flavor as a separate
  renderer. Minecraft 26.1 moved the Overworld region files from the world root into
  `dimensions/minecraft/overworld/`; the proof must discover both layouts.
  Resolve vanilla and available mod/resource-pack block assets without
  modifying a running save. Count and visibly mark missing block models and
  textures rather than silently substituting a plausible block.
- **Verify:** Run full 4×4-chunk exports and visual comparisons on
  representative worlds: Vanilla 26.3 for the new layout, a 26.2 world for
  version coverage, Pupur 1.21.11 for the older layout and version boundary,
  and a genuinely mod-built ATM10 Lite 1.21.1 area for mod models and
  textures. Record Minecraft/data version, layout, chunk coverage,
  exact/fallback model and texture counts, export time, and memory. A nearly
  vanilla ATM10 spawn does not establish modded rendering. Give the other
  selected server flavors a short export smoke check for discovery, coverage,
  and fallback counts; expand to a full visual comparison only if the save
  layout, block palette, or assets introduce a distinct case.
  Prove saved-terrain refresh on at least one 1.21.x and one 26.x world while
  the server remains usable. Check Nether and End separately. Discover and
  render saved custom dimensions when present, recording missing assets or
  unsupported chunk data; validate ATM10 custom terrain when it exists. Accept
  the standalone renderer when representative views are correct and every
  selected server's smoke check has explicit fallbacks and no unexplained
  missing chunks.
- **Order:** Establish the two save layouts with Vanilla 26.3 and Pupur
  1.21.11, cover the 26.2 data version, then exercise modded assets on ATM10
  Lite. Run short checks on Fabric, Forge, NeoForge, and Paper where they were
  not used as the 26.2 representative.
  Choose implementation checkpoints after reviewing each result; this matrix
  is the acceptance gate before the first MSC Worlds-tab integration.
- **Evidence so far (2026-09-30):** A read-only Anvil helper selected 16/16
  chunks in each server and emitted scratch regions. Purpur 1.21.11 uses root
  `region/` (DataVersion 4671); Paper, Fabric, and NeoForge 26.2 use the new
  Overworld layout (4903); Vanilla and Forge 26.3 use the new layout (5023).
  Vantage 0.15.1 misreads 26.3's new `id` palette entries as air; normalizing
  them in the scratch region restored 28,927 non-air blocks in a sampled chunk
  and produced complete Vanilla and Forge exports. None of the server saves
  were modified. The four full tile exports had 16 loaded/0 missing and ran
  in 0.07–0.13 s with 22–35 MB maximum resident set size on this Mac.
  Distinct saved-state audit: Purpur 49 modeled/0 missing textures; Paper 88/0;
  Vanilla 80/0 with 3 states having no JSON geometry; ATM10 Lite 159 modeled
  exactly, 14 states with missing textures, 0 unresolved models. ATM10's tile
  contains 64 Croptopia crop blocks plus mod ores; 13 Xycraft ore states refer
  to an absent `cloudfx` sprite (851 saved blocks), and 2 Lootr chests use a
  checker placeholder for their unsupported entity renderer. Fabric, Forge,
  and NeoForge smoke exports also had 16/16 chunks and no unresolved models or
  textures. Paper Nether and End tiles exported 16/16 chunks separately;
  their visuals were later accepted alongside the Purpur 1.21.11 previews.
  The private reports and tiles are under `/private/tmp/msc-java-terrain-proof`.
  Cameron subsequently accepted the ATM10 tile and the other Java previews.
- **Offline change-selection evidence (2026-09-30):** A Paper 26.2 region
  compared against itself skipped export after checking all 16 chunks (54 ms).
  In an offline scratch copy, changing a grass palette entry in chunk (0, 0)
  to diamond block selected exactly that chunk and exported its 4×4 tile
  (56 ms comparison, 780 ms to ready). This establishes the local comparison
  and export path only. It does not establish a safe live Java snapshot,
  server availability during capture, or visual acceptance. Cameron deferred
  the ATM10 visual review until later.
- **Visual review follow-up (2026-09-30):** Cameron accepted the other Java
  previews but reported sideways tree logs in Vanilla 26.3. The new 26.3
  palette omits default `axis=y`; Vantage picked the first horizontal variant
  when the scratch converter left the state empty. The converter now restores
  omitted defaults for logs and other observed vanilla states. A corrected
  Vanilla tile at `/private/tmp/msc-java-terrain-proof/vanilla-26.3-defaults-fix`
  exported with 16/16 chunks. Cameron's latest response indicates the corrected
  view looks good.
- **Batch:** solo — finish Java and Bedrock rendering evidence before MSC UI
  integration

### P18.5 — Shape the standalone viewer controls before MSC integration

- **Status:** Implemented and accepted in the standalone proof viewer.
- **What:** Use Vantage's dark, translucent map UI as a visual reference. Add a
  compact bottom toolbar for 2D/3D view, creative-style free flight (WASD,
  Space up, Shift down), coordinates, zoom, camera capture, and Home to world
  spawn. Put the player list in a right-side panel where the demo shows biomes.
  Keep player Fly/Follow actions. Defer lighting, quality, and biome controls.
- **Verify:** In the standalone viewer, switch modes without moving the map
  target, fly in all six directions, return to spawn, read accurate cursor or
  camera coordinates, zoom, save a camera image, and use Fly/Follow from the
  player panel. Check desktop and narrow layouts. Home needs the selected
  world's actual spawn coordinates from its save metadata, not an assumed
  origin.
- **Implementation:** Added a dark bottom toolbar, a right-side player list,
  Vantage's existing top-down/tilted/free-flight controls, coordinate display,
  zoom buttons, and PNG capture. Java proof preparation now writes spawn
  metadata. Home uses spawn when it is in the loaded 4×4 tile; otherwise it
  returns to the loaded tile's starting view and explains why. The standalone
  proof viewer still renders only one tile at a time.
- **Follow-up (2026-09-30):** Cameron confirmed the new controls work. His
  underground Vanilla screenshot showed sideways deepslate and sky-colored
  openings. The deepslate was stored in 26.3's compact string palette form,
  which also omitted the default `axis=y`; the converter now handles both
  string and compound forms. A corrected 16/16 tile is at
  `/private/tmp/msc-java-terrain-proof/vanilla-26.3-compact-defaults-fix`.
  The screenshot camera was at Z −71, outside the selected tile's Z range
  −160..−97; the standalone viewer now labels that condition beside the
  coordinates. This proof tile has no neighboring chunks to close its edges.

### P18.6 — Prove live Java saved-terrain refresh

- **Status:** Overworld live snapshot and changed-tile proof accepted on Purpur
  1.21.11 and Vanilla 26.3. Nether and End terrain visuals are accepted in both
  save layouts. Custom-dimension refresh is unproven and will be checked after
  ATM10 generates terrain there.
- **What:** Extend the proof-only `world map-snapshot` path to an MSC-managed
  Java server. Send `save-off`, force `save-all flush`, require its completion
  line from the same server run, copy the world under the existing time/size
  limits, and send `save-on` even when capture fails. The normal backup policy
  remains unchanged. Feed two safe snapshots to Java changed-tile selection.
- **Verify:** On a 1.21.x and a 26.x Java server, capture before and after a
  visible block change, select and export only the affected tile, and confirm
  the viewer updates while the server keeps accepting gameplay. Record the
  save-off duration, bytes copied, comparison time, export time, and any
  timeout. A missing flush acknowledgement is a failed capture, not a usable
  snapshot. Inspect Nether/End and custom dimensions separately before
  promising them in the integrated map.
- **Purpur 1.21.11 live evidence (2026-09-30):** MSC-managed `Pupur` remained
  running across two strict snapshots (13,047,634 and 14,733,720 bytes copied;
  save-off durations 203 and 1,210 ms). Cameron placed a block at X35/Y67/Z36.
  The offline comparison selected chunk (2,2), which contains that coordinate,
  and two neighboring changed chunks. Comparison took 58 ms; the changed tile
  was ready in 928 ms at `/private/tmp/msc-java-live-proof/purpur-change`.
- **Vanilla 26.3 live evidence (2026-09-30):** The same endpoint read the
  `dimensions/minecraft/overworld/region/` layout and kept Vanilla running.
  Two snapshots copied 23,142,809 and 23,357,209 bytes with save-off durations
  of 265 and 783 ms. A block change at X32/Z32 was within selected chunk (2,2).
  That chunk and five others changed; comparison took 217 ms, and the tile was
  ready in 3,254 ms at `/private/tmp/msc-java-live-proof/vanilla-change`.
  Cameron confirmed the new block appears in the viewer and Vanilla continues
  accepting block changes.
- **Dimension sample preparation (2026-09-30):** Purpur already had saved
  Nether and End chunks, so no in-game travel was needed. I assembled the
  best fully populated 4×4 tile across region boundaries in scratch copies;
  both exports have 16/16 chunks and use the 1.21.11 assets. They are at
  `/private/tmp/msc-java-terrain-proof/pupur-1.21.11-the_nether-full` and
  `/private/tmp/msc-java-terrain-proof/pupur-1.21.11-the_end-full`.
  Cameron accepted both Purpur 1.21.11 dimension previews and both existing
  Paper 26.2 dimension previews. ATM10 has five custom dimension directories
  (`ae2:spatial_storage`, `allthemodium:mining`, `allthemodium:the_beyond`,
  `allthemodium:the_other`, plus `irons_spellbooks:pocket_dimension`); none
  contains saved `.mca` region chunks yet. A custom-dimension rendering claim
  needs one of those dimensions to be generated and saved first.

### P18.8 — Integrate the map resource API with the Worlds tab

- **Status:** Awaiting Cameron verification
- **Files:** `crates/msc-agent/src/`, `crates/msc-api/src/dto/worlds.rs`, `docs/msc2/api-contract/`, `clients/desktop-web/src/lib/api/generated.ts`, `docs/msc2/rolling-plan.md`
- **What:** Add the authenticated `GET /v1/worlds/map/dimensions` Worlds
  endpoint and CLI inspection command for the selected active server's
  configured world. Return path-free dimension IDs, display names, region-file
  counts, and availability explanations. Java discovery recognizes the
  standard dimensions in both legacy and current save layouts, and lists
  namespaced custom dimension folders even when they have no saved region
  chunks. The current Bedrock filesystem catalog reports `not_indexed`, since
  Bedrock chunks live in LevelDB records rather than region files. This step
  establishes the authenticated discovery contract only; consistent snapshots
  and tile/texture transport are part of P18.9's embedded map bridge. The
  generated TypeScript contract was regenerated too, which also syncs three
  existing OpenAPI routes missing from the previously generated file.
- **Implementation:** The route requires Worlds permission, reads only the
  active server's configured level directory, rejects unsafe level names and
  symlinked world roots, bounds custom-dimension and region-directory scans,
  and never returns host paths. Java dimension availability is based on `.mca`
  region files; Bedrock availability is explicit rather than inferred from
  directory presence. No chunk bytes or player samples are exposed here.
- **Verify:** Build with `cargo build -p msc-agent --bin msc`, then run
  `MSC2_DATA_DIR="$HOME/Library/Application Support/MSC 2" target/debug/msc --json world map-dimensions`
  against a configured Java world and a Bedrock world. Confirm the Java
  standard dimensions and saved or empty namespaced dimensions, and confirm
  Bedrock dimensions say `not_indexed`; responses must contain no filesystem
  paths. This is Cameron's authenticated contract verification. Snapshot and
  tile reads remain for P18.9.
- **Commit:** P18.8: add authenticated world dimension catalog
- **Batch:** A (P18.8) — authenticated map resource boundary

### P18.9a — Bundle the Vantage Java terrain renderer

- **Status:** Awaiting Cameron verification — owner approved bundling Vantage on 2026-10-01
- **Files:** `tools/release/stage-vantage.py`, `clients/desktop-web/tools/prepare-agent-dev.mjs`, `clients/desktop-web/src-tauri/target/package/agent/`, `tools/release/build-*-headless.*`, `packaging/`, `docs/msc2/clients/headless-installation.md`, `THIRD-PARTY-NOTICES.md`, `docs/msc2/rolling-plan.md`
- **What:** Pin Vantage 0.15.1 platform executables by upstream release URL and archive SHA-256. Stage the matching macOS, Windows, or Linux binary into desktop resource bundles and headless archives; install it beside the agent; remove it with the MSC-owned agent installation. Document the MIT notice and renderer paths. Do not bundle Minecraft data or assets. This is the renderer-delivery prerequisite; P18.9b will add the authenticated agent bridge before P18.9 embeds it in Worlds.
- **Verify:** On Intel macOS, run `python3 tools/release/stage-vantage.py --platform macos-x86_64 --output-dir /private/tmp/msc-vantage-check`, then `/private/tmp/msc-vantage-check/vantage --version`; on Apple Silicon, use `macos-aarch64` instead. Confirm Vantage 0.15.1. Run `node --check clients/desktop-web/tools/prepare-agent-dev.mjs`, `bash -n tools/release/build-linux-headless.sh tools/release/build-macos-headless.sh packaging/linux/install.sh packaging/linux/uninstall.sh packaging/macos/install.sh packaging/macos/uninstall.sh`, parse the changed PowerShell scripts with `pwsh`, and run `git diff --check`. This confirms the renderer can be staged for packaging and establishes the next necessary P18.9b bridge step; it does not yet deliver a visible Worlds map.
- **Commit:** P18.9a: bundle Vantage terrain renderer
- **Batch:** B (P18.9) — Java map renderer delivery

### P18.9b — Add authenticated Vantage terrain bridge

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/worlds/map_terrain.rs`, `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, `docs/msc2/rolling-plan.md`
- **What:** The Worlds-authorized `GET /v1/worlds/map/terrain` route launches one private Vantage renderer for the active Java world and a cataloged dimension. It proxies only bounded manifest, texture, and tile artifacts, validates the world root and artifact path, and retires the renderer after idle time or dimension changes. It returns explicit unavailable results for unsupported Bedrock and empty dimensions. Java client assets must be installed on the host for Vantage's default asset discovery.
- **Verify:** Start the updated agent with a selected Java server that has saved terrain and the bundled Vantage executable. Request `GET /v1/worlds/map/terrain?dimension=minecraft%3Aoverworld&path=manifest.json` using a Worlds-authorized credential and confirm a manifest. Fetch a tile named by that manifest and `terrain.vtexarr`; an unauthenticated request must fail, and `path=../level.dat` must return 400. Confirm the responses contain no host paths. This is the secure renderer boundary for P18.9's in-window map, not the UI or live-refresh proof.
- **Commit:** P18.9b: add authenticated Vantage terrain bridge
- **Batch:** B (P18.9) — Java map renderer bridge

### P18.9 — Embed saved terrain and dimension selection in Worlds

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `docs/msc2/rolling-plan.md`
- **What:** The selected active world opens an in-window Vantage map through the Worlds-authorized terrain bridge. Dimension choices include saved Java dimensions and clearly mark empty ones. The viewer has 2D/3D, free flight, XYZ, zoom, Home to spawn, and screenshot controls. The right-side player panel names its pending live-feed capability instead of inventing players; Bedrock names its proof-only status. Exiting restores the world list and its selection. The Java view uses Vantage's installed-host assets and native visible fallback behavior.
- **Verify:** Restart `npx tauri dev` and repair/restart the MSC agent so it loads the P18.9b route and bundled Vantage binary. In Worlds, select the active Java slot and choose View Map. Open Overworld, Nether, and End where saved; check an empty dimension's explanation, 2D/3D, Fly with WASD/Space/Shift, XYZ, zoom, Home, screenshot, and return to the same selected slot. Open an active Bedrock slot and confirm it explains that in-app terrain is pending. This advances in-window saved Java terrain; Bedrock geometry and live players remain P18.10 integration work.
- **Commit:** P18.9: embed saved terrain in Worlds
- **Batch:** B (P18.9) — in-window saved terrain

### P18.9c — Stage the terrain renderer with desktop agent repairs

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `clients/desktop-web/src-tauri/src/lib.rs`, `docs/msc2/rolling-plan.md`
- **What:** Desktop Agent repair now copies the bundled Vantage executable beside `msc` in the versioned agent build directory. The build digest includes both executables, so replacing either creates a new staged build. A missing renderer in the desktop package produces a specific repair error.
- **Verify:** Restart `npx tauri dev`, use MSC's Agent repair/restart control, and reopen the Vanilla Overworld map in Worlds. The saved terrain should appear instead of the renderer unavailable message. The running server does not need to be stopped.
- **Commit:** P18.9c: stage terrain renderer with desktop agent
- **Batch:** B (P18.9) — desktop renderer handoff

### P18.9d — Render Java 26.x saved dimensions in MSC

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `crates/msc-agent/src/routes/worlds/map_terrain.rs`, `crates/msc-agent/src/routes/worlds/map_terrain/java_terrain_compat.rs`, `docs/msc2/rolling-plan.md`
- **What:** Vantage 0.15.1 expects the old Java save layout and old block palette names. For saved Overworld, Nether, and End regions in the 26.x layout, the agent now prepares a private, bounded compatibility copy with legacy region paths and palette entries before starting Vantage. The server's world files are read only. The renderer copy is static for its session; consistent live refresh remains P18.10.
- **Verify:** Stop Vanilla in MSC, restart `npx tauri dev`, repair/restart the agent, then start Vanilla again and open Worlds → Vanilla → View Map. The saved Overworld terrain should render instead of a sky-only view. Inspect a tree log and deepslate face for the 26.3 default orientation. The server should remain writable.
- **Commit:** P18.9d: render Java 26.x dimensions in MSC
- **Batch:** B (P18.9) — Java save compatibility

### P18.9e — Make desktop Fly camera controls usable

- **Status:** Needs correction — Cameron observed a steep camera jump on desktop mouse movement, 2026-10-01
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `clients/desktop-web/src-tauri/capabilities/default.json`, `docs/msc2/rolling-plan.md`
- **What:** Enter Fly with a near-level pitch instead of carrying the steep map angle into first-person movement. In the macOS desktop webview, use Tauri window cursor control when browser pointer lock is unavailable, so a click on the map enables continuous mouse look; Escape, window blur, leaving Fly, and closing the map restore the cursor. Browser pointer lock remains the viewer's normal path.
- **Verify:** Restart `npx tauri dev` and open a saved Java map in MSC. Select Fly: the view should start nearly level. Click the terrain, move the mouse beyond the former window edge, and confirm the camera keeps turning while WASD/Space/Shift still move. Press Escape and confirm the pointer returns. Repeat after switching 2D/3D and closing the map. Check the browser proof viewer still uses its usual pointer lock.
- **Commit:** P18.9e: fix desktop fly camera and mouse look
- **Batch:** B (P18.9) — desktop map controls

### P18.9f — Correct desktop mouse-look deltas

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `docs/msc2/rolling-plan.md`
- **What:** The initial desktop fallback treated the cursor's absolute distance from the map center as movement on every event. That fed the repositioning jump into pitch and could point the camera straight at the sky. Use consecutive real pointer positions as deltas, ignore repositioning events, and recenter only near the map edge. Show a visible mouse-look indicator while captured. Keep the Fly entry pitch near level.
- **Verify:** Restart `npx tauri dev`, open a saved Java map, and select Fly. Confirm the initial view is near level. Click terrain: the mouse-look indicator appears. Turn slowly in all directions and continue past the map edge; the camera should turn smoothly without jumping to the sky. Press Escape and confirm the indicator clears and ordinary drag works again.
- **Commit:** P18.9f: correct desktop mouse look deltas
- **Batch:** B (P18.9) — desktop map controls

### P18.9g — Align MSC Fly heading with the browser viewer

- **Status:** Needs correction — Cameron's movement screenshots showed W still drifting sideways, 2026-10-01
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Remove MSC's extra downward Fly entry pitch. Start the desktop camera level so its forward heading and the direction in the center of the view agree, as they do in the browser proof viewer.
- **Verify:** In the running MSC app, reopen a saved map, select Fly, and check that the horizon starts level. Face a distant landmark, press W, and confirm it stays centered while getting closer. Turn with mouse look and repeat. The app and browser proof should now start Fly with the same level heading.
- **Commit:** P18.9g: align MSC Fly heading with browser viewer
- **Batch:** B (P18.9) — desktop map controls

### P18.9h — Correct desktop cursor coordinates for Fly look

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `clients/desktop-web/src-tauri/capabilities/default.json`, `docs/msc2/rolling-plan.md`
- **What:** The desktop mouse fallback centered the cursor using WebView coordinates even though Tauri expects decorated-window coordinates. Add the native content inset before cursor warps and ignore the synthetic movement while the warp completes. Grant the three window position/scale reads needed for the conversion. Revert the attempted camera-vector W movement, which made Y descend and did not resolve lateral drift.
- **Verify:** Restart the desktop app so the capability change loads. In MSC Fly mode, click to capture the pointer and aim at a distinctive tree or block near the screen center. Press W repeatedly; the target should grow without the view drifting diagonally. Turn with the mouse, repeat, and check that Space/Shift are the only keys changing Y. Compare the level view with the browser proof.
- **Commit:** P18.9h: correct desktop Fly cursor coordinates
- **Batch:** B (P18.9) — desktop map controls

### P18.9i — Enter Fly near terrain from a streamed-world overview

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `docs/msc2/rolling-plan.md`
- **What:** The browser proof frames a small tile near terrain, while MSC opens a streamed world from a high overview. Entering Fly preserved that distant overview eye, producing the steep aerial angle Cameron showed (MSC Y 386 versus proof Y 107). When the eye is more than 32 blocks above terrain at the current map focus, place it two blocks above that terrain and start with a level pitch. Keep close views where they are.
- **Verify:** Open a saved Java world in MSC, select Fly from the initial overview, and confirm the camera starts near ground instead of hundreds of blocks above it. Face a block near the center, move with W, then turn with mouse look and use Space/Shift. Check that switching to Fly from an already close map view stays nearby.
- **Commit:** P18.9i: start streamed-world Fly near terrain
- **Batch:** B (P18.9) — desktop map controls

### P18.9j — Correct macOS Fly pointer recentering

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `clients/desktop-web/src-tauri/capabilities/default.json`, `docs/msc2/rolling-plan.md`
- **What:** Tauri's macOS cursor API measures from the content area, so the previous Fly fallback added the title-bar inset a second time. Center the cursor in content coordinates and discard the first movement event after a warp before resuming relative mouse look. Remove the unused window geometry permissions. Tile geometry and Fly movement both already use the same world X/Z axes.
- **Verify:** Restart the desktop app, open a Java map, enter Fly, and click to capture the pointer. Aim along a straight block edge, then press W with the mouse still: movement should follow the view without lateral turning. Turn the camera and repeat. Confirm Space and Shift change only height.
- **Commit:** P18.9j: correct macOS Fly pointer recentering
- **Batch:** B (P18.9) — desktop map controls

**Open camera observation (2026-10-01):** Cameron remains unhappy with Fly camera behavior. He also saw right-facing block sides while 2D was selected; P18.10a changes the 2D button to set exact top-down pitch immediately, pending his visual check.

### P18.9k — Size the embedded map canvas to its viewport

- **Status:** Done — Cameron confirmed the camera is fixed, 2026-10-01
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Apply the P18.3i Retina canvas fix to MSC's embedded viewer. Vantage sizes its drawing buffer using device pixel ratio but leaves the canvas CSS size unchanged. The resulting oversized canvas is clipped by the map panel, making the camera's true center appear near the lower-right corner even while the toolbar reports the correct world focus. Keep the canvas at the panel's CSS width and height so the full camera image is visible.
- **Verify:** Reload MSC's world map on the Retina display, select 2D, and center the view on the diamond blocks at X 33, Z 30. Zoom out without panning; the blocks should remain at the viewport center. In Fly mode, aim at a block edge and press W to confirm that travel follows the visible view. Check the browser proof viewer for comparison.
- **Commit:** P18.9k: size embedded map canvas to viewport
- **Batch:** B (P18.9) — desktop map controls

### P18.10a — Refresh embedded Java terrain from a consistent save

- **Status:** Done — Cameron confirmed refresh and stopped-server behavior, 2026-10-01
- **Files:** `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/worlds/map_terrain.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `clients/desktop-web/src/lib/api/generated.ts`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/rolling-plan.md`
- **What:** Add a Worlds-authorized Refresh terrain action for the running active Java server. Reuse the bounded save-off/flush/copy/save-on snapshot operation, retain its private copy as the renderer source, retire the previous renderer, and reload the selected dimension after the operation succeeds. Report copy bytes and save hold duration without exposing the snapshot path. Set the 2D camera pitch and heading immediately to a true top-down view instead of marking an in-progress tilt as 2D. This first refresh reloads the dimension; changed-tile selection and live player feeds remain later P18.10 work.
- **Verify:** Restart MSC and its agent, open a running Java world map, place a distinctive block in game, and choose Refresh terrain. Confirm the new block appears, the server still accepts block changes, and another Refresh updates again. Check that an attempted refresh while the server is stopped gives a clear error. Select 2D and confirm block side faces disappear; switch to 3D and back.
- **Commit:** P18.10a: refresh embedded Java terrain from a consistent save
- **Batch:** C (P18.10) — live refresh and players

**P18.10a field verification (2026-10-01):** Cameron confirmed a second Refresh terrain updated the map, the Vanilla server continued accepting block changes, and trying refresh with Vanilla stopped showed an error. This verifies the embedded Java refresh path on Vanilla; other runtime coverage remains open.

### P18.10b — Show current players in the embedded map

- **Status:** Done — Cameron confirmed end-to-end behavior, 2026-10-01
- **Files:** `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/src/routes/lifecycle/map_player_query.rs`, `crates/msc-agent/src/routes/worlds.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `clients/desktop-web/src/lib/api/generated.ts`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/rolling-plan.md`
- **What:** Expose a Worlds-authorized, server-scoped live player endpoint. Reuse the already ingested BDS behavior-pack samples and accept structured Java samples where a server feed exists. When no Java feed is present, request current Position, Rotation, and Dimension through the managed server console while the map is open, then publish a complete sample only after the matching online roster arrives. Expire samples after five seconds and discard partial replies. Poll from the embedded viewer, draw current-dimension models, list connected players, and provide Fly and Follow actions. A followed player changing to a saved dimension switches maps; disconnect or stale feed releases Follow. The query fallback uses the player's current name as the model identity within that server run; it does not claim a UUID.
- **Verify:** Restart the development agent and MSC, open Vanilla's Overworld map, and join the server. The Players panel should show the connected player and a model at the live coordinates. Walk and turn; both should update. Fly to and Follow the player, then disconnect; the panel/model should clear within about five seconds. Switch dimensions with a player if saved terrain exists and confirm Follow moves to that dimension. On BDS, the roster should show fresh behavior-pack samples; Bedrock terrain inside MSC remains a separate integration step.
- **Commit:** P18.10b: connect live players to embedded map
- **Batch:** C (P18.10) — live refresh and players

**P18.10b field verification (2026-10-01):** Cameron confirmed the roster and model update as described, Fly and Follow work, and disconnect clears the player.

### P18.10c — Verify the embedded map on Purpur 1.21.11

- **Status:** Deferred to consolidated runtime verification — Cameron prefers not to test each runtime separately, 2026-10-01
- **Files:** `docs/msc2/rolling-plan.md`
- **What:** Exercise the embedded refresh and Java console player-query paths on the already-proven Purpur 1.21.11 world. This covers the legacy root `region/` layout and a pre-26.x server version in MSC, beyond Vanilla 26.3. Check a visible terrain update and live roster/model movement while Purpur remains usable. This is a runtime-compatibility smoke check; it does not repeat full offline asset auditing or claim modded block-model fidelity.
- **Verify:** In MSC, stop Vanilla if it is still running, select and start the existing Purpur 1.21.11 server, open its Overworld map, join using Minecraft 1.21.11, and confirm the player appears and moves in the roster/model. Place a distinctive block, refresh terrain, confirm the block appears and gameplay still works, then disconnect and confirm the marker clears. If saved Nether or End terrain is present, check that the map opens that dimension. This advances the visible promise that the supported legacy Java save layout refreshes and tracks live players inside MSC; report any missing chunks, stale player state, or renderer/refresh error as a blocker before expanding to other Java flavors.
- **Batch:** C (P18.10) — Java runtime compatibility

### P18.10d — Deliver the Bedrock terrain exporter with MSC

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `tools/world-map-proof/src/main.rs`, `clients/desktop-web/tools/prepare-agent-dev.mjs`, `clients/desktop-web/src-tauri/src/lib.rs`, `packaging/agent-service-layout.json`, `packaging/{linux,macos,windows}/`, `tools/release/build-{linux,macos,windows}-headless.*`, `docs/msc2/rolling-plan.md`
- **What:** Build the proven Bedrock tile exporter as `bedrock-map` alongside the desktop agent and in headless archives. Agent repair copies it into the versioned agent directory, and the build digest changes when the exporter changes. Headless installers install and remove the same binary. The exporter identifies its version with `--version`. Bedrock resource-pack textures remain a separate asset-discovery prerequisite; Minecraft image assets are not bundled here. This step delivers the executable but does not yet expose Bedrock terrain in the Worlds map.
- **Verify:** On the development Mac, run the staged `bedrock-map --version` executable and confirm its version. Review the staged desktop and headless package paths. Keep Bedrock map runtime checks in the consolidated batch rather than asking Cameron to repeat them by server flavor.
- **Commit:** P18.10d: package Bedrock terrain exporter with agent
- **Batch:** C (P18.10) — Bedrock map integration

### P18.10e — Open a saved Bedrock Overworld tile inside MSC

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `tools/world-map-proof/src/main.rs`, `crates/msc-agent/Cargo.toml`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/worlds/map_terrain.rs`, `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/rolling-plan.md`
- **What:** The Worlds-authorized terrain route exports one saved 4×4-chunk Bedrock Overworld area using the packaged exporter and returns only its bounded tile, texture, and spawn metadata artifacts. A running BDS world is copied behind `save hold`/`save resume` before export; a stopped world is read from its saved files. The embedded viewer loads that single tile with the same map controls and live player layer. Home reaches spawn when it lies in that tile. MSC obtains Mojang's pinned Bedrock sample textures on first use, verifies the archive SHA-256, extracts only texture files into private app data, and permits a local `MSC2_BEDROCK_RESOURCE_PACK` override. No Minecraft images enter the MSC package or repository. Nether/End terrain, wider tile coverage, and embedded Bedrock terrain refresh remain open; this step does not claim them.
- **Verify:** During the consolidated Bedrock runtime check, run `npx tauri dev`, repair/restart the agent, then open an existing BDS world in Worlds. The first Overworld load may download textures; confirm one textured saved area appears, 2D/3D/Fly controls operate, and the running server still accepts changes. A missing network or mismatched asset archive must produce a clear map error, not a fabricated tile. Defer this physical check until Cameron's chosen batch.
- **Commit:** P18.10e: open saved Bedrock terrain in MSC
- **Batch:** C (P18.10) — Bedrock map integration

### P18.10f — Show neighboring Bedrock terrain tiles inside MSC

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `tools/world-map-proof/src/main.rs`, `tools/world-map-proof/src/render.rs`, `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/rolling-plan.md`
- **What:** Export up to nine populated 4×4-chunk tiles in a 3×3 grid around Bedrock world spawn from one saved BDS snapshot. The tiles use one shared texture array, and the agent serves a bounded manifest and only its numbered tile paths. The MSC viewer loads that manifest through Vantage's tiled world source, so panning across neighboring tiles no longer stops at the original 4×4 area. This remains a bounded saved-area view, not full-world paging; refresh and Nether/End terrain remain separate steps.
- **Verify:** In the consolidated Bedrock runtime check, open a BDS Overworld with saved chunks around spawn in MSC, pan or Fly across a former 4×4 tile edge, and confirm adjacent saved terrain loads with consistent textures and coordinates. The first export can take longer because it prepares several tiles. Check that a request for an unlisted tile returns a clear error and BDS remains writable. Do not run this as another per-flavor check now.
- **Commit:** P18.10f: show neighboring Bedrock tiles in MSC
- **Batch:** C (P18.10) — Bedrock map integration

### P18.10g — Refresh saved Bedrock terrain inside MSC

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/worlds/map_terrain.rs`, `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/rolling-plan.md`
- **What:** Enable Refresh terrain for a running BDS world. The operation takes a consistent save hold/resume snapshot, records bytes and hold time, and selects that snapshot for the next Bedrock tile export. The map reloads after the operation succeeds; BDS resumes writes before success is reported. The cached render and prior unused snapshot are retired. Rendering still rebuilds the bounded tile grid in full; dirty-tile selection is not yet used by the embedded map.
- **Verify:** In the consolidated Bedrock runtime check, change a block in a loaded tile, use Refresh terrain, confirm the block appears and BDS still accepts edits. Stop BDS and confirm Refresh terrain gives a clear stopped-server error.
- **Commit:** P18.10g: refresh saved Bedrock terrain in MSC
- **Batch:** C (P18.10) — Bedrock map integration

### P18.10h — Page through all saved Bedrock Overworld tiles

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `tools/world-map-proof/src/main.rs`, `tools/world-map-proof/src/render.rs`, `crates/msc-agent/src/backup_operations.rs`, `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/rolling-plan.md`
- **What:** Catalog every saved Bedrock Overworld 4×4-chunk tile from one consistent world copy. Vantage requests tile paths as the camera moves; the agent renders only requested tiles and uses an append-only shared texture array so earlier tiles keep the same texture indices. A stopped world is also copied first, ensuring later tile requests do not read a LevelDB save after BDS starts. Refresh replaces the snapshot and catalog. The existing 2 GiB/30-second world-copy limit and per-artifact size limit still apply; Nether/End terrain and low-detail whole-world previews remain separate.
- **Verify:** In the consolidated Bedrock runtime check, open a saved BDS Overworld, move farther than the former 3×3 tile boundary, and confirm newly reached terrain loads without a fixed edge or mismatched textures. Revisit an earlier tile, then edit a block and refresh; confirm the edit appears and BDS remains writable. A large world above the existing snapshot limit should report the copy error clearly.
- **Commit:** P18.10h: page saved Bedrock tiles across the world
- **Batch:** C (P18.10) — Bedrock map integration

### P18.10i — Show saved Bedrock Nether and End terrain

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `tools/world-map-proof/src/main.rs`, `tools/world-map-proof/src/render.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/rolling-plan.md`
- **What:** Add dimension-specific saved chunk catalogs and on-demand tiles for the Bedrock Nether and End. MSC shows the same dimension selector used by Java. A consistent saved world copy is shared across dimension catalogs, while each dimension keeps separate tiles and textures. Nether and End get their own atmosphere and do not use the Overworld spawn. A dimension with no generated chunks reports an export error when selected.
- **Verify:** In the consolidated Bedrock runtime check, generate and save Nether and End chunks, select each dimension in MSC, and confirm its terrain, atmosphere, tile paging, and dimension-specific coordinates. Refresh terrain after changing a block in each dimension and confirm BDS still accepts edits. Also select a dimension with no generated chunks and confirm the error is clear. No separate runtime check is requested now.
- **Commit:** P18.10i: show saved Bedrock Nether and End terrain
- **Batch:** C (P18.10) — Bedrock map integration

### P18.10j — Reuse unchanged Bedrock tiles on refresh

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `tools/world-map-proof/src/main.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/rolling-plan.md`
- **What:** After a consistent BDS snapshot, catalog each cached dimension and compare only its previously rendered 4×4 tiles against the new copy. Keep unchanged tile files and their shared texture index; leave changed or newly generated tiles for on-demand rendering. Retire removed tiles. Preserve the prior snapshot and cache if catalog or comparison fails. Refresh operation details record reused, changed, and removed tile counts. The save copy itself remains a full bounded snapshot.
- **Verify:** In the consolidated Bedrock runtime check, open two distant saved tiles, change a block in one, and refresh. Confirm the changed tile updates, the unchanged tile still displays, both dimensions remain selectable, and BDS accepts new edits. Check the refresh operation details for reused and changed counts. Repeat without edits and confirm already rendered tiles are reused.
- **Commit:** P18.10j: reuse unchanged Bedrock tiles on refresh
- **Batch:** C (P18.10) — Bedrock map integration

### P18.10k — Keep the Bedrock initial view on terrain

- **Status:** Awaiting Cameron verification — implemented 2026-10-01
- **Files:** `tools/world-map-proof/src/main.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Ignore Bedrock's out-of-range Y sentinel instead of using it as a real camera spawn. Keep the Bedrock terrain status explicit while the first on-demand tiles render, then show the number of tiles ready. This addresses the high initial camera and blank-looking load while large Bedrock saves render.
- **Verify:** Open a Bedrock world with saved terrain in MSC. Confirm the camera starts over terrain when the level.dat spawn Y is 32767, the status says tiles are rendering, and then reports tiles ready as terrain appears. Check an empty dimension reports no saved terrain.
- **Commit:** P18.10k: keep Bedrock initial view on terrain
- **Batch:** C (P18.10) — Bedrock map integration

### P18.10 — Connect saved-terrain refresh and live player controls

- **Status:** Planned — owner direction, 2026-09-30
- **Files:** `crates/msc-agent/src/`, `clients/desktop-web/src/`, `docs/msc2/rolling-plan.md`
- **What:** Replace proof-only refresh and player-feed calls with the Worlds
  capability. Refresh the selected dimension from a consistent snapshot while
  keeping the game writable, and update only affected tiles when possible.
  Connect the proven Java and BDS live player feeds to the map's roster and
  models, keeping terrain-save time distinct from player-sample time. Keep
  Fly and Follow dimension-aware, stop following on stale or missing samples,
  and never label stored player coordinates as live.
- **Verify:** Cameron makes one visible terrain change on a Java server and
  BDS server and confirms it appears after refresh while both servers continue
  accepting gameplay. Walk, turn, change dimension, and disconnect a player on
  each runtime; confirm the roster/model follows current samples, dimension
  transitions fly to the correct terrain, and disconnect clears or marks the
  player stale. This advances fresh terrain and live-player promises together;
  record snapshot hold, bytes copied, refresh latency, and player sample age.
- **Batch:** C (P18.10) — live refresh and players

### P18.11 — Close the integrated map acceptance record

- **Status:** Planned — owner direction, 2026-09-30
- **Files:** `docs/msc2/rolling-plan.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/msc2-port-plan.md`
- **What:** Review the integrated map against the selected runtime matrix and
  the Worlds-tab UX promise. Record Java and BDS terrain coverage, supported
  dimensions, capability gaps, model/texture fallbacks, freshness, refresh
  cost, and resource bounds. Keep ATM10 custom-dimension discovery/rendering
  in the supported design; defer only its visual proof until that world has
  saved chunks in a custom dimension. State clearly that a dimension with no
  saved terrain is discoverable but cannot yet display terrain. Update the
  proposed Phase 18 exit gate from observed results without claiming universal
  mod compatibility or an unmeasured resource target.
- **Verify:** Cameron reviews the complete evidence record and confirms the
  selected Java/BDS worlds open inside MSC, the controls and clean exit work,
  terrain updates while servers remain usable, and live roster/Fly/Follow work
  with fresh samples. Confirm the record names unsupported states and empty
  custom dimensions explicitly. This advances the final integrated Worlds-map
  promise; a remaining gap stays visible as a named limitation with a next
  proof checkpoint.
- **Batch:** D (P18.11) — acceptance record and gate proposal
