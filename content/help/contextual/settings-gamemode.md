---
id: settings.gamemode
kind: contextual
title: Default game mode
category: settings
analogy: A world's default game mode is the house rule new players start with.
relatedIds: [settings.difficulty, concept.settings, handbook.bedrock]
source: {path: "MSCmacOS/MSCmacOS Swift/RemoteAPIModels.swift", symbol: SettingFieldDTO.help}
---
Default game mode is saved with the selected world slot and used when that
world becomes active. Creative changes gameplay; on Java it changes
advancement and command behavior, while on Bedrock it may permanently disable
Xbox achievements and therefore requires an acknowledgement.

`force-gamemode` is different. It is a server-wide, default-off policy that can
override every world's saved default, so changing it requires its own
confirmation.
