# MSC 2 — Rolling Plan

> ## STATUS: Phase 9 complete — gate reviewed and passed. Phase 10 is ready for PLAN. The exact Phase 9 code candidate `94802fcf85ade8dbf30aaface98cd51cbbb50ec3` passed CI run [32605909026](https://github.com/ctemple9/msc2/actions/runs/32605909026) across macOS, Linux, Windows, and the headless no-GUI job. No live-provider success is claimed where evidence was unavailable.
> **Next move:** PLAN Phase 10 — Bedrock runtimes. The complete Phase 9 step history, amendments, and gate review are in `rolling-plan-archive.md`.
> **Repo:** https://github.com/ctemple9/msc2 · GitHub Actions run [32544701401](https://github.com/ctemple9/msc2/actions/runs/32544701401) is green for exact Phase 8 code candidate `3e04f484bdbee3e821ea55dda6a06cc8e8f5c887`, including repository invariants, macOS, Linux, Windows, and the headless no-GUI link check.
> **Last updated:** 2026-08-22

**Previous phases (Setup, Phase 0 through Phase 9) and their amendments have moved to `rolling-plan-archive.md`** to keep this file small. That archive is historical only — current status and active work stay here.

---

## How this document works

This is the **working state** of the build. The vision documents say where we're going; the port plan says in what order; this file says **where we actually are right now**.

Phases are fixed and come from `msc2-port-plan.md`. **Steps are written one phase at a time**, as we reach each phase — not up front. Phase 10 is now the active planning task.

Each phase runs the six-move loop in `CLAUDE.md`: Plan → Read → Execute → Verify → Review → Advance.

### Step format

Every step looks like this:

```
### P0.3 — Extract TPS parser fixtures
**Status:** not started | in progress | awaiting verification | DONE
**Files:** fixtures/tps/, tools/extract-fixtures/
**What:** Pull the 27 TPS test cases out of MSC 1's TpsMonitoringTests.swift
         into input/expected JSON pairs.
**Verify:** `ls fixtures/tps/*.json | wc -l` → 27
**Commit:** P0.3: extract TPS parser fixtures        <- the message, not a hash
```

Every step also carries a **Batch:** field, telling an agent whether it may be run unattended:

| Batch value | Meaning |
|---|---|
| `safe` | Mechanical, and its Verify is a script Cameron has already reviewed. Batch freely. |
| `stop-after` | Runnable in a batch, but the batch **ends here** — the result needs looking at before continuing. |
| `solo` | Judgment work or a new checker script. Run it alone. Its output needs a cross-check by the other agent before the phase closes. |

**Status is only moved to DONE by Cameron**, after he runs the Verify command himself. An agent may set it to *awaiting verification* and stop.

**A step whose Verify only counts things is `stop-after` at best.** Counting proves something exists, not that it is right.

---

## Phases

Gates are in `msc2-port-plan.md`. This is the map, not the detail.

| Phase | Name | State |
|---|---|---|
| **Setup** | Repo, docs, agent instructions, CI, editor config | complete |
| **0** | Freeze the baseline and build the harness | complete |
| 1 | Domain types and pure rules | complete |
| 2 | API contract and operation model | complete |
| 3 | Safety substrate | complete |
| 4 | Java lifecycle vertical slice | complete |
| 5 | Configuration and migration | complete |
| **6** | Worlds and backups | complete |
| **7** | Server families and provisioning | complete |
| **8** | Mods, plugins, modpacks | complete |
| **9** | Networking and helpers | complete |
| **10** | Bedrock runtimes | **planned — PLAN next** |
| 11 | Desktop and web clients | not started |
| 12 | Terminal UI (deferred from v1) | not started |

---

## Phase 6 — Worlds and backups

All 51 steps (P6.1–P6.51) are `DONE`, and Codex's gate review (2026-08-18) confirms the gate holds on exact candidate `8568dea`. The full record — scope, characterization, world/backup services, public-client wiring, gate-review corrections, exact-commit tri-platform CI proof, and the review itself — has moved to `rolling-plan-archive.md`.

---

## Phase 7 — Server families and provisioning

All 38 steps (P7.1–P7.38) are `DONE` per Cameron. The final exact candidate is `79d5044` on `phase7-corrections`; GitHub Actions run [32448912726](https://github.com/ctemple9/msc2/actions/runs/32448912726) is fully green across macOS, Linux, Windows, the Phase 7 provisioning/launch smoke, and the headless no-GUI check. The full record — scope, characterization, six-family providers and provisioning, runtime and installer behavior, diagnostics and repairs, review corrections, evidence, and amendments — has moved to `rolling-plan-archive.md`.

---

## Phase 9 — Networking and helpers

Phase 9 is complete and its full step history, amendments, and gate review are
archived in [rolling-plan-archive.md](rolling-plan-archive.md). The exact code
candidate `94802fcf85ade8dbf30aaface98cd51cbbb50ec3` passed GitHub Actions run
[32605909026](https://github.com/ctemple9/msc2/actions/runs/32605909026) across
macOS, Linux, Windows, and the headless no-GUI link check. The approved
loopback-by-default/Tailscale-only management boundary holds, and unavailable
live-provider operations remain recorded honestly.

## Phase 10 — Bedrock runtimes

**Gate** (`msc2-port-plan.md` §3): the `BedrockRuntime` trait is implemented
native Linux → native Windows → macOS VZ Swift sidecar, without absorbing
macOS-specific assumptions. Bedrock files, properties, players, LevelDB,
allowlist, permissions, metrics, and UDP behavior pass shared fixtures. The
Bedrock compatibility matrix is published separately from the MSC-agent
matrix, and imported Bedrock records are reconciled with actual lifecycle and
settings behavior.

**Working exit criteria:** a supported Bedrock server can be imported or
created, provisioned from verified official BDS files, started, observed,
commanded, stopped, and recovered through the same public API, CLI, and copied
iOS contract as a Java server. Linux and Windows use native BDS processes;
macOS uses the already-frozen JSON-lines Swift-sidecar protocol over stdio and
Virtualization.framework. Console output, readiness, metrics, rolling logs,
players, allowlist, permissions, settings, worlds, and backups behave against
the shared fixture corpus. No client claims Bedrock support on a platform or
version that the published compatibility evidence does not prove.

**Source oracle:** MSC 1 at `~/Documents/Swift Projects/minecraft-server-controller`,
read-only. The Phase 10 source set begins with `BedrockPropertiesManager.swift`,
`BedrockProvisioner.swift`, `BedrockVersionFetcher.swift`,
`BedrockServerBackend.swift` (Docker-based, D-008 — behavioral reference only,
never ported), `VMBedrockServerBackend.swift`, `BedrockPlayerDataManager.swift`,
`BedrockNameCache.swift`, `BedrockHiddenProfiles.swift`, `BedrockLevelDB.swift`,
`BedrockNBTReader.swift`, `UDPRelay.swift`, `AppViewModel+ServerCreation.swift`
(including `resolvedBedrockWorldFolder` and its `WorldSlotManager.swift`
`importedWorldMetadata(...)` callees), `AppViewModel+ServerControls.swift`,
`AppViewModel+OutputHandling.swift` (including
`backfillBedrockAllowlistXUIDIfNeeded` and the Bedrock console-to-log-file
mirroring functions), `AppViewModel+ComponentsVersions.swift`,
`AppViewModel+ServerInfo.swift`, `AppViewModel+Backups.swift`, the Bedrock
route handling embedded in `RemoteAPIServer.swift`/`+HTTP`/`+Settings`/
`+ComponentRoutes` (there is no separate `RemoteAPIServer+BedrockRoutes.swift`
— that filename does not exist in the oracle), and their copied iOS route
consumers. `AppViewModel+BedrockPerformance.swift` is a thin UI-facing wrapper
over `VMBedrockServerBackend.swift`'s own `[MSCSTATS]` parsing and needs no
separate extraction. `BedrockSkinFetcher.swift` (player skin/avatar
resolution) is explicitly out of scope — client-presentation logic deferred
to Phase 11, not an agent capability; record the exclusion rather than
dropping it silently. `docs/msc2/sidecar-ipc-contract.md` is the
already-frozen macOS process boundary, not a replacement for reading the
oracle. Phase 5 import records, Phase 6 world/backup behavior (including
`WorldSlotManager.swift`'s Bedrock-specific import-metadata derivation), Phase
9 networking and Geyser/Floodgate boundaries, D-014, and D-021 are mandatory
reconciliation inputs. **D-007 (macOS Bedrock stays Swift behind a sidecar)
and D-022 (separate Bedrock compatibility matrix) are both still status
`Proposed`, not Approved** — this phase's entire native→native→sidecar,
separate-matrix architecture is built on them, so P10.1 must say so plainly to
Cameron rather than proceeding as if they were settled.

**Execution order and batches:** P10.1–P10.6 establish the boundary,
evidence, fixtures, and contract. P10.7–P10.10 are the shared pure and storage
foundation. P10.11–P10.13 build Linux first and end at the first native BDS
runtime. P10.14–P10.15 add Windows and end at the second native runtime.
P10.16–P10.18 add the macOS protocol client and Swift sidecar and end at the
macOS-only boundary. P10.19 through P10.23 (including P10.19a) integrate the
common application, API, CLI, and copied iOS surfaces. P10.24–P10.28 are
proof and gate work. Never continue a batch after a failed Verify.

**Verification budget:** each implementation step runs only the targeted
fixture directory, crate test target, route test, smoke, or client test named
below. The full workspace suite appears only in P10.28 as part of the gate;
P10.27 records exact-candidate CI instead of repeating it locally.

### Scope, evidence, and contract

### P10.1 — Scope Bedrock runtimes and reconcile prior-phase handoffs
**Status:** awaiting verification
**Files:** `docs/msc2/bedrock/phase10-scope.md`, `docs/msc2/audit/msc2-symbol-ledger.csv`, `docs/msc2/rolling-plan.md`
**What:** Read the complete Phase 10 oracle set against the current Rust workspace, Phase 5 Bedrock import records, Phase 6 worlds/backups (including `WorldSlotManager.swift`'s Bedrock import-metadata derivation), Phase 9 networking helpers, and the frozen sidecar contract. Record the exact Linux, Windows, and macOS boundaries; the owned ledger rows; native-versus-sidecar responsibilities; download/provisioning provenance (record plainly that MSC 1's own provisioner performs no checksum or signature verification at all — any verification MSC 2 adds is new, not ported); and every behavior that is new rather than silently presented as an MSC 1 port. Explicitly resolve the port plan's open sequencing question (§6): `UDPRelay.swift` is confirmed VM-specific host↔guest forwarding, not a general Bedrock need — a native Linux/Windows `bedrock_server` binds the host UDP port directly with no relay stage, so `fixtures/bedrock-udp/` holds only VM-relay cases and native UDP-bind cases live in `fixtures/bedrock-runtime/` instead. State plainly that this phase's architecture depends on D-007 and D-022, both still `Proposed`, and flag that to Cameron rather than treating them as settled. Write no Rust.
**Verify:** `python3 -c "from pathlib import Path; s=Path('docs/msc2/bedrock/phase10-scope.md').read_text().lower(); required=['linux','windows','macos','sidecar','leveldb','allowlist','permissions','udp','phase 5','phase 6','phase 9','d-007','d-022','vm-specific']; missing=[x for x in required if x not in s]; assert not missing, missing; print('OK')"`
**Commit:** `P10.1: scope Bedrock runtimes`
**Batch:** solo

### P10.2 — Capture Bedrock files, settings, player, and console fixtures
**Status:** awaiting verification
**Files:** `fixtures/bedrock-properties/`, `fixtures/bedrock-players/`, `fixtures/bedrock-console/`, `fixtures/bedrock-logging/`, `corpus/bedrock/`
**What:** Extract exactly 24 `bedrock-properties` fixtures from `BedrockPropertiesManager.swift` (server.properties, allowlist.json, permissions.json — including the absence of any range clamping/validation, unrecognized enum values being silently ignored rather than rejected, and unknown keys surviving a round-trip write); 22 `bedrock-players` fixtures from `BedrockPlayerDataManager.swift`, `BedrockNameCache.swift`, `BedrockHiddenProfiles.swift`, and `AppViewModel+OutputHandling.swift`'s `backfillBedrockAllowlistXUIDIfNeeded` (the full LevelDB-key classification tree, name-cache and hidden-profile persistence, and the Java-server backfill guard); 16 `bedrock-console` fixtures from `AppViewModel+OutputHandling.swift`/`VMBedrockServerBackend.swift` (the `"Server started"` readiness substring match, version-line parsing, player connect/disconnect including reconnect and empty-gamertag edge cases, the `[MSCSTATS]` line, and the guest-IP discovery line); and 8 `bedrock-logging` fixtures from the Bedrock-specific console-to-`logs/latest.log` mirroring (`startBedrockLogFile`/`appendBedrockLogLine`/`closeBedrockLogFile`/`pruneRolledBedrockLogs` — Bedrock has no log file of its own, so this is a distinct mechanism from Java's rolling logs, including the exact keep-10 rotation boundary). Include malformed and legacy data; do not invent values that cannot be observed from the oracle.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-properties --expect 24 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-players --expect 22 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-console --expect 16 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-logging --expect 8`
**Commit:** `P10.2: capture Bedrock behavior fixtures`
**Batch:** solo

### P10.3 — Capture LevelDB, NBT, world-layout, and backup fixtures
**Status:** awaiting verification
**Files:** `fixtures/bedrock-leveldb/`, `fixtures/bedrock-nbt/`, `fixtures/bedrock-world-layout/`, `fixtures/bedrock-backup/`, `corpus/bedrock/`
**What:** Extract exactly 22 `bedrock-leveldb` fixtures from `BedrockLevelDB.swift` (both block-compression types and an invalid compression byte, footer/truncation rejection, WAL FULL/FIRST-MIDDLE-LAST record reassembly, an unknown WAL record type, varint overflow, and a fixture pinning the oracle's own filesystem-order-dependent `.ldb` conflict resolution — distinct from `.log` files, which are explicitly sorted newest-wins); 32 `bedrock-nbt` fixtures from `BedrockNBTReader.swift` (all three `PlayerStats` dimension branches and XP-formula bands, inventory item field-type variants, enchantment key variants, custom-name handling, and corrupt/truncated/bad-tag parse failures); 10 `bedrock-world-layout` fixtures from `AppViewModel+ServerCreation.swift`'s `resolvedBedrockWorldFolder` (direct `level.dat` hit, one-level-deep single-subdir match, ambiguous zero/multiple-subdir fallback, level-name sanitization, and a symlink-escape case against Phase 3's path safety) plus `WorldSlotManager.swift`'s Bedrock import-metadata derivation; and 10 `bedrock-backup` fixtures from `AppViewModel+Backups.swift`'s Bedrock branch (`save hold`→`save query` polling to ready, send-failure and timeout cases — timeout is explicitly not a failure in the oracle, it proceeds anyway — `save resume`, the console-line-wait race, and the fact that MSC 1 has no live-backup *restore* path for Bedrock at all: Bedrock restore redirects to the slot-based Worlds tab, a real scope boundary P10.19a must preserve, not an oversight). Record unsupported/corrupt inputs explicitly so later code never treats partial data as a valid player or world.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-leveldb --expect 22 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-nbt --expect 32 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-world-layout --expect 10 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-backup --expect 10`
**Commit:** `P10.3: capture Bedrock storage fixtures`
**Batch:** solo

### P10.4 — Capture provisioning, runtime, sidecar, and UDP lifecycle fixtures
**Status:** not started
**Files:** `fixtures/bedrock-provisioning/`, `fixtures/bedrock-runtime/`, `fixtures/bedrock-sidecar/`, `fixtures/bedrock-udp/`, `corpus/bedrock/`
**What:** Extract 10 MSC-1-characterized `bedrock-provisioning` fixtures from `BedrockProvisioner.swift`/`BedrockVersionFetcher.swift` (pinned/newest-release resolution, offline fallback, legacy-marker backfill, no-op/force-reinstall, and the preserved-file exclusion list) plus 6 labeled MSC 2 net-new cases (real checksum verification, per-platform manifest-entry dispatch — MSC 1 always reads the `linux` entry even for its own VM guest — corrupt-archive rejection, and atomic rollback on a failed update); **MSC 1's provisioner performs no checksum or signature verification at all**, so none of the net-new group is a port and the scope note must say so. Extract 6 MSC-1-characterized `bedrock-runtime` fixtures from the real, portable parts of `VMBedrockServerBackend.swift`/`AppViewModel+OutputHandling.swift` (the `"Server started"` readiness match, console framing, the `stop` command name and 20-second graceful-then-forced timeout, and clean-stop-vs-error-stop) plus 8 labeled MSC 2 net-new cases for native process supervision — reusing Phase 3/4's already-proven OS-level process-stats and crash-detection mechanism, not the VM's `[MSCSTATS]` line, which is sidecar-only plumbing — including native Windows process-tree ownership and native UDP port-bind/port-in-use cases (a direct bind, not a relay — see P10.1). Extract 16 `bedrock-sidecar` fixtures directly from `docs/msc2/sidecar-ipc-contract.md`'s ten message/behavior sections (one well-formed round trip each, plus malformed-frame/out-of-order/EOF variants for the six sections where framing failure is meaningfully distinct). Extract 5 `bedrock-udp` fixtures from `UDPRelay.swift` covering only VM-guest relay behavior (per-client-flow isolation, bidirectional pump start, cleanup on cancel, bind failure, and the DHCP-then-relay sequencing dependency) — do not add native UDP-bind cases here; those belong in `bedrock-runtime` per P10.1's resolution of the open UDPRelay sequencing question.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-provisioning --expect 16 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-runtime --expect 14 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-sidecar --expect 16 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-udp --expect 5`
**Commit:** `P10.4: capture Bedrock runtime fixtures`
**Batch:** solo

### P10.5 — Publish Bedrock support and compatibility evidence rules
**Status:** not started
**Files:** `docs/msc2/bedrock/compatibility-matrix.csv`, `docs/msc2/bedrock/evidence/README.md`, `tools/phase10/compatibility-check.py`
**What:** Create the separate D-022 Bedrock compatibility matrix and its checker. It must distinguish agent-host support from BDS runtime support, name each native/sidecar backend, and require each advertised cell to cite reproducible evidence rather than inheriting the Java-server matrix.
**Verify:** `python3 tools/phase10/compatibility-check.py docs/msc2/bedrock/compatibility-matrix.csv`
**Commit:** `P10.5: add Bedrock compatibility evidence rules`
**Batch:** solo

### P10.6 — Freeze the Bedrock API, operation, and capability contract
**Status:** not started
**Files:** `docs/msc2/bedrock/phase10-api.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/api-contract/websocket-v1.json`, `docs/msc2/client-capability-matrix.csv`, `crates/msc-api/tests/phase10_conformance.rs`
**What:** Reconcile the frozen `/v1` baseline with Bedrock creation, lifecycle, settings, players, allowlist, permissions, metrics, logs, version changes, and runtime-unavailable states. Define additive DTO fields, permission categories, operation/cancellation semantics, error/help behavior, and platform capability disclosure before application code exists; do not add a Java-shaped route where a shared route already has a compatible home.
**Verify:** `python3 tools/api-contract-check.py --v1-summary && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && cargo nextest run -p msc-api --test phase10_conformance`
**Commit:** `P10.6: freeze Bedrock runtime contract`
**Batch:** solo

### Shared domain and storage foundation

### P10.7 — Port pure Bedrock settings, console, and player rules
**Status:** not started
**Files:** `crates/msc-domain/src/bedrock.rs`, `crates/msc-domain/src/lib.rs`, `crates/msc-domain/tests/bedrock.rs`, `fixtures/bedrock-properties/`, `fixtures/bedrock-console/`, `fixtures/bedrock-players/`
**What:** Implement the fixture-backed parsing, validation, clamping, command selection, console-line classification, player identity extraction, and display-safe status rules. Keep process control, filesystem mutation, and LevelDB I/O outside `msc-domain`.
**Verify:** `cargo nextest run -p msc-domain --test bedrock`
**Commit:** `P10.7: add Bedrock domain rules`
**Batch:** safe

### P10.8 — Add bounded Bedrock NBT and LevelDB readers
**Status:** not started
**Files:** `crates/msc-infrastructure/src/bedrock_nbt.rs`, `crates/msc-infrastructure/src/bedrock_leveldb.rs`, `crates/msc-infrastructure/tests/bedrock_storage.rs`, `fixtures/bedrock-leveldb/`, `fixtures/bedrock-nbt/`
**What:** Add read-only, bounded adapters for the fixture corpus: decode the MSC 1-required NBT/player fields, tolerate real LevelDB table and WAL layouts, and return explicit unavailable/corrupt outcomes without mutating a live world database. Preserve the existing path-safety and resource bounds.
**Verify:** `cargo nextest run -p msc-infrastructure --test bedrock_storage`
**Commit:** `P10.8: add Bedrock storage readers`
**Batch:** stop-after

### P10.9 — Define the portable runtime and sidecar protocol boundary
**Status:** not started
**Files:** `crates/msc-application/src/bedrock_runtime.rs`, `crates/msc-application/tests/bedrock_runtime.rs`, `docs/msc2/sidecar-ipc-contract.md`, `fixtures/bedrock-runtime/`, `fixtures/bedrock-sidecar/`
**What:** Define one `BedrockRuntime` abstraction and its platform-neutral lifecycle, readiness, console, command, termination, and capability vocabulary. The metrics vocabulary must be backend-agnostic: native backends report it from Phase 3/4's existing OS-level process-stats mechanism, the macOS backend from the sidecar's `[MSCSTATS]` parse — neither format belongs in the shared trait itself. Implement protocol encoding/decoding against the frozen JSON-lines contract with fake transports; do not put macOS VM types or native-process assumptions in the shared interface.
**Verify:** `cargo nextest run -p msc-application --test bedrock_runtime`
**Commit:** `P10.9: define Bedrock runtime boundary`
**Batch:** solo

### P10.10 — Add verified Bedrock distribution staging
**Status:** not started
**Files:** `crates/msc-infrastructure/src/bedrock_distribution.rs`, `crates/msc-application/src/bedrock_provisioning.rs`, `crates/msc-application/tests/bedrock_provisioning.rs`, `fixtures/bedrock-provisioning/`
**What:** Implement the scoped official-BDS acquisition and staging path used by all three runtime backends. This is new MSC 2 behavior, not a port — MSC 1's own provisioner performs no checksum or signature verification at all. Add real checksum/identity verification and correct per-platform manifest-entry selection (MSC 1 always reads the `linux` entry, even for its own VM guest), retain provenance and version selection, preserve the Phase 7-style downgrade backup guard, and leave the prior working installation intact on failure; never make an unverified archive runnable.
**Verify:** `cargo nextest run -p msc-application --test bedrock_provisioning`
**Commit:** `P10.10: add verified Bedrock provisioning`
**Batch:** stop-after

### Native runtimes

### P10.11 — Implement the native Linux Bedrock runtime
**Status:** not started
**Files:** `crates/msc-infrastructure/src/bedrock_native.rs`, `crates/msc-application/src/bedrock_linux.rs`, `crates/msc-application/tests/bedrock_linux.rs`, `fixtures/bedrock-runtime/`
**What:** Make the first concrete `BedrockRuntime` implementation a native Linux BDS process. Reuse the established process supervisor, preserve output framing and graceful-then-forced stop behavior, bind UDP directly to the host port (no relay stage — `UDPRelay` is confirmed VM-guest-specific per P10.1, and a native process never needs it), and expose truthful capability/unavailable results on unsupported hosts.
**Verify:** `cargo nextest run -p msc-application --test bedrock_linux`
**Commit:** `P10.11: add native Linux Bedrock runtime`
**Batch:** solo

### P10.12 — Integrate Linux Bedrock lifecycle, metrics, and logs
**Status:** not started
**Files:** `crates/msc-application/src/bedrock_service.rs`, `crates/msc-application/tests/bedrock_service.rs`, `fixtures/bedrock-console/`, `fixtures/bedrock-logging/`, `fixtures/bedrock-backup/`
**What:** Connect the Linux runtime to server readiness, command delivery, metrics (sourced from Phase 3/4's existing OS-level process-stats mechanism, not the VM-only `[MSCSTATS]` protocol), player events, rolling Bedrock logs, save-hold backup coordination, restart recovery, and operation journal state. The service must report a crash separately from a clean stop and must bound retained console and log state under D-021.
**Verify:** `cargo nextest run -p msc-application --test bedrock_service`
**Commit:** `P10.12: integrate Bedrock lifecycle service`
**Batch:** safe

### P10.13 — Exercise the Linux native runtime through the public contract
**Status:** not started
**Files:** `crates/msc-agent/tests/bedrock_linux_routes.rs`, `crates/msc-cli/tests/bedrock_linux.rs`, `tools/phase10/linux-smoke.sh`, `docs/msc2/bedrock/evidence/`
**What:** Drive the Linux runtime from HTTP and CLI through a disposable or fake BDS boundary, covering provision, start, status, command, stop, metrics, and explicit runtime unavailability. Record only reproducible evidence; do not use a real account, private world, or unrestricted public network access.
**Verify:** `bash tools/phase10/linux-smoke.sh --synthetic`
**Commit:** `P10.13: prove Linux Bedrock public path`
**Batch:** stop-after

### P10.14 — Implement the native Windows Bedrock runtime
**Status:** not started
**Files:** `crates/msc-application/src/bedrock_windows.rs`, `crates/msc-application/tests/bedrock_windows.rs`, `crates/msc-infrastructure/tests/bedrock_native_windows.rs`, `fixtures/bedrock-runtime/`
**What:** Add the second concrete `BedrockRuntime` as a native Windows BDS process, using the shared interface unchanged. Prove Windows process-tree ownership, path and file-lock behavior, direct UDP port binding (no relay stage, same as Linux), output framing, stop escalation, and service-session survival without adding Linux-only assumptions.
**Verify:** `cargo nextest run -p msc-application --test bedrock_windows && cargo nextest run -p msc-infrastructure --test bedrock_native_windows`
**Commit:** `P10.14: add native Windows Bedrock runtime`
**Batch:** solo

### P10.15 — Exercise the Windows native runtime through the public contract
**Status:** not started
**Files:** `crates/msc-agent/tests/bedrock_windows_routes.rs`, `crates/msc-cli/tests/bedrock_windows.rs`, `tools/phase10/windows-smoke.ps1`, `docs/msc2/bedrock/evidence/`
**What:** Exercise the same public lifecycle and unavailable-state contract on Windows, including a service-owned server surviving client exit and a failure that leaves no orphaned BDS process. Keep the smoke reproducible and separate an unavailable real BDS package from a passing fake-runtime test.
**Verify:** `pwsh -File tools/phase10/windows-smoke.ps1 -Synthetic`
**Commit:** `P10.15: prove Windows Bedrock public path`
**Batch:** stop-after

### macOS sidecar runtime

### P10.16 — Implement the Rust macOS sidecar runtime client
**Status:** not started
**Files:** `crates/msc-infrastructure/src/bedrock_sidecar.rs`, `crates/msc-application/src/bedrock_macos.rs`, `crates/msc-application/tests/bedrock_macos.rs`, `fixtures/bedrock-sidecar/`
**What:** Implement the macOS `BedrockRuntime` client over the frozen stdio JSON-lines protocol. It supervises the sidecar, validates message order and IDs, translates EOF and malformed frames into bounded failure states, and never embeds VZ-specific behavior in Rust.
**Verify:** `cargo nextest run -p msc-application --test bedrock_macos`
**Commit:** `P10.16: add macOS Bedrock sidecar client`
**Batch:** solo

### P10.17 — Build the Swift Virtualization sidecar
**Status:** not started
**Files:** `sidecar/bedrock/`, `sidecar/bedrock/Tests/`, `fixtures/bedrock-sidecar/`, `docs/msc2/bedrock/phase10-scope.md`
**What:** Build the narrow macOS Swift executable that owns `Virtualization.framework` and implements exactly the frozen provision/start/ready/command/stop/force-stop/terminated/console protocol. It may share the server directory through virtio-fs but may not introduce a second management API or persist Bedrock state outside that directory.
**Verify:** `xcodebuild -project sidecar/bedrock/BedrockSidecar.xcodeproj -scheme BedrockSidecar test`
**Commit:** `P10.17: add macOS Bedrock sidecar`
**Batch:** solo

### P10.18 — Prove the macOS VM and UDP relay lifecycle
**Status:** not started
**Files:** `crates/msc-agent/tests/bedrock_macos_routes.rs`, `tools/phase10/macos-smoke.sh`, `docs/msc2/bedrock/evidence/`, `fixtures/bedrock-udp/`
**What:** Run the complete agent-to-sidecar lifecycle against a disposable VM appliance: readiness only after DHCP and relay setup, console/command framing, graceful and forced shutdown, sidecar crash recovery, and host-directory persistence across a fresh VM. Record hardware or virtualization unavailability honestly rather than claiming a native-macOS BDS runtime.
**Verify:** `bash tools/phase10/macos-smoke.sh --synthetic`
**Commit:** `P10.18: prove macOS Bedrock sidecar lifecycle`
**Batch:** stop-after

### Shared application and public surfaces

### P10.19 — Reconcile Bedrock imports and creation
**Status:** not started
**Files:** `crates/msc-application/src/bedrock_service.rs`, `crates/msc-application/src/provisioning.rs`, `crates/msc-application/tests/bedrock_imports.rs`, `fixtures/bedrock-world-layout/`
**What:** Make Phase 5 imported Bedrock records authoritative only after their real BDS directory, level name, settings, and lifecycle implications are reconciled against the running-host truth. Implement Bedrock create/import with transactional rollback; clearly report any imported record that cannot run on its host rather than presenting it as ready.
**Verify:** `cargo nextest run -p msc-application --test bedrock_imports`
**Commit:** `P10.19: reconcile Bedrock imports and creation`
**Batch:** solo

### P10.19a — Implement Bedrock world and backup operations
**Status:** not started
**Files:** `crates/msc-application/src/worlds.rs`, `crates/msc-application/tests/bedrock_world_backup.rs`, `fixtures/bedrock-backup/`
**What:** Make Phase 6 world-slot data authoritative for Bedrock's flat `worlds/<level-name>/` layout, and implement Bedrock backup with save-hold/save-query coordination when running and transactional rollback on failure. Preserve MSC 1's own scope boundary: Bedrock has no live-backup restore path — restore always goes through the slot-based Worlds model, never a direct in-place live restore the way Java's does. Do not invent a Bedrock live-restore path MSC 1 never had.
**Verify:** `cargo nextest run -p msc-application --test bedrock_world_backup`
**Commit:** `P10.19a: implement Bedrock world and backup operations`
**Batch:** solo

### P10.20 — Add Bedrock players, allowlist, permissions, and settings services
**Status:** not started
**Files:** `crates/msc-application/src/bedrock_players.rs`, `crates/msc-application/src/bedrock_settings.rs`, `crates/msc-application/tests/bedrock_players.rs`, `fixtures/bedrock-properties/`, `fixtures/bedrock-players/`, `fixtures/bedrock-leveldb/`
**What:** Provide the shared services for Bedrock player discovery, XUID/name cache updates, allowlist and permissions mutation, live reload where supported, and validated `server.properties` changes. Every write must use the substrate’s atomic path and preserve a valid prior file if validation or replacement fails.
**Verify:** `cargo nextest run -p msc-application --test bedrock_players`
**Commit:** `P10.20: add Bedrock player and settings services`
**Batch:** safe

### P10.21 — Wire Bedrock HTTP, WebSocket, and CLI behavior
**Status:** not started
**Files:** `crates/msc-agent/src/routes/bedrock.rs`, `crates/msc-agent/tests/bedrock_routes.rs`, `crates/msc-cli/src/commands/bedrock.rs`, `crates/msc-cli/tests/bedrock.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement the P10.6 contract through the public agent routes and scriptable CLI, including operations, cancellation, capability disclosure, status/metrics/logs, settings, and player/allowlist actions. Reuse existing shared lifecycle routes where specified and ensure unsupported backend features are explicit, permission-checked results rather than absent or Java-only behavior.
**Verify:** `cargo nextest run -p msc-agent --test bedrock_routes && cargo nextest run -p msc-cli --test bedrock`
**Commit:** `P10.21: expose Bedrock public contract`
**Batch:** stop-after

### P10.22 — Update the copied iOS Bedrock client contract
**Status:** not started
**Files:** `clients/ios/MSCRemoteiOS_Swift/`, `clients/ios/MSCRemoteiOSTests/`, `docs/msc2/client-capability-matrix.csv`, `tools/phase10/ios-contract-check.py`
**What:** Update the copied iOS client’s Bedrock DTO decoding and supported lifecycle/settings/player flows against the P10.6 public contract, while keeping presentation-specific behavior client-owned. The MSC 1 oracle remains read-only. Record each delivered or intentionally unavailable capability in the matrix; do not claim a desktop/web surface before Phase 11.
**Verify:** `python3 tools/phase10/ios-contract-check.py`
**Commit:** `P10.22: update copied iOS Bedrock contract`
**Batch:** solo

### P10.23 — Add one synthetic cross-backend Bedrock smoke
**Status:** not started
**Files:** `tools/phase10/phase10-smoke.sh`, `crates/msc-agent/tests/bedrock_routes.rs`, `crates/msc-cli/tests/bedrock.rs`, `docs/msc2/bedrock/evidence/`
**What:** Add one offline public-path smoke that runs the same fixture-backed API and CLI workflow against Linux-native, Windows-native, and macOS-sidecar fakes. It must cover provision, lifecycle, console, command, player/settings state, cancellation/recovery, runtime-unavailable disclosure, and ensure no test needs a live provider or personal world.
**Verify:** `bash tools/phase10/phase10-smoke.sh --synthetic`
**Commit:** `P10.23: add Bedrock cross-backend smoke`
**Batch:** stop-after

### Evidence and gate

### P10.24 — Record safe official-distribution evidence
**Status:** not started
**Files:** `docs/msc2/bedrock/evidence/`, `tools/phase10/evidence-check.py`, `docs/msc2/bedrock/compatibility-matrix.csv`
**What:** Record the reproducible official-distribution and package-identity evidence each runtime needs, or a precise unavailable result where licensing, host support, or safe access prevents it. The checker must reject fabricated success and must link every supported matrix cell to its matching record.
**Verify:** `python3 tools/phase10/evidence-check.py --distribution`
**Commit:** `P10.24: record Bedrock distribution evidence`
**Batch:** solo

### P10.25 — Record native and sidecar runtime evidence
**Status:** not started
**Files:** `docs/msc2/bedrock/evidence/`, `docs/msc2/bedrock/compatibility-matrix.csv`, `tools/phase10/evidence-check.py`
**What:** Record Linux-native, Windows-native, and macOS-sidecar lifecycle evidence using the same terms as the capability matrix: supported, unsupported, or unavailable. Include UDP reachability and clean/crash termination where a safe disposable environment exists; retain unavailable outcomes rather than replacing them with claims from a fake runtime.
**Verify:** `python3 tools/phase10/evidence-check.py --runtimes && python3 tools/phase10/compatibility-check.py docs/msc2/bedrock/compatibility-matrix.csv`
**Commit:** `P10.25: record Bedrock runtime evidence`
**Batch:** stop-after

### P10.26 — Run Phase 10 synthetic checks in tri-platform CI
**Status:** not started
**Files:** `.github/workflows/ci.yml`, `tools/phase10/phase10-smoke.sh`, `tools/phase10/compatibility-check.py`, `tools/phase10/evidence-check.py`
**What:** Extend the existing macOS/Linux/Windows jobs with the offline Phase 10 smoke and documentary checks, while preserving the headless no-GUI link proof. CI must use fakes and fixtures only; it must not download BDS, require a Mojang account, start a VM, or make public-network calls.
**Verify:** `git diff --check && rg -n 'phase10-smoke.sh --synthetic|phase10/compatibility-check.py|phase10/evidence-check.py' .github/workflows/ci.yml`
**Commit:** `P10.26: run Bedrock checks in CI`
**Batch:** solo

### P10.27 — Record the exact tri-platform Phase 10 candidate
**Status:** not started
**Files:** `docs/msc2/bedrock/evidence/phase10-ci.md`, `docs/msc2/rolling-plan.md`
**What:** Record the exact commit and green macOS/Linux/Windows/headless CI run that exercised P10.26’s candidate. Tie the evidence to that commit only; do not substitute a later documentation commit or a partial local run.
**Verify:** `test -s docs/msc2/bedrock/evidence/phase10-ci.md && rg -n 'commit|macOS|Linux|Windows|headless' docs/msc2/bedrock/evidence/phase10-ci.md`
**Commit:** `P10.27: record Phase 10 CI candidate`
**Batch:** stop-after

### P10.28 — Run and record the exact Phase 10 gate
**Status:** not started
**Files:** `tools/phase10/phase10-check.py`, `docs/msc2/bedrock/phase10-scope.md`, `docs/msc2/rolling-plan.md`
**What:** Check the literal Phase 10 gate against the shared fixtures, public API/CLI/iOS contract, separate compatibility matrix, real-or-unavailable runtime evidence, synthetic smoke, and exact CI candidate. This is the phase’s only full-workspace test run. It reports evidence only; the other agent decides in REVIEW whether the gate holds.
**Verify:** `python3 tools/phase10/phase10-check.py --gate && bash tools/phase10/phase10-smoke.sh --synthetic && cargo nextest run --workspace`
**Commit:** `P10.28: check Bedrock runtime gate`
**Batch:** solo
