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

Protocol-capable MSIs now block ordinary downgrades. Deliberate restoration uses
`MSC_RESTORE_PREVIOUS=1`; the in-app updater uses it only for a verified retained
previous MSI. Historical MSIs do not acquire this guard retroactively. Full signed
update/recovery acceptance remains pending.

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

## Local agent replacement in P16.47

Setup now coordinates the fixed local Windows agent service in full, reduced and
silent installation modes. It checks the actual service account, command and MSC
metadata before changing anything. A copied desktop build must match the previously
installed package and its owner's data directory; later replacements use a protected
ownership record. A separate marked headless installation is left alone. An
unrecognized or inconsistent installation stops Setup for inspection.

For a running agent built with this step, Setup reserves the host against concurrent
server operations, requests a graceful Minecraft stop and waits for helper cleanup.
It replaces the service payload in a protected machine build directory and restores
the agent's previous running/stopped state. It keeps the existing account, stored
Windows password, startup policy, data paths and service permissions. It does not
automatically restart Minecraft. Initial service registration remains a first-launch
choice, including the one-time Windows service password prompt.

Older agents cannot acknowledge this shutdown request. Before replacing an older
running installation, stop Minecraft through MSC and wait until it reports stopped;
then stop the local agent. Run Setup again. Do not force-stop Minecraft to bypass
this requirement. A stopped older copied build can be migrated when its ownership
and payload match the old desktop package.

Failure/cancellation uses MSI rollback to restore the old package first, then the
service command, metadata and previous agent state. Previous immutable builds remain
for later update recovery. An externally changed service or failed restoration is
reported as an error; Setup does not overwrite an unknown service or force success.
Sudden power loss and post-install health rollback are not established by this step.

P16.48 supplies ordinary repair and safe package/service detachment with retained
data; separately confirmed complete removal remains a different operation. P16.49
adds signed update health checks and previous-package recovery. Physical acceptance
is pending; use the consolidated P16.50 walkthrough below.

## Cameron's P16.47 verification

Use disposable VMs with snapshots for service and rollback cases; Sandbox is useful
for the no-service installation. Record old/new MSI hashes and the service's account,
startup setting, binary/data paths and state before each case. `Get-CimInstance
Win32_Service -Filter "Name='com.ctemple.msc2.agent'"` reads the actual registration.

1. Install with no service. Confirm Setup does not create an agent service or ask
   for its password. First-launch local setup remains available.
2. Install an older desktop, create its local service, stop Minecraft and then the
   agent, and upgrade. Confirm the same account/startup setting, retained data and
   stopped agent; the binary should move to the protected digest-named machine build.
3. With an older agent still running, try the candidate. Confirm refusal before
   old-package removal, with the old desktop/service retained. Follow the explicit
   stop instructions and repeat.
4. With this candidate's agent running, reopen the candidate or upgrade with a later
   package built from this implementation. Include a running Minecraft server.
   Confirm Minecraft stops gracefully, helpers exit, and only the agent resumes.
   A previously stopped agent must stay stopped. Repeat with non-default boot policy.
5. Exercise cancellation while the service is stopped and a controlled package
   failure after replacement in a snapshotted VM. Inspect the verbose MSI log and
   confirm old files, service command/metadata and previous agent state return.
   Fault injection is acceptance work; no fault switch ships in the helper.
6. Repeat with a separately installed marked headless service, an ambiguous service
   registration, and elevation approved by another administrator. Headless must be
   untouched; ambiguous ownership must refuse changes; another administrator must
   not become the service owner. No remote host should receive a service action.

Record restoration failures and retained recovery records under
`C:\ProgramData\MSC2\Services\DesktopLifecycle` for inspection. Do not delete a
recovery record or edit its JSON to make Setup proceed.

## Current candidate review (P16.50)

Use the exact P16.49 local candidate and
[consolidated owner checklist](../release/windows-msi-review.md#p1650--consolidated-owner-acceptance-and-independent-review-handoff)
for installation, maintenance, service, UAC, scaling and recovery acceptance.
The earlier P16.46/P16.47 procedures above remain useful case details.
The local unsigned MSI cannot replace the two signed protocol-capable releases
required for in-app update acceptance. Both prior and target packages must support
`MSC_UPDATE_PROTOCOL=1`; migrate legacy installations manually first. Successful
updates require actual shell readiness and, when previously running, authenticated
owned-agent health. Restart-required results retain an explicit resume command;
failed replacement attempts verified prior-package/service restoration. No remote
service is controlled and Minecraft is not restarted automatically.
