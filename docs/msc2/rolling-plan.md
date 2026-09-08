# MSC 2 — Rolling Plan

> ## STATUS: Phase 12 client redesign and its post-phase corrections are complete. The planned Phase 13 full-screen terminal client is retired by D-034.
> **Next move:** Cameron reviews this reconciled plan and advances the repository to the next product phase. All prior Phase 12 verification entries are recorded as DONE in the archive. P12.140 is the current CI-maintenance step awaiting verification.

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
