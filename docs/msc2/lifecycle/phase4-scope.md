# Phase 4 scope: Java lifecycle vertical slice

**Status:** P4.1 scoping note. CLI packaging confirmed by Cameron Temple, 2026-08-02; the rest awaits P4.1 verification.
**Source of truth:** `msc2-port-plan.md` Phase 4 gate, `msc2-decisions.md`, `docs/msc2/substrate/phase3-scope.md`, and the Phase 0 symbol ledger.
**MSC 1 oracle:** `~/Documents/Swift Projects/minecraft-server-controller`, read-only.

This note fixes the boundaries for Phase 4 before code starts. It does not approve new product behavior on its own; when a choice is still open, this document records the recommended working answer and names what Cameron's approval changes.

## Vertical Slice Definition

"One imported Paper server" means one existing Java server directory whose runtime is Paper or Paper-compatible enough to be detected as Paper by the Phase 4 importer.

In scope:

- Adopt a user-supplied existing Paper server directory by path.
- Read `eula.txt` and `server.properties`.
- Preserve unknown `server.properties` keys through the Phase 1 property model.
- Infer the game port, max players, active world name, Paper jar, and a stable MSC 2 server id where possible.
- Register the directory as the active server without copying, unzipping, provisioning, or mutating the world.
- Start it with a validated Java executable, the Phase 4 Paper launch command, bounded console capture, command input, status/performance snapshots, graceful stop, and restart.
- Drive the same behavior through the HTTP API, CLI, and existing iOS app.
- Prove the same server keeps running after every client closes, and prove service ownership on macOS, Linux, and Windows.

Out of scope:

- Creating or downloading a new server.
- MSC 1 transfer-package import.
- Raw ZIP import.
- Moving, copying, repairing, replacing, or backing up worlds.
- Fabric, Forge, NeoForge, Vanilla, Purpur, Bedrock, mods, plugins, modpacks, networking helpers, and router automation except where a Paper directory happens to contain inert files from those systems.
- Desktop/Tauri/web UI work.
- Full D-012 remote desktop pairing, LAN TLS, Tailscale posture, and browser CSRF/browser-cookie design. Phase 4 only does the real credential path needed by the CLI and existing iOS slice.

If the directory's EULA is not accepted, Phase 4 should report that clearly and refuse to start rather than silently editing it. EULA acceptance UX belongs to the later configuration/import work unless a Phase 4 live test cannot proceed without a narrow explicit command.

## API Slice

The Phase 4 lifecycle route set is:

| Route | Phase 4 behavior |
|---|---|
| `GET /v1/servers` | Return the imported Paper server and active-server state. |
| `POST /v1/servers/import` | Adopt the existing Paper directory described above. |
| `POST /v1/active-server` | Select the imported server as the one lifecycle commands target. |
| `POST /v1/start` | Start the active imported Paper server. |
| `POST /v1/stop` | Request graceful stop of the active server. |
| `POST /v1/command` | Send one console command to the running process. |
| `GET /v1/status` | Report real lifecycle state, not the Phase 2 canned status. |
| `GET /v1/performance` | Report the active server's bounded metrics snapshot. |
| `GET /v1/console/tail` | Return bounded recent console output from the real buffer. |
| Console WebSocket | Send bounded backfill followed by live console lines. |

Real authentication is load-bearing for these routes once they mutate a real server. P4.2/P4.5 replace the Phase 2 fixed `MSC_DEV_TOKEN` with `SecretStore`-backed credentials for this slice before real lifecycle mutation is accepted.

## CLI Packaging Recommendation

Recommendation: ship one Rust binary per platform, installed as `msc`, with `msc serve` starting the agent service process and the other `msc ...` subcommands acting as CLI clients.

Why this matches the controlled set:

- D-002 says the engine, service, API, and CLI ship as a single binary per platform.
- `msc2-engineering.md` already describes `msc serve` and CLI mode as two modes of the same program.
- One binary gives headless installs one artifact to place, sign, update, verify for GUI-free linkage, and register with the platform service manager.
- CLI subcommands should talk through the HTTP API path the iOS client uses, so the CLI does not become a privileged shortcut around route validation, auth, audit, operation journaling, or API conformance.

Practical shape:

```text
msc serve
msc token ...
msc server import ...
msc server start ...
msc server stop ...
msc server restart ...
msc command ...
msc status
msc console tail
```

The exception is `msc serve`: it assembles dependencies and hosts the API. Every other command targets a local or remote agent over the versioned API unless a later, explicit install/service command needs local platform privileges.

Status: **Confirmed by Cameron Temple, 2026-08-02** — one binary is the Phase 4 packaging target. P4.18, P4.21, service definitions, update mechanics, and D-021 no-GUI-link checks should all name one headless artifact unless a later approved decision reopens D-002's single-binary wording.

## CLI Slice

Phase 4 CLI commands are limited to the vertical slice:

- `msc serve` for local agent execution.
- Token/pairing helpers needed by P4.2/P4.5.
- `msc server import <path>` for the existing Paper directory.
- `msc server start`, `msc server stop`, and `msc server restart`.
- `msc command <text>`.
- `msc status`.
- `msc console tail`.
- `--json` output where `msc2-engineering.md` requires scriptable output.

No CLI commands for creation, backups, worlds, mods, players, networking, Bedrock, desktop install UX, or TUI behavior are in this phase.

## iOS Slice

The existing Swift iOS client is a Phase 4 driver, not a status-only observer.

Screens/files expected to move in this phase:

- Pairing/settings storage: real token path and the P2.20 empty-Keychain-token fallback bug.
- Dashboard/status: connection state, imported server visibility, active server state, start, stop, restart, status, and performance fields needed for the slice.
- Console/commands: recent console tail/live stream where available and command send.
- Shared networking/model files: only the DTOs and calls required for the route set above.

No iOS worlds, backups, mods, players, settings editor, router, or Bedrock screens are required by Phase 4 unless a missing minimal field blocks the imported Paper lifecycle gate.

## MSC 1 Oracle Symbols

The Phase 4 Rust code is not designed from memory. These MSC 1 symbols are the primary oracle for the behavior this slice preserves:

| Area | MSC 1 source |
|---|---|
| Java path validation and launch command | `ServerProcessManager.validatedJavaLaunchInfo`, `validateLooksLikeJava`, `startServer`; `JavaServerBackend.start`; `JavaServerLaunchHelper.resolve` |
| Process state, command input, graceful stop, termination ordering | `ServerProcessManager.sendCommand`, `requestStop`, `terminate`; `JavaServerBackend.stop`; `AppViewModel+ServerControls.startServer` and `stopServer` |
| Console byte framing | `ServerProcessManager.handleIncoming`, `flushPendingOutput`; `AppViewModel.drainConsoleBatch`, `commitConsoleBatch`, `LineAccumulator` |
| Ready/running output parsing | `AppViewModel+OutputHandling.handleServerOutputLine`, including Paper `Done (` readiness and Java join/leave parsing |
| Metrics/status | `ServerLifecycleManager.startMetricsTimer`, `AppViewModel+Metrics.updateResourceUsageMetrics`, Phase 1 TPS parsing |
| Paper directory detection/import | `AppViewModel+ServerImport.scanServerDirectory`, `detectJavaFlavor`, `importExistingServer`; `EULAManager`; `ServerPropertiesManager` |
| Orphan/ghost Java process handling | `AppViewModel+JavaProcessCleanup`, `JavaProcessScanner` |
| Java runtime policy | `JavaRuntimeManager`, `ServerProcessManager.validatedJavaLaunchInfo`, and existing Java-runtime fixtures |
| Console display parsing | `ConsoleManager.ConsoleLineParser` for bounded agent-side parsed console records where Phase 4 exposes them |

Phase 4 keeps only the Paper lifecycle subset of these files. Bedrock branches, first-run network/broadcast initiation, backups, force-stop confirmation UI, quick-command catalogs, and mod/plugin diagnostics stay with their later phases unless the Phase 4 gate exposes a direct dependency.

## Service Proof Plan

Phase 4 must prove headless service ownership on all three platforms. A local process launched from a terminal is not enough.

macOS proof:

- Install a root-owned LaunchDaemon plist under `/Library/LaunchDaemons`.
- Set `UserName` to the installing user, per D-025.
- Start the agent through `launchd`, import and start the Paper server through the API/CLI, close all clients, and verify the Java process continues.
- Run the P4.4 LaunchDaemon keychain/TCC check in the daemon context.
- Stop and uninstall cleanly.
- Confirm the headless artifact does not link AppKit or require a WindowServer session.

Linux proof:

- Target Debian 12 or any supported systemd >= 250 host.
- Install a system unit with `User=`/`Group=` set to the installing user.
- Start the agent through `systemd`, import and start the Paper server through the API/CLI, close all clients, and verify the Java process continues.
- Apply the P4.3 decision: either install the privileged `systemd-creds` helper or explicitly reconfirm the file-based `LinuxSecretStore` stand-in with a revisit trigger.
- Stop and uninstall cleanly.
- Confirm the headless artifact has no desktop dependencies.

Windows proof:

- Register a Windows Service that logs on as the installing user's account, not `LocalSystem`.
- Start the agent through the Service Control Manager.
- Start the Paper server through the API/CLI.
- Verify closing CLI/iOS clients does not stop the server.
- Verify lifecycle-owned Java processes are assigned to Job Objects where Phase 4 uses them for cleanup.
- Record a sign-out checkpoint, have Cameron sign out and back in, then verify the service and server survived.
- Stop and uninstall cleanly.
- Confirm the headless artifact uses the console/service shape, not a GUI subsystem dependency.

## Deferred Phase 3 Items Now Load-Bearing

Phase 4 must consume or resolve these Phase 3 deferrals:

| Item | Phase 4 handling |
|---|---|
| Fixed Phase 2 dev token | P4.2/P4.5 replace it with `SecretStore`-backed real credentials before lifecycle mutation. |
| Linux `LinuxSecretStore` stand-in | P4.3 chooses the privileged helper path or explicitly reconfirms the stand-in for this gate. |
| macOS LaunchDaemon login-keychain/TCC unknowns | P4.4 writes executable checks; P4.22 runs them in the real service context. |
| D-024 power management | P4.25 implements and verifies the two host-role policies alongside service lifecycle. |
| D-021 no-GUI-link check | P4.26 gives the packaging check a concrete home. |
| Operation journal/exclusivity substrate | P4.17 uses it for real lifecycle operations and restart reconciliation. |

## Exit Evidence

The phase gate holds only when all of this evidence exists:

- Contract checks pass for the Phase 4 route set.
- A live imported Paper server can be imported, selected, started, observed, commanded, stopped, restarted, and stopped again through public API/CLI paths.
- The existing iOS app drives the same lifecycle path.
- macOS LaunchDaemon, Linux systemd, and Windows Service runs each prove client closure does not stop the server.
- Windows sign-out survival is recorded.
- Power-policy checks for both host roles pass.
- Headless package no-GUI-link checks pass.
- `cargo fmt`, `cargo clippy`, and `cargo nextest run` are clean.

Anything less than this is partial progress, not a Phase 4 gate pass.
