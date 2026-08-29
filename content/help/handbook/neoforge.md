---
id: handbook.neoforge
kind: handbook
title: What is NeoForge?
category: modded-java
subtitle: "The modern successor to Minecraft Forge, with a large modpack ecosystem."
analogy: "NeoForge is like a full engine rebuild that exposes thousands of internal hooks for mod developers. It’s more invasive than Fabric, but that depth enables mods that fundamentally change how Minecraft works — full tech trees, magic systems, custom dimensions, and entire modpacks built around new gameplay loops."
relatedIds: [handbook.forge, handbook.client-requirements]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: neoforgeContent}
---

**NeoForge** is the actively-maintained community fork of the original Minecraft Forge project (forked in 2023). It makes extensive changes to Minecraft’s internals, which is what enables mods to add deep gameplay systems — but it also means updates to new Minecraft versions take longer than Fabric (usually weeks, sometimes months).

**Installation:** NeoForge uses an installer-based setup. MSC downloads the NeoForge installer JAR and runs it automatically. The installer remaps Minecraft’s code and generates a `libraries/` folder and a `run.sh` launch script. This process takes **30–90 seconds** depending on download speed — longer than Fabric, but only happens once per version.

MSC reads the generated launch script and runs it for you. You never need to interact with it directly.

### Callout: note

The first start of a NeoForge server can take 2–3 minutes as it remaps Minecraft’s code. Subsequent starts are normal speed. The console will show progress during this phase.

### Body

**Mods** live in the `mods/` folder. The Mods tab in Edit Server shows installed mods. Use the Mod Browser in the Components tab to search Modrinth and download compatible mods.

**Client requirement:** Every player who joins must have NeoForge at the same version installed on their Minecraft client, plus all required mods. See the Client Requirements topic for how to share this with your players.

### Callout: tip

For new servers on Minecraft 1.21+, NeoForge is the recommended choice over Forge. Most new large modpacks target NeoForge. Only choose Forge if your specific modpack requires it.

### In This App

- Create New Server → Modded → NeoForge.
- Components tab → Versions: change NeoForge or Minecraft version. Re-runs the installer.
- Components tab → Mod Browser: search and download compatible mods.
- Edit Server → Mods tab: shows all installed mods.
- Components tab → Export: generate a client mod list for your players.

### Advanced Details

The NeoForge installer generates:
  libraries/        — remapped Minecraft classes + NeoForge dependencies
  run.sh            — shell script that builds the full classpath and launch args
  @user_jvm_args.txt — JVM flags (MSC injects -Xms/-Xmx here)

MSC reads run.sh to extract the launch command, injects your RAM settings, and runs the server process. When you change the NeoForge version, MSC deletes the old libraries/ and re-runs the installer to rebuild them.

NeoForge loader versions look like 21.1.172 — the first two numbers match the Minecraft minor version (21.1 = MC 1.21.1). Each MC version has many NeoForge builds; MSC’s version picker shows all available builds from the NeoForge version manifest.
