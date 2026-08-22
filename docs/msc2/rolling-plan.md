# MSC 2 — Rolling Plan

> ## STATUS: Phase 9 in progress — P9.1 through P9.12 are DONE, and the helper-acquisition amendment's P9.6a and P9.7a are DONE. The amendment's checksum QUESTION was **answered and then amended** (2026-08-22): verification is absolute for every helper, but the hash is repository-pinned for `playitd`/MCXboxBroadcast and upstream-published for Geyser/Floodgate. Remaining: P9.6b, P9.10a, P9.11a, then P9.13–P9.15.
> **Next move:** EXECUTE the amendment's remaining steps in order — P9.6b, P9.10a, P9.10b, P9.11a — then P9.13–P9.15. Both amendment QUESTIONs are answered and no decision entry is outstanding: only `playitd` is pinned, and Geyser and Broadcast keep MSC 1's resolve-latest behavior with upstream-published checksums added.
> **Repo:** https://github.com/ctemple9/msc2 · GitHub Actions run [32544701401](https://github.com/ctemple9/msc2/actions/runs/32544701401) is green for exact Phase 8 code candidate `3e04f484bdbee3e821ea55dda6a06cc8e8f5c887`, including repository invariants, macOS, Linux, Windows, and the headless no-GUI link check.
> **Last updated:** 2026-08-21

**Previous phases (Setup, Phase 0 through Phase 8) and their amendments have moved to `rolling-plan-archive.md`** to keep this file small. That archive is historical only — current status and active work stay here.

---

## How this document works

This is the **working state** of the build. The vision documents say where we're going; the port plan says in what order; this file says **where we actually are right now**.

Phases are fixed and come from `msc2-port-plan.md`. **Steps are written one phase at a time**, as we reach each phase — not up front. Steps written today for Phase 9 are the current planning task.

Each phase runs the six-move loop in `CLAUDE.md`: Plan → Read → Execute → Verify → Review → Advance.

### Step format

Every step looks like this:

```
### P0.3 — Extract TPS parser fixtures
**Status:** not started | in progress | awaiting verification | DONE
**Files:** fixtures/tps/, tools/extract-fixtures/
**What:** Pull the 27 TPS test cases out of MSC 1's TpsMonitoringTests.swift
         into input/expected JSON pairs.
**Verify:** `ls fixtures/tps/*.json | wc -l` → 27
**Commit:** P0.3: extract TPS parser fixtures        <- the message, not a hash
```

Every step also carries a **Batch:** field, telling an agent whether it may be run unattended:

| Batch value | Meaning |
|---|---|
| `safe` | Mechanical, and its Verify is a script Cameron has already reviewed. Batch freely. |
| `stop-after` | Runnable in a batch, but the batch **ends here** — the result needs looking at before continuing. |
| `solo` | Judgment work or a new checker script. Run it alone. Its output needs a cross-check by the other agent before the phase closes. |

**Status is only moved to DONE by Cameron**, after he runs the Verify command himself. An agent may set it to *awaiting verification* and stop.

**A step whose Verify only counts things is `stop-after` at best.** Counting proves something exists, not that it is right.

---

## Phases

Gates are in `msc2-port-plan.md`. This is the map, not the detail.

| Phase | Name | State |
|---|---|---|
| **Setup** | Repo, docs, agent instructions, CI, editor config | complete |
| **0** | Freeze the baseline and build the harness | complete |
| 1 | Domain types and pure rules | complete |
| 2 | API contract and operation model | complete |
| 3 | Safety substrate | complete |
| 4 | Java lifecycle vertical slice | complete |
| 5 | Configuration and migration | complete |
| **6** | Worlds and backups | complete |
| **7** | Server families and provisioning | complete |
| **8** | Mods, plugins, modpacks | complete |
| **9** | Networking and helpers | **planned — PLAN next** |
| 10 | Bedrock runtimes | not started |
| 11 | Desktop and web clients | not started |
| 12 | Terminal UI (deferred from v1) | not started |

---

## Phase 6 — Worlds and backups

All 51 steps (P6.1–P6.51) are `DONE`, and Codex's gate review (2026-08-18) confirms the gate holds on exact candidate `8568dea`. The full record — scope, characterization, world/backup services, public-client wiring, gate-review corrections, exact-commit tri-platform CI proof, and the review itself — has moved to `rolling-plan-archive.md`.

---

## Phase 7 — Server families and provisioning

All 38 steps (P7.1–P7.38) are `DONE` per Cameron. The final exact candidate is `79d5044` on `phase7-corrections`; GitHub Actions run [32448912726](https://github.com/ctemple9/msc2/actions/runs/32448912726) is fully green across macOS, Linux, Windows, the Phase 7 provisioning/launch smoke, and the headless no-GUI check. The full record — scope, characterization, six-family providers and provisioning, runtime and installer behavior, diagnostics and repairs, review corrections, evidence, and amendments — has moved to `rolling-plan-archive.md`.

---

## Phase 9 — Networking and helpers

Phase 9 makes player-facing connection helpers and the already-designed
named-token model real without exposing the MSC management API to the public
internet. It must preserve the sharp distinction in `msc2-engineering.md` §10:
Playit, DuckDNS, Geyser, Floodgate, Xbox Broadcast, and resource-pack hosting
serve Minecraft players; MSC administration remains loopback, LAN, or
Tailscale with token authentication. The port plan has no standalone Phase 9
exit-criteria paragraph, so P9.1 must state the evidence-based working gate
and identify any owner decision that cannot be inferred from MSC 1.

### P9.1 — Freeze Phase 9 scope, source inventory, and working gate

**Status:** DONE
**Files:** `docs/msc2/networking/phase9-scope.md`, `docs/msc2/rolling-plan.md`
**What:** Read every Phase 9-relevant MSC 1 implementation, test, route, DTO, and configuration field; record the symbol-level disposition, the behavior that MSC 1 proves, and the cross-platform behavior MSC 2 must newly define. State a working gate covering all Phase 9 deliverables, public API/CLI/iOS reachability, durable credential revocation across restart, cancellation/recovery of long-running helpers, and tri-platform/headless proof. Preserve the explicit management-port boundary; do not turn a player-connectivity feature into public MSC administration. List only genuinely unresolved D-012 choices as owner questions with a recommendation and downstream consequence.

Four corrections from the cross-check, to carry into the scope note rather than re-derive:

- **DuckDNS has no token or update call in MSC 1.** `AppConfig.swift:557`'s `duckdnsHostname` is a plain display label the user types in and points at a DuckDNS updater they run themselves — there is no `duckdns.org/update` call, no stored token, anywhere in the source (`AppViewModel+ServerSettings.swift:310`'s `saveDuckDNSHostname` just stores the string and re-syncs Xbox Broadcast config). **Decided for scope purposes:** P9.9 ports the label-only behavior MSC 1 actually has, not a token-based dynamic-DNS updater. A real DuckDNS API updater is optional future work, not a Phase 9 blocker — flag it in the scope note as a possible extension and let Cameron override during the Read move if he wants it built now.
- **MSC 1's actual notification content is player join/leave and server start/stop** (`AppViewModel+Notifications.swift`, `ServerNotificationEvent`), delivered via native `UNUserNotificationCenter` — confirmed by the symbol ledger's own row 16, which already disposes the delivery mechanism as **client**-owned and implies the agent's job is emitting the underlying events. Record this as the real port target for the "notifications" bullet in `msc2-port-plan.md` §3's Phase 9 line, alongside (not replaced by) any new helper/connectivity notification events P9.11 adds.
- **`GET /v1/connectivity` already exists**, frozen from Phase 2/P0.30 (`openapi.json`, `ConnectivityResponseDTO`) — it's the direct contract target for MSC 1's `connectivitySnapshot` (`AppViewModel+HealthCards.swift:1022`: playit → DuckDNS → public-IP priority, port reachability, playit/broadcast state). Record that Phase 9 implements the body behind this already-promised route rather than inventing a parallel one.
- **The first-run two-pass orchestration** (`AppViewModel+ServerControls.swift`, starting around line 899: `beginInitiationProgress`, `startInitiationPass2`, `scheduleInitiationPlayitWatchdog`, `armInitiationBroadcastTechTimeout`) holds server-creation completion open until Playit/Broadcast transports come up, with real timeouts (~75s/~60s) and a safety cap. Phase 7's own gate record deferred Playit/Broadcast bring-up to Phase 9 ("Phase 7 never downloads a helper"), so this sequencing lands here too — record it explicitly rather than leaving it implicit.

**Verify:** `git diff --check && test -f docs/msc2/networking/phase9-scope.md && rg -n 'working gate|management|SecretStore|D-012|duckdns_label_only|connectivitySnapshot|initiation' docs/msc2/networking/phase9-scope.md`
**Commit:** `P9.1: define networking and helpers scope`
**Batch:** solo

### P9.2 — Capture Phase 9 characterization fixtures and live evidence

**Status:** DONE
**Files:** `fixtures/networking/`, `fixtures/helper-lifecycle/`, `fixtures/credentials/`, `docs/msc2/networking/evidence/`, `docs/msc2/networking/phase9-scope.md`
**What:** Before translating behavior, extract exactly 14 language-neutral networking fixtures, 8 helper-lifecycle fixtures, and 8 credential fixtures covering Playit status/error handling, DuckDNS request/response interpretation, resource-pack URL and SHA-1 rules, port-diagnostic outcomes, Xbox Broadcast prompts and status, Geyser/Floodgate detection and configuration, helper startup/exit/restart behavior, and named-token CRUD/revocation. Capture reproducible live evidence only where a third-party integration can be exercised safely; record unavailable cases honestly rather than inventing successes. Extend the scope note with fixture provenance and the exact behavior that has no MSC 1 oracle.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/networking --expect 14 && python3 tools/fixture-runner/run.py --validate-dir fixtures/helper-lifecycle --expect 8 && python3 tools/fixture-runner/run.py --validate-dir fixtures/credentials --expect 8`
**Commit:** `P9.2: capture networking helper fixtures`
**Batch:** solo

### P9.3 — Resolve the remaining Phase 9 credential and remote-access posture

**Status:** DONE
**Files:** `docs/msc2/networking/phase9-scope.md`, `docs/msc2/msc2-decisions.md`, `docs/msc2/api-contract/auth-scope-phase2.md`, `docs/msc2/lifecycle/pairing-phase4.md`
**What:** Turn the P9.1 evidence into a narrowly scoped D-012 decision record: which of remote desktop pairing, per-host credential persistence, off-loopback TLS, Tailscale, browser origins, and CSRF is implemented in Phase 9 versus explicitly deferred to Phase 11. Do not silently choose an unresolved security posture. If Cameron’s approval is required, prepare the required plain-language question and stop; after an answer, record it with its rationale and testable security invariant. Keep named-token `/users` CRUD separate from per-person identity, which remains a v1 non-goal.
**Verify:** `git diff --check && rg -n 'Phase 9|D-012|deferred|approved|owner question' docs/msc2/networking/phase9-scope.md docs/msc2/msc2-decisions.md`
**Commit:** `P9.3: record Phase 9 access posture`
**Batch:** solo

### P9.4 — Freeze the Phase 9 API and capability contract

**Status:** DONE
**Files:** `docs/msc2/networking/phase9-api.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/api-contract/websocket-v1.json`, `docs/msc2/client-capability-matrix.csv`, `crates/msc-api/tests/phase9_conformance.rs`
**What:** Map every supported Phase 9 action to an additive, versioned route and DTO contract before application code exists: player-network status/configuration, resource-pack hosting, helper operations, notifications, Geyser/Floodgate, and named-token list/create/update/revoke. Declare permission categories, operation/cancellation semantics, secret-redaction rules, help identifiers where a response explains a user-facing state, and which contract elements are intentionally delayed by P9.3. Update the capability matrix without claiming a client surface exists before it does. **`GET /v1/connectivity` (`ConnectivityResponseDTO`) already exists in the frozen contract from Phase 2/P0.30** — extend that existing schema for port-diagnostic and reachability fields instead of adding a parallel route; P9.9's application service implements its body.
**Verify:** `python3 tools/api-contract-check.py --v1-summary && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && cargo nextest run -p msc-api --test phase9_conformance && rg -n 'ConnectivityResponseDTO' docs/msc2/api-contract/openapi.json`
**Commit:** `P9.4: freeze networking and helpers contract`
**Batch:** solo

### P9.5 — Port pure network and helper status rules

**Status:** DONE
**Files:** `crates/msc-domain/src/networking.rs`, `crates/msc-domain/src/helper.rs`, `crates/msc-domain/src/lib.rs`, `crates/msc-domain/tests/networking.rs`, `crates/msc-domain/tests/helper.rs`, `fixtures/networking/`, `fixtures/helper-lifecycle/`
**What:** Implement the fixture-backed, side-effect-free rules from P9.2: safe player address presentation, resource-pack metadata validation, provider/helper status classification, diagnostic result vocabulary, and helper lifecycle transition rules. Keep raw credentials, private addresses where masking is requested, and provider-specific process details outside domain display types.
**Verify:** `cargo nextest run -p msc-domain --test networking --test helper`
**Commit:** `P9.5: add networking helper domain rules`
**Batch:** safe

### P9.6 — Build the managed helper-process foundation

**Status:** DONE
**Files:** `crates/msc-infrastructure/src/helper_process.rs`, `crates/msc-infrastructure/src/process.rs`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/tests/helper_process.rs`, `fixtures/helper-lifecycle/`
**What:** Add one bounded, supervised helper-process abstraction for the Phase 9 programs rather than bespoke subprocess ownership per integration. It must preserve output framing, record readiness/failure/exit, prevent duplicate helpers for the same server/function, support graceful stop then forced termination, retain bounded diagnostics, and recover honestly after agent restart. Reuse the Phase 4 process and operation-journal boundaries; do not weaken server-process ownership.
**Verify:** `cargo nextest run -p msc-infrastructure --test helper_process`
**Commit:** `P9.6: add managed helper process support`
**Batch:** solo

### P9.7 — Add Playit tunnel lifecycle and secret handling

**Status:** DONE
**Files:** `crates/msc-infrastructure/src/playit.rs`, `crates/msc-application/src/playit.rs`, `crates/msc-application/tests/playit.rs`, `crates/msc-agent/tests/playit_routes.rs`, `fixtures/networking/`
**What:** Port Playit configuration, tunnel status, start/stop/update behavior, and player-facing connection details through the managed-helper foundation. Store its secret only through `SecretStore`, redact it from status, logs, audit records, exports, and API responses, and make network work an operation with cancellation and restart recovery. Treat a tunnel as Minecraft transport only; it must never make the agent’s management port public. Expose a bounded "tunnel became ready" signal (MSC 1's own creation-time watchdog waits ~75s before giving up) — P9.13 needs it to reproduce MSC 1's first-run orchestration.
**Verify:** `cargo nextest run -p msc-application --test playit && cargo nextest run -p msc-agent --test playit_routes`
**Commit:** `P9.7: add Playit tunnel lifecycle`
**Batch:** stop-after

### P9.8 — Add resource-pack hosting and transactional pack publication

**Status:** DONE
**Files:** `crates/msc-infrastructure/src/resource_pack_store.rs`, `crates/msc-application/src/resource_packs.rs`, `crates/msc-application/tests/resource_packs.rs`, `crates/msc-agent/tests/resource_pack_routes.rs`, `fixtures/networking/`
**What:** Implement resource-pack upload/import, SHA-1 calculation, hosted URL construction, server.properties mutation, replacement rollback, disable/remove behavior, and bounded serving according to the P9.4 contract. Stage bytes before publication, validate paths and size, preserve the prior working configuration on failure, and make the public pack endpoint serve only an approved file rather than an arbitrary server path.
**Verify:** `cargo nextest run -p msc-application --test resource_packs && cargo nextest run -p msc-agent --test resource_pack_routes`
**Commit:** `P9.8: add resource pack hosting`
**Batch:** stop-after

### P9.9 — Add DuckDNS hostname handling and the `/v1/connectivity` diagnostics behind it

**Status:** DONE
**Files:** `crates/msc-infrastructure/src/duckdns.rs`, `crates/msc-infrastructure/src/port_diagnostics.rs`, `crates/msc-application/src/network_diagnostics.rs`, `crates/msc-application/tests/network_diagnostics.rs`, `crates/msc-agent/tests/network_diagnostic_routes.rs`, `fixtures/networking/`
**What:** MSC 1's DuckDNS feature is a plain hostname label (`AppConfig.swift:557`'s `duckdnsHostname`) the user sets and manages their own updater for — there is no token and no `duckdns.org` API call anywhere in MSC 1. Port that: store/validate the hostname as ordinary (non-secret) configuration, and reuse it wherever MSC 1 does (Xbox Broadcast host resolution, connection-info display). A real token-based DuckDNS updater is out of scope for this step; note it as available future work rather than building it now. Then implement the port-diagnostic probes (`checkPortReachability`/`probeLocalPort`/`queryServerStatus` in `AppViewModel+HealthCards.swift`) and compose them, DuckDNS, and playit/broadcast state into the **existing** `GET /v1/connectivity` contract (`ConnectivityResponseDTO`, frozen at Phase 2/P0.30) rather than a new route — this is MSC 1's own `connectivitySnapshot` workflow. Make provider calls cancellable and bounded, and distinguish a provider failure from a closed or unreachable Minecraft port.
**Verify:** `cargo nextest run -p msc-application --test network_diagnostics && cargo nextest run -p msc-agent --test network_diagnostic_routes && rg -n 'ConnectivityResponseDTO' crates/msc-agent/tests/network_diagnostic_routes.rs`
**Commit:** `P9.9: add DuckDNS hostname handling and connectivity diagnostics`
**Batch:** stop-after

### P9.10 — Add Geyser and Floodgate management

**Status:** DONE
**Files:** `crates/msc-application/src/geyser.rs`, `crates/msc-application/tests/geyser.rs`, `crates/msc-agent/tests/geyser_routes.rs`, `fixtures/networking/`, `docs/msc2/client-capability-matrix.csv`
**What:** Complete the Phase 7 provisioning placeholder with managed Geyser/Floodgate installation/update detection, compatibility/configuration validation, Bedrock-facing address/status reporting, and safe mutation of the relevant server files. Reuse the Phase 8 managed-plugin rules where they apply, retain the existing exclusion from client-mod export, and report unavailable update information honestly rather than presenting these helpers as ordinary add-ons.
**Verify:** `cargo nextest run -p msc-application --test geyser && cargo nextest run -p msc-agent --test geyser_routes`
**Commit:** `P9.10: add Geyser and Floodgate management`
**Batch:** stop-after

### P9.11 — Add Xbox Broadcast lifecycle and notifications

**Status:** DONE
**Files:** `crates/msc-infrastructure/src/xbox_broadcast.rs`, `crates/msc-application/src/xbox_broadcast.rs`, `crates/msc-application/src/notifications.rs`, `crates/msc-application/tests/xbox_broadcast.rs`, `crates/msc-agent/tests/xbox_broadcast_routes.rs`, `fixtures/networking/`, `fixtures/dto-contract/`
**What:** Port the Xbox Broadcast helper’s staged download, configuration, account-prompt/status, supervised lifecycle, and secret migration/use. Keep passwords and account tokens in `SecretStore`, constrain logs to non-secret status, and expose a bounded "broadcast became ready" signal (MSC 1's creation-time watchdog waits ~60s once authenticated) for P9.13's first-run orchestration. Build the `notifications` service around MSC 1's **actual** notification content — `ServerNotificationEvent`'s four cases (server started, server stopped, player joined, player left), per `AppViewModel+Notifications.swift` and symbol-ledger row 16, which disposes native delivery as client-owned and the event source as agent-owned: the agent emits these as WebSocket/notification-feed events, clients render them as local OS notifications. Helper-crash and connectivity-change notifications are additive new event types on top of that real baseline, not a replacement for it.
**Verify:** `cargo nextest run -p msc-application --test xbox_broadcast && cargo nextest run -p msc-agent --test xbox_broadcast_routes && rg -n 'ServerStarted|ServerStopped|PlayerJoined|PlayerLeft' crates/msc-application/src/notifications.rs`
**Commit:** `P9.11: add Xbox Broadcast and notifications`
**Batch:** stop-after

### Amendment — helper-binary acquisition (raised 2026-08-22 from live MSC 1 evidence)

Running MSC 1's Playit setup surfaced a defect class that Phase 9's own scope note
(`docs/msc2/networking/phase9-scope.md`) already assigned to this phase but the executed
steps did not deliver: **acquiring a managed helper's binary**. The scope note says the
Playit "binary download/install shape is also agent work", that Geyser "downloading,
updates, compatibility checks" remain Phase 9 work, and that Broadcast must "maintain the
JAR library, staged download". None of the three was built.

MSC 1's observed failure, for the record:

```
[Playit] Failed to start: No 'playitd' asset found in the latest MSC GitHub release.
[Playit] Java tunnel create timed out — agent didn't come online in time.
```

Cause: `PlayitBinaryManager.swift` resolves `/releases/latest` and looks for a `playitd`
asset. The binary lives on its own tag (`playitd-v1.0.10`), so every app release since
v1.14 has shadowed it. Two distinct defects — an unpinned, unverified acquisition, and a
dishonest failure in which the real error was masked 29 seconds later by a readiness
timeout.

The four steps below replace the three drafted earlier. The acquisition mechanism is built
**once**, as P9.6a on the managed-helper foundation, and consumed by three thin steps —
rather than reimplemented per helper, which is what the earlier draft would have produced.

### QUESTION — before P9.6a — **ANSWERED**

**Answer: (a) — no artifact runs unless its bytes match a published SHA-256.** Confirmed by
Cameron Temple, 2026-08-22. **Amended 2026-08-22** — see "Amendment: two rules, not one" below.

The invariant is absolute and applies to every managed helper: the agent verifies a
SHA-256 before an artifact is staged or spawned, and refuses anything that does not match.
There is no best-effort path.

### Amendment: two rules, not one

The original answer said the hash is recorded "in this repository … for all managed helpers
without exception." That was written on a wrong assumption — that Geyser might publish no
checksum. **It does.** `download.geysermc.org/v2` serves a per-build, per-artifact `sha256`
for every platform JAR. The strict-verification-versus-unverified-executables trade-off the
question posed was therefore a false choice for Geyser, and the uniform rule would have
bought nothing while costing a great deal (see P9.10a).

Two rules, split by whether the helper is coupled to Minecraft's protocols:

| Helper | Release stream | Chases | Rule |
|---|---|---|---|
| `playitd` | **ours** — `ctemple9/minecraft-server-controller`, shared with unrelated app releases | nothing; a tunnel daemon | **Pinned**, hash computed and committed by us |
| Geyser / Floodgate | upstream, ~weekly builds | Java MC version **and** Bedrock protocol | **Resolve latest, verify the `sha256` the Geyser API publishes** |
| MCXboxBroadcast | upstream, ~every 9 days | Xbox Live auth **and** Bedrock protocol | **Resolve latest, verify the `digest` the GitHub releases API publishes** |

**The rule, stated generally: pin what we own; resolve-and-verify what chases a moving
third-party target.** `playitd` is the only helper we publish, so it is the only one we pin.

**Amended again 2026-08-22.** MCXboxBroadcast was first placed in the pinned bucket on the
claim that it "updates rarely." That was a guess and it was wrong — tags 139→150 span
2026-05-05 to 2026-08-20, two of them fifteen minutes apart. It tracks Xbox Live and the
Bedrock protocol, neither of which we control, and GitHub serves a `sha256` digest per
release asset. It belongs with Geyser.

Geyser is genuinely different and a fixed pin cannot serve it. Geyser versions on its own
scheme (2.1.0 → 2.11.2), not by Minecraft version, and keeps no per-MC-version branches, so
no single pin is correct for every server. Worse, **Bedrock clients auto-update through the
app stores** — when Mojang ships a Bedrock protocol change, only newer Geyser builds speak
it, and a repository pin would lock every MSC user's Bedrock players out of crossplay until
we cut a release. A helper that goes stale for reasons unrelated to the user's server cannot
be pinned in our release cycle.

MSC 1 reached the same conclusion independently: `PluginDownloader.swift` resolves
`versions/latest/builds/latest` from the Geyser API rather than pinning. P9.10a ports that
resolution as-is and adds only the checksum.

**What the originating defect actually taught us**, stated precisely, because "never use
`latest`" is the wrong lesson: `latest` broke for `playitd` not because it is dynamic, but
because the release stream it queried is **ours and shared** — app releases v1.14 onward
shadowed the `playitd-v1.0.10` tag, so `latest` began returning something that was never
the helper. A dedicated upstream release stream that publishes only that helper does not
have this failure mode. Hence the rule above: the question is who owns the stream and
whether what it tracks moves outside our control, not whether resolution is dynamic.

Provenance is recorded, never inferred: `HelperArtifactMetadata` carries which of the two
sources vouched for the bytes (see P9.6b), so an audit can always answer *who* stood behind
a given artifact.

**Accepted consequences**, recorded so they are not rediscovered as friction later:

- Bumping a **repository-pinned** helper (`playitd`, MCXboxBroadcast) is a repository
  change: pin the new version, compute its SHA-256, commit both, release.
- A repository-pinned helper whose hash is missing is **unavailable**, not best-effort.
  `GET /v1/components` must report that honestly rather than silently degrading, and the
  failure must name the missing pin — not surface as a readiness timeout.
- For **Geyser/Floodgate**, resolution failure, an unparseable API response, or a missing
  upstream `sha256` is equally fatal: no hash, no install. Upstream publishing a checksum
  is what makes dynamic resolution acceptable, so its absence removes the permission.
- Upstream re-tagging or replacing a release under a pinned version stays a *detected*
  condition rather than a silent substitution of the executable the agent runs.

**Rationale.** Phase 3 already made checksum-verified staging the rule for every download
in the product. A long-lived executable that the agent spawns — on a host where the agent
also holds filesystem authority over the user's worlds — is the worst possible thing to
exempt from that rule. Pinning is required regardless; once a version is pinned, the hash
is the cheap part.

The original question, for the record:

```
QUESTION — Who owns the checksum for a pinned helper binary?

What it is:      Every acquisition step below requires a SHA-256 before a downloaded
                 helper is allowed to run. Playit's binary is published on our own
                 GitHub release, but Geyser, Floodgate, and MCXboxBroadcast are
                 third-party projects that may publish no checksum at all.

The choice:      (a) MSC 2 owns the hash. For each pinned helper version we record the
                     expected SHA-256 in the repo; the agent refuses anything that
                     does not match.
                 (b) Verify only where upstream publishes a checksum, and record
                     origin + pinned version without integrity data where it does not.

Why it matters:  (a) means the agent executes only bytes we have personally vouched
                 for, and a compromised or silently re-tagged upstream release cannot
                 run. The cost is real and recurring: bumping any helper becomes a repo
                 change plus a release, and you produce the hash by hand.
                 (b) is free to maintain but leaves the agent running unverified
                 third-party executables — on a machine it also has filesystem
                 authority over.

If unsure:       (a). Phase 3 already made checksum-verified staging the rule for every
                 other download; a *long-lived executable* is the worst thing to exempt.
                 The maintenance cost is a few minutes per helper bump, and pinning is
                 required regardless — the hash is the cheap part once a version is
                 pinned.
```

### P9.6a — Pinned, verified helper-binary acquisition

**Status:** DONE
**Files:** `crates/msc-infrastructure/src/helper_acquisition.rs`, `crates/msc-infrastructure/src/lib.rs`, `crates/msc-infrastructure/tests/helper_acquisition.rs`, `fixtures/networking/`
**What:** Extend P9.6's managed-helper foundation with the one acquisition primitive all three helpers need, so it is not written three times. Resolve an **explicitly pinned** release identity — never `latest` — select an exact named asset for the running platform, verify the SHA-256 recorded in this repository for that pinned version — required for every helper, upstream-published or not, per the answer above — and promote through Phase 3's `stage_download`. Persist origin, pinned version, asset name, and checksum alongside the cached artifact, and keep any previously working artifact in place until the replacement is fully downloaded, verified, and staged. Acquisition is a distinct journaled boundary that terminates with its **own** error: release-resolution, download, checksum, staging, permission, and spawn failures must each surface as themselves and must never be allowed to arm a downstream readiness watchdog. Fakeable — no test may reach the public network.
**Verify:** `cargo nextest run -p msc-infrastructure --test helper_acquisition`
**Commit:** `P9.6a: add pinned verified helper acquisition`
**Batch:** solo

### P9.6b — Record checksum provenance and correct the primitive's stated policy

**Status:** DONE
**Files:** `crates/msc-infrastructure/src/helper_acquisition.rs`, `crates/msc-infrastructure/tests/helper_acquisition.rs`
**What:** P9.6a's shape is correct and is **not** being redone — it already takes the expected digest as caller-supplied data, which is exactly what both checksum sources need. Two things now misstate the design. First, `PinnedHelperRelease`'s doc comment declares a policy the type does not own — "One repository-owned helper release pin … the expected digest is deliberately stored here, even when the upstream project publishes no checksum of its own" — which would tell a cold agent that P9.10a's upstream-published hash violates the design. Reword it to describe a verifier that accepts a digest from either source. Second, `HelperArtifactMetadata` records `sha256` but not where it came from; add a `checksum_source` field (`repository-pinned` | `upstream-published`) so an audit can answer who vouched for the bytes on disk. No behavior change to download, verification, staging, or failure handling.
**Verify:** `cargo nextest run -p msc-infrastructure --test helper_acquisition && rg -n 'checksum_source' crates/msc-infrastructure/src/helper_acquisition.rs`
**Commit:** `P9.6b: record checksum provenance on acquired helpers`
**Batch:** safe

### P9.7a — Acquire `playitd` through the pinned-asset primitive

**Status:** DONE
**Files:** `crates/msc-infrastructure/src/playit.rs`, `crates/msc-application/src/playit.rs`, `crates/msc-application/tests/playit.rs`, `fixtures/networking/`
**What:** Close the Playit acquisition gap. `playit.rs` currently accepts an `executable_path: PathBuf` and spawns whatever it is handed; nothing obtains the binary. Acquire it through P9.6a against a pinned release identity (MSC 1's current evidence is tag `playitd-v1.0.10`, asset `playitd`), selecting the platform-appropriate artifact. Reserve the ~75-second readiness watchdog for a `playitd` that spawned successfully and then stayed silent or never supplied a player address — an acquisition failure must report itself, which is precisely the confusion observed in MSC 1.
**Verify:** `cargo nextest run -p msc-application --test playit`
**Commit:** `P9.7a: acquire playitd through the pinned primitive`
**Batch:** stop-after

### P9.10a — Port MSC 1's Geyser and Floodgate acquisition

**Status:** DONE
**Files:** `crates/msc-infrastructure/src/geyser.rs`, `crates/msc-application/src/geyser.rs`, `crates/msc-application/tests/geyser.rs`, `crates/msc-infrastructure/tests/geyser_resolution.rs`, `fixtures/networking/`
**What:** Close the Geyser/Floodgate acquisition gap by **porting MSC 1's behavior**, not by inventing a compatibility system. MSC 1 (`PluginDownloader.swift`) resolves `download.geysermc.org/v2/projects/{geyser,floodgate}/versions/latest/builds/latest`, then downloads `versions/{version}/builds/{build}/downloads/spigot`, and reports the result as `"{version} (build {build})"` (`AppViewModel+ComponentsVersions.swift`). Reproduce exactly that: latest version, latest build, the `spigot` artifact, both helpers. Keep MSC 1's model in which **Geyser and Floodgate are Paper-family plugins** living in `plugins/` — that is why `spigot` is the only artifact MSC 1 fetches, and it is coherent rather than an oversight; do not extend to `fabric`/`neoforge` in this step. Feed the resolved release into P9.6a's `acquire_pinned_helper` rather than writing a second download path. Resolution failure or an unparseable response is fatal and must surface as its own error, never as a later helper-readiness timeout. Keep the prior working JARs active until download, verification, and configuration validation succeed. Preserve the existing safe YAML mutation and the exclusion of these managed helpers from client-mod export. Resolution must be fakeable — no test may reach the public network.
**Verify:** `cargo nextest run -p msc-infrastructure --test geyser_resolution && cargo nextest run -p msc-application --test geyser`
**Commit:** `P9.10a: port geyser and floodgate acquisition`
**Batch:** solo

**The one deviation from MSC 1, and why.** MSC 1 verifies nothing — it downloads the JAR and uses it. MSC 2 reads the `sha256` the Geyser API already returns for that exact build and verifies against it, per the answered checksum QUESTION above. This is nearly free (the hash is in the same response the version and build come from) and it is the invariant this whole amendment exists to establish. Record `checksum_source: upstream-published` via P9.6b. Mark it as an explicit strengthening of MSC 1 in the step's commit body.

**Explicitly out of scope, matching MSC 1.** No Minecraft-version-to-Geyser-version mapping is built. MSC 1 has none — it installs the newest Geyser onto whatever server exists, with no compatibility guard — and the Geyser API exposes no such mapping to copy. This step preserves that behavior deliberately rather than inventing a system with no oracle behind it. See QUESTION 2 below for the one piece of this that is still open.

### QUESTION 2 — before P9.10a — **ANSWERED**

**Answer: (b) — install as MSC 1 does, and diagnose the failure.** Confirmed by Cameron
Temple, 2026-08-22.

P9.10a's install behavior is unchanged and unblocked: newest version, newest build, `spigot`
artifact, exactly as MSC 1 does it. No version mapping is built. Separately, P9.10b teaches
the startup analyzer to recognise a Geyser load failure and name it.

This turns out cheaper than the question estimated. `StartupProblemKind::IncompatibleVersion`
already exists in `crates/msc-domain/src/crash_analysis.rs` — "Offender is built for a
different MC/loader/mod version" — and `fixtures/paper-plugin-crash-analysis/` already
exists, which is precisely where a Paper-family plugin belongs. This is a new pattern and
new context inside machinery Phase 7 already built and already runs on every failed start,
not a new problem kind and not new machinery.

**Degrades honestly:** if the pattern never matches, behavior is identical to (a) — the same
silence MSC 1 has today. Nothing is made worse by trying.

The original question, for the record:

```
QUESTION 2 — What happens when the newest Geyser does not support the server?

What it is:      Geyser tracks current Minecraft and current Bedrock. Install it on a
                 server running an older Minecraft version and the JAR may simply
                 refuse to load. MSC 1 does nothing about this: it downloads the
                 newest build onto whatever server exists, and the user finds out
                 when the server starts and crossplay silently isn't there.

The choice:      (a) Port MSC 1 exactly — install the newest build, say nothing. The
                     user diagnoses a non-loading plugin themselves.
                 (b) Port MSC 1's install behavior unchanged, but teach Phase 7's
                     startup crash diagnostics to recognise a Geyser load failure and
                     name it: which Geyser version was installed, what the server's
                     Minecraft version is, and that the two do not match.

Why it matters:  Neither option changes what gets downloaded, so neither needs a
                 version mapping and neither delays P9.10a. The difference is purely
                 whether a real, already-reachable failure is explained or left silent.
                 (b) is a second, smaller step after P9.10a, in the crash-diagnostics
                 code Phase 7 already built — not new machinery.

If unsure:       (b). "The server told you nothing and crossplay just didn't work" is
                 the exact class of problem MSC exists to solve, and the diagnosis
                 costs one pattern in a parser that already runs on every failed
                 start. It also degrades honestly: if the pattern never matches,
                 behavior is identical to (a).
```

### P9.10b — Name a Geyser load failure in startup diagnostics

**Status:** DONE
**Files:** `crates/msc-domain/src/crash_analysis.rs`, `crates/msc-domain/tests/paper_plugin_crash_analysis.rs`, `fixtures/paper-plugin-crash-analysis/`
**What:** Per QUESTION 2's answer, close the silence MSC 1 leaves when the newest Geyser will not load on an older server. P9.10a deliberately installs the newest build with no compatibility guard, exactly as MSC 1 does; this step makes the resulting failure explicable instead of invisible. Characterize real Geyser load failures against an older Paper server first — do not invent log lines — then teach the analyzer to classify them as the existing `StartupProblemKind::IncompatibleVersion`, carrying as context the installed Geyser version and build (which P9.10a records in `HelperArtifactMetadata`) and the server's Minecraft version, so the finding states plainly that the two do not match. **No new problem kind, no new machinery** — this is a pattern and its context inside the analyzer Phase 7 already runs on every failed start. Where the analyzer offers a repair action, the correct one is not an automatic version walk-back (P9.10a explicitly builds no mapping) but naming the mismatch; whether a repair is offered at all is P9.10b's to determine from the evidence, not to assume. If the pattern does not match, the analyzer must fall through silently — behavior identical to MSC 1's today.
**Verify:** `cargo nextest run -p msc-domain --test paper_plugin_crash_analysis`
**Commit:** `P9.10b: name geyser load failures in startup diagnostics`
**Batch:** solo

### P9.11a — Verify the Xbox Broadcast JAR through the acquisition primitive

**Status:** awaiting verification
**Files:** `crates/msc-infrastructure/src/xbox_broadcast.rs`, `crates/msc-application/src/xbox_broadcast.rs`, `crates/msc-application/tests/xbox_broadcast.rs`, `fixtures/networking/`
**What:** **MSC 1's resolution behavior is correct here and is kept.** `XboxBroadcastDownloader.swift:41,51` queries `api.github.com/repos/MCXboxBroadcast/Broadcaster/releases/latest` for the tag and asset — the comment says "so we KNOW the version" — and `xbox_broadcast.rs:16,18` already reproduces it. Per the amended two-rule table, Broadcast resolves latest rather than pinning: it tracks Xbox Live authentication and the Bedrock protocol, releases roughly every nine days, and a repository pin would strand every user's Broadcast until we cut a release. This step therefore adds **one thing**: the GitHub releases API returns a `digest` field per asset (`"digest": "sha256:…"`), and MSC 2 must verify the downloaded JAR against it and refuse a mismatch — MSC 1 verifies nothing. Route the resolved release through P9.6a's `acquire_pinned_helper` rather than keeping a separate download path, and record `checksum_source: upstream-published` via P9.6b. Preserve MSC 1's existing behavior of resolving and recording the tag. A release whose asset carries no `digest` is fatal: no hash, no install. Metadata, download, checksum, staging, and process-start failures must each surface as their own operation error before the ~60-second broadcast-readiness watchdog is armed. Keep the existing JAR active until the replacement is downloaded, verified, staged, and recorded.
**Verify:** `cargo nextest run -p msc-application --test xbox_broadcast`
**Commit:** `P9.11a: verify the xbox broadcast jar on download`
**Batch:** stop-after

**No decision entry is required.** An earlier draft of this step pinned the JAR and therefore
needed D-028 to record a deliberate divergence from MSC 1. That draft rested on a wrong
belief about the project's release cadence; there is no divergence to record. Adding
checksum verification is a strengthening of the same kind P9.10a makes, noted in the commit
body rather than promoted to a numbered decision.

### P9.12 — Implement durable named-token administration and revocation

**Status:** DONE
**Files:** `crates/msc-agent/src/auth.rs`, `crates/msc-agent/src/routes/users.rs`, `crates/msc-agent/src/routes/mod.rs`, `crates/msc-agent/tests/user_routes.rs`, `crates/msc-infrastructure/src/credential_repository.rs`, `crates/msc-infrastructure/tests/credential_repository.rs`, `fixtures/credentials/`
**What:** Wire the existing durable registry and verifier into the P9.4 `GET /users`, `POST /users`, `POST /users/update`, and `POST /users/revoke` contract. Admin-only access, label/role/permission/expiry validation, secret issuance exactly once, audit attribution, secret deletion, registry persistence, and revoked-token rejection must all be explicit. Prove that revoke wins over stale in-memory state and remains effective after the agent restarts using the same production `SecretStore` path Phase 4/5 use; never return raw bearer secrets from list/update responses.
**Verify:** `cargo nextest run -p msc-infrastructure --test credential_repository && cargo nextest run -p msc-agent --test user_routes`
**Commit:** `P9.12: add durable user credential management`
**Batch:** solo

### P9.13 — Wire Phase 9 routes, operations, capability discovery, and CLI

**Status:** DONE
**Files:** `crates/msc-agent/src/routes/networking.rs`, `crates/msc-agent/src/routes/users.rs`, `crates/msc-agent/src/routes/mod.rs`, `crates/msc-agent/src/routes/capabilities.rs`, `crates/msc-agent/src/cli/mod.rs`, `crates/msc-agent/tests/phase9_routes.rs`, `crates/msc-agent/tests/cli_phase9.rs`, `docs/msc2/client-capability-matrix.csv`
**What:** Connect the completed application services to the frozen HTTP, WebSocket, capability, and scriptable CLI surfaces. Every long-running helper action must enter the shared operation model and support status/poll/cancel; unavailable host or server requirements must be advertised instead of inferred by a client. CLI output stays machine-readable and never prints secrets except the one-time token-creation value on an explicitly interactive-safe path. Mark only actually reachable API/CLI/iOS surfaces implemented in the capability matrix. Reproduce MSC 1's first-run two-pass orchestration for server creation with Playit/Broadcast enabled (`AppViewModel+ServerControls.swift`'s initiation pass 1/2, symbol-ledger row 195): hold creation's completion open until every awaited transport's readiness signal (P9.7, P9.11) resolves or the ~10-minute safety cap trips, using the shared operation model rather than MSC 1's ad hoc timers.
**Verify:** `cargo nextest run -p msc-agent --test phase9_routes --test cli_phase9`
**Commit:** `P9.13: wire networking helper interfaces`
**Batch:** stop-after

### P9.14 — Prove public-path safety, restart recovery, and real integration evidence

**Status:** awaiting verification
**Files:** `tools/phase9/phase9-smoke.sh`, `tools/phase9/phase9-check.py`, `tools/phase9/credential-revocation-check.py`, `docs/msc2/networking/evidence/`, `docs/msc2/networking/phase9-scope.md`, `docs/msc2/client-capability-matrix.csv`
**What:** Build the reviewed Phase 9 evidence runner and use it to prove that player-facing helpers cannot expose the management API, secrets are absent from returned/logged/audited data, revoked credentials fail after restart, operations cancel and recover honestly, and each supported integration has either reproducible success evidence or an explicit unavailable record. Run safe real-provider checks only with disposable credentials and no production server mutation. This step reports evidence; it does not mark the phase complete.
**Verify:** `python3 tools/phase9/phase9-check.py --evidence && bash tools/phase9/phase9-smoke.sh --synthetic && python3 tools/phase9/credential-revocation-check.py`
**Commit:** `P9.14: prove networking helper safety`
**Batch:** solo

### P9.15 — Close the Phase 9 working gate

**Status:** awaiting verification
**Files:** `tools/phase9/phase9-check.py`, `tools/phase9/phase9-smoke.sh`, `docs/msc2/networking/phase9-scope.md`, `docs/msc2/client-capability-matrix.csv`, `docs/msc2/rolling-plan.md`
**What:** Check P9.1’s working gate against one exact candidate: every port-plan deliverable is implemented or honestly recorded as an owner-approved deferral; player-network and management-network boundaries hold; helper lifecycle, resource-pack safety, credential CRUD/revocation across restart, public API/CLI/iOS paths, fixture/evidence provenance, cancellation/recovery, tri-platform CI, and headless no-GUI linking all pass. Run the full workspace suite once here because this is the phase-wide regression sweep. Report any gap without marking the phase complete or pre-empting the other agent’s REVIEW.
**Verify:** `python3 tools/phase9/phase9-check.py --gate && bash tools/phase9/phase9-smoke.sh --synthetic && cargo nextest run --workspace`
**Commit:** `P9.15: close the Phase 9 gate`
**Batch:** solo

### P9.16 — Repair and re-verify Phase 9 API conformance

**Status:** awaiting verification
**Files:** `crates/msc-api/tests/phase9_conformance.rs`, `docs/msc2/rolling-plan.md`
**What:** Fix the Phase 9 conformance test's Rust name-shadowing compile error: `resolve`'s `schema` parameter hides the helper function named `schema`. Rename only the parameter or otherwise make the helper call unambiguous; do not alter the API contract or weaken the assertions. Run P9.4's literal Verify command afterward. If that command fails for any other reason, stop and report it rather than broadening the change.
**Correction:** The notification WebSocket payload now uses the exact bare OpenAPI schema reference expected by this conformance test; the prior explanatory suffix was not a valid machine-readable reference.
**Verify:** `python3 tools/api-contract-check.py --v1-summary && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && cargo nextest run -p msc-api --test phase9_conformance && rg -n 'ConnectivityResponseDTO' docs/msc2/api-contract/openapi.json`
**Commit:** `P9.16: repair phase9 API conformance`
**Batch:** solo

### P9.17 — Verify the Xbox Broadcast acquisition amendment

**Status:** awaiting verification
**Files:** `crates/msc-application/src/xbox_broadcast.rs`, `crates/msc-application/tests/xbox_broadcast.rs`, `docs/msc2/rolling-plan.md`
**What:** Run P9.11a's own Verify command against the completed Broadcast checksum-acquisition implementation. Confirm that the upstream digest is required and verified before promotion, the prior JAR remains intact on failure, and readiness timing is not armed by acquisition failure. After the command passes, update P9.11a to `awaiting verification`; Cameron closes it after independently running the same command.
**Verify:** `cargo nextest run -p msc-application --test xbox_broadcast`
**Commit:** `P9.17: verify Xbox Broadcast acquisition`
**Batch:** stop-after

### P9.18 — Run the Phase 9 synthetic gate in tri-platform CI

**Status:** awaiting verification
**Files:** `.github/workflows/ci.yml`, `docs/msc2/rolling-plan.md`
**What:** Extend the existing macOS/Linux/Windows toolchain job so every platform runs `python3 tools/phase9/phase9-check.py --gate` and `bash tools/phase9/phase9-smoke.sh --synthetic` in addition to the existing workspace, Phase 6, Phase 7, and Phase 8 checks. Keep the headless no-GUI artifact and link job unchanged. The Phase 9 smoke is offline and must use its existing fake transports; do not add provider credentials or live network calls to CI. This creates the exact candidate whose CI result P9.19 will inspect.
**Verify:** `git diff --check && rg -n 'phase9-check.py --gate|phase9-smoke.sh --synthetic' .github/workflows/ci.yml`
**Commit:** `P9.18: run phase9 synthetic gate in CI`
**Batch:** solo

### P9.19 — Re-run and record the exact Phase 9 gate

**Status:** not started
**Files:** `docs/msc2/networking/phase9-scope.md`, `docs/msc2/rolling-plan.md`
**What:** Run the complete P9.15 gate against the exact P9.18 code candidate: the documentary checker, synthetic smoke, full workspace suite, and the GitHub Actions run whose `headSha` is that candidate. Require green macOS, Linux, Windows, and headless no-GUI jobs. Then reconcile the review record: state the exact candidate SHA and CI run, correct the stale Phase 9 header, mark P9.11a/P9.14/P9.15 accurately, and remove the Phase 9 scope wording that still implies general-LAN management is allowed; the approved D-012 posture is loopback by default with explicitly configured Tailscale only. Do not claim live provider success where the evidence remains honestly unavailable. Leave this step `awaiting verification` for the independent REVIEW move; Cameron alone marks it `DONE` after running the Verify command.
**Verify:** `candidate_sha=$(git log -1 --format=%H --grep='P9.18: run phase9 synthetic gate in CI') && test -n "$candidate_sha" && python3 tools/phase9/phase9-check.py --gate && bash tools/phase9/phase9-smoke.sh --synthetic && cargo nextest run --workspace && run_id=$(gh run list --workflow ci.yml --commit "$candidate_sha" --json databaseId,headSha,conclusion --jq 'map(select(.headSha == "'"$candidate_sha"'" and .conclusion == "success"))[0].databaseId') && test -n "$run_id" && gh run view "$run_id" --exit-status`
**Commit:** `P9.19: re-run and record the phase9 gate`
**Batch:** solo

## Phase 9 amendments log

### 2026-08-22 — Managed-helper acquisition and truthful startup failures

Real-world MSC 1 evidence showed that the Playit claim flow can succeed while the required `playitd` binary is never downloaded: `PlayitBinaryManager.swift` asks MSC's own `/releases/latest` for an asset named `playitd`, but the binary is published under the separate `playitd-v1.0.10` release and app releases have shadowed it. The subsequent readiness timeout therefore described a consequence, not the cause.

The MSC 2 audit found three different current states:

- **Playit:** P9.7 has no acquisition path at all. `PlayitLaunch` accepts an executable path, and P9.7 supervises the process and records spawn failures immediately; there is no binary-resolution error to surface, and a readiness timeout applies only after a process has spawned.
- **Geyser/Floodgate:** P9.10 has no acquisition path. It detects already-present JARs and edits `Geyser-Spigot/config.yml`; no version is resolved dynamically, but installation/update behavior promised by P9.10 is still uncovered.
- **Xbox Broadcast:** P9.11 resolves `/releases/latest`, falls back to `/releases/latest/download/MCXboxBroadcastStandalone.jar`, and calls `stage_download(..., None)`. It is staged atomically but not checksum-verified and is not pinned.

The three amendments above establish one rule for all required helper binaries: acquisition must be explicit, pinned, integrity-checked through Phase 3 staging, and completed before readiness timing begins. They are new work; none changes the DONE P9.6, P9.7, P9.10, or P9.11 entries, and P9.12–P9.15 remain untouched.

---
