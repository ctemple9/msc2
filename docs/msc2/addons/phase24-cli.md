# Phase 24 CLI content commands

These commands use this host's authenticated agent and explicit server IDs.
They do not select a different active server. Run `msc content --help` for
flags and `msc content request --help` for the closed list of supported actions.
Content results are JSON in either display mode; long operations wait and return
the complete operation/result record. `--no-wait` returns its operation ID.
The existing operation command can poll/cancel it; Ctrl-C while waiting requests
cancellation and waits for acknowledgment.

## Discovery and acquisition

```sh
msc content search "example" --provider modrinth --kind modpack
msc content search "example" --provider curseforge --kind modpack
msc content search "example" --provider curseforge --kind mod --game-version 1.21.1 --loader fabric
msc content inspect PROJECT_ID --provider curseforge --offset 0
msc content acquire PROJECT_ID RELEASE_ID --provider modrinth --confirm
msc content stage-pack ./downloaded-pack.mrpack
```

Supported discovery categories: mod, plugin, modpack, resourcepack, shader,
datapack. Pack search deliberately includes incompatible packs. A pack's
selected release is inspected before an update/replacement review chooses its
setup. CurseForge release enumeration is paginated with `--offset`; Modrinth's
project-version endpoint returns its version list. Acquisition returns an
operation whose result contains staged archive metadata; it does not install.
Local pack staging emits `stagedUploadId` for the same later review.
CurseForge key configuration remains in the existing secret-storage flow.
Commands never take API keys in arguments or print them.

## Collection, source and reviews

```sh
msc content get --server SERVER_ID --kind collection
msc content get --server SERVER_ID --kind identify
msc content get --server SERVER_ID --kind updates
msc content request --server SERVER_ID --action install-review --file install.json --confirm
msc content request --server SERVER_ID --action install --file apply.json --confirm
msc content get --server SERVER_ID --kind review --id REVIEW_ID
```

For a CurseForge install review, `install.json` contains the selected numeric
provider project/release identities:

```json
{"provider":"curseforge","projectId":123,"versionId":456}
```

Inspect the returned files, required dependencies, warnings and replacements.
The install/apply JSON then names that exact review:

```json
{"reviewId":"REVIEW_ID"}
```

Existing Modrinth individual installation remains available through
`msc addon install-catalog`; both providers have explicit discovery here.
Per-file request bodies use the shared API contract in
[openapi.json](../api-contract/openapi.json). Request files are bounded to 8 MiB;
`--file -` reads an object from stdin. The API may apply narrower request limits.
No conflict decisions are invented from filenames or silently accepted.

| Action | Behavior |
|---|---|
| source / update-review | Exact-hash provider source assignment / reviewed individual update |
| review / apply | Independent player content, audience, optional/enabled edits and restores |
| recover | Existing-server recovery from retained archive or newly staged original |
| pack-update-review / pack-update | Compare old/current/new and apply explicit per-path choices |
| pack-replacement-review / pack-replacement | Replace the one base pack with explicit compatibility/retention decisions |
| install-manual-admit / install-manual-complete | Restricted individual-file recovery |
| player-admit | Purpose-bound independent player content upload |
| pack-manual-admit / pack-manual-complete | Pending pack review file recovery; requires --id REVIEW_ID |
| retained-retry / retained-manual-admit / retained-manual-complete | Retained pack recovery; requires --id PACK_ID |
| publication-review / publish | Review selected content and freeze immutable dated publication |

For admission actions, add `--upload ./local-file` to stream that file into the
returned purpose-bound slot. The JSON must include the exact requested size,
hash and pinned identity required by that admission. Successful upload emits its
receipt and staged ID; a separate completion/review action is still required.
Failed uploads request slot cleanup and report inability to confirm it.
All mutation commands require `--confirm`; this flag does not replace the
agent's exact hashes, review IDs, owner identity or conflict choices.

Read `pack-review`, `retained-pack`, `publication-review` and `publication`
with `get --id`. Read `datapacks --id WORLD_SLOT_ID` to choose explicit server
world content without exporting world saves.

## Publication and host export

```sh
msc content request --server SERVER_ID --action publication-review --file publication.json --confirm
msc content get --server SERVER_ID --kind publication-review --id REVIEW_ID
msc content request --server SERVER_ID --action publish --file publish.json --confirm
msc content get --server SERVER_ID --kind publications
msc content get --server SERVER_ID --kind mrpack-eligibility --id PUBLICATION_ID --audience players
msc content get --server SERVER_ID --kind export-destination --path friends-pack.zip
msc content export --server SERVER_ID --publication PUBLICATION_ID --audience players --destination friends-pack.zip --include-mrpack --confirm
```

A publication review JSON starts with `collectionRevision`, selected
`audiences`, any explicit `playerSetup`, and per-file redistribution/config
approvals. It can include selected server datapacks and owner exclusions.
Publish JSON contains `reviewId` and explicit `incompleteConsent` when needed.
Publishing/exporting never bypasses known author restrictions.

Export saves inside this server's `exports/` folder on this agent host;
existing subfolders can be chosen. The returned operation reports its actual
host path, selected audience, publication and completeness. The host destination
requires Admin plus Add-ons permission. An existing filename is refused unless
`--replace-sha512` matches the exact existing identity from destination review.
Use `--allow-incomplete` only after reviewing missing-content reasons.
`--include-mrpack` requires eligible format/setup; a plugin-server bundle stays
an ordinary ZIP. Player optional shaders/resource packs remain inactive.
These terminal exports do not use the desktop's local save picker.
