# Remote host profile migration

**Introduced by:** P14.11
**Profile schema:** version 2
**Scope:** desktop client metadata only

P14.11 replaces the temporary remote-host shape
`{ id, label, baseUrl }` with an editable connection profile. The stable `id`
continues to identify the agent host and remains the key for in-memory caches
and the native bearer credential. Changing a hostname, LAN address, Tailscale
address, or route preference does not create another logical host.

## Stored profile

The desktop stores this JSON-safe metadata in `localStorage` under
`msc2.saved-remote-hosts`. The current value is an envelope with
`{ version: 2, hosts: [...] }`:

| Field | Meaning | Secret? |
|---|---|---|
| `id` | Stable agent host identity | No |
| `displayName` | Name shown in the host switcher | No |
| `lanAddresses` | Ordered LAN hostnames, IPs, or HTTP(S) origins | No |
| `tailscaleAddresses` | Ordered Tailscale hostnames, IPs, or HTTP(S) origins | No |
| `preferredRouteOrder` | Route families to try first (`lan`, `tailscale`) | No |
| `ssh.hostname` | SSH host or IP | No |
| `ssh.port` | SSH port, normally `22` | No |
| `ssh.username` | SSH account name | No |
| `ssh.authentication` | `password`, `private-key`, or `agent` | No |
| `ssh.privateKeyPath` | Path or OS key reference, if selected | No |
| `managementPort` | Remote MSC management port, default `48001` | No |
| `localForwardedPort` | Client-side tunnel port, default `48002` | No |

The profile has no fields for SSH passwords, private-key contents, bearer
tokens, pairing codes, or host-key material. Passwords are session-only unless
a later, explicit secure-store decision adds a remembered credential. Bearer
credentials remain in the existing native per-host secure store.

## Migration behavior

`loadSavedRemoteHosts()` accepts both the old array of `{ id, label, baseUrl }`
records and the version-2 envelope. Legacy records are converted in memory as
follows:

- `label` becomes `displayName`.
- `baseUrl` becomes the first LAN address.
- The address hostname becomes the initial SSH hostname when it can be parsed.
- `preferredRouteOrder` starts with LAN, then Tailscale if one exists.
- SSH defaults to port `22` and `agent` authentication.
- `managementPort` defaults to `48001`, and the local forward defaults to
  `48002`.

The next save writes the version-2 envelope and only the canonical metadata
fields. Invalid, duplicate, or `local-agent` records are ignored during load.
The migration does not touch the native secure store: its host-keyed bearer
credential remains owned by the existing Tauri bridge.
