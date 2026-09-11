# MSC 2

Built by ctemple9

> **A free, self-hosted way to manage Minecraft servers.**
>
> Whether your players use Bedrock on Xbox, PlayStation 5, Nintendo Switch,
> Windows, or mobile, Java on PC, or a combination of both, MSC 2 is being
> built to manage the server behind that group.

MSC 2 runs Minecraft servers on a computer you own and gives you a desktop
app, desktop browser, or terminal interface to manage them. Running a server
still involves server files, Java settings, network ports, and configuration.
MSC 2 puts as much of that as possible into guided controls and explains the
rest as you go, so you can learn what you need without having to master
everything first.

MSC 2 is **currently an unsigned prerelease**. The current release supports:

- Intel and Apple Silicon macOS
- 64-bit Windows
- 64-bit Linux

Apple Silicon macOS can manage Java servers locally and remote hosts, but its
local Bedrock VM is unavailable because that VM runs only on Intel Macs. The
release is unsigned, so macOS, Windows, or Linux may show a security warning
the first time you open or install it.

[Download MSC 2 v0.1.4](https://github.com/ctemple9/msc2/releases/tag/v0.1.4)

## What MSC 2 does

### Manages Java and Bedrock servers

MSC 2 helps you create, install, start, stop, and maintain both major
Minecraft server editions:

- **Java Edition:** Vanilla, Paper, Purpur, Fabric, NeoForge, and Forge
- **Bedrock Edition:** Bedrock Dedicated Server where Mojang supports it (no Docker needed!)

MSC 2 manages the server, not the Minecraft game installed on each player's
device. A player can join from a computer, Xbox, PlayStation 5, Nintendo
Switch, or mobile device when the server and connection method support it.

### Supports mixed Java and Bedrock groups

Java and Bedrock players normally use different server systems. MSC 2 can set
up the tools that allow them to play together, including Geyser and Floodgate,
where the selected server version and platform support that setup. To be clear, 
these tools allow for Bedrock players to join Java (Paper) worlds, not the other
way around. (Java is better though, try to get everyone on Java anyway haha).

That means you can run a world for a group where one person plays Java on a
computer, someone else plays Bedrock on an Xbox, and someone else joins from a
Switch. 

### Keeps your worlds safe

MSC 2 treats worlds as something you manage, not just folders on a disk. You
can create, import, duplicate, rename, activate, export, repair, and remove
worlds. It can also convert worlds between Java and Bedrock where supported
with Chunker, in app.

Backups can run manually, on a schedule, or before risky changes. MSC 2 does
not call a backup successful until it has finished writing and passed its
verification checks. Restores are handled as carefully as possible too.

### Handles mods, plugins, and modpacks

MSC 2 can find and install server software, mods, plugins, and modpacks from
supported providers such as Modrinth and CurseForge. It checks Minecraft
versions and loaders, resolves dependencies, and keeps client-only mods off
the server when it can identify them.

### Learn how players can connect

Getting players connected usually requires choosing a connection method. MSC
2's built-in handbook explains the common options, what each one does, and what
you need to set up. Some options integrate with MSC 2; others require changes
to your router or a separate third-party service.

- [**LAN**](content/help/handbook/networking-basics.md), when everyone is on
  the same network
- [**Port forwarding**](content/help/handbook/port-forwarding-duckdns.md),
  when you control the router; the handbook includes setup guides for selected
  routers
- [**Playit.gg**](content/help/handbook/playit.md), when you do not want to
  change the router
- [**DuckDNS**](content/help/handbook/port-forwarding-duckdns.md), for using a
  stable hostname
- [**Tailscale**](content/help/handbook/tailscale.md), for privately managing
  the computer that runs MSC 2 from another computer
- [**Xboxbroadcast**](content/help/handbook/xbox-broadcast.md), which helps
  Bedrock players on consoles find a reachable server through their Friends
  tab

MSC 2 does not automatically make every network setup work. It explains the
pieces involved and helps you understand what still needs to be configured.

### Explains problems in plain language

When a server does not start, MSC 2 tries to identify the useful problem
instead of showing you a wall of log output and leaving you there. It can
recognise common issues such as the wrong Java version, a missing dependency,
a port already in use, a full disk, or unsafe memory settings.

The raw log is still available when you want it. It just is not the only answer.

### Shows how the server and computer are doing

MSC 2 watches the numbers that matter to a Minecraft server, including CPU,
memory, disk space, player activity, and tick rate and warns when the computer 
is running out of room for Minecraft.

## How it works

MSC 2 has one part that runs your servers and several ways to control it.

- **The server computer** runs the MSC 2 agent and the Minecraft server. This
  can be your Mac, Windows PC, Linux computer, or an old machine with no
  monitor attached.
- **The desktop app** gives you a graphical interface on the same computer or
  another computer.
- **The desktop browser** lets you manage a screenless server from another
  computer.
- **The CLI** gives you a scriptable terminal interface when you want
  automation or prefer the command line.

The server keeps running when you close the desktop app, close your browser,
or sign out. The app is a control panel; it is not the thing keeping Minecraft
alive.

## A simple way to think about it

If you are new to servers, there are two computers to think about:

1. The **host** is the computer that runs the Minecraft world. It can sit in a
   closet with the lid closed and no monitor attached.
2. The **control device** is the computer where you open the MSC 2 app, browser,
   or CLI to make changes.

They can be the same computer. They can also be different computers. The
players' consoles, phones, and gaming PCs are separate again: they connect to
the Minecraft server but do not need MSC 2 installed.

## Download and install

Choose the installation that matches how you want to use MSC 2. The desktop
app already includes the MSC 2 agent; you do not need to download both.

- Use the **desktop app** when you want a graphical interface on the server
  computer.
- Use the **headless agent** when the server computer has no monitor or desktop
  environment. Manage it from another computer with the desktop app, a desktop
  browser, or the CLI.

### macOS desktop — Intel and Apple Silicon Macs

Download the matching macOS disk image from the [v0.1.4 release](https://github.com/ctemple9/msc2/releases/tag/v0.1.4), open it, and drag MSC 2 into your Applications folder. Intel Macs include the local Bedrock runtime. Apple Silicon Macs include the app and Java runtime support, but no local Bedrock VM; Bedrock servers can still be managed on a compatible remote host.

### Windows desktop — 64-bit Windows

Download the Windows `.msi` installer from the [v0.1.4 release](https://github.com/ctemple9/msc2/releases/tag/v0.1.4) and follow the installation prompts.

### Debian or Ubuntu desktop

Download the Linux `.deb` package from the [v0.1.4 release](https://github.com/ctemple9/msc2/releases/tag/v0.1.4), then install it with:

~~~sh
sudo apt install ./msc2-0.1.4-linux-x86_64.deb
~~~

### Fedora or other RPM-based Linux

Download the Linux `.rpm` package from the [v0.1.4 release](https://github.com/ctemple9/msc2/releases/tag/v0.1.4), then install it with:

~~~sh
sudo dnf install ./msc2-0.1.4-linux-x86_64.rpm
~~~

### Headless agent — macOS, Windows, and Linux

Standalone headless archives are published for all three supported desktop
platforms. They contain the same `msc` agent/CLI binary, with `msc.exe` on
Windows. The archive's `HEADLESS-INSTALL.md` describes the platform-specific
command location, PATH ownership, upgrade/uninstall boundary, and the separate
operating-system service installation.

The current beta Linux archive includes `install.sh` and `uninstall.sh` for
the systemd service. The macOS and Windows archive command installers are the
next Phase 14 packaging step; until they land, those archives can be unpacked
and the binary run directly. The release artifact contract is recorded in
[`headless-installation.md`](docs/msc2/clients/headless-installation.md).

### Updating a headless installation

Signed releases can be checked and staged locally from the agent binary:

~~~sh
msc update check
msc update install --release-id 0.1.4
~~~

The install command asks for a second confirmation. Pass `--yes` for
automation, and add `--json` when a script needs machine-readable output.

## Start your first server

1. Open MSC 2, or connect to the headless agent from another computer.
2. Choose **Add Server**.
3. Choose **Java** or **Bedrock**, then select the Minecraft version and server
   software.
4. Configure the world and connection method.
5. Create and start the server.
6. Complete the EULA and connection setup when MSC 2 asks you to.

For a first Java server, **Paper** is a good default. For a first Bedrock
server, choose **Bedrock Dedicated Server**.

## New here?

You do not need to read a Minecraft server administration manual before using
MSC 2. The application is meant to explain things as you go, and the built-in
handbook covers the parts that tend to confuse people.

- [Your first Java server](content/help/handbook/first-server.md)
- [Your first Bedrock server](content/help/handbook/first-bedrock-server.md)
- [Playing with Bedrock friends](content/help/handbook/plugins-crossplay.md)
- [How Minecraft servers actually connect](content/help/handbook/networking-basics.md)
- [Port forwarding](content/help/handbook/port-forwarding-duckdns.md)
- [What to do if you cannot port forward](content/help/handbook/playit.md)
- [Using Xboxbroadcast](content/help/handbook/xbox-broadcast.md)
- [Worlds and backups](content/help/handbook/worlds-backups.md)

## Local-first

MSC 2 runs on hardware you control. There is no required MSC account, MSC
subscription, telemetry backend, or MSC cloud that your server has to stay
connected to. Your Minecraft server and worlds remain on your hardware.

Optional third-party services can still be used when you choose them, such as
Playit.gg, Tailscale, DuckDNS, Modrinth, or CurseForge.

## The story behind MSC 2

The reason MSC 2 exists—and the story of moving from a rough Python script to
MSC 1 and then to this cross-platform version—is in [MSC 2's origin story](docs/msc2/msc2-origin.md).

## Project documentation

If you are interested in how MSC 2 is built or want to contribute:

| Document | What it is |
|---|---|
| [Vision index](docs/msc2/MSC2-VISION.md) | How the MSC 2 documentation fits together |
| [Product](docs/msc2/msc2-product.md) | The product's goals and user experience |
| [Engineering](docs/msc2/msc2-engineering.md) | Architecture, API contract, and platform support |
| [Decisions](docs/msc2/msc2-decisions.md) | Decisions, reasoning, and rejected alternatives |
| [Port plan](docs/msc2/msc2-port-plan.md) | Implementation phases and exit gates |

## Built on other people's work

MSC 2 uses and builds on projects maintained by other people:

| Project | What MSC uses it for |
|---|---|
| [PaperMC](https://papermc.io) | The Paper server platform |
| [PurpurMC](https://purpurmc.org) | Paper fork with extended configuration |
| [FabricMC](https://fabricmc.net) | Lightweight mod loader for Java servers |
| [NeoForge](https://neoforged.net) | Modern Forge-based mod loader |
| [MinecraftForge](https://minecraftforge.net) | The original Java mod loader |
| [Geyser](https://github.com/GeyserMC/Geyser) | Lets Bedrock players join a Java server |
| [Floodgate](https://github.com/GeyserMC/Floodgate) | Lets Bedrock players join without a Java account |
| [Chunker](https://github.com/HiveGamesOSS/Chunker) | Converts worlds between Java and Bedrock |
| [Xboxbroadcast](https://github.com/MCXboxBroadcast/Broadcaster) | Lets players on Bedrock editions find your server from their Friends tab |
| [Modrinth](https://modrinth.com) | Mod and modpack catalog |
| [CurseForge](https://www.curseforge.com) | Mod and modpack catalog |
| [Playit.gg](https://playit.gg) | Lets people connect without port forwarding |
| [Adoptium Temurin](https://adoptium.net) | The Java runtime that Java servers need |
| [Mojang & Microsoft](https://www.minecraft.net) | Minecraft itself and the Bedrock Dedicated Server |

If I forgot to credit your project, that's on me. Let me know and I'll address that; thank you to everyone who helped maintain the projects above.

## Built with

**Rust** · **Tauri** · **Svelte** · **Swift**

Swift is used for the macOS Bedrock runtime.

## License

TBD
