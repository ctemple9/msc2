# Phase 10 API contract: Bedrock runtimes

**Status:** P10.6 contract freeze. This document defines the additive v1
surface that later Phase 10 implementation steps must satisfy. It does not
implement a runtime or claim that a host can run BDS merely because a server
record says `serverType: "bedrock"`.

## 1. Contract principles

Bedrock uses the existing `/v1` routes. The API does not grow a parallel
Java-shaped route family: creation, lifecycle, command delivery, settings,
players, logs, metrics, version changes, operations, worlds, and backups keep
their shared homes. Bedrock-specific behavior is selected by the server type
and disclosed by additive fields.

The existing field names and required fields remain valid. New Bedrock fields
are optional, so an older client can continue to decode a Java response and
ignore a Bedrock runtime description it does not know yet. A Bedrock response
must never make `serverType: "bedrock"` look runnable without also reporting
the runtime state.

## 2. Creation and imported records

`POST /v1/servers/create` accepts the existing `ServerCreateRequestDTO` with
`serverType: "bedrock"` and the additive `bedrockVersion` field. Java-only
fields remain ignored or rejected by the implementation according to the
existing validation rules; they are not repurposed for BDS.

`ServerCreateResultDTO.runtime` is an optional `BedrockRuntimeStateDTO`.
Creation may be accepted as an operation while provisioning is pending, but a
runtime that is unavailable is not presented as a started server. Imported
records follow the same rule: import identifies files and metadata; the
runtime state is reconciled separately before start or other live operations.

The `runtime` object has three states:

| State | Meaning |
|---|---|
| `available` | The selected host/backend has verified BDS files and can perform the requested lifecycle operation. |
| `provisioning_required` | The host/backend is a supported target, but verified BDS files or sidecar resources still need staging. |
| `unavailable` | The requested runtime cannot be used here; the reason and optional help topic are explicit. |

`backend` is `native` on Linux/Windows and `vz-sidecar` on the Intel-macOS
sidecar path. It is `null` when no backend can be selected. Apple Silicon is
not silently mapped to the Intel sidecar: its compatibility evidence remains
`unavailable — no test hardware` under D-028.

## 3. Shared lifecycle and operations

`POST /v1/start`, `POST /v1/stop`, and `POST /v1/command` retain their shared
routes and permission categories. The implementation chooses native BDS or
the macOS sidecar behind the same lifecycle boundary. Bedrock commands strip
one leading `/`; allowlist commands use `allowlist`, save coordination uses
`save hold` / `save query` / `save resume`, and operator changes update
`permissions.json` by XUID rather than pretending that Java's `op` command is
the storage mechanism.

Long-running Bedrock work uses the existing `OperationDTO`,
`GET /v1/operations/{id}`, `POST /v1/operations/{id}/cancel`, and
`/v1/operations/{id}/stream` contracts. Operation `type` values remain an
open additive string vocabulary; Phase 10 may use names such as
`bedrock-provision`, `bedrock-start`, `bedrock-stop`, and
`bedrock-version-change`.

Cancellation is cooperative and follows the existing atomic admission rule:
the cancel request either wins the operation record lock and returns a
captured `202` non-terminal snapshot, or loses to a terminal transition and
returns `409 conflict`. Provisioning cancellation removes only its staging
transaction; a native or sidecar stop cancellation must not leave an orphaned
process or VM. Once graceful termination has begun, the stop operation is not
cancelable. The optional `OperationDTO.cancelable` field discloses that fact
to clients.

## 4. Settings, players, allowlist, and permissions

`GET/POST /v1/settings` remains the settings surface for BDS
`server.properties`. The raw Bedrock file manager preserves unknown keys and
does not clamp values; the API settings layer still owns field-level
validation and any user-facing rejection. The response can include runtime
state so a client can distinguish “settings are readable” from “the server
can start”.

`GET /v1/players`, `/v1/players/profiles`, and `/v1/session-log` remain the
player surfaces. Bedrock XUIDs use the existing optional player identity
fields; a missing XUID or empty gamertag is represented as data, not silently
converted into a Java UUID. `PlayerProfileDTO.isOp` is the shared display
state for operator permission. The Phase 10 agent changes Bedrock operator
state through the shared command/player flow and persists the XUID entry in
`permissions.json`; no Java-only permissions route is added.

`GET/POST /v1/allowlist` remains the Bedrock allowlist surface. It reads and
writes `allowlist.json`, preserves XUIDs when known, and returns the refreshed
list. A non-Bedrock server receives the existing error envelope rather than
an empty Bedrock list. A runtime-unavailable state is carried in the additive
response field when the file can be read; a mutation that requires a live
runtime returns `409` with `ErrorDTO.code: "capability_unavailable"` and a
structured reason.

## 5. Metrics and logs

`GET /v1/performance` remains the metrics route. Native Linux/Windows metrics
come from the existing OS process-statistics substrate. macOS sidecar metrics
come from the sidecar's `[MSCSTATS]` parsing. The shared DTO exposes values,
not either backend's wire format, and each snapshot includes the additive
runtime state when relevant.

`GET /v1/console/tail` and `/v1/console/stream` remain the log/console
surfaces. Bedrock has no server-owned log file in the oracle: console lines
are mirrored to `logs/latest.log`, with the existing bounded rolling behavior.
The websocket payload remains `ConsoleLineDTO`; no Bedrock-only websocket is
introduced. If a runtime cannot supply console data, the HTTP route uses the
standard `ErrorDTO` envelope with `code: "capability_unavailable"` and the
same runtime reason used by `GET /v1/capabilities`.

## 6. Versions, backups, and runtime-unavailable errors

`GET /v1/versions` and `POST /v1/components/version` remain the version
surfaces. For Bedrock, the response sets `isBedrock: true`, reports BDS
versions from the verified distribution manifest, and uses the existing
operation result for a version change. The client never selects a Linux
manifest entry for a Windows or macOS backend by inference; platform dispatch
is agent-owned and provenance is retained.

`POST /v1/backups` keeps the shared backup operation. A running Bedrock server
uses `save hold`, bounded `save query` polling, and `save resume` in cleanup;
the documented timeout behavior proceeds honestly. `POST /v1/backups/restore`
does not become a fabricated live-Bedrock restore path: Bedrock restore is
redirected to the slot-based Worlds workflow, and the existing
`capability_unavailable` guard remains until a supported live restore exists.

All runtime-unavailable failures use the contract-wide `ErrorDTO`:

```json
{
  "code": "capability_unavailable",
  "message": "Bedrock is unavailable on this host.",
  "helpId": "bedrock.runtime-unavailable",
  "details": {
    "capability": "bedrock-runtime",
    "serverType": "bedrock",
    "state": "unavailable",
    "backend": null,
    "reasonCode": "no_test_hardware",
    "hostOs": "macos"
  }
}
```

`helpId` is optional in the general error envelope but required on the
documented Bedrock capability-unavailable examples where the client can
explain a recovery action. Clients branch on `code` and `details.reasonCode`,
never on the human message.

## 7. Capability disclosure

`GET /v1/capabilities` remains the host-and-token capability route. Its
existing `serverTypes.bedrock.supported` and `backend` fields stay for
compatibility; the additive `runtime` object is authoritative for the current
runtime state. It distinguishes agent-host support from BDS support and never
inherits a claim from the Java matrix.

The separate Bedrock compatibility matrix remains the evidence authority for
published platform/version claims. The API reports the current host's
detected state; it does not turn an unverified or unavailable matrix cell into
`supported: true`. Apple Silicon is a distinct unavailable state, not a
missing row and not a tested-negative `unsupported` claim.

## 8. WebSocket relationship

The existing `console`, `operation-progress`, and `notifications` channels
remain the only v1 channels. Bedrock console lines use `ConsoleLineDTO`, and
Bedrock lifecycle/provisioning updates use `OperationDTO`. The operation
stream sends the current snapshot first, then live changes, and closes after a
terminal snapshot exactly as the shared operation contract specifies. A
runtime-unavailable operation fails with the same `ErrorDTO` carried in its
terminal `OperationDTO`; no second Bedrock error frame is invented.

The Phase 10 implementation must not expose sidecar JSON-lines frames,
`[MSCSTATS]` text, guest IPs, or UDP relay details on any public route or
websocket.

