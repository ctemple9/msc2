---
id: performance.tps
kind: contextual
title: Ticks per second
category: performance
analogy: TPS is the server heartbeat; 20 beats per second is the usual smooth target.
relatedIds: [performance.cpu, health.tick-lag]
source: {path: "crates/msc-agent/src/routes/status.rs", symbol: AgentRuntimeState.performance}
---
TPS measures whether the server is keeping time with the game. Sustained low TPS makes gameplay feel late and should be investigated with CPU, RAM, and console evidence.
