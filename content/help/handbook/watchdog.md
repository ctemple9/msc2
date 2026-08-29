---
id: handbook.watchdog
kind: handbook
title: Watchdog & Crash Recovery
category: server-management
subtitle: "Automatically restart your server if it crashes unexpectedly."
analogy: "The watchdog is a supervisor that checks on your server at regular intervals. If the server stopped but was never told to stop, the supervisor restarts it. If you stopped it intentionally, the supervisor sees the note you left and leaves it stopped."
relatedIds: [health.last-startup, diagnostics.crash.unknown]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: watchdogContent}
---

Minecraft servers can crash — a buggy plugin, an out-of-memory condition, or a rare JVM error can bring the server down without warning. Without a watchdog, it stays down until someone manually restarts it.

**How the MSC watchdog works:**

### Bullet List

- A background launchd agent (a macOS system service) polls periodically to check whether your server process is running
- When you stop the server intentionally via the Stop button or the /stop command, the app writes a clean-quit marker to the server folder
- If the watchdog finds the server stopped but no clean-quit marker present, it treats the stop as a crash and restarts the server automatically
- If the clean-quit marker is present, the watchdog recognizes the intentional stop and leaves the server down

### Callout: note

The watchdog runs via macOS launchd, so it continues monitoring even when the Minecraft Server Controller app is not open on screen.

### In This App

- Preferences → Watchdog: enable and configure crash recovery per server.
- Restart delay: how long the watchdog waits before restarting after detecting a crash. Default is 30 seconds to let the system settle.
- Watchdog activity appears in the console log tagged [Watchdog] so you can see when it took action.
- To stop a server and keep it stopped: always use the Stop button in MSC or the /stop command. These write the clean-quit marker.

### Callout: warning

Don’t stop your server by killing the process in Activity Monitor or Terminal. This bypasses the clean-quit marker, and the watchdog will restart the server immediately — which is probably not what you intended.

### Callout: tip

Enable the watchdog any time you’re leaving your server running overnight or while away from your Mac. It’s especially valuable if you have players who rely on the server being available without you monitoring it constantly.

### Advanced Details

The launchd agent is a lightweight plist that macOS loads on login. It polls at a configurable interval (default: 60 seconds). The interval is intentionally not too short — a rapid restart loop after a repeating crash could hammer disk I/O unnecessarily.

The clean-quit cookie is a file written to the server’s directory when MSC initiates an intentional stop, and removed when the server is started again. The watchdog reads this file before deciding whether to restart.

If your server crashes immediately on every restart due to a bug or corrupt world, the watchdog will keep restarting it. In that case, stop the server intentionally (which writes the cookie) and investigate the console log before re-enabling the watchdog.
