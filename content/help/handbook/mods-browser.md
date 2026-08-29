---
id: handbook.mods-browser
kind: handbook
title: Mods & Mod Browser
category: modded-java
subtitle: "Find, download, and manage mods from within the app."
analogy: "The Mod Browser is like an app store built into MSC. Instead of going to a website, downloading a file, and manually placing it in a folder, you search inside the app, click download, and it ends up in the right place automatically."
relatedIds: [handbook.client-requirements, handbook.fabric]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: modsModBrowserContent}
---

**Mods** are `.jar` files that live in your server’s `mods/` folder. They are loaded when the server starts. Mods can add new blocks, items, dimensions, crafting systems, mobs, biomes, and fundamentally new gameplay loops.

The **Mod Browser** in the Components tab pulls from **Modrinth** — the primary open-source mod repository with tens of thousands of mods. The browser automatically filters results to show only mods compatible with your server’s Minecraft version and mod loader (Fabric, NeoForge, or Forge).

**Server-side vs client-side:** not all mods need to be installed on both sides.

### Bullet List

- Required on both sides: the most common category. Adds blocks, items, or gameplay that both the server and client must understand. Everyone installs these.
- Server-side only: the server runs these, but clients don’t need them. Examples: Spark (performance profiler), Chunky (chunk pre-generation), LuckPerms-Fabric (permissions).
- Client-side only: visual or audio mods that the server doesn’t run. Not relevant for server management.
- MSC shows a badge on each mod in the browser indicating which type it is.

### Body

**Modpack import:** MSC can import a Modrinth modpack file (`.mrpack`). The import wizard automatically downloads and installs all server-compatible mods from the pack.

**Update detection:** MSC compares the hash of each installed mod JAR against the latest version on Modrinth. If an update is available, it appears in the Components tab.

**Export for clients:** Once you’ve set up your mods, use the Export button in the Components tab to generate a list of required mods with Modrinth links — or a ZIP of the JAR files themselves. Share this with your players so they know exactly what to install.

### In This App

- Edit Server → Components tab → Mod Browser: available for Fabric, NeoForge, and Forge servers only.
- Search by name, or browse by category. Filter by server-side / client-side badge.
- Click a mod to see details, version history, and a Download button.
- Downloaded mods appear immediately in the Mods tab. No server restart needed to install — but the mod only loads on next server start.
- Mods tab: shows installed mods. Click Delete to remove a mod (requires server restart to take effect).
- Components tab → Export: generate client mod list or ZIP.

### Callout: note

The Mod Browser is not available for Standard servers (Paper, Purpur, Vanilla). Those servers use the Plugin tab instead. The two ecosystems — plugins and mods — are completely separate.

### Advanced Details

MSC uses Modrinth’s v2 API for mod search. Version filtering uses both game_versions and loaders parameters to show only mods the server can actually run.

Update detection works by computing the SHA512 hash of each installed JAR and comparing it to the hash of the latest version file on Modrinth. This is accurate for mods downloaded through MSC; manually-placed JARs may not have matching hashes.

.mrpack files are ZIP archives containing a modrinth.index.json with a manifest of mods (download URLs + SHA512 hashes) and an optional overrides/ folder with extra files. MSC reads the manifest, downloads mods marked as server or both, verifies hashes, and places them in the mods/ folder.
