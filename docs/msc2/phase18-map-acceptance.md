# Phase 18 — Integrated world map acceptance

**Owner:** Cameron Temple · **Recorded:** 2026-10-02
**Implementation branch:** `feature/world-map-3d`
**Disposition:** P18.0 through P18.11, including corrective substeps, are Done at Cameron's explicit direction. This record closes P18.11's consolidation work. It does not assert completion of deferred physical checks, an independent gate review, or release-artifact acceptance.

## Owner observations

| Area | Recorded result |
|---|---|
| Embedded experience | Owner used the map inside MSC's Worlds tab throughout the integration. Terrain exploration, dimension selection and player controls were observed there. |
| Camera | Owner confirmed the camera was finally fixed after canvas/viewport sizing corrected the Retina-related mismatch (P18.9k). Earlier camera corrections are historical attempts, not remaining separate implementation tasks. |
| Java refresh | Owner placed blocks and saw them after Refresh terrain, initially reporting approximately 30 seconds. Repeat refresh and stopped-server clear-error checks both passed. |
| Live players | Owner confirmed roster/player controls worked; after polling starvation was corrected, Bedrock players stayed live while detailed tile residency settled around 40. No universal sample-latency claim follows from this observation. |
| Bedrock terrain | Owner confirmed Overworld terrain and progressive tiles appeared after export/transport corrections. Detailed tiles are resident near the camera, with lightweight Overworld/End overview coverage beyond them. |
| Dimensions | Owner checked the Bedrock End and reported it good. Nether terrain and Fly work; owner reported the 2D depth slider better. Further Nether gap troubleshooting is deferred. |
| Block appearance | Owner reports textures and block shapes look good and considers that work already completed. Prior standalone proofs covered glass panes and other non-cube shapes. This is representative acceptance, not every block or custom asset. |
| UI | Terrain status and Players have disclosure controls; the dimension selector now uses the shared Settings component. Screenshot shows the themed selector in use; no additional exhaustive UI acceptance is inferred. |

## Runtime and asset coverage

Java proof records cover legacy and current save layouts: Purpur 1.21.11, ATM10 Lite 1.21.1, Paper/Fabric/NeoForge 26.2 and Vanilla/Forge 26.3. Representative terrain exports and standalone player-feed proofs remain documented in P18.3–P18.6. Embedded Vanilla terrain refresh/player behavior and the mature BDS world were observed by Cameron. This does not mean every runtime received a separate full in-window acceptance pass; those repetitive checks were deferred by the owner.

Standard saved Overworld, Nether and End terrain is supported through the respective Java and Bedrock pipelines. ATM10 custom dimension discovery remains part of the design. Its recorded custom folders had no saved region chunks, so custom-dimension visual proof remains deferred until suitable terrain exists. Empty/discovered dimensions do not establish rendered terrain.

Unknown or unavailable block assets may use visible fallbacks. Representative successful models/textures do not prove universal mod/add-on compatibility. The implementation transports map resources through the authenticated Worlds capability; the standalone browser viewer is development proof, not a supported browser management client.

## Freshness, costs and resource bounds

Terrain represents a saved snapshot; moving the MSC camera does not generate Minecraft chunks. Refresh captures a new consistent save. Java uses the save boundary; BDS uses hold/query/copy/resume. Player samples have a separate clock and do not make terrain live. Repeat/no-edit tile reuse is implemented, but a separate quantitative integrated reuse acceptance has not been recorded.

Recorded measurements are local diagnostic evidence, not performance guarantees:

- Java proof: Purpur copied 13,047,634 / 14,733,720 bytes with 203 / 1,210 ms save pauses and changed tile ready in 928 ms. Vanilla copied about 23 MB with 265/783 ms save pauses and a changed tile ready in 3,254 ms.
- Bedrock export correction: one developed tile improved from 29.91 seconds to 2.394 seconds with the same output bytes. Later owner observations reported tiles arriving around 5–10 seconds; timing varies with content and workload.
- Bedrock overview profile: 2,327 Overworld detailed references and 27 overview files totaling 571,131 bytes; catalog export 12.06 seconds. Nether has 171 detailed references and intentionally no roof-only overview; End profile had 76 detailed references and six overview files.
- Bedrock detail configuration: one concurrent request, 512 MiB tile byte budget and maximum 120 resident tiles. Owner observed roughly 37–43 detailed tiles. These are residency limits, not measured total RSS, and do not load all 2,327 detailed tiles simultaneously. Overviews provide distant coverage where appropriate.
- Snapshot copy is bounded at 2 GiB and 30 seconds. Full-world copying still occurs; reuse reduces tile regeneration. No exhaustive integrated CPU, RSS, server tick impact, or cross-platform performance acceptance is recorded.

## Deferred findings and limits

1. **Nether saved-data gaps:** independent inspection found absent chunk records inside completed tiles. At camera X381/Z-137, chunk (23,-9) has no saved Nether records in the inspected current snapshot, and nearby chunks are also absent. Earlier chunk (9,-15) was absent even in an independent scan of all physical database tables; all 73 snapshot immutable tables matched the live source. This explains the inspected map holes but does not establish why source records are absent. Owner will visit/generate/save the area in Minecraft and refresh later. The holes are not claimed fixed.
2. **ATM10 custom dimensions:** visual proof awaits generated and saved custom terrain. Universal mod assets/versions remain unclaimed.
3. **Runtime/platform breadth:** additional consolidated runtime and exact release-artifact checks are deferred. Existing proof results must not be represented as every platform/runtime passing the complete integrated flow.
4. **Control breadth:** camera/navigation and Nether depth are observed. Lighting, quality and biome controls were explicitly deferred in P18.5; this record does not claim they were subsequently delivered. No separate exhaustive acceptance of every exit/error path or all freshness targets is recorded here.
5. **Independent review:** the agent that implemented this phase cannot provide its independent gate review. That review remains outstanding, with these named limits available for disposition.

## Proposed phase gate disposition

Implementation steps are Done by owner instruction. The representative Worlds-tab terrain, refresh and live-player experience is owner-accepted as recorded above. P18.11 is complete because coverage, costs, unsupported states and deferred checkpoints are now explicit. The independent reviewer must assess the selected runtime matrix and original UX promises against this evidence; it must not infer universal compatibility, measured memory compliance, or completion of deferred checks from Done status alone. Phase 18 remains on its isolated feature branch until integration/release is separately directed.
