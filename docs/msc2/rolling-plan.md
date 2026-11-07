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
| 24 | Modpack management and content exports | Implementation candidate ready for final owner review; illustrated guides and physical acceptance incomplete |

## Phase 24 — Modpack management and content exports

**State:** final implementation candidate, 2026-11-06. P24.1–19 and P24.21–23 implemented with owner verification pending. P24.20 has offline text/HTML preparation but remains incomplete without owner screenshots and exact launcher-build verification. P24.24 records final candidate/physical acceptance evidence; no phase closure or independent review claimed.
**Scope:** [owner-agreed behavior and implementation boundaries](addons/phase24-scope.md).
**Gate:** [port plan](msc2-port-plan.md#phase-24--modpack-management-and-content-exports).

**Execution:** [autonomous order, checkpoint ledger and reusable prompt](addons/phase24-execution.md).
The owner's November 5 workflow amendment replaces solo handoffs. Execute in
this dependency order (step numbers remain stable):

`1–10 → 15–18 → 21 → 11 → 12 → 13 → 14 → 19 → 22 → 23 → 20 → 24`.

Keep one commit per step; each implemented status remains awaiting owner
verification. Routine verification does not require an owner prompt before the
next step. Cameron’s later final-review amendment defers all intermediate
sheet/view stops to the final owner review; it does not authorize tests or closure.
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

**Status:** implemented — awaiting owner verification
**Files:** clients/desktop-web/src/lib/sections/components/; clients/desktop-web/src/lib/sections/addons/; clients/desktop-web/src/lib/api/
**What:** Expose provider choice, detail/version/dependency/manual-download workflows, source identification and update review in existing-server content screens. Reuse shared components; implement loading, error and old-agent states. Read antiAIslop.md before all frontend work in this phase.
**Verify:** `npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.11: browse both providers in desktop content screens`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

**Working-tree checkpoint:** Base commit a14ff1cc (P24.21). New ProviderCatalogBrowser.svelte plus PluginBrowserSheet.svelte provider selection supply the first new view: CurseForge search, pagination, project description, release metadata/dependencies/manual-download status, explicit game-version match labels, missing-key setup, error/retry and older-agent states. Host snapshot/generation guards suppress stale async results. This checkpoint is discovery only; reviewed install/manual upload, existing-file identification and update review remain P24.11 work after this view is approved. The desktop Cargo.lock also records existing new application/infrastructure dependencies required by the normal native dev build. No P24.11 commit until all its new surfaces are reviewed. Frontend type-check/build passed with existing CSS/chunk warnings; normal npx tauri dev built and launched the native app successfully. No tests run. Navigate Components → Browse mods/plugins → CurseForge. Candidate launch: npm --prefix clients/desktop-web exec tauri dev (or cd clients/desktop-web && npx tauri dev).

**Owner feedback revision:** Screenshot review requested matching Modrinth presentation, real project icons, Add, version compatibility/Install anyway and the existing CurseForge key/console behavior. Revised discovery rows to the same compact icon/title/byline/description geometry and bounded results area; version rows now use expandable details, channel/compatibility badges and Stable only. Compatibility checks both exact game version and declared loader (plugin platforms separately). Added a missing-key-only key control plus explicit external console link (owner follow-up: hide the key link once a key is saved); saved keys remain agent-owned and are cleared from the form. Allowed only the official media.forgecdn.net image origin in native CSP. Current action remains Versions: Add, reviewed installation and explicit incompatible-release consent are the next P24.11 portion, not a false install action. Creation provider selection remains P24.12 as agreed by the owner's permission to defer already-planned work. This is requested revision evidence, not approval.

**Approval evidence:** Cameron said “ui passes” after the missing-key-only control correction. Continue into reviewed installation; this does not approve future review/recovery surfaces or close physical acceptance.

**Installation checkpoint:** Added functional CurseForge Add and exact-release Install/Install anyway actions opening ContentInstallReviewSheet.svelte. The agent resolves and pins the dependency set before confirmation; the sheet lists files/identities/warnings, permits manual provider downloads plus exact-file selection/checksum verification, then runs real stopped-server installation with progress/cancellation and inventory refresh. Manual uploads reuse bounded chunk transport through a purpose-bound admission, clean up on failure/cancellation/navigation, and retain cleanup notices. Existing generic staging ownership remains enforced by UploadStore; provider-owned upload writes now additionally recheck Add-ons permission. Explicit allowIncompatible consent permits only the selected root setup mismatch; required dependencies still target the current server setup, with unavailable files and declared conflicts blocked. Consent is stored in the private review and content evidence. Older agents retain normal installs because default-false consent is omitted on the wire, while the new mismatch action is capability-gated. Corrected CLI capability JSON to retain additive feature discovery rather than discarding it. Focused Clippy, debug build, generated API/frontend check and Vite build passed; existing CSS/chunk/SDK warnings remain. Essential in-memory provider-resolution regression source protects default rejection/explicit consent/current-setup dependency resolution/unavailable rejection (existing staging tests miss resolution; <10ms expected); not run. Updated constructor in an existing test without running it. Native app remains live with HMR; normal dev preparation stages the updated bundled agent. Read-only local capability inspection confirms the running service is the earlier build (normal reviewed install supported; mismatch capability false). Owner should use Agent → Repair agent with servers stopped for the mismatch portion. No live content installation/service restart was performed. Review this new sheet before remaining source identification/update UI; P24.11 stays uncommitted until its surfaces are approved.

**Detail density revision:** Owner requested a more compact CurseForge project view matching Modrinth. Removed the extra 10px outer grid gap in detail mode, matched version-row padding (8px), 12px monospace version names, 3px metadata spacing, 15px project heading and compact icon/byline/stat grouping. Grouped About paragraphs into the same 8px paragraph/18px section rhythm instead of independent grid rows. Search-result layout and install behavior are unchanged. This feedback does not approve the pending installation sheet.

**Shared-sheet correction:** Cameron clarified that both providers must use the same sheet, rather than separate layouts with similar spacing. Removed the CurseForge-specific project header/About/version markup and all of its detail CSS from ProviderCatalogBrowser. Both provider paths now instantiate ProjectDetailSheet.svelte, whose one header, About renderer, compatibility badges, version rows, expansion and install controls own the layout. New catalog-project.ts adapts each provider's transport response into the shared data shape; unknown CurseForge follower/side metadata remains unknown. Safe provider-body text goes through the same inline renderer. Shared Show more versions preserves CurseForge pagination and exposes Modrinth releases beyond the previous 40-row display limit. CurseForge actions still open the existing exact-file dependency/installation review, with capability and availability checks; opening that review does not falsely mark a file installed. The change is an owner-requested refactor of the existing view, not installation-sheet approval. No tests added or run for presentation structure.

**Shared-sheet approval:** Cameron said “ui passes” after the actual shared ProjectDetailSheet refactor. Discovery and project details are approved. The separate installation/dependency/manual-file confirmation sheet remains awaiting explicit review, so source identification/update implementation has not begun.

**Owner-directed installation simplification:** Cameron explicitly rejected the routine CurseForge confirmation sheet, since Modrinth has none. Removed that surface and its approval requirement. Add/Install automatically invokes the agent's exact dependency preflight and then applies the resulting bound review; Install anyway supplies consent from that explicit action without a second confirmation. Declared conflicts/validation errors remain inline failures. Progress stays in the existing browser/project sheet and installed state updates only after operation success. Only an actual author-required manual file interrupts: ContentManualDownloadSheet shows the required files, bounded upload/checksum handling and Finish installation; it no longer lists the ordinary dependency set as a mandatory confirmation. Existing publication, update-conflict and pack review requirements remain unchanged. This supersedes earlier P24.11 notes asking the owner to approve an ordinary installation review. No tests added or run for this flow simplification.

**302 download repair:** Owner encountered “CurseForge returned status 302” while adding a mod. Artifact acquisition disabled redirects entirely, so the CDN handoff was treated as failure. Added a maximum of five explicit redirect hops with one overall download deadline, cancellation checks between hops, URL resolution via the standard url parser, and HTTPS/provider-host/port/user-info/query-credential validation before each request. API keys are attached only to the documented edge.forgecdn.net authentication host, never inherited by redirected storage endpoints. Final size/SHA1/SHA512 verification and non-overwriting staged publication remain intact; metadata requests retain their no-credential-redirect policy. Official authentication source: https://blog.curseforge.com/introducing-api-key-authentication-for-curseforge-file-downloads/. Added one essential pure-input redirect/credential-boundary regression source (<1ms expected), since existing fixture tests did not cover this download handoff; not run. Focused agent Clippy and native desktop cargo check passed after permitted dependency resolution; no tests or live mod installations run. Both Cargo lockfiles include the URL parser dependency. Updated agent build/development staging succeeded. After the Repair agent/retry instructions, Cameron reported “nice it worked,” verifying the reported CurseForge Add/302 case. Other P24.11 scenarios and conditional manual-download UI review remain pending.

**Final-review amendment and completion:** Cameron explicitly deferred all remaining UI review until the end of Phase 24. This supersedes the intermediate stops described above. Added ContentSourcesSheet through Components → add-on menu → Content sources and updates: exact-byte discovery, explicit exact CurseForge candidate linking, clearly labeled project-only hints, source removal and compatible update lookup. Update review shows pinned dependency files/warnings before applying; required manual files reuse the same bounded manual-download workflow. Reviewed updates remain bound to current installed hashes and the agent enforces stopped-server activation. Host snapshots/disposal guards prevent stale UI results; old agents receive an explicit upgrade message. Existing Modrinth update controls remain available. Frontend type-check passed with the same ten existing CSS warnings; Rust formatting passed and earlier focused Clippy/native checks cover unchanged Rust. No tests run. Conditional manual-download and source/update surfaces are recorded for final owner review, not claimed accepted.

### P24.12 — Select catalog packs during server creation

**Status:** implemented — awaiting owner verification
**Files:** clients/desktop-web/src/lib/sections/fleet/wizard/; crates/msc-agent/src/routes/servers.rs; crates/msc-application/src/
**What:** Add both-provider pack browsing to AddOnsStep while keeping local import paths. Show incompatible packs; matching release defaults; Try current setup vs automatic exact game/loader change, then staged-content conflict review. Revalidate on create, preserve draft on failure and respect supported-version floors.
**Verify:** `cargo check -p msc-agent && npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.12: select catalog packs during server creation`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

**Implementation evidence:** Added both-provider catalog pack browsing in the Add-ons step, with exact-release selection through the shared project sheet, matching-first release ordering and incompatible loaders/releases still visible. Download uses the journaled catalog acquisition API and purpose-bound staging, with cancellation, size/hash verification and cleanup; local archive import remains available. One base pack is enforced. The setup review offers automatic exact Minecraft/loader selection or explicit current-setup consent, and lists staged individual content with declared-version comparisons/unknown local-file evidence plus explicit keep/remove choices. Setup changes invalidate runtime selection, and a reviewed-setup key rejects changed drafts at create. Creation supports an explicit runtime override while preserving the original archive/manifest and full retained player collection; Minecraft 1.20 remains the floor. Fabric resolves its stable loader when current setup has no explicit loader pin; Forge/NeoForge require exact current loader selection. Added capability discovery and the additive create DTO/schema, plus CLI flags for the same explicit choice. CurseForge individual mods/plugins can now stage exact releases in the wizard and redeem against the newly created server; required manual files produce an actionable post-create warning rather than a false success. Creation failures return the original upload to staging, including native session ownership/cleanup guards, so the draft can retry; success consumes it once. Extended the existing local staged-pack regression source to cover failed-consumer restoration and committed consumption (<10ms expected), because it previously covered only redemption; not run. Existing DTO conformance source updated for the additive field without running it. Focused Clippy and frontend check/build passed with existing warnings; no tests, live server creations or service changes performed. New pack browser/setup review and creation choices await final owner review under the owner's amendment.

### P24.13 — Extend world content browsing to curseforge

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-agent/src/routes/worlds.rs; crates/msc-application/src/; clients/desktop-web/src/lib/sections/worlds/; clients/desktop-web/src/lib/sections/fleet/wizard/WorldStep.svelte
**What:** Provide CurseForge equivalents for Modrinth Java datapack browsing, details, selection, installation and updates wherever currently supported, including creation. Preserve D-030 world-slot ownership and existing Bedrock behavior; finish any inventory-specific provider gaps.
**Verify:** `cargo check -p msc-agent && npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.13: extend world content browsing to curseforge`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

**Implementation evidence:** Added CurseForge Java datapack class discovery/search/pagination, shared project/version detail rendering and exact-release install alongside Modrinth, including creation staging, provider-qualified duplicate handling, and existing-world View/update navigation. Dedicated capability discovery refuses old agents. The adapter validates the dynamic datapack class and exact Minecraft version, streams bounded provider acquisition with hash checks, and preserves D-030 slot ownership and active-world stopped-server rules. Author-required downloads use native/chunk staging and exact filename/size/SHA1 checks; remote/native ownership and post-read hashes are checked before use. Request cancellation/revocation interrupts acquisition and server/slot/version/permissions are revalidated afterward. Both-provider updates replace only recorded owned paths within the existing backup/rollback world transaction, preserve unknown paths, and compare per-file SHA512 baselines. Modified or legacy-unknown contents remain by default; explicit restore is bound to a fingerprint of current world bytes, pack records, server/slot and exact release. The archive is rechecked before replacement. Added persisted per-file checksums and DTO/schema mappings; required CurseForge dependencies must already be present for this world/version, with an explicit missing-project error rather than a false complete install. Named world-pack CLI search/inspect/install now accept provider choice, manual file and explicit restore fingerprint. Existing Bedrock behavior stays on its adapter. Added one essential controlled-ZIP regression source for owned-path replacement, preservation of unrelated terrain/content, stale-world rejection and unsafe recorded paths (<50ms expected); existing install/delete tests missed update ownership, not run. Updated an existing test constructor for the additive baseline map. Focused Clippy and frontend check/build passed with existing warnings; no tests or live world mutation performed. Manual/recovery and provider-specific world UI join final owner review; supported-platform/live-provider acceptance remains pending.

### P24.14 — Edit player content and review pack changes on desktop

**Status:** implemented — awaiting owner verification
**Files:** clients/desktop-web/src/lib/sections/addons/; clients/desktop-web/src/lib/sections/components/
**What:** Expose audience, required/optional, client-only additions, uncertain classification and server-mod player checks. Provide existing-server recovery, pack update and replacement comparisons with per-item keep-owner/restore-upstream and stale-result handling.
**Verify:** `npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.14: edit player content and review pack changes on desktop`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

**Implementation evidence:** Added PlayerContentSheet from Components with distinct server and player collections, uncertain classification/requirements, reviewed server-to-player copying, server-only classification, required/optional and selected/inactive controls, independent client-file additions, per-file license evidence, removal and explicit upstream restoration. New server mod installs/updates automatically check player requirements and present an inline review action. Player changes preserve active server bytes. Added PackChangesSheet for retained/local/catalog original recovery, exact missing-file retry/manual recovery, selected server-file recovery, three-way base update and replacement. Comparisons show per-side actions/conflicts, preserve owner state by default and require refreshed comparisons after per-item restore/audience/compatibility choices; replacement includes the explicit world-compatibility warning. Activation uses the real guarded operations and missing-content/stale-state checks. Uploads use native/bounded staging and server-side exact identification, rather than buffering/hash calculation in JavaScript. New prepared-file DTO/endpoint binds the original immutable upload to credential, server, path and SHA512, preserving the native consumer reservation across purpose rebinding. Extended the existing six-byte native ownership fixture to protect final-consumer guard lifetime (<1s existing fixture, no new network/timing conditions); not run. Pack reviews now expose structured missing-file descriptors for bounded exact-file recovery. Added cancellation/progress/error and older-agent capability states. Focused Clippy and frontend type-check/build passed with existing CSS/chunk warnings; no tests, live collection mutations or server changes were run. New player/pack surfaces are deferred to final owner UI review by the explicit amendment; physical recovery, customizations, cancellation/revocation and platform acceptance remain pending.

### P24.15 — Publish immutable dated content revisions

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/content_publications.rs; publication_review.rs; publication_datapacks.rs; content_guides.rs; crates/msc-agent/src/routes/components/content_publications.rs and router/capabilities; provider DTOs; OpenAPI/generated clients; tests/content_publication_safety.rs
**What:** Freeze reviewed content bytes/hashes and metadata under date/revision identity; lock and revalidate concurrent changes. Track unpublished changes; complete vs explicitly incomplete publications; re-export old revisions independently of current files or provider availability. Persist eligibility evidence and retention ownership.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.15: publish immutable dated content revisions`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added explicit-audience publication review/read/publish/history/snapshot and world-slot datapack inventory APIs. Reviews bind exact bytes, owner, collection/game/loader, selection/options and permissions; config/scripts require explicit per-file review, known distribution restrictions cannot be overridden, and API download permission alone is not redistribution evidence. Owner-selected optional content is included when eligible; selected unresolved content requires explicit incomplete consent. Freeze preflights unique bytes, streams independent copies, pins date plus monotonic publication revision, checksums manifest in the registry and freezes versioned guide files. Current source bytes are checked after copying before atomic registry commit; historical reads/re-export inputs do not depend on current content/provider availability. World datapacks come only from an explicit slot; active live content takes precedence over stale archives and archive inspection stages only selected datapack entries. Runtime/private/world data and arbitrary collection extras are excluded; child symlinks/case collisions are rejected. History pages report unavailable manifests and current unpublished changes. Guide text-v1 is a truthful development fallback; launcher-specific owner screenshots/HTML remain P24.20, not accepted here. Focused agent Clippy and frontend check passed (10 existing CSS warnings); schema references resolve. Three essential local tests protect independent frozen bytes, manifest checksum integrity and restricted-content/incomplete-consent boundaries; prior retention/JAR tests do not cover the publication registry/eligibility, expected <100 ms each, not run. Live publication/cancellation/revocation, large world/pack memory/disk behavior and physical platforms remain owner acceptance.

### P24.16 — Generate reviewed player and server zip archives

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/content_archives.rs; content_publications.rs/content_management.rs/publication_review.rs/operations.rs; crates/msc-agent/src/routes/components/content_archives.rs and staging/download/router/capabilities; provider DTOs; OpenAPI/generated clients; tests/content_archive_safety.rs
**What:** Stream snapshot ZIPs with audience-correct folders, manifest and instruction hooks. Include approved config/scripts and selected world-slot datapacks; exclude runtime/world/private files. Preflight space, reject unsafe paths/symlinks, avoid arbitrary config copying and make missing/restricted content conspicuous.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.16: generate reviewed player and server zip archives`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added asynchronous explicit-publication/audience ZIP generation and authenticated staged file delivery. The builder reads registry-verified frozen files only, checks SHA1/SHA512 while streaming, preflights space, rejects collisions/symlinks/overwrite, writes README.html/README.txt plus audience-scoped manifest/instructions and reports partial-cleanup failures. Optional inactive shaders/resource packs are included under optional-content; disabled JARs stay disabled. `zip-v1` pins historical path/README behavior. Corrected disabled server add-on discovery/publication source handling and narrowed credential-name filtering so legitimate mod/resource names are not rejected by substrings; actual credential/player-list files remain excluded from reviewed config selections. Staged ZIPs carry an addons-permission requirement checked again at download admission; existing generic downloads retain their prior behavior. Content operation cancellation is classified as addons for its initiator, acknowledged separately from failure, and remaining cleanup concerns remain visible. Revalidates host/server/publication binding before transfer admission and never returns archive bytes as JSON or a public sharing endpoint. Focused agent Clippy and frontend check passed (10 existing CSS warnings); schema references resolve. Two essential local ZIP tests cover audience isolation/frozen bytes/HTML escaping/inactive cosmetics and deterministic cancellation cleanup; snapshot-only tests do not inspect ZIP output, expected <150 ms each, not run. Disk exhaustion, large archives, revocation/cancellation, transfers and physical platforms remain owner acceptance.

### P24.17 — Include optional launcher importable mrpack files

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/content_mrpack.rs; content_environment.rs; content_archives.rs; content_publications.rs; publication_review.rs; content_guides.rs; crates/msc-agent/src/routes/components/; crates/msc-api/src/dto/provider_catalog.rs; API contract/generated client; focused archive/publication test sources
**What:** Generate optional nested .mrpack from the same revision with hashes, dependencies, environments and appropriate overrides. Resolve public downloads vs permitted embedded content, no credentials. Report per-file/launcher limitations and incomplete eligibility; no falsely importable plugin/server bundle.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.17: include optional launcher importable mrpack files`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Optional nested .mrpack streams the same frozen audience bytes into a stored outer ZIP entry; SHA1/SHA512 are rechecked without an inner temporary pack or full-pack memory buffer. Native format v1 index pins Minecraft and supported loader dependencies; files are embedded in client/server override layers, with exact per-file hashes, source and optional/side flags in the MSC manifest. Permitted bytes require the publication redistribution review; no provider credentials or mutable download URLs are needed. Eligibility reports unsupported plugin platforms and world-slot datapacks rather than producing a misleading import. Added explicit player loader metadata so Paper-family players use vanilla unless independent client mods require a reviewed loader. Legacy frozen guides remain intact with a visible player-header correction; new text-v2 guides use player setup. Required disabled mods/missing setup make readiness incomplete; native unsupported setup blocks .mrpack while ZIP remains available with explicit incomplete consent. Agent Clippy and frontend check passed (10 existing CSS warnings); no tests run. One essential small local nested-archive test protects byte identity, scope, inactive shaders and pinned setup, which existing outer-ZIP tests miss; expected <150ms. Actual launcher imports, large archives and platforms remain owner verification.

### P24.18 — Save exports locally or on the selected host

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-agent/src/routes/components.rs; clients/desktop-web/src-tauri/src/native_transfer/; clients/desktop-web/src/lib/platform/; clients/desktop-web/src/lib/api/
**What:** Use authenticated bounded binary transfer for desktop-local saves from any paired agent; add authorized host-local export destination handling. Revalidate host/server/publication/destination and overwrite consent; atomic completion, progress, cancellation and disconnect/disk-full cleanup. Preserve older export clients explicitly.
**Verify:** `cargo check -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.18: save exports locally or on the selected host`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added selected-host saves within the existing admin-authorized server filesystem root at exports/, including existing subfolders. Addons plus Admin permission is mandatory; absolute paths, traversal, symlink destinations and missing parent directories are refused. Destination inspection returns exact existing-file SHA512; overwrite requires the matching value and revalidation immediately before atomic completion. Generation uses a private sibling file and never replaces an existing destination on failed/cancelled work. Host/server/publication registry and credential permissions are checked throughout generation and at commit. Selected-host operations report their actual host path; ordinary staged-download clients remain unchanged. Added typed API helpers that pass completed exports into the existing native binary save picker (local or remote agent), preserving bounded transport, digest validation, progress, cancellation and partial cleanup without JS archive buffers. Agent Clippy, native desktop cargo check and frontend check passed; existing SDK discovery warnings and 10 CSS warnings remain. Essential local host-save test protects stale overwrite consent/cancellation/path escape; existing native tests do not cover agent destinations, expected <50ms, not run. Physical overwrite/disk-full/disconnection/cancellation and Windows/macOS/Linux acceptance remain pending.

### P24.19 — Add the publish and export review sheet

**Status:** implemented — awaiting owner verification
**Files:** clients/desktop-web/src/lib/sections/addons/; clients/desktop-web/src/lib/sections/components/
**What:** Implement Player/Server, categories/items, date/revision/name, optional .mrpack, completeness, size and named local/remote destination. Export prior revisions or publish current changes deliberately; show progress/results and actionable missing content. Follow antiAIslop.md.
**Verify:** `npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.19: add the publish and export review sheet`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

**Implementation evidence:** Added ContentExportSheet through Components → Export content, with Player/Server selection, dated immutable revision history/paging, unpublished-change notice, optional version name, included files/byte size/optional-active status, exclusions and explicit incomplete publication/export consent. Current-content review is independent of historical reads, so unreadable current files or unavailable world inventory cannot hide existing frozen revisions. Publication approvals bind to exact candidate hashes; config/scripts require per-file review, unknown licenses require evidence and known restrictions remain blocked. Added read-only candidate metadata to publication reviews so owners do not calculate checksums or approve unseen path identities. Explicit world-slot datapack selection uses current exact inventory and evidence; no terrain/runtime/private data is selected. Player loader/version overrides are explicit where inference is insufficient. ZIP exports offer optional nested .mrpack only when the frozen revision is eligible, with reasons/warnings otherwise. Local saves use bounded native transfer even for remote hosts; host saves use the existing exports root and permission checks. Existing destination replacement binds to the hash the owner actually reviewed and requires renewed consent if the file changes. Real operations, progress/cancellation, staged-download cleanup, stale-review errors, old-agent messages and historical completeness are wired. Text guide fallback is labeled honestly; illustrated assets remain P24.20. Focused Clippy and frontend check/build passed with existing warnings. No tests, live publications/downloads, service repairs or releases were run; existing publication eligibility/identity safety source covers approval enforcement and was not duplicated for presentation. New export/publication surfaces remain pending final owner UI review.

### P24.20 — Bundle illustrated offline launcher instructions

**Status:** prepared — incomplete; awaiting owner screenshots and launcher-build verification
**Files:** docs/msc2/addons/launcher-guides/; crates/msc-application/src/content_guides.rs
**What:** With Cameron-supplied screenshots, create versioned offline HTML/text guides for Minecraft Launcher, Prism, Modrinth App, CurseForge App and SKlauncher. Verify exact launcher import capabilities; show manual copy and optional .mrpack routes, correct loader, isolated installs, optional content and clean updates. Add separate Server instructions; no external assets or invitation links. Do not complete this step without screenshots.
**Verify:** `cargo check -p msc-application`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.20: prepare offline launcher instructions`
**Batch:** C — screenshot prerequisite and offline guide review

**Preparation evidence:** Added five launcher-specific text drafts and bundled local-only HTML/text assets with an index, exact publication setup readme, manual-copy route, compatible .mrpack route, separate instance/update and optional-content guidance. Official primary documentation was checked on 2026-11-06; direct .mrpack import is not claimed for Minecraft Launcher, CurseForge App or stable SKlauncher 3.2. SKlauncher 4.0 beta is kept distinct. New publications freeze text-v3-launchers; historical guide bytes remain unchanged. HTML escapes text, disables scripts/remote assets and uses only bundled local navigation. Focused Clippy passed; no tests or physical launcher verification run. Owner screenshots, exact installed launcher build validation, illustrated asset integration/render review remain required. This preparation commit does not complete P24.20 or its illustrated-guide gate.

### P24.21 — Expose content and export operations in the cli

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-agent/src/cli/content.rs; cli/mod.rs; routes/components/provider_catalog.rs; routes/components.rs; API contract/generated client; docs/msc2/addons/phase24-cli.md; client-capability-matrix.csv
**What:** Add both-provider catalog/detail/install/identification/update and pack selection, player management, recovery, conflict review, publication and local host export commands. Require explicit conflict/destination/incomplete choices, safe secret input and scriptable outputs; reflect actual capabilities.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.21: expose content and export operations in the cli`
**Batch:** A — autonomous foundations; no routine owner handoff

**Implementation evidence:** Added named content search/inspect/acquire/get/request/stage-pack/export commands with explicit provider/server/release selection. Review, individual install/source/update, independent player changes, retained recovery, pack update/replacement/manual recovery and publication actions map to closed named routes; bounded JSON files/stdin carry per-item decisions and every mutation requires confirm. Admission actions optionally stream a local file, verify the purpose-bound route and request cleanup on failure. Capability discovery refuses unsupported mutation families; agent validation remains authoritative. Long work supports no-wait, JSON operation results and existing Ctrl-C cancellation. Host exports require explicit destination/overwrite/incomplete/mrpack choices. Added provider-category Modrinth discovery independent of active-server selection (legacy filtered routes unchanged); pack results remain unfiltered for compatibility review. Documented usable examples and body/flow requirements. Focused Clippy and frontend check passed (10 existing CSS warnings); built CLI content --help ran successfully. No tests added or run for command mapping; physical provider/auth/upload/conflict/export acceptance remains pending.

### P24.22 — Expose content and export operations in the tui

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-agent/src/cli/tui/content.rs; tui.rs; tui/create.rs; cli/transport.rs; docs/msc2/addons/phase24-tui.md; client-capability-matrix.csv
**What:** Provide host-local interactive provider browsing, creation compatibility choices, audience editing, dependency review, recovery, update/replacement and publish/export review. Use the shared API and host-local destinations; preserve SSH behavior and existing accepted TUI flows.
**Verify:** `cargo check -p msc-agent`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.22: expose content and export operations in the tui`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

**Implementation evidence:** Added a keyboard-driven content workspace opened with Shift-P from a selected server workspace, plus both-provider staged creation browsing. Named fields and choices cover source hints/exact linking, provider releases and pagination, explicit incompatible choices, separate player content and server requirements, retained recovery/manual files, three-way pack changes and audience/restoration decisions, dated immutable publications/history and audience exports. Publication supports explicit world-slot datapack paths and per-file permission/configuration review. Ordinary compatible provider installation runs directly; only missing manual files interrupt. Missing-key entry and developer console are offered only after the agent reports a missing key. Long operations run in background workers with real operation cancellation requests; writes remain server/revision/checksum bound. New local exports use an atomic non-overwriting commit; host replacements bind the reviewed destination hash. Draft additions are installed only after creation succeeds and use the operation-returned server identity; follow-up errors cannot create duplicate servers. Focused Clippy and formatting passed; no tests or live mutations were run. SSH/Windows/Linux interaction, launcher behavior and final UI review remain owner acceptance tasks.

### P24.23 — Account for retained packs and publication storage

**Status:** implemented — awaiting owner verification
**Files:** crates/msc-application/src/; crates/msc-agent/src/routes/; clients/desktop-web/src/lib/sections/; docs/msc2/storage/
**What:** Integrate retained originals/player content/publications/staging with Phase 21 ownership and accounting. Protect live/referenced snapshots, export-in-progress and rollback data, prevent double counting, explain storage use and reuse explicit reviewed cleanup; no automatic historical-revision deletion.
**Verify:** `cargo check -p msc-agent && npm --prefix clients/desktop-web run check`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.23: account for retained packs and publication storage`
**Batch:** B — continue automatically; stop at each new sheet/view for owner review

**Implementation evidence:** Split registered-server content storage into retained originals, separate player content, immutable publication revisions, review/registry metadata, activation/install rollback and datapack staging, plus completed host exports. Existing root partitioning and physical-identity deduplication prevent parent/child and hard-link double counting. Content-specific protection reasons and the existing Storage detail view explain independent copies and reference protection. Cleanup reuses transfer-owner exact-selection review for eligible expired/completed export/native staging, rechecking readers/reservations/fingerprints under existing locks; retained snapshots, unknown references and rollback never become generic cache candidates. No age-based historical deletion or orphan sweep was added. Extended the existing controlled storage discovery fixture with four tiny content/rollback files to protect disjoint attribution and totals (essential because generic scanner fixtures do not exercise application rules; expected <1s, not run). Formatting/focused Clippy passed; frontend check passed with existing CSS warnings. No live scans, cleanup, tests or releases run.

### P24.24 — Record integrated owner acceptance and gate evidence

**Status:** candidate packet prepared — awaiting owner physical acceptance; gate remains open
**Files:** docs/msc2/addons/phase24-acceptance.md; phase24-execution.md; phase24-contract.md; client-capability-matrix.csv; rolling-plan.md; msc2-port-plan.md; desktop AddonsSection/HealthSection/PluginBrowserSheet/SetupIntro; tui/create.rs
**What:** Prepare and record Cameron-run physical scenarios listed in scope across supported OSes, local/remote and real launchers using disposable content. Record missing screenshots/provider prerequisites and unresolved results honestly. No automated tests, release runs or step/phase closure by the implementing agent. Other agent gate review follows only when Cameron requests it.
**Verify:** `git diff HEAD^ -- docs/msc2/addons/phase24-acceptance.md docs/msc2/client-capability-matrix.csv docs/msc2/rolling-plan.md`. Cameron also verifies the corresponding behavior scenarios in `addons/phase24-scope.md` and the P24.1 acceptance record.
**Commit:** `P24.24: record integrated owner acceptance and gate evidence`
**Batch:** D — final owner acceptance; no agent phase closure

**Final candidate evidence:** Added requirement-to-source/navigation/physical-evidence map, exact staged development agent hash, final UI route checklist, permitted check results and explicit missing screenshot/platform/launcher acceptance. Final source inspection found two omitted consumer integrations: legacy Add-ons still had independent Modrinth-only search, and diagnostic CurseForge repair only displayed a review-required error. Reused the shared provider browser/source-update/player screens in Add-ons; Health now follows the provider-qualified error into those same scoped flows and checks player requirements after content changes. Added initialProvider to the shared browser, corrected obsolete Modrinth-only help/TUI text, and updated capability notes. Formatting/focused Clippy, frontend check/build, native check and development agent/sidecar staging passed; target and staged SHA256 match. No tests, live mutations, service restart, releases or independent phase review run. P24.20 and all unrecorded physical results remain incomplete; this packet is not acceptance or phase closure.

### P24.25 — Arrange creation add-on choices in a two-column grid

**Status:** implemented — awaiting owner visual verification
**Files:** clients/desktop-web/src/lib/sections/fleet/wizard/AddOnsStep.svelte; docs/msc2/rolling-plan.md
**What:** Owner-requested creation layout revision: show Browse Mods and Browse Modpacks on the first row, Import Modpack and Import Mods on the second. Remove the detached top modpack button; keep all four existing actions wired to their current browser/import flows. Preserve plugin-specific wording and offer catalog pack browsing in the staged-content header too.
**Verify:** `npm --prefix clients/desktop-web run check`. Open Add Server → Fabric → Add-ons and confirm the 2×2 ordering and each action.
**Commit:** `P24.25: arrange creation add-on choices in a two-column grid`
**Batch:** owner-requested bounded visual revision; final verification pending

No tests added or run for this reversible layout change. Rust is unchanged; prior Rust formatting/Clippy results remain applicable. Frontend check passed with the existing CSS warnings.
