# Physical Linux beta handoff

**Machine:** ______________________________  
**Date (UTC):** ____________________________  
**OS release:** ____________________________  
**Architecture:** __________________________  
**Installing user:** ________________________  
**Release tag:** ___________________________  
**Headless asset:** ________________________  
**Headless asset SHA-256:** ________________________________________________

Run this on a clean physical x86_64 Ubuntu Server installation or another
qualifying Linux partition from the release contract. Do not run it inside a
desktop environment or a virtual machine and call that the Linux gate.

## Artifact and host baseline

- [ ] `target/release/sha256sums.txt` is the downloaded `SHA256SUMS` for the
      selected tag, not a manifest generated from local bytes.
- [ ] The exact asset set passes the command in the packet `README.md`.
- [ ] `uname -m` reports `x86_64`.
- [ ] `systemd --version` reports version 250 or newer.
- [ ] The host is a clean Ubuntu Server/qualifying Linux install with no
      graphical desktop, X11, Wayland, WebKitGTK, or Tauri packages.

Record the package inspection output here. The expected result for the
desktop-package query is no matching package names:

```text
uname -m
systemd --version
dpkg-query -W -f='${binary:Package}\n' | grep -Ei \
  '^(ubuntu-desktop|ubuntu-desktop-minimal|kubuntu-desktop|xubuntu-desktop|lubuntu-desktop|gnome-shell|plasma-desktop|xfce4|xorg|wayland|libwebkit2gtk)' || true
```

**Recorded output / interpretation:**

______________________________________________________________________________

______________________________________________________________________________

## Install and boot

- [ ] Install the published Linux headless archive using its `install.sh`.
- [ ] Record the one elevated installation window, if the installer requested
      it. Do not use `sudo` for pairing or ordinary agent operation.
- [ ] `com.ctemple.msc2.agent.service` is enabled for boot.
- [ ] `User=` and `Group=` are the installing user's UID and primary group,
      not `root`.
- [ ] The data, logs, and managed-server directories belong to the installing
      user; the installed binary and credential store retain their documented
      restricted ownership.
- [ ] Reboot the physical partition and confirm the agent is active before
      opening the desktop client.

Record the commands and relevant output, with no bearer credentials:

```text
systemctl is-enabled com.ctemple.msc2.agent.service
systemctl show com.ctemple.msc2.agent.service \
  -p User -p Group -p ActiveState -p SubState -p ExecMainStatus
stat -c '%U %G %a %n' ~/.local/share/msc2 ~/.local/share/msc2/logs
```

**Recorded service state:**

______________________________________________________________________________

______________________________________________________________________________

**Boot checkpoint after reboot:**

______________________________________________________________________________

## Tunnel and pairing

The management API remains loopback-only by default. From a client on another
network, use one of the documented management paths and record which one was
used:

- [ ] SSH local forward:
      `ssh -N -L 48001:127.0.0.1:48001 <installing-user>@<host>`
- [ ] Explicit Tailscale path: host name/address __________________________

Record the client network, host network, and how the tunnel was confirmed
without writing a token or pairing code here:

______________________________________________________________________________

______________________________________________________________________________

On the Linux host, as the installing user, run the local pairing command once:

```text
msc pairing create --client-kind desktop
```

- [ ] The short-lived code was displayed locally to the operator.
- [ ] The code was entered into the Tauri desktop pairing flow and was not
      pasted into this record, shell history, logs, or ordinary configuration.
- [ ] The Tauri client connected to the selected Linux host.
- [ ] Closing and reopening the Tauri client reconnects with the stored
      platform credential; no new pairing is needed.

**Pairing/reconnect observation:**

______________________________________________________________________________

## Minecraft and recovery

Use a disposable managed Java server for this handoff. Record its name and
server ID, not credentials:

**Server:** ______________________________  **Server ID:** ________________

- [ ] The Tauri client on the other network starts the remote Minecraft server
      through the agent API.
- [ ] The server reaches its expected running state.
- [ ] The Tauri client stops the remote Minecraft server cleanly.
- [ ] The agent is stopped through `systemctl`; the desktop reports the
      expected disconnect rather than silently claiming control.
- [ ] The agent is started again through `systemctl`; the Tauri client
      reconnects and the server inventory/lifecycle state is correct.
- [ ] Closing the Tauri client does not stop the agent or an intentionally
      running Minecraft process.

**Lifecycle and recovery observation:**

______________________________________________________________________________

______________________________________________________________________________

## Logs and final record

- [ ] `journalctl` contains an inspectable boot, pairing/service, lifecycle,
      stop, and recovery record without raw bearer credentials or pairing
      codes.
- [ ] Helper-socket ownership and mode are recorded:
      `/run/msc2/credential-helper.sock`.
- [ ] Any expected warning or unavailable behavior is recorded as such; no
      synthetic result is promoted to a pass.

```text
journalctl -u com.ctemple.msc2.agent.service -b --no-pager
stat -c '%U %G %a %n' /run/msc2/credential-helper.sock
```

**Log and ownership observation:**

______________________________________________________________________________

______________________________________________________________________________

**Linux result:**  Pending / Pass / Unavailable (leave gate open if not Pass)

**Operator initials:** ______________________
