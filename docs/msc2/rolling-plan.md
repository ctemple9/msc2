# MSC 2 — Rolling Plan

**Current priority:** Repository presentation polish, requested by Cameron on 2026-10-30. Earlier phase history and acceptance records remain available in the archive.

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

The previous phase work is finished. The old Phase 16 acceptance packet and owner-testing findings remain in the archive as historical records; their earlier pending statuses describe the state when recorded, not the current work list.

### Repository presentation polish

#### P23.1 — Correct stale public documentation claims
**Status:** done; Cameron waived verification for the presentation polish steps
**Files:** `README.md`, `CONTRIBUTING.md`, `docs/msc2/rolling-plan.md`
**What:** Align the map availability statement with the published v0.1.38 release and remove the contributing guide's claim that pull requests run an active CI workflow.
**Verify:** `rg -n 'saved-terrain maps are included|Saved-terrain maps are included|Pull requests do not currently run|v0.1.18 release|CI workflow describes' README.md CONTRIBUTING.md`
**Batch:** solo
**Commit:** `P23.1: correct stale public documentation claims`

#### P23.2 — Shorten and structure the README
**Status:** done; verification waived by Cameron
**Files:** `README.md`, `docs/msc2/rolling-plan.md`
**What:** Keep the first-use path and current capabilities easy to scan; move historical release detail behind existing release-note links without losing important support limits. Removed the older release-by-release narrative while retaining the current platform, security, update, and macOS repair guidance.
**Verify:** `git diff HEAD^ HEAD -- README.md`
**Batch:** solo
**Commit:** `P23.2: shorten README release history`

#### P23.3 — Add current product screenshots
**Status:** deferred; Cameron will add screenshots later
**Files:** `README.md`, `docs/msc2/rolling-plan.md`, screenshot assets to be chosen during the step
**What:** Show the current desktop app and host-local TUI near the top of the README, with accurate captions and accessible descriptions.
**Verify:** `git diff HEAD^ HEAD -- README.md`
**Batch:** solo

#### P23.4 — Clarify published versus source-only features
**Status:** done; verification waived by Cameron
**Files:** `README.md`, `docs/msc2/rolling-plan.md`
**What:** Group current-release availability notes so readers can tell what the download contains and what requires a source build. Put the published/source distinction beside installation and remove scattered version notes from feature descriptions.
**Verify:** `rg -n 'published|source|release|TUI' README.md`
**Batch:** solo
**Commit:** `P23.4: clarify download and source feature availability`

#### P23.5 — Improve the contributor entry point
**Status:** done; verification waived by Cameron
**Files:** `CONTRIBUTING.md`, `docs/msc2/rolling-plan.md`
**What:** Add a concise repo map and launch instructions for the desktop app and agent, checked against the actual project scripts and Tauri configuration.
**Verify:** `git diff HEAD^ HEAD -- CONTRIBUTING.md`
**Batch:** solo
**Commit:** `P23.5: improve contributor entry point`

#### P23.6 — Clarify the host and control-device introduction
**Status:** done; verification waived by Cameron
**Files:** `README.md`, `docs/msc2/rolling-plan.md`
**What:** Address readers who are new to MSC 2 in the host and control-device explanation.
**Verify:** `rg -n 'If you are new to MSC 2' README.md`
**Batch:** solo
**Commit:** `P23.6: clarify the host and control-device introduction`

#### P23.7 — Preview the agent status symbol
**Status:** done; awaiting Cameron's design feedback
**Files:** `docs/msc2/agent-indicator-preview.html`, `docs/msc2/rolling-plan.md`
**What:** Show the proposed helmet symbol in running, stopped, and unavailable states at real system-bar size, with a sample status menu. This is a design preview, not an installed indicator.
**Verify:** Open `docs/msc2/agent-indicator-preview.html` in a browser.
**Batch:** solo
**Commit:** `P23.7: preview the agent status symbol`

#### P23.8 — Correct the helmet silhouette preview
**Status:** done; awaiting Cameron's design feedback
**Files:** `docs/msc2/agent-indicator-preview.html`, `docs/msc2/rolling-plan.md`
**What:** Replace the ring-shaped draft with a Minecraft-style helmet silhouette based on Cameron's reference image: broad crown, face opening, and two separate lower sides.
**Verify:** Refresh `docs/msc2/agent-indicator-preview.html` in a browser.
**Batch:** solo
**Commit:** `P23.8: correct the helmet indicator silhouette`

**Phase 22 disposition:** Cameron accepted the current TUI on 2026-10-30. Detailed P22 records and their original verification commands are in the archive.

## Archived owner-testing details

Detailed records for Findings 5, 9, 10, 15 and 26, plus P16.97, P16.101, and P16.130–P16.136, are in the [rolling plan archive](rolling-plan-archive.md#archived-owner-testing-records--0132). These records preserve the investigation and verification history. Cameron confirmed their follow-up work is finished on 2026-10-30.
