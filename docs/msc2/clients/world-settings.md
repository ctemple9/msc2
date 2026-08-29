# World-settings release boundary

P12.29 closes the native world-settings work for MSC 2's current release
scope. It is a bounded profile for Minecraft 1.20 and newer, not an attempt to
model every setting a server, plugin, or mod might expose.

## Ownership contract

Every world slot carries one versioned `WorldProfile`. The selected profile is
the source of truth for that world's identity, seed, generation choices,
difficulty, default game mode, gamerules, and the edition-specific values that
the selected runtime advertises. Activation projects that profile onto the
runtime and reports what was applied.

Server settings remain common to every slot: ports, player limits, MOTD,
authentication, allowlist/ops, runtime and process policy, view/simulation
distance, cross-play, broadcast, and tunnels. `force-gamemode` is also
server-owned. It is off by default and, when enabled deliberately, can
override every world's saved default game mode; it is never enabled as a side
effect of choosing Creative for one world.

The shared `WorldSettingsForm` is used for both the Create Server wizard's
first-world path and later Worlds-tab create/edit. Essentials appear first;
World Generation and Gameplay Rules are neutral disclosures. Creation-only
values become read-only after generation, while live-safe, activation, and
restart-required timing is shown beside the value. Settings and the sidebar
link back to the active world's profile rather than duplicating world controls
inside the server settings surface.

## Native support boundary

The agent evaluates capabilities using server family, Minecraft version, Java
flavor, loader, and installed runtime. The UI re-evaluates after a version or
flavor change. An unavailable field is explained and omitted from a write;
unknown properties are preserved and shown as unknown. The native profile
covers the common 1.20+ Java and Bedrock fields already represented by the
contract:

| Profile area | Java | Bedrock | Timing examples |
| --- | --- | --- | --- |
| Identity and common generation | identity, world type, structures, bonus chest | identity, world type, structures, bonus chest | seed and generation choices are creation-only |
| Gameplay | difficulty, default game mode, gamerules, hardcore, commands | difficulty, default game mode, gamerules, cheats, experiments, coordinates, starting map, advertised toggles | profile values are live-safe, activation-scoped, or restart-required according to runtime |
| Java flavor additions | flat preset, biome/generator options, data packs where advertised | not applicable | capability-gated by flavor/version |

Settings supplied by a particular server build, plugin, or mod are explicitly
**provided by this server/mod**. They are outside the native MSC contract and
are handed off to that server or mod's own configuration path. MSC does not
invent a universal editor or silently claim an unsupported setting applied.

## Evidence map

The targeted checks below are the implementation evidence for the release
boundary. They stay narrow so the P12.29 verifier does not become a second
full-workspace gate.

| Requirement | Evidence |
| --- | --- |
| Fresh server and later Worlds-tab creation use the same form | `clients/desktop-web/tests/screens/world-settings.test.ts`; `clients/desktop-web/tests/screens/add-server-wizard.test.ts`; `WorldSettingsForm.svelte` is imported by both flows |
| Two slots retain different difficulty/game-mode profiles | `crates/msc-application/tests/world_activation.rs::world_activation_switches_between_distinct_profiles_and_preserves_server_settings` |
| Duplicate, import, copy, and restore retain the slot boundary | `crates/msc-application/tests/world_slot_crud.rs` (`world_slot_crud_duplicate_slot_fresh_uuid_source_untouched`, `world_slot_crud_copy_into_existing_success_overwrites_destination`, and import coverage); `crates/msc-application/tests/backup_restore.rs` (`backup_restore_success_extracts_into_server_directory`); `crates/msc-agent/tests/world_backup_routes.rs` restart/recovery coverage |
| Activation and restart timing are truthful | `world_activation.rs` activation tests and `worlds.rs::apply_world_profile`; the shared UI test checks `pending_restart` messaging |
| Server-only values remain common and the force-gamemode default is separate | `world_activation.rs::world_activation_switches_between_distinct_profiles_and_preserves_server_settings`; `clients/desktop-web/tests/screens/world-settings.test.ts`; `content/help/concept/settings.md` |
| Safety confirmation is shared across settings, Quick Commands, console, CLI, and API | `crates/msc-application/tests/command_input.rs::world_safety_confirmation_contract_distinguishes_bedrock_and_server_scope`; `crates/msc-agent/tests/bedrock_routes.rs::safety_confirmation_is_part_of_the_shared_api_contract`; `crates/msc-agent/tests/bedrock_cli.rs::safety_sensitive_cli_commands_expose_confirmation_tokens`; route guards in `routes/settings.rs`, `routes/commands.rs`, and `routes/worlds.rs` |
| Bedrock achievement-disabled readback and Java/Bedrock wording | `msc-application::worlds::detected_profile`; `content/help/handbook/bedrock.md`; `content/help/contextual/settings-gamemode.md` |
| Unsupported version/flavor is not applied | `crates/msc-domain/tests/capability.rs::capability_world_settings_refuse_old_or_unselected_versions_explicitly`; `crates/msc-agent/tests/provisioning_routes.rs`; `clients/desktop-web/tests/screens/world-settings.test.ts` |

## Cameron's live UI walk

The automated evidence does not substitute for the two requested product
walks. In a fresh profile, Cameron should:

1. Create one Java server and one Bedrock server. In each, create a first world
   through Create Server and a second world through the Worlds tab.
2. Give the two slots different difficulty and default game mode values,
   switch between them while stopped, and confirm the server summary remains
   common while each world summary follows its slot.
3. Set Bedrock Creative or cheats and confirm the deliberate achievement
   warning. Confirm that the Handbook explains the Bedrock consequence, Java's
   different advancement/command semantics, the separate `force-gamemode`
   policy, and the server/mod handoff for unsupported settings.

Any setting not advertised by the selected version, edition, flavor, loader, or
runtime must remain visibly unavailable and must not be described as applied.
