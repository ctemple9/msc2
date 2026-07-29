# MSC 2

Self-hosting a Minecraft server should feel like using an application, not maintaining a collection of terminal commands, Java arguments, config files, and networking workarounds.

MSC 2 is a **background service** that runs and manages Minecraft servers, plus the interfaces you use to control it — a desktop app, a web page, an iPhone app, and a command line. All four talk to the same service, so anything you can do in one, you can do from the others.

It runs on **macOS, Windows, and Linux**, with or without a graphical desktop. A closed laptop with no monitor is a first-class deployment, not a workaround.

> **Status: pre-alpha.** Nothing is built yet. The architecture and requirements are settled and documented; the code starts now.

---

## What it does

- Creates and runs Java servers — Vanilla, Paper, Purpur, Fabric, NeoForge, Forge — and Bedrock servers
- Installs modpacks from Modrinth and CurseForge, resolving dependencies and pruning client-only mods
- Manages worlds as named slots you can swap, duplicate, export, and repair
- Backs up on a schedule, verifies the archive, and restores safely
- Explains failed startups in plain language, with one-tap repairs
- Handles player connections: LAN, port forwarding, Playit.gg tunnels, Bedrock cross-play, Xbox friend discovery
- Monitors host and server health, and says what the numbers mean

## Design commitments

- **Local-first.** No account, no telemetry, no cloud backend. Your worlds stay on hardware you own.
- **Headless everywhere.** The graphical app is optional on every platform.
- **One engine, many interfaces.** No client reimplements server logic, so no client can fall behind by design.
- **Data safety over convenience.** Nothing is called a backup until it has completed and verified.

## Documentation

| Document | For |
|---|---|
| [Vision index](docs/msc2/MSC2-VISION.md) | **Start here** — how the document set fits together |
| [Product](docs/msc2/msc2-product.md) | What MSC 2 is, in plain language |
| [Engineering](docs/msc2/msc2-engineering.md) | Architecture, API contract, platform support |
| [Decisions](docs/msc2/msc2-decisions.md) | Every decision, with reasoning and rejected alternatives |
| [Port plan](docs/msc2/msc2-port-plan.md) | Phases and their exit gates |
| [Rolling plan](docs/msc2/rolling-plan.md) | Where the build currently is |

## Relationship to MSC 1

[MSC 1](https://github.com/) is a mature macOS app — around 97,000 lines of Swift — that does most of this today, on one platform, inside a single GUI application.

MSC 2 is a **separate project.** It does not modify MSC 1, read its configuration, or share state with it. Servers move across by explicit import.

MSC 1 remains the **executable specification**: its behavior is the reference every ported domain is verified against. Two independent audits agreed at file level on 88.6% of its 246 source files, and identified roughly a third of the codebase as engine logic worth carrying forward.

## Built with

Rust · Tauri · Svelte · Swift (iOS client and the macOS Bedrock runtime)

## License

TBD
