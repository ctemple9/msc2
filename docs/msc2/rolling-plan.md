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
