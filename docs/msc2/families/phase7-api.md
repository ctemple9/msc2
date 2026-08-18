# Phase 7 API, operation, and capability contract

**Status:** P7.9 contract-freeze note, Proposed (carries forward the Proposed status of everything it builds on: `operation-model.md`, D-023's matrix mechanism) until Cameron confirms during the Read move.
**Source of truth:** `docs/msc2/api-contract/openapi.json` (the file this note reconciles), `docs/msc2/families/phase7-scope.md` (P7.1's reconciliation rule and the D-006 addendum), `fixtures/server-jar-providers/`, `fixtures/loader-installers/`, `fixtures/server-creation/`, `fixtures/jar-templates/`, `fixtures/java-runtime-selection/`, `fixtures/java-runtime-guards/`, `fixtures/startup-problems/` (P7.4–P7.8's characterization this contract is built to serve), `docs/msc2/api-contract/operation-model.md`, `docs/msc2/api-contract/versioning-and-errors.md`, `docs/msc2/api-contract/permission-vocabulary.csv`, `docs/msc2/msc2-decisions.md` D-006/D-009/D-019/D-023.

This note does two things: it reconciles the frozen `openapi.json` baseline's seventeen Phase-7-owned routes (plus the one route P7.1 already committed to adding) against MSC 1's actual handlers, and it updates `docs/msc2/client-capability-matrix.csv` to the status each of those eighteen routes actually holds at this commit — following the exact methodology `docs/msc2/worlds/phase6-api.md` established for Phase 6.

---

## 1. What already existed

All seventeen routes named in `rolling-plan.md`'s Phase 7 preamble were already in the baseline (P0.23/P2.8), each with a request/response DTO pair, error-code mapping, and `x-permission-category` already assigned — this phase inherited a contract, it did not start from nothing:

| Route | Maps to (MSC 1) | Shape |
|---|---|---|
| `POST /v1/servers/create` | `handleCreateServer` → `createServerProvider` → `createNewServer` | corrected this step, §2 |
| `POST /v1/servers/delete` | `handleDeleteServer` → `deleteServerProvider` | sync `ServerDeleteResultDTO` |
| `POST /v1/servers/rename` | `handleRenameServer` → `renameServerProvider` | sync `ServerRenameResultDTO` |
| `POST /v1/servers/eula` | `handleAcceptServerEULA` → `acceptEULAProvider` | sync `ServerEULAResultDTO` |
| `GET /v1/versions` | `handleGetVersions` → `versionsProvider` | sync `VersionsResponseDTO` |
| `GET /v1/versions/create` | `handleGetCreateVersions` → `createVersionsProvider` | sync `VersionsResponseDTO` |
| `POST /v1/components/version` | `handleChangeVersion` → `changeVersionProvider` | corrected this step, §2 |
| `GET /v1/templates` | `handleGetTemplates` → `templatesProvider` | sync `TemplatesResponseDTO` |
| `POST /v1/templates` | `handleMutateTemplates` → `templateMutationProvider` | sync `TemplateMutationResultDTO` |
| `GET /v1/java-runtimes` | `handleGetJavaRuntimes` → `javaRuntimesProvider` | sync `JavaRuntimesResponseDTO` |
| `GET`/`POST /v1/config/java-runtime` | `handleGetJavaConfig`/`handleSetJavaConfig` | sync `JavaConfigResponseDTO` |
| `GET`/`POST /v1/config/ram` | `handleGetRAMConfig`/`handleUpdateRAMConfig` | sync `RAMConfigResponseDTO`/`RAMConfigUpdateResultDTO` |
| `GET /v1/health` | `handleGetHealth` → `healthProvider` | sync `HealthResponseDTO` |
| `GET /v1/health/problems` | `handleGetHealthProblems` → `healthProblemsProvider` | sync `HealthProblemsResponseDTO` |
| `POST /v1/health/repair` | `handleRepairHealthProblem` → `repairHealthProblemProvider` | sync `HealthRepairResultDTO`, scope narrowed this step, §6 |

None of these DTOs' existing fields change meaning. D-006 preserves them; this note only adds fields additively (§2) and adds the one operation P7.1 already committed to (§3).

**Confirmed, not re-decided: every permission category above already matches the oracle exactly.** `docs/msc2/api-contract/permission-vocabulary.csv`, read directly from `RemoteAPIServer+HTTP.swift`'s `adminOnlyPOSTPaths`/`pathPermissions`, gives `fleet` for all four `servers/*` mutations and `templates` POST, `addons` for `components/version`, `settings` for `config/java-runtime` POST/`config/ram` POST/`health/repair`, and `none` for every GET — identical to what `openapi.json` already carries. There was nothing here for this step to decide.

---

## 2. Corrections: making "async" mean what it says on the wire

Two routes in the baseline carry a real, load-bearing bug in their own `x-notes`, caught only by reading `changeVersionProvider`'s implementation alongside `createServerProvider`'s call site rather than trusting the earlier note's wording.

**The bug.** `handleCreateServer` and `handleChangeVersion` both wrap their provider call in a Swift `Task { ... }`, but the `sendJSON` that writes the HTTP response sits *after* the `await` on that provider call, inside the same `Task` (`RemoteAPIServer+ComponentRoutes.swift:87-112`, `611-645`). That `Task` not blocking MSC 1's accept loop is a Swift-concurrency fact about the *server process*; it says nothing about the *client's* connection, which stays open, waiting, for however long `createServerProvider`/`changeVersionProvider` actually takes. The previous `x-notes` on `POST /v1/servers/create` ("the handler ultimately runs async via createServerProvider") conflated the two and is corrected in this commit.

For four of the six families that duration is a jar download — seconds on a good connection, longer on a bad one. For NeoForge and Forge it is a **supervised installer subprocess that P7.3 actually ran and timed**: real minutes, not an estimate. `changeVersionProvider`'s own source comment even names this correctly — *"Java: authoritative-async (awaits the real result)"* (`AppViewModel+APIWiringAddons.swift:357`) — "awaits the real result" is exactly the problem: the HTTP response is one of the things waiting on it. A headless agent on a flaky link, or an iOS client that backgrounds mid-request, has no way to survive that today. This is the precise gap `operation-model.md`'s own opening rationale names — *"loader installations take minutes, not milliseconds... hostile to every client"* — and `rolling-plan.md`'s own P7.9 "What" line flags it directly: *"server creation with an install step is minutes long and must survive an agent restart."*

**The fix, under D-006's "correction" clause (a documented behavior change, not a silent one):**

- **`POST /v1/servers/create`.** After the existing synchronous validation (name/type/flavor, unchanged 400s) and, for `serverType: "bedrock"`, the new `capability_unavailable` refusal (§5), the `200` response now returns as soon as the operation is admitted — carrying `serverId`/`serverName` (both knowable synchronously: folder derivation is a pure function of the trimmed name, `phase7-scope.md`'s "Cross-family creation mechanics" step 2) and a populated `operationId`. `warnings` and final `success` move to the operation's own terminal `result` (`operation-model.md` §2) instead of riding the initial response. `ServerCreateResultDTO.operationId` was already reserved as an optional field in the P2.8 baseline — it exists in the schema today but nothing sets it; this step is the first time anything actually populates it. `operationId` stays optional on the schema (a client that only reads `success`/`message`/`serverId`/`serverName` keeps working unmodified), the same "optional so older clients can ignore it" convention `phase6-api.md` §2 established for `activate`/`backups/now`/`backups/restore`. Operation `type: "server-create"`.
- **`POST /v1/components/version`.** Same correction, same shape: `VersionChangeResultDTO` gains an additive optional `operationId` field (it had none before — the baseline never anticipated this route being slow). `success`/`requiresRestart` move to the terminal operation result; the `200` response returns once the existing in-flight-download admission check passes. Operation `type: "version-change"`. The `429 download_in_progress` guard is **preserved unchanged** (D-006: preserve rate-limiting intent) — a second request against the same server while one operation is already running still gets `429`, exactly as `isDownloadingJar` already refused it; it now names a real operation instead of a bare boolean flag.

**What does *not* change.** Every other route in §1 stays exactly as fast as MSC 1's own handler — a local file read, a config write, a metadata lookup. None of them cost the class of time `operation-model.md` exists for, so none of them get an `operationId`. `POST /v1/templates`'s create-from-template action, in particular, was checked against this bar specifically because it superficially resembles server creation: `templateMutationProvider`'s `createServer` case (`AppViewModel+APIWiringServerMgmt.swift`, per P7.6's citation) copies an already-downloaded template jar off local disk — no network, no installer subprocess — so it stays synchronous.

---

## 3. New route: `POST /v1/java-runtimes/install`

Already committed to in P7.1's dated addendum to D-006 (`msc2-decisions.md`) and detailed in `phase7-scope.md`'s "Java runtime install" section — Cameron's answer to "Questions before P7.1" was (a): MSC 2 installs Java itself rather than only detecting and reporting it. This step is where that commitment becomes a real, frozen route.

- **No MSC 1 equivalent.** `JavaInstaller.swift`'s `installerURL`/`downloadInstaller` (line 75) fetch a macOS-only Temurin `.pkg` and hand it to `Installer.app` for a human to double-click — a GUI, same-machine mechanism this route does not port. This is greenfield agent behavior, the same weight `operation-model.md` itself carries as a whole document (recorded Proposed, not extracted from a Swift source).
- **Request is host-relative, not host-specified.** `JavaRuntimeInstallRequestDTO` carries only `major` (one of the four values `JavaInstaller.minecraftInstallOptions` already offers — 8/17/21/25, `fixtures/java-runtime-selection/`). There is no `os`/`arch` field: per D-009 (MSC 2 owns its own runtime state, never reaches into a system package manager or another machine's filesystem), the agent always installs a runtime for **its own host**, never a remote client's.
- **Always async — no synchronous variant exists.** Unlike `create`/`version-change` above (fast for four of six families, slow for two), a managed Java install is *always* a real network download plus checksum verification plus unpack. This is the same shape `phase6-api.md` §3 gives `POST /v1/worlds/convert`: `operationId` is **required** on `JavaRuntimeInstallResultDTO`, not optional, because there is no fast path a client could be tempted to assume. Operation `type: "java-download"` — the exact name `operation-model.md` §2 already listed as an anticipated future value, reused rather than inventing a near-duplicate.
- **Error vocabulary reused, not invented.** `400 invalid_body` for a bad/unsupported `major`; `429 download_in_progress` for a second install request against a major already installing — the same code `/v1/components/version` uses, not a new one, since it means the same thing (an in-flight download this request collided with); `500 internal_error` for anything else. No new `ErrorDTO.code` value.
- **Rollback discipline matches server creation.** `phase7-scope.md` already commits P7.16 to "an interrupted-install-leaves-nothing-behind guarantee, the same rollback discipline P7.17/P7.18 already apply to server creation" — this route's contract states that guarantee; P7.16/P7.24 are where it's actually built.
- **Permission category: `settings`, decided for you.** MSC 1 has no analog to confirm against, so this is a fresh call rather than a confirmation. `settings` was chosen over `fleet` because the closest existing sibling, `POST /v1/config/java-runtime` (sets the global Java executable override), is host-level configuration, not server-fleet management — installing a runtime the agent will offer to every server is the same kind of host-level state, not a per-server mutation. Nothing here is visible or different to Cameron either way; noted so the reasoning is on record, not asked as a question.

`EXPECTED_TOTAL` in `tools/api-contract-check.py` moves from 106 to 107 (one new operation); `--selftest` and `--v1-summary` both still pass.

---

## 4. Cancellation semantics for a running installer

`operation-model.md` §4.3's generic `POST /v1/operations/{id}/cancel` already exists and needs no new design for its wire shape — this section only says what "cooperative cancellation" means for the three operation types this phase adds (`server-create`, `version-change`, `java-download`), since none of them existed when that document was written.

For all three, the cooperative flag the cancel route sets must be checked at the same points a hard failure already is: **between** the network download/installer-subprocess step and the next one, and, for the two install-step families, at the points `NeoForgeInstaller.install`/`ForgeInstaller.install`'s own subprocess (`runJavaInstaller`, `NeoForgeInstaller.swift:261`) is running — a cancellation there kills the child `java -jar <installer> --installServer` process rather than waiting for it to finish on its own. This is a **deliberate strengthening over the oracle**, not a port: P7.5 already found that MSC 1's installers never clean up after a non-zero exit or a missing post-install args file (the downloaded installer jar, and for NeoForge its log, are simply left behind). A cancelled Phase 7 operation must leave the same clean state a *failed* one does — for `server-create`, the full directory removed (this phase's own working exit criteria: "every failed create rolls its directory back completely, leaving no half-provisioned server behind"); for `version-change`, the previous jar left untouched and any partially-downloaded replacement discarded; for `java-download`, nothing left under MSC's runtimes directory. This note fixes the *contract-level guarantee* only — which bytes get deleted in which order is P7.13 (jar-provider boundary) and P7.14 (loader-installer runner)'s job, both flagged `stop-after` in the batch plan specifically because this is where MSC 2 first touches the network and first runs a third-party installer.

---

## 5. `capability_unavailable` for Bedrock creation: reused, not a new code

`versioning-and-errors.md` §5 already anticipates a small, open `code` vocabulary; `capability_unavailable` itself already exists — Phase 6 added it (`phase6-api.md` §5) for `POST /v1/backups/restore`'s Bedrock-runtime guard. Phase 7 reuses the exact same code for the exact same reason, on a second route: `POST /v1/servers/create` with `serverType: "bedrock"` returns `409` with `ErrorDTO.code: "capability_unavailable"` rather than half-provisioning a server directory no runtime (until Phase 10) can start. This is the phase's own working exit criterion, verbatim: *"Bedrock creation is refused with an advertised `capability_unavailable` until Phase 10, not faked."* No new error code, no new design — `openapi.json`'s `409` response for this route documents the reuse explicitly (§2's edit) so a reader doesn't have to guess whether `create_failed` or `capability_unavailable` fires for this specific case.

---

## 6. `POST /v1/health/repair`: this phase's action scope

`repairHealthProblemProvider` (`AppViewModel+APIWiringBackupsHealth.swift:186-245`) accepts four `action` values — `update`, `install`, `disable`, `delete` — but only the last two are this phase's job. `update`/`install` call `repairIncompatibleAddon`/`installMissingDependency` (P7.8's own citation), which are genuinely asynchronous (they spawn a `Task` hitting the Modrinth API) and operate on **add-ons**, not the server JAR — squarely inside "Add-ons, modpacks, and the rest of `/v1/components`… stay Phase 8," this phase's own "Not in this phase" note. `disable`/`delete` operate on whatever `installedJarStem` names, synchronously (a file rename or removal), and are Phase 7's to build.

**No schema change required.** `HealthRepairRequestDTO.action` is already an unconstrained `string`, not a closed enum, in the frozen contract — there's no field to narrow. What's added here is scope, not shape: Phase 7's `repairHealthProblemProvider` port implements `disable`/`delete` for real and returns the existing `action_unavailable` 400 for `update`/`install` unconditionally this phase (not conditionally on the problem's own `kind`/`installedFile` fields, the way MSC 1 does — Phase 7 has no add-on repair machinery at all yet to condition on). Phase 8 is where `update`/`install` get built and this scope note stops applying.

---

## 7. `helpId` and error-envelope: nothing new to design

`GET /v1/health`'s `HealthCardDTO.helpId` and `GET /v1/health/problems`'s `StartupProblemDTO.helpId` are already in the `HELPID_FIELDS` set `tools/api-contract-check.py` checks (from P2.2), and P7.8 already assigned real values for the four Phase-7-owned health cards (`health.directory`, `health.java`, `health.ram`, `health.last-startup`) and all five `StartupProblemKind` cases (`diagnostics.crash.<kind>`) into those fixtures' `expected`. This step's own `--v1-summary` run (§ Verify) confirms `missing-helpid: 0` — nothing here needed a new schema field or a new `helpid-contract.md` entry. Every non-2xx response across all eighteen routes already resolves to `ErrorDTO` (checked mechanically by the same run); the two operationId additions (§2) don't touch error responses at all.

---

## 8. The client capability matrix

Same methodology `phase6-api.md` §7 established, re-applied here rather than re-derived: status cells are **assessed as of this commit**, grounded in what `crates/msc-agent/src/main.rs::build_app()` and `crates/msc-agent/src/cli/mod.rs` actually mount today, not what this phase intends to build.

- **`agent_status`.** `build_app()` mounts exactly one of these eighteen routes today: `GET /v1/health`, still returning P2's canned `demo-card` (`routes/health.rs`'s own doc comment: *"a single canned health card, standing in for real health-check detection"*) — unchanged by this step, since P7.19–P7.22 replace that handler, not P7.9. It stays `Implemented` (the route exists and answers, even if the answer is a placeholder — the same standard P6.8 applied). Every other row, including the new `java-runtimes/install` row, is `Planned`: P7.9 freezes the contract, P7.13–P7.24 build the services and wire the routes behind it.
- **`desktop_web_status`** stays `Planned` on every row without exception, per `rolling-plan.md`'s own Phase 7 preamble — the same blanket rule Phase 6 applied, not a fresh decision.
- **`ios_status`/`cli_status`** stay `Planned` on every row — neither the copied iOS client nor the CLI has been repointed at any of these eighteen operations yet (P7.23–P7.26 do that); nothing to ground an `Implemented` cell in today.
- **No row uses `Intentional exception`.** Nothing here is a capability some client permanently won't get — every `Planned` row is a later step in this same phase's own step list, not an approved gap.

Two existing rows (`POST /v1/servers/create`, `POST /v1/components/version`) gain their `operation_id` value (previously blank) and a `notes` entry pointing at §2's correction; the new `POST /v1/java-runtimes/install` row is added in full. `tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv` (unchanged tool, still generic over any `openapi.json`) confirms shape and coverage: 109 rows now (107 `openapi.json` operations + 2 WebSocket channels), no blank required cell, no unapproved exception.

---

## Verify

```
python3 tools/api-contract-check.py --v1-summary && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv
```
