# MSC 2 — What It Is

**Revision:** 1.3 · **Date:** 2026-07-29 · **Owner:** Cameron Temple

This document describes MSC 2 in plain language: what it does, who it's for, and what using it feels like. No code, no architecture.

**Companion documents:**
- `MSC2-VISION.md` — index and precedence for this document set
- `msc2-engineering.md` — how it's built
- `msc2-decisions.md` — what was decided, by whom, and why
- `msc2-port-plan.md` — how it gets built

---

## In one sentence

MSC 2 lets you run a Minecraft server on a computer you own, and manage it from your phone, your laptop, a web browser, or a terminal — without ever learning what a JVM argument is.

---

## The problem it solves

Hosting Minecraft for your friends has two options, and both are bad.

**Realms** is easy and never stops charging you. You pay in the months nobody plays. You can't use most mods. You don't control your world.

**Self-hosting** is free and gives you everything — if you're comfortable with terminals, Java versions, server JARs, config files, port forwarding, mod loaders, process management, and reading crash logs at midnight because your nephew wants to play.

MSC turns the second option into an application. You click things. It explains what's wrong in sentences. Your world stays on your hardware.

MSC 2 does that on **any** computer you own — Mac, Windows, or Linux — and lets you manage it from anywhere.

---

## Who it's for

- Someone replacing a Realm
- A family running a shared world
- Friends playing a modpack together
- Someone with an old laptop who'd rather use it than replace it
- Someone curious about server administration who wants to learn without being punished for it
- Someone experienced who just wants reliable controls instead of repetitive manual work

MSC assumes you know Minecraft. It does not assume you know infrastructure.

---

## What's new in MSC 2

MSC 1 is a Mac application. It works well, and everything it does happens inside that window on that Mac.

MSC 2 splits into two halves: a small program that actually runs your server, and the interfaces you use to control it. That one change produces everything below.

**Your server keeps running when you close the app.** The window is a remote control, not the server. Quit it, log out, walk away — the server doesn't notice.

**Your server can live on a machine with no screen.** An old laptop with the lid shut, in a closet, plugged into ethernet. No desktop, no monitor, no keyboard. This matters more than it sounds: on an 8 GB machine, not running a graphical desktop environment can be the difference between a modpack that runs and one that stutters.

**You can manage it from anywhere.** Phone, laptop, browser, terminal — all talking to the same server, all able to reach the same capabilities.

**Windows and Linux work too.** Not a lesser version. The same application.

**Your phone stops being structurally limited.** In MSC 1, the iOS app could only do what had been separately built into it *and* separately exposed by the Mac — so it always trailed. In MSC 2 there is one copy of "restore a backup," and every interface calls it.

To be precise about what that does and doesn't promise: it means no capability is ever *unavailable* to the phone because the plumbing is missing. It does not mean every screen ships at once — someone still has to build each one. MSC 2 tracks that in a published capability list, so anything not yet on the phone is a recorded decision rather than a surprise.

---

## Where your server can live

Anywhere you want, and it can move.

- The Mac you're using right now
- A Windows PC
- A spare laptop running Linux with nothing else on it
- An old machine you were going to throw away

A server created on one machine can move to another. You aren't locked to the computer where you started.

The ideal setup for a demanding modpack is a machine that does nothing else — no desktop, no browser, no login screen — so all its memory goes to Minecraft. MSC 2 makes that setup practical without giving up the interface.

**And that works on every platform, not just Linux.** You can install MSC on a Mac, a Windows PC, or a Linux box *without the app at all* — just the part that runs servers, plus terminal commands. The graphical app is always optional. If you already keep a spare Mac running servers, it can stop running a desktop session entirely.

**A note on Bedrock.** Java servers run essentially anywhere MSC does. Bedrock is fussier — Mojang publishes it for specific systems, so the list of machines that can host a Bedrock server is narrower than the list that can run MSC. MSC tells you up front whether a machine can host Bedrock rather than letting you find out when it fails.

---

## The four ways to use it

All four talk to the same server and can reach the same capabilities. Pick whichever is closest to hand.

Some things are genuinely better suited to one surface than another — editing a large config file is nicer on a big screen than on a phone. Where a capability is deliberately left off a surface, that's recorded as a decision, not left as a gap.

**The desktop app.** Mac, Windows, or Linux. Looks and feels like MSC always has — dark, focused, a list of servers on the left, tabs across the top, the console always available at the bottom.

**A web browser.** Any device, no install. This is how you manage the screenless machine in the closet: it runs the server, your laptop or tablet just displays the interface.

**Your iPhone or iPad.** A real management app, not a status widget. Start and stop, watch the console, manage players and worlds, restore backups, install mods, fix problems.

**The terminal.** For automation and for people who like terminals. Every action available as a command, with proper output for scripts.

---

## What it feels like

### Setting up

You tell MSC what kind of server you want — Vanilla, Paper, Fabric, a modpack you downloaded — and which version of Minecraft. It handles the rest: finding the right server files, installing the correct version of Java, setting up folders, choosing safe memory settings.

It asks questions in Minecraft language. Not *"select a JVM heap allocation"* but *"how much of this computer's memory should the server be allowed to use?"* — with a recommendation, and a warning if you push it somewhere unsafe.

### Every day

You open MSC and the first screen answers four questions immediately:

1. Is it running?
2. Can people connect?
3. Is it healthy?
4. Does anything need attention?

Green means fine. Amber means look at this soon. Red means something's wrong. Anything flagged is clickable, and clicking it explains the problem and offers the fix.

### When something breaks

This is where MSC earns its place.

Minecraft servers fail in a small number of predictable ways: wrong Java version, a mod meant for the client installed on the server, a port already in use, the EULA not accepted, a mod missing something it depends on, a corrupt download, out of memory, out of disk.

MSC recognises these. Instead of a wall of red text, you get:

> **This server needs Java 21, but Java 17 is selected.**
> Minecraft 1.21 requires a newer Java than the one configured.
> **[Install Java 21]** · **[Choose a different runtime]** · **[Show the log]**

The raw log is always one click away. It's just not the first thing you see.

And MSC never says a repair worked until it has actually verified it.

---

## What you can do

### Servers

Create, import, start, stop, restart, and force-stop. Multiple servers on one machine, each with its own settings. MSC warns you before starting one that would use more memory than the machine safely has.

### Console

Live output as it happens, colour-coded so chat, joins, warnings, and errors are distinguishable at a glance. Search it, filter it, pause it, copy it, export it. Send commands, with history and saved favourites.

Closing the app and coming back doesn't lose the console — you get recent history, then live output.

### Players

Who's online, who's been on before, how long they've played, when you last saw them. Skins and avatars. Operator and allowlist status. Message, kick, ban, pardon, whitelist, promote.

If Bedrock cross-play is on, Java and Bedrock players appear together with their real identities.

Where information genuinely isn't available for a server type, MSC says so rather than inventing it.

### Worlds

Multiple worlds per server, switched whenever you like. Create, duplicate, rename, activate, archive, export, import, replace, delete. Repair worlds damaged by version changes. Convert between Java and Bedrock where supported.

Before anything destructive, MSC checks the paths and either offers or requires a backup — depending on how dangerous the operation is.

### Backups

Manual and scheduled. Automatically before risky operations. Retention by count, age, or storage limit. Every backup records what server and world it came from, when, why, how big, and whether it verified.

Restore to the current server, or as a new world slot so you can look before committing.

**MSC never calls something a backup until it has completed and verified.** A failed backup is reported as a failure, loudly.

### Mods, plugins, and modpacks

Browse Modrinth and plugin sources inside the app. Results are filtered to what actually works with your server's version and loader. Install, update, disable, remove, pin a version.

MSC installs required dependencies automatically, explains optional ones, and keeps client-only mods off the server.

Install a modpack from a Modrinth or CurseForge file. MSC works out what's needed server-side, downloads it, applies the pack's own overrides, and can produce a matching package or instruction list for your players.

It won't quietly change files a modpack is deliberately controlling — unless you tell it to take ownership.

### Settings

The things you actually change — difficulty, PvP, game mode, view distance, player limit, whitelist, seed, ports, memory — as ordinary controls with explanations.

MSC tells you which changes take effect immediately, which need a restart, and which could affect your world.

The raw config files are always available if you want them, and MSC never rewrites a file in a way that throws away settings it doesn't recognise.

### Performance

Both halves of the picture: how the computer is doing, and how Minecraft is doing. CPU, memory, swap, disk, and Minecraft's own tick rate and timing.

Numbers are shown, and then explained:

> *"The server is falling behind — the average tick is taking longer than 50 ms."*
> *"Java is using swap. Reduce the memory setting or close other programs."*
> *"Disk space is too low for the next scheduled backup."*

### Connections

How players reach you: local network, port forwarding with guidance for your specific router, Playit.gg tunnels, a DuckDNS hostname, Tailscale for private groups, Bedrock cross-play, Xbox friend discovery.

Addresses can be masked for screenshots and streams.

### Files

A file browser scoped to your server. View and edit config files, upload and download, rename, move, archive, delete. Critical paths are protected from accidents.

### Help

The Server Handbook is available inside every interface. Help is contextual — clicking a warning, a setting, or a performance number opens the explanation for that specific thing.

You should never need to leave MSC and search for terminology.

---

## Why memory matters so much here

This is the reason MSC 2 exists, so it's worth being plain about.

A big modpack wants around 5 GB of memory. A machine with 8 GB total, running a full graphical desktop, often can't safely give it that — the desktop itself, the browser you left open, and the login session take their share first. The server ends up squeezed, and squeezed servers stutter.

Take the desktop away and the same laptop becomes a genuinely capable server.

So MSC 2 is built to get out of the way:

- The part that runs your servers is small, and stays small even after running for weeks
- It never needs a graphical interface installed on the machine hosting the server
- It shows you honestly how much memory is installed, free, and in use — and what's actually available for Minecraft
- It recommends a safe amount, warns you before an unsafe one, and lets you override it if you know what you're doing
- It watches for the machine falling back on swap, which is the usual invisible cause of a laggy server, and tells you when that's happening

MSC treats "how much of this computer is left for Minecraft" as a number worth measuring, not a hope.

---

## Safety

MSC is infrastructure for worlds people care about. Reliability comes before convenience.

- Always tries a graceful stop before forcing anything
- Backs up automatically before destructive world operations
- Checks compatibility before installing
- Checks Java before launching
- Warns before unsafe memory settings — but lets you override, informed
- Writes configuration in a way that survives interruption
- Keeps a recovery copy before replacing files
- Requires confirmation, and the right permission, for destructive remote actions
- Keeps enough history to explain what happened and who did it

On a phone, high-risk actions can be protected with Face ID or your passcode.

---

## Privacy

MSC is local-first. Your worlds, player data, and configuration stay on hardware you control.

**There is no MSC account and there is no MSC server.** No sign-up, no subscription, no telemetry, no analytics, and nothing of yours is relayed through anything belonging to the people who make MSC. There is no service on the other end to send it to.

That is different from saying MSC never touches the internet. It does, for things you asked for:

- Downloading server software, Java, or mods
- Searching Modrinth or CurseForge
- Checking for updates
- Creating a Playit.gg tunnel
- Connecting through Tailscale, if you use it
- Updating a DuckDNS hostname, if you set one up
- Looking up a player's skin
- Checking whether your server is reachable from outside

These are useful third-party services and MSC will keep working with them. The interface always says which one is being contacted and why. What MSC will never do is require an account with *us*, phone home, or put your world behind a subscription.

---

## What MSC 2 deliberately does not do

Written down so it stays true.

- **No MSC-operated cloud service, ever.** No accounts, no hosting, no marketplace, no telemetry, no relay, no subscription. This is permanent. Optional third-party integrations — Tailscale, Playit, DuckDNS, Modrinth, CurseForge — stay fully supported; the rule is about *us* running a backend, not about MSC being offline.
- **Not a server network.** MSC manages servers on machines you own. It is not a proxy or multi-server network orchestrator.
- **Not a billing platform or a hosting business.**
- **No Android app.** iPhone and iPad only.
- **No third-party plugin system.** Not in v1.
- **No individual user accounts yet.** The existing access model continues unchanged — admin and guest roles, **named tokens with scoped permissions and expiry dates**, so you can already hand someone limited access. What's deferred is *human identity*: personal logins, invitations, and account recovery.
- **No terminal dashboard yet.** The command line works fully in v1; the full-screen terminal interface comes in a later release.
- **Minecraft 1.20 and newer.** Older versions may still work, but aren't tested or supported.
- **Bedrock only where Mojang supports it.** MSC runs on more machines than Bedrock servers do, and says so plainly rather than failing later.

---

## A day with MSC 2

An old MacBook sits closed on a shelf, plugged into power and ethernet. Nobody is logged in and no graphical session is running — it boots, starts MSC as a background service, and joins your private network. Nobody has looked at its screen in months.

You're on the bus. Your sister texts asking if the server is up.

You open MSC on your phone. The dashboard shows the modded server is stopped, last night's backup is healthy, the modpack has no unresolved dependencies, and there's enough memory free. You tap **Start**.

MSC checks Java, memory, ports, files, and the active world, then starts it. You watch progress on your phone. Thirty seconds later the state changes to **Running** and the console starts scrolling.

By the time you're home, four people are on. Your phone shows tick rate, memory, and the chat going past.

That evening you open the desktop app on your Mac. It connects to the same machine on the shelf and shows the same familiar layout — same servers, same tabs, same console. You install two mods and schedule a restart for 3 a.m.

You close the laptop. Nothing happens to the server. It's not your laptop's server. It's the one on the shelf, and it's been running the whole time.

---

## Why this exists

MSC was built to make hosting a server for friends and family less frustrating and less expensive. Not to turn a home Minecraft world into a product.

The interface can be powerful without being cold. Player faces, world thumbnails, readable connection cards, plain-language health messages, and the console always within reach — all of it keeps the application connected to the game it's managing.

MSC 2 is the same idea, running wherever you need it, with the computer spending its memory on Minecraft instead of on a desktop nobody is looking at.
