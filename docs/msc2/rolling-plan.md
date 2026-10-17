# MSC 2 — Rolling Plan

Implemented steps were marked Done and moved to [the archive](rolling-plan-archive.md) at Cameron's direction on 2026-10-08 and 2026-10-13. The October 13 cleanup archived 27 implemented or completed-record steps, including P16.80–P16.82. Planned work, publication records without confirmed completion, and independent phase acceptance remain below. Outstanding verification notes are preserved with the archived steps; Done does not establish physical acceptance or close a phase gate.

### P16.89 — Keep XboxBroadcast running through Bedrock status polling

**Status:** Implemented; awaiting Cameron's verification on the affected Intel Mac. Owner explicitly requested the fix and a quick Intel delivery on October 16.
**Files:** `crates/msc-application/src/bedrock_runtime.rs`, `xbox_broadcast.rs`; their focused integration tests; `crates/msc-agent/src/routes/lifecycle.rs`, `networking.rs`; `tools/release/prepare-macos-agent-fix.sh`; `docs/msc2/clients/broadcast-agent-fix.md`; this plan. Generated app/archive are not committed.
**What:** Queue asynchronous Bedrock events received while waiting for command/provision/start replies, then drain them in order through the normal event reader. Preserve broadcast when a rejected event leaves the Bedrock runtime live; log the actual event error to console and agent stderr, while retaining cleanup for lost runtimes and confirmed termination. Reserve broadcast start before acquisition, return its operation immediately, download on a blocking worker outside helper-state locks, and prevent stopped/cancelled/replaced preparations from launching late. Serialize broadcast acquisition separately to protect its shared staging files. Reuse the published Intel v0.1.29 desktop/helper bundle with a newly compiled native agent for temporary local delivery; resign the replaced agent and outer app, retain the Swift helper signature/entitlement and document normal app replacement plus Repair agent. No transport change, release publication, tag, installed-service mutation or world-data changes on this machine.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings -A dead_code`
**Batch:** P16.89 only; one corrective commit and local delivery artifacts, no release workflow or test execution.
**Commit:** `P16.89: preserve broadcast through bedrock status polling`

**Evidence:** The owner's process sample captures `spawn_playit_output_pump -> start_pending -> status_snapshot -> drain_bedrock_events -> stop_broadcast_for_server`, waiting for the broadcast lock while the start handler downloads/checksums the helper. It also captures that stop routine cancelling the operation. Bedrock remains running. The precise rejected sidecar frame is not in the sample; inspection establishes that `receive_response` previously consumed arbitrary console/metrics/readiness frames as replies and left the actual acknowledgement for `poll_event` to reject.

**Essential regressions:** Two cases exercise command and startup acknowledgements interleaved with console/metrics/readiness, protecting preserved event order and successful command/start completion. Two cases exercise stop/replacement and cancellation before acquisition completion, protecting against late helper launch or an old worker failing the new operation. Existing focused cases assumed uninterrupted replies and synchronous acquisition. All use injected transports/filesystems/processes, with no VM, network, wall-clock waits or machine dependencies; expected runtime below one second for the four cases. Compiled, not executed.

**Checks:** Native Intel `cargo build --locked -p msc-agent --bin msc` passed. Agent Clippy passed with only the pre-existing unused `auth::forbidden` function excluded through `-A dead_code`; focused Bedrock runtime and broadcast regression targets compiled with Clippy and warnings denied. Formatting, shell syntax and diff whitespace checks passed. An initial broad all-target check exposed unrelated existing `provisioning::purpur_transport`, `cli_pairing` route imports and `cli_service` argument errors; none were changed. No tests executed. Published Intel base DMG checksum matched; prepared app passed deep strict signature verification, retained the original Swift helper/appliance bytes and virtualization entitlement, and contains an x86_64 replacement agent. Local installed app/services were not changed. Physical acceptance remains pending.

**Delivery:** Native Intel development agent and a temporary app ZIP using the checksum-verified published 0.1.29 Intel DMG. The installed local app is older, so it is not used as the delivery base. Owner must stop Minecraft, replace the desktop app, and Repair agent before acceptance. Keep the normal published installer for rollback. Version remains 0.1.29; this local correction is identified by the step commit and BUILD.txt rather than a release claim.

### P16.88 — Reuse release compilation and cache Rust dependencies

**Status:** Implemented; awaiting owner verification in a future workflow run. Cameron explicitly requested faster release builds, particularly Intel macOS.
**Files:** `.github/workflows/release.yml`, `clients/desktop-web/tools/prepare-agent-dev.mjs`, `tools/release/build-macos-headless.sh`, this plan.
**What:** Keep the existing four-platform build-only release workflow. Pass its native Rust target to desktop staging so the agent uses the already-built `target/<triple>/release/msc` output and the map exporter shares its output with headless packaging. Preserve ordinary development staging without a target argument and reject duplicate or nonnative targets. Keep Cargo dependency resolution locked during staging and macOS headless builds. Add a commit-pinned Rust cache covering agent, desktop shell, map exporter and Windows lifecycle-helper dependency outputs, separated by platform and automatically keyed by toolchain/dependency configuration. Preserve all nine release artifacts, checksums, signed update metadata, sidecar appliance checks and entitlement verification. No new tests, lint gates, runner purchases, release tags or workflow triggers.
**Verify:** `git show --stat --oneline --grep='P16.88:' HEAD`
**Batch:** P16.88 only; update workflow and build scripts, commit and push main. Do not restart the current release or dispatch a new build.
**Commit:** `P16.88: reuse release compilation and cache rust dependencies`

**Checks:** JavaScript syntax check, macOS packaging shell syntax check, YAML parsing and whitespace inspection passed. Inspected Cargo output paths against desktop staging, headless packaging and Windows staging. No Rust source changed; no tests or full release builds run. Build-time improvement remains unmeasured. The previous successful Intel run spent approximately 16 minutes building the agent and another 13 minutes staging its payload; staging includes other work, so these figures do not establish a promised saving.

**Cache operation:** GitHub tag caches are isolated from later tags. An owner-directed, build-only run on the default branch seeds reusable dependencies for subsequent release tags: `gh workflow run release.yml --ref main -f publish=false`. This command builds all four platforms and is deliberately not launched here. Later tags can restore that default-branch cache while available; dependency/toolchain changes and cache eviction can require fresh compilation. No automatic main-branch builds are added. The target-directory reuse benefits the next release even with an empty cache. Existing v0.1.29 continues using its immutable workflow definition.

### P16.87 — Present v0.1.29 as released in the README

**Status:** Implemented; awaiting owner verification. Owner explicitly requested README wording as if v0.1.29 were released.
**Files:** `README.md`, this plan.
**What:** Remove the pending-candidate notice and present v0.1.29 as the latest published prerelease throughout current-release text, download links, package examples and the update command. Replace the older opening feature summary with automatic SSH pairing, reduced diagnostic polling, continuing shutdown-status reconciliation and Intel Mac Bedrock worker exit. Include the updated-app Repair agent instruction and preserve unsigned-beta, platform support, ownership-recovery and open physical acceptance distinctions. This wording request does not establish workflow completion or change the existing release tag.
**Verify:** `git show --format= -- README.md`
**Batch:** P16.87 only; documentation only, no release workflow or tag changes.
**Commit:** `P16.87: present v0.1.29 as released in readme`

**Checks:** Inspected all README version references, current-release summary, download links and package/update examples; whitespace check passed. No tests run or workflow monitoring performed.

### P16.86 — Publish v0.1.29 with remote pairing and shutdown corrections

**Status:** Prepared; publication explicitly authorized by Cameron on October 14. Push the new tag once and report the workflow link without monitoring builders, following the owner's existing preference. Publication and artifact acceptance remain separate.
**Files:** Agent/desktop/frontend manifests and lockfiles, source/static bundle identity, README candidate notice, `docs/msc2/release/v0.1.29.md`, this plan.
**What:** Increment the latest confirmed published v0.1.28 to v0.1.29 and publish through the existing build-only workflow. Include P16.80 automatic SSH command discovery/shared pairing storage, P16.82 removal of hidden health polling and bounded diagnostics, P16.84 continuing shutdown-status reconciliation, and P16.85 reliable Bedrock VM-worker exit. Align the static bundle identity with the owned source versions (its previous static value was still 0.1.25). Keep README's latest-confirmed publication factual until the new run publishes, with a separate pending candidate notice. Preserve all nine desktop/headless artifacts, checksums and signed update metadata. Do not move an existing tag, add checks to the workflow, or execute tests. World-time investigation is deferred at the owner's request; case-alias world swapping is not claimed fixed.
**Verify:** `gh release view v0.1.29 --json tagName,isPrerelease,assets,url`
**Batch:** P16.86 only; prepare, commit, push main and a new immutable v0.1.29 tag once, report the triggered run without waiting for builders.
**Commit:** `P16.86: prepare v0.1.29 connection and shutdown release`

**Preparation evidence:** GitHub confirms v0.1.28 published successfully with all nine expected platform installers/archives, SHA256SUMS, release notes and both signed-update metadata files. Its successful workflow is run 38352114339; the older failed run remains historical and is not retried. The active workflow remains unchanged. Repository configuration includes the required signing-secret and public-key variable names; secret values were not read. Agent and desktop locked Cargo metadata resolve. Owned package versions and source/static identities agree at 0.1.29. Frontend production build and bundle-identity inspection passed with the existing warnings/advisories. The tracked appliance SHA-256 matches all four declarations. Inspection confirms Intel staging rebuilds the corrected Swift helper from source and the headless package copies that staged payload. The P16.85 Release and test-compilation builds already passed; no tests were executed for this release. Whitespace check passed.

**Acceptance still open:** Fresh remote pairing without shell workarounds or saved credentials, sustained ATM10 management without the captured health-scan stall, long shutdown UI reconciliation, and fresh Bedrock initiation/shutdown with the corrected installed helper. The manual removal of the previously stuck worker established live recovery only. Release notes direct Intel Mac users to install the corrected app and repair the agent/helper; they preserve physical acceptance and known-limit distinctions. No live server or installed helper is changed by publication preparation.

### P16.85 — Let the macOS Bedrock VM worker exit after clean shutdown

**Status:** Implemented; awaiting owner verification of the corrected installed helper. Owner explicitly authorized stopping the stuck local server; live recovery completed.
**Files:** `sidecar/bedrock/BedrockSidecarCore.swift`, `sidecar/bedrock/Tests/BedrockSidecarTests.swift`, this plan.
**What:** Replace `RunLoop.main.run()` in the standalone/VM-worker sidecar entry point with `CFRunLoopRun()`, matching the existing `CFRunLoopStop` invoked only after controller shutdown completes. Foundation's repeating run loop could re-enter after the stop, keeping a cleaned-up worker process alive. The root helper correctly waits for that worker's actual exit before acknowledging teardown, so the idle worker blocked Stopped status and first-start completion. Preserve the existing VM cleanup, non-root worker ownership and parent waitpid boundary; do not substitute the guest's console text for proof of host cleanup. No release publication or installed-binary replacement performed.
**Verify:** `xcodebuild -project sidecar/bedrock/BedrockSidecar.xcodeproj -scheme BedrockSidecar -configuration Release -derivedDataPath /tmp/msc-bedrock-shutdown-build build CODE_SIGNING_ALLOWED=NO`
**Batch:** P16.85 only; implementation and explicitly authorized local stop. No test execution or other server actions.
**Commit:** `P16.85: let bedrock vm workers exit after shutdown`

**Live evidence and recovery:** The owner confirmed this is the local Mac and authorized direct diagnostics. BDS logged `Quit correctly` and `[appliance] BDS exited — powering off`, but user-owned VM worker PID 31953 remained under root helper PID 30346. A three-second sample showed only the idle main Foundation run loop and an idle workqueue thread, with a roughly 2.4 MiB footprint, consistent with completed VM teardown and the input reader already gone. After confirming the PID, parent and owner again, sent SIGTERM only to the stuck worker at the owner's request. The root helper reaped it and remained running. `/usr/local/bin/msc status --json` subsequently reported `running:false` and `lastTeardownReason:clean`; the worker was absent. No world files or service registration were changed. The old missing-socket log messages do not explain this hang: the socket directory and socket existed during diagnosis.

**Essential regression:** `testSidecarExitsAfterSupervisorClosesInput` launches the built standalone sidecar and `--vm-worker` with an input pipe, then closes the pipe and requires successful process exit. It starts no VM, Minecraft, service or network, touches no world, and terminates a stuck child on failure. This exercises the actual executable's EOF/cleanup/run-loop-exit boundary rather than comparing source text. Existing controller-level tests did not exercise process exit. Expected runtime under one second for both successful cases; a five-second condition-based wait bounds each failure without arbitrary sleeps. Compiled, not executed.

**Checks:** Xcode Release build and Debug `build-for-testing` passed with signing disabled for build-only verification. The latter compiled the XCTest target without executing it. Existing TCP relay Sendable, resource-validation build-script and AppIntents metadata warnings remain. Whitespace check passed. No Rust or frontend source changed, so their checks are not relevant to this step. The live stop used the old installed helper and establishes recovery, not acceptance of the new binary.

**Owner acceptance:** Rebuild/install the corrected desktop/helper payload and use Repair agent to register the new helper before the next Bedrock run. Create/initiate a Bedrock server and require BDS shutdown, worker process exit, agent Stopped status and the first-start completion page without a refresh or manual termination. Repeat an ordinary start/stop and confirm only the persistent helper service remains. Verify the root supervisor still waits for worker teardown and the worker runs under the installing account. Do not count the manual recovery above as verification of this fix.

### P16.84 — Reconcile long Minecraft shutdowns without a manual refresh

**Status:** Implemented; awaiting owner verification.
**Files:** `clients/desktop-web/src/App.svelte`, `src/lib/lifecycle/serverLifecycle.ts`, `src/lib/components/ApplicationShell.svelte`, `src/lib/components/shell/ControlSidebar.svelte`, `src-tauri/src/lib.rs`, `tests/lifecycle/serverLifecycle.test.ts`, this plan.
**What:** Fix the stale shell/Stop-button state after a shutdown exceeds the existing ten-second action wait. While a connected desktop is mounted, reconcile only lightweight `/v1/status` every two seconds, independent of the active tab. Keep a host/server-scoped pending stop until the agent reports stopped (or a different PID establishes a replacement process), displaying disabled `Stopping…` rather than enabling duplicate Start/Stop actions during that stop. The initial action wait remains bounded, but routine reconciliation continues afterward and also observes stops issued from another desktop or the console. Coalesce overlapping reads, invalidate old reads on reconnect/selection/lifecycle actions, guard application by captured client/host/server/generation, and bind action/status/operation requests to the client that received the action. Keep pending actions scoped when switching hosts. Clear the polling timer on unmount. Display status-read failures below the sidebar controls, retain the last confirmed state on failure, and retry on subsequent ticks. Native GET `/v1/status` requests have a five-second timeout so a stalled request cannot indefinitely occupy the poll slot; other request timeouts are unchanged. Restore no full health polling and do not change agent stop semantics or introduce a forced shutdown.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P16.84 only; no publication, live server actions or test execution.
**Commit:** `P16.84: reconcile server shutdown state without manual refresh`

**Diagnosis:** The shell assigned its running state after `waitForRunningState`, which returns its last snapshot after ten seconds. It had no continuing shell status reconciliation afterward. Individual tabs' polling updated their own data rather than the shell's `status`. A large modpack still saving at that boundary therefore left the sidebar showing Stop until a manual refresh. The owner's shutdown duration has not been measured; the code gap is directly established by inspection.

**Essential regressions:** Three cases extend the existing lifecycle test file: an initially timed-out shutdown later becoming stopped, coalescing concurrent reads while rejecting a previous host context's late response, and resuming status application after a failed read. These protect the reported stale state plus host isolation and recovery introduced by this monitor; earlier tests covered only the bounded initial wait. Use injected promises and a synthetic clock, without real timers/sleeps, servers, networks or environment mutation; expected runtime below one second for the focused file. Compiled through Svelte/TypeScript checking; no tests executed. Optional owner command: `npm --prefix clients/desktop-web run test:unit -- tests/lifecycle/serverLifecycle.test.ts`.

**Checks:** Frontend type checking passed with zero errors and the same 11 existing warnings. Production frontend build passed with existing Svelte, mixed Tauri-import and chunk-size advisories. Desktop Rust formatting and Clippy compilation passed with the three previously recorded unrelated lint exclusions (`needless_borrow`, `needless_borrows_for_generic_args`, `needless_return`). Whitespace check passed. No tests run and no installed app/agent/server changed. Read `antiAIslop.md`; the visual change uses the existing button with a text state and a restrained status-error paragraph, without decorative effects or indicators.

**Owner acceptance:** Use the corrected desktop build against All the Mods 10. Click Stop and leave a non-Overview tab open; require `Stopping…` while Minecraft saves and exits, then an automatic return to Start after the agent reports stopped, including a shutdown longer than ten seconds. Confirm a stop through the other desktop or Minecraft console updates the first desktop without refresh. Switch hosts during a pending shutdown and ensure the old host's late replies cannot populate the new view; return to confirm its current state. Confirm transient status-read failure shows a notice and recovers when reads succeed. Check an ordinary start, short shutdown, uptime and sidebar controls still work. Refreshing/reconnecting must remain compatible with P16.80/P16.82; no full `/v1/health` requests are introduced by this monitor. Live acceptance remains open.

### P16.76 — Republish the failed v0.1.28 at the owner's request

**Status:** Prepared; publication explicitly reauthorized October 13. Owner directed reuse of 0.1.28 rather than 0.1.29 after lifting the publication hold. Artifact and physical verification remain open.
**Files:** Agent/desktop/frontend versions and lockfiles, bundle identity, README, revised `docs/msc2/release/v0.1.28.md`, removal of unused 0.1.29 notes, this plan.
**What:** Restore all owned versions to 0.1.28, include P16.73's map changes and P16.74's checksum correction, and replace the failed v0.1.28 tag at the owner's explicit request. This is an owner-directed exception to the ordinary no-tag-movement rule. GitHub's release-by-tag API returned HTTP 404 and the failed workflow completed without publication. Replace only the verified old tag `517e7de636ad7998259aa01111bfa900b81845eb` using an explicit force-with-lease; preserve the failed run history. No 0.1.29 tag exists. File-structure/pack artifact cleanup remains deferred at the owner's direction. Existing build-only workflow and complete signed artifact set remain unchanged.
**Verify:** `gh release view v0.1.28 --json tagName,isPrerelease,assets,url`
**Batch:** P16.76 only; push the corrected tag once and report the workflow without monitoring builders, following the owner's earlier preference.
**Commit:** `P16.76: republish corrected v0.1.28 at owner direction`

**Evidence:** Both focused snapshot regressions passed under specific owner authorization. Full local desktop payload staging, including Release Swift helper and appliance checks, passed after correcting the fourth checksum declaration. Locked offline metadata and actual appliance validation pass with restored 0.1.28 versions. Rebuilding the frontend verifies the changed bundle identity. No source behavior changes follow the tested P16.73/P16.74 implementation. Physical ownership/networking and the actual large map remain unverified by the implementing agent.

### P16.75 — Prepare v0.1.29 and hold publication for the owner

**Status:** Superseded by P16.76. Owner explicitly requested reusing the failed, unpublished v0.1.28; no v0.1.29 tag or workflow was created.
**Files:** Agent/desktop/frontend versions and lockfiles, bundle identity, README, `docs/msc2/release/v0.1.29.md`, this plan.
**What:** Prepare a fresh immutable v0.1.29 candidate containing P16.71's ownership correction, P16.73's large-map snapshots and P16.74's Intel packaging fix. Preserve nine packages, appliance checksums and signed update metadata. Do not move/retry v0.1.28, push a release tag or dispatch a workflow. No new release gates.
**Verify:** `git show --stat --oneline HEAD && node clients/desktop-web/tools/prepare-agent-dev.mjs --validate-appliance`
**Batch:** P16.75 only. Stop after local preparation and checks, before starting publication, as explicitly requested by Cameron.
**Commit:** `P16.75: prepare large bedrock map release without starting publication`

**Evidence and limits:** v0.1.28 failed only at Intel appliance staging and did not publish; the original tag remains unchanged. All four appliance declarations now match the actual archive, and the complete local debug staging path passed (including Release Swift helper, entitlement checks and optimized map exporter). The two specifically authorized map snapshot regressions passed after the controlled disk-budget checks were added. Source/lock versions and locked offline Cargo metadata agree on 0.1.29. Agent/native desktop Clippy, frontend production build, production bundle identity, Rust formatting, Node syntax, actual appliance validation and whitespace checks pass with existing unrelated warnings. README continues to link v0.1.27 as the latest published release and identifies 0.1.29 as a held candidate. Physical ownership/networking and the actual 7.2 GB Bedrock map remain owner acceptance; Prizma installation outcome and Chunker performance are not claimed fixed. No live server/agent was modified. No v0.1.29 tag was created and no workflow was started; wait for the owner's edit/question before publication.

### P16.72 — Publish v0.1.28 with Bedrock ownership correction

**Status:** Publication failed; superseded by the next candidate. v0.1.28 tag remains immutable. Intel macOS appliance staging failed; no GitHub release was published. See P16.74.
**Files:** Agent/desktop/frontend versions and lockfiles, bundle identity, README, `docs/msc2/release/v0.1.28.md`, this plan.
**What:** Release P16.71's approved root-supervisor/user-VM-worker split, early detection of existing misowned world data, and recoverable conversion activation diagnostics. Push main and a new immutable v0.1.28 tag once through the existing build-only workflow. Keep all nine packages, appliance resources, checksums and signed update metadata; add no workflow gates. Existing root-owned worlds need explicit administrator repair. The physical VM test remains owner acceptance, not an implicit publication claim.
**Verify:** `gh release view v0.1.28 --json tagName,isPrerelease,assets,url`
**Batch:** P16.72 only. Owner previously requested no release monitoring; report the triggered run without waiting for all builders.
**Commit:** `P16.72: prepare bedrock ownership correction release`

**Preparation:** v0.1.27 is published with all nine packages and metadata, and its release workflow succeeded. v0.1.28 was unused when checked. Locked offline Cargo metadata resolves for agent and desktop. Helper Release build and Debug test compilation, appliance/source/checksum consistency, guest shell syntax, Python acceptance-script compilation, Clippy and focused Rust test-target compilation passed during P16.71. No live installation was changed. Release notes distinguish implementation/build evidence from pending physical ownership and networking acceptance, and do not claim Prizma install or Chunker speed improvements.

**Release checks:** All owned versions agree on 0.1.28; locked Cargo metadata, agent/native desktop Clippy, frontend production build and bundle-identity check pass with existing unrelated warnings. Rust formatting and whitespace checks pass. The previous three publication runs succeeded. The existing workflow is unchanged: no test/lint gates added and no release reruns or moved tags. Test execution and the real VM acceptance check remain pending; the release is a beta for owner verification.

### P16.69 — Publish large-world conversion and agent recovery corrections

**Status:** Prepared; publication reauthorized by the owner on October 12 for affected-Mac testing. Release v0.1.27 includes P16.68 and P16.70. Publication results are reported with the GitHub release/workflow; owner verification remains open.
**Files:** Owned version fields/lockfiles, README, release notes, release-note selection in the existing build-only workflow and this plan.
**What:** Inspect remote tags/main and recent build results, select the next unused patch, synchronize versions and publish via the unchanged build-only workflow using one new immutable tag. Monitor to completion; verify all nine packages, checksums and signed update metadata. Record actual local conversion evidence and its limits; do not claim the affected Mac has been verified. Use committed version-specific release notes in the existing publish step so the manual macOS upgrade requirement is included in both the release body and downloadable notes. Preserve all nine packages, checksums/signatures and the build-only jobs; no test or CI gate added. No further implementation step follows.
**Verify:** `gh release view v0.1.27 --json tagName,isPrerelease,assets,url`
**Batch:** P16.69 only on resumed execution; explicitly authorized publication includes already committed P16.68 and P16.70. No live conversion or service restart.
**Commit:** `P16.69: publish conversion and agent recovery fixes`

### P16.64 — Record ATM10 investigation and compact correction plan

**Status:** Plan written; awaiting Cameron's review. No implementation in this conversation.
**Files:** `docs/msc2/rolling-plan.md` only.
**What:** Record the October 12 ATM10 8.2 / MSC 2 v0.1.25 report, confirmed causes, evidence limits and two execution steps below. Cameron requested a short sequence whose last execution delivers the correction without a separate release-planning conversation.
**Verify:** `git show --check --stat --oneline HEAD`
**Batch:** P16.64 only (PLAN).
**Commit:** `P16.64: plan atm10 restart and import reporting fixes`

**Investigation results:**

- Owner reported an Intel Mac on v0.1.25: inspection warned that six files might require manual completion; completion showed four Modrinth recoveries and 445 direct downloads; First Start succeeded; the next start failed with “Invalid data pack ID.” Creating a new world allowed one start, followed by the same restart failure. Screenshots confirm the warning, recovery summary, setup confirmation and error.
- Confirmed restart cause: `java_world_settings.rs::apply_existing_world` preserves enabled loader-owned packs from `level.dat`, then passes them through `valid_pack_id`, which incorrectly treats non-file pack identifiers as lowercase Minecraft resource identifiers. The local saved world's enabled list contains an OpenLoader identifier shaped like `openloader//Users/example/Library/Application Support/MSC 2/servers/java/all_the_mods_10/datapacks/sawmill.zip`. Its uppercase letters and spaces fail that rule. This is a loader-generated identifier already accepted by Minecraft, not evidence of world corruption. Personal path components above are anonymized.
- `routes/lifecycle.rs::apply_active_java_world_profile` chooses Creation without `level.dat` and Activation once it exists. Profile application happens before Java launch. The retained failed `java-start` operation records `world_settings_failed` / “Invalid data pack ID.” This explains both initialization success and new-world first-start success followed by restart failure. Relevant application source matches v0.1.25.
- Confirmed diagnostic defect: `validate_pack_files` omits the offending identifier and reason; the startup UI presents the resulting error with general Java/server/port advice. The saved clean-start record is not evidence that the later pre-launch profile application succeeded.
- Confirmed accounting defect: inspection's `resolve_manual_downloads` counts every CurseForge file lacking a download URL before import classification. Import subsequently skips non-JAR/client-only files and may classify a recovery match as client-only. The completion DTO/UI exposes downloads, recoveries and unresolved files without reconciling the preliminary warning. The two other flagged files were not individually identified in retained evidence; client-only/non-JAR exclusion is a likely explanation, not a proven historical disposition. Do not claim two missing server mods or a connection to the restart failure.
- Investigation was read-only; no tests or live launches. MSC 1 was unavailable at its documented path on this laptop; no parity claim was established from its source.

### P16.66 — Publish the corrections and complete the ATM10 handoff

**Status:** Prepared for v0.1.26; publication authorized, owner artifact verification pending.
**Files:** Owned version manifests/lockfiles and bundle identity; README; `docs/msc2/release/` notes for the next unused version; this plan. Existing release workflow is inspected, not expanded.
**What:** This is the final execution step for these issues. An owner instruction “execute P16.66” authorizes preparing and publishing the corrected release: inspect current remote main/tags and available workflow failures, choose the next unused patch version, synchronize owned versions, document the ATM10 and Chunker fixes and evidence limits, and commit once. Push the authorized branch and a new immutable tag through the unchanged build-only release workflow. Preserve all nine artifacts, helper payloads, checksums and signed update metadata. Monitor that run to a definite result, inspect any failure before retrying, and do not move the tag or add release gates. Report the actual release URL/version and the single physical acceptance exercise above; no separate README, release-planning or closure execution step follows. Do not describe the bugs as physically verified or mark the steps Done before Cameron reports success. If publication fails, explicitly report the blocker rather than claim closure. Routine corrective work remains within this final step's scope, subject to the one-commit-per-step rule; an unforeseen change requiring another commit must be disclosed rather than hidden.
**Verify:** `gh release view v0.1.26 --json tagName,isPrerelease,assets,url`
**Batch:** P16.66 only; final publication/handoff for P16.65, no unrelated pending plans.
**Commit:** `P16.66: publish atm10 restart and import reporting fixes`

**Preparation evidence:** Inspected remote main/tags, the unchanged workflow and complete artifact assembly/signing path, the latest five successful runs and available signing configuration. v0.1.25 completed in about 32 minutes, with Intel macOS last; expect roughly 30–45 minutes subject to runner availability and packaging failures. No remote main changes require integration. v0.1.26 is the next unused patch tag and includes P16.65 and P16.67. Synchronize owned versions only; retain registry dependency entries. README/download examples and release notes are prepared in this same commit. Publish using a new immutable tag, report actual workflow/release evidence in the final handoff, and do not add a second commit merely to record a hash or run result. GitHub release notes will use `docs/msc2/release/v0.1.26.md` after the workflow publishes its generic body. Locked Cargo metadata resolves for agent and desktop; owned versions match and registry lock entries are unchanged. Rust formatting, agent Clippy, frontend production build, bundle identity and diff whitespace checks pass with existing warnings. No tests run or release gates added.

**Release cost and acceptance:** Recent complete release builds took 33–47 minutes; execution must reassess the current estimate before starting. The published artifact is what Cameron verifies using P16.65's single exercise. His success report closes these issues without another execute command. This narrow closure does not close unrelated phase acceptance.

### P16.62 — Publish v0.1.25 with anonymized examples

**Status:** Prepared; publication requested, owner artifact verification pending.
**Files:** Agent/desktop/frontend manifests and lockfiles; bundle identity and existing assertion; README; `docs/msc2/release/v0.1.25.md`; this plan.
**What:** Increment the latest remote tag v0.1.24 to v0.1.25 and synchronize owned versions without modifying registry dependency versions. Include P16.60–P16.61 privacy cleanup, preserving service/publisher identity and existing world map controls. Push main and the new immutable tag through the unchanged build-only release workflow. Preserve nine platform artifacts, checksums, helper bundles and signed update metadata. No tests or release gates added. Owner explicitly requested the next release/tag. The preceding v0.1.24 run was still finishing macOS builds during preparation; Linux and Windows passed and no failures were reported.
**Verify:** `gh release view v0.1.25 --json tagName,isPrerelease,assets,url`
**Batch:** P16.62 only.
**Commit:** `P16.62: prepare v0.1.25 release`

**Preparation:** Inspected active workflow, recent successful runs, preceding run jobs, remote tags/main and signing-secret/public-key configuration. No remote main changes to integrate. Recent complete runs took 33–47 minutes. Locked agent/desktop metadata, synchronized release identity, unchanged registry lock entries, Rust formatting, frontend production build and diff whitespace checks pass with existing frontend warnings. Privacy checks are recorded under P16.61; physical acceptance remains owner verification.

### P16.43 — Plan a complete Windows MSI refinement

**Status:** Plan written; awaiting Cameron's review. PLAN scope: review the entire MSI process and propose improvements after the owner reported a briefly blank opening page (2026-10-07).
**Files:** `docs/msc2/rolling-plan.md` only.
**What:** Record the source/artifact findings, proposed installer experience and P16.44–P16.50. No installer implementation, installation, service control, tests, builds or release runs in this conversation.
**Verify:** `git show --check --stat --oneline HEAD`
**Batch:** P16.43 only (PLAN). Future execution requires an owner instruction naming its step or range.
**Commit:** `P16.43: plan complete windows msi refinement`

**Review findings and limits:**

- The current `clients/desktop-web/src-tauri/tauri.conf.json` uses Tauri's stock WiX UI. The sole custom fragment, `packaging/windows/desktop-cli-path.wxs`, adds `[INSTALLDIR]agent` to machine PATH with a package-owned registry marker. There is no MSC-specific introductory copy, custom UI, or MSI service lifecycle integration.
- Read-only Windows Installer database inspection of `C:\Users\example\Downloads\msc2-0.1.15-windows-x86_64.msi` found `PrepareDlg` at UI sequence 49, followed by searches/costing and `WelcomeDlg` at 1298. The preparation and welcome text exists in that artifact; preparation Back/Next are disabled, while Cancel is authored visible/enabled. Cameron confirmed the blank page is brief and advances. This strongly points to preparation/initial painting, but does not prove why text or Cancel initially fails to paint/respond. Do not describe it as missing strings or a permanently stuck installer. The exact owner-observed MSI version remains unconfirmed; this older downloaded artifact is a baseline, not evidence about every current release. SHA-256: `1952d89bb5538f2487e5563135619dfe063aee395edfe5effe107e4aa247ffc2`.
- That older MSI has machine installation, generic destination/review/progress/completion and maintenance dialogs, a checked Launch MSC 2 completion option, and no custom MSC service actions. Its download-WebView2 action silently retrieves Microsoft's runtime bootstrapper. It has no Environment table, so it predates the current CLI PATH fragment. Do not mistake that older payload for current packaging.
- Current desktop service registration stages the agent, Vantage renderer and Bedrock exporter into a checksum-named `agent/builds` directory under the desktop agent data root. On Windows that default root is the installing user's `AppData/Roaming/MSC2`. The service therefore normally runs a copied payload rather than the MSI's packaged `agent/msc.exe`. A locked MSI agent binary is not the general cause of upgrade trouble; the important gap is coordinated replacement of the actual service payload.
- `clients/desktop-web/src-tauri/src/update.rs::install_windows_msi` invokes `msiexec /i`, treats any nonzero status as failure, and relaunches the current executable. It has no Windows-specific previous-package retention, explicit agent replacement, health handshake or post-install rollback. It also does not distinguish cancellation from MSI success requiring reboot (3010). These are source-backed gaps against D-032 and the existing Phase 16 update gate, not evidence of a particular failed update.
- Ordinary MSI removal is distinct from the already owner-approved Phase 19 complete local uninstall. The latter previews permanent data loss, requires typed confirmation, stops/removes services, clears data/credentials and then invokes MSI package removal. Direct MSI removal currently has no equivalent MSC service coordination and must not silently become Phase 19's destructive cleanup.
- `allowDowngrades` is unspecified, and the installed Tauri CLI schema defaults it to true. The inspected old MSI's Upgrade table also permits older packages to replace newer ones. Repair/Modify are hidden from Installed Apps in the stock template; reopening an MSI can still expose generic maintenance dialogs. Both need deliberate behavior and wording.
- The active `.github/workflows/release.yml` stages the complete Windows payload, prepares the ICO, builds the MSI and publishes the existing artifact set/checksums/signed update metadata. Windows installers are currently unsigned. Signed update metadata does not provide an Authenticode publisher identity for Windows/UAC. Signing enrollment is a separate owner decision, not an assumed prerequisite of this refinement.

**Primary-source references:** [Tauri CLI 2.11.4 MSI template](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-bundler/src/bundle/windows/msi/main.wxs), [WiX preparation dialog](https://github.com/wixtoolset/wix3/blob/develop/src/ext/UIExtension/wixlib/PrepareDlg.wxs), [Tauri Windows installer/prerequisite options](https://v2.tauri.app/distribute/windows-installer/), and [Microsoft MSI result codes](https://learn.microsoft.com/en-us/windows/win32/msi/error-codes). The local CLI package reports 2.11.4; preserve the lockfile-selected version when deriving a maintained template.

**Proposed experience:** Retain the native MSI installer and use a small, maintained MSC WiX template based on the locked Tauri CLI's packaging contract. Native controls, keyboard navigation, system text and readable contrast take precedence over imitating the app's dark interface. Use the existing MSC mark and restrained neutral artwork only where it helps identification; no decorative gradients, generic marketing panels or baked-in text. Read `antiAIslop.md` before UI work.

| Screen | What the user sees and can do |
|---|---|
| Preparation | Immediately readable “Preparing MSC 2 Setup” and a short explanation of installation checks. No empty interactive-looking page. If the stock modeless preparation dialog cannot paint reliably, suppress that transient dialog rather than adding a delay or pretending Next works before checks finish. |
| Welcome | “Welcome to MSC 2”; a short description of Minecraft server management; what this installer includes; Next and Cancel. Distinguish a new installation from replacing an existing version. |
| Installation options | Destination folder, clear machine-wide installation scope, Start-menu entry and optional desktop shortcut. Explain that the `msc` terminal command is included and becomes available in new terminals. Local hosting is configured on first launch. |
| Ready to install/update | Actual destination and current/target version, required Windows administrator approval, WebView2 requirement if missing, and any local-server stop requirement. Install/Update starts changes only after this review. |
| Progress | Name the real phase: application files, required WebView2 runtime, local-agent coordination when applicable, or recovery. Use Windows Installer progress without invented percentages. Cancel where safely supported; clearly explain any brief commit/rollback interval where cancellation cannot apply immediately. |
| Completion | Accurate installed/updated result; Launch MSC 2 option; first-launch service setup guidance for new local hosts; explicit reboot requirement or recovery report when applicable. Do not claim the agent is Running simply because the MSI succeeded. |
| Maintenance/removal | Repair installed application files or remove this package with clear consequences. Ordinary MSI removal preserves Minecraft worlds/backups/settings and does not promise complete cleanup. Explain the separate destructive Settings/CLI uninstall flow. |

**Draft Welcome copy:** “MSC 2 helps you create and manage Minecraft Java and Bedrock servers on computers you own. This installer includes the desktop app, background-agent files, and the msc terminal command. After installation, open MSC 2 to set up local hosting or connect to another computer.” Add version/scope separately, and do not imply that Bedrock is supported on every possible remote host.

**Recommended boundaries for review:**

- Keep initial service installation on first launch under D-011. Its UAC and Windows-account password prompts stay in the existing local setup flow; the MSI should explain them, not duplicate them or collect service passwords into MSI properties/logs. A remote-only desktop must not create a local service just by installing.
- Keep `msc` on PATH as a supported capability under D-040. Start-menu registration is default; propose an optional desktop shortcut instead of the stock unconditional shortcut. Avoid an unnecessary feature-selection tree.
- Propose blocking ordinary accidental downgrades, while supporting deliberate restoration of a verified prior version in the coordinated updater. Define that recovery route before changing downgrade policy; do not break rollback by simply setting a flag. This is a proposal for Cameron's review, not a new approved decision.
- Direct MSI repair/removal/upgrade must inspect the actual local service installation and its ownership. A headless service owned by a separate installation must not be stopped, overwritten or removed. Preserve the installing account even if a different administrator approves UAC. Any unresolved ownership blocks the affected service action with an explanation.
- Present graceful local Minecraft shutdown explicitly before a coordinated update/removal; do not force-kill servers or perform lifecycle work when merely entering Welcome. Preserve prior running/stopped state and boot configuration in recovery. Remote agents are never targets.
- Ordinary MSI removal removes package-owned files/shortcuts/PATH and detaches only its verified owned local service, retaining managed data. Do not call `msc uninstall --danger` from MSI actions. Phase 19's separate confirmed full removal continues to work, including when the service has already been removed by its worker.
- Use the default small online WebView2 strategy with explicit connectivity/error messaging for now. Do not silently grow every download with the offline runtime. Missing runtime and network failure require honest completion/error states, not a blank progress page or successful launch claim.

**Execution order and acceptance:** P16.44 establishes the exact-package baseline before P16.45 changes UI. P16.46 resolves identity/payload/prerequisite choices. P16.47–P16.49 handle lifecycle/removal/update recovery. P16.50 consolidates owner-observed acceptance against one exact candidate. Each future step is one commit containing its plan update; it remains awaiting Cameron's verification after implementation. Build once when a fresh native binary is necessary, then use Tauri's bundle-only command for dialog/package changes. No repeated release builds, new CI/release gates or automatic test execution. An essential new regression, if needed during execution, must name its concrete missing risk, controlled inputs and expected runtime under the workflow policy.

### P18.40 — Plan complete server transfers

**Status:** Plan written; awaiting Cameron's review. Cameron changed this conversation to PLAN only before implementation began (2026-10-07); the earlier BATCH EXECUTE authorization is superseded.
**Files:** `docs/msc2/rolling-plan.md` only.
**What:** Record P18.41–P18.45 for complete selected-server transfers, with remote-to-local Tectonic acceptance as the first required outcome. No application code, tests, builds, agent installation, releases or live server actions in this conversation.
**Verify:** `git show --stat --oneline HEAD`
**Batch:** P18.40 only (PLAN). Future execution requires a new owner instruction naming its range.
**Commit:** `P18.40: plan complete server transfers`

**Owner-required outcome:** From the desktop connected to the Xubuntu agent, select Tectonic and transfer the entire server to the local agent. On Fedora, macOS or Windows, switch to the local agent, select Tectonic in the normal server dropdown, repoint Java and review port allocation, then manage it through the existing MSC screens. The same transfer mechanism supports other paired destinations and the reverse direction; direct agent-to-agent network reachability is not required. Java-to-Bedrock conversion is the motivation, but conversion implementation is outside these transfer steps.

**Product flow:** Manage Servers → Transfer → choose server and destination (default: This computer for a remote source) → review size, destination space and stop requirement → Stop and transfer, or Transfer if already stopped → byte/file progress with Cancel → completion showing Open on destination and Review Java and ports. Stop is graceful and explicit; no forced stop or automatic destination start. Closing the sheet or navigating between hosts preserves the transfer/progress. The copied server receives a new destination identity and a noncolliding folder; the source remains available. Deleting the source is a separate existing action after owner verification.

**Architecture:** The source agent inventories and reads the server; the destination agent stages, validates and registers it. The native desktop relays bounded chunks through the two existing authenticated connections. File bytes never become a whole-world browser buffer or a desktop temporary archive. The source needs only small session metadata; the destination needs approximately one complete server copy plus small metadata and reserved headroom, because finalization renames staging on the same filesystem. Size means the entire server, including backups and inactive world slots, so Tectonic's 13 GB world is a lower bound. Show conversion scratch-space needs separately from transfer space where already known; do not imply that transfer capacity guarantees conversion capacity.

**Complete-server boundary:** Include all ordinary files and empty directories inside the selected server: live Java dimensions or Bedrock worlds; world slots and their profiles/pack state; backups; mods/plugins; loader libraries and launch files; resource/behavior packs; configuration, player data and existing logs; and MSC server metadata, component provenance and modpack identity. Preserve unknown files and config keys. Map only explicit server-owned references, including a configured Java server JAR outside the server directory; do not follow arbitrary external paths. The destination's Java installation, OS service, agent identity and host credentials remain destination-owned. Re-root server-owned paths and identify source-specific helper/account settings for review without copying agent pairing, keychain/DPAPI entries, or host helper credentials. Check for copied third-party native executables and absolute loader paths: preserve their files but report platform dependencies accurately. A Bedrock copy across operating systems uses the destination's supported runtime/provisioning path; copying a Linux executable alone does not make a Windows/macOS server launchable.

**Transfer contract:** Versioned capability discovery; explicit source/destination/server identity; inventory with relative paths, lengths and file digests; credential-bound sessions; bounded chunk sizes; idempotent acknowledged writes; progress, cancellation, completion and cleanup. Validate traversal, symlink escape, Windows reserved filenames, case collisions and path limits before copying. Reject unsupported entries with their paths and an actionable explanation, never silently omit them. Refuse running-server reads until graceful stop completes. Hold existing server-operation exclusivity during the source session and destination commit, including start, settings/file edits, world operations, backups and host reset. Inspect actual admission paths rather than assuming every edit already uses the journal. Session idle expiry must refresh during reads, hashing and progress, so a large server cannot expire while working normally.

**Integrity and interruption:** Hash files with bounded reads, validate completed destination files and total inventory, and verify that the stopped source has not changed before finalization. Detect same-size edits; length alone is insufficient. Retain destination checkpoints so transient disconnects and desktop/agent restarts can resume only after authenticating both hosts and confirming the same source inventory. Changed sources require a new session; do not splice two snapshots. Cancel removes only this session's unpublished staging and releases source locks. On restart, reconcile unfinished sessions and journals explicitly. A failed transfer cannot appear as a usable server in the dropdown. Rename and configuration registration need recoverable commit bookkeeping: failure between those operations must neither lose received data nor register duplicates. Report cleanup failures and reclaim interrupted staging through an explicit owned action.

**Observed gaps:** Existing `POST /v1/servers/export` stages an archive on the source and exports every configured server. `TransferSheet.svelte` downloads it as one byte array and imports from an agent-local path; it does not implement connected-host transfer. The existing transfer importer copies a selected file/subdirectory list and contains best-effort restoration paths, so it cannot be reused unchanged as proof of a complete, verified transfer. The native desktop already has per-host credentials, local bootstrap and managed SSH forwarding. Preserve those security boundaries and keep both transport sessions alive during relay. The MSC 1 source path supplied by AGENTS.md is absent on this machine; use the existing port and format documentation as evidence, and do not claim a fresh Swift-source comparison.

**Feature acceptance gate:** Cameron transfers the real stopped Tectonic server from Xubuntu to the local Fedora agent without a source archive or another 13 GB source copy; it appears in the local dropdown and its existing worlds, slots, components and settings are available. After Java selection and port review it starts through MSC. Record equivalent macOS and Windows destination acceptance before claiming those platforms physically verified. Exercise cancellation, insufficient destination space, interrupted/resumed transfer, source modification and interrupted finalization with controlled inputs. Preserve the original server. A successful build does not close this gate. The independent gate review belongs to the other agent, per the repository loop.

### P18.41 — Define transfer sessions and stream a stopped source server

**Status:** Planned.
**Files:** `docs/msc2/api-contract/openapi.json`; transfer DTOs under `crates/msc-api/src/dto/`; new `crates/msc-agent/src/routes/server_transfer.rs`; `crates/msc-agent/src/routes/mod.rs`, `main.rs`, `lifecycle.rs` and mutation admission callers as needed; `crates/msc-application/src/operations.rs`; capability discovery; generated client API types; this plan.
**What:** Freeze the complete-server contract above against current ConfigServer/world/loader behavior. Add capability/version discovery and source inventory/read/end routes scoped to the initiating credential and selected server. Stream file hashes and chunks without a source archive. Inventory external configured JARs through an explicit safe mapping. Validate portable names and reject unsupported entries. Add explicit graceful-stop consent and preserve stopped-state/source-operation guards through hashing and transfer. Persist session identity and source inventory for restart reconciliation and safe resume; audit existing settings/file/lifecycle mutation admission and close transfer races. Expose meaningful errors for older agents, permissions, changed sources and unsupported layouts. Update the supported-client capability record; retained host-local CLI session inspection/cancellation must respect D-040 rather than introducing remote CLI credentials.
**Verify:** `cargo clippy -p msc-api -p msc-agent --bin msc -- -D warnings`
**Batch:** P18.41 only; Cameron verifies before destination work unless a later explicit batch instruction authorizes P18.41–P18.45.
**Commit:** `P18.41: define complete server transfer sessions`

### P18.42 — Receive, verify and register complete servers

**Status:** Planned.
**Files:** `crates/msc-agent/src/routes/server_transfer.rs`; destination transfer persistence/adapters as needed under `crates/msc-application/src/` and `crates/msc-infrastructure/src/`; `crates/msc-agent/src/routes/lifecycle.rs` and startup recovery; API contract/DTOs as needed; this plan.
**What:** Add destination preflight, session creation, checkpoint queries, bounded idempotent chunk writes, final verification, cancel and recovery. Check full-server space plus headroom, and check writes for space exhaustion throughout. Write private staging on the destination server filesystem and reject symlink/case/path collisions. Resume only matching inventories. Preserve metadata and all server-owned files, re-root known paths, use a fresh destination ID and folder, and report Java/ports/helper/platform review items without silently reallocating ports. Commit with recoverable rename/registration bookkeeping through existing registration and world reconciliation. Make replayed completion return the same server ID. An incomplete server stays out of normal selection. Honor revocation, host reset, session expiry, restart and owned cleanup. Add only focused essential tests for concrete corruption, traversal, resume and duplicate-registration risks absent from existing coverage; record controlled inputs, purpose and expected runtime before adding them. No test execution is authorized by this plan.
**Verify:** `cargo clippy -p msc-agent --bin msc -- -D warnings`
**Batch:** P18.42 only; depends on P18.41 verification unless included in a newly authorized batch.
**Commit:** `P18.42: receive and register verified server transfers`

### P18.43 — Relay bounded transfers through the native desktop

**Status:** Planned.
**Files:** New `clients/desktop-web/src-tauri/src/server_transfer.rs`; `clients/desktop-web/src-tauri/src/lib.rs`; dependencies only if required; desktop platform/host connection bridge; this plan.
**What:** Resolve source and destination by stored authenticated host identity, bootstrapping the local agent through the existing native path. Keep both paired connections alive without requiring source-to-destination reachability. Reject same-agent transfers even when two saved addresses refer to it. Relay bounded file chunks with connection reuse, checksum validation, bounded transient retries and acknowledged checkpoint recovery. Keep credentials and data bytes out of webview state and errors. Expose transfer status/cancel/resume independently of the sheet's lifetime. Stop new writes promptly on cancellation; finish cleanup/release reliably and report uncertain finalization by querying the persisted destination result rather than creating another copy. Agent compatibility is checked before requesting source stop. Do not install or update a remote service through the transfer API.
**Verify:** `cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib -- -D warnings`
**Batch:** P18.43 only; depends on P18.41–P18.42 verification unless included in a newly authorized batch.
**Commit:** `P18.43: relay complete server transfers through desktop`

### P18.44 — Add the local-agent transfer and dropdown flow

**Status:** Planned.
**Files:** `clients/desktop-web/src/lib/sections/fleet/TransferSheet.svelte`, `ManageSheet.svelte`; transfer state/platform bridge; `clients/desktop-web/src/App.svelte` and host-state integration as needed; this plan.
**What:** Apply `antiAIslop.md` in the existing Transfer sheet. Select one complete server and paired destination, defaulting remote sources to This computer. Show inventory size, actual destination capacity, capability failures and review items before the explicit Stop and transfer action. Display stages, file/byte progress and cancellation; preserve transfer state across sheet closure/host navigation. Offer Resume for interrupted matching sessions. On success, refresh the destination's host-scoped server list; Open on destination switches host and selects the returned new server ID through the normal dropdown/selection path. Show Java and port review without automatic start or source deletion. Keep existing transfer-file import/export available. Distinguish transfer completion from platform/runtime startup readiness, and preserve any owner-required Java/port review. Guard late responses so source-host data cannot populate destination state.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P18.44 only; depends on P18.43 verification unless included in a newly authorized batch.
**Commit:** `P18.44: add connected host server transfer flow`

### P18.45 — Prepare runnable artifacts and record transfer acceptance

**Status:** Planned.
**Files:** Transfer instructions and acceptance record under `docs/msc2/`; existing packaging instructions only if needed; this plan. Build artifacts are generated locally, not committed.
**What:** Build the updated Fedora desktop and agent using the repository's current packaging path and provide exact artifact/install commands for Cameron's Xubuntu and local hosts. Record the required minimum source/destination agent versions and complete-server storage requirement. Document normal transfer, Java/port review, resume/cancel, staging cleanup and cross-platform native-runtime limits. Give Cameron concrete acceptance instructions for Tectonic and controlled fault cases; keep their outcomes pending until he reports them. Document macOS/Windows delivery through the existing build-only release pipeline if requested separately, preserving artifacts/checksums/signed metadata and adding no workflow gates. No publishing, remote installation, live Tectonic stop/copy or test-suite execution is implied by the current PLAN request. After authorized implementation, an unavailable platform or unpublished artifact remains a delivery limitation rather than a claim that that host is ready.
**Verify:** `cargo build -p msc-agent --bin msc`
**Batch:** P18.45 only; depends on P18.41–P18.44 verification unless included in a newly authorized batch. Physical acceptance remains Cameron's separate Verify move.
**Commit:** `P18.45: document complete server transfer acceptance`

### P16.42 — Publish v0.1.23 with Java map dependency recovery

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.23.md`, this plan.
**What:** Increment the latest remote/published tag v0.1.22 to v0.1.23 and synchronize owned release identities without changing registry dependency versions. Include P18.12's complete headless helper updates, missing Vantage recovery, verified version-matched Java assets and diagnostic errors. Push main and the new immutable tag through the unchanged four-platform build-only workflow, preserving nine artifacts, checksums and signed update metadata. Cameron explicitly requested implementation, commit pushes and the next release tag.
**Verify:** `gh release view v0.1.23 --json tagName,isPrerelease,assets,url`
**Batch:** P16.42 only.
**Commit:** `P16.42: prepare v0.1.23 release`

**Preparation:** Inspected the complete active release workflow and successful v0.1.22 publication. No release gates changed or tests run. Implementation formatting, Clippy and frontend checks recorded under P18.12; synchronized source identity and locked metadata checked before tagging. README retains the actually published v0.1.22 while introducing the new candidate. Recent successful release runs took 38–47 minutes. Ubuntu map acceptance remains Cameron's check.

### P16.39 — Replace the cancelled v0.1.22 candidate

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** `clients/desktop-web/src-tauri/Cargo.lock`, `docs/msc2/release/v0.1.22.md`, this plan.
**What:** Restore serde_repr 0.1.21 with its original checksum; the P16.38 version bump accidentally changed this unrelated registry package to nonexistent 0.1.22. Include P12.249 modern datapack support and retain the headless Linux fix. Cameron explicitly requested cancellation/removal and confirmed replacement of the unpublished v0.1.22 tag. Remove cancelled run 37177543797 and its old tag, then publish the corrected source using the unchanged build-only release workflow. No published v0.1.22 release or uploaded workflow artifacts existed when inspected.
**Verify:** `gh release view v0.1.22 --json tagName,isPrerelease,assets,url`
**Batch:** P16.39 only.
**Commit:** `P16.39: repair and replace v0.1.22 candidate`

**Checks:** Inspected all three completed failed platform logs: identical unavailable serde_repr dependency, before packaging. Fourth build cancelled. Restored the original registry version/checksum from the prior tag. Locked desktop metadata, synchronized application versions, formatting and diff whitespace checks passed. Application Clippy and addon regression compilation recorded in P12.249. No tests run or workflow gates changed. Nine artifacts, SHA256SUMS and signed update metadata remain required. Expected build time about 40 minutes based on recent successful runs; physical acceptance remains Cameron's verification. Tag replacement is a specifically owner-authorized exception, not a routine retry.

### P16.38 — Publish v0.1.22 with the headless Linux status fix

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.22.md`, this plan.
**What:** Increment the latest remote tag v0.1.21 to v0.1.22 and synchronize release identity. Include P12.248's headless Linux status correction. Integrate the two existing remote commits before publication and retain their changes. Push main and the new immutable tag through the existing four-platform build-only workflow, preserving nine artifacts, checksums and signed update metadata. Cameron explicitly requested the push and next tag.
**Verify:** `gh release view v0.1.22 --json tagName,isPrerelease,assets,url`
**Batch:** P16.38 only.
**Commit:** `P16.38: prepare v0.1.22 release`

**Preparation:** Inspected the complete release workflow and successful v0.1.21 run. No workflow changes or tests run. Fix formatting, Clippy and regression compilation passed before the version bump; release identity and locked metadata checked during preparation. Ubuntu live acceptance remains Cameron's verification.

### P16.37 — Publish v0.1.21 through the build-only release workflow

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.21.md`, this plan.
**What:** Increment v0.1.20 to v0.1.21 and synchronize locked release identity. Publish current main through the existing four-platform build-only workflow, preserving all nine desktop/headless artifacts, checksums, and signed update metadata. Monitor completion and inspect all failures before targeted fixes or retries; keep the tag immutable. Cameron explicitly authorized publication and release-error fixes. Preserve the uncommitted agent heading edit locally.
**Verify:** `gh release view v0.1.21 --json tagName,isPrerelease,assets,url`
**Batch:** P16.37 only.
**Commit:** `P16.37: prepare v0.1.21 release`

**Preparation:** Inspected the complete active release workflow, previous successful release, changes since v0.1.20, and configured signing-secret/public-key names. No workflow gates or tests added; no tests run. Physical acceptance remains Cameron's responsibility.

### P16.36 — Publish v0.1.20 through the build-only release workflow

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.20.md`, this plan.
**What:** Increment v0.1.19 to v0.1.20, synchronize locked release identity, and include the required Windows Tokio dependency correction in the Tauri lockfile. Publish current main through the existing release workflow only; preserve nine artifacts, checksums, and signed update metadata. Monitor until completion, inspect failures before any targeted retry, and keep the tag immutable. Cameron explicitly authorized publication and fixes for release failures. No CI/test workflows or test execution.
**Verify:** `gh release view v0.1.20 --json tagName,isPrerelease,assets,url`
**Batch:** P16.36 only.
**Commit:** `P16.36: prepare v0.1.20 release`

**Preparation:** Inspected the full release workflow and prior successful v0.1.19 run. Only release.yml is active. Signing-secret and public-key variable names are present. Locked Cargo metadata resolves for the agent and desktop manifests; version fields are synchronized. Publishing will run the existing four-platform build matrix; physical acceptance remains separate.

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

## Owner-requested release

### P16.35 — Prepare and publish v0.1.19

**Status:** Prepared; publication requested by Cameron, exact-artifact verification pending.
**Files:** `Cargo.lock`, `crates/msc-agent/Cargo.toml`, client package manifests/lockfile, Tauri package manifests/lockfile/configuration, bundle identity (source and static asset) and its existing assertion, `tools/release/stage-windows-agent.ps1`, `README.md`, `docs/msc2/release/v0.1.19.md`, this plan.
**What:** Increment the latest release tag from v0.1.18 to v0.1.19, preserving the current main-branch agent-home redesign, Cameron's subsequent adjustment and merged world-map work. Keep agent, desktop, frontend and locked package versions consistent. Include the two terrain helper executables and their existing license in Windows desktop staging, matching the other desktop/headless packaging paths. Push main and the new immutable tag once to trigger the existing build-only beta workflow, preserving all nine artifacts, checksums and signed update metadata. No workflow gates or tests added. The pre-existing Tauri Cargo.lock dependency edit remains uncommitted; stage only the release-version change from that file.
**Verify:** `gh release view v0.1.19 --json tagName,isPrerelease,assets,url`
**Batch:** P16.35 only.
**Commit:** `P16.35: prepare v0.1.19 release`

**Release preparation checks:** Locked Cargo metadata resolved for the agent and desktop packages; all release version fields agree at 0.1.19. Frontend production build and Windows staging PowerShell syntax inspection passed. Existing workflow signing-key variable/secret names are configured. No tests were run.


## Phase 19 — Complete local uninstall (owner-requested 2026-10-02)

**Planning state:** Cameron authorized implementation on 2026-10-02. P19.1 is implemented pending owner verification; later steps remain planned.

**Owner-approved intent:** Add **Uninstall MSC 2…** beside Reset in app settings and a local `msc uninstall --danger` command. Permanently remove this computer's MSC 2 agent services, managed servers/worlds/backups, MSC-owned helpers and runtimes, host/client data and credentials, caches/logs, installed command, and desktop app. Never contact or uninstall a saved remote agent. Running this flow on a remote computer means running its local MSC desktop or CLI there. Preserve MSC 1, source checkouts, separately installed Java/Tailscale/Docker, unrelated files, and OS-owned package caches.

**Confirmation contract:** Both interfaces must show the actual computer, server root(s), installation(s), and exact deletion list before execution. Desktop: review sheet, acknowledgement of permanent world/backup loss, exact typed `UNINSTALL MSC 2`, then a final destructive confirmation dialog. CLI: `--danger` enables the destructive flow but does not bypass review; print the same inventory and require the exact phrase interactively. `--confirm "UNINSTALL MSC 2"` is the explicit non-interactive equivalent, used only after a separate `--dry-run` inventory review; refuse redirected input without it. `--dry-run` performs no writes, elevation, service changes, or cleanup. A native command must enforce the confirmation independently of UI state. Local cleanup must still work while the selected desktop host is remote and while the local agent is missing/offline; no HTTP uninstall route.

**Downloaded installers:** MSC cannot prove ownership of every renamed/moved installer or its original download location. Include verified MSC release installers in known download/update locations and let the operator explicitly select additional installers. Show every selected file before confirmation, validate its package/bundle identity or signed release checksum, and remove only those files. No filename-only recursive disk search. Never promise that an unknown original DMG has been found. A mounted disk image needs explicit unmount handling; if another application holds it open, report the remaining file instead of claiming full removal. Do not delete MSI/OS package-manager caches directly.

**Completion contract:** Gracefully stop Minecraft and MSC-managed helpers before removing services/data. If shutdown or service removal fails, stop and report what remains. Remove the app through its OS installation mechanism: verified macOS bundle removal, Windows registered MSI uninstall, Linux owning package removal (or a verified standalone AppImage). Use a narrowly scoped detached continuation where the running app/command cannot remove itself. The continuation must validate its inventory again, propagate failures, and leave a readable result outside the deleted MSC trees; let the owner choose whether to retain that report. “Scheduled” is not “Uninstalled.” Reject unsupported/dev installations rather than deleting a source checkout. Clean up continuation files when finished.


### P16.44 — Publish v0.1.24 with world map controls

**Status:** Prepared; publication requested, owner artifact verification pending.
**Files:** Agent/desktop/frontend manifests and lockfiles, bundle identity and existing assertion, README, `docs/msc2/release/v0.1.24.md`, this plan.
**What:** Increment latest published v0.1.23 to v0.1.24, including biome, quality and depth controls. Push main and a new immutable tag once to trigger the existing build-only release workflow. Preserve all nine artifacts, helper bundles, checksums and signed metadata. No tests or gates added.
**Verify:** `gh release view v0.1.24 --json tagName,isPrerelease,assets,url`
**Batch:** P16.44 only.
**Commit:** `P16.44: prepare v0.1.24 release`

**Preparation checks:** Locked Cargo metadata resolved for agent and desktop; frontend production build and whitespace check passed with existing warnings. Headless packaging/installer inspection confirms both terrain helpers are bundled and installed. No tests run.
