See `../README.md`. **Not yet populated — needs P7.3's real evidence.**
This directory holds real, recorded provider responses (Paper fill v3,
Purpur, Mojang's version manifest, Fabric meta, the NeoForge/Forge Maven
`maven-metadata.xml` listings) plus the on-disk shape a real Forge and a
real NeoForge installer leave behind, per `rolling-plan.md`'s P7.3 step.
This note (P7.2) fixes the shape that evidence must arrive in, and the
checker that gates it, before any of it is collected — the same ordering
`tools/phase6/corpus-check.py` and `tools/phase5/real-corpus-check.py`
used for their own corpora.

`tools/phase7/provider-corpus-check.py` (P7.2) is the dependency-free gate.
It has two modes:

## Inventory mode

`python3 tools/phase7/provider-corpus-check.py --inventory [--providers DIR]`
(default `DIR`: `corpus/providers`). Checks this directory against its own
`manifest.json`. Requires, for every recorded file in the tree (anything
except `manifest.json` and `README.md`):

- A `manifest.json` entry keyed by the file's path relative to this
  directory, recording:
  - `family` — one of the six this phase's create flow offers: `vanilla`,
    `paper`, `purpur`, `fabric`, `neoforge`, `forge`. Any other value
    (`pufferfish`, `spigot`, `bedrock`, a typo) fails loudly — coverage
    mode below depends on every recorded response being attributed to a
    real family.
  - `source_url` — the exact URL the response was captured from.
  - `captured` — the capture date.
  - `sha256` — the file's SHA-256 at capture time.
  - `byte_size` — the file's size in bytes at capture time.
- The recomputed SHA-256 matching what the manifest recorded — an input
  that changed after being catalogued fails loudly rather than silently
  drifting from what the manifest claims.
- No two files sharing a SHA-256 — a duplicate isn't a second sample.
- Every `.json` file parsing as JSON, every `.xml` file (the Forge/NeoForge
  `maven-metadata.xml` shape) parsing as XML. Other evidence — an args
  file's contents, a run script, a `libraries/` directory listing — isn't
  JSON or XML and isn't parsed as either.

## Coverage mode

`python3 tools/phase7/provider-corpus-check.py --coverage FIXTURE_DIR
[--providers DIR]`. Checks a fixture directory (e.g.
`fixtures/server-jar-providers/`, built in P7.4) against this corpus:

- A fixture may carry an optional top-level `corpus_source` field — a list
  of paths, relative to `corpus/providers/`, naming which recorded
  response(s) it was characterized from. This is additive to the six
  fields `docs/msc2/fixture-format.md` defines; existing fixture tooling
  ignores fields it doesn't know about, so no fixtures actually need to
  change until they choose to carry this.
- Every path a fixture cites must have a real manifest entry here — a
  fixture cannot claim a response that was never recorded.
- Across every fixture in the directory, all six families must be cited by
  at least one fixture — silently skipping one (e.g. never characterizing
  against a real Forge response) fails coverage even if every citation
  that *is* present is genuine.

## Directory convention

`<family>/<descriptive-name>.<ext>` for a single recorded response (e.g.
`paper/projects-paper.json`, `forge/maven-metadata.xml`); installer
evidence that isn't a single response — the args file's name and its
`@`-file contents, the `libraries/` layout, the run scripts a real Forge or
NeoForge install produces — goes under `<family>/installer-evidence/`.
Large binaries (a `libraries/` tree's actual jars) don't belong here; P7.3
should capture their *shape* (a manifest-recorded file listing relative
paths and sizes) rather than the jars themselves, the same
truncate-and-document-it approach P7.3's own step text already requires
for long version arrays.

The checker's own passing and deliberately-broken self-test cases (both
modes) live under `tools/phase7/fixtures/` instead, precisely so nothing
invented ends up here standing in for the real thing.
