See `../README.md`. Populated by P8.3 with real, live-captured Modrinth,
Hangar, GitHub Releases, direct-download, and CurseForge responses
(captured 2026-08-21). This note (P8.2) fixed the shape that evidence must
arrive in, and the checker that gates it, before any of it was collected —
the same ordering `tools/phase7/provider-corpus-check.py` and
`tools/phase6/corpus-check.py` used for their own corpora.

## What's recorded (P8.3, captured 2026-08-21)

- **`modrinth/`** — `search-sodium.json` (a real `/v2/search` call),
  `project-iris.json` (`/v2/project/iris`), `dependencies-iris.json`
  (`/v2/project/{id}/dependencies` — Iris genuinely requires Sodium, a real
  `required`-type dependency edge), `version-list-iris-fabric-1.21.1.json`
  (`/v2/project/{id}/version` filtered by loader/game-version, the shape
  update-checking reads), and `version-file-hash-iris.json`
  (`/v2/version_file/{sha512}` — the exact-identity lookup, resolved from a
  hash taken out of the version-list response above).
- **`hangar/`** — `project-essentials.json` and
  `versions-latest-essentials.json`, both real responses for the
  EssentialsX project (`hangar.papermc.io`, project slug `Essentials`).
- **`github/`** — `releases-latest-essentialsx.json`, a real
  `/repos/{owner}/{repo}/releases/latest` response (asset-name shapes for
  the GitHub add-on source).
- **`direct/`** — `luckperms-bukkit-direct-download.json`. Unlike the other
  four, "direct" isn't a JSON API — `PluginSourceDetector.detect` only
  classifies a URL string (any `http(s)` or `.jar`-suffixed URL not
  matching the three named domains), and the actual byte transfer is Phase
  9's `PluginDownloader`. This file instead records a real captured HEAD
  response (status, `content-type`, `content-length`, filename) against a
  genuine direct-download URL (LuckPerms' own download host), so the
  direct-source shape is evidence-backed rather than invented.
- **`curseforge/`** — captured with a real, Cameron-supplied CurseForge
  Core API key (`x-api-key`), same as `CurseForgeAPI.swift`'s own gate.
  `mods-files-blocked-entityculling.json` is a real `POST /v1/mods/files`
  response for a genuinely author-blocked file (Entity Culling
  Fabric/Forge, modId `448233`, `allowModDistribution: false`, file
  `8287121`) — `downloadUrl` is `null` while `isAvailable` is `true`,
  exactly the shape the D-027 pending-file workflow needs to characterize
  against. `mods-metadata-entityculling.json` is the matching
  `POST /v1/mods` response for that same mod (name/slug/`websiteUrl`, used
  to build the manual-download list). `mods-files-resolvable-fabulously-
  optimized-pack.json` is a real, non-blocked `POST /v1/mods/files`
  response (modId `396246`, file `8439077`) for contrast — an ordinary
  resolvable `downloadUrl`, the common case the blocked case is the
  exception to.

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
