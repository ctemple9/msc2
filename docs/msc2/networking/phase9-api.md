# Phase 9 API and capability contract

**Status:** P9.4 contract-freeze note. This note implements the approved P9.3
access posture; it does not reopen D-012's Phase 11 browser/desktop work.
**Source of truth:** `docs/msc2/api-contract/openapi.json` and
`docs/msc2/api-contract/websocket-v1.json`. This note explains their Phase 9
additions and the implementation obligations they create.

Phase 9 uses the already-versioned `/v1` surface. It adds no alternate
management listener and no player-facing route that can proxy management
traffic. A Playit address or resource-pack URL is for Minecraft clients, not
for controlling MSC.

## 1. Route map

All listed HTTP routes already existed in the frozen baseline. P9.4 fixes their
meaning before P9.5+ supplies implementations; it creates only the
`WS /v1/notifications/stream` channel below.

| Area | Read/status | Mutation | Permission | Timing |
| --- | --- | --- | --- | --- |
| Connectivity and DuckDNS label | `GET /v1/connectivity`, `GET /v1/duckdns` | `POST /v1/duckdns` | `none`; `settings` | synchronous configuration; diagnostics are bounded reads |
| Playit | `GET /v1/playit` | `POST /v1/playit/start`, `/stop` | `none`; `networking` | managed operation; start is cancellable before readiness, stop is not cancellable once graceful termination begins |
| Resource packs | `GET /v1/resourcepacks` | `POST /v1/resourcepacks/activate`, `/seturl`, `/toggle`, `/remove` | `none`; `addons` | synchronous transactional mutation; no arbitrary file path is accepted or served |
| Geyser/Floodgate | `GET /v1/components`, `GET /v1/config/geyser` | `POST /v1/components/update`, `POST /v1/config/geyser` | `none`; `addons`; `settings` | component download/update is an operation; config write is synchronous |
| Xbox Broadcast | `GET /v1/broadcast/{status,autostart,auth-prompt,jar-status}` | `POST /v1/broadcast/{autostart,auth-prompt/dismiss,credentials,download-jar,start,stop,restart}` | `none`; `broadcast` | download/start/stop/restart are managed operations; changing stored credentials/preferences is synchronous |
| Named tokens | `GET /v1/users` | `POST /v1/users`, `/users/update`, `/users/revoke` | `admin` only | synchronous and durable; revoke takes effect before the response succeeds |
| Notifications | `WS /v1/notifications/stream` | none | `none` | bounded history then live, observation only |

`GET /v1/connectivity` remains the sole connectivity route. Its additive
`portDiagnostics` object reports the local probe and the public status-query
separately, with each outcome `open`, `closed`, `unreachable`, `unavailable`,
or `not_applicable`. `unavailable` means MSC could not run the diagnostic; it
does not claim the Minecraft port is closed. `joinAddressSource` says whether
the displayed address came from Playit, the DuckDNS label, public IP, or is
unavailable. These values supplement—not replace—the baseline fields so older
clients can keep reading the original summary.

## 2. Operations and cancellation

Every long-running helper action returns the existing operation identity in an
additive `operationId` field and may use `202 Accepted`. A client follows the
existing `GET /v1/operations/{id}`, operation-progress stream, and cancellation
route; it must not infer readiness from process launch alone.

- Playit start, Broadcast JAR download/start/restart: cancellable until the
  helper reaches its terminal ready/failed state.
- Playit/Broadcast stop: operation-backed but not cancellable after the agent
  starts graceful termination, because cancelling then could leave duplicate
  player-facing helper processes.
- Resource-pack configuration and Geyser configuration: synchronous only;
  their filesystem transaction completes or rolls back before the response.
- Component installation/update uses the existing add-on operation contract.
- Token CRUD is not an operation: creation returns the bearer token once;
  list, update, and revoke never return it. A revoke is durable before `200`.

The baseline `200` response remains allowed for a completed/no-op result.
`202` plus `operationId` is used where work continues. This is additive for
older clients, which can ignore the unknown field and continue polling their
normal status routes.

## 3. Secrets, events, and help

`PlayitStatusResponseDTO`, `BroadcastStatusDTO`, connectivity data, operation
status lines, notification events, audit records, and all list/update token
responses are secret-free. They may expose a player join address but never a
Playit secret, Xbox password/account token, bearer secret, verifier, or a
general management address. `BroadcastCredentialsDTO.password` is accepted
only as a request value and is neither echoed nor retained in any response.
`UserCreateResultDTO.token` is the single exception: the newly generated raw
bearer token is returned once over the authenticated admin route.

The new notification stream carries `NotificationEventDTO`: server started,
server stopped, player joined, player left, helper failed, or connectivity
changed. The first four preserve MSC 1's actual notification events; the last
two are Phase 9 additions. The channel is bearer-authenticated before upgrade,
read-only, and bounded to the current server's recent events followed by live
events. Native OS delivery remains client-owned. A user-facing state that needs
an explanation carries optional `helpId`; clients resolve it through the
already-promised `GET /v1/help/{helpId}` when Phase 11 provides content.

## 4. Deferred by P9.3

This contract deliberately has no desktop pairing endpoint, browser cookie
endpoint, origin/CSP setting, CSRF token, general-LAN management bind, or TLS
certificate endpoint. Phase 9 management is loopback by default, with an
explicit Tailscale path only; bearer authentication and permission checks still
apply on that path. These omissions are intentional Phase 11 work, not missing
Phase 9 capabilities.

## 5. Capability matrix and count

Every newly added Phase 9 row remains `Planned` for agent, iOS, and CLI at
this contract-only point. The older generic `components` routes retain their
existing `Implemented` status, but their Phase 9 Geyser/Floodgate behavior is
still planned. Desktop/web remains `Planned`, because Phase 11 owns that
client. The matrix therefore does not claim a new client surface exists.

OpenAPI remains at 110 HTTP operations. The WebSocket contract grows from two
to three channels, so the capability matrix covers 113 operations total.

## Verify

```
python3 tools/api-contract-check.py --v1-summary && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && cargo nextest run -p msc-api --test phase9_conformance && rg -n 'ConnectivityResponseDTO' docs/msc2/api-contract/openapi.json
```
