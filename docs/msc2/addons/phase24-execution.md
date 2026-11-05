# Phase 24 — Autonomous execution and review stops

**Owner authorization:** 2026-11-05 request to revise the plan for autonomous
looping, stopping for review of new sheets/views. This file prepares that
workflow; feature implementation has not started.

## Plan review findings and corrections

1. **Solo handoffs prevented autonomy.** Replace 24 conversation boundaries with
   continuous execution and explicit interface checkpoints. Pending routine owner
   verification does not block dependent code work. Final acceptance remains
   Cameron's responsibility.
2. **Export plumbing followed early UI work.** Complete backend publication,
   ZIP/.mrpack, native transfer and CLI foundations before building the first
   new desktop surface. UI reviews must exercise working behavior, not mockups.
3. **Screenshots blocked unrelated work.** Move P24.20 after terminal and Storage
   integration; record missing images without fabricating them. Earlier exports
   can use truthful text instructions in development; final acceptance requires
   the illustrated guides. Pin guide versions in immutable publications.
4. **New retained roots lacked early Storage protection.** Ownership, references,
   staging and rollback protection are requirements of P24.2; P24.23 adds the
   Storage presentation and reviewed cleanup. Never leave new roots generically
   deletable until the final integration step.
5. **Broad backend steps could hide unfinished adapters.** Each slice wires its
   actual DTOs/routes/generated clients and checks permissions and capabilities.
   Provider pack acquisition is a server-side catalog-to-staged-archive path
   shared by creation, existing-server replacement/update and terminal clients;
   it must not depend on a fake browser upload or stub endpoint. P24.3 supplies
   acquisition, P24.6 import retention, P24.9/10 update/replacement, P24.12 creation.
6. **A build did not specify resumable progress.** Keep implementation status,
   owner acceptance, UI approval and next action separately in the ledger below.
   A continuation can recover from repository state without asking which step
   comes next or repeating completed work.

This is a review of plan feasibility, not an independent Phase 24 gate review.
No product requirement was dropped to reduce handoffs.

## Execution order and dependencies

Keep stable step numbers and one final commit per step:

| Order | Steps | Result and boundary |
|---|---|---|
| A | P24.1–P24.10 | Integration contract, durable collections, provider behavior, retention, recovery, updates/replacement |
| A continued | P24.15–P24.18, then P24.21 | Publication, ZIP, .mrpack, saving, scriptable CLI; continue without routine owner handoff |
| B | P24.11 | Existing-server provider discovery; stop for any new sheet/view |
| B continued | P24.12 | Creation pack browsing and compatibility choices; runnable UI review |
| B continued | P24.13 | World catalog counterpart; stop if a new surface is introduced |
| B continued | P24.14 | Player management and recovery/update/replacement reviews; review each new surface before proceeding |
| B continued | P24.19 | Publish/export sheet with real local/remote behavior; runnable UI review |
| B continued | P24.22 | TUI counterparts; stop for each new full-screen view/dialog |
| B continued | P24.23 | Storage integration; stop if it adds a new view/sheet |
| C | P24.20 | Offline guides; request missing owner screenshots together, review resulting HTML/images |
| D | P24.24 | Consolidated owner acceptance packet and outstanding platform evidence; no agent closure |

Backend pack/content review APIs are not owner UI review stops by themselves.
Changes to existing components do not automatically create new review stops;
a new mode with substantially different interaction (e.g. player management or
creation compatibility review) counts as a new view even in an existing file.
Do not rename new views as existing components to bypass review. P24.1 records
the expected surface list; add unexpected ones to the ledger as discovered.

## Work loop

After the owner requests execution:

1. Read required repo instructions and current policy, this protocol, scope,
   rolling statuses, ledger and Git status. Preserve unrelated changes. Check
   actual files and commits against the recorded next action before editing.
2. If waiting for UI review, check for explicit owner approval of that candidate.
   Without it, report the pending review and stop. An automatic continuation,
   “keep working” loop prompt or elapsed time does not approve a surface.
3. Select the first unimplemented step in the order above. No new prompt needed.
   Make ordinary technical decisions autonomously within the agreed contract.
   Record consequential choices and evidence; ask only about material product
   scope changes, unavailable prerequisites or permissions that tools require.
4. Implement the complete slice and essential error/recovery behavior. No silent
   placeholders, success responses without effects or claims of verified
   compatibility. Stage new features behind truthful capabilities when needed.
5. Run relevant permitted non-test checks; Rust formatting and focused Clippy
   remain required. Use the narrow affected packages, warm build cache and one
   frontend type-check per changed slice. An already-passing equivalent stronger
   check can satisfy a Verify command if documented; do not repeat identical
   builds on unchanged files. No broad test/CI/release runs. Never treat a
   composite script as safe without checking whether it invokes tests.
6. A local check failure stays within this step: identify the cause, repair it
   and recheck the affected code. Do not advance/commit failed work or rerun
   unchanged failures. If the issue requires owner input or lies outside the
   authorized scope, record the evidence and stop. This same-step repair rule
   supersedes the normal batch rule's immediate handoff on the first failure.
7. For a new sheet/view, prepare a runnable candidate before stopping. Use the
   normal development launch path, ensure required endpoints exist and provide
   exact navigation, what to inspect, a concrete example and known limitations.
   Screenshots/previews help but do not replace the runnable behavior. Use
   disposable development data; no unrequested changes to real servers.
8. Record the pending surface, candidate revision and next action. A multi-view
   step can pause with uncommitted changes; preserve them and identify the base
   commit plus changed files. Do not manufacture multiple commits for one step.
   Commit a UI step only after its new surfaces have been reviewed and any
   requested changes resolved. This preserves one commit without rewriting
   published history or adding separate checkpoint commits.
9. On explicit approval, record what was approved and any requested changes.
   Implement review changes, recheck, present materially changed surfaces again,
   then commit the completed step with its rolling-plan/ledger update. Set
   `implemented — awaiting owner verification`, never done. A visual approval
   does not close unrelated behavior/platform acceptance. Continue immediately
   to the next eligible step in the same execution request.
10. Routine completed steps also commit once and continue immediately. Update
    progress with useful findings, without per-step permission questions. If a
    context/runtime limit interrupts mid-step, record exact files, checks and
    next action; the next loop resumes that work rather than starting over.

Continue independent work around unavailable API access/screenshots only within
this dependency order: unavailable live-provider verification can remain pending
when controlled local inspection/build work is meaningful, but do not claim it
passed. If implementation itself cannot proceed, stop at that prerequisite.
At a UI review stop do not proceed to later implementation behind the owner's
back. A continuation that encounters the same stop performs no repeated builds,
no new edits, and no busy polling. The external loop must wait for owner input.

## Durable execution ledger

Update this section in the same step commit; for a mid-step review/interruption,
write the working-tree checkpoint before yielding. Do not create housekeeping
commits per turn. Rolling-plan statuses and Git history remain the step record.

| Field | Current value |
|---|---|
| Run state | executing — P24.15 implemented; autonomous goal active |
| Current step | P24.16 |
| Next action | Implement streaming Player/Server ZIP export from frozen publications with manifest, truthful frozen instruction assets and no runtime/world/private content. Then P24.17 .mrpack, P24.18 saving and P24.21 CLI before the first UI checkpoint. Continue without routine handoff. |
| Last implementation commit | P24.15: publish immutable dated content revisions |
| Pending UI review | none |
| Candidate base commit / changed files | P24.15 committed; next slice P24.16 |
| Owner approval evidence | none |
| Checks and outcomes | P24.15 focused agent Clippy passed; API generation/frontend check passed (10 existing CSS warnings); schema references resolve. Three essential publication safety tests added, not run. Text-v1 guides explicitly lack owner screenshots; P24.20 acceptance remains open. |
| External prerequisites | owner screenshots for P24.20; usable provider key/content for live acceptance |
| Deferred owner acceptance | all physical feature/platform scenarios |

For each UI checkpoint add a row here, including navigation and approval evidence:

| Surface | Step | Candidate and navigation | Status | Owner response |
|---|---|---|---|---|
| Expected new surfaces inventoried in P24.1 | P24.1 | not implemented | pending inventory | none |

Approval evidence records the owner's actual response and date, linked to the
candidate; never infer approval from unrelated messages. Keep requested fixes
and unreviewed new surfaces visible.

## Start/resume instruction

The same instruction can be used once to start and for any later continuation:

> Execute Phase 24 using docs/msc2/addons/phase24-execution.md and the agreed
> scope. Resume from its ledger and repository evidence. Continue through
> routine steps without asking me to name the next step. Keep one commit per
> step and owner verification pending. Stop with a runnable candidate for each
> new sheet/view and wait for my explicit review; do not cross an unapproved
> checkpoint. Run permitted non-test checks only. Do not run tests, change
> release workflows, publish releases or mark owner acceptance complete.

This is an execution prompt, not an instruction to install an unattended daemon
or weaken tool approvals. An external continuation loop should halt on pending
UI review/prerequisites and restart after the owner's response. Runtime/budget
limits still apply; the ledger supplies resumption, not unlimited execution.
For a goal-based runner, use “implement through the next owner review checkpoint”
as the current goal's finish line; record the pending review and hand off there.
After approval, resume the phase with the next checkpoint as the finish line.
Do not set an unlimited “finish the phase” goal that repeatedly wakes at review
stops or treats them as approval. Final implementation handoff is not phase-gate
acceptance: Cameron verifies and the other agent reviews when requested.
