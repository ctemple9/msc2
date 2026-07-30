# MSC 2 — Rolling Plan

> ## STATUS: Setup complete (S.1–S.4 verified) — Phase 0 not yet planned
> **Next move:** PLAN (Phase 0)
> **Repo:** https://github.com/ctemple9/msc2 · CI green on macOS, Linux, Windows
> **Last updated:** 2026-07-30

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
| **Setup** | Repo, docs, agent instructions, CI, editor config | complete |
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
**Verify:** `cd ~/msc2 && ls docs/msc2/ && git log --oneline` → five vision docs + rolling-plan present, commits exist
**Commit:** `e0771ed`

### S.2 — Publish to GitHub
**Status:** awaiting verification
**What:** Created the public `msc2` repository and pushed `main`.
**Verify:** open https://github.com/ctemple9/msc2 — README renders, 19 files, docs/msc2/ browsable
**Commit:** _(n/a — push only)_

### S.3 — CI skeleton
**Status:** awaiting verification
**What:** `.github/workflows/ci.yml`. Two jobs — `repo-invariants` (CLAUDE.md/AGENTS.md must not drift; all six controlled documents must exist) and `toolchain` (macOS + Linux + Windows, installs Rust, builds once `Cargo.toml` appears).
**Verify:** `cd ~/msc2 && gh run list --limit 1` → shows `success`. Or the green check at https://github.com/ctemple9/msc2/actions
**Commit:** `S.3` — all four jobs passed on first run

### S.4 — Shared VS Code configuration
**Status:** awaiting verification
**Files:** `.vscode/extensions.json`, `.vscode/settings.json`
**What:** Extension recommendations (rust-analyzer, TOML) so the workspace configures itself on open. Whitespace/final-newline hygiene to keep diffs clean, markdown wrapping, Rust format-on-save so `cargo fmt --check` never fails in CI for an avoidable reason.
**Note:** the rust-analyzer extension ships no prebuilt language server for x86_64 macOS. Resolved by `rustup component add rust-analyzer` plus `"rust-analyzer.server.path": "rust-analyzer"` — portable via the rustup PATH shim, not a hard-coded home directory.
**Verify:** open `~/msc2` in VS Code, reload the window — no rust-analyzer error in the notifications
**Commit:** `S.4` (two commits)

---

## Phase 0 — Freeze the baseline and build the harness

**Gate:** a fixture can be written, run, and compared. The API baseline exists. The symbol ledger exists.

_Steps not yet written. Run the PLAN move._

---

## Amendments log

When a review amends an earlier phase or a decision, record it here so the change isn't silent.

_(none yet)_
