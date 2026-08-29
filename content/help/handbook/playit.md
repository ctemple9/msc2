---
id: handbook.playit
kind: handbook
title: Playit.gg Tunneling
category: connection-access
subtitle: "Let friends connect to your server without touching your router."
analogy: "Normally your server is like a shop inside a building with a locked front door — friends need to know the address and the building needs to unlock the door (port forwarding). Playit.gg is like opening a franchise counter at their shopping mall. Friends go to the mall address; Playit.gg passes the traffic to your shop in the background. You never had to unlock your door."
relatedIds: [handbook.port-forwarding-duckdns, handbook.tailscale]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: playitSetupContent}
---

**Playit.gg** is a free tunneling service for game servers. Instead of requiring your router to accept inbound connections, the app runs a small native `playitd` agent that connects outbound to Playit.gg’s relay servers. Playit.gg gives you a stable public address — like abc123.jth.mc.ply.gg — that routes traffic back through the relay to your server.

**Why this matters:**

### Bullet List

- No router configuration needed — the agent makes outbound connections only
- Works behind CGNAT (some mobile and cable ISPs) where port forwarding is impossible
- Works on networks where you don’t have router access — dorms, apartments, offices
- The address Playit.gg gives you is stable and doesn’t change when your home IP changes

### Callout: tip

Playit.gg requires no extra software. The app downloads and manages the native playit agent automatically.

### Body

**Tradeoffs vs. port forwarding:**

### Bullet List

- Latency: game traffic passes through Playit.gg’s relay servers, adding roughly 10–50 ms. For most players this is unnoticeable.
- Account required: you need a free Playit.gg account (no credit card). One-time setup takes about 5 minutes.
- No extra installs: the app downloads and manages the native playit agent automatically.
- Free tier: Playit.gg’s free tier supports Minecraft tunnels without player count limits.

### In This App

- Create Server wizard: choose Tunnel (playit.gg) in the Network step instead of a direct port.
- Existing server: Edit Server → Settings → Network → toggle Playit.gg tunnel on.
- First start: an in-app sign-in sheet appears. Enter your Playit.gg email and password — no browser needed. The app handles the full setup natively.
- The app signs in, claims an agent, and automatically creates Java and Bedrock tunnels. If you already have an agent or tunnels, it reuses them rather than creating duplicates.
- Your Playit.gg addresses appear in the Overview connection card automatically once the agent is running.
- To start over (e.g. for re-testing), use the Reset button in the Playit.gg setup sheet to clear the local agent configuration.
- Voice Chat (Simple Voice Chat plugin): after enabling the tunnel, also enable Voice Chat Tunnel in Edit Server → Settings → Network, then create a matching Custom UDP tunnel in the Playit.gg dashboard.

### Callout: tip

You can use Playit.gg and port forwarding simultaneously on the same server. Players connecting via the Playit.gg address are relayed; players connecting to your direct IP go straight through. Useful as a fallback for players who have trouble with one approach.

### Advanced Details

The app runs the Playit.gg agent as a native background process alongside your Minecraft server. One agent can tunnel multiple ports — your Java port and Bedrock/Geyser port can both go through the same agent.

Setup is fully native: the app communicates directly with api.playit.gg using URLSession (not a browser or WebView), so there are no CORS restrictions. The sign-in → claim → exchange flow retrieves your secret key and stores it locally. The agent_id is also persisted and can be backfilled from the daemon’s connect log if you migrated from an earlier version.

Tunnels are created automatically for the server type (Java TCP, Bedrock UDP). The process is idempotent — if the agent or a tunnel already exists in your Playit.gg account, the app reuses it and skips creation.

Supported tunnel types used by MSC:
  • Minecraft Java (TCP) — for Paper servers
  • Minecraft Bedrock (UDP) — for Geyser or native Bedrock servers
  • Custom UDP — for Simple Voice Chat (port 24454)
