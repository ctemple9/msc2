# Phase 16 exact-artifact release acceptance

**Status:** Incomplete. v0.1.18 was published from the post-D-038 source commit;
no Cameron-observed physical release results have been recorded. This packet
does not close the Phase 16 gate.

The latest published release at packet creation was v0.1.16, published on
2026-09-26 from source commit
`d9671612bfdc7ceba0679f6a6f7b6866edee0472`. It predates the 2026-09-28
browser-client retirement and P16.27, so it is historical and cannot serve as
the Phase 16 acceptance candidate. See the [v0.1.16 release](https://github.com/ctemple9/msc2/releases/tag/v0.1.16).

v0.1.18 was published from a commit containing P16.27 and P16.28.
The candidate tag, source commit, exact asset names and digests, and
all physical observations below must come from that published release. Do not
copy results from v0.1.16, a local build, or an earlier phase record.

## Candidate release identity

| Field | Value |
|---|---|
| Candidate tag | v0.1.18 |
| Published release URL | https://github.com/ctemple9/msc2/releases/tag/v0.1.18 |
| Source commit (full SHA) | ededaf33632bbbdcc518ae8928a54bb3ba073cc6 |
| Published at (UTC) | 2026-09-29T02:10:43Z |
| Published SHA256SUMS verification | PENDING |
| Signed update manifest verification | PENDING |

## Exact published artifacts

Record all nine installer/archive files from the same candidate release. The
checker derives each expected filename from the candidate tag and rejects a
missing asset, a digest that does not match `SHA256SUMS`, or an unobserved
install/launch result.

| ID | Installation | Platform | Expected filename | Published filename | Bytes | SHA-256 | SHA256SUMS | Install and launch | Observed by | Observed (UTC) | Evidence |
|---|---|---|---|---|---:|---|---|---|---|---|---|
| desktop-macos-x86_64 | Desktop | macOS Intel | msc2-0.1.18-macos-x86_64.dmg | msc2-0.1.18-macos-x86_64.dmg | 26596342 | 8d3fed6f82dc3519fe55b5af02df7e64451eece1c305a19342139b8389d0daab | PENDING | UNAVAILABLE | — | — | Published asset metadata; Cameron install pending |
| desktop-macos-aarch64 | Desktop | macOS Apple Silicon | msc2-0.1.18-macos-aarch64.dmg | msc2-0.1.18-macos-aarch64.dmg | 14224516 | 0f55cdaa6b39c97997f3dd4ac9b4638256baa37f71c035a45e8cd24e1e33f7f9 | PENDING | UNAVAILABLE | — | — | Published asset metadata; Cameron install pending |
| desktop-windows-x86_64 | Desktop | Windows x86_64 | msc2-0.1.18-windows-x86_64.msi | msc2-0.1.18-windows-x86_64.msi | 14249984 | fbdf3f0487dd7c960446c3505e8e53af1d66113dc27c0debed12a0b442318cbe | PENDING | UNAVAILABLE | — | — | Published asset metadata; Cameron install pending |
| desktop-linux-deb-x86_64 | Desktop .deb | Linux x86_64 | msc2-0.1.18-linux-x86_64.deb | msc2-0.1.18-linux-x86_64.deb | 16599228 | 6d4f4de692b5a707df2a2d7d611d4eb10db30fa507e565669724e023f4ce4316 | PENDING | UNAVAILABLE | — | — | Published asset metadata; Cameron install pending |
| desktop-linux-rpm-x86_64 | Desktop .rpm | Linux x86_64 | msc2-0.1.18-linux-x86_64.rpm | msc2-0.1.18-linux-x86_64.rpm | 16600653 | 1bf1e47aef80b35c833c1530f5a35261a43cfa0b8c724e0bd1a98aa614a6ad9e | PENDING | UNAVAILABLE | — | — | Published asset metadata; Cameron install pending |
| headless-macos-x86_64 | Headless | macOS Intel | msc2-headless-0.1.18-macos-x86_64.tar.gz | msc2-headless-0.1.18-macos-x86_64.tar.gz | 20423698 | 51307ebe736cffb1489378d59528e956798036793688117d7b82c2ad0f4ece43 | PENDING | UNAVAILABLE | — | — | Published asset metadata; Cameron install pending |
| headless-macos-aarch64 | Headless | macOS Apple Silicon | msc2-headless-0.1.18-macos-aarch64.tar.gz | msc2-headless-0.1.18-macos-aarch64.tar.gz | 8278636 | d10c8542ba8102bf114aa6dee9740950eda1e0b88b5ceb27bda9478d48c3663b | PENDING | UNAVAILABLE | — | — | Published asset metadata; Cameron install pending |
| headless-windows-x86_64 | Headless | Windows x86_64 | msc2-headless-0.1.18-windows-x86_64.zip | msc2-headless-0.1.18-windows-x86_64.zip | 8495055 | 142d7e51d4c2306fa3cae263258394a1bc486a41f6bcc00a13a1589f7736303c | PENDING | UNAVAILABLE | — | — | Published asset metadata; Cameron install pending |
| headless-linux-x86_64 | Headless | Linux x86_64 | msc2-headless-0.1.18-linux-x86_64.tar.gz | msc2-headless-0.1.18-linux-x86_64.tar.gz | 9069514 | d15395e90966e90f2beadb7048af89c0ed4d6f6dd412c762faea983a0376f9bc | PENDING | UNAVAILABLE | — | — | Published asset metadata; Cameron install pending |

## Required release checks

Each row needs an exact candidate asset name or the full candidate source
commit, a `PASS` result observed by Cameron, a UTC observation date, and a
link or repository path to the evidence. Use `FAIL` or `UNAVAILABLE` honestly;
either leaves the phase gate open. For platform checks, include every exact
artifact to which the observation applies.

| ID | Required evidence | Exact artifact(s) or source commit | Result | Observed by | Observed (UTC) | Evidence |
|---|---|---|---|---|---|---|
| archive-confinement | World archive extraction cannot write outside world data | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| operation-exclusivity | Conflicting operations are refused while target admission is reserved | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| host-reset-exclusivity | Host reset reserves the host against concurrent operations | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| world-replacement-recovery | Interrupted world replacement reaches a provable complete state | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| backup-save-acknowledgement | Online backup accepts only the current save acknowledgement | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| cancellation-and-revocation | Cancellation permissions and WebSocket termination follow credential permissions/revocation | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| bounded-operation-history | Operation history and admission cost remain bounded | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| host-connection-generation | Delayed host connections cannot publish stale state | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| windows-service-lifecycle | Windows service starts through the production Service Control Manager path | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| windows-update-rollback | Failed Windows update restores the previous healthy installation | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| macos-update-rollback | Failed macOS update restores the previous healthy installation | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| browser-retirement | Exact release agents serve no browser page, accept no browser sessions, and contain no browser-only assets/routes | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| supported-client-pairing | Tauri desktop and CLI pair with local and remote/headless hosts on supported platforms | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| minecraft-lifecycle | Create/import, start, manage, stop, and reconnect to a Minecraft server | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| world-import-backup-restore | World import, backup, restore, and resulting data are correct | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| interruption-recovery | Interrupt world/archive/restore work and confirm safe recovery | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| service-reboot-signout | Agent service survives client close, user sign-out, and host reboot as applicable | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| permission-revocation | Scoped permissions deny disallowed actions and revocation ends access | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| uninstall-data-retention | Uninstall removes owned application/service files and preserves managed data | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| artifact-identity | Installer/archive identities, embedded agent versions, and published digests match | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| release-provenance | Published artifacts have verified checksums and a signed manifest tied to the candidate tag | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| linux-minimum | Exact Linux desktop and headless artifacts install and launch on the declared minimum | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| generated-api-and-frontend | Generated API types and desktop frontend static checks pass at the candidate commit | PENDING | UNAVAILABLE | — | — | Cameron observation pending |
| public-documents | License, security contact, and support claims match approved current scope | PENDING | UNAVAILABLE | — | — | Cameron observation pending |

## Gate result

**Not accepted.** Candidate publication succeeded; all Cameron-observed
physical results remain pending. Do not mark Phase 16 complete until the
evidence checker passes and the independent Phase 16 review is recorded.

## P16.50 local Windows MSI refinement handoff (2026-10-07)

This supplemental candidate is a local installer review, separate from the
published v0.1.18 packet above. It does not replace published artifact identities
or transfer observations across versions. The complete owner checklist W01-W22
and independent-review handoff are in
[windows-msi-review.md](windows-msi-review.md#p1650--consolidated-owner-acceptance-and-independent-review-handoff).

| Field | Local refinement candidate |
|---|---|
| Filename | msc2-p16.49-windows-x86_64.msi |
| Source commit | 7f07f36f0c206a37c733f4d61788d16b93f9a41d |
| Version / bytes | 0.1.23 / 19,431,424 |
| SHA-256 | 83fd1b0f9b32e56e6cb6453b072ca585e4d61c81ae1c7ae57b6441fe1e26be5e |
| Provenance | Local build; unsigned MSI; no signed release manifest |
| Package/UI/service/removal observations | PENDING (W01-W17) |
| Signed update/recovery/downgrade observations | UNAVAILABLE pending required version pairs/environments (W18-W22) |
| Independent review | PENDING; Claude Code, report-only REVIEW |

Cameron's earlier "it looks good" followed P16.45 and is retained as a visual
impression only, not exact-candidate acceptance. P16.50 creates no PASS results.
Windows service and rollback rows in the published-release table remain unchanged.
The local MSI checksum was rechecked during packet preparation; that is artifact
identity evidence, not physical acceptance. The Phase 16 gate remains open.

### P16.51 Sandbox failure and retry

Cameron's P16.49 Sandbox fresh installation failed: error 1723, helper preparation
and rollback both returned 1157. The submitted log is recorded in the Windows
review packet; observation UTC and runtime state were not supplied. This is a
local-candidate failure, not an observation of the historical published release.
P16.51 supplies a statically linked helper retry candidate, MSI SHA-256
`57bd665eb122987a5c2150a5586506c59f37fbb2bb1f23fdc0ce7d5fcc2a303b`. Its identity and checks are in
[windows-msi-review.md](windows-msi-review.md#p1651---fix-the-clean-machine-installer-helper-dependency).
Retry installation, service/recovery and independent review remain pending.

### P16.52 full-payload runtime correction

Cameron reports P16.51 Setup completed, but desktop launch failed with a screenshot
explicitly naming missing VCRUNTIME140.dll. This does not close install-and-launch
acceptance. P16.52 supplies a distinct candidate with static runtime linkage across
the Windows Rust payload; no signed release was published. Exact retry MSI digest:
`59bd0445260b6206001d069f6c9845ccc1d03ede2d3a313f2af105dc58ac7859`.
See the [P16.52 record](windows-msi-review.md#p1652---package-the-runtime-across-the-windows-native-payload)
for packaged-file digests and import/cabinet inspection. New candidate physical
acceptance, original window geometry/preparation delay and independent review
remain pending. Earlier published-release rows are unchanged.

### P16.53 observed launch and repair failure

P16.52 desktop launch is owner-observed in the supplied screenshot. After attempted
Sandbox local service setup, SCM start failed with 1069; MSI repair then failed
account translation. These are distinct observations. No successful hosting or
repair PASS is recorded. P16.53 normalizes local-account shorthand without
changing credentials or bypassing identity checks; candidate details and retry
instructions are in the [Windows review packet](windows-msi-review.md#p1653---repair-local-account-identity-lookup).
Physical retry and independent review remain pending.
