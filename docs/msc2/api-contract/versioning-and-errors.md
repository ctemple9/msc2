# v1 route namespace, skew behavior, and error envelope

**Status: Proposed.** This designs the mechanism D-010 already approved (floor + degradation + refusal + major-namespace) down to wire-level detail — header names, status codes, exact DTO shape. The one number D-010 explicitly left open, the floor width (was estimated at N-3), stays open here too; see §7. The error-envelope unification in §5–6 is a new correction against the baseline, recorded under D-006 point 3, and is Proposed until Cameron confirms it during the Read move.

---

### 1. Route namespace

MSC 1 has no version namespace today — confirmed by reading the actual dispatcher (`RemoteAPIServer+HTTP.swift:173-199`, `knownPathsCanonical`): paths are bare (`/servers`, `/worlds/create`, `/config/ram`, …), and nothing in the Remote API reports a schema/API version at all (checked: no `apiVersion`/`apiMajor`/`X-API-Version` anywhere in `MSCmacOS/`). This is greenfield design, not extraction.

**Every MSC 2 route lives under `/v1/`**: `/v1/servers`, `/v1/worlds/create`, `/v1/config/ram`, and so on — otherwise identical to the baseline's own path shapes (P0.23a–s), just prefixed. A request against the unprefixed path (`/servers` instead of `/v1/servers`) is a plain 404: there is no live MSC 1 wire format to stay compatible with underneath the prefix, since MSC 2's API is a new contract that an existing MSC 1 client never speaks.

**The major segment is the only version information in the path.** `/v1` → `/v2` only on a breaking change (D-006 point 3: a correction, not a routine bump). Minor versions are not path segments — they're carried out-of-band (§3), because D-010's degrade-within-window behavior requires the *same* route to behave slightly differently for an old-but-supported client, not a different route.

**404-vs-405 semantics are preserved** (D-006's preserved list) at the new prefix: a known `/v1/...` path hit with the wrong HTTP method is 405; an unknown path is 404. `msc-api`'s router carries forward the same "canonical known-path set to disambiguate 404 from 405" design MSC 1 uses (`knownPathsCanonical`), just rebuilt from the P2.8 OpenAPI file instead of a hand-maintained Swift `Set`.

**Multiple majors may be mounted at once.** The design doesn't require it now (only `/v1` exists this phase), but nothing about routing prevents `msc-agent` from serving `/v1` and `/v2` side by side during a future deprecation window — worth stating now so P2.8 doesn't accidentally hard-code single-version assumptions into the router.

### 2. What the agent reports about itself

Per D-010, "the agent reports API major/minor and its capability set on connect." Two channels carry this, at different costs:

1. **`GET /v1/capabilities`** (designed fully in P2.6) is the authoritative source: full `api_major`/`api_minor`, plus the capability set. Any client can call it once after connecting.
2. **Every response also carries `X-MSC-Api-Version: <major>.<minor>`** as a response header, cheap enough to check on every request without a round trip dedicated to it — useful for a client that wants to notice mid-session that it has drifted (e.g., after the agent restarts on a newer build) without re-polling capabilities.

Both report the same two numbers; `/v1/capabilities` is the one with the actual feature list attached (§ design lives in P2.6, not duplicated here).

### 3. What the client reports about itself

D-010's degrade-within-window behavior needs the agent to know what the *client* was built against, not just what it's asking for right now. **Every request may carry `X-MSC-Client-Api-Version: <major>.<minor>`** — the API version the client was built against, set once at build time, not derived from anything dynamic.

- **Header present:** the agent evaluates skew (§4) against the declared minor.
- **Header absent:** the agent assumes the caller is not participating in the skew protocol (a dev tool, `curl`, an internal test) and skips skew evaluation entirely — full current behavior, no degradation, no refusal. The real iOS client always sends this header once P2.18/P2.20 wire it; omission is the exception, not the norm, so defaulting it to "most permissive" rather than "assume oldest" avoids punishing tooling that has no opinion about versioning.

### 4. Skew behavior within a major version

Given the agent's current minor `C` and a floor width `N` (§7, still unset), the supported window is `[C-N, C]`. For a request carrying `X-MSC-Client-Api-Version: 1.<m>`:

| Where `m` falls | Behavior |
|---|---|
| `m == C` | Full current behavior. |
| `C-N <= m < C` | **Capability degradation.** `GET /v1/capabilities` (P2.6) omits capability flags for any feature introduced after minor `m`, computed relative to the *requesting* client's declared version — an old-but-supported client is never told about a feature it wouldn't understand, and never sees agent behavior change on it as a surprise. All new fields on existing DTOs stay additive and optional (D-010's own requirement) so an old client that ignores unknown fields keeps working even without consulting capabilities at all. |
| `m < C-N` | **Clear refusal**, not the generic 404 an unversioned mismatch would otherwise produce. `426 Upgrade Required` (the HTTP code that means exactly this) with the `ErrorDTO` from §5: `code: "client_version_unsupported"`, `message` naming the minimum supported minor, `helpId: "versioning.unsupported-client"`, `details: { minimum_supported_minor, current_minor, client_reported_minor }`. |
| `m > C` (client claims to be newer than the agent) | Treated the same as `m < C-N` — refuse, don't guess. An agent cannot safely serve a contract it hasn't implemented yet; there is no forward-degradation mode. |

A **major mismatch** (client hard-codes `/v2/...`, agent only serves `/v1`) is handled by ordinary routing (§1) — it's a 404 at the path level, not a skew decision at the handler level, because the two majors are different route trees, not different behaviors of one tree.

### 5. One error envelope: `ErrorDTO`

```json
{
  "code": "not_found",
  "message": "No server named 'survival2' exists.",
  "helpId": "servers.not-found",
  "details": { "requestedName": "survival2" }
}
```

- **`code`** — a stable, machine-readable snake_case identifier. Small closed-ish vocabulary per failure kind (`invalid_body`, `missing_field`, `not_found`, `conflict`, `forbidden`, `unauthorized`, `rate_limited`, `client_version_unsupported`, `internal_error`, …), not one-off per route — a client branches on `code`, never on `message` text.
- **`message`** — human-readable, iOS-visible (D-006's preserved-semantics list: "iOS-visible error semantics"). This is the string MSC 1's existing UI-facing error text maps onto; wording is preserved where a baseline route already has one, per D-006 point 1.
- **`helpId?`** — optional, resolves through P2.2's `GET /v1/help/{helpId}` when the failure has associated educational content (e.g. why a Java version guard rejected a runtime). Absent when there's nothing more to say than `message` already says.
- **`details?`** — optional, free-form structured object for whatever route-specific context used to live inside a typed failure DTO (§6) — validation field names, conflicting version strings, retry-after seconds for a 429, etc. Structured, not prose; `message` carries the prose.

Every non-2xx response across every `/v1/` route uses this one shape. No route defines its own failure DTO.

### 6. Why this replaces the baseline's split pattern (D-006 point 3)

P0.32 read every mutation handler in MSC 1 and found the baseline doesn't have one failure shape — it has two, and which one a client gets back depends on an implementation accident, not a deliberate contract:

- **Pre-provider guards** (`missing_body`, `invalid_json`, field-required checks that run before MSC 1's internal `Task` block starts) return the generic `{"error": string}`.
- **Post-provider results** (everything the provider itself decides — the actual 404/409/422/429/500 cases, confirmed unambiguous across all 27 route+method pairs P0.32 catalogued) return that route's own typed result DTO, because MSC 1's `sendJSON(statusCode:encodable:...)` reuses the same success-shape object whether the provider succeeded or failed.

A client integrating against the baseline has to know, per route and per status code, which of two unrelated shapes to expect for the same "this failed" event — 68 status-code entries' worth of that, per P0.32's count. That's DTO nesting driven by *which line of Swift happened to produce the response*, which is exactly the category D-006 names as **not preserved**: "DTO nesting that exists only for Swift file organization." `ErrorDTO` collapses both cases into one shape; whatever route-specific payload the old typed DTO carried moves into `details`.

This is recorded here as a conscious D-006-point-3 correction — MSC 1's own behavior is not carried forward on this one point — rather than silently diverging from "preserve iOS-visible error semantics." The *text and status codes* stay recognizable (§5's `message` field); only the shape wrapping them is unified.

### 7. What's still open beyond this step

- **The floor width `N`** (§4's `[C-N, C]`) is not set here. D-010 already flags this as "not yet approved... should be set from real App Store update-adoption data once MSC 2 ships, not guessed now" — nothing in Phase 2 changes that; the skeletal agent can run the mechanism above with any placeholder `N` (a small constant is fine for now, e.g. for P2.11-P2.17's tests) without the real value being decided.
- **The exact `code` vocabulary** is sketched in §5 but not exhaustively enumerated against all 88 baseline routes — that enumeration happens in P2.8 when the full OpenAPI contract is assembled and every route's failure responses get a concrete `code`.
