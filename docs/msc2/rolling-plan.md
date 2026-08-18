# MSC 2 — Rolling Plan

> ## STATUS: Phase 6 is closed. Phase 7 is planned (30 steps, P7.1–P7.30). QUESTION 1 is answered. P7.1 through P7.9 are DONE. P7.10 through P7.13 are built and awaiting Cameron's verification.
> **Next move:** Verify — Cameron runs P7.10's through P7.13's `Verify:` commands and, if he's satisfied, moves their Status to DONE. Unlike P7.5 through P7.9 (each executed before the prior steps were marked verified, on Cameron's direct instruction each time — recorded in their own Actual results), P7.10 was only started after P7.1 through P7.9 were all already DONE. P7.13 is `stop-after` — it's the first place MSC 2 makes an outbound network request, and both this rolling-plan's own batch-range note and Cameron's own EXECUTE instruction stopped the batch there on purpose. The next EXECUTE should not start P7.14 until P7.10 through P7.13 are verified.
> **Repo:** https://github.com/ctemple9/msc2 · `main` is fast-forwarded to `8568dea` (`P6.51: validate backup archives in one process`), the exact commit Codex's review checked (GitHub Actions run [32068857631](https://github.com/ctemple9/msc2/actions/runs/32068857631), fully green). Review detail: `rolling-plan-archive.md`'s Amendments log, "2026-08-18 — Codex Phase 6 review: gate holds."
> **Last updated:** 2026-08-18

**Previous phases (Setup, Phase 0 through Phase 6) and their amendments log have moved to `rolling-plan-archive.md`** to keep this file small. That archive is historical only — current status, active work, and every amendment from Phase 7 onward stay here.

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
| **7** | Server families and provisioning | **planned — P7.1–P7.30 written, none started** |
| 8 | Mods, plugins, modpacks | not started |
| 9 | Networking and helpers | not started |
| 10 | Bedrock runtimes | not started |
| 11 | Desktop and web clients | not started |
| 12 | Terminal UI (deferred from v1) | not started |

---

## Phase 6 — Worlds and backups

All 51 steps (P6.1–P6.51) are `DONE`, and Codex's gate review (2026-08-18) confirms the gate holds on exact candidate `8568dea`. The full record — scope, characterization, world/backup services, public-client wiring, gate-review corrections, exact-commit tri-platform CI proof, and the review itself — has moved to `rolling-plan-archive.md`.

---

## Phase 7 — Server families and provisioning

**Gate** (`msc2-port-plan.md` §3): "Vanilla, Paper, Purpur, Fabric, NeoForge, Forge. Runtime selection, installer flows, archive behavior, startup diagnostics. Scope bounded by the 1.20 floor (D-014)." Phase 7 must also satisfy the port plan's own later-audit clause: "after Phase 5's broad raw import, Phase 7 must prove non-Paper Java servers are not merely classified but actually launchable with the correct family-specific startup shape."

**Working exit criteria:** a new Java server of each of the six named families can be created through the frozen API, the CLI, and the copied iOS client, and each one lands with the correct family-specific launch shape — `-jar <jar> --nogui` for Vanilla/Paper/Purpur/Fabric, `@<args-file> nogui` for Forge/NeoForge — plus its `eula.txt`, `server.properties`, add-on folder, initial world slot, and recorded Minecraft/build/loader versions; Forge and NeoForge really run their installer as a supervised subprocess and launch from the file that installer generated; every failed create rolls its directory back completely, leaving no half-provisioned server behind; version listing, version change, and jar archiving go through Phase 3's staged-download path with size and checksum verification rather than writing into a live server directory; Java runtime discovery, selection, and the required-major guard gate both creation and start, and report an unusable runtime instead of failing at launch; startup diagnostics turn a real failed boot into attributed problems with repairs that MSC verifies before claiming success; provider outages, malformed catalogs, and absent networks degrade honestly instead of fabricating a version list; a Phase 5-imported non-Paper server (already classified by `fixtures/raw-server-import/`) actually starts; and macOS, Linux, and Windows CI pass on the same committed synthetic smoke. Bedrock creation is refused with an advertised `capability_unavailable` until Phase 10, not faked.

**Source oracle:** MSC 1 at `~/Documents/Swift Projects/minecraft-server-controller`, read-only. Primary files: `ServerJarProviders.swift` (the six families' catalogs and downloads, plus `PufferfishDownloader`), `PaperDownloader.swift` (Paper fill v3 API, stable-ceiling walk, build selection), `NeoForgeInstaller.swift` (both `NeoForgeInstaller` and `ForgeInstaller`, including the shared subprocess helper and `findArgsFile`), `AppViewModel+ServerCreation.swift` (`createNewServer`, rollback, `archiveServerJar`, cross-play template copy), `AppViewModel+Templates.swift` + `AppViewModel+PaperTemplateDownload.swift` (the jar archive/template store), `AppViewModel+ComponentsVersions.swift` (version change, `upgradeModdedLoader`, `recordLoaderVersion`), `JavaRuntimeManager.swift` + `JavaInstaller.swift` + `PrerequisitesView.swift` (runtime detection, normalization, install options), `JavaServerLaunchHelper.swift` + `ServerProcessManager.swift` (launch shape), `StartupCrashAnalyzer.swift` + `StartupProblemsSheet.swift` + `AppViewModel+HealthCards.swift` (`checkLastStartup`, `writeLastStartupResult`, `checkJavaRuntime`, `checkDirectory`, `checkRAMAllocation`), `EULAManager.swift`, `ComponentVersionParsing.swift`, `PaperVersionSidecar.swift`, `RemoteAPIServer+ComponentRoutes.swift` and `AppViewModel+APIWiringServerMgmt.swift` (the wire behavior of every route below), and the copied iOS `ServerVersionView.swift`/`HealthView.swift`/`DashboardView.swift`/`RemoteAPIClient.swift`.

**Routes this phase makes real.** All of them are already frozen in `docs/msc2/api-contract/openapi.json` (Phase 2, plus P6.8); Phase 7 adds no route except the one named in "Questions before P7.1". Every one currently reads `Planned` for Agent in `docs/msc2/client-capability-matrix.csv`:

`POST /v1/servers/create` · `POST /v1/servers/delete` · `POST /v1/servers/rename` · `POST /v1/servers/eula` · `GET /v1/versions` · `GET /v1/versions/create` · `POST /v1/components/version` · `GET /v1/templates` · `POST /v1/templates` · `GET /v1/java-runtimes` · `GET /v1/config/java-runtime` · `POST /v1/config/java-runtime` · `GET /v1/config/ram` · `POST /v1/config/ram` · `GET /v1/health/problems` · `POST /v1/health/repair` · and the real replacement for `GET /v1/health`'s Phase 2 placeholder card.

30 steps, seven groups:

| Group | Steps | Deliverable |
|---|---|---|
| Scope and evidence | P7.1–P7.3 | confirmed family boundary, self-tested provider-corpus checker, real recorded catalogs and installer evidence |
| Characterization and contract | P7.4–P7.9 | catalog/download, installer/launch-shape, creation/archive, runtime, and diagnostics fixtures; the reconciled Phase 7 contract and capability rows |
| Pure domain | P7.10–P7.12 | version entries and comparison, family launch shape, creation and runtime-selection policy |
| Infrastructure | P7.13–P7.16 | jar-provider boundary, loader-installer runner, template/archive store, Java runtime discovery and install |
| Application services | P7.17–P7.22 | download-and-go creation, install-step creation as an operation, version change, fleet CRUD, templates, startup diagnostics |
| Public clients | P7.23–P7.26 | routes, CLI, copied iOS |
| Proof and gate | P7.27–P7.30 | portable six-family smoke, real provisioning evidence, tri-platform CI, literal gate check |

**Planned batch ranges:** after the preceding solo step is verified, `P7.10–P7.12`, `P7.15–P7.16`, `P7.17–P7.18`, `P7.19–P7.22`, and `P7.23–P7.26` may each run as one BATCH EXECUTE conversation. P7.13 and P7.14 are each `stop-after` and start no range — they build the two boundaries where MSC 2 first touches the network and first runs a third-party installer, and both want looking at before anything is stacked on them. Every `stop-after` step ends its range. No batch crosses a failed Verify.

**Fixture counts in the Verify lines are planned targets, not measurements.** A characterization step that finds the oracle yields a different number of genuine cases records the real count and the reason in its own "Actual result", and amends its Verify in the same commit. Inventing filler cases to hit a planned number is the failure this note exists to prevent.

**Not in this phase**, deferred on purpose:

- **Bedrock creation and Bedrock versions** stay Phase 10. `POST /v1/servers/create` with `serverType: "bedrock"` returns P6.8's `capability_unavailable` error rather than half-provisioning something no runtime can start. `BedrockProvisioner.swift`, `BedrockVersionFetcher.swift`, and `updateBedrockVMFiles`/`updateBedrockImageAndRestart` are untouched.
- **Add-ons, modpacks, and the rest of `/v1/components`** stay Phase 8. Phase 7 claims exactly one components route — `POST /v1/components/version`, which changes the *server JAR*, not an add-on — because that is the same download/verify/archive/replace machinery provisioning already builds. `GET /v1/components`, `/components/install`, `/components/remove`, `/components/update`, `/components/client-export`, `/catalog/search`, and the wizard's staged add-ons (`applyStagedAddOn`) are Phase 8. `stagedAddOns` has no field in the frozen `ServerCreateRequestDTO`, so nothing in the contract is left dangling by this.
- **Geyser, Floodgate, Playit, and Xbox Broadcast** stay Phase 9. `enableCrossPlay`, `enablePlayit`, and `enableXboxBroadcast` on the create request are honoured only as far as MSC 1 honours them at creation time: the flags are recorded in the server's config, and `applyCrossPlayTemplatesIfAvailable` copies Geyser/Floodgate jars **that already exist in the local template directory**. Phase 7 never downloads a helper. `downloadLatestGeyserTemplate`/`downloadLatestFloodgateTemplate` are Phase 9.
- **The other health cards.** Phase 7 replaces `GET /v1/health`'s Phase 2 canned `demo-card` with the real cards it owns — server directory, Java runtime, RAM allocation, last startup — and reports the rest (port reachability, component jars, Bedrock world data, VM runtime) as an explicit not-yet-implemented note rather than a fabricated `ok`. Those cards land with their own phases (9, 8, 10).
- **Serving help content.** Phase 7 populates `helpId` on the health cards and startup problems it creates, per D-026, but `GET /v1/help/{helpId}` itself stays Phase 11 as the port plan says. A populated pointer with no resolver yet is the intended interim state, not a gap.
- **Spigot, Quilt, and Pufferfish.** MSC 1 carries flavor entries for all three and a working `PufferfishDownloader`, but `isAvailableInCreateFlow` excludes all three, so MSC 1 itself never provisions them. Phase 7 preserves that exactly: all nine flavors stay classifiable on import and launchable if imported, and the create-flow catalog offers the six the port plan names. Spigot's BuildTools compile is not built.
- **Desktop/web screens** stay Phase 11. Their cells are `Planned` in the capability matrix; that is not an exception. The CLI and the copied iOS client are Phase 7's client surfaces.
- **Modpack-driven creation** (`.mrpack`/CurseForge server packs as a create source) stays Phase 8, along with D-027's open manual-download question.

### Questions before P7.1

One question needs Cameron's answer before P7.1 is written, because it changes the size of P7.16 and decides whether Phase 7 adds a route.

```
QUESTION 1 — Should MSC 2 install Java itself, or just tell you what to install?

What it is:      Minecraft needs a specific Java version — 1.20-1.20.4 wants Java 17,
                 1.21+ wants Java 21. MSC 1 handles a missing one with a macOS sheet:
                 it downloads Adoptium's .pkg installer and asks you to double-click it.
                 That is a graphical, macOS-only, same-machine flow. MSC 2's agent may
                 be a Debian box in a closet with no browser and nobody logged in.

The choice:      (a) The agent installs Java itself — downloads Adoptium's plain archive
                     for its own OS/architecture, verifies the checksum, unpacks it into
                     MSC's own data directory, and uses it. Needs one new API route
                     (POST /v1/java-runtimes/install, returning an operation id), which
                     is an additive superset addition under D-006, the same shape as the
                     thirteen operations P6.8 added.
                 (b) The agent only detects and explains — it reports which Java versions
                     it found, which one this server needs, and a link plus instructions,
                     and you install it yourself on the host.

Why it matters:  msc2-product.md promises both "installing the correct version of Java"
                 during setup and the "[Install Java 21]" button in its own worked
                 example. Option (b) makes both of those untrue on exactly the deployment
                 MSC 2 exists for. Option (a) is roughly one extra step's worth of work
                 (P7.16 grows) and one new route.

If unsure:       (a). The product document already promises it, the download/verify/stage
                 substrate exists from Phase 3, and MSC-owned runtimes also remove a whole
                 class of "which Java is on PATH today" problems. (b) would need
                 msc2-product.md amended to stop promising it, which is a bigger change
                 than building it.
```

**Decided without asking** (recorded here so the reasoning is visible, per `CLAUDE.md`):

- **The 1.20 floor filters the offered catalogs, not imports.** D-014 says older versions are "not carried in provisioning logic". `GET /v1/versions/create` and `GET /v1/versions` therefore drop entries below Minecraft 1.20; a below-floor server that is imported still lists, starts, and runs. This is a deliberate divergence from MSC 1, which filters nothing.
- **Provisioning tests never touch the network.** Every catalog and download in the test suite is served by a fake provider fed from `corpus/providers/`, and both loader installers are exercised against a locally built fake installer jar — the same technique `tools/phase6/phase6-gate-smoke.sh` already uses for its fake Paper server, which is why CI installs a JDK. Real network provisioning is proved once, by hand, in P7.28.
- **Bedrock create is refused, not stubbed.** Per P6.8's precedent, an advertised `capability_unavailable` beats a server directory no runtime can start.

---

### Scope and evidence

### P7.1 — Scope Phase 7 and settle the family and runtime boundary
**Status:** DONE
**Files:** `docs/msc2/families/phase7-scope.md`, `docs/msc2/msc2-decisions.md`
**What:** Read MSC 1's six provisioning paths (`ServerJarProviders`, `PaperDownloader`, `NeoForgeInstaller`/`ForgeInstaller`, `createNewServer`) beside the frozen contract and Phase 5's raw-import classifier, then write the authoritative Phase 7 boundary as a design record — no Rust. Fix, per family: catalog source, version-entry identity, download-and-go vs install-step, launch shape, what `ConfigServer` fields the create must end up with, and what a failed create must leave behind (nothing). Record the 1.20 filter rule, the Bedrock refusal, the Spigot/Quilt/Pufferfish carry-forward, the cross-play template copy-but-never-download rule, and every symbol-ledger row this phase owns (`server-creation`, `java-runtime`, `templates`, `startup-diagnostics`, `components-versions`, `component-version`, `server-installation`, `setup`, `prerequisites`). Record Cameron's answer to QUESTION 1 as a dated addendum to D-006 (additive route) or, if he chooses (b), as a flagged conflict with `msc2-product.md` for him to resolve. Record the working gate above.
**Actual result:** Cameron answered QUESTION 1 — (a), MSC 2 installs Java itself — recorded as a dated addendum to D-006 in `msc2-decisions.md` and expanded in `phase7-scope.md`. Wrote `docs/msc2/families/phase7-scope.md`: per-family catalog/identity/provisioning-kind/launch-shape table for all six create-flow families; a sourced correction to this rolling-plan's own P7.6 wording (`archiveServerJar` does not archive NeoForge/Forge "via their own installer path" — it simply never archives them; no such path exists in source); `createNewServer` decomposed in source order with the two-path rollback guarantee and an unflagged world-source-failure gap noted for P7.17/P7.18 to decide; the 1.20 filter, Bedrock refusal, and cross-play copy rules pinned precisely; a per-flavor (not per-bucket) accounting of Pufferfish/Spigot/Quilt showing they differ more than "excluded from create flow" implies (Pufferfish has a working latest-only downloader; Spigot has no installer implementation at all; Quilt has no provider of any kind but still launches from an on-disk jar); and the 46-row symbol-ledger table for this phase's nine target domains, with `createNewBedrockServer` and `applyStagedAddOn` explicitly rescheduled (Phase 10, Phase 8) rather than silently dropped.
**Verify:** `python3 -c "from pathlib import Path; s=Path('docs/msc2/families/phase7-scope.md').read_text(); required=['vanilla','paper','purpur','fabric','neoforge','forge','install-step','download-and-go','args file','1.20','capability_unavailable','rollback','pufferfish']; missing=[x for x in required if x.lower() not in s.lower()]; assert not missing, missing; print('OK')"`
**Commit:** `P7.1: scope Phase 7 server families and provisioning`
**Batch:** solo

### P7.2 — Build the Phase 7 provider corpus and gate checker first
**Status:** DONE
**Files:** `tools/phase7/provider-corpus-check.py`, `tools/phase7/fixtures/`, `corpus/providers/README.md`
**What:** Build a dependency-free checker before any evidence is collected, so the bar is set before it can be bent to fit what turned up. Inventory mode requires, for every recorded provider response: source URL, capture date, SHA-256, byte size, and which family it belongs to; it fails on a missing provenance field, a duplicate hash, malformed JSON/XML, or a response mutated after recording. Coverage mode takes a fixture directory and asserts every one of the six families is represented and that no fixture cites a recorded response that is absent from the corpus. Passing and deliberately failing self-tests prove each rejection fires. No network access anywhere in this tool.
**Actual result:** Built `tools/phase7/provider-corpus-check.py` (stdlib only, same shape as `tools/phase6/corpus-check.py`). Inventory mode requires a `manifest.json` entry per evidence file with `family` (must be one of `vanilla`/`paper`/`purpur`/`fabric`/`neoforge`/`forge` — an unknown family fails loudly too, since coverage mode's family count depends on every recorded response being attributed correctly), `source_url`, `captured`, `sha256`, `byte_size`; rejects a missing manifest entry or field, a duplicate SHA-256, a `.json`/`.xml` file that doesn't parse, and a recomputed SHA-256 that doesn't match what was recorded. Coverage mode reads an optional `corpus_source` field (a list of paths into the provider corpus) that a fixture may carry — additive to `fixture-format.md`'s existing six fields, nothing there needed to change — and fails on a citation with no corpus manifest entry or a family with zero citations across the fixture directory. Ten self-test cases (7 inventory, 3 coverage) under `tools/phase7/fixtures/` prove every rejection fires and the passing case doesn't; `corpus/providers/README.md` documents the schema, both modes, and the `<family>/<name>.<ext>` directory convention for P7.3. `corpus/providers/` itself is still empty — deliberately; P7.3 populates it.
**Verify:** `python3 tools/phase7/provider-corpus-check.py --selftest`
**Commit:** `P7.2: build the Phase 7 provider corpus checker`
**Batch:** solo

### P7.3 — Record real provider catalogs and installer evidence
**Status:** DONE
**Files:** `corpus/providers/`, `corpus/providers/README.md`, `corpus/providers/manifest.json`
**What:** Capture one real response from each live catalog MSC 1 uses — PaperMC fill v3 (projects, versions, builds), Purpur, Mojang's version manifest plus one version JSON, Fabric meta (game, loader, installer), the NeoForge maven listing, and the Forge `maven-metadata.xml` — plus the on-disk shape a real Forge and a real NeoForge installer leaves behind (the args file's name and its `@`-file contents, the `libraries/` layout, the run scripts). Record provenance, capture date, byte size, and SHA-256 for each. Keep responses small: truncate long version arrays to a documented, representative slice rather than committing megabytes, and say in the manifest exactly what was truncated. If a provider is unreachable or has changed shape since MSC 1 was written, record that as a finding and stop rather than hand-writing a plausible response — a fabricated catalog would make every downstream fixture worthless.
**Actual result:** All six live catalogs reached and captured 2026-08-18, plus Forge's `promotions_slim.json` (not named in this step's file list, but read by the oracle's `latestRecommendedVersion()`, so captured alongside `maven-metadata.xml` rather than left for P7.4 to discover missing). 23 evidence files, all six families represented. Large responses truncated to documented representative slices per-file in `manifest.json`'s `note` field (Paper builds 92→7, Mojang manifest 907→11, Mojang per-version 131 libraries→3, Fabric game 67→12, Fabric loader 251→3, Fabric installer 67→8, NeoForge versions 1662→7, Forge versions 5040→9); small responses (Paper/Purpur project info, Purpur per-version, Forge promotions) kept whole. Real Forge (`1.20.1-47.4.5`) and NeoForge (`20.4.237`) installers were downloaded and actually run (`--installServer`) in a scratch directory outside the repo; `run.sh`/`run.bat`/`user_jvm_args.txt`/the `@`-args files are committed verbatim under each family's `installer-evidence/`, and the `libraries/` trees they produced (104 files/161 MB Forge, 115 files/171 MB NeoForge) are captured only as a `size relative/path` shape listing, not the jars themselves, per this directory's `README.md`. Four findings recorded in `corpus/providers/README.md`: Forge's `maven-metadata.xml` `<latest>`/`<release>` tag is stale relative to its own `<versions>` array (explains why the oracle prefers `promotions_slim.json`); NeoForge's Maven briefly 404'd behind a stale CDN negative-cache entry (confirmed not a real outage or shape change, retried successfully, did not trigger the stop clause); Minecraft's real versioning has moved from `1.x` to a `YY.n` scheme (current release `26.2`) which the oracle's `compareMCVersions` already special-cases, so P7.4/P7.10 characterize it rather than treat it as a break; and the real Forge/NeoForge installers produce a byte-identical `user_jvm_args.txt`, recorded once under `forge/` rather than twice to avoid a false duplicate-hash failure.
**Verify:** `python3 tools/phase7/provider-corpus-check.py --selftest && python3 tools/phase7/provider-corpus-check.py --inventory --providers corpus/providers`
**Commit:** `P7.3: record real provider catalogs and installer evidence`
**Batch:** stop-after

---

### Characterization and contract

### P7.4 — Characterize the six families' version catalogs and jar downloads
**Status:** DONE
**Files:** `fixtures/server-jar-providers/`, `fixtures/server-jar-providers/samples/`
**What:** Characterize, against P7.3's recorded responses: Paper's fill v3 walk (all-versions sort, stable-ceiling search, the 20-candidate cap, `server:default` download selection, `STABLE`/`BETA`/`ALPHA` channel filtering, build-date formatting), Purpur's and Vanilla's listing and download, Fabric's three-part loader/installer/game resolution and its `firstStableVersion` fallback, NeoForge's `listVersionPairs` and `minecraftVersion(forNeoForge:)` derivation, and Forge's `parseMavenMetadata`/`parseMavenVersion` XML parse plus `latestRecommendedVersion`. Include the `ServerVersionEntry` identity and `isLatest`/`isStable` rules the frozen `VersionEntryDTO` mirrors, the numeric dotted-version comparisons each provider does by hand (they differ — do not unify them silently), and the 1.20 floor filter as a Phase 7 addition marked as such. Cover failure shapes too: HTTP error, empty version list, malformed JSON, malformed XML, and a build entry missing its download URL.
**Actual result:** 26 fixtures written to `fixtures/server-jar-providers/`, all citing P7.3's real recorded evidence via `corpus_source` except where the behavior genuinely isn't in the corpus (the 20-candidate-cap loop is a pure algorithm property; Pufferfish's dispatch shape and the five failure shapes are read from source, not a live response). All six families cited at least once (19 citations total). Two small hand-crafted samples live under `fixtures/server-jar-providers/samples/` (a stable-entries-removed slice of the real Paper builds response, and a synthetic no-stable-loader response) — both excluded from the 26-file count since `--validate-dir`'s glob isn't recursive. Every numeric expected value (sort orders, best-build selection, NeoForge/Forge version-pair derivation, Purpur's Paper-alignment target version, Forge's stale-metadata-vs-promotions finding) was recomputed independently in Python against the real corpus bytes before being written into a fixture, not hand-derived from reading the Swift alone. Two source-reading findings worth flagging: (1) `PaperDownloader.swift`'s `fetchAvailableVersions`/`fetchBestBuild`/`fetchAllVersionsSorted` (the 20-cap, stable-ceiling walk used by `downloadLatestPaper`/`fetchLatestMetadata`) is a *different* function from `ServerJarProviders.swift`'s `PaperDownloader.listVersions()` extension (the uncapped picker walk used by the create-flow version list) — both are characterized, under different case names, since P7.4's "What" names pieces of both. (2) NeoForge/Forge's `maven-metadata.xml` is never parsed by a real XML parser at all — both `listVersionPairs`/`parseMavenMetadata` hand-scrape `<version>` substrings — so genuinely malformed/non-XML input doesn't throw at that layer, it silently yields an empty list; the throw only happens one level up, in `latestStableVersion`'s empty-after-filter guard. Recorded as its own fixture (`malformed-xml-metadata-silently-yields-empty-list-not-an-error`) since it's the one shape here that isn't what a reader would guess.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/server-jar-providers --expect 26 && python3 tools/phase7/provider-corpus-check.py --coverage fixtures/server-jar-providers --providers corpus/providers`
**Commit:** `P7.4: characterize server jar catalogs and downloads`
**Batch:** solo

### P7.5 — Characterize the loader installers and the family launch shape
**Status:** DONE
**Files:** `fixtures/loader-installers/`, `fixtures/args-file-resolution/`, `fixtures/headless-script/`
**What:** Characterize `NeoForgeInstaller.install` and `ForgeInstaller.install` end to end: installer URL construction, download into the server directory, the `java -jar <installer> --installServer` invocation and its working directory, streamed stdout/stderr, non-zero exit handling, what is cleaned up afterwards, and what the version-resolution path does when no specific version is requested. Then pin the launch shape that follows: `@<args-file> nogui` for Forge/NeoForge against `-jar <jar> --nogui` for the rest, the missing-args-file failure, and the `paper.jar` fallback when `paperJarPath` is empty. `fixtures/args-file-resolution/` (12 cases) and `fixtures/headless-script/` (19 cases) already exist from earlier phases and are **reused, not rewritten** — extend them only where a real gap shows up, and say in the step's Actual result which existing cases now carry Phase 7 weight.
**Actual result:** 16 fixtures written to `fixtures/loader-installers/`, all citing `NeoForgeInstaller.swift` (the file holding both `NeoForgeInstaller` and `ForgeInstaller` — there is no separate `ForgeInstaller.swift`). All seven "What" dimensions covered, one fixture pair (Forge/NeoForge) per dimension except invocation, which is one shared private function (`runJavaInstaller`, line 261) used identically by both, so its two argv-shape cases (`shared-installer-invocation-absolute-java-path-argv`, `shared-installer-invocation-bare-java-command-via-env`) aren't split per family. Real corpus evidence cited via `corpus_source` for the URL-construction and version-resolution cases (`forge/installer-evidence/`, `neoforge/installer-evidence/`, `forge/promotions-slim.json`, `neoforge/maven-metadata.xml`); the version-resolution expected values (Forge → mc `26.2`/forge `65.1.0`, NeoForge → `26.2.0.61`) were recomputed independently in Python against the real, full corpus files before being written into the fixtures, not hand-derived from reading the Swift alone. Three findings worth flagging: (1) on a non-zero installer exit *or* a missing post-install args file, neither installer's cleanup (`try? removeItem`) ever runs — a failed or incomplete install leaves the downloaded installer jar (and, for NeoForge, `installer.log`) sitting in the server directory with nothing removing it; (2) `ForgeInstaller.install`'s success path only removes its installer jar, while `NeoForgeInstaller.install`'s removes both the jar and `installer.log` — a genuine asymmetry between the two, not something to unify in the Rust port; (3) `process.standardOutput` and `process.standardError` are wired to the *same* `Pipe`, so installer stdout and stderr interleave into one `onLog` stream with no way to tell them apart downstream. `fixtures/args-file-resolution/` (12 cases) and `fixtures/headless-script/` (19 cases) needed no new cases — every one of P7.5's launch-shape claims (`@<args-file> nogui` vs `-jar <jar> --nogui`, the missing-args-file failure, Forge's configured-pair-vs-fallback scan, NeoForge's configured-version-vs-fallback scan) is already exercised by an existing case, and all 12 + 19 now carry Phase 7 weight as characterizations of the frozen `JavaServerLaunchConfig`/`HeadlessScriptGenerator` shape rather than orphaned earlier-phase tests. One gap found but *not* fixed here, since fixing it would mean changing the pinned `--expect 19` count this step's own Verify line commits to: no existing `headless-script` (or `args-file-resolution`) case exercises `paperJarPath` empty → `jarName` falls back to `"paper.jar"` (`JavaServerLaunchHelper.resolve`, line 70-77) — MSC 1 itself has no test for this either. Recorded in the rolling-plan's own notes below rather than silently added.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/loader-installers --expect 16 && python3 tools/fixture-runner/run.py --validate-dir fixtures/args-file-resolution --expect 12 && python3 tools/fixture-runner/run.py --validate-dir fixtures/headless-script --expect 19`
**Commit:** `P7.5: characterize loader installers and launch shape`
**Batch:** solo

### P7.6 — Characterize server creation, rollback, and the jar archive
**Status:** DONE
**Files:** `fixtures/server-creation/`, `fixtures/jar-templates/`
**What:** Characterize `createNewServer` step by step in source order: name trim and empty refusal, `servers_root/java/<name-lowercased-underscored>` folder derivation, the pre-existing-folder refusal, the install-step branch against the download-and-go branch, Paper's archive-first shortcut (metadata check, archived filename match, sidecar write), `eula.txt` written as `eula=false`, the exact `server.properties` key set and its imported-metadata overrides, the add-on folder per `addOnKind` (`plugins/`, `mods/`, none for Vanilla), the cross-play template copy, the three `WorldSource` branches, the `ConfigServer` field set including the modded 3/6 GB RAM default, the initial-slot failure path that deletes the whole directory, `recordLoaderVersion`, and the `catch` that removes `newDir` on any throw. Then characterize the archive/template store: `archiveServerJar`'s naming, `latestTemplate(in:prefixLowercased:)`, `jarSummary`, template listing and sort order, export-as-template, and create-from-template. Mark as a deliberate Phase 7 strengthening — not oracle parity — any place MSC 1 leaves partial state that this port will roll back instead.
**Actual result:** This step ran ahead of P7.4/P7.5's own verification — this rolling-plan's status line explicitly said not to start P7.6 until at least those two were verified, and Cameron gave a direct instruction in the EXECUTE conversation to run it anyway; noted here rather than silently followed, the same pattern P7.5 used for running ahead of P7.4. 24 fixtures written to `fixtures/server-creation/` covering every clause of the "What" line in `createNewServer`'s source order (name trim/refusal, folder derivation, pre-existing-folder refusal, install-step vs download-and-go branch, Paper archive-first hit/miss/gated-off, eula.txt, the exact server.properties key set, imported-metadata overrides, all three addOnKind cases, cross-play copy applied/skipped, both WorldSource copy-failure paths, the ConfigServer field set, the 2/4 vs 3/6 GB RAM default, initial-slot failure cleanup, recordLoaderVersion's three-part guard, and the top-level catch cleanup). 10 fixtures written to `fixtures/jar-templates/` covering `archiveServerJar`'s per-flavor naming (Paper's Int-parsed build, Purpur/Vanilla/Fabric's patterns, the unsupported-flavor no-op, the already-archived no-op), `latestTemplate`, `jarSummary`, template-listing sort order, and the remote-API `exportServer`/`createServer` actions (`AppViewModel+APIWiringServerMgmt.swift`) as the export-as-template/create-from-template pair, since MSC 1 has no dedicated `exportAsTemplate`/`createFromTemplate` function — those two remote-API actions are the actual implementation. All numeric/behavioral claims were read directly from source with file:line citations, not inferred. Three findings worth flagging: (1) a genuine wording gap in this step's own "What" line — it names "the running-server refusal" for export-as-template, but the `exportServer` case in `templateMutationProvider` (line 339-386) has no `isServerRunning` guard anywhere in it (unlike `applyPaperTemplateToSelectedServer`, which does); recorded as a fixture note and a wording correction rather than characterizing a refusal that doesn't exist, the same kind of correction P7.1 made for `archiveServerJar` and NeoForge/Forge; (2) the two `WorldSource` copy-failure paths (`backupZip`/`existingFolder`, lines 329-334) return `false` with **no** `newDir` cleanup and **no** `lastServerCreateError` set — unlike the initial-world-slot failure (line 356-367) and the top-level `catch` (line 395-401), both of which do both — this is exactly the "MSC 1 leaves partial state" gap this step's own "What" line asked to flag as a Phase 7 strengthening point rather than port as-is, left for P7.17/P7.18 to close; (3) `latestTemplate`'s "latest" pick (`fixtures/jar-templates/latest-template-picks-lexicographically-last-matching-prefix.json`) uses a raw string `<` compare, not a version-aware one, so it can pick a lower Minecraft version's jar over a higher one (e.g. `1.21.4` sorts after `1.21.10`) — a genuine quirk to preserve, not unify with the sort `loadPaperTemplates`/`loadPluginTemplates` use for the on-screen list (`localizedCaseInsensitiveCompare`), which is a different algorithm and can disagree with `latestTemplate` on the same directory.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/server-creation --expect 24 && python3 tools/fixture-runner/run.py --validate-dir fixtures/jar-templates --expect 10`
**Commit:** `P7.6: characterize server creation and the jar archive`
**Batch:** solo

### P7.7 — Characterize Java runtime discovery, selection, and installation
**Status:** DONE
**Files:** `fixtures/java-runtime-selection/`, `fixtures/java-runtime-guards/`
**What:** Characterize the runtime half of provisioning: `detectInstalledJavaRuntimes`' search paths and per-platform candidates, `normalizedJavaExecutablePath`, `parseMajor(fromVersionOutput:)` across real `java -version` banner shapes (Temurin, Zulu, GraalVM, OpenJDK, and a non-Java binary that must be rejected), `validateLooksLikeJava`, `resolvedJavaPath`'s per-server-then-global precedence, `checkJavaOnPath`, `isJavaInstalled`/`hasCriticalMissingDependency`, and `JavaInstaller.minecraftInstallOptions`/`recommendedOption(forMinecraftVersion:)`. Cover the guard that matters at start time: required major against detected major, both directions, including the Java-17-era-with-a-newer-runtime warning. `fixtures/java-runtime-guards/` (15 cases) already exists and is reused. If QUESTION 1 was answered (a), also characterize the managed-runtime install as new Phase 7 behavior rather than an MSC 1 port — Adoptium archive URL per OS/architecture, checksum verification, unpack layout under MSC's own runtimes directory, and what an interrupted install must leave behind.
**Actual result:** This step ran ahead of P7.4–P7.6's own verification — this rolling-plan's status line explicitly said not to start P7.7 until at least those three were verified, and Cameron gave a direct instruction in the EXECUTE conversation to run it anyway; noted here rather than silently followed, the same pattern P7.5/P7.6 used. 18 fixtures written to `fixtures/java-runtime-selection/`, all citing MSC 1 source with file:line except the two managed-install cases, which cite `docs/msc2/families/phase7-scope.md`'s D-006 addendum instead since MSC 1 has no equivalent to port (`JavaInstaller.swift`'s existing `installerURL`/`downloadInstaller` fetch a macOS-only Temurin `.pkg` for a human to double-click, with no checksum step at all). `fixtures/java-runtime-guards/` needed no new cases — its existing 15 already cover `detectInstalledJavaRuntimes`, `normalizedJavaExecutablePath`, the required/detected major mapping, and both directions of the compatibility warning (including the Java-17-era-with-newer-runtime case), so this step's new fixtures cover only what that domain doesn't: `parseMajor` across four vendor banner shapes (4 cases — Temurin and the legacy 1.8-style banner captured live from this machine's real installed JDKs 2026-08-18; Zulu and GraalVM are each vendor's publicly documented banner shape, flagged in their own fixture's notes as not freshly captured, since neither JDK was available locally), `validateLooksLikeJava` (3 cases, including which of its five OR'd substrings independently passes and the first-line-only error text), `checkJavaOnPath` (2), `isJavaInstalled`/`hasCriticalMissingDependency` (2, including the case where the Java check is skipped entirely for a Bedrock-only fleet), `resolvedJavaPath`'s precedence (3 — the create-time override, the create-time fallback to the global default, and Settings' own empty-string-defaults-to-bare-`java` case, which is a different function from the create-flow's `??` fallback and is called out as such), `JavaInstaller`'s option table and `recommendedOption` (2, the second of which flags that `recommendedOption`'s own two `??` fallback branches are unreachable dead code given `requiredJavaMajor`'s real output range), and the managed install (2, covering URL/checksum/no-asset-fallback and the unpack/rollback design respectively). For the managed-install fixtures, real Adoptium API responses were fetched live (`api.adoptium.net/v3/assets/latest/...`) for linux/x64, mac/aarch64, and windows/x64 at real majors (17/21/25) plus a genuine empty-array response for windows/aarch64 at major 17 — not invented — establishing that Adoptium's `binary.package` object already carries a SHA-256 checksum (no separate checksum-file fetch needed) and that asset availability is architecture- *and* major-dependent (Windows/aarch64 has no build for major 17 but does for 21). Two findings worth flagging: (1) `recommendedOption(forMinecraftVersion:)`'s two `??` fallback expressions (`JavaInstaller.swift:54-55`) can never actually fire, since `requiredJavaMajor`'s only possible outputs (8/17/21/25) are exactly the four majors `minecraftInstallOptions` offers — recorded as a fixture note rather than silently exercised as if reachable; (2) MSC 1's own arm64→x64 installer fallback (`JavaInstaller.swift:76-80`) is Mac-specific (Java 8 has no native Apple Silicon build) and was deliberately *not* generalized to Linux/Windows in the managed-install characterization — the real captured windows/aarch64-empty response shows that OS needs its own no-asset handling rather than inheriting Mac's fallback assumption, left for P7.16 to encode precisely.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/java-runtime-selection --expect 18 && python3 tools/fixture-runner/run.py --validate-dir fixtures/java-runtime-guards --expect 15`
**Commit:** `P7.7: characterize Java runtime selection and installation`
**Batch:** solo

### P7.8 — Characterize startup diagnostics, problems, and repairs
**Status:** DONE
**Files:** `fixtures/startup-problems/`, `fixtures/startup-crash-analyzer/`
**What:** Characterize what turns a failed boot into something a person can act on: `writeLastStartupResult`'s record shape and where it is persisted, `checkLastStartup`'s reading of it into a health card (clean, soft-fail, hard-fail, never-started, stale), the `StartupProblem` shape the frozen `StartupProblemDTO` mirrors — `kind`, `kindTitle`, `offenderName`, `requirement`, `installedFile`, `installedJarStem`, `missingDependency`, `rawExcerpt`, `availableActions`, `isRepairing` — and the repair actions themselves (`delete`, `disable`, and the guards that refuse a repair while the server is running). Cover the Phase 7-owned health cards too: `checkDirectory`, `checkJavaRuntime`, `checkRAMAllocation`, and the severity each produces. `fixtures/startup-crash-analyzer/` and `fixtures/connector-crash-analysis/` already exist from Phase 1 and supply the parse side — this step characterizes what the agent does with the parse result, not the parse itself. Assign a `helpId` to every card and problem kind per `docs/msc2/api-contract/helpid-contract.md`, and record which help topics Phase 11 will therefore have to serve.
**Actual result:** This step ran ahead of P7.4–P7.7's own verification — the rolling-plan's status line explicitly said not to start P7.8 until earlier Phase 7 steps were verified, and Cameron gave a direct instruction in the EXECUTE conversation to run it anyway; noted here rather than silently followed, the same pattern P7.5/P7.6/P7.7 used. 38 fixtures written to `fixtures/startup-problems/`, all citing MSC 1 source with file:line (`AppViewModel+HealthCards.swift` for `writeLastStartupResult`/`checkLastStartup`/`checkDirectory`/`checkJavaRuntime`/`checkRAMAllocation`; `AppViewModel+OutputHandling.swift` for `diagnoseUnexpectedStop`/`reopenStartupProblems`/`scanPaperSoftFailures`; `AppViewModel+APIWiringBackupsHealth.swift` for `mapProblem`'s `availableActions`, the `GET /v1/health/problems` provider, and the `POST /v1/health/repair` dispatcher; `AppViewModel+AddonUpdates.swift` for `repairIncompatibleAddon`/`installMissingDependency`'s async-vs-sync split). `fixtures/startup-crash-analyzer/` and `fixtures/connector-crash-analysis/` needed no new cases — every claim here treats `StartupCrashAnalyzer.analyze`'s output as an already-characterized input, per the "What" line's own instruction, the same reuse pattern P7.5 used for `fixtures/args-file-resolution/`/`fixtures/headless-script/`. One wording correction to this step's own "What" line: MSC 1 has no "stale" state for the last-startup card — `checkLastStartup` reads `last_startup_result.json` regardless of its age (no timestamp-vs-now comparison anywhere in the function), so a nine-month-old clean result still reads green forever; recorded as a finding (case 8's notes) rather than fabricated as a fixture that doesn't correspond to real behavior, the same kind of correction P7.1/P7.6 made to earlier wording. `helpId`s are assigned inline in each card/problem fixture's `expected` rather than as separate thin fixtures: `health.directory`, `health.java`, `health.ram`, `health.last-startup` for the four Phase-7-owned cards (component-jar and port-reachability cards stay Phase 8/9 per this phase's own "Not in this phase" list, so they get no `helpId` here), and `diagnostics.crash.<kind-kebab-case>` for each of the five `StartupProblemKind` cases per `helpid-contract.md` §4's `diagnostics.crash.<kind>` namespace — including `diagnostics.crash.duplicate` and `diagnostics.crash.unknown`, even though a source read confirms `StartupCrashAnalyzer` never actually constructs a `.duplicate` or `.unknown` problem anywhere (only `.missingDependency`, `.incompatibleVersion`, and `.loadError` are ever built); both dead-but-declared kinds still get a `helpId` for contract completeness since `StartupProblemsSheet` renders a (permanently empty) UI section for each. Five findings worth flagging beyond the "stale" correction above: (1) `checkJavaRuntime` (the health card) is a wholly separate implementation from the create/launch-time Java runtime selection P7.7/P7.12 characterize — it hardcodes `major >= 21` with no awareness of `server.minecraftVersion`, so a 1.20.4 server correctly running Java 17 shows a yellow "minimum is Java 21" card that is simply wrong for that server (case 14); (2) `checkJavaRuntime` returns on the first candidate that responds at all, even if its version output fails to parse, rather than continuing to the next candidate (case 15); (3) the `POST /v1/health/repair` running-server guard fires before the problem-id is even looked up, so a bogus `problemId` against a running server reports `server_running`, never `problem_not_found` (case 31); (4) "update"/"install" repairs are genuinely asynchronous (they spawn a `Task` hitting the Modrinth API) while "disable"/"delete" mutate state synchronously before the HTTP response is built — the wire response's `updated` snapshot for "update"/"install" still contains the problem being repaired, now flagged `isRepairing: true`, not yet removed (case 38); (5) `diagnoseUnexpectedStop` only calls `writeLastStartupResult` when `isHardFail` is true, so a server that reached ready state and later crashed mid-session shows the generic alert but leaves `last_startup_result.json` — and therefore the Last Startup health card — untouched from the prior clean boot (case 22).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/startup-problems --expect 38`
**Commit:** `P7.8: characterize startup diagnostics and repairs`
**Batch:** solo

### P7.9 — Reconcile the Phase 7 API, operation, and capability surface
**Status:** DONE
**Files:** `docs/msc2/families/phase7-api.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/client-capability-matrix.csv`, `tools/api-contract-check.py`
**What:** Write the route-by-route reconciliation for the seventeen routes listed in this phase's preamble: request/response field meanings against MSC 1's actual handlers, which are synchronous and which return an `operationId`, the operation types provisioning needs (server creation with an install step is minutes long and must survive an agent restart per `operation-model.md`), the exact error codes each 400/404/409/429 maps to, permission categories (already frozen — confirm, do not re-decide), cancellation semantics for a running installer, and the `capability_unavailable` response for Bedrock creation. Add the one new route from QUESTION 1 if the answer was (a), additively, and move `EXPECTED_TOTAL` in `tools/api-contract-check.py` accordingly. Update every Phase 7 row in `client-capability-matrix.csv` to the status each surface will actually reach this phase — no blank cells, no `Intentional exception` without an owner-approved decision entry.
**Actual result:** This step ran ahead of P7.1–P7.8's own verification — this rolling-plan's status line explicitly said the next EXECUTE should not start P7.9 until earlier Phase 7 steps were verified, and Cameron gave a direct instruction in the EXECUTE conversation to run it anyway; noted here rather than silently followed, the same pattern P7.5/P7.6/P7.7/P7.8 used. Wrote `docs/msc2/families/phase7-api.md`, the full route-by-route reconciliation for all eighteen routes (the seventeen frozen baseline routes plus P7.1's committed D-006 addendum route), grounded in MSC 1 source read directly for this step (`RemoteAPIServer+ComponentRoutes.swift`'s handlers, `AppViewModel+APIWiringAddons.swift`'s `changeVersionProvider`, `AppViewModel+APIWiringBackupsHealth.swift`'s `repairHealthProblemProvider`), not re-derived from earlier steps' summaries alone. Two real corrections applied under D-006's "correction" clause: `POST /v1/servers/create` and `POST /v1/components/version` both had an `x-notes`/design gap where MSC 1's own HTTP handler blocks the client's connection open for the full duration of provider work (a `Task` whose `sendJSON` sits after its `await`, not a true fire-and-forget) — for the two install-step families this is real minutes (P7.3's timed installer runs), so both routes now return as soon as the operation is admitted, carrying a populated `operationId` (`ServerCreateResultDTO.operationId` already existed in the P2.8 baseline schema but nothing set it until now; `VersionChangeResultDTO.operationId` is a new additive field). Added `POST /v1/java-runtimes/install` (`installJavaRuntime`, `type: "java-download"`, permission category `settings` — decided for you, reasoning in `phase7-api.md` §3) as a fully async, no-synchronous-variant route, matching `POST /v1/worlds/convert`'s precedent of a required (not optional) `operationId`. `POST /v1/servers/create`'s Bedrock refusal reuses P6.8's existing `capability_unavailable` error code rather than inventing a new one. `POST /v1/health/repair`'s scope is narrowed in the doc, not the schema, to `disable`/`delete` this phase — `update`/`install` stay Phase 8's `action_unavailable`. `tools/api-contract-check.py`'s `EXPECTED_TOTAL` moved from 106 to 107; `docs/msc2/client-capability-matrix.csv` gained one new row (`java-runtimes/install`, all four client statuses `Planned`) and two `operation_id`/`notes` updates on the corrected routes, with every status cell grounded in what `crates/msc-agent/src/main.rs::build_app()` and `cli/mod.rs` actually mount today (only `GET /v1/health`, still P2's canned placeholder, is `Implemented`; everything else across all four client columns is `Planned`) — the same grounding rule `phase6-api.md` §7 established, not a fresh policy call. `--v1-summary` and `capability-matrix-check.py` both pass (107 routes, 109 matrix rows including the two WebSocket channels).
**Verify:** `python3 tools/api-contract-check.py --v1-summary && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P7.9: freeze the Phase 7 provisioning contract`
**Batch:** solo

---

### Pure domain

### P7.10 — Port version entries, catalog parsing, and version comparison
**Status:** awaiting verification
**Files:** `crates/msc-domain/src/server_versions.rs`, `crates/msc-domain/src/lib.rs`, `crates/msc-domain/tests/server_versions.rs`, `crates/msc-domain/tests/component_version.rs`
**What:** Port the pure half of P7.4: the `ServerVersionEntry` model, each provider's response-to-entries parse (fed a byte slice, never a URL), the per-provider version comparisons, the stable/latest flags, and the 1.20 floor filter. Port `ComponentVersionParsing` against the existing `fixtures/component-version/` (21 cases, characterized in an earlier phase and never ported) — `parsePaperJarFilename`, the build-number forms, and `isVersionNewer`. No HTTP, no filesystem: `msc-domain` depends on nothing, per `msc2-engineering.md` §6.
**Actual result:** Built `crates/msc-domain/src/server_versions.rs` porting all six providers' catalog parse/compare against `fixtures/server-jar-providers/`: Vanilla's release-only manifest filter plus its two-hop metadata→download-URL resolution; Purpur's `1.`-prefix filter and its Paper-alignment target-version pick; Fabric's game-version list plus both shapes of its "first stable" rule (the nested `loader.stable` scan and the flat `{version,stable}` helper are genuinely different JSON shapes, not the same function — worth flagging since the case names alone suggested otherwise until the sample JSON was re-read); Paper's fill v3 walk (`paper_flatten_and_sort`, `paper_select_build`'s STABLE/BETA/ALPHA qualification and `hasStableBuild` guard, the 20-candidate-cap walk, and `findStableCeiling`); and NeoForge's/Forge's hand-scraped `<version>` tag scanning, ported to match source exactly — neither ever uses a real XML parser, and malformed/non-XML input silently yields an empty list rather than an error. 25 of the 26 `fixtures/server-jar-providers/` cases are exercised, one per test, in `crates/msc-domain/tests/server_versions.rs`; the 26th (`pufferfish-excluded-from-list-versions-and-download-version-download-latest-only`) documents the `ServerJarProvider` *dispatcher's* per-flavor routing, not any of the six providers' own parsing — that dispatcher doesn't exist in any file this phase has built yet and isn't this step's job, per the fixture's own notes. The one comparator algorithm copy-pasted six times in Swift (`compareMCVersions`/`compareMinecraftVersions`/NeoForge's and Forge's own private `compare`/`compareMCStrings`/`compareForgeVersions`) is ported once as `compare_mc_versions`, collapsing duplication without touching any of the real per-family differences (empty-list handling, `isStable` derivation, sort order) the fixtures document and this module preserves as-is. The 1.20 floor filter is `filter_to_create_flow_floor`, applied uniformly on top of any provider's raw id list by the caller — matching D-014's text that the floor isn't carried in provisioning logic itself.

Correction to this step's own premise: `crates/msc-domain/tests/component_version.rs` was not created, and this step's Verify line originally named a `component_version` test-name substring that matches nothing. `ComponentVersionParsing` was already fully ported in an earlier phase — `parse_paper_jar_filename`, `parse_trailing_build_number`, `build_display_string`, and `is_downgrade` all already live in `crates/msc-domain/src/version.rs`, tested against all 21 `fixtures/component-version/` cases by the pre-existing `crates/msc-domain/tests/version_comparison.rs` (21/21 passing, untouched by this step). This step's "characterized in an earlier phase and never ported" premise was wrong on the "never ported" half; nothing was duplicated here. Verify line amended below to name the real test-file substring, per this rolling-plan's own "amend the Verify in the same commit" convention.
**Verify:** `cargo nextest run -p msc-domain server_versions version_comparison`
**Commit:** `P7.10: port server version catalogs and comparison`
**Batch:** safe

### P7.11 — Port the family launch shape and args-file resolution
**Status:** awaiting verification
**Files:** `crates/msc-domain/src/launch_shape.rs`, `crates/msc-application/src/java_launch.rs`, `crates/msc-domain/tests/launch_shape.rs`, `crates/msc-application/tests/family_launch.rs`
**What:** Generalize Phase 4's Paper-only `build_paper_launch_command` into the six-family launch shape from P7.5, without changing the argv Phase 4 already proves byte-for-byte for Paper. Port `findArgsFile` for both Forge and NeoForge (candidate discovery, configured-pair preference, first-match fallback, nothing-installed nil) against the existing `fixtures/args-file-resolution/`, and the headless script generator against `fixtures/headless-script/`. Keep the *selection* rule in `msc-domain` and the directory listing that feeds it in the caller, the same split `world::first_level_dat_path` already uses.
**Actual result:** Extended `crates/msc-application/src/java_launch.rs` with the six-family generalization (`resolve_java_launch`, `build_headless_java_script`, `find_neoforge_args_file`/`find_forge_args_file`) alongside the untouched Phase 4 Paper-only `build_paper_launch_command`/`PaperLaunchRequest` — the existing byte-for-byte Paper argv proof (`java_launch_paper`, 8/8) passes unchanged; the only edit to that path was routing its jar-basename computation through the new shared `launch_shape::jar_basename` instead of a private duplicate. Built `crates/msc-domain/src/launch_shape.rs`: `shell_quote`, `effective_java_command` (empty path defaults to the bare `java` command), `jar_basename`, `neoforge_select_args_file`/`forge_select_args_file` (the pure selection half of `findArgsFile` — configured-version/pair preference, first-installed fallback, nil when nothing's installed; the directory listing that feeds them stays I/O in `java_launch.rs`'s two finder functions, the same domain/caller split `nbt::first_level_dat_path` already uses), `build_java_invocation` (the `@<args-file> nogui` vs `-jar <jar> --nogui` vs Forge-family missing-args-file `exit 1` dispatch), and `wrap_command_lines` (None/AutoRestart/Screen). All 12 `fixtures/args-file-resolution/` cases and all 19 `fixtures/headless-script/` cases are now exercised: 12 args-file cases plus 4 of the headless-script cases (the 3 pure java-path shapes and the jar-name case) are covered directly in `crates/msc-domain/tests/launch_shape.rs` (19 tests total, the remaining 3 being direct, non-fixture coverage of `shell_quote`/`build_java_invocation`/`wrap_command_lines`); the other 15 headless-script cases, which need the full I/O composition, are covered end-to-end in `crates/msc-application/tests/family_launch.rs` (15 tests). One unfixtured behavior, ported directly from source since P7.5 already flagged that no case exercises it: `jar_basename`'s empty-`paperJarPath` → `"paper.jar"` fallback (`JavaServerLaunchHelper.resolve`, source lines 70-77) — MSC 1 itself has no test for this branch either.
**Verify:** `cargo nextest run -p msc-domain launch_shape && cargo nextest run -p msc-application family_launch java_launch_paper`
**Commit:** `P7.11: port family launch shape and args-file resolution`
**Batch:** safe

### P7.12 — Port creation and runtime-selection policy
**Status:** awaiting verification
**Files:** `crates/msc-domain/src/provisioning.rs`, `crates/msc-domain/src/java_runtime.rs`, `crates/msc-domain/tests/provisioning.rs`, `crates/msc-domain/tests/java_runtime_selection.rs`
**What:** Port the pure decisions creation makes before it touches a disk: folder-name derivation from a display name, the default `server.properties` map and its imported-metadata overrides, the add-on folder per flavor, default RAM by category, initial world identity (reusing `world::sanitized_world_level_name` from P6.9 rather than a second copy), and the create-flow catalog filter. Port runtime selection: `java -version` banner parsing, per-server-then-global path precedence, the required-vs-detected guard, and the install-option table. Extend the existing `java_runtime.rs` rather than starting a parallel module.
**Actual result:** Built `crates/msc-domain/src/provisioning.rs`: the pure decisions `createNewServer` makes before touching a disk, in source order — `trimmed_server_name`'s empty-after-trim refusal, `folder_name_from_safe_name`, `add_on_folder_name` per flavor (reusing a new one-line `AddOnKind::folder_name` on the already-ported `identity.rs` enum rather than a second mapping), `default_ram_gb`'s 2/4 vs 3/6 GB modded default, `effective_world_settings`'s imported-metadata overrides (the seed's fallback order is reversed from difficulty/gamemode — the wizard-normalized seed wins over an imported one, matching source's own asymmetry exactly rather than "fixing" it), `fresh_server_properties`'s exact key set, `should_record_loader_version`'s three-part guard, `should_use_archive_first_shortcut`'s gate, and `new_server_config_fields`, the full `ConfigServer` field set. Neither "initial world identity" nor "the create-flow catalog filter" from this step's own What line needed new code here: the former has no fixture in `fixtures/server-creation/` exercising a pure derivation independent of a caller-supplied `level_name`, and the latter is already fully built by P7.10's `filter_to_create_flow_floor` plus the family list — composing the two is the application layer's job (P7.17), not a reimplementation this step owed.

12 of `fixtures/server-creation/`'s 24 cases are ported here (name/folder derivation, all 3 add-on-folder cases, the RAM default, the server.properties key set, imported-metadata overrides, the archive-shortcut gate, and the loader-version-recording guard); the other 12 need a real directory/file in the loop (the pre-existing-folder refusal, both branches' actual writes, both `WorldSource` copy-failure paths, initial-world-slot failure cleanup, the cross-play template copy, and the top-level `catch` cleanup) and are deferred to P7.17/P7.18's application-service port, per this step's own domain-vs-I/O split. `fixtures/jar-templates/`'s 10 cases are entirely about a real template directory (listing, archiving, reading a template's version from its filename) and are deferred to P7.15 in full — none is pure enough for this step. One case that should have been ported here and was missed: `eula-txt-written-as-eula-false` — the literal constant `"eula=false\n"` is exactly as pure as `fresh_server_properties` — worth a ten-minute follow-up when P7.17 lands rather than reopening this step for it.

Extended `crates/msc-domain/src/java_runtime.rs` (kept as one module, not a parallel one) with the runtime-selection half: `parse_major` (vendor-agnostic banner parsing — first double-quoted token; the legacy `1.x.y_z` scheme takes the second component), `validate_looks_like_java` (five independently-sufficient vendor substrings), `MINECRAFT_INSTALL_OPTIONS`'s fixed four-major table and `recommended_option` (its own two fallback branches are unreachable with any real `required_java_major` output, per P7.7's fixture note — kept anyway, matching source, not simplified into an `unwrap`), and the per-server-then-global java-path precedence (`resolve_create_time_java_path`, and `resolved_settings_java_path` for Settings' own distinct call site). 12 of `fixtures/java-runtime-selection/`'s 18 cases are ported here (the 4 banner shapes, `validate_looks_like_java`'s 3 cases, the option table plus `recommended_option`'s 2 cases, and the 3 precedence cases); the other 6 (the managed Adoptium install, `checkJavaOnPath`, `hasCriticalMissingDependency`) need real filesystem/process/network I/O and are deferred to P7.16, matching this step's own Files list, which names no infrastructure file. `fixtures/java-runtime-guards/`'s pre-existing 7 cases (`crates/msc-domain/tests/java_runtime_guards.rs`, ported in an earlier phase) are untouched and still pass.
**Verify:** `cargo nextest run -p msc-domain provisioning java_runtime`
**Commit:** `P7.12: port creation and runtime selection policy`
**Batch:** safe

---

### Infrastructure

### P7.13 — Build the server-jar provider boundary
**Status:** awaiting verification
**Files:** `crates/msc-infrastructure/src/jar_provider.rs`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/Cargo.toml`, `crates/msc-infrastructure/tests/jar_provider.rs`
**What:** Define the trait every family's catalog and download goes through — list versions, resolve latest, fetch one jar — and implement it over real HTTP for the six families plus the staged-download path Phase 3 already built (`download_staging.rs`): temporary location, size and checksum verification where the provider publishes one, atomic move into place, safe retry, recorded origin and version. Provide a fake provider fed from `corpus/providers/` for every test in this phase. Bound the work explicitly: request timeouts, a response-size cap, and a refusal rather than a hang when a provider is unreachable. This is the first place MSC 2 makes an outbound request on a user's behalf, so it is also where the honest-degradation behavior lives.
**Actual result:** Built `crates/msc-infrastructure/src/jar_provider.rs`, the first place MSC 2 makes an outbound network request. Architecture call, decided rather than asked, per this phase's own "decided without asking" precedent: a blocking HTTP client (`ureq` 3, its default rustls-backed feature set — `msc-infrastructure`'s first HTTP dependency) rather than `reqwest`+`tokio`. Every existing `msc-infrastructure` trait (`FileSystem`, process) is already synchronous, and this stays consistent; the async agent layer wraps a blocking call in `spawn_blocking` when it gets there, which is not this step's job. `Transport` is the boundary trait (`get(url, what, max_bytes) -> Result<Vec<u8>, JarProviderError>`); `HttpTransport` is the real `ureq`-backed implementation, with a 30-second global timeout (connect through full body read — long enough for a real slow download, short enough that a hung provider degrades honestly per this phase's "honest degradation" requirement, rather than blocking a create/version-change operation forever) and two size caps enforced through `ureq` 3's own `body.with_config().limit(n).read_to_vec()`: 20 MB for catalog/metadata responses, 300 MB for jar/installer downloads, chosen against the P7.3 corpus evidence that real server jars run 40–65 MB. Every family function (Vanilla/Purpur/Paper/Fabric/NeoForge/Forge's list-versions and download paths) composes `Transport::get` with P7.10's pure parsers and routes every successful download through the existing `download_staging::stage_download`. Running an installer (as opposed to just downloading its jar) stays P7.14's `loader_installer` job, not this one's.

15 tests in `crates/msc-infrastructure/tests/jar_provider.rs`. 13 exercise the real family logic through a `FakeTransport` fed from `corpus/providers/`'s real recorded responses — zero real network calls, per this phase's "provisioning tests never touch the network" rule. 2 exercise `HttpTransport` itself (the size-cap-fires and under-cap-read-succeeds cases) against a real local loopback server (127.0.0.1, an ephemeral port, spawned in-process for the test) — this is testing this crate's own bounding code against bytes it controls, not a real provider's uptime or shape, so it does not touch the rule the "no network in tests" note is guarding against (real external providers going down, changing shape, or costing rate-limit budget in CI). A third case (connection-refused degrades to a typed error, not a panic) binds a listener and drops it rather than waiting on a real timeout, to stay fast.
**Verify:** `cargo nextest run -p msc-infrastructure jar_provider`
**Commit:** `P7.13: build the server jar provider boundary`
**Batch:** stop-after

### P7.14 — Build the loader-installer runner
**Status:** not started
**Files:** `crates/msc-infrastructure/src/loader_installer.rs`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/tests/loader_installer.rs`
**What:** Run a third-party installer JAR as a supervised subprocess through Phase 3's process substrate: working directory pinned to the new server folder, streamed stdout/stderr surfaced as operation progress rather than swallowed, non-zero exit turned into a typed error carrying the tail of the output, a timeout, and cooperative cancellation that kills the process tree rather than orphaning a half-installed tree. Discover the generated args file afterwards using P7.11's resolver and fail loudly if the installer claimed success but produced none. Tests build a small fake installer JAR locally with `javac`/`jar` — the technique `tools/phase6/phase6-gate-smoke.sh` already uses — covering success, non-zero exit, no-args-file-produced, timeout, and cancellation.
**Verify:** `cargo nextest run -p msc-infrastructure loader_installer`
**Commit:** `P7.14: build the loader installer runner`
**Batch:** stop-after

### P7.15 — Build the jar archive and template store
**Status:** not started
**Files:** `crates/msc-infrastructure/src/template_store.rs`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/tests/template_store.rs`
**What:** Implement the two directories `AppConfig` already carries — `paper_template_dir` and `plugin_template_dir` — as a real store over approved roots and atomic writes: list with the sort and display shape `TemplateItemDTO` needs, archive a downloaded jar under its versioned name, look up the newest template by prefix, read a template's version/build from its filename via P7.10's parser, and copy a template into a server directory. Creating a missing template directory is allowed; escaping the approved root is not.
**Verify:** `cargo nextest run -p msc-infrastructure template_store`
**Commit:** `P7.15: build the jar archive and template store`
**Batch:** safe

### P7.16 — Build Java runtime discovery, selection, and installation
**Status:** not started
**Files:** `crates/msc-infrastructure/src/java_runtime_detection.rs`, `crates/msc-infrastructure/src/java_runtime_install.rs`, `crates/msc-infrastructure/tests/java_runtime_detection.rs`, `crates/msc-infrastructure/tests/java_runtime_install.rs`
**What:** Extend the existing `java_runtime_detection.rs` to the full discovery surface from P7.7 — per-platform search paths on macOS, Linux, and Windows, `JAVA_HOME`, bare `java` on `PATH`, executable normalization, and `java -version` probing behind a trait so tests need no real JDK — and resolve a server's effective runtime through P7.12's precedence rule. If QUESTION 1 was answered (a), also build the managed install: fetch the Adoptium archive for this OS and architecture through P7.13's staged-download path, verify its published checksum, unpack into an MSC-owned runtimes directory, register the result, and leave nothing behind on an interrupted install. If (b), build the reporting path only and say so in the module docs.
**Verify:** `cargo nextest run -p msc-infrastructure java_runtime`
**Commit:** `P7.16: build Java runtime discovery and installation`
**Batch:** stop-after

---

### Application services

### P7.17 — Provision the download-and-go families end to end
**Status:** not started
**Files:** `crates/msc-application/src/provisioning.rs`, `crates/msc-application/src/lib.rs`, `crates/msc-application/tests/provisioning.rs`
**What:** Build the creation workflow for Vanilla, Paper, Purpur, and Fabric against P7.6's fixtures: name validation, folder derivation and collision refusal, Paper's archive-first shortcut through P7.15's store, jar download through P7.13, `eula.txt`, `server.properties`, add-on folder, cross-play template copy, the three world sources reusing Phase 6's world services, the initial world slot, the `ConfigServer` record with its resolved versions, and registration. Every failure path removes the directory it created and leaves the server registry untouched — proved by injected failures at each stage, not asserted.
**Verify:** `cargo nextest run -p msc-application provisioning`
**Commit:** `P7.17: provision download-and-go server families`
**Batch:** safe

### P7.18 — Provision the install-step families as a cancellable operation
**Status:** not started
**Files:** `crates/msc-application/src/provisioning.rs`, `crates/msc-application/src/operations.rs`, `crates/msc-application/tests/provisioning_install_step.rs`
**What:** Add the Forge and NeoForge path: journal the operation before the installer starts, stream installer output as progress, honour cancellation, and reconcile on agent restart so an interrupted install is explained rather than silently forgotten — the operation-journal contract from `msc2-engineering.md` §7. On success, record the resolved Minecraft and loader versions, leave `paperJarPath` empty, and confirm the args file exists before the server is registered as usable. On failure or cancellation, remove the whole directory — a Forge install writes a large `libraries/` tree, so a partial one is both large and unusable.
**Verify:** `cargo nextest run -p msc-application provisioning_install_step`
**Commit:** `P7.18: provision install-step server families`
**Batch:** stop-after

### P7.19 — Change the server JAR version
**Status:** not started
**Files:** `crates/msc-application/src/server_versions.rs`, `crates/msc-application/tests/server_version_change.rs`
**What:** Build version listing for an existing server (its flavor's catalog, current version marked, 1.20 filter applied) and the change itself: refuse while running, download and verify to staging, archive the outgoing jar if `saveDownloadedJars` is set, swap atomically, update the recorded version/build/loader and the Paper sidecar, and for modded loaders run `upgradeModdedLoader`'s re-install rather than a jar swap. A failed download or verification leaves the current jar exactly as it was.
**Verify:** `cargo nextest run -p msc-application server_version_change`
**Commit:** `P7.19: change the server jar version`
**Batch:** safe

### P7.20 — Delete, rename, and accept the EULA for a server
**Status:** not started
**Files:** `crates/msc-application/src/fleet.rs`, `crates/msc-application/tests/fleet.rs`
**What:** Build the three remaining fleet mutations against MSC 1's actual semantics: delete (running-server refusal, what is removed from disk versus what is only deregistered, and how the active-server selection moves), rename (display name against directory name — MSC 1 renames the former and leaves the latter, which the port preserves rather than "improves"), and EULA acceptance through `EULAManager`'s read/write, including the read of an existing `eula.txt` that is neither `true` nor `false`.
**Verify:** `cargo nextest run -p msc-application fleet`
**Commit:** `P7.20: delete, rename, and accept eula for servers`
**Batch:** safe

### P7.21 — List, export, and create from templates
**Status:** not started
**Files:** `crates/msc-application/src/templates.rs`, `crates/msc-application/tests/templates.rs`
**What:** Build the template workflows over P7.15's store: list Paper and plugin templates in the shape `TemplatesResponseDTO` needs; export the active server as a template (its jar, and its plugin jars when `includePlugins` is set, with the running-server refusal); and create a new server from a template, which is P7.17's workflow with the jar source swapped for a local copy and the version read from the template's filename. An unsupported template kind is refused with the frozen `unsupported_template` conflict, not best-effort guessed.
**Verify:** `cargo nextest run -p msc-application templates`
**Commit:** `P7.21: list, export, and create from templates`
**Batch:** safe

### P7.22 — Report startup diagnostics and perform repairs
**Status:** not started
**Files:** `crates/msc-application/src/diagnostics.rs`, `crates/msc-application/src/lifecycle.rs`, `crates/msc-application/tests/diagnostics.rs`
**What:** Wire Phase 1's already-ported crash analyzer into the real lifecycle: on a failed or soft-failed start, gather the console excerpt and any newest crash report, analyze it, persist the last-startup record, and expose the resulting problems with their `helpId`s. Build the repairs — delete an offending jar, disable it by rename — each guarded against a running server and each **re-checked after the fact**, so MSC never reports a repair as successful without verifying it, per `msc2-product.md`'s own promise. Build the four Phase 7-owned health cards (directory, Java runtime, RAM allocation, last startup) and make the not-yet-implemented cards say so explicitly instead of returning a fabricated `ok`.
**Verify:** `cargo nextest run -p msc-application diagnostics`
**Commit:** `P7.22: report startup diagnostics and perform repairs`
**Batch:** stop-after

---

### Public clients

### P7.23 — Wire the provisioning and fleet routes
**Status:** not started
**Files:** `crates/msc-api/src/dto/provisioning.rs`, `crates/msc-api/src/dto/templates.rs`, `crates/msc-api/src/dto/mod.rs`, `crates/msc-agent/src/routes/servers.rs`, `crates/msc-agent/src/routes/templates.rs`, `crates/msc-agent/src/main.rs`, `crates/msc-api/tests/provisioning_conformance.rs`, `crates/msc-agent/tests/provisioning_routes.rs`
**What:** Wire `POST /v1/servers/create`, `/servers/delete`, `/servers/rename`, `/servers/eula`, `GET /v1/templates`, and `POST /v1/templates` to the Phase 7 services, with every status code, error code, and DTO field matching `openapi.json` exactly — including the create result's optional `operationId` and the Bedrock `capability_unavailable` refusal. Permission categories come from the frozen contract (`fleet` for all six mutations). Conformance tests compare the emitted JSON against the schema the same way `dto_conformance.rs` and `world_backup_conformance.rs` already do.
**Verify:** `cargo nextest run -p msc-api provisioning_conformance && cargo nextest run -p msc-agent provisioning_routes`
**Commit:** `P7.23: wire the provisioning and fleet routes`
**Batch:** safe

### P7.24 — Wire the runtime, version, and diagnostics routes
**Status:** not started
**Files:** `crates/msc-api/src/dto/versions.rs`, `crates/msc-api/src/dto/health.rs`, `crates/msc-agent/src/routes/versions.rs`, `crates/msc-agent/src/routes/java_runtime.rs`, `crates/msc-agent/src/routes/health.rs`, `crates/msc-agent/src/main.rs`, `crates/msc-agent/tests/runtime_diagnostics_routes.rs`
**What:** Wire `GET /v1/versions`, `GET /v1/versions/create`, `POST /v1/components/version`, `GET /v1/java-runtimes`, `GET`/`POST /v1/config/java-runtime`, `GET`/`POST /v1/config/ram`, `GET /v1/health/problems`, `POST /v1/health/repair`, and the real `GET /v1/health` — replacing the Phase 2 `demo-card` placeholder and its "no real health-check detection yet" note. Include `/config/ram`'s `no_changes` 400 and `/components/version`'s `download_in_progress` 429, both of which are in the frozen contract and neither of which is optional. If QUESTION 1 was answered (a), wire the managed-install route added in P7.9 as an operation.
**Verify:** `cargo nextest run -p msc-agent runtime_diagnostics_routes`
**Commit:** `P7.24: wire the runtime, version, and diagnostics routes`
**Batch:** safe

### P7.25 — Extend the CLI with provisioning, runtime, and diagnostics commands
**Status:** not started
**Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/tests/cli_provisioning.rs`
**What:** Add the commands a headless host needs, following the shape `msc world` and `msc backup` already set: `msc server create` (family, version, port, world options, `--json`, operation polling for install-step families), `msc server delete`, `msc server rename`, `msc server eula`, `msc version list`/`msc version set`, `msc template list`/`export`/`create`, `msc java list`/`java set` (plus `java install` if QUESTION 1 was answered (a)), and `msc doctor` for the health cards and startup problems with their repairs. Every command goes through the HTTP API like every other CLI command — no direct library calls — so the CLI cannot acquire a capability the API lacks.
**Verify:** `cargo nextest run -p msc-agent cli_provisioning`
**Commit:** `P7.25: extend the cli with provisioning commands`
**Batch:** safe

### P7.26 — Prove the copied iOS client's create, version, and health screens
**Status:** not started
**Files:** `clients/ios/MSCRemoteiOS_Swift/RemoteAPIClient.swift`, `clients/ios/MSCRemoteiOS_Swift/ServerVersionView.swift`, `clients/ios/MSCRemoteiOS_Swift/HealthView.swift`, `clients/ios/MSCRemoteiOS_Swift/DashboardView.swift`, `clients/ios/docs/`, `tools/phase7/ios-provisioning-check.md`
**What:** The copied iOS client already carries a create-server sheet, `ServerVersionView`, and the health problems/repair UI written against MSC 1. Point them at the real MSC 2 agent and fix what actually differs — decoding, error shapes, operation polling for a long install, the Bedrock refusal, and the D-023 rule that a capability may not be quietly dropped from the phone. Record the manual check the same way `tools/phase4/ios-lifecycle-check.md` does, and update the iOS cells in `client-capability-matrix.csv` to what was really observed, not what was intended.
**Verify:** `cargo nextest run -p msc-agent provisioning_routes runtime_diagnostics_routes && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P7.26: prove the copied ios provisioning screens`
**Batch:** stop-after

---

### Proof and gate

### P7.27 — Build the portable six-family provisioning and launch smoke
**Status:** not started
**Files:** `tools/phase7/phase7-gate-smoke.sh`, `tools/phase7/fixtures/`
**What:** Drive a real foreground `msc-agent` through nothing but the CLI and API — the same surface iOS uses — to create all six families and start each one. Portable and committed: a local fake provider serving `corpus/providers/` responses and a locally built fake server jar, a fake installer JAR that writes a real args file, no network, no MSC 1 data, no absolute local paths. It must prove the thing the port plan's later-audit clause asks for: that a Forge and a NeoForge server launch from `@<args-file> nogui` while the other four launch from `-jar <jar> --nogui`, and that a Phase 5-imported non-Paper directory starts too. It must also prove the failure side — an injected download failure and an injected installer failure each leave no directory behind — and kill the agent mid-install to prove the journal reconciles it.
**Verify:** `bash tools/phase7/phase7-gate-smoke.sh --synthetic`
**Commit:** `P7.27: build the six-family provisioning smoke`
**Batch:** stop-after

### P7.28 — Provision real servers from real providers
**Status:** not started
**Files:** `docs/msc2/families/provisioning-evidence/`, `corpus/providers/README.md`
**What:** The one step that uses the real internet. Create, boot, and stop a real server of each family MSC 2 claims to provision, on Cameron's own machine, against the live catalogs — because a fake provider proves the code path, not that PaperMC's API still returns what MSC 1 was written against. Record for each family: the resolved Minecraft and loader version, the download URL and verified checksum, the launch argv, whether the server reached a ready state, and how long the install took. Where a provider has changed shape since MSC 1 was written, that is a finding to record and fix, not to work around. If a family genuinely cannot be provisioned today, stop and report it rather than marking the gate passed on five of six.
**Verify:** `python3 tools/phase7/provider-corpus-check.py --evidence docs/msc2/families/provisioning-evidence`
**Commit:** `P7.28: record real provisioning evidence`
**Batch:** stop-after

### P7.29 — Run the Phase 7 smoke on macOS, Linux, and Windows
**Status:** not started
**Files:** `.github/workflows/ci.yml`
**What:** Add P7.27's synthetic smoke to the existing three-platform `toolchain` job, beside the Phase 6 smoke it already runs. Windows is the leg that matters: path separators in the args file, the `@`-file syntax, quoting a Java path with spaces, and killing a process tree through Job Objects rather than POSIX signals. Fix whatever the Windows runner exposes rather than skipping the leg — D-017 exists precisely so this is discovered here and not after the engine is written against POSIX semantics.
**Verify:** `gh run list --branch <this branch> --limit 1` shows the CI run green on all three platforms, and the run's log contains the Phase 7 smoke step passing on `windows-latest`
**Commit:** `P7.29: run the phase 7 smoke on all three platforms`
**Batch:** stop-after

### P7.30 — Close the Phase 7 exit gate
**Status:** not started
**Files:** `docs/msc2/families/phase7-scope.md`, `docs/msc2/rolling-plan.md`
**What:** Check the phase's literal gate, clause by clause, against the exact candidate commit — not against the step list. For each of the six families: created, launched with the right shape, version-changeable, archivable, and diagnosable. For each deferral in this preamble: still true, still advertised honestly, still owned by a named later phase. For the port plan's later-audit clause: named, and answered with the specific evidence that answers it. Report what does not hold as plainly as what does; a gate that half-holds does not close. Write the result as a gate record, then stop — Codex reviews Phase 7, since Claude Code planned and built it.
**Verify:** `python3 tools/phase7/provider-corpus-check.py --selftest && bash tools/phase7/phase7-gate-smoke.sh --synthetic && cargo nextest run --workspace`
**Commit:** `P7.30: close the phase 7 gate`
**Batch:** solo

---

## Amendments log

Every amendment from Phase 7 onward is recorded here. Earlier phases' amendments are in `rolling-plan-archive.md`.

**2026-08-18 — P7.3: real provider corpus recorded; four live-data findings.** `corpus/providers/` now holds 23 real, provenance-recorded evidence files across all six families, including Forge's `promotions_slim.json` (not named in the step's file list, but needed by `latestRecommendedVersion()` — added rather than left for P7.4 to trip over). Full finding detail lives in `corpus/providers/README.md`; summarized in P7.3's own "Actual result" above. None of the four findings required invoking the step's stop clause — the NeoForge 404 was a stale CDN cache entry (confirmed and retried, not a real outage), and the `1.x`→`26.n` Minecraft version scheme is live-data evolution the oracle already special-cases, not a structural break.

**2026-08-18 — P7.1: QUESTION 1 answered; a wording correction to this file's own P7.6 step.** Cameron chose (a) — MSC 2 installs Java itself — closing "Questions before P7.1"; full reasoning in `docs/msc2/families/phase7-scope.md` and a dated addendum on D-006 in `msc2-decisions.md`. Separately, P7.1 found that this file's P7.6 "What" describes `archiveServerJar` as archiving NeoForge/Forge jars "via their own installer path" — reading `AppViewModel+ServerCreation.swift:622-660` directly shows no such path exists; the function simply never runs for install-step flavors. P7.6/P7.15 should port "Forge/NeoForge have no jar-template equivalent," not port a mechanism that isn't there. The step text itself is left as-is (steps aren't edited retroactively per `CLAUDE.md`); the correction lives in `phase7-scope.md` for P7.6's implementer to read first.
