# MSC 2 — Rolling Plan

> ## STATUS: Phase 6 is closed. Phase 7 is planned (30 steps, P7.1–P7.30). QUESTION 1 is answered. P7.1 is done and awaiting Cameron's verification.
> **Next move:** Verify — Cameron runs P7.1's `Verify:` command and, if he's satisfied, moves its Status to DONE. P7.2 starts after that.
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
**Status:** awaiting verification
**Files:** `docs/msc2/families/phase7-scope.md`, `docs/msc2/msc2-decisions.md`
**What:** Read MSC 1's six provisioning paths (`ServerJarProviders`, `PaperDownloader`, `NeoForgeInstaller`/`ForgeInstaller`, `createNewServer`) beside the frozen contract and Phase 5's raw-import classifier, then write the authoritative Phase 7 boundary as a design record — no Rust. Fix, per family: catalog source, version-entry identity, download-and-go vs install-step, launch shape, what `ConfigServer` fields the create must end up with, and what a failed create must leave behind (nothing). Record the 1.20 filter rule, the Bedrock refusal, the Spigot/Quilt/Pufferfish carry-forward, the cross-play template copy-but-never-download rule, and every symbol-ledger row this phase owns (`server-creation`, `java-runtime`, `templates`, `startup-diagnostics`, `components-versions`, `component-version`, `server-installation`, `setup`, `prerequisites`). Record Cameron's answer to QUESTION 1 as a dated addendum to D-006 (additive route) or, if he chooses (b), as a flagged conflict with `msc2-product.md` for him to resolve. Record the working gate above.
**Actual result:** Cameron answered QUESTION 1 — (a), MSC 2 installs Java itself — recorded as a dated addendum to D-006 in `msc2-decisions.md` and expanded in `phase7-scope.md`. Wrote `docs/msc2/families/phase7-scope.md`: per-family catalog/identity/provisioning-kind/launch-shape table for all six create-flow families; a sourced correction to this rolling-plan's own P7.6 wording (`archiveServerJar` does not archive NeoForge/Forge "via their own installer path" — it simply never archives them; no such path exists in source); `createNewServer` decomposed in source order with the two-path rollback guarantee and an unflagged world-source-failure gap noted for P7.17/P7.18 to decide; the 1.20 filter, Bedrock refusal, and cross-play copy rules pinned precisely; a per-flavor (not per-bucket) accounting of Pufferfish/Spigot/Quilt showing they differ more than "excluded from create flow" implies (Pufferfish has a working latest-only downloader; Spigot has no installer implementation at all; Quilt has no provider of any kind but still launches from an on-disk jar); and the 46-row symbol-ledger table for this phase's nine target domains, with `createNewBedrockServer` and `applyStagedAddOn` explicitly rescheduled (Phase 10, Phase 8) rather than silently dropped.
**Verify:** `python3 -c "from pathlib import Path; s=Path('docs/msc2/families/phase7-scope.md').read_text(); required=['vanilla','paper','purpur','fabric','neoforge','forge','install-step','download-and-go','args file','1.20','capability_unavailable','rollback','pufferfish']; missing=[x for x in required if x.lower() not in s.lower()]; assert not missing, missing; print('OK')"`
**Commit:** `P7.1: scope Phase 7 server families and provisioning`
**Batch:** solo

### P7.2 — Build the Phase 7 provider corpus and gate checker first
**Status:** not started
**Files:** `tools/phase7/provider-corpus-check.py`, `tools/phase7/fixtures/`, `corpus/providers/README.md`
**What:** Build a dependency-free checker before any evidence is collected, so the bar is set before it can be bent to fit what turned up. Inventory mode requires, for every recorded provider response: source URL, capture date, SHA-256, byte size, and which family it belongs to; it fails on a missing provenance field, a duplicate hash, malformed JSON/XML, or a response mutated after recording. Coverage mode takes a fixture directory and asserts every one of the six families is represented and that no fixture cites a recorded response that is absent from the corpus. Passing and deliberately failing self-tests prove each rejection fires. No network access anywhere in this tool.
**Verify:** `python3 tools/phase7/provider-corpus-check.py --selftest`
**Commit:** `P7.2: build the Phase 7 provider corpus checker`
**Batch:** solo

### P7.3 — Record real provider catalogs and installer evidence
**Status:** not started
**Files:** `corpus/providers/`, `corpus/providers/README.md`, `corpus/providers/manifest.json`
**What:** Capture one real response from each live catalog MSC 1 uses — PaperMC fill v3 (projects, versions, builds), Purpur, Mojang's version manifest plus one version JSON, Fabric meta (game, loader, installer), the NeoForge maven listing, and the Forge `maven-metadata.xml` — plus the on-disk shape a real Forge and a real NeoForge installer leaves behind (the args file's name and its `@`-file contents, the `libraries/` layout, the run scripts). Record provenance, capture date, byte size, and SHA-256 for each. Keep responses small: truncate long version arrays to a documented, representative slice rather than committing megabytes, and say in the manifest exactly what was truncated. If a provider is unreachable or has changed shape since MSC 1 was written, record that as a finding and stop rather than hand-writing a plausible response — a fabricated catalog would make every downstream fixture worthless.
**Verify:** `python3 tools/phase7/provider-corpus-check.py --selftest && python3 tools/phase7/provider-corpus-check.py --inventory --providers corpus/providers`
**Commit:** `P7.3: record real provider catalogs and installer evidence`
**Batch:** stop-after

---

### Characterization and contract

### P7.4 — Characterize the six families' version catalogs and jar downloads
**Status:** not started
**Files:** `fixtures/server-jar-providers/`, `fixtures/server-jar-providers/samples/`
**What:** Characterize, against P7.3's recorded responses: Paper's fill v3 walk (all-versions sort, stable-ceiling search, the 20-candidate cap, `server:default` download selection, `STABLE`/`BETA`/`ALPHA` channel filtering, build-date formatting), Purpur's and Vanilla's listing and download, Fabric's three-part loader/installer/game resolution and its `firstStableVersion` fallback, NeoForge's `listVersionPairs` and `minecraftVersion(forNeoForge:)` derivation, and Forge's `parseMavenMetadata`/`parseMavenVersion` XML parse plus `latestRecommendedVersion`. Include the `ServerVersionEntry` identity and `isLatest`/`isStable` rules the frozen `VersionEntryDTO` mirrors, the numeric dotted-version comparisons each provider does by hand (they differ — do not unify them silently), and the 1.20 floor filter as a Phase 7 addition marked as such. Cover failure shapes too: HTTP error, empty version list, malformed JSON, malformed XML, and a build entry missing its download URL.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/server-jar-providers --expect 26 && python3 tools/phase7/provider-corpus-check.py --coverage fixtures/server-jar-providers --providers corpus/providers`
**Commit:** `P7.4: characterize server jar catalogs and downloads`
**Batch:** solo

### P7.5 — Characterize the loader installers and the family launch shape
**Status:** not started
**Files:** `fixtures/loader-installers/`, `fixtures/args-file-resolution/`, `fixtures/headless-script/`
**What:** Characterize `NeoForgeInstaller.install` and `ForgeInstaller.install` end to end: installer URL construction, download into the server directory, the `java -jar <installer> --installServer` invocation and its working directory, streamed stdout/stderr, non-zero exit handling, what is cleaned up afterwards, and what the version-resolution path does when no specific version is requested. Then pin the launch shape that follows: `@<args-file> nogui` for Forge/NeoForge against `-jar <jar> --nogui` for the rest, the missing-args-file failure, and the `paper.jar` fallback when `paperJarPath` is empty. `fixtures/args-file-resolution/` (12 cases) and `fixtures/headless-script/` (19 cases) already exist from earlier phases and are **reused, not rewritten** — extend them only where a real gap shows up, and say in the step's Actual result which existing cases now carry Phase 7 weight.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/loader-installers --expect 16 && python3 tools/fixture-runner/run.py --validate-dir fixtures/args-file-resolution --expect 12 && python3 tools/fixture-runner/run.py --validate-dir fixtures/headless-script --expect 19`
**Commit:** `P7.5: characterize loader installers and launch shape`
**Batch:** solo

### P7.6 — Characterize server creation, rollback, and the jar archive
**Status:** not started
**Files:** `fixtures/server-creation/`, `fixtures/jar-templates/`
**What:** Characterize `createNewServer` step by step in source order: name trim and empty refusal, `servers_root/java/<name-lowercased-underscored>` folder derivation, the pre-existing-folder refusal, the install-step branch against the download-and-go branch, Paper's archive-first shortcut (metadata check, archived filename match, sidecar write), `eula.txt` written as `eula=false`, the exact `server.properties` key set and its imported-metadata overrides, the add-on folder per `addOnKind` (`plugins/`, `mods/`, none for Vanilla), the cross-play template copy, the three `WorldSource` branches, the `ConfigServer` field set including the modded 3/6 GB RAM default, the initial-slot failure path that deletes the whole directory, `recordLoaderVersion`, and the `catch` that removes `newDir` on any throw. Then characterize the archive/template store: `archiveServerJar`'s naming, `latestTemplate(in:prefixLowercased:)`, `jarSummary`, template listing and sort order, export-as-template, and create-from-template. Mark as a deliberate Phase 7 strengthening — not oracle parity — any place MSC 1 leaves partial state that this port will roll back instead.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/server-creation --expect 24 && python3 tools/fixture-runner/run.py --validate-dir fixtures/jar-templates --expect 10`
**Commit:** `P7.6: characterize server creation and the jar archive`
**Batch:** solo

### P7.7 — Characterize Java runtime discovery, selection, and installation
**Status:** not started
**Files:** `fixtures/java-runtime-selection/`, `fixtures/java-runtime-guards/`
**What:** Characterize the runtime half of provisioning: `detectInstalledJavaRuntimes`' search paths and per-platform candidates, `normalizedJavaExecutablePath`, `parseMajor(fromVersionOutput:)` across real `java -version` banner shapes (Temurin, Zulu, GraalVM, OpenJDK, and a non-Java binary that must be rejected), `validateLooksLikeJava`, `resolvedJavaPath`'s per-server-then-global precedence, `checkJavaOnPath`, `isJavaInstalled`/`hasCriticalMissingDependency`, and `JavaInstaller.minecraftInstallOptions`/`recommendedOption(forMinecraftVersion:)`. Cover the guard that matters at start time: required major against detected major, both directions, including the Java-17-era-with-a-newer-runtime warning. `fixtures/java-runtime-guards/` (15 cases) already exists and is reused. If QUESTION 1 was answered (a), also characterize the managed-runtime install as new Phase 7 behavior rather than an MSC 1 port — Adoptium archive URL per OS/architecture, checksum verification, unpack layout under MSC's own runtimes directory, and what an interrupted install must leave behind.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/java-runtime-selection --expect 18 && python3 tools/fixture-runner/run.py --validate-dir fixtures/java-runtime-guards --expect 15`
**Commit:** `P7.7: characterize Java runtime selection and installation`
**Batch:** solo

### P7.8 — Characterize startup diagnostics, problems, and repairs
**Status:** not started
**Files:** `fixtures/startup-problems/`, `fixtures/startup-crash-analyzer/`
**What:** Characterize what turns a failed boot into something a person can act on: `writeLastStartupResult`'s record shape and where it is persisted, `checkLastStartup`'s reading of it into a health card (clean, soft-fail, hard-fail, never-started, stale), the `StartupProblem` shape the frozen `StartupProblemDTO` mirrors — `kind`, `kindTitle`, `offenderName`, `requirement`, `installedFile`, `installedJarStem`, `missingDependency`, `rawExcerpt`, `availableActions`, `isRepairing` — and the repair actions themselves (`delete`, `disable`, and the guards that refuse a repair while the server is running). Cover the Phase 7-owned health cards too: `checkDirectory`, `checkJavaRuntime`, `checkRAMAllocation`, and the severity each produces. `fixtures/startup-crash-analyzer/` and `fixtures/connector-crash-analysis/` already exist from Phase 1 and supply the parse side — this step characterizes what the agent does with the parse result, not the parse itself. Assign a `helpId` to every card and problem kind per `docs/msc2/api-contract/helpid-contract.md`, and record which help topics Phase 11 will therefore have to serve.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/startup-problems --expect 18`
**Commit:** `P7.8: characterize startup diagnostics and repairs`
**Batch:** solo

### P7.9 — Reconcile the Phase 7 API, operation, and capability surface
**Status:** not started
**Files:** `docs/msc2/families/phase7-api.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/client-capability-matrix.csv`, `tools/api-contract-check.py`
**What:** Write the route-by-route reconciliation for the seventeen routes listed in this phase's preamble: request/response field meanings against MSC 1's actual handlers, which are synchronous and which return an `operationId`, the operation types provisioning needs (server creation with an install step is minutes long and must survive an agent restart per `operation-model.md`), the exact error codes each 400/404/409/429 maps to, permission categories (already frozen — confirm, do not re-decide), cancellation semantics for a running installer, and the `capability_unavailable` response for Bedrock creation. Add the one new route from QUESTION 1 if the answer was (a), additively, and move `EXPECTED_TOTAL` in `tools/api-contract-check.py` accordingly. Update every Phase 7 row in `client-capability-matrix.csv` to the status each surface will actually reach this phase — no blank cells, no `Intentional exception` without an owner-approved decision entry.
**Verify:** `python3 tools/api-contract-check.py --v1-summary && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P7.9: freeze the Phase 7 provisioning contract`
**Batch:** solo

---

### Pure domain

### P7.10 — Port version entries, catalog parsing, and version comparison
**Status:** not started
**Files:** `crates/msc-domain/src/server_versions.rs`, `crates/msc-domain/src/lib.rs`, `crates/msc-domain/tests/server_versions.rs`, `crates/msc-domain/tests/component_version.rs`
**What:** Port the pure half of P7.4: the `ServerVersionEntry` model, each provider's response-to-entries parse (fed a byte slice, never a URL), the per-provider version comparisons, the stable/latest flags, and the 1.20 floor filter. Port `ComponentVersionParsing` against the existing `fixtures/component-version/` (21 cases, characterized in an earlier phase and never ported) — `parsePaperJarFilename`, the build-number forms, and `isVersionNewer`. No HTTP, no filesystem: `msc-domain` depends on nothing, per `msc2-engineering.md` §6.
**Verify:** `cargo nextest run -p msc-domain server_versions component_version`
**Commit:** `P7.10: port server version catalogs and comparison`
**Batch:** safe

### P7.11 — Port the family launch shape and args-file resolution
**Status:** not started
**Files:** `crates/msc-domain/src/launch_shape.rs`, `crates/msc-application/src/java_launch.rs`, `crates/msc-domain/tests/launch_shape.rs`, `crates/msc-application/tests/family_launch.rs`
**What:** Generalize Phase 4's Paper-only `build_paper_launch_command` into the six-family launch shape from P7.5, without changing the argv Phase 4 already proves byte-for-byte for Paper. Port `findArgsFile` for both Forge and NeoForge (candidate discovery, configured-pair preference, first-match fallback, nothing-installed nil) against the existing `fixtures/args-file-resolution/`, and the headless script generator against `fixtures/headless-script/`. Keep the *selection* rule in `msc-domain` and the directory listing that feeds it in the caller, the same split `world::first_level_dat_path` already uses.
**Verify:** `cargo nextest run -p msc-domain launch_shape && cargo nextest run -p msc-application family_launch java_launch_paper`
**Commit:** `P7.11: port family launch shape and args-file resolution`
**Batch:** safe

### P7.12 — Port creation and runtime-selection policy
**Status:** not started
**Files:** `crates/msc-domain/src/provisioning.rs`, `crates/msc-domain/src/java_runtime.rs`, `crates/msc-domain/tests/provisioning.rs`, `crates/msc-domain/tests/java_runtime_selection.rs`
**What:** Port the pure decisions creation makes before it touches a disk: folder-name derivation from a display name, the default `server.properties` map and its imported-metadata overrides, the add-on folder per flavor, default RAM by category, initial world identity (reusing `world::sanitized_world_level_name` from P6.9 rather than a second copy), and the create-flow catalog filter. Port runtime selection: `java -version` banner parsing, per-server-then-global path precedence, the required-vs-detected guard, and the install-option table. Extend the existing `java_runtime.rs` rather than starting a parallel module.
**Verify:** `cargo nextest run -p msc-domain provisioning java_runtime`
**Commit:** `P7.12: port creation and runtime selection policy`
**Batch:** safe

---

### Infrastructure

### P7.13 — Build the server-jar provider boundary
**Status:** not started
**Files:** `crates/msc-infrastructure/src/jar_provider.rs`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/Cargo.toml`, `crates/msc-infrastructure/tests/jar_provider.rs`
**What:** Define the trait every family's catalog and download goes through — list versions, resolve latest, fetch one jar — and implement it over real HTTP for the six families plus the staged-download path Phase 3 already built (`download_staging.rs`): temporary location, size and checksum verification where the provider publishes one, atomic move into place, safe retry, recorded origin and version. Provide a fake provider fed from `corpus/providers/` for every test in this phase. Bound the work explicitly: request timeouts, a response-size cap, and a refusal rather than a hang when a provider is unreachable. This is the first place MSC 2 makes an outbound request on a user's behalf, so it is also where the honest-degradation behavior lives.
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

**2026-08-18 — P7.1: QUESTION 1 answered; a wording correction to this file's own P7.6 step.** Cameron chose (a) — MSC 2 installs Java itself — closing "Questions before P7.1"; full reasoning in `docs/msc2/families/phase7-scope.md` and a dated addendum on D-006 in `msc2-decisions.md`. Separately, P7.1 found that this file's P7.6 "What" describes `archiveServerJar` as archiving NeoForge/Forge jars "via their own installer path" — reading `AppViewModel+ServerCreation.swift:622-660` directly shows no such path exists; the function simply never runs for install-step flavors. P7.6/P7.15 should port "Forge/NeoForge have no jar-template equivalent," not port a mechanism that isn't there. The step text itself is left as-is (steps aren't edited retroactively per `CLAUDE.md`); the correction lives in `phase7-scope.md` for P7.6's implementer to read first.
