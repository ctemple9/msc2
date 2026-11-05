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
| 24 | Modpack management and content exports | Planned; awaiting Cameron's Read |

## Phase 24 — Modpack management and content exports

**State:** planned 2026-11-05; awaiting owner review. No code implemented.
**Scope:** [owner-agreed behavior and implementation boundaries](addons/phase24-scope.md).
**Gate:** [port plan](msc2-port-plan.md#phase-24--modpack-management-and-content-exports).

Run in order, one solo step per EXECUTE conversation. Every implementation step
updates this record to awaiting owner verification and makes one commit; Cameron
closes it. All Verify commands below are non-test checks and are not substitutes
for the step-specific physical scenarios in the scope/acceptance record. No
agent-run tests are authorized. Rust work also runs relevant formatting/Clippy;
no release workflow or CI changes. Frontend steps require antiAIslop.md.
Each backend step includes the corresponding DTO/OpenAPI, capability and generated
client updates needed to keep that slice usable; do not defer contract wiring
to the final client steps. P24.20 depends on owner-supplied screenshots; do not fabricate them or skip its
acceptance. New source module paths below are implementation proposals.

### P24.1 — Freeze integration and export contracts

**Status:** planned — awaiting owner Read
**Files:** docs/msc2/addons/phase24-scope.md; docs/msc2/addons/phase24-acceptance.md; docs/msc2/api-contract/openapi.json; docs/msc2/client-capability-matrix.csv
**What:** Inventory every Modrinth call/site and its CurseForge counterpart, including diagnostics and world paths. Specify provider-qualified catalog/content DTOs, permissions, operation and capability semantics, destination rules and existing API compatibility. Freeze exact ZIP/.mrpack eligibility, classification review and recovery/update contracts. Record primary-source evidence and external prerequisites; no implementation in this step.
**Verify:** `git diff HEAD^ -- docs/msc2/addons docs/msc2/api-contract/openapi.json docs/msc2/client-capability-matrix.csv`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.1: freeze integration and export contracts`
**Batch:** solo

### P24.2 — Persist content ownership and player collections

**Status:** planned — awaiting owner Read
**Files:** crates/msc-domain/src/{modpack,app_config_schema}.rs; crates/msc-infrastructure/src/; docs/msc2/addons/phase24-acceptance.md
**What:** Add versioned upstream/current/player/publication metadata, path ownership, hashes, exclusions, required/optional and classification evidence. Migrate existing servers non-destructively with unknown provenance explicit. Store retained data under owned roots without putting client-only files in the live tree.
**Verify:** `cargo check -p msc-infrastructure`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.2: persist content ownership and player collections`
**Batch:** solo

### P24.3 — Extend shared curseforge catalog operations

**Status:** planned — awaiting owner Read
**Files:** crates/msc-domain/src/addon_provider.rs; crates/msc-infrastructure/src/addon_provider.rs
**What:** Implement Java categories, paginated search, project descriptions, files/releases, filters and exact file identity. Reuse secret-backed keys; scope headers to trusted provider hosts and handle redirects, authentication, rate limits, unavailable categories and restricted downloads explicitly. Preserve existing providers/Bedrock.
**Verify:** `cargo check -p msc-infrastructure`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.3: extend shared curseforge catalog operations`
**Batch:** solo

### P24.4 — Resolve curseforge installs and dependencies

**Status:** planned — awaiting owner Read
**Files:** crates/msc-application/src/{addons,addon_dependencies,curseforge_manual}.rs; crates/msc-agent/src/routes/components.rs; crates/msc-api/src/
**What:** Wire provider-qualified installation and dependency review, pinned compatible files, cycles/conflicts and shared dependencies. Extend D-027 staged manual uploads to individual content and player content with exact expected identity; no automatic cross-provider substitution.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.4: resolve curseforge installs and dependencies`
**Batch:** solo

### P24.5 — Extend identification and updates across providers

**Status:** planned — awaiting owner Read
**Files:** crates/msc-application/src/{add_on_inventory,addon_updates,diagnostics}.rs; crates/msc-infrastructure/src/addon_provider.rs; crates/msc-agent/src/routes/components.rs
**What:** Add CurseForge fingerprint/exact matching, manual source links and update discovery/install for all inventoried Modrinth equivalents. Keep ambiguous/unidentified files visible; preserve evidence and detect outside edits.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.5: extend identification and updates across providers`
**Batch:** solo

### P24.6 — Retain complete imported player content

**Status:** planned — awaiting owner Read
**Files:** crates/msc-application/src/modpacks.rs; crates/msc-domain/src/modpack_manifest.rs; crates/msc-agent/src/routes/components.rs
**What:** Retain original archives and resolve complete client/shared content before server filtering for both pack formats, including overrides and manual-download checkpoints. Distinguish source server packs from full client packs; journal bounded staging, failure/cancel recovery and content readiness.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.6: retain complete imported player content`
**Batch:** solo

### P24.7 — Manage independent player content and requirements

**Status:** planned — awaiting owner Read
**Files:** crates/msc-application/src/; crates/msc-agent/src/routes/components.rs; crates/msc-api/src/
**What:** Implement Server/Players/Both and required/optional mutation/review with separate storage. Adding/updating/removing/toggling server content recomputes player dependency needs without discarding owner choices; ambiguous or contradictory evidence requires review. Include resource packs, shaders and config/scripts.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.7: manage independent player content and requirements`
**Batch:** solo

### P24.8 — Recover player packs for existing servers

**Status:** planned — awaiting owner Read
**Files:** crates/msc-application/src/modpacks.rs; crates/msc-agent/src/routes/components.rs
**What:** Preview non-destructive inventory recovery and optional original-pack recovery. Match exact identities/hashes, show missing client content, resolve conflicts and retain player files without reinstalling or modifying the live server.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.8: recover player packs for existing servers`
**Batch:** solo

### P24.9 — Preview and apply upstream pack updates

**Status:** planned — awaiting owner Read
**Files:** crates/msc-application/src/modpacks.rs; crates/msc-agent/src/routes/components.rs; crates/msc-api/src/
**What:** Implement old-upstream/current/new-upstream comparison, owner removal tombstones and per-item restore-upstream resolution. Stage and recover updates atomically within journal/operation locking; stopped-server enforcement, stale-review checks and truthful partial recovery.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.9: preview and apply upstream pack updates`
**Batch:** solo

### P24.10 — Replace base packs through a content review

**Status:** planned — awaiting owner Read
**Files:** crates/msc-application/src/modpacks.rs; crates/msc-agent/src/routes/components.rs
**What:** Enforce one base pack. Preview removal of old owned paths, compatible owner additions and conflicts; preserve unknown files and worlds, warn about world compatibility, and apply through the same recoverable transaction as updates.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.10: replace base packs through a content review`
**Batch:** solo

### P24.11 — Browse both providers in desktop content screens

**Status:** planned — awaiting owner Read
**Files:** clients/desktop-web/src/lib/sections/components/; clients/desktop-web/src/lib/sections/addons/; clients/desktop-web/src/lib/api/
**What:** Expose provider choice, detail/version/dependency/manual-download workflows, source identification and update review in existing-server content screens. Reuse shared components; implement loading, error and old-agent states. Read antiAIslop.md before all frontend work in this phase.
**Verify:** `npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.11: browse both providers in desktop content screens`
**Batch:** solo

### P24.12 — Select catalog packs during server creation

**Status:** planned — awaiting owner Read
**Files:** clients/desktop-web/src/lib/sections/fleet/wizard/; crates/msc-agent/src/routes/servers.rs; crates/msc-application/src/
**What:** Add both-provider pack browsing to AddOnsStep while keeping local import paths. Show incompatible packs; matching release defaults; Try current setup vs automatic exact game/loader change, then staged-content conflict review. Revalidate on create, preserve draft on failure and respect supported-version floors.
**Verify:** `cargo check -p msc-agent && npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.12: select catalog packs during server creation`
**Batch:** solo

### P24.13 — Extend world content browsing to curseforge

**Status:** planned — awaiting owner Read
**Files:** crates/msc-agent/src/routes/worlds.rs; crates/msc-application/src/; clients/desktop-web/src/lib/sections/worlds/; clients/desktop-web/src/lib/sections/fleet/wizard/WorldStep.svelte
**What:** Provide CurseForge equivalents for Modrinth Java datapack browsing, details, selection, installation and updates wherever currently supported, including creation. Preserve D-030 world-slot ownership and existing Bedrock behavior; finish any inventory-specific provider gaps.
**Verify:** `cargo check -p msc-agent && npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.13: extend world content browsing to curseforge`
**Batch:** solo

### P24.14 — Edit player content and review pack changes on desktop

**Status:** planned — awaiting owner Read
**Files:** clients/desktop-web/src/lib/sections/addons/; clients/desktop-web/src/lib/sections/components/
**What:** Expose audience, required/optional, client-only additions, uncertain classification and server-mod player checks. Provide existing-server recovery, pack update and replacement comparisons with per-item keep-owner/restore-upstream and stale-result handling.
**Verify:** `npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.14: edit player content and review pack changes on desktop`
**Batch:** solo

### P24.15 — Publish immutable dated content revisions

**Status:** planned — awaiting owner Read
**Files:** crates/msc-domain/src/; crates/msc-application/src/client_export.rs; crates/msc-agent/src/routes/components.rs; crates/msc-api/src/
**What:** Freeze reviewed content bytes/hashes and metadata under date/revision identity; lock and revalidate concurrent changes. Track unpublished changes; complete vs explicitly incomplete publications; re-export old revisions independently of current files or provider availability. Persist eligibility evidence and retention ownership.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.15: publish immutable dated content revisions`
**Batch:** solo

### P24.16 — Generate reviewed player and server zip archives

**Status:** planned — awaiting owner Read
**Files:** crates/msc-application/src/client_export.rs; crates/msc-infrastructure/src/; crates/msc-agent/src/routes/components.rs
**What:** Stream snapshot ZIPs with audience-correct folders, manifest and instruction hooks. Include approved config/scripts and selected world-slot datapacks; exclude runtime/world/private files. Preflight space, reject unsafe paths/symlinks, avoid arbitrary config copying and make missing/restricted content conspicuous.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.16: generate reviewed player and server zip archives`
**Batch:** solo

### P24.17 — Include optional launcher importable mrpack files

**Status:** planned — awaiting owner Read
**Files:** crates/msc-application/src/client_export.rs; crates/msc-domain/src/modpack_manifest.rs; crates/msc-infrastructure/src/
**What:** Generate optional nested .mrpack from the same revision with hashes, dependencies, environments and appropriate overrides. Resolve public downloads vs permitted embedded content, no credentials. Report per-file/launcher limitations and incomplete eligibility; no falsely importable plugin/server bundle.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.17: include optional launcher importable mrpack files`
**Batch:** solo

### P24.18 — Save exports locally or on the selected host

**Status:** planned — awaiting owner Read
**Files:** crates/msc-agent/src/routes/components.rs; clients/desktop-web/src-tauri/src/native_transfer/; clients/desktop-web/src/lib/platform/; clients/desktop-web/src/lib/api/
**What:** Use authenticated bounded binary transfer for desktop-local saves from any paired agent; add authorized host-local export destination handling. Revalidate host/server/publication/destination and overwrite consent; atomic completion, progress, cancellation and disconnect/disk-full cleanup. Preserve older export clients explicitly.
**Verify:** `cargo check -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.18: save exports locally or on the selected host`
**Batch:** solo

### P24.19 — Add the publish and export review sheet

**Status:** planned — awaiting owner Read
**Files:** clients/desktop-web/src/lib/sections/addons/; clients/desktop-web/src/lib/sections/components/
**What:** Implement Player/Server, categories/items, date/revision/name, optional .mrpack, completeness, size and named local/remote destination. Export prior revisions or publish current changes deliberately; show progress/results and actionable missing content. Follow antiAIslop.md.
**Verify:** `npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.19: add the publish and export review sheet`
**Batch:** solo

### P24.20 — Bundle illustrated offline launcher instructions

**Status:** planned — awaiting owner Read
**Files:** docs/msc2/addons/launcher-guides/; clients/desktop-web/static/; crates/msc-application/src/client_export.rs
**What:** With Cameron-supplied screenshots, create versioned offline HTML/text guides for Minecraft Launcher, Prism, Modrinth App, CurseForge App and SKlauncher. Verify exact launcher import capabilities; show manual copy and optional .mrpack routes, correct loader, isolated installs, optional content and clean updates. Add separate Server instructions; no external assets or invitation links. Do not complete this step without screenshots.
**Verify:** `cargo check -p msc-application`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.20: bundle illustrated offline launcher instructions`
**Batch:** solo

### P24.21 — Expose content and export operations in the cli

**Status:** planned — awaiting owner Read
**Files:** crates/msc-agent/src/cli/mod.rs; docs/msc2/client-capability-matrix.csv
**What:** Add both-provider catalog/detail/install/identification/update and pack selection, player management, recovery, conflict review, publication and local host export commands. Require explicit conflict/destination/incomplete choices, safe secret input and scriptable outputs; reflect actual capabilities.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.21: expose content and export operations in the cli`
**Batch:** solo

### P24.22 — Expose content and export operations in the tui

**Status:** planned — awaiting owner Read
**Files:** crates/msc-agent/src/cli/tui/; docs/msc2/client-capability-matrix.csv
**What:** Provide host-local interactive provider browsing, creation compatibility choices, audience editing, dependency review, recovery, update/replacement and publish/export review. Use the shared API and host-local destinations; preserve SSH behavior and existing accepted TUI flows.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.22: expose content and export operations in the tui`
**Batch:** solo

### P24.23 — Account for retained packs and publication storage

**Status:** planned — awaiting owner Read
**Files:** crates/msc-application/src/; crates/msc-agent/src/routes/; clients/desktop-web/src/lib/sections/; docs/msc2/storage/
**What:** Integrate retained originals/player content/publications/staging with Phase 21 ownership and accounting. Protect live/referenced snapshots, export-in-progress and rollback data, prevent double counting, explain storage use and reuse explicit reviewed cleanup; no automatic historical-revision deletion.
**Verify:** `cargo check -p msc-agent && npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.23: account for retained packs and publication storage`
**Batch:** solo

### P24.24 — Record integrated owner acceptance and gate evidence

**Status:** planned — awaiting owner Read
**Files:** docs/msc2/addons/phase24-acceptance.md; docs/msc2/client-capability-matrix.csv; docs/msc2/rolling-plan.md
**What:** Prepare and record Cameron-run physical scenarios listed in scope across supported OSes, local/remote and real launchers using disposable content. Record missing screenshots/provider prerequisites and unresolved results honestly. No automated tests, release runs or step/phase closure by the implementing agent. Other agent gate review follows only when Cameron requests it.
**Verify:** `git diff HEAD^ -- docs/msc2/addons/phase24-acceptance.md docs/msc2/client-capability-matrix.csv docs/msc2/rolling-plan.md`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.24: record integrated owner acceptance and gate evidence`
**Batch:** solo
