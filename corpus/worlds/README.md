See `../README.md`. **Empty — needs real evidence from Cameron, collected by
P6.3.**

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

## P6.3 investigation results (2026-08-13) — blocked, nothing collected

Searched for real evidence across Cameron's machine: both Java servers MSC 1
currently manages (`~/MinecraftServers/java/campack`,
`~/MinecraftServers/java/paper`, per `server_config_swift.json`), an older
unmanaged copy of the same modpack (`~/Downloads/Minecraft/camcraft_modpack`),
the Desktop/Downloads folders, and local Time Machine snapshots. This
directory is still empty on purpose — every gap below is real, and this
step's own instruction is to stop rather than invent, so nothing was written
here yet.

**What's real and present:** three real `world_slots/` trees (`campack`,
`paper`, and the unmanaged `camcraft_modpack` copy), each with a non-empty
`active_slot_id.txt` marker naming a slot directory that exists and a real
`slot.json` (`id`/`name`/`world_level_name`/`created_at`; `campack`'s also
has `world_seed`); two real live Java worlds with an actual `level.dat`
outside `world_slots/`.

**Two gaps against this checker's own inventory requirements:**

1. **No slot has a `world.zip` archive.** All three real `world_slots/`
   entries are archive-less — the "fresh archive-less slot" / "initial-slot
   bootstrap" case P6.4's own characterization work already names, not yet
   the "copied slot archive" case this checker requires. MSC 1 only writes
   `world.zip` into a slot on an explicit save/duplicate-to-slot action;
   neither real server has ever had that triggered.
2. **Neither real world has a `<name>_nether` / `<name>_the_end` sibling
   directory.** `campack` is a Fabric server, so Minecraft's own vanilla
   world format nests dimensions inside the main world folder (`DIM-1`,
   `DIM1`) — Fabric/vanilla servers structurally never produce sibling
   dimension folders, now or later. `paper`'s dimensions (both generated —
   `the_nether` and `the_end` region data both exist) live under a nested
   `Paper/dimensions/minecraft/{overworld,the_nether,the_end}/` tree
   instead of the classic sibling-folder layout this checker expects — this
   looks like a real PaperMC storage-layout change in whatever version is
   now installed versus whatever version `WorldSlotManager.swift`'s
   multi-folder assumption was written against, not a configuration choice
   Cameron made.

No real backup evidence exists anywhere searched — see `../backups/README.md`.

This is exactly the "stop instead of inventing it" case this step's own
text names. Nothing here or in `../backups/` was fabricated to fill the
gap; `docs/msc2/rolling-plan.md`'s P6.3 entry records the question this
raises for Cameron.
