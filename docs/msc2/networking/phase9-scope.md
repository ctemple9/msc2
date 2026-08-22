# Phase 9 — networking and helpers scope

**Status:** P9.1 scope record; implementation has not begun.

Phase 9 makes player-connection tools and the existing named-token model
durable. It does not make the MSC management API public. The separation is
non-negotiable: a Playit tunnel, a resource-pack HTTP server, DuckDNS display
name, Geyser/Floodgate, and Xbox Broadcast carry Minecraft-player traffic;
administration remains loopback by default, or an explicitly chosen LAN or
Tailscale connection with token authentication.

## Source inventory and disposition

The following is the Phase 9 source-level inventory. “Port” means behavior
belongs in the Rust agent; “client” means a client renders or presents the
agent-owned result; “new” has no MSC 1 cross-platform oracle and must be
specified and tested in MSC 2.

| Area | MSC 1 implementation, route/DTO, and tests read | Disposition and proven behavior |
| --- | --- | --- |
| Playit | `PlayitAgentManager.swift`, `PlayitBinaryManager.swift`, `AppViewModel+Playit.swift`, `AppViewModel+APIWiring.swift`; `/playit`, `/playit/start`, `/playit/stop`; `PlayitStatusResponseDTO`, `PlayitActionResultDTO`; iOS `PlayitView.swift`, `RemoteAPIClient.swift` | **Port.** Configure, start, stop, and report the Minecraft tunnel; distinguish disabled, missing secret, already-running, and not-running states. Tunnel addresses are player connection details, never an administration endpoint. The binary download/install shape is also agent work. Secret storage, cancellation, bounded output, restart recovery, and non-macOS process supervision are **new**. |
| DuckDNS and connection display | `AppConfig.swift:557`, `AppViewModel+ServerSettings.swift:310`, `AppViewModel+APIWiring.swift`, `AppViewModel+HealthCards.swift:1022`; `/duckdns`, `/connectivity`; `DuckDNS*DTO`, `ConnectivityResponseDTO`; iOS `DashboardViewModel.swift`, `SettingsView.swift` | **Port, label-only.** `duckdnsHostname` is trimmed ordinary configuration and re-syncs Xbox Broadcast. MSC 1 has no DuckDNS token, no stored update credential, and no `duckdns.org/update` request. `connectivitySnapshot` prefers Playit, then DuckDNS, then public IP and reports port/tunnel/broadcast state. A DuckDNS updater is a possible future extension, not Phase 9 work. `GET /v1/connectivity` already exists and is the route to implement, not a new parallel route. |
| Port diagnostics | `AppViewModel+HealthCards.swift:537-818,1022`; `OverviewConnectionHelpers.swift`; `ConnectivityResponseDTO`; iOS `HealthView.swift` | **Port.** MSC 1 checks whether a server has ever run, performs a local TCP/UDP best-effort probe, and queries mcsrvstat.us with a ten-second request timeout. A provider/parse failure remains distinct from an unreachable Minecraft port. Cross-platform socket behavior, cancellation, retry limits, and provider outage evidence are **new**. |
| Resource packs | `ResourcePackManager.swift`, `ResourcePackHostServer.swift`, `AppViewModel+ResourcePacks.swift`, `AppViewModel+APIWiringSettings.swift`; `/resourcepacks*`; `ResourcePack*DTO`; iOS `ResourcePacksView.swift` | **Port.** Java packs must be ZIPs; activation calculates SHA-1, percent-encodes the filename, serves the approved active pack, and writes the Minecraft URL/hash/require settings. Removing an active pack stops its host; reload/start self-heals hosting. Geyser packs remain separately managed. Bounded serving, path isolation, transactional replacement, cancellation, and cross-platform listener ownership are **new**. |
| Geyser and Floodgate | `GeyserConfigManager.swift`, `AppViewModel+ServerInfo.swift`, `AppViewModel+ServerSettings.swift`, `AppViewModel+APIWiring.swift`, `AppViewModel+Templates.swift`; `/config/geyser`, `/components`; `GeyserConfig*DTO`; iOS `ComponentsView.swift`, `SettingsView.swift` | **Port.** Existing installation detection is filename-based and Geyser configuration is validated/preserved. Phase 7 only copied already-archived templates; downloading, updates, compatibility checks, safe configuration mutation, and Bedrock-facing reporting remain Phase 9 work. |
| Xbox Broadcast | `XboxBroadcastProcessManager.swift`, `XboxBroadcastDownloader.swift`, `BroadcastConfigManager.swift`, `AppViewModel+Broadcasting.swift`, `AppViewModel+XboxBroadcastDownload.swift`, `AppViewModel+OutputHandling.swift`; `/broadcast/*`; broadcast DTOs; iOS `XboxAuthBanner.swift`, `RemoteAPIClient.swift` | **Port.** Maintain the JAR library, staged download, configuration, account prompt extraction, process lifecycle, autostart, and ready signal. Per-server alternate password is secret-only in MSC 1’s Keychain; it must use `SecretStore`, never normal configuration, logs, API responses, or exports. Supervision, cancellation, restart recovery, and non-macOS support are **new**. |
| Notifications | `AppViewModel+Notifications.swift`, `AppViewModel+OutputHandling.swift`, `AppConfig.swift` (`ServerNotificationPrefs`); iOS client notification presentation | **Split correctly.** MSC 1’s actual events are `serverStarted`, `serverStopped`, `playerJoined`, and `playerLeft`; output parsing emits the player events. Native `UNUserNotificationCenter` delivery is **client** work. Phase 9’s agent emits the events for WebSocket/feed consumers; helper-crash and connectivity-change events are additive. Per-server preferences remain agent configuration. |
| First-run orchestration | `ServerLifecycleManager.swift`, `AppViewModel+ServerControls.swift:893-1030`, `AppViewModel+OutputHandling.swift` | **Port.** After initial creation, MSC 1’s second pass starts transport helpers and holds completion while it awaits their signals: Playit gets about 75 seconds after a secret exists, Broadcast gets about 60 seconds after authentication, and a safety cap bounds the whole workflow. The overlay is client-only; the start/stop sequencing is agent behavior. MSC 2 must express it through the shared operation journal rather than UI timers. |
| Named tokens and revocation | `AppConfig.swift:458-515`, `MSCSettingsView.swift`, `RemoteAPIServer+HTTP.swift`, `RemoteAPIServer+UserRoutes.swift`; `/users`, `/users/update`, `/users/revoke`; user DTOs; iOS `UsersView.swift`, `RemoteAPIClient.swift`; `RemoteAPITestSupport.swift` and DTO contract tests | **Port and harden.** MSC 1 has named token records with label, role, permissions, optional expiry, issue/update/revoke, and dispatcher enforcement. MSC 2’s Phase 4 credential format/registry is the baseline. Phase 9 must wire the frozen CRUD routes, issue the raw secret once, and prove a revoked token fails after a restart using the production `SecretStore` path. Raw-token-at-rest avoidance, durable registry authority, and cross-platform secret backends are MSC 2 requirements. |

The inventory is cross-checked against `docs/msc2/audit/msc2-symbol-ledger.csv`
rows 10, 16, 19, 27–29, 34, 36, 48, 70–76, 92, 94, 96, 98–101,
and 303; the frozen API baseline and current `/v1` contract; and the copied
iOS route consumers. The listed native SwiftUI, Finder, Keychain, QR, and
notification-presentation code is not a Rust-agent port target.

## Decisions carried into implementation

### `duckdns_label_only`

P9.9 stores and validates the user-supplied hostname as non-secret
configuration and uses it for connection display and Xbox Broadcast resolution.
It does not call DuckDNS or store a DuckDNS credential. This preserves the
actual MSC 1 behavior and avoids presenting an unimplemented updater as if it
were working.

### Existing connectivity contract

`GET /v1/connectivity` and `ConnectivityResponseDTO` were frozen in Phase 2.
P9.9 supplies its real body from the one connectivity workflow above and may
extend that schema additively in P9.4. It must not create a duplicate
connectivity route or make a public management probe.

### Secrets and management boundary

Playit secrets, Xbox account passwords/tokens, bearer-token verifier records,
and any future provider credential use `SecretStore`. They are redacted from
status, API responses, logs, audit records, transfer exports, fixture output,
and CLI output (except the deliberate one-time named-token issuance response).

No Phase 9 listener is an alternate management listener. Resource-pack hosting
is restricted to an approved active pack; Playit and player-facing integrations
carry Minecraft traffic only. The management API remains token-protected, with
loopback as its default bind and LAN/Tailscale an explicit administrative
choice.

## Working gate

Phase 9 is ready for review only when one exact candidate proves all of these:

1. Every Phase 9 deliverable in `msc2-port-plan.md` is implemented, or an
   owner-approved deferral says why it is not; no DuckDNS updater is implied.
2. Player and management traffic remain separated: no tunnel or resource-pack
   endpoint can reach the management API, and secrets never appear in a public,
   logged, audited, exported, or ordinary CLI value.
3. Playit, resource-pack hosting, diagnostics, Geyser/Floodgate, and Xbox
   Broadcast have fixture-backed behavior; helper processes are bounded,
   single-owner, cancellable, stop cleanly or forcibly, and recover honestly
   after an agent restart.
4. `GET /v1/connectivity`, all Phase 9 HTTP/WebSocket routes, operations and
   cancellation, capability discovery, and scriptable CLI are reachable. The
   capability matrix records only delivered desktop/web, iOS, and CLI surfaces;
   a blank cell is not acceptable.
5. Named-token create/update/list/revoke use the same durable production
   `SecretStore` path as Phase 4/5. Revocation wins over stale memory and stays
   effective after a process restart.
6. The four baseline server/player events reach clients; first-run creation
   waits on Playit/Broadcast readiness using bounded operation state rather
   than a client-only timer.
7. Synthetic and safe live-provider evidence states success or unavailability
   honestly. Targeted tests and the Phase 9 gate run pass on macOS, Linux, and
   Windows, including the headless no-GUI link check.

## D-012: access posture carried from P9.1

P9.1 does not silently settle D-012. Phase 4’s CLI/iOS bearer path is the
current baseline, but these six connected choices remain unresolved: proving a
local desktop shell, pairing a desktop to a remote host, per-host desktop
credential storage, LAN TLS and certificate trust, whether Tailscale changes
any rule, and browser origin/CSP/CSRF policy.

P9.1 recommended a loopback-default management bind, mandatory bearer tokens
on Tailscale, no Playit management exposure, retention of the per-host
secret-store shape, and deferral of desktop/browser-specific pairing, cookie,
origin, and CSRF mechanics to Phase 11. Cameron approved that posture in P9.3
below: opt-in general-LAN administration is unavailable until its certificate
and trust design exists. This affects only management access, not player
connectivity.

## P9.2 fixture provenance and live evidence

`fixtures/networking/` contains 14 language-neutral cases extracted from the
listed MSC 1 implementation: Playit state/output, the label-only DuckDNS
setting, Java resource-pack URL/SHA-1 behavior, local/public port diagnostics,
Xbox Broadcast prompts/readiness, and Geyser/Floodgate detection/configuration.
`fixtures/helper-lifecycle/` contains 8 cases from the Playit and first-run
orchestration paths. `fixtures/credentials/` contains 8 cases from the named
token route providers.

Two cases have no MSC 1 oracle and say so in their `notes`: an agent restart
must reconcile a helper rather than claim it is running, and revocation must
remain effective after a restart through the production `SecretStore` path.
These are MSC 2 acceptance requirements from the Phase 9 working gate, not
retrospective claims about the macOS app.

Live evidence is deliberately limited to a read-only mcsrvstat.us request in
`evidence/mcsrvstat-us.md`. No configured Playit secret, Xbox account,
resource-pack listener, or disposable Minecraft server was available, and no
stateful third-party operation was attempted. DuckDNS has no MSC 1 update API,
so request/response evidence for one would be fabricated rather than useful.

## D-012: Phase 9 access posture

**Approved by Cameron Temple, 2026-08-22.** Phase 9 keeps the management API
loopback-only by default and permits an explicitly configured Tailscale path
only. It does not bind the management API to a general LAN address and does
not add off-loopback HTTP or TLS certificate provisioning in this phase.
Tailscale membership does not authenticate an administrator: every management
request still requires the existing bearer credential and permission checks.
Playit, resource-pack hosting, and every other player-facing listener remain
incapable of forwarding traffic to the management API.

Phase 9 retains the Phase 4 per-host bearer-credential persistence for the CLI
and iOS, and adds durable named-token administration. It does not implement
remote desktop pairing, desktop-local automatic authorization, browser cookie
issuance, browser origins/CSP, or CSRF. Those browser and desktop mechanisms,
along with any general-LAN TLS/certificate-trust design, are explicitly
deferred to Phase 11. Named tokens remain credentials with roles and
permissions, not per-person identities.

**Testable security invariant.** With no explicit Tailscale configuration, the
management listener accepts connections only on loopback. With Tailscale
enabled, it accepts management requests only from the chosen Tailscale path
and rejects absent, malformed, expired, revoked, or unauthorized bearer
credentials; no general LAN or player-facing endpoint reaches it.
