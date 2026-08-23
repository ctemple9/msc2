# Windows synthetic public-contract evidence

This is P10.15's reproducible, offline proof for the Windows-shaped Bedrock
public contract. It uses a fake native Windows runtime and the real `msc` CLI;
it does not claim that a Windows Bedrock Dedicated Server package was
downloaded, verified, or started.

## Boundary

- Host/backend: Windows public-contract shape with a synthetic native backend
- BDS distribution/version: none; `1.26.32.2` is fixture data only
- Network and credentials: loopback only; no account, private world, or public download
- Ownership: the fake runtime remains service-owned after the client connection closes
- Failure handling: a synthetic start failure leaves no live or orphaned process

## Reproduction

Run from the repository root on a machine with PowerShell 7, Rust, and
`cargo-nextest`:

```text
pwsh -File tools/phase10/windows-smoke.ps1 -Synthetic
```

The route proof covers provision, start, status, command, stop, service-owned
survival after client exit, explicit runtime unavailability, and cleanup after
a failed start. The CLI proof covers the same lifecycle verbs against a
disposable loopback server and reads the unavailable capability state.

## Limits

The synthetic tests prove the shared public contract only. They do not prove
native Windows process-tree behavior, UDP reachability, or support for a real
BDS distribution. Those live or explicitly unavailable outcomes belong in the
later Phase 10 evidence steps.
