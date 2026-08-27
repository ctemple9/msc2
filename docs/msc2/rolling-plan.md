# MSC 2 — Rolling Plan

> ## STATUS: Phase 11 (desktop/web clients) is in progress; **Phase 12 (client redesign) is now planned** below. Phase 11 shipped a working client wired to the real agent, but its UI diverged from MSC 1's information architecture and design language — Phase 12 rebuilds the presentation layer to MSC 1 fidelity, *refreshed*. Terminal UI moved to Phase 13. Phase 12's design system (S0) and shell (S1) were shaped and locked as reference specimens in `docs/msc2/renderings/`, governed by `docs/msc2/antiAIslop.md` (hard rule #11).
> **Next move:** P12.2 (Overview tab) is DONE — Cameron verified it 2026-08-25. P12.2b–j (Java player-data NBT backend, built with Codex) all landed and are marked DONE. P12.3/P12.3a/P12.3b/P12.3c/P12.3d/P12.3e (Players tab, session-log backend, Bedrock identify/skin fixes, real Bedrock stats/inventory) are all DONE — Cameron verified them. P12.3f (session-log client swap) and P12.3g (live-review polish) are built and **awaiting Cameron's verification**. P12.4 (Worlds tab + world wizards) is built and **awaiting Cameron's verification**. **P12.4a–e (2026-08-26) gave P12.4's five found gaps real backends and are all DONE** — Cameron verified them: P12.4a exposes Chunker's already-real `supported_formats` over HTTP, P12.4b wires the already-built `set_slot_thumbnail`/`save_thumbnail` application code to a route, P12.4c lets `POST /v1/worlds/import` redeem an already-on-disk backup, and P12.4d/e port and wire the one genuinely new feature, real Bedrock world repair. **P12.4f (2026-08-26) connects all five to their frontends and is built, awaiting Cameron's verification** — see its own entry below for a real contract/code drift it found and fixed (repair's response schema) and a pre-existing tooling inconsistency it found and deliberately left for a separate decision (`generate.ts`'s quote style vs. the rest of the repo's). See this file's 2026-08-25 notes below for the fuller history of gaps found and how each was handled. **P12.5 and P12.6 (Packs tab removal, Performance tab) are DONE** — Cameron verified them. **P12.7 (Components tab + plugin browser) is built (2026-08-26), awaiting Cameron's verification** — see its own entry below for a small additive `ScreenApi.upload` interface fix it needed and the real pre-existing backend/contract gaps it found and left alone. **P12.7a (2026-08-26) is DONE** — after seeing the real plugin browser, Cameron flagged blank authors/`0 downloads`/no icons; traced to a real two-layer backend bug (`ModrinthSearchHit` never captured those fields, and the route hardcoded them to empty anyway) and fixed both, plus the client's missing icon render. **P12.7b (2026-08-26) is DONE** — icons still didn't show after P12.7a's rebuild; the real cause was one layer deeper, a `#[serde(rename_all = "camelCase")]` acronym-casing bug turning `icon_url` into wire key `iconUrl` instead of the contract's `iconURL`, silently unread by the client. Fixed with explicit `#[serde(rename = ...)]` overrides on all three affected DTO fields plus regression tests. **P12.7c and P12.7d close the one documented gap P12.7/P12.7a left open** — MSC 1's `ModrinthProjectDetailView` (gallery, full About text, per-version compatibility + install) has no backing route. Split into two steps so Codex and Claude Code could work them independently: **P12.7c (2026-08-26) is DONE** — Codex added the contract + Rust backend (two new `GET /v1/catalog/projects/:projectId[/versions]` routes, an additive `versionId` field on the existing install request). **P12.7d (2026-08-26) is built, awaiting Cameron's verification** — the client detail sheet; found along the way that `ServerDTO` carries no Minecraft-version field, so the compatibility banner/badges thread it through from `GET /v1/components`'s existing `primaryComponent.installedVersion` instead (see its own entry for the full finding). **P12.7e (2026-08-26) is DONE** — installing a plugin through the (now-working) browser surfaced a real, pre-existing Phase 8 gap: installed add-ons never got a `currentVersion`, always reading "Unknown version," even though both values the oracle's fallback chain needs were already computed elsewhere in the codebase and simply never threaded through. **P12.7f (2026-08-26) is built, awaiting Cameron's verification** — his own UX preference, not an oracle port: the plugin row's persistent Toggle + Remove button is replaced with a click-anywhere action menu (Enable/Disable, View, Uninstall), with "View" opening `ProjectDetailSheet` for an already-installed add-on via a new, smaller `ProjectDetailItem` type instead of a contract change.
> **P12.3 blocked on missing backend (decided 2026-08-25):** before rebuilding the Players tab, Cameron flagged that MSC 1's Players tab includes a read-only Java player inventory/stats viewer (`PlayerNBTReader.swift` + `PlayerInventoryView.swift`, hosted in `PlayerProfileDetailSheet.swift`) that never made it past the file-inventory audit into an actual phase step — no domain crate, no API route, and P12.3's own `What:` line never mentioned it. Investigation found `GET /v1/players/profiles` is **already frozen in the API contract** (`docs/msc2/api-contract/openapi.json`: `PlayerProfileDTO`/`PlayerStatsDTO`/`InventoryItemDTO`, plus `POST /v1/players/hidden`, `POST /v1/players/skin-override`, `GET /v1/players/{profileId}/skin`) but has **no handler at all** — today `GET /v1/players` only serves Bedrock (`crates/msc-agent/src/routes/bedrock.rs`; a Java server gets `note: "not_bedrock"`, empty list). This is a straight port against an already-frozen contract, not new API design. Cameron chose to block P12.3 and build the backend first (steps P12.2b–P12.2j below) rather than ship Players tab without it. Online Now / Seen This Session / Session Log are unaffected — those are console-derived (already built in P11.11) and stay in P12.3 itself. **Mutation actions, decided 2026-08-25:** of MSC 1's 5 player-data mutation actions (migrate to offline UUID, migrate to manual UUID, copy, duplicate, delete), none were in the frozen contract. Cameron chose **4 of the 5** — delete, migrate-to-offline-UUID, migrate-to-custom-UUID, and duplicate — added as new steps P12.2g (contract amendment) through P12.2j (route wiring), fully specified (exact DTO field names/types, exact error codes, pinned known-answer test vectors for the offline-UUID algorithm) since Cameron is running these with Codex. `copyPlayerData` (overwrite one player's data onto another's) is the one action still **deferred, not dropped** — add it later as its own contract-amendment step when wanted.
> **Phase 11 → 12 sequencing (decided 2026-08-25):** the committed P11.28g–j agent work is done and carries forward as Phase 12's foundation. The two unfinished Phase 11 steps — P11.28k and the P11.29 gate — are **superseded and folded into P12.17**, because they verify the first-launch UI and MSC 1 fidelity that only the redesign delivers; the whole client gate now runs once against the redesigned client. Phase 12 begins now.
> **Last updated:** 2026-08-26 (P12.7f)

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
| **12** | Client redesign (MSC 1 fidelity, refreshed) | **in progress — P12.2b–j, P12.3/a–e, P12.4a–j, P12.5, P12.6, P12.7a, and P12.7b DONE; P12.3f/g, P12.4, and P12.7 built, awaiting Cameron's verification** |
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
**Status:** DONE. Rebuilt against the now-real P12.2b–j backend: `OnlineNowCard` (Online Now / Seen This Session), `BedrockAllowlistCard` (Bedrock-only), `SessionLogCard` (filter/day-grouping/show-more/clear), `PlayerDataCard` (search/sort/hidden-toggle grid) → `PlayerDetailSheet` (identity, skin lookup override, stats, inventory via a new `InventoryGrid`, and the 4 approved mutations — migrate-offline, migrate-custom, duplicate, delete — plus hide/unhide). New `crates`-adjacent client files only; no backend touched. Regenerated `src/lib/api/generated.ts` from the current contract (P12.2g's routes weren't in it yet). Real gap found and handled the same way as this file's other honest gaps rather than built around quietly: `GET /v1/session-log` is *also* frozen-but-unimplemented (zero handler anywhere in `crates/`) — Session Log is derived client-side from `/v1/console/tail` by reusing `chatFeed.ts`'s existing join/leave parsing (already built for Overview's Chat card) rather than adding a second backend detour; "Clear Log" is client-local per host+server (localStorage cutoff timestamp), same "no server field yet" treatment `notes.ts` already established, since the console tail is a bounded recent buffer with nothing durable on the agent to send a clear mutation to. **Follow-up planned, not just noted:** P12.3a–c (below) give this a real backend — MSC 1's own `SessionLogManager.swift`/`session_log.json` ported directly, plus wiring the Java join/leave events `lifecycle.rs` already detects and currently discards. A later client step (not yet written) swaps this section over to it. `copyPlayerData` stays deferred per this file's 2026-08-25 note — only 4 of 5 mutation actions are wired, matching P12.2g–j. Two design decisions confirmed with Cameron mid-build rather than improvised per his own added S0-extension note: kept 2 new `Icon.svelte` glyphs (`clock`, `id-card`, same 24x24/stroke-1.8 language as the existing set); kept an inline expand-in-place delete confirmation (no modal) over migrating the still-Phase-11-styled `ConfirmDialog.svelte`. Anti-slop self-review caught and removed two accent colors I'd introduced outside the locked status ramp (a gold Operator badge, a blue enchanted-item glow) — fixed to neutral before commit. **Not verified: live browser check against real agent data.** Got the dev server running and the shell rendering (past the first-launch splash) via Playwright, but reaching Players requires a paired agent + registered server, and touching Cameron's real local agent/server registration to manufacture that felt like the wrong call to make unprompted — leaving that verification to Cameron, who already has real servers (`test`, `campak`, `test_2`) to check it against directly, which is a better test than anything synthetic here anyway. Structural (`npm run test:screen-players-online`, 11 tests, rewritten — the old suite asserted `/v1/players/profiles` must *not* be used, which was P11.11's deliberate scope limit and is now obsolete) and `svelte-check`/`prettier --check` both pass on every touched file with zero new errors.
**Files:** `src/lib/sections/players-online/`, `src/lib/components/base/Icon.svelte`, `src/lib/api/generated.ts`
**What:** Rebuild Players — Online Now / Seen This Session, Session Log (filter + clear), Player Data (profiles, sort, stats + inventory detail sheet, hidden toggle, skin/avatar, delete). Reference MSC 1 `DetailsPlayersTabView` + `PlayerProfilesCard`/`PlayerProfileDetailSheet`/`PlayerInventoryView` and the Players screenshot.
**Verify:** `npm run dev`, open Players against a real connected agent; compare to MSC 1 + checklist. Structural: `npm run test:screen-players-online`.
**Commit:** `P12.3: rebuild the Players tab`
**Batch:** solo

### P12.3a — Amend the contract: add POST /v1/session-log/clear
**Status:** DONE
**Files:** `docs/msc2/api-contract/openapi.json`, `tools/api-contract-check.py`
**What:** `GET /v1/session-log` → `SessionLogResponseDTO` is already frozen (from an earlier phase) but has no handler anywhere in `crates/` — P12.3b/c implement it. This step covers the one piece that's missing from the contract entirely: MSC 1's Clear Log button (`SessionLogManager.clearEvents`) has nothing to call. Add:

`POST /v1/session-log/clear` — `x-permission-category: "players"` (a real mutation, unlike GET's own `"none"` read-only category; matches the category already used for `/v1/players/*`). No request body. Returns the same `SessionLogResponseDTO` schema GET already uses, with `events: []` — same "always return fresh state" pattern P12.2g's `PlayerMutationResultDTO.profiles` already established. Error responses: `409 conflict` / `no_active_server` (`ErrorDTO`, matching every other active-server-scoped mutation's shape); no `404`/`400` case exists since there's no target id to miss or body to malform.

Bump `EXPECTED_TOTAL` in `tools/api-contract-check.py` by 1, append a clause to its running-total comment in the same style as every prior entry there.
**Verify:** `python3 tools/api-contract-check.py`
**Commit:** `P12.3a: amend the contract with POST /v1/session-log/clear`
**Batch:** solo

### P12.3b — Port the session log store and wire the live Java hook
**Status:** DONE
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
**Status:** DONE
**Files:** `crates/msc-agent/src/routes/session_log.rs` (new), `crates/msc-agent/src/routes/mod.rs`, `crates/msc-agent/src/main.rs`
**What:** Implement both routes against P12.3b's `session_log` module. `GET /v1/session-log`: `session_log::load_events` for the active server, mapped to `SessionEventDTO { id, playerName: player_name, eventType: event_type as string, timestamp }`, wrapped in `SessionLogResponseDTO { activeServerId: Some(id), events }`; no active server → `events: []`, `activeServerId: None` (matches this route's existing no-error 200-empty convention, same as `GET /v1/players` today for a not-yet-selected server — do not 409 here). `POST /v1/session-log/clear`: `409 conflict`/`no_active_server` if none active, else `session_log::clear_events` then return the same shape with `events: []`.

Tests for these two routes belong inline (`#[cfg(test)] mod tests`) in the new `routes/session_log.rs` itself, matching every other route file's convention (settings.rs, players.rs, etc.) — `msc-agent` has no lib target, only its `msc` bin, so `--bin msc <filter>` is the only way to reach them.
**Verify:** `cargo fmt --check && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --bin msc session_log`
**Commit:** `P12.3c: wire GET /v1/session-log and POST /v1/session-log/clear`
**Batch:** solo

### P12.3d — Add and wire POST /v1/players/identify, fix Bedrock skin resolution, split Head/Body preview
**Status:** DONE. Cameron shared real MSC 1 screenshots (Bedrock player "camkage" on `campak`) mid-review of P12.3, surfacing two more real gaps beyond session log, same shape as before: (1) MSC 1's "Identify Player"/"Change Gamertag" action (`identifyBedrockPlayer` → `BedrockNameCache.record`) had no route at all — but the application-layer function already existed (`bedrock_players::record_name`, presumably from earlier Bedrock work), just never wired to HTTP; (2) the Skin Override section shows Head *and* Body thumbnails, not the single portrait P12.3 shipped with — and while investigating, found the client's own `avatarUrl` unnecessarily hard-excluded Bedrock entirely, when `bedrock_profile_to_dto` (P12.2e) already computes a working dotted-gamertag `imageIdentifier` for Bedrock, matching MSC 1's own `PlayerProfile.imageIdentifier` scheme exactly (confirmed working in Cameron's screenshot). Both approved by Cameron before building, per the same design-decision-check discipline as everything else in this phase.

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
**Status:** DONE
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

### P12.3f — Swap the client's Session Log over to the real backend (Java), keep console-tail for Bedrock
**Status:** awaiting verification. Closes the gap P12.3's own write-up flagged ("A later client step... swaps this section over to it") now that P12.3b/c gave Session Log a real backend. Confirmed first that P12.3e's Bedrock stats/inventory carry-through needed **no client change at all** — `PlayerDetailSheet.svelte`'s Stats/Inventory sections already render generically off `profile.stats`/`profile.inventory` with no Java-only gate, so real Bedrock data just starts appearing once loaded; only fixed a stale comment there that still said Bedrock stats/inventory were Java-only.

Session Log itself did need real wiring, done inline (Cameron approved skipping a separate plan-first pass since it's client-only and contained): `PlayersOnlineSection.svelte`'s `loadSessionEvents` now branches on `isBedrock` — Java calls the real `GET /v1/session-log` (mapped via new `sessionEventsFromLog` in `model.ts`), Bedrock keeps deriving from `/v1/console/tail` (P12.3b's own scope note: Bedrock has no live-drain wiring for `BedrockServiceEvent` yet, so this isn't a regression). `onClearSessionLog` likewise branches — Java calls the real `POST /v1/session-log/clear`, Bedrock keeps the `localStorage` cutoff-timestamp fallback (nothing durable to clear server-side there). `visibleSessionEvents` only applies the client-side `clearedAt` filter for Bedrock now, since Java's clear is real (the backend actually deletes the events, so re-applying a stale local cutoff would incorrectly hide events for a Java player after any earlier client-local clear).
**Files:** `clients/desktop-web/src/lib/sections/players-online/{model.ts,PlayersOnlineSection.svelte,PlayerDetailSheet.svelte}`, `clients/desktop-web/tests/screens/players-online.test.ts`
**What:** Wire `GET /v1/session-log` and `POST /v1/session-log/clear` into the Players tab for Java servers; keep the existing console-tail derivation as the Bedrock fallback since Bedrock has no live session-log backend yet.
**Verify:** `npx svelte-check --tsconfig ./tsconfig.json` (only the 7 pre-existing unrelated errors in `auth/desktop.ts`, `SetupIntro.svelte`, `route.ts`, `desktop.test.ts` should remain — none in the touched files), `npx prettier --check src/lib/sections/players-online/model.ts src/lib/sections/players-online/PlayersOnlineSection.svelte src/lib/sections/players-online/PlayerDetailSheet.svelte tests/screens/players-online.test.ts`, `npm run test:screen-players-online` (14 tests).
**Commit:** `P12.3f: swap Session Log to the real backend for Java, keep console-tail for Bedrock`
**Batch:** solo

### P12.3g — Live polish from Cameron's first real-server review: uniform action buttons, hidden sheet scrollbar, flippable health cards
**Status:** awaiting verification. Cameron's first real look at the Players tab against real `campak` data (P12.3a-f all confirmed working end to end — real profile, real 4-event session log) surfaced three small live-UI notes, fixed inline as found rather than planned first, matching the P12.3f precedent for contained client-only work:

1. **Data Management action buttons were different widths** (`PlayerDetailSheet.svelte`'s `.actions`) — `align-items: flex-start` let each `Button` shrink to its own label's width. Changed to `align-items: stretch` so every action (Migrate to Offline UUID, Migrate to Custom UUID, Duplicate, Delete Player Data, Hide Profile, and the inline confirm/gamertag rows) fills the same full width.
2. **Visible scrollbar track on the Player Profile sheet** — `Sheet.svelte`'s `.sheet` is the actual scrolling element for every sheet in the app, not just this one. Applied the same `scrollbar-width: none` + `.sheet::-webkit-scrollbar { display: none; }` pattern already established in `FirstLaunchGate.svelte` for the onboarding gate — content still scrolls, the track just doesn't render. Fixes this for every sheet, not only Players.
3. **Server Health cards on Overview were uneven and information-heavy** (`HealthGrid.svelte`) — originally a P12.2 design call (`docs/msc2/renderings/status-card.html`) that deliberately dropped MSC 1's real flip interaction ("no 3D flip... detail line + repair action sit right on the one card face"). Cameron reviewed MSC 1's actual behavior live and asked to restore it, with two explicit deviations from MSC 1: keep the existing responsive `repeat(auto-fit, minmax(180px,1fr))` grid (not MSC 1's fixed 3-column + "essential 3 / show all" toggle — Cameron prefers the column count scaling with window width, already MSC 2's behavior), and drop the Server Directory card entirely (5 cards remain: Java Runtime, RAM Allocation, Last Startup, Port Reachability, Add-on Jars, plus Bedrock's VM Runtime/World Data when applicable).

   Ported MSC 1's actual mechanism (`HealthGridCardTile.rotation3DEffect`, confirmed via oracle read of `HealthCardsGridView.swift`) as a CSS 3D flip: front face is icon + title + status dot/label only (no detail, no button); a small chevron (added to `Icon.svelte`, `content-icon` style) is the only affordance that the card is interactive; click/tap/Enter/Space flips it (`perspective` + `transform-style: preserve-3d` + `backface-visibility: hidden`, 320ms ease, one card flipped at a time via a single `flippedId` matching MSC 1's single `flippedCardID`); back face repeats icon/title/status plus the full detail text (3-line clamp) and the existing repair-action button when the card carries one. Both faces share one fixed tile height (110px) so flipping never reflows the grid — MSC 1 grew the tile's frame height on flip instead, which would have caused CSS grid row jank here. Updated `docs/msc2/renderings/status-card.html` to lock in the revised design, per this file's own rule that a new UI pattern gets a specimen rather than a one-off.
**Files:** `clients/desktop-web/src/lib/sections/players-online/PlayerDetailSheet.svelte`, `clients/desktop-web/src/lib/components/base/{Sheet.svelte,Icon.svelte}`, `clients/desktop-web/src/lib/sections/home/HealthGrid.svelte`, `docs/msc2/renderings/status-card.html`
**What:** Three small live-review fixes: uniform-width Data Management action buttons, hidden scrollbar track on all sheets, and a restored flip interaction + reduced front-face content + dropped Server Directory card on the Overview Server Health grid.
**Verify:** `npx svelte-check --tsconfig ./tsconfig.json` (only the same 7 pre-existing unrelated errors noted in P12.3f should remain), `npx prettier --check src/lib/sections/players-online/PlayerDetailSheet.svelte src/lib/components/base/Sheet.svelte src/lib/components/base/Icon.svelte src/lib/sections/home/HealthGrid.svelte`.
**Commit:** `P12.3g: uniform action buttons, hidden sheet scrollbar, flippable health cards`
**Batch:** solo

### P12.4 — Worlds tab (+ world wizards)
**Status:** awaiting verification. Rebuilt against `DetailsWorldsTabView.swift`: a World Slots grid (`WorldSlotCard` — real/placeholder thumbnail, size + Active badges, Activate/Convert/Rename/Delete) plus a `BackupsPanel` for whichever slot is selected (day-grouped, auto-backup toggle+interval, Back Up Now, Restore/Delete, Legacy/Unmatched sub-list), `CreateWorldSheet`, `RenameWorldSheet`, `WorldRepairSheet`, and `WorldConversionWizard`. New `crates`-adjacent client files only; no backend touched.

**Scope correction found before building, not guessed from the plan step's own wording:** the step named "Rename/Replace/Convert/Repair" as the world wizards, but grepping the oracle shows `RenameWorldView.swift` and `ReplaceWorldView.swift` are never instantiated anywhere in `minecraft-server-controller` — dead code superseded by `DetailsWorldsTabView`'s own inline `RenameSlotSheet`, with no "replace the live world" affordance in this tab at all. Both live-world wizards, plus Duplicate and Import ZIP, belong to `ServerEditorWorldTab.swift` instead (Phase 12.12's Server Editor sheet) and are ported there, not here. This step ports exactly what `DetailsWorldsTabView` itself does: Create, Save Current, Activate, the inline Rename sheet, Delete, Convert, and (Bedrock) Repair — confirmed against the real, frozen `WorldReplaceRequestDTO`/`WorldReplaceActiveRequestDTO` split in `crates/msc-agent/src/routes/worlds.rs`, which independently corroborates the same split.

**Real gaps found and handled the same way as this file's other honest gaps — not built around quietly:**
- `POST /v1/worlds/repair` is fully frozen and wired end to end, but its route handler always returns `409 repair_unavailable` today (its own doc comment: the level.dat regeneration workflow doesn't exist on this runtime yet). Built the complete real MSC 1 flow (prompt → busy → done) rather than faking a log-streaming animation; the `repair_unavailable` response gets its own honest "not available on this runtime yet" terminal state, and the same call will just start succeeding the moment a future step wires the real regeneration — no client change needed then.
- `WorldConvertRequestDTO.targetFormat` must be one of Chunker's own installed format strings, and MSC 1's wizard always populates that picker from a live query (`ChunkerManager.supportedFormats`) — but no HTTP route exposes `WorldConverter::supported_formats` yet. Rather than fabricate a hardcoded format list or a freeform text field the user has to guess a working value for, `WorldConversionWizard` runs its real Preflight and Target Server steps, then stops at an honest "not available yet" panel instead of a fake version picker. **Follow-up needed, not just noted:** a contract-amendment step shaped like P12.3a (add a route over `supported_formats`) would let this wizard finish the rest of the way with no other changes.
- No route exists to set a world slot's thumbnail (`GET /v1/worlds/{slotId}/thumbnail` is read-only) — `WorldSlotCard` fetches and shows a real thumbnail when `hasThumbnail` is true, else falls back to the same deterministic gradient placeholder `ActiveWorldCard.svelte` already established on Overview (P12.2, that component's own comment named this tab as the one that would "own real world art"). No "Set Thumbnail" affordance is offered since nothing would back it.
- "Import as Slot" for a legacy/unmatched backup has no matching route (`POST /v1/worlds/import` only redeems a client-staged upload, not an already-on-disk agent path) — shown, disabled, with an explanation, rather than wired to a route that doesn't fit.
- `POST /v1/backups/now` takes no `slotId` — it always backs up whichever slot is active, unlike MSC 1's per-selected-slot manual backup. Labeled plainly ("Back Up Now") rather than implying slot targeting.
- `POST /v1/backups/restore` 409s if the backup's slot isn't the currently active one, and is Bedrock-unsupported entirely (`crates/msc-agent/src/routes/backups.rs`) — narrower than MSC 1's own restore-into-any-owning-slot behavior. Restore is disabled with an explanation for a non-active-slot backup or on Bedrock, rather than calling a route that would just reject it.

Destructive actions (Activate, Delete Slot) use the same inline expand-in-place confirmation P12.3g established for this app (no modal `ConfirmDialog`). Activate/Convert/Back-Up-Now/Restore are all operation-backed (`docs/msc2/worlds/phase6-api.md`) — a new `pollOperation` helper in `model.ts` polls `GET /v1/operations/{id}` to a terminal state. **Not verified: live browser check against real agent data** — same reasoning as P12.3: reaching Worlds needs a paired agent + registered server, and manufacturing that unprompted felt like the wrong call; `svelte-check`/`prettier --check`/`npm run build` all pass with zero new errors, and the dev server + Playwright shell smoke shows no new console errors (the pre-existing `/v1/capabilities` 404 and splash-video abort are unrelated to this change, same as prior phases). Structural (`npm run test:screen-worlds-backups`, rewritten, 13 tests) passes.
**Files:** `src/lib/sections/worlds/{model.ts,WorldsSection.svelte,WorldSlotCard.svelte,BackupsPanel.svelte,CreateWorldSheet.svelte,RenameWorldSheet.svelte,WorldRepairSheet.svelte,WorldConversionWizard.svelte}`
**What:** Rebuild Worlds — World Slots (cards w/ thumbnail, Active badge, Activate/edit/delete, Save Current/Create New) and Backups. Include the world wizards (Rename/Replace/Convert/Repair) as sheets. Reference MSC 1 `DetailsWorldsTabView`, `WorldSlotsView`, the world wizard views, and the Worlds screenshot.
**Verify:** `npm run dev`, open Worlds and each wizard; compare to MSC 1 + checklist. Structural: `npm run test:screen-worlds-backups`.
**Commit:** `P12.4: rebuild the Worlds tab`
**Batch:** solo

### P12.4a — Amend the contract and wire Chunker's supported conversion formats
**Status:** DONE
**Files:** `docs/msc2/api-contract/openapi.json`, `tools/api-contract-check.py`, `crates/msc-api/src/dto/worlds.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/tests/world_backup_routes.rs`
**What:** Closes P12.4's own gap: `WorldConversionWizard.svelte` stops at an honest "not available yet" panel because nothing lets a client list Chunker's installed target formats before submitting `POST /v1/worlds/convert`. The good news found while scoping this: `crates/msc-agent/src/routes/worlds.rs`'s `LiveWorldConverter` is **already a real production implementation** (`resolve_java_path`/`is_installed`/`supported_formats` really shell out to `java`/the installed `chunker-cli.jar`) — `convert()`'s own validation already calls `converter.supported_formats(&resolved_java_path)` before running a conversion. This step only exposes that same already-working call as its own route; it adds no new capability to the agent.

Add `GET /v1/worlds/convert/formats` (`x-permission-category: "worlds"`) → new `WorldConvertFormatsResponseDto { formats: Vec<String> }` (raw, unfiltered list — both `JAVA_*` and `BEDROCK_*` prefixed strings, matching `WorldConversionWizardView.targetFormats`'s own client-side `hasPrefix` filtering, which `WorldConversionWizard.svelte` should keep doing rather than filtering server-side). No request body; resolves against the active server the same implicit way every other `/v1/worlds/*` route does. Errors: `409 capability_unavailable` when no Java can be resolved or Chunker isn't installed — reuse the exact two messages `convert()` already returns today ("No Java runtime could be resolved for Chunker." / "Chunker is not installed on this agent."), not new wording, so the two surfaces stay consistent.

Bump `EXPECTED_TOTAL` in `tools/api-contract-check.py` by 1, append a clause in the same style as every prior entry there. Backend only — `WorldConversionWizard.svelte` calling this route and finishing the version/placement/summary/converting steps is this phase's own "connect the backends to the frontends" follow-up, not part of this step.
**Verify:** `python3 tools/api-contract-check.py && cargo fmt --check && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --test world_backup_routes` (NOT a bare `-p msc-agent world_backup_routes` filter — see this file's 2026-08-25 nextest-scoping note; use `--test world_backup_routes` so only this one binary builds)
**Commit:** `P12.4a: expose Chunker's supported conversion formats`
**Batch:** solo

### P12.4b — Amend the contract and wire world-slot thumbnail upload
**Status:** DONE
**Files:** `docs/msc2/api-contract/openapi.json`, `tools/api-contract-check.py`, `Cargo.lock`, `crates/msc-api/src/dto/backups.rs`, `crates/msc-api/src/dto/worlds.rs`, `crates/msc-agent/Cargo.toml`, `crates/msc-agent/src/routes/components.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/tests/world_backup_routes.rs`
**What:** Closes P12.4's other gap: `GET /v1/worlds/{slotId}/thumbnail` is already real (P6.21), but nothing lets a client *set* one, even though the application layer already can — `crates/msc-application/src/worlds.rs::set_slot_thumbnail` and `crates/msc-infrastructure/src/world_store.rs::save_thumbnail` (both from P6.10/P6.12) are fully built and just never got an HTTP door. Reuse this app's existing generic staged-upload mechanism (`crates/msc-agent/src/routes/components.rs`'s real, already-wired `/v1/staged-uploads` + `/v1/staged-uploads/:id` routes, the same ones `POST /v1/worlds/import` already redeems from) rather than inventing a second, raw-bytes upload path — add a `WorldThumbnail` variant to `StagedUploadPurposeDto` (`crates/msc-api/src/dto/backups.rs`) alongside the existing `WorldImport`/`ActiveWorldReplace`/etc.

Add `POST /v1/worlds/{slotId}/thumbnail` (`x-permission-category: "worlds"`) → new `WorldThumbnailUploadRequestDto { stagedUploadId: String }` → reuse `WorldMutationResultDTO` for the response (same "always return fresh state" shape every other slot mutation already uses — no new result DTO needed). Redeem exactly like `import` does: missing/expired/wrong-purpose staged id is a plain `404`; on success, decode the staged bytes as an image and call `worlds::set_slot_thumbnail`, then delete the staged file the same way `import` already does. `404 not_found` if the slot doesn't exist. Decoding-failure/oversized-image handling can reuse whatever error shape `set_slot_thumbnail` already reports — don't invent new error codes here.

Bump `EXPECTED_TOTAL` in `tools/api-contract-check.py` by 1. Backend only — unlike P12.4a/c (which just remove a client-side `disabled`), no client UI for choosing/uploading a thumbnail exists yet at all (`WorldSlotCard.svelte` only ever *displays* one); adding that affordance is this phase's own "connect the backends to the frontends" follow-up, not part of this step.
**Verify:** `python3 tools/api-contract-check.py && cargo fmt --check && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --test world_backup_routes`
**Commit:** `P12.4b: wire world-slot thumbnail upload`
**Batch:** solo

### P12.4c — Amend the contract and wire "import a legacy backup as a new slot"
**Status:** DONE
**Files:** `docs/msc2/api-contract/openapi.json`, `crates/msc-api/src/dto/worlds.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/tests/world_backup_routes.rs`, `crates/msc-agent/src/cli/mod.rs`, `crates/msc-api/tests/world_backup_conformance.rs`
**What:** Closes `BackupsPanel.svelte`'s disabled "Import as Slot" action for a legacy/unmatched backup. `POST /v1/worlds/import` already does everything needed (`worlds::import_zip_as_new_slot` just takes any `&Path`) except accept a source that's already on the agent's own disk instead of a client-staged upload — a legacy backup is already sitting under this server's backups folder, so re-uploading it through the client would be actively worse than a route that just points at the file it can already see.

**File list widened after this plan's first draft, not silently expanded during EXECUTE:** the original `Files:` line omitted `crates/msc-agent/src/cli/mod.rs` and `crates/msc-api/tests/world_backup_conformance.rs`. An executing agent correctly stopped short rather than touch either file outside its declared scope: adding a new field to an existing `WorldImportRequestDto` struct means every other Rust site that already constructs one by name (a Rust struct literal, unlike JSON, doesn't let `#[serde(default)]` paper over a missing field) fails to compile without it — the `msc worlds import` CLI command and the DTO-shape conformance test both do. Widened the list 2026-08-26 to cover both: one line each (`backup_id: None`, matching the "CLI never gains this capability, only the client's Import-as-Slot UI does" scope decision) plus one new conformance case exercising the `backup_id` variant.

Widen `WorldImportRequestDto`: `staged_upload_id` keeps its `String` type but gains `#[serde(default)]` (empty string when the field is omitted from JSON — note this only relaxes *deserialization*; a Rust struct literal still has to name every field, `backup_id` included, which is exactly what broke the two call sites above), and a new `backup_id: Option<String>` is added — exactly one of the two must be non-empty/present, the same "exactly one of X or Y" shape `WorldConvertRequestDto` (`targetName`/`targetSlotId`) already established in this same file, so no new validation idiom is introduced. When `backupId` is given, resolve its on-disk path via `backups::list_backups` (the same lookup `routes/backups.rs::restore` already performs against `entry.filename == body.backup_id`) instead of redeeming a staged upload, then call the existing `worlds::import_zip_as_new_slot` unchanged — and skip the staged-file cleanup afterward, since there's no staged file to remove when the source was already a real backup on disk. `404 backup_not_found` for an unknown id (matching `backups::restore`'s own `backup_not_found` code, not `worlds.rs`'s usual bare `not_found`, since this is genuinely a backup lookup miss). This adds no new path/method to the contract (same `POST /v1/worlds/import` route), so `EXPECTED_TOTAL` does not change — only the request schema grows. Backend only — `BackupsPanel.svelte` removing the `disabled` on "Import as Slot" and calling this is this phase's own "connect the backends to the frontends" follow-up, not part of this step.
**Verify:** `python3 tools/api-contract-check.py && cargo fmt --check && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo clippy -p msc-api --all-targets -- -D warnings && cargo nextest run -p msc-agent --test world_backup_routes && cargo nextest run -p msc-api --test world_backup_conformance`
**Commit:** `P12.4c: wire importing a legacy backup as a new world slot`
**Batch:** solo

### P12.4d — Port the Bedrock world-repair orchestration
**Status:** DONE
**Files:** `crates/msc-application/src/world_repair.rs` (new), `crates/msc-application/src/lib.rs`, `crates/msc-application/tests/world_repair.rs` (new)
**What:** Ports `AppViewModel+WorldRepair.swift::repairWorldLevelDat` — the only genuinely new backend feature among P12.4's gaps, not just a missing route. Read `AppViewModel+WorldRepair.swift` in full before starting (already read once for P12.4's own investigation; re-read for the exact ordering, since every step here is safety-relevant). Sequence, matching source exactly: read `level-name` from `server.properties` → mandatory safety backup (abort on failure, nothing else touched) → rewrite `level-name` to a throwaway temp value → start the server and poll until it reaches its ready state (deadline: 180s, matching source) → stop it and wait for it to fully exit (deadline: 30s) → copy `level.dat`/`level.dat_old`/`levelname.txt` from the temp world folder into the real one (missing source files are skipped, not an error) → delete the temp world folder → restore the original `level-name` in `server.properties` regardless of outcome (every failure branch in source restores this before returning `false`).

Follow this codebase's own established "policy vs. mechanism" split (`msc_application::backups::BackupConsole`, `msc_application::world_conversion::WorldConverter`) rather than reaching directly into `crates/msc-agent`'s real process/lifecycle machinery from this crate: define a small port trait (e.g. `RepairServerControl`) covering exactly "start", "is ready", "stop", "is still running" — the same four signals source's own polling loops read (`lifecycle.serverReadyForAutoMetrics`, `isServerRunning`). This step ships the orchestration and fixtures against a scripted fake implementation of that port (mirroring `FakeWorldConverter`'s own precedent in `tests/world_conversion.rs`); the real production implementation over this agent's actual Bedrock start/stop/readiness primitives (`crates/msc-application/src/bedrock_service.rs`, `bedrock_runtime.rs`) is P12.4e's job, not this one. No route changes in this step — `POST /v1/worlds/repair` keeps returning its current `409 repair_unavailable` stub throughout.
**Verify:** `cargo fmt --check && cargo clippy -p msc-application --all-targets -- -D warnings && cargo nextest run -p msc-application --test world_repair` (NOT a bare `-p msc-application world_repair` filter — `msc-application` has 60+ separate integration-test binaries; `--test world_repair` builds only this one)
**Commit:** `P12.4d: port the Bedrock world-repair orchestration`
**Batch:** solo

### P12.4e — Wire POST /v1/worlds/repair to the real repair orchestration
**Status:** DONE
**Files:** `docs/msc2/api-contract/openapi.json`, `crates/msc-api/src/dto/worlds.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/tests/world_backup_routes.rs`
**What:** Replaces today's always-`409 repair_unavailable` stub (`repair()`'s own doc comment already names this as deliberate until "the real regeneration workflow exists"). Implement P12.4d's `RepairServerControl` port against this agent's real Bedrock lifecycle (start/stop through the same path `routes/lifecycle.rs::start_active_server`/`stop_active_server` already use; readiness/running-state through whatever `bedrock_service.rs`/`bedrock_runtime.rs` already expose for this exact purpose — read both in full before wiring, don't assume a signal exists that isn't already there). Keep every existing guard in `repair()` (Bedrock-only, active-slot-only, not-currently-running) exactly as they are today; only the final `error_response(... "repair_unavailable" ...)` line changes.

Real wall-clock cost here (up to source's own 180s start-timeout) rules out a synchronous response — make this operation-backed like `activate`/`convert`. **This is a contract amendment, not just a route change:** `POST /v1/worlds/repair`'s response schema changes from the currently-frozen `WorldMutationResultDTO` to a new `WorldRepairResultDto { result: String, operationId: Option<String> }`, matching `WorldActivateResultDto`'s exact shape — same path and method, so `EXPECTED_TOTAL` doesn't change, only the response schema. The client side (`WorldRepairSheet.svelte`) currently expects the old synchronous `WorldMutationResultDTO` shape and needs updating to poll `operationId` the same way `WorldsSection.svelte::confirmActivate` already does — that client update is this phase's own "connect the backends to the frontends" follow-up once Cameron is back, not part of this step.
**Verify:** `python3 tools/api-contract-check.py && cargo fmt --check && cargo clippy -p msc-agent --all-targets -- -D warnings && cargo nextest run -p msc-agent --test world_backup_routes`
**Commit:** `P12.4e: wire real Bedrock world repair`
**Batch:** solo

### P12.4f — Connect the P12.4a-e backends to their frontends
**Status:** DONE. Closes every "connect the backends to the frontends" follow-up P12.4a-e's own write-ups flagged. `WorldConversionWizard.svelte` now runs the full real flow — Preflight (now also checks Chunker/Java via P12.4a's route, not just source-stopped/target-exists), Target Server, Target Version (a real `Select` populated from `GET /v1/worlds/convert/formats`, filtered by edition and displayed via a ported `ChunkerManager.displayName(forFormat:)`), Summary (editable new-slot name), Converting (polls the real `operationId` via `pollOperation`, showing the operation's live `statusLine`), and Done/Failed terminal states — calling the real `POST /v1/worlds/convert`. One MSC 1 placement option is still intentionally absent, not overlooked: replacing an existing slot on the target server needs that server's own slot list, and `/v1/worlds` only ever answers for the *active* server — there's no route to list a different, non-active server's slots, so every conversion here places into a new slot until that gap gets its own backend step (not raised as a question — it doesn't change anything a wizard user would need to weigh in on, only what's offered).

`WorldRepairSheet.svelte` now expects the real operation-backed `WorldRepairResultDTO` instead of the old synchronous `WorldMutationResultDTO`, polls to a terminal state showing each `statusLine`, and its "not available" branch now checks for `capability_unavailable` (the real Bedrock-runtime-missing code `require_runtime` returns) instead of the old stub's `repair_unavailable`, which no longer exists anywhere in the real `repair()` handler. `BackupsPanel.svelte`'s "Import as Slot" is no longer disabled — it calls the real `POST /v1/worlds/import` with `backupId`, naming the new slot the same way `AppViewModel+WorldSlots.swift::importLegacyBackupAsNewSlot` does (the backup's own recorded slot name, else "Imported {displayName}", else a flat fallback — ported as `legacyImportName` in `model.ts`).

`WorldSlotCard.svelte` gains the one piece that had no client UI at all before this step: a "Set Thumbnail" control. MSC 1 offers this only through a right-click context menu; this app has never established a context-menu pattern anywhere else, so it's a small always-visible overlay button on the thumbnail instead (same capability, more discoverable) — file picking reuses the same `getPlatform().pickFile`/browser-input-fallback shape `TransferPanel.svelte` already established, then `api.upload('world-thumbnail', bytes)` and `POST /v1/worlds/{slotId}/thumbnail` redeem it, matching P12.4b's real staged-upload plumbing exactly.

**One real bug found and fixed along the way, not left for the frontend to work around:** `docs/msc2/api-contract/openapi.json`'s `POST /v1/worlds/repair` still documented its `200` response as `WorldMutationResultDTO` even though P12.4e's actual Rust handler returns the new `WorldRepairResultDto { result, operationId }` — a real contract/code drift that slipped past `api-contract-check.py` (it doesn't cross-check a response schema against what the handler actually constructs). Fixed the contract to reference `WorldRepairResultDTO`, confirmed against the handler's own `Json(WorldRepairResultDto { ... })` literal, then regenerated `src/lib/api/generated.ts` — which also picked up P12.4a/b/e's schemas for the first time (none of those three steps' own Verify lines included client regeneration, so `generated.ts` had been silently stale by three schemas since they landed).

**One pre-existing tooling inconsistency found, deliberately not fixed here — flagged instead of silently worked around:** `clients/desktop-web/src/lib/api/generate.ts` has hardcoded `singleQuote: false` in its own internal Prettier call since P11.29i, but every previously-committed `generated.ts` (and the project's own `.prettierrc`) is single-quoted. Running `npm run api:generate` now produces double-quoted output that fails the project's own `prettier --check`; reformatting it back to single quotes (done here, to match every other file in the repo) then makes `npm run api:generate -- --check` itself report "stale," because that check demands an exact byte match against its own double-quoted internal formatting. The two checks currently cannot both pass at once. This session's `generated.ts` is single-quoted (matching the rest of the codebase and every prior commit) and verified correct by `svelte-check`, not by `api:generate -- --check`, which is skipped here for that reason. Needs its own decision (drop `singleQuote: false` from `generate.ts`, or accept double-quoted generated output project-wide) — not decided or silently picked in this step.

**Unrelated, pre-existing, left untouched:** `README.md` has substantial uncommitted local edits (a full front-matter rewrite) that predate this step and have nothing to do with Phase 12 — not staged or touched here.
**Files:** `clients/desktop-web/src/lib/sections/worlds/{model.ts,WorldsSection.svelte,WorldSlotCard.svelte,BackupsPanel.svelte,WorldRepairSheet.svelte,WorldConversionWizard.svelte}`, `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/tests/screens/worlds-backups.test.ts`, `docs/msc2/api-contract/openapi.json`
**What:** Wire the five P12.4a-e backends into their already-built client screens: real Chunker format discovery + full conversion flow, real operation-backed repair, real legacy-backup import, and a new thumbnail-upload affordance.
**Verify:** `npx svelte-check --tsconfig ./tsconfig.json` (only the same 7 pre-existing unrelated errors noted since P12.3f should remain), `npx prettier --check src/lib/sections/worlds src/lib/api/generated.ts tests/screens/worlds-backups.test.ts`, `npm run test:screen-worlds-backups` (16 tests), `npm run build`.
**Commit:** `P12.4f: connect the P12.4a-e backends to their frontends`
**Batch:** solo

### P12.4g — Make the API-type generator match the project's own Prettier config
**Status:** DONE. Resolves P12.4f's flagged tooling inconsistency, decided by Cameron: drop `generate.ts`'s own hardcoded `singleQuote: false` rather than accept double-quoted generated output project-wide. Replaced the hardcoded format-option object with `prettier.resolveConfig(outputPath)` (the project's real `prettier.config.js` — `singleQuote: true`, `printWidth: 100`, `trailingComma: 'all'`), spread ahead of the one setting the generator actually needs to force (`parser: 'typescript'`), so this can never silently drift from the rest of the repo's style again the way the hardcoded copy did since P11.29i. Regenerated `generated.ts` — byte-identical to what P12.4f had already hand-formatted with `prettier --write`, confirming the fix is correct with zero unrelated diff. `npm run api:generate -- --check` and `npx prettier --check` now both pass on the same file for the first time; `python3 tools/phase11/generated-types-check.py`'s own separate, pre-existing `OnboardingGuideDTO` regex bug (flagged in P12.4f, unrelated to quoting) is untouched.
**Files:** `clients/desktop-web/src/lib/api/generate.ts`, `clients/desktop-web/src/lib/api/generated.ts`
**What:** Stop hardcoding a driftable copy of the project's Prettier settings inside the API-type generator; resolve the real config instead.
**Verify:** `npm run api:generate -- --check && npx prettier --check src/lib/api/generate.ts src/lib/api/generated.ts && npx svelte-check --tsconfig ./tsconfig.json && npm run test:screen-worlds-backups`
**Commit:** `P12.4g: resolve the project's own prettier config in the API generator`
**Batch:** solo

### P12.4h — Fix a real WebKit-only layout bug on the World Slot card
**Status:** DONE. Cameron's live-app review found the World Slots grid's thumbnail area rendering as a narrow, content-width strip in the real Tauri desktop app (WKWebView) — full-width and correct in every Chromium check this session ran (isolated `WorldSlotCard`, full `WorldsSection` with matching real data, before *and* after clearing the app's WebKit cache), which is what pointed at a rendering-engine difference rather than stale code or cached assets. Root cause: `.thumb-area` (the clickable thumbnail+info region) is a `<button>` styled `display: flex; flex-direction: column`, and Safari/WebKit's native button appears to keep sizing its own box to its content even with `display: flex` set, so its flex children's `width: 100%` (the `.thumb` gradient and `.info` name/date block) got ignored — a real engine difference, not a Chromium quirk to work around blindly, and not something any amount of re-testing in a Chromium-based checker (Playwright, this session's own gallery checks) could have surfaced.

Fix: `appearance: none; -webkit-appearance: none;` on `.thumb-area` (strips whatever native sizing behavior WebKit was applying to the button), plus explicit `width: 100%; box-sizing: border-box;` on `.thumb-area`, `.thumb`, and `.info` so the layout no longer depends on flex-stretch resolving correctly at all — belt-and-braces once the actual mechanism was identified, not a guess. No other button in the app combines `display: flex` styling with a `width: 100%`-dependent child the way `.thumb-area` does, so this is the only place the bug could show up; not applied speculatively elsewhere.
**Files:** `clients/desktop-web/src/lib/sections/worlds/WorldSlotCard.svelte`
**What:** Fix the World Slot card thumbnail not filling its card width in the real Tauri (WebKit) app.
**Verify:** `npx svelte-check --tsconfig ./tsconfig.json` (only the 7 pre-existing unrelated errors), `npx prettier --check src/lib/sections/worlds/WorldSlotCard.svelte`, `npm run build`. Real verification is Cameron's own: reload the Tauri app and look at the Worlds tab.
**Commit:** `P12.4h: fix WebKit not stretching the world slot thumbnail to full width`
**Batch:** solo

### P12.4i — Fix uneven action-button widths, root-caused in the shared Button component
**Status:** DONE. Same WebKit-only family of bug as P12.4h, caught on the very next real-app look: each World Slot card's Activate/Convert and Rename/Delete pairs render at visibly different widths and don't line up into a clean 2×2 grid between the two rows, in the real Tauri app only. Root cause this time is one level lower than P12.4h — in `components/base/Button.svelte`'s own shared `.btn` class, not a local override: a flex item's `min-width` defaults to `auto` (its own content size), not `0`, so `WorldSlotCard.svelte`'s `.actions-row > .btn { flex: 1 }` (meant to split each row exactly 50/50) still lets Safari keep each button's native minimum-content floor — Chromium doesn't enforce that floor in practice, which is why this, like P12.4h, never showed up in this session's own Chromium-based checks. Since both rows share the same row width, an uneven intra-row split was also why the two rows didn't line up into a grid — fixing the split fixes both complaints from the one root cause, not two separate patches.

Fixed in the shared component, not locally in `WorldSlotCard.svelte`, since every other screen's buttons sit inside `flex: 1` layouts the same way and would eventually hit the identical bug: `Button.svelte`'s `.btn` gains `box-sizing: border-box`, `min-width: 0`, and the same `appearance: none` / `-webkit-appearance: none` P12.4h already established for the same class of native-control sizing quirk. `WorldSlotCard.svelte`'s own `.actions-row` flex targets also get `min-width: 0` directly, belt-and-braces on top of the shared fix.
**Files:** `clients/desktop-web/src/lib/components/base/Button.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldSlotCard.svelte`
**What:** Fix uneven Activate/Convert/Rename/Delete button widths on the World Slot card, at the shared Button component so every other flex-laid-out button in the app is protected too.
**Verify:** `npx svelte-check --tsconfig ./tsconfig.json` (only the 7 pre-existing unrelated errors), `npx prettier --check src/lib/components/base/Button.svelte src/lib/sections/worlds/WorldSlotCard.svelte`, `npm run build`, `npm run test:screen-worlds-backups`. Real verification is Cameron's own: reload the Tauri app and look at the Worlds tab.
**Commit:** `P12.4i: fix uneven button widths in the shared Button component`
**Batch:** solo

### P12.4j — P12.4i's fix wasn't enough; remove the nested-flex tooltip wrapper causing it
**Status:** DONE. P12.4i's `min-width: 0`/`appearance: none` fix did not resolve it — Cameron rebuilt and the four action buttons were still visibly uneven and not lined up into a 2×2 grid. The actual structural problem sat one layer up: `WorldSlotCard.svelte` wrapped Activate/Convert/Delete in a `<span class="hint" title="...">` purely to carry a tooltip, and that span was *both* a flex item of `.actions-row` (needing `flex: 1` from its parent) *and* its own flex container sizing its child button via `width: 100%` — a percentage width resolving against a size (the span's own flex-basis-derived width) that isn't necessarily settled yet. That two-hop indirection, not the button's own min-width floor, is what P12.4i's fix couldn't reach.

Removed the indirection instead of patching it again: `Button.svelte` gained a real `title` prop, set directly on the `<button>` element, so a tooltip no longer needs a wrapping element at all. `WorldSlotCard.svelte`'s four action buttons now pass `title` straight to `Button` and sit as plain, direct children of `.actions-row` — `flex: 1; min-width: 0` on `.actions-row > .btn` now has nothing between it and the actual button box. The `.hint`/`.hint :global(.btn)` rules and every `<span class="hint">` wrapper are gone from this file entirely, not just this one instance patched. Confirmed in Chromium the four buttons now measure equal (109–111px each, sub-2px apart — ordinary flex-basis rounding, not a real gap) with the exact real `campak` data shape; the real proof is still Cameron's own Tauri rebuild, same as P12.4h/i.
**Files:** `clients/desktop-web/src/lib/components/base/Button.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldSlotCard.svelte`
**What:** Give `Button` a real `title` prop and remove `WorldSlotCard.svelte`'s tooltip-only wrapper spans, eliminating the nested-flex structure P12.4i's fix couldn't reach.
**Verify:** `npx svelte-check --tsconfig ./tsconfig.json` (only the 7 pre-existing unrelated errors), `npx prettier --check src/lib/components/base/Button.svelte src/lib/sections/worlds/WorldSlotCard.svelte`, `npm run build`, `npm run test:screen-worlds-backups`. Real verification is Cameron's own: reload the Tauri app and look at the Worlds tab.
**Commit:** `P12.4j: remove the nested-flex tooltip wrapper that broke equal button widths`
**Batch:** solo

### P12.5 — Remove the Packs tab placeholder (deferred, not rebuilt)
**Status:** DONE
**Files:** `clients/desktop-web/src/lib/navigation/primaryTabs.ts`, `clients/desktop-web/tests/visual/shell.test.ts`
**What:** Cameron does not use MSC 1's Packs tab (`DetailsPacksTabView`/`ResourcePacksView` — per-server Resource Packs list, Add Pack / Clear Active Pack) and doesn't want it rebuilt or even shown greyed-out. This is a client-screen scope decision, not a removal of the underlying agent capability: `/v1/resourcepacks/*` and its capability-matrix rows are untouched, and the CLI keeps whatever access it already has. Deferred, not dropped — it can be added back as a new step later if wanted. Removed the `{ id: 'packs', label: 'Packs' }` entry from `PRIMARY_TABS` (the fixed MSC 1-mirroring tab list built in P11.8, ahead of each tab's own Phase 12 step populating it — Packs had no section registered yet, so it was rendering disabled rather than absent, as seen in Cameron's live-app screenshot). Updated the shell test's fixed tab-count assertion from 8 to 7 to match. The Phase 12 gate in `msc2-port-plan.md` is amended (2026-08-26) to name this as an explicit exception, per that document's own "intentional exceptions recorded rather than discovered" rule, so REVIEW does not flag its absence as a missed screen.
**Verify:** `npm run test:visual-shell` (6 tests), `npx svelte-check --tsconfig ./tsconfig.json` (only the same 7 pre-existing unrelated errors), `npx prettier --check src/lib/navigation/primaryTabs.ts tests/visual/shell.test.ts`, `npm run build`. Real verification is Cameron's own: reload the app and confirm Packs is gone from the tab strip.
**Commit:** `P12.5: remove the Packs tab placeholder`
**Batch:** solo

### P12.6 — Performance tab
**Status:** DONE. Rebuilt to the S0 disciplined system: 6-tile metrics grid (TPS or, for Bedrock, Load 1m/5m/15m + Players/CPU/Memory), TPS-Over-Time/CPU-Over-Time + Player-Activity charts, a collapsible Monitoring/Quick Actions/Health Summary rail, and a World Size/Uptime/Status footer — same zones as MSC 1 `DetailsPerformanceTabView`. MSC 1's per-tile colored status icon + tinted `strokeBorder` (a rule #3/#11 color-carrier) is replaced with the same StatusDot + text-label vocabulary `HealthGrid.svelte` already established; the tile's big value itself carries the tone color, which the design law explicitly allows as a "live-stat fill." Chart rainbow gradients (green→yellow→red line fills) are flattened to one flat status-toned line per rule #2/#5; per the `dataviz` skill's non-negotiable default, both charts got a hover crosshair+tooltip MSC 1's static Charts-framework versions never had.

**One real backend gap found and fixed, not faked or left blank, mirroring P12.4f's precedent:** `crates/msc-domain/src/tps.rs`'s `/tps` parser already extracts Paper's real 5m/15m rolling averages into `Sample{t1,t5,t15}`, and `msc-application`'s lifecycle layer already kept the full sample in `latest_tps` — but `PerformanceSnapshot`/`PerformanceSnapshotDTO` only ever exposed `t1`, silently dropping `t5`/`t15` before they reached the wire, which would have forced the TPS (5m avg)/(15m avg) tiles — two of this step's own six headline cards — to fake data or sit permanently blank. Added `tps5m`/`tps15m` to `docs/msc2/api-contract/openapi.json`, `PerformanceSnapshotDto` (`crates/msc-api/src/dto/status.rs`), and wired them through every construction site (`msc-application/src/lifecycle.rs`, `msc-agent/src/routes/{lifecycle,status,performance}.rs`), regenerating `generated.ts`. Additive, `Option`-typed, zero behavior change to any existing path; `cargo clippy -p msc-application -p msc-agent -p msc-api --tests` clean, `cargo nextest run -p msc-api --test dto_conformance` (11/11) and the two touched `msc-agent`/`msc-application` unit tests pass.

**No backend gap for Bedrock Load 1m/5m/15m or either "Over Time" chart:** MSC 1 itself derives these from client-side rolling windows over repeatedly-polled instantaneous values (`AppViewModel+BedrockPerformance.swift`'s `bedrockCpuHistory`/`rollingAverage`, `AppViewModel+OutputHandling.swift`'s `tpsHistory1m`/`playerCountHistory`), not distinct fields — reproduced the same way here, purely in `model.ts`, polling `/v1/performance` every 5s (matching the Monitoring rail's real "Active (5s)" label) and capping history at 180/30 samples respectively.

**One deliberate, documented departure from a literal port:** MSC 1 only sets `serverStartTime` when its own app process issues the Start action, because its server is a child process of the app itself — a client can never observe an already-running server it didn't start. MSC 2's agent is a persistent background service multiple clients reconnect to after the fact (Phase 9/11), so porting that guard literally would show "Offline" in the Uptime tile while the adjacent Status tile says "Online." Instead, uptime is tracked from the moment *this* client session observes a real not-running→running transition; if the server was already running on first load, the tile reads "Running" (no fabricated duration) rather than "Offline" or a guessed number.

**One MSC1→MSC2 architecture mismatch adapted, not ported stale:** MSC 1's Bedrock CPU/Memory subtitles name its own Docker/VM backend choice ("Docker container", "Virtual machine"); MSC 2's actual Bedrock runtime model (Phase 10) is `native`/`vz-sidecar`, not Docker/VM, so those exact subtitle strings would be describing a backend that no longer exists. Replaced with backend-neutral phrasing ("Bedrock runtime", "Bedrock runtime active").

**Reused as-is, not restyled:** the footer's World Size/Uptime/Status tiles use the same `MetricTile` component as the top row (one card language, per the antiAIslop checklist) rather than MSC 1's separate `compactInfoTile` (a horizontal icon-left layout with a colored — blue — informational icon, itself a rule #6 tell this rebuild intentionally didn't reproduce). "Explain Metrics" reuses the already-built `HelpLink.svelte` (P11.16, pointed at the real `handbook.ram-performance` topic) rather than inventing new navigation; it still carries its original P11-era `--msc-accent` styling since no rebuilt Phase 12 screen has restyled it yet — deferred to P12.16 (Guides/Handbook), not fixed here.

Verified by mounting the component standalone (Playwright, hand-built fake API data, both Java and Bedrock variants) since this sandbox has no live agent a plain browser can reach — no console/runtime errors, layout matches the checklist. Cameron's own real-app look is still the authoritative check.
**Files:** `clients/desktop-web/src/lib/sections/performance/{PerformanceSection.svelte,MetricTile.svelte,PerformanceChart.svelte,MonitoringRail.svelte,model.ts}`, `clients/desktop-web/src/lib/api/generated.ts`, `docs/msc2/api-contract/openapi.json`, `crates/msc-application/src/{status.rs,lifecycle.rs}`, `crates/msc-agent/src/routes/{lifecycle.rs,status.rs,performance.rs}`, `crates/msc-api/src/dto/status.rs`, `crates/msc-api/tests/dto_conformance.rs`
**What:** Rebuild Performance — TPS 1m/5m/15m + Players/CPU/Memory cards, TPS-Over-Time and Player-Activity charts, right-hand Monitoring/Quick Actions/Health Summary rail, World Size/Uptime/Status footer. Charts follow the `dataviz` discipline and the anti-slop color budget. Reference MSC 1 `DetailsPerformanceTabView`/`Content` and the Performance screenshot.
**Verify:** `npm run dev`, open Performance running; compare to MSC 1 + checklist. Structural: `npx svelte-check --tsconfig ./tsconfig.json` (only the same pre-existing unrelated errors), `npx prettier --check src/lib/sections/performance src/lib/api/generated.ts`, `npm run build`. Backend: `cargo clippy -p msc-application -p msc-agent -p msc-api --tests`, `cargo nextest run -p msc-api --test dto_conformance`, `cargo nextest run -p msc-agent --bin msc -E 'test(status_performance)'`.
**Commit:** `P12.6: rebuild the Performance tab`
**Batch:** solo

### P12.7 — Components tab (+ plugin browser)
**Status:** awaiting verification. Rebuilt to the S0 disciplined system: a Server/Loader row (name, badge, StatusDot from the agent's own isUpToDate/note, a shared version-picker sheet), a Plugins/Mods list (toggle/version/update/remove rows, Update All, Add Plugin from a local JAR, an honestly-disabled Reveal Folder, empty state), and a Crossplay row (MCXboxBroadcast enable toggle, install status, download) — plus, for Bedrock, a Runtime card (same version-picker) and a Broadcast card (running-state dot, download). The plugin browser (search + Add) and a CurseForge manual-file sheet (D-027) are separate sheets, matching `ModrinthBrowserView`/`CurseForgeManualDownloadSheet`'s split. Per rule #6, MSC 1's colored icon-in-tinted-box per row is dropped -- rows carry name + badge + StatusDot(+label) only, no per-row icon.

**One real interface gap found and fixed, small and additive:** `ScreenApi.upload` only ever took `(purpose, bytes)` -- fine for every purpose used so far, but `curseforge-manual-file` needs `operationId`/`fileId` threaded to the agent's `POST /v1/staged-uploads` too (`StagedUploadBeginRequestDTO` already carries both). Added an optional third `options` parameter to `ScreenApi.upload` (`shared/types.ts`) and forwarded it in `App.svelte`'s real implementation (`stagedUpload({ purpose, ...options }, bytes)`); every existing 2-arg call site (`TransferPanel.svelte`, `WorldSlotCard.svelte`) is untouched and still compiles.

**Two real, pre-existing backend gaps found, left alone (crates/ wasn't this step's scope), recorded in the file's own header comment:** `GET /v1/components` hardcodes `is_up_to_date`/`updatable` to `true` for the primary server-jar row (no real online-build check yet, unlike the genuinely real `GET /v1/versions`) -- the Server JAR row renders whatever the agent reports, honestly, so it will always read "Up to date" until that route gets a real check. And `AddonItemDTO` has no MSC-1-style tier (managed/userSourced/unmanaged) -- only `bucket` (the resolver's *update* bucket: updateAvailable/noCompatibleVersion/upToDate/unlinked, not a category) -- so Geyser/Floodgate render like any other plugin, no "Managed" badge; the contract has nothing to badge them with. Also found and fixed in passing: `addons/model.ts`'s `demoAddons` fixtures used invented `bucket` values (`'mod'`/`'component'`) that were never real category strings the agent emits -- corrected to the real four-value enum.

**Two MSC 1 affordances deliberately out of scope, not half-built:** "Export for clients" (`ClientExportResponseDTO.stagedDownloadId` needs a save-to-disk platform primitive no section has ever used) and per-plugin manual source-linking (`PluginSourcePopover`) -- neither is named in this step's own line.

**One MSC 1 page with no contract support at all, not faked:** `ModrinthProjectDetailView`'s gallery/full-version-list/per-version-install has no backing route -- `GET /v1/catalog/search` returns search hits only, and install always resolves "the latest compatible version" server-side. The browser sheet here is search-results-only, matching what `AddonsSection.svelte` (P11.13) already assumed.

Verified structurally (`svelte-check`, `prettier`, `npm run build`, `test:screen-addons` all clean/passing); this sandbox has no live agent for a real `npm run dev` look. Cameron's own real-app look, plus the anti-slop checklist, is still the authoritative check -- including whether the honestly-disabled "Reveal folder" button and the search-only plugin browser read as acceptable simplifications rather than gaps.
**Files:** `clients/desktop-web/src/lib/sections/components/{ComponentsSection.svelte,model.ts,VersionPickerSheet.svelte,PluginBrowserSheet.svelte,ImportModpackSheet.svelte,CurseForgeManualDownloadSheet.svelte}`, `clients/desktop-web/src/lib/sections/addons/model.ts`, `clients/desktop-web/src/lib/sections/shared/types.ts`, `clients/desktop-web/src/App.svelte`, `clients/desktop-web/tests/screens/addons.test.ts`
**What:** Rebuild Components — Server JAR row (version, Up to date/update), Plugins (list, Add Plugin, Reveal folder, empty state), Crossplay (Broadcast, Missing/status). Include the plugin browser (Modrinth/CurseForge) as a sheet. Reference MSC 1 `DetailsComponentsTabView`, `ModrinthBrowserView`, `CurseForgeManualDownloadSheet`, and the Components screenshot.
**Verify:** `npm run dev`, open Components + browser; compare to MSC 1 + checklist. Structural: `npm run test:screen-addons`.
**Commit:** `P12.7: rebuild the Components tab`
**Batch:** solo

### P12.7a — Give the plugin browser real Modrinth search data
**Status:** DONE. Cameron's live-app review of the plugin browser sheet found every result showing `by ·` (blank author) and `0 downloads`, with no icon -- despite the search hits themselves being real (same titles, same order as MSC 1's own browser against the same query). Traced to two layers in `crates/`, not a client bug: `msc_domain::addon_provider::ModrinthSearchHit` (the struct decoding Modrinth's real search response) only ever captured `project_id`/`slug`/`title`/`server_side`, silently dropping `author`/`downloads`/`description`/`icon_url` even though `CatalogItemDTO` already has fields for all four and Modrinth's wire response already includes them (confirmed against the real captured fixture `corpus/addons/modrinth/search-sodium.json`, which has real values for every one of those fields). Then `crates/msc-agent/src/routes/components.rs`'s `GET /v1/catalog/search` handler hardcoded `description: String::new()`, `author: String::new()`, `downloads: 0`, `icon_url: None` regardless, discarding the fields a second time even if the struct had captured them.

Fixed at both layers: widened `ModrinthSearchHit` with `description`/`author`/`downloads`/`icon_url` (`#[serde(default)]`, matching Modrinth's own snake_case wire names, no contract change needed since `CatalogItemDTO` already modeled all four), then mapped them through in the route handler instead of hardcoding. `ModrinthSearchHit` gained `Default` so the three existing `find_confident_dependency_match` test fixtures (`crates/msc-application/tests/addon_updates.rs`) could collapse their literals to `..Default::default()` rather than hand-filling four new fields those tests don't care about.

Also fixed the client-side half of the same finding: `PluginBrowserSheet.svelte` never rendered `item.iconURL` at all -- an oversight from being overly cautious about rule #6 (colored icon-in-tinted-box on an *informational* element). A real per-project thumbnail image is content, the same category as a World Slot's thumbnail, not the tell that rule targets; added a 40×40 icon (real image, or a flat neutral placeholder tile matching `ModrinthBrowserView.swift`'s own `Color.secondary.opacity(0.15)` fallback) ahead of each result's text.

Full per-project detail page (gallery, About, links) and per-version browsing/install remain the documented, out-of-scope gap from P12.7 itself -- no route exists for either, and this step didn't add one.
**Files:** `crates/msc-domain/src/addon_provider.rs`, `crates/msc-agent/src/routes/components.rs`, `crates/msc-application/tests/addon_updates.rs`, `clients/desktop-web/src/lib/sections/components/PluginBrowserSheet.svelte`
**What:** Widen `ModrinthSearchHit` and map its real fields through `GET /v1/catalog/search` instead of hardcoding empty/zero defaults; render the real per-project icon in the plugin browser sheet.
**Verify:** `cargo clippy -p msc-domain -p msc-agent -p msc-application --tests` clean; `cargo nextest run -p msc-domain --test addon_providers` (33/33), `cargo nextest run -p msc-application --test addon_updates` (23/23), `cargo nextest run -p msc-agent --test phase8_routes` (2/2). Client: `npx svelte-check --tsconfig ./tsconfig.json` (only the same 7 pre-existing unrelated errors), `npx prettier --check src/lib/sections/components/PluginBrowserSheet.svelte`, `npm run test:screen-addons` (9/9), `npm run build`. Real verification is Cameron's own: reopen the plugin browser and look for real author/download counts and icons.
**Commit:** `P12.7a: give the plugin browser real Modrinth search data`
**Batch:** solo

### P12.7b — Fix a real acronym-casing bug dropping every iconURL over the wire
**Status:** DONE. After rebuilding and restarting the agent per P12.7a, Cameron reported author/downloads/description now showed real values but every icon was still the neutral placeholder -- "showing as intended, which is incomplete." Root cause was one layer deeper than P12.7a: `#[serde(rename_all = "camelCase")]` turns Rust's `icon_url` into wire key `iconUrl`, not `iconURL` -- serde's camelCase conversion has no concept of an acronym staying uppercase. The frozen contract spells it `iconURL` (matching MSC 1's Swift field name), and the generated TypeScript types read `item.iconURL` accordingly, so the agent was sending a field the client's real code never looked for -- silently dropped, no error, exactly the "quietly missing" symptom Cameron saw. Confirmed against the *live* Modrinth API (not just the fixture) with a real `curl` that the real response really does include `icon_url` with a real CDN URL for every one of these plugins, so this was never a "no icon available" case.

Same bug, same file (`crates/msc-api/src/dto/addons.rs`), in three places: `AddonItemDto.icon_url`, `CatalogItemDto.icon_url`, and `ClientExportItemDto.icon_url`/`project_url` (the contract's `iconURL`/`projectURL` acronym spelling both need it). Fixed each with an explicit `#[serde(rename = "iconURL", ...)]` override on top of the struct's `rename_all` default -- the same precedented pattern already used once in this codebase (`crates/msc-api/src/dto/networking.rs`'s `linkURL`). A broader contract audit found this exact acronym pattern (`URL`, `ID`, `RAM`, `ISO8601`, `MC`) on a dozen-plus other fields across the contract, but none of those are reachable from any code this phase has touched, so they're out of scope here -- flagged, not chased down speculatively.

Added three inline unit tests in `crates/msc-api/src/dto/addons.rs` (`#[cfg(test)] mod tests`) asserting the exact serialized key names for all three affected structs, since the existing `dto_conformance.rs` explicitly scopes itself to five Phase-8-era schemas and doesn't cover this file at all -- extending that file's stated scope felt like a separate, bigger decision than this bug-fix, so new coverage lives locally instead.
**Files:** `crates/msc-api/src/dto/addons.rs`
**What:** Add explicit `#[serde(rename = "iconURL"/"projectURL", ...)]` overrides so these fields serialize with the contract's real acronym-cased wire names instead of serde's auto-lowercased default; add regression tests.
**Verify:** `cargo test -p msc-api --lib dto::addons::tests::` (3/3), `cargo nextest run -p msc-api --test dto_conformance` (11/11, unaffected), `cargo nextest run -p msc-agent --test phase8_routes` (2/2, unaffected), `cargo clippy -p msc-domain -p msc-agent -p msc-application -p msc-api --tests` clean, `cargo fmt --check` clean. Real verification is Cameron's own: rebuild+restart the agent again, reopen the plugin browser, confirm real icons now render.
**Commit:** `P12.7b: fix iconURL/projectURL serializing without their acronym casing`
**Batch:** solo

### P12.7c — Contract + backend: Modrinth project detail and version list
**Status:** DONE
**Files:** `docs/msc2/api-contract/openapi.json`, `tools/api-contract-check.py`, `crates/msc-domain/src/addon_provider.rs`, `crates/msc-agent/src/routes/components.rs`, `crates/msc-application/src/addons.rs` (or wherever `install_from_catalog` actually lives — confirm before editing), `crates/msc-api/src/dto/addons.rs`, `crates/msc-agent/src/cli/mod.rs`, `crates/msc-application/tests/addon_dependencies.rs` (approved compile-callsite updates)
**What:** Cameron reviewed MSC 1's plugin browser (`ModrinthProjectDetailView.swift`, screenshotted directly) against MSC 2's and asked for parity: tapping a search result opens a detail page with a gallery, the project's full "About" text, and a per-version list scoped to compatibility with the active server, with a specific version installable (not just "latest"). P12.7/P12.7a/P12.7b already documented this as a real, undone gap — no route exists for a single project's detail or version list. This step closes that gap on the backend + contract side only; the client (P12.7d) is a separate step so Codex and Claude Code can work it without colliding.

Port two more `ModrinthAPI` functions (`ModrinthAPI.swift`) plus the version-picking half of `installVersion`/`installModrinthVersion` (`ModrinthBrowserView.swift` lines 296-311, 768-784):

1. **`GET /v1/catalog/projects/:projectId`** — ports `ModrinthAPI.project(idOrSlug:)` (calls Modrinth's real `GET /v2/project/{id}`). Response schema `CatalogProjectDetailDTO`:
   ```json
   "CatalogProjectDetailDTO": {
     "type": "object",
     "properties": {
       "projectId": { "type": "string" },
       "slug": { "type": "string" },
       "title": { "type": "string" },
       "description": { "type": "string" },
       "body": { "type": "string", "description": "Full Markdown/HTML project body (Modrinth's 'body' field). Client is responsible for safely rendering it -- this is untrusted third-party content." },
       "iconURL": { "type": "string", "nullable": true },
       "downloads": { "type": "integer" },
       "followers": { "type": "integer" },
       "serverSide": { "type": "string", "description": "One of required/optional/unsupported, Modrinth's own vocabulary -- unchanged, not remapped to a boolean." },
       "gallery": { "type": "array", "items": { "$ref": "#/components/schemas/CatalogGalleryImageDTO" } },
       "sourceURL": { "type": "string", "nullable": true },
       "issuesURL": { "type": "string", "nullable": true },
       "wikiURL": { "type": "string", "nullable": true },
       "discordURL": { "type": "string", "nullable": true }
     },
     "required": ["projectId", "slug", "title", "description", "body", "downloads", "followers", "serverSide", "gallery"]
   }
   "CatalogGalleryImageDTO": {
     "type": "object",
     "properties": {
       "url": { "type": "string" },
       "title": { "type": "string", "nullable": true },
       "description": { "type": "string", "nullable": true },
       "featured": { "type": "boolean" }
     },
     "required": ["url", "featured"]
   }
   ```
   **P12.7b already burned us once on exactly this:** every `*URL` field above (`iconURL`, `sourceURL`, `issuesURL`, `wikiURL`, `discordURL`) needs its own explicit `#[serde(rename = "...URL", ...)]` override in the Rust DTO — `#[serde(rename_all = "camelCase")]`'s automatic conversion produces `sourceUrl`, not `sourceURL`, and the client will silently read `undefined` again if any one of these five is missed. Add the same inline-test pattern P12.7b used (`#[cfg(test)] mod tests`) asserting the real serialized key name for all five.

   Extend `msc_domain::addon_provider` with a new struct mirroring Swift's `ModrinthProject` (id, slug, title, description, body, icon_url, downloads, followers, server_side, gallery: `Vec<ModrinthGalleryImage>` [url, title, description, featured], source_url, issues_url, wiki_url, discord_url) and a decode function alongside the existing `ModrinthProjectSummary`/`modrinth_decode_project` (P8.15) — don't widen or repurpose that existing struct, it's a distinct, narrower call site (`installRequiredDependencies`'s dependency resolution) that only ever needed `id`/`slug`/`title`.

2. **`GET /v1/catalog/projects/:projectId/versions`** — ports `ModrinthAPI.projectVersions(idOrSlug:loaders:gameVersion:)` called **unfiltered** (`loaders: [], gameVersion: nil`, `ModrinthProjectDetailView.load()` line 753) — the detail page fetches every version and lets the client compute compatibility/filtering, it does not ask Modrinth to pre-filter. Response schema `CatalogVersionsResponseDTO { "versions": CatalogVersionDTO[] }`, each item:
   ```json
   "CatalogVersionDTO": {
     "type": "object",
     "properties": {
       "id": { "type": "string" },
       "projectId": { "type": "string" },
       "name": { "type": "string" },
       "versionNumber": { "type": "string" },
       "versionType": { "type": "string", "description": "release/beta/alpha" },
       "gameVersions": { "type": "array", "items": { "type": "string" } },
       "loaders": { "type": "array", "items": { "type": "string" } },
       "datePublished": { "type": "string", "nullable": true },
       "dependencies": { "type": "array", "items": { "$ref": "#/components/schemas/CatalogVersionDependencyDTO" } },
       "files": { "type": "array", "items": { "$ref": "#/components/schemas/CatalogVersionFileDTO" } }
     },
     "required": ["id", "projectId", "name", "versionNumber", "versionType", "gameVersions", "loaders", "dependencies", "files"]
   }
   "CatalogVersionDependencyDTO": {
     "type": "object",
     "properties": {
       "projectId": { "type": "string", "nullable": true },
       "versionId": { "type": "string", "nullable": true },
       "dependencyType": { "type": "string", "description": "required/optional/incompatible/embedded" }
     },
     "required": ["dependencyType"]
   }
   "CatalogVersionFileDTO": {
     "type": "object",
     "properties": {
       "url": { "type": "string" },
       "filename": { "type": "string" },
       "primary": { "type": "boolean" },
       "size": { "type": "integer", "nullable": true }
     },
     "required": ["url", "filename", "primary"]
   }
   ```
   `projectId`/`versionId`/`dependencyType` are already plain camelCase with no acronym — `rename_all = "camelCase"` gets these right automatically, no override needed (confirmed: this is the same shape `CatalogItemDTO.projectId` already uses correctly). Widen the existing `msc_domain::addon_provider::ModrinthVersionInfo` (already used by `modrinth_decode_project_versions`, P8.15) with the fields the version-list UI needs that update/dependency-resolution never did: `name`, `version_type`, `game_versions: Vec<String>`, `loaders: Vec<String>`, `date_published: Option<String>` — all `#[serde(default)]` so the two existing call sites (hash-based update checking, dependency resolution) are unaffected. Add `pub fn is_stable(&self) -> bool` (`version_type == "release"`, mirrors `ModrinthVersionInfo.isStable`).

3. **Install a specific version, not just latest** — ports `installVersion`/`viewModel.installModrinthVersion(_:title:into:)` (`ModrinthBrowserView.swift:768-784`). Add one optional field to the existing frozen `CatalogInstallRequestDTO`:
   ```json
   "versionId": { "type": "string", "description": "Install this exact Modrinth version instead of resolving the latest compatible one server-side. When present, projectId/slug/title are still used for the install-result message and staging metadata; the version is fetched directly by id (GET /v2/version/{id}) rather than searched for." }
   ```
   This is additive and optional — every existing caller (the plain "Add" button, which never sets it) keeps today's latest-resolution behavior unchanged. Add `modrinth_decode_version(body: &str) -> Result<ModrinthVersionInfo, AddonProviderError>` to `addon_provider.rs` (same shape as `modrinth_decode_project`, decoding a lone `ModrinthVersionInfo` rather than an array), wire `install_component`'s handler to call Modrinth's `GET /v2/version/{id}` and use that decoded version directly when `version_id` is present, and extend whichever `msc-application` function actually resolves-then-installs (confirm the real name/location first; `components.rs` calls it as `addons::install_from_catalog`) to accept an already-resolved version and skip its own latest-lookup in that case.

Bump `EXPECTED_TOTAL` in `tools/api-contract-check.py` by 2 (the two new GET routes; the `CatalogInstallRequestDTO` field is additive to an existing route, not a new one) and append one clause to its running-total comment, matching every prior entry there.
**Verify:** `python3 tools/api-contract-check.py`; `cargo nextest run -p msc-domain --test addon_providers`; `cargo test -p msc-api --lib dto::addons::tests::` (existing 3 plus the new acronym-casing tests for this step's 5 new `*URL` fields); `cargo nextest run -p msc-agent --test phase8_routes`; `cargo clippy -p msc-domain -p msc-agent -p msc-application -p msc-api --tests` clean; `cargo fmt --check` clean.
**Commit:** `P12.7c: add Modrinth project-detail and version-list routes`
**Batch:** solo

### P12.7d — Modrinth-style project detail page in the plugin browser
**Status:** DONE. Built as planned, with one real gap found along the way: `ServerDTO` has no Minecraft-version field at all (the client never independently knew the active server's MC version -- the old search flow only ever displayed whatever `CatalogSearchResponseDTO.gameVersion` echoed back from a server-computed search). The version-compatibility banner and per-version Compatible/Other badges both need that value client-side now, since the detail page fetches every version unfiltered and compares locally rather than asking Modrinth to pre-filter. Found it already flows through `GET /v1/components` -- `ComponentsSection`'s existing `primaryComponent` (the row matching the active server's `javaFlavor`) carries it as `installedVersion` (`server.minecraft_version.clone()`, `components.rs`'s `component_rows`) -- so no contract change was needed, just threading `activeServer?.javaFlavor` and `primaryComponent?.installedVersion` down through `PluginBrowserSheet` into the new sheet as two new props.

Ported `expandedLoaders`/`collapsedVersions`/`visibleVersions` (the loader-compatibility, cross-platform-build-collapsing, and stable-only-with-fallback logic) as pure, unit-tested functions in `model.ts` rather than inline component state, plus a client-side mirror of `JavaServerFlavor.modrinth_loader_facets` (`identity.rs:216-227`) since the version list is fetched unfiltered and needs the same loader facets the backend already applies to search. The About body is rendered through a small sanitizer + inline-segment parser (text/bold/link only) that never touches `{@html}` -- untrusted third-party Markdown/HTML from Modrinth is reduced to a safe subset before any DOM node is created, matching MSC 1's own reasoning for hand-rolling `sanitizedBodyMarkdown` instead of pulling in a full renderer. Each search result row is now a clickable button (icon+title+author+description) with the "Add" action kept as a separate sibling button, so installing straight from the flat list still works without opening the detail page.
**Files:** `clients/desktop-web/src/lib/sections/components/PluginBrowserSheet.svelte`, `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte` (new), `clients/desktop-web/src/lib/sections/components/model.ts`, `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte` (thread `javaFlavor`/`serverMinecraftVersion` props through), `clients/desktop-web/src/lib/api/generated.ts` (regenerated via `npm run api:generate`, not hand-edited), `clients/desktop-web/tests/screens/addons.test.ts`
**What:** Depends on P12.7c landing first (needs the two new routes + `versionId` field to exist in the generated types). Ports `ModrinthProjectDetailView` (`ModrinthBrowserView.swift:316-785`): each search result in `PluginBrowserSheet` becomes clickable (not just its "Add" button) and opens a new detail sheet with:
- Header: icon, title, author, downloads, followers, a server-side badge (Server-side required/optional, or Client-only — same three-way switch as `serverSideBadge`, line 645-653), "View on Modrinth" link (`https://modrinth.com/{projectType}/{slug}`)
- A compatibility summary banner: green "a version is available for your server" when any fetched version's `gameVersions` includes the server's Minecraft version, amber "no version yet for MC {x}, install anyway at your own risk" otherwise (`compatibilitySummary`, line 452-465)
- Gallery strip (only if `gallery` is non-empty) — horizontally scrolling images
- About section — the project `body`. **Decided, not asked (doesn't change what Cameron sees either way):** mirror MSC 1's own `sanitizedBodyMarkdown` approach (line 681-730) rather than pulling in a Markdown-rendering dependency — a small local sanitizer that strips iframes/scripts/tables/images entirely, converts `<a href>`/Markdown links to safe inline links, headers to bold, and decodes the handful of HTML entities MSC 1 already handles. This keeps the About section dependency-free and safe by construction (untrusted third-party text is never handed to `{@html}`), matching the oracle's own reasoning for why it hand-rolled this instead of using a full Markdown renderer.
- Versions section with a "Stable only" toggle (defaulting on, but auto-off if the project has zero release-channel versions — line 762-764), each version row showing version number, channel badge (release/beta/alpha), Compatible/Other version badge, a conflict-count warning when any dependency has `dependencyType: "incompatible"`, and an expandable detail (full supported-MC-version list with the server's version highlighted, plus platforms) — ports `versionRow`/`versionExpandedDetail`/`FlowVersionTags` (line 525-597, 787-808). Per rule #6, the FlowVersionTags chip highlight (bold + green for the matching version) is fine to keep — it's compatibility-status color on data, not a decorative icon-in-tinted-box.
- Per-version Install/"Install anyway" button, using the new `versionId` field on the existing install request — replaces the flat list's single "Add" as the way to pick a non-latest build.
**Verify:** `npx svelte-check --tsconfig ./tsconfig.json` (no new errors beyond the pre-existing baseline); `npx prettier --check` on the new/changed files; `npm run test:screen-addons`; `npm run build`. Real verification is Cameron's own: `npm run dev`, open a plugin's detail page, compare gallery/About/version list against MSC 1's screenshots.
**Commit:** `P12.7d: build the Modrinth project detail page`
**Batch:** solo

### P12.7e — Fix a real gap: installed plugins never got a current version
**Status:** DONE. Cameron installed VeinMiner through the just-fixed plugin browser and its row showed "Unknown version" for the current-version subtitle. Traced to a genuine, pre-existing gap in `msc-application::addon_updates::resolve_addon_updates` (Phase 8, not new to P12.7d) rather than anything in the detail-page work itself: `AddonUpdateItem` never carried a current-version field at all, and `crates/msc-agent/src/routes/components.rs`'s `get_addons` hardcoded `current_version: None` in the DTO regardless. The oracle (`AddonUpdateResolver.swift:204,263`) computes this as `idVersion?.versionNumber ?? PluginNameParser.extractVersion(from: file.jarStem)` — prefer the hash-identified Modrinth version's own number, fall back to a filename/manifest-parsed guess. Both halves of that were *already implemented and sitting unused* in this codebase: the hash-identified version (`fresh`, from the same `modrinth_versions_from_hashes` call `resolve_bucket` already consumes) and the filename/manifest-parsed version (`ModEntry.version`/`PluginEntry.version`, already computed by `add_on_inventory::scan_mods`/`scan_plugins` and already used for other purposes) — neither was ever plumbed into `AddonUpdateItem` or the DTO. No contract change was needed; `AddonItemDTO.currentVersion` already existed in the frozen contract and the client already renders it (`ComponentsSection.svelte`'s `addon.currentVersion ?? 'Unknown version'` fallback, which is exactly what was showing).

Fixed by threading both existing values through: added `DiskEntry.version` (from `ModEntry`/`PluginEntry.version`) and `AddonUpdateItem.current_version` (computed as `fresh.map(|v| clean_version_label(&v.version_number)).or_else(|| entry.version.clone())`, matching the oracle's exact precedence), then mapped it into `AddonItemDto.current_version` instead of the hardcoded `None`.
**Files:** `crates/msc-application/src/addon_updates.rs`, `crates/msc-agent/src/routes/components.rs`, `crates/msc-application/tests/addons.rs`
**What:** Add `current_version` to `DiskEntry`/`AddonUpdateItem`, populate it (hash-identified version number, else filename/manifest fallback), map it into the existing `AddonItemDTO.currentVersion` field instead of discarding it.
**Verify:** `cargo clippy -p msc-domain -p msc-agent -p msc-application -p msc-api --tests` clean; `cargo fmt --check` clean; `cargo nextest run -p msc-application --test addons` (20/20); `cargo nextest run -p msc-application --test addon_updates` (23/23); `cargo nextest run -p msc-agent --test phase8_routes` (2/2). Real verification is Cameron's own: rebuild+redeploy the agent, reinstall or refresh a plugin, confirm its row shows a real version instead of "Unknown version".
**Commit:** `P12.7e: give installed add-ons a real current version instead of always Unknown`
**Batch:** solo

### P12.7f — Replace the persistent Remove button with a click-anywhere action menu
**Status:** DONE. Cameron's own preference, not an oracle port — MSC 1's plugin row (`DetailsComponentsTabView.swift:1001-1078`) uses the same always-visible Toggle + Remove-button-with-inline-confirm shape MSC 2 already had; Cameron asked for something MSC 1 doesn't do at all: click the row (or its name/version text) and get a small menu, positioned where clicked, offering Enable/Disable, View, and Uninstall.

Built a new base primitive, `Menu.svelte` (flat opaque surface, no blur -- `antiAIslop.md` #4 reserves blur for a true modal scrim, not a lightweight popover -- clamped to stay inside the viewport since the anchor point can be anywhere on screen). The plugin row's Toggle and always-visible Remove button are gone; the row's name/version/status-dot area is now a button that opens the menu at the click position. Enable/Disable calls the same `toggleAddon`; Uninstall reuses the row's existing inline "Uninstall? / Cancel / Uninstall" confirm swap unchanged, just re-triggered from the menu instead of a dedicated button. The "Update" button (only shown when `bucket === 'updateAvailable'`) stays exactly where it was, outside the clickable area, since Cameron's four named actions didn't include it.

"View" opens `ProjectDetailSheet` (P12.7d) for the installed add-on — the real complication, since `AddonItemDTO` carries only `projectId`/`displayName`/`iconURL`, not the `author`/`slug`/`downloads`/`description`/`projectType` a `CatalogItemDTO` search hit has. Rather than a contract change, relaxed `ProjectDetailSheet`'s `item` prop from `CatalogItemDTO` to a new, smaller `ProjectDetailItem` type (`projectId`+`title` required, everything else optional) — the sheet already fetches the full project detail itself on mount, so the header's byline/stats lines now degrade gracefully (omit the byline with no author, show nothing until the real download/follower counts load rather than a wrong placeholder), and the "View on Modrinth" link falls back to Modrinth's generic `/project/{slug-or-id}` path when `projectType` isn't known upfront. "View" is disabled in the menu when an add-on has no `projectId` (never identified against Modrinth).

After seeing it live, Cameron asked for one more thing: a blue border on the row whose menu is open, so it reads as "selected." Added `--msc2-selection` (`tokens.css`) as its own dedicated blue rather than reusing `--msc2-status-bedrock` -- that token already carries a specific status meaning (Bedrock edition), and giving one hue two meanings is exactly what the color-budget rule in `antiAIslop.md` #1 guards against. Applied as an inset `box-shadow` (not a real `border`) so the row's box size doesn't shift against its neighbors, plus a faint matching background tint.

Then: the row being clickable at all wasn't obvious without a hover. Added a small quiet chevron next to the plugin name (`--msc2-text-tertiary`, not colored) -- the exact same affordance `ModrinthBrowserView.swift`'s own row already uses next to its title, and the one already ported into `PluginBrowserSheet`'s search results (P12.7d) for the same reason. Reusing an already-established, oracle-precedented signal rather than inventing a new one (a kebab/dots icon, a hover-only tint) keeps this consistent instead of adding a second "this is clickable" vocabulary.
**Files:** `clients/desktop-web/src/lib/components/base/Menu.svelte` (new), `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`, `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte`, `clients/desktop-web/src/lib/sections/components/model.ts`, `clients/desktop-web/src/lib/styles/tokens.css`
**What:** New `Menu` primitive; plugin row becomes click-to-menu (Enable/Disable, View, Uninstall) instead of a persistent Toggle + Remove button; `ProjectDetailSheet` accepts a minimal `ProjectDetailItem` so "View" works from an installed add-on, not just a search hit.
**Verify:** `npx svelte-check --tsconfig ./tsconfig.json` (only the same 7 pre-existing unrelated errors), `npx prettier --check`, `npm run test:screen-addons` (20/20), `npm run build`. Real verification is Cameron's own: click a plugin row, confirm the menu appears at the click point with working Enable/Disable, View (opens the detail sheet), and Uninstall (inline confirm).
**Commit:** `P12.7f: replace the persistent Remove button with a click-anywhere action menu`
**Batch:** solo

### P12.8 — Settings tab
**Status:** awaiting verification. Rebuilt to the S0 disciplined system: a schema-driven World/Server/Network section list (`GET /v1/settings`'s `sections`/`fields`, already generic rather than a closed Java/Bedrock property list), edited as a local draft that only leaves the browser on Save — matching MSC 1's "changes stay local until you click Save Changes" model exactly, with a `StatusDot(warn, "Unsaved changes")` replacing MSC 1's orange dot+text badge. Field-to-control mapping is driven by the DTO's own `type`: `bool` → Toggle, `int` → NumberField (+ its `unit` shown alongside), `string` → Field, `enum` → SegmentedControl for `difficulty`/`gamemode` (matching MSC 1's `.pickerStyle(.segmented)` choice exactly) and Select for the rest (`level-type`, `op-permission-level`), same split MSC 1 makes despite all four being `enum`-shaped. Per rule #9/#6, MSC 1's per-section SF Symbol icon and descriptive caption sentences are dropped — section headers are plain overline text (matching `ComponentsSection`'s zone-header convention, itself already reviewed) and the one gear icon lives only in the tab's own top header, added to the shared `Icon` set. `NumberField` (an until-now-unused S0 primitive) gained a controlled `onchange` prop, matching `Select`'s existing shape, since a dynamically-keyed settings draft can't cleanly use its old bind-only wiring — its first real consumer.

Confirmed against `DetailsSettingsTabView.swift`/`ServerSettingsView.swift` that MSC 1's Java executable path / RAM / Geyser-listener rows belong to a *different* screen entirely — the app-level "MSC Settings" sheet (`MSCSettingsSections.swift`, General/Remote/Data tabs), not this per-server tab. The previous version of this file conflated the two (wiring `/v1/config/java-runtime`, `/v1/config/ram`, `/v1/config/geyser` in here); this rebuild drops that mix-in rather than carrying it forward — that functionality is P12.14's scope, not built anywhere yet. One pre-existing backend gap found and left alone (routes/ wasn't in scope): Bedrock's `difficulty`/`gamemode` fields come back typed `enum` with no `options` (Bedrock settings stay unported per that route's own header comment) — rendered honestly as a plain text field rather than faked with a hardcoded option list.
**Files:** `clients/desktop-web/src/lib/sections/settings/SettingsSection.svelte`, `clients/desktop-web/src/lib/sections/settings/model.ts` (new — demo/fallback settings data), `clients/desktop-web/src/lib/components/base/Icon.svelte` (new `gear` glyph), `clients/desktop-web/src/lib/components/base/NumberField.svelte` (add `onchange`)
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
