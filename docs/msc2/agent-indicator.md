# Local agent indicator

The helmet means the local MSC 2 agent is running. It disappears when the
connection to that agent closes. It does not describe a remote host or whether
a Minecraft server is running. A process that hangs without exiting may retain
the symbol: this is a process-lifecycle indication, not a health assessment.

The separate indicator process can run while the desktop window is closed.
Its menu provides **Open MSC 2** and **Quit indicator**. Quitting it leaves the
agent and Minecraft servers running. The indicator appears again at the next
login or when the desktop app starts.

## Desktop startup

After a release desktop build finds or installs a local agent, it registers the
indicator for that desktop user and starts it. Registration uses a LaunchAgent
on macOS, the user's Run registry key on Windows, and an XDG autostart entry on
Linux. Linux needs a desktop with system-tray support. Standalone headless
packages acquire no graphical dependencies or login registrations.

The indicator uses the agent's existing account-authenticated local socket or
Windows named pipe. The agent acknowledges the subscription once and holds
the connection open. There is no periodic HTTP check or heartbeat. When
disconnected, connection retries back off from one second to a maximum of
30 seconds, so an agent started later may take that long to appear.

Coordinated desktop updates stop the indicator before replacing the app.
Removing the local agent through the desktop app stops the indicator and
removes its login registration. The full-uninstall path through the desktop
also does this. An indicator left running during CLI removal cleans up its
registration after its desktop agent data directory disappears.

## Development

Debug builds do not register login startup or launch a companion automatically.
Install or repair the agent from a desktop development build containing the
lifecycle-channel changes, then run this from the repository root:

```sh
cargo run --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- --agent-indicator
```

The companion has no webview. It uses a file lock to permit one indicator per
desktop user's agent data directory. A separate local control connection lets
updates stop the companion without stopping the agent.

## Owner verification

On each platform, use a build containing both the agent and desktop changes:

1. Start the local agent and confirm the helmet appears. Close the desktop
   window and confirm the helmet remains.
2. Stop the agent and confirm the helmet disappears. Start it again and allow
   up to 30 seconds for the helmet to return.
3. Open the desktop app repeatedly and confirm there is one helmet. Use
   **Open MSC 2**, then **Quit indicator**, and confirm the agent still runs.
4. With a release build, sign out and back in. Confirm startup with both a
   running and a stopped agent. On Linux, use a desktop with tray support.
5. Check coordinated update and local-agent removal. Confirm replacement can
   proceed and removal clears that user's startup registration.

Agent `cargo check` and Clippy passed on macOS. Desktop `cargo check` passed;
Clippy passed with allowances for three pre-existing warnings in updater and
service code (`needless_borrow`, `needless_borrows_for_generic_args`, and
`needless_return`). No test suites or physical platform checks were run by the
implementing agent. Windows and Linux builds and desktop behavior still need
owner verification.
