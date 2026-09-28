# MSC 2 — Rolling Plan

> ## STATUS: P14.38–P14.101 have been moved to `rolling-plan-archive.md` with every step marked awaiting verification. The active plan is the external review audit below; its findings remain recommendations and have not yet been turned into implementation steps.
> **Next move:** Select a high-priority audit finding to plan as the next implementation step.

## How this document works

The vision documents describe where MSC 2 is going; the port plan defines the phase sequence and exit gates; this file records the active priorities and current phase state. Completed phase plans and historical step records live in `rolling-plan-archive.md`.

Each implementation step is planned, read, executed, verified by Cameron, reviewed against its phase gate, and then archived. A step's status here records whether Cameron has completed his verification; moving a step to the archive does not mean that verification is complete.

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
| 14 | Operational refinements | complete |
| 15 | Maintenance follow-ups | complete |

## External review audit — 2026-09-17

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
