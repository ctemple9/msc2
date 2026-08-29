---
id: handbook.port-forwarding-duckdns
kind: handbook
title: Port Forwarding & DuckDNS
category: connection-access
subtitle: "Configure your router to let friends outside your house reach your server."
analogy: "The internet is a city. Your public IP is your building's address. Ports are the apartment numbers inside. Your server is apartment 25565 (Java) or 19132 (Bedrock). Friends need both the street address and the apartment number to visit — and your router needs to be told which apartment to send them to."
relatedIds: [handbook.networking-basics, handbook.playit]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: portsForwardingDuckDNSContent}
---

### Callout: note

There are two ways to let external players connect. Port forwarding (this topic) configures your router directly. Playit.gg tunneling (next topic) routes traffic through a relay service — no router access needed. See the "How Servers Connect" topic for a side-by-side comparison. If you’re unsure which to use, read that topic first.

### Body

**IP Address**: your internet connection's numeric address, like `123.45.67.89`. It's assigned by your internet provider and can change.

**Port**: a numbered channel on your computer. Different programs listen on different ports. Minecraft's defaults are:
- Java/Paper: **25565 (TCP)**
- Bedrock/BDS or Geyser: **19132 (UDP)**

**Port forwarding** tells your router to pass incoming traffic on a specific port to a specific computer on your home network. Without it, friends outside your house are blocked by your router and can't reach your server.

### Callout: pitfall

LAN (local network) players don't need port forwarding — they connect directly using your Mac's local IP (like 192.168.1.x). Port forwarding is only needed for players outside your home.

### Body

**DuckDNS** solves a common problem: your home IP address changes over time. DuckDNS gives you a free, stable hostname like `yourname.duckdns.org` that automatically updates to point at your current IP. You give this hostname to friends instead of your raw IP.

### Callout: note

Setting up DuckDNS is optional but highly recommended. Without it, you'll need to send your friends your updated IP address every time it changes (which can happen weekly or after power outages).

### In This App

- Settings → Network: set Server Port (Java) and Bedrock Port (Geyser or BDS).
- Details tab: shows Java address, Bedrock address, and your DuckDNS hostname if configured.
- Preferences: paste your DuckDNS hostname — the app uses it throughout the connection info panels.
- The app does NOT configure your router. You must log into your router and set up port forwarding manually.

### Advanced Details

Router port forwarding varies by manufacturer, but the concept is always:
  "When someone connects to [my external IP]:[port], send them to [Mac's local IP]:[same port]."

For a typical setup:
  • TCP port 25565 → your Mac's local IP (find it in System Settings → Network)
  • UDP port 19132 → same Mac (for Bedrock BDS or Geyser cross-play)

Some routers call this "Virtual Servers" or "NAT rules" instead of "Port Forwarding" — it's the same thing.

Important: Java uses TCP. Bedrock uses UDP. These are different protocols. If you forward TCP 19132 instead of UDP 19132 for Bedrock, it won't work.
