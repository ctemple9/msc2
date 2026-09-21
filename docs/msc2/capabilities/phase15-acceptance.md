# Phase 15 — Java and modpack workflow acceptance

**Step:** P15.6 · **Status:** awaiting owner verification · **Date:** 2026-09-21

This is the static acceptance record for the first end-to-end scenario. It is
not a claim that a real ATM10 Lite server, CurseForge account, or downloaded
JAR set has been exercised in this workspace. Cameron must perform the live
walkthrough before the step or phase can be closed.

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
| Skip and notes | Partial | The import route writes a server-side unresolved-note block, but the regular Overview notes editor still reads and writes client-local storage. P15.7 must make the server-owned notes contract authoritative. |
| Return to unresolved work | Partial | The structured pending-file map is agent-memory-only and the client only receives it during the original import handoff. A later client session or agent restart cannot reopen the inventory yet. |
| Components count and search | Ready | Components renders the installed count, name/filename search, and installed/missing/unresolved/disabled filters. |
| Console readability | Ready | The console requests `hideAuto=true`, while the agent keeps automatic monitoring output in a separate diagnostic path and retains human history independently. |
| Java-sheet visual parity | Ready for visual walkthrough | The install sheet uses the same dark MSC surface, restrained typography, secondary path text, and normal MSC buttons as the surrounding server-editor sheets. |

## Gate assessment

The implementation is ready for Cameron's static-check and live walkthrough,
but the Phase 15 gate is not yet claimed. Two product-level boundaries remain
open:

- P15.7 must move regular Overview notes from client-local storage to the
  agent-owned server contract, while preserving the unresolved-modpack block.
- Unresolved modpack imports need a durable, discoverable recovery record so a
  later client session or agent restart can reopen the work.

Those are recorded as partial rather than hidden behind a successful ATM10
walkthrough. The named scenario still needs live verification with Java 17,
Java 21, a CurseForge key, at least one automatic Modrinth match, one rejected
JAR, one accepted JAR, one skipped file, Components search, and a readable
console while monitoring is active.
