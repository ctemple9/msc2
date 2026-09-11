# Phase 14 — operational-refinement source and acceptance matrix

**Step:** P14.2 · **Status:** source map recorded; implementation steps remain
planned · **Date:** 2026-09-11

This matrix is the source map for Phase 14. It identifies the current MSC 2
owner, the MSC 1 oracle where one exists, the API or packaging boundary, the
client surface, and the evidence needed before each behavior can be accepted.
It is deliberately honest about partial and missing implementation: this step
does not make any of the later P14.3–P14.18 changes.

## Evidence classes

| Mark | Meaning |
|---|---|
| **S** | Static inspection of source, generated contract, packaging metadata, or a release artifact. |
| **M** | Live Minecraft verification against a real server/runtime. |
| **O** | Real operating-system installation or service exercise. |
| **C** | Manual walkthrough of a retained client surface: Tauri desktop, desktop browser, or headless CLI. |

An acceptance row may require more than one class. The matrix does not turn a
static inspection into a substitute for a live runtime or OS install.

## Source and boundary matrix

| ID | Reported behavior | Current MSC 2 owner and state | MSC 1 oracle | API / transport boundary | Client surface | Owning implementation step |
|---|---|---|---|---|---|---|
| T1 | Dawn, dusk, and night are semantic **same Minecraft day** actions. | `clients/desktop-web/src/lib/sections/console/model.ts:581-590` exposes the generic `time` command; `crates/msc-domain/src/commands.rs` and `crates/msc-agent/src/routes/commands.rs` forward commands. No relative-time operation exists yet. | `MSCmacOS Swift/QuickCommandsView.swift:10-21, 136-148` maps Dawn/Dusk/Night to literal `1000`, `13000`, and `18000`; `AppViewModel+ServerControls.swift` owns the send path. | Current `POST /v1/command` (`docs/msc2/api-contract/openapi.json`, `/v1/command`) is raw command transport. P14.3 must add the semantic operation without changing raw-command meaning. | Shared Svelte command picker/sidebar; desktop browser; Tauri desktop; CLI command path. | P14.3 domain/API operation, then P14.4 client actions. |
| T2 | Exact day changes are explicit; numeric `time set` remains absolute. | No separate day-change operation or UI boundary exists; the command picker currently presents `time set`, `time add`, and `time query` as one generic command. | MSC 1's `MinecraftCommandRegistry.swift` exposes `time set`, `time add`, and `time query`; `QuickCommandsView.swift` uses absolute literals for presets. | Same `POST /v1/command` route today; the future API must make semantic presets and absolute raw commands distinguishable. | Desktop/browser command picker, Tauri shell, and CLI. | P14.3 contract and P14.4 presentation. |
| T3 | The same time behavior must cover Vanilla, Paper, Purpur, Fabric, Forge, NeoForge, and Bedrock. | `CapabilitiesDTO.serverTypes` advertises five named Java flavor booleans (`vanilla`, `paper`, `fabric`, `forge`, `neoforge`) plus Bedrock; Purpur is not a separate advertised boolean and must be covered as an explicit Paper-family acceptance case unless the contract expands. Selected-server Bedrock runtime state is exposed, but no time capability exists. | `JavaServerFlavor.swift` and the MSC 1 command registry cover the Java-family command surface; MSC 1's Bedrock command routing is in `AppViewModel+ServerControls.swift`. MSC 1 does not provide a same-day semantic contract. | `GET /v1/capabilities`, selected-server capability routes, and shared `POST /v1/command`; Bedrock unsupported/runtime-unavailable errors use `ErrorDTO`. | All retained clients must consume capability discovery rather than assume Java parity. | P14.2 coverage map; P14.3 runtime mapping; P14.4 client degradation. |
| C1 | Automatic TPS, player-count, Spark, and related polling traffic must not displace human console history. | `crates/msc-infrastructure/src/console_buffer.rs:77-95` classifies lines but stores every line in the single 5,000-line ring; `crates/msc-agent/src/ws/console.rs:50-71` serves that same ring to HTTP and WebSocket clients. The frontend hides auto lines only after receiving them (`clients/desktop-web/src/lib/sections/console/model.ts:335-350`). | `ConsoleManager.swift`, `AppViewModel+OutputHandling.swift`, `TpsLineParser.swift`, and `TpsMonitoringTests.swift` provide the console/metric oracle and distinguish metric parsing from presentation. | `GET /v1/console/tail`, `/v1/console/stream`, and `ConsoleLineDTO.auto`; `docs/msc2/api-contract/websocket-v1.json` defines the shared 5,000-line ring and 200-line backfill. | Tauri desktop, desktop browser, and CLI console/history consumers; all server runtimes. | P14.5 defines retention; P14.6 moves classification before retention; P14.7 aligns controls. |
| C2 | Human console history remains bounded and reconnectable while any optional diagnostic stream is separately bounded. | Agent and client bounds exist, but there is no separate diagnostic ring: `ConsoleState` broadcasts the same stored lines, and `HostStore` keeps the latest 200 per host. | `ConsoleManager.swift` and `RemoteAPIServer+WebSocket.swift` cover MSC 1's bounded history/reconnect behavior; MSC 1 has no separate automatic-diagnostic contract. | WebSocket `console` channel and HTTP tail remain the public history boundary; any diagnostic stream must be additive and separately bounded. | Same three retained clients; reconnect behavior must be checked for each. | P14.5–P14.7. |
| H1 | Headless installation is first-class on macOS, Windows, and Linux, with `msc` discoverable on PATH and management service port `48001`. | Linux has `packaging/linux/install.sh` and `uninstall.sh`, installing `/usr/lib/msc2/msc` and a systemd service, but neither owns a PATH entry. The macOS/Windows headless build scripts create archives only; they do not install PATH entries or services. `msc serve --bind 127.0.0.1:48001` is documented in `msc2-engineering.md`. | MSC 1 has `HeadlessScriptGenerator.swift`, which generates server launch scripts, but no tri-platform MSC agent, CLI, or service installation equivalent. | Packaging/service boundary, not an HTTP route. The agent service defaults to `127.0.0.1:48001`; the CLI binary is `msc` / `msc.exe`. | Headless CLI on all three OSes; Tauri desktop may be absent; served browser connects to an installed agent. | P14.8 defines the contract; P14.9 Linux; P14.10 macOS/Windows. |
| H2 | Install, upgrade, and uninstall must preserve only MSC-owned PATH/service state. | Linux uninstall removes explicit MSC systemd units and `/usr/lib/msc2/msc` but has no PATH ownership model. macOS and Windows have no corresponding uninstall/PATH implementation in the current release scripts. | No MSC 1 equivalent; this is new MSC 2 platform behavior. | OS packaging and service boundary; no remote API may perform these operations on another host. | Headless CLI and local Tauri service-management surface; browser can manage an already-installed agent but cannot install it. | P14.8–P14.10, then P14.18 OS acceptance. |
| R1 | A saved remote host keeps a stable identity while LAN, Tailscale, or DNS addresses change. | `HostRecord` is only `{ id, label, baseUrl }` (`clients/desktop-web/src/lib/hosts/types.ts:7-11`); `saved.ts` stores it in localStorage. `HostStore` keys caches by `id`, but there is no editable route set or SSH metadata. | MSC 1 has no multi-host desktop profile equivalent. Its remote API and iOS pairing client are historical auth/transport evidence only. | Existing `/v1/auth/desktop-pairings` exchanges a code for a host-scoped credential; Tauri stores the credential natively in `clients/desktop-web/src-tauri/src/lib.rs`. No SSH/tunnel API exists. | Tauri desktop supports saved remote metadata; served browser is origin-local; CLI uses explicit host configuration. | P14.11 profile model and migration. |
| R2 | Direct LAN/Tailscale routing may fall back to an app-managed SSH forward, with remote `48001` and editable local `48002` example. | `AgentSetupSection.svelte:61-63` displays a manual SSH command, but `App.svelte:205-228` currently requires a user-supplied base URL and pairing code. There is no native tunnel/session manager. | MSC 1 has no managed SSH tunnel or cross-machine host switcher. | Direct authenticated HTTP/WebSocket to the agent, or future desktop-owned SSH transport; transport must not bypass bearer/session authorization. | Tauri desktop owns the tunnel; desktop browser can use a direct reachable origin; CLI remains direct/API-based. Tailscale is optional, not required. | P14.12 tunnel capability, P14.13 wizard, P14.15 route selection. |
| R3 | Remote desktop pairing should be bootstrapped through the authenticated SSH session, with manual recovery available. | Current flow calls `/v1/auth/desktop-pairings` with a user-entered code through the Tauri bridge; no remote command execution or one-use bootstrap orchestration exists. | MSC 1 has one-use pairing and remote API auth in `RemoteAPIServer.swift`, `RemoteAPIServer+HTTP.swift`, `RemoteAPIServerDTOs.swift`, and the former iOS pairing flow. It does not have SSH bootstrap. | `/v1/auth/pairings` creates a challenge; `/v1/auth/desktop-pairings` exchanges it; native secure store owns the bearer credential. No arbitrary shell or service-install route is allowed. | Tauri desktop normal path; desktop browser keeps its existing browser-session path; CLI remains a separate retained client. | P14.13 wizard and P14.14 automated pairing. |
| R4 | Remote connection errors, host-key identity, credential scope, and no-cloud/no-required-Tailscale boundary must be explicit. | Current Tauri auth validates the stored credential's origin and host ID, but no host-key store, tunnel process state, route fallback, or managed SSH error vocabulary exists. | MSC 1 has network safety, token/permission, and Tailscale guidance, but no SSH host-key/session lifecycle. | Existing authenticated API and per-host credential model remain authoritative; MSC operates no relay and cannot install/start/stop/replace/uninstall the host service remotely. | Tauri desktop connection manager; browser/CLI direct routes where available. | P14.15–P14.17, then P14.18 acceptance. |

## Runtime and platform acceptance matrix

The Java rows are separate even though they share command syntax: the matrix
must show evidence for every supported flavor rather than treating “Java” as a
single runtime. Bedrock rows remain separate because D-022 makes its backend
and host-OS constraints explicit.

| Runtime / surface | Time semantics | Console retention | Headless install / PATH | Remote connection | Evidence required |
|---|---|---|---|---|---|
| Java Vanilla | Same-day preset, explicit day, raw absolute command | Polling classification, human history, reconnect | Managed by host OS row below | Via Tauri/browser/CLI transport | **S+M+C** live server plus client walkthrough |
| Java Paper | Same as Vanilla; include Paper TPS output | Include Paper TPS and player list polling | Managed by host OS row below | Same | **S+M+C** |
| Java Purpur | Same as Paper; include Purpur-specific metric wording | Include Purpur output | Managed by host OS row below | Same | **S+M+C** |
| Java Fabric | Same-day operation and capability response | Include Fabric/Spark output where installed | Managed by host OS row below | Same | **S+M+C** |
| Java Forge | Same-day operation and capability response | Include Forge TPS output and multiline responses | Managed by host OS row below | Same | **S+M+C** |
| Java NeoForge | Same-day operation and capability response | Include NeoForge TPS output and multiline responses | Managed by host OS row below | Same | **S+M+C** |
| Bedrock — native Linux | Bedrock command mapping and unsupported-capability explanation | Bedrock tick/player polling and reconnect | Linux systemd package and PATH | Direct or Tauri-managed tunnel | **S+M+O+C** on a supported Linux host |
| Bedrock — native Windows | Bedrock command mapping and unsupported-capability explanation | Bedrock tick/player polling and reconnect | Windows Service package and PATH | Direct or Tauri-managed tunnel | **S+M+O+C** on native Windows |
| Bedrock — macOS Intel VZ sidecar | Sidecar-backed command mapping or clear unavailable state | Sidecar console/metrics and reconnect | macOS LaunchDaemon/headless package and PATH; include sidecar | Direct or Tauri-managed tunnel | **S+M+O+C** on Intel macOS |
| Tauri desktop — macOS | Shared Svelte operation and capability handling | Shared console filter/history behavior | Local service controls remain native and local-only | Direct LAN/Tailscale, then managed SSH | **S+O+C** desktop and remote-host walkthrough |
| Tauri desktop — Windows | Same shared frontend contract | Same | Native Windows service/CLI PATH | Direct LAN/Tailscale, then managed SSH | **S+O+C** native Windows walkthrough |
| Tauri desktop — Linux | Same shared frontend contract | Same | Linux systemd/CLI PATH | Direct LAN/Tailscale, then managed SSH | **S+O+C** minimal-Linux/Xubuntu walkthrough |
| Served desktop browser | Uses API semantic operation; no native assumptions | Receives filtered human history and reconnects | Browser does not install; it manages an existing agent | Direct reachable origin or user-provided route; no native SSH child | **S+C** desktop-browser walkthrough |
| Headless CLI — macOS, Windows, Linux | Raw commands remain available; semantic operation must be scriptable once P14.3 lands | `console tail`/stream behavior must preserve human history | `msc`/`msc.exe` is discoverable after install and upgrade | Direct authenticated API; no required Tailscale | **S+O+C** per OS plus CLI walkthrough |

## Evidence ownership and stop conditions

1. **P14.3–P14.4 own T1–T3.** No client shortcut is accepted by copying the
   old literal values; the live matrix must record the runtime flavor and the
   observed current-day result.
2. **P14.5–P14.7 own C1–C2.** Static evidence may prove classification and
   bounds, but only a live server plus reconnect can show that human output was
   not evicted by polling.
3. **P14.8–P14.10 own H1–H2.** A build archive is not an installation result;
   each OS row needs a real install, PATH lookup, upgrade, and owned uninstall
   observation.
4. **P14.11–P14.17 own R1–R4.** The existing manual pairing flow is a fallback,
   not evidence that managed SSH or address repair exists. A remote acceptance
   run must show both a direct route and the managed tunnel path without
   requiring a cloud relay or a manually opened terminal.
5. **P14.18 records the cross-platform evidence; P14.19 reviews the phase gate.**
   Until then, this matrix is the source map and not a claim that the phase is
   complete.
