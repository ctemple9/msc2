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
  with at least one dimension sibling directory next to it
  (`<name>_nether` and/or `<name>_the_end`).
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

**Original gap 2 — still open, and generating fresh evidence didn't close
it:** neither real world has a `<name>_nether` / `<name>_the_end` sibling
directory next to `level.dat`, which `check_worlds_structure` in
`tools/phase6/corpus-check.py` requires. Confirmed by running
`python3 tools/phase6/corpus-check.py --inventory --worlds corpus/worlds
--backups corpus/backups` against this now-real, now-complete evidence: it
gets past every provenance/hash/manifest/safety check and fails on exactly
one line —

```
corpus/worlds/Paper: no dimension sibling directory (Paper_nether or Paper_the_end) -- not a Java multi-folder world
```

This isn't a missing-evidence problem any more — it's a real, structural
mismatch between what this checker expects (MSC 1's classic split-folder
Java world layout) and what both of Cameron's real servers actually produce:
`campack` is Fabric, whose vanilla world format nests dimensions inside the
main world folder (`DIM-1`/`DIM1`) and structurally can never produce
sibling folders, and `paper` uses a newer nested
`Paper/dimensions/minecraft/{overworld,the_nether,the_end}/` layout instead
of the classic sibling convention `WorldSlotManager.swift`'s multi-folder
assumption was written against. `docs/msc2/rolling-plan.md`'s P6.3 entry
records the question this raises for Cameron.
