---
id: performance.cpu
kind: contextual
title: CPU use
category: performance
analogy: CPU is the number of workers available to process the world each moment.
relatedIds: [performance.tps, performance.ram]
source: {path: "crates/msc-agent/src/routes/status.rs", symbol: AgentRuntimeState.performance}
---
CPU use helps explain lag when TPS falls. Compare it with player activity, add-ons, and memory pressure rather than reading one number alone.
