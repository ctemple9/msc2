# Phase 6 scope: Worlds and backups

**Status:** P6.1 scoping note, per the P5.33 gate-review amendment carried into `msc2-port-plan.md` §3.
**Source of truth:** `msc2-port-plan.md` §3 (Phase 6 gate), `docs/msc2/rolling-plan.md` (Phase 6 section), `msc2-decisions.md`, the Phase 0 symbol ledger (`docs/msc2/audit/msc2-symbol-ledger.csv`), and `docs/msc2/config-migration/phase5-scope.md` (the boundary this note picks up).
**MSC 1 oracle:** `~/Documents/Swift Projects/minecraft-server-controller`, read-only. Primary files for this note: `WorldSlotManager.swift` (slot resolution/CRUD/NBT), `AppViewModel+WorldSlots.swift` (`createInitialWorldSlotIfNeeded`, orchestration).

This note fixes the boundary for Phase 6 before code starts, in the same role `phase3-scope.md`/`phase4-scope.md`/`phase5-scope.md` played for their phases, and it discharges the specific debt `phase5-scope.md` recorded and left for this phase (see that file's "Deferred and homeless" section, "The formal world-slot model" bullet). It does not approve new product behavior on its own; where a choice is genuinely open, it records the recommended working answer.

## Working exit gate

Quoted from `rolling-plan.md`'s Phase 6 header, which is itself pinned to `msc2-port-plan.md` §3:

> World discovery, slots, transactional mutations, backups, retention, verification, restore. Phase 6 must also satisfy the P5.33 amendment: audit and reconcile Phase 5's imported live-world and `world_slots` data before world mutations become authoritative.

And the P5.33 amendment itself, from `msc2-port-plan.md` line 116:

> Phase 5 imports and labels world data but does not reconcile it into the formal world-slot model. Phase 6 must audit imported live-world and `world_slots` data before world mutations become authoritative.

This note's job is the "audit and reconcile" half specifically: what state can Phase 5 actually leave on disk, and what is the one rule Phase 6 applies to every one of those states before any mutation route (activate, restore, delete, replace) is allowed to run against it.

## Why this is a real problem, not a hypothetical one

I read both Phase 5 import paths and MSC 1's own slot-bootstrap logic directly, rather than guessing at what "imported world data" could look like:

- **Raw import** (`crates/msc-application/src/import.rs::import_raw_server`) copies the **entire** source directory tree wholesale via `copy_dir_recursive` — it does not filter to just the worlds `discover_worlds` detects for labelling. If the folder a user points raw import at is itself a real MSC-1-managed server directory (its own `world_slots/` alongside live `world`/`world_nether`/`world_the_end` folders), Phase 5 copies **both** in, verbatim, with no reconciliation. Raw import never touches `world_slots/` itself otherwise, so an ordinary external Java/Bedrock install (never touched by MSC 1) lands as **live folders only**.
- **Transfer import** (`crates/msc-application/src/transfer.rs::apply_one_server`) copies `world_slots/` unconditionally whenever the package contains it (`WHOLESALE_SUBDIRS`), *and separately* restores live world folders when the package bundles them (preferred), falling back to `restore_active_slot_world` — which extracts the resolved active slot's `world.zip` into the live-folder location — only when no live folders were bundled. Both mechanisms can fire on the same entry: a package built from a real, previously-stopped MSC 1 server (which auto-gets a slot on first stop, see below) commonly bundles **both** live folders and `world_slots/`, landing exactly in the **both together** state. A package whose most recent slot is fresh/archive-less (no `world.zip` yet) and bundles no live folders leaves `world_slots/` present with **no** live folder materialized — **world_slots only**, with no live data to run at all until a slot is activated.
- MSC 1 itself only builds `world_slots/` lazily, via `AppViewModel+WorldSlots.swift::createInitialWorldSlotIfNeeded`, called **after a server's first stop** — not at creation or import time. That is why a live-folders-only state is completely normal for anything that hasn't been stopped once inside MSC 1 yet, and why Phase 6 cannot assume `world_slots/` is either always present or always absent.

So all three states named in the gate are real, reachable outcomes of the two Phase 5 import paths as they exist today, not edge cases invented for completeness:

1. **live folders only** — plain external installs via raw import; transfer packages from a never-stopped MSC 1 server.
2. **world_slots only** — a raw-imported folder whose live world was removed but whose `world_slots/` survived; a transfer package whose only slot is fresh/archive-less and bundled no live folders.
3. **both together** — a raw-imported MSC-1-managed folder; a transfer package from a server that has been stopped (and thus slot-bootstrapped) at least once, which is the common case for any real, used MSC 1 install.

## MSC 1's own active-slot resolution (the chain Phase 6 must reuse)

`WorldSlotManager.resolvedActiveSlotID` (source line 128-144) is the existing, already-characterized (P6.4) fallback chain and Phase 6 reuses it unchanged for reading `world_slots/`, whether that directory came from MSC 1 natively or was copied in by Phase 5:

1. The explicit marker (`world_slots/active_slot_id.txt`), if it names a slot that still exists after tolerant per-slot loading.
2. Otherwise, the most-recently-played slot (`lastPlayedAt`), if any slot has that field set.
3. Otherwise, the newest-created slot (`createdAt`).
4. If `world_slots/` is missing, empty, or every `slot.json` fails to parse, there is no recorded active slot at all.

`loadSlots` (source line 274-...) is tolerant of a corrupt individual `slot.json` — it skips that one entry rather than failing the whole directory — and Phase 6's reader must preserve that, since a partially-corrupt `world_slots/` copied in by Phase 5 is exactly the kind of input this reconciliation exists to handle safely.

## The reconciliation rule

Applies once, at Phase 6 startup, per server, before any world-mutation route (activate, restore, delete, replace, backup-restore) is reachable for that server. Never mutates the Phase 5 corpus/import inputs themselves — it only acts on the copied, owned directory tree already inside MSC 2's server root.

### State 1 — live folders only

No `world_slots/` directory, or one present but resolving to no active slot at all (see resolution chain step 4). There is nothing to reconcile *against* — archive the current live folders into a brand-new slot (mirroring `createInitialWorldSlotIfNeeded`'s own archive step, `WorldSlotManager.createSlot`) and persist that slot as the active marker. The live folders are left untouched on disk; the new slot is a parallel archived copy, exactly as MSC 1's own post-first-stop bootstrap behaves. No recovery snapshot is needed here — there is only one source of truth and it is not at risk.

### State 2 — world_slots only

No live world folders exist yet (Java: none of `<level>`/`<level>_nether`/`<level>_the_end` exist at the server root; Bedrock: no `worlds/` directory). Resolve the active slot via the chain above.

- If that slot has a real archive (`world.zip`), extract it into the live-folder location — the same operation Phase 5's `restore_active_slot_world` already performs for transfer import, except Phase 6's version **does** persist the active marker afterward (Phase 5's version deliberately does not, since the formal slot model didn't exist yet when that code was written).
- If the resolved slot is fresh/archive-less (`createFreshWorldSlot`'s no-backing-archive case), there is no live data to materialize. Leave no live folders; the server gets a newly-generated world on first start, identical to MSC 1's own behavior for a not-yet-activated fresh slot. Persist the active marker regardless, so activation state is well-defined before any route is reachable.

### State 3 — both together

Live folders exist **and** `world_slots/` resolves to a recorded active slot. This is the safety-critical case the gate names explicitly, and the one this note is most responsible for getting right.

1. Inventory both sources: the live folders on disk, and the recorded active slot's archived content (if any — the recorded active slot could itself be fresh/archive-less).
2. If the recorded active slot has no archive, or resolution finds no active slot at all despite `world_slots/` existing (every entry corrupt, or none marked/inferable), treat this exactly like **State 1**: archive the live folders as a new slot and make it active, without touching whatever unresolvable/archive-less slot data is already there — it stays on disk as an inactive, still-browsable slot.
3. Otherwise, compare the live folders against the recorded active slot's archived content. Equality must be **proven**, not assumed: extract the archive to a scratch location and compare file-by-file (presence, size, and content hash) against the live folders. A cheap check that could produce a false "identical" (e.g., matching only file names or sizes) is not acceptable here — a wrong "equal" verdict is a silent data-loss bug, not a performance shortcut.
   - **Proven identical:** persist the active marker pointing at the existing recorded slot. No new slot is created — the common, no-op case for a server that was cleanly stopped and never touched again.
   - **Different, or equality cannot be established** (extraction fails, comparison is inconclusive, etc.): create a **recovery snapshot** — archive the live folders into a **new** slot, distinct from the existing recorded slot. Never overwrite the existing slot's archive or metadata to do this. The new snapshot slot becomes active (this is what "preserve Phase 5's established live-world precedence" means in practice: the live folders are the freshest, most-recently-touched data Phase 5 actually imported, so they win the active slot going forward), while the previously-recorded slot survives untouched as an ordinary, selectable, non-active slot. Nothing the user could reach through MSC 1 before Phase 5 ran is deleted or hidden by this step — it just stops being the default.

### Ordering and crash safety

Across all three states, the active marker is the **last** write of the reconciliation, after every required archive and metadata write for that server has succeeded — matching P6.11's own description of this step. This note fixes the *rule*; the transactional mechanics that make it crash-safe (staging a new slot's contents before it is atomically visible under `world_slots/`, and using a dedicated marker to distinguish "reconciliation already ran" from "a marker happens to already resolve," so a copied-in MSC-1-native `active_slot_id.txt` is never mistaken for proof that Phase 6's own live-vs-archive comparison already happened) are P6.11's job, not this note's — P6.11 already has an operation-journal pattern available from the rest of this phase (see P6.13) to reuse rather than invent a second mechanism. The requirement this note fixes for P6.11 to satisfy: **a second startup against an already-reconciled server must make no additional changes**, and a crash between any two of the writes above must leave the server in a state where either the old recorded slot or the new snapshot slot is completely intact and resolvable — never a partial mixture.

## Symbol-ledger rows owned by this phase

Every `docs/msc2/audit/msc2-symbol-ledger.csv` row whose `target_domain` is `worlds`, `backups`, `world-conversion`, or `worlds/players` — 38 rows total, every one `disposition=agent` (none are client-only, so nothing here needs re-bucketing to a client). Grouped by MSC 1 source file:

| MSC 1 file | Symbols (rows) | Rust destination in this phase |
|---|---|---|
| `WorldSlotManager.swift` | active-slot resolution chain, level-name/NBT parsing, `loadSlots`/`saveMetadata`, `saveThumbnail`, `createSlot`/`updateSlotFromCurrentWorld`, `createFreshWorldSlot`/`applyWorldIdentity`, `activateSlot`, rename/delete/duplicate/copy/export/import-from-ZIP, NBT parsing engine | P6.9 (`msc-domain`), P6.10 (`msc-infrastructure`), P6.12–P6.14 (`msc-application`) |
| `AppViewModel+WorldSlots.swift` | `defaultPersistentSlotName`/`ensureActiveWorldSlotExists`, `saveCurrentWorldToActiveSlot`/`saveCurrentWorldAsSlot`, `createNewWorldSlot`, `activateWorldSlot`, slot CRUD orchestration, `restoreSlotBackup`, `importLegacyBackupAsNewSlot`/`importZIPAsSlot`/`exportWorldSlot`, `createInitialWorldSlotIfNeeded` | P6.11 (reconciliation reuses this note's rule directly), P6.12–P6.14 |
| `AppViewModel+WorldManagement.swift` | `refreshWorldSize`, `replaceWorld`, `renameWorld`, `validateZipArchive`/`unzipWorldBackup`/`copyExistingWorldFolder` | P6.14 |
| `AppViewModel+WorldRepair.swift` | `repairWorldLevelDat` | **Bedrock runtime-dependent — stays Phase 10**, per the deferral below. |
| `AppViewModel+HealthCards.swift` | `checkBedrockWorldData` | Phase 6 for the file-layout check; the health-card surface itself is a later client concern. |
| `AppViewModel+WorldConversion.swift` | `isRunning`, `performWorldConversion`/`unzipSlot`/`createConvertedSlot`/`replaceSlot` | P6.19 |
| `AppViewModel+Backups.swift` | `loadBackupsForSelectedServer`/`makeDisplayName`, `createBackup` family, `pauseSavesForBackup`/`resumeSavesAfterBackup`/`waitForBedrockSaveReady`, `readBackupMeta`/`writeBackupMeta`/`deleteBackup`, `pruneAutoBackupsForSelectedServer`/`pruneAutoBackupsIfNeeded`, `restoreBackup`, `backupWorld(for:)`, `removeWorldFolders` | P6.15–P6.18 |
| `AppViewModel+ServerControls.swift` | `startAutoBackupTimer`/`stopAutoBackupTimer`/`setAutoBackupEnabled`/schedule state | P6.17 |
| `AppViewModel+APIWiringWorlds.swift` | `wirePlayerAndWorldProviders` (world-slot half) | P6.21 |
| `AppViewModel+APIWiringBackupsHealth.swift` | `wireBackupProviders` | P6.21 |
| `AppViewModel+APIWiringSettings.swift` | `backupConfigProvider`/`updateBackupConfigProvider` | P6.15/P6.21 |

One row in this set, `AppViewModel+Backups.swift::duplicateBackupToNewServer`, is explicitly **not** built in this phase — see "Deferred on purpose" below. It stays on the ledger as `disposition=agent` because it is genuinely agent-owned work, just scheduled for Phase 7.

## Deferred on purpose

Restated from `rolling-plan.md`'s Phase 6 "Not in this phase" list so this note is self-contained:

- **Bedrock `level.dat` repair and production online-backup command delivery** stay **Phase 10** — both require a real Bedrock runtime, which doesn't exist until then. `repairWorldLevelDat` and the Bedrock half of the online-consistency backup protocol port their file-layout/NBT rules and fake-runtime protocol tests now, but return an explicit capability-unavailable error for imported Bedrock records in production rather than pretending the operation ran.
- **Provisioning a new server from a backup** (`duplicateBackupToNewServer`) stays **Phase 7** with server-family provisioning. Phase 6 can restore a backup into the current server or import it as a world slot; it does not construct a new runtime.
- **Installing or updating Chunker** is not folded into world mutation — an absent converter is an advertised unavailable capability, not an implicit download. Helper acquisition belongs with later helper/provisioning work.
- **Desktop/web screens** stay **Phase 11**. The copied iOS client and CLI are this phase's client surfaces.
- **Arbitrary host filesystem browsing** stays outside the world API; import/export use bounded, operation-scoped staging under approved roots.

## Not resolved by this note

This note pins the reconciliation rule and the evidence behind it; it does not decide P6.11's transactional implementation details (staging layout, the dedicated reconciliation-complete marker's exact name/location, or how the operation journal integrates), the exact content-comparison algorithm's performance characteristics under real-world-sized data, or any other later step's design. Where a later step hits a genuine judgment call, it raises it as a question in the format `CLAUDE.md` requires rather than deciding it here.
