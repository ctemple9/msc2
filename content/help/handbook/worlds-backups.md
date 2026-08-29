---
id: handbook.worlds-backups
kind: handbook
title: Worlds & Backups
category: server-management
subtitle: "Protect your world and understand how data is stored."
analogy: "A world is a save file folder. A backup is a snapshot of that folder at a specific moment in time. If you try a new plugin and it corrupts your world, a backup lets you roll back to exactly how things were before."
relatedIds: [handbook.world-conversion, handbook.server-transfer]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: worldsBackupsContent}
---

In your server folder, each world is stored as a group of folders:
- `world/` — the overworld
- `world_nether/` — the Nether
- `world_the_end/` — The End

The base name (`world`) comes from the `level-name` setting in `server.properties`. If you change it to `myserver`, the folders become `myserver/`, `myserver_nether/`, etc.

For Bedrock servers, world data is stored in a `worlds/` folder inside your server directory and is shared with the VM automatically via a direct file share.

**Always back up before:**
- Installing or updating plugins
- Changing major server settings
- Updating to a new Paper/Minecraft version
- Letting new players join for the first time

### Callout: pitfall

Never delete or rename world folders while the server is running. Always stop the server first, make your change, then restart.

### In This App

- Worlds tab: create, restore, and manage backups for each world slot, with a legacy/unmatched section when older backups do not map cleanly.
- Backups are stored as .zip files in a backups/ folder inside your server directory.
- Restore: unpacks a backup zip, replacing current world folders. Server must be stopped.
- Duplicate to new server: creates a fresh server directory from a backup — great for testing changes.
- World tools in the Worlds tab: Replace World (swap in a different world folder) and Rename World (safely updates level-name and folder names together).
- Automated rotating backups: the app can be configured to create backups on a schedule and automatically delete old ones.

### Callout: tip

Tip: use the Rename World tool instead of manually renaming folders. The tool updates both the folder names and the level-name setting in server.properties together, so they stay in sync.

### Advanced Details

Backups made by the app are standard zip archives. You can open them in Finder to inspect or extract specific files (like player data).

The automated rotating backup system (if enabled) creates a backup every N hours and keeps only the last X backups. This prevents backup folders from growing indefinitely.

If a world gets corrupted, you can sometimes recover it without a full restore using Minecraft's built-in /replaceitem or by editing the region files — but for most cases, restoring a recent backup is faster and safer.
