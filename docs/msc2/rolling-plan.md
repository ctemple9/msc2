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

### P13.1 — Define the TUI boundary and preserve the command-line contract
**Status:** not started
**Files:** `docs/msc2/terminal-ui/phase13-scope.md`, `docs/msc2/client-capability-matrix.csv`, `crates/msc-agent/src/main.rs`, `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/tests/tui_contract.rs`
**What:** Record the accepted invocation matrix and the client/agent boundary; add a distinct TUI capability-matrix column so an implemented one-shot CLI command is never mistaken for an implemented screen. Implement and test only the command-dispatch seam: named commands, `--json`, help, and non-TTY use remain conventional; bare interactive `msc` selects the TUI. Use the existing explicit token inputs for the first session and make any host switching in-memory only; do not create a plaintext profile store.
**Verify:** `cargo nextest run -p msc-agent --test tui_contract`
**Commit:** P13.1: define tui invocation contract
**Batch:** solo

### P13.2 — Establish terminal lifecycle, responsive layout, and deterministic rendering
**Status:** not started
**Files:** `crates/msc-agent/Cargo.toml`, `crates/msc-agent/src/cli/tui/mod.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/src/cli/tui/layout.rs`, `crates/msc-agent/src/cli/tui/render.rs`, `crates/msc-agent/tests/tui_terminal_lifecycle.rs`
**What:** Add the approved `ratatui` and `crossterm` foundation and a terminal guard that always restores raw mode, cursor, and alternate screen after normal exit, error, resize, or panic. Build the event loop, resize handling, visible keyboard focus, and the wide/medium/small layout state before feature screens. Use `ratatui`'s test backend to assert deterministic cell rendering at the three size classes; keep the shell quiet and terminal-native rather than filling it with generic cards.
**Verify:** `cargo nextest run -p msc-agent --test tui_terminal_lifecycle`
**Commit:** P13.2: establish tui terminal lifecycle
**Batch:** solo

### P13.3 — Extract only the shared authenticated transport the TUI needs
**Status:** not started
**Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/src/cli/tui/transport.rs`, `crates/msc-agent/src/cli/tui/session.rs`, `crates/msc-agent/tests/tui_transport.rs`
**What:** Move the existing CLI's HTTP request, bearer-auth, API-error, and selected-host primitives behind a shared client seam only where both one-shot commands and the TUI need them. Add authenticated WebSocket connection/reconnection support for the already-defined console, operation-progress, and notification paths, with bounded exponential backoff and a re-fetch after reconnect where the contract requires it. Preserve existing one-shot JSON output, polling, exit-code, and non-TTY behavior exactly; do not add a new API, local filesystem access to a remote host, or credential persistence.
**Verify:** `cargo nextest run -p msc-agent --test tui_transport`
**Commit:** P13.3: share tui api transport
**Batch:** solo

### P13.4 — Deliver the host/server and overview vertical slice
**Status:** not started
**Files:** `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/src/cli/tui/overview.rs`, `crates/msc-agent/src/cli/tui/render.rs`, `crates/msc-agent/tests/tui_overview.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Deliver a usable first TUI slice: an always-visible selected host and server, keyboard server selection, in-memory session-host switching, lifecycle controls, and Overview content driven by status, capabilities, health, connectivity, and performance API responses. Tab availability must come from the agent advertisement and token permissions, not a hardcoded product promise. Keep the live console reachable from every layout and use focused, labeled status rather than decorative color.
**Verify:** `cargo nextest run -p msc-agent --test tui_overview`
**Commit:** P13.4: add tui overview slice
**Batch:** solo

### P13.5 — Deliver the live console and raw-command vertical slice
**Status:** not started
**Files:** `crates/msc-agent/src/cli/tui/console.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_console.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Consume the console history-plus-live WebSocket stream with reconnect and a bounded local scrollback, falling back to the documented tail route only for recovery. Add keyboard search, pause/follow, copy-friendly selection, command history, and completion for TUI management actions. Make the input boundary explicit: `>` sends literal raw Minecraft console text only to `/v1/command`; a separate command palette/keybinding layer invokes MSC management actions. Never invent Minecraft command completion the API does not expose.
**Verify:** `cargo nextest run -p msc-agent --test tui_console`
**Commit:** P13.5: add tui live console
**Batch:** solo

### P13.6 — Deliver operation progress, notifications, and confirmation behavior
**Status:** not started
**Files:** `crates/msc-agent/src/cli/tui/activity.rs`, `crates/msc-agent/src/cli/tui/confirm.rs`, `crates/msc-agent/src/ws/notifications.rs`, `crates/msc-agent/tests/tui_activity.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Show bounded current-session operations and notifications, subscribe to each operation's existing progress stream, treat its documented terminal close as normal, and resync an operation with HTTP after reconnect. Verify the notification stream has real existing agent producers; if it is only an empty mounted stream, connect those producers to this already-specified channel rather than inventing a TUI-only feed, route, or event shape. Make destructive requests visibly target the selected host/server and require the agent's existing acknowledgement/confirmation response before dispatch; cancellation remains the agent's cooperative operation API.
**Verify:** `cargo nextest run -p msc-agent --test tui_activity`
**Commit:** P13.6: add tui activity streams
**Batch:** solo

### P13.7 — Deliver capability-backed Players and Performance sections
**Status:** not started
**Files:** `crates/msc-agent/src/cli/tui/players.rs`, `crates/msc-agent/src/cli/tui/performance.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_players_performance.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Add keyboard-first Players and Performance tabs using only their implemented, capability-advertised API data. Present a roster/session context and live metrics with labeled health meaning, compacting to focused lists on small terminals; unavailable edition-specific data is explained plainly rather than represented by fake empty widgets. Keep player mutations behind permission checks and the shared confirmation surface.
**Verify:** `cargo nextest run -p msc-agent --test tui_players_performance`
**Commit:** P13.7: add tui players and performance
**Batch:** solo

### P13.8 — Deliver the Worlds and Backups vertical slice
**Status:** not started
**Files:** `crates/msc-agent/src/cli/tui/worlds.rs`, `crates/msc-agent/src/cli/tui/backups.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_worlds_backups.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Add capability-filtered world-slot and backup workflows, including operation-backed progress and the agent's existing safety refusals. Put active-world identity, backup verification state, and destructive target/confirmation ahead of secondary metadata; preserve server-owned versus world-owned settings boundaries instead of presenting duplicate editors. On narrow terminals, use focused list/detail flows rather than a compressed table.
**Verify:** `cargo nextest run -p msc-agent --test tui_worlds_backups`
**Commit:** P13.8: add tui worlds and backups
**Batch:** solo

### P13.9 — Deliver the Components section
**Status:** not started
**Files:** `crates/msc-agent/src/cli/tui/components.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_components.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Add a focused Components tab for the capability-backed inventories, catalog actions, updates, and modpack state already available through the agent. Show operation progress and pack-managed or provider-unavailable responses as the API reports them; never present a control merely because a visual slot exists. Use searchable lists and detail views instead of terminal dashboard tiles, and route every mutation through the shared confirmation/error path.
**Verify:** `cargo nextest run -p msc-agent --test tui_components`
**Commit:** P13.9: add tui components
**Batch:** solo

### P13.10 — Deliver Settings and Connections sections
**Status:** not started
**Files:** `crates/msc-agent/src/cli/tui/settings.rs`, `crates/msc-agent/src/cli/tui/connections.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_settings_connections.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Add typed server settings, connection information, health/repair entry points, and supported networking/helper controls only when the active host and token advertise them. Render API-supplied help, timing, capability, and confirmation information rather than duplicating policy in the TUI. Treat credentials as write-only sensitive input, do not echo them in history or logs, and keep management-service controls distinct from Minecraft console commands.
**Verify:** `cargo nextest run -p msc-agent --test tui_settings_connections`
**Commit:** P13.10: add tui settings and connections
**Batch:** solo

### P13.11 — Deliver the Files section without expanding filesystem authority
**Status:** not started
**Files:** `crates/msc-agent/src/cli/tui/files.rs`, `crates/msc-agent/src/cli/tui/app.rs`, `crates/msc-agent/tests/tui_files.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Add the capability- and permission-gated Files tab using the existing scoped browse and bounded preview routes. Clearly label this initial surface as read-only wherever the API exposes only read-only behavior; do not smuggle in remote filesystem access, arbitrary paths, or file mutations. Retain the selected host/server context and provide a narrow, keyboard-first browser/detail flow that works at small terminal sizes.
**Verify:** `cargo nextest run -p msc-agent --test tui_files`
**Commit:** P13.11: add tui files view
**Batch:** solo

### P13.12 — Record Phase 13 gate evidence
**Status:** not started
**Files:** `docs/msc2/client-capability-matrix.csv`, `docs/msc2/terminal-ui/phase13-gate.md`, `crates/msc-agent/tests/tui_phase_gate.rs`, `docs/msc2/rolling-plan.md`
**What:** Verify and record that bare interactive `msc` is a resilient full-screen API client while named and non-TTY commands remain scriptable; all delivered sections are capability/permission-gated; console, operation, and notification state reconnects with bounded local memory; every size class has deterministic rendering evidence; and no screen claims backend behavior that is absent. Update the TUI matrix cells only for capability-backed workflows actually delivered, leaving the scriptable CLI column independent. This is gate evidence, not an excuse to add routes or broaden filesystem/credential authority.
**Verify:** `cargo nextest run -p msc-agent --test tui_phase_gate`
**Commit:** P13.12: record terminal ui gate evidence
**Batch:** stop-after
