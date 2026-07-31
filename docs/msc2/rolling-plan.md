# MSC 2 — Rolling Plan

> ## STATUS: Phase 1 in progress — P1.3 done, awaiting verification
> **Next move:** VERIFY (Cameron runs P1.3's Verify command, then EXECUTE continues with P1.4)
> **Repo:** https://github.com/ctemple9/msc2 · CI green on macOS, Linux, Windows
> **Last updated:** 2026-07-30

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
| 1 | Domain types and pure rules | **planned** |
| 2 | API contract and operation model | not started |
| 3 | Safety substrate | not started |
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
**Status:** awaiting verification
**Files:** `crates/msc-domain/src/version.rs`, `crates/msc-domain/tests/version_comparison.rs`
**What:** Port `ComponentVersion` parsing and comparison (`ComponentVersionParsingTests.swift` origin, `fixtures/component-version/`) — the primitive MSC 2 needs everywhere a Paper/Purpur build number, Minecraft version string, or loader version gets compared, including the ordering behavior the downgrade guards several agent workflows depend on (`MCVersionComparator.isDowngrade`, symbol ledger target_domain `java-runtime`/`components` — those call sites port later; Phase 1 only needs the comparison primitive). Wire all 21 fixtures through the P1.2 harness.
**Verify:** `cargo nextest run -p msc-domain version_comparison` → `21 tests run: 21 passed`
**Commit:** `P1.3: port version comparison`
**Batch:** safe

### P1.4 — Port TPS parsing
**Status:** not started
**Files:** `crates/msc-domain/src/tps.rs`, `crates/msc-domain/tests/tps.rs`
**What:** Port the TPS-sample parser (`TpsMonitoringTests.swift` origin, `fixtures/tps/`) — console-reply-line to TPS-figure conversion, including the negative-sample clamp-to-zero behavior `fixture-format.md`'s own worked example documents. Wire all 27 fixtures.
**Verify:** `cargo nextest run -p msc-domain tps` → `27 tests run: 27 passed`
**Commit:** `P1.4: port TPS parsing`
**Batch:** safe

### P1.5 — Port Java runtime policy (pure subset)
**Status:** not started
**Files:** `crates/msc-domain/src/java_runtime.rs`, `crates/msc-domain/tests/java_runtime_guards.rs`
**What:** Port the pure guard/warning logic from `JavaRuntimeGuardsTests.swift` (`fixtures/java-runtime-guards/`): `requiredJavaMajor`'s Minecraft-version-to-Java-major mapping, and the too-old/too-new compatibility-warning classification. **Scope note, a deliberate call, not a silent skip:** 8 of the 15 fixtures in this domain touch the real filesystem — `detect-installed-java-runtimes-*` (×3) scans a directory tree, `normalization-*` (×5) stats candidate paths — and `msc-domain` carries no I/O per `msc2-engineering.md` §6. Those 8 stay unported here and move to `msc-infrastructure` once Phase 3 builds the filesystem substrate behind a trait. Only the 7 pure fixtures (`no-warning-*` ×3, `too-old-warning-still-fires`, `too-new-warning-*` ×2, `required-java-major-mapping`) are wired in this step. Flagged here for Cameron to overrule if he'd rather stub a filesystem trait early instead of waiting for Phase 3.
**Verify:** `cargo nextest run -p msc-domain java_runtime_guards` → `7 tests run: 7 passed`
**Commit:** `P1.5: port Java runtime policy (pure subset)`
**Batch:** stop-after

### P1.6 — Port property models
**Status:** not started
**Files:** `crates/msc-domain/src/properties.rs`, `crates/msc-domain/src/settings_schema.rs`, `crates/msc-domain/tests/server_properties.rs`, `crates/msc-domain/tests/settings_schema.rs`
**What:** Port `ServerPropertiesModel` (`ServerPropertiesModelTests.swift` origin, `fixtures/server-properties/` — the unknown-key-preserving round trip `msc2-engineering.md` §7 names directly: "silently rewriting `server.properties` with only the recognized keys is destructive") and the settings schema (`ServerSettingsSchemaTests.swift` origin, `fixtures/settings-schema/` — type coercion, range clamping, the level-type wire-token mapping, case-insensitive enums). Two modules, each wired to its own fixture directory.
**Verify:** `cargo nextest run -p msc-domain server_properties` → `7 tests run: 7 passed`; then `cargo nextest run -p msc-domain settings_schema` → `16 tests run: 16 passed`
**Commit:** `P1.6: port property models`
**Batch:** safe

### P1.7 — Port crash analysis and slug normalization
**Status:** not started
**Files:** `crates/msc-domain/src/crash_analysis.rs`, `crates/msc-domain/src/slug.rs`, `crates/msc-domain/tests/connector_crash_analysis.rs`, `crates/msc-domain/tests/startup_crash_analyzer.rs`
**What:** Port `StartupCrashAnalyzer` (`ConnectorCrashAnalysisTests.swift` + `StartupCrashAnalyzerTests.swift` origins — Forge dependency-block parsing, connector entrypoint failure attribution, Fabric/Forge missing- and wrong-dependency-version attribution) and `ModrinthSlugNormalizer` (`canonicalSlug` / `normalizedSlug` / `isKnownAlias`). MSC 1 has no separate test file for the normalizer — it doesn't need new characterization, because 4 of the 11 `connector-crash-analysis` fixtures already exercise it directly (MSC 1's own test file bundles the two together). `searchQuery`, the normalizer's one method with no fixture of its own, is a one-line wrapper (`canonical.isEmpty ? raw : canonical`) — port it as part of `slug.rs` but don't invent a fixture for a wrapper the existing 4 already cover the substance of.
**Verify:** `cargo nextest run -p msc-domain connector_crash_analysis` → `11 tests run: 11 passed`; then `cargo nextest run -p msc-domain startup_crash_analyzer` → `7 tests run: 7 passed`
**Commit:** `P1.7: port crash analysis and slug normalization`
**Batch:** safe

---

### New-characterization domains

Both files below have **no MSC 1 test file** — nothing to extract. Per `fixture-format.md`, `expected` values still may not be invented; they come from reading the source's closed, deterministic logic directly (every case is enumerable by inspection) — the same evidentiary standard `fixture-format.md` calls "MSC 1 run by hand" for untested pure functions. `source.test` in each new fixture should name the property or method being characterized, not a fabricated Swift test name.

### P1.8 — Characterize and port server identity & flavors
**Status:** not started
**Files:** `fixtures/server-identity/`, `crates/msc-domain/src/identity.rs`, `crates/msc-domain/tests/server_identity.rs`
**What:** `ServerType` (`java`/`bedrock`, `AppConfig.swift`) and `JavaServerFlavor` (`JavaServerFlavor.swift`, 246 lines, 9 cases: `paper, purpur, pufferfish, vanilla, fabric, neoforge, spigot, forge, quilt`). Write fixtures covering, per flavor: `category`, `isForgeFamily`, `addOnKind`, `provisioningKind`, `modrinthProjectType`, `modrinthLoaderFacets`, `autoTpsCommand`, `isRecommended`, `isAvailableInCreateFlow` — one case per flavor bundling all nine. Add boundary cases for `tpsPollCommand(minecraftVersion:)` / `supportsVanillaTickQuery` around the 1.20.3 threshold (below, exactly at, above, and nil/empty version), and one case per `JavaServerCategory` for `createFlowChoices`. `displayName`, `shortDescription`, and `iconName` are client-rendering per the port plan's deletion test (§1) and are not ported. This is judgment work with a new fixture domain — cross-check the finished fixture set against `JavaServerFlavor.swift` line by line before wiring the Rust port.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/server-identity --expect <n>` → `ok <n>` (count fixed once the fixtures are written — roughly 9 property-bundle cases + ~5 TPS-threshold cases + 2 `createFlowChoices` cases); then `cargo nextest run -p msc-domain server_identity` → `<n> tests run: <n> passed`
**Commit:** `P1.8: characterize and port server identity & flavors`
**Batch:** solo

### P1.9 — Characterize and port the command catalog
**Status:** not started
**Files:** `fixtures/command-catalog/`, `crates/msc-domain/src/commands.rs`, `crates/msc-domain/tests/command_catalog.rs`
**What:** `MinecraftCommandRegistry.swift` (542 lines, 42 command definitions). Two things to characterize: (1) the static catalog's `commands(for:)` Java/Bedrock filter — assert the exact filtered name list per `ServerType`, not a re-typed copy of all 42 definitions; (2) the autocomplete engine, `suggestions(for:serverType:onlinePlayers:)` — command-name-prefix completion, argument-slot detection (including "input ends with a space starts a new slot"), player-name filtering against a fake online-player list, keyword-option filtering, and the empty-input / unknown-command-name cases that return `[]`.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/command-catalog --expect <n>` → `ok <n>`; then `cargo nextest run -p msc-domain command_catalog` → `<n> tests run: <n> passed`
**Commit:** `P1.9: characterize and port the command catalog`
**Batch:** solo

---

### Router rule engine

Five files, 2,077 lines total, **zero MSC 1 test coverage** for any of them — per `msc2-decisions.md` D-026 point 3, "the matcher, fallback resolver, composer, and troubleshooting decision tree are executable behavior and are translated to Rust" (the runtime resolver is a fifth, separately named in the port plan and already adjudicated agent-owned in the symbol ledger, see P1.14). The guide **catalog and step content** are data, not logic — per D-026 point 1, they migrate to JSON "at any time," on their own schedule, not gated to this phase. P1.10–P1.13 introduce one small, shared, representative sample of guide records — not the real catalog — sufficient to exercise the engines; P1.10 builds it, P1.11–P1.13 reuse it.

### P1.10 — Characterize and port the router matcher
**Status:** not started
**Files:** `fixtures/router-matcher/`, `fixtures/router-sample-catalog.json`, `crates/msc-domain/src/router/matcher.rs`, `crates/msc-domain/tests/router_matcher.rs`
**What:** `RouterPortForwardGuideMatcher.swift` (320 lines) — "scores guide candidates against user input and returns ranked results with confidence metadata... normalizes freeform user input, infers likely router/provider families, ranks candidate guides, and suggests a fallback when there is no exact family guide in the current catalog" (the file's own doc comment). Build `fixtures/router-sample-catalog.json` first — a handful of representative guide records covering at least two router brands and one mesh-system brand, used by this step and reused by P1.11–P1.13. Characterize: exact-name match, fuzzy/partial match, family inference from a misspelled or partial brand name, tie-break ordering between equally-scored candidates, and the no-match fallback path.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/router-matcher --expect <n>` → `ok <n>` (live count, fixed at write time); then `cargo nextest run -p msc-domain router_matcher` → `<n> tests run: <n> passed`
**Commit:** `P1.10: characterize and port the router matcher`
**Batch:** solo

### P1.11 — Characterize and port the router fallback decision tree
**Status:** not started
**Files:** `fixtures/router-fallback-tree/`, `crates/msc-domain/src/router/fallback_tree.rs`, `crates/msc-domain/tests/router_fallback_tree.rs`
**What:** `RouterPortForwardFallbackDecisionTree.swift` (610 lines — the largest of the five) — "models a deterministic decision tree plus a resolver that can route a user toward the best available guide, an honest fallback, or an unknown-router help path," driving "the step-by-step router identification funnel" (the file's own doc comment). Characterize the full `RouterPortForwardDecisionNodeID` state machine directly from source — it's a closed enum-driven tree, not open-ended parsing: every node's transitions, and all three terminal outcomes (best-guide, fallback-guide, unknown-router-help). Reuses P1.10's sample catalog.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/router-fallback-tree --expect <n>` → `ok <n>`; then `cargo nextest run -p msc-domain router_fallback_tree` → `<n> tests run: <n> passed`
**Commit:** `P1.11: characterize and port the router fallback decision tree`
**Batch:** solo

### P1.12 — Characterize and port the router guide composer
**Status:** not started
**Files:** `fixtures/router-composer/`, `crates/msc-domain/src/router/composer.rs`, `crates/msc-domain/tests/router_composer.rs`
**What:** `RouterPortForwardGuideComposer.swift` (306 lines) — "composes fully ordered logical guide structures from seed data, merging router-specific steps, prerequisites, value summaries, and notes into a renderable section list" (the file's own doc comment). Characterize section ordering, merge precedence when a router-specific step overrides a shared one, and prerequisite/value-summary/notes assembly. Reuses P1.10's sample catalog.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/router-composer --expect <n>` → `ok <n>`; then `cargo nextest run -p msc-domain router_composer` → `<n> tests run: <n> passed`
**Commit:** `P1.12: characterize and port the router guide composer`
**Batch:** solo

### P1.13 — Characterize and port the router troubleshooting engine
**Status:** not started
**Files:** `fixtures/router-troubleshooting/`, `crates/msc-domain/src/router/troubleshooting.rs`, `crates/msc-domain/tests/router_troubleshooting.rs`
**What:** `RouterPortForwardTroubleshootingEngine.swift` (550 lines) — a "rule-based troubleshooting engine for router and port-forwarding failures. Accepts user-reported symptoms and returns prioritised causes and recommended actions" (the file's own doc comment). Characterize the full `RouterPortForwardSymptomID` set from the source's rule table: each symptom's ranked causes, recommended actions, and any linked-topic references.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/router-troubleshooting --expect <n>` → `ok <n>`; then `cargo nextest run -p msc-domain router_troubleshooting` → `<n> tests run: <n> passed`
**Commit:** `P1.13: characterize and port the router troubleshooting engine`
**Batch:** solo

### P1.14 — Characterize and port the router runtime resolver
**Status:** not started
**Files:** `fixtures/router-runtime-resolver/`, `crates/msc-domain/src/router/runtime_resolver.rs`, `crates/msc-domain/tests/router_runtime_resolver.rs`
**What:** `RouterPortForwardGuideRuntimeResolver.swift` (291 lines) — already adjudicated agent-owned in the symbol ledger (`msc2-symbol-ledger.csv`, two rows for this file: `makeRecommendedProtocol`, and the `resolve`/`resolveGuide`/`resolveBestMatch`/`resolveItem`/`resolveText` family — the latter corrected to agent during Codex's P0.27 review, on the strength of D-026 naming it directly). Resolves dynamic tokens (selected server's live IP/gateway/ports) against a composed guide's placeholders, plus `makeRecommendedProtocol`'s rule (TCP always; UDP only when Bedrock or Geyser is enabled). Characterize both: token resolution across a `RouterPortForwardGuideRuntimeContext` fixture matrix (server selected/not selected, IP known/unknown, Bedrock on/off), and the protocol-recommendation rule across Java-only / Java+Geyser / Bedrock-only combinations.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/router-runtime-resolver --expect <n>` → `ok <n>`; then `cargo nextest run -p msc-domain router_runtime_resolver` → `<n> tests run: <n> passed`
**Commit:** `P1.14: characterize and port the router runtime resolver`
**Batch:** solo

---

### Phase exit

### P1.15 — Phase 1 exit gate check
**Status:** not started
**Files:** none (verification only)
**What:** Run every Phase 1 domain together and confirm the crate stays clean: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and the full `cargo nextest run -p msc-domain` suite (every domain from P1.3–P1.14 in one run). Confirm `msc-domain` still carries no I/O dependency, per its module-boundary rule — check its `Cargo.toml` pulls in no filesystem/network/process crates. This checks the port plan's own Phase 1 exit criteria verbatim: "Rust passes the Phase 0 pure fixtures. No user files touched."
**Verify:** `cd ~/msc2 && cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo nextest run -p msc-domain` → all green; then `grep -E '^\s*(tokio|reqwest|std::fs|walkdir|notify)' crates/msc-domain/Cargo.toml` → no matches
**Commit:** _(n/a — verification only, unless a fix is needed)_
**Batch:** stop-after

---

## Amendments log

When a review amends an earlier phase or a decision, record it here so the change isn't silent.

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
