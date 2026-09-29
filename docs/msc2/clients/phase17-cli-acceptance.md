# Phase 17 CLI physical acceptance

**Status:** Prepared; not run. Cameron records each result after using the
locally packaged desktop or headless installation. This sheet is separate from
the Phase 16 release acceptance record.

Run the matrix on macOS, Windows, and Linux for both desktop and headless
installations. Use a disposable host or a host with a verified backup before
trying a destructive command. Record the exact package version and host
details; do not infer one platform's result from another.

## Installation matrix

| OS | Install type | Package / version | Host details | PATH and ownership result | Service and reboot result | Cameron's result |
|---|---|---|---|---|---|---|
| macOS | Desktop |  |  |  |  | Not run |
| macOS | Headless |  |  |  |  | Not run |
| Windows | Desktop |  |  |  |  | Not run |
| Windows | Headless |  |  |  |  | Not run |
| Linux | Desktop |  |  |  |  | Not run |
| Linux | Headless |  |  |  |  | Not run |

## Checks to perform on each installation

1. **PATH ownership:** Start from a new terminal after installation. Confirm
   that `msc` (or `msc.exe`) resolves to the installed package. Check the
   installer reports a new-shell refresh when the current shell cannot see a
   PATH change. Confirm an unrelated pre-existing command is not replaced.
2. **Service while the API is unavailable:** Run `msc status agent`,
   `msc stop agent`, `msc status agent`, then `msc start agent`. Confirm these
   service commands work while the management API is stopped. Confirm ordinary
   stop preserves boot enablement and `msc disable agent` / `msc enable agent`
   changes boot policy separately.
3. **Reboot behavior:** Reboot after installation and after a routine service
   stop. Confirm an enabled service starts at boot, and that a deliberate
   disable remains disabled across reboot. Record observations before
   re-enabling the service.
4. **Local and SSH authorization:** As the installing account, run locally and
   through SSH: `msc status`, `msc server list`, and `msc status --json`.
   Confirm both shells work without token entry or pairing and JSON parses as
   one complete result. Confirm the SSH process is running on the agent host.
5. **wrong user refusal:** From a second local OS account, run an API-backed
   command such as `msc status`. Confirm the CLI refuses local authorization
   and does not fall back to TCP or ask for a bearer token.
6. **Forwarded-port refusal:** From another computer, create an SSH TCP
   forward to the host's management port and request a protected API resource
   through that forward without credentials. Confirm it is refused. Confirm
   the CLI has no host, URL, port, or token option that uses the forward.
7. **Representative tasks:** As the installing account, inspect the server
   list and status, players, worlds, backups, installed add-ons/components, and
   one help topic. Where supported, exercise active-server RAM read/write, a
   registered-server Bedrock transport or connectivity switch, player profile
   maintenance, session-history clearing, and Xbox credential clearing on a
   disposable host. Confirm active-server context, edition limits, permission
   refusals, restart requirements, and JSON output are clear. The resource-pack
   toggle may report unsupported until its backing store exists; record that
   refusal accurately. Do not perform destructive actions solely for this
   checklist. D-041's gamerule-catalog and router symptom-analysis exceptions
   remain available through the API/desktop; `msc command` and router guide
   reading remain supported CLI paths.
8. **Catalog and provider behavior:** Search and inspect a supported catalog
   item, then stop before install unless using a disposable server. Record a
   missing-key or provider-refusal response. Confirm the CLI preserves the
   provider's actionable error and never reports a failed install as
   successful. For a CurseForge modpack, begin with a user-supplied `.zip`;
   record the missing-key response and any author-blocked file recovery
   result without putting an API key in command arguments or shell history.
9. **Uninstall ownership:** Uninstall through the package's documented path.
   Confirm the MSC-owned command entry and service are removed, while an
   unrelated command/PATH entry and managed server data remain. Record any
   package-manager-owned Linux result under the package's own removal method.

## Results

Record each item as `Pass`, `Fail`, or `Not applicable`, with the package
version and a short observation. For failures, include the command or manual
action and the visible result. Do not mark the Phase 17 gate passed while any
platform result is missing or any user-facing route lacks a CLI task or an
owner-approved exception.

| OS / install type | Check numbers | Result | Package version and notes | Cameron / date |
|---|---|---|---|---|
| macOS desktop | 1–9 | Not run |  |  |
| macOS headless | 1–9 | Not run |  |  |
| Windows desktop | 1–9 | Not run |  |  |
| Windows headless | 1–9 | Not run |  |  |
| Linux desktop | 1–9 | Not run |  |  |
| Linux headless | 1–9 | Not run |  |  |

The route inventory is closed by P17.25–P17.27 and D-041; two bounded
API-only exceptions remain documented in [`phase17-cli.md`](phase17-cli.md).
The Phase 17 gate still requires Cameron's physical results and an independent
review. No test suite, CI gate, release tag, or publication run is part of this
sheet.
