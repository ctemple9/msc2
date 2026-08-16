See `../README.md`. **Populated by P6.3** with real evidence from Cameron's
live MSC 1 install — the actual `level.dat`/`world.zip`/backup-zip bytes are
git-ignored (`.gitignore` in this directory), since they carry real
per-player NBT data; `manifest.json` (committed) records their source,
hashes, and why. See "P6.3 real evidence collected" below.

`tools/phase6/corpus-check.py` (P6.2) is the dependency-free gate that
checks this directory (together with `../backups/`) before P6.4 onward
characterizes against it. Run `python3 tools/phase6/corpus-check.py
--inventory` to check the default paths, or `--worlds DIR --backups DIR` to
point at others. In inventory mode it requires, directly in this directory:

- A real Java multi-folder world: a `level.dat` (outside `world_slots/`)
  with dimension evidence in any of three real layouts (see "P6.3 real
  evidence collected" below for why there are three, not one): classic
  sibling directories (`<name>_nether` and/or `<name>_the_end`),
  vanilla/Fabric nested folders (`DIM-1` and/or `DIM1` inside the world
  folder itself), or current PaperMC's nested folders
  (`dimensions/minecraft/the_nether` and/or `dimensions/minecraft/the_end`
  inside the world folder).
- A real MSC 1 `world_slots/` tree: a non-empty `active_slot_id.txt` marker
  naming a slot that actually exists, at least one slot's `slot.json`
  metadata, and at least one slot's `world.zip` archive.
- A `manifest.json` alongside them with a `files` entry per evidence file
  (`level.dat`, `active_slot_id.txt`, every `slot.json`, every `world.zip`)
  recording that file's `source`, `sanitized` description, and SHA-256.
- No two evidence files (here or in `../backups/`) sharing a SHA-256 -- a
  duplicate isn't a second sample.
- No archive (`world.zip`) containing an entry with an absolute path or a
  `..` component.

Real Bedrock evidence (a top-level directory named `bedrock*`, containing
its own `level.dat`) is optional and checked when present, per
`docs/msc2/worlds/phase6-scope.md`'s Bedrock deferral (repair stays Phase
10) -- never fabricated to fill this category.

The checker's own passing and deliberately-broken self-test cases live
under `tools/phase6/fixtures/` instead, precisely so nothing invented ends
up here standing in for the real thing.

## P6.3 real evidence collected (2026-08-13)

An initial search (both MSC 1-managed Java servers, an older unmanaged
modpack copy, Desktop/Downloads, local Time Machine snapshots) found real
`world_slots/` metadata but every real slot was archive-less and no real
backup existed anywhere — recorded below as "Original gap 1/3", both now
closed. Cameron then generated the missing evidence for real, in the real
app: MSC 1's **Back Up** (Backups tab) and **Save Current World** (Worlds
tab) actions, run against both `campack` and `paper`, 2026-08-13 22:29.

**What's here now**, all real, hashed and provenance-recorded in
`manifest.json`, actual bytes git-ignored:

- `Paper/` and `campack/` — two real live Java worlds (vanilla-Paper and
  Fabric-modded), each with a real `level.dat`.
- `world_slots/` — `paper`'s real slot, now **with** a real `world.zip`
  (565,734 bytes; see `manifest.json` for the exact hash), produced live by
  "Save Current World." `campack`'s equally real `world_slots/world.zip`
  (11,269,354 bytes) exists on Cameron's own disk at
  `~/MinecraftServers/java/campack/world_slots/` and can swap in if a
  modded-server slot example is ever needed instead — only one server's
  `world_slots/` tree fits this checker's single `corpus/worlds/world_slots/`
  path at a time.

**Original gap 1 (no slot had a `world.zip`) — closed.** Both real slots now
have one, generated live rather than invented.

**Original gap 3 (no real backup existed) — closed**, see `../backups/README.md`.

**Original gap 2 — resolved by relaxing the checker, not by more evidence.**
Generating fresh evidence didn't close this one: neither real world has a
`<name>_nether` / `<name>_the_end` sibling directory next to `level.dat`,
the only layout `check_worlds_structure` in `tools/phase6/corpus-check.py`
originally accepted. Confirmed by running `python3 tools/phase6/corpus-check.py
--inventory --worlds corpus/worlds --backups corpus/backups` against this
real, complete evidence before the checker was changed: it got past every
provenance/hash/manifest/safety check and failed on exactly one line —
`corpus/worlds/Paper: no dimension sibling directory (Paper_nether or
Paper_the_end) -- not a Java multi-folder world`. That wasn't a
missing-evidence problem, it was a real, structural mismatch between what
the checker expected (MSC 1's classic split-folder Java world layout) and
what both of Cameron's real servers actually produce: `campack` is Fabric,
whose vanilla world format nests dimensions inside the main world folder
(`DIM-1`/`DIM1`) and structurally can never produce sibling folders, and
`paper` uses a newer nested
`Paper/dimensions/minecraft/{overworld,the_nether,the_end}/` layout instead
of the classic sibling convention `WorldSlotManager.swift`'s multi-folder
assumption was written against.

Cameron chose to relax the checker (`docs/msc2/rolling-plan.md`'s P6.3
entry records the question and the choice) rather than chase evidence for a
layout neither real server produces. `check_worlds_structure` now accepts
all three real shapes — see this file's own requirements list above — and
`corpus/worlds/manifest.json`'s note records that `corpus/worlds/Paper` is
real evidence for the third (nested PaperMC) shape specifically, not the
classic one the checker was first built around. A new self-test fixture,
`tools/phase6/fixtures/no-dimension-evidence/`, pins the negative case the
relaxation could otherwise have quietly broken: a world with *none* of the
three shapes still fails inventory mode.

## P6.26 real evidence exercised (2026-08-16)

Inventory mode (above) only proves the real evidence is *present and
well-formed*. Exercise mode runs it through the real Rust code paths:

```
python3 tools/phase6/corpus-check.py --exercise --worlds corpus/worlds \
  --backups corpus/backups --private-root "$MSC2_PHASE6_PRIVATE_CORPUS"
```

This runs every inventory check first, then
`crates/msc-application/tests/real_world_backup_corpus.rs` against both
real worlds and both real backups: `world_store::load_slots` on the real
`world_slots/` tree; `archive::validate_archive_safety` on every real
`.zip` (`world_slots/.../world.zip` and both backup zips);
`imported_world_metadata_from_level_dat` on both real `level.dat` files
(`Paper` parses to `gamemode=survival`, `campack` to
`difficulty=normal, gamemode=survival` — real values, not fixture ones);
`worlds::reconcile_imported_worlds` against a temporary copy of `Paper/` +
`world_slots/` (resolves to `RecoverySnapshotCreated`, not
`LiveFoldersProvenIdenticalToRecordedSlot` — the live folder and the
recorded slot's archive aren't byte-identical, a real result neither
forced nor assumed); and `backups::restore_backup` restoring the real
`Paper_manual_...zip` backup into a temporary root, followed by
`create_slot_from_current_world` + repository reload + re-extraction to
prove a full save/reload round trip is byte-identical. Every real source
file touched is hashed before and after, both inside the Rust test and
again by the Python wrapper, independently.

`Paper` (the smaller, ~600KB real world) carries the write-path exercises
(reconciliation, restore, save/reload); `campack` (~11MB, Fabric-modded)
is exercised by every read-only check (repository load, archive safety,
NBT parsing) but not doubled through the write-path ones — this phase's
own plan text ("where size permits") for that split.

`--private-root` is the plan's other requirement, "run the real package/
world/backup through the public Phase 6 smoke where size permits" —
`tools/phase6/phase6-gate-smoke.sh` (P6.25) itself only has a
`--synthetic` mode; giving it a real-corpus mode is scoped work outside
this step's own `Files:` list (which doesn't include the smoke script), so
today `--private-root` only detects whether a private corpus root was
supplied and reports the public-smoke leg as not yet wired, rather than
silently declaring it done. Flagged in `rolling-plan.md`'s P6.26 entry.
