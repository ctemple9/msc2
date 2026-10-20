# MSC 2 — Rolling Plan

**Current priority:** P18.69 — smooth Bedrock map movement while saved terrain continues loading.

### P18.69 — Defer detailed Bedrock tile swaps during camera movement

**Status:** Implemented; awaiting Cameron's verification.
**Files:** `crates/msc-agent/src/routes/worlds/map_terrain.rs`; `crates/msc-agent/src/routes/worlds/map_terrain/bedrock.rs`; `clients/desktop-web/src/lib/sections/worlds/WorldMapViewer.svelte`; this plan.
**What:** Bedrock camera updates run every two seconds and previously downloaded and parsed the complete manifest on the UI thread while also triggering a detailed tile replacement. The agent now answers camera updates with a small status message, keeps discovering saved terrain around the camera, and waits until camera movement has settled before replacing preview tiles with detailed meshes. Position, rotation, and zoom all count as movement, covering 2D, 3D, and Fly. Older agents continue to receive the original manifest request. The renderer still receives its normal manifest updates and can stream newly discovered previews. No test was added or run; the stutter requires owner verification in the actual large Bedrock world.
**Verify:** Open `~/Desktop/msc2-bedrock-map-intel-p18.69.dmg`, install the app, repair the agent, and reopen the imported Bedtonic-world map. Move continuously in 2D, 3D, and Fly while preview terrain loads; motion should remain smooth, new saved areas should still appear, and detailed textures should fill in after stopping for a few seconds. Compare against a Java map to confirm Java behavior is unchanged.
**Batch:** P18.69 only.
**Commit:** `P18.69: defer bedrock map details while moving`

Cameron directed the implemented Add Server, Bedrock map, imported-JAR and Services follow-ups to Done and into [the rolling-plan archive](rolling-plan-archive.md#october-19-2026--v0131-follow-up-step-closure). Failed intermediate Bedrock attempts retain their supersession history there. Independent phase reviews and remaining physical checks stay in [pending acceptance](pending-acceptance.md).

Previously accepted Phase 20 and Storage steps remain in the archive. This cleanup does not close a phase gate or establish exact release-artifact acceptance.
