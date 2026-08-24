---
id: diagnostics.crash.duplicate
kind: contextual
title: Duplicate add-on
category: diagnostics
analogy: A duplicate add-on is two copies of the same key trying to fit one lock.
relatedIds: [health.component-jars, diagnostics.crash.load-error]
source: {path: "crates/msc-application/src/diagnostics.rs", symbol: crash_help_id}
---
The startup analysis found duplicate add-on content. Keep the intended compatible copy and remove the duplicate only after confirming the server is stopped.
