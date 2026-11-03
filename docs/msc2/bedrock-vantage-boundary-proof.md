# P18.82a — Bedrock input at the shared Vantage boundary

The optimized, read-only `anvil-probe` converted a **4×4-chunk** sample from
Bedtonic's existing 7.5 GB private snapshot into a new private Anvil region.
Vantage's `meshtex` accepted it and produced a textured, detailed `.vtile`:
16 loaded chunks, 64×192×64 blocks, 92,908 vertices, 46,454 triangles, and
1,388 water vertices. The resulting tile was 2.2 MB. This establishes that
Bedrock terrain can enter the **same detailed renderer** used by Java after a
private input adapter. It does **not** establish visual parity yet.

| Bounded sample | Snapshot open | Scan / read saved chunks | Encode private Anvil | Vantage detailed tile |
|---|---:|---:|---:|---:|
| Bedtonic Overworld, chunk 0,0 through 3,3 | 29.86 s | 0.056 / 0.033 s | 0.93 s | 0.76 s, 33 MB peak RSS |
| Bedtonic Nether, same coordinates | 26.30 s | 0.030 s | 0.078 s | 4 loaded chunks, 7,170 triangles |
| Bedtonic End, same coordinates | 29.63 s | 0.022 s | 0.128 s | 4 loaded chunks, 304 triangles |
| Prepared Java Tectonic, a different 4×4 sample | already prepared | — | — | 0.21 s, 46 MB peak RSS, 34,032 triangles |

The Java tile is a renderer check on a different region, not an equivalent
terrain benchmark. The Java world was already in Vantage-compatible format.
The Bedrock times exclude making the private snapshot and do not predict the
cost of converting the full 7.5 GB world. A full-region inventory and
conversion throughput measurement remain necessary before setting a first-open
expectation. In this sample, LevelDB open dominated. A production adapter
should open the snapshot once and process many regions through that handle.
The optimized helper's peak resident memory was about 22 MB, and the private
Overworld `.mca` was 144 KB for the 16-chunk sample.

The Overworld sample contained 89 Bedrock block names, four numeric biome IDs
(`25`, `35`, `42`, `189`), eight sections with secondary block layers, and
sections from Y -4 through 19. All 16 chunks decoded as modern palettes; no
section format was rejected. The source names `waxed_copper`,
`wooden_button`, `web`, and `mob_spawner` lacked matching Java blockstate
assets (1,881 block positions in this sample). The current probe deliberately
omits Bedrock state properties and secondary layers and assigns the Java plains
biome to every section, so foliage tint, stairs, fluids, orientation, and
waterlogged blocks cannot be judged from its tile. The biome registry must be
matched to the saved Bedrock version before names or tints can be trusted.
Bedrock resource packs and custom blocks also require an explicit asset
mapping or visible fallback; official Java assets alone are insufficient.
The Nether/End samples prove their section ranges and region encoding work;
they do not cover the full dimensions or prove roof/cave presentation.

**Selected boundary:** a private Bedrock-to-modern-Anvil adapter feeding
Vantage's existing region reader. P18.83 adds the full-region `anvil-convert`
path, carries block state properties and 3D biome cells, preserves the three
standard dimensions, and accepts an exact-save-version biome registry when
available. Without one, unknown Bedrock biome IDs use Vantage's neutral Plains
fallback. Its
atomic region files and manifest resume safely after interruption. Resuming
currently still scans and decodes source chunks to compute content digests;
the manifest avoids recompressing and rewriting unchanged region files, not
the LevelDB read. The conversion report exposes block aliases, biome coverage,
vertical range, and sections with secondary layers. Those layered sections
currently write the first visible Bedrock layer; their count remains a known
fidelity limitation for later review. The existing 1.26.31.1 Bedtonic proof
does not contain an authoritative biome registry, so named biome tint fidelity
and practical first-open timing remain unverified. Do not route the production
Bedrock map through the bounded `anvil-probe`; app routing and its progress
panel are P18.84.

Reproduce the bounded proof with an optimized helper and a **new** output
directory separate from the saved world:

```sh
cargo build --manifest-path tools/world-map-proof/Cargo.toml --release
tools/world-map-proof/target/release/msc-world-map-proof anvil-probe \
  "$HOME/Library/Application Support/MSC 2/map-staging/<snapshot>/world" \
  /private/tmp/msc-map-boundary-example 0 0 overworld
clients/desktop-web/src-tauri/target/x86_64-apple-darwin/release/agent/vantage meshtex \
  /private/tmp/msc-map-boundary-example/region/r.0.0.mca \
  /private/tmp/msc-map-boundary-example/sample.vtile \
  "$HOME/Library/Application Support/MSC 2/map-dependencies/java-assets/26.3/assets/minecraft" \
  0 0 3 3
```

The bounded probe writes `probe.json`; it is only a renderer-boundary proof.
For durable conversion, build the optimized helper. An exact-version registry
JSON (`version`, `source`, `source_sha256`, and unique `ids` mapping names to
numeric IDs) gives named biome tints. Without it, unknown numeric IDs use the
Vantage neutral Plains fallback; the report still lists each unmapped ID and
its block-position count.

```sh
cargo build --manifest-path tools/world-map-proof/Cargo.toml --release
tools/world-map-proof/target/release/msc-world-map-proof anvil-convert \
  "$HOME/Library/Application Support/MSC 2/map-staging/<snapshot>/world" \
  "$HOME/Library/Application Support/MSC 2/map-cache/bedrock-anvil/<world-id>" \
  [/path/to/exact-bedrock-biome-registry.json]
```

If a registry is supplied, the converter refuses a version mismatch. It writes
`conversion-report.json`, `bedrock-conversion.json`, and dimension region
files. No full Bedtonic conversion or test suite was run for P18.83; Cameron's
release-build conversion, fidelity review, and timing are still needed before
marking it Done.

### Bounded conversion measurement after P18.84c

`anvil-convert` can select one to eight saved regions without converting the
rest. It still indexes the immutable snapshot once, so the log separates that
cost from conversion. Run the optimized helper on a private snapshot with
separate new output directories for each comparison. The `--server-running`
option forces one region worker for a controlled single-worker comparison; it
does not start the Minecraft server. The production agent supplies that option
when BDS is running and restarts the helper at one worker if BDS starts during
conversion, retaining regions already published to its private staging area.

```sh
SNAPSHOT="$HOME/Library/Application Support/MSC 2/map-staging/<snapshot>/world"
HELPER=tools/world-map-proof/target/release/msc-world-map-proof
"$HELPER" anvil-convert "$SNAPSHOT" /private/tmp/msc-bedrock-sample-one \
  --server-running \
  --region=overworld:0:0 --region=overworld:0:-1 \
  --region=overworld:-1:0 --region=overworld:-1:-1
"$HELPER" anvil-convert "$SNAPSHOT" /private/tmp/msc-bedrock-sample-pool \
  --region=overworld:0:0 --region=overworld:0:-1 \
  --region=overworld:-1:0 --region=overworld:-1:-1
```

Use fresh output directory names for an uncached comparison. Repeat against
the same output to observe verified region reuse. Logs report LevelDB open,
indexing, worker capacity, aggregate worker time spent reading chunks,
encoding NBT and compressing regions, and wall time spent publishing files and
syncing the manifest. Aggregate worker times may exceed elapsed wall time when
workers overlap. Progress reports each completed region; no time estimate is
promised from a sparse or dense four-region sample. Check that selected regions
exist before running; a missing selection fails without converting others.
