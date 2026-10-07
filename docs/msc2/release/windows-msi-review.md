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
| Review copy | `C:\Users\Cameron\Downloads\msc2-p16.47-windows-x86_64.msi` |
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
