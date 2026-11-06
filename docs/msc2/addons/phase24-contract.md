# Phase 24 integration contract

P24.1 inspection, 2026-11-05. This contract is the implementation target;
new routes/capabilities are not advertised until their handlers exist.

## Provider integration inventory

| Existing behavior | Backend anchors | Required counterpart |
|---|---|---|
| Catalog search/detail/versions | infrastructure addon_provider; agent components routes | Provider-qualified Modrinth/CurseForge Java projects and pinned releases |
| Catalog installs and dependencies | application addons, addon_dependencies | Compatible file resolution, reviewed required dependencies, restricted-file checkpoints |
| Inventory, manual links, updates | add_on_inventory, addon_updates | Exact hashes/fingerprints, qualified identities, ambiguous matches remain unresolved |
| Health diagnostics | diagnostics; routes/health | Preserve offline health behavior; explicit provider-backed resolution only in authenticated actions |
| Pack inspection/import | modpacks; components routes | Local or provider-staged archive, retain full client content before filtering |
| Client export | client_export; components/client-export | Keep old response contract; new dated publication API alongside it |
| World datapacks | worlds routes; WorldPackBrowserSheet, WorldStep | Provider choice with world-slot ownership; preserve Bedrock paths |
| Creation | servers routes; wizard AddOnsStep/UploadStep/model | Shared pack acquisition; matching release defaults and both mismatch choices |
| Desktop details and updates | PluginBrowserSheet, ProjectDetailSheet, VersionPickerSheet, AddonsSection | Preserve provider identity through each action |
| Terminal commands | cli/mod.rs and tui/components/create/worlds | Same API and review semantics, host-local destinations |
| Educational copy | help/SetupIntro.svelte | Remove incorrect Modrinth-only claims after counterpart is implemented |

Literal source inventory (generated from rg over production application, agent
and desktop sources; generated.ts is derived, not independently edited):

```
crates/msc-application/src/diagnostics.rs
crates/msc-application/src/modpacks.rs
crates/msc-application/src/addon_updates.rs
crates/msc-agent/src/cli/mod.rs
crates/msc-application/src/addon_dependencies.rs
crates/msc-application/src/addons.rs
clients/desktop-web/src/lib/help/SetupIntro.svelte
crates/msc-application/src/client_export.rs
crates/msc-application/src/add_on_inventory.rs
crates/msc-agent/src/cli/tui/create.rs
clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte
crates/msc-agent/src/routes/components.rs
clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte
crates/msc-agent/src/routes/worlds.rs
crates/msc-agent/src/routes/health.rs
clients/desktop-web/src/lib/sections/fleet/wizard/WorldStep.svelte
crates/msc-agent/src/routes/servers.rs
clients/desktop-web/src/lib/sections/components/PluginBrowserSheet.svelte
clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte
clients/desktop-web/src/lib/sections/fleet/wizard/UploadStep.svelte
clients/desktop-web/src/lib/sections/components/model.ts
clients/desktop-web/src/lib/sections/components/ImportModpackSheet.svelte
clients/desktop-web/src/lib/sections/fleet/wizard/ModpackCreationSummarySheet.svelte
clients/desktop-web/src/lib/sections/fleet/wizard/model.ts
clients/desktop-web/src/lib/sections/fleet/wizard/AddOnsStep.svelte
clients/desktop-web/src/lib/api/generated.ts
```

## Wire contract and compatibility

Existing catalog calls without a provider continue to mean Modrinth. Existing
unqualified project IDs and addon_links retain their old interpretation; never
reinterpret numeric IDs as CurseForge merely by shape. New identities carry
provider, project ID and exact version/file ID. Keep same-title projects distinct.
Do not rewrite legacy client-export response fields or repurpose its GET as
publication. Existing callers remain functional during capability rollout.

New collection and review bodies carry explicit serverId, collectionRevision,
and reviewed hashes. Reject active-server changes rather than accidentally
following global selection. Requests bind to the authenticated host; native
transfers additionally retain paired-host identity until completion.

Proposed additive route families under /v1/servers/{serverId}/content:

- GET collection: upstream identity, revision, items, source/classification
  evidence, required/optional, audience and unresolved reasons.
- POST review: requested content/audience changes, update, replacement or
  recovery; returns review ID, source revision, per-path effects and conflicts.
- POST apply: review ID plus explicit resolutions; 202 operationId on accepted
  long work, never success before filesystem effects are committed.
- GET publications and POST publications: historical metadata and explicit
  immutable snapshot creation from a reviewed revision.
- POST exports: publication ID, audience, selection, includeMrpack, incomplete
  consent and destination; operation produces a staged download or authorized
  host file, never a base64 JSON archive.

Provider acquisition uses catalog provider/project/release identities, not
caller-supplied arbitrary remote URLs. Resolve/download to purpose-bound staging
and inspect before applying. Use existing operations/events/cancellation and
D-027 manual-file upload identity validation. Required dependency chains are
resolved per audience with cycles and incompatible selections surfaced.

Read metadata requires authenticated access; mutation, publication and export
require addons permission. Host filesystem destination selection additionally
uses existing authorized filesystem roots/capabilities; addons alone grants no
arbitrary-path write. Guide data never exposes agent credentials. Permission
revocation cancels access to staged downloads and prevents queued mutation.

Response semantics: invalid input 400; permission failure 403; missing resource
404; changed collection/review/destination or conflicting operation 409;
unavailable capability follows existing capability_unavailable semantics.
Provider missing key, restricted download, network failure and rate limit remain
separate actionable reasons, not empty successful search results. Revalidate
permissions, source bytes, server state and review resolutions at apply time.

Capabilities distinguish catalog provider, retained player collection,
review/apply, publication and export readiness. Capability names and concrete
DTOs are added to OpenAPI/generated types in each implementation slice, keeping
this specification out of the advertised supported API until implemented.

## Snapshot and archive invariants

A publication pins content bytes, relative paths, hashes, exact game/loader,
audience/optional evidence and guide asset version. Source/provider reachability
cannot change an existing snapshot. Unresolved required content blocks complete
publication. Incomplete consent and missing-file reasons persist in metadata,
manifest and README; unknown local-file provenance is reviewed without inventing
redistribution permission. A manual download is not permission to redistribute.

ZIP exports contain actual eligible content and offline guides. The optional
nested .mrpack describes the same audience/revision using exact dependencies,
SHA-1/SHA-512, sizes, env flags and valid overrides. Authenticated CurseForge CDN
URLs are not public launcher downloads; never embed API keys. Permitted bytes
may be included as overrides; unsupported/restricted entries require resolution
or explicit incomplete labeling. Plugin-only server exports do not pretend to
be launcher-importable. Unknown loader/version metadata blocks a complete
.mrpack rather than inventing values.

Safe relative paths only, reject traversal/symlinks and case-colliding archive
entries. Server exports allow reviewed content paths, not arbitrary recursive
server/config/world copying. Datapacks identify their selected world slot.
Space checks and bounded streaming precede atomic publication/export commit.
Rollback, references and unfinished staging remain protected from cleanup.

## UI review inventory

Owner final-review amendment (2026-11-05) supersedes the intermediate stops
described below: all candidates proceed to final UI review. See the execution
ledger and final acceptance packet; no unreviewed surface is marked approved.


Runnable review stops: creation pack browser/compatibility mode; player content
management; original-pack recovery review; upstream conflict review; base-pack
replacement review; publish/export sheet; corresponding new TUI dialogs/views.
Reused provider/detail/world screens stop only if a substantially new view is
introduced. Review surfaces individually within multi-view steps before the
single final step commit. Guides stop for owner screenshots and rendered review.

## Verification boundaries

Source inspection establishes the inventory, not provider behavior. Current
provider/launcher evidence sources and limitations are in phase24-scope.md.
Live credentialed access, actual launcher installs, platform recovery and memory
behavior remain owner acceptance items; no network success is claimed here.

## Owner clarification — individual install interaction

Ordinary CurseForge mod/plugin Add and Install actions match Modrinth: resolve
and install without a separate routine confirmation sheet. The agent still
prepares and validates its pinned dependency review internally before applying
files. Clicking Install anyway supplies explicit setup-mismatch consent. Only
required manual-download file handling interrupts the normal flow; conflicts
remain actionable failures. This owner direction does not remove content
publication/export eligibility, pack conflict or other specified review flows.
