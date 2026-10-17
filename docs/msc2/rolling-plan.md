# MSC 2 — Rolling Plan

**Current priority:** P20.9 large server admission correction implemented; awaiting
owner verification. Phase 20 physical acceptance and independent review remain open.

Historical implementation and published release records were reconciled and
moved to [the archive](rolling-plan-archive.md) at Cameron's October 16 request.
Outstanding physical checks and independent reviews live in
[pending acceptance](pending-acceptance.md). Unimplemented connected-host server
transfers remain in [deferred work](deferred-work.md). Neither link is an active
step queue, and no phase gate was closed by cleanup.

**Recent owner verification:** P16.89 XboxBroadcast works on the affected
Intel Mac with RakNet retained. Publication records live in the archive and
[release notes](release/v0.1.30.md); live publication status belongs on GitHub,
not in this implementation queue.

## Phase 20 — Faster native file imports and uploads

**Execution state:** Owner authorized P20.1–P20.3 on October 16 and P20.4–P20.7 on October 17.
Contract, receipt, native desktop sender, local handoff and server import/export
adapters and import-screen wiring implemented; native candidate and
[acceptance handoff](uploads/acceptance.md) prepared. Owner step verification,
physical measurements and independent Phase 20 review remain pending.
P20.8 is an owner-requested acceptance correction; the P20.7 candidate predates it.
No upload code is included in the XboxBroadcast release.
**Contract and gate:** [Native transfer plan](uploads/native-transfer-plan.md).
Preserve archive/integrity checks and source data; measure speed rather than
promise instant 8 GB imports. UI receives progress, not file bytes. Local
sources use a safe native handoff/copy; remote sources stream directly in Rust.
Complete connected-host transfer/resume remains [deferred](deferred-work.md).

### P20.1 — Freeze the transfer contract and capture the performance baseline

**Status:** Implemented; awaiting owner verification.
**Files:** `docs/msc2/uploads/native-transfer-plan.md`; a compact owner measurement record under `docs/msc2/uploads/`; API capability/endpoint specification in `docs/msc2/api-contract/openapi.json` and the supported-client capability record if needed; this plan.
**What:** Inventory every world/modpack/raw-server/MSC-transfer/add-on input and export path, including CLI and current source-path imports. Specify native operation ownership, stream admission, source handles, exact byte/purpose limits, cancellation/cleanup, memory/concurrency budgets and old-agent fallback before code. Give Cameron concrete before/after measurement commands for his approximately 8 GB world, a representative modpack and server folder, separating transfer from validation/extraction/provider work. Record hardware, disk placement, network and cache state; leave unavailable measurements pending rather than inventing results. Confirm current Rust/Tauri APIs from the locked dependencies. No app implementation in this step.
**Verify:** `git show --check --stat --oneline HEAD`
**Batch:** P20.1 only; owner review/verification before P20.2 unless separately authorized as a batch.
**Commit:** `P20.1: define native upload contract and baseline`

### P20.2 — Receive and verify streamed uploads in the agent

**Status:** Implemented; awaiting owner verification.
**Files:** `crates/msc-agent/src/routes/components.rs`, `worlds.rs`, `worlds/import_activation.rs`, `main.rs` and staging/recovery helpers including `routes/components/native_upload.rs` and Windows private staging ACL support; `crates/msc-api/src/dto/backups.rs`; infrastructure streaming/staging support; OpenAPI/capability data; focused existing upload tests; this plan.
**What:** Add capability-negotiated authenticated stream ingestion with bounded buffers, retained writers and incremental SHA-256. Keep global staging locks out of network/disk I/O, enforce per-purpose exact sizes and pending-file bindings, and publish only complete verified staging. Refresh activity expiry during real work; bound concurrency/disk usage and distinguish active streams from abandoned sessions on disconnect/restart. Cancel cannot redeem a partial file, overwrite an existing world or lose the source. Retain the old chunk endpoint unchanged for compatibility. Compile only essential extended regressions; no test execution by default.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code`
**Batch:** P20.2 only; depends on P20.1 verification unless included in a newly authorized batch.
**Commit:** `P20.2: receive bounded uploads with incremental integrity checks`

### P20.3 — Stream selected files from the native desktop

**Status:** Implemented; awaiting owner verification.
**Files:** Native transfer module and command registration under `clients/desktop-web/src-tauri/src/`; native auth/host transport integration; `clients/desktop-web/src/lib/auth/desktop.ts`, `platform/tauri.ts`, `platform/types.ts`, `api/client.ts`; focused transport tests; generated API types; narrow agent capability/permission integration corrections; `docs/msc2/uploads/native-transfer-plan.md`; this plan.
**What:** Keep selected source bytes in Rust. Start a host-bound native operation using a scoped picker handle, reuse its HTTP client/connection pool, resolve credentials and service readiness once, and stream directly with backpressure and rate/byte progress events. Handle cancellation, revoked credentials, host switching, disconnect and restart without redirecting requests or exposing tokens. New agents use P20.2; older agents use native bounded chunks without JavaScript byte arrays. Reject incompatible hosts before copying. Preserve the generic request bridge for ordinary API calls; do not silently change unrelated authentication behavior.
**Verify:** `cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib -- -D warnings -A clippy::needless_borrow -A clippy::needless_borrows_for_generic_args -A clippy::needless_return`
**Batch:** P20.3 only; depends on P20.2 verification unless included in a newly authorized batch.
**Commit:** `P20.3: stream file uploads through native desktop operations`

### P20.4 — Import same-computer files without an upload round trip

**Status:** Implemented; awaiting owner verification.
**Files:** Native transfer/local-source modules; shared Unix descriptor/identity helpers; existing installing-user socket identity helpers; agent staging/source redemption and production composition; API/capability descriptions and generated types; focused source-handoff cases; `docs/msc2/uploads/native-transfer-plan.md`; this plan.
**What:** Recognize the authenticated local agent and hand off only the selected source through a local-user-bound capability. Copy into private staging directly, optionally using a safe filesystem clone with cross-volume copy fallback; never hard-link mutable input, move/delete the source or accept arbitrary remote-readable paths. Prevent source substitution, expired/replayed handles and host confusion. Preserve exact size/purpose/integrity and importer checks, coalescing reads where possible while reporting required validation work. Compare the direct path with the native streaming fallback; no constant-time promise for 8 GB.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code`
**Batch:** P20.4 only; depends on P20.3 verification unless included in a newly authorized batch.
**Commit:** `P20.4: stage verified local file imports directly`

### P20.5 — Connect server folders and archive imports to native transfer

**Status:** Implemented; awaiting owner verification.
**Files:** `crates/msc-agent/src/routes/servers/import.rs`; existing raw/transfer import services; native file/folder transfer module; `clients/desktop-web/src/lib/sections/fleet/TransferSheet.svelte` and import wizard adapters; CLI source import/upload plumbing; import DTO/OpenAPI types as needed; shared source-tree traversal and bounded extraction; capability matrix and native-transfer implementation record; essential regression cases; this plan.
**What:** Support the inventoried raw server ZIP/folder and MSC-transfer flows without assuming all use staged-world uploads. Local folders use bounded direct copying after existing explicit stopped-source checks. Where supported, remote selected folders generate a validated archive into the native stream without a complete intermediate source ZIP. Retain all supported server files/settings/worlds/backups and source data; reject unsupported/symlink/path-escape entries explicitly. Keep destructive replace-all review/backups and destination registration/recovery semantics. Use native streamed downloads for affected export plumbing that currently buffers whole archives. Reuse importer services; do not implement the deferred paired-agent transfer/resume feature. CLI stays host-local under D-040.
**Verify:** `cargo build --locked -p msc-agent --bin msc`
**Batch:** P20.5 only; depends on P20.4 verification unless included in a newly authorized batch.
**Commit:** `P20.5: route server file imports through native transfer`

### P20.6 — Give every import flow truthful progress and cancellation

**Status:** Implemented; awaiting owner verification.
**Files:** `clients/desktop-web/src/lib/sections/components/StagedUploadSheet.svelte`, `ModpackUploadProgress.svelte`, modpack import entry points; fleet wizard `UploadStep.svelte`, `WorldStep.svelte`, `TransferSheet.svelte`; world import/replace sheets; shared API/platform state; `App.svelte` only as needed; shared source/context/cleanup adapters and bounded host notices; the shared Sheet Escape handler; native creation-context negotiation and narrow DTO/admission updates; essential regression sources; native-transfer implementation record and capability matrix; this plan.
**What:** Apply `antiAIslop.md` within the existing sheets. Connect all inventoried entry points to the native source operation and preserve original-host state when navigating. Show preparing, transferring/local copying, verifying, inspecting and installing/importing as separate real stages. Display byte rate/progress only with a valid denominator; 100% transferred is not import success. Cancellation must await its real cleanup outcome and expose failures. Remove buffer-size tuning from the native path; retain it only for applicable legacy fallback. Surface older-agent capability limits before work and prevent late results from changing another selected host.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P20.6 only; depends on P20.5 verification unless included in a newly authorized batch.
**Commit:** `P20.6: show native import progress across supported flows`

### P20.7 — Prepare owner acceptance and record measured improvements

**Status:** Implemented; awaiting owner verification and physical acceptance.
**Files:** Phase 20 measurement/acceptance record and installation guidance under `docs/msc2/uploads/`; this plan. Generated artifacts remain uncommitted.
**What:** Build a matching native candidate using existing packaging and give Cameron exact artifact/install instructions. Compare the same approximately 8 GB world, representative pack and server sources against P20.1, including direct-copy/network baselines and peak memory. Record local same-volume/cross-volume and available remote/platform observations, plus cancellation, truncated/corrupt input, wrong-host rejection and old-agent fallback. Keep unavailable physical results pending; name remaining bottlenecks without claiming an unmeasured factor. Hand the feature gate to the other agent after owner verification. No automatic test suite, release publication, new CI gate or repeated full release run.
**Verify:** `cargo build --locked -p msc-agent --bin msc`
**Batch:** P20.7 only; depends on P20.6 verification unless included in a newly authorized batch. Physical acceptance is Cameron's separate Verify move.
**Commit:** `P20.7: document native upload performance and acceptance`

### P20.8 — Remove the system alert icon from folder confirmation

**Status:** Implemented; awaiting owner verification.
**Files:** `clients/desktop-web/src/App.svelte`; `clients/desktop-web/src-tauri/src/native_transfer.rs`; this plan.
**What:** Owner-requested acceptance correction: replace the native folder warning alert with the existing icon-free MSC sheet for selected and dropped server folders. Keep explicit stopped-source confirmation, release the selected handle on cancellation, and recheck the original host/server before proceeding. No tests added for this reversible presentation change; no tests executed. The existing P20.7 ZIP predates this correction and must be rebuilt to display it.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P20.8 only.
**Commit:** `P20.8: use an icon-free server folder confirmation`

### P20.9 — Admit complete server sources above the world upload ceiling

**Status:** Implemented; awaiting owner verification.
**Files:** Native desktop source admission; agent native staging; shared source-tree TAR bound; `uploads/native-transfer-plan.md`; this plan. Generated candidate remains uncommitted.
**What:** Owner-reported 20,066,596,805-byte complete server ZIP and large folder failed the individual-world 10 GiB ceiling. Give raw server ZIP/folder and MSC transfers a shared 1 TiB source bound matching existing archive safety, with a 2 TiB aggregate staging reservation ceiling for TAR plus extracted contents. Retain exact-byte, concurrency, disk-margin, extraction and individual-world/modpack limits. Distinguish empty, oversize and actual insufficient disk errors, including sizes. Add one essential admission regression for the reported size, disk refusal and unchanged world ceiling: injected capacity and empty staging only, no large allocation/network/sleeps, expected under one second excluding compilation. Compile regression metadata only; do not execute tests. Prepare a fresh optimized local candidate including P20.8.
**Verify:** `cargo build --locked -p msc-agent --bin msc`
**Batch:** P20.9 only.
**Commit:** `P20.9: admit large server sources with disk space checks`

## Phase 21 — Storage inventory and management

**Planning state:** Owner requested PLAN on October 16 after discussing the
screen and cleanup behavior. P21.1–P21.8 are unstarted; this plan authorizes no
implementation, scanning of live hosts, deletion, release or test execution.
Phase 20 remains the current execution priority. Before implementing storage,
reconcile its staging/operation ownership with the actual completed Phase 20
code; do not build a competing upload/staging registry.

**Owner-agreed experience:** Settings → Storage opens an overview of saved
computers, including This computer. Each row shows MSC usage, available disk
space and last scan time; offline computers retain explicitly dated results.
Selecting a computer opens its inventory, then a category/server breakdown.
Manage rebuildable files and valuable backups/worlds/servers from here, using
existing deletion rules and confirmations. Reviews stay within one category
and one computer: multi-select backups or caches together, while world and
server removal have their own reviews naming the target computer and affected
data. No cross-host or mixed-category deletion cart.

**Screen contract:** Use `antiAIslop.md` and existing Settings/navigation tokens.
A spacious, flat list has names on the left and sizes aligned on the right,
with one restrained proportional storage bar. Categories are Servers, Desktop
app, Agent, Shared tools & runtimes, and Cache & temporary files. Servers are
largest-first and expand to worlds, backups, mods/server files and logs/other
files. Show measured total, available capacity and last scan time prominently;
use a per-volume breakdown when roots span disks, not a misleading combined
free-space number. Specific actions include Review backups, Clear map cache,
Remove old downloads and Open folder when a local folder actually exists.
Removal review shows selected items, size, dependencies/consequences and the
computer; the result reports actual outcomes and any retained/failed items.

**Accounting and protection:** Count owned files once across disjoint roots
and categories. Explain that Desktop app includes its packaged payload while
Agent/shared categories include separately staged installations; do not count
the same subtree in both. Report unreadable/unreachable sources as partial or
unknown, never zero. Separate logical file size from allocated/on-disk usage
where supported; sparse files, hard links and shared filesystem clones require
an explicit accounting basis. Do not promise that summed file sizes equal the
space deletion will release. Shared Java/tools count once at host level; an
externally installed Java/runtime is not MSC-owned. Retain credentials,
current installations, active/rollback-required agent versions, in-use tools,
active transfers/map snapshots, and unrecognized data. Whole-app/agent removal
uses the separate existing complete-uninstall flow, not a cache-cleanup button.

**Scan behavior:** Show cached results promptly and scan on explicit Refresh
or a bounded stale-data policy, not on every Settings render or health poll.
Use bounded background workers, cancellation and incremental progress without
blocking server management. Limit concurrent host scans and do not connect to
all saved hosts automatically just to open Settings. Bind caches/operations to
stable host identity and volume/root identity; host switching/removal or path
changes must not apply old results to another computer. Offline inventory is
viewable but cannot authorize deletion. Scan known owned roots only; do not
crawl arbitrary disks or other users' homes.

**Feature gate:** The agreed overview → computer → category/server → removal
review flow works on supported desktop platforms and paired agents. Known
storage is explained without overlapping counts, unknown coverage is visible,
and multi-volume capacity is honest. Headless agents report their own footprint
without inventing a desktop installation; local desktop footprint remains
available when its agent is offline, with appropriate partial results. Cleanup
revalidates exact selections, permissions, ownership and live references at
execution, cannot target another host or active data, and preserves existing
world/server/backup confirmations. Failed or cancelled work reports partial
outcomes truthfully. Cameron verifies representative local/remote/offline,
large-server, active-operation and destructive cases in disposable data before
the other agent reviews the gate. No new CI/release gate, automatic deletion,
continuous deep scan or broad test matrix is implied.

### P21.1 — Specify the Storage screens and inventory contract

**Status:** Planned.
**Files:** A concise design/accounting document and static screen sketches under `docs/msc2/storage/`; `docs/msc2/msc2-port-plan.md` for the feature gate; this plan.
**What:** Turn the owner-agreed flow above into concrete overview, host breakdown, server detail, category selection and separate cache/backup/world/server removal reviews. Apply `antiAIslop.md`; show loading, partial, offline, permission-denied, empty, changed-since-scan and cleanup-result states. Inventory actual installed bundles, staged agent builds, tools/runtimes, custom server roots, world slots, backups, logs, map output and Phase 20 staging. Specify disjoint categories, volume-aware capacity, logical versus allocated size, hard-link/clone limits, scan budgets, cache lifetime and stable host/item IDs. Reuse read-only ownership discovery from uninstall where appropriate without coupling inventory to its destructive executor. Freeze API/native responsibilities and backward compatibility. Cameron reviews the sketches/contract before UI implementation; no application code in this step.
**Verify:** `git show --check --stat --oneline HEAD`
**Batch:** P21.1 only; follows owner selection of Phase 21, after reconciling Phase 20's actual delivered contract. Later steps need separate execution authorization.
**Commit:** `P21.1: specify storage inventory screens and accounting`

### P21.2 — Measure owned storage with bounded host scans

**Status:** Planned.
**Files:** A storage inventory service/adapters under `crates/msc-application/src/` and `crates/msc-infrastructure/src/`; platform disk-allocation/capacity adapters as needed in `crates/msc-platform-{macos,windows,linux}/`; existing ownership/root discovery and focused inventory tests; this plan.
**What:** Discover only verified MSC roots/installations and measure them without overlapping parent/child categories, symlink escape, arbitrary external runtime inclusion or duplicate hard-link accounting. Resolve actual agent configuration/custom server roots and separately staged/bundled assets. Attribute per-server worlds/backups/components/logs while keeping shared tools host-scoped. Return logical/allocated accounting basis, per-volume capacity, scan time/progress and partial errors; retain unknown data as visible, protected inventory. Use bounded cancellable background traversal with limited concurrency and no heavy process/network calls per file. Cache completed/partial results under host/root identity and avoid filesystem-sized memory. Extend only essential controlled-input coverage for overlapping roots, inaccessible data and link boundaries; document runtime, compile without running tests.
**Verify:** `cargo clippy -p msc-application -p msc-infrastructure --lib -- -D warnings`
**Batch:** P21.2 only; depends on P21.1 review/verification unless a later explicit batch includes both.
**Commit:** `P21.2: inventory owned storage with bounded scans`

### P21.3 — Expose host inventory and local CLI scan operations

**Status:** Planned.
**Files:** `crates/msc-agent/src/routes/storage.rs`, operation/capability wiring and DTOs under `crates/msc-api/src/dto/`; OpenAPI and generated client types; `crates/msc-agent/src/cli/mod.rs` and transport modules as needed; supported-client capability record; focused storage route tests; this plan.
**What:** Add authenticated cached-inventory, scan/progress and cancellation operations with stable host/item IDs, explicit coverage and timestamps. Preserve existing map-preview/cleanup endpoints and permission vocabulary; full-host inventory uses an appropriate existing authorized scope, while scoped server data cannot expose unrelated paths or secrets. Reuse operation journaling/admission and implement headless reconciliation/expiry without repeated deep scans. Add host-local CLI inventory/refresh/cancel and JSON output under D-040; do not introduce direct-remote CLI credentials. Advertise capability/minimum version and make older-agent results unavailable explicitly. Separate inventory display data from execution authority; cached results never authorize deletion by themselves.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code`
**Batch:** P21.3 only; depends on P21.2 verification unless included in a newly authorized batch.
**Commit:** `P21.3: expose storage inventory and scan operations`

### P21.4 — Combine saved-host inventories with the local desktop footprint

**Status:** Planned.
**Files:** A native storage module/commands under `clients/desktop-web/src-tauri/src/`; local installation discovery; `clients/desktop-web/src/lib/platform/`, `auth/desktop.ts`, host-scoped storage state and API adapters as needed; focused host-state tests; this plan.
**What:** Compose the saved-computer overview from cached inventories and explicitly requested remote scans, with bounded fan-out and credentials retained in Rust. Measure the installed local app and local-only client data even when the local agent is absent/offline; merge only matching local-host identities and disjoint roots. An agent may report a verified desktop installation on its host, but inaccessible or undiscoverable remote desktop files remain unknown rather than guessed or assigned the local app's size. Retain dated offline snapshots, invalidate replaced hosts/roots, and guard late responses and scan cancellation across navigation. Folder reveal works only for verified local paths; remote paths are copyable text with their host named. No remote service installation, arbitrary remote path reads or offline cleanup.
**Verify:** `cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib -- -D warnings -A needless_borrow -A needless_borrows_for_generic_args -A needless_return`
**Batch:** P21.4 only; depends on P21.3 verification unless included in a newly authorized batch.
**Commit:** `P21.4: compose host storage and local desktop inventory`

### P21.5 — Add reviewed cleanup for disposable and obsolete owned files

**Status:** Planned.
**Files:** `crates/msc-agent/src/routes/storage.rs`, `map_staging.rs`; storage cleanup/application adapters; Phase 20 staging/operation ownership integration; tool/agent version reference discovery; API/CLI cleanup requests; essential existing cleanup tests; this plan.
**What:** Extend the current preview-bound map cleanup to selected same-category cache/download/obsolete-version entries. Bind an expiring preview to the authenticated host, credential, selected item identities and allowed action; recheck ownership, path identity, changed files, current/rollback versions and live leases immediately before deletion. Active uploads, conversions, maps and shared runtime dependencies remain protected. Resolve interruption/restart and cleanup cancellation at safe boundaries; return removed, retained, failed and remaining items with measured results. Treat reclaimable bytes as an estimate where filesystem sharing prevents exact attribution; refresh affected volume availability without claiming unrelated free-space changes as MSC savings. Reuse existing permission/admission/cancellation mechanisms and prevent cleanup racing with new work. CLI retains local preview and explicit selection/confirmation. Do not repurpose uninstall inventory or general path deletion as authority.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code`
**Batch:** P21.5 only; depends on P21.4 verification unless included in a newly authorized batch.
**Commit:** `P21.5: clean reviewed inactive storage safely`

### P21.6 — Reuse server-data deletion through storage selections

**Status:** Planned.
**Files:** Storage action coordination; existing `crates/msc-agent/src/routes/backups.rs`, world and server deletion routes/services only where needed; relevant DTO/CLI adapters; desktop confirmation/action state; focused action-permission regressions; this plan.
**What:** Map inventory selections to existing backup/world/server IDs and established deletion services, permissions, backups, stopped-state requirements and confirmations. Allow multiple selected backups within one computer/category, with previews and per-item results; keep world removal and whole-server removal in separate reviews. Show a server removal's actual contained worlds/backups and relevant consequences; block active world/server deletion unless the existing supported lifecycle/action permits it. Preview never stops a server, changes selection or deletes data; any required graceful stop is explicit. Revalidate IDs, owner host, current state and preview before every action. Cancellation stops later items but does not pretend earlier deletions were undone; partial failures remain visible. Refuse a stale host/item or changed source rather than falling back to a raw path. Do not relax deletion rules or trigger complete uninstall.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code`
**Batch:** P21.6 only; depends on P21.5 verification unless included in a newly authorized batch.
**Commit:** `P21.6: manage server data through scoped storage reviews`

### P21.7 — Build the Settings Storage overview and management screens

**Status:** Planned.
**Files:** `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`, `StorageCleanup.svelte` or its replacement Storage view; host/category/server detail and removal review components under that section; shared host/storage state and existing backup/world/server confirmation components; focused UI behavior cases; this plan.
**What:** Implement the P21.1 reviewed screens using existing design tokens and `antiAIslop.md`: saved-computer overview, host totals/category rows, largest-first servers and their detail, specific category actions, and separate consequential removal reviews. Keep computer identity visible throughout reviews, neutral flat rows with right-aligned sizes and one restrained proportional bar. Show partial/unknown allocation, multi-volume capacity, dated offline data, scan progress/cancel/refresh and truthful cleanup results. Retain navigation state while host-bound operations run; a closed sheet or changed selected server must not redirect actions/results. Require explicit scans before stale offline items become actionable. Display current app/agent/in-use tools as protected inventory with appropriate existing management links, not deletion buttons. Keep safe map cleanup reachable during migration and provide a clear unsupported-agent state. No decorative status dots, gradients or nested cards.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P21.7 only; depends on P21.6 and Cameron's P21.1 visual review unless included in a newly authorized batch.
**Commit:** `P21.7: add settings storage inventory and management`

### P21.8 — Prepare storage acceptance and record owner observations

**Status:** Planned.
**Files:** Storage owner acceptance/design evidence under `docs/msc2/storage/`; supported-client documentation and CLI help guidance as needed; this plan. Generated candidate artifacts/screenshots remain uncommitted.
**What:** Build a matching candidate through existing packaging and give Cameron exact install/verification instructions. Record overview/detail/review visuals; complete and partial accounting with custom roots, multiple volumes, hard links/sparse/clone limits; local agent offline, remote host offline/reconnection and older-agent behavior; large-world scan responsiveness; active upload/map/runtime protection; and same-category backups versus separate world/server deletion in disposable data. Include stale-preview/path/host changes, permission denial, cancellation and partial failure/restart outcomes. Use small controlled files for essential missing regressions, extending existing cases rather than duplicating deletion tests; document concrete risk and expected runtime, compile only unless a specific test command is authorized. Keep actual observations pending until Cameron reports them, then hand the feature gate to the other agent. No publishing, full uninstall, live valued-data deletion, new CI/release gates or automatic test execution.
**Verify:** `cargo build --locked -p msc-agent --bin msc`
**Batch:** P21.8 only; depends on P21.7 verification unless included in a newly authorized batch. Physical acceptance remains Cameron's separate Verify move.
**Commit:** `P21.8: document storage management acceptance`
