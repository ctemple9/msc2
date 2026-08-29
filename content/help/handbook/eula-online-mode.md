---
id: handbook.eula-online-mode
kind: handbook
title: EULA & Online Mode
category: java-servers
subtitle: "The two must-know settings before your Java server can start."
analogy: "The EULA is the rental agreement for using the server software — you must sign it before you can move in. Online mode is the ID check at the door — it verifies that everyone who tries to enter actually owns a ticket."
relatedIds: [settings.online-mode, handbook.first-server]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: eulaOnlineModeContent}
---

**EULA** (End User License Agreement) is Mojang's legal agreement for running their server software. The first time your server starts, it creates a file called `eula.txt` with `eula=false`. The server will refuse to fully boot until that becomes `eula=true`.

This is required by Mojang. The app makes it simple — you just click a button.

### Callout: pitfall

If you see "You need to agree to the EULA" in the console and the server stops immediately, your EULA hasn't been accepted yet. Use the Details tab → Accept EULA.

### Body

**Online Mode** (`online-mode` in `server.properties`) controls whether the server verifies player accounts with Mojang's authentication servers.

- `online-mode=true` (recommended): Every player's account is verified as legitimate. Standard and secure.
- `online-mode=false`: No account verification. Anyone can join with any username — often called a "cracked" server.

**Important for Geyser/Floodgate users:** If you're using Floodgate for Bedrock players, keep `online-mode=true`. Floodgate handles Bedrock authentication separately.

### Callout: warning

Setting online-mode=false disables account verification, allows unverified clients, and can break plugins that depend on real player UUIDs. Only do this if you have a specific reason.

### In This App

- Details tab → EULA section: Accept EULA button writes eula=true for you.
- Settings tab → server.properties editor: Online Mode toggle (ON = true, OFF = false).
- The app will warn you if you try to start a server with an unaccepted EULA.
