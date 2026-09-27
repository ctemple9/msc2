# Exact-release gameplay rules

The agent embeds `crates/msc-infrastructure/data/gamerules/catalogs.json`.
Individual catalogs retain their source URL/checksum. Neither the API nor the UI
substitutes a nearby version. Unsupported versions retain manual entry.

## Java

Use Python 3 and a JDK providing `javap`:

```sh
python3 tools/gamerules/extract-java.py 1.21.11 --cache /tmp/msc-rule-extraction
```

This downloads the official release manifest, server jar, mappings when present,
and matching client language file. Download SHA-1 values must match Mojang's
manifest. `javap` reads server registration bytecode; it does not run Minecraft
classes. Registration names, types, defaults, experiment gates, and exposed
integer bounds come from that exact jar. Labels/descriptions use the matching
client language data, with a plain description fallback where Mojang supplies
none. Unrecognized registration shapes abort extraction and need inspection.

Coverage is 25 exact releases from 1.19.4 through 26.3, including each bundled
1.20 and 1.21 release. This is the complete built-in registration list, including
experiment-gated rules. Mods/plugins can add rules that are not in these vanilla
catalogs; MSC preserves manual entries and does not claim to discover extensions.

## Bedrock

Coverage is BDS 1.26.52.3, build 51798919: 39 native rules. The source was a fresh
disposable world with no packs, querying `gamerule` and `help gamerule` after
startup and then stopping cleanly. User worlds and the installed service were
not changed. The catalog records the source executable's SHA-256.

To add a release, use a separate temporary working directory, a fresh level name,
and the exact official BDS binary/resources. Capture the version/build banner,
fresh-world banner, native enumeration, help, and clean shutdown. Do not expose
a scratch server to players or reuse an existing world to obtain defaults.
Import that transcript:

```sh
python3 tools/gamerules/import-bedrock.py --version 1.26.52.3 \
  --binary /path/to/bedrock_server --transcript /tmp/bds-rules.txt
```

Inspect newly introduced native choice types, bounds, aliases, and experiment
requirements before publishing another catalog. The importer preserves curated
labels for an existing release and rejects unfamiliar value schemas. Bedrock's
Locator Bar and Player Waypoints are two forms of the same control; the world
picker replaces one override when selecting the other, and validation rejects
conflicting manual values. Coordinates uses the existing dedicated world toggle.

Both scripts rebuild the embedded combined bundle. Catalog updates ship with the
agent, so an offline client and remote client use the same agent-owned data.
Creating a server resolves Latest to an exact release before requesting its
catalog and retains that selection for the download. A missing catalog never
blocks creating a server with Minecraft defaults or manually entered rules.
