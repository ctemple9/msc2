# MSC 2 — Rolling Plan

## Phases

This table is the quick map; detailed steps and historical findings are linked from the [rolling plan archive](rolling-plan-archive.md). Phase gates and scope are defined in the [port plan](msc2-port-plan.md).

| Phase | Name | Current disposition |
|---|---|---|
| Setup | Repository and project documents | Complete |
| 0 | Baseline and harness | Complete |
| 1 | Domain types and pure rules | Complete |
| 2 | API contract and operation model | Complete |
| 3 | Safety substrate | Complete |
| 4 | Java lifecycle | Complete |
| 5 | Configuration and migration | Complete |
| 6 | Worlds and backups | Implemented; historical records archived |
| 7 | Server families and provisioning | Implemented; historical records archived |
| 8 | Mods, plugins and modpacks | Implemented; historical records archived |
| 9 | Networking and helpers | Implemented; historical records archived |
| 10 | Bedrock runtimes | Implemented; historical records archived |
| 11 | Desktop clients | Implemented; historical records archived |
| 12 | Client redesign | Complete |
| 13 | Original terminal UI | Retired by D-034 |
| 14 | Server time and scheduling | Historical work archived |
| 15 | Server state and interaction fixes | Historical work archived |
| 16 | Release safety and codebase readiness | Owner reports the tracked follow-up work resolved; historical acceptance packet retained |
| 17 | Local CLI refinement | Historical work archived |
| 18 | 3D world map | Owner reports tracked map follow-up resolved; acceptance history archived |
| 19 | Complete local uninstall | Implementation records archived |
| 20 | Faster native file imports and uploads | Implementation records archived |
| 21 | Storage inventory and management | Implementation records archived |
| 22 | Restored full-screen TUI | Accepted by Cameron 2026-10-30; implementation records archived |
| 23 | Repository presentation, agent indicator and release follow-ups | Complete; records archived at Cameron’s request 2026-11-05 |
| 24 | Modpack management and content exports | Complete by owner direction; steps archived; illustrated guides deferred; independent gate review pending |
| 25 | Windows spot fixes | Active; owner verifies each issue before commit |

## Current work

Phase 25 follows Cameron's November 10 direction: implement only reported
issues, preserve other platforms, and leave each fix uncommitted until Cameron
confirms it works. This overrides the usual commit-before-verification order. Phase 24's completed steps are in the
[archive](rolling-plan-archive.md#phase-24--modpack-management-and-content-exports).
Illustrated launcher guides (P24.20) are deferred by Cameron. Independent gate
review and unrecorded physical evidence remain tracked in the
[acceptance record](addons/phase24-acceptance.md).

Owner acceptance - 2026-11-11: Cameron requested marking P25.1-P25.5 done
and committing them after the performance recheck. Earlier pending/uncommitted
notes below describe the implementation history. This acceptance supersedes
those statuses without claiming unrecorded platform or scenario checks passed.
The original thumbnail commit is retained; P25.2-P25.5 receive separate commits.
No phase gate review, release or publication is implied.

## Phase 25 ? Windows spot fixes

### P25.1 — allow provider catalog images in the desktop app

**Status:** DONE - owner accepted and authorized commit on 2026-11-11.

**Files:** `clients/desktop-web/src-tauri/tauri.conf.json`; `docs/msc2/rolling-plan.md`.

**What:** Owner requested fixing blocked Modrinth and CurseForge images after
Windows Intel 0.1.44 first-server setup showed missing datapack thumbnails.
Added Modrinth's HTTPS CDN and HTTPS CurseForge CDN subdomains to the desktop
image allowlist, retaining every existing source and all other security
directives. This shared desktop configuration applies on Windows, macOS and
Linux; no platform-specific behavior or UI code changes. The existing
CurseForge media host was already allowed, so this addresses blocking on other
CDN subdomains rather than claiming all CurseForge images were blocked.
Owner follow-up November 11: Modrinth thumbnails appeared. CurseForge image
verification remains pending because its API key was not configured; see P25.3.
No tests added or run; JSON formatting and diff inspection are the relevant
non-test checks. Physical thumbnail/gallery verification remains pending.

**Verify:** `cd clients/desktop-web; npx tauri dev` — in the native app, search
Modrinth and CurseForge datapacks during creation, then inspect project
thumbnails and galleries; repeat representative mod/modpack searches. Confirm
existing player images still load. Windows owner verification and macOS/Linux
physical regression checks remain pending. The installed 0.1.44 app needs a
rebuilt native shell to receive this configuration change.

**Batch:** This issue only; stop for owner verification.

**Commit:** Previously committed prematurely as `P24.45: allow modrinth and curseforge catalog images` (`c8133dda`); history retained. Reclassified as P25.1 by owner direction. Owner accepted this step on 2026-11-11; original commit history retained.


### P25.2 - repair an existing Windows desktop agent for the current app build

**Status:** DONE - owner accepted and authorized commit on 2026-11-11.

**Files:** `crates/msc-agent/build.rs`; `clients/desktop-web/src-tauri/src/lib.rs`; `clients/desktop-web/src-tauri/src/windows_service.rs`; `clients/desktop-web/src-tauri/src/windows_service.ps1`; `packaging/windows/desktop-service-lifecycle.ps1`; this plan.

**What:** Repair previously called first-install registration, which refuses an
existing service. Route Windows repair through the existing protected service
coordinator to replace the owned executable bundle and explicitly start it.
Retain the service account, stored password, environment, data directory and
startup policy. Validate account/data/port and desktop ownership before changing
anything; retain the graceful maintenance handshake and rollback record. A
legacy desktop build can be repaired to a different build after its digest and
standard owner data directory are validated. Marked headless installs remain
independent. Transport the embedded elevated script compressed in memory to
stay within Windows command-line limits without an elevated temporary script.
Resolve the machine-staged binary after repair. macOS/Linux service paths are
unchanged. No tests added or run; no live service repair performed by the agent.
PowerShell parser checks and `git diff --check` passed; Rust formatting applied
to the changed files. `cargo clippy --manifest-path
clients/desktop-web/src-tauri/Cargo.toml --locked --lib -- -D warnings` stopped
on the pre-existing `clippy::needless_return` in `installing_user` at
`src/lib.rs:1553`; an infrastructure dead-code warning was also reported.
No unrelated lint fixes made; no clean-Clippy claim. Owner verification pending.

**Verify:** `cd clients/desktop-web; npx tauri dev` ? close the installed release
app first, click Repair agent in the development app and approve Windows UAC.
Confirm Agent connected, existing servers/settings remain, and Modrinth and
CurseForge images load (P25.1). Repair again to confirm repeat use. The two apps
share one agent; different builds require repair when switching between them.
Owner physical verification remains pending.

**Batch:** P25.2 only; owner has now authorized commit.

**Commit:** `P25.2: repair existing windows desktop agent builds`

**Owner verification follow-up:** Cameron approved UAC but the app returned to
unavailable. Windows Application Error events identify `0xc00000fd` (stack
overflow) in the newly staged debug agent, at `__chkstk` according to its PDB.
The service path was restored by rollback. Inspection of the debug PE reports
a 1 MiB stack reserve and an unoptimized TUI future function frame of 2,749,624
bytes, which exceeds that reserve. Added `crates/msc-agent/build.rs` to reserve
16 MiB of stack address space only for Windows MSVC debug agent executables;
pages are committed on demand. Release agents and other platforms keep their
existing linker settings. This is a same-step repair; P25.2 remains uncommitted
and owner verification has not passed.

Follow-up non-test verification: `cargo build --locked -p msc-agent` passed
with existing warnings. PE header inspection confirms the rebuilt debug
executable reserves 16,777,216 bytes of stack. No service launch or tests were
run; Cameron must restart `npx tauri dev` to stage it, then retry Repair agent.

### P25.3 — show the existing CurseForge key form for catalog searches

**Status:** DONE - owner accepted and authorized commit on 2026-11-11.

**Files:** `crates/msc-agent/src/routes/components.rs`; `clients/desktop-web/src/lib/sections/components/ProviderCatalogBrowser.svelte`; this plan.

**What:** Owner screenshot shows the creation wizard's plugin browser reporting
no CurseForge key with only Retry. Its existing key-entry form requires the
`missing_curseforge_api_key` response code, but the shared catalog error mapper
returned `provider_unavailable` for every provider error. Return the specific
missing-key code with HTTP 409, matching the world-pack route convention. Keep
other provider failures unchanged. Reuse the existing form, agent secret storage,
permission checks and retry-after-save behavior; no UI redesign or platform
branch changes. No tests added or run. `cargo fmt --all --check`,
`cargo check --locked -p msc-agent` (existing warnings only), and
`git diff --check` passed.

**Verify:** `cd clients/desktop-web; npx tauri dev` — repair the agent to stage
the rebuilt source, then open Add Server → Add-ons → Browse plugins → CurseForge
without a configured key. Confirm the existing password field, console link and
Save key control appear. Save a key and confirm results/thumbnails load without
losing staged selections. Confirm Modrinth still browses normally.

**Batch:** P25.3 only; owner has now authorized commit.

**Commit:** `P25.3: expose curseforge key entry for catalog searches`

P25.3 owner UI follow-up: screenshot confirms the missing-key form now appears.
Removed the redundant API key focus link and explanatory paragraph at Cameron's
request; retained the console link, password field, Save key action and existing
permission handling. Key saving/catalog results still await owner verification.
Prettier check passed. No commit or tests. This frontend-only adjustment updates
in the running development app without another agent repair.


### P25.4 - use the creation draft for World step context

**Status:** DONE - owner accepted and authorized commit on 2026-11-11.

**Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/WorldStep.svelte`;
`clients/desktop-web/src/lib/sections/worlds/WorldSettingsForm.svelte`;
`crates/msc-agent/src/routes/capabilities.rs`; this plan.

**What:** Owner selected Purpur and installed/selected Java 25, but World showed
Paper and host-default Java 21. World omitted the selected runtime path from its
capability request and refresh key. Include it, snapshot edition/flavor/runtime
across asynchronous requests, and reject responses for a different edition or
flavor instead of displaying incorrect context. The current source already sent
the flavor; its reported mismatch was not independently reproduced. Prevent
explicit creation contexts from inheriting missing flavor/loader/type values
from the existing active server. Ordinary active-server capability requests
retain their fallback behavior; Bedrock sends no Java runtime context. No tests
added or run. No Java installation deleted by the agent.

**Verify:** `cd clients/desktop-web; npx tauri dev` ? repair the rebuilt agent,
create Purpur with latest Minecraft, install/select Java 25, and enter World.
Confirm Purpur and available Java runtime; go back, change flavor/version and
select its compatible runtime, then confirm refreshed context. Check Bedrock
shows Bedrock without Java context and an existing server's settings still use
that server. Owner verification pending.

**Batch:** P25.4 only; owner has now authorized commit.

**Commit:** `P25.4: bind world context to creation selections`

Retry preparation: inspected the managed Windows installation at
`C:/Users/Cameron/AppData/Local/MSC2/runtimes/temurin-25-windows-x64`.
Owner can remove that exact directory after closing the wizard and stopping
any Minecraft server using it; no other Java installation is targeted.

P25.4 context also displays the detected Java major beside availability, so the
owner can verify Java 25 directly. `cargo check --locked -p msc-agent` passed
with existing warnings. Frontend type-check and formatting verification recorded
below. No runtime removal or tests performed.

P25.4 final frontend checks: Prettier passed; `npm run check` passed with
0 errors and 10 existing unused CSS warnings. Owner verification pending.


### P25.5 - avoid repeated Windows machine-build hashing during status refresh

**Status:** DONE - owner accepted and authorized commit on 2026-11-11.

**Files:** `clients/desktop-web/src-tauri/src/lib.rs`; this plan.

**What:** Owner reports desktop lag, Windhawk popups and local status network
failures. Read-only sampling found desktop using 25.5 CPU seconds over two wall
seconds and about 1.4 GiB working memory; agent service remained running and
healthz returned 204. Windhawk was idle during that sample; no causal claim for
its popups. Windows expected-agent lookup reread and hashed all three executables
on every guarded API request. Cache successful validation of the immutable,
administrator-protected machine build per desktop process, serialize first
verification, and revalidate when the expected build path changes. Still query
actual service identity for every lookup; failures are not cached. Other
platform paths unchanged. No tests or live operations run.

**Verify:** `cd clients/desktop-web; npx tauri dev` ? after closing the old app,
check local server status refreshes reliably and desktop CPU/memory settles;
retry after a Repair to a new build. Windhawk popup cause remains unconfirmed.

**Batch:** P25.5 only; owner has now authorized commit.

**Commit:** `P25.5: cache protected windows agent build verification`

P25.5 checks: Rust formatting and `git diff --check` passed; native desktop
`cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml --locked
--lib` passed with an existing infrastructure dead-code warning. Owner
performance/network verification remains pending.

P25.5 accepted recheck: desktop used 0.11 CPU seconds over 3 seconds and 73 MB
RAM; agent was running at 53 MB and healthz returned 204. About 9 GB RAM was
free. This confirms lower load in that sample; Windhawk causality and sustained
status-refresh reliability were not independently established.

Commit preparation: workspace formatting passed. Agent and native desktop
Clippy completed with pre-existing warnings (including needless returns); no
unrelated warning fixes, tests, release actions or platform-pass claims.
