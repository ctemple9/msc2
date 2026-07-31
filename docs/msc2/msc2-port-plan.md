# MSC 2 — Port Plan

**Revision:** 1.2 · **Date:** 2026-07-29
**Status:** **Execution document — Proposed, not owner-approved.**

**This document is deliberately separate from the vision.** `msc2-product.md` and `msc2-engineering.md` describe the destination, its guarantees, and its architecture. This document describes a *route*, and routes change. Nothing here constrains the vision; if a phase order proves wrong, this file changes and the vision does not.

**Do not treat sequencing here as settled.** The principles it implements are in `msc2-decisions.md` D-016 (port strategy), D-017 (Windows validation timing), and D-018 (behavioral evidence before translation). Those principles are the durable part.

**Companion documents:** `MSC2-VISION.md` · `msc2-product.md` · `msc2-engineering.md` · `msc2-decisions.md`

---

## 1. Shape of the work

**Vertical slices, not subsystems.** After the substrate exists, each stage cuts through engine, API, and clients together. The failure mode for a rewrite this size is a long stretch with a working engine and no working software — easy to fall into when the specification is organized by subsystem.

**Extraction precedes translation, per domain.** Parsers and policies embedded in MSC 1's views and view-model extensions are pulled out *in Swift*, where the compiler still checks the refactor, before that domain is translated to Rust.

This is **not** a blanket gate before all Rust, and **not** a mechanical grep rule. Extraction is driven by a **symbol ledger that does not exist yet**. The two audit CSVs are *file-level* dispositions — they say which files to open, not which symbols inside them must survive. Building that ledger is a Phase 0 deliverable. Only behavior that must be preserved is extracted. Client-side concerns — file pickers, image cropping, window presentation, avatar rendering — legitimately remain in UI code and are not extraction targets.

**The deletion test.** For any SwiftUI file, the question is not *"does it contain I/O?"* but *"does this behavior belong to the agent or to the replacement client?"*

| Behavior | Belongs to |
|---|---|
| Avatar image cropping and rendering | Client |
| Finder / file-picker presentation | Client |
| Window and navigation state | Client |
| Server installation detection | **Agent** |
| Pairing security and token lifecycle | **Agent** |
| Console log parsing | **Agent** |
| Settings validation and clamping | **Agent** (schema), rendered by client |

A file may be retired once every agent-owned symbol in it has a disposition record. Static scanning flags candidates; it does not make the decision.

---

## 2. Estimated corpus

**≈33,000–36,000 lines of engine logic to translate** — roughly one third of MSC 1's 97,357 lines.

**This is not an effort estimate.** It measures *preserved behavior only*. MSC 2 additionally requires substantial code with no MSC 1 equivalent:

- Cross-platform service integration (`launchd`, Windows Service, `systemd`)
- Process ownership and supervision across three operating systems
- Operation journaling and restart recovery
- Long-operation progress, cancellation, and idempotency
- Safe remote file streaming
- OpenAPI generation and client codegen
- Cross-platform secret stores
- Self-update for app + agent + sidecar
- Native Linux and Windows Bedrock runtimes
- Tauri and web client state, reconnect, and multi-host handling

That new work may well exceed the ported corpus. Use the 33–36k figure only as evidence that the preserved behavior is *finite*.

---

## 3. Phases

### Phase 0 — Freeze the baseline and build the harness

No Rust. **Deliberately small.** Phase 0 is not "characterize everything MSC 1 does" — that would be unbounded and would delay the first working software indefinitely. It establishes the machinery that makes characterization possible, and freezes what cannot be recovered later.

| Deliverable | Why it must be global |
|---|---|
| **Fixture harness** — the format, runner, and comparison tooling for language-neutral fixtures | Every later domain depends on it |
| **API baseline** — OpenAPI plus WebSocket event schemas captured from MSC 1 | The contract everything else is generated from |
| **Symbol ledger** — one row per parser, policy, and workflow inside a Mixed or UI file | Drives all later extraction. The audit CSVs are *file-level inputs* to this, not the ledger itself |
| **Sidecar IPC contract** — the macOS Bedrock process protocol | Shapes the `BedrockRuntime` trait |
| **Reference corpus** — real logs, packs, server directories, historical configs, DTO examples | Snapshot of live evidence; cheapest to gather now |
| **Extraction of the 270 existing tests** from inline Swift literals | Mechanical, and they are the seed set |

**Exit criteria:** a fixture can be written, run, and compared. The API baseline exists. The ledger exists.

### Per-domain characterization — immediately before each translation

Everything else in §4A happens **just before the domain that needs it**, not up front. Characterizing world mutation before anything can run a world is wasted precision; characterizing it the week before it is translated is exactly right.

This keeps Phase 0 bounded, keeps evidence fresh, and lets Phase 1 begin quickly.

### Phase 1 — Domain types and pure rules

Server identity, flavors, version comparison, Java policy, property models, command catalog, TPS parsing, crash analysis, slug normalization, and the router rule engine (matcher, fallback resolver, composer, troubleshooting engine — these are executable behavior, not data).

Per-domain Swift extraction happens immediately ahead of each translation.

**Exit criteria:** Rust passes the Phase 1-scoped Phase 0 pure fixtures, plus Phase 1 characterization fixtures. No user files touched.

### Phase 2 — API contract and operation model

Versioned HTTP and WebSocket contract generated from the schema. Operation IDs, progress, structured errors, capability advertisement, cancellation. A skeletal agent whose routes can be exercised without real mutation.

**Exit criteria:** the existing iOS app connects and reads status against a stub agent.

### Phase 3 — Safety substrate

Approved server roots and path safety · atomic writes · versioned configuration with migrations · `SecretStore` trait · audit log · download staging with checksum verification · operation journal · operation exclusivity.

**Windows CI begins here** (D-017), covering path separators and length limits, file-locking semantics, service lifecycle, and case-insensitive path comparison.

**Exit criteria:** substrate fixtures pass on macOS, Linux, and Windows.

### Phase 4 — Java lifecycle vertical slice

One imported Paper server, end to end: import and detect · start · console · command · status and metrics · graceful stop · restart. Driven from the CLI **and the existing iOS app**.

**Exit criteria:** headless service ownership proven on **macOS (LaunchDaemon), Linux (systemd), and Windows (Service)** — all three, not two. Closing every client changes nothing about the running server; on Windows, neither does signing out. This is the first stage that produces genuinely useful software.

*Windows is included here deliberately.* Service ownership is the single assumption most expensive to discover late, and D-017 already starts Windows CI one phase earlier.

### Phase 5 — Configuration and migration

Historical MSC config corpus · settings schema as a versioned contract · corruption recovery · MSC 1 transfer-package import (D-009) · raw server-directory import.

### Phase 6 — Worlds and backups

World discovery, slots, transactional mutations, backups, retention, verification, restore.

**Placed before breadth deliberately:** the highest data-loss domain, ported while the codebase is still small enough to review carefully.

### Phase 7 — Server families and provisioning

Vanilla, Paper, Purpur, Fabric, NeoForge, Forge. Runtime selection, installer flows, archive behavior, startup diagnostics. Scope bounded by the 1.20 floor (D-014).

### Phase 8 — Mods, plugins, modpacks

Modrinth / Hangar / CurseForge providers · metadata parsing · dependency resolution · client-only classification · pack-managed guards · import · update · client export.

### Phase 9 — Networking and helpers

Playit · resource-pack hosting · DuckDNS · port diagnostics · Xbox Broadcast · Geyser and Floodgate · notifications · helper process lifecycle.

### Phase 10 — Bedrock runtimes

The `BedrockRuntime` trait, implemented **native Linux → native Windows → macOS VZ Swift sidecar**, so the contract cannot absorb macOS-specific assumptions (D-007).

Bedrock files, properties, players, LevelDB, allowlist, permissions, metrics, and UDP behavior against shared fixtures. Publish the Bedrock compatibility matrix separately from the MSC agent matrix (D-022).

### Phase 11 — Desktop and web clients

Tauri shell plus the Svelte frontend, built against the proven API, preserving MSC 1's information architecture and design language.

**UI completion never gates headless agent correctness.**

### Phase 12 — Terminal UI

The `ratatui` dashboard. Deferred from v1 (D-015); built only once the API has stopped moving.

### Continuous, from Phase 1 onward

The client capability matrix (D-023) is updated as each capability lands, with intentional exceptions recorded rather than discovered.

The `RouterPortForward*` guide catalog, router records, and static step content migrate to JSON at any time — but the rule engine belongs to Phase 1.

---

## 4. Test inventory

Two categories, and they belong in different places.

### 4A — MSC 1 characterization (Phase 0)

Behavior MSC 1 exhibits today and MSC 2 must reproduce. Capturable now, by observing the running application. **Impossible to capture later.**

MSC 1's existing 270 tests concentrate on parsing and API contracts. Coverage is strong where failure is cheap and weak where failure is expensive. The following do not exist and must be written in Phase 0 (D-018).

**Worlds and backups**
Full slot create/activate/duplicate/copy/delete/import/export matrix · Java multi-folder worlds (`world`, `world_nether`, `world_the_end`) · Bedrock layouts · archive path traversal and symlink escape · backup while running (`save-off`, `save-all`, timeout, resume) · failed archive creation · interrupted restore · retention when only one known-good backup exists · rollback after partial rename or replacement · real historical MSC backup metadata.

**Process lifecycle** *(as MSC 1 exhibits it — macOS)*
Partial output lines and mixed newline conventions · graceful-stop timeout into forced termination · process tree cleanup · duplicate launch prevention · port conflict · Java executable validation **on macOS** · Forge and NeoForge args-file launches across path syntaxes.

**Modpacks and components**
Real `.mrpack` and CurseForge server packs · overrides precedence and permission bits · blocked and manual CurseForge files · missing and circular dependencies · pack-managed update refusal · hash and provenance matching · atomic JAR replacement and rollback · interrupted downloads and corrupt archives · loader installers across supported generations (1.20+).

**Console and diagnostics**
Java and Bedrock join/leave lines · chat and advancement lines · broadcast authentication prompts · ready-state detection per server family · crash logs gathered from real failures · bounded console history and reconnect behavior.

**Bedrock (macOS VZ only — the part MSC 1 has)**
Real compacted LevelDB tables and write-ahead logs · Bedrock NBT and player records across BDS versions · allowlist and permissions round trips · VM boot, readiness, stop, and crash lifecycle · host-directory persistence across VM replacement.

**API** *(baseline behavior MSC 1 actually has)*
At least one authorization and validation test per existing mutating route · WebSocket reconnect and malformed frames · file transfer size, traversal, partial upload, atomic completion · role and permission enforcement · rate limiting and request-size limits.

**Configuration**
A corpus of historical `server_config_swift.json` versions · missing, renamed, malformed, and unknown fields · duplicate or conflicting server IDs and paths · atomic-write interruption.

### 4B — New MSC 2 acceptance tests (built with their vertical slice)

Behavior MSC 1 has never had, on platforms it has never run on. **Nothing to characterize** — these are written against the Rust implementation as it lands.

| Test area | Lands with |
|---|---|
| Windows service lifecycle, sign-out survival, Job Object process trees | Phase 3 substrate / Phase 4 |
| Windows path separators, length limits, file-locking, case-insensitivity | Phase 3 |
| Linux secret storage — chosen headless backend, and degradation behavior | Phase 3 |
| `systemd` unit behavior, boot ordering, restart policy | Phase 3 / Phase 4 |
| Native Linux and Windows Bedrock runtimes | Phase 10 |
| Sidecar IPC failure modes — crash, hang, restart | Phase 10 |
| Multi-host client state, per-host credentials, host switching | Phase 11 |
| Remote desktop pairing and browser origin policy | Phase 2 / Phase 11 |
| Cross-platform sleep inhibition and the two power policies (D-024) | Phase 3 |
| Service identity, file ownership, privilege boundaries (D-025) | Phase 3 |
| Java executable validation on **Windows and Linux** | Phase 4 |
| Secret migration across platforms | Phase 3 / Phase 5 |
| Agent restart while a server runs; operation cancellation and recovery | Phase 3 / Phase 4 |
| Old-client/new-agent and new-client/old-agent skew fixtures | Phase 2 onward |
| Sign-out and reboot survival per platform | Phase 4 |

**Resource efficiency** (D-021) — headless packages verified to link no GUI frameworks · bounded-growth assertions for console buffers, metric history, and caches · idle-agent memory regression tests against measured baselines · the 8 GB minimal-Linux acceptance scenario. *Per-platform, as each platform's agent lands.*

---

## 5. Fixture strategy

| Kind | Approach |
|---|---|
| **Pure functions** | Input/output fixtures. Cheap, exhaustive. |
| **I/O workflows** | Temporary directories, fake providers, process doubles, interruption cases, rollback assertions. Expensive, and required. |

**Parity rule.** Rust output is compared against **expected values**, never against Swift implementation details. A domain is not ported until its fixtures pass and its rollback behavior is explicit.

MSC 1 must remain runnable and buildable throughout as the compatibility oracle (D-005).

---

## 6. Open sequencing questions

| Question | Affects |
|---|---|
| Does the CLI ship inside the agent binary or separately? | Phase 4 packaging |
| Is `UDPRelay` a general Bedrock need or VM-specific? | Phase 10 scope |
| Console history bound — lines, bytes, or time? | Phase 4, and D-021 memory bounds |
| Self-update mechanics for app + agent + sidecar as a set | Phase 11 |
| Is the permission category vocabulary correct across all 87 routes? | Phase 2 contract freeze |
