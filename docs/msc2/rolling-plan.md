# MSC 2 — Rolling Plan

Implemented steps awaiting verification were marked Done and moved to [the archive](rolling-plan-archive.md) at Cameron's direction on 2026-10-08. Planned work, pending publications and independent phase acceptance remain below.

### P16.43 — Plan a complete Windows MSI refinement

**Status:** Plan written; awaiting Cameron's review. PLAN scope: review the entire MSI process and propose improvements after the owner reported a briefly blank opening page (2026-10-07).
**Files:** `docs/msc2/rolling-plan.md` only.
**What:** Record the source/artifact findings, proposed installer experience and P16.44–P16.50. No installer implementation, installation, service control, tests, builds or release runs in this conversation.
**Verify:** `git show --check --stat --oneline HEAD`
**Batch:** P16.43 only (PLAN). Future execution requires an owner instruction naming its step or range.
**Commit:** `P16.43: plan complete windows msi refinement`

**Review findings and limits:**

- The current `clients/desktop-web/src-tauri/tauri.conf.json` uses Tauri's stock WiX UI. The sole custom fragment, `packaging/windows/desktop-cli-path.wxs`, adds `[INSTALLDIR]agent` to machine PATH with a package-owned registry marker. There is no MSC-specific introductory copy, custom UI, or MSI service lifecycle integration.
- Read-only Windows Installer database inspection of `C:\Users\Cameron\Downloads\msc2-0.1.15-windows-x86_64.msi` found `PrepareDlg` at UI sequence 49, followed by searches/costing and `WelcomeDlg` at 1298. The preparation and welcome text exists in that artifact; preparation Back/Next are disabled, while Cancel is authored visible/enabled. Cameron confirmed the blank page is brief and advances. This strongly points to preparation/initial painting, but does not prove why text or Cancel initially fails to paint/respond. Do not describe it as missing strings or a permanently stuck installer. The exact owner-observed MSI version remains unconfirmed; this older downloaded artifact is a baseline, not evidence about every current release. SHA-256: `1952d89bb5538f2487e5563135619dfe063aee395edfe5effe107e4aa247ffc2`.
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


### P18.46 — Add biome colors and legend to the world viewer

**Status:** Implemented; awaiting Cameron verification.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`, this plan.
**What:** Add a collapsed Biomes panel beneath Players with Vantage's existing biome colors and click-to-highlight legend. Update shares as tiles stream; preserve the color toggle across refresh/dimension changes and clear highlighting. Shares describe loaded terrain. Missing biome data is explicit. Generic modded shapes remain unchanged; D-042 stays in force. No tests added or run.
**Verify:** `cd clients/desktop-web && npm run check && npm run build`
**Batch:** P18.46 only.
**Commit:** `P18.46: add world map biome colors and legend`

**Owner visual verification:** Open the modded Java map, expand Biomes beneath Players, enable colors, select/deselect a biome, stream terrain, refresh and switch dimensions. Confirm the toggle persists, highlighting clears on reload, and both panels collapse independently.

**Implementation checks:** Frontend type/Svelte check passed with zero errors and existing unrelated warnings; production build and whitespace check passed. No Rust changes or tests. Runtime visual acceptance remains pending.
