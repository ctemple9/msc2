# MSC 2 — Agent Instructions

> **This file is duplicated as `CLAUDE.md` for Claude Code. If you change one, change both.**

## What this repo is

MSC 2 is a cross-platform Minecraft server manager: a **Rust agent** that runs as a background service, plus clients (web/desktop, iOS, CLI) that talk to it over an API. It is a behavior-preserving port of MSC 1 (a macOS SwiftUI app), not a blank-slate rewrite.

**Owner:** Cameron Temple. He is not a Rust developer yet and is learning as this is built. Explain what you're doing in plain language. He runs every verification himself.

## Read before doing anything

| File | What it is |
|---|---|
| `docs/msc2/MSC2-VISION.md` | **Start here.** Index, precedence rules, what's owner-approved vs merely proposed |
| `docs/msc2/msc2-decisions.md` | Numbered decisions with reasoning. **The authority.** |
| `docs/msc2/msc2-engineering.md` | Architecture, API contract, platform matrices |
| `docs/msc2/msc2-product.md` | What MSC 2 is, in plain language |
| `docs/msc2/msc2-port-plan.md` | The phases and their **exit gates** |
| `docs/msc2/rolling-plan.md` | **Current state.** Which phase, which step, what's done |

`docs/msc2/audit/` holds the MSC 1 analysis — including two per-file inventory CSVs used during extraction.

## The loop

Work proceeds one phase at a time. Each phase runs through six moves, and **each move is a separate conversation.**

| # | Move | Who | What happens |
|---|---|---|---|
| 1 | **Plan** | An agent | Reads vision + port plan + rolling plan. Writes the step list for this phase into `rolling-plan.md`. **Writes no code.** |
| 2 | **Read** | Cameron | Reviews the plan before anything is built |
| 3 | **Execute** | An agent | One step (or small batch) per conversation. Does the work, commits. |
| 4 | **Verify** | Cameron | Runs each step's `Verify:` command himself |
| 5 | **Review** | **The other agent** | Checks the phase **gate**, not whether steps were followed. May amend earlier phases. |
| 6 | **Advance** | — | Status line updated, next phase begins |

Two agents work on this repo: **Claude Code** and **Codex**. Whoever implements a phase does not review it.

## Hard rules

1. **Do only the move you were asked to do.** Planning means planning — no code. Reviewing means reporting — no fixes.
2. **Every step produces a commit.** Message starts with the step number: `P0.3: extract TPS fixtures`. If there's no commit, the step didn't happen.
3. **Every step has a `Verify:` line** — one command Cameron can run to confirm it worked. If you can't write one, the step is too vague; split it.
4. **Never mark a step done yourself.** Report the verify command and stop. Cameron closes it.
5. **Don't skip ahead.** Don't start the next step, don't fix unrelated things you notice. Note them at the end of your response instead.
6. **Approved beats Proposed.** Never change something marked **Approved** in `msc2-decisions.md` without asking. If you disagree with a **Proposed** item, say so and record it — don't silently override.
7. **A phase ends when its gate holds**, not when its steps are ticked. Gates are in `msc2-port-plan.md`.
8. **MSC 1 is the oracle.** It lives at `~/Documents/Swift Projects/minecraft-server-controller` and must never be modified. Read it freely; write to it never.
9. **If something contradicts the vision, stop and say so.** Don't build around it quietly.

## Conventions

- Rust: `cargo fmt` and `cargo clippy` clean before any commit
- Tests run with `cargo nextest run`
- Commit messages: `P<phase>.<step>: <what changed>` — imperative, lowercase
- No `Co-Authored-By` or other AI attribution trailers. Enforced by `.githooks/commit-msg` and by CI, not by memory. Enable the hook once per clone: `git config core.hooksPath .githooks`
- Comments explain **why**, not what

## Current state

See the status line at the top of `docs/msc2/rolling-plan.md`.
