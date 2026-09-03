# MSC 2 — Rolling Plan

> ## STATUS: Phase 12 (client redesign) is complete and archived. Phase 13 (Terminal UI, deferred from v1) is next.
> **Next move:** Phase 13 is not started. Phase 11 and Phase 12 are complete, with their full records in `rolling-plan-archive.md`.

**Previous phases (Setup through Phase 12) and their amendments have moved to `rolling-plan-archive.md`** to keep this file small. That archive is historical only — current status and active work stay here.

---

## How this document works

This is the **working state** of the build. The vision documents say where we're going; the port plan says in what order; this file says **where we actually are right now**.

Phases are fixed and come from `msc2-port-plan.md`. **Steps are written one phase at a time**, as we reach each phase — not up front. Phase 13 is next and has not started.

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
| 13 | Terminal UI (deferred from v1) | not started |

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

---

## Phase 13 — Terminal UI

**Entry gate.** Phase 12's redesign gate is complete. Before execution begins,
the authenticated HTTP and WebSocket contract must be stable enough that the
TUI is not repeatedly rebuilt for route or DTO churn. Phase 13 does not start
by changing the agent's management semantics: capability checks, permission
checks, confirmations, operation journaling, structured errors, host scoping,
and bearer authentication stay with the agent. Any API correction discovered
while implementing a client must be additive or an already-documented contract
repair, and must be recorded before a TUI screen relies on it.

### Surface boundary

| Surface | Responsibility | Must not do |
|---|---|---|
| **Scriptable CLI** | Conventional one-shot commands such as `msc status`, `msc backup now`, `msc server restart "Paper"`, and `msc --json ...`; stable stdout/stderr, exit codes, pipes, and automation. | Enter raw mode, alternate-screen mode, or write TUI output. |
| **Interactive TUI** | A persistent, full-screen terminal-native MSC client opened by bare `msc` only in an interactive terminal; keyboard-first presentation and request initiation through the same API. | Own server state, bypass authentication/capabilities/permissions/confirmations, or reinterpret raw Minecraft commands as MSC management commands. |
| **Shared client infrastructure** | Authenticated HTTP and WebSocket transport, agent error decoding, capability discovery, host/session state, reconnection, and bounded local caches. | Store bearer tokens in ordinary plaintext configuration or create a second management API. |

**TTY contract.** Bare `msc` opens the TUI only when stdin and stdout are TTYs,
`TERM` is usable, and `--json` is absent. A non-TTY or JSON bare invocation
returns the normal command-line usage outcome without terminal control bytes;
every named command retains its current behavior. The current CLI has only
`--host`/`--port`/`--base-url` plus `--token` or environment-token resolution,
not a secure remembered-host store. Phase 13 therefore begins with explicit or
in-memory host sessions only. It may not add remembered profiles unless later
evidence demonstrates a need and they can use the established secret-storage
boundary rather than plaintext config.

**Terminal presentation and responsiveness.** This is a terminal application,
not a browser dashboard rendered in cells: monospace hierarchy, restrained ANSI
color, whitespace before boxes, one meaningful focus target, and status color
only for a labeled state. The anti-slop law applies in spirit: clear first,
second, and third read; no decorative panels, rails, glow, gradients, or
meaningless dots. The layout contract is wide (120+ columns and 36+ rows:
server controls/sidebar, tab content, and docked live console), medium (80–119
columns or 24–35 rows: compact selector/sidebar and a collapsible console),
and small (under 80 columns or 24 rows: one focused view at a time with a
dedicated, immediately reachable console/activity view). No size may overflow
or require a graphical desktop.

**Tauri-layout fidelity.** At wide size, the TUI's reference is the established
MSC Tauri server window, translated into terminal-native cells rather than
copied as a browser-like dashboard. Its first screen must preserve this reading
order: (1) the selected host/server and runtime-state identity in the header;
(2) a left server-controls rail with picker, lifecycle controls, and grouped
actions; (3) the server identity band; (4) a horizontal server-section tab
strip in the same information family; (5) Overview's Connection plus Live
Stats, then Server Health, then Activity; and (6) a persistent bottom console
reachable from every section. Terminal controls may replace graphical
controls, imagery, and pixel-level styling, but may not replace that
information architecture with a generic terminal dashboard. Cameron reviews
wide, medium, and small reference renders against the Tauri window and the
anti-slop checklist before the terminal shell is accepted.

### P13.1 — Define the TUI boundary and preserve the command-line contract
**Status:** awaiting verification
**Files:** `docs/msc2/terminal-ui/phase13-scope.md`, `docs/msc2/client-capability-matrix.csv`, `crates/msc-agent/src/main.rs`, `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/tests/tui_contract.rs`
**What:** Record the accepted invocation matrix, client/agent boundary, and a Tauri-to-TUI parity ledger before drawing a screen. The ledger must name every desktop surface represented by the supplied references — shell/control rail; Overview including Notes; Players; Worlds and Backups; Performance; Components; Settings/Health/Connectivity/Access; Files; fleet/manage-server and its create/import flow; server editor General/Services/Java; agent/pairing; Handbook/router guides; and MSC Settings/reset — and give each one (a) its exact Phase 13 destination, (b) its required wide/medium/small behavior, (c) the capability/API evidence, and (d) either a terminal treatment or a deliberately recorded, owner-reviewed terminal exception. Images, macOS window chrome, Finder reveal, and avatar/skin/thumbnail art may be translated or excepted; no workflow may disappear merely because it was presented in a desktop sheet. Add a distinct TUI capability-matrix column so an implemented one-shot CLI command is never mistaken for an implemented screen. Implement and test only the command-dispatch seam: named commands, `--json`, help, and non-TTY use remain conventional; bare interactive `msc` selects the TUI. A first interactive connection can use an explicit bearer token or exchange the existing one-use desktop-pairing code, but its resulting host session and credential exist only in memory; do not create a plaintext profile or token store. Set the test treatment here as well: tests cover lifecycle, transport, state selection, confirmations, and regressions with real behavioral risk — not static labels, callback wiring, or a second assertion of an already-tested agent rule.
**Visual reference:** the ledger indexes every image under `/Users/camerontemple/Documents/msc2 pictures/`; it is the binding photo index for all later interactive-surface steps.
**Verify:** `cargo nextest run -p msc-agent --test tui_contract`
**Commit:** P13.1: define tui invocation contract
**Batch:** solo

### P13.2 — Establish terminal lifecycle, responsive layout, and deterministic rendering
**Status:** awaiting verification
**Files:** `crates/msc-agent/Cargo.toml`, `crates/msc-agent/src/cli/tui/mod.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/src/cli/tui/layout.rs`, `crates/msc-agent/src/cli/tui/render.rs`, `crates/msc-agent/tests/tui_terminal_lifecycle.rs`
**What:** Add the approved `ratatui` and `crossterm` foundation and a terminal guard that always restores raw mode, cursor, and alternate screen after normal exit, error, resize, or panic. Build the event loop, resize handling, focus order, and the wide/medium/small shell before feature screens. Wide must visibly preserve the desktop's app/context header, left controls rail, server identity band, seven-tab strip, content region, and reserved bottom console dock. Medium must make the rail and dock explicitly collapsible rather than silently dropping them; small must present one focused surface at a time with the current host/server, section switcher, console, and help immediately reachable. Use `ratatui`'s test backend for deterministic structural renders at all three sizes, including visible focus and absence of clipping. The shell must be terminal-native and quiet — hierarchy and whitespace, not a grid of generic cards or decorative ANSI color.
**Visual reference:** `/Users/camerontemple/Documents/msc2 pictures/Main View/mainview.png`; `/Users/camerontemple/Documents/msc2 pictures/Main View/sidebarcollapsed.png`; `/Users/camerontemple/Documents/msc2 pictures/Main View/consolecollapsed.png`.
**Verify:** `cargo nextest run -p msc-agent --test tui_terminal_lifecycle`
**Commit:** P13.2: establish tui terminal lifecycle
**Batch:** solo

### P13.3 — Extract only the shared authenticated transport the TUI needs
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/tui/transport.rs`, `crates/msc-agent/src/cli/tui/session.rs`, `crates/msc-agent/tests/tui_transport.rs`
**What:** Move the existing CLI's HTTP request, bearer-auth, API-error, and selected-host primitives behind a shared client seam only where both one-shot commands and the TUI need them. Add authenticated WebSocket connection/reconnection support for the already-defined console, operation-progress, and notification paths, with bounded exponential backoff and a re-fetch after reconnect where the contract requires it. Preserve existing one-shot JSON output, polling, exit-code, and non-TTY behavior exactly; do not add a new API, local filesystem access to a remote host, or credential persistence.
**Verify:** `cargo nextest run -p msc-agent --test tui_transport`
**Commit:** P13.3: share tui api transport
**Batch:** solo

### P13.4 — Deliver the host/server and overview vertical slice
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/src/cli/tui/overview.rs`, `crates/msc-agent/src/cli/tui/render.rs`, `crates/msc-agent/tests/tui_overview.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Deliver the actual first TUI screen, not a generic status dashboard: always-visible selected host and server; keyboard server selection and in-memory host switching; lifecycle controls that reflect the real state; and the rail's Services, How to Connect, Maintenance, and Quick Commands entry points. At wide size establish the desktop reading order exactly: left controls rail, server identity band (name, edition/flavor, path, state), seven section tabs, Connection Information plus Live Stats, Server Health, Activity, per-server local Notes, and docked console. Connection information must distinguish Java/Bedrock and local/public/hidden states; Health details, player state, active world, chat, and server-local notes must retain their meaning when the terminal cannot show the original imagery. The Notes treatment remains client-local and keyed by host/server, never sent to the agent and never used for credentials. A terminal path/details action is the maintenance equivalent of desktop Finder/logs actions; it must not claim arbitrary access to a remote host. Tab availability must come from the agent advertisement and token permissions, not a hardcoded product promise. Keep the live console reachable from every layout and use focused, labeled status rather than decorative color.
**Visual reference:** `/Users/camerontemple/Documents/msc2 pictures/Tabs/overview.png`; `/Users/camerontemple/Documents/msc2 pictures/Main View/mainview.png`; `/Users/camerontemple/Documents/msc2 pictures/SIdebar/services.png`; `/Users/camerontemple/Documents/msc2 pictures/SIdebar/howtoconnect.png`; `/Users/camerontemple/Documents/msc2 pictures/SIdebar/maintenance.png`.
**Verify:** `cargo nextest run -p msc-agent --test tui_overview`
**Commit:** P13.4: add tui overview slice
**Batch:** solo

### P13.5 — Deliver the live console and raw-command vertical slice
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/tui/console.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_console.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Consume the console history-plus-live WebSocket stream with reconnect and bounded local scrollback, falling back to the documented tail route only for recovery. Match the dock's actual working vocabulary: collapse/expand, search, follow/pause, copy-friendly selection, clear-local-history, and the All/Server/Plugins/Warnings/Controller/Commands/custom-filter distinction where that source metadata exists. Supply the command history and an explicit action palette; its Quick Commands entries reproduce the rail's time, weather, difficulty, gamemode, whitelist, save-all, and reload actions through their real raw-command or agent-backed boundary. Make the input boundary unmistakable: `>` sends literal raw Minecraft console text only to `/v1/command`; a separate palette/keybinding layer invokes MSC management actions. Never invent Minecraft command completion the API does not expose.
**Visual reference:** `/Users/camerontemple/Documents/msc2 pictures/Main View/mainview.png`; `/Users/camerontemple/Documents/msc2 pictures/Main View/consolecollapsed.png`; `/Users/camerontemple/Documents/msc2 pictures/SIdebar/quickcommands.png`.
**Verify:** `cargo nextest run -p msc-agent --test tui_console`
**Commit:** P13.5: add tui live console
**Batch:** solo

### P13.6 — Deliver operation progress, notifications, and confirmation behavior
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/tui/activity.rs`, `crates/msc-agent/src/cli/tui/confirm.rs`, `crates/msc-agent/src/ws/notifications.rs`, `crates/msc-agent/tests/tui_activity.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Show bounded current-session operations and notifications as a focused activity surface that can be opened without losing the current tab or console. Subscribe to each operation's existing progress stream, treat its documented terminal close as normal, and resync an operation with HTTP after reconnect. Verify the notification stream has real existing agent producers; if it is only an empty mounted stream, connect those producers to this already-specified channel rather than inventing a TUI-only feed, route, or event shape. Make destructive or disruptive requests visibly name the selected host, server, affected world/backup/component where applicable, and consequence before the agent's existing acknowledgement/confirmation response is dispatched; cancellation remains the agent's cooperative operation API. Use a modal/focused terminal flow, not a desktop-style sheet imitation.
**Visual reference:** `/Users/camerontemple/Documents/msc2 pictures/Main View/mainview.png` for the persistent shell context; terminal activity and confirmation flows are the explicit modal/focused equivalent of the desktop sheets in the later fleet, world, and settings references.
**Verify:** `cargo nextest run -p msc-agent --test tui_activity`
**Commit:** P13.6: add tui activity streams
**Batch:** solo

### P13.7 — Deliver capability-backed Players and Performance sections
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/tui/players.rs`, `crates/msc-agent/src/cli/tui/performance.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_players_performance.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Add keyboard-first Players and Performance tabs using only implemented, capability-advertised API data. Players preserves the desktop's Online Now/session summary, session-log filtering and clear-local action, player-data search/sort/detail flow, and supported profile actions; compact player identity and text replace skin art without losing Java/Bedrock or online/history distinctions. Performance preserves live TPS (1m/5m/15m), players, CPU, memory, world-size, uptime, and status meaning, with readable terminal history/trend treatment rather than pretending a terminal can reproduce browser charts pixel-for-pixel. Unavailable edition-specific data is explained plainly rather than represented by fake empty widgets. Keep player mutations behind permission checks and the shared confirmation surface.
**Visual reference:** `/Users/camerontemple/Documents/msc2 pictures/Tabs/players.png`; `/Users/camerontemple/Documents/msc2 pictures/Tabs/performance.png`.
**Verify:** `cargo nextest run -p msc-agent --test tui_players_performance`
**Commit:** P13.7: add tui players and performance
**Batch:** solo

### P13.8 — Deliver the Worlds and Backups vertical slice
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/tui/worlds.rs`, `crates/msc-agent/src/cli/tui/backups.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_worlds_backups.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Add the capability-filtered world-slot and backup workflows, including create/import, rename, duplicate, save-current, activate, copy/replace-active, export, conversion, Bedrock repair where advertised, manual backup, schedule/retention, restore, and delete. Preserve the desktop's selected-slot relationship: a list of world slots with active identity leads to its detail/actions and then its backup context — it is not two unrelated generic tables. Put active-world identity, backup verification state, required stopped-server/safety-backup implications, destructive target, and confirmation ahead of secondary metadata; preserve server-owned versus world-owned settings boundaries instead of presenting duplicate editors. World thumbnails become name/type/status treatment unless a terminal-specific rendering option is deliberately approved in the ledger. On narrow terminals, use focused list/detail flows rather than a compressed table.
**Visual reference:** `/Users/camerontemple/Documents/msc2 pictures/Tabs/worlds.png`; `/Users/camerontemple/Documents/msc2 pictures/Tabs/overview.png` (active-world and Backup entry point).
**Verify:** `cargo nextest run -p msc-agent --test tui_worlds_backups`
**Commit:** P13.8: add tui worlds and backups
**Batch:** solo

### P13.9 — Deliver the Components section
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/tui/components.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_components.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Add a focused Components tab that preserves the desktop's distinct server-JAR/version, installed add-on, catalog/browse, update, enable/disable, remove, reveal/detail, cross-play/helper, resource-pack, and modpack state flows wherever the active host advertises them. Show operation progress and pack-managed or provider-unavailable responses as the API reports them; never present a control merely because a visual slot exists. Use searchable lists, selected-item detail, and explicit action menus rather than terminal dashboard tiles, and route every mutation through the shared confirmation/error path.
**Visual reference:** `/Users/camerontemple/Documents/msc2 pictures/Tabs/components.png`.
**Verify:** `cargo nextest run -p msc-agent --test tui_components`
**Commit:** P13.9: add tui components
**Batch:** solo

### P13.10 — Deliver fleet management and the server-editor workflow
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/tui/fleet.rs`, `crates/msc-agent/src/cli/tui/server_editor.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_fleet.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Make the rail's Manage action a complete terminal fleet flow: list servers with active/lifecycle/type context; set active; create or import through the desktop flow's explicit staged choices rather than an opaque one-line command; rename; accept EULA; and delete through the shared confirmation surface. Provide the server editor as a focused General/Services/Java subflow: display name, server directory/path semantics, RAM, ports, storage size, EULA and deletion boundary; capability-backed Playit/Xbox service state; Java detect/path/version/arguments actions. A terminal may request a host-side path as text but must not pretend it can browse a remote host or reuse the local desktop's Finder picker. The selected server must update the entire shell consistently after every fleet action.
**Visual reference:** `/Users/camerontemple/Documents/msc2 pictures/Edit Server/manageservers.png`; `/Users/camerontemple/Documents/msc2 pictures/Edit Server/editserver.png`; `/Users/camerontemple/Documents/msc2 pictures/Edit Server/generaltab2.png`; `/Users/camerontemple/Documents/msc2 pictures/Edit Server/java.png`; `/Users/camerontemple/Documents/msc2 pictures/Edit Server/services.png`.
**Verify:** `cargo nextest run -p msc-agent --test tui_fleet`
**Commit:** P13.10: add tui fleet and server editor
**Batch:** solo

### P13.11 — Deliver Settings, Connections, Health, and Access sections
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/tui/settings.rs`, `crates/msc-agent/src/cli/tui/connections.rs`, `crates/msc-agent/src/cli/tui/health.rs`, `crates/msc-agent/src/cli/tui/access.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_settings_connections.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Add the desktop Settings tab's schema-driven setting groups (including editable booleans, numbers, and values with supplied help), plus distinct Health/repair, connection, service, and player-access flows rather than hiding them behind raw command entry. Connection treatment retains local/public/hidden visibility, Java/Bedrock/console join instructions, reachability diagnosis, and supported Playit, Xbox Broadcast, and DuckDNS controls. The rail's Services and How to Connect disclosures lead to the same state, while Maintenance reports safe host-provided paths/log access without expanding filesystem authority. Render API-supplied help, timing, capability, and confirmation information rather than duplicating policy in the TUI. Treat credentials as write-only sensitive input, do not echo them in history or logs, and keep management-service controls distinct from Minecraft console commands.
**Visual reference:** `/Users/camerontemple/Documents/msc2 pictures/Tabs/settings.png`; `/Users/camerontemple/Documents/msc2 pictures/SIdebar/services.png`; `/Users/camerontemple/Documents/msc2 pictures/SIdebar/howtoconnect.png`; `/Users/camerontemple/Documents/msc2 pictures/SIdebar/maintenance.png`.
**Verify:** `cargo nextest run -p msc-agent --test tui_settings_connections`
**Commit:** P13.11: add tui settings and connections
**Batch:** solo

### P13.12 — Deliver the Files section without expanding filesystem authority
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/tui/files.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_files.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Add the capability- and permission-gated Files tab using the existing scoped browse and bounded preview routes. Preserve the desktop's Server Root → folders/files → selected preview reading flow, server context, and path metadata, but label it honestly as read-only wherever the API exposes read-only behavior. A copyable/reported path is the terminal equivalent of Show in Finder; do not smuggle in remote filesystem access, arbitrary paths, or file mutations. Provide a narrow, keyboard-first browser/detail flow that works at small terminal sizes.
**Visual reference:** `/Users/camerontemple/Documents/msc2 pictures/Tabs/files.png`.
**Verify:** `cargo nextest run -p msc-agent --test tui_files`
**Commit:** P13.12: add tui files view
**Batch:** solo

### P13.13 — Deliver agent, help, and terminal-local support surfaces
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/tui/agent.rs`, `crates/msc-agent/src/cli/tui/handbook.rs`, `crates/msc-agent/src/cli/tui/app_settings.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_support.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Provide the surfaces a real client needs outside the seven tabs: local agent install/start/stop/reconnect/repair status; create or exchange pairing codes and choose an in-memory host session; the served 31-topic Handbook with search, related topics, and router-guide search/reader/troubleshooting; and the terminal-local preference/reset flow. Preserve the desktop distinction between client-local reset and authenticated host reset, including the host reset's selected target, running-server refusal, operation state, and fresh-pairing consequence. Replace graphical onboarding/tour affordances with a short, immediately discoverable keyboard help/first-session guide; do not omit the teaching content. Only settings meaningful to a terminal client may be exposed locally; service credentials and host-owned settings remain in their agent-backed flows, and no credential persists in ordinary configuration.
**Visual reference:** `/Users/camerontemple/Documents/msc2 pictures/Agent /Screenshot 2026-09-02 at 4.14.39 AM.png`; `/Users/camerontemple/Documents/msc2 pictures/MSC Settings/Screenshot 2026-09-02 at 4.15.35 AM.png`; `/Users/camerontemple/Documents/msc2 pictures/MSC Settings/Screenshot 2026-09-02 at 4.15.37 AM.png`; `/Users/camerontemple/Documents/msc2 pictures/Server handbook/Screenshot 2026-09-02 at 4.21.27 AM.png`; `/Users/camerontemple/Documents/msc2 pictures/Server handbook/Screenshot 2026-09-02 at 4.21.43 AM.png`.
**Verify:** `cargo nextest run -p msc-agent --test tui_support`
**Commit:** P13.13: add tui support surfaces
**Batch:** solo

### P13.14 — Record Phase 13 gate evidence
**Status:** awaiting verification
**Files:** `docs/msc2/client-capability-matrix.csv`, `docs/msc2/terminal-ui/phase13-gate.md`, `crates/msc-agent/tests/tui_phase_gate.rs`, `docs/msc2/rolling-plan.md`
**What:** Verify and record that bare interactive `msc` is a resilient full-screen API client while named and non-TTY commands remain scriptable; all delivered sections are capability/permission-gated; console, operation, and notification state reconnects with bounded local memory; and every parity-ledger row is either delivered or a named, owner-reviewed terminal exception. Record deterministic wide/medium/small rendering evidence and Cameron's side-by-side review against the exact linked Tauri screenshots and anti-slop checklist, including first/second/third reading order, rail/dock reachability, selected host/server visibility, state clarity, and the absence of a generic dashboard treatment. Update the TUI matrix cells only for capability-backed workflows actually delivered, leaving the scriptable CLI column independent. This is gate evidence, not an excuse to add routes or broaden filesystem/credential authority.
**Verify:** `cargo nextest run -p msc-agent --test tui_phase_gate`
**Commit:** P13.14: record terminal ui gate evidence
**Batch:** stop-after
