# Phase 13 — Terminal UI scope and parity ledger

**Status:** owner-reviewed scope for Phase 13 execution, 2026-09-02.

Phase 13 adds a terminal-native interactive client to the existing MSC 2
management API. It does not move management policy into the client, add a
second API, or turn the scriptable CLI into a full-screen application.

## Invocation contract

| Invocation | Dispatch | Output and exit contract |
|---|---|---|
| `msc status`, `msc backup now`, `msc server restart "Paper"`, and every other named command | Existing one-shot CLI runner | Existing human output, JSON output with `--json`, API error encoding, polling, and exit codes remain in force. |
| `msc --json ...` with a named command | Existing one-shot CLI runner | JSON remains machine-readable; it never enters the TUI. |
| `msc --help` or command help | Clap help path | Conventional help text and exit status; no terminal control bytes. |
| Bare `msc` with stdin and stdout attached to TTYs, `TERM` present and not empty or `dumb`, and no `--json` | Interactive TUI entry point | P13.2 owns terminal setup and restoration. The host session is explicit or paired and memory-only. |
| Bare `msc` without both TTY streams, with unusable/missing `TERM`, or with `--json` | Normal usage outcome | Exit 2 and ordinary stderr only; no raw mode, alternate screen, cursor control, or other terminal bytes. |
| `msc serve` and the hidden credential-helper command | Existing service-mode dispatch | Service behavior is unchanged; these are not TUI commands. |

The command-selection seam lives in `crates/msc-agent/src/cli/mod.rs` and is
called by `crates/msc-agent/src/main.rs`. P13.1 only reserves the TUI entry
point; P13.2 supplies the event loop and terminal guard. This keeps a bare
interactive invocation from accidentally putting a terminal into raw mode
before the restoration guarantees exist.

## Client/agent boundary

The TUI is a client of the same authenticated HTTP and WebSocket contract used
by the desktop, web, iOS, and one-shot CLI surfaces.

- The agent remains authoritative for capability advertisement, bearer
  authentication, host scoping, role and permission checks, confirmations,
  lifecycle safety, operation journaling, structured errors, and filesystem
  boundaries.
- The TUI may select a host and server, hold bounded presentation caches, keep
  client-local notes and session history, and initiate requests through
  existing routes. It must not duplicate agent rules or infer availability from
  a visual slot.
- The first connection accepts an explicit bearer token or exchanges the
  existing one-use desktop-pairing code through `POST /v1/auth/desktop-pairings`.
  The resulting host session and credential are memory-only. No remembered host
  profile, plaintext token file, or ordinary configuration secret is added.
- Raw Minecraft console text is a separate boundary from MSC management
  actions. Later console work sends literal text only to `/v1/command`; the TUI
  does not reinterpret Minecraft commands as management commands.
- Remote paths are agent-provided details or bounded API resources. A terminal
  path display is not local Finder access and does not grant arbitrary remote
  filesystem access.

## Tauri-to-TUI parity ledger

The destination column is the exact Phase 13 step that owns the workflow. Every
row must be delivered there or converted into a named, owner-reviewed
exception. “Translated” means that imagery, sheets, and mouse affordances are
re-expressed as keyboard-first terminal controls while the product meaning and
workflow remain.

| Desktop surface | Phase 13 destination | Wide behavior | Medium behavior | Small behavior | Capability/API evidence | Terminal treatment or exception |
|---|---|---|---|---|---|---|
| Shell and server-control rail | P13.2 shell; P13.4 overview | Header identity, left picker/control rail, identity band, seven-tab strip, content, and docked console in that order. | Compact selector and rail can be explicitly collapsed; selected host/server and state remain visible. | One focused view at a time; host/server, section switcher, console, and help stay immediately reachable. | `GET /v1/status`, `GET /v1/servers`, `POST /v1/active-server`, `GET /v1/capabilities`, `GET /v1/me`. | Translated into whitespace, hierarchy, labeled lifecycle state, and keyboard focus. No decorative dashboard cards or rail color bands. |
| Overview, including per-server Notes | P13.4 overview | Connection, Live Stats, Health, Activity, local Notes, and console follow the Tauri reading order. | Rail/dock collapse is explicit; overview sections remain selectable and labeled. | Focused overview subview with an immediate console/activity route. | `/v1/status`, `/v1/performance`, `/v1/health`, `/v1/capabilities`; Notes are keyed by host/server in client-local state and never sent to the agent. | Connection and health imagery become text and labeled status. Notes are a client-local exception to the API boundary, not a server setting. |
| Players | P13.7 Players | Online roster, session summary/log, search/sort/detail, and supported actions remain separate flows. | Roster and detail are selectable list/detail views; session log can collapse. | Focused roster, player detail, or session-log view with back/help keys. | `GET /v1/players`, `/v1/players/profiles`, `/v1/session-log`, `POST /v1/session-log/clear`, player mutation routes, `/v1/capabilities`. | Skin/avatar art is replaced by compact identity text and Java/Bedrock plus online/history labels. No workflow is removed. |
| Worlds and Backups | P13.8 Worlds and Backups | World-slot list → selected world detail/actions → related backup context; active identity leads the flow. | Explicit list/detail focus; backup actions remain reachable without duplicating world settings. | One focused list/detail flow; destructive target, stopped-server requirement, and safety-backup consequence precede actions. | `/v1/worlds*`, `/v1/backups*`, operation routes, `/v1/capabilities`; permission categories `worlds` and `backups` where advertised. | Thumbnails are translated to name/type/status. Import/export uses bounded staging; no local or arbitrary remote path picker. |
| Performance | P13.7 Performance | TPS (1m/5m/15m), players, CPU, memory, world size, uptime, and status are readable in the content region. | Trends become a compact labeled history; current values remain first read. | Focused metric/trend view with no clipped chart. | `GET /v1/performance`, `GET /v1/status`, `/v1/capabilities`. | Terminal trend text or compact bars replace pixel charts; unavailable edition data is explained, never faked. |
| Components | P13.9 Components | Server JAR/version, add-ons, catalog, update, enable/disable/remove, helpers, resource packs, and modpacks remain distinct. | Searchable list/detail with an explicit action menu. | Focused list/detail/action flow with progress and errors reachable. | `/v1/versions*`, `/v1/addons*`, `/v1/catalog/*`, `/v1/resourcepacks*`, `/v1/modpacks*`, `/v1/playit*`, `/v1/capabilities`. | Images, thumbnails, and browse tiles become text metadata. Provider-unavailable and pack-managed states are shown as API responses. |
| Settings, Health, Connectivity, and Access | P13.11 settings/connections | Settings, health/repair, connection instructions, service controls, and access are distinct labeled surfaces. | Each surface can be focused independently; service and connection disclosures remain reachable from the rail. | Focused settings or diagnostic flow with immediate back/help and no compressed form grid. | `/v1/settings`, `/v1/health*`, `/v1/connectivity`, `/v1/playit*`, `/v1/broadcast*`, `/v1/duckdns`, `/v1/allowlist`, `/v1/users`, `/v1/capabilities`, `/v1/me`. | Credentials are write-only sensitive input. Local/public/hidden and Java/Bedrock meanings remain text. Host-provided path/log actions do not expand filesystem authority. |
| Files | P13.12 Files | Server Root → folders/files → selected read-only preview, with server context and path metadata. | Compact browse/detail view with a reported copyable path. | One focused browser/detail view; keyboard back and preview are immediate. | `GET /v1/files` with its scoped path contract, `admin` permission, and `no_active_server` response. | Show in Finder becomes a copyable/reported path. No arbitrary path access or file mutation is added. |
| Fleet / Manage Server, including create and import | P13.10 fleet | Server list has active/lifecycle/type context; create/import uses staged choices; rename, EULA, and delete are explicit. | List/detail flow with active-server changes reflected in the whole shell. | Focused server list or staged wizard; destructive confirmation names the target and consequence. | `GET /v1/servers`, `/v1/servers/create`, `/v1/servers/import`, `/v1/servers/rename`, `/v1/servers/eula`, `/v1/servers/delete`, `/v1/active-server`, `/v1/capabilities`. | Finder-based import is translated to typed host-path text or bounded staged upload. No opaque one-line shortcut replaces the create/import workflow. |
| Server editor: General, Services, Java | P13.10 server editor | General path/RAM/ports/storage/EULA, capability-backed Services, and Java runtime/path/version/arguments are separate subflows. | Selected editor section is explicit; path is text entry, not a fake remote browser. | One focused form/action at a time with clear save/cancel and validation state. | `/v1/servers/directory`, `/v1/servers/size`, `/v1/config/ram`, `/v1/settings`, `/v1/playit*`, `/v1/broadcast*`, `/v1/config/java-runtime`, `/v1/java-runtimes`, `/v1/versions`, `/v1/capabilities`. | macOS window chrome and Finder picker are exceptions translated to terminal-native context/path entry. Agent validation, permissions, and confirmation remain authoritative. |
| Agent and pairing | P13.13 agent/support | Agent install/start/stop/reconnect/repair status and one-use pairing are reachable outside the seven tabs. | Pairing and reconnect are focused support flows without losing current host context. | Short first-session/help path with pairing immediately reachable. | `/v1/auth/pairings`, `/v1/auth/desktop-pairings`, `/v1/me`, `/v1/capabilities`, platform service commands, existing service/pairing modules. | No persisted token/profile. Desktop pairing code exchange is reused; credential remains memory-only. |
| Handbook and router guides | P13.13 Handbook | Served topic catalog, search, related topics, router-guide search/reader, and troubleshooting remain browsable. | Search and selected article/guide are separate focus states. | Focused search → article/guide flow; teaching content remains available. | `/v1/help/catalog`, `/v1/help/{helpId}`, `/v1/guides/router/search`, `/v1/guides/router/{guideId}`, `/v1/guides/router/troubleshooting/analyze`. | Onboarding animation and graphical tour affordances are translated to concise keyboard help; content and routing meaning are retained. |
| MSC Settings and reset | P13.13 client settings/support | Terminal-local preferences/reset are distinct from authenticated host reset and its fresh-pairing consequence. | Reset target and consequence are explicit before confirmation. | Focused reset confirmation with cancel/back immediately available. | Client-local state boundary; `POST /v1/host/reset`, `GET /v1/status`, operations, auth pairing routes, `/v1/me`. | Client-local reset never becomes host reset. Host reset keeps agent refusal, confirmation, operation, and pairing semantics; no plaintext secret store is added. |

## Reference photo index

This is the binding index for later interactive-surface work. Every supplied PNG
under `/Users/camerontemple/Documents/msc2 pictures/` is listed; `.DS_Store`
files are not visual references.

| Reference group | Files |
|---|---|
| Main View | `Main View/mainview.png`; `Main View/sidebarcollapsed.png`; `Main View/consolecollapsed.png` |
| Sidebar | `SIdebar/avatar.png`; `SIdebar/dropdown.png`; `SIdebar/emptyavatar.png`; `SIdebar/howtoconnect.png`; `SIdebar/maintenance.png`; `SIdebar/quickcommands.png`; `SIdebar/services.png`; `SIdebar/sidebarcollapsed.png` |
| Tabs | `Tabs/overview.png`; `Tabs/players.png`; `Tabs/worlds.png`; `Tabs/performance.png`; `Tabs/components.png`; `Tabs/settings.png`; `Tabs/files.png` |
| Edit Server | `Edit Server/manageservers.png`; `Edit Server/editserver.png`; `Edit Server/generaltab2.png`; `Edit Server/general ab.png`; `Edit Server/java.png`; `Edit Server/services.png` |
| Agent | `Agent /Screenshot 2026-09-02 at 4.14.39 AM.png`; `Agent /Screenshot 2026-09-02 at 4.14.41 AM.png` |
| MSC Settings | `MSC Settings/Screenshot 2026-09-02 at 4.15.35 AM.png`; `MSC Settings/Screenshot 2026-09-02 at 4.15.37 AM.png` |
| Server handbook | `Server handbook/Screenshot 2026-09-02 at 4.21.27 AM.png`; `Server handbook/Screenshot 2026-09-02 at 4.21.32 AM.png`; `Server handbook/Screenshot 2026-09-02 at 4.21.37 AM.png`; `Server handbook/Screenshot 2026-09-02 at 4.21.43 AM.png` |

## Test treatment

The command-dispatch seam gets focused behavioral coverage for named-command
preservation, JSON suppression of TUI dispatch, the two-stream TTY requirement,
usable `TERM`, and the placeholder's non-terminal error. Later steps add tests
only for lifecycle restoration, authenticated transport, state selection,
confirmation/safety behavior, reconnect/terminal-state handling, and real
regressions. Static labels, callback wiring, and duplicate assertions of agent
rules do not receive tests.
