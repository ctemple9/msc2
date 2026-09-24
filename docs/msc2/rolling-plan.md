# MSC 2 — Rolling Plan

> ## STATUS: Phases 14 and 15 are complete and archived. The external static-review record is preserved in the archive. All verification entries are recorded DONE, with P15.69 retaining its accepted failed-verification result.
> **Next move:** No implementation phase remains open in the rolling plan. The external review findings remain available in `rolling-plan-archive.md` for future triage. The current workspace has an unrelated pre-existing `dead_code` failure in `crates/msc-application/tests/provisioning.rs:152`. Phase 12 visual parity, anti-slop review, release/update handoff, and Bedrock product acceptance are recorded complete on 2026-09-08. P12.121–P12.189 are archived below with all verification entries recorded as DONE. The planned Phase 13 full-screen terminal client remains retired by D-034.

The detailed Phase 12 working plan is preserved in `rolling-plan-archive.md` under “Reconciliation snapshot — 2026-09-08”. This file contains only the current status and next move.

## Current maintenance step

### P15.84 — Repair CI after Phase 15 transport merge

- **Status:** IN PROGRESS
- **Files:** `crates/msc-agent/src/routes/bedrock_runtime.rs`, `crates/msc-agent/src/routes/servers.rs`, `crates/msc-agent/src/routes/worlds.rs`, existing Bedrock request fixtures, five formatted desktop-web files, `docs/msc2/client-capability-matrix.csv`, `docs/msc2/rolling-plan.md`
- **What:** Restore the CI contract after the Bedrock transport-choice merge. Keep production eligibility detection visible to the Phase 10 wiring guard, update existing request fixtures and route fixtures for the transport and pending-modpack arguments, accept the already-defined unresolved-modpack upload purpose in the world route, format the five client files reported by validation, and register the seven Phase 15 operations that were added to the OpenAPI contract but omitted from the client capability matrix. No new tests are added and no runtime behavior is changed beyond wiring the already-required inputs.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the pushed P15.84 commit's CI run is green
- **Batch:** N12 — CI regression repair
- **Commit:** `P15.84: repair CI after Phase 15 transport merge`

### P15.85 — Accommodate the stable Clippy response-size lint

- **Status:** IN PROGRESS
- **Files:** `crates/msc-agent/src/routes/commands.rs`, `docs/msc2/rolling-plan.md`
- **What:** Keep the relative-time query helper’s existing Axum response contract while explicitly documenting the same `result_large_err` allowance already used by neighboring route helpers. GitHub’s current stable Rust toolchain promotes this advisory to `-D warnings`; the local toolchain does not yet emit it. No runtime behavior changes.
- **Verify:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && gh run list --workflow ci.yml --limit 5`
- **Batch:** N12 — CI regression repair
- **Commit:** `P15.85: allow large relative-time error response`

### P15.86 — Refresh regression fixtures for current contracts

- **Status:** IN PROGRESS
- **Files:** `fixtures/bedrock-sidecar/start-round-trip.json`, `fixtures/bedrock-sidecar/start-out-of-order-rejected.json`, `crates/msc-application/tests/curseforge_pack_import.rs`, `crates/msc-application/tests/world_slot_crud.rs`, `docs/msc2/rolling-plan.md`
- **What:** Update existing regression fixtures and fake transports for the already-shipped transport-aware sidecar frames and CurseForge bulk project lookup, and change the existing world export assertion to verify the current profile-carrying archive contract instead of expecting an unchanged byte-for-byte copy. No new tests are added and no production behavior changes.
- **Verify:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings -A dead-code -A unused-mut -A clippy::needless-return -A clippy::collapsible-if -A clippy::derivable-impls -A clippy::useless-format && gh run list --workflow ci.yml --limit 5`
- **Batch:** N12 — CI regression repair
- **Commit:** `P15.86: refresh regression fixtures for current contracts`

### P15.87 — Prepare the v0.1.9 release

- **Status:** IN PROGRESS
- **Files:** `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `clients/desktop-web/src/lib/bundle-identity.ts`, `clients/desktop-web/src/lib/bundle-identity.test.ts`, `README.md`, `docs/msc2/rolling-plan.md`
- **What:** Synchronize the coordinated agent, desktop shell, Tauri, lockfile, bundle identity, and download instructions to v0.1.9 so the release workflow accepts the matching v0.1.9 tag. This is a normal unsigned beta prerelease; no signing-key rotation is part of this release.
- **Verify:** `python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard && rg -n '0\.1\.9|v0\.1\.9' crates/msc-agent/Cargo.toml Cargo.lock clients/desktop-web/package.json clients/desktop-web/package-lock.json clients/desktop-web/src-tauri/Cargo.toml clients/desktop-web/src-tauri/Cargo.lock clients/desktop-web/src-tauri/tauri.conf.json clients/desktop-web/src/lib/bundle-identity.ts README.md && gh run list --workflow release.yml --limit 3`
- **Batch:** N13 — release preparation
- **Commit:** `P15.87: prepare v0.1.9 release`

---

## How this document works

This is the working state of the build. The vision documents say where MSC 2 is going; the port plan says the intended sequence; this file says where the repository actually is now.

Phases come from `msc2-port-plan.md`. Steps are written as work arrives rather than being invented in advance. Each step has a status, file scope, description, verification command, commit subject, and batch classification.

Phase 12 is complete. Phases 14 and 15 are complete and archived.

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
