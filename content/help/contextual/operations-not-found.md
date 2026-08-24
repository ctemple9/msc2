---
id: operations.not-found
kind: contextual
title: Operation not found
category: operations
analogy: An operation record is a receipt; if it is gone, MSC cannot safely claim its current state.
relatedIds: [operations.cancel-not-legal, servers.not-found]
source: {path: "docs/msc2/api-contract/operation-model.md", symbol: ErrorDTO not-found}
---
The requested operation is not available to the agent. Refresh the server state rather than assuming the action succeeded or failed.
