# MSC 2 headless installation contract

**Status:** P14.8 contract; implementation is split between P14.9 (Linux) and
P14.10 (macOS and Windows)

This document defines the command-install shape for the standalone MSC 2
headless artifacts. It is deliberately separate from the operating-system
service contract: putting `msc` on a user's PATH must not be confused with
registering, starting, stopping, or removing the management service.

The supported headless control surface is the scriptable CLI in the same
binary as the agent. The command is `msc` on macOS and Linux and `msc.exe` on
Windows. The management service listens on `127.0.0.1:48001` by default.

## Artifact set

The release workflow publishes these standalone archives:

| Host | Rust target | Archive |
|---|---|---|
| macOS Intel | `x86_64-apple-darwin` | `msc2-headless-<release>-macos-x86_64.tar.gz` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `msc2-headless-<release>-macos-aarch64.tar.gz` |
| Windows 64-bit | `x86_64-pc-windows-msvc` | `msc2-headless-<release>-windows-x86_64.zip` |
| Linux 64-bit | `x86_64-unknown-linux-gnu` | `msc2-headless-<release>-linux-x86_64.tar.gz` |

Every archive contains the matching headless binary and this document under
the name `HEADLESS-INSTALL.md`. The macOS Intel archive also contains the
Bedrock sidecar and its appliance resources. The Apple Silicon archive has no
Bedrock sidecar; that is a platform capability boundary, not an installation
failure.

The Linux archive additionally contains `install.sh`, `uninstall.sh`, and the
systemd input definitions used by its service installer. macOS and Windows
archive installers are the P14.10 implementation work; until that work lands,
their archives can still be unpacked and the binary can be run directly.

## Command-install shapes

The standalone archive installers use these stable locations:

| Host | Installed executable | PATH entry owned by MSC | Upgrade behavior |
|---|---|---|---|
| Linux archive | `/usr/lib/msc2/msc` | `/usr/local/bin/msc` symlink | Replace the MSC-owned target while preserving the symlink. |
| macOS archive | `/usr/local/lib/msc2/<architecture>/<version>/msc` | `/usr/local/bin/msc` symlink | Install the new version beside the old one, then move the MSC-owned symlink. |
| Windows archive | `%LOCALAPPDATA%\\MSC2\\bin\\msc.exe` | `%LOCALAPPDATA%\\MSC2\\bin` in the installing user's PATH | Replace only the MSC-owned executable in the owned directory. |

The Unix locations are intentionally conventional command locations. The
installer may request local administrator approval to write them, but the
agent and managed Minecraft servers keep their documented non-root service
identity. On Windows, adding the directory to the installing user's PATH is
separate from installing a Windows Service and does not require machine-wide
PATH changes unless the operator explicitly chooses that scope.

An installer must establish ownership before changing an existing command
target:

- a Unix `msc` path may be replaced only when it is an MSC-owned symlink whose
  target is inside the corresponding MSC installation root;
- a Windows PATH entry may be added or removed only for the exact MSC-owned
  directory; and
- an existing non-MSC file, symlink, directory, or PATH entry is reported as a
  conflict and is never overwritten or deleted automatically.

Upgrades are idempotent. Running the same installer twice leaves one command
entry and one active target, not duplicate PATH entries or versioned links.
Uninstall removes the MSC-owned command link or executable and its PATH entry
only while ownership still matches the recorded MSC installation. It never
removes `/usr/local/bin`, a user's unrelated PATH entries, or managed server
data, logs, configuration, worlds, backups, or credentials.

## Linux package-manager boundary

The `.deb` and `.rpm` artifacts in the current beta are desktop packages, not
standalone headless packages. They remain owned by `apt`/`dpkg` or `dnf`/`rpm`;
the archive `install.sh` and `uninstall.sh` must not be used for them.

If a distribution-managed headless package is published later, its package
manager owns `/usr/bin/msc` directly. That shape has no archive symlink and no
archive ownership marker: upgrades and removal go through the distribution
package manager. The standalone archive shape above remains available for
systems that do not use a supported package manager.

## Service boundary

Command installation does not install or control the operating-system
service. Service registration is a local, elevated operation owned by the
platform installer:

| Host | Service manager | Management endpoint |
|---|---|---|
| macOS | `launchd` LaunchDaemon | `127.0.0.1:48001` |
| Windows | Windows Service | `127.0.0.1:48001` |
| Linux | `systemd` | `127.0.0.1:48001` |

The service may invoke the installed binary, but a PATH change must not be
required for the service to start. Closing a client does not stop the service;
remote API clients cannot install, start, stop, replace, or uninstall another
host's operating-system service. `msc server stop` remains a Minecraft
process operation, not service management.

## Noninteractive output and shell refresh

Headless installers must work without a graphical prompt. They report the
result on standard output and use a nonzero exit status for a failed install.
When the command location is already on PATH, they report that no shell
refresh is needed. When a user or process must start a new shell, they report
the exact path entry and say so plainly, for example:

```text
MSC 2 installed.
The command is available as msc in new shells.
Refresh PATH or open a new shell before using it from this shell.
```

The installer must not claim that a command is available in the current
process when that process inherited an older PATH. A noninteractive mode may
use machine-readable output, but it must retain the same state distinction:
installed, PATH refresh required, or failed. It must never print pairing codes,
bearer credentials, private keys, or other secrets as installation metadata.

For service installation, the final message separately identifies the service
manager, service name, endpoint, and whether the service is enabled and
running. A successful PATH install is not reported as a successful service
install.

## Repository references

The artifact and checksum contract is `docs/msc2/clients/phase12-release.md`.
The Linux service implementation is `packaging/linux/install.sh`; the Phase 14
source and acceptance map is
`docs/msc2/capabilities/phase14-operational-refinements.md`.
