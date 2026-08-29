---
id: handbook.player-management
kind: handbook
title: Player Management
category: server-management
subtitle: "Control who can connect, what they can do, and how they appear in the app."
analogy: "Your server is like a private club. The allowlist is the guest list — only people on it can get in. Ops are the staff — they can do things regular members can’t. Banning is a permanent removal from the premises. These are independent systems you can use in any combination."
relatedIds: [handbook.bedrock, handbook.eula-online-mode]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: playerManagementContent}
---

**Allowlist (Whitelist)**

The allowlist restricts who can join your server. When enabled, only players whose usernames appear on the list can connect — anyone else gets a “Not whitelisted” rejection.

Use this for private servers — family servers, friend groups, or any server where you don’t want strangers joining. It’s disabled by default.

**Operators (Ops)**

Ops are trusted players with elevated command permissions. They can use commands regular players cannot — like /gamemode, /tp, /give, and player management commands. Java servers support four op levels:

### Bullet List

- Level 1: bypass spawn protection only
- Level 2: gameplay commands (/give, /tp, /gamemode, /time, /weather, etc.)
- Level 3: player management commands (/kick, /ban, /whitelist)
- Level 4: full server control including /stop and /op — use sparingly

### Body

**Banning**

Banning prevents a specific player from connecting even if they’re on the allowlist. Java servers track bans by UUID (account ID) so name changes don’t bypass them. Bedrock servers track by XUID.

### In This App

- Players tab: see all online players with their head renders, health, and status. Right-click a player (or use the action menu) for op, deop, kick, ban, message, teleport, and whitelist actions.
- Overview → Players strip: scrolling head grid of online players. Tap a head to feature the player; tap their character render for quick actions.
- Overview → Players strip: right-click a head to hide a player from the overview grid. Right-click empty space to show hidden players again.
- Edit Server → Players tab: manage the allowlist and op list without typing commands in the console.
- Bedrock players joining via Geyser appear with a period prefix (e.g. .PlayerName) — this is Floodgate’s identifier so they can be distinguished from Java Edition players.

### Callout: tip

For a typical friends server: enable the allowlist and add each friend’s username, set yourself and at least one trusted friend as ops (level 2 is usually enough), and leave online mode on.

### Advanced Details

Java servers store player data in these files inside the server directory:
  whitelist.json — the allowlist by UUID and username
  ops.json — operator list with UUIDs and op levels
  banned-players.json — banned players by UUID
  banned-ips.json — IP-based bans

Bedrock servers use:
  allowlist.json — the allowlist with platform XUIDs
  permissions.json — operator and visitor permission levels

Changes made in the Players tab are written directly to these files. Paper reloads allowlist and ops changes dynamically — they take effect on a running server without a restart.

Floodgate assigns Bedrock players a Java UUID starting with 00000000-0000-0000-0009-... which is how whitelist.json and ops.json can track them even though they don’t have Java Edition accounts.
