# MSC 2 — Rolling Plan

> ## STATUS: Phases 14 and 15 are complete and archived. Releases v0.1.13 through v0.1.15 are published. P14.50 and P14.51 repair the v0.1.13 CI failures and the Ubuntu 26.04 headless PolicyKit update path; both await Cameron verification. P14.52 adds visible, chunk-level progress to large modpack uploads and regression coverage for the remote create-server flow; it awaits Cameron verification. P14.54 fixes the missing Tauri file-stream permission exposed by the Fedora test; it awaits Cameron verification. P14.55 adds selectable chunk sizing, streamed world archives, and cancellation of partial uploads; it awaits Cameron verification. P14.56 updates the browser regressions for the upload sheet and rebuilds the production artifact before testing; it awaits Cameron verification. P14.57 prepares v0.1.15 and awaits Cameron verification. P14.58 adapts Linux updates to the headless service unit installed by the signed archive; it awaits Cameron verification. P14.59 adds a regression using the exact archive-installed unit template; it awaits Cameron verification. P14.60 prepares v0.1.16 with the Linux update repair and awaits the guarded release workflow. The external static-review record is preserved in the archive. All prior verification entries are recorded DONE, with P15.69 retaining its accepted failed-verification result.
> **Next move:** Push the coordinated v0.1.16 preparation commit and exact tag to start the guarded cross-platform release workflow. Cameron verifies P14.55, then P14.54, P14.52, P14.50, and P14.51. The external review findings remain available in `rolling-plan-archive.md` for future triage. The current workspace has unrelated pre-existing diagnostics: a `dead_code` failure in `crates/msc-application/tests/provisioning.rs:152` and an `unused_mut` warning in `crates/msc-agent/src/routes/bedrock_runtime.rs:385`. Phase 12 visual parity, anti-slop review, release/update handoff, and Bedrock product acceptance are recorded complete on 2026-09-08. P12.121–P12.189 are archived below with all verification entries recorded as DONE. The planned Phase 13 full-screen terminal client remains retired by D-034.

> **Fedora local-agent follow-up:** P14.61–P14.67 await Cameron verification. P14.67 installs the corrected local helper package and repairs the live service; the agent and credential helper run successfully, the health endpoint returns HTTP 204, and the service executable matches the current Cargo debug build. P14.68 fixes Linux Reconnect using native host-local pairing; Cameron's next verification is restarting `npx tauri dev` and checking the connection and four installed-service controls.

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

#### P14.53 — Prepare and trigger the v0.1.14 prerelease

Status: awaiting release workflow
Files: crates/msc-agent/Cargo.toml, Cargo.lock, clients/desktop-web/package.json, clients/desktop-web/package-lock.json, clients/desktop-web/src-tauri/Cargo.toml, clients/desktop-web/src-tauri/Cargo.lock, clients/desktop-web/src-tauri/tauri.conf.json, clients/desktop-web/src/lib/bundle-identity.ts, clients/desktop-web/src/lib/bundle-identity.test.ts, README.md, docs/msc2/rolling-plan.md
What: Bump the coordinated release identity from 0.1.13 to 0.1.14 and update download and install instructions. Push the preparation commit and exact v0.1.14 tag to start the guarded cross-platform release workflow, which runs the modpack-upload and updater regressions before publishing.
Verify: python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard && python3 tools/release/check-update-gate.py && npm --prefix clients/desktop-web run bundle:identity && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build && cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format
Commit: P14.53: prepare 0.1.14 prerelease
Batch: solo

#### P14.54 — Enable scoped native file streaming for modpack imports

Status: awaiting Cameron verification
Files: clients/desktop-web/src-tauri/capabilities/default.json, clients/desktop-web/src/lib/sections/shared/types.ts, clients/desktop-web/tests/tauri/platform-boundary.test.ts, .github/workflows/release.yml, tools/release/check-release-workflow.py, docs/msc2/rolling-plan.md
What: Grant the Tauri filesystem read commands needed to open, seek, and read a file explicitly chosen through the native picker. The picker scopes access to the selected file. Preserve native string rejections in the UI instead of replacing them with a misleading remote-agent error, and run a capability regression in release CI.
Verify: npm --prefix clients/desktop-web run test:tauri-boundary && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml && python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard
Commit: P14.54: enable native streaming for modpack imports
Batch: solo

#### P14.55 — Add upload sizing and cancellation for large imports

Status: awaiting Cameron verification
Files: clients/desktop-web/src/App.svelte, clients/desktop-web/src/lib/api/client.ts, clients/desktop-web/src/lib/api/generated.ts, clients/desktop-web/src/lib/platform/types.ts, clients/desktop-web/src/lib/platform/upload-settings.ts, clients/desktop-web/src/lib/sections/shared/types.ts, clients/desktop-web/src/lib/sections/components/ModpackUploadProgress.svelte, clients/desktop-web/src/lib/sections/components/StagedUploadSheet.svelte, clients/desktop-web/src/lib/sections/components/ImportModpackSheet.svelte, clients/desktop-web/src/lib/sections/fleet/wizard/UploadStep.svelte, clients/desktop-web/src/lib/sections/fleet/wizard/AddOnsStep.svelte, clients/desktop-web/src/lib/sections/fleet/wizard/WorldStep.svelte, clients/desktop-web/src/lib/sections/fleet/wizard/model.ts, clients/desktop-web/src/lib/sections/worlds/ImportWorldZipSheet.svelte, clients/desktop-web/src/lib/sections/worlds/ReplaceWorldSheet.svelte, clients/desktop-web/tests/transport/transport.test.ts, clients/desktop-web/tests/auth/desktop/desktop.test.ts, clients/desktop-web/tests/e2e/browser/contract-harness.mjs, clients/desktop-web/tests/e2e/browser/workflows.spec.ts, crates/msc-agent/src/routes/components.rs, crates/msc-agent/src/routes/worlds.rs, crates/msc-api/src/dto/backups.rs, crates/msc-api/tests/world_backup_conformance.rs, docs/msc2/api-contract/openapi.json, docs/msc2/client-capability-matrix.csv, .github/workflows/release.yml, tools/release/check-release-workflow.py, docs/msc2/rolling-plan.md
What: Let users choose a remembered 1–8 MiB upload chunk size before each large file transfer, defaulting to 2 MiB. Negotiate the agent's chunk ceiling so older modpack agents safely stay at 2 MiB. Stream modpack archives, imported world ZIPs, and active-world replacement ZIPs from disk through the same bounded-memory route. Add a visible cancel action that stops further chunks and asks the agent to delete the partial staged file; keep the progress sheet visible while the active request or cleanup finishes. Extend the API contract and release regressions for chunk sizing, world-purpose acceptance, and cancellation cleanup.
Verify: cargo fmt --all -- --check && cargo test -p msc-agent --bin msc routes::components::staged_upload_tests::chunked_modpack_upload_requires_order_and_completes_with_verified_size -- --exact && cargo test -p msc-agent --bin msc routes::components::staged_upload_tests::world_import_chunks_can_be_cancelled_and_removed_idempotently -- --exact && npm --prefix clients/desktop-web run test:fedora-regressions && npm --prefix clients/desktop-web run test:auth-desktop && npm --prefix clients/desktop-web run api:check && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build && npm --prefix clients/desktop-web run test:e2e-browser:artifact -- tests/e2e/browser/workflows.spec.ts --project=chromium && python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard
Commit: P14.55: add upload sizing and cancellation
Batch: solo

#### P14.56 — Make the upload browser regression deterministic

Status: awaiting Cameron verification
Files: clients/desktop-web/tests/e2e/browser/workflows.spec.ts, docs/msc2/rolling-plan.md
What: Make the browser regression exercise the new pre-upload choice for both modpacks and world ZIPs. Hold the final chunk response long enough to observe a nonzero progress value, and build the production bundle before the artifact-based browser test so it cannot silently exercise a stale client.
Verify: npm --prefix clients/desktop-web run build && npm --prefix clients/desktop-web run test:e2e-browser:artifact -- tests/e2e/browser/workflows.spec.ts --project=chromium
Commit: P14.56: fix upload browser regressions
Batch: solo

#### P14.57 — Prepare and trigger the v0.1.15 prerelease

Status: awaiting release workflow
Files: crates/msc-agent/Cargo.toml, Cargo.lock, clients/desktop-web/package.json, clients/desktop-web/package-lock.json, clients/desktop-web/src-tauri/Cargo.toml, clients/desktop-web/src-tauri/Cargo.lock, clients/desktop-web/src-tauri/tauri.conf.json, clients/desktop-web/src/lib/bundle-identity.ts, clients/desktop-web/src/lib/bundle-identity.test.ts, README.md, docs/msc2/rolling-plan.md
What: Bump the coordinated release identity from 0.1.14 to 0.1.15 and update download/install instructions. Include P14.54 and P14.55 so the desktop client and remote headless agent both support the 8 MiB upload limit and staged-upload cancellation. Push the preparation commit and exact v0.1.15 tag to start the guarded cross-platform release workflow; publication remains gated on the matrix checks and release regressions.
Verify: gh release view v0.1.15 --json url,isPrerelease,assets
Commit: P14.57: prepare 0.1.15 prerelease
Batch: solo

#### P14.58 — Update Linux headless installs without private unit metadata

Status: awaiting Cameron verification
Files: crates/msc-agent/src/cli/update.rs, crates/msc-platform-linux/src/service.rs, docs/msc2/rolling-plan.md
What: Make the privileged Linux update path read the fixed MSC systemd unit's runtime state and control that same unit directly. The standalone headless installer renders a valid unit without the private metadata comments required by the desktop service-definition parser; update and health-check rollback must continue to work for those existing installs without rewriting the unit.
Verify: cargo fmt --all -- --check && cargo check -p msc-agent --bin msc && cargo clippy -p msc-agent --bin msc -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format
Commit: P14.58: support headless units in Linux updates
Batch: solo

#### P14.59 — Regress the archive-installed Linux update unit

Status: awaiting Cameron verification
Files: crates/msc-platform-linux/tests/systemd_unit.rs, docs/msc2/rolling-plan.md
What: Load the actual Linux archive service-unit template, which has no private service metadata, then verify the updater can read its state and stop/start the fixed MSC unit name.
Verify: cargo fmt --all -- --check && cargo test -p msc-platform-linux --test systemd_unit archive_unit_update_lifecycle_works_without_private_metadata -- --exact
Commit: P14.59: cover metadata-free Linux update units
Batch: solo

#### P14.60 — Prepare and trigger the v0.1.16 prerelease

Status: awaiting release trigger
Files: crates/msc-agent/Cargo.toml, Cargo.lock, clients/desktop-web/package.json, clients/desktop-web/package-lock.json, clients/desktop-web/src-tauri/Cargo.toml, clients/desktop-web/src-tauri/Cargo.lock, clients/desktop-web/src-tauri/tauri.conf.json, clients/desktop-web/src/lib/bundle-identity.ts, clients/desktop-web/src/lib/bundle-identity.test.ts, README.md, docs/msc2/rolling-plan.md
What: Bump the coordinated release identity from 0.1.15 to 0.1.16 and update download/install instructions. Include the Linux headless update fix and its archive-unit regression. Push the preparation commit and exact v0.1.16 tag to start the guarded cross-platform release workflow; publication remains gated on the matrix checks and release regressions.
Verify: gh release view v0.1.16 --json url,isPrerelease,assets
Commit: P14.60: prepare 0.1.16 prerelease
Batch: solo

#### P14.61 — Align the Linux desktop bootstrap socket path

Status: awaiting Cameron verification
Files: clients/desktop-web/src-tauri/src/lib.rs, crates/msc-platform-linux/src/service.rs, crates/msc-platform-linux/tests/desktop_service_elevation.rs, docs/msc2/rolling-plan.md
What: Use `local-bootstrap.sock` in the desktop install request, matching the Linux service helper's allowlisted path. The prior desktop value, `bootstrap.sock`, caused the helper to reject the otherwise valid environment before writing the service unit.
Verify: cargo fmt --all -- --check && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml
Commit: P14.61: align linux desktop bootstrap socket path
Batch: solo

#### P14.62 — Use the installed helper from Linux development builds

Status: awaiting Cameron verification
Files: clients/desktop-web/src-tauri/src/lib.rs, docs/msc2/rolling-plan.md
What: Keep staging the agent payload from the running desktop build, while selecting the root-owned helper from the installed Linux package for the privileged service change. This lets `npx tauri dev` exercise current agent code without passing a helper from the user-writable Cargo target directory to `pkexec`; the helper still validates its ownership and all parent directories.
Verify: cargo fmt --all -- --check && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml
Commit: P14.62: use installed helper for linux development builds
Batch: solo

#### P14.63 — Preserve option-like agent arguments across elevation

Status: awaiting Cameron verification
Files: crates/msc-platform-linux/src/service.rs, docs/msc2/rolling-plan.md
What: Pass each service argument to the elevated helper as one `--arg=value` option token. This keeps agent flags such as `--bind` attached to the helper's `--arg` field instead of letting the helper CLI parse them as its own options.
Verify: cargo fmt --all -- --check && cargo check -p msc-platform-linux -p msc-agent
Commit: P14.63: pass agent arguments safely through helper
Batch: solo

#### P14.64 — Run the Linux desktop agent from its package path

Status: awaiting Cameron verification
Files: clients/desktop-web/src-tauri/src/lib.rs, crates/msc-platform-linux/src/service.rs, docs/msc2/rolling-plan.md
What: Point the Linux desktop service at the root-owned agent executable installed under `/usr/lib`, while continuing to run the service as the installing user and keeping its data in that user's home. Accept only the two fixed MSC package paths and validate their ownership and parent directories. This avoids Fedora SELinux denying systemd execution of the staged binary labeled `data_home_t`.
Verify: cargo fmt --all -- --check && cargo check -p msc-platform-linux -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml
Commit: P14.64: use packaged agent for linux desktop service
Batch: solo

#### P14.65 — Refresh Fedora's trusted agent copy from development builds

Status: awaiting Cameron verification
Files: clients/desktop-web/src-tauri/src/lib.rs, crates/msc-platform-linux/Cargo.toml, crates/msc-platform-linux/src/service.rs, Cargo.lock, packaging/agent-service-layout.json, docs/msc2/rolling-plan.md
What: Make Linux debug desktop builds stage the freshly compiled development agent on every install or repair. The elevated helper verifies its content digest, copies it into a root-owned directory beside the installed helper so Fedora applies the system executable label, and points systemd at that immutable copy. Packaged release builds continue to use their packaged agent. This lets repeated `npx tauri dev` runs exercise current Rust agent edits without replacing the installed helper each time.
Verify: cargo fmt --all -- --check && cargo check -p msc-platform-linux -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml
Commit: P14.65: refresh fedora development agent builds
Batch: solo

#### P14.66 — Import the Unix file mode extension

Status: awaiting Cameron verification
Files: crates/msc-platform-linux/src/service.rs, docs/msc2/rolling-plan.md
What: Import `OpenOptionsExt` so the helper can create its root-owned staged agent file with executable permissions on Linux.
Verify: cargo fmt --all -- --check && cargo check -p msc-platform-linux -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml
Commit: P14.66: import unix file mode extension
Batch: solo

#### P14.67 — Keep repair, start, and the service controls consistent

Status: awaiting Cameron verification
Files: clients/desktop-web/src-tauri/src/lib.rs, clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte, crates/msc-platform-linux/src/service.rs, packaging/agent-service-layout.json, docs/msc2/rolling-plan.md
What: Compare Linux repair results with the final system executable path used by status and start. Resolve the development agent directly from Cargo's debug output so Tauri resource copying cannot substitute a release build. Keep Start, Stop, Reconnect, and Repair visible for an installed service, including when it is stopped or its build differs. Protect each system build directory before copying executable bytes and restore the executable's SELinux context. Install the required credential helper service, socket, root-owned store, and boot-time socket directory before starting the agent; wait for its public health route before reporting repair success. Build and install a local RPM containing the current helper so development repairs stop invoking the older installed helper that writes the home-directory executable into the unit.
Verify: cargo fmt --all -- --check && cargo check -p msc-platform-linux -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml && npm --prefix clients/desktop-web run check
Commit: P14.67: align linux repair paths and service controls
Batch: solo
Evidence: On 2026-09-26, the installed helper completed the live repair with exit status 0. The agent runs as Cameron, the credential helper runs as root, both units are active, the agent and helper socket are enabled for boot, and the health endpoint returns HTTP 204. The service executable's SHA-256 matches `target/debug/msc`. Formatting, focused Rust Clippy checks, Svelte checks, and RPM builds passed; no tests ran. Visual confirmation of the rebuilt desktop controls remains Cameron's verification.

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


#### P14.68 — Connect the Linux desktop through host-local pairing

Status: awaiting Cameron verification
Files: clients/desktop-web/src-tauri/src/lib.rs, docs/msc2/rolling-plan.md
What: Fix desktop_bootstrap_local returning unsupported on Linux despite the agent service running. Use the existing host-local one-use desktop pairing command and redeem its code inside the native backend, following phase11-auth.md's pairing fallback when signed-process bootstrap is unavailable. Run the installed service's expected executable with the same MSC2_DATA_DIR. Keep pairing codes, child output, and bearer credentials out of webview responses and errors. Save the local host identity and check saved credentials through authenticated /v1/me before reuse. Await the native bootstrap from browser handoff commands as well.
Evidence: Live Fedora host-local pairing command exited 0; pairing redemption returned HTTP 200 and the resulting credential authenticated /v1/me with HTTP 200. Native cargo check, cargo clippy (allowing the two existing update dead-code warnings), cargo build, formatting, and git diff checks passed. No tests created or run.
Verify: Restart `npx tauri dev` in clients/desktop-web, click Reconnect, and confirm the connection becomes available while all four service controls remain present; close and reopen the dev app and reconnect again to confirm the saved credential works.
Commit: P14.68: connect linux desktop through host-local pairing
Batch: solo


#### P14.69 — Label Linux agent executables for daemon startup

Status: awaiting Cameron verification
Files: crates/msc-platform-linux/src/service.rs, docs/msc2/rolling-plan.md
What: Fix Fedora Bedrock manifest downloads failing with Permission denied. Audit evidence showed init_t denied outbound HTTPS and child setpgid; copied agents had inherited lib_t from their /usr/lib directory, which did not transition systemd into a service domain. On SELinux hosts, elevated install/repair registers a persistent bin_t file-context rule limited to the two supported packaged agent locations and their hash-named dev executables, then applies restorecon to both the helper and selected agent before service startup. Fedora's existing policy transitions bin_t from init_t into unconfined_service_t. Escape the package directory's space as a PCRE hexadecimal escape because semanage rejects literal spaces. Fail clearly if SELinux tools are unavailable. Apply this for packaged installations as well as development builds. Retain enforcing SELinux and existing service user identity.
Evidence: Live host persistent rule applied; restarted agent runs as unconfined_service_t. Authenticated GET /v1/versions/create?serverType=bedrock returned HTTP 200 and downloaded 58 version entries through the service's real HTTPS transport. Updated RPM installed with its helper matching the current release build; rpm verification reported no package changes. Debug and release agent builds, desktop/frontend release build, platform clippy, and formatting checks passed. No tests created or run.
Verify: Restart `npx tauri dev`, reconnect (repair once if the debug build changed), and create the Bedrock server again. Confirm its archive provisions and the server appears. `ps -Z -p $(systemctl show com.ctemple.msc2.agent.service -p MainPID --value)` must show unconfined_service_t.
Commit: P14.69: label linux agent executables for daemon startup
Batch: solo


#### P14.70 — Accept server IDs in Linux credential keys

Status: awaiting Cameron verification
Files: crates/msc-platform-linux/src/credential_helper.rs, docs/msc2/rolling-plan.md
What: Fix Xbox Broadcast setup during Bedrock initiation failing with credential key is not allowed. MSC creates uppercase UUID server IDs; Xbox's auth-token and legacy server-password keys append that ID. Permit ASCII uppercase letters in credential key suffixes as well as existing lowercase letters, digits, dots, and hyphens. Retain the namespace-leading restriction, length bound, traversal rejection, and rejection of whitespace and path separators. Update both the agent-side client validator and the installed privileged helper.
Evidence: Reproduced the live helper rejecting the actual server's Xbox auth-token key with no credential values displayed. Installed the corrected RPM and repaired the service with the current debug agent; both services are active and health returns HTTP 204. Live credential reads now accept the actual server's Xbox auth-token and legacy alt-password keys; no credential values displayed or changed. Debug/release agent builds, RPM packaging, platform clippy, formatting, and git diff checks passed. No tests created or run.
Verify: Restart `npx tauri dev`, reconnect, then initiate test-Bedrock and complete Xbox Broadcast setup. Confirm the credential-key error is gone and initiation proceeds to the account setup or connection check.
Commit: P14.70: accept server ids in linux credential keys
Batch: solo


#### P14.71 — Track Bedrock online names from runtime events

Status: awaiting Cameron verification
Files: crates/msc-domain/src/bedrock.rs, crates/msc-agent/src/routes/lifecycle.rs, crates/msc-agent/src/routes/players.rs, docs/msc2/rolling-plan.md
What: Fix the online roster displaying Unknown Player after a named Bedrock join. GET /v1/players incorrectly enumerated saved world LevelDB records, including unnamed server UUID records and disconnected players. Track actual connected players in lifecycle state from the existing Bedrock connection/disconnection parser; use that roster for the online endpoint, remove players on disconnect, and clear it for successful starts and termination. Persist console-provided name/XUID pairs through the existing name cache. Trim the XUID at its comma delimiter so modern disconnect messages' pfid field cannot become part of the account ID. Keep world player-data discovery separate.
Evidence: Inspected the screenshot's named connection and disconnect messages and traced the online endpoint to saved world records. Cargo check, domain clippy, agent clippy (allowing the existing unused-mut warning), formatting, git diff checks, and prepare:agent debug build passed. The updated agent is staged for the next dev run; the running Minecraft server was not stopped or replaced. MSC 1 oracle checkout is unavailable at its documented path on this host. No tests created or run.
Verify: Stop the Bedrock server, restart `npx tauri dev`, Repair Service to load the latest debug agent, reconnect, and start the server. Join: Players must show camkage; leave: it must disappear from Online Now. Player Data remains a separate saved-world view.
Commit: P14.71: track bedrock online names from runtime events
Batch: solo


#### P14.72 — Install Bedrock resource packs per world

Status: awaiting Cameron verification
Files: crates/msc-infrastructure/src/addon_provider.rs, crates/msc-application/src/addons.rs, crates/msc-application/src/worlds.rs, crates/msc-agent/src/routes/worlds.rs, clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte, clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte, clients/desktop-web/src/lib/api/generated.ts, docs/msc2/api-contract/openapi.json, docs/msc2/rolling-plan.md
What: Replace the Bedrock Behavior Packs section with World Packs and a single Browse Packs browser filtered by All, Resource Packs, or Behavior Packs. Search CurseForge's corresponding Bedrock categories; retain existing behavior-pack endpoints for compatibility. Support resource-only downloads and linked add-ons through the existing validated archive installer, and accept staged mcpack/mcaddon/zip file imports when catalog access is unavailable. Put pack folders and world activation lists inside the archive's selected Bedrock world directory. Refresh active-world progress before installation, preserve source records and join preferences on snapshots, and activate installed archive contents through the existing recovery transaction. Add a per-world Require resource packs to join preference mapped to BDS texturepack-required; default to optional downloads when activating a world without a preference. Packs reside inside the world so backups/export/switch carry their files. No Marketplace decryption or promise of Vibrant Visuals support on incompatible clients.
Evidence: Frontend type check, generated API check, and production build passed with existing frontend warnings. Agent build and scoped clippy passed with the pre-existing unused-mut warning allowed. No tests created or run. Minecraft's actual download prompt and Prizma rendering await Cameron verification. CurseForge API key is absent on this host; catalog downloading requires the existing provider setup, while file import works without it.
Verify: Stop the Bedrock server, restart npx tauri dev, Repair Service, and reconnect. In Worlds select the intended world, open Browse Packs, choose Resource Packs, and install a compatible pack (or Import Pack using the creator's mcpack). Start the server and join from Minecraft; accept its resource-pack download and confirm the pack appears. Toggle Require resource packs to join while stopped, verify declining prevents joining, and switch to a different world to confirm packs and preference stay with their world.
Commit: P14.72: install bedrock resource packs per world
Batch: solo


#### P14.73 — Search the Bedrock texture pack category

Status: awaiting Cameron verification
Files: crates/msc-infrastructure/src/addon_provider.rs, docs/msc2/rolling-plan.md
What: Fix resource-pack catalog search treating CurseForge's Texture Packs category as a top-level class. Resolve the Addons class, fetch its current categories, and pass the resolved Texture Packs category ID to mod search. This allows the existing Browse Packs search to find projects such as Prizma Visuals Legacy.
Evidence: Compared MSC's category lookup with CurseForge's current Prizma project page, which identifies it as Minecraft Bedrock → Texture Packs under the Addons class. Scoped clippy/build verification passes. No tests created or run.
Verify: Restart npx tauri dev, reconnect, open Worlds → Browse Packs → Resource Packs, search “Prizma Visuals”, and confirm “Prizma Visuals Legacy (Vibrant Visuals Pack Deferred)” appears.
Commit: P14.73: search bedrock texture pack category
Batch: solo


#### P14.74 — Search available Bedrock pack classes

Status: awaiting Cameron verification
Files: crates/msc-infrastructure/src/addon_provider.rs, docs/msc2/rolling-plan.md
What: Fix CurseForge Texture Packs category lookup failing because the API does not return that category under the Bedrock Addons class. Resolve available Addons and Resource Packs classes from the API's class list, search the matching class for the selected filter, and fall back to Addons when the Resource Packs class is absent. The UI's Resource Packs filter uses Minecraft's broad term for what CurseForge labels Texture Packs; a second texture-pack filter would duplicate it.
Evidence: The host returned “CurseForge did not return its Bedrock Texture Packs category” for that category lookup. Removed dependence on that missing category and use the available class IDs. Scoped clippy passes. No tests created or run.
Verify: Restart npx tauri dev, Repair Service, reconnect, then search “Prizma Visuals” in Browse Packs → Resource Packs. Confirm its Legacy project appears.
Commit: P14.74: search available bedrock pack classes
Batch: solo


#### P14.75 — Keep Bedrock search projects without indexed files

Status: awaiting Cameron verification
Files: crates/msc-infrastructure/src/addon_provider.rs, crates/msc-agent/src/routes/worlds.rs, clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte, docs/msc2/rolling-plan.md
What: Keep CurseForge Bedrock projects in search results when search metadata omits latest file indexes; opening the project loads its full version list, where the user can choose an installable file. If class-filtered text search returns no projects, retry the same query without the class filter to account for Bedrock projects missing from CurseForge's API class index. Key result rows by project ID because multiple projects may not have a search file ID.
Evidence: Source inspection found the agent discarded projects without latest file indexes before returning catalog results, even though the project detail endpoint fetches published files independently. Scoped clippy and Svelte type checks pass. No tests created or run.
Verify: Restart npx tauri dev, Repair Service, reconnect, then search “Prizma Visuals” in Browse Packs → Resource Packs. Confirm the project appears, open it, choose the Legacy release, and check whether its files are installable.
Commit: P14.75: keep bedrock projects without indexed files
Batch: solo


#### P14.76 — Search CurseForge's actual Bedrock game catalog

Status: awaiting Cameron verification
Files: crates/msc-infrastructure/src/addon_provider.rs, docs/msc2/rolling-plan.md
What: Correct the Bedrock catalog game ID from Minecraft Java (432) to Minecraft Bedrock (78022). Discover Addons (4984) and Texture Packs (6929) from the Bedrock game's actual class list. Search the selected class without crossing into another class when it is absent. Remove the speculative unfiltered fallback. The earlier P14.73/P14.74 category diagnoses were based on Java's taxonomy and did not explain or resolve the failure; this step supersedes them. Preserve visibility of projects with incomplete search-file metadata from P14.75.
Evidence: Used this host's credential helper to perform read-only official CurseForge API requests without exposing the key. GET /v1/mods/1076812 reports gameId 78022 and classId 6929; GET /v1/games/78022 names Minecraft Bedrock. Java gameId 432 returns no Prizma projects. A temporary command compiled against the updated production libraries called the real curseforge_search_bedrock_packs function: “Prizma Visuals”/resource returned project 1076812 with 30 file indexes; “prizma”/all returned Legacy, PrizmaRTX, and two other projects; “Faithful”/resource returned 20 texture-pack projects. Scoped clippy, formatting, and prepare:agent build passed with the pre-existing unused-mut warning. No tests created or run.
Verify: Restart npx tauri dev, Repair Service to load this agent build, and reconnect. Search “Prizma Visuals” under Resource Packs; the Legacy project must appear. Open it and select its release file. Confirm “prizma” under All also finds PrizmaRTX and a different texture-pack search returns Bedrock projects.
Commit: P14.76: search curseforge bedrock game catalog
Batch: solo


#### P14.77 — Check packs against the installed Bedrock version

Status: awaiting Cameron verification
Files: crates/msc-agent/src/routes/versions.rs, crates/msc-agent/src/routes/worlds.rs, clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte, docs/msc2/rolling-plan.md
What: Fix pack installation reading ConfigServer.minecraft_version, the Java version field, rather than the installed Bedrock distribution. Share the existing Versions endpoint's verified Bedrock version reader, including backend platform selection and the pinned Bedrock fallback. Pass that version to the existing manifest minimum-engine-version check. Replace exact CurseForge tag mismatches' Other version/Install anyway wording with neutral Version not listed and Install labels, and explain that tags do not establish runtime compatibility.
Evidence: A read-only command using the production installed-distribution reader verified this host's Bedrock distribution as 1.26.52.3. Downloaded the creator's PrizmaVisuals 1.3.10.mcpack through the official CurseForge API/CDN and inspected its manifest: module resources, capability pbr, minimum engine 1.16.200, which the installed version exceeds. Agent clippy, prepare:agent build, formatting, frontend type check, and production frontend build pass with existing warnings. No tests created or run; actual client rendering awaits Cameron.
Verify: Restart npx tauri dev, Repair Service, reconnect, stop the Bedrock server, and install Prizma's Legacy release from Browse Packs. Confirm the unknown Bedrock version error is gone, then start and join the world to accept the resource-pack download. Version not listed labels describe only missing exact CurseForge tags.
Commit: P14.77: check packs against installed bedrock version
Batch: solo


#### P14.78 — Read pack requirements from the manifest header

Status: awaiting Cameron verification
Files: crates/msc-domain/src/bedrock.rs, clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte, docs/msc2/rolling-plan.md
What: Fix the shared resource/behavior manifest parser reading min_engine_version from the JSON root instead of header. Track the selected CurseForge file ID so only its release shows Installing; disable other release installs during the operation and guard concurrent requests. Remove repeated exact-tag mismatch badges from search results and releases. Search results offer Versions, while creator version tags and the existing single compatibility explanation remain available in details.
Evidence: Downloaded the real PrizmaVisuals 1.3.10.mcpack through the official CurseForge API/CDN. The corrected production parser reads its header minimum as 1.16.200, pack version as 1.3.0, and resource UUID as 3bcccbf1-ff4d-43a5-8452-12b9534dd70d. This minimum is below the previously verified installed Bedrock version 1.26.52.3. Scoped clippy, formatting, Svelte check (0 errors, 7 existing warnings), frontend production build, and prepare:agent build pass. No tests created or run. Native Minecraft download and rendering await Cameron verification.
Verify: Run npx tauri dev in clients/desktop-web, Repair Service and reconnect to load the new agent, stop the Bedrock server, then install PrizmaVisuals 1.3.10 through Browse Packs → Versions. Confirm only that release shows Installing and the minimum-version error is gone. Start and join the world to check the native resource-pack download and appearance.
Commit: P14.78: read pack requirements from manifest header
Batch: solo

#### P14.79 — Match world packs and backups panel layout

Status: awaiting Cameron verification
Files: clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte, clients/desktop-web/src/lib/sections/worlds/BackupsPanel.svelte, crates/msc-agent/src/routes/worlds.rs, crates/msc-application/src/addons.rs, docs/msc2/rolling-plan.md
What: Put the World Packs title above its panel and move the Backups title above its panel. Give the Bedrock pack panel a top-right Required toggle and Browse Packs action, with a centered empty state when the selected slot has no packs. List installed Bedrock pack names and versions only, with Enable/Disable and confirmed Delete actions; disabled packs show struck-through name and version. Route pack changes through the existing world-profile endpoint and update the saved Bedrock world archive’s activation list and files, with rollback on profile or activation failure and guards for required dependencies. Preserve Java datapack presentation.
Evidence: Scoped Rust clippy, rustfmt, Svelte check (0 errors, 7 existing warnings), frontend production build, and prepare:agent build pass. The changed Rust agent is staged at the Tauri debug agent path. No tests created or run.
Verify: Run npx tauri dev from clients/desktop-web. On the Bedrock Worlds tab, check the headings sit above their panels, the empty pack state fills the panel, and Required/Browse Packs align at its top right. With the server stopped, disable the installed pack and confirm its name/version are struck through; enable it again, then delete it and confirm it disappears. Reinstall it, start the server, and check required-pack joining. Confirm Java datapacks and backups still appear normally.
Commit: P14.79: match world packs and backups panel layout
Batch: solo

#### P14.80 — Align world section controls and summaries

Status: awaiting Cameron verification
Files: clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte, clients/desktop-web/src/lib/sections/worlds/BackupsPanel.svelte, clients/desktop-web/src/lib/api/generated.ts, crates/msc-agent/src/routes/worlds.rs, crates/msc-api/src/dto/worlds.rs, docs/msc2/api-contract/openapi.json, docs/msc2/rolling-plan.md
What: Group each pack row's Enable/Disable and Delete buttons at the right edge. Anchor the pack Required and backup Auto toggles to the same column, keep the backup interval to the toggle's left, and remove the backup size from the actions row. Show pack and backup counts plus aggregate sizes at the left of each panel; report pack bytes from the uncompressed world-archive entries owned by each pack.
Evidence: Rust formatting/clippy pass; API contract generation check passes; Prettier check passes; Svelte check reports 0 errors and 7 existing warnings; production build passes with existing bundler warnings; prepare:agent stages the updated dev agent. No tests created or run.
Verify: Run npx tauri dev from clients/desktop-web. Confirm Enable/Disable sits beside Delete, Required and Auto toggles align with both interval states, and each panel's count and total size match its listed content. Confirm pack totals use unpacked pack files and backup totals include all listed backups, including legacy/unmatched entries.
Commit: P14.80: align world section controls and summaries
Batch: solo

#### P14.81 — Align Required and Auto toggle labels

Status: awaiting Cameron verification
Files: clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte, clients/desktop-web/src/lib/sections/worlds/BackupsPanel.svelte, docs/msc2/rolling-plan.md
What: Widen both fixed toggle columns equally, move their switches left together, and give Required and Auto labels the same gap and left alignment.
Evidence: Prettier check passes; Svelte check reports 0 errors and the same 7 existing warnings. No tests created or run.
Verify: Run npx tauri dev from clients/desktop-web. Confirm Required and Auto switches align, both labels start at the same offset from their switches, and the longer Required label no longer crowds its switch.
Commit: P14.81: align Required and Auto toggle labels
Batch: solo
