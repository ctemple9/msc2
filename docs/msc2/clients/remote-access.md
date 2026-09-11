# MSC 2 remote access boundary

**Status:** P14.17 documentation; the implementation and connection lifecycle
are defined by P14.11–P14.16.

MSC does not provide a cloud relay and does not require Tailscale. A client and
agent on different computers need a reachable path that the operator controls:

| Route | What it means | MSC's role |
|---|---|---|
| LAN or DNS | Both computers already share a reachable private network. | Save the address and try the direct authenticated connection. |
| Tailscale | An optional private overlay supplies a tailnet address or name. | Treat it as another direct route; Tailscale provides reachability, not authorization. |
| User-operated VPN or overlay | The operator supplies another private network path. | Save its address and use the normal agent credential. |
| Managed SSH tunnel | SSH reaches the host even when its agent port is not directly reachable. | Forward desktop `127.0.0.1:<local-port>` to remote `127.0.0.1:48001`, remember the host key, and clean up the session. |
| Existing tunnel or relay | The operator maintains the transport, or chooses a third-party relay. | MSC can use a manually maintained local endpoint, but does not operate or endorse a relay as part of the product. |

There is no “Tailscale without Tailscale” magic. If a LAN, DNS, VPN, SSH
tunnel, or other route does not connect the two computers, MSC cannot create
reachability on its own. Router port forwarding and public exposure are
possible network choices, but they add firewall, encryption, authentication,
and maintenance responsibilities. They must never be confused with exposing
Minecraft's player port: the management API is a separate administrator path.

## Default MSC path

The Tauri desktop tries the saved direct LAN or Tailscale route according to the
host's preference. If direct access is unavailable, it can own an SSH tunnel
whose remote target is the agent's loopback-only management service on
`127.0.0.1:48001`. The desktop side binds the chosen local port to loopback;
`48002` is only the example default and must move if occupied. A manually
maintained tunnel remains a recovery path for operators who already manage SSH
themselves.

The tunnel is transport, not authorization. After the route is available, MSC
still uses the host-scoped bearer credential and the agent's normal permission
checks. The remote setup flow may invoke only the fixed desktop pairing
bootstrap, `msc pairing create --client-kind desktop --json`; it cannot install,
start, stop, replace, or uninstall the remote operating-system service.

## CLI shape

The headless CLI uses the same management endpoint and authentication boundary.
For a direct private route:

```text
msc --host 192.168.1.42 --port 48001 status
```

For an operator-maintained SSH forward:

```text
ssh -N -L 127.0.0.1:48002:127.0.0.1:48001 user@server-host
msc --host 127.0.0.1 --port 48002 status
```

The CLI does not install or control the operating-system service on a remote
host. Service installation and lifecycle actions remain local operations on
the machine that hosts the agent.

## Related contracts

- Agent-served guidance: `content/help/handbook/remote-access.md`
- Host profile and route lifecycle: `docs/msc2/msc2-engineering.md` §5 and
  `docs/msc2/remote-host-profile-migration.md`
- Authentication boundary: `docs/msc2/clients/phase11-auth.md`
- Player-facing networking: `content/help/handbook/networking-basics.md`
