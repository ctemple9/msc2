# The operation model: `OperationDTO` and the three operation routes

**Status: Proposed.** MSC 1 has no operation concept at all — no operation ID, no progress channel, no cancel route (confirmed by P2.9's own framing: "MSC 1 has no operation-journal concept... there is no MSC 1 fixture to extract"). Everything below is greenfield design against `msc2-engineering.md` §5's "Long-running operations" paragraph, not baseline extraction, so it carries the same weight as P2.4's error-envelope correction: recorded as Proposed here, for Cameron to confirm or overrule during the Read move, not silently treated as settled.

---

### 1. Why this exists

Modpack installs, Java downloads, world conversions, backup restores, and loader installations take minutes, not milliseconds. A request that blocks an HTTP connection for that long is hostile to every client (iOS backgrounding, a flaky LAN link, a desktop app the user closes mid-install). §5's answer: return an ID immediately, let the client poll or subscribe for progress, and let it disconnect and reconnect without losing the operation. This document designs the wire shape only — `OperationDTO`, the state machine, and the three HTTP routes that create, read, and cancel one. It does not design how operations survive an agent restart; see §6.

### 2. `OperationDTO`

```json
{
  "id": "01J8XG7K9QZR3F5T6M2N8P0VBC",
  "type": "demo-install",
  "target": "survival2",
  "state": "running",
  "progress": { "current": 42, "total": 100 },
  "statusLine": "Downloading Java 21 runtime (42/86 MB)",
  "result": null,
  "error": null
}
```

- **`id`** — opaque string, server-generated on `POST /v1/operations`. Never client-supplied. Format is an implementation detail (ULID/UUID); clients treat it as an opaque token, per the same "clients don't parse identifiers" discipline the rest of the contract already assumes.
- **`type`** — a string naming the kind of work (`demo-install` this phase; `modpack-install`, `java-download`, `world-conversion`, `backup-restore`, `loader-install`, … as later phases port the real orchestration behind them). Deliberately **not** a closed enum: a closed set would force this contract document to be revised — and potentially version-bumped — every time a later phase adds one more kind of long-running work. New `type` values are an additive, backward-compatible change under D-010's own rule ("additive/optional new fields"); this document does not attempt to enumerate the eventual full set, only note the pattern. The set of `type` values the skeletal agent actually accepts this phase is whatever P2.13/P2.14 wire (just `demo-install`) — a real, growing vocabulary is Phase 4+ territory.
- **`target`** — the thing the operation acts on, typically a server name/ID. Optional: some operation types have no natural target (e.g. a bare Java runtime download not yet tied to any server), so `target` may be omitted from the `POST` body and is then `null` on the DTO. When present, it is a plain string, not a nested object — consistent with how the baseline already names servers by string ID elsewhere in the API.
- **`state`** — closed enum: `queued|running|succeeded|failed|cancelled`. See §3 for the transition rules.
- **`progress`** — nullable object `{ "current": <int>, "total": <int> }`, both non-negative. Present once the operation type has a natural unit to count (files, bytes, steps) and the agent has started counting; `null` while `queued`, and `null` for any operation type with no natural unit (there is no synthetic percentage invented to fill the gap). Deliberately just two integers, not a pre-computed percentage — the client divides; the wire format doesn't carry a derived value that could drift from the two numbers it's derived from.
- **`statusLine`** — nullable human-readable string, meant for direct display (§5's own phrase, "a human-readable status line"). Independent of `progress`: an operation can have a status line before it has anything countable ("Waiting for a free download slot") or after counting has finished ("Verifying archive"). `null` until the agent has something to say.
- **`result?`** — present only when `state == "succeeded"`. Free-form object, shape defined per `type` (a `java-download`'s result names the installed runtime path; a `backup-restore`'s result names the restored backup ID — each future operation type documents its own `result` shape when that type is designed, not here). `null`/absent otherwise.
- **`error?`** — present only when `state == "failed"`. Reuses P2.4's `ErrorDTO` verbatim (`code`, `message`, `helpId?`, `details?`) rather than inventing an operation-specific failure shape — the same "one error envelope" principle P2.4 §5 establishes for HTTP responses applies here too. `null`/absent otherwise. Note `state == "cancelled"` carries **neither** `result` nor `error`: cancellation is not a failure, so there is nothing to explain beyond the state itself (`statusLine` may still read something like "Cancelled by user" for the human-readable trail, but structurally no `error` object is populated).

### 3. The state machine: `queued|running|succeeded|failed|cancelled`

Legal transitions only:

| From | To | Trigger |
|---|---|---|
| `queued` | `running` | The agent picks the operation up and starts work. |
| `running` | `succeeded` | Work completes normally. |
| `running` | `failed` | Work errors out; `error` is populated. |
| `queued` | `cancelled` | `POST /v1/operations/{id}/cancel` on a not-yet-started operation. |
| `running` | `cancelled` | `POST /v1/operations/{id}/cancel` on an in-flight operation. |

`succeeded`, `failed`, and `cancelled` are **terminal** — no transition is legal out of any of them. A cancel request against a terminal operation is refused (§4.3), not silently accepted or ignored. This is the exact rule set P2.9 implements as a Rust enum with compile-time-checked transitions; this document is that implementation's specification, written first.

### 4. The three routes

#### 4.1 `POST /v1/operations` — create

Request:

```json
{ "type": "demo-install", "target": "survival2", "params": {} }
```

`type` is required. `target` is optional (§2). `params?` is a free-form object carrying whatever type-specific arguments that `type` needs (which modpack version, which backup snapshot to restore, …) — like `result`, its shape is defined per `type` when that type is designed, not enumerated generically here.

Response: `202 Accepted` (not `200` — the request is accepted for asynchronous processing, nothing has necessarily happened yet) with a fresh `OperationDTO`, always `state: "queued"` at creation.

Failure: an unrecognized `type` value is `400` with P2.4's `ErrorDTO` (`code: "invalid_body"`). The known-`type` set is whatever the running agent's build actually implements (§2) — this phase, only `demo-install`.

#### 4.2 `GET /v1/operations/{id}` — read current state

Response: `200` with the current `OperationDTO`.

Failure: unknown `id` is `404` with `ErrorDTO` (`code: "not_found"`, `helpId: "operations.not-found"`). This phase, "unknown" includes any operation the agent has forgotten across a restart (§6) — a `404` here is the honest, expected response this phase, not a bug to design around.

#### 4.3 `POST /v1/operations/{id}/cancel` — request cancellation

No request body. Cancellation is cooperative. The agent makes cancellation admission and its response snapshot one atomic application-level decision under the operation record lock. If admission wins, it sets the cooperative flag and returns `202 Accepted` with that captured non-terminal `OperationDTO` (normally `state: "running"`, `statusLine: "Cancelling…"`). A worker transition after that decision cannot rewrite the already-captured response body. Clients poll `GET /v1/operations/{id}` or follow the operation stream until the worker reaches a terminal state. The per-target operation lock remains held until that worker finishes cleanup and records its own terminal transition.

Failure: `409` with `ErrorDTO` (`code: "conflict"`, `helpId: "operations.cancel-not-legal"`) if the worker's `succeeded`, `failed`, or `cancelled` transition wins the same lock first. Cancelling a finished operation is refused, not treated as a silent no-op, so a client always knows whether its cancel actually did anything. Unknown `id` is `404`, same as §4.2.

#### 4.4 Mutating server imports

`POST /v1/servers/import` keeps `action=scan` synchronous and returns its
existing `ServerImportScanResponseDTO` with `200 OK`. The mutating
`importExisting`, `importTransfer`, and `rescan` actions are an explicit D-006
correction: after validation and operation admission they return `202 Accepted`
with the existing `ServerImportResultDTO`, whose `operationId` names the durable
operation. Copying, extraction, registration, and world reconciliation continue
in a background worker. Clients poll `GET /v1/operations/{operationId}` (or use
the operation stream) for the final state and result; they must not keep the
import request open while filesystem work runs.

The worker alone records `succeeded`, `failed`, or `cancelled`. Successful
operation results carry `serverId`, `serverName`, `imported`, `skipped`, and
`replaced` where those values apply. A Java server is selected only after world
reconciliation reports it `Ready`; a reconciliation failure leaves the imported
server registered as `Degraded` for diagnosis. Cancellation is honored before
work begins or after an unregistered raw import has been copied and can still be
removed safely. Once registration or another irreversible transfer boundary has
been crossed, the worker finishes with the truthful outcome instead.

### 5. Not designed here

- **Progress/cancellation delivery over WebSocket** — `/v1/operations/{id}/stream`, pushing `OperationDTO` updates as they happen, is P2.7's job. This document is the DTO that channel carries; it does not itself design the streaming transport.
- **Permission category and `helpId` assignment for the three routes above** — P2.1 (D-019 vocabulary) and P2.2 (`helpId` placement) both run at the whole-baseline level; P2.8 is where every route in this document gets its category and, where applicable, its `helpId` values assigned into the assembled `openapi.json`. This document reserves the slots (`ErrorDTO.helpId` is already optional on every failure) without assigning specific values.
- **Operation exclusivity** (`msc2-engineering.md` §7: "Only one conflicting operation runs against a server at a time") is orchestration policy owned by `msc-application`, not a wire-contract concern. The skeletal `POST /v1/operations` this phase accepts operations unconditionally — there is no real work yet for two operations to conflict over. Enforcing exclusivity, and deciding what `ErrorDTO.code` a conflicting request gets back, is Phase 3/4 scope once real orchestration exists.
- **The full `type` vocabulary** — only whatever P2.13/P2.14 need to exercise this state machine (`demo-install`) exists this phase. Each later phase that ports a real long-running workflow adds its own `type` value (and, per §2, that is an additive change, not a contract-version bump).

### 6. Restart survival is explicitly out of scope here

Per this phase's own "Not in this phase" note and P2.14's design, operations this phase live in an **in-memory map only**. An agent restart forgets every in-flight operation — expected, not a defect, until Phase 3 builds the operation journal `msc2-engineering.md` §7 and §15 describe: every long operation journaled before it begins, incomplete operations reconciled and their outcome *explained* on restart rather than silently dropped. This document specifies the wire shape (`OperationDTO`, the state machine, the three routes) that the journal will sit behind; nothing here — the DTO fields, the routes, their request/response shapes — needs to change when Phase 3 adds durable storage underneath `GET /v1/operations/{id}`. Only the *honesty* of a `404` for a forgotten operation changes, from "expected, this phase" to "should no longer happen, once journaled."
