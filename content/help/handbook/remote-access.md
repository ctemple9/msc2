---
id: handbook.remote-access
kind: handbook
title: Remote access
category: connection-access
subtitle: "Connect a supported desktop, browser, or CLI client to an agent on another computer."
analogy: "The agent stays with the Minecraft servers. A desktop app, browser, or CLI on another computer is the remote control that sends instructions and reads the results."
relatedIds: [handbook.tailscale, handbook.networking-basics]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: remoteAccessContent}
---

MSC 2 keeps the control panel separate from the **agent** that runs your Minecraft servers. The agent can run headless on another computer while you connect to it with the Tauri desktop app, a desktop browser, or the headless CLI.

Once connected, you can:
- See which server is running and its status
- Read the live console log
- Send commands to the server
- Check player counts and performance stats

### Callout: note

For a remote host, make the agent reachable through an SSH tunnel or an optional private network such as Tailscale. Tailscale is not required when the client and agent are on the same computer.

### Body

**Pairing — How access works**

Create a one-use pairing code on the computer running the agent. Enter that code in the client you are connecting from. The agent exchanges it for a lasting client credential, so the code itself does not need to be stored or reused.

The browser client keeps its session in an httpOnly cookie. The Tauri desktop client and CLI use local bearer credentials. In every case, the agent checks the credential before accepting a management request.

### In This App

- **Tauri desktop app:** connect to a local or remote agent and manage its servers.
- **Desktop browser:** connect through the agent-served page or the browser client, then use the pairing flow.
- **Headless CLI:** create or exchange a pairing code from Terminal, then use the agent address for commands.
- **Remote host service controls:** start, stop, and repair the service on the computer that hosts the agent.

### Advanced Details

The agent exposes an authenticated HTTP and WebSocket API for server status, console streaming, operations, and command dispatch. Keep that API on loopback or a private path such as an SSH tunnel or tailnet; do not expose it directly to the public internet.

The client and agent may be on different computers, but the Minecraft server files and service remain on the agent's host. Closing the client does not stop a running server.
