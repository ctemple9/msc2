# MSC 2 — Rolling Plan

> ## STATUS: Phase 16 execution is underway. P16.1–P16.12 are implemented and awaiting Cameron's verification. The September 17 and September 28 audits remain the source findings; the remaining recommendations are not marked fixed.
> **Next move:** Cameron verifies P16.1–P16.12 using their `Verify:` commands.

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
| 16 | Release safety and codebase readiness | in progress |

## Phase 16 — Release safety and codebase readiness

**Execution.** One step per conversation unless Cameron names a `Batch:` range. Each executed step gets one commit containing its implementation and rolling-plan status update. Cameron closes the step after running its `Verify:` command. The checks below are builds, type checks, format checks, static validators, or production-path commands; none authorizes this agent to run or create tests. The Phase 16 gate is in `msc2-port-plan.md`. Physical and fault-path evidence is required at that gate; a successful `cargo check` alone never proves a race or recovery fix.

**Audit coverage.** September 17 findings 1–12 map respectively to P16.4, P16.2, P16.5, P16.3, P16.6, P16.10, P16.11, P16.12, P16.13, P16.7, P16.8, and P16.9. September 28 findings 1–12 map respectively to P16.14, P16.1, P16.16, P16.15, P16.17, P16.17, P16.17, P16.18, P16.19–20, P16.21, P16.22, and P16.23. P16.24 closes the release evidence gate. P15.101 landed after the September 28 snapshot; P16.17 first checks whether it already resolved that audit's player-action, permission, contract, and formatting findings. Do not rebuild an already-correct feature merely to satisfy this map.

### Scope and safety substrate

#### P16.1 — Record the Linux headless browser boundary

Status: implemented — awaiting Cameron's verification
Files: README.md, docs/msc2/msc2-decisions.md, docs/msc2/msc2-engineering.md, docs/msc2/clients/headless-installation.md, docs/msc2/rolling-plan.md
What: Record Cameron's September 28 clarification that the Linux headless archive does not serve a browser UI. Explain the difference between a host requiring no graphical desktop and an agent serving a remote browser page. Narrow the README and D-003 support wording to the intended installation types without removing browser access from packages that actually promise it. Keep the Linux headless API/CLI and no-GUI-link guarantees.
Verify: Run `rg -n 'Linux headless|browser|web UI' README.md docs/msc2/msc2-decisions.md docs/msc2/msc2-engineering.md docs/msc2/clients/headless-installation.md` and confirm one consistent support boundary.
Batch: solo

#### P16.2 — Make operation admission atomic

Status: implemented — awaiting Cameron's verification
Files: crates/msc-infrastructure/src/operation_journal.rs, crates/msc-application/src/operations.rs, crates/msc-agent/src/routes/operations.rs, docs/msc2/rolling-plan.md
What: Replace check-then-write admission with one shared reservation transaction per target. Reserve before work begins, persist enough state for restart reconciliation, refuse concurrent conflicting admissions, and release only at a truthful terminal transition. Preserve refusal rather than silently queuing. Design the mechanism to support host-wide maintenance in P16.3.
Verify: Run `cargo check -p msc-infrastructure -p msc-application -p msc-agent`.
Batch: solo

#### P16.3 — Reserve the host during reset

Status: implemented — awaiting Cameron's verification
Files: crates/msc-agent/src/routes/host_reset.rs, crates/msc-application/src/host_reset.rs, crates/msc-application/src/operations.rs, crates/msc-infrastructure/src/operation_journal.rs, docs/msc2/rolling-plan.md
What: Acquire a host-wide maintenance reservation before reset preconditions. Reject reset while any server mutation is active, reject new mutation while reset owns the host, and retain that exclusion through deletion, credential reset, and recovery after interruption. Report a useful conflict instead of deleting under a worker.
Verify: Run `cargo check -p msc-agent -p msc-application`.
Batch: solo

#### P16.4 — Confine world archives to world data

Status: implemented — awaiting Cameron's verification
Files: crates/msc-application/src/worlds.rs, crates/msc-infrastructure/src/archive.rs, crates/msc-agent/src/routes/worlds.rs, docs/msc2/rolling-plan.md
What: Define allowed Java and Bedrock archive roots and entries before activation. Reject extra executables, configuration, links, and malformed layouts before moving live folders. Install only approved world paths so a Worlds credential cannot overwrite a server JAR or settings. Preserve legitimate legacy world layouts through explicit normalization rather than a full-server merge.
Verify: Run `cargo check -p msc-application -p msc-agent`.
Batch: solo

#### P16.5 — Recover partial world replacement deterministically

Status: implemented — awaiting Cameron's verification
Files: crates/msc-application/src/worlds.rs, crates/msc-application/src/backups.rs, crates/msc-infrastructure/src/operation_journal.rs, crates/msc-agent/src/routes/lifecycle.rs, docs/msc2/rolling-plan.md
What: Give activation and restore a durable progress manifest listing the folders to move and install. On restart, reconcile every partial phase, including destinations already created, to one complete old or new world. Keep further mutation blocked while recovery is incomplete and surface a repair error if neither state can be proven.
Verify: Run `cargo check -p msc-application -p msc-infrastructure`.
Batch: solo

#### P16.6 — Match online-backup acknowledgements to the current save

Status: implemented — awaiting Cameron's verification
Files: crates/msc-agent/src/backup_operations.rs, crates/msc-agent/src/routes/lifecycle.rs, crates/msc-agent/src/ws/console.rs, crates/msc-application/src/backups.rs, docs/msc2/rolling-plan.md
What: Capture a server-run and console-sequence boundary when issuing the save command. Accept only later matching acknowledgement from that run; preserve the separately documented timeout policy. Prevent an older line in the retained console tail from certifying a new online backup.
Verify: Run `cargo check -p msc-agent -p msc-application`.
Batch: solo

#### P16.7 — Authorize operation cancellation

Status: implemented — awaiting Cameron's verification
Files: crates/msc-agent/src/auth.rs, crates/msc-agent/src/routes/operations.rs, crates/msc-application/src/operations.rs, crates/msc-infrastructure/src/operation_journal.rs, crates/msc-infrastructure/tests/operation_exclusivity.rs, crates/msc-infrastructure/tests/operation_journal.rs, docs/msc2/rolling-plan.md
What: Store each operation's initiating credential and required permission in its record/journal. Check both when cancelling, with an explicit owner/admin override policy; continue to return the true terminal state when a worker already finished. Do not let knowledge of an operation ID grant cancellation rights.
Verify: Run `cargo check -p msc-agent -p msc-application -p msc-infrastructure`.
Batch: solo

#### P16.8 — End WebSocket streams when credentials end

Status: implemented — awaiting Cameron's verification
Files: crates/msc-agent/src/auth.rs, crates/msc-agent/src/auth/browser.rs, crates/msc-agent/src/ws/console.rs, crates/msc-agent/src/ws/notifications.rs, docs/msc2/rolling-plan.md
What: Bind upgraded console and notification streams to credential/session identity. Close them on revocation, expiry, and host-identity reset, including while idle; do not rely on another incoming HTTP request. Keep one-use stream-ticket behavior intact.
Verify: Run `cargo check -p msc-agent`.
Batch: solo

#### P16.9 — Bound operation history and admission cost

Status: implemented — awaiting Cameron's verification
Files: crates/msc-application/src/operations.rs, crates/msc-infrastructure/src/operation_journal.rs, crates/msc-agent/src/routes/operations.rs, docs/msc2/rolling-plan.md
What: Set a documented retention limit for terminal records and cancellation flags, preserve the durable records needed for recovery and user history, and admit against an active-reservation index instead of rescanning every historical file. Make cleanup safe across restart and ensure long-lived hosts have bounded memory and admission time.
Verify: Run `cargo check -p msc-application -p msc-infrastructure -p msc-agent`.
Batch: solo

### Clients and platform recovery

#### P16.10 — Publish one host connection generation at a time

Status: implemented — awaiting Cameron's verification
Files: clients/desktop-web/src/App.svelte, docs/msc2/rolling-plan.md
What: Keep a newly connected client and its readiness, server list, status, and host-context state local until the generation still matches. Cancel or ignore every stale asynchronous continuation, including host-context restoration and navigation. Prevent a late host A result from replacing host B's transport or state.
Verify: Run `npm run check` from `clients/desktop-web`.
Batch: solo

#### P16.11 — Give the Windows agent a real service lifecycle

Status: implemented — awaiting Cameron's Windows verification
Files: crates/msc-agent/src/main.rs, crates/msc-agent/src/windows_service.rs, crates/msc-agent/src/cli/mod.rs, crates/msc-platform-windows/src/service.rs, crates/msc-agent/Cargo.toml, Cargo.lock, packaging/windows/service-lifecycle.md, docs/msc2/rolling-plan.md
What: Make the production-installed executable complete the Windows Service Control Manager start handshake and handle stop/shutdown control, or package a production wrapper that does so. Keep the installing-user identity and normal CLI `serve` mode. Validate the exact installer-created service path on Windows rather than relying on the separate lifecycle smoke wrapper.
Verify: On Windows, run `cargo check -p msc-agent -p msc-platform-windows`.
Batch: solo

#### P16.12 — Update Windows headless without self-replacement

Status: implemented — awaiting Cameron's Windows verification
Files: crates/msc-agent/src/cli/update.rs, crates/msc-platform-windows/src/service.rs, packaging/windows/service-lifecycle.md, docs/msc2/rolling-plan.md
What: Launch a distinct temporary updater image/process so the running `msc.exe` is never asked to remove itself. Preserve the prior service state on every extraction, replacement, restart, and health-check failure; keep rollback bytes until health succeeds. Cover both initially running and stopped services.
Verify: On Windows, run `cargo check -p msc-agent -p msc-platform-windows`.
Batch: solo

#### P16.13 — Retain macOS desktop rollback until health succeeds

Status: implemented — awaiting Cameron's macOS verification
Files: clients/desktop-web/src-tauri/src/update.rs, clients/desktop-web/src-tauri/Cargo.toml, docs/msc2/rolling-plan.md
What: Keep the previous signed app bundle until the replacement launches and the coordinated desktop/agent health check succeeds within a deadline. Restore and relaunch the previous bundle when launch or health fails, including authorized replacement paths. Clean rollback bytes only after success.
Verify: On macOS, run `cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`.
Batch: solo

### Release build and current client follow-ups

#### P16.14 — Embed the same frontend bytes in desktop and browser packages

Status: planned — awaiting Cameron's review
Files: .github/workflows/ci.yml, .github/workflows/release.yml, clients/desktop-web/package.json, clients/desktop-web/tools/package-agent-bundle.mjs, crates/msc-agent/src/web_ui.rs, tools/release/check-client-bundle.py, docs/msc2/rolling-plan.md
What: Build the shared frontend once before compiling any agent that serves it. Feed the same output to Tauri and the agent; decide whether `web-ui` is generated or checked in and enforce synchronization. Add a release artifact identity/content comparison for installation types that serve browsers. Preserve the Linux headless exclusion from P16.1.
Verify: Run `python3 tools/release/check-client-bundle.py clients/desktop-web/dist crates/msc-agent/web-ui` after producing the staged frontend and confirm matching content.
Batch: solo

#### P16.15 — Build Linux artifacts to the promised minimum

Status: planned — awaiting Cameron's review
Files: .github/workflows/release.yml, tools/release/build-linux-headless.sh, docs/msc2/clients/phase12-release.md, docs/msc2/rolling-plan.md
What: Pin the Linux release builder/toolchain to a baseline compatible with Debian 12 and record the minimum required glibc symbols for both archive and desktop artifacts. Require a clean Debian 12 install/launch of the exact release bytes, plus a current Fedora path, before publication. Keep no-GUI-link checks for the headless archive.
Verify: On clean Debian 12 with the staged headless artifact installed, run `msc --help` and confirm the binary launches without a missing-symbol error.
Batch: solo

#### P16.16 — Gate publication on same-commit CI evidence

Status: planned — awaiting Cameron's review
Files: .github/workflows/ci.yml, .github/workflows/release.yml, tools/release/check-release-workflow.py, docs/msc2/rolling-plan.md
What: Make tag publication require the full required Rust, client, browser, platform, and native checks for the same source commit as the release artifacts. Record exact workflow run IDs and refuse publication on missing, stale, cancelled, or failed jobs; keep the existing signed-manifest and nine-artifact checks.
Verify: Run `python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard`.
Batch: solo

#### P16.17 — Reconcile P15.101 with the September audit

Status: planned — awaiting Cameron's review
Files: crates/msc-agent/src/routes/commands.rs, crates/msc-agent/src/routes/bedrock.rs, crates/msc-agent/src/routes/lifecycle.rs, clients/desktop-web/src/lib/sections/players-online, clients/desktop-web/src/lib/api/generated.ts, .github/workflows/ci.yml, .github/workflows/release.yml, docs/msc2/rolling-plan.md
What: Inspect the committed P15.101 server-selection guard, Bedrock allowlist guard, Java/Bedrock permission checks, generated API types, and formatting against September 28 findings 5–8. Record which are already fixed and repair only remaining gaps. Add `api:check` to required CI and release checks so later contract edits cannot leave generated types stale; preserve the one-server dispatch invariant under concurrent selection.
Verify: Run `npm run api:check` from `clients/desktop-web`.
Batch: frontend-quality (P16.17–P16.18)

#### P16.18 — Clear frontend static warnings and formatting debt

Status: planned — awaiting Cameron's review
Files: clients/desktop-web/src/lib/sections/worlds/WorldSettingsForm.svelte, clients/desktop-web/src/lib/sections/setup, clients/desktop-web/src/lib/sections/players-online, docs/msc2/rolling-plan.md
What: Remove or justify the eight Svelte warnings reported on September 28, and keep Prettier clean after P15.101. Change only dead selectors/unused exports or the smallest necessary view wiring; avoid visual redesign. Document any warning intentionally retained.
Verify: Run `npm run check` from `clients/desktop-web` and confirm zero unexplained warnings.
Batch: frontend-quality (P16.17–P16.18)

### Repository quality and public handoff

#### P16.19 — Split large Rust modules along behavior boundaries

Status: planned — awaiting Cameron's review
Files: crates/msc-agent/src/routes/worlds.rs, crates/msc-agent/src/routes/lifecycle.rs, crates/msc-agent/src/routes/servers.rs, crates/msc-application/src/worlds.rs, docs/msc2/rolling-plan.md
What: Extract cohesive world import/activation/recovery, server lifecycle, and route orchestration modules after the safety fixes settle. Move in-file verification helpers beside their subjects without adding new tests. Preserve public routes, permissions, transaction order, and API schemas. Do not pursue an arbitrary line-count target or a whole-repo rewrite.
Verify: Run `cargo check -p msc-agent -p msc-application`.
Batch: solo

#### P16.20 — Split host connection orchestration from the app shell

Status: planned — awaiting Cameron's review
Files: clients/desktop-web/src/App.svelte, clients/desktop-web/src/lib/hosts, clients/desktop-web/src/lib/sections, docs/msc2/rolling-plan.md
What: Extract the cohesive host-connection and generation-state logic stabilized in P16.10 into a named client module. Keep App.svelte focused on shell composition and navigation while preserving visible behavior and the single shared desktop/browser screen contract. Remove obsolete phase comments in touched code when they obscure current intent.
Verify: Run `npm run check` from `clients/desktop-web`.
Batch: solo

#### P16.21 — Publish owner-selected license and security contact

Status: planned — requires Cameron's source/distribution license choice before execution
Files: LICENSE, SECURITY.md, CONTRIBUTING.md, README.md, docs/msc2/msc2-decisions.md, docs/msc2/rolling-plan.md
What: Present Cameron with concrete license choices and the effect on reuse/contributions; publish only his selected terms. Add a private vulnerability-reporting route, supported-version policy, contributor start instructions, and notices/ownership for third-party code and bundled Bedrock VM material. Do not infer an open-source license from the word “free.”
Verify: Run `rg -n 'License|Security|Contribut|vulnerabilit' README.md LICENSE SECURITY.md CONTRIBUTING.md` and inspect that the published terms match Cameron's recorded choice.
Batch: solo

#### P16.22 — Record dependency and artifact provenance

Status: planned — awaiting Cameron's review
Files: .github/workflows/ci.yml, .github/workflows/release.yml, rust-toolchain.toml, clients/desktop-web/package-lock.json, Cargo.lock, tools/release/check-provenance.py, docs/msc2/rolling-plan.md
What: Pin release-critical toolchains/actions or record exact immutable versions, generate a reviewable dependency and component inventory for each release, and define advisory/license triage with an owner and documented exceptions. Add a static release check that binds source commit, toolchain, manifest, and artifacts without treating a scan as a security proof.
Verify: Run `python3 tools/release/check-provenance.py --manifest target/release-assets/msc2-update-manifest.json` against staged release metadata; it must print the source commit, toolchain, and dependency inventory paths.
Batch: solo

#### P16.23 — Publish a current support and release-status page

Status: planned — awaiting Cameron's review
Files: README.md, docs/msc2/clients/phase12-release.md, docs/msc2/clients/headless-installation.md, docs/msc2/rolling-plan.md
What: Separate historical beta plans from current public support claims. State exact platform/architecture and browser availability by installation type, unsigned installer limitations, current release link, update path, and support floor; link to the Phase 16 acceptance record once it exists. Keep previous phase evidence as history rather than rewriting it as current proof.
Verify: Run `rg -n 'headless|browser|Debian 12|unsigned|release' README.md docs/msc2/clients/headless-installation.md` and confirm the public claims match the support matrix.
Batch: solo

#### P16.24 — Record exact-artifact release acceptance

Status: planned — awaiting all earlier Phase 16 steps and Cameron's physical verification
Files: docs/msc2/release/phase16-acceptance.md, tools/release/check-phase16-evidence.py, docs/msc2/rolling-plan.md
What: Assemble evidence for every Phase 16 gate item and every supported installer/archive using exact published bytes: install, pairing and supported clients, Minecraft lifecycle, world import/backup/restore and interruption recovery, service reboot/sign-out, update rollback, permission and revocation, uninstall/data retention, artifact identity, CI/provenance, and Linux minimum. Mark unavailable or failed checks honestly; do not close the phase while any required row lacks Cameron's result. The reviewer for this phase must be the other agent, per repository rules.
Verify: Run `python3 tools/release/check-phase16-evidence.py docs/msc2/release/phase16-acceptance.md` and confirm it accepts only complete gate rows with exact artifacts and Cameron's observed results.
Batch: solo

## Public-release and codebase review — 2026-09-28

**Purpose and scope.** This is a fresh static review of the current source, the uncommitted player-actions/API edits present on September 28, the release and CI workflows, packaging, documentation, and the September 17 findings. It is a release-readiness audit, not a declaration that the product has passed its platform acceptance gates. Findings are recommendations for Cameron to triage; no product decision is changed here. In particular, D-003 requires one shared desktop/browser frontend, D-011 requires useful headless installations on all three platforms, and D-032 requires recoverable coordinated updates.

**What is already strong.** The repository has explicit domain/application/infrastructure/platform/API boundaries, an unusually detailed decision record and phase evidence, cross-platform CI, generated API types, signed update manifests, and focused packaging checks. `cargo fmt --all -- --check`, the repository's CI-equivalent `cargo clippy --workspace --all-targets` invocation, and `npm run check` passed locally on September 28. The Svelte check reported eight warnings, no errors. No test command was run, per the repository rule.

### Release blockers and one support-scope clarification

1. **High — Published agent/browser UI can lag behind the desktop UI.** The agent embeds the tracked `crates/msc-agent/web-ui` directory at compile time (`crates/msc-agent/src/web_ui.rs:10–11`). Its last commit was September 24, while frontend source changed through September 28. CI builds `dist` and then builds the agent (`.github/workflows/ci.yml:111–120`) without copying `dist` into `web-ui`; the release workflow builds the agent before Tauri builds the current frontend (`.github/workflows/release.yml:143–172`). The available `bundle:package-agent` script is not called by either workflow. Thus a new installer can carry a current desktop screen and an older agent-served browser screen, violating D-003. Build the frontend once, embed those exact bytes in the agent **before** compiling it, and compare the packaged desktop and served browser bundle identity/content as a release gate. Decide whether generated bundles remain tracked or are produced only by the build, then enforce that choice.

2. **Scope clarification — Linux headless deliberately has no browser interface.** `tools/release/build-linux-headless.sh:28` compiles with `--no-default-features`, which disables `msc-agent`'s default `web-ui` feature; `crates/msc-agent/src/web_ui.rs:27–30` then returns `web_ui_unavailable`. Cameron clarified on September 28 that he does not want a browser interface in the Linux headless archive. The earlier review incorrectly treated its absence as a code defect. The remaining issue is documentation and decision consistency: `README.md:153–155` says a headless host can be managed from a desktop browser, while Approved D-003 says the agent serves the shared frontend to browsers. Resolve whether that browser promise applies only to other installation types, then align the README and decision/support matrix with Cameron's intended scope. Do not add a browser bundle to the Linux headless archive on the basis of this audit.

3. **High — The release workflow can publish without the full CI gates passing for the tag.** `.github/workflows/release.yml:121–141` runs a targeted subset, then `publish` depends only on its own `build` matrix (`:230–237`). It does not require the full workspace regression suite, browser workflows, native desktop checks, or platform smokes in `.github/workflows/ci.yml`. A green release workflow therefore is not evidence that the release commit passed the repository's own acceptance suite. Make publication depend on a successful, same-commit required-check set, and record the run IDs for the exact tag and artifact set. Do not treat a prior `main` run as evidence for a later tag.

4. **High — The Linux compatibility floor is unproved by the release build.** The release contract promises Debian 12 and `systemd` ≥250 (`docs/msc2/clients/phase12-release.md`, §3), but `.github/workflows/release.yml:47–50` builds GNU/Linux binaries and desktop packages on floating `ubuntu-latest`; no release job checks the resulting ELF's minimum glibc symbol version or installs the artifact on Debian 12. A binary built against a newer runner libc may fail before MSC starts. Pin a build baseline compatible with the oldest supported distro and add clean Debian 12 installation/launch evidence for the **release artifact**, including its service and CLI behavior. This is a verified gap in the gate, not a claim that every current artifact fails.

5. **High — The draft player-action guard can still send a command to the wrong server.** In the uncommitted change, `crates/msc-agent/src/routes/commands.rs:47–55` checks `expectedActiveServerId`, then later selects and sends through separately locked state (`:61–72` and `routes/lifecycle.rs:2035–2056`). Another request can select a different active server between those operations. The Bedrock allowlist path checks `/v1/status` in the client and then makes a separate mutation (`PlayerActionsSheet.svelte:162–169`), with the same gap. Bind expected server ID, authorization, and dispatch to one server-scoped operation on the agent; give the allowlist request an equivalent server-scoped guard. Reproduce an interleaved selection/action before accepting the feature.

### Previous audit: current disposition

The September 17 audit remains below in full, with its evidence and proposed remedies. Static reinspection found **#1–5 and #7–12 still open**: world ZIP overwrite, non-atomic operation admission, partial-world recovery, host reset overlap, stale save acknowledgement, Windows service entry point, Windows self-update, macOS rollback timing, cancellation authorization, WebSocket revocation, and unbounded operation history. **#6 is partially mitigated** by generation checks now present in `App.svelte:794–820,949–1000`; `client` is still assigned before the generation check at `:989`, and `restoreHostContext` publishes readiness/host state across multiple awaits. Keep #6 open until stale connections cannot publish *any* client or host state. These are release blockers where they threaten worlds, access control, service startup, or update recovery; do not count them as closed because a later phase was marked complete.

### Codebase and release-process quality findings

6. **Medium — Generated API types are allowed to drift from the contract.** `npm run api:check` currently fails because `docs/msc2/api-contract/openapi.json` changed but `clients/desktop-web/src/lib/api/generated.ts` did not. Neither CI nor the release workflow invokes `api:check`; `npm run check` passes anyway. Add this check to required CI/release gates and regenerate as part of the feature commit. This exact failure is in the uncommitted September 28 worktree; it is not attributed to the last commit.

7. **Medium — The draft Java access-list buttons advertise the wrong permission.** `PlayersOnlineSection.svelte:57–58` enables access-list actions for `players` permission. The draft sheet sends Java `whitelist` changes through `/v1/command` (`PlayerActionsSheet.svelte:111–132,172–176`), which requires `serverControl` (`routes/commands.rs:32–34`). A Players-only user sees an enabled action that fails with 403; a ServerControl-only user has the action hidden despite being authorized by the route. Align the client capability check with the actual endpoint or introduce a dedicated Players-authorized mutation, and document the permission boundary.

8. **Medium — The current draft fails the frontend formatting gate.** `npm run format:check` reports `OnlineNowCard.svelte`, `PlayerActionsSheet.svelte`, and `PlayersOnlineSection.svelte`; `cargo fmt` passes. Format the owned draft before its commit. `npm run check` passes but still reports eight existing Svelte warnings in world settings and setup components; clear those or explicitly justify them before claiming a warning-clean frontend.

9. **Medium — High-change modules are difficult to audit as units.** `routes/worlds.rs` is 5,463 lines, `routes/lifecycle.rs` 4,196, `routes/servers.rs` 4,012, `msc-application/src/worlds.rs` 3,101, and `App.svelte` 1,414. Several route files combine HTTP parsing, orchestration, filesystem work, and in-file tests. The structure has sound crate boundaries, but these files make state/permission/transaction invariants hard to follow; the September 17 races sit at those seams. Split by cohesive behavior (world import/activation/recovery, service state, host connection) and move large in-file test modules beside their subjects while preserving behavior. Do this after safety fixes, with focused review of the moved call paths; a line-count target by itself is not the goal.

10. **Medium — The public repository lacks a licensing and security-reporting contract.** There is no tracked `LICENSE`, `SECURITY.md`, or contributor guide, while `README.md:253` invites contribution and the product distributes a network-accessible server manager. Cameron should choose and publish the source/distribution license, describe third-party component notices and bundled Bedrock VM material, and give researchers a private vulnerability-reporting route with supported-version expectations. This is a publication/legal ownership decision for Cameron, not a license choice made by this audit.

11. **Medium — Dependency and release provenance review is absent from required gates.** The workflows install Rust/Node dependencies and third-party GitHub actions, but contain no required vulnerability/license scan, dependency-update policy, or published software bill of materials. Add a reviewable dependency inventory and security advisory process with severity triage; pin or otherwise verify release-critical actions/tooling and record exact toolchain/artifact provenance. This matters especially because `rust-toolchain.toml` says `stable` and release runners float, so reproducing an old installer from its tag may select different compilers and OS libraries.

12. **Low — The documentation should distinguish current evidence from historical plan language.** The README still links v0.1.16 and calls the release unsigned; `phase12-release.md` describes an earlier beta candidate workflow, while the September 28 plan calls phases complete and has unresolved release blockers. Keep a short, versioned public support/release-status page and point the README at it; retain historical phase records as history. State clearly which installation types serve a browser UI once the scope clarification in #2 is resolved.

### Closure gate for a public release

Close the high findings above and September 17 findings with code review and Cameron-run verification. For each supported installer/headless archive, record clean-install, first-run, supported remote pairing/CLI, real Minecraft start/stop, world import/backup/restore and crash recovery, service reboot/sign-out survival, update failure/rollback, and uninstall/data-retention results against the **exact published bytes**. Verify browser access only for installation types that are intended to serve it; Linux headless is excluded by Cameron's September 28 clarification. Include macOS Intel Bedrock helper and Apple Silicon limitation, Windows Service Control Manager behavior, Debian 12 plus a current Fedora Linux path, multi-host switching under delayed responses, and permission/revocation cases. Require generated contract sync, formatting/type/lint, full same-commit CI, artifact identity, dependency/provenance review, and owner approval of licensing/support claims. Keep regression checks specific to each fixed failure mode. This is a proposed release gate, not evidence that it has run.

**Limits of this review.** Static inspection cannot guarantee that no further work will surface under senior review or on physical machines. I did not run tests, build release installers, inspect every file or third-party dependency, compare every behavior to the MSC 1 Swift oracle, or verify current GitHub Actions and published binaries. The uncommitted player-action files may change before they are committed; recheck findings #5–8 after that edit is finished. The glibc concern is a compatibility risk requiring artifact evidence. No finding here silently approves a Proposed or Open decision.

## External review audit — 2026-09-17

The review followed the Rust domain/application crates, infrastructure and platform crates, agent/API boundaries, Svelte/Tauri clients, and packaging/update tooling. Confidence is high for each finding. The recommendations below are recorded for triage; they are not yet implementation decisions.

1. **High — A world ZIP can replace the server executable.** `crates/msc-application/src/worlds.rs:1887` activates an imported archive through the unrestricted move at `crates/msc-application/src/worlds.rs:1646`. A ZIP containing valid world data plus a top-level file matching the configured Java server JAR can overwrite that JAR on macOS/Linux; configuration can also be overwritten by an accidental full-server archive. Import and activation require only Worlds permission. Validate an edition-specific world-path set before moving anything, reject executable/configuration/non-world entries, and never merge an arbitrary archive into the server root.

2. **High — Conflicting operations can both pass admission.** `crates/msc-infrastructure/src/operation_journal.rs:172`, called from `crates/msc-application/src/operations.rs:184`, checks for conflicts and writes the journal separately. Concurrent requests for one server can both see no conflict and then mutate the same files. Reserve each server atomically through a shared admission mechanism, persist the reservation, and retain it until terminal state.

3. **High — Interrupted world replacement cannot reliably recover partial folder moves.** Restore at `crates/msc-application/src/backups.rs:798` and recovery at `:849`, with equivalent activation recovery at `crates/msc-application/src/worlds.rs:1995`, infer progress from directory existence even though several independent renames occur. A stop after one folder is installed can leave nonempty partial destinations that old-folder renames cannot replace. Persist explicit transaction progress, handle partially installed destinations, and keep mutation unavailable until recovery establishes a complete old or new world.

4. **High — Host reset can delete files underneath an active operation.** `crates/msc-agent/src/routes/host_reset.rs:107` reaches deletion at `crates/msc-application/src/host_reset.rs:113`. Reset checks Minecraft and other resets but does not exclude restore, activation, backup, or provisioning workers; its `target: None` journal registration conflicts with nothing. Acquire a host-wide maintenance reservation before reset preconditions, refuse reset while conflicting work remains, and block new work through completion.

5. **High — Online backup readiness can be satisfied by an old save acknowledgement.** `crates/msc-agent/src/backup_operations.rs:219` searches retained console history without recording the console position when the current save command is issued. A prior acknowledgement among the last 50 lines can make a new backup copy changing files. Capture a console sequence boundary and accept only subsequent acknowledgements from the relevant server run. This finding concerns stale acknowledgement matching and does not settle the documented best-effort timeout behavior.

6. **High — A delayed connection can replace the client after the user switches hosts.** `clients/desktop-web/src/App.svelte:916`, with related assignments at `:725`, can publish a late connection A after the user has switched to host B. A can replace B's transport or overwrite B's readiness/server/status state. Await into local variables, check the connection generation, and publish the client and associated state together; apply the same ordering to readiness and host-context restoration.

7. **High — The production Windows service launches an executable without a Windows service entry point.** `crates/msc-platform-windows/src/service.rs:151` renders a direct `msc … serve` command at `:306`, while `crates/msc-agent/src/main.rs:39` enters the ordinary CLI/HTTP path. The production executable contains no service dispatcher, entry point, or service-control handler, so Service Control Manager startup cannot complete the expected handshake. Implement the native Windows service lifecycle or ship a wrapper used by the production installer. The existing lifecycle script's separate `ServiceBase` wrapper does not prove the production path.

8. **Medium — The Windows headless updater attempts to replace its own running executable.** `crates/msc-agent/src/cli/update.rs:131` launches `msc.exe update apply`, and replacement occurs at `:570`. On Windows the helper remains an active instance of the image it must remove, so replacement can fail; a stopped service is not restarted when this error returns. Run an independent updater or temporary executable copy and restore the prior service state on every installation failure.

9. **Medium — macOS update rollback is discarded before launch or health recovery.** `clients/desktop-web/src-tauri/src/update.rs:283` deletes the previous app bundle after renaming it, before the launch at `:217` proves desktop/agent health. A correctly signed replacement that cannot launch leaves the owner without the previous coordinated installation, contrary to Approved D-032. Retain the previous installation until health succeeds within a deadline and restore it on launch or health failure.

10. **Medium — Operation cancellation bypasses permission categories.** `crates/msc-agent/src/routes/operations.rs:259` authenticates the request but does not authorize the credential against the operation. A guest without world-management permission who learns a backup or restore ID can interrupt work they could not initiate. Associate operations with their required permission and initiating identity, then authorize cancellation centrally; make any owner/admin override explicit.

11. **Medium — Credential revocation leaves existing WebSocket streams authorized indefinitely.** `crates/msc-agent/src/ws/console.rs:140` and `crates/msc-agent/src/ws/notifications.rs:103` authenticate only during upgrade. An open socket continues receiving console or notification data after revocation or expiry. Bind each stream to credential/session state and close it on revocation, expiry, and host-identity reset.

12. **Medium — Operation history grows without bounds and is rescanned for every admission.** `crates/msc-application/src/operations.rs:90` retains terminal records and cancellation flags without eviction, while `crates/msc-infrastructure/src/operation_journal.rs:193` rescans historical journal files. Repeated scheduled operations therefore grow memory and disk use and make admission progressively more expensive, contrary to Approved D-021. Bound terminal history, discard completed cancellation flags, maintain a bounded active-reservation index, and preserve only the durable recovery/history data required by the chosen limit.

**Review limits:** the reviewer did not inspect every line or dependency, compare directly against MSC 1 Swift source, inspect shipped binaries, validate live releases, or exercise Minecraft, browsers, or OS service managers. Existing tests, fixtures, and smoke scripts were used as source material only. The review found no additional verified issue in browser origin/CSRF enforcement, credential-verifier storage, file-browser path confinement, provider checksum enforcement, signed-manifest validation, or Bedrock sidecar messaging; this is not an exhaustive safety claim. Phase 14 acceptance and platform behavior still require Cameron's verification.
