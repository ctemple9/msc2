# Phase 13 — Terminal UI gate evidence

**Evidence status:** implementation evidence recorded 2026-09-02; awaiting
Cameron's Verify run and side-by-side visual review before the phase is closed.

## Boundary evidence

Phase 13 keeps the scriptable CLI and the interactive TUI as separate
invocation targets. Named commands, `--json`, help, and non-TTY bare
invocations remain on the conventional CLI path. Bare `msc` selects the TUI
only when stdin and stdout are TTYs, `TERM` is usable, and JSON mode is absent.
The TUI holds host sessions, notes, scrollback, and activity state in memory;
the agent remains authoritative for authentication, capabilities,
permissions, confirmations, operations, and filesystem scope. No second API,
plaintext credential store, or arbitrary remote path access was added.

The exact dispatch and boundary evidence is in `tui_contract`; the terminal
guard and panic/error restoration evidence is in `tui_terminal_lifecycle`; and
the shared authenticated HTTP/WebSocket behavior is in `tui_transport`.

## Automated gate evidence

`tui_phase_gate` adds the phase-level acceptance check at the rendered-client
boundary:

| Requirement | Evidence |
|---|---|
| Wide Tauri reading order | The rendered 140×42 shell retains host/server/state identity, controls rail, server identity, seven sections, overview content, and the bottom console in order. |
| Medium layout | The rendered 100×30 shell keeps the rail and console visible, and keyboard toggles report each as explicitly shown or hidden. |
| Small layout | The rendered 70×20 shell keeps host context and exposes section, console, and help surfaces without clipping. |
| Capability and permission filtering | A restricted advertised token receives only Overview and Performance; a token advertising the required categories and `admin` receives all seven sections, including Files. |
| Terminal-native output | Every rendered line fits its test backend width; the test checks labeled hierarchy and focus/reachability rather than decorative color or dashboard tiles. |
| Bounded/reconnecting live state | `tui_console`, `tui_activity`, and `tui_transport` provide the focused scrollback, deduplication, reconnect, backfill, operation resync, and terminal-close evidence used by this gate. |

The capability matrix is valid CSV and its `tui_status` column is independent
of `cli_status`. Only workflows delivered by the Phase 13 steps are marked
`Implemented`; deferred or unsupported client workflows remain `Planned`.

## Parity-ledger disposition

Every row in `phase13-scope.md` has an explicit Phase 13 destination and a
terminal treatment. The treatments translate graphical affordances into
keyboard-first list/detail, focused, or modal flows while preserving the
underlying workflow. Named exceptions are limited to presentation that has no
terminal equivalent: window chrome, Finder reveal/pickers, and image art. They
do not remove a management workflow or broaden the agent boundary.

The seven primary sections are owned by P13.4, P13.7, P13.8, P13.9, P13.11,
and P13.12. Manage Servers and its editor are owned by P13.10. Agent/pairing,
Handbook/router guides, and terminal-local settings/reset are owned by P13.13.
The shell, responsive layout, lifecycle, and console dock are owned by P13.2
through P13.6. The ledger remains the binding list of surfaces and references.

## Cameron’s visual review

**Status: pending.** Cameron reviews the exact reference PNGs indexed by the
ledger, at wide (120+ columns × 36+ rows), medium (80–119 columns or 24–35
rows), and small (under 80 columns or 24 rows), against the rendered shell and
the checklist in `docs/msc2/antiAIslop.md`.

The review records these points explicitly:

- first read: selected host/server and runtime state;
- second read: controls rail, server identity, and section context;
- third read: Overview’s Connection and Live Stats, Health, Activity, local
  Notes, and the persistent console path;
- rail and console reachability at every supported size;
- labeled state clarity without decorative status dots or competing accents;
- no generic dashboard treatment, decorative panels, gradients, glow, or
  overflow.

Reference groups are the exact paths in the scope ledger: Main View, Sidebar,
Tabs, Edit Server, Agent, MSC Settings, and Server handbook under
`/Users/camerontemple/Documents/msc2 pictures/`.
