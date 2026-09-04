# Physical Windows beta handoff

**Machine:** ______________________________  
**Date (UTC):** ____________________________  
**Windows release:** _______________________  
**Architecture:** __________________________  
**Installing user:** ________________________  
**Release tag:** ___________________________  
**Desktop asset:** _________________________  
**Desktop asset SHA-256:** _________________________________________________

Run this on a clean physical x86_64 Windows installation or partition. The
unknown-publisher warning is expected for this unsigned beta and must be
recorded, not silently treated as a signing failure or ignored.

## Artifact and installer launch

- [ ] The exact published asset set passes the command in the packet
      `README.md`.
- [ ] The downloaded Windows installer filename appears in
      `target/release/sha256sums.txt` with the recorded SHA-256.
- [ ] `Get-FileHash` independently reports the same SHA-256:

```powershell
Get-FileHash .\msc2-<release>-windows-x86_64.msi -Algorithm SHA256
```

- [ ] Launching the installer shows the expected SmartScreen/unknown-publisher
      warning, and the observed wording is recorded below.
- [ ] The installer completes without adding a second agent or requiring a
      graphical session for the agent service.
- [ ] The installed Tauri desktop launches and embeds the matching agent/CLI
      version.

**Checksum and installer observation:**

______________________________________________________________________________

______________________________________________________________________________

**Unsigned warning wording / screenshot reference:**

______________________________________________________________________________

## Tauri pairing and reconnect

Pair the installed Tauri client with the selected agent through the documented
SSH/Tailscale path. The client may control Minecraft remotely, but it must not
gain operating-system service-management authority on the host.

- [ ] Pairing succeeds with the one-use code from the host-local command.
- [ ] The selected host and server context load in the Tauri desktop.
- [ ] Closing and reopening Tauri reconnects using the stored platform
      credential without a second pairing code.
- [ ] Stopping or restarting the agent is only performed through the host's
      local service-management path, not a remote Tauri control.

**Pairing/reconnect observation:**

______________________________________________________________________________

## Windows service ownership

The service must run as the installing user, not `LocalSystem`. Use the
service name recorded by the installer; the beta contract's canonical name is
`com.ctemple.msc2.agent`.

```powershell
Get-CimInstance Win32_Service -Filter "Name='com.ctemple.msc2.agent'" |
  Format-List Name,State,StartMode,StartName,PathName
```

- [ ] The service exists and is set to start automatically.
- [ ] `StartName` is the installing user's account.
- [ ] `StartName` is not `LocalSystem`, `NT AUTHORITY\LocalSystem`, or an
      unexpected shared service account.
- [ ] The service remains running after the Tauri client closes.
- [ ] A disposable local Minecraft server, if used, remains under agent
      lifecycle control rather than becoming a GUI-owned process.
- [ ] After signing out and signing back in, the service is still registered,
      starts as the installing user, and the expected Minecraft/service state
      is recoverable.

**Service query before sign-out:**

______________________________________________________________________________

______________________________________________________________________________

**Service query after sign-out/sign-in:**

______________________________________________________________________________

______________________________________________________________________________

## Remote Minecraft lifecycle and recovery

- [ ] From the paired Tauri client on the other network, start a disposable
      Minecraft server through the agent API.
- [ ] Confirm the server reaches running state, then stop it through the same
      client.
- [ ] Exercise a local agent stop/start through the Windows Service Control
      Manager and record the reconnect result:

```powershell
sc.exe stop com.ctemple.msc2.agent
sc.exe start com.ctemple.msc2.agent
```

- [ ] The agent returns to the expected running state and Tauri reconnects.
- [ ] A remote Tauri client cannot install, stop, or uninstall the Windows
      operating-system service.

**Lifecycle and recovery observation:**

______________________________________________________________________________

______________________________________________________________________________

**Windows result:**  Pending / Pass / Unavailable (leave gate open if not Pass)

**Operator initials:** ______________________
