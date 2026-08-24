---
id: diagnostics.crash.missing-dependency
kind: contextual
title: Missing add-on dependency
category: diagnostics
analogy: A missing dependency is a tool a component expects to find in the workshop but cannot.
relatedIds: [diagnostics.crash.incompatible-version, handbook.mods-browser]
source: {path: "crates/msc-application/src/diagnostics.rs", symbol: crash_help_id}
---
The named add-on needs another compatible component. Install the required dependency or use a version whose requirements match the server.
