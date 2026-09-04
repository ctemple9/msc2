# MSC 2 — Agent Instructions

> **This file is duplicated as `AGENTS.md` for Codex. If you change one, change both.**

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
| `docs/msc2/antiAIslop.md` | **Design law.** Anti-AI-slop guiding principle — **required reading before any design, styling, or frontend work** |

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

## Modes

Every prompt names one of these. Do only the named mode.

| Mode | You do |
|---|---|
| **PLAN** | Write the step list into `rolling-plan.md`. No code. Stop. **Every step must carry all five fields: Status, Files, What, Verify, Batch.** A step without a `Batch:` value is incomplete — a later batch run has no way to know where to stop. |
| **EXECUTE** | One step. Work, verify, commit, stop. |
| **BATCH EXECUTE** | A named range of steps. Same rules per step, run in order. **Run each step's own Verify yourself before moving on. If one fails, STOP** — do not work around it. Never batch past the range given. |
| **REVIEW** | Check the phase gate. Report only, fix nothing. |
| **CROSS-CHECK** | Audit another agent's work against MSC 1 source. Report only. |

## Asking Cameron a question

Cameron wrote MSC 1 — roughly 97,000 lines of Swift — so he reads Swift comfortably and works in Python. Rust is new to him, and he is learning it deliberately as this project is built.

He owns the product and every decision. What he should not have to do is reconstruct your analysis to make a call. **Never hand him a bare list of symbol names and ask which bucket they belong in** — that is asking him to redo work you were assigned. Do the reasoning, state a recommendation, and ask the question that's actually left.

**When code genuinely is the answer, show it and teach it.** A short explanation of what an unfamiliar Rust construct does, and why it's written that way, is wanted — not noise. Prefer a five-line excerpt with two sentences of explanation over a file path he has to go read. He is learning this language on purpose; treat every explanation as part of the deliverable.

Each question gets this shape, in plain language:

```
QUESTION n — <plain-English title>

What it is:      what this code does, in Minecraft/product terms
The choice:      option A vs option B, in one sentence each
Why it matters:  what changes downstream depending on the answer
If unsure:       your recommendation and why
```

Rules:
- No jargon without a plain-English gloss on first use.
- Never say "needs your call" without saying what the call changes.
- If a question doesn't affect anything Cameron would notice, decide it yourself, record the reasoning, and list it under "decided for you" instead of asking.
- Group related questions. Thirteen questions is usually three questions.

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
10. **No AI names in repo-visible identifiers.** Do not create filenames, directory names, branch names, tags, artifact names, or other repo-visible identifiers containing assistant/vendor/product names such as Codex, Claude, ChatGPT, or OpenAI. The required instruction filename `CLAUDE.md` is the only filename exception unless Cameron explicitly approves another one.
11. **No signs of AI slop.** The MSC 2 redesign must not look vibe-coded, generic, or like any other app — every visual decision is deliberate and specific to MSC. Before any design, styling, or frontend work, read `docs/msc2/antiAIslop.md` and hold every screen to its checklist. Owner-approved guiding principle; as binding as the rest of this list.
12. **Never run tests.** Do not run any test command, including `cargo nextest`, `cargo test`, Vitest, Playwright, Xcode tests, or equivalent, unless Cameron explicitly instructs you to run that specific test.
13. **Do not create tests.** Never add a new test unless Cameron explicitly requests it or gives approval after you explain why it is necessary.

## Conventions

- Rust: `cargo fmt` and `cargo clippy` clean before any commit
- Tests are run only when Cameron explicitly requests a specific test command.
- **No test verification by default.** Do not run a test as part of implementation or verification. Use inspection, formatting checks, type-checks, builds, static validators, or Cameron's own manual verification instead. A step's declared Verify command does not override Cameron's explicit approval requirement.
- Commit messages: `P<phase>.<step>: <what changed>` — imperative, lowercase
- **One commit per step. Never two.** That single commit contains the work *and* the `rolling-plan.md` status update. Do not add a follow-up commit to record the hash — you cannot know a hash before committing, and the step number in the message is already the link (`git log --grep="P0.1"`). Leave the `Commit:` field as the message subject, not a hash.
- No `Co-Authored-By` or other AI attribution trailers. Enforced by `.githooks/commit-msg` and by CI, not by memory. Enable the hook once per clone: `git config core.hooksPath .githooks`
- Comments explain **why**, not what

## Current state

See the status line at the top of `docs/msc2/rolling-plan.md`.
