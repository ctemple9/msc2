# Native file imports and uploads — Phase 20 plan

**State:** Planned only, owner requested October 16, 2026. No upload code is
implemented or included in the XboxBroadcast release. Active step fields and
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
