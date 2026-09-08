# MSC 2 — Rolling Plan

> ## STATUS: Phase 12 (client redesign) is complete and archived. Phase 12 post-phase corrections continue; the planned Phase 13 full-screen terminal client is retired by D-034.
> **Next move:** Cameron verifies P12.114 — record and verify the terminal UI retirement audit. P12.109 through P12.113 remain awaiting verification; Phase 11 and Phase 12 remain complete, with their historical records in `rolling-plan-archive.md`.

**Previous phases (Setup through Phase 12) and their amendments have moved to `rolling-plan-archive.md`** to keep this file small. That archive is historical only — current status and active work stay here.

---

## How this document works

This is the **working state** of the build. The vision documents say where we're going; the port plan says in what order; this file says **where we actually are right now**.

Phases are fixed and come from `msc2-port-plan.md`. **Steps are written one phase at a time**, as we reach each phase — not up front. The retired Phase 13 remains historical while this owner-requested Phase 12 post-phase correction is completed.

Each phase runs the six-move loop in `CLAUDE.md`: Plan → Read → Execute → Verify → Review → Advance.

### Step format

Every step looks like this:

```
### P0.3 — Extract TPS parser fixtures
**Status:** not started | in progress | awaiting verification | DONE
**Files:** fixtures/tps/, tools/extract-fixtures/
**What:** Pull the 27 TPS test cases out of MSC 1's TpsMonitoringTests.swift
         into input/expected JSON pairs.
**Verify:** `ls fixtures/tps/*.json | wc -l` → 27
**Commit:** P0.3: extract TPS parser fixtures        <- the message, not a hash
```

Every step also carries a **Batch:** field, telling an agent whether it may be run unattended:

| Batch value | Meaning |
|---|---|
| `safe` | Mechanical, and its Verify is a script Cameron has already reviewed. Batch freely. |
| `stop-after` | Runnable in a batch, but the batch **ends here** — the result needs looking at before continuing. |
| `solo` | Judgment work or a new checker script. Run it alone. Its output needs a cross-check by the other agent before the phase closes. |

**Status is only moved to DONE by Cameron**, after he runs the Verify command himself. An agent may set it to *awaiting verification* and stop.

**A step whose Verify only counts things is `stop-after` at best.** Counting proves something exists, not that it is right.

### P12.74 — Remember remote hosts across desktop restarts
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/hosts/saved.ts`, `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Persist remote host names and addresses in the desktop client so paired hosts return after MSC is reopened. Keep bearer credentials in the existing native secure store, never in browser storage. Add a Saved hosts section on the agent setup page with connection state, host switching, and removal that also forgets the native credential. Keep SSH tunnel instructions explicit: a remembered host can still require its tunnel to be opened again.
**Verify:** `cd clients/desktop-web && npx prettier --check src/App.svelte src/lib/hosts/saved.ts src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.74: remember remote hosts across desktop restarts`
**Batch:** solo

### P12.75 — Make Saved hosts collapsible
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Make the Saved hosts section collapsible using the same remembered disclosure behavior as the other agent setup sections. Keep the saved-host count visible while collapsed so experienced users can reduce the page without losing context.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.75: make saved hosts collapsible`
**Batch:** solo

### P12.76 — Separate remote disconnect from host removal
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Replace the current remote agent’s destructive Remove paired host action with a non-destructive Disconnect action. Disconnect returns the client to the local agent while preserving the saved host and secure credential; permanent removal remains available from Saved hosts.
**Verify:** `cd clients/desktop-web && npx prettier --check src/App.svelte src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.76: separate remote disconnect from host removal`
**Batch:** solo

### P12.77 — Separate connection and service status cards
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Render Connection and Background service as two equal cards instead of two columns inside one shared card, preserving their content and stacking them on narrow windows.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.77: separate connection and service status cards`
**Batch:** solo

### P12.78 — Remove duplicate status dots
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Remove the redundant signal dots from the Connection and Background service cards because each card already presents the same state in its badge and status text.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.78: remove duplicate status dots`
**Batch:** safe

### P12.79 — Remove the Saved hosts status dot
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Remove the Saved hosts row’s status dot while retaining its short plain-text state label, host identity, address, server count, and management actions.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.79: remove the saved hosts status dot`
**Batch:** safe

### P12.80 — Keep connection badges in the status cards
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Remove repeated Agent connected badges from the page heading and agent management disclosures. Keep the Connection and Background service cards as the single status summary surface at the bottom of the page.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.80: keep connection badges in the status cards`
**Batch:** safe

### P12.81 — Open Agent setup after switching hosts
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `docs/msc2/rolling-plan.md`
**What:** After an accepted host switch, clear the previous host’s retained tabs, initialize the new host, and open Agent setup so the selected host’s connection state is immediately visible instead of leaving an empty content area.
**Verify:** `cd clients/desktop-web && npx prettier --check src/App.svelte && npm run build`
**Commit:** `P12.81: open agent setup after switching hosts`
**Batch:** safe

### P12.82 — Resolve public addresses for port-forwarded servers
**Status:** awaiting verification
**Files:** `crates/msc-infrastructure/src/public_ip.rs`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-application/src/network_diagnostics.rs`, `crates/msc-agent/src/routes/network_diagnostics.rs`, `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, `docs/msc2/rolling-plan.md`
**What:** When DuckDNS or Playit is not supplying a public join address, resolve the host’s public IP through a bounded third-party lookup and expose it as the existing `public_ip` connectivity source. Honor an existing per-server public host override, use each server’s real configured port, and show the resulting Java or Bedrock port-forwarding address in the sidebar, Overview connection card, and completed first-start summary. Keep public reachability separate from address discovery so MSC does not claim a router rule works without proving it.
**Verify:** `cargo fmt --all -- --check && cargo check -p msc-infrastructure -p msc-application -p msc-agent && cd clients/desktop-web && npx prettier --check src/lib/sections/server-editor/FirstStartSheet.svelte && npm run build`
**Commit:** `P12.82: resolve public addresses for port-forwarded servers`
**Batch:** solo

### P12.83 — Recover create flow from an unusable Java runtime
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, `clients/desktop-web/src/lib/sections/fleet/wizard/model.ts`, `clients/desktop-web/src/lib/sections/server-editor/JavaInstallSheet.svelte`, `clients/desktop-web/src/lib/sections/server-editor/JavaTab.svelte`, `crates/msc-application/src/provisioning.rs`, `crates/msc-application/src/templates.rs`, `crates/msc-application/tests/provisioning.rs`, `crates/msc-application/tests/provisioning_install_step.rs`, `crates/msc-application/tests/templates.rs`, `crates/msc-agent/src/routes/servers.rs`, `crates/msc-infrastructure/src/jar_provider.rs`, `docs/msc2/rolling-plan.md`
**What:** Preserve the Add Server wizard and its draft when creation fails with `unusable_java_runtime`. Show the exact required Java major and failure explanation with Cancel or Download; open the shared Adoptium installer with the required major preselected; after a successful install, save the managed runtime path as the host Java executable, show a clear success acknowledgement, and return to the unchanged create confirmation step. Preserve the existing Java settings install entry point by extracting it into the shared sheet. Honor the wizard's selected Java version during provisioning so the runtime guard checks and creates the requested Minecraft version instead of silently downloading latest; keep modpack-pinned provisioning and Bedrock creation unchanged.
**Verify:** `cargo fmt --all -- --check && cargo check -p msc-infrastructure -p msc-application -p msc-agent && cargo clippy -p msc-infrastructure -p msc-application --lib -- -D warnings && cd clients/desktop-web && npx prettier --check src/lib/sections/fleet/wizard/AddServerWizard.svelte src/lib/sections/fleet/wizard/model.ts src/lib/sections/server-editor/JavaInstallSheet.svelte src/lib/sections/server-editor/JavaTab.svelte src/lib/sections/server-editor/FirstStartSheet.svelte && npm run check && npm run build`
**Commit:** `P12.83: recover create flow from unusable java runtime`
**Batch:** solo

### P12.84 — Select an installed compatible Java runtime during create
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/routes/servers.rs`, `docs/msc2/rolling-plan.md`
**What:** Before creating a regular Java server, validate the configured executable against the selected Minecraft version. If it is too old, search the same installed-runtime roots used by the Java runtimes screen, probe each executable, silently select and persist the first compatible runtime, and continue creation. Leave the recovery popup as the fallback when no installed runtime can satisfy the requirement; keep modpack and Bedrock creation on their existing paths.
**Verify:** `cargo fmt --all -- --check && cargo check -p msc-agent`
**Commit:** `P12.84: select installed compatible java during create`
**Batch:** solo

### P12.85 — Remove connection status signal dots from setup surfaces
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/components/base/StatusDot.svelte`, `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, `clients/desktop-web/src/lib/sections/server-editor/GeneralTab.svelte`, `clients/desktop-web/src/lib/sections/server-editor/BroadcastTab.svelte`, `docs/msc2/rolling-plan.md`
**What:** Keep the status labels and controls in the first-start connection check, EULA, and Services tab, but remove the small signal dots from those surfaces. Preserve the shared status component's dotted presentation for other screens that still use it.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/components/base/StatusDot.svelte src/lib/sections/server-editor/FirstStartSheet.svelte src/lib/sections/server-editor/GeneralTab.svelte src/lib/sections/server-editor/BroadcastTab.svelte && npm run check && npm run build`
**Commit:** `P12.85: remove setup status signal dots`
**Batch:** solo

### P12.86 — Persist EULA status and scope DuckDNS to port forwarding
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/main.rs`, `crates/msc-agent/src/routes/network_diagnostics.rs`, `crates/msc-agent/src/routes/servers.rs`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`, `clients/desktop-web/src/lib/sections/server-editor/GeneralTab.svelte`, `clients/desktop-web/src/lib/sections/server-editor/model.ts`, `docs/msc2/rolling-plan.md`
**What:** Read the persisted Java `eula.txt` state when opening Edit Server so an EULA accepted during first-start is shown as accepted. Move DuckDNS feedback into its own section, add an explicit remove action, hide the setting for the selected server when Playit is enabled, and keep connectivity diagnostics from using DuckDNS for Playit servers.
**Verify:** `cargo fmt --all -- --check && cargo check -p msc-agent && cd clients/desktop-web && npm run api:check && npx prettier --check src/App.svelte src/lib/sections/app-settings/AppSettingsSheet.svelte src/lib/sections/server-editor/GeneralTab.svelte src/lib/sections/server-editor/model.ts && npm run check && npm run build`
**Commit:** `P12.86: persist eula status and scope duckdns to port forwarding`
**Batch:** solo

### P12.87 — Align the Xbox Broadcast password field
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`, `docs/msc2/rolling-plan.md`
**What:** Keep the Microsoft email, Xbox gamertag, and password inputs the same width and right alignment. Place the password visibility control inside the password field so it does not change that field's outer dimensions.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/app-settings/AppSettingsSheet.svelte && npm run check && npm run build`
**Commit:** `P12.87: align xbox broadcast password field`
**Batch:** solo

### P12.88 — Add Linux agent service extra notes
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Add a clickable Extra notes section beneath the remote Connect agent button. Explain that the commands apply to the Linux computer hosting the agent, provide a generic service-name discovery command instead of exposing a package-owner identifier, and provide copyable systemd commands for starting, stopping, restarting, checking status, enabling at boot, and watching live logs, including the status and Ctrl+C guidance.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run check && npm run build`
**Commit:** `P12.88: add linux agent service extra notes`
**Batch:** solo

### P12.89 — Bump the coordinated prerelease version
**Status:** awaiting verification
**Files:** `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `docs/msc2/rolling-plan.md`
**What:** Increment the synchronized desktop, embedded agent, and headless package version from 0.1.0 to 0.1.1 so the next prerelease tag and every generated artifact carry the same release identity.
**Verify:** `python3 tools/release/check-release-workflow.py .github/workflows/release.yml && git diff --check`
**Commit:** `P12.89: bump coordinated prerelease version`
**Batch:** solo

### P12.90 — Fix absolute Linux systemd working directory rendering
**Status:** awaiting verification
**Files:** `packaging/linux/systemd/com.ctemple.msc2.agent.service.in`, `docs/msc2/rolling-plan.md`
**What:** Remove the unit-value quotes around `WorkingDirectory` so the Linux headless installer renders an absolute path that systemd accepts. The release builder was checked and copies this source template; the service identity, user ownership, data directory, bind address, credential-helper behavior, and quoted `Environment=` setting remain unchanged.
**Verify:** `grep -qx 'WorkingDirectory=@MSC2_DATA_DIR@' packaging/linux/systemd/com.ctemple.msc2.agent.service.in && bash -n packaging/linux/install.sh && git diff --check`
**Commit:** `P12.90: fix linux systemd working directory`
**Batch:** solo

### P12.91 — Add release installation quick start to the README
**Status:** awaiting verification
**Files:** `README.md`, `docs/msc2/rolling-plan.md`
**What:** Replace the outdated prerelease notice with direct v0.1.1 desktop and headless download links, curl installation commands, checksum verification, platform requirements, and a short first-server walkthrough.
**Verify:** `git diff --check && rg -n 'Download and install|Linux headless agent|Start your first server|v0\.1\.1' README.md`
**Commit:** `P12.91: add release installation quick start to readme`
**Batch:** solo

### P12.92 — Fix Fedora desktop agent resource lookup
**Status:** awaiting verification
**Files:** `clients/desktop-web/src-tauri/src/lib.rs`, `packaging/agent-service-layout.json`, `tools/phase12/bedrock-package-check.py`, coordinated version manifests and locks, `README.md`, `docs/msc2/rolling-plan.md`
**What:** Align the Linux desktop shell with Tauri v2's actual Debian/RPM resource directory, `/usr/lib/MSC 2/agent/msc`, which is derived from the product name rather than the executable name. Keep the development/AppImage resource path as a fallback, update the package contract and checker, and bump the coordinated prerelease from 0.1.1 to 0.1.2 so Fedora receives the fix in the next release.
**Verify:** `rg -n '\.\./lib/MSC 2/agent/msc|v0\.1\.2' clients/desktop-web/src-tauri/src/lib.rs packaging/agent-service-layout.json README.md && cargo fmt --all -- --check && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml --lib -- -D warnings && git diff --check`
**Commit:** `P12.92: fix fedora desktop agent resource lookup`
**Batch:** solo

---

## Phases

Gates are in `msc2-port-plan.md`. This is the map, not the detail.

| Phase | Name | State |
|---|---|---|
| **Setup** | Repo, docs, agent instructions, CI, editor config | complete |
| **0** | Freeze the baseline and build the harness | complete |
| 1 | Domain types and pure rules | complete |
| 2 | API contract and operation model | complete |
| 3 | Safety substrate | complete |
| 4 | Java lifecycle vertical slice | complete |
| 5 | Configuration and migration | complete |
| **6** | Worlds and backups | complete |
| **7** | Server families and provisioning | complete |
| **8** | Mods, plugins, modpacks | complete |
| **9** | Networking and helpers | complete |
| **10** | Bedrock runtimes | complete |
| **11** | Desktop and web clients | complete |
| **12** | Client redesign (MSC 1 fidelity, refreshed) | complete |
| 13 | Terminal UI | retired by D-034 |

## Phase 12 amendment — Bedrock checksum metadata

### P12.35 — Use published Bedrock checksums for all host archives
**Status:** awaiting verification
**Files:** `crates/msc-infrastructure/src/bedrock_distribution.rs`, `crates/msc-application/src/bedrock_provisioning.rs`, `crates/msc-application/tests/bedrock_provisioning.rs`, `docs/msc2/rolling-plan.md`
**What:** Replace the unusable Bedrock release source with Endstone's two-document registry and per-version metadata, which publishes SHA-256 values for the official Mojang Linux and Windows archives. Keep the existing manifest shape supported for mirrors and fixtures, retain strict verification before staging, and prove both native platform paths. Intel macOS continues to consume the verified Linux guest archive through the existing platform mapping.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-infrastructure -p msc-application --all-targets -- -D warnings && cargo nextest run -p msc-application --test bedrock_provisioning`
**Commit:** `P12.35: use published Bedrock checksums`
**Batch:** solo

### P12.36 — Keep Overview Server Health to actionable checks
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/home/HealthGrid.svelte`, `docs/msc2/rolling-plan.md`
**What:** Narrow the Overview Server Health grid to RAM Allocation, Last Startup, and Port Reachability. Remove Java Runtime, Add-on Jars, Bedrock World Data, and the placeholder VM Runtime from this compact Overview surface without changing the agent health payload or other client surfaces. Keep the health grid's existing card flip behavior and responsive layout.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/home/HealthGrid.svelte && npx vitest run tests/screens/overview.test.ts`
**Commit:** `P12.36: keep overview health checks actionable`
**Batch:** solo

### P12.37 — Implement the Bedrock VM Runtime health card
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/routes/health.rs`, `clients/desktop-web/src/lib/sections/home/HealthGrid.svelte`, `docs/msc2/rolling-plan.md`
**What:** Keep VM Runtime in the Bedrock Overview health grid and replace its placeholder with the agent's real Bedrock runtime state. Report native Linux/Windows support as green without implying a VM is needed, report the Intel macOS Virtualization Framework sidecar as green, show provisioning-required as yellow, and show unsupported hosts as a neutral unavailable state. Keep Java Runtime, Add-on Jars, and Bedrock World Data out of this compact Overview surface while retaining the existing flip interaction and cross-platform wording.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc -- -D warnings -A unused-mut && cargo nextest run -p msc-agent --bin msc -E 'test(bedrock_vm_runtime_card_reflects_backend_state)' && cd clients/desktop-web && npx prettier --check src/lib/sections/home/HealthGrid.svelte && npx vitest run tests/screens/overview.test.ts`
**Commit:** `P12.37: implement bedrock vm runtime health card`
**Batch:** solo

### P12.38 — Refresh the embedded agent web bundle
**Status:** awaiting verification
**Files:** `crates/msc-agent/web-ui/`, `docs/msc2/rolling-plan.md`
**What:** Rebuild and package the current desktop-web output into the agent's embedded web UI so installed agents on macOS, Windows, and Linux serve the complete-pair RAM save flow. The served editor accepts decimal gigabyte values such as 4.5 and sends both `minRamGB` and `maxRamGB`; the tracked bundle must no longer serve the retired partial-save editor.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/server-editor/GeneralTab.svelte src/lib/components/base/NumberField.svelte && npx vitest run tests/screens/server-editor.test.ts && cd ../.. && rg -l 'minRamGB:se,maxRamGB:ve' crates/msc-agent/web-ui/assets --glob '*.js'`
**Commit:** `P12.38: refresh embedded agent web bundle`
**Batch:** solo

### P12.39 — Make RAM field updates explicit across the component boundary
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/components/base/NumberField.svelte`, `clients/desktop-web/src/lib/sections/server-editor/GeneralTab.svelte`, `clients/desktop-web/tests/screens/server-editor.test.ts`, `clients/desktop-web/tests/auth/desktop/desktop.test.ts`, `crates/msc-agent/web-ui/`, `docs/msc2/rolling-plan.md`
**What:** Rename the number field's value callback from the DOM-shaped `onchange` prop to `onValueChange`, so the Bedrock RAM editor cannot lose typed or stepped values at the reusable component boundary. Keep complete-pair saves, decimal parsing, and 0.1 GB steps intact, then rebuild the embedded agent bundle that the desktop and headless agent serve on all supported host platforms.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/components/base/NumberField.svelte src/lib/sections/server-editor/GeneralTab.svelte tests/screens/server-editor.test.ts tests/auth/desktop/desktop.test.ts && npx vitest run tests/screens/server-editor.test.ts tests/components/base.test.ts tests/auth/desktop/desktop.test.ts && npm run build && node ./tools/package-agent-bundle.mjs && cd ../.. && rg -l 'onValueChange' crates/msc-agent/web-ui/assets --glob '*.js'`
**Commit:** `P12.39: make ram field updates explicit`
**Batch:** solo

### P12.40 — Correct RAM acronym wire names
**Status:** awaiting verification
**Files:** `crates/msc-api/src/dto/versions.rs`, `docs/msc2/rolling-plan.md`
**What:** Explicitly serialize and deserialize RAM fields as the frozen `minRamGB`/`maxRamGB` API names. Rust's generic `camelCase` rule lowercases the acronym suffix to `minRamGb`/`maxRamGb`; the desktop was sending the documented names, so the agent silently parsed both values as absent and returned `no_changes`. Accept the lowercase-suffix spelling while reading for compatibility, but always emit the documented spelling across macOS, Windows, Linux, browser, and desktop clients.
**Verify:** `cargo check -p msc-api`
**Commit:** `P12.40: correct ram acronym wire names`
**Batch:** solo

### P12.41 — Sign the macOS Bedrock sidecar for Virtualization.framework
**Status:** awaiting verification
**Files:** `sidecar/bedrock/BedrockSidecar.xcodeproj/project.pbxproj`, `clients/desktop-web/tools/prepare-agent-dev.mjs`, `docs/msc2/rolling-plan.md`
**What:** Apply `BedrockSidecar.entitlements` to the executable instead of packaging it as an unused resource. Enable ad-hoc signing for Debug and Release so the macOS sidecar carries `com.apple.security.virtualization`, then make development staging fail if the built executable is unsigned or missing that entitlement. This affects only the macOS Bedrock VM process; native Windows/Linux Bedrock and the Phase 13 terminal UI are unchanged.
**Verify:** `cd clients/desktop-web && npm run prepare:agent && codesign -d --entitlements :- src-tauri/target/Resources/agent/sidecar/BedrockSidecar 2>&1 | rg -A1 'com.apple.security.virtualization|<true/>'`
**Commit:** `P12.41: sign macos bedrock sidecar`
**Batch:** solo

### P12.42 — Keep the macOS sidecar main queue available for VM callbacks
**Status:** awaiting verification
**Files:** `sidecar/bedrock/BedrockSidecarCore.swift`, `docs/msc2/rolling-plan.md`
**What:** Read sidecar protocol input on a background queue and keep the main run loop active for Virtualization.framework VM callbacks and guest serial output. The previous blocking `readLine()` loop prevented the start completion, readiness event, and console lines from being delivered, leaving first-start stuck at “Bedrock process spawned.” EOF still force-stops the guest before the sidecar exits. The terminal UI files and native Windows/Linux Bedrock paths are unchanged.
**Verify:** `cd clients/desktop-web && npm run prepare:agent && codesign --verify --deep --strict --verbose=2 src-tauri/target/Resources/agent/sidecar/BedrockSidecar`
**Commit:** `P12.42: keep sidecar vm callbacks available`
**Batch:** solo

### P12.43 — Allow Bedrock provisioning after a stopped retry
**Status:** awaiting verification
**Files:** `crates/msc-application/src/bedrock_runtime.rs`, `crates/msc-application/tests/bedrock_runtime.rs`, `docs/msc2/rolling-plan.md`
**What:** Allow the macOS sidecar runtime to provision from `Stopped`, matching the native Windows/Linux runtimes. A failed or manually stopped first-start attempt can then be retried instead of being rejected before the sidecar receives the provisioning request. Keep the existing `New` path unchanged.
**Verify:** `cargo nextest run -p msc-application --test bedrock_runtime stopped_sidecar_can_be_reprovisioned_for_a_retry`
**Commit:** `P12.43: allow bedrock reprovisioning after stop`
**Batch:** solo

### P12.44 — Fix Bedrock retry and first-start console separation
**Status:** awaiting verification
**Files:** `sidecar/bedrock/BedrockSidecarCore.swift`, `sidecar/bedrock/Tests/BedrockSidecarTests.swift`, `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, `clients/desktop-web/tests/screens/first-start.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Let the long-lived macOS Bedrock sidecar accept a new provision request after its previous VM reaches `terminated`, matching the Rust runtime's stopped-retry behavior, and reset per-run guest state before binding the retry. Add a visible local Clear button to the first-start sheet's live console; keep the agent's bounded history intact, hide already-rendered lines after the next poll, allow newer lines through, and ignore stale in-flight poll results so a retry can be visually separated from the previous run.
**Verify:** `xcodebuild test -project sidecar/bedrock/BedrockSidecar.xcodeproj -scheme BedrockSidecar -destination 'platform=macOS' CODE_SIGNING_ALLOWED=NO && cd clients/desktop-web && npx prettier --check src/lib/sections/server-editor/FirstStartSheet.svelte tests/screens/first-start.test.ts && npx vitest run tests/screens/first-start.test.ts tests/screens/live.test.ts`
**Commit:** `P12.44: fix Bedrock retry and first-start console separation`
**Batch:** solo

### P12.45 — Make macOS Bedrock startup compatible and honest
**Status:** awaiting verification
**Files:** `sidecar/bedrock/Resources/appliance-initramfs.gz`, `sidecar/bedrock/Resources/README.md`, `sidecar/bedrock/BedrockSidecar.xcodeproj/project.pbxproj`, `sidecar/bedrock/BedrockSidecarCore.swift`, `sidecar/bedrock/Tests/BedrockSidecarTests.swift`, `clients/desktop-web/tools/prepare-agent-dev.mjs`, `tools/phase12/bedrock-package-check.py`, `docs/msc2/rolling-plan.md`
**What:** Add the glibc compatibility links required by the current official Linux Bedrock binary to the Intel VM appliance and update its recorded checksum everywhere the sidecar resources are validated. Make the sidecar emit its readiness event only after both the UDP relay and Bedrock's `Server started` console line are present, so a missing library or other early BDS exit becomes a failed first-start operation instead of an automatic stop that can remain visually stuck.
**Verify:** `xcodebuild test -project sidecar/bedrock/BedrockSidecar.xcodeproj -scheme BedrockSidecar -destination 'platform=macOS' CODE_SIGNING_ALLOWED=NO && cd clients/desktop-web && npx prettier --check src/lib/sections/server-editor/FirstStartSheet.svelte tests/screens/first-start.test.ts tools/prepare-agent-dev.mjs && npx vitest run tests/screens/first-start.test.ts tests/screens/live.test.ts`
**Commit:** `P12.45: make macos bedrock startup compatible and honest`
**Batch:** solo

### P12.46 — Restore Release appliance validation inputs
**Status:** awaiting verification
**Files:** `sidecar/bedrock/BedrockSidecar.xcodeproj/project.pbxproj`, `docs/msc2/rolling-plan.md`
**What:** Restore the Intel kernel checksum build setting in the Release sidecar configuration. P12.45 updated the initramfs checksum but accidentally dropped this companion setting, causing `npm run prepare:agent` to fail under `set -u` before the newly fixed sidecar could be staged.
**Verify:** `xcodebuild build -project sidecar/bedrock/BedrockSidecar.xcodeproj -scheme BedrockSidecar -configuration Release -derivedDataPath /tmp/msc2-bedrock-sidecar-release ARCHS=x86_64 ONLY_ACTIVE_ARCH=NO MSC2_BEDROCK_APPLIANCE_DIR=/Users/camerontemple/msc2/sidecar/bedrock/Resources CODE_SIGNING_ALLOWED=NO`
**Commit:** `P12.46: restore release appliance validation inputs`
**Batch:** solo

### P12.47 — Stop Bedrock through its selected runtime during first start
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/tests/playit_routes.rs`, `docs/msc2/rolling-plan.md`
**What:** Route automatic first-start shutdown through the Bedrock runtime when the active server is Bedrock, preserving the original lifecycle operation so clean sidecar termination can complete pass one or pass two. Keep Java first-start shutdown on the Java lifecycle service and fail the operation if a Bedrock stop request is rejected.
**Verify:** `cargo nextest run -p msc-agent --test playit_routes && cargo fmt --all -- --check`
**Commit:** `P12.47: stop bedrock through its selected runtime during first start`
**Batch:** solo

### P12.48 — Reuse the first-start operation when stopping Bedrock
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/tests/playit_routes.rs`, `docs/msc2/rolling-plan.md`
**What:** Keep the Pass 2 operation alive when the first-start sheet requests `/v1/stop`. Reuse the existing first-start operation instead of replacing it with a separate `bedrock-stop` operation, and make repeated stops during `Stopping` or `Stopped` idempotent so the Bedrock pump can report the clean termination that completes the sheet.
**Verify:** `cargo nextest run -p msc-agent --test playit_routes && cargo fmt --all -- --check`
**Commit:** `P12.48: reuse the first-start operation when stopping Bedrock`
**Batch:** solo

### P12.49 — Show Bedrock addresses in the correct surfaces
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, `clients/desktop-web/src/lib/components/shell/sidebar/HowToConnectSection.svelte`, `clients/desktop-web/tests/screens/first-start.test.ts`, `clients/desktop-web/tests/screens/overview.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Thread the agent-reported local host address into the completed Bedrock first-start sheet, replacing its placeholder text while retaining an honest fallback when discovery is unavailable. Build the sidebar connection rows from the selected server type so Bedrock shows only Bedrock endpoints, while Java retains its optional Geyser rows.
**Verify:** `cd clients/desktop-web && npx prettier --check src/App.svelte src/lib/sections/server-editor/FirstStartSheet.svelte src/lib/components/shell/sidebar/HowToConnectSection.svelte tests/screens/first-start.test.ts tests/screens/overview.test.ts && npx vitest run tests/screens/first-start.test.ts tests/screens/overview.test.ts`
**Commit:** `P12.49: show Bedrock addresses in the correct surfaces`
**Batch:** solo

### P12.50 — Keep Bedrock connection surfaces on one endpoint
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/home/ConnectionCard.svelte`, `clients/desktop-web/tests/screens/overview.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Make the Overview Connection Info card use the selected server's protocol-specific Playit endpoint. A Bedrock server now uses the Bedrock tunnel address and port, matching the sidebar, while Java continues using the Java endpoint and optional Geyser endpoint.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/home/ConnectionCard.svelte tests/screens/overview.test.ts && npx vitest run tests/screens/overview.test.ts`
**Commit:** `P12.50: keep Bedrock connection surfaces on one endpoint`
**Batch:** solo

---

## Phase 12 amendment — modpack creation flow

### P12.50 — Make modpack creation manifest-authoritative and inspectable
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/{AddServerWizard.svelte,UploadStep.svelte,WorldStep.svelte,ConfirmStep.svelte,model.ts}`, `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`, `clients/desktop-web/src/lib/api/generated.ts`, `crates/msc-api/src/dto/addons.rs`, `crates/msc-agent/src/routes/components.rs`, `crates/msc-application/src/modpacks.rs`, `docs/msc2/api-contract/openapi.json`
**What:** Give modpacks a dedicated Create from Modpack path while keeping older modpack uploads on the same semantics. Treat the manifest as authoritative: show its pinned Minecraft/loader context, pass that context to world capabilities, and remove the misleading change-loader/version affordance. Report server files, client-only files skipped, and override files, with an expandable manifest-file list. Present the final page as a newly created server and first world, and expose pack-managed state in Components so individual changes are not offered while whole-pack replacement remains available. Explain CurseForge API-key requirements only for CurseForge archives; Modrinth `.mrpack` imports state that no CurseForge key is needed.
**Verify:** `cd clients/desktop-web && npm run api:check && npx vitest run tests/screens/add-server-wizard.test.ts && npm run build && cd ../.. && cargo fmt --all -- --check && cargo nextest run -p msc-api --test phase8_conformance`
**Commit:** `P12.50: make modpack creation manifest-authoritative`
**Batch:** solo

### P12.51 — Remove decorative accent from pinned modpack context
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/UploadStep.svelte`, `docs/msc2/rolling-plan.md`
**What:** Keep the pinned Minecraft/loader explanation visible while removing the blue left-edge accent from its neutral context block. The information remains labeled and readable without turning a static explanation into a decorative status treatment.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/fleet/wizard/UploadStep.svelte && npx vitest run tests/screens/add-server-wizard.test.ts`
**Commit:** `P12.51: remove pinned modpack accent`
**Batch:** safe

### P12.52 — Remove redundant pinned modpack section
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/UploadStep.svelte`, `docs/msc2/rolling-plan.md`
**What:** Remove the standalone “Pinned by modpack” explanation because the existing Software and Minecraft summary rows already present the manifest-authoritative values. Keep the inspection summary and file contents unchanged.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/fleet/wizard/UploadStep.svelte && npx vitest run tests/screens/add-server-wizard.test.ts`
**Commit:** `P12.52: remove redundant pinned modpack section`
**Batch:** safe

### P12.53 — Make imported modpacks ordinary mutable servers and update checks opt-in
**Status:** awaiting verification
**Files:** `crates/msc-domain/src/app_config_schema.rs`, `crates/msc-domain/src/modpack.rs`, `crates/msc-application/src/{addon_updates.rs,addons.rs,import.rs}`, `crates/msc-agent/src/routes/{components.rs,servers.rs}`, `crates/msc-api/src/dto/{addons.rs,lifecycle.rs,provisioning.rs}`, `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`, `clients/desktop-web/src/lib/sections/fleet/wizard/{ConfirmStep.svelte,model.ts}`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, `fixtures/pack-managed-guard/`, `docs/msc2/rolling-plan.md`
**What:** Keep modpack metadata and explicit whole-pack replacement, but allow normal individual add-on management after import. Return the local add-on inventory without provider calls when the new per-server update preference is off, render the Components tab without waiting for add-on resolution, prevent overlapping refreshes, and expose the opt-in preference during create/import and later in Components. Persist the preference with a false default and carry it through the shared Rust agent/API so Windows, Linux, and macOS use the same behavior.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-domain -p msc-api -p msc-application -- -D warnings && cargo check -p msc-agent --bin msc && cargo test -p msc-domain --test modpack_policy && cargo test -p msc-application --test raw_server_import && cd clients/desktop-web && npm run api:check && npm run build && npm run test:screen-addons`
**Commit:** P12.53: make imported modpacks ordinary mutable servers and update checks opt-in
**Batch:** solo

### P12.56 — Keep server tabs alive within the active server scope
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/{home/HomeSection.svelte,players-online/PlayersOnlineSection.svelte,worlds/WorldsSection.svelte,performance/PerformanceSection.svelte,components/ComponentsSection.svelte}`, `clients/desktop-web/tests/navigation/navigation.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Retain visited tab instances for the current host/server so returning to a tab is immediate, pause hidden live-tab polling, and clear the retained tab set before an accepted server, host, or deep-link context switch.
**Verify:** `cd clients/desktop-web && npx prettier --check src/App.svelte src/lib/sections/home/HomeSection.svelte src/lib/sections/players-online/PlayersOnlineSection.svelte src/lib/sections/worlds/WorldsSection.svelte src/lib/sections/performance/PerformanceSection.svelte src/lib/sections/components/ComponentsSection.svelte tests/navigation/navigation.test.ts && npm run test:navigation && npm run build`
**Commit:** P12.56: keep server tabs alive within the active server scope
**Batch:** solo

## Phase 12 amendment — large raw archive import scan

### P12.57 — Scan server archives without extracting them
**Status:** awaiting verification
**Files:** `crates/msc-application/src/import.rs`, `crates/msc-application/tests/raw_server_scan.rs`, `crates/msc-agent/src/routes/servers.rs`, `clients/desktop-web/src/lib/sections/fleet/wizard/UploadStep.svelte`, `docs/msc2/rolling-plan.md`
**What:** Make raw server ZIP inspection use the archive central directory and a virtual scan filesystem instead of extracting every mod and library into temporary storage. Preserve traversal/symlink validation, world-size reporting, Java flavor detection, and the single-root unwrap while ignoring Finder's `__MACOSX` metadata directory. Run folder and archive scans on Tokio's blocking pool. Dropped `.zip` server archives go directly to the path-based scan so the desktop client does not upload a potentially hundreds-of-megabytes archive as a modpack before inspection; explicit Choose Modpack remains the modpack path.
**Verify:** `cargo nextest run -p msc-application --test raw_server_scan`
**Commit:** P12.57: scan server archives without extraction
**Batch:** solo

## Phase 12 amendment — editor diagnostics

### P12.58 — Clear stale frontend and test-target editor diagnostics
**Status:** awaiting verification
**Files:** `.vscode/settings.json`, `clients/desktop-web/src/lib/`, `clients/desktop-web/tests/auth/desktop/desktop.test.ts`, `docs/msc2/rolling-plan.md`
**What:** Carry the `NumberField` callback rename through every frontend caller, correct the strict TypeScript boundary types, remove one unused performance prop, and format the affected client files. Configure rust-analyzer to check the production workspace targets instead of every integration-test target, so the intentionally deferred TUI scaffold does not multiply expected unused-code diagnostics across the Problems pane. Keep the full test and lint matrix available through the terminal and CI.
**Verify:** `cd clients/desktop-web && npm run check && npm run format:check && npm run test:auth-desktop && cd ../.. && cargo check --workspace && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Commit:** `P12.58: clear editor diagnostics`
**Batch:** stop-after

## Phase 12 amendment — Tauri and headless-agent beta release

### P12.59 — Freeze the beta release artifact contract
**Status:** awaiting verification
**Files:** `docs/msc2/clients/phase12-release.md`, `docs/msc2/rolling-plan.md`
**What:** Record the first-release boundary as the Tauri shell plus the Rust agent/CLI; explicitly leave iOS and the TUI out of the beta artifact set. Define the supported headless Linux baseline, the Ubuntu/Debian installation shape, service and credential-helper ownership, artifact names and target architectures, version/tag rules, unsigned beta limitations, checksum expectations, pairing-code workflow, SSH/Tailscale access pattern, and the physical Windows/Linux verification handoff. Keep the agent/API contract and the remote-client boundary unchanged: a remote Tauri client may control Minecraft servers through the agent but never install or stop the host's operating-system service.
**Verify:** `test -f docs/msc2/clients/phase12-release.md && rg -n "Tauri|headless|pairing|systemd|checksum|unsigned|iOS|TUI" docs/msc2/clients/phase12-release.md`
**Commit:** `P12.59: freeze beta release artifact contract`
**Batch:** solo

### P12.60 — Make Tauri release builds stage a release agent
**Status:** awaiting verification
**Files:** `clients/desktop-web/tools/prepare-agent-dev.mjs`, `clients/desktop-web/package.json`, `clients/desktop-web/src-tauri/tauri.conf.json`, `docs/msc2/rolling-plan.md`
**What:** Make the existing agent-staging command understand the Tauri build profile instead of always compiling `target/debug/msc`. Development keeps its fast debug path; `tauri build` stages the matching `target/release` agent and preserves the macOS Intel Bedrock sidecar/resource validation. Keep the packaged agent and the Tauri shell on one version and fail before bundling if the expected profile binary or required sidecar input is absent. Do not change the agent's headless feature boundary or add the TUI to desktop packaging.
**Verify:** `cd clients/desktop-web && npm run prepare:agent -- --release && test -f src-tauri/target/package/agent/msc && npm exec tauri build -- --no-sign`
**Commit:** `P12.60: stage release agent for tauri builds`
**Batch:** stop-after

### P12.61 — Package the standalone Linux agent and systemd services
**Status:** awaiting verification
**Files:** `packaging/linux/`, `tools/release/`, `crates/msc-platform-linux/src/credential_helper.rs`, `docs/msc2/clients/phase12-release.md`, `docs/msc2/rolling-plan.md`
**What:** Build the Ubuntu/Debian headless package around the single `msc` binary and the existing Linux production credential path. Ship an install/uninstall path that places the binary, creates the agent data/log directories with the installing user's ownership, installs the agent `systemd` unit plus the restricted credential-helper socket/service units, enables the agent for boot, and leaves routine start/stop under `systemctl`. Include a post-install instruction for `msc pairing create --client-kind desktop`; never run pairing as root or put bearer tokens in the unit file, shell history, ordinary configuration, or release metadata. Keep the package free of Tauri/WebKit/desktop dependencies.
**Verify:** `bash -n packaging/linux/install.sh packaging/linux/uninstall.sh && cargo nextest run -p msc-platform-linux --test systemd_unit && python3 tools/phase4/headless-link-check.py --all-artifacts target/release-headless`
**Commit:** `P12.61: package linux headless agent`
**Batch:** solo

### P12.62 — Build cross-platform beta artifacts in GitHub Actions
**Status:** awaiting verification
**Files:** `.github/workflows/release.yml`, `tools/release/`, `docs/msc2/clients/phase12-release.md`, `docs/msc2/rolling-plan.md`
**What:** Add a manually dispatched and version-tagged release-candidate workflow with native macOS, Windows, and Linux runners. Run the targeted client/agent checks, build the release agent/CLI, build the Tauri shell with the correct platform bundle formats, build the Linux headless package, retain explicit no-signing/no-notarization evidence, and upload platform-labeled artifacts without including iOS or TUI outputs. Keep ordinary CI unchanged except for any small shared build-script seam required by the release profile.
**Verify:** `python3 tools/release/check-release-workflow.py .github/workflows/release.yml && git diff --check`
**Commit:** `P12.62: add cross-platform beta workflow`
**Batch:** stop-after

### P12.63 — Publish checksummed beta releases from tags
**Status:** awaiting verification
**Files:** `.github/workflows/release.yml`, `tools/release/`, `docs/msc2/clients/phase12-release.md`, `docs/msc2/rolling-plan.md`
**What:** Extend the successful candidate workflow with a guarded tag-only publication job that creates a GitHub prerelease, uploads the Tauri installers and headless packages, and publishes a single SHA-256 manifest covering every asset. Keep manual dispatch available for artifact-only runs, require an explicit publish input where appropriate, and make failed or partial matrix builds unable to publish. Do not claim code signing, notarization, a signed coordinated-update manifest, or production auto-update support until those keys and checks exist.
**Verify:** `python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard && python3 tools/release/verify-artifact-manifest.py --help`
**Commit:** `P12.63: publish checksummed beta releases`
**Batch:** solo

### P12.64 — Record the physical beta release gate
**Status:** awaiting verification
**Files:** `docs/msc2/clients/phase12-release.md`, `docs/msc2/clients/phase12-release-evidence/`, `tools/release/verify-artifact-manifest.py`, `docs/msc2/rolling-plan.md`
**What:** Add the release evidence checklist and artifact verifier for Cameron's physical-partition run. The gate must cover a clean Ubuntu Server install with no desktop packages, boot-time agent start, SSH access from another network through the chosen tunnel, local pairing-code display, Tauri desktop pairing and reconnect, remote Minecraft start/stop, agent stop/start recovery, Linux logs, Windows installer launch, Windows service ownership after sign-out, and explicit unavailable signing evidence. A green GitHub build is necessary but cannot replace the hands-on Windows/Linux acceptance run.
**Verify:** `python3 tools/release/verify-artifact-manifest.py --manifest target/release/sha256sums.txt --artifacts target/release/artifacts`
**Commit:** `P12.64: record physical beta release gate`
**Batch:** stop-after

## Phase 12 post-phase correction — agent connection teaching flow

### P12.66 — Redesign the agent connection page
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Reorganize the Agents page around the client/agent model and the two setup paths: an agent on this computer and an agent on another computer. Move the existing “MSC has two parts” explanation to a remembered collapsible section, keep service maintenance and secondary pairing actions progressively disclosed, and add concise remote connection teaching that distinguishes the agent address from the one-use pairing code and explains the SSH-tunnel path without changing the pairing or service APIs.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.66: redesign the agent connection page`
**Batch:** solo

### P12.67 — Clarify the SSH connection placeholder
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Replace the abstract `user@agent-host` SSH placeholder with `username@ip-address`, explain that it means the username used to sign in to the other computer followed by `@` and that computer’s IP address, show a concrete example, and give `whoami` and `hostname -I` commands for users who do not know those values.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.67: clarify the SSH connection placeholder`
**Batch:** solo

### P12.69 — Simplify remote connection teaching
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Remove the private-network-address alternative from the remote connection help so first-time users are taught one clear path: open an SSH tunnel, use the local forwarded address, and enter the pairing code from the other computer. Keep the remote address field and existing connection behavior unchanged.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.69: simplify remote connection teaching`
**Batch:** solo

### P12.70 — Expand remote connection steps
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Turn the four remote-agent checklist items into expandable instructions. Explain how to start the agent through the desktop app or headless terminal command, place the SSH tunnel command inside the reachability step, explain where the pairing command runs and what its one-use code does, and clarify that the forwarded local address belongs in the connection form. Keep the four-step sequence and existing pairing/service behavior.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.70: expand remote connection steps`
**Batch:** solo

### P12.71 — Remove duplicate remote-step numbering
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Hide the native ordered-list markers from the expandable remote connection steps so each step displays only its custom numbered circle.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.71: remove duplicate remote-step numbering`
**Batch:** solo

### P12.73 — Add a hint for expandable remote steps
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `docs/msc2/rolling-plan.md`
**What:** Add a quiet inline hint beneath the remote-agent introduction telling users that each numbered step can be clicked for more information.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/setup/AgentSetupSection.svelte && npm run build`
**Commit:** `P12.73: add a hint for expandable remote steps`
**Batch:** solo

## Phase 12 post-phase correction — startup health freshness

### P12.68 — Persist successful startup health and refresh the Overview
**Status:** awaiting verification
**Files:** `crates/msc-application/src/{lifecycle.rs,diagnostics.rs}`, `crates/msc-application/tests/lifecycle_state.rs`, `crates/msc-agent/src/routes/{health.rs,lifecycle.rs}`, `clients/desktop-web/src/{App.svelte,lib/sections/home/HomeSection.svelte}`, `docs/msc2/rolling-plan.md`
**What:** Record a clean startup when a Java or Bedrock server reaches readiness, replacing any earlier failed-start record while preserving Paper's later soft-failure warnings. Mark the dynamic health response as non-cacheable, and send a targeted Overview health refresh after every lifecycle attempt without discarding the retained tab instance or its page-cache behavior.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-application -p msc-agent --bin msc -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && cargo test -p msc-application --test lifecycle_state lifecycle_state_ready_start_replaces_previous_failed_start_record -- --exact && cargo check -p msc-agent --bin msc && cargo nextest run -p msc-agent --test runtime_diagnostics_routes -E 'test(runtime_diagnostics_routes_are_mounted_behind_bearer_auth)' && cd clients/desktop-web && npx prettier --check src/App.svelte src/lib/sections/home/HomeSection.svelte && npm run check && npx vitest run tests/screens/overview.test.ts && npm run build && node ./tools/package-agent-bundle.mjs`
**Commit:** `P12.68: persist successful startup health`
**Batch:** solo

## Phase 12 post-phase correction — cross-platform CI release gate

### P12.72 — Repair cross-platform CI gates
**Status:** awaiting verification
**Files:** `.github/workflows/ci.yml`, `crates/msc-api/tests/provisioning_conformance.rs`, `crates/msc-agent/src/cli/tui/transport.rs`, `crates/msc-agent/tests/{tui_console.rs,tui_support.rs,tui_transport.rs}`, `crates/msc-application/tests/raw_server_scan.rs`, `crates/msc-infrastructure/src/playit.rs`, `docs/msc2/rolling-plan.md`
**What:** Use the published `tauri-driver` version in the Linux desktop-test setup, make the shared TUI transport compile in all-target integration tests, remove the Windows-only dead helper, and apply the already-established release Clippy policy that keeps the unfinished TUI outside the Tauri/agent beta quality gate.
**Verify:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && cargo test -p msc-agent --test cli_service && cargo test -p msc-agent --test tui_console && cargo test -p msc-agent --test tui_transport && cargo test -p msc-agent --test tui_support && python3 tools/release/check-release-workflow.py .github/workflows/release.yml`
**Commit:** `P12.72: repair cross-platform CI gates`
**Batch:** solo

### P12.73 — Repair all-target authentication imports
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/auth.rs`, `docs/msc2/rolling-plan.md`
**What:** Keep the shared desktop pairing error available to production routes and the desktop-auth integration test while allowing all-target CLI pairing tests to compile under Linux's strict unused-import lint.
**Verify:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && cargo test -p msc-agent --test cli_pairing`
**Commit:** `P12.73: repair all-target authentication imports`
**Batch:** solo

### P12.74 — Modernize fixed-size hex fixture parsing
**Status:** awaiting verification
**Files:** `crates/msc-domain/tests/player_nbt.rs`, `docs/msc2/rolling-plan.md`
**What:** Use the fixed-size slice-chunk API for the player-NBT hex fixture helper so the newer hosted Clippy toolchain accepts the existing test logic without changing its decoded bytes.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-domain --test player_nbt -- -D warnings && cargo test -p msc-domain --test player_nbt`
**Commit:** `P12.74: modernize fixed-size hex fixture parsing`
**Batch:** solo

### P12.75 — Repair cross-platform Bedrock smoke boundaries
**Status:** awaiting verification
**Files:** `crates/msc-application/src/bedrock_provisioning.rs`, `crates/msc-agent/tests/{bedrock_production_cli.rs,bedrock_production_lifecycle.rs}`, `crates/msc-agent/tests/support/bedrock_smoke.rs`, `docs/msc2/rolling-plan.md`
**What:** Make fresh Bedrock promotion create the required world root, give parallel production fixtures independent UDP ports, and keep Apple Silicon CLI smoke on its documented unavailable-runtime path instead of entering Linux-only provisioning and lifecycle assertions.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-application -p msc-agent --tests -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && cargo test -p msc-application --test bedrock_provisioning && cargo test -p msc-agent --test bedrock_production_cli && cargo test -p msc-agent --test bedrock_production_lifecycle`
**Commit:** `P12.75: repair cross-platform Bedrock smoke boundaries`
**Batch:** solo

### P12.76 — Align production Bedrock fixture contracts
**Status:** awaiting verification
**Files:** `crates/msc-agent/tests/bedrock_production_cli.rs`, `crates/msc-agent/tests/bedrock_production_lifecycle.rs`, `crates/msc-agent/tests/bedrock_production_smoke.rs`, `crates/msc-agent/tests/bedrock_production_surfaces.rs`, `docs/msc2/rolling-plan.md`
**What:** Keep the hosted production fixtures deterministic and aligned with the real API: player responses do not advertise runtime state, Apple Silicon exercises the unavailable path before Linux-only create/start work, the unavailable-runtime fixture cannot download a live Bedrock archive, and the lifecycle fixture starts outside the first-start auto-stop flow.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --tests -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && cargo test -p msc-agent --test bedrock_production_cli && cargo test -p msc-agent --test bedrock_production_surfaces && cargo test -p msc-agent --test bedrock_production_lifecycle`
**Commit:** `P12.76: align production Bedrock fixture contracts`
**Batch:** solo

### P12.77 — Stabilize hosted contract assertions
**Status:** awaiting verification
**Files:** `crates/msc-agent/tests/bedrock_production_cli.rs`, `crates/msc-agent/tests/bedrock_production_smoke.rs`, `crates/msc-agent/tests/lifecycle_routes.rs`, `docs/msc2/rolling-plan.md`
**What:** Match the hosted assertions to the production contract and fixture lifecycle: validate Bedrock allowlist shape and the newly added name instead of assuming a particular active server, edit a server-owned capacity setting, and update the lifecycle source-marker check to the helper method that actually exists.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --tests -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && cargo test -p msc-agent --test lifecycle_routes && cargo test -p msc-agent --test bedrock_production_cli && cargo test -p msc-agent --test bedrock_production_smoke`
**Commit:** `P12.77: stabilize hosted contract assertions`
**Batch:** solo

### P12.78 — Repair final hosted contract timing and TUI expectations
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/main.rs`, `crates/msc-agent/tests/{bedrock_production_smoke.rs,startup_secret_migration.rs,tui_terminal_lifecycle.rs}`, `docs/msc2/rolling-plan.md`
**What:** Match the TUI lifecycle assertion to the current keyboard-help modal title, wait for native Bedrock runtime readiness before issuing a console command, and complete legacy owner-token migration after config loading has placed the token in the secret store so the real-process restart proof receives its one-time bearer output.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --tests -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && cargo test -p msc-agent --test tui_terminal_lifecycle && cargo test -p msc-agent --test bedrock_production_smoke && cargo test -p msc-agent --test startup_secret_migration`
**Commit:** `P12.78: repair final hosted contract timing and tui expectations`
**Batch:** solo

### P12.79 — Expose hosted production-agent startup diagnostics
**Status:** awaiting verification
**Files:** `crates/msc-agent/tests/{support/bedrock_smoke.rs,bedrock_production_cli.rs,bedrock_production_lifecycle.rs,bedrock_production_surfaces.rs}`, `docs/msc2/rolling-plan.md`
**What:** Keep the real production-agent subprocess diagnostics visible for the cross-platform Bedrock fixtures so a Windows startup-health failure reports its underlying platform error rather than only a 20-second timeout.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --tests -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && cargo test -p msc-agent --test bedrock_production_cli && cargo test -p msc-agent --test bedrock_production_lifecycle && cargo test -p msc-agent --test bedrock_production_smoke && cargo test -p msc-agent --test bedrock_production_surfaces`
**Commit:** `P12.79: expose hosted production-agent startup diagnostics`
**Batch:** solo

### P12.80 — Record owner test-execution boundary
**Status:** awaiting verification
**Files:** `AGENTS.md`, `CLAUDE.md`, `docs/msc2/rolling-plan.md`
**What:** Make Cameron's explicit boundary authoritative: agents must not run tests or create new tests unless he specifically approves the exact test work, and implementation verification must use inspection, static checks, builds, or Cameron's own manual verification.
**Verify:** `diff <(tail -n +4 AGENTS.md) <(tail -n +4 CLAUDE.md)` → identical instructions after the filename-specific line
**Commit:** `P12.80: record owner test-execution boundary`
**Batch:** solo

## Phase 12 post-phase correction — coordinated release updates

This owner-requested correction completes the update foundation that Phase 11
left staged but not connected to a release source or installer. It covers MSC
application releases only: the desktop shell, its bundled agent and required
sidecar, and the independently installed headless agent. It does not update
Minecraft server jars, worlds, loaders, add-ons, modpacks, or plugins. GitHub
is the transport and release-note source; a signed MSC manifest remains the
trust decision. Updates are always local to the computer being updated. A
remote client can check its own desktop, but it cannot install, start, stop, or
replace the operating-system service on another host.

### P12.96 — Establish the signed release-update contract
**Status:** awaiting verification
**Files:** `docs/msc2/clients/phase11-update.md`, `docs/msc2/clients/phase12-release.md`, `packaging/update-release-schema.json`, `docs/msc2/msc2-decisions.md`, `docs/msc2/rolling-plan.md`
**What:** Amend the release contract for the owner-approved update flow: GitHub release metadata and notes are fetched over HTTPS, but only an Ed25519-signed coordinated manifest can make an update eligible; the manifest identifies the release, compatible API range, platform/architecture, exact desktop/agent/sidecar or package/archive assets, and SHA-256 digests. Define explicit confirmation, staging, rollback, preserved user data, bounded downloads, and the local-only privilege boundary. Replace the current Linux package-manager-only exception with the precise distinction between Tauri package updates, standalone headless archive updates, and distribution-managed installs, while keeping remote service control forbidden.
**Verify:** `rg -n "signed|Ed25519|GitHub|explicit|rollback|Linux|headless|package manager|remote" docs/msc2/clients/phase11-update.md docs/msc2/clients/phase12-release.md packaging/update-release-schema.json docs/msc2/msc2-decisions.md`
**Commit:** `P12.96: establish signed release update contract`
**Batch:** solo

### P12.97 — Publish signed platform release metadata
**Status:** awaiting verification
**Files:** `.github/workflows/release.yml`, `tools/release/`, `packaging/update-release-schema.json`, `docs/msc2/clients/phase12-release.md`, `docs/msc2/rolling-plan.md`
**What:** Extend the tag publication workflow to create the signed, canonical update manifest from the final platform assets and release notes, using a GitHub Actions secret for the private signing key that never enters the repository or shipped binaries. Publish one manifest/signature pair whose platform entries distinguish macOS, Windows, Linux desktop packages, and headless archives; fail publication if an asset, digest, signature, or required coordinated member is missing. Keep the existing checksum file as an integrity aid and retain the unsigned-prerelease path as explicitly ineligible for in-app installation until signing is configured.
**Verify:** `python3 tools/release/check-release-workflow.py .github/workflows/release.yml && python3 tools/release/verify-artifact-manifest.py --help && python3 tools/release/sign-update-manifest.py --help`
**Commit:** `P12.97: publish signed platform release metadata`
**Batch:** solo

### P12.98 — Add verified update retrieval and platform installation
**Status:** awaiting verification
**Files:** `crates/msc-infrastructure/src/{lib.rs,release_update.rs}`, `clients/desktop-web/src-tauri/src/{update.rs,lib.rs}`, `clients/desktop-web/src/lib/platform/{types.ts,tauri.ts,index.ts}`, `packaging/linux/`, `docs/msc2/rolling-plan.md`
**What:** Turn the existing native signature/hash verifier and immutable staging directory into a complete local update service. Fetch the signed manifest and release notes from the configured GitHub repository with HTTPS, bounded response sizes, release/version comparison, platform/architecture filtering, and no trust in unsigned API fields; download only the selected signed assets, verify before activation, and make interrupted or invalid work discardable. After separate confirmation, hand off to the correct local installer: coordinated desktop replacement on macOS/Windows, authorized `.deb`/`.rpm` installation for Linux desktop, and the appropriate standalone headless replacement path. Stop/restart the local agent only within the verified replacement sequence, preserve configuration/secrets/worlds/server files, run health recovery, and roll back on failure. Browser and remote-host paths report their boundary instead of attempting native installation.
**Verify:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml && cd clients/desktop-web && npx prettier --check src/lib/platform/types.ts src/lib/platform/tauri.ts src/lib/platform/index.ts && npm run check && npm run build`
**Commit:** `P12.98: add verified update retrieval and installation`
**Batch:** solo

### P12.99 — Add the Settings update workflow
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`, `clients/desktop-web/src/lib/updates/coordinated.ts`, `clients/desktop-web/src/lib/platform/{types.ts,tauri.ts,browser.ts}`, `clients/desktop-web/src/lib/bundle-identity.ts`, `docs/msc2/rolling-plan.md`
**What:** Put Updates at the top of MSC Settings with the actual bundled version, a Check for updates action, clear checking/available/current/error states, release notes and release ID, and a second explicit confirmation before installation. Make the section explain when the installed release is unsigned or when the selected host is remote, keep progress and rollback outcomes visible, and make the browser client state that native installation belongs to the local desktop or headless host. Follow the anti-slop law: one compact functional section, plain status text, no decorative update dashboard or automatic background install.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/app-settings/AppSettingsSheet.svelte src/lib/updates/coordinated.ts src/lib/platform/types.ts src/lib/platform/tauri.ts src/lib/platform/browser.ts src/lib/bundle-identity.ts && npm run check && npm run build`
**Commit:** `P12.99: add settings update workflow`
**Batch:** solo

### P12.100 — Add headless update commands
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/{mod.rs,update.rs}`, `crates/msc-infrastructure/src/release_update.rs`, `packaging/linux/`, `docs/msc2/clients/phase12-release.md`, `README.md`, `docs/msc2/rolling-plan.md`
**What:** Add local CLI commands `msc update check` and `msc update install`, with human-readable and `--json` output, release notes, explicit confirmation, and an explicit non-interactive approval flag for automation. Reuse the same signed-manifest, compatibility, digest, staging, replacement, service-restart, health-check, and rollback rules as the desktop flow. Detect whether the installation is a standalone archive or distribution-managed package: standalone headless installs may replace their own verified binary/resources after authorization, while `.deb`/`.rpm` installations direct the user through the platform package manager. Do not overload `msc status`, do not require a running remote management API for a local self-update, and never allow a remote client request to update another host's service.
**Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc -p msc-infrastructure --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && cargo check -p msc-agent --no-default-features && bash -n packaging/linux/install.sh packaging/linux/uninstall.sh && rg -n "msc update check|msc update install|--json|package manager|standalone" crates/msc-agent/src/cli docs/msc2/clients/phase12-release.md README.md`
**Commit:** `P12.100: add headless update commands`
**Batch:** solo

### P12.101 — Record the cross-platform update gate
**Status:** awaiting verification
**Files:** `docs/msc2/clients/phase12-release.md`, `docs/msc2/clients/phase12-release-evidence/`, `tools/release/`, `.github/workflows/release.yml`, `docs/msc2/rolling-plan.md`
**What:** Record the release/update acceptance evidence and static gate. Cover signed-release publication, current-versus-new version handling, release notes, declined and cancelled updates, invalid signature/digest, interrupted download, already-staged release, macOS/Windows coordinated replacement, Linux desktop package authorization, standalone headless update, package-manager guidance, preserved user data, agent health recovery and rollback, CLI text/JSON behavior, unsigned prerelease refusal, and the rule that remote clients cannot update host services. Leave Cameron's physical macOS, Windows, and Linux runs as the final verification rather than claiming cross-platform support from a local build.
**Verify:** `python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard && python3 tools/release/check-update-gate.py && git diff --check && rg -n "P12\.96|P12\.97|P12\.98|P12\.99|P12\.100|P12\.101" docs/msc2/rolling-plan.md`
**Commit:** `P12.101: record cross-platform update gate`
**Batch:** stop-after

## Product amendment — retire native iOS and supported mobile access

This owner-approved amendment retires the native iOS application and any
supported mobile management client from MSC 2 v1. The shared Svelte client
remains a desktop and desktop-browser product; responsive layout is an
implementation detail, not a mobile support promise. The Rust agent,
HTTP/WebSocket contract, headless CLI, and Tauri remote-host support remain in
scope. Optional Tailscale may provide remote desktop/browser access, but is not
required for ordinary MSC use and does not justify general-LAN management.
Historical phase records and MSC 1 audit evidence continue to mention the iOS
client where necessary to describe work that actually happened; active product
claims, build checks, capability columns, and release paths must not imply a
native or supported mobile control surface.

### P12.102 — Record the native iOS retirement decision
**Status:** DONE
**Files:** `docs/msc2/msc2-decisions.md`, `docs/msc2/MSC2-VISION.md`, `docs/msc2/msc2-product.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/rolling-plan.md`
**What:** Add an owner-approved decision recording that the native iOS client is retired from MSC 2, amend D-004 as superseded, and define the replacement boundary: phone access may use the responsive browser client, while native iOS UI, App Store packaging, iOS-specific notifications, and iOS-specific capability parity are no longer v1 deliverables. Reconcile the vision, product promise, engineering architecture, port-plan gates, and current plan without rewriting historical archive entries or the read-only MSC 1 oracle.
**Verify:** `git diff --check && rg -n "D-033|superseded|responsive browser|native iOS|App Store" docs/msc2/msc2-decisions.md docs/msc2/MSC2-VISION.md docs/msc2/msc2-product.md docs/msc2/msc2-engineering.md docs/msc2/msc2-port-plan.md`
**Commit:** `P12.102: record native ios retirement decision`
**Batch:** solo

### P12.108 — Amend the mobile-support boundary after P12.102
**Status:** awaiting verification
**Files:** `docs/msc2/msc2-decisions.md`, `docs/msc2/MSC2-VISION.md`, `docs/msc2/msc2-product.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/rolling-plan.md`
**What:** Amend D-033 and D-023 so the native iOS client and any supported mobile management client are both out of v1. Supersede P12.102's responsive-browser replacement language without rewriting that completed step. Keep the shared Svelte client as a Tauri desktop and desktop-browser product, retain the headless CLI, and state that optional Tailscale may provide remote desktop/browser access without being required for ordinary MSC use or expanded into general-LAN management. Remove mobile control-surface promises from the active vision, product copy, engineering architecture, and port-plan gates. Update P12.103–P12.107 so their future cleanup removes native-mobile artifacts and promises rather than replacing them with phone-browser support. Do not delete the iOS project in this step.
**Verify:** `test -d clients/ios && git diff --check && rg -n "P12\.108|no supported mobile|desktop browser|optional Tailscale|general-LAN|D-033|D-023" docs/msc2/msc2-decisions.md docs/msc2/MSC2-VISION.md docs/msc2/msc2-product.md docs/msc2/msc2-engineering.md docs/msc2/msc2-port-plan.md docs/msc2/rolling-plan.md`
**Commit:** `P12.108: amend mobile support boundary`
**Batch:** stop-after

### P12.103 — Remove the native iOS project and dedicated checklists
**Status:** awaiting verification
**Files:** `clients/ios/`, `tools/phase4/ios-lifecycle-check.md`, `tools/phase7/ios-provisioning-check.md`, `tools/phase10/ios-contract-check.py`
**What:** Remove the MSC 2-owned native iOS application, Xcode project, Swift sources, iOS tests, screenshots, README, and iOS-only phase checklists/checker from the working tree. Do not replace them with a phone/browser client or mobile support promise. Do not touch the MSC 1 oracle and do not rewrite git history; the removed client remains recoverable through the repository history.
**Verify:** `test ! -e clients/ios && test ! -e tools/phase4/ios-lifecycle-check.md && test ! -e tools/phase7/ios-provisioning-check.md && test ! -e tools/phase10/ios-contract-check.py && git diff --check`
**Commit:** `P12.103: remove native ios client`
**Batch:** stop-after

### P12.104 — Remove iOS-only validation and capability plumbing
**Status:** awaiting verification
**Files:** `tools/phase6/capability-matrix-check.py`, `tools/phase8/phase8-check.py`, `tools/phase10/phase10-check.py`, `tools/phase11/phase11-check.py`, `tools/release/check-release-workflow.py`, `docs/msc2/client-capability-matrix.csv`, `docs/msc2/api-contract/`
**What:** Remove the `ios_status` matrix column and all checker dependencies on an iOS source tree, Xcode target, iOS contract checker, or iOS-specific release exclusion. Update expected client-column sets and contract comments to the remaining supported clients: Tauri desktop, desktop browser, and headless CLI. Do not add a mobile client column or replace the retired app with a phone-browser promise; retain only generic third-party router `mobile_app` meanings and supported desktop/browser responsive layout behavior where those are implementation details.
**Verify:** `git diff --check && if git grep -n -i -E 'ios_status|clients/ios|ios-contract-check|MSCRemoteiOS' -- tools docs/msc2/client-capability-matrix.csv docs/msc2/api-contract .github; then exit 1; fi`
**Commit:** `P12.104: remove ios validation plumbing`
**Batch:** solo

### P12.105 — Remove active native-mobile references
**Status:** awaiting verification
**Files:** `AGENTS.md`, `CLAUDE.md`, `README.md`, `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src/lib/help/TourOverlay.svelte`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/lib/api/generated.ts`, `content/help/handbook/overview.md`, `content/help/handbook/remote-access.md`, `content/help/handbook/tailscale.md`, `crates/msc-agent/web-ui/assets/`
**What:** Rewrite active documentation, onboarding/help copy, generated API descriptions, agent setup architecture text, and the built agent bundle so they describe only the supported Tauri desktop, desktop browser, headless CLI, and optional Tailscale remote-access control surfaces. Turn the old MSC Remote help topic into the supported desktop/browser remote-access explanation or retire it if the content contract no longer needs a separate topic. Remove promises of a native or supported mobile management client; retain router `mobile_app` fixtures only when they describe third-party router software, and retain responsive layout wording only when it describes supported desktop/browser window behavior.
**Verify:** `git diff --check && if git grep -n -i -E 'MSC Remote|iPhone app|phone app|iOS client|native iOS|clients/ios' -- AGENTS.md CLAUDE.md README.md clients/desktop-web/src clients/desktop-web/tests content/help/handbook crates/msc-agent/web-ui/assets; then exit 1; fi`
**Commit:** `P12.105: remove active native-mobile references`
**Batch:** solo

### P12.106 — Reconcile active client references and source provenance
**Status:** awaiting verification
**Files:** `docs/msc2/addons/phase8-scope.md`, `docs/msc2/networking/phase9-scope.md`, `docs/msc2/bedrock/phase10-scope.md`, `docs/msc2/clients/phase11-scope.md`, `docs/msc2/terminal-ui/phase13-scope.md`, `docs/msc2/api-contract/`, `crates/msc-agent/src/`, `crates/msc-api/src/`, `crates/msc-application/src/`, `crates/msc-domain/src/`, `crates/msc-infrastructure/src/`
**What:** Remove active scope claims that MSC 2 still has an iOS or supported mobile client, replace implementation comments that describe either as a live consumer with accurate Tauri desktop, desktop-browser, headless-CLI, or historical-provenance wording, and keep Rust behavior unchanged. Keep optional Tailscale remote desktop/browser access distinct from ordinary local use and do not add general-LAN management. Leave archived rolling-plan history and MSC 1 audit records factual, with the final audit identifying them as historical rather than silently editing the record of completed work.
**Verify:** `git diff --check && if git grep -n -i -E 'iOS client|MSC Remote|clients/ios|ios_status|SwiftUI.*iOS' -- docs/msc2/addons docs/msc2/networking docs/msc2/bedrock docs/msc2/clients docs/msc2/terminal-ui docs/msc2/api-contract crates/msc-agent/src crates/msc-api/src crates/msc-application/src crates/msc-domain/src crates/msc-infrastructure/src; then exit 1; fi`
**Commit:** `P12.106: reconcile active client references`
**Batch:** solo

### P12.107 — Record and verify the native-mobile retirement audit
**Status:** awaiting verification
**Files:** `docs/msc2/ios-retirement-audit.md`, `docs/msc2/rolling-plan.md`, `docs/errorhandling.md`, `docs/msc2/MSC2-VISION.md`, `docs/msc2/families/phase7-api.md`, `docs/msc2/lifecycle/pairing-phase4.md`, `docs/msc2/lifecycle/phase4-scope.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/msc2-product.md`, `docs/msc2/worlds/phase6-api.md`, `docs/msc2/worlds/phase6-scope.md`, `tools/phase6/phase6-gate-smoke.sh`, `tools/phase7/phase7-gate-smoke.sh`
**What:** Record the search terms, repository areas, removed artifacts, intentionally retained historical references, and intentionally retained third-party mobile meanings. Confirm that no native iOS project, iOS test target, iOS capability column, iOS-only release/check path, or active native/supported-mobile product claim remains, while noting that git history and the MSC 1 oracle remain available for historical reference. Do not treat responsive desktop/browser layout as supported mobile access.
**Verify:** `test ! -e clients/ios && git diff --check && if git grep -n -i -E 'clients/ios|MSCRemoteiOS|MSC Remote|iPhone app|iOS client|native iOS|ios_status|ios-contract-check' -- ':!docs/msc2/rolling-plan.md' ':!docs/msc2/rolling-plan-archive.md' ':!docs/msc2/audit/**' ':!docs/msc2/msc2-decisions.md' ':!docs/msc2/ios-retirement-audit.md'; then exit 1; fi`
**Commit:** `P12.107: record native-mobile retirement audit`
**Batch:** stop-after

## Product amendment — retire the terminal UI

This owner-approved amendment removes the full-screen terminal client from
MSC 2 rather than carrying it into a later release. The supported control
surfaces remain the Tauri desktop app, desktop browser, and scriptable
headless CLI. The headless agent, authenticated HTTP/WebSocket contract,
server lifecycle, and remote desktop/browser management remain in scope.
The completed Phase 13 implementation is historical work: it stays recoverable
through git history, but no TUI code, dependency, capability column, release
promise, or active documentation remains in the working tree after these
steps.

### P12.109 — Record the terminal UI retirement decision
**Status:** awaiting verification
**Files:** `docs/msc2/msc2-decisions.md`, `docs/msc2/MSC2-VISION.md`, `docs/msc2/msc2-product.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/rolling-plan.md`
**What:** Add an owner-approved decision that retires the full-screen terminal UI from MSC 2. Supersede the Terminal TUI row in D-015 while preserving D-015's requirement that the scriptable CLI and interactive command confirmations remain. Define the retained boundary in product terms: a headless host still runs the agent, while management happens through the desktop app, desktop browser, or one-shot CLI from another device. Remove any promise that a later TUI release is planned, but do not remove the agent API, WebSocket channels, or headless installation story.
**Verify:** `git diff --check && rg -n "D-034|terminal UI.*retired|full-screen terminal|scriptable CLI|headless CLI" docs/msc2/msc2-decisions.md docs/msc2/MSC2-VISION.md docs/msc2/msc2-product.md docs/msc2/msc2-engineering.md docs/msc2/msc2-port-plan.md`
**Commit:** `P12.109: record terminal ui retirement decision`
**Batch:** solo

### P12.110 — Extract CLI transport and server selection from the TUI namespace
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/transport.rs`, `crates/msc-agent/src/cli/session.rs`, `crates/msc-agent/src/cli/tui/transport.rs`, `crates/msc-agent/src/cli/tui/session.rs`, `docs/msc2/rolling-plan.md`
**What:** Move the one-shot CLI's HTTP client, bearer-token handling, API error decoding, operation polling support, and active-server resolution out of `cli::tui` into CLI-owned modules. Preserve named-command output, `--json`, exit codes, confirmations, and remote host selection exactly. Keep the agent's server-side WebSocket routes for desktop/browser consumers, but remove client-side WebSocket reconnect machinery from the retained CLI transport; the retiring TUI keeps its temporary stream transport until P12.111 removes it. Do not move terminal presentation state or create a second API.
**Verify:** `cargo fmt --all -- --check && cargo check -p msc-agent --bin msc && cargo clippy -p msc-agent --bin msc -- -D warnings && if rg -n "tui::(transport|session)|tokio_tungstenite|futures_util" crates/msc-agent/src/cli/mod.rs crates/msc-agent/src/cli/session.rs crates/msc-agent/src/cli/transport.rs; then exit 1; fi`
**Commit:** `P12.110: extract cli transport from tui`
**Batch:** solo

### P12.111 — Remove the terminal client, dispatch, tests, and dependencies
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/tui/`, `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/main.rs`, `crates/msc-agent/src/auth/local_bootstrap.rs`, `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `crates/msc-agent/tests/tui_*.rs`
**What:** Delete the TUI implementation and its dedicated tests, remove `ratatui`, `crossterm`, and any client-only WebSocket dependencies that P12.110 proves unused, and remove bare-invocation TUI dispatch. Bare `msc` must return the ordinary usage outcome without entering raw mode or emitting terminal control bytes; every named CLI command, `--json`, `--help`, `serve`, and the desktop local-bootstrap path must remain intact. Retain desktop bootstrap authentication while removing the CLI/TUI-only bootstrap client allowance and client code.
**Verify:** `test ! -d crates/msc-agent/src/cli/tui && test -z "$(find crates/msc-agent/tests -maxdepth 1 -name 'tui_*.rs' -print -quit)" && git diff --check && cargo fmt --all -- --check && cargo check -p msc-agent --bin msc && cargo clippy -p msc-agent --bin msc -- -D warnings`
**Commit:** `P12.111: remove terminal ui implementation`
**Batch:** stop-after

### P12.112 — Remove TUI capability-matrix and checker plumbing
**Status:** awaiting verification
**Files:** `docs/msc2/client-capability-matrix.csv`, `tools/phase6/capability-matrix-check.py`, `tools/phase11/phase11-check.py`, `tools/phase11/scope-check.py`, `tools/release/check-release-workflow.py`
**What:** Remove the `tui_status` capability column and every checker expectation or release-path exclusion that treats the TUI as a supported client. Keep the matrix's `cli_status` column as the independent one-shot CLI surface. Preserve WebSocket contract validation and desktop/browser client coverage; a WebSocket route is not TUI-specific merely because the deleted TUI consumed it.
**Verify:** `git diff --check && if git grep -n -i -E 'tui_status|ratatui|crossterm|P13\.' -- docs/msc2/client-capability-matrix.csv tools .github; then exit 1; fi`
**Commit:** `P12.112: remove tui capability plumbing`
**Batch:** solo

### P12.113 — Remove active TUI documentation and preserve historical provenance
**Status:** awaiting verification
**Files:** `docs/msc2/terminal-ui/`, `docs/msc2/rolling-plan.md`, `docs/msc2/rolling-plan-archive.md`, `docs/msc2/clients/phase12-release.md`, `docs/msc2/MSC2-VISION.md`, `docs/msc2/msc2-product.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/msc2-port-plan.md`, `crates/msc-agent/src/ws/notifications.rs`, `AGENTS.md`, `CLAUDE.md`
**What:** Remove the active Phase 13 scope/gate documents and the live Phase 13 plan block, update release and source comments to describe only the retained desktop, browser, headless-CLI, and agent surfaces, and record a short factual historical note in the rolling-plan archive if needed. Do not rewrite completed Phase 13 history, the MSC 1 audit, or git history. Do not remove generic terminal wording that refers to Minecraft's own console or to terminal-safe server execution rather than the retired client.
**Verify:** `git diff --check && test ! -d docs/msc2/terminal-ui && test -z "$(git grep -n '^## Phase 13' -- docs/msc2/rolling-plan.md)" && if git grep -n -i -E '\bTUI\b|terminal UI|terminal dashboard|ratatui|crossterm|P13\.' -- AGENTS.md CLAUDE.md README.md docs/msc2/MSC2-VISION.md docs/msc2/msc2-product.md docs/msc2/msc2-engineering.md docs/msc2/msc2-port-plan.md docs/msc2/clients docs/msc2/networking docs/msc2/addons docs/msc2/bedrock docs/msc2/api-contract crates/msc-agent/src tools .github; then exit 1; fi`
**Commit:** `P12.113: remove active tui documentation`
**Batch:** solo

### P12.114 — Record and verify the terminal UI retirement audit
**Status:** awaiting verification
**Files:** `docs/msc2/tui-retirement-audit.md`, `docs/msc2/rolling-plan.md`
**What:** Record the search terms, source/dependency/test/documentation inventory, extracted CLI boundary, removed artifacts, retained agent/WebSocket behavior, intentionally retained historical references, and the final supported-client set. Confirm that no TUI source, test target, terminal dependency, capability column, bare-launch path, active release promise, or active TUI documentation remains, while noting that the completed implementation remains recoverable through git history and historical planning records remain factual.
**Verify:** `test ! -d crates/msc-agent/src/cli/tui && test ! -d docs/msc2/terminal-ui && git diff --check && if git grep -n -i -E '\bTUI\b|terminal UI|terminal dashboard|ratatui|crossterm|tui_status|P13\.' -- ':!docs/msc2/rolling-plan-archive.md' ':!docs/msc2/audit/**' ':!docs/msc2/msc2-decisions.md' ':!docs/msc2/rolling-plan.md' ':!docs/msc2/worlds/phase6-api.md' ':!docs/msc2/tui-retirement-audit.md'; then exit 1; fi`
**Commit:** `P12.114: record terminal ui retirement audit`
**Batch:** stop-after

### P12.115 — Gate Linux package update variants by target
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/update.rs`, `docs/msc2/rolling-plan.md`
**What:** Compile the Debian and RPM installation-kind variants, plus their update-channel match arms, only on Linux. Preserve package-manager detection and update behavior for Linux while preventing macOS and Windows builds from reporting those valid cross-platform variants as dead code.
**Verify:** `cargo fmt --all -- --check && cargo check -p msc-agent --bin msc && cargo clippy -p msc-agent --bin msc -- -D warnings`
**Commit:** `P12.115: gate linux package update variants by target`
**Batch:** solo

### P12.116 — Remove stale terminal-client references
**Status:** awaiting verification
**Files:** `.github/workflows/ci.yml`, `.vscode/settings.json`, `crates/msc-agent/src/cli/transport.rs`, `docs/msc2/client-capability-matrix.csv`, `docs/msc2/lifecycle/phase4-scope.md`, `docs/msc2/rolling-plan.md`
**What:** Remove active comments and capability/scope notes that still describe the retired full-screen terminal client as scaffolded, retiring, or client-specific. Keep D-034, the retirement audit, the retired Phase 13 note, and other historical records that intentionally document the decision and its provenance.
**Verify:** `git diff --check && if rg -n -i -P '\\bTUI\\b|terminal UI|terminal dashboard|ratatui|crossterm|tui_status' .github .vscode crates/msc-agent/src docs/msc2/client-capability-matrix.csv docs/msc2/lifecycle/phase4-scope.md; then exit 1; fi`
**Commit:** `P12.116: remove stale terminal-client references`
**Batch:** solo

### P12.118 — Show the Geyser LAN address in first start
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/sections/server-editor/FirstStartSheet.svelte`, `docs/msc2/rolling-plan.md`
**What:** Use the detected host address when rendering the Bedrock same-Wi-Fi row for Java servers with Geyser. Keep the configured Bedrock port, IPv6 formatting, and the existing fallback when the host address is unavailable.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/sections/server-editor/FirstStartSheet.svelte && npm run check`
**Commit:** `P12.118: show the geyser lan address in first start`
**Batch:** solo

### P12.117 — Remove obsolete server template storage
**Status:** awaiting verification
**Files:** `crates/msc-domain/src/app_config_schema.rs`, `crates/msc-application/src/provisioning.rs`, `crates/msc-agent/src/`, `crates/msc-api/src/`, `crates/msc-infrastructure/src/`, `clients/desktop-web/`, `content/help/handbook/`, `fixtures/`, `docs/msc2/api-contract/`
**What:** Remove the Paper/plugin template subsystem and its API/CLI/configuration surfaces. Server JARs are downloaded directly into their owning server directory and version changes replace that server-owned JAR; cross-play add-ons remain per-server files under `plugins/`. Preserve existing user directories rather than deleting files during migration, but stop creating, reading, or writing the obsolete template directories.
**Verify:** `cargo check --workspace && cd clients/desktop-web && npm run check && npm run build && cd ../.. && if rg -n -i 'paper.?templates|plugin.?templates|_paper_templates|_plugin_templates|save_downloaded_jars|/v1/templates|TemplateItemDTO|TemplatesResponseDTO|TemplateMutation' crates/msc-agent/src crates/msc-api/src crates/msc-application/src crates/msc-domain/src crates/msc-infrastructure/src clients/desktop-web/src clients/desktop-web/tests content/help/handbook tools crates/msc-agent/web-ui/assets; then exit 1; fi`
**Commit:** `P12.117: remove obsolete server template storage`
**Batch:** solo

### P12.119 — Normalize handbook paragraph spacing
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/markdown.ts`, `clients/desktop-web/src/lib/sections/handbook/HandbookBrowser.svelte`, `docs/msc2/rolling-plan.md`
**What:** Treat consecutive non-empty Markdown lines as one paragraph, so source files may use readable hard-wrapped prose without rendering each line as a separate block. Give handbook `##` subsection headings deliberate spacing and a smaller hierarchy than the topic title.
**Verify:** `cd clients/desktop-web && npx prettier --check src/lib/help/markdown.ts src/lib/sections/handbook/HandbookBrowser.svelte && npm run check`
**Commit:** `P12.119: normalize handbook paragraph spacing`
**Batch:** solo
