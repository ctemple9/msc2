---
id: handbook.first-bedrock-server
kind: handbook
title: Your First Bedrock Server
category: getting-started
subtitle: "A step-by-step checklist from zero to friends connecting on Bedrock Edition."
relatedIds: [handbook.bedrock, handbook.how-bedrock-runs, bedrock.runtime-unavailable]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: bedrockSetupContent}
---

### Callout: tip

You don't need to memorize this. Work through it step by step. See the "How Bedrock Runs" topic in the Bedrock section if you want to understand how MSC runs BDS natively.

### Checklist: Phase 1 — Initial Setup

1. **Open the app** — Launch Minecraft Server Controller. Bedrock runs in a built-in VM — no Docker, no extra installs. The app is self-contained.
2. **Run the Setup Wizard** — On first launch, the Setup Wizard appears. Choose a Servers Root folder (~/MinecraftServers is fine). Select Bedrock as your server type. No Java path needed — no extra installs needed.

### Checklist: Phase 2 — Create Your Server

4. **Create a new Bedrock server** — Click Manage Servers → Create New Server → select Bedrock. Set a display name, port (19132 is the default), max players, difficulty, and gamemode. Click Create.
5. **Review server settings** — In the Settings tab, confirm your MOTD, max players, and difficulty. No EULA step needed for Bedrock — BDS handles it automatically on first run.

### Checklist: Phase 3 — Go Online

5. **Start the server** — Click Start. On first launch, the app downloads BDS automatically — this may take a moment depending on your internet speed. Watch the console. When you see "Server started", it's ready.
6. **Test local connection** — Open Minecraft on your phone, tablet, or another device on your home network. Go to Servers → Add Server. Enter your Mac's local IP (e.g. 192.168.1.x) and port 19132. You should be able to connect.
7. **Enable external access** — Friends outside your home need one of two approaches — (A) Port Forwarding: forward UDP port 19132 on your router to your Mac’s local IP. Bedrock requires UDP, not TCP — forwarding TCP 19132 will not work. Or (B) Playit.gg Tunneling: no router access needed; no extra installs needed. Enable the tunnel in Edit Server → Settings → Network. See Connection & Access for full guides on both.
8. **(Optional) Set up DuckDNS** — Create a free hostname at duckdns.org and add it to Preferences. Share yourname.duckdns.org with friends instead of your raw IP.

### Checklist: Phase 4 — Stay Safe

9. **Create a backup** — Once everything works, open Server Details → Worlds and create your first backup. Label it "initial setup" or similar.
10. **Invite friends** — Share your DuckDNS hostname (or public IP) and port 19132. Friends add it as a custom server in Bedrock's server list (Settings → Servers → Add Server). Bedrock cross-play is built in — mobile, console, and Windows 10/11 players can all join.

### Callout: note

Congratulations! You're hosting a native Bedrock server. All Bedrock Edition platforms can join — no plugins or translators needed. Check out the other topics in this guide for backups, world management, and remote access.
