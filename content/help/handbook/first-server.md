---
id: handbook.first-server
kind: handbook
title: Your First Java Server
category: getting-started
subtitle: "Create, start, and manage a standard Java server in MSC 2."
relatedIds: [handbook.paper, handbook.plugins-crossplay, handbook.playit, handbook.worlds-backups]
---

This guide starts after MSC 2 is installed and First-time Setup is finished. It also works if you skipped the tour or are returning to server creation later. You can leave this page open and work through the phases in the app.

### Checklist: Phase 1 — Open the server creator

1. **Open Manage Servers** — In the left sidebar, click the server drop-down, then **Manage…**. In Manage Servers, click **Add Server…** and choose **Start Fresh**. This starts the six-step creator; you do not need to make folders or download a server JAR yourself.
2. **Choose Java and name the server** — On **Configure**, choose **Java** and enter a name you will recognize in the server list. Under Server Software, choose **Standard**.
3. **Choose the server software** — Pick [Paper](help:handbook.paper), [Purpur](help:handbook.purpur), or [Vanilla](help:handbook.vanilla). Paper is a practical first choice if you want plugins. The linked pages explain what each flavor changes. Choose the Minecraft version under **Source**; the latest version is offered by default.
4. **Choose who can join** — For Paper or Purpur, **Enable Bedrock Cross-play** adds [Geyser and Floodgate](help:handbook.plugins-crossplay) so Bedrock players can join. Vanilla has no plugin support, so this option is unavailable. If you want your Bedrock friends to see the server in their Xbox Friends tab, you can also enable [Xbox Broadcast](help:handbook.xbox-broadcast). Both are optional.

### Checklist: Phase 2 — Choose Java and connection

5. **Select Java for this server** — When you continue from Configure, MSC checks which Java runtime the selected Minecraft version needs. If a suitable runtime is already installed, select the recommended one. If not, use **Install Java…** in the chooser and let MSC install the version it recommends. You do not need to guess a Java version before reaching this step.
6. **Choose how friends connect** — On **Network**, choose [Port Forwarding](help:handbook.port-forwarding-duckdns) if you will configure your router, or [Tunnel (playit.gg)](help:handbook.playit) if you want MSC to arrange a public address without router changes. Review the Java TCP port and, if cross-play is on, the separate Bedrock/Geyser UDP port. MSC shows the relevant addresses once the connection is ready.

### Checklist: Phase 3 — World and add-ons

7. **Set up the world** — On **World**, keep **New World** for a new server or choose the backup ZIP option if you have one. Give the world a name and review its seed, difficulty, game mode, and other offered settings. You can adjust world settings later in the Worlds tab; see [Worlds & Backups](help:handbook.worlds-backups).
8. **Add plugins if you want them** — On **Add-ons**, browse for compatible plugins or add a local JAR. You can skip this and use the Components tab later. If you enabled cross-play, MSC handles Geyser and Floodgate; you do not need to add them manually.
9. **Review and create** — On **Confirm**, check the server type, version, ports, world, and optional features, then click **Create Server**. MSC creates the server and downloads its software. When creation finishes, select the new server in the sidebar if it is not already selected.

### Checklist: Phase 4 — Give it memory and initiate

10. **Set RAM** — Click the server name or open **Manage… → Edit**, then review the server's Java memory settings. Pick an amount that leaves enough memory for macOS and Minecraft on the same computer. [RAM & Performance](help:handbook.ram-performance) explains the tradeoff.
11. **Initiate the first start** — Click **Initiate** in the left sidebar. This first-start flow handles the EULA and checks the services you selected, including cross-play, Playit, and broadcast. Follow any sign-in or access prompts. Watch the status and console until MSC reports that the server is ready. Later starts use **Start**.
12. **Connect and make a backup** — Test the local Java address shown in **Overview** or **How to Connect** before sending a public address to friends. If Bedrock cross-play is enabled, test its Bedrock address too. Once the world is working, open **Worlds** and make a first backup; [Worlds & Backups](help:handbook.worlds-backups) explains how to restore it.
