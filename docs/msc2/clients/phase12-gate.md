# Phase 12 client consistency and parity gate

This `phase12-gate.md` record separates automated implementation evidence from
Cameron's required live parity review.

**Status:** implementation evidence recorded; Cameron's visual parity and
anti-slop review remains required before the Phase 12 gate can be declared
closed.

**Authority:** `MSC2-VISION.md`, `msc2-port-plan.md` Phase 12, the locked
specimens in `docs/msc2/renderings/`, and `antiAIslop.md`. MSC 1 is the
behavior oracle; this record names the corresponding source areas rather than
copying Swift implementation details into the client.

## What this gate proves

The automated portion is intentionally a consistency boundary, not a claim
that a static test can replace a human visual comparison. The exact P12.LAST
Verify command runs the client unit/type checks, this gate runner, the
capability-matrix checker, and the workspace tests. Cameron's final review
must still compare every rebuilt screen and sheet against the MSC 1 checkout
and run every item in `antiAIslop.md`'s checklist.

The gate runner checks that:

- the capability matrix is exactly the frozen HTTP and WebSocket contract,
  has no blank cells, and leaves only the documented future/profile/Bedrock
  surfaces as Desktop/Web `Planned`;
- the shared Svelte route tree and Tauri boundary preserve D-003's same-screen
  rule, while generated DTOs, host-keyed state, permissions, capabilities,
  served help, browser auth, and Tauri auth remain covered by their existing
  focused checks;
- CI still names the macOS, Linux, and Windows client candidates, native
  Linux WebKitGTK, and the separate headless no-GUI artifact check;
- first launch remains setup → tour → Handbook, with agent-owned setup and
  client-owned presentation, and reset remains client-only versus host-owned;
- the locked rendering specimens and anti-slop checklist remain present.

## MSC 1 shape → MSC 2 counterpart

| MSC 1 source shape | MSC 2 counterpart | Behavior/evidence boundary |
|---|---|---|
| `ContentView.swift`, `DetailsView.swift`, `DetailsHeaderSectionView.swift`, `MSCTabBar.swift`, `SidebarView.swift`, `ConsoleView.swift` | `ApplicationShell.svelte`, shell components, the seven-tab `PRIMARY_TABS`, `ConsoleDock.svelte` | Shared host/server context, terrain banner, control rail, tab strip, and persistent console. Packs is the named owner-approved exception in `msc2-port-plan.md`; its agent routes and matrix rows remain intact. |
| `DetailsOverviewTabView.swift`, overview helper/card files | `sections/home/` | Server state, connection addresses, health, active world, players, chat, notes, and live statistics remain separate cards with the locked one-card language. |
| `DetailsPlayersTabView.swift`, `PlayerProfilesCard.swift`, `PlayerProfileDetailSheet.swift`, `PlayerInventoryView.swift`, `PlayerSessionTimelineView.swift` | `sections/players-online/` | Online roster, Java/Bedrock player data, inventory, skin/identity treatment, and session log use the delivered shared routes; unsupported future profile mutations remain capability-gated. |
| `DetailsWorldsTabView.swift`, `WorldSlotsView.swift`, `BackupsView.swift`, `CreateWorldSlotSheet.swift`, `WorldConversionWizardView.swift`, `WorldRepairView.swift` | `sections/worlds/` and `sections/backups/` | Slot activation and profile ownership, create/import/duplicate/replace/delete/export/convert/repair, backups, staged transfers, and safety confirmations stay agent-backed. |
| `DetailsComponentsTabView.swift`, `ModrinthBrowserView.swift`, `AddonUpdateSheet.swift`, `ClientExportSheet.swift` | `sections/components/`, `sections/addons/` | Installed component state, catalog/detail/version compatibility, install/update/toggle/remove, modpack flows, and client export remain provider- and operation-aware. |
| `DetailsPerformanceTabView.swift`, performance helpers/charts | `sections/performance/` | Bounded metric history, help, and live/stat states use the shared API and the same shell. |
| `DetailsSettingsTabView.swift`, `ServerSettingsView.swift`, `HealthView.swift`, `ConnectivityView.swift`, `PlayitView.swift`, `JoinCardView.swift` | `sections/settings/`, `sections/health/`, `sections/connectivity/`, `sections/access/`, `server-editor/`, shell sidebar | Schema-driven server settings are separated from world profiles; health, connectivity, helpers, access, and broadcast keep capability, permission, and active-server boundaries visible. |
| `ServerFilesTabView.swift` | `sections/files/` | Bounded browse/read and platform file-manager reveal use the shared screen and platform adapter. |
| `ManageServersView.swift`, `ServerEditorView.swift`, `ServerEditor*Tab.swift` | `sections/fleet/`, `sections/server-editor/`, `ManageSheet.svelte`, `ServerEditorSheet.svelte` | Fleet selection, create/import, lifecycle, EULA, rename/delete, editor tabs, and first start stay in shared sheets and routes. |
| `AddServerWizardView.swift` | `sections/fleet/wizard/` | Fresh and Import Existing paths retain the oracle's five-step information shape, staged transfer boundary, network/world/add-on choices, confirmation, and explicit create action. |
| `SetupWizardView.swift`, `OnboardingManager.swift`, `OnboardingOverlayView.swift`, `ServerHandbookView.swift`, `SplashGateView.swift` | `FirstLaunchGate.svelte`, `SetupIntro.svelte`, `TourOverlay.svelte`, `sections/handbook/`, `SplashGate.svelte` | Setup is agent-ready gated; the tour is anchored and resumable; the Handbook is served content; splash and reduced-motion fallback remain client/platform presentation. |
| `MSCSettingsView.swift` and reset flows | `sections/app-settings/`, `ResetSheet.svelte`, host reset evidence | Client reset never calls the agent; host reset is authenticated, operation-backed, refuses running servers, preserves the selected deletion boundary, and requires fresh pairing after identity rotation. |

## Consistency and anti-slop review sheet

The locked reference specimens establish one vocabulary: surface tiers rather
than effects, one card depth, 4pt spacing, three sheet widths (480/640/820),
restrained system-sans weights, neutral category treatments, and accent only
for defined state/action meaning. The implementation checks cover the tokens,
primitives, shell, and source boundaries; the final visual pass must mark each
item below after opening every screen and sheet.

| Review item | Automated evidence | Human result |
|---|---|---|
| Shared sheet widths, spacing, card depth, and button/type roles | `tests/components/base.test.ts`, `tests/visual/shell.test.ts` | Pending Cameron visual review |
| No decorative glow, glass, gradients, side rails, emoji, or accidental colored informational icons | `antiAIslop.md`, `tests/visual/shell.test.ts`, rendering specimens | Pending Cameron visual review |
| One clear first/second/third read on each screen | rendering specimens and screen checks | Pending Cameron visual review |
| Motion communicates a state/spatial change and reduced-motion remains usable | splash/onboarding checks and native-renderer workflow | Pending Cameron visual review |
| Shape and behavior match the MSC 1 source rows above | focused screen tests plus MSC1 source comparison | Pending Cameron screen-by-screen pass |

## Deferred or explicit boundaries

These are not hidden failures:

- Packs is the named Phase 12 screen exception. The underlying agent
  capability remains available to CLI and future clients.
- Watchdog controls and the remaining stream/future operations stay `Planned`
  where the matrix says so; a disabled or absent control is not counted as
  implemented. The rebuilt Desktop/Web Players, Worlds, Server Editor,
  Handbook, setup, and reset surfaces are marked `Implemented` only where
  their current source actually calls the corresponding agent route.
- Release signing/notarization and live cross-platform Bedrock runtime
  evidence are not claimed by this client gate. They remain in their recorded
  packaging/Bedrock handoffs.
- Cameron must supply the real-agent first-launch, reset, and full visual
  parity observations. This document deliberately does not turn static or
  fake-harness evidence into a claim that those walks happened.
