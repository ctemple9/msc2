# MSC 2 — Rolling Plan

> ## STATUS: Phase 6 is planned. Phase 5 is complete; Phase 6 execution has not started.
> **Next move:** Read — Cameron reviews the Phase 6 step list before any code is written.
> **Repo:** https://github.com/ctemple9/msc2 · the last checked Phase 5 candidate is commit `d229192`, with GitHub Actions run [`31757826552`](https://github.com/ctemple9/msc2/actions/runs/31757826552) green on macOS, Linux, Windows, repo invariants, and the D-021 headless check. P4.43 also records macOS/Linux/Windows real-service credential persistence evidence.
> **Last updated:** 2026-08-13

**Previous phases (Setup, Phase 0 through Phase 5) and their amendments log have moved to `rolling-plan-archive.md`** to keep this file small. That archive is historical only — current status, active work, and every amendment from Phase 6 onward stay here.

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
| **6** | Worlds and backups | planned |
| 7 | Server families and provisioning | not started |
| 8 | Mods, plugins, modpacks | not started |
| 9 | Networking and helpers | not started |
| 10 | Bedrock runtimes | not started |
| 11 | Desktop and web clients | not started |
| 12 | Terminal UI (deferred from v1) | not started |

---

## Phase 6 — Worlds and backups

**Gate** (`msc2-port-plan.md` §3): "World discovery, slots, transactional mutations, backups, retention, verification, restore." Phase 6 must also satisfy the P5.33 amendment: audit and reconcile Phase 5's imported live-world and `world_slots` data before world mutations become authoritative.

**Working exit criteria:** a Phase 5-imported Java server with only live world folders, only a copied slot archive, or both can enter the formal slot model without discarding either source; active-slot resolution and every slot mutation reproduce the characterized MSC 1 behavior; archive traversal, symlink escape, partial rename/copy, interrupted activation, and interrupted restore leave the last known-good world recoverable; manual and scheduled backups capture all Java dimension folders, coordinate safely with a running server, resume saves on every exit path, verify before being reported as backups, retain at least one known-good recovery point, and restore only after a mandatory safety backup; the frozen API, CLI, and copied iOS client exercise the same operations; the real local world/backup corpus passes without mutation; macOS, Linux, and Windows CI pass. Bedrock file layouts and pure policies are covered now, but any workflow that requires a live Bedrock runtime stays unavailable until Phase 10 and advertises that honestly.

**Source oracle:** MSC 1 at `~/Documents/Swift Projects/minecraft-server-controller`, read-only. Primary files: `WorldSlotManager.swift` (slot model, active resolution, archives, metadata, NBT), `AppViewModel+WorldSlots.swift` (slot orchestration), `AppViewModel+WorldManagement.swift` (rename/replace rollback), `AppViewModel+Backups.swift` (creation, online consistency, metadata, retention, restore), `AppViewModel+WorldConversion.swift` (Chunker workflow), `AppViewModel+WorldRepair.swift` (Bedrock runtime-dependent repair), `AppViewModel+APIWiringWorlds.swift`, `AppViewModel+APIWiringBackupsHealth.swift`, `AppViewModel+APIWiringSettings.swift`, and the copied iOS `WorldsView.swift`/`RemoteAPIClient.swift`/`RemoteAPIModels.swift`.

28 steps, eight groups:

| Group | Steps | Deliverable |
|---|---|---|
| Scope and evidence | P6.1–P6.3 | confirmed boundary, self-tested corpus checker, real world/backup evidence |
| Characterization and contract | P6.4–P6.8 | destructive-workflow fixtures, reconciliation rule, full Phase 6 API and capability rows |
| World model and transactions | P6.9–P6.14 | records/NBT, safe archive store, import reconciliation, CRUD, activation, rename/replace |
| Backups and recovery | P6.15–P6.18 | inventory/config, verified creation, scheduling/retention, transactional restore |
| Conversion | P6.19 | restart-safe conversion behind an injected Chunker boundary |
| Public clients | P6.20–P6.24 | routes/operations, CLI, and iOS world/backup workflows |
| Public-path and real-corpus proof | P6.25–P6.27 | restart-sensitive smoke, real evidence run, tri-platform CI |
| Phase exit | P6.28 | literal gate check |

**Planned batch ranges:** after their preceding solo characterization/contract step is verified, `P6.9–P6.11`, `P6.12–P6.14`, `P6.15–P6.18`, `P6.20–P6.21`, and `P6.22–P6.24` may each run as one BATCH EXECUTE conversation. Every `stop-after` step ends its range. No batch crosses a failed Verify.

**Not in this phase**, deferred on purpose:

- **Bedrock `level.dat` repair and production online-backup command delivery** stay Phase 10 because both require a real Bedrock runtime. Phase 6 ports the file-layout/NBT rules and fake-runtime protocol tests, and returns an explicit capability-unavailable error for imported Bedrock records rather than pretending the operation ran.
- **Provisioning a new server from a backup** (`duplicateBackupToNewServer`) stays Phase 7 with server-family provisioning. Phase 6 can restore a backup into the current server or import it as a world slot; it does not construct a new runtime.
- **Installing or updating Chunker** is not folded into world mutation. Phase 6 defines and exercises the converter process boundary and uses an already-installed executable; helper acquisition belongs with later helper/provisioning work. An absent converter is an advertised unavailable capability, not an implicit download.
- **Desktop/web screens** stay Phase 11. Their cells are `Planned` in the capability matrix; that is not an exception. The copied iOS client and CLI are the Phase 6 client surfaces.
- **Arbitrary host filesystem browsing** remains outside the world API. Import/upload and export/download use bounded, operation-scoped staging under approved roots rather than accepting an unrestricted server-side path from a remote client.

---

### Scope and evidence

### P6.1 — Scope Phase 6 and decide the imported-world reconciliation rule
**Status:** DONE
**Files:** `docs/msc2/worlds/phase6-scope.md`, `docs/msc2/config-migration/phase5-scope.md`
**What:** Read the Phase 5 import implementation and real package layout beside MSC 1's slot manager, then write the authoritative reconciliation rule for the three starting states: live folders only, `world_slots` only, and both together. Preserve Phase 5's established live-world precedence without overwriting a distinct copied slot archive: inventory both, identify the recorded active slot, create a recovery snapshot when the live data differs or cannot be proven identical, and only then persist the formal active marker. Record every symbol-ledger row owned here, the Bedrock/Phase 7/Phase 10 deferrals above, and the working gate. This is a design record, not Rust code.
**Verify:** `python3 -c "from pathlib import Path; s=Path('docs/msc2/worlds/phase6-scope.md').read_text(); required=['live folders only','world_slots only','both together','recovery snapshot','Bedrock','Phase 7','Phase 10']; missing=[x for x in required if x not in s]; assert not missing, missing"`
**Commit:** `P6.1: scope Phase 6 world and backup authority`
**Batch:** solo

### P6.2 — Build the Phase 6 corpus and gate checker first
**Status:** DONE
**Files:** `tools/phase6/corpus-check.py`, `tools/phase6/fixtures/`, `corpus/worlds/README.md`, `corpus/backups/README.md`
**What:** Build a dependency-free checker before evidence is collected. Inventory mode requires provenance, hashes, a Java multi-folder world, at least one real MSC 1 `world_slots` tree with metadata/active marker/archive, and at least one real backup ZIP plus any adjacent `.meta.json`; optional Bedrock evidence is reported separately and never fabricated. Exercise mode is added later by P6.26. Passing and deliberately failing self-tests prove missing provenance, duplicate hashes, malformed metadata, unsafe archive entries, and mutated inputs fail loudly.
**Verify:** `python3 tools/phase6/corpus-check.py --selftest`
**Commit:** `P6.2: build the Phase 6 corpus checker`
**Batch:** solo

### P6.3 — Collect real MSC 1 world and backup evidence
**Status:** DONE
**Files:** `corpus/worlds/`, `corpus/backups/`, `corpus/worlds/README.md`, `corpus/backups/README.md`, `tools/phase6/corpus-check.py`, `tools/phase6/fixtures/no-dimension-evidence/`
**What:** Inventory the real world-slot and backup material already present in Cameron's MSC 1 installation and the real `.msctransfer` package used in Phase 5. Commit only small sanitized structural evidence whose player/world data can be removed without changing layout, metadata keys, archive member names, or dimension relationships; keep large/private archives outside git behind environment paths. Record source, sanitization, byte size, and SHA-256. If the required Java slot/backup evidence is unavailable, stop instead of inventing it.

**Actual result:** An initial thorough search (both MSC 1-managed Java servers, an older unmanaged copy of the same modpack, Desktop/Downloads, local Time Machine snapshots) found real `world_slots/` metadata but every real slot archive-less and no real backup anywhere. Cameron chose to generate the missing evidence live rather than relax the checker's bar: MSC 1's real **Back Up** and **Save Current World** actions, run against both `campack` and `paper`, 2026-08-13 22:29. Real evidence is staged in `corpus/worlds/` and `corpus/backups/` — two real live Java worlds, one real archived `world.zip` slot, and two real backup zips, each hashed and provenance-recorded in a committed `manifest.json`; the actual bytes are git-ignored (`.gitignore` in each directory) since they carry real per-player NBT data, matching how `$MSC2_PHASE5_TRANSFER_PACKAGE` kept the Phase 5 transfer package out of git. This closed two of the three original gaps (archive-less slots, missing backups). The third didn't close by generating fresh evidence, because it was structural, not a missing-sample problem: neither real world has a `<name>_nether`/`<name>_the_end` sibling directory next to `level.dat` — `campack` is Fabric, whose vanilla world format nests dimensions inside the main world folder (`DIM-1`/`DIM1`) and can never produce sibling folders, and `paper` uses a newer nested `Paper/dimensions/minecraft/{overworld,the_nether,the_end}/` layout instead of the classic sibling convention `WorldSlotManager.swift`'s multi-folder assumption was written against. Asked Cameron, who chose to relax the checker rather than chase evidence for a layout neither real server produces (P5.3 precedent: relaxing an unmeetable evidence bar once real data proves it wrong, not weakening the gate arbitrarily). `tools/phase6/corpus-check.py`'s `check_worlds_structure` now accepts any of three real shapes — classic sibling folders, vanilla/Fabric nested `DIM-1`/`DIM1`, or current-PaperMC nested `dimensions/minecraft/the_nether`/`the_end` — and a new self-test fixture, `tools/phase6/fixtures/no-dimension-evidence/`, pins that a world with none of the three still fails, so the relaxation didn't quietly turn the check into a no-op. Full detail in `corpus/worlds/README.md`'s "P6.3 real evidence collected" section.
**Verify:** `python3 tools/phase6/corpus-check.py --selftest && python3 tools/phase6/corpus-check.py --inventory --worlds corpus/worlds --backups corpus/backups`
**Commit:** `P6.3: collect real MSC 1 world/backup evidence and relax the checker's dimension-layout bar`
**Batch:** stop-after

---

### Characterization and contract

### P6.4 — Characterize world slots and Phase 5 import reconciliation
**Status:** DONE
**Files:** `fixtures/world-slots/`, `fixtures/world-import-reconciliation/`, `docs/msc2/worlds/phase6-scope.md`
**What:** Capture MSC 1's slot metadata/defaults, tolerant corrupt-entry loading, newest-first ordering, explicit-active → most-recently-played → newest-created fallback, Java/Bedrock level-name rules, fresh archive-less slots, and initial-slot bootstrap. Add the Phase 5 handoff matrix: raw live folders only, copied slots only, live plus matching slot, live plus stale/different active slot, missing/corrupt marker, corrupt slot metadata, and no world data. Expected results must preserve both recoverable sources and follow P6.1's reviewed authority rule.

**Actual result:** Read `WorldSlotManager.swift` and `AppViewModel+WorldSlots.swift` directly (no dedicated MSC 1 XCTest file exists for either — `source.test` in each fixture names the function characterized, per the pattern already used by `config-recovery` and `transfer-package`). `fixtures/world-slots/` (12 cases) covers: `WorldSlot` JSON decode defaults for absent optional fields; `loadSlots`'s tolerance of a non-directory entry, a missing `slot.json`, and an unparseable `slot.json` in the same pass; its newest-first sort independent of directory-enumeration order; its missing-`world_slots/`-returns-empty guard; all three links of `resolvedActiveSlotID`'s fallback chain (explicit marker wins over a more-recent `lastPlayedAt`; an explicit marker naming a since-deleted slot falls through to most-recently-played; with no slot ever played, falls through again to newest-`createdAt`) plus the empty-slots-returns-nil base case; `sanitizedWorldLevelName`'s invalid-character stripping (including the `=`-padding Realm-export case the function exists to fix); `currentLevelName`'s distinct Java/Bedrock fallback strings; `createFreshWorldSlot`'s archive-less construction and seed normalization; and `ensureActiveWorldSlotExists`'s from-nothing bootstrap path (the one slot-creation path where `lastPlayedAt` is set at creation instead of left `nil`). `fixtures/world-import-reconciliation/` (8 cases) exercises every state in `docs/msc2/worlds/phase6-scope.md`'s reconciliation rule: State 1 (live-only, archived as a new active slot); State 2 split into its two real branches (archived resolved slot → extracted into place; archive-less resolved slot → marker persisted with nothing materialized); State 3's three outcomes (proven-identical → no new slot; different/unproven → recovery snapshot becomes active while the old slot survives inactive; every `world_slots/` entry corrupt so resolution finds nothing → treated as State 1 without deleting the unresolvable slot data); a State-2 case where `loadSlots`'s per-entry tolerance recovers one valid slot out of three corrupt entries; and the no-data-at-all no-op. Every reconciliation fixture's `source` points at the specific MSC 1 function Phase 6 reuses (per phase6-scope.md's own mapping) since the reconciliation rule itself is new Phase-6-only logic, not a direct MSC 1 port — each fixture's `notes` cites the exact phase6-scope.md section it pins. `docs/msc2/worlds/phase6-scope.md` itself was read for the authority rule but not edited; nothing in this step required amending it.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/world-slots --expect 12 && python3 tools/fixture-runner/run.py --validate-dir fixtures/world-import-reconciliation --expect 8`
**Commit:** `P6.4: characterize slots and imported-world reconciliation`
**Batch:** solo

### P6.5 — Characterize transactional world mutations and hostile archives
**Status:** DONE
**Files:** `fixtures/world-mutations/`, `fixtures/world-archive-safety/`
**What:** Characterize slot create/update/rename/delete/duplicate/copy/import/export, activation, direct world rename/replace, and rollback after each injected rename/copy/delete/extract failure. Cover Java's main/nether/end folder set, Bedrock's `worlds/<level-name>` layout, fresh-slot activation, wrong/running-server guards, mandatory pre-activation backup, legacy ZIP layout relocation, partial activation recovery, traversal, absolute paths, Windows path forms, symlink entries, corrupt ZIPs, and extraction limits. Record deliberate security corrections against MSC 1's shell-based ZIP handling as D-006 corrections rather than oracle parity.

**Actual result:** Read `WorldSlotManager.swift` and the two relevant slices of `AppViewModel+WorldSlots.swift`/`AppViewModel+WorldManagement.swift` directly (no dedicated MSC 1 XCTest file exists for either, same as P6.4). `fixtures/world-mutations/` (20 cases) covers all eight slot CRUD verbs (create, twice — Java's three-folder zip vs. Bedrock's single `worlds/` folder, plus a zip-process-failure rollback that cleans up the slot directory; update, via its zip-failure branch that leaves the previous archive untouched thanks to the temp-file-then-atomic-move pattern; rename, metadata-only with no file I/O; delete, via the active-slot refusal guard that lives in the orchestration layer, not `WorldSlotManager`; duplicate, fresh-UUID with the source left untouched; copy-into-existing, via its own temp-file-then-atomic-move rollback; import-from-ZIP, pinning MSC 1's documented "no structural validation enforced here" baseline and pointing at where the correction actually lives; export, which overwrites an existing destination file). Activation gets six cases: the mandatory pre-activation backup step itself, the backup-failure abort that happens before any folder is touched, fresh/archive-less-slot activation (which still removes the current live folders even though nothing is extracted to replace them), the legacy loose-`worlds/`-root relocation for old Bedrock exports, the dangerous unzip-failure window where the current folders are already gone and recovery depends entirely on the safety backup (not an automatic rollback — MSC 1 has none here), and the running-server guard. Direct world rename/replace gets four: rename's all-or-nothing pre-check across all three target names before any move, rename's `rollbackMovedFolders()` reversing a mid-sequence move failure, replace's folder-removal failure aborting before the new source is ever extracted or copied, and replace's own running-server guard (same shape as activation's and rename's, three independent copies of one check in MSC 1). `fixtures/world-archive-safety/` (10 cases) characterizes the corrected extractor Phase 6 must build rather than any existing MSC 1 behavior, since MSC 1 has none — `createSlotFromZIP`'s doc comment states plainly that no structural validation is enforced, and every extraction call (`activateSlot`, `validateZipArchive`/`unzipWorldBackup`) shells out to `/usr/bin/unzip` with no entry-path, entry-type, or size inspection. Each fixture's `notes` states explicitly that it is a D-006 correction, not oracle parity, and cites the specific unsafe MSC 1 call site being corrected: relative-path traversal, an absolute-path entry, a Windows drive-absolute entry, a Windows backslash-traversal entry, a symlink entry pointing outside the target root, a symlink entry rejected outright regardless of target (world archives never legitimately contain symlinks), a corrupt ZIP whose central directory doesn't match its local file data (replacing MSC 1's black-box `unzip -t` trust with an auditable Rust structural check), a declared-uncompressed-size zip-bomb, a declared-entry-count zip-bomb, and one positive control case proving an ordinarily-shaped world archive still extracts normally through the corrected path (without it, none of the nine rejection cases would demonstrate the checks are correctly scoped rather than a blanket refusal).
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/world-mutations --expect 20 && python3 tools/fixture-runner/run.py --validate-dir fixtures/world-archive-safety --expect 10`
**Commit:** `P6.5: characterize transactional world mutations`
**Batch:** solo

### P6.6 — Characterize backup creation, retention, verification, and restore
**Status:** DONE
**Files:** `fixtures/backups/`, `fixtures/backup-online-consistency/`, `fixtures/backup-restore/`
**What:** Capture listing/display-name/meta-sidecar compatibility, association with the active slot, manual/auto/pre-mutation naming, config interval fallback and max-count clamp, pruning only MSC-managed files, and the no-players scheduled-backup skip. Cover Java `save-all flush` → `save-off`, timeout-as-best-effort, unconditional `save-on`, archive/write/meta failures, verification before visibility, failed/interrupted restore, mandatory safety-backup ordering, cross-slot and running-server guards, and retention when only one verified backup remains. Where Phase 6 strengthens MSC 1 by retaining a last known-good verified backup or rolling back an interrupted restore, mark the correction explicitly.

**Actual result:** Read `AppViewModel+Backups.swift` directly (997 lines; no dedicated MSC 1 XCTest file exists for backups either, same pattern as P6.4/P6.5), plus the auto-backup timer/no-players guard in `AppViewModel+ServerControls.swift` and the interval-default/max-count-clamp evidence in `AppConfig.swift` and `ServerEditorBackupsTab.swift`. `fixtures/backups/` (16 cases) covers: empty/missing backups directory, zip-extension filtering with newest-first sort, all three `makeDisplayName` branches (new auto/manual token format, legacy dash-suffix format, unparseable-suffix raw fallback), sidecar-present-overrides-filename-default and sidecar-missing-or-corrupt-leaves-default (`readBackupMeta`'s silent-nil contract), `effectiveBackupAssociation`'s explicit-slot-id-wins vs. falls-back-to-active-slot branches, manual/auto filename-token-and-trigger-reason pairing, the pre-replace backup's deliberate no-token/unprunable naming (`backupWorld`, distinct from `createBackup`), `autoBackupIntervalMinutes`'s 30-minute decode default, the editor Stepper's UI-only `3...50` clamp (not enforced by the model), `pruneAutoBackupsIfNeeded`'s oldest-first deletion down to `maxCount - 1` plus orphaned-sidecar cleanup, and the auto-backup timer's per-tick no-players skip. `fixtures/backup-online-consistency/` (10 cases) covers `pauseSavesForBackup`'s Java (`save-all flush` → `save-off`, confirmation-observed and timeout-as-best-effort, both-sends-fail skips the pause) and Bedrock (`save hold` → polled `save query` until "ready to be copied", timeout-as-best-effort) branches, `resumeSavesAfterBackup`'s unconditional `save-on` resend and its own independent running-server re-check that can skip resume even when the pause happened, a nonzero zip exit status failing the backup while saves are still unconditionally resumed, and a sidecar-write failure being logged as a non-fatal warning. `fixtures/backup-restore/` (12 cases) covers `restoreBackup`'s four refusal guards in source order (Bedrock-unsupported, running-server, cross-slot, missing-source-file), the mandatory pre-restore safety backup and its own hard-abort-on-failure, `validateZipArchive` running before `removeWorldFolders` with an abort-leaves-world-untouched case, and a positive-control successful restore. Three fixtures in this domain are explicit D-006-style Phase 6 corrections (not oracle parity), each naming exactly what MSC 1 lacks: MSC 1 removes world folders unconditionally before extracting with no rollback if `unzip` then fails (Phase 6 auto-restores the just-made safety backup); MSC 1 treats a zero zip exit status as sufficient to make a backup visible/restorable with no structural check at creation time (Phase 6 reuses P6.5's archive-safety check before visibility); and MSC 1's count-based pruning has no floor against deleting the sole remaining verified backup (Phase 6 adds one). `cargo fmt`/`cargo clippy` not applicable — no Rust exists yet for this domain, matching P6.4/P6.5's own schema-only verify.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/backups --expect 16 && python3 tools/fixture-runner/run.py --validate-dir fixtures/backup-online-consistency --expect 10 && python3 tools/fixture-runner/run.py --validate-dir fixtures/backup-restore --expect 12`
**Commit:** `P6.6: characterize backup and restore safety`
**Batch:** solo

### P6.7 — Characterize world metadata and conversion
**Status:** DONE
**Files:** `fixtures/world-nbt/`, `fixtures/world-conversion/`
**What:** Extract real small `level.dat` samples where sanitization preserves binary shape, then characterize the minimal NBT reader: compressed big-endian Java, headered little-endian Bedrock, every tag type the parser accepts, key-path fallbacks, seed/difficulty/gamemode/day-time extraction, ZIP member selection, and adjacent backup metadata precedence. Characterize conversion guards, nested-world discovery, temp cleanup, converter arguments, output packaging, new-slot versus replace-slot placement, mandatory target backup, atomic archive replacement, and failure after each stage. Do not characterize client navigation state.

**Actual result:** Read `WorldSlotManager.swift`'s NBT section (lines 1084-1493; no dedicated MSC 1 XCTest file exists for this either, same pattern as P6.4-P6.6) plus `ChunkerManager.swift` and `AppViewModel+WorldConversion.swift` in full. For the real-sample requirement, parsed both real level.dat files already staged locally by P6.3 (`corpus/worlds/campack/level.dat`, `corpus/worlds/Paper/level.dat`, git-ignored, never modified in place) with a from-scratch Python NBT reader mirroring `WorldSlotManager.swift`'s algorithm byte-for-byte, confirmed both real files round-trip through it, then re-serialized ONLY the Data-compound keys the Swift extractors actually inspect (GameType, Difficulty, DataVersion, Time, DayTime, LevelName, WorldGenSettings.seed / difficulty_settings) into two new minimal gzip-compressed NBT files with LevelName replaced by a placeholder — same binary shape (valid gzip, big-endian NBT, root Data compound), every kept value and its original NBT tag type genuinely read off the real bytes, everything else (mod generator subtrees, spawn coordinates, version strings) dropped. These carry no player data (a multiplayer server's level.dat has none — player state lives in `playerdata/`, untouched) and are committed at `fixtures/world-nbt/samples/` (144 and 142 bytes). The two real samples turned up a genuine, non-obvious finding: `campack` (older DataVersion 3465, Fabric) has every legacy field (`Data.Difficulty` int, `Data.WorldGenSettings.seed`, `Data.DayTime`) present and extracts cleanly, while `Paper` (current 2026 PaperMC, DataVersion 4903) has NEITHER a legacy `Data.Difficulty` tag NOR any seed field under `Data` at all — difficulty moved to a string under `Data.difficulty_settings.difficulty`, and the seed isn't stored under `Data` in any form `extractSeedString`/`findInteger` would recognize. `extractSeedString`/`extractDifficultyString` genuinely return `nil` against this real, current server; `extractDayTime` falls through its Java-preferred `Data.DayTime` (absent) to `Data.Time` (present). `fixtures/world-nbt/` (14 cases) pairs these two real fixtures with synthetic characterization (grounded in Swift source, same as every prior P6.4-P6.6 case) of: gzip-failure-before-parse vs. malformed-NBT-after-gunzip vs. non-compound-root as three distinct failure points; the Bedrock 8-byte little-endian header detection and its unheadered fallback (no real Bedrock evidence exists per P6.2's never-fabricate rule, so — like every other fixture domain before Bedrock support lands — this is synthesized from the source, not stood in as real evidence); all twelve NBT tag types round-tripping through the reader; the Java-path seed/dayTime preference order when multiple candidates exist; the recursive `findInteger` fallback; every difficulty/gamemode enum value including the unmapped case; `firstLevelDatPath`'s positional (not shortest-path) ZIP member selection with `__MACOSX` exclusion; and the adjacent `.meta.json` sidecar's seed taking precedence over a parsed level.dat's seed. `fixtures/world-conversion/` (10 cases) covers `performWorldConversion`'s guard order (Java-path-missing checked before jar-not-installed; empty/whitespace slot name rejected before any file I/O; missing source archive aborts before the temp directory even exists), `findInputWorldFolder`'s Java (lexicographically-sorted fallback) vs. Bedrock (unsorted, enumeration-order-dependent fallback) discovery, `cleanup()` running on both the success and every mid-pipeline failure path via `try?`, the exact five-flag Chunker CLI invocation with streamed stdout/stderr and non-zero-exit handling, `packageOutput`'s Java (`{name}/`) vs. Bedrock (`worlds/{name}/`) zip layout and its empty-output refusal, and two real gaps worth flagging rather than silently fixing: `replaceSlotWithConvertedZip` removes the previous archive before copying the new one in (not the temp-file-then-atomic-rename pattern P6.5 found everywhere else in `WorldSlotManager`), so a copy failure mid-replace can leave a slot with no archive at all; and the mandatory pre-conversion target backup only logs a warning on failure and lets conversion proceed (unlike `activateSlot`'s own hard-abort-on-backup-failure guard, characterized in P6.5), while a later activation failure leaves the newly written/replaced slot on disk, unactivated and unreverted. `cargo fmt`/`cargo clippy` not applicable — no Rust exists yet for this domain, matching P6.4-P6.6's own schema-only verify.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/world-nbt --expect 14 && python3 tools/fixture-runner/run.py --validate-dir fixtures/world-conversion --expect 10`
**Commit:** `P6.7: characterize world metadata and conversion`
**Batch:** solo

### P6.8 — Freeze the complete Phase 6 API and capability surface
**Status:** DONE
**Files:** `docs/msc2/worlds/phase6-api.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/client-capability-matrix.csv`, `tools/api-contract-check.py`, `tools/phase6/capability-matrix-check.py`
**What:** Reconcile the frozen baseline routes with the full agent-owned surface. Preserve existing DTO fields and status meanings, add operation IDs additively for activation/backup/restore/conversion, and add the missing slot CRUD/import/export, backup delete, and conversion operations needed so no client is architecturally blocked. Define bounded staged upload/download instead of arbitrary remote paths. Assign permission categories, error/help IDs, cancellation/restart behavior, and capability-unavailable responses for Bedrock-runtime work. Create the overdue D-023 matrix and fill every existing and Phase 6 row for Agent, Desktop/Web, iOS, and CLI; no blank cells or unapproved exceptions.

**Actual result:** Read `crates/msc-agent/src/main.rs::build_app()` and `crates/msc-agent/src/cli/mod.rs` directly to ground Agent/CLI status cells in what's actually wired today, not what's planned — only `health`, `operations` (create/get/cancel/stream), `servers` (list/import), `active-server`, `start`, `stop`, `command`, `status`, `performance`, `settings` (get/post), `capabilities`, and `console` (tail/stream) are real Agent routes; everything else, world/backup domains included, is `Planned` until P6.9–P6.19 build the services behind this contract. iOS status cells are grounded in P2.19's/P4.19's own "Actual result" text (status, servers, active-server, start/stop, command, console tail/stream, performance); CLI cells in the exact `/v1/...` paths `cli/mod.rs` calls. `docs/msc2/worlds/phase6-api.md` records the full reconciliation: six existing world/backup routes kept unchanged (with one naming trap worth flagging — the existing `POST /v1/worlds/rename` is `WorldSlotManager`'s metadata-only slot rename, not `AppViewModel+WorldManagement.swift::renameWorld`'s direct live-folder rename, which had no route until this step); three existing routes (`worlds/activate`, `backups/now`, `backups/restore`) gain an additive optional `operationId` field on their result DTOs, reusing the exact convention P4's `SimpleResult` already established rather than inventing a second one; thirteen new operations close the slot CRUD/import/export/backup-delete/conversion gaps `fixtures/world-mutations`, `fixtures/world-archive-safety`, `fixtures/backups`, and `fixtures/world-conversion` characterized (P6.4–P6.7) but the baseline never exposed, including a bounded staged-upload/staged-download trio (`POST /v1/staged-uploads`, `PUT /v1/staged-uploads/{id}`, `GET /v1/staged-downloads/{id}`) replacing any notion of an arbitrary remote path, and an async-only `POST /v1/worlds/convert` that creates its operation with `operation-model.md`'s already-anticipated `type: "world-conversion"` rather than a fourth bespoke async convention. One new `ErrorDTO.code` — `capability_unavailable` — is recorded (in `phase6-api.md`, not by reopening the Confirmed `versioning-and-errors.md`) for `backups/restore`'s Bedrock-unsupported guard, distinct from D-023's "Intentional exception" concept: this one is a runtime gap that Phase 10 closes on its own, not a client screen needing owner approval. `tools/api-contract-check.py`'s `EXPECTED_TOTAL` moved from 93 to 106 (88 baseline + 5 P2.8 + 13 P6.8); `--selftest` and `--v1-summary` both still pass. `docs/msc2/client-capability-matrix.csv` has one row per `openapi.json` operation (106) plus the two `websocket-v1.json` channels (108 total) — every `desktop_web_status` cell reads `Planned` per the Phase 6 preamble's own rule, and no row uses `Intentional exception` yet, since nothing in the current surface is a client gap needing owner approval rather than a later phase's scheduled work. `tools/phase6/capability-matrix-check.py` (new, self-tested) checks the matrix's shape and its coverage against the real `openapi.json`/`websocket-v1.json` operation set mechanically, so the two can't silently drift apart.

**Verify:** `python3 tools/api-contract-check.py --v1-summary && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P6.8: freeze the Phase 6 world and backup contract`
**Batch:** solo

---

### World model and transactions

### P6.9 — Port world-slot records, identity rules, and NBT metadata
**Status:** DONE
**Files:** `crates/msc-domain/src/world.rs`, `crates/msc-domain/src/nbt.rs`, `crates/msc-domain/src/lib.rs`, `crates/msc-domain/tests/world.rs`, `crates/msc-domain/tests/world_nbt.rs`, `crates/msc-domain/Cargo.toml`
**What:** Port the pure `WorldSlot`/imported-metadata types, active-resolution policy, Java/Bedrock level-name sanitization and dimension-set derivation, backup association policy, and the minimal NBT reader against P6.4/P6.7 fixtures. Keep filesystem/archive/process work out of `msc-domain`.

**Actual result:** `world.rs` ports `WorldSlot` (decode/encode matching source's `CodingKeys`, `zip_size_bytes` excluded from JSON per source), `sort_newest_first`/`resolve_active_slot_id` (the four-step fallback chain read directly off already-loaded slots and an already-read marker, no I/O), `current_level_name`/`sanitized_world_level_name`/`world_folder_candidates` (the candidate-name half of `worldFolderNames`, filtering to what exists on disk stays P6.10's job), and three separate slot-metadata constructors because source itself has three, not because this port invented variety: `build_archived_slot` (mirrors `createSlot` — name untrimmed, `world_level_name` via `current_level_name`, `last_played_at` starts `None`), `build_fresh_slot` (mirrors `createFreshWorldSlot` — name trimmed, `world_level_name` via `sanitized_world_level_name`), and `build_bootstrap_slot` (mirrors `ensureActiveWorldSlotExists`'s from-nothing path — the one path where `last_played_at` is set at creation). `effective_backup_association` ports `AppViewModel+Backups.swift`'s policy (explicit non-blank slot id wins, looked up against already-loaded slots for its seed; otherwise falls back to an already-resolved active slot) even though its own fixture domain (`fixtures/backups/`) isn't characterized until P6.6/built until P6.15-18 — P6.9's own step text names it explicitly, so it's ported now alongside the rest of the slot model and covered by direct unit-style tests in `tests/world.rs` rather than left unported until later.

`nbt.rs` ports `WorldSlotManager`'s private `NBTReader`/`NBTValue` engine (all 12 tag types, big/little-endian, source's exact quirks: `byteArray`'s negative-count hard failure vs. `list`/`intArray`/`longArray`'s negative-count-clamps-to-zero, `readString`'s same clamp) and `extractSeedString`/`extractDifficultyString`/`extractGamemodeString`/`extractDayTime`/`nbtInteger`/`findInteger`. Gzip decompression uses `flate2` in-memory rather than shelling out to `/usr/bin/gunzip` (source's own mechanism) — a new direct dependency, but pure computation over bytes already in memory, not filesystem/process I/O, so it stays in `msc-domain` per the module-boundary rule rather than moving to `msc-infrastructure`. `first_level_dat_path` ports `firstLevelDatPath`'s *selection* rule over an already-obtained `unzip -Z -1` listing (obtaining that listing is I/O, left to a later step). `merge_sidecar_metadata` preserves a real source quirk exactly rather than fixing it: `importedWorldMetadata(fromZIP:)`'s sidecar-priority merge only ever touches `seed`/`difficulty`/`gamemode` (source lines 1265-1267) — a parsed `day_time` is silently dropped by this specific merge path even though the same NBT parse computed one.

Both modules' internal types (`NbtValue`, `NbtReader`, the enum-extraction helpers) are private, matching this crate's established convention (no other `msc-domain` module carries inline `#[cfg(test)]` — every one is tested from `tests/*.rs` against the public API only), so `tests/world_nbt.rs` drives the byte-level reader black-box: each fixture case hand-builds the raw `level.dat`-shaped bytes it describes (a small `be_*`/`le_*` byte-builder local to the test file) and asserts on `imported_world_metadata_from_level_dat`'s result, the same public entry point later I/O-bearing layers will call. All 12 world-slots (P6.4) and 14 world-nbt (P6.7) fixtures are covered — `world_slots_load_slots_missing_directory_returns_empty` and `world_nbt_java_gzip_corrupt_input_fails_before_nbt_parse` cover directory-listing/process-invocation guards that are I/O-shaped in source but whose *domain-visible* content ("no entries in, no slots out" / "gunzip failure ⇒ default metadata") is still exercised through the pure functions here. `cargo fmt`/`cargo clippy --all-targets -- -D warnings` clean; `cargo nextest run -p msc-domain world`: 30 tests, 0 failures (16 world-slots + 14 world-nbt); full workspace build (`cargo build --workspace`) still succeeds.

**Verify:** `cargo nextest run -p msc-domain world`
**Commit:** `P6.9: port world records and metadata rules`
**Batch:** safe

### P6.10 — Build the safe world archive and slot repository
**Status:** DONE
**Files:** `crates/msc-infrastructure/src/world_store.rs`, `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/src/fs.rs`, `crates/msc-infrastructure/tests/world_store.rs`, `crates/msc-infrastructure/tests/world_archive.rs`, `crates/msc-infrastructure/Cargo.toml`
**What:** Implement the `world_slots/{id}/{slot.json,world.zip,thumbnail.*}` repository over approved roots and atomic writes. Load corrupt entries independently, compute sizes without extracting, persist metadata/active markers atomically, apply the fixed thumbnail transform, and create/extract archives with traversal, symlink, entry-count, expanded-size, and destination-bound checks. No destructive live-world swap yet.

**Actual result:** `world_store.rs` ports `WorldSlotManager`'s directory-helper functions (`slots_directory`/`slot_directory`/`zip_path`/`metadata_path`/`active_marker_path`), `loadExplicitActiveSlotID`/`setActiveSlotID` (trim-to-`None`-if-blank; `None` removes an already-absent marker without erroring), and `loadSlots`/`saveMetadata` (tolerant per-entry loading via P6.9's `WorldSlot::decode`, zip-size stat, `sort_newest_first`; atomic-write persistence with key order already alphabetical since no crate in this workspace enables serde_json's `preserve_order` feature, matching source's `.sortedKeys`). "Over approved roots" is upheld the same way `config_repository.rs` already established, not re-implemented here: this module's functions take `server_dir: &Path` directly and trust the caller already resolved it through `path_safety::safe_path` at the API/route boundary, rather than every low-level path-join helper re-deriving that check. `FileSystem` gained a new trait method, `create_dir_all` — no earlier consumer needed to create a directory from scratch (every prior write landed inside an already-provisioned server directory); a brand-new slot's `world_slots/{id}/` is the first real case, so the trait grew the one primitive genuinely missing rather than working around its absence.

`archive.rs` is the D-006 correction `fixtures/world-archive-safety/` characterizes (P6.5): `is_safe_archive_entry_name` rejects traversal/absolute/Windows-drive-absolute entries by splitting on both `/` and `\` regardless of host platform (closing the exact gap flagged against P5's `is_safe_zip_entry_name`, which relies on `Path`'s host-dependent component parsing and would miss a backslash-traversal entry on Unix); any symlink-mode entry is refused outright regardless of target. `extract_zip` runs three passes — declared-metadata checks (entry count, per-entry name/mode, running total declared uncompressed size) against fixed ceilings before any decompression; a dry-run decompression to `io::sink()` that catches a corrupt archive (central directory/local file data disagreement, surfaced as a CRC mismatch) with zero bytes written; then the real extraction — so every rejection reason (unsafe entry, exceeded limit, corrupt archive) is a zero-bytes-written outcome, not a partial one. `ArchiveLimits` factors the two ceilings out of the fixed module constants so tests exercise "exceeded" against a small real archive and a small limit rather than constructing a multi-GB or million-entry zip on disk. `create_zip_from_folders` mirrors `createSlot`'s `zip -r` shape (top-level entries named after each source folder), reusing the same recursive-directory-walk pattern `msc-application::transfer`'s own `add_dir_recursive` already established (deterministic sorted-by-name output, a Rust-side improvement over source's unspecified enumeration order, not a parity requirement).

`saveThumbnail`'s real image resize/JPEG-encode (AppKit-specific, no fixture pins pixel output, and source's own comment marks the field "future use") is deliberately narrowed to its one deterministic, testable half — `thumbnail_dest_size`'s aspect-ratio-preserving bounding-box math — with `save_thumbnail` storing whatever encoded bytes the caller already produced verbatim; decoding/resizing a real image is flagged as a client/UI-layer concern with no fixture-backed reason to take on now, not silently dropped.

`fixtures/world-archive-safety`'s 10 cases are driven by hand-built real zip files (via the same `zip` crate `extract_zip` itself uses) matching each fixture's described shape — including discovering mid-implementation that the `zip` crate's `unix_permissions` alone does not set the symlink file-type bits on `start_file`; `ZipWriter::add_symlink` is the correct API and is what the tests use. `world_store.rs` is tested against `FakeFileSystem` (6 cases; no dedicated fixture domain of its own — the domain-level policy it wires to disk is already fixture-tested in `msc-domain`'s `tests/world.rs`, P6.9). `cargo fmt`/`cargo clippy --all-targets -- -D warnings` clean; `cargo nextest run -p msc-infrastructure -E 'test(/world_(store|archive)/)'`: 16 tests, 0 failures (10 world-archive-safety + 6 world_store; two report nextest's pre-existing, unrelated `LEAK` notice — already seen on this crate's `power` tests before this step, not a failure); full workspace build succeeds.

**Verify:** `cargo nextest run -p msc-infrastructure -E 'test(/world_(store|archive)/)'`
**Commit:** `P6.10: build the safe world-slot store`
**Batch:** safe

### P6.11 — Reconcile Phase 5 imported worlds into the formal slot model
**Status:** DONE
**Files:** `crates/msc-application/src/worlds.rs`, `crates/msc-application/src/lib.rs`, `crates/msc-application/tests/world_import_reconciliation.rs`, `crates/msc-agent/src/routes/lifecycle.rs`
**What:** Implement P6.1's idempotent handoff before any mutation route becomes available. Inventory live folders and copied slots, materialize slot-only legacy imports safely, create an initial slot for raw imports, preserve a distinct live recovery snapshot when both sources differ or equality is unknown, and persist the active marker only after every required archive/metadata write succeeds. A second startup must make no additional changes. Never mutate the original Phase 5 corpus input.

**Actual result:** `msc-application/src/worlds.rs` implements `reconcile_imported_worlds(fs, server_dir, server_type, raw_level_name, now)`, matching phase6-scope.md's rule via a four-way match on `(live_folders.is_empty(), resolved_active_slot)`: empty+`None` → `NoWorldData` (no-op); empty+`Some` → State 2, splitting on `has_archive` into archive-extraction (persisting the active marker, unlike Phase 5's own `restore_active_slot_world` which deliberately didn't) vs. archive-less (marker-only); non-empty+`None` → State 1 (archive live folders as a new active slot); non-empty+`Some(!has_archive)` → treated as State 1 (the archive-less/unresolvable recorded slot is left on disk, untouched); non-empty+`Some(has_archive)` → State 3's file-by-file comparison, branching to either persisting the marker on the existing slot (proven identical) or a recovery snapshot (different/unproven, reusing the same State-1 archiving path). Every "archive live folders as a new slot" branch shares one helper, `archive_live_folders_as_new_active_slot`, built on `msc_domain::world::build_bootstrap_slot` — confirmed against source (`AppViewModel+WorldSlots.swift::createInitialWorldSlotIfNeeded`, line 732-761) that it, not a plain `createSlot` snapshot, is the actual function this bootstrap mirrors: it calls `WorldSlotManager.createSlot` with `defaultPersistentSlotName`, then explicitly sets `lastPlayedAt = Date()` before saving — the exact same two-step shape `ensureActiveWorldSlotExists` uses, which is why P6.9's `build_bootstrap_slot` (not `build_archived_slot`) is reused for State 1, State 3's corrupt-treated-as-State-1 sub-case, and State 3's recovery-snapshot case alike, flagged as this step's own reasoned choice since phase6-scope.md names the mirrored function but not which of P6.9's two builders to use for it.

State 3's "proven, not assumed" comparison (`live_folders_proven_identical_to_archive`) extracts the recorded slot's `world.zip` to a scratch directory outside `server_dir` via `msc_infrastructure::archive::extract_zip`, then fingerprints both trees (relative path, size, and a SHA1 content hash — reusing `msc_infrastructure::download_staging::sha1_hex` rather than adding a new hashing dependency) and compares for exact equality; any failure along the way (corrupt archive, unreadable file) is "equality cannot be established" and falls through to the recovery-snapshot branch, per phase6-scope.md, not a hard `Result::Err` that would abort reconciliation.

**Idempotency** (phase6-scope.md's "Ordering and crash safety" section) uses a dedicated marker, `world_slots/.p6_reconciled`, distinct from `WorldSlotManager`'s own `active_slot_id.txt` — checked first (an already-reconciled server short-circuits to `AlreadyReconciled` with no further reads or writes) and written last, only after every other write for that server has already succeeded. This is the one mechanism the note explicitly left to this step to invent, flagged there as such; a copied-in, MSC-1-native `active_slot_id.txt` that already resolves to something the moment Phase 5 finishes importing is therefore never mistaken for proof that Phase 6's own comparison already ran.

Two scope narrowings, both flagged rather than silent: `read_java_level_name` only reads `server.properties`' `level-name` (no P6.11 fixture names a Bedrock case, and Bedrock's runtime stays unavailable until Phase 10 per this phase's own deferral); and the newly-created bootstrap slot's `zip_size_bytes` is left `None` in the returned in-memory value rather than stat'd immediately after zipping (source does stat it inline) — it self-heals on the next real `world_store::load_slots` read, which always computes it live, so nothing persisted is wrong, only a value this function's own return type doesn't bother computing before handing back.

`crates/msc-agent/src/routes/lifecycle.rs` wires this into `LifecycleRoutesState::with_dependencies` (every construction path: production `new`/`new_migrating_legacy_secrets`/`with_app_config_and_auth`, and the test-only `with_fake_process*` paths) — called once per registered server, before the server registry or `LifecycleService` are constructed, matching "before any mutation route becomes available" (no world-mutation route exists to gate yet; this is the one hook point that will front all of them once P6.12+ builds them). Best-effort per server: a reconciliation failure is logged (`eprintln!`) and does not block agent startup, the same non-fatal-warning shape this file already uses elsewhere. `iso8601_now`/`civil_from_days` are a small, self-contained duplicate of `msc-infrastructure::audit_log`'s own private Howard Hinnant calendar-math helper, formatted without milliseconds to match `WorldSlot`'s actual `.iso8601` (not `.withFractionalSeconds`) encoding — reusing `audit_log`'s copy directly wasn't possible without making it `pub` across a crate boundary for one call site, so this duplicates the ~15-line algorithm instead.

All 8 `fixtures/world-import-reconciliation/` cases are driven by a real on-disk server directory per test (live folders, `world_slots/` entries, and real `world.zip` archives via the `zip` crate directly) — the same "genuinely disk-shaped" precedent P5.13/P5.14 already set, necessary here since `archive::extract_zip`/`create_zip_from_folders` require real files. A ninth test proves the idempotency requirement literally: a second call against an already-reconciled server returns `AlreadyReconciled`, creates no second slot, and leaves the active marker unchanged. `cargo fmt`/`cargo clippy --all-targets -- -D warnings` clean on `msc-application` and `msc-agent`; `cargo nextest run -p msc-application world_import_reconciliation`: 9 tests, 0 failures; the full pre-existing `msc-agent` suite (49 tests) still passes with the new startup hook wired in; full workspace build succeeds.

**Verify:** `cargo nextest run -p msc-application world_import_reconciliation`
**Commit:** `P6.11: reconcile imported worlds into slots`
**Batch:** stop-after

### P6.12 — Implement slot CRUD, copy, import, export, and thumbnails
**Status:** awaiting verification
**Files:** `crates/msc-application/src/worlds.rs`, `crates/msc-application/tests/world_slot_crud.rs`, `crates/msc-infrastructure/src/archive.rs`
**What:** Implement create fresh, save live into active, rename, delete-nonactive, duplicate, copy-into-existing, staged ZIP import, staged export, and deterministic thumbnail update over P6.10's repository. Every overwrite uses a temp artifact plus atomic replacement, every failure preserves the previous slot, and server/runtime state guards live in the application service rather than clients.

**Actual result:** `worlds.rs` gains a new `WorldError` enum (shared by this step and P6.13/14) and eight CRUD/copy/import/export functions ported from `WorldSlotManager`'s matching verbs, each folding in the orchestration-layer guard MSC 1 applies at its own call site (name trim/empty checks, `deleteWorldSlot`'s active-slot refusal) rather than leaving it to a caller, per this file's established P6.11 pattern: `create_slot_from_current_world` (`createSlot`), `update_active_slot_from_current_world` (`updateSlotFromCurrentWorld`, scratch-file-then-atomic-replace), `rename_slot` (`renameSlot`, metadata-only), `delete_slot` (`deleteSlot` + the active-slot guard), `duplicate_slot` (`duplicateSlot`, fresh UUID), `copy_slot_into_existing` (`copySlotIntoExisting`, scratch-copy-then-atomic-replace, metadata-save failure non-fatal per source's own comment), `export_slot_zip` (`exportSlotZIP`, overwrite-at-destination), and `import_zip_as_new_slot` (`createSlotFromZIP`, verbatim copy, no structural validation — the D-006 correction lives once, uniformly, in `archive::extract_zip` at activation time). `import_zip_as_new_slot`'s level-name/seed inference ports `inferJavaLevelName(fromSlotZIP:)` (a real-zip-listing heuristic P6.9 didn't port, since P6.9 only needed `first_level_dat_path`'s narrower selection) as a new private `worlds.rs` helper, and reuses P6.9's `nbt::first_level_dat_path`/`imported_world_metadata_from_level_dat`/`merge_sidecar_metadata` for the seed half — both needing a real zip listing/member read, which `archive.rs` gains as two new small primitives (`list_entry_names`, `read_entry_bytes`, native via the `zip` crate rather than shelling to `unzip -Z -1`/`unzip -p`, same D-006 precedent as `extract_zip`/`create_zip_from_folders`). `set_slot_thumbnail` is a thin pass-through to P6.10's `world_store::save_thumbnail` so every slot mutation is reachable through this one module.

Every zip-writing operation goes through `msc_infrastructure::archive`/real files exactly as P6.10/P6.11 already established (bypassing the injectable `FileSystem` trait for that half only); `copy_via_fs` is this step's own small addition — a copy expressed as `write(read(from))` through the trait, used for every zip-to-zip copy (duplicate/copy-into-existing/export/import) so at least that half stays behind the same abstraction as the metadata writes alongside it. Zip-write-failure fixtures (`create-slot-zip-failure-cleans-up-slot-directory`, `update-active-slot-zip-failure-preserves-previous-archive`, `copy-into-existing-mid-copy-failure-preserves-destination`) are exercised via Unix-only, `#[cfg(unix)]`-gated permission locks (`chmod`) rather than a directory-collision trick, since the destination path for each is only known after a random UUID is generated — flagged as a real, if narrow, Windows coverage gap: this native (non-shell) archive writer has no injectable failure point for a would-be Windows-equivalent test, unlike source's own shelled-out `zip`/`unzip` processes which P6.10/11 never needed to fail this way. `fixtures/world-mutations/`'s remaining 10 CRUD/copy/import/export cases are otherwise all covered by 13 tests in `tests/world_slot_crud.rs`, including a real, committed P6.7 NBT sample (`fixtures/world-nbt/samples/java-real-legacy-fields-level.dat.gz`, known seed `"0"`) for the import test rather than a synthetic one. `cargo fmt`/`cargo clippy --all-targets -- -D warnings` clean; `cargo nextest run -p msc-application world_slot_crud`: 13 tests, 0 failures (1 reports nextest's pre-existing, unrelated `LEAK` notice, same as already seen on this crate's other archive-touching tests before this step); full workspace build succeeds.

**Verify:** `cargo nextest run -p msc-application world_slot_crud`
**Commit:** `P6.12: implement world-slot CRUD`
**Batch:** safe

### P6.13 — Implement transactional world activation and restart recovery
**Status:** not started
**Files:** `crates/msc-application/src/worlds.rs`, `crates/msc-application/src/operations.rs`, `crates/msc-application/tests/world_activation.rs`, `crates/msc-infrastructure/src/operation_journal.rs`
**What:** Activate a slot as a journaled transaction: refuse a running server, require the safety-backup port, stage the replacement, move the prior live folder set aside, install/relocate the new world, update world identity, commit the active marker/last-played metadata, then remove rollback material. Inject failure after every boundary and reconcile an interrupted operation on restart to either the old complete world or the new complete world, never a mixture.
**Verify:** `cargo nextest run -p msc-application world_activation`
**Commit:** `P6.13: make world activation transactional`
**Batch:** safe

### P6.14 — Implement transactional world rename and replacement
**Status:** not started
**Files:** `crates/msc-application/src/worlds.rs`, `crates/msc-application/tests/world_mutations.rs`
**What:** Port MSC 1's Java/Bedrock direct rename and replacement workflows for the public compatibility routes. Preflight every destination and source, require the configured safety backup before destructive replacement, stage fresh/folder/backup input through the safe archive boundary, roll back partial multi-dimension renames in reverse order, and keep slot metadata/active identity consistent with the committed live folders.
**Verify:** `cargo nextest run -p msc-application world_mutations`
**Commit:** `P6.14: implement transactional world mutations`
**Batch:** stop-after

---

### Backups and recovery

### P6.15 — Port backup inventory, metadata, deletion, and configuration
**Status:** not started
**Files:** `crates/msc-domain/src/backup.rs`, `crates/msc-infrastructure/src/backup_store.rs`, `crates/msc-application/src/backups.rs`, `crates/msc-application/tests/backup_inventory.rs`, `crates/msc-domain/src/lib.rs`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-application/src/lib.rs`
**What:** Port `BackupMeta`, legacy/current filename display parsing, newest-first listing, sidecar fallback, verified-state representation, paired ZIP/sidecar deletion, active-slot association, interval-option fallback, and max-count clamping. Configuration persists through the existing durable `ConfigServer` record and re-reads after save.
**Verify:** `cargo nextest run -p msc-application backup_inventory`
**Commit:** `P6.15: port backup inventory and configuration`
**Batch:** safe

### P6.16 — Create and verify offline and running-server backups
**Status:** not started
**Files:** `crates/msc-application/src/backups.rs`, `crates/msc-application/tests/backup_creation.rs`, `crates/msc-application/tests/backup_online_consistency.rs`
**What:** Implement one authoritative backup path for manual, automatic, safety, and pre-replace triggers. Capture every Java dimension folder; when the target is actively running, send `save-all flush`, await the characterized line or timeout, send `save-off`, archive, and unconditionally send `save-on` even after cancellation/error. Verify the completed archive and required members before publishing final metadata or a success result. Keep the Bedrock hold/query/resume protocol behind the same fakeable runtime port but unavailable in production until Phase 10.
**Verify:** `cargo nextest run -p msc-application -E 'test(/backup_(creation|online_consistency)/)'`
**Commit:** `P6.16: create verified consistent backups`
**Batch:** safe

### P6.17 — Implement scheduled backups and known-good retention
**Status:** not started
**Files:** `crates/msc-application/src/backups.rs`, `crates/msc-agent/src/main.rs`, `crates/msc-application/tests/backup_retention.rs`, `crates/msc-agent/tests/backup_scheduler.rs`
**What:** Add a bounded scheduler driven by each persisted server's enabled/interval/max-count settings. Preserve MSC 1's no-online-players skip and live reconfiguration, prune only MSC-managed backups and paired orphan sidecars, and never delete the final verified recovery point. Scheduler ticks enter through operation exclusivity, so they conflict cleanly with activation/restore rather than racing filesystem mutation.
**Verify:** `cargo nextest run -p msc-application backup_retention && cargo nextest run -p msc-agent backup_scheduler`
**Commit:** `P6.17: schedule backups with known-good retention`
**Batch:** safe

### P6.18 — Implement transactional backup restore and restart recovery
**Status:** not started
**Files:** `crates/msc-application/src/backups.rs`, `crates/msc-application/src/operations.rs`, `crates/msc-application/tests/backup_restore.rs`
**What:** Preserve the safety-critical gate order: resolve server/slot, refuse unsupported/running/cross-slot requests, verify source, create and verify a mandatory safety backup, then stage restoration. Swap the current live folders through rollback names, install the verified archive, and journal each boundary. Cancellation or restart must reconcile to a complete old or restored world, retain the safety backup, and explain the outcome through the operation record.
**Verify:** `cargo nextest run -p msc-application backup_restore`
**Commit:** `P6.18: make backup restore transactional`
**Batch:** stop-after

---

### Conversion

### P6.19 — Port world conversion behind a fakeable Chunker boundary
**Status:** not started
**Files:** `crates/msc-application/src/world_conversion.rs`, `crates/msc-application/src/lib.rs`, `crates/msc-application/tests/world_conversion.rs`
**What:** Define a `WorldConverter` process port and port the agent-owned conversion workflow: validate stopped source/target, unzip into unique staging, locate the actual nested world, invoke an already-installed Chunker, package output as a slot archive, create or atomically replace the destination slot, require a verified target safety backup before activating, and clean every temp directory on success/failure/cancel/restart. Missing Chunker reports capability unavailable and performs no mutation.
**Verify:** `cargo nextest run -p msc-application world_conversion`
**Commit:** `P6.19: port the world conversion workflow`
**Batch:** stop-after

---

### Public clients

### P6.20 — Add Phase 6 DTOs and keep OpenAPI conformance executable
**Status:** not started
**Files:** `crates/msc-api/src/dto/worlds.rs`, `crates/msc-api/src/dto/backups.rs`, `crates/msc-api/src/dto/mod.rs`, `crates/msc-api/tests/world_backup_conformance.rs`, `docs/msc2/api-contract/openapi.json`
**What:** Implement every P6.8 request/response type, preserving the copied iOS client's existing field names/defaults and making all additions optional where skew requires it. Include operation IDs, verification state, staged transfer descriptors, and structured errors/capability-unavailable responses. Round-trip representative legacy and new payloads against the contract.
**Verify:** `cargo nextest run -p msc-api world_backup_conformance && python3 tools/api-contract-check.py --v1-summary`
**Commit:** `P6.20: add world and backup API types`
**Batch:** safe

### P6.21 — Back world and backup routes with the real services
**Status:** not started
**Files:** `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/backups.rs`, `crates/msc-agent/src/routes/mod.rs`, `crates/msc-agent/src/main.rs`, `crates/msc-agent/tests/world_backup_routes.rs`
**What:** Replace absent/stub behavior with P6.11–P6.19 services through the one durable Phase 5 state. Enforce `worlds`/`settings` permissions, approved-root staging, request limits, audit attribution, operation journaling/progress/cancellation, and per-server exclusivity. Every GET reflects re-read disk/config state; mutation responses do not claim success before commit and verification.
**Verify:** `cargo nextest run -p msc-agent world_backup_routes && python3 tools/contract-conformance-check.py --phase6`
**Commit:** `P6.21: wire real world and backup routes`
**Batch:** stop-after

### P6.22 — Add complete world and backup CLI commands
**Status:** not started
**Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/tests/cli_worlds_backups.rs`
**What:** Add list/create/rename/activate/delete/duplicate/copy/import/export/convert commands under `msc world`, and list/now/delete/restore/config commands under `msc backup`. Long operations print the operation ID, wait with progress by default, support cancellation, emit stable JSON under `--json`, and preserve meaningful nonzero exit codes.
**Verify:** `cargo nextest run -p msc-agent cli_worlds_backups`
**Commit:** `P6.22: add world and backup CLI commands`
**Batch:** safe

### P6.23 — Repoint iOS world/backup models and networking
**Status:** not started
**Files:** `clients/ios/MSCRemoteiOS_Swift/RemoteAPIModels.swift`, `clients/ios/MSCRemoteiOS_Swift/RemoteAPIClient.swift`, `clients/ios/MSCRemoteiOS_Swift/DashboardViewModel.swift`, `clients/ios/MSCRemoteiOSTests/Phase6WorldBackupAPITests.swift`, `clients/ios/MSCRemoteiOS.xcodeproj/project.pbxproj`
**What:** Replace the copied MSC 1 world/backup calls with `/v1` DTOs and operation polling/streaming, keep credentials host-keyed, and add multipart/staged upload/download support for slot import/export without exposing arbitrary server paths. Tests decode both preserved baseline payloads and additive Phase 6 payloads and prove auth/version headers remain attached.
**Verify:** `xcodebuild test -project clients/ios/MSCRemoteiOS.xcodeproj -scheme MSCRemoteiOS -destination 'platform=iOS Simulator,name=iPhone 17 Pro' -only-testing:MSCRemoteiOSTests/Phase6WorldBackupAPITests`
**Commit:** `P6.23: repoint iOS world and backup networking`
**Batch:** safe

### P6.24 — Complete the iOS world and backup workflows
**Status:** not started
**Files:** `clients/ios/MSCRemoteiOS_Swift/WorldsView.swift`, `clients/ios/MSCRemoteiOS_Swift/ServerView.swift`, `clients/ios/MSCRemoteiOS_Swift/` (Phase 6 backup views), `clients/ios/MSCRemoteiOSTests/Phase6WorldBackupViewModelTests.swift`, `docs/msc2/client-capability-matrix.csv`
**What:** Make the phone a real Phase 6 client: show active slot and verified backups; create/rename/activate/duplicate/delete/import/export/convert slots; create/delete/restore backups; edit schedule/retention; show progress/cancel/failure/recovery states; and require the existing device-auth protection for destructive restore/delete actions. Update every Phase 6 iOS matrix cell to `Implemented`; Desktop/Web remains `Planned`, never silently excepted.
**Verify:** `xcodebuild test -project clients/ios/MSCRemoteiOS.xcodeproj -scheme MSCRemoteiOS -destination 'platform=iOS Simulator,name=iPhone 17 Pro' -only-testing:MSCRemoteiOSTests/Phase6WorldBackupViewModelTests && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P6.24: complete iOS world and backup workflows`
**Batch:** stop-after

---

### Public-path and real-corpus proof

### P6.25 — Build a restart-sensitive Phase 6 public-path smoke
**Status:** not started
**Files:** `tools/phase6/phase6-gate-smoke.sh`, `tools/phase6/fixtures/`
**What:** Start a real foreground agent with isolated durable roots and use only the CLI/API to import a Java multi-folder world, reconcile it into slots, run slot CRUD, activate with a safety backup, take and verify manual/scheduled backups, inject failures into save coordination and archive creation, restore, restart the process mid-activation and mid-restore, and prove recovery leaves one complete world plus a known-good backup. The committed synthetic path runs everywhere; private real evidence is supplied only to P6.26.
**Verify:** `tools/phase6/phase6-gate-smoke.sh --synthetic`
**Commit:** `P6.25: add the restart-sensitive Phase 6 smoke`
**Batch:** solo

### P6.26 — Exercise the real MSC 1 world and backup corpus
**Status:** not started
**Files:** `tools/phase6/corpus-check.py`, `crates/msc-application/tests/real_world_backup_corpus.rs`, `corpus/worlds/README.md`, `corpus/backups/README.md`
**What:** Add exercise mode and run the real material collected in P6.3 through repository load, import reconciliation, safe archive validation, metadata/NBT parsing, a non-destructive restore into a temporary root, and save/reload. Hash every source before/after and report each independently. Run the real package/world/backup through the public Phase 6 smoke where size permits; a direct library-only pass is insufficient for the gate.
**Verify:** `python3 tools/phase6/corpus-check.py --exercise --worlds corpus/worlds --backups corpus/backups --private-root "$MSC2_PHASE6_PRIVATE_CORPUS"`
**Commit:** `P6.26: validate the real MSC 1 world and backup corpus`
**Batch:** stop-after

### P6.27 — Run Phase 6 fixtures and public smoke on all three platforms
**Status:** not started
**Files:** `.github/workflows/ci.yml`, `tools/phase6/phase6-gate-smoke.sh`, `docs/msc2/rolling-plan.md`
**What:** Add the committed synthetic Phase 6 fixture, application, route, CLI, and restart-smoke path to macOS, Linux, and Windows CI. Exercise Windows case-insensitive/path-separator/locked-file rollback cases and require all three jobs for the exact candidate commit. Do not put private corpus data or local absolute paths in CI.
**Verify:** `gh workflow run ci.yml --ref "$(git branch --show-current)" && sleep 5 && run_id=$(gh run list --workflow ci.yml --branch "$(git branch --show-current)" --limit 1 --json databaseId --jq '.[0].databaseId') && test -n "$run_id" && gh run watch "$run_id" --exit-status`
**Commit:** `P6.27: run Phase 6 safety checks on every platform`
**Batch:** stop-after

---

### Phase exit

### P6.28 — Phase 6 exit gate check
**Status:** not started
**Files:** `docs/msc2/rolling-plan.md` (this entry only unless the gate finds a defect)
**What:** Run the working gate from this phase's header, not the checklist: formatting and native/cross-target clippy; every workspace test; static API/capability checks; the restart-sensitive synthetic public-path smoke; the real MSC 1 world/backup corpus through readers and public operations; and the exact-commit GitHub Actions macOS/Linux/Windows jobs. Inspect the recovered live folders, slots, markers, backup archives, metadata, and operation records after the injected interruption cases. If any leg fails, stop and plan only the failing correction. Cameron alone marks this step `DONE` and advances to Phase 7.
**Verify:** `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings && cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- -D warnings && cargo nextest run --workspace && python3 tools/api-contract-check.py --v1-summary && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && tools/phase6/phase6-gate-smoke.sh --synthetic && python3 tools/phase6/corpus-check.py --exercise --worlds corpus/worlds --backups corpus/backups --private-root "$MSC2_PHASE6_PRIVATE_CORPUS" && run_id=$(gh run list --commit "$(git rev-parse HEAD)" --limit 1 --json databaseId --jq '.[0].databaseId') && test -n "$run_id" && gh run watch "$run_id" --exit-status`
**Commit:** `P6.28: run the Phase 6 exit gate`
**Batch:** stop-after

---
