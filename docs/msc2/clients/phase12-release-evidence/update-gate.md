# Phase 12 coordinated-update acceptance evidence

**Step:** P12.101 — Record the cross-platform update gate
**Status:** static contract recorded; physical platform runs remain pending
**Scope:** signed MSC application releases, desktop updates, Linux package
guidance, standalone headless updates, and the local-only privilege boundary

This packet records what the repository can establish statically and where
Cameron's physical macOS, Windows, and Linux runs must provide the final
evidence. A source check, local build, or green GitHub workflow does not prove
that an installer launched, a service survived sign-out, or a real machine
rolled back successfully.

## Static gate

Run the following from the repository root. These checks inspect the release
workflow, signed manifest schema, update implementations, and this evidence
packet. They do not download or install a release, and they do not replace the
physical handoff.

```text
python3 tools/release/check-release-workflow.py \
  .github/workflows/release.yml --expect-publish-guard
python3 tools/release/check-update-gate.py
git diff --check
```

The release workflow's static gate runs before the platform build matrix. It
requires the complete x86_64 artifact set, guarded publication, release notes,
the signed manifest path when a signing key is configured, and the explicit
unsigned-prerelease path when it is not.

## Acceptance coverage

The status in the last column is deliberately an evidence classification, not
a claim that the behavior has passed on physical hardware.

| ID | Acceptance point | Static evidence | Final evidence | Status |
|---|---|---|---|---|
| U01 | Signed publication | `check-release-workflow.py --expect-publish-guard`, `release.yml`, `sign-update-manifest.py` | Successful exact tag workflow and release URL | Static recorded; physical pending |
| U02 | Version and release notes | `release_update.rs`, `AppSettingsSheet.svelte`, `cli/update.rs` | Check an older and newer published release and inspect displayed notes | Static recorded; physical pending |
| U03 | Decline and cancel | `AppSettingsSheet.svelte`, `cli/update.rs` | Decline the desktop confirmation and cancel the CLI prompt without changing the install | Static recorded; physical pending |
| U04 | Signature and digest refusal | `release_update.rs`, `update-release-schema.json` | Use a deliberately altered staged manifest, signature, or asset and record refusal | Static recorded; physical pending |
| U05 | Interrupted and already-staged work | Temporary staging cleanup and existing-release refusal in `release_update.rs` | Interrupt retrieval and repeat staging; confirm the old staged release is not overwritten | Static recorded; physical pending |
| U06 | macOS and Windows coordinated replacement | Tauri `tauri-coordinated` dispatch plus platform manifest entries | Install on physical x86_64 macOS and Windows and confirm desktop, agent, and required components remain paired | Static recorded; physical pending |
| U07 | Linux desktop package authorization | `authorized-package-install` dispatch and `.deb`/`.rpm` manifest entries | Authorize a local package install and record package ownership | Static recorded; physical pending |
| U08 | Standalone headless update | `msc update check/install` and `standalone-archive` manifest entries | Update a physical headless installation and record service restart/reconnect | Static recorded; physical pending |
| U09 | Distribution package-manager guidance | `package-manager-guidance` and `apt`/`dnf` paths in `cli/update.rs` | Confirm a distribution-managed install is guided, not overwritten | Static recorded; physical pending |
| U10 | Preserved user data | Schema and release contract preserve configuration, secrets, worlds, and server files | Compare those paths before and after a real update | Static recorded; physical pending |
| U11 | Health recovery and rollback | `/v1/healthz`, previous-payload backup, and rollback path in `cli/update.rs` | Exercise a failed recovery with a disposable installation and retain the result | Static recorded; physical pending |
| U12 | CLI text and JSON | `--json`, release notes, declined, staged, installed, and package-guidance output paths | Run `msc update check` and `msc update install` in both output modes | Static recorded; physical pending |
| U13 | Unsigned prerelease refusal | `UNSIGNED-BETA-NOTICE.txt`, missing-key errors, and publication fallback | Confirm an unsigned prerelease cannot enter native installation | Static recorded; physical pending |
| U14 | Remote service boundary | Desktop remote-host message and CLI `update commands are local-only` rejection | Attempt update/service control through a remote client and record refusal | Static recorded; physical pending |
| U15 | Physical platform handoff | Existing `linux.md`, `windows.md`, and `signing.md` worksheets | Cameron completes the physical macOS, Windows, and Linux runs | Pending — this row keeps the gate open |

## Gate decision

The repository-side update contract is recorded and statically checked. The
release/update gate is **not closed** by this step. It remains open until the
published bytes and the physical platform evidence are recorded by Cameron,
including installer behavior, service ownership and recovery, preserved user
data, and the expected unsigned warnings.

Do not paste pairing codes, bearer tokens, private keys, passwords, or private
host addresses into this packet.
