---
id: handbook.first-server
kind: handbook
title: Your First Java Server
category: getting-started
subtitle: "A step-by-step checklist from zero to friends connecting on Java Edition."
relatedIds: [handbook.paper, handbook.eula-online-mode, handbook.port-forwarding-duckdns]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: firstServerContent}
---

### Callout: tip

You don't need to memorize this. Bookmark it and work through it step by step. You can always come back to other topics in this guide for deeper explanations of any step.

### Checklist: Phase 1 — Initial Setup

1. **Install Java** — Download Temurin 21 from adoptium.net. This is the Java runtime the server needs. After installing, verify it in Preferences → Check for Java.
2. **Run the Setup Wizard** — On first launch, the Setup Wizard appears. Choose a Servers Root folder (~/MinecraftServers is fine) and confirm your Java path.
3. **Download Paper templates** — Open Archives in the sidebar → Download latest Paper. This saves the newest Paper build as a reusable template.
4. **(Optional) Download plugin templates** — If you want Bedrock cross-play, open Archives → Download latest Geyser and Download latest Floodgate.

### Checklist: Phase 2 — Create Your Server

5. **Add a new server** — Click Manage Servers → Create New Server. Choose Standard → Paper. Give it a name, choose your Paper template, set RAM (2 GB min / 4 GB max is a good start), and enable Bedrock Cross-play if needed.
6. **Accept the EULA** — Go to the Details tab for your server and click Accept EULA. The server can't start until this is done.
7. **Configure basic settings** — In the Settings tab, set your MOTD (the message players see in the server list), max players, difficulty, and gamemode. Leave Online Mode ON.

### Checklist: Phase 3 — Go Online

8. **Start the server** — Click Start. Watch the console. When you see "Done (X.XX s)! For help, type /help", the server is ready.
9. **Test local connection** — Open Minecraft Java on the same Mac or another computer on your home network. Add a server using your Mac's local IP (e.g. 192.168.1.x) and port 25565. You should be able to connect.
10. **Enable external access** — Friends outside your home need one of two approaches — (A) Port Forwarding: log into your router and forward TCP port 25565 (and UDP 19132 for Geyser cross-play). Or (B) Playit.gg Tunneling: no router access needed; create a free Playit.gg account and enable the tunnel in Edit Server → Settings → Network. See Connection & Access for full step-by-step guides on both.
11. **(Optional) Set up DuckDNS** — Create a free hostname at duckdns.org and add it to Preferences. Share yourname.duckdns.org with friends instead of your raw IP.

### Checklist: Phase 4 — Stay Safe

12. **Create a backup** — Once everything works, open Server Details → Worlds and create your first backup. Label it "initial setup" or similar.
13. **Invite friends** — Share your DuckDNS hostname (or IP) and port with friends. Java players use the Multiplayer menu; Bedrock players with Geyser add a server in their Servers tab.

### Callout: note

Congratulations! If you made it through this checklist, you're hosting a Minecraft Java server. Check out the other topics in this guide whenever you want to understand something more deeply or add new features.
