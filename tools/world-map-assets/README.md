# Collect private map acceptance evidence

`collect-acceptance.py` reads existing agent reports, native observations,
package/helper identities and owner confirmations. It prints JSON or Markdown
to stdout. It never installs, downloads, launches a process, runs tests,
contacts an agent, changes a fixture or writes an input file. Shell redirection
can save a new private output file. Keep worlds, packages and screenshots
outside Git.

Build-only evidence cannot fill rendering cells. The collector always leaves
the feature gate open for Cameron and subsequent independent review. It does
not implement the missing P18.20 production exporter/import/adoption path.

## Current real build evidence

This invocation reads the actual corrected Forge build receipt and JAR:

```sh
python3 tools/world-map-assets/collect-acceptance.py \
  --build "$HOME/.cache/msc-map-forge-capture/build-receipt.json" \
    tools/java-map-export/forge-1.20.1/build/libs/msc-forge-map-capture-fixture-0.1.0.jar \
  --format markdown
```

It verifies JAR SHA-256/size against the build receipt and records game/loader
pins, retaining `visual: pending`. It fills **no** transport or rendering row.
No input sessions produces the full pending matrix, rather than inferred Linux
passes. `--build RECEIPT JAR` and `--session FILE` can both be repeated.

## Actual platform observations

Export JSON from the current desktop repair sheet or from the selected agent:

```sh
msc --json world map-assets report --slot SLOT_UUID > after.json
msc --json world map-assets rendering --slot SLOT_UUID --dimension minecraft:overworld > rendering.json
```

Run those commands on the agent host, locally or through its SSH login shell.
Capture `before.json` **before** the offered repair, not after it. The collector
does not execute these commands. It reads their actual outputs and extracts
counts, operations, saved bounds, snapshot and resource/geometry generations
without transcribing those values into a matrix.

A private session file gives the platform metadata once, package checksums and
the observation-to-fixture association. All file references are relative to
that session directory; links, traversal and absolute paths are refused.
Exported JSON references can be filenames or `{ "file": "after.json",
"sha256": "…" }`; a supplied expected digest must match. Package/bundle/image
references require expected SHA-256. JSON is limited to 8 MiB, images to 32 MiB,
and package/bundle hashing to 8 GiB per file, streamed in 1 MiB blocks.

The following is a **schema sketch**, not an observation or runnable fixture;
ellipsis values must be replaced by the actual platform/artifact information:

```json
{
  "schemaVersion": 1,
  "desktop": {"platform": "linux-fedora-x86_64", "hostId": "desktop-id"},
  "agent": {"platform": "linux-ubuntu-x86_64", "hostId": "agent-id-from-report", "role": "headless"},
  "artifacts": {
    "desktop": {"file": "desktop.rpm", "sha256": "…", "platform": "linux-fedora-x86_64", "role": "desktop"},
    "agent": {"file": "headless.tar.gz", "sha256": "…", "platform": "linux-ubuntu-x86_64", "role": "agent"}
  },
  "observations": [{
    "id": "fedora-ubuntu-missing-model",
    "case": "missing-bytes",
    "flavor": "neoforge",
    "report": "after.json",
    "before": "before.json",
    "rendering": "rendering.json",
    "fixture": "fixture.json",
    "visual": "visual.json",
    "metrics": "metrics.json"
  }]
}
```

Supported desktop metadata: `macos-x86_64`, `macos-aarch64`, `windows-x86_64`,
`linux-fedora-x86_64`, `linux-ubuntu-x86_64`. Agents use the same names and
`role: desktop|headless|service`. Platform labels are supplied by the platform's
observer; hashing a portable JAR does not establish its OS/architecture. The
agent identity must equal the report's `binding.agentHostId`. The collector
exports hashed desktop/agent/world identities rather than machine names/paths.

`case` uses the fixture catalogue IDs, the repair/control/scenario IDs printed
in the generated coverage table, or `transport-import`. Transport observations
also supply a `bundle` reference with its exact transferred bytes/checksum.
The transport table expands all nine OS-family pairs to sixteen pairs covering
both macOS architectures, then includes local Fedora and local Ubuntu. Fedora
desktop → Ubuntu headless is an explicit pair. The local Ubuntu desktop label
records that installation without asserting that a headless host needs a GUI.

## Fixture identity and scoped repairs

Each private fixture receipt is created once per pinned saved area/input set
and reused across observations. Its fields are:

- `schemaVersion: 1`, `id`, `gameVersion`, and `loaderVersion` for mod loaders.
- `snapshotId`, `dimension`, `area` matching the report exactly.
- `configSha256`, `resourceSelectionSha256`, `clientArtifactSha256`,
  `serverArtifactSha256`, and `sourceSha256` (the set of actual report source
  hashes). These are observed fixture inputs, not guessed from mod filenames.
- `namedBlocks`: objects containing original namespaced `id`, full `state` and
  `position: [x,y,z]` within the checked area.
- For first-use/cache/source/version/dimension/responsiveness scenario rows,
  `scenario` identifies the specific scenario being observed.

JSON identity is calculated automatically. Before/after report counts and
operation IDs are copied into the output. A Java repair row needs failures in
the before report, the same host/server/slot/world incarnation, snapshot,
dimension and area, a `repaired` after result with matching repair evidence,
validated geometry identity, zero unresolved counts, no omitted issues and
matching ready/non-stale rendering status. Unknown classifications and changed
saved scope are refused. A download/upload, `model_resolved` count, `checked`
report, missing rendering receipt or `unsupported` result cannot satisfy it.

Revision changes are allowed for resource selection repair; a world replacement
is not. Resource generations are preserved separately from saved-world scope.
Technical eligibility still leaves the visual result pending.

## Owner visual confirmation

After Cameron compares the named blocks with Minecraft at the same positions,
this owner-only invocation prints a bound confirmation receipt without manually
copying hashes, names or states:

```sh
python3 tools/world-map-assets/collect-acceptance.py \
  --session /PRIVATE/EVIDENCE/session.json \
  --confirmation-for fedora-ubuntu-missing-model --owner-confirmed \
  --map-capture map.png --minecraft-capture minecraft.png > /PRIVATE/EVIDENCE/visual.json
```

Only use `--owner-confirmed` after the comparison succeeds. The tool reads
existing captures; it does not take screenshots or inspect their appearances.
An agent must not run this action to manufacture Cameron's confirmation.
If an appearance fails, a receipt with `confirmed: false` records rejection;
the collector preserves a failure even when another receipt for that cell
previously succeeded. Explicit observation IDs must be unique.

The receipt binds `reportSha256`, `fixtureSha256`, all `namedBlocks`, both image
checksums, `confirmedBy: Cameron` and a UTC `observedAt`. Changing the report,
fixture or captures invalidates it. Missing confirmation leaves `pending_visual`.

## Bedrock and native controls

Bedrock and some native navigation/install observations have no Java
map-assets report. They must not invent one. Such an observation supplies
`native: "native.json"` instead of Java report/fixture/rendering fields. This
route is restricted to `bedrock-control` and the listed control IDs, and cannot
fill a Java repair or transport row.

The native owner receipt contains `schemaVersion: 1`, the exact `case`,
`agentHostId`, `confirmedBy: Cameron`, `confirmed` boolean, UTC `observedAt`,
the saved-scene `snapshotId`/`dimension`, and `namedBlocks` with IDs/positions.
It references the actual captured terrain `manifest` and its `manifestSha256`,
plus `captures.map` and `captures.minecraft` filename/SHA-256 references.
These are explicitly owner-observed controls; they have no inferred resource
diagnostic counts. Use a supported remote BDS host for Apple Silicon.
The snapshot correspondence for this native receipt is the owner's saved-scene
record, not a field invented in the existing terrain manifest.

## Resource costs and redaction

Optional measured `metrics.json` binds `operationId` and `reportSha256` to
`firstVisibleMs`, `preparationMs`, peak agent/renderer/helper RSS in bytes,
`cacheBytes`, `downloadedBytes`, `reusedBytes`, `scannedBlocks` and
`scannedChunks`. Only supplied finite nonnegative measurements are emitted;
missing measurements remain pending. The collector does not measure runtime
cost or turn the earlier proof's memory guardrail into a whole-pack budget.
Even complete measurements remain awaiting comparison with the design budgets.

Output allowlists hashes, platform/role labels, original case IDs, generations,
operation IDs, classified counts, bounds, measured numbers and capture hashes.
It omits raw diagnostics, download URLs, filenames/paths, inventory contents,
configs, player identities and screenshot data. Invalid input errors use codes
and input indices without echoing private paths or JSON. Duplicate JSON
keys in a single object, wrong checksums, mismatched bindings and malformed
records are rejected. Unknown extra fields are not exported.

Run collection after adding actual platform receipts:

```sh
python3 tools/world-map-assets/collect-acceptance.py \
  --session /PRIVATE/EVIDENCE/fedora-ubuntu/session.json \
  --session /PRIVATE/EVIDENCE/windows/session.json --format json > /PRIVATE/EVIDENCE/collected.json
```

Exit 0 means the collection inputs were readable/consistent; pending cells can
still be present. Exit 2 reports rejected inputs. Neither is a release gate.
The original Phase 18/Phase 16 acceptance, missing production capture route and
independent phase review remain separate requirements.
