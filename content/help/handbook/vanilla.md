---
id: handbook.vanilla
kind: handbook
title: Vanilla Server
category: java-servers
subtitle: "The official Mojang server — pure, simple, and without any add-ons."
analogy: "Vanilla is the raw, unmodified recipe straight from Mojang’s kitchen. No extra ingredients, no substitutions. It’s the same experience as singleplayer, just with multiple people in the same world at the same time."
relatedIds: [handbook.paper, handbook.purpur]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: vanillaContent}
---

A **Vanilla server** runs Mojang’s official server binary with no third-party modifications. No plugins, no mods, no performance patches. The game behaves exactly as it does in singleplayer.

**When to use Vanilla:**

### Bullet List

- You want a pure, unmodified Minecraft experience for a small group
- You’re testing Minecraft behavior without any interference from plugins or mods
- You want to match singleplayer exactly (same bugs, same quirks, same behavior)
- Simplicity is a priority — fewer moving parts, fewer things to break

### Body

**What’s missing compared to Paper:**

### Bullet List

- No plugin support — you cannot install Bukkit/Spigot/Paper plugins
- Fewer configuration options in server.properties (Paper exposes many extras)
- Slower performance under load compared to Paper’s optimizations
- Slower bug fixes than the Paper team’s rapid patching cadence

### Callout: note

Vanilla servers support full vanilla Java Edition clients and all vanilla gameplay features. If you later decide you want plugins, you’d need to create a new Paper server and copy the world folder over — the world data is compatible.

### In This App

- Create New Server → Standard → Vanilla.
- Vanilla doesn’t need a Paper template. MSC downloads the vanilla server JAR directly from Mojang for your chosen Minecraft version.
- Edit Server → JARs tab: shows the vanilla server JAR, read-only (no Update button — version changes go through the Versions picker).
- No EULA accept needed — MSC handles it automatically for Vanilla servers the same as Paper.

### Advanced Details

The Vanilla server JAR is downloaded directly from Mojang’s version manifest. Each Minecraft version has an exact JAR SHA1 that MSC verifies before using it.

Performance notes: Vanilla’s chunk loading and entity processing is noticeably slower than Paper under multi-player load. For 1–2 players casually exploring, this doesn’t matter. For 5+ players or redstone-heavy builds, Paper’s optimizations become meaningful.

MSC shows native world settings only when the selected Minecraft version and installed Java runtime can support them. The common 1.20+ profile includes generation choices, difficulty, game mode, gamerules, and Java's native creation options. A setting the runtime does not advertise is marked unavailable and is not sent as though Vanilla applied it; unrecognized values remain visible as unknown and are preserved.

Settings added by a datapack or another third-party component are **provided by this server/mod**, not part of MSC's universal world editor. Use the server's own configuration path for those values; MSC does not invent controls or silently drop them.
