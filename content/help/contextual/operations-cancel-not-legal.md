---
id: operations.cancel-not-legal
kind: contextual
title: This operation cannot be cancelled
category: operations
analogy: Some steps are like sealing a parcel: once the irreversible part starts, stopping midway would be less safe.
relatedIds: [operations.not-found, handbook.worlds-backups]
source: {path: "docs/msc2/api-contract/operation-model.md", symbol: ErrorDTO cancel-not-legal}
---
The operation reached a state where cancellation is not safe or not supported. Wait for its terminal result and read the reported recovery guidance.
