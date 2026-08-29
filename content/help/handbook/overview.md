---
id: handbook.overview
kind: handbook
title: Overview
category: concepts
subtitle: "What this app is and what it can do for you."
analogy: "Normally in Minecraft, your world lives inside one player's game. A server is like a separate, always-available room that everyone can visit. Minecraft Server Controller is the manager who sets up and runs that room on your Mac — whether it's a Java room or a Bedrock room."
relatedIds: [handbook.first-server, handbook.standard-vs-modded]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: overviewContent}
---

Minecraft Server Controller is a native macOS app that gives you a clean interface for running Minecraft servers. Instead of typing commands in Terminal, you get buttons, toggles, and visual feedback.

The app supports a wide range of server types as first-class citizens:

**Standard Java servers — Paper, Purpur, and Vanilla.** Great for plugin-heavy setups, Bedrock cross-play via Geyser, and Java Edition players. Players connect with no extra setup on their end.

**Modded Java servers — Fabric, NeoForge, and Forge.** For mods that add new blocks, items, dimensions, and gameplay systems. Every player must install the same mod loader and mods to connect.

**Bedrock Dedicated Server (BDS).** The native server for mobile, console, and Windows 10/11 players. Runs in a built-in lightweight VM — no extra software required. Cross-play with all Bedrock platforms is built in.

### In This App

- Start and stop any server type with a single click — Java Standard, Modded, and Bedrock
- Watch the live console — see exactly what your server is doing
- Manage multiple servers from one window — mix any combination of server types
- Browse and download mods from Modrinth; browse and download plugins from the Components tab
- Configure ports, RAM, cross-play, and Playit.gg tunneling
- Handle backups, world conversions, and server transfers
- Manage world slots, resource packs, and player allowlists
- Monitor live performance — TPS, CPU, RAM, player health, and in-game time
- Remote control from iOS via MSC Remote (companion app)
- Watchdog crash recovery keeps your server running overnight

### Callout: tip

Starting fresh? Check the Getting Started section for step-by-step checklists: Your First Java Server (Paper), Your First Modded Server (Fabric/NeoForge), or Your First Bedrock Server. Come back to other topics for deeper explanations of anything along the way.

### Advanced Details

Under the hood, Standard Java servers (Paper/Purpur/Vanilla) run a shell command like:
  java -Xms2G -Xmx4G -jar paper.jar

Fabric modded servers launch from a generated launcher JAR:
  java -jar fabric-server-launch.jar

NeoForge and Forge modded servers use a generated shell script that passes an @args file to Java — the installer sets all of this up, and MSC runs the resulting script automatically.

Bedrock servers run in a lightweight Linux VM bundled with the app — no Docker or external software needed. The app manages the VM lifecycle entirely — start, stop, console streaming, world file sharing — so you never need to open any external tool.

All server types are fully managed. The complexity lives inside the app, not in front of you.
