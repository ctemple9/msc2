See `../README.md`. **Populated by P7.3** with real, recorded provider
responses (Paper fill v3, Purpur, Mojang's version manifest, Fabric meta,
the NeoForge/Forge Maven `maven-metadata.xml` listings, plus Forge's
`promotions_slim.json`) and the on-disk shape a real Forge and a real
NeoForge installer leave behind, captured 2026-08-18. This note (P7.2)
fixed the shape that evidence must arrive in, and the checker that gates
it, before any of it was collected — the same ordering
`tools/phase6/corpus-check.py` and `tools/phase5/real-corpus-check.py`
used for their own corpora.

## P7.3 findings

- **Forge's own `maven-metadata.xml` under-reports its newest version.**
  Its `<latest>`/`<release>` tags read `1.21.5-55.1.13`, but the `<versions>`
  array itself already contains a newer `1.21.11-61.0.0` entry. This is
  presumably why the oracle's `latestRecommendedVersion()` reads
  `promotions_slim.json` instead of trusting the metadata tag — recorded
  here, and `promotions-slim.json` captured alongside it, even though the
  step's file list named only `maven-metadata.xml` for Forge.
- **A stale negative CDN cache briefly made NeoForge's Maven return 404**
  for `maven-metadata.xml` (Reposilite via CDN77, `x-77-cache: HIT`,
  `age: 19`) even though the file demonstrably exists (confirmed via the
  directory listing and a cache-busted request). Retried and captured
  successfully; not a real shape change or outage, so P7.3 did not stop.
- **Minecraft's own versioning has moved from `1.x` to a `YY.n` scheme**
  (current release `26.2`, e.g. via Paper's and Fabric's `stable` game-version
  lists and Mojang's own `version_manifest_v2.json`). The oracle's
  `compareMCVersions` already special-cases this ("including the new
  26.x.x scheme"), so this is a live-data fact worth knowing, not a parse
  break — Fabric's `firstStableVersion()` as written would now resolve
  `downloadLatest()` to `26.2`, not a `1.x` release. Purpur's `listVersions()`
  filters to `1.`-prefixed versions and Vanilla's does not; both behaviors
  are captured as-is for P7.4/P7.10 to characterize, not corrected here.
- **Minecraft 26.2 requires Java 25** (`javaVersion.majorVersion` in its
  piston-meta JSON), matching the Temurin 25 JDK already on this machine —
  worth carrying into P7.7's runtime-selection characterization.
- **NeoForge's and Forge's real installers produce a byte-identical
  `user_jvm_args.txt`** at the server directory root (same boilerplate
  template in both installers). Recorded once, under `forge/`, with a note
  on both installers' manifest entries — a second copy would trip the
  checker's duplicate-hash rule for the right reason (it isn't a second
  sample).

`tools/phase7/provider-corpus-check.py` (P7.2) is the dependency-free gate.
It has three modes:

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

## Evidence mode (P7.28)

`python3 tools/phase7/provider-corpus-check.py --evidence [DIR]` (default
`DIR`: `docs/msc2/families/provisioning-evidence`). Independent of this
corpus -- it checks the real-provisioning evidence P7.28 recorded there
against the live internet, not against anything captured here. See that
directory's own `README.md` for what was found. Requires exactly one
`<family>.json` per family, `reached_ready: true` in every one, and the
rest of the shape documented in the checker's own module docstring.

## P7.28 findings

Real provisioning against the live internet (2026-08-19, `docs/msc2/
families/provisioning-evidence/`) confirmed every finding above still holds
one day later against live data -- Forge's `maven-metadata.xml`
under-report, the `26.2` `YY.n` release, Java 25 required -- and surfaced
three more, live-data facts rather than corpus-shape gaps, so recorded here
rather than in this corpus's own manifest:

- **Checksum shape genuinely differs per real provider.** Mojang publishes
  SHA-1 per version, Paper's fill v3 publishes SHA-256, Purpur's per-build
  API publishes MD5 only, and Fabric's composed download endpoint plus
  NeoForge's/Forge's Maven publish no checksum for the jar/installer they
  serve at all.
- **The Mojang EULA gate is real, live, and unbypassed.** A freshly
  provisioned vanilla server's first real boot refuses to start until a
  human flips `eula.txt`'s `eula=false` to `true` -- confirms MSC2 doesn't
  (and shouldn't) auto-agree on the operator's behalf.
- **Forge's and NeoForge's real installers delete their own installer jar
  on success**, matching MSC 1's behavior -- a post-hoc checksum of
  precisely the bytes MSC2 consumed isn't possible for those two families;
  P7.28's evidence documents this rather than treating it as a gap.

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
