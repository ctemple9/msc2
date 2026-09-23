# macOS privileged Bedrock helper acceptance

P15.75 uses the read-only inspector below on the owner Intel Mac:

```text
python3 tools/phase15/inspect_macos_bedrock_helper.py --live --server-port 19001
```

The inspector does not install, start, stop, restart, or delete anything. It
reads `launchd`, plist metadata, file ownership and modes, process state, the
existing agent/helper logs, TCP `19001`, and UDP `19002-19033`. A failed result
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
- Every UDP port `19002-19033` has a live listener.
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
`HTTP/1.1 404 Not Found`, UDP `19002-19033` were all bound, no rollback was
present, and only the launchd-managed root helper process remained.
