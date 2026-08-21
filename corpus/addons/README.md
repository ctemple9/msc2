See `../README.md`. **Not yet populated** — P8.3 records real Modrinth,
Hangar, CurseForge, and GitHub Releases responses here (plus one
author-blocked CurseForge file, per `docs/msc2/addons/phase8-scope.md`'s
D-027 finding). This note (P8.2) fixed the shape that evidence must arrive
in, and the checker that gates it, before any of it was collected — the
same ordering `tools/phase7/provider-corpus-check.py` and
`tools/phase6/corpus-check.py` used for their own corpora.

`tools/phase8/phase8-check.py` (P8.2) is the dependency-free gate. It has
three modes; this directory is what its **inventory** and **coverage**
modes check.

## Inventory mode

`python3 tools/phase8/phase8-check.py --inventory [DIR]` (default `DIR`:
`corpus/addons`). Checks this directory against its own `manifest.json`.
Requires, for every recorded file in the tree (anything except
`manifest.json` and `README.md`):

- A `manifest.json` entry keyed by the file's path relative to this
  directory, recording:
  - `provider` — one of the five providers/dispatch cases
    `docs/msc2/addons/phase8-scope.md`'s "Provider purposes" table names:
    `modrinth`, `hangar`, `curseforge`, `github`, `direct`. Any other value
    fails loudly — coverage mode below depends on every recorded response
    being attributed to a real provider.
  - `purpose` — what the response was captured to characterize (e.g.
    `search`, `version_file_hash`, `files_batch`, `releases_latest`).
  - `source_url` — the exact URL the response was captured from.
  - `captured` — the capture date.
  - `sha256` — the file's SHA-256 at capture time.
  - `byte_size` — the file's size in bytes at capture time.
- The recomputed SHA-256 matching what the manifest recorded — an input
  that changed after being catalogued fails loudly rather than silently
  drifting from what the manifest claims.
- No two files sharing a SHA-256 — a duplicate isn't a second sample.
- Every `.json` file parsing as JSON. Every `.zip`/`.mrpack`/`.jar` file
  (an evidence sample that happens to itself be an archive, e.g. the
  author-blocked CurseForge file) being a valid, safe zip — no entry with
  an absolute path or a `..` component.

## Coverage mode

`python3 tools/phase8/phase8-check.py --coverage FIXTURE_DIR [--inventory DIR]`.
Checks a fixture directory (e.g. `fixtures/add-on-providers/`, built from
P8.4 onward) against this corpus:

- A fixture may carry an optional top-level `corpus_source` field — a list
  of paths, relative to `corpus/addons/`, naming which recorded
  response(s) it was characterized from. This is additive to the six
  fields `docs/msc2/fixture-format.md` defines; existing fixture tooling
  ignores fields it doesn't know about.
- Every path a fixture cites must have a real manifest entry here — a
  fixture cannot claim a response that was never recorded.
- A fixture may also carry a top-level `workflow` field naming which of
  Phase 8's eight symbol-ledger domains
  (`docs/msc2/addons/phase8-scope.md`'s "Symbol-ledger rows owned by this
  phase" table: `addon-updates`, `modpack-client-only`, `modpack-import`,
  `modpacks`, `modrinth-deps`, `mods`, `plugin-management`, `plugins`) it
  characterizes.
- Across every fixture in the directory, all five providers must be cited
  by at least one fixture, and all eight workflows must be named by at
  least one fixture — silently skipping one (e.g. never characterizing
  against a real Hangar response, or never characterizing
  `modrinth-deps`) fails coverage even if every citation that *is* present
  is genuine.

## Directory convention

`<provider>/<descriptive-name>.<ext>` for a single recorded response (e.g.
`modrinth/version-file-hash.json`, `hangar/latest-release.json`). The
CurseForge author-blocked file D-027 needs for P8.3 goes under
`curseforge/` with its manifest entry noting the block in `purpose`.

The checker's own passing and deliberately-broken self-test cases (all
three modes) live under `tools/phase8/fixtures/` instead, precisely so
nothing invented ends up here standing in for the real thing.
