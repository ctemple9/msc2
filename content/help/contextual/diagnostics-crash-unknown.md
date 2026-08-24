---
id: diagnostics.crash.unknown
kind: contextual
title: Unclassified startup problem
category: diagnostics
analogy: An unknown problem is a warning light without a reliable label yet.
relatedIds: [health.last-startup, handbook.watchdog]
source: {path: "crates/msc-application/src/diagnostics.rs", symbol: crash_help_id}
---
MSC could not classify the startup failure. Preserve the console and log evidence, then inspect the first meaningful error rather than deleting files blindly.
