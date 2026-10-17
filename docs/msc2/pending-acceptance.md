# Pending acceptance and known follow-ups

This index preserves work that must not be confused with the active implementation
queue. Archived implementation is not automatic physical acceptance. Independent
phase review remains with the other agent under AGENTS.md. Do not reopen or run
tests, destructive checks or release workflows from this index without Cameron's
instruction. Full evidence and limitations remain in the linked records.

| Item | Evidence and remaining work | Record |
|---|---|---|
| Phase 20 native imports | P20.1–P20.10 closed and archived by Cameron on October 17. Owner accepts the behavior after the final installation-text correction. Independent gate review remains pending. Before/after, direct-copy/network and full peak-memory comparisons were not supplied; unavailable platforms, older-agent and fault exercises are not claimed passed. Earlier source-change refusal and local HTTP-versus-handoff selection remain recorded follow-ups. The P20.9 candidate predates P20.10 wording. | [Acceptance and observations](uploads/acceptance.md), [measurement record](uploads/baseline.md), [archived steps](rolling-plan-archive.md#phase-20--faster-native-file-imports-and-uploads), [gate](msc2-port-plan.md#phase-20--faster-native-file-imports-and-uploads-owner-requested-plan-october-16) |
| XboxBroadcast P16.89 | Cameron confirmed the temporary Intel correction works with RakNet retained. Normal-release artifacts, other platforms and cancellation fault cases remain separate acceptance. | [Archived P16.89](rolling-plan-archive.md#p1689--keep-xboxbroadcast-running-through-bedrock-status-polling), [delivery instructions](clients/broadcast-agent-fix.md) |
| Release build reuse/cache P16.88 | Implemented; build-time saving remains unmeasured. The next release uses it. No cache-seeding run is implied. | [Archived P16.88](rolling-plan-archive.md#p1688--reuse-release-compilation-and-cache-rust-dependencies) |
| Phase 16 physical gate | The v0.1.18 exact-artifact record and independent gate review remain open. Later releases being published does not close that gate. | [Phase 16 acceptance](release/phase16-acceptance.md), [port plan](msc2-port-plan.md) |
| Phase 17 CLI | Step records closed by owner direction; physical acceptance and independent review remain outstanding. | [CLI acceptance guidance](clients/phase17-cli.md), [archive](rolling-plan-archive.md#phase-17--local-cli-refinement) |
| Phase 18 map | Step records closed by owner direction; named runtime/platform observations and independent review remain deferred. | [Map acceptance](phase18-map-acceptance.md) |
| Windows MSI | P16.44–P16.52 implementation records are archived. Exact candidate acceptance remains required; the old P16.43 planning header was stale. | [Archive](rolling-plan-archive.md) |
| Complete uninstall | P19.1–P19.5 implementations are archived. Destructive physical acceptance must use owner-selected disposable environments. | [Archive](rolling-plan-archive.md#p191--inventory-local-installations-and-define-the-deletion-boundary) |
| Pairing, management stalls and shutdown | P16.80/P16.82/P16.84/P16.85 implementations are recorded; each retains its own live acceptance limitations. | [Archive](rolling-plan-archive.md), [v0.1.29 notes](release/v0.1.29.md) |
| World time and case-alias world swapping | Existing unresolved observations; explicitly deferred by the owner and not claimed fixed by the broadcast correction. | [v0.1.29 notes](release/v0.1.29.md) |
| Existing broad test-target compilation errors | Unused provisioning test helper, cli_pairing route imports and stale cli_service CommonArgs literals were found during P16.89. Unrelated to the fix and not changed. | [Archived P16.89](rolling-plan-archive.md#p1689--keep-xboxbroadcast-running-through-bedrock-status-polling) |

Published release records v0.1.19–v0.1.29 now live in the archive, with publication
reconciled against GitHub. Historical failed/superseded candidates stay historical;
owner artifact acceptance is not inferred from publication.
