---
id: handbook.bedrock
kind: handbook
title: What is Bedrock Dedicated Server?
category: bedrock-servers
subtitle: "The native server for Minecraft on mobile, console, and Windows 10/11."
analogy: "If Java Edition is the PC version of Minecraft, Bedrock Edition is the universal version that runs everywhere else — phones, tablets, Xbox, PlayStation, Nintendo Switch, and Windows 10/11. BDS is the dedicated server built for that universal version. Everyone on those platforms can connect to it with no extra setup on their end."
relatedIds: [handbook.how-bedrock-runs, handbook.plugins-crossplay, bedrock.runtime-unavailable]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: bedrockContent}
---

**Bedrock Edition** is the version of Minecraft available on mobile (iOS/Android), consoles (Xbox, PlayStation, Nintendo Switch), and Windows 10/11. It's sometimes called the "universal" edition because all those platforms play together seamlessly.

**BDS (Bedrock Dedicated Server)** is Mojang's official server software for Bedrock Edition. When you run a native Bedrock server:

### Bullet List

- Mobile players (iOS/Android) can join directly
- Console players (Xbox, PlayStation, Switch) can join directly
- Windows 10/11 Minecraft players can join directly
- No plugins, translators, or extra configuration required for any of this — it's built in

### Callout: note

Java Edition players cannot join a Bedrock server natively. If you want both Java and Bedrock players on the same server, use a Java (Paper) server with the Geyser plugin. See the Plugins & Cross-Play topic.

### Body

**Bedrock vs. Paper — which should you run?**

Choose **Bedrock** if your players are primarily on mobile, console, or Windows 10/11, and you don't need Java plugins.

Choose **Paper (Java)** if your players are primarily on Java Edition, or if you want the rich plugin ecosystem (economy, claims, mini-games, etc.).

You can run both from this app simultaneously.

### In This App

- Create New Server → choose Bedrock to create a native BDS server.
- Bedrock server port: Default port is 19132 UDP (not TCP). Port forwarding must be UDP.
- Player management uses allowlist.json and permissions.json — the app handles these for you.
- No Java installation needed for Bedrock servers — the built-in VM provides the runtime.

### Advanced Details

MSC checks the selected Bedrock version and installed Bedrock runtime before presenting native controls. The common 1.20+ profile includes generation, difficulty, game mode, gamerules, coordinates, starting-map, cheats, and experiments where that runtime advertises them. Unavailable controls are explained and are never sent as though the runtime applied them; unknown properties remain visible and preserved.

Settings supplied by a particular server build or add-on are **provided by this server/mod**, outside MSC's native Bedrock profile. Use that server or mod's own configuration path for them; MSC will not silently discard a setting it cannot understand.
