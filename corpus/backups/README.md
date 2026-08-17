See `../README.md`. **Populated by P6.3** with two real backup zips (one per
MSC 1-managed Java server) from Cameron's live MSC 1 install — the actual
`.zip`/`.meta.json` bytes are git-ignored (`.gitignore` in this directory),
since they carry real per-player NBT data; `manifest.json` (committed)
records their source, hashes, and why. See "P6.3 real evidence collected"
below.

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

## P6.3 real evidence collected (2026-08-13)

An initial search found no real backup `.zip` anywhere on this machine (see
`../worlds/README.md`'s matching section for the full search). Cameron then
generated two, for real, in the real app: MSC 1's **Back Up** action
(server editor → Backups tab → "Back Up" under Manual Actions), run against
both `campack` and `paper`, 2026-08-13 22:29.

- `Paper_manual_20260813-222932.zip` + `.meta.json` (565,734 bytes)
- `campack_manual_20260813-222917.zip` + `.meta.json` (11,269,354 bytes)

Both are real `AppViewModel.createBackupForSelectedServer(isAutomatic: false)`
output — same production code path a user's own manual backup takes, not a
synthetic fixture. The `.zip` bytes are git-ignored (real per-player NBT
data); the `.meta.json` sidecars are small, contain only server/slot ids and
names, and are committed as-is. `manifest.json` records source and SHA-256
for all four files.

## P6.26 real evidence exercised (2026-08-16)

`python3 tools/phase6/corpus-check.py --exercise` (see `../worlds/README.md`'s
matching section for the full command and what it runs) restores the real
`Paper_manual_20260813-222932.zip` backup here through the real
`backups::restore_backup` into a temporary root — never touching this
directory itself — and validates both real backup `.zip`s' archive safety.
`campack_manual_20260813-222917.zip` (~11MB) is exercised by the
archive-safety check but not restored, matching the write-path-stays-small
split `../worlds/README.md` records.

## P6.35 real evidence driven through the public path (2026-08-16)

The evidence in this directory stays exercised only at the application-
library level (above). P6.35's own public-path leg — proving backup/
restore work over the real agent's CLI/HTTP surface, not just direct
library calls — runs against a *different*, larger private corpus
(`$MSC2_PHASE6_PRIVATE_CORPUS`, a real MSC-1-managed servers root, not
this directory) via `tools/phase6/phase6-gate-smoke.sh --private-corpus
<root>`. See `../worlds/README.md`'s matching P6.35 section for what that
mode does and the one real, pre-existing limitation it surfaced.
