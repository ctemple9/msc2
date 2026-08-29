---
id: handbook.client-requirements
kind: handbook
title: Client Requirements
category: modded-java
subtitle: "Why every player needs the same mods — and how to make that easy."
analogy: "Joining a modded server is like joining a book club where everyone must read the same edition of the same book. If one person shows up with a different edition — missing chapters, renamed sections — the conversation doesn’t line up. The server enforces this by checking what every client has before letting them in."
relatedIds: [handbook.fabric, handbook.neoforge, handbook.forge]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: clientRequirementsModdedContent}
---

This is the most important practical difference between Standard and Modded servers.

**Standard servers (Paper, Purpur, Vanilla):** plugins run on the server only. Your friends connect with a completely unmodified Minecraft client — no downloads, no setup, just the game they already have.

**Modded servers (Fabric, NeoForge, Forge):** mods run on both the server and every client simultaneously. When a player tries to connect, the server and client exchange their mod lists during the handshake. If the lists don’t match, the player sees an error like “Mod list mismatch” or “Missing mods” and cannot connect.

**What each player needs to install:**

### Bullet List

- The correct Minecraft version (same as the server)
- The same mod loader (Fabric, NeoForge, or Forge) at the same version as the server
- All required mods — server-side-only mods are excluded from this requirement

### Callout: warning

Fabric and NeoForge mods are not interchangeable. A player with Fabric installed cannot join a NeoForge server, and vice versa. Everyone must use the same loader.

### Body

**How to share mods with your players:**

The easiest approach is to use MSC’s **Export for clients** feature in the Components tab. This generates either:
- A list of Modrinth links so players can download mods individually
- A ZIP of the required JAR files they can drop directly into their mods/ folder

Players can also use a modpack launcher like **Prism Launcher**, **ATLauncher**, or the official **Modrinth App** to install a mod bundle with a single click if you’ve set up a Modrinth modpack.

### Callout: tip

If setting this up for non-technical friends, a Modrinth modpack (exported from your server’s mod list) that they install in the Modrinth App is the lowest-friction option. One click installs everything they need.

### In This App

- Edit Server → Components tab → Export: generate client mod list.
- The export lists only mods required on both sides — server-side-only mods are automatically excluded.
- Players who use Prism Launcher can import the exported list as a local modpack.
- If a player gets a "mod list mismatch" error: compare their mod list to the server’s Mods tab and identify the difference.

### Advanced Details

Server-side-only mods are identified by their environment metadata. Fabric mods declare this in their fabric.mod.json: "environment": "server". NeoForge/Forge mods use the @Mod annotation and declare client-side dependencies.

MSC reads this metadata for Fabric mods automatically. For NeoForge/Forge mods, it uses Modrinth’s server_side field when the mod was downloaded through the Mod Browser.

Common server-side-only mods that do NOT need to be on clients:
• Spark (profiler)
• Chunky (pre-generation)
• LuckPerms-Fabric (permissions)
• Lithium, Ferrite Core, Krypton (performance, server-side effect only)
