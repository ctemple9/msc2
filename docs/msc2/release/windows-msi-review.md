# Windows MSI baseline and refinement review

Recorded 2026-10-07 in P16.44. Source baseline: `8af559be`.
Static inspection is complete; Cameron confirmed the artifact identification.
Physical verification remains pending. No installer was launched or package extracted.

## Exact artifact inspected

| Field | Observed value |
|---|---|
| Local path | `C:\Users\Cameron\Downloads\msc2-0.1.15-windows-x86_64.msi` |
| File size | 14,225,408 bytes |
| SHA-256 | `1952d89bb5538f2487e5563135619dfe063aee395edfe5effe107e4aa247ffc2` |
| Product/version | MSC 2 / 0.1.15 |
| Manufacturer | `ctemple` |
| ProductCode | `{3FCDD73A-E5EE-46FA-A73B-861DA8496F1A}` |
| UpgradeCode | `{816BE706-5775-5A3A-917C-339FBF0976E9}` |
| Product language | 1033, English (United States) |
| Installation scope | `ALLUSERS=1`; x64 Program Files destination |
| Signature | `Get-AuthenticodeSignature`: `NotSigned` |
| Owner-observed artifact | Cameron confirmed this v0.1.15 MSI during P16.44 |

The owner reported that the initially blank page **briefly appears and then
advances**, and confirmed this v0.1.15 MSI was the package he opened. That
observation does not establish that the preparation page is permanently stuck,
or that Cancel is authored disabled. This downloaded package is an exact inspectable baseline, not a
substitute for a current release artifact or physical UI evidence.

Inspection used the Windows Installer COM database API with
`OpenDatabase(path, 0)` (read-only), `Get-FileHash`, and
`Get-AuthenticodeSignature`. Tables examined: Property, Dialog, Control,
ControlEvent, ControlCondition, InstallUISequence, InstallExecuteSequence,
CustomAction, File, Directory, Feature, FeatureComponents, Component,
Shortcut, Registry, RegLocator, Upgrade and `_Tables`. No installation,
repair, removal, service control, test suite or build was performed.

## Preparation and navigation evidence

The UI sequence is:

| Order | Action/dialog | Condition or purpose |
|---|---|---|
| 25 | FindRelatedProducts | Identify an installed related product |
| 49 | PrepareDlg | Unconditional modeless preparation screen |
| 50 | AppSearch | Previous destination and WebView2 registry searches |
| 700 | ValidateProductID | Installer validation |
| 800–1000 | CostInitialize, FileCost, CostFinalize | Determine installation cost/destination |
| 1001 | SetARPINSTALLLOCATION | Publish destination property |
| 1200 | MigrateFeatureStates | Carry related-product feature selections |
| 1296 | MaintenanceWelcomeDlg | Installed, no resume/preselection/patch |
| 1297 | ResumeDlg | Installed and resume/preselection |
| 1298 | WelcomeDlg | New product or patch |
| 1299 | ProgressDlg | Execution progress |
| 1300 | ExecuteAction | Enter installation execution |

`PrepareDlg` is 370 × 270 installer units, positioned at 50/50 percent,
with Dialog attributes 5 (visible, modeless). Its Title and Description
controls have attributes 196611, including Visible and Enabled. The title
contains a product-name substitution and the description contains preparation
copy; these are not missing strings. Its bitmap occupies the upper 370 × 234
area, and transparent text controls overlay it.

Back and Next have attributes 1 (visible, not enabled). Cancel has attributes
3 (visible and enabled), is the default/cancel control and publishes
`SpawnDialog=CancelDlg`. No ControlCondition row disables PrepareDlg Cancel
or hides its title/description. The owner-visible disabled appearance can
therefore be a transient painting/responsiveness issue rather than a database
instruction to disable all three buttons. Its precise cause remains unproved
without observing the exact artifact. Searches/costing occur after this
modeless dialog appears; a briefly unpainted preparation page is consistent
with that ordering, but static inspection cannot measure rendering time.

Welcome Next has an order-2 transition to InstallDirDlg, overriding the
order-1 stock license transition. Destination Back similarly returns to
Welcome at order 2. A license dialog exists in the database but is bypassed
in this package's normal new-install path. Destination Next sets the target
path, validates it, shows InvalidDirDlg on failure and advances to
VerifyReadyDlg only when valid. Cancel uses the standard confirmation dialog.

## Screen-by-screen before/after map

These are database/source observations and proposed outcomes, not assertions
that the new screens have been implemented or visually accepted.

| Screen/path | Baseline behavior | Planned outcome / step |
|---|---|---|
| Preparation | Modeless screen before searches/costing; Back/Next disabled; Cancel authored enabled; owner reports brief blankness | Readable preparation status, or suppress the unreliable transient dialog; no sleeps or premature Next. P16.45 |
| Welcome | Generic product setup introduction; Next/Cancel enabled, Back disabled | Explain Minecraft management, included app/agent/CLI, version and first-launch hosting/remote choices. P16.45 |
| Destination | Program Files/MSC 2 default; Change opens a directory browser; local-path validation | Clear scope, folder and shortcut choices; CLI discovery explained. P16.45–46 |
| Ready | Generic install/repair/remove text and conditional shield buttons | Actual destination/version transition, prerequisites, elevation and any explicit local-server stop requirement. P16.45–47 |
| Progress | Modeless stock action/status/progress controls; Cancel confirmation; stock mode-specific titles | Real phases and honest cancellation/recovery status. P16.45,47,49 |
| Completion | Generic completion; checked Launch MSC 2 option shown only when not Installed | Accurate result, first-launch guidance and reboot/recovery state; one relaunch owner. P16.45,49 |
| Same-version maintenance | Welcome → operation chooser → Ready; Change and Repair disabled by properties | Deliberate supported repair/removal choices and retention explanation. P16.48 |
| Upgrade | FindRelatedProducts, migrate features, remove old product during execution; new package follows new-product welcome path | Explicit existing/target versions; owned service/payload coordination and recovery. P16.46–49 |
| Ordinary removal | Package removal; no custom service/data cleanup | Detach verified package-owned service safely, retain worlds/data; clearly distinguish complete destructive cleanup. P16.47–48 |
| Failure/cancel/resume | Standard FatalError/UserExit/Resume and files-in-use/disk-space dialogs | Specific useful result and retained/recovered state; no false healthy-agent claim. P16.45,47–49 |

Maintenance is more restricted than the presence of generic dialog rows
suggests: `ARPNOMODIFY=1` and `ARPNOREPAIR=yes` cause ControlCondition entries
to disable ChangeButton and RepairButton and show their disabled explanations.
Remove remains enabled unless ARPNOREMOVE is supplied. Do not equate authored
repair controls with a usable repair feature in this artifact.

Completion Finish launches `LaunchApplication` only when the checkbox is 1
and the product is not Installed. A separate execution-sequence launch at
6601 uses `AUTOLAUNCHAPP AND NOT Installed`. Future updater work must account
for both launch mechanisms. This baseline does not prove that duplicate
launch currently occurs with the existing updater's arguments.

## Package identity, payload and actions

The File table has exactly two entries:

| Installed file | Uncompressed table size | Version |
|---|---|---|
| `msc2-desktop-web.exe` | 17,106,944 bytes | 0.1.15.0 |
| `agent/msc.exe` | 23,139,328 bytes | No File-table version |

There is no Vantage renderer, Bedrock terrain exporter or Vantage license in
this artifact's File table. These absent helpers do not describe today's
package, which stages them explicitly. Shortcuts target the desktop executable
in Start Menu and Desktop; an uninstall shortcut inside INSTALLDIR invokes
`[System64Folder]msiexec.exe /x [ProductCode]`.

The Feature table includes Application, Shortcuts, Environment and External.
The feature named Environment references the desktop executable component,
but `_Tables` contains **no Environment table**. Its label is not evidence
that this MSI adds either executable to PATH. There are also no ServiceInstall
or ServiceControl tables, no stop/start service execution actions and no
MSC-specific lifecycle custom action.

All custom actions are LaunchApplication, WixUIValidatePath, WixUIPrintEula,
SetARPNOMODIFY, DownloadAndInvokeBootstrapper and SetARPINSTALLLOCATION.
The WebView2 action executes at 6599, before InstallFinalize (6600), only when
neither removal nor INSTALLED_WEBVIEW2_VERSION applies. It retrieves Microsoft's
bootstrapper using hidden PowerShell and waits for a silent runtime install.
That is execution-stage work, not a demonstrated cause of opening-page
blankness. Registry searches cover user and machine runtime registrations.

Four package registry values use HKCU `Software\ctemple\MSC 2`: InstallDir,
Start Menu Shortcut, Desktop Shortcut and Uninstaller Shortcut. Previous-folder
searches also use HKCU, while the package installs machine-wide. This mixed
scope needs a different-administrator elevation exercise in P16.46; the table
alone does not prove which user's registry receives every operation.

RemoveExistingProducts is scheduled at 1501, directly after InstallInitialize
(1500), before InstallFiles (4000). This ordering is relevant to future
service/rollback actions: old-product removal can occur before replacement
files exist. The Upgrade table accepts related versions from 0 upward,
attributes 257, without an upper version limit. It does not implement ordinary
older-version refusal. Preserve the verified UpgradeCode when refining the
installer so existing installations remain in the same product family.

## Comparison with current v0.1.23 source

`package-lock.json` and installed `@tauri-apps/cli/package.json` both resolve
CLI **2.11.4**. The original CLI version that built the old MSI has not been
established; matching template structure does not establish identical tooling.

| Area | Current source evidence | Implication |
|---|---|---|
| UI | `clients/desktop-web/src-tauri/tauri.conf.json` has no custom template/UI/localization/artwork override | Current build uses stock screens, subject to the locked CLI |
| PATH | `packaging/windows/desktop-cli-path.wxs` adds machine `[INSTALLDIR]agent` PATH and HKLM `Software\CTemple\MSC2` ownership marker | Current source corrects the old package's missing CLI PATH registration; next actual MSI must be inspected |
| Payload | `tools/release/stage-windows-agent.ps1` stages agent, Vantage, its license and Bedrock exporter | Old two-file payload cannot validate current release completeness |
| WebView2/downgrades | Options omitted; local CLI schema defaults downloadBootstrapper/silent and allowDowngrades=true | Explicit reviewed policy still required |
| Service setup | `lib.rs::agent_install_request` uses the staged build; `stage_packaged_agent` hashes/copies the three helpers into `agent/builds/<digest>` | Running service normally uses a separate copied payload; replacing packaged files alone does not coordinate its replacement |
| Data | Windows desktop data defaults to installing user's `AppData/Roaming/MSC2` | Service/data ownership must follow the original account and metadata |
| In-app update | `update.rs::install_windows_msi` runs `/i`, accepts only success(), then spawns current executable | No explicit Windows health/previous-package restoration; cancellation and reboot-success handling need refinement |
| Full cleanup | `crates/msc-platform-windows/src/uninstall.rs` removes services before `/x /passive /norestart`; accepts 0/3010 | Existing Phase 19 confirmed cleanup is separate from direct package removal; maintain compatibility |
| Release | `.github/workflows/release.yml` stages payload, prepares ICO, builds unsigned MSI, collects artifacts | Keep build-only publication and existing signed update metadata; no additional release gate |

This comparison does not assert that a v0.1.23 MSI was physically inspected,
installed, successfully upgraded or accepted. P16.46/P16.50 must inspect and
observe the candidate built from the changed source.

## Cameron's narrow reproduction procedure

Use a disposable Windows VM/Sandbox or a separate disposable installed MSC
environment. The command below launches the installer and **can install or
modify MSC if its installation button is used**; it is not a read-only check.
It was not run by the agent. Copy the identified MSI into that environment
first; keep the existing development machine and valued worlds out of this
exercise.

```powershell
$reviewMsi = (Resolve-Path -LiteralPath '.\msc2-0.1.15-windows-x86_64.msi').Path
Get-FileHash -LiteralPath $reviewMsi -Algorithm SHA256
$reviewLog = Join-Path $env:TEMP 'msc2-msi-opening-review.log'
& "$env:SystemRoot\System32\msiexec.exe" /i $reviewMsi /L*V $reviewLog
```

For the opening-page observation, record the filename/hash, Windows version,
display scaling, whether a related product is already installed, approximate
blank duration and whether text/buttons subsequently paint. Capture the
preparation screen and subsequent Welcome page if possible. Review
Welcome → destination → Ready, then Cancel **before Install**. Use the log to
identify action/dialog ordering and lengthy searches/costing; an MSI log alone
cannot prove what pixels were drawn. Do not infer a WebView2 opening delay
merely because its registry search appears in AppSearch.

Record separate later exercises for actual installation/maintenance/update/
removal rather than clicking through destructive paths during this opening
reproduction. Share only relevant log excerpts after checking for personal
paths and supplied property values. No service passwords should be supplied
as MSI command-line properties.

## Evidence still awaiting owner verification

- Artifact identification is confirmed by Cameron; no further filename confirmation is needed.
- Observe preparation/Welcome and Cancel responsiveness in the disposable
  environment; explain any difference from the table-authored state.
- Validate the future current-source MSI's helpers, PATH, maintenance,
  ownership, upgrade/recovery and removal behavior in their named steps.

P16.44 does not mark the blank-page cause proved, change installer behavior,
close any phase gate or authorize P16.45 implementation.

## Primary sources for the next steps

- [Locked CLI 2.11.4 MSI template](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.4/crates/tauri-bundler/src/bundle/windows/msi/main.wxs).
- [WiX PrepareDlg source](https://github.com/wixtoolset/wix3/blob/develop/src/ext/UIExtension/wixlib/PrepareDlg.wxs).
- [Tauri Windows installer options](https://v2.tauri.app/distribute/windows-installer/).
- [Microsoft MSI control attributes](https://learn.microsoft.com/en-us/windows/win32/msi/control-attributes).
- [Microsoft MSI command-line options](https://learn.microsoft.com/en-us/windows/win32/msi/command-line-options).
- [Microsoft MSI result codes](https://learn.microsoft.com/en-us/windows/win32/msi/error-codes).
