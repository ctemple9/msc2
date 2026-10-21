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
Vantage's existing region reader. The proof validates the file/mesh boundary
and avoids a second renderer. P18.83 must close the named fidelity gaps and
measure full-region throughput before this route is eligible for the normal
map. If those gates fail, reconsider native Bedrock input inside Vantage
without changing the Java viewer or its downstream cache/streaming behavior.
Do not route the production Bedrock map through this deliberately lossy probe.

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

The helper refuses an existing output directory and writes `probe.json` with
timings, saved-chunk count, block names, biome IDs, section coverage and
known fidelity omissions. It never writes to the source snapshot or server.
No test suite was run for this step. Cameron's release-build visual comparison
of matching Java/Bedrock terrain and full-region first-open timing is still
needed before marking P18.82a Done.
