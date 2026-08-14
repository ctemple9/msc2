See `../README.md`. **Empty — needs real evidence from Cameron, collected by
P6.3.**

`tools/phase6/corpus-check.py` (P6.2) is the dependency-free gate that
checks this directory (together with `../worlds/`) before P6.4 onward
characterizes against it. Run `python3 tools/phase6/corpus-check.py
--inventory` to check the default paths, or `--worlds DIR --backups DIR` to
point at others. In inventory mode it requires, directly in this directory:

- At least one real MSC 1 backup `.zip`. An adjacent `<name>.meta.json` is
  checked for valid JSON when present, but is never required on its own.
- A `manifest.json` alongside them with a `files` entry per `.zip` and per
  `.meta.json` recording that file's `source`, `sanitized` description, and
  SHA-256.
- No two evidence files (here or in `../worlds/`) sharing a SHA-256 -- a
  duplicate isn't a second sample.
- No backup `.zip` containing an entry with an absolute path or a `..`
  component.

The checker's own passing and deliberately-broken self-test cases live
under `tools/phase6/fixtures/` instead, precisely so nothing invented ends
up here standing in for the real thing.

## P6.3 investigation results (2026-08-13) — blocked, nothing collected

No real backup `.zip` exists anywhere searched on this machine: MSC 1 only
creates `<serverDir>/backups/` on the first backup it takes
(`ConfigManager.backupsDirectoryURL`), and neither Java server MSC 1
currently manages has that folder at all; an older, unmanaged copy of the
same modpack under `~/Downloads/Minecraft/camcraft_modpack/backups/` has the
folder but it's empty. No manual or scheduled backup has ever run on this
machine. See `../worlds/README.md`'s matching section for the full search
and the two other real-evidence gaps found alongside this one.

This directory stays empty on purpose, per this step's own "stop instead of
inventing it" instruction — Cameron would need to trigger at least one real
backup through MSC 1 (or supply one from elsewhere) before this category has
anything real to inventory.
