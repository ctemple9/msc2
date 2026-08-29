---
id: handbook.tailscale
kind: handbook
title: Tailscale
category: connection-access
subtitle: "Access MSC Remote from anywhere using a secure private network."
analogy: "Tailscale gives every device you add a permanent private phone number that only your approved devices can call — no matter where any of them are in the world. Your Mac gets a Tailscale IP (like 100.64.0.1) that never changes and is only reachable from your other Tailscale devices."
relatedIds: [handbook.remote-access, handbook.port-forwarding-duckdns]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: tailscaleContent}
---

**Tailscale** is a zero-config VPN that creates a secure private network between your devices. There’s no server to maintain and no port forwarding to set up. You install Tailscale on your Mac and on your iPhone, sign into the same account on both, and they can reach each other from anywhere — home Wi-Fi, cellular, a friend’s network, anywhere with internet.

**Why you’d use it:**

### Bullet List

- Control your Minecraft server via MSC Remote from outside your home without exposing the Remote API port on your router
- Access the console and send commands while you’re away
- Your Tailscale IP never changes, so your MSC Remote pairing stays valid even when your home IP changes
- Fully encrypted end-to-end — no one can intercept Remote API traffic

### Callout: note

Tailscale is for MSC Remote app access and server management. It is not a replacement for port forwarding or Playit.gg for Minecraft game clients — players still need one of those two approaches to join your server.

### Body

**Setup (one time per device):**

### Checklist: Setup (one time per device)

1. **Create a Tailscale account** — Go to tailscale.com and sign up. Free for personal use with up to 3 users and 100 devices. No credit card required.
2. **Install Tailscale on your Mac** — Download from the Mac App Store or tailscale.com. Open it and sign in with your account. Your Mac is now assigned a Tailscale IP — click the menu bar icon to see it. It starts with 100.
3. **Install Tailscale on your iPhone** — Search “Tailscale” in the App Store. Sign in with the same account. Tap Connect.
4. **Verify both devices appear connected** — In Tailscale on your iPhone, you should see your Mac listed with a green dot. That’s all the setup required.
5. **Use the Tailscale IP in MSC Remote** — In MSC Remote → Settings, set the Base URL to your Mac’s Tailscale IP and the Remote API port — for example: http://100.64.0.1:48400. This works on any network automatically.

### In This App

- Preferences → Remote API: shows your Mac’s local URL and port. Replace the local IP with your Tailscale IP when configuring MSC Remote for use outside your home network.
- Your Mac’s Tailscale IP is shown in the Tailscale menu bar icon. It starts with 100.
- Once configured with a Tailscale IP, MSC Remote works everywhere — you don’t need to switch between home and away configurations.

### Advanced Details

Tailscale uses WireGuard under the hood — a modern, fast, and widely-audited VPN protocol. Traffic between your devices is encrypted end-to-end. Tailscale’s coordination servers handle device discovery but never see your traffic.

For most home setups, Tailscale establishes direct peer-to-peer connections once both devices are online, giving very low latency. When a direct path isn’t possible (strict NATs, some firewalls), it falls back to Tailscale’s DERP relay servers automatically.

If you run multiple Macs with MSC, each gets its own Tailscale IP and can be reached independently from MSC Remote.
