# Phase 14 — cross-platform acceptance evidence

**Step:** P14.18 · **Status:** awaiting owner verification · **Date:** 2026-09-11

This note records the final static pass that can be performed in the shared
workspace. It does not turn a source check into evidence of a live Minecraft
runtime, native OS installation, or remote-host walkthrough. Those rows remain
for Cameron's manual verification before P14.19.

## Static checks completed

| Boundary | Result | Evidence |
|---|---|---|
| Rust workspace | Pass | `cargo fmt --all -- --check` and `cargo check --workspace` |
| Tauri Rust shell | Pass | `cargo fmt --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- --check` and `cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml` |
| Shared frontend | Pass | `npm run check` and `npm run build` from `clients/desktop-web`; Svelte reported 0 errors and 0 warnings |
| Unix packaging | Pass | `bash -n` for Linux/macOS install, uninstall, and headless-build scripts |
| Windows packaging | Pass | PowerShell parser accepted the Windows install, uninstall, and headless-build scripts |
| Release workflow | Pass | `python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard` |
| Update gate | Pass | `python3 tools/release/check-update-gate.py` |
| Repository diff | Pass | `git diff --check` |

The P14.18 command's final `npm run check` must be run from
`clients/desktop-web`; the repository has no root `package.json`, so invoking
that portion from the repository root fails before npm can run Svelte checks.

## Static coverage recorded

- The release matrix contains macOS Intel, macOS Apple Silicon, native Windows
  x86_64, and Linux x86_64 builds, with desktop installers and standalone
  headless archives. The archive builders include `HEADLESS-INSTALL.md` and
  the platform-owned install/uninstall scripts.
- The headless contract keeps `msc`/`msc.exe` installation separate from the
  local service. Linux and macOS own `/usr/local/bin/msc`; Windows owns its
  selected PATH directory. The service endpoint remains loopback
  `127.0.0.1:48001`.
- The saved-host model carries editable LAN and Tailscale addresses, stable
  host identity, route preference, SSH metadata, remote management port, and a
  configurable local forwarded port. The wizard defaults to remote `48001`
  and local `48002`, detects saved-host collisions, and teaches the generated
  SSH command.
- The desktop connection manager probes preferred direct routes before the
  managed SSH tunnel, reuses a host-scoped session, and routes the tunnel to
  remote loopback `48001`. The native bridge exposes status, retry, stop, host
  key review, and the fixed remote pairing bootstrap; bearer credentials remain
  in the native secure store.
- Documentation keeps direct LAN/DNS, optional Tailscale, user-operated
  VPN/overlay, managed SSH, and manual recovery distinct. No path requires
  Tailscale, a manually opened terminal, or a cloud relay.

## Still required before the phase gate

Static checks cannot establish the acceptance classes marked **M**, **O**, and
**C** in `phase14-operational-refinements.md`. Cameron must still walk through:

1. Linux Xubuntu headless install, PATH lookup, upgrade, owned uninstall, and
   service reachability on `127.0.0.1:48001`.
2. macOS local and remote cases, including Intel Bedrock-sidecar packaging and
   Apple Silicon's documented sidecar boundary.
3. Native Windows headless install, PATH refresh, upgrade/uninstall ownership,
   and service survival without a signed-in desktop session.
4. Tauri LAN and Tailscale save/edit, direct-route fallback, managed SSH
   password/key/agent prompt, host-key change warning, reconnect, configurable
   local forwarding, and automated pairing with manual recovery available.
5. Live server coverage for the supported Java flavors and Bedrock, including
   same-day time actions and console retention while polling, backups, and
   helper processes are active.
