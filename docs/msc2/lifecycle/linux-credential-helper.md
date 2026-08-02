# Linux credential helper for Phase 4

**Status:** P4.3 implementation decision. Proposed until Cameron verifies P4.3.
**Source of truth:** `docs/msc2/substrate/secret-storage.md` section 12,
`docs/msc2/substrate/service-identity.md`, `docs/msc2/lifecycle/phase4-scope.md`,
and `msc2-decisions.md` D-025.

Phase 4 should build the helper. The file-based `LinuxSecretStore` from P3.11
remains available for local development and tests, but it is not good enough for
the Phase 4 Linux service gate once real `systemd` registration exists.

## Decision

Build the privileged Linux credential helper in P4.23, alongside the real
`systemd` unit installation.

This follows the P3.11 two-track decision instead of reopening it:

- Track 1, the privileged helper backed by `systemd-creds`, was confirmed as
  the real target once Phase 4 service registration exists.
- Track 2, the file-based `LinuxSecretStore`, was an explicit stand-in because
  Phase 3 had no service registration surface where a privileged helper could
  honestly be installed and tested.

P4.23 is exactly that missing surface. It installs the agent unit, proves the
agent runs under `User=`/`Group=` as the installing user, and already needs
install-time elevation. Adding the helper there does not create a new routine
privilege path; it uses the same installer-elevation window.

## Product Meaning

For Cameron or a future user, this means a headless Linux install has the same
shape as the product promise:

- Running the server day to day does not ask for `sudo`.
- Pairing a phone or CLI token after install does not ask for `sudo`.
- Secrets are not left in the weaker Phase 3 file store once the Linux service
  is installed for real.
- If the helper cannot be installed, the Linux Phase 4 service gate fails
  loudly instead of silently accepting the stand-in as the permanent answer.

The file-based store still has a use: non-service local development, unit tests,
and temporary agent runs where no `systemd` unit was installed. It is not the
accepted backend for the Linux headless-service proof.

## Binary Shape

Keep P4.1's one-binary decision. The helper is a hidden service mode of the
same `msc` binary, not a second shipped artifact.

Planned service command:

```text
msc credential-helper serve --allowed-uid <installing-user-uid> --store-dir /var/lib/msc2/credentials
```

The command is not a user-facing CLI feature. It exists so the `systemd` helper
unit can run the same signed headless artifact as root for one narrow job:
reading, writing, and deleting encrypted credential blobs.

## Installation Boundary

The installer, already elevated for service registration, does all privileged
setup in one window:

- Installs the agent unit with `User=`/`Group=` set to the installing user.
- Installs the credential-helper socket and service units as root-owned files.
- Creates `/var/lib/msc2/credentials` owned by root with mode `0700`.
- Starts/enables the helper socket, not a long-running helper process.

After that point, normal agent operation is unprivileged. The agent talks to the
socket as the installing user. Updating helper unit files, changing the allowed
UID, or moving the root-owned store directory requires the same elevated
service-management path as initial installation.

## Socket Permissions

Use a Unix domain socket managed by `systemd` socket activation.

Socket path:

```text
/run/msc2/credential-helper.sock
```

Required socket properties:

```ini
SocketUser=<installing-user>
SocketGroup=<installing-user-primary-group>
SocketMode=0600
RemoveOnStop=yes
```

The helper must also verify the connecting process with peer credentials
(`SO_PEERCRED`) and reject any UID other than the configured installing-user UID.
The filesystem mode is a first filter; peer-credential checking is the authority.

Root may install, start, stop, and debug the helper through `systemctl`, but the
agent protocol itself is for the installing-user agent only. Do not make the
socket group-writable for convenience; that would quietly widen who can mint or
read MSC bearer-token verifiers on a shared Linux host.

## Request Protocol

Use one newline-delimited JSON request per connection. The helper replies with
one newline-delimited JSON response and closes the connection.

Request envelope:

```json
{"version":1,"op":"get","key":"remote-api.token.example"}
```

Operations:

| Operation | Request fields | Success response |
|---|---|---|
| `get` | `version`, `op`, `key` | `{"ok":true,"value":"..."}` or `{"ok":true,"value":null}` |
| `set` | `version`, `op`, `key`, `value` | `{"ok":true}` |
| `delete` | `version`, `op`, `key` | `{"ok":true}` |
| `ping` | `version`, `op` | `{"ok":true}` |

Failure response:

```json
{"ok":false,"error":{"code":"invalid_key","message":"credential key is not allowed"}}
```

Protocol rules:

- Maximum request size: 64 KiB.
- Maximum plaintext value size: 32 KiB. Phase 4 stores token verifiers, not
  large documents.
- Keys must match `^[a-z0-9][a-z0-9.-]{0,191}$`.
- `..`, `/`, backslash, whitespace, uppercase, empty keys, and control
  characters are rejected before touching the filesystem.
- `delete` of a missing key returns success, matching the `SecretStore`
  contract.
- `get` of a missing key returns `value: null`, matching the `SecretStore`
  contract.
- The helper must never log plaintext values or bearer tokens.

This is intentionally smaller than a general RPC system. It is just the live
`SecretStore::get` / `set` / `delete` shape that `systemd-creds` does not expose
to an unprivileged process.

## Storage Behavior

The helper stores one encrypted credential blob per `SecretStore` key under:

```text
/var/lib/msc2/credentials/<safe-key>.cred
```

The key validation above makes `<safe-key>` a direct filename, not a path to
sanitize after the fact.

Behavior:

- `set` encrypts the plaintext through `systemd-creds`, writes a temporary blob
  in the same directory, sets mode `0600`, and atomically renames it into place.
- `get` decrypts the matching blob through `systemd-creds`.
- `delete` removes the matching blob if present.
- Missing files map to `None` / success, not errors.
- Corrupt or undecryptable blobs are errors, because silently treating them as
  missing would turn credential corruption into an unexplained auth failure.

P4.23 should confirm the exact `systemd-creds` command-line flags against the
Debian 12/systemd >= 250 target while implementing. This P4.3 decision fixes the
helper boundary and protocol; it does not pretend the CLI syntax is the product
contract.

## Relationship To Pairing

P4.5's real credential path calls `SecretStore` the same way on every platform.
On Linux service installs, the platform `SecretStore` implementation should use
this helper. The server-side keys from P4.2 stay unchanged:

```text
remote-api.token.<credential-id>
```

The helper stores token verifiers, not raw bearer tokens. It does not create
credentials, decide permissions, know token roles, or bypass route-level auth.
It only provides Linux's privileged at-rest storage operation.

## Phase 4 Verification Impact

P4.23's Linux service check must include helper evidence:

- The helper socket exists at `/run/msc2/credential-helper.sock`.
- The socket is owned by the installing user and mode `0600`.
- A process running as a different unprivileged UID cannot connect.
- The agent, running as the installing user, can `set`, `get`, overwrite, and
  `delete` a test key through the helper.
- Encrypted blobs land under `/var/lib/msc2/credentials`, not the user's
  Phase 3 file-store directory.
- The agent can pair or bootstrap a Phase 4 token through the same API path
  without any runtime `sudo`.

If any of those fail, the Linux Phase 4 service gate is not closed.

## Open Items Left For P4.23

These are implementation details, not product questions:

- Exact `systemd-creds encrypt` / `decrypt` invocation flags on Debian 12.
- The final unit names.
- The final hardening directives after testing against TPM and non-TPM hosts.
- Whether helper storage is a direct file per key or gains a small metadata file
  for versioning before v1.

The decision that matters for planning is closed here: Phase 4 builds the helper
and does not accept the file-based Linux stand-in as the Linux service-gate
backend.
