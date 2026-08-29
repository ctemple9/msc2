---
id: handbook.purpur
kind: handbook
title: What is Purpur?
category: java-servers
subtitle: "Paper with hundreds of extra configuration options on top."
analogy: "If Paper is a car with a good engine and sensible controls, Purpur is the same car with a cockpit full of extra dials. Most of them start in the same position as Paper — but now you can adjust things Paper never exposed."
relatedIds: [handbook.paper, handbook.plugins-crossplay]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: purpurContent}
---

**Purpur** is a fork of Paper (via Pufferfish) that inherits everything Paper has — the same plugin ecosystem, the same performance improvements — and adds hundreds of extra configuration toggles that Paper doesn’t expose.

**All Paper plugins work on Purpur without modification.** If you’ve been running Paper, switching to Purpur is a JAR swap, not a migration.

Examples of what Purpur adds:

### Bullet List

- Per-entity behavior toggles (disable silverfish spawning, change zombie burn thresholds, set spider climb height)
- Per-material settings (how blocks behave in specific situations)
- Extra mob controls — aggression radius, spawn caps, entity-specific damage values
- Spectator and creative mode tweaks beyond Paper’s options
- Additional server gameplay adjustments (fall damage, hunger, sleep mechanics)

### Callout: note

Purpur’s extra options are all disabled or set to Paper-equivalent defaults out of the box. You get Paper behavior unless you intentionally change something.

### Callout: tip

Start with Paper. Switch to Purpur if you find yourself wanting to tune something that Paper’s configuration doesn’t expose.

### In This App

- Create New Server → Standard → Purpur.
- Purpur uses the same JAR template system as Paper. MSC downloads Purpur builds from purpurmc.org.
- Purpur’s extra settings are in purpur.yml inside the server folder (created on first run).
- Edit Server → JARs tab: shows the Purpur Server JAR with an Update button.

### Advanced Details

The Standard server family tree, with Purpur at the end:

  CraftBukkit → Spigot → Paper → Pufferfish → Purpur

Pufferfish adds CPU optimization patches on top of Paper. Purpur builds on Pufferfish and adds the configuration layer. In practice, Purpur vs. Paper performance on a small home server is negligible — the main reason to run Purpur is the configuration options, not the performance.

Purpur maintains API compatibility with Paper plugins. Any plugin that declares compatibility with Paper (Bukkit, Spigot, or Paper API) will run on Purpur.
