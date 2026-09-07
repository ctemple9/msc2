---
id: handbook.tailscale
kind: handbook
title: Tailscale
category: connection-access
subtitle: "Optional remote access for desktop, browser, and CLI clients."
analogy: "Tailscale gives the computer running your agent and the computer running your client private addresses they can use to reach each other without exposing the agent to the public internet."
relatedIds: [handbook.remote-access, handbook.port-forwarding-duckdns]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: tailscaleContent}
---

**Tailscale** is an optional private network for connecting the computer that runs MSC 2's agent to the computer from which you manage it. Install it on both computers, sign into the same tailnet, and they can reach each other without port forwarding.

**Why you’d use it:**

### Bullet List

- Reach a headless host from the Tauri desktop app or a desktop browser without opening the agent API to the public internet
- Use the same private host address from the headless CLI
- Keep the connection encrypted between your approved computers
- Leave Minecraft game-client networking to port forwarding or Playit.gg; Tailscale is for managing the host

### Callout: note

Tailscale is optional. Local MSC use does not require it, and it does not turn MSC into a general-LAN management service.

### Body

**Setup (one time per computer):**

### Checklist: Setup (one time per computer)

1. **Create a Tailscale account** — Go to tailscale.com and sign up.
2. **Install Tailscale on the agent host** — Sign in and confirm that the computer running the MSC agent appears in your tailnet.
3. **Install Tailscale on the client computer** — Sign in to the same tailnet on the computer running the Tauri desktop app, desktop browser, or CLI.
4. **Verify both computers appear connected** — Find the agent host in the Tailscale device list and copy its private address or tailnet name.
5. **Connect MSC to the host** — Use that address in the desktop/browser connection form or as the agent address for the CLI, then complete MSC pairing.

### In This App

- Use the agent host's Tailscale address in **Connect another agent**.
- Complete the normal one-use pairing flow; Tailscale provides reachability, not authentication.
- The agent remains on the host computer, and the desktop/browser/CLI client remains the control surface.

### Advanced Details

Tailscale uses WireGuard under the hood. Traffic between your approved devices is encrypted end-to-end. When a direct path is not possible, Tailscale can relay the encrypted traffic automatically.
