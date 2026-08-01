# The capability-advertisement contract: `GET /v1/capabilities`

**Status: Confirmed** by Cameron Temple, 2026-07-31. Like P2.4's error envelope and P2.5's operation model, this is new design against `msc2-engineering.md` §5's "Capability discovery" paragraph, not baseline extraction — MSC 1 has no equivalent route. §7's discrepancy against the engineering doc's own wording (whether "server state" belongs on this route) was confirmed during the Read move that followed: it does not — per-server state stays on its own route, not folded into a host/token-level capabilities response.

---

### 1. Why this exists

Per §5: "Clients ask the agent what it can do. Capabilities reflect host OS, server type, installed helpers, token permissions, agent version, and server state. One client build controls hosts with different capabilities without assuming feature parity underneath." The same iOS binary talks to a Linux headless agent with no Bedrock support and a macOS agent with the VZ sidecar (§9); it needs one place to ask what's actually available on *this* host, for *this* token, before it renders UI that would otherwise fail against a host that can't do the thing.

This document designs the route and response shape only. The detection logic behind most fields — real installed-helper probing, real per-server-type support checks — is Phase 3/4/10 infrastructure work; see §6.

### 2. The route

`GET /v1/capabilities`

- No request body.
- Permission category: `none`, per P2.1's finding that every GET route in the baseline is available to any authenticated token — this route follows the same rule, consistent with the vocabulary P2.8 will assign.
- Auth: the same bearer-token check as every other `/v1/` route (P2.3's dev-token stand-in this phase).
- Response: `200` with a `CapabilitiesDTO`.

### 3. `CapabilitiesDTO`

```json
{
  "agentVersion": "2.0.0-dev",
  "apiMajor": 1,
  "apiMinor": 0,
  "hostOs": "macos",
  "permissions": ["serverControl", "players", "settings"],
  "serverTypes": {
    "vanilla": true,
    "paper": true,
    "fabric": true,
    "forge": true,
    "neoforge": true,
    "bedrock": { "supported": false, "backend": null }
  },
  "helpers": {
    "playit": false,
    "duckdns": false,
    "geyser": false
  }
}
```

- **`agentVersion`** — the running agent build's own semver string. Real this phase (a compile-time constant), not a placeholder.
- **`apiMajor` / `apiMinor`** — the same two numbers P2.4 §2 puts on every response's `X-MSC-Api-Version` header. This route is the one place the numbers come attached to an actual feature list (§4 covers how the list changes with client skew); the header stays the cheap, no-round-trip echo. Real this phase — these are build/route-table constants, not detected at runtime.
- **`hostOs`** — closed enum `macos | linux | windows`, matching `msc2-engineering.md` §8's first support matrix ("MSC agent host"). Real this phase: `std::env::consts::OS` is a compile-time constant, not I/O, so there's no reason to placeholder it even in a skeletal agent.
- **`permissions`** — the calling token's granted categories, drawn from whatever P2.1's validated vocabulary ends up being (nine categories per D-019's revised-but-still-**Proposed** finding: `serverControl`, `players`, `settings`, `addons`, `worlds`, `broadcast`, `networking`, `fleet`, `admin`). Real this phase: P2.3 already wires a fixed dev token, and that token's permission set is exactly what this field echoes back — no separate detection needed. If D-019 is later confirmed with a different bucket list, this field's *contents* change; its *shape* (an array of category strings) does not.
- **`serverTypes`** — per-server-type feature flags, one boolean per Java flavor (`vanilla`, `paper`, `fabric`, `forge`, `neoforge`) plus a `bedrock` object (`supported` bool, `backend`: `"native" | "vz-sidecar" | null`) reflecting §8's second and third support matrices ("Java server host" and "Bedrock runtime"). **Placeholder this phase**, clearly labeled as such in code: real support requires checking installed Java runtimes and, for Bedrock, the native-vs-VZ-sidecar story §9 designs — none of that detection exists yet. The skeletal agent returns fixed values (Java flavors `true`, Bedrock `false`/`null`) rather than guessing at real host state.
- **`helpers`** — installed-helper presence flags for the three genuine external helper integrations named in §5's player-access list (line 414: LAN and port-forwarding need no helper; Playit.gg, Geyser, and DuckDNS do — Xbox Broadcast is LAN-native, not a separate installed helper, so it isn't listed here). **Placeholder this phase** (`false` for all three): real presence detection is filesystem/process probing, which is explicitly Phase 3 substrate work per this phase's own "Not in this phase" note ("Real host/helper/capability detection... all Phase 3 substrate work. This phase's capability and auth responses are honest placeholders, documented as such").

No `helpId` field: P2.2's enumeration of DTOs needing one (settings fields, health cards, diagnostics, performance metrics, connection methods, crash-analysis findings) doesn't include capabilities, and there's no failure mode here for a `helpId` to attach to — `GET /v1/capabilities` has no documented failure response beyond the standard auth/skew ones every `/v1/` route shares.

### 4. Interaction with client-version skew (P2.4 §4)

Per P2.4 §4, an old-but-supported client (`C-N <= m < C`) gets **capability degradation**: this route omits flags for any feature introduced after the *requesting* client's declared minor. That filtering happens server-side, computed from each feature's internal `since_minor` bookkeeping — it is not a new field on the wire. A degraded response is still a valid `CapabilitiesDTO`; it simply has fewer `true` values or narrower lists than the same host would report to a current client. Nothing about the shape in §3 changes for this — this phase can run the mechanism with any placeholder floor `N` (P2.4 §7 already defers the real value), including `N = 0` for early testing, without the DTO itself needing to change later when the real floor is set.

### 5. Caching

No `ETag` or cache-control story is designed here. §5's own framing ("ask the agent what it can do") implies a client calls this once after connecting and again after reconnecting following a version-skew signal (P2.4 §2's `X-MSC-Api-Version` header) — infrequent enough that a full round-trip each time isn't a cost worth optimizing away in a skeletal agent. Revisit if a later phase's client behavior shows otherwise.

### 6. Not designed here

- **Real detection logic** behind `serverTypes` and `helpers` — Phase 3 (helper presence, host capability probing) and Phase 4/10 (per-flavor Java support, native/VZ-sidecar Bedrock backend selection) territory, per this phase's own scope note.
- **Server state.** MSC 2 manages a fleet of servers (D-019's `fleet` category exists precisely because server CRUD is multi-server), so no single global "is a server running" boolean belongs on a host-and-token-level capabilities response. Per-server status stays on the existing `GET /status`-style route. See §7 for why this departs from the engineering doc's literal wording.
- **Push delivery of capability changes** (e.g., a helper getting installed mid-session). This route is polled, matching every other host-level fact this phase reports out-of-band from WebSocket (P0.24's finding, carried forward by this phase's "Not in this phase" note: only `console` and `operation-progress` get WS channels).

### 7. Open discrepancy: "server state" in `msc2-engineering.md` §5

§5's own sentence lists six things capabilities reflect: "host OS, server type, installed helpers, token permissions, agent version, and **server state**." This document's field list (§3) covers the first five and deliberately omits the sixth. Recorded here rather than silently dropped, per `CLAUDE.md` rule 9.

**Why it's omitted:** `GET /v1/capabilities` is a single per-connection, host-and-token-scoped response — it does not take a server name as a parameter, and nothing else in P2.4/P2.5's design suggests it should. "Server state" (presumably: whether a given server is running, stopped, crashed) is inherently per-server, multi-valued across a fleet, and already has a natural home in a per-server status route (MSC 1's own `GET /status`, unversioned equivalent expected under `/v1/` too, though that route's own v1 design isn't this document's job). Folding a fleet's worth of per-server state into a single capabilities blob would either force an array keyed by server name (turning a lightweight "what can this host do" call into an "everything about every server" call) or silently narrow to "the active server," which doesn't hold once MSC 2 manages multiple concurrent servers as a first-class case.

**Confirmed:** §5's "server state" describes the *category* of information capability-flavored UI needs (e.g., "can I start this server" legitimately depends on whether it's already running), not a literal field this route must carry — that need is met by the client combining this route's host/token-level answer with a per-server status call it already has to make. `GET /v1/capabilities` stays host-and-token-scoped only; no per-server state field is added.
