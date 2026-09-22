# Phase 15 — World packs and modpack identity

**Step:** P15.23 · **Status:** D-030 confirmed; acceptance map awaiting verification · **Date:** 2026-09-21

This is the contract and acceptance map for P15.24–P15.29. It records the
Phase 15 scope approved in Cameron's 2026-09-20 recap. The detailed world-slot
ownership rule is **Approved** under D-030. The remaining world-pack
implementation contract stays **Proposed** pending acceptance evidence and
separate review.

## Ownership and API boundary

| Data or action | Owner | API boundary | Source areas | Acceptance evidence |
|---|---|---|---|---|
| Java datapack files and state | Selected Java world slot | Agent Worlds API, addressed by slot ID; exact route and DTO are frozen in P15.25–P15.26 | `crates/msc-domain/src/world.rs`, `world_profile.rs`; `crates/msc-infrastructure/src/world_store.rs`; `crates/msc-application/src/worlds.rs`, `addons.rs`; `crates/msc-agent/src/routes/worlds.rs`; `crates/msc-api/src/dto/worlds.rs` | Install into slot A; verify it is absent from B; activate, duplicate, back up, restore, export, and import with the slot while preserving provider, version, checksum, compatibility, and enabled state. Reject malformed and path-traversing archives and incompatible Java metadata before mutation. |
| Bedrock behavior-pack files and state | Selected Bedrock world slot in MSC, even if BDS supports shared folders | Agent Worlds API, addressed by slot ID; Bedrock manifest and dependency results remain edition-specific | `crates/msc-domain/src/bedrock.rs`, `addon_dependency.rs`, `world.rs`; `crates/msc-application/src/addons.rs`, `addon_dependencies.rs`, `bedrock_service.rs`, `worlds.rs`; `crates/msc-infrastructure/src/addon_provider.rs`, `world_store.rs`; `crates/msc-agent/src/routes/bedrock.rs`, `worlds.rs`; `crates/msc-api/src/dto/worlds.rs`, `addons.rs` | Install into one slot and prove isolation and portability through activation, duplication, backup, restore, export, and import. Validate manifest identity, UUIDs, versions, minimum Bedrock version, and dependencies. If a linked resource pack is bundled, install it with the behavior pack; if missing, report the dependency and leave the world unchanged. |
| Imported modpack identity | Host agent's server metadata | Server metadata read model, visible on the Components surface; exact route and DTO are frozen in P15.24 | `crates/msc-domain/src/modpack.rs`, `app_config_schema.rs`; `crates/msc-infrastructure/src/config_repository.rs`; `crates/msc-application/src/modpacks.rs`, `provisioning.rs`; `crates/msc-api/src/dto/lifecycle.rs`; `crates/msc-agent/src/routes/servers.rs`, `components.rs` | Create or import from a named pack and verify name, provider, and imported version survive ordinary server operations and agent restart. Missing source values display as unavailable. Ordinary servers have no identity. Installed component names never produce an inferred identity. |
| Pack and identity presentation | Each connected client renders agent-owned state | Existing Worlds and Components surfaces consume the agent API; clients do not own files or decide identity | `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, `components/ComponentsSection.svelte`, corresponding `model.ts` files, generated API types | Java and Bedrock show their edition-specific world-slot organization. The selected slot controls the displayed pack list. The Components summary is compact and read-only and does not duplicate the inventory. |
| Backup and transfer portability | Host agent; backup/transfer operations | Agent backups and world import/export preserve slot-associated files and metadata | `crates/msc-domain/src/backup.rs`; `crates/msc-application/src/backups.rs`, `worlds.rs`, `transfer.rs`; `crates/msc-infrastructure/src/backup_store.rs`, `world_store.rs`; `crates/msc-api/src/dto/backups.rs`; `crates/msc-agent/src/routes/backups.rs`, `worlds.rs` | Round-trip a slot through each named lifecycle operation and compare slot association and pack metadata. Existing server-wide backup behavior remains intact. |

P15.25 uses `GET` and `POST /v1/worlds/{slotId}/profile` for the first
pack-record API. The profile's `packs` array carries edition, pack kind,
provider/source, version, checksum, compatibility, enabled state, relative
world paths, and dependencies. Java records use `java_datapack`; Bedrock
records use `bedrock_behavior_pack`. The agent rejects a record from the wrong
edition and file paths that can leave the selected world's root. Pack files
remain in the archived Java or Bedrock world folder. Export archives carry the
raw world profile in a reserved metadata entry that is not extracted into the
Minecraft world; backup sidecars carry the same profile. The existing slot
profile copy path carries records through activation and duplication.

## Contract decisions

- **D-030 is Approved.** Cameron confirmed that each slot owns a versioned
  world profile. The remaining pack-provider and lifecycle details in this
  map stay Proposed pending acceptance evidence and separate review.
- **Java provider:** Modrinth is the first datapack catalog. Provider records
  retain project/source, version, and checksum when available.
- **Bedrock dependency:** install a linked resource pack only when it is
  included in the downloaded add-on. Otherwise fail before world mutation with
  an actionable explanation. Do not add a separate resource-pack browser.
- **Identity:** record the pack name, provider, and imported version as
  server-owned metadata. Missing values stay missing; the installed component
  inventory is not an identity source.
- **Out of scope:** Java shader packs, Java client resource packs, Bedrock
  resource-pack browsing, skin packs, and client-only mods.
- **First usable release:** browsing, installation, listing, compatibility,
  enabled state, source, and dependency information are in scope. Enable,
  disable, update, and remove remain later controls unless Cameron's acceptance
  review finds one necessary for initial use. Pack installation still reports
  enabled state and update availability when the provider makes them known.

## Acceptance sequence

1. Confirm the world-profile and pack ownership boundary before implementation.
2. Confirm that imported modpack metadata comes from import/create inputs and
   remains server-owned, with no inference from installed components.
3. Confirm Java and Bedrock pack isolation between slots and preservation across
   activation, duplication, backup, restore, export, and import.
4. Confirm archive and manifest validation, including rejection before
   mutation; confirm the bundled and missing Bedrock linked-resource-pack
   cases.
5. Confirm Java Modrinth discovery, compatibility state, and the edition-
   specific Worlds views plus the read-only Components summary.
6. Cameron runs live Minecraft and backup/transfer checks against a Java server
   and a supported Bedrock runtime. Static inspection and cargo checks do not
   replace this owner-run evidence.

The requirement-to-source crosswalk and remaining owner evidence are reviewed
again in P15.29. This map does not close Phase 15 or its release gate.
