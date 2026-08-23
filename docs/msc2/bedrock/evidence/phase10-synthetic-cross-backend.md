# Phase 10 production cross-backend smoke

This is P10.35's offline public-path evidence. The smoke starts the real
`msc serve` composition root and exercises its selected runtime and operation
journal. The same test is run by the macOS, Linux, and Windows CI jobs, where
the platform fixture adapter is respectively the Intel-macOS VZ Swift sidecar,
Linux native BDS shape, or Windows native BDS shape. The fixture files are
disposable and never start a real BDS package, VM, or public provider.

## Reproduction

Run from the repository root:

```text
bash tools/phase10/phase10-smoke.sh --synthetic
```

The workflow covers production-router create/import and provision paths,
start/status/stop, readiness and console output, command delivery, players,
settings reads and writes, allowlist reads and writes, operation cancellation,
restart recovery, capability disclosure, and explicit runtime-unavailable
responses. It also checks that the production router reports the expected
backend identity for the host running the test.

## Limits

This is production integration evidence only. It does not prove a native BDS
package, Windows process-tree behavior, Intel Virtualization.framework boot,
UDP reachability, or Apple Silicon support. Those claims require P10.24/P10.25
runtime evidence; Apple Silicon remains unavailable under D-028. A local run
therefore proves only the adapter selected by the local host; tri-platform CI
is what supplies the three adapter identities.
