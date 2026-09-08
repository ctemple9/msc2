# MSC 2 — Rolling Plan

> ## STATUS: Phase 12 client redesign and its post-phase corrections are complete. The planned Phase 13 full-screen terminal client is retired by D-034.
> **Next move:** Cameron reviews this reconciled plan and advances the repository to the next product phase. All prior Phase 12 verification entries are recorded as DONE in the archive. P12.121 is the only new reconciliation step awaiting verification.

Previous phases and completed work remain in `rolling-plan-archive.md`. The archive is historical; this file contains only the current state and the next move.

---

## How this document works

This is the working state of the build. The vision documents say where MSC 2 is going; the port plan says the intended sequence; this file says where the repository actually is now.

Phases come from `msc2-port-plan.md`. Steps are written as work arrives rather than being invented in advance. Each step has a status, file scope, description, verification command, commit subject, and batch classification.

**Status is only moved to DONE by Cameron after he runs the Verify command.** An agent may set a step to awaiting verification and stop.

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

The pre-reconciliation Phase 12 working plan is preserved in `rolling-plan-archive.md` under “Reconciliation snapshot — 2026-09-07”. Its original step text and amendment history remain available there, with verified entries normalized to DONE per Cameron’s confirmation. The original file is also recoverable from commit `c00964d`.

### P12.121 — Reconcile the rolling plan
**Status:** DONE
**Files:** `docs/msc2/rolling-plan.md`, `docs/msc2/rolling-plan-archive.md`
**What:** Reduce the active rolling plan to one truthful current status and one unambiguous next step. Preserve the pre-reconciliation Phase 12 plan in the archive, normalize all previously verified entries to DONE, retain the retired Phase 13 and client-retirement history, and leave no product source or generated artifact changed.
**Verify:** `git diff --check && test "$(grep -c '^### P12\\.' docs/msc2/rolling-plan.md)" -eq "$(grep '^### P12\\.' docs/msc2/rolling-plan.md | sed -E 's/^### ([^ ]+).*/\\1/' | sort -u | wc -l)" && grep -q 'Reconciliation snapshot — 2026-09-07' docs/msc2/rolling-plan-archive.md && test "$(grep -c '^\\*\\*Status:\\*\\* awaiting verification' docs/msc2/rolling-plan.md)" -eq 1`
**Commit:** `P12.121: reconcile the rolling plan`
**Batch:** solo
