# MSC 2 — Rolling Plan

> ## STATUS: Phase 11 (desktop/web clients) is in progress; **Phase 12 (client redesign) is now planned** below. Phase 11 shipped a working client wired to the real agent, but its UI diverged from MSC 1's information architecture and design language — Phase 12 rebuilds the presentation layer to MSC 1 fidelity, *refreshed*. Terminal UI moved to Phase 13. Phase 12's design system (S0) and shell (S1) were shaped and locked as reference specimens in `docs/msc2/renderings/`, governed by `docs/msc2/antiAIslop.md` (hard rule #11).
> **Next move:** P12.2 (Overview tab) is DONE — Cameron verified it 2026-08-25. P12.2b–j (Java player-data NBT backend, built with Codex) all landed and are marked DONE. P12.3 (Players tab) and its follow-ups P12.3a (session-log contract), P12.3d (Bedrock identify + skin fixes) are built and **awaiting Cameron's verification**; P12.3b/c (session-log application layer + routes) and P12.3e (carry real Bedrock stats/inventory through) are planned but **not started**. See this file's 2026-08-25 notes below for the full history of gaps found and how each was handled.
> **P12.3 blocked on missing backend (decided 2026-08-25):** before rebuilding the Players tab, Cameron flagged that MSC 1's Players tab includes a read-only Java player inventory/stats viewer (`PlayerNBTReader.swift` + `PlayerInventoryView.swift`, hosted in `PlayerProfileDetailSheet.swift`) that never made it past the file-inventory audit into an actual phase step — no domain crate, no API route, and P12.3's own `What:` line never mentioned it. Investigation found `GET /v1/players/profiles` is **already frozen in the API contract** (`docs/msc2/api-contract/openapi.json`: `PlayerProfileDTO`/`PlayerStatsDTO`/`InventoryItemDTO`, plus `POST /v1/players/hidden`, `POST /v1/players/skin-override`, `GET /v1/players/{profileId}/skin`) but has **no handler at all** — today `GET /v1/players` only serves Bedrock (`crates/msc-agent/src/routes/bedrock.rs`; a Java server gets `note: "not_bedrock"`, empty list). This is a straight port against an already-frozen contract, not new API design. Cameron chose to block P12.3 and build the backend first (steps P12.2b–P12.2j below) rather than ship Players tab without it. Online Now / Seen This Session / Session Log are unaffected — those are console-derived (already built in P11.11) and stay in P12.3 itself. **Mutation actions, decided 2026-08-25:** of MSC 1's 5 player-data mutation actions (migrate to offline UUID, migrate to manual UUID, copy, duplicate, delete), none were in the frozen contract. Cameron chose **4 of the 5** — delete, migrate-to-offline-UUID, migrate-to-custom-UUID, and duplicate — added as new steps P12.2g (contract amendment) through P12.2j (route wiring), fully specified (exact DTO field names/types, exact error codes, pinned known-answer test vectors for the offline-UUID algorithm) since Cameron is running these with Codex. `copyPlayerData` (overwrite one player's data onto another's) is the one action still **deferred, not dropped** — add it later as its own contract-amendment step when wanted.
> **Phase 11 → 12 sequencing (decided 2026-08-25):** the committed P11.28g–j agent work is done and carries forward as Phase 12's foundation. The two unfinished Phase 11 steps — P11.28k and the P11.29 gate — are **superseded and folded into P12.17**, because they verify the first-launch UI and MSC 1 fidelity that only the redesign delivers; the whole client gate now runs once against the redesigned client. Phase 12 begins now.
> **Last updated:** 2026-08-25

**Previous phases (Setup through Phase 10) and their amendments have moved to `rolling-plan-archive.md`** to keep this file small. That archive is historical only — current status and active work stay here.

---

## How this document works

This is the **working state** of the build. The vision documents say where we're going; the port plan says in what order; this file says **where we actually are right now**.

Phases are fixed and come from `msc2-port-plan.md`. **Steps are written one phase at a time**, as we reach each phase — not up front. Phase 11 is active; P11.1 now awaits Cameron's verification.

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
| **9** | Networking and helpers | complete |
| **10** | Bedrock runtimes | complete |
| **11** | Desktop and web clients | agent layer done (P11.28g–j); UI verification (P11.28k, P11.29) folded into Phase 12 |
| **12** | Client redesign (MSC 1 fidelity, refreshed) | **in progress — P12.2b–j DONE; P12.3/P12.3a/P12.3d awaiting verification; P12.3b/c/e not started** |
| 13 | Terminal UI (deferred from v1) | not started |

---

## Phase 11 — Desktop and web clients

**Gate** (`msc2-port-plan.md` §3): one Svelte frontend is delivered through
both the Tauri desktop shell and the agent-served browser UI, built against
the proven API while preserving MSC 1's information architecture and design
language. `GET /v1/help/{helpId}` resolves the embedded handbook, concept, and
router-guide content, and clients consume that contract rather than carrying
their own divergent teaching text. UI completion never gates headless agent
correctness.

**Working exit criteria:** macOS, Windows, and Linux desktop builds and the
browser load the same generated frontend bundle and expose every Phase
11-owned capability that the connected agent advertises and the calling token
permits. All client state is host-keyed, the current host and active server are
always visible, reconnecting restores bounded console and operation state, and
dangerous actions remain explicit. TypeScript DTOs are generated from
`docs/msc2/api-contract/openapi.json`; no handwritten mirror DTOs are allowed.
The agent serves the same static bundle plus embedded educational content, and
the Desktop/Web column of `client-capability-matrix.csv` is updated with every
surface rather than filled in retrospectively. Browser cookie auth and Tauri
local/remote auth close Phase 11's remaining D-012 scope without weakening
Phase 9's loopback-by-default, explicit-Tailscale-only posture. Linux proof
must launch the real Tauri binary through WebKitGTK under a display server and
interact with it; a Chromium-only browser run is not Linux desktop evidence.
The gate also requires fresh-profile evidence for MSC 1's first-launch
experience: setup sheet, Concept Guide, Handbook handoff, guided tour ordering
and anchors, skip/reopen state, splash playback or fallback, reduced-motion
behavior, and the boundary between agent-owned first-server initiation and
client-owned presentation.

**Extensibility boundary:** navigation and routing are a registry of section
descriptors keyed by stable strings and capability predicates, not a closed
enum, exhaustive switch, or fixed tab array. The route families
`/hosts/:hostId/servers/:serverId/bedrock/*` and
`/hosts/:hostId/servers/:serverId/profiles/*` are reserved without shipping
either screen. A later Bedrock client group registers its sections only when
`GET /v1/capabilities` advertises a usable Bedrock backend; it never infers support
from the host OS or client build. A later player-profiles phase must first port
the ledgered agent workflows (profile loads, Mojang/Floodgate resolution,
manual Bedrock identification, UUID migration/data mutation, hidden profiles,
and skin storage/serving), extend the public contract and capability response,
regenerate TypeScript, then register its section. Phase 11's registry,
host/server-scoped route parameters, lazy section loading, permission filters,
and unknown-capability tolerance are the seam that later phase consumes; none
of that work should require replacing Phase 11 navigation.

**Execution order:** no Phase 11 step is blocked on ownership of `crates/`,
the shared contract, CI, or packaging. P11.1–P11.19 build and prove the common
client foundation and non-Bedrock surfaces. P11.20 consumes the completed
Bedrock capability/runtime-state contract to prove the extension seam without
shipping Bedrock screens.
P11.21–P11.29 may then change shared agent/auth/packaging code and close the
gate. Step-to-step dependencies still apply, and a batch must never continue
after a failed Verify.

**Owner choices confirmed during PLAN (2026-08-22):** general-LAN management
remains unavailable for v1. Browser management stays on loopback or an
explicitly configured Tailscale path, and Tailscale never replaces
authentication or permission checks; Phase 11 does not build a local
certificate authority or ask users to bypass browser certificate warnings.
On macOS and Windows, MSC may download and verify a coordinated desktop,
agent, and compatible sidecar update, but it asks before installation; it does
not silently install automatically. Linux update installation remains owned by
the package manager, with MSC limited to an actionable availability notice.

### Group A — Shared client foundation and non-Bedrock surfaces (in progress)

### P11.1 — Scope the client rebuild from the iOS oracle and capability matrix
**Status:** DONE
**Files:** `docs/msc2/clients/phase11-scope.md`, `docs/msc2/audit/msc2-symbol-ledger.csv`, `tools/phase11/scope-check.py`, `docs/msc2/rolling-plan.md`
**What:** Read all 53 copied iOS client files as the primary behavioral and screen-structure reference, then use the MSC 1 macOS views only for the desktop information architecture and visual language. Map every current OpenAPI/WebSocket operation and matrix row to a Phase 11 screen, shared infrastructure, honest future state, or explicitly out-of-scope agent gap. Record the D-003 same-screen rule, D-013 host scoping, D-023 matrix-update rule, D-026 help ownership, D-021 client resource bounds, and the exact Bedrock/player-profile extensibility handoffs above. Do not assign the player-profile agent rows to Phase 11 merely because their old DTOs remain in the frozen baseline.
**Verify:** `python3 tools/phase11/scope-check.py docs/msc2/clients/phase11-scope.md docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.1: scope desktop and web clients`
**Batch:** solo

### P11.2 — Scaffold one standalone Svelte and Tauri client
**Status:** DONE
**Files:** `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src/`, `clients/desktop-web/static/`, `clients/desktop-web/src-tauri/`, `clients/desktop-web/svelte.config.js`, `clients/desktop-web/vite.config.ts`, `clients/desktop-web/tsconfig.json`
**What:** Create one TypeScript Svelte frontend with a static build suitable for both agent serving and a thin Tauri 2 shell. Keep `src-tauri` standalone from the root Cargo workspace, with its own lockfile and no server-management behavior, so the existing headless Rust workspace does not acquire GUI build dependencies. Establish formatting, type-checking, unit-test, production-build, and bundle-identity commands.
**Verify:** `npm --prefix clients/desktop-web run verify:scaffold`
**Commit:** `P11.2: scaffold shared Svelte client`
**Batch:** solo

### P11.3 — Generate TypeScript from the frozen OpenAPI contract
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/api/generate.ts`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `tools/phase11/generated-types-check.py`
**What:** Generate the HTTP request/response type surface directly from the current frozen `docs/msc2/api-contract/openapi.json`, preserving optional/additive fields needed for D-010 skew, including `BedrockRuntimeStateDTO` even though Phase 11 ships no Bedrock screens. Make regeneration deterministic and fail when checked-in output differs from the contract. Handwritten transport helpers may wrap generated types, but no hand-authored DTO mirror is permitted.
**Verify:** `npm --prefix clients/desktop-web run api:check`
**Commit:** `P11.3: generate TypeScript API types`
**Batch:** solo

### P11.4 — Build the contract-backed client test harness
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/testing/`, `clients/desktop-web/tests/contract/`, `clients/desktop-web/tests/fixtures/`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`
**What:** Add deterministic fake HTTP, WebSocket, upload/download, operation, auth, capability, permission, old-agent/new-agent, and reconnect scenarios using generated DTO shapes. Include unknown optional fields and absent future capability keys so the UI proves additive skew tolerance. This becomes the reviewed test boundary later `safe` screen batches use; it must not invent Bedrock screens or player-profile behavior. The separate P11.20 seam test consumes the real Bedrock capability/runtime-state contract.
**Verify:** `npm --prefix clients/desktop-web run test:contract`
**Commit:** `P11.4: add client contract harness`
**Batch:** solo

### P11.5 — Establish extensible information architecture and routing
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/navigation/`, `clients/desktop-web/src/routes/`, `clients/desktop-web/tests/navigation/`, `docs/msc2/clients/phase11-scope.md`
**What:** Implement the descriptor registry, nested host/server route parameters, permission and capability predicates, lazy component loading, stable deep links, narrow/wide layouts, and unknown-section fallback. Prohibit a closed section enum, exhaustive section switch, fixed tab-count assumptions, and checks such as `hostOs == linux` standing in for capability discovery. Reserve but do not register or render Bedrock and player-profile route families; tests must prove a synthetic future descriptor can be added without editing the shell/router and remains hidden until its named advertised capability is present.
**Verify:** `npm --prefix clients/desktop-web run test:navigation`
**Commit:** `P11.5: add extensible client routing`
**Batch:** solo

### P11.6 — Make all connection and cache state host-scoped
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/hosts/`, `clients/desktop-web/src/lib/stores/`, `clients/desktop-web/tests/hosts/`
**What:** Implement D-013's host registry, minimal host switcher, per-host connection/capability/permission/server/console/operation caches, active-server selection, stale-data isolation, and explicit host identity on every destructive confirmation. Credentials remain behind an injected credential adapter so the browser and Tauri mechanisms can land later without migrating store shapes. No singleton active host, global console buffer, or credential field may leak across hosts.
**Verify:** `npm --prefix clients/desktop-web run test:hosts`
**Commit:** `P11.6: add host-scoped client state`
**Batch:** safe

### P11.7 — Implement the generated HTTP and resilient stream client
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/api/`, `clients/desktop-web/src/lib/streams/`, `clients/desktop-web/src/lib/operations/`, `clients/desktop-web/tests/transport/`
**What:** Build one host-aware transport over generated request/response types, `ErrorDTO`, version headers, capability refresh, bounded staged transfers, and cookie-or-bearer credential adapters. Add console, operation, and notification stream reconnect with bounded history, deduplication, cancellation, terminal-state recovery, and explicit unsupported/old-client states. Keep browser and desktop on the same calls; shell IPC may supply credentials or native services but never an alternative management API.
**Verify:** `npm --prefix clients/desktop-web run test:transport`
**Commit:** `P11.7: build shared API transport`
**Batch:** safe

### P11.8 — Build the responsive MSC design system and application shell
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/components/`, `clients/desktop-web/src/lib/styles/`, `clients/desktop-web/src/routes/+layout.svelte`, `clients/desktop-web/tests/visual/`
**What:** Translate the copied iOS component structure and MSC 1 macOS design language into reusable tokens, cards, tables, forms, dialogs, alerts, empty/loading/error states, keyboard focus, reduced motion, and responsive sidebar/bottom-navigation shells. Preserve desktop's server-list/sidebar and always-available console concepts without baking today's section count into layout. Include the client-owned first-launch/splash seam needed for the setup sheet, Concept Guide, Handbook handoff, guided-tour overlay, and animation/fallback behavior; do not replace that sequence with a generic welcome screen. The shell must visibly name the selected host and server and remain usable at phone, tablet, and desktop widths.
**Verify:** `npm --prefix clients/desktop-web run test:visual-shell`
**Commit:** `P11.8: build shared MSC interface shell`
**Batch:** stop-after

### P11.9 — Build fleet, provisioning, and lifecycle workflows
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/home/`, `clients/desktop-web/src/lib/sections/fleet/`, `clients/desktop-web/tests/screens/fleet.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement status, active-server switching, create/import/rename/delete/EULA, Java family/version/runtime selection and install, templates, start/stop/restart, clear confirmations, capability/permission gates, and durable operation progress. Use the iOS create/import flows as the functional reference and desktop macOS views for hierarchy only. Update each delivered Desktop/Web matrix row in this same step; unsupported or agent-Planned routes stay `Planned`, never implied by a disabled decorative control.
**Verify:** `npm --prefix clients/desktop-web run test:screen-fleet && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.9: add fleet and lifecycle screens`
**Batch:** safe

### P11.10 — Build console, commands, operations, notifications, and performance
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/console/`, `clients/desktop-web/src/lib/sections/performance/`, `clients/desktop-web/src/lib/components/operations/`, `clients/desktop-web/src/lib/components/notifications/`, `clients/desktop-web/tests/screens/live.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement the bounded live console with history/search/filter/pause/copy/clear-local-view, command history/favorites, operation progress/cancel/recovery, notification feed, performance metrics/charts, help affordances, and reconnect behavior. Use DOM/SVG/CSS rendering with a low-cost fallback rather than assuming Chromium-only WebGL/canvas behavior. Update the matching Desktop/Web matrix cells.
**Verify:** `npm --prefix clients/desktop-web run test:screen-live && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.10: add live server screens`
**Batch:** safe

### P11.11 — Build the online roster without claiming player profiles
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/players-online/`, `clients/desktop-web/tests/screens/players-online.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Render only the generic online roster the connected agent actually advertises. Do not build the Bedrock allowlist/permissions UI in this phase. Keep the registered section identity distinct from the reserved future `profiles` route; do not call the frozen-but-unimplemented profile, skin, hidden-profile, session-history, UUID migration, or player-data mutation routes and do not present their matrix cells as implemented. Prove the online section still works when profile capability fields are unknown, absent, or later added.
**Verify:** `npm --prefix clients/desktop-web run test:screen-players-online && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.11: add online player roster`
**Batch:** safe

### P11.12 — Build worlds, backups, and staged transfer workflows
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/worlds/`, `clients/desktop-web/src/lib/sections/backups/`, `clients/desktop-web/src/lib/transfers/`, `clients/desktop-web/tests/screens/worlds-backups.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement slot inventory/activation/create/rename/duplicate/delete/import/export/convert, direct active-world mutations where the API supports them, thumbnails, backup create/config/delete/restore, bounded uploads/downloads, transactional warnings, progress/cancel/recovery, and risk-appropriate confirmations. Update every genuinely delivered Desktop/Web matrix cell and leave unavailable agent paths visible only as truthful capability explanations.
**Verify:** `npm --prefix clients/desktop-web run test:screen-worlds-backups && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.12: add world and backup screens`
**Batch:** safe

### P11.13 — Build add-on, modpack, and component workflows
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/addons/`, `clients/desktop-web/src/lib/sections/components/`, `clients/desktop-web/tests/screens/addons.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Implement installed add-ons, catalog search, install/update/toggle/remove/source actions, system-component state, client export, modpack inspect/import/replace, and D-027 manual browser-download then bounded staged-upload completion. Preserve provider-unavailable, dependency, pack-managed, cancellation, and provenance explanations. Update the matching Desktop/Web matrix rows; never hardcode provider or server-family lists where the contract supplies them.
**Verify:** `npm --prefix clients/desktop-web run test:screen-addons && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.13: add add-on and modpack screens`
**Batch:** safe

### P11.14 — Build settings, health, networking, helpers, and access administration
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/sections/settings/`, `clients/desktop-web/src/lib/sections/health/`, `clients/desktop-web/src/lib/sections/connectivity/`, `clients/desktop-web/src/lib/sections/access/`, `clients/desktop-web/tests/screens/administration.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Render schema-driven settings without a client-side field enum; implement health cards/problems/repairs, RAM/Java/Geyser, connectivity diagnostics, Playit, DuckDNS, Xbox Broadcast, resource packs, and named-token create/update/revoke with one-time-secret handling. Permission and capability filters must remove unavailable actions while keeping explanations. Agent-Planned files/watchdog/profile routes remain Planned rather than receiving fake screens. Update each delivered matrix row.
**Verify:** `npm --prefix clients/desktop-web run test:screen-administration && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.14: add administration screens`
**Batch:** safe

### terra clean up

#### Review findings

- The original P11.5–P11.8 foundations were not connected to the running
  shell: it selected hard-coded sections and a hard-coded host/server instead
  of loading the token's permissions and the host's advertised capabilities.
- Reconnecting streams changed label but never scheduled a retry, and staged
  downloads had no client-side memory ceiling.
- The P11.11 roster requested `/v1/session-log` and presented a join/leave
  history even though that route is deliberately deferred with player profiles.
- The capability matrix overstated deferred Bedrock/world and shared-context
  routes as delivered. It also marked live WebSocket channels implemented while
  the shell still reads the bounded HTTP snapshots; those channels wait for the
  authenticated wiring in P11.21–P11.23. A rejected world mutation also
  changed local UI state, and destructive confirmations did not name their
  exact host and target.
- The focused tests covered helpers but did not protect all of those integration
  boundaries against regression.

#### Items completed in this cleanup

- Connect the shell to `/v1/capabilities` and `/v1/me`, use the descriptor
  registry to filter routes, and retain host/server identity in the route.
- Give streams bounded automatic reconnect attempts and give staged downloads a
  512 MiB client-memory ceiling.
- Remove the deferred session-history request and restore its matrix row to
  `Planned`.
- Correct the affected capability-matrix claims, preserve state after rejected
  world actions, name host plus target in destructive confirmations, and leave
  live streams honestly `Planned` until their authentication boundary exists.
- Add regression coverage for the stream/download and deferred-route boundaries.

### P11.15 — Extract and validate the educational content corpus
**Status:** DONE
**Files:** `content/help/`, `content/guides/`, `fixtures/help-content/`, `fixtures/onboarding/`, `tools/phase11/help-content-check.py`, `docs/msc2/clients/phase11-scope.md`
**What:** Extract MSC 1's 31-topic handbook, concept guide, router catalog/records/steps, troubleshooting content, and onboarding copy into the confirmed Markdown-with-YAML-front-matter and structured guide data formats. Preserve source citations and label content versus executable router rules; do not duplicate prose in Svelte. Record unresolved diagram assets honestly. Include the Concept Guide page order, onboarding step content/order, skip/reopen wording, and the source mapping for every first-launch explanation; visual anchoring and animation remain client-owned. Include coverage for every `helpId` already emitted by settings, health, diagnostics, performance, connectivity, and errors, including the `bedrock.runtime-unavailable` later-audit requirement. Add deterministic onboarding fixtures proving fresh-install, already-seen, skipped, reopened, and unknown/future-topic cases.
**Verify:** `python3 tools/phase11/help-content-check.py --all`
**Commit:** `P11.15: extract educational content`
**Batch:** solo

### P11.16 — Render contract-served help and guides in the shared client
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/help/`, `clients/desktop-web/src/lib/sections/handbook/`, `clients/desktop-web/tests/screens/help.test.ts`, `docs/msc2/client-capability-matrix.csv`
**What:** Build safe Markdown rendering, related-topic navigation, handbook/concept/router-guide readers, contextual `helpId` links, unknown-topic degradation, and client-owned onboarding anchors against the fake contract. Implement and test the fresh-install sequence of setup sheet → Concept Guide → guided tour, including the exact step ordering, user-action pauses, form-card hide/resume behavior, completion state, skip behavior, Handbook handoff, and reopen-from-preferences behavior. Implement the splash animation seam with a bounded fallback and reduced-motion path; preserve the source asset or record an explicit reviewed replacement. Every explanation comes from a response or structured content fixture, never a screen-local copy. The registry must allow future Bedrock and profile topics/sections to appear additively without new shell logic.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && python3 tools/phase11/help-content-check.py --client`
**Commit:** `P11.16: render shared help content`
**Batch:** safe

### P11.17 — Prove browser parity, accessibility, and responsive layouts
**Status:** DONE
**Files:** `clients/desktop-web/tests/e2e/browser/`, `clients/desktop-web/playwright.config.ts`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`
**What:** Exercise the production static bundle against the contract harness at narrow and wide widths, keyboard-only navigation, reduced motion, destructive confirmations, host switching, reconnect, upload/download, and deep-link reload. Add a fresh-profile onboarding walkthrough proving setup completion, Concept Guide → tour sequencing, step ordering and anchors, user-action pauses, skip/resume/reopen flags, Handbook handoff, splash playback/fallback, and reduced-motion behavior. Run Chromium plus browser WebKit for fast compatibility feedback, while recording plainly that this is browser evidence and does not replace P11.19's native Linux WebKitGTK proof.
**Verify:** `npm --prefix clients/desktop-web run test:e2e-browser`
**Commit:** `P11.17: prove shared browser workflows`
**Batch:** stop-after

### P11.18 — Add the thin Tauri shell without desktop-only screens
**Status:** DONE
**Files:** `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/tests/tauri/`
**What:** Load the exact production Svelte bundle and expose only narrow native adapters for credentials, file pickers, notifications, menus, window lifecycle, and later agent installation/update. Each native affordance must invoke a shared web workflow with a browser fallback; no route or screen may test `isTauri` to reveal desktop-only management behavior. Keep the standalone Tauri crate outside the root workspace so headless Rust builds do not acquire GUI dependencies.
**Verify:** `npm --prefix clients/desktop-web run test:tauri-boundary`
**Commit:** `P11.18: add thin Tauri shell`
**Batch:** solo

### P11.19 — Exercise the real Linux Tauri renderer through WebKitGTK
**Status:** DONE
**Files:** `clients/desktop-web/tests/e2e/tauri-linux/`, `clients/desktop-web/wdio.conf.ts`, `tools/phase11/linux-webkitgtk-smoke.sh`, `docs/msc2/clients/evidence/`
**What:** On a Debian/Ubuntu desktop runner with `libwebkit2gtk-4.1`, `webkit2gtk-driver`, and Xvfb, launch the built Tauri binary and drive its real window through the native WebDriver path. Verify visible shell, navigation, CSS layout, forms, dialogs, live-console fallback, deep links, one mutating fake workflow, and the fresh-profile onboarding entry path including the reduced-motion/fallback branch; record the WebKitGTK package/version and screenshot evidence. A Vite page opened in Chrome or Playwright's bundled WebKit does not satisfy this step.
**Verify:** `bash tools/phase11/linux-webkitgtk-smoke.sh --native`
**Commit:** `P11.19: prove Linux WebKitGTK rendering`
**Batch:** stop-after

### Group B — Bedrock extension seam (ready after Group A; no Bedrock screens)

### P11.20 — Prove capability-driven Bedrock extension seams
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/navigation/`, `clients/desktop-web/tests/navigation/bedrock-extension.test.ts`, `docs/msc2/client-capability-matrix.csv`, `docs/msc2/clients/phase11-scope.md`
**What:** Recheck generated TypeScript against the current frozen OpenAPI document and consume its finalized `serverTypes.bedrock` plus `BedrockRuntimeStateDTO` advertisement without hand-written Bedrock DTOs or host-OS inference. Use a test-only future section descriptor to prove Bedrock navigation is absent when unsupported, can be registered when capability state permits it, survives unknown backend/reason values additively, and fits existing layouts/routes without restructuring. Ship no Bedrock section, creation flow, settings, player, allowlist, world, backup, console, or runtime screen in Phase 11; keep those matrix cells Planned for the later Bedrock client group.
**Verify:** `npm --prefix clients/desktop-web run api:check && npm --prefix clients/desktop-web run test:bedrock-extension && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv`
**Commit:** `P11.20: preserve Bedrock client extension seam`
**Batch:** solo

### Group C — Shared agent, auth, packaging, and gate work (ready after Group B)

### P11.21 — Close the remaining desktop and browser authentication design
**Status:** DONE
**Files:** `docs/msc2/clients/phase11-auth.md`, `docs/msc2/msc2-decisions.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/api-contract/auth-scope-phase2.md`, `docs/msc2/lifecycle/pairing-phase4.md`, `crates/msc-api/tests/phase11_auth_conformance.rs`
**What:** Turn D-012's approved mechanisms and Phase 9 posture into one testable contract: same-machine Tauri bootstrap resistant to arbitrary local-process impersonation, per-host remote desktop pairing and secret-store keys, browser pairing-to-httpOnly-SameSite cookie exchange, session revocation/expiry, exact allowed-origin/CSP rules, CSRF tokens for cookie-authenticated mutations, and bearer exemption. Preserve loopback-by-default management and authenticated explicit-Tailscale access; per the owner-confirmed v1 choice above, keep general-LAN management unavailable and do not build certificate provisioning or a local trust system. Use additive versioned routes and `ErrorDTO`; do not put raw credentials in Svelte-accessible storage or URLs.
**Verify:** `python3 tools/api-contract-check.py --v1-summary && cargo nextest run -p msc-api --test phase11_auth_conformance`
**Commit:** `P11.21: freeze desktop and browser auth`
**Batch:** solo

### P11.22 — Implement browser sessions, origin policy, CSP, and CSRF
**Status:** DONE
**Files:** `crates/msc-agent/src/auth/`, `crates/msc-agent/src/routes/browser_session.rs`, `crates/msc-agent/tests/browser_auth.rs`, `clients/desktop-web/src/lib/auth/browser.ts`, `clients/desktop-web/tests/auth/browser.test.ts`
**What:** Implement the frozen browser pairing/session path on the existing credential registry, with one-use challenges, httpOnly cookies, revocation and restart behavior, exact origin checks, restrictive CSP, CSRF on every cookie-authenticated mutation, rate limits, and audit attribution. Prove bearer clients remain unaffected and a hostile origin/local script cannot turn ambient browser authority into a server mutation.
**Verify:** `cargo nextest run -p msc-agent --test browser_auth && npm --prefix clients/desktop-web run test:auth-browser`
**Commit:** `P11.22: add secure browser sessions`
**Batch:** stop-after

### P11.23 — Implement local and remote Tauri credentials per host
**Status:** DONE
**Files:** `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/auth/desktop.ts`, `clients/desktop-web/tests/auth/desktop/`, `crates/msc-agent/src/auth/`, `crates/msc-agent/tests/desktop_auth.rs`
**What:** Implement the chosen same-machine authorization handshake and remote pairing exchange, storing one credential per agent host ID in the platform credential store through the shell so secrets never enter browser storage. Verify local convenience does not become loopback-open authorization, remote credentials obey permission/expiry/revocation, switching hosts cannot reuse another host's credential, and the web build retains its cookie flow with no divergent screen.
**Verify:** `cargo nextest run -p msc-agent --test desktop_auth && npm --prefix clients/desktop-web run test:auth-desktop`
**Commit:** `P11.23: add per-host desktop credentials`
**Batch:** stop-after

### P11.24 — Serve embedded help content and port router-guide rules
**Status:** DONE
**Files:** `crates/msc-domain/src/router_guides.rs`, `crates/msc-domain/tests/router_guides.rs`, `crates/msc-agent/src/help.rs`, `crates/msc-agent/src/routes/help.rs`, `crates/msc-agent/tests/help_routes.rs`, `content/help/`, `content/guides/`, `fixtures/help-content/`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/client-capability-matrix.csv`
**What:** Embed the validated content corpus, implement `GET /v1/help/{helpId}` plus the additive handbook/concept/router-guide catalog routes required to browse it, and port the ledgered router matcher/fallback/composer/troubleshooting rules into Rust against fixtures. Explicitly include the `bedrock.runtime-unavailable` topic and its structured unavailable reasons. Return raw Markdown/structured steps for every client to render; unknown or version-new topics degrade through `ErrorDTO`. Do not absorb client onboarding anchors or presentation into the agent.
**Verify:** `cargo nextest run -p msc-domain --test router_guides && cargo nextest run -p msc-agent --test help_routes && npm --prefix clients/desktop-web run test:screen-help`
**Commit:** `P11.24: serve shared educational content`
**Batch:** solo

### P11.25 — Serve the same production bundle from the agent
**Status:** DONE
**Files:** `crates/msc-agent/src/web_ui.rs`, `crates/msc-agent/tests/web_ui.rs`, `clients/desktop-web/`, `tools/phase11/bundle-identity-check.py`
**What:** Embed or package the exact Svelte production output the Tauri shell loads, serve hashed assets with correct MIME/cache headers and CSP, support safe client-side deep-link fallback without shadowing `/v1`, and provide an explicit unavailable result when a headless package intentionally omits web assets. Prove byte identity between the browser-served and Tauri-loaded bundles and preserve D-011's no-GUI dependency boundary in the agent.
**Verify:** `python3 tools/phase11/bundle-identity-check.py && cargo nextest run -p msc-agent --test web_ui`
**Commit:** `P11.25: serve shared web bundle`
**Batch:** stop-after

### P11.26 — Install and manage the local agent through the shell
**Status:** DONE
**Files:** `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/setup/`, `clients/desktop-web/tests/agent-install/`, `tools/phase11/agent-install-smoke.sh`, `packaging/`
**What:** Add shell-only native commands behind shared setup screens to detect, install, start, stop, repair, and report the platform service and compatible agent/sidecar package. Closing the window must never stop the service or a server. Browser users see the same status/setup route with a truthful instruction/fallback when native install is unavailable; there is no desktop-only screen. Preserve existing service identity, privilege, headless, and rollback rules.
**Verify:** `bash tools/phase11/agent-install-smoke.sh --synthetic`
**Commit:** `P11.26: manage local agent installation`
**Batch:** stop-after

### P11.27 — Define and prove coordinated desktop, agent, and sidecar updates
**Status:** DONE
**Files:** `docs/msc2/clients/phase11-update.md`, `clients/desktop-web/src-tauri/`, `clients/desktop-web/src/lib/updates/`, `packaging/`, `tools/phase11/update-smoke.sh`
**What:** Implement the owner-confirmed prompted, signed macOS/Windows update policy as a compatibility-aware set: download and verify release identity before staging, ask before installation, keep the running agent until replacement is ready, preserve configuration/secrets/worlds, pair the exact compatible Bedrock sidecar where applicable, roll back a failed update, and allow app/agent version skew only within D-010's advertised window. Never install silently. Linux defers installation to its package manager and receives an actionable notice, not a second self-updater. Never merge MSC updates with server/loader/add-on update controls.
**Verify:** `bash tools/phase11/update-smoke.sh --synthetic --all-platforms`
**Commit:** `P11.27: add coordinated client updates`
**Batch:** solo

### P11.28 — Build and exercise desktop and web candidates on all three platforms
**Status:** DONE
**Files:** `.github/workflows/ci.yml`, `tools/phase11/desktop-web-smoke.sh`, `clients/desktop-web/`, `docs/msc2/clients/evidence/`
**What:** Add production frontend/type tests, agent-served browser smoke, and real Tauri builds to macOS, Windows, and Linux CI while preserving the headless no-GUI job. Exercise the same core workflow in browser and desktop modes; on Linux run P11.19's native WebKitGTK smoke, not a Chromium substitute. Record platform renderer/package versions and explicit unavailable signing/notarization evidence without claiming unperformed release distribution.
**Verify:** `bash tools/phase11/desktop-web-smoke.sh --synthetic --all-surfaces`
**Commit:** `P11.28: prove tri-platform clients`
**Batch:** stop-after

### P11.28a — Port the remaining MSC 1 first-time setup pages
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src/lib/help/FirstLaunchGate.svelte`, `clients/desktop-web/tests/screens/help.test.ts`, `clients/desktop-web/tests/e2e/browser/`, `clients/desktop-web/tests/e2e/tauri-linux/`, `crates/msc-agent/src/routes/capabilities.rs`, `crates/msc-agent/src/routes/networking.rs`, `crates/msc-agent/src/routes/versions.rs`, `crates/msc-api/src/dto/`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/client-capability-matrix.csv`, `differences.md`
**What:** Port MSC 1’s Server Setup, Playit.gg, Xbox Broadcast, Tailscale, and You’re All Set pages into the existing MSC 2 first-launch flow. Use real agent probes for the servers root, Java 21+, Bedrock runtime, Xbox helper, and Tailscale; keep optional services skippable; persist the selected server types; and hand off explicitly to first-server creation through the existing Concept Guide.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && npm --prefix clients/desktop-web run build && npm --prefix clients/desktop-web run api:generate -- --check && cargo check -p msc-api -p msc-agent && cargo nextest run -p msc-api --test dto_conformance`
**Commit:** `P11.28a: port remaining first-time setup pages`
**Batch:** stop-after

### P11.28b — Repair first-time setup controls and host probes
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/src-tauri/src/lib.rs`, `clients/desktop-web/tests/tauri/platform-boundary.test.ts`
**What:** Keep Bedrock selectable when the selected built-in runtime is in `provisioning_required`, open setup links through the operating system’s default browser in Tauri, verify the Xbox helper after download by re-reading the agent’s status and filename, and make Tailscale checks visibly report Checking, installed, not installed, or unavailable.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && npm --prefix clients/desktop-web run test:tauri-boundary && npm --prefix clients/desktop-web run build && cargo check -p msc-api -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Commit:** `P11.28b: repair first-time setup controls`
**Batch:** stop-after

### P11.28c — Add native setup pickers and Java verification
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/tests/screens/help.test.ts`, `clients/desktop-web/tests/tauri/platform-boundary.test.ts`, `crates/msc-agent/src/routes/versions.rs`
**What:** Use the native Tauri dialog for the servers-root folder and Java executable Browse actions, retain a manual path fallback in browser mode, and include manually configured Java outside standard search roots in the agent’s real version probe so Check for Java and Use PATH report truthfully.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && npm --prefix clients/desktop-web run test:tauri-boundary && npm --prefix clients/desktop-web run build && cargo check -p msc-api -p msc-agent && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Commit:** `P11.28c: add native setup pickers and Java verification`
**Batch:** stop-after

### P11.28d — Make Xbox helper verification stateful in the test host
**Status:** awaiting verification
**Files:** `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, `clients/desktop-web/tests/e2e/browser/workflows.spec.ts`, `clients/desktop-web/playwright.config.ts`
**What:** Keep the deterministic browser/Tauri test host’s Xbox helper status consistent with its download endpoint, return the helper filename, and exercise the setup flow through the verified-download message so a successful fake download cannot be reported as missing. This remains test-only and must not become a production host.
**Verify:** `npm --prefix clients/desktop-web run test:e2e-browser`
**Commit:** `P11.28d: make Xbox helper verification stateful in the test host`
**Batch:** stop-after

### P11.28e — Wire Tailscale checks through every setup surface
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/src/lib/sections/handbook/HelpSection.svelte`, `clients/desktop-web/tests/screens/help.test.ts`
**What:** Pass the agent API into the Handbook’s compact first-launch setup, which was previously rendered without it, and make the Tailscale Check button report an unavailable connection instead of silently returning when no API is present.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && npm --prefix clients/desktop-web run build`
**Commit:** `P11.28e: wire Tailscale checks through every setup surface`
**Batch:** stop-after

### P11.28f — Reserve port 48001 for MSC2
**Status:** awaiting verification
**Files:** `crates/msc-agent/src/cli/mod.rs`, `crates/msc-domain/src/app_config_schema.rs`, `clients/desktop-web/src-tauri/src/lib.rs`, `packaging/agent-service-layout.json`, platform service tests, `docs/msc2/`, `tools/`, `corpus/server-dirs/README.md`
**What:** Move MSC2’s default management/service port from 48400 to 48001 so it can run beside MSC1 without collisions. Keep MSC1 and historical audit/iOS references on 48400, while updating MSC2’s agent defaults, Tauri installer, service metadata, live-test examples, and assertions.
**Verify:** `cargo fmt --all -- --check && cargo test -p msc-infrastructure --test service_model && cargo test -p msc-platform-macos --test service_plist && cargo test -p msc-platform-linux --test systemd_unit && cargo test -p msc-platform-windows --test service_definition && cargo check -p msc-agent -p msc-domain && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml && bash tools/phase11/agent-install-smoke.sh --synthetic`
**Commit:** `P11.28f: reserve port 48001 for MSC2`
**Batch:** stop-after

### P11.28g — Remove fake hosts from the shipped client
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/navigation/`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/tests/`, `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`
**What:** Remove the hard-coded `demo-agent` and fake host switch from the production shell. Keep D-013’s host-keyed architecture for real configured hosts, but make the initial desktop host the actual local agent and keep fake HTTP/contract-host behavior exclusively in test entry points. The shipped app must never present a fake host as a selectable destination.
**Verify:** `npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run build && ! rg -n "demo-agent" clients/desktop-web/src`
**Commit:** `P11.28g: remove fake hosts from the shipped client`
**Batch:** stop-after

### P11.28h — Connect the Tauri client to the real local agent
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/api/`, `clients/desktop-web/src/lib/auth/desktop.ts`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/tests/auth/desktop/`, `clients/desktop-web/tests/tauri/`
**What:** Wire `DesktopSessionAuth` and its native authorized-request bridge into the real `ApiClient` instead of falling back to cookie authentication in Tauri. Use `http://127.0.0.1:48001` as the local agent origin, obtain/store a host-scoped credential through the approved desktop pairing/bootstrap path, and keep browser sessions on the browser cookie adapter. Do not add a shell-token environment-variable shortcut to the shipped client.
**Verify:** `npm --prefix clients/desktop-web run test:auth-desktop && npm --prefix clients/desktop-web run test:tauri-boundary && npm --prefix clients/desktop-web run build && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Commit:** `P11.28h: connect Tauri to the real local agent`
**Batch:** stop-after

### P11.28i — Start the installed local agent without terminal commands
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/platform/`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src-tauri/src/lib.rs`, `clients/desktop-web/tests/agent-install/`, `clients/desktop-web/tests/tauri/`, `tools/phase11/`
**What:** On desktop launch, inspect the local service through the existing native service adapter. Start an installed-but-stopped MSC2 agent automatically and wait for its health endpoint on `48001`; when the service is not installed, expose one explicit Install action that uses the existing packaged-agent/service path and elevation boundary. Closing the window must not stop the agent or Minecraft servers, and normal operation must not require a terminal.
**Verify:** `bash tools/phase11/agent-install-smoke.sh --synthetic && npm --prefix clients/desktop-web run test:tauri-boundary && cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
**Commit:** `P11.28i: start the local agent from the desktop shell`
**Batch:** stop-after

### P11.28j — Show an honest first-run and agent-unavailable state
**Status:** awaiting verification
**Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/lib/help/FirstLaunchGate.svelte`, `clients/desktop-web/src/lib/help/SetupIntro.svelte`, `clients/desktop-web/tests/screens/`, `clients/desktop-web/tests/agent-install/`
**What:** Replace the generic connection failure path with a clear state model for missing, stopped, starting, ready, incompatible, and unavailable agents. Give the operator the next safe action—install, start, repair, or reconnect—without showing the first-time setup as complete before the real local agent is ready. Preserve browser users’ truthful no-native-service fallback.
**Verify:** `npm --prefix clients/desktop-web run test:screen-help && npm --prefix clients/desktop-web run test:tauri-boundary && npm --prefix clients/desktop-web run build`
**Commit:** `P11.28j: show real agent readiness states`
**Batch:** stop-after

### P11.28k — Walk the real client and agent through first-time setup
**Status:** superseded — deferred into P12.17 (decided 2026-08-25). This exercises the first-time-setup UI that Phase 12 rebuilds (P12.13); its real-agent/auth/first-server-handoff proof runs once against the redesigned client at the Phase 12 gate rather than against the superseded UI. The committed P11.28g–j agent wiring it would have exercised carries forward as Phase 12's foundation.
**Files:** `tools/phase11/real-client-agent-smoke.sh`, `clients/desktop-web/tests/e2e/`, `docs/msc2/clients/evidence/`, `differences.md`
**What:** Add a real-agent smoke path that launches or connects to MSC2 on `48001`, opens the actual Tauri client, authenticates through the real desktop credential path, and exercises the first-time setup from the opening screen through the final first-server handoff. Cover the server-root picker, Java check, Bedrock disclosure, optional helper links/checks, helper download state, Back/Next/Skip behavior, and restart/reopen behavior. Record fake-harness-only coverage separately and update `differences.md` only from this real-client evidence.
**Verify:** `bash tools/phase11/real-client-agent-smoke.sh --macos`
**Commit:** `P11.28k: prove real client and agent setup`
**Batch:** stop-after

### P11.29 — Reconcile the capability matrix and run the exact Phase 11 gate
**Status:** superseded — folded into P12.17 (decided 2026-08-25). Its checks (capability matrix, D-003 bundle/screen identity, generated DTO drift, D-013 host isolation, browser/Tauri auth, native Linux WebKitGTK rendering, tri-platform packaging, headless independence, and the onboarding/first-launch preservation contract) depend on the final client UI, so they run once against the *redesigned* client at the Phase 12 gate. Running this gate against the superseded UI is pointless — its own criterion requires preserving MSC 1's design, which only the redesign delivers.
**Files:** `docs/msc2/client-capability-matrix.csv`, `tools/phase11/phase11-check.py`, `docs/msc2/clients/evidence/phase11-ci.md`, `docs/msc2/clients/phase11-scope.md`, `docs/msc2/rolling-plan.md`
**What:** Run only after P11.28a–P11.28k. Check every contract and WebSocket operation against real Desktop/Web implementation evidence, leaving agent-Planned, Bedrock-screen, and player-profile rows explicitly Planned and no cell blank. Prove D-003 bundle/screen identity, generated DTO drift, D-013 host isolation, capability/permission routing, D-026 served-content use, browser/Tauri auth, browser responsive behavior, native Linux WebKitGTK rendering, tri-platform packaging, headless independence, and the exact green CI candidate. Also require the onboarding preservation contract in `phase11-scope.md`: fresh-profile setup → Concept Guide → guided tour → Handbook behavior, step/anchor coverage, skip/reopen persistence, splash playback/fallback, reduced-motion behavior, and honest separation of agent-owned first-server initiation from client-owned presentation. This is Phase 11's only full-workspace run; the other agent decides in REVIEW whether the literal gate holds.
**Verify:** `python3 tools/phase11/phase11-check.py --gate && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && cargo nextest run --workspace`
**Commit:** `P11.29: check desktop and web gate`
**Batch:** solo

### P11.30 — Allow CORS on the unauthenticated /v1/health probe
**Status:** DONE (committed `0e2cc4f`). Discovered during P12.2 visual verification, not part of any planned step: a dev-mode Tauri window's own `fetch()` to this pre-credential readiness probe is cross-origin (devUrl `:1420` vs agent `:48001`), and the agent sent no CORS header at all, so the probe silently failed and the shell never got past "Agent starting" even though the agent was healthy. This route already runs outside the bearer-auth gate and carries no secrets, so a permissive allowance is safe here.
**Files:** `crates/msc-agent/src/routes/health.rs`
**What:** Set `Access-Control-Allow-Origin: *` on the `GET /v1/health` response only.
**Verify:** `curl -s -D - http://127.0.0.1:48001/v1/health` (with the agent running) shows the header; `cargo nextest run -p msc-agent --test browser_auth --test desktop_auth`.
**Commit:** `P11.30: allow CORS on the unauthenticated /v1/health probe`
**Batch:** solo

### P11.31 — Finish and land the local-agent auto-bootstrap auth
**Status:** DONE. Full account in `toughproblems/local-agent-auth-bootstrap.md` — read that before touching this area again. Discovered during P12.2 visual verification (same as P11.30): the real Tauri app could not connect to the local agent at all, through six stacked, independent bugs. Found `git stash@{0}` ("WIP: local-agent auto-bootstrap auth (pre-P12.1)") — Cameron's own substantial prior implementation of the same-machine bootstrap channel `docs/msc2/clients/phase11-auth.md` describes but this codebase never actually had working — and applied it (clean; only `App.svelte` had a trivial non-conflicting overlap with P12.2) rather than rewriting from scratch. Fixed forward from there: (1) the LaunchDaemon needs macOS admin elevation to start/stop, now done via `osascript ... with administrator privileges`; (2) ad-hoc-signing an already-running dev binary invalidates its own live code identity, fixed by signing-then-re-exec at startup instead of signing in place; (3) **keychain-based storage for the installation key and the store's root key was removed entirely** — both are now plain 0600 files under the agent's own secrets directory, since the agent and its desktop shell always run as the same regular user and keychain's ACL/session semantics kept failing in non-interactive-daemon contexts in ways that were each individually correct-looking and still wrong in practice; (4) a `ClientProof` struct was missing `#[serde(rename_all = "camelCase")]`, so every bootstrap proof submission failed JSON parsing (`hostId` sent, `host_id` expected) before it was ever checked. Real dev-workflow caveat recorded in the toughproblems doc: the installed plist's `MSC2_MACOS_DESKTOP_REQUIREMENT` snapshots one specific run's ad-hoc cdhash, which changes on every `tauri dev` relaunch — click "Repair service" once, right after the app under test opens, and don't restart it again before testing.
**Files:** `crates/msc-platform-macos/src/service.rs`, `crates/msc-platform-macos/src/secret_store.rs`, `crates/msc-platform-macos/Cargo.toml`, `crates/msc-agent/src/auth.rs`, `crates/msc-agent/src/auth/desktop.rs`, new `crates/msc-agent/src/auth/local_bootstrap.rs`, `crates/msc-agent/src/main.rs`, `crates/msc-agent/Cargo.toml`, `clients/desktop-web/src-tauri/src/lib.rs`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/auth/desktop.ts`, `clients/desktop-web/src/lib/platform/{index,browser,tauri,types}.ts`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, associated tests, new `toughproblems/local-agent-auth-bootstrap.md`
**What:** Make the real Tauri app able to authenticate to its own freshly-installed local agent without any manual pairing step, end to end, on macOS.
**Verify:** `cargo nextest run -p msc-platform-macos && cargo nextest run -p msc-agent --test browser_auth --test desktop_auth`; `npm --prefix clients/desktop-web run test:tauri-boundary && npx vitest run clients/desktop-web/tests/agent-install/agent-install.test.ts && npm --prefix clients/desktop-web run test:auth-desktop`. Cameron's own real-app verification: fresh install → Repair service → Reconnect connects without a manual pairing code.
**Commit:** `P11.31: land the local-agent auto-bootstrap auth`
**Batch:** solo

### P11.31a — Move the desktop app's own stored credential off keychain too
**Status:** awaiting verification. Cameron hit a real login-keychain access prompt ("msc2-desktop-web wants to use your confidential information stored in 'com.ctemple.msc2.desktop'...") during P12.3 verification and correctly recalled that keychain had been removed — P11.31's "removed entirely" only covered two *agent-daemon* secrets (installation key, store root key). A third secret was missed: the Tauri desktop app's own stored pairing credential (`StoredDesktopCredential`), still going through `MacosSecretStore::default_keychain_for_service` (the real login keychain) in `desktop_secret_store()`. Fixed to `MacosSecretStore::system()` — the same self-provisioning, file-rooted store the agent itself uses, sharing `agent_data_directory()`'s `secrets/` directory with the `local-bootstrap.key` file this same process already writes there directly (same user, same reasoning P11.31 already established for the other two secrets). Removed the now-unused `DESKTOP_SECRET_SERVICE` constant. Left `crates/msc-agent/src/auth.rs`'s own `default_keychain_for_service` call untouched — that one backs the foreground-`msc serve`-smoke-harness path the module's own doc comment already carves out as deliberate, not a fourth instance of this gap. Full account appended to `toughproblems/local-agent-auth-bootstrap.md` (§5).
**Files:** `clients/desktop-web/src-tauri/src/lib.rs`, `toughproblems/local-agent-auth-bootstrap.md`
**What:** Stop the Tauri desktop app's own credential store from touching the macOS login keychain, matching the reasoning and pattern P11.31 already applied to the agent's two other secrets.
**Verify:** `cd clients/desktop-web/src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test`. Cameron's own real-app verification: fresh pair/reconnect on macOS no longer shows a Keychain access prompt.
**Commit:** `P11.31a: move the desktop app's own stored credential off keychain too`
**Batch:** solo

---

## Phase 12 — Client redesign (MSC 1 visual + behavioral fidelity)

**Gate** (`msc2-port-plan.md` §3): every MSC 1 screen and sheet has a rebuilt
MSC 2 counterpart that (a) matches MSC 1's shape and behavior — verified
screen-by-screen against MSC 1 by Cameron — and (b) passes the `antiAIslop.md`
checklist. The data/agent layer is unchanged in contract, and no screen exists in
the desktop app that is absent from the web UI (D-003 corollary).

**Why this phase exists.** Phase 11 delivered a working desktop/web client wired
to the real agent, but its UI diverged from MSC 1's information architecture and
design language — the very thing Phase 11 was meant to preserve. Phase 12 course-
corrects: it rebuilds the presentation layer so MSC 2 looks *and behaves* like
MSC 1, **refreshed** into one consistent, deliberately-designed system (MSC 1
grew by accretion and is inconsistent with itself; the refresh unifies it —
cleaner, less bulky, more modern, without becoming a different app).

**Keep / replace line (do not cross it).**
- **KEEP untouched** — the data layer that talks to the agent: `src/lib/api/`,
  `src/lib/platform/`, `src/lib/auth/`, `src/lib/stores/`, `src/lib/streams/`,
  `src/lib/operations/`, `src/lib/hosts/`. Backend/agent/Tauri/Rust changes are
  allowed only when a screen genuinely needs one, and are called out per step.
- **REPLACE** — the presentation: `src/App.svelte`, `src/lib/components/`,
  `src/lib/sections/*`, `src/lib/navigation/`, the shell.

**Required reading before any step:** `docs/msc2/antiAIslop.md` (hard rule #11)
and the locked reference specimens in `docs/msc2/renderings/`
(`status-card`, `buttons-and-type`, `primitives`, `shell`,
`decorated-vs-disciplined`). MSC 1 is the oracle for shape + behavior; its source
is at `~/Documents/Swift Projects/minecraft-server-controller/MSCmacOS/` and its
screenshots at `~/Documents/MSCSS/`. Each step begins by reading that screen's
own MSC 1 view(s) for behavior, not just its screenshot.

**How Phase 12 verification differs (recorded deviation from the normal loop).**
This is a *design* phase. Its `Verify:` is Cameron's **visual review** of the
running screen against MSC 1 plus the anti-slop checklist — not a single runnable
command. Two normal hard rules are relaxed here, deliberately:
- **Verify is a visual review**, not a pass/fail command. Where a cheap structural
  component test exists (`vitest`), the step names it too, but the *gate* is the
  eye, and the loop is: execute → Cameron looks → adjustment requests → repeat.
- **More than one commit per step is expected** — one commit per view/sheet, plus
  adjustment commits from the review loop. The "one commit per step" rule does not
  bind Phase 12.
Every step's Batch is therefore `solo`.

**Sections.** S0 (design system) and S1 (shell) were shaped and locked in advance
as the reference specimens; their steps below turn those locked specimens into
code. The rest apply the locked system to each screen, one at a time.

If a screen needs a UI pattern not covered by the locked S0 primitives, stop and surface it to Cameron as a design decision — extend the system deliberately (and add a renderings/ specimen), never improvise a one-off.

### P12.0 — Implement the locked design system (S0) as code
**Status:** DONE. Implemented in `src/lib/styles/tokens.css` (new `--msc2-*` tokens appended alongside the untouched Phase 11 `--msc-*` tokens, which unconverted screens still use) and `src/lib/components/base/` (Card, Button, SegmentedControl, Toggle, Field, NumberField, Select, Badge, ListRow, EmptyState, StatusDot, Sheet). A dev-only component gallery lives at `clients/desktop-web/gallery.html` (open via `npm run dev`, then visit `/gallery.html`) for the visual comparison this step's Verify calls for — it is not linked from the shipped app. `src/app.css`'s root font-family now points at the locked system-sans stack instead of Inter, since the type scale depends on it.
**Files:** `src/lib/styles/tokens.css`, `src/lib/styles/`, `src/lib/components/` (base components), reference `docs/msc2/renderings/`
**What:** Turn the locked S0 specimens into real code: `tokens.css` (4 surface tiers, status ramp, spacing/radius scales, the 7-role type scale, opacity text steps) and the base Svelte components matching the specimens exactly — `Card`, the button set (Primary/Start/Stop/Secondary/Destructive/GhostIcon in md/sm), `SegmentedControl`, `Toggle` (green-on), `Field`/`NumberField`/`Select`, `Badge` (category/status), `ListRow`, `EmptyState`, `StatusDot`, and the `Sheet` frame with the three fixed widths (480/640/820). No screen wiring yet.
**Verify:** `cd clients/desktop-web && npm run test:unit`, then `npm run dev` and visually compare each rendered base component to its `docs/msc2/renderings/*.html` reference; run the `antiAIslop.md` checklist against the component gallery.
**Commit:** `P12.0: implement the locked design system`
**Batch:** solo

### P12.1 — Build the app shell (S1)
**Status:** DONE. Cameron verified the running shell against `renderings/shell.html` and the MSC 1 screenshots on 2026-08-25.
**Files:** `src/App.svelte`, `src/lib/components/ApplicationShell.svelte`, shell subcomponents, `src/lib/navigation/`
**What:** Build the shell skeleton per `renderings/shell.html` and MSC 1 (`ContentView`, `SidebarView`, `DetailsHeaderSectionView`, `MSCTabBar`): window chrome + `bannerColor` system + terrain banner (static-faithful, animation deferred), sidebar control rail with the **host-aware picker** (Host ▸ Server + Manage…) and collapsible sections, header, 8-tab strip (selected pill = `bannerColor`), and the docked collapsible console *frame* (console behavior is P12.10). Wire it to the kept navigation/host stores. Adjustment rounds (2026-08-25): dropped the picker's connection-status dot — it had no visible label, so it read as antiAIslop tell #12 (a "meaningless status dot"); connection state is already stated by the agent-unavailable panel. Made the console dock manually resizable (drag the handle above it), matching MSC 1 ContentView.swift's consoleDivider — drag-past-the-floor-and-release collapses it. Found during review: the real player-avatar feature (Java/Bedrock skin lookup) wasn't assigned to any P12.x step — recorded as new step P12.1a rather than built here.
**Verify:** `cd clients/desktop-web && npm run dev`; compare the running shell to `renderings/shell.html` and `~/Documents/MSCSS/Main View` + `SIdebar`; run the anti-slop checklist. Structural: `npm run test:visual-shell`.
**Commit:** `P12.1: build the app shell`
**Batch:** solo

### P12.1a — Build the sidebar player avatar
**Status:** DONE. Moved the avatar out of P12.1's fixed footer into the scroll flow as an "Actions" block, matching MSC 1's placement (`SIdebar` screenshot) — the old sticky footer was too short for the real content (segmented toggle, entry row, 160px rendered skin). Dropped the idle-sway animation: antiAIslop's design law #8 reserves the app's one deliberate flourish for the terrain banner, and a second sway on the avatar would compete with it. Java fetch uses an `Image()` load/error probe against `minotar.net` rather than `fetch()`, so it isn't blocked by CORS on a plain `<img>`-style render. New `src/lib/player/avatarIdentity.ts` holds edition/identity in global (not host/server-scoped) `localStorage` keys, since this is the human's own identity, not per-server agent data. Adjustment round (2026-08-25): Cameron caught the block flowing in-place right after Quick Commands instead of sitting at the bottom of the rail like MSC 1's Spacer-pushed layout — fixed by making `.scroll` a flex column with `margin-top: auto` on `.actions-block`, which still degrades to normal scroll once content overflows.
**Files:** `src/lib/components/shell/ControlSidebar.svelte`, a new `src/lib/components/shell/PlayerAvatar.svelte`, a new `src/lib/player/avatarIdentity.ts`
**What:** Replace P12.1's inert "Your avatar" placeholder with MSC 1's real `PlayerAvatarView`: a Java/Bedrock segmented toggle, username/gamertag entry (Add/Change), and the rendered full-body skin. Persist the chosen edition and identity client-locally per MSC 1's `minecraftUsername`/`minecraftBedrockGamertag`/`minecraftAvatarEditionRawValue` config fields (this is personal app identity, not per-server agent data — same "client-local until a real field exists" treatment P12.1 gave `bannerColor`). The Java path is a plain public image fetch (`minotar.net/body/{username}/160`) and can be built in full. The Bedrock path in MSC 1 delegates to `BedrockSkinFetcher` (join-cache, then live Xbox lookup, then dotted-gamertag fallback) — that resolver depends on the player-profile/Xbox work Phase 11's scope doc explicitly defers to a later phase, so Bedrock here should degrade honestly (a truthful "not available yet" state), not fake a working lookup. Reference MSC 1 `PlayerAvatarView.swift` and the `SIdebar` screenshot. The idle-sway animation is MSC 1's only avatar flourish; keep it or drop it per antiAIslop's "motion must serve a purpose" call at build time.
**Verify:** `npm run dev`, add a Java username, confirm the skin renders and persists across reload; confirm Bedrock shows an honest unavailable state; compare to MSC 1 + anti-slop checklist.
**Commit:** `P12.1a: build the sidebar player avatar`
**Batch:** solo

### P12.2 — Overview tab
**Status:** DONE. Cameron verified the running Overview tab on 2026-08-25. Original build: rebuilt `HomeSection.svelte` (the `home` section id — `PRIMARY_TABS` labels it "Overview") as an orchestrator over six new sub-components in `src/lib/sections/home/`: `ConnectionCard`, `LiveStatsCard`, `HealthGrid`, `PlayersCard`, `ActiveWorldCard`, `ChatCard`, matching MSC 1's zone order (Status → Server Health → Activity → Notes). Added `src/lib/components/base/Icon.svelte`, a small neutral content-icon set (the status-card.html spec keeps a neutral icon next to each card's label, unlike antiAIslop's default "drop the icon" — this follows the locked exception). Health cards dropped MSC 1's 3D flip, colored side rail, and colored icon-in-box entirely: one face per card, status as dot + label only, per `renderings/status-card.html`. Real contract gaps found and handled honestly rather than faked: `ServerDTO.hostAddress` is always `null` today (`crates/msc-api/src/dto/lifecycle.rs`), so Local connection info shows the real game port and states the LAN IP as "Not reported by this host yet" instead of inventing one; the backend's actual health-card ids (`directory`/`java`/`ram`/`lastStartup`/`portReachability`/`componentJars`, `crates/msc-agent/src/routes/health.rs`) differ from MSC 1's Swift model (no `jar`/`port`/`vm` ids), so the essential/secondary split was dropped and the grid renders whatever cards the agent actually returns, including its honest "gray — not yet implemented" cards; there is no EULA-acceptance signal on any route today (no `eulaAccepted` field, no `diagnostics.crash` "eula" problem kind), so MSC 1's EULA alert banner is omitted rather than wired to nothing; the in-game world clock needs `level.dat` day/time data the contract doesn't expose, so it's omitted from Active World; Notes has no backend field, so it's client-local per host+server (`src/lib/sections/home/notes.ts`), same treatment P12.1a gave avatar identity. Chat reuses a TypeScript port of MSC 1's `ChatFeedParser` (`chatFeed.ts`) over a point-in-time `/v1/console/tail` snapshot rather than the live stream P12.10 owns. Overview polls `/v1/health`, `/v1/connectivity`, `/v1/performance`, `/v1/servers`, `/v1/config/geyser`, `/v1/players`, `/v1/worlds`, `/v1/settings`, and `/v1/console/tail` every 8s. Added `onWorlds` callback wiring in `App.svelte` for the Active World card's Switch action. Adjustment rounds (2026-08-25): (1) the three Live Stats gauges had a fixed 128px height so they didn't fill the card when it stretched to match Connection Info's height — `LiveStatsCard.svelte` now wraps its content in a flex column and the gauges row flexes to fill; (2) the Public toggle in Connection Info swapped to a different single-column layout with different card sizing — rebuilt so Local/Public always render the same two-column shape, only the per-cell IP/port/source-tag values change; (3) collapsing the sidebar left the Overview content at a fixed max-width instead of filling the freed space — removed the hardcoded `max-width: 1100px`; (4) added a console show/hide button in the top-right (`ShellIcon`'s new `console` glyph, wired through `TopBar`/`ApplicationShell`) alongside the existing sidebar-toggle button, matching that button's placement and style, while keeping the console's existing manual resize-drag behavior untouched. Server Health showing only "Waiting for the agent to report health data." is the correct empty state for a fresh install with no server yet, not a defect — confirmed with Cameron.
**Files:** `src/lib/sections/home/HomeSection.svelte`, new `src/lib/sections/home/{ConnectionCard,LiveStatsCard,HealthGrid,PlayersCard,ActiveWorldCard,ChatCard}.svelte`, new `src/lib/sections/home/{notes,chatFeed}.ts`, new `src/lib/components/base/Icon.svelte`, `src/App.svelte`, `src/lib/components/shell/{ShellIcon,TopBar}.svelte`, `src/lib/components/ApplicationShell.svelte`
**What:** Rebuild Overview to MSC 1's three bands — STATUS (Connection Info w/ Local/Public + Live Stats), SERVER HEALTH (Components/Port/Last Start, disciplined status cards), ACTIVITY (Players / Active World w/ Switch·Backup / Chat), NOTES. Reference MSC 1 `DetailsOverviewTabView` + the `Overview*CardView` files (incl. the chat/advancement parser card) and `~/Documents/MSCSS/Tabs` Overview shot + `Main View`.
**Verify:** `npm run dev`, open Overview stopped and running; compare to MSC 1 Overview + `renderings/status-card.html`; anti-slop checklist. Structural: `npm run test:screen-live`.
**Commit:** `P12.2: rebuild the Overview tab`
**Batch:** solo

### P12.2b — Extract Java player-data NBT fixtures
**Status:** DONE
**Files:** `fixtures/player-nbt/`
**What:** Extract fixtures characterizing `PlayerNBTReader.swift`'s Java player `.dat` parsing (378 lines, gzip-compressed big-endian NBT): the `extractStats` fields (health/maxHealth/foodLevel/xpLevel/xpTotal/gameMode/posX/posY/posZ/dimensionDisplay/score — these already match `PlayerStatsDTO` in `docs/msc2/api-contract/openapi.json` field-for-field, confirming the frozen contract was modeled on this exact reader) and `extractInventory` (slot/itemID/iconName/count/displayName/enchantments/damage — matching `InventoryItemDTO`), plus corrupt/truncated/non-compound-root failure cases (same three-way split P6.7 used for `level.dat`). `crates/msc-domain/src/nbt.rs` (P6.9) already implements a general big-endian tag-level reader — this step's characterization must say explicitly which of `PlayerNBTReader`'s behavior is generic tag parsing already covered by `nbt.rs` versus player-`.dat`-specific extraction rules, so P12.2c doesn't rebuild the reader from scratch. Per the P6.3/P6.7 real-evidence precedent, at least one real player `.dat` sample is required rather than an entirely synthetic fixture set — but per that same precedent, "real evidence" means *at least one genuine file proving the gzip/parse pipeline works end-to-end*, not that every tag variant must be physically demonstrated in a live save. P6.7 itself paired two real `level.dat` files with synthetic characterization for cases no real evidence existed for (the Bedrock little-endian header, "synthesized from the source, not stood in as real evidence" — see that step's own committed note in `rolling-plan-archive.md`). Apply the same split here: any already-available real player `.dat` with at least one inventory item (a stacked item is enough — MSC 1's live `campak` server already has one, no new server session or manual enchanting/damaging needed) satisfies the real-evidence bar. Enchantment, damage, custom-name, and other tag-shape variants `extractInventory` handles may be characterized with hand-built synthetic NBT bytes grounded in `PlayerNBTReader.swift`'s own field-reading code, clearly labeled synthetic in the fixture, exactly like P6.7's Bedrock-header cases. Git-ignore the real sample's raw bytes the same way `fixtures/world-nbt/samples/` does.
**Verify:** `python3 tools/fixture-runner/run.py --validate-dir fixtures/player-nbt --expect 13`
**Commit:** `P12.2b: extract Java player-data NBT fixtures`
**Batch:** solo

### P12.2c — Port the Java player NBT reader
**Status:** DONE
**Files:** `crates/msc-domain/src/player_nbt.rs`, `crates/msc-domain/src/lib.rs`, `crates/msc-domain/tests/player_nbt.rs`
**What:** Port `extractStats`/`extractInventory` against the P12.2b fixtures, reusing `nbt.rs`'s existing tag-level reader per that step's findings rather than re-implementing gzip/tag parsing — same domain-module convention P6.9 established (pure computation only, no filesystem access; I/O stays in `msc-infrastructure`/callers).

Mirror `PlayerProfile.swift`/`PlayerStats`/`InventoryItem`/`ItemEnchantment`'s exact type split (`PlayerProfile.swift:75-176`) rather than reshaping to the wire DTOs here — DTO shaping is P12.2e's job, not this step's:

```rust
pub struct PlayerStats {
    pub health: f32,
    pub max_health: f32,
    pub food_level: i32,
    pub xp_level: i32,
    pub xp_total: i32,
    pub game_mode: i32,     // 0=Survival, 1=Creative, 2=Adventure, 3=Spectator
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub dimension: String,  // raw NBT string, e.g. "minecraft:overworld" — NOT the display form
    pub score: i32,
}
impl PlayerStats {
    pub fn game_mode_display(&self) -> String { /* port PlayerStats.gameModeDisplay's switch, PlayerProfile.swift:89-97, verbatim */ }
    pub fn dimension_display(&self) -> String { /* port PlayerStats.dimensionDisplay, PlayerProfile.swift:99-109, verbatim: "minecraft:overworld"->"Overworld", "minecraft:the_nether"->"Nether", "minecraft:the_end"->"The End", else the component after the last ':' with '_' replaced by ' ' then title-cased */ }
}

pub struct ItemEnchantment { pub id: String, pub level: i32 }
impl ItemEnchantment {
    pub fn display_name(&self) -> String { /* port ItemEnchantment.displayName, PlayerProfile.swift:159-164: component after last ':' in `id`, '_'->' ', title-cased, then " " + (roman numeral I-V if level<=5, else the raw integer) */ }
}

pub struct InventoryItem {
    pub slot: i32,
    pub item_id: String,
    pub count: i32,
    pub enchantments: Vec<ItemEnchantment>,
    pub custom_name: Option<String>,
    pub damage: i32,
}
impl InventoryItem {
    pub fn display_name(&self) -> String { /* port InventoryItem.displayName, PlayerProfile.swift:139-144: custom_name if Some and non-empty, else the icon_name prettified the same way (see below) */ }
    pub fn icon_name(&self) -> String { /* port InventoryItem.iconName, PlayerProfile.swift:147-149: component of item_id after the last ':' */ }
}

pub fn extract_stats(root: &NbtValue) -> Option<PlayerStats> { /* ports PlayerNBTReader.extractStats */ }
pub fn extract_inventory(root: &NbtValue) -> Vec<InventoryItem> { /* ports PlayerNBTReader.extractInventory */ }
pub fn read_all(gzip_bytes: &[u8]) -> (Option<PlayerStats>, Vec<InventoryItem>) { /* ports PlayerNBTReader.readAll: gunzip, parse root compound via nbt.rs, call both extract fns; any failure at any stage returns (None, vec![]), never panics */ }
```

Title-case note: Swift's `.capitalized` title-cases *every* whitespace-separated word in the string, not just the first character of the whole string — port that exactly (e.g. two-word item/dimension names must come out with both words capitalized).
**Verify:** `cargo fmt --check && cargo clippy -p msc-domain --all-targets -- -D warnings && cargo nextest run -p msc-domain --test player_nbt` (NOT `-p msc-domain player_nbt` with no `--test` — that form makes cargo compile all 38 of `msc-domain`'s separate integration-test binaries before applying the name filter, which is why earlier runs of this pattern took far longer than the tests themselves warrant; `--test player_nbt` builds only this one binary)
**Commit:** `P12.2c: port the Java player NBT reader`
**Batch:** solo

### P12.2d — Port the Java player-profile pipeline
**Status:** DONE
**Files:** `crates/msc-application/src/player_profiles.rs`, `crates/msc-application/src/lib.rs`, `crates/msc-application/tests/player_profiles.rs`
**What:** Port `loadPlayerProfiles` (`AppViewModel+PlayerProfiles.swift:77`, backed by `PlayerDataManager.swift`) for Java. Do not port UUID/Mojang resolution, skin override, or the migrate/copy/duplicate/delete mutation actions here — skin is P12.2f, mutations are P12.2i; `copyPlayerData` (overwrite-one-onto-another) stays deferred per this file's 2026-08-25 note.

Struct (mirrors `PlayerProfile.swift:13-30`'s Java-relevant fields only — skip `xuid`/`floodgateUUID`, Bedrock-only):
```rust
pub struct JavaPlayerProfile {
    pub uuid: Uuid,
    pub username: Option<String>,
    pub dat_file_path: PathBuf,
    pub last_modified: SystemTime,
    pub is_online: bool,
    pub is_op: bool,
    pub is_hidden: bool,
    pub stats: Option<PlayerStats>,      // P12.2c type, populated eagerly (see below)
    pub inventory: Vec<InventoryItem>,   // P12.2c type, populated eagerly
}
```

Directory scan (`PlayerDataManager.swift:21-68`) — two candidate directories, **both** scanned when both exist (this differs from the single-directory resolution P12.2i's mutation functions use — don't conflate the two):
- `{server_dir}/{level_name}/playerdata/`
- `{server_dir}/{level_name}/players/data/` (some Paper configs)

`level_name` comes from the same resolution `crates/msc-application/src/backups.rs:220` already uses: `msc_application::worlds::read_java_level_name(fs, server_dir)` passed through `msc_domain::world::current_level_name(ServerType::Java, raw.as_deref())` — reuse that exact two-line pattern, don't re-derive the Java default level name yourself.

For each existing directory (in the order listed above), list entries via `fs.list`, keep only names ending in `.dat` (exclude `.dat_old`), parse the filename stem as a UUID, and skip any UUID already seen from an earlier directory in this scan (first-seen wins — matches Swift's `seen: Set<UUID>` dedup order exactly).

`usercache.json` (`{server_dir}/usercache.json`, JSON array of `{"name": string, "uuid": string}`, `PlayerDataManager.swift:70-92`) → username lookup. `ops.json` (`{server_dir}/ops.json`, JSON array of `{"uuid": string}`, `PlayerDataManager.swift:94-108`) → `is_op` set. Both: missing file or malformed JSON → empty map/set, not an error (matches Swift's `try?`-and-fall-through).

`is_online`: call `output_reducer`'s existing `online_players() -> &[String]` (already tracks live Java names from console join/leave parsing — no new tracking needed) and match by username (case-sensitive, matching Minecraft usernames' own case-sensitivity).

Hidden set — file is **`{server_dir}/java_hidden.json`** (JSON array of lowercase UUID strings, at the server root, not under the world/level directory — pinned exactly from `JavaHiddenProfiles.swift:9,16-42`, do not invent a different name or shape). Missing file → empty set. Provide `is_hidden`, `hide`, `unhide` functions operating on this file, writing via `msc_infrastructure::atomic_write::atomic_write` (same import `bedrock_players.rs` already uses for its own JSON sidecars).

Read stats+inventory via P12.2c's `read_all` for every scanned profile in this same pass, eagerly (not lazy/async like Swift's UI-driven version) — every mutation route in P12.2j already re-runs this whole scan afterward to build its response, so there is no separate "lazy load, fill in later" state to replicate; this is a deliberate simplification for a synchronous request/response server, not a question for Cameron.

Error type — define `PlayerProfileError` in this module: variants `ProfileNotFound`, `UsernameUnknown`, `Io(std::io::Error)`. This is the *only* place this enum is defined; P12.2i extends its usage (no new variants needed) and P12.2j maps it to HTTP responses — don't create a second error type anywhere else in this feature.

Shape the result (`Vec<JavaPlayerProfile>`) so P12.2e can map it alongside Bedrock's existing `BedrockPlayerRecord` (`bedrock_players.rs`) into one `PlayerProfileDTO` list.
**Verify:** `cargo fmt --check && cargo clippy -p msc-application --all-targets -- -D warnings && cargo nextest run -p msc-application --test player_profiles` (NOT a bare `-p msc-application player_profiles` filter — `msc-application` has 60 separate integration-test binaries and a package-wide filter compiles all of them first; `--test player_profiles` builds only this one)
**Commit:** `P12.2d: port the Java player-profile pipeline`
**Batch:** solo

### P12.2e — Wire GET /v1/players/profiles and POST /v1/players/hidden
**Status:** DONE
**Files:** `crates/msc-agent/src/routes/players.rs` (new — the shared `/v1/players` route currently lives misnamed inside `routes/bedrock.rs`), `crates/msc-agent/src/routes/mod.rs`, `crates/msc-agent/src/routes/templates.rs`, `crates/msc-agent/src/main.rs`, `crates/msc-application/src/bedrock_players.rs`
**What:** Implement the two already-frozen, currently-unimplemented routes from `docs/msc2/api-contract/openapi.json` — `GET /v1/players/profiles` → `PlayerProfilesResponseDTO` and `POST /v1/players/hidden` — for Java (P12.2d) and Bedrock (existing `bedrock_players.rs`), merged into one list. Leave `skinOverrideIdentifier`/`hasSkinFileOverride` present but unpopulated (P12.2f).

`id` scheme, ported from `PlayerProfile.id` (`PlayerProfile.swift:43`): Java profiles use the bare lowercase UUID string; Bedrock profiles use `"xuid_{xuid}"`. A merged list contains both kinds, so `POST /v1/players/hidden`'s `profileId` must be dispatched by prefix — `xuid_` present → Bedrock, else → Java UUID — to know which backend's hidden-store to write.

**Bedrock currently has no write path for hidden profiles** — `bedrock_players.rs` only has `load_hidden` (read-only, line 162). Add a `set_hidden(fs, server_dir, xuid, hidden: bool) -> Result<(), BedrockPlayerError>` there, following the exact same `write_json`-based pattern `load_name_cache`/its own save function already use in that file (`bedrock_players.rs:150-159`), writing back to the same `bedrock_hidden.json`. Without this, the frozen route can only ever half-work.

Java → `PlayerProfileDTO` field mapping (every field, no gaps):
- `id` = `profile.uuid` as lowercase hyphenated string
- `username` = `profile.username`
- `imageIdentifier` = `profile.uuid` as lowercase string with hyphens **removed** (ports `PlayerProfile.imageIdentifier`'s Java fallback branch, `PlayerProfile.swift:60-62`)
- `isOnline`, `isOp`, `isHidden` = direct from `JavaPlayerProfile`
- `isBedrockPlayer` = `false`
- `lastSeen` = `profile.last_modified` formatted as ISO8601. `system_time_to_iso8601` already exists for exactly this in `routes/templates.rs:55` but is private to that file — move it to `routes/mod.rs` as `pub(crate) fn`, update `templates.rs` to call `super::system_time_to_iso8601`, and call the same function here. Don't write a second date-formatting function.
- `skinOverrideIdentifier`/`hasSkinFileOverride` = omit (P12.2f)
- `stats` = `None` if `profile.stats` is `None`, else `Some(PlayerStatsDTO { health, maxHealth: max_health, foodLevel: food_level, xpLevel: xp_level, xpTotal: xp_total, gameMode: game_mode, gameModeDisplay: stats.game_mode_display(), posX: pos_x, posY: pos_y, posZ: pos_z, dimensionDisplay: stats.dimension_display(), score })` — note `dimension` (raw) is dropped, only `dimensionDisplay` goes on the wire
- `inventory` = `profile.inventory` mapped to `InventoryItemDTO { slot, itemID: item_id, iconName: item.icon_name(), count, displayName: item.display_name(), enchantments: [...], damage }`, `enchantments` mapped to `ItemEnchantmentDTO { id, level, displayName: e.display_name() }` — always `[]`, never omitted, if empty

Bedrock → `PlayerProfileDTO`: reuse `BedrockPlayerRecord`'s existing fields for identity/`isOnline`-equivalent state. `BedrockPlayerRecord` today only carries `has_stats: bool`/`inventory_items: usize` (counts, not the actual parsed data) — it cannot fully populate `stats`/`inventory` yet. That's a known, pre-existing gap, **not something to fix in this step**: emit `stats: None`, `inventory: []` for Bedrock profiles here rather than extending `bedrock_players.rs`'s data model, which is separate scope from what Cameron asked for (the Java viewer).

Tests: put them inline as `#[cfg(test)] mod tests` inside `routes/players.rs` itself, matching the convention every other route file with tests already uses (e.g. `routes/settings.rs`) — not a separate `crates/msc-agent/tests/players.rs` integration file. `msc-agent` has no lib target (only the `msc` bin), so this step's `--bin msc` Verify scoping only finds tests that live inside that binary's own source tree.
**Verify:** `cargo fmt --check && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --bin msc players` (NOT a bare `-p msc-agent players` filter — `msc-agent` has 36 separate integration-test binaries and no lib target, so a package-wide filter compiles all of them first; `--bin msc` builds only the agent binary these tests actually live in)
**Commit:** `P12.2e: wire GET /v1/players/profiles and POST /v1/players/hidden`
**Batch:** solo

### P12.2f — Wire player skin resolution and override
**Status:** DONE
**Files:** `crates/msc-agent/src/routes/players.rs`, `crates/msc-application/src/player_skin.rs` (new)
**What:** Port `playerSkinProvider`/`playerSkinOverrideProvider` (`AppViewModel+APIWiringContent.swift:68-147`) for `GET /v1/players/{profileId}/skin` and `POST /v1/players/skin-override`.

**Scope cut from Swift's version, and why:** `PlayerSkinOverrideRequestDTO` in the frozen contract has only `profileId`/`lookupIdentifier` — no field for uploading a skin file at all. So the local-skin-file-upload path (`PlayerSkinStore.saveSkin`, the `hasSkinFileOverride`/`skinFileName` half of Swift's model, and the face-crop-from-a-full-skin-texture code in `PlayerSkinRenderer.swift`) has nothing to attach to on the wire today and is **out of scope for this step** — not a simplification to double check with Cameron, just a direct consequence of what's already frozen. Leave `hasSkinFileOverride` always `false` in `PlayerProfileDTO` (P12.2e). Do not add the `image` crate or any PNG-decoding dependency; this step needs none.

Override storage — file is **`{server_dir}/player_overrides.json`** (JSON object, `profileId -> {"lookupIdentifier": string|null, "skinFileName": string|null}`, pinned from `PlayerSkinStore.swift:19-39`; keep the unused `skinFileName` key in the Rust struct too, always `None`, so the file format doesn't need to change when skin upload is eventually added). Missing file → empty map.

`GET /v1/players/{profileId}/skin` (ports `resolveAppearance`, `PlayerSkinStore.swift:94-111`, minus the skin-file branch per the cut above):
1. Resolve `identifier`: the stored override's `lookupIdentifier` if set and non-empty, else `PlayerProfileDTO.imageIdentifier`'s Java value (uuid, no hyphens — same value P12.2e computes) for a Java profile, or the honest-unavailable state below for Bedrock.
2. Java: fetch `https://mc-heads.net/avatar/{identifier}/128` using the same `ureq::Agent` pattern already established in `crates/msc-infrastructure/src/addon_provider.rs:147-168` (reuse that config approach, don't invent a different HTTP client setup). On success, base64-encode the raw response bytes as-is and return `imageMimeType: "image/png"`, `source: "lookup_override"` or `"profile_lookup"` depending on whether step 1 used an override. On fetch failure or non-200, return `500 internal_error` (this route's contract has no other applicable code for a transient upstream failure — `404 profile_not_found` is reserved for `profileId` not matching any known profile, checked before this fetch even happens).
3. Bedrock: the Xbox-lookup dependency this needs was already deferred by Phase 11's scope doc and is still undone — degrade the same honest way P12.1a's sidebar avatar does for Bedrock (a real, truthful "not available yet" response, `success: false`, not a fabricated image).

`POST /v1/players/skin-override`: set or clear (empty/absent `lookupIdentifier` → clear) the override's `lookupIdentifier` for `profileId` in `player_overrides.json`, leaving `skinFileName` untouched (always `None` per the scope cut above).
**Verify:** `cargo fmt --check && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --bin msc player_skin`
**Commit:** `P12.2f: wire player skin resolution and override`
**Batch:** solo

### P12.2g — Amend the API contract: 4 player-data mutation routes
**Status:** DONE
**Files:** `docs/msc2/api-contract/openapi.json`, `tools/api-contract-check.py`
**What:** Of MSC 1's 5 player-data mutation actions in `AppViewModel+PlayerProfiles.swift` (lines 475–516) and `PlayerDataManager.swift`, Cameron approved 4 for MSC 2 (2026-08-25 decision, superseding the earlier "delete only" plan): **delete, migrate-to-offline-UUID, migrate-to-custom-UUID, duplicate**. `copyPlayerData` (copy one player's data onto another's, overwriting) stays deferred, not dropped. None of these 4 exist in the frozen contract today — this step is a pure contract amendment, no Rust code, following the exact pattern P6.8/P6.34/P7.9/P8.9 each used for their own additions (see `tools/api-contract-check.py`'s `EXPECTED_TOTAL` comment for the running list). Do this step precisely as specified below — every field name, type, and error code is fixed so P12.2h–j have nothing left to invent.

Add 4 new paths under `components/paths`, each `x-permission-category: "players"` (already used by `/v1/players/hidden` and `/v1/players/skin-override` — reuse that category, do not invent a new one) and each returning the same new `PlayerMutationResultDTO` on 200:

1. `POST /v1/players/delete` — deletes a player's `.dat` file permanently.
   - Request: `PlayerDeleteRequestDTO { profileId: string (required) }`
2. `POST /v1/players/migrate-offline` — copies a player's data to the UUID Minecraft computes for that player's username in offline mode.
   - Request: `PlayerDeleteRequestDTO` (reuse — same single `profileId` field, no new schema needed)
3. `POST /v1/players/migrate` — copies a player's data to an arbitrary caller-supplied UUID.
   - Request: `PlayerMigrateRequestDTO { profileId: string (required), targetUuid: string (required) }`
4. `POST /v1/players/duplicate` — copies a player's data under a fresh random UUID.
   - Request: `PlayerDeleteRequestDTO` (reuse)

New response schema, shared by all 4:
```json
"PlayerMutationResultDTO": {
  "type": "object",
  "properties": {
    "success": { "type": "boolean" },
    "message": { "type": "string" },
    "newProfileId": { "type": "string", "nullable": true, "description": "Set only by duplicate/migrate routes: the UUID the data now also lives under." },
    "profiles": { "$ref": "#/components/schemas/PlayerProfilesResponseDTO" }
  },
  "required": ["success", "message", "profiles"]
}
```
(`profiles` is always the freshly re-scanned full list, same envelope-refresh pattern `WorldMutationResultDTO.updated` already uses — this lets P12.3's client re-render from the response instead of firing a second request.)

New request schemas:
```json
"PlayerDeleteRequestDTO": {
  "type": "object",
  "properties": { "profileId": { "type": "string" } },
  "required": ["profileId"]
}
"PlayerMigrateRequestDTO": {
  "type": "object",
  "properties": {
    "profileId": { "type": "string" },
    "targetUuid": { "type": "string" }
  },
  "required": ["profileId", "targetUuid"]
}
```

Error responses — identical shape on all 4 routes, matching every other `/v1/players/*` mutation route already in the contract (`ErrorDTO`, `x-error-code` per status):
- `400 invalid_body` — missing/empty `profileId`; for `/migrate`, also a `targetUuid` that fails to parse as a UUID (`invalid_uuid`)
- `404 not_found` — `profile_not_found`: no `.dat` file exists for `profileId`
- `409 conflict` — `no_active_server`; `not_bedrock` on all 4 routes when the active server is Bedrock (these 4 actions are Java-only — MSC 1's `PlayerProfileDetailSheet.swift` shows only identify/hide for Bedrock profiles, never these; its `bedrockActionsNote` states Bedrock data can't be edited from the app at all, since it lives in LevelDB); and, **only on `/migrate-offline`**, `username_unknown` — ported from `ProfileError.usernameUnknown` (`AppViewModel+PlayerProfiles.swift:476`): this profile's username hasn't resolved yet (Bedrock/unresolved-Mojang profiles have no username to hash), so there's no offline UUID to compute
- `500 internal_error` — filesystem failure mid-copy/delete

Bump `EXPECTED_TOTAL` in `tools/api-contract-check.py` by 4 and append one clause to its running-total comment, same style as every prior entry there.
**Verify:** `python3 tools/api-contract-check.py`
**Commit:** `P12.2g: amend the API contract with 4 player-data mutation routes`
**Batch:** solo

### P12.2h — Port the offline-UUID algorithm
**Status:** DONE
**Files:** `crates/msc-domain/src/player_nbt.rs` (add to the module P12.2c created — this is a small pure function, not a new file), `crates/msc-domain/tests/player_nbt.rs`
**What:** Port `PlayerDataManager.offlineUUID(for:)` (`PlayerDataManager.swift:118-129`) exactly. This is Java's `UUID.nameUUIDFromBytes`, a fixed public algorithm — not project-specific behavior, so no fixture-extraction step is needed; pin it with known-answer test vectors instead (computed independently below, not merely asserted — reproducible with any MD5 implementation):

  1. MD5 hash of the UTF-8 bytes of `"OfflinePlayer:{username}"`.
  2. On the 16-byte digest, set byte 6 to `(byte[6] & 0x0F) | 0x30` (UUID version 3).
  3. Set byte 8 to `(byte[8] & 0x3F) | 0x80` (RFC 4122 variant).
  4. Format the 16 bytes as a standard `8-4-4-4-12` lowercase UUID string.

  Signature: `pub fn offline_uuid(username: &str) -> Uuid` (use the `uuid` crate's `Uuid` type already a dependency elsewhere in the workspace; MD5 via the `md-5` crate — check `Cargo.lock` first in case something already vendors it before adding a new dependency).

  Pinned test vectors (assert exact equality, all lowercase, hyphenated):
  | username | offline UUID |
  |---|---|
  | `Notch` | `b50ad385-829d-3141-a216-7e7d7539ba7f` |
  | `Bob` | `faa5dca3-c3d4-354b-ae1b-dde9e5a14b3b` |
  | `jeb_` | `a762f560-4fce-3236-812a-b80efff0b62b` |
  | `Dinnerbone` | `4d258a81-2358-3084-8166-05b9faccad80` |
  | `` (empty string — must not panic; MSC 1's caller guards against this at the profile level, but the function itself must stay total) | `fc5bc365-aedf-30a8-8b89-04e462e29bde` |

**Verify:** `cargo fmt --check && cargo clippy -p msc-domain --all-targets -- -D warnings && cargo nextest run -p msc-domain --test player_nbt offline_uuid` (same file as P12.2c added — scope with `--test player_nbt`, not a bare package-wide filter, for the same reason noted there)
**Commit:** `P12.2h: port the offline-UUID algorithm`
**Batch:** solo

### P12.2i — Port the player-data file mutation primitives
**Status:** DONE
**Files:** `crates/msc-application/src/player_profiles.rs` (the module P12.2d created), `crates/msc-application/tests/player_profiles.rs`
**What:** Port `PlayerDataManager`'s file-operation primitives (`PlayerDataManager.swift:131-164`) and the 4 approved `AppViewModel+PlayerProfiles.swift` action wrappers (lines 471-516; skip `copyPlayerData` at line 493 — deferred) into `player_profiles.rs`. These operate against a **single** resolved directory, unlike P12.2d's scan (which reads both candidate directories) — add the single-directory resolver first:

  - `resolve_player_data_dir(fs, server_dir, level_name) -> PathBuf` — ports `PlayerDataManager.playerDataDir` (`PlayerDataManager.swift:29-34`): checks `{server_dir}/{level_name}/playerdata/` then `{server_dir}/{level_name}/players/data/` via `fs.list(...).is_ok()`, returns the first that exists; if neither exists, returns the `playerdata/` path anyway (a non-existent path — matches Swift's `?? candidates[0]` fallback, callers then get a natural `NotFound` on the actual file op rather than a special-cased error here).
  - `dat_path(uuid, player_data_dir) -> PathBuf` — `{player_data_dir}/{uuid lowercased}.dat`.
  - `copy_player_data(source_uuid, dest_uuid, player_data_dir, fs) -> Result<(), PlayerProfileError>` — **note this differs from the literal Swift mechanics**: Swift's `FileManager.copyItem` requires removing an existing destination first (`PlayerDataManager.swift:135-143`), but Rust's injectable `FileSystem` trait (`crates/msc-infrastructure/src/fs.rs`) has no `copy`, only `read`/`write`/`remove`, and `write` already overwrites unconditionally — so port this as `fs.write(&dat_path(dest_uuid, dir), &fs.read(&dat_path(source_uuid, dir))?)`, no separate remove call. Same one-line "copy via read+write, since the trait has no native copy" pattern already established by `copy_via_fs` in `crates/msc-application/src/worlds.rs:540` — read that function for the pattern, don't call it directly (it's private to that module).
  - `delete_player_data(uuid, player_data_dir, fs) -> Result<(), PlayerProfileError>` — `fs.remove(&dat_path(uuid, dir))`.
  - `duplicate_player_data(uuid, player_data_dir, fs) -> Result<Uuid, PlayerProfileError>` — generates a fresh random `Uuid::new_v4()`, calls `copy_player_data(uuid, new_uuid, dir, fs)`, returns `new_uuid`.
  - `migrate_to_offline_uuid(profile, player_data_dir, fs) -> Result<Uuid, PlayerProfileError>` — requires `profile.username` to be `Some` and non-empty, else returns `PlayerProfileError::UsernameUnknown` (ports `ProfileError.usernameUnknown`, `AppViewModel+PlayerProfiles.swift:476`); computes the target via P12.2h's `offline_uuid`, calls `copy_player_data`, returns the target UUID.
  - `migrate_to_uuid(profile, target_uuid, player_data_dir, fs) -> Result<(), PlayerProfileError>` — no username requirement; straight `copy_player_data(profile.uuid, target_uuid, dir, fs)`.

  Error mapping, all 5 functions: any `io::Error` from `fs.read`/`fs.remove` whose `.kind() == std::io::ErrorKind::NotFound` becomes `PlayerProfileError::ProfileNotFound`; every other `io::Error` becomes `PlayerProfileError::Io`. In practice P12.2j always resolves and confirms `profileId` via a fresh scan before calling any of these, so this mapping is a defensive backstop (e.g. a file removed between the scan and the mutation), not the primary source of a route's `404` — but keep it precise regardless, since these functions must stay independently correct and independently testable, not rely on a caller having already checked existence. Reuse the `PlayerProfileError` enum P12.2d already defined — do not create a second error type in this module.
**Verify:** `cargo fmt --check && cargo clippy -p msc-application --all-targets -- -D warnings && cargo nextest run -p msc-application --test player_profiles` (NOT a bare `-p msc-application player_profiles` filter — `msc-application` has 60 separate integration-test binaries and a package-wide filter compiles all of them first; `--test player_profiles` builds only this one)
**Commit:** `P12.2i: port the player-data file mutation primitives`
**Batch:** solo

### P12.2j — Wire the 4 player-data mutation routes
**Status:** DONE
**Files:** `crates/msc-agent/src/routes/players.rs` (the file P12.2e created), `crates/msc-agent/src/main.rs`
**What:** Add the 4 handlers for P12.2g's routes (`delete`, `migrate-offline`, `migrate`, `duplicate`). Per handler, in this order:
1. Parse the request DTO — `400 invalid_body` on missing/empty `profileId`, or (on `/migrate`) a `targetUuid` that doesn't parse as a UUID (`invalid_uuid`).
2. `409 not_bedrock` if the active server is Bedrock (per P12.2g's contract) — this is a genuine `409`, unlike `routes/bedrock.rs`'s read-only `GET /v1/players`, which answers a wrong-platform request with `200` + `note: "not_bedrock"` since a read has a valid empty answer and a mutation does not.
3. Run P12.2d's Java profile scan (same call `GET /v1/players/profiles`'s handler already makes) and find the profile whose `uuid` matches `profileId`. Not found → `404 profile_not_found`. **This scan-and-find is the primary source of the route's 404** — P12.2i's own `ProfileNotFound` mapping only fires on the rarer case of the file vanishing between this scan and the mutation call in step 4, and should map to the same `404 profile_not_found` if it does.
4. Call `resolve_player_data_dir` (P12.2i) for the directory, then the matching P12.2i function (`delete_player_data`, `migrate_to_offline_uuid`, `migrate_to_uuid`, or `duplicate_player_data`), passing the real `FileSystem` impl. Map any resulting `PlayerProfileError` to its status/`x-error-code`: `ProfileNotFound`→404, `UsernameUnknown`→409 (`/migrate-offline` only), `Io`→500.
5. On success, re-run the same P12.2d scan once more (the mutation changed disk state, so the pre-mutation scan from step 3 is now stale) to build the `profiles` field of `PlayerMutationResultDTO`; set `newProfileId` for `/migrate-offline`, `/migrate`, and `/duplicate` (the target/new UUID), leave it absent for `/delete`.

Register all 4 routes in `main.rs` next to the existing `/v1/players/*` routes.
**Verify:** `cargo fmt --check && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --bin msc players` (NOT a bare `-p msc-agent players` filter — `msc-agent` has 36 separate integration-test binaries and no lib target, so a package-wide filter compiles all of them first; `--bin msc` builds only the agent binary these tests actually live in)
**Commit:** `P12.2j: wire the 4 player-data mutation routes`
**Batch:** solo

### P12.3 — Players tab
**Status:** awaiting verification. Rebuilt against the now-real P12.2b–j backend: `OnlineNowCard` (Online Now / Seen This Session), `BedrockAllowlistCard` (Bedrock-only), `SessionLogCard` (filter/day-grouping/show-more/clear), `PlayerDataCard` (search/sort/hidden-toggle grid) → `PlayerDetailSheet` (identity, skin lookup override, stats, inventory via a new `InventoryGrid`, and the 4 approved mutations — migrate-offline, migrate-custom, duplicate, delete — plus hide/unhide). New `crates`-adjacent client files only; no backend touched. Regenerated `src/lib/api/generated.ts` from the current contract (P12.2g's routes weren't in it yet). Real gap found and handled the same way as this file's other honest gaps rather than built around quietly: `GET /v1/session-log` is *also* frozen-but-unimplemented (zero handler anywhere in `crates/`) — Session Log is derived client-side from `/v1/console/tail` by reusing `chatFeed.ts`'s existing join/leave parsing (already built for Overview's Chat card) rather than adding a second backend detour; "Clear Log" is client-local per host+server (localStorage cutoff timestamp), same "no server field yet" treatment `notes.ts` already established, since the console tail is a bounded recent buffer with nothing durable on the agent to send a clear mutation to. **Follow-up planned, not just noted:** P12.3a–c (below) give this a real backend — MSC 1's own `SessionLogManager.swift`/`session_log.json` ported directly, plus wiring the Java join/leave events `lifecycle.rs` already detects and currently discards. A later client step (not yet written) swaps this section over to it. `copyPlayerData` stays deferred per this file's 2026-08-25 note — only 4 of 5 mutation actions are wired, matching P12.2g–j. Two design decisions confirmed with Cameron mid-build rather than improvised per his own added S0-extension note: kept 2 new `Icon.svelte` glyphs (`clock`, `id-card`, same 24x24/stroke-1.8 language as the existing set); kept an inline expand-in-place delete confirmation (no modal) over migrating the still-Phase-11-styled `ConfirmDialog.svelte`. Anti-slop self-review caught and removed two accent colors I'd introduced outside the locked status ramp (a gold Operator badge, a blue enchanted-item glow) — fixed to neutral before commit. **Not verified: live browser check against real agent data.** Got the dev server running and the shell rendering (past the first-launch splash) via Playwright, but reaching Players requires a paired agent + registered server, and touching Cameron's real local agent/server registration to manufacture that felt like the wrong call to make unprompted — leaving that verification to Cameron, who already has real servers (`test`, `campak`, `test_2`) to check it against directly, which is a better test than anything synthetic here anyway. Structural (`npm run test:screen-players-online`, 11 tests, rewritten — the old suite asserted `/v1/players/profiles` must *not* be used, which was P11.11's deliberate scope limit and is now obsolete) and `svelte-check`/`prettier --check` both pass on every touched file with zero new errors.
**Files:** `src/lib/sections/players-online/`, `src/lib/components/base/Icon.svelte`, `src/lib/api/generated.ts`
**What:** Rebuild Players — Online Now / Seen This Session, Session Log (filter + clear), Player Data (profiles, sort, stats + inventory detail sheet, hidden toggle, skin/avatar, delete). Reference MSC 1 `DetailsPlayersTabView` + `PlayerProfilesCard`/`PlayerProfileDetailSheet`/`PlayerInventoryView` and the Players screenshot.
**Verify:** `npm run dev`, open Players against a real connected agent; compare to MSC 1 + checklist. Structural: `npm run test:screen-players-online`.
**Commit:** `P12.3: rebuild the Players tab`
**Batch:** solo

### P12.3a — Amend the contract: add POST /v1/session-log/clear
**Status:** not started
**Files:** `docs/msc2/api-contract/openapi.json`, `tools/api-contract-check.py`
**What:** `GET /v1/session-log` → `SessionLogResponseDTO` is already frozen (from an earlier phase) but has no handler anywhere in `crates/` — P12.3b/c implement it. This step covers the one piece that's missing from the contract entirely: MSC 1's Clear Log button (`SessionLogManager.clearEvents`) has nothing to call. Add:

`POST /v1/session-log/clear` — `x-permission-category: "players"` (a real mutation, unlike GET's own `"none"` read-only category; matches the category already used for `/v1/players/*`). No request body. Returns the same `SessionLogResponseDTO` schema GET already uses, with `events: []` — same "always return fresh state" pattern P12.2g's `PlayerMutationResultDTO.profiles` already established. Error responses: `409 conflict` / `no_active_server` (`ErrorDTO`, matching every other active-server-scoped mutation's shape); no `404`/`400` case exists since there's no target id to miss or body to malform.

Bump `EXPECTED_TOTAL` in `tools/api-contract-check.py` by 1, append a clause to its running-total comment in the same style as every prior entry there.
**Verify:** `python3 tools/api-contract-check.py`
**Commit:** `P12.3a: amend the contract with POST /v1/session-log/clear`
**Batch:** solo

### P12.3b — Port the session log store and wire the live Java hook
**Status:** not started
**Files:** `crates/msc-application/src/session_log.rs` (new), `crates/msc-application/src/lib.rs`, `crates/msc-application/src/lifecycle.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-application/tests/session_log.rs` (new)
**What:** Port `SessionLogManager.swift` (`{serverDir}/session_log.json` — read-all/append-one/clear, `.atomic` write) into `session_log.rs`. Pin the JSON shape exactly to `SessionEvent.swift`'s `Codable` output, since nothing needs to change about it — it's plain, already correct, and any client eventually reading it directly should see the same shape MSC 1 always wrote:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    pub id: Uuid,
    pub player_name: String,
    pub event_type: SessionEventType,
    pub timestamp: String,  // ISO8601, caller-supplied (see below) -- this module does not read the clock itself
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionEventType { Joined, Left }  // serializes to "joined"/"left", matching Swift's raw values exactly

pub fn load_events(fs: &dyn FileSystem, server_dir: &Path) -> Vec<SessionEvent> { /* missing/malformed file -> empty, matches loadEvents' silent-failure behavior */ }
pub fn append_event(fs: &dyn FileSystem, server_dir: &Path, player_name: &str, event_type: SessionEventType, timestamp: String) -> Result<Vec<SessionEvent>, SessionLogError> { /* load, push, atomic_write, return the full updated list -- matches appendEvent's inout-list contract */ }
pub fn clear_events(fs: &dyn FileSystem, server_dir: &Path) -> Result<(), SessionLogError> { /* fs.remove; NotFound is not an error, matches clearEvents' fileExists-then-removeItem guard */ }
```

`timestamp` is caller-supplied rather than read from the clock inside this module (unlike Swift's `SessionEvent.init`'s `Date()` default) so the P12.3b wiring below can pass the exact same `iso8601_now()` value it already computes once per line, and so `tests/session_log.rs` can assert exact timestamps without a clock-mocking seam.

**Wire the live Java hook** — the only real "new behavior" in this step. `crates/msc-application/src/lifecycle.rs:405-416`'s `ingest_console_line` already emits `OutputEvent::PlayerJoined(String)`/`PlayerLeft(String)` for every join/leave line (`output_reducer.rs`) but its match arm at line 414 currently discards both (`=> {}`). Two options for where the disk write happens — pick whichever keeps `lifecycle.rs`'s existing I/O boundary honest (check whether `self.console.append_system_line` at line 381 already means this struct does its own I/O, in which case call `session_log::append_event` right there in `lifecycle.rs`; if that struct is meant to stay pure and I/O happens only at the caller, the append instead belongs in `crates/msc-agent/src/routes/lifecycle.rs`'s `drain_process_events` (around line 1250-1261, which already inspects this same `ingest_console_line` return value for `OutputEvent::Ready`) — resolve which by reading `lifecycle.rs` in full first, don't guess from this excerpt alone. Either way: on `PlayerJoined(name)` write `SessionEventType::Joined`, on `PlayerLeft(name)` write `Left`, using the active server's `server_dir` and the same `iso8601_now()` already computed for that line. A write failure must not interrupt lifecycle processing — log and continue, matching Swift's `catch` in `recordSessionEvent` (`logAppMessage`, no propagation).

**Bedrock is explicitly out of scope for this step, not silently dropped:** `bedrock_service.rs` already defines an equivalent `BedrockServiceEvent::PlayerJoined`/`PlayerLeft(BedrockPlayer)`, but nothing in `crates/msc-agent` currently drains `BedrockServiceEvent` at all — wiring Bedrock session-log support needs that live-drain infrastructure built first, which is separate, larger scope than this step. This is not a regression: today's client-side console-tail derivation (P12.3's `sessionEventsFromConsole`) already covers Bedrock via `chatFeed.ts`'s "Player connected/disconnected" parsing, and keeps doing so until a client step swaps it over to this new backend — which that future step must handle explicitly (e.g. keep the console-tail path for Bedrock, real backend for Java) rather than silently regressing Bedrock coverage.
**Verify:** `cargo fmt --check && cargo clippy -p msc-application --all-targets -- -D warnings && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-application --test session_log` (NOT a bare `-p msc-application session_log` filter — see this file's 2026-08-25 nextest-scoping note; `msc-application` has 60+ separate integration-test binaries)
**Commit:** `P12.3b: port the session log store and wire the live Java hook`
**Batch:** solo

### P12.3c — Wire GET /v1/session-log and POST /v1/session-log/clear
**Status:** not started
**Files:** `crates/msc-agent/src/routes/session_log.rs` (new), `crates/msc-agent/src/routes/mod.rs`, `crates/msc-agent/src/main.rs`
**What:** Implement both routes against P12.3b's `session_log` module. `GET /v1/session-log`: `session_log::load_events` for the active server, mapped to `SessionEventDTO { id, playerName: player_name, eventType: event_type as string, timestamp }`, wrapped in `SessionLogResponseDTO { activeServerId: Some(id), events }`; no active server → `events: []`, `activeServerId: None` (matches this route's existing no-error 200-empty convention, same as `GET /v1/players` today for a not-yet-selected server — do not 409 here). `POST /v1/session-log/clear`: `409 conflict`/`no_active_server` if none active, else `session_log::clear_events` then return the same shape with `events: []`.

Tests for these two routes belong inline (`#[cfg(test)] mod tests`) in the new `routes/session_log.rs` itself, matching every other route file's convention (settings.rs, players.rs, etc.) — `msc-agent` has no lib target, only its `msc` bin, so `--bin msc <filter>` is the only way to reach them.
**Verify:** `cargo fmt --check && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --bin msc session_log`
**Commit:** `P12.3c: wire GET /v1/session-log and POST /v1/session-log/clear`
**Batch:** solo

### P12.3d — Add and wire POST /v1/players/identify, fix Bedrock skin resolution, split Head/Body preview
**Status:** awaiting verification. Cameron shared real MSC 1 screenshots (Bedrock player "camkage" on `campak`) mid-review of P12.3, surfacing two more real gaps beyond session log, same shape as before: (1) MSC 1's "Identify Player"/"Change Gamertag" action (`identifyBedrockPlayer` → `BedrockNameCache.record`) had no route at all — but the application-layer function already existed (`bedrock_players::record_name`, presumably from earlier Bedrock work), just never wired to HTTP; (2) the Skin Override section shows Head *and* Body thumbnails, not the single portrait P12.3 shipped with — and while investigating, found the client's own `avatarUrl` unnecessarily hard-excluded Bedrock entirely, when `bedrock_profile_to_dto` (P12.2e) already computes a working dotted-gamertag `imageIdentifier` for Bedrock, matching MSC 1's own `PlayerProfile.imageIdentifier` scheme exactly (confirmed working in Cameron's screenshot). Both approved by Cameron before building, per the same design-decision-check discipline as everything else in this phase.

Contract: `POST /v1/players/identify` (`x-permission-category: "players"`) — `PlayerIdentifyRequestDTO { profileId, gamertag }` → `PlayerIdentifyResultDTO { success, message, profileId?, username? }`. `409 not_bedrock` if `profileId` isn't a Bedrock id (`xuid_` prefix) — this route only makes sense for Bedrock, since Java usernames already resolve from `usercache.json`. `EXPECTED_TOTAL` 125→126. Inserted as a surgical text edit next to `/v1/players/hidden`/`PlayerMigrateRequestDTO` rather than a full JSON parse-and-rewrite — the latter was tried first and produced a 20,000-line diff by silently reformatting unrelated pre-existing compact JSON blocks elsewhere in the file; reverted.

Backend: `mutate_identify` in `crates/msc-agent/src/routes/players.rs`, mirroring `mutate_hidden`'s exact shape (permission check → active-server check → body parse → profile-exists check → `xuid_` prefix dispatch, except this route requires the prefix rather than branching on it) → calls the pre-existing `bedrock_players::record_name`.

Client: fixed `avatarUrl`/`model.ts` to stop excluding Bedrock (now always returns a real mc-heads.net URL from `imageIdentifier`, matching the backend's already-correct computation); added `bodyUrl` alongside it. `PlayerDetailSheet.svelte`: identity header portrait now prefers the backend-resolved `skin.imageBase64` (Java only, P12.2f) then falls back to a client-side `bodyUrl` render (both editions) with an `onerror`-driven initial-letter degrade, rather than a hard Bedrock exclusion; Skin Override section (renamed from "Skin Lookup", now shown for both editions — `mutate_skin_override` was never actually Java-gated on the backend, only in the client) gained a Head+Body thumbnail pair reflecting the current override-or-own identifier; Data Management gained the Identify/Change Gamertag action for Bedrock (mirrors the Java migrate/duplicate/delete block's inline-expand pattern already established) and folded Hide/Unhide into the same section for both editions, matching MSC 1's actual `actionsSection` structure instead of the separate standalone section P12.3 shipped with.

**A third, larger gap found but deliberately not built here, flagged for Cameron to decide on separately:** MSC 1's real screenshot shows a Bedrock player's Stats and Inventory sections fully populated (Health/Food/XP/Mode/Position, a real inventory grid) — but `BedrockPlayerRecord` (P12.2e) only ever carried `has_stats: bool`/`inventory_items: usize` (counts, not the parsed data), so `bedrock_profile_to_dto` always emits `stats: None`, `inventory: []`. The underlying NBT *is* already parsed in memory at scan time (`bedrock_players::discover_players` calls `read_player_nbt` and immediately discards everything but a bool and a count) — so this is likely a smaller fix than it sounds (carry the real `PlayerStats`/`Vec<InventoryItem>` through `BedrockPlayerRecord` instead of summarizing them away, then map to the same DTOs `player_stats_to_dto`/`inventory_item_to_dto` already build for Java) — but it's still new scope, not something to fold into this step silently.
**Files:** `docs/msc2/api-contract/openapi.json`, `tools/api-contract-check.py`, `crates/msc-agent/src/routes/players.rs`, `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/sections/players-online/{model.ts,PlayerDataCard.svelte,PlayerDetailSheet.svelte}`, `clients/desktop-web/tests/screens/players-online.test.ts`
**What:** Add the missing Bedrock-identify route backed by the already-existing `record_name` function; fix the client to actually use the backend's already-correct Bedrock avatar identifier; add the Head/Body preview pair MSC 1 actually has.
**Verify:** `python3 tools/api-contract-check.py && cargo fmt --check && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --bin msc players`; client: `npx svelte-check --tsconfig ./tsconfig.json && npm run format:check && npm run test:screen-players-online`.
**Commit:** `P12.3d: add POST /v1/players/identify, fix Bedrock skin resolution, split Head/Body preview`
**Batch:** solo

### P12.3e — Carry real Bedrock stats and inventory through instead of discarding them
**Status:** not started
**Files:** `crates/msc-application/src/bedrock_players.rs`, `crates/msc-agent/src/routes/players.rs`
**What:** Cameron approved planning this after P12.3d's investigation found it's smaller than it looks: `crates/msc-infrastructure/src/bedrock_nbt.rs`'s `read_player_nbt` already parses full Bedrock stats and inventory from LevelDB — `bedrock_players::discover_players` (`crates/msc-application/src/bedrock_players.rs:212-224`) calls it, then immediately throws the result away, keeping only `nbt.stats.is_some()` and `nbt.inventory.len()`. No new parsing, no new fixtures, no contract change — `PlayerProfileDTO.stats`/`inventory` are already wired and already populate correctly for Java; this step only stops Bedrock from being short-changed at the one point it's discarded. Nothing in `msc-infrastructure/src/bedrock_nbt.rs` changes — leave that module, and its own P10 fixtures/tests, completely untouched.

**Widen `BedrockPlayerRecord`** (`bedrock_players.rs:28-33`): replace `has_stats: bool` / `inventory_items: usize` with the real data:
```rust
pub struct BedrockPlayerRecord {
    pub xuid: String,
    pub name: String,
    pub stats: Option<msc_infrastructure::bedrock_nbt::PlayerStats>,
    pub inventory: Vec<msc_infrastructure::bedrock_nbt::InventoryItem>,
}
```
In `discover_players` (line ~219), change `has_stats: nbt.stats.is_some(), inventory_items: nbt.inventory.len()` to `stats: nbt.stats, inventory: nbt.inventory` — the `nbt` binding already holds the parsed value right there; this is a one-line swap, not new logic. The existing `if nbt.stats.is_none() && nbt.inventory.is_empty() { continue; }` skip-guard immediately above stays exactly as-is (still the right filter: skip records with no real data).

**Convert in `bedrock_profile_to_dto`** (`players.rs:919-942`) rather than duplicating display-string logic: `msc_infrastructure::bedrock_nbt::PlayerStats`/`InventoryItem`/`ItemEnchantment` are field-for-field equivalent to Java's `msc_domain::player_nbt::PlayerStats`/`InventoryItem`/`ItemEnchantment` (same meanings, same units) with exactly one shape difference — Bedrock's `position: [f64; 3]` versus Java's separate `pos_x`/`pos_y`/`pos_z` fields. Write a small conversion (a `From` impl or a private function in `players.rs`) from the Bedrock structs to the Java ones, unpacking `position[0]/[1]/[2]` into `pos_x`/`pos_y`/`pos_z`, then call the **exact same** `player_stats_to_dto`/`inventory_item_to_dto`/`enchantment_to_dto` functions (`players.rs:944` onward) Java already uses — including their `game_mode_display()`/`dimension_display()` calls, so Bedrock's dimension/game-mode strings come out identically formatted to Java's for free. Do not write a second set of display-string logic for Bedrock's copy of these fields.

**Update the one test this breaks:** `bedrock_profile_mapping_uses_xuid_identity_and_always_empty_inventory` (`players.rs:1117-1133`) currently pins the old discard-everything behavior by name and by assertion (`assert_eq!(json["inventory"], serde_json::json!([]))`, `assert!(json.get("stats").is_none())`) — construct its `BedrockPlayerRecord` fixture with a real `stats`/`inventory` value instead of `has_stats: true, inventory_items: 4`, rename the test to reflect the new behavior, and assert the DTO actually carries the values through (mirroring `java_profile_mapping_preserves_contract_field_names_and_derived_values`'s assertion style one test up). Grepped confirmed `has_stats`/`inventory_items` appear nowhere else in the workspace — this is the only other place to touch.
**Verify:** `cargo fmt --check && cargo clippy -p msc-application --all-targets -- -D warnings && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --bin msc players`
**Commit:** `P12.3e: carry real Bedrock stats and inventory through instead of discarding them`
**Batch:** solo

### P12.4 — Worlds tab (+ world wizards)
**Status:** not started
**Files:** `src/lib/sections/worlds/`
**What:** Rebuild Worlds — World Slots (cards w/ thumbnail, Active badge, Activate/edit/delete, Save Current/Create New) and Backups. Include the world wizards (Rename/Replace/Convert/Repair) as sheets. Reference MSC 1 `DetailsWorldsTabView`, `WorldSlotsView`, the world wizard views, and the Worlds screenshot.
**Verify:** `npm run dev`, open Worlds and each wizard; compare to MSC 1 + checklist. Structural: `npm run test:screen-worlds-backups`.
**Commit:** `P12.4: rebuild the Worlds tab`
**Batch:** solo

### P12.5 — Packs tab
**Status:** not started
**Files:** `src/lib/sections/` (packs section)
**What:** Rebuild Packs — Resource Packs list, Add Pack / Clear Active Pack, empty state, drag-drop. Reference MSC 1 `DetailsPacksTabView`, `ResourcePacksView`, and the Packs screenshot.
**Verify:** `npm run dev`, open Packs; compare to MSC 1 + checklist (empty + populated).
**Commit:** `P12.5: rebuild the Packs tab`
**Batch:** solo

### P12.6 — Performance tab
**Status:** not started
**Files:** `src/lib/sections/performance/`
**What:** Rebuild Performance — TPS 1m/5m/15m + Players/CPU/Memory cards, TPS-Over-Time and Player-Activity charts, right-hand Monitoring/Quick Actions/Health Summary rail, World Size/Uptime/Status footer. Charts follow the `dataviz` discipline and the anti-slop color budget. Reference MSC 1 `DetailsPerformanceTabView`/`Content` and the Performance screenshot.
**Verify:** `npm run dev`, open Performance running; compare to MSC 1 + checklist.
**Commit:** `P12.6: rebuild the Performance tab`
**Batch:** solo

### P12.7 — Components tab (+ plugin browser)
**Status:** not started
**Files:** `src/lib/sections/components/`, `src/lib/sections/addons/`
**What:** Rebuild Components — Server JAR row (version, Up to date/update), Plugins (list, Add Plugin, Reveal folder, empty state), Crossplay (Broadcast, Missing/status). Include the plugin browser (Modrinth/CurseForge) as a sheet. Reference MSC 1 `DetailsComponentsTabView`, `ModrinthBrowserView`, `CurseForgeManualDownloadSheet`, and the Components screenshot.
**Verify:** `npm run dev`, open Components + browser; compare to MSC 1 + checklist. Structural: `npm run test:screen-addons`.
**Commit:** `P12.7: rebuild the Components tab`
**Batch:** solo

### P12.8 — Settings tab
**Status:** not started
**Files:** `src/lib/sections/settings/`
**What:** Rebuild Settings — World Settings (difficulty/gamemode segmented, toggles, world type, spawn protection) and Server Settings (MOTD, Max Players, distances, whitelist…), the "Unsaved changes / stays local until Save Changes" model with a Save primary. Reference MSC 1 `DetailsSettingsTabView`, `ServerSettingsView`, and the Settings screenshot.
**Verify:** `npm run dev`, open Settings, edit a field, confirm the unsaved-changes/Save flow; compare to MSC 1 + checklist.
**Commit:** `P12.8: rebuild the Settings tab`
**Batch:** solo

### P12.9 — Files tab
**Status:** not started
**Files:** `src/lib/sections/` (files section)
**What:** Rebuild Files — Server Root breadcrumb, Folders + Files divided lists, Show in Finder (web fallback marked), file preview/edit. Reference MSC 1 `ServerFilesTabView` and the Files screenshot.
**Verify:** `npm run dev`, open Files, browse + preview; compare to MSC 1 + checklist.
**Commit:** `P12.9: rebuild the Files tab`
**Batch:** solo

### P12.10 — Console (docked, full behavior)
**Status:** not started
**Files:** `src/lib/sections/console/`
**What:** Complete the docked console — filter chips (All/Server/Plugins/Warnings/Controller/Commands/Custom), search, buffered log, command input + Send, collapse/expand, copy/clear. Reference MSC 1 `ConsoleView` and the console in `~/Documents/MSCSS/Main View`.
**Verify:** `npm run dev`, run a server, exercise filters/search/command; compare to MSC 1 + checklist.
**Commit:** `P12.10: rebuild the docked console`
**Batch:** solo

### P12.11 — Manage Servers / Hosts sheet (+ multi-host)
**Status:** not started
**Files:** `src/lib/sections/fleet/`, `src/lib/sections/connectivity/`, sheet components
**What:** Rebuild the Manage sheet as the multi-host home — servers grouped by host, add/rename/delete servers, add/connect/manage hosts (folding the old fleet/connectivity/agent-setup content in here, not as rival top-level screens). Reference MSC 1 `ManageServersView` + `~/Documents/MSCSS/Manage Servers`, adapted for D-013 multi-host.
**Verify:** `npm run dev`, open Manage, exercise host + server management; compare to MSC 1 (adapted) + checklist. Structural: `npm run test:screen-fleet`.
**Commit:** `P12.11: rebuild manage servers and hosts`
**Batch:** solo

### P12.12 — Server Editor sheet (7 sub-tabs)
**Status:** not started
**Files:** server-editor sheet components
**What:** Rebuild the Server Editor sheet with its sub-tabs — General, Settings, Jars, World, Backups, Broadcast, and Docker (Docker marked per D-008 exclusion; drop or adapt). Reference MSC 1 `ServerEditorView` + `ServerEditor*Tab` files.
**Verify:** `npm run dev`, open the editor, walk each sub-tab; compare to MSC 1 + checklist.
**Commit:** `P12.12: rebuild the server editor`
**Batch:** solo

### P12.13 — First-time setup / Prerequisites / Setup wizard
**Status:** not started
**Files:** `src/lib/sections/setup/`, `src/lib/help/` (first-launch)
**What:** Rebuild the first-launch flow — setup sheet, prerequisites/Java check, Bedrock disclosure, helper links, first-server handoff — to MSC 1's shape and ordering. Reference MSC 1 `SetupWizardView`, `PrerequisitesView`, `FirstStartSheetView` and `~/Documents/MSCSS/First Time Setup`. Preserve the agent-owned vs client-owned initiation boundary from P11 scope.
**Verify:** `npm run dev` on a fresh profile, walk setup start → first-server handoff; compare to MSC 1 + checklist.
**Commit:** `P12.13: rebuild first-time setup`
**Batch:** solo

### P12.14 — MSC Settings (app settings)
**Status:** not started
**Files:** `src/lib/sections/settings/` (app-level) or a dedicated app-settings section
**What:** Rebuild the app-wide MSC Settings sheet. Reference MSC 1 `MSCSettingsView` + `~/Documents/MSCSS/MSC settings`.
**Verify:** `npm run dev`, open MSC Settings, exercise each pane; compare to MSC 1 + checklist.
**Commit:** `P12.14: rebuild MSC settings`
**Batch:** solo

### P12.15 — Onboarding tour / contextual help
**Status:** not started
**Files:** `src/lib/help/`, onboarding overlay components
**What:** Rebuild the guided tour + contextual help anchors/overlay to MSC 1's ordering and anchors. Reference MSC 1 `OnboardingOverlayView`, the contextual-help system, and `~/Documents/MSCSS/Onboarding Tour`. Respect reduced-motion.
**Verify:** `npm run dev`, run the tour start→finish, check anchors + skip/reopen; compare to MSC 1 + checklist.
**Commit:** `P12.15: rebuild the onboarding tour`
**Batch:** solo

### P12.16 — Guides / Handbook / How MSC Works
**Status:** not started
**Files:** `src/lib/sections/handbook/`, `src/lib/help/`
**What:** Rebuild the Server Handbook, Concept Guide ("How MSC Works"), and router/port-forward guides, consuming the `GET /v1/help/{helpId}` content contract (no hardcoded divergent text). Reference MSC 1 `ServerHandbookView`, `ConceptGuideView`, `RouterPortForwardGuideSheet` and `~/Documents/MSCSS/{Guides,Server Handbook,How MSC Works}`.
**Verify:** `npm run dev`, open each guide, confirm content comes from the help contract; compare to MSC 1 + checklist.
**Commit:** `P12.16: rebuild guides and handbook`
**Batch:** solo

### P12.17 — Consistency sweep + parity gate
**Status:** not started
**Files:** `docs/msc2/clients/`, `docs/msc2/client-capability-matrix.csv`, `docs/msc2/renderings/`
**What:** Run after P12.0–P12.16. Sweep every rebuilt screen for design-system consistency (uniform sheet sizes, spacing, one card language, no drift), run the `antiAIslop.md` checklist across all screens, and do the screen-by-screen MSC 1 parity comparison (shape + behavior). Confirm the D-003 corollary (no desktop-only screen) and that the kept data layer's contract is unchanged. **This step also absorbs the deferred Phase 11 gate (P11.28k + P11.29), now evaluated against the redesigned client:** reconcile the capability matrix; prove D-003 bundle/screen identity, generated DTO drift, D-013 host isolation, capability/permission routing, D-026 served-content use, browser/Tauri auth, browser responsive behavior, native Linux WebKitGTK rendering, tri-platform packaging, and headless independence; and satisfy the first-launch preservation contract (real-agent/auth path, fresh-profile setup → Concept Guide → guided tour → Handbook, step/anchor coverage, skip/reopen, splash/reduced-motion, and the agent-owned vs client-owned initiation boundary). Record evidence. The other agent decides in REVIEW whether the combined Phase 12 gate holds.
**Verify:** `cd clients/desktop-web && npm run test:unit && npm run check`, then `python3 tools/phase11/phase11-check.py --gate && python3 tools/phase6/capability-matrix-check.py docs/msc2/client-capability-matrix.csv && cargo nextest run --workspace`, then Cameron's full visual parity + anti-slop pass across every screen against `~/Documents/MSCSS/` and MSC 1. (This is Phase 12's only full-workspace run.)
**Commit:** `P12.17: consistency sweep and parity gate`
**Batch:** solo
