---
id: health.ram
kind: contextual
title: RAM allocation
category: health
analogy: RAM allocation decides how much desk space the server reserves before the host needs its own room.
relatedIds: [performance.ram, handbook.ram-performance]
source: {path: "crates/msc-application/src/diagnostics.rs", symbol: check_ram_allocation}
---
An allocation that is too small can cause lag or failure; one that is too large can starve the host operating system and other work.
