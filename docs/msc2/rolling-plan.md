# MSC 2 — Rolling Plan

**Current priority:** Implement the agent-owned running indicator approved by Cameron on 2026-10-30. Repository presentation polish remains recorded below; screenshots are deferred to Cameron.

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

## Current work

### P18.100 — Preserve renderer code during macOS signing

**Status:** awaiting Cameron verification
**Files:** `third_party/vantage/build.zig`; `docs/msc2/rolling-plan.md`
**What:** Reserve 4096 bytes of Mach-O header padding on macOS. The November 2 Intel crash report shows signing metadata overwriting the entry instructions of `libdeflate_crc32`, reached while reading the world's spawn. Re-signing a temporary copy of the locally installed renderer reproduced an eight-byte overwrite at the beginning of `__text`: the load commands grew from ending at 0x7d8 to 0x7e8, while code began at 0x7e0. The local executable UUID differs from the supplied report, so this reproduces the packaging defect rather than identifies that exact artifact. Fix the executable layout rather than bypass another checksum function; retain existing decompression behavior and world files. No tests or release workflow gates added.
**Verify:** `python3 tools/release/stage-vantage.py --platform macos-x86_64 --output-dir /private/tmp/msc-renderer-headerpad` — package the corrected renderer in the next signed Intel candidate, open the same ATM10 map, and confirm startup succeeds without a new vantage crash report. Local build and before/after signing inspection are recorded in the implementation handoff; world verification remains Cameron's.
**Batch:** solo
**Commit:** `P18.100: reserve renderer header space for macos signing`
**Local checks:** Intel `zig build -Doptimize=ReleaseFast`, Zig formatting and `git diff --check` passed. With padding, code begins at 0x17e0; ad hoc hardened-runtime signing grows load commands from 0x7d8 to 0x7e8 and preserves all 1,509,856 instruction bytes. Strict signature verification passed. Developer ID packaging and the ATM10 map remain owner verification; no tests run.

The previous phase work is finished. The old Phase 16 acceptance packet and owner-testing findings remain in the archive as historical records; their earlier pending statuses describe the state when recorded, not the current work list.

### Repository presentation polish

Completed P23.1–P23.2 and P23.4–P23.10 records are in the [archive](rolling-plan-archive.md#phase-23--completed-presentation-and-indicator-planning-steps). Original statuses and verification notes are preserved there.

#### P23.3 — Add current product screenshots
**Status:** deferred; Cameron will add screenshots later
**Files:** `README.md`, `docs/msc2/rolling-plan.md`, screenshot assets to be chosen during the step
**What:** Show the current desktop app and host-local TUI near the top of the README, with accurate captions and accessible descriptions.
**Verify:** `git diff HEAD^ HEAD -- README.md`
**Batch:** solo

### Agent running indicator

The helmet appears only while the local agent runs. A small companion process owns the graphical icon in the signed-in user's session; the agent continues to work on headless hosts without it. The companion receives local agent lifecycle state through an event-driven connection, not periodic HTTP health checks. Losing the connection removes the icon. Only the desktop package registers the companion at login.

#### P23.11 — Add the local agent lifecycle channel
**Status:** awaiting Cameron verification
**Files:** `crates/msc-agent/src/`, `docs/msc2/rolling-plan.md`
**What:** Expose an authenticated, host-local connection that stays open while the agent runs and closes on shutdown or crash. Reuse the existing local IPC and owner boundary on macOS, Windows, and Linux where possible. Keep the headless agent free of GUI dependencies and avoid periodic HTTP requests.
**Verify:** `cargo check -p msc-agent`
**Batch:** solo
**Commit:** `P23.11: add the local agent lifecycle channel`

#### P23.12 — Build the running-only helmet companion
**Status:** awaiting Cameron verification
**Files:** `clients/desktop-web/src-tauri/`, `docs/msc2/rolling-plan.md`
**What:** Launch a separate user-session process with the approved pixel helmet in the native system bar. Show it only while connected to the local agent channel; hide it immediately on disconnect and reconnect with bounded backoff. Provide Open MSC 2 and Quit indicator without changing agent lifecycle.
**Verify:** `cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Batch:** solo
**Commit:** `P23.12: build the running-only helmet companion`

#### P23.13 — Register the companion with desktop installations
**Status:** awaiting Cameron verification
**Files:** `clients/desktop-web/src-tauri/`, `packaging/macos/`, `packaging/windows/`, `packaging/linux/`, `docs/msc2/rolling-plan.md`
**What:** Start one indicator companion per signed-in desktop user at login on macOS, Windows, and Linux where a tray is available. Include it in desktop installs, updates, repair, and removal; leave standalone headless packages unchanged.
**Verify:** `git diff HEAD^ HEAD -- clients/desktop-web/src-tauri packaging`
**Batch:** solo
**Commit:** `P23.13: register the agent indicator for desktop sessions`

#### P23.14 — Document and inspect platform behavior
**Status:** awaiting Cameron verification
**Files:** `README.md`, `docs/msc2/agent-indicator.md`, `docs/msc2/rolling-plan.md`
**What:** Explain the indicator's local-only meaning and login-session limits. Inspect startup and removal paths on all platforms; hand physical macOS, Windows, and Linux checks to Cameron without running test suites unless he requests them.
**Verify:** `git diff HEAD^ HEAD -- README.md docs/msc2/rolling-plan.md packaging`
**Batch:** solo
**Commit:** `P23.14: document the agent indicator and platform checks`

#### P23.15 — Share the desktop and indicator application context
**Status:** awaiting Cameron verification
**Files:** `clients/desktop-web/src-tauri/src/lib.rs`, `clients/desktop-web/src-tauri/src/agent_indicator.rs`, `docs/msc2/rolling-plan.md`
**What:** Fix the macOS build failure reported during owner verification. Generate Tauri's application context in one shared factory so the two process modes embed only one Info.plist symbol.
**Verify:** `cargo build --manifest-path clients/desktop-web/src-tauri/Cargo.toml --no-default-features`
**Batch:** solo
**Commit:** `P23.15: share the desktop and indicator application context`

#### P23.16 — Control the indicator from the local Agent page
**Status:** awaiting Cameron verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/src-tauri/src/lib.rs`, `clients/desktop-web/src-tauri/src/indicator_startup.rs`, `docs/msc2/agent-indicator.md`, `docs/msc2/rolling-plan.md`
**What:** Add a saved Enable agent indicator setting and a Show indicator action for restoring the companion after quitting it. Scope controls to the local desktop host. Enabling also works in debug builds without registering a development executable at login.
**Verify:** `cd clients/desktop-web && npx tauri dev` — open the local Agent page, use Show indicator after quitting its menu, then disable and re-enable the setting.
**Batch:** solo
**Commit:** `P23.16: control the indicator from the local Agent page`

#### P23.17 — Use one indicator toggle beside the service buttons
**Status:** awaiting Cameron verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src-tauri/src/lib.rs`, `clients/desktop-web/src-tauri/src/agent_indicator.rs`, `docs/msc2/agent-indicator.md`, `docs/msc2/rolling-plan.md`
**What:** Remove the separate indicator heading, explanatory paragraph, and checkbox. Put one Show/Hide indicator toggle beside Start/Stop agent and Repair agent. Check the companion's actual process lock so quitting its menu does not prevent restoring it from the button.
**Verify:** `cd clients/desktop-web && npx tauri dev` — on the local Agent page, toggle the indicator on and off using the button beside Repair agent.
**Batch:** solo
**Commit:** `P23.17: simplify the agent indicator control`

#### P23.18 — Use a terminal editor on Windows
**Status:** awaiting Cameron verification
**Files:** `crates/msc-agent/src/cli/tui/files.rs`, `docs/msc2/rolling-plan.md`
**What:** Use Microsoft Edit (`edit.exe`) for the Windows TUI's Files editing action, with an actionable `winget install Microsoft.Edit` message when unavailable. Preserve nano and its arguments on macOS/Linux, the existing selected-server path boundary, and terminal restoration after the editor exits or fails. No additional tests: this is a small platform command selection; native Windows launch remains an owner check.
**Verify:** `cargo run -p msc-agent -- tui` — on Windows open a text file from Files, save in Microsoft Edit, exit, and confirm the TUI returns; on macOS confirm the same action still uses nano.
**Batch:** solo
**Commit:** `P23.18: use Microsoft Edit for Windows TUI files`

#### P23.19 — Prepare and publish v0.1.39
**Status:** prepared; publication pending
**Files:** agent and desktop version manifests and lockfiles, `clients/desktop-web/src/lib/bundle-identity.ts`, `README.md`, `packaging/windows/desktop-cli-path.wxs`, `docs/msc2/release/v0.1.39.md`, `docs/msc2/rolling-plan.md`
**What:** Align v0.1.39 identities and release notes for the restored TUI, agent indicator, and intervening fixes. During packaging inspection, replace an invalid WiX RemoveRegistryValue uninstall attribute with the desktop executable's explicit indicator-unregistration action. Preserve the build-only release workflow, nine artifacts, checksums, and signed update metadata. Publish one new immutable tag; no test suites are authorized or run.
**Verify:** `gh release view v0.1.39 --json url,isPrerelease,assets`
**Batch:** solo
**Commit:** `P23.19: prepare v0.1.39 release`

#### P23.20 — Replace the retiring Apple Silicon release runner
**Status:** implemented; v0.1.39 recovery pending
**Files:** `.github/workflows/release.yml`, `docs/msc2/rolling-plan.md`
**What:** Change future Apple Silicon release builds from macos-14 to macos-15. The v0.1.39 Apple Silicon job was canceled before receiving a runner during GitHub's scheduled macos-14 brownout; all three other platforms uploaded artifacts successfully. Preserve the immutable v0.1.39 tag. Its one targeted Apple Silicon retry uses the original tagged workflow outside the brownout window, before the runner's November 2 retirement, reusing the successful platform artifacts and allowing publication to proceed. No test, lint, or other gates added.
**Verify:** `gh run view 41134932773 --json status,conclusion,jobs`
**Batch:** solo
**Commit:** `P23.20: replace the retiring Apple Silicon release runner`

#### P23.21 — Archive recently completed steps
**Status:** documentation cleanup prepared; awaiting Cameron verification
**Files:** `docs/msc2/rolling-plan.md`, `docs/msc2/rolling-plan-archive.md`
**What:** Move the nine steps explicitly marked done (P23.1–P23.2 and P23.4–P23.10) into the archive without changing their records. Keep deferred screenshots, steps awaiting verification, and pending release work in the rolling plan.
**Verify:** `git show --stat --oneline HEAD && git show --format= -- docs/msc2/rolling-plan.md docs/msc2/rolling-plan-archive.md` — confirm the nine completed records moved intact and unfinished work remains visible.
**Batch:** solo
**Commit:** `P23.21: archive recently completed rolling plan steps`

**Phase 22 disposition:** Cameron accepted the current TUI on 2026-10-30. Detailed P22 records and their original verification commands are in the archive.

## Archived owner-testing details

Detailed records for Findings 5, 9, 10, 15 and 26, plus P16.97, P16.101, and P16.130–P16.136, are in the [rolling plan archive](rolling-plan-archive.md#archived-owner-testing-records--0132). These records preserve the investigation and verification history. Cameron confirmed their follow-up work is finished on 2026-10-30.
