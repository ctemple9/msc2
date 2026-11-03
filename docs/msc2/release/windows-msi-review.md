# Windows MSI baseline and refinement review

> Privacy cleanup (2026-10-08): personal account paths and captured network addresses in this record have been replaced with examples. Substitute your own paths and addresses when following commands; example values are not the original observations.

Recorded 2026-10-07 in P16.44. Source baseline: `8af559be`.
Static inspection is complete; Cameron confirmed the artifact identification.
Physical verification remains pending. No installer was launched or package extracted.

## Exact artifact inspected

| Field | Observed value |
|---|---|
| Local path | `C:\Users\example\Downloads\msc2-0.1.15-windows-x86_64.msi` |
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

## P16.45 implementation and local candidate

The maintained template derives from locked Tauri CLI 2.11.4, with MIT
attribution retained beside it. It preserves generated resources, architecture
handling, component identities and WebView2 bootstrap behavior. Localized
native-control screens introduce MSC, offer destination and a real optional
desktop shortcut, review version/scope/folder/prerequisites, show MSI progress
and explain first launch. Text is authored in controls, not bitmap artwork.
The CLI PATH fragment and Start-menu discovery remain in the package.

The transient preparation screen is suppressed with a false sequence condition,
without sleeps or premature navigation. Searches/costing finish before the
interactive Welcome screen. Required stock auxiliary dialog dependencies remain
compiled, but stock Welcome/Progress/Exit screens are disabled in the sequence.
Custom dialogs use separate IDs. Navigation, destination validation, disk-space
checks, cancellation and license routes are wired explicitly. Completion launch
requires selection and excludes maintenance/removal, automatic launch and known
reboot conditions.

Local unsigned release candidate (not published or physically accepted):

- Path: `clients/desktop-web/src-tauri/target/release/bundle/msi/MSC 2_0.1.23_x64_en-US.msi`.
- Size: 16,990,208 bytes.
- SHA-256: `3a2b768c891bd8df7fff5b316e23a6d54edf8082dfee69df2224ba249dc1518b`.
- ProductVersion: `0.1.23`.
- UpgradeCode: `{816BE706-5775-5A3A-917C-339FBF0976E9}`, matching the baseline.

The requested build compiled the frontend, release agent and release desktop.
Initial packaging failed on WiX dialog/event conflicts; those failures were
inspected and corrected before successful final release packaging with
`npx tauri bundle --bundles msi --no-sign --verbose`. Intermediate debug bundling
encountered an in-use executable and was not used as this candidate. Release
rebundling warned that the bundle-type marker had already been patched by the
earlier build. No Rust source changed.

Final packaging passed normal MSI validation. Retained upstream warnings were
ICE03 (WebView2 action target length), ICE40 (REINSTALLMODE), ICE57 (shortcut
registry/machine scope) and ICE61 (downgrade policy). Identity/scope and recovery
policy remain for P16.46 onward; no release gates were added. XML parsing,
config formatting and diff checks passed. No tests were run.

Read-only Windows Installer database inspection of this exact candidate found:

- Stock preparation/Welcome/Progress/Exit sequence conditions are `0`;
  custom Welcome is at 1294 after costing, Progress at 1299 and Exit on success
  in both install and administrator UI sequences.
- Navigation, validation, cancellation and completion launch conditions are
  present in ControlEvent/ControlCondition tables.
- The desktop shortcut is transitive, conditioned on
  `MSC_DESKTOP_SHORTCUT = "1"`, and retains baseline component identity
  `{8920A060-D815-5BCF-BF89-20659E1F4D58}` and registry keypath.
- File rows include desktop, agent/CLI, Vantage renderer/license and Bedrock
  exporter. The Environment table retains the package-owned agent PATH entry.
  Shortcut and launch choices default to selected.

Tables establish authored behavior, not painted pixels or successful
installation. Cameron must review screens, keyboard navigation, Back/Cancel
before installation and layouts at 100%, 125%, 150% and 200% scaling in a
disposable environment. No installer was launched, service action performed or
release workflow run. Service coordination, enabled repair/removal and updater
recovery remain P16.47–P16.49; the introduction does not claim those complete.

## P16.46 identity, machine registration and prerequisite candidate

Cameron reported that the P16.45 installer looked good after the Sandbox review
instructions. This records his visual observation only; exact scaling, keyboard
and installation results were not supplied and are not inferred.

P16.46 pins the verified UpgradeCode in Tauri configuration, preserves publisher
`ctemple`, product MSC 2 and configured version, and supplies existing real
repository/Issues/Releases destinations for Installed Apps metadata. It preserves
the existing desktop file component and CLI PATH component identities. The old
v0.1.15 and new candidate both identify the desktop file component as
`{435E9180-FA08-5CE9-9162-90B8459E20F5}`; this is the component used for lookup.
Moving InstallDir/shortcut markers out of HKCU requires new component identities because
their registry keypaths changed; the old components are not silently repurposed.
No service registration, payload staging or Rust source changed.

Previous-directory lookup now prefers machine `DesktopInstallDir`, then MSI's
registered desktop file component, then the old user's HKCU marker when a related
upgrade is detected. Explicit INSTALLDIR wins. The registered component lookup
uses the generated desktop GUID, not an arbitrary folder/file-name search, and
works without depending on the approving administrator's HKCU marker. Lookup and
directory-setting actions run in both UI and execution sequences, before costing.
The old user's leftover HKCU markers are not swept across account hives.

Shortcuts use common-folder aliases resolved from MSI's standard folder
properties after CostInitialize. ALLUSERS=1 is enforced. Shortcut keypaths use
HKMU (Registry Root=-1), which resolves to HKLM under that installation scope;
InstallDir and uninstall shortcut markers use HKLM directly. The desktop choice
remains an actual conditional/transitive component. Machine PATH remains the
unchanged single Environment row `=-*PATH`, `[~];[INSTALLDIR]agent`, with CLI
component `{6E7F2CAA-693C-598D-AC9E-C072B6B8CF58}`. No blanket PATH cleanup or
standalone-headless marker removal was added.

Decided within this step: embed the small online Evergreen bootstrapper rather
than downloading it into a temporary path through the inherited PowerShell
command. Both are online strategies; embedding adds about 1.85 MB and lets MSI
invoke the binary directly and check its result. The full offline runtime is not
bundled. Machine-runtime detection avoids treating another administrator's
per-user runtime as available to all users. Empty/`0.0.0.0` versions schedule the
bootstrapper; a present nonzero machine version skips it. The action runs deferred
without user impersonation, `/silent /install`, checked return code, before
InstallFinalize. Localized progress and an MSC failure screen explain connectivity
and log diagnosis. Shared WebView2 is not removed on uninstall/rollback.

Downgrade permission is explicitly kept at its prior value (`true`). P16.49 must
define signed previous-version restoration before ordinary downgrades are blocked.
This does not amend D-032 or claim recovery is implemented.

Exact local unsigned candidate replacing the P16.45 bytes at the same path:

| Field | Observed value |
|---|---|
| Path | `clients/desktop-web/src-tauri/target/release/bundle/msi/MSC 2_0.1.23_x64_en-US.msi` |
| Bytes | 18,845,696 |
| SHA-256 | `37238381fdd79dd606697f0e82daa94a37e7464785ce19369cde053274d243cc` |
| Product / publisher / version | MSC 2 / ctemple / 0.1.23 |
| UpgradeCode | `{816BE706-5775-5A3A-917C-339FBF0976E9}` |
| MSC Authenticode status | NotSigned |
| Embedded bootstrapper source status | Valid; Microsoft Corporation |
| Bootstrapper source SHA-256 | `aa38a8cfce6179b87181609b1c730a29eaf26138fc833af5759e67576770f3a3` |

Read-only candidate database inspection confirmed all five payload File rows
(desktop, agent/CLI, Bedrock exporter, Vantage executable/license), support metadata,
machine/scope-aware registry markers, folder discovery, unchanged PATH semantics,
secure directory handoff properties, and bootstrapper CustomAction type 3074
(deferred executable binary, no impersonation, checked result). The bootstrapper
has ActionText and the failure dialog runs on error in both UI sequences.

Bundle-only packaging reused the P16.45 release binary. Initial MSI validation
rejected HKLM shortcut keypaths under dual-purpose standard folder identifiers;
the inspected errors led to scope-aware keypaths and explicit common-folder
aliases, without disabling validation. Final
`npx tauri bundle --bundles msi --no-sign --verbose` passed. The inherited ICE03
and ICE57 warnings are absent; ICE40 (reinstall policy) and ICE61 (deliberately
deferred downgrade restriction) remain. Tauri also reports its previously patched
bundle-type marker warning during rebundling. No tests, native rebuilds, installer
launches, service operations, tags or release runs occurred.

Physical results remain pending. The [Windows installation walkthrough](../clients/windows-installation.md)
covers new-terminal CLI discovery, old-version/custom-folder upgrade, headless
PATH coexistence, different-administrator approval and missing-runtime online/
offline failure cases. Static authoring/packaging evidence is not proof of those
results or of P16.47–P16.49 lifecycle/recovery behavior.

## P16.47 owned local service lifecycle candidate

Implemented from P16.46 source `38052be7`; commit subject
`P16.47: coordinate windows msi with the owned local agent`. No version bump or
publication. The exact unsigned candidate replaces the prior bytes at the build path.

| Field | Observed value |
|---|---|
| Build path | `clients/desktop-web/src-tauri/target/release/bundle/msi/MSC 2_0.1.23_x64_en-US.msi` |
| Review copy | `C:\Users\example\Downloads\msc2-p16.47-windows-x86_64.msi` |
| Bytes | 19,152,896 |
| SHA-256 | `8d2e3a3f6db651c704f00a0add6eb8c35441119d21c3616b55fe163c404e8a08` |
| Embedded x64 lifecycle DLL | 292,352 bytes; SHA-256 `a68c58b0959d5b02d1993e8baf619185480e2001b5b34adeb10b3b749c66eece` |
| Product / version / signature | MSC 2 / 0.1.23 / NotSigned |

MSI's Binary table contains the native helper; it cannot select an external helper
script, registry command or executable supplied by a caller. The DLL launches the
system-directory PowerShell with its fixed embedded code and literal JSON request
through stdin. Fixed native operations inspect the local SCM service and change its
binary command without supplying an account/password. Directory handles pin paths
against replacement, reparse/remote/alternate-stream paths are refused, recovery
records and immutable payload directories have administrator/System ownership and
restricted write permissions, and metadata replacement is atomic. The four payload
hashes are compiled into the DLL, not taken from MSI properties. Read-only stream
inspection confirmed the embedded DLL equals the staged file and contains the hashes
of the current agent, Vantage executable/license and Bedrock exporter.

Ownership checks compare SCM command/account with metadata. A legacy copied desktop
build also needs the original account's registered profile/data root, a recognized
checksum-named build path and an exact digest match to the old MSI's registered
desktop payload. An administrator-owned record binds later service replacements to
the desktop directory, binary, data root and account SID. Marked standalone headless
services remain independent; missing/inconsistent ownership refuses changes.

SCM control 128 requests maintenance on the local agent only. The agent reserves the
whole host against conflicting operations, closes new map work, requests Minecraft
graceful stop and checks process state without treating errors as stopped. Playit
reset is used here only to synchronously stop its helper and remove its temporary
credential bridge; the stored host key/tunnels/settings are retained. Xbox Broadcast
exit is polled, and terrain helpers' shutdown/exit errors prevent a successful
acknowledgement. No forced Minecraft stop or remote API operation is used. Older
running agents lack the protocol and require an explicit Minecraft stop followed
by agent stop before Setup can proceed.

Verified staged payloads move into
`C:\ProgramData\MSC2\Services\DesktopLifecycle\builds\<digest>`; partial copies do
not become final immutable files. Account, stored Windows password, service ACL,
data/log/environment paths and boot policy remain. A previously running agent
resumes; a stopped agent stays stopped. A running service configured Disabled is
temporarily set to demand start for the explicit restart, then restored to Disabled;
that temporary change is recorded for rollback. Minecraft is not automatically
restarted. Previous builds remain for P16.49 health recovery. Existing recovery
records block another transaction instead of being silently discarded.

Read-only inspection found these execution positions in the exact final candidate:

| Action | Sequence | Purpose |
|---|---:|---|
| InstallInitialize | 1500 | Begin transaction |
| MscRequireRollback | 1501 | Refuse disabled Windows Installer rollback |
| MscRollbackService | 1502 | Queue restoration before stopping anything |
| MscPrepareService | 1503 | Queue ownership/snapshot/graceful stop |
| InstallExecute | 1504 | Execute preparation before old-product removal |
| RemoveExistingProducts | 1505 | Remove old package inside rollback |
| ProcessComponents | 1600 | New component changes follow removal |
| InstallFiles | 4000 | Install new payload |
| MscApplyService | 4001 | Queue hash-verified staged replacement |
| MscResumeService | 4002 | Queue prior agent state restoration |
| MscCommitService | 6599 | Queue successful transaction-record cleanup |
| InstallFinalize | 6600 | Execute remaining script and commit |

This early script flush avoids relying on stable component GUIDs for Tauri-generated
resource files: removal precedes new file installation. Rollback of old-product
removal precedes the earlier queued service restoration. Required actions are in the
execution sequence, so reduced/silent modes follow the same ownership rules.
Old-product removal skips these service actions using `UPGRADINGPRODUCTCODE`.
CustomAction types are 3073 for checked deferred elevated DLL calls, 3329 for checked
rollback and 3649 for commit cleanup. The fixed rollback-disabled error is type 19.
First registration remains on first launch; it refuses an existing SCM service before
asking for credentials, rather than deleting/recreating an existing account binding.

Welcome/review copy now discloses the Minecraft stop before Update/Repair is
confirmed. Maintenance completion describes the coordinated agent's prior state
instead of claiming that service coordination is separate. Existing dialog layouts
are retained; the changed text still needs Cameron's physical/scaling review.

Checks completed: desktop Verify Clippy, agent/infrastructure and helper Clippy,
Rust formatting, PowerShell parsing and embedded C# compilation, native release
build and MSI packaging with normal validation, exact database/stream/hash/signature
inspection and diff checks. Existing Rust warnings remain; no new Rust warning.
The first native build was followed by targeted agent/helper rebuilds for confirmed
shutdown/rollback gaps and bundle-only refreshes; no release workflow was run.
Final WiX warnings remain ICE40 (existing reinstall policy) and ICE61 (downgrade
policy deferred to P16.49). No validation suppression or release/test gate was added.

One essential fake-filesystem regression covers service maintenance conflicting
with server start in both admission orders. Existing target-only exclusivity cases
miss this host-wide reservation. Expected runtime is under one second, with no real
processes, network, sleeps or machine-specific filesystem; it was **not run**.
No tests, installer launches, service changes, tags or publication were performed.

Physical acceptance remains pending. Follow the
[P16.47 Windows walkthrough](../clients/windows-installation.md#camerons-p1647-verification)
for no-service, stopped/running owned agent, legacy refusal, headless coexistence,
different-administrator approval, non-default boot policy and cancellation/fault
restoration. Static evidence does not establish the SCM handshake or rollback on a
real machine. Abrupt power-loss recovery and health-triggered update rollback remain
unverified/P16.49. This intermediate candidate refuses ordinary owned-agent removal
until P16.48 supplies detachment; full-removal integration/remaining ownership-record
cleanup must be checked there. Do not treat Phase 16's gate as closed.

References: [Microsoft execution/removal sequencing](https://learn.microsoft.com/en-us/windows/win32/msi/removeexistingproducts-action),
[WiX major-upgrade/component constraints](https://docs.firegiant.com/wix/schema/wxs/majorupgrade/),
and [Microsoft rollback-disabled behavior](https://learn.microsoft.com/en-us/windows/win32/msi/rollbackdisabled).

## P16.48 ? Repair and ordinary package removal

Implementation is awaiting Cameron's exact-artifact verification. Windows app
maintenance is no longer hidden by ARPNOREPAIR/ARPNOMODIFY. Reopened MSI offers a
custom MSC repair/removal chooser and a final review explaining graceful shutdown
and retained data. Repair reinstalls all package bytes (`amus`) and preserves the
service account, password, delayed/ordinary boot policy and prior running state.
A legacy copied payload can be identified by compiled hashes for same-product
repair even if installed package files are missing. Major-upgrade child removal
continues to skip all service actions.

Removal prepares the same protected snapshot and graceful stop, then disables
boot while the package is removed. It retains copied payloads and SCM credentials
through deferred file operations. A separate checked elevated commit action
removes metadata/ownership and finally marks the fixed service for deletion.
Before detachment, rollback restores metadata, ownership, boot policy and agent
state. Detachment is the last fallible native operation; transaction cleanup
failure afterward emits a recovery-record warning rather than requesting an
impossible password restoration. Full removal is supported after its separately
confirmed worker removes the service: its adapter validates and removes only the
protected, exact-name/hash copied cache before MSI performs package-only removal.
No user-data path from an owner record is used as a deletion target.

Windows service deletion can remain pending while another program holds a service
handle. Close Services or restart before reinstalling local hosting if Windows
reports pending deletion. A later unrelated Windows Installer commit failure
cannot automatically recreate the deleted account password; the failure dialog
and rollback error require reinstall/local hosting setup and preserve a recovery
record instead of claiming complete restoration. This is a commit-boundary
limitation, not evidence of physical rollback acceptance.

Microsoft documents that [commit failures can trigger rollback which cannot undo
every commit change](https://learn.microsoft.com/en-us/windows/win32/msi/commit-custom-actions)
and that [service deletion waits for stopped state and open handles to close](https://learn.microsoft.com/en-us/windows/win32/api/winsvc/nf-winsvc-deleteservice).

### Cameron's P16.48 verification

Use a disposable installed VM, keep worlds/settings/backup samples and record
service account, command, boot mode and running state beforehand. Capture verbose
MSI logs with `/l*v` to a path outside MSC data directories.

1. Reopen this exact MSI. Confirm the MSC repair/remove chooser, final review and
   readable dialog text at your display scaling; Cancel changes nothing.
2. Delete a package-owned app/tool file in the VM and Repair. Confirm it is
   restored, settings/worlds/credentials match and the service account/boot policy
   remain. Check both Running and Stopped agents; Minecraft remains stopped.
3. Remove from Windows' app list and, on a fresh disposable install, from the
   reopened MSI. Confirm app/tools/shortcuts/PATH removal, no Launch option,
   service detached and server/settings/backup/credential files retained.
4. Repeat with no local service, an independently marked headless service and
   administrator approval using another account. Unrelated services remain.
5. Refuse UAC or cancel before changes. Separately interrupt/fail a reversible
   removal action while the service is disabled; inspect log and confirm rollback
   restores files/metadata/account/boot state and the previous agent state.
6. Reinstall after ordinary removal. Confirm retained data is available and local
   hosting setup asks for Windows credentials again. Close Services/reboot if
   Windows still reports the deleted service as pending removal.
7. On a separate disposable installation, run the existing confirmed full-removal
   flow. Confirm its report records actual completion, protected copied builds are
   removed, MSI package removal succeeds after service removal, and its reviewed
   data/credentials are cleared. Unknown cache files or an unfinished transaction
   must produce a retained/partial result, never recursive deletion.

Commands for package building remain the P16.48 Verify line. No live install,
removal, service mutation, tests, tags or publication are agent verification.

### Exact P16.48 candidate and build evidence

- Review copy: `C:\Users\example\Downloads\msc2-p16.48-windows-x86_64.msi`.
- Product version: **0.1.23**; unsigned MSI; **19,156,992 bytes**.
- MSI SHA-256: `f4b4a12fdd17a33584f1dbca503691c2f380e219d899501ab408c0a781c317bc`.
- Embedded lifecycle DLL: **295,936 bytes**, SHA-256
  `6b08cf93866fec57a037bad6bd16297b1a931a80d232b5123a76231ea168af73`.
- Embedded agent SHA-256:
  `d47af9e62cac97dd61337daa7c0cb959cce1aa62ef9b5eec255b6189f88e0dfb`.

Native build/staging and bundle-only MSI packaging passed. One targeted native
refresh followed the final cache path-pinning correction. Rust formatting and
platform/helper Clippy passed with the existing infrastructure dead-code warning;
agent build has its existing four warnings. Both PowerShell scripts parse and
both embedded C# classes compile. Read-only MSI inspection confirms no NoModify/
NoRepair registration, the custom maintenance entry, `amus` repair, hidden Launch
on removal, early preparation/rollback, and the separate checked commit type 3585
at sequence 6599 (`REMOVE="ALL" AND NOT UPGRADINGPRODUCTCODE`). Normal commit cleanup
is now sequence 6598 and excludes removal. Embedded DLL bytes exactly match the
staged helper and contain all four staged payload hashes. Diff checks passed.
No tests or live installation/service/removal operations were run. The verification
walkthrough above remains pending with Cameron.

## P16.49 ? Coordinated Windows update and recovery

Implemented; awaiting Cameron's verification. This step changes the in-app updater
as well as MSI integration. It does not publish a release or close the phase gate.

The updater retains the signed current-release MSI, rechecks both manifests and
package bytes immediately before applying, validates their package family/version
and recovery protocol, and pins files/directories against replacement. Both MSIs
must declare `MSC_UPDATE_PROTOCOL=1`; legacy installations must first be upgraded
manually to a signed protocol-capable release. The installed desktop must live in
a machine-protected directory for elevated fixed service operations. The temporary
worker runs as the original user; it is not an elevated user-writable executable.

A retained worker waits for the original desktop to exit, runs MSI with passive
presentation, a verbose log and suppressed automatic reboot, and owns relaunch.
The MSI launch checkbox cannot launch a second desktop. Service coordination
retains protected prior state, copied payload and the old desktop digest. Local
ownership is checked; absent services and independent headless services remain
separate. Previously stopped agents remain stopped, and Minecraft is not restarted.

Healthy completion requires the new desktop to report shell initialization and,
for a previously running owned service, an authenticated loopback agent with the
expected version/API identity and a listener belonging to that service process.
During this health trial a 401 response cannot automatically discard saved host
credentials. Successful acceptance retains a protected previous-state record.
Installation or health failure attempts restoration of the verified previous MSI,
service state and desktop, then checks recovery health. Recovery failure is reported
explicitly. User cancellation, another busy installer, failure and reboot-required
results are distinct. Only the desktop child started by the worker is closed during
recovery. No remote agent is controlled and no worlds/settings are reset.

A restart-required result retains recovery material and displays a quoted PowerShell
command to resume the retained worker after restarting and closing MSC. Resume checks
that the target package is registered before health/acceptance. This is not a guarantee
of automatic recovery from power loss at every installer stage: an interrupted
transaction before registration requires inspection of the retained records/logs.
See Microsoft's [installer result codes](https://learn.microsoft.com/en-us/windows/win32/msi/error-codes)
and [REBOOT property](https://learn.microsoft.com/en-us/windows/win32/msi/reboot).

Ordinary newer-to-older installation is blocked unless restoration is explicitly
requested with `MSC_RESTORE_PREVIOUS=1`. The updater permits that flag only for its
verified previous package. This guard applies to protocol-capable MSI packages;
historical MSI packages cannot acquire the guard retroactively.

Diagnostics are under the user's `updates` directory: `windows-last-result.json`
and a unique `windows-worker-*` directory with `install.log`/`recovery.log`. These
contain operation status and package diagnostics; service passwords are never
requested or written by this path. Protected pending service records block another
managed transaction. Separately confirmed full removal can clean accepted previous
records and recognized partial build staging, but still refuses unknown files or
unfinished transactions.

This also corrects restoring automatic service startup: Windows reports `Auto`,
not `Automatic`, in [Win32_Service.StartMode](https://learn.microsoft.com/en-us/windows/win32/cimwin32prov/win32-service).
Use the P16.49 candidate for remaining P16.48 repair/removal acceptance.

### Exact candidate and checks

- Review copy: `C:\Users\example\Downloads\msc2-p16.49-windows-x86_64.msi`.
- Product version: **0.1.23**; unsigned local MSI; **19,431,424 bytes**.
- MSI SHA-256: `83fd1b0f9b32e56e6cb6453b072ca585e4d61c81ae1c7ae57b6441fe1e26be5e`.
- Embedded lifecycle DLL SHA-256: `bccd3fa6de8e9685bec91a0aee16ccea32ad36ce5719381b5b286e9373d6bd29`.
- Embedded agent SHA-256: `5d0bb8199b890771f4ce5640060ecbd0bc6be7d8ffd956a74ea35e5aba9374fd`.

Desktop/infrastructure/helper Clippy and Rust formatting passed. Svelte checking
reported zero errors and eleven existing warnings; frontend build passed. Native
release binaries/staging were produced and bundle-only MSI packaging passed. Both
PowerShell scripts parse and their embedded C# compiles. Read-only MSI inspection
confirmed protocol/security properties, downgrade guard, six-field managed-action
handoff and suppressed duplicate launch; embedded DLL bytes match the staged helper
and contain all four staged payload digests. Existing compiler/frontend warnings
remain. No tests, live installer/service operations, release workflow, tags or
publication were run.

### Owner verification in disposable Windows environments

The unsigned local review candidate permits installer inspection; it does not
substitute for signed-update acceptance. Full updater acceptance requires two
immutable signed protocol-capable releases matching the desktop's trusted key and
different version numbers. Record both manifests, MSI hashes and installed paths.
Do not alter signature verification to make a local candidate eligible.

1. Verify a successful update with a running owned agent: one desktop relaunch,
   expected desktop/agent version, unchanged service account/boot settings/data,
   healthy result and retained previous package. Repeat with an initially stopped
   agent, no local service, and a separate headless installation.
2. Cancel or refuse elevation. Confirm honest cancellation/error reporting and a
   usable prior installation. A busy Windows Installer must not trigger competing
   recovery. Corrupt signed staging in the disposable environment before applying;
   verification must refuse it before closing the original desktop.
3. Exercise a controlled replacement health failure, for example close the worker's
   new desktop before readiness is accepted. Confirm restoration, prior desktop
   digest/version, prior service state and recovery result. Record failures rather
   than marking them successful because MSI returned zero.
4. Observe a genuine restart-required installation in a disposable VM. Confirm no
   automatic reboot or healthy-success claim, retained packages and visible resume
   command. Restart, close MSC and run that exact command; verify subsequent health
   and acceptance. Do not fabricate a passing result if this case is unavailable.
5. Verify ordinary downgrade refusal and deliberate previous-package restoration.
   Recheck P16.48 repair/removal and separately confirmed full cleanup using this
   candidate, including unknown/pending-cache refusal. Preserve valued server data.

Physical acceptance remains pending. P16.50 has not been started.

## P16.50 — Consolidated owner acceptance and independent review handoff

**Status:** Packet prepared; physical acceptance and independent review pending.
**Candidate:** `C:\Users\example\Downloads\msc2-p16.49-windows-x86_64.msi`.
**Source:** `7f07f36f0c206a37c733f4d61788d16b93f9a41d` (P16.49).
**Version / size:** 0.1.23 / 19,431,424 bytes; unsigned local review MSI.
**SHA-256:** `83fd1b0f9b32e56e6cb6453b072ca585e4d61c81ae1c7ae57b6441fe1e26be5e`.
P16.50 changes documentation only; it reuses these exact bytes without rebuilding.

Cameron reported that the v0.1.15 opening page was briefly blank before advancing.
After P16.45 he said "it looks good." That is an owner visual impression of the
earlier review, with no exact digest or detailed cases recorded in the conversation.
It is not transferred as PASS to this candidate. All rows below remain PENDING
unless marked UNAVAILABLE with a specific missing prerequisite. Use PASS or FAIL
only for a recorded observation; neither a build nor static inspection proves it.

### Run the review

Use Windows Sandbox for the opening flow and fresh package review. Use disposable
VM snapshots for service-account, upgrade, reboot and failure cases. Copy the MSI
into the guest, verify its digest there, and keep this checklist with your notes.
Record guest Windows version, WebView2 state, display scaling, approving account,
and starting service state. Take a clean snapshot before each destructive case.
Keep valued worlds and the development installation outside these environments.

```powershell
$reviewMsi = (Resolve-Path -LiteralPath '.\msc2-p16.49-windows-x86_64.msi').Path
(Get-FileHash -LiteralPath $reviewMsi -Algorithm SHA256).Hash
$reviewLog = Join-Path $env:TEMP 'msc2-p16.50-install.log'
& "$env:SystemRoot\System32\msiexec.exe" /i $reviewMsi /norestart /L*V $reviewLog
```

The final command opens Setup and can change the guest installation. Back out and
cancel for the first case, then reopen for installation. Save logs/screenshots
before resetting Sandbox or reverting a VM. Review logs for personal information
before sharing; never pass service passwords through MSI properties.

| ID | Owner action and expected observation | Result | Observer / UTC / evidence |
|---|---|---|---|
| W01 | Open Setup: introduction appears without an unexplained blank preparation page; options, review, progress and completion are readable. | PENDING | Pending |
| W02 | Use Tab/Shift+Tab, Enter/Escape and Back/Next; cancel before install and refuse UAC. Choices/navigation are honest and cancellation leaves no successful-install claim. | PENDING | Pending |
| W03 | Repeat screen review at 100%, 125%, 150% and 200% scaling; no clipped text or inaccessible controls. | PENDING | Pending |
| W04 | Fresh install with shared WebView2 present: chosen folder/shortcut, complete desktop/agent/helpers/license, one Installed Apps entry and successful launch. | FAIL | Cameron; supplied Sandbox log at local clock 21:03:21, timezone not supplied; preparation error 1723/1157; WebView2 state unrecorded |
| W05 | Fresh VM missing shared WebView2, online: prerequisite is explained and installed; MSC opens. | PENDING | Pending |
| W06 | Restore missing-runtime snapshot, disconnect network: clear failure and no successful completion/launch claim. | PENDING | Pending |
| W07 | First launch: remote-only use creates no local service; deliberate local hosting setup uses the intended Windows account and one-time password prompt. | PENDING | Pending |
| W08 | New PowerShell: `Get-Command msc` and `msc --help` resolve the installed CLI; compare PATH before/after with an independent headless directory. | PENDING | Pending |
| W09 | Direct upgrade from an exact recorded prior MSI/custom folder: folder and family retained, one registration; no settings/world/credential reset. Stop legacy Minecraft/agent first when instructed. | PENDING | Pending |
| W10 | Reopen same MSI and repair missing package files: files/registration restored, account/boot policy/data unchanged. | PENDING | Pending |
| W11 | Owned service running: graceful Minecraft/helper stop, verified payload replacement, same account/boot settings, agent resumes and Minecraft stays stopped. | PENDING | Pending |
| W12 | Owned service stopped: repair/update leaves it stopped. Include automatic, delayed-automatic, manual and disabled startup cases where applicable. | PENDING | Pending |
| W13 | Independent marked headless service untouched; ambiguous ownership refuses changes. Remote connections receive no service-control operation. | PENDING | Pending |
| W14 | Different administrator approves UAC: original service owner retained, machine registration/shortcuts correct, original user can open MSC. | PENDING | Pending |
| W15 | Cancel/fail package work after preparation in a snapshotted VM: previous files/service metadata/state restored or explicit recovery failure with retained diagnostics. | PENDING | Pending |
| W16 | Installed Apps removal and reopened-MSI removal: only owned package/service detached, no Launch option, data/settings/credentials retained; reinstall can reuse data and request local registration. | PENDING | Pending |
| W17 | Separate confirmed full uninstall: inspect preview, acknowledge permanent loss and confirm phrase; reviewed data/credentials/cache/package removed, unrelated files protected, partial result honest. | PENDING | Pending |
| W18 | In-app signed newer update: one desktop relaunch, initialized shell plus owned-agent authenticated version health, success report and retained previous package. Repeat running/stopped/no-service/headless cases. | UNAVAILABLE | No eligible signed old/new protocol-capable pair recorded |
| W19 | Signed updater cancellation/UAC refusal, corrupted staging and busy installer: clear refusal/cancellation, usable prior installation, no competing recovery. | UNAVAILABLE | Same signed-pair prerequisite |
| W20 | Controlled new-desktop/agent health failure: verified prior MSI/payload restored, old desktop usable, prior agent state/account retained; failed restoration reported honestly. | UNAVAILABLE | Same signed-pair prerequisite |
| W21 | Genuine restart-required update: no automatic reboot or premature health success; retained command resumes health/acceptance after restart. | UNAVAILABLE | Signed pair and reproducible restart-required VM needed |
| W22 | Protocol-capable ordinary downgrade blocked; deliberate verified previous restoration succeeds. Historical packages cannot acquire the guard retroactively. | UNAVAILABLE | Distinct protocol-capable version pair needed |

For W09/W15/W18-W22 record both old/new MSI names, digests, source commits and,
for the in-app updater, signed manifest identities. The local MSI has no signed
release metadata; do not bypass signature checks or create a release merely to
fill these rows. W18-W22 use the detailed P16.49 procedure above. A controlled
fault needs an agreed reproducible VM procedure and stage/log evidence; failure
to reproduce is UNAVAILABLE, not PASS. Full uninstall in W17 deliberately deletes
reviewed guest worlds, unlike ordinary removal in W16.

For each result append the observer, UTC time, VM/snapshot identity, exact artifact
hash, observed behavior and evidence location. A FAIL includes expected versus
actual behavior and the retained log/report. UNAVAILABLE includes the prerequisite
or environment that prevented observation. Do not delete protected recovery records
to force another attempt. Power-loss recovery at every installer stage is not
established; retain records for inspection rather than claiming automatic recovery.

### Handoff to the other agent

The implementation was performed by Codex. Claude Code is the independent
reviewer; no review has been performed in this step. Review P16.44?P16.50 against
the acceptance gate in the rolling plan and the Phase 16 exit criteria in the port
plan, using current source, this exact candidate and Cameron's recorded outcomes.
Check ownership/elevation boundaries, rollback ordering, previous-package trust,
health/readiness, cancellation/reboot reporting, data retention, headless coexistence
and separately confirmed full cleanup. Report findings without implementing fixes
in REVIEW mode. Missing signed-pair/fault-path evidence keeps acceptance open.

The published-release packet remains separate in [phase16-acceptance.md](phase16-acceptance.md).
This local refinement neither replaces its release candidate nor satisfies the
nine-artifact/provenance or other platform gates. Publication/version/tag requires
a separate owner instruction. No implementation agent self-review or phase advance.

## P16.51 - Fix the clean-machine installer helper dependency

**Status:** Implemented; awaiting Cameron's Sandbox retry. This is the current
retry candidate; keep the P16.49 failure and do not transfer its observations as
PASS. Reuse W01-W22 above with this candidate's identity recorded in each result.

Cameron observed preparation lasting over a minute before Welcome, followed by
an empty Applying progress bar and error 1723. His logged repeat failed at
`MscPrepareService` with code 1157; `MscRollbackService` also returned 1157.
The developer-machine helper imported `VCRUNTIME140.dll` and external C runtime
APIs. Those imports explain a likely clean-machine dependency failure, though
the Sandbox log does not name the missing dependency. WebView2 state was not
independently recorded, and this evidence does not prove the preparation delay's
cause. Welcome eventually appeared; installation/launch acceptance failed.

The normal payload staging command now uses `cargo rustc` with
`-C target-feature=+crt-static` for this helper alone. This links its C runtime
into the DLL; see the [Rust linkage reference](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes).
No service/rollback logic, desktop or agent binary was changed. No extra runtime
installation or release workflow gate was added. Direct helper builds for MSI
must use the same flag; `cargo build` alone still selects the default linkage.

- Retry copy: `C:\Users\example\Downloads\msc2-p16.51-windows-x86_64.msi`.
- Product version: 0.1.23; unsigned local MSI; 19,521,536 bytes.
- MSI SHA-256: `57bd665eb122987a5c2150a5586506c59f37fbb2bb1f23fdc0ce7d5fcc2a303b`.
- Embedded helper SHA-256: `0dc2e438a214c174a6d0976b2c970f4d92202b259fdf8db8c662586f569ae2a3`.
- Source change: commit subject `P16.51: include runtime in windows installer helper`.

Helper release build, formatting/Clippy, Node syntax and staging-script formatting
passed; bundle-only MSI packaging passed. Read-only import inspection shows only
`kernel32.dll`, `msi.dll`, `ntdll.dll` and `api-ms-win-core-synch-l1-2-0.dll` (plus
case-duplicate KERNEL32). No external VC++/CRT imports remain. All four staged
payload hashes remain embedded; the MSI Binary stream matches the rebuilt DLL.
No tests, live installation/service operations or release publication were run.

Retry in a fresh Sandbox: copy this differently named MSI, launch with a verbose
log, install and check desktop launch without local hosting setup first. Report
the full log if it fails; copy it out before closing Sandbox. Keep repair/removal,
service, signed update and independent-review results pending. The prior >1-minute
preparation delay and uninformative progress remain open observations.

## P16.52 - Package the runtime across the Windows native payload

**Status:** Implemented; awaiting Cameron's verification. Current retry candidate
supersedes the local P16.51 candidate for W01-W22; older observations remain
attached to their original bytes, not transferred as PASS.

Cameron corrected the mistaken P16.49 retry, then ran P16.51. His screenshot
shows WebView2 installation with visible action text and a progressing bar. He
reported Setup finished, but opening MSC produced a system error explicitly
naming missing `VCRUNTIME140.dll`; the app window was blank. This establishes
installation completion as reported, not successful desktop launch or hosting.
The previously empty action area eventually displayed status; its initial delay
remains an observation to investigate, not a claim that updates never appear.

| Candidate | Owner observation | Result |
|---|---|---|
| P16.49, hash 83fd1b0f? | Fresh Sandbox install: helper preparation/rollback error 1723/1157 | FAIL |
| P16.51, hash 57bd665e? | WebView2 progress shown, Setup finished; desktop launch missing VCRUNTIME140.dll | FAIL for install-and-launch acceptance |
| P16.52, exact identity below | Fresh install, desktop and bundled-tool launch | PENDING |

The complete prior native import audit found VCRUNTIME imports in the desktop,
agent and Bedrock exporter. The P16.51 helper was already static. Pinned Vantage
imports Universal C Runtime APIs but no VCRUNTIME/MSVCP DLLs. The
[Universal C Runtime is part of Windows 10 and later](https://learn.microsoft.com/en-us/cpp/windows/universal-crt-deployment?view=msvc-170).

Repository `.cargo/config.toml` now applies `target-feature=+crt-static` to Windows
MSVC targets. This embeds the C runtime in all locally built Windows Rust payload,
including normal desktop, agent, exporter and lifecycle-helper release commands.
The existing Windows release staging script runs Cargo from the repository and
inherits this setting, as does Tauri's desktop build. No workflow, release gate,
extra runtime installer, signature bypass or product/service behavior was added.
The pinned third-party Vantage binary is unchanged. Runtime-linking overrides in
external environments can supersede Cargo configuration and must not be used for
these packages. See [Rust runtime linkage](https://doc.rust-lang.org/reference/linkage.html#static-and-dynamic-c-runtimes).

### Exact retry candidate

- Review copy: `C:\Users\example\Downloads\msc2-p16.52-windows-x86_64.msi`.
- Version: 0.1.23; unsigned local MSI; 19,742,720 bytes.
- MSI SHA-256: `59bd0445260b6206001d069f6c9845ccc1d03ede2d3a313f2af105dc58ac7859`.
- Embedded lifecycle DLL SHA-256: `b25d93b5ed294a449949d90f995a496d266fe5c58f8379be51daaea75e800743`.
- Code change: commit subject `P16.52: include runtime across windows native payload`.

| Packaged file | SHA-256 |
|---|---|
| msc2-desktop-web.exe | `513d8d92900d28edca6fc326c742067cdc956a2374381eef61cc7f09ccfca665` |
| bedrock-map.exe | `8ac304532335cb9b9d566b08975937a99836aaad662e927a479d296bc221ab95` |
| msc.exe | `04ea96e9bfe32ef7025de93fc4d111d544b05e26ff1000cd78df4461af48a9ce` |
| VANTAGE-LICENSE.txt | `388c961d7135c46acbbd5a99f7cd0907b43dec7e124c6441a9c6e0761b71bdbb` |
| vantage.exe | `7cf7d374a649e1d9a072c6d2f86a852196cb4dfe7afe038c5a7beed3e9aedbc0` |

Normal native staging rebuilt agent/exporter/helper and refreshed all four payload
hashes in the helper. The desktop release build and MSI bundle passed. Read-only
normal and delay-import inspection found no external VC++/CRT imports in our four
native binaries; Vantage retains only its OS UCRT imports. Cabinet extraction
confirmed all five installed payload files match staging. The desktop comparison
accounts for precisely Tauri's `__TAURI_BUNDLE_TYPE_VAR_UNK` to `MSI` marker patch.
The embedded MSI lifecycle Binary exactly matches the staged DLL and contains all
four correct payload hashes. Node staging syntax/config parsing, release desktop
Clippy and diff checks passed; existing Rust warnings remain. Workspace/exporter/
helper Rust formatting checks passed; the desktop formatting check reports an
existing unformatted protocol check in `src/update_windows.rs` from P16.49.
No Rust source was changed in this fix, and that unrelated formatting was retained.
No tests, app/installer/service execution, release workflow or publication ran.

### Retry in a fresh Sandbox

Close and reopen Sandbox, copy **msc2-p16.52-windows-x86_64.msi** and confirm its
hash before starting. Install, observe prerequisite/status screens, and launch
MSC without setting up local hosting first. Record startup/window behavior; the
runtime fix does not claim to fix the original large/off-centre window observation.
Then open a new PowerShell in the guest and run these bundled command entry points:

```powershell
& 'C:\Program Files\MSC 2\agent\msc.exe' --help
& 'C:\Program Files\MSC 2\agent\bedrock-map.exe' --help
& 'C:\Program Files\MSC 2\agent\vantage.exe' --help
```

Use the selected install folder if different. These manual checks establish loader/
command startup only, not map rendering or service lifecycle. Keep evidence before
closing Sandbox. Repair/removal, owned-service transitions, signed update/recovery
and independent gate review remain pending.

## P16.53 - Repair local-account identity lookup

**Status:** Implemented; awaiting Cameron's repair retry. Candidate identity is
recorded below after packaging. Reuse W01-W22 with the exact candidate identity;
this fix does not close independent review or signed-update acceptance.

Cameron's P16.52 screenshot confirms the desktop shell opened. He entered his
host Windows password during Sandbox local hosting setup; starting the guest
agent reported SCM 1069 (logon failure). Subsequent MSI repair displayed
`Exception calling Translate ... Some or all identity references could not be
translated.` The exact guest SCM account string/log was not supplied. Therefore
service authentication and repair are failed observations, not accepted hosting.
The desktop's oversized/off-centre geometry and content under the tabs remain
separate open observations.

The helper passed SCM `StartName` directly to `NTAccount.Translate`. SCM can
represent a local account as `.\username`; lookup of that form failed with the
same exception during read-only inspection of this development machine's current
account, whereas its fully qualified machine/user name resolved to a SID. This
identifies a concrete repair defect consistent with the screenshot, without
claiming the unrecorded guest account string was inspected. See Microsoft's
[local service account name form](https://learn.microsoft.com/en-us/windows/win32/ad/local-user-accounts).

`Account-Sid` now expands only the exact `.\` prefix to
`[Environment]::MachineName + '\'` before lookup. Domain-qualified, UPN and other
account names retain their existing lookup behavior. It still compares actual
resolved SIDs against metadata/recovery ownership; no fallback guesses identity
or bypasses failed lookup. A remaining unresolved identity produces a clear
maintenance refusal instead of the raw .NET Translate exception. The service
account, stored password and running/stopped policy are not changed by this fix.
Both the MSI DLL and desktop updater embed the script, so both are refreshed.
The agent, renderer/exporter and runtime-linking policy remain unchanged.

The guest account is separate from the host login. This repair fix is not a
credential fix and makes no promise that a host password authenticates the guest
service. Use a disposable ordinary-account VM for successful hosting acceptance.
In the existing Sandbox, retry repair first: the stopped registered service must
stay stopped, settings/data remain, and account translation must not fail. If an
ownership or credential failure remains, retain its log instead of deleting the
service/metadata to make the result pass. Then review ordinary removal/reinstall
as separate cases. The original preparation delay remains open.

### Exact P16.53 candidate and checks

- Review copy: `C:\Users\example\Downloads\msc2-p16.53-windows-x86_64.msi`.
- Version: 0.1.23; unsigned local MSI; 19,746,816 bytes.
- MSI SHA-256: `8515836fa62a3c5c12edaca3cc5b67814b16789d48a26d09bf2ac545f3afdad6`.
- Packaged desktop SHA-256: `26b549bc3f912ec4f2c868683dc4fd7c2f09b5f4adba3a7500b9c50d3f159399`.
- Embedded lifecycle DLL SHA-256: `d26271439022054b327c5209904f291c32461c574756a0833b3e0194798329b8`.
- Code change: `P16.53: resolve local service account aliases for maintenance`.

PowerShell parsing/embedded C# compilation, helper and desktop native release
builds, both release Clippy checks, validated bundle-only packaging, normal/delay
runtime import inspection and diff checks passed. The final desktop build refresh
includes the final diagnostic wording. Cabinet comparison confirms the desktop
matches its fresh binary with Tauri's expected MSI marker; the Binary stream
matches the refreshed DLL. Both contain the normalized-account code and the DLL
contains the unchanged four payload digests. Existing compiler warnings and the
previously documented P16.49 desktop formatting issue remain; no Rust source was
changed. No tests, installer/app/service execution or release publication ran.
Physical repair acceptance remains Cameron's.

## P16.54 - Normalize the service executable before comparison

**Status:** Implemented; awaiting Cameron's same-state update/repair verification.
Cameron's P16.53 screenshot shows update mode, followed by preparation refusal:
`The service executable differs from its recorded installation.` This is an update
failure; no successful repair or service replacement is recorded. It advances
beyond the preceding account-translation exception but does not prove full recovery.

Source inspection found that the desktop stages the agent beneath
`data_directory.join("agent/builds")`, so Windows paths can contain mixed forward
and backward separators. The service writer renders that path into SCM's command
and metadata. The installer normalized metadata with `Guard-Path` but compared
that result to a raw SCM prefix. Equivalent path spellings can therefore fail.
The exact guest SCM command has not been supplied; this is a concrete source defect
consistent with the reported refusal, not a claim that the guest command was read.

`Assert-Definition` now extracts either a quoted executable token or an unquoted
no-whitespace token, then uses the existing guarded normalized-path comparison on
both sides. Ambiguous commands still refuse maintenance. The tail must still be
the fixed `service-run --service-name com.ctemple.msc2.agent --bind 127.0.0.1:port`
with the metadata port. Account SID, profile/data roots, copied-build hashes,
protected ownership and transaction checks remain enforced. No prefix-only match,
service re-registration, credential change or ownership bypass is introduced.
Both the helper DLL and updater embed this script and are rebuilt.

Use the new candidate in the same Sandbox, close MSC and proceed through update
mode if offered. After the update actually succeeds, reopening its installed MSI
can exercise Repair separately. Do not call an update result a repair result.
The registered guest service should remain stopped; its prior failed credentials
are neither repaired nor replaced by package maintenance. Keep Sandbox and logs
if another refusal appears. Window geometry, preparation delay, normal-account
service logon, signed update recovery and independent review remain open.

### Exact P16.54 candidate and checks

- Review copy: `C:\Users\example\Downloads\msc2-p16.54-windows-x86_64.msi`.
- Version: 0.1.23; unsigned local MSI; 19,746,816 bytes.
- MSI SHA-256: `054a0da809b008cd51904c8b6cc1a827c815e2e6e49020a11b054265371cb2cf`.
- Packaged desktop SHA-256: `f189df7834bb2b693cdf04b979929a9f9dcbf4a3a846daced3e4ffa361cb485d`.
- Embedded lifecycle DLL SHA-256: `0587665d8194ffc40273f2a0a642b46b2c1f330c5445367e01ddca1266e8166c`.
- Code change: `P16.54: normalize service executable ownership comparisons`.

PowerShell parsing and embedded C# compilation, desktop/helper release builds and
Clippy, validated bundle-only packaging and diff checks passed with existing
warnings. No external VC++ dependency returned in normal/delay import inspection.
MSI cabinet desktop bytes match the fresh binary with the expected Tauri MSI marker;
its lifecycle Binary matches the refreshed helper. Both contain the new path
comparison, and all four compiled payload digests still match staging. The prior
P16.49 formatting limitation remains; no Rust source changed. No tests, live
installer/app/service operations or release publication ran. Same-state update,
subsequent Repair and independent acceptance remain pending with Cameron.
# P16.57/P16.58 local follow-up — uninstall continuation and cancellation

The abandoned Windows full-removal continuation contained its copied executable
and job but no retained result report. Read-only Windows process-path inspection
reproduced the worker's normal drive path versus the job's extended-prefix path:
the direct parent comparison was false and canonicalizing both made it true.
The worker now canonicalizes those paths on Windows and preserves the original
preview environment rather than injecting a data-directory override. Linux and
macOS continuation behavior is unchanged. This addresses an early refusal; actual
full removal still requires Cameron's retained result and residual checks.

Cancellation previously entered the stock UserExit dialog, showing generic
artwork and blank text. Both compiled UI sequences now route cancellation (-2)
to MscCancelledDlg, with resolved title/body/data copy and a Finish action; stock
UserExit is disabled. Read-only package extraction confirms the staged binaries
are included and the desktop retains Windows GUI subsystem 2.

Local unsigned candidate:
`C:\Users\example\Downloads\msc2-0.1.23-windows-x86_64-local.msi`
SHA256 `36b18c2c46f322aca89cf0f0caae206d57a2ec774fc273ea9c51c13271a094e7`.
Build, Rust formatting/agent Clippy and package inspection passed with existing
warnings. No live removal, test suites or publication were run. Cancel-before-
install visual acceptance and installed Settings full-removal acceptance remain
open; ordinary MSI removal still retains MSC data and credentials.
# P16.59 — Windows full-removal scheduling and adjacent cleanup audit

The P16.58 owner attempt launched its worker but left the desktop scheduling until
the worker's 60-second wait failed. The retained report stated that nothing was
removed; read-only residual inspection confirmed the installed MSI, service,
data and five credentials remained. The agent did not close or uninstall it.

Windows desktop/JSON continuation now uses CreateProcessW with handle inheritance
disabled. This prevents inherited scheduler pipes from holding the desktop's
response open while the worker waits for the desktop. The owner explicitly
authorized `cargo test -p msc-platform-windows --test uninstall_handoff`.
Its controlled temporary helpers use named events rather than sleeps; the normal
case passes, and a negative control reproduces the old inherited-pipe deadlock.
Neither scenario touches MSC installations or data. Interactive CLI and Unix
launch behavior are preserved.

The adjacent cleanup audit corrected undisposed Windows service polling handles,
added confirmation of SCM deletion before metadata/data removal, and made MSI
restart-required removal report incomplete rather than successful. The audit
also inspected credential scope, inventory revalidation, graceful shutdown,
protected targets, worker reports and existing cancellation routing. It is an
implementation audit, not an independent phase review or a physical gate PASS.

Rebuilt unsigned local candidate:
`C:\Users\example\Downloads\msc2-0.1.23-windows-x86_64-local.msi`
SHA256 `517d868341057542f275756cdc2db4951959fac3cc54d88d6386bd2c42783f13`.
Rust formatting/Clippy, PowerShell parsing, authorized regression, MSI build and
read-only embedded-file/UI table inspection passed with existing warnings.
Actual installed Settings full removal and residual verification remain open.
