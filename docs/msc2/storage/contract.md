# Storage inventory contract

P21.1 · October 17, 2026 · Implemented specification, awaiting Cameron's review.
This freezes the target for P21.2–P21.7, not shipped API support. No live host
was scanned. [Screen sketches](screens.md) use illustrative data only.
The [Phase 21 gate](../msc2-port-plan.md#phase-21--storage-inventory-and-management)
and owner-approved decisions take precedence. No deletion, installation,
publication or subsequent implementation is authorized by this document.

## Navigation and ownership of actions

Settings → Storage lists saved computers plus This computer, independently of
the selected Minecraft server. Select computer → category or Servers → server
→ category selection → removal review → result. Back preserves that computer,
scroll and selection. Reviews bind the original computer and selections; changing
the application host never redirects them. Closing a view leaves accepted work
visible on its original computer; explicitly cancelling stops at safe boundaries.

Show measured MSC total, accounting basis, coverage, scan date and available
capacity before actions. Sort servers largest first, unknown sizes last. Category
rows have names left and sizes right. One neutral proportional bar represents
measured categories, not disk capacity. Unknown coverage is text, never a zero
segment. Do not combine different volumes' available bytes into one number.
Use existing S0 Settings tokens, system sans, 21px/600 title, 15px/500 sections,
13px/400 rows, quiet 11px metadata, 24–32px group spacing and flat separators.
No nested cards, decorative icons, dots, gradients, rails or animation. Selected
controls and the primary action earn emphasis; valuable-data removal uses the
existing danger treatment. Keyboard focus, labels and meaning survive grayscale.

Reviews contain one computer and one category. Caches and backups support
multi-selection separately. A world review names exactly one world and server;
a server review names exactly one server and its contained data. Existing world,
server and backup services retain their permissions, stopped-state conditions,
safety backups, confirmation wording and retention rules. Preview performs no
stop or mutation. Any required graceful stop is a separate explicit action.
Full uninstall remains its existing separate flow.

## Source inventory and disjoint attribution

Resolve service definitions, MSC2_DATA_DIR, MSC2_APP_CONFIG_PATH and
MSC2_AGENT_SERVERS_ROOT through existing discovery/configuration code; never assume
the desktop user's default is the service account's root. Read only known MSC
locations and explicitly configured server roots. Never search other users'
homes or a whole disk. Read-only ownership evidence may be extracted/reused
from infrastructure/platform/native uninstall discovery. Its broad parent-tree
entries, fingerprint and destructive executor are not inventory categories or
cleanup authority. Missing config makes custom ownership unknown, not disposable.

| Category | Actual source/evidence and treatment |
|---|---|
| Servers | Configured `server_dir` records, including custom/imported roots. Active Java/Bedrock world directories from configuration; `world_slots/<id>/world.zip` and slot metadata from `world_store`; `backups/` archives and companion metadata from `backup_store`; mods/plugins, server JARs, packs, runtime-local files, logs and remaining server-owned files. Partition each server into Worlds, Backups, Mods & server files, Logs & other files. Unknown files remain counted and protected; never infer deletability from an extension. |
| Desktop app | Verified installed bundle/package and its packaged agent/renderer/exporter/sidecar payload, plus client settings, WebView and client data. Platform package receipts/native inspection establish installation; Linux shared package dependencies are not owned. Exclude cache subtrees assigned below. Headless host may have no desktop; failed discovery is unknown, not absent. |
| Agent | Separately staged service binaries/build directories, service config, journals, audit logs and protected credentials. Build identity is content-based, not version label alone. Current and rollback-required builds protected. Exclude server/shared/cache child roots. |
| Shared tools & runtimes | MSC-managed Java, renderer/exporter/helper/runtime installations discovered from their existing install/reference records. Count once per host even when several servers use them. External Java, Tailscale, Docker and OS caches excluded. In-use/current dependencies protected; unknown reference state protected. |
| Cache & temporary files | Map output/snapshots and `map-staging`, component `_addon_cache`, download/export/import staging and verified obsolete downloads. Attribute here even if physically inside a server/data root, subtracting from that parent's category. Active and unknown staging remains visible and protected. User-selected sources and completed exports outside owned roots are excluded. |

Build a root ownership graph before walking. Split parents at verified child
boundaries; aliases to the same filesystem object are references, never extra
bytes. If two servers claim the same directory, show an ownership conflict and
one counted protected source; neither server receives a duplicate total. Do not
follow symlinks, junctions, reparse points or mount boundaries while traversing.
Count link metadata where supported and report excluded targets. A configured
root on another volume is separately validated and scanned as its own root.
Unrecognized contents inside verified owned roots stay visible under that
root's residual category; an unverified root gets a coverage warning, not a crawl.

### Phase 20 reconciliation against delivered code

`worlds.rs::staging_root` resolves `<servers_root>/.msc2-staging`.
`routes/components.rs` has the existing 30-minute staging lifetime, uploads,
downloads and modpack staging. `components/native_upload.rs` owns `UploadStore`,
native sessions, reservations, `native/<id>/owner.json`, `payload.bin`, extracted
folder inputs, credential/purpose binding, inspection/consumer ownership and its
reaper. Restart recovery retains cleanup ownership; it does not restore ready
receipts. `ConsumedInput` releases importer ownership after the consumer ends.
The ledger's age alone cannot prove inactivity. Read these owners' snapshots;
any cleanup extension must coordinate with their admission/reaper locks and
revalidate references. Do not write another staging ledger or unlink their files
from a generic storage walker. Legacy slots also require their owner's decision.

`src-tauri/src/native_transfer.rs` and its download/local-source modules own
native source handles, original-host operations and destination partial files.
A selected original is never MSC cleanup material. Active export production,
readers and pending final rename remain protected. Unknown cleanup stays unknown.
`map_staging.rs` separately owns map leases/reservations/admission locking and
checks renderer processes: an unlocked lease can still belong to an orphaned
renderer. Preserve that protection. `routes/storage.rs` currently provides only
Admin map preview/cleanup, with credential-bound 300-second single-use previews
and a 64-preview cap. Keep these endpoints and semantics compatible; broader
selection cleanup uses new versioned request shapes, not changed old defaults.
Phase 20 acceptance and its measurement/review gaps remain in
[its record](../uploads/acceptance.md); this inventory does not close them.

## Measurement and identity

Return integer bytes, format binary units (GiB) consistently. Logical bytes are
file lengths; allocated bytes are filesystem-reported allocation where available.
Choose one host-wide displayed basis: logical by default; allocated only when
coverage supports it. Never sum logical and allocated values. Each measure has
its own coverage and nullable value; permission-denied and unreachable are not
zero. A verified absent source is zero with absent status. Partial totals say
“at least”; retain per-source issues and last successful observation separately.

Deduplicate hard links by volume/file identity across all roots; charge once to
a deterministic category/root/item order and list other references. If identity
tracking reaches its memory budget, stop affected accounting as partial rather
than double-count. Sparse allocation may be less than logical size. Filesystem
clones/compression/snapshots can share blocks beyond exposed identities; allocated
sizes do not establish uniquely reclaimable space. Never label a file-size sum
“space freed.” Report removed measured bytes, retained/failed items and fresh
volume availability separately; other processes also change available space.

Capacity is sampled per stable volume identity with total/available bytes and
time, separately from scan time. Same-volume roots share one capacity row.
Unmounted/replaced disks show unavailable; volume names/drive letters alone are
not identities. Never request administrator escalation merely to fill coverage.

Snapshot schema target (additive DTOs, camelCase):

| Record | Required meaning |
|---|---|
| Snapshot | schemaVersion=1, snapshotId, authenticated hostId, rootGeneration, startedAt/completedAt, state (complete/partial/cancelled), basis, logicalBytes/allocatedBytes, coverage, volumes, categories, issues |
| Source/item | opaque itemId, rootId, volumeId, parent/category, existing server/slot/backup ID where applicable, label, measured bytes/coverage, observedAt, protection reasons and allowed review actions; paths only for authorized display |
| Scan | operationId, host/rootGeneration, state, roots/entries/bytes visited, current source, issues; no percent without a known denominator |
| Review | host, credential, category/action, exact selected IDs and source fingerprints, dependencies/consequences, expiring previewToken; estimates labelled as estimates |
| Result | operationId, original host and selection, per-item removed/retained/failed/not-attempted outcomes with reasons, measured removed bytes/basis, current coverage and volume samples |

Host identity is authenticated stable agent identity mapped to the saved-host
record, not its name, URL or list index. This computer has a native local identity;
merge agent data only after verified local identity match. Replacement/re-pairing
invalidates association even at the same URL. rootId includes resolved path,
volume and filesystem object identity plus ownership/config generation. itemId
maps an existing domain ID or an opaque durable registry identity to that root;
never use a display path as an execution ID. Fingerprints detect file replacement
and size/mtime/identity changes without hashing every large file. Missing reliable
identity makes a destructive action unavailable. Inventory IDs grant no authority.

## Bounded scans and cache policy

Decided for implementation: opening Storage reads cached metadata only. Refresh
is explicit; there is no background stale-data fan-out in v1. Mark snapshots
older than 15 minutes stale. Retain the latest dated snapshot per saved host
until removal/replacement; it can remain viewable offline indefinitely. Cache
keys include host identity, schema and root generation. A root/config change
invalidates affected measures and previews; old data remains labelled historical.
Host removal deletes its cached association and cannot attach results to another
record. Late responses must match the original generation before publication.

At most one scan per host and two across desktop requests; agent admission also
caps one host scan independently of callers. Use one bounded background traversal
worker per scan, at most 64 open handles, depth 128, 1,000,000 entries, five
minutes wall time and a 32 MiB working-memory budget. Exceeding a bound returns
partial with an explicit reason; a new Refresh starts a new scan, not an endless
automatic retry. Walk streaming directories; use bounded identity tables and
paged item results (200/page), never keep the full file listing. Retain aggregated
categories plus domain items, with at most 10,000 persisted items/8 MiB per host;
report truncation and permit narrower explicit category/server refresh within
these same limits. Stream progress no more than twice per second; check cancel
between directory/entry batches (at most 256). Blocked OS filesystem calls can
delay cancellation; never claim immediate cancellation. No deep scan on health
polls or Settings renders, file hashing, process spawning or network calls per
file. Reuse owner-provided activity snapshots; scope any process inspection to
existing cleanup preview, not ordinary measurement. Concurrent writes make this
an observation over time, not a transactional filesystem snapshot.

## API, native and compatibility boundary

Agent: inventory its own authorized owned roots, volumes, cache, scan operations,
progress/cancel, preview and execution. Full-host inventory/refresh/cleanup require
existing Admin permission, including CLI; narrower server data cannot reveal
other servers. Reuse operation admission/journaling/cancellation. Redact secrets,
credential contents and private ledger fields; paths are not accepted as targets.
Headless recovery marks interrupted scans partial and reconciles cleanup through
existing owners, never deleting just because a scan was interrupted.

New routes, all under `/v1/host/storage`, use additive protocol-1 DTOs:

| Method/path | Contract |
|---|---|
| GET `/inventory` | Latest cached snapshot or explicit not-scanned state; no traversal. Item paging binds snapshotId and opaque cursor; expired/mismatched cursor refuses. |
| POST `/scans` | Explicit refresh, optional known root/category/server filter, returns 202 operationId; concurrent scan returns existing-operation conflict, never queues unlimited work. |
| GET `/scans/{operationId}` | Authorized host-bound progress/result reference; reuse existing operation cancellation endpoint and permissions. |
| POST `/reviews` | Exact item IDs, snapshotId, category and action; returns refreshed consequences and expiring preview or conflict/denial. |
| POST `/cleanups` | Preview token plus existing action-specific confirmation fields; accepts only that preview's selection, returns 202 operationId. Result reachable through existing operation status plus a paged storage result reference. |

Use existing structured API errors: unauthorized/forbidden, stale or changed
selection conflict, scan admission conflict, unavailable source and expired
preview remain distinguishable. No raw path parameters. Existing world/server/
backup services remain the execution authority behind review coordination;
protocol confirmation fields must preserve their existing action-specific shapes.
Native commands mirror cached inventory, refresh/status/cancel and verified local
folder reveal; remote transport still uses the authenticated agent routes.

Add capability `storageInventory` with protocol 1 and explicit scan/cleanup action
flags, generated DTO/OpenAPI/client definitions in P21.3. Missing/unknown protocol
means “Storage inventory unavailable on this agent”; do not guess from version
0.1.30 or invent a minimum shipped version before publication. Existing map
cleanup remains separately reachable. New desktop with old agent retains local
desktop measurement and clearly missing agent coverage; old clients keep current
map endpoints unchanged. Do not widen the supported-version floor.

Native desktop: discover local installation/client-only roots even with agent
offline, resolve saved identities and credentials in Rust, compose disjoint local
and agent sources, cache snapshots and guard request generations. Agent may
measure a verified desktop on its host; inaccessible remote user data stays
unknown. Renderer receives metadata only. Folder reveal accepts a verified local
item identity and re-resolves it; remote paths are copyable text naming the
computer. No arbitrary native path-read/reveal command or remote installation.

CLI: host-local inventory, refresh, status/cancel and JSON via existing local
authorization (D-040). Cleanup uses explicit selections, preview and existing
confirmations. No direct-remote credential mode. UI only navigates, displays,
selects and confirms; it never authorizes deletion using cached sizes.

Cleanup previews expire after 300 seconds and bind authenticated host, credential,
action, category and exact selection. Execution rechecks permission, identity,
source changes, current references, leases and lifecycle admission atomically
with the owning service. Changed selection/source requires a new review; no raw
path fallback. Offline/stale snapshots cannot authorize cleanup. Acquire new
owner-approved preview before mutation even with fresh inventory. Unknown,
active, rollback-required and current installations remain protected. Cancellation
stops later items; removed items stay removed. Failed cleanup or restart must
preserve per-item uncertainty and never manufacture a success result.

## Review handoff

Cameron reviews these sketches and accounting choices before UI implementation.
No application code, tests or runtime checks are part of P21.1. Documentation
verification is `git show --check --stat --oneline HEAD`. Physical accounting,
platform behavior and destructive acceptance remain pending P21.8; a static
sketch does not verify them. No CI/release gate is added.
