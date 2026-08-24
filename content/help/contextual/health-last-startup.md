---
id: health.last-startup
kind: contextual
title: Last startup
category: health
analogy: The last-startup card is the shift report after a server attempted to open.
relatedIds: [diagnostics.crash.unknown, handbook.watchdog]
source: {path: "crates/msc-application/src/diagnostics.rs", symbol: check_last_startup}
---
This card summarizes the latest startup record. A warning or failure should lead to the specific diagnostics rather than a guess based only on console text.
