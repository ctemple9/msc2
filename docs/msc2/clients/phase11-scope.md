# Phase 11 — Desktop and Web Client Scope

## Purpose and authority

This is the P11.1 boundary for the shared Svelte frontend, its thin Tauri
desktop shell, and the agent-served browser page. It is a scope document, not
an implementation plan for individual screens. The frozen OpenAPI and
WebSocket contracts and `client-capability-matrix.csv` are the route inventory;
this document says where each row belongs.

The copied iOS client is the primary behavioral reference. It is the closest
existing client to the API and contains the real workflows, loading/error
states, operation polling, reconnect behavior, and mobile reshaping that must
survive in the shared client. The MSC 1 macOS views are used only for desktop
information architecture and visual language: a server-list/sidebar control
surface, an always-available console, an editor organized by general/settings,
worlds, backups, components, and broadcast, and handbook/help readers. macOS
view code is not a second desktop implementation target.

P11.1 does not change any capability status in the matrix. **Matrix changes:
none.** Every Desktop/Web cell remains `Planned` until the step that ships and
proves that surface updates it with evidence. The checker below requires all
113 matrix rows to appear in the route appendix and also checks that those rows
are exactly the current HTTP plus WebSocket contract.

## Source reading and behavioral inventory

The repository contains the 53 Swift files copied in P2.18, plus
`ConvertWorldView.swift` and `ImportWorldView.swift` added by P6.24. The
following responsibilities are carried forward from those files:

| iOS source area | Behavior to preserve in the shared client |
|---|---|
| `RootView.swift`, `MSCRemoteApp.swift`, `MSCStyles.swift`, `AppIconMark.swift` | Five top-level destinations, iPhone tab versus iPad sidebar adaptation, shared colors/cards/status treatment, splash and first-launch guide timing. The web shell uses responsive navigation rather than duplicating a desktop-only screen. |
| `DashboardView.swift`, `DashboardViewModel*.swift`, and dashboard cards | Host connection state, active-server selection, create/import/rename/delete/EULA flows, start/stop/restart, bounded status polling, console WebSocket reconnect, operation polling, performance history, player summary, notifications, and truthful error states. |
| `RemoteAPIClient.swift`, `RemoteAPIModels.swift` | The existing request paths, query/body shapes, optional-field tolerance, `ErrorDTO` handling, operation terminal polling, staged-transfer calls, and console framing are behavioral evidence. These hand-maintained DTOs are not copied into TypeScript; P11.3 generates the TypeScript surface from OpenAPI. |
| `ConsoleView.swift`, `ConsoleFilterBar.swift`, `CommandsView.swift`, `CommandPickerSheet.swift` | Bounded console history, search/filter/pause/clear behavior, command history/favorites, quick-command categories, and Java-versus-Bedrock command vocabulary. Phase 11 ships the generic console; Bedrock-specific sections wait for the later capability extension. |
| `ServerView.swift`, `ServerSettingsView.swift`, `ServerVersionView.swift`, `ComponentsView.swift`, `CatalogBrowserView.swift`, `ResourcePacksView.swift` | Server editor hierarchy, schema-driven settings, version selection and async result handling, add-on/component inventory and mutation, catalog search/install, and resource-pack workflows. The shared UI must keep provider-unavailable, pack-managed, permission, and operation states visible. |
| `WorldsView.swift`, `ImportWorldView.swift`, `ConvertWorldView.swift` | World-slot inventory and activation, create/rename/duplicate/delete/replace/import/export/convert, staged transfer progress, destructive confirmations, and Bedrock repair capability messaging. |
| `HealthView.swift`, `ConnectivityView.swift`, `PlayitView.swift`, `JoinCardView.swift`, `XboxAuthBanner.swift` | Health cards and repairs, connectivity diagnostics, join-address presentation, Playit and Xbox Broadcast helper state, and capability/permission-aware administration. Native share sheets and notifications remain platform adapters, not divergent screens. |
| `SettingsView.swift`, `SettingsStore.swift`, `SettingsPairingCard.swift`, `SettingsConnectionTestSection.swift`, `SettingsJoinCardSection.swift`, `TailscaleHelpSheet.swift`, `NetworkSafety.swift`, `KeychainTokenStore.swift` | Per-host connection state, pairing/token lifecycle, loopback/private-host safety, notification preferences, join-card preferences, and Tailscale guidance. The Phase 11 auth steps still have to close browser cookies and Tauri local/remote credentials; P11.1 reserves their shared screen seam. |
| `AllowlistView.swift`, `PlayersView.swift`/`PlayerRow.swift`, `UsersView.swift`, `QuickGuideView.swift`, `MaintenanceView.swift`, `SplashGateView.swift`, `HapticHelpers.swift`, `NotificationManager.swift`, `MSCNotificationDelegate.swift` | Existing Bedrock/player/profile, access-administration, guide, lifecycle, and native-presentation evidence. Profile/skin/hidden-player behavior is not silently promoted into Phase 11; the educational prose becomes served content, and native haptics/notifications get web fallbacks. |

### First-launch and onboarding preservation contract

The MSC 1 first-launch experience is a behavior to preserve, not merely a
visual reference. The implementation and evidence must trace these source
areas separately:

| MSC 1 source | Behavior that must survive | MSC 2 owner |
|---|---|---|
| `SetupWizardView.swift` | Fresh-install setup sheet, page order, required Java/server-root checks, optional helper pages, back/next behavior, and completion persistence. | Shared client presentation; agent setup/probes |
| `AppViewModel+ServerSettings.swift` | Setup dismissal → Concept Guide handoff → onboarding-tour handoff, one-time flags, skip behavior, reopen-from-preferences behavior, and presentation timing needed to avoid overlapping sheets. | Shared client |
| `ConceptGuideView.swift`, `ServerHandbookView.swift`, `ServerHandbookTopics.swift` | Concept Guide page order and explanations, Handbook entry point, 31 handbook topics, and the relationship between the guide and the Handbook. | Agent-served content; shared client rendering |
| `OnboardingManager.swift`, `OnboardingOverlayView.swift`, and onboarding anchors in the wizard/details views | The guided tour's step order, titles, instructions, user-action pauses, form-card hiding/resume behavior, spotlight anchors, completion state, and restart behavior. | Shared client |
| `SplashGateView.swift`, bundled `splash_intro` asset | Cold-launch splash behavior, playback completion/fallback, safety timeout, and reduced-motion/accessibility degradation. | Shared client/platform adapter |
| `AppViewModel+ServerControls.swift` | First-server initiation, readiness-driven sequencing, automatic stop/start behavior, and first-start completion state. | Agent; client displays progress and completion |

The source inventory is complete only when each row has either a delivered
implementation or an explicit, evidence-backed deviation. “Onboarding text
was extracted” alone is not sufficient: the fresh-install sequence, state
transitions, anchors, and animation/fallback behavior must also be exercised.

### P11.15 educational-content handoff

P11.15 extracts the data that must not be rewritten in a Svelte component:

- `content/help/handbook/` has one Markdown-with-front-matter file for every
  one of the 31 `HandbookTopic` cases. Its front matter cites the exact MSC 1
  content symbol, preserving both the topic order/category and the source of
  its explanation.
- `content/help/concept/` and `content/guides/concept-guide.json` preserve the
  seven Concept Guide pages and their order. `ConceptGuideDiagrams.swift`
  remains cited but its drawings are honestly recorded as unresolved assets:
  P11.16 owns a reviewed client rendering or replacement. It also owns visual
  anchoring and any animation/reduced-motion fallback.
- `content/guides/onboarding.json` preserves first-launch content, ordering,
  branching, form-card hide/resume behavior, skip wording, and the
  `msc_onboarding_tour_complete` persistence key. Its complete per-step source
  mapping lives in `content/guides/onboarding-source-map.json`. The matching
  fixtures cover fresh installation, already-seen state, skipping, reopening
  from Preferences, and an unknown future topic.
- P11.16 replaces the unresolved, non-extracted `splash_intro` asset with a
  bounded CSS mark. It is explicitly a reviewed presentation replacement, not
  a claim that the original animation asset was preserved; reduced-motion
  users bypass it immediately.
- `content/guides/router-catalog.json` and
  `content/guides/router-troubleshooting.json` contain catalog records and
  human-readable guidance only. They explicitly cite the existing Rust matcher,
  fallback, composer, runtime replacement, and troubleshooting engine as
  executable behavior rather than duplicating a second implementation.
- `fixtures/help-content/help-id-coverage.json` is the named inventory for
  currently emitted settings, health, diagnostics, performance, connectivity,
  operation/error, and `bedrock.runtime-unavailable` pointers. The checker
  requires a Markdown topic for every one.

No content is served or rendered by P11.15. P11.24 embeds and serves it; P11.16
renders it in the shared client.

### Desktop information architecture and visual language

The macOS oracle supplies a hierarchy, not a screen fork. The shared client
will have a host picker and server list/sidebar; a selected host and active
server are always visible. Its main sections are Home/Fleet, Console/Live,
Players Online, Worlds and Backups, Add-ons and Components, and
Settings/Health/Connectivity/Access. The editor uses cards, clear section
headers, status pills, explicit destructive confirmations, and a persistent
console affordance. The web layout and Tauri shell load the same Svelte route
tree, so D-003's same-screen rule is structural rather than a test convention.

## Non-negotiable boundaries

* **D-003 same-screen rule:** Tauri supplies credentials, file pickers,
  notifications, menus, and window integration through narrow adapters. It may
  not reveal a desktop-only management screen. Browser fallbacks use the same
  workflow.
* **D-013 host scoping:** connection, credential, capability, active-server,
  console, operation, notification, and cached screen state are keyed by
  `hostId`. Every management route is rendered in a host/server context where
  applicable, and the host and active server are prominent before destructive
  actions.
* **D-023 matrix discipline:** a route is not `Implemented` because a button or
  disabled placeholder exists. The Desktop/Web cell changes only with a tested
  screen or shared infrastructure path. No intentional exception is invented
  here, and no iOS parity requirement is weakened because a workflow is hard
  on a small screen.
* **D-026 help ownership:** handbook, concept, router-guide, troubleshooting,
  onboarding text, and contextual explanations are served data. Screens render
  `helpId` and structured content; they do not carry a second prose corpus.
  The P11.15/P11.16 steps own extraction and rendering. `QuickGuideView.swift`
  is evidence of anchors and content shape, not permission to duplicate text.
  The first-launch sequence and its client-owned presentation behavior are
  covered by the explicit preservation contract above.
* **D-021 resource bounds:** console history, operation snapshots, reconnect
  queues, notifications, performance points, catalog results, and staged
  transfer state all have explicit client bounds. A browser tab or Tauri window
  cannot grow an unbounded copy of an agent stream.
* **Bedrock extension:** P11.1 reserves a registry keyed by stable section
  identifiers and advertised capabilities. It ships **no Bedrock section**,
  creation flow, settings, allowlist, permissions, world, backup, console, or
  runtime screen. It never infers Bedrock support from the host OS.
* **Player-profile extension:** P11.1 ships **no profiles screen**. The frozen
  profile DTOs do not assign the player-profile agent rows to Phase 11. A later
  phase must first port the ledgered profile loads, Mojang/Floodgate
  resolution, manual Bedrock identification, UUID migration/data mutation,
  hidden profiles, and skin storage/serving, then extend the contract and
  capabilities before registering profile sections.
* **Network posture:** browser management is loopback by default or an
  explicitly configured Tailscale path. Tailscale never replaces
  authentication. General-LAN management, a local certificate authority, and
  bypassing browser certificate warnings are not client features in v1.

### P11.5 routing contract

The shared client owns a descriptor registry keyed by stable section IDs and
URL segments. A descriptor declares whether it is host- or server-scoped, its
permission and capability predicates, and a lazy component loader. The route
shape is `/hosts/:hostId/<section>` for host sections and
`/hosts/:hostId/servers/:serverId/<section>` for server sections; parameters
and remaining deep-link segments are encoded and decoded as data, never stored
in a global active-route enum. Unknown sections resolve to the shared fallback
while preserving the URL. The reserved `bedrock/*` and `profiles/*` families
also resolve to that boundary without registering or rendering a Phase 11
screen. Capability predicates read the advertised capability response; they
never infer support from `hostOs`. Narrow and wide layouts consume the same
filtered descriptor list and do not assume a fixed number of sections.

P11.20 proves the Bedrock seam with a test-only descriptor under the reserved
`bedrock` family. That descriptor reads the generated `serverTypes.bedrock`
advertisement and optional `BedrockRuntimeStateDTO`: it is visible only when
the agent advertises support and a present runtime state is `available`. Its
predicate deliberately ignores backend and reason strings, so an agent can add
new values without requiring a client release. No Bedrock descriptor is
registered in the production registry, and no matrix cell changes to
`Implemented` from this seam evidence.

## Screen and infrastructure ownership

| Phase 11 area | Planned client responsibility | Deliberate non-claim |
|---|---|---|
| Home and fleet | Host/server picker, active-server state, create/import/rename/delete/EULA, Java family/version/runtime selection and install, templates, lifecycle controls, capability and permission explanations. | No Bedrock creation or runtime screen before the Phase 10 contract closes. |
| Console and live | Console tail and WebSocket, bounded history/filter/search/pause/clear, command entry/history/favorites, operation progress/cancel/recovery, notifications, performance charts, help links. | No profile or Bedrock-only console UI. |
| Online players | Generic roster from the advertised online-player capability, unknown/absent future profile fields tolerated. | No profile, skin, hidden-player, session-history, UUID-migration, or player-data mutation screen. |
| Worlds, backups, transfers | Java-capable slot/world/backup workflows, staged upload/download, conversion, thumbnails when advertised, risk-aware confirmation and recovery. | Bedrock-specific rows remain capability explanations until the later Bedrock client group; Bedrock backup restore keeps the agent's slot-based boundary. |
| Add-ons and components | Installed add-ons, catalog, install/update/toggle/remove/source, component state, client export, modpack inspect/import/manual-file completion. | No fake provider success and no client-owned copy of agent dependency/provenance rules. |
| Administration | Schema-driven settings, RAM/Java/Geyser helper settings, health/problems/repairs, connectivity, Playit, DuckDNS, Xbox Broadcast, resource packs, named-token access. | Agent-Planned watchdog/files/profile operations remain future or unavailable explanations, not pretend controls. |
| Help and onboarding | Render agent-served Markdown/structured guides, contextual `helpId`, related topics, unknown-topic degradation, the fresh-install setup → Concept Guide → tour sequence, tour anchors and pauses, Handbook reopening, and client-owned splash/animation fallbacks. | No screen-local handbook or router-guide prose; no silent replacement of the first-launch experience with a generic welcome screen. |
| Shared infrastructure | Generated DTOs, host-keyed stores, auth adapters, capability/permission filtering, operation/reconnect state, staged-transfer transport, WebSocket framing, error/help routing, and the exact bundle identity between browser and Tauri. | P11.1 does not implement any frontend, Tauri crate, browser session, or agent route. |

## Route and matrix appendix

The tags in this appendix are the P11.1 disposition for the corresponding
matrix row:

* `[screen]` is a Java/shared client screen or workflow owned by a later
  Phase 11 implementation step.
* `[shared]` is client infrastructure consumed by several screens.
* `[future]` is an honest future client/agent capability; it remains `Planned`
  and must not be implied by a disabled control.
* `[gap]` is an agent capability gap explicitly left outside this phase. The
  client may explain that it is unavailable, but does not recreate the agent
  behavior locally.

Every current Desktop/Web cell is still `Planned` in this scope step.

### Fleet, lifecycle, and shared state

- `[screen]` `POST /v1/active-server` — active-server picker and host/server context.
- `[screen]` `GET /v1/servers` — registered-server list.
- `[screen]` `POST /v1/servers/create` — Java create flow and durable operation handoff.
- `[screen]` `POST /v1/servers/delete` — destructive server deletion.
- `[screen]` `POST /v1/servers/eula` — EULA acceptance.
- `[screen]` `POST /v1/servers/import` — scan/import/rescan and staged transfer entry.
- `[screen]` `POST /v1/servers/rename` — server rename.
- `[screen]` `GET /v1/status` — current host/server runtime status.
- `[screen]` `POST /v1/start` — start control.
- `[screen]` `POST /v1/stop` — stop control.
- `[screen]` `GET /v1/templates` — Java template inventory.
- `[screen]` `POST /v1/templates` — template export/create.
- `[screen]` `GET /v1/java-runtimes` — host Java runtime inventory.
- `[screen]` `POST /v1/java-runtimes/install` — agent-owned runtime installation observed by the client.
- `[screen]` `GET /v1/versions` — available versions for the active Java server.
- `[screen]` `GET /v1/versions/create` — version choices for the Java create flow.
- `[shared]` `GET /v1/capabilities` — host-scoped capability advertisement and future-section registry input.
- `[shared]` `GET /v1/me` — role and permission context for action filtering.
- `[shared]` `POST /v1/operations` — shared operation admission shape; screens do not invent a second progress model.
- `[shared]` `GET /v1/operations/{id}` — durable operation polling.
- `[shared]` `POST /v1/operations/{id}/cancel` — cooperative cancellation.
- `[shared]` `WS /v1/operations/{id}/stream` — optional live operation progress.

### Console, notifications, performance, and online players

- `[screen]` `POST /v1/command` — console command entry and quick commands.
- `[screen]` `GET /v1/console/tail` — bounded console history.
- `[shared]` `WS /v1/console/stream` — bounded-history-then-live console frames.
- `[shared]` `WS /v1/notifications/stream` — status event feed; native notification delivery remains an adapter.
- `[screen]` `GET /v1/performance` — bounded performance snapshot/chart data.
- `[screen]` `GET /v1/players` — generic online roster only.
- `[future]` `GET /v1/session-log` — historical join/leave timeline requires an explicitly delivered client surface later.
- `[future]` `GET /v1/players/profiles` — reserved player profiles; no profiles screen.
- `[future]` `POST /v1/players/hidden` — reserved hidden-profile mutation; no profiles screen.
- `[future]` `GET /v1/players/{profileId}/skin` — reserved skin serving; no profiles screen.
- `[future]` `POST /v1/players/skin-override` — reserved skin override; no profiles screen.

### Worlds, backups, and bounded transfers

- `[screen]` `GET /v1/worlds` — world-slot inventory.
- `[screen]` `POST /v1/worlds/activate` — slot activation with confirmation and operation state.
- `[screen]` `POST /v1/worlds/create` — create a world slot.
- `[screen]` `POST /v1/worlds/delete` — delete a non-active slot.
- `[screen]` `POST /v1/worlds/duplicate` — duplicate a slot.
- `[screen]` `POST /v1/worlds/export` — stage a world archive for download.
- `[screen]` `POST /v1/worlds/import` — import a staged world archive.
- `[screen]` `POST /v1/worlds/rename` — rename slot metadata.
- `[screen]` `POST /v1/worlds/replace` — copy saved-slot content.
- `[screen]` `POST /v1/worlds/convert` — staged conversion workflow.
- `[screen]` `POST /v1/worlds/rename-active-world` — direct active-world mutation where the agent advertises it.
- `[screen]` `POST /v1/worlds/replace-active-world` — direct active-world replacement where the agent advertises it.
- `[future]` `POST /v1/worlds/update` — active-world save-to-slot capability remains an explicit future row.
- `[future]` `POST /v1/worlds/repair` — Bedrock-only repair remains unavailable until the later Bedrock client group.
- `[future]` `GET /v1/worlds/{slotId}/thumbnail` — thumbnail rendering waits for delivered evidence and capability advertisement.
- `[screen]` `GET /v1/backups` — backup inventory.
- `[screen]` `GET /v1/backups/config` — auto-backup settings.
- `[screen]` `POST /v1/backups/config` — auto-backup mutation.
- `[screen]` `POST /v1/backups/delete` — backup deletion.
- `[screen]` `POST /v1/backups/now` — immediate backup.
- `[screen]` `POST /v1/backups/restore` — Java backup restore; Bedrock capability-unavailable results remain visible and route to slots.
- `[shared]` `POST /v1/staged-uploads` — bounded staged-upload admission.
- `[shared]` `PUT /v1/staged-uploads/{id}` — bounded upload bytes.
- `[shared]` `GET /v1/staged-downloads/{id}` — bounded download bytes.

### Add-ons, components, modpacks, and resource packs

- `[screen]` `GET /v1/addons` — installed add-on inventory and update status.
- `[screen]` `GET /v1/components` — installed system-component inventory.
- `[screen]` `GET /v1/components/client-export` — client-side component export staging.
- `[screen]` `POST /v1/components/install` — catalog/local add-on install and operation progress.
- `[screen]` `POST /v1/components/remove` — add-on removal.
- `[screen]` `POST /v1/components/update` — component/add-on update.
- `[screen]` `POST /v1/components/version` — server JAR version change.
- `[screen]` `GET /v1/catalog/search` — catalog search.
- `[screen]` `POST /v1/modpacks/inspect` — staged pack inspection.
- `[screen]` `POST /v1/modpacks/import` — pack import/replace operation.
- `[screen]` `POST /v1/modpacks/{operationId}/manual-file` — bounded D-027 manual-file completion.
- `[screen]` `GET /v1/resourcepacks` — resource-pack inventory.
- `[screen]` `POST /v1/resourcepacks/activate` — local Java resource-pack activation.
- `[screen]` `POST /v1/resourcepacks/remove` — resource-pack removal.
- `[screen]` `POST /v1/resourcepacks/seturl` — custom Java pack URL.
- `[screen]` `POST /v1/resourcepacks/toggle` — Geyser resource-pack toggle.

### Settings, health, networking, helpers, and access

- `[screen]` `GET /v1/settings` — schema-driven server settings.
- `[screen]` `POST /v1/settings` — sparse settings mutation.
- `[screen]` `GET /v1/config/java-runtime` — Java executable setting.
- `[screen]` `POST /v1/config/java-runtime` — Java executable mutation.
- `[screen]` `GET /v1/config/ram` — RAM allocation view.
- `[screen]` `POST /v1/config/ram` — RAM allocation mutation.
- `[screen]` `GET /v1/config/geyser` — managed Java Geyser helper settings.
- `[screen]` `POST /v1/config/geyser` — managed Java Geyser helper mutation.
- `[screen]` `GET /v1/health` — health cards.
- `[screen]` `GET /v1/health/problems` — startup problems.
- `[screen]` `POST /v1/health/repair` — repair action and operation state.
- `[screen]` `GET /v1/connectivity` — reachability and join method diagnostics.
- `[screen]` `GET /v1/playit` — Playit status.
- `[screen]` `POST /v1/playit/start` — Playit start.
- `[screen]` `POST /v1/playit/stop` — Playit stop.
- `[screen]` `GET /v1/duckdns` — DuckDNS status.
- `[screen]` `POST /v1/duckdns` — DuckDNS mutation.
- `[screen]` `GET /v1/broadcast/autostart` — Xbox Broadcast auto-start setting.
- `[screen]` `POST /v1/broadcast/autostart` — Xbox Broadcast auto-start mutation.
- `[screen]` `GET /v1/broadcast/auth-prompt` — pending helper sign-in prompt.
- `[screen]` `POST /v1/broadcast/auth-prompt/dismiss` — dismiss helper prompt.
- `[screen]` `GET /v1/broadcast/status` — helper status.
- `[screen]` `POST /v1/broadcast/start` — helper start.
- `[screen]` `POST /v1/broadcast/stop` — helper stop.
- `[screen]` `POST /v1/broadcast/restart` — helper restart.
- `[screen]` `POST /v1/broadcast/credentials` — one-time credential update through an explicit form.
- `[screen]` `GET /v1/broadcast/jar-status` — helper JAR status.
- `[screen]` `POST /v1/broadcast/download-jar` — helper JAR download operation.
- `[future]` `GET /v1/users` — named-token inventory waits for the Phase 11 authentication design.
- `[future]` `POST /v1/users` — named-token creation waits for the Phase 11 authentication design.
- `[future]` `POST /v1/users/revoke` — named-token revocation waits for the Phase 11 authentication design.
- `[future]` `POST /v1/users/update` — named-token update waits for the Phase 11 authentication design.

### Agent gaps and Bedrock-only rows

- `[gap]` `GET /v1/allowlist` — Bedrock allowlist; no Bedrock section.
- `[gap]` `POST /v1/allowlist` — Bedrock allowlist mutation; no Bedrock section.
- `[gap]` `GET /v1/files` — agent-Planned server file browser; no client-side filesystem substitute.
- `[gap]` `GET /v1/files/read` — agent-Planned file preview; no client-side filesystem substitute.
- `[gap]` `POST /v1/watchdog/enable` — agent-Planned watchdog control.
- `[gap]` `POST /v1/watchdog/disable` — agent-Planned watchdog control.
- `[gap]` `GET /v1/watchdog/status` — agent-Planned watchdog status.
- `[gap]` `GET /v1/help/{helpId}` — help route remains agent-Planned in the current matrix; P11.15/P11.24 own the content corpus and serving handoff, so P11.1 does not claim content exists.

The route appendix intentionally contains the exact matrix identity once per
method/path. The checker reads only those bullet entries and compares them
against both the matrix and the frozen HTTP/WebSocket documents.

## Later handoffs

P11.2–P11.19 may build the shared foundation and Java/shared screens against
the frozen baseline while Phase 10 is active. P11.20 may later regenerate and
prove the Bedrock extension seam after Phase 10 closes, but it must ship no
Bedrock screen. P11.21 onward owns browser/Tauri authentication, served help,
bundle serving, installation, coordinated updates, and the final gate. Those
steps may add evidence and change matrix cells; they do not change this
P11.1 ownership boundary without an explicit scope amendment.
