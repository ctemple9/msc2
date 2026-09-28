# Windows headless service lifecycle

The headless archive installer copies `msc.exe` but does not register a service.
From an elevated PowerShell window, install one under the account that owns the
Minecraft server files. Replace the paths and account below with their actual
values; provide that account's service logon password through the installer's
`MSC2_WINDOWS_SERVICE_PASSWORD` environment variable.

```powershell
$env:MSC2_WINDOWS_SERVICE_PASSWORD = '<account password>'
msc.exe service install --service-name msc2-agent `
  --binary-path "$env:LOCALAPPDATA\MSC2\bin\msc.exe" `
  --working-directory "$env:LOCALAPPDATA\MSC2" `
  --log-path "$env:LOCALAPPDATA\MSC2\agent.log" `
  --run-user '.\<account name>' --expected-port 48001
Remove-Item Env:\MSC2_WINDOWS_SERVICE_PASSWORD
```

The Windows service adapter registers the production command as
`"...\msc.exe" service-run --service-name msc2-agent --bind 127.0.0.1:48001`.
`service-run` is internal: it registers with the Service Control Manager,
reports `RUNNING` after the HTTP listener is ready, and handles stop and
shutdown. `msc.exe serve` remains the ordinary foreground command.

On Windows, inspect the **installed** definition and its live behavior:

```powershell
sc.exe qc msc2-agent
sc.exe start msc2-agent
sc.exe queryex msc2-agent
Invoke-WebRequest http://127.0.0.1:48001/v1/health
sc.exe stop msc2-agent
sc.exe queryex msc2-agent
```

`BINARY_PATH_NAME` must name that installed `msc.exe service-run` command,
`SERVICE_START_NAME` must be the selected account, and the state should reach
`RUNNING` then `STOPPED`. The health request may require authorization; an
HTTP response still demonstrates that the service accepted a connection.
