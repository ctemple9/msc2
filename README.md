# MSC 2
Built by ctemple9

> **MSC 2 is currently an unsigned prerelease.**
>
> The current release supports x86_64/Intel computers:
>
> - Intel macOS
> - 64-bit Windows
> - 64-bit Linux
>
> Apple Silicon macOS and ARM Linux/Windows are not part of this prerelease.
> The release is unsigned, so macOS, Windows, or Linux may show a security
> warning the first time you open or install it.
>
> [Download MSC 2 v0.1.1](https://github.com/ctemple9/msc2/releases/tag/v0.1.1)

## Download and install

Choose the installation that matches how you want to use MSC 2.

- Use the **desktop app** if you want to manage the server from the same computer with a graphical interface.
- Use the **headless agent** if the server computer has no monitor or desktop environment. You can manage it from another computer, phone, or browser.

The desktop app already includes the MSC 2 agent. You do not need to download both.

### macOS desktop — Intel Macs

Download the disk image:

```sh
curl -fL -o msc2-0.1.1-macos-x86_64.dmg \
  https://github.com/ctemple9/msc2/releases/download/v0.1.1/msc2-0.1.1-macos-x86_64.dmg
```

Then open the downloaded `.dmg` file and drag MSC 2 into your Applications folder.

```sh
open msc2-0.1.1-macos-x86_64.dmg
```

### Windows desktop — 64-bit Windows

Download the installer:

```powershell
curl.exe -fL -o msc2-0.1.1-windows-x86_64.msi `
  https://github.com/ctemple9/msc2/releases/download/v0.1.1/msc2-0.1.1-windows-x86_64.msi
```

Open the `.msi` file and follow the installation prompts.

### Debian or Ubuntu desktop

Download the `.deb` package:

```sh
curl -fL -o msc2-0.1.1-linux-x86_64.deb \
  https://github.com/ctemple9/msc2/releases/download/v0.1.1/msc2-0.1.1-linux-x86_64.deb
```

Install it with:

```sh
sudo apt install ./msc2-0.1.1-linux-x86_64.deb
```

### Fedora or other RPM-based Linux

Download the `.rpm` package:

```sh
curl -fL -o msc2-0.1.1-linux-x86_64.rpm \
  https://github.com/ctemple9/msc2/releases/download/v0.1.1/msc2-0.1.1-linux-x86_64.rpm
```

Install it with:

```sh
sudo dnf install ./msc2-0.1.1-linux-x86_64.rpm
```

### Linux headless agent

Use this on a Linux computer that will host the Minecraft servers without a graphical desktop.

The current Linux headless package is intended for Debian 12, Ubuntu, and other mainstream distributions using systemd 250 or newer.

Download the archive and checksum file:

```sh
curl -fLO \
  https://github.com/ctemple9/msc2/releases/download/v0.1.1/msc2-headless-0.1.1-linux-x86_64.tar.gz

curl -fLO \
  https://github.com/ctemple9/msc2/releases/download/v0.1.1/SHA256SUMS
```

Verify the download:

```sh
sha256sum --ignore-missing -c SHA256SUMS
```

Install MSC 2:

```sh
mkdir msc2-headless
tar -xzf msc2-headless-0.1.1-linux-x86_64.tar.gz -C msc2-headless
cd msc2-headless
./install.sh
```

Run `install.sh` as your normal user. It requests administrator permission when needed and installs the agent as your user instead of running your Minecraft servers as root.

After installation, the agent starts automatically and is configured to start again after reboot.

Check its status with:

```sh
systemctl status com.ctemple.msc2.agent.service --no-pager -l
```

## Start your first server

After installing MSC 2:

1. Open the desktop app, or connect to the headless agent from another device.
2. Choose **Add Server**.
3. Select Java or Bedrock.
4. Choose the Minecraft version and server software.
5. Configure the world and network connection.
6. Create the server.
7. Start the server.
8. Complete the EULA and connection setup when MSC 2 asks you to.

For a first Java server, **Paper** is a good default. For a first Bedrock server, choose **Bedrock Dedicated Server**.

More detailed guides:

- [Your first Java server](content/help/handbook/first-server.md)
- [Your first Bedrock server](content/help/handbook/first-bedrock-server.md)
- [How Minecraft servers connect](content/help/handbook/networking-basics.md)
- [Port forwarding](content/help/handbook/port-forwarding-duckdns.md)
- [Using Playit.gg](content/help/handbook/playit.md)

I wanted running a Minecraft server to feel like running an app.

You shouldn't need to know a bunch of terminal commands, Java arguments, config files, networking stuff, and whatever else just because you want to make a world your friends can join.

That's what **MSC 2** is for.

At the center of MSC 2 is a background service that actually runs and manages your Minecraft servers. Then you can control that service however you want: from the desktop app, a web browser, an iPhone, or the command line.

They're all controlling the same thing. The desktop app doesn't have its own version of the server logic, and neither does the web app or CLI. If you start a server from your phone, the desktop app sees it. If you change something from the CLI, the web app sees it.

MSC 2 is being built to run on **macOS, Windows, and Linux**, and it does not need a graphical interface to work.

So if you have an old laptop sitting closed in a closet with no monitor attached to it, that's a completely normal way to run MSC 2. You can install the engine there and control it from another computer or your phone.

## What it does

Some of these capabilities are already working in the engine. Others are part of the product MSC 2 is being built toward.

- **Runs Java and Bedrock servers.** Java (Vanilla, Paper, Purpur, Fabric, NeoForge, Forge) and Bedrock (Bedrock Dedicated Server) are managed for you instead of making you deal directly with jars and terminal commands.

- **Handles Java/Bedrock cross-play.** MSC 2 sets up Geyser and Floodgate and handles the version matching so Java and Bedrock players can join the same server.

- **Installs mods and plugins.** You can install packs, mods, and plugins from Modrinth (through the MSC2!) and CurseForge . MSC 2 resolves the dependencies and filters out client-only mods that don't belong on the server.

- **Manages worlds.** Worlds are treated as actual things in the app instead of just folders you hopefully remember not to mess up. You can swap them, duplicate them, export them, and repair them.

- **Makes and verifies backups.** Backups can run on a schedule, and MSC 2 checks the finished archive before calling it a successful backup. Restores are handled safely too.

- **Gets other people connected.** That can mean LAN if everybody is in the same house, normal port forwarding if you control the router, Playit.gg if you don't want to touch the router at all, or Xboxbroadcast for players on Bedrock editions.

- **Explains why something broke.** If a server doesn't start, the goal isn't to dump a wall of Java output on you and wish you luck. MSC 2 tries to figure out what actually happened, explain it normally, and give you the fix when it can.

- **Watches the server and the computer running it.** RAM usage, CPU usage, TPS, player activity, and the other numbers are useful, but only if you know what they mean. MSC 2 tries to tell you that too.

- And a bunch of other things you'd probably expect when it comes to hosting server.

## What makes it different

There are a few things MSC does that I think are especially useful.

### Bedrock servers on Mac

There is no official Bedrock Dedicated Server for macOS. The official server is available for Windows and Linux.

MSC gets around that by running the Linux Bedrock server inside a small Linux virtual machine on the Mac. You don't need to set up the VM or really even know it's there. As far as MSC is concerned, you press Start and you have a Bedrock server.

The VM is built into MSC, so you do not need Docker to run a Bedrock server on macOS. You don't have to install Docker, configure a container, or set up the virtual machine yourself.

Apple Silicon support for macOS Bedrock is deferred for now. I don't have an M-series Mac to test that path, so the current Bedrock VM works on Intel Macs.

### Java and Bedrock world conversion

MSC can convert worlds between Java and Bedrock using Chunker.

So if you have a world from one edition and want to move it to the other, you don't need to go figure out the conversion process separately.

### Xboxbroadcast

Getting people on consoles onto a self-hosted server is weirdly annoying.

Xboxbroadcast works for players on all Bedrock editions, including Xbox, PlayStation 5, Nintendo Switch, and mobile. It does not replace the network connection: you still need port forwarding or Playit.gg so the server can be reached. What Xboxbroadcast does is let those players find the server through their Friends tab instead of making them enter an IP address and port — something consoles make unnecessarily difficult because they don't normally allow custom servers.

### The server doesn't depend on the app being open

That separation matters in practice: closing the desktop app doesn't stop the server. You can manage it from another computer, your phone, or the CLI.

The rest — modpacks, backups, crash explanations, world management, networking — isn't me claiming MSC invented some new idea.

The point is mostly that all of it is in one place, so you don't have to go assemble the whole setup yourself.

## New here?

You do **not** need to read a Minecraft server administration manual before using MSC 2.

The application walks you through things as you get to them, and the built-in handbook explains the parts that tend to confuse people.

But if you're the kind of person who would rather understand what is happening first, start here:

- [Your first Java server](content/help/handbook/first-server.md)
- [Your first Bedrock server](content/help/handbook/first-bedrock-server.md)
- [Playing with Bedrock friends](content/help/handbook/plugins-crossplay.md)
- [How Minecraft servers actually connect](content/help/handbook/networking-basics.md)
- [Port forwarding](content/help/handbook/port-forwarding-duckdns.md)
- [What to do if you can't port forward](content/help/handbook/playit.md)
- [Using Xboxbroadcast](content/help/handbook/xbox-broadcast.md)
- [Worlds and backups](content/help/handbook/worlds-backups.md)

## A few things I don't want MSC 2 to become

### Your server shouldn't depend on my server

MSC 2 is **local-first**.

There is no required MSC account, no telemetry backend, and no MSC cloud that your server has to stay connected to.

Your Minecraft server and your worlds live on hardware you control.

If the MSC project disappeared tomorrow, the computer running your server should not suddenly become useless.

### The GUI should always be optional

MSC 2 is designed to run headless on every platform.

The graphical apps are interfaces for the engine, not the engine itself.

If you want to install MSC 2 on a Linux box with no desktop environment and control the entire thing over the CLI or from another device, that is a normal supported setup.

### There should only be one server engine

The desktop app, web app, iPhone app, and CLI do not each contain their own slightly different implementation of Minecraft server management.

They all talk to the same MSC 2 service.

That matters because otherwise one client eventually gets a feature another client doesn't have, behavior starts drifting between platforms, and now there are four versions of the same bug.

I don't want that.

### A backup should actually be a backup

MSC 2 favors data safety over pretending something succeeded.

If a backup hasn't finished writing and passed verification, MSC 2 does not call it a successful backup.

The same idea applies anywhere your world data is involved.

Convenience is nice. Not losing somebody's Minecraft world is more important.

## Why I built this

I really just wanted to play Minecraft with my people.

I had switched from Bedrock on Console to Java on my MacBook, but most of my people were still playing Bedrock. So if I wanted everybody in the same world, I needed cross-play.

That meant setting up a Paper (which is a type of Minecraft server for Java) server with Geyser and Floodgate (which are plugins for Paper that allow for Java <-> Bedrock crossplay).

At the time, I had basically never used a terminal before.

So now, just to play Minecraft, I had to figure out Java, server jars, plugins, config files, ports, and how all of these pieces were supposed to fit together. And even after I got it working, I still had to keep it working. Paper could update before Geyser supported the new version, so something that worked yesterday would suddenly stop, and I'd be back in the terminal replacing files and restarting the server to find out whether I'd fixed it.

It was just a lot of stuff to deal with when all I was trying to do was play Minecraft with my friends.

So I built something to handle it for me. First a rough script built in python, then a real Mac app called **Minecraft Server Controller (MSC)** that I taught myself Swift to write.

MSC 1 worked, and I used it for a long time. But two limitations eventually became hard to ignore.

First, it was a Mac app. If you didn't have a Mac, you couldn't use it.

Second, the app *was* the server. The graphical program and the thing actually running Minecraft were one and the same, so your server only ran while the app was open, on the machine it was open on. The spare MacBook I used as a server only had 8 GB of RAM, and I didn't want to burn a chunk of that running macOS and a full GUI on a computer that mostly sat in a corner with the lid shut.

I wanted to run the server on whatever made sense — Linux, Windows, some cheap little box with no monitor at all — and control it from somewhere else.

MSC 1 fundamentally wasn't built that way.

That's where **MSC 2** came from.

MSC 2 separates those two things: the engine runs in the background, while the desktop app, web app, phone, and CLI are just ways to control it.

And somewhere along the way, this stopped being a nicer way to launch Paper.

Cross-play led to Geyser and Floodgate. Playing with people outside my house led to port forwarding and Playit.gg. Playing with people on consoles led to Xboxbroadcast. Then came backups, world management, modpacks, world conversion, server health, crash handling, and all the other little problems you run into when you host Minecraft yourself.

So now the goal is to fold that whole pile of problems into one application.

The people I imagine using this are pretty different.

Maybe you know Linux, you're perfectly comfortable in a shell, and you just want a good headless Minecraft server manager. That's fine.

But maybe you just want to play with your niece, your siblings, or your friends. One person is on a computer. Somebody else has an Xbox. You've never opened a terminal and you have no desire to learn what a 'xyz' is tonight.

There's a funny bit of irony here. I built MSC because I was trying to get away from the terminal. Now MSC 2 is being built to run headless and be controlled through one lol.

MSC 2 should work for you too.

## Why I'm building this

There are already other ways to host Minecraft servers, and some of them are really good. I'm not pretending I invented Minecraft server management.

MSC originally started partly because I wanted to learn how to build something like this. But then I kept using it, kept running into new problems, and kept adding ways to solve them.

## MSC 1

[MSC 1](https://github.com/ctemple9/minecraft-server-controller) is still its own project.

It's a mature macOS app with around **97,000 lines of Swift**, and it already does most of the things that eventually led to MSC 2. The problem is that it was built around one Mac application, with the server management logic living inside the GUI.

MSC 2 is built differently, so I'm not slowly converting MSC 1 into MSC 2.

They don't share configuration or live state, and MSC 2 doesn't reach into an MSC 1 installation and start changing things. Moving a server between them is something you explicitly choose to do through an import.

But MSC 1 is still really important to MSC 2 because it's the reference for how a lot of this stuff is supposed to work.

Instead of rewriting a feature from memory and hoping I got all of the behavior right, I can compare it against the application I've already been using.

Two independent audits of MSC 1 agreed at the file level on **88.6% of its 246 source files** and identified roughly a third of the codebase as engine logic worth carrying forward.

So MSC 2 isn't really me throwing MSC 1 away and starting from zero.

It's me taking what worked, separating it from the parts of the architecture that eventually got in the way, and rebuilding it around what I actually want MSC to be.

→ [MSC 1 on GitHub](https://github.com/ctemple9/minecraft-server-controller)

## Built on other people's work

A lot of MSC 2 only exists because other people already did the hard work of building the tools underneath it.

MSC isn't trying to replace those projects. A lot of the time, it's taking something that already works, figuring out how it fits into the rest of the server, and putting an interface around it so you don't have to set up every piece separately.

A lot of these features literally would not exist without the people maintaining these projects:

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
| [Xboxbroadcast](https://github.com/MCXboxBroadcast/Broadcaster) | Lets players on all Bedrock editions find your server from their Friends tab |
| [Modrinth](https://modrinth.com) | Mod and modpack catalog |
| [CurseForge](https://www.curseforge.com) | Mod and modpack catalog |
| [Playit.gg](https://playit.gg) | Lets people connect without port forwarding |
| [Adoptium Temurin](https://adoptium.net) | The Java runtime that Java servers need |
| [Mojang & Microsoft](https://www.minecraft.net) | Minecraft itself, and the Bedrock Dedicated Server |

If MSC is using your work and I missed you here, that's on me. Open an issue and I'll add it.

## How it's built

If you're interested in how MSC 2 actually works or want to contribute, I've documented the architecture in way more detail than would make sense to put in this README.

| Document | What it is |
|---|---|
| [Vision index](docs/msc2/MSC2-VISION.md) | **Start here.** How the rest of the MSC 2 documentation fits together. |
| [Product](docs/msc2/msc2-product.md) | What MSC 2 is supposed to be from the user's perspective. |
| [Engineering](docs/msc2/msc2-engineering.md) | The architecture, API contract, and platform support. |
| [Decisions](docs/msc2/msc2-decisions.md) | Decisions I've made, why I made them, and the alternatives I rejected. |
| [Port plan](docs/msc2/msc2-port-plan.md) | The implementation phases and what has to be true before each one is done. |
| [Rolling plan](docs/msc2/rolling-plan.md) | What I'm actually working on right now. |

## Built with

**Rust** · **Tauri** · **Svelte** · **Swift**

Swift is used for the iOS client and the macOS Bedrock runtime.

## License

TBD
