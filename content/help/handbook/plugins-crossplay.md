---
id: handbook.plugins-crossplay
kind: handbook
title: Plugins & Cross-Play
category: java-servers
subtitle: "Extend your Java server and let Bedrock players join."
analogy: "Plugins are app add-ons — you drop them in a folder and they give the server new abilities. Geyser is a real-time translator between Java and Bedrock Minecraft, and Floodgate is the guest pass that lets Bedrock players in without a Java account."
relatedIds: [handbook.paper, handbook.bedrock]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: pluginsGeyserFloodgateContent}
---

### Callout: note

This topic covers cross-play for Java servers via Geyser/Floodgate. If you're running a native Bedrock server, cross-play with mobile, console, and Windows 10/11 players is built in — no plugins needed.

### Callout: warning

Geyser and Floodgate only work on Standard Java servers (Paper, Purpur). Modded servers (Fabric, NeoForge, Forge) cannot use Geyser. Bedrock clients would need to install the same mods as Java players, which isn’t possible — Bedrock Edition has no mod-loading support.

### Body

**Plugins** are `.jar` files in your server's `plugins/` folder. They load when the server starts and can add almost anything — new commands, economy systems, claim protection, chat formatting, mini-games, and more.

**Geyser** is a plugin (and also a standalone proxy) that translates the Bedrock network protocol into Java protocol in real-time. This means Bedrock players on phones, tablets, consoles, and Windows 10/11 can join your Java Paper server.

**Floodgate** works alongside Geyser to allow Bedrock players to join without owning a separate Java Edition account. Without Floodgate, `online-mode=true` would block Bedrock players.

### Callout: note

Geyser/Floodgate doesn't make the experience perfect — some Java-only features like certain inventory layouts or maps behave differently on Bedrock. But basic gameplay works well.

### In This App

- Plugin Templates: download the latest Geyser and Floodgate JARs once into a global template folder.
- When creating a server: enable Bedrock Cross-play to automatically copy Geyser and Floodgate into the server's plugins/ folder.
- Update Geyser / Update Floodgate buttons: one-click to pull the newest version from your templates into the current server.
- Bedrock Port: configure this in Settings → Network. Default is 19132 (UDP).

### Callout: pitfall

Common pitfall: Geyser listens on a separate port from your Java server (usually 19132 UDP vs. 25565 TCP). You need to forward BOTH ports on your router for external connections.

### Advanced Details

Geyser works in two modes:
• Plugin mode (used here): Geyser runs inside your Paper server. Simpler setup.
• Proxy mode: Geyser runs as a standalone proxy in front of the server. More flexible but more complex.

This app uses plugin mode by default, which is correct for most home servers.

Bedrock players connect to the same external IP as Java players, but use a different port. In Minecraft Bedrock, they go to Settings → Servers → Add Server and enter your IP and Bedrock port.

Floodgate also handles skin data for Bedrock players, so they show up in-game with their Bedrock skin and a "." prefix on their username (configurable).
