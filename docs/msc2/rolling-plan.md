# MSC 2 — Rolling Plan

> ## STATUS: Phases 14 and 15 are complete and archived. The external static-review record is preserved in the archive. All verification entries are recorded DONE, with P15.69 retaining its accepted failed-verification result.
> **Next move:** No implementation phase remains open in the rolling plan. The external review findings remain available in `rolling-plan-archive.md` for future triage. The current workspace has an unrelated pre-existing `dead_code` failure in `crates/msc-application/tests/provisioning.rs:152`. Phase 12 visual parity, anti-slop review, release/update handoff, and Bedrock product acceptance are recorded complete on 2026-09-08. P12.121–P12.189 are archived below with all verification entries recorded as DONE. The planned Phase 13 full-screen terminal client remains retired by D-034.

The detailed Phase 12 working plan is preserved in `rolling-plan-archive.md` under “Reconciliation snapshot — 2026-09-08”. This file contains only the current status and next move.

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
