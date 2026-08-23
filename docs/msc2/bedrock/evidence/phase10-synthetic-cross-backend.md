# Phase 10 synthetic cross-backend smoke

This is P10.23's offline public-path evidence. The route and CLI integration
tests share one loopback API harness and run the same workflow for all three
backend identities: Linux native, Windows native, and the Intel-macOS VZ Swift
sidecar. The harness is fixture-backed and never starts BDS, a VM, or a public
provider.

## Reproduction

Run from the repository root:

```text
bash tools/phase10/phase10-smoke.sh --synthetic
```

The workflow covers verified-fixture provision, start/status/stop, readiness
and console output, command delivery, players, settings reads and writes,
allowlist reads and writes, operation cancellation, restart recovery, and
explicit runtime-unavailable capability/error responses. The CLI test drives
the same API paths through the real `msc` binary; the route test checks the
wire responses and operation transitions directly.

## Limits

This is shared contract evidence only. It does not prove a native BDS package,
Windows process-tree behavior, Intel Virtualization.framework boot, UDP
reachability, or Apple Silicon support. Those claims require P10.24/P10.25
runtime evidence; Apple Silicon remains unavailable under D-028.
