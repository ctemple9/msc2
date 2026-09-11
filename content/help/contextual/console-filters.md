---
id: console.filters
kind: contextual
title: Console filters
category: console
analogy: The console is the operator's window into the server room; filters decide which human-facing signals stay in that window.
relatedIds: [performance.tps, handbook.overview]
source: {path: "docs/msc2/api-contract/openapi.json", symbol: "ConsoleLineDTO and /v1/console/tail"}
---
The main console contains bounded human history: commands you enter and output from the Minecraft server. MSC's monitoring commands, backup coordination, and routine helper output are classified before retention, so they cannot push real server output out of the history window.

Use search and the Server, Plugins, Warnings, or Commands filters to narrow that history. A helper failure or authentication prompt belongs in the related status, operation, notification, or connectivity surface rather than being hidden as routine console noise.
