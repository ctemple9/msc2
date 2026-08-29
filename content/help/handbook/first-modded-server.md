---
id: handbook.first-modded-server
kind: handbook
title: Your First Modded Server
category: getting-started
subtitle: "A step-by-step checklist for getting a Fabric or NeoForge server running."
relatedIds: [handbook.fabric, handbook.client-requirements, handbook.mods-browser]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: firstModdedServerContent}
---

### Callout: tip

Not sure which loader to pick? Fabric is the easier starting point: faster setup, snappier updates, and less RAM overhead. Choose NeoForge if your friends have a specific NeoForge modpack they want to play.

### Checklist: Phase 1 — Before You Create the Server

1. **Install Java 21** — Modded servers require Java, same as Standard Java servers. Download Temurin 21 from adoptium.net. Verify it in Preferences → Check for Java.
2. **Decide: Fabric or NeoForge?** — Fabric: simpler, faster to set up, lighter RAM usage, great ecosystem. NeoForge: larger ecosystem, required for most big modpacks (1.21+). If you have a specific modpack in mind, check which loader it requires — the pack page will say.
3. **(Optional) Find your modpack** — Browse modrinth.com for modpacks. Filter by server-side support. Note the Minecraft version and loader. You’ll need this information when creating the server.

### Checklist: Phase 2 — Create the Server

4. **Create a new server** — Click Manage Servers → Create New Server → Modded. Choose Fabric or NeoForge, then select your Minecraft version and loader version. Set RAM: at least 4 GB min / 6 GB max for a small Fabric pack; 6 GB min / 10 GB max for a NeoForge pack.
5. **Wait for installation** — Fabric installs in about 10–15 seconds. NeoForge takes 30–90 seconds — it’s downloading and remapping Minecraft’s code. The console shows progress. Don’t interrupt it.
6. **(Optional) Import a modpack** — If you have a .mrpack file: in the Components tab, use Import Modpack to install all server-compatible mods automatically. If installing mods manually, continue to Phase 3.

### Checklist: Phase 3 — Install Mods

7. **Open the Mod Browser** — Edit Server → Components tab → Mod Browser. Search for mods by name. The browser only shows mods compatible with your server’s MC version and loader. Click Download to add a mod.
8. **Check Fabric API (Fabric servers only)** — Most Fabric mods depend on Fabric API. MSC adds it automatically, but verify it’s in the Mods tab. If it’s missing, search “Fabric API” in the Mod Browser and download it.
9. **Start the server once to test** — Click Start. The server loads all mods at startup — watch the console for errors. A successful Fabric start ends with “Done!”. NeoForge takes longer on first start due to remapping; progress shows in the console. Stop the server after confirming it starts cleanly.

### Checklist: Phase 4 — Set Up Your Players

10. **Export the client mod list** — Edit Server → Components tab → Export. This generates a list of required mods. Share it with your players. Each player needs to install the same Minecraft version, same loader, and the same mods.
11. **Have each player install the mods** — Easiest: point them to Prism Launcher or the Modrinth App and import the mod list. Manual: they download each mod JAR and place it in their .minecraft/mods folder. Each player must also have the correct mod loader installed (Fabric Installer or NeoForge Installer from their respective websites).

### Checklist: Phase 5 — Go Live

12. **Set up external access** — Same as a Standard server: port forwarding (TCP 25565 on your router) or Playit.gg tunneling (Edit Server → Settings → Network). See Connection & Access in this handbook.
13. **Test with one player first** — Have one friend test the connection before inviting everyone. Confirm they can load into the world and that the mods are working. A mod mismatch shows as an error during the connection handshake — compare the Mods tab with what the player has installed.
14. **Create a first backup** — Once everything works, open Server Details → Worlds and create your first backup. Modded worlds can be harder to recover from corruption, so back up before installing new mods or updating.

### Callout: note

Congratulations! Modded servers take more setup than Standard servers, but the result is a much richer gameplay experience. Check the Mods & Mod Browser and Client Requirements topics in this handbook whenever you need more detail on any step.
