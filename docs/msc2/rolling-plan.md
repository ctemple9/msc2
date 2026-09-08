# MSC 2 — Rolling Plan

> ## STATUS: Phase 12 client redesign and its post-phase corrections are complete. The planned Phase 13 full-screen terminal client is retired by D-034.
> **Next move:** Cameron reviews this reconciled plan and advances the repository to the next product phase. All prior Phase 12 verification entries are recorded as DONE in the archive. P12.163 is the current CI-maintenance step awaiting verification.

Previous phases and completed work remain in `rolling-plan-archive.md`. The archive is historical; this file contains only the current state and the next move.

---

## How this document works

This is the working state of the build. The vision documents say where MSC 2 is going; the port plan says the intended sequence; this file says where the repository actually is now.

Phases come from `msc2-port-plan.md`. Steps are written as work arrives rather than being invented in advance. Each step has a status, file scope, description, verification command, commit subject, and batch classification.

**Status is only moved to DONE by Cameron after he runs the Verify command.** An agent may set a step to awaiting verification and stop.

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

## Historical records

The pre-reconciliation Phase 12 working plan is preserved in `rolling-plan-archive.md` under “Reconciliation snapshot — 2026-09-07”. Its original step text and amendment history remain available there, with verified entries normalized to DONE per Cameron’s confirmation. The original file is also recoverable from commit `c00964d`.

### P12.121 — Reconcile the rolling plan
**Status:** DONE
**Files:** `docs/msc2/rolling-plan.md`, `docs/msc2/rolling-plan-archive.md`
**What:** Reduce the active rolling plan to one truthful current status and one unambiguous next step. Preserve the pre-reconciliation Phase 12 plan in the archive, normalize all previously verified entries to DONE, retain the retired Phase 13 and client-retirement history, and leave no product source or generated artifact changed.
**Verify:** `git diff --check && test "$(grep -c '^### P12\\.' docs/msc2/rolling-plan.md)" -eq "$(grep '^### P12\\.' docs/msc2/rolling-plan.md | sed -E 's/^### ([^ ]+).*/\\1/' | sort -u | wc -l)" && grep -q 'Reconciliation snapshot — 2026-09-07' docs/msc2/rolling-plan-archive.md && test "$(grep -c '^\\*\\*Status:\\*\\* awaiting verification' docs/msc2/rolling-plan.md)" -eq 1`
**Commit:** `P12.121: reconcile the rolling plan`
**Batch:** solo

### P12.122 — Audit supported client surfaces
**Status:** DONE
**Files:** `docs/msc2/worlds/phase6-api.md`, `docs/msc2/client-capability-matrix.csv`, `docs/msc2/tui-retirement-audit.md`, `docs/msc2/rolling-plan.md`
**What:** Reconcile the live capability documentation with the retained MSC 2 product surfaces: Tauri desktop, desktop browser, and headless CLI. Update the historical Phase 6 matrix snapshot to the current nine-column checker, label retained former-iOS notes as historical compatibility evidence, and keep the TUI/mobile retirement records and third-party/gameplay uses of “mobile” intact. No retired client source, dependency, route, or release artifact is restored.
**Verify:** `git diff --check && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && test ! -e clients/ios && test ! -d crates/msc-agent/src/cli/tui && ! grep -RniE --binary-files=without-match 'ios_status|tui_status|ratatui|crossterm' crates clients tools .github docs/msc2/client-capability-matrix.csv docs/msc2/worlds/phase6-api.md --exclude-dir=node_modules --exclude-dir=target`
**Commit:** `P12.122: audit supported client surfaces`
**Batch:** solo

### P12.123 — Clean disposable generated artifacts
**Status:** DONE
**Files:** `docs/msc2/rolling-plan.md`; local ignored build output only
**What:** Remove disposable local build and cache output (`target`, desktop `dist`/`node_modules`, Tauri generated directories, Python `__pycache__`, and `.DS_Store`) while preserving the checked-in agent web bundle, `.claude/`, and ignored corpus data that may be intentional local evidence. This changes no product source or tracked artifact.
**Verify:** `git diff --check && test ! -d target && test ! -d clients/desktop-web/dist && test ! -d clients/desktop-web/node_modules && test ! -d clients/desktop-web/src-tauri/target && test ! -d clients/desktop-web/src-tauri/gen && test ! -d tools/phase4/__pycache__ && test ! -d tools/phase6/__pycache__ && test ! -d tools/phase8/__pycache__ && test ! -d tools/release/__pycache__ && test -d crates/msc-agent/web-ui/assets && test -z "$(find . -name .DS_Store -print -prune)"`
**Commit:** `P12.123: clean disposable generated artifacts`
**Batch:** solo

### P12.124 — Finish repository hygiene
**Status:** DONE
**Files:** `AGENTS.md`, `CLAUDE.md`, `docs/msc2/rolling-plan.md`
**What:** Restore the required byte-for-byte parity between the two agent-instruction copies and record the final hygiene audit. Confirm the hook path is configured, tracked build outputs and secret-like files are absent, and the working tree is clean. Historical documentation, ignored local corpus data, and the checked-in agent bundle remain untouched.
**Verify:** `cmp -s AGENTS.md CLAUDE.md && test "$(git config --get core.hooksPath)" = .githooks && test -x .githooks/commit-msg && ! git ls-files | grep -E '(^|/)(target|dist|node_modules|__pycache__|\.DS_Store)(/|$)' && ! git ls-files | grep -E '(^|/)(\.env($|\.)|.*\.(pem|p12|key|secret))' && git diff --check && test -z "$(git status --porcelain)"`
**Commit:** `P12.124: finish repository hygiene`
**Batch:** solo

### P12.127 — Make the Bedrock version picker real
**Status:** awaiting verification
**Files:** `crates/msc-infrastructure/src/bedrock_distribution.rs`, `crates/msc-api/src/dto/versions.rs`, `crates/msc-agent/src/routes/versions.rs`, `crates/msc-agent/src/routes/components.rs`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`, `clients/desktop-web/src/lib/sections/components/VersionPickerSheet.svelte`
**What:** Replace the Bedrock placeholder with a catalog-backed picker. Report the installed distribution instead of the Java-only version field, expose `Latest` plus exact Bedrock releases, preserve whether the server tracks latest or pins an exact version, show the Bedrock component row, and route an explicit approved selection through the existing checksum-verified staged installer. Selecting Latest when a newer release exists shows an approval prompt before downloading. Automatic latest reconciliation at server start remains a separate safety step.
**Verify:** `cargo check --workspace && cd clients/desktop-web && npm run check && npm run build`
**Commit:** `P12.127: make the Bedrock version picker real`
**Batch:** solo

### P12.128 — Repair Bedrock picker configuration-error reporting
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/routes/versions.rs`, `docs/msc2/rolling-plan.md`
**What:** Fix the compile error exposed by the first Tauri build of P12.127. The Bedrock version-change worker now maps its configuration mutation result explicitly, reporting persistence failures without requiring `TryMutateError<()>` to implement `Display`.
**Verify:** `cargo check --workspace && cd clients/desktop-web && npx tauri dev`
**Commit:** `P12.128: repair Bedrock picker configuration-error reporting`
**Batch:** solo

### P12.129 — Update the desktop top-bar byline
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/components/shell/TopBar.svelte`, `docs/msc2/rolling-plan.md`
**What:** Apply the owner-requested top-bar copy change from `TempleTech` to `ctemple9h`.
**Verify:** `git diff --check && grep -q 'by ctemple9h' clients/desktop-web/src/lib/components/shell/TopBar.svelte`
**Commit:** `P12.129: update the desktop top-bar byline`
**Batch:** solo

### P12.130 — Correct the desktop top-bar byline
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/components/shell/TopBar.svelte`, `docs/msc2/rolling-plan.md`
**What:** Correct the owner-requested top-bar byline from `ctemple9h` to `ctemple9`.
**Verify:** `git diff --check && grep -q 'by ctemple9' clients/desktop-web/src/lib/components/shell/TopBar.svelte && ! grep -q 'by ctemple9h' clients/desktop-web/src/lib/components/shell/TopBar.svelte`
**Commit:** `P12.130: correct the desktop top-bar byline`
**Batch:** solo

### P12.131 — Repair recurring CI workflow failures
**Status:** awaiting verification
**Files:** `crates/msc-application/tests/addons.rs`, `crates/msc-agent/src/cli/update.rs`, `crates/msc-agent/tests/support/bedrock_smoke.rs`, `docs/msc2/rolling-plan.md`
**What:** Align the existing add-on fixture with the approved mutable-imported-pack behavior, prevent the offline Bedrock production smoke from attempting a live manifest lookup by seeding its version marker, and make the Windows update-install branch compile without unreachable-code warnings. This addresses the recurring Phase 8 smoke failure, the Ubuntu Bedrock smoke timeout, and the Windows clippy failure without weakening any workflow gate.
**Verify:** `git diff --check && cargo fmt --all -- --check && cargo check --workspace && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard`
**Commit:** `P12.131: repair recurring CI workflow failures`
**Batch:** solo

### P12.132 — Complete mutable modpack CI fixtures
**Status:** awaiting verification
**Files:** `crates/msc-application/tests/addons.rs`, `docs/msc2/rolling-plan.md`
**What:** Update the remaining add-on and plugin mutation fixtures that still expect pack-managed servers to reject staged installs, updates, toggles, removals, and direct plugin-source downloads. The implementation already follows P12.53 and permits those ordinary mutations; the fixtures now assert the successful disk changes instead.
**Verify:** `git diff --check && cargo fmt --all -- --check && cargo check --workspace && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format`
**Commit:** `P12.132: complete mutable modpack CI fixtures`
**Batch:** solo

### P12.133 — Correct direct plugin fixture path assertion
**Status:** awaiting verification
**Files:** `crates/msc-application/tests/addons.rs`, `docs/msc2/rolling-plan.md`
**What:** Replace the remaining direct plugin-source fixture's `Path::ends_with(".jar")` check with an exact path assertion. `Path::ends_with` compares path components rather than filename suffixes, so the fixture rejected the correct `/server/plugins/x.jar` destination on every operating system.
**Verify:** `git diff --check && cargo fmt --all -- --check && cargo check --workspace && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format`
**Commit:** `P12.133: correct direct plugin fixture path assertion`
**Batch:** solo

### P12.134 — Restore the required workspace regression gate
**Status:** awaiting verification
**Files:** `.github/workflows/ci.yml`, `docs/msc2/rolling-plan.md`
**What:** Restore the workspace-wide `cargo nextest` regression step that the Phase 9 gate checker requires. A prior CI-focused change removed it while leaving `tools/phase9/phase9-check.py`'s tri-platform gate contract unchanged, so every OS correctly rejected the workflow as incomplete before the Phase 9 smoke could run.
**Verify:** `git diff --check && python3 tools/phase9/phase9-check.py --gate`
**Commit:** `P12.134: restore the required workspace regression gate`
**Batch:** solo

### P12.135 — Reconcile the workspace regression suite with current Phase 12 behavior
**Status:** awaiting verification
**Files:** `.github/workflows/ci.yml`, `crates/msc-agent/tests/cli_provisioning.rs`, `crates/msc-agent/tests/bedrock_production_surfaces.rs`, `crates/msc-agent/src/routes/settings.rs`, `crates/msc-agent/src/routes/versions.rs`, `crates/msc-api/tests/phase11_auth_conformance.rs`, `crates/msc-domain/tests/player_nbt.rs`, `docs/msc2/rolling-plan.md`
**What:** Keep the required workspace regression suite in CI, serialize its test execution to protect shared production Bedrock fixtures from concurrent timeout pressure, and reconcile existing assertions with approved Phase 12 changes: D-035's removal of global template commands, P12.127's real Bedrock version picker/change flow, the settings rejection result, the final Phase 11 wording, and the intentionally local-only player capture.
**Verify:** `git diff --check && cargo fmt --all -- --check && cargo check --workspace && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && python3 tools/phase9/phase9-check.py --gate`
**Commit:** `P12.135: reconcile the workspace regression suite with current phase 12 behavior`
**Batch:** solo

### P12.136 — Finish platform-aware workspace regression fixtures
**Status:** awaiting verification
**Files:** `crates/msc-agent/tests/bedrock_production_surfaces.rs`, `crates/msc-api/tests/phase11_auth_conformance.rs`, `docs/msc2/rolling-plan.md`
**What:** Make the Bedrock version-change fixture honor the approved Apple Silicon unavailable-runtime contract while continuing to assert the real 200/operation response on available runtimes, and correct the Phase 11 fixture to match the document's actual line break without a patch-marker character.
**Verify:** `git diff --check && cargo fmt --all -- --check && cargo check --workspace && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format`
**Commit:** `P12.136: finish platform-aware workspace regression fixtures`
**Batch:** solo

### P12.137 — Match the Phase 11 authentication wording fixture
**Status:** awaiting verification
**Files:** `crates/msc-api/tests/phase11_auth_conformance.rs`, `docs/msc2/rolling-plan.md`
**What:** Correct the final Phase 11 conformance fixture to match the approved document's exact wording: same-machine desktop bootstrap uses local IPC, not an unauthenticated loopback HTTP exception.
**Verify:** `git diff --check && cargo fmt --all -- --check && cargo check --workspace && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format`
**Commit:** `P12.137: match the phase 11 authentication wording fixture`
**Batch:** solo

### P12.138 — Match the Phase 11 bootstrap fallback fixture
**Status:** awaiting verification
**Files:** `crates/msc-api/tests/phase11_auth_conformance.rs`, `docs/msc2/rolling-plan.md`
**What:** Replace the last retired phrase in the Phase 11 design conformance fixture with the current documented behavior: when local bootstrap cannot prove package identity, the desktop falls back to the ordinary remote-pairing code flow.
**Verify:** `git diff --check && cargo fmt --all -- --check && cargo check --workspace && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format`
**Commit:** `P12.138: match the phase 11 bootstrap fallback fixture`
**Batch:** solo

### P12.139 — Update the Phase 10 production wiring guard
**Status:** awaiting verification
**Files:** `tools/phase10/phase10-production-check.py`, `docs/msc2/rolling-plan.md`
**What:** Update the Phase 10 static production-wiring guard to require the current lifecycle constructor, which includes the shared notification state added by the retained activity-stream surface. Keep the guard fail-closed while matching the production composition now used by `main.rs`.
**Verify:** `git diff --check && python3 tools/phase10/phase10-production-check.py --check`
**Commit:** `P12.139: update the phase 10 production wiring guard`
**Batch:** solo

### P12.140 — Format the desktop version picker
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/components/VersionPickerSheet.svelte`, `docs/msc2/rolling-plan.md`
**What:** Apply the repository’s Prettier formatting to the existing Bedrock-aware version picker so the shared desktop-client validation passes on macOS, Windows, and Linux. This is formatting-only; behavior and API wiring are unchanged.
**Verify:** `cd clients/desktop-web && npm run format:check`
**Commit:** `P12.140: format the desktop version picker`
**Batch:** solo

### P12.141 — Reconcile stale desktop client CI fixtures
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/bundle-identity.test.ts`, `clients/desktop-web/tests/agent-install/agent-install.test.ts`, `clients/desktop-web/tests/e2e/tauri-linux/native-renderer.test.ts`, `clients/desktop-web/tests/playit/playit-setup.test.ts`, `clients/desktop-web/tests/screens/first-launch-reset.test.ts`, `clients/desktop-web/tests/screens/server-editor.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Align the existing desktop-client source assertions with the current Phase 12 copy, layout, bundle identity, Playit ownership, and editor structure. Correct the Linux Tauri smoke’s test-runner import so WebdriverIO supplies the Mocha globals at runtime. No new tests are added.
**Verify:** `cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.141: reconcile stale desktop client CI fixtures`
**Batch:** solo

### P12.142 — Give the production Bedrock smoke a hosted-runner budget
**Status:** awaiting verification
**Files:** `crates/msc-agent/tests/bedrock_production_smoke.rs`, `docs/msc2/rolling-plan.md`
**What:** Increase the existing production smoke’s bounded operation and lifecycle polling window from 20 to 60 seconds. The full workspace suite now runs serially before this smoke, and hosted runners can leave a valid Bedrock create operation running just beyond the old deadline; the smoke still fails closed if it does not reach a terminal state within the larger bounded budget.
**Verify:** `git diff --check && cargo fmt --all -- --check && cargo check --workspace`
**Commit:** `P12.142: give the production bedrock smoke a hosted-runner budget`
**Batch:** solo

### P12.143 — Declare the Linux Tauri test runner types
**Status:** awaiting verification
**Files:** `clients/desktop-web/tsconfig.json`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Declare the existing Mocha globals used by the WebdriverIO Linux Tauri smoke in the shared client type configuration, while keeping runtime ownership with WebdriverIO’s injected globals. Remove the two unused setup-screen CSS selectors reported by Svelte’s checker.
**Verify:** `cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.143: declare the linux tauri test runner types`
**Batch:** solo

### P12.144 — Reconcile Windows Bedrock CI fixtures
**Status:** awaiting verification
**Files:** `crates/msc-agent/tests/bedrock_production_cli.rs`, `crates/msc-agent/tests/bedrock_production_lifecycle.rs`, `crates/msc-agent/tests/bedrock_production_surfaces.rs`, `crates/msc-api/tests/phase11_auth_conformance.rs`, `docs/msc2/rolling-plan.md`
**What:** Make the existing Bedrock production fixtures match each platform's real contract: accept the Windows native runtime's available start response, keep unavailable-runtime assertions on hosts where that capability is genuinely unavailable, decode CLI errors across Windows output streams, and normalize documentation line endings before checking the Phase 11 wording.
**Verify:** `git diff --check && cargo fmt --all -- --check && cargo check --workspace && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format`
**Commit:** `P12.144: reconcile windows bedrock CI fixtures`
**Batch:** solo

### P12.145 — Bound Windows unavailable Bedrock production fixtures
**Status:** awaiting verification
**Files:** `crates/msc-agent/tests/bedrock_production_cli.rs`, `crates/msc-agent/tests/bedrock_production_lifecycle.rs`, `docs/msc2/rolling-plan.md`
**What:** Keep the production-composition Bedrock fixtures bounded on Windows when no native runtime is installed. The dedicated Windows CLI and route fixtures already assert the structured unavailable-start behavior; these two cross-platform fixtures now validate the shared capability and read-only surfaces without waiting on a real Windows provisioning path.
**Verify:** `git diff --check && cargo fmt --all -- --check && cargo check --workspace && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format`
**Commit:** `P12.145: bound windows unavailable bedrock production fixtures`
**Batch:** solo

### P12.146 — Separate unit and native Tauri test runners
**Status:** awaiting verification
**Files:** `clients/desktop-web/vite.config.ts`, `crates/msc-agent/tests/bedrock_production_lifecycle.rs`, `crates/msc-platform-windows/tests/job_object.rs`, `docs/msc2/rolling-plan.md`
**What:** Keep the WDIO-only native Tauri suite out of Vitest's unit-test collection, match the Windows Bedrock fixture's actual `provisioning_required` capability state, and give the existing Windows graceful-stop process check a larger hosted-runner wait budget without changing what it asserts.
**Verify:** `git diff --check && cargo fmt --all -- --check && cargo check --workspace && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.146: separate unit and native tauri test runners`
**Batch:** solo

### P12.147 — Sequence browser reconnect harness responses
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `docs/msc2/rolling-plan.md`
**What:** Make the browser contract harness return a successful initial status for normal and fresh-profile workflows, then reserve its simulated reconnect-pending response for the reconnect test's second status request. The prior counter returned `503` before the shared shell could render, causing the macOS browser smoke to fail across both engines.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.147: sequence browser reconnect harness responses`
**Batch:** solo

### P12.148 — Scope browser reconnect simulation to its workflow
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `docs/msc2/rolling-plan.md`
**What:** Restrict the contract harness's simulated reconnect response to the existing reconnect workflow's explicit test header. The prior user-agent counter was shared by multiple Playwright contexts, so unrelated browser tests could receive the deliberate `503` response and never render the client shell.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.148: scope browser reconnect simulation to its workflow`
**Batch:** solo

### P12.149 — Isolate browser host-setup fixture state
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `docs/msc2/rolling-plan.md`
**What:** Keep the existing host-setup fixture state in its per-browser-context cookie instead of a process-wide user-agent map. Parallel Playwright contexts share the user-agent, so the previous map let one fresh-profile test force unrelated workflows back into the setup gate.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.149: isolate browser host-setup fixture state`
**Batch:** solo

### P12.150 — Surface hosted browser startup failures
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `clients/desktop-web/tests/e2e/browser/reset-recovery.spec.ts`, `docs/msc2/rolling-plan.md`
**What:** Add diagnostics to the existing browser workflow specs for page-level JavaScript errors and failed network requests. The hosted Chromium and WebKit jobs both lose the shared shell while the same contract harness succeeds locally, so the next run needs to expose the browser-side startup failure before the diagnostics are removed or converted into the smallest fix.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.150: surface hosted browser startup failures`
**Batch:** solo

### P12.151 — Reconcile browser smoke selectors with the shared shell
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `clients/desktop-web/tests/e2e/browser/reset-recovery.spec.ts`, `docs/msc2/rolling-plan.md`
**What:** Align the existing browser workflow checks with the current Phase 12 shared shell: server sections are a tablist, handbook is opened from the Help & guides action, the server picker owns Manage…, reconnect is represented by the agent setup screen, and the management sheet must be closed before changing tabs. Remove the temporary diagnostics now that the failures are known stale selectors rather than browser startup errors.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.151: reconcile browser smoke selectors with the shared shell`
**Batch:** solo

### P12.152 — Isolate browser harness state and align onboarding anchors
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `docs/msc2/rolling-plan.md`
**What:** Keep the broadcast-helper fixture isolated per browser context so Chromium and WebKit cannot change each other's setup state; point the onboarding fixture at the visible server picker that users must open before choosing Manage; and update the existing browser workflow to finish the current tour dialog and avoid the removed Home heading.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.152: isolate browser harness state and align onboarding anchors`
**Batch:** solo

### P12.153 — Correct isolated broadcast fixture response
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `docs/msc2/rolling-plan.md`
**What:** Return the fixed broadcast-helper filename from the isolated download fixture instead of the removed process-wide state object. The stale reference terminated the browser harness after the first download request, making the remaining browser cases appear as connection failures.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.153: correct isolated broadcast fixture response`
**Batch:** solo

### P12.154 — Reconcile browser workflow actions with current fleet controls
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `docs/msc2/rolling-plan.md`
**What:** Match the existing browser smoke to the current Add Server and Manage Servers controls: the import path is labeled “Import or Create from Modpack,” and server removal is opened through More actions and confirmed with Remove from Controller rather than the retired Delete server alert.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.154: reconcile browser workflow actions with current fleet controls`
**Batch:** solo

### P12.155 — Reconcile browser world import workflow with current worlds controls
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `docs/msc2/rolling-plan.md`
**What:** Match the browser smoke to the current wizard’s accessible path-card name and the current Worlds section flow: open the header actions menu, choose Import ZIP…, select the existing fixture through the sheet’s file chooser, and complete the staged world import. Remove the retired Stage file and world-export assertions that no longer exist in the current client.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.155: reconcile browser world import workflow with current worlds controls`
**Batch:** solo

### P12.156 — Close browser workflow sheets before handbook assertion
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `docs/msc2/rolling-plan.md`
**What:** Close the Add Server wizard and its parent Manage Servers sheet after the onboarding tour’s final card. The tour completion only removes the coach mark; the underlying sheets remain open, so the existing handbook assertion must first return the browser to the page that the workflow opened.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.156: close browser workflow sheets before handbook assertion`
**Batch:** solo

### P12.157 — Reconcile browser handbook reader selector
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `docs/msc2/rolling-plan.md`
**What:** Point the fresh-profile browser workflow at the handbook article’s current `.reader` container. The previous `.topic-reader` selector belonged to an older handbook implementation, so Chromium and WebKit completed the workflow but could not find the final Overview heading.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.157: reconcile browser handbook reader selector`
**Batch:** solo

### P12.158 — Reconcile browser handbook restart label
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `docs/msc2/rolling-plan.md`
**What:** Use the current handbook action label, “Restart the guide,” when reopening the onboarding tour. The browser workflow had already returned to the current Overview article; only its final restart-button name was retired.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.158: reconcile browser handbook restart label`
**Batch:** solo

### P12.159 — Open onboarding view before browser tour restart
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `docs/msc2/rolling-plan.md`
**What:** Select the handbook’s current Onboard tab before clicking “Restart the guide.” The workflow had reached the correct Overview article, but the restart action belongs to the Onboard view rather than the Handbook reader.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.159: open onboarding view before browser tour restart`
**Batch:** solo

### P12.160 — Refresh onboarding state when opening handbook tab
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/handbook/HelpSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Re-read the persisted onboarding flag when the Handbook’s Onboard tab opens. The global first-launch overlay correctly marked the tour complete, but the already-mounted Handbook retained its earlier in-memory state and hid the current “Restart the guide” action.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.160: refresh onboarding state when opening handbook tab`
**Batch:** solo

### P12.161 — Synchronize handbook onboarding completion
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/FirstLaunchGate.svelte`, `clients/desktop-web/src/lib/sections/handbook/HelpSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Notify the mounted Handbook when the global first-launch tour completes, then refresh its persisted onboarding state. The browser workflow exposed that the tour correctly wrote local storage while the already-mounted Handbook retained the earlier in-memory state and hid “Restart the guide” on all hosted runners.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.161: synchronize handbook onboarding completion`
**Batch:** solo

### P12.162 — Notify handbook when tour finish advances
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/FirstLaunchGate.svelte`, `docs/msc2/rolling-plan.md`
**What:** Route the final tour card’s normal “Finish” action through the same completion notifier as skip and asynchronous completion. The hosted browser trace showed the final action used `advance`, which persisted completion but bypassed the event that refreshes the mounted Handbook.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.162: notify handbook when tour finish advances`
**Batch:** solo

### P12.163 — Synchronize handbook setup completion
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/FirstLaunchGate.svelte`, `clients/desktop-web/src/lib/sections/handbook/HelpSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Notify the mounted Handbook when the global host setup wizard completes, then preserve that setup-complete state when the tour-complete event arrives. The browser trace showed the Handbook had refreshed the tour flag but still retained its initial `setupComplete: false`, so it remained in its setup branch and hid the restart action.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.163: synchronize handbook setup completion`
**Batch:** solo

### P12.164 — Normalize source line endings across hosted runners
**Status:** awaiting verification
**Files:** `.gitattributes`, `docs/msc2/rolling-plan.md`
**What:** Declare LF as the repository line ending for text files so Windows checkouts do not turn every formatted source file into CRLF and fail the shared client format check.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.164: normalize source line endings across hosted runners`
**Batch:** solo

### P12.165 — Reconcile Linux native renderer selectors
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/tauri-linux/native-renderer.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Update the existing Linux WebKitGTK smoke to use the current shared shell, handbook, fleet-management, and console controls. The hosted runner showed the native window was reached, but the harness still searched for retired `nav[aria-label="Sections"]` and `.application-shell` markup.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.165: reconcile Linux native renderer selectors`
**Batch:** solo

### P12.166 — Repair cross-platform client paths
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/tauri-linux/native-renderer.test.ts`, `clients/desktop-web/tools/verify-bundle-identity.mjs`, `clients/desktop-web/tools/package-agent-bundle.mjs`, `clients/desktop-web/tools/prepare-agent-dev.mjs`, `docs/msc2/rolling-plan.md`
**What:** Target the native renderer's explicit picker label and convert file URLs with `fileURLToPath`, so WebKitGTK text lookup and Windows production-bundle validation use the current UI and real platform paths.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.166: repair cross-platform client paths`
**Batch:** solo

### P12.167 — Repair hosted browser and native text harnesses
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `clients/desktop-web/tests/e2e/tauri-linux/native-renderer.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Convert the browser contract server's dist URL with `fileURLToPath` on Windows and read native-renderer assertions from DOM text content, because the hosted Windows server received a doubled drive prefix and WebKitGTK's WebDriver text endpoint returned an empty value for visible picker text.
**Verify:** `git diff --check && cd clients/desktop-web && npm run format:check && npm run check`
**Commit:** `P12.167: repair hosted browser and native text harnesses`
**Batch:** solo

### P12.168 — Bound hosted process and native view timing
**Status:** awaiting verification
**Files:** `crates/msc-platform-windows/tests/job_object.rs`, `clients/desktop-web/tests/e2e/tauri-linux/native-renderer.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Give the existing Windows Job Object fixture the same 60-second hosted-runner startup budget as its sibling process fixtures, and make the Linux native text helper poll visible matching DOM nodes without holding a stale element reference between render updates.
**Verify:** `git diff --check && cargo fmt --all -- --check && cd clients/desktop-web && npx prettier --check tests/e2e/browser/contract-harness.mjs tests/e2e/tauri-linux/native-renderer.test.ts && npm run format:check && npm run check`
**Commit:** `P12.168: bound hosted process and native view timing`
**Batch:** solo

### P12.169 — Preserve native host setup fixture state
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `docs/msc2/rolling-plan.md`
**What:** Keep host-setup state for the native Tauri origin in the deterministic contract harness because WebKitGTK does not retain the harness's cross-origin test cookie; browser tests continue using their cookie-backed state.
**Verify:** `git diff --check && cargo fmt --all -- --check && cd clients/desktop-web && npx prettier --check tests/e2e/browser/contract-harness.mjs tests/e2e/tauri-linux/native-renderer.test.ts && npm run format:check && npm run check`
**Commit:** `P12.169: preserve native host setup fixture state`
**Batch:** solo

### P12.170 — Fall back to native harness request identity
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `docs/msc2/rolling-plan.md`
**What:** Key the deterministic native host-setup override by the request origin when WebKitGTK sends one, and by its user-agent when the native cross-origin request omits `Origin`; the hosted native smoke can then retain the incomplete setup state across the refresh that follows its reset.
**Verify:** `git diff --check && cargo fmt --all -- --check && cd clients/desktop-web && npx prettier --check tests/e2e/browser/contract-harness.mjs tests/e2e/tauri-linux/native-renderer.test.ts && npm run format:check && npm run check`
**Commit:** `P12.170: fall back to native harness request identity`
**Batch:** solo

### P12.171 — Isolate native harness identity from browser fixtures
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `clients/desktop-web/tests/e2e/tauri-linux/native-renderer.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Register a user-agent fallback only when the Linux native smoke explicitly marks its host-setup reset request, so WebKitGTK can retain its fixture state without allowing browser workflow requests to inherit the native override.
**Verify:** `git diff --check && cargo fmt --all -- --check && cd clients/desktop-web && npx prettier --check tests/e2e/browser/contract-harness.mjs tests/e2e/tauri-linux/native-renderer.test.ts && npm run format:check && npm run check`
**Commit:** `P12.171: isolate native harness identity from browser fixtures`
**Batch:** solo

### P12.172 — Preserve native setup state across the desktop bridge
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `docs/msc2/rolling-plan.md`
**What:** Keep the native host-setup override as isolated harness state for the Linux smoke, because desktop API calls travel through Rust and cannot share the webview request's origin or user-agent; browser fixtures remain cookie-backed and unchanged.
**Verify:** `git diff --check && cargo fmt --all -- --check && cd clients/desktop-web && npx prettier --check tests/e2e/browser/contract-harness.mjs tests/e2e/tauri-linux/native-renderer.test.ts && npm run format:check && npm run check`
**Commit:** `P12.172: preserve native setup state across the desktop bridge`
**Batch:** solo

### P12.173 — Accept mixed native request origins
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `docs/msc2/rolling-plan.md`
**What:** Apply the isolated native host-setup state to every cookie-less request in the native-only harness process, because WebKitGTK can alternate between requests with and without an Origin while the browser fixture remains cookie-scoped.
**Verify:** `git diff --check && cargo fmt --all -- --check && cd clients/desktop-web && npx prettier --check tests/e2e/browser/contract-harness.mjs tests/e2e/tauri-linux/native-renderer.test.ts && npm run format:check && npm run check`
**Commit:** `P12.173: accept mixed native request origins`
**Batch:** solo

### P12.174 — Reset native host setup before origin lookup
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `docs/msc2/rolling-plan.md`
**What:** Make the native host-setup override take precedence over stale browser origin state and clear the browser-only map when native setup begins, so the native onboarding smoke receives the incomplete state it requested.
**Verify:** `git diff --check && cargo fmt --all -- --check && cd clients/desktop-web && npx prettier --check tests/e2e/browser/contract-harness.mjs tests/e2e/tauri-linux/native-renderer.test.ts && npm run format:check && npm run check`
**Commit:** `P12.174: reset native host setup before origin lookup`
**Batch:** solo

### P12.175 — Narrow native smoke and split CI boundaries
**Status:** awaiting verification
**Files:** `.github/workflows/ci.yml`, `clients/desktop-web/package.json`, `clients/desktop-web/tests/e2e/tauri-linux/native-renderer.test.ts`, `tools/phase11/desktop-web-smoke.sh`, `tools/phase11/linux-webkitgtk-smoke.sh`, `docs/msc2/rolling-plan.md`
**What:** Remove the native renderer's duplicate long browser journey and onboarding-transition assertion, keep only native shell/layout/motion assertions, and split CI into independently rerunnable build, Rust regression, platform smoke, client validation, browser smoke, native desktop, and headless-link jobs. Browser jobs consume the shared frontend artifact instead of rebuilding it.
**Verify:** `git diff --check && cargo fmt --all -- --check && bash -n tools/phase11/desktop-web-smoke.sh tools/phase11/linux-webkitgtk-smoke.sh && cd clients/desktop-web && npx prettier --check package.json tests/e2e/tauri-linux/native-renderer.test.ts && npm run format:check && npm run check`
**Commit:** `P12.175: narrow native smoke and split ci boundaries`
**Batch:** solo

### P12.176 — Restore executable bit for staged smoke binary
**Status:** awaiting verification
**Files:** `.github/workflows/ci.yml`, `docs/msc2/rolling-plan.md`
**What:** Restore execute permission after downloading the shared headless artifact so Phase 6/7 platform smokes can launch the staged agent on Linux, macOS, and Windows runners.
**Verify:** `git diff --check && ruby -e "require 'yaml'; YAML.load_file('.github/workflows/ci.yml'); puts 'workflow YAML parses'"`
**Commit:** `P12.176: restore executable bit for staged smoke binary`
**Batch:** solo
