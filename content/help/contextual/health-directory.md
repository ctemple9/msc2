---
id: health.directory
kind: contextual
title: Server directory
category: health
analogy: The server directory is its workshop; it must exist and allow MSC to read and write safely.
relatedIds: [handbook.server-files, health.last-startup]
source: {path: "crates/msc-application/src/diagnostics.rs", symbol: check_directory}
---
MSC needs the approved server directory to be present and writable. A missing or restricted directory can prevent startup and safe backups.
