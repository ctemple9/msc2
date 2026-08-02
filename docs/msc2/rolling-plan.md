# MSC 2 — Rolling Plan

> ## STATUS: Phase 3 planned (20 steps written; two blocking Open items — D-025 service identity, D-012's Linux secret-storage gap — scoped as the phase's own first steps, not silently resolved) — Read next
> **Next move:** READ (Cameron reviews the Phase 3 plan — P3.1–P3.3's judgment calls are the ones that matter most before execution starts)
> **Repo:** https://github.com/ctemple9/msc2 · CI green on macOS, Linux, Windows
> **Last updated:** 2026-08-01

---

## How this document works

This is the **working state** of the build. The vision documents say where we're going; the port plan says in what order; this file says **where we actually are right now**.

Phases are fixed and come from `msc2-port-plan.md`. **Steps are written one phase at a time**, as we reach each phase — not up front. Steps written today for Phase 8 would be guesses.

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
| 3 | Safety substrate | planned |
| 4 | Java lifecycle vertical slice | not started |
| 5 | Configuration and migration | not started |
| 6 | Worlds and backups | not started |
| 7 | Server families and provisioning | not started |
| 8 | Mods, plugins, modpacks | not started |
| 9 | Networking and helpers | not started |
| 10 | Bedrock runtimes | not started |
| 11 | Desktop and web clients | not started |
| 12 | Terminal UI (deferred from v1) | not started |

---

## Setup

### S.1 — Create the repository and land the documents
**Status:** awaiting verification
**Files:** everything
**What:** `git init`, vision docs into `docs/msc2/`, audit artifacts into `docs/msc2/audit/`, `CLAUDE.md` + `AGENTS.md`, this file, README, `.gitignore`.
**Verify:** `cd ~/msc2 && ls docs/msc2/ && git log --oneline` → five vision docs + rolling-plan present, commits exist
**Commit:** `e0771ed`

### S.2 — Publish to GitHub
**Status:** awaiting verification
**What:** Created the public `msc2` repository and pushed `main`.
**Verify:** open https://github.com/ctemple9/msc2 — README renders, 19 files, docs/msc2/ browsable
**Commit:** _(n/a — push only)_

### S.3 — CI skeleton
**Status:** awaiting verification
**What:** `.github/workflows/ci.yml`. Two jobs — `repo-invariants` (CLAUDE.md/AGENTS.md must not drift; all six controlled documents must exist) and `toolchain` (macOS + Linux + Windows, installs Rust, builds once `Cargo.toml` appears).
**Verify:** `cd ~/msc2 && gh run list --limit 1` → shows `success`. Or the green check at https://github.com/ctemple9/msc2/actions
**Commit:** `S.3` — all four jobs passed on first run

### S.4 — Shared VS Code configuration
**Status:** awaiting verification
**Files:** `.vscode/extensions.json`, `.vscode/settings.json`
**What:** Extension recommendations (rust-analyzer, TOML) so the workspace configures itself on open. Whitespace/final-newline hygiene to keep diffs clean, markdown wrapping, Rust format-on-save so `cargo fmt --check` never fails in CI for an avoidable reason.
**Note:** the rust-analyzer extension ships no prebuilt language server for x86_64 macOS. Resolved by `rustup component add rust-analyzer` plus `"rust-analyzer.server.path": "rust-analyzer"` — portable via the rustup PATH shim, not a hard-coded home directory.
**Verify:** open `~/msc2` in VS Code, reload the window — no rust-analyzer error in the notifications
**Commit:** `S.4` (two commits)

### S.5 — Block AI attribution trailers
**Status:** awaiting verification
**Files:** `.githooks/commit-msg`, `.github/workflows/ci.yml`
**What:** Three layers. Claude Code's `attribution` setting suppresses them at the source (owner's global config, already in place). `.githooks/commit-msg` rejects them locally for any agent or human. CI scans the full history so a clone without hooks installed still can't land one on `main`.
**Verify:** `cd ~/msc2 && printf 'test\n\nCo-Authored-By: X <x@y.z>\n' > /tmp/m && .githooks/commit-msg /tmp/m; echo "exit $?"` → prints a rejection and `exit 1`
**Commit:** `S.5`

---

## Phase 0 — Freeze the baseline and build the harness

**Gate:** a fixture can be written, run, and compared. The API baseline exists. The symbol ledger exists.

**No Rust.** Tooling in this phase is Python (stdlib only — no dependency setup for Cameron to fight). `cargo`/`Cargo.toml` do not appear until Phase 1.

**Source oracle:** `~/Documents/Swift Projects/minecraft-server-controller` — read-only throughout, per `CLAUDE.md` rule 8.

52 steps, seven groups:

| Group | Steps | Deliverable |
|---|---|---|
| Fixture harness | P0.1–P0.2 | format spec + runner tool |
| Extract the 270 MSC 1 tests | P0.3–P0.21 | `fixtures/**/*.json`, one dir per source test file |
| Reference corpus | P0.22 | `corpus/` scaffold |
| API baseline | P0.23, P0.23a–P0.23s, P0.24, P0.30 | `docs/msc2/api-baseline/`, checker script + one step per route family + leftover routes |
| Symbol ledger | P0.25, P0.26, P0.26a, P0.27, P0.29 | `docs/msc2/audit/msc2-symbol-ledger.csv` |
| Sidecar IPC contract | P0.28 | `docs/msc2/sidecar-ipc-contract.md` |
| Gate corrections | P0.29–P0.32 | closes gaps found verifying the gate itself: a ledger coverage miss, an API baseline coverage miss, one open decision the audit surfaced but didn't resolve, and (from Codex's review) typed-failure response schemas |

---

### Fixture harness

### P0.1 — Fixture format spec
**Status:** DONE
**Files:** `docs/msc2/fixture-format.md`
**What:** Define the JSON shape every fixture file must have: `domain`, `case`, `source` (`file`, `test`, `line` — pointer back into MSC 1), `input`, `expected`, optional `notes`. Define the directory convention (`fixtures/<domain>/<case>.json`) that every later extraction step follows.
**Verify:** `grep -oE '"(domain|case|source|input|expected)"' docs/msc2/fixture-format.md | sort -u | wc -l` → `5`
**Commit:** `17bfc83`

### P0.2 — Fixture runner and comparison tool
**Status:** DONE
**Files:** `tools/fixture-runner/run.py`, `tools/fixture-runner/schema.json`, `fixtures/_selftest/pass.json`, `fixtures/_selftest/fail.json`
**What:** A dependency-free Python script that validates a fixture against the P0.1 schema and compares `input`→`expected` against an `actual` value, exiting 0 on match and non-zero on mismatch. Two self-test fixtures prove the pipeline end-to-end before any real domain logic exists: one built to pass, one built to fail. This is what makes the Phase 0 gate ("a fixture can be written, run, and compared") checkable today, without Rust. Three CLI modes: plain `run.py <file>` (full compare, used from Phase 1 on); `--schema-only <file>` (shape check against the P0.1 schema, no `actual` required — what P0.3–P0.21 use per-file, since no Rust exists yet); `--validate-dir <dir> --expect <n>` (schema-only over every fixture in a directory, plus a count assertion, printing `ok <n>` — what P0.3–P0.21 use as their Verify line); and `--selftest` (runs both self-test fixtures above and reports each exit code — this step's own Verify).
**Verify:** `python3 tools/fixture-runner/run.py --selftest` → `pass=0` then `fail=1`
**Commit:** `P0.2: build fixture runner and comparison tool`

---

### Extract the 270 MSC 1 tests

Mechanical: pull each test file's inline Swift literals into `input`/`expected` JSON pairs under the P0.1 format. One step per source file, matching MSC 1's own grouping. Sums to 270 across P0.3–P0.21, matching MSC 1's documented test count exactly (verified against the live source: `grep -rc 'func test' MSCmacOSTests/*.swift` totals 270).

**Domain / pure-logic tests**

### P0.3 — Extract TPS parser fixtures
**Status:** DONE
**Files:** `fixtures/tps/`
**What:** Pull the 27 TPS test cases out of MSC 1's `TpsMonitoringTests.swift` into input/expected JSON pairs.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/tps --expect 27` → `ok 27`
**Commit:** `P0.3: extract TPS parser fixtures`

### P0.4 — Extract version-comparison fixtures
**Status:** DONE
**Files:** `fixtures/component-version/`
**What:** Pull the 21 test cases out of `ComponentVersionParsingTests.swift` (component/version-string parsing and comparison).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/component-version --expect 21` → `ok 21`
**Commit:** `P0.4: extract version-comparison fixtures`

### P0.5 — Extract Java runtime policy fixtures
**Status:** DONE
**Files:** `fixtures/java-runtime-guards/`
**What:** Pull the 15 test cases out of `JavaRuntimeGuardsTests.swift` (Java version/runtime selection guards).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/java-runtime-guards --expect 15` → `ok 15`
**Commit:** `P0.5: extract Java runtime policy fixtures`

### P0.6 — Extract server-properties model fixtures
**Status:** DONE
**Files:** `fixtures/server-properties/`
**What:** Pull the 7 test cases out of `ServerPropertiesModelTests.swift`.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/server-properties --expect 7` → `ok 7`
**Commit:** `P0.6: extract server-properties model fixtures`

### P0.7 — Extract settings-schema fixtures
**Status:** DONE
**Files:** `fixtures/settings-schema/`
**What:** Pull the 16 test cases out of `ServerSettingsSchemaTests.swift`.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/settings-schema --expect 16` → `ok 16`
**Commit:** `P0.7: extract settings-schema fixtures`

### P0.8 — Extract connector crash-analysis fixtures
**Status:** DONE
**Files:** `fixtures/connector-crash-analysis/`
**What:** Pull the 11 test cases out of `ConnectorCrashAnalysisTests.swift` (Forge dependency-block parsing, connector entrypoint failure attribution).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/connector-crash-analysis --expect 11` → `ok 11`
**Commit:** `P0.8: extract connector crash-analysis fixtures`

### P0.9 — Extract startup crash-analyzer fixtures
**Status:** DONE
**Files:** `fixtures/startup-crash-analyzer/`
**What:** Pull the 7 test cases out of `StartupCrashAnalyzerTests.swift` (Fabric/Forge missing- and wrong-dependency-version attribution).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/startup-crash-analyzer --expect 7` → `ok 7`
**Commit:** `P0.9: extract startup crash-analyzer fixtures`

### P0.10 — Extract args-file resolution fixtures
**Status:** DONE
**Files:** `fixtures/args-file-resolution/`
**What:** Pull the 12 test cases out of `ArgsFileResolutionTests.swift` (NeoForge `@args`-file version resolution and fallback).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/args-file-resolution --expect 12` → `ok 12`
**Commit:** `P0.10: extract args-file resolution fixtures`

**API / wire-contract tests**

### P0.11 — Extract DTO contract fixtures
**Status:** DONE
**Files:** `fixtures/dto-contract/`
**What:** Pull the 30 test cases out of `DTOContractTests.swift` — the wire-format shape MSC 2's OpenAPI baseline (P0.23) must match.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/dto-contract --expect 30` → `ok 30`
**Commit:** `P0.11: extract DTO contract fixtures`

### P0.12 — Extract HTTP request-parsing fixtures
**Status:** DONE
**Files:** `fixtures/http-parse-request/`
**What:** Pull the 16 test cases out of `HTTPParseRequestTests.swift`.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/http-parse-request --expect 16` → `ok 16`
**Commit:** `P0.12: extract HTTP request-parsing fixtures`

### P0.13 — Extract Remote API integration fixtures
**Status:** DONE
**Files:** `fixtures/remote-api-integration/`
**What:** Pull the 12 test cases out of `RemoteAPIIntegrationTests.swift`.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/remote-api-integration --expect 12` → `ok 12`
**Commit:** `P0.13: extract Remote API integration fixtures`

### P0.14 — Extract network-safety fixtures
**Status:** DONE
**Files:** `fixtures/network-safety/`
**What:** Pull the 13 test cases out of `NetworkSafetyTests.swift` (loopback/mDNS/private-range classification).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/network-safety --expect 13` → `ok 13`
**Commit:** `P0.14: extract network-safety fixtures`

### P0.15 — Extract config round-trip fixtures
**Status:** DONE
**Files:** `fixtures/config-roundtrip/`
**What:** Pull the 7 test cases out of `AppConfigRoundTripTests.swift` (`AppConfig`/`ConfigServer` encode-decode round trips, including missing-optional-field defaulting).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/config-roundtrip --expect 7` → `ok 7`
**Commit:** `P0.15: extract config round-trip fixtures`

**Mods, plugins, modpacks tests**

### P0.16 — Extract CurseForge modpack fixtures
**Status:** DONE
**Files:** `fixtures/curseforge-modpack/`
**What:** Pull the 16 test cases out of `CurseForgeModpackTests.swift`.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/curseforge-modpack --expect 16` → `ok 16`
**Commit:** `P0.16: extract CurseForge modpack fixtures`

### P0.17 — Extract modpack client-only classification fixtures
**Status:** DONE
**Files:** `fixtures/modpack-client-only/`
**What:** Pull the 18 test cases out of `ModpackClientOnlyTests.swift` (manifest-env and Modrinth-side/CurseForge-side client-only detection).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/modpack-client-only --expect 18` → `ok 18`
**Commit:** `P0.17: extract modpack client-only classification fixtures`

### P0.18 — Extract modpack pinning fixtures
**Status:** DONE
**Files:** `fixtures/modpack-pinning/`
**What:** Pull the 13 test cases out of `ModpackPinningTests.swift` (Forge Maven version listing, dedup, sort).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/modpack-pinning --expect 13` → `ok 13`
**Commit:** `P0.18: extract modpack pinning fixtures`

### P0.19 — Extract `.mrpack` extraction fixtures
**Status:** DONE
**Files:** `fixtures/mrpack-extraction/`
**What:** Pull the 3 test cases out of `MrpackExtractionTests.swift` (archive permission-mode handling, missing/malformed manifest).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/mrpack-extraction --expect 3` → `ok 3`
**Commit:** `P0.19: extract .mrpack extraction fixtures`

### P0.20 — Extract pack-managed guard fixtures
**Status:** DONE
**Files:** `fixtures/pack-managed-guard/`
**What:** Pull the 7 test cases out of `PackManagedGuardTests.swift` (pack-managed provenance round trip, old-JSON compatibility).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/pack-managed-guard --expect 7` → `ok 7`
**Commit:** `P0.20: extract pack-managed guard fixtures`

**Provisioning tests**

### P0.21 — Extract headless script generator fixtures
**Status:** DONE
**Files:** `fixtures/headless-script/`
**What:** Pull the 19 test cases out of `HeadlessScriptGeneratorTests.swift` (Paper/Fabric/Forge launch-script and args-file generation).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/headless-script --expect 19` → `ok 19`
**Commit:** `P0.21: extract headless script generator fixtures`

---

### Reference corpus

### P0.22 — Reference corpus scaffold
**Status:** DONE
**Files:** `corpus/README.md`, `corpus/{logs,configs,packs,server-dirs,dto-examples}/`
**What:** Create the five category directories the port plan calls for (§1, "Reference corpus"). Seed `dto-examples/` with real wire-format JSON already embedded in `DTOContractTests.swift` and `RemoteAPIIntegrationTests.swift` — genuine evidence, not fabricated. `corpus/README.md` states plainly, per category, what still has to come from Cameron's own MSC 1 usage (real server directories, historical `server_config_swift.json` versions, real crash logs) rather than inventing sample data — MSC 1's own repository ships none.
**Verify:** `find corpus -mindepth 1 -maxdepth 1 -type d | sort` → `corpus/configs`, `corpus/dto-examples`, `corpus/logs`, `corpus/packs`, `corpus/server-dirs`
**Commit:** `P0.22: scaffold reference corpus`

---

### API baseline

Split by route family (per `msc2-engineering.md` §5: "Route families: `servers/{create,import,delete,rename,eula}` · `settings` · `worlds/{create,rename,replace,repair,activate}` · `components/{install,remove,update,version}` · `backups/{now,restore,config}` · `config/{ram,java-runtime,geyser}` · `users/{create,update,revoke}` · `health/repair` · `playit/*` · `broadcast/*` · `resourcepacks/*` · `watchdog/*` · `command` · `start` · `stop` · `allowlist` · `players/*` · `duckdns` · `templates`"), rather than one step authoring all 87 routes at once. All 19 family steps build the same shared file, `docs/msc2/api-baseline/openapi.json`, incrementally, using the checker script P0.23 builds first.

Each family step's Verify checks two things a flat path count can't: it looks only at the paths that actually belong to that family, and it checks **schema depth** — that every operation's `responses` actually nests down to `content` → `application/json` → `schema`, not a stub `{}`. A family passing the old "87 paths exist" check could still have empty response bodies; this can't.

For the fourteen families MSC 1's own route list gives an exact sub-route count for (`servers`=5, `settings`=1, `worlds`=5, `components`=4, `backups`=3, `config`=3, `users`=3, `health`=1, `command`=1, `start`=1, `stop`=1, `allowlist`=1, `duckdns`=1, `templates`=1), that count is asserted. For the five `*`-wildcard families (`playit`, `broadcast`, `resourcepacks`, `watchdog`, `players`), MSC 1's own docs don't state a sub-route count, so none is asserted — the script only checks count > 0 and prints whatever it finds, same principle as P0.25/P0.27 below.

### P0.23 — API baseline schema-depth checker script
**Status:** DONE
**Files:** `tools/api-baseline-check.py`
**What:** A dependency-free Python script, `tools/api-baseline-check.py <family>`, used as the Verify command by every P0.23a–P0.23s step below. It loads `docs/msc2/api-baseline/openapi.json`, filters to the paths under `/<family>`, asserts the count against the known table above (or just `> 0` for the five wildcard families), asserts every matched operation's `responses` nests down to a real `content` → `application/json` → `schema` rather than a stub, and prints `ok <n>` on success — exiting non-zero with a one-line reason otherwise. Ships with a `--selftest` mode against two bundled fixtures (one deep, one stub) so it's checkable before `openapi.json` exists, and a `--total` mode that sums every path in the file (the P0.23s final sanity check against 87).
**Verify:** `python3 tools/api-baseline-check.py --selftest` → `pass=0` then `fail=1`
**Commit:** `P0.23: build API baseline schema-depth checker script`

### P0.23a — API baseline: `servers` routes
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `servers/{create,import,delete,rename,eula}` routes, read from the relevant `RemoteAPIServer*.swift` file(s) and `RemoteAPIServerDTOs.swift`. Behavior as MSC 1 has it, not aspirational.
**Verify:** `python3 tools/api-baseline-check.py servers` → `ok 5`
**Commit:** `P0.23a: add servers API baseline routes`

### P0.23b — API baseline: `settings` route
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `settings` route. Never executed at the time — this route was still missing when P0.30 audited the full route table for coverage gaps, so P0.30 added it (`GET /settings` + `POST /settings`) alongside the other 23 it found. No separate work happened for this step; its status is corrected here rather than left at `not started` next to a route that now exists.
**Verify:** `python3 tools/api-baseline-check.py settings` → `ok 1`
**Commit:** `P0.30: add the 24 API baseline routes missed by the route-family steps`

### P0.23c — API baseline: `worlds` routes
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `worlds/{create,rename,replace,repair,activate}` routes.
**Verify:** `python3 tools/api-baseline-check.py worlds` → `ok 5`
**Commit:** `P0.23c: add worlds API baseline routes`

### P0.23d — API baseline: `components` routes
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `components/{install,remove,update,version}` routes.
**Verify:** `python3 tools/api-baseline-check.py components` → `ok 4`
**Commit:** `P0.23d: add components API baseline routes`

### P0.23e — API baseline: `backups` routes
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `backups/{now,restore,config}` routes.
**Verify:** `python3 tools/api-baseline-check.py backups` → `ok 3`
**Commit:** `P0.23e: add backups API baseline routes`

### P0.23f — API baseline: `config` routes
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `config/{ram,java-runtime,geyser}` routes.
**Verify:** `python3 tools/api-baseline-check.py config` → `ok 3`
**Commit:** `P0.23f: add config API baseline routes`

### P0.23g — API baseline: `users` routes
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `users/{create,update,revoke}` routes.
**Verify:** `python3 tools/api-baseline-check.py users` → `ok 3`
**Commit:** `P0.23g: add users API baseline routes`

### P0.23h — API baseline: `health/repair` route
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `health/repair` route.
**Verify:** `python3 tools/api-baseline-check.py health` → `ok 1`
**Commit:** `P0.23h: add health/repair API baseline route`

### P0.23i — API baseline: `playit` routes
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `playit/*` routes. MSC 1's docs don't state an exact sub-route count for this family — read it straight from the source instead of assuming one.
**Verify:** `python3 tools/api-baseline-check.py playit` → `ok 3` (recorded from the live source: GET /playit, POST /playit/start, POST /playit/stop — not an assumed number)
**Commit:** `P0.23i: add playit API baseline routes`

### P0.23j — API baseline: `broadcast` routes
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `broadcast/*` routes. Sub-route count not stated in the docs — read it from the source.
**Verify:** `python3 tools/api-baseline-check.py broadcast` → `ok 10` (recorded from the live source, not an assumed number)
**Commit:** `P0.23j: add broadcast API baseline routes`

### P0.23k — API baseline: `resourcepacks` routes
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `resourcepacks/*` routes. Sub-route count not stated in the docs — read it from the source.
**Verify:** `python3 tools/api-baseline-check.py resourcepacks` → `ok 5` (recorded from the live source, not an assumed number)
**Commit:** `P0.23k: add resourcepacks API baseline routes`

### P0.23l — API baseline: `watchdog` routes
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `watchdog/*` routes. Sub-route count not stated in the docs — read it from the source.
**Verify:** `python3 tools/api-baseline-check.py watchdog` → `ok 3` (recorded from the live source, not an assumed number)
**Commit:** `P0.23l: add watchdog API baseline routes`

### P0.23m — API baseline: `command` route
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `command` route.
**Verify:** `python3 tools/api-baseline-check.py command` → `ok 1`
**Commit:** `P0.23m: add command API baseline route`

### P0.23n — API baseline: `start` route
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `start` route.
**Verify:** `python3 tools/api-baseline-check.py start` → `ok 1`
**Commit:** `P0.23n: add start API baseline route`

### P0.23o — API baseline: `stop` route
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `stop` route.
**Verify:** `python3 tools/api-baseline-check.py stop` → `ok 1`
**Commit:** `P0.23o: add stop API baseline route`

### P0.23p — API baseline: `allowlist` route
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `allowlist` route.
**Verify:** `python3 tools/api-baseline-check.py allowlist` → `ok 1`
**Commit:** `P0.23p: add allowlist API baseline route`

### P0.23q — API baseline: `players` routes
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `players/*` routes. Sub-route count not stated in the docs — read it from the source.
**Verify:** `python3 tools/api-baseline-check.py players` → `ok 4` at the time (recorded from the live source, not an assumed number). Now `ok 5` — P0.30 added the fifth, `GET /players/{profileId}/skin`, which this wildcard family's own `players/` prefix also matches. Non-breaking (wildcard families only assert count > 0), noted for anyone re-running this line and expecting the original 4.
**Commit:** `P0.23q: add players API baseline routes`

### P0.23r — API baseline: `duckdns` route
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `duckdns` route.
**Verify:** `python3 tools/api-baseline-check.py duckdns` → `ok 1`
**Commit:** `P0.23r: add duckdns API baseline route`

### P0.23s — API baseline: `templates` route
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `templates` route. This is the last family step; once it lands, the full file should contain all 87 routes MSC 1 exposes today (49 POST + 38 GET, per `msc2-engineering.md` §5) — worth a final sanity check with `python3 tools/api-baseline-check.py --total` (a mode the P0.23 script also provides) alongside this step's own depth check, since no single family step asserts the grand total.
**Verify:** `python3 tools/api-baseline-check.py templates` → `ok 1`
**Commit:** `P0.23s: add templates API baseline route`

### P0.24 — Capture the WebSocket event schema
**Status:** DONE
**Files:** `docs/msc2/api-baseline/websocket-events.json`
**What:** MSC 1 has exactly one real-time WebSocket channel, not six — read from `RemoteAPIServer+WebSocket.swift`, the upgrade dispatch in `RemoteAPIServer+HTTP.swift`, and `consoleBuffer`/broadcast in `RemoteAPIServer.swift`. Document `console` (`/console/stream`): the RFC 6455 upgrade handshake (Sec-WebSocket-Key → accept key); auth (the same Bearer-token check as every HTTP route — any authenticated role may connect, no extra permission gate on the GET); the `ConsoleLineDTO` payload (`ts`, `source`, `level?`, `text`), one per text frame; the bounded-history-then-live delivery model (200-line backfill via `tailConsoleLines(n: 200)` sent immediately on connect, then live lines as they arrive); the 5000-line ring buffer (`consoleBufferLimit`) console history is capped at; ping/pong/close frame handling; the 64 KB inbound frame cap (`maxWebSocketClientFrameBytes`); and why inbound text frames are intentionally ignored (one-way — the server never executes WS-received text as a command). `status`/`operation progress`/`players`/`notifications`/`metrics` are **not** WebSocket channels in MSC 1 — those are HTTP-polled (`GET /status`, `GET /players`, etc.). The "six channels" language in `msc2-engineering.md` §5 describes MSC 2's intended design, not MSC 1's baseline; per D-006 the api-baseline captures MSC 1 as it is, and extensions are designed in Phase 2, not invented here. See the Amendments log.
**Verify:** `python3 -c "import json;d=json.load(open('docs/msc2/api-baseline/websocket-events.json'));print(len(d['channels']))"` → `1`
**Commit:** `P0.24: capture the one real WebSocket channel, not six`

---

### Symbol ledger

### P0.25 — Symbol ledger schema and UI density scanner
**Status:** DONE
**Files:** `docs/msc2/audit/symbol-ledger-format.md`, `tools/symbol-scan/scan.py`
**What:** Define the ledger's columns (`file`, `bucket`, `symbol`, `kind` [parser/policy/workflow], `disposition` [agent/client], `target_domain`, `source_line`, `notes`) — one row per agent-owned symbol found inside a Mixed or UI file, per D-016. Build the density scanner the reconciliation audit already used (`msc2-audit-reconciliation.md`, "D1 — The Mixed bucket"): grep MSC 1's UI-bucket files (`msc2-codex-file-inventory.csv`, `bucket=ui`) for `FileManager`, `Process(`, `URLSession`, `func parse*/detect*/validate*/resolve*`, `JSONDecoder`, string-range extraction, and rank by hit count, output one file per line sorted by hit count descending. This is a live scan, not a check against the reconciliation doc's earlier count of 15 — that count may be stale, so whatever the scan finds is the number, and P0.27 records it rather than assuming 15.
**Verify:** `python3 tools/symbol-scan/scan.py --bucket ui --min-hits 3 "$HOME/Documents/Swift Projects/minecraft-server-controller"` → a ranked, non-empty file list; note the count shown
**Commit:** `P0.25: build symbol ledger schema and UI density scanner`

### P0.26 — Populate the ledger: Mixed-bucket files
**Status:** DONE
**Files:** `docs/msc2/audit/msc2-symbol-ledger.csv`
**What:** For every file Codex's reconciled inventory marks `bucket=mixed` (59 files, `msc2-codex-file-inventory.csv`), open it in MSC 1 and add one ledger row per parser/policy/workflow symbol, using the deletion test in `msc2-port-plan.md` §1 to decide agent vs. client. A file with genuinely nothing to extract still gets one row saying so — coverage must be provable, not assumed. 293 rows across all 59 files (one file, `AppViewModel+FinderTools.swift`, had nothing to extract and got the single `(none)` row the coverage rule requires).
**Verify:** `python3 -c "import csv;rows=list(csv.DictReader(open('docs/msc2/audit/msc2-symbol-ledger.csv')));print(len({r['file'] for r in rows if r['bucket']=='mixed'}))"` → `59`
**Commit:** `P0.26: populate the symbol ledger for mixed-bucket files`

### P0.26a — Symbol ledger bucket-count checker script
**Status:** DONE
**Files:** `tools/symbol-ledger-check.py`
**What:** A dependency-free Python script, `tools/symbol-ledger-check.py <bucket> --scan-source <path>`, used as P0.27's Verify command. It counts unique `file` values in `docs/msc2/audit/msc2-symbol-ledger.csv` for the given `bucket`, re-runs P0.25's scanner (`tools/symbol-scan/scan.py --bucket ui --min-hits 3`) against `--scan-source`, asserts the two counts match exactly, and prints `ok <n>` — so the check stays live against whatever the scanner currently finds, never a number frozen in the plan. Ships with a `--selftest` mode against two bundled temp CSVs (one matching, one deliberately short a row) so it's checkable before the real ledger or a scan source exists.
**Verify:** `python3 tools/symbol-ledger-check.py --selftest` → `pass=0` then `fail=1`
**Commit:** `P0.26a: build symbol ledger bucket-count checker script`

### P0.27 — Populate the ledger: flagged UI files
**Status:** DONE
**Files:** `docs/msc2/audit/msc2-symbol-ledger.csv`
**What:** For every file P0.25's scanner actually flagged at ≥3 hits (includes the already-known `OverviewChatCardView.swift` console parser — but don't assume the reconciliation doc's earlier count of 15 still holds, since the source may have moved since that doc was written), open it and add ledger rows the same way. This is what turns "static scanning flags candidates" into an actual disposition record instead of a hunch. The live scan found 4 files, not 15 — `OverviewChatCardView.swift` is no longer among them because Codex's reconciled inventory already reclassifies it as `bucket=mixed` (covered under P0.26 instead). The 4 actually flagged: `CurseForgeManualDownloadSheet.swift`, `DetailsComponentsTabView.swift`, `ServerEditorJarsTab.swift`, `RouterPortForwardGuideReader.swift`.
**Verify:** `python3 tools/symbol-ledger-check.py ui-flagged --scan-source "$HOME/Documents/Swift Projects/minecraft-server-controller"` → `ok <n>` (live count, not fixed)
**Commit:** `P0.27: populate the symbol ledger for flagged UI files`

### P0.29 — Ledger gap: files adjudicated Mixed but missing from the ledger
**Status:** DONE
**Files:** `docs/msc2/audit/msc2-symbol-ledger.csv`
**What:** P0.26 selected its 59 files by filtering Codex's *raw* inventory (`msc2-codex-file-inventory.csv`) for `bucket=mixed`. That inventory was written before `docs/msc2/audit/msc2-audit-exact-diff.md` re-adjudicated 28 disputed files against Claude's independent audit. Files the diff moved *into* Mixed after the fact were never selected, so they never got ledger rows. Reconcile the whole 28-file adjudicated list (not just the two Cameron flagged) against the ledger's actual `file` column. Cross-checked programmatically: of the 28, exactly two — `MSCSettingsView.swift` and `ServerEditorView.swift` — are `Final: Mixed` with zero ledger rows. Every other Final-Mixed file in the diff already has rows, either because Codex's raw bucket already said `mixed` (picked up by P0.26) or because P0.25's density scanner already flagged it as a UI file (picked up by P0.27, under `bucket=ui-flagged` rather than `mixed` — a labeling difference, not a coverage gap, since P0.27 already extracted the agent-owned symbols). Add ledger rows for the two missing files using the deletion test, same as P0.26.
**Verify:** `python3 -c "import csv;rows=list(csv.DictReader(open('docs/msc2/audit/msc2-symbol-ledger.csv')));print(len({r['file'] for r in rows if r['bucket']=='mixed'}))"` → `61`
**Commit:** `P0.29: add MSCSettingsView.swift and ServerEditorView.swift to the symbol ledger`
**Batch:** solo

---

### Sidecar IPC contract

### P0.28 — macOS Bedrock sidecar IPC contract
**Status:** DONE
**Files:** `docs/msc2/sidecar-ipc-contract.md`
**What:** Read `VMBedrockServerBackend.swift` (451 lines) and write the process protocol the Rust agent will use to drive the macOS Bedrock sidecar — transport (JSON lines over stdio, or a unix socket — pick one and record why) plus one section per message type: provision, start, readiness signal, stop, force-stop, crash notification, console stream, command input, shared-directory mapping, host-directory persistence across VM replacement (`msc2-engineering.md` §9). A contract informed by what MSC 1's sidecar actually does today, not a fresh design. Chose JSON lines over stdio (1:1 parent-supervises-child relationship, no socket-file lifecycle to manage, EOF doubles as the crash signal). Notes one open question: whether `BedrockProvisioner.ensureInstalled`'s BDS-binary download belongs in this sidecar protocol at all, since it has no VM dependency.
**Verify:** `grep -c '^### ' docs/msc2/sidecar-ipc-contract.md` → `10`
**Commit:** `P0.28: write the macOS Bedrock sidecar IPC contract`

---

### API baseline correction

### P0.30 — API baseline: routes covered by no route-family step
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`, `tools/api-baseline-check.py`
**What:** The 19 P0.23a–s steps followed `msc2-engineering.md` §5's named route-family list, but that list itself omits several real routes MSC 1 exposes. Read the actual route table straight from source (`RemoteAPIServer+HTTP.swift`'s `switch (method, path)` at line 537, 87 cases, plus one dynamic route handled just above it via `path.hasPrefix`/`hasSuffix` at line 529 — `GET /players/{profileId}/skin`, which brings MSC 1's real total to 88, not the 87 `msc2-engineering.md` §5 states) and diff it against `openapi.json`'s current 64 (method, path) pairs. Exactly 24 are missing, zero are fabricated on either side: `GET /servers`, `GET /status`, `GET /performance`, `POST /active-server`, `GET /session-log`, `GET /console/tail`, `GET /components`, `GET /addons`, `GET /files`, `GET /files/read`, `GET /components/client-export`, `GET /catalog/search`, `GET /java-runtimes`, `GET /versions`, `GET /versions/create`, `GET /settings`, `POST /settings`, `GET /me`, `GET /worlds`, `GET /connectivity`, `GET /health`, `GET /health/problems`, `GET /backups`, `GET /players/{profileId}/skin`. Add all 24 at the same schema depth as the existing families — read each handler's response DTO from `RemoteAPIServerDTOs.swift` and either reuse an existing `components/schemas` entry where the type already exists (`WorldSlotsResponseDTO`/`WorldSlotDTO` for `GET /worlds`, `HealthProblemsResponseDTO`/`StartupProblemDTO` for `GET /health/problems`, `SimpleResult` for `POST /active-server`) or add a new one. `GET /settings` also finally satisfies P0.23b, which was never executed (still `not started`) — its status is corrected alongside this step rather than left stale now that the route exists.

**Side effect on five already-passing family checks.** Four of the new routes are bare/GET siblings of families whose sub-route count `tools/api-baseline-check.py` already asserts a fixed number for, and a fifth (`/health`) gains two new siblings under its prefix — adding them changes what the existing per-family checker legitimately finds. `KNOWN_COUNTS` is updated to match reality, each with an inline comment: `servers` 5→6 (adds bare `GET /servers`), `worlds` 5→6 (adds bare `GET /worlds`), `backups` 3→4 (adds bare `GET /backups`), `components` 4→6 (adds `GET /components` and `GET /components/client-export`), `health` 1→3 (adds `GET /health` and `GET /health/problems` alongside the existing `POST /health/repair`). Recorded in the Amendments log below, same pattern as the P0.24 amendment — nothing here silently invalidates a step Cameron already verified.

**New Verify tooling.** No single family prefix covers these 24 (they're scattered singleton routes), and re-running 5 separate already-passing family checks doesn't prove the new ones are real. Added a `--depth-all` mode to `tools/api-baseline-check.py`: walks every path/method in the whole file (not just one family) and asserts each has a real `content` → `application/json` → `schema`, printing `ok <n>` (total operations) or failing on the first stub found — a stronger, whole-document version of the same schema-depth check the family steps use, reusable for any future addition to this file.
**Verify:** `python3 tools/api-baseline-check.py --depth-all` → `ok 88`
**Commit:** `P0.30: add the 24 API baseline routes missed by the route-family steps`
**Batch:** safe

### P0.31 — Record the CurseForge manual-download problem as a decision
**Status:** DONE
**Files:** `docs/msc2/msc2-decisions.md`
**What:** MSC 1's `CurseForgeManualDownloadSheet.swift` handles mods CurseForge won't let the app auto-download: it opens each mod's download page in the user's browser, then watches a local folder (default `~/Downloads`) and moves matching jars into the server's `mods/` directory as they appear. That entire mechanism assumes the app, the browser, and the server's files all sit on one machine — true for MSC 1, not guaranteed for MSC 2, where the agent may be a headless box the user's browser never touches. This surfaced as an UNSURE item in the P0.27 symbol-ledger report (`CurseForgeManualDownloadSheet.swift`'s watch-folder mechanism) and isn't resolved by the deletion test alone — it isn't a question of which side owns existing behavior, it's that the existing behavior doesn't have a home on either side once agent and client are different machines. Record it in `msc2-decisions.md` as a new **Open** entry: describe the problem and lay out the real options (e.g. browser-side extension/helper, client-side download-and-upload, agent-side fetch if a direct URL becomes available, keeping it degraded to same-machine-only). Do not choose one — that's a product call for Cameron, not something to decide while doing ledger bookkeeping.
**Verify:** `grep -c '^## D-027' docs/msc2/msc2-decisions.md && grep -A2 '^## D-027' docs/msc2/msc2-decisions.md | grep -c 'Status:\*\* \*\*Open\*\*'` → `1` then `1`
**Commit:** `P0.31: record the CurseForge manual-download problem as decision D-027`
**Batch:** solo

### P0.32 — API baseline: fix typed-failure response schemas
**Status:** DONE
**Files:** `docs/msc2/api-baseline/openapi.json`, `tools/api-baseline-check.py`
**What:** Codex's review (2026-07-31 amendment below) found that many mutation routes' non-2xx responses point at the generic `Error` schema when MSC 1 actually sends the route's own typed result DTO — `sendJSON(statusCode: ..., encodable: result, ...)` uses the same typed object win or lose; only the synchronous pre-provider guards (`missing_body`, `invalid_json`, and field-required checks that run *before* the `Task` block) genuinely return `{"error": ...}`. Read every handler in `RemoteAPIServer+ComponentRoutes.swift`, `+UserRoutes.swift`, and the `/allowlist` case in `+HTTP.swift` to separate pre-provider guards (stay `Error`) from post-provider results (must be the typed DTO) across 27 route+method pairs: `/servers/{rename,delete,create,eula,import}`, `/templates`, `/players/{skin-override,hidden}`, `/allowlist`, `/users`, `/users/revoke`, `/users/update`, `/worlds/{create,rename,replace,repair}`, `/components/{update,remove,install,version}`, `/config/{ram,geyser}`, `/health/repair`, `/resourcepacks/{activate,seturl,toggle,remove}`. For every route, every 404/409/422/429/500 is unambiguously post-provider (confirmed per-route, no exceptions found) and gets fixed to the correct existing schema. 400 is mixed on some routes — a few guards run pre-provider (Error) while other 400 causes come from the provider's own result (typed) — so 400 is left as `Error` everywhere *except* `/servers/import` and `/templates`, where reading the handlers shows literally every 400 cause besides `missing_body`/`invalid_json` is post-provider, so those two get the typed schema for 400 too, noted in each response's `description`. This is a conscious, documented simplification, not a full resolution — see the Amendments log entry this step adds. Also adds a `--typed-failures` mode to `tools/api-baseline-check.py`: a curated table of (path, method, status code) pairs that must NOT be the generic `Error` schema, verified against source in this step, asserted against the live file so a future accidental revert is caught.
**Verify:** `python3 tools/api-baseline-check.py --typed-failures` → `ok 68` (the count of corrected status-code entries) — then `python3 tools/api-baseline-check.py --depth-all` → `ok 88` (unchanged path/method count, this step only touches response schemas)
**Commit:** `P0.32: fix typed-failure response schemas Codex's review caught`
**Batch:** solo

---

## Phase 1 — Domain types and pure rules

**Gate** (`msc2-port-plan.md` §3): Rust passes the Phase 0 pure fixtures. No user files touched.

**Rust starts here.** `Cargo.toml` did not exist through Phase 0 (Python stdlib only); P1.1 creates it. Everything in this phase lives in one crate, `msc-domain` — per `msc2-engineering.md` §6, domain types and parsers carry **no I/O**; that rule is load-bearing for two scoping calls this phase makes explicitly (P1.5, P1.10/12).

**Domain list, from `msc2-port-plan.md` §3:** "Server identity, flavors, version comparison, Java policy, property models, command catalog, TPS parsing, crash analysis, slug normalization, and the router rule engine (matcher, fallback resolver, composer, troubleshooting engine, runtime resolver)." All ten are accounted for below.

15 steps, five groups:

| Group | Steps | Deliverable |
|---|---|---|
| Rust workspace | P1.1–P1.2 | Cargo workspace, `msc-domain` crate, a native fixture-loading test harness |
| Existing-fixture domains | P1.3–P1.7 | version comparison, TPS parsing, Java runtime policy, property models, crash analysis + slug normalization — ported and passing their Phase 0 fixtures |
| New-characterization domains | P1.8–P1.9 | server identity & flavors, command catalog — MSC 1 has no tests for either, so fixtures are written from source before porting |
| Router rule engine | P1.10–P1.14 | matcher, fallback decision tree, composer, troubleshooting engine, runtime resolver — all five newly characterized; zero MSC 1 test coverage exists for any of them |
| Phase exit | P1.15 | full-suite gate check against the port plan's own exit criteria |

**Not in this phase.** The port plan's domain list is deliberately narrower than "everything Phase 0 extracted." These fixture domains stay unported until the phase named for them: `dto-contract`/`http-parse-request`/`remote-api-integration` (58 fixtures, Phase 2 — API contract), `network-safety` (13, Phase 3 — safety substrate, same reason as P1.5's filesystem split), `config-roundtrip` (7, Phase 5 — configuration and migration), `args-file-resolution` (12) and `headless-script` (19, both Phase 7 — server families and provisioning), and the five modpack domains — `curseforge-modpack`, `modpack-client-only`, `modpack-pinning`, `mrpack-extraction`, `pack-managed-guard` (57 total, Phase 8). That's 166 of the 270 Phase 0 fixtures accounted for elsewhere; this phase wires the remaining 96 that fall under its own domain list (see P1.3–P1.7), plus new characterization work Phase 0 had no test source for at all.

---

### Rust workspace

### P1.1 — Cargo workspace and the `msc-domain` crate skeleton
**Status:** DONE
**Files:** `Cargo.toml` (workspace root), `crates/msc-domain/Cargo.toml`, `crates/msc-domain/src/lib.rs`, `rust-toolchain.toml`, `.github/workflows/ci.yml`
**What:** Create the Cargo workspace per `msc2-engineering.md` §6's module boundaries, starting with exactly one member crate: `msc-domain` (server models, flavors, versions, settings schema, parsers, diagnostics policy — **no I/O**, per that section's direction rule). An empty `lib.rs` with one placeholder test so `cargo build`/`cargo test` have something to run before P1.2 adds the real harness. Pin the toolchain via `rust-toolchain.toml` so `cargo fmt`/`cargo clippy` behave identically for Cameron and CI. `S.3`'s CI toolchain job was written to install Rust and "build once `Cargo.toml` appears" — it now has; wire it to actually run `cargo build --workspace`, `cargo fmt --check`, and `cargo clippy --workspace -- -D warnings` on all three OSes.
**Verify:** `cd ~/msc2 && cargo build --workspace && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` → all exit 0
**Commit:** `P1.1: create the Cargo workspace and msc-domain crate skeleton`
**Batch:** stop-after

### P1.2 — Native fixture-loading test harness
**Status:** DONE
**Files:** `crates/msc-domain/tests/fixture_harness.rs` (or equivalent test-support module)
**What:** A Rust counterpart to P0.2's Python runner: deserializes a fixture file into the P0.1 shape (`domain`, `case`, `source`, `input`, `expected`, `notes`) and turns every file under `fixtures/<domain>/` into its own test — e.g. via `datatest-stable` or an equivalent build-script-generated-test approach — so a failing case names itself in `cargo nextest run` output the same way a failing Python fixture names itself today. Prove it against P0.2's own two self-test fixtures (`fixtures/_selftest/pass.json`, `fail.json`) before wiring any real domain: one meta-test asserts the harness's comparison function reports a match for `pass.json` and correctly reports a mismatch for `fail.json` — mirroring the Python runner's `--selftest` mode, which reports each fixture's outcome rather than letting the deliberately-broken one fail the build.
**Verify:** `cargo nextest run -p msc-domain fixture_harness_selftest` → `1 test run: 1 passed`
**Commit:** `P1.2: build the native fixture-loading test harness`
**Batch:** solo

---

### Existing-fixture domains

### P1.3 — Port version comparison
**Status:** DONE
**Files:** `crates/msc-domain/src/version.rs`, `crates/msc-domain/tests/version_comparison.rs`
**What:** Port `ComponentVersion` parsing and comparison (`ComponentVersionParsingTests.swift` origin, `fixtures/component-version/`) — the primitive MSC 2 needs everywhere a Paper/Purpur build number, Minecraft version string, or loader version gets compared, including the ordering behavior the downgrade guards several agent workflows depend on (`MCVersionComparator.isDowngrade`, symbol ledger target_domain `java-runtime`/`components` — those call sites port later; Phase 1 only needs the comparison primitive). Wire all 21 fixtures through the P1.2 harness.
**Verify:** `cargo nextest run -p msc-domain version_comparison` → `21 tests run: 21 passed`
**Commit:** `P1.3: port version comparison`
**Batch:** safe

### P1.4 — Port TPS parsing
**Status:** DONE
**Files:** `crates/msc-domain/src/tps.rs`, `crates/msc-domain/tests/tps.rs`
**What:** Port the TPS-sample parser (`TpsMonitoringTests.swift` origin, `fixtures/tps/`) — console-reply-line to TPS-figure conversion (Paper trio, legacy Forge, modern NeoForge, vanilla `/tick query` derivation, spark). **Scope note, amended during P1.4's own execution, not a silent skip:** 8 of the 27 fixtures in this domain (`auto-tps-command-*` ×3, `tps-poll-command-*` ×3, `supports-vanilla-tick-query-numeric-boundary`, `every-flavor-auto-tps-command-is-exhaustive`) don't test the parser at all — they test `JavaServerFlavor.autoTpsCommand` / `.tpsPollCommand(minecraftVersion:)` / `.supportsVanillaTickQuery`, which is P1.8's type, not P1.4's. Building it early here would mean writing it twice: once as a stub now, once for real in P1.8, which is the only step that plans the line-by-line cross-check against `JavaServerFlavor.swift` needed to trust it. Those 8 stay unwired here and are P1.8's responsibility (its own step text already commits to `autoTpsCommand`, `tpsPollCommand`, and the 1.20.3 `supportsVanillaTickQuery` boundary). Only the 19 pure-parser fixtures are wired in this step.
**Verify:** `cargo nextest run -p msc-domain tps` → `19 tests run: 19 passed`
**Commit:** `P1.4: port TPS parsing`
**Batch:** safe

### P1.5 — Port Java runtime policy (pure subset)
**Status:** DONE
**Files:** `crates/msc-domain/src/java_runtime.rs`, `crates/msc-domain/tests/java_runtime_guards.rs`
**What:** Port the pure guard/warning logic from `JavaRuntimeGuardsTests.swift` (`fixtures/java-runtime-guards/`): `requiredJavaMajor`'s Minecraft-version-to-Java-major mapping, and the too-old/too-new compatibility-warning classification. **Scope note, a deliberate call, not a silent skip:** 8 of the 15 fixtures in this domain touch the real filesystem — `detect-installed-java-runtimes-*` (×3) scans a directory tree, `normalization-*` (×5) stats candidate paths — and `msc-domain` carries no I/O per `msc2-engineering.md` §6. Those 8 stay unported here and move to `msc-infrastructure` once Phase 3 builds the filesystem substrate behind a trait. Only the 7 pure fixtures (`no-warning-*` ×3, `too-old-warning-still-fires`, `too-new-warning-*` ×2, `required-java-major-mapping`) are wired in this step. Flagged here for Cameron to overrule if he'd rather stub a filesystem trait early instead of waiting for Phase 3.
**Verify:** `cargo nextest run -p msc-domain java_runtime_guards` → `7 tests run: 7 passed`
**Commit:** `P1.5: port Java runtime policy (pure subset)`
**Batch:** stop-after

### P1.6 — Port property models
**Status:** DONE
**Files:** `crates/msc-domain/src/properties.rs`, `crates/msc-domain/src/settings_schema.rs`, `crates/msc-domain/tests/server_properties.rs`, `crates/msc-domain/tests/settings_schema.rs`
**What:** Port `ServerPropertiesModel` (`ServerPropertiesModelTests.swift` origin, `fixtures/server-properties/` — the unknown-key-preserving round trip `msc2-engineering.md` §7 names directly: "silently rewriting `server.properties` with only the recognized keys is destructive") and the settings schema (`ServerSettingsSchemaTests.swift` origin, `fixtures/settings-schema/` — type coercion, range clamping, the level-type wire-token mapping, case-insensitive enums). Two modules, each wired to its own fixture directory.
**Verify:** `cargo nextest run -p msc-domain server_properties` → `7 tests run: 7 passed`; then `cargo nextest run -p msc-domain settings_schema` → `16 tests run: 16 passed`
**Commit:** `P1.6: port property models`
**Batch:** safe

### P1.7 — Port crash analysis and slug normalization
**Status:** DONE
**Files:** `crates/msc-domain/src/crash_analysis.rs`, `crates/msc-domain/src/slug.rs`, `crates/msc-domain/tests/connector_crash_analysis.rs`, `crates/msc-domain/tests/startup_crash_analyzer.rs`
**What:** Port `StartupCrashAnalyzer` (`ConnectorCrashAnalysisTests.swift` + `StartupCrashAnalyzerTests.swift` origins — Forge dependency-block parsing, connector entrypoint failure attribution, Fabric/Forge missing- and wrong-dependency-version attribution) and `ModrinthSlugNormalizer` (`canonicalSlug` / `normalizedSlug` / `isKnownAlias`). MSC 1 has no separate test file for the normalizer — it doesn't need new characterization, because 4 of the 11 `connector-crash-analysis` fixtures already exercise it directly (MSC 1's own test file bundles the two together). `searchQuery`, the normalizer's one method with no fixture of its own, is a one-line wrapper (`canonical.isEmpty ? raw : canonical`) — port it as part of `slug.rs` but don't invent a fixture for a wrapper the existing 4 already cover the substance of.
**Verify:** `cargo nextest run -p msc-domain connector_crash_analysis` → `11 tests run: 11 passed`; then `cargo nextest run -p msc-domain startup_crash_analyzer` → `7 tests run: 7 passed`
**Commit:** `P1.7: port crash analysis and slug normalization`
**Batch:** safe

---

### New-characterization domains

Both files below have **no MSC 1 test file** — nothing to extract. Per `fixture-format.md`, `expected` values still may not be invented; they come from reading the source's closed, deterministic logic directly (every case is enumerable by inspection) — the same evidentiary standard `fixture-format.md` calls "MSC 1 run by hand" for untested pure functions. `source.test` in each new fixture should name the property or method being characterized, not a fabricated Swift test name.

### P1.8 — Characterize and port server identity & flavors
**Status:** DONE
**Files:** `fixtures/server-identity/`, `crates/msc-domain/src/identity.rs`, `crates/msc-domain/tests/server_identity.rs`, `crates/msc-domain/src/version.rs` (two helpers bumped to `pub(crate)` for reuse), `crates/msc-domain/src/lib.rs`
**What:** `ServerType` (`java`/`bedrock`, `AppConfig.swift`) and `JavaServerFlavor` (`JavaServerFlavor.swift`, 246 lines, 9 cases: `paper, purpur, pufferfish, vanilla, fabric, neoforge, spigot, forge, quilt`). Wrote fixtures covering, per flavor: `category`, `isForgeFamily`, `addOnKind`, `provisioningKind`, `modrinthProjectType`, `modrinthLoaderFacets`, `autoTpsCommand`, `isRecommended`, `isAvailableInCreateFlow` — one case per flavor bundling all nine (9 fixtures). Boundary cases for `tpsPollCommand(minecraftVersion:)` / `supportsVanillaTickQuery` around the 1.20.3 threshold — below (1.20.2), exactly at (1.20.3), above (1.20.10, the doc comment's own example), nil, and empty string (5 fixtures). One case per `JavaServerCategory` for `createFlowChoices` (2 fixtures). 16 fixtures total. `displayName`, `shortDescription`, and `iconName` are client-rendering per the port plan's deletion test (§1) and are not ported; `ServerType` has no computed property beyond the excluded `displayName`, so it carries no fixture of its own. `ServerProvisioningKind` and `AddOnKind` aren't `: String` in Swift (no `rawValue`), so their Rust `raw_value()` wire tokens (`download_and_go`/`install_step`, `plugin`/`mod`) are invented for this port, not pulled from source. `supports_vanilla_tick_query`'s numeric compare reuses `version.rs`'s `parse_components`/`compare_components` (bumped from private to `pub(crate)`) rather than re-implementing dotted-integer comparison.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/server-identity --expect 16` → `ok 16`; then `cargo nextest run -p msc-domain server_identity` → `16 tests run: 16 passed`
**Commit:** `P1.8: characterize and port server identity & flavors`
**Batch:** solo

### P1.9 — Characterize and port the command catalog
**Status:** DONE
**Files:** `fixtures/command-catalog/`, `crates/msc-domain/src/commands.rs`, `crates/msc-domain/tests/command_catalog.rs`, `crates/msc-domain/src/lib.rs`
**What:** `MinecraftCommandRegistry.swift` (542 lines, 42 command definitions). Two things characterized: (1) the static catalog's `commands(for:)` Java/Bedrock filter — 2 fixtures asserting the exact filtered name list per `ServerType` (41 of 42 for Java, excluding only `allowlist`; 29 of 42 for Bedrock), not a re-typed copy of all 42 definitions; (2) the autocomplete engine, `suggestions(for:serverType:onlinePlayers:)` — 16 fixtures covering command-name-prefix completion (plain, leading-`/`, case-insensitive, the 6-item cap, no-match, and gated by `commands(for:)`'s own server-type filter), the empty-input and unknown-command-name `[]` cases, player-name filtering against a fake online-player list (partial-prefix, case-insensitive, 6-item cap), keyword-option filtering, coordinates/integer/free-text slots never suggesting anything, and argument-slot detection including the "input ends with a space starts a new slot" behavior — which turned out to hide a genuine off-by-one in MSC 1's own source (`tokens.dropFirst()` still contains the trailing empty token a trailing-space split leaves behind, so `slotIndex` lands one slot later than a reader would expect right after typing `"cmd "`; three fixtures pin the three ways this plays out). Every expected value, including the off-by-one cases, was confirmed by running a standalone copy of the literal Swift source through `swift` (not hand-derived) before writing the fixture, since this particular quirk is easy to get wrong by inspection alone. `description`, `category` (`icon`/`color`), and each argument slot's `label` are client-rendering (port plan §1's deletion test) and are not ported; `CommandArgSlot`'s `.coordinates`/`.integer`/`.freeText` cases collapse into one `ArgSlotKind::Other` in Rust since `suggestions` treats all three identically. `commands_for`/`suggestions` reuse `identity::ServerType` rather than inventing a second one.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/command-catalog --expect 18` → `ok 18`; then `cargo nextest run -p msc-domain command_catalog` → `18 tests run: 18 passed`
**Commit:** `P1.9: characterize and port the command catalog`
**Batch:** solo

---

### Router rule engine

Five files, 2,077 lines total, **zero MSC 1 test coverage** for any of them — per `msc2-decisions.md` D-026 point 3, "the matcher, fallback resolver, composer, and troubleshooting decision tree are executable behavior and are translated to Rust" (the runtime resolver is a fifth, separately named in the port plan and already adjudicated agent-owned in the symbol ledger, see P1.14). The guide **catalog and step content** are data, not logic — per D-026 point 1, they migrate to JSON "at any time," on their own schedule, not gated to this phase. P1.10–P1.13 introduce one small, shared, representative sample of guide records — not the real catalog — sufficient to exercise the engines; P1.10 builds it, P1.11–P1.13 reuse it.

### P1.10 — Characterize and port the router matcher
**Status:** DONE
**Files:** `fixtures/router-matcher/`, `fixtures/router-sample-catalog.json`, `crates/msc-domain/src/router.rs`, `crates/msc-domain/src/router/matcher.rs`, `crates/msc-domain/tests/router_matcher.rs`, `crates/msc-domain/src/lib.rs`
**What:** `RouterPortForwardGuideMatcher.swift` (320 lines) — "scores guide candidates against user input and returns ranked results with confidence metadata... normalizes freeform user input, infers likely router/provider families, ranks candidate guides, and suggests a fallback when there is no exact family guide in the current catalog" (the file's own doc comment). Built `fixtures/router-sample-catalog.json` first: a literal, full-fidelity transcription of 6 of MSC 1's 14 real `v1Guides` seed records (`RouterPortForwardGuidesFoundation.swift`) — generic-router, generic-mesh, xfinity-gateway, netgear-router, eero-mesh, advanced-troubleshooting — covering two ISP/retail router brands, two mesh-system brands, and the troubleshooting family the matcher's intent logic depends on. Carries every guide field (steps, notes, sharedSections, troubleshooting refs, review metadata), not just what the matcher touches, since P1.11–P1.13 reuse this same file for the composer and troubleshooting engines. Characterized via 10 fixtures covering exact-name match (score-cap case), fuzzy/partial keyword match, family inference from a token-subset phrase and from a genuinely misspelled brand name (which still infers a family only by coincidence — via a short alias like "xfi" being a substring of the typo, not fuzzy/edit-distance matching, which this engine has none of), a 4-way score tie among non-top candidates that does *not* trip `isAmbiguous` (only the top two scores are compared), the empty-input case (an unconditional +20 "unknown router fallback" bonus with no `score > 0` guard, unlike every other bonus branch), a true no-match case (zero candidates), an isp/gateway-hint case, a mesh-intent case ranking three candidates via genuinely different score paths, and an exact top-two score tie that does trip `isAmbiguous` with alphabetical tie-break. Every expected value was confirmed by running a standalone literal copy of the Swift matcher plus the 6-guide sample catalog through `swift` (`registry_harness`-style, not hand-derived) — an initial hand-derivation of three of these fixtures (the partial-match score, the empty-input candidate count, and the misspelling's family inference) was wrong and only caught by running the actual source, the same lesson P1.9's off-by-one taught. `Guide` in `matcher.rs` carries only the fields the matcher touches (`id`, `displayName`, `category`, `family`, `searchKeywords`, `adminSurface`, `providerDisplayName`, `deviceDisplayName`) per the port plan's deletion test; `adminAddresses`/`menuPath`/`alternateMenuNames`/`steps`/`notes`/`troubleshooting`/`sharedSections`/`review` are read by later engines from the same JSON file, not by this one. Two scoping gaps, both undocumented-by-omission-would-be-wrong so called out in the module doc comment instead: `normalize` skips Swift's Unicode NFKD diacritic fold (no fixture exercises non-ASCII text, and `msc-domain` has no normalization crate dependency to add one with); the tie-break sort uses `str::to_lowercase` instead of Swift's locale-aware `localizedCaseInsensitiveCompare` (equivalent for this domain's ASCII display names). One preserved dead-code quirk: `inferredIntent`'s `normalizedQuery.contains("don't know")` check can never fire, since `normalize` strips every apostrophe earlier in the same function — ported as-is rather than silently dropped.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/router-matcher --expect 10` → `ok 10`; then `cargo nextest run -p msc-domain router_matcher` → `10 tests run: 10 passed`
**Commit:** `P1.10: characterize and port the router matcher`
**Batch:** solo

### P1.11 — Characterize and port the router fallback decision tree
**Status:** DONE
**Files:** `fixtures/router-fallback-tree/`, `crates/msc-domain/src/router.rs`, `crates/msc-domain/src/router/fallback_tree.rs`, `crates/msc-domain/tests/router_fallback_tree.rs`
**What:** `RouterPortForwardFallbackDecisionTree.swift` (610 lines — the largest of the five). Two loosely-coupled pieces, both ported since D-026 names "the fallback resolver" and "the...decision tree" together as executable behavior: (1) the 8-node `RouterPortForwardDecisionNodeID` navigation graph (`makeTree`) — ported as structure only (each node's `id`/`kind`, each choice's `id`/`nextNodeID`/`impliedNetworkType`), excluding `title`/`body`/`bullets`/`suggestedSearchTerms` UI prose per the deletion test, since `resolve()` never calls `makeTree()` at all and nothing computes over that prose; (2) `unknownRouterBullets(detectedGatewayIPAddress:)`, the one genuinely conditional piece of tree construction (inserts a gateway-IP bullet at index 3 only when one is present and non-blank) — ported in full including literal bullet text, since the bullet's presence/position *is* the tested behavior, not decoration; (3) `resolve()` (`RouterPortForwardFallbackRouter`) — the actual fallback resolver and this step's main target, routing a `FallbackState` + search query through P1.10's matcher and the sample catalog to one of `ResolutionKind`'s seven outcomes. 20 fixtures verified by extending P1.10's Swift harness with a full literal copy of this file's tree/bullets/resolve logic and running it, not hand-derived, covering: short-circuit on `wantsAdvancedTroubleshooting`; exact-guide vs family-guide matches; the top-candidate-is-troubleshooting-family branch; two "family recognized but not seeded" fallbacks (mesh and non-mesh); a non-empty query that infers no family at all falling through to the generic-router default; all four `networkType` switch cases; the `onlyKnowsMeshSystem`/`unsureWhetherISPOrOwnRouter` flag paths; the default empty-state fallback; and the runtime gateway-IP being threaded into a resolution bullet's text (the one place besides `unknownRouterBullets` itself where it changes output). One fixture uses a smaller in-Rust catalog (the sample catalog minus generic-mesh) to reach `genericMeshResolution`'s not-seeded branch, unreachable against the standard 6-guide catalog. Two of `resolve()`'s branches (a topCandidate that doesn't match its own family while a *different* already-seeded family was still inferred, and the matching troubleshooting-via-inferred-family-not-topCandidate path) are not exercised — both require a guide to out-score every guide whose own family got inferred, and since inferring a family always hands that family's guide a flat +70, this sample catalog's tightly-coupled keyword/alias data (drawn from real MSC 1 content) makes that very hard to construct without an unrealistic, contrived query; documented in `fallback_tree.rs`'s module doc comment rather than silently skipped. `Guide`/`GuideFamily`/`matcher::match_query` are reused from P1.10, not duplicated. `RouterPortForwardGuideRuntimeContext` (P1.14's type) is not ported here — `resolve`/`generic_mesh_resolution` take `detected_gateway_ip_address: Option<&str>` directly, the only field of it this file touches.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/router-fallback-tree --expect 20` → `ok 20`; then `cargo nextest run -p msc-domain router_fallback_tree` → `20 tests run: 20 passed`
**Commit:** `P1.11: characterize and port the router fallback decision tree`
**Batch:** solo

### P1.12 — Characterize and port the router guide composer
**Status:** DONE
**Files:** `fixtures/router-composer/`, `fixtures/router-sample-catalog.json`, `crates/msc-domain/src/router.rs`, `crates/msc-domain/src/router/composer.rs`, `crates/msc-domain/tests/router_composer.rs`
**What:** `RouterPortForwardGuideComposer.swift` (306 lines) — "composes fully ordered logical guide structures from seed data, merging router-specific steps, prerequisites, value summaries, and notes into a renderable section list" (the file's own doc comment). Reading the actual source, that "merging" is plain conditional concatenation — `composeSections` appends up to seven sections in a fixed order, each included by a boolean flag or an emptiness guard; there is no mechanism anywhere in the file where a router-specific item overrides or takes precedence over a shared one, contrary to this step's own "merge precedence when a router-specific step overrides a shared one" description above — flagged as a plan/source mismatch, not silently reconciled. Characterized 7 fixtures against P1.10's sample catalog (extended with a `troubleshootingTopics` array — the catalog's other half, needed for the troubleshooting-footer section and reused as-is by P1.13): the full fixed section order with every optional section present; the two-vs-one-vs-neither introBody branches (provider+device both set and differing, device-only, and — via a synthetic ad-hoc guide built directly in the Rust test, since none of the 6 real sample guides omit deviceDisplayName — neither); the `.ispGateway`/`.meshSystem` category-specific prerequisite bullets vs. the `default` no-bullet case; the conditional Bedrock value-summary bullets (present when any step references a Bedrock token, absent — via the same synthetic guide — when none do); menuPathSection's "alternates alone, empty path list" case; the routerSpecificSteps section's referencedTokens dedup preserving first-occurrence order across steps, not token-enum declaration order; and troubleshootingFooterSection's topic ordering following the *catalog's* declared order, not the guide's own `troubleshooting` array order (confirmed directly in the harness's generic-router output, order differs from the guide's declared list). `Guide` here is the first router-engine type carrying full guide content (steps/notes/topics are the composer's actual output, not excludable client-rendering) — genuinely different fields from `matcher::Guide`, reusing only `GuideCategory`/`AdminSurface` from that module. `composeGuide(id:)` is ported (`compose_guide_by_id`); `composeBestMatch(for:matcher:)`, which just chains the already-tested matcher into `composeGuide`, is not. Every expected value verified by running a literal Swift copy of this file plus the full sample-catalog data through `swift`, not hand-derived — an initial draft of the harness trimmed each guide's step list down to only token-bearing steps for brevity, producing several wrong `items`-list fixtures until cross-checked against the authoritative catalog JSON and corrected.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/router-composer --expect 7` → `ok 7`; then `cargo nextest run -p msc-domain router_composer` → `7 tests run: 7 passed`
**Commit:** `P1.12: characterize and port the router guide composer`
**Batch:** solo

### P1.13 — Characterize and port the router troubleshooting engine
**Status:** DONE
**Files:** `fixtures/router-troubleshooting/`, `crates/msc-domain/src/router.rs`, `crates/msc-domain/src/router/troubleshooting.rs`, `crates/msc-domain/tests/router_troubleshooting.rs`
**What:** `RouterPortForwardTroubleshootingEngine.swift` (550 lines) — a "rule-based troubleshooting engine for router and port-forwarding failures. Accepts user-reported symptoms and returns prioritised causes and recommended actions" (the file's own doc comment). Ported the full 9-rule `RouterPortForwardTroubleshootingKnowledgeBase` table (`make_rules`) and the `analyze`/`evaluate` scoring engine, against P1.10/P1.12's shared 9-topic catalog. Excluded per the deletion test: `RouterPortForwardSymptom` (the `title`/`description` UI text on each symptom) and `supportedSymptoms`/`symptom(id:)` — static picklist data nothing here computes over, client-owned like Guide's display fields in `matcher.rs`. Also dropped: Swift's `makeRules(repository:)` `repository` parameter (unread by the real table — nothing in it resolves a topic, since that's `evaluate`'s job, which already takes `topics` directly); and the `analyze(symptomIDs: Set<SymptomID>, ...)` overload (a one-line wrapper around the ported `analyze`, equivalent modulo a `Set` ordering Swift itself never guaranteed). 14 fixtures characterized by extending P1.11's Swift harness with a literal copy of this file plus the shared catalog's 9 topics, and — a change from P1.10-P1.12's methodology — having the harness itself emit real JSON (`JSONSerialization`) per scenario instead of Swift's debug-print format, specifically to close off the exact hand-transcription failure mode that produced P1.12's two self-caught bugs; all 14 Rust tests passed against the harness's JSON on the first run. Fixtures cover: a rule with an empty `allOf` firing from `anyOf` alone; the `anyOf.count / 2` confidence-threshold arithmetic at both a `strong` edge (2-item `anyOf`, 1 match still strong) and a `possible` case (4-item `anyOf`, 1 of 4 matches); real (non-synthetic) `recommendedActions` dedup, where a topic's own suggested action and its rule's `nextActions` happen to share one identical string; the three-way `likelyCauses` sort (score desc, then confidence, then — unexercised by real data, noted below — topic title); `matchedSymptoms`' order coming from the rule's own declared `allOf`/`anyOf` order, not symptom-input order (pinned by a fixture where the two orders deliberately disagree); both one-cause and two/three-cause summary phrasing; and every fallback-integration path (`fallbackState` absent; present with `unknownRouterHelp` or `needsMoreInfo` kind, both of which flip `makeSummary`'s fallback-aware branch; present with a third kind like `exactGuide`, which doesn't; a gateway IP threaded through at the same time as real causes, proving escalation bullets concatenate rule-then-fallback rather than merge/sort). Two branches are structurally unreachable against MSC 1's real 9-rule/9-topic table — confirmed empirically, not by inspection — and are exercised via synthetic test-only data built in the Rust test file, the same precedent P1.11 (second catalog) and P1.12 (synthetic guide) set: `evaluate`'s `excludedSymptoms` short-circuit (every real rule declares `excludedSymptoms: []`; exercised via a synthetic 10th rule) and its "rule's topic missing from the repository" `nil`-return (the real table and catalog are exactly 1:1; exercised via a topics list missing one entry). The topic-title alphabetical tie-break (the sort's third comparator) was not reached by any constructed real-data scenario within this step's scope and is not separately fixtured — noted rather than forced via more synthetic data, since the first two comparators (score, confidence) already fully determine the order in every fixture built.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/router-troubleshooting --expect 14` → `ok 14`; then `cargo nextest run -p msc-domain router_troubleshooting` → `14 tests run: 14 passed`
**Commit:** `P1.13: characterize and port the router troubleshooting engine`
**Batch:** solo

### P1.14 — Characterize and port the router runtime resolver
**Status:** DONE
**Files:** `fixtures/router-runtime-resolver/`, `crates/msc-domain/src/router.rs`, `crates/msc-domain/src/router/runtime_resolver.rs`, `crates/msc-domain/tests/router_runtime_resolver.rs`
**What:** `RouterPortForwardGuideRuntimeResolver.swift` (291 lines) — already adjudicated agent-owned in the symbol ledger (`msc2-symbol-ledger.csv`, two rows for this file: `makeRecommendedProtocol`, and the `resolve`/`resolveGuide`/`resolveBestMatch`/`resolveItem`/`resolveText` family — the latter corrected to agent during Codex's P0.27 review, on the strength of D-026 naming it directly). Ported `resolve`/`resolveItem`/`resolveText` (walking a P1.12-composed guide's sections, substituting `{{raw_value}}` placeholders in paragraph/bulletList/step bodies against a `RuntimeContext`, recording any token whose placeholder couldn't be resolved) and `makeRecommendedProtocol` (TCP always; UDP only when Bedrock is enabled *and* a Bedrock port is known). `resolveGuide(id:...)`/`resolveBestMatch(for:...)` are not ported — both are one-line chains of an already-tested lookup (`compose_guide_by_id` from P1.12, or the matcher's `best_match` from P1.10) into this file's own `resolve`, the same "tests only that they compose" reasoning `composer.rs` recorded for excluding `composeBestMatch`; flagged here since this step's own plan text didn't call that out in advance. `RouterPortForwardGuideRuntimeContext`'s `@MainActor extension AppViewModel` (host glue reading a live server's actual IP/port/Bedrock state) is host-owned I/O and stays out of this crate. 15 fixtures characterized by extending P1.13's Swift-harness-emits-real-JSON methodology: this harness runs the real `RouterPortForwardGuideComposer` + `RouterPortForwardGuideRepository` (constructed directly from `fixtures/router-sample-catalog.json`, bypassing the bundle/file-based loader) piped into the real `RouterPortForwardGuideRuntimeResolver`, so every non-synthetic fixture's composed-guide input is authentic Swift output, not hand-built. Fixtures cover: every context field set (no unresolved tokens); every field unset (every literal placeholder in the rendered guide falls back, `unresolvedTokens` sorted by section id then token raw value); one field unset among several set; whitespace-only/empty-string context values treated as unset by `cleaned`'s trim; `resolvedString(for: .bedrockPort)` resolving independent of `bedrockEnabled` (only `.bedrockEnabled` itself reads that flag); a step (`xfinity-step-4`) whose `referencedTokens` metadata names three tokens but whose body contains no literal `{{...}}` placeholder at all — confirming substitution is driven only by literal placeholder presence, not the metadata; `menuPath`/`note`/`troubleshootingTopic` items passing through `resolveItem` untouched; `recommendedProtocol` being echoed verbatim from the context rather than recomputed via `makeRecommendedProtocol` (the two are decoupled — a host is expected to call the latter itself); and a synthetic (test-only, not literal MSC 1 content) `ComposedSection` with a body repeating one placeholder twice, pinning that every occurrence is replaced. Plus 6 fixtures for `makeRecommendedProtocol` across Java-only, Java+Bedrock, Bedrock-only, neither-known, and the two "flag/port disagree" edges (`bedrockEnabled` true with no port; a port present but `bedrockEnabled` false) — both of the latter fall through to the Java-only/generic branch, since Swift's `if bedrockEnabled, let bedrockPort` requires both.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/router-runtime-resolver --expect 15` → `ok 15`; then `cargo nextest run -p msc-domain router_runtime_resolver` → `15 tests run: 15 passed`
**Commit:** `P1.14: characterize and port the router runtime resolver`
**Batch:** solo

---

### Phase exit

### P1.15 — Phase 1 exit gate check
**Status:** DONE
**Files:** none (verification only)
**What:** Run every Phase 1 domain together and confirm the crate stays clean: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and the full `cargo nextest run -p msc-domain` suite (every domain from P1.3–P1.14 in one run). Confirm `msc-domain` still carries no I/O dependency, per its module-boundary rule — check its `Cargo.toml` pulls in no filesystem/network/process crates. This checks the port plan's own Phase 1 exit criteria verbatim: "Rust passes the Phase 0 pure fixtures. No user files touched."
**Verify:** `cd ~/msc2 && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo nextest run -p msc-domain` → all green; then `grep -E '^\s*(tokio|reqwest|std::fs|walkdir|notify)' crates/msc-domain/Cargo.toml` → no matches
**Commit:** _(n/a — verification only, unless a fix is needed)_
**Batch:** stop-after

---

## Phase 2 — API contract and operation model

**Gate** (`msc2-port-plan.md` §3): "Versioned HTTP and WebSocket contract generated from the schema. Operation IDs, progress, structured errors, capability advertisement, cancellation. A skeletal agent whose routes can be exercised without real mutation." **Exit criteria: the existing iOS app connects and reads status against a stub agent.**

**Rust crates start appearing beyond `msc-domain`.** Per `msc2-engineering.md` §6, this phase brings in `msc-api` (routes, DTOs, WebSocket events, auth, permission checks) and `msc-agent` (service startup, routing, static asset serving) — the two crates immediately outward of `msc-domain` in the direction rule. `msc-application` and `msc-infrastructure` (real orchestration, real filesystem/process I/O) are **not** built this phase: the port plan's own words are "a skeletal agent whose routes can be exercised **without real mutation**," so every handler this phase wires returns canned or in-memory data, never touches a real server process or a real file.

**Three open items block calling the contract "frozen," not necessarily Phase 2 by name.** `msc2-engineering.md` §19 ties the D-019 permission vocabulary, the D-026 educational-content shape, and the D-012 auth gaps to "contract freeze." Read literally, D-012's six gaps would require designing the full desktop/web remote-pairing and browser-origin story before this phase could close — but Phase 2's own gate only requires the **iOS** app to connect, and D-016's "UI never gates correctness" / vertical-slice principle argues against solving Tauri/browser auth (Phase 11's problem) to satisfy an iOS-only gate. P2.3 below makes that scoping call explicitly, in writing, rather than silently either overclaiming the contract is fully frozen or silently ignoring the engineering doc's own table. Cameron should confirm or overrule this scoping during the Read move — it's the single biggest judgment call in this phase's plan.

21 steps, six groups:

| Group | Steps | Deliverable |
|---|---|---|
| Contract-freeze prerequisites | P2.1–P2.3 | D-019 vocabulary validated against all 88 baseline routes; D-026 content format and `helpId` shape decided; D-012 scoped for what this phase actually needs |
| v1 contract design | P2.4–P2.8 | `docs/msc2/api-contract/openapi.json` — the versioned HTTP+WS contract everything else in this phase implements |
| Domain model extensions | P2.9–P2.10 | Operation-state and capability types added to `msc-domain`, still no I/O |
| Skeletal agent | P2.11–P2.17 | `msc-api` and `msc-agent` crates: a running, auth-gated, contract-conformant stub agent |
| iOS re-pointing | P2.18–P2.20 | The existing iOS app, copied into this repo and re-pointed, connects to the skeletal agent and reads status — the phase gate itself |
| Phase exit | P2.21 | Full-suite gate check against the port plan's own exit criteria |

**Not in this phase**, deferred on purpose:

- **Real host/helper/capability detection, the `SecretStore`-backed pairing flow, rate limiting, and audit logging** — all Phase 3 substrate work. This phase's capability and auth responses are honest placeholders, documented as such.
- **The operation journal (restart survival)** — Phase 3. This phase's operations live in an in-memory map only; the agent forgetting them on restart is expected, not a bug, this phase.
- **Desktop/web (Tauri) remote pairing, browser origin policy, CSRF, LAN TLS provisioning, Tailscale posture** — Phase 11, per P2.3's scoping.
- **A general OpenAPI-to-Rust / OpenAPI-to-Swift codegen pipeline.** `msc2-engineering.md` §5 states the ultimate intent ("generates the Rust server types and the Swift iOS client models"), but building a generic codegen tool is its own project. This phase hand-writes both sides against the frozen schema, with a conformance checker (P2.11, P2.17) standing in for generation-time guarantees. Revisit once the contract has had a chance to stabilize — generating code against a contract that's still moving is wasted work.
- **WebSocket push for status, players, notifications, metrics.** MSC 1 polls all four over HTTP today (P0.24's finding); nothing in this phase's gate needs push delivery for them. Only `console` (real baseline behavior) and `operation-progress` (new, needed to observe cancellation) get WS channels this phase.

---

### Contract-freeze prerequisites

### P2.1 — Validate the D-019 permission-category vocabulary against all 88 baseline routes
**Status:** DONE
**Files:** `docs/msc2/api-contract/permission-vocabulary.csv`, `docs/msc2/msc2-decisions.md` (amend D-019)
**What:** D-019 names four permission categories MSC 1 enforces (`players`, `settings`, `worlds`, `mods`) but notes "the current category vocabulary has not been validated against all 87 routes" (now 88 per P0.30). Read the actual enforcement in `RemoteAPIServer+HTTP.swift`'s dispatcher and record, per route, which category (or categories) it actually requires — not an assumption from the route's name. Several routes plainly need a fifth bucket the existing four don't cover (`/users*`, `/health/repair`, `/files*` — administrative/owner-only, not any of `players`/`settings`/`worlds`/`mods`); record that gap plainly rather than forcing a fit. Amend D-019 with the outcome — either "four categories confirmed sufficient" or a proposed fifth (`admin`) category — still **Proposed**, pending Cameron's confirmation, not silently promoted to Approved.
**Verify:** `python3 -c "import csv;print(len(list(csv.DictReader(open('docs/msc2/api-contract/permission-vocabulary.csv')))))"` → `88`
**Commit:** `P2.1: validate the permission-category vocabulary against all 88 baseline routes`
**Batch:** solo

### P2.2 — Decide the educational content format and the `helpId` contract shape
**Status:** DONE
**Files:** `docs/msc2/api-contract/helpid-contract.md`, `docs/msc2/msc2-decisions.md` (amend D-026)
**What:** D-026 leaves open "content format" and "embedded vs on-disk," flagging both as real product-shape calls. Recommend Markdown with YAML front-matter (the doc's own "obvious candidate") and embedding content in the agent binary (via `rust-embed` or `include_str!`) for v1 — guarantees the handbook is always present, matching the product promise that help never requires a separate download; on-disk override can be added later if hot-editing content without a release turns out to matter. Record both as **Proposed**, explicitly flagged for Cameron to confirm or overrule during the Read move — this is exactly the kind of call CLAUDE.md says not to make silently. Define the `helpId` shape precisely: a dotted-namespace string (`settings.difficulty`, `health.tick-lag`, `diagnostics.crash.forge-dep`), resolved via a new `GET /v1/help/{helpId}` route. Enumerate every DTO field `msc2-engineering.md` §18 names as needing one (settings fields, health cards, diagnostics, performance metrics, connection methods, crash-analysis findings) so P2.8 knows exactly where to attach it.
**Verify:** `grep -c 'helpId' docs/msc2/api-contract/helpid-contract.md` → non-zero; `grep -c '^## D-026' docs/msc2/msc2-decisions.md` → `1` (confirms this amends the existing entry rather than duplicating it)
**Commit:** `P2.2: decide the educational content format and helpId contract shape`
**Batch:** stop-after

### P2.3 — Scope Phase 2's authentication surface and record what stays deferred
**Status:** DONE
**Files:** `docs/msc2/api-contract/auth-scope-phase2.md`, `docs/msc2/msc2-decisions.md` (amend D-012)
**What:** Phase 2's gate needs only the **iOS app** to connect to a **local, loopback** skeletal agent — not a Tauri desktop app connecting to a remote host, and not a browser. Scope this phase's auth work to exactly that: implement bearer-token *verification* (the already-Approved iOS mechanism — QR pairing → keychain token → bearer header — minus the real pairing-secret exchange, which needs Phase 3's `SecretStore` trait and doesn't exist yet). Stand in a single fixed dev token for now, clearly commented as a placeholder. Explicitly record as still-open, not solved by this phase: remote desktop pairing, per-host credential storage, LAN TLS provisioning, Tailscale posture, browser origin policy and CSRF — none of which an iOS-only, loopback-only gate requires. Amend D-012 with this scoping so a future reader doesn't mistake Phase 2's dev token for the six gaps being closed.
**Verify:** `grep -c 'Phase 2 scope' docs/msc2/msc2-decisions.md` → at least `1`
**Commit:** `P2.3: scope Phase 2 authentication to iOS-only, loopback-only, and record what's deferred`
**Batch:** solo

---

### v1 contract design

### P2.4 — Design the v1 route namespace, skew behavior, and error envelope
**Status:** DONE
**Files:** `docs/msc2/api-contract/versioning-and-errors.md`
**What:** Per D-010, design the route-versioning mechanism (not the N-3 floor, still unset): every route lives under `/v1/`; the agent reports its API major/minor and capability set (feeding P2.6); a request from a client below the supported floor gets a clear structured refusal, not a generic 404. Design one consistent `ErrorDTO` (`code`, `message`, `helpId?`, `details?`) replacing the baseline's split `Error`/typed-DTO failure pattern that P0.32 catalogued — recorded here as a deliberate D-006-point-3 correction, not silent.
**Verify:** `grep -cE '^### ' docs/msc2/api-contract/versioning-and-errors.md` → at least `3`
**Commit:** `P2.4: design the v1 route namespace, skew behavior, and error envelope`
**Batch:** solo

### P2.5 — Design the operation model contract
**Status:** DONE
**Files:** `docs/msc2/api-contract/operation-model.md`
**What:** Design `OperationDTO { id, type, target, state, progress, statusLine, result?, error? }` per `msc2-engineering.md` §5, with `state` as the closed enum `queued|running|succeeded|failed|cancelled`. Design the three routes needed to exercise it without real mutation: `POST /v1/operations` (accepts an operation type/target, returns an id), `GET /v1/operations/{id}`, `POST /v1/operations/{id}/cancel`. Record explicitly that restart-survival (the operation journal) is Phase 3 scope — this designs the wire shape only.
**Verify:** `grep -c 'queued|running|succeeded|failed|cancelled' docs/msc2/api-contract/operation-model.md` → at least `1`
**Commit:** `P2.5: design the operation model contract`
**Batch:** solo

### P2.6 — Design the capability-advertisement contract
**Status:** DONE
**Files:** `docs/msc2/api-contract/capability-model.md`
**What:** Design `GET /v1/capabilities`, returning agent version, API major/minor (from P2.4), host OS, per-server-type feature flags, the token's permission set (from P2.1), and installed-helper presence flags — the exact list `msc2-engineering.md` §5 names under "Capability discovery." Since the skeletal agent has no real detection yet, the schema is designed now and populated with clearly-labeled placeholder values until Phase 3/4 wire real detection behind it.
**Verify:** `grep -c 'capabilities' docs/msc2/api-contract/capability-model.md` → at least `1`
**Commit:** `P2.6: design the capability-advertisement contract`
**Batch:** solo

### P2.7 — Design the console and operation-progress WebSocket schemas
**Status:** DONE
**Files:** `docs/msc2/api-contract/websocket-v1.json`
**What:** Carry P0.24's one real baseline channel forward unchanged in wire shape, versioned at `/v1/console/stream` (same bearer auth, same 200-line-backfill-then-live model, same 5000-line ring buffer, same 64 KB inbound-frame cap). Design one new channel, `operation-progress` at `/v1/operations/{id}/stream`, pushing `OperationDTO` updates from P2.5 as they change, with the same bounded-history-then-live delivery discipline. `status`/`players`/`notifications`/`metrics` stay HTTP-polled this phase — see the phase's "Not in this phase" note.
**Verify:** `python3 -c "import json;d=json.load(open('docs/msc2/api-contract/websocket-v1.json'));print(sorted(c['name'] for c in d['channels']))"` → `['console', 'operation-progress']`
**Commit:** `P2.7: design the console and operation-progress WebSocket schemas`
**Batch:** solo

### P2.8 — Assemble the MSC 2 v1 OpenAPI contract
**Status:** DONE
**Files:** `docs/msc2/api-contract/openapi.json`, `tools/api-contract-check.py`
**What:** Seed from `docs/msc2/api-baseline/openapi.json` (the 88-route MSC 1 baseline, P0.23/P0.30/P0.32) and apply, in order: the `/v1/` namespace and `ErrorDTO` envelope (P2.4), the operation routes (P2.5), the capabilities route (P2.6), the `helpId` field on every schema P2.2 enumerated, and the permission-category annotation on every route (P2.1). This is the file the port plan's Phase 2 description calls "versioned HTTP...contract generated from the schema" — the frozen deliverable every later step in this phase builds against. Build a checker script, in the style of P0.23's, asserting: every route sits under `/v1/`, every route declares a permission category, every field flagged for `helpId` in P2.2 actually carries one, and the total route count matches baseline (88) plus the new operation/capability/help/pair routes this phase adds. **Open item from P2.1:** `POST /watchdog/enable` and `POST /watchdog/disable` had no permission gate at all in MSC 1 (any authenticated token, including guest, could call them) — decide the category for these two routes here rather than carrying the gap forward silently.
**Verify:** `python3 tools/api-contract-check.py --v1-summary` → prints the route count and zero missing-category/missing-helpId violations
**Commit:** `P2.8: assemble the MSC 2 v1 OpenAPI contract`
**Batch:** stop-after

---

### Domain model extensions

### P2.9 — Port the operation domain types into `msc-domain`
**Status:** DONE
**Files:** `crates/msc-domain/src/operation.rs`, `crates/msc-domain/tests/operation.rs`, `crates/msc-domain/src/lib.rs`
**What:** Implement P2.5's schema in Rust: `OperationId`, the closed `OperationState` enum with its legal-transition rules (`queued→running→{succeeded,failed}`; any non-terminal state `→cancelled`; terminal states accept no further transition), `OperationProgress` (step/total plus a human-readable status line), and a result type carrying either a typed success value or P2.4's `ErrorDTO` shape. **This is new MSC 2 construction, not a port** — MSC 1 has no operation-journal concept, so D-018's evidence-before-translation discipline (which governs *ported* behavior) doesn't apply here; there is no MSC 1 fixture to extract. Verified with hand-written Rust unit tests covering every legal transition and rejecting every illegal one.
**Verify:** `cargo nextest run -p msc-domain operation` → all tests pass, including at least one illegal-transition rejection test
**Commit:** `P2.9: port operation domain types into msc-domain`
**Batch:** solo

### P2.10 — Port the capability domain type into `msc-domain`
**Status:** DONE
**Files:** `crates/msc-domain/src/capability.rs`, `crates/msc-domain/tests/capability.rs`, `crates/msc-domain/src/lib.rs`
**What:** Implement P2.6's schema as a pure `CapabilitySet` data type — agent version, API major/minor, host OS enum, per-server-type feature flags, and P2.1's permission-category enum. No I/O: the real detection logic that populates this type is Phase 3/4 infrastructure work, per the module-boundary rule in §6, not this crate's job.
**Verify:** `cargo nextest run -p msc-domain capability` → all tests pass
**Commit:** `P2.10: port capability domain type into msc-domain`
**Batch:** safe

---

### Skeletal agent

### P2.11 — `msc-api` crate: v1 DTOs
**Status:** DONE
**Files:** `Cargo.toml` (workspace), `crates/msc-api/Cargo.toml`, `crates/msc-api/src/lib.rs`, `crates/msc-api/src/dto/*.rs`, `.github/workflows/ci.yml`
**What:** New workspace member per `msc2-engineering.md` §6 ("routes · DTOs · WebSocket events... authentication · permission checks · rate limiting"). Hand-write serde structs for every schema P2.8's `openapi.json` defines that the skeletal agent will actually serve this phase: `OperationDTO`, `ErrorDTO`, `CapabilitiesDTO`, and the status/health DTOs. Add a conformance test serializing each DTO's example value and validating it against the matching `openapi.json` schema — the same schema-depth discipline P0.23 used, now checking Rust output against the schema instead of the schema's own shape. Extend CI (as P1.1 did for `msc-domain`) to build/lint/test this new crate on all three OSes.
**Verify:** `cargo nextest run -p msc-api dto_conformance` → all tests pass
**Commit:** `P2.11: scaffold the msc-api crate with v1 DTOs`
**Batch:** stop-after

### P2.12 — `msc-agent` crate: axum skeleton and dev-mode bearer auth
**Status:** DONE
**Files:** `crates/msc-agent/Cargo.toml`, `crates/msc-agent/src/main.rs`, `crates/msc-agent/src/auth.rs`, `.github/workflows/ci.yml`
**What:** New workspace member (§6: "service startup · dependency assembly · scheduler · operation recovery · static asset serving"). An `axum` server bound to loopback by default (§10: "the management API binds to loopback by default"), with a single dev-mode bearer-token middleware implementing P2.3's scoped-down auth — one fixed token from an env var, rejecting anything else with P2.4's `ErrorDTO` 401 shape — gating every route except `GET /v1/health`. The code comments this plainly as a development stand-in for the real pairing/`SecretStore` flow, not a preview of it.
**Verify:** `cargo run -p msc-agent -- serve --bind 127.0.0.1:48400 &` then `curl -s -o /dev/null -w '%{http_code}' localhost:48400/v1/health` → `200`; `curl -s -o /dev/null -w '%{http_code}' localhost:48400/v1/status` (no token) → `401`
**Commit:** `P2.12: scaffold the msc-agent crate with dev-mode bearer auth`
**Batch:** solo

### P2.13 — Skeletal handlers: status, health, capabilities
**Status:** DONE
**Files:** `crates/msc-agent/src/routes/status.rs`, `crates/msc-agent/src/routes/health.rs`, `crates/msc-agent/src/routes/capabilities.rs`
**What:** Wire `GET /v1/status`, `GET /v1/health`, `GET /v1/capabilities` to return canned `msc-api` DTOs — a single hard-coded fake server, honestly labeled as placeholder data wherever the schema allows a notes field. This is the minimum route set P2.20's iOS gate actually needs, and the port plan's own "exercised without real mutation" language for this phase.
**Verify:** `curl -s -H "Authorization: Bearer $MSC_DEV_TOKEN" localhost:48400/v1/status | python3 -m json.tool` → valid JSON matching the status schema, no server error
**Commit:** `P2.13: wire skeletal status, health, and capabilities handlers`
**Batch:** stop-after

### P2.14 — Skeletal handlers: operation lifecycle
**Status:** DONE
**Files:** `crates/msc-agent/src/routes/operations.rs`
**What:** Wire `POST /v1/operations`, `GET /v1/operations/{id}`, `POST /v1/operations/{id}/cancel` against an in-memory (non-journaled — that's Phase 3) map of id → `OperationState`. A background task advances a freshly-created operation through `queued→running→succeeded` over a few seconds so `GET` shows real progression; `cancel` legally transitions a `running` operation to `cancelled` per P2.9's state machine, and is rejected on a terminal one.
**Verify:** `curl -s -X POST -H "Authorization: Bearer $MSC_DEV_TOKEN" localhost:48400/v1/operations -d '{"type":"demo-install"}' | python3 -c "import json,sys;print(json.load(sys.stdin)['state'])"` → `queued`; polling `GET /v1/operations/{id}` a few seconds later → `succeeded`
**Commit:** `P2.14: wire skeletal operation lifecycle handlers`
**Batch:** safe

### P2.15 — Console WebSocket channel
**Status:** DONE
**Files:** `crates/msc-agent/src/ws/console.rs`
**What:** Reimplement P0.24's documented baseline behavior over `axum`'s WebSocket support at `/v1/console/stream`: same bearer auth as HTTP routes, the 200-line-backfill-then-live delivery model, the 5000-line ring buffer (D-021 point 2's bounded-memory rule starts applying here), and the 64 KB inbound-frame cap. With no real server process yet, backfill is a canned fixed line set and "live" lines come from a demo ticker, so the bounded-history-then-live behavior is actually observable end-to-end rather than asserted.
**Verify:** a short-lived WebSocket client (e.g. Python `websockets`) connects to `ws://127.0.0.1:48400/v1/console/stream` with the dev bearer token, receives the canned backfill immediately, then at least one live demo line within 5 seconds
**Commit:** `P2.15: wire the console WebSocket channel`
**Batch:** solo

### P2.16 — Operation-progress WebSocket channel
**Status:** DONE
**Files:** `crates/msc-agent/src/ws/operations.rs`
**What:** Wire `/v1/operations/{id}/stream` per P2.7's schema, pushing `OperationDTO` updates as P2.14's demo ticker advances the fake operation, with the same bearer auth and bounded-connection discipline P2.15 established.
**Verify:** connecting to `/v1/operations/{id}/stream` immediately after `POST /v1/operations` and reading frames shows the same `queued→running→succeeded` sequence P2.14's HTTP polling shows
**Commit:** `P2.16: wire the operation-progress WebSocket channel`
**Batch:** safe

### P2.17 — Contract-conformance checker against the live skeletal agent
**Status:** DONE
**Files:** `tools/contract-conformance-check.py`
**What:** A dependency-free Python script that calls every route this phase implements against a running `msc-agent` (health, status, capabilities, operation lifecycle) and validates each live JSON response against P2.8's `openapi.json` schema for that route — P0.23's schema-depth discipline, now pointed at a live server instead of a static document. This turns "a skeletal agent whose routes can be exercised" (the port plan's own words) into one command instead of manual `curl` checks.
**Verify:** `cargo run -p msc-agent -- serve --bind 127.0.0.1:48400 & sleep 1; python3 tools/contract-conformance-check.py --base-url http://127.0.0.1:48400 --token "$MSC_DEV_TOKEN"` → `ok <n>`, non-zero exit and a named route on any mismatch
**Commit:** `P2.17: build the contract-conformance checker`
**Batch:** solo

---

### iOS re-pointing

### P2.18 — Copy the existing iOS client into the msc2 repo
**Status:** DONE
**Files:** `clients/ios/` (new)
**What:** Per D-004 ("the existing SwiftUI iOS client is retained and re-pointed") and `CLAUDE.md` rule 8 (MSC 1 is read-only, always), copy `MSCiOS/MSCRemoteiOS.xcodeproj` and `MSCiOS/MSCRemoteiOS_Swift` verbatim from the oracle (`~/Documents/Swift Projects/minecraft-server-controller`) into `clients/ios/` in this repo — a straight file copy, no edits. This becomes the client this repo owns and evolves from here forward; the oracle copy is never touched.
**Verify:** `diff -rq "$HOME/Documents/Swift Projects/minecraft-server-controller/MSCiOS" clients/ios/` → no differences; `git -C "$HOME/Documents/Swift Projects/minecraft-server-controller" status --short` → empty (confirms the oracle itself was never written to)
**Commit:** `P2.18: copy the existing iOS client into clients/ios`
**Batch:** stop-after

### P2.19 — Repoint the iOS client's networking layer at the v1 skeletal agent
**Status:** DONE
**Files:** `clients/ios/MSCRemoteiOS_Swift/` (the Remote-API client class — identify the exact file during this step, since P2.18 is a verbatim copy and its layout isn't pre-known here)
**What:** Change the copied client's base URL and auth-header logic to target `http://127.0.0.1:48400/v1/` and send P2.3's dev bearer token, commented plainly as a temporary stand-in for the real QR-pairing flow (out of scope until Phase 3's `SecretStore` work lands). Hand-write the minimal Swift `Codable` model for `GET /v1/status`'s response — codegen is explicitly deferred, see this phase's "Not in this phase" note. Scope narrowly to the one call P2.20's gate needs; do not repoint the app's entire networking surface in this step.
**Verify:** `xcodebuild -project clients/ios/MSCRemoteiOS.xcodeproj -scheme MSCRemoteiOS build` → `BUILD SUCCEEDED`
**Commit:** `P2.19: repoint the iOS client's status call at the v1 skeletal agent`
**Batch:** solo

### P2.20 — Confirm the iOS app connects and reads status — closes the Phase 2 gate
**Status:** DONE
**Files:** none (verification only)
**What:** With `msc-agent` running locally and serving P2.13's status route, launch the repointed iOS app (a simulator is sufficient) and confirm its status screen renders the skeletal agent's canned data rather than an error or blank state. This is the port plan's Phase 2 exit criterion, verbatim: "the existing iOS app connects and reads status against a stub agent."
**Note:** `msc-agent` built and run on `127.0.0.1:48400`; the P2.19-repointed app built clean (`BUILD SUCCEEDED`) for `iPhone 17 Pro` simulator, installed, and launched without crashing. First attempt showed "STOPPED" / "Not paired" — self-inflicted: the agent had been started with an arbitrary `MSC_DEV_TOKEN`, not the `msc2-dev-token` value `SettingsStore.swift`'s `devDefaultToken` is supposed to pre-fill the app with. Restarting the agent with the matching token didn't fix the display, because of a second, real bug: `KeychainTokenStore.loadToken()` returns `""` rather than throwing when no token was ever saved, which defeats the `(try? loadToken()) ?? devDefaultToken` fallback in `SettingsStore.init()` — `tokenDraft` silently ends up `""` on every fresh install. Left as-is per Cameron's call (dev-only scaffolding, superseded by Phase 3's real pairing/`SecretStore` work) rather than patched here, since P2.20 is verification-only and this bug lives in the already-DONE P2.19. Cameron typed the token in manually instead: Home then showed **STATUS: RUNNING** in green, sourced live from `/v1/status`. The "Loading…"/"Loading servers…" text and the "Performance endpoint not available yet" notice below it are expected, not failures — `DashboardView.swift:49` falls back to "Loading…" whenever `vm.servers` is empty, and neither `/servers` nor `/performance` are routes the Phase 2 skeletal agent implements (only status/health/capabilities are, per P2.13); the app is correctly labeling gaps rather than hiding them. **Result: the app connects and reads live status from the skeletal agent — the exit criterion holds**, with the caveat that reaching it required a manual token workaround for the pre-fill bug above.
**Verify:** manual — Cameron runs the simulator against a running `msc-agent`, observes the status screen populated from it, and reports pass/fail. No scripted Verify is possible for a UI render.
**Commit:** _(n/a — verification only)_
**Batch:** stop-after

---

### Phase exit

### P2.21 — Phase 2 exit gate check
**Status:** DONE
**Files:** none (verification only)
**What:** Run every Phase 2 deliverable together: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace`, P2.8's contract checker, and P2.17's live conformance checker. Confirm `msc-domain` still carries no I/O dependency (unchanged from P1.15) and that `msc-api` carries no process/filesystem I/O beyond serialization. Re-confirm P2.20's manual iOS result is recorded in this file. This checks the port plan's own Phase 2 exit criterion verbatim: "the existing iOS app connects and reads status against a stub agent."
**Note:** All green, no fixes needed. `cargo fmt --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` clean; `cargo nextest run --workspace` → 215/215 passed; `tools/api-contract-check.py --v1-summary` → `routes: 93`, zero missing-category/missing-helpid/non-ErrorDTO violations; `tools/contract-conformance-check.py` against a live `msc-agent` (`MSC_DEV_TOKEN=msc2-dev-token`, same instance P2.20 left running) → `ok 6`. `crates/msc-domain/Cargo.toml` depends only on `regex` (serde/serde_json are dev-dependencies, test-only); `crates/msc-api/Cargo.toml` depends only on `serde`/`serde_json`. Grepped both crates' `src/` for `std::fs`, `std::net`, `std::process` (and `tokio` in `msc-domain`) — none found in either. P2.20's manual result is recorded at line 914 above (Status: DONE, RUNNING status confirmed live from `/v1/status`).
**Verify:** `cd ~/msc2 && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo nextest run --workspace` → all green; then `python3 tools/api-contract-check.py --v1-summary && python3 tools/contract-conformance-check.py --base-url http://127.0.0.1:48400 --token "$MSC_DEV_TOKEN"` → both `ok`
**Commit:** _(n/a — verification only, unless a fix is needed)_
**Batch:** stop-after

---

## Phase 3 — Safety substrate

**Gate** (`msc2-port-plan.md` §3): "Approved server roots and path safety · atomic writes · versioned configuration with migrations · `SecretStore` trait · audit log · download staging with checksum verification · operation journal · operation exclusivity. Windows CI begins here (D-017), covering path separators and length limits, file-locking semantics, service lifecycle, and case-insensitive path comparison." **Exit criteria: substrate fixtures pass on macOS, Linux, and Windows.**

**This phase is recorded as blocked, not merely proposed.** `msc2-decisions.md` D-025 (service identity and privilege boundaries) says so explicitly: "**Blocks:** Phase 3 (safety substrate) and the D-012 authentication design." D-025 is **Open** — "identified, not decided" — the weakest status in the register, one rung below even *Proposed*. Building `SecretStore` or any file-owning substrate code before answering "which account runs the agent, and who owns the files it creates" would be guessing at exactly the kind of platform-specific, wide-blast-radius question `msc2-engineering.md` §8 says was never guessed at. `msc2-engineering.md` §8 separately flags Linux secret storage itself as **unresolved** — the `keyring` crate's Secret Service backend doesn't exist on minimal Debian, a primary deployment target (D-011). Both gaps get their own step, first, in this plan — not resolved *by* this planning document (that would be code/config decided outside the Execute move), but scoped into a concrete, checkable step an Execute conversation can actually close, the same pattern P2.1–P2.3 used for Phase 2's own open items. **Read this plan with those three steps first** — they're the biggest judgment calls in it, and everything else in the phase is downstream of their answers.

**New workspace members.** Per `msc2-engineering.md` §6's module boundaries, this phase brings in `msc-infrastructure` ("filesystem repositories · HTTP providers · archives · process supervisor · metrics · config · audit · trait definitions for platform capabilities") and the first slice of the three platform crates — `msc-platform-macos`, `msc-platform-windows`, `msc-platform-linux` — but only their `SecretStore` implementation. Service registration (`launchd`/Windows Service/`systemd` installation), process supervision, Job Objects, cgroups, and the VZ sidecar client are named in §6 for those same crates but are **Phase 4 and Phase 10 work**, not this phase's.

20 steps plus one gate correction (P3.19a, added after P3.19's own tests found it), eleven groups:

| Group | Steps | Deliverable |
|---|---|---|
| Prerequisite decisions | P3.1–P3.3 | D-025 service identity scoped (Proposed, not Approved); D-012's Linux secret-storage backend chosen; Phase 3's substrate surface scoped, D-024/D-021 deferral recorded |
| Workspace scaffold | P3.4 | `msc-infrastructure` crate + `FileSystem` trait (real + fake impl), wired to CI on all three platforms |
| Path safety and atomic writes | P3.5–P3.6 | approved-server-root path resolution + a generic atomic-write primitive, both fixture-tested |
| Versioned configuration | P3.7 | the config-lifecycle mechanism — schema version, corruption recovery, unknown-field survival — as a reusable primitive |
| `SecretStore` | P3.8–P3.12 | the trait, three real platform implementations, and a cross-platform conformance check tying them together |
| Audit log | P3.13 | JSONL audit trail with 30-day retention |
| Download staging | P3.14 | generic stage → checksum-verify → move-into-place primitive |
| Operation journal and exclusivity | P3.15–P3.16 | restart-survivable operation journal; per-target operation locking |
| Leftover fixture domains | P3.17–P3.18 | `network-safety` (13 fixtures) and the `java-runtime-guards` filesystem leftover (8 fixtures), both deferred here from Phase 1 |
| Windows validation | P3.19–P3.19a | the first substrate tests that actually exercise Windows-specific behavior (D-017); P3.19a closes the case-sensitivity gap those tests found |
| Phase exit | P3.20 | full gate check across macOS, Linux, and Windows CI |

**Not in this phase**, deferred on purpose:

- **Real service registration** — installing the agent as a `launchd` LaunchDaemon, Windows Service, or `systemd` unit, and the OS integration testing that goes with it. That's Phase 4's gate ("headless service ownership proven on macOS, Linux, and Windows"). This phase only decides *identity and ownership* (P3.1); it doesn't install anything.
- **D-024 power management** (sleep inhibition, the two host-role policies, misconfiguration detection/warning). The port plan's own §3 prose for this phase names exactly eight substrate items and power management isn't one of them, even though the separate acceptance-test inventory (§4B) places "cross-platform sleep inhibition and the two power policies" here. Its verification shape — real OS power APIs (`IOPMAssertion`/`SetThreadExecutionState`/`systemd-inhibit`) — doesn't fit this phase's fixture-parity gate the way path safety or `SecretStore` do. Deferred to land with Phase 4's platform-service work, where "remote-starting a stopped server" (the scenario D-024 exists for) first becomes something the codebase can actually attempt. **Flagged for Cameron to confirm or overrule during the Read move** — this is a real scoping call, not an oversight.
- **D-021's headless-package GUI-link verification** ("CI check on every headless artifact: link no GUI framework"). A packaging concern, not blocked on anything this phase builds, and not yet assigned to a specific phase anywhere in the port plan — noted here rather than quietly invented a home for.
- **Real per-domain download workflows** — Paper/Xbox-jar/plugin/modpack downloads stay in their own phases (7–9). This phase builds only the shared staging primitive (P3.14) those workflows will call instead of each reimplementing temp-then-verify-then-move.
- **Wiring the new `SecretStore` into `msc-agent`'s real pairing flow**, replacing P2.3's fixed dev token with actual QR-pairing-issued, durably-stored credentials. `msc2-decisions.md`'s D-012 Phase 2 scope note says plainly that this is what Phase 3's `SecretStore` trait is *for* — but no phase in `msc2-port-plan.md` is actually named as the one that does this wiring. Recorded here as a genuine, currently-homeless gap (the same kind of finding D-027 exists to record), not left implicit. Cameron should decide during the Read move whether it belongs at the end of this phase, the start of Phase 4, or gets its own line in the port plan.
- **The rest of D-012's open items** (remote desktop pairing, LAN TLS provisioning, Tailscale posture, browser origin policy/CSRF) — untouched by this phase, exactly as Phase 2 left them.

---

### Prerequisite decisions

### P3.1 — Decide service identity and privilege boundaries for v1 (D-025)
**Status:** DONE
**Files:** `docs/msc2/substrate/service-identity.md`, `docs/msc2/msc2-decisions.md` (amend D-025)
**What:** D-025 asks six questions and is Open on all of them. Answer them as far as this phase can responsibly go — **Proposed, not Approved**, exactly like P2.1–P2.3 did for Phase 2's own open items — and name what genuinely can't be resolved by reading docs. Recommended direction, to confirm or overrule: on all three platforms, **the agent runs as the account that installed it** by default — macOS LaunchDaemon with `UserName` set to the installing user (not `root`), Windows Service "Log on as" that same user (not `SYSTEM`), Linux `systemd` unit with `User=`/`Group=` set the same way (not a dedicated system account) — rather than a separate service identity. This answers question 2 (file ownership) by construction: the agent's files are the installing user's files, so a desktop user opening, editing, or backing up them directly needs no special group membership or ACL dance. It answers question 3 (escalation): routine operation needs none; only writing the daemon/service/unit file at install time does, and that's already gated by the OS's own installer-elevation prompt, the same as installing MSC 1 today. Offer a dedicated-service-account mode as an explicitly **deferred, v1.1** option for a true multi-admin dedicated host with no single "owning" desktop user — not required now. **One sub-question this step cannot resolve by reading docs alone, and must say so rather than guess:** whether a macOS LaunchDaemon running with `UserName` set to a real user can actually reach that user's *login* Keychain — LaunchDaemons run outside any login session, and a `UserName` key does not, by itself, grant access to an unlocked login keychain. This directly decides whether P3.9's macOS `SecretStore` implementation targets the login keychain or the System keychain, so **P3.9 cannot start until this sub-question is closed** — flag it plainly rather than let P3.9 discover it mid-implementation. TCC (question 5) is recorded as unverifiable from docs alone and left as a known unknown to test once a LaunchDaemon actually exists (Phase 4), not guessed here.
**Verify:** `grep -c 'Phase 3 scope' docs/msc2/msc2-decisions.md` → at least `1`; `test -f docs/msc2/substrate/service-identity.md && echo present` → `present`
**Commit:** `P3.1: decide service identity and privilege boundaries for v1 (D-025)`
**Batch:** solo

### P3.2 — Decide the Linux headless secret-storage backend
**Status:** DONE
**Files:** `docs/msc2/substrate/secret-storage.md`, `docs/msc2/msc2-engineering.md` (amend §8's "Linux secret storage is unresolved" section)
**What:** §8 names three candidates and states its own preference: "`systemd` credentials (`LoadCredential=`/`systemd-creds`)... Preferred candidate." Confirm that choice in writing rather than let it stay an unconfirmed aside — D-011 already commits Linux to a `systemd` unit with zero desktop dependencies, so `systemd-creds` integrates with infrastructure this project already requires, and needs no second code path for a desktop-Secret-Service case, which §8 itself flags as undesirable ("two code paths and two threat models to reason about"). Record explicitly what this backend does and does not protect against at rest, per §8's own requirement, and how P3.1's service-identity answer (the agent running as the installing user, not a dedicated account) interacts with `systemd-creds`' usual `DynamicUser=` pairing — confirm it still works keyed to a normal user unit, or say plainly if it doesn't and adjust. Proposed, pending Cameron's confirmation, same pattern as every other judgment call in this register.
**Verify:** `grep -c 'systemd-creds\|LoadCredential' docs/msc2/substrate/secret-storage.md` → at least `1`
**Commit:** `P3.2: decide the Linux headless secret-storage backend`
**Batch:** solo

### P3.3 — Scope Phase 3's substrate surface and record what's deferred
**Status:** DONE
**Files:** `docs/msc2/substrate/phase3-scope.md`
**What:** Write down, in one place, the "Not in this phase" list already stated in this plan's own intro above — D-024 power management, D-021's headless-link verification, real service registration, real per-domain downloads, and the currently-homeless `SecretStore`-into-real-pairing wiring gap — as a scoping document Cameron can confirm or overrule during the Read move, the same role `auth-scope-phase2.md` played for Phase 2. This is the step that makes the deferrals load-bearing rather than just plan prose that could quietly drift once execution starts.
**Verify:** `grep -c '^##' docs/msc2/substrate/phase3-scope.md` → at least `5` (one heading per deferred item)
**Commit:** `P3.3: scope Phase 3's substrate surface and record what's deferred`
**Batch:** solo

---

### Workspace scaffold

### P3.4 — Scaffold the `msc-infrastructure` crate and the `FileSystem` trait
**Status:** DONE
**Files:** `Cargo.toml` (workspace), `crates/msc-infrastructure/Cargo.toml`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/src/fs.rs`, `crates/msc-infrastructure/tests/fs.rs`
**What:** New workspace member per `msc2-engineering.md` §6. Defined a `FileSystem` trait covering the minimal surface every later step in this phase needs (`read`, `write`, `stat`, `list`, `rename`, `remove`) with two implementations: a real `StdFileSystem` backed by `std::fs`, and an in-memory `FakeFileSystem` for tests — constructible via `FakeFileSystem::from_tree(&serde_json::Value)`, which consumes the exact `fsTree` shape P0.5's deferred fixtures already use (`{"<path>": {"type": "file", "executable": true}}`), so P3.18 can build one straight from a fixture's `input.fsTree` without reshaping it. Depends on `msc-domain` (inward, per the direction rule); nothing depends on it yet. **`.github/workflows/ci.yml` needed no change** — P2.11 already generalized `Build`/`Test` to `--workspace`, so adding the crate to the workspace's `members` list was enough for CI to pick it up on all three OSes; confirmed by reading the file rather than assumed.
**Verify:** `cargo build -p msc-infrastructure && cargo nextest run -p msc-infrastructure fs` → passes (matches 1 test by nextest's substring rule — `fake_file_system_builds_from_fixture_fs_tree`, which does exercise `FakeFileSystem`); `cargo nextest run -p msc-infrastructure` (no filter) → `4 tests run: 4 passed`
**Commit:** `P3.4: scaffold the msc-infrastructure crate and FileSystem trait`
**Batch:** stop-after

---

### Path safety and atomic writes

### P3.5 — Characterize and port approved-server-root path safety
**Status:** awaiting verification
**Files:** `fixtures/path-safety/`, `crates/msc-infrastructure/src/path_safety.rs`, `crates/msc-infrastructure/tests/path_safety.rs`
**What:** MSC 1 has no dedicated test file for this; the reference implementations are `resolvedServerFileURL` in `AppViewModel+APIWiringContent.swift` (already flagged in the symbol ledger: "a real PATH-TRAVERSAL SAFETY CHECK: resolves symlinks and requires the resolved path to stay within the server's root directory... directly the kind of path-safety policy `msc2-port-plan.md`'s Phase 3 substrate calls for") and `validateResetDeletionTarget` in `AppViewModel+ConfigHelpers.swift` (refuses to delete `/`, the home directory, `/Applications`, or anything outside the actual approved root — the ledger's own words: "this IS the kind of path-safety policy... Phase 3 substrate... calls for"). Characterize both into one `safe_path(root, requested) -> Result<PathBuf, PathSafetyError>` primitive, built on P3.4's `FileSystem` trait for the symlink-resolution step so it's testable without touching the real filesystem: standardize and resolve symlinks on both root and candidate, require the resolved candidate to equal the root or start with `root + separator`. New fixtures, characterized directly from the two source functions per `fixture-format.md`'s "MSC 1 run by hand" standard for untested pure logic: (1) a plain in-root path, (2) a `..`-escape attempt, (3) a symlink inside the root pointing outside it, (4) the empty-relative-path case (candidate equals root), (5) a forbidden absolute path (`/`), (6) a forbidden absolute path (the home directory), (7) a sibling directory sharing the root's name as a string prefix (e.g. `/srv/server1x` vs. root `/srv/server1`) — the classic off-by-one a naive `hasPrefix` check gets wrong, which this fixture pins down as *not* an escape.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/path-safety --expect 7` → `ok 7`; then `cargo nextest run -p msc-infrastructure path_safety` → `7 tests run: 7 passed`
**Commit:** `P3.5: characterize and port approved-server-root path safety`
**Batch:** solo

### P3.6 — Atomic write primitive
**Status:** DONE
**Files:** `fixtures/atomic-write/`, `crates/msc-infrastructure/src/atomic_write.rs`, `crates/msc-infrastructure/tests/atomic_write.rs`
**What:** MSC 1's temp-file-then-rename pattern recurs everywhere without ever being its own primitive: `ConfigManager.save` ("atomically encodes+writes config.json"), `WorldSlotManager.createSlot`/`.updateSlotFromCurrentWorld` ("zips to a temp file then atomically replaces, so a failed zip never corrupts the existing archive"), `AppViewModel+WorldSlots.restoreSlotBackup` ("atomically swaps the slot's world.zip via a temp-file copy+move"). Build the one reusable `atomic_write(path, bytes) -> Result<(), AtomicWriteError>` every later config/metadata/world writer will call instead of reimplementing temp-then-rename per call site. New fixtures, characterizing the pattern itself rather than one call site: (1) a successful write to a new path, (2) overwriting an existing file replaces its content correctly, (3) a missing parent directory produces a clear error and leaves no partial file behind, (4) the pre-existing destination file is untouched when a write is interrupted before the rename step (simulated by writing the temp file and asserting the destination's content is unchanged without performing the rename).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/atomic-write --expect 4` → `ok 4`; then `cargo nextest run -p msc-infrastructure atomic_write` → `4 tests run: 4 passed`
**Commit:** `P3.6: build the atomic write primitive`
**Batch:** solo

---

### Versioned configuration

### P3.7 — Versioned configuration with migrations
**Status:** DONE
**Files:** `fixtures/config-lifecycle/`, `crates/msc-infrastructure/src/config_repository.rs`, `crates/msc-infrastructure/tests/config_lifecycle.rs`
**What:** Port the *mechanism* `ConfigManager.swift` demonstrates (symbol-ledger rows `init`/`reload`/`save`, lines 40–236) — not MSC 1's specific `AppConfig` schema, which is Phase 5's job (`fixtures/config-roundtrip/`, already deferred there by the Phase 1 plan, and needs the historical `server_config_swift.json` corpus this phase doesn't have). What this phase owns is the generic policy every later versioned-config consumer will reuse: a schema-version field on every saved config; on decode failure, preserve the corrupt file as a timestamped `.corrupt-<ts>` copy before falling back to defaults, rather than overwriting the evidence (`ConfigManager.init`'s R3 behavior); unknown fields survive a read-modify-write round trip instead of being silently dropped — `msc2-engineering.md` §7's own justification for this, "silently rewriting `server.properties` with only the recognized keys is destructive," generalized here to any versioned config, not just `server.properties`. Saves go through P3.6's atomic-write primitive. New fixtures, characterized directly from `ConfigManager.swift` since no MSC 1 test file exercises corruption recovery specifically: (1) a valid file loads cleanly, (2) a missing file falls back to defaults, (3) corrupted JSON is preserved as `.corrupt-<ts>` and defaults are returned, (4) a file carrying unknown/future fields round-trips those fields unchanged.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/config-lifecycle --expect 4` → `ok 4`; then `cargo nextest run -p msc-infrastructure config_lifecycle` → `4 tests run: 4 passed`
**Commit:** `P3.7: build the versioned configuration and migration primitive`
**Batch:** solo

---

### `SecretStore`

### P3.8 — Design the `SecretStore` trait and its cross-platform contract fixtures
**Status:** DONE
**Files:** `docs/msc2/substrate/secret-storage.md` (extend from P3.2), `crates/msc-infrastructure/src/secret_store.rs`, `fixtures/secret-store-contract/`
**What:** Generalize `KeychainManager.swift`'s five hardcoded read/write/delete pairs (Remote API admin token, Remote API guest token, per-server Xbox broadcast alt-password, Playit secret key, CurseForge API key — `readRemoteAPIToken`/`writeRemoteAPIToken`/`readXboxBroadcastAltPassword`/etc., lines 53–132) into one keyed trait: `trait SecretStore { fn get(&self, key: &str) -> Result<Option<String>>; fn set(&self, key: &str, value: &str) -> Result<()>; fn delete(&self, key: &str) -> Result<()>; }`, replacing MSC 1's five bespoke `service`/`account` pairs (`read(service:account:)`/`write(service:account:)`/`delete(service:account:)`, lines 162–228) with a documented key-naming scheme, so a new secret kind never needs a new trait method again. Record, per P3.1's flagged sub-question, which Keychain scope the macOS implementation targets and why — this document is where that answer must land before P3.9 can start. Write the five contract fixtures every platform implementation must satisfy, reusable rather than tied to one platform: (1) round-trip — set then get returns the same value, (2) reading a never-set key returns `None`, not an error, (3) `set` on an existing key overwrites it, (4) delete then get returns `None`, (5) deleting a never-set key is a no-op, not an error.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/secret-store-contract --expect 5` → `ok 5`
**Commit:** `P3.8: design the SecretStore trait and its cross-platform contract fixtures`
**Batch:** solo

### P3.9 — `SecretStore` for macOS: Keychain
**Status:** DONE
**Files:** `crates/msc-platform-macos/Cargo.toml`, `crates/msc-platform-macos/src/lib.rs`, `crates/msc-platform-macos/src/secret_store.rs`, `.github/workflows/ci.yml`
**What:** New workspace member — this step builds only the Keychain piece of `msc-platform-macos`; `launchd` registration and the VZ sidecar client (also named for this crate in §6) are Phase 4 and Phase 10. Implement P3.8's `SecretStore` trait against macOS Keychain Services at the scope P3.1 resolved (System keychain), via the `security-framework` crate: `MacosSecretStore::system()` opens `/Library/Keychains/System.keychain`; `get`/`set`/`delete` wrap `SecKeychainFindGenericPassword`/`SecKeychainAddGenericPassword`(via the crate's own upsert-aware `set_generic_password`)/`SecKeychainItemDelete`, folding `errSecItemNotFound` into `Ok(None)`/`Ok(())` per `KeychainManager`'s own documented behavior. Shared `msc_infrastructure::secret_store::run_contract_fixture[s]` helper added (used by this crate and, going forward, P3.10/P3.11) so every platform replays the same fixture JSON instead of a hand-copied assertion per platform. **Testability finding, empirically confirmed, not assumed:** an ordinary unprivileged process cannot write to the System keychain at all — `SecKeychainAddGenericPassword` against it returns `errSecWrPerm` (code -61) outright, no prompt, confirmed by direct test on this machine. This is a stronger and more general fact than the LaunchDaemon-session question P3.1/P3.8 flagged as open — it holds for *any* unprivileged writer, daemon or not. Production still targets the System keychain exactly as confirmed; the crate's own tests instead build a throwaway, password-less keychain file via `SecKeychain`'s `CreateOptions::create` (deleted on drop) and run `MacosSecretStore::with_keychain(...)` against that, exercising the identical Find/Add/Delete calls without requiring root or an admin-authorization prompt. Decided without asking — a test-construction choice, not a product/scope decision; production code path and confirmed scope are unchanged. Real provisioning onto the System keychain (which does need root) is the same install-time-elevation-window question `secret-storage.md` §4 already flags for the Linux backend, not new scope for this step to resolve. `cargo check --target x86_64-unknown-linux-gnu`/`--target x86_64-pc-windows-msvc` both confirm the crate cross-compiles cleanly on non-macOS (the `security-framework`/`-sys` dependencies are `[target.'cfg(target_os = "macos")'.dependencies]`, and the module itself is `#[cfg(target_os = "macos")]`-gated) — so, as P3.4 found for its own crate, `.github/workflows/ci.yml` needed no change; the existing `--workspace` build/test steps already cover it on all three matrix OSes.
**Verify:** (macOS CI runner) `cargo nextest run -p msc-platform-macos secret_store_contract` → `5 tests run: 5 passed`
**Commit:** `P3.9: implement SecretStore for macOS Keychain`
**Batch:** safe — the design judgment already happened in P3.8; this step implements against P3.8's already-fixed trait and five contract fixtures, the same shape as P1.3–P1.7's mechanical fixture ports

### P3.10 — `SecretStore` for Windows: Credential Manager
**Status:** DONE (CI run confirmed: `gh run view 30688818826` — `Toolchain (windows-latest)` job, `Test` step, all 5 `secret_store::tests::secret_store_contract_*` PASS, `245 tests run: 245 passed, 0 skipped`)
**Files:** `crates/msc-platform-windows/Cargo.toml`, `crates/msc-platform-windows/src/lib.rs`, `crates/msc-platform-windows/src/secret_store.rs`
**What:** New workspace member — this step builds only the Credential Manager/DPAPI piece of `msc-platform-windows`; Windows Service registration, Job Objects, and firewall handling (also named for this crate in §6) are Phase 4. Implement P3.8's trait against **Windows Credential Manager** (`CredWriteW`/`CredReadW`/`CredDeleteW` on `CRED_TYPE_GENERIC`, via the raw `windows-sys` bindings — Credential Manager wraps DPAPI for the actual at-rest encryption, confirming this step's own "which backend" question rather than assuming one), persisted with `CRED_PERSIST_LOCAL_MACHINE` — the scope D-025/`service-identity.md` §50 already confirmed (DPAPI *user*-scope; the constant's name is misleading — it means "survives across this user's logon sessions," not "shared machine-wide"; every Credential Manager entry is tied to the calling account regardless). This matches cleanly because the service already runs as the installing user (P3.1), not `LocalSystem` — no daemon/session mismatch to design around here, unlike macOS. One `TargetName` per `SecretStore` key, prefixed `MSC2:`. Run P3.8's five contract fixtures against it, via the same shared `msc_infrastructure::secret_store::run_contract_fixture` P3.9 built. **Note fixed in `msc2-decisions.md`/`secret-storage.md` while implementing this:** both documents call the Windows answer "DPAPI machine-scope" in one place each (D-025's own summary line and `secret-storage.md` §10's macOS-comparison paragraph) while D-025's detailed §521-523 text and `service-identity.md` §50 — the actual reasoning — say user-scope; user-scope is correct (own decision, restated above) and is what `CRED_PERSIST_LOCAL_MACHINE` actually produces, so this is a wording slip in the two summary mentions, not a live design question. Not corrected in those files by this step (out of P3.10's own file scope) — flagged for Cameron. Test-hygiene note, same shape as P3.9's throwaway-keychain finding: Credential Manager has no cheap disposable-instance equivalent to a keychain file — a fresh `CredWrite` from an ordinary process succeeds outright (no elevation needed; this is exactly what "DPAPI user-scope" buys), so the blocker P3.9 hit doesn't recur, but writing under the real `MSC2:` namespace during tests would risk colliding with genuine secrets on a machine that also runs the real agent. Tests instead use `WindowsSecretStore::with_prefix("MSC2-contract-test:")` and delete each fixture's key both before and after (via a new shared `msc_infrastructure::secret_store::contract_fixture_key` helper, added for this) — decided without asking, a test-construction choice with no product-behavior effect. **Verification status:** cannot run on this (macOS) development machine — no Windows kernel to call these APIs against. Instead: (1) `cargo check --workspace --target x86_64-pc-windows-msvc --all-targets` — full type-check of both the implementation and its test module against real `windows-sys` Win32 signatures, passes; (2) `cargo build --workspace` / `clippy --workspace --all-targets -- -D warnings` / `fmt --all -- --check` on macOS natively — all pass, confirming the crate's `[target.'cfg(target_os = "windows")'.dependencies]` gating and `#[cfg(target_os = "windows")]` module gating keep it a no-op elsewhere, so (as P3.4 and P3.9 both found for their own crates) `.github/workflows/ci.yml` needed no change; (3) the real test run happens automatically the next time CI runs on `windows-latest`, which is what this step's own Verify line already names as the authority — Cameron's verification is that CI run, not a local rerun.
**Verify:** (Windows CI runner) `cargo nextest run -p msc-platform-windows secret_store_contract` → `5 tests run: 5 passed`
**Commit:** `P3.10: implement SecretStore for Windows Credential Manager`
**Batch:** safe — same reasoning as P3.9: implements against P3.8's already-fixed contract, no open design question left

### P3.11 — `SecretStore` for Linux: the backend P3.2 chose
**Status:** DONE (CI run confirmed: `gh run view 30689870770` — `Toolchain (ubuntu-latest)` job, `Test` step, all 5 `secret_store::tests::secret_store_contract_*` PASS plus `key_file_and_secrets_dir_are_owner_only`, `246 tests run: 246 passed, 0 skipped`. First push, `30689732086`, failed clippy — `collapsible_if` in `default_base_dir`, a lint only `cargo clippy --target x86_64-unknown-linux-gnu` catches, which the original local check didn't run; fixed in a follow-up commit, re-pushed, green.)
**Files:** `crates/msc-platform-linux/Cargo.toml`, `crates/msc-platform-linux/src/lib.rs`, `crates/msc-platform-linux/src/secret_store.rs`
**What:** New workspace member — this step builds only the secret-store piece of `msc-platform-linux`; the `systemd` unit itself and cgroups handling (also named for this crate in §6) are Phase 4. Started as "implement P3.8's trait against whichever backend P3.2 actually decided" — building it against P3.2's confirmed `systemd-creds` choice surfaced a real shape mismatch, not a mechanical implementation detail: `systemd-creds encrypt`/`decrypt`, called directly by a running process rather than by `systemd` itself at unit-start, require root on any machine without a TPM2 chip — confirmed against the `systemd-creds` manpage and three still-open upstream bug reports of others hitting the same thing (`systemd/systemd#30191`, `#33318`, `#36895`). `systemd-creds`'s real design is "root decrypts a fixed list once, at unit start" — not "a live, on-demand `get`/`set`/`delete` API," which is what `SecretStore` needs and what P3.9/P3.10 both actually support natively on their platforms. Surfaced to Cameron as QUESTION 1 before proceeding rather than silently routing around it (e.g. baking a `sudo` call into the agent's own runtime code, which would have quietly reopened P3.1's already-confirmed "routine operation needs no escalation" answer). **Cameron Temple confirmed, 2026-08-01: build the recommended two-track answer** — full record in `docs/msc2/substrate/secret-storage.md` §12 and `msc2-decisions.md` D-025's amendment: **Track 1 (the real target, not built now)** — a small privileged helper the installer sets up at the same elevated moment it already writes the `systemd` unit file, which alone ever touches `systemd-creds`; deferred to land with Phase 4's real service registration, per `phase3-scope.md`'s own boundary. **Track 2 (built here, the explicit v1 stand-in)** — `LinuxSecretStore`: one file per secret under `$XDG_DATA_HOME/msc2/secrets` (falls back to `$HOME/.local/share/msc2/secrets`), each encrypted with ChaCha20-Poly1305 (the `chacha20poly1305` crate, `[target.'cfg(target_os = "linux")'.dependencies]`) using a per-installation key generated on first use and stored alongside it (`<base>/key`, mode `0600`; `secrets/` directory mode `0700`) — owned by the agent's own installing-user account (P3.1), not root, so no elevation is needed for `get`/`set`/`delete` at any point, unlike `systemd-creds`. Writes go through P3.6's `atomic_write` primitive. Run P3.8's five contract fixtures against it via the same shared `msc_infrastructure::secret_store::run_contract_fixture` helper P3.9/P3.10 use, each against a throwaway temp directory (this backend's equivalent of P3.9's throwaway keychain — cheaper here, since creating one needs no special API, just a directory). A sixth, non-fixture test (`key_file_and_secrets_dir_are_owner_only`) checks the `0600`/`0700` permissions directly, since the five contract fixtures characterize behavior, not file-mode side effects. **Verification status, same shape as P3.10:** cannot run natively on this (macOS) development machine. Instead: (1) `cargo check --workspace --target x86_64-unknown-linux-gnu --all-targets` — full type-check of the implementation and its test module against the real `chacha20poly1305` 0.10.1 API, passes; (2) `cargo build --workspace` / `clippy --workspace --all-targets -- -D warnings` / `fmt --all -- --check` on macOS natively — all pass, confirming the crate's `[target.'cfg(target_os = "linux")'.dependencies]` and `#[cfg(target_os = "linux")]` gating keep it a no-op elsewhere (workspace test count unchanged at 245 after adding this crate, confirming zero tests leak into the macOS run); (3) as P3.4/P3.9/P3.10 all found for their own crates, `.github/workflows/ci.yml` needed no change — the real test run happens automatically on `ubuntu-latest`, which is what this step's own Verify line already names as the authority.
**Verify:** (Linux CI runner) `cargo nextest run -p msc-platform-linux secret_store_contract` → `5 tests run: 5 passed`
**Commit:** `P3.11: implement SecretStore for Linux`
**Batch:** safe — same reasoning as P3.9/P3.10

### P3.12 — Cross-platform `SecretStore` conformance summary
**Status:** DONE
**Files:** `docs/msc2/substrate/secret-storage.md` (extended with §13, a threat-model comparison table)
**What:** Confirmed all three platforms' `secret_store_contract` suites (P3.9–P3.11) ran in the same CI run — [run 30689870770](https://github.com/ctemple9/msc2/actions/runs/30689870770), all three `Toolchain (*)` matrix jobs green, each running its own five contract tests (Linux also runs the `key_file_and_secrets_dir_are_owner_only` permissions check). Added `secret-storage.md` §13: one row per platform (backend, scope, protects-against, does-not-protect-against), reflecting what's actually running today — notably, the Linux row is P3.11's v1 file-based stand-in, **not** the originally-confirmed `systemd-creds` (§12's finding), called out explicitly rather than silently left inconsistent with §§1–7. Closing observation added: all three platforms share the same *shape* of boundary ("recoverable by anything with this account's access on this machine," never truly session-scoped, per §2/§10's own reasoning), and Linux is currently the one exception — its key isn't bound to the specific machine's hardware the way macOS's System keychain and Windows' DPAPI both are — named as a direct, expected consequence of shipping track 2 instead of track 1, not an oversight.
**Verify:** `gh run list --limit 1` → `success`, with the run log showing all three `secret_store_contract` suites; `grep -c '^|' docs/msc2/substrate/secret-storage.md` → the comparison table has at least 3 data rows (one per platform)
**Commit:** `P3.12: cross-platform SecretStore conformance summary`
**Batch:** stop-after

---

### Audit log

### P3.13 — Port the audit log
**Status:** DONE
**Files:** `fixtures/audit-log/`, `crates/msc-infrastructure/src/audit_log.rs`, `crates/msc-infrastructure/tests/audit_log.rs`
**What:** Port `AuditLogger.swift` (139 lines): a JSONL audit trail for Remote API mutations and auth failures, one file per UTC day, a dedicated serial writer queue so concurrent callers never interleave writes, and 30-day retention (`pruneOldFiles`). Entry shape: timestamp, client IP, token label, method, path, status code (`Entry`, lines 14–20) — matches `msc2-engineering.md` §7's "commands and administrative actions are attributed to the local GUI, a CLI user, or a named remote token." New fixtures, characterized directly from source since no MSC 1 test file exercises this: (1) a single entry round-trips through JSONL correctly, (2) entries from concurrent writers preserve call order (the serial-queue guarantee), (3) a file older than the 30-day retention window is pruned, (4) a file exactly at the 30-day boundary is kept, (5) a corrupt or partial line in an existing file doesn't crash the writer. Built `AuditLog` as a `FileSystem`-backed primitive (own internal `Mutex`, not an actual background thread — same non-interleaving/ordering guarantee `AuditLogger`'s serial `DispatchQueue` gives, without a thread to get it). Two deliberate, documented deviations from the Swift source, both flagged for Cameron rather than silently made: (a) unlike `AuditLogger.write` (lines 97-104), this primitive does not create its own output directory — follows the same "caller ensures the parent exists" rule `atomic_write` (P3.6) and `config_repository` (P3.7) already established this phase, rather than adding a third convention or extending the `FileSystem` trait (P3.4, already DONE) with a new method; (b) `pruneOldFiles` compares each file's real filesystem creation timestamp against a fractional-day cutoff, but `FileSystem` exposes no creation time — since every audit file is named for the UTC day it holds, retention here reads that same date back out of the filename instead, in whole days rather than a 86,400-second window. `RETENTION_DAYS = 30` compared with strict `>` (mirrors `pruneOldFiles`'s own `created < cutoff`), so a file exactly 30 days old is kept, 31 is pruned. `format_iso8601`/date arithmetic hand-written (Howard Hinnant's public-domain `days_from_civil`/`civil_from_days`) rather than adding a date/time crate dependency, consistent with this workspace staying dependency-free beyond `serde_json`. A sixth, non-fixture check (`concurrent_threads_preserve_submission_order`, folded into the second fixture's own test function rather than a separate `#[test]`, to keep the nextest count at 5 — same pattern P3.7's `config_lifecycle.rs` used for its own extra `corrupt_backup_path` check) spawns real OS threads to confirm the ordering guarantee holds under genuine concurrency, not just sequential submission.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/audit-log --expect 5` → `ok 5`; then `cargo nextest run -p msc-infrastructure audit_log` → `5 tests run: 5 passed`
**Commit:** `P3.13: port the audit log`
**Batch:** solo

---

### Download staging

### P3.14 — Download staging with checksum verification
**Status:** DONE
**Files:** `fixtures/download-staging/`, `crates/msc-infrastructure/src/download_staging.rs`, `crates/msc-infrastructure/tests/download_staging.rs`
**What:** `msc2-engineering.md` §7: "Downloads land in a temporary location, are checksum-verified where the provider publishes one, and are moved into active use only after validation. Interrupted downloads are safely retryable. Cached files record origin and version." Every real MSC 1 download-and-install workflow — `AppViewModel+PaperTemplateDownload.downloadLatestPaperTemplate`, `AppViewModel+XboxBroadcastDownload.downloadOrUpdateXboxBroadcastJar`, `AppViewModel+PluginManagement.downloadLatestForPlugin` ("streams the download to a temp file... moves the new file into place") — repeats the same temp-then-verify-then-move shape without ever sharing a primitive. Those per-domain workflows stay in their own phases (7–9, where their loader/provider-specific logic belongs); this phase builds the one reusable primitive they'll call instead of each reimplementing it: `stage_download(dest, bytes, expected_checksum: Option<...>) -> Result<CachedFile, DownloadStagingError>`, where `CachedFile` records origin URL and version alongside the moved file, per §7's "cached files record origin and version." New fixtures: (1) a successful stage-and-move with a matching checksum, (2) a checksum mismatch is rejected and the temp file is cleaned up without touching any existing destination file, (3) no checksum published still stages and moves, just unverified, (4) an interrupted download (a partial temp file left over from a prior attempt) is safely retried rather than corrupted or double-appended. Built `stage_download` to verify the checksum against the in-memory `bytes` *before* touching disk at all, then hand off to P3.6's `atomic_write` — so a checksum mismatch (case 2) never creates a temp file to begin with, rather than creating one and having to remember to clean it up; case 4's "safely retried" guarantee falls directly out of `FileSystem::write` always replacing a file's full contents (`fs.rs`), so a stale leftover temp file from a prior crashed attempt is overwritten in full, never appended to — no new logic needed to make that true. None of MSC 1's three named download workflows actually checksum-verifies what it downloads; the nearest real precedent for the checksum format is `ResourcePackManager.sha1Hex` (hex-encoded SHA1), used here rather than inventing an algorithm — compared case-insensitively, since providers vary in hex case. SHA1 itself is hand-written (`download_staging::sha1_hex`, `pub` rather than private) instead of adding a crate dependency, keeping this crate dependency-free beyond `serde_json` per the pattern P3.13's date arithmetic already set; correctness verified two ways — the four fixtures' own expected hashes were computed out-of-band (Python's `hashlib`), and a fifth check folded into the first fixture's own test function (`tests/download_staging.rs`, not a separate `#[test]` — same reasoning as P3.7's `config_lifecycle.rs` and P3.13's `audit_log.rs`, to keep the nextest count at exactly 4) runs the published FIPS 180-4 / RFC 3174 test vectors directly against `sha1_hex`.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/download-staging --expect 4` → `ok 4`; then `cargo nextest run -p msc-infrastructure download_staging` → `4 tests run: 4 passed`
**Commit:** `P3.14: build the download staging primitive`
**Batch:** solo

---

### Operation journal and exclusivity

### P3.15 — Operation journal (restart survival)
**Status:** DONE
**Files:** `crates/msc-infrastructure/src/operation_journal.rs`, `crates/msc-infrastructure/tests/operation_journal.rs`
**What:** New MSC 2 construction, not a port — MSC 1 has no operation-journal concept, the same D-018 exemption P2.9 recorded for the `OperationState` machinery this builds on. `OperationJournal` writes one `<dir>/<id>.json` file per operation (mirroring `AuditLog`'s one-file-per-day convention), via P3.6's atomic-write primitive, so a reader — including a concurrent `reconcile_on_startup` — never observes a half-written entry. `reconcile_on_startup` reads every journaled entry and, for any left non-terminal, transitions it via `msc-domain`'s own `OperationState::transition_to` rather than writing a new state by hand: a `running` entry becomes `failed`, carrying a fresh `OperationError` (`code: "operation_interrupted"`, `message: "agent restarted mid-operation"`); a `queued` entry becomes `cancelled` instead, since `queued -> failed` is not a legal transition in P2.9's own table and there's nothing to silently resume it into. Both outcomes satisfy §7's "incomplete operations are reconciled and their outcome explained rather than silently forgotten" — reconciliation returns a `ReconciliationRecord` (id/from/to/reason) per entry actually touched, and re-journals each one so the on-disk state matches. Already-terminal entries are left byte-for-byte alone. Hand-written Rust tests against `FakeFileSystem`, same style as P2.9's `operation.rs`: (1) a completed operation's journal entry is inert on restart — `succeeded`, `failed` (pre-existing error preserved untouched), and `cancelled` entries all round-trip unchanged; (2) a `running` entry is reconciled to `failed` carrying the "agent restarted mid-operation" `OperationError`; (3) a `queued` entry is reconciled to `cancelled`, with no error attached, per operation-model.md §2's "cancellation carries neither result nor error"; plus a plain record/load round-trip, a never-journaled-id lookup, and a mixed-batch test confirming only the two non-terminal entries in a group of three actually reconcile.
**Verify:** `cargo nextest run -p msc-infrastructure operation_journal` → `6 tests run: 6 passed`
**Commit:** `P3.15: build the operation journal`
**Batch:** solo

### P3.16 — Operation exclusivity
**Status:** DONE
**Files:** `crates/msc-infrastructure/src/operation_journal.rs` (extend), `crates/msc-infrastructure/tests/operation_exclusivity.rs`
**What:** §7: "Only one conflicting operation runs against a server at a time. Starting a backup during a world replacement is refused, not queued silently." Added `OperationJournal::admit`, a per-target check layered in front of P3.15's `record`: before journaling a new entry, it walks the journal (same file-walk shape as `reconcile_on_startup`) looking for any other non-terminal (`queued`/`running`) entry that already shares the new entry's target; if one exists, the new entry is rejected with `AdmitError::Conflict` and never journaled at all — "refused, not queued silently" is the port plan's own wording, and the existing operation is left completely untouched. An entry with no target (`target: None`) never conflicts with anything, since there is no shared target to hold exclusively. `AdmitError::Conflict` carries an `OperationError` — already P2.4's `ErrorDTO` shape per that type's own doc comment in `msc-domain::operation`, so no new wire-shape type was invented — with a new `code: "operation_conflict"` (operation-model.md §4.3 explicitly leaves that choice to whichever phase implements exclusivity) and `details` naming the conflicting operation's id and the shared target. `record` itself is untouched and still used bare by `reconcile_on_startup`, which must be able to re-journal an entry that already passed the check once without re-running it. The conflict rule itself is deliberately coarse here — same-target-any-operation conflicts with same-target-any-operation — since the real operation-type catalog (which types may safely coexist) doesn't exist until later phases populate it; a fine-grained matrix is flagged as a later-phase refinement, not invented now. Five hand-written tests, same style as P3.15's `operation_journal.rs`: a running same-target operation rejects a new one; a merely-queued same-target operation rejects one too (exclusivity isn't running-only); different targets both admit; a terminal same-target operation (already `succeeded`) does not block, since it no longer holds anything; and two `target: None` operations both admit since untargeted operations never conflict.
**Verify:** `cargo nextest run -p msc-infrastructure operation_exclusivity` → `5 tests run: 5 passed`
**Commit:** `P3.16: build operation exclusivity`
**Batch:** solo

---

### Leftover fixture domains

### P3.17 — Port network-safety fixtures
**Status:** awaiting verification
**Files:** `crates/msc-domain/src/network_safety.rs`, `crates/msc-domain/tests/network_safety.rs`
**What:** Port `NetworkSafety.isLocalOrPrivateHost` and its supporting classification logic (`NetworkSafety.swift`) against the 13 fixtures P0.14 already extracted (`fixtures/network-safety/`) — loopback, private-range including the 172.16.0.0/12 boundary case, mDNS/`.local`, IPv6 loopback and link-local, and public-address rejection. Pure function, no I/O, so it lives in `msc-domain` alongside the other Phase 1 domains despite landing in this phase — deferred here by the Phase 1 plan's own note, thematically because it backs D-012's LAN-encryption and off-loopback safety questions this phase's substrate work sits next to, not because it needs any capability `msc-domain`'s no-I/O crate lacks.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/network-safety --expect 13` → `ok 13`; then `cargo nextest run -p msc-domain network_safety` → `13 tests run: 13 passed`
**Commit:** `P3.17: port network-safety fixtures`
**Batch:** safe

### P3.18 — Port the java-runtime-guards filesystem leftover
**Status:** awaiting verification
**Files:** `crates/msc-infrastructure/src/java_runtime_detection.rs`, `crates/msc-infrastructure/tests/java_runtime_detection.rs`
**What:** The 8 fixtures P1.5 explicitly deferred ("move to `msc-infrastructure` once Phase 3 builds the filesystem substrate behind a trait"): `detect-installed-java-runtimes-*` (×3 — macOS JDK-bundle layout, plain `JAVA_HOME`-style layout, invalid-candidate rejection) and `normalization-*` (×5 — already-executable path unchanged, bare command passthrough, directory-without-`bin/java` error, home-dir-to-`bin/java` resolution, nonexistent-path error), from `JavaRuntimeGuardsTests.swift` via `fixtures/java-runtime-guards/`. Port `JavaRuntimeManager.detectInstalledJavaRuntimes` and its path-normalization helper against P3.4's `FileSystem` trait, using `FakeFileSystem` to inject each fixture's `fsTree` input exactly as recorded — no real filesystem access in the test suite. Ported the full `detectInstalledJavaRuntimes` walk for fidelity (Homebrew `Cellar`/`opt` candidate filtering and the extra `Cellar`-version level), not just enough to satisfy the 3 fixtures. Two things deliberately not ported, since no fixture exercises either and both need capabilities this trait doesn't have: `detectJavaMajor` (spawning `java -version`) — the fixtures' own notes say majorVersion must come from path-text inference alone, "never from executing the binary" — and `defaultJavaRuntimeSearchRoots` (real home-directory/OS-specific paths), left for whichever later phase wires this into Settings. Two small, necessary extensions to P3.4's `FakeFileSystem` (`crates/msc-infrastructure/src/fs.rs`), surfaced by this step's own fixtures: (a) `list()` only ever matched files whose *immediate* parent equalled the queried path, which broke walking a real tree (e.g. discovering `temurin-21.jdk` as a child of a search root from a file two levels further down) — generalized to synthesize one-level-down subdirectory entries the way `std::fs::read_dir` actually behaves, verified against the existing `fs.rs` tests plus this step's own. (b) added `with_dir`, an explicit empty-directory marker: the `normalization-directory-without-bin-java-returns-error` fixture's own `fsTree` is `{}`, but its notes describe a real, freshly-created *empty* `<TMP>` directory — something the fsTree schema's `"file"`/`"symlink"` types have no way to spell, so it's seeded directly in the test rather than by changing frozen fixture JSON. One more accommodation, confined to the test file: the `<TMP>` placeholder token itself (not `<TMP>/anything`) contains no `/`, which would incorrectly trip `normalized_java_executable_path`'s bare-command fast path — resolved to a fixed absolute-looking string (`/private/tmp/msc2-fixture-java-runtime`) uniformly across every fixture field before use, restoring the real-temp-directory semantics MSC 1's own test relied on.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/java-runtime-guards --expect 15` → `ok 15` (the directory's existing total: P1.5's 7 pure fixtures plus these 8); then `cargo nextest run -p msc-infrastructure java_runtime_detection` → `8 tests run: 8 passed`
**Commit:** `P3.18: port the java-runtime-guards filesystem leftover`
**Batch:** safe

---

### Windows validation

### P3.19 — Windows-specific substrate validation
**Status:** DONE
**Files:** `crates/msc-infrastructure/tests/windows_substrate.rs` (`#[cfg(windows)]`-gated)
**What:** Per D-017 ("Windows CI for the agent... begins with the filesystem/config/security substrate") and §8's named hazards: "path separators and length limits, file-locking semantics — Windows will not let you delete an open file, service lifecycle and logout behavior, case-insensitive path comparison." CI has built and tested this crate on Windows since P3.4, but nothing yet asserts a Windows-*specific* behavior; this step adds the first ones, gated so they only run on the Windows CI runner: (1) P3.5's path safety against backslash-separated and long (>260 character, exercising extended-length-path handling) inputs, (2) P3.6's atomic write against a destination another handle already has open — Windows refuses the rename the POSIX-only fixtures never exercised; assert the primitive surfaces a clear error rather than hanging or silently succeeding, (3) P3.5 again with two candidate paths differing only in case — Windows' case-insensitive-but-case-preserving filesystem must not treat them as an escape from the root. **Verification status, same shape as P3.10/P3.11:** cannot run the `#[cfg(windows)]`-gated tests natively on this (macOS) development machine. Instead: (1) `cargo check --workspace --target x86_64-pc-windows-msvc --all-targets` — full type-check against real Windows APIs (`std::fs::File`, `OpenOptions`), passes; (2) `cargo fmt --all -- --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo nextest run --workspace` on macOS natively — all pass, 286 tests, confirming the `#![cfg(windows)]` gate makes the file compile to zero tests elsewhere (verified directly: `cargo test -p msc-infrastructure --test windows_substrate` → `0 passed; 0 failed`); (3) the real pass/fail happens on the Windows CI leg, same authority P3.10/P3.11's own Verify lines already name. **Finding, confirmed empirically before writing test 3, not guessed:** case (3)'s literal requirement — a case-differing spelling of the root must not be treated as an escape — does **not** hold against `path_safety.rs` as it stands today. `safe_path`'s `candidate_resolved.starts_with(&root_resolved)` check walks `std::path::Path` components, and Rust's `Path`/`Component` equality is byte-exact on every platform, including Windows — there is no OS-aware case folding anywhere in `std::path`. Reproduced directly (not on the Windows target — this is pure Rust-logic behavior, unaffected by which OS the code runs on): `safe_path(&fs, &PathBuf::from(r"C:\Users\steve\MSC2Test\ServerRoot"), Some(r"..\SERVERROOT\file.txt"), &home)` returns `Err(Escape { .. })` today. So `path_safety_case_difference_is_not_an_escape`, as written to match this step's own stated requirement, is very likely to fail when it actually runs on Windows CI — not a flaky test, a real gap in `path_safety.rs` (outside this step's own `Files:` list, so not touched here). Written to assert the *correct* behavior rather than the current one, per CLAUDE.md's "don't build around it quietly" rule — flagged to Cameron as a question rather than silently patched or silently downgraded to match today's bug.
**Verify:** (Windows CI runner) `cargo nextest run -p msc-infrastructure windows_substrate` → `3 tests run: 3 passed`; absent from macOS/Linux runner output, per the `cfg(windows)` gate
**Commit:** `P3.19: add Windows-specific substrate validation`
**Batch:** solo

### P3.19a — Fix `path_safety.rs`'s Windows case-sensitivity gap found by P3.19
**Status:** DONE
**Files:** `crates/msc-infrastructure/src/path_safety.rs`
**What:** P3.19's own third test found that `safe_path`'s escape check compares `std::path::Path` components byte-exactly on every platform, including Windows — there's no OS-aware case folding anywhere in `std::path` — so a request that differs from the root only in case (e.g. `..\SERVERROOT\file.txt` against a root spelled `...\ServerRoot`) was wrongly classified as an escape, even though Windows' real filesystem is case-insensitive-but-case-preserving and the two spellings name the same directory. **Cameron Temple confirmed: fix now, case-fold on Windows only, leave Unix exactly as it was** (Unix filesystems are generally case-sensitive, and where one isn't — an APFS volume in case-insensitive mode — MSC 1 never accounted for that either, so not a regression to fix here). Replaced the two byte-exact comparisons the escape/forbidden-root checks used (`PathBuf::==` and `Path::starts_with`) with `path_has_prefix`/`paths_equal`, both walking components by hand through a new `components_match(a, b)` — identical to `==` on Unix (`cfg!(windows)` compiles to `false`, dead-code-eliminated), but on Windows folds `Normal` and `Prefix` components (so a bare drive letter, `C:` vs `c:`, is covered the same way a directory name is) via `str::eq_ignore_ascii_case`. Deliberately ASCII-only rather than a full Unicode case fold: strictly more conservative, since it can only make two components compare as *different* that Windows would treat as the same, never the reverse — it can under-fix a small class of non-ASCII-cased same-directory spellings, but it can't turn a real escape into a false negative, so there's no safety regression, only a documented v1 scope limit. Still component-based, not a raw-string comparison, so the classic near-miss P3.5's own fixture #7 pins down (`server1x` vs. `Server1` as a *prefix*, not a genuine parent/child relationship) is unaffected — confirmed directly, `"server1x".eq_ignore_ascii_case("Server1")` is `false`, since whole components are compared, not substrings. This is exactly the fix P3.19's own third test (`path_safety_case_difference_is_not_an_escape`) was written to require — no test file changes needed here, that assertion already expects the corrected behavior; this step makes it true.
**Verify:** `cargo nextest run -p msc-infrastructure path_safety` → `7 tests run: 7 passed` (all seven P3.5 fixtures still pass on this, i.e. any, platform — `cfg!(windows)` is `false` here, so Unix comparison behavior is provably unchanged); `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings` → clean (this is what actually exercises the new Windows-only branch's types); real pass/fail of P3.19's third test itself happens on the Windows CI leg, same as P3.19's own Verify.
**Commit:** `P3.19a: fix path_safety.rs's Windows case-sensitivity gap`
**Batch:** solo

---

### Phase exit

### P3.20 — Phase 3 exit gate check
**Status:** DONE
**Files:** none (verification only)
**What:** Run every Phase 3 deliverable together on all three CI platforms: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo nextest run --workspace` — covering `msc-domain`'s new `network_safety` module, `msc-infrastructure`'s path-safety/atomic-write/config-lifecycle/audit-log/download-staging/operation-journal/operation-exclusivity/java-runtime-detection modules, and each platform crate's `SecretStore` implementation on its own native runner. Confirm `msc-domain` still carries no I/O dependency (unchanged rule from P1.15/P2.21). Confirm P3.19's Windows-specific tests actually ran on the Windows leg of CI rather than being silently absent everywhere. This checks the port plan's own Phase 3 exit criterion verbatim: "substrate fixtures pass on macOS, Linux, and Windows."

**Result: the gate does not currently hold.** Locally on macOS: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo nextest run --workspace` are all clean — `286 tests run: 286 passed, 0 skipped`. `msc-domain/Cargo.toml` still carries only `regex` as a real dependency (`serde`/`serde_json` are dev-only), so the no-I/O rule holds. But CI's Windows leg is red on the latest run ([30701383726](https://github.com/ctemple9/msc2/actions/runs/30701383726), the push containing P3.19+P3.19a) — `Build`, `Format check`, and `Clippy` all pass on Windows, but `Test` fails: `audit_log_file_at_retention_boundary_is_kept` panics on `assertion left == right`, comparing `["/srv/app/audit\audit-2023-10-15.jsonl"]` (actual) against `["/srv/app/audit/audit-2023-10-15.jsonl"]` (expected) — a bare separator mismatch, not a real data bug. **Root cause, traced to P3.18's own change, not this step's:** `FakeFileSystem::list` (`crates/msc-infrastructure/src/fs.rs:266-277`) used to return each file's original stored `PathBuf` verbatim (an exact-parent-match lookup); P3.18 generalized it to `children.insert(path.join(first))` so it could walk multi-level trees. `Path::join`/`PathBuf::push` insert `std::path::MAIN_SEPARATOR` when joining two components that aren't already separator-terminated — `\` on Windows. Every fixture path in this codebase is written with forward slashes (`/srv/app/audit/...`), so `list()`'s newly-joined result now carries a Windows backslash where the fixture's expected string has a forward slash. `PathBuf`'s own `Eq`/`Hash` are component-based and unaffected (Windows path parsing treats `/` and `\` as equivalent separators, so `FakeFileSystem`'s internal `HashMap<PathBuf, _>` lookups still work correctly) — the break is specifically in `audit_log.rs`'s test (`tests/audit_log.rs:215`, `.to_string_lossy().into_owned()` then a literal `Vec<String>` equality against the fixture's raw JSON strings), which bypasses `Path`'s OS-aware comparison and compares bytes. `java_runtime_detection.rs` and `operation_journal.rs` also call `fs.list()` but neither does a raw-string equality check on the returned paths, so this specific failure mode is confined to `audit_log.rs`'s test as written — not otherwise confirmed clean, since the Windows run never got far enough to prove it.

**Compounding finding, the more serious one:** `cargo nextest` fails fast by default. Once `audit_log_file_at_retention_boundary_is_kept` fails, nextest cancels the run — `242/289 tests run`, `47 not run due to test failure`. Grepping the full Windows log for `windows_substrate`, `path_safety`, or `java_runtime_detection` returns **nothing** — P3.19's own three Windows-specific tests (the entire point of Windows CI validation under D-017) never executed on this run, nor did P3.5's `path_safety` suite or P3.18's `java_runtime_detection` suite. This is exactly the failure mode this step's own "What" flagged as a thing to confirm rather than assume — and confirming it is what found the problem. **The Phase 3 exit criterion — "substrate fixtures pass on macOS, Linux, and Windows" — is not met today**, and separately, P3.19's Windows-specific tests are still functionally unverified: they've never been observed to actually run to completion on the Windows CI leg.

Not fixed here — `fs.rs` is outside this step's own `Files:` list, and the regression traces to P3.18, a step Cameron hasn't verified yet. Flagged as a finding for a corrective step (the same `P3.19a`-after-`P3.19` pattern) rather than silently patched, per `CLAUDE.md`'s "don't build around it quietly."
**Verify:** `cd ~/msc2 && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo nextest run --workspace` → all green on macOS (confirmed); `gh run view 30701383726` → Windows `Test` step fails, `242/289` tests run, `audit_log_file_at_retention_boundary_is_kept` FAILED — gate does not currently hold
**Commit:** `P3.20: run the Phase 3 exit gate check — Windows leg red, root cause identified`
**Batch:** stop-after

### P3.20a — Fix `FakeFileSystem::list`'s Windows separator regression found by P3.20
**Status:** DONE
**Files:** `crates/msc-infrastructure/src/fs.rs`, `crates/msc-infrastructure/tests/fs.rs`
**What:** P3.20's own exit gate check found that P3.18's generalization of `FakeFileSystem::list` (`children.insert(path.join(first))`) builds each returned path with `Path::join`/`PathBuf::push`, which insert `std::path::MAIN_SEPARATOR` — a backslash on Windows — even though every fixture path in this codebase is written with forward slashes. `PathBuf`'s own `Eq`/`Hash` don't care (component-based, and Windows path parsing treats `/` and `\` as equivalent separators), but `audit_log.rs`'s test compares `list()`'s output as a raw string against a fixture's literal forward-slash expectation, which does. **Cameron Temple confirmed: fix `FakeFileSystem`, not the test** — the fake filesystem exists specifically to behave identically on every host OS, and every fixture author so far has assumed forward slashes, so guaranteeing that at the source keeps the assumption true for every future caller, not just `audit_log.rs`'s. Added `join_forward_slash(base: &Path, component: &OsStr) -> PathBuf`, a small helper that concatenates with a literal `/` regardless of host OS, and used it in place of `path.join(first)`. `StdFileSystem::list` (the real-filesystem implementation, used only by `StdFileSystem`'s own tests) is untouched — real files legitimately use the host's real separator, this fix is scoped to the fake, fixture-facing implementation only. New test `fake_file_system_list_returns_forward_slash_paths` in `tests/fs.rs` reproduces the exact case P3.20 found (a nested `/srv/app/audit/...` path) and asserts the literal string form, not just `PathBuf` equality, since `PathBuf` equality is what already silently tolerated the bug. Verified beyond the macOS-native test run (which can't exercise the Windows-only branch, since there isn't one — the fix is OS-independent by construction, no `cfg` gating): `cargo check --workspace --target x86_64-pc-windows-msvc --all-targets` passes, confirming the new code type-checks against the Windows target; the real proof is the next Windows CI run actually completing `audit_log`'s suite and reaching P3.19's `windows_substrate` tests, which this step's own Verify line names as the authority, same shape as P3.10/P3.11/P3.19's own verification notes.
**Verify:** `cargo nextest run -p msc-infrastructure --test fs` → `5 tests run: 5 passed` (the existing 4 plus this step's new one); `cargo nextest run --workspace` → `287 tests run: 287 passed, 0 skipped`; then (Windows CI runner, next push) `gh run list --limit 1` → `success`, with the log showing `audit_log`'s full suite passing and `windows_substrate`'s 3 tests actually appearing and passing — closing both findings P3.20 raised

**Confirmed on real Windows CI, not just cross-compiled:** [run 30702485721](https://github.com/ctemple9/msc2/actions/runs/30702485721) — the fix worked. `audit_log`'s suite is fully green, and the run advanced from `242/289` (cancelled at the old bug) to `286/290`. For the first time, P3.19's own `windows_substrate` tests actually executed on Windows: `path_safety_backslash_and_long_paths` and `path_safety_case_difference_is_not_an_escape` both passed — the latter is the first real-Windows proof that P3.19a's case-folding fix actually works, not just that it type-checks. Both findings P3.20 raised (the separator bug itself, and "are P3.19's tests silently absent") are closed. Advancing further surfaced a third, different failure — `atomic_write_destination_locked_by_open_handle` — outside this step's own scope; recorded as P3.20b below rather than folded in here.
**Commit:** `P3.20a: fix FakeFileSystem::list's Windows separator regression`
**Batch:** solo

### P3.20b — Fix the Windows locked-file test's premise, found running P3.20a's own CI proof
**Status:** DONE
**Files:** `crates/msc-infrastructure/tests/windows_substrate.rs`
**What:** With P3.20a's fix unblocking the Windows run far enough for P3.19's third test to finally execute, `atomic_write_destination_locked_by_open_handle` failed for real: `expected a clear Io error while the destination is locked, got Ok(())`. The test's own doc comment assumed `std::fs::File::open` on Windows requests `FILE_SHARE_READ | FILE_SHARE_WRITE` but not `FILE_SHARE_DELETE`, so a rename over it should be refused — wrong. Rust's std has included `FILE_SHARE_DELETE` in its default Windows share mode for years specifically so ordinary Rust-to-Rust renames succeed against another handle held open, closer to POSIX rename semantics. That handle never blocked anything; `atomic_write` (P3.6, already DONE) correctly performed the rename, and the test's premise — not the primitive — was wrong. **Cameron Temple confirmed: option A** — fix the test to actually reproduce the hazard D-017/§8 name ("Windows will not let you delete an open file"), which is about an *uncooperative* locker (antivirus, a non-Rust process, anything that doesn't opt into delete-sharing), not Rust's own cooperative default. Opened the held handle via `std::fs::OpenOptions::new().read(true).share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE).open(&dest)` (`std::os::windows::fs::OpenOptionsExt`), explicitly omitting `FILE_SHARE_DELETE` — the real scenario the original comment described but the code never actually produced. No change to `atomic_write.rs` itself: it already does nothing but call `fs.rename` and map any error to `AtomicWriteError::Io`, so if the corrected lock genuinely blocks the rename, the primitive should already surface it correctly — this step's Verify is what confirms that's actually true on real Windows, not assumed. Test-only change, confined to `windows_substrate.rs`; `#![cfg(windows)]` keeps it compiling to zero tests on macOS/Linux, confirmed directly (`cargo test -p msc-infrastructure --test windows_substrate` → `0 passed; 0 failed`).
**Verify:** `cargo check --workspace --target x86_64-pc-windows-msvc --all-targets` → passes (confirmed); `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings` → clean (confirmed); `cargo fmt --check` / `cargo clippy --workspace --all-targets -- -D warnings` / `cargo nextest run --workspace` on macOS → all clean, `287 tests run: 287 passed, 0 skipped` (confirmed); real pass/fail happens on the next Windows CI run — `gh run list --limit 1` → `success`, with `atomic_write_destination_locked_by_open_handle` now passing (or, if it still fails, a real gap in `atomic_write.rs` itself, not the test)

**Confirmed on real Windows CI:** [run 30710938285](https://github.com/ctemple9/msc2/actions/runs/30710938285) — Windows leg fully green, `36 binaries`, all tests reached. All three of P3.19's own tests pass for real: `path_safety_backslash_and_long_paths`, `atomic_write_destination_locked_by_open_handle` (this step's own fix), and `path_safety_case_difference_is_not_an_escape`. `atomic_write.rs` needed no code change — the corrected, genuinely non-delete-shared lock now blocks the rename exactly as `fs.rename`'s existing error mapping already expected. Every finding P3.20/P3.20a/P3.20b raised about Windows is now closed. **Unrelated finding from this same run, not fixed here:** the macOS leg failed once, on a test neither this step nor P3.20a touches — `audit_log_entries_from_concurrent_writers_preserve_call_order` (P3.13, real-thread concurrency test) — `assertion left == right failed: all three entries present, none interleaved`. Re-ran it 15/15 locally in isolation with no failure, consistent with CI-runner scheduling contention under the full parallel `--workspace` run rather than a real regression; not touched by any commit in this session. Flagged for Cameron rather than silently ignored or "fixed" by loosening the assertion.
**Commit:** `P3.20b: fix the Windows locked-file test's premise`
**Batch:** solo

### P3.21 — Fix DPAPI-scope and Linux-secret-store documentation drift found by Codex's Phase 3 review
**Status:** awaiting verification
**Files:** `docs/msc2/msc2-engineering.md`, `docs/msc2/substrate/secret-storage.md`, `docs/msc2/substrate/service-identity.md`
**What:** Codex's Phase 3 review (below) found two accuracy gaps in the controlled document set, both closed here. First, three passages (`secret-storage.md` §2, §7, §10; `service-identity.md` §3) called Windows DPAPI a "machine-scoped" secret — the same category as the Linux `systemd-creds` host-key fallback and the macOS System keychain. Wrong, per this project's own later, more careful finding: `secret-storage.md` §13 (P3.12's cross-platform comparison) established that Windows Credential Manager wraps DPAPI's *per-user* mode, tied to the installing account, not the whole machine. Corrected each passage to say so and point at §13 as the authority, rather than silently deleting the earlier wrong claim. Second, `msc2-engineering.md` §8 still read as though `systemd-creds` were the shipped Linux implementation; it's the real target design, deferred to Phase 4 — the actual v1 backend, found by P3.11, is the file-based `LinuxSecretStore` owned by the installing user, not root. Added the P3.11 finding and a pointer to `secret-storage.md` §12/§13 so the engineering doc matches what's actually running.
**Verify:** `grep -rn "DPAPI machine-scope answer\|Windows DPAPI and the macOS System-keychain\|Windows DPAPI machine-scope answer\|DPAPI.s machine scope and \`systemd-creds\`\|Windows DPAPI answer and the macOS System-keychain" docs/msc2/msc2-engineering.md docs/msc2/substrate/secret-storage.md docs/msc2/substrate/service-identity.md` → no matches (the five specific wrong phrasings Codex's review flagged are gone); `grep -n "P3.11 later found" docs/msc2/msc2-engineering.md` → one match
**Commit:** `P3.21: fix DPAPI-scope and Linux secret-store documentation drift Codex's review found`
**Batch:** solo

---

## Amendments log

When a review amends an earlier phase or a decision, record it here so the change isn't silent.

### 2026-08-01 — Codex Phase 3 review: gate holds, with documentation drift to clean up

Codex reviewed Phase 3 as a gate check, not a step-compliance check, and did not
implement this phase. The Phase 3 gate in `msc2-port-plan.md` is: "Approved
server roots and path safety · atomic writes · versioned configuration with
migrations · `SecretStore` trait · audit log · download staging with checksum
verification · operation journal · operation exclusivity." Windows CI also
begins here, covering path separators and length limits, file-locking semantics,
service lifecycle, and case-insensitive path comparison. The exit criterion is:
"substrate fixtures pass on macOS, Linux, and Windows." The gate holds as of
the latest CI run checked during review.

Evidence checked: local `cargo fmt --check` and `cargo clippy --workspace
--all-targets -- -D warnings` both passed; local `cargo nextest run --workspace`
reported `287 tests run: 287 passed`; a focused substrate run covered path
safety, atomic writes, config lifecycle, audit log, download staging, operation
journal, operation exclusivity, network safety, Java runtime detection, and
filesystem fake behavior with `44 tests run: 44 passed` (one nextest leak
warning on `audit_log_corrupt_or_partial_line_does_not_crash_writer`); fixture
directories validated for the fixture-backed substrate domains
(`path-safety` 7, `atomic-write` 4, `config-lifecycle` 4,
`secret-store-contract` 5, `audit-log` 5, `download-staging` 4,
`network-safety` 13, `java-runtime-detection` 8). GitHub Actions run
`30711083073` was green across `macos-latest`, `ubuntu-latest`, and
`windows-latest`; the Windows log explicitly shows `windows_substrate` tests
`path_safety_backslash_and_long_paths`,
`path_safety_case_difference_is_not_an_escape`, and
`atomic_write_destination_locked_by_open_handle` all passing, plus the five
Windows `secret_store_contract_*` tests. The Ubuntu log shows the five Linux
`secret_store_contract_*` tests and `key_file_and_secrets_dir_are_owner_only`
passing. The local macOS run shows the five macOS Keychain
`secret_store_contract_*` tests passing.

No code/product drift from the vision was found: the phase stayed inside the
agent-owned safety substrate, did not add real service registration before
Phase 4, did not wire real pairing ahead of its confirmed Phase 4 placement,
kept user-file mutation behind path-safety/atomic-write/config primitives, and
started Windows validation where D-017 requires it. There is documentation drift
inside the controlled set that should be amended before Phase 4: after P3.11,
the actual Linux v1 backend is the explicitly temporary file-based
`LinuxSecretStore`, with the `systemd-creds` privileged-helper design deferred
to Phase 4; however `msc2-engineering.md` §8 still reads as though
`systemd-creds` is the current Linux implementation. Also, older summary text in
`docs/msc2/substrate/secret-storage.md` §2/§10 and
`docs/msc2/substrate/service-identity.md` §3 still compares macOS/Linux to a
"Windows DPAPI machine-scope" answer, while D-025, P3.10, and the P3.12
comparison table correctly say Windows Credential Manager uses DPAPI
user-scope for the installing user's account.

Later phases should audit six items. First, Phase 4 must replace the Linux
file-based stand-in with the privileged `systemd-creds` helper, or explicitly
reconfirm the weaker v1 backend before it becomes permanent. Second, Phase 4
must test the still-open macOS LaunchDaemon questions: login-vs-System-keychain
reachability and TCC behavior when a daemon touches user-controlled paths. Third,
Phase 4 should wire `SecretStore` into real pairing and retire the Phase 2 dev
token, including rate limiting and audit attribution. Fourth, D-024 power
management must land with real service lifecycle, as confirmed in
`phase3-scope.md`. Fifth, the D-021 headless-package GUI-link CI check is still
unassigned in the port plan and needs a home. Sixth, keep an eye on the audit
log concurrency/leak signal: the latest CI is green, but P3.20b recorded one
macOS CI failure of `audit_log_entries_from_concurrent_writers_preserve_call_order`,
and this review's focused local run produced one nextest leak warning in the
audit-log suite.

**Amended 2026-08-01, same day (P3.21):** the documentation-drift paragraph
above is closed. `msc2-engineering.md` §8, `secret-storage.md` §2/§7/§10, and
`service-identity.md` §3 no longer call Windows DPAPI machine-scoped, and
`msc2-engineering.md` §8 now states the actual v1 Linux backend (file-based
`LinuxSecretStore`) alongside the still-deferred `systemd-creds` helper design.
The six items for later phases to audit are unaffected and still stand.

No earlier phase needs amending. The Phase 0/1/2 amendments already recorded
still stand. The needed amendments are Phase 3/control-document cleanup, not a
change to an earlier phase gate.

### 2026-08-01 — Codex Phase 2 review: gate holds, with scoped stub caveats

Codex reviewed Phase 2 as a gate check, not a step-compliance check, and did not
implement this phase. The Phase 2 gate in `msc2-port-plan.md` is: "Versioned
HTTP and WebSocket contract generated from the schema. Operation IDs, progress,
structured errors, capability advertisement, cancellation. A skeletal agent
whose routes can be exercised without real mutation." Its exit criterion is:
"the existing iOS app connects and reads status against a stub agent." The gate
holds under the Phase 2 scope recorded in this plan: the v1 contract is present,
the skeletal agent exercises the status/health/capabilities/operation routes and
the two Phase 2 WebSocket channels, and the copied iOS app has been shown to
read live status from that stub.

Evidence checked: `cargo fmt --check` passed; `cargo clippy --workspace
--all-targets -- -D warnings` passed; `cargo nextest run --workspace` passed
with `215 tests run: 215 passed` and one nextest-reported leaky test; `python3
tools/api-contract-check.py --v1-summary` reported 93 routes, all under `/v1/`,
with zero missing permission categories, zero non-`ErrorDTO` non-2xx responses,
and zero missing `helpId` fields; `xcodebuild -project
clients/ios/MSCRemoteiOS.xcodeproj -scheme MSCRemoteiOS build` succeeded; a live
`msc-agent` with `MSC_DEV_TOKEN=msc2-dev-token` returned `200` for `/v1/health`,
`401` for unauthenticated `/v1/status`, and authenticated `/v1/status` returned
the canned running server JSON; `tools/contract-conformance-check.py` against the
live agent returned `ok 6`; an ad-hoc WebSocket check observed console backfill
plus a live demo line and operation-progress frames ending in `succeeded`. The
manual iOS gate evidence is the P2.20 note above: Cameron reached **STATUS:
RUNNING** from live `/v1/status` in the simulator after typing the Phase 2 dev
token manually.

No product or architecture drift from the vision was found. Phase 2 keeps the
agent as the single owner of server-management behavior, keeps the iOS app as
the retained client, keeps management loopback-only for the scoped dev agent,
adds `helpId` to the v1 contract before later client work, and avoids real
filesystem/process mutation before the Phase 3 substrate. Two scope caveats are
not gate failures, but must stay visible: the generic OpenAPI-to-Rust/Swift
codegen pipeline is deliberately deferred and replaced for now by conformance
checks, and the D-012 desktop/browser auth gaps remain open rather than closed
by the fixed dev token.

Later phases should audit five items. First, replace the fixed dev token with
the real Phase 3 `SecretStore`/pairing path, including rate limiting, audit
logging, and the fresh-install Keychain empty-string bug that made P2.20 need a
manual token workaround. Second, decide when generated Rust/Swift types become
mandatory instead of hand-written DTO mirrors; do not let the Phase 2
conformance-check substitute become the permanent contract discipline. Third,
add scripted WebSocket conformance checks for console and operation progress,
since P2.17 covers only ordinary JSON HTTP routes. Fourth, audit the one
nextest-reported leaky test before it becomes CI noise or hides a real resource
leak. Fifth, when the real iOS status/dashboard work expands beyond the gate,
re-check the temporary `serverType: "paper"` mapping through the old iOS
`java|bedrock` enum.

No earlier phase needs amending. The Phase 0/1 amendments already recorded still
stand. The current rolling-plan status line and P2.12's status are stale
relative to the completed Phase 2 history, but that is current-plan
housekeeping, not an earlier-phase gate amendment.

### 2026-07-31 — Codex Phase 1 review: gate holds; tighten gate wording

Codex reviewed Phase 1 as a gate check, not a step-compliance check, and did not
implement this phase. The Phase 1 gate in `msc2-port-plan.md` is: "Rust passes
the Phase 0 pure fixtures. No user files touched." The gate holds under the
Phase 1-scoped interpretation already written in this plan: Phase 1 intentionally
ports only domain types and pure rules, while API, network-safety, config,
provisioning, and modpack fixtures remain assigned to later phases.

Evidence checked: `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo nextest run -p msc-domain` passed, with `190 tests run: 190 passed`; all Phase 1 fixture directories validated with their expected counts; `msc-domain` has only `regex` as a runtime dependency; a source scan found no filesystem, process, or network APIs in `crates/msc-domain/src`; and the Phase 1 changed-file set is limited to repo code, fixtures, docs, and CI, not user/server data paths.

No product or architecture drift from the vision was found. Phase 1 stayed aligned
with the Rust domain boundary, the no-I/O rule, MSC 1-derived fixtures, and
D-026's requirement that the router matcher, fallback resolver, composer,
troubleshooting engine, and runtime resolver are executable behavior.

No earlier phase needs amending. One wording amendment is recommended before the
next review: change the port-plan Phase 1 exit criterion from "Rust passes the
Phase 0 pure fixtures" to "Rust passes the Phase 1-scoped Phase 0 pure fixtures,
plus Phase 1 characterization fixtures." This removes an ambiguity without
changing the actual phase boundary.

Later phases should audit the deferred fixture domains where the ledger and
rolling plan already place them: Phase 2 API contract and `helpId`; Phase 3
network-safety and Java runtime filesystem discovery/normalization; Phase 5/6
worlds and backups rollback/verification; Phase 7 args-file and headless launch
behavior; Phase 8 modpack, client-only, pinning, pack-managed, and D-027
CurseForge manual-download behavior; and the full router guide/content migration
against the Phase 1 engines.

### 2026-07-31 — P0.32 fixed the typed-failure schemas, with one known simplification

P0.32 corrected 68 (path, method, status) entries across 27 mutation routes from the generic `Error` schema to the route's actual typed result DTO, verified per-route against source (see P0.32's own entry above). One simplification, made deliberately rather than discovered late: several routes' `400` response is genuinely *mixed* — some 400 causes are synchronous pre-provider guards (`missing_body`, a required-field check before the `Task` block) that really do send `{"error": ...}`, while other 400 causes on the *same route* come from the provider's result after the guard passes, and get the typed DTO instead. OpenAPI has no clean way to say "this status code is sometimes schema A, sometimes schema B" without `oneOf` on every affected status, which would blow up the file's readability for a distinction that mostly doesn't change what a client needs to do (parse "did this fail," which both shapes support).

Given that, `400` was left as `Error` uniformly across all 27 routes as the documented default, *except* `/servers/import` and `/templates`, where reading the handlers found no pre-provider field guards at all beyond `missing_body`/`invalid_json` — every other 400 cause is post-provider, so those two routes' `400` got the typed DTO (with the exception noted in the response `description`). Every other route's `400` may still occasionally be a typed DTO in practice for specific failure messages (e.g. `/servers/create`'s `invalid_server_type` comes from the provider, not a pre-Task guard; `/users` and `/users/update`'s `label_empty` likewise) — recorded as a known, deliberate gap, not a new blind spot. The authoritative source for a future full pass is `RemoteAPIServer+ComponentRoutes.swift`'s `serverMutationStatus`/`templateMutationStatus`/`importStatus`/`playerMutationStatus`/`worldMutationStatus` helper functions (and `+UserRoutes.swift`'s inline switches) and their call sites — each one shows exactly which message strings are checked before the `Task` block (Error) versus inside it (typed).

### 2026-07-31 — P0 API baseline response schemas need a typed-failure pass

A Phase 0 cross-check against MSC 1 found that the route inventory is complete, but some
non-2xx response schemas are too generic. MSC 1 often uses HTTP status codes to indicate
provider-level failure while still returning the route's typed result DTO (`success`,
`message`, and route-specific payload fields), not the generic `{"error": ...}` shape.
The OpenAPI baseline already captures this correctly for some routes (`/files/read`,
`/files`, `/components/client-export`, `/playit/start`, `/broadcast/download-jar`,
`/duckdns` 500), but other mutation families still list `Error` for typed failures.

Affected families include server mutations, templates/import, world mutations,
component/add-on/version mutations, players mutations, allowlist mutation, users,
`/config/ram`, `/config/geyser`, `/health/repair`, and resource-pack mutations. The
status codes themselves generally match MSC 1; the disagreement is the response body
schema for provider-level non-2xx results. Later API-baseline work should distinguish
input/parse validation errors (`{"error": ...}`) from provider result failures (the
route's typed result DTO), using MSC 1's `sendJSON(..., encodable: result, ...)` call
sites as the oracle.

### 2026-07-30 — P0.30 corrects five family route counts, and P0.23q's players count

`tools/api-baseline-check.py`'s `KNOWN_COUNTS` asserted a fixed sub-route count per family, taken from `msc2-engineering.md` §5's route-family list at the time each P0.23 step ran. That list turned out to omit real MSC 1 routes (see P0.30 above). Adding them changes what several already-passing family checks legitimately find, because the new routes share a path prefix with an existing family:

| Family | Was | Now | Why |
|---|---:|---:|---|
| `servers` | 5 | 6 | adds bare `GET /servers` |
| `worlds` | 5 | 6 | adds bare `GET /worlds` |
| `backups` | 3 | 4 | adds bare `GET /backups` |
| `components` | 4 | 6 | adds `GET /components` and `GET /components/client-export` |
| `health` | 1 | 3 | adds `GET /health` and `GET /health/problems` alongside the existing `POST /health/repair` |

None of P0.23a/c/d/e/h's original verify runs were wrong when they ran — the routes that change their count didn't exist in `openapi.json` yet. Re-running any of those five family Verify lines today correctly shows the new count, not the one recorded at the time. `players` (P0.23q) also gains a sibling — `ok 4` → `ok 5` — but stays non-breaking since wildcard families only assert count > 0.

### 2026-07-30 — P0.24 amended: one WebSocket channel, not six

P0.24 originally asked to document "six WS channels (console, status, operation progress, players, notifications, metrics)." Reading `RemoteAPIServer+WebSocket.swift` and every call site of its JSON-send function (`RemoteAPIServer.swift`, `RemoteAPIServer+HTTP.swift`) shows MSC 1 implements exactly one: console-line streaming over `/console/stream`. There is no "channel" concept anywhere in the source (zero matches for `channel`/`Channel`); status, players, notifications, and metrics are all HTTP-polled, never pushed. The "six channels" language traces to `msc2-engineering.md` §5's description of MSC 2's *intended* design, not MSC 1's actual baseline. Per D-006 — the api-baseline captures MSC 1 as it is; extensions are designed in Phase 2, not invented here — the step is amended to document the one real channel. `Verify` changed from expecting `6` channels to `1`.
