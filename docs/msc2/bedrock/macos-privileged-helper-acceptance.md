# macOS privileged Bedrock helper acceptance

P15.75 uses the read-only inspector below on the owner Intel Mac:

```text
python3 tools/phase15/inspect_macos_bedrock_helper.py --live --server-port 19001
```

The inspector does not install, start, stop, restart, or delete anything. It
reads `launchd`, plist metadata, file ownership and modes, process state, the
existing agent/helper logs, TCP `19001`, and UDP `19002-19017`. A failed result
must be treated as evidence about the installed pair, not worked around by
changing router or Xbox settings.

## Required evidence

- `com.ctemple.msc2.agent` is loaded and its process runs as the installing user.
- `com.ctemple.msc2.bedrock-helper` is loaded and its process runs as root.
- The helper plist has no `UserName` and names the configured installing UID.
- The helper executable, appliance files, plist, and runtime directory are
  root-owned and not group/other-writable.
- The control socket is a user-owned `0600` Unix socket.
- Existing logs contain `Server started` without a start-operation rollback.
- TCP `19001` returns an HTTP response from BDS through the host relay.
- Every UDP port `19002-19017` has a live listener.
- No Bedrock sidecar or VM process remains outside the loaded helper.

## Live result — 2026-09-23

The first inspector run found no installed or loaded helper. After the agent
was started, the helper transaction was present and its root-owned artifacts
passed inspection, but launchd reported `state = spawn scheduled`, `last exit
code = 1`, and no socket. The helper log identified the evidence-backed cause:
`unknown service argument: /var/run/msc2/bedrock.sock`. The `--service` flag in
`BedrockSidecarMain.swift` was incorrectly advancing past `--socket-path`.
P15.75 fixes that parser bug. The installed helper must be rebuilt and
reinstalled before repeating the live relay check; the current run therefore
does not claim BDS readiness, UDP binding, or restart cleanup.

P15.73/P15.74 verification and the administrator-authorized helper install are
the prerequisite for repeating the command and closing this acceptance step.

The subsequent live retry also exposed an installed-root mismatch: the helper
plist allowed `/Users/camerontemple/Library/Application Support/MSC2/servers`,
while the agent's actual data and Bedrock server directory are under
`/Users/camerontemple/Library/Application Support/MSC 2/servers`. The desktop
installer now derives the helper root from its own `agent_data_directory()` so
the two service components cannot silently choose different macOS paths.

Another live retry showed the remaining lifecycle failure: launchd recorded
`Broken pipe: 13` for the root helper after a client session closed. The
service wrote a response to the disconnected Unix socket while SIGPIPE still
had its default process-killing behavior. P15.75 now ignores SIGPIPE only in
the privileged service mode, allowing the existing write error handling to
close that session while keeping the helper available for the next agent
connection.

The authenticated desktop credential then reached the repaired agent and
helper. Its console showed the VM booting successfully but BusyBox rejecting
the unsupported `setpriv --reuid` launch syntax before BDS executed. The
appliance now uses its compiled-in `chpst` applet to assume the installing
UID/GID. A second live issue was in the TCP relay: the accepted client and
guest connection began copying independently, which could close the client
with an empty response. The relay now retains both Network.framework
connections and starts its two copy loops only after both report ready.

After rebuilding and administrator-authorized reinstall, an authenticated
`POST /v1/start` created operation `op-24878-1`. It completed with `Bedrock
server is ready.` The exact inspector command above reported `17 passed, 0
failed`: the helper log contains `Server started`, TCP `19001` returned BDS's
`HTTP/1.1 404 Not Found`, UDP `19002-19017` were all bound, no rollback was
present, and only the launchd-managed root helper process remained.

## P15.76 — explicit NetherNet mappings

The sidecar's 16 BDS UDP relays are the bounded range immediately after the
TCP signaling port. MSC now writes that range as individual public-to-private
mappings rather than BDS range shorthand. For the installed acceptance port
`19001`, the property must therefore enumerate public and private ports
`19002` through `19017` one at a time, for example
`PUBLIC_IP:19002:19002,...,PUBLIC_IP:19017:19017`. Xbox Broadcast retains its
separate UDP `19034-19049` ICE range.

The agent requires exactly one valid advertised IP address (public discovery,
with the existing LAN fallback) and validates the final `server-udp-ports`
value against the 16 relay listeners before BDS starts. The previous 32-entry
implementation was rejected by BDS with `server-udp-ports: too many port
mappings (max 16)` and cannot be used as acceptance evidence.

## P15.77 — Bedrock and Xbox Broadcast connection acceptance

The prescribed installed-pair report was run on 2026-09-23 before the
sixteen-mapping correction:

```text
python3 tools/phase15/inspect_macos_bedrock_helper.py --live --server-port 19001 --connection-report
```

The report passed `18 passed, 0 failed`. It confirmed the user-owned agent,
root-owned helper, authenticated socket, BDS `Server started` evidence, no
start rollback, the TCP `19001` relay returning `HTTP/1.1 404 Not Found`, and
the host listener report. Its 32-UDP-listener result is superseded because BDS
rejected the 32-entry property with a maximum of 16 mappings; the report must
be rerun after P15.78 is installed.
A same-host request to `http://10.0.0.142:19001/` also returned BDS's HTTP 404;
that proves the host's LAN-facing relay boundary, not an iPad session.

The report-only diagnostic was corrected during this step: `lsof` cannot see
the root helper's sockets from the installing-user context, so the connection
report now uses macOS `netstat` for listener evidence while retaining the
separate process-identity checks.

The product-level paths remain owner verification rather than completed
acceptance:

| Path | Result | Evidence or first boundary |
|---|---|---|
| iPad direct LAN → `10.0.0.142:19001` | Not yet exercised | Host-side LAN probe passed; no iPad session was available to this run. |
| iPad direct remote → public address | Not yet exercised | No cellular/off-LAN client session was available to this run. |
| Xbox Broadcast discovery and transfer on home LAN | Not yet exercised | The managed Broadcast JVM is running, but no discovery or transfer session was observed. |
| Reconnect without restarting BDS | Not yet exercised | No client session was available to disconnect and reconnect. |
| Two simultaneous clients | Not yet exercised | No two-client session was available. |

One installed-runtime prerequisite is also still open. The live Bedrock
`server.properties` observed during this run contains the pre-P15.76 value
`server-udp-ports=73.135.129.135:19002-19033:19002-19033`. The P15.76 source
was then corrected by P15.78 to write sixteen individual mappings, but this
running installation has not yet been restarted or refreshed from that build,
so this step does not claim remote NetherNet or Xbox acceptance. The next owner
run must refresh the installed agent/server from P15.78, confirm `19002`
through `19017` appear as individual public-to-private entries and BDS emits
no `too many port mappings` error, then perform each client path above and
correlate the client attempt with BDS, helper, relay, and Broadcast logs.
