# Phase 16 exact-artifact release acceptance

**Status:** Incomplete. No post-D-038 release candidate has been published and
no Cameron-observed release results have been recorded. This packet does not
close the Phase 16 gate.

The latest published release at packet creation was v0.1.16, published on
2026-09-26 from source commit
`d9671612bfdc7ceba0679f6a6f7b6866edee0472`. It predates the 2026-09-28
browser-client retirement and P16.27, so it is historical and cannot serve as
the Phase 16 acceptance candidate. See the [v0.1.16 release](https://github.com/ctemple9/msc2/releases/tag/v0.1.16).

The next release must be published from a commit containing P16.27 and P16.28.
The candidate tag, source commit, exact asset names and digests, CI run, and
all physical observations below must come from that published release. Do not
copy results from v0.1.16, a local build, or an earlier phase record.

## Candidate release identity

| Field | Value |
|---|---|
| Candidate tag | v0.1.17 |
| Published release URL | PENDING |
| Source commit (full SHA) | PENDING |
| Published at (UTC) | PENDING |
| Successful full CI run for this tag commit (run ID and URL) | PENDING |
| Published SHA256SUMS verification | PENDING |
| Signed update manifest and provenance verification | PENDING |

## Exact published artifacts

Record all nine installer/archive files from the same candidate release. The
checker derives each expected filename from the candidate tag and rejects a
missing asset, a digest that does not match `SHA256SUMS`, or an unobserved
install/launch result.

| ID | Installation | Platform | Expected filename | Published filename | Bytes | SHA-256 | SHA256SUMS | Install and launch | Observed by | Observed (UTC) | Evidence |
|---|---|---|---|---|---:|---|---|---|---|---|---|
| desktop-macos-x86_64 | Desktop | macOS Intel | Derived from tag | PENDING | PENDING | PENDING | PENDING | UNAVAILABLE | — | — | No candidate published |
| desktop-macos-aarch64 | Desktop | macOS Apple Silicon | Derived from tag | PENDING | PENDING | PENDING | PENDING | UNAVAILABLE | — | — | No candidate published |
| desktop-windows-x86_64 | Desktop | Windows x86_64 | Derived from tag | PENDING | PENDING | PENDING | PENDING | UNAVAILABLE | — | — | No candidate published |
| desktop-linux-deb-x86_64 | Desktop .deb | Linux x86_64 | Derived from tag | PENDING | PENDING | PENDING | PENDING | UNAVAILABLE | — | — | No candidate published |
| desktop-linux-rpm-x86_64 | Desktop .rpm | Linux x86_64 | Derived from tag | PENDING | PENDING | PENDING | PENDING | UNAVAILABLE | — | — | No candidate published |
| headless-macos-x86_64 | Headless | macOS Intel | Derived from tag | PENDING | PENDING | PENDING | PENDING | UNAVAILABLE | — | — | No candidate published |
| headless-macos-aarch64 | Headless | macOS Apple Silicon | Derived from tag | PENDING | PENDING | PENDING | PENDING | UNAVAILABLE | — | — | No candidate published |
| headless-windows-x86_64 | Headless | Windows x86_64 | Derived from tag | PENDING | PENDING | PENDING | PENDING | UNAVAILABLE | — | — | No candidate published |
| headless-linux-x86_64 | Headless | Linux x86_64 | Derived from tag | PENDING | PENDING | PENDING | PENDING | UNAVAILABLE | — | — | No candidate published |

## Required release checks

Each row needs an exact candidate asset name or the full candidate source
commit, a `PASS` result observed by Cameron, a UTC observation date, and a
link or repository path to the evidence. Use `FAIL` or `UNAVAILABLE` honestly;
either leaves the phase gate open. For platform checks, include every exact
artifact to which the observation applies.

| ID | Required evidence | Exact artifact(s) or source commit | Result | Observed by | Observed (UTC) | Evidence |
|---|---|---|---|---|---|---|
| archive-confinement | World archive extraction cannot write outside world data | PENDING | UNAVAILABLE | — | — | No candidate published |
| operation-exclusivity | Conflicting operations are refused while target admission is reserved | PENDING | UNAVAILABLE | — | — | No candidate published |
| host-reset-exclusivity | Host reset reserves the host against concurrent operations | PENDING | UNAVAILABLE | — | — | No candidate published |
| world-replacement-recovery | Interrupted world replacement reaches a provable complete state | PENDING | UNAVAILABLE | — | — | No candidate published |
| backup-save-acknowledgement | Online backup accepts only the current save acknowledgement | PENDING | UNAVAILABLE | — | — | No candidate published |
| cancellation-and-revocation | Cancellation permissions and WebSocket termination follow credential permissions/revocation | PENDING | UNAVAILABLE | — | — | No candidate published |
| bounded-operation-history | Operation history and admission cost remain bounded | PENDING | UNAVAILABLE | — | — | No candidate published |
| host-connection-generation | Delayed host connections cannot publish stale state | PENDING | UNAVAILABLE | — | — | No candidate published |
| windows-service-lifecycle | Windows service starts through the production Service Control Manager path | PENDING | UNAVAILABLE | — | — | No candidate published |
| windows-update-rollback | Failed Windows update restores the previous healthy installation | PENDING | UNAVAILABLE | — | — | No candidate published |
| macos-update-rollback | Failed macOS update restores the previous healthy installation | PENDING | UNAVAILABLE | — | — | No candidate published |
| browser-retirement | Exact release agents serve no browser page, accept no browser sessions, and contain no browser-only assets/routes | PENDING | UNAVAILABLE | — | — | No candidate published |
| supported-client-pairing | Tauri desktop and CLI pair with local and remote/headless hosts on supported platforms | PENDING | UNAVAILABLE | — | — | No candidate published |
| minecraft-lifecycle | Create/import, start, manage, stop, and reconnect to a Minecraft server | PENDING | UNAVAILABLE | — | — | No candidate published |
| world-import-backup-restore | World import, backup, restore, and resulting data are correct | PENDING | UNAVAILABLE | — | — | No candidate published |
| interruption-recovery | Interrupt world/archive/restore work and confirm safe recovery | PENDING | UNAVAILABLE | — | — | No candidate published |
| service-reboot-signout | Agent service survives client close, user sign-out, and host reboot as applicable | PENDING | UNAVAILABLE | — | — | No candidate published |
| permission-revocation | Scoped permissions deny disallowed actions and revocation ends access | PENDING | UNAVAILABLE | — | — | No candidate published |
| uninstall-data-retention | Uninstall removes owned application/service files and preserves managed data | PENDING | UNAVAILABLE | — | — | No candidate published |
| artifact-identity | Installer/archive identities, embedded agent versions, and published digests match | PENDING | UNAVAILABLE | — | — | No candidate published |
| same-commit-ci | Exact-tag publication requires successful full CI on the same source commit | PENDING | UNAVAILABLE | — | — | No candidate published |
| release-provenance | Published artifacts have verified checksum, signed manifest, and source/toolchain provenance | PENDING | UNAVAILABLE | — | — | No candidate published |
| linux-minimum | Exact Linux desktop and headless artifacts install and launch on the declared minimum | PENDING | UNAVAILABLE | — | — | No candidate published |
| generated-api-and-frontend | Generated API types and desktop frontend static checks pass at the candidate commit | PENDING | UNAVAILABLE | — | — | No candidate published |
| public-documents | License, security contact, and support claims match approved current scope | PENDING | UNAVAILABLE | — | — | No candidate published |

## Gate result

**Not accepted.** Candidate publication and all Cameron-observed results are
pending. Do not mark Phase 16 complete until the evidence checker passes and
the independent Phase 16 review is recorded.
