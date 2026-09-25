# MSC 2 — Rolling Plan

> ## STATUS: Phases 14 and 15 are complete and archived; Linux desktop service elevation corrections P14.38–P14.39 await owner verification. P14.40 is blocked on a Linux-only compile error fixed in P14.41; the v0.1.11 release will be retried from that correction. The external static-review record is preserved in the archive. All prior verification entries are recorded DONE, with P15.69 retaining its accepted failed-verification result.
> **Next move:** Verify and commit P14.41, then wait for the retriggered P14.40 release workflow and Linux RPM. Cameron verifies P14.38–P14.41 and closes the steps. The external review findings remain available in `rolling-plan-archive.md` for future triage. The current workspace has an unrelated pre-existing `dead_code` failure in `crates/msc-application/tests/provisioning.rs:152`. Phase 12 visual parity, anti-slop review, release/update handoff, and Bedrock product acceptance are recorded complete on 2026-09-08. P12.121–P12.189 are archived below with all verification entries recorded as DONE. The planned Phase 13 full-screen terminal client remains retired by D-034.

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

Status: awaiting retry after P14.41 compile correction
Files: coordinated version manifests and lockfiles, clients/desktop-web/src/lib/bundle-identity.ts, README.md, docs/msc2/rolling-plan.md
What: Bump the coordinated version from 0.1.10 to 0.1.11, update the download instructions, push the release preparation to main, and push tag v0.1.11 to trigger the GitHub release workflow. The initial Linux workflow exposed a CLI error-conversion compile failure, corrected in P14.41; retry the tag after that correction and wait for the Linux RPM to appear. The first agent installation will be Cameron's install from that release. If release signing or the workflow blocks publication, stop and report the blocker.
Verify: gh release view v0.1.11 --json url,isPrerelease,assets
Commit: P14.40: publish Linux desktop service elevation fix
Batch: solo

#### P14.41 — Fix Linux helper CLI error conversion

Status: in progress
Files: crates/msc-agent/src/main.rs, docs/msc2/rolling-plan.md
What: Convert Linux service helper errors to their display text before wrapping them in the CLI's internal error type, so the Linux-only desktop helper compiles for release builds.
Verify: cargo fmt --all -- --check && cargo check -p msc-platform-linux -p msc-agent
Commit: P14.41: fix Linux helper CLI error conversion
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
