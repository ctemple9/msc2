# MSC 2 — Rolling Plan

**Current priority:** Close the remaining Phase 16 release gate and owner findings below. The Phase 22 TUI was accepted by Cameron on 2026-10-30; its implementation history is in the [archive](rolling-plan-archive.md#phase-22--restored-full-screen-terminal-client). Rust workspace warning cleanup is complete; cross-platform physical checks are separate from TUI acceptance.

## Phases

This table is the quick map; detailed steps and historical findings are linked from the [rolling plan archive](rolling-plan-archive.md). Phase gates and scope are defined in the [port plan](msc2-port-plan.md).

| Phase | Name | Current disposition |
|---|---|---|
| Setup | Repository and project documents | Complete |
| 0 | Baseline and harness | Complete |
| 1 | Domain types and pure rules | Complete |
| 2 | API contract and operation model | Complete |
| 3 | Safety substrate | Complete |
| 4 | Java lifecycle | Complete |
| 5 | Configuration and migration | Complete |
| 6 | Worlds and backups | Implemented; historical records archived |
| 7 | Server families and provisioning | Implemented; historical records archived |
| 8 | Mods, plugins and modpacks | Implemented; historical records archived |
| 9 | Networking and helpers | Implemented; historical records archived |
| 10 | Bedrock runtimes | Implemented; historical records archived |
| 11 | Desktop clients | Implemented; historical records archived |
| 12 | Client redesign | Complete |
| 13 | Original terminal UI | Retired by D-034 |
| 14 | Server time and scheduling | Historical work archived; see phase records for unresolved acceptance |
| 15 | Server state and interaction fixes | Historical work archived |
| 16 | Release safety and codebase readiness | **Open:** exact-artifact acceptance and owner findings below |
| 17 | Local CLI refinement | Historical work archived; physical acceptance is tracked in its phase record |
| 18 | 3D world map | Implemented; owner map acceptance remains open |
| 19 | Complete local uninstall | Implementation records archived |
| 20 | Faster native file imports and uploads | Implementation records archived |
| 21 | Storage inventory and management | Implementation records archived |
| 22 | Restored full-screen TUI | Accepted by Cameron 2026-10-30; implementation records archived |

## Open work at a glance

| Item | State | Next action |
|---|---|---|
| Phase 16 release gate | Open. Its exact-artifact packet is for v0.1.18; v0.1.38 is now published, so the old packet is historical evidence rather than acceptance of the current release. | Refresh the acceptance candidate and identity from the current intended release, then complete the required artifact and platform observations. See [Phase 16 acceptance](release/phase16-acceptance.md). |
| Xbox Broadcast sharing (Finding 5) | Agent-owned JAR reuse is implemented. P16.134 sign-in reuse was owner-confirmed on Test 2. Agent-wide account migration details and conditional Components visibility remain open; concurrent multi-server sign-in is not verified. | Finish remaining Components/account scope; verify a clean install, existing sign-in migration, later enablement, and concurrent enabled servers. See [archived details](rolling-plan-archive.md#archived-owner-testing-records--0132). |
| Add Server success page (Finding 9) | The success branch exists in the UI code, but Cameron saw a blank page. The cause is unknown and no fix is recorded. | Reproduce with a current build, trace why the success body is absent while Done appears, then verify Java and Bedrock creation paths. See the archive. |
| Bedrock map coverage (Finding 15) | Gaps are present in converted input; whether they originate in the snapshot or conversion remains unknown. Cache-complete status does not establish nearby coverage. | Compare a fresh consistent snapshot and converted regions at the same coordinates; correct conversion or coverage messaging from that evidence. See Finding 15 and P16.97 in the archive. |
| Java Initiate lifecycle (Finding 26) | P16.131–P16.132 did not solve it; P16.133–P16.134 concern Broadcast sign-in. A later retest found duplicate Stop can cause `stopping -> stopping`; current-code diagnosis and retest remain open. | Trace current start/stop requests and operation logs, fix the confirmed duplicate-stop path if still present, then verify on Intel. See Finding 26 and P16.131–P16.134 in the archive. |
| World pack cards (P16.135) | Implementation committed; owner visual verification and the recorded frontend check are pending. | Verify Bedrock pack and Java datapack cards and actions on the current desktop build. Run the recorded `npm run check` only when explicitly requested. |
| Discard hidden setup (P16.136) | Implementation prepared; owner verification pending. | Verify that discarding preserves an existing world and that a fresh setup can restart from its first screen. |
| Intel Bedrock decoder (P18.99) | Implemented; same-world pre-render verification pending. | Complete the recorded same-world Intel check. |

**Watch, no current code action:** Playit setup failed once with helper exit code 1, then worked after the failed Playit agent was deleted. The cause was not established. If it recurs, capture redacted helper output before changing code. See archived Finding 10.

**Phase 22 disposition:** Cameron accepted the current TUI on 2026-10-30. Detailed P22 records and their original verification commands are in the archive.

## Archived owner-testing details

Detailed records for Findings 5, 9, 10, 15 and 26, plus P16.97, P16.101, and P16.130–P16.136, are in the [rolling plan archive](rolling-plan-archive.md#archived-owner-testing-records--0132). Their outstanding state remains summarized in the open-work table above; moving the details does not close those items.
