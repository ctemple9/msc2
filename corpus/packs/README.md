See `../README.md` (§ `packs/`). **Not yet populated** — P8.3 records a
real `.mrpack` and a real CurseForge-format modpack archive here (at
minimum something in BMC4's shape, the pack referenced throughout the
P0.16/P0.18 fixtures, and a real Modrinth pack), per
`docs/msc2/addons/phase8-scope.md`. This note (P8.2) fixed the shape that
evidence must arrive in, and the checker that gates it, before any of it
was collected — the same ordering `tools/phase7/provider-corpus-check.py`
and `tools/phase6/corpus-check.py` used for their own corpora.

`tools/phase8/phase8-check.py` (P8.2) is the dependency-free gate. Pack
mode checks this directory against its own `manifest.json`:

`python3 tools/phase8/phase8-check.py --packs [DIR]` (default `DIR`:
`corpus/packs`). Requires, for every recorded archive:

- A `manifest.json` entry keyed by the file's path relative to this
  directory, recording `source_url`, `captured`, `sha256`, `byte_size`,
  and `pack_format` (`mrpack` or `curseforge`).
- The recomputed SHA-256 matching what the manifest recorded, and no two
  archives sharing one — the same provenance/mutation/duplicate checks
  `corpus/addons/`'s inventory mode runs.
- The archive is a valid zip with no entry carrying an absolute path or a
  `..` component. Nothing is ever extracted to disk by the checker — every
  shape check below reads member bytes/names in-memory via `zipfile`.
- An `mrpack`-format archive contains a genuine `modrinth.index.json` at
  its root (non-empty `game`/`versionId`/`name`/`dependencies`), and every
  other entry falls under one of the three known override roots
  (`overrides/`, `client-overrides/`, `server-overrides/`).
- A `curseforge`-format archive contains a genuine `manifest.json` at its
  root (`manifestType == "minecraftModpack"`, non-empty
  `minecraft.version`/`minecraft.modLoaders`/`name`/`version`/`overrides`),
  and the folder its `overrides` field names actually appears in the
  archive.

The checker's own passing and deliberately-broken self-test cases live
under `tools/phase8/fixtures/` instead, precisely so nothing invented ends
up here standing in for the real thing.
