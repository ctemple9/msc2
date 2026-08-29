---
id: handbook.paper
kind: handbook
title: What is Paper?
category: java-servers
subtitle: "Why Paper, and how it's different from vanilla Minecraft."
analogy: "Vanilla Minecraft is like the stock engine in a car — official, reliable, but limited. Paper is a performance-tuned replacement engine that's still compatible with the original car, runs better, and lets you bolt on extra parts (plugins)."
relatedIds: [handbook.vanilla, handbook.purpur, handbook.plugins-crossplay]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: paperContent}
---

Paper is the most popular server in the **Standard Java** family. Standard servers support plugins — server-side extensions that players don’t need to install anything to benefit from.

**Vanilla Minecraft** is the official server from Mojang. It works, but it has limited configuration options and no plugin support.

**Paper** is a "fork" — a modified version of the vanilla server that keeps gameplay the same but adds:

### Bullet List

- Significantly better performance under load
- Hundreds of extra configuration options
- Support for the massive Bukkit/Spigot/Paper plugin ecosystem
- Faster bug fixes than the vanilla server

### Callout: note

Paper maintains full compatibility with vanilla Java clients. Your friends don't need to install anything special to join — they just use their normal Minecraft launcher.

### In This App

- Each server folder needs a paper.jar file. The app downloads and manages these via Paper Templates.
- Use Paper Templates → Download latest Paper to grab the newest build.
- Each server entry stores its own Paper JAR path — different servers can run different Paper versions.

### Advanced Details

The Standard Java server family tree:

  CraftBukkit → Spigot → Paper → Pufferfish → Purpur

Each adds more features and performance improvements on top of the last. Vanilla sits outside this tree — it’s the official Mojang server with no third-party modifications.

Paper is the sweet spot for most home servers. Purpur is for server operators who want hundreds of extra configuration toggles on top of Paper’s foundation. Vanilla is for purists who want zero modifications.

The Standard family is entirely separate from Modded servers (Fabric, NeoForge, Forge). Those use a different loading system and a different add-on ecosystem (mods instead of plugins). See the Modded Servers section of this handbook for details.
