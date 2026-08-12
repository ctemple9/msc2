# Legacy plaintext secret transition

Documents `migrate_legacy_secrets` (`crates/msc-infrastructure/src/config_repository.rs`,
P5.8) — the source-parity half of `ConfigManager.swift`'s one-time
migration (`init`, lines 73-99). No MSC 1 XCTest exercises this path
directly; `fixtures/secret-migration/`'s 5 cases were characterized
straight from source, the same precedent `fixtures/config-recovery/`
(P5.7) set.

## What MSC 1 actually migrates

Two plaintext keys, read by literal name out of the raw JSON dict — never
by decoding into `AppConfig` first, since `AppConfig`'s `CodingKeys`
already excludes both (P5.4/P5.5):

| Legacy JSON key | Where | Migrates to (Keychain in MSC 1) |
|---|---|---|
| `remote_api_token` | top level | `KeychainManager.shared.writeRemoteAPIToken(oldToken)` |
| `xbox_broadcast_alt_password` | per entry in `servers[]`, keyed by that entry's `id` | `KeychainManager.shared.writeXboxBroadcastAltPassword(oldPassword, forServerId:)` |

There is no guest-token counterpart. `remote_api_token` is the one
credential MSC 1's Remote API ever authenticated with — not a two-tier
owner/guest scheme — so this migration does not invent a second input to
handle (`docs/msc2/config-migration/phase5-scope.md` §"Secret migration"
already flags this as a mistake earlier plan drafts made).

A value only migrates when it's non-blank after trimming whitespace
(source lines 80 and 95); the *raw, untrimmed* string is what gets
written, not the trimmed form. A server entry with no `id` has its
password key dropped but nothing migrated for it — there's no id to key
the secret under (source line 93's `guard let serverId = ...`).

## Where the strip actually happens in MSC 1 — and why this port differs

In source, this migration step never removes either key itself. It only
writes forward into Keychain. The key is dropped later, as a side effect
of the immediately-following `JSONDecoder().decode(AppConfig.self, ...)`
plus `save()` — `AppConfig` never had `remote_api_token` or
`xbox_broadcast_alt_password` in its `CodingKeys` to begin with, so they
simply don't survive that round trip.

`migrate_legacy_secrets` runs on a raw `serde_json::Value`, before any
typed `AppConfig` decode exists in this crate's config-loading pipeline.
To reach the same on-disk end state, it strips both keys itself rather
than relying on a later decode to do it. This is a documented deviation in
mechanism, not in observable behavior: the rewritten config this function
returns is the same shape `AppConfig::encode()` (P5.4) would produce
regardless.

## Contract

- Pure adapter over an explicitly supplied `Value` and a `&dyn SecretStore`
  — it never discovers or opens MSC 1's (or MSC 2's) application-support
  path itself. Callers own locating the real config file.
- Blank values are ignored (left unmigrated), independently of whether the
  *other* legacy key is blank, present, or absent — the owner token and
  each per-server password all migrate independently of one another.
- Feeding already-clean input (neither legacy key present) back through is
  a no-op: nothing is read out of `config`, nothing is written to
  `secrets`, and the returned config is unchanged.
- Storage keys match `docs/msc2/substrate/secret-storage.md` §9, already
  reserved there since Phase 3: `remote-api.owner-token` and
  `xbox-broadcast.alt-password.<server-id>`.

## What this step deliberately does not do — the P5.9 boundary

`remote-api.owner-token` is a holding key for the raw legacy token alone.
It is **not** a bearer secret Phase 4's auth middleware understands: per
`crates/msc-agent/src/auth.rs`, a request authenticates via
`msc2_<credential-id>_<secret>`, verified against
`remote-api.token.<credential-id>` — a different key namespace, scoped per
credential, that only exists once a credential has actually been issued
(`CredentialStore::issue_credential`).

So the raw legacy owner token this step migrates is inert on its own — it
cannot authenticate anything yet. P5.9 ("make the migrated owner
credential durable and authenticating") is what reads
`remote-api.owner-token`, generates one new credential id, stores a salted
verifier under `remote-api.token.<new-id>` using the old token as the
verifier's secret component, persists a non-secret admin registry entry
for that credential, and returns the replacement bearer
`msc2_<new-id>_<old-token>` once. This step stops before any of that —
its output is a config with the plaintext gone and the raw token sitting
in `remote-api.owner-token`, waiting for P5.9 to pick it up.
