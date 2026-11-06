# Phase 24 terminal content workspace

Implementation candidate; physical acceptance and final UI review are pending.
Run `msc tui`, open the selected server workspace, then press **Shift-P**.
For creation, use **Add-ons → Browse mods or a base pack**. Catalog choices
are staged until creation succeeds. Existing local-file creation remains available.

Use arrow keys to select a row and Enter to choose it. Text fields enter an
editing mode; Enter saves that field, Escape ends editing. Choice fields cycle
on Enter. Escape goes back, preserving the current server contents. No raw JSON
editing or shell escape is needed. Paths refer to the TUI's host, including when
accessed over SSH; this does not transfer files onto the SSH client's computer.

The workspace offers both providers, exact releases, incompatible-release consent,
server requirements, independent player files, source identification/hints and
updates, retained-original recovery, pack update/replacement, publication review,
frozen publication history and export. Pack changes preserve customizations unless
an individual upstream restoration is selected. Refresh a comparison after changing
its choices. Manual-download exceptions require exact local bytes; the agent
validates their purpose, filename, size and checksums.

Publications review exact permission/configuration evidence and excluded paths.
Datapack publication requires an explicit world slot and exact inventory paths
(separate multiple paths with semicolons); terrain is never selected. World slot
IDs appear in **World datapacks**. Missing required content needs explicit incomplete
consent. Known redistribution restrictions cannot be overridden. Historical export
reads the frozen revision independently of the current collection. Illustrated
guides remain subject to P24.20's screenshot prerequisite.

Exports offer a new local ZIP or a destination under the host's exports folder,
with optional nested `.mrpack` when that revision supports it. The local action
refuses existing files, including a destination created during transfer. The host
replacement action requires consent tied to the exact existing hash. It cannot
silently replace a later changed file.

Provider acquisition and mutations use background jobs and display actual operation
progress. Escape requests cancellation when an operation ID exists; uploads or
reads without a cancellable operation remain visible until a confirmed result.
Errors do not imply that a mutation was undone. Refresh before retrying an
unconfirmed result. Creation follow-up failure reports that the server already
exists, rather than presenting creation again.

Owner verification: `cargo check -p msc-agent`, then exercise these actions against
disposable servers. Check keyboard navigation over SSH, mismatches, manual files,
customized pack paths, cancellation, unavailable current content with old publications,
and existing/changed export destinations on each supported host OS. No automated
tests or physical scenarios were run by the implementing agent for this step.
