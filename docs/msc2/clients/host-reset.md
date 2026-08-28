# Host reset and recovery contract

**Status:** Approved in D-029 · contract frozen by P12.19a · 2026-08-28

This document defines the boundary between clearing one client's remembered
state and clearing the host that owns MSC's server state. It is normative for
the client work in P12.19c–d and the agent work in P12.19b.

## Two different resets

### Reset this client

This is a local client operation. It does not contact the agent and does not
change anything on a host. It clears only the current device's:

- remembered host records and selected-host state;
- host-scoped credentials;
- preferences, including per-host presentation preferences; and
- first-launch, Concept Guide, and guided-tour state.

It does not delete server files, host configuration, host credentials, or the
agent service. A remote host remains installed and unchanged. The client
returns to its normal add-host/first-launch entry point.

### Reset this host

This is `POST /v1/host/reset`. The route acts on the host serving the request;
there is no `hostId` selector in the request, so a client cannot accidentally
send a reset intended for one host through another host's connection.

The route requires an administrator credential and the exact confirmation
string `RESET <current-agent-host-id>`. The client must show the human-readable
host label and the resolved server root before it asks for that confirmation.
The agent compares the supplied host ID with its current identity before it
starts the operation. A stale client therefore cannot confirm a different host
after an identity change.

The route refuses with `409 server_running` if any managed Minecraft server is
running, and with `409 reset_in_progress` if another host reset owns the reset
lock. Reset is admitted as a long-running operation so deletion, credential
revocation, and identity rotation are journaled and recoverable after an agent
restart.

## Deletion boundary

The following are logical paths under the platform-resolved agent data and
server-root directories. The logical names are stable across macOS, Windows,
and Linux; the platform adapter supplies their OS-specific locations.

| Mode | Removed | Preserved |
|---|---|---|
| `configuration` | `config/host.json`; `config/servers.json`; `host/identity.json`; every record under `auth/credentials/`, `auth/sessions/`, and `auth/pairings/`; reset-owned entries under `operations/` | The complete configured `servers-root/` tree, including Minecraft worlds, jars, logs, backups, and server-local configuration files; the installed agent service |
| `everything` | Everything in `configuration`, plus the complete configured `servers-root/` tree | The installed agent service, unless the local desktop separately uninstalls its own service after the operation succeeds |

The agent deletes only those allowlisted paths. It resolves the configured
server root before deletion and refuses path traversal, symlink escape, root,
home-directory, or out-of-scope targets. It never deletes arbitrary paths
named by the client. `everything` removes the managed tree, not the parent
directory that contains it.

The reset clears host setup/configuration, revokes all existing browser,
desktop, iOS, CLI, and named-token credentials, expires outstanding pairing
challenges, and creates a new host identity. No old credential remains valid
after the reset is committed. The reset result identifies whether the agent
will restart, is unavailable, or remains installed but needs pairing; clients
must not infer that state from a dropped connection.

## Service ownership and recovery

The agent route does not call an operating-system service manager. A local
desktop may, after a successful `everything` reset, stop and uninstall only its
own local service, forget its local credential, and show **Install and
Continue**. That is a native client action, not an HTTP capability and not
available for a remote host.

For a remote reset, the host service remains installed. The operator must run
the host-local pairing command, which displays a short-lived one-use pairing
code once and never logs it. The remote client uses **Pair Again** or **Add
Host**, exchanges that code, stores the returned host-scoped credential, and
reopens host setup. Old credentials, cached host state, and the previous host
ID cannot authorize the client through this path.

For either reset mode, first-time setup may hand off to the Add Server wizard.
It must not create a Minecraft server as a side effect of reset or recovery.

## Wire contract

The public route and DTOs live in `docs/msc2/api-contract/openapi.json`:

- `POST /v1/host/reset` is `202` operation-backed and `x-permission-category:
  admin`.
- `HostResetRequestDTO` carries `mode` and the exact `confirmation` value.
- `HostResetAcceptedDTO` carries the operation ID, the old host ID, selected
  mode, a human-readable message, and `agentState` (`restarting`,
  `needs_pairing`, or `unavailable`).
- All failures use `ErrorDTO`: `400 invalid_body`, `403 forbidden`, `409
  server_running` or `reset_in_progress`, and `500 internal_error`.

The operation's terminal result is the source of truth for completion. A
client must handle the accepted response as the last reliable response from
the old credential and must not retry the destructive request automatically.
