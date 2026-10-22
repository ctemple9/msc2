---
id: handbook.first-modded-server
kind: handbook
title: Your First Modded Server
category: getting-started
subtitle: "Create a Fabric, NeoForge, or Forge server with MSC 2."
relatedIds: [handbook.fabric, handbook.neoforge, handbook.forge, handbook.client-requirements, handbook.mods-browser]
---

This guide assumes MSC 2 and First-time Setup are already complete. A modded server needs a matching Minecraft version, loader, and usually mods on every player's computer. MSC handles the server installation, but players still need compatible client files. See [Client Requirements](help:handbook.client-requirements) before inviting friends.

### Checklist: Phase 1 — Choose the server

1. **Open the creator** — In the left sidebar, open the server drop-down, choose **Manage…**, click **Add Server…**, then choose **Start Fresh**. On **Configure**, choose **Java**, give the server a name, and choose **Modded** under Server Software.
2. **Choose the loader and version** — Select [Fabric](help:handbook.fabric), [NeoForge](help:handbook.neoforge), or [Forge](help:handbook.forge), then choose the offered Minecraft and loader version under **Source**. If you plan to use a particular modpack, match the loader and Minecraft version listed by that pack. The loader pages explain the differences.
3. **Understand cross-play** — The creator does not offer Geyser/Floodgate or Xbox Broadcast for a modded Java server. Bedrock clients cannot load the Java mods. If Bedrock friends need to join, consider a separate standard Java server with [Geyser and Floodgate](help:handbook.plugins-crossplay).

### Checklist: Phase 2 — Runtime and network

4. **Select Java for this server** — After Configure, MSC tells you which Java runtime this Minecraft version needs. Select the recommended installed runtime, or click **Install Java…** and let MSC install it. Modded servers do not all use the same Java major version.
5. **Choose a public connection** — On **Network**, choose [Port Forwarding](help:handbook.port-forwarding-duckdns) if you can configure your router, or [Tunnel (playit.gg)](help:handbook.playit) to use MSC's managed tunnel. Review the Java TCP port. You can test locally before inviting anyone from outside your network.

### Checklist: Phase 3 — World and mods

6. **Choose the world** — On **World**, use **New World** or import a backup ZIP. Review the name, seed, difficulty, game mode, and other available world settings. [Worlds & Backups](help:handbook.worlds-backups) explains later changes and recovery.
7. **Stage mods** — On **Add-ons**, browse Modrinth for mods compatible with the selected loader and Minecraft version, or add a local JAR. The [Mods Browser](help:handbook.mods-browser) topic explains how MSC finds compatible downloads. You can also add mods after creation from **Components**. If you have a supported modpack archive, the creator's separate **Import or Create from Modpack** path may be easier than Start Fresh.
8. **Review and create** — On **Confirm**, verify the loader, version, network, world, and staged mods, then click **Create Server**. MSC installs the server software and applies the staged items; installation may take longer than a standard Java server. Wait for completion before selecting the new server.

### Checklist: Phase 4 — Start and share

9. **Set memory** — Click the server name or use **Manage… → Edit** to review Java memory settings. Give a modded server enough RAM for its pack while leaving memory for the operating system and any local Minecraft client. See [RAM & Performance](help:handbook.ram-performance).
10. **Initiate the first start** — Click **Initiate** in the left sidebar. MSC handles the EULA and first-start setup; follow any prompts and watch the console for missing dependencies or version errors. When the server is ready, later launches use **Start**.
11. **Prepare players and protect the world** — Test with a client using the same Minecraft version, loader, and required mods. Share the mod list or pack details with friends; [Client Requirements](help:handbook.client-requirements) explains mismatches. Once it works, make a backup in **Worlds** before changing the mod set.
