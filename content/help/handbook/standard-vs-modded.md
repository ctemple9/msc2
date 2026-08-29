---
id: handbook.standard-vs-modded
kind: handbook
title: Standard vs Modded
category: java-servers
subtitle: "The two main categories of Java server and the key difference between them."
analogy: "Standard Java servers are like a restaurant — the kitchen adds toppings and sauces (plugins) that diners don’t need to know about. Modded servers are like a potluck — everyone must bring the exact same dish from the same recipe. If one person shows up with a different dish, they don’t fit at the table."
relatedIds: [handbook.paper, handbook.fabric, handbook.client-requirements]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: standardVsModdedContent}
---

All Java servers fall into one of two categories. The choice shapes everything: who can connect, what add-ons work, and how much RAM you need.

**Standard servers — Paper, Purpur, and Vanilla**
Run plugins: server-side extensions that add features without requiring players to install anything. Your friends connect with normal Minecraft, no changes needed.

**Modded servers — Fabric, NeoForge, and Forge**
Run mods: code that modifies the game on both the server and every client simultaneously. Every player must install the same mod loader and the same set of mods before they can connect. If their mod list doesn’t match, they get a connection error.

### Bullet List

- Plugins: server-side only. Players need no extra setup — just vanilla Minecraft.
- Mods: both sides. Every player installs the same mod loader and mods.
- You cannot mix plugins and mods on the same server (with rare exceptions).
- Geyser/Floodgate (Bedrock cross-play) only works on Standard servers — not Modded.

### Callout: tip

Not sure which to pick? Choose Standard if your friends aren’t technical or if you want Bedrock cross-play. Choose Modded if you want new blocks, items, dimensions, or the experience of a specific modpack.

### In This App

- Create New Server: the first step asks you to choose Standard or Modded, then select the specific server type.
- Standard options: Paper (recommended), Purpur (Paper with extras), Vanilla (official Mojang, no plugins).
- Modded options: Fabric (lightweight, fast updates), NeoForge (large ecosystem), Forge (legacy/older packs).

### Advanced Details

There are hybrid approaches, but they’re advanced and not recommended for beginners:

• Fabric has some Plugin API compatibility mods (like Polymer) that allow limited plugin-like functionality on Fabric. These are still mods, so the client requirement applies.
• NeoForge had PluginLoader experiments historically, but the ecosystems are genuinely separate today.

For the vast majority of home servers, you pick one category and stick with it. Switching later requires creating a new server — the world data carries over but your add-ons do not.
