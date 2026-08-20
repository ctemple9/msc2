# Phase 7 scope: Server families and provisioning

**Status:** P7.1 scoping note, per `docs/msc2/rolling-plan.md`'s Phase 7 section.
**Source of truth:** `msc2-port-plan.md` §3 (Phase 7 gate), `docs/msc2/rolling-plan.md` (Phase 7 section), `msc2-decisions.md` (D-006, D-014), the Phase 0 symbol ledger (`docs/msc2/audit/msc2-symbol-ledger.csv`), and Phase 5's raw-import classifier (`crates/msc-application/src/import.rs`), which already assigns every one of the nine `JavaServerFlavor` cases on import regardless of whether Phase 7's create flow offers them.
**MSC 1 oracle:** `~/Documents/Swift Projects/minecraft-server-controller`, read-only. Every fact below was read directly from source, not inferred — file and line are cited so a reviewer can jump straight to it. Primary files: `JavaServerFlavor.swift`, `ServerJarProviders.swift`, `PaperDownloader.swift`, `NeoForgeInstaller.swift` (both `NeoForgeInstaller` and `ForgeInstaller`), `AppViewModel+ServerCreation.swift`, `JavaServerLaunchHelper.swift`, `EULAManager.swift`.

This note fixes the Phase 7 family and runtime boundary before code starts, in the same role `phase3-scope.md` through `phase6-scope.md` played for their phases. It does not approve new product behavior on its own; where a choice is genuinely open, it records the recommended working answer and flags it as such.

## Working exit gate

Quoted from `rolling-plan.md`'s Phase 7 header, itself pinned to `msc2-port-plan.md` §3:

> Vanilla, Paper, Purpur, Fabric, NeoForge, Forge. Runtime selection, installer flows, archive behavior, startup diagnostics. Scope bounded by the 1.20 floor (D-014).

Plus the port plan's later-audit clause: "after Phase 5's broad raw import, Phase 7 must prove non-Paper Java servers are not merely classified but actually launchable with the correct family-specific startup shape."

The working exit criteria from `rolling-plan.md` are not repeated verbatim here — they're the phase's job to satisfy, not this note's to restate. This note's job is narrower: fix, per family, what "correct" means before any of those criteria can be checked against it.

## Java runtime install: Cameron's answer (D-006 addendum)

**Dated addendum to D-006, 2026-08-18.** Cameron chose option (a) from "Questions before P7.1" in `rolling-plan.md`: **MSC 2 installs Java itself.** The agent downloads Adoptium's plain archive for its own OS/architecture, verifies the published checksum, unpacks it into MSC's own data directory, and uses it — no graphical installer, no user double-click, matching msc2-product.md's promised "[Install Java 21]" flow on a headless host.

Consequences for this phase's step list, recorded here so later steps don't have to re-derive them:

- **One new route, additive.** `POST /v1/java-runtimes/install` joins the seventeen routes already listed in `rolling-plan.md`'s Phase 7 preamble, under D-006's "extension" clause (superset, not a break to the baseline). P7.9 adds it to `openapi.json` and bumps `EXPECTED_TOTAL` in `tools/api-contract-check.py`; P7.24 wires it as an operation (it downloads an archive, which can be slow on a poor connection).
- **P7.16 grows.** Beyond discovery/selection (which both branches of QUESTION 1 needed), P7.16 now also builds the managed install: Adoptium archive URL resolution per OS/architecture, checksum verification, unpack into an MSC-owned runtimes directory (not the system Java location — D-009 applies here too: MSC 2 owns its own runtime state, never reaches into a system package manager), and an interrupted-install-leaves-nothing-behind guarantee, the same rollback discipline P7.17/P7.18 already apply to server creation.
- **MSC 1 has no equivalent to port.** MSC 1's `JavaInstaller.swift` (`installerURL`/`downloadInstaller`/`manualDownloadURL`, source line 75) fetches a Temurin **.pkg** and hands it to macOS's Installer.app for a human to click through — a GUI, same-machine, macOS-only mechanism with no cross-platform equivalent. The symbol-ledger row for this (already marked `UNSURE` at ledger time, see "Symbol-ledger rows" below) is **not** what Phase 7 ports. Phase 7's managed install is new agent-owned behavior, built against Adoptium's plain archive API (the same one the .pkg flow itself downloads from, minus the macOS installer wrapper), characterized fresh in P7.7 rather than characterized as a port of `installerURL`/`downloadInstaller`.

## The six-plus-three family boundary

Nine `JavaServerFlavor` cases exist in MSC 1 (`JavaServerFlavor.swift:75-87`); Phase 7's create flow offers exactly the six the port plan names. Per family:

Six families total, split across two provisioning kinds — `downloadAndGo` (Swift enum case; referred to below in the port plan's own hyphenated form, **download-and-go**) and `installStep` (**install-step**):

| Family | Catalog source | Version-entry identity | Provisioning kind | Launch shape |
|---|---|---|---|---|
| **Vanilla** | Mojang `version_manifest_v2.json`, then per-version metadata for `downloads.server.url` (`ServerJarProviders.swift:338-416`) | `id = mcVersion` (e.g. `"1.21.4"`), `loaderVersion = nil`, `buildLabel = nil`, `isStable = true` for every `type: "release"` entry — snapshots are filtered out entirely, not just deprioritized | download-and-go | `-jar <jar> --nogui` |
| **Paper** | `fill.papermc.io/v3` — project versions list, then per-version `/builds` (`PaperDownloader.swift`, `ServerJarProviders.swift:134-255`) | `id = mcVersion`; `buildLabel = "build N"` (STABLE channel, highest build id wins) or `"build N · beta"` (BETA/ALPHA fallback when no STABLE build exists — new-scheme dev versions like `26.x.x`); `isStable` reflects which branch fired | download-and-go | `-jar <jar> --nogui` |
| **Purpur** | `api.purpurmc.org/v2/purpur` (`ServerJarProviders.swift:257-334`) | `id = mcVersion`; the version list itself has **no per-entry build field** — `buildLabel = nil` for every picker entry, `isStable = true` unconditionally (any version starting `"1."`) | download-and-go | `-jar <jar> --nogui` |
| **Fabric** | `meta.fabricmc.net/v2` — three separate lookups: `versions/game` (Minecraft), `versions/loader/{game}` (loader), `versions/installer` (installer), composed into one download URL (`ServerJarProviders.swift:418-518`) | `id = mcVersion`; `loaderVersion` only populated by `downloadLatest`/`downloadVersion`'s result, **not** by `listVersions` — the picker's `ServerVersionEntry` rows carry `loaderVersion: nil` (game-version-only picker; loader is always "the latest stable for this game version" unless a `.mrpack` pins one) | download-and-go (single self-contained launcher jar; Fabric fetches vanilla itself on first run) | `-jar <jar> --nogui` |
| **NeoForge** | `maven.neoforged.net` `maven-metadata.xml`, hand-scraped `<version>` tags, filtered to non-hyphenated (stable) entries (`NeoForgeInstaller.swift:43-95`) | `id = "{mc}—{neoForgeVersion}"` (em dash, matches Forge's separator); `mcVersion` derived by `minecraftVersion(forNeoForge:)` — classic scheme `"21.1.234"` → `"1.21.1"`, new scheme (major ≥ 26) `"26.2.x"` → `"26.2"` with no `"1."` prefix; `loaderVersion = neoForgeVersion`; every stable build listed (not one per MC version) because modpacks pin exact builds | install-step | `@<args-file> nogui` |
| **Forge** | `maven.minecraftforge.net` `maven-metadata.xml` for the **picker** (every published build — `promotions_slim.json` only has the "recommended" one, which modpacks routinely don't pin); `promotions_slim.json` for the **latest-recommended** path only (`NeoForgeInstaller.swift:397-528`) | `id = "{mc}—{forgeVersion}"`; parsed from Maven's `"{mc}-{forge}"` version string; `isStable = !forgeVersion.contains("-")` | install-step | `@<args-file> nogui` |

**A precise correction to `rolling-plan.md`'s own P7.6 wording.** That step's "What" describes `archiveServerJar`'s per-flavor pattern as covering "NeoForge/Forge, which archive via their own installer path." Read against source, this isn't accurate: `archiveServerJar` (`AppViewModel+ServerCreation.swift:622-660`) is a `switch` over `.paper/.purpur/.vanilla/.fabric` with a bare `default: return` — and it is never even called for NeoForge/Forge in `createNewServer`, because the call site (`AppViewModel+ServerCreation.swift:290`) sits inside the non-`installStep` branch only. **NeoForge and Forge are not archived to the jar-template store by any mechanism** — there is no parallel "archive via installer path." P7.6 and P7.15 should port this as: the archive/template store covers the four `downloadAndGo` flavors only; Forge/NeoForge have no jar-template equivalent in MSC 1, and Phase 7 does not invent one for them (their reusable artifact is the generated `libraries/` tree plus args file, not a single portable jar).

### What `install`/`downloadVersion` need per family, precisely

- **Version-entry `id` is not a Minecraft version for every family.** Downstream Rust code (`ServerVersionEntry`/`VersionEntryDTO` in P7.10) must not assume `id == mcVersion` — true for Vanilla/Paper/Purpur/Fabric, false for NeoForge/Forge, where `id` is the paired string and `mcVersion` is a separately-derived field. A version-change route (`P7.19`) that keys off `id` alone will silently misbehave for the two `installStep` families.
- **`ServerVersionEntry.latest` is a sentinel, not a real entry** (`ServerJarProviders.swift:26-34`): `id = "__latest__"`, `mcVersion = ""`. Every call site branches on `!specificVersion.isLatest` before trusting `mcVersion`/`loaderVersion` — `mcVersion == ""` is never a real version to download. Phase 7's Rust port needs the same explicit `isLatest` short-circuit, not an accidental empty-string lookup.
- **Fabric's picker `loaderVersion` is always `nil`.** A version-change UI that pre-fills a loader field from `listVersions()`'s result will always show blank for Fabric — that's oracle-faithful, not a gap; the loader is resolved fresh at download time unless a `.mrpack` pins one (out of scope this phase per D-027).
- **Purpur's picker has no build number at all.** Anywhere Phase 7 shows "current build" for a Purpur server, the true source is `AppViewModel+ComponentsVersions.swift`'s local-file-based read (component-version domain, already characterized), not the create-time catalog — the catalog literally doesn't carry one.

## Cross-family creation mechanics (`createNewServer`, `AppViewModel+ServerCreation.swift:128-403`)

Fixed in source order, since every step from P7.6 onward depends on getting this order right:

1. **Name trim and empty refusal** — `safeName` trimmed; empty name aborts before any I/O.
2. **Folder derivation** — `servers_root/java/<name lowercased, spaces→underscores>` (line 169). Pre-existing folder at that path is refused with a named-conflict message (line 178-182) before any directory is created — this is a **check-then-create**, not an atomic claim; Phase 7's Rust port should close that race (a second concurrent create with the same name) rather than reproduce it, since nothing downstream depends on the race being preserved.
3. **Branch on `provisioningKind`** — `installStep` (NeoForge/Forge) runs the installer and never touches `jarSource`/`primaryJarPath` beyond leaving it `""`; everything else resolves `jarSource` (`.template` copy-in or `.downloadLatest`/specific-version download to `newDir/paper.jar` — **every downloadAndGo flavor's jar lands at a file literally named `paper.jar`**, not a flavor-specific filename; only the *archive* copy gets a flavor-specific name).
4. **Paper's archive-first shortcut** — only for `flavor == .paper && saveDownloadedJars`: checks `PaperDownloader.fetchLatestMetadata()`, builds the expected archive filename (`paper-<version>-build<build>.jar`), and copies from the template dir instead of downloading if it's already there (lines 258-272). No other flavor gets this shortcut — Purpur/Vanilla/Fabric always download fresh even if an identical jar sits in the archive.
5. **`eula.txt`** written as exactly `"eula=false\n"` (line 296) — a fresh server always starts EULA-unaccepted, regardless of import metadata.
6. **`server.properties`** — exact key set: `server-port`, `motd` (= server display name, not a separate field), `max-players` (hardcoded `"20"`), `online-mode` (hardcoded `"true"`), `difficulty`, `gamemode`, `level-name`, and `level-seed` only if a seed resolved. `difficulty`/`gamemode`/`seed` are **overridden by imported-world metadata** when the world source is `.backupZip` or `.existingFolder` (lines 160-162) — the user's wizard-selected values are the fallback, not the winner, when the imported data disagrees.
7. **Add-on folder** per `flavor.addOnKind` — `plugins/` for standard, `mods/` for modded, **no folder at all for Vanilla** (`addOnKind == nil`, line 136-139 of `JavaServerFlavor.swift`).
8. **Cross-play template copy** — fires only when `enableCrossPlay && addOn == .plugin` (line 317), i.e. **never** for modded or Vanilla servers even if cross-play was requested. See "Cross-play" below for the copy rule itself.
9. **World source** — one of three (`WorldSource.fresh` / `.backupZip(URL)` / `.existingFolder(URL)`); `.backupZip`/`.existingFolder` failures abort creation (`return false`) but **do not** trigger the directory-removal rollback at this point — that only happens for a thrown error or the initial-slot failure below. This is a real gap worth flagging: a failed world-source unzip/copy at this stage leaves `newDir` (with jar, eula, properties, add-on folder already written) on disk. MSC 1 accepts this; Phase 7 should decide explicitly rather than silently inherit it (see "Not resolved by this note").
10. **`ConfigServer` construction** — `paperJarPath = primaryJarPath` (empty for install-step families), `minRamGB = 2`/`maxRamGB = 4` by default, **overridden to `3`/`6` when `flavor.category == .modded`** (line 345-348, applies to Fabric/NeoForge/Forge alike — not just the install-step pair), plus `javaFlavor`, `minecraftVersion`, `serverBuild`, `loaderVersion`, `bannerColorHex` (from global config default), `playitEnabled`, `xboxBroadcastEnabled`, and `bedrockPort` (only set if cross-play requested a Bedrock port — a field that exists on a *Java* `ConfigServer` for the cross-play listener, not a type error).
11. **Initial world slot** — `createInitialPersistentWorldSlot` failure removes `newDir` and returns `false` with a set error message (lines 356-367): **this is the first of two rollback paths**, and it fires even though no Swift error was thrown.
12. **Staged add-ons** applied after the server is otherwise complete (line 370-380) — **out of scope this phase**, see "Deferred on purpose."
13. **Registration** (`upsertServer`) and loader-version recording (`recordLoaderVersion`, modded only) happen last, after everything above has succeeded.
14. **Top-level `catch`** (line 395-402) — **the second rollback path**: any thrown Swift error (network failure, installer non-zero exit, file I/O error at any step 1-9) removes `newDir` wholesale and returns `false`.

**The rollback guarantee Phase 7 must preserve exactly:** two independent paths (explicit removal on initial-slot failure, blanket removal in the `catch`) converge on the same outcome — a failed create leaves no directory behind. Phase 7's Rust port should unify these into one guarantee (e.g. a single `Drop`-guarded or explicitly-scoped cleanup that fires on *any* early return, closing the world-source-failure gap noted in step 9 above) rather than porting two separate ad hoc removal sites — this is exactly the kind of "deliberate Phase 7 strengthening, not oracle parity" the port plan invites for partial-state cases (see `rolling-plan.md`'s P7.6 note).

## The 1.20 floor filter (D-014)

Applies to the **offered catalog**, not to what can run. `GET /v1/versions/create` and `GET /v1/versions` drop entries below Minecraft 1.20 for every family; a below-floor server that reaches MSC 2 through raw or transfer import still lists, starts, and is fully manageable — D-014's own text is explicit that older versions are "not carried in provisioning logic," not blocked. None of the six providers above filter by version floor themselves in MSC 1 (Vanilla/Purpur filter by `release`/`"1."` prefix only); the 1.20 cut is a Phase 7 addition layered on top of each provider's raw list, applied identically regardless of provider-specific quirks (e.g. Purpur's own experimental-build noise above the stable line, which the 1.20 filter does not need to reason about specially — it's a separate concern from stable-vs-experimental).

## The Bedrock refusal

`POST /v1/servers/create` with `serverType: "bedrock"` returns `capability_unavailable` — the exact `ErrorDTO.code` P6.8 already established for the Bedrock-unsupported branch of `backup-restore` (`docs/msc2/api-contract/openapi.json`'s `x-notes` on that route: "Bedrock-unsupported ... carries ErrorDTO.code 'capability_unavailable'"). `createNewBedrockServer` (`AppViewModel+ServerCreation.swift:408-542`) is read for reference only; none of it is ported this phase. Its ledger row (`server-creation` domain) stays `disposition=agent` but is rescheduled to Phase 10 below.

## Spigot, Quilt, and Pufferfish: what "carried forward" actually means

All nine flavors remain classifiable on import (Phase 5's classifier doesn't filter by `isAvailableInCreateFlow`) and launchable if imported. But the three differ from each other more than "excluded from create flow" suggests, and Phase 7 should carry the *right* behavior forward for each rather than treating them as one bucket:

- **Pufferfish** (`provisioningKind = .downloadAndGo`, `category = .standard`) has a real, working downloader — `PufferfishDownloader` hits Jenkins CI at `ci.pufferfish.host`, resolves the last successful build's `reobf.jar` artifact, and derives the MC version from the artifact filename (`ServerJarProviders.swift:520-563`). It's reachable from `ServerJarProvider.downloadLatest` but **not** from `listVersions` (falls into the `default: return []` case, line 76) or `downloadVersion` (falls into `default: throw unsupportedFlavor`, line 92) — Pufferfish has a "download the latest" path and nothing else, even in MSC 1 itself. An imported Pufferfish server launches exactly like Paper (`-jar <jar> --nogui`, since it's `category = .standard`/not install-step).
- **Spigot** (`provisioningKind = .installStep`, per line 145 of `JavaServerFlavor.swift` — it "looks like a download to the user but actually needs a local BuildTools compile") has **no installer implementation anywhere in this codebase** — no `SpigotInstaller.swift` exists. It is carried forward as classifiable/launchable-if-imported only in the sense that *if* a Spigot jar already exists on disk from outside MSC entirely, nothing in Phase 7 prevents it from starting; MSC 1 itself never provisions one, so there is no working behavior to port. `rolling-plan.md`'s "Spigot's BuildTools compile is not built" is precisely correct and needs no correction.
- **Quilt** (`category = .modded`, `addOnKind = .mod`) has **no provider at all**, not even a latest-only path — `ServerJarProvider.listVersions`/`downloadVersion`/`downloadLatest` have no `.quilt` case anywhere in the three `switch` statements; it silently falls through to `default` in each (empty list, unsupported-flavor throw, unsupported-flavor throw). An imported Quilt server still **launches** fine (`JavaServerLaunchHelper` only needs a jar on disk and doesn't consult `ServerJarProvider` at launch time — `quilt-server-launch.jar` is even named explicitly in `ServerEditorJarsTab.swift`'s modded-install-detection heuristic, confirming MSC 1 expects Quilt jars to exist from external provisioning), but there is no download/version-change path to port for it, this phase or ever, without inventing new behavior MSC 1 never had.

**Net effect on Phase 7's fixtures (P7.4):** the "26 cases" catalog/download count should include Pufferfish's `downloadLatest`-only path and its failure shapes, but should **not** invent `listVersions`/`downloadVersion` coverage for Pufferfish or any coverage at all for Spigot/Quilt catalog behavior — there is nothing there to characterize. Launch-shape fixtures (P7.5) should include a Quilt case proving launch works from an on-disk jar with no provider involvement, since that's the one Quilt behavior that's real.

## Cross-play template copy-but-never-download

`applyCrossPlayTemplatesIfAvailable` (`AppViewModel+ServerCreation.swift:547-580`), called only from the `addOn == .plugin` branch of creation:

1. Reads the **global** plugin-template directory (`configManager.pluginTemplateDirURL` — shared across all servers, not per-server).
2. Filters to `.jar` files, finds the first whose lowercased filename `contains("geyser")` and the first `contains("floodgate")` — **substring match, no version pinning, no "newest" selection** (unlike `latestTemplate(in:prefixLowercased:)` used elsewhere in the templates domain).
3. If **either** is missing, logs a message and returns — **does not fail server creation**. Cross-play being requested with no templates downloaded yet is a soft no-op, not an error.
4. If both are found, overwrites any same-named file already in the new server's `plugins/` dir and copies both in.

Phase 7 never downloads a Geyser/Floodgate template itself (`downloadLatestGeyserTemplate`/`downloadLatestFloodgateTemplate` stay Phase 9, per `rolling-plan.md`) — it only copies from what's already archived, exactly as above.

## Symbol-ledger rows owned by this phase

Every `docs/msc2/audit/msc2-symbol-ledger.csv` row whose `target_domain` is one of `server-creation`, `java-runtime`, `templates`, `startup-diagnostics`, `components-versions`, `component-version`, `server-installation`, `setup`, or `prerequisites` — **46 rows total**, every one `disposition=agent`. Grouped by MSC 1 source file:

| MSC 1 file | Symbols (rows) | Rust destination in this phase |
|---|---|---|
| `ServerJarProviders.swift` / `PaperDownloader.swift` / `NeoForgeInstaller.swift` | Not separately ledgered (P0 predates their read as Mixed-bucket files in the same way) — characterized fresh in P7.4/P7.5, ported in P7.10/P7.11 | P7.4–P7.5 (characterize), P7.10–P7.11 (`msc-domain`) |
| `AppViewModel+ServerCreation.swift` | `initialWorldSlotName`/`normalizedInitialWorldSeed`, `updateWorldIdentityForNewServer`, `createInitialPersistentWorldSlot`, `createNewServer`, `applyCrossPlayTemplatesIfAvailable`, `resolvedBedrockWorldFolder`, `archiveServerJar`, `applyStagedAddOn` (8 rows) + `createNewBedrockServer` (1 row, rescheduled — see below) | P7.6 (characterize), P7.12 (`msc-domain` policy half), P7.17/P7.18 (`msc-application` workflow half) |
| `AppViewModel+Templates.swift` / `AppViewModel+APIWiringServerMgmt.swift` (templates rows) | Plugin/Paper template load/add/remove/apply, `downloadLatestPaperTemplate`, `downloadLatestGeyserTemplate`/`downloadLatestFloodgateTemplate` (download mechanism only — the *files* they fetch stay Phase 9, but the template-store plumbing they write into is this phase's), `jarSummary`, `latestTemplate(in:prefixLowercased:)`, update-from-template family, API-wiring template routes (12 rows) | P7.6/P7.15 (`msc-infrastructure` template store), P7.21 (`msc-application` template workflows), P7.23 (routes) |
| `JavaRuntimeManager.swift` / `JavaInstaller.swift` / `PrerequisitesView.swift` / `ServerProcessManager.swift` / `SetupWizardView.swift` / `AppViewModel+ServerSettings.swift` / `AppViewModel+HealthCards.swift` (java-runtime rows) / `AppConfig.swift` | Runtime discovery/detection/parsing, path validation, `resolvedJavaPath` precedence, RAM MB conversion (16 rows across java-runtime + setup) | P7.7 (characterize), P7.12/P7.16 (`msc-domain`/`msc-infrastructure`) |
| `PrerequisitesView.swift` (prerequisites rows) | `detectTailscale`/`activeTailscaleIP`, `hasCriticalMissingDependency`/`isJavaInstalled` (2 rows) | **Not this phase** — Tailscale detection is networking/reachability, not provisioning; `hasCriticalMissingDependency`'s Java half is already covered by the java-runtime rows above. Flagged here for completeness (coverage rule), scheduled to Phase 9 alongside the rest of reachability. |
| `AppViewModel+HealthCards.swift` (startup-diagnostics row) | `checkLastStartup`/`writeLastStartupResult` (1 row) | P7.8 (characterize), P7.22 (`msc-application`) |
| `AppViewModel+ComponentsVersions.swift` | Snapshot refresh/online-check workflow, Paper-track switching, local-version parsers (components-versions, 4 rows) + `downloadAndApplySelectedPaperVersion`/`downloadAndApplyJarVersion`/`upgradeModdedLoader`/loader-version-record CRUD (java-runtime, 4 rows) | The `components-versions` rows are Components-tab state aggregation for the *existing* `POST /v1/components/version` route this phase owns — P7.19. The `java-runtime`-tagged rows here are version-change workflow (download-and-apply, modded-loader upgrade with its downgrade-guard-forces-backup policy, loader-version library) — P7.19 also. |
| `DetailsComponentsTabView.swift` | `ComponentStatus.derive`/`isVersionNewer`/`buildNumber`/`versionsMatch` (component-version, 1 row) | Already flagged at ledger time as likely overlapping `fixtures/component-version/`'s P0.4 coverage — P7.4/P7.10 should cross-check against that existing fixture set rather than treat this as new characterization. |
| `ServerEditorJarsTab.swift` | `moddedServerIsInstalled(cfg:)` (server-installation, 1 row) | Flagged at ledger time as duplicating `AppViewModel+ServerImport.detectJavaFlavor` (Phase 5) — P7.7/P7.12 should consolidate rather than port a second implementation. |

**Two rows in this set are explicitly not built in this phase**, same convention as `phase6-scope.md`'s `duplicateBackupToNewServer` note — both stay `disposition=agent` on the ledger, just rescheduled:

- `AppViewModel+ServerCreation.swift::createNewBedrockServer` — Bedrock creation stays **Phase 10**; Phase 7 refuses it with `capability_unavailable` per "The Bedrock refusal" above.
- `AppViewModel+ServerCreation.swift::applyStagedAddOn` — wizard-staged add-ons at creation time stay **Phase 8**; `stagedAddOns` has no field on the frozen `ServerCreateRequestDTO` (confirmed by reading the schema directly), so Phase 7 leaves nothing dangling by deferring it.

## Deferred on purpose

Restated from `rolling-plan.md`'s Phase 7 "Not in this phase" list so this note is self-contained:

- **Bedrock creation and Bedrock versions** stay Phase 10 (`capability_unavailable`, above).
- **Add-ons, modpacks, and the rest of `/v1/components`** stay Phase 8 — Phase 7 claims only `POST /v1/components/version` (the server *core jar*, not an add-on).
- **Geyser, Floodgate, Playit, Xbox Broadcast downloads** stay Phase 9 — Phase 7 only copies templates that already exist locally (above).
- **The other health cards** (port reachability, component jars, Bedrock world data, VM runtime) report an explicit not-yet-implemented note rather than a fabricated `ok`.
- **`GET /v1/help/{helpId}`'s resolver** stays Phase 11; `helpId` population on this phase's cards/problems does not.
- **Spigot, Quilt, and Pufferfish** stay out of the create-flow catalog — see "What 'carried forward' actually means" above for exactly what that does and doesn't mean per flavor.
- **Desktop/web screens** stay Phase 11.
- **Modpack-driven creation** (`.mrpack`/CurseForge as a create source) stays Phase 8, along with D-027.

## Not resolved by this note

This note fixes the family boundary and the creation/rollback rules as MSC 1 actually implements them; it does not decide:

- **The world-source-failure gap** flagged in step 9 of "Cross-family creation mechanics" — MSC 1 leaves a partially-written `newDir` on disk if `.backupZip`/`.existingFolder` staging fails after `eula.txt`/`server.properties`/add-on-folder are already written, without triggering either rollback path. Whether P7.17/P7.18 close this gap (recommended — it's the same "deliberate strengthening" class as the two-rollback-paths unification above, and costs nothing since the directory-removal mechanism already exists for the other two failure points) or preserve it is P7.17's call to make explicitly, not this note's.
- **The folder-name check-then-create race** flagged in step 2 — same treatment: recommended to close, not this note's decision to finalize.
- **Exact staged-download integration** for P7.13's provider boundary (timeouts, size caps, retry policy) — this note fixes *what* each provider returns and *how* a version is identified, not the HTTP-boundary mechanics P7.13 owns.
- **The managed Java-install unpack layout** beyond "MSC-owned runtimes directory, not system-owned" — P7.16's job to pin exactly.
- Any other later step's design. Where a later step hits a genuine judgment call, it raises it as a question in the format `CLAUDE.md` requires rather than deciding it here.

## Gate closure — P7.30 (2026-08-20)

**Method:** every clause below was checked against the exact working tree at the head of `phase5-corrections` (P7.29's commits, `dc72f7f` and earlier), by reading the actual production call sites — not by re-reading step write-ups — plus running `provider-corpus-check.py --selftest`, `phase7-gate-smoke.sh --synthetic`, and `cargo nextest run --workspace`. Codex reviews this from scratch per this step's own instruction; what follows is Claude Code's own read, reported plainly rather than summarized as a pass.

### Six families: created, launched, version-changeable, archivable, diagnosable

| Family | Created | Launch shape | Version-changeable | Archivable | Diagnosable |
|---|---|---|---|---|---|
| Vanilla | holds — smoke + P7.28 real evidence + iOS walkthrough | holds — `-jar paper.jar --nogui`, proven synthetic and real | holds — `change_download_and_go` via staged download | holds — `archive_jar` (P7.15) | see "Startup diagnostics" finding below |
| Paper | holds | holds | holds | holds | see below |
| Purpur | holds | holds | holds | holds | see below |
| Fabric | holds | holds | holds — `change_fabric` | holds | see below |
| NeoForge | holds — smoke + P7.28 + iOS walkthrough (`neoforge1`) | holds — `@<args-file> nogui`, proven synthetic, real-network, and real-iOS | holds — `change_neoforge`, installer re-run | **N/A by design, not a gap** — `archive_jar` never archives install-step flavors; no such mechanism exists in the MSC 1 oracle either (P7.1's own correction to this file's original wording) | see below |
| Forge | holds — same evidence as NeoForge, plus the imported-raw-Forge case the port plan's later-audit clause names | holds — same | holds — `change_forge` | N/A by design, same as NeoForge | see below |

The "created/launched" and "version-changeable" columns are the best-evidenced part of this gate: synthetic smoke, real-network evidence (`docs/msc2/families/provisioning-evidence/`), and a live iOS walkthrough all agree, independently, for all six families. The port plan's later-audit clause — "Phase 7 must prove non-Paper Java servers are not merely classified but actually launchable with the correct family-specific startup shape" — is answered concretely: `phase7-gate-smoke.sh`'s `import a synthetic raw Forge-shaped server` step launches a Phase-5-imported (not Phase-7-created) Forge directory from its on-disk args file, and the real iOS session started a genuinely-created `forge1` and confirmed its host process argv directly (`ps aux`) matched `@libraries/.../unix_args.txt nogui`.

### Startup diagnostics — does not hold as written

The working exit criteria say: "startup diagnostics turn a real failed boot into attributed problems with repairs that MSC verifies before claiming success." Reading the actual call graph:

- `msc_application::diagnostics::diagnose_unexpected_stop` and `write_last_startup_result` are built (P7.22) and unit-tested, but **are never called from `msc_application::lifecycle`'s real stop path** — confirmed by `grep -rn "diagnose_unexpected_stop" crates/`, which shows every call site is either the function's own definition or a test. `crates/msc-agent/src/routes/health.rs`'s own module doc already flags this plainly ("nothing in this batch calls `diagnose_unexpected_stop`/`write_last_startup_result` from the real `LifecycleService` stop path yet") and attributes it to a Phase 4 `lifecycle.rs` integration no P7 step's `Files:` list names.
- The practical effect: `GET /v1/health/problems` serves real data only when a record already exists on disk (from a test, or a future hook) — a live agent whose Forge server actually crashes on boot today produces **no** attributed problem and no repair option. This is exactly item 8 of the iOS walkthrough ("not exercised... no real startup problem was manufactured"), and this audit's own read of the code shows *why* it couldn't have been exercised even if Cameron had tried: the wiring genuinely isn't there yet, not just untested.
- This was flagged honestly at the time it was built (health.rs's doc comment, `client-capability-matrix.csv`'s own note on `GET /v1/health/problems`) — it is not a silently-introduced gap. But it is a real gap against this phase's own working exit criteria, not a documentation nicety.

**"Diagnosable" holds for the mechanism (attribution logic, repair actions, health-card rendering — all built and fixture-proven) but does not hold for the live path** ("a real failed boot" → "attributed problems") for any of the six families, because nothing calls the attribution function when a real boot actually fails.

### The required-major guard — does not hold

The working exit criteria say: "Java runtime discovery, selection, and the required-major guard gate both creation and start, and report an unusable runtime instead of failing at launch."

`msc_domain::java_runtime::required_java_major`, `compatibility_warning_text`, and `validate_looks_like_java` are ported and unit-tested at the domain layer (P7.12). Checked every call site in the workspace (`grep -rn` for each symbol, excluding `tests/`): **none of the three is called from `msc-application::provisioning` (creation), `msc-application::lifecycle` (start), or any `msc-agent` route.** `crates/msc-application/src/diagnostics.rs::check_java_runtime` (the `GET /v1/health` card) is the only production caller of the related `parse_major`, and that is a passive diagnostic card, not a gate — it is read after the fact, not consulted before a create or a start is allowed to proceed.

Concretely: today, creating or starting a server with an incompatible or missing Java runtime is not refused by MSC 2 at all. It will fail however the JVM itself fails on a bad launch (a raw process-spawn error or a JVM version-mismatch stack trace), not with the "unusable runtime" typed report this criterion promises. No production code path returns anything resembling `UnusableRuntime` (confirmed absent by search). This gap was not flagged as deferred anywhere in P7.12's, P7.16's, P7.17's, or P7.18's own "Actual result" write-ups — it appears to have fallen through the domain/application boundary rather than being a deliberate scope call.

### Rollback — holds for synchronous failures, one known gap for async ones

"Every failed create rolls its directory back completely, leaving no half-provisioned server behind" holds for the two failure modes the smoke actually injects (download failure, installer failure — both proven to leave no directory). It does **not** hold for the async case already flagged in the 2026-08-19 amendment and left for this step to decide: an agent killed mid-Forge/NeoForge-install and restarted has its *operation* correctly reconciled to `failed` (`LifecycleOperations::reconcile_on_startup`, confirmed — journal-only), but the half-written server directory itself is never swept; `reconcile_on_startup` touches only the operation journal, not the filesystem or the server registry. Per Cameron's own 2026-08-19 call, this step's job was to accept this as documented residual behavior or scope a follow-up — recorded here as **still open, not decided by this report**; see "Questions" below.

One smaller, related, previously-flagged-and-never-closed item: `phase7-scope.md`'s own P7.1 note recommended closing "the folder-name check-then-create race" (`fs.stat` then `fs.create_dir_all`, not an atomic claim) rather than reproducing it. Reading `provisioning.rs:645-652` today, the check-then-create shape is still exactly what was flagged — never closed by P7.17/P7.18. Narrow (only two concurrent creates of the identical server name), but real and still open.

### Everything else in the working exit criteria — holds

- **Version listing, version change, jar archiving through the staged-download path**: `server_versions.rs::change_version` composes `jar_provider.rs` functions for every family (verified by reading `change_download_and_go`/`change_fabric`/`change_neoforge`/`change_forge`), all of which route through P7.13's `Transport` + `download_staging::stage_download`. Holds.
- **Provider outages / malformed catalogs / absent networks degrade honestly**: `routes/versions.rs::fetch_versions_response` returns an empty version list with a `note` string on any transport or join error — never a fabricated list. Holds.
- **Bedrock creation refused, not faked**: `routes/servers.rs:1168` returns `capability_unavailable` before any directory is touched. Holds. Confirmed live in the iOS walkthrough (item 3).
- **Deferrals** (components route, Geyser/Floodgate downloads, help-content resolver, the other three health cards, desktop/web screens, modpack-driven creation, Spigot/Quilt/Pufferfish exclusion from the create-flow catalog): every one checked against the actual code (no `routes/components.rs`, no `downloadLatestGeyser/Floodgate` call, no `/v1/help/{helpId}` route, `not_yet_implemented_card` used for the other four health cards, no desktop/web route implementations, no modpack import source). All still true, still honestly advertised (matrix/route absence matches documentation), still owned by their named later phase. Holds.

### Verdict

**This gate does not fully close as written.** Four of its clauses hold with strong, independently-cross-checked evidence (creation, launch shape, version change, honest degradation, the Bedrock refusal, and every deferral). Two do not: the required-major guard does not gate creation or start in production code at all, and startup diagnostics cannot attribute a real failed boot because nothing calls `diagnose_unexpected_stop` outside a test. A third — orphaned-directory cleanup after a mid-install kill — is a known, smaller gap Cameron already asked this step to explicitly resolve one way or the other, not silently carry forward again.

Per this step's own instruction, this report fixes nothing and closes nothing; Codex reviews Phase 7 next, since Claude Code planned and built it.

### Questions for Cameron

```
QUESTION — How should the two unwired Java-runtime/diagnostics gaps be closed, and what happens to the orphaned-directory gap?

What it is:      Two things this phase promised are built but not connected to the live
                 path: (1) MSC 2 doesn't actually check whether a server's Java runtime
                 is compatible before creating or starting it — an incompatible JVM just
                 fails however it fails, instead of MSC 2 catching it first with a clear
                 message. (2) When a server's boot genuinely fails, nothing currently
                 writes down what went wrong, so the Health screen's problem list and
                 repair button stay empty even for a real crash. A third, smaller item:
                 if MSC 2 is killed mid-install and restarted, the operation is correctly
                 marked failed, but the half-installed server folder itself is left on
                 disk rather than cleaned up.

The choice:      (a) Treat all three as new Phase 7 steps (P7.31+) before the gate closes
                     — wire the guard into create/start, wire the real stop path into
                     diagnostics, and sweep orphaned directories on reconciliation.
                 (b) Close Phase 7 now with these three recorded as known, load-bearing
                     gaps, and pick them up explicitly in a later phase or a dedicated
                     hardening pass.

Why it matters:  (a) keeps Phase 7's own promises fully true before the gate closes, at
                 the cost of more steps before Phase 8 starts. (b) lets Phase 8 begin
                 sooner, but ships Phase 7 with two of its named working exit criteria
                 not actually true in the running agent — someone hits a real crash or a
                 wrong-Java server today and gets a raw failure, not the attributed
                 report/refusal this phase advertises.

If unsure:       (a) for the two unwired guards — they're small, self-contained wiring
                 gaps against code that already exists and is already tested (the guard
                 is one call in `provisioning.rs`/`lifecycle.rs`; the diagnostics hook is
                 one call in the real stop path), not new design. (b) is more defensible
                 for the orphaned-directory sweep specifically, since Phase 6 built a
                 comparable reconciler for world activation/restore as its own scoped
                 step rather than folding it into whatever step found the gap — the same
                 precedent would suggest a dedicated step here too, not a rushed fix.
```

### Re-check — P7.34 (2026-08-20)

Cameron answered P7.30's question with (a): P7.31, P7.32, and P7.33 closed the three named gaps as their own steps. This addendum re-runs P7.30's own three failing clauses against the working tree at the head of `phase5-corrections` (through P7.33's commits, `b305cb8` and earlier) — reading production call sites directly, the same method P7.30 used, not re-reading P7.31–P7.33's own "Actual result" write-ups as if they were proof. The rest of the original report (six-family table, rollback for synchronous failures, "everything else holds") is unaffected by P7.31–P7.33 and is not re-checked here.

**The required-major guard now holds.** `grep -rn "evaluate_java_runtime_guard" crates/` (excluding `tests/`) shows three real production callers, not zero: `crates/msc-application/src/provisioning.rs:218` (Forge/NeoForge, before the installer is spawned), `crates/msc-agent/src/routes/lifecycle.rs:823` (start, after resolving the server's configured Java path), and `crates/msc-agent/src/routes/servers.rs:1478` (the download-and-go families' post-stage, pre-registration check). All three route through the same domain function P7.30 found unreachable. A missing or below-required-major Java runtime is now refused with a typed `unusable_java_runtime` error before creation or start proceeds, not left to fail however the JVM fails.

**Startup diagnostics now hold for the mechanism this report can check.** `grep -rn "diagnose_unexpected_stop"` shows its one production caller is `crates/msc-application/src/lifecycle.rs:449`, inside a new private `record_stop_diagnostics` called from `mark_process_exited` (`lifecycle.rs:390`) — and `mark_process_exited` is itself reached by `handle_process_event` (`lifecycle.rs:472`), which `crates/msc-agent/src/routes/lifecycle.rs:953` calls from the real process-event pump every process exit already flows through. A real failed boot now writes an attributed record, not just a test-primed one. One limitation P7.32 itself flagged and this re-check did not re-derive: no mod-jar directory scanner exists yet, so a modded crash's `installed_mods` is always empty, which loses jar-stem attribution for the disable/delete repair action (not missing-dependency detection itself, which a dedicated P7.32 test already proved works from the console excerpt alone). Still a real, open, honestly-flagged gap — narrower than "nothing is attributed," which is what P7.30 found.

**The orphaned-directory sweep now holds, confirmed two ways, not just read.** First, by code: `OperationsState::default()` (`crates/msc-agent/src/routes/operations.rs:178-181`, what `main.rs:82` actually constructs) delegates to `default_journaled()`, which passes `Some(default_servers_root())` — the real, non-test path — enabling `LifecycleOperations::reconcile_on_startup`'s sweep. Second, independently, by running it: this re-check built the real `msc`/`msc-agent` binaries, ran a modified copy of `phase7-gate-smoke.sh` with one added assertion (`[[ -e "${KILL_DIR}" ]]` after the existing kill-mid-install-and-restart sequence), and confirmed the directory is gone after restart — not just that the operation journal reconciles, which is all the committed script currently checks. The committed `tools/phase7/phase7-gate-smoke.sh` itself was not changed (out of this step's own `Files:` list); see the amendments log for that as a deliberately-not-acted-on item.

**The folder-name check-then-create race is also closed**, though it wasn't one of P7.30's two headline gaps — P7.33's own scope covered it alongside the sweep. `provisioning.rs`'s `claim_new_server_directory` now uses `FileSystem::create_dir_exclusive`, proven non-racing by a 16-iteration two-thread test (`provisioning_concurrent_creates_of_same_name_never_both_succeed`).

**Verdict: the gate now holds.** All three of P7.30's real, load-bearing gaps — the unwired Java guard, the unwired diagnostics hook, and the orphaned-directory sweep — have real production callers, confirmed by reading the call graph directly rather than trusting prior write-ups, plus one independent manual reproduction for the sweep specifically. Full verification for this re-check: `python3 tools/phase7/provider-corpus-check.py --selftest` (15/15), `bash tools/phase7/phase7-gate-smoke.sh --synthetic` (green, unmodified script), `cargo nextest run --workspace` (1123/1123), `cargo fmt --all -- --check` (clean), `cargo clippy --workspace --all-targets -- -D warnings` (clean). Per this step's own instruction, this addendum decides nothing about process — Codex reviews Phase 7 next, per P7.30's own instruction, unchanged by this re-check.
