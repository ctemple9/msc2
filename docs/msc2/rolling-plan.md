# MSC 2 — Rolling Plan

> ## STATUS: Phase 16 step records P16.1–P16.34 are Done at Cameron's direction and archived. [v0.1.18](https://github.com/ctemple9/msc2/releases/tag/v0.1.18) published all nine artifacts from `ededaf33632bbbdcc518ae8928a54bb3ba073cc6`. The Phase 16 exit gate remains open for Cameron's physical results and an independent review.
> **Next move:** Cameron reviews the proposed Phase 17 CLI plan below. Phase 16 still needs Cameron's exact-artifact results in `docs/msc2/release/phase16-acceptance.md` and an independent gate review. Done step statuses do not assert that pending gate evidence exists.

## How this document works

The vision documents describe where MSC 2 is going; the port plan defines the phase sequence and exit gates; this file records the active priorities and current phase state. Completed phase plans and historical step records live in `rolling-plan-archive.md`.

Each implementation step is planned, read, executed, verified by Cameron, reviewed against its phase gate, and then archived. A step's status here records whether Cameron has completed his verification; moving a step to the archive does not mean that verification is complete.

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
| 17 | Local CLI refinement | planned; owner review pending |

## Active Phase 16 acceptance

The [v0.1.18 release](https://github.com/ctemple9/msc2/releases/tag/v0.1.18)
completed its build and publish workflow with all nine platform artifacts.
Cameron's exact-artifact physical observations and the independent Phase 16
review remain outstanding. See `docs/msc2/release/phase16-acceptance.md` for
the release identity, published asset metadata, and pending acceptance rows.

P16.1–P16.34 and the September 17 and September 28 audits are preserved in
`docs/msc2/rolling-plan-archive.md`. Phase 16 remains in progress until its
exit gate holds.

## Proposed Phase 17 — Local CLI refinement

**Scope for Cameron's review.** After installing either the desktop app or a
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

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `docs/msc2/msc2-decisions.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/msc2-product.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/clients/phase17-cli.md`
- **What:** Record Cameron's local-or-SSH-host-only CLI rule, invisible local authentication, service and boot semantics, desktop pairing separation, supported catalog boundary, no new Minecraft commands, and the Phase 17 gate. Reconcile old direct-remote CLI claims without changing desktop multi-host support.
- **Verify:** `rg -n 'Phase 17|SSH|boot|pairing|CurseForge' docs/msc2/msc2-port-plan.md docs/msc2/clients/phase17-cli.md`
- **Batch:** A (P17.1–P17.2) — contract and inventory

### P17.2 — Map every API route to a CLI task or explicit exclusion

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `docs/msc2/clients/phase17-cli.md`, `docs/msc2/api-contract/openapi.json`
- **What:** Compare the live router and OpenAPI with the CLI. Record every user task, existing command, missing command, permission, edition limit, active-server context, and output rule. Mark staging, stream tickets, and desktop bootstrap as internal. Identify any missing API capability before implementing commands.
- **Verify:** `rg -n 'servers|players|worlds|backups|catalog|operations|help|internal' docs/msc2/clients/phase17-cli.md`
- **Batch:** A (P17.1–P17.2) — contract and inventory

### P17.3 — Define host-local CLI authentication

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `crates/msc-agent/src/auth.rs`, `crates/msc-agent/src/auth/local_cli.rs`, `docs/msc2/clients/phase17-cli.md`, `docs/msc2/rolling-plan.md`
- **What:** Define the shared, compile-safe local-authentication interface and in-memory short-lived API credential policy. Document the trusted OS peer and service-account inputs, authorization, route permissions, audit identity, failure behavior, and restart behavior. P17.4–P17.6 add the actual platform IPC listeners and peer verification; P17.7 connects the CLI and removes its existing remote/token options. No local exchange is available to operators in this step.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** B (P17.3–P17.7) — local authentication

### P17.4 — Implement Linux local authentication

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `crates/msc-agent/src/auth/`, `crates/msc-agent/src/main.rs`, `crates/msc-platform-linux/src/`, `docs/msc2/clients/phase17-cli.md`, `docs/msc2/rolling-plan.md`
- **What:** Use a Unix socket with peer credentials and installation-user ownership. Authorize the same account at the keyboard or through SSH after agent and host restarts, while preserving the privileged credential helper boundary.
- **Verify:** `cargo check -p msc-agent -p msc-platform-linux`
- **Batch:** B (P17.3–P17.7) — local authentication

### P17.5 — Implement macOS local authentication

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `crates/msc-agent/src/auth/`, `crates/msc-platform-macos/src/`, `packaging/macos/`
- **What:** Verify the local or SSH-shell OS peer identity without a GUI login. Keep CLI access distinct from the desktop's signed-package bootstrap key and preserve service-account and macOS consent boundaries.
- **Verify:** `cargo check -p msc-agent -p msc-platform-macos --target x86_64-apple-darwin`
- **Batch:** B (P17.3–P17.7) — local authentication

### P17.6 — Implement Windows local authentication

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `crates/msc-agent/src/auth/`, `crates/msc-platform-windows/src/`, `packaging/windows/`
- **What:** Use a local named pipe with an explicit access-control list and verified caller identity. Permit the installing account after reboot or remote Windows login, reject unrelated users, and preserve the Service Control Manager privilege boundary.
- **Verify:** `cargo check -p msc-agent -p msc-platform-windows --target x86_64-pc-windows-msvc`
- **Batch:** B (P17.3–P17.7) — local authentication

### P17.7 — Make CLI API calls local and automatic

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/transport.rs`, `crates/msc-agent/src/cli/session.rs`, `crates/msc-agent/src/main.rs`
- **What:** Obtain local authorization on each invocation and use the existing authenticated API without visible tokens. Remove public direct-remote host/URL/port/token options and `token print`; explain the migration. Report a stopped or uninstalled agent and unauthorized OS account clearly. Preserve JSON scripting and route permission checks.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** B (P17.3–P17.7) — local authentication

### P17.8 — Add plain local agent-service commands

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Commit:** P17.8: add local agent service commands
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/service.rs`, `crates/msc-platform-linux/src/service.rs`, `crates/msc-platform-macos/src/service.rs`, `crates/msc-platform-windows/src/service.rs`
- **What:** Add `msc start agent`, `msc stop agent`, and `msc status agent` around the installed local service, without an API credential or internal service-name flags. Show installed, running, stopped, and boot-enabled states. Keep `msc server start/stop` distinct and require only OS-level privilege where needed.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** C (P17.8–P17.10) — installation and service

### P17.9 — Put the CLI on PATH for both installation types

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Commit:** P17.9: register desktop cli command paths
- **Files:** `packaging/linux/`, `packaging/macos/`, `packaging/windows/`, `clients/desktop-web/src-tauri/`, `docs/msc2/clients/headless-installation.md`
- **What:** Audit desktop packages and headless installers on all three OSes. Install `msc` in a standard command location or installer-owned PATH entry; preserve conflict, upgrade, and uninstall ownership rules. Tell users when only a new shell can see a PATH change.
- **Verify:** `git diff --check`
- **Batch:** C (P17.8–P17.10) — installation and service

### P17.10 — Make boot and stop behavior consistent

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Commit:** P17.10: align agent boot and stop behavior
- **Files:** `crates/msc-platform-linux/src/service.rs`, `crates/msc-platform-macos/src/service.rs`, `crates/msc-platform-windows/src/service.rs`, `packaging/linux/`, `packaging/macos/`, `packaging/windows/`
- **What:** Start and enable the agent during approved installation on every OS, correcting macOS's current `RunAtLoad=false`. A routine stop lasts until an explicit start or next boot; a separate explicit disable action controls future boot startup. Preserve server-process shutdown guarantees.
- **Verify:** `git diff --check`
- **Batch:** C (P17.8–P17.10) — installation and service

### P17.11 — Add CLI access administration

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/users.rs`
- **What:** Expose named-token list/create/update/revoke for intentional delegation and inspect the current local authorization. Show a new delegated secret once through a safe terminal path; never use it to authenticate the local CLI. Keep admin permissions and expiry visible.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** D (P17.11–P17.15) — core administration

### P17.12 — Complete server discovery and transfer

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/session.rs`, `crates/msc-agent/src/routes/lifecycle.rs`
- **What:** Add list, detail, active selection, size, notes, and export/transfer commands over existing APIs. Make active-server context obvious before mutations; accept name or ID with clear ambiguity errors. Preserve create, import, start, stop, and EULA behavior.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** D (P17.11–P17.15) — core administration

### P17.13 — Complete status, metrics, sessions, and console reading

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `crates/msc-agent/Cargo.toml`, `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/transport.rs`
- **What:** Surface performance, host resources, player session log, and bounded console history; add cancellable live console follow if the stream contract supports it. Label unavailable edition-specific data honestly. Distinguish agent-service status from server status.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** D (P17.11–P17.15) — core administration

### P17.14 — Add player inspection and moderation

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/players.rs`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/clients/phase17-cli.md`
- **What:** Show online and known players, then expose message, kick, ban/pardon, operator, Java whitelist, and Bedrock allowlist tasks using existing permissions and capabilities. Leave other Minecraft commands to raw `msc command`.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** D (P17.11–P17.15) — core administration

### P17.15 — Add saved player-data operations

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/players.rs`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/clients/phase17-cli.md`
- **What:** Inspect available stats and inventory; duplicate/delete data and perform Java offline/custom UUID migration. Explain Bedrock's stopped-server requirement and Java-only migration; explicitly confirm destructive actions.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** D (P17.11–P17.15) — core administration

### P17.16 — Complete world and backup maintenance

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Commit:** P17.16: complete world maintenance commands
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/backups.rs`
- **What:** Add profile inspection, save-current, repair, safe live-world rename, supported conversion formats, and Chunker acquisition to the existing world/backup commands. Preserve mandatory safety backups, stopped-server guards, confirmation tokens, and operation progress; do not duplicate existing verbs.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** E (P17.16–P17.20) — content and configuration

### P17.17 — Manage packs attached to worlds

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Commit:** P17.17: add world pack management commands
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/components.rs`
- **What:** Inspect installed Java data packs and Bedrock behavior packs by slot; expose supported enable/disable/remove actions. For provider installs, show search, detail, version compatibility, dependencies, and target world before confirmation. Keep packs world-scoped.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** E (P17.16–P17.20) — content and configuration

### P17.18 — Expose host, server, and network settings

- **Status:** IMPLEMENTED — awaiting Cameron verification
- **Commit:** P17.18: expose host and network settings
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/settings.rs`, `crates/msc-agent/src/routes/networking.rs`
- **What:** Cover host setup, server root, memory, Geyser/cross-play, CurseForge key, Playit setup/reset, Xbox Broadcast state, Java resource-pack URL/required/remove, and watchdog controls through task-oriented CLI actions. Read secrets by prompt or protected input, never positional arguments or shell history. Report unsupported combinations.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** E (P17.16–P17.20) — content and configuration

### P17.19 — Make supported catalog installs inspectable

- **Status:** PLANNED
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/components.rs`
- **What:** Improve Modrinth-backed add-on search/install to search → project/version detail → compatibility and dependencies → explicit confirmation → progress/result. Keep local-JAR install and installed add-on update/enable/disable/remove usable; preserve JSON automation and provider errors. Do not claim CurseForge modpack browsing.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** E (P17.16–P17.20) — content and configuration

### P17.20 — Make archive modpack import and recovery clear

- **Status:** PLANNED
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/components.rs`
- **What:** Keep Modrinth `.mrpack` and CurseForge `.zip` as user-supplied archives. Show inspection, API-key setup, exact target, pack-managed consequences, progress, and author-blocked files with links and expected names. Resume with a matching local file through the existing operation-bound upload; make cancel/retry clear.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** E (P17.16–P17.20) — content and configuration

### P17.21 — Add operation and recovery controls

- **Status:** PLANNED
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/operations.rs`, `crates/msc-agent/src/routes/host_reset.rs`
- **What:** Inspect and cancel owned long-running operations and expose host reset with existing guards and exact confirmation. Keep local client reset distinct from host reset; preserve role permissions and no remote service-control route.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** F (P17.21–P17.24) — information and acceptance

### P17.22 — Read permitted files and built-in help

- **Status:** PLANNED
- **Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/routes/files.rs`, `crates/msc-agent/src/routes/help.rs`
- **What:** Browse/read permitted server files without escaping API path limits. Search and read handbook topics, onboarding guidance, and router guides as terminal text. Preserve permissions and avoid a full-screen interface.
- **Verify:** `cargo check -p msc-agent`
- **Batch:** F (P17.21–P17.24) — information and acceptance

### P17.23 — Document and audit the finished CLI

- **Status:** PLANNED
- **Files:** `README.md`, `docs/msc2/clients/phase17-cli.md`, `docs/msc2/clients/headless-installation.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/api-contract/openapi.json`
- **What:** Replace obsolete remote-CLI/token instructions, publish task-first local and SSH examples with edition limits, and close every row of the API-to-CLI inventory or record an owner-approved exception. Preserve desktop remote-host docs. Do not add release gates.
- **Verify:** `git diff --check`
- **Batch:** F (P17.21–P17.24) — information and acceptance

### P17.24 — Prepare physical acceptance without publishing

- **Status:** PLANNED
- **Files:** `docs/msc2/clients/phase17-cli-acceptance.md`, `docs/msc2/clients/phase17-cli.md`
- **What:** Record reproducible desktop/headless checks on macOS, Windows, and Linux: PATH, start/stop/status, reboot, local and SSH-shell authorization, wrong-user and forwarded-port refusal, representative server/player/world/backup/content commands, JSON scripting, provider failures, and uninstall ownership. Cameron runs and records results; the other agent reviews the gate. No test suite, CI gate, release tag, or publication run is implied.
- **Verify:** `rg -n 'macOS|Windows|Linux|SSH|reboot|stopped|PATH|wrong user|catalog|CurseForge' docs/msc2/clients/phase17-cli-acceptance.md`
- **Batch:** F (P17.21–P17.24) — information and acceptance
