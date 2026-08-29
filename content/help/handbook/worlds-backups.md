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

## World settings and server settings

**This world has its own gameplay and generation settings. They are saved with the world and applied whenever this world becomes active. Server settings such as ports and player limits apply to every world.**

That distinction follows where Minecraft actually stores and applies a value,
not which group an older editor placed it in. A slot's profile carries its
display name and level name, seed, generator/world type, flat preset,
structures, biome and generator options, bonus chest, and data packs. It also
carries the world's difficulty, default game mode, hardcore/commands state,
gamerules, and the edition-specific Bedrock choices: cheats, experiments,
coordinates, starting map, and supported gameplay toggles.

Java's `difficulty` and `gamemode` keys may still appear in a compatibility
settings response, but MSC treats the active values as the selected world's
profile and reads them back from that world. `force-gamemode` is different: it
is a server-wide policy that can override a world's default for joining
players. PvP, mob spawning, flight, Nether availability, spawn protection,
view/simulation distance, whitelist and ops, MOTD, authentication, ports,
player limits, runtime/process settings, crossplay, broadcast, and tunnels are
server-owned. On Bedrock, the same rule keeps ports, max players, online mode,
and default player permission out of the world profile.

MSC labels when a change takes effect. Seeds and generator choices are
creation-only; slot identity and data-pack selection are applied on
activation; values the active runtime can accept safely are live-safe; and
Bedrock cheats or experiments that require a stopped runtime are marked
restart-required. An imported or older world is never given a made-up value:
the profile can say detected, unknown, unsupported, or achievement-disabled.

### Advanced Details

The native profile covers the common settings that MSC can read, preserve, and
apply for Minecraft 1.20 and newer: identity, generation, difficulty, default
game mode, gamerules, and the Bedrock choices advertised by the selected
runtime. The form re-checks that capability when you change the Minecraft
version, Java flavor, loader, or edition. A setting the runtime does not
advertise is shown as unavailable and is not sent as though it applied.

Settings supplied by a particular server build, plugin, or mod are **provided by this server/mod**.
They are outside MSC's native world-profile contract. Use that server or mod's
own configuration path; MSC keeps unknown properties visible and preserved
instead of inventing controls or silently dropping them.

### Advanced Details

MSC asks for an explicit acknowledgement before a Creative or cheat change
that carries a safety consequence. On Bedrock, Creative, cheats, and some
experiments may permanently disable Xbox achievements for that world, even if
you later turn the setting off. The same warning is enforced by the agent for
the Worlds form, Quick Commands, the in-app console, the CLI, and the API, so a
different control surface cannot bypass it.

On Java, the warning explains the different consequence: Creative and command
settings change the world's gameplay and advancement/command behavior, but do
not claim the Bedrock Xbox-achievement consequence. The server-wide
`force-gamemode` policy has its own confirmation because it applies to every
world and can override each slot's saved default. It is off by default and is
never enabled by choosing Creative for one world.

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
