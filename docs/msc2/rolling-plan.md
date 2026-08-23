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
Virtualization.framework, verified on **Intel Macs only** — Apple Silicon is
explicitly deferred per D-028 (MSC 1's VM appliance is a single-architecture,
x86_64-only build, and the owner has no Apple Silicon hardware to build or
verify the new arm64-appliance-plus-Rosetta-for-Linux path that would be
needed) and must be recorded as unavailable, never silently omitted or
claimed. Console output, readiness, metrics, rolling logs, players, allowlist,
permissions, settings, worlds, and backups behave against the shared fixture
corpus. No client claims Bedrock support on a platform or version that the
published compatibility evidence does not prove.

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
and D-022 (separate Bedrock compatibility matrix) were both confirmed Approved
by Cameron on 2026-08-22**, after reviewing the concrete Rust-bridge
alternative and its risks; this phase's native→native→sidecar, separate-matrix
architecture proceeds on that basis. **D-028 (new, 2026-08-22)** scopes the
macOS backend to Intel Macs only for this phase — MSC 1's VM appliance is a
single-architecture x86_64 build with no Rosetta-for-Linux support, and
Apple Silicon has no owner-available test hardware, so it is deferred and
recorded as unavailable rather than attempted or silently dropped.

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
**Status:** DONE
**Files:** `docs/msc2/bedrock/phase10-scope.md`, `docs/msc2/audit/msc2-symbol-ledger.csv`, `docs/msc2/rolling-plan.md`
**What:** Read the complete Phase 10 oracle set against the current Rust workspace, Phase 5 Bedrock import records, Phase 6 worlds/backups (including `WorldSlotManager.swift`'s Bedrock import-metadata derivation), Phase 9 networking helpers, and the frozen sidecar contract. Record the exact Linux, Windows, and macOS boundaries; the owned ledger rows; native-versus-sidecar responsibilities; download/provisioning provenance (record plainly that MSC 1's own provisioner performs no checksum or signature verification at all — any verification MSC 2 adds is new, not ported); and every behavior that is new rather than silently presented as an MSC 1 port. Explicitly resolve the port plan's open sequencing question (§6): `UDPRelay.swift` is confirmed VM-specific host↔guest forwarding, not a general Bedrock need — a native Linux/Windows `bedrock_server` binds the host UDP port directly with no relay stage, so `fixtures/bedrock-udp/` holds only VM-relay cases and native UDP-bind cases live in `fixtures/bedrock-runtime/` instead. Record that D-007 and D-022 are Approved (2026-08-22) and that D-028 scopes the macOS backend to Intel Macs only — MSC 1's single-architecture x86_64 appliance cannot boot on Apple Silicon at all (`Virtualization.framework` does not emulate a foreign CPU architecture), and no arm64 appliance or Rosetta-for-Linux wiring exists to port; record Apple Silicon as unavailable, not attempted this phase. Write no Rust.
**Verify:** `python3 -c "from pathlib import Path; s=Path('docs/msc2/bedrock/phase10-scope.md').read_text().lower(); required=['linux','windows','macos','sidecar','leveldb','allowlist','permissions','udp','phase 5','phase 6','phase 9','d-007','d-022','d-028','apple silicon','intel']; missing=[x for x in required if x not in s]; assert not missing, missing; print('OK')"`
**Commit:** `P10.1: scope Bedrock runtimes`
**Batch:** solo

### P10.2 — Capture Bedrock files, settings, player, and console fixtures
**Status:** DONE
**Files:** `fixtures/bedrock-properties/`, `fixtures/bedrock-players/`, `fixtures/bedrock-console/`, `fixtures/bedrock-logging/`, `corpus/bedrock/`
**What:** Extract exactly 24 `bedrock-properties` fixtures from `BedrockPropertiesManager.swift` (server.properties, allowlist.json, permissions.json — including the absence of any range clamping/validation, unrecognized enum values being silently ignored rather than rejected, and unknown keys surviving a round-trip write); 22 `bedrock-players` fixtures from `BedrockPlayerDataManager.swift`, `BedrockNameCache.swift`, `BedrockHiddenProfiles.swift`, and `AppViewModel+OutputHandling.swift`'s `backfillBedrockAllowlistXUIDIfNeeded` (the full LevelDB-key classification tree, name-cache and hidden-profile persistence, and the Java-server backfill guard); 16 `bedrock-console` fixtures from `AppViewModel+OutputHandling.swift`/`VMBedrockServerBackend.swift` (the `"Server started"` readiness substring match, version-line parsing, player connect/disconnect including reconnect and empty-gamertag edge cases, the `[MSCSTATS]` line, and the guest-IP discovery line); and 8 `bedrock-logging` fixtures from the Bedrock-specific console-to-`logs/latest.log` mirroring (`startBedrockLogFile`/`appendBedrockLogLine`/`closeBedrockLogFile`/`pruneRolledBedrockLogs` — Bedrock has no log file of its own, so this is a distinct mechanism from Java's rolling logs, including the exact keep-10 rotation boundary). Include malformed and legacy data; do not invent values that cannot be observed from the oracle.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-properties --expect 24 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-players --expect 22 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-console --expect 16 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-logging --expect 8`
**Commit:** `P10.2: capture Bedrock behavior fixtures`
**Batch:** solo

### P10.3 — Capture LevelDB, NBT, world-layout, and backup fixtures
**Status:** DONE
**Files:** `fixtures/bedrock-leveldb/`, `fixtures/bedrock-nbt/`, `fixtures/bedrock-world-layout/`, `fixtures/bedrock-backup/`, `corpus/bedrock/`
**What:** Extract exactly 22 `bedrock-leveldb` fixtures from `BedrockLevelDB.swift` (both block-compression types and an invalid compression byte, footer/truncation rejection, WAL FULL/FIRST-MIDDLE-LAST record reassembly, an unknown WAL record type, varint overflow, and a fixture pinning the oracle's own filesystem-order-dependent `.ldb` conflict resolution — distinct from `.log` files, which are explicitly sorted newest-wins); 32 `bedrock-nbt` fixtures from `BedrockNBTReader.swift` (all three `PlayerStats` dimension branches and XP-formula bands, inventory item field-type variants, enchantment key variants, custom-name handling, and corrupt/truncated/bad-tag parse failures); 10 `bedrock-world-layout` fixtures from `AppViewModel+ServerCreation.swift`'s `resolvedBedrockWorldFolder` (direct `level.dat` hit, one-level-deep single-subdir match, ambiguous zero/multiple-subdir fallback, level-name sanitization, and a symlink-escape case against Phase 3's path safety) plus `WorldSlotManager.swift`'s Bedrock import-metadata derivation; and 10 `bedrock-backup` fixtures from `AppViewModel+Backups.swift`'s Bedrock branch (`save hold`→`save query` polling to ready, send-failure and timeout cases — timeout is explicitly not a failure in the oracle, it proceeds anyway — `save resume`, the console-line-wait race, and the fact that MSC 1 has no live-backup *restore* path for Bedrock at all: Bedrock restore redirects to the slot-based Worlds tab, a real scope boundary P10.19a must preserve, not an oversight). Record unsupported/corrupt inputs explicitly so later code never treats partial data as a valid player or world.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-leveldb --expect 22 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-nbt --expect 32 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-world-layout --expect 10 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-backup --expect 10`
**Commit:** `P10.3: capture Bedrock storage fixtures`
**Batch:** solo

### P10.4 — Capture provisioning, runtime, sidecar, and UDP lifecycle fixtures
**Status:** DONE
**Files:** `fixtures/bedrock-provisioning/`, `fixtures/bedrock-runtime/`, `fixtures/bedrock-sidecar/`, `fixtures/bedrock-udp/`, `corpus/bedrock/`
**What:** Extract 10 MSC-1-characterized `bedrock-provisioning` fixtures from `BedrockProvisioner.swift`/`BedrockVersionFetcher.swift` (pinned/newest-release resolution, offline fallback, legacy-marker backfill, no-op/force-reinstall, and the preserved-file exclusion list) plus 6 labeled MSC 2 net-new cases (real checksum verification, per-platform manifest-entry dispatch — MSC 1 always reads the `linux` entry even for its own VM guest — corrupt-archive rejection, and atomic rollback on a failed update); **MSC 1's provisioner performs no checksum or signature verification at all**, so none of the net-new group is a port and the scope note must say so. Extract 6 MSC-1-characterized `bedrock-runtime` fixtures from the real, portable parts of `VMBedrockServerBackend.swift`/`AppViewModel+OutputHandling.swift` (the `"Server started"` readiness match, console framing, the `stop` command name and 20-second graceful-then-forced timeout, and clean-stop-vs-error-stop) plus 8 labeled MSC 2 net-new cases for native process supervision — reusing Phase 3/4's already-proven OS-level process-stats and crash-detection mechanism, not the VM's `[MSCSTATS]` line, which is sidecar-only plumbing — including native Windows process-tree ownership and native UDP port-bind/port-in-use cases (a direct bind, not a relay — see P10.1). Extract 16 `bedrock-sidecar` fixtures directly from `docs/msc2/sidecar-ipc-contract.md`'s ten message/behavior sections (one well-formed round trip each, plus malformed-frame/out-of-order/EOF variants for the six sections where framing failure is meaningfully distinct). Extract 5 `bedrock-udp` fixtures from `UDPRelay.swift` covering only VM-guest relay behavior (per-client-flow isolation, bidirectional pump start, cleanup on cancel, bind failure, and the DHCP-then-relay sequencing dependency) — do not add native UDP-bind cases here; those belong in `bedrock-runtime` per P10.1's resolution of the open UDPRelay sequencing question.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-provisioning --expect 16 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-runtime --expect 14 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-sidecar --expect 16 && python3 tools/fixture-runner/run.py --validate-dir fixtures/bedrock-udp --expect 5`
**Commit:** `P10.4: capture Bedrock runtime fixtures`
**Batch:** solo

### P10.5 — Publish Bedrock support and compatibility evidence rules
**Status:** DONE
**Files:** `docs/msc2/bedrock/compatibility-matrix.csv`, `docs/msc2/bedrock/evidence/README.md`, `tools/phase10/compatibility-check.py`
**What:** Create the separate D-022 Bedrock compatibility matrix and its checker. It must distinguish agent-host support from BDS runtime support, name each native/sidecar backend, and require each advertised cell to cite reproducible evidence rather than inheriting the Java-server matrix. Per D-028, the checker must require a distinct Apple Silicon Mac row/cell whose status is exactly `unavailable` with a reason citing no test hardware — never merged with the Intel Mac cell, never `unsupported` (that would claim a tested negative this project never tested), and never silently absent from the matrix.
**Verify:** `python3 tools/phase10/compatibility-check.py docs/msc2/bedrock/compatibility-matrix.csv --require-cell "macOS (Apple Silicon)=unavailable"`
**Commit:** `P10.5: add Bedrock compatibility evidence rules`
**Batch:** solo

### P10.6 — Freeze the Bedrock API, operation, and capability contract
**Status:** DONE
**Files:** `docs/msc2/bedrock/phase10-api.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/api-contract/websocket-v1.json`, `docs/msc2/client-capability-matrix.csv`, `crates/msc-api/tests/phase10_conformance.rs`
**What:** Reconcile the frozen `/v1` baseline with Bedrock creation, lifecycle, settings, players, allowlist, permissions, metrics, logs, version changes, and runtime-unavailable states. Define additive DTO fields, permission categories, operation/cancellation semantics, error/help behavior, and platform capability disclosure before application code exists; do not add a Java-shaped route where a shared route already has a compatible home.
**Verify:** `python3 tools/api-contract-check.py --v1-summary && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && cargo nextest run -p msc-api --test phase10_conformance`
**Commit:** `P10.6: freeze Bedrock runtime contract`
**Batch:** solo

### Shared domain and storage foundation

### P10.7 — Port pure Bedrock settings, console, and player rules
**Status:** DONE
**Files:** `crates/msc-domain/src/bedrock.rs`, `crates/msc-domain/src/lib.rs`, `crates/msc-domain/tests/bedrock.rs`, `fixtures/bedrock-properties/`, `fixtures/bedrock-console/`, `fixtures/bedrock-players/`
**What:** Implement the fixture-backed parsing, validation, clamping, command selection, console-line classification, player identity extraction, and display-safe status rules. Keep process control, filesystem mutation, and LevelDB I/O outside `msc-domain`.
**Verify:** `cargo nextest run -p msc-domain --test bedrock`
**Commit:** `P10.7: add Bedrock domain rules`
**Batch:** safe

### P10.8 — Add bounded Bedrock NBT and LevelDB readers
**Status:** DONE
**Files:** `crates/msc-infrastructure/src/bedrock_nbt.rs`, `crates/msc-infrastructure/src/bedrock_leveldb.rs`, `crates/msc-infrastructure/tests/bedrock_storage.rs`, `fixtures/bedrock-leveldb/`, `fixtures/bedrock-nbt/`
**What:** Add read-only, bounded adapters for the fixture corpus: decode the MSC 1-required NBT/player fields, tolerate real LevelDB table and WAL layouts, and return explicit unavailable/corrupt outcomes without mutating a live world database. Preserve the existing path-safety and resource bounds.
**Verify:** `cargo nextest run -p msc-infrastructure --test bedrock_storage`
**Commit:** `P10.8: add Bedrock storage readers`
**Batch:** stop-after

### P10.9 — Define the portable runtime and sidecar protocol boundary
**Status:** DONE
**Files:** `crates/msc-application/src/bedrock_runtime.rs`, `crates/msc-application/tests/bedrock_runtime.rs`, `docs/msc2/sidecar-ipc-contract.md`, `fixtures/bedrock-runtime/`, `fixtures/bedrock-sidecar/`
**What:** Define one `BedrockRuntime` abstraction and its platform-neutral lifecycle, readiness, console, command, termination, and capability vocabulary. The metrics vocabulary must be backend-agnostic: native backends report it from Phase 3/4's existing OS-level process-stats mechanism, the macOS backend from the sidecar's `[MSCSTATS]` parse — neither format belongs in the shared trait itself. Implement protocol encoding/decoding against the frozen JSON-lines contract with fake transports; do not put macOS VM types or native-process assumptions in the shared interface.
**Verify:** `cargo nextest run -p msc-application --test bedrock_runtime`
**Commit:** `P10.9: define Bedrock runtime boundary`
**Batch:** solo

### P10.10 — Add verified Bedrock distribution staging
**Status:** DONE
**Files:** `crates/msc-infrastructure/src/bedrock_distribution.rs`, `crates/msc-application/src/bedrock_provisioning.rs`, `crates/msc-application/tests/bedrock_provisioning.rs`, `fixtures/bedrock-provisioning/`
**What:** Implement the scoped official-BDS acquisition and staging path used by all three runtime backends. This is new MSC 2 behavior, not a port — MSC 1's own provisioner performs no checksum or signature verification at all. Add real checksum/identity verification and correct per-platform manifest-entry selection (MSC 1 always reads the `linux` entry, even for its own VM guest), retain provenance and version selection, preserve the Phase 7-style downgrade backup guard, and leave the prior working installation intact on failure; never make an unverified archive runnable.
**Verify:** `cargo nextest run -p msc-application --test bedrock_provisioning`
**Commit:** `P10.10: add verified Bedrock provisioning`
**Batch:** stop-after

### Native runtimes

### P10.11 — Implement the native Linux Bedrock runtime
**Status:** DONE
**Files:** `crates/msc-infrastructure/src/bedrock_native.rs`, `crates/msc-application/src/bedrock_linux.rs`, `crates/msc-application/tests/bedrock_linux.rs`, `fixtures/bedrock-runtime/`
**What:** Make the first concrete `BedrockRuntime` implementation a native Linux BDS process. Reuse the established process supervisor, preserve output framing and graceful-then-forced stop behavior, bind UDP directly to the host port (no relay stage — `UDPRelay` is confirmed VM-guest-specific per P10.1, and a native process never needs it), and expose truthful capability/unavailable results on unsupported hosts.
**Verify:** `cargo nextest run -p msc-application --test bedrock_linux`
**Commit:** `P10.11: add native Linux Bedrock runtime`
**Batch:** solo

### P10.12 — Integrate Linux Bedrock lifecycle, metrics, and logs
**Status:** DONE
**Files:** `crates/msc-application/src/bedrock_service.rs`, `crates/msc-application/tests/bedrock_service.rs`, `fixtures/bedrock-console/`, `fixtures/bedrock-logging/`, `fixtures/bedrock-backup/`
**What:** Connect the Linux runtime to server readiness, command delivery, metrics (sourced from Phase 3/4's existing OS-level process-stats mechanism, not the VM-only `[MSCSTATS]` protocol), player events, rolling Bedrock logs, save-hold backup coordination, restart recovery, and operation journal state. The service must report a crash separately from a clean stop and must bound retained console and log state under D-021.
**Verify:** `cargo nextest run -p msc-application --test bedrock_service`
**Commit:** `P10.12: integrate Bedrock lifecycle service`
**Batch:** safe

### P10.13 — Exercise the Linux native runtime through the public contract
**Status:** DONE
**Files:** `crates/msc-agent/tests/bedrock_linux_routes.rs`, `crates/msc-agent/tests/bedrock_linux_cli.rs`, `tools/phase10/linux-smoke.sh`, `docs/msc2/bedrock/evidence/`
**What:** Drive the Linux runtime from HTTP and CLI through a disposable or fake BDS boundary, covering provision, start, status, command, stop, metrics, and explicit runtime unavailability. Record only reproducible evidence; do not use a real account, private world, or unrestricted public network access.
**Verify:** `bash tools/phase10/linux-smoke.sh --synthetic`
**Commit:** `P10.13: prove Linux Bedrock public path`
**Batch:** stop-after

### P10.14 — Implement the native Windows Bedrock runtime
**Status:** DONE
**Files:** `crates/msc-application/src/bedrock_windows.rs`, `crates/msc-application/tests/bedrock_windows.rs`, `crates/msc-infrastructure/src/bedrock_native.rs`, `crates/msc-infrastructure/tests/bedrock_native_windows.rs`, `fixtures/bedrock-runtime/`
**What:** Add the second concrete `BedrockRuntime` as a native Windows BDS process, using the shared interface unchanged. Prove Windows process-tree ownership, path and file-lock behavior, direct UDP port binding (no relay stage, same as Linux), output framing, stop escalation, and service-session survival without adding Linux-only assumptions.
**Verify:** `cargo nextest run -p msc-application --test bedrock_windows && cargo nextest run -p msc-infrastructure --test bedrock_native_windows`
**Commit:** `P10.14: add native Windows Bedrock runtime`
**Batch:** solo

### P10.15 — Exercise the Windows native runtime through the public contract
**Status:** DONE
**Files:** `crates/msc-agent/tests/bedrock_windows_routes.rs`, `crates/msc-agent/tests/bedrock_windows_cli.rs`, `tools/phase10/windows-smoke.ps1`, `docs/msc2/bedrock/evidence/`
**What:** Exercise the same public lifecycle and unavailable-state contract on Windows, including a service-owned server surviving client exit and a failure that leaves no orphaned BDS process. Keep the smoke reproducible and separate an unavailable real BDS package from a passing fake-runtime test.
**Verify:** `pwsh -File tools/phase10/windows-smoke.ps1 -Synthetic`
**Commit:** `P10.15: prove Windows Bedrock public path`
**Batch:** stop-after

### macOS sidecar runtime

### P10.16 — Implement the Rust macOS sidecar runtime client
**Status:** DONE
**Files:** `crates/msc-infrastructure/src/bedrock_sidecar.rs`, `crates/msc-application/src/bedrock_macos.rs`, `crates/msc-application/tests/bedrock_macos.rs`, `fixtures/bedrock-sidecar/`
**What:** Implement the macOS `BedrockRuntime` client over the frozen stdio JSON-lines protocol. It supervises the sidecar, validates message order and IDs, translates EOF and malformed frames into bounded failure states, and never embeds VZ-specific behavior in Rust.
**Verify:** `cargo nextest run -p msc-application --test bedrock_macos`
**Commit:** `P10.16: add macOS Bedrock sidecar client`
**Batch:** solo

### P10.17 — Build the Swift Virtualization sidecar
**Status:** DONE
**Files:** `sidecar/bedrock/`, `sidecar/bedrock/Tests/`, `fixtures/bedrock-sidecar/`, `docs/msc2/bedrock/phase10-scope.md`
**What:** Build the narrow macOS Swift executable that owns `Virtualization.framework` and implements exactly the frozen provision/start/ready/command/stop/force-stop/terminated/console protocol. It may share the server directory through virtio-fs but may not introduce a second management API or persist Bedrock state outside that directory. Per D-028, the bundled kernel/initramfs appliance is Intel (x86_64) only, matching MSC 1's own single-architecture build; do not attempt an arm64 appliance or Rosetta-for-Linux wiring this phase, and make the host-architecture requirement an explicit, checked precondition rather than an unexplained failure to boot on Apple Silicon.
**Verify:** `xcodebuild -project sidecar/bedrock/BedrockSidecar.xcodeproj -scheme BedrockSidecar test`
**Commit:** `P10.17: add macOS Bedrock sidecar`
**Batch:** solo

### P10.18 — Prove the macOS VM and UDP relay lifecycle
**Status:** DONE
**Files:** `crates/msc-agent/tests/bedrock_macos_routes.rs`, `tools/phase10/macos-smoke.sh`, `docs/msc2/bedrock/evidence/`, `fixtures/bedrock-udp/`
**What:** Run the complete agent-to-sidecar lifecycle against a disposable VM appliance on an Intel Mac: readiness only after DHCP and relay setup, console/command framing, graceful and forced shutdown, sidecar crash recovery, and host-directory persistence across a fresh VM. Record hardware or virtualization unavailability honestly rather than claiming a native-macOS BDS runtime, and record Apple Silicon specifically as out of scope per D-028 rather than untested-and-unmentioned.
**Verify:** `bash tools/phase10/macos-smoke.sh --synthetic`
**Commit:** `P10.18: prove macOS Bedrock sidecar lifecycle`
**Batch:** stop-after

### Shared application and public surfaces

### P10.19 — Reconcile Bedrock imports and creation
**Status:** DONE
**Files:** `crates/msc-application/src/bedrock_service.rs`, `crates/msc-application/src/provisioning.rs`, `crates/msc-application/tests/bedrock_imports.rs`, `fixtures/bedrock-world-layout/`
**What:** Make Phase 5 imported Bedrock records authoritative only after their real BDS directory, level name, settings, and lifecycle implications are reconciled against the running-host truth. Implement Bedrock create/import with transactional rollback; clearly report any imported record that cannot run on its host rather than presenting it as ready.
**Verify:** `cargo nextest run -p msc-application --test bedrock_imports`
**Commit:** `P10.19: reconcile Bedrock imports and creation`
**Batch:** solo

### P10.19a — Implement Bedrock world and backup operations
**Status:** DONE
**Files:** `crates/msc-application/src/worlds.rs`, `crates/msc-application/tests/bedrock_world_backup.rs`, `fixtures/bedrock-backup/`
**What:** Make Phase 6 world-slot data authoritative for Bedrock's flat `worlds/<level-name>/` layout, and implement Bedrock backup with save-hold/save-query coordination when running and transactional rollback on failure. Preserve MSC 1's own scope boundary: Bedrock has no live-backup restore path — restore always goes through the slot-based Worlds model, never a direct in-place live restore the way Java's does. Do not invent a Bedrock live-restore path MSC 1 never had.
**Verify:** `cargo nextest run -p msc-application --test bedrock_world_backup`
**Commit:** `P10.19a: implement Bedrock world and backup operations`
**Batch:** solo

### P10.20 — Add Bedrock players, allowlist, permissions, and settings services
**Status:** DONE
**Files:** `crates/msc-application/src/bedrock_players.rs`, `crates/msc-application/src/bedrock_settings.rs`, `crates/msc-application/tests/bedrock_players.rs`, `fixtures/bedrock-properties/`, `fixtures/bedrock-players/`, `fixtures/bedrock-leveldb/`
**What:** Provide the shared services for Bedrock player discovery, XUID/name cache updates, allowlist and permissions mutation, live reload where supported, and validated `server.properties` changes. Every write must use the substrate’s atomic path and preserve a valid prior file if validation or replacement fails.
**Verify:** `cargo nextest run -p msc-application --test bedrock_players`
**Commit:** `P10.20: add Bedrock player and settings services`
**Batch:** safe

### P10.21 — Wire Bedrock HTTP, WebSocket, and CLI behavior
**Status:** DONE
**Files:** `crates/msc-agent/src/routes/bedrock.rs`, `crates/msc-agent/tests/bedrock_routes.rs`, `crates/msc-cli/src/commands/bedrock.rs`, `crates/msc-cli/tests/bedrock.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement the P10.6 contract through the public agent routes and scriptable CLI, including operations, cancellation, capability disclosure, status/metrics/logs, settings, and player/allowlist actions. Reuse existing shared lifecycle routes where specified and ensure unsupported backend features are explicit, permission-checked results rather than absent or Java-only behavior.
**Verify:** `cargo nextest run -p msc-agent --test bedrock_routes && cargo nextest run -p msc-agent --test bedrock_cli`
**Commit:** `P10.21: expose Bedrock public contract`
**Batch:** stop-after

### P10.22 — Update the copied iOS Bedrock client contract
**Status:** DONE
**Files:** `clients/ios/MSCRemoteiOS_Swift/`, `clients/ios/MSCRemoteiOSTests/`, `docs/msc2/client-capability-matrix.csv`, `tools/phase10/ios-contract-check.py`
**What:** Update the copied iOS client’s Bedrock DTO decoding and supported lifecycle/settings/player flows against the P10.6 public contract, while keeping presentation-specific behavior client-owned. The MSC 1 oracle remains read-only. Record each delivered or intentionally unavailable capability in the matrix; do not claim a desktop/web surface before Phase 11.
**Verify:** `python3 tools/phase10/ios-contract-check.py`
**Commit:** `P10.22: update copied iOS Bedrock contract`
**Batch:** solo

### P10.23 — Add one synthetic cross-backend Bedrock smoke
**Status:** DONE
**Files:** `tools/phase10/phase10-smoke.sh`, `crates/msc-agent/tests/bedrock_routes.rs`, `crates/msc-agent/tests/bedrock_cli.rs`, `crates/msc-agent/tests/support/bedrock_smoke.rs`, `docs/msc2/bedrock/evidence/`
**What:** Add one offline public-path smoke that runs the same fixture-backed API and CLI workflow against Linux-native, Windows-native, and macOS-sidecar fakes. It must cover provision, lifecycle, console, command, player/settings state, cancellation/recovery, runtime-unavailable disclosure, and ensure no test needs a live provider or personal world.
**Verify:** `bash tools/phase10/phase10-smoke.sh --synthetic`
**Commit:** `P10.23: add Bedrock cross-backend smoke`
**Batch:** stop-after

### Evidence and gate

### P10.24 — Record safe official-distribution evidence
**Status:** DONE
**Files:** `docs/msc2/bedrock/evidence/`, `tools/phase10/evidence-check.py`, `docs/msc2/bedrock/compatibility-matrix.csv`
**What:** Record the reproducible official-distribution and package-identity evidence each runtime needs, or a precise unavailable result where licensing, host support, or safe access prevents it — including Apple Silicon Mac distribution, unavailable per D-028 (no test hardware). The checker must reject fabricated success and must link every supported matrix cell to its matching record.
**Verify:** `python3 tools/phase10/evidence-check.py --distribution`
**Commit:** `P10.24: record Bedrock distribution evidence`
**Batch:** solo

### P10.25 — Record native and sidecar runtime evidence
**Status:** DONE
**Files:** `docs/msc2/bedrock/evidence/`, `docs/msc2/bedrock/compatibility-matrix.csv`, `tools/phase10/evidence-check.py`
**What:** Record Linux-native, Windows-native, and macOS-sidecar (Intel) lifecycle evidence using the same terms as the capability matrix: supported, unsupported, or unavailable. Include UDP reachability and clean/crash termination where a safe disposable environment exists; retain unavailable outcomes rather than replacing them with claims from a fake runtime. Record Apple Silicon Mac evidence as unavailable per D-028, not omitted.
**Verify:** `python3 tools/phase10/evidence-check.py --runtimes && python3 tools/phase10/compatibility-check.py docs/msc2/bedrock/compatibility-matrix.csv`
**Commit:** `P10.25: record Bedrock runtime evidence`
**Batch:** stop-after

### P10.26 — Run Phase 10 synthetic checks in tri-platform CI
**Status:** DONE
**Files:** `.github/workflows/ci.yml`, `tools/phase10/phase10-smoke.sh`, `tools/phase10/compatibility-check.py`, `tools/phase10/evidence-check.py`
**What:** Extend the existing macOS/Linux/Windows jobs with the offline Phase 10 smoke and documentary checks, while preserving the headless no-GUI link proof. CI must use fakes and fixtures only; it must not download BDS, require a Mojang account, start a VM, or make public-network calls.
**Verify:** `git diff --check && rg -n 'phase10-smoke.sh --synthetic|phase10/compatibility-check.py|phase10/evidence-check.py' .github/workflows/ci.yml`
**Commit:** `P10.26: run Bedrock checks in CI`
**Batch:** solo

### P10.27 — Record the exact tri-platform Phase 10 candidate
**Status:** awaiting verification
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

## Phase 11 — Desktop and web clients

**Gate** (`msc2-port-plan.md` §3): one Svelte frontend is delivered through
both the Tauri desktop shell and the agent-served browser UI, built against
the proven API while preserving MSC 1's information architecture and design
language. `GET /v1/help/{helpId}` resolves the embedded handbook, concept, and
router-guide content, and clients consume that contract rather than carrying
their own divergent teaching text. UI completion never gates headless agent
correctness.

**Working exit criteria:** macOS, Windows, and Linux desktop builds and the
browser load the same generated frontend bundle and expose every Phase
11-owned capability that the connected agent advertises and the calling token
permits. All client state is host-keyed, the current host and active server are
always visible, reconnecting restores bounded console and operation state, and
dangerous actions remain explicit. TypeScript DTOs are generated from
`docs/msc2/api-contract/openapi.json`; no handwritten mirror DTOs are allowed.
The agent serves the same static bundle plus embedded educational content, and
the Desktop/Web column of `client-capability-matrix.csv` is updated with every
surface rather than filled in retrospectively. Browser cookie auth and Tauri
local/remote auth close Phase 11's remaining D-012 scope without weakening
Phase 9's loopback-by-default, explicit-Tailscale-only posture. Linux proof
must launch the real Tauri binary through WebKitGTK under a display server and
interact with it; a Chromium-only browser run is not Linux desktop evidence.

**Extensibility boundary:** navigation and routing are a registry of section
descriptors keyed by stable strings and capability predicates, not a closed
enum, exhaustive switch, or fixed tab array. The route families
`/hosts/:hostId/servers/:serverId/bedrock/*` and
`/hosts/:hostId/servers/:serverId/profiles/*` are reserved without shipping
either screen. A later Bedrock client group registers its sections only when
`GET /v1/capabilities` advertises the Phase 10 backend; it never infers support
from the host OS or client build. A later player-profiles phase must first port
the ledgered agent workflows (profile loads, Mojang/Floodgate resolution,
manual Bedrock identification, UUID migration/data mutation, hidden profiles,
and skin storage/serving), extend the public contract and capability response,
regenerate TypeScript, then register its section. Phase 11's registry,
host/server-scoped route parameters, lazy section loading, permission filters,
and unknown-capability tolerance are the seam that later phase consumes; none
of that work should require replacing Phase 11 navigation.

**Parallel execution rule:** the first group below is completely independent
of Phase 10 and may execute while Phase 10 owns `crates/`. It uses only the
already-frozen Phase 2 contract and client/test/document trees, keeps the Tauri
crate standalone from the root Cargo workspace, and does not edit
`.github/workflows/ci.yml`. Every later group is explicitly blocked until
Phase 10 closes because it either consumes Phase 10's final additive contract,
touches `crates/`, changes shared CI/packaging, or relies on the exact Phase 10
candidate. Never continue a batch after a failed Verify.

**Owner choices confirmed during PLAN (2026-08-22):** general-LAN management
remains unavailable for v1. Browser management stays on loopback or an
explicitly configured Tailscale path, and Tailscale never replaces
authentication or permission checks; Phase 11 does not build a local
certificate authority or ask users to bypass browser certificate warnings.
On macOS and Windows, MSC may download and verify a coordinated desktop,
agent, and compatible sidecar update, but it asks before installation; it does
not silently install automatically. Linux update installation remains owned by
the package manager, with MSC limited to an actionable availability notice.

### Group A — Phase 10-independent client foundation and Java surfaces (may execute before Phase 10 closes)

### P11.1 — Scope the client rebuild from the iOS oracle and capability matrix
**Status:** awaiting verification
**Files:** `docs/msc2/clients/phase11-scope.md`, `docs/msc2/audit/msc2-symbol-ledger.csv`, `tools/phase11/scope-check.py`, `docs/msc2/rolling-plan.md`
**What:** Read all 53 copied iOS client files as the primary behavioral and screen-structure reference, then use the MSC 1 macOS views only for the desktop information architecture and visual language. Map every current OpenAPI/WebSocket operation and matrix row to a Phase 11 screen, shared infrastructure, honest future state, or explicitly out-of-scope agent gap. Record the D-003 same-screen rule, D-013 host scoping, D-023 matrix-update rule, D-026 help ownership, D-021 client resource bounds, and the exact Bedrock/player-profile extensibility handoffs above. Do not assign the player-profile agent rows to Phase 11 merely because their old DTOs remain in the frozen baseline.
**Verify:** `python3 tools/phase11/scope-check.py docs/msc2/clients/phase11-scope.md docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.1: scope desktop and web clients`
**Batch:** solo

### P11.2 — Scaffold one standalone Svelte and Tauri client
**Status:** not started
**Files:** `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src/`, `clients/desktop-web/static/`, `clients/desktop-web/src-tauri/`, `clients/desktop-web/svelte.config.js`, `clients/desktop-web/vite.config.ts`, `clients/desktop-web/tsconfig.json`
**What:** Create one TypeScript Svelte frontend with a static build suitable for both agent serving and a thin Tauri 2 shell. Keep `src-tauri` out of the root Cargo workspace until Phase 10 closes so this step cannot churn Phase 10's Cargo graph or `crates/`; give it its own lockfile and no server-management behavior. Establish formatting, type-checking, unit-test, production-build, and bundle-identity commands.
**Verify:** `npm --prefix clients/desktop-web run verify:scaffold`
**Commit:** `P11.2: scaffold shared Svelte client`
**Batch:** solo

### P11.3 — Generate TypeScript from the frozen OpenAPI contract
**Status:** not started
**Files:** `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/api/generate.ts`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `tools/phase11/generated-types-check.py`
**What:** Generate the HTTP request/response type surface directly from `docs/msc2/api-contract/openapi.json`, preserving optional/additive fields needed for D-010 skew. Make regeneration deterministic and fail when checked-in output differs from the contract. Handwritten transport helpers may wrap generated types, but no hand-authored DTO mirror is permitted.
**Verify:** `npm --prefix clients/desktop-web run api:check`
**Commit:** `P11.3: generate TypeScript API types`
**Batch:** solo

### P11.4 — Build the contract-backed client test harness
**Status:** not started
**Files:** `clients/desktop-web/src/lib/testing/`, `clients/desktop-web/tests/contract/`, `clients/desktop-web/tests/fixtures/`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`
**What:** Add deterministic fake HTTP, WebSocket, upload/download, operation, auth, capability, permission, old-agent/new-agent, and reconnect scenarios using generated DTO shapes. Include unknown optional fields and absent future capability keys so the UI proves additive skew tolerance. This becomes the reviewed test boundary later `safe` screen batches use; it must not emulate Bedrock or player-profile behavior that no finalized agent contract advertises.
**Verify:** `npm --prefix clients/desktop-web run test:contract`
**Commit:** `P11.4: add client contract harness`
**Batch:** solo

### P11.5 — Establish extensible information architecture and routing
**Status:** not started
**Files:** `clients/desktop-web/src/lib/navigation/`, `clients/desktop-web/src/routes/`, `clients/desktop-web/tests/navigation/`, `docs/msc2/clients/phase11-scope.md`
**What:** Implement the descriptor registry, nested host/server route parameters, permission and capability predicates, lazy component loading, stable deep links, narrow/wide layouts, and unknown-section fallback. Prohibit a closed section enum, exhaustive section switch, fixed tab-count assumptions, and checks such as `hostOs == linux` standing in for capability discovery. Reserve but do not register or render Bedrock and player-profile route families; tests must prove a synthetic future descriptor can be added without editing the shell/router and remains hidden until its named advertised capability is present.
**Verify:** `npm --prefix clients/desktop-web run test:navigation`
**Commit:** `P11.5: add extensible client routing`
**Batch:** solo

### P11.6 — Make all connection and cache state host-scoped
**Status:** not started
**Files:** `clients/desktop-web/src/lib/hosts/`, `clients/desktop-web/src/lib/stores/`, `clients/desktop-web/tests/hosts/`
**What:** Implement D-013's host registry, minimal host switcher, per-host connection/capability/permission/server/console/operation caches, active-server selection, stale-data isolation, and explicit host identity on every destructive confirmation. Credentials remain behind an injected credential adapter so the browser and Tauri mechanisms can land later without migrating store shapes. No singleton active host, global console buffer, or credential field may leak across hosts.
**Verify:** `npm --prefix clients/desktop-web run test:hosts`
**Commit:** `P11.6: add host-scoped client state`
**Batch:** safe

### P11.7 — Implement the generated HTTP and resilient stream client
**Status:** not started
**Files:** `clients/desktop-web/src/lib/api/`, `clients/desktop-web/src/lib/streams/`, `clients/desktop-web/src/lib/operations/`, `clients/desktop-web/tests/transport/`
**What:** Build one host-aware transport over generated request/response types, `ErrorDTO`, version headers, capability refresh, bounded staged transfers, and cookie-or-bearer credential adapters. Add console, operation, and notification stream reconnect with bounded history, deduplication, cancellation, terminal-state recovery, and explicit unsupported/old-client states. Keep browser and desktop on the same calls; shell IPC may supply credentials or native services but never an alternative management API.
**Verify:** `npm --prefix clients/desktop-web run test:transport`
**Commit:** `P11.7: build shared API transport`
**Batch:** safe

### P11.8 — Build the responsive MSC design system and application shell
**Status:** not started
**Files:** `clients/desktop-web/src/lib/components/`, `clients/desktop-web/src/lib/styles/`, `clients/desktop-web/src/routes/+layout.svelte`, `clients/desktop-web/tests/visual/`
**What:** Translate the copied iOS component structure and MSC 1 macOS design language into reusable tokens, cards, tables, forms, dialogs, alerts, empty/loading/error states, keyboard focus, reduced motion, and responsive sidebar/bottom-navigation shells. Preserve desktop's server-list/sidebar and always-available console concepts without baking today's section count into layout. The shell must visibly name the selected host and server and remain usable at phone, tablet, and desktop widths.
**Verify:** `npm --prefix clients/desktop-web run test:visual-shell`
**Commit:** `P11.8: build shared MSC interface shell`
**Batch:** stop-after

### P11.9 — Build fleet, provisioning, and lifecycle workflows
**Status:** not started
**Files:** `clients/desktop-web/src/lib/sections/home/`, `clients/desktop-web/src/lib/sections/fleet/`, `clients/desktop-web/tests/screens/fleet.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement status, active-server switching, create/import/rename/delete/EULA, Java family/version/runtime selection and install, templates, start/stop/restart, clear confirmations, capability/permission gates, and durable operation progress. Use the iOS create/import flows as the functional reference and desktop macOS views for hierarchy only. Update each delivered Desktop/Web matrix row in this same step; unsupported or agent-Planned routes stay `Planned`, never implied by a disabled decorative control.
**Verify:** `npm --prefix clients/desktop-web run test:screen-fleet && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.9: add fleet and lifecycle screens`
**Batch:** safe

### P11.10 — Build console, commands, operations, notifications, and performance
**Status:** not started
**Files:** `clients/desktop-web/src/lib/sections/console/`, `clients/desktop-web/src/lib/sections/performance/`, `clients/desktop-web/src/lib/components/operations/`, `clients/desktop-web/src/lib/components/notifications/`, `clients/desktop-web/tests/screens/live.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement the bounded live console with history/search/filter/pause/copy/clear-local-view, command history/favorites, operation progress/cancel/recovery, notification feed, performance metrics/charts, help affordances, and reconnect behavior. Use DOM/SVG/CSS rendering with a low-cost fallback rather than assuming Chromium-only WebGL/canvas behavior. Update the matching Desktop/Web matrix cells.
**Verify:** `npm --prefix clients/desktop-web run test:screen-live && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.10: add live server screens`
**Batch:** safe

### P11.11 — Build the online roster without claiming player profiles
**Status:** not started
**Files:** `clients/desktop-web/src/lib/sections/players-online/`, `clients/desktop-web/tests/screens/players-online.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Render only the generic online roster the connected agent actually advertises. Do not build the Bedrock allowlist/permissions UI in this phase. Keep the registered section identity distinct from the reserved future `profiles` route; do not call the frozen-but-unimplemented profile, skin, hidden-profile, session-history, UUID migration, or player-data mutation routes and do not present their matrix cells as implemented. Prove the online section still works when profile capability fields are unknown, absent, or later added.
**Verify:** `npm --prefix clients/desktop-web run test:screen-players-online && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.11: add online player roster`
**Batch:** safe

### P11.12 — Build worlds, backups, and staged transfer workflows
**Status:** not started
**Files:** `clients/desktop-web/src/lib/sections/worlds/`, `clients/desktop-web/src/lib/sections/backups/`, `clients/desktop-web/src/lib/transfers/`, `clients/desktop-web/tests/screens/worlds-backups.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement slot inventory/activation/create/rename/duplicate/delete/import/export/convert, direct active-world mutations where the API supports them, thumbnails, backup create/config/delete/restore, bounded uploads/downloads, transactional warnings, progress/cancel/recovery, and risk-appropriate confirmations. Update every genuinely delivered Desktop/Web matrix cell and leave unavailable agent paths visible only as truthful capability explanations.
**Verify:** `npm --prefix clients/desktop-web run test:screen-worlds-backups && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.12: add world and backup screens`
**Batch:** safe

### P11.13 — Build add-on, modpack, and component workflows
**Status:** not started
**Files:** `clients/desktop-web/src/lib/sections/addons/`, `clients/desktop-web/src/lib/sections/components/`, `clients/desktop-web/tests/screens/addons.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement installed add-ons, catalog search, install/update/toggle/remove/source actions, system-component state, client export, modpack inspect/import/replace, and D-027 manual browser-download then bounded staged-upload completion. Preserve provider-unavailable, dependency, pack-managed, cancellation, and provenance explanations. Update the matching Desktop/Web matrix rows; never hardcode provider or server-family lists where the contract supplies them.
**Verify:** `npm --prefix clients/desktop-web run test:screen-addons && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.13: add add-on and modpack screens`
**Batch:** safe

### P11.14 — Build settings, health, networking, helpers, and access administration
**Status:** not started
**Files:** `clients/desktop-web/src/lib/sections/settings/`, `clients/desktop-web/src/lib/sections/health/`, `clients/desktop-web/src/lib/sections/connectivity/`, `clients/desktop-web/src/lib/sections/access/`, `clients/desktop-web/tests/screens/administration.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Render schema-driven settings without a client-side field enum; implement health cards/problems/repairs, RAM/Java/Geyser, connectivity diagnostics, Playit, DuckDNS, Xbox Broadcast, resource packs, and named-token create/update/revoke with one-time-secret handling. Permission and capability filters must remove unavailable actions while keeping explanations. Agent-Planned files/watchdog/profile routes remain Planned rather than receiving fake screens. Update each delivered matrix row.
**Verify:** `npm --prefix clients/desktop-web run test:screen-administration && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.14: add administration screens`
**Batch:** safe

### P11.15 — Extract and validate the educational content corpus
**Status:** not started
**Files:** `content/help/`, `content/guides/`, `fixtures/help-content/`, `tools/phase11/help-content-check.py`, `docs/msc2/clients/phase11-scope.md`
**What:** Extract MSC 1's 31-topic handbook, concept guide, router catalog/records/steps, troubleshooting content, and onboarding copy into the confirmed Markdown-with-YAML-front-matter and structured guide data formats. Preserve source citations and label content versus executable router rules; do not duplicate prose in Svelte. Record unresolved diagram assets honestly. Include coverage for every `helpId` already emitted by settings, health, diagnostics, performance, connectivity, and errors.
**Verify:** `python3 tools/phase11/help-content-check.py --all`
**Commit:** `P11.15: extract educational content`
**Batch:** solo

### P11.16 — Render contract-served help and guides in the shared client
**Status:** not started
**Files:** `clients/desktop-web/src/lib/help/`, `clients/desktop-web/src/lib/sections/handbook/`, `clients/desktop-web/tests/screens/help.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Build safe Markdown rendering, related-topic navigation, handbook/concept/router-guide readers, contextual `helpId` links, unknown-topic degradation, and client-owned onboarding anchors against the fake contract. Every explanation comes from a response or structured content fixture, never a screen-local copy. The registry must allow future Bedrock and profile topics/sections to appear additively without new shell logic.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && python3 tools/phase11/help-content-check.py --client`
**Commit:** `P11.16: render shared help content`
**Batch:** safe

### P11.17 — Prove browser parity, accessibility, and responsive layouts
**Status:** not started
**Files:** `clients/desktop-web/tests/e2e/browser/`, `clients/desktop-web/playwright.config.ts`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`
**What:** Exercise the production static bundle against the contract harness at narrow and wide widths, keyboard-only navigation, reduced motion, destructive confirmations, host switching, reconnect, upload/download, and deep-link reload. Run Chromium plus browser WebKit for fast compatibility feedback, while recording plainly that this is browser evidence and does not replace P11.19's native Linux WebKitGTK proof.
**Verify:** `npm --prefix clients/desktop-web run test:e2e-browser`
**Commit:** `P11.17: prove shared browser workflows`
**Batch:** stop-after

### P11.18 — Add the thin Tauri shell without desktop-only screens
**Status:** not started
**Files:** `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/tests/tauri/`
**What:** Load the exact production Svelte bundle and expose only narrow native adapters for credentials, file pickers, notifications, menus, window lifecycle, and later agent installation/update. Each native affordance must invoke a shared web workflow with a browser fallback; no route or screen may test `isTauri` to reveal desktop-only management behavior. Keep the standalone Tauri crate outside the root workspace while Phase 10 is active.
**Verify:** `npm --prefix clients/desktop-web run test:tauri-boundary`
**Commit:** `P11.18: add thin Tauri shell`
**Batch:** solo

### P11.19 — Exercise the real Linux Tauri renderer through WebKitGTK
**Status:** not started
**Files:** `clients/desktop-web/tests/e2e/tauri-linux/`, `clients/desktop-web/wdio.conf.ts`, `tools/phase11/linux-webkitgtk-smoke.sh`, `docs/msc2/clients/evidence/`
**What:** On a Debian/Ubuntu desktop runner with `libwebkit2gtk-4.1`, `webkit2gtk-driver`, and Xvfb, launch the built Tauri binary and drive its real window through the native WebDriver path. Verify visible shell, navigation, CSS layout, forms, dialogs, live-console fallback, deep links, and one mutating fake workflow; record the WebKitGTK package/version and screenshot evidence. A Vite page opened in Chrome or Playwright's bundled WebKit does not satisfy this step.
**Verify:** `bash tools/phase11/linux-webkitgtk-smoke.sh --native`
**Commit:** `P11.19: prove Linux WebKitGTK rendering`
**Batch:** stop-after

### Group B — Phase 10-dependent Bedrock extension seam (must wait until Phase 10 closes; no Bedrock screens)

### P11.20 — Regenerate against Phase 10 and prove capability-driven extension seams
**Status:** not started — blocked on Phase 10
**Files:** `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/navigation/`, `clients/desktop-web/tests/navigation/bedrock-extension.test.ts`, `docs/msc2/client-capability-matrix.csv`, `docs/msc2/clients/phase11-scope.md`
**What:** Regenerate TypeScript from the exact post-Phase-10 OpenAPI document and consume its finalized capability advertisement without hand-written Bedrock DTOs or host-OS inference. Use a test-only future section descriptor to prove Bedrock navigation is absent when unsupported, can be registered when `serverTypes.bedrock` advertises support, survives unknown backend values additively, and fits existing layouts/routes without restructuring. Ship no Bedrock section, creation flow, settings, player, allowlist, world, backup, console, or runtime screen in Phase 11; keep those matrix cells Planned for the later Bedrock client group.
**Verify:** `npm --prefix clients/desktop-web run api:check && npm --prefix clients/desktop-web run test:bedrock-extension && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.20: preserve Bedrock client extension seam`
**Batch:** solo

### Group C — Shared agent, auth, packaging, and gate work (must wait until Phase 10 closes)

### P11.21 — Close the remaining desktop and browser authentication design
**Status:** not started — blocked on Phase 10
**Files:** `docs/msc2/clients/phase11-auth.md`, `docs/msc2/msc2-decisions.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/api-contract/auth-scope-phase2.md`, `docs/msc2/lifecycle/pairing-phase4.md`, `crates/msc-api/tests/phase11_auth_conformance.rs`
**What:** Turn D-012's approved mechanisms and Phase 9 posture into one testable contract: same-machine Tauri bootstrap resistant to arbitrary local-process impersonation, per-host remote desktop pairing and secret-store keys, browser pairing-to-httpOnly-SameSite cookie exchange, session revocation/expiry, exact allowed-origin/CSP rules, CSRF tokens for cookie-authenticated mutations, and bearer exemption. Preserve loopback-by-default management and authenticated explicit-Tailscale access; per the owner-confirmed v1 choice above, keep general-LAN management unavailable and do not build certificate provisioning or a local trust system. Use additive versioned routes and `ErrorDTO`; do not put raw credentials in Svelte-accessible storage or URLs.
**Verify:** `python3 tools/api-contract-check.py --v1-summary && cargo nextest run -p msc-api --test phase11_auth_conformance`
**Commit:** `P11.21: freeze desktop and browser auth`
**Batch:** solo

### P11.22 — Implement browser sessions, origin policy, CSP, and CSRF
**Status:** not started — blocked on Phase 10 and P11.21
**Files:** `crates/msc-agent/src/auth/`, `crates/msc-agent/src/routes/browser_session.rs`, `crates/msc-agent/tests/browser_auth.rs`, `clients/desktop-web/src/lib/auth/browser.ts`, `clients/desktop-web/tests/auth/browser.test.ts`
**What:** Implement the frozen browser pairing/session path on the existing credential registry, with one-use challenges, httpOnly cookies, revocation and restart behavior, exact origin checks, restrictive CSP, CSRF on every cookie-authenticated mutation, rate limits, and audit attribution. Prove bearer clients remain unaffected and a hostile origin/local script cannot turn ambient browser authority into a server mutation.
**Verify:** `cargo nextest run -p msc-agent --test browser_auth && npm --prefix clients/desktop-web run test:auth-browser`
**Commit:** `P11.22: add secure browser sessions`
**Batch:** stop-after

### P11.23 — Implement local and remote Tauri credentials per host
**Status:** not started — blocked on Phase 10 and P11.21
**Files:** `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/auth/desktop.ts`, `clients/desktop-web/tests/auth/desktop/`, `crates/msc-agent/src/auth/`, `crates/msc-agent/tests/desktop_auth.rs`
**What:** Implement the chosen same-machine authorization handshake and remote pairing exchange, storing one credential per agent host ID in the platform credential store through the shell so secrets never enter browser storage. Verify local convenience does not become loopback-open authorization, remote credentials obey permission/expiry/revocation, switching hosts cannot reuse another host's credential, and the web build retains its cookie flow with no divergent screen.
**Verify:** `cargo nextest run -p msc-agent --test desktop_auth && npm --prefix clients/desktop-web run test:auth-desktop`
**Commit:** `P11.23: add per-host desktop credentials`
**Batch:** stop-after

### P11.24 — Serve embedded help content and port router-guide rules
**Status:** not started — blocked on Phase 10
**Files:** `crates/msc-domain/src/router_guides.rs`, `crates/msc-domain/tests/router_guides.rs`, `crates/msc-agent/src/help.rs`, `crates/msc-agent/src/routes/help.rs`, `crates/msc-agent/tests/help_routes.rs`, `content/help/`, `content/guides/`, `fixtures/help-content/`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/client-capability-matrix.csv`
**What:** Embed the validated content corpus, implement `GET /v1/help/{helpId}` plus the additive handbook/concept/router-guide catalog routes required to browse it, and port the ledgered router matcher/fallback/composer/troubleshooting rules into Rust against fixtures. Return raw Markdown/structured steps for every client to render; unknown or version-new topics degrade through `ErrorDTO`. Do not absorb client onboarding anchors or presentation into the agent.
**Verify:** `cargo nextest run -p msc-domain --test router_guides && cargo nextest run -p msc-agent --test help_routes && npm --prefix clients/desktop-web run test:screen-help`
**Commit:** `P11.24: serve shared educational content`
**Batch:** solo

### P11.25 — Serve the same production bundle from the agent
**Status:** not started — blocked on Phase 10
**Files:** `crates/msc-agent/src/web_ui.rs`, `crates/msc-agent/tests/web_ui.rs`, `clients/desktop-web/`, `tools/phase11/bundle-identity-check.py`
**What:** Embed or package the exact Svelte production output the Tauri shell loads, serve hashed assets with correct MIME/cache headers and CSP, support safe client-side deep-link fallback without shadowing `/v1`, and provide an explicit unavailable result when a headless package intentionally omits web assets. Prove byte identity between the browser-served and Tauri-loaded bundles and preserve D-011's no-GUI dependency boundary in the agent.
**Verify:** `python3 tools/phase11/bundle-identity-check.py && cargo nextest run -p msc-agent --test web_ui`
**Commit:** `P11.25: serve shared web bundle`
**Batch:** stop-after

### P11.26 — Install and manage the local agent through the shell
**Status:** not started — blocked on Phase 10
**Files:** `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/setup/`, `clients/desktop-web/tests/agent-install/`, `tools/phase11/agent-install-smoke.sh`, `packaging/`
**What:** Add shell-only native commands behind shared setup screens to detect, install, start, stop, repair, and report the platform service and compatible agent/sidecar package. Closing the window must never stop the service or a server. Browser users see the same status/setup route with a truthful instruction/fallback when native install is unavailable; there is no desktop-only screen. Preserve existing service identity, privilege, headless, and rollback rules.
**Verify:** `bash tools/phase11/agent-install-smoke.sh --synthetic`
**Commit:** `P11.26: manage local agent installation`
**Batch:** stop-after

### P11.27 — Define and prove coordinated desktop, agent, and sidecar updates
**Status:** not started — blocked on Phase 10
**Files:** `docs/msc2/clients/phase11-update.md`, `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/updates/`, `packaging/`, `tools/phase11/update-smoke.sh`
**What:** Implement the owner-confirmed prompted, signed macOS/Windows update policy as a compatibility-aware set: download and verify release identity before staging, ask before installation, keep the running agent until replacement is ready, preserve configuration/secrets/worlds, pair the exact compatible Bedrock sidecar where applicable, roll back a failed update, and allow app/agent version skew only within D-010's advertised window. Never install silently. Linux defers installation to its package manager and receives an actionable notice, not a second self-updater. Never merge MSC updates with server/loader/add-on update controls.
**Verify:** `bash tools/phase11/update-smoke.sh --synthetic --all-platforms`
**Commit:** `P11.27: add coordinated client updates`
**Batch:** solo

### P11.28 — Build and exercise desktop and web candidates on all three platforms
**Status:** not started — blocked on Phase 10
**Files:** `.github/workflows/ci.yml`, `tools/phase11/desktop-web-smoke.sh`, `clients/desktop-web/`, `docs/msc2/clients/evidence/`
**What:** Add production frontend/type tests, agent-served browser smoke, and real Tauri builds to macOS, Windows, and Linux CI while preserving the headless no-GUI job. Exercise the same core workflow in browser and desktop modes; on Linux run P11.19's native WebKitGTK smoke, not a Chromium substitute. Record platform renderer/package versions and explicit unavailable signing/notarization evidence without claiming unperformed release distribution.
**Verify:** `bash tools/phase11/desktop-web-smoke.sh --synthetic --all-surfaces`
**Commit:** `P11.28: prove tri-platform clients`
**Batch:** stop-after

### P11.29 — Reconcile the capability matrix and run the exact Phase 11 gate
**Status:** not started — blocked on Phase 10
**Files:** `docs/msc2/client-capability-matrix.csv`, `tools/phase11/phase11-check.py`, `docs/msc2/clients/evidence/phase11-ci.md`, `docs/msc2/clients/phase11-scope.md`, `docs/msc2/rolling-plan.md`
**What:** Check every contract and WebSocket operation against real Desktop/Web implementation evidence, leaving agent-Planned, Bedrock-screen, and player-profile rows explicitly Planned and no cell blank. Prove D-003 bundle/screen identity, generated DTO drift, D-013 host isolation, capability/permission routing, D-026 served-content use, browser/Tauri auth, browser responsive behavior, native Linux WebKitGTK rendering, tri-platform packaging, headless independence, and the exact green CI candidate. This is Phase 11's only full-workspace run; the other agent decides in REVIEW whether the literal gate holds.
**Verify:** `python3 tools/phase11/phase11-check.py --gate && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && cargo nextest run --workspace`
**Commit:** `P11.29: check desktop and web gate`
**Batch:** solo
