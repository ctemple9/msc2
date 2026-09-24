# Troubleshooting Bedrock on Intel macOS

**Incident window:** 2026-09-22 through 2026-09-23  
**Affected server:** official Bedrock Dedicated Server 1.26.51.1, build 51061372, protocol 2193  
**Affected client used for final proof:** Minecraft Bedrock 1.26.51 on iPadOS 18.7.8  
**Host:** Intel MacBook Pro at `10.0.0.142`, running BDS inside MSC's Virtualization.framework appliance  
**Public address during diagnosis:** `73.135.129.135`  
**Current working transport:** RakNet on UDP `19001`

This document records the full investigation because a later BDS release may
repair NetherNet and make it worth retesting. It distinguishes observed facts
from discarded theories and gives a safe rollback path. The Phase 15 step
history in `rolling-plan.md` remains the implementation record; this is the
operator-facing account of what happened and why.

## Final result

BDS 1.26.51.1's NetherNet signaling endpoint worked over plaintext HTTP but
rejected TLS before the WebRTC/ICE stage. The same BDS process returned:

```text
GET http://127.0.0.1:19001/v1/join
HTTP/1.1 200 OK

{"name":"Bedrock","protocol":2193,"version":"1.26.51",...}
```

but failed locally over TLS:

```text
GET https://127.0.0.1:19001/v1/join
LibreSSL: sslv3 alert handshake failure
```

A remote iPad attempt produced the same server response on the wire. The seven
payload bytes were:

```text
15 03 01 00 02 02 28
```

They decode as a TLS alert record (`15`), fatal severity (`02`), alert `0x28`
(decimal 40, `handshake_failure`). Because the TLS signaling exchange failed,
NetherNet never reached UDP candidate checks or gameplay traffic. No UDP
packets on the advertised ICE range were expected after that failure.

The successful compatibility configuration is:

```properties
server-port=19001
transport=raknet
```

`server-ip` and `server-udp-ports` are absent. MSC binds UDP `19001` on the Mac
and relays it to UDP `19001` in the VZ guest. Xfinity forwards UDP `19001` to
`10.0.0.142`. With that configuration Cameron successfully joined:

- directly from the iPad on the home LAN at `10.0.0.142:19001`;
- directly from the iPad over a cellular hotspot at
  `73.135.129.135:19001`;
- through MCXboxBroadcast build 155 while the iPad was on the hotspot.

This live evidence disproved the warning's practical implication that all
1.26.51 clients require NetherNet. BDS prints that NetherNet is the only
supported transport, but the tested 1.26.51 iPad still speaks RakNet.

## Current working network contract

| Boundary | Working value |
|---|---|
| BDS property | `server-port=19001` |
| BDS transport | `transport=raknet` |
| Mac host relay | UDP `0.0.0.0:19001` to guest UDP `19001` |
| Xfinity forward | UDP `19001` to `10.0.0.142` |
| LAN manual address | `10.0.0.142:19001` |
| Remote manual address | `73.135.129.135:19001` during this incident |
| Xbox Broadcast target | public address, port `19001` |
| MCXboxBroadcast build | 155, advertising Bedrock protocol 2193 |

TCP `19001` is not required in RakNet mode. Xfinity would not allow separate
TCP and UDP entries for the same port, so the former TCP `19001` rule was
replaced with UDP `19001`. UDP `19002-19049` belonged to the abandoned
NetherNet experiment and can be removed once no rollback test is pending.

The public address is historical evidence, not a permanent hostname. Confirm
the current public IP before using it in a future test.

## What the client errors meant

The iPad reported `InitialConnection-13`, `Transport: NetherNet:2193`, and the
codeword `NetherNet`. That message did not mean the UDP gameplay ports were
blocked. In this incident it meant the connection failed during the initial
TLS signaling exchange, before the client could submit its SDP offer and
before ICE selected any UDP path.

The server often logged no player connection or rejection because the request
never reached the normal Bedrock player-session layer. `Signed in to signaling
service successfully` and `Server started` proved startup health, not that the
public TLS handshake was usable.

## Evidence from the public path

The decisive capture was:

```text
sudo tcpdump -ni en0 -vv 'tcp port 19001 or udp portrange 19002-19017'
```

The cellular client at `172.58.241.250` completed a TCP handshake with
`10.0.0.142:19001`, sent 517 bytes, received seven bytes, and the server closed
the connection. There was no UDP traffic. This proved all of the following:

- the public address reached the Xfinity gateway;
- Xfinity's TCP forward reached the Mac;
- macOS accepted and answered the connection;
- the MSC TCP relay reached BDS;
- failure occurred after the client payload reached the server but before ICE.

The focused capture was:

```text
sudo tcpdump -ni en0 -s 96 -XX \
  'src host 10.0.0.142 and src port 19001'
```

Its response payload ended in:

```text
1503 0100 0202 28
```

The localhost HTTP/HTTPS comparison then reproduced the distinction without
Xfinity, cellular NAT, or the iPad in the path. That was the evidence that
stopped further router changes.

## Chronology and lessons

### P15.62-P15.65 — transport assumptions and the broken UDP relay

MSC first normalized the server to NetherNet, then switched to RakNet because
the existing direct, Playit, and Xbox Broadcast paths were UDP-oriented, then
restored NetherNet after BDS printed that RakNet was unsupported. Those early
transport conclusions were not reliable because the macOS host UDP relay was
itself broken.

Directly querying the VZ guest at `192.168.64.64:19000` returned a valid
1.26.51/protocol-2193 RakNet pong, while `127.0.0.1:19000` and
`10.0.0.142:19000` timed out. P15.65 repaired the reusable wildcard UDP bind,
per-client forwarding, idle cleanup, and real RakNet-ping readiness check.
Twelve stale detached sidecars from earlier agent runs were also identified
and stopped. MCXboxBroadcast build 155 was confirmed current, the configured
public target was correct, and the macOS application firewall was off.

Lesson: do not infer transport incompatibility while the host-to-guest relay
cannot pass a protocol-valid ping.

### P15.66-P15.68 — building the documented NetherNet shape

The 1.26.51 documentation and server output led MSC to implement NetherNet as:

- TCP on the configured server port for HTTP signaling;
- a bounded adjacent UDP range for WebRTC gameplay;
- a separate 16-port ICE range for Xbox Broadcast.

For server port `19001`, the initial layout was TCP `19001`, BDS UDP
`19002-19033`, and Xbox Broadcast UDP `19034-19049`. The TCP relay initially
accepted the client before its guest connection was ready and stranded the
first bytes. P15.68 made the relay wait for both endpoints before pumping.

Lesson: a listening socket is not proof that the first application bytes reach
the guest. Readiness must validate the complete relay path.

### P15.69-P15.75 — the macOS installed-service boundary

The same signed sidecar reached the VZ NAT guest when launched from Cameron's
foreground session, but received `EHOSTUNREACH` when descended from MSC's
installing-user system LaunchDaemon. Network.framework, POSIX sockets,
explicit `IP_BOUND_IF`, and entitlements did not change that result.

Cameron approved a narrow root-owned helper that owns only the VZ VM and its
host relays. The normal Rust agent remains the installing-user service and
talks to the helper through an authenticated local Unix-domain socket. The
work also corrected:

- a helper argument parser that skipped `--socket-path` incorrectly;
- a mismatch between `MSC2` and `MSC 2` application-support roots;
- SIGPIPE terminating the helper after a client disconnected;
- unsupported BusyBox `setpriv` syntax, replaced with the available `chpst`;
- relay startup and ownership/cleanup reporting.

After reinstalling, the live inspector passed 17/17: BDS stayed running, the
helper/agent identities and filesystem boundaries were correct, the host
relay reached BDS, and no orphan VM or sidecar remained.

Lesson: the VM, BDS, and relay code can all be healthy while the macOS service
execution context prevents host-to-guest routing. Keep this separate from
Minecraft transport diagnosis.

### P15.76-P15.78 — explicit NetherNet mappings

Community reports showed better NAT behavior when `server-udp-ports` contained
individual public-to-private mappings. MSC first generated 32 entries. Live
BDS output rejected them:

```text
server-udp-ports: too many port mappings (max 16)
```

MSC reduced the BDS range to exactly 16 mappings, `19002-19017`, while keeping
Xbox Broadcast on `19034-19049`. BDS then started without the mapping-limit
error. LAN manual connection could work, but cellular manual connection and
Xbox Broadcast transfer still ended in the NetherNet error.

Lesson: correct ICE mappings cannot repair a failure that occurs earlier in
the TLS signaling layer.

### P15.79 — controlled RakNet fallback

The upstream reports included users whose 1.26.51 Windows and Switch paths
still connected after removing `transport`, `server-udp-ports`, and
`server-ip`, despite BDS's NetherNet-only warning. MSC's earlier RakNet attempt
had never exposed UDP on the main port: the then-current helper bound TCP
`19001` and UDP `19002-19017`, while Xfinity forwarded TCP `19001`.

P15.79 made a real A/B test for the Intel-macOS sidecar only:

- write `transport=raknet`;
- remove `server-udp-ports` and `server-ip`;
- replace the TCP-plus-UDP-range relays with one UDP relay on `19001`;
- require a valid RakNet pong through that relay before reporting ready;
- replace Xfinity's TCP `19001` forward with UDP `19001`.

The direct LAN, direct cellular, and hotspot Xbox Broadcast joins then worked.
Native Linux and Windows remain on their existing NetherNet path; P15.79 did
not change them.

## Diagnoses that were ruled out

- **Missing TCP/UDP reachability:** the TCP handshake and client payload were
  captured arriving from cellular, and BDS answered them.
- **Wrong public IP:** the configured address matched the public address used
  successfully after the RakNet switch.
- **macOS application firewall:** it was disabled during diagnosis.
- **Outdated Xbox Broadcast JAR:** build 155 authenticated successfully,
  created its Xbox Live session, and started its protocol-2193 broadcaster.
- **World corruption or resource packs:** BDS loaded the existing world and
  reached `Server started`; the failure preceded player/world admission.
- **Server process dying because MSC refreshed:** CPU/RAM fell before refresh;
  refresh exposed the failed startup rather than causing it. The privileged
  helper work fixed the actual lifecycle teardown path.
- **More NetherNet UDP ports:** ICE never began in the failing capture, so
  opening a larger range could not affect the TLS alert.
- **Port 7551 from another user's PowerShell output:** that was an observed
  runtime socket on another machine, not a universal BDS port and was not
  copied into MSC.

## Retesting NetherNet after a Mojang update

Do not replace the working configuration merely because a new BDS build exists.
Retest it as a reversible experiment and record the exact server and client
builds.

1. Back up the working `server.properties` and record the current public and
   LAN addresses.
2. Confirm the new BDS release notes or Mojang issue tracker mentions the
   NetherNet TLS/signaling failure. Relevant reports during this incident were
   BDS-23108, BDS-23110, and BDS-23111.
3. Install the new BDS build without changing the world, router, relay, and
   transport simultaneously.
4. First probe locally:

   ```text
   curl --max-time 5 -sS -D - http://127.0.0.1:19001/v1/join -o -
   curl --max-time 5 -ksS -D - https://127.0.0.1:19001/v1/join -o -
   ```

   The HTTPS probe must complete a TLS handshake. If it still returns alert
   40, stop: router and UDP changes cannot fix that build.
5. If TLS works, restore the NetherNet configuration in a dedicated MSC step:
   TCP signaling on `19001`, no UDP listener on the same host port, exactly the
   BDS-supported number and syntax of explicit `server-udp-ports` mappings,
   and a non-overlapping Xbox Broadcast ICE range.
6. Change Xfinity from UDP `19001` back to TCP `19001`. Xfinity cannot retain
   both separate rules for this device, so treat this as the rollback point.
7. Start BDS and verify all boundaries in order:
   local `/v1/join`, LAN client, remote cellular client, observed UDP ICE
   traffic, Xbox Broadcast discovery/transfer, reconnect without restart, and
   simultaneous clients when available.
8. Capture the first missing boundary. Do not describe a generic connection
   error as a port-forwarding failure without packet evidence.
9. If any required path fails, restore RakNet and UDP `19001` from the backup.

## Useful commands

Inspect the installed agent/helper pair in the current RakNet build:

```text
python3 tools/phase15/inspect_macos_bedrock_helper.py \
  --live --server-port 19001 --connection-report
```

Observe the current working RakNet port:

```text
sudo tcpdump -ni en0 -vv 'udp port 19001'
```

Observe a future NetherNet experiment without assuming it works:

```text
sudo tcpdump -ni en0 -vv \
  'tcp port 19001 or udp portrange 19002-19017'
```

Check current transport properties:

```text
rg -n '^(server-port|transport|server-ip|server-udp-ports)=' \
  "$HOME/Library/Application Support/MSC 2/servers/bedrock/bedrock/server.properties"
```

## External references used during diagnosis

- Mojang NetherNet HTTP signaling guide:
  <https://mojang.github.io/bedrock-protocol-docs/guides/nether-net-onboarding-guide/>
- Community BDS 1.26.51 investigation and explicit-mapping results:
  <https://www.reddit.com/r/Minecraft/comments/1wivv9z/bedrock_dedicated_server_126511_nethernet/>
- Matching `InitialConnection-13` / TLS alert 40 report for BDS 1.26.51.1:
  <https://github.com/itzg/docker-minecraft-bedrock-server/issues/680>

Community reports were clues, not acceptance evidence. The conclusion for MSC
comes from Cameron's packet captures, localhost reproduction, installed-helper
inspection, and successful LAN/cellular/Xbox Broadcast joins.
