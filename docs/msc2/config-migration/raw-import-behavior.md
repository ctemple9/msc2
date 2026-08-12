# MSC 1 raw server-directory import — read-only scan behavior

Characterizes the read-only half of `AppViewModel+ServerImport.swift` (511
lines) before any Rust translation, per P5.18: `scanServerDirectory` and the
`detectJavaFlavor` family it calls. The mutating half — `importExistingServer`
(copy/unzip into the owned servers root, write overrides, create a world
slot) — is P5.20's job and is out of scope here beyond what's needed to
explain where the zip-unwrap step actually lives (see below).

No MSC 1 test file exercises `scanServerDirectory`, `detectJavaFlavor`, or
any of their private helpers — confirmed by a whole-tree grep for the
function names against every `*Tests*.swift` file, none matched. This is
the same situation P5.12's transfer-package characterization hit
(`transfer-package-format.md`), and the same precedent applies: the 16
fixtures in `fixtures/raw-server-import/` are characterized straight from
source, and `source.test` in each names the function and behavior under
test, not an XCTest.

## Fixture input/expected shape

Fixtures in this domain use a directory-tree description distinct from
`fixtures/paper-import/`'s (Phase 4, top-level-only `entries`/`files`):

```json
"input": {
  "serverDir": "/srv/example",
  "entries": [ { "path": "relative/nested/path", "kind": "file" | "directory" } ],
  "files": { "relative/path": "text file content" }
}
```

`entries` may nest arbitrarily deep (needed for e.g.
`libraries/net/neoforged/neoforge/<version>/unix_args.txt`); a file entry
may also carry `"sizeBytes"` where a fixture exercises size aggregation.
One fixture (the zip-unwrap case) describes a zip archive's extracted
contents instead, under `extractedEntries`/`extractedFiles`, since that
input isn't a plain already-resolved directory.

`expected` mirrors `ScannedServerInfo`'s real field names one-to-one —
`serverType`, `port`, `maxPlayers`, `eulaAccepted`, `worlds`,
`defaultWorldName`, `javaFlavor`, `detectedMCVersion`,
`detectedLoaderVersion` — plus `folderPath`/`sizeBytes`/`hasNether`/
`hasEnd` per `DetectedWorld`. Per the step's own instruction, every
inferred field distinguishes three cases in these fixtures: an **observed**
value (read off a file), an **MSC 1 default** (explicitly documented as
such, e.g. port 25565), and **genuinely undetermined** data (null, e.g. an
unmatched jar's version). Where a field lands in the "default" bucket
rather than "undetermined," the fixture's `notes` says so explicitly —
this distinction is easy to get wrong by porting every empty case to
`null`.

## Server-type selection

```swift
let hasJar = contents.contains { $0.lowercased().hasSuffix(".jar") }
let hasBedrock = contents.contains { $0 == "bedrock_server" || $0 == "bedrock_server.exe" }
let detectedType: ServerType = (hasBedrock && !hasJar) ? .bedrock : .java
```

(source line 241-246). Bedrock is selected only when a `bedrock_server`
(or `.exe`) binary is present **and** no jar is present. A jar always wins
if both exist — there is no ambiguity error. An empty directory (neither)
also resolves to `.java`, by the same formula. See
`bedrock-selected-when-only-binary-present.json`,
`java-selected-when-jar-and-bedrock-binary-both-present.json`, and
`missing-jar-and-binary-still-classified-java.json`.

## Java flavor detection order

`detectJavaFlavor(in:)` (source line 370-437) checks signatures in this
fixed order, returning on the first match:

1. **NeoForge** — `libraries/net/neoforged/neoforge/<version>/unix_args.txt`
   exists. `mcVersion` comes from `NeoForgeInstaller.minecraftVersion(forNeoForge:)`
   (line 224-231): split the version on `-`, take the numeric
   `major.minor` prefix, and render `1.<major>.<minor>` (or `1.<major>` if
   minor is 0) — unless `major >= 26`, a forward-compatibility branch for
   Minecraft's post-1.21 flat-year versioning scheme, which instead
   renders `<major>.<minor>` directly. `primaryJarPath` is always `nil`
   for NeoForge — it launches via `unix_args.txt`, not a jar. See
   `neoforge-detected-via-unix-args-signature.json`.
2. **Forge** — `libraries/net/minecraftforge/forge/<mc>-<forgeVersion>/unix_args.txt`
   exists. The directory name is split on the first `-` only
   (`maxSplits: 1`) into `mcVersion`/`forgeVersion`. See
   `forge-detected-via-unix-args-signature.json`.
3. **Fabric** — a root `fabric-server-launch*.jar`. `mcVersion` parses
   from the jar stem's `fabric-server-launch-` prefix.
   `detectFabricLoaderVersion` (line 450-456) lists
   `.fabric/server/libraries/net/fabricmc/fabric-loader/` subdirectories
   and returns `.sorted().last` — **a lexicographic string sort, not a
   semantic-version comparison**. See
   `fabric-launcher-and-loader-version-lexicographic-quirk.json`, which
   exercises a case where this actually changes the answer
   (`"0.15.9"` sorts before `"0.9.0"` as strings, so `.last` picks the
   numerically older `"0.9.0"`). This is preserved as-is, not corrected —
   MSC 1 is the oracle (`CLAUDE.md`).
4. **Purpur** — root jar filename prefix `"purpur"`. See
   `purpur-jar-name-matched.json`.
5. **Vanilla** — root jar filename prefix `"minecraft_server"`. See
   `vanilla-minecraft-server-name-matched.json`.
6. **Paper (default)** — `jars.first(where: hasPrefix "paper") ?? jars.first`.
   If no jar's name starts with `"paper"`, whatever jar exists is still
   labelled Paper. If there are no jars at all, `paperJar` is `nil` and
   the flavor is still `.paper` with a `nil` jar and `nil` version. See
   `unmatched-jar-falls-back-to-paper.json` and
   `missing-jar-and-binary-still-classified-java.json`.

`parseJarMCVersion(_:prefix:)` (line 461-473) requires the stem to start
with the exact given prefix (e.g. `"paper-"`, `"purpur-"`,
`"minecraft_server-"`) and strips a trailing `-<numeric build>` if
present. A jar that doesn't carry the expected prefix — even if it was
*matched* as that flavor via the looser `hasPrefix("paper")` fallback —
yields a `nil` version rather than a guess.

## Java and Bedrock properties

Both branches read the raw key=value dictionary directly
(`ServerPropertiesManager.readProperties` / `BedrockPropertiesManager.readRawProperties`)
and apply the *same* inline fallbacks in `scanServerDirectory` itself
(line 248-250, 259) — **not** either manager's own typed model defaults:

| Field | Java default | Bedrock default |
|---|---|---|
| `port` | 25565 | 19132 |
| `maxPlayers` | 20 | **20** |
| `level-name` (used for `defaultWorldName`/sort) | `"world"` | `"world"` |

The Bedrock `maxPlayers` default is the one easily-missed divergence
here: `BedrockPropertiesModel.maxPlayers` (`BedrockPropertiesManager.swift:25`)
defaults to **10**, but `scanServerDirectory` never calls
`BedrockPropertiesManager.readModel` — only `readRawProperties` — so a
freshly-dropped-in Bedrock folder with no `server.properties` yet scans as
20 max players, not 10. See
`bedrock-missing-properties-use-defaults-not-typed-model.json`. Ordinary
reads are `java-properties-port-maxplayers-levelname.json` and (implicitly,
via its default-driven values) the Bedrock fixture above.

## EULA

```swift
let eulaContent = (try? String(contentsOf: ..., encoding: .utf8)) ?? ""
let eulaAccepted = eulaContent.contains("eula=true")
```

(source line 253-256). Two things are easy to get wrong porting this:

1. **A missing `eula.txt` reads as `false`, not `nil`.** This is a real
   divergence from Phase 4's `fixtures/paper-import/` fixtures, which used
   `EULAManager.readEULA` — a genuine tri-state reader (`true`/`false`/
   `nil` for "no file yet"). `scanServerDirectory` doesn't call
   `EULAManager` at all; it does its own inline check, and the `?? ""`
   fallback means "no file" and "file says `eula=false`" both end up
   indistinguishable from "definitely not accepted." See
   `eula-missing-file-defaults-to-false-not-null.json`.
2. **The check is a raw substring match on the whole file content, not a
   per-line `eula=` parse.** `EULAManager` scans line by line for a
   `eula=` prefix; this code just asks whether the literal text
   `"eula=true"` appears anywhere in the file. In practice real
   `eula.txt` files are one or two lines and this rarely matters, but a
   port that "helpfully" swaps in `EULAManager`'s logic here would be
   changing observable behavior, not fixing a bug — noted in the fixture
   above, not separately fixtured.

## ZIP single-root unwrapping

The scan path's zip handling — extract to a disposable temp directory,
then unwrap one level if the extraction produced exactly one subdirectory
and zero loose top-level files — lives in **`AddServerWizardView.performScan`**
(line 2132-2201), not in `AppViewModel+ServerImport.swift`. The condition
itself:

```swift
if subdirs.count == 1 && files.isEmpty { scanDir = subdirs[0] }
```

(line 2185-2193) is duplicated verbatim as the private `resolvedImportDir`
helper in `AppViewModel+ServerImport.swift` (line 478-494), but that copy
belongs to the *mutating* import path (`importExistingServer`), run after
the copy/extract into the owned servers root — P5.20's oracle, not this
step's. Both copies extract via `ditto -x -k` before applying the same
one-line unwrap rule; P5.18 characterizes only the scan-path copy. See
`zip-single-root-folder-unwrapped-before-scan.json`.

## World discovery, dimension grouping, and ordering

Two search roots are unioned (line 265-300): `<serverDir>/worlds/` and
`<serverDir>` itself. Both are subject to the same rules — skip a fixed
set of non-world directory names (`plugins`, `logs`, `cache`,
`crash-reports`, `libraries`, `versions`, `mods`, `config`, `backups`,
`worlds`, `__MACOSX`), skip dotfiles, require a `level.dat` to exist
inside, and dedupe by name across both roots. See
`worlds-discovered-from-root-and-worlds-subdirectory.json`.

Vanilla dimension companions (`<name>_nether`, `<name>_the_end`) are
folded into their root world rather than listed separately (line
302-342): a companion is recognized either by a standalone sibling folder
of that name, **or** by an inline `DIM-1`/`DIM1` subfolder inside the root
world itself — this fixture set exercises only the standalone-sibling
path (the inline path is a simple existence check on the same entry, not
separately fixtured). `sizeBytes` for a grouped world sums
`directorySizeBytes` across the root folder **and** any standalone
companion folders that exist. See
`nether-and-end-companions-grouped-with-summed-size.json`.

Finally, `worlds.sort` (line 345-349) is not a plain alphabetical sort: it
puts whichever world's name equals the configured `level-name` first,
unconditionally, then falls back to `a.name < b.name` for everything
else. `defaultWorldName` is `worlds.first?.name ?? configuredLevelName` —
the configured level-name only if no world folders were found at all. See
`configured-level-name-sorted-first.json`.

## A stale fixture in `fixtures/paper-import/`, noted not fixed

`fixtures/paper-import/rejects-directory-without-java-jar.json` (Phase 4)
asserts `scanServerDirectory` throws with
`errorContains: "no Java server JAR found"` for a directory with no jar.
Current MSC 1 source contradicts this on two counts: `scanServerDirectory`
returns `ScannedServerInfo` non-throwing (it cannot produce an
`errorContains` shape at all), and the string `"no Java server JAR found"`
does not appear anywhere in the MSC 1 tree (whole-tree grep, no match).
The actual current behavior — no rejection, fall through to Paper with a
`nil` jar — is what
`missing-jar-and-binary-still-classified-java.json` in this directory
pins instead. This is flagged here and in P5.18's own step response;
fixing the Phase 4 fixture is outside this step's Files list
(`fixtures/raw-server-import/`, `docs/msc2/config-migration/raw-import-behavior.md`)
and is left for Cameron to decide how to handle.

## Where this connects downstream

- **P5.19** (read-only Java/Bedrock directory scanning) ports
  `scanServerDirectory`/`detectJavaFlavor` against every fixture in
  `fixtures/raw-server-import/`, plus `performScan`'s zip-source-adapter
  behavior for the zip fixture above.
- **P5.20** (mutating folder/ZIP import into the owned root) ports
  `importExistingServer`, including `resolvedImportDir`'s separate copy of
  the single-root-unwrap rule — noted above, not characterized by this
  step's fixtures.
- **P5.22** (`rescanAndImportServers` port) reuses `detectJavaFlavor`
  directly (`phase5-scope.md` line 55) — the flavor-detection fixtures
  here (NeoForge/Forge/Fabric/Purpur/vanilla/Paper-fallback) are shared
  oracle evidence for that step too, not raw-import-only.
