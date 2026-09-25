# Phase 15 — Java, modpack, and usability acceptance

**Step:** P15.13 · **Status:** complete · **Owner confirmation:** 2026-09-24

This is the acceptance record for the first end-to-end scenario and the
additional Phase 15 usability flow. The implementation and documentation work
is complete by owner direction. Live runtime walkthroughs remain useful
follow-up evidence, but are no longer a Phase 15 completion blocker.

## Acceptance scenario

1. Existing Fabric 1.20.1 server continues to use Java 17.
2. New ATM10 Lite / NeoForge 1.21.1 server requires Java 21.
3. A CurseForge import requests its API key inside the import flow, links to
   the CurseForge API Console, saves the key on the agent, and resumes import.
4. Files that cannot be downloaded are listed with filename, provider, reason,
   and provider link. Confident Modrinth matches are installed only when the
   project, loader/version compatibility, and filename all match.
5. A downloaded JAR can be selected or dropped, is checked by the agent, and
   can be retried or skipped. The remaining list is written into server notes.
6. Components shows a count, searches names and filenames, and distinguishes
   installed, missing, unresolved, and disabled entries.
7. Monitoring traffic stays out of the human console history while metrics
   continue to update.

## Static review

| Area | Result | Evidence |
|---|---|---|
| Java 17 / Java 21 scenario | Ready | The wizard uses the selected Minecraft version and selected runtime path. The shared rule now maps 1.20.1 to Java 17, 1.20.5+ and 1.21.x to Java 21, and 26.1+ to Java 25. |
| Inline CurseForge key setup | Ready | `ImportModpackSheet.svelte` checks the connected agent before import, provides the approved API Console link, saves the key, resumes, and permits skipping. Manual recovery can reopen it for CurseForge entries. |
| Unresolved inventory and provider links | Ready | `CurseForgeManualDownloadSheet.svelte` shows project name, filename, provider, reason, and provider link; its instructions and credential controls now remain correct for Modrinth and CurseForge entries. |
| Automatic Modrinth recovery | Ready | `modpacks.rs` requires an exact normalized project match, compatible loader and Minecraft version, and matching filename before installing a substitute. |
| Drag-and-drop validation | Ready | The agent checks filename, expected size, valid JAR/ZIP structure, and the available manifest hash before writing the destination file. |
| Skip and notes | Ready | The import route and regular Overview notes editor both use the agent-owned server notes field. The unresolved-modpack marker is replaced without discarding the user's notes before it. |
| Return to unresolved work | Partial | The structured pending-file map is agent-memory-only and the client only receives it during the original import handoff. A later client session or agent restart cannot reopen the inventory yet. |
| Components count and search | Ready | Components renders the installed count, name/filename search, and installed/missing/unresolved/disabled filters. |
| Console readability | Ready | The console requests `hideAuto=true`, while the agent keeps automatic monitoring output in a separate diagnostic path and retains human history independently. |
| Java-sheet visual parity | Ready for visual walkthrough | The install sheet uses the same dark MSC surface, restrained typography, secondary path text, and normal MSC buttons as the surrounding server-editor sheets. |

## Additional Phase 15 acceptance flow

The additional flow covers the fixes that followed the ATM10 review:

1. Edit server notes in client A, then observe the same server-owned notes in
   client B. Existing unresolved-modpack note updates remain present.
2. Confirm a fresh desktop window opens at 1240×760, while an existing custom
   window size is not forcibly changed.
3. Run Dawn, Dusk, and Night from the command palette on a world whose day is
   known. Confirm each changes only the time of day and leaves the day number
   unchanged. Confirm raw numeric `/time set` remains absolute.
4. Confirm Active World shows the live Minecraft day and time while connected,
   shows `Unavailable` when the agent cannot provide it, and does not grow the
   card beyond its existing layout.
5. Browse enough Modrinth results to require scrolling. Confirm the scrollbar
   is hidden while wheel, trackpad, keyboard, and touch scrolling still work.
6. Open a sheet, type or select state, click outside it, and confirm it stays
   open. Confirm its close button, Cancel action, and Escape key still work.
   Confirm the transient action menu and confirmation dialog retain their
   intentional dismissal behavior.

### Static review of the additional flow

| Area | Result | Evidence |
|---|---|---|
| Server notes across clients | Ready for live walkthrough | `ServerDTO.notes` is agent-owned, `POST /v1/servers/notes` writes the selected server, Overview reads the returned server record, and the one-time legacy merge clears the old local copy after a successful migration. |
| Unresolved-modpack note preservation | Ready for live walkthrough | The unresolved-note writer removes and rebuilds only its own `[MSC unresolved modpack files]` section, preserving the user's notes before that marker. |
| Initial desktop window | Ready for live walkthrough | `tauri.conf.json` sets the default window to `1240` × `760`; no runtime resize path was added. |
| Same-day Dawn/Dusk/Night | Ready for live walkthrough | `RelativeTimeTarget::for_current_day` derives the absolute target from the queried daylight-cycle day and daytime, and the command palette labels numeric `/time set` as the explicit day-changing path. |
| Active World time | Ready for live walkthrough | The agent's performance snapshot supplies `worldDay` and `worldTimeTicks`; `HomeSection.svelte` passes that snapshot to `ActiveWorldCard.svelte`, which renders one compact line or `Unavailable`. |
| Modrinth scrollbar | Ready for live walkthrough | The bounded `.results` container keeps `overflow-y: auto` and hides scrollbars with Firefox and WebKit rules. |
| Sheet backdrop behavior | Ready for live walkthrough | `Sheet.svelte` defaults `dismissOnBackdrop` to `false`; explicit close, Cancel, and Escape paths remain. `Menu.svelte` still intentionally closes from its scrim, and `ConfirmDialog.svelte` still intentionally closes when its backdrop is clicked. |

### Regression boundaries

| Boundary | Static result |
|---|---|
| Explicit sheet close | Preserved through the shared close button and caller-owned `onClose`. |
| Cancel actions | Preserved in import and confirmation flows; backdrop clicks are not their replacement. |
| Escape | Preserved by the shared sheet key handler whenever a close callback exists. |
| Modpack notes | Preserved through the server-owned notes field and marker-aware unresolved-note update. |

### Repository verification

The step's non-test checks were run on 2026-09-21:

- `cargo check -p msc-agent -p msc-application` — passed.
- `npm --prefix clients/desktop-web run check` — passed with 0 errors and 7
  existing Svelte warnings in two files outside this step's acceptance surfaces.

## Gate assessment

The Phase 15 gate is complete by owner direction. One product-level boundary
is recorded as follow-up work rather than a Phase 15 blocker:

- Unresolved modpack imports still need a durable, discoverable recovery record
  so a later client session or agent restart can reopen the work.

The named scenarios still need live verification with Java 17, Java 21, a
CurseForge key, at least one automatic Modrinth match, one rejected JAR, one
accepted JAR, one skipped file, Components search, a readable console while
monitoring is active, a second connected client, same-day time actions, and the
sheet, window, and scrolling interactions above.
