# Reference corpus

Real-world artifacts used to validate the port against actual MSC 1 usage, not
synthetic data — per `msc2-port-plan.md` §1 ("Reference corpus"). This is a
different thing from `fixtures/`: fixtures are small, pinned input/expected
pairs extracted from MSC 1's own test assertions; this corpus is raw material
for broader, end-to-end checks that a handful of unit fixtures can't cover.

None of these categories should ever be filled with invented data standing in
for the real thing. An empty or thin category here means "not yet supplied,"
not "doesn't matter" — MSC 1's own repository ships none of this, since it's
all either personal-install artifacts or large binary files that were never
meant to be checked into source control.

## configs/

**Populated by P5.3.** One real, sanitized `server_config_swift.json` from
Cameron's live MSC 1 install, plus a real MSC 1-generated `.msctransfer`
package (referenced via `$MSC2_PHASE5_TRANSFER_PACKAGE`, not committed here —
see `configs/README.md`). A second config from a distinct schema era was
sought but confirmed unavailable anywhere on Cameron's machines; `configs/README.md`
records that gap and how P5.4/P5.5's fixtures cover the era-diversity behavior
it would otherwise have exercised. Still useful if one ever surfaces: any real
`.corrupt-<timestamp>` sibling file produced by the R3 recovery path.

## logs/

**Empty — needs real files from Cameron.** Complete real Minecraft server
console/crash logs — Paper, Forge, NeoForge, Fabric, vanilla — covering both
healthy startup and the failure shapes the P0.8/P0.9 crash-analyzer fixtures
pin (missing dependency, incompatible version, Connector/Fabric entrypoint
failure). Those fixtures use short log-line excerpts quoted directly in
MSC 1's own Swift tests, which is genuine evidence but narrow; full real logs
from real servers would catch shapes a few pinned lines can't.

## packs/

**Empty — needs real files from Cameron.** Real `.mrpack` and CurseForge-format
modpack archives — at minimum something in BMC4's shape (the pack referenced
throughout the P0.16/P0.18 fixtures) and a real Modrinth pack — to validate
extraction and parsing end-to-end, beyond the unit-level manifest fixtures
already extracted. These are large binary files; Cameron would supply them
from his own collection or a public download.

## server-dirs/

**Empty — needs real directories from Cameron.** Complete real Minecraft
server directory trees (`server.properties`, world data, `plugins`/`mods`,
etc.) for at least one server per flavor MSC 2 must support. Used for
end-to-end validation beyond the pure-function fixtures in `fixtures/`.
Worth a decision at review: these are large, so this category may end up
git-ignored with the harness pointed at a local path instead of committed
directly, rather than living in the repo the way `dto-examples/` does.

## dto-examples/

**Seeded now**, unlike the other four categories, with wire-format JSON that
is genuinely embedded as raw string literals in MSC 1's own test source —
copied verbatim, not reconstructed from Swift struct field values:

| File | Source |
|---|---|
| `simple-result-example.json` | `DTOContractTests.swift:366` |
| `simple-result-nil-server-id.json` | `DTOContractTests.swift:374` |
| `command-result-example.json` | `DTOContractTests.swift:381` |
| `users-create-request-example.json` | `RemoteAPIIntegrationTests.swift:184` (also used at :227) |
| `command-request-example.json` | `RemoteAPIIntegrationTests.swift:220` |

These three DTOs (`SimpleResult`, `CommandResult`, and the `/users`,
`/command` request bodies) are the ones MSC 1's tests happen to write as raw
JSON text — mostly because `SimpleResult`/`CommandResult`'s macOS-side types
are private local structs the tests can't construct directly (see
`fixtures/dto-contract/simple-result-round-trip.json`'s notes), so the test
hand-writes the wire format instead of building then encoding a Swift value.
Every other DTO in `fixtures/dto-contract/` and `fixtures/pack-managed-guard/`
is equally genuine evidence of the wire format, just captured as a Swift
struct literal (what MSC 1's test actually constructs) rather than literal
JSON text in the source file — those aren't duplicated here.
