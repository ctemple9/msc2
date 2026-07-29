# MSC 2 — Rolling Plan

> ## STATUS: Phase 0 — not yet planned
> **Next move:** PLAN (Phase 0)
> **Last updated:** 2026-07-29

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
**Commit:** (filled in by the executing agent)
```

**Status is only moved to DONE by Cameron**, after he runs the Verify command himself. An agent may set it to *awaiting verification* and stop.

---

## Phases

Gates are in `msc2-port-plan.md`. This is the map, not the detail.

| Phase | Name | State |
|---|---|---|
| **Setup** | Repo, docs, agent instructions, CI skeleton | in progress |
| **0** | Freeze the baseline and build the harness | **next** |
| 1 | Domain types and pure rules | not started |
| 2 | API contract and operation model | not started |
| 3 | Safety substrate | not started |
| 4 | Java lifecycle vertical slice | not started |
| 5 | Configuration and migration | not started |
| 6 | Worlds and backups | not started |
| 7 | Server families and provisioning | not started |
| 8 | Mods, plugins, modpacks | not started |
| 9 | Networking and helpers | not started |
| 10 | Bedrock runtimes | not started |
| 11 | Desktop and web clients | not started |
| 12 | Terminal UI (deferred from v1) | not started |

---

## Setup

### S.1 — Create the repository and land the documents
**Status:** awaiting verification
**Files:** everything
**What:** `git init`, vision docs into `docs/msc2/`, audit artifacts into `docs/msc2/audit/`, `CLAUDE.md` + `AGENTS.md`, this file, README, `.gitignore`.
**Verify:** `cd ~/msc2 && ls docs/msc2/ && git log --oneline` → five vision docs present, one commit exists
**Commit:** _(pending)_

### S.2 — Publish to GitHub
**Status:** not started
**What:** Create the public `msc2` repository and push `main`.
**Verify:** the repo loads in a browser and shows the README
**Commit:** _(n/a — push only)_

### S.3 — CI skeleton
**Status:** not started
**What:** A GitHub Actions workflow that runs on macOS, Linux, and Windows. It has nothing to build yet — it exists so the three-platform matrix is proven before there's code depending on it.
**Verify:** green check on the commit in GitHub's Actions tab, all three platforms
**Commit:** _(pending)_

---

## Phase 0 — Freeze the baseline and build the harness

**Gate:** a fixture can be written, run, and compared. The API baseline exists. The symbol ledger exists.

_Steps not yet written. Run the PLAN move._

---

## Amendments log

When a review amends an earlier phase or a decision, record it here so the change isn't silent.

_(none yet)_
