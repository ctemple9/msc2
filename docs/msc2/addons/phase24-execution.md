# Phase 24 — Autonomous execution and review stops

**Owner authorization:** 2026-11-05 request to revise the plan for autonomous
looping, stopping for review of new sheets/views. This file prepares that
workflow. Current implementation and review state are recorded in the ledger.

## Owner amendment — final UI review

On 2026-11-05 Cameron instructed: “ill check the ui work at the end. finsh the phase without stopping”. This supersedes the intermediate UI approval stops below, including the pending manual-download surface. Continue all authorized Phase 24 implementation in the stated order, one commit per completed step. Record every new surface for the final owner review instead of waiting between surfaces. No tests, releases, destructive live changes or owner/independent phase-gate closure are authorized by this amendment. Missing owner screenshots remain an honest prerequisite gap, not permission to fabricate images.

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
| Run state | Active — intermediate UI reviews deferred to final handoff by explicit owner amendment |
| Current step | P24.22 |
| Next action | P24.22: supported host-local TUI counterparts for catalog packs, both providers, player management, recovery/update/replacement and publication/export; then P24.23 Storage, P24.20 guides and P24.24 acceptance packet. Intermediate UI reviews remain deferred. |
| Last implementation commit | P24.19: add the publish and export review sheet |
| Pending UI review | Deferred to final handoff: conditional manual-download exception and all subsequent new surfaces. Prior discovery/project-detail approvals remain recorded; no ordinary installation confirmation sheet exists. |
| Candidate base commit / changed files | P24.11 commit contains the shared catalog/project browser, direct installation/manual-file handling, source/update sheet and supporting agent contract/download repairs. New P24.12 work follows this commit. |
| Owner approval evidence | Cameron: “ui passes” for the revised CurseForge discovery browser, after requesting missing-key-only key entry/console links. Subsequent goal resume authorizes continuation, not approval of future install surfaces. Owner later requested compact CurseForge details to match Modrinth; density revised without treating the feedback as installation-sheet approval. Cameron then clarified both providers must use the same actual sheet; both now use ProjectDetailSheet with provider data adapted separately, and the duplicate CurseForge detail layout/CSS is removed. Cameron then said “ui passes” for that shared-sheet refactor. This approves project details. Cameron then explicitly rejected a routine CurseForge installation sheet because Modrinth has none; compatible Add/Install and explicit Install anyway now run directly. Backend preflight/review remains automatic; only required manual-file handling interrupts. This supersedes the prior routine-sheet review stop. |
| Checks and outcomes | Direct-install frontend check/build passed. Owner-reported 302 download failure traced to redirects disabled in artifact acquisition; bounded validated CDN handoff added, API-key header limited to edge.forgecdn.net. Focused agent Clippy and native desktop check passed; new essential pure redirect-boundary test source added (<1ms expected), not run. Updated agent build/development staging succeeded. Cameron subsequently reported “nice it worked,” confirming the reported CurseForge Add/302 case. No tests or assistant-run live installations were performed; remaining installation scenarios and conditional manual-download UI review remain pending. |
| External prerequisites | updated running agent for incompatible-release review (bundled binary ready; owner Repair agent action); owner screenshots for P24.20; usable provider key/content for live acceptance |
| Deferred owner acceptance | all physical feature/platform scenarios |

For each UI checkpoint add a row here, including navigation and approval evidence:

| Surface | Step | Candidate and navigation | Status | Owner response |
|---|---|---|---|---|
| CurseForge discovery mode | P24.11 | a14ff1cc + revised working tree; Components → Browse mods/plugins → either provider → project | approved — discovery and shared project details | Cameron: “ui passes” after replacing both provider detail layouts with the one shared ProjectDetailSheet |
| Routine CurseForge installation confirmation | P24.11 | removed | cancelled by owner direction; no approval needed | Cameron: “there doesnt need to be a sheet, there isnt one for modrinth” |
| Content sources and updates | P24.11 | Components → add-on menu → Content sources and updates; exact matching/linking, hints, checksum-bound update review and apply | implemented — final review pending | owner deferred UI review to phase end |
| Creation pack browser and setup review | P24.12 | Add Server → Add-ons → Browse modpacks; exact release → Review modpack setup, keep/remove staged content, matching or current setup | implemented — final review pending | owner deferred UI review to phase end |
| Java world datapacks and modified-content restore | P24.13 | Worlds → selected slot → Datapacks → CurseForge; shared details, manual exact ZIP, or update modified/unknown contents | implemented — final review pending | owner deferred UI review to phase end |
| Player content and pack recovery/update/replacement | P24.14 | Components → Player content; independent player files/classification and Recover/update/replace pack | implemented — final review pending | owner deferred UI review to phase end |
| Dated publication and export | P24.19 | Components → Export content; audience/history/current review → publish → ZIP/.mrpack and local/host destination | implemented — final review pending | owner deferred UI review to phase end |
| Required manual-download exception | P24.11 | Add/Install a release whose author requires a browser download; only then show ContentManualDownloadSheet | conditional exception only; awaiting owner review when exercised | none |

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
