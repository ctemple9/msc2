---
id: handbook.first-bedrock-server
kind: handbook
title: Your First Bedrock Server
category: getting-started
subtitle: "Create and start a Bedrock Dedicated Server in MSC 2."
relatedIds: [handbook.bedrock, handbook.how-bedrock-runs, handbook.playit, handbook.xbox-broadcast, handbook.worlds-backups]
---

Start here after installing MSC 2 and completing First-time Setup, whether you skipped the tour or need a reminder. Bedrock availability depends on the selected host; MSC shows Bedrock in the creator only when that host supports it. [How Bedrock Runs](help:handbook.how-bedrock-runs) explains the host-specific runtime.

### Checklist: Phase 1 — Create a Bedrock server

1. **Open the creator** — In the left sidebar, open the server drop-down, choose **Manage…**, click **Add Server…**, then choose **Start Fresh**. On **Configure**, select **Bedrock** and enter a name for the server.
2. **Review Bedrock options** — Leave **Bedrock Version** at `LATEST` unless you need a specific verified build, and set the player limit. Bedrock does not use a Java runtime. If you want friends to discover the world through their Xbox Friends tab, optionally enable [Xbox Broadcast](help:handbook.xbox-broadcast). It advertises the server; players still need a working connection path.
3. **Choose how friends connect** — On **Network**, choose [Port Forwarding](help:handbook.port-forwarding-duckdns) or [Tunnel (playit.gg)](help:handbook.playit). Bedrock uses a **UDP** port, normally 19132; forwarding only TCP will not work. MSC shows the local and public connection details when available.

### Checklist: Phase 2 — World and confirmation

4. **Set up the world** — On **World**, keep **New World** or choose a backup ZIP. Review its name, seed, difficulty, game mode, and other available Bedrock world settings. You can manage the world later from **Worlds**; see [Worlds & Backups](help:handbook.worlds-backups).
5. **Review and create** — The fresh Bedrock path goes from World to **Confirm**, without a Java Add-ons step. Check the version, UDP port, connection choice, and world, then click **Create Server**. MSC creates the server record and manages the Bedrock runtime for this host.

### Checklist: Phase 3 — Initiate and connect

6. **Initiate the first start** — Select the new server and click **Initiate** in the left sidebar. MSC handles the first-start runtime and service work and prompts for any Playit or broadcast setup you chose. Follow the progress and console until the server reports ready. Later starts use **Start**; no Java selection or separate Java EULA step is needed.
7. **Test a local connection** — Open Bedrock Edition on another device on the same network. Use the address and UDP port shown in **Overview** or **How to Connect**. Test the public address after the local connection works. Console players may need the Xbox Friends method described in [Xbox Broadcast](help:handbook.xbox-broadcast), depending on their platform.
8. **Make a backup** — Once the world works, open **Worlds** and create a backup before adding packs or changing important settings. [Worlds & Backups](help:handbook.worlds-backups) explains how to restore or switch worlds.
