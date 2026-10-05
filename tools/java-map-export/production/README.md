# Matching-client map capture adapters

`build.py` generates and compiles MSC's matching Fabric, Forge and NeoForge
adapters. It packages code only and never launches Minecraft or runs tests.
The runtime protocol is `msc-contextual-mesh-1`; helper version is 0.2.0.

| Loader | Minecraft | Loader version | Java | Gradle |
|---|---|---|---|---|
| Fabric | 1.20.1 | 0.16.14 | 17 | 8.8 |
| Forge | 1.20.1 | 47.4.10 | 17 | 8.8 |
| NeoForge | 1.21.1 | 21.1.251 | 21 | 9.2.1 |

These are exact supported preparation targets. A successful compilation is
not a visual compatibility result for a mod, platform, or another version.
Unsupported versions are refused; the acceptance record remains authoritative.

Install JDKs 17 and 21 and set `JAVA17_HOME` and `JAVA21_HOME`, then run:

```sh
python3 tools/release/stage-map-capture.py --build-only
```

The driver obtains checksum-pinned Gradle distributions if needed. Explicit
JDK/Gradle overrides are listed by `build.py --help`. Outputs live under
`target/map-capture-helpers`; `helpers.json` records each exact pin, byte count,
SHA-256 and build source identity. A stale or incomplete set is rebuilt before
packaging. `--output-dir PATH` stages the complete payload beneath
`PATH/map-capture/0.2.0`. An explicit `MSC2_MAP_CAPTURE_HELPERS` override must
already contain a complete, source-current verified payload.

Desktop and headless release builds embed this small payload in their Rust
binaries and also include the versioned directory. This preserves delivery
when an older archive updater only knows the old fixed filenames. Discovery
can restore embedded helper files locally; it does not download or launch a
client. The agent and CLI consume validated capture data without loading mods.

Preparation uses a dedicated copy of the explicitly selected matching Prism
instance and saved context. The selected source instance and server world are
not the capture working directory. Launch requires a separate owner action.
Inside that working client, `/mscmapcapture prepare` restores the saved context;
`/mscmapcapture export` captures it. Commands refuse mismatched or unavailable
inputs rather than reporting a successful repair. Durable helper output is
validated again before import; actual appearance and repair success must be
observed in the production map. See the acceptance/design records for the
remaining platform and real-world checks.
