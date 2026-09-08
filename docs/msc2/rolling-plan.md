# MSC 2 — Rolling Plan

> ## STATUS: Phase 12 client redesign and its post-phase corrections are complete. The planned Phase 13 full-screen terminal client is retired by D-034.
> **Next move:** Cameron reviews this reconciled plan and advances the repository to the next product phase. All prior Phase 12 verification entries are recorded as DONE in the archive. P12.124 is the current final repository hygiene step awaiting verification.

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
