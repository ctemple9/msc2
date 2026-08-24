---
id: performance.world-size
kind: contextual
title: World size
category: performance
analogy: World size is the amount of map material the server must store and protect.
relatedIds: [handbook.worlds-backups, health.directory]
source: {path: "crates/msc-agent/src/routes/status.rs", symbol: AgentRuntimeState.performance}
---
World size helps plan disk space and backups. It is not a direct measure of live performance, but growth can affect backup time and storage health.
