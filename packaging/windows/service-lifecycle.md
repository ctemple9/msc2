# Windows headless service lifecycle

Run `install.ps1` from an elevated PowerShell window. The installer copies the
CLI, asks for the regular account that will own MSC 2 server files, grants
that account access to `%ProgramData%\MSC2`, and registers
`com.ctemple.msc2.agent` with automatic startup. An automation script may
pass a PowerShell `PSCredential` as `-ServiceCredential` to avoid an
interactive prompt. The password is supplied only while the Service Control
Manager registers the service.

The service launches the installed `msc.exe service-run` entry point.
`service-run` registers with the Service Control Manager, reports `RUNNING`
after the HTTP listener is ready, and handles stop and shutdown. `msc.exe
serve` remains the foreground command.

From the installing account's terminal, including a remote Windows login:

```powershell
msc.exe status agent
msc.exe stop agent
msc.exe start agent
msc.exe disable agent
msc.exe enable agent
```

Start, stop, enable, and disable require the Windows privilege to change a
service. A routine stop keeps automatic startup enabled, so the service
starts at the next boot. `disable agent` changes that boot policy without
stopping a running service. `uninstall.ps1` removes only the service that
points at its own installed executable, then removes its executable and PATH
entry. It leaves server files and `%ProgramData%\MSC2` in place.

## Headless updates

`msc.exe update install --release-id <release> --yes` copies the installed
binary to a separate updater executable under the staged release directory.
The updater waits for the confirming CLI to exit, then replaces the installed
binary. It identifies the service by its registered executable path. When
the service was running, it waits for `STOPPED`, installs the signed payload,
restarts the service, and checks `/v1/healthz` before considering the update
healthy. A service that was stopped before the update remains stopped.
