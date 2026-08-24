# P11.28 tri-platform candidate evidence

The `toolchain` matrix builds the one production Svelte bundle and a real
debug Tauri binary on macOS, Linux, and Windows. Each runner runs the same
formatting, type-check, unit-test, production-build, bundle-identity, agent
bundle, and browser-workflow checks. Linux also runs the native WebKitGTK
Tauri workflow in `linux-webkitgtk-smoke.sh`; Playwright's bundled WebKit is
browser evidence only and never substitutes for that desktop proof.

Each job uploads `desktop-web-evidence-<platform>`, a generated record of its
Node, Rust, Tauri, and native renderer/package version. The Linux native smoke
also produces its screenshot and `linux-webkitgtk-native.md`. P11.29 will tie
the exact successful workflow run and its retained artifacts to the Phase 11
candidate; this file deliberately does not claim that a run has succeeded.

## Signing and notarization status

Signing and notarization are **unavailable** in this CI workflow. It has no
release certificate, notarization credential, signed installer, or published
distribution artifact. The macOS and Windows builds are debug, unbundled
Tauri candidates only. This is intentional evidence of what the candidate did
not prove, not a claim that release distribution was tested.
