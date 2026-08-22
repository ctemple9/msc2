# Phase 4 pairing and credential storage

**Status:** P4.2 design note. Proposed, pending Cameron's verification; D-012
stays incomplete outside this CLI/iOS slice.
**Source of truth:** `msc2-decisions.md` D-012 and D-019,
`docs/msc2/lifecycle/phase4-scope.md`, `docs/msc2/substrate/secret-storage.md`,
and MSC 1's `RemoteAPIServer+HTTP.swift`, `RemoteAPIServer.swift`,
`AppViewModel.swift`, `MSCSettingsView.swift`, and the copied iOS
`SettingsStore.swift` / `KeychainTokenStore.swift`.

This note replaces the Phase 2 `MSC_DEV_TOKEN` stand-in for Phase 4's real
mutating lifecycle slice. It does not design desktop/browser pairing, LAN TLS,
Tailscale policy, cookie sessions, or CSRF. Those D-012 questions remain open.

**P4.40 amendment, 2026-08-12:** this document remains the Phase 4 credential
contract, but it is not an accurate statement of what production `msc serve`
currently wires. The implemented bearer verifier uses the `SecretStore` trait
interface and the documented token shape, yet the service construction path
still supplies `FakeSecretStore`. Treat P4.5 as having implemented the auth
model and tests, not as having proven durable platform secret storage in the
installed service. P4.42 must replace that construction path with a production
store factory before this contract is true in the running agent.

## Scope

Phase 4 has two real clients: the CLI and the existing iOS app. Both use bearer
tokens against the same HTTP API the lifecycle routes use.

In scope:

- Issue durable bearer credentials for the CLI and iOS.
- Store server-side token secrets through `SecretStore`, not in ordinary config.
- Store client-side tokens per host, so one client can later control multiple
  hosts without overwriting a different host's credential.
- Preserve MSC 1's role and permission model from D-019: admin, guest, named
  tokens with permission categories and optional expiry.
- Preserve MSC 1's auth-failure rate limit shape for the Phase 4 route set.
- Attribute auth failures, forbidden requests, rate-limited requests, and
  lifecycle mutations in the audit log.
- Fix the copied iOS fresh-install empty-token bug from P2.20.

Out of scope:

- Tauri local automatic authorization.
- Tauri remote-host pairing.
- Browser cookie session issuance and CSRF.
- LAN TLS certificate provisioning and trust.
- Whether Tailscale membership relaxes anything. Default remains no: bearer
  token authentication is still required over Tailscale.
- Per-person identity, invitations, account recovery, or an MSC-hosted account
  service.

## Token Shape

MSC 1 stores raw tokens and looks them up in a dictionary. MSC 2 should not
store raw bearer tokens at rest when the Phase 3 `SecretStore` exists.

Phase 4 token format:

```text
msc2_<credential-id>_<secret>
```

`credential-id` is a generated opaque id stored in the non-secret credential
registry. `secret` is high-entropy random bytes encoded for URLs. The agent
never needs to list the secret store to authenticate a request: it parses the
id from the bearer token, loads exactly `remote-api.token.<credential-id>` from
`SecretStore`, and compares a stored hash of the secret portion against the
presented secret.

Server-side secret-store key:

```text
remote-api.token.<credential-id>
```

The value is a small JSON record containing the hash algorithm, salt if needed,
and hash of the token secret. It does not contain the raw bearer token. The
non-secret registry stores the same credential id, label, role, permission
categories, creation time, optional expiry, revoked state, and last-used time.
That registry is what backs `GET /v1/users`; `SecretStore` is the authority for
whether a presented bearer token can authenticate.

`remote-api.owner-token` and `remote-api.guest-token` from
`secret-storage.md` remain MSC 1 migration names. New Phase 4 credentials use
`remote-api.token.<credential-id>` so multiple named tokens can exist without
adding new hardcoded `SecretStore` methods.

## Issuance

There are three issuance paths in Phase 4.

1. Initial local owner credential:
   The first install/bootstrap creates one admin credential for the installing
   user's local CLI. This is the root of trust for creating the first iOS token.
   It is generated once, returned once, stored server-side as described above,
   and saved client-side for the local CLI host entry.

2. CLI-created pairing challenge:
   An authenticated admin CLI command creates a short-lived pairing challenge
   for another client. For Phase 4 that client is iOS. The challenge is kept in
   agent memory with an expiry and one-use flag, and the CLI prints a pairing
   URI/QR payload.

3. Pairing exchange:
   iOS sends the pairing challenge to the agent and receives one durable bearer
   token. The agent creates a normal named credential record, stores its token
   hash in `SecretStore`, returns the raw token once, and invalidates the
   challenge immediately.

The pairing challenge endpoint is the only unauthenticated auth endpoint in this
slice. It is acceptable because the challenge is high entropy, short-lived, and
created by an already-authenticated admin token; failed exchanges are rate
limited and audited. It does not grant local desktop automatic authorization,
which remains a D-012 open item.

Recommended Phase 4 CLI names:

```text
msc token bootstrap
msc token create --label <label> --role admin|guest|named --permissions <csv>
msc token revoke <credential-id>
msc pair create --label <label> --role admin|guest|named --permissions <csv>
```

P4.18 can rename these for CLI polish, but the behavior above is the contract
P4.5 should implement.

## Client-Side Per-Host Storage

Each client stores the raw bearer token under a host-specific key. The host id
comes from the agent and is included in the pairing payload and `/v1/me` or
capabilities response; it is not inferred from IP address, because LAN IPs and
Tailscale names can change.

CLI key:

```text
client.host-token.<agent-host-id>
```

The CLI also keeps non-secret host metadata in a normal config file: label,
base URL, API version last seen, and current host selection. A `--token` flag can
override storage for scripting, but stored tokens use the platform secret store
where available.

iOS Keychain key:

```text
service: com.camerontemple.MSCRemoteiOS
account: host-token.<agent-host-id>
```

iOS keeps the selected host id and base URL in UserDefaults. P4.19 may still
show a single selected host if the copied app has only one host screen in this
phase, but the storage shape must not keep the old single global
`remote_api_token` account as the only durable token slot.

## Revocation

Revocation is id-based, not token-string-based.

When an admin revokes `credential-id`, the agent:

- Marks or removes the non-secret registry entry.
- Deletes `remote-api.token.<credential-id>` from `SecretStore`.
- Rejects any future bearer token with that credential id as `401`.
- Writes an audit entry attributed to the admin token that performed the
  revocation.

Deleting an already-missing secret is still success, matching the `SecretStore`
contract. If registry and secret store drift, the safer answer wins: no
matching `SecretStore` record means the token cannot authenticate.

## Authentication Lookup

For every `/v1/` route except explicitly public health and pairing bootstrap
endpoints:

1. Parse `Authorization: Bearer <token>`.
2. Reject missing, malformed, empty, unknown-id, expired, revoked, or hash-mismatched
   tokens.
3. Apply the auth-failure rate limit before returning the response.
4. Return P2.4's `ErrorDTO` with `401 unauthorized`, or `429 rate_limited` when
   the failure limit has tripped.
5. On success, attach the credential id, role, label, and permissions to the
   request context for route permission checks and audit attribution.

The token comparison must use a constant-time equality check for the stored
hash result. The Phase 2 plain string comparison was explicitly acceptable only
because `MSC_DEV_TOKEN` was a throwaway loopback dev token.

## Rate Limit and Audit

MSC 1's existing behavior is the Phase 4 baseline:

- Auth failures: failures 1 through 10 from one client IP in a 60 second window
  return `401`; failure 11 and later return `429`.
- Sensitive POST routes: ten allowed requests from one client IP in a five
  second window; the eleventh returns `429`.

Phase 4 applies both to the lifecycle slice. The auth-failure limiter runs
before route handling. The POST limiter runs after authentication and permission
checks, matching MSC 1's ordering.

Audit labels:

| Case | Label |
|---|---|
| Missing token | `anonymous` |
| Malformed or unknown token | `unknown` |
| Valid token | credential registry label |
| Bootstrap-created owner token | `owner-admin` unless explicitly renamed |

Audit records are required for auth failures, forbidden requests, rate-limited
requests, token creation/revocation, and lifecycle mutations. Lifecycle route
records use the final response status, matching MSC 1's pending audit context
pattern rather than logging only request receipt.

## iOS Fresh-Install Bug

P2.20 found a real bug in the copied iOS app:
`KeychainTokenStore.loadToken()` returns an empty string when no Keychain item
exists, so this expression never reaches its fallback:

```swift
(try? KeychainTokenStore.loadToken()) ?? SettingsStore.devDefaultToken
```

Phase 4 removes the dev fallback instead of repairing it. A missing Keychain item
must mean "not paired"; it must not silently produce an empty token draft, and it
must not prefill `MSC_DEV_TOKEN`. After a successful pairing exchange, iOS saves
the returned token under `host-token.<agent-host-id>` and considers that host
paired.

Implementation requirement for P4.5/P4.19: make the token-loading API return
`nil` or throw a distinct not-found error for an absent item, and treat an empty
or whitespace-only stored token as invalid by deleting it and showing the
unpaired state.

## What D-012 Still Does Not Decide

This closes only D-012's Phase 4 CLI/iOS credential path and the per-host
credential key shape needed by those clients. Still open:

- How the Tauri shell proves same-machine origin to a local agent.
- How desktop pairs with a remote host.
- Whether off-loopback HTTP is ever allowed without TLS.
- How a local CA or certificate trust path works across platforms.
- Whether Tailscale changes any auth requirement.
- Browser cookie issuance, allowed origins, CSP, and CSRF.

Those remain Phase 11/client-networking questions unless a later Phase 4 step
finds a direct gate blocker.

## Phase 9 access-posture addendum

Approved by Cameron Temple on 2026-08-22: Phase 9 keeps the management API on
loopback by default and permits only an explicitly configured Tailscale
management path. Tailscale does not replace the bearer credential, expiry,
revocation, role, or permission checks defined above. General-LAN management
binding and its TLS certificate/trust design are unavailable until Phase 11;
so are remote desktop pairing, desktop-local automatic authorization, and
browser cookie/origin/CSP/CSRF mechanics. This addendum preserves this
document's per-host CLI/iOS credential model and does not turn named tokens
into per-person accounts.
