# MSC 2 — Rolling Plan

> ## STATUS: Phase 14 operational refinements are in progress; P14.4, P14.5, P14.6, P14.9, P14.10, P14.11, P14.12, P14.15, P14.16, P14.17, P14.18, P14.19, P14.26, P14.27, P14.28, P14.29, P14.30, P14.31, P14.32, P14.33, P14.34, and P14.35 are awaiting verification.
> **Next move:** Cameron runs the outstanding Phase 14 verification commands, including the focused release-signature regression check in P14.26, signing-pipeline validation in P14.32, active-world size verification in P14.34, and the console-delivery correction in P14.35, and closes each step if its behavior is sound. P14.32 identifies a release-key rotation and one-time manual recovery install as prerequisites to restoring automatic updates for already-installed binaries. P14.33 prepares recovery release v0.1.8; release only after world-size and console behavior are verified, CI is green, and the rotated key verifies its detached signature. The current workspace has an unrelated pre-existing `dead_code` failure in `crates/msc-application/tests/provisioning.rs:152`. Phase 12 visual parity, anti-slop review, release/update handoff, and Bedrock product acceptance are recorded complete on 2026-09-08. P12.121–P12.189 are archived below with all verification entries recorded as DONE. The planned Phase 13 full-screen terminal client remains retired by D-034.

The detailed Phase 12 working plan is preserved in `rolling-plan-archive.md` under “Reconciliation snapshot — 2026-09-08”. This file contains only the current status and next move.

---

## How this document works

This is the working state of the build. The vision documents say where MSC 2 is going; the port plan says the intended sequence; this file says where the repository actually is now.

Phases come from `msc2-port-plan.md`. Steps are written as work arrives rather than being invented in advance. Each step has a status, file scope, description, verification command, commit subject, and batch classification.

Phase 12 is complete. Phase 14 is active, with P14.4–P14.19 awaiting owner verification. P14.19 records the gate handoff; the phase remains open until the live Minecraft, real OS-install, and retained-client evidence in the acceptance note is confirmed.

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
- **What:** Replace the current `label + baseUrl` profile with a host record that has a stable host ID and editable connection details: display name, one or more LAN addresses/hostnames, one or more Tailscale addresses/hostnames, preferred route order, SSH username, authentication choice, optional local forwarded port, and the remote MSC management port defaulting to 48001. Store LAN and Tailscale addresses together so either can be selected later, and allow edits when DHCP, DNS, or Tailscale addresses change without creating a new logical host. Store only non-secret SSH metadata in ordinary client state; keep bearer credentials in the existing per-host native secure store and never persist an SSH password in localStorage.
- **Implementation amendment (2026-09-11):** The remote connection UI now derives the SSH destination from the selected LAN/Tailscale address and uses SSH port 22 internally. Remote records are tunnel-first; legacy SSH host/port and manual-tunnel fields remain readable only for compatibility.
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
  - remember the remote host key on the first explicit connection without showing the fingerprint; block a changed key, explain it without displaying key values, and require explicit approval before trusting the replacement;
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
- **What:** Rework the existing manual setup section into a compact in-app guided flow. The form collects and explains:
  - host name;
  - LAN address/hostname and optional Tailscale address/hostname, with collapsed cross-platform help for finding both;
  - username and password/key/agent choice;
  - remote MSC port, default `48001`;
  - local forwarded port, default `48002`, with collision detection and an automatic alternative;
  - LAN/Tailscale route selection for the SSH destination.

  Show the generated command as an explanation of what MSC is doing, for example `ssh -N -L 48002:127.0.0.1:48001 username@host`, while making clear that the user does not need to run it manually. The normal wizard always uses the managed SSH tunnel, fixes SSH port 22 internally, and does not expose separate SSH hostname, SSH port, direct-access, or manual-tunnel controls. The redundant route explanation, wizard step labels, and extra network-help copy were removed after owner review; the normal flow now goes directly from connection details to the connection review.
- **Implementation amendment (2026-09-11):** Keep “Review connection” clickable so invalid details produce a specific explanation instead of a silently disabled button. Normalize number-field text to valid numeric ports for validation, collision detection, the generated tunnel command, and connection submission.
- **Implementation amendment (2026-09-11):** First-time “Save and connect” now remembers the SSH host key without showing its fingerprint. A changed identity still stops the connection and requires explicit approval, but MSC explains the change without displaying either fingerprint.
- **Implementation amendment (2026-09-11):** The wizard no longer displays fingerprint values or comparison instructions. First-time setup remembers the identity on “Save and connect”; changed identities still require explicit approval, with the warning shown in plain language and no key values exposed.
- **Release preparation (2026-09-11):** Synchronized application, embedded agent, installer, lockfile, bundle identity, and README references to `0.1.5`; publication awaits the tagged release workflow.
- **Verify:** `npm run check`
- **Batch:** F — remote connection experience
- **Commit:** `P14.13: add guided remote host connection flow`

### P14.14 — Automate remote agent discovery and desktop pairing

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/auth/desktop.ts`, Tauri bridge, CLI pairing output, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, auth/API contract docs
- **What:** After the managed tunnel is established, perform the normal health/capability check. If the host has no saved desktop credential, use the authenticated SSH session to invoke a narrowly-scoped remote pairing operation equivalent to `msc pairing create --client-kind desktop --json`, capture the one-use short-lived challenge, exchange it through the forwarded management connection, and store the resulting durable bearer credential in the OS secure store keyed by the stable host ID. The ordinary flow must not ask the user to copy a pairing code or open a second SSH session. The UI should say what is happening (“creating a one-time desktop authorization on the host”) and show progress/failure plainly.
- **Implementation amendment (2026-09-11):** The selected LAN/Tailscale address is now the SSH destination, while authenticated API traffic and pairing use the local forwarded address. This keeps route choice, tunnel transport, and saved credentials aligned.

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
- **What:** Cover the failure cases that would otherwise make the guided flow unsafe or confusing: wrong SSH password, unsupported key format, locked SSH agent, changed SSH identity, occupied local port, unreachable LAN address, unreachable Tailscale address, tunnel process exit, remote `msc` missing from PATH, agent stopped, agent below the supported version floor, pairing challenge expiry, revoked token, and switching hosts during an active operation. Error messages must identify whether the failure is network, SSH, MSC agent, authentication, or Minecraft. Sensitive input must be redacted from logs, screenshots, diagnostics, and error telemetry (MSC has no hosted telemetry). Keep the existing per-host credential and permission model; the tunnel is transport, not authorization.
- **Verify:** `rg -n "password|private key|fingerprint|pairing|48001|48002|remote client|service" clients/desktop-web/src-tauri clients/desktop-web/src/lib docs/msc2/msc2-engineering.md docs/msc2/msc2-decisions.md`
- **Batch:** G — remote hardening and documentation
- **Commit:** `P14.16: harden managed remote connections`

### P14.17 — Explain the no-third-party and optional-Tailscale paths

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, handbook/help content, remote-access documentation, CLI help text, product/engineering docs
- **What:** Teach the connectivity choices without requiring a third-party service. State plainly that there is no “Tailscale without Tailscale” magic: a remote computer must be reachable by a direct LAN/WAN route, an SSH route, a user-operated VPN/overlay, or a relay. MSC's normal built-in path is direct access when available plus an app-managed SSH tunnel; Tailscale remains an optional convenient private route, not a prerequisite. Explain that router port forwarding and public exposure carry their own security burden, while the management API remains authenticated and loopback-first by default. Make the wizard useful for local IPs, Tailscale IPs, DNS names, and manually maintained tunnels.
- **Verify:** `rg -n "Tailscale|SSH tunnel|48001|48002|no.*relay|direct|VPN|port forwarding" clients/desktop-web/src/lib docs/msc2/msc2-product.md docs/msc2/msc2-engineering.md docs/msc2/msc2-decisions.md`
- **Batch:** G — remote hardening and documentation
- **Commit:** `P14.17: document remote access choices`

### P14.18 — Verify release and platform coverage

- **Status:** awaiting verification
- **Files:** `.github/workflows/release.yml`, `packaging/`, `tools/release/`, Tauri manifests, platform capability matrix, installation and remote-access documentation, `docs/msc2/capabilities/phase14-acceptance.md`
- **What:** Perform the final static and manual acceptance pass across macOS, native Windows, and Linux. Confirm each platform has a usable headless artifact and PATH story; the Tauri app can save/edit LAN and Tailscale endpoints; managed SSH can prompt and reconnect; port `48001` is treated as the remote management port; local `48002` forwarding is configurable; and no flow requires Tailscale, a manually opened terminal, or a second manual pairing session. Include the Linux headless Xubuntu scenario, macOS local/remote scenarios, and native Windows installation scenario. Check that package/archive updates and uninstall behavior preserve or remove only the state they own.
- **Verify:** `cargo fmt --all -- --check && cargo check --workspace && npm run check`
- **Batch:** H — cross-platform acceptance
- **Commit:** `P14.18: verify operational refinements across platforms`

### P14.19 — Phase gate and owner verification handoff

- **Status:** awaiting verification
- **Files:** `docs/msc2/rolling-plan.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/msc2-decisions.md`, acceptance notes and capability matrix
- **What:** Review the phase gate as a product behavior, not merely as completed implementation steps. The static handoff is recorded in `docs/msc2/capabilities/phase14-acceptance.md`, but the gate is not claimed complete until Cameron confirms the required live Minecraft, real OS-install, and retained-client walkthrough evidence. The gate holds only when: time-of-day shortcuts preserve the current Minecraft day across the supported Java flavors and Bedrock; explicit day changes remain explicit; all MSC-generated monitoring, backup, and helper traffic cannot evict human console output; `msc` is discoverable after headless installation on all three operating systems; a Tauri user can save both LAN and Tailscale routes, edit changed addresses, see the SSH command being taught, let the app manage forwarding, and pair without routine manual code copying; manual recovery remains available; and the no-cloud/no-required-Tailscale boundary is still true. Record any amended decision or deferred edge case before proposing the next phase.
- **Verify:** `rg -n "P14\.1[1-9]|Status:|Verify:|Batch:" docs/msc2/rolling-plan.md`
- **Batch:** H — cross-platform acceptance
- **Commit:** `P14.19: record Phase 14 gate and handoff`

### P14.20 — Repair cross-platform CI regressions

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/console_buffer.rs`, `crates/msc-agent/src/ws/console.rs`, `docs/msc2/clients/phase11-auth.md`, `docs/msc2/client-capability-matrix.csv`, `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, three desktop-web files reported by Prettier
- **What:** Address the shared failures from CI run 34660639860. Classify routine metric, player-count, session-status, and Xbox Broadcast output before bounded public console history and WebSocket delivery, while preserving errors and prompts. Add the missing General-LAN authentication boundary and `/v1/time/relative` capability-matrix entry. Isolate browser harness setup/reconnect state per browser context so parallel smoke tests cannot change one another's onboarding or reconnect result. Format the three client files reported by CI. This step does not move or republish a release tag.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the new commit's CI run is green
- **Batch:** I — CI regression repair
- **Commit:** `P14.20: repair cross-platform CI regressions`

### P14.21 — Repair browser host startup and remaining client formatting

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/setup/connection/RemoteConnectionWizard.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Fix the shared browser startup failure found in CI: the active server was selected in the per-host cache before the server list had been copied into that cache, causing connection initialization to throw and leaving onboarding, guides, reconnect, and server management unavailable. Store the server list before selecting the active server. Format the additional Svelte file reported by client validation. Do not move or republish a release tag.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the new commit's CI run is green
- **Batch:** I — CI regression repair
- **Commit:** `P14.21: restore browser host context before selecting server`

### P14.22 — Refresh outdated client source assertions

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/tests/navigation/navigation.test.ts`, `clients/desktop-web/tests/agent-install/agent-install.test.ts`, `clients/desktop-web/tests/screens/first-launch-reset.test.ts`, `docs/msc2/rolling-plan.md`
- **What:** Update existing source-contract assertions that still expected the pre-Phase-14 host ID declaration, transport setup location, remote pairing URL, and older setup-screen wording/error handling. Assert against the current shared host constant, browser/Tauri transport implementation, profile-based pairing URL, and owner-approved control-panel/agent explanation. This aligns validation expectations with the implemented behavior; it does not add tests or change runtime behavior.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the new commit's CI run is green
- **Batch:** I — CI regression repair
- **Commit:** `P14.22: align client source assertions with current host flow`

### P14.23 — Stabilize cross-platform browser smoke timing

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/playwright.config.ts`, `docs/msc2/rolling-plan.md`
- **What:** Run the Playwright browser smoke with one worker in CI while preserving default parallelism for local development. The browser cases share an in-memory contract server and exercise stateful setup/reset navigation; runner-dependent parallel scheduling coincided with an Ubuntu WebKit timeout when the client reset was expected to reopen the first-launch tour. This avoids overlapping those browser workflows and makes CI scheduling consistent across operating systems.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the new commit's CI run is green
- **Batch:** I — CI regression repair
- **Commit:** `P14.23: stabilize cross-platform browser smoke timing`

### P14.24 — Prepare the v0.1.6 release

- **Status:** awaiting verification
- **Files:** `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `clients/desktop-web/src/lib/bundle-identity.ts`, `clients/desktop-web/src/lib/bundle-identity.test.ts`, `README.md`, `docs/msc2/rolling-plan.md`
- **What:** Synchronize all release identity versions and current download/update instructions to `0.1.6`. The existing `v0.1.5` tag remains untouched because it points to the earlier release-preparation commit, before the latest connection-flow corrections and CI repairs. Publish `v0.1.6` only from the current green mainline commit, using the guarded release workflow to build and attach the platform desktop/headless artifacts and signed update metadata.
- **Superseded (2026-09-12):** After `v0.1.6` was published, the owner clarified that `v0.1.5` had never been released and was the intended next version. The replacement is recorded in P14.25.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the version-bump commit's CI is green before pushing `v0.1.6`
- **Batch:** I — release and update handoff
- **Commit:** `P14.24: prepare v0.1.6 release`

### P14.25 — Correct the release version to v0.1.5

- **Status:** awaiting verification
- **Files:** `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `clients/desktop-web/src/lib/bundle-identity.ts`, `clients/desktop-web/src/lib/bundle-identity.test.ts`, `README.md`, `docs/msc2/rolling-plan.md`
- **What:** Owner clarification: `v0.1.5` was tagged but never released, so `v0.1.6` was an unintended version skip. Restore the coordinated application, agent, installer, lockfile, bundle, and README versions to `0.1.5`. After this commit passes CI, replace the published `v0.1.6` prerelease and tag, move the old `v0.1.5` tag from its stale release-preparation commit to this verified commit, then let the guarded workflow rebuild and publish the correctly versioned assets and signed update metadata.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the correction commit's CI is green before replacing either tag
- **Batch:** I — release and update handoff
- **Commit:** `P14.25: correct release version to v0.1.5`

### P14.26 — Accept signed update signatures with a final newline

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/release_update.rs`, `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `clients/desktop-web/src/lib/bundle-identity.ts`, `clients/desktop-web/src/lib/bundle-identity.test.ts`, `README.md`, `docs/msc2/rolling-plan.md`
- **What:** Fix the headless updater failure reported by the owner: the release signer writes the detached Base64 Ed25519 signature with a final newline, while the updater previously decoded the raw file bytes strictly and rejected that valid format. Trim only surrounding ASCII whitespace before decoding; malformed Base64 within the signature remains rejected. Add one focused regression test for the exact signer output shape. Synchronize the coordinated application, agent, desktop, lockfile, bundle, and README release identity to `0.1.6`; do not publish until verification and CI are green.
- **Verify:** `cargo test -p msc-infrastructure release_update::tests::accepts_signer_signature_file_with_trailing_newline`
- **Batch:** I — release and update handoff
- **Commit:** `P14.26: accept signed update signatures with final newline`

### P14.27 — Pass the known-hosts file as one SSH option
- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src-tauri/src/ssh.rs`, `docs/msc2/rolling-plan.md`
- **What:** Fix the managed SSH tunnel and remote pairing bootstrap failure reported on v0.1.6. OpenSSH requires each `-o` argument to contain a complete `Name=Value` option; MSC previously passed `UserKnownHostsFile` without its value, then passed the path as a separate argument, so SSH rejected the command before connecting. Build `UserKnownHostsFile=<path>` as one OS string and pass it as the value of `-o`, preserving non-UTF-8 path bytes. This uses the shared SSH command builder, so both the managed tunnel and pairing command are corrected without changing host-key trust behavior.
- **Verify:** `cargo fmt --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- -D warnings`
- **Batch:** J — SSH connection repair and v0.1.7 release
- **Commit:** `P14.27: pass known-hosts path as one SSH option`

### P14.28 — Prompt for SSH password during guided connection
- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/setup/connection/RemoteConnectionWizard.svelte`, `clients/desktop-web/src-tauri/src/ssh.rs`, `docs/msc2/rolling-plan.md`
- **What:** Fix the guided connection that stayed at “Creating authorization…” when the remote Ubuntu account used password-based SSH. New hosts now default to password authentication and ask for the password after the connection review when the user clicks **Save and connect**; the value is sent only for that attempt/session and is not persisted. Hosts using SSH-agent or private-key authentication now run without interactive password prompts when no password is supplied, avoiding an invisible OpenSSH askpass wait. Bound remote pairing execution and the HTTP exchange with deadlines so neither can leave the connection button spinning forever.
- **Verify:** `cd clients/desktop-web && npm run format:check && npm run check && npm run build && cd ../.. && cargo fmt --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- -D warnings`
- **Batch:** J — SSH connection repair and v0.1.7 release
- **Commit:** `P14.28: prompt for SSH password during guided connection`

### P14.29 — Accept a supplied SSH password
- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src-tauri/src/ssh.rs`, `docs/msc2/rolling-plan.md`
- **What:** Fix the local validation branch exposed by Cameron's first password-based connection attempt. The validator previously accepted the `password` authentication mode only when the password was empty; with a supplied password, it fell through to the unsupported-authentication error before SSH could contact Ubuntu. Treat password mode as supported when a non-empty password is supplied, while keeping the explicit missing-password error.
- **Verify:** `cd clients/desktop-web && cargo fmt --manifest-path src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`
- **Batch:** J — SSH connection repair and v0.1.7 release
- **Commit:** `P14.29: accept supplied SSH passwords`

### P14.30 — Prompt again for saved-host SSH passwords
- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/hosts/connection.ts`, `docs/msc2/rolling-plan.md`
- **What:** After MSC quits, the app-owned SSH tunnel and its in-memory password are intentionally cleared. When reconnecting to a saved password-authenticated host, ask for that host's SSH password before showing an agent-unavailable error; keep it in memory only for the current app session. Re-prompt with a clear message if the password is refused, and allow Cancel to return to the local host. Preserve saved/manual tunnel behavior.
- **Verify:** `cd clients/desktop-web && npm run format:check && npm run check && npm run build` — then manually quit and reopen the desktop app, switch to a saved password-authenticated host, confirm the password prompt appears, connect successfully, and confirm an incorrect password can be retried.
- **Batch:** J — SSH connection repair and v0.1.7 release
- **Commit:** `P14.30: prompt for saved-host SSH passwords`

### P14.31 — Prepare the v0.1.7 release
- **Status:** awaiting verification
- **Files:** `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `clients/desktop-web/src/lib/bundle-identity.ts`, `clients/desktop-web/src/lib/bundle-identity.test.ts`, `README.md`, `docs/msc2/rolling-plan.md`
- **What:** Synchronize the coordinated application and agent identity to `0.1.7`, including lockfiles, Tauri metadata, bundle identity, and download/update instructions. Publish only from the green mainline using the guarded release workflow; it will build the platform desktop/headless artifacts and signed update metadata.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm CI for the version-bump commit is green before pushing tag `v0.1.7`.
- **Batch:** J — SSH connection repair and v0.1.7 release
- **Commit:** `P14.31: prepare v0.1.7 release`

### P14.32 — Correct Ed25519 update signing

- **Status:** awaiting verification
- **Files:** `tools/release/sign-update-manifest.py`, `tools/release/generate-update-key.py`, `tools/release/check-release-workflow.py`, `docs/msc2/rolling-plan.md`
- **What:** Fix the Ed25519 point-recovery equation shared by the manifest signer and release-key generator. The old equation produced non-standard public keys and signatures that the Rust updater correctly rejected, while the signer compared key material using the same faulty calculation. Confirm the standard Ed25519 base point before use and independently verify every generated manifest signature with OpenSSL before release publication. The currently configured release key must be rotated to a standard Ed25519 key pair before another signed release can publish. Existing 0.1.6/0.1.7 agents trust the old key and cannot authenticate signatures from a corrected key, so restoring automatic updates requires one manually installed recovery release; update data remains separate from the executable and is preserved by the headless installer.
- **Verify:** `python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard && python3 -c 'import ast, pathlib; [ast.parse(pathlib.Path(p).read_text()) for p in ("tools/release/sign-update-manifest.py", "tools/release/generate-update-key.py", "tools/release/check-release-workflow.py")]'`
- **Batch:** K — update-signature recovery
- **Commit:** `P14.32: correct Ed25519 update signing`

### P14.33 — Prepare the v0.1.8 recovery release

- **Status:** awaiting verification
- **Files:** `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `clients/desktop-web/src/lib/bundle-identity.ts`, `clients/desktop-web/src/lib/bundle-identity.test.ts`, `README.md`, `docs/msc2/rolling-plan.md`
- **What:** Synchronize application, agent, Tauri, lockfile, bundle, and download instructions to v0.1.8. This is a one-time recovery release: rotate the GitHub Actions signing secret and public variable to a fresh standard Ed25519 key pair, embed the new public key, and rely on the release signer’s OpenSSL verification before publication. Existing v0.1.6/v0.1.7 installs cannot verify this new trust key; the owner must manually install the signed v0.1.8 headless archive once. The installer replaces only MSC executables/service files; existing server data remains under the configured data directory.
- **Verify:** `python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard && rg -n '0\.1\.8|v0\.1\.8' crates/msc-agent/Cargo.toml Cargo.lock clients/desktop-web/package.json clients/desktop-web/package-lock.json clients/desktop-web/src-tauri/Cargo.toml clients/desktop-web/src-tauri/Cargo.lock clients/desktop-web/src-tauri/tauri.conf.json clients/desktop-web/src/lib/bundle-identity.ts README.md`
- **Batch:** K — update-signature recovery
- **Commit:** `P14.33: prepare v0.1.8 recovery release`

### P14.34 — Measure the configured active world on Java and Bedrock

- **Status:** awaiting verification
- **Files:** `crates/msc-application/src/worlds.rs`, `crates/msc-application/src/lifecycle.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/src/routes/performance.rs`, `clients/desktop-web/src/lib/sections/performance/PerformanceSection.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Replace the hard-coded Java `server/world` measurement with a calculation from the configured `level-name`: sum Java's main, Nether, and End live folders, or Bedrock's `worlds/<level-name>` folder. Do not count archived world-slot ZIPs. Report no value when the active world folder cannot be read, and show `—` rather than claiming the world is `0 B`; label the metric as the active world rather than implying every server has three dimensions.
- **Verify:** `cargo fmt --all -- --check && cargo check --workspace && npm --prefix clients/desktop-web run check`
- **Batch:** L — active-world performance metric
- **Commit:** `P14.34: measure active Java and Bedrock world size`

### P14.35 — Make Hide Auto stop automatic console delivery

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/console_buffer.rs`, `crates/msc-agent/src/ws/console.rs`, `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/components/ApplicationShell.svelte`, `clients/desktop-web/src/lib/components/shell/ConsoleDock.svelte`, `clients/desktop-web/src/lib/sections/console/model.ts`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/api-contract/websocket-v1.json`, `docs/msc2/antiAIslop.md`, `docs/msc2/rolling-plan.md`
- **What:** Correct the P14.6/P14.7 console regression. Restore the Hide Auto toggle, checked by default, and pass its state to both console history and WebSocket requests. While checked, controller/helper output is not sent to that console client; automatic monitoring, time queries, backup coordination, Xbox Broadcast, Playit, and their internal parsers keep running. Keep automatic lines in their own 200-line diagnostics ring and internal consumer history, never in the 5,000-line human console ring, so they cannot evict server or manual-command output. Unchecking Hide Auto may show only that bounded diagnostics history in the console. Expand classification to recognize Spark's actual `spark-worker-pool-…/INFO` multiline output as well as its older logger format and the known metrics/player-count families. Preserve genuine server and manual-command output; helper errors and prompts remain identifiable as automatic and are handled through their status surfaces rather than leaking into the console while Hide Auto is on. Remove the misleading filter explanation and its unapproved dead HelpLink. Add the owner rule to antiAIslop: never add a “Learn more” hyperlink without Cameron's explicit approval, and never ship a link with a missing or nonfunctional destination.
- **Verify:** `cargo fmt --all -- --check && cargo clippy --workspace -- -D warnings && cargo check --workspace && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run api:check`
- **Batch:** C — console retention
- **Commit:** `P14.35: make hide auto stop console delivery`

## Historical records

Detailed records for Setup through Phase 12, including the completed P12.121–P12.189 steps, remain in `rolling-plan-archive.md`.
