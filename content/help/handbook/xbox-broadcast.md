---
id: handbook.xbox-broadcast
kind: handbook
title: Xbox Broadcast
category: connection-access
subtitle: "Let Bedrock friends see your Java server appear in their Friends tab."
analogy: "Geyser lets Bedrock players connect to your Java server, but they still have to know your address. Xbox Broadcast is like hanging a party flyer in the hallway — your world shows up in Bedrock friends' \"Friends\" tab as joinable, so they can join with one tap instead of manually entering an address."
relatedIds: [handbook.bedrock, handbook.plugins-crossplay]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: broadcastContent}
---

Minecraft Server Controller integrates with **MCXboxBroadcastStandalone** — an open-source tool that uses a dedicated Xbox/Microsoft account to broadcast a "joinable session" so Bedrock friends can see your server in their Friends tab.

**How it works:**
1. You create a free alt Microsoft/Xbox account specifically for broadcasting.
2. Your Bedrock-playing friends add that alt account as a friend.
3. When broadcast is running, your server appears as joinable in their Friends tab.

### Callout: warning

This feature requires a dedicated alt Xbox account, not your personal one. Never use your main Microsoft account.

### Callout: note

Why a separate account? Xbox Broadcast keeps your server visible in the friends list by staying signed into Xbox Live continuously while the server runs. Using your real account would mean it appears permanently “online” elsewhere, which can conflict with other Xbox activity and Game Pass sessions. A free dedicated alt account — any Outlook.com address works — keeps this completely separate from your personal gaming.

### Body

**Requirements before enabling broadcast:**
- Geyser and Floodgate installed and working on your Java server
- Bedrock port forwarded on your router
- A free alt Microsoft account created specifically for this purpose

### Callout: note

Broadcast does not tunnel traffic or bypass your router. It only advertises the session. Friends still connect through your normal Bedrock port — port forwarding is still required.

### In This App

- Manage Servers → Edit → Broadcast tab: enable broadcast, then click Authenticate.
- An inline Microsoft sign-in window opens inside the app — sign in with your alt account. The code is pre-filled; no copying or browser switching needed.
- Sign-in uses a private, isolated session — your personal Microsoft accounts are never touched.
- The window closes automatically once sign-in is complete.
- When the server starts with broadcast enabled, MCXboxBroadcastStandalone starts automatically.
- Broadcast log lines appear in the console tagged as [Broadcast] so you can see its status.
- The broadcast helper JAR is managed through the app’s JAR library — downloaded once, used by any server.

### Advanced Details

MCXboxBroadcastStandalone is an independent open-source project. Source code and documentation are available at:
github.com/MCXboxBroadcast/Broadcaster

The app generates a per-server config.yml for the broadcast helper, which includes your server's Bedrock IP and port. This config is stored in the server's folder and updated if you change Bedrock port settings.

The inline sign-in sheet uses a non-persistent WKWebView data store — cookies and session data are discarded after sign-in, so your personal Microsoft account is never at risk of cross-contamination.

Xbox broadcast has occasional authentication token expiry — if it stops working, the broadcast helper usually recovers automatically. If it doesn't, stopping and restarting the server resets the auth flow.
