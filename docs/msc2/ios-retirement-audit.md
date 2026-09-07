# Native-mobile retirement audit

**Date:** 2026-09-07
**Decision:** D-033, as amended by P12.108
**Step:** P12.107

## Result

MSC 2 v1 has no native iOS project and no supported mobile management client.
The retained control surfaces are the Tauri desktop app, desktop browser, and
headless CLI. A responsive layout may make a desktop/browser page fit a narrow
window, but that is not a supported phone client or a promise of mobile
management. Optional Tailscale remains a way to reach a desktop app or desktop
browser remotely; it does not create general-LAN management.

## Search terms and scope

The retirement scan searched these repository-visible remnants:

```text
clients/ios
MSCRemoteiOS
MSC Remote
iPhone app
iOS client
native iOS
ios_status
ios-contract-check
```

The active areas inspected were the client tree, phase checkers and release
checks, the capability matrix and API-contract notes, active Phase 4–13 scope
notes, Rust source comments, README/help content, generated agent web assets,
and the controlled vision/product/engineering/port-plan documents. The final
scan excludes only the working plan itself (which contains this step's
historical file names and its verification command), the historical rolling
plan archive, MSC 1 audit evidence, the decision register, and this audit.

```sh
git grep -n -i -E 'clients/ios|MSCRemoteiOS|MSC Remote|iPhone app|iOS client|native iOS|ios_status|ios-contract-check' \
  -- ':!docs/msc2/rolling-plan.md' \
  ':!docs/msc2/rolling-plan-archive.md' \
  ':!docs/msc2/audit/**' \
  ':!docs/msc2/msc2-decisions.md' \
  ':!docs/msc2/ios-retirement-audit.md'
```

That scan is clean. The working-plan exclusion is deliberate: without it, the
command finds the historical P12.102–P12.107 descriptions and its own literal
search terms, so it cannot be a meaningful content check.

## Removed MSC 2 artifacts

- The `clients/ios/` native application, Xcode project, Swift sources, tests,
  screenshots, README, and bundled assets were removed in P12.103.
- The iOS-only Phase 4 lifecycle checklist, Phase 7 provisioning checklist,
  and Phase 10 contract checker were removed in P12.103.
- The iOS capability-matrix column and checker/release dependencies were
  removed in P12.104. The matrix now describes the retained client columns.
- Active onboarding, handbook, generated API, agent setup, and embedded-bundle
  wording was reconciled in P12.105.
- Active phase-scope notes, API provenance notes, and source comments were
  reconciled in P12.106 and this audit's final scan.

No phone/browser client was added as a replacement. The Rust agent, HTTP and
WebSocket contract, server lifecycle, desktop/browser client, and headless CLI
remain in scope.

## Intentionally retained historical references

These references remain available because they explain work that really
happened; they are not current product surfaces:

- Git history retains the P12.102–P12.106 commits and the deleted client is
  recoverable from that history.
- `docs/msc2/rolling-plan-archive.md` retains the phase record as historical
  evidence.
- `docs/msc2/audit/` retains the MSC 1 extraction and portability evidence.
- `docs/msc2/msc2-decisions.md` retains D-004's supersession and D-033's
  owner-approved boundary.
- The read-only MSC 1 oracle at
  `~/Documents/Swift Projects/minecraft-server-controller` remains untouched
  and available for compatibility comparison.

## Intentionally retained third-party and gameplay meanings

The word “mobile” still appears where it means something other than an MSC
management client:

- Router guide fixtures and `AdminSurface::MobileApp` describe a third-party
  router's own mobile app. They do not describe an MSC client.
- Bedrock handbook and setup copy describe Minecraft players on mobile phones,
  consoles, and Windows. They describe server audience/platform support, not
  MSC control-surface support.
- Responsive desktop/browser layout and generic “Mobile navigation” test
  language describe viewport behavior and layout implementation only.

Those meanings are intentionally preserved because removing them would change
the networking and Minecraft product documentation rather than retire the
native management client.
