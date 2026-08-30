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
string `RESET AGENT`. The client must show the human-readable host label and the
resolved server root before it asks for that confirmation. The reset still acts
only on the host serving the authenticated request; the short phrase is a
deliberate usability boundary, not a host selector.

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
| `configuration` | `config/host.json`; `config/servers.json`; `host/identity.json`; every record under `auth/credentials/`, `auth/sessions/`, and `auth/pairings/`; reset-owned entries under `operations/` | The complete configured `servers-root/` tree, the downloaded helper cache under `helpers/` (including the Broadcast JAR), and the installed agent service |
| `everything` | Everything in `configuration`, plus the complete configured `servers-root/` tree and the downloaded helper cache under `helpers/` | The installed agent service, unless the local desktop separately uninstalls its own service after the operation succeeds |

The agent deletes only those allowlisted paths. It resolves the configured
server root before deletion and refuses path traversal, symlink escape, root,
home-directory, or out-of-scope targets. It never deletes arbitrary paths
named by the client. `everything` removes the managed tree, not the parent
directory that contains it.

The reset clears host setup/configuration, removes full-reset helper artifacts,
revokes all existing browser,
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

## Playit local reset and re-authentication

`POST /v1/playit/reset` is the Playit-specific local reset. It requires the
networking permission and acts on the serving host's active Playit state; it
has no host or server selector. The agent stops and reconciles every managed
`playitd` helper before it removes the secret bridge, then clears the stored
Playit agent key, agent ID, Java/Bedrock/voice public addresses, and each
server's Simple Voice Chat tunnel prompt state. The Playit dashboard is not
called, so cloud agents and tunnels remain intact. Repeating the request is
safe and returns `already_clear` when no local key remains.

The desktop server editor only loads or mutates Playit for the currently
active server. It invalidates an in-flight status response when the active
host/server boundary changes, so a delayed response from the former selection
cannot repaint the new server's Playit panel.

After local reset, setup signs in again through the agent. The email,
password, and temporary Playit session exist only during that setup operation;
only the resulting host-scoped agent key is persisted. Setup then reuses a
matching local agent when one exists, or claims an agent and creates/reuses
only the tunnels applicable to the active server. A later voice setup follows
the same memory-only credential boundary and does not return a reusable
session to the client.

### Manual Playit reset walkthrough

1. Select the server that is active on the serving host, open **Broadcast →
   Playit**, and choose **Manage setup…**. Confirm the local-reset warning.
2. Confirm the agent reports the local key and public addresses as cleared,
   the `playitd` helper is stopped, and the server's
   `.msc2-playit/secret-bridge` file is gone. The Simple Voice Chat setup
   prompt should be available again when voice is installed.
3. Open the Playit dashboard separately and confirm the existing cloud agent
   and tunnels are still present.
4. Run the reset again to confirm the idempotent `already_clear` result, then
   sign in through **Set up…** and confirm setup can reuse or claim the agent
   and rebuild the applicable tunnel set without showing a password or
   reusable session in status or operation results.

## Wire contract

The public route and DTOs live in `docs/msc2/api-contract/openapi.json`:

- `POST /v1/host/reset` is `202` operation-backed and `x-permission-category:
  admin`.
- `HostResetRequestDTO` carries `mode` and the exact `RESET AGENT` confirmation
  value.
- `HostResetAcceptedDTO` carries the operation ID, the old host ID, selected
  mode, a human-readable message, and `agentState` (`restarting`,
  `needs_pairing`, or `unavailable`).
- All failures use `ErrorDTO`: `400 invalid_body`, `403 forbidden`, `409
  server_running` or `reset_in_progress`, and `500 internal_error`.

The operation's terminal result is the source of truth for completion. A
client must handle the accepted response as the last reliable response from
the old credential and must not retry the destructive request automatically.
