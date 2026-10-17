# MSC 2 — Rolling Plan

**Current priority:** Plan faster native file imports/uploads, then execute one
owner-selected step per conversation. No upload implementation is authorized
by the October 16 PLAN request.

Historical implementation and published release records were reconciled and
moved to [the archive](rolling-plan-archive.md) at Cameron's October 16 request.
Outstanding physical checks and independent reviews live in
[pending acceptance](pending-acceptance.md). Unimplemented connected-host server
transfers remain in [deferred work](deferred-work.md). Neither link is an active
step queue, and no phase gate was closed by cleanup.

**Published baseline:** v0.1.29. Cameron confirmed the P16.89 XboxBroadcast
correction works on the affected Intel Mac with RakNet retained. The next
release will include that correction; upload improvements are planning only.

## Phase 20 — Faster native file imports and uploads

**Planning state:** Owner requested PLAN on October 16. Review the steps before
EXECUTE. No upload code is included in the XboxBroadcast release.
**Contract and gate:** [Native transfer plan](uploads/native-transfer-plan.md).
Preserve archive/integrity checks and source data; measure speed rather than
promise instant 8 GB imports. UI receives progress, not file bytes. Local
sources use a safe native handoff/copy; remote sources stream directly in Rust.
Complete connected-host transfer/resume remains [deferred](deferred-work.md).

### P20.1 — Freeze the transfer contract and capture the performance baseline

**Status:** Planned.
**Files:** `docs/msc2/uploads/native-transfer-plan.md`; a compact owner measurement record under `docs/msc2/uploads/`; API capability/endpoint specification in `docs/msc2/api-contract/openapi.json` and the supported-client capability record if needed; this plan.
**What:** Inventory every world/modpack/raw-server/MSC-transfer/add-on input and export path, including CLI and current source-path imports. Specify native operation ownership, stream admission, source handles, exact byte/purpose limits, cancellation/cleanup, memory/concurrency budgets and old-agent fallback before code. Give Cameron concrete before/after measurement commands for his approximately 8 GB world, a representative modpack and server folder, separating transfer from validation/extraction/provider work. Record hardware, disk placement, network and cache state; leave unavailable measurements pending rather than inventing results. Confirm current Rust/Tauri APIs from the locked dependencies. No app implementation in this step.
**Verify:** `git show --check --stat --oneline HEAD`
**Batch:** P20.1 only; owner review/verification before P20.2 unless separately authorized as a batch.
**Commit:** `P20.1: define native upload contract and baseline`

### P20.2 — Receive and verify streamed uploads in the agent

**Status:** Planned.
**Files:** `crates/msc-agent/src/routes/components.rs`, `worlds.rs`, `worlds/import_activation.rs`, `main.rs` and staging/recovery helpers as needed; `crates/msc-api/src/dto/backups.rs`; infrastructure streaming/staging support; OpenAPI/capability data; focused existing upload tests; this plan.
**What:** Add capability-negotiated authenticated stream ingestion with bounded buffers, retained writers and incremental SHA-256. Keep global staging locks out of network/disk I/O, enforce per-purpose exact sizes and pending-file bindings, and publish only complete verified staging. Refresh activity expiry during real work; bound concurrency/disk usage and distinguish active streams from abandoned sessions on disconnect/restart. Cancel cannot redeem a partial file, overwrite an existing world or lose the source. Retain the old chunk endpoint unchanged for compatibility. Compile only essential extended regressions; no test execution by default.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code`
**Batch:** P20.2 only; depends on P20.1 verification unless included in a newly authorized batch.
**Commit:** `P20.2: receive bounded uploads with incremental integrity checks`

### P20.3 — Stream selected files from the native desktop

**Status:** Planned.
**Files:** Native transfer module and command registration under `clients/desktop-web/src-tauri/src/`; native auth/host transport integration; `clients/desktop-web/src/lib/auth/desktop.ts`, `platform/tauri.ts`, `platform/types.ts`, `api/client.ts`; focused transport tests; this plan.
**What:** Keep selected source bytes in Rust. Start a host-bound native operation using a scoped picker handle, reuse its HTTP client/connection pool, resolve credentials and service readiness once, and stream directly with backpressure and rate/byte progress events. Handle cancellation, revoked credentials, host switching, disconnect and restart without redirecting requests or exposing tokens. New agents use P20.2; older agents use native bounded chunks without JavaScript byte arrays. Reject incompatible hosts before copying. Preserve the generic request bridge for ordinary API calls; do not silently change unrelated authentication behavior.
**Verify:** `cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib -- -D warnings -A needless_borrow -A needless_borrows_for_generic_args -A needless_return`
**Batch:** P20.3 only; depends on P20.2 verification unless included in a newly authorized batch.
**Commit:** `P20.3: stream file uploads through native desktop operations`

### P20.4 — Import same-computer files without an upload round trip

**Status:** Planned.
**Files:** Native file-selection/transfer module; existing local bootstrap/OS-local authorization boundary; agent staging/source redemption; world/modpack import adapters; API/capability data as needed; focused source-handoff tests; this plan.
**What:** Recognize the authenticated local agent and hand off only the selected source through a local-user-bound capability. Copy into private staging directly, optionally using a safe filesystem clone with cross-volume copy fallback; never hard-link mutable input, move/delete the source or accept arbitrary remote-readable paths. Prevent source substitution, expired/replayed handles and host confusion. Preserve exact size/purpose/integrity and importer checks, coalescing reads where possible while reporting required validation work. Compare the direct path with the native streaming fallback; no constant-time promise for 8 GB.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code`
**Batch:** P20.4 only; depends on P20.3 verification unless included in a newly authorized batch.
**Commit:** `P20.4: stage verified local file imports directly`

### P20.5 — Connect server folders and archive imports to native transfer

**Status:** Planned.
**Files:** `crates/msc-agent/src/routes/servers/import.rs`; existing raw/transfer import services; native file/folder transfer module; `clients/desktop-web/src/lib/sections/fleet/TransferSheet.svelte` and import wizard adapters; CLI source import/upload plumbing; import DTO/OpenAPI types as needed; this plan.
**What:** Support the inventoried raw server ZIP/folder and MSC-transfer flows without assuming all use staged-world uploads. Local folders use bounded direct copying after existing explicit stopped-source checks. Where supported, remote selected folders generate a validated archive into the native stream without a complete intermediate source ZIP. Retain all supported server files/settings/worlds/backups and source data; reject unsupported/symlink/path-escape entries explicitly. Keep destructive replace-all review/backups and destination registration/recovery semantics. Use native streamed downloads for affected export plumbing that currently buffers whole archives. Reuse importer services; do not implement the deferred paired-agent transfer/resume feature. CLI stays host-local under D-040.
**Verify:** `cargo build --locked -p msc-agent --bin msc`
**Batch:** P20.5 only; depends on P20.4 verification unless included in a newly authorized batch.
**Commit:** `P20.5: route server file imports through native transfer`

### P20.6 — Give every import flow truthful progress and cancellation

**Status:** Planned.
**Files:** `clients/desktop-web/src/lib/sections/components/StagedUploadSheet.svelte`, `ModpackUploadProgress.svelte`, modpack import entry points; fleet wizard `UploadStep.svelte`, `WorldStep.svelte`, `TransferSheet.svelte`; world import/replace sheets; shared API/platform state; `App.svelte` only as needed; this plan.
**What:** Apply `antiAIslop.md` within the existing sheets. Connect all inventoried entry points to the native source operation and preserve original-host state when navigating. Show preparing, transferring/local copying, verifying, inspecting and installing/importing as separate real stages. Display byte rate/progress only with a valid denominator; 100% transferred is not import success. Cancellation must await its real cleanup outcome and expose failures. Remove buffer-size tuning from the native path; retain it only for applicable legacy fallback. Surface older-agent capability limits before work and prevent late results from changing another selected host.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P20.6 only; depends on P20.5 verification unless included in a newly authorized batch.
**Commit:** `P20.6: show native import progress across supported flows`

### P20.7 — Prepare owner acceptance and record measured improvements

**Status:** Planned.
**Files:** Phase 20 measurement/acceptance record and installation guidance under `docs/msc2/uploads/`; this plan. Generated artifacts remain uncommitted.
**What:** Build a matching native candidate using existing packaging and give Cameron exact artifact/install instructions. Compare the same approximately 8 GB world, representative pack and server sources against P20.1, including direct-copy/network baselines and peak memory. Record local same-volume/cross-volume and available remote/platform observations, plus cancellation, truncated/corrupt input, wrong-host rejection and old-agent fallback. Keep unavailable physical results pending; name remaining bottlenecks without claiming an unmeasured factor. Hand the feature gate to the other agent after owner verification. No automatic test suite, release publication, new CI gate or repeated full release run.
**Verify:** `cargo build --locked -p msc-agent --bin msc`
**Batch:** P20.7 only; depends on P20.6 verification unless included in a newly authorized batch. Physical acceptance is Cameron's separate Verify move.
**Commit:** `P20.7: document native upload performance and acceptance`
