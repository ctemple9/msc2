# Windows desktop installation

This describes the local P16.46 candidate for MSC 2 0.1.23. It is not a
publication or a claim that the complete Windows installation/update lifecycle
has passed acceptance. Published older packages can behave differently.

## What Setup installs

The x64 MSI installs the desktop app, `agent/msc.exe` (background agent and CLI),
`agent/vantage.exe`, `agent/VANTAGE-LICENSE.txt` and `agent/bedrock-map.exe`.
It installs for the computer and requires administrator approval. The default
folder is `C:\Program Files\MSC 2`; an existing MSI installation folder is
reused when found. An explicit folder selection takes precedence.

The Start-menu entry is shared by users of the computer. The optional desktop
shortcut is also shared; clearing its checkbox excludes that component.
Setup appends only its own `<installation folder>\agent` entry to machine PATH.
It does not replace PATH or delete another headless installation's distinct
entry. Existing terminals keep their old environment; open a new PowerShell
and run `Get-Command msc` to check which executable Windows finds first.
If another installation precedes this one, use the intended executable's full
path rather than deleting that installation's PATH entry.

Initial local service setup remains in the app's first-launch flow, including
Windows approval and the installing account's password. Installing a remote-only
desktop does not register a local service. Direct upgrades/removal are still
awaiting service coordination in P16.47–P16.49; use disposable installations for
this candidate's review.

## WebView2 and connectivity

The desktop needs Microsoft's shared WebView2 runtime. Setup checks its machine
registry registration and skips installation when a nonzero version is present.
A runtime installed privately for one account does not satisfy this machine-wide
package; Setup installs a shared runtime so another approving administrator's
private runtime cannot mask a missing dependency for the original user.

The MSI includes Microsoft's small Evergreen bootstrapper, not the full offline
runtime. If the machine runtime is missing, the bootstrapper downloads it from
Microsoft; internet access and permission to install it are required. Setup
shows this requirement before Install and identifies the action during progress.
The bootstrapper runs elevated, and a reported failure fails the MSI operation
rather than presenting successful completion or offering app launch. The failure
screen points to connectivity, organization policy and the installer log without
assuming every installation failure was caused by WebView2.

WebView2 is shared with other apps. MSC Setup does not uninstall it during package
removal or rollback. A failed MSC install can therefore leave an installed shared
runtime. Offline installation requires an already installed machine runtime;
this MSI does not contain the full runtime.

## Publisher and update identity

The product is MSC 2, the publisher field remains the existing `ctemple`, and
the installer version follows Tauri's version configuration. The verified MSI
upgrade family is pinned to `{816BE706-5775-5A3A-917C-339FBF0976E9}`.
The product/support/update links point to the repository, its Issues page and
Releases page. A publisher label is not a signature: the MSC MSI and application
remain unsigned, so Windows can report an unknown publisher or SmartScreen
warning. Microsoft's embedded bootstrapper has its own signature; that does not
authenticate the enclosing MSC installer.

Ordinary downgrade blocking remains deferred until P16.49 defines the verified
older-version recovery path. This candidate retains Tauri's existing downgrade
permission; do not treat it as a completed safe downgrade/update workflow.

## Cameron's P16.46 verification

Use Windows Sandbox for fresh installation and a disposable VM with a prior MSI
for upgrades/different-account approval. Leave valued worlds and the development
installation outside these exercises. Record each MSI hash and observed result.

1. With WebView2 already present, review Setup and confirm it reports the shared
   runtime present. Install, check the Start-menu/desktop choice, and use a new
   PowerShell to run `Get-Command msc` and `msc --help`.
2. In a VM where the machine runtime is genuinely absent, install with internet
   access; confirm the WebView2 action runs and the desktop opens afterward.
3. From that VM's clean snapshot, disconnect networking and repeat. Confirm
   failure instead of successful completion or automatic app launch. Do not
   remove WebView2 from the development computer to create this case.
4. Install the old MSI in a custom folder, then upgrade with this candidate.
   Confirm Setup discovers that folder, retains one Installed Apps registration
   and installs every payload file. Also exercise approval using a different
   administrator account, confirming shared shortcuts and machine markers.
5. With a separate headless installation present, record machine PATH before
   and after package changes. Its distinct directory must remain. Full removal
   and service lifecycle acceptance belong to later steps.

For a verbose log, run inside the disposable environment after copying the MSI:

```powershell
$installMsi = (Resolve-Path -LiteralPath '.\MSC 2_0.1.23_x64_en-US.msi').Path
$installLog = Join-Path $env:TEMP 'msc2-msi-install.log'
& "$env:SystemRoot\System32\msiexec.exe" /i $installMsi /norestart /L*V $installLog
```

This command can install or modify the disposable environment. Check logs for
personal paths/property values before sharing them; never supply service
passwords as MSI properties.

References: [Tauri Windows installer configuration](https://v2.tauri.app/distribute/windows-installer/),
[Microsoft WebView2 deployment](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution),
[MSI directory lookup through registered components](https://learn.microsoft.com/en-us/windows/win32/msi/complocator-table),
[machine desktop folder resolution](https://learn.microsoft.com/en-us/windows/win32/msi/desktopfolder),
and [MSI PATH entry semantics](https://learn.microsoft.com/en-us/windows/win32/msi/environment-table).
