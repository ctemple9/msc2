# Native file imports and uploads — Phase 20 plan

**State:** P20.4 Unix local descriptor handoff implemented October 17, 2026; owner
verification pending. Windows uses native streaming; screen integration remains
planned. These changes
are not included in the published XboxBroadcast release. Active step fields and
execution boundaries live in [rolling-plan.md](../rolling-plan.md).

## Outcome

World ZIPs, modpack archives, supported raw server ZIPs/folders and MSC transfer
files should move at a rate governed primarily by disk/network throughput.
Selecting a source must respond promptly; copying, hashing, archive inspection,
provider downloads and installation remain real work with separate progress.
An 8 GB import is not promised to be instant. Keep bounded memory and existing
validation, authorization, source preservation and recovery requirements.

## Source-backed findings

- P14.44 replaced whole-file desktop modpack buffering with 2 MiB chunks to
  bound memory. P14.55 added selectable 1–8 MiB chunks and world support. The
  records do not establish a throughput benchmark for those values.
- `src/lib/api/client.ts::stagedUploadFromFile` reads a chunk, awaits one PUT
  acknowledgement, then reads the next. No read/send overlap is present.
- `src/lib/platform/tauri.ts` reads files natively but returns chunks to the
  webview. `src/lib/auth/desktop.ts::authorizedRequest` spreads the request
  body into a JavaScript number array for the Tauri command request. Binary
  response framing does not make those request bodies binary.
- `src-tauri/src/lib.rs::desktop_authorized_request` loads the credential,
  ensures the local service when applicable, and constructs a new reqwest
  client for every request. Upload data takes this same generic route.
- `routes/components.rs::upload_staged_chunk` holds the shared staging map
  lock while doing file I/O, reopens the staging file for each chunk, and
  rereads the entire completed file to hash it. Several routes accept a
  fully buffered `Bytes` body rather than consuming a stream.
- World ZIP normalization validates and rewrites archive packaging. The
  current normalization uses raw compressed-entry copying where possible;
  do not describe it as necessarily decompressing/recompressing every entry.
- Raw server/MSC-transfer import has its own agent-local path contract. The
  Transfer sheet's export downloads the complete archive into memory. Those
  flows need an explicit inventory; they are not all the staged upload API.

These establish avoidable work, not its measured share of the reported delay.
The baseline must distinguish source reading, native/webview conversion, wire
transfer, final flush/hash, archive inspection, extraction/copy and provider
installation. CPU, peak resident memory and source/destination disk placement
belong with elapsed time and throughput. Cache state and network route must be
recorded so before/after comparisons are meaningful.

## Shared native transport

The interface owns selection, confirmation and progress, not file bytes. A
native-selected, scoped source handle and authenticated host identity start one
native operation. Rust opens the source and streams directly to the destination
using a reused HTTP client/connection pool. Credential resolution and local
service preflight occur once at operation start; a credential/host change,
revocation, agent restart or cancellation must invalidate/terminate the correct
operation. Never log credentials or accept an arbitrary URL/header combination.

The new agent endpoint consumes a bounded stream with backpressure (reading
only as fast as the destination can accept data), writes a private staging file
through a retained writer and hashes bytes incrementally. It must not put a
whole request/file in memory or hold the global staging map across file I/O.
Authenticate before accepting data, check exact byte ceilings incrementally,
and atomically publish a complete staged object only after successful receipt,
size/digest validation and flush. Length, purpose, credential owner and optional
pending-operation/file binding remain mandatory. Concurrent transfer limits,
idle timeouts and temporary storage must be bounded and observable.

Compatibility is negotiated before sending the source. Existing agents retain
the current bounded chunk protocol as an explicit fallback; their request
ceiling is not silently raised. The native fallback still avoids webview byte
arrays and can reuse one HTTP client. New endpoint support gets a documented
capability and minimum agent version without breaking existing API consumers.

Initial remote streaming does not promise restart/resume. A dropped/cancelled
stream cannot be redeemed and must clean up its owned partial file or report
cleanup failure. Retry starts a new operation. Genuine persisted resume, if
later requested, needs authenticated offsets and matching source identity; it
must not be implied by the old chunk-size dropdown.

## Same-computer imports

Use a verified local-agent identity and OS-local authorization boundary for
source handoff, not a remote HTTP parameter that grants arbitrary file access.
A native picker must yield a scoped handle/token bound to the initiating local
user, source, purpose and selected host. Reject symlink/path substitution,
unsupported entry types, expired/replayed handles and host switching. Do not
infer locality solely from a host display name or a client-supplied boolean.

For an existing file, prefer a native copy into private staging; allow a safe
copy-on-write clone when available, with ordinary bounded copy fallback on
another filesystem. Never hard-link mutable user input, move/delete the source,
change its permissions, or let later source edits alter committed imports.
A clone still requires integrity/format checks. Coalesce hashing/validation
passes where the format allows it; retain necessary reads honestly. No complete
source archive is required merely to import an existing server folder locally.

Folder imports need a bounded manifest and controlled copying of all supported
files, including hidden server-owned data. Preserve current importer semantics,
unknown files/configuration and destination identity. Refuse running-server
sources unless the existing explicit graceful-stop flow has finished. Remote
folder import may generate a supported archive directly into the native stream,
without writing a complete intermediate source ZIP; destination staging remains
necessary. Preserve traversal, symlink, decompression and total-size defenses.
Never omit unsupported files silently. Do not adopt an arbitrary directory in
place or reuse this path to read an unselected folder on a remote host.

## Existing operations and progress

Reuse world/modpack/server import services after safe source staging. Archive
safety, format/edition detection, modpack manifest/dependency downloads, world
activation backups, destination naming and registration must retain behavior.
Do not bypass validation or call an import complete when upload reaches 100%.

Show only real stages: preparing/reading, transferring or local copying,
verifying, inspecting, importing/installing, complete. Byte progress and rate
apply to measured transfer work; use truthful activity for stages without a
known denominator. Cancel must stop further reads/network/file work, cooperate
with the importer at safe boundaries and retain the prior server/world. Keep
operation state scoped to the original host through view changes; late results
cannot populate a different host. A closed view does not orphan the operation.

Apply `antiAIslop.md` using the existing progress sheet and controls. The
performance buffer is an internal implementation setting on the native path,
not a choice users must tune to obtain reasonable speed. Keep legacy-only
chunk selection visible only where it still changes fallback behavior.

## Acceptance and essential checks

Cameron runs the physical comparisons using the same large world (about 8 GB),
a representative modpack and a supported server-folder/transfer import, with
local same-volume, cross-volume and remote cases where available. Compare
before/after against a native disk copy or equivalent network transfer baseline.
Record peak memory, stage times, MiB/s, cancellation/cleanup and resulting data.
A cold provider dependency download is not upload throughput. Do not promise
an improvement factor without measurements or repeatedly run full releases.

The gate requires source bytes absent from webview IPC on the native path;
reused transfer connections; bounded memory independent of total file size;
streamed receipt/hash with no compulsory final whole-file hash reread; correct
local fallback/copy semantics; authentic progress; and no loss of integrity,
source preservation, permissions, cancellation or old-agent compatibility.
Necessary archive validation/rewrite reads are reported separately.

Add only essential missing regressions: corrupted/truncated streams cannot be
redeemed, cancellation cannot publish or overwrite an active world, wrong
host/credential/source handles are refused, and host changes cannot redirect a
live transfer. Prefer extending existing focused tests with injected streams,
controlled paths and tiny files. Document the concrete missing risk and runtime;
expected focused cases under one second each, with no live network, VM or timing
assumptions. No suite runs without Cameron requesting its specific command.
No new CI/release gates, benchmark dashboards or broad test matrix.

## Relationship to deferred server transfers

[P18.41–P18.45](../deferred-work.md) describe complete transfers between two
paired agents, including source locking, persisted resume and registration.
That feature stays deferred. Phase 20 covers user-selected local input sent to
one destination, plus supported file import/export plumbing; it may provide a
reusable native stream but does not claim the complete host-to-host feature.

## P20.1 implementation contract

This section freezes the implementation target, not a claim of shipped support.
Existing endpoints and approved decisions retain their meaning. P20.2–P20.6
implement this contract; changes to these bounds must be recorded explicitly.
Performance observations live in [baseline.md](baseline.md).

### Input/output inventory

Paths below are relative to `clients/desktop-web/src` or the agent route tree.
The shared `App.svelte` upload adapter reaches `api/client.ts`; changing only
one sheet would leave other entry points on the expensive path.

| Input/output | Current route and consumer | Phase 20 destination |
|---|---|---|
| World ZIP, new-server wizard and Worlds import | staged `world-import`, `/worlds/import`; wizard UploadStep/WorldStep and world sheets | native selection, local copy or remote stream, same importer |
| Active-world ZIP replacement | staged `active-world-replace`, `/worlds/replace-active` | same transport; retain stopped-target checks and safety backup |
| World thumbnail | staged `world-thumbnail`, `/worlds/{slot_id}/thumbnail` | native file stream; retain image decoding/resize validation |
| Modpack ZIP/mrpack, wizard and Add-ons import/replace | staged `modpack-archive`, `/modpacks/inspect`, `/modpacks/import` | inspection and subsequent redemption share verified staging; provider work separate |
| Author-blocked CurseForge file and unresolved modpack file | staged `curseforge-manual-file` / `modpack-unresolved-file`, pending-file completion routes | native file stream bound to pending operation/file and provider identity |
| Local mod/plugin, Java datapack, Bedrock pack/add-on | staged `addon-local-file`, component install and world datapack/Bedrock-pack routes | same purpose bound to actual consumer; retain edition/archive checks |
| Catalog modpack/add-on/datapack/provider downloads | provider URLs fetched by agent | retain agent download; no picker upload or proxy through desktop |
| Raw server folder/ZIP | `/servers/import` scan/importExisting with agent-local sourcePath; fleet wizard | preserve existing path API; new desktop-selected source uses verified local handoff or staged remote archive |
| MSC transfer, including MSC 1 export | `/servers/import` importTransfer, `.msctransfer`; TransferSheet | dedicated staged source adapter, preserve merge/replaceAll review and registration |
| Recovery rescan | `/servers/import` rescan | unchanged; no desktop source upload |
| World export | `/worlds/export` then `/staged-downloads/{id}` | native download to a selected destination; private partial then rename |
| Server/MSC-transfer export | `/servers/export` then staged download; TransferSheet currently buffers response | native streamed download; preserve complete server/settings/world/backup set |
| Client add-on export | `/components/client-export`, staged ZIP or metadata result | stream ZIP when returned; metadata stays ordinary JSON |
| CLI file/folder import and export | `cli/mod.rs`: stage_file_upload, world archive builder, raw sourcePath imports, get_raw_bytes exports | bounded native reads/writes; preserve D-040 host-local connection and existing commands |

CLI currently buffers several files/exports and builds world folder archives;
its native implementation is not already a bounded stream. Folder generation
must enumerate hidden supported files and fail explicitly on unsupported types.
Existing agent-local sourcePath imports remain authenticated administrative
operations; the new handoff must not expand that remote path-reading surface.

### Admission, wire shape and compatibility

`GET /v1/capabilities` gains optional `nativeTransfers` only when implemented:
`{protocol: 1, streamUpload: true, localHandoff: false, serverArchive: false}`.
The last two become true only when their full paths exist. Omission means old
agent; false means unavailable. Capability is filtered for the calling token
and supported client minor. Minimum support is protocol 1 with advertised
streamUpload, not an invented future release number; record the first shipped
agent version in P20.7. Never infer support merely from a version string.

Add `POST /v1/staged-uploads/stream` for admission. JSON fields are `purpose`,
`fileName`, `expectedBytes` (required positive integer), optional
`expectedSha256` (64 hexadecimal characters), `operationId`, `fileId`, and
`targetServerId` where the consumer acts on an existing server. Return 201 with
existing slot identity/expiry/ceiling fields plus `streamPath` and protocol 1.
Then `PUT /v1/staged-uploads/{id}/stream`, application/octet-stream,
Content-Length exactly expectedBytes, identity encoding only. Reject body
before reading if auth, length, purpose, owner, target, expiry or capacity fails.
Return the existing completion shape only after exact receipt, incremental
SHA-256, optional digest match and successful file sync. Counting received
bytes remains mandatory even with Content-Length. No compression, multipart,
caller-selected filesystem path, URL or redirect is accepted.

Admission binds credential identity, destination host instance, purpose, target
and pending file identity. World purposes require Worlds; modpack/add-on purposes require Addons, except
shared addon-local-file staging also serves Worlds-authorized world-pack consumers.
The final consumer still requires its own permission; server archive purposes require Fleet. Recheck authorization and
bindings at redemption, including revoked credentials and changed pending files.
One active writer and one redemption per slot; a conflicting stream PUT returns
409. Existing importers keep their 404 response for unavailable input, including
an incomplete native slot that has never been published. Existing legacy slot
POST/PUT/chunks wire behavior remains unchanged for old clients.

Use the existing error envelope: 400 invalid length/digest metadata; 401/403
auth/permission; 404 unknown slot; 409 wrong binding/state, short input or digest
mismatch; 413 ceiling/overlong body; 429 capacity; 507 insufficient staging space;
500 write/sync/cleanup failure. Network disconnect may prevent delivery of an
error response; status and cleanup outcome remain inspectable after reconnect.
`GET /v1/staged-uploads/{id}/status` returns owner-only state, receivedBytes,
expectedBytes, expiry and cleanup error, never a path or credential.

Native fallback uses sequential existing chunks only for world-import,
active-world-replace and modpack-archive. Honor returned maxChunkBytes; a missing
field means 2 MiB only for legacy modpacks. Reject legacy world admission without
that field before reading bytes, preserving the existing world-support check.
The native sender caps its own chunk at 1 MiB, never guessing 8 MiB. Other purposes use the existing whole-body
PUT via a bounded native request stream within the advertised ceiling, explicitly
reporting that the older agent buffers receipt. Do not pretend old-server memory
is bounded. Remote folders/server archives require serverArchive capability;
otherwise offer the existing agent-local path workflow before reading/copying.

### Exact limits and resource ownership

| Purpose | Input ceiling in bytes | Required identity |
|---|---:|---|
| world-import, active-world-replace, modpack-archive | 10,737,418,240 each | exact selected file length |
| world-thumbnail | 10,737,418,240 (existing ceiling retained) | exact file length; image checks still required |
| addon-local-file | 536,870,912 | exact file length; consumer validation |
| curseforge-manual-file, modpack-unresolved-file | pending expected_byte_size | expectedBytes must equal pending size; provider hash when available |
| raw-server-archive, msc-transfer-archive (P20.5 additive purposes) | 10,737,418,240 | exact file length; Fleet permission |

Keep existing archive defenses: default 200,000 entries and 1,099,511,627,776
uncompressed bytes in infrastructure/archive.rs; stricter importer rules win.
These are format limits, not disk reservations. Folder manifests have the same
entry/total ceilings and stream enumeration; no all-entry list in memory.
A generated remote ZIP has no known compressed length in advance: prepare it
in private native staging to obtain exact length in the initial implementation.
Direct ZIP generation into the network is allowed only after a separately
specified bounded unknown-length admission protocol; do not fake Content-Length.
This resolves the plan's “where supported” condition without weakening admission.

Agent: at most two active receivers/copies, one per credential, eight admitted
unredeemed slots, and 21,474,836,480 bytes of aggregate reserved staged input per
host. Reserve expectedBytes at admission; release only after cleanup/redemption.
Reject when available disk cannot cover reservation plus a 1 GiB free-space
margin. Extraction/copy estimates must be checked separately before mutation;
input reservation does not promise room for expanded content. Existing downloads
and legacy slots count toward actual occupied staging space. No unbounded queue.

Use 256 KiB read/write blocks with at most four queued blocks per transfer;
application transfer buffers at most 2 MiB per endpoint per active transfer,
excluding bounded HTTP/TLS overhead and existing importer memory. Desktop runs
one bulk operation per host and at most two globally. Coalesce progress to at
most four events/second, always send terminal state. These budgets are targets
for measurement, not an assertion about present RSS.

State: admitted → receiving/copying → verifying → ready → redeemed; failures
enter cancelling/cleanup-failed/cancelled or failed. Active I/O refreshes a
30-minute activity lease; abort after 120 seconds without byte progress.
Ready slots expire after 30 minutes without consumer activity. Inspection keeps
a leased slot alive; failed inspection cannot mark it redeemable. Never hold
the shared map lock during disk/network I/O. Incremental hashing avoids a final
mandatory hash reread; format checks may legitimately reread a ready file.

DELETE cancels its owned slot idempotently, interrupts writer, awaits writer
close and removal, and reports success only after cleanup. Expose cleanup
failure and retain its reservation for recovery. Import cancellation uses the
existing operation cancellation boundaries and safety backup; once committed,
report completion rather than claiming rollback. On disconnect, fail the stream
and clean the partial; no offset resume. On restart, no partial is redeemable;
reconcile private staging before advertising capacity. Persist a bounded cleanup
ledger for failed removals and owner-visible terminal status; never adopt an
unrecognized staging file as complete. New retry means new slot and source token.

### Native source and local handoff

Rust owns source selection/opening, credential snapshot, pooled client, active
operation and cancellation. Webview messages carry opaque source/operation IDs,
metadata and progress only. A picker token is random, single-use, expires after
five minutes, and binds local user, host instance, purpose and source identity.
Reject symlinks/reparse-point substitutions and nonregular input. Hold the open
file through transfer; check identity/size/modification before and after reading,
failing on observed change. Folder traversal uses controlled handles and rejects
links, devices, path escapes and running sources. Do not read MSC 1 state beyond
the explicitly selected source or modify its source tree.

Locality requires the existing OS-local bootstrap boundary, peer-user check and
agent instance identity, never only localhost/DNS. Redeem handoff on that local
boundary, not public HTTP. Bind and verify source handles before copying. When
safe descriptor/handle transfer is unavailable, fall back to native loopback
streaming; never trust a bare path from the webview. Destination independently
opens its private staging file. Same-volume safe clone is optional, ordinary
bounded copy mandatory; no hard links or source mutation. Validate the snapshot
before ready. Downloads write a private sibling partial and atomically commit
only after size/digest validation; cancellation preserves an existing destination.

### Locked API feasibility (inspected locally, October 16)

Root lock: axum 0.7.9, tokio 1.53.1. Desktop lock: tauri 2.11.5,
tauri-plugin-dialog 2.7.2, reqwest 0.12.28 (direct dependency), tokio 1.53.1,
tokio-util 0.7.19. A separate transitive reqwest 0.13.4 is not the shell API.
Registry source confirms axum Body::into_data_stream, reqwest Body::wrap_stream
and Response::bytes_stream, dialog blocking_pick_file and Tauri Emitter::emit.
Reqwest streaming is feature-gated: the current desktop manifest enables only
json/rustls-tls and must explicitly enable stream. Tokio-util ReaderStream also
requires its io feature and a direct dependency; do not rely on transitive
features. Blocking picker/copy work belongs off the event loop. P20.3 must check
Send/lifetime constraints through its authorized compiler check. No dependency
or implementation change is made in P20.1.

## P20.2 implementation record

The agent now implements protocol 1 admission, stream receipt and owner-only
status, plus native dispatch through the existing cancellation route. Capability
advertisement is optional and permission-dependent; localHandoff/serverArchive
remain false. No desktop, CLI, local-source handoff or server-archive sending is
implemented in this step. No performance or physical acceptance result is claimed.

Native files live in a separate UUID directory under `.msc2-staging/native`.
Admission writes a small owner/size cleanup ledger and creates the payload before
returning a token. Startup removes abandoned native files rather than treating
an interrupted stream as ready. Failed deletion retains its reservation and
owner-visible cleanup status; the janitor retries. Unrecognized recovery entries
block native admission. An interrupted admission without a complete ledger can
be discarded inside this isolated directory; it never identifies a source file.

The stream uses one open Tokio file, 256 KiB writes, a bounded 1 MiB transport
frame and incremental SHA-256. Tokio's file buffer is explicitly limited to
256 KiB, keeping application buffers below the 2 MiB contract. Empty frames do
not reset the 120-second no-byte-progress deadline. File sync precedes publication.
No new receiver network/disk work holds the shared staging map. Legacy chunk
implementation/wire behavior remains intact, including its existing hash reread.
Disk admission includes native reservations and existing legacy/download storage;
free-space checks reserve outstanding unwritten bytes plus the 1 GiB margin,
rather than subtracting already allocated data a second time. Legacy routes do
not gain a new protocol or release gate from this work.

The shared upload access boundary checks the authenticated request's credential,
purpose permission, original target and original servers-root binding for native
files. A native slot cannot be overwritten by legacy PUT/chunks. Consumption
atomically claims a ready native file once. A consumer guard follows the source
through background server/world work; when that work ends, including an early
return or panic, it signals the janitor to remove leftover staging and release
its reservation. This prevents failed importer admission from leaking native
capacity forever. Guards perform no disk work while map locks may be held.

Active receipt watches credential revocation and permission reduction. Manual
files are checked against their exact pending size at admission and against the
original provider file identity/hash/destination again at receipt and redemption.
The redemption check uses the already locked pending-file record, avoiding a
second lock on that record. Modpack inspection keeps an input lease alive during
real work; failed inspection cancels native staging. Existing importer format,
archive, safety-backup and activation checks still run after verified receipt.

Required target details for the next desktop step: world-import, replacement,
thumbnail and local add-on admission require targetServerId. Existing-server
modpack import also requires a matching target at redemption; new-server modpack
creation accepts only an upload without an existing target. A wizard must create
its destination before staging a world, or wait to start that source operation
until the new server ID exists. This avoids silently rebinding an already selected
existing server. The original basename is bounded and rejects path/control syntax and Windows
device names; native local JAR installation preserves that safe name. Legacy naming
behavior is unchanged. A changed servers root can initialize a new private staging
root only after existing native reservations have been cleared.

Unix directories/files use 0700/0600. Windows creates directories with a protected
ACL (the Windows list of allowed accounts) granting inheritable access only to the
service account, rather than inheriting a broad custom volume's permissions.
The same installing-user service identity remains in force; no elevation is added.

### Essential regression justification

Four focused cases extend the agent upload coverage. Existing chunk tests do not
exercise streamed body failure, native ownership, cancellation or restart ledgers.

| Case | User-visible risk protected | Controlled inputs / expected runtime |
|---|---|---|
| native_stream_publishes_exact_bytes_only_to_its_owner_once | Another token redeems/cancels input, or legacy PUT replaces a verified upload | Six bytes in two frames, fake credentials, isolated directory; under one second |
| native_stream_rejects_short_overlong_and_corrupt_bodies_without_publication | Partial/corrupted bytes become importer-visible or a selected filename escapes the destination | Three tiny deterministic bodies, a wrong digest, and rejected path/device basenames; under one second |
| native_stream_cancellation_and_revocation_close_and_remove_partial_staging | Cancel/revocation leaves a redeemable or orphaned active upload | A deliberately pending body and watch notifications, no sleeps/socket; under one second |
| native_stream_restart_discards_abandoned_slots_without_redeeming_them | Restart adopts an incomplete prior file as valid | Two-byte abandoned file and new store; under one second |

Disk-space measurement is injected in these fixtures, so production's 1 GiB
margin does not make tests depend on the test machine's available capacity.
Temporary directories are isolated and cleaned. Runtime estimates exclude Rust
compilation. No test was executed.

### Checks and remaining owner verification

- `cargo fmt --all` and the P20.2 agent Clippy check passed during implementation;
  final checks are recorded with the step handoff.
- Unit-test harness compilation uses
  `cargo rustc -p msc-agent --bin msc --profile test -- --emit=metadata`;
  it compiles these cases without executing them.
- Broad `cargo check -p msc-agent --bin msc --tests` encountered pre-existing
  integration harness errors: desktop_auth/cli_pairing include auth.rs without
  its existing crate::routes dependency; cli_service initializes removed
  CommonArgs fields (base_url/host/port/token). Those files were not changed.
- Full Windows package checking from this Mac stops in ring's C build because
  the Windows C headers/toolchain are unavailable (`assert.h` missing). The new
  Windows ACL module passed isolated Windows-target Rust metadata compilation
  against the locked windows-sys API. Full Windows build/runtime permission
  acceptance remains Cameron's check; no platform result is invented.

Owner physical checks, focused test execution if requested, throughput/RSS and
end-to-end desktop acceptance remain pending. No CI/release workflow was changed,
no release was run, and no phase gate or step verification was closed here.

## P20.3 implementation record

The desktop exposes a separate native picker and upload operation. Existing
screens keep their uploader until P20.6; same-computer sources still use HTTP
until P20.4. No paths, file bytes, bearer tokens or arbitrary destination URLs
cross the new JavaScript API. The caller supplies host, purpose and optional
target/pending-file identities:

```ts
const source = await platform.pickNativeFile?.({
  label: 'Choose world ZIP',
  extensions: ['zip'],
  binding: { agentHostId: client.host, purpose: 'world-import', targetServerId: server.id },
});
if (source) await client.stagedUploadNative(source, { signal, onProgress });
```

Rust retains a read-only file descriptor behind a single-use opaque handle.
The descriptor and selected path must still identify the original regular file,
length and modification time before and after sending. Final symlinks, Windows
reparse points and devices are rejected. The source is never moved, deleted,
rewritten or given different permissions. At most sixteen unused handles expire
after five minutes, with a thirty-second native reaper. Identity checks detect
replacement and ordinary edits; they are not an atomic filesystem snapshot
against deliberate timestamp restoration. The sender hashes the actual bytes
and compares the agent's exact length and SHA-256 before returning a staging ID.

Each operation resolves its credential and local service readiness once, then
retains that origin/token and a process-wide pooled HTTP client with redirects
disabled. Capabilities and admission precede source reads. Protocol 1 sends one
exact-length HTTP body using 256 KiB reads and HTTP backpressure. Older agents
use sequential native chunks for world/modpack archives, capped at 1 MiB and
the advertised limit. Other existing purposes use a bounded native request stream
but report `legacy-buffered-agent`: those agents still buffer receipt. Unsupported
native protocols and explicit `streamUpload: false` reject without fallback.
Missing world chunk support also rejects before reading. Ordinary API calls
retain their generic request bridge.

There are at most two active operations globally and one per initiating host.
Metadata responses are capped at 256 KiB. Upload duration has no overall request
timeout; connection, control requests, source reads and cleanup have separate
bounds. Long active uploads are not limited by a short control timeout.
Progress carries identity, phase, counts, average rate and transport mode.
Ordinary events are limited to four per second; phase/terminal events are
immediate. Stream counts mean bytes supplied to HTTP, not remote durable
acknowledgements. Legacy chunk counts follow acknowledgements. Completion means
verified staging, not finished archive validation/extraction or installation.
The webview filters progress using both initiating host and operation identity.
Changing the visible host cannot redirect an operation; screen callers retain
an AbortSignal to cancel their own work when closing.

The source handle also names the operation, allowing cancellation before a start
reply arrives or recovery when that reply is lost. Cancellation atomically
retires an unused selection or signals its worker and waits for cleanup. A
verified completed result wins late cancellation. Credential replacement or
forgetting, authoritative 401 responses, SSH route start/retry/stop and main-window
reload invalidate affected work. Cleanup uses the original origin/token, never
the newly selected host. Failures distinguish no admission, confirmed cleanup
and unconfirmed cleanup, carrying original host/operation/staging IDs when known.
Interrupted admission with a lost slot ID cannot claim cleanup confirmation.
Process restart does not resume operations; agent staging recovery handles
abandoned receipts.

Two integration corrections preserve existing behavior. A new agent with
unavailable staging now advertises explicit `streamUpload: false`, preventing
accidental old-agent fallback. Shared `addon-local-file` admission accepts Addons
or Worlds permission because it also serves world packs; existing consumer routes
still require the respective installation permission. Admin continues to satisfy
admission and subsequent binding checks. Generated TypeScript API types now
reflect the P20.2 OpenAPI additions.

### Essential checks and remaining acceptance

Three Rust cases protect single-use/host/concurrency boundaries, source
substitution/edits and cancellation before startup/host invalidation. Three
transport cases protect metadata-only multi-gigabyte selection, foreign progress
and delayed cleanup, and lost-start-reply recovery with late completion. Existing
upload coverage did not protect this new native IPC lifecycle. Inputs are tiny
owned temporary files or controlled promises, without live servers, sleeps or
large allocations. Expected runtime is below one second per case excluding
compilation; this is an estimate. No tests were executed.

Implementation checks passed: desktop and agent Clippy, Rust formatting,
Svelte/TypeScript checking (zero errors; eleven existing warnings in untouched
screens), and desktop Rust test-harness compilation with `cargo rustc --lib
--profile test -- --emit=metadata`. Windows source-opening/identity helpers were
separately compiled with Clippy against locked windows-sys metadata for
`x86_64-pc-windows-msvc`. This is not a complete Windows app build or OS-dialog
runtime check. Full Windows linking remains subject to the unavailable MSVC C
headers recorded in P20.2. Physical cancellation, service restart, throughput
and memory measurements remain owner acceptance work; compilation does not
establish a speed improvement or close the phase gate.

The P20.3 Verify command now qualifies its existing three lint allowances with
`clippy::`; unqualified names are deprecated and fail under `-D warnings`.
Allowances and scope are otherwise unchanged. No release workflow changed and
no beta was published.

## P20.4 implementation record

macOS and Linux can now copy a selected regular file into agent-owned private
staging without sending the file bytes over loopback HTTP. Admission and cleanup
remain small authenticated HTTP requests. The source travels as an OS file
descriptor over `local-source-v1.sock` in the existing private service-user data
directory. No path is sent to the agent, no HTTP path-reading route was added,
and the webview still sees only its single-use selection and progress.

The desktop attempts this only for its saved bootstrap-local host using the
canonical local origin, protocol 1 and advertised localHandoff. The socket and
its containing directory must be owned by the installing user with private
permissions; both ends check kernel-provided peer UID using the existing local
CLI identity helpers. The agent independently checks the saved host ID and
current scoped credential. Before accepting a descriptor it must hold the exact
random admission ID in this process, owned by that credential, with matching
length, unexpired lease, current target/pending-file bindings and unstarted state.
This ties handoff to the admitted agent instance rather than a localhost label.
Another process, a remote forward or a restarted agent cannot redeem an old
in-memory slot. Passing a descriptor twice cannot start a second writer.

The descriptor must be read-only and identify the selected regular file by
volume/device, inode, exact size, modification time and change time. The sender
retains its original handle and rechecks its selected path before/after copying;
the receiver checks the descriptor before reading and before end-of-input.
Unix picker identities now also retain change time, catching ordinary edits even
when modification time is restored. Unsupported or malformed ancillary messages,
extra descriptors and writable/device handles fail closed; owned received handles
are closed on error. Descriptors are marked close-on-exec. No capability accepts
a source pathname, follows a substituted pathname, or changes source contents,
ownership or permissions.

The agent reads bounded 256 KiB blocks directly from that descriptor and feeds
its existing native staging writer. The writer counts exact bytes, incrementally
hashes the same copied blocks, syncs private staging, and publishes only after
final source/binding/authorization checks. It retains P20.2 concurrency/disk
reservations, cancellation, revocation, cleanup and restart recovery. The trusted
local agent produces the digest in this one copy pass; the desktop validates the
receipt identity, length and digest format without rereading the source to hash
it again. Remote streaming keeps its independent sender/receiver digest check.

`local-copy` transport and `local-copying` progress report actual agent staging
bytes, at most four times per second, followed by verifying/complete. Socket
messages are bounded to 16 KiB, with ten-second handshake/write bounds. At most
sixteen socket exchanges can be active; the existing smaller staging/transfer
budgets still govern writers. Disconnect drops the receiver request guard and
signals its detached cleanup worker; desktop cancellation also waits for the
existing original-host DELETE result. A failure never triggers a second copy
after a descriptor has been passed. A missing or unsafe local socket falls back
to the existing native stream before any descriptor/source bytes are sent.

**Decided for you:** use ordinary bounded copying on every source/staging volume,
without a clone optimization. This works across volumes without hard links,
extra source copies or filesystem-specific clone assumptions. Clone support is
optional in the contract. Windows advertises localHandoff false and retains
P20.3 native loopback streaming: no safe Windows handle-transfer path is claimed
by this Unix implementation. The contract explicitly permits that fallback.
Existing world/modpack consumers receive the same verified staging IDs through
the P20.3 API; screen integration remains P20.6. Folder/server-archive selection
is P20.5 and was not added here.

Two essential cases extend the existing native-upload fixture: a real socket-pair
handoff verifies source/digest/staged bytes and refuses wrong-host/replayed
admission; a descriptor case proves pathname replacement cannot redirect an open
source, detects edits, and rejects writable handles. Existing receiver cases
already cover exact-length failure, cancellation/revocation and abandoned staging;
those were not duplicated. Files are six bytes or smaller in the fixture's owned
temporary directory, socket peers are local, and no live agent, arbitrary sleeps,
external files or large allocation is needed. Expected runtime is below one
second per case excluding compilation (unmeasured). Tests were compiled, not run.

Agent and desktop Clippy, Rust formatting, generated API types and TypeScript
checking passed on the development Mac. TypeScript retains eleven existing
warnings in untouched screens. Agent and desktop test harnesses were compiled
with `cargo rustc ... --profile test -- --emit=metadata`; the agent harness reports
one existing unused-item warning. No test executable was launched. Linux runtime,
Windows fallback, OS-dialog behavior, cancellation under physical disk load,
cross-volume copying, throughput and peak memory remain owner acceptance work.
No clone/speedup measurement, release publication, CI gate or phase completion is
claimed.
