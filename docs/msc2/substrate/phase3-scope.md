# Phase 3's substrate surface — what's in, what's deferred

**Status: Proposed**, pending Cameron's confirmation during the Read move — the same role `docs/msc2/api-contract/auth-scope-phase2.md` played for Phase 2. This step makes `rolling-plan.md`'s own "Not in this phase" list load-bearing rather than plan prose that could quietly drift once execution starts. It resolves nothing new; it collects deferrals already stated elsewhere (the Phase 3 intro in `rolling-plan.md`, `msc2-port-plan.md` §3, and `msc2-decisions.md`) into one place Cameron can confirm or overrule in a single pass.

One item below — D-024 power management — is flagged explicitly for a decision, not just an FYI: it's a genuine scoping call where two source documents disagree, not an oversight.

---

## What Phase 3's gate actually requires

Per `msc2-port-plan.md` §3, verbatim: "Approved server roots and path safety · atomic writes · versioned configuration with migrations · `SecretStore` trait · audit log · download staging with checksum verification · operation journal · operation exclusivity." Plus Windows CI (D-017): path separators and length limits, file-locking semantics, service lifecycle, case-insensitive path comparison. **Exit criteria: substrate fixtures pass on macOS, Linux, and Windows.**

That is eight substrate items and a CI platform requirement. Everything below is named somewhere in the document set as adjacent to this phase, but is not one of those eight items — each gets its own heading so the reason it's out, and where it actually lands, is explicit rather than assumed.

---

## Real service registration

Installing the agent as a `launchd` LaunchDaemon, a Windows Service, or a `systemd` unit — plus the OS integration testing that goes with actually running as one. This phase only decides *identity and ownership* (P3.1: which account runs the agent, who owns its files); it installs nothing.

**Where it lands:** Phase 4. Its own gate, verbatim from `msc2-port-plan.md`: "headless service ownership proven on macOS (LaunchDaemon), Linux (systemd), and Windows (Service) — all three, not two." Confirmed by the §4B acceptance-test table, which places "Windows service lifecycle, sign-out survival, Job Object process trees" at "Phase 3 substrate / Phase 4" — the substrate half (path/lock/case-insensitivity behavior) is this phase's; the actual service-lifecycle behavior is Phase 4's.

## D-024 power management — flagged for Cameron to confirm or overrule

Sleep inhibition and the two host-role policies (dedicated-host: never sleep while remote management is enabled; normal desktop: sleep unless something's running), plus misconfiguration detection and warning (clamshell suspend, aggressive sleep timers, hibernation, power settings that override application inhibition).

**Why this one needs an explicit call, not just a note.** The document set disagrees with itself about where this lands:

- `msc2-port-plan.md` §3's own prose for Phase 3 names exactly eight substrate items (quoted above). Power management is not one of them.
- `msc2-port-plan.md` §4B's separate acceptance-test inventory places "Cross-platform sleep inhibition and the two power policies (D-024)" at "Phase 3."

Those two sections of the same document contradict each other. This step's recommendation: **defer to Phase 4**, not Phase 3. Reasoning — D-024 exists for a specific scenario, "remote-starting a stopped server on a closed MacBook," which only becomes something the codebase can attempt once Phase 4 lands real service lifecycle and remote start. D-024's own verification shape (real OS power APIs — `IOPMAssertion` on macOS, `SetThreadExecutionState` on Windows, `systemd-inhibit` on Linux) doesn't fit this phase's fixture-parity gate the way path safety or `SecretStore` do; every other Phase 3 item is verified by comparing a fixture's expected output against actual output, not by exercising a live OS power subsystem end to end.

**If Cameron overrules this** (keeps it in Phase 3, per §4B's placement): D-024 becomes a ninth substrate item, needs its own step or steps inserted into this plan before P3.20's exit-gate check, and Windows CI (already starting this phase per D-017) would need to cover `SetThreadExecutionState` alongside the path/locking/case-insensitivity behavior it already covers.

**Where it lands if this recommendation holds:** Phase 4, alongside real service registration.

## D-021's headless-package GUI-link verification

The CI check that every headless build artifact links no GUI framework, on every platform — D-021's requirement #1 ("No GUI dependencies in headless packages... verified — mechanically, in CI").

**Where it lands:** unresolved everywhere, not just here. `msc2-port-plan.md` §4B places D-021's resource-efficiency tests generally as "per-platform, as each platform's agent lands" — true of the bounded-growth and idle-memory-regression pieces of D-021, but the GUI-link check specifically is a packaging concern with no phase named anywhere in the port plan. Not invented a home here. Flagged as a genuine gap for the port plan itself to assign, not assumed to be Phase 3's because a headless package is exactly this phase's kind of concern.

## Real per-domain download workflows

Paper jar downloads, Xbox-jar downloads, plugin downloads, modpack downloads — the actual per-source fetch-and-install logic.

**Where it lands:** their own phases (7 for server families/provisioning, 8 for mods/plugins/modpacks). This phase builds only the shared primitive those workflows will call — P3.14, stage-then-checksum-verify-then-move-into-place — once, generically, so Phases 7–9 consume it instead of each reimplementing temp-file handling and verification from scratch.

## Wiring `SecretStore` into `msc-agent`'s real pairing flow

Replacing P2.3's fixed dev token (`MSC_DEV_TOKEN`, a hardcoded environment-variable string — see `auth-scope-phase2.md` §3) with actual QR-pairing-issued, durably-stored credentials, once a real `SecretStore` exists to store them in.

**Where it lands:** genuinely undecided — recorded here as a homeless gap rather than silently assigned, the same pattern D-027 exists to record for the CurseForge manual-download workflow. `msc2-decisions.md`'s D-012 Phase 2 scope note says plainly this is what Phase 3's `SecretStore` trait is *for*, but no phase in `msc2-port-plan.md` is actually named as the one that does the wiring. Three candidates, not adjudicated here:

1. **End of this phase** — natural fit, since `SecretStore` is freshly built and the pairing flow is the concrete thing it exists to serve, but it would pull real HTTP-handler and iOS-client changes into a phase whose gate is otherwise pure substrate (fixture parity across three OSes, no live client involved).
2. **Start of Phase 4** — Phase 4 is the first phase with a real running server and a real client-facing vertical slice, so wiring real auth in alongside it keeps "real credentials" and "real functionality" landing together.
3. **Its own line in `msc2-port-plan.md`** — if it's substantial enough (iOS-side pairing-link scanning changes, not just the agent-side store) to warrant tracking as a named unit of work rather than a sub-task of whichever phase absorbs it.

**Cameron should decide this during the Read move.** Whichever phase it lands in, nothing about this phase's own steps (P3.4–P3.20) depends on the answer — `SecretStore`'s trait and three platform implementations are built and fixture-tested on their own terms, unconsumed by the real pairing flow until whichever later step this gap resolves to.

## The rest of D-012's open items

Local automatic authorization, remote desktop pairing (Tauri → remote host), LAN TLS provisioning, Tailscale posture, and browser origin policy/CSRF — the four items from D-012's six-item gap list that the `SecretStore`-wiring item above doesn't cover. Untouched by this phase, exactly as `auth-scope-phase2.md` §4 left them after Phase 2. Per-host credential storage (item 3 of that same list) is the one exception — `SecretStore` (P3.8–P3.12) is what closes it, but only the trait and its platform implementations, not the wiring into a live per-host flow (see above).

---

## Summary

| Item | Named in Phase 3's own gate? | Lands |
|---|---|---|
| Real service registration (LaunchDaemon/Service/systemd install) | No | Phase 4 |
| D-024 power management | Contradiction between `msc2-port-plan.md` §3 and §4B | **Phase 4 (recommended) — Cameron to confirm or overrule** |
| D-021 headless-package GUI-link CI check | No | Unassigned anywhere in the port plan — flagged, not invented a home |
| Real per-domain download workflows (Paper/Xbox-jar/plugins/modpacks) | No — only the shared staging primitive (P3.14) is | Phases 7–9 |
| `SecretStore`-into-real-pairing wiring | No | Undecided — Cameron to choose end of Phase 3, start of Phase 4, or its own port-plan line |
| Remaining D-012 gaps (local auto-auth, remote pairing, LAN TLS, Tailscale, CSRF) | No | Untouched, same as Phase 2 left them |
