# macOS synthetic sidecar lifecycle evidence

This is P10.18's reproducible, offline proof for the agent-to-sidecar
boundary. It uses the real Rust `MacosBedrockRuntime` adapter and a disposable
in-memory implementation of the frozen JSON-lines transport. It does not boot
Virtualization.framework, start a real BDS binary, or claim a live macOS
runtime cell.

## Boundary

- Host/backend: macOS sidecar lifecycle shape, exercised through the agent's
  loopback HTTP boundary
- Architecture: synthetic test architecture; no VM appliance is started
- BDS distribution/version: none; `1.26.32.2` is fixture data only
- Network and credentials: loopback only; no account, private world, or
  public download
- Apple Silicon: unavailable by D-028; this proof does not turn the Intel-only
  appliance into an Apple Silicon support claim

## Reproduction

Run from the repository root:

```text
bash tools/phase10/macos-smoke.sh --synthetic
```

The route proof covers readiness remaining pending until the synthetic
sidecar emits the DHCP console line and relay-up ready frame, console and
command framing, graceful stop, forced stop, sidecar EOF becoming an explicit
unavailable capability, and reuse of a host-owned world marker by a fresh
runtime instance. The application test also covers the process-transport
framer and the frozen adapter vocabulary.

## Limits

This evidence proves the shared Rust/Swift boundary behavior only. It does not
prove Intel VM boot, DHCP reachability, UDP forwarding against a real guest,
or support for a real BDS distribution. Those live or explicitly unavailable
outcomes belong in the later Phase 10 evidence steps.
