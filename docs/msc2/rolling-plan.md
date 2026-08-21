# MSC 2 — Rolling Plan

> ## STATUS: Phase 7 is closed on exact candidate `79d5044`. Phase 8 is planned and awaiting Cameron's READ.
> **Next move:** READ — Cameron reviews the Phase 8 boundary, QUESTION 1, step list, Verify commands, and batch ranges before any Phase 8 implementation begins.
> **Repo:** https://github.com/ctemple9/msc2 · Phase 7 candidate branch `phase7-corrections` at `79d5044`; GitHub Actions run [32448912726](https://github.com/ctemple9/msc2/actions/runs/32448912726) is fully green across repo invariants, macOS, Linux, Windows, both Phase 6/7 smokes, and the headless no-GUI link check.
> **Last updated:** 2026-08-21

**Previous phases (Setup, Phase 0 through Phase 7) and their amendments have moved to `rolling-plan-archive.md`** to keep this file small. That archive is historical only — current status, active work, and every amendment from Phase 8 onward stay here.

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
| 3 | Safety substrate | complete |
| 4 | Java lifecycle vertical slice | complete |
| 5 | Configuration and migration | complete |
| **6** | Worlds and backups | complete |
| **7** | Server families and provisioning | complete |
| **8** | Mods, plugins, modpacks | **planned — READ next** |
| 9 | Networking and helpers | not started |
| 10 | Bedrock runtimes | not started |
| 11 | Desktop and web clients | not started |
| 12 | Terminal UI (deferred from v1) | not started |

---

## Phase 6 — Worlds and backups

All 51 steps (P6.1–P6.51) are `DONE`, and Codex's gate review (2026-08-18) confirms the gate holds on exact candidate `8568dea`. The full record — scope, characterization, world/backup services, public-client wiring, gate-review corrections, exact-commit tri-platform CI proof, and the review itself — has moved to `rolling-plan-archive.md`.

---

## Phase 7 — Server families and provisioning

All 38 steps (P7.1–P7.38) are `DONE` per Cameron. The final exact candidate is `79d5044` on `phase7-corrections`; GitHub Actions run [32448912726](https://github.com/ctemple9/msc2/actions/runs/32448912726) is fully green across macOS, Linux, Windows, the Phase 7 provisioning/launch smoke, and the headless no-GUI check. The full record — scope, characterization, six-family providers and provisioning, runtime and installer behavior, diagnostics and repairs, review corrections, evidence, and amendments — has moved to `rolling-plan-archive.md`.

---

## Phase 8 — Mods, plugins, modpacks

**Gate** (`msc2-port-plan.md` §3): "Modrinth / Hangar / CurseForge providers · metadata parsing · dependency resolution · client-only classification · pack-managed guards · import · update · client export."

**Working exit criteria:** a supported Java server can discover its installed mods or plugins, identify exact Modrinth projects by SHA-512, report compatible updates, install/remove/enable/disable/update one add-on or a dependency-aware batch through the public API and CLI, and preserve disabled state and user-linked provenance across replacement. Modrinth catalog search is filtered to the active server's Minecraft version, loader, and add-on kind. GitHub, Modrinth, Hangar, and direct-URL plugin sources resolve without guessing. A staged `.mrpack` or CurseForge archive can be inspected and used to create a correctly pinned Fabric/Forge/NeoForge server as a durable, cancellable operation; archive traversal, corrupt downloads, interruption, dependency cycles, and failed overrides leave no half-created server. Manifest, Modrinth metadata, embedded JAR metadata, and the known client-only list are applied in the MSC 1 precedence order. Pack-managed servers refuse individual mutation while still allowing an explicit whole-pack replacement path. CurseForge files whose authors block API distribution follow Cameron's answer to QUESTION 1. Client export produces links for Paper-like servers and a portable ZIP for modded servers without shelling out to macOS-only tools. Phase 7's `update`/`install` health repairs become real and are only reported successful after the filesystem and persisted problem record both verify. The copied iOS components/catalog/export flows, CLI, synthetic smoke, and macOS/Linux/Windows CI all exercise the same public contract. Provider failure is reported honestly; no test except the explicit real-evidence step uses the public network.

**Source oracle:** MSC 1 at `~/Documents/Swift Projects/minecraft-server-controller`, read-only. Primary files: `AddonUpdateResolver.swift`, `ModrinthAPI.swift`, `CurseForgeAPI.swift`, `CurseForgeModpack.swift`, `HangarAPI.swift`, `PluginSourceDetector.swift`, `PluginDownloader.swift`, `ModJarMetadataParser.swift`, `ModpackClientOnlyClassifier.swift`, `AppViewModel+AddonUpdates.swift`, `AppViewModel+ModManagement.swift`, `AppViewModel+PluginManagement.swift`, `AppViewModel+ComponentsVersions.swift`, `AppViewModel+ClientExport.swift`, `AppViewModel+ServerCreation.swift` (`applyStagedAddOn`), `AppViewModel+APIWiringAddons.swift`, `AppViewModel+APIWiringContent.swift`, `RemoteAPIServer+ComponentRoutes.swift`, `CurseForgeManualDownloadSheet.swift`, and the copied iOS `ComponentsView.swift`, `CatalogBrowserView.swift`, `HealthView.swift`, `DashboardViewModel.swift`, `RemoteAPIClient.swift`, and `RemoteAPIModels.swift`.

**Existing routes this phase makes real:** `GET /v1/addons` · `GET /v1/catalog/search` · `GET /v1/components` · `GET /v1/components/client-export` · `POST /v1/components/install` · `POST /v1/components/remove` · the add-on forms of `POST /v1/components/update` · and the `update`/`install` actions of `POST /v1/health/repair`. `POST /v1/components/version` remains Phase 7 behavior. P8.9 may add only the staged-upload-backed inspection/import and manual-file endpoints needed to expose MSC 1 capabilities that have no baseline route; it must reuse `/v1/staged-uploads` rather than accepting an arbitrary client path or base64-encoding large uploads.

30 steps, eight groups:

| Group | Steps | Deliverable |
|---|---|---|
| Scope and evidence | P8.1–P8.3 | authoritative boundary and D-027 answer, self-tested corpus checker, real pack/provider evidence |
| Characterization and contract | P8.4–P8.9 | source/update, dependency/classification, modpack/rollback, export/manual-download fixtures, reconciled API and operation model |
| Pure domain | P8.10–P8.12 | provider/source models, update resolver, dependency/client-only/pack policy |
| Infrastructure | P8.13–P8.15 | fakeable providers, verified add-on replacement, bounded dependency resolver |
| Application services | P8.16–P8.23 | inventory/update state, mutations, modpack inspection/import/create, export, health repair |
| Public clients | P8.24–P8.26 | routes, CLI, copied iOS |
| Proof | P8.27–P8.29 | one synthetic public-path smoke, real provider/pack evidence, exact-candidate tri-platform CI |
| Gate | P8.30 | one literal Phase 8 gate check and the phase's only full-workspace test run |

**Planned batch ranges:** after P8.3 is verified, P8.4–P8.8 may run together and stop at P8.8; after P8.9 is verified, P8.10–P8.12 may run together; P8.13 ends its batch because it is the first network-provider boundary; P8.14–P8.15 may run together and stop at P8.15; P8.16–P8.19 may run together; P8.20 ends its batch because it implements Cameron's D-027 choice and CurseForge's partial-download behavior; P8.21–P8.23 may run together and stop at P8.23; P8.24–P8.26 may run together. P8.27–P8.30 are each solo or stop-after. No batch crosses a failed Verify.

**Verification budget:** steps use only their focused fixture directory, crate, route module, client build, or Phase 8 smoke. Do not add `cargo nextest run --workspace` to any step. The full workspace suite appears exactly once, in P8.30, because that step is the phase gate. P8.29 checks CI rather than repeating the suite locally.

**Not in this phase**, deferred on purpose:

- Geyser/Floodgate downloads and updates, Playit, resource-pack hosting, DuckDNS, Xbox Broadcast, notifications, and helper lifecycle stay Phase 9. `GET /v1/components` may report their installed state but must label unavailable updates honestly.
- Bedrock add-ons and packs stay Phase 10. Phase 8 supports the Java add-on folders and Java loader/runtime shapes already proven in Phase 7.
- Desktop/web screens and serving help content stay Phase 11. Phase 8 exposes the complete agent contract and keeps those matrix cells `Planned`.
- CurseForge browsing is not invented. The named provider is used for CurseForge modpack metadata/files and the author-blocked download workflow; MSC 1's searchable in-app catalog remains Modrinth.
- A general remote file browser is not built here. Add-on/modpack input reuses the bounded, purpose-tagged staged-upload primitive from Phase 6.
- Whole-pack marketplace updating is limited to replacing from an explicitly supplied pack archive. Automatic discovery of "the next version" of an arbitrary imported pack is not promised by MSC 1 and is not inferred.

### Question before P8.1

**Answered by Cameron Temple, 2026-08-21:** option **(a)** — the client downloads an author-blocked CurseForge file and uploads it through MSC's bounded staged-upload path; the agent verifies it and resumes the pending pack operation. P8.1 records this as the D-027 decision.

```
QUESTION 1 — How should blocked CurseForge files reach a headless server?

What it is:      Some CurseForge authors forbid API downloads. MSC 1 opens each file's
                 web page, watches the Mac's Downloads folder, matches the filename,
                 and moves it into that same Mac's server. A remote/headless agent has
                 no browser or relationship to the client's Downloads folder.

The choice:      (a) The client downloads the file, then uploads it through MSC's bounded
                     staged-upload path; the agent verifies the expected filename/file ID
                     and finishes the pending pack operation.
                 (b) Keep the convenience only when client and agent are the same machine,
                     and otherwise tell the user to place the file manually on the host.

Why it matters:  This decides whether CurseForge pack import works coherently from a phone,
                 laptop, or CLI against a headless host, and whether P8.9 adds a small
                 purpose-bound manual-file completion endpoint. It does not permit a
                 general arbitrary-path upload.

If unsure:       (a). Phase 6 already built bounded staged uploads, so this preserves MSC 1's
                 convenience without assuming the browser and Minecraft server share a disk.
```

### Scope and evidence

### P8.1 — Scope Phase 8 and settle the D-027 workflow
**Status:** DONE
**Files:** `docs/msc2/addons/phase8-scope.md`, `docs/msc2/msc2-decisions.md`, `docs/msc2/audit/msc2-symbol-ledger.csv`
**What:** Read every Phase 8 oracle symbol against the current Rust inventory, Phase 7 handoff, frozen routes, and staged-upload primitive. Record the exact provider purposes, add-on identity/update precedence, pack-managed rule, modpack create/import boundary, rollback/cancellation contract, client-export behavior, Phase 9 exclusions, and every owned ledger row (`addon-updates`, `components`/`components-versions` only where not Phase 9, `modpack-client-only`, `modpack-import`, `modpacks`, `modrinth-deps`, `mods`, `plugin-management`, `plugins`, and `applyStagedAddOn`). Record Cameron's answer to QUESTION 1 as a dated D-027 decision. Write no Rust.
**Verify:** `python3 -c "from pathlib import Path; s=Path('docs/msc2/addons/phase8-scope.md').read_text().lower(); required=['modrinth','hangar','curseforge','dependency','client-only','pack-managed','rollback','staged upload','client export','phase 9','d-027']; missing=[x for x in required if x not in s]; assert not missing, missing; print('OK')"`
**Commit:** `P8.1: scope Phase 8 add-ons and modpacks`
**Batch:** solo

**Actual result:** Read all nine core oracle files in full (`AddonUpdateResolver.swift`, `ModrinthAPI.swift`, `CurseForgeAPI.swift`, `CurseForgeModpack.swift`, `HangarAPI.swift`, `PluginSourceDetector.swift`, `PluginDownloader.swift`, `ModJarMetadataParser.swift`, `ModpackClientOnlyClassifier.swift`) plus the relevant slices of `AppViewModel+AddonUpdates.swift`, `AppViewModel+ModManagement.swift`, `AppViewModel+PluginManagement.swift`, `AppViewModel+ClientExport.swift`, `AppViewModel+ServerCreation.swift` (`applyStagedAddOn`), `RemoteAPIServer+ComponentRoutes.swift`, `CurseForgeManualDownloadSheet.swift`, `GitHubReleaseChecker.swift`, `AppViewModel+ComponentsVersions.swift`, `AppViewModel+HealthCards.swift`, `AppViewModel+ServerInfo.swift`, `AppConfig.swift`, and `AppModels.swift`, against the current Rust workspace (`crates/msc-application/src/add_on_inventory.rs`'s existing P7.36 scanner, `msc-domain`'s opaque `plugin_sources`/`addon_links` pass-through), Phase 7's own handoff (`phase7-scope.md`), the frozen `openapi.json` routes, and Phase 6's staged-upload primitive (`crates/msc-agent/src/routes/worlds.rs`). Wrote `docs/msc2/addons/phase8-scope.md` (provider purposes table, add-on identity/update precedence, dependency depth-guard-vs-cycle-detection distinction, the two-different-precedence-chains finding for client-only classification, the pack-managed-guard finding, the modpack create/import boundary, a rollback/cancellation summary table, client-export behavior, Phase 9 exclusions, and the 35-row symbol-ledger table). Updated `msc2-decisions.md`'s D-027 from Open to Approved (option 1, dated 2026-08-21) and bumped the register to rev 1.5. Five findings worth flagging beyond the step's own checklist: (1) **the pack-managed guard does not exist in MSC 1 today** — `packManaged` gates only SwiftUI confirmation-dialog wording in two files, never a mutation; every install/remove/update call site proceeds unconditionally regardless of the flag, so Phase 8's "refuse individual mutation" criterion is new agent-owned policy, not a port. (2) **MSC 1 has no pack-driven server-creation primitive** — `importModpack`/CurseForge import both take an *already-existing* `ConfigServer`; the wizard's "create from pack" experience is an ordinary create followed by `applyStagedAddOn` calling the same in-place importer, and because that importer never `throw`s, the outer create rollback is structurally unreachable from a pack-import failure today — Phase 8's transactional-create guarantee is new rollback discipline, not a port. (3) **Client-only classification runs two different tier orders depending on path** — manifest-listed `.mrpack` files skip the Tier 0 hardcoded blocklist entirely (Tier 1 → 2 → 3), while override/CurseForge jars skip Tier 1 (Tier 0 → 2 → 3); `rolling-plan.md`'s working exit criteria describe one shared precedence, which only matches the second path. (4) **The dependency "cycle guard" is a flat recursion-depth cap of 3, not cycle detection** — a real visited-set detector is new work for P8.12, not a faithful port. (5) **`GET /v1/components`/`components-versions`'s 4 already-ledgered rows were fully ported by Phase 7's P7.19**, and the ledger's separate 9-row `components` domain (Paper-jar/Bedrock health cards, cross-play templates) is not Phase 8 material at all — it's Phase 7's still-open "jar" health-card gap and Phase 10 Bedrock territory; neither set is re-scoped to Phase 8 by this note. Verify command passes (`OK`). No Rust written, no other file touched.

### P8.2 — Build the Phase 8 corpus and gate checker first
**Status:** DONE
**Files:** `tools/phase8/phase8-check.py`, `tools/phase8/fixtures/`, `corpus/addons/README.md`, `corpus/packs/README.md`
**What:** Build one dependency-free checker before collecting evidence. Inventory mode verifies provenance, capture time, source URL, byte size, SHA-256, provider/purpose, and archive immutability; pack mode validates a real `.mrpack` and CurseForge archive contain their genuine manifest and referenced override roots without extracting outside a temporary root; fixture-coverage mode requires every Phase 8 provider and workflow named in P8.1. Add passing and deliberately failing self-tests for missing evidence, mutation, malformed JSON/archive, duplicate hash, unsafe path, absent family, and missing provider coverage. No network access.
**Verify:** `python3 tools/phase8/phase8-check.py --selftest`
**Commit:** `P8.2: build the Phase 8 evidence checker`
**Batch:** solo

**Actual result:** Built `tools/phase8/phase8-check.py`, stdlib-only, three independent modes, following `tools/phase7/provider-corpus-check.py`'s and `tools/phase6/corpus-check.py`'s established shape. **Inventory mode** (`--inventory [DIR]`, default `corpus/addons`) checks a `manifest.json`-recorded add-on evidence corpus: every file has a `provider` (one of the five `phase8-scope.md` "Provider purposes" table names — `modrinth`/`hangar`/`curseforge`/`github`/`direct`), `purpose`, `source_url`, `captured`, `sha256`, `byte_size`; hashes are recomputed and checked for both mutation and duplication; `.json` evidence parses; any `.zip`/`.mrpack`/`.jar` evidence file (an archive sample, e.g. an author-blocked CurseForge file) is checked for zip validity and safe (non-traversing) entries — this is where the step's "archive immutability" language is applied literally, on top of the ordinary provenance-hash check every evidence file gets. **Pack mode** (`--packs [DIR]`, default `corpus/packs`) checks a `manifest.json`-recorded pack-archive corpus with the same provenance/mutation/duplicate checks plus `pack_format` (`mrpack`/`curseforge`), then validates archive shape entirely in-memory via `zipfile` (nothing is ever extracted to disk, which is how "without extracting outside a temporary root" is satisfied — there is no root to escape): an `mrpack` must have a genuine `modrinth.index.json` with non-empty `game`/`versionId`/`name`/`dependencies`, and every other entry must fall under `overrides/`, `client-overrides/`, or `server-overrides/`; a `curseforge` archive must have a genuine `manifest.json` with `manifestType == "minecraftModpack"` and non-empty `minecraft.version`/`minecraft.modLoaders`/`name`/`version`/`overrides`, with the named `overrides` folder actually present in the archive. **Fixture-coverage mode** (`--coverage FIXTURE_DIR`) checks a fixture directory's optional `corpus_source` citations against the add-on corpus and its optional `workflow` field against the eight symbol-ledger domains `phase8-scope.md`'s "Symbol-ledger rows owned by this phase" table names (`addon-updates`, `modpack-client-only`, `modpack-import`, `modpacks`, `modrinth-deps`, `mods`, `plugin-management`, `plugins`), requiring all five providers and all eight workflows to be represented across the directory. `--inventory` and `--packs` may be combined in one invocation (confirmed against P8.3's own Verify line, `--inventory corpus/addons --packs corpus/packs`, which currently fails cleanly since neither directory is populated yet). 14 self-test cases under `tools/phase8/fixtures/` (6 inventory, 4 pack, 4 coverage) cover all seven required failure classes — missing evidence (`missing-provenance`), mutation (`mutated-input`), malformed JSON/archive (`malformed-json`, `pack-malformed-archive`), duplicate hash (`duplicate-hash`), unsafe path (`pack-unsafe-path`), absent family (`unknown-provider` — Phase 8's real vocabulary is "provider," not "family"; read as the same category P7.2's checklist wording named), and missing provider coverage (`coverage-missing-provider`, plus an unrequested but analogous `coverage-missing-workflow`) — plus one extra `pack-missing-manifest` case and a `coverage-dangling-citation` case mirroring P7.2's own citation check, both included as low-cost strengthening beyond the checklist's minimum. `--selftest` passes all 14 (`OK`, exit 0). Wrote `corpus/addons/README.md` (new) and rewrote `corpus/packs/README.md`, both following `corpus/providers/README.md`'s established per-directory convention: what's expected, both checker modes' rules, and the `<provider>/<name>.<ext>` naming convention P8.3 should use. No real evidence collected, no Rust written — that's P8.3.

### P8.3 — Record real provider responses and modpack archives
**Status:** awaiting verification
**Files:** `corpus/addons/`, `corpus/addons/README.md`, `corpus/packs/`, `corpus/packs/README.md`
**What:** Capture the smallest complete real response set used by MSC 1: Modrinth search/project/version/hash/update/dependency responses, Hangar latest-release metadata, CurseForge files/mods responses including one author-blocked file, and the GitHub/direct-source shapes used by plugin updates. Add one real `.mrpack` and one real CurseForge-format pack with their original bytes kept outside git when licensing or size requires it and a manifest/provenance record in git. If any provider or required pack evidence is unavailable, record the exact gap and stop; do not fabricate it.
**Verify:** `python3 tools/phase8/phase8-check.py --inventory corpus/addons --packs corpus/packs`
**Commit:** `P8.3: record Phase 8 provider and pack evidence`
**Batch:** stop-after

**Actual result:** Captured real, live responses for all five providers, all on 2026-08-21: **Modrinth** — `search-sodium.json` (`/v2/search`), `project-iris.json` (`/v2/project/iris`), `dependencies-iris.json` (`/v2/project/{id}/dependencies` — Iris genuinely requires Sodium, a real `required`-type edge), `version-list-iris-fabric-1.21.1.json` (`/v2/project/{id}/version` filtered by loader/game-version — the update-check shape), and `version-file-hash-iris.json` (`/v2/version_file/{sha512}` — exact-identity lookup, hash taken from the version-list response itself). **Hangar** — `project-essentials.json` and `versions-latest-essentials.json` for the real `Essentials` project. **GitHub** — `releases-latest-essentialsx.json` (`/repos/EssentialsX/Essentials/releases/latest`, real asset-name shapes). **Direct** — `luckperms-bukkit-direct-download.json`: since `PluginSourceDetector.detect` only classifies a URL string and has no JSON API to capture (the actual transfer is Phase 9's `PluginDownloader`), this instead records a real HEAD response (status/content-type/content-length/filename) against a genuine direct-download URL. **CurseForge** — initially blocked (confirmed live: an unauthenticated `GET https://api.curseforge.com/v1/games` returned `403`); Cameron supplied a real CurseForge Core API key in the next message, which unblocked it. With that key: `mods-files-blocked-entityculling.json`, a real `POST /v1/mods/files` response for a genuinely author-blocked file (Entity Culling Fabric/Forge, modId `448233`, `allowModDistribution: false`, file `8287121` — `downloadUrl: null`, `isAvailable: true`, exactly the D-027 pending-file shape); `mods-metadata-entityculling.json`, the matching `POST /v1/mods` response for that mod (name/slug/`websiteUrl`); and `mods-files-resolvable-fabulously-optimized-pack.json`, a normal non-blocked `POST /v1/mods/files` response for contrast. The API key itself was used only in-memory (shell env var, curl headers) and was never written to any file, commit, or log — confirmed by grepping the working tree for the key material before committing. Every evidence file is SHA-256-recorded in `corpus/addons/manifest.json`; `python3 tools/phase8/phase8-check.py --inventory corpus/addons --packs corpus/packs` passes (`providers present: curseforge, direct, github, hangar, modrinth`). Two real packs were captured for `corpus/packs/`, deliberately the same underlying pack in both formats so they cross-check each other: `fabulously-optimized-v13.3.0.mrpack` (Modrinth, fetched from `cdn.modrinth.com`, verified against Modrinth's own recorded SHA-512) and `fabulously-optimized-v13.3.0-curseforge.zip` (CurseForge modId `396246`, file `8439077`, resolved via the now-working `POST /v1/mods/files` → `downloadUrl` → `edge.forgecdn.net`, SHA-256-verified). Both are small enough (~152 KiB / ~146 KiB) to keep directly in git — no out-of-git storage needed. `python3 tools/phase8/phase8-check.py --packs corpus/packs` reports `formats present: curseforge, mrpack`. Both `corpus/addons/README.md` and `corpus/packs/README.md` were updated to describe every recorded file (no remaining gap section). `--selftest` still passes all 14 cases (unaffected — self-tests never touch the real corpus). Status set to *awaiting verification*, not DONE, per `CLAUDE.md`.

### Characterization and contract

### P8.4 — Characterize provider parsing and plugin-source resolution
**Status:** awaiting verification
**Files:** `fixtures/addon-providers/`, `fixtures/plugin-source-resolution/`, `docs/msc2/addons/phase8-scope.md`
**What:** Extract MSC 1 expectations for Modrinth search/facets, project/version/file/hash metadata, primary-file choice, CurseForge file/mod metadata and missing download URLs, Hangar platform/version selection, GitHub release assets, direct URLs, and URL-to-source parsing. Cover malformed responses and honest provider failure. Cite source lines or recorded evidence in every fixture.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/addon-providers --expect 33 && python3 tools/fixture-runner/run.py --validate-dir fixtures/plugin-source-resolution --expect 16`
**Commit:** `P8.4: characterize add-on providers and sources`
**Batch:** safe

**Actual result:** Read `ModrinthAPI.swift`, `CurseForgeAPI.swift`, `HangarAPI.swift`, `GitHubReleaseChecker.swift`, and `PluginSourceDetector.swift` in full, plus `AppViewModel+ComponentsVersions.swift`'s `fetchOnlineVersion` dispatch and `AppModels.swift`'s `PluginSourceType` enum (confirmed exactly 4 cases: `github`/`modrinth`/`hangar`/`direct`). Wrote 33 fixtures to `fixtures/addon-providers/` covering Modrinth (search facets including the plugin/mod OR-group special case, index sort, real-response decode, project client-only flag, empty-batch short circuit, primary-file selection on both the legacy single-plugin fetcher and the newer browser-API model, every thrown-error branch, exact-hash identity including the 404-is-not-an-error case, and batch-update body construction), CurseForge (missing-key/unauthorized/generic-error branches, ID dedup+sort before batching, and — grounded in P8.3's real captures — both an author-blocked file's `downloadUrl: null` shape and a normal resolvable file for contrast), Hangar (both download-URL branches, the empty-result error, and fallback-URL percent-encoding), GitHub (real 9-asset EssentialsX release proving first-array-order-match selection, the not-an-error empty case, case-insensitive suffix match), and the direct-URL dispatch case (invalid-URL guard, the literal `"(direct)"` version string, grounded against P8.3's real LuckPerms HEAD capture). Wrote 16 fixtures to `fixtures/plugin-source-resolution/` covering `PluginSourceDetector.detect`'s four-branch host/suffix classification plus blank/unrecognized-URL nil cases, and `parseGitHub`/`parseModrinth`/`parseHangar`/`stripScheme`'s exact segment parsing including each function's too-short-path nil guard. Every fixture cites an exact oracle file/line; 9 cite real `corpus/addons/` evidence via a `corpus_source` field (verified every citation resolves to a real file). Appended a short addendum to `phase8-scope.md` with two findings not already in the P8.1 note: Modrinth's plugin-search OR-group rationale, and that `parseModrinth` never actually validates its URL's `plugin`/`mod` segment despite the doc comment implying it does. The step's own Verify line was missing `--expect N` (required by `tools/fixture-runner/run.py`'s `--validate-dir` mode per this repo's own established convention — every prior phase's equivalent step pairs `--validate-dir` with `--expect`); fixed in this commit to `--expect 33`/`--expect 16`, both passing (`ok 33`, `ok 16`). Status set to *awaiting verification*, not DONE, per `CLAUDE.md`.

### P8.5 — Characterize add-on identity, update planning, and source persistence
**Status:** awaiting verification
**Files:** `fixtures/addon-update-resolution/`, `fixtures/plugin-source-mapping/`, `docs/msc2/addons/phase8-scope.md`
**What:** Characterize SHA-512 exact identity, persisted-link fallbacks, Geyser/Floodgate exclusion on plugin servers, compatible-version selection, all four update buckets, enabled/disabled filename handling, discovered-link merge precedence, user-linked preservation, stale-plan caching, plugin tier/source matching, source-key rekeying after versioned filename changes, and deterministic ordering.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/addon-update-resolution --expect 29 && python3 tools/fixture-runner/run.py --validate-dir fixtures/plugin-source-mapping --expect 24`
**Commit:** `P8.5: characterize add-on update resolution`
**Batch:** safe

**Actual result:** Read `AddonUpdateResolver.swift` in full (313 lines) plus the relevant slices of `AppViewModel+AddonUpdates.swift`, `AppViewModel+ComponentsVersions.swift`, and `AppViewModel+PluginManagement.swift` named in P8.1's ledger table, and `AddonLinkProvenance`/`AddonLink`/`PluginTier`/`PluginEntry`/`PluginSourceConfig` in `AppModels.swift`. Wrote 29 fixtures to `fixtures/addon-update-resolution/` covering: enabled/disabled jar enumeration and stem derivation; the plugin-server-only Geyser/Floodgate exclusion (and its mod-server non-exclusion counterpart); the three-clause identity fallback chain (fresh hash → persisted installedHash → persisted installedFileName → unlinked); all four update buckets including the two upToDate sub-cases (fresh-latest-match, and the persisted-installedVersionId comparison fallback used when this pass had no fresh hash hit); the no-compatible-version-vs-upToDate split depending on whether `minecraftVersion` is configured; self-healing link recording (written only on a fresh hash match, never on a persisted-fallback match); provenance assignment; the `cleanVersionLabel` loader-prefix-stripping helper (3 cases); deterministic bucket-then-alphabetical ordering; `mergeDiscoveredLinks`'s user-linked-preserving vs wholesale-overwrite merge (3 cases); the stale-plan cache's skip/force/server-switch behavior (3 cases); the concurrent (not sequential) identify+latest lookup; and the confirmed-dead `sha1Hex` fallback. Wrote 24 fixtures to `fixtures/plugin-source-mapping/` covering `PluginTier` derivation (managed always wins over a coincidental source match; userSourced; unmanaged), `findSource`'s exact-then-symmetric-prefix matching (4 cases, both prefix directions plus the no-match case), the Components-tab sort order (tier rank, Geyser-before-Floodgate convention, alphabetical tie-break), `setPluginSource`/`removePluginSource`'s create/replace/stale-prefix-sweep/empty-collapse behavior (5 cases), `downloadLatestForPlugin`'s post-download rekey (including both final-filename derivation branches and the old-file cleanup sweep that removes both enabled and disabled prior copies), the `.direct`-source short-circuit that skips the online check entirely, and the managed-vs-userSourced split in how online versions get populated (mirrored from the Components snapshot vs individually fetched). Every fixture cites an exact oracle file/line. Appended a short addendum to `phase8-scope.md`: a latent MSC 1 gap (a server with no configured `minecraftVersion` can never report `noCompatibleVersion`, only `upToDate`, for a linked add-on — preserved as oracle-faithful for P8.16, flagged for Cameron's awareness) and a confirmation that `findSource`'s prefix match is symmetric (works whether the on-disk stem grew or shrank), which P8.11's port must not narrow to one direction. The step's own Verify line was missing `--expect N`; fixed to `--expect 29`/`--expect 24`, both passing. Status set to *awaiting verification*, not DONE, per `CLAUDE.md`.

### P8.6 — Characterize dependencies, client-only precedence, and pack guards
**Status:** awaiting verification
**Files:** `fixtures/modrinth-dependencies/`, `fixtures/modpack-client-only/`, `fixtures/pack-managed-guard/`, `docs/msc2/addons/phase8-scope.md`
**What:** Fill the two ledgered client-only gaps (`knownClientOnlyReason`, disabled-path derivation), then characterize required vs optional dependencies, installed-dependency matching, transitive ordering, missing and circular graphs, the depth cap, manifest → Modrinth → embedded-JAR → known-list precedence, and every mutation the pack-managed guard refuses or permits. Extend existing fixture directories rather than duplicate their covered cases.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/modrinth-dependencies --expect 15 && python3 tools/fixture-runner/run.py --validate-dir fixtures/modpack-client-only --expect 28 && python3 tools/fixture-runner/run.py --validate-dir fixtures/pack-managed-guard --expect 17`
**Commit:** `P8.6: characterize dependency and pack policies`
**Batch:** safe

**Actual result:** Read `ModpackClientOnlyClassifier.swift` in full (already mostly characterized by 18 pre-existing fixtures from Phase 6/7) and confirmed via the oracle's own `ModpackClientOnlyTests.swift` that neither `knownClientOnlyReason` nor `disabledURL(forActiveJar:)` has an existing test — exactly the two gaps P8.1's ledger pass flagged. Added 10 new fixtures to `fixtures/modpack-client-only/` (now 28 total): 7 for the Tier 0 hardcoded shader/renderer blocklist (exact match, all three separator characters, case-insensitivity, and two negative cases — a genuinely unlisted mod, and a substring-without-separator near-miss that must NOT match) and 3 for `disabledURL`'s pure path computation (appends rather than replaces the extension, performs no filesystem access, matches what `disableJar` — already covered — calls internally). Read `installRequiredDependencies` (`AppViewModel+ModManagement.swift:271-328`) in full and wrote 15 fixtures to a new `fixtures/modrinth-dependencies/` directory covering the required-vs-optional filter, the depth-3 guard, both already-installed checks (mod-ID match, filename-slug scan) that run before every recursive call, the no-compatible-version/no-primary-file continue-not-fatal paths, per-dependency failure isolation, transitive recursion, the addOn-kind-conditional refresh dispatch, and two fixtures giving concrete shape to this note's own diamond-dependency and depth-cap-is-not-cycle-detection findings. For the pack-managed guard, confirmed by grep that MSC 1's production code has zero `packManaged` mutation-gating call sites anywhere (only two SwiftUI confirmation-dialog copy sites) — so per this note's own "Pack-managed guard" finding, 10 new fixtures were added to `fixtures/pack-managed-guard/` (now 17 total) citing the DECIDED contract (`docs/msc2/addons/phase8-scope.md`/`rolling-plan.md`'s working exit criteria) as their source rather than an MSC 1 file/line — refusing individual install/remove/toggle/update, allowing an explicit whole-pack replacement while refusing an ambiguous/implicit re-import, the non-pack-managed no-op contrast, health-repair's update/install actions inheriting the same gate, and dependency-installs never being reached for a refused parent mutation — plus one fixture (`msc1-baseline-warns-but-never-gates-contrast`) that cites the real oracle file (`AddonUpdateSheet.swift`) to document what MSC 1 actually does today, as the contrast baseline the rest of the set is measured against. Appended a short addendum to `phase8-scope.md`. All three Verify commands' own `--expect N` were missing; fixed to 15/28/17. Status set to *awaiting verification*, not DONE, per `CLAUDE.md`.

### P8.7 — Characterize modpack inspection, import, interruption, and rollback
**Status:** awaiting verification
**Files:** `fixtures/modpack-import/`, `fixtures/modpack-archive-safety/`, `fixtures/modpack-pinning/`, `fixtures/curseforge-modpack/`, `fixtures/mrpack-extraction/`, `docs/msc2/addons/phase8-scope.md`
**What:** Characterize `.mrpack`/CurseForge detection and pinning, file hashes, optional/client-only skips, Modrinth 100-ID metadata chunks, overrides then server-overrides precedence and permission bits, existing active/disabled JAR skips, CurseForge API-key absence and blocked-file pending state, plain top-level-JAR ZIP behavior, hostile paths/symlinks, corrupt archives/downloads, cancellation, partial override failure, and complete new-server rollback. Record explicit D-006 corrections where MSC 1 leaves partial files instead of inventing parity.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/modpack-import --expect 29 && python3 tools/fixture-runner/run.py --validate-dir fixtures/modpack-archive-safety --expect 7`
**Commit:** `P8.7: characterize safe modpack import`
**Batch:** safe

**Actual result:** Read `importModpack`, `importExtractedCurseForgeModpack`, `fetchMrpackProjectMetadata`, `classifyAndDisableManifestJar`, `disableClientOnlyOverrideJars`, and `mergeDirectory` in full (`AppViewModel+ModManagement.swift:407-866`). Wrote 29 fixtures to a new `fixtures/modpack-import/` covering: format detection (mrpack/CurseForge/unrecognized), manifest read/parse failure aborting before any write, pack-provenance being persisted before any file download is even attempted, Tier-1 pre-filtering, the disabled-jar-never-reclassified vs active-jar-always-reclassified asymmetry on re-import, ordered-mirror download with all-mirrors-failed being non-fatal, overrides-then-server-overrides copy order (server-overrides wins on conflict since merge always overwrites), the Modrinth 100-ID batch chunking with per-chunk-failure isolation, CurseForge's missing-API-key/blocked-file/manual-download-list behavior, a documented structural asymmetry between the two pack formats' re-import classification timing, guaranteed temp-directory cleanup via `defer`, and the confirmed fact that `importModpack` has no `throws` in its signature at all (every failure is logged, never propagated). Wrote 7 fixtures to a new `fixtures/modpack-archive-safety/` documenting that MSC 1 has **no archive- or download-integrity protection whatsoever**: manifest-declared per-file SHA-1/SHA-512 hashes are decoded but never checked against downloaded bytes (a genuinely new finding, not previously flagged anywhere in this note), no path-traversal guard exists on manifest-declared file paths, no symlink guard exists anywhere in extraction or the override merge, downloads write directly to their final path with no atomic temp-then-rename, and `mergeDirectory` unconditionally overwrites any destination with no conflict detection — contrasted against the one narrow safety property that IS already sound (the extraction temp-dir name is UUID-suffixed, not predictable), and linked explicitly to P8.14's working exit criteria as the step that closes these gaps. Deliberately left `fixtures/modpack-pinning/`, `fixtures/curseforge-modpack/`, and `fixtures/mrpack-extraction/` untouched — all three already characterize manifest-parsing/pinning/extraction-mechanics from Phase 6/7 (a different layer than this step's workflow-and-safety focus), and this step's own Verify line only names the two new directories, confirming they're out of scope here. Appended an addendum to `phase8-scope.md` recording the hash-verification gap as a new finding. Both Verify commands were missing `--expect N`; fixed to `--expect 29`/`--expect 7`. Status set to *awaiting verification*, not DONE, per `CLAUDE.md`.

### P8.8 — Characterize client export and manual-file completion
**Status:** awaiting verification
**Files:** `fixtures/client-addon-export/`, `fixtures/curseforge-manual-download/`, `docs/msc2/addons/phase8-scope.md`
**What:** Characterize export classification and ordering, default selection, Paper-like link text, modded ZIP contents and filenames, disabled JAR handling, empty/unsupported results, and deterministic portable archive bytes. Characterize the three-tier CurseForge filename matcher and translate Cameron's D-027 choice into purpose-bound staged-upload acceptance/rejection cases, including wrong file, expired token, duplicate browser suffix, and one-file fallback.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/client-addon-export --expect 28 && python3 tools/fixture-runner/run.py --validate-dir fixtures/curseforge-manual-download --expect 16`
**Commit:** `P8.8: characterize export and manual downloads`
**Batch:** stop-after

**Actual result:** Read `AppViewModel+ClientExport.swift` and `CurseForgeManualDownloadSheet.swift` in full. Wrote 28 fixtures to a new `fixtures/client-addon-export/` covering: `ClientSideStatus`'s default-selection rule (required/optional/unknown default checked, serverOnly doesn't), the Geyser/Floodgate exclusion, the plugin-server-only server-only/unknown drop (with the modded-server contrast that keeps both), the Modrinth-link-then-jar-manifest-then-assumed status source precedence, both classification value-mapping switches, the export-specific sort order (confirmed independent from AddonUpdateResolver's own bucket ranking), display-name precedence, empty-folder and no-add-on-kind guards, the modrinth-URL slug/projectId fallback, both delivery paths (clipboard link list and ZIP), the zip filename sanitization and MC-version fallback, the `/usr/bin/zip -j` flat-archive shape (confirmed as the fifth and final macOS-only shell-out site this note's Phase 8 read already enumerated), temp-staging cleanup, and exit-status handling — plus one finding not previously recorded: `ClientExportItem` has no `isEnabled` field at all, so a disabled `.jar.disabled` mod is exported identically to an active one, with no special handling anywhere in the function. Wrote 16 fixtures to a new `fixtures/curseforge-manual-download/`: 11 characterizing MSC 1's actual three-tier folder-watch matcher (exact filename, macOS duplicate-suffix with extension guard, single-remaining fallback with its ambiguity guard, partial-download-extension exclusion, pre-watch-snapshot exclusion, in-flight-claim exclusion, and the move-vs-cross-volume-copy-fallback), and 5 translating that same tolerance logic into Cameron's decided D-027 staged-upload contract (accept exact/duplicate-suffix/one-file-fallback, reject wrong-file/expired-token, and purpose-binding to one specific pending operation) — cited against `phase8-scope.md`'s own D-027 section as the decision source, per the docs-as-source convention P8.6 established for `pack-managed-guard`. Appended an addendum to `phase8-scope.md`. Both Verify commands were missing `--expect N`; fixed to `--expect 28`/`--expect 16`. This closes the P8.4-P8.8 batch range per `rolling-plan.md`'s own "Planned batch ranges" note and this step's `Batch: stop-after`. Status set to *awaiting verification*, not DONE, per `CLAUDE.md`.

### P8.9 — Reconcile the Phase 8 API, operations, and capability surface
**Status:** not started
**Files:** `docs/msc2/addons/phase8-api.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/client-capability-matrix.csv`, `crates/msc-api/tests/phase8_conformance.rs`
**What:** Reconcile every existing route with the oracle and working gate, then add only the missing staged-upload-backed contract for pack inspection/import, local JAR install, and—if selected—CurseForge manual-file completion. Long installs, updates, pack replacement, and pack-backed create return `202` with durable operation IDs; reads remain synchronous. Specify cancellation, pack-managed conflicts, provider errors, upload purpose/size limits, typed results, audit events, and `helpId`s. Keep Phase 9 component update shapes unavailable rather than pretending they work, and update matrix rows only for contract availability, not implementation.
**Verify:** `cargo nextest run -p msc-api --test phase8_conformance`
**Commit:** `P8.9: freeze the Phase 8 public contract`
**Batch:** solo

### Pure domain

### P8.10 — Port provider metadata and plugin-source rules
**Status:** not started
**Files:** `crates/msc-domain/src/addon_provider.rs`, `crates/msc-domain/src/plugin_source.rs`, `crates/msc-domain/src/lib.rs`, `crates/msc-domain/tests/addon_providers.rs`, `crates/msc-domain/tests/plugin_source_resolution.rs`
**What:** Port provider response models, primary-file selection, loader/add-on-kind facets, version labels, author-blocked classification, and GitHub/Modrinth/Hangar/direct URL parsing against P8.4. Keep transport out of the domain crate.
**Verify:** `cargo nextest run -p msc-domain -E 'test(/addon_provider|plugin_source/)'`
**Commit:** `P8.10: port add-on provider rules`
**Batch:** safe

### P8.11 — Port add-on identity and update planning
**Status:** not started
**Files:** `crates/msc-domain/src/addon_update.rs`, `crates/msc-domain/src/app_config_schema.rs`, `crates/msc-domain/src/lib.rs`, `crates/msc-domain/tests/addon_update_resolution.rs`, `crates/msc-domain/tests/plugin_source_mapping.rs`
**What:** Replace opaque `AddonLink`/`PluginSourceConfig` config pass-through with typed, unknown-field-preserving records, then port exact-hash identity, fallback precedence, update buckets, merge/rekey policy, and ordering. Preserve user-linked provenance while refreshing installed-file bookkeeping.
**Verify:** `cargo nextest run -p msc-domain -E 'test(/addon_update|plugin_source_mapping|app_config_schema/)'`
**Commit:** `P8.11: port add-on update planning`
**Batch:** safe

### P8.12 — Port dependency, client-only, and pack-managed policy
**Status:** not started
**Files:** `crates/msc-domain/src/addon_dependency.rs`, `crates/msc-domain/src/modpack.rs`, `crates/msc-domain/src/lib.rs`, `crates/msc-domain/tests/addon_dependency.rs`, `crates/msc-domain/tests/modpack_policy.rs`
**What:** Port dependency graph decisions, client-only precedence including the Tier 0 list, manifest/archive metadata and version pinning, disabled-path rules, manual-download matching, and the pack-managed mutation matrix. The domain layer returns decisions and typed errors only; it does not download or mutate files.
**Verify:** `cargo nextest run -p msc-domain -E 'test(/addon_dependency|modpack_policy/)'`
**Commit:** `P8.12: port dependency and modpack policy`
**Batch:** safe

### Infrastructure

### P8.13 — Build fakeable Modrinth, Hangar, CurseForge, and plugin-source providers
**Status:** not started
**Files:** `crates/msc-infrastructure/src/addon_provider.rs`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/tests/addon_provider.rs`, `corpus/addons/`
**What:** Build transport adapters behind one fakeable provider trait using configurable base URLs. Enforce status/body limits, timeouts, pagination/chunk caps, API-key lookup through `SecretStore`, and provider-specific response validation. Tests use only recorded corpus responses and a local fake HTTP server. This step fetches metadata; all payload installation waits for P8.14.
**Verify:** `cargo nextest run -p msc-infrastructure --test addon_provider`
**Commit:** `P8.13: build add-on provider adapters`
**Batch:** stop-after

### P8.14 — Build verified add-on staging and atomic replacement
**Status:** not started
**Files:** `crates/msc-infrastructure/src/addon_store.rs`, `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/src/download_staging.rs`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/tests/addon_store.rs`
**What:** Reuse Phase 3 download staging to stream into an operation-owned temporary area, enforce the publisher hash when present, reject hostile archive paths/symlinks, preserve executable/read permission bits where the pack supplies them, and atomically install/replace/remove/toggle JARs without clobbering an existing `.disabled` target. Provide interruption cleanup and rollback material; use Rust ZIP support, never `ditto` or `/usr/bin/zip`.
**Verify:** `cargo nextest run -p msc-infrastructure --test addon_store`
**Commit:** `P8.14: build verified add-on storage`
**Batch:** safe

### P8.15 — Build the bounded dependency installer
**Status:** not started
**Files:** `crates/msc-application/src/addon_dependencies.rs`, `crates/msc-application/src/lib.rs`, `crates/msc-application/tests/addon_dependencies.rs`
**What:** Resolve and install required dependencies through the provider/store boundaries, skip already-installed identities, preserve deterministic parent-before-child progress, reject or terminate cycles without duplicate downloads, honor cancellation between files, and roll back this operation's newly installed files on terminal failure. Optional dependencies remain explanatory, not silently installed.
**Verify:** `cargo nextest run -p msc-application --test addon_dependencies`
**Commit:** `P8.15: build dependency-aware add-on installs`
**Batch:** stop-after

### Application services

### P8.16 — Build durable add-on inventory and update resolution
**Status:** not started
**Files:** `crates/msc-application/src/add_on_inventory.rs`, `crates/msc-application/src/addon_updates.rs`, `crates/msc-application/src/lib.rs`, `crates/msc-application/tests/addon_updates.rs`
**What:** Extend Phase 7's real JAR inventory with provider/source tiers, SHA-512 identity, compatible latest versions, pack provenance, cache invalidation, and durable self-healing links. Return the `/v1/addons` model without holding global state across network awaits, and bound cached plans per D-021.
**Verify:** `cargo nextest run -p msc-application --test addon_updates`
**Commit:** `P8.16: build add-on update resolution`
**Batch:** safe

### P8.17 — Implement add-on install, update, toggle, remove, and source linking
**Status:** not started
**Files:** `crates/msc-application/src/addons.rs`, `crates/msc-application/src/addon_updates.rs`, `crates/msc-application/tests/addons.rs`
**What:** Implement catalog and staged-local install, one/all update, enable/disable, remove, manual Modrinth link, and plugin-source set/remove/update. Preserve disabled suffixes and source records across replacement, enforce stopped-server and pack-managed rules where required, serialize same-server mutations, verify the result before success, and audit each mutation. Leave Geyser/Floodgate/Broadcast special follow-up actions to Phase 9.
**Verify:** `cargo nextest run -p msc-application --test addons`
**Commit:** `P8.17: implement add-on lifecycle operations`
**Batch:** safe

### P8.18 — Implement safe modpack inspection and extraction
**Status:** not started
**Files:** `crates/msc-application/src/modpacks.rs`, `crates/msc-infrastructure/src/addon_store.rs`, `crates/msc-application/tests/modpack_inspection.rs`
**What:** Redeem a purpose-bound staged upload, identify `.mrpack`, CurseForge, or supported plain-JAR ZIP, parse and validate its manifest, report pinned Minecraft/loader versions and manual-file requirements, and extract into an operation-owned staging tree with traversal/symlink/size/count limits. Inspection never mutates a server and cleans up expired or invalid uploads.
**Verify:** `cargo nextest run -p msc-application --test modpack_inspection`
**Commit:** `P8.18: implement safe modpack inspection`
**Batch:** safe

### P8.19 — Import Modrinth packs transactionally
**Status:** not started
**Files:** `crates/msc-application/src/modpacks.rs`, `crates/msc-application/tests/modrinth_pack_import.rs`
**What:** Implement the `.mrpack` pipeline: verify manifest hashes, filter client-only entries, fetch project metadata in bounded chunks, download server files, merge `overrides/` then `server-overrides/`, classify override JARs, preserve permission bits, record pack provenance, and support cancellation/restart recovery. Failure restores the exact prior tree or removes a not-yet-published new server.
**Verify:** `cargo nextest run -p msc-application --test modrinth_pack_import`
**Commit:** `P8.19: import Modrinth packs transactionally`
**Batch:** safe

### P8.20 — Import CurseForge packs and complete blocked files
**Status:** not started
**Files:** `crates/msc-application/src/modpacks.rs`, `crates/msc-application/src/curseforge_manual.rs`, `crates/msc-application/tests/curseforge_pack_import.rs`, `crates/msc-application/tests/curseforge_manual.rs`
**What:** Implement CurseForge file resolution through the secret-backed API key, overrides, classification, provenance, and the exact partial/pending behavior for author-blocked files. Implement Cameron's D-027 choice: for the recommended upload path, bind each pending file to its operation, expected file identity/name, size ceiling, one-use staged-upload purpose, and final hash/JAR validation before resuming. Wrong or missing files leave the operation honestly pending/failed and never publish a half-server.
**Verify:** `cargo nextest run -p msc-application -E 'test(/curseforge_pack_import|curseforge_manual/)'`
**Commit:** `P8.20: import CurseForge packs and blocked files`
**Batch:** stop-after

### P8.21 — Integrate staged add-ons and packs into server creation
**Status:** not started
**Files:** `crates/msc-application/src/provisioning.rs`, `crates/msc-application/src/modpacks.rs`, `crates/msc-application/tests/modpack_server_creation.rs`, `crates/msc-application/tests/provisioning.rs`
**What:** Carry inspected pack metadata into Phase 7 provisioning, select the exact loader build, apply staged JAR/URL/ZIP/pack inputs in source order, and make the whole create one durable cancellable operation. Do not publish the server registry entry until loader provisioning, pack application, initial world setup, and pack provenance all succeed; rollback the directory and staged artifacts on every failure or restart.
**Verify:** `cargo nextest run -p msc-application -E 'test(/modpack_server_creation|provisioning/)'`
**Commit:** `P8.21: create servers from staged modpacks`
**Batch:** safe

### P8.22 — Build portable client export
**Status:** not started
**Files:** `crates/msc-application/src/client_export.rs`, `crates/msc-application/tests/client_export.rs`
**What:** Build export items from live inventory and persisted provider links, exclude managed server-only plugins, apply classification and default-selection rules, return link text for Paper-like servers, and generate a deterministic ZIP for modded servers with Rust archive code. Enforce selection/path bounds and avoid base64 for any new large-payload route; preserve the baseline JSON response only where compatibility requires it and document the size ceiling.
**Verify:** `cargo nextest run -p msc-application --test client_export`
**Commit:** `P8.22: build portable client add-on export`
**Batch:** safe

### P8.23 — Complete component health and add-on repairs
**Status:** not started
**Files:** `crates/msc-application/src/diagnostics.rs`, `crates/msc-application/src/addon_updates.rs`, `crates/msc-application/tests/diagnostics.rs`, `crates/msc-agent/src/routes/health.rs`, `crates/msc-agent/src/routes/components.rs`
**What:** Replace the Phase 7 placeholder component-health portion with real add-on folder/version/dependency findings. Implement `update` and `install` repairs through the same verified operation paths as ordinary add-on mutations, keep Phase 9 components explicitly unavailable, require stopped state where replacement demands it, and remove only the repaired persisted problem after both disk and record writes succeed.
**Verify:** `cargo nextest run -p msc-application --test diagnostics && cargo nextest run -p msc-agent -E 'test(/health|components/)'`
**Commit:** `P8.23: complete add-on diagnostics and repairs`
**Batch:** stop-after

### Public clients

### P8.24 — Wire Phase 8 routes through real services
**Status:** not started
**Files:** `crates/msc-api/src/dto/addons.rs`, `crates/msc-api/src/dto/mod.rs`, `crates/msc-agent/src/routes/components.rs`, `crates/msc-agent/src/routes/mod.rs`, `crates/msc-agent/src/main.rs`, `crates/msc-agent/tests/phase8_routes.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Wire the existing add-on/catalog/components/export routes plus P8.9's staged pack/manual-file additions to the real services with authentication, `addons`/`fleet` permissions, request limits, audit records, operation IDs, typed errors, and capability degradation. Test through the HTTP router with fake providers and disk-backed state across restart. Mark Agent cells Implemented only after the public path passes.
**Verify:** `cargo nextest run -p msc-agent --test phase8_routes`
**Commit:** `P8.24: wire Phase 8 agent routes`
**Batch:** safe

### P8.25 — Add complete Phase 8 CLI commands
**Status:** not started
**Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/tests/cli_phase8.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Add scriptable commands for inventory, catalog search, install from catalog/local file, update one/all, enable/disable/remove, source link management, pack inspect/create/replace/manual-file completion, health repair, and client export. Local inputs upload through staged uploads, long operations poll/cancel using the shared operation commands, JSON output stays machine-readable, and noninteractive use never prompts unexpectedly.
**Verify:** `cargo nextest run -p msc-agent --test cli_phase8`
**Commit:** `P8.25: add Phase 8 CLI commands`
**Batch:** safe

### P8.26 — Repoint and prove the copied iOS add-on workflows
**Status:** not started
**Files:** `clients/ios/MSCRemoteiOS_Swift/ComponentsView.swift`, `clients/ios/MSCRemoteiOS_Swift/CatalogBrowserView.swift`, `clients/ios/MSCRemoteiOS_Swift/HealthView.swift`, `clients/ios/MSCRemoteiOS_Swift/DashboardViewModel.swift`, `clients/ios/MSCRemoteiOS_Swift/RemoteAPIClient.swift`, `clients/ios/MSCRemoteiOS_Swift/RemoteAPIModels.swift`, `clients/ios/MSCRemoteiOSTests/`, `docs/msc2/client-capability-matrix.csv`
**What:** Repoint the existing components/add-ons/catalog/update/remove/export/repair flows to `/v1`, generated/frozen DTO shapes, typed errors, and operation polling. Add document-picker upload for modpack/local-JAR/manual CurseForge completion if the existing copied UI exposes those workflows; otherwise record the still-Planned screen without claiming parity. Preserve per-host state and show pack-managed/provider-unavailable explanations plainly.
**Verify:** `xcodebuild -project clients/ios/MSCRemoteiOS.xcodeproj -scheme MSCRemoteiOS -destination 'platform=iOS Simulator,name=iPhone 16 Pro' test`
**Commit:** `P8.26: repoint iOS add-on workflows`
**Batch:** safe

### Proof and gate

### P8.27 — Build one portable Phase 8 public-path smoke
**Status:** not started
**Files:** `tools/phase8/phase8-gate-smoke.sh`, `tools/phase8/fake-provider-server.py`, `tools/phase8/fixtures/`, `.github/workflows/ci.yml`
**What:** Build one synthetic smoke, not a collection of overlapping scripts. Through ordinary CLI/HTTP paths and local fake providers, create a modded server from a tiny pack, search/install/update/disable/enable/remove an add-on, resolve dependencies and a cycle, refuse a corrupt hash and hostile archive, resume a manual CurseForge file, enforce pack-managed refusal, perform client export, complete health repairs, cancel/restart an interrupted pack operation, and prove no staging/orphan residue. Add this same smoke as macOS/Linux/Windows CI legs.
**Verify:** `bash tools/phase8/phase8-gate-smoke.sh --synthetic`
**Commit:** `P8.27: build the Phase 8 gate smoke`
**Batch:** solo

### P8.28 — Exercise real providers and real packs
**Status:** not started
**Files:** `docs/msc2/addons/provider-evidence/`, `docs/msc2/addons/modpack-evidence/`, `docs/msc2/addons/phase8-scope.md`
**What:** With all provider overrides absent, use the ordinary CLI to search Modrinth, resolve one Hangar-backed plugin source, inspect/import the recorded `.mrpack` and CurseForge pack, complete one author-blocked file if the evidence contains one, reach a real server ready line, export its client package, and stop it. Record exact provider URLs, versions, checksums, operation outcomes, pack file disposition counts, and any unavailable evidence. This is the phase's only live-network verification step.
**Verify:** `python3 tools/phase8/phase8-check.py --evidence docs/msc2/addons/provider-evidence --modpack-evidence docs/msc2/addons/modpack-evidence`
**Commit:** `P8.28: record real Phase 8 evidence`
**Batch:** stop-after

### P8.29 — Prove the exact candidate on all three platforms
**Status:** not started
**Files:** `.github/workflows/ci.yml`, `docs/msc2/addons/phase8-scope.md`
**What:** Push the exact candidate containing P8.27/P8.28, require its own GitHub Actions run—not an earlier run—to pass repo invariants plus macOS, Linux, and Windows Phase 8 smoke legs and the headless no-GUI check, and record the run/candidate in the scope evidence. Fix nothing in this step; if CI fails, stop and plan a correction step for the actual failure.
**Verify:** `gh run view "$(gh run list --commit "$(git rev-parse HEAD)" --limit 1 --json databaseId --jq '.[0].databaseId')" --json conclusion,jobs` → `conclusion` is `success`, with green macOS, Linux, and Windows Phase 8 smoke jobs for this exact `HEAD`
**Commit:** `P8.29: prove Phase 8 across platforms`
**Batch:** solo

### P8.30 — Close the Phase 8 exit gate
**Status:** not started
**Files:** `tools/phase8/phase8-gate-smoke.sh`, `tools/phase8/phase8-check.py`, `docs/msc2/addons/phase8-scope.md`, `docs/msc2/client-capability-matrix.csv`, `docs/msc2/rolling-plan.md`
**What:** Check the literal port-plan gate and every working exit criterion against one exact candidate: provider parsing, dependency resolution, client-only precedence, pack guards, transactional imports/updates, D-027 behavior, client export, public API/CLI/iOS paths, real evidence, cancellation/restart recovery, and tri-platform CI. Run the full workspace suite once here. Report gaps honestly; do not mark the phase complete or pre-empt the other agent's REVIEW.
**Verify:** `python3 tools/phase8/phase8-check.py --gate && bash tools/phase8/phase8-gate-smoke.sh --synthetic && cargo nextest run --workspace`
**Commit:** `P8.30: close the Phase 8 gate`
**Batch:** solo

---

## Amendments log

Every amendment from Phase 8 onward is recorded here. Earlier phases' amendments are in `rolling-plan-archive.md`.
