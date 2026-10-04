# MSC 2 headless installation contract

**Status:** Current headless installation contract. Browser management is not
supported. Published v0.1.16 desktop-agent bytes predate D-038 and may contain
historical browser assets; v0.1.17 was tagged but never published. v0.1.18 is
the first published browser-free prerelease. The Phase 16 exact-artifact
acceptance record (P16.29) remains open pending Cameron's physical results.

This document defines the command-install shape for standalone headless and
desktop packages. Service registration and command registration have separate
ownership rules.

The supported control surfaces are the Tauri desktop app and the scriptable
CLI in the same binary as the agent. The command is `msc` on macOS and Linux
and `msc.exe` on Windows. A headless host is managed from its CLI or remotely
from a Tauri desktop app. CLI commands run on the host itself, either in a
local terminal or an SSH login shell as the account that installed the agent;
the CLI does not choose another host. The management service listens on
`127.0.0.1:48001` by default.

No installation type is promised as a browser client. The agent provides the
authenticated API used by supported desktop and CLI clients. No graphical
desktop is required to run the headless agent.

## Release support

The current release contract covers macOS Intel (`x86_64`) and Apple Silicon (`aarch64`) desktop and
headless artifacts, plus x86_64 Windows and Linux desktop/headless artifacts.
Linux desktop artifacts are `.deb` and `.rpm`; no Linux arm64 artifact is
published. The Linux minimum is Debian 12 (Bookworm), or another distribution
with `systemd` 250 or newer. The release contract does not state minimum OS
versions for macOS or Windows.

The published installers and archives are unsigned by their platform publishers: no macOS
Developer ID signature or notarization, no Windows Authenticode signature,
and no signed Linux MSC package repository. The signed update manifest is not
a publisher signature for these packages. Verify the checksum listed in
`SHA256SUMS`; checksums confirm byte identity, not publisher identity.

The numerical agent/client compatibility window has not been decided. Agents
report their minimum supported client version and refuse clients below that
floor; keep the client and agent current. Security support is limited to the
latest stable release under `SECURITY.md`; there
is not yet a stable release eligible for that policy. Phase 16 exact-artifact
acceptance remains open as P16.29.

## Artifact set

The release workflow publishes these standalone archives:

| Host | Rust target | Archive |
|---|---|---|
| macOS Intel | `x86_64-apple-darwin` | `msc2-headless-<release>-macos-x86_64.tar.gz` |
| macOS Apple Silicon | `aarch64-apple-darwin` | `msc2-headless-<release>-macos-aarch64.tar.gz` |
| Windows 64-bit | `x86_64-pc-windows-msvc` | `msc2-headless-<release>-windows-x86_64.zip` |
| Linux 64-bit | `x86_64-unknown-linux-gnu` | `msc2-headless-<release>-linux-x86_64.tar.gz` |

Every archive contains the matching headless binary and this document under
the name `HEADLESS-INSTALL.md`. The macOS and Windows archives also contain
`MSC2-VERSION` plus their platform installer and uninstaller. The macOS Intel
archive also contains the Bedrock sidecar and its appliance resources. The
Apple Silicon archive has no
Bedrock sidecar; that is a platform capability boundary, not an installation failure.
All desktop and headless Java map packages also include the pinned Vantage
0.15.1 renderer binary and `VANTAGE-LICENSE.txt` beside the MSC agent. The
renderer does not include Minecraft assets. The first Java map launch downloads
and verifies the official Minecraft client assets for the saved world's version
(or the configured Minecraft version when the save has no version metadata).
MSC extracts only block assets and game data into
`<MSC2_DATA_DIR>/map-dependencies/java-assets/<version>` and reuses that cache
on later launches. No Minecraft launcher, player login, or player join is
required on the server host. First use requires internet access from that host.

Headless updates replace both terrain helpers alongside the agent and preserve
their previous copies for rollback. If an older update left Vantage missing,
the map downloads the checksum-pinned renderer into MSC's own dependency cache
without elevation. An explicit `MSC2_VANTAGE_BIN` override remains authoritative;
MSC reports a missing override instead of replacing it. Renderer startup
diagnostics go to the agent logs, with the private renderer token redacted.

The Linux archive additionally contains `install.sh`, `uninstall.sh`, and the
systemd input definitions used by its service installer. macOS archives
contain `install.sh` and `uninstall.sh`; Windows archives contain
`install.ps1` and `uninstall.ps1`. The installers register and start the
local agent service, and enable it for boot.

## Command-install shapes

The installers use these stable locations:

| Host | Installed executable | PATH entry owned by MSC | Upgrade behavior |
|---|---|---|---|
| Linux archive | `/usr/lib/msc2/msc` and `/usr/lib/msc2/vantage` | `/usr/local/bin/msc` symlink | Replace the MSC-owned targets while preserving the symlink. |
| macOS archive | `/usr/local/lib/msc2/<architecture>/<version>/msc` and sibling `vantage` | `/usr/local/bin/msc` symlink | Install the new version beside the old one, then move the MSC-owned symlink. |
| Windows archive | `%ProgramFiles%\\MSC2\\bin\\msc.exe` and sibling `vantage.exe` | `%ProgramFiles%\\MSC2\\bin` in the machine PATH | Replace only the MSC-owned executables in the owned directory. |
| Linux desktop `.deb`/`.rpm` | Package resource under `/usr/lib` | Package-owned `/usr/local/bin/msc` symlink | Package scripts update and remove only a link targeting this desktop package. |
| macOS desktop DMG | Agent staged when the desktop installs its local service | `/usr/local/bin/msc` symlink installed with the local service | Service repair updates the link; service removal removes only its matching link. |
| Windows desktop MSI | `agent\\msc.exe` inside the MSI installation | MSI-owned machine PATH entry for the agent directory | MSI removes its PATH entry on uninstall. |

The Unix locations are intentionally conventional command locations. The
installer may request local administrator approval to write them, but the
agent and managed Minecraft servers keep their documented non-root service
identity. On Windows, the headless installer owns a machine PATH entry and
registers the service under the chosen regular account.

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

The `.deb` and `.rpm` artifacts are desktop packages, not standalone headless
packages. They remain owned by `apt`/`dpkg` or `dnf`/`rpm`; the archive
`install.sh` and `uninstall.sh` must not be used for them. Their package
scripts reject another command at `/usr/local/bin/msc`, and remove the symlink
only when the package target is gone. A DMG has no install hook: on macOS the
desktop registers its command when the owner explicitly installs the local
agent service. The MSI owns its machine PATH entry and removes it with the MSI.

If a distribution-managed headless package is published later, its package
manager owns `/usr/bin/msc` directly. That shape has no archive symlink and no
archive ownership marker: upgrades and removal go through the distribution
package manager. The standalone archive shape above remains available for
systems that do not use a supported package manager.

## macOS archive installation

From the unpacked archive, run:

```sh
./install.sh
```

The installer asks for administrator approval for `/usr/local` and the
LaunchDaemon, stores the binary under the host architecture's version
directory, creates the MSC-owned `/usr/local/bin/msc` symlink, and starts the
agent with boot startup enabled. Intel archives include the Bedrock sidecar
beside the binary; Apple Silicon archives intentionally do not. To remove
the command, its service, and owned version directories for that architecture,
run `./uninstall.sh` from an archive or retained copy. Managed server data is
retained. A conflicting command or service from another installation is
never overwritten.

## Windows archive installation

From an elevated PowerShell window, run:

```powershell
.\install.ps1
```

The default machine installation uses `%ProgramFiles%\MSC2\bin` and the
machine PATH. The installer prompts for the regular Windows account that
will own the service and server files; its password is passed to the Service
Control Manager during registration and cleared from the installer process.
Remove the installation with `.\uninstall.ps1` from the archive. The scripts
refuse an unmarked existing `msc.exe` and remove only their own service,
executable, and PATH entry. Managed server data remains in `%ProgramData%\MSC2`.

PowerShell and Command Prompt sessions inherit PATH when they start. After an
install or removal, open a new shell before using Get-Command msc,
where.exe msc, or msc.exe from that shell. PATH installation is separate
from Windows Service operation; the service does not depend on PATH.

## Service boundary

Command and service registration are owned by the platform installer:

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

Headless installers report the result on standard output and use a nonzero
exit status for a failed install. Windows prompts for a service account or
accepts a `PSCredential` supplied by automation; macOS and Linux request
administrator authorization through the terminal.
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

## Updating from a headless Linux terminal

For a standalone archive installation, check and stage a signed release, then
install the exact release after reviewing its notes:

```sh
msc update check
msc update install --release-id <release-id>
```

When the installed binary is protected, MSC registers a temporary,
unprivileged PolicyKit text agent for the running CLI process before requesting
local administrator authorization. This supports an interactive SSH terminal
and keeps authentication attached to the process that requested the update.
Run the command from a session with a controlling terminal; without one, MSC
reports the authorization setup failure and leaves the staged release
untouched. Distribution-managed `.deb` and `.rpm` installations remain owned
by their package manager and use package-manager guidance instead.

The final installer message identifies the command path and agent boot state.
`msc status agent` reports installed, running, and boot-enabled state.
`msc stop agent` lasts until an explicit start or the next boot. Use
`msc disable agent` to change future boot startup, and `msc enable agent` to
restore it.

Use the same CLI account locally or through SSH. The agent checks the
operating-system identity and issues a short-lived credential for that
invocation, so there is no token to export or pairing step for the CLI:

```sh
msc status agent
msc status --json
msc server list
ssh msc-linux msc status
```

The SSH command runs on `msc-linux`. To manage a different host from the
current computer, use the Tauri desktop app's separate remote-host pairing.
CLI options for a remote host, API URL, port, or bearer token are not part of
the local CLI contract.

## Repository references

The artifact and checksum contract is `docs/msc2/clients/phase12-release.md`.
The Linux service implementation is `packaging/linux/install.sh`; the
cross-platform source and acceptance map is
`docs/msc2/capabilities/phase14-operational-refinements.md`.
