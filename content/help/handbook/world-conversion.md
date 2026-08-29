---
id: handbook.world-conversion
kind: handbook
title: World Conversion
category: server-management
subtitle: "Convert a world between Java Edition and Bedrock Edition format."
analogy: "Java and Bedrock worlds are written in different formats — like the same novel published in two different languages. Conversion reads the original and writes it out in the other language. Most content comes through cleanly; some edition-specific features don’t have an equivalent in the other format and get left behind."
relatedIds: [handbook.worlds-backups, handbook.server-transfer]
source: {path: "MSCmacOS/MSCmacOS Swift/ServerHandbookTopics.swift", symbol: worldConversionContent}
---

Java Edition and Bedrock Edition store world data in fundamentally different formats. A world created on a Java server cannot be directly loaded by a Bedrock server, and vice versa. The World Conversion wizard handles the translation automatically.

**When you’d use this:**

### Bullet List

- You’re switching from a Java server to a Bedrock server (or vice versa) and want to keep your existing world
- You want to run the same world on both a Java and a Bedrock server simultaneously
- A player wants to continue their singleplayer Bedrock world on your Java server

### Callout: warning

World conversion is a best-effort process. Blocks, items, and entities that don’t exist in the target edition are dropped or approximated. Always create a backup before converting, and test the result before replacing your active world.

### In This App

- Worlds tab → World Conversion Wizard: select the source world and target format.
- The wizard creates a converted copy — it does not replace or delete your original world.
- Review the conversion summary after it finishes: it lists anything that couldn’t be converted.
- To use the converted world: open the Worlds tab and use Replace World to swap it in after the server is stopped.

### Callout: note

For most survival worlds, terrain, blocks, and player inventory convert cleanly. The main losses are edition-exclusive content: Java-only technical blocks, Bedrock-only items, and any content added by plugins or mods.

### Advanced Details

Java worlds store chunk data in the Anvil format — .mca region files inside a world/ directory. Bedrock worlds use LevelDB — a key-value store inside a leveldb/ directory. These are completely different storage formats, which is why direct file copying doesn’t work between editions.

Conversion quality is generally good for vanilla survival worlds built on recent versions. The further apart the source and target versions, or the more modded/plugin-generated the content, the more conversion gaps you’ll see.

The converted world should be tested in the target edition before you commit to using it as your active server world. Run around the spawn area, check your inventory, and test key locations before retiring the original.
