---
id: handbook.forge
kind: handbook
title: What is Forge?
category: modded-java
subtitle: "The original Minecraft mod loader — still used for older modpacks."
analogy: "Forge is the original workshop where Minecraft modding was invented. NeoForge is a renovated version of the same workshop by a newer team. They use the same tools and techniques, but NeoForge gets the ongoing maintenance."
relatedIds: [handbook.neoforge, handbook.client-requirements]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: forgeContent}
---

**Minecraft Forge** is the original server-side mod loader, with roots going back to early Minecraft versions. It built the foundation for the modded ecosystem and still has an enormous library of mods — particularly for older Minecraft versions like 1.12.2, 1.7.10, and earlier.

In 2023, the **NeoForge** project forked from Forge to address development and governance concerns. For new servers on Minecraft 1.21 and later, NeoForge is generally the better choice. Forge remains the right option for:

### Bullet List

- Modpacks that specifically require Forge (check the pack’s documentation)
- Older Minecraft versions (1.20 and earlier) where Forge has better mod coverage
- Existing Forge servers you’re importing into MSC

### Callout: note

Forge and NeoForge are not interchangeable. A modpack built for Forge requires a Forge server. A modpack built for NeoForge requires a NeoForge server. Check which loader your modpack targets before creating your server.

### Body

**Installation** uses the same process as NeoForge: MSC downloads the Forge installer, runs it to generate the `libraries/` folder and launch script, and starts the server from the generated script. First-time setup takes 30–90 seconds.

**Client requirement:** Every player must have Forge (same version) and the same required mods installed. The client requirement works exactly the same as NeoForge.

### In This App

- Create New Server → Modded → Forge.
- Components tab → Versions: change Forge or Minecraft version. Re-runs the installer.
- Components tab → Mod Browser: search and download compatible mods from Modrinth.
- Edit Server → Mods tab: shows all installed mods with Delete buttons.

### Advanced Details

The Forge installer generates the same libraries/ + run.sh structure as NeoForge. MSC handles them identically.

Forge version numbers look like 1.21-51.0.33 — the first part is the Minecraft version, the second is the Forge build number. MSC’s version picker pulls available builds from Forge’s Maven repository.

For very old Minecraft versions (1.12.2 and earlier), Forge is the only option — NeoForge doesn’t support pre-1.20 versions. These older packs may have unusual startup behaviors; the MSC startup crash diagnostics apply the same way.
