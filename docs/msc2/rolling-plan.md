# MSC 2 — Rolling Plan

**Current priority:** Implement the agent-owned running indicator approved by Cameron on 2026-10-30. Repository presentation polish remains recorded below; screenshots are deferred to Cameron.

### P12.253 — Keep the Agent page's local service status current

**Status:** Implemented; awaiting Cameron's verification. Owner reported the connected Windows agent remaining at checking with no Stop agent action.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `service-status.ts`, `clients/desktop-web/tests/agent-install/service-status.test.ts`, this plan.
**What:** Replace one-shot service checks with a visible-page monitor. Refresh on activation, reconnect and after service actions, then five seconds after each completed query. Reject stale replies across page/host changes and service mutations. A failed check shows unavailable and its actual error; a ten-second timeout replaces indefinite checking without piling up native requests. Reopening while an earlier native query is stuck also gets a bounded unavailable state. Stop polling while hidden, remote or busy; dispose timers on destruction. Offer Refresh status and label unknown service state Checking/Status unavailable instead of Start agent. Use existing button and detail styles after reading antiAIslop.md.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.253 only.
**Commit:** `P12.253: refresh local agent service status`

**Evidence and checks:** Read-only Windows `sc.exe queryex com.ctemple.msc2.agent` confirmed RUNNING, STOPPABLE, PID 5448. Source exposed one-shot refresh with no retry and an unset status on failure, explaining how the page can stay at checking; the screenshot alone does not establish the original native query's exact failure. Frontend type-check and production build passed with existing warnings outside changed files. Rust formatting and agent Clippy passed with existing warnings. No service mutations, test execution, release or installation performed.

**Essential tests:** Three deterministic monitor cases protect recovery after a rejected query, bounded display/no overlapping native queries during a hung request including hide/reopen, and stale-result rejection/polling shutdown when hidden. Deferred promises and fake timers avoid real service access, arbitrary waiting or environment assumptions; expected total runtime under one second. Existing source-only screen assertions did not exercise asynchronous service status behavior. Tests added but not run.

### P12.252 — Persist server Java selections and deduplicate Windows runtimes

**Status:** Implemented; awaiting Cameron's verification. Owner approved per-server Java ownership on 2026-11-03 after Java 25 selected during Paper creation reverted to the host's Java 21.
**Files:** Server configuration schema and focused round-trip test; agent creation, lifecycle, version/config routes, main routing and CLI; runtime detection and focused identity test; server Java editor and route-model comments; API contract/generated client; decision D-044, engineering/CLI docs and this plan.
**What:** Persist explicit creation-time Java selections per server. Resolve saved server Java before the host fallback for launches and installer-based version changes; read the latest saved selection at start so a settings edit does not depend on restarting MSC. Scope the Java editor's executable and arguments to its explicit server ID. Empty server path restores host inheritance; explicit java means PATH. Keep unscoped API/CLI access for the host default and add msc java get/set --server <id>. Preserve old configurations through missing optional fields; do not invent historical server selections. Compare Windows executable identities without separator/case differences while preserving Unix path case and displayed executable paths. Owner decision D-044 supersedes previous host-wide editor notes. Read antiAIslop.md; retain the existing editor layout.
**Verify:** `cargo clippy -p msc-agent --bin msc --no-deps`
**Batch:** P12.252 only.
**Commit:** `P12.252: persist server java selections`

**Essential tests:** Three controlled cases protect server-override serialization/backward compatibility, isolated server settings and reset-to-inheritance/unknown-ID refusal, and Windows path identity with Unix case preservation. These cover concrete bugs without live Java, downloads, timers or physical-platform assumptions; expected combined runtime under one second. Added to existing test files; no test suites executed.

**Checks:** Frontend type-check and production build; Rust formatting and agent Clippy; domain configuration and infrastructure detection regression-source compilation. Existing warnings remain outside changed code. An exploratory broad agent test-source compilation encountered unrelated stale CommonArgs fields in tests/cli_service.rs; this is not a passing suite claim. The targeted agent test-profile type-check passed. No installation, release, or live Java configuration changes performed.

**Renderer diagnosis (owner-authorized single startup check):** The installed Windows Vantage 0.15.1 crashed using a temporary world assembled from the existing prepared region cache and a copy of level.dat; no live world writes. Exit 3221225501 (0xC000001D, illegal instruction), no stderr/stdout. Windows Application events show the same fault offset 0x1766da for the owner's attempts at 01:54:40/01:54:54 and the diagnostic at 02:00:44. At that RVA the installed PE contains bytes 0f 38 cc ee (SHA256MSG1), pointing to an incompatible CPU instruction in the native Windows renderer; this is independent of the server JDK. The diagnostic folder remains at %TEMP%/msc2-renderer-diagnostic-pw2new2s. Renderer source, arguments, binaries and all platform release pins remain unchanged in this Java fix. macOS/Linux renderer execution was not verified on this Windows host. A compatible Windows renderer build is still required; this step does not claim to repair that separate native crash. Opcode reference: Intel's SHA Extension Definitions, https://www.intel.com/content/dam/develop/external/us/en/documents/intel-sha-extensions-white-paper-402097.pdf.

### P12.251 — Explain cleanup after incomplete Playit setup

**Status:** Implemented; awaiting Cameron's verification. Owner authorized this recovery message after reviewing the Windows first-start failure records.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/PlayitSetupSheet.svelte`, its `model.ts`, `clients/desktop-web/tests/playit/playit-setup.test.ts`, this plan.
**What:** Explain that incomplete setup may leave Playit agents/tunnels behind; direct the user through Reset local setup, website cleanup of only the failed attempt's resources, and returning to setup. Keep Reset accessible despite stale credential status, show the original error, and carry the website reminder through reset completion. Incorrect credentials, banned accounts and two-factor sign-in failures retain their existing handling. Generic provider failures are treated conservatively because operation polling may miss the claim stage. MSC never deletes cloud resources. Read `antiAIslop.md`; reuse existing sheet typography, buttons and spacing.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.251 only.
**Commit:** `P12.251: explain cleanup after incomplete playit setup`

**Essential test:** Extend the existing Playit unit file with one controlled-input case protecting cleanup classification: partial-setup errors require guidance even if polling missed progress, while rejected sign-in does not prompt cloud cleanup. No network, timers or environment dependencies; expected runtime under one second. Test execution remains Cameron's responsibility.

**Checks:** Frontend type-check and production build passed (existing warnings outside the changed files); changed frontend files formatted. Rust formatting passed and agent Clippy completed with existing Windows warnings. No tests run. Visual verification remains Cameron's; no release or installed-app update performed.

### P16.43 — Plan a complete Windows MSI refinement
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

### P23.26 — Recover v0.1.42 with the Apple Silicon ticket-transport workaround

**Status:** awaiting Cameron verification; recovery publication pending
**Files:** `.github/workflows/release.yml`; `.github/workflows/release-recovery.yml`; `docs/msc2/rolling-plan.md`
**What:** v0.1.42 Apple Silicon passed notarization but exhausted all three staple attempts with the same -1005 CloudKit transport error. Extend the documented UDP-443 workaround to both macOS runners. Add a manually invoked, fixed-v0.1.42 build-only recovery workflow: check out the unchanged release tag, build only Apple Silicon with the workaround, wait for original run 41551215711, require its Intel/Windows/Linux artifacts to be successful and from the same source commit, and combine those artifacts with the recovered Apple Silicon build. Retain nine-asset assembly, checksums, signing, Gatekeeper, ticket validation and signed update metadata. Do not move the tag or rebuild successful platforms. This avoids another full-release attempt; expected Apple Silicon rebuild time is about 15–20 minutes. No test suites or unrelated gates added.
**Verify:** `gh release view v0.1.42 --json url,isPrerelease,assets` — require complete downloads and signed metadata; owner map verification remains separate.
**Batch:** solo
**Commit:** `P23.26: recover apple silicon release ticket transport`

### P23.25 — Use GitHub's Intel Apple-ticket transport workaround and release v0.1.42

**Status:** awaiting Cameron verification; publication pending
**Files:** `.github/workflows/release.yml`; agent and desktop version manifests and lockfiles; `clients/desktop-web/src/lib/bundle-identity.ts`; `README.md`; `docs/msc2/release/v0.1.42.md`; `docs/msc2/rolling-plan.md`
**What:** Both v0.1.41 Intel attempts passed signing, notarization, stapling and Gatekeeper, then exhausted ticket-validation retries with -1005. All other platforms passed. GitHub maintainers reproduced the Intel runner's broken HTTP/3 path in actions/runner-images#14924 and supply a packet-filter rule blocking outbound UDP 443, forcing Apple's tool to use working HTTPS over TCP. Apply their exact rule only on the ephemeral Intel hosted runner, before Tauri's app notarization as well as DMG operations. Preserve every signing and publication requirement and retain bounded retries. Publish a new immutable v0.1.42 tag including the renderer fix; do not move v0.1.41. Full rebuild cost is about 40 minutes; the documented transport workaround addresses the repeated failure, while Apple service availability remains external. No tests or additional gates.
**Verify:** `gh release view v0.1.42 --json url,isPrerelease,assets` — confirm nine downloads, SHA256SUMS and signed update metadata after a successful workflow; Cameron verifies the map later.
**Batch:** solo
**Commit:** `P23.25: fix intel apple ticket transport and release v0.1.42`

### P23.24 — Prepare and publish v0.1.41

**Status:** prepared; publication pending; physical verification deferred
**Files:** agent and desktop version manifests and lockfiles; `clients/desktop-web/src/lib/bundle-identity.ts`; `README.md`; `docs/msc2/release/v0.1.41.md`; `docs/msc2/rolling-plan.md`
**What:** Publish the next immutable tag at Cameron's request, including the renderer header-padding fix and P23.23's bounded retry of Apple's explicit ticket-stapling connection failure. All three other platform builds passed in v0.1.40; Intel passed build and notarization before the transport failure. Retain all nine artifacts, checksums, signed update metadata and macOS signing requirements. No tests run. This is a new full platform build and may take roughly 40 minutes; external Apple availability remains uncertain.
**Verify:** `gh release view v0.1.41 --json url,isPrerelease,assets` — confirm the complete published artifact set and metadata; Cameron will verify map behavior later.
**Batch:** solo
**Commit:** `P23.24: prepare v0.1.41 release`

### P23.23 — Retry interrupted Apple disk-image ticket stapling

**Status:** awaiting Cameron verification
**Files:** `.github/workflows/release.yml`; `docs/msc2/rolling-plan.md`
**What:** Inspect all four v0.1.40 jobs: Linux, Windows and Apple Silicon succeeded; Intel passed build, signing and notarization but failed stapling when Apple's ticket service lost its network connection (NSURLErrorDomain -1005, exit 68). Apply the existing validation retry policy to the earlier staple operation: at most three attempts, ten seconds apart, only for the explicit connection-loss diagnostic. Preserve the original failure status on exhaustion or any other error. Keep all signature, notarization, artifact and metadata requirements unchanged. No tests or new gates added. A rerun of v0.1.40 uses its original immutable workflow, not this correction.
**Verify:** `git show -- .github/workflows/release.yml` — confirm only the staple command gains the existing narrowly scoped retry policy; the next corrected-workflow release must finish publication with all nine assets and signed metadata.
**Batch:** solo
**Commit:** `P23.23: retry interrupted apple ticket stapling`

### P23.22 — Prepare and publish v0.1.40

**Status:** prepared; publication pending; Cameron will test later
**Files:** agent and desktop version manifests and lockfiles; `clients/desktop-web/src/lib/bundle-identity.ts`; `README.md`; `docs/msc2/release/v0.1.40.md`; `docs/msc2/rolling-plan.md`
**What:** Align release identities at 0.1.40 and publish a new immutable tag containing P18.100's renderer signing-space correction and the supported Apple Silicon runner change. Preserve the build-only workflow, nine artifacts, checksums and signed update metadata. No tests run; physical map verification is deferred at Cameron's direction.
**Verify:** `gh release view v0.1.40 --json url,isPrerelease,assets` — confirm all nine desktop/headless artifacts, SHA256SUMS and signed update metadata are published, then test the ATM10 map later.
**Batch:** solo
**Commit:** `P23.22: prepare v0.1.40 renderer fix release`

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


### P23.27 - Merge Windows fixes and publish v0.1.43

**Status:** Prepared; publication requested, owner verification pending.
**Files:** Merged source and generated API client; version manifests/lockfiles and bundle identity; Windows renderer staging; TUI Java editor; release notes and this plan.
**What:** Preserve both remote main and the three local fix commits. Resolve service-monitor/indicator conflicts, regenerate the merged API client, renumber the Java decision D-044 to preserve the existing TUI D-043, and align the TUI Java editor with server scope. Use baseline CPU only for the Windows renderer to avoid unsupported SHA instructions. Synchronize application identity at 0.1.43, then push main and the new immutable tag through the existing build-only workflow. Preserve macOS signing, all nine packages, checksums and signed metadata. No test suites, service mutations or release gates added. Expected remote build time roughly 40 minutes; Apple service availability remains external.
**Verify:** `gh release view v0.1.43 --json url,isPrerelease,assets`
**Batch:** P23.27 only.
**Commit:** `P23.27: merge windows fixes and release v0.1.43`

**Merge/release checks:** Preserved all three original local fix commits and remote main. Resolved API generation, Java editor comments, service-status/indicator and documentation conflicts; Java is D-044 because remote D-043 restores the TUI. Frontend type-check/build, locked Cargo metadata, formatting and agent Clippy passed with existing warnings. The TUI Java editor now also targets its named server. No test suites or full local renderer build ran. Windows CPU-target change is build-only and physical acceptance remains pending.


### P23.28 - Recover Apple Silicon compiler installation for v0.1.43

**Status:** Prepared; recovery publication requested, owner verification pending.
**Files:** tools/release/stage-vantage.py; .github/workflows/release-recovery.yml; this plan.
**What:** All original run 41602854205 jobs except Apple Silicon succeeded. Apple Silicon lost the cached Zig executable, then pip skipped replacing existing target directories, leaving no compiler. Repair future installations with --upgrade and a missing-compiler diagnostic. Adapt the existing build-only recovery to the immutable v0.1.43 tag and original run: install the exact Zig 0.16.0 wheel into an isolated runner-temp directory and supply the existing MSC2_ZIG_BIN override, then build only Apple Silicon. Retain source-SHA/job-success checks before reusing Intel/Windows/Linux artifacts, complete nine-package assembly, checksums, macOS signing/notarization/stapling and signed update metadata. No tag movement, test suites or additional release gates. Expected 15-25 minutes; Apple service availability remains external.
**Verify:** `gh release view v0.1.43 --json url,isPrerelease,assets`
**Batch:** P23.28 only.
**Commit:** `P23.28: recover apple silicon renderer compiler`

**Checks:** Reviewed every original platform conclusion and the failed compiler-install log. Python source syntax and recovery workflow references inspected; source checkout and retained artifacts must resolve to the same v0.1.43 commit. No tests run. Recovery overrides only the compiler location, not release application source.
