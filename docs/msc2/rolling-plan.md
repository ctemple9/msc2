# MSC 2 — Rolling Plan

> ## STATUS: Phases 14 and 15 are complete and archived. Release v0.1.13 published, but its main CI workflow failed on Windows/Linux/macOS clippy compilation and the Phase 9 capability-matrix check. P14.50 and P14.51 repair those failures and the Ubuntu 26.04 headless PolicyKit update path; both await Cameron verification. P14.52 adds visible, chunk-level progress to large modpack uploads and regression coverage for the remote create-server flow; it awaits Cameron verification. The external static-review record is preserved in the archive. All prior verification entries are recorded DONE, with P15.69 retaining its accepted failed-verification result.
> **Next move:** Cameron verifies P14.52 and closes it, then verifies P14.50 and P14.51. The external review findings remain available in `rolling-plan-archive.md` for future triage. The current workspace has unrelated pre-existing diagnostics: a `dead_code` failure in `crates/msc-application/tests/provisioning.rs:152` and an `unused_mut` warning in `crates/msc-agent/src/routes/bedrock_runtime.rs:385`. Phase 12 visual parity, anti-slop review, release/update handoff, and Bedrock product acceptance are recorded complete on 2026-09-08. P12.121–P12.189 are archived below with all verification entries recorded as DONE. The planned Phase 13 full-screen terminal client remains retired by D-034.

The detailed Phase 12 working plan is preserved in `rolling-plan-archive.md` under “Reconciliation snapshot — 2026-09-08”. This file contains only the current status and next move.

---

## How this document works

This is the working state of the build. The vision documents say where MSC 2 is going; the port plan says the intended sequence; this file says where the repository actually is now.

Phases come from `msc2-port-plan.md`. Steps are written as work arrives rather than being invented in advance. Each step has a status, file scope, description, verification command, commit subject, and batch classification.

Phase 12 is complete. Phases 14 and 15 are complete and archived, with Linux service corrections and the release below reopened from Phase 14. The archive already assigns P14.21–P14.23 to earlier work, so these corrective steps use the next unused identifiers.

### Phase 14 corrective work — Linux desktop service elevation

#### P14.38 — Request elevation for Linux desktop service changes

Status: awaiting Cameron verification
Files: clients/desktop-web/src-tauri/src/lib.rs, crates/msc-platform-linux/src/service.rs, crates/msc-agent/src/main.rs, crates/msc-agent/src/cli/mod.rs, crates/msc-agent/src/cli/service.rs, packaging/agent-service-layout.json, docs/msc2/rolling-plan.md
What: Route the desktop app’s local Linux install, repair, and uninstall actions through an elevation prompt and a fixed-name helper that can change only MSC’s own systemd service. Validate the helper’s executable and data paths; keep the service and its data owned by the installing user. Show a clear error if elevation is unavailable or cancelled.
Verify: cargo fmt --all -- --check && cargo check -p msc-platform-linux -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml
Commit: P14.38: request elevation for Linux desktop service changes
Batch: solo

#### P14.39 — Prove the elevated service flow without installing it

Status: awaiting Cameron verification
Files: crates/msc-platform-linux/tests/desktop_service_elevation.rs, crates/msc-platform-linux/src/service.rs, crates/msc-agent/src/main.rs, docs/msc2/rolling-plan.md
What: Add one focused test using a fake authorization runner, fake systemctl, and temporary directories. Check install, repair, and uninstall go through the privileged boundary; the unit still names your user and group; and the test never touches /etc/systemd/system or runs real pkexec or systemctl.
Verify: cargo test -p msc-platform-linux --test desktop_service_elevation && command -v pkexec && rpm -q polkit && systemctl --version
Commit: P14.39: test Linux desktop service elevation safely
Batch: solo

#### P14.40 — Publish the corrected desktop release

Status: awaiting Cameron verification
Files: coordinated version manifests and lockfiles, clients/desktop-web/src/lib/bundle-identity.ts, clients/desktop-web/src/lib/bundle-identity.test.ts, README.md, docs/msc2/rolling-plan.md
What: Bump the coordinated version from 0.1.10 to 0.1.11, update the download instructions, push the release preparation to main, and push tag v0.1.11 to trigger the GitHub release workflow. The initial Linux workflow exposed a CLI error-conversion compile failure, corrected in P14.41. The retried workflow succeeded and published the Linux RPM. The first agent installation will be Cameron's install from that release. If release signing or the workflow blocks publication, stop and report the blocker.
Verify: gh release view v0.1.11 --json url,isPrerelease,assets
Commit: P14.40: publish Linux desktop service elevation fix
Batch: solo

#### P14.41 — Fix Linux helper CLI error conversion

Status: awaiting Cameron verification
Files: crates/msc-agent/src/main.rs, docs/msc2/rolling-plan.md
What: Convert Linux service helper errors to their display text before wrapping them in the CLI's internal error type, so the Linux-only desktop helper compiles for release builds.
Verify: cargo fmt --all -- --check && cargo check -p msc-platform-linux -p msc-agent
Commit: P14.41: fix Linux helper CLI error conversion
Batch: solo

#### P14.42 — Record corrected release publication

Status: awaiting Cameron verification
Files: docs/msc2/rolling-plan.md
What: Record that the retried v0.1.11 workflow succeeded and published the Fedora RPM, DEB, and other release assets after the Linux helper compile correction.
Verify: gh release view v0.1.11 --json url,isPrerelease,assets
Commit: P14.42: record corrected desktop release publication
Batch: solo

### Follow-up — Fedora client issues

P14.43 addresses the Xbox Broadcast issue reported from the Fedora desktop connected to a remote Linux agent. P14.44 addresses the separate modpack memory issue.

#### P14.43 — Make Xbox Broadcast sign-in available from Settings

Status: awaiting Cameron verification
Files: clients/desktop-web/src/App.svelte, clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte, clients/desktop-web/src/lib/sections/server-editor/BroadcastAuthSheet.svelte, clients/desktop-web/src/lib/components/shell/sidebar/HowToConnectSection.svelte, clients/desktop-web/src/lib/sections/connectivity/ConnectivitySection.svelte, clients/desktop-web/src/lib/sections/components/model.ts, clients/desktop-web/src/lib/api/generated.ts, crates/msc-agent/src/routes/networking.rs, crates/msc-agent/tests/xbox_broadcast_routes.rs, crates/msc-api/src/dto/networking.rs, docs/msc2/api-contract/openapi.json, docs/msc2/rolling-plan.md
What: Let Settings start Xbox Broadcast sign-in for the selected eligible server while its Minecraft process is stopped, and surface the device code/link in a sheet that works for remote clients. Keep the first-start flow using the same prompt behavior. Make the sidebar's connection row reflect the agent's authenticated Xbox identity after sign-in instead of remaining at “Not signed in yet”; read status from agent-owned auth state and runtime output consistently.
Verify: cargo fmt --all -- --check && cargo clippy -p msc-agent -p msc-api --lib --bins -- -D warnings -A unused-mut && cargo check -p msc-agent -p msc-api && npm --prefix clients/desktop-web run api:check && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build
Commit: P14.43: make xbox broadcast sign-in available from settings
Batch: solo

#### P14.44 — Stream modpack imports with bounded memory

Status: awaiting Cameron verification
Files: clients/desktop-web/src/App.svelte, clients/desktop-web/src/lib/api/client.ts, clients/desktop-web/src/lib/api/generated.ts, clients/desktop-web/src/lib/platform/types.ts, clients/desktop-web/src/lib/platform/browser.ts, clients/desktop-web/src/lib/platform/tauri.ts, clients/desktop-web/src/lib/sections/shared/types.ts, clients/desktop-web/src/lib/sections/fleet/wizard/UploadStep.svelte, clients/desktop-web/src/lib/sections/fleet/wizard/AddOnsStep.svelte, clients/desktop-web/src/lib/sections/components/ImportModpackSheet.svelte, crates/msc-agent/src/cli/mod.rs, crates/msc-agent/src/routes/components.rs, crates/msc-agent/src/routes/servers.rs, crates/msc-agent/src/routes/worlds.rs, crates/msc-api/src/dto/backups.rs, docs/msc2/api-contract/openapi.json, docs/msc2/rolling-plan.md
What: Replace whole-file modpack reads and single-body uploads with a bounded-memory staged upload path. The desktop reads and sends 2 MiB chunks from the selected local file, including through the existing Tauri authorized-request bridge; the agent appends chunks to disk, enforces contiguous offsets, purpose, exact size and byte ceilings, and enables redemption only after the whole archive is received and hashed. Browser imports use bounded slices through the same path.
Verify: cargo fmt --all -- --check && cargo check -p msc-agent -p msc-api && cargo clippy -p msc-agent -p msc-api -- -D warnings -A unused-mut && npm --prefix clients/desktop-web run api:check && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build
Commit: P14.44: stream modpack imports with bounded memory
Batch: solo

#### P14.45 — Keep Linux update authorization attached to its caller

Status: awaiting Cameron verification
Files: crates/msc-agent/src/cli/update.rs, docs/msc2/rolling-plan.md
What: Run the privileged Linux updater in the foreground and wait for polkit to finish, so its caller remains alive for authorization. Report cancellation and authorization failures as failed installs instead of saying the update was scheduled. Add a Linux-only fake-authorizer test that proves the CLI waits and handles cancellation without a release build, real polkit, or a service restart.
Verify: cargo fmt --all -- --check && cargo test -p msc-agent --bin msc cli::update::tests::authorized_update_waits_for_authorizer_and_reports_cancellation -- --exact && cargo clippy -p msc-agent --bin msc -- -D warnings -A unused-mut && cargo check -p msc-agent --bin msc
Commit: P14.45: wait for Linux update authorization
Batch: solo

#### P14.46 — Add Fedora regression coverage for sign-in, modpack streaming, and updates

Status: awaiting Cameron verification
Files: clients/desktop-web/tests/screens/fedora-regressions.test.ts, clients/desktop-web/tests/transport/transport.test.ts, crates/msc-agent/src/routes/components.rs, crates/msc-agent/src/cli/update.rs, docs/msc2/rolling-plan.md
What: Prove the Xbox Broadcast Settings action can start sign-in while Minecraft is stopped, the shell polls the selected agent and shows its device-code sheet, and the sidebar renders the agent's authenticated identity. Exercise the desktop modpack upload client across multiple bounded chunks and a short-read failure; exercise the agent route's offset rejection, ordered append, and completion response. Extend the Linux fake-authorizer test to check release arguments, data directory, JSON forwarding, foreground wait, and cancellation without building a release or invoking real polkit.
Verify: cargo fmt --all -- --check && cargo test -p msc-agent --bin msc routes::components::staged_upload_tests::chunked_modpack_upload_requires_order_and_completes_with_verified_size -- --exact && cargo test -p msc-agent --bin msc cli::update::tests::authorized_update_waits_for_authorizer_and_reports_cancellation -- --exact && cargo clippy -p msc-agent --bin msc -- -D warnings -A unused-mut && npx --prefix clients/desktop-web vitest run tests/transport/transport.test.ts tests/screens/fedora-regressions.test.ts && npm --prefix clients/desktop-web run check
Commit: P14.46: add Fedora regression coverage for sign-in, uploads, and updates
Batch: solo

#### P14.47 — Prepare the v0.1.12 prerelease

Status: awaiting Cameron verification
Files: crates/msc-agent/Cargo.toml, Cargo.lock, clients/desktop-web/package.json, clients/desktop-web/package-lock.json, clients/desktop-web/src-tauri/Cargo.toml, clients/desktop-web/src-tauri/Cargo.lock, clients/desktop-web/src-tauri/tauri.conf.json, clients/desktop-web/src/lib/bundle-identity.ts, clients/desktop-web/src/lib/bundle-identity.test.ts, .github/workflows/release.yml, tools/release/check-release-workflow.py, README.md, docs/msc2/rolling-plan.md
What: Synchronize the app, agent, Tauri shell, bundle identity, lockfiles, and download instructions to 0.1.12. Add the new Fedora regression tests to the release matrix so desktop transport and remote sign-in checks run on each platform and the Linux agent chunking and updater authorization checks run before packaging. Push the release-preparation commit and exact v0.1.12 tag to start the guarded cross-platform prerelease workflow.
Verify: python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard && python3 tools/release/check-update-gate.py && npm --prefix clients/desktop-web run bundle:identity && gh run list --workflow release.yml --commit "$(git rev-parse v0.1.12)" --json databaseId,status,conclusion,url && gh release view v0.1.12 --json url,isPrerelease,assets
Commit: P14.47: prepare 0.1.12 prerelease
Batch: solo

### Follow-up — Fedora remote modpack upload

#### P14.48 — Preserve bodyless responses in the desktop transport

Status: awaiting Cameron verification
Files: clients/desktop-web/src/lib/auth/desktop.ts, clients/desktop-web/tests/auth/desktop/desktop.test.ts, docs/msc2/rolling-plan.md
What: Make the native desktop transport construct bodyless HTTP responses without a response body, so each accepted 2 MiB chunk can return 204 and the Fedora client can continue uploading a remote modpack. Add a remote-host transport regression using an All the Mods 10 archive name, multiple chunks, and the agent's 204 intermediate response.
Verify: npm --prefix clients/desktop-web run test:auth-desktop && npm --prefix clients/desktop-web run test:fedora-regressions && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check
Commit: P14.48: handle bodyless desktop upload responses
Batch: solo

#### P14.49 — Publish the corrected v0.1.13 prerelease

Status: awaiting Cameron verification (release published; main CI failed)
Files: coordinated version manifests and lockfiles, clients/desktop-web/src/lib/bundle-identity.ts, README.md, docs/msc2/rolling-plan.md
What: Bump the coordinated release identity to 0.1.13, update download instructions, commit and push the release preparation, then push exact tag v0.1.13 to trigger the guarded prerelease workflow. The release workflow published successfully. Main CI failed because an optional DTO field was missing from a fixture, a Linux-only integration target compiled on Windows, and the client capability matrix lacked the staged-upload chunk route; P14.50 repairs those CI failures.
Verify: python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard && python3 tools/release/check-update-gate.py && npm --prefix clients/desktop-web run bundle:identity && gh run list --workflow release.yml --commit "$(git rev-parse v0.1.13)" --json databaseId,status,conclusion,url && gh release view v0.1.13 --json url,isPrerelease,assets
Commit: P14.49: prepare 0.1.13 prerelease
Batch: solo

#### P14.50 — Fix release CI compile and capability-matrix failures

Status: awaiting Cameron verification
Files: crates/msc-api/tests/world_backup_conformance.rs, crates/msc-platform-linux/tests/desktop_service_elevation.rs, docs/msc2/client-capability-matrix.csv, docs/msc2/rolling-plan.md
What: Complete the newly required optional byte-count field in the world-backup staged-upload fixture; compile the Linux desktop-elevation integration target only on Linux so Windows does not try to build Unix APIs; and record the staged-upload chunk route in the API/client capability matrix required by the Phase 9 smoke check.
Verify: cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv
Commit: P14.50: fix cross-platform CI regressions
Batch: solo

#### P14.51 — Use a separate PolicyKit agent for headless updates

Status: awaiting Cameron verification
Files: crates/msc-agent/src/cli/update.rs, docs/msc2/clients/headless-installation.md, docs/msc2/rolling-plan.md
What: Before running pkexec for a protected Linux archive update, register an unprivileged pkttyagent for the live CLI process and wait for its documented registration notification. Preserve any already-registered agent, disable pkexec's buggy built-in fallback, keep authorization foreground and report cancellation/failure accurately, and stop the temporary agent on every exit path. Document the controlling-terminal requirement for headless SSH updates.
Verify: cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc -- -D warnings -A unused-mut && cargo check -p msc-agent --bin msc
Commit: P14.51: use separate polkit agent for headless updates
Batch: solo

#### P14.52 — Show progress while staging large modpacks

Status: awaiting Cameron verification
Files: clients/desktop-web/src/App.svelte, clients/desktop-web/src/lib/api/client.ts, clients/desktop-web/src/lib/platform/types.ts, clients/desktop-web/src/lib/sections/shared/types.ts, clients/desktop-web/src/lib/sections/components/ModpackUploadProgress.svelte, clients/desktop-web/src/lib/sections/components/ImportModpackSheet.svelte, clients/desktop-web/src/lib/sections/fleet/wizard/UploadStep.svelte, clients/desktop-web/src/lib/sections/fleet/wizard/AddOnsStep.svelte, clients/desktop-web/tests/transport/transport.test.ts, clients/desktop-web/tests/auth/desktop/desktop.test.ts, clients/desktop-web/tests/tauri/platform-boundary.test.ts, clients/desktop-web/tests/e2e/browser/contract-harness.mjs, clients/desktop-web/tests/e2e/browser/workflows.spec.ts, crates/msc-agent/src/routes/components.rs, docs/msc2/rolling-plan.md
What: Send chunk-level progress from the existing 2 MiB bounded upload transport to a visible, non-dismissable progress sheet in all modpack upload entry points. Keep the sheet open through remote inspection and show picker/upload errors in the current flow. Add transport progress assertions and a browser end-to-end regression that selects a multi-chunk AllTheMods10-named archive, observes progress while the remote host is slow to accept chunks, verifies no request exceeds 2 MiB, and continues through the network, world, and confirmation steps. Expand the agent's real staged-upload route regression to receive multiple 2 MiB chunks in order before it reports the verified byte count.
Verify: cargo fmt --all -- --check && cargo test -p msc-agent --bin msc routes::components::staged_upload_tests::chunked_modpack_upload_requires_order_and_completes_with_verified_size -- --exact && cargo test -p msc-application --test modpack_inspection && npm --prefix clients/desktop-web run test:transport && npm --prefix clients/desktop-web run test:auth-desktop && npm --prefix clients/desktop-web run test:tauri-boundary && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build && npm --prefix clients/desktop-web run test:e2e-browser:artifact -- tests/e2e/browser/workflows.spec.ts --project=chromium
Commit: P14.52: show progress while staging large modpacks
Batch: solo

## Current phase

| Phase | Name | State |
|---|---|---|
| Setup | Repo, docs, agent instructions, CI, editor config | complete |
| 0 | Freeze the baseline and build the harness | complete |
| 1 | Domain types and pure rules | complete |
| 2 | API contract and operation model | complete |
| 3 | Safety substrate | complete |
| 4 | Java lifecycle vertical slice | complete |
| 5 | Configuration and migration | complete |
| 6 | Worlds and backups | complete |
| 7 | Server families and provisioning | complete |
| 8 | Mods, plugins, modpacks | complete |
| 9 | Networking and helpers | complete |
| 10 | Bedrock runtimes | complete |
| 11 | Desktop and web clients | complete |
| 12 | Client redesign and post-phase corrections | complete |
| 13 | Full-screen terminal client | retired by D-034 |

## Historical records

Detailed records for Setup through Phase 12, including the completed P12.121–P12.189 steps, remain in `rolling-plan-archive.md`.
