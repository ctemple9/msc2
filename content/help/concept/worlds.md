---
id: concept.worlds
kind: concept
title: A server holds multiple worlds.
category: concept-guide
analogy: World slots are separate save files in one server’s cabinet.
relatedIds: [concept.active-world, handbook.worlds-backups]
source: {path: "MSCmacOS/MSCmacOS Swift/ConceptGuideView.swift", symbol: CGPage4_Worlds}
---
One server can keep several worlds, but only one is active at a time. Each
world slot carries its own versioned profile: its identity, seed, generation
choices, and gameplay settings travel with the saved world instead of being
copied from whichever world was active last.

**This world has its own gameplay and generation settings. They are saved with the world and applied whenever this world becomes active. Server settings such as ports and player limits apply to every world.**

The active runtime is a projection of the selected slot's profile. Switching
slots must not rewrite another slot's difficulty, default game mode, gamerules,
or generation choices.

MSC's native profile stops at settings it can verify for the selected Minecraft
version, edition, and Java flavor. A server or mod may provide additional
settings; those are handed off to that server or mod's own configuration path,
not silently presented as MSC controls.
