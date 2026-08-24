---
id: performance.ram
kind: contextual
title: Memory use
category: performance
analogy: Memory is the working desk used by the server while it keeps a world running.
relatedIds: [health.ram, handbook.ram-performance]
source: {path: "crates/msc-agent/src/routes/status.rs", symbol: AgentRuntimeState.performance}
---
This metric reports the server’s current and maximum memory values. It does not by itself prove a memory leak or justify giving the server all host memory.
