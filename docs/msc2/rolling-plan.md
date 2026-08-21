# MSC 2 — Rolling Plan

> ## STATUS: Phase 7 is closed on exact candidate `79d5044`. Phase 8 has not been planned yet.
> **Next move:** PLAN — read the vision, port plan, current rolling plan, Phase 7 handoff/audit notes, and the MSC 1 oracle; then write Phase 8's complete step list here. No Phase 8 code in the PLAN move.
> **Repo:** https://github.com/ctemple9/msc2 · Phase 7 candidate branch `phase7-corrections` at `79d5044`; GitHub Actions run [32448912726](https://github.com/ctemple9/msc2/actions/runs/32448912726) is fully green across repo invariants, macOS, Linux, Windows, both Phase 6/7 smokes, and the headless no-GUI link check.
> **Last updated:** 2026-08-21

**Previous phases (Setup, Phase 0 through Phase 7) and their amendments have moved to `rolling-plan-archive.md`** to keep this file small. That archive is historical only — current status, active work, and every amendment from Phase 8 onward stay here.

---


## How this document works

This is the **working state** of the build. The vision documents say where we're going; the port plan says in what order; this file says **where we actually are right now**.

Phases are fixed and come from `msc2-port-plan.md`. **Steps are written one phase at a time**, as we reach each phase — not up front. Steps written today for Phase 8 would be guesses.

Each phase runs the six-move loop in `CLAUDE.md`: Plan → Read → Execute → Verify → Review → Advance.

### Step format

Every step looks like this:

```
### P0.3 — Extract TPS parser fixtures
**Status:** not started | in progress | awaiting verification | DONE
**Files:** fixtures/tps/, tools/extract-fixtures/
**What:** Pull the 27 TPS test cases out of MSC 1's TpsMonitoringTests.swift
         into input/expected JSON pairs.
**Verify:** `ls fixtures/tps/*.json | wc -l` → 27
**Commit:** P0.3: extract TPS parser fixtures        <- the message, not a hash
```

Every step also carries a **Batch:** field, telling an agent whether it may be run unattended:

| Batch value | Meaning |
|---|---|
| `safe` | Mechanical, and its Verify is a script Cameron has already reviewed. Batch freely. |
| `stop-after` | Runnable in a batch, but the batch **ends here** — the result needs looking at before continuing. |
| `solo` | Judgment work or a new checker script. Run it alone. Its output needs a cross-check by the other agent before the phase closes. |

**Status is only moved to DONE by Cameron**, after he runs the Verify command himself. An agent may set it to *awaiting verification* and stop.

**A step whose Verify only counts things is `stop-after` at best.** Counting proves something exists, not that it is right.

---

## Phases

Gates are in `msc2-port-plan.md`. This is the map, not the detail.

| Phase | Name | State |
|---|---|---|
| **Setup** | Repo, docs, agent instructions, CI, editor config | complete |
| **0** | Freeze the baseline and build the harness | complete |
| 1 | Domain types and pure rules | complete |
| 2 | API contract and operation model | complete |
| 3 | Safety substrate | complete |
| 4 | Java lifecycle vertical slice | complete |
| 5 | Configuration and migration | complete |
| **6** | Worlds and backups | complete |
| **7** | Server families and provisioning | complete |
| **8** | Mods, plugins, modpacks | **not started — PLAN next** |
| 9 | Networking and helpers | not started |
| 10 | Bedrock runtimes | not started |
| 11 | Desktop and web clients | not started |
| 12 | Terminal UI (deferred from v1) | not started |

---

## Phase 6 — Worlds and backups

All 51 steps (P6.1–P6.51) are `DONE`, and Codex's gate review (2026-08-18) confirms the gate holds on exact candidate `8568dea`. The full record — scope, characterization, world/backup services, public-client wiring, gate-review corrections, exact-commit tri-platform CI proof, and the review itself — has moved to `rolling-plan-archive.md`.

---

## Phase 7 — Server families and provisioning

All 38 steps (P7.1–P7.38) are `DONE` per Cameron. The final exact candidate is `79d5044` on `phase7-corrections`; GitHub Actions run [32448912726](https://github.com/ctemple9/msc2/actions/runs/32448912726) is fully green across macOS, Linux, Windows, the Phase 7 provisioning/launch smoke, and the headless no-GUI check. The full record — scope, characterization, six-family providers and provisioning, runtime and installer behavior, diagnostics and repairs, review corrections, evidence, and amendments — has moved to `rolling-plan-archive.md`.

---

## Phase 8 — Mods, plugins, modpacks

Not planned yet. The next conversation is the Phase 8 **PLAN** move: write the complete step list here with all five required fields (`Status`, `Files`, `What`, `Verify`, `Batch`) and no implementation.

---

## Amendments log

Every amendment from Phase 8 onward is recorded here. Earlier phases' amendments are in `rolling-plan-archive.md`.
