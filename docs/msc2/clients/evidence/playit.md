# Playit provisioning and cross-platform evidence

This worksheet is the live-evidence companion for P12.20j. The automated
Rust/API check proves the release pins, acquisition boundary, lifecycle code,
and contract shape. Cameron's live run proves the real Playit account,
agent/tunnel reuse, helper process, server lifecycle, reset behavior, and the
addresses that P12.21/P12.22 display.

Do not record a Playit password, agent key, session token, secret bridge
contents, or an unredacted screenshot of credentials. Public addresses may be
recorded only when needed to verify the client display; redact them in shared
artifacts.

## Pinned helper matrix

`playitd` is MSC's cache and process name. The Linux and Windows assets below
are the exact upstream `playit-agent` release assets; every download is
selected from the pinned release metadata and checked against the listed
SHA-256 before promotion.

| Host target | Release | Asset | SHA-256 | Playit status |
| --- | --- | --- | --- | --- |
| macOS x86_64 | `playitd-v1.0.10` | `playitd` | `91ae745a35aad7a058a9bfb3320d7dc27a54f66a8bb81831360966dd69acc791` | Supported |
| macOS arm64 | `playitd-v1.0.10` | `playitd` | `91ae745a35aad7a058a9bfb3320d7dc27a54f66a8bb81831360966dd69acc791` | Supported |
| Linux x86_64 | `v1.0.10` | `playit-linux-amd64` | `2df7d9f10227ab312b1ad341853db4e8a8243df5cfcdbae58713a4271711c339` | Supported |
| Linux arm64 | `v1.0.10` | `playit-linux-aarch64` | `4c0db3e7b3a8158e249441c2f0b73f54e83429395890c7b1ca45fd7a6303d763` | Supported |
| Windows x86_64 | `v1.0.10` | `playit-windows-x86_64-signed.exe` | `2dbdaad119844cbbc062cc9774b8b462afa5f1b4b7832a9fc5ef4676cae887cf` | Supported |
| Windows arm64 | — | — | — | Explicitly unsupported: no pinned upstream asset; acquisition must return `helper_unavailable` before download or launch. |

The saved `playitEnabled` value is a server preference. It is not evidence
that the helper is available or running; use `isRunning`, the addresses, and
the returned error/operation state for that claim.

## Live run scope

Run every applicable row on each supported host target available for testing.
Record the host target, MSC build, server name, Playit agent label (redacted),
and result in the row. If a physical architecture is unavailable, leave the
row pending instead of substituting another architecture.

| Scenario | Action | Expected evidence | Result |
| --- | --- | --- | --- |
| First-use sign-in and claim | On a clean host, enable Playit and submit credentials through MSC's setup flow. Complete sign-in and claim. | Setup reaches success; the host-scoped key is stored locally; the agent is claimed/reused; no password or session appears in logs or API responses; the Java tunnel receives a public address. | Pending |
| Existing-agent reuse | Run setup again for the same host/account after the first run. | The existing agent and named `MSC Java` tunnel are reused; no duplicate agent or tunnel is created; the public address remains the inventory value. | Pending |
| Java-only | Use a Java server without Bedrock/Geyser or Simple Voice Chat. Start the server with Playit enabled. | Only `MSC Java` is provisioned and shown; helper starts with the protected secret-path bridge; automatic stop stops the helper when the server stops. | Pending |
| Java + Bedrock | Use a Java server with the Bedrock/Geyser path enabled. | `MSC Java` uses TCP and `MSC Bedrock` uses UDP; both addresses are persisted and shown; neither tunnel is duplicated on restart. | Pending |
| Java + Bedrock + voice | Use a server with Simple Voice Chat installed and enabled. | `MSC Voice` uses UDP port `24454`; Java, Bedrock, and voice addresses are all persisted and reported; the server's `voice_host` is synchronized. | Pending |
| Later SVC installation | Add or enable Simple Voice Chat after Playit is already configured. | MSC detects the later add-on, offers the narrower voice setup, reuses the existing agent, creates only the missing voice tunnel, and explains that a running server needs a restart to read `voice_host`. | Pending |
| Automatic stop | Let the first-start flow or normal server lifecycle stop the helper automatically. | Playit stops after the managed server stops; no orphan helper remains; the saved addresses and preference are preserved. | Pending |
| Restart recovery | Stop and restart the managed server and its Playit helper. | The helper is reacquired from the pinned cache when needed, recovers the saved agent/tunnels, and does not create duplicates. | Pending |
| Reset preserves cloud resources | Use Reset after stopping the helper, then inspect the host and Playit dashboard. | Local key, agent ID, addresses, secret bridge, and prompt state are cleared; the Playit cloud agent and tunnels remain; a later setup can reuse or claim them again. | Pending |
| P12.21/P12.22 public addresses | Open How to Connect and Console Access/Services for the active server. | The client shows the API inventory's Java/Bedrock/voice public addresses, with masking and copy behavior; it does not invent an address while the helper is not running. | Pending |

## Run record

Copy this block once per host target tested:

```text
Host target:
MSC build/commit:
Server type and add-ons:
Playit agent label (redacted):
Date:
Rows completed:
Rows pending and why:
Unexpected behavior:
Screenshots/logs with secrets removed:
```

The Windows arm64 row remains a contract check, not a live provisioning target,
until an upstream or repository-owned asset is published and pinned.
