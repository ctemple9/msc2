# MSC 2 — Rolling Plan

### P12.240 — Keep the initial Bedrock world in its original slot

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/provisioning.rs`, `crates/msc-application/tests/bedrock_imports.rs`, this plan.
**What:** After applying a fresh Bedrock world's creation profile, archive its initial live folder into the existing creation slot using the existing slot-update operation. Preserve slot identity, name, timestamp, and profile; return the updated slot. Registration can then verify the live world matches that archive instead of creating a second slot. This applies to fresh Bedrock creation on all platforms; Java creation and imported-world recovery branches are unchanged. No live user slots or worlds are removed.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.240 only.
**Commit:** `P12.240: archive fresh bedrock worlds into their initial slot`

**Evidence:** Cameron's Bedrock server has one live world folder and two same-name/same-timestamp slots. The original slot contains only its creation profile; the second, active slot has world.zip and detected world metadata. Fresh Bedrock creation applies its profile by writing level.dat, while registration's import reconciliation treats a live folder alongside an archive-less slot as a separate world and creates a new slot. Archiving into the original slot before registration closes that mismatch without weakening recovery for actual imported or mismatched world data.
**Checks:** Rust formatting and shipping-agent Clippy passed with existing unrelated Windows warnings. `cargo check -p msc-application --test bedrock_imports` compiled the focused regression target. The agent is rebuilt/staged for desktop Repair. No tests executed or live world/slot data changed.
**Essential coverage:** Extend the existing fresh Bedrock creation regression through the same import-reconciliation call used by server registration. Assert registration recognizes the original slot, only one slot remains, and the complete profile is unchanged. Uses the existing unique temporary directory with automatic cleanup and a tiny local world archive; no Bedrock process, network, port, or sleep assumptions. Expected runtime under 100 ms; execution deferred to Cameron.
**Manual acceptance:** Repair with the staged agent, create a new fresh Bedrock server, and confirm Worlds shows exactly one slot before initiation and after both initiation runs. Confirm its selected seed/gameplay settings and active identity remain intact. Existing duplicate entries are retained; this correction prevents their creation in new servers.

### P12.239 — Clear Windows Bedrock shutdown tracking after exit

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/bedrock_windows.rs`, `crates/msc-application/tests/bedrock_windows.rs`, this plan.
**What:** Clear graceful-stop timing and forced-stop tracking when Windows Bedrock exits. Restrict deadline escalation to the Stopping state and consume queued process exits before attempting forced termination. Continue observing the termination event when a force-stop is needed. This prevents status polling after pass one from raising a nonexistent-process error, cancelling Playit/Broadcast, and aborting initiation. Linux/macOS runtime sources are unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.239 only.
**Commit:** `P12.239: clear windows bedrock shutdown tracking after exit`

**Evidence:** The current Windows pass one stopped at 17:00:03, followed by Playit helper cancellation/stop at 17:00:24 and two Broadcast operations cancelled as Xbox Broadcast stopped. Windows poll_event checked its stale 20-second graceful-stop deadline before checking whether the process still existed. The error handler stopped both helpers and aborted the first-start coordinator on each later poll. P12.238 corrected setup display and click handling but did not address this runtime error. Linux source contains a similar timer pattern; investigation/fixes there are outside this Windows change and no Linux behavior has been modified.
**Checks:** Rust formatting and shipping-agent Clippy passed with existing unrelated Windows warnings. The focused Windows runtime regression target compiled with `cargo check -p msc-application --test bedrock_windows`; no tests executed. The agent is rebuilt/staged for desktop Repair. Live acceptance remains with Cameron.
**Essential coverage:** One fake-process/fake-clock regression exercises clean exit observed both before and after the shutdown deadline, then repeats stopped-state polling past the deadline. It asserts one clean termination, no subsequent error/events, and no forced termination. Existing coverage checks real deadline escalation but missed polling after clean exit. It reuses the existing ephemeral UDP-port setup; no real Bedrock, provider requests, or sleeps are used. Expected runtime under 10 ms; execution deferred to Cameron.
**Manual acceptance:** Repair the Windows service using the newly staged agent. Initiate Bedrock with Playit and Broadcast; leave the stopped connection stage open beyond 20 seconds, complete Playit, and click Xbox Set up. Confirm Microsoft sign-in appears and the helper remains running while authenticating. Complete sign-in, confirm pass two runs, then confirm the final Minecraft server is stopped. Existing server/configuration/credentials can be retained; no fresh-install reset is needed for this correction.

### P12.238 — Separate initiation setup from connection readiness

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, `clients/desktop-web/src/lib/sections/server-editor/BroadcastAuthSheet.svelte`, `crates/msc-application/src/xbox_broadcast.rs`, `crates/msc-application/tests/xbox_broadcast.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/src/routes/servers.rs`, this plan.
**What:** Call Broadcast setup from the actual click handler. Represent successfully saved Playit credentials/tunnels as Configured during the stopped setup stage, remove its credentials action, and explicitly start/check Playit again in pass two. Monitor its operation failures instead of leaving arbitrary failures dependent on prose matching. Prevent duplicate Broadcast starts, bound startup before sign-in, preserve a device code when closing the initiation prompt, and clear prompts on helper stop/failure/cancellation/timeout. Keep the initiation coordinator as prompt owner while its sheet is hidden. Offer Continue without Broadcast after a failure; the explicit action disables Broadcast in the saved server settings, stops its helper, and excludes it from pass two. Keep the existing stopped-server completion guard and Playit-only public-address summary.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.238 only.
**Commit:** `P12.238: repair initiation connection setup and recovery`

**Evidence:** Live Windows operations showed successful Playit account/tunnel provisioning alongside a cancelled helper start and successful helper stop. Status contained the saved key/address with isRunning false and no error note. The previous UI always reset successful setup to Waiting and required a running helper to leave it; Set up then reopened credentials. P12.232 also introduced a Broadcast click handler that referenced its function without calling it; P12.237 changed availability but retained that error. The exact caller of the historical helper stop remains unproven; configuration-stage success no longer depends on that helper remaining alive, and pass two explicitly starts/checks it again. No live sign-ins or account/provider changes were performed during implementation.
**Checks:** Svelte check passed with zero errors and eleven existing warnings; production frontend build passed. Rust formatting and shipping-agent Clippy passed with existing unrelated Windows warnings. The focused Broadcast test target compiled without execution. The agent is rebuilt/staged for desktop Repair. Checks establish compilation/build, not live Windows, macOS, or Linux acceptance.
**Essential coverage:** Extend the existing cancellation/watchdog test to assert discarded device codes, and add one fake-process regression covering stop and crash after a prompt. This protects the observed late sign-in sheet without networks, real Java, sleeps, or wall-clock assumptions; expected runtime under 10 ms. Test execution remains with Cameron.
**Manual acceptance:** Rebuild the desktop and Repair its agent. Initiate a fresh Bedrock server with Playit and Broadcast: after tunnel creation, confirm Configured and no repeated credentials action; click Xbox Set up and confirm Starting then the Microsoft code; close/reopen the code without another helper launch; authenticate, observe pass two, and confirm the final server is stopped with the Playit public address and friend name. Check a failed Broadcast launch offers Retry and Continue without Broadcast, and that choosing the latter disables it in settings, completes pass two, and leaves no late prompt. Existing saved Playit credentials must avoid another login. Check the same flow on macOS/Linux; these shared changes retain the platform runtime implementations.

### P12.237 — Make Xbox setup actionable after Playit readiness

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, this plan.
**What:** Preserve Xbox setup's existing Playit-attempted unlock condition and additionally allow the displayed Playit Ready state to unlock it. If the agent already has a Microsoft sign-in prompt, use the button to reopen it instead of disabling the button. Label that action Sign in, and explain when Playit setup still blocks the action. Preserve the existing Broadcast launch/sign-in-before-pass-two sequence. This shared frontend change applies to all platforms; it does not remove any previously allowed setup action on macOS/Linux.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.237 only.
**Commit:** `P12.237: unlock broadcast setup from playit readiness`

**Evidence:** Cameron reports Set up stays completely unchanged on click. Inspection found two independent disabling conditions: a separate Playit attempted flag and any existing Broadcast prompt. These could leave setup disabled while the sheet showed Playit Ready or while sign-in was available. The live agent currently reports no running Broadcast/prompt and the active server has no Broadcast working directory; this does not establish which condition applied to the earlier click. Both disabled-action paths are corrected without claiming a reproduced Microsoft sign-in.
**Checks:** Svelte check passed with zero errors and eleven existing warnings. No Rust changes or tests run.
**Manual acceptance:** Rebuild the desktop frontend and initiate a Windows server with both helpers. After Playit becomes Ready, confirm Xbox Set up is enabled and changes to Starting on click. Confirm the Microsoft sign-in sheet appears, closing it offers Sign in/Show code again, and reopening does not start a second helper. Authentication should continue the existing pass-two/shutdown sequence. Physical acceptance remains pending.

### P12.236 — Recognize executable files on the real Windows filesystem

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-infrastructure/src/fs.rs`, `crates/msc-infrastructure/tests/java_runtime_detection.rs`, this plan.
**What:** Replace the Windows filesystem's unconditional `executable: false` stub with a file-and-extension check for exe/com/bat/cmd. Preserve the exact Unix execute-permission calculation. This allows the discovery and normalization fixes from P12.233–P12.235 to accept the installed Java 25 executable.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.236 only.
**Commit:** `P12.236: recognize windows executable files during java discovery`

**Diagnosis:** The newly repaired running agent still returned only configured Java 21. Source inspection found that `StdFileSystem.stat` used a non-Unix placeholder always returning false for executable files. Runtime discovery requires both `is_file` and `executable`, so no real Windows JDK could pass even after correcting filenames/search roots. Earlier fake-filesystem coverage marked java.exe executable and missed this production boundary; environment-only fixes were incomplete.
**Checks:** Rust formatting, shipping-agent Clippy and compilation of the discovery regression target passed with existing unrelated warnings. No tests run. The agent was rebuilt/staged for desktop Repair; the running service is not replaced by building alone.
**Essential coverage:** Add one Windows-only real-filesystem regression with a temporary metadata-only java.exe, a text file, and a directory ending in .exe. Assert discovery and home normalization find the JDK, and reject non-executable files/directories. The fake executable is never launched; no Java install/network/timing wait is needed. A unique temporary folder is cleaned by an RAII guard even after an assertion failure. Expected runtime below 100 ms. Execution remains deferred to Cameron.
**Manual acceptance:** Load the freshly staged agent with Repair, reopen the Java picker, and confirm Local Temurin Java 25 is listed. The live CLI Java list should include its `bin/java.exe` path. No reinstall is required. macOS/Linux retain the same permission-bit behavior; physical acceptance remains with Cameron.

### P12.235 — Find Windows Java without an interactive Local AppData variable

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/routes/versions.rs`, this plan.
**What:** On Windows, include `HOME/AppData/Local/MSC2/runtimes` alongside the agent-managed root and optional LOCALAPPDATA root. The desktop service explicitly records the installing user's HOME; discovery must not rely solely on an interactive LOCALAPPDATA variable. macOS/Linux paths are unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.235 only.
**Commit:** `P12.235: locate windows java from the service user home`

**Evidence:** Java 25.0.4.1 executed successfully again from Cameron's Local runtime folder. The repaired service binary and rebuilt `target/debug/msc.exe` have matching SHA-256 hashes, but the live Java list still returns only Java 21. Service metadata from the preceding diagnosis records `HOME=C:/Users/Cameron`. P12.234 added the environment-dependent Local root but did not cover missing/different service LOCALAPPDATA. This correction derives the known installed root directly from HOME; the service process's live environment has not been inspected, so that cause remains an inference until owner acceptance.
**Checks:** Rust formatting and shipping-agent Clippy passed with existing unrelated Windows warnings. `npm --prefix clients/desktop-web run prepare:agent` rebuilt and staged the debug/package agent successfully. No tests, service restarts, or Java downloads run.
**Manual acceptance:** Repair using the freshly staged agent, reopen the Java picker, and confirm the already installed Local Java 25 appears. No Java reinstall is needed. Live acceptance remains pending until the service runs this revision.

### P12.234 — Discover Windows Java installed under the default data folder

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/routes/versions.rs`, this plan.
**What:** Include `%LOCALAPPDATA%/MSC2/runtimes` in Windows Java discovery as well as the current agent's managed runtime root. Preserve macOS/Linux search roots and leave existing installations in place.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.234 only.
**Commit:** `P12.234: discover windows java across agent data folders`

**Diagnosis:** The running service binary matched the rebuilt packaged agent by SHA-256. Its live `msc --json java list` returned only configured Java 21. Service metadata sets `MSC2_DATA_DIR` to `C:/Users/Cameron/AppData/Roaming/MSC2`, but the verified Temurin 25 executable resides under `C:/Users/Cameron/AppData/Local/MSC2/runtimes`. P12.233 fixed executable recognition but missed this separate root mismatch.
**Checks:** Rust formatting and `cargo clippy -p msc-agent --bin msc` passed with existing Windows warnings outside the change. Inspection confirms the additional root is guarded by `HostOs::Windows`; macOS/Linux discovery behavior is unchanged. No tests run.
**Manual acceptance:** Rebuild the packaged agent and Repair the Windows service, reopen the runtime picker, and confirm the existing Java 25 appears without another install. The CLI `msc --json java list` should include its Local `bin/java.exe` path. No service restart, credential changes, Java download, or tests performed by the agent in this step.

### P12.233 — Refresh newly installed Java in the Windows server picker

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, `crates/msc-infrastructure/src/java_runtime_detection.rs`, `crates/msc-agent/src/routes/versions.rs`, `crates/msc-infrastructure/tests/java_runtime_detection.rs`, this plan.
**What:** Refresh the builder's runtime list after installation. Discover Windows `bin/java.exe`, recognize backslash-separated executable/home paths, and return the installed executable rather than its directory on Windows. Preserve the existing macOS/Linux installer return behavior, macOS bundle inspection, and preferred `bin/java` discovery/normalization.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.233 only.
**Commit:** `P12.233: refresh installed java in the windows server picker`

**Checks:** Svelte check passed with zero errors and eleven existing warnings. Rust formatting and shipping-agent Clippy passed with existing Windows warnings outside this change. The focused discovery test target compiled with `cargo check -p msc-infrastructure --test java_runtime_detection`; no tests run.
**Essential coverage:** One in-memory filesystem regression covers discovery and normalization of a Java 25 `java.exe`, then confirms `bin/java` remains preferred when present. It protects the reported missing Windows runtime and the owner's Unix compatibility requirement; no real JDK, network, timing or host paths are required. Expected runtime below one millisecond; execution deferred to Cameron. Existing Unix/macOS discovery fixtures remain unchanged.
**Manual acceptance:** Rebuild the desktop and agent. On Windows, create a Paper server requiring Java 25, install Java from the runtime picker, return with Okay, and confirm Java 25 appears selected with a `bin/java.exe` path and can be used to continue. Check Detect also finds already installed managed Windows runtimes. On macOS/Linux, confirm the existing Java picker/install flow still works; physical platform acceptance remains pending.

### P12.232 — Coordinate Bedrock initiation connection setup and shutdown

**Status:** Implemented; awaiting Cameron's Windows verification.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, `crates/msc-agent/src/routes/networking.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, this plan.
**What:** Refresh Playit/Broadcast during the stopped connection-choice stage. Start Xbox Broadcast from its Set up action and finish its real setup operation/device sign-in before starting pass two; give failed setup a retry instead of treating every failure as a timeout. Hide the coordinator behind credential sheets and prevent late Broadcast prompts after completion. Feed Broadcast readiness to the agent's first-start coordinator outside the helper lock. Apply the existing safety limit to the Bedrock process pump. Confirm the server is stopped before displaying completion, and suppress port-forwarding addresses when Playit was selected. Require a true firstStartComplete result rather than the mere presence of that field.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.232 only.
**Commit:** `P12.232: coordinate initiation helper setup and shutdown`

**Checks:** Svelte check passed with zero errors and eleven existing warnings; Rust formatting passed. `cargo clippy -p msc-agent --bin msc` compiled successfully with existing Windows warnings outside the changed code. Strict all-target Clippy was blocked by the existing unused `BEDROCK_HELPER_SOCKET_MODE` constant; the broader non-strict run also found existing outdated `CommonArgs` fields in `tests/cli_service.rs`. Those unrelated files were left unchanged. No test suites or account/provider operations run, and no source-text assertion tests added.
**Manual acceptance:** On Windows, initiate a new Bedrock server with Playit and Xbox Broadcast enabled. Complete Playit and confirm its row updates without clicking Xbox setup. Click Xbox Set up and authenticate in the device-code sheet while the Minecraft server remains stopped. Confirm pass two begins only after authentication, completion waits for Minecraft shutdown, and the summary has the Playit endpoint and authenticated friend name without a port-forwarding endpoint. Close the result and confirm the server remains stopped and no late sign-in sheet appears. Also check a saved Playit key and an Xbox sign-in failure/retry. This account and process acceptance remains pending; local checks do not establish it.

### P12.231 — Remove repeated onboarding instructions

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `content/guides/onboarding.json`, `content/guides/onboarding-source-map.json`, `clients/desktop-web/src/lib/help/TourOverlay.svelte`, `clients/desktop-web/src/lib/help/SetupIntro.svelte`, this plan.
**What:** Remove repeated body/footer instructions and duplicate setup completion text. Fold the first-world introduction into Essentials. Suppress the world-review popup while retaining its Continue action listener. Give section-opening cards distinct titles, preserve their expansion actions, shorten add-on/create cards, and remove the repeated running-state claim from the later tour completion copy. Keep source mapping and order aligned with the remaining content.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.231 only.
**Commit:** `P12.231: remove repeated onboarding instructions`

**Checks:** Svelte check passed with zero errors and eleven existing warnings. No Rust changes or tests added/run.
**Manual acceptance:** Restart setup/tour. Confirm setup completion has one heading; each card gives its action once; Essentials follows connectivity directly; expansion and Okay buttons still advance through world options; the wizard's Continue advances from world settings without a review popup; add-on and creation cards dismiss correctly and creation finishes the tour.

### P12.230 — Hide scrollbars throughout first-time setup

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, this plan.
**What:** Hide scrollbars on every setup page and the nested Java runtime list, matching the outer first-launch window. Preserve scrolling so overflow content remains accessible.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.230 only.
**Commit:** `P12.230: hide scrollbars throughout first-time setup`

**Checks:** Svelte check passed with zero errors and eleven existing warnings. No Rust changes or tests run.
**Manual acceptance:** Open first-time setup and move through every page, including step 2 with Java and Bedrock selected. Confirm no scrollbars appear; confirm overflow content and long Java runtime lists remain reachable by scrolling.

> ## STATUS: Phase 16 step records P16.1–P16.34 and Phase 17 step records P17.1–P17.27 and all Phase 18 steps/substeps are Done at Cameron's direction and archived. [v0.1.18](https://github.com/ctemple9/msc2/releases/tag/v0.1.18) published all nine artifacts from `ededaf33632bbbdcc518ae8928a54bb3ba073cc6`. Outstanding physical acceptance, deferred checks and independent phase reviews remain separate from Done step status.
> **Next move:** Cameron records Phase 17 physical acceptance, then the other agent reviews its gate. Phase 16 still needs Cameron's exact-artifact results in `docs/msc2/release/phase16-acceptance.md` and an independent gate review. Phase 18 awaits independent review against its consolidated map acceptance record; named checks remain deferred. Done step statuses do not assert that pending gate evidence exists.

## How this document works

The vision documents describe where MSC 2 is going; the port plan defines the phase sequence and exit gates; this file records the active priorities and current phase state. Completed phase plans and historical step records live in `rolling-plan-archive.md`.

Each implementation step is planned, read, executed, verified by Cameron, reviewed against its phase gate, and then archived. A step's status records the completion state Cameron directs; phase acceptance evidence and independent review are tracked separately. Marking steps Done does not by itself close a phase gate.

## Current phase

| Phase | Name | State |
|---|---|---|
| Setup | Repo, docs, agent instructions, CI, editor config | complete |
| 0 | Freeze the baseline and build the harness | complete |
| 1 | Domain types and pure rules | complete |
| 2 | API contract and operation model | complete |
| 3 | Safety substrate | complete |
| 4 | Java lifecycle vertical slice | complete |
| 5 | Configuration and migration | complete |
| 6 | Worlds and backups | complete |
| 7 | Server families and provisioning | complete |
| 8 | Mods, plugins, modpacks | complete |
| 9 | Networking and helpers | complete |
| 10 | Bedrock runtimes | complete |
| 11 | Desktop and web clients | complete |
| 12 | Client redesign and post-phase corrections | complete |
| 13 | Full-screen terminal client | retired by D-034 |
| 14 | Operational refinements | complete |
| 15 | Maintenance follow-ups | complete |
| 16 | Release safety and codebase readiness | complete |
| 17 | Local CLI refinement | step records done; physical acceptance and independent review pending |
| 18 | Integrated 3D world map | all step records done by owner direction; named checks deferred, independent review pending |

## Active Phase 16 acceptance

The [v0.1.18 release](https://github.com/ctemple9/msc2/releases/tag/v0.1.18)
completed its build and publish workflow with all nine platform artifacts.
Cameron's exact-artifact physical observations and the independent Phase 16
review remain outstanding. See `docs/msc2/release/phase16-acceptance.md` for
the release identity, published asset metadata, and pending acceptance rows.

P16.1–P16.34 and the September 17 and September 28 audits are preserved in
`docs/msc2/rolling-plan-archive.md`. Phase 16 remains in progress until its
exit gate holds.

## Active Phase 17 acceptance

P17.1–P17.27 are Done at Cameron's direction. Their complete records are in
[the archive](rolling-plan-archive.md#phase-17--local-cli-refinement), including
[completion steps](rolling-plan-archive.md#phase-17-completion-steps).
Physical acceptance and independent gate review remain outstanding. The
local CLI contract and acceptance guidance remain in
[Phase 17 CLI](clients/phase17-cli.md) and the [port plan](msc2-port-plan.md).

## Active Phase 18 acceptance

All Phase 18 steps and corrective substeps, including P18.11, are Done at
Cameron's direction. Their complete records are in
[the archive](rolling-plan-archive.md#phase-18--3d-world-viewer).
The [map acceptance record](phase18-map-acceptance.md) preserves owner
observations, measured costs and limits. Nether absent-source troubleshooting,
ATM10 custom-dimension visual proof and further runtime/platform checks remain
deferred. Independent gate review remains outstanding. The map work is merged
into main; no release publication or exact-artifact acceptance is implied.

## Owner-requested Phase 12 follow-up

### P12.194 — Redesign agent home and remove signal/status dots

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/App.svelte`, shared status components and their callers, player-presence/chat/setup-progress surfaces, `clients/desktop-web/tests/agent-install/agent-install.test.ts`, `clients/desktop-web/tests/screens/first-launch-reset.test.ts`, `clients/desktop-web/tests/components/base.test.ts`, `clients/desktop-web/tests/archive/agent-home/`, `docs/msc2/antiAIslop.md`, this plan.
**What:** Implement Cameron's reviewed HTML home design inside the existing desktop shell. Show the current agent, its actual server list and a permanently visible explanation of the app/agent/Minecraft relationship. Missing local service offers Install; installed local service offers one Start/Stop agent button plus Repair. Those native actions await the existing connection refresh; remove separate reconnect/disconnect buttons. Service controls remain scoped to the selected local host; a remote host's service remains managed on that computer. The remote dropdown offers Connect new host / View saved hosts and populates the chosen content below, retaining SSH/tunnel review, route selection, secure credentials, editing, pairing replacement and removal confirmation. The root route opens agent home; server Overview and explicit deep links remain available. Apply Cameron's 2026-10-02 app-wide prohibition on signal/status dots and record it in the design law. Preserve state information as text. Archive obsolete exact-source assertions that required the replaced page's wording/layout; retain the controlled behavioral checks for automatic installation/startup. No tests added or run; Rust formatting/lint are not applicable because no Rust is changed.
**Verify:** `npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build`
**Batch:** P12.194 only.
**Commit:** `P12.194: redesign agent home and remove status dots`
**Owner visual verification:** Open agent home with a missing, stopped and running local service; confirm Install or the single Start/Stop button plus Repair, and automatic connection refresh after each action. Open both remote-dropdown choices, edit/switch a saved host, and open a server's Overview. Confirm signal/status dots are absent in setup, service status, player lists and chat. These interactions remain Cameron's verification; builds do not assert physical service or remote-host acceptance.

**Agent checks:** Frontend type-check and production build passed after restoring the checkout’s dependencies with `npm ci --ignore-scripts`; 10 existing type-check warnings remain outside this change. Targeted frontend formatting and `git diff --check` passed. Tests were not run. Physical app/service/remote verification remains pending.


## Owner-requested release

### P16.35 — Prepare and publish v0.1.19

**Status:** Prepared; publication requested by Cameron, exact-artifact verification pending.
**Files:** `Cargo.lock`, `crates/msc-agent/Cargo.toml`, client package manifests/lockfile, Tauri package manifests/lockfile/configuration, bundle identity (source and static asset) and its existing assertion, `tools/release/stage-windows-agent.ps1`, `README.md`, `docs/msc2/release/v0.1.19.md`, this plan.
**What:** Increment the latest release tag from v0.1.18 to v0.1.19, preserving the current main-branch agent-home redesign, Cameron's subsequent adjustment and merged world-map work. Keep agent, desktop, frontend and locked package versions consistent. Include the two terrain helper executables and their existing license in Windows desktop staging, matching the other desktop/headless packaging paths. Push main and the new immutable tag once to trigger the existing build-only beta workflow, preserving all nine artifacts, checksums and signed update metadata. No workflow gates or tests added. The pre-existing Tauri Cargo.lock dependency edit remains uncommitted; stage only the release-version change from that file.
**Verify:** `gh release view v0.1.19 --json tagName,isPrerelease,assets,url`
**Batch:** P16.35 only.
**Commit:** `P16.35: prepare v0.1.19 release`

**Release preparation checks:** Locked Cargo metadata resolved for the agent and desktop packages; all release version fields agree at 0.1.19. Frontend production build and Windows staging PowerShell syntax inspection passed. Existing workflow signing-key variable/secret names are configured. No tests were run.


### P12.195 — Remove local service details disclosure

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Remove the local “Service details & pairing another desktop” disclosure and its contents from the Agents screen at Cameron's request. Retain the local Install/Start/Stop/Repair controls and remote host/pairing workflows. Preserve Cameron's uncommitted heading edit without including it in this step's commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.195 only.
**Commit:** `P12.195: remove local service details disclosure`

**Agent checks:** Frontend type-check passed with zero errors and 10 existing warnings; `git diff --check` passed. Tests were not run.


### P12.196 — Explain closing MSC and stopping services

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Replace the connection/pairing explanation with the approved “What happens when you close MSC?” guidance: running servers continue after the app closes; use the server’s Stop button for Minecraft and Stop agent for the agent. Remove the duplicated closing-window note from the left panel and its unused styling. Preserve Cameron's uncommitted heading edit without including it in this step's commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.196 only.
**Commit:** `P12.196: clarify closing the app and stopping the agent`

### P12.197 — Collapse the agent server list and open rows directly

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/lib/sections/performance/PerformanceSection.svelte`, `clients/desktop-web/src/lib/sections/shared/server-uptime.ts`, `clients/desktop-web/src/App.svelte`, this plan.
**What:** Collapse the agent server list by default with a labeled disclosure and server count. Open Overview by clicking a keyboard-accessible server row with a quiet arrow. Place the selected server first and distinguish its running/stopped state using existing status colors without dots. Show server type/port and selected-server live players/RAM, refreshing only while the list is expanded and the screen is active. Share the existing Performance uptime semantics across tabs and the shell: count from an observed stopped-to-running transition; show Running when the start time is unknown. Forget host observations when the connection is reset, reject late list responses after host/server changes, and never present the selected server's stats on other rows. Preserve Cameron's uncommitted heading and Tauri lockfile edits outside this commit. No Rust changed; no tests added or run.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.197 only.
**Commit:** `P12.197: make agent server rows collapsible and show live stats`

**Manual verification:** Expand On this agent; confirm the selected server is first, click another row to open its Overview, and compare Players/RAM with that server's existing screens. Start a previously stopped server while connected and compare uptime across Agents and Performance. Reconnecting to an already-running server should show Running rather than inventing elapsed time.

### P12.198 — Match the selected server row to the rest of the list

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Remove the selected row's permanent gray background. Place its extra stats beside the existing type/port details rather than adding a third line, so rows share the same spacing and height when room permits. Allow details to wrap on narrow screens. Retain selected-first ordering, colored status, and direct Overview navigation. Preserve Cameron's uncommitted heading and lockfile edits outside this commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.198 only.
**Commit:** `P12.198: match selected server row styling to the list`

### P12.199 — Update managed Geyser helpers and surface load failures

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-infrastructure/src/geyser.rs`, `crates/msc-application/src/geyser.rs`, `crates/msc-application/src/addon_updates.rs`, `crates/msc-application/tests/addon_updates.rs`, `crates/msc-agent/src/routes/components.rs`, `crates/msc-domain/src/crash_analysis.rs`, `crates/msc-domain/tests/paper_plugin_crash_analysis.rs`, `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`, `clients/desktop-web/src/lib/sections/components/model.ts`, `clients/desktop-web/src/lib/sections/server-editor/StartupFailureSheet.svelte`, `clients/desktop-web/src/lib/sections/server-editor/StartupFailurePanel.svelte`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, this plan.
**What:** In the installed-plugin action menu, replace View with Update for Geyser and Floodgate. Keep both helpers out of Modrinth update checks, including when a stale project link exists. On request, resolve GeyserMC's latest Spigot build, report when the installed version/build is current, or checksum-verify and atomically replace it while preserving the existing JAR on failure. Report the resulting version/build and that a restart is needed. Detect the CraftItemStack reflection error from Geyser's Paper startup output, record a plain-language Paper API incompatibility diagnosis with the relevant log evidence, and open the existing startup issue sheet when that helper fails even if Paper reaches ready. Add one controlled regression test for the reported Geyser failure signature; strengthen existing update-resolution coverage for stale Modrinth links. Do not run tests.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-domain -p msc-application -- -D warnings && cargo clippy -p msc-agent -- -D warnings -A dead_code && cargo check -p msc-application --test addon_updates && cargo check -p msc-domain --test paper_plugin_crash_analysis && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build && npm --prefix clients/desktop-web run api:check`
**Batch:** P12.199 only.
**Commit:** `P12.199: add managed geyser updates and startup diagnosis`

**Essential test rationale:** The new single regression case protects the user-reported Paper/Geyser failure from being reduced to a generic plugin load error and verifies that the diagnosis retains the CraftItemStack cause. Existing tests cover other Paper plugin failures and Geyser's separate minimum-Minecraft-version message, not this Paper API signature. The case uses two fixed console lines and one in-memory plugin entry, with no network or timing assumptions. Expected runtime: under one second. It is added but not run.

**Agent checks:** Rust formatting passed. Strict Clippy passed for `msc-domain` and `msc-application`; the `msc-agent` Clippy check passed with its pre-existing `dead_code` lint allowed because `auth::forbidden` is unused elsewhere. Both affected integration-test targets compiled with `cargo check --test`; no tests were run. Svelte check passed with zero errors and 11 existing warnings; production frontend build and API generation passed. Manual desktop verification remains pending.

## Phase 19 — Complete local uninstall (owner-requested 2026-10-02)

**Planning state:** Cameron authorized implementation on 2026-10-02. P19.1 is implemented pending owner verification; later steps remain planned.

**Owner-approved intent:** Add **Uninstall MSC 2…** beside Reset in app settings and a local `msc uninstall --danger` command. Permanently remove this computer's MSC 2 agent services, managed servers/worlds/backups, MSC-owned helpers and runtimes, host/client data and credentials, caches/logs, installed command, and desktop app. Never contact or uninstall a saved remote agent. Running this flow on a remote computer means running its local MSC desktop or CLI there. Preserve MSC 1, source checkouts, separately installed Java/Tailscale/Docker, unrelated files, and OS-owned package caches.

**Confirmation contract:** Both interfaces must show the actual computer, server root(s), installation(s), and exact deletion list before execution. Desktop: review sheet, acknowledgement of permanent world/backup loss, exact typed `UNINSTALL MSC 2`, then a final destructive confirmation dialog. CLI: `--danger` enables the destructive flow but does not bypass review; print the same inventory and require the exact phrase interactively. `--confirm "UNINSTALL MSC 2"` is the explicit non-interactive equivalent, used only after a separate `--dry-run` inventory review; refuse redirected input without it. `--dry-run` performs no writes, elevation, service changes, or cleanup. A native command must enforce the confirmation independently of UI state. Local cleanup must still work while the selected desktop host is remote and while the local agent is missing/offline; no HTTP uninstall route.

**Downloaded installers:** MSC cannot prove ownership of every renamed/moved installer or its original download location. Include verified MSC release installers in known download/update locations and let the operator explicitly select additional installers. Show every selected file before confirmation, validate its package/bundle identity or signed release checksum, and remove only those files. No filename-only recursive disk search. Never promise that an unknown original DMG has been found. A mounted disk image needs explicit unmount handling; if another application holds it open, report the remaining file instead of claiming full removal. Do not delete MSI/OS package-manager caches directly.

**Completion contract:** Gracefully stop Minecraft and MSC-managed helpers before removing services/data. If shutdown or service removal fails, stop and report what remains. Remove the app through its OS installation mechanism: verified macOS bundle removal, Windows registered MSI uninstall, Linux owning package removal (or a verified standalone AppImage). Use a narrowly scoped detached continuation where the running app/command cannot remove itself. The continuation must validate its inventory again, propagate failures, and leave a readable result outside the deleted MSC trees; let the owner choose whether to retain that report. “Scheduled” is not “Uninstalled.” Reject unsupported/dev installations rather than deleting a source checkout. Clean up continuation files when finished.

### P19.1 — Inventory local installations and define the deletion boundary

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-infrastructure/src/uninstall.rs` (new), `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/tests/uninstall.rs` (new only for essential boundary cases), `docs/msc2/clients/local-uninstall.md` (new), this plan.
**What:** Build one serializable local inventory for CLI and desktop, with canonical paths, ownership evidence, missing/unavailable states, and exclusions. Discover service definitions and their actual MSC2_DATA_DIR/MSC2_APP_CONFIG_PATH/MSC2_AGENT_SERVERS_ROOT overrides rather than guessing from the current shell. Cover desktop and headless data layouts (macOS MSC 2 vs MSC2; Windows roaming/local data; Linux desktop and system headless roots), registered server roots and external managed server/backups with explicit ownership boundaries, client WebView state, credential stores, helper installs, old marked headless versions, command links/PATH entries, application/package identity, and verified installer files. Inventory is read-only; corrupt configuration or ambiguous ownership blocks affected deletion and is visible. Reject root/home/shared-parent targets, MSC 1 paths, symlink escapes, traversal, and unsupported developer checkout removal. Record essential controlled tests for path escapes, ambiguous ownership, and remote exclusion; do not run them without a specific owner instruction. Avoid brittle timing or real-machine installation fixtures.
**Verify:** `cargo check -p msc-infrastructure`
**Batch:** P19.1 only.
**Commit:** `P19.1: inventory local msc installations for uninstall`

**Implementation boundary:** The shared layer consumes local OS service/package inspection from platform adapters; P19.2 wires those native inspections and execution. Missing service/package inspection produces a blocking entry, never an assumption that an installation is absent. No CLI command or Settings uninstall action is available in this step. The preview function itself has no network, elevation, process control, or filesystem mutation.

**Essential tests:** Added six controlled in-memory deletion-boundary cases for protected/symlink/source targets, corrupt-config parent removal, ambiguous custom data roots, service override/secret separation, unverified/changed installers, and preview mutation. These protect permanent data loss or secret disclosure rather than structure/prose. Expected runtime is under one second after compilation. No tests were run.

**Agent checks:** Package type-check, compilation of the focused test target without execution, formatting, and package-library Clippy with warnings denied passed. No Rust test executables were run. Owner verification remains open.

### P19.2 — Remove services, data, credentials, and OS installation locally

**Status:** Implemented; awaiting Cameron's verification. Cameron authorized completion of P19.2–P19.4 together on 2026-10-02.
**Files:** `crates/msc-infrastructure/src/uninstall.rs`, platform-specific uninstall modules under `crates/msc-platform-macos/src/`, `crates/msc-platform-windows/src/`, `crates/msc-platform-linux/src/`, existing service/secret-store adapters where necessary, essential controlled boundary tests if current coverage misses a concrete risk, `docs/msc2/clients/local-uninstall.md`, this plan.
**What:** Execute the inventory through existing platform privilege boundaries, with no remote service API. Authenticate to the local agent if available, request graceful server/helper shutdown and verify it; handle an offline/stopped installation through its inspected service definition without guessing process ownership. Stop/unregister all verified MSC-owned service/helper definitions, remove approved data and credential records, remove verified links/PATH entries, uninstall the desktop/package and marked headless artifacts, then delete the approved installer files. Distinguish Windows MSI uninstall from raw file deletion; use Linux package ownership and macOS bundle identifier checks. Secure detached continuation state against tampering; revalidate filesystem boundaries/ownership immediately before deletion. Never execute a user-writable elevated cleanup script blindly. Provide partial-failure/result reporting and retry inventory for leftovers, preserving failures rather than suppressing them. Do not launch the real uninstaller while implementing or verifying this step.
**Verify:** `cargo check -p msc-platform-macos -p msc-platform-windows -p msc-platform-linux`
**Batch:** P19.2 only.
**Commit:** `P19.2: implement complete local uninstall execution`

**Agent checks:** Host-platform checks and package-library Clippy passed without running tests or uninstall. Linux cleanup code also compiles on Unix for inspection. A Windows-target check was attempted but blocked by missing Windows C headers in ring (assert.h); Windows native compilation/acceptance remains required on Windows. The copied worker and CLI handoff are wired in P19.3; native execution is not invoked here.

### P19.3 — Expose the confirmed uninstall command

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/uninstall.rs` (new), CLI documentation/help, `docs/msc2/clients/local-uninstall.md`, this plan.
**What:** Add `msc uninstall --danger`, `--dry-run`, exact `--confirm` for explicit automation, and validated additional-installer selection. Share inventory/execution with desktop rather than duplicating deletion logic. Ignore/refuse remote target overrides and obtain all destructive targets locally. Print warnings and inventory before the typed interactive confirmation. Return nonzero on blocked/partial cleanup and distinguish a detached scheduled action from actual completion. Keep ordinary `msc service uninstall` and host-reset behavior unchanged. Review CLI parsing/help through non-destructive checks; do not run a destructive invocation on the developer machine.
**Verify:** `cargo check -p msc-agent`
**Batch:** P19.3 only.
**Commit:** `P19.3: add confirmed local uninstall command`

**Agent checks:** Agent compilation and Clippy passed (one pre-existing auth.rs dead-code warning); no tests or uninstall commands ran. A private copied worker waits for the parent/desktop to exit, rediscovers and compares the inventory, authenticates only to the local agent to stop Minecraft, and records partial/failure outcomes. Explicit optional reports survive outside the deleted directories; otherwise successful worker files are removed. Native Windows acceptance remains pending as noted in P19.2.

### P19.4 — Add Uninstall MSC 2 to app settings

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`, `clients/desktop-web/src/lib/sections/app-settings/UninstallSheet.svelte` (new), `clients/desktop-web/src/App.svelte`, platform adapter types/implementation, `clients/desktop-web/src-tauri/src/lib.rs` and a native uninstall module if needed, `docs/msc2/clients/local-uninstall.md`, this plan.
**What:** Read antiAIslop.md before frontend work. Add a separate destructive Uninstall action beside Reset. Show native local inventory independent of selected remote host; support verified additional-installer selection. Require the loss acknowledgement, typed phrase, and final dialog before invoking native execution. Enforce typed confirmation and inventory identity natively, disable duplicate submissions, and show OS elevation and failures clearly. Clear all local saved remote credentials/connections without contacting those agents. Exit only after native handoff is established; communicate scheduled continuation and its result location accurately. Existing Reset remains unchanged. Preserve unrelated owner edits and commit only this step's work.
**Verify:** `npm --prefix clients/desktop-web run check && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Batch:** P19.4 only.
**Commit:** `P19.4: add confirmed complete uninstall to settings`

**Agent checks:** Svelte check passed with only ten pre-existing warnings; desktop native compilation passed. No tests or destructive uninstall ran. Settings uses only the packaged local command, previews verified targets, enforces the loss acknowledgement/typed phrase/final dialog, and closes after private worker handoff. Reports default to the home folder. Installer discovery is bounded by retained signed metadata, and Windows native acceptance remains pending. JSON CLI worker output is detached so the scheduled response stays parseable; Linux elevated tools use fixed absolute paths; marked Windows archives remove their exact User/Machine PATH entry. Agent and desktop Clippy completed with only existing warnings.

**Phase 19 acceptance gate:** Cameron verifies both entry points on disposable installed MSC 2 environments for macOS, Windows MSI, and Linux desktop/headless packaging. Observe server shutdown, service/helper removal, data/credential/cache cleanup, OS package deregistration, self-removal, verified installer deletion, and readable partial-failure results. Check cancellation at each confirmation, no-write dry run, stale/tampered inventory rejection, missing/offline agent behavior, protected symlink/root paths, and that saved remote agents plus MSC 1 remain unchanged. Developer source trees are never used for destructive acceptance. The other agent independently reviews the deletion boundary and phase gate. No release workflow gates or release runs are added by this work.

### P12.199 — Show remote host actions directly

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Replace Connect remote agent and its dropdown with two visible buttons, Connect new host and View saved hosts (including the saved count). Retain the existing form/list underneath. Remove unused dropdown state, focus/Escape handling, and styles. Preserve owner heading and lockfile edits outside this commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.199 only.
**Commit:** `P12.199: show remote host actions as separate buttons`

### P12.200 — Update server-computer instructions for the Phase 17 CLI

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, this plan.
**What:** Rename Remote service commands to Commands on your server computer. Replace Linux service-name discovery and lower-level service commands with msc status/start/stop/enable/disable agent. Explain local terminal or SSH use as the installing account, support across all three platforms while the agent is stopped, separate Minecraft server controls, and the distinction between stopping and disabling boot startup. Remove the unsupported repair wording; preserve desktop pairing. Keep owner heading and lockfile edits outside this commit. No tests added or run; no Rust changed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.200 only.
**Commit:** `P12.200: update agent instructions for the local cli`


## Phase 6 import correction — owner-requested 2026-10-02

### P6.52 — Accept external world archive folder layouts

**Status:** Implemented; awaiting Cameron's verification. Cameron explicitly authorized implementation in this conversation.
**Files:** `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/tests/world_archive.rs`, `crates/msc-application/src/worlds.rs`, `clients/desktop-web/src/lib/sections/worlds/ImportWorldZipSheet.svelte`, this plan.
**What:** Recognize Bedrock world files at the archive root, in a named folder, or inside enclosing folders; retain existing MSC layouts. Accept `.mcworld` in the Worlds import picker. Normalize only the new slot archive, removing known macOS packaging metadata while preserving file contents, compression and permissions. Store the detected single Bedrock world folder name so activation opens that imported world. Also accept loose Java worlds and enclosing folders around Java worlds with their sibling dimensions. Keep generic source safety/CRC/size checks and strict final world-layout checks; reject unrelated files and ambiguous external multi-world bundles with an explanatory error. Existing MSC-format multiple Bedrock worlds remain supported. Original archives are untouched; failed normalization removes the partial archive/slot. MSC 1's `WorldSlotManager.createSlotFromZIP` copies ZIPs unchanged without structural checks; this correction preserves P16.4's stronger safety boundary while broadening input packaging. No API contract or release workflow changed.
**Verify:** `cargo test -p msc-infrastructure --test world_archive world_import_`
**Batch:** P6.52 only.
**Commit:** `P6.52: normalize external world archive layouts on import`

**Essential test rationale:** Three focused cases protect previously rejected external layouts, correct Bedrock folder identity, original archive preservation, retention of Java dimensions, and refusal of traversal, links, executable permissions, server configuration, unrelated entries and ambiguous bundles. Existing strict-layout tests do not exercise normalization. All inputs are tiny local ZIPs with controlled contents and independent temporary directories; no network, Minecraft runtime or timing assumptions. Expected combined runtime: under one second. Tests were added and compiled with Clippy but not run.

**Agent checks:** Rust formatting and Clippy for the affected libraries and archive test target; Svelte static check (zero errors, ten existing warnings). No test commands run.

**Manual acceptance:** Rebuild/restart the local app and agent, then import `/Users/camerontemple/msc2-servers/XqKXS4++O7k=.zip` unchanged through the Bedrock Worlds tab. With the server stopped, activate the imported slot and confirm the existing terrain loads. Also import a `.mcworld` file without renaming it. External bundles containing several worlds require individual imports; a selection screen remains a separate UI/API change.


### P6.53 — Let API imports reach external archive normalization

**Status:** Implemented; awaiting Cameron's verification. Follow-up to Cameron's report that the unchanged ZIP still receives the original error after P6.52.
**Files:** `crates/msc-agent/src/routes/worlds/import_activation.rs`, `crates/msc-agent/src/routes/worlds.rs`, this plan.
**What:** Remove the import route's premature activation-layout check. All desktop and CLI imports now reach P6.52's source safety checks, normalization, and strict final layout validation. Preserve invalid-archive HTTP 400 responses and consumed staging cleanup; failures are recorded on the exclusive import operation. Diagnosis confirmed that the running local agent contains the new normalization code, so the repeated rejection was a missed route-level check, not an outdated binary. Preserve activation/restore validation unchanged.
**Verify:** `cargo test -p msc-agent --bin msc world_backup_routes_staged_upload_import_round_trip`
**Batch:** P6.53 only.
**Commit:** `P6.53: normalize world imports before checking stored layout`

**Essential coverage:** Strengthen the existing staged-upload/import round-trip with a loose `level.dat` ZIP and assert that the stored archive contains `world/level.dat`. This directly catches a route precheck preventing normalization, the gap missed by P6.52's library cases. Reuses the existing controlled local ZIP and fake process/journal setup; no additional test count, network, live Minecraft or timing assumptions. Expected runtime remains under one second. Test execution is deferred.

**Agent checks:** Rust formatting and production agent Clippy passed (one existing unused `auth::forbidden` warning). Broader test-target compilation was blocked by pre-existing `crates/msc-agent/tests/cli_service.rs` references to removed `CommonArgs` fields (`base_url`, `host`, `port`, `token`); no tests ran and that unrelated file was not changed. Rebuild/restart the development app/agent and retry the original Bedrock ZIP unchanged for manual acceptance.


### P6.54 — Preserve directory types when normalizing ZIPs

**Status:** Implemented; awaiting Cameron's verification. Follow-up to Cameron's `unsafe archive entry: worlds/XqKXS4++O7k=/db/` import failure.
**Files:** `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/tests/world_archive.rs`, this plan.
**What:** Write normalized directory markers with ZIP `add_directory`, retaining their permission bits. Raw copying remains only for file contents. The ZIP library's `raw_copy_file_rename` reconstructs options via `unix_permissions`, which strips entry type bits; directory markers therefore became regular-file entries and failed strict validation. Inspect original entry modes before copying to reject executables and non-regular types rather than let option reconstruction disguise them. Source archives and strict activation validation remain unchanged.
**Verify:** `cargo test -p msc-infrastructure --test world_archive world_import_`
**Batch:** P6.54 only.
**Commit:** `P6.54: preserve directory entry types during world import`

**Essential coverage:** Extend the existing Bedrock packaging case with an explicit `db/` directory marker (the actual failed entry) for every supported layout, and assert the stored directory type and original 0700 permissions. No new test count; existing tiny local fixtures, expected combined runtime under one second. No tests run.

**Agent checks:** Rust formatting and Clippy for the affected libraries and archive test target. Manual acceptance requires rebuilding/restarting the agent and importing the unchanged original ZIP.


### P6.55 — Show real world activation progress and status age

**Status:** Implemented; awaiting Cameron's verification. Cameron explicitly requested progress reporting after successfully importing the original Bedrock ZIP.
**Files:** `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/tests/world_archive.rs`, `crates/msc-application/src/backups.rs`, `crates/msc-application/src/worlds.rs`, `crates/msc-application/src/worlds/activation.rs`, `crates/msc-application/tests/world_activation.rs`, `crates/msc-agent/src/routes/worlds/import_activation.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Report activation stages through the existing operation DTO: checking imported world, creating/checking safety backup, saving the outgoing slot, checking archive before extraction, extracting imported world, installing world, and applying settings. Count actual verified/copied bytes; measure compression source totals only for progress-aware archive creation. Show a flat inline Worlds progress panel with the stage, per-stage bar/percentage and byte counts when a total is known, elapsed time, and time since the stage/count last changed. Connection/read failures explicitly say status is unavailable and retry reads without reissuing activation or declaring it failed. Stop monitoring on destruction or host/server changes and ignore late responses. Existing ordinary backup/archive entry points remain available. Progress callbacks preserve source safety/CRC checks, cancellation boundaries, world-swap ordering and recovery behavior. Throttle agent publication to four updates per second, plus stage starts/completions, to bound journal writes. No API contract or release workflow changed. Read the anti-slop design law; the panel uses existing neutral tokens, one flat group, a functional bar, and text for state.
**Verify:** `cargo test -p msc-infrastructure --test world_archive world_activation_archive_progress_counts_real_bytes`
**Batch:** P6.55 only.
**Commit:** `P6.55: report world activation stages and byte progress`

**Essential coverage:** One new small local archive round-trip checks measured compression/verification/extraction totals, intermediate byte counts and monotonic counts, plus preserved extracted contents. Existing archive safety and cancellation coverage exercises the shared primitives. Extend the existing activation/profile case to ensure stage reporting accompanies the unchanged resulting settings. Both use controlled local files, with no network, live Minecraft, sleeps, or timing assertions; expected combined runtime under one second. Tests were compiled by Clippy but not run.

**Agent checks:** Rust formatting; Clippy for the affected libraries, archive/activation test targets, and production agent (one existing unused `auth::forbidden` warning); Svelte static check (zero errors, ten existing warnings). No tests or live world mutations run.

**Manual acceptance:** Rebuild/restart the development app and agent. Activate a sizable saved world while the server is stopped; verify the checking/backup/save/extraction stages and real byte bar, followed by success. During a long stage, observe elapsed time and last-progress age. A status read/connection failure must show retrying and last-agent-contact age, then resume tracking the same operation after recovery. Switching tabs retains tracking; switching host/server or destroying the view must stop its timer/polling and prevent late old-operation notices. Percentages describe the current named stage and can reset at the next stage; periods with no countable byte work show the stage/time rather than a simulated percentage. Progress age reveals inactivity but does not by itself prove a stall.


### P12.201 — Open Modrinth project links in the default browser

**Status:** Implemented; awaiting Cameron's verification. Cameron reported that View on Modrinth does nothing in the Paper datapack project sheet.
**Files:** `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte`, this plan.
**What:** Route the shared Modrinth project sheet's external anchors through the existing platform `openExternal` function, which invokes Tauri's validated OS browser opener. Ordinary `target="_blank"` anchors did not invoke that desktop command. Apply the same handling to About-description links and Source/Issues/Wiki/Discord anchors in this sheet. Preserve link destinations and styling; show browser-opening errors inline instead of silently failing. The shared sheet covers datapacks, mods and plugins. No native code, URL policy, API contract or release workflow changes; no new tests needed for this small wiring correction and no tests run.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.201 only.
**Commit:** `P12.201: open project detail links in the default browser`

**Manual acceptance:** In the Paper world's datapack browser, open Terratonic and click View on Modrinth; confirm the default browser opens the project page. Check the About wiki link and a project's Source/Issues/Wiki links through the same shared sheet. The existing desktop opener reports failed/unsupported URL launches inline. Similar plain anchors in the separate CurseForge Bedrock pack sheet are outside this step.


### P12.202 — Install datapacks with overlays and remember installed versions

**Status:** Implemented; awaiting Cameron's verification. Cameron clarified that the Terratonic install actually failed, and also requested recognition of previously installed datapacks.
**Files:** `crates/msc-application/src/addons.rs`, `crates/msc-application/tests/addons.rs`, `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, this plan.
**What:** Select the outermost `pack.mcmeta` rather than the first member in ZIP order. Inspection of the actual Modrinth Terratonic 3.0.27 download (version cT2AsHrJ) showed seven metadata members: six overlay copies before the root copy; the old installer selected an overlay and rejected legitimate sibling files. Preserve all overlay contents and keep the common-root, path, symlink, size, compatibility and checksum checks. Read the selected world's saved Modrinth datapack records when opening the browser, pass their version IDs into project details, and label successful/pre-existing recorded installations Installed (staging remains Added). Record successful new versions immediately. Shared project details no longer mark a failed/cancelled background install as installed. The browser-link correction remains its own P12.201 commit. Manual/imported packs without saved provider/version identity cannot be matched to a Modrinth release by these labels; no identity is guessed. No live install performed and no release workflow changes.
**Verify:** `cargo test -p msc-application --test addons java_datapack_install_uses_outer_metadata_and_preserves_overlays`
**Batch:** P12.202 only.
**Commit:** `P12.202: install overlay datapacks and restore installed labels`

**Essential test rationale:** One focused regression case reproduces overlay metadata preceding main metadata, at archive root and inside an enclosing folder; checks preserved overlays/main metadata, prior-world backup, and refusal of ambiguous separate packs without changing the world. Existing addon tests did not cover datapack metadata selection. Uses tiny local ZIPs and a unique directory with automatic cleanup, no network, live Minecraft, sleeps or timing assertions. Expected runtime under one second. Test compiled but not run.

**Manual acceptance:** Rebuild/restart the app and agent; on a stopped Paper server, install Terratonic 3.0.27 from the datapack sheet and confirm success/Installed. Close and reopen the browser and project details; the same saved Modrinth version should remain Installed. Installation errors must remain errors rather than create Installed labels. World-generation effects and external datapack dependencies remain Minecraft/pack behavior, not proved by installation alone.


### P12.203 — Limit world datapack choices to datapack releases

**Status:** Implemented; awaiting Cameron's verification. Cameron reported Fabric/NeoForge mod releases labeled Compatible in the Paper datapack browser and requested a correction.
**Files:** `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-application/src/addons.rs`, `crates/msc-application/tests/addons.rs`, this plan.
**What:** Require the provider's datapack loader/type marker in the world datapack detail sheet before collapsing/filtering versions or computing the compatible-version summary. Exclude mod builds even if they match the server Minecraft version; do not use the general add-on browser's fallback to show them when no datapack remains. Base Stable-only fallback on datapack releases. Automatic search-result installation selects a matching datapack release, not the first same-Minecraft mod release. Reject non-datapack releases before download in the agent and before mutation in the application installer. Datapacks for other Minecraft versions remain visible with Other version; the existing installer still refuses versions that do not list the server version. Shared mod/plugin browsing retains its existing behavior. No API contract or release workflow changes.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.203 only.
**Commit:** `P12.203: exclude mod builds from world datapack installs`

**Essential coverage:** Extend the existing overlay-install regression with Fabric and NeoForge releases advertising the same Minecraft version and containing otherwise valid datapack metadata. Assert refusal and unchanged world bytes. Reuse controlled tiny local ZIPs, no additional test count or runtime assumptions; expected under one second. Tests compiled but not run.

**Manual acceptance:** Rebuild/restart app and agent. Browse the mixed Tectonic project from the Paper world's datapack browser: Fabric/NeoForge builds must be absent; only datapack builds can receive the Compatible/Other version badge and install controls. A mod-only release list must remain empty rather than fall back to incompatible builds. Install from the search result must select a matching datapack release. Compatible here means the provider lists the server Minecraft version, not proof of runtime or dependency behavior.


**Agent checks for P12.203:** Rust formatting, application regression test compilation with Clippy, and production agent Clippy passed (one existing unused `auth::forbidden` warning). Svelte static check passed with zero errors and ten existing warnings. No tests or live installs run.

### P12.204 — Manage installed world datapacks

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-application/src/addons.rs`, `crates/msc-application/tests/addons.rs`, this plan.
**What:** Clicking an installed Java datapack opens its status and Delete action, with confirmation. View datapack opens its saved Modrinth project inside MSC, including Installed labels; packs without a recorded catalog identity explain why no page is available. Extend existing pack management to delete Java datapack files, retain safety backup and profile rollback, and refresh/reapply stopped active worlds using their actual edition and configured level name. Java enable/disable is not offered because it requires a separate Minecraft activation-list change. Keep existing Bedrock controls.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.204 only.
**Commit:** `P12.204: add installed datapack deletion and project navigation`

**Essential coverage:** Extend the existing small local overlay fixture to delete all installed files while preserving level.dat, verify backup bytes, and refuse unsupported Java disable without mutation. No network or timing assumptions; expected under one second. Compiled with Clippy, not run.
**Manual acceptance:** Rebuild/restart app and agent. Click an installed datapack on a stopped Paper world. View datapack must open its in-app project page if a saved Modrinth source exists. Delete requires confirmation, removes the row and its files from the selected world, and stays removed after reopening and activating the world. Cancel leaves it installed. Active running servers refuse deletion.

### P12.205 — Match datapack actions to Components menus

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Replace the datapack action sheet with the same shared popup Menu used by Components. Show View and destructive Uninstall only, with matching selected-row styling and a chevron. View opens the saved in-app Modrinth project; disable View when no supported catalog identity exists. Uninstall switches the row to inline Uninstall?/Cancel/Uninstall confirmation, matching Components. Keep stopped-server protection and existing removal/backup behavior. Anchor keyboard-triggered menus to the row.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.205 only.
**Commit:** `P12.205: match datapack actions to components menus`

**Manual acceptance:** Click a datapack row: the popup must look like Components with View and Uninstall. View opens its in-app catalog page where available. Uninstall shows inline confirmation; Cancel preserves the pack, confirmed Uninstall removes it. Clicking away or Escape closes the popup. Svelte check passed with zero errors and ten existing warnings; no tests added or run.

### P12.206 — Check Chunker updates and quiet conversion guidance

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldConversionWizard.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-infrastructure/src/chunker.rs`, `docs/msc2/worlds/phase6-api.md`, this plan.
**What:** Remove the orange side rail and inset from conversion guidance. Add an explicit Check for Chunker updates action on preflight when installed. Report the official latest release, up-to-date status, or lookup failure; offer Update Chunker when the release differs or installed provenance is unknown. Share official release/JAR selection validation between metadata lookup and acquisition. New permission-checked/audited GET reads metadata only on a blocking worker. Updating remains user-selected via existing operation/progress/download validation; reload supported formats and installed version on success. Preserve ready conversion controls if an update attempt fails. No automatic update, tests, or release workflow changes.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.206 only.
**Commit:** `P12.206: add optional chunker update checks`

**Manual acceptance:** Rebuild/restart app and agent. Convert World preflight has plain conversion guidance without an orange rail. Check for updates displays latest/current status; lookup failure does not prevent conversion with the installed version. Choose Update Chunker if offered, observe acquisition progress, then confirm installed version/formats refresh. No converter update occurs from checking alone. Svelte check passed with zero errors and ten existing warnings; Rust formatting and production Clippy passed with the existing unused auth helper warning. No tests or live downloads run.

### P12.207 — Remove decorative Manage Servers icons

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/fleet/ManageSheet.svelte`, this plan.
**What:** Remove the decorative server glyph and its boxed surface from each Manage Servers row, including unused styling. Server names and paths now begin at the existing row inset; preserve badges, activation and menu actions.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.207 only.
**Commit:** `P12.207: remove decorative server row icons`

**Manual acceptance:** Open Manage Servers: every row starts with the server name, without a left icon or reserved icon gap. No tests added or run.

### P12.208 — Open server actions from the whole row

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/fleet/ManageSheet.svelte`, this plan.
**What:** Replace per-server Set Active and three-dot buttons with a full-row button and trailing chevron. Use the existing shared Menu, Components selection treatment, and Set Active/Edit/Remove labels. Keep removal confirmation and editor behavior; disable Set Active for the already active server or missing control permission. Keyboard activation anchors the menu to the row.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.208 only.
**Commit:** `P12.208: open server actions from row clicks`

**Manual acceptance:** Manage Servers has a chevron instead of action buttons on each row. Clicking or keyboard-activating the row opens the Components-style three-action menu; Set Active updates the selected server, Edit opens its editor, Remove requires existing confirmation. Escape/click-away dismisses the menu. No tests added or run.

### P12.209 — Expand activation progress and simplify elapsed text

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Remove the Last progress age from activation display and replace its 520px cap with full available width. Preserve elapsed time, percentage, byte counts and lost-contact reporting. Display-only change; no activation logic, backend changes, restarts or live operations performed.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.209 only.
**Commit:** `P12.209: expand world activation progress display`

**Manual acceptance:** Activation progress spans the world section width and shows elapsed time without Last progress text. Lost-agent-contact reporting remains available. No tests added or run.

### P12.210 — Fill stretched world slot cards with selection outline

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldSlotCard.svelte`, this plan.
**What:** Make the inner slot wrapper fill the grid-stretched Card. The selected border and background now cover the entire card height, which follows the tallest card in each grid row even when a slot has fewer metadata lines.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.210 only.
**Commit:** `P12.210: fill world slot selection cards`

**Manual acceptance:** Select slots with and without seed/profile details in the same row. Their selected border should run around the full, equal-height card including the blank space below shorter metadata. Svelte check passed; no tests added or run.

### P12.211 — Keep long active world names inside Overview cards

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/home/HomeSection.svelte`, `clients/desktop-web/src/lib/sections/home/ActiveWorldCard.svelte`, this plan.
**What:** Allow the Activity grid tracks and Active World column to shrink below their contents' intrinsic width. Let the world title metadata column take only available space and clip its existing single-line ellipsis inside the card. Long slot names no longer push into or overlap Chat.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.211 only.
**Commit:** `P12.211: contain long active world names`

**Manual acceptance:** Open Overview with an unusually long active-world slot name. The title should truncate within Active World and Chat should keep its own column with no overlap. Svelte check passed; no tests added or run.

### P12.212 — Freeze activation elapsed time at completion

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, this plan.
**What:** Stop the one-second elapsed-time clock as soon as the agent reports succeeded, failed, or cancelled. Keep the terminal progress and elapsed value steady while worlds/backups refresh; existing cleanup then closes the progress display.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.212 only.
**Commit:** `P12.212: freeze world activation elapsed timer`

**Manual acceptance:** Activate a world. On terminal status, elapsed time must stop changing immediately, including while the refreshed world list is loading. No tests added or run.

### P12.213 — Keep sidebar gameplay defaults in the active world profile

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, `crates/msc-agent/src/routes/worlds.rs`, `docs/msc2/clients/world-settings.md`, this plan.
**What:** Read sidebar Difficulty and Gamemode from the active slot's `WorldProfile` and save changes back through its profile endpoint. Refresh the sidebar after profile saves, world activation, and world creation. Running Bedrock servers apply these fields with runtime commands and report pending restart if a command cannot be sent. Java uses the shared profile application path for Paper and all other Java flavors.
**Verify:** `npm --prefix clients/desktop-web run check` and `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.213 only.
**Commit:** `P12.213: sync sidebar world gameplay settings`

**Manual acceptance:** On Bedrock and Java servers, confirm the sidebar and active slot's World Settings show the same difficulty and default game mode. Change each value from either surface and confirm the other reflects it. Check Creative confirmation behavior on Bedrock and Java; confirm Paper, Fabric, Forge, and NeoForge share the Java behavior. No tests added or run.

### P12.214 — Keep latest Java selection stable through server creation

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, this plan.
**What:** Pin the resolved latest Java version when the wizard first asks which runtime to use. This keeps the selected Java runtime associated with the same release through the World step and Create action, so the wizard doesn't ask twice. Changing the selected version or Java flavor still invalidates the selection.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.214 only.
**Commit:** `P12.214: keep latest java selection through creation`

**Manual acceptance:** Create a fresh Java server with Download latest selected. Choose Java at the first prompt; the final Create Server action should not prompt again. If you go back and change Minecraft version or Java flavor, the runtime picker should appear again before proceeding. No tests added or run.

### P12.215 — Apply Bedrock default game mode to online players

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/routes/worlds.rs`, `docs/msc2/clients/world-settings.md`, this plan.
**What:** When the active, running Bedrock world's default game mode changes, keep applying `defaultgamemode` for the saved world default and also run `gamemode <mode> @a` so currently connected players switch immediately. Difficulty continues to apply through Bedrock's live difficulty command. Keep Creative's existing achievements confirmation.
**Verify:** `cargo fmt --all` and `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.215 only.
**Commit:** `P12.215: apply bedrock gamemode to online players`

**Manual acceptance:** With multiple players online, change the active Bedrock world's default game mode from the sidebar or World Settings. Every connected player should switch immediately; new players should also receive the saved default. Confirm Bedrock Creative still requires the existing achievement warning. No tests added or run.

### P12.216 — Replace sidebar whitelist with Enforce Gamemode

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, `clients/desktop-web/src/lib/sections/settings/SettingsSection.svelte`, `crates/msc-domain/src/settings_schema.rs`, `docs/msc2/clients/world-settings.md`, this plan.
**What:** Replace the sidebar Whitelist control with Enforce Gamemode. Read and save the server-wide `force-gamemode` property through `/v1/settings`, preserve its existing confirmation when enabled, and refresh the sidebar when Server Settings changes it. Mark this property restart-required for Java and Bedrock because the running server reads it at startup; explain that it enforces the server default when players join.
**Verify:** `npm --prefix clients/desktop-web run check` and `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.216 only.
**Commit:** `P12.216: replace sidebar whitelist with enforce gamemode`

**Manual acceptance:** Confirm the sidebar shows Force Gamemode's saved value on Java and Bedrock, including values changed in Server Settings. Enable it and accept the server-wide confirmation; the setting should persist and indicate restart when the server is running. After restart, players joining should receive the server default. Disabling should persist without confirmation. No tests added or run.

### P17.28 — Resolve the CLI socket from the installed agent service

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/cli/transport.rs`, `crates/msc-agent/src/cli/service.rs`, `docs/msc2/clients/phase17-cli.md`, this plan.
**What:** On macOS and Linux, locate the local CLI socket using an explicit process `MSC2_DATA_DIR` when set, otherwise read `MSC2_DATA_DIR` from the installed agent service definition, and use the existing platform default when the service has no override. This fixes the macOS headless installer path mismatch and also supports Linux custom data roots. Windows uses a fixed named pipe and is unaffected. Add focused path-precedence tests without running them.
**Verify:** `cargo check -p msc-agent`
**Batch:** P17.28 only.
**Commit:** `P17.28: resolve cli socket from installed service`

**Essential tests:** Two pure path-selection cases protect the concrete CLI authorization failure caused by a service/CLI data-directory mismatch and retain the documented explicit environment override. They use fixed paths, do not touch the host, and should complete in under one second after compilation. Tests were not run.

**Manual acceptance:** On macOS, run `msc capabilities` without setting `MSC2_DATA_DIR`; confirm it uses the installed service path. On Linux, repeat with the default installer path and with an explicitly configured service data root. Confirm an explicit shell `MSC2_DATA_DIR` still takes precedence. Windows needs no path-specific change because its CLI connects to the fixed local named pipe.

### P12.217 — Remove Enforce Gamemode helper text

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, this plan.
**What:** Remove the explanatory sentence beneath the sidebar Enforce Gamemode toggle.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.217 only.
**Commit:** `P12.217: remove enforce gamemode helper text`

### P12.218 — Allow older Java runtimes in first-run setup

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src-tauri/tauri.conf.json`, this plan.
**What:** Let first-run setup select Java 8 or later instead of incorrectly requiring Java 21 before a Minecraft version is known. Keep version-specific compatibility checks at server creation, and set the default Tauri window to 1740 × 1080 logical pixels to match Cameron's current window.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.218 only.
**Commit:** `P12.218: allow older java runtimes during setup`

### P12.219 — Match the default window to the resized app

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src-tauri/tauri.conf.json`, this plan.
**What:** Set the initial Tauri content area to 1600 × 900 logical pixels to match Cameron's manually resized app window.
**Verify:** Rebuild and launch the desktop app; confirm the initial window opens at the resized dimensions instead of filling the display.
**Batch:** P12.219 only.
**Commit:** `P12.219: match default window to resized app`

### P12.220 — Remove Tailscale from first-run setup

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, this plan.
**What:** Remove the Tailscale page from first-run setup and remove its mention from the intro page. Keep the remaining setup steps, optional skips, and completion navigation in order.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.220 only.
**Commit:** `P12.220: remove tailscale from first-run setup`

### P12.221 — Correct managed helper updates and startup recovery

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/geyser.rs`, `crates/msc-application/tests/geyser.rs`, desktop Components, App, FirstStartSheet, StartupFailurePanel, StartupFailureSheet, this plan.
**What:** Compare installed helper checksums against official releases; preserve disabled paths on update; restrict managed helper menus to plugins. Restart running servers during startup recovery, label plugin failures accurately, retain helper diagnosis, use official updates for helper repair, and surface soft helper failures after first-start completion. Add one essential controlled regression for disabled Floodgate replacement followed by a current suffixed snapshot: protects repeated-download and accidental-enable failures absent from existing coverage. Uses a fake provider and unique temporary directory, no network/timing assumptions; expected runtime under one second. Tests added but not run.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-application -- -D warnings && cargo check -p msc-application --test geyser && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build`
**Batch:** P12.221 only.
**Commit:** `P12.221: correct managed helper updates and startup recovery`


### P12.222 — Select first-world packs during server creation

**Status:** Implemented; awaiting Cameron's verification.
**Files:** Desktop wizard WorldStep, ConfirmStep and model; shared WorldPackBrowserSheet; Java datapack route, DTO and archive installer; world pack archive preparation/activation; world activation regression; API contract/generated types; this plan.
**What:** Add a Packs disclosure after Gameplay Rules, with edition-specific browse/import choices and selected pack rows matching the Add-ons step. Reuse the Worlds pack browser in staging mode, resolving Java loader version IDs to Minecraft versions for datapack filtering. Retain selection across wizard navigation, support removal and show the staged count on Confirm. Install packs into the new active world after creation and any backup import/activation; report individual failures and skip pack installation if the selected backup fails. Add local Java ZIP import through the existing bounded staging protocol and the shared archive validation/backup/profile rollback path. Record imported Java compatibility as unknown, with no invented catalog identity. Preserve Bedrock linked pack behavior. Refresh generated active Java worlds before archive changes and reapply the modified world, so packs selected for an imported backup also reach its live folder. Keep pre-generation packs in a separate `packs.zip`, install only validated pack paths during fresh-world activation, and preserve seed/first-generation settings without creating a fake saved world. Add one essential controlled regression covering local Java import, Java/Bedrock pre-generation activation twice, retained seed and refusal of non-pack files. Uses tiny local ZIPs and unique temporary directories with cleanup, no network or timing assumptions; expected under one second after compilation. Test compiled but not run; Cameron verifies wizard integration manually.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && cargo check -p msc-application --test world_activation && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build`
**Batch:** P12.222 only.
**Commit:** `P12.222: select first-world packs during server creation`

**Checks:** Formatting, ordinary Clippy, regression compilation, Svelte check and production frontend build passed. Svelte reports eleven existing warnings. Clippy reports the existing unused `auth::forbidden` function; strict `-D warnings` fails on that unrelated warning. No release workflow or test suite was run.

**Manual acceptance:** Rebuild/restart the app and agent. On a Java server, expand Packs below Gameplay Rules, browse datapacks filtered to the configured Minecraft version, add a catalog release and import a local ZIP. Confirm their rows show titles/descriptions/icons where available and Remove works; navigate Back/Continue and confirm the selection persists. Create the server and verify the selected packs in its Worlds tab and after first start. Repeat on Bedrock with behavior/resource filters and a linked `.mcaddon`; inspect the resulting behavior/resource records. Try a backup world and confirm packs target the imported active slot. Cancel creation and confirm no existing world received the staged packs. Local Java imports are validated as datapack archives but their Minecraft compatibility remains unverified.


### P12.223 — Include first-world packs in the onboarding tour

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `content/guides/onboarding.json`, `content/guides/onboarding-source-map.json`, desktop tour anchors/overlay, wizard WorldStep, this plan.
**What:** After Gameplay Rules, spotlight Packs and ask the user to expand it. Advance to its overview on the section click, then wait for Okay before the world review. Explain Java datapacks, Bedrock behavior/resource packs, browsing/importing, deferred installation and the option to add packs later in Worlds. Reuse the existing disclosure action and overview presentation; retain the Packs layout as Cameron directed. Renumber downstream guide steps and map the new first-world cards to their source section. No tests added or run for this small tour wiring change.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.223 only.
**Commit:** `P12.223: include world packs in onboarding tour`

**Manual acceptance:** Restart the tour on Java and Bedrock. After Gameplay Rules → Okay, confirm Packs is highlighted with an expansion prompt; expand it and confirm its overview appears with Okay. Confirm Okay advances to world review, without requiring a pack selection.


### P12.224 — Configure CurseForge without leaving pack browsing

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, this plan.
**What:** Turn the missing-key API error into a clickable prompt that opens a small API key sheet. Provide a masked key field, the owner-requested CurseForge console link through the native external opener, and Save/Cancel. Save through the existing agent settings endpoint and retry the current search without closing the browser or creation wizard. Cancel and Escape retain the browser, query, filters and wizard draft; clear key input on dismissal/success. Missing credentials no longer show the misleading no-results/search-term advice. No tests added or run for this existing API/UI integration.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.224 only.
**Commit:** `P12.224: configure curseforge key from pack browser`

**Manual acceptance:** With no CurseForge key, open Bedrock Browse Packs during creation and click the missing-key message. Confirm the console link opens externally, Cancel/Escape return to browsing without losing the draft, and Save stores the key and retries the current search. Saving failures stay in the key sheet. Repeat from the Worlds tab. The API key remains saved for the agent as in Settings; it is not read back into the field.


### P12.225 — Remove duplicate world review tour instruction

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/help/TourOverlay.svelte`, this plan.
**What:** Remove the hardcoded world-review hint that repeats the guide's body. Show the instruction once, retaining Okay and Continue behavior. No tests added or run for this copy removal.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.225 only.
**Commit:** `P12.225: remove duplicate world review tour instruction`

**Manual acceptance:** Restart the tour and reach Nice, Your World Is Configured. Confirm the review instruction appears once and Okay still reveals the world page for review and Continue.

### P12.226 — Show startup failure explanations once

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/StartupFailurePanel.svelte`, this plan.
**What:** Show each diagnosed failure explanation in its finding only, omitting the duplicate heading summary. For load failures with a supplied explanation, use that explanation directly and normalize its final period. Retain fallback summaries when no diagnosis exists and all repair/restart actions. No tests added or run for this copy cleanup.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.226 only.
**Commit:** `P12.226: remove repeated startup failure explanations`

**Manual acceptance:** Start Paper with the incompatible Geyser build. Confirm its explanation appears once, ends with one period, and the existing actions remain available. The same rendering applies to Floodgate findings.

### P12.227 — Exclude Tauri build output from Vite watching

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/vite.config.ts`, this plan.
**What:** Exclude `src-tauri/target` from Vite's recursive file watcher. Tauri compiles Rust dependencies into this directory while Vite watches the frontend root; on Windows, watching a locked proc-macro DLL fails with `EBUSY`. This generated directory is not frontend source on any platform. No tests added or run.
**Verify:** From `clients/desktop-web`, run `npx tauri dev` and confirm Vite starts without an `EBUSY` watcher error and the app window opens.
**Batch:** P12.227 only.
**Commit:** `P12.227: exclude tauri target from vite watcher`

### P12.228 — Keep Windows agent installation responsive and elevate registration

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src-tauri/src/lib.rs`, `windows_service.rs`, `windows_service.ps1`, `windows_service_prompt.cs`, `crates/msc-platform-windows/src/service.rs`, this plan.
**What:** Run service actions and status checks on blocking workers so OS prompts and subprocess waits cannot freeze the desktop event loop. Replace the piped PowerShell credential prompt with an explicitly displayed native Windows credential dialog owned by the app. Ask for UAC approval for registration/removal; retain the original installing account even when a different administrator approves UAC. Collect its password inside the elevated helper, preserve the existing install request/environment through the agent CLI, grant service-logon permission and query/start/stop access to this service, and wait for startup. Routine start/stop stay unelevated under D-025. Return cancellation and diagnostic errors through a temporary result file containing no credentials, then remove it. Hide helper consoles and redact credentials from service-controller failures, including echoed diagnostics. macOS/Linux keep their existing platform actions with the new background dispatch.
**Verify:** From `clients/desktop-web`, run `npx tauri dev`; follow the manual acceptance below.
**Batch:** P12.228 only.
**Commit:** `P12.228: keep windows agent installation responsive`

**Checks:** Rust formatting, ordinary Clippy for the desktop and Windows platform crate, PowerShell parsing, and C# native-helper compilation passed. Strict Clippy is blocked by existing warnings: the unused infrastructure Bedrock socket-mode constant, unused desktop update helper and Windows installing-user return. The password-redaction regression was compiled with the platform tests; no tests were run. No release workflow changed or run.
**Essential regression:** A service-registration failure previously included the password-bearing command arguments in its error. The new controlled regression supplies command arguments and fake stdout/stderr containing a password, then checks that the password is absent and the useful error remains. It performs no OS calls, uses no timing/environment assumptions and should run in under one millisecond. It protects secret disclosure rather than incidental error wording.
**Manual acceptance:** Restart the development session. Choose Install, cancel UAC, and confirm the app reports cancellation and responds normally. Retry, approve UAC, cancel the native password dialog, and confirm the same. Retry with the installing account's Windows password (not its Hello PIN); confirm the agent reaches Running and connects. Stop/start the installed agent and confirm no further UAC or password prompt. Repair should use UAC and the credential dialog again. Check a failed credential/start attempt reports its error without disclosing the password. Live Windows acceptance and macOS/Linux physical checks remain Cameron's verification.

### P12.229 — Authenticate the Windows desktop with its running local agent

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src-tauri/src/lib.rs`, this plan.
**What:** Enable the existing host-local desktop pairing flow on Windows as well as Linux. The local bootstrap command previously returned unavailable on Windows even after successful service installation. Reuse the installing-account pairing CLI, redeem its one-use code through the loopback API, and store the resulting desktop credential in the native credential store. Preserve cached-credential probing and the agent identity check. Hide the Windows pairing subprocess console; codes and bearer tokens remain inside the native backend. No authentication bypass, agent-service change or tests added.
**Verify:** From `clients/desktop-web`, run `npx tauri dev`; confirm the already running Windows agent connects without another installation or repair.
**Batch:** P12.229 only.
**Commit:** `P12.229: authenticate local windows desktop agent`

**Diagnosis:** The installed Windows service was Running under Cameron's account and `/v1/healthz` returned HTTP 204. The desktop bootstrap had only macOS/Linux branches and returned unavailable on Windows; the setup screen replaced that failure with its generic reach/authenticate message.
**Checks:** Desktop Rust formatting and ordinary Clippy passed, with the same three existing Windows/infrastructure warnings recorded in P12.228. No tests or pairing commands were run by the agent.
**Manual acceptance:** Restart the desktop development session and confirm the existing running local agent connects. Restart again to confirm the stored credential is reused. If Repair is subsequently used, confirm its automatic connection retry also authenticates. Live pairing/credential verification remains Cameron's check; no tests were run.

### P18.10w — Expose depth slicing in every 2D dimension

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, this plan.
**What:** Extend the existing Depth Y control to every dimension in 2D. Use the viewer's manifest-backed height range (including its supported lower slice limit), retaining the Nether's Y126 roof limit and Y83 default when entering from above the roof. Other dimensions enter at full height; lowering Y cuts away terrain above that level. Hold camera focus at the selected height so surface tracking cannot lift it out of the cave; changing depth stops Follow. 3D, Fly, player focus and Home restore unsliced terrain and normal height tracking. Preserve the existing control styling under the required `antiAIslop.md` design law. No tests added or run for this narrow use of the existing viewer slice API.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P18.10w only.
**Commit:** `P18.10w: enable depth slicing in all 2d dimensions`

**Checks:** Svelte check passed with zero errors and eleven existing warnings outside the map component. No Rust files changed, so Rust formatting/Clippy are not relevant to this step. No test suites or release workflows run.

**Manual acceptance:** Reopen the map, select 2D in the Overworld and lower Depth Y below the surface, including negative Y. Confirm saved caves appear, panning/zooming retain the selected focus height, and 3D/Fly restore full terrain. Repeat in the End and a saved custom dimension; confirm each range follows its terrain manifest. Check Nether 2D still opens at Y83 from above the roof and stops at Y126. Follow a player, then change depth and confirm Follow stops. Home should restore the unsliced spawn view. Cave visibility depends on the saved geometry present in the map; this frontend change does not generate missing chunks.
