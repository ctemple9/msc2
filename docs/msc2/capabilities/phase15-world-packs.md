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

P15.26 adds `GET /v1/catalog/datapacks` and
`POST /v1/worlds/{slotId}/datapacks/install`. Search uses Modrinth's
`datapack` project category and the selected Minecraft version, without Java
loader facets. Installation resolves an immutable version ID, checks the
provider's Minecraft-version list and SHA-512 when published, validates ZIP
paths and `pack.mcmeta`, and refuses changes while the selected active world
is running. The prior slot archive is preserved as a recovery copy before
replacement. The updated slot profile records source, version, compatibility,
enabled state, file paths, and SHA-512. Update availability is not reported
until a provider check supplies it.

P15.27's **Proposed** provider choice is CurseForge's Minecraft Bedrock
catalog because Modrinth's first-provider decision applies only to Java
datapacks. The agent
looks up the Bedrock `Addons` class through CurseForge's categories endpoint, searches
that class, and returns immutable project/file IDs plus the candidate game
version. It uses the existing host-side CurseForge API key; the key never
reaches a client. `GET /v1/catalog/behaviorpacks` accepts `q`, `gameVersion`,
and `offset`; `POST /v1/worlds/{slotId}/behaviorpacks/install` accepts the
selected project and file IDs. The agent verifies that the file belongs to
the selected project, resolves its official download URL, and accepts payloads
only from CurseForge's ForgeCDN.

P15.32 corrected the CurseForge class lookup route. P15.34 corrects its game
identifier to Minecraft's CurseForge game ID, `432`, in both category lookup
and search; the former `1303` value caused the category request to return 404.
P15.33 labels failures from the category lookup separately from failures in
the subsequent Bedrock add-on search, so provider status errors identify which
request failed.

Before replacing the selected world's archive, installation validates every
archive path and symbolic-link flag, caps expanded content, requires a valid
behavior-pack manifest and UUID, and checks pack versions and the minimum
Bedrock version. Any UUID dependency not provided by another bundled behavior
pack or linked resource pack stops installation before the world changes. A
bundled resource pack is copied into that same slot and added to the slot's
`world_resource_packs.json`; behavior packs are added to
`world_behavior_packs.json`. A recovery copy of the prior world archive is
made before replacement, and the selected slot profile records the behavior
pack source, immutable CurseForge file ID, checksum, compatibility, paths, and
dependencies. CurseForge files with no API download URL remain unavailable for
one-click installation and receive a clear manual-download response.

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

## P15.29 static review — 2026-09-21

The implementation was inspected against the acceptance sequence. These are
static findings; no live Minecraft, Bedrock, backup, or transfer scenario was
run.

### Findings

| Severity | Finding | Evidence and effect |
|---|---|---|
| **Resolved by static re-review — backup profile restore** | The initial review incorrectly concluded that restore ignored the saved world profile. | `crates/msc-agent/src/routes/backups.rs` reads `world_profile` from the backup sidecar and writes it to the active slot after `restore_backup` succeeds. If that metadata write fails, the operation reports `metadata_restore_failed` with an explicit message that the world itself has already been restored. The live backup/restore round trip remains owner verification. |
| **Addressed in P15.30 — error handling** | Copying a slot into an existing slot previously ignored metadata and profile-copy failures. | The application now retains the old archive under a unique recovery name while installing the replacement and atomically saving the updated slot metadata with the source profile. A metadata-write failure removes the replacement and restores the prior archive before returning the write error. If rollback itself fails, the returned error identifies the retained recovery archive. Cameron should verify the successful copy path and review the reported rollback boundary. |

### Static checks that passed by inspection

- World profiles contain edition-specific pack records, including provider,
  version, checksum, compatibility, enabled state, relative files, and
  dependencies. They are stored per slot; duplicate and activation paths copy
  the profile with the slot.
- Export adds the profile as a reserved archive entry. Import reads that entry
  when present, and archive extraction skips it so it does not enter the
  Minecraft world. Backup creation writes the profile into its sidecar.
- Java datapack installation validates archive entry paths, links, expansion
  size, `pack.mcmeta`, and compatibility before replacing the world archive.
- Bedrock installation validates nested archive paths, manifests, UUIDs,
  versions, minimum Bedrock version, and dependencies before replacing the
  world archive. A missing linked resource pack returns an error before
  mutation; a bundled one is installed into the selected slot.
- Modpack identity is captured from the imported manifest and saved in
  server-owned configuration. Plain JAR ZIPs have no identity, and there is
  no component-list inference path.
- Worlds displays the selected slot's edition-matching pack list and its
  source, version, dependency, compatibility, checksum, and enabled state.
  Components displays a compact read-only identity summary with unavailable
  values made explicit.
- P15.31 aligns the world-pack browser with the Components browser's search,
  provider/version line, flat icon-led results, and compact Add action while
  retaining Java and Bedrock-specific install requests.

### Remaining owner-run acceptance evidence

Run these against a Java server and a supported Bedrock runtime before closing
the Phase 15 gate:

1. Install packs in slot A and verify slot B remains unchanged. Activate and
   duplicate A, then confirm each slot has the expected files and profile.
2. Back up a slot, change its pack state, restore the backup, and compare both
   the world files and displayed profile. Confirm the successful restore path
   applies the profile from the backup sidecar.
3. Export a packed slot, import it as a new slot, activate it, and confirm the
   reserved profile metadata never appears as a Minecraft world file.
4. Try malformed, path-traversing, and incompatible Java and Bedrock archives;
   confirm rejection leaves the selected world unchanged.
5. For Bedrock, verify both a bundled linked resource pack and an add-on with
   a missing linked resource pack; the latter must leave the world unchanged.
6. Create/import a named modpack, restart the agent, and confirm its source
   identity remains accurate; confirm an ordinary server has no identity.
7. Walk through the Java and Bedrock Worlds views and Components summary with
   realistic pack names and dependency lists, checking that the information is
   readable at the supported desktop window size.

P15.29 records the portability findings; it does not resolve them or close the
phase gate.

### Bedrock catalog detail view — P15.36

The behavior-pack browser uses CurseForge's mod, description, and file
resources through `GET /v1/catalog/behaviorpacks/{projectId}`. The response
keeps third-party HTML as data; the client renders only a small safe formatting
subset, alongside the project's screenshots and up to 50 published files.
Version rows show release status and whether a file's Minecraft version tags
match the selected server version. The selected file ID is the one sent to the
existing slot-local install route. The Modrinth Java detail flow is unchanged.
The result list searches the full CurseForge catalog without a Minecraft
version filter, so packs for other versions remain available to inspect. A
result marked “Other version” opens its details instead of offering direct Add;
the detail list keeps its explicit “Install anyway” action. If the selected
server version cannot be read, the list marks versions “Version unknown” and
withholds direct installation.

### P15.30 failure boundary

`copy_slot_into_existing` stages the incoming archive, moves an existing
destination archive to a unique rollback path, then installs the staged archive
and writes the updated slot metadata plus source profile in one atomic metadata
write. If that write fails, the new archive is removed and the prior archive
is restored before the error is returned. If the archive rollback itself
fails, the error includes the retained rollback path so the previous world
data remains recoverable. A successful operation removes the rollback archive;
failure to clean up that extra file does not invalidate the committed copy.
