# MSC 2 — Rolling Plan

> ## STATUS: Phase 14 operational refinements are in progress; P14.4, P14.5, P14.6, P14.9, P14.10, P14.11, P14.12, P14.15, and P14.16 are awaiting verification.
> **Next move:** Cameron runs the outstanding P14.4–P14.6, P14.8–P14.12, and P14.15–P14.16 verification commands and closes each step if the client behavior, installer contracts, console classification, remote profile, managed SSH capability, saved-host route lifecycle, and remote error/security boundaries are sound. The current workspace has an unrelated pre-existing `dead_code` failure in `crates/msc-application/tests/provisioning.rs:152`. Phase 12 visual parity, anti-slop review, release/update handoff, and Bedrock product acceptance are recorded complete on 2026-09-08. P12.121–P12.189 are archived below with all verification entries recorded as DONE. The planned Phase 13 full-screen terminal client remains retired by D-034.

The detailed Phase 12 working plan is preserved in `rolling-plan-archive.md` under “Reconciliation snapshot — 2026-09-08”. This file contains only the current status and next move.

---

## How this document works

This is the working state of the build. The vision documents say where MSC 2 is going; the port plan says the intended sequence; this file says where the repository actually is now.

Phases come from `msc2-port-plan.md`. Steps are written as work arrives rather than being invented in advance. Each step has a status, file scope, description, verification command, commit subject, and batch classification.

Phase 12 is complete. Phase 14 is active, with P14.4 awaiting owner verification. Later Phase 14 steps remain planned until this source-and-acceptance map is verified.

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
| 14 | Operational refinements: time, console, packaging, and remote hosts | in progress |

## Proposed Phase 14 — operational refinements

This phase turns the issues reported from real use into four bounded areas:
Minecraft time semantics, automatic-console output, cross-platform headless
installation, and a Tauri-managed connection flow for remote hosts. The remote
flow is deliberately detailed because it replaces several manual SSH, tunnel,
and pairing steps at once.

The management service port is **48001**. The desktop's local forwarded port is
**48002** in the examples below, but it is a client-side choice and must be
editable or automatically selected when occupied.

### P14.1 — Record the operational-refinement contracts

- **Status:** DONE
- **Files:** `docs/msc2/msc2-decisions.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/msc2-product.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/rolling-plan.md`
- **What:** Add the decisions that make the rest of the phase testable and prevent accidental scope drift:
  - time shortcuts are semantic “same Minecraft day” actions; exact day changes are separate and explicit; raw numeric `time set` remains an explicit absolute command;
  - the behavior must be shared by all supported Java flavors and Bedrock rather than being a Java-only UI trick;
  - automatic metric commands remain internal monitoring traffic and cannot displace human console history;
  - headless installation is a first-class macOS, Windows, and Linux contract, including a documented `msc` PATH entry;
  - a Tauri desktop may manage an SSH tunnel and perform the remote pairing bootstrap, but MSC does not operate a cloud relay or require Tailscale;
  - the remote client does not gain an API for installing, starting, stopping, replacing, or uninstalling the operating-system service; SSH setup actions must respect that boundary;
  - host identity is stable even when LAN and Tailscale addresses change, and credentials remain scoped to that host.
- **Verify:** `rg -n "same Minecraft day|48001|automatic metric|PATH|SSH tunnel|stable host|Tailscale" docs/msc2/msc2-decisions.md docs/msc2/msc2-engineering.md docs/msc2/msc2-product.md docs/msc2/api-contract/openapi.json`
- **Batch:** A — contracts and source map
- **Commit:** `P14.1: record operational refinement contracts`

### P14.2 — Build the source and acceptance matrix

- **Status:** DONE
- **Files:** `docs/msc2/rolling-plan.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/capabilities/`, `clients/desktop-web/src/lib/hosts/`, `clients/desktop-web/src/lib/sections/setup/`, `packaging/`, `tools/release/`
- **What:** Map every reported behavior to its current implementation, MSC 1 oracle, API boundary, client surface, and platform acceptance evidence. The matrix must explicitly cover Java Vanilla/Paper/Purpur/Fabric/Forge/NeoForge, Bedrock, Tauri on macOS/Windows/Linux, the served browser client, and the headless CLI. It must call out which checks are static inspection, which are live Minecraft verification, and which require a real OS install. No implementation work starts until this map identifies the owning layer for each behavior.
- **Amendment (2026-09-11):** The first matrix wording was too narrow: its C1 row named periodic TPS/player/Spark polling but did not explicitly inventory backup save commands or helper-process output. P14.2 remains complete as the source-map step, but this correction expands C1 and adds C3/C4 in `docs/msc2/capabilities/phase14-operational-refinements.md`. The later console steps must therefore cover all MSC-generated traffic, not metrics alone.
- **Verify:** `rg -n "P14\.1|P14\.2|time|console|headless|SSH|pairing|48001|48002" docs/msc2/rolling-plan.md`
- **Batch:** A — contracts and source map
- **Commit:** `P14.2: map operational refinement evidence`

### P14.3 — Define the cross-runtime relative-time operation

- **Status:** DONE
- **Files:** `crates/msc-domain/`, `crates/msc-api/`, `crates/msc-agent/src/routes/commands.rs`, `docs/msc2/api-contract/openapi.json`, generated client types, domain fixtures if the existing fixture system needs new cases
- **What:** Add an explicit operation for a relative time preset instead of sending `time set 1000`, `13000`, or `18000` directly. The agent must query or use the runtime's current absolute daytime, derive the current Minecraft day and tick position, calculate the target tick within that same day, and send the runtime-specific absolute command needed to reach it. Define behavior at day boundaries, for a stopped server, when the runtime cannot answer the query, and when a server reports an unsupported time capability. Preserve raw command entry as an explicit absolute operation. The API must make the distinction visible so clients cannot accidentally recreate the old behavior.
- **Verify:** `cargo check --workspace`
- **Batch:** B — relative Minecraft time
- **Commit:** `P14.3: add relative Minecraft time operation`

### P14.4 — Update sidebar and command-picker time actions

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/components/ApplicationShell.svelte`, `clients/desktop-web/src/lib/components/shell/ControlSidebar.svelte`, `clients/desktop-web/src/lib/components/shell/ConsoleDock.svelte`, `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, `clients/desktop-web/src/lib/sections/console/CommandPaletteSheet.svelte`, `clients/desktop-web/src/lib/sections/console/model.ts`, generated API client types
- **What:** Route Dawn, Dusk, and Night buttons through the new semantic operation in the sidebar and terminal command picker. Label or group exact-day actions separately so “change the day” is deliberate rather than an invisible side effect of choosing a time of day. Keep typed raw commands available and explain that a numeric `time set` is absolute. Use capability discovery to disable or explain an unsupported action rather than guessing across Java and Bedrock. Verify the full supported Java-flavor and Bedrock mapping against the matrix from P14.2.
- **Verify:** `npm run check`
- **Batch:** B — relative Minecraft time
- **Commit:** `P14.4: make time shortcuts day-relative`

### P14.5 — Separate human console history from automatic monitoring traffic

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/console_buffer.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/src/backup_operations.rs`, `crates/msc-application/src/backups.rs`, `crates/msc-application/src/bedrock_service.rs`, `crates/msc-agent/src/routes/networking.rs`, `crates/msc-application/src/playit.rs`, `crates/msc-application/src/xbox_broadcast.rs`, `crates/msc-agent/src/ws/console.rs`, console WebSocket/history DTOs, `docs/msc2/api-contract/openapi.json`
- **What:** Define the retention and delivery contract for every MSC-generated source before changing the buffer. This includes periodic monitoring (`list`, `tps`, `forge tps`, `neoforge tps`, `spark tps`, `tick query`), relative-time's internal `time query gametime`, backup save coordination (`save-all flush`, `save-off`, `save-on`, `save hold`, repeated `save query`, and `save resume`), and helper output from Xbox Broadcast and Playit. Internal parsers and operation waiters must continue receiving these events, but hidden controller/helper traffic must not enter or displace the bounded human console ring. Keep optional helper/controller diagnostics separate, smaller, and independently bounded. Apply filtering before retention and before history/WebSocket delivery, not after the main buffer fills. Preserve genuine server output and operator-entered commands, even when their text resembles a metric. Move actionable helper errors/prompts to structured helper status, notifications, or a dedicated diagnostics view rather than leaking them into the main console solely because they were not classified as routine.
- **Verify:** `cargo check --workspace`
- **Batch:** C — console retention
- **Commit:** `P14.5: define separate automatic console retention`

### P14.6 — Implement early automatic-output classification

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/console_buffer.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/src/backup_operations.rs`, `crates/msc-application/src/backups.rs`, `crates/msc-application/src/bedrock_service.rs`, `crates/msc-agent/src/routes/networking.rs`, `crates/msc-application/src/playit.rs`, `crates/msc-application/src/xbox_broadcast.rs`, `crates/msc-agent/src/ws/console.rs`, metric parsers, backup waiters, helper status/event code, and console event/history serializers
- **What:** Replace the metrics-only classifier with producer-aware ingestion. Tag each line or event by origin—`user`, `server`, `controller`, or `helper`—at the point it is generated or correlated. Cover periodic `list`/TPS/Spark/tick polling; the one-shot `time query gametime`; Java and Bedrock backup save commands and their confirmation/readiness responses; Xbox Broadcast stdout/stderr, auth prompts, readiness, and failures; Playit stdout/stderr, retries, and failures; retries, delayed responses, multiline responses, server restarts, and overlapping operations. Internal metrics and backup waiters must consume the controller stream before presentation filtering. The main console ring and reconnect backfill must retain human/server output only by default, while any diagnostics stream is independently bounded and cannot evict it. Do not hide operator-entered commands or genuine server warnings merely because their text contains `TPS`, `list`, or another known automatic pattern.
- **Verify:** `cargo clippy --workspace --all-targets -- -D warnings`
- **Batch:** C — console retention
- **Commit:** `P14.6: filter automatic output before console retention`

### P14.7 — Align console controls and visible behavior

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/console/model.ts`, console components, WebSocket client/event handling, help content for console filters
- **What:** Remove the current assumption that client-side hiding is sufficient. The default console view and reconnect path must request human/server output without controller or routine helper traffic, so “no items match console filters” is not caused by hidden monitoring, backup, Xbox Broadcast, or Playit output consuming the history window. If an explicit diagnostics view is retained, make it a separate bounded view with clear copy and no effect on the main console. Preserve filter/search behavior for genuine server output, manual command echo, and actionable helper state displayed through its proper status/notification surface.
- **Verify:** `npm run check`
- **Batch:** C — console retention
- **Commit:** `P14.7: keep automatic output out of the main console`

### P14.8 — Define the tri-platform headless command-install contract

- **Status:** awaiting verification
- **Files:** `packaging/linux/install.sh`, `packaging/linux/uninstall.sh`, `tools/release/build-linux-headless.sh`, `tools/release/build-macos-headless.sh`, `tools/release/build-windows-headless.ps1`, release workflow, headless installation documentation
- **What:** Decide and document the supported install shapes for macOS, Windows, and Linux. The contract must answer where the executable lives, how a shell discovers `msc`, how upgrades preserve the PATH entry, how uninstall removes only MSC-owned links, how package-managed Linux installs differ from archives, and how a noninteractive/headless install reports that a new shell or PATH refresh is required. Keep the binary name consistent (`msc`/`msc.exe`) and do not imply that Linux is the only platform with headless support.
- **Verify:** `rg -n "headless|PATH|msc\.exe|/usr/local/bin/msc|Windows|macOS|Linux" packaging tools/release .github/workflows docs/msc2`
- **Batch:** D — headless packaging
- **Commit:** `P14.8: define tri-platform headless installation contract`

### P14.9 — Put the Linux headless command on PATH safely

- **Status:** awaiting verification
- **Files:** `packaging/linux/install.sh`, `packaging/linux/uninstall.sh`, Linux package/archive templates, systemd/service documentation
- **What:** Make a normal Linux install expose `msc` without requiring `cd` or `./msc`. For package installs, use the distribution's normal executable location or an owned symlink in a standard command directory such as `/usr/local/bin`; for archive installs, make the choice explicit and idempotent. Detect an existing non-MSC target before replacing it, support upgrades and uninstall cleanly, explain root/user installation differences, and preserve the existing management service on port 48001. Verify both a fresh install and an upgrade do not create duplicate binaries or stale links.
- **Verify:** `bash -n packaging/linux/install.sh && bash -n packaging/linux/uninstall.sh`
- **Batch:** D — headless packaging
- **Commit:** `P14.9: expose Linux headless CLI on PATH`

### P14.10 — Make macOS and Windows headless installs equally usable

- **Status:** awaiting verification
- **Files:** `packaging/macos/install.sh`, `packaging/macos/uninstall.sh`, `packaging/windows/install.ps1`, `packaging/windows/uninstall.ps1`, `tools/release/build-macos-headless.sh`, `tools/release/build-windows-headless.ps1`, release workflow, headless CLI documentation
- **What:** Give macOS and native Windows the same usable command story. macOS must install or clearly guide an owned `msc` link into a standard PATH location without breaking Intel/Apple Silicon packaging. Windows must install `msc.exe` into an owned directory and add/remove that directory from the appropriate user or machine PATH with an explicit elevation choice. Document shell refresh behavior, PowerShell and Command Prompt discovery, upgrade/uninstall ownership, and service installation separately from CLI PATH installation.
- **Verify:** `rg -n "PATH|msc\.exe|headless|Intel|arm64|uninstall" tools/release packaging .github/workflows docs/msc2`
- **Batch:** D — headless packaging
- **Commit:** `P14.10: complete macOS and Windows headless installs`

### P14.11 — Add a durable remote-host connection profile

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/hosts/types.ts`, `clients/desktop-web/src/lib/hosts/saved.ts`, host switcher/state stores, native secure-store bridge, migration documentation
- **What:** Replace the current `label + baseUrl` profile with a host record that has a stable host ID and editable connection details: display name, one or more LAN addresses/hostnames, one or more Tailscale addresses/hostnames, preferred route order, SSH hostname/IP, SSH port, SSH username, authentication choice, optional local forwarded port, and the remote MSC management port defaulting to 48001. Store LAN and Tailscale addresses together so either can be selected later, and allow edits when DHCP, DNS, or Tailscale addresses change without creating a new logical host. Store only non-secret SSH metadata in ordinary client state; keep bearer credentials in the existing per-host native secure store and never persist an SSH password in localStorage.
- **Verify:** `npm run check`
- **Batch:** E — remote connection foundation
- **Commit:** `P14.11: model editable remote host profiles`

### P14.12 — Build the native SSH tunnel/session capability

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src-tauri/`, Tauri invoke/command bridge, desktop auth/transport modules, platform dependency manifests, secure host-key storage
- **What:** Implement the desktop-owned SSH capability used by the profile and connection manager. It must:
  - open `localPort -> 127.0.0.1:48001` on the remote host, using the remembered local port such as `48002`;
  - support SSH host/port/user, password prompt, private-key reference, and the platform SSH agent where available;
  - keep the password in memory only for the connection attempt/session unless the OS credential store is explicitly chosen later;
  - verify and remember the remote host key fingerprint, warn on a changed key, and never silently accept a new identity;
  - expose connection state, stderr, exit reason, and retry/stop controls to Svelte without leaking passwords or bearer tokens to logs;
  - reconnect or report a recoverable failure when the tunnel drops, and clean up the child process when the host is switched or the app closes;
  - work on macOS, Windows, and Linux Tauri builds with the same frontend contract.
  Decide whether to use the system `ssh` executable behind a managed native process or an embedded library; the user experience must not depend on manually opening a terminal.
- **Verify:** `cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
- **Batch:** E — remote connection foundation
- **Commit:** `P14.12: add managed SSH tunnel capability`

### P14.13 — Add the teaching connection wizard in “Connect to another host”
- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/hosts/types.ts`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/lib/sections/setup/connection/RemoteConnectionWizard.svelte`, help content, generated client types
- **What:** Rework the existing manual setup section into an in-app guided flow. The form must collect and explain:
  - host name;
  - LAN address/hostname and optional Tailscale address/hostname;
  - SSH hostname/IP, SSH port, username, and password/key/agent choice;
  - remote MSC port, default `48001`;
  - local forwarded port, default `48002`, with collision detection and an automatic alternative;
  - route preference and whether direct connection should be tried before the SSH tunnel.

  Show the generated command as an explanation of what MSC is doing, for example `ssh -N -L 48002:127.0.0.1:48001 username@host`, while making clear that the user does not need to run it manually. Include a “looks good” review step that summarizes the route, ports, credential storage, and host identity before saving. Keep an advanced/manual path for users who already maintain their own tunnel, but do not make it the normal path.
- **Verify:** `npm run check`
- **Batch:** F — remote connection experience
- **Commit:** `P14.13: add guided remote host connection flow`

### P14.14 — Automate remote agent discovery and desktop pairing

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/auth/desktop.ts`, Tauri bridge, CLI pairing output, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, auth/API contract docs
- **What:** After direct connection or tunnel establishment, perform the normal health/capability check. If the host has no saved desktop credential, use the authenticated SSH session to invoke a narrowly-scoped remote pairing operation equivalent to `msc pairing create --client-kind desktop --json`, capture the one-use short-lived challenge, exchange it through the forwarded management connection, and store the resulting durable bearer credential in the OS secure store keyed by the stable host ID. The ordinary flow must not ask the user to copy a pairing code or open a second SSH session. The UI should say what is happening (“creating a one-time desktop authorization on the host”) and show progress/failure plainly.

  Define recovery explicitly: if remote command execution is unavailable, offer the existing manual pairing-code fallback; if the code expires, create a new one; if the host has an existing credential, use it without re-pairing; if the credential is revoked or the host identity no longer matches, require an intentional repair flow. Do not permit arbitrary shell commands through this feature, and do not let it install/start/stop the operating-system service.
- **Verify:** `npm run check`
- **Batch:** F — remote connection experience
- **Commit:** `P14.14: automate remote desktop pairing`

### P14.15 — Add route selection, host switching, and address repair

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/hosts/connection.ts`, `clients/desktop-web/src/lib/hosts/types.ts`, `clients/desktop-web/src/lib/auth/desktop.ts`, `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/lib/sections/setup/connection/RemoteConnectionWizard.svelte`, `clients/desktop-web/src-tauri/src/lib.rs`
- **What:** Implement the lifecycle for saved hosts. When connecting, try the preferred direct LAN/Tailscale route according to the profile, verify the agent, and fall back to the managed SSH tunnel when direct access is unavailable. Let the user switch explicitly between LAN and Tailscale addresses, edit either address later, and reorder the preference. When a host is selected, start or reuse only that host's tunnel, connect its saved credential, restore its server/console state, and close or suspend the previous host's tunnel according to the connection policy. A normal switch must not request a new pairing code. If an address changes, edit-and-retry must preserve the stable host ID and credential rather than creating duplicate host entries.
- **Verify:** `npm run check`
- **Batch:** F — remote connection experience
- **Commit:** `P14.15: support saved host routes and switching`

### P14.16 — Harden remote connection errors and security boundaries

- **Status:** awaiting verification
- **Files:** Tauri SSH bridge, host/credential stores, auth transport, connection UI, `docs/msc2/msc2-decisions.md`, `docs/msc2/msc2-engineering.md`
- **What:** Cover the failure cases that would otherwise make the guided flow unsafe or confusing: wrong SSH password, unsupported key format, locked SSH agent, changed host fingerprint, occupied local port, unreachable LAN address, unreachable Tailscale address, tunnel process exit, remote `msc` missing from PATH, agent stopped, agent below the supported version floor, pairing challenge expiry, revoked token, and switching hosts during an active operation. Error messages must identify whether the failure is network, SSH, MSC agent, authentication, or Minecraft. Sensitive input must be redacted from logs, screenshots, diagnostics, and error telemetry (MSC has no hosted telemetry). Keep the existing per-host credential and permission model; the tunnel is transport, not authorization.
- **Verify:** `rg -n "password|private key|fingerprint|pairing|48001|48002|remote client|service" clients/desktop-web/src-tauri clients/desktop-web/src/lib docs/msc2/msc2-engineering.md docs/msc2/msc2-decisions.md`
- **Batch:** G — remote hardening and documentation
- **Commit:** `P14.16: harden managed remote connections`

### P14.17 — Explain the no-third-party and optional-Tailscale paths

- **Status:** planned
- **Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, handbook/help content, remote-access documentation, CLI help text, product/engineering docs
- **What:** Teach the connectivity choices without requiring a third-party service. State plainly that there is no “Tailscale without Tailscale” magic: a remote computer must be reachable by a direct LAN/WAN route, an SSH route, a user-operated VPN/overlay, or a relay. MSC's normal built-in path is direct access when available plus an app-managed SSH tunnel; Tailscale remains an optional convenient private route, not a prerequisite. Explain that router port forwarding and public exposure carry their own security burden, while the management API remains authenticated and loopback-first by default. Make the wizard useful for local IPs, Tailscale IPs, DNS names, and manually maintained tunnels.
- **Verify:** `rg -n "Tailscale|SSH tunnel|48001|48002|no.*relay|direct|VPN|port forwarding" clients/desktop-web/src/lib docs/msc2/msc2-product.md docs/msc2/msc2-engineering.md docs/msc2/msc2-decisions.md`
- **Batch:** G — remote hardening and documentation
- **Commit:** `P14.17: document remote access choices`

### P14.18 — Verify release and platform coverage

- **Status:** planned
- **Files:** `.github/workflows/release.yml`, `packaging/`, `tools/release/`, Tauri manifests, platform capability matrix, installation and remote-access documentation
- **What:** Perform the final static and manual acceptance pass across macOS, native Windows, and Linux. Confirm each platform has a usable headless artifact and PATH story; the Tauri app can save/edit LAN and Tailscale endpoints; managed SSH can prompt and reconnect; port `48001` is treated as the remote management port; local `48002` forwarding is configurable; and no flow requires Tailscale, a manually opened terminal, or a second manual pairing session. Include the Linux headless Xubuntu scenario, macOS local/remote scenarios, and native Windows installation scenario. Check that package/archive updates and uninstall behavior preserve or remove only the state they own.
- **Verify:** `cargo fmt --all -- --check && cargo check --workspace && npm run check`
- **Batch:** H — cross-platform acceptance
- **Commit:** `P14.18: verify operational refinements across platforms`

### P14.19 — Phase gate and owner verification handoff

- **Status:** planned
- **Files:** `docs/msc2/rolling-plan.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/msc2-decisions.md`, acceptance notes and capability matrix
- **What:** Review the phase gate as a product behavior, not merely as completed implementation steps. The gate holds only when: time-of-day shortcuts preserve the current Minecraft day across the supported Java flavors and Bedrock; explicit day changes remain explicit; automatic polling cannot evict human console output; `msc` is discoverable after headless installation on all three operating systems; a Tauri user can save both LAN and Tailscale routes, edit changed addresses, see the SSH command being taught, let the app manage forwarding, and pair without routine manual code copying; manual recovery remains available; and the no-cloud/no-required-Tailscale boundary is still true. Record any amended decision or deferred edge case before proposing the next phase.
- **Verify:** `rg -n "P14\.1[1-9]|Status:|Verify:|Batch:" docs/msc2/rolling-plan.md`
- **Batch:** H — cross-platform acceptance
- **Commit:** `P14.19: record Phase 14 gate and handoff`

## Historical records

Detailed records for Setup through Phase 12, including the completed P12.121–P12.189 steps, remain in `rolling-plan-archive.md`.
