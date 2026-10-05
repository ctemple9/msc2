# Forge 1.20.1 capture fixture — build verified, owner capture pending

P18.20 preparation, 2026-10-05. **Not a runnable or shipped repair helper.**
The first compilation failed; the authorized Forge access-rule correction now
compiles and packages successfully. No game was launched, no capture exists,
and no production rendering support is claimed.

## Pinned public fixture

Cameron authorized selecting a public Forge fixture rather than requiring an
existing private Forge instance. `pins.json` records exact provider release/file
IDs, published hashes, downloaded SHA-256/size, game/loader artifacts and tools.
Original game/mod JARs remain outside Git.

- Minecraft 1.20.1 / Forge 47.4.10, the recommended build on the
  [official release page](https://files.minecraftforge.net/net/minecraftforge/forge/index_1.20.1.html).
- [Supplementaries release S0TIJ1hU](https://modrinth.com/mod/supplementaries/version/S0TIJ1hU),
  declared mod version `1.20-3.1.43`.
- [Moonlight release W0ZWjZib](https://modrinth.com/mod/moonlight/version/W0ZWjZib),
  declared mod version `1.20-2.16.35`. The Supplementaries JAR requires at least
  `1.20-2.16.26`; this selected release satisfies that declared dependency.
- ForgeGradle 6.0.54, Gradle 8.8, Java 17. Gradle's archive SHA-256 is pinned.

This older Supplementaries release has no barnacles model. Its goblet at
`9,65,3` selects the actual `supplementaries:goblet` model loader. The expected
context pair is two pedestals at `3,65,3` and `6,65,3` displaying a diamond and
emerald. These are intended fixture positions, **not observed rendering passes**.

The draft adapts the isolated NeoForge capture proof to Forge's older vertex
API and registered baked-model/block-entity renderers. Only a marked private
client and a new world named `MSC Forge Capture Fixture` are eligible. It has
no production world import, remote correspondence or supplemental-map adoption.
ForgeGradle remaps the two exact original mod JARs as development runtime
dependencies; the ordinary Prism instances are never modified. The original
NeoForge proof and its evidence remain untouched.

Unlike the newer game's tick-freeze API, 1.20.1 needs the draft pause-based
capture path. Its save/context checks and paused-frame behavior need actual
runtime evidence after compilation is corrected. No freeze/runtime claim is
made from this source.

## Build-only verification and the corrected access rules

The attempted build-only command was:

```bash
JAVA_HOME=/usr/lib/jvm/temurin-17-jdk python3 tools/java-map-export/forge-1.20.1/fixture.py build --gradle /home/camerontemple/.cache/msc-map-forge-capture-inputs/gradle-8.8/bin/gradle --workspace /home/camerontemple/.cache/msc-map-forge-capture
```

Only `jar` and `reobfJar` were requested; compilation stopped at `compileJava`
with 13 access errors and three deprecated-constructor warnings. An earlier
toolchain attempt used the host's Java 25 default and failed before source
compilation because Gradle 8.8 cannot parse that class-file version. Explicit
Java 17 got past that setup problem; source compilation still failed.

The copied access rules use readable mapped Minecraft member names. Forge's
[1.20.1 access-transformer contract](https://docs.minecraftforge.net/en/1.20.1/advanced/accesstransformers/)
requires SRG names (the stable names used by this Forge toolchain) for Minecraft
fields/methods. The generated `srg_to_official_1.20.1.tsrg` establishes:

| Readable member | SRG member |
|---|---|
| `RenderStateShard.name` | `f_110133_` |
| `CompositeRenderType.state()` | `m_173265_` |
| `CompositeState.textureState` | `f_110576_` |
| `EmptyTextureStateShard.cutoutTexture()` and both overrides | `m_142706_` |
| `LightTexture.lightPixels` | `f_109871_` |

Cameron explicitly authorized the correction and compilation retry. All those
rules now use the recorded SRG member names, including the missing
`RenderStateShard.name` rule. The command above passed `compileJava`, `jar` and
`reobfJar` on this Linux host, retaining three deprecated-constructor warnings.
The driver checked the exact game/loader artifacts and recorded the resulting
JAR identity in the private workspace build receipt. No tests or game tasks ran.

Java 17 selection still needs a portable driver; the explicit Linux invocation
is local build evidence, not an all-platform command or runtime pass.

`fixture.py run` would launch Minecraft, and must not be run by an agent under
a build-only instruction. Its runtime behavior remains unverified.
The draft viewer/validator likewise have no generated capture to inspect.
Remaining P18.20 work includes Fabric/NeoForge production adapters, private
snapshot correspondence, bounded portable bundles, host validation/adoption,
native controls, CLI, packaging and successful owner rendering evidence.

No tests were added or run. No live client, source world, release, tag or
workflow was changed. Downloaded Gradle/mod inputs occupy about 293 MiB in
the private cache; the newly populated ForgeGradle cache measured about
316 MiB, with about 72 MiB of ignored project build output at the failed check.
