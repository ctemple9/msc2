---
id: handbook.ram-performance
kind: handbook
title: RAM & Performance
category: concepts
subtitle: "How memory affects your Java server and what settings to use."
analogy: "RAM is like the size of your desk while you're working. The Minecraft world, players, and plugins all spread out on that desk. If the desk is too small, things fall off and the server lags or crashes. Too big, and your Mac's other apps have nothing left."
relatedIds: [performance.ram, performance.tps, health.ram]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: ramPerformanceContent}
---

RAM settings apply to Java servers only — Bedrock servers run in the built-in VM and manage their own memory. For Java servers, here are practical starting points:

### Table

| Players | Min RAM | Max RAM |
|---|---|---|
| 1–2 players | 2 GB | 4 GB |
| 3–5 players | 2 GB | 6 GB |
| 6–10 players | 4 GB | 8 GB |
| Plugins/mods | +1 GB | +2 GB |

### Body

**Modded servers need significantly more RAM.** The table above covers Standard servers (Paper, Purpur, Vanilla). Fabric and NeoForge/Forge packs load many more systems at startup and keep more data in memory during play.

Rough modded starting points:
• **Small Fabric pack (10–30 mods):** 3–5 GB max
• **Medium pack (30–100 mods):** 5–8 GB max
• **Large NeoForge/Forge pack (100+ mods):** 8–12 GB max

Check the modpack’s documentation — most published modpacks include server RAM recommendations.

### In This App

- Manage Servers → Edit → General tab: Min RAM and Max RAM sliders.
- These map to Java's -Xms (minimum heap) and -Xmx (maximum heap) flags.
- Setting min = max reduces GC pauses at the cost of memory always being reserved.
- TPS (ticks per second) in the console overview shows server health. 20 TPS = smooth. Below 15 = noticeable lag.

### Callout: tip

Leave at least 2–3 GB of RAM free for macOS and other apps. For a typical Paper server, 4–6 GB is the sweet spot. For modded servers, start with the modpack’s recommendation and adjust based on TPS and GC log entries.

### Advanced Details

Paper exposes many performance tuning options in paper.yml, bukkit.yml, and spigot.yml. These files are created in your server folder automatically and can be edited by hand for fine-tuning.
