# MSC 2 — Rolling Plan

**Current priority:** P20.12 corrects imported Java JAR presence/version reporting;
awaiting owner verification. Earlier Phase 20 steps remain accepted and archived.
Phase 21 P21.1–P21.7 are implemented, awaiting owner verification; P21.8
is authorized for sequential execution with verification deferred until the end.
P21.1 visual acceptance remains pending.
Independent Phase 20 gate review remains in [pending acceptance](pending-acceptance.md).

Historical implementation and published release records were reconciled and
moved to [the archive](rolling-plan-archive.md) at Cameron's October 16 request.
Outstanding physical checks and independent reviews live in
[pending acceptance](pending-acceptance.md). Unimplemented connected-host server
transfers remain in [deferred work](deferred-work.md). Neither link is an active
step queue, and no phase gate was closed by cleanup.

**Recent owner acceptance:** Phase 20 P20.1–P20.10 closed and archived by owner
direction after local native-import acceptance; see [acceptance](uploads/acceptance.md).
P16.89 XboxBroadcast works on the affected
Intel Mac with RakNet retained. Publication records live in the archive and
[release notes](release/v0.1.30.md); live publication status belongs on GitHub,
not in this implementation queue.

## Phase 20 — Imported server component follow-up

P20.1–P20.10 remain archived and owner-accepted. This later reported correction
is separate from that closure; no earlier unobserved check is promoted to Pass.

### P20.12 — Detect installed imported Java server JAR versions

**Status:** Implemented; awaiting owner verification.
**Files:** `crates/msc-agent/src/routes/components.rs`; `crates/msc-application/src/import.rs`; `crates/msc-infrastructure/src/java_jar_metadata.rs` and module registration; candidate evidence under `uploads/`; this plan.
**What:** Check configured JAR presence independently of version fields. Read small known JAR metadata entries offline, recovering the reported Paperclip Minecraft version from root `version.json`, plus explicit server-manifest version/build when available. Persist recovered metadata on future raw imports; existing imports detect it when Components loads without re-import or data mutation. Existing unreadable/unknown metadata shows installed/version unknown; an absent JAR stays missing even with stale config metadata. Do not invent an online update verdict, run Java or download/replace the JAR. Inspect off the async API worker. Add three essential regression cases covering the reported Paperclip layout, dependency-version confusion/oversize metadata, and real component presence versus stale version fields. Tiny memory ZIPs/isolated files, no timing/network dependencies, expected under one second each excluding compilation; compile only, do not run tests. Prepare a matching optimized Intel candidate.
**Verify:** `cargo build --locked -p msc-agent --bin msc`
**Batch:** P20.12 only.
**Commit:** `P20.12: detect versions of imported server jars`

## Phase 21 — Storage inventory and management

**Planning state:** Owner requested PLAN on October 16 after discussing the
screen and cleanup behavior. P21.1–P21.7 are implemented and awaiting owner
verification/review. Cameron explicitly authorized P21.3–P21.8 to execute in
sequence with one commit per step and verification deferred until the end.
This also authorizes the P21.1-based UI implementation; visual acceptance remains
pending and is not inferred from execution authorization. No live-host
scanning, deletion, release or test execution was performed.
Phase 20 implementation records are archived. Before implementing storage,
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

**Status:** Implemented; awaiting owner verification and sketch/contract review.
**Files:** `docs/msc2/storage/contract.md` and `docs/msc2/storage/screens.md`; `docs/msc2/msc2-port-plan.md` for the feature gate; this plan.
**What:** Turn the owner-agreed flow above into concrete overview, host breakdown, server detail, category selection and separate cache/backup/world/server removal reviews. Apply `antiAIslop.md`; show loading, partial, offline, permission-denied, empty, changed-since-scan and cleanup-result states. Inventory actual installed bundles, staged agent builds, tools/runtimes, custom server roots, world slots, backups, logs, map output and Phase 20 staging. Specify disjoint categories, volume-aware capacity, logical versus allocated size, hard-link/clone limits, scan budgets, cache lifetime and stable host/item IDs. Reuse read-only ownership discovery from uninstall where appropriate without coupling inventory to its destructive executor. Freeze API/native responsibilities and backward compatibility. Cameron reviews the sketches/contract before UI implementation; no application code in this step.
**Implementation:** Added the concrete screen wireframes, source-backed Phase 20 staging reconciliation, disjoint accounting/identity rules, bounded scan/cache policy and additive API/native contract. Documentation only; no application code or tests. Owner review remains pending before subsequent dependent implementation.
**Verify:** `git show --check --stat --oneline HEAD`
**Batch:** P21.1 only; follows owner selection of Phase 21, after reconciling Phase 20's actual delivered contract. Later steps need separate execution authorization.
**Commit:** `P21.1: specify storage inventory screens and accounting`

### P21.2 — Measure owned storage with bounded host scans

**Status:** Implemented; awaiting owner verification.
**Files:** `crates/msc-application/src/storage.rs`; `crates/msc-infrastructure/src/storage.rs`, `storage/native.rs`, `source_tree.rs` and `source_tree/windows.rs`; module registration; `docs/msc2/storage/contract.md`; `docs/msc2/msc2-port-plan.md`; this plan.
**What:** Discover only verified MSC roots/installations and measure them without overlapping parent/child categories, symlink escape, arbitrary external runtime inclusion or duplicate hard-link accounting. Resolve actual agent configuration/custom server roots and separately staged/bundled assets. Attribute per-server worlds/backups/components/logs while keeping shared tools host-scoped. Return logical/allocated accounting basis, per-volume capacity, scan time/progress and partial errors; retain unknown data as visible, protected inventory. Use bounded cancellable background traversal with limited concurrency and no heavy process/network calls per file. Cache completed/partial results under host/root identity and avoid filesystem-sized memory. Extend only essential controlled-input coverage for overlapping roots, inaccessible data and link boundaries; document runtime, compile without running tests.
**Implementation:** Added discovery from read-only uninstall ownership plus actual decoded configuration/effective server root; disjoint root/category measurement; regular-file logical/allocation totals; native volume/file identities and capacity samples; conservative partial coverage; bounded background scan admission, cancellation/progress and generation-scoped in-memory cache. Shared retained-handle enumeration preserves importer name restrictions while inventory accepts platform-valid names. All inventory remains protected; API/CLI, durable snapshots and cleanup integration belong to later steps. Four essential tiny-file regression cases protect overlap/hard-link accounting, denied/blocked roots, symlink escape and custom-root/world/backup attribution with external Java excluded; under one second each expected, controlled inputs and injected denial, no network/sleeps. Compiled regression harnesses as metadata only. Formatting and required Clippy passed; isolated shared-source Windows adapter type-check passed. No tests/live scans; Linux build and physical platform/accounting acceptance remain pending. See contract implementation notes for bounds and accounting limits.
**Verify:** `cargo clippy -p msc-application -p msc-infrastructure --lib -- -D warnings`
**Batch:** P21.2 only; depends on P21.1 review/verification unless a later explicit batch includes both.
**Commit:** `P21.2: inventory owned storage with bounded scans`

### P21.3 — Expose host inventory and local CLI scan operations

**Status:** Implemented; awaiting owner verification.
**Files:** `crates/msc-agent/src/routes/storage.rs`, operation/capability wiring and DTOs under `crates/msc-api/src/dto/`; OpenAPI and generated client types; `crates/msc-agent/src/cli/mod.rs` and transport modules as needed; supported-client capability record; focused storage route tests; this plan.
**What:** Add authenticated cached-inventory, scan/progress and cancellation operations with stable host/item IDs, explicit coverage and timestamps. Preserve existing map-preview/cleanup endpoints and permission vocabulary; full-host inventory uses an appropriate existing authorized scope, while scoped server data cannot expose unrelated paths or secrets. Reuse operation journaling/admission and implement headless reconciliation/expiry without repeated deep scans. Add host-local CLI inventory/refresh/cancel and JSON output under D-040; do not introduce direct-remote CLI credentials. Advertise capability/minimum version and make older-agent results unavailable explicitly. Separate inventory display data from execution authority; cached results never authorize deletion by themselves.
**Implementation:** Added Admin-only cached inventory/paging, journaled explicit scans, status/cancellation through existing operations, dated bounded snapshot persistence and local CLI inventory/refresh/status/cancel. Capability protocol 1 is additive; no first shipped version is claimed before publication. Reused read-only ownership discovery, actual service paths and existing operation admission; old map cleanup stays compatible. One essential metadata-only regression protects persisted host/schema binding (no I/O/timing, expected under 1 ms); compiled without execution. Required Clippy and generated client types passed. Owner authorized P21.3–P21.8 sequential execution with physical verification deferred until the end; this does not close step statuses or the phase gate.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code`
**Batch:** P21.3 only; depends on P21.2 verification unless included in a newly authorized batch.
**Commit:** `P21.3: expose storage inventory and scan operations`

### P21.4 — Combine saved-host inventories with the local desktop footprint

**Status:** Implemented; awaiting owner verification.
**Files:** A native storage module/commands under `clients/desktop-web/src-tauri/src/`; local installation discovery; `clients/desktop-web/src/lib/platform/`, `auth/desktop.ts`, host-scoped storage state and API adapters as needed; focused host-state tests; this plan.
**What:** Compose the saved-computer overview from cached inventories and explicitly requested remote scans, with bounded fan-out and credentials retained in Rust. Measure the installed local app and local-only client data even when the local agent is absent/offline; merge only matching local-host identities and disjoint roots. An agent may report a verified desktop installation on its host, but inaccessible or undiscoverable remote desktop files remain unknown rather than guessed or assigned the local app's size. Retain dated offline snapshots, invalidate replaced hosts/roots, and guard late responses and scan cancellation across navigation. Folder reveal works only for verified local paths; remote paths are copyable text with their host named. No remote service installation, arbitrary remote path reads or offline cleanup.
**Implementation:** Added native local-client/package measurement independent of agent availability, bounded credential-bound saved-host caches and explicit remote scans, snapshot paging, active scan tracking and local ownership-checked folder reveal. Added physical root/volume identities, labels and link metadata so native/agent composition retains one accounting graph; uncertain cross-source overlaps/shared links are excluded from the combined total with partial coverage instead of being summed. Reused read-only package/service discovery, with independent client discovery on service-inspection failure; ownership configuration reads are now bounded. Added the host-generation bridge and two essential no-I/O regressions for late/wrong-host responses and local double counting/identity mismatch (expected under 1 ms each); compile only. Updated the existing three Clippy allowance names to their current `clippy::` spellings because unqualified spellings are deprecated; no lint scope or release gate changed. Physical Windows/Linux and owner acceptance remain pending.
**Verify:** `cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib -- -D warnings -A clippy::needless_borrow -A clippy::needless_borrows_for_generic_args -A clippy::needless_return`
**Batch:** P21.4 only; depends on P21.3 verification unless included in a newly authorized batch.
**Commit:** `P21.4: compose host storage and local desktop inventory`

### P21.5 — Add reviewed cleanup for disposable and obsolete owned files

**Status:** Implemented; awaiting owner verification.
**Files:** `crates/msc-agent/src/routes/storage.rs`, `map_staging.rs`; storage cleanup/application adapters; Phase 20 staging/operation ownership integration; tool/agent version reference discovery; API/CLI cleanup requests; essential existing cleanup tests; this plan.
**What:** Extend the current preview-bound map cleanup to selected same-category cache/download/obsolete-version entries. Bind an expiring preview to the authenticated host, credential, selected item identities and allowed action; recheck ownership, path identity, changed files, current/rollback versions and live leases immediately before deletion. Active uploads, conversions, maps and shared runtime dependencies remain protected. Resolve interruption/restart and cleanup cancellation at safe boundaries; return removed, retained, failed and remaining items with measured results. Treat reclaimable bytes as an estimate where filesystem sharing prevents exact attribution; refresh affected volume availability without claiming unrelated free-space changes as MSC savings. Reuse existing permission/admission/cancellation mechanisms and prevent cleanup racing with new work. CLI retains local preview and explicit selection/confirmation. Do not repurpose uninstall inventory or general path deletion as authority.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code`
**Batch:** P21.5 only; depends on P21.4 verification unless included in a newly authorized batch.
**Commit:** `P21.5: clean reviewed inactive storage safely`

**Implementation:** Added exact-selection, five-minute host/credential-bound reviews for map cache, transfer-owner staging and verified obsolete desktop-staged builds. Existing upload/download ownership, admission and inspection/consumer leases remain authoritative; unknown, current and rollback builds remain protected. Build staging/repair shares cross-process exclusion with removal. Per-item results are persisted before/after removal; interruption never resumes deletion and unconfirmed outcomes stay explicit. Native/CLI adapters accept typed selections, never arbitrary paths. Corrected the inventory scan runtime route to Axum 0.7 `:id` syntax (OpenAPI retains `{id}`). Windows inventory now opens metadata handles independently from copy handles, avoiding conflicts with active file sharing. No tests or live cleanup executed. Required agent/native Clippy and isolated Windows shared-source compilation passed; physical behavior remains pending.

### P21.6 — Reuse server-data deletion through storage selections

**Status:** Implemented; awaiting owner verification.
**Files:** Storage action coordination; existing `crates/msc-agent/src/routes/backups.rs`, world and server deletion routes/services only where needed; relevant DTO/CLI adapters; desktop confirmation/action state; focused action-permission regressions; this plan.
**What:** Map inventory selections to existing backup/world/server IDs and established deletion services, permissions, backups, stopped-state requirements and confirmations. Allow multiple selected backups within one computer/category, with previews and per-item results; keep world removal and whole-server removal in separate reviews. Show a server removal's actual contained worlds/backups and relevant consequences; block active world/server deletion unless the existing supported lifecycle/action permits it. Preview never stops a server, changes selection or deletes data; any required graceful stop is explicit. Revalidate IDs, owner host, current state and preview before every action. Cancellation stops later items but does not pretend earlier deletions were undone; partial failures remain visible. Refuse a stale host/item or changed source rather than falling back to a raw path. Do not relax deletion rules or trigger complete uninstall.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code`
**Batch:** P21.6 only; depends on P21.5 verification unless included in a newly authorized batch.
**Commit:** `P21.6: manage server data through scoped storage reviews`

**Implementation:** Added host-bound backup/world/server candidates and exact reviews through existing deletion services. Backup batches recheck the last-verified-backup floor before every removal; active slots, unready reconciliation, running servers and overlapping registered roots are retained. World/server reviews each accept one item and have distinct typed confirmations. Whole-server reviews enumerate saved worlds/backups and directory consequences. Preview does not stop or select a server. Existing backup/server deletion routes now share operation admission so they cannot race a Storage cleanup. Per-item cancellation and changed-source results use P21.5 persistence. Existing permission semantics (including Admin's established category override) remain intact. One essential pure permission regression protects Worlds/Fleet distinctions, with controlled credentials, no I/O/timing and expected runtime under 1 ms; compile only. Required agent/native Clippy passed. The binary unit-test target compiled without execution; the broader check still encounters the pre-existing `cli_pairing`/`desktop_auth` route imports and `cli_service` argument mismatches. The documented MSC 1 checkout is unavailable at its stated path; parity was checked against the existing ported deletion services and their source references, not a new direct Swift audit.

### P21.7 — Build the Settings Storage overview and management screens

**Status:** Implemented; awaiting owner verification.
**Files:** `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`, `StorageCleanup.svelte` or its replacement Storage view; host/category/server detail and removal review components under that section; shared host/storage state and existing backup/world/server confirmation components; focused UI behavior cases; this plan.
**What:** Implement the P21.1 reviewed screens using existing design tokens and `antiAIslop.md`: saved-computer overview, host totals/category rows, largest-first servers and their detail, specific category actions, and separate consequential removal reviews. Keep computer identity visible throughout reviews, neutral flat rows with right-aligned sizes and one restrained proportional bar. Show partial/unknown allocation, multi-volume capacity, dated offline data, scan progress/cancel/refresh and truthful cleanup results. Retain navigation state while host-bound operations run; a closed sheet or changed selected server must not redirect actions/results. Require explicit scans before stale offline items become actionable. Display current app/agent/in-use tools as protected inventory with appropriate existing management links, not deletion buttons. Keep safe map cleanup reachable during migration and provide a clear unsupported-agent state. No decorative status dots, gradients or nested cards.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P21.7 only; depends on P21.6 and Cameron's P21.1 visual review unless included in a newly authorized batch.
**Commit:** `P21.7: add settings storage inventory and management`

**Implementation:** Settings now opens a cached-computer Storage overview, per-computer categories/volumes, largest-first servers, grouped server detail, exact selections and typed removal reviews. Refresh is explicit per computer; desktop measurement remains independent of agent availability. Host-bound pending operations survive closing the sheet, do not redirect when another computer is selected, pause status polling on disconnect and offer explicit status/cancel. Removal disables further action until refresh. Remote paths have no local reveal control; protected app/agent/tools show existing management locations. Earlier map cleanup stays available in a separately labelled disclosure. Added server IDs to candidate display metadata to scope server-detail selections. Fixed the schema generator's omitted last DTO field and kept legacy map request/result schemas distinct from reviewed cleanup. Final cleanup inspection also refuses nested mount/volume boundaries and unresolvable relative helper references; outcomes sample each affected volume separately (removed/unavailable sources retain unknown capacity). Required Svelte check passed with only ten pre-existing unused-style warnings; agent/native Clippy passed. No test suites ran. Browser skill connection returned no available browser; actual rendered screenshots and visual acceptance remain pending, not asserted from source inspection.

### P21.8 — Prepare storage acceptance and record owner observations

**Status:** Planned.
**Files:** Storage owner acceptance/design evidence under `docs/msc2/storage/`; supported-client documentation and CLI help guidance as needed; this plan. Generated candidate artifacts/screenshots remain uncommitted.
**What:** Build a matching candidate through existing packaging and give Cameron exact install/verification instructions. Record overview/detail/review visuals; complete and partial accounting with custom roots, multiple volumes, hard links/sparse/clone limits; local agent offline, remote host offline/reconnection and older-agent behavior; large-world scan responsiveness; active upload/map/runtime protection; and same-category backups versus separate world/server deletion in disposable data. Include stale-preview/path/host changes, permission denial, cancellation and partial failure/restart outcomes. Use small controlled files for essential missing regressions, extending existing cases rather than duplicating deletion tests; document concrete risk and expected runtime, compile only unless a specific test command is authorized. Keep actual observations pending until Cameron reports them, then hand the feature gate to the other agent. No publishing, full uninstall, live valued-data deletion, new CI/release gates or automatic test execution.
**Verify:** `cargo build --locked -p msc-agent --bin msc`
**Batch:** P21.8 only; depends on P21.7 verification unless included in a newly authorized batch. Physical acceptance remains Cameron's separate Verify move.
**Commit:** `P21.8: document storage management acceptance`
