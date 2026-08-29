---
id: concept.settings
kind: concept
title: Settings have two homes.
category: concept-guide
analogy: Server settings run the building; world settings arrange one room.
relatedIds: [settings.difficulty, settings.server-port]
source: {path: "MSCmacOS/MSCmacOS Swift/ConceptGuideView.swift", symbol: CGPage6_Settings}
---
**This world has its own gameplay and generation settings. They are saved with the world and applied whenever this world becomes active. Server settings such as ports and player limits apply to every world.**

World settings cover the selected slot's name/level name, seed, generator,
structures, bonus chest, data packs, difficulty, default game mode, hardcore,
commands, gamerules, and the supported Bedrock choices for cheats,
experiments, coordinates, starting map, and gameplay toggles. Server settings
cover ports, player limits, MOTD, authentication, allowlist/ops, runtime and
process policy, view/simulation distance, crossplay, broadcast, and tunnels.

The old MSC 1 editor grouped some of these together because they were all
written through one screen. MSC 2 follows Minecraft's actual runtime behavior:
the active world projects its saved profile, while a server-wide policy such as
`force-gamemode` stays separate and can be explained as an override.

`force-gamemode` is off by default and has a server-wide confirmation because it
can supersede every world's saved default. Creative and cheat changes use the
world-scoped confirmation; on Bedrock, the explanation calls out the permanent
Xbox-achievement consequence, while Java uses its different advancement and
command semantics.
