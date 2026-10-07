# MSC 2 — Rolling Plan

### P18.30 — Fit the world map inside its tab

**Status:** Implemented; awaiting Cameron's window-resize verification.
**Files:** ApplicationShell.svelte, WorldMapViewer.svelte, this plan.
**What:** Replace viewport-minus-fixed-offset sizing and 620/520-pixel map minimums with the actual space remaining inside the Worlds tab. The active map pane fills a bounded flex container; the header stays visible and the canvas shrinks with window/console resizing. Preserve scrolling for the ordinary Worlds list and other tabs. Follow the existing design without adding controls or decoration.
**Verify:** `npm --prefix clients/desktop-web run check`; Cameron opens Java/Bedrock maps, resizes the window, opens/resizes/collapses the console, and confirms the map header and bottom controls remain visible without vertical tab scrolling; return to Worlds list and check ordinary scrolling.
**Batch:** P18.30 only, owner-requested layout fix.
**Commit:** `P18.30: fit world map to available tab space`

**Checks:** Svelte check passed with zero errors and eleven existing warnings. Frontend build, native desktop build, native Clippy and whitespace checks passed with existing warnings. Inspected the compiled map-container selectors. No tests or live visual verification ran. Rebuilt desktop: `clients/desktop-web/src-tauri/target/debug/msc2-desktop-web`; installed desktop packages are not replaced by this build. Owner visual acceptance remains pending.


### P18.29 — Correct map staging storage estimates

**Status:** Implemented; awaiting Cameron's large-map verification.
**Files:** Agent map staging and Java compatibility estimate; headless installation guide; this plan.
**What:** Owner found 14 GiB in an old TMPDIR renderer copy and reclaimed it after stopping the agent. Root now reports 14 GiB available. P18.28 demanded 26,685,489,152 bytes by doubling the source estimate; remove that arbitrary multiplier and estimate one working copy plus existing headroom. Preserve actual normalized-size free-space checks and underlying write/quota failures. Subtract bytes already written from live reservations because filesystem free space already accounts for them; failed/racing inspection conservatively retains the full reservation. No source-world deletion, automatic abandoned-copy purge or retired repair feature.
**Verify:** `cargo clippy -p msc-agent --bin msc`; Cameron installs the corrected agent and opens Tectonic, confirms terrain loads and storage errors retain useful diagnostics, and repeats standard Java/Bedrock observations.
**Batch:** P18.29 only, continuation of P18.28 owner verification.
**Commit:** `P18.29: correct map staging storage estimates`

**Checks:** Formatting, agent Clippy, diff whitespace and locked release build passed with existing warnings. No tests or remote launches ran. Updated `target/map-staging-agent/msc` SHA-256: `175cc894204c031c2619afdc265ea44d235b11f3e7fcfbbd0c8eb8e9d6528880`; highest glibc ABI 2.39. Owner-confirmed reclamation returned root from 100% usage/7.2 MiB free to 75% usage/14 GiB free. New exact-artifact rendering remains pending.


### P18.28 — Use MSC-owned disk storage for large map staging

**Status:** Implemented at Cameron's request on 2026-10-06; awaiting owner verification.
**Files:** `crates/msc-agent/src/map_staging.rs`, agent manifest/root lockfile and module registration; shared map snapshots in `backup_operations.rs`; Java renderer/compatibility preparation and Bedrock tile ownership; `docs/msc2/clients/headless-installation.md`; this plan.
**What:** Java compatibility copies/renderer output, shared Java/Bedrock snapshots and Bedrock tile output now live in unique directories under the platform's agent application-data directory (`MSC2_DATA_DIR/map-staging` when configured), independent of TMPDIR. Unix directories use mode 0700; Windows inherits application-data access controls. Ownership guards remove only their own copies after failure/cancellation/cache replacement; Java stops its child first, and Bedrock stops its helper on wait errors/timeouts. Existing world-save hold/resume commands and copy limits remain intact. A filesystem admission lock and live operation leases account for concurrent size estimates plus 64 MiB headroom; Java reserves twice the modern-region source estimate and rechecks actual normalized sizes before writes. Estimates are conservative, not output bounds or quota guarantees. Errors/logs retain the working location and underlying failures. No global temporary-directory override is required. Retired modded repair/import/Prism capture remains retired.
**Verify:** `cargo clippy -p msc-agent --bin msc`; then Cameron installs the staged agent, removes only the earlier map-temp.conf override using the headless guide, opens the large saved Tectonic map, confirms staging beneath MSC2_DATA_DIR, and checks standard Java/Bedrock rendering and source-world preservation. Check failed/cancelled copies are removed, Java renderer copies disappear after idle expiry, and cached snapshots/Bedrock tiles are removed when replaced/released. Closing the viewer does not immediately release every cache. Linux rendering and Windows/macOS physical observations remain pending until supplied. No release workflow gates or test execution authorized.
**Batch:** P18.28 only; executed on 2026-10-06.
**Commit:** `P18.28: stage map copies on disk`

**Checks:** Agent type-check, Clippy, Rust formatting, diff whitespace and locked release build passed with the existing infrastructure request, Bedrock runtime mutability and unused auth helper warnings. No tests, Minecraft launches, remote installations or release actions ran. Ubuntu verification binary: `target/map-staging-agent/msc`, SHA-256 `9c06c3bb07bc90cf60b6e74063ad0553d1f448e3a7d430e0b9aa1cb0bae82e87`; highest required glibc ABI 2.39, within cambookpro13's recorded 2.43. Platform storage/free-space and lease operations use fs2's Linux/macOS/Windows implementations; physical Windows/macOS checks have not run.

**Recovery decision:** Unlocked abandoned directories are retained, not deleted by age or on restart: an agent crash can leave a renderer child alive. Manual recovery requires stopping the agent and terrain helpers and checking each directory's ownership. Original worlds, old TMPDIR contents and unrelated files are never purged by this change. This avoids unsafe recovery while leaving crash leftovers as an explicit storage maintenance limitation.

**Essential regression:** Added `staging_guard_removes_only_its_owned_copy` because deleting the wrong directory could destroy a world. It creates tiny controlled sibling files and an owned lease, drops the guard, and confirms only that owned copy disappears while an original-world sibling and abandoned-copy sibling retain their bytes. No network, timing, global environment or free-space assumption; expected runtime under one second. Added but not run; Cameron has not authorized a test command.


**Observed failure:** On cambookpro13, the restored agent logged `Java terrain compatibility preparation failed: Disk quota exceeded (os error 122)` at 18:43:35 on 2026-10-06. The agent currently creates `msc-map-renderer-*` beneath `std::env::temp_dir()`. Host /tmp is a 3.9 GiB tmpfs, showing 2.9 GiB available after failure; the main filesystem has 14 GiB available. Inspection inside the service's mount namespace shows tmpfs with `usrquota` and PrivateTmp. The error and mount configuration are consistent with a per-user tmpfs quota; the exact quota and peak staging usage have not been measured. Main-disk free space does not rule out quota exhaustion.

**Temporary workaround applied by Cameron:** Created `/home/camerontemple/.local/share/msc2/tmp` with mode 0700 and `/etc/systemd/system/com.ctemple.msc2.agent.service.d/map-temp.conf` setting `TMPDIR` to that directory. Reloaded systemd and restarted the agent; owner reported `active`. This persistent host-local override affects all agent temporary files, not just map staging. Cameron subsequently reported “java world loaded”; this records the workaround observation without claiming the application fix has been verified. Once the application fix is installed, guide removal of only this override and verify the default staging path; retain existing temporary data until its ownership and live use are known. Original worlds and client instances were not changed by the workaround.

**Related owner verification:** After installing the exact-size download correction, Cameron reported the standard Java map “looks good now.” This records the rendering observation without closing the broader map acceptance gate. Bedrock and the large Tectonic map remain pending.


### P18.27 — Correct exact-size vanilla texture downloads after retirement

**Status:** Implemented; awaiting Cameron's map verification.
**Files:** `crates/msc-infrastructure/src/jar_provider.rs`, its existing `tests/jar_provider.rs` size-cap regression, this plan.
**What:** Owner logs from the restored v0.1.23 agent show vanilla Minecraft 1.20.1 and 26.3 client downloads incorrectly failing at their exact publisher-declared byte counts. The full retirement rollback also removed the general HTTP inclusive-size correction; restore only that correction. Let ureq read one lookahead byte and independently reject payloads larger than the original bound. Preserve checksums, timeouts and maximum sizes. No retired repair/import/capture/Prism paths return.
**Verify:** Reopen the standard Java world from the rebuilt desktop with the corrected agent and confirm textures and terrain load; inspect the service log if preparation fails. Repeat the original Bedrock verification independently.
**Batch:** P18.27 only; the withdrawn repair gate review is not resumed.
**Commit:** `P18.27: allow exact-size vanilla texture downloads`

**Checks:** Rust formatting, agent Clippy, release agent build and whitespace checks passed with existing warnings. Exact binary `target/map-repair-retirement-v0.1.23/msc-textures-fixed` SHA-256 `5c148a1df3a3c61329d679976288edb9462f610d6c3d49fad4a29318283a331a`; highest required glibc ABI 2.39, below the host's recorded 2.43. Retired native capture registrations and agent capture routes remain absent. No tests, Minecraft launches or release actions ran. Remote installation and actual standard Java/Bedrock rendering remain owner verification.

**Essential regression:** Restore the earlier owner-passed extension of the existing HTTP size-cap case: a controlled 100-byte response must succeed at cap 100 and 101 bytes must fail at cap 100. A local loopback server with no timing or external service dependency, expected runtime under one second. This protects the actual inclusive download boundary rather than test counts. Earlier owner run passed both transport cases; no new test execution is authorized or performed in this follow-up.


### P18.26 — Retire the map repair workflow

**Status:** Implemented; awaiting Cameron's verification.
**Files:** Revert P18.13–P18.25 and the uncommitted repair follow-ups to application baseline `f6a1f794`; decision register and vision notes; this plan.
**What:** Cameron rejected the private Prism sign-in/capture design on 2026-10-06 and explicitly requested returning to before any repair work. Restore the standard Java and Bedrock saved-terrain viewer, agent, API, CLI and build-only release packaging from the v0.1.23 preparation commit. Remove modded-resource repair/import/capture UI, exporter helpers and acceptance promises. Keep Git history and a private recovery snapshot of uncommitted source at `/home/camerontemple/.local/state/msc2/retired-map-repair/20261006-165119`. Do not delete server worlds, original client instances, credentials or existing private captures. This supersedes the earlier objective to finish and release the repair feature; it is a retirement, not successful rendering acceptance.
**Verify:** Reopen the rebuilt desktop with a v0.1.23 agent; view one known standard Java world and one known Bedrock world, confirm saved terrain loads, 2D/3D navigation works, and the repair/private-client controls are absent.
**Batch:** P18.26 only.
**Commit:** `P18.26: retire map repair and restore released terrain maps`

**Checks:** Frontend build, native desktop build, Rust formatting, agent/native Clippy and Svelte check passed with existing warnings (zero Svelte errors, eleven warnings). Application source matches the v0.1.23 tag exactly outside these retirement records. No tests or Minecraft launches ran. Published Linux headless archive checksum verified against SHA256SUMS: `ea0324fdcf010b4be6a37f92ba3e7d738468a1fb93ddb91fb11781b819ec87ef`. Extracted agent SHA-256: `583f99ada20c99af52e672ecb6f48dbd65fb274781b99f2e43f741e55a12d5e6`, staged at `target/map-repair-retirement-v0.1.23/msc`. Installing it and manual map acceptance remain pending. No release/tag changes.


### P16.42 — Publish v0.1.23 with Java map dependency recovery

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.23.md`, this plan.
**What:** Increment the latest remote/published tag v0.1.22 to v0.1.23 and synchronize owned release identities without changing registry dependency versions. Include P18.12's complete headless helper updates, missing Vantage recovery, verified version-matched Java assets and diagnostic errors. Push main and the new immutable tag through the unchanged four-platform build-only workflow, preserving nine artifacts, checksums and signed update metadata. Cameron explicitly requested implementation, commit pushes and the next release tag.
**Verify:** `gh release view v0.1.23 --json tagName,isPrerelease,assets,url`
**Batch:** P16.42 only.
**Commit:** `P16.42: prepare v0.1.23 release`

**Preparation:** Inspected the complete active release workflow and successful v0.1.22 publication. No release gates changed or tests run. Implementation formatting, Clippy and frontend checks recorded under P18.12; synchronized source identity and locked metadata checked before tagging. README retains the actually published v0.1.22 while introducing the new candidate. Recent successful release runs took 38–47 minutes. Ubuntu map acceptance remains Cameron's check.

### P18.12 — Prepare Java map dependencies on headless hosts

**Status:** Implemented; awaiting Cameron's verification.
**Files:** Agent Java terrain bridge and new dependency module; headless CLI updater; agent manifest/lockfile; shared Vantage release pin and stager; map loading copy; headless installation guide; this plan.
**What:** Correct the headless updater's agent-only replacement list: validate and install Vantage, the Bedrock map helper and Vantage license together with the agent, with rollback for every replaced/new component. Validate the complete extracted payload before stopping the agent. Recover Vantage missing from older installations into the agent's own cache using the same platform-specific SHA-256 pin as release packaging; retain an explicit MSC2_VANTAGE_BIN override. Determine Minecraft's version from the saved world's level.dat, falling back to the configured Minecraft version. Download official version metadata/client bytes, verify their published SHA-1 digests and sizes, and safely extract only Minecraft assets/game data into a versioned cache. Reuse complete caches offline; reject incomplete caches and unsafe or oversized archives. Pass the matching asset directory explicitly to Vantage. Distinguish missing overrides, renderer download, client asset preparation, terrain preparation, startup exits and startup timeout; retain bounded stderr diagnostics in agent logs and redact the private renderer token. Explain terrain/texture preparation in the existing loading text. Do not modify game saves, install a launcher or restart Minecraft during dependency setup.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && npm --prefix clients/desktop-web run check`
**Batch:** P18.12 only.
**Commit:** `P18.12: prepare java map dependencies on headless hosts`

**Checks:** Formatting, ordinary agent Clippy, Python stager syntax/shared-pin inspection, Svelte check and diff whitespace passed. Existing warnings: unused infrastructure request, unnecessary mut in Bedrock runtime, unused auth helper, eleven Svelte warnings. New unit regressions compiled during cargo check; compiling all agent test targets is blocked by existing cli_service.rs references to four removed CommonArgs fields. No tests, map smoke suites or release workflows run for implementation verification.
**Essential regressions:** Three controlled unit regressions protect helper installation/complete rollback/refusal before mutation, version-matched asset selection/cache reuse/incomplete-cache repair, and checksum/path-traversal rejection without publishing partial assets. Existing coverage missed the helper omission and this new first-use asset path. Tiny local ZIPs, a fake HTTP transport and unique temporary directories with cleanup; no live network, sleeps or environment assumptions. Expected runtime under one second each after compilation; not run.
**Manual acceptance:** Update the Ubuntu agent, then reopen the saved Paper/Tectonic Overworld from Fedora when convenient after Chunky finishes. With the older updater's missing Vantage, confirm first use recovers the renderer and prepares Minecraft textures under MSC2_DATA_DIR/map-dependencies. Reopen and confirm cache reuse without another download; saved terrain should render without joining Minecraft. Inspect agent logs for preparation/failure details. A future headless update should install both helpers beside msc; fresh archive installers already do so. Offline first use reports the relevant dependency failure and can be retried after connectivity returns. Vanilla assets do not add third-party mod resource packs.

### P16.41 — Prepare README publication wording for v0.1.22

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `README.md`, this plan.
**What:** At Cameron's explicit request, identify v0.1.22 as the latest published build and update download/install examples accordingly. Remove candidate/building wording and retain the fixes and prerelease acceptance limitations. This wording is prepared ahead of publication: GitHub returned release not found at editing time; Cameron was informed. Do not change the active release tag or restart the build.
**Verify:** `git show --check --oneline HEAD`
**Batch:** P16.41 only.
**Commit:** `P16.41: update readme release references to v0.1.22`

**Checks:** README diff and whitespace inspected. Documentation only; Rust checks are not applicable. No tests run.

### P16.40 — Update README for the corrected v0.1.22 candidate

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `README.md`, this plan.
**What:** Describe the modern Java datapack fix and terrain-generation limitation, link the corrected release build, retain v0.1.21 as the newest published release, and update installation/update examples from stale v0.1.18 to v0.1.21. Preserve the active release tag and workflow; push documentation to main only.
**Verify:** `git show --check --stat --oneline --grep='P16.40' HEAD`
**Batch:** P16.40 only.
**Commit:** `P16.40: document corrected v0.1.22 candidate`

**Checks:** Inspected README diff and whitespace; checked replacement workflow is in progress. Documentation only; Rust formatting/Clippy are not applicable. No tests run and no release rebuild started.

### P16.39 — Replace the cancelled v0.1.22 candidate

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** `clients/desktop-web/src-tauri/Cargo.lock`, `docs/msc2/release/v0.1.22.md`, this plan.
**What:** Restore serde_repr 0.1.21 with its original checksum; the P16.38 version bump accidentally changed this unrelated registry package to nonexistent 0.1.22. Include P12.249 modern datapack support and retain the headless Linux fix. Cameron explicitly requested cancellation/removal and confirmed replacement of the unpublished v0.1.22 tag. Remove cancelled run 37177543797 and its old tag, then publish the corrected source using the unchanged build-only release workflow. No published v0.1.22 release or uploaded workflow artifacts existed when inspected.
**Verify:** `gh release view v0.1.22 --json tagName,isPrerelease,assets,url`
**Batch:** P16.39 only.
**Commit:** `P16.39: repair and replace v0.1.22 candidate`

**Checks:** Inspected all three completed failed platform logs: identical unavailable serde_repr dependency, before packaging. Fourth build cancelled. Restored the original registry version/checksum from the prior tag. Locked desktop metadata, synchronized application versions, formatting and diff whitespace checks passed. Application Clippy and addon regression compilation recorded in P12.249. No tests run or workflow gates changed. Nine artifacts, SHA256SUMS and signed update metadata remain required. Expected build time about 40 minutes based on recent successful runs; physical acceptance remains Cameron's verification. Tag replacement is a specifically owner-authorized exception, not a routine retry.

### P12.249 — Accept modern Java datapack metadata

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/addons.rs`, `crates/msc-application/tests/addons.rs`, this plan.
**What:** Accept modern min_format/max_format bounds as integers or one/two-element major/minor arrays, including inclusive upper major versions. Retain legacy positive integer pack_format support and require a non-null description. Reject malformed, missing or reversed modern bounds before changing the world archive. Preserve provider Minecraft-version checks, archive safety, overlays, backups and original pack bytes. Confirmed official Tectonic 3.0.29 datapack targets 26.3 and declares min_format/max_format 121 without pack_format.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-application --lib && cargo check -p msc-application --test addons`
**Batch:** P12.249 only.
**Commit:** `P12.249: accept modern datapack format metadata`

**Checks:** Formatting and ordinary application Clippy passed; focused addon regression source compiled. Existing infrastructure unused-request warning remains. No tests run. Extended the existing controlled archive regression to cover modern integer/array bounds, preserved metadata bytes, and rejection without world modification. Essential because valid modern packs were blocked and malformed metadata must not modify saved worlds; no network, runtime or timing assumptions; expected additional runtime under one second.
**Manual acceptance:** With the updated agent, add Tectonic 3.0.29 to a new Paper 26.3 world during creation and to a stopped world's datapacks afterward. Confirm metadata rejection is gone and Paper lists/enables the pack. Already generated terrain is not regenerated by installing it afterward.

### P16.38 — Publish v0.1.22 with the headless Linux status fix

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.22.md`, this plan.
**What:** Increment the latest remote tag v0.1.21 to v0.1.22 and synchronize release identity. Include P12.248's headless Linux status correction. Integrate the two existing remote commits before publication and retain their changes. Push main and the new immutable tag through the existing four-platform build-only workflow, preserving nine artifacts, checksums and signed update metadata. Cameron explicitly requested the push and next tag.
**Verify:** `gh release view v0.1.22 --json tagName,isPrerelease,assets,url`
**Batch:** P16.38 only.
**Commit:** `P16.38: prepare v0.1.22 release`

**Preparation:** Inspected the complete release workflow and successful v0.1.21 run. No workflow changes or tests run. Fix formatting, Clippy and regression compilation passed before the version bump; release identity and locked metadata checked during preparation. Ubuntu live acceptance remains Cameron's verification.

### P12.248 — Read headless Linux service status without private metadata

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-platform-linux/src/service.rs`, `crates/msc-platform-linux/tests/systemd_unit.rs`, this plan.
**What:** Recognize the shipped Linux headless archive service template when the installed agent unit has no private metadata. Reconstruct its installing account, custom data directory, binary and loopback arguments for status and routine service control. Keep metadata-based desktop units supported. Require the complete shipped template to match before reporting its fixed settings; reject unknown definitions instead of guessing. Read the existing service without rewriting it, so updating the binary repairs existing archive installations. The shared report's required log path uses the conventional data-directory path; the archive service continues logging to journald and status creates no log file.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc && cargo check -p msc-platform-linux --test systemd_unit`
**Batch:** P12.248 only.
**Commit:** `P12.248: read headless linux service status`

**Evidence:** Cameron's Ubuntu `msc status` failed with missing metadata ServiceName. The headless installer renders a standard systemd template without the private comments required by the previous status parser; the CLI queried that parser before reading the live service state.
**Checks:** Rust formatting, shipping-agent Clippy and focused regression compilation passed. Existing warnings remain in infrastructure uninstall and agent runtime/auth code. No tests ran; no installed binaries or live services changed.
**Essential coverage:** One controlled regression renders the real package template with a custom data path containing spaces and a distinct primary group, checks running status, configured data-directory recovery, start/stop and preservation of the service file, and refuses an altered bind address rather than reporting a false default. Uses a temporary directory with cleanup and fake systemctl, with no network, real service changes or timing assumptions; expected runtime under 10 ms. Compiled only.
**Manual acceptance:** Update the Ubuntu headless agent binary with a build containing this commit. Run `msc status` and compare with `systemctl status com.ctemple.msc2.agent.service --no-pager`. Confirm the CLI reports the actual state instead of a metadata error. Routine start/stop may be checked when managed Minecraft servers can safely be stopped. Existing unit files, data and journald logging should remain intact.

### P16.37 — Publish v0.1.21 through the build-only release workflow

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.21.md`, this plan.
**What:** Increment v0.1.20 to v0.1.21 and synchronize locked release identity. Publish current main through the existing four-platform build-only workflow, preserving all nine desktop/headless artifacts, checksums, and signed update metadata. Monitor completion and inspect all failures before targeted fixes or retries; keep the tag immutable. Cameron explicitly authorized publication and release-error fixes. Preserve the uncommitted agent heading edit locally.
**Verify:** `gh release view v0.1.21 --json tagName,isPrerelease,assets,url`
**Batch:** P16.37 only.
**Commit:** `P16.37: prepare v0.1.21 release`

**Preparation:** Inspected the complete active release workflow, previous successful release, changes since v0.1.20, and configured signing-secret/public-key names. No workflow gates or tests added; no tests run. Physical acceptance remains Cameron's responsibility.

### P12.247 — Hide initiation and main console scrollbars

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, `clients/desktop-web/src/lib/components/shell/ConsoleDock.svelte`, this plan.
**What:** Hide the initiation and main dock console scrollbars using the existing app pattern for standard and WebKit scrollbar styling, keeping overflow scrolling available. Owner-requested narrow visual change; no tests added or run.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.247 only.
**Commit:** `P12.247: hide initiation and main console scrollbars`

**Checks:** Svelte check passed with zero errors and eleven existing warnings outside the changed components. No Rust files changed; no tests or release workflows run.

**Manual acceptance:** Open Initiate Server and the main console, wait for enough console output to overflow, and confirm both scrollbars are hidden while the mouse wheel or trackpad still scrolls the output.


### P19.5 — Authorize Fedora complete uninstall once

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-platform-linux/src/uninstall.rs`, `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/uninstall.rs`, `docs/msc2/clients/local-uninstall.md`, this plan.
**What:** Keep one protected installed MSC helper running after a single Fedora authorization prompt. Send it only fixed service-stop, system-data, verified package, and marked-archive actions while the existing copied worker checks the reviewed inventory and removes user-owned data. Preserve package ownership checks, ordered service shutdown before data deletion, the rule that a partial removal retains the installed command, and the current macOS/Windows uninstall paths.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P19.5 only.
**Commit:** `P19.5: authorize linux uninstall once`

**Evidence:** `LinuxUninstall` previously launched `pkexec` separately for every `systemctl`, `rm`, and `rpm` command. Fedora therefore prompted Cameron repeatedly during one confirmed removal. The installed agent is a root-owned executable that can accept a narrow privileged cleanup command after one OS authorization. The copied unprivileged worker still owns inventory comparison and user data cleanup; the privileged helper accepts no caller-supplied filesystem paths.
**Checks:** Rust formatting, shipping-agent Clippy, and Fedora RPM build pass with existing unrelated warnings. No tests or destructive uninstall ran; the installed RPM and live services were not changed.
**Manual acceptance:** Install an RPM containing this commit, open Settings → Uninstall MSC 2, review the inventory and confirm. Fedora should request authorization once for the complete local removal, then show a completed result. Verify `rpm -q msc-2` reports it absent and the local MSC services are gone. Test a partial-failure report by observation only if it occurs; do not interrupt or deliberately damage a live removal. Other platforms retain their previous uninstall code.

### P12.246 — Install live Bedrock map players for every active world

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/assets/bedrock-map-feed/`, `crates/msc-application/src/bedrock_map_feed.rs`, `crates/msc-application/src/lib.rs`, `crates/msc-application/tests/bedrock_map_feed.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, this plan.
**What:** Bundle the existing Bedrock player-position behavior pack with the agent and register it for the active world before each stopped-server start, including newly created and later selected worlds. Preserve other behavior packs and update an installed proof pack in place. Remove Bedrock feed lines, Java feed lines, and Java position-query replies and command echoes before they enter MSC's console, regardless of Auto-hide. Keep Java's existing fallback query path and all platform runtime/service implementations unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.246 only.
**Commit:** `P12.246: install bedrock map feed automatically`

**Evidence:** The live Fedora Bedrock server reported its online player while `/v1/worlds/map/players` remained `awaiting-feed`; its new world had no feed pack or world pack registration. The prior pack existed only under the manual map-proof tool. Java already polls position fields through built-in console commands when no server feed exists, so it needs console filtering but no pack installer. The agent currently pushed both feed messages and Java query replies into the console buffer, where disabling Auto-hide exposed frequent coordinate lines.
**Checks:** Rust formatting, focused regression compilation, shipping-agent Clippy, and desktop agent build/staging pass. No tests were run, and the running agent and worlds were not changed.
**Essential coverage:** An isolated filesystem regression installs the bundled feed in an active world, checks preservation of another pack and idempotence, then switches the active world and checks registration there. A small console-filter regression checks that Bedrock roster lines and Java coordinate replies/command echoes are hidden while ordinary server events remain visible. These catch the observed new-world `awaiting-feed` failure and the reported coordinate spam without Minecraft, network, timing, global state, or credentials. Expected runtime under 10 ms total; compiled only, not executed.
**Manual acceptance:** Rebuild the desktop, Repair the agent, and create or select a new Bedrock world. Start it, join, open View Map, and confirm live player coordinates appear and update. Turn Auto-hide off and confirm map feed positions never appear in MSC's console. Start and join a Java server, confirm live players appear using the existing query path, and confirm its position and rotation replies do not appear in the console. Repeat on Windows and macOS when available; the shared start and console logic applies there, while platform runtime/service code is untouched.

### P12.245 — Keep Fedora Bedrock shutdown from cancelling Broadcast

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/bedrock_linux.rs`, `crates/msc-application/tests/bedrock_linux.rs`, this plan.
**What:** Apply the P12.239 native Windows shutdown correction to the Linux Bedrock runtime. Consume queued process exits before escalating a graceful stop, clear stop timing when the process exits, and leave later stopped-state polls inert. Keep true 20-second forced shutdown for a live process. Windows and macOS runtime source is unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.245 only.
**Commit:** `P12.245: preserve broadcast through linux bedrock shutdown`

**Evidence:** The live Fedora journal shows successful first-run and Playit account operations followed by two Xbox Broadcast operations cancelled without error; the desktop therefore displayed its generic setup failure. The native Linux runtime still ran its 20-second graceful-stop check before draining a queued Bedrock exit and retained the stop deadline after setting its state to Stopped. The lifecycle error handler stops helpers and aborts first-start on that invalid force-stop. P12.239 corrected the same sequence on Windows only and explicitly identified the Linux path as still affected. This explains the observed cancellation pattern; a live Broadcast sign-in has not yet been repeated after the correction.
**Checks:** Rust formatting, shipping-agent Clippy, focused Linux regression compilation, and `npm --prefix clients/desktop-web run prepare:agent` passed with existing unrelated warnings. No tests ran and the live service was not restarted.
**Essential coverage:** One fake-process/fake-clock regression now covers a clean Linux exit observed before or after the stop deadline, then repeated stopped-state polls beyond it. It catches the observed helper-cancelling runtime error without Minecraft, provider calls, sleeps, or real credentials. It uses the existing ephemeral UDP-port fixture and should run in under 10 ms; compiled only, not executed.
**Manual acceptance:** Restart `npx tauri dev`, Repair the Fedora agent with this staged build, then retry Xbox Broadcast from the Bedrock initiation sheet. Confirm the Microsoft sign-in code appears, authentication completes, and pass two finishes with the server stopped and Playit configured. Waiting beyond the first pass's 20-second shutdown deadline must not cancel Broadcast. Recheck the Windows and macOS initiation flows; their runtime sources were not changed.

### P12.244 — Recognize Linux Playit connections during Bedrock setup

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/playit.rs`, `crates/msc-application/tests/playit.rs`, `crates/msc-agent/src/routes/networking.rs`, this plan.
**What:** Remove ANSI control sequences from Playit's connection line before matching its agent ID. If setup starts a Playit helper but the connection wait fails or is cancelled, reset that helper before returning the error. Leave existing helpers, saved credentials, cloud agents, and tunnels intact. Plain Windows and macOS output remains accepted.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.244 only.
**Commit:** `P12.244: recognize linux playit connections`

**Evidence:** The installed Playit v1.0.10 Linux binary emits ANSI sequences between log field names and `=`. MSC searched for literal `agent_id=`, so it could not recognize a successful connection and reported a 75-second timeout. The setup route marked a helper as newly started only after that wait succeeded, leaving its own helper running after this failure. Upstream Playit enables ANSI output on Linux but not Windows/macOS.
**Checks:** Rust formatting, shipping-agent Clippy, and compilation of the focused application regression target passed with pre-existing unrelated warnings. `npm --prefix clients/desktop-web run prepare:agent` built and staged the corrected agent and its existing helper bundle. No tests ran, and the live service was not restarted.
**Essential coverage:** The existing connection-recognition regression now supplies Playit's actual Linux ANSI field formatting. It catches the observed 75-second setup failure; a separate existing case retains plain output coverage. The fake process supplies controlled bytes with no network, files, timing, or live credentials. Expected runtime under one millisecond; compiled, not executed.
**Manual acceptance:** Repair the desktop agent from this staged build, then retry the saved Playit setup for Bedrock. Confirm it reaches tunnel provisioning and shows an address without waiting 75 seconds or creating another cloud agent. Also confirm a cancelled or failed setup does not leave a newly started Playit helper running. Check Playit setup on Windows and macOS with their plain logs.

### P12.243 — Align Fedora development Repair with staged agent bundles

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-platform-linux/src/service.rs`, this plan.
**What:** Make the elevated Linux helper verify the same agent, Vantage renderer, and Bedrock map exporter digest that the desktop uses for a staged development build. Copy all three verified executables into the root-owned system build directory so the repaired agent can find its map tools beside itself. Packaged agent installs retain their existing system-package path; Windows and macOS service paths are unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.243 only.
**Commit:** `P12.243: align linux dev repair with staged agent bundles`

**Evidence:** The live Fedora `tauri dev` Repair command returned `staged MSC executable does not match its content-addressed build name` from the installed elevated helper. The staged directory name matches SHA-256 over `msc`, `vantage`, and `bedrock-map` in that order, while the old helper compared it with SHA-256 of `msc` alone. It rejected the valid bundle before changing the running service. The agent resolves both map tools beside its own executable, so the helper must copy the complete bundle when installing the system-owned dev build.
**Checks:** Rust formatting and shipping-agent Clippy passed with existing unrelated warnings. The live staged bundle's three-file digest matched its directory name. No tests ran. A corrected RPM is needed to replace the installed elevated helper before development Repair can use this fix.
**Manual acceptance:** Install a Fedora RPM containing this commit, restart `npx tauri dev`, then click Repair agent and approve the OS prompt. Confirm the dev desktop connects and the service runs from `/usr/lib/MSC 2/agent/dev-builds/<digest>/msc`; confirm its sibling `vantage` and `bedrock-map` executables are present. Confirm a packaged desktop still connects using `/usr/lib/MSC 2/agent/msc`.

### P12.242 — Decode desktop API frames on Fedora

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `clients/desktop-web/src/lib/auth/desktop.ts`, this plan.
**What:** Normalize Tauri's binary API response to bytes before decoding its length, headers, and body. Fedora WebKit returns the response as a JavaScript number array; other desktop runtimes may return an ArrayBuffer. Keep the Rust agent, native authorization, and Windows/macOS response content unchanged.
**Verify:** `npm --prefix clients/desktop-web run check`
**Batch:** P12.242 only.
**Commit:** `P12.242: decode desktop api frames on fedora`

**Evidence:** On the installed v0.1.20 Fedora desktop, the native `desktop_bootstrap_local` call succeeds. The same desktop's `desktop_authorized_request_binary` returns an Array of 704 numbers for `/v1/me`, while the frontend passed it directly to `DataView`, which requires an ArrayBuffer. That exception prevents the app from reading successful API responses and leaves the setup screen at its generic unavailable state. The running agent, credential helper, saved desktop credential, and all four startup API endpoints were healthy. The fix converts either IPC shape to a byte view before reading the existing frame format.
**Checks:** Svelte check passed with zero errors and eleven existing warnings; production frontend build passed. No test suite ran. The installed RPM still contains the old frontend until a corrected package is built and installed.
**Manual acceptance:** Install a desktop package containing this commit on Fedora, open MSC, and confirm the local agent connects without another repair. Confirm Repair reconnects. Check the same packaged desktop connection on Windows and macOS; their native authorization and service installation paths were not changed.

### P16.36 — Publish v0.1.20 through the build-only release workflow

**Status:** Prepared; publication and Cameron's artifact acceptance pending.
**Files:** Agent/client/Tauri version manifests and lockfiles, source/static bundle identity and its existing assertion, README, `docs/msc2/release/v0.1.20.md`, this plan.
**What:** Increment v0.1.19 to v0.1.20, synchronize locked release identity, and include the required Windows Tokio dependency correction in the Tauri lockfile. Publish current main through the existing release workflow only; preserve nine artifacts, checksums, and signed update metadata. Monitor until completion, inspect failures before any targeted retry, and keep the tag immutable. Cameron explicitly authorized publication and fixes for release failures. No CI/test workflows or test execution.
**Verify:** `gh release view v0.1.20 --json tagName,isPrerelease,assets,url`
**Batch:** P16.36 only.
**Commit:** `P16.36: prepare v0.1.20 release`

**Preparation:** Inspected the full release workflow and prior successful v0.1.19 run. Only release.yml is active. Signing-secret and public-key variable names are present. Locked Cargo metadata resolves for the agent and desktop manifests; version fields are synchronized. Publishing will run the existing four-platform build matrix; physical acceptance remains separate.

### P18.10y — Retry maps after an initially empty Bedrock world

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`, this plan.
**What:** Reject and remove empty dimension catalogs instead of retaining them indefinitely. When no rendered dimension remains, release the old saved-world snapshot so the next map request captures fresh data. Keep a shared snapshot while a populated dimension still uses it. Preserve readiness gating, save-resume cleanup, exporters, Java maps, and platform runtimes. This shared Bedrock cache correction applies on Windows, macOS, and Linux.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P18.10y only.
**Commit:** `P18.10y: release empty bedrock map snapshots for retry`

**Evidence:** Bedrock 2's first map snapshot at 17:43:27 preceded Cameron's player connection at 17:45:09. The snapshot database had two tiny .ldb files and a 64-byte log; its exported Overworld manifest had zero tiles and no spawn. The live database subsequently had a roughly 2.9 MB log. The cache retained both the empty catalog and original snapshot, so reopening could never see the later generated world. Console output confirmed the previous readiness fix resumed saves promptly after its successful repeated query.
**Checks:** Rust formatting, shipping-agent Clippy, and agent build passed with existing unrelated Windows warnings. `cargo rustc -p msc-agent --bin msc --profile test -- --emit=metadata` compiled the binary unit-test target without executing tests or including the unrelated broken CLI integration-test target. Built agent staged in desktop development/package resource directories with matching SHA256 hashes; the running service was not restarted or repaired. Live rendering and macOS acceptance remain Cameron's verification.
**Essential coverage:** One controlled regression rejects an empty Overworld catalog and checks that its temporary catalog/snapshot files are released, then checks an empty Nether catalog leaves a populated Overworld and its shared snapshot available. This protects the observed persistent empty-map failure and prevents invalidating an already usable dimension. Unique automatically cleaned temporary directories; no Minecraft, exporter, network, sleeps, or timing assumptions. Expected runtime under 10 ms; compiled only, not executed.
**Manual acceptance:** Stop Minecraft, Repair the agent using the staged development binary, restart Bedrock 2, join it, and open View Map. Confirm Overworld terrain renders. On a fresh server, open the map before joining, then join and reopen; confirm an initially empty result no longer prevents later terrain from loading. Confirm an unvisited Nether still reports empty without breaking the populated Overworld. Repeat the map retry on macOS Bedrock. No live worlds or existing user snapshots were changed by the investigation.

### P12.241 — Preserve Java's initial slot through fresh-server registration

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-application/src/provisioning.rs`, `crates/msc-application/src/worlds.rs`, `crates/msc-application/tests/provisioning.rs`, this plan.
**What:** Once fresh Java creation has saved its profile and active slot ID, record the existing world-reconciliation marker for that owned initial slot. The shared creation finalizer covers download-and-go and installer-based Java server families. A custom generation data pack can create a world folder before level.dat exists; import recovery must not turn that preparation into a second slot. Keep this slot archive-less so Minecraft still generates the world on first start. Imported Java servers retain existing reconciliation/archive comparison; Bedrock creation and platform runtimes are unchanged.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P12.241 only.
**Commit:** `P12.241: preserve fresh java slots during registration`

**Evidence:** Java generation_properties creates a world folder for custom biome/generator settings. Registration counts an existing folder as live data; an archive-less initial slot then triggers a second slot. Java's preparation lacks level.dat, so creating a world archive here would violate archive/activation expectations. Recording the owned fresh slot as reconciled prevents import recovery from adopting the generation pack as another world. Normal fresh creation previously reached the same marker through the archive-less/no-live-folder reconciliation branch.
**Checks:** Rust formatting and shipping-agent Clippy passed with existing unrelated Windows warnings. `cargo check -p msc-application --test provisioning` compiled the focused regression with existing warnings. The updated agent is rebuilt/staged for desktop Repair. No tests executed or live world/slot data changed.
**Essential coverage:** One regression covers fresh Java creation and registration with and without custom generation. Assert one original active slot, identical saved profile, preserved generation-preset contents, no pre-generated level.dat, and no world archive. Existing fake download transport and unique automatically cleaned temporary directories avoid real Java/Minecraft, live networks, ports, sleeps, and clock assumptions. Expected runtime under 100 ms; execution remains with Cameron.
**Manual acceptance:** Repair with the staged agent. Create a Java server with a custom Biome Source or Generator Options, confirm one slot before/after initiation, and confirm the chosen generation settings apply. Also confirm normal default-world generation. Existing duplicate entries are retained; this prevents duplicates in new fresh Java servers across platforms.


### P18.10x — Retry Bedrock map readiness while save preparation finishes

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/backup_operations.rs`, this plan.
**What:** Bound each live Bedrock `save query` confirmation wait to 500 ms within the existing ten-second overall budget. This lets the existing readiness loop query again after BDS initially reports that save preparation is incomplete. Preserve immediate readiness, same-run console boundaries, required readiness before map copying, and save-resume cleanup. Keep Java flush confirmation's full wait and leave platform runtimes, terrain exporters, and frontend rendering unchanged. The shared correction also applies to macOS/Linux Bedrock and live Bedrock backup readiness.
**Verify:** `cargo clippy -p msc-agent --bin msc`
**Batch:** P18.10x only.
**Commit:** `P18.10x: retry live bedrock map save readiness`

**Evidence:** The running Windows BDS console recorded Saving followed immediately by A previous save has not been completed at 17:23:48 and 17:26:44, followed by Changes to the world are resumed approximately ten seconds later. The production waiter previously spent the entire overall budget on the first query, preventing the existing retry loop from sending a second. The bundled BDS how-to explicitly requires repeated queries until preparation finishes. Java instead receives a completion response to `save-all flush`; it does not use Bedrock's query protocol. MSC 1's checkout is unavailable on this Windows machine; no source there was modified.
**Checks:** Rust formatting, shipping-agent Clippy, and agent build passed with existing unrelated Windows warnings. `cargo check -p msc-agent --bin msc --tests` compiled the binary's unit-test target, including the new regression, but the overall command failed on existing `cli_service` integration-test references to removed CommonArgs fields base_url/host/port/token. Those unrelated tests remain unchanged. No tests were executed. Built agent copied to the desktop development/package resource directories with matching SHA256 hashes; the running service was not restarted or repaired.
**Essential coverage:** One regression exercises the production wait loop with a simulated clock: first query not ready, a later query ready inside the overall budget, immediate readiness, a Java flush completing after the Bedrock query interval, and readiness never arriving before the overall timeout. Existing application fakes returned immediately and missed the production waiter exhausting the deadline. No real sleeps, server processes, files, ports, or network assumptions; expected runtime under one millisecond. Compiled only; execution remains Cameron's decision.
**Manual acceptance:** Stop the running Minecraft server, apply desktop Settings → Repair to load the staged agent, then start Bedrock and open View Map while it is running. Confirm terrain appears and world saving resumes; refresh/reopen and check another saved dimension. Repeat the live map on macOS Bedrock and on a Java server. Actual Windows terrain rendering and macOS compatibility remain owner verification; build/compilation alone do not establish those results.

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
