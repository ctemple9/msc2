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
| 24 | Modpack management and content exports | Planned; autonomous execution protocol recorded |

## Phase 24 — Modpack management and content exports

**State:** planned 2026-11-05; owner-authorized autonomous workflow recorded.
Implementation has not started. Start/resume using the execution protocol below.
**Scope:** [owner-agreed behavior and implementation boundaries](addons/phase24-scope.md).
**Gate:** [port plan](msc2-port-plan.md#phase-24--modpack-management-and-content-exports).

**Execution:** [autonomous order, checkpoint ledger and reusable prompt](addons/phase24-execution.md).
The owner's November 5 workflow amendment replaces solo handoffs. Execute in
this dependency order (step numbers remain stable):

`1–10 → 15–18 → 21 → 11 → 12 → 13 → 14 → 19 → 22 → 23 → 20 → 24`.

Keep one commit per step; each implemented status remains awaiting owner
verification. Routine verification does not require an owner prompt before the
next step. New sheets/views require runnable owner review before continuing;
a step with several new surfaces may have several reviews but one final commit.
See the ledger for exact stop/resume rules. Read antiAIslop.md for frontend work.

Every backend slice updates DTO/OpenAPI, capability and generated clients as
needed. Endpoint adapters are completed with their consuming steps; do not
publish a capability for a stub. Retained-data ownership/reference protection
starts in P24.2; P24.23 integrates the Storage presentation and cleanup review.
P24.20 moves after terminal/storage work so missing screenshots cannot block
independent implementation. Final acceptance still requires the screenshots.
No tests, release workflow changes or publication are authorized. Run relevant
non-test checks per step, plus Rust formatting/Clippy; reuse already-successful
checks when no affected source has changed. New module paths are proposals.

### P24.1 — Freeze integration and export contracts

**Status:** implemented — awaiting owner verification
**Files:** docs/msc2/addons/phase24-contract.md; docs/msc2/addons/phase24-scope.md; docs/msc2/addons/phase24-acceptance.md; docs/msc2/api-contract/openapi.json; docs/msc2/client-capability-matrix.csv
**What:** Inventory every Modrinth call/site and its CurseForge counterpart, including diagnostics and world paths. Specify provider-qualified catalog/content DTOs, permissions, operation and capability semantics, destination rules and existing API compatibility. Freeze exact ZIP/.mrpack eligibility, classification review and recovery/update contracts. Record primary-source evidence and external prerequisites; no implementation in this step.
**Verify:** `git diff HEAD^ -- docs/msc2/addons docs/msc2/api-contract/openapi.json docs/msc2/client-capability-matrix.csv`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.1: freeze integration and export contracts`
**Batch:** A — autonomous foundations; no routine owner handoff

### P24.2 — Persist content ownership and player collections

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-domain/src/{modpack,app_config_schema}.rs; crates/msc-infrastructure/src/; docs/msc2/addons/phase24-acceptance.md
**What:** Add versioned upstream/current/player/publication metadata, path ownership, hashes, exclusions, required/optional and classification evidence. Migrate existing servers non-destructively with unknown provenance explicit. Store retained data under owned roots without putting client-only files in the live tree.
**Verify:** `cargo check -p msc-infrastructure`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.2: persist content ownership and player collections`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added versioned optional `content_collection` to server config, provider-qualified sources, upstream/current paths, exclusions, player audience/optional metadata and publication references. Existing configs omit it unchanged; unknown collection/item fields round-trip. Uses the existing atomic config repository. Validates hash identity, portable paths and reserved retained roots. `.msc2-content` stays in protected registered-server fallback accounting (Storage Group::Other); no new cleanup eligibility or capability is advertised. `cargo check -p msc-infrastructure`, `cargo fmt --all` and focused Clippy passed; no tests run.

### P24.3 — Extend shared curseforge catalog operations

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-domain/src/addon_provider.rs; crates/msc-infrastructure/src/addon_provider.rs
**What:** Implement Java categories, paginated search, project descriptions, files/releases, filters and exact file identity. Supply provider pack acquisition into staged archives for both providers; creation/update/import must consume this shared path. Reuse secret-backed keys; scope headers to trusted provider hosts and handle redirects, authentication, rate limits, unavailable categories and restricted downloads explicitly. Preserve existing providers/Bedrock.
**Verify:** `cargo check -p msc-infrastructure`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.3: extend shared curseforge catalog operations`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Additive CurseForge catalog/detail/release routes, typed DTO/OpenAPI/generated clients and capability disclosure; dynamic Java classes, paging, dependencies and distribution/manual-download metadata. Shared Modrinth/CurseForge pinned pack acquisition streams to protected staging with size/SHA checks, cooperative cancellation, credential/host revalidation, owner-only redemption and no authenticated redirects. Existing Modrinth routes retained. Agent build and focused Clippy passed; frontend type-check passed with 10 existing unused-selector warnings. No tests or live provider calls run.

### P24.4 — Resolve curseforge installs and dependencies

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/{addons,addon_dependencies,curseforge_manual}.rs; crates/msc-agent/src/routes/components.rs; crates/msc-api/src/
**What:** Wire provider-qualified installation and dependency review, pinned compatible files, cycles/conflicts and shared dependencies. Extend D-027 staged manual uploads to individual content and player content with exact expected identity; no automatic cross-provider substitution.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.4: resolve curseforge installs and dependencies`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added explicit-server CurseForge review/apply/manual-admission/completion APIs with typed OpenAPI/generated clients and capability disclosure. Dependency review pins bounded compatible releases, required/shared/cyclic dependencies and known incompatibilities; owner/config/revision/byte baselines are persisted. Downloads and exact-size/manual SHA1 validation stage before stopped-server activation; a durable marker recovers failed/interrupted activation while preserving changed/pre-existing files. Startup refuses unresolved recovery. Identical installs preserve audience/optional settings. Three essential deterministic safety tests added for rejected manual bytes, metadata-save rollback and changed-file recovery (existing pack/manual tests do not cover this new transaction); expected <100 ms each, not run. Agent build/focused Clippy and frontend type-check passed (10 existing CSS warnings). Player-target consumption of shared review/staging remains in P24.7; no player-install capability is advertised here.

### P24.5 — Extend identification and updates across providers

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/{add_on_inventory,addon_updates,diagnostics}.rs; crates/msc-infrastructure/src/addon_provider.rs; crates/msc-agent/src/routes/components.rs
**What:** Add CurseForge fingerprint/exact matching, manual source links and update discovery/install for all inventoried Modrinth equivalents. Keep ambiguous/unidentified files visible; preserve evidence and detect outside edits.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.5: extend identification and updates across providers`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added bounded two-pass fingerprinting with exact SHA1/size verification, ambiguous/unidentified/changed-file reporting, provider-qualified inventory and manual source links/hints. Compatible update discovery and pinned update-review API preserve verified installed dependencies and disabled filenames. Extended the shared activation journal with original-file rollback and old/new filename mapping; source changes clear obsolete classification evidence. Diagnostics refuses to pass CurseForge-linked repairs to Modrinth and returns the shared review/catalog paths for the P24.11 client. Added two essential deterministic rollback tests covering same-name and renamed updates (not run; expected <100 ms each; prior install-only checks missed replacement originals). Updated DTO/OpenAPI/generated types, capabilities and matrix. Agent Clippy/build, API generation and frontend check passed (10 existing CSS warnings); live provider/physical acceptance pending.

### P24.6 — Retain complete imported player content

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/pack_retention.rs; modpacks.rs; provisioning.rs; crates/msc-domain/src/content_collection.rs; crates/msc-agent/src/routes/components/pack_retention.rs and upload/import routes; provider DTOs, OpenAPI/generated clients; tests/pack_retention_safety.rs
**What:** Retain original archives and resolve complete client/shared content before server filtering for both pack formats, including overrides and manual-download checkpoints. Distinguish source server packs from full client packs; journal bounded staging, failure/cancel recovery and content readiness.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.6: retain complete imported player content`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Original archives and all override layers are retained before server filtering, with streaming checksums, bounded copies and protected roots. Server/player baselines use separate maps, preserving different bytes at the same game path. Both manifest formats resolve client/shared files into private retained storage; exact missing files remain explicit, with class-aware folders, importer exclusions and bounded embedded environment evidence. Provider acquisition retains source identity and CurseForge server-pack evidence; local source type stays unknown. Status, journaled retry from the verified original and owner/file-bound raw manual recovery APIs are wired; existing server manual completion reconciles exact matching retained files. New endpoints/types/capability are published. Focused agent Clippy and frontend type-check pass (10 existing CSS warnings). Two essential deterministic tests protect retained manual bytes and side-specific override separation; no existing import checks cover those retained copies, expected <100 ms each, not run. Live provider, cancellation/restart, disk-full, physical platform and source-server-pack recovery acceptance remain pending. Uncertain classification, unsupported mirrors or unavailable exact hashes stay unresolved; retention is not publication approval.

### P24.7 — Manage independent player content and requirements

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/content_management.rs; content_install.rs; crates/msc-agent/src/routes/components/content_management.rs and router/capabilities; provider DTOs; domain collection; OpenAPI/generated clients; tests/player_content_safety.rs
**What:** Implement Server/Players/Both and required/optional mutation/review with separate storage. Adding/updating/removing/toggling server content recomputes player dependency needs without discarding owner choices; ambiguous or contradictory evidence requires review. Include resource packs, shaders and config/scripts.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.7: manage independent player content and requirements`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added explicit-server collection/requirements read, owner/server/path/hash-bound player upload admission, review and journaled apply endpoints. Immutable player blobs live outside running server content; metadata application is one atomic config commit. Reviews pin collection, game/loader, server identity and byte baseline, expire and revalidate retained bytes/permissions. Owners can add/remove independent files, choose required/optional/enabled, classify server content, copy matching server bytes to players and explicitly restore individual upstream player files. Owner removal tombstones are separate from server removals. Classification changes do not install/uninstall live server files; existing stopped-server install/remove/toggle workflows remain authoritative and the review warns about client-only content still installed on the server. Every collection read recomputes server additions/changes/removals/disabled state and known provider dependency/conflict gaps without discarding player customizations. Unknown evidence requires review. New shader/resource selections are inactive by default, and known redistribution restrictions cannot be overridden by an attestation. Focused Clippy and frontend check passed (10 existing CSS warnings); two essential local tests protect stale/owner-bound reviews and independent retained bytes, expected <100 ms each, not run. No new UI yet; physical behavior, provider-source dependency completeness and all platform acceptance remain pending.

### P24.8 — Recover player packs for existing servers

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/content_recovery.rs; content_management.rs; pack_retention.rs; crates/msc-agent/src/routes/components/content_recovery.rs and content review read/router; provider DTOs/capabilities; OpenAPI/generated clients; tests/content_recovery_safety.rs
**What:** Preview non-destructive inventory recovery and optional original-pack recovery. Match exact identities/hashes, show missing client content, resolve conflicts and retain player files without reinstalling or modifying the live server.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.8: recover player packs for existing servers`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added journaled recovery review preparation and owner-bound review read, applying through P24.7 atomic player metadata. Recovery can explicitly retain installed files, inspect an uploaded matching original, or re-use a checksum-verified retained original (including protected interrupted-import references). It never calls the live server importer or writes server/world files. Exact matching files may gain provenance/environment evidence; conflicting server/player customizations and player removal tombstones are preserved unless individual upstream restoration is selected. Missing manifest files, server-only source evidence and unverified local archive origin stay explicit. Game/loader/version mismatches and known different base projects are refused. Review binding now also pins loader version. Focused agent Clippy and frontend type-check passed (10 existing CSS warnings); schema references resolve. One essential deterministic recovery test protects server config/world bytes and original/client override retention, expected <100 ms, not run. Live network/manual recovery, permission/cancellation/restart and cross-platform scenarios remain owner-unverified.

### P24.9 — Preview and apply upstream pack updates

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/pack_updates.rs; pack_review.rs; pack_activation.rs; content_install.rs/pack_retention.rs; crates/msc-agent/src/routes/components/pack_updates.rs and router/lifecycle guards; provider DTOs/capabilities; OpenAPI/generated clients; tests/pack_update_comparison.rs and pack_activation_safety.rs
**What:** Implement old-upstream/current/new-upstream comparison, owner removal tombstones and per-item restore-upstream resolution. Stage and recover updates atomically within journal/operation locking; stopped-server enforcement, stale-review checks and truthful partial recovery.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.9: preview and apply upstream pack updates`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added stopped-server, operation-locked pack review/read/apply and review-bound manual admission/completion APIs. Catalog-acquired or uploaded archives feed retained original/client/server content; a previous review can retry its pinned original without another upload. Three-way comparison preserves owner edits, additions, intentional removals and options; explicit per-path upstream restore and provider-qualified filename changes prevent duplicate/resurrected versions. Observations cover active mod inventory plus old/new tracked content paths; unsupported/runtime/world paths remain untouched and visible as exclusions. Matching game/loader setup and known base-project identity are enforced; unverified local association needs explicit confirmation. Missing exact files or new server audience decisions block apply until recovered/reviewed. Before/after bytes stage privately, active changes journal before mutation, atomic config commit identifies completed transactions, and rollback/startup recovery preserve later edits behind a blocking marker. Existing add-on actions refuse unresolved pack recovery. Generated contracts and capability are wired. Focused agent Clippy and frontend check passed (10 existing CSS warnings); schema references resolve. Three essential pure comparison tests (<5 ms each) cover owner edits/removals/restoration and renamed project identity; two local generic activation tests (<100 ms each) cover metadata-save rollback and preserved subsequent edits. Existing JAR-only tests do not cover these old-pack/generic-config paths; none run. Live providers, large packs, cancellation/restart, manual recovery and physical platforms remain owner-unverified. Cross-game/loader migration is refused rather than silently changing server setup in this content-update path.

### P24.10 — Replace base packs through a content review

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/pack_review.rs; crates/msc-agent/src/routes/components/pack_updates.rs and router/capabilities; provider DTOs; OpenAPI/generated clients
**What:** Enforce one base pack. Preview removal of old owned paths, compatible owner additions and conflicts; preserve unknown files and worlds, warn about world compatibility, and apply through the same recoverable transaction as updates.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.10: replace base packs through a content review`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added explicit replacement review/apply endpoints using P24.9 staging, manual recovery and rollback. Update/replacement intent is persisted and cannot be interchanged at apply. One new base identity/baseline replaces the current one; historical retained roots remain protected. Old unmodified pack-owned paths are removed, while owner changes/additions and unknown paths remain preserved. Matching game/loader setup is enforced; retained owner mods require per-path exact-hash compatibility confirmation, and known project-version/incompatibility conflicts remain blocking. Replacement requires acknowledgment that changing packs may make existing worlds incompatible; worlds are excluded from mutation. Current legacy pack identity/name/version are updated together. A fresh review can reuse a pinned prior archive after owner edits while apply still rejects stale byte/collection baselines. Focused agent Clippy and frontend type-check passed (10 existing CSS warnings); schema references resolve. Reused P24.9 comparison/activation safety coverage rather than adding duplicate tests; no tests run. Owner physical replacement, compatibility evidence, cancellations/recovery and all platforms remain pending.

### P24.11 — Browse both providers in desktop content screens

**Status:** planned — not started
**Files:** clients/desktop-web/src/lib/sections/components/; clients/desktop-web/src/lib/sections/addons/; clients/desktop-web/src/lib/api/
**What:** Expose provider choice, detail/version/dependency/manual-download workflows, source identification and update review in existing-server content screens. Reuse shared components; implement loading, error and old-agent states. Read antiAIslop.md before all frontend work in this phase.
**Verify:** `npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.11: browse both providers in desktop content screens`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

### P24.12 — Select catalog packs during server creation

**Status:** planned — not started
**Files:** clients/desktop-web/src/lib/sections/fleet/wizard/; crates/msc-agent/src/routes/servers.rs; crates/msc-application/src/
**What:** Add both-provider pack browsing to AddOnsStep while keeping local import paths. Show incompatible packs; matching release defaults; Try current setup vs automatic exact game/loader change, then staged-content conflict review. Revalidate on create, preserve draft on failure and respect supported-version floors.
**Verify:** `cargo check -p msc-agent && npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.12: select catalog packs during server creation`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

### P24.13 — Extend world content browsing to curseforge

**Status:** planned — not started
**Files:** crates/msc-agent/src/routes/worlds.rs; crates/msc-application/src/; clients/desktop-web/src/lib/sections/worlds/; clients/desktop-web/src/lib/sections/fleet/wizard/WorldStep.svelte
**What:** Provide CurseForge equivalents for Modrinth Java datapack browsing, details, selection, installation and updates wherever currently supported, including creation. Preserve D-030 world-slot ownership and existing Bedrock behavior; finish any inventory-specific provider gaps.
**Verify:** `cargo check -p msc-agent && npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.13: extend world content browsing to curseforge`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

### P24.14 — Edit player content and review pack changes on desktop

**Status:** planned — not started
**Files:** clients/desktop-web/src/lib/sections/addons/; clients/desktop-web/src/lib/sections/components/
**What:** Expose audience, required/optional, client-only additions, uncertain classification and server-mod player checks. Provide existing-server recovery, pack update and replacement comparisons with per-item keep-owner/restore-upstream and stale-result handling.
**Verify:** `npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.14: edit player content and review pack changes on desktop`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

### P24.15 — Publish immutable dated content revisions

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/content_publications.rs; publication_review.rs; publication_datapacks.rs; content_guides.rs; crates/msc-agent/src/routes/components/content_publications.rs and router/capabilities; provider DTOs; OpenAPI/generated clients; tests/content_publication_safety.rs
**What:** Freeze reviewed content bytes/hashes and metadata under date/revision identity; lock and revalidate concurrent changes. Track unpublished changes; complete vs explicitly incomplete publications; re-export old revisions independently of current files or provider availability. Persist eligibility evidence and retention ownership.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.15: publish immutable dated content revisions`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added explicit-audience publication review/read/publish/history/snapshot and world-slot datapack inventory APIs. Reviews bind exact bytes, owner, collection/game/loader, selection/options and permissions; config/scripts require explicit per-file review, known distribution restrictions cannot be overridden, and API download permission alone is not redistribution evidence. Owner-selected optional content is included when eligible; selected unresolved content requires explicit incomplete consent. Freeze preflights unique bytes, streams independent copies, pins date plus monotonic publication revision, checksums manifest in the registry and freezes versioned guide files. Current source bytes are checked after copying before atomic registry commit; historical reads/re-export inputs do not depend on current content/provider availability. World datapacks come only from an explicit slot; active live content takes precedence over stale archives and archive inspection stages only selected datapack entries. Runtime/private/world data and arbitrary collection extras are excluded; child symlinks/case collisions are rejected. History pages report unavailable manifests and current unpublished changes. Guide text-v1 is a truthful development fallback; launcher-specific owner screenshots/HTML remain P24.20, not accepted here. Focused agent Clippy and frontend check passed (10 existing CSS warnings); schema references resolve. Three essential local tests protect independent frozen bytes, manifest checksum integrity and restricted-content/incomplete-consent boundaries; prior retention/JAR tests do not cover the publication registry/eligibility, expected <100 ms each, not run. Live publication/cancellation/revocation, large world/pack memory/disk behavior and physical platforms remain owner acceptance.

### P24.16 — Generate reviewed player and server zip archives

**Status:** planned — not started
**Files:** crates/msc-application/src/client_export.rs; crates/msc-infrastructure/src/; crates/msc-agent/src/routes/components.rs
**What:** Stream snapshot ZIPs with audience-correct folders, manifest and instruction hooks. Include approved config/scripts and selected world-slot datapacks; exclude runtime/world/private files. Preflight space, reject unsafe paths/symlinks, avoid arbitrary config copying and make missing/restricted content conspicuous.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.16: generate reviewed player and server zip archives`
**Batch:** A — autonomous foundations; no routine owner handoff

### P24.17 — Include optional launcher importable mrpack files

**Status:** planned — not started
**Files:** crates/msc-application/src/client_export.rs; crates/msc-domain/src/modpack_manifest.rs; crates/msc-infrastructure/src/
**What:** Generate optional nested .mrpack from the same revision with hashes, dependencies, environments and appropriate overrides. Resolve public downloads vs permitted embedded content, no credentials. Report per-file/launcher limitations and incomplete eligibility; no falsely importable plugin/server bundle.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.17: include optional launcher importable mrpack files`
**Batch:** A — autonomous foundations; no routine owner handoff

### P24.18 — Save exports locally or on the selected host

**Status:** planned — not started
**Files:** crates/msc-agent/src/routes/components.rs; clients/desktop-web/src-tauri/src/native_transfer/; clients/desktop-web/src/lib/platform/; clients/desktop-web/src/lib/api/
**What:** Use authenticated bounded binary transfer for desktop-local saves from any paired agent; add authorized host-local export destination handling. Revalidate host/server/publication/destination and overwrite consent; atomic completion, progress, cancellation and disconnect/disk-full cleanup. Preserve older export clients explicitly.
**Verify:** `cargo check -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.18: save exports locally or on the selected host`
**Batch:** A — autonomous foundations; no routine owner handoff

### P24.19 — Add the publish and export review sheet

**Status:** planned — not started
**Files:** clients/desktop-web/src/lib/sections/addons/; clients/desktop-web/src/lib/sections/components/
**What:** Implement Player/Server, categories/items, date/revision/name, optional .mrpack, completeness, size and named local/remote destination. Export prior revisions or publish current changes deliberately; show progress/results and actionable missing content. Follow antiAIslop.md.
**Verify:** `npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.19: add the publish and export review sheet`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

### P24.20 — Bundle illustrated offline launcher instructions

**Status:** planned — not started
**Files:** docs/msc2/addons/launcher-guides/; clients/desktop-web/static/; crates/msc-application/src/client_export.rs
**What:** With Cameron-supplied screenshots, create versioned offline HTML/text guides for Minecraft Launcher, Prism, Modrinth App, CurseForge App and SKlauncher. Verify exact launcher import capabilities; show manual copy and optional .mrpack routes, correct loader, isolated installs, optional content and clean updates. Add separate Server instructions; no external assets or invitation links. Do not complete this step without screenshots.
**Verify:** `cargo check -p msc-application`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.20: bundle illustrated offline launcher instructions`
**Batch:** C — screenshot prerequisite and offline guide review

### P24.21 — Expose content and export operations in the cli

**Status:** planned — not started
**Files:** crates/msc-agent/src/cli/mod.rs; docs/msc2/client-capability-matrix.csv
**What:** Add both-provider catalog/detail/install/identification/update and pack selection, player management, recovery, conflict review, publication and local host export commands. Require explicit conflict/destination/incomplete choices, safe secret input and scriptable outputs; reflect actual capabilities.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.21: expose content and export operations in the cli`
**Batch:** A — autonomous foundations; no routine owner handoff

### P24.22 — Expose content and export operations in the tui

**Status:** planned — not started
**Files:** crates/msc-agent/src/cli/tui/; docs/msc2/client-capability-matrix.csv
**What:** Provide host-local interactive provider browsing, creation compatibility choices, audience editing, dependency review, recovery, update/replacement and publish/export review. Use the shared API and host-local destinations; preserve SSH behavior and existing accepted TUI flows.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.22: expose content and export operations in the tui`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

### P24.23 — Account for retained packs and publication storage

**Status:** planned — not started
**Files:** crates/msc-application/src/; crates/msc-agent/src/routes/; clients/desktop-web/src/lib/sections/; docs/msc2/storage/
**What:** Integrate retained originals/player content/publications/staging with Phase 21 ownership and accounting. Protect live/referenced snapshots, export-in-progress and rollback data, prevent double counting, explain storage use and reuse explicit reviewed cleanup; no automatic historical-revision deletion.
**Verify:** `cargo check -p msc-agent && npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.23: account for retained packs and publication storage`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

### P24.24 — Record integrated owner acceptance and gate evidence

**Status:** planned — not started
**Files:** docs/msc2/addons/phase24-acceptance.md; docs/msc2/client-capability-matrix.csv; docs/msc2/rolling-plan.md
**What:** Prepare and record Cameron-run physical scenarios listed in scope across supported OSes, local/remote and real launchers using disposable content. Record missing screenshots/provider prerequisites and unresolved results honestly. No automated tests, release runs or step/phase closure by the implementing agent. Other agent gate review follows only when Cameron requests it.
**Verify:** `git diff HEAD^ -- docs/msc2/addons/phase24-acceptance.md docs/msc2/client-capability-matrix.csv docs/msc2/rolling-plan.md`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.24: record integrated owner acceptance and gate evidence`
**Batch:** D — final owner acceptance; no agent phase closure
