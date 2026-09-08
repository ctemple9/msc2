# Bedrock implementation readiness

**Status:** implementation complete; live Bedrock operation owner-confirmed · **Date:** 2026-09-08 · **Step:** P12.33

This record closes the Bedrock implementation boundary. It proves that the
production code, verified provisioning path, public API, and distributable
layouts agree. Cameron has also confirmed that MSC 2 creates and runs Bedrock
servers successfully. The separate compatibility matrix remains a
machine-checkable evidence record and is not rewritten with details that are
not transcribed into reproducible host/package records.

## What is implemented

| Host | Runtime selected by the agent | Package/runtime boundary | What this record proves |
|---|---|---|---|
| Linux (Debian 12), x86_64 | `native-linux-bds` | Native `server-directory/bedrock_server`; the BDS archive is a verified first-run download | Linux host selection, native UDP bind preflight, process lifecycle, and service layout are wired |
| Windows, x86_64 | `native-windows-bds` | Native `server-directory/bedrock_server.exe`; the BDS archive is a verified first-run download | Windows host selection, native UDP bind preflight, Job Object-backed process lifecycle, and service layout are wired |
| macOS (Intel), x86_64 | `macos-vz-swift-sidecar` | `BedrockSidecar` plus `vmlinuz-kata` and `appliance-initramfs.gz`; the Linux guest BDS archive is a verified first-run download | Intel sidecar/resource lookup, shared-directory mapping, guest-package selection, and sidecar lifecycle are wired |
| macOS (Apple Silicon), arm64 | unavailable (`no_test_hardware`) | No arm64 appliance or Rosetta-for-Linux path | D-028 is preserved; Apple Silicon is not silently mapped to the Intel sidecar |

The runtime selector detects the actual host and checks the selected server's
own files. Linux maps to the Linux BDS package, Windows maps to the Windows
package, and Intel macOS maps to the Linux guest package behind the Swift
sidecar. The sidecar directory is resolved from
`MSC2_BEDROCK_SIDECAR_DIR` or the packaged sibling resources
`BedrockSidecar`, `vmlinuz-kata`, and `appliance-initramfs.gz`.

## Verified provisioning boundary

The shared provisioner selects the platform before selecting a release,
requires a published SHA-256, checks the archive before extraction, rejects
unsafe paths and archives without the expected executable, stages outside the
live server directory, and writes `.msc_bds_provenance.json` beside the
promoted files. The final eligibility check reads that provenance rather than
treating a copied executable as trusted.

Updates preserve the existing `worlds/` tree and the user-owned
`server.properties`, `allowlist.json`, `permissions.json`, and `whitelist.json`.
The promotion swap is recoverable, and downgrade protection runs before the
old directory is moved aside. A provisioning or staging failure is returned
through the existing operation result; it is not reported as a ready server.

## Shared public path

Bedrock uses the existing API families, not a parallel Bedrock-only lifecycle:

- `POST /v1/start`, `POST /v1/stop`, and `POST /v1/command` use the selected
  native or sidecar adapter.
- `GET /v1/status` and `GET /v1/capabilities` expose the same additive runtime
  state, including `available`, `provisioning_required`, or `unavailable`.
- Settings, versions, players, allowlist, performance, worlds, backups,
  console, and operation streams retain their shared routes and carry the
  runtime state or the standard `capability_unavailable` error where a live
  runtime is required.
- Production lifecycle tests cover the fixture-backed start/command/stop path,
  create-time provisioning and provenance, and the unavailable-runtime path;
  the shared-surface test covers disk-readable Bedrock data and structured
  live-operation errors.

## Service and headless packaging

The package layout keeps the agent independent of the GUI:

- macOS GUI and headless packages carry the agent and the Intel
  `BedrockSidecar` resource set. The service manager is a `launchd LaunchDaemon`.
- Windows carries the native agent and installs it as a `Windows Service`; no
  sidecar artifact is included.
- Linux carries the native agent through the package-manager installation and
  uses `systemd`; no desktop or sidecar dependency is introduced.
- The Bedrock runtime archive is deliberately not bundled into the GUI or
  headless package. It remains a verified first-run download, so a release
  cannot accidentally ship a stale or unverified BDS archive.

These are package/layout and source-contract checks, not live installer runs.
They do not prove that a particular host's service manager, kernel, Windows
installation, or Virtualization.framework will execute successfully.

## Owner acceptance

On 2026-09-08, Cameron confirmed successful creation and operation of Bedrock
servers through the implemented MSC 2 path. This closes the product-development
and owner-acceptance boundary for the Bedrock implementation. It does not
change D-028: Apple Silicon remains unavailable because the Intel-only
appliance has no supported arm64 or Rosetta-for-Linux path.

The acceptance statement is recorded separately in
[`evidence/owner-acceptance.md`](evidence/owner-acceptance.md). It intentionally
does not reproduce private host details, credentials, or package hashes.

## Evidence boundary

The P12.33 checker validates the source seams, package paths, release schema,
public route contract, and the existing targeted production-test fixtures. It
also verifies that every current Bedrock compatibility-matrix row remains
`unavailable` and that its existing evidence file is present. Synthetic
adapter tests and fixture-backed API tests remain implementation evidence; they
are not runtime support claims.

The existing compatibility-matrix cells remain conservative because their
committed runtime records still lack the exact distribution identity and
reproducible host details required for a `supported` status. The owner
acceptance above closes the implementation/product boundary; it is not a
replacement for that machine-checkable evidence. Apple Silicon remains
unavailable under D-028.

## P12.33 handoff

The owner handoff described below has been completed for the Bedrock servers
Cameron ran. The detailed host/package values were not transcribed into this
repository, so the compatibility matrix remains conservative until that
reproducible evidence record exists:

1. Prepare a disposable Bedrock server directory and an exact verified BDS
   distribution for the host's selected backend.
2. On native Linux, run the agent headlessly under the supported Linux service
   layout and start the disposable server. On native Windows, do the same
   through the Windows Service layout. On an Intel Mac, run the GUI or
   headless LaunchDaemon with the packaged `BedrockSidecar`, Intel appliance
   resources, and Linux guest distribution.
3. Confirm the agent reports `available`, the server reaches readiness, and a
   real Bedrock client can join through UDP. For Intel macOS, confirm the guest
   DHCP/host relay path as part of that UDP reachability check.
4. Stop cleanly, start again, and exercise lifecycle recovery after the
   disposable process or sidecar is interrupted. Record the observed console,
   operation, status, and recovery results.
5. Add evidence only for the host/backend that was actually run, then promote
   only the matching matrix cells. Keep every untested cell `unavailable`;
   synthetic results cannot promote one.

The implementation readiness check remains satisfied when
`bedrock-package-check.py --readiness` passes. Live Bedrock operation is now
owner-accepted; matrix promotion remains a separate evidence-record action.
