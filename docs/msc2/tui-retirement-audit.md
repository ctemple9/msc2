# Full-screen terminal client retirement audit

**Date:** 2026-09-07
**Decision:** D-034
**Steps:** P12.109–P12.114

## Result

MSC 2 no longer ships or promises a persistent full-screen terminal client.
The retained control surfaces are the Tauri desktop app, desktop browser, and
scriptable headless CLI. The Rust agent remains the server-side service and
continues to expose the shared HTTP and WebSocket contract consumed by the
desktop and browser clients.

The completed terminal-client implementation is recoverable from git history.
Historical planning and contract records remain factual and are identified
below rather than being rewritten to pretend the retired work never happened.

## Search terms and scope

The final repository scan searched for these retirement remnants:

```text
\bTUI\b
terminal UI
terminal dashboard
ratatui
crossterm
tui_status
P13.
```

The inventory covered the former CLI source and test target, Cargo manifests
and lockfile, CLI dispatch and bootstrap authentication, the capability matrix
and its checkers, active release and client documentation, Phase 13 scope
documents, the Rust agent WebSocket modules, and the controlled product and
engineering documents.

The active-content scan is:

```sh
git grep -n -i -E '\bTUI\b|terminal UI|terminal dashboard|ratatui|crossterm|tui_status|P13\.' \
  -- ':!docs/msc2/rolling-plan-archive.md' \
  ':!docs/msc2/audit/**' \
  ':!docs/msc2/msc2-decisions.md' \
  ':!docs/msc2/rolling-plan.md' \
  ':!docs/msc2/worlds/phase6-api.md' \
  ':!docs/msc2/tui-retirement-audit.md'
```

That scan is clean. The exclusions are deliberate:

- The working plan and this audit contain the literal search terms and the
  historical step descriptions needed to verify this retirement.
- The rolling archive, decision register, and MSC 1 audit are historical
  evidence, not active product promises.
- `docs/msc2/worlds/phase6-api.md` is a Phase 6 contract snapshot. Its old
  matrix-header examples document what was true when that phase was written;
  the current matrix and checker no longer contain that column.

## Source, dependency, and test inventory

- P12.111 deleted all 26 files under `crates/msc-agent/src/cli/tui/`.
  `main.rs` no longer chooses a terminal mode, and `cli/mod.rs` no longer
  exports or runs a TUI module.
- P12.111 deleted the 14 `crates/msc-agent/tests/tui_*.rs` integration-test
  targets. No TUI test target remains.
- P12.111 removed the direct `ratatui`, `crossterm`, `futures-util`, and
  `tokio-tungstenite` dependencies from `msc-agent`; the corresponding unused
  lockfile packages were pruned.
- P12.110 moved the retained CLI's HTTP transport and active-server selection
  into `crates/msc-agent/src/cli/transport.rs` and `session.rs` before the
  terminal client was deleted. This prevents the one-shot CLI from depending
  on terminal presentation code.
- P12.112 removed `tui_status` from the current capability matrix and removed
  TUI-specific checker and release-path plumbing. `cli_status` remains the
  independent scriptable-CLI column.
- P12.113 deleted `docs/msc2/terminal-ui/` and removed active Phase 13 scope,
  gate, and release language. The port plan's short retired-Phase-13 note is
  a factual product-history statement, not an implementation promise.

## Extracted CLI boundary

The retained CLI is one-shot and command-oriented:

- `clap` parses a named command; a bare `msc` invocation returns ordinary
  usage and never enters raw terminal mode or emits terminal control bytes.
- `cli::transport::SharedClient` owns bearer-authenticated HTTP requests and
  response decoding. `cli::session` resolves a server id or name and asks the
  agent to make it active through the normal API.
- Command modules render human-readable output or `--json`, preserve
  confirmations and exit codes, and use the same agent routes as the desktop
  and browser clients.
- Interactive behavior that remains supported is limited to command
  confirmations and other ordinary stdin/stdout CLI interaction. There is no
  persistent screen, layout loop, terminal renderer, or client-side
  WebSocket reconnect layer.

## Retained agent and WebSocket behavior

Retiring the client did not retire the shared service contract. The agent
still serves:

- `GET /v1/console/stream` for the desktop/browser console WebSocket, with
  `GET /v1/console/tail` and the short-lived stream-ticket route;
- `GET /v1/operations/{id}/stream` for live operation progress, alongside
  ordinary operation polling and cancellation; and
- `GET /v1/notifications/stream` for desktop/browser notifications.

Those routes are agent behavior used by retained graphical clients. Their
presence is not evidence of a terminal client, and the retained CLI uses the
HTTP command boundary rather than those WebSocket streams.

## Intentionally retained historical and generic references

- Git history retains P12.109–P12.113 and the deleted implementation. The
  `rolling-plan-archive.md` entry records that Phase 13 was implemented before
  D-034 retired it.
- `docs/msc2/msc2-decisions.md` retains D-034 and its reasoning; the MSC 1
  audit remains the compatibility evidence for the separate oracle project.
- `docs/msc2/worlds/phase6-api.md` retains its old Phase 6 capability-matrix
  examples, including the former column name, as a historical contract note.
- Generic terminal wording remains where it means Minecraft's own console,
  terminal-safe service administration, Java's `jline.terminal` setting, or
  ordinary CLI stdin/stdout behavior. Those references do not describe a
  supported full-screen MSC client and must not be removed.

## Final supported-client set

| Surface | Status | Boundary |
|---|---|---|
| Tauri desktop app | Supported | Shared Svelte frontend in a native shell |
| Desktop browser | Supported | The same served Svelte frontend |
| Headless CLI | Supported | Named, scriptable commands; local or remote agent |
| Full-screen terminal client | Retired | No source, test target, dependency, dispatch path, or release promise |

The agent is the shared service behind these clients, not a fourth client
surface. Optional Tailscale remains a transport option for remote desktop or
browser access, not a new control surface.
