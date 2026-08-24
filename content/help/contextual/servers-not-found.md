---
id: servers.not-found
kind: contextual
title: Server not found
category: errors
analogy: A server ID is an inventory label; a missing label cannot identify a safe target for an action.
relatedIds: [operations.not-found, handbook.server-transfer]
source: {path: "docs/msc2/api-contract/versioning-and-errors.md", symbol: ErrorDTO servers-not-found}
---
The selected server is no longer registered or is unavailable to this request. Refresh the host’s server list before retrying a management action.
