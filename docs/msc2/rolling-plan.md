# MSC 2 — Rolling Plan

> ## STATUS: Phase 15 is the priority next phase; Phase 14 is paused with P14.4, P14.5, P14.6, P14.9, P14.10, P14.11, P14.12, P14.15, P14.16, P14.17, P14.18, P14.19, P14.26, P14.27, P14.28, P14.29, P14.30, P14.31, P14.32, P14.33, P14.34, P14.35, P14.36, and P14.37 awaiting verification. P14.38 records twelve unverified static-review findings and is awaiting owner triage.
> **Next move:** Cameron verifies P15.3, P15.4, and P15.5. Phase 14 verification and P14.38 triage remain recorded and paused until Phase 15 is complete or Cameron explicitly resumes Phase 14. The current workspace has an unrelated pre-existing `dead_code` failure in `crates/msc-application/tests/provisioning.rs:152`. Phase 12 visual parity, anti-slop review, release/update handoff, and Bedrock product acceptance are recorded complete on 2026-09-08. P12.121–P12.189 are archived below with all verification entries recorded as DONE. The planned Phase 13 full-screen terminal client remains retired by D-034.

The detailed Phase 12 working plan is preserved in `rolling-plan-archive.md` under “Reconciliation snapshot — 2026-09-08”. This file contains only the current status and next move.

---

## How this document works

This is the working state of the build. The vision documents say where MSC 2 is going; the port plan says the intended sequence; this file says where the repository actually is now.

Phases come from `msc2-port-plan.md`. Steps are written as work arrives rather than being invented in advance. Each step has a status, file scope, description, verification command, commit subject, and batch classification.

Phase 12 is complete. Phase 15 is the priority next phase and is planned for execution before Phase 14 resumes. Phase 14 is paused, with its outstanding verification, release, console, and static-review work preserved below.

## Current phase

| Phase | Name | State |
|---|---|---|
| Setup | Repo, docs, agent instructions, CI, editor config | complete |
| 0 | Freeze the baseline and build the harness | complete |
| 1 | Domain types and pure rules | complete |
| 2 | API contract and operation model | complete |
| 3 | Safety substrate | complete |
| 4 | Java lifecycle vertical slice | complete |
| 5 | Configuration and migration | complete |
| 6 | Worlds and backups | complete |
| 7 | Server families and provisioning | complete |
| 8 | Mods, plugins, modpacks | complete |
| 9 | Networking and helpers | complete |
| 10 | Bedrock runtimes | complete |
| 11 | Desktop and web clients | complete |
| 12 | Client redesign and post-phase corrections | complete |
| 13 | Full-screen terminal client | retired by D-034 |
| 15 | Java, modpack import, components, and console usability fixes | priority next phase |
| 14 | Operational refinements: time, console, packaging, and remote hosts | paused |

## Proposed Phase 15 — Java and modpack workflow fixes

This phase is the priority next phase. The following scope summary is recorded
verbatim from Cameron's approved recap:

**Scope amendment (2026-09-20):** ATM10 Lite is the discovery case, not the
product boundary. Java selection, CurseForge credential handling, unresolved
file recovery, notes, Components search, and console behavior must work for
every supported modpack and provider. ATM10 is the first end-to-end acceptance
case because it exposed the problems.

Yes. I’d treat this as six bounded improvements, not one large “modpack issue.”

## Recommended order

### 1. Fix Java selection during server creation

This is the most fundamental problem because it blocks creating a server.

The create-server flow should:

- determine the required Java major from the Minecraft version and loader;
- show detected compatible runtimes;
- offer “Install Java” when needed;
- require the user to explicitly select a runtime before continuing;
- assign that runtime to the new server only.

The same Java-selection component should be reused by onboarding.

The detected-runtime sheet also needs a visual correction: dark MSC rows, restrained typography, secondary paths, and normal MSC buttons matching the existing “Install Java” sheet.

### 2. Add CurseForge key setup inside import

When a CurseForge manifest needs an API key:

- explain why the key is needed;
- provide a direct “Open CurseForge API Console” link;
- allow the key to be entered and saved in the sheet;
- resume the import after saving;
- offer the same setup again during unresolved-file recovery;
- allow the user to skip.

The important principle is: missing credentials should be recoverable inside the flow, not discovered only after a failed import.

### 3. Build the missing-mod recovery flow

After importing a modpack, MSC should show an actual inventory of unresolved files:

- mod name;
- filename;
- provider;
- reason it was not downloaded;
- download/project link.

Then MSC should:

1. Look for confident matches on Modrinth.
2. Download exact compatible matches automatically.
3. List everything still unresolved.
4. Let the user open selected or all download links.
5. Provide a drag-and-drop area for downloaded JARs.
6. Check each dropped JAR and confirm whether it is the expected file.
7. Let the user skip unresolved files.
8. Write the remaining missing mods into the server’s Overview notes.
9. Preserve the unresolved state so the user can return later.

The app should never silently substitute an approximate mod. Automatic Modrinth downloads should require a trustworthy identity and compatibility match.

### 4. Improve the Components tab

Add:

- a visible installed-mod count;
- search by mod name and filename;
- clear states for installed, missing, unresolved, and disabled components.

This should make it unnecessary to manually compare the manifest against the component list.

### 5. Quiet the ATM10 console

MSC’s own TPS and dimension-monitoring traffic should not appear as noisy human console output.

Recommended behavior:

- preserve the metrics in the metrics/statistics UI;
- keep real server messages in the console;
- hide or separately classify MSC-generated monitoring commands and responses;
- ensure monitoring cannot push useful human console history out of the buffer.

This should be handled as a general console/telemetry rule, not an ATM10-only exception.

### 6. Review the complete flow

The end-to-end acceptance scenario should be:

- Existing Fabric 1.20.1 server continues using Java 17.
- User creates ATM10 Lite / NeoForge 1.21.1.
- MSC requires Java 21 and lets the user detect, install, and select it.
- CurseForge import asks for the missing key inline.
- Unavailable mods are listed clearly.
- Modrinth matches download automatically.
- Remaining mods can be opened, dragged in, verified, or skipped.
- Missing files appear in server notes.
- Components shows a count and supports search.
- Console remains readable while metrics continue working.

## How I’d sequence the actual work

1. Java runtime selection and shared visual component.
2. CurseForge credential prompt.
3. Unresolved modpack recovery.
4. Components count/search.
5. Console telemetry separation.
6. Final visual and end-to-end review.

The current rolling plan says the outstanding Phase 14 verification and triage should happen before new implementation work is scheduled. Once that is cleared, these can be added as separate, narrowly scoped steps rather than mixed into one risky change.

### P15.1 — Move Java selection into server creation

- **Status:** awaiting verification
- **Files:** `crates/msc-domain/src/java_runtime.rs`, `crates/msc-application/src/provisioning.rs`, `crates/msc-agent/src/routes/servers.rs`, `crates/msc-agent/src/routes/versions.rs`, `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, `clients/desktop-web/src/lib/sections/fleet/wizard/model.ts`, `clients/desktop-web/src/lib/sections/server-editor/JavaInstallSheet.svelte`, `clients/desktop-web/src/lib/sections/server-editor/JavaTab.svelte`, `clients/desktop-web/src/lib/help/SetupIntro.svelte`
- **What:** Make Java selection a required step after the Minecraft version and loader are known. Reuse detection and installation, assign the chosen runtime to the new server, block continuation without an explicit selection, and reuse the same flow during onboarding. Restyle the detected-runtime sheet to match the dark MSC Install Java sheet.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-domain -p msc-application -p msc-agent && npm --prefix clients/desktop-web run check`
- **Batch:** A — Java selection and presentation
- **Commit:** `P15.1: require Java selection during server creation`

### P15.2 — Add inline CurseForge API-key setup

- **Status:** awaiting verification
- **Files:** `crates/msc-domain/src/app_config_schema.rs`, `crates/msc-application/src/curseforge_manual.rs`, `crates/msc-application/src/modpacks.rs`, `crates/msc-agent/src/routes/components.rs`, `clients/desktop-web/src/lib/sections/components/ImportModpackSheet.svelte`, `clients/desktop-web/src/lib/sections/components/CurseForgeManualDownloadSheet.svelte`, `clients/desktop-web/src/lib/sections/app-settings/AppSettingsSheet.svelte`
- **What:** Detect a missing CurseForge key before manifest import fails. Show the reason, provide the approved CurseForge API Console link, save the key from the sheet, resume the import, and allow the prompt to be skipped or reopened during manual-file recovery.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-application -p msc-agent && npm --prefix clients/desktop-web run check`
- **Batch:** B — provider credentials
- **Commit:** `P15.2: add inline CurseForge key setup`

### P15.3 — Complete unresolved modpack files

- **Status:** awaiting verification
- **Files:** `crates/msc-domain/src/modpack_manifest.rs`, `crates/msc-domain/src/modpack.rs`, `crates/msc-application/src/modpacks.rs`, `crates/msc-application/src/curseforge_manual.rs`, `crates/msc-application/src/addon_updates.rs`, `crates/msc-agent/src/routes/components.rs`, `clients/desktop-web/src/lib/sections/components/ImportModpackSheet.svelte`, `clients/desktop-web/src/lib/sections/components/CurseForgeManualDownloadSheet.svelte`, `clients/desktop-web/src/lib/sections/components/ProjectDetailSheet.svelte`, `clients/desktop-web/src/lib/sections/home/notes.ts`
- **What:** Return named unresolved files with reasons and links. Resolve confident Modrinth matches automatically, expose remaining provider links, validate dragged-in JARs against the expected file, support skip and retry, and persist the remaining list in the server Overview notes without using notes as the structured source of truth.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-domain -p msc-application -p msc-agent && npm --prefix clients/desktop-web run check`
- **Batch:** C — unresolved modpack recovery
- **Commit:** `P15.3: finish unresolved modpack files`

### P15.4 — Add Components count and search

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`, `clients/desktop-web/src/lib/sections/components/model.ts`, `clients/desktop-web/src/lib/sections/shared/types.ts`, `crates/msc-agent/src/routes/components.rs`
- **What:** Show the installed mod count, search by name and filename, and distinguish installed, missing, unresolved, and disabled components in the Components tab.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** D — component discoverability
- **Commit:** `P15.4: add component count and search`

### P15.5 — Keep monitoring traffic out of human console history

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/console_buffer.rs`, `crates/msc-application/src/output_reducer.rs`, `crates/msc-agent/src/ws/console.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `clients/desktop-web/src/lib/sections/console/ConsoleSection.svelte`, `clients/desktop-web/src/lib/sections/console/model.ts`, `clients/desktop-web/src/lib/components/shell/ConsoleDock.svelte`
- **What:** Build on P14.35–P14.37 so TPS, dimension, and other MSC-generated monitoring output is classified separately from genuine server output. Keep metrics working, preserve useful console history, and make automatic diagnostics available without flooding the human console. Do not add an ATM10-only exception.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-infrastructure -p msc-application -p msc-agent && npm --prefix clients/desktop-web run check`
- **Batch:** E — console clarity
- **Commit:** `P15.5: separate monitoring traffic from console history`

### P15.6 — Review the ATM10 end-to-end flow

- **Status:** awaiting verification
- **Files:** `docs/msc2/capabilities/phase15-acceptance.md`, `docs/msc2/rolling-plan.md`, `crates/msc-domain/src/java_runtime.rs`, `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, `clients/desktop-web/src/lib/sections/components/ImportModpackSheet.svelte`, `clients/desktop-web/src/lib/sections/components/CurseForgeManualDownloadSheet.svelte`, `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`, `clients/desktop-web/src/lib/sections/console/ConsoleSection.svelte`, `clients/desktop-web/src/lib/sections/server-editor/JavaInstallSheet.svelte`
- **What:** Use the Fabric 1.20.1/Java 17 and ATM10 Lite/NeoForge 1.21.1/Java 21 scenario as the first acceptance case, then review the same Java and modpack-import behavior for every supported modpack and provider. Correct the shared 1.20.5+ Java requirement and keep unresolved-file recovery provider-neutral. Record the inline key setup, unresolved-file recovery, drag-and-drop validation, skip-and-notes behavior, Components search, readable console, Java-sheet visual parity, and the remaining owner-verification boundary before closing the phase.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-domain -p msc-application -p msc-agent && npm --prefix clients/desktop-web run check`
- **Batch:** F — end-to-end review
- **Commit:** `P15.6: record ATM10 workflow acceptance`

### Additional Phase 15 scope — server state and interaction fixes

The following discussion summary is recorded verbatim:

### 1. Server notes are currently client-local

The notes are stored in browser/Tauri `localStorage` under a host-and-server key. The UI even says “Visible only in this app.”

They should become server-owned metadata:

- stored by the MSC agent with the server;
- available to every connected client;
- read and written through the API;
- still scoped to the selected server;
- existing local notes should receive a one-time migration or merge path.

I recommend the server becoming authoritative for all future edits.

### 2. Current app size

I measured the open MSC window:

**1240 × 760 pixels**

The configured default is currently **1100 × 760** in [`tauri.conf.json`](/Users/camerontemple/msc2/clients/desktop-web/src-tauri/tauri.conf.json:17).

So the likely change is:

- default width: `1240`;
- default height: `760`;
- only affect the initial/default window size, not force-resize users who already customized their window.

### 3. Modrinth browser scrollbar

The Modrinth browser has its own scroll container inside `PluginBrowserSheet.svelte`. We can hide the visible scrollbar while preserving mouse-wheel, trackpad, keyboard, and touch scrolling.

This is a small visual fix.

### 4. Minecraft day/time regression

The day changing from 34 to 49 means the existing relative-time fix is still incorrect in practice.

This should be treated as a correction to the existing Phase 14 time work, not a new feature:

- Dawn, dusk, and night must target the current Minecraft day;
- the day number must not change;
- the calculation must use the server’s current absolute time;
- the behavior must work consistently across supported runtimes.

This needs to be revisited before Phase 14 can be considered correct.

### 5. Live world time in the Active World card

Add a compact world-time line inside the existing Active World card without increasing its size.

For example:

`Day 34 · 18:42`

It should:

- update while the server is connected;
- use Minecraft world time, not the computer’s clock;
- remain compact enough to preserve the current card dimensions;
- show an unavailable/disconnected state when the server cannot provide it.

### 6. Sheets should not close when clicking outside

I found the shared cause: [`Sheet.svelte`](/Users/camerontemple/msc2/clients/desktop-web/src/lib/components/base/Sheet.svelte:19) currently closes whenever the click lands on the backdrop.

The better global behavior is:

- clicking outside a sheet does nothing;
- the close button still closes it;
- Escape still closes it;
- Cancel buttons still work;
- text selection or accidental mouse release cannot destroy the current sheet.

You do not need to enumerate every sheet. We can audit the shared `Sheet` component plus custom overlays such as confirmation dialogs and menus. Examples are only useful if a particular sheet behaves differently from the shared pattern.

I’d group these into three future work areas:

1. Server-owned notes and live world-time data.
2. Correct Minecraft day-preserving time actions.
3. Window, scrollbar, and sheet interaction polish.

The ATM10 discovery work and this additional usability work share Phase 15, but the time correction remains explicitly tied back to the paused Phase 14 implementation so it cannot be treated as finished merely because the new client controls exist.

### P15.7 — Make server notes host-owned

- **Status:** awaiting verification
- **Files:** `crates/msc-api/src/dto/lifecycle.rs`, `crates/msc-agent/src/routes/servers.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/sections/home/HomeSection.svelte`, `clients/desktop-web/src/lib/sections/home/notes.ts`, `clients/desktop-web/src/lib/sections/components/CurseForgeManualDownloadSheet.svelte`
- **What:** Add server-scoped notes to the agent-owned server contract, provide read/write API behavior, replace client-local storage, and migrate or merge existing local notes once. Keep the unresolved-modpack note block compatible with the same server-owned field.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-api -p msc-application -p msc-agent && npm --prefix clients/desktop-web run check`
- **Batch:** G — server-owned notes
- **Commit:** `P15.7: move server notes to the agent`

### P15.8 — Reconcile same-day time behavior

- **Status:** awaiting verification
- **Files:** `crates/msc-domain/src/time.rs`, `crates/msc-agent/src/routes/commands.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, `clients/desktop-web/src/lib/components/shell/ConsoleDock.svelte`, `clients/desktop-web/src/lib/sections/console/CommandPaletteSheet.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Correct the P14 relative-time implementation after the observed day 34 → 49 regression. The previous route used total `gametime`, which can be ahead of the daylight-cycle day; it now queries live `day` and `daytime`, derives the absolute target from the daylight-cycle day, rejects stale/unmatched query lines, and preserves that day across Java and Bedrock where supported. The acceptance rule is: Dawn, Dusk, and Night may change only the time-of-day; after each action, the world remains on the same Minecraft day, while raw numeric `time set` remains an explicit absolute day-changing command.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-domain -p msc-agent && npm --prefix clients/desktop-web run check`
- **Batch:** H — same-day time correction
- **Commit:** `P15.8: preserve the current Minecraft day`

### P15.9 — Show live world time in Active World

- **Status:** awaiting verification
- **Files:** `crates/msc-api/src/dto/status.rs`, `crates/msc-api/tests/dto_conformance.rs`, `crates/msc-agent/src/routes/commands.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/src/routes/performance.rs`, `crates/msc-agent/src/routes/status.rs`, `crates/msc-domain/src/time.rs`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/sections/home/HomeSection.svelte`, `clients/desktop-web/src/lib/sections/home/ActiveWorldCard.svelte`
- **What:** Expose the active world’s current Minecraft day and time through the existing live status/performance path. Render one compact line inside the existing Active World card without increasing its dimensions, and show a clear unavailable state when the server is disconnected or cannot answer.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-api -p msc-agent -p msc-application && npm --prefix clients/desktop-web run check`
- **Batch:** G — server-owned notes
- **Commit:** `P15.9: show live active-world time`

### P15.10 — Set the default desktop window size

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src-tauri/tauri.conf.json`
- **What:** Change the Tauri default window size from 1100×760 to the captured current size, 1240×760. This changes the initial default only and must not force-resize a user’s existing customized window.
- **Verify:** `python3 -m json.tool clients/desktop-web/src-tauri/tauri.conf.json >/dev/null`
- **Batch:** I — desktop presentation polish
- **Commit:** `P15.10: set the current desktop window default`

### P15.11 — Hide the Modrinth browse scrollbar

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/components/PluginBrowserSheet.svelte`
- **What:** Hide the visible scrollbar in the Modrinth browse results while preserving scrolling with the wheel, trackpad, keyboard, and touch input. Keep the results container bounded and usable at the current sheet size.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** I — desktop presentation polish
- **Commit:** `P15.11: hide the Modrinth browse scrollbar`

### P15.12 — Stop sheets closing on backdrop clicks

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/components/base/Sheet.svelte`, `clients/desktop-web/src/lib/components/ConfirmDialog.svelte`, `clients/desktop-web/src/lib/components/base/Menu.svelte`, `clients/desktop-web/src/lib/components/shell/ConsoleDock.svelte`, `clients/desktop-web/src/lib/sections/components/ImportModpackSheet.svelte`, `clients/desktop-web/src/lib/sections/server-editor/ServerEditorSheet.svelte`
- **What:** Make outside-click dismissal opt-in rather than the default for sheets. Clicking the scrim must leave an open sheet and its in-progress text untouched; explicit close buttons, Cancel actions, and Escape remain available. Audit custom overlays separately so transient menus and intentional confirmation behavior are not changed accidentally.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && rg -n "dismissOnBackdrop|event\.target === event\.currentTarget|onclick=.*onClose" clients/desktop-web/src/lib/components clients/desktop-web/src/lib/sections`
- **Batch:** I — desktop interaction polish
- **Commit:** `P15.12: make sheet dismissal explicit`

### P15.13 — Review the additional Phase 15 acceptance flow

- **Status:** awaiting verification
- **Files:** `docs/msc2/capabilities/phase15-acceptance.md`, `docs/msc2/rolling-plan.md`, `clients/desktop-web/src/lib/sections/home/HomeSection.svelte`, `clients/desktop-web/src/lib/sections/home/ActiveWorldCard.svelte`, `clients/desktop-web/src/lib/components/base/Sheet.svelte`, `clients/desktop-web/src-tauri/tauri.conf.json`
- **What:** Record acceptance evidence for server notes visible from a second client, the 1240×760 initial window, same-day Dawn/Dusk/Night behavior, live Active World time without card growth, hidden Modrinth scrollbar, and sheets surviving outside clicks. Confirm the new behavior does not regress explicit close, Cancel, Escape, or existing modpack note updates.
- **Verify:** `cargo check -p msc-agent -p msc-application && npm --prefix clients/desktop-web run check`
- **Batch:** J — additional Phase 15 review
- **Commit:** `P15.13: record additional Phase 15 acceptance`

### P15.14 — Show the modpack creation summary

- **Status:** awaiting verification
- **Files:** `crates/msc-agent/src/main.rs`, `crates/msc-agent/src/routes/components.rs`, `crates/msc-agent/src/routes/servers.rs`, `clients/desktop-web/src/lib/sections/fleet/wizard/model.ts`, `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, `clients/desktop-web/src/lib/sections/fleet/wizard/ModpackCreationSummarySheet.svelte`, `clients/desktop-web/src/lib/sections/components/CurseForgeManualDownloadSheet.svelte`
- **What:** Preserve the unresolved-file report when a new server is created from a modpack, share that recovery state with the Components routes, return a pack summary in the completed create operation, and show a post-creation sheet with unresolved files first, downloaded filenames below, and a direct handoff to the existing manual recovery flow. Keep unresolved files available after the summary closes and write them to server notes.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-agent -p msc-application && cargo clippy -p msc-agent -p msc-application -- -D warnings && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** K — modpack creation handoff
- **Commit:** `P15.14: show modpack creation summary`

### P15.15 — Reconcile client-only recovery and provider links

- **Status:** awaiting verification
- **Files:** `crates/msc-application/src/modpacks.rs`, `crates/msc-agent/src/routes/servers.rs`, `clients/desktop-web/src/lib/sections/components/CurseForgeManualDownloadSheet.svelte`, `clients/desktop-web/src/lib/sections/fleet/wizard/model.ts`, `clients/desktop-web/src/lib/sections/fleet/wizard/ModpackCreationSummarySheet.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Check exact compatible Modrinth matches before manual recovery, classify client-only matches as intentionally absent from the server instead of unresolved, show the connected agent’s CurseForge-key status, route provider links through the external-browser bridge, and keep Choose/Skip actions adjacent.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-application -p msc-agent && cargo clippy -p msc-application -p msc-agent -- -D warnings && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** K — modpack recovery corrections
- **Commit:** `P15.15: reconcile client-only recovery and provider links`

### P15.16 — Clarify server files recovered during modpack import

- **Status:** awaiting verification
- **Files:** `crates/msc-application/src/modpacks.rs`, `crates/msc-agent/src/routes/servers.rs`, `clients/desktop-web/src/lib/sections/fleet/wizard/model.ts`, `clients/desktop-web/src/lib/sections/fleet/wizard/ModpackCreationSummarySheet.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Classify confident client-only CurseForge entries before downloading and silently skip them. Keep direct downloads separate from files recovered automatically from Modrinth, show the latter in a dedicated completion section, and remove the incomplete client-only list from the user-facing summary. Describe the remaining completion state as server files ready.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-application -p msc-agent && cargo clippy -p msc-application -p msc-agent -- -D warnings && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** K — modpack summary corrections
- **Commit:** `P15.16: clarify server files recovered during modpack import`

### P15.17 — Polish modpack completion summary wording

- **Status:** awaiting verification
- **Files:** `crates/msc-agent/src/routes/components.rs`, `clients/desktop-web/src/lib/sections/fleet/wizard/ModpackCreationSummarySheet.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Clarify that manual-download counts may change before import finishes, style the Modrinth recovery supporting sentence like the other summary subtitles, correct singular grammar, and remove the duplicate divider after the recovered-file list.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-agent && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** K — modpack summary corrections
- **Commit:** `P15.17: polish modpack completion summary wording`

### P15.18 — Simplify server-created confirmation status

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/ConfirmStep.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Keep the green created-state label but hide the decorative status dot, leaving a text-only confirmation in the final Add Server step.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** K — add-server confirmation polish
- **Commit:** `P15.18: simplify server-created confirmation status`

### P15.19 — Increase server-created confirmation emphasis

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/ConfirmStep.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Increase the text-only created-state label to 18px with a stronger weight so the final confirmation uses the available space more deliberately without restoring the decorative dot.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** K — add-server confirmation polish
- **Commit:** `P15.19: increase server-created confirmation emphasis`

### P15.20 — Keep client-only overrides out of the server and count local mods

- **Status:** awaiting verification
- **Files:** `crates/msc-application/src/modpacks.rs`, `clients/desktop-web/src/lib/sections/components/model.ts`, `docs/msc2/rolling-plan.md`
- **What:** Identify client-only override jars before merging pack overrides into the server, preserve the existing post-merge safeguard for already-present files, and treat active local jars without a provider link as installed in Components. Pending import files remain unresolved, and disabled jars remain disabled.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-application -p msc-agent && cargo clippy -p msc-application -p msc-agent -- -D warnings && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** K — modpack and Components inventory corrections
- **Commit:** `P15.20: count local mods and skip client-only overrides`

### P15.21 — Skip client-only CurseForge files before download

- **Status:** awaiting verification
- **Files:** `crates/msc-application/src/modpacks.rs`, `docs/msc2/rolling-plan.md`
- **What:** Apply a confident Modrinth project's `server_side: unsupported` classification before requiring an exact compatible filename match. This lets the CurseForge manifest import skip client-only projects before downloading them or counting them as installed; the post-download classifier remains a safeguard for content it could not confidently classify earlier.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-application -p msc-agent && cargo clippy -p msc-application -p msc-agent -- -D warnings`
- **Batch:** K — modpack and Components inventory corrections
- **Commit:** `P15.21: skip client-only CurseForge files before download`

### P15.22 — Keep client-only files and shader packs out of imported servers

- **Status:** awaiting verification
- **Files:** `crates/msc-domain/src/addon_provider.rs`, `crates/msc-infrastructure/src/addon_provider.rs`, `crates/msc-application/src/modpacks.rs`, `crates/msc-application/tests/modrinth_pack_import.rs`, `crates/msc-application/tests/curseforge_pack_import.rs`, `crates/msc-application/tests/modpack_server_creation.rs`, `docs/msc2/rolling-plan.md`
- **What:** Use CurseForge's exact SHA-1 file hashes for pre-download Modrinth identification; apply the same exact-hash client-only check to Modrinth manifests; exclude shader/resource-pack paths and non-JAR files in `mods/` before merge/download; inspect newly written JARs afterward and delete identified client-only files instead of leaving disabled files; remove deleted files from import counts, and fail the import if cleanup itself fails.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-application -p msc-infrastructure -p msc-domain -p msc-agent && cargo clippy -p msc-application -p msc-infrastructure -p msc-domain -p msc-agent -- -D warnings`
- **Batch:** K — modpack and Components inventory corrections
- **Commit:** `P15.22: exclude client-only files and shader packs`

### Additional Phase 15 scope — world packs and modpack identity

This scope and its planned implementation steps are recorded here for Phase 15.

## Overall scope

We are adding three focused concepts:

1. Java datapacks.
2. Modpack identity for servers created from modpacks.
3. Bedrock behavior packs.

Client-only content is deliberately out of scope:

- Java shader packs;
- Java client resource packs;
- Bedrock resource-pack browsing;
- skin packs;
- client-only mods.

Resource-pack dependencies still need to be detected where required for a behavior pack to function correctly. If the linked resource pack is included with the downloaded add-on, MSC installs the complete add-on together. If it is not included, MSC stops and explains the missing dependency; it does not install an incomplete behavior pack or provide a separate resource-pack browser.

## Java Worlds tab

The Java Worlds tab should be organized as:

1. World Slots
2. Datapacks
3. Backups

### Datapacks section

The Datapacks section belongs to the selected world slot because Java datapacks are world-owned. Vanilla Java stores them inside the world’s `datapacks/` directory. [Mojang’s datapack documentation](https://www.minecraft.net/de-de/article/minecraft-snapshot-17w43a)

The section should show:

- datapacks installed in the selected slot;
- datapack name and version;
- Minecraft-version compatibility;
- enabled or disabled state;
- provider and source information;
- update availability when known.

It should include a:

```text
Browse Datapacks
```

button, similar to the existing Browse Mods button.

The first provider is Modrinth, which already has a Data Packs catalog. [Modrinth Data Packs](https://modrinth.com/discover/datapacks)

### Datapack installation behavior

Installing a datapack should:

- target the currently selected world slot;
- download and validate the datapack archive;
- verify its Java pack metadata;
- check Minecraft-version compatibility;
- prevent archive path traversal or malformed contents;
- preserve provider, version, and hash information;
- create a backup before changing the world;
- require the server to be stopped when necessary;
- make the datapack travel with the world slot.

A datapack installed into World Slot A should not appear in World Slot B. It should follow the slot through:

- activation;
- duplication;
- backup;
- restore;
- export;
- import.

Later controls should support:

- enable;
- disable;
- update;
- remove;
- reload or restart when required.

World-generation datapacks deserve a warning because removing one does not undo terrain already generated using it.

If no world slot is selected, the section should explain that the user must select one first.

## Modpack identity in Components

Modpack installation already happens during server creation or import. The new behavior is simply to make the imported pack visible afterward.

At the top of the Components tab, servers created from modpacks should show a compact read-only summary such as:

```text
Modpack
All the Mods 10 Lite
Version 1.0.x · CurseForge
```

The summary should:

- appear above the component list;
- show the pack name;
- show the imported pack version;
- show the provider, such as CurseForge or Modrinth;
- remain compact;
- be persisted as server metadata;
- be absent for ordinary non-modpack servers;
- not duplicate the full list of installed mods.

The Components tab remains responsible for the individual mods. The modpack summary only answers:

> What pack is this server based on?

If a server was imported from a modpack but the metadata is missing, MSC should not guess from the installed mods. It should either omit the summary or show that the source metadata is unavailable.

## Bedrock Worlds tab

For Bedrock Dedicated Server, the Worlds tab should use the same structure with an edition-specific second section:

1. World Slots
2. Behavior Packs
3. Backups

### Behavior Packs section

Behavior packs are the closest Bedrock equivalent to Java datapacks. They can change gameplay behavior, entities, items, recipes, loot, spawning, trades, functions, and scripts. [Microsoft’s behavior-pack documentation](https://learn.microsoft.com/en-us/minecraft/creator/documents/behaviorpackfromscratch?view=minecraft-bedrock-stable)

The section should include:

```text
Browse Behavior Packs
```

It should show behavior packs for the selected Bedrock world slot, including:

- name;
- version;
- minimum Bedrock version;
- enabled or disabled state;
- provider and source;
- UUID;
- dependencies;
- update availability when known.

### Bedrock world-scoped decision

Bedrock Dedicated Server can technically keep packs in shared server-level folders or inside an individual world folder. We are intentionally choosing world-scoped behavior packs for MSC because it is simpler and matches the World Slots model.

That means:

- Behavior Pack A installed for World Slot A does not automatically affect World Slot B.
- Activating another slot changes the visible behavior-pack list.
- Duplicating or backing up a slot includes its behavior-pack state.
- Restoring a slot restores the pack configuration with it.

The shared BDS folders can remain an implementation detail or future feature. They are not part of this first design. [Bedrock Dedicated Server pack layout](https://learn.microsoft.com/en-us/minecraft/creator/documents/bedrockserver/getting-started?view=minecraft-bedrock-stable)

### Behavior-pack dependencies

Some behavior packs require a linked resource pack. We are not building a resource-pack browser, but MSC must still inspect and explain that dependency.

If the linked resource pack is included with the downloaded add-on, install it together with the behavior pack. Otherwise stop and explain that the behavior pack cannot be installed alone. Do not silently install an incomplete behavior pack.

MSC must not silently install an incomplete behavior pack. Bedrock manifests explicitly support pack dependencies. [Bedrock manifest reference](https://learn.microsoft.com/en-us/minecraft/creator/reference/content/addonsreference/packmanifest?view=minecraft-bedrock-stable)

## Shared implementation shape

The backend can share a general world-pack model, but the user-facing names and validation rules should remain edition-specific:

```text
Java world pack       → datapack
Bedrock world pack    → behavior pack
```

Each installed pack should retain:

- pack kind;
- world-slot ID;
- provider;
- project ID or URL;
- version;
- file name;
- checksum;
- compatibility information;
- enabled state;
- dependency information.

The installation system should remain separate from the Java mod/plugin system because the destinations and lifecycles differ:

- Java mods/plugins are server-owned runtime components.
- Java datapacks are world-owned.
- Bedrock behavior packs are world-owned in MSC, even though BDS supports shared storage.
- Modpacks are distribution metadata plus collections of other components.

## Resulting MSC experience

A Java server would expose:

```text
Worlds
├── World Slots
├── Datapacks
└── Backups

Components
└── Imported Modpack summary
    └── Mods and other installed components
```

A Bedrock server would expose:

```text
Worlds
├── World Slots
├── Behavior Packs
└── Backups
```

This gives Java and Bedrock parallel concepts without pretending their underlying pack systems are identical.

### P15.23 — Freeze the world-pack and modpack-identity contract

- **Status:** awaiting verification
- **Files:** `docs/msc2/msc2-decisions.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Map each requirement to its owner, API boundary, source files, and acceptance evidence in `docs/msc2/capabilities/phase15-world-packs.md`. Record that datapacks and behavior packs belong to a world slot, identify how slot activation, duplication, backup, restore, export, and import preserve pack state, define server-owned modpack identity and the no-guessing behavior, set Modrinth as the first Java datapack provider, and record the Bedrock rule for bundled linked resource packs. D-030 is now owner-confirmed; the remaining Phase 15 pack contract stays proposed. Explicitly keep client-only packs and separate Bedrock resource-pack browsing out of scope, and identify enable/disable/update/remove as later controls unless the acceptance map shows they are required for the first usable release.
- **Verify:** `rg -n "world slot|Modrinth|linked resource pack|modpack identity|D-030|enable|disable|update|remove" docs/msc2/capabilities/phase15-world-packs.md docs/msc2/msc2-decisions.md docs/msc2/msc2-engineering.md`
- **Batch:** L — world-pack contract and acceptance map
- **Commit:** `P15.23: define world-pack contracts`

### P15.24 — Persist modpack identity with the server

- **Status:** awaiting verification
- **Files:** `crates/msc-domain/src/modpack.rs`, `crates/msc-domain/src/app_config_schema.rs`, `crates/msc-infrastructure/src/config_repository.rs`, `crates/msc-application/src/modpacks.rs`, `crates/msc-application/src/provisioning.rs`, `crates/msc-api/src/dto/lifecycle.rs`, `crates/msc-agent/src/routes/servers.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/src/routes/components.rs`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/msc2-decisions.md`, `docs/msc2/msc2-engineering.md`
- **What:** Preserve the source pack name, provider, and imported version as server-owned metadata for servers created or imported from a modpack. Expose that metadata through the API, keep it with the server through ordinary server operations, and represent unavailable source metadata honestly. Do not infer pack identity from the installed component list; ordinary servers remain without a modpack identity.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-domain -p msc-application -p msc-agent && cargo clippy -p msc-domain -p msc-application -p msc-agent -- -D warnings`
- **Batch:** M — modpack identity
- **Commit:** `P15.24: persist modpack source identity`

### P15.25 — Add world-slot pack metadata and lifecycle support

- **Status:** awaiting verification
- **Files:** `crates/msc-domain/src/world_profile.rs`, `crates/msc-domain/src/backup.rs`, `crates/msc-application/src/worlds.rs`, `crates/msc-application/src/backups.rs`, `crates/msc-infrastructure/src/archive.rs`, `crates/msc-infrastructure/src/world_store.rs`, `crates/msc-api/src/dto/worlds.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-agent/src/routes/backups.rs`, `crates/msc-agent/src/routes/servers.rs`, `crates/msc-application/tests/backup_inventory.rs`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Add edition-aware, world-slot-scoped pack records to the existing slot-profile API without routing world packs through the Java mod/plugin system. Keep pack files under the owning world root and carry provider, version, checksum, compatibility, enabled state, and dependency metadata through activation, duplication, backup, restore, export, and import. Backups retain the profile in their sidecar; exported archives carry it in a reserved entry that is not extracted into a Minecraft world. Preserve Java datapack and Bedrock behavior-pack validation as edition-specific rules, and ensure one slot's packs cannot appear in another slot.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-domain -p msc-api -p msc-infrastructure -p msc-application -p msc-agent && cargo clippy -p msc-domain -p msc-api -p msc-infrastructure -p msc-application -p msc-agent -- -D warnings`
- **Batch:** N — world-slot pack model and persistence
- **Commit:** `P15.25: persist packs with world slots`

### P15.26 — Browse and install Java datapacks

- **Status:** awaiting verification
- **Files:** `crates/msc-domain/src/addon_provider.rs`, `crates/msc-infrastructure/src/addon_provider.rs`, `crates/msc-application/src/addons.rs`, `crates/msc-agent/src/routes/components.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-api/src/dto/addons.rs`, `crates/msc-api/src/dto/worlds.rs`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Add the Modrinth Data Packs catalog and installation path for the selected Java world slot. Validate downloaded archives and Java pack metadata, check Minecraft-version compatibility, reject malformed or path-traversing archives, retain provider/version/hash details, back up the world before mutation, and require a stopped server when live changes are unsafe. Report enabled state and update availability when known; follow the P15.23 contract for controls deferred from the first release.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-domain -p msc-api -p msc-infrastructure -p msc-application -p msc-agent && cargo clippy -p msc-domain -p msc-api -p msc-infrastructure -p msc-application -p msc-agent -- -D warnings`
- **Batch:** O — Java datapacks
- **Commit:** `P15.26: add Java datapack installation`

### P15.27 — Browse and install Bedrock behavior packs

- **Status:** awaiting verification
- **Files:** `crates/msc-domain/src/bedrock.rs`, `crates/msc-infrastructure/src/addon_provider.rs`, `crates/msc-application/src/addons.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-api/src/dto/worlds.rs`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Browse CurseForge's Bedrock Addons catalog using the existing host-side key, then install a selected file into the chosen world slot. Validate archive paths and manifests, pack/module/dependency UUIDs and versions, and minimum Bedrock version. Install linked resource packs only when bundled; otherwise explain the missing pack and leave the world unchanged. Keep files and pack lists inside the owning world despite BDS's shared pack folders. The provider choice is Proposed until separately reviewed.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-domain -p msc-api -p msc-infrastructure -p msc-application -p msc-agent && cargo clippy -p msc-domain -p msc-api -p msc-infrastructure -p msc-application -p msc-agent -- -D warnings && python3 -m json.tool docs/msc2/api-contract/openapi.json >/dev/null`
- **Batch:** P — Bedrock behavior packs
- **Commit:** `P15.27: add Bedrock behavior-pack installation`

### P15.28 — Add world-pack and modpack views

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `clients/desktop-web/src/lib/sections/worlds/model.ts`, `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`, `clients/desktop-web/src/lib/sections/components/model.ts`, `clients/desktop-web/src/lib/sections/addons/AddonsSection.svelte`, `clients/desktop-web/src/lib/sections/addons/model.ts`, `clients/desktop-web/src/lib/api/generated.ts`, `docs/msc2/capabilities/phase15-world-packs.md`
- **What:** Organize Java Worlds as World Slots, Datapacks, and Backups, and Bedrock Worlds as World Slots, Behavior Packs, and Backups. Show packs for the selected slot with the agreed identity, compatibility, enabled state, source, dependency, and update information; provide the Java Browse Datapacks and Bedrock Browse Behavior Packs flows; explain when a slot must be selected or the server stopped. Show the compact read-only imported-modpack summary above Components without duplicating the component inventory.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** Q — world-pack and modpack client surfaces
- **Commit:** `P15.28: show world packs and modpack identity`

### P15.29 — Review world-pack portability and modpack identity

- **Status:** awaiting verification
- **Files:** `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`, `crates/msc-domain/src/world_profile.rs`, `crates/msc-application/src/worlds.rs`, `crates/msc-application/src/backups.rs`, `crates/msc-application/src/modpacks.rs`, `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, `clients/desktop-web/src/lib/sections/components/ComponentsSection.svelte`
- **What:** Review the Java and Bedrock pack flows against the phase acceptance map: slot isolation and portability across activation, duplication, backup, restore, export, and import; archive and manifest validation; incomplete Bedrock dependency handling; correct imported-modpack identity; and readable edition-specific Worlds views. Record static findings and the remaining owner-run live Minecraft checks. Do not close the phase until the documented acceptance evidence is complete.
- **Verify:** `rg -n "activation|duplication|backup|restore|export|import|path traversal|linked resource pack|modpack identity|owner verification" docs/msc2/capabilities/phase15-world-packs.md`
- **Batch:** R — world-pack acceptance review
- **Commit:** `P15.29: record world-pack acceptance`

### P15.30 — Preserve slot metadata when copying into an existing slot

- **Status:** awaiting verification
- **Files:** `crates/msc-application/src/worlds.rs`, `crates/msc-infrastructure/src/world_store.rs`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Make `copy_slot_into_existing` preserve a consistent destination archive and profile when metadata persistence fails. Stage the replacement archive and profile, stop ignoring `save_metadata` and `copy_profile` errors, and retain enough rollback state that a returned failure does not leave source world files paired with the destination's old pack profile. Record the failure boundary in the acceptance map. Do not add tests; use the declared static checks and Cameron's manual verification.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-application -p msc-agent && cargo clippy -p msc-application -p msc-agent -- -D warnings`
- **Batch:** S — slot-copy portability correction
- **Commit:** `P15.30: preserve slot profile on copy failure`

### P15.31 — Match the world-pack browser to the Components browser

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `clients/desktop-web/src/lib/sections/worlds/WorldsSection.svelte`, `clients/desktop-web/tests/screens/worlds-backups.test.ts`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Bring datapack and behavior-pack result browsing into line with the Components Modrinth browser: search field and quiet provider/version line at the top, icon-led flat result rows with author/download metadata and restrained descriptions, and a compact Add action. Keep the provider-specific install behavior and do not introduce nested result cards. Resolve the missing UI imports and update the existing world-profile fixture for the current schema so the static frontend check can run.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** T — world-pack browser presentation
- **Commit:** `P15.31: align world-pack browser presentation`

### P15.32 — Correct the CurseForge Bedrock category request

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/addon_provider.rs`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Use CurseForge's documented `/v1/categories` endpoint with the Bedrock game ID and `classesOnly=true` to resolve its Addons class. Keep the existing CurseForge provider choice; the 404 came from requesting a non-existent API path.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-infrastructure -p msc-agent && cargo clippy -p msc-infrastructure -p msc-agent -- -D warnings`
- **Batch:** U — Bedrock catalog endpoint correction
- **Commit:** `P15.32: correct Bedrock catalog category lookup`

### P15.33 — Identify the failing Bedrock catalog request

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/addon_provider.rs`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Preserve CurseForge's status and authorization errors while identifying whether the Bedrock class lookup or the subsequent add-on search failed. This makes another provider 404 actionable without exposing the API key.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-infrastructure -p msc-agent && cargo clippy -p msc-infrastructure -p msc-agent -- -D warnings`
- **Batch:** V — Bedrock catalog error diagnosis
- **Commit:** `P15.33: identify Bedrock catalog request failures`

### P15.34 — Use Minecraft's CurseForge game ID for Bedrock packs

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/addon_provider.rs`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Use Minecraft's CurseForge game ID (`432`) for both Bedrock Addons category lookup and search. Keep resolving the Addons class from provider metadata instead of hardcoding its class ID.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-infrastructure -p msc-agent && cargo clippy -p msc-infrastructure -p msc-agent -- -D warnings`
- **Batch:** W — Bedrock game identifier correction
- **Commit:** `P15.34: use Minecraft CurseForge game ID`

### P15.35 — Open Bedrock behavior-pack details

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Make each Bedrock behavior-pack result selectable, matching the Components browser's row-to-detail flow. Show the full available description, download count, Minecraft version, and pack filename in a detail sheet, with the existing Add operation available there.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** X — Bedrock behavior-pack details
- **Commit:** `P15.35: open Bedrock behavior-pack details`

### P15.36 — Match Bedrock pack details to the Mods browser

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/addon_provider.rs`, `crates/msc-agent/src/routes/worlds.rs`, `crates/msc-api/src/dto/worlds.rs`, `docs/msc2/api-contract/openapi.json`, `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/sections/components/model.ts`, `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Add an authenticated agent route that loads a Bedrock add-on's full CurseForge description, screenshots, and published files. Expand the detail sheet to show the gallery, safe formatted description, release types, and a selectable version list that highlights files tagged for the selected Minecraft version and installs the chosen file.
- **Verify:** `cargo fmt --all -- --check && cargo check -p msc-infrastructure -p msc-agent && cargo clippy -p msc-infrastructure -p msc-agent -- -D warnings && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run api:check`
- **Batch:** Y — Bedrock detail parity
- **Commit:** `P15.36: match Bedrock details to Mods browser`

### P15.37 — Mark incompatible packs in Bedrock search results

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Read the active Bedrock server version when the browser opens, keep other-version search results available for inspection, and offer direct Add only when the selected server version is known to match. Show an “Other version” or “Version unknown” status otherwise. Keep “Install anyway” available in the detail version list.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** Z — Bedrock result compatibility guard
- **Commit:** `P15.37: mark incompatible Bedrock results`

### P15.38 — Search compatible Bedrock packs first

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Pass the selected server's Bedrock version to CurseForge search by default so the result list contains compatible files. Add a “Show other versions” control and an empty-state action when no compatible results match; preserve the detail sheet's explicit “Install anyway” path.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** AA — compatible Bedrock catalog search
- **Commit:** `P15.38: search compatible Bedrock packs first`

### P15.39 — Show all Bedrock behavior packs across versions

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/worlds/WorldPackBrowserSheet.svelte`, `docs/msc2/capabilities/phase15-world-packs.md`, `docs/msc2/rolling-plan.md`
- **What:** Search the full CurseForge Bedrock add-on catalog without filtering by the selected server version. Keep all results selectable for details, show version compatibility in each result, withhold direct Add for a version mismatch, and preserve the detail view's explicit “Install anyway” action. Remove the redundant “Show other versions” toggle and its version-filtered empty state.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** AB — unfiltered Bedrock pack discovery
- **Commit:** `P15.39: show all Bedrock behavior packs`

### P15.40 — Wait for Java selection before advancing onboarding

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, `docs/msc2/rolling-plan.md`
- **What:** When Configure's Continue action opens required Java selection, defer both wizard progression and the onboarding tour's Continue action until a runtime is confirmed. Confirmation advances to Network and then reveals “How will friends connect?”; cancelling keeps the tour on Configure.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check`
- **Batch:** AC — Java selection and onboarding sequencing
- **Commit:** `P15.40: defer onboarding until Java is selected`

### P15.41 — Include the onboarding fix in the agent web bundle

- **Status:** awaiting verification
- **Files:** `crates/msc-agent/web-ui/**`, `docs/msc2/rolling-plan.md`
- **What:** Regenerate the agent's embedded production web UI from the current Svelte source so rebuilding or repairing the service serves P15.40's Java-selection/onboarding sequencing fix.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run bundle:identity`
- **Batch:** AD — package the onboarding correction
- **Commit:** `P15.41: package onboarding fix in agent UI`

### P15.42 — Advance the tour only after Java selection closes

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, `clients/desktop-web/src/lib/help/tourAnchors.ts`, `crates/msc-agent/web-ui/**`, `docs/msc2/rolling-plan.md`
- **What:** Remove the tour action anchor from Configure's Continue button while Java selection is pending. After a runtime is confirmed, advance to Network, wait for Svelte to remove the Java sheet, then dispatch the deferred tour action so “How will friends connect?” appears against the visible Network step. Keep Cancel on Configure without advancing the tour.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run bundle:identity`
- **Batch:** AE — sequence onboarding after Java selection
- **Commit:** `P15.42: advance onboarding after Java selection`

### P15.43 — Block the Configure tour action during Java selection

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/help/onboarding.ts`, `clients/desktop-web/src/lib/help/TourOverlay.svelte`, `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, `crates/msc-agent/web-ui/**`, `docs/msc2/rolling-plan.md`
- **What:** Make Java selection an explicit synchronous gate in the tour listener. Configure's Continue sets the gate before the browser reports its anchored click, so the tour cannot advance while the Java sheet is open. Confirming a Java runtime clears the gate only after that sheet closes, then advances the tour to Network. Bedrock retains its ordinary Continue behavior because it never sets the Java gate.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run bundle:identity`
- **Batch:** AF — Java selection blocks the onboarding action
- **Commit:** `P15.43: block onboarding during Java selection`

### P15.44 — Make wizard completion the only tour advance signal

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/components/base/Button.svelte`, `clients/desktop-web/src/lib/help/tourAnchors.ts`, `clients/desktop-web/src/lib/sections/fleet/wizard/AddServerWizard.svelte`, `crates/msc-agent/web-ui/**`, `docs/msc2/rolling-plan.md`
- **What:** Keep the wizard Continue button visible to the tour without allowing its browser click to advance the tour automatically. The wizard now sends the tour action only after it has completed its own transition. Required Java selection therefore leaves the tour on Configure; confirmation closes the Java sheet, advances to Network, then advances the tour. Bedrock continues directly because its transition completes immediately.
- **Verify:** `npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run bundle:identity`
- **Batch:** AG — make wizard transitions control onboarding
- **Commit:** `P15.44: sequence onboarding from wizard completion`

## Proposed Phase 14 — operational refinements

This phase turns the issues reported from real use into four bounded areas:
Minecraft time semantics, automatic-console output, cross-platform headless
installation, and a Tauri-managed connection flow for remote hosts. The remote
flow is deliberately detailed because it replaces several manual SSH, tunnel,
and pairing steps at once.

The management service port is **48001**. The desktop's local forwarded port is
**48002** in the examples below, but it is a client-side choice and must be
editable or automatically selected when occupied.

### P14.1 — Record the operational-refinement contracts

- **Status:** DONE
- **Files:** `docs/msc2/msc2-decisions.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/msc2-product.md`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/rolling-plan.md`
- **What:** Add the decisions that make the rest of the phase testable and prevent accidental scope drift:
  - time shortcuts are semantic “same Minecraft day” actions; exact day changes are separate and explicit; raw numeric `time set` remains an explicit absolute command;
  - the behavior must be shared by all supported Java flavors and Bedrock rather than being a Java-only UI trick;
  - automatic metric commands remain internal monitoring traffic and cannot displace human console history;
  - headless installation is a first-class macOS, Windows, and Linux contract, including a documented `msc` PATH entry;
  - a Tauri desktop may manage an SSH tunnel and perform the remote pairing bootstrap, but MSC does not operate a cloud relay or require Tailscale;
  - the remote client does not gain an API for installing, starting, stopping, replacing, or uninstalling the operating-system service; SSH setup actions must respect that boundary;
  - host identity is stable even when LAN and Tailscale addresses change, and credentials remain scoped to that host.
- **Verify:** `rg -n "same Minecraft day|48001|automatic metric|PATH|SSH tunnel|stable host|Tailscale" docs/msc2/msc2-decisions.md docs/msc2/msc2-engineering.md docs/msc2/msc2-product.md docs/msc2/api-contract/openapi.json`
- **Batch:** A — contracts and source map
- **Commit:** `P14.1: record operational refinement contracts`

### P14.2 — Build the source and acceptance matrix

- **Status:** DONE
- **Files:** `docs/msc2/rolling-plan.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/msc2-engineering.md`, `docs/msc2/capabilities/`, `clients/desktop-web/src/lib/hosts/`, `clients/desktop-web/src/lib/sections/setup/`, `packaging/`, `tools/release/`
- **What:** Map every reported behavior to its current implementation, MSC 1 oracle, API boundary, client surface, and platform acceptance evidence. The matrix must explicitly cover Java Vanilla/Paper/Purpur/Fabric/Forge/NeoForge, Bedrock, Tauri on macOS/Windows/Linux, the served browser client, and the headless CLI. It must call out which checks are static inspection, which are live Minecraft verification, and which require a real OS install. No implementation work starts until this map identifies the owning layer for each behavior.
- **Amendment (2026-09-11):** The first matrix wording was too narrow: its C1 row named periodic TPS/player/Spark polling but did not explicitly inventory backup save commands or helper-process output. P14.2 remains complete as the source-map step, but this correction expands C1 and adds C3/C4 in `docs/msc2/capabilities/phase14-operational-refinements.md`. The later console steps must therefore cover all MSC-generated traffic, not metrics alone.
- **Verify:** `rg -n "P14\.1|P14\.2|time|console|headless|SSH|pairing|48001|48002" docs/msc2/rolling-plan.md`
- **Batch:** A — contracts and source map
- **Commit:** `P14.2: map operational refinement evidence`

### P14.3 — Define the cross-runtime relative-time operation

- **Status:** DONE
- **Files:** `crates/msc-domain/`, `crates/msc-api/`, `crates/msc-agent/src/routes/commands.rs`, `docs/msc2/api-contract/openapi.json`, generated client types, domain fixtures if the existing fixture system needs new cases
- **What:** Add an explicit operation for a relative time preset instead of sending `time set 1000`, `13000`, or `18000` directly. The agent must query or use the runtime's current absolute daytime, derive the current Minecraft day and tick position, calculate the target tick within that same day, and send the runtime-specific absolute command needed to reach it. Define behavior at day boundaries, for a stopped server, when the runtime cannot answer the query, and when a server reports an unsupported time capability. Preserve raw command entry as an explicit absolute operation. The API must make the distinction visible so clients cannot accidentally recreate the old behavior.
- **Verify:** `cargo check --workspace`
- **Batch:** B — relative Minecraft time
- **Commit:** `P14.3: add relative Minecraft time operation`

### P14.4 — Update sidebar and command-picker time actions

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/components/ApplicationShell.svelte`, `clients/desktop-web/src/lib/components/shell/ControlSidebar.svelte`, `clients/desktop-web/src/lib/components/shell/ConsoleDock.svelte`, `clients/desktop-web/src/lib/components/shell/sidebar/QuickCommandsSection.svelte`, `clients/desktop-web/src/lib/sections/console/CommandPaletteSheet.svelte`, `clients/desktop-web/src/lib/sections/console/model.ts`, generated API client types
- **What:** Route Dawn, Dusk, and Night buttons through the new semantic operation in the sidebar and terminal command picker. Label or group exact-day actions separately so “change the day” is deliberate rather than an invisible side effect of choosing a time of day. Keep typed raw commands available and explain that a numeric `time set` is absolute. Use capability discovery to disable or explain an unsupported action rather than guessing across Java and Bedrock. Verify the full supported Java-flavor and Bedrock mapping against the matrix from P14.2.
- **Verify:** `npm run check`
- **Batch:** B — relative Minecraft time
- **Commit:** `P14.4: make time shortcuts day-relative`

### P14.5 — Separate human console history from automatic monitoring traffic

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/console_buffer.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/src/backup_operations.rs`, `crates/msc-application/src/backups.rs`, `crates/msc-application/src/bedrock_service.rs`, `crates/msc-agent/src/routes/networking.rs`, `crates/msc-application/src/playit.rs`, `crates/msc-application/src/xbox_broadcast.rs`, `crates/msc-agent/src/ws/console.rs`, console WebSocket/history DTOs, `docs/msc2/api-contract/openapi.json`
- **What:** Define the retention and delivery contract for every MSC-generated source before changing the buffer. This includes periodic monitoring (`list`, `tps`, `forge tps`, `neoforge tps`, `spark tps`, `tick query`), relative-time's internal `time query gametime`, backup save coordination (`save-all flush`, `save-off`, `save-on`, `save hold`, repeated `save query`, and `save resume`), and helper output from Xbox Broadcast and Playit. Internal parsers and operation waiters must continue receiving these events, but hidden controller/helper traffic must not enter or displace the bounded human console ring. Keep optional helper/controller diagnostics separate, smaller, and independently bounded. Apply filtering before retention and before history/WebSocket delivery, not after the main buffer fills. Preserve genuine server output and operator-entered commands, even when their text resembles a metric. Move actionable helper errors/prompts to structured helper status, notifications, or a dedicated diagnostics view rather than leaking them into the main console solely because they were not classified as routine.
- **Verify:** `cargo check --workspace`
- **Batch:** C — console retention
- **Commit:** `P14.5: define separate automatic console retention`

### P14.6 — Implement early automatic-output classification

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/console_buffer.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/src/backup_operations.rs`, `crates/msc-application/src/backups.rs`, `crates/msc-application/src/bedrock_service.rs`, `crates/msc-agent/src/routes/networking.rs`, `crates/msc-application/src/playit.rs`, `crates/msc-application/src/xbox_broadcast.rs`, `crates/msc-agent/src/ws/console.rs`, metric parsers, backup waiters, helper status/event code, and console event/history serializers
- **What:** Replace the metrics-only classifier with producer-aware ingestion. Tag each line or event by origin—`user`, `server`, `controller`, or `helper`—at the point it is generated or correlated. Cover periodic `list`/TPS/Spark/tick polling; the one-shot `time query gametime`; Java and Bedrock backup save commands and their confirmation/readiness responses; Xbox Broadcast stdout/stderr, auth prompts, readiness, and failures; Playit stdout/stderr, retries, and failures; retries, delayed responses, multiline responses, server restarts, and overlapping operations. Internal metrics and backup waiters must consume the controller stream before presentation filtering. The main console ring and reconnect backfill must retain human/server output only by default, while any diagnostics stream is independently bounded and cannot evict it. Do not hide operator-entered commands or genuine server warnings merely because their text contains `TPS`, `list`, or another known automatic pattern.
- **Verify:** `cargo clippy --workspace --all-targets -- -D warnings`
- **Batch:** C — console retention
- **Commit:** `P14.6: filter automatic output before console retention`

### P14.7 — Align console controls and visible behavior

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/console/model.ts`, console components, WebSocket client/event handling, help content for console filters
- **What:** Remove the current assumption that client-side hiding is sufficient. The default console view and reconnect path must request human/server output without controller or routine helper traffic, so “no items match console filters” is not caused by hidden monitoring, backup, Xbox Broadcast, or Playit output consuming the history window. If an explicit diagnostics view is retained, make it a separate bounded view with clear copy and no effect on the main console. Preserve filter/search behavior for genuine server output, manual command echo, and actionable helper state displayed through its proper status/notification surface.
- **Verify:** `npm run check`
- **Batch:** C — console retention
- **Commit:** `P14.7: keep automatic output out of the main console`

### P14.8 — Define the tri-platform headless command-install contract

- **Status:** awaiting verification
- **Files:** `packaging/linux/install.sh`, `packaging/linux/uninstall.sh`, `tools/release/build-linux-headless.sh`, `tools/release/build-macos-headless.sh`, `tools/release/build-windows-headless.ps1`, release workflow, headless installation documentation
- **What:** Decide and document the supported install shapes for macOS, Windows, and Linux. The contract must answer where the executable lives, how a shell discovers `msc`, how upgrades preserve the PATH entry, how uninstall removes only MSC-owned links, how package-managed Linux installs differ from archives, and how a noninteractive/headless install reports that a new shell or PATH refresh is required. Keep the binary name consistent (`msc`/`msc.exe`) and do not imply that Linux is the only platform with headless support.
- **Verify:** `rg -n "headless|PATH|msc\.exe|/usr/local/bin/msc|Windows|macOS|Linux" packaging tools/release .github/workflows docs/msc2`
- **Batch:** D — headless packaging
- **Commit:** `P14.8: define tri-platform headless installation contract`

### P14.9 — Put the Linux headless command on PATH safely

- **Status:** awaiting verification
- **Files:** `packaging/linux/install.sh`, `packaging/linux/uninstall.sh`, Linux package/archive templates, systemd/service documentation
- **What:** Make a normal Linux install expose `msc` without requiring `cd` or `./msc`. For package installs, use the distribution's normal executable location or an owned symlink in a standard command directory such as `/usr/local/bin`; for archive installs, make the choice explicit and idempotent. Detect an existing non-MSC target before replacing it, support upgrades and uninstall cleanly, explain root/user installation differences, and preserve the existing management service on port 48001. Verify both a fresh install and an upgrade do not create duplicate binaries or stale links.
- **Verify:** `bash -n packaging/linux/install.sh && bash -n packaging/linux/uninstall.sh`
- **Batch:** D — headless packaging
- **Commit:** `P14.9: expose Linux headless CLI on PATH`

### P14.10 — Make macOS and Windows headless installs equally usable

- **Status:** awaiting verification
- **Files:** `packaging/macos/install.sh`, `packaging/macos/uninstall.sh`, `packaging/windows/install.ps1`, `packaging/windows/uninstall.ps1`, `tools/release/build-macos-headless.sh`, `tools/release/build-windows-headless.ps1`, release workflow, headless CLI documentation
- **What:** Give macOS and native Windows the same usable command story. macOS must install or clearly guide an owned `msc` link into a standard PATH location without breaking Intel/Apple Silicon packaging. Windows must install `msc.exe` into an owned directory and add/remove that directory from the appropriate user or machine PATH with an explicit elevation choice. Document shell refresh behavior, PowerShell and Command Prompt discovery, upgrade/uninstall ownership, and service installation separately from CLI PATH installation.
- **Verify:** `rg -n "PATH|msc\.exe|headless|Intel|arm64|uninstall" tools/release packaging .github/workflows docs/msc2`
- **Batch:** D — headless packaging
- **Commit:** `P14.10: complete macOS and Windows headless installs`

### P14.11 — Add a durable remote-host connection profile

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/hosts/types.ts`, `clients/desktop-web/src/lib/hosts/saved.ts`, host switcher/state stores, native secure-store bridge, migration documentation
- **What:** Replace the current `label + baseUrl` profile with a host record that has a stable host ID and editable connection details: display name, one or more LAN addresses/hostnames, one or more Tailscale addresses/hostnames, preferred route order, SSH username, authentication choice, optional local forwarded port, and the remote MSC management port defaulting to 48001. Store LAN and Tailscale addresses together so either can be selected later, and allow edits when DHCP, DNS, or Tailscale addresses change without creating a new logical host. Store only non-secret SSH metadata in ordinary client state; keep bearer credentials in the existing per-host native secure store and never persist an SSH password in localStorage.
- **Implementation amendment (2026-09-11):** The remote connection UI now derives the SSH destination from the selected LAN/Tailscale address and uses SSH port 22 internally. Remote records are tunnel-first; legacy SSH host/port and manual-tunnel fields remain readable only for compatibility.
- **Verify:** `npm run check`
- **Batch:** E — remote connection foundation
- **Commit:** `P14.11: model editable remote host profiles`

### P14.12 — Build the native SSH tunnel/session capability

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src-tauri/`, Tauri invoke/command bridge, desktop auth/transport modules, platform dependency manifests, secure host-key storage
- **What:** Implement the desktop-owned SSH capability used by the profile and connection manager. It must:
  - open `localPort -> 127.0.0.1:48001` on the remote host, using the remembered local port such as `48002`;
  - support SSH host/port/user, password prompt, private-key reference, and the platform SSH agent where available;
  - keep the password in memory only for the connection attempt/session unless the OS credential store is explicitly chosen later;
  - remember the remote host key on the first explicit connection without showing the fingerprint; block a changed key, explain it without displaying key values, and require explicit approval before trusting the replacement;
  - expose connection state, stderr, exit reason, and retry/stop controls to Svelte without leaking passwords or bearer tokens to logs;
  - reconnect or report a recoverable failure when the tunnel drops, and clean up the child process when the host is switched or the app closes;
  - work on macOS, Windows, and Linux Tauri builds with the same frontend contract.
  Decide whether to use the system `ssh` executable behind a managed native process or an embedded library; the user experience must not depend on manually opening a terminal.
- **Verify:** `cargo check --manifest-path clients/desktop-web/src-tauri/Cargo.toml`
- **Batch:** E — remote connection foundation
- **Commit:** `P14.12: add managed SSH tunnel capability`

### P14.13 — Add the teaching connection wizard in “Connect to another host”
- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/hosts/types.ts`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/lib/sections/setup/connection/RemoteConnectionWizard.svelte`, help content, generated client types
- **What:** Rework the existing manual setup section into a compact in-app guided flow. The form collects and explains:
  - host name;
  - LAN address/hostname and optional Tailscale address/hostname, with collapsed cross-platform help for finding both;
  - username and password/key/agent choice;
  - remote MSC port, default `48001`;
  - local forwarded port, default `48002`, with collision detection and an automatic alternative;
  - LAN/Tailscale route selection for the SSH destination.

  Show the generated command as an explanation of what MSC is doing, for example `ssh -N -L 48002:127.0.0.1:48001 username@host`, while making clear that the user does not need to run it manually. The normal wizard always uses the managed SSH tunnel, fixes SSH port 22 internally, and does not expose separate SSH hostname, SSH port, direct-access, or manual-tunnel controls. The redundant route explanation, wizard step labels, and extra network-help copy were removed after owner review; the normal flow now goes directly from connection details to the connection review.
- **Implementation amendment (2026-09-11):** Keep “Review connection” clickable so invalid details produce a specific explanation instead of a silently disabled button. Normalize number-field text to valid numeric ports for validation, collision detection, the generated tunnel command, and connection submission.
- **Implementation amendment (2026-09-11):** First-time “Save and connect” now remembers the SSH host key without showing its fingerprint. A changed identity still stops the connection and requires explicit approval, but MSC explains the change without displaying either fingerprint.
- **Implementation amendment (2026-09-11):** The wizard no longer displays fingerprint values or comparison instructions. First-time setup remembers the identity on “Save and connect”; changed identities still require explicit approval, with the warning shown in plain language and no key values exposed.
- **Release preparation (2026-09-11):** Synchronized application, embedded agent, installer, lockfile, bundle identity, and README references to `0.1.5`; publication awaits the tagged release workflow.
- **Verify:** `npm run check`
- **Batch:** F — remote connection experience
- **Commit:** `P14.13: add guided remote host connection flow`

### P14.14 — Automate remote agent discovery and desktop pairing

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/auth/desktop.ts`, Tauri bridge, CLI pairing output, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, auth/API contract docs
- **What:** After the managed tunnel is established, perform the normal health/capability check. If the host has no saved desktop credential, use the authenticated SSH session to invoke a narrowly-scoped remote pairing operation equivalent to `msc pairing create --client-kind desktop --json`, capture the one-use short-lived challenge, exchange it through the forwarded management connection, and store the resulting durable bearer credential in the OS secure store keyed by the stable host ID. The ordinary flow must not ask the user to copy a pairing code or open a second SSH session. The UI should say what is happening (“creating a one-time desktop authorization on the host”) and show progress/failure plainly.
- **Implementation amendment (2026-09-11):** The selected LAN/Tailscale address is now the SSH destination, while authenticated API traffic and pairing use the local forwarded address. This keeps route choice, tunnel transport, and saved credentials aligned.

  Define recovery explicitly: if remote command execution is unavailable, offer the existing manual pairing-code fallback; if the code expires, create a new one; if the host has an existing credential, use it without re-pairing; if the credential is revoked or the host identity no longer matches, require an intentional repair flow. Do not permit arbitrary shell commands through this feature, and do not let it install/start/stop the operating-system service.
- **Verify:** `npm run check`
- **Batch:** F — remote connection experience
- **Commit:** `P14.14: automate remote desktop pairing`

### P14.15 — Add route selection, host switching, and address repair

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/hosts/connection.ts`, `clients/desktop-web/src/lib/hosts/types.ts`, `clients/desktop-web/src/lib/auth/desktop.ts`, `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, `clients/desktop-web/src/lib/sections/setup/connection/RemoteConnectionWizard.svelte`, `clients/desktop-web/src-tauri/src/lib.rs`
- **What:** Implement the lifecycle for saved hosts. When connecting, try the preferred direct LAN/Tailscale route according to the profile, verify the agent, and fall back to the managed SSH tunnel when direct access is unavailable. Let the user switch explicitly between LAN and Tailscale addresses, edit either address later, and reorder the preference. When a host is selected, start or reuse only that host's tunnel, connect its saved credential, restore its server/console state, and close or suspend the previous host's tunnel according to the connection policy. A normal switch must not request a new pairing code. If an address changes, edit-and-retry must preserve the stable host ID and credential rather than creating duplicate host entries.
- **Verify:** `npm run check`
- **Batch:** F — remote connection experience
- **Commit:** `P14.15: support saved host routes and switching`

### P14.16 — Harden remote connection errors and security boundaries

- **Status:** awaiting verification
- **Files:** Tauri SSH bridge, host/credential stores, auth transport, connection UI, `docs/msc2/msc2-decisions.md`, `docs/msc2/msc2-engineering.md`
- **What:** Cover the failure cases that would otherwise make the guided flow unsafe or confusing: wrong SSH password, unsupported key format, locked SSH agent, changed SSH identity, occupied local port, unreachable LAN address, unreachable Tailscale address, tunnel process exit, remote `msc` missing from PATH, agent stopped, agent below the supported version floor, pairing challenge expiry, revoked token, and switching hosts during an active operation. Error messages must identify whether the failure is network, SSH, MSC agent, authentication, or Minecraft. Sensitive input must be redacted from logs, screenshots, diagnostics, and error telemetry (MSC has no hosted telemetry). Keep the existing per-host credential and permission model; the tunnel is transport, not authorization.
- **Verify:** `rg -n "password|private key|fingerprint|pairing|48001|48002|remote client|service" clients/desktop-web/src-tauri clients/desktop-web/src/lib docs/msc2/msc2-engineering.md docs/msc2/msc2-decisions.md`
- **Batch:** G — remote hardening and documentation
- **Commit:** `P14.16: harden managed remote connections`

### P14.17 — Explain the no-third-party and optional-Tailscale paths

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/setup/AgentSetupSection.svelte`, handbook/help content, remote-access documentation, CLI help text, product/engineering docs
- **What:** Teach the connectivity choices without requiring a third-party service. State plainly that there is no “Tailscale without Tailscale” magic: a remote computer must be reachable by a direct LAN/WAN route, an SSH route, a user-operated VPN/overlay, or a relay. MSC's normal built-in path is direct access when available plus an app-managed SSH tunnel; Tailscale remains an optional convenient private route, not a prerequisite. Explain that router port forwarding and public exposure carry their own security burden, while the management API remains authenticated and loopback-first by default. Make the wizard useful for local IPs, Tailscale IPs, DNS names, and manually maintained tunnels.
- **Verify:** `rg -n "Tailscale|SSH tunnel|48001|48002|no.*relay|direct|VPN|port forwarding" clients/desktop-web/src/lib docs/msc2/msc2-product.md docs/msc2/msc2-engineering.md docs/msc2/msc2-decisions.md`
- **Batch:** G — remote hardening and documentation
- **Commit:** `P14.17: document remote access choices`

### P14.18 — Verify release and platform coverage

- **Status:** awaiting verification
- **Files:** `.github/workflows/release.yml`, `packaging/`, `tools/release/`, Tauri manifests, platform capability matrix, installation and remote-access documentation, `docs/msc2/capabilities/phase14-acceptance.md`
- **What:** Perform the final static and manual acceptance pass across macOS, native Windows, and Linux. Confirm each platform has a usable headless artifact and PATH story; the Tauri app can save/edit LAN and Tailscale endpoints; managed SSH can prompt and reconnect; port `48001` is treated as the remote management port; local `48002` forwarding is configurable; and no flow requires Tailscale, a manually opened terminal, or a second manual pairing session. Include the Linux headless Xubuntu scenario, macOS local/remote scenarios, and native Windows installation scenario. Check that package/archive updates and uninstall behavior preserve or remove only the state they own.
- **Verify:** `cargo fmt --all -- --check && cargo check --workspace && npm run check`
- **Batch:** H — cross-platform acceptance
- **Commit:** `P14.18: verify operational refinements across platforms`

### P14.19 — Phase gate and owner verification handoff

- **Status:** awaiting verification
- **Files:** `docs/msc2/rolling-plan.md`, `docs/msc2/msc2-port-plan.md`, `docs/msc2/msc2-decisions.md`, acceptance notes and capability matrix
- **What:** Review the phase gate as a product behavior, not merely as completed implementation steps. The static handoff is recorded in `docs/msc2/capabilities/phase14-acceptance.md`, but the gate is not claimed complete until Cameron confirms the required live Minecraft, real OS-install, and retained-client walkthrough evidence. The gate holds only when: time-of-day shortcuts preserve the current Minecraft day across the supported Java flavors and Bedrock; explicit day changes remain explicit; all MSC-generated monitoring, backup, and helper traffic cannot evict human console output; `msc` is discoverable after headless installation on all three operating systems; a Tauri user can save both LAN and Tailscale routes, edit changed addresses, see the SSH command being taught, let the app manage forwarding, and pair without routine manual code copying; manual recovery remains available; and the no-cloud/no-required-Tailscale boundary is still true. Record any amended decision or deferred edge case before proposing the next phase.
- **Verify:** `rg -n "P14\.1[1-9]|Status:|Verify:|Batch:" docs/msc2/rolling-plan.md`
- **Batch:** H — cross-platform acceptance
- **Commit:** `P14.19: record Phase 14 gate and handoff`

### P14.20 — Repair cross-platform CI regressions

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/console_buffer.rs`, `crates/msc-agent/src/ws/console.rs`, `docs/msc2/clients/phase11-auth.md`, `docs/msc2/client-capability-matrix.csv`, `clients/desktop-web/tests/e2e/browser/contract-harness.mjs`, three desktop-web files reported by Prettier
- **What:** Address the shared failures from CI run 34660639860. Classify routine metric, player-count, session-status, and Xbox Broadcast output before bounded public console history and WebSocket delivery, while preserving errors and prompts. Add the missing General-LAN authentication boundary and `/v1/time/relative` capability-matrix entry. Isolate browser harness setup/reconnect state per browser context so parallel smoke tests cannot change one another's onboarding or reconnect result. Format the three client files reported by CI. This step does not move or republish a release tag.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the new commit's CI run is green
- **Batch:** I — CI regression repair
- **Commit:** `P14.20: repair cross-platform CI regressions`

### P14.21 — Repair browser host startup and remaining client formatting

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/sections/setup/connection/RemoteConnectionWizard.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Fix the shared browser startup failure found in CI: the active server was selected in the per-host cache before the server list had been copied into that cache, causing connection initialization to throw and leaving onboarding, guides, reconnect, and server management unavailable. Store the server list before selecting the active server. Format the additional Svelte file reported by client validation. Do not move or republish a release tag.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the new commit's CI run is green
- **Batch:** I — CI regression repair
- **Commit:** `P14.21: restore browser host context before selecting server`

### P14.22 — Refresh outdated client source assertions

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/tests/navigation/navigation.test.ts`, `clients/desktop-web/tests/agent-install/agent-install.test.ts`, `clients/desktop-web/tests/screens/first-launch-reset.test.ts`, `docs/msc2/rolling-plan.md`
- **What:** Update existing source-contract assertions that still expected the pre-Phase-14 host ID declaration, transport setup location, remote pairing URL, and older setup-screen wording/error handling. Assert against the current shared host constant, browser/Tauri transport implementation, profile-based pairing URL, and owner-approved control-panel/agent explanation. This aligns validation expectations with the implemented behavior; it does not add tests or change runtime behavior.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the new commit's CI run is green
- **Batch:** I — CI regression repair
- **Commit:** `P14.22: align client source assertions with current host flow`

### P14.23 — Stabilize cross-platform browser smoke timing

- **Status:** awaiting verification
- **Files:** `clients/desktop-web/playwright.config.ts`, `docs/msc2/rolling-plan.md`
- **What:** Run the Playwright browser smoke with one worker in CI while preserving default parallelism for local development. The browser cases share an in-memory contract server and exercise stateful setup/reset navigation; runner-dependent parallel scheduling coincided with an Ubuntu WebKit timeout when the client reset was expected to reopen the first-launch tour. This avoids overlapping those browser workflows and makes CI scheduling consistent across operating systems.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the new commit's CI run is green
- **Batch:** I — CI regression repair
- **Commit:** `P14.23: stabilize cross-platform browser smoke timing`

### P14.24 — Prepare the v0.1.6 release

- **Status:** awaiting verification
- **Files:** `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `clients/desktop-web/src/lib/bundle-identity.ts`, `clients/desktop-web/src/lib/bundle-identity.test.ts`, `README.md`, `docs/msc2/rolling-plan.md`
- **What:** Synchronize all release identity versions and current download/update instructions to `0.1.6`. The existing `v0.1.5` tag remains untouched because it points to the earlier release-preparation commit, before the latest connection-flow corrections and CI repairs. Publish `v0.1.6` only from the current green mainline commit, using the guarded release workflow to build and attach the platform desktop/headless artifacts and signed update metadata.
- **Superseded (2026-09-12):** After `v0.1.6` was published, the owner clarified that `v0.1.5` had never been released and was the intended next version. The replacement is recorded in P14.25.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the version-bump commit's CI is green before pushing `v0.1.6`
- **Batch:** I — release and update handoff
- **Commit:** `P14.24: prepare v0.1.6 release`

### P14.25 — Correct the release version to v0.1.5

- **Status:** awaiting verification
- **Files:** `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `clients/desktop-web/src/lib/bundle-identity.ts`, `clients/desktop-web/src/lib/bundle-identity.test.ts`, `README.md`, `docs/msc2/rolling-plan.md`
- **What:** Owner clarification: `v0.1.5` was tagged but never released, so `v0.1.6` was an unintended version skip. Restore the coordinated application, agent, installer, lockfile, bundle, and README versions to `0.1.5`. After this commit passes CI, replace the published `v0.1.6` prerelease and tag, move the old `v0.1.5` tag from its stale release-preparation commit to this verified commit, then let the guarded workflow rebuild and publish the correctly versioned assets and signed update metadata.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm the correction commit's CI is green before replacing either tag
- **Batch:** I — release and update handoff
- **Commit:** `P14.25: correct release version to v0.1.5`

### P14.26 — Accept signed update signatures with a final newline

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/release_update.rs`, `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `clients/desktop-web/src/lib/bundle-identity.ts`, `clients/desktop-web/src/lib/bundle-identity.test.ts`, `README.md`, `docs/msc2/rolling-plan.md`
- **What:** Fix the headless updater failure reported by the owner: the release signer writes the detached Base64 Ed25519 signature with a final newline, while the updater previously decoded the raw file bytes strictly and rejected that valid format. Trim only surrounding ASCII whitespace before decoding; malformed Base64 within the signature remains rejected. Add one focused regression test for the exact signer output shape. Synchronize the coordinated application, agent, desktop, lockfile, bundle, and README release identity to `0.1.6`; do not publish until verification and CI are green.
- **Verify:** `cargo test -p msc-infrastructure release_update::tests::accepts_signer_signature_file_with_trailing_newline`
- **Batch:** I — release and update handoff
- **Commit:** `P14.26: accept signed update signatures with final newline`

### P14.27 — Pass the known-hosts file as one SSH option
- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src-tauri/src/ssh.rs`, `docs/msc2/rolling-plan.md`
- **What:** Fix the managed SSH tunnel and remote pairing bootstrap failure reported on v0.1.6. OpenSSH requires each `-o` argument to contain a complete `Name=Value` option; MSC previously passed `UserKnownHostsFile` without its value, then passed the path as a separate argument, so SSH rejected the command before connecting. Build `UserKnownHostsFile=<path>` as one OS string and pass it as the value of `-o`, preserving non-UTF-8 path bytes. This uses the shared SSH command builder, so both the managed tunnel and pairing command are corrected without changing host-key trust behavior.
- **Verify:** `cargo fmt --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- -D warnings`
- **Batch:** J — SSH connection repair and v0.1.7 release
- **Commit:** `P14.27: pass known-hosts path as one SSH option`

### P14.28 — Prompt for SSH password during guided connection
- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/lib/sections/setup/connection/RemoteConnectionWizard.svelte`, `clients/desktop-web/src-tauri/src/ssh.rs`, `docs/msc2/rolling-plan.md`
- **What:** Fix the guided connection that stayed at “Creating authorization…” when the remote Ubuntu account used password-based SSH. New hosts now default to password authentication and ask for the password after the connection review when the user clicks **Save and connect**; the value is sent only for that attempt/session and is not persisted. Hosts using SSH-agent or private-key authentication now run without interactive password prompts when no password is supplied, avoiding an invisible OpenSSH askpass wait. Bound remote pairing execution and the HTTP exchange with deadlines so neither can leave the connection button spinning forever.
- **Verify:** `cd clients/desktop-web && npm run format:check && npm run check && npm run build && cd ../.. && cargo fmt --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path clients/desktop-web/src-tauri/Cargo.toml -- -D warnings`
- **Batch:** J — SSH connection repair and v0.1.7 release
- **Commit:** `P14.28: prompt for SSH password during guided connection`

### P14.29 — Accept a supplied SSH password
- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src-tauri/src/ssh.rs`, `docs/msc2/rolling-plan.md`
- **What:** Fix the local validation branch exposed by Cameron's first password-based connection attempt. The validator previously accepted the `password` authentication mode only when the password was empty; with a supplied password, it fell through to the unsupported-authentication error before SSH could contact Ubuntu. Treat password mode as supported when a non-empty password is supplied, while keeping the explicit missing-password error.
- **Verify:** `cd clients/desktop-web && cargo fmt --manifest-path src-tauri/Cargo.toml -- --check && cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`
- **Batch:** J — SSH connection repair and v0.1.7 release
- **Commit:** `P14.29: accept supplied SSH passwords`

### P14.30 — Prompt again for saved-host SSH passwords
- **Status:** awaiting verification
- **Files:** `clients/desktop-web/src/App.svelte`, `clients/desktop-web/src/lib/hosts/connection.ts`, `docs/msc2/rolling-plan.md`
- **What:** After MSC quits, the app-owned SSH tunnel and its in-memory password are intentionally cleared. When reconnecting to a saved password-authenticated host, ask for that host's SSH password before showing an agent-unavailable error; keep it in memory only for the current app session. Re-prompt with a clear message if the password is refused, and allow Cancel to return to the local host. Preserve saved/manual tunnel behavior.
- **Verify:** `cd clients/desktop-web && npm run format:check && npm run check && npm run build` — then manually quit and reopen the desktop app, switch to a saved password-authenticated host, confirm the password prompt appears, connect successfully, and confirm an incorrect password can be retried.
- **Batch:** J — SSH connection repair and v0.1.7 release
- **Commit:** `P14.30: prompt for saved-host SSH passwords`

### P14.31 — Prepare the v0.1.7 release
- **Status:** awaiting verification
- **Files:** `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `clients/desktop-web/src/lib/bundle-identity.ts`, `clients/desktop-web/src/lib/bundle-identity.test.ts`, `README.md`, `docs/msc2/rolling-plan.md`
- **What:** Synchronize the coordinated application and agent identity to `0.1.7`, including lockfiles, Tauri metadata, bundle identity, and download/update instructions. Publish only from the green mainline using the guarded release workflow; it will build the platform desktop/headless artifacts and signed update metadata.
- **Verify:** `gh run list --workflow ci.yml --limit 5` — confirm CI for the version-bump commit is green before pushing tag `v0.1.7`.
- **Batch:** J — SSH connection repair and v0.1.7 release
- **Commit:** `P14.31: prepare v0.1.7 release`

### P14.32 — Correct Ed25519 update signing

- **Status:** awaiting verification
- **Files:** `tools/release/sign-update-manifest.py`, `tools/release/generate-update-key.py`, `tools/release/check-release-workflow.py`, `docs/msc2/rolling-plan.md`
- **What:** Fix the Ed25519 point-recovery equation shared by the manifest signer and release-key generator. The old equation produced non-standard public keys and signatures that the Rust updater correctly rejected, while the signer compared key material using the same faulty calculation. Confirm the standard Ed25519 base point before use and independently verify every generated manifest signature with OpenSSL before release publication. The currently configured release key must be rotated to a standard Ed25519 key pair before another signed release can publish. Existing 0.1.6/0.1.7 agents trust the old key and cannot authenticate signatures from a corrected key, so restoring automatic updates requires one manually installed recovery release; update data remains separate from the executable and is preserved by the headless installer.
- **Verify:** `python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard && python3 -c 'import ast, pathlib; [ast.parse(pathlib.Path(p).read_text()) for p in ("tools/release/sign-update-manifest.py", "tools/release/generate-update-key.py", "tools/release/check-release-workflow.py")]'`
- **Batch:** K — update-signature recovery
- **Commit:** `P14.32: correct Ed25519 update signing`

### P14.33 — Prepare the v0.1.8 recovery release

- **Status:** awaiting verification
- **Files:** `crates/msc-agent/Cargo.toml`, `Cargo.lock`, `clients/desktop-web/package.json`, `clients/desktop-web/package-lock.json`, `clients/desktop-web/src-tauri/Cargo.toml`, `clients/desktop-web/src-tauri/Cargo.lock`, `clients/desktop-web/src-tauri/tauri.conf.json`, `clients/desktop-web/src/lib/bundle-identity.ts`, `clients/desktop-web/src/lib/bundle-identity.test.ts`, `README.md`, `docs/msc2/rolling-plan.md`
- **What:** Synchronize application, agent, Tauri, lockfile, bundle, and download instructions to v0.1.8. This is a one-time recovery release: rotate the GitHub Actions signing secret and public variable to a fresh standard Ed25519 key pair, embed the new public key, and rely on the release signer’s OpenSSL verification before publication. Existing v0.1.6/v0.1.7 installs cannot verify this new trust key; the owner must manually install the signed v0.1.8 headless archive once. The installer replaces only MSC executables/service files; existing server data remains under the configured data directory.
- **Verify:** `python3 tools/release/check-release-workflow.py .github/workflows/release.yml --expect-publish-guard && rg -n '0\.1\.8|v0\.1\.8' crates/msc-agent/Cargo.toml Cargo.lock clients/desktop-web/package.json clients/desktop-web/package-lock.json clients/desktop-web/src-tauri/Cargo.toml clients/desktop-web/src-tauri/Cargo.lock clients/desktop-web/src-tauri/tauri.conf.json clients/desktop-web/src/lib/bundle-identity.ts README.md`
- **Batch:** K — update-signature recovery
- **Commit:** `P14.33: prepare v0.1.8 recovery release`

### P14.34 — Measure the configured active world on Java and Bedrock

- **Status:** awaiting verification
- **Files:** `crates/msc-application/src/worlds.rs`, `crates/msc-application/src/lifecycle.rs`, `crates/msc-agent/src/routes/lifecycle.rs`, `crates/msc-agent/src/routes/performance.rs`, `clients/desktop-web/src/lib/sections/performance/PerformanceSection.svelte`, `docs/msc2/rolling-plan.md`
- **What:** Replace the hard-coded Java `server/world` measurement with a calculation from the configured `level-name`: sum Java's main, Nether, and End live folders, or Bedrock's `worlds/<level-name>` folder. Do not count archived world-slot ZIPs. Report no value when the active world folder cannot be read, and show `—` rather than claiming the world is `0 B`; label the metric as the active world rather than implying every server has three dimensions.
- **Verify:** `cargo fmt --all -- --check && cargo check --workspace && npm --prefix clients/desktop-web run check`
- **Batch:** L — active-world performance metric
- **Commit:** `P14.34: measure active Java and Bedrock world size`

### P14.35 — Make Hide Auto stop automatic console delivery

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/src/console_buffer.rs`, `crates/msc-agent/src/ws/console.rs`, `clients/desktop-web/src/lib/api/generated.ts`, `clients/desktop-web/src/lib/components/ApplicationShell.svelte`, `clients/desktop-web/src/lib/components/shell/ConsoleDock.svelte`, `clients/desktop-web/src/lib/sections/console/model.ts`, `docs/msc2/api-contract/openapi.json`, `docs/msc2/api-contract/websocket-v1.json`, `docs/msc2/antiAIslop.md`, `docs/msc2/rolling-plan.md`
- **What:** Correct the P14.6/P14.7 console regression. Restore the Hide Auto toggle, checked by default, and pass its state to both console history and WebSocket requests. While checked, controller/helper output is not sent to that console client; automatic monitoring, time queries, backup coordination, Xbox Broadcast, Playit, and their internal parsers keep running. Keep automatic lines in their own 200-line diagnostics ring and internal consumer history, never in the 5,000-line human console ring, so they cannot evict server or manual-command output. Unchecking Hide Auto may show only that bounded diagnostics history in the console. Expand classification to recognize Spark's actual `spark-worker-pool-…/INFO` multiline output as well as its older logger format and the known metrics/player-count families. Preserve genuine server and manual-command output; helper errors and prompts remain identifiable as automatic and are handled through their status surfaces rather than leaking into the console while Hide Auto is on. Remove the misleading filter explanation and its unapproved dead HelpLink. Add the owner rule to antiAIslop: never add a “Learn more” hyperlink without Cameron's explicit approval, and never ship a link with a missing or nonfunctional destination.
- **Verify:** `cargo fmt --all -- --check && cargo clippy --workspace -- -D warnings && cargo check --workspace && npm --prefix clients/desktop-web run format:check && npm --prefix clients/desktop-web run check && npm --prefix clients/desktop-web run api:check`
- **Batch:** C — console retention
- **Commit:** `P14.35: make hide auto stop console delivery`

### P14.36 — Classify the complete vanilla tick-query response

- **Status:** awaiting verification
- **Files:** `crates/msc-agent/src/routes/lifecycle.rs`, `docs/msc2/rolling-plan.md`
- **What:** Follow up on P14.35 after live use exposed a partial `/tick query` leak. Correlate its opening status line and its `Target tick rate`, `Average time per tick`, and `Percentiles` lines as one short-lived automatic response. Keep the existing TPS parser fed by every line, but classify the recognized report lines as controller output before they enter console history. Preserve unrelated and actionable server output, and clear the continuation state when the report ends, the lifecycle resets, or the correlation expires.
- **Verify:** `cargo fmt --all -- --check && cargo clippy -p msc-agent --bin msc -- -D warnings && cargo check -p msc-agent --bin msc`
- **Batch:** C — console retention
- **Commit:** `P14.36: classify complete tick query reports`

### P14.37 — Align helper-output regression expectations

- **Status:** awaiting verification
- **Files:** `crates/msc-infrastructure/tests/console_framing.rs`, `docs/msc2/rolling-plan.md`
- **What:** Update the existing helper-output regression expectations to match the P14.35 contract: Xbox Broadcast failures and Playit login prompts are automatic helper output, not human server output. This lets Hide Auto keep them out of public console history while retaining them in the bounded diagnostics stream. No new test is added.
- **Verify:** `cargo test -p msc-infrastructure --test console_framing console_auto_classifier_marks_helper_attention_lines_as_automatic`
- **Batch:** C — console retention
- **Commit:** `P14.37: align helper console expectations`

### P14.38 — Record external static-review findings

- **Status:** awaiting owner triage
- **Files:** `docs/msc2/rolling-plan.md`; affected implementation files are listed with each finding below
- **What:** Record the external static review received on 2026-09-17. The review reported twelve actionable findings, ranked by severity below. All remain unverified: the reviewer ran no tests, builds, linters, type-checkers, or runtime verification, and changed nothing. No Phase 14 gate status changes until Cameron triages the findings and chooses which require implementation steps.
- **Verify:** `rg -n "P14\.38|external static-review findings|world ZIP|Windows service entry point|WebSocket streams" docs/msc2/rolling-plan.md`
- **Batch:** M — static-review triage
- **Commit:** `P14.38: record external static review findings`

#### External review record — 2026-09-17

The review followed the Rust domain/application crates, infrastructure and platform crates, agent/API boundaries, Svelte/Tauri clients, and packaging/update tooling. Confidence is high for each finding. The recommendations below are recorded for triage; they are not yet implementation decisions.

1. **High — A world ZIP can replace the server executable.** `crates/msc-application/src/worlds.rs:1887` activates an imported archive through the unrestricted move at `crates/msc-application/src/worlds.rs:1646`. A ZIP containing valid world data plus a top-level file matching the configured Java server JAR can overwrite that JAR on macOS/Linux; configuration can also be overwritten by an accidental full-server archive. Import and activation require only Worlds permission. Validate an edition-specific world-path set before moving anything, reject executable/configuration/non-world entries, and never merge an arbitrary archive into the server root.

2. **High — Conflicting operations can both pass admission.** `crates/msc-infrastructure/src/operation_journal.rs:172`, called from `crates/msc-application/src/operations.rs:184`, checks for conflicts and writes the journal separately. Concurrent requests for one server can both see no conflict and then mutate the same files. Reserve each server atomically through a shared admission mechanism, persist the reservation, and retain it until terminal state.

3. **High — Interrupted world replacement cannot reliably recover partial folder moves.** Restore at `crates/msc-application/src/backups.rs:798` and recovery at `:849`, with equivalent activation recovery at `crates/msc-application/src/worlds.rs:1995`, infer progress from directory existence even though several independent renames occur. A stop after one folder is installed can leave nonempty partial destinations that old-folder renames cannot replace. Persist explicit transaction progress, handle partially installed destinations, and keep mutation unavailable until recovery establishes a complete old or new world.

4. **High — Host reset can delete files underneath an active operation.** `crates/msc-agent/src/routes/host_reset.rs:107` reaches deletion at `crates/msc-application/src/host_reset.rs:113`. Reset checks Minecraft and other resets but does not exclude restore, activation, backup, or provisioning workers; its `target: None` journal registration conflicts with nothing. Acquire a host-wide maintenance reservation before reset preconditions, refuse reset while conflicting work remains, and block new work through completion.

5. **High — Online backup readiness can be satisfied by an old save acknowledgement.** `crates/msc-agent/src/backup_operations.rs:219` searches retained console history without recording the console position when the current save command is issued. A prior acknowledgement among the last 50 lines can make a new backup copy changing files. Capture a console sequence boundary and accept only subsequent acknowledgements from the relevant server run. This finding concerns stale acknowledgement matching and does not settle the documented best-effort timeout behavior.

6. **High — A delayed connection can replace the client after the user switches hosts.** `clients/desktop-web/src/App.svelte:916`, with related assignments at `:725`, can publish a late connection A after the user has switched to host B. A can replace B's transport or overwrite B's readiness/server/status state. Await into local variables, check the connection generation, and publish the client and associated state together; apply the same ordering to readiness and host-context restoration.

7. **High — The production Windows service launches an executable without a Windows service entry point.** `crates/msc-platform-windows/src/service.rs:151` renders a direct `msc … serve` command at `:306`, while `crates/msc-agent/src/main.rs:39` enters the ordinary CLI/HTTP path. The production executable contains no service dispatcher, entry point, or service-control handler, so Service Control Manager startup cannot complete the expected handshake. Implement the native Windows service lifecycle or ship a wrapper used by the production installer. The existing lifecycle script's separate `ServiceBase` wrapper does not prove the production path.

8. **Medium — The Windows headless updater attempts to replace its own running executable.** `crates/msc-agent/src/cli/update.rs:131` launches `msc.exe update apply`, and replacement occurs at `:570`. On Windows the helper remains an active instance of the image it must remove, so replacement can fail; a stopped service is not restarted when this error returns. Run an independent updater or temporary executable copy and restore the prior service state on every installation failure.

9. **Medium — macOS update rollback is discarded before launch or health recovery.** `clients/desktop-web/src-tauri/src/update.rs:283` deletes the previous app bundle after renaming it, before the launch at `:217` proves desktop/agent health. A correctly signed replacement that cannot launch leaves the owner without the previous coordinated installation, contrary to Approved D-032. Retain the previous installation until health succeeds within a deadline and restore it on launch or health failure.

10. **Medium — Operation cancellation bypasses permission categories.** `crates/msc-agent/src/routes/operations.rs:259` authenticates the request but does not authorize the credential against the operation. A guest without world-management permission who learns a backup or restore ID can interrupt work they could not initiate. Associate operations with their required permission and initiating identity, then authorize cancellation centrally; make any owner/admin override explicit.

11. **Medium — Credential revocation leaves existing WebSocket streams authorized indefinitely.** `crates/msc-agent/src/ws/console.rs:140` and `crates/msc-agent/src/ws/notifications.rs:103` authenticate only during upgrade. An open socket continues receiving console or notification data after revocation or expiry. Bind each stream to credential/session state and close it on revocation, expiry, and host-identity reset.

12. **Medium — Operation history grows without bounds and is rescanned for every admission.** `crates/msc-application/src/operations.rs:90` retains terminal records and cancellation flags without eviction, while `crates/msc-infrastructure/src/operation_journal.rs:193` rescans historical journal files. Repeated scheduled operations therefore grow memory and disk use and make admission progressively more expensive, contrary to Approved D-021. Bound terminal history, discard completed cancellation flags, maintain a bounded active-reservation index, and preserve only the durable recovery/history data required by the chosen limit.

**Review limits:** the reviewer did not inspect every line or dependency, compare directly against MSC 1 Swift source, inspect shipped binaries, validate live releases, or exercise Minecraft, browsers, or OS service managers. Existing tests, fixtures, and smoke scripts were used as source material only. The review found no additional verified issue in browser origin/CSRF enforcement, credential-verifier storage, file-browser path confinement, provider checksum enforcement, signed-manifest validation, or Bedrock sidecar messaging; this is not an exhaustive safety claim. Phase 14 acceptance and platform behavior still require Cameron's verification.

## Historical records

Detailed records for Setup through Phase 12, including the completed P12.121–P12.189 steps, remain in `rolling-plan-archive.md`.
