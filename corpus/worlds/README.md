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
