# MSC 2 — Rolling Plan

> ## STATUS: Setup complete (S.1–S.4 verified) — Phase 0 planned, awaiting Read
> **Next move:** READ (Cameron reviews the Phase 0 step list below)
> **Repo:** https://github.com/ctemple9/msc2 · CI green on macOS, Linux, Windows
> **Last updated:** 2026-07-29

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

**Status is only moved to DONE by Cameron**, after he runs the Verify command himself. An agent may set it to *awaiting verification* and stop.

---

## Phases

Gates are in `msc2-port-plan.md`. This is the map, not the detail.

| Phase | Name | State |
|---|---|---|
| **Setup** | Repo, docs, agent instructions, CI, editor config | complete |
| **0** | Freeze the baseline and build the harness | **next** |
| 1 | Domain types and pure rules | not started |
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

48 steps, six groups:

| Group | Steps | Deliverable |
|---|---|---|
| Fixture harness | P0.1–P0.2 | format spec + runner tool |
| Extract the 270 MSC 1 tests | P0.3–P0.21 | `fixtures/**/*.json`, one dir per source test file |
| Reference corpus | P0.22 | `corpus/` scaffold |
| API baseline | P0.23, P0.23a–P0.23s, P0.24 | `docs/msc2/api-baseline/`, checker script + one step per route family |
| Symbol ledger | P0.25, P0.26, P0.26a, P0.27 | `docs/msc2/audit/msc2-symbol-ledger.csv` |
| Sidecar IPC contract | P0.28 | `docs/msc2/sidecar-ipc-contract.md` |

---

### Fixture harness

### P0.1 — Fixture format spec
**Status:** awaiting verification
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
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `servers/{create,import,delete,rename,eula}` routes, read from the relevant `RemoteAPIServer*.swift` file(s) and `RemoteAPIServerDTOs.swift`. Behavior as MSC 1 has it, not aspirational.
**Verify:** `python3 tools/api-baseline-check.py servers` → `ok 5`
**Commit:** `P0.23a: add servers API baseline routes`

### P0.23b — API baseline: `settings` route
**Status:** not started
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `settings` route.
**Verify:** `python3 tools/api-baseline-check.py settings` → `ok 1`
**Commit:** (filled in by the executing agent)

### P0.23c — API baseline: `worlds` routes
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `worlds/{create,rename,replace,repair,activate}` routes.
**Verify:** `python3 tools/api-baseline-check.py worlds` → `ok 5`
**Commit:** `P0.23c: add worlds API baseline routes`

### P0.23d — API baseline: `components` routes
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `components/{install,remove,update,version}` routes.
**Verify:** `python3 tools/api-baseline-check.py components` → `ok 4`
**Commit:** `P0.23d: add components API baseline routes`

### P0.23e — API baseline: `backups` routes
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `backups/{now,restore,config}` routes.
**Verify:** `python3 tools/api-baseline-check.py backups` → `ok 3`
**Commit:** `P0.23e: add backups API baseline routes`

### P0.23f — API baseline: `config` routes
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `config/{ram,java-runtime,geyser}` routes.
**Verify:** `python3 tools/api-baseline-check.py config` → `ok 3`
**Commit:** `P0.23f: add config API baseline routes`

### P0.23g — API baseline: `users` routes
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `users/{create,update,revoke}` routes.
**Verify:** `python3 tools/api-baseline-check.py users` → `ok 3`
**Commit:** `P0.23g: add users API baseline routes`

### P0.23h — API baseline: `health/repair` route
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `health/repair` route.
**Verify:** `python3 tools/api-baseline-check.py health` → `ok 1`
**Commit:** `P0.23h: add health/repair API baseline route`

### P0.23i — API baseline: `playit` routes
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `playit/*` routes. MSC 1's docs don't state an exact sub-route count for this family — read it straight from the source instead of assuming one.
**Verify:** `python3 tools/api-baseline-check.py playit` → `ok 3` (recorded from the live source: GET /playit, POST /playit/start, POST /playit/stop — not an assumed number)
**Commit:** `P0.23i: add playit API baseline routes`

### P0.23j — API baseline: `broadcast` routes
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `broadcast/*` routes. Sub-route count not stated in the docs — read it from the source.
**Verify:** `python3 tools/api-baseline-check.py broadcast` → `ok 10` (recorded from the live source, not an assumed number)
**Commit:** `P0.23j: add broadcast API baseline routes`

### P0.23k — API baseline: `resourcepacks` routes
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `resourcepacks/*` routes. Sub-route count not stated in the docs — read it from the source.
**Verify:** `python3 tools/api-baseline-check.py resourcepacks` → `ok 5` (recorded from the live source, not an assumed number)
**Commit:** `P0.23k: add resourcepacks API baseline routes`

### P0.23l — API baseline: `watchdog` routes
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `watchdog/*` routes. Sub-route count not stated in the docs — read it from the source.
**Verify:** `python3 tools/api-baseline-check.py watchdog` → `ok 3` (recorded from the live source, not an assumed number)
**Commit:** `P0.23l: add watchdog API baseline routes`

### P0.23m — API baseline: `command` route
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `command` route.
**Verify:** `python3 tools/api-baseline-check.py command` → `ok 1`
**Commit:** `P0.23m: add command API baseline route`

### P0.23n — API baseline: `start` route
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `start` route.
**Verify:** `python3 tools/api-baseline-check.py start` → `ok 1`
**Commit:** `P0.23n: add start API baseline route`

### P0.23o — API baseline: `stop` route
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `stop` route.
**Verify:** `python3 tools/api-baseline-check.py stop` → `ok 1`
**Commit:** `P0.23o: add stop API baseline route`

### P0.23p — API baseline: `allowlist` route
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `allowlist` route.
**Verify:** `python3 tools/api-baseline-check.py allowlist` → `ok 1`
**Commit:** `P0.23p: add allowlist API baseline route`

### P0.23q — API baseline: `players` routes
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `players/*` routes. Sub-route count not stated in the docs — read it from the source.
**Verify:** `python3 tools/api-baseline-check.py players` → `ok 4` (recorded from the live source, not an assumed number)
**Commit:** `P0.23q: add players API baseline routes`

### P0.23r — API baseline: `duckdns` route
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `duckdns` route.
**Verify:** `python3 tools/api-baseline-check.py duckdns` → `ok 1`
**Commit:** `P0.23r: add duckdns API baseline route`

### P0.23s — API baseline: `templates` route
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/openapi.json`
**What:** Add the `templates` route. This is the last family step; once it lands, the full file should contain all 87 routes MSC 1 exposes today (49 POST + 38 GET, per `msc2-engineering.md` §5) — worth a final sanity check with `python3 tools/api-baseline-check.py --total` (a mode the P0.23 script also provides) alongside this step's own depth check, since no single family step asserts the grand total.
**Verify:** `python3 tools/api-baseline-check.py templates` → `ok 1`
**Commit:** `P0.23s: add templates API baseline route`

### P0.24 — Capture the WebSocket event schema
**Status:** awaiting verification
**Files:** `docs/msc2/api-baseline/websocket-events.json`
**What:** MSC 1 has exactly one real-time WebSocket channel, not six — read from `RemoteAPIServer+WebSocket.swift`, the upgrade dispatch in `RemoteAPIServer+HTTP.swift`, and `consoleBuffer`/broadcast in `RemoteAPIServer.swift`. Document `console` (`/console/stream`): the RFC 6455 upgrade handshake (Sec-WebSocket-Key → accept key); auth (the same Bearer-token check as every HTTP route — any authenticated role may connect, no extra permission gate on the GET); the `ConsoleLineDTO` payload (`ts`, `source`, `level?`, `text`), one per text frame; the bounded-history-then-live delivery model (200-line backfill via `tailConsoleLines(n: 200)` sent immediately on connect, then live lines as they arrive); the 5000-line ring buffer (`consoleBufferLimit`) console history is capped at; ping/pong/close frame handling; the 64 KB inbound frame cap (`maxWebSocketClientFrameBytes`); and why inbound text frames are intentionally ignored (one-way — the server never executes WS-received text as a command). `status`/`operation progress`/`players`/`notifications`/`metrics` are **not** WebSocket channels in MSC 1 — those are HTTP-polled (`GET /status`, `GET /players`, etc.). The "six channels" language in `msc2-engineering.md` §5 describes MSC 2's intended design, not MSC 1's baseline; per D-006 the api-baseline captures MSC 1 as it is, and extensions are designed in Phase 2, not invented here. See the Amendments log.
**Verify:** `python3 -c "import json;d=json.load(open('docs/msc2/api-baseline/websocket-events.json'));print(len(d['channels']))"` → `1`
**Commit:** `P0.24: capture the one real WebSocket channel, not six`

---

### Symbol ledger

### P0.25 — Symbol ledger schema and UI density scanner
**Status:** awaiting verification
**Files:** `docs/msc2/audit/symbol-ledger-format.md`, `tools/symbol-scan/scan.py`
**What:** Define the ledger's columns (`file`, `bucket`, `symbol`, `kind` [parser/policy/workflow], `disposition` [agent/client], `target_domain`, `source_line`, `notes`) — one row per agent-owned symbol found inside a Mixed or UI file, per D-016. Build the density scanner the reconciliation audit already used (`msc2-audit-reconciliation.md`, "D1 — The Mixed bucket"): grep MSC 1's UI-bucket files (`msc2-codex-file-inventory.csv`, `bucket=ui`) for `FileManager`, `Process(`, `URLSession`, `func parse*/detect*/validate*/resolve*`, `JSONDecoder`, string-range extraction, and rank by hit count, output one file per line sorted by hit count descending. This is a live scan, not a check against the reconciliation doc's earlier count of 15 — that count may be stale, so whatever the scan finds is the number, and P0.27 records it rather than assuming 15.
**Verify:** `python3 tools/symbol-scan/scan.py --bucket ui --min-hits 3 "$HOME/Documents/Swift Projects/minecraft-server-controller"` → a ranked, non-empty file list; note the count shown
**Commit:** `P0.25: build symbol ledger schema and UI density scanner`

### P0.26 — Populate the ledger: Mixed-bucket files
**Status:** awaiting verification
**Files:** `docs/msc2/audit/msc2-symbol-ledger.csv`
**What:** For every file Codex's reconciled inventory marks `bucket=mixed` (59 files, `msc2-codex-file-inventory.csv`), open it in MSC 1 and add one ledger row per parser/policy/workflow symbol, using the deletion test in `msc2-port-plan.md` §1 to decide agent vs. client. A file with genuinely nothing to extract still gets one row saying so — coverage must be provable, not assumed. 293 rows across all 59 files (one file, `AppViewModel+FinderTools.swift`, had nothing to extract and got the single `(none)` row the coverage rule requires).
**Verify:** `python3 -c "import csv;rows=list(csv.DictReader(open('docs/msc2/audit/msc2-symbol-ledger.csv')));print(len({r['file'] for r in rows if r['bucket']=='mixed'}))"` → `59`
**Commit:** `P0.26: populate the symbol ledger for mixed-bucket files`

### P0.26a — Symbol ledger bucket-count checker script
**Status:** awaiting verification
**Files:** `tools/symbol-ledger-check.py`
**What:** A dependency-free Python script, `tools/symbol-ledger-check.py <bucket> --scan-source <path>`, used as P0.27's Verify command. It counts unique `file` values in `docs/msc2/audit/msc2-symbol-ledger.csv` for the given `bucket`, re-runs P0.25's scanner (`tools/symbol-scan/scan.py --bucket ui --min-hits 3`) against `--scan-source`, asserts the two counts match exactly, and prints `ok <n>` — so the check stays live against whatever the scanner currently finds, never a number frozen in the plan. Ships with a `--selftest` mode against two bundled temp CSVs (one matching, one deliberately short a row) so it's checkable before the real ledger or a scan source exists.
**Verify:** `python3 tools/symbol-ledger-check.py --selftest` → `pass=0` then `fail=1`
**Commit:** `P0.26a: build symbol ledger bucket-count checker script`

### P0.27 — Populate the ledger: flagged UI files
**Status:** not started
**Files:** `docs/msc2/audit/msc2-symbol-ledger.csv`
**What:** For every file P0.25's scanner actually flagged at ≥3 hits (includes the already-known `OverviewChatCardView.swift` console parser — but don't assume the reconciliation doc's earlier count of 15 still holds, since the source may have moved since that doc was written), open it and add ledger rows the same way. This is what turns "static scanning flags candidates" into an actual disposition record instead of a hunch.
**Verify:** `python3 tools/symbol-ledger-check.py ui-flagged --scan-source "$HOME/Documents/Swift Projects/minecraft-server-controller"` → `ok <n>` (live count, not fixed)
**Commit:** (filled in by the executing agent)

---

### Sidecar IPC contract

### P0.28 — macOS Bedrock sidecar IPC contract
**Status:** not started
**Files:** `docs/msc2/sidecar-ipc-contract.md`
**What:** Read `VMBedrockServerBackend.swift` (451 lines) and write the process protocol the Rust agent will use to drive the macOS Bedrock sidecar — transport (JSON lines over stdio, or a unix socket — pick one and record why) plus one section per message type: provision, start, readiness signal, stop, force-stop, crash notification, console stream, command input, shared-directory mapping, host-directory persistence across VM replacement (`msc2-engineering.md` §9). A contract informed by what MSC 1's sidecar actually does today, not a fresh design.
**Verify:** `grep -c '^### ' docs/msc2/sidecar-ipc-contract.md` → `10`
**Commit:** (filled in by the executing agent)

---

## Amendments log

When a review amends an earlier phase or a decision, record it here so the change isn't silent.

### 2026-07-30 — P0.24 amended: one WebSocket channel, not six

P0.24 originally asked to document "six WS channels (console, status, operation progress, players, notifications, metrics)." Reading `RemoteAPIServer+WebSocket.swift` and every call site of its JSON-send function (`RemoteAPIServer.swift`, `RemoteAPIServer+HTTP.swift`) shows MSC 1 implements exactly one: console-line streaming over `/console/stream`. There is no "channel" concept anywhere in the source (zero matches for `channel`/`Channel`); status, players, notifications, and metrics are all HTTP-polled, never pushed. The "six channels" language traces to `msc2-engineering.md` §5's description of MSC 2's *intended* design, not MSC 1's actual baseline. Per D-006 — the api-baseline captures MSC 1 as it is; extensions are designed in Phase 2, not invented here — the step is amended to document the one real channel. `Verify` changed from expecting `6` channels to `1`.
