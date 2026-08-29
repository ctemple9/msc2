---
id: concept.active-world
kind: concept
title: One world is always active.
category: concept-guide
analogy: The active world is the room new guests are directed into.
relatedIds: [concept.worlds, handbook.worlds-backups]
source: {path: "MSCmacOS/MSCmacOS Swift/ConceptGuideView.swift", symbol: CGPage5_ActiveWorld}
---
New player connections enter the active world; players already in a world stay where they are. When a slot becomes active, MSC applies that slot's saved profile and reports what the runtime actually accepted.

**This world has its own gameplay and generation settings. They are saved with the world and applied whenever this world becomes active. Server settings such as ports and player limits apply to every world.**

The live world is therefore a projection, not a second source of truth. A
difficulty or default game mode changed for this world belongs to this slot;
the server's port, player limit, MOTD, access rules, process policy, and
connection helpers remain common to every slot.
