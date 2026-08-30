---
id: handbook.remote-access
kind: handbook
title: MSC Remote (iOS)
category: connection-access
subtitle: "Monitor and control your servers from your iPhone."
analogy: "MSC Remote is like a remote control for your Mac's server. You can check if the server is running, see the console, and send commands — all from your iPhone, even when you're away from your Mac."
relatedIds: [handbook.tailscale, handbook.networking-basics]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: remoteAccessContent}
---

**MSC Remote** is a companion iOS app that connects to the **Remote API** built into Minecraft Server Controller on your Mac.

When the Remote API is enabled, your Mac acts as a small local server that MSC Remote can connect to. You can:
- See which server is running and its status (works for both Java and Bedrock servers)
- Read the live console log
- Send commands to the server
- Check player count and performance stats

### Callout: note

MSC Remote works over your local network by default using your Mac’s local IP. To use it from outside your home — a different network, cellular, traveling — you need Tailscale. Tailscale creates a secure private connection between your iPhone and Mac that works from anywhere with no port forwarding required. See the Tailscale topic in Connection & Access for a step-by-step setup guide.

### Body

**Access Tokens — How Security Works**

The Remote API uses tokens instead of passwords. There are two types:
- **Owner token**: full access — stored securely in your Mac's Keychain
- **Shared access tokens**: limited read-only access — you can create these for guests or family members who should be able to check the server but not send commands

### Callout: warning

Keep your Owner token private. Anyone with the Owner token can send commands to your server. Shared access tokens are safer to distribute.

### In This App

- Preferences → Remote API: enable the API server and configure the port (default: a local port you choose).
- Owner token: generated once and stored in macOS Keychain. View it in Preferences to enter in MSC Remote.
- Shared access tokens: create in Preferences for guests. These tokens have read-only access.
- MSC Remote iOS app: available separately. Enter your Mac's local IP, port, and token to connect.
- MSC Remote works with both Java and Bedrock servers.

### Advanced Details

The Remote API runs as a lightweight HTTP server on your Mac. It only accepts connections from authorized tokens — unauthenticated requests are rejected.

For remote access outside your home network, Tailscale is a popular zero-config VPN option. It assigns your Mac a stable private IP that you can reach from anywhere, which you can use with MSC Remote instead of your local IP.

The API exposes endpoints for server status, console streaming (via polling or SSE), and command dispatch. If you're technically inclined, you can also build your own client using the same API.
