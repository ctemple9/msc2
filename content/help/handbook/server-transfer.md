---
id: handbook.server-transfer
kind: handbook
title: Server Import & Transfer
category: server-management
subtitle: "Move an existing server into the app or transfer it to a new Mac."
analogy: "Your server is a folder on disk — all the settings, world data, and plugins live there. Import just tells the app where that folder is. Transfer is copying that folder to a different computer and pointing the new app at it."
relatedIds: [handbook.worlds-backups, handbook.server-files]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: serverTransferContent}
---

If you have an existing Minecraft server — one you ran manually from Terminal, one you managed with another tool, or one you’re moving from a different Mac — you can bring it into Minecraft Server Controller without starting from scratch. Your world data, plugins, settings, and player lists all carry over.

**Import an existing server:**
Point the app at an existing server folder. MSC reads the configuration, detects whether it’s a Java or Bedrock server, and adds it to your server list. Nothing in the folder is changed during import.

**Transfer to a new Mac:**

### Bullet List

- On your old Mac: create a backup from the Worlds tab, or simply locate your server folder in Finder
- Copy the entire server folder to your new Mac — AirDrop, an external drive, or any file transfer method works
- On your new Mac: use File → Import Server (or the + button in the server list) and select the copied folder
- The app reads the existing configuration automatically

### In This App

- File → Import Server (or the + button in the server list): browse to an existing server folder to add it.
- The server folder must be self-contained — all config files and world data should be inside one directory.
- Java servers: your Paper JAR path may need updating after a move if the folder is now in a different location.
- Bedrock servers: BDS is downloaded automatically on first start after import if not already present.

### Callout: tip

The most reliable way to transfer a server is to create a backup zip from the Worlds tab first. The backup is a complete, self-contained archive that’s easy to move and easy to restore from.

### Callout: note

The app stores the path to each server folder but doesn’t move or copy files itself. If you relocate a server folder in Finder after importing, update the path in Manage Servers → Edit.
