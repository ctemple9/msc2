# MSC 1 transfer-package (`.msctransfer`) format

Characterizes `AppViewModel+ServerTransfer.swift` (603 lines) before any Rust
translation, per P5.12. No MSC 1 test file exercises any of
`exportServerTransfer`/`inspectTransferPackage`/`applyTransferImport` —
confirmed by a whole-tree grep for the type/function names against every
`*Tests*.swift` file, none matched — so the 7 fixtures in
`fixtures/transfer-package/` are characterized straight from source, the
same precedent `fixtures/config-recovery/` (P5.7) and
`fixtures/secret-migration/` (P5.8) set. `source.test` in each fixture names
the function and behavior under test, not an XCTest.

This is the v2 package format only. There is no MSC 1 v1 reader to
characterize — `ServerTransfer.formatVersion = 2` is the only value this
build of MSC 1 ever writes, and the source comment block (lines 5-28)
frames v2 as the current and only implemented design, not a migration from
some prior wire format still in the field.

## Package layout

```
<name>.msctransfer                          (zip, created by `zip -r -q -X`)
├── manifest.json
└── servers/
    └── <folderName>/
        ├── paper.jar                       Java only, when bundledPaperJar
        ├── world_slots/…                   wholesale, if present
        ├── backups/…                       wholesale, if present
        ├── plugins/…                       wholesale, if present
        ├── mods/…                          wholesale, if present
        ├── resource-packs/…                wholesale, if present
        ├── libraries/…                     NeoForge/Forge only, if present
        ├── <levelName>/                    Java live world, if present
        ├── <levelName>_nether/             Java live world, if present
        ├── <levelName>_the_end/            Java live world, if present
        ├── worlds/                         Bedrock live world, if present
        └── configs/
            └── <top-level config files>    allowlisted extensions only
```

`folderName` is a sanitized, de-duplicated slug of `displayName`
(`uniqueTransferFolderName`, source line 557-566): lowercase, spaces →
`_`, non `[a-z0-9_-]` characters stripped, truncated to 40 characters,
`"server"` if that leaves nothing, and `-2`/`-3`/… appended on collision
within one export run.

App-level settings (Java path, Remote API token/port, Xbox broadcast
account) are never in the package — those stay per-Mac (source comment,
line 20-21).

## Two easily-confused facts, pinned explicitly

1. **`ServerTransfer.excludedTopLevelDirs` (line 41-45) is a stale, unused
   constant.** It lists `logs`, `cache`, `crash-reports`, `versions`,
   `libraries`, `world`, `world_nether`, `world_the_end`, `.git`,
   `__MACOSX` — and is declared once, referenced nowhere else in the file
   or the wider codebase (confirmed by grep). It does **not** suppress
   export of `world`/`world_nether`/`world_the_end`: those are bundled
   unconditionally whenever they exist, by the separate live-world loop at
   line 195-204, which never consults this constant. `logs`/`cache`/`.git`
   are excluded only because nothing in the export code ever names them,
   not because of this list. See `fixtures/transfer-package/java-paper-full-export.json`.
   Do not port this constant as new exclusion policy (`msc2-decisions.md`
   scope note, `phase5-scope.md` "Transfer behavior").
2. **The manifest carries no world-precedence marker.** Nothing in
   `TransferManifest`/`TransferServerEntry` says "prefer live worlds" or
   "prefer the active slot." `applyTransferImport` decides purely by
   testing which folders exist in the already-copied destination directory
   at apply time (source line 465-497): live world folders present → they
   win and slot activation is never invoked; live world folders absent →
   fall back to `WorldSlotManager.activeSlot`/`activateSlot` against the
   copied `world_slots/`. See `fixtures/transfer-package/live-world-plus-slot-layout.json`
   and `fixtures/transfer-package/older-package-no-live-worlds.json`, which
   exercise the two branches of that one `if !restoredLiveWorld`.

## Manifest fields

`TransferManifest`/`TransferServerEntry`/`TransferPluginLink` are plain
Swift `Codable` with **no `CodingKeys` override**, encoded with
`[.prettyPrinted, .sortedKeys]` — so their JSON keys are the literal
Swift property names, i.e. **camelCase**. The one exception is the nested
`server` object, which is a full `ConfigServer` and therefore inherits
*its* own explicit `CodingKeys` (`AppConfig.swift:267-299`) — **snake_case**.
A reader that assumes one casing convention for the whole manifest will
silently fail to decode the embedded server. This split is pinned in
every fixture's `expected.manifest_server_entry`/`sanitized_fields_blanked`.

| `manifest.json` field | Type | Meaning |
|---|---|---|
| `formatVersion` | int | Always `2` on write. Read-side rule below. |
| `appConfigVersion` | int | `AppConfig.latestConfigVersion`, currently `1`. Informational only — nothing in the read path branches on it. |
| `createdAt` | string | ISO8601, export time. |
| `sourceMachineName` | string | `Host.current().localizedName`, falling back to `"Unknown Mac"`. |
| `servers` | array of entry | One per exported server, in `configManager.config.servers` order. |

Per-entry (`TransferServerEntry`):

| Field | Type | Meaning |
|---|---|---|
| `server` | `ConfigServer` (snake_case) | Sanitized — see below. |
| `folderName` | string | See folder-naming rule above. |
| `javaPort` | int? | Read from the *source* server's `server.properties["server-port"]` at export time; `nil` for Bedrock or on parse failure. Used only for import-time conflict detection, not applied automatically. |
| `paperMCVersion` | string? | From `PaperVersionSidecarManager`, if present. |
| `paperBuild` | int? | Same sidecar. |
| `bundledPaperJar` | bool | True only if `paper.jar` both should be bundled (`server.isJava`) and actually existed on disk at export time. Gates apply-time jar restoration too — see `fixtures/transfer-package/no-bundled-paper-jar.json`. |
| `pluginLinks` | array | One per `server.pluginSources` entry: `{filename, url, type}` (informational; not replayed as a re-download on import). |

## Sanitization (export-time)

Before an entry's `server` is written into the manifest, these fields are
overwritten (source line 241-248), regardless of server type:

| Field | Sanitized to |
|---|---|
| `serverDir` / `server_dir` | `""` |
| `paperJarPath` / `paper_jar_path` | `""` |
| `xboxBroadcastConfigPath` | `nil` |
| `xboxBroadcastAltEmail` | `nil` |
| `xboxBroadcastAltGamertag` | `nil` |
| `xboxBroadcastAltPassword` | `nil` (already Keychain-only, never JSON — blanked defensively anyway) |
| `xboxBroadcastAltAvatarPath` | `nil` |

`serverDir`/`paperJarPath` are re-rooted at import time under the target
Mac's `serversRootURL` (source line 371, 386-410, 456); `xboxBroadcastConfigPath`
is reset to `nil` again on the imported copy (line 458), independent of
whatever the sanitized manifest already carried.

## Config-file allowlist (`configs/`)

Only **top-level, non-hidden files** directly in `server_dir` (not
subdirectories) whose lowercased extension is in
`ServerTransfer.configFileExtensions` are copied into `configs/`:

```
properties, yml, yaml, json, txt, toml, conf
```

(source line 47-49, 209-220). A file with an unlisted extension, or any
directory, is silently skipped — this pass never errors.

## Wholesale bundling

These subdirectories, if present under `server_dir`, are copied whole —
no filtering inside them:

`world_slots`, `backups`, `plugins`, `mods`, `resource-packs` (source line
169, shared verbatim between export and apply). `libraries` bundles
separately and only when `server.javaFlavor` is `.neoforge` or `.forge`
(line 181, mirrored on apply at line 431).

## Live world folders

Bundled through a distinct code path from the wholesale list above —
`WorldSlotManager.worldFolderNames(for:)` (source line 195-204):

- **Java**: reads `level-name` from `server.properties`, returns
  `[level, level_nether, level_the_end]`, filtered to folders that exist
  on disk.
- **Bedrock**: returns `["worlds"]` if that directory exists, `[]`
  otherwise.

Exported **unconditionally whenever present** — there is no timestamp
comparison against `world_slots/`'s most recent snapshot, and no flag
anywhere that turns this off. See `fixtures/transfer-package/bedrock-worlds-export.json`
for the Bedrock case; `fixtures/transfer-package/java-paper-full-export.json`
for Java.

## `formatVersion` rejection rule (`inspectTransferPackage`)

```
guard manifest.formatVersion <= ServerTransfer.formatVersion else { … }
```

(source line 324). **Strictly greater is rejected**, with the exact
message:

> This transfer file was created by a newer version of MSC. Update the
> app and try again.

Equal is accepted. Nothing in the current source rejects a *lower*
`formatVersion` — "older" and "unsupported" are not synonyms here. See
`fixtures/transfer-package/newer-unsupported-format-rejected.json`.

## Port-conflict messages (`inspectTransferPackage`)

Computed by comparing each entry against the *existing* local servers'
ports, collected once before staging begins (source line 301-306,
332-337):

| Condition | Message |
|---|---|
| `entry.server.isJava`, `entry.javaPort` set, and it's in the local Java port set | `Java port <p> is already in use — edit below.` |
| `entry.server.isBedrock`, `entry.server.bedrockPort` set, and it's in the local Bedrock port set | `Bedrock port <p> is already in use — edit below.` |

Both strings are reproduced byte-for-byte here since MSC 1 renders them
directly as UI copy in `ManageServersView`.

## Inspect-time failure shape

All three of `inspectTransferPackage`'s failure paths — no `manifest.json`
found (line 314-317), `formatVersion` too new (line 324-327), and a decode
exception (line 349-352) — follow the same shape: remove the staging
directory, then return `.failure(<message>)`. Only the message text
differs between them.

## Apply-time world precedence

See "Two easily-confused facts" above for the rule itself. Mechanically,
on the Java branch: `applyTransferImport` reads `level-name` back out of
the **already-copied** `destURL/server.properties` (not the source
server's), builds the same three-candidate list export used, and copies
whichever candidates exist in the package (source line 469-484). Only if
none of those copies happened does it fall back to
`WorldSlotManager.activeSlot(forServerDir: destURL.path)` — resolved
against the package's own copied `world_slots/` and
`active_slot_id.txt`, not the source machine's original — followed by
`activateSlot(..., backupCurrent: false, ..., backupWorld: { _ in true })`
(line 497-505).

## Where this connects downstream

- **Export** (`exportServerTransfer`) — P5.13. Never gets a public HTTP
  endpoint (`phase5-scope.md` "Deferred and homeless"); it's consumed
  internally by the HTTP import handler's pre-`replaceAll` backup step
  (P5.16) and nothing else.
- **Inspect** (`inspectTransferPackage`) — P5.14. Owns path-traversal /
  absolute-path / symlink-escape rejection as new Rust-side hardening —
  MSC 1's own implementation has none of that (it shells out to
  `/usr/bin/unzip -o`); this format doc doesn't characterize hardening
  that doesn't exist in the oracle.
- **Apply** (`applyTransferImport`) — P5.15. The `older-package-no-live-worlds`
  fallback is where Phase 5 substitutes a narrow read-only compatibility
  adapter over the active-slot marker/archive for MSC 1's real
  `WorldSlotManager`, since the formal slot model stays Phase 6
  (`phase5-scope.md` "Deferred and homeless").
- **Replace-all backup ordering** — P5.16, which owns calling
  `exportServerTransfer` as a precondition before a destructive
  `replaceAll`, per `phase5-scope.md` "Transfer behavior".
