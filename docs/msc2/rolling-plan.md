# MSC 2 — Rolling Plan

**Current priority:** Resolve the open owner-testing findings below and finish the remaining Phase 16 acceptance checks. Phase 22's TUI was accepted by Cameron on 2026-10-30; its implementation records are in the [archive](rolling-plan-archive.md#phase-22--restored-full-screen-terminal-client). A clean Rust workspace build was verified in P22.70; that record is archived too. Preserve the distinction between implemented changes, owner verification, and phase acceptance.

## Open work at a glance

| Area | Current state | Next evidence needed |
|---|---|---|
| Phase 16 release acceptance | Gate remains open; see [phase acceptance record](release/phase16-acceptance.md). | Complete the required exact-artifact and platform checks recorded there. |
| Xbox Broadcast shared account and Components visibility (Finding 5) | Shared JAR behavior partly implemented; account sharing, migration, and conditional visibility remain open. | See the archived Finding 5 and P16.101 records; shared JAR is implemented, while account migration and Components visibility remain open. |
| Add Server success page (Finding 9) | Visible success body defect is diagnosed but its cause/fix remains open. | See archived Finding 9; reproduce on the current desktop build and verify guidance across create paths. |
| Playit helper failure (Finding 10) | Could not reproduce; retry worked after deleting the failed Playit agent. | See archived Finding 10; if it recurs, capture redacted helper output. No code change is currently justified. |
| Bedrock saved map coverage (Finding 15) | Sparse converted input observed; source snapshot versus conversion omission unresolved. | See archived Finding 15 and P16.97; compare a fresh snapshot with converted regions and correct coverage reporting. |
| Java Initiate repeated-start failure (Finding 26) | P16.131–P16.134 attempts did not close the issue; retest exposed duplicate-stop behavior. | See archived Finding 26 and P16.131–P16.134; inspect current code/logs, establish a reproducible lifecycle path, then verify on Intel. |
| P16.135 world pack cards | Implementation committed; owner verification pending. | See archived P16.135; implementation awaits owner UI verification and its recorded check. |
| P16.136 discard hidden setup | Implementation prepared; owner verification pending. | See archived P16.136; verify preserve-world and fresh-world scenarios. |
| P18.99 Intel Bedrock decoder | Implemented; same-world pre-render verification pending. | Complete the recorded Intel-host pre-render check. |

**Phase 22 disposition:** Cameron accepted the current TUI on 2026-10-30. The archived records retain their original implementation and verification details; no further TUI work is implied by the open items above.

## Archived owner-testing details

Detailed records for Findings 5, 9, 10, 15 and 26, plus P16.97, P16.101, and P16.130–P16.136, are in the [rolling plan archive](rolling-plan-archive.md#archived-owner-testing-records--0132). Their outstanding state remains summarized in the open-work table above; moving the details does not close those items.
