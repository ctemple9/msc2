See `../README.md` (§ `packs/`). This note (P8.2) fixed the shape that
evidence must arrive in, and the checker that gates it, before any of it
was collected — the same ordering `tools/phase7/provider-corpus-check.py`
and `tools/phase6/corpus-check.py` used for their own corpora.

## What's recorded (P8.3, captured 2026-08-21)

- **`fabulously-optimized-v13.3.0.mrpack`** — a real, unmodified Modrinth
  pack download (`Fabulously Optimized` v13.3.0, Modrinth project
  `1KVo5zza`, version `Jng8txuM`), fetched directly from
  `cdn.modrinth.com` and verified against Modrinth's own recorded SHA-512
  before being added here. 48 mod entries under `overrides/mods/`, mixed
  `env.client`/`env.server` requiredness per file (useful for P8.6/P8.7's
  client-only-classification fixtures later), genuine `fabric-loader`/
  `minecraft` version pins in `modrinth.index.json`. Small enough
  (~152 KiB) to keep in git directly — no out-of-git storage needed for
  this one.
- **`fabulously-optimized-v13.3.0-curseforge.zip`** — the same pack,
  same version, in CurseForge format instead (CurseForge modId `396246`,
  file `8439077`), fetched with a real CurseForge Core API key via
  `POST /v1/mods/files` → `downloadUrl` → `edge.forgecdn.net` and verified
  by SHA-256 before being added here. Genuine `manifest.json`
  (`manifestType: minecraftModpack`, `minecraft.version`/`modLoaders`,
  48-entry `files` list of `projectID`/`fileID` pairs) plus a real
  `overrides/` root. Picking the same underlying pack as the `.mrpack`
  above (rather than an unrelated CurseForge pack) means both archives can
  be cross-checked against each other, not just against their own format
  rules. ~146 KiB, kept in git directly.

Note: `fixtures/curseforge-modpack/` and `fixtures/modpack-pinning/`
(from P0.16/P0.18) already carry synthetic, oracle-derived BMC4-shaped
fixtures — those remain independent of this corpus (`corpus_source` is
optional) and are not superseded by the smaller real pack recorded here.

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
