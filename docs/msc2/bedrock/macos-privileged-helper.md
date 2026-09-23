# macOS Bedrock privileged helper contract

**Status:** Owner-approved on 2026-09-23 under D-007 and D-025

**Scope:** Intel macOS Virtualization.framework Bedrock backend only

**Evidence:** P15.69 live diagnosis on macOS 15.7.8

## 1. Why this boundary exists

Bedrock Dedicated Server has no macOS build, so MSC runs it in a lightweight
Virtualization.framework guest and relays its host ports. P15.69 separated that
runtime into four observable layers:

1. BDS reached `Server started` inside the guest.
2. The host reached BDS directly at the VZ NAT address and received its HTTP
   response.
3. The identical signed sidecar relayed successfully from a foreground user
   session.
4. From MSC's installing-user system LaunchDaemon, Network.framework and POSIX
   sockets both failed to reach the guest; the POSIX error was `EHOSTUNREACH`.

That evidence justifies changing the VZ process context. It does not justify
running MSC's API, server manager, downloads, backups, or arbitrary filesystem
work as root.

## 2. Fixed ownership model

| Component | OS identity | Owns |
|---|---|---|
| Rust MSC agent | Installing user | Configuration, API, operations, server catalog, policy, diagnostics, and supervision |
| Swift Bedrock helper | Root LaunchDaemon with no `UserName` | One VZ VM session and that session's TCP/UDP host relays |
| BDS guest process | Installing user's configured numeric UID/GID at the shared-folder boundary | Bedrock server files written through the shared directory |

The helper is a mechanism behind the agent, not a second server manager. It has
no public API, user accounts, bearer tokens, scheduling, or independent server
catalog. The agent remains the only component that decides which configured
server may start.

## 3. Control channel

Installed operation uses one local Unix-domain socket. The control socket:

- lives below a root-owned runtime directory;
- is owned by the configured installing UID and has mode `0600`;
- has no TCP or UDP equivalent and is never exposed beyond the host;
- carries the versioned, newline-delimited JSON sidecar command/event protocol;
- applies a fixed maximum line/request size and closes on malformed framing;
- rejects unknown protocol versions, message kinds, and fields that would
  broaden authority;
- obtains the peer credentials from the accepted macOS socket and compares the
  peer UID with the root-owned configured installing UID before reading a
  request.

Filesystem mode is defense in depth, not authentication by itself. The helper
must perform the peer-UID check even when the socket appears correctly owned.
It must never trust a UID, username, path, or privilege claim supplied inside a
client message.

The permitted protocol is the minimum needed to provision the fixed appliance,
start and stop BDS, deliver BDS console input, report console/lifecycle events,
and tear down the session. It cannot execute a host command, select an arbitrary
host executable, load an arbitrary VM appliance, read an arbitrary host file,
change its allowlist, install software, or mutate its own service definition.

## 4. Shared-path rules

The helper configuration names the installing UID/GID and one or more approved
Bedrock root directories. That configuration is written only during an
administrator-authorized service transaction and is root-owned and not
group/other-writable.

Before creating a shared directory, the helper must:

1. require an absolute server path;
2. resolve the existing path and every symlink to a canonical path;
3. compare path components, not string prefixes, against a canonical approved
   root;
4. reject a missing root, `..` escape, symlink escape, filesystem alias that
   resolves outside the root, or a path equal to a broader host directory;
5. reject a server directory not owned by the configured installing user unless
   a later owner-approved import contract explicitly defines a safe exception.

The default allowlist is MSC's managed Bedrock server root. Supporting an
external server root requires adding that canonical root during an
administrator-authorized configuration change; a routine start request cannot
expand the allowlist.

The helper must not solve an invalid path or ownership state by recursively
changing permissions or ownership. A rejection is returned to the agent as a
specific diagnostic before VM creation.

## 5. Privileged artifacts and installation

These are privileged artifacts:

- helper executable;
- LaunchDaemon plist;
- fixed VM appliance and boot resources;
- helper configuration and approved-root list;
- parent runtime directory.

They are installed atomically as root, owned by root, and not writable by the
installing user, group, or everyone. Production may not execute a helper or load
an appliance from a build directory, download staging directory, user-writable
application-support directory, or path selected in an IPC request.

Install, upgrade, configuration change, bootstrap, bootout, and uninstall are
one coherent extension of MSC's existing administrator-authorized macOS service
transaction. A partial failure rolls back to the previous complete agent/helper
pair or leaves both stopped with a precise recovery instruction. Routine
Bedrock start, stop, command, status, and console operations do not ask for an
administrator password.

The helper must remain available before login. Moving it to a LaunchAgent or
requiring the desktop application would violate D-011's headless contract.

## 6. Session and failure semantics

One authenticated agent connection owns at most one VM. The helper rejects a
second provision/start session instead of merging ownership or abandoning the
first VM. The owning session ends when the agent requests teardown, the socket
disconnects, the helper exits, or an unrecoverable VM/relay failure occurs.
Ending the session stops BDS when possible, stops the VM, closes every relay,
and removes the control socket only when the helper itself exits.

At startup the helper removes a stale socket only after proving that no live
helper owns it. Agent and helper restarts reconcile stale state and must not
leave sidecars, VMs, or listeners detached under PID 1.

An installed agent treats these as distinct failures:

- helper not installed or not running;
- socket has the wrong owner, mode, or file type;
- helper rejected the peer UID;
- protocol version mismatch;
- server path or ownership rejected;
- privileged artifact ownership/signature invalid;
- VM failed before BDS startup;
- BDS failed inside a healthy VM;
- TCP signaling relay failed;
- UDP gameplay relay failed;
- helper disconnected and tore the session down.

The agent surfaces the first meaningful failure in Bedrock diagnostics and the
operation result. It must not collapse these into a generic “Bedrock could not
start” message. In an installed service it must not silently fall back to
spawning the sidecar as the user-owned daemon. Direct child-process mode is an
explicit development option and must identify itself as such in diagnostics.

## 7. File-ownership invariant

Starting, running, stopping, crashing, and restarting Bedrock through the root
helper must leave configuration, worlds, logs, allowlists, permissions, and
backups owned as though the installing user had run BDS directly. Guest writes
must therefore use or map the configured installing UID/GID at the shared-folder
boundary. Root-owned files appearing inside a server directory are a failed
invariant, not cleanup work for the agent.

The helper may write its own root-owned service logs and transient runtime state
only outside server roots. It never writes backup archives, edits
`server.properties`, or rewrites MSC's server catalog; those remain agent work.

## 8. Threat model

The installing user already controls MSC and their Minecraft servers. The
helper may let that user run the fixed Bedrock VM against an approved server
directory; it must not let that user turn root authority into arbitrary host
code execution or arbitrary filesystem access.

The boundary specifically defends against:

- replacing a user-writable helper, appliance, plist, or configuration and
  having launchd execute it as root;
- connecting as another local account;
- substituting `/`, `/etc`, another user's home, or a symlink escape as the
  shared server directory;
- injecting an unsupported command or unbounded request into privileged IPC;
- accumulating orphan VMs/listeners after either process crashes;
- silently weakening security because the installed helper is missing or
  incompatible.

It does not claim to defend against an attacker who already has root, a
compromised macOS kernel/hypervisor, or malicious code already executing as the
installing user reading that user's own Minecraft data. The helper still limits
the last case so control of the agent account does not automatically become
general root execution.

## 9. Acceptance obligations

Implementation is not accepted until later Phase 15 steps prove all of these on
the owner Intel Mac:

- the agent runs as the installing user and the helper runs as root with no
  plist `UserName`;
- privileged artifacts, runtime directory, and socket match this ownership and
  mode contract;
- an unauthorized UID and an out-of-root/symlink path are rejected before VM
  creation;
- BDS remains running after startup, and TCP signaling plus the bounded UDP
  relay set are reachable through the helper-owned host listeners;
- server/world files remain owned by the installing UID/GID;
- restarting either process leaves no orphan VM, sidecar, or listener;
- installed operation refuses unsafe fallback, while explicit development mode
  remains usable for diagnostics.

NetherNet public UDP advertisement and Xbox Broadcast acceptance are subsequent
network-compatibility gates. They cannot substitute for this helper boundary
passing its own startup, ownership, and teardown checks.
