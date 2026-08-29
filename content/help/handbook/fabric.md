---
id: handbook.fabric
kind: handbook
title: What is Fabric?
category: modded-java
subtitle: "A lightweight mod loader known for fast updates and an active ecosystem."
analogy: "Fabric installs a minimal framework onto the Minecraft server — like adding a thin expansion slot to a circuit board. The slot adds nothing to the game itself, but mods can plug in cleanly. Because the framework is small and makes few assumptions, it’s easy to update when new Minecraft versions drop."
relatedIds: [handbook.client-requirements, handbook.mods-browser]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: fabricContent}
---

**Fabric** is a lightweight mod-loading framework. Unlike NeoForge, it makes minimal changes to Minecraft’s internals, which means it’s typically updated to support new Minecraft versions within days of release.

**Installation:** When you create a Fabric server, MSC downloads the Fabric installer and runs it automatically for your chosen Minecraft + loader version. The result is a `fabric-server-launch.jar` in your server folder. Installation takes about 10–15 seconds.

**Mods** live in the `mods/` folder inside your server directory. The Mods tab in Edit Server shows all installed mods with Delete buttons. Use the Mod Browser in the Components tab to search Modrinth and download mods directly.

### Callout: note

Many Fabric mods depend on Fabric API — a shared library that provides common utilities. MSC adds Fabric API automatically when you create a Fabric server.

### Body

**Loader version:** Each Fabric loader version is compatible with a specific range of Minecraft versions. MSC shows available loader versions in Edit Server → Components → Versions. Changing the MC version or loader version re-runs the installer automatically.

### Callout: warning

Fabric has no native plugin support. Paper/Bukkit plugins will not load on a Fabric server. Every player who connects must also have Fabric and the same mods installed on their client.

### In This App

- Create New Server → Modded → Fabric.
- Components tab → Versions: change Minecraft or Fabric loader version. MSC re-runs the installer.
- Components tab → Mod Browser: search Modrinth and download mods directly into the server.
- Edit Server → Mods tab: shows all installed mods with Delete buttons.
- Components tab → Export: generates a client mod list so players know what to install.

### Advanced Details

Quilt is a community fork of Fabric with additional features and a different governance model. MSC supports Quilt servers with the same setup flow as Fabric — Create New Server → Modded → Quilt. Most Fabric mods are compatible with Quilt, but check the mod’s documentation.

The Fabric loader version is different from Fabric API. The loader is the framework that loads mods at startup. Fabric API is a regular mod (JAR file) that provides shared utilities. You need both, but MSC manages them separately: the loader version is in the Versions picker, and Fabric API is a mod in your mods/ folder.
