# Phase 17 — Host-local CLI contract and API inventory

The `msc` CLI manages the agent on the host where the command runs. A local terminal and an SSH login shell use the same installation-authorized OS account. The agent verifies the local peer and issues an in-memory, short-lived API credential; users do not export tokens or pair the CLI. A wrong account, forwarded TCP connection, or copied binary does not confer access. Desktop remote pairing and per-host credentials remain separate. A desktop-managed `msc pairing create --client-kind desktop --json` invocation through SSH is a fixed-purpose bootstrap, not a general remote CLI mode.

## P17.3 local authentication contract

P17.3 adds the shared credential policy and authenticated API integration. It
does **not** start an IPC listener or change the current CLI transport. The
CLI transport changed in P17.7 to local OS authorization; no operator-visible
token or remote-host option is used by API commands.

The platform listener accepts only a local Unix socket (Linux/macOS) or local
named pipe (Windows). It obtains the caller's UID or SID from the **accepted
connection** using OS peer-identity APIs, and the installing account's UID or
SID from the running agent process. Neither value comes from a request body,
command argument, environment variable, forwarded HTTP header, or executable
path. The listener calls `AuthState::issue_for_local_cli_peer` only after both
OS lookups succeed. The shared policy requires equal identities and rejects
root or LocalSystem as an installation account. Platform packaging must
restrict access to its local IPC endpoint as another layer of protection.

The exchange returns a random bearer credential valid for five minutes. Its
salted verifier and the account audit label live only in this agent process;
the credential is absent from the durable registry and secret store. It is
valid only against the agent instance that issued it, uses the existing API
permission middleware, and attributes requests to `cli:uid:<uid>` or
`cli:sid:<sid>`. The installing account gets the admin role and its existing
permission categories. The CLI keeps the credential in process memory for
that invocation; it never writes or prints it. The exchange is separate from
desktop package bootstrap and desktop remote pairing.

An unknown or mismatched peer, failed OS lookup, unavailable endpoint, or
full issuance capacity yields no credential. The listener must not fall back
to a TCP exchange. A forwarded TCP connection or copied binary carries no OS
peer evidence and cannot obtain a CLI credential. If the service is stopped,
there is no endpoint; the CLI must report that condition when P17.7 connects
it. Agent restart and host reboot discard all issued credentials. The same
authorized OS account can obtain a fresh one after the service returns,
without persistent CLI setup. A host reset clears outstanding credentials.

On macOS and Linux, the CLI locates the Unix socket using an explicit
`MSC2_DATA_DIR` from its own environment when supplied. Otherwise it reads the
installed agent service definition and uses that service's `MSC2_DATA_DIR`,
falling back to the platform default only when no service override exists.
This keeps CLI and agent socket paths aligned even when installation uses a
custom data root. Windows uses a fixed named pipe and does not depend on the
data-directory path.

### Linux exchange (P17.4)

The Linux agent binds `local-cli.sock` inside its installing-user-owned MSC
data directory, with directory mode 0700 and socket mode 0600. It removes an
old socket on restart only when the old entry is a socket owned by that same
user; an unexpected file causes startup to fail. For each accepted connection,
the Linux adapter reads `SO_PEERCRED` from the kernel and compares the UID
with the agent process's effective UID. A successful exchange sends one JSON
line containing the short-lived token; refusal sends an error code without a
token. The root credential helper and its socket remain separate and retain
their existing privilege boundary. P17.7 supplies the CLI client for this
exchange; a local or SSH shell running as the installing user will use the
same socket.

### macOS exchange (P17.5)

The macOS LaunchDaemon also runs as the installing user. It binds the same
`local-cli.sock` name inside that account's MSC data directory, restricts the
directory to mode 0700 and the socket to 0600, and removes a stale socket only
when it is a socket owned by that account. For each connection, macOS
`getpeereid` supplies the kernel-attached peer UID; the agent compares it with
its effective UID before issuing the same five-minute in-memory API
credential. A local terminal or SSH login as the installing user works, while
another account and root are refused. This listener does not read or use the
desktop bootstrap key, signed-code requirement, or bootstrap socket. Its data
directory stays in the account's application-support area, outside
user-selected folders that require macOS privacy consent.

### Windows exchange (P17.6)

The Windows agent creates `\\.\pipe\msc2-local-cli-v1` with a protected
named-pipe DACL granting access only to the SID in the agent process token.
Remote pipe clients are rejected. After receiving a bounded version-1 hello,
the agent impersonates the connected pipe client long enough to read the
kernel-backed token SID, then immediately reverts to its service identity.
Shared auth issues a credential only when the client SID matches the service
SID and is not LocalSystem. A local login or SSH login as the installing
account therefore works after service or host restarts; other accounts cannot
open the pipe and still would fail the identity comparison. The pipe has no
relationship to the Service Control Manager, and this listener cannot start,
stop, or replace the service.

`msc start agent`, `msc stop agent`, and `msc status agent` control the installed local service while the API is down. Installation enables startup at boot. A routine stop holds until explicit start or the next boot; only an explicit disable changes future startup. Minecraft server start/stop are separate operations. Desktop and headless packages own the command on PATH.

Catalog installs follow search → inspect compatible versions and dependencies → confirm → install. CurseForge modpacks begin with a local `.zip`; the agent downloads permitted manifest files and reports author-blocked files for manual supply. `msc command` is the sole raw Minecraft command path. Player profile maintenance takes profile IDs from `msc player profiles`; pass `--server` to select a registered target before the route acts on active-server context. Xbox credential clearing requires `--confirm`, and account secrets remain stdin-only. The Geyser pack toggle route is currently unsupported by the agent. RAM commands act on the active server; inspect `msc status` or `msc server list` first. `msc components` lists installed server components. Server-level Playit, Xbox Broadcast, and Bedrock transport commands take a server name or ID explicitly. All task commands keep human-readable output and `--json` for scripting. Destructive tasks require explicit confirmation. The inventory below reflects the CLI source after P17.22; rows marked for owner review remain open and are not approved exceptions.

## Task-first examples

Run these on the agent host, either in a local terminal or an SSH shell as
the account that installed the service:

```sh
msc status agent
msc server list
msc server use "Modded Survival"
msc player online
msc world list
msc backup list
msc addon search "map"
msc help topic handbook.first-server
ssh msc-linux msc server list
```

API commands obtain a short-lived credential from the local OS endpoint for
that invocation. Do not add a host, API URL, port, or token option; use the
desktop app's remote-host pairing when managing another machine. Add `--json`
for scripts, for example `msc server list --json`.

Java-only actions include Java whitelist changes, Java UUID migration,
resource-pack publication, and Java data packs. Bedrock-only actions include
the allowlist, Bedrock world repair, and Bedrock behavior packs. Bedrock
player-data changes require a stopped server; Java UUID migration is not
available for Bedrock. Commands report unsupported edition combinations from
the route contract instead of treating them as successful operations.

## P17.2 API-to-CLI route inventory

Source: `openapi.json` (158 method/path entries, 142 paths), checked against `crates/msc-agent/src/main.rs`, route modules, and `cli/mod.rs` on 2026-09-29. The permission column is the OpenAPI `x-permission-category`; the live router remains the enforcement authority. “Existing” names the task command or command group that covers the route. “Internal” names transport or client-only use. “Owner review” is an uncovered user task for which this inventory does not assume an exception. `none` does not imply unauthenticated access.

For each user task, terminal output must identify the target and result; `--json` must emit machine-readable success or error. Mutations involving a selected server must show or require the active server before execution. Destructive mutations require exact confirmation; provider and edition refusals must be explicit. Server, player, world, backup, component, and status routes use active-server context unless a route accepts a target server ID. Java-only and Bedrock-only limits follow the route request/response contract and must be shown in command help.

| Method and route | Task or internal purpose | CLI disposition | Permission | Context / limit |
|---|---|---|---|---|
| GET `/v1/catalog/gamerules` | Return verified built-in gamerules for an exact Minecraft release | Approved API-only exception (D-041); use `msc command` for a raw gamerule | `none` | active server / capability |
| POST `/v1/servers/create` | Create a new server | Existing: `msc server create` | `fleet` | host |
| POST `/v1/servers/import` | Scan, import, or rescan existing servers | Existing: `msc server import` | `fleet` | host |
| POST `/v1/servers/export` | Stage all configured servers as a transfer file | Existing: `msc server export` | `fleet` | host |
| POST `/v1/servers/delete` | Delete a server | Existing: `msc server delete` | `fleet` | host |
| POST `/v1/servers/rename` | Rename a server | Existing: `msc server rename` | `fleet` | host |
| POST `/v1/servers/directory` | Update a server's configured directory | Existing: `msc server rescan` | `fleet` | host |
| GET `/v1/servers/eula` | Read the Minecraft EULA status for a server | Existing: `msc server eula` | `fleet` | host |
| POST `/v1/servers/eula` | Accept the Minecraft EULA for a server | Existing: `msc server eula` | `fleet` | host |
| POST `/v1/worlds/create` | Create a new world slot | Existing: `msc world create` | `worlds` | active server / capability |
| POST `/v1/worlds/rename` | Rename a world slot | Existing: `msc world rename` | `worlds` | active server / capability |
| POST `/v1/worlds/replace` | Copy a saved slot's content into another existing slot | Existing: `msc world copy --from --into` | `worlds` | active server / capability |
| POST `/v1/worlds/repair` | Repair a Bedrock world's level.dat | Existing: `msc world repair` | `worlds` | active server / capability; Bedrock; server stopped |
| POST `/v1/worlds/activate` | Activate a world slot (starts activation asynchronously) | Existing: `msc world activate` | `worlds` | active server / capability |
| POST `/v1/components/update` | Update a system component (paper/geyser/floodgate) or a Modrinth-tracked add-on | Existing: `msc addon update` | `addons` | active server / capability |
| POST `/v1/components/remove` | Remove an installed Modrinth-tracked add-on | Existing: `msc addon remove` | `addons` | active server / capability |
| POST `/v1/components/install` | Install an add-on from the Modrinth catalog into the active server | Existing: `msc addon install-catalog/local` | `addons` | active server / capability |
| POST `/v1/components/version` | Change the active server's JAR version/build | Existing: `msc version set` | `addons` | active server / capability |
| POST `/v1/modpacks/inspect` | Inspect a staged modpack archive (.mrpack or CurseForge) without mutating any server | Existing: `msc modpack inspect` | `none` | active server / capability |
| POST `/v1/modpacks/import` | Import a staged modpack archive into the active server, or explicitly replace an already pack-managed server's pack | Existing: `msc modpack import/replace` | `addons` | active server / capability |
| POST `/v1/modpacks/{operationId}/manual-file` | Complete one pending author-blocked CurseForge file for a running modpack-import operation (D-027) | Existing: `msc modpack manual-file` | `addons` | active server / capability |
| POST `/v1/backups/now` | Start an immediate backup | Existing: `msc backup create` | `worlds` | active server / capability |
| POST `/v1/backups/restore` | Restore a backup by id (filename) | Existing: `msc backup restore` | `worlds` | active server / capability |
| GET `/v1/backups/config` | Get auto-backup configuration | Existing: `msc backup config` | `none` | active server / capability |
| POST `/v1/backups/config` | Update auto-backup configuration | Existing: `msc backup config` | `settings` | active server / capability |
| GET `/v1/config/host-setup` | Get whether one-time setup is complete for this agent host | Existing: `msc config host-setup` | `none` | host |
| POST `/v1/config/host-setup/complete` | Mark one-time setup complete for this agent host | Existing: `msc config complete-host-setup` | `settings` | host |
| GET `/v1/config/java-runtime` | Get the global Java executable path override | Existing: `msc java get/set` | `none` | host; Java |
| POST `/v1/config/java-runtime` | Set the global Java executable path override | Existing: `msc java get/set` | `settings` | host; Java |
| GET `/v1/config/servers-root` | Get the folder where this agent stores servers | Existing: `msc config servers-root` | `none` | host |
| POST `/v1/config/servers-root` | Set the folder where this agent stores servers | Existing: `msc config set-servers-root` | `settings` | host |
| GET `/v1/config/curseforge` | Get whether this agent has a CurseForge API key | Existing: `msc config curseforge` | `none` | host |
| POST `/v1/config/curseforge` | Save or clear this agent's CurseForge API key | Existing: `msc config set-curse-forge-key --key-stdin` | `settings` | host; protected stdin |
| GET `/v1/config/ram` | Get the active server's RAM allocation | Existing: `msc config ram get` | `none` | active server |
| POST `/v1/config/ram` | Update the active server's RAM allocation | Existing: `msc config ram set` | `settings` | active server; restart may be required |
| GET `/v1/config/geyser` | Get the active server's Geyser config | Existing: `msc config geyser` | `none` | host |
| POST `/v1/config/geyser` | Update the active server's Geyser config | Existing: `msc config set-geyser` | `settings` | host |
| GET `/v1/users` | List named-access users | Existing: `msc access list` | `admin` | host |
| POST `/v1/users` | Create a named-access user | Existing: `msc access create` | `admin` | host |
| POST `/v1/users/revoke` | Revoke a named-access user | Existing: `msc access revoke` | `admin` | host |
| POST `/v1/users/update` | Update a named-access user's label/role/permissions/expiry | Existing: `msc access update` | `admin` | host |
| POST `/v1/health/repair` | Trigger a repair action for a diagnosed startup problem | Existing: `msc doctor repair` | `settings` | active server / capability |
| GET `/v1/playit` | Get Playit tunnel status | Existing: `msc playit status` | `none` | host |
| POST `/v1/playit/setup` | Start native Playit account and tunnel setup | Existing: `msc playit setup` | `networking` | host; password via protected stdin |
| POST `/v1/playit/reset` | Clear host-local Playit credentials and derived state | Existing: `msc playit reset` | `networking` | host |
| POST `/v1/servers/playit` | Enable or disable Playit for one registered server | Existing: `msc server playit <server> <enabled>` | `networking` | host; resolved server ID |
| POST `/v1/playit/start` | Start the Playit tunnel as a cancellable managed operation | Existing: `msc playit start` | `networking` | host |
| POST `/v1/playit/stop` | Stop the Playit tunnel as a managed operation | Existing: `msc playit stop` | `networking` | host |
| GET `/v1/broadcast/autostart` | Get Xbox broadcast auto-start setting | Existing: `msc broadcast autostart` | `none` | host |
| POST `/v1/broadcast/autostart` | Set Xbox broadcast auto-start | Existing: `msc broadcast autostart` | `broadcast` | host |
| GET `/v1/broadcast/auth-prompt` | Get pending MCXboxBroadcast auth prompt | Existing: `msc broadcast auth-prompt` | `none` | host |
| POST `/v1/broadcast/auth-prompt/dismiss` | Dismiss the pending auth prompt | Existing: `msc broadcast dismiss-auth-prompt` | `broadcast` | host |
| GET `/v1/broadcast/status` | Get Xbox/Bedrock broadcast running status | Existing: `msc broadcast status` | `none` | host; Bedrock |
| POST `/v1/broadcast/start` | Start Xbox broadcast as a cancellable managed operation | Existing: `msc broadcast start` | `broadcast` | host |
| POST `/v1/broadcast/stop` | Stop Xbox broadcast as a managed operation | Existing: `msc broadcast stop` | `broadcast` | host |
| POST `/v1/broadcast/restart` | Restart Xbox broadcast as a cancellable managed operation | Existing: `msc broadcast restart` | `broadcast` | host |
| GET `/v1/broadcast/credentials` | Get the host-wide MCXboxBroadcast account status | Existing: `msc broadcast credentials` | `undocumented` | host |
| POST `/v1/broadcast/credentials` | Update the host-wide MCXboxBroadcast Microsoft account credentials | Existing: `msc broadcast credentials` | `broadcast` | host |
| POST `/v1/broadcast/credentials/clear` | Clear saved MCXboxBroadcast credentials | Existing: `msc broadcast clear-credentials --confirm` | `broadcast` | host |
| POST `/v1/servers/xbox-broadcast` | Enable or disable Xbox Broadcast for one registered server | Existing: `msc server xbox-broadcast <server> <enabled>` | `broadcast` | host; Bedrock server ID |
| GET `/v1/broadcast/jar-status` | Get MCXboxBroadcast JAR install status | Existing: `msc broadcast download-jar` reports status before/after the operation | `none` | host |
| POST `/v1/broadcast/download-jar` | Download the MCXboxBroadcast JAR as a cancellable managed operation | Existing: `msc broadcast download-jar` | `broadcast` | host |
| GET `/v1/resourcepacks` | List resource packs | Existing: `msc resource-pack list` | `none` | active server / capability; Java |
| POST `/v1/resourcepacks/activate` | Activate (or clear) the local Java resource pack | Existing: `msc resource-pack activate` | `addons` | active server / capability; Java |
| POST `/v1/resourcepacks/seturl` | Set a custom resource-pack URL directly in server.properties | Existing: `msc resource-pack set-url` | `addons` | active server / capability; Java |
| POST `/v1/resourcepacks/toggle` | Enable/disable a Geyser resource pack | Existing: `msc resource-pack toggle <pack-id> <enabled>`; current route reports unsupported until the Bedrock client-pack store exists | `addons` | active server / capability |
| POST `/v1/resourcepacks/remove` | Remove a resource pack from disk | Existing: `msc resource-pack remove` | `addons` | active server / capability; Java |
| GET `/v1/watchdog/status` | Get watchdog enabled status | Existing: `msc config watchdog` | `none` | active server / capability |
| POST `/v1/watchdog/enable` | Enable the watchdog | Existing: `msc config watchdog-enable` | `settings` | active server / capability |
| POST `/v1/watchdog/disable` | Disable the watchdog | Existing: `msc config watchdog-disable` | `settings` | active server / capability |
| POST `/v1/command` | Send a console command to the active server | Existing: `msc command` | `serverControl` | active server / capability |
| POST `/v1/time/relative` | Set a named time of day within the current Minecraft day | Excluded by P17 scope: use the sole raw path, `msc command` | `serverControl` | active server / capability |
| POST `/v1/start` | Start the active server | Existing: `msc server start` | `serverControl` | active server / capability |
| POST `/v1/stop` | Stop the active server | Existing: `msc server stop` | `serverControl` | active server / capability |
| GET `/v1/allowlist` | Get the Bedrock allowlist | Existing: `msc bedrock allowlist` | `none` | active server / capability; Bedrock |
| POST `/v1/allowlist` | Add or remove a Bedrock allowlist entry | Existing: `msc bedrock allowlist` | `players` | active server / capability; Bedrock |
| GET `/v1/players` | List currently-online players | `msc player online` | `none` | active server / capability |
| POST `/v1/players/action` | Send a typed player message, kick, ban/pardon, operator, or Java whitelist action | `msc player message/kick/ban/pardon/op/deop/whitelist` | `players` | active server / capability; running server required; Bedrock runtime required for live actions; Java only for whitelist |
| POST `/v1/players/skin-override` | Set or clear a manual skin lookup override for a player | Existing: `msc player skin-override <profile-id> [--lookup-identifier value]` | `players` | active server / capability |
| POST `/v1/players/hidden` | Hide or unhide a player profile | Existing: `msc player hide <profile-id> --hidden true|false` | `players` | active server / capability |
| POST `/v1/players/identify` | Assign a gamertag to an unresolved Bedrock profile | Existing: `msc player identify <profile-id> <gamertag>` | `players` | active server / capability; Bedrock |
| GET `/v1/players/profiles` | List all-time player profiles with stats | `msc player profiles` | `none` | active server / capability |
| POST `/v1/players/delete` | Delete a player's data | `msc player data delete` | `players` | active server / capability; explicit `--confirm`; Bedrock server must be stopped |
| POST `/v1/players/migrate-offline` | Migrate player data to its offline UUID | `msc player data migrate-offline` | `players` | active server / capability; Java only; explicit `--confirm` |
| POST `/v1/players/migrate` | Migrate player data to a custom UUID | `msc player data migrate` | `players` | active server / capability; Java only; explicit `--confirm` |
| POST `/v1/players/duplicate` | Duplicate a player's data | `msc player data duplicate` | `players` | active server / capability; Bedrock server must be stopped |
| GET `/v1/duckdns` | Get the configured DuckDNS hostname | Existing: `msc network duckdns` | `none` | host |
| POST `/v1/duckdns` | Update the DuckDNS hostname | Existing: `msc network duckdns` | `settings` | host |
| GET `/v1/servers` | List all registered servers | Existing: `msc server list` | `none` | host |
| POST `/v1/servers/notes` | Update a server's Overview notes | Existing: `msc server notes` | `fleet` | host |
| POST `/v1/servers/bedrock-transport` | Select the managed connection transport for one Bedrock server | Existing: `msc server bedrock-transport <server> <transport>` | `fleet` | host; Bedrock |
| GET `/v1/servers/size` | Measure a registered server's directory | Existing: `msc server size` | `none` | host |
| GET `/v1/status` | Current run status (active server, pid, running state) | Existing: `msc status` | `none` | active server / capability |
| GET `/v1/performance` | Latest performance snapshot (TPS, players, CPU, RAM, world size) | Existing: `msc metrics` | `none` | active server / capability |
| POST `/v1/active-server` | Select which registered server is active | Existing: `msc server use` | `serverControl` | host |
| GET `/v1/session-log` | Join/leave event history for the active server | Existing: `msc sessions` | `none` | active server / capability |
| POST `/v1/session-log/clear` | Clear join/leave event history for the active server | Existing: `msc clear-sessions` | `players` | active server / capability |
| GET `/v1/console/tail` | Last N console lines; hideAuto defaults to true and prevents automatic output from being sent to the client | Existing: `msc console tail` | `none` | active server / capability |
| POST `/v1/console/stream-ticket` | Create a short-lived console WebSocket ticket | Internal — desktop WebSocket ticket | `none` | active server / capability |
| GET `/v1/components` | Installed system components (Paper/Geyser/Floodgate/flavor jar) and update status | Existing: `msc components` | `none` | active server / capability |
| GET `/v1/addons` | Installed add-ons (mods/plugins) with update status | Existing: `msc addon list` | `none` | active server / capability |
| GET `/v1/files` | Browse the active server's directory (admin-only; query param path). 409 reuses the same shape with note=no_active_server. | Existing: `msc file browse` | `admin` | active server / capability |
| GET `/v1/files/read` | Read a previewable file's contents (admin-only; query param path, required) | Existing: `msc file read` | `admin` | active server / capability |
| GET `/v1/components/client-export` | Package selected components/add-ons for client-side install (query param selected, comma-separated ids) | Existing: `msc addon export` | `none` | active server / capability |
| GET `/v1/catalog/search` | Search the Modrinth add-on catalog for the active server, or an explicit flavor with no active server needed (query params q, offset, javaFlavor, minecraftVersion) | Existing: `msc addon search` | `none` | active server / capability; Java |
| GET `/v1/catalog/projects/{projectId}` | Fetch full Modrinth project detail for the catalog browser | Existing: `msc addon inspect` | `none` | active server / capability |
| GET `/v1/catalog/projects/{projectId}/versions` | Fetch every Modrinth version for a catalog project | Existing: `msc addon versions` | `none` | active server / capability |
| GET `/v1/java-runtimes` | Java runtimes detected on this host | Existing: `msc java list` | `none` | active server / capability; Java |
| POST `/v1/java-runtimes/install` | Install a Java runtime the agent manages itself | Existing: `msc java install` | `settings` | active server / capability; Java |
| GET `/v1/versions` | Available server JAR versions for the active server's flavor | Existing: `msc version list` | `none` | active server / capability |
| GET `/v1/versions/create` | Available versions for the create-server flow (query params serverType, javaFlavor) | Existing: `msc version list --create` | `none` | active server / capability; Java |
| GET `/v1/settings` | Typed server.properties schema for the active server, as sections of fields | Existing: `msc settings get/set` | `none` | active server / capability |
| POST `/v1/settings` | Apply a sparse set of settings changes (key -> string value), validated and clamped per field | Existing: `msc settings get/set` | `settings` | active server / capability |
| GET `/v1/me` | The calling token's own role and permissions | Existing: `msc access me` | `none` | host |
| POST `/v1/host/reset` | Reset this host's MSC state | Existing: `msc host-reset configuration/everything --confirm "RESET AGENT"` | `admin` | host-local only; client reset remains local to the client |
| POST `/v1/auth/desktop-pairings` | Exchange a desktop pairing code for a host-scoped bearer credential | Internal — desktop credential bootstrap | `none` | transport only |
| GET `/v1/worlds` | World slots for the active server, plus which is active | Existing: `msc world list` | `none` | active server / capability |
| GET `/v1/connectivity` | Reachability summary: join address, method, playit/broadcast status | Existing: `msc network status` | `none` | host |
| GET `/v1/health` | Diagnostic health cards for the active server | Existing: `msc doctor` | `none` | active server / capability |
| GET `/v1/health/problems` | Startup problems detected for the active server (missing deps, incompatible versions, ...) | Existing: `msc doctor` | `none` | active server / capability |
| GET `/v1/backups` | Backups available for the active server | Existing: `msc backup list` | `none` | active server / capability |
| GET `/v1/players/{profileId}/skin` | A player's skin image (base64), by profile id. Handled outside the main route switch via a path.hasPrefix/hasSuffix match, not a `case` in it -- the one MSC 1 route with a path parameter. | Internal — binary display asset | `none` | active server / capability |
| POST `/v1/operations` | Create a long-running operation | Internal — operation start transport | `serverControl` | host |
| GET `/v1/operations/{id}` | Read an operation's current state | Existing: `msc operation show` | `none` | host; requires an operation ID |
| POST `/v1/operations/{id}/cancel` | Request cancellation of an operation | Existing: `msc operation cancel` | `serverControl` | host; owner/admin authorization and cooperative stop |
| GET `/v1/capabilities` | Report agent capabilities for this host and this token | Existing: `msc capabilities` | `none` | host |
| GET `/v1/help/{helpId}` | Resolve an educational content topic | Existing: `msc help topic` | `none` | host |
| GET `/v1/help/catalog` | List available educational content topics | Existing: `msc help catalog` | `none` | host |
| GET `/v1/guides/onboarding` | Read the structured first-launch guide | Existing: `msc help onboarding` | `none` | host |
| GET `/v1/guides/router-catalog` | List router guides and troubleshooting topics | Existing: `msc help router-catalog` | `none` | host |
| GET `/v1/guides/router/search` | Search and match router guides | Existing: `msc help router-search` | `none` | host |
| GET `/v1/guides/router/{guideId}` | Compose and resolve one router guide | Existing: `msc help router-guide` | `none` | host; selected server needed for resolved fields |
| POST `/v1/guides/router/troubleshooting/analyze` | Analyze router troubleshooting symptoms | Approved API-only exception (D-041); router guide catalog/search/reading remain in `msc help` | `none` | host |
| POST `/v1/worlds/update` | Save the current live world into the active slot | Existing: `msc world save-current` | `worlds` | active server / capability |
| POST `/v1/worlds/delete` | Delete a non-active world slot | Existing: `msc world delete` | `worlds` | active server / capability |
| POST `/v1/worlds/duplicate` | Duplicate a world slot under a fresh id | Existing: `msc world duplicate` | `worlds` | active server / capability |
| POST `/v1/worlds/import` | Import a staged world ZIP as a new slot | Existing: `msc world import` | `worlds` | active server / capability |
| POST `/v1/worlds/export` | Stage a world slot's archive for download | Existing: `msc world export` | `worlds` | active server / capability |
| POST `/v1/worlds/rename-active-world` | Directly rename the active/live world's on-disk folders | Existing: `msc world rename-active` | `worlds` | active server / capability |
| POST `/v1/worlds/replace-active-world` | Replace the active/live world's on-disk content directly (starts asynchronously) | Existing: `msc world replace-active` | `worlds` | active server / capability |
| GET `/v1/worlds/convert/formats` | List the installed Chunker world-conversion formats | Existing: `msc world convert (requires format id; discovery missing)` | `worlds` | active server / capability |
| POST `/v1/worlds/convert/chunker` | Download the official Chunker world-conversion CLI | Existing: `msc world acquire-chunker` | `worlds` | active server / capability |
| POST `/v1/worlds/convert` | Start a Chunker world-format conversion | Existing: `msc world convert` | `worlds` | active server / capability |
| GET `/v1/catalog/datapacks` | Search Modrinth Java datapacks for a Minecraft version | Existing: `msc world pack search` | `none` | active server / capability; Java |
| POST `/v1/worlds/{slotId}/datapacks/install` | Install a Modrinth Java datapack into one world slot | Existing: `msc world pack install` | `worlds` | active server / capability; Java |
| GET `/v1/catalog/behaviorpacks` | Search CurseForge Minecraft Bedrock resource and behavior packs for a Bedrock version | Existing: `msc world pack search` | `none` | active server / capability; Bedrock; CurseForge key required |
| GET `/v1/catalog/behaviorpacks/{projectId}` | Get a CurseForge Bedrock add-on's description, gallery, and version files | Existing: `msc world pack inspect` | `none` | active server / capability; Bedrock |
| POST `/v1/worlds/{slotId}/behaviorpacks/install` | Install a CurseForge Bedrock behavior pack and bundled linked resource packs into one world slot | Existing: `msc world pack install` | `worlds` | active server / capability; Bedrock |
| GET `/v1/worlds/{slotId}/profile` | Read one world slot's saved profile and runtime metadata | Existing: `msc world profile` | `none` | active server / capability |
| POST `/v1/worlds/{slotId}/profile` | Save a world slot profile and apply its accepted runtime projection | Existing: `msc world profile-set (write only)` | `worlds` | active server / capability |
| GET `/v1/worlds/{slotId}/thumbnail` | A world slot's thumbnail image, if one was generated | Internal — binary display asset | `none` | active server / capability |
| POST `/v1/worlds/{slotId}/thumbnail` | Set a world slot's thumbnail from a staged image upload | Internal — binary display asset | `worlds` | active server / capability |
| POST `/v1/backups/delete` | Delete a backup by id | Existing: `msc backup delete` | `worlds` | active server / capability |
| POST `/v1/staged-uploads` | Begin a bounded staged upload | Internal — upload staging | `none` | transport only |
| DELETE `/v1/staged-uploads/{id}` | Cancel and remove a staged upload | Internal — upload staging | `none` | transport only |
| PUT `/v1/staged-uploads/{id}` | Upload bytes into a previously begun staging slot | Internal — upload staging | `none` | transport only |
| PUT `/v1/staged-uploads/{id}/chunks` | Append one bounded chunk to a modpack or world archive upload | Internal — upload staging | `none` | transport only |
| GET `/v1/staged-downloads/{id}` | Download bytes from a previously prepared staged export | Internal — download staging | `worlds` | transport only |

### Approved API-only exceptions (D-041)

Cameron approved two bounded exceptions on 2026-09-29. Built-in gamerule
lookup remains available through the API and desktop app; CLI users can send a
gamerule through the sole raw path, `msc command`. Router symptom analysis
remains available through the API and desktop app; the CLI keeps router guide
catalog, search, and reading through `msc help`. No other user-facing route
remains without a CLI task or an explicit internal classification.

P17.25 and P17.26 add commands for the server setup and player/pack maintenance
rows. The Geyser resource-pack toggle command reaches the existing route, which
currently reports that toggling is unsupported until the Bedrock client-pack
store is implemented. This is documented behavior, not a successful toggle.

### Router-only surfaces and contract gaps

P17.23 adds the three live HTTP routes that were missing from OpenAPI: `/v1/broadcast/credentials/clear`, `/v1/servers/playit`, and `/v1/servers/xbox-broadcast`. Path-parameter spelling differences (`:slot_id` versus `{slotId}`) are the same routes. The live router also exposes WebSocket streams (`/v1/console/stream`, `/v1/operations/:id/stream`, `/v1/notifications/stream`) and a health probe (`/v1/healthz`). WebSocket payloads and authentication are specified separately in `docs/msc2/api-contract/websocket-v1.json`; console follow and operation status use those contracts, while desktop notifications and the probe remain internal CLI surfaces.

### P17.7 local CLI transport and migration

Every CLI command that uses the management API first obtains a short-lived
credential from the host-local OS endpoint, keeps it in process memory, then
calls the agent at `127.0.0.1:48001`. Linux and macOS use the private Unix
socket; Windows opens the named pipe and sends its version-1 hello. If the
agent is stopped or unavailable, the CLI says to confirm it is installed and
running. If the OS account is not authorized, the CLI says to run as the
account that installed the agent. JSON output remains available with
`--json`.

The direct-remote CLI options `--host`, `--base-url`, `--port`, `--token`, the
`MSC2_CLI_TOKEN` environment variable, and `msc token print` are removed. Run
the same task command locally or through SSH as the installing account, for
example `msc status --json`. Remote host management stays in the Tauri
desktop; this change does not affect desktop pairing or credentials.

The CLI now covers the planned local authentication, service control, server/player/world/backup tasks, supported catalogs, files, help, and operation recovery. P17.25–P17.27 close the route inventory with two owner-approved API-only exceptions. There is no API for changing OS service state, by design; P17.8 uses local platform service managers. CurseForge modpacks have archive inspection/import and manual-file recovery, but no modpack search route; Phase 17 does not promise such browsing. `GET /v1/worlds/convert/formats` is available through `msc world convert-formats`.

### Step ownership for the route audit

P17.11 access administration; P17.12 server list/detail/selection/export; P17.13 performance, sessions, and console follow; P17.14–P17.15 player tasks; P17.16 world/backup gaps; P17.17 world packs; P17.18 host/network settings; P17.19 catalog inspection and installed add-ons; P17.20 modpack recovery; P17.21 operations/host reset; P17.22 files/help. P17.23 checked the shipped CLI against every inventory row. P17.25–P17.26 add the approved commands; D-041 records the two API-only exceptions.
