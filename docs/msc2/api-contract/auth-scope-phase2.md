# Phase 2's authentication surface — what's in, what's deferred

**Status: Proposed**, pending Cameron's confirmation during the Read move, per `rolling-plan.md`'s own note that this is "the single biggest judgment call in this phase's plan." D-012 is amended accordingly (still **Proposed** for everything outside its already-Approved core — this step narrows *what Phase 2 must build* toward that core, it does not close any of the six open gaps).

---

## 1. What Phase 2's gate actually requires

Per `msc2-port-plan.md` §3, Phase 2's exit criterion is: **"the existing iOS app connects and reads status against a stub agent."** Not a Tauri desktop app connecting to a remote host. Not a browser. One client, one transport, one machine:

- The iOS app running against an agent on the **same local network as the Mac it's paired with today** — in Phase 2's case, simplified further to **loopback** (`127.0.0.1:48400`, per P2.18/P2.20), since no real host is being provisioned yet and the skeletal agent runs on the developer's own machine.
- D-016 ("UI never gates correctness") argues against solving Tauri/browser auth just to satisfy an iOS-only gate — that's Phase 11's client, Phase 11's problem.

Everything below scopes Phase 2's auth work to exactly that one path.

## 2. MSC 1's baseline mechanism, read from source

D-012's "Approved core" describes iOS auth as "QR pairing → durable keychain token → bearer header." Read literally against MSC 1:

- **The token is not derived from a cryptographic exchange.** An admin or named token is created on the Mac side (`RemoteAPIServer+HTTP.swift`'s `TokenRole` / `RemoteAPISharedAccessEntry` in `AppConfig.swift:470`) and is the *same string* embedded in the pairing artifact — there is no separate ephemeral "pairing secret" that gets exchanged for a longer-lived token afterward.
- **The pairing artifact is a deep link**, built in `MSCSettingsView.swift:686` (`buildPairingLink(token:)`): `mscremote://pair?base=<http://host:port>&token=<token>`, rendered either as a QR code or copied as a link. The comment at `MSCSettingsView.swift:684` is explicit that a loopback fallback is disallowed here ("callers must never fall through to 127.0.0.1 (U1)") — that rule is about the *macOS pairing-link generator* offering a reachable LAN address to a *different* device, not about whether loopback is a valid transport in general; it doesn't apply to Phase 2's single-machine dev loop.
- **The iOS app stores the scanned token in Keychain** (`KeychainTokenStore.swift`, `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`) and sends it as `Authorization: Bearer <token>` on every request thereafter (confirmed in `RemoteAPIClient.swift` and verified server-side at `RemoteAPIServer+HTTP.swift:378-389`).
- **Verification is a flat dictionary lookup**, not a cryptographic check: `respond(to:clientFD:)` strips the `Bearer ` prefix, looks the presented string up in a `[String: TokenRole]` map, and treats an unknown or missing token as `401 unauthorized` (rate-limited to `429` after repeated failures from the same client IP, per `checkAndRecordAuthFail`).

This confirms the "minus the real pairing-secret exchange" framing in the step brief is accurate: what's actually missing for Phase 2 isn't a cryptographic protocol MSC 1 has and MSC 2 lacks — it's the **token-issuance and persistent-storage machinery** (creating named tokens, writing them to versioned config, exposing the pairing-link/QR UI, keeping the Mac-side token map in sync). That machinery depends on a config/secrets substrate (`SecretStore` trait, `msc2-engineering.md` §8) that is Phase 3 scope and does not exist yet.

## 3. In scope for Phase 2

1. **Bearer-token verification middleware in `msc-agent`.** Every route except `GET /v1/health` requires `Authorization: Bearer <token>`; a missing or wrong token gets P2.4's structured `ErrorDTO` 401 — the same status MSC 1 returns today, expressed in the new envelope.
2. **A single fixed dev token**, sourced from an environment variable (e.g. `MSC_DEV_TOKEN`), checked with a constant string comparison. Code comments mark this plainly as a development stand-in, not a preview of the real flow — matching P2.12's own description in `rolling-plan.md`.
3. **The iOS client re-pointed to send that fixed token** (P2.18/P2.20) — manually configured for Phase 2's purposes, not scanned through the real QR flow, since there is no real pairing-link generator on the Rust side yet.
4. **Loopback-only binding.** `msc-agent` binds `127.0.0.1` by default (`msc2-engineering.md` §10), so there is no LAN-exposure surface to secure this phase in the first place.

## 4. Explicitly deferred — not solved by this phase

Carried forward unchanged from D-012's six-item gap list; Phase 2 closes none of them:

1. **Local automatic authorization** (same-machine process impersonation) — unaffected either way by a fixed dev token; still open.
2. **Remote desktop pairing** (Tauri → remote host) — no desktop client exists yet; Phase 11.
3. **Per-host credential storage** — no `SecretStore` trait yet; Phase 3, consumed by later phases per-client.
4. **LAN TLS provisioning** — moot while binding is loopback-only; revisit whenever a phase turns LAN exposure on by default.
5. **Tailscale posture** — no Tailscale integration exists in the skeletal agent; unaffected.
6. **Browser origin policy / CSRF** — no browser client exists yet; Phase 11, and only relevant once cookie auth (D-012's other Approved leg) is implemented.

Also carried forward, not part of D-012's numbered list but adjacent and worth naming so it isn't mistaken for closed: **rate limiting and audit logging** on auth failures (MSC 1's `checkAndRecordAuthFail` / `AuditLogger`) are Phase 3 substrate work per `rolling-plan.md`'s Phase 2 "Not in this phase" note — Phase 2's dev-token check fails closed (401) but does not rate-limit or audit-log failures.

## 5. Why a fixed dev token is safe to accept here

The skeletal agent this phase builds touches no real server process and no real file (`rolling-plan.md`: "every handler this phase wires returns canned or in-memory data"). A hardcoded dev token guarding a loopback-only stub with no real mutation carries none of the risk a hardcoded token guarding a LAN-reachable agent with real servers behind it would — which is exactly the distinction Phase 3+ has to draw before any of this scoping could extend past the dev loop.
