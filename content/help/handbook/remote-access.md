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

There is no universal remote route and no “Tailscale without Tailscale” shortcut. The remote computer
must be reachable through a path you control: a direct LAN or DNS route, an SSH tunnel, a user-operated
VPN or overlay, or a relay service. MSC provides no cloud relay and does not require Tailscale.

### Body

**Choose the route that matches your network**

- **LAN or DNS:** Use a private IP address or hostname when the client and agent can already reach each other. This is the simplest direct path.
- **Tailscale:** Use a Tailscale IP address or name as an optional private route. It can connect computers that are not on the same LAN, but both computers need to be on the same tailnet. Tailscale supplies reachability, not MSC authorization.
- **SSH tunnel:** MSC's Tauri desktop can manage a local forward from `127.0.0.1:48002` to the agent's loopback-only `127.0.0.1:48001`. The local port is configurable when it is busy. MSC owns the tunnel session and cleans it up when you switch hosts or close the app.
- **Your own VPN or overlay:** A VPN, mesh, or other private network that you operate can provide the address. Enter that address and use the same authenticated MSC connection.
- **Router forwarding or a relay:** These are possible network arrangements, but they are not MSC's built-in remote-management path. Public exposure adds firewall, encryption, authentication, and maintenance responsibilities; a relay adds a service provider and its trust boundary.

MSC does not operate a relay and cannot manufacture reachability. If the two computers cannot reach each other directly, through a tunnel, or through a network you operate, another service or network change is required.

### Callout: warning

Do not expose the MSC management port to the public internet just because Minecraft players need a public address. Player access and administrator access are different paths. Playit.gg and Minecraft port forwarding are for player traffic; the management API remains authenticated and binds to loopback by default.

### Body

**Pairing — How access works**

Create a one-use pairing code on the computer running the agent. Enter that code in the client you are connecting from. The agent exchanges it for a lasting client credential, so the code itself does not need to be stored or reused.

The browser client keeps its session in an httpOnly cookie. The Tauri desktop client and CLI use local bearer credentials. In every case, the agent checks the credential before accepting a management request.

### In This App

- **Tauri desktop app:** connect to a local or remote agent and manage its servers.
- **Desktop browser:** connect through the agent-served page or the browser client, then use the pairing flow.
- **Headless CLI:** create or exchange a pairing code from Terminal, then use the agent address for commands.
- **Remote host service controls:** service installation, start, stop, and repair happen on the computer that hosts the agent. A remote client manages Minecraft through the authenticated agent; it does not manage the host operating-system service.

### Body

**The CLI with a direct route or tunnel**

For a direct private address, point the CLI at the agent's management port:

```text
msc --host 192.168.1.42 --port 48001 status
```

For a manually maintained SSH tunnel, forward the remote loopback port and point
the CLI at the local end:

```text
ssh -N -L 127.0.0.1:48002:127.0.0.1:48001 user@server-host
msc --host 127.0.0.1 --port 48002 status
```

The Tauri desktop uses the same transport boundary without requiring the user to
open a terminal. In both cases, the route carries requests; the per-host bearer
credential and permission checks still authorize them.

### Advanced Details

The agent exposes an authenticated HTTP and WebSocket API for server status, console streaming, operations, and command dispatch. Keep that API on loopback or a private path such as an SSH tunnel, tailnet, LAN, or user-operated VPN. Router forwarding and public exposure are deliberate security decisions with their own firewall, encryption, authentication, and maintenance burden; MSC does not recommend forwarding the management port publicly.

The client and agent may be on different computers, but the Minecraft server files and service remain on the agent's host. Closing the client does not stop a running server.
