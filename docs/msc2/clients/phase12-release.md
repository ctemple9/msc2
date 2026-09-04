# MSC 2 beta release artifact contract

**Status:** frozen for the first beta artifact set (P12.59)
**Date:** 2026-09-04
**Authority:** `MSC2-VISION.md`, `msc2-decisions.md`,
`msc2-engineering.md`, and the Phase 12 port-plan entry

This document defines what the first beta release contains and what a person
may safely expect from it. It is a packaging contract, not evidence that the
artifacts have already been built, signed, or accepted on physical hardware.

## 1. Beta boundary

The first beta is the Tauri desktop shell, the shared Svelte frontend, and the
single Rust `msc` binary that provides both the headless agent and the CLI.
The Tauri application embeds the matching agent binary; the standalone
headless archives expose that same binary directly. The agent and its API stay
the source of truth for server management.

The beta artifact set includes:

- Tauri desktop installers for macOS, Windows, and Linux;
- standalone headless agent/CLI archives for macOS, Windows, and Linux; and
- on macOS, the Intel Bedrock sidecar and its appliance resources wherever a
  desktop or headless package includes the macOS agent.

iOS is not a beta release artifact. The existing iOS client remains a
supported client of the API, but it is not built, uploaded, or versioned by
this release set. The full-screen terminal UI (TUI) is also not a beta
artifact; the command-line interface in `msc` is included.

This boundary does not remove any agent route or change the API contract. In
particular, a remote Tauri client may manage Minecraft servers through the
authenticated agent, but it may not install, start, stop, or uninstall the
operating-system service on the host. Service installation and removal are
local installer or local CLI operations, protected by the host operating
system. `server stop` means stopping a Minecraft process; it is not a remote
service-management command.

## 2. Platform and architecture matrix

The first beta publishes x86_64 artifacts only. This is the architecture that
the physical Linux and Windows handoff can verify and the architecture of the
Intel macOS Bedrock appliance. An artifact that is not in this table is not a
beta support claim.

| Surface | Rust target | Canonical beta asset | Notes |
|---|---|---|---|
| macOS desktop | `x86_64-apple-darwin` | `msc2-<release>-macos-x86_64.dmg` | Includes the agent and Intel Bedrock sidecar/resources. |
| macOS headless | `x86_64-apple-darwin` | `msc2-headless-<release>-macos-x86_64.tar.gz` | Includes the agent/CLI and Intel Bedrock sidecar/resources. |
| Windows desktop | `x86_64-pc-windows-msvc` | `msc2-<release>-windows-x86_64.msi` | Includes the agent/CLI; no sidecar. |
| Windows headless | `x86_64-pc-windows-msvc` | `msc2-headless-<release>-windows-x86_64.zip` | Includes the agent/CLI; no sidecar. |
| Linux desktop | `x86_64-unknown-linux-gnu` | `msc2-<release>-linux-x86_64.deb` | Tauri desktop package for supported Debian/Ubuntu systems. |
| Linux headless | `x86_64-unknown-linux-gnu` | `msc2-headless-<release>-linux-x86_64.tar.gz` | Contains the single `msc` binary, installer, uninstaller, and service definitions. |

`<release>` is the release ID without the leading `v`, for example
`0.1.0-beta.1`. Names are stable and case-sensitive; there is no `latest`
asset. Each desktop installer contains the matching agent version, so a
desktop user does not download a second agent to make the shell work.

Apple Silicon macOS (`aarch64-apple-darwin`) is outside this first beta asset
set for Bedrock purposes. D-028 remains in force: there is no arm64 appliance
or Rosetta-for-Linux Bedrock path, and Apple Silicon must be reported as
Bedrock unavailable rather than silently mapped to the Intel sidecar. Linux
and Windows arm64 assets are likewise outside this beta set. This packaging
boundary does not change the broader platform decisions or make an
unsupported architecture appear tested.

## 3. Linux headless baseline and installation shape

The supported Linux headless baseline is:

- Debian 12 (Bookworm), or Ubuntu or another mainstream distribution whose
  `systemd` is version 250 or newer;
- x86_64 userspace with the libraries required by the Rust binary;
- no graphical desktop, X11, Wayland, WebKitGTK, or Tauri installation; and
- a normal local user account that owns the managed server directories.

Debian 11 is below the baseline because its `systemd` is too old for the
credential-helper contract. The Linux package does not require a logged-in
graphical session.

The headless archive is installed by its `install.sh` and removed by its
`uninstall.sh`. The installer may request one elevated installation window;
it must preserve the invoking user's identity instead of turning the service
into a root-owned server manager. The resulting layout is:

| Item | Beta contract |
|---|---|
| Installed binary | `/usr/lib/msc2/msc`, root-owned and not writable by the service user |
| Agent unit | `/etc/systemd/system/com.ctemple.msc2.agent.service` |
| Agent identity | `User=` and `Group=` are the installing user's UID and primary group |
| Agent data | The installing user's `~/.local/share/msc2` (or the explicitly configured `MSC2_DATA_DIR`) |
| Agent logs | The installing user's MSC state/log directory, plus the normal `journalctl` stream for the unit |
| Credential store | `/var/lib/msc2/credentials`, root-owned mode `0700` |
| Helper socket | `/run/msc2/credential-helper.sock`, mode `0600`, owned by the installing user |
| Helper units | `msc2-credential-helper.socket` and `msc2-credential-helper.service` |

The installer creates the user-owned data and log directories before first
start, with the installing user's UID and primary group. It enables the agent
for boot. Routine lifecycle control is through the operating system:

```text
systemctl status com.ctemple.msc2.agent.service
systemctl start com.ctemple.msc2.agent.service
systemctl stop com.ctemple.msc2.agent.service
```

The package does not add a second daemon, desktop dependency, or background
updater. Uninstall stops and disables the units, removes the installed MSC
program and service definitions, and leaves managed server data and explicit
user configuration for the documented recovery decision rather than
silently deleting worlds.

### Linux credential-helper ownership

The agent itself never runs as root. The credential helper is the narrow
exception required for the Linux production secret path: systemd starts
`msc2-credential-helper.service` as root through
`msc2-credential-helper.socket`, while the socket is owned by the installing
user and set to mode `0600`. The helper checks the connecting process's
`SO_PEERCRED` UID against the installing user's UID before serving a request.

The helper stores encrypted credential verifiers under
`/var/lib/msc2/credentials`; it never receives or logs a raw bearer token as
part of release metadata. The agent uses the helper for the existing
`SecretStore` interface. Pairing, server management, and normal agent
operation do not require `sudo` after installation. Changing the helper
units, allowed UID, or root-owned store is an installation/service-management
operation and therefore requires local elevation.

The same installing-user identity rule applies to the other headless
platforms: macOS uses a `launchd` LaunchDaemon with `UserName` set to the
installing user, and Windows uses a Windows Service configured to log on as
that user. The Windows service is not `LocalSystem`; the macOS agent is not a
LaunchAgent that depends on a login session.

## 4. Version and tag rules

One version is used by the Rust workspace, the npm package, the Tauri
configuration, the embedded agent, the headless archives, and the release
metadata. The beta uses SemVer prereleases:

```text
v<MAJOR>.<MINOR>.<PATCH>-beta.<N>
```

For example, tag `v0.1.0-beta.1` produces release ID `0.1.0-beta.1` and the
asset names in the matrix above. A final release uses `v<MAJOR>.<MINOR>.<PATCH>`;
tags are immutable and never moved. A release build fails if the source
version, Tauri version, embedded agent version, tag version, or artifact
version disagree.

Manual workflow dispatch is for candidate artifacts and checks only. It does
not publish a GitHub release and does not create a moving tag. Publication is
allowed only from an exact version tag after every required platform job has
completed successfully. A failed or partial matrix cannot publish a release.

The release ID is not an API-major change. The existing version-skew and
capability rules remain the authority for clients and agents; packaging does
not invent a second compatibility scheme.

## 5. Unsigned beta limitations

The first beta is explicitly unsigned for distribution purposes:

- macOS is not Developer ID signed or notarized. The macOS Bedrock sidecar
  may carry an ad-hoc signature where Virtualization.framework requires its
  entitlement, but that is not publisher identity and does not remove the
  Gatekeeper warning;
- Windows installers and binaries are not Authenticode signed, so SmartScreen
  and the unknown-publisher warning are expected; and
- Linux packages are not distributed through a signed MSC package repository.

The person installing the beta must obtain it from the intended release page,
inspect its checksum, and explicitly approve the operating system's warning.
The beta has no production auto-update path. In particular, an unsigned beta
must not be presented as a trusted coordinated update set, and the private
key for a future signed update manifest must not be invented or committed to
the repository.

These limitations are release evidence to record, not failures to hide. A
successful checksum comparison proves that downloaded bytes match the
published bytes; it does not prove who published them.

## 6. Checksum contract

Every release page publishes one `SHA256SUMS` file alongside the assets. It
contains one line per release asset, using lowercase hexadecimal SHA-256 and
the standard two-space separator:

```text
<64 lowercase hex characters>  <exact asset filename>
```

The manifest is generated in CI from the final bytes after packaging. It
includes every desktop installer and every headless archive. The checksum
file itself is metadata, not a self-hashed release asset. The filename in the
manifest must exactly match the uploaded filename; no wildcard or `latest`
entry is valid.

Installers and handoff notes verify the SHA-256 before opening or installing
an artifact. A missing, duplicate, malformed, or mismatched line fails the
release check. The checksum file is an integrity aid, not a signature, and
does not change the unsigned-beta warning above.

## 7. Pairing and remote access

After a headless install, pairing is created locally on the host as the
installing user:

```text
msc pairing create --client-kind desktop
```

The command displays a short-lived, one-use pairing code once and does not
write it to logs, shell history, ordinary configuration, release metadata,
or an operation result. It must not be run as root. The Tauri backend redeems
the code for one host-scoped bearer credential and stores that credential in
the platform credential store; the Svelte page never receives the raw
credential. Browser pairing uses the existing httpOnly session-cookie flow.

For a host whose management API remains on its default loopback bind, use an
SSH local forward from the client machine:

```text
ssh -N -L 48001:127.0.0.1:48001 <installing-user>@<host>
```

Then add or pair the host through the Tauri client at the forwarded local
address. SSH transports the management connection; it does not grant the
client operating-system service privileges.

An explicitly configured Tailscale management path may instead use the
host's tailnet address or name. Tailscale membership never replaces bearer
authentication or permission checks. The default remains loopback-only;
there is no general-LAN management bind, public management port, or TLS
certificate bypass in this beta contract. Playit and other public tunnels are
for Minecraft player traffic, not the management API.

After a host reset, old credentials are invalid. The operator must run the
same host-local pairing command again and use the new one-use code. A remote
client may control Minecraft through the recovered API connection, but it
still cannot uninstall or stop the host's operating-system service.

## 8. Physical Linux and Windows handoff

CI can prove that an artifact was built. It cannot prove that a clean machine
boots the service, preserves ownership, survives client closure, or accepts
the operating-system's unsigned warning. The beta handoff therefore remains
open until Cameron records evidence from physical x86_64 Linux and Windows
machines.

The Linux handoff records, on a clean Debian 12 or qualifying Ubuntu host:

1. no graphical desktop or WebKit/Tauri dependency is installed;
2. the downloaded headless archive matches `SHA256SUMS`;
3. installation creates the user-owned data/log directories and the
   root-owned, UID-restricted credential-helper units;
4. the agent is enabled and starts at boot under the installing user;
5. pairing is created without `sudo`, then a Tauri client connects over SSH
   forwarding or the explicitly configured Tailscale path;
6. an authenticated client starts and stops a Minecraft server, and closing
   the client leaves the agent and server running; and
7. `systemctl`, `journalctl`, helper-socket permissions, and clean uninstall
   leave an inspectable record.

The Windows handoff records, on a clean physical x86_64 Windows machine:

1. the downloaded installer matches `SHA256SUMS` and the expected
   SmartScreen/unknown-publisher warning is explicitly noted;
2. the Tauri installer launches and embeds the matching agent/CLI;
3. the Windows Service is registered to run as the installing user, not
   `LocalSystem`;
4. the service and Minecraft process remain alive with the desktop closed
   and after signing out and back in; and
5. authenticated remote control changes Minecraft server state without
   gaining any ability to install, stop, or uninstall the Windows Service.

Each evidence record names the OS release, architecture, artifact filename,
SHA-256, installation identity, exact service state, and any expected
unsigned warning. A green CI run or a successful local build is necessary
but is not a substitute for this physical Linux/Windows handoff.

## 9. Linux headless package implementation (P12.61)

The Linux headless archive is assembled by
`tools/release/build-linux-headless.sh`. It builds `msc-agent` with its
optional browser bundle disabled, so the archive contains one native `msc`
binary and no Tauri, WebKitGTK, or other desktop payload. The staged checker
copy is `target/release-headless/linux/msc`; the release archive is named
`msc2-headless-<version>-linux-x86_64.tar.gz`.

The archive contains `install.sh`, `uninstall.sh`, and four systemd input
definitions. The installer renders the installing user's UID, primary group,
and data path into the definitions, installs the root-owned binary at
`/usr/lib/msc2/msc`, creates the user-owned data, `logs`, and `servers`
directories, and creates the root-owned credential store at
`/var/lib/msc2/credentials`. It installs and enables
`com.ctemple.msc2.agent.service` and `msc2-credential-helper.socket`; the
helper service is socket-activated and runs the existing Rust
`systemd-creds` backend as root, while `SO_PEERCRED` limits requests to the
installing user's UID. The agent itself always runs as that user.

The installer starts the helper socket and agent after enabling them for boot.
It never creates a pairing code, writes a bearer token into a unit or ordinary
configuration, or invokes pairing through `sudo`. The printed follow-up is the
host-local command the operator runs as the installing user:

```text
msc pairing create --client-kind desktop
```

Uninstall stops and disables only these MSC units, removes the installed
binary, unit definitions, and runtime-directory rule, and retains managed
server data, logs, configuration, and credential blobs for an explicit later
cleanup decision.
