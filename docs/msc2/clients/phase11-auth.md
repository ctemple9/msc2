# Phase 11 desktop and browser authentication contract

**Status:** Proposed contract, P11.21. It implements the already-approved
D-012 browser-cookie and desktop-local-token mechanisms in a form later steps
can build and test. Cameron's verification of P11.21 does not silently promote
the remaining D-012 design detail to owner-approved status.

## What this protects

MSC management is never open merely because a request came from the same
computer, a tailnet, or an MSC-looking page. Every management request is one
of two authenticated forms:

- A bearer credential, used by the CLI, iOS, and the Tauri desktop backend.
- A browser session cookie, issued only after a short-lived pairing code has
  been exchanged by the agent-served frontend.

The browser never receives a bearer credential. The Svelte page never receives
a desktop credential. Raw bearer credentials occur only in the single Tauri
backend response that creates them, then go directly into the platform secret
store.

The existing `msc2_<credential-id>_<secret>` bearer format, durable registry,
expiry, revocation, permission checks, and audit rules in
`docs/msc2/lifecycle/pairing-phase4.md` remain the authority for bearer
credentials. This document adds session records; it does not create a second
permission model.

## Transport boundary

The normal management bind remains loopback-only. An administrator may opt in
to the Phase 9 Tailscale management bind; that is the sole off-loopback path
in v1. General-LAN management, arbitrary off-loopback HTTP, local certificate
issuance, a local CA, and browser certificate-warning workarounds are not
available.

Tailscale encrypts the network path but is not identity. Requests on that path
still require the same bearer credential or browser session and then the same
permission check. Player-facing listeners, including Playit and resource-pack
hosting, never proxy management traffic.

The agent has one canonical browser origin for each enabled management bind:
the loopback origin or the explicitly configured Tailscale origin. It serves
the frontend only from those origins. The API sends no permissive CORS headers;
cross-origin browser requests are refused. A request with an `Origin` header is
accepted only when it exactly equals the canonical origin that received it;
missing `Origin` remains valid for non-browser bearer clients.

The agent adds this Content-Security-Policy to the served frontend:

```text
default-src 'self'; base-uri 'none'; object-src 'none'; frame-ancestors 'none';
form-action 'self'; script-src 'self'; style-src 'self'; img-src 'self' data: blob:;
connect-src 'self'; worker-src 'self' blob:
```

It also sends `X-Content-Type-Options: nosniff` and `Referrer-Policy:
no-referrer`. Auth responses are `Cache-Control: no-store`. The policy has no
third-party script, iframe, or network exception; future additions must be
explicitly reviewed rather than weakening the default.

## Pairing codes and browser sessions

An authenticated administrator creates a one-use pairing code with
`POST /v1/auth/pairings`. The code is 256 bits of random data, shown once,
stored only as a verifier, bound to the requested client kind and permission
grant, expires after 10 minutes, and is consumed before credential/session
creation. Creation, expiry, failed redemption, and redemption are rate-limited
and audit-recorded. A code cannot be retried, changed from browser to desktop,
or used to broaden its original permissions.

The browser submits a browser-bound code to
`POST /v1/auth/browser-sessions` from the agent-served same-origin page. A
successful exchange returns `204` and sets `msc2_session`: `HttpOnly`,
`SameSite=Strict`, `Path=/v1`, and a bounded `Max-Age`. It uses `Secure` when
the canonical origin is HTTPS. The one supported non-HTTPS case is the
loopback or Tailscale transport selected above; neither creates a general-LAN
cookie path. The cookie value is an opaque 256-bit session secret and is stored
server-side only as a verifier.

A session is tied to the credential created for the pairing code. It has an
8-hour idle lifetime and a 30-day absolute lifetime; successful use may renew
the idle deadline but never the absolute deadline. The agent stores the session
record durably, so restart does not turn revocation into a best-effort promise.
Logging out with `DELETE /v1/auth/browser-sessions/current`, credential
revocation, expiry, or session-verifier deletion makes the cookie immediately
unauthorized. The logout endpoint clears the browser cookie too.

`GET /v1/auth/csrf` returns the authenticated session's opaque CSRF token with
`Cache-Control: no-store`. Every cookie-authenticated non-GET/HEAD/OPTIONS
request must carry that exact token in `X-MSC-CSRF`; it is checked after exact
origin validation and before route mutation. The token is rotated on session
creation and invalidated with the session. A valid `Authorization: Bearer …`
credential is deliberately exempt from this header requirement, even if a
browser cookie is also present; bearer authentication takes precedence. This
gives a browser page enough information to protect its own requests without
putting a credential in JavaScript-readable storage.

## Desktop credentials, one host at a time

For a remote host, the Tauri backend redeems a desktop-bound pairing code at
`POST /v1/auth/desktop-pairings`. The raw bearer credential is returned once to
the Rust shell backend, not to Svelte. It is stored under
`msc.desktop.host-token.<agent-host-id>` in the platform credential store. The
host id comes from the pairing result, not the URL, so a changed LAN address or
Tailscale name cannot cause one host's credential to be reused for another.
The backend exposes only authenticated request operations to the shared client;
it never exposes a “read token” command.

Same-machine desktop startup uses a local-bootstrap channel, not an unauthenticated
loopback HTTP exception. During an approved desktop installation, the installer
registers the signed desktop package identity and a non-exportable installation
key with the local agent. The Tauri Rust backend proves possession of that key
over a one-use, short-lived challenge on an OS local-IPC endpoint. The agent
also verifies the connecting process identity: macOS code-signing identity,
Windows signed package identity, or the Linux package-managed executable
identity. The proof binds the agent host id, challenge, package identity, and
protocol version. A process that merely knows a loopback port cannot mint a
credential; a copied binary, stale challenge, wrong host, or unregistered
package identity fails closed. The successful backend stores its resulting
host-scoped bearer credential using the same key above.

## Host reset recovery

The host-local agent command `msc pairing create` is the recovery handoff
after a host reset. It creates a desktop- or browser-bound administrator
pairing code with a ten-minute lifetime and prints the code once to the local
terminal; the agent does not log it. The remote desktop's Pair Again flow or
the browser's Add Host flow redeems that code through the ordinary one-use
pairing routes. A reset revokes the previous credentials and pairing records,
rotates the agent host id, and requires a new pairing; it never grants a
remote client control of the operating-system service registration.

The local IPC endpoint is never exposed through HTTP, Tailscale, a webview
command, or a URL. If a platform cannot prove the installed package identity
and key protection at runtime, automatic bootstrap is unavailable and the
desktop uses the ordinary remote-pairing code flow—even for the local host.
That is intentionally less convenient than pretending same-user process
identity is a security boundary.

## Versioned API additions and failure rules

P11.21 adds only these `/v1` routes. Existing routes inherit the contract-wide
“bearer or browser session” authentication rule; their permission categories do
not change.

| Route | Authentication | Result |
|---|---|---|
| `POST /v1/auth/pairings` | admin bearer | One-use, 10-minute pairing code |
| `POST /v1/auth/browser-sessions` | exact same-origin browser request and code | `204` plus httpOnly session cookie |
| `GET /v1/auth/csrf` | browser session | session CSRF token |
| `DELETE /v1/auth/browser-sessions/current` | browser session + CSRF | revoke current session and clear cookie |
| `POST /v1/auth/desktop-pairings` | code in Tauri backend | one host-scoped bearer credential, shown once |

All refusals use `ErrorDTO`: `401 unauthorized` for missing, expired, revoked,
or wrong credentials; `403 forbidden` for a wrong origin, missing/bad CSRF
token, or insufficient permission; `409 pairing_consumed`; `410 pairing_expired`
or `session_expired`; and `429 rate_limited`. Responses never echo a pairing
code, session cookie, CSRF token, or bearer secret in an error, audit record,
log, URL, or browser storage.

## Acceptance checks for P11.22 and P11.23

P11.22 must prove a hostile origin cannot redeem or use ambient cookie authority,
CSRF is required for every cookie-authenticated mutation, expiration and
revocation survive restart, and bearer clients continue to work without CSRF.
P11.23 must prove a local unregistered process cannot pass bootstrap, a remote
desktop stores credentials per returned host id, revocation/expiry applies,
and host switching cannot reuse a different host's credential. Both steps must
exercise the routes and fields frozen in `openapi.json`, not a private parallel
protocol.
