# MSC 2 error handling research notes

**Status:** living research notes — deliberately incomplete and expected to change.

**Last researched:** 2026-09-03

**Purpose:** collect the failure vocabulary, error formats, diagnostic signals, and likely repair paths MSC 2 will need for server startup and the services around a server. This is research, not an implementation plan or a final product decision.

The central product goal is simple:

> When a server does not start, or a server-related feature does not work, the user should get a plain-language explanation, the evidence behind it, a recommended next action, and a safe way to retry. The user should not have to read the console to understand what happened.

The console and raw files should remain available for detail and support, but they should be evidence behind the explanation rather than the explanation itself.

---

## 1. Initial conclusion

The proposed mental model is good, but it needs one additional axis.

The useful model is not only:

`error -> class -> specific error`

It is:

`operation -> failure phase -> error class -> specific finding -> evidence -> recommended action -> retry/verification`

For example, “Playit is offline” and “the Java server failed to bind port 25565” are both networking-related in a broad sense, but they require different evidence, different owners, and different actions. Similarly, “Geyser cannot accept Bedrock connections” does not mean the Java server failed to start.

MSC 2 should therefore distinguish at least these outcomes:

1. **Process could not be launched** — executable, permissions, working directory, Java runtime, or operating-system failure.
2. **Process launched but server boot failed** — loader, mod, plugin, configuration, world, port, license, or server-runtime failure.
3. **Process launched but never became ready** — startup timeout, hang, deadlock, resource starvation, or a server that emits no recognizable ready signal.
4. **Server became ready but a capability failed** — Geyser, Floodgate, voice chat, Playit, Xbox Broadcast, DuckDNS, Tailscale, or another helper.
5. **Server is healthy but a client cannot connect** — address, DNS, firewall, protocol, authentication, NAT, UDP, or client-version problem.
6. **MSC itself failed to observe or operate the process** — agent restart, lost child process, stale state, log read failure, API operation failure, or an interrupted repair.
7. **Unknown or contradictory evidence** — the system must say that the cause is not yet known, preserve evidence, and offer safe collection/retry actions rather than inventing a diagnosis.

This prevents a common UX failure: showing a generic “server failed” sheet for an error that is actually in a secondary service, or showing a networking repair action when the server never reached the networking stage.

### Console-only parsing is necessary but insufficient

Console parsing should be one diagnostic adapter among several:

- process spawn result and OS error;
- exit code, signal, and elapsed time;
- stdout and stderr, with timestamps and stream identity;
- server log files (`logs/latest.log`, crash reports, loader logs, Bedrock logs);
- server readiness and port-bind observations;
- server configuration and selected Java runtime;
- installed add-on manifests and provider metadata;
- helper process state and helper logs;
- local filesystem, permissions, free space, and network probes;
- external service response codes and bounded response bodies;
- the user’s requested operation and the server’s configured capabilities.

Researching common errors and encoding known signatures in MSC 2 is worthwhile. It gives users stable explanations and actions. However, a closed list of every possible console line will never be complete: mods, plugins, loaders, Java versions, operating systems, providers, and third-party services can all introduce new failures. The correct design is a **known-finding catalog plus a safe unknown-error path**, not “the app knows every error.”

---

## 2. What MSC 2 currently operates

This inventory is based on the current MSC 2 source and documentation. It is the scope that the error system needs to understand, even when a particular feature is still being built or is platform-limited.

### Server runtimes and Java flavors

Java server flavors currently represented in MSC 2 include:

- Vanilla;
- Paper;
- Purpur;
- Fabric;
- Forge;
- NeoForge;
- Quilt;
- Spigot;
- Pufferfish.

The create-flow catalog and the imported-server path are not identical. Some flavors may be imported or classified without having a complete MSC-managed provisioning path. Error handling must report the actual capability available for the selected server instead of implying that every flavor has the same installer, log format, or add-on folder.

Broadly:

- **Standard/plugin servers:** Vanilla, Paper, Purpur, Spigot, Pufferfish. The primary add-on folder is generally `plugins/` for the Paper-family servers. Vanilla has no add-on folder.
- **Modded/loader servers:** Fabric, Forge, NeoForge, Quilt. The primary add-on folder is `mods/`, and the loader’s dependency solver and classloader are major sources of startup failures.
- **Bedrock Dedicated Server:** a separate runtime, not a Java process and not a Java mod/plugin environment. MSC 2 supports native Windows/Linux operation where the official runtime is supported, with the macOS Bedrock sidecar/virtualization path documented elsewhere in MSC 2.

### Adjacent services and helpers

The server can also be surrounded by:

- **Geyser** — Java server plugin that exposes a Bedrock listener;
- **Floodgate** — commonly paired with Geyser to permit Bedrock/Xbox-authenticated players without a Java account;
- **Simple Voice Chat** — a separate UDP service associated with a Minecraft server/mod/plugin;
- **Playit.gg** — a tunnel/helper process for public Java, Bedrock, or voice-chat reachability;
- **MCXboxBroadcastStandalone** — helper that advertises a Geyser/Bedrock server through Xbox Live;
- **DuckDNS** — dynamic DNS updater;
- **Tailscale** — private tailnet connectivity and, depending on setup, subnet routing;
- **Modrinth and CurseForge** — add-on/modpack metadata and download providers;
- **GitHub releases and other direct providers** — helper and server artifact acquisition sources;
- **MSC agent, desktop/web client, iOS client, and CLI** — API and process-observation surfaces that can themselves fail.

An error sheet should identify which layer failed. “The server failed” is too broad when the actual issue is “the server is running, but Playit cannot establish its tunnel.”

---

## 3. Common error formats and signals

### A. Process and operating-system errors

These may occur before the server writes a useful Minecraft log. They are often more authoritative than console text.

Typical formats/signatures:

- executable not found / `No such file or directory`;
- permission denied / `Permission denied`;
- working directory missing or inaccessible;
- failed to spawn child process;
- invalid executable format / architecture mismatch;
- macOS quarantine, code-signing, or privacy permission failure;
- Windows access denied, missing DLL, or blocked executable;
- Linux `Exec format error` or missing shared library;
- process exits immediately with no output;
- process terminated by signal;
- helper path points at a directory or an HTML/error download instead of a binary/JAR.

Useful diagnosis:

- record the exact executable path and working directory;
- record the host OS and CPU architecture;
- check that the path exists and is a regular file;
- check executable permission where relevant;
- identify whether the downloaded bytes match the expected artifact type;
- capture the OS error code/message and exit status;
- do not overwrite the configured server artifact while diagnosing.

Recommended user-facing actions:

- “MSC could not launch the server process.”
- “The selected Java executable was not found / could not run.”
- “The server file is not executable on this computer.”
- “The server file may be incomplete or is not the expected file type.”
- action: select a detected compatible runtime, repair permissions where safe, re-download/verify the artifact, or open the containing folder;
- retry: only after the underlying spawn condition changes.

### B. Java runtime and JVM errors

Java failures have a useful split between “Java itself cannot start” and “Java started but rejected the server or an add-on.”

Known signatures:

- `java: command not found`;
- `Unable to access jarfile ...`;
- `Error: Could not find or load main class ...`;
- `UnsupportedClassVersionError` / `Unsupported major.minor version`;
- `Unrecognized option` or `Invalid maximum heap size`;
- `Could not reserve enough space for object heap`;
- `There is insufficient memory for the Java Runtime Environment to continue`;
- `OutOfMemoryError: Java heap space`;
- `OutOfMemoryError: Metaspace`;
- `StackOverflowError`;
- `NoClassDefFoundError` / `ClassNotFoundException`;
- `LinkageError`, `VerifyError`, `ClassFormatError`;
- native library loading errors;
- illegal reflective access or module/export errors;
- JVM crash with `hs_err_pid*.log`.

`UnsupportedClassVersionError` specifically means the JVM found a class file whose major/minor version it does not support. In product terms, the selected Java is too old for the server/mod/plugin, or a wrong Java binary was used. Paper’s troubleshooting documentation calls out this failure and recommends using the Java version supported by the server version. The fix should name both versions where possible: “This server requires Java 21; MSC launched Java 17.”

Java errors need configuration-aware recommendations:

- compare Minecraft/server flavor/version to the supported Java matrix;
- show the actual Java executable path and `java -version` output;
- distinguish “Java missing” from “Java present but wrong version”;
- distinguish heap reservation failure from runtime heap exhaustion;
- treat an OOM as possibly caused by too-large allocation, too-small allocation, host pressure, a mod/plugin leak, or a world-generation spike;
- never blindly recommend larger memory;
- if Java itself crashes, point to the JVM crash log and preserve it.

### C. EULA, configuration, and first-run gates

Common signatures:

- `You need to agree to the EULA in order to run the server. Go to eula.txt for more info.`;
- malformed `server.properties` or another YAML/JSON/TOML/config parse error;
- invalid enum/value, duplicate key, unknown option, or unsupported option;
- missing required configuration file or directory;
- server accepts configuration but cannot create its default world;
- server is already initialized but MSC incorrectly repeats the first-start path.

Actions:

- identify the exact file and key when the parser gives one;
- offer “Open settings” or “Review file” where safe;
- for EULA, show the EULA-specific agreement UI rather than a generic error;
- preserve a backup before any automatic configuration rewrite;
- after a successful configuration repair, return to the original operation (initiate or normal start) and show that it is being retried.

### D. Port binding and address errors

Known formats:

- `Failed to bind to port!`;
- `java.net.BindException: Address already in use: bind`;
- `java.net.BindException: Cannot assign requested address`;
- `Connection refused`;
- `Unknown host`;
- `getsockopt` / DNS resolution failures;
- server binds loopback only when the user expects LAN/public access;
- IPv4/IPv6 family mismatch;
- Bedrock UDP port is occupied or blocked while Java TCP is fine;
- voice UDP port collides with Geyser or another service.

The most common Paper recommendation is to leave `server-ip` empty unless there is a deliberate reason to bind a specific local interface. A port-bind finding should include:

- protocol (TCP/UDP);
- port;
- configured bind address;
- whether MSC found a local listener;
- process owning the port if the platform permits that lookup;
- whether this is the main Java listener, Bedrock/Geyser listener, voice listener, or helper control socket.

Actions must be conditional:

- “Stop the process already using TCP 25565”;
- “Clear the server bind address and retry”;
- “Choose a different unused port and update the matching service/configuration”;
- “Open the port in the host/router firewall” is a reachability action, not a server-start action;
- do not advise killing an unknown process without showing its identity and asking for confirmation.

### E. World and data-version errors

Common signatures:

- world saved by a newer Minecraft version;
- attempted downgrade;
- failed level/world conversion;
- missing or corrupt `level.dat`;
- unable to read/write region files;
- data pack or generator failure during world load;
- “This world was saved with a newer version” / “incompatible data version”;
- world lock or stale session lock;
- insufficient disk space during world creation or save.

Paper specifically warns that ignoring a newer-world-version warning can corrupt or damage world data. The recommended action should be conservative:

- back up before migration or repair;
- recommend upgrading the server to the world’s version before suggesting a downgrade;
- never make “ignore the warning” the primary action;
- identify the named world/slot;
- distinguish a recoverable startup block from a world that is merely unavailable while another slot can start.

### F. License, authentication, and account gates

These are different from file/configuration failures:

- EULA not accepted;
- Microsoft/Xbox authentication required or expired;
- Floodgate key mismatch;
- Xbox Broadcast device-code login incomplete/expired;
- Playit credentials missing, expired, or invalid;
- CurseForge API key missing, rejected, or not authorized;
- GitHub/provider request blocked or rate-limited.

The sheet should say who must act: “You need to sign in,” “MSC needs a CurseForge API key,” or “the server’s Floodgate key does not match the Geyser installation.” Avoid exposing tokens, passwords, cookies, or full authorization responses in UI or logs.

---

## 4. Mod and loader failure taxonomy

This deserves its own detailed catalog because the failure is often deterministic and repairable. The categories below should be separate findings, not one “mod incompatibility” bucket.

### 4.1 Required dependency missing

Typical Fabric format:

```text
Incompatible mods found!
Mod 'Chipped' (chipped) 3.0.7 requires version 3.1.1 or later of athena, which is missing!
Fix: add [add:athena 3.1.1 ([[3.1.1,∞)])]
```

Typical Forge/NeoForge family formats include “Mod `<name>` requires `<dependency>` `<range>` or above,” “Missing mandatory dependency,” or a dependency list in the loading screen/log.

Facts that matter:

- dependency ID is more reliable than display name;
- version range is not the same as one exact version;
- the missing dependency may be nested in another mod JAR or expected as a separate file;
- the dependency must match Minecraft version, loader, side, and sometimes modpack constraints;
- installing “the latest” without checking the target Minecraft/loader can create a second incompatibility.

Best action ranking:

1. find a compatible dependency release for this Minecraft version and loader;
2. if the provider metadata names an exact compatible file, offer install/download;
3. update the offender if a newer offender removes or changes the dependency;
4. disable the offender if the dependency is optional to the user’s goals;
5. delete only after explicit confirmation, with a backup/undo path.

### 4.2 Dependency present but wrong version

Typical forms:

- “requires version X or later”;
- “requires version in range [a,b), found version c”;
- “mod X requires Y, but Y is incompatible”;
- multiple mods require mutually exclusive versions.

The repair engine should build a constraint set rather than act on the first line. If two installed mods require incompatible ranges, there may be no one-file update that solves the set. The sheet should say that clearly and list the conflicting constraints.

### 4.3 Explicit conflict

Loader metadata can express incompatibility/conflict relationships. Fabric metadata distinguishes hard `breaks` from soft `conflicts` behavior; NeoForge metadata has an `incompatible` dependency type. The UI should distinguish:

- hard conflict: the loader refuses to load;
- soft conflict/warning: the loader may continue, but behavior is unsupported;
- detected runtime conflict: the metadata did not declare it, but the stack trace/mixin failure implicates a pair.

Recommended actions should prefer updating both members, then disabling one member. Do not automatically delete either side based only on a warning.

### 4.4 Minecraft version mismatch

Examples:

- mod built for 1.20.1 loaded on 1.20.4;
- modpack manifest says 1.19.2 while the server is 1.20.1;
- loader version is correct but game version is not;
- a dependency exists only for a different game version.

This is often the root cause behind many downstream errors. Show the server’s Minecraft version, the add-on’s declared versions, and whether a compatible provider version exists. Updating the server is a major operation and should not be silently selected as a mod repair.

### 4.5 Loader mismatch

Examples:

- Fabric mod placed in Forge/NeoForge `mods/`;
- Forge mod placed in Fabric `mods/`;
- Quilt is loading a Fabric-compatible mod but a Forge-only mod is present;
- a plugin JAR was placed in `mods/`, or a mod JAR was placed in `plugins/`;
- a client-only mod is present on a dedicated server.

The manifest is the first source of truth where available. File names are only a fallback. Explain the required loader and the selected loader, then offer move/disable only when MSC can prove the file’s type and destination.

### 4.6 Side/environment mismatch

Fabric metadata has an environment (`client`, `server`, or `*`); Forge/NeoForge metadata has side declarations. A client-only mod can crash a dedicated server by referencing client classes that are absent on the server.

Recommended action: remove/disable the client-only add-on from the server, or install the server-compatible variant. Never recommend adding client classes to a dedicated server.

### 4.7 Broken, partial, or wrong-file JAR

Possible evidence:

- ZIP/JAR cannot be opened;
- manifest missing or malformed;
- downloaded content is an HTML login/blocked page;
- checksum/size does not match provider metadata;
- file is truncated or zero bytes;
- duplicate JAR versions are installed;
- duplicate mod ID appears in multiple files;
- nested dependency extraction failed.

Action: quarantine the file, re-download from a known provider, validate checksum where available, and preserve the original path/name for the repair record. Do not delete the only copy automatically.

### 4.8 Mixin, transformer, and classloading failure

Common signatures:

- `Mixin apply failed`;
- `InjectionError`;
- `InvalidInjectionException`;
- `NoSuchMethodError`;
- `NoSuchFieldError`;
- `ClassNotFoundException`;
- `NoClassDefFoundError`;
- `VerifyError`;
- `MixinTransformerError`;
- “failed to load class”;
- crash report points at a transformed class but the first named mod is not necessarily the only cause.

These are often caused by a stale mod, wrong Minecraft mapping, wrong loader version, duplicate libraries, or an interaction between two mods. The first stack-trace frame is evidence, not proof. The diagnosis should show:

- named mod(s);
- failing target class/method/field;
- loader and game version;
- whether the implicated JAR declares a compatible version;
- whether the failure repeats with one add-on disabled (only if MSC provides a safe, reversible test).

Safe actions: update the implicated mod and its dependency, check the exact game/loader version, disable one implicated add-on, or open the crash report. Avoid claiming certainty when the stack trace only gives an indirect clue.

### 4.9 Runtime mod crash after loader acceptance

The loader may successfully resolve dependencies, then a mod can crash during initialization, registry setup, world load, or tick startup. Examples include:

- `ExceptionInInitializerError`;
- `NullPointerException` in a mod initializer;
- registry duplicate;
- failed data generation;
- invalid resource/data pack;
- server-only code path bug;
- memory exhaustion during world generation.

This is a different class from dependency resolution. The likely action is update, roll back the last changed add-on, disable the implicated add-on, or restore a backup. Automatic “install missing dependency” is not appropriate.

### 4.10 Warnings that should not be promoted to blockers

Loader logs can contain warnings about version-string formatting, metadata fields, credits, deprecated APIs, or optional dependencies. MSC should record them as warnings, but should not say “the server did not start because of this warning” unless the process actually failed and evidence connects the warning to the failure.

### 4.11 Modpack-level failures

Modpack import adds another layer:

- invalid or unsupported manifest;
- manifest loader/game version not supported by selected server;
- required file not downloadable;
- provider authorization/rate limit;
- author-blocked third-party download;
- optional/required override missing;
- hash mismatch;
- path traversal or invalid destination path;
- two files map to the same destination;
- dependency manifest and downloaded contents disagree;
- pack imports successfully but fails at first start.

The repair sheet should retain the distinction between **pack acquisition failure** and **server boot failure**. If the pack was not fully staged, retrying the server is not useful. If the pack staged and the server booted into a missing dependency, offer the dependency repair and then retry the original start.

---

## 5. Server-family notes

### Vanilla

Vanilla has fewer add-on dependency problems, but it still has:

- Java/runtime mismatch;
- EULA gate;
- port bind;
- invalid properties;
- missing/corrupt JAR;
- world data version and corruption;
- insufficient memory/disk;
- process crash or startup timeout.

The absence of a mod/plugin folder does not mean a generic error sheet is sufficient. Vanilla’s world/config/license path is still distinct.

### Paper, Purpur, Spigot, and Pufferfish

The Paper-family server commonly fails because of:

- wrong Java version;
- wrong/missing server JAR or incorrect startup path;
- failed port bind;
- plugin missing a dependency;
- plugin JAR is actually an HTML/error download;
- plugin has not enabled successfully;
- plugin is incompatible with the Paper/Minecraft version;
- circular plugin loading;
- world is newer than the server;
- watchdog or unexpected shutdown;
- server starts but a plugin-provided feature is unavailable.

Paper’s official troubleshooting advice is especially useful for the diagnosis workflow: inspect `logs/latest.log`, look for the first meaningful error rather than only the final cascade, and binary-search plugins when one is suspected. MSC can make that workflow safer by proposing a reversible disable test and keeping a repair journal.

Paper’s plugin metadata can expose a dependency name, but not every plugin is well behaved or fully declared. The error catalog needs an “undeclared runtime incompatibility” path.

### Fabric

Fabric supplies unusually structured evidence:

- `fabric.mod.json` declares ID, version, dependencies, environment, breaks/conflicts, and entrypoints;
- required dependency failures are normally reported as an “Incompatible mods found!” loading error;
- crash reports include system details and a Fabric Mods list;
- nested JARs can sometimes provide dependencies.

MSC should parse the structured metadata before parsing prose. It should preserve the solver’s dependency range and turn it into a finding with a machine-readable offender and missing/installed dependency.

### Quilt

Quilt’s “Incompatible mod set” path is conceptually aligned with the desired MSC sheet: missing dependencies and incompatible mods should be named, and troubleshooting guidance recommends removing the named mod to reproduce the problem. Quilt’s ecosystem also shows the benefit of making dependency errors human-readable and, in the future, assisting with update/disable decisions.

Quilt can load many Fabric mods, but compatibility should not be assumed for every mod. The sheet must still identify the selected loader and the mod’s declared loader/environment.

### Forge and NeoForge

Forge and NeoForge use TOML metadata (`mods.toml` / `neoforge.mods.toml`) with dependency concepts that include:

- required/mandatory dependency;
- optional dependency;
- incompatible dependency;
- version range;
- ordering (`before`/`after`);
- side (`CLIENT`, `SERVER`, `BOTH`).

NeoForge’s documentation states that required dependencies prevent loading, optional dependencies do not, incompatible dependencies prevent loading when present, and discouraged relationships warn. This provides a good normalized model for MSC even though the surface text differs by loader version.

Forge/NeoForge-specific failures also include:

- installer subprocess failure or timeout;
- wrong installer Java;
- installer produced an incomplete server directory;
- duplicate packages/classes;
- client-only package access on a dedicated server;
- mixin/transformer failures;
- missing libraries or failed dependency extraction;
- mod initialization/world-load exceptions.

The app should not confuse an installer/provisioning failure with a later startup failure. Preserve each operation’s phase and artifact state.

### Bedrock Dedicated Server

Bedrock is a separate diagnostic adapter. The official distribution currently documents Windows and Ubuntu Linux requirements and supplies a native `bedrock_server` executable. Bedrock errors therefore should not be passed through Java/mod/plugin classification.

Common Bedrock startup failures:

- executable missing/not executable or incompatible OS/architecture;
- required runtime libraries absent;
- EULA/license not accepted where applicable;
- invalid `server.properties` or allow-list configuration;
- port already in use;
- firewall or router port mismatch;
- world creation/load failure;
- insufficient disk, memory, or permissions;
- client/server protocol version mismatch;
- process exits without a recognized ready marker.

Bedrock reachability also differs from Java: server port and IPv6 port may be separate, and Bedrock player access commonly depends on UDP. A Java TCP success must not be used as proof that Bedrock is reachable.

On macOS, the sidecar/virtualization boundary adds failures such as missing sidecar, virtualization unavailable, image boot failure, guest filesystem failure, and host-to-guest port forwarding failure. Those belong to a `bedrock_runtime` / `virtual_machine` class, not to Bedrock world or mod errors.

---

## 6. Integration and service failures

### Geyser

Geyser failures are commonly one of three types:

1. **Plugin installation/load failure** — wrong version, missing dependency, wrong server flavor, or failed plugin initialization.
2. **Bedrock listener failure** — UDP port bind, wrong bind address, firewall, proxy/tunnel mismatch.
3. **Player connection/authentication failure** — outdated client/server, invalid IP/SRV, Floodgate configuration, MTU/UDP, or authentication.

Important signatures from Geyser troubleshooting include:

- `java.net.BindException: Address already in use: bind`;
- Java class version mismatch;
- `Connection refused`;
- invalid IP/SRV record;
- Floodgate key/authentication errors;
- `Unable to connect to world` on Bedrock;
- UDP/MTU symptoms.

MSC should ask or infer which path is failing. If Java users can connect but Bedrock users cannot, the finding should be `geyser_reachability`, not `server_start_failed`.

### Floodgate

Floodgate-specific evidence includes:

- key mismatch between Geyser and Floodgate;
- AEAD/decryption errors such as `AEADBadTagException`;
- missing profile/access token;
- IP forwarding/configuration mismatch;
- Xbox account/authentication failure.

Actions should be ordered from least destructive:

- verify Geyser and Floodgate versions are compatible;
- verify both point at the same key and installation;
- regenerate or re-copy keys only with a clear warning about existing player identity data;
- re-authenticate Xbox/Microsoft account if required;
- retry Bedrock connection test.

Never display the key itself.

### Simple Voice Chat

Simple Voice Chat is usually a separate UDP path. The server can be healthy while voice chat is unavailable.

Known issues:

- UDP port not forwarded/open;
- port used by Geyser or another service;
- wrong bind address;
- Docker/container forgot UDP mapping;
- proxy/tunnel provider does not support UDP;
- wrong loader/API/plugin JAR;
- corrupted JAR or mixin version problem;
- client does not have the required mod or compatible version;
- microphone permissions or client audio device issue;
- configuration moved between plugin/mod versions.

The official guide recommends the voice-chat test command and warns that generic port tools do not test UDP correctly. MSC should expose a voice-specific test and report:

- whether the voice component loaded;
- configured UDP port/bind address;
- whether the port is locally bound;
- whether the selected Playit/tunnel path supports voice UDP;
- whether the client has connected to the voice handshake.

Recommended wording: “Minecraft is running. Voice chat cannot reach its UDP service on port 24454.” This is much clearer than restarting the entire Minecraft server.

### Playit.gg

Playit has at least two failure boundaries:

- MSC cannot install/start/authenticate the local `playitd` helper;
- the helper is running but the cloud tunnel is offline, not attached, or cannot reach the local target.

Observed community/support signatures include attach/control timeouts, “failed to connect to tunnel,” dashboard agent offline, connection refused, and server-local success with external-player failure. Reddit reports also show CGNAT, ISP/VPN-specific behavior, wrong public address, and Bedrock UDP tunnel problems where Java works but Bedrock disconnects.

MSC should not claim that the Minecraft server failed when Playit is the failed component. The status model needs:

- helper installed/running;
- account authenticated;
- agent registered/online;
- tunnel configured;
- target host/port/protocol;
- last control connection;
- last data-plane connection/test;
- Java TCP, Bedrock UDP, and voice UDP status separately.

Actions:

- start/restart the helper;
- re-authenticate or re-run setup;
- validate local target port and protocol;
- re-create or repair a tunnel only with explicit user confirmation;
- use the Playit dashboard/support path when cloud-side evidence is unavailable.

Do not silently delete credentials or cloud tunnels. MSC’s reset action should be clearly described as clearing local credentials/derived state and should not imply that it removes cloud resources.

### MCXboxBroadcastStandalone

The official project README describes this as a tool that broadcasts an existing Geyser/Bedrock server over Xbox Live. It is not the Minecraft server itself. Its failure states include:

- JAR missing or bad download;
- Java runtime failure;
- Microsoft device-code login incomplete/expired;
- account not followed/authorized as required by the project;
- target IP/port wrong in `config.yml`;
- Geyser/Bedrock target not running or unreachable;
- Xbox Live/API/session creation failure;
- helper starts but the listing does not appear yet;
- helper process timeout or unexpected exit.

The project explicitly warns that it emulates client features and should be used at the operator’s risk. MSC should show that external-risk note in setup/help, keep credentials out of diagnostics, and make broadcast status independent from Minecraft start status.

Useful success evidence from MSC’s current tests is a line like `Creation of Xbox LIVE session was successful`; absence of that line is not, by itself, proof of failure until the helper’s bounded startup timeout expires.

### DuckDNS

DuckDNS has a small, explicit HTTP API:

- required: `domains`, `token`;
- optional: IPv4 `ip`, IPv6 `ipv6`, `verbose`, `clear`;
- normal success: `OK`;
- normal failure: `KO`;
- verbose output can report current IP and `UPDATED`/`NOCHANGE`.

Common failures:

- wrong token or domain;
- updater not running on a schedule;
- `curl`/network/TLS/DNS failure;
- updater detects the wrong external IP, especially with IPv6;
- DNS propagation/cache makes a correct update appear stale;
- domain resolves, but the router/firewall/server port is not reachable;
- user passes a full hostname where the endpoint expects a subdomain value, depending on form.

MSC should separate “DuckDNS record update failed” from “the hostname resolves but the server cannot be reached.” Redact the token from every log and error payload. A diagnostic can safely show hostname, detected IP family, response category, and last successful update time.

### Tailscale

Tailscale is a private network path, not a public tunnel. Common failures include:

- Tailscale not installed/running/authenticated;
- machine not in the expected tailnet;
- ACL/policy denies the connection;
- subnet route not advertised or not approved;
- IP forwarding/firewall prevents subnet routing;
- client is using the wrong Tailscale IP/hostname;
- DNS/MagicDNS resolution failure;
- UDP blocked, forcing DERP relay or causing poor/unavailable connectivity;
- no DERP home relay, commonly due to firewall/internet/service availability;
- server binds only to an address unavailable over the tailnet;
- host firewall allows LAN/public traffic but not `tailscale0`.

Tailscale’s own `netcheck` output provides useful structured signals: UDP availability, IPv4/IPv6, NAT mapping behavior, port mapping, nearest DERP, and relay latency. MSC should use a bounded equivalent where available, not merely ping the hostname.

The product should explain the scope: “This server is reachable only to tailnet members” versus “this is a public address.” A Tailscale failure should never lead the user to open the Minecraft port publicly as the first action.

---

## 7. Provider, download, and installation failures

A repair action can be correct but still fail before it changes the server. Provider errors need their own class.

### Modrinth

Modrinth metadata exposes stable project/version/file IDs, Minecraft versions, loaders, dependencies, hashes, and file metadata. This is valuable for choosing a compatible repair candidate. MSC should prefer IDs and hashes over display names/slugs when available.

Known provider/API failures:

- no compatible version for the server’s Minecraft/loader;
- project or version deleted/unlisted;
- API rate limit;
- API version deprecated or gone;
- request blocked by network/TLS/DNS;
- incomplete/invalid JSON response;
- download URL returns an error page;
- checksum mismatch;
- file path collision or unsafe path;
- provider metadata says dependency exists, but the dependency file is not downloadable.

Modrinth documents rate-limit headers and per-file SHA-1/SHA-512 values. MSC should preserve response category and retry-after information where safe, but should not loop indefinitely.

### CurseForge

CurseForge adds authentication and author-controlled download restrictions. The API may require an API key, and direct file downloads can return `401 Unauthorized` without the expected `x-api-key`. Some authors restrict third-party downloads, so a modpack can be valid while MSC is unable to fetch one file automatically.

Distinct outcomes:

- API key not configured;
- API key invalid/rejected;
- API access/rate limit;
- author-blocked third-party file;
- file not found or no longer available;
- response is not the expected archive/JAR;
- manual download is required;
- archive passes acquisition but fails manifest validation.

The repair sheet should offer a browser/manual-download handoff for author-blocked files, then a clearly bounded “continue staged import” action. It should not classify an author block as a mod incompatibility.

### GitHub and direct release downloads

MSC uses release metadata for at least Playit and MCXboxBroadcast. Common failures:

- API rate limit or unauthenticated request limit;
- release missing or changed asset name;
- asset URL redirects to an HTML page;
- checksum/digest absent or mismatched;
- downloaded file is not the expected JAR/binary;
- GitHub unavailable or network failure;
- release is for a different platform/architecture.

The infrastructure layer should validate artifact identity before promotion. A downloaded helper should be staged, checked, and atomically moved into its managed location only after validation.

### Forge/NeoForge installer subprocess

Installer-specific states:

- installer JAR download failure;
- wrong Java for installer;
- installer exits non-zero;
- installer hangs or exceeds timeout;
- post-install argument/config file missing;
- partial directory left after failure;
- rerunning installer into an existing directory changes state unexpectedly;
- cancellation leaves temporary files or child processes.

The user-facing operation should say whether failure occurred while downloading, running the installer, or starting the newly provisioned server. Cleanup/rollback should be explicit and safe.

---

## 8. A robust diagnostic pipeline

This is a proposed shape for later design discussion.

### Phase 1: Capture the operation

Every start-like action should create or reuse an operation record:

- server ID;
- requested action: initiate, start, restart, repair-and-retry, helper start, etc.;
- server type/flavor/version/loader;
- selected Java path/version if Java;
- enabled capabilities and helpers;
- start timestamp and deadline;
- whether a repair is already in progress;
- prior attempt ID if this is a retry.

This is how the sheet can later say “Repair completed; retrying the original initiate” instead of losing context.

### Phase 2: Preflight without mutating

Check:

- executable/artifact exists and is readable;
- expected server type/flavor;
- Java exists and is compatible;
- working directory exists;
- required directories are present;
- disk space and permissions are plausible;
- configured ports are valid and not already owned;
- required helper artifacts are present;
- mod/plugin manifests can be read;
- modpack staging is complete;
- EULA/config gate status.

Preflight should catch obvious failures before launching, but it must not pretend it can prove that every mod will run correctly. Runtime evidence still matters.

### Phase 3: Supervise launch

Record:

- spawn success/failure;
- PID/process identity;
- stdout/stderr separately;
- log file path and last-read offset;
- exit code/signal;
- readiness marker(s);
- first bind attempt and bound ports;
- startup deadline and reason for timeout.

The process supervisor must remain authoritative if the app/client restarts. The agent should be able to recover the operation state and inspect the child or its persisted result.

### Phase 4: Analyze evidence

Use analyzers in this order:

1. direct process/OS failure;
2. structured server/loader crash report;
3. known startup signatures in stderr/log;
4. manifest/dependency graph;
5. port and filesystem probes;
6. helper/service state;
7. generic exit/timeout inference;
8. unknown.

Several findings can be emitted. One should be designated the primary blocker, but secondary warnings and related findings should remain visible. For example, a missing Fabric dependency may be primary while metadata-format warnings are secondary.

### Phase 5: Present a user-facing finding

The sheet/card should contain:

- what failed in plain language;
- whether the Minecraft server is running, stopped, or unknown;
- the affected component;
- exact named items (mod, plugin, dependency, file, port, helper);
- why MSC believes this is the cause;
- recommended action first;
- alternative actions with tradeoffs;
- whether MSC can perform the action automatically;
- backup/undo warning if relevant;
- “View console/log” for raw evidence;
- “Retry initiate” or “Retry start” after repair;
- a way to leave the issue unresolved without losing the report.

Avoid presenting five equally weighted buttons. The system should rank actions and explain why the first is recommended.

### Phase 6: Verify the repair and retry

An action is not complete because a file downloaded. Verify:

- dependency graph is now satisfiable;
- file checksum/format is valid;
- helper artifact starts;
- port is available/bound;
- server reaches ready state;
- capability-specific probe succeeds;
- the original operation is retried only once automatically unless the user explicitly asks for another attempt.

If the retry fails with a new finding, update the same incident chain rather than opening a fresh generic error. Show “The first issue was fixed; startup then exposed a second issue.”

---

## 9. Proposed normalized finding shape

This is not yet a contract. It is a vocabulary sketch to test against the existing MSC2 `StartupProblem`, health, operation, and error DTOs.

```text
Incident
  incident_id
  operation_id
  server_id
  operation_kind: initiate | start | restart | helper_start | repair | unknown
  lifecycle_state: preflight | launching | booting | ready | helper_setup | reachable_check
  outcome: blocked | degraded | unknown | resolved
  primary_finding_id
  started_at / detected_at / resolved_at
  attempts[]

Finding
  finding_id
  component: process | java | server | loader | mod | plugin | world | network |
             bedrock | geyser | floodgate | voice_chat | playit | broadcast |
             duckdns | tailscale | provider | msc
  class: missing_dependency | incompatible_version | port_bind | java_runtime |
         provider_auth | startup_timeout | helper_offline | ...
  code: stable_machine_code
  severity: blocker | error | warning | info
  confidence: confirmed | strong | probable | possible | unknown
  title
  summary
  details
  subjects[]: named mod/plugin/file/port/process/service
  evidence[]
  recommended_action_id
  alternatives[]
  can_retry
  retry_kind

Evidence
  source: process | stdout | stderr | log | crash_report | manifest | probe | provider
  path_or_endpoint (redacted where necessary)
  line_or_range (if available)
  excerpt (bounded and redacted)
  observed_at

Action
  action_id
  kind: install | update | disable | quarantine | delete | change_config |
        choose_java | free_port | authenticate | start_helper | retry | collect_logs
  target
  rationale
  risk: none | reversible | backup_required | destructive | external_effect
  automatic: yes | no | requires_confirmation
  preconditions[]
  postconditions[]
```

Important design properties:

- stable `code` for clients and analytics-free support tooling;
- human text generated from structured fields, not only a raw parser string;
- evidence and confidence visible to support/debug views;
- actions describe preconditions and postconditions;
- repair is idempotent where possible;
- destructive actions are never the default;
- secrets and full external responses are redacted;
- one incident can contain multiple attempts and findings.

Possible stable-code families:

```text
process.spawn_failed
process.executable_missing
java.not_installed
java.incompatible_runtime
java.heap_unavailable
server.eula_required
server.config_invalid
server.port_in_use
server.startup_timeout
server.crashed_unknown
server.world_version_newer
loader.missing_dependency
loader.incompatible_dependency
loader.loader_mismatch
loader.side_mismatch
addon.corrupt_archive
addon.runtime_crash
provider.rate_limited
provider.authentication_required
provider.file_blocked
bedrock.runtime_unavailable
geyser.listener_failed
floodgate.key_mismatch
voice_chat.udp_unreachable
playit.agent_offline
broadcast.authentication_required
duckdns.update_failed
tailscale.route_unavailable
msc.observation_lost
msc.unknown_failure
```

The exact code list should be decided only after comparing it to the current API contract and existing `StartupProblem` serialization. Avoid creating duplicate concepts under different spellings.

---

## 10. Error sheet behavior

The sheet should be a diagnosis and recovery surface, not just a modal console viewer.

Suggested layout/content:

### Header

“`testpak` could not start” or “`testpak` started, but Bedrock access is unavailable.”

The verb should match the lifecycle state. This is the first important distinction.

### Primary explanation

One sentence naming the blocker:

“Chipped 3.0.7 requires Athena 3.1.1 or later, but no compatible Athena mod is installed.”

### Affected items

Show structured rows for offender, missing dependency, installed version, required range, and target server version. Avoid a giant raw stack trace at the top.

### Recommended fix

Show one primary action, for example:

“Install a compatible Athena version for Fabric 1.20.1.”

Then state what MSC will do: download, validate, place in `mods/`, re-scan, and retry initiate. If it cannot safely do that, explain why and provide the manual path.

### Alternatives

- update Chipped;
- disable Chipped temporarily;
- remove Chipped (destructive; backup/confirmation);
- view the dependency in the provider catalog;
- open the console/log.

Do not show “delete” beside “install” as equal first-class choices without risk labeling.

### Retry

The retry button should preserve the originating operation:

- “Retry initiate” if initiate failed;
- “Retry start” if a regular start failed;
- “Retry Geyser” if only Geyser failed;
- “Retry voice test” for a voice-only issue.

For a repair-and-retry flow, show progress and the result of both phases. The retry must be safe if the user closes/reopens the sheet or if the client restarts.

### Details

Expandable sections:

- “Why MSC thinks this is the cause”;
- “What MSC checked”;
- “Console and logs”;
- “Copy diagnostic summary.”

The diagnostic summary should be shareable and redacted by default.

---

## 11. Testpak case study

Local MSC 2 testing produced a concrete example of the desired behavior.

The server is a Fabric 1.20.1 modpack with 102 JARs. Startup produced:

```text
Incompatible mods found!
Mod 'Chipped' (chipped) 3.0.7 requires version 3.1.1 or later of athena, which is missing!
Fix: add [add:athena 3.1.1 ([[3.1.1,∞)])]
```

MSC 2’s persisted startup result already captured the useful structured facts:

- `wasClean: false`;
- fatal startup error;
- problem code equivalent to `missingDependency`;
- offender: Chipped;
- missing dependency: Athena;
- installed-file context.

This is an excellent baseline because it proves the raw console line can be upgraded into a repairable finding. The remaining product behavior to discuss later is:

1. the initiate sheet should fetch/display the structured health problem immediately after failure;
2. the sheet should explain that the server did not start because Athena is missing;
3. it should resolve a compatible Athena file against Minecraft 1.20.1 + Fabric and the required version range;
4. it should download and validate the file, or explain why manual download is required;
5. it should re-scan the pack before starting;
6. it should retry the original initiate flow and report whether the retry passed or exposed another issue;
7. if the client/app restarts during console viewing, the agent’s persisted incident/operation record should make the same sheet recoverable.

Warnings seen in the same kind of modpack log, such as metadata/version-format warnings, should remain secondary unless startup actually fails because of them.

---

## 12. What to research or decide next

These are open questions for later product/architecture work, not decisions made by this document.

### Known catalog versus parser rules

Recommended direction: keep a small, versioned catalog of stable failure codes and action policies, while allowing parser adapters and provider metadata to add evidence. Do not hard-code every exact prose message from every loader version.

### Automatic action confidence

Recommended direction:

- automatic: non-destructive downloads into a staging area, metadata scan, retry;
- confirmation: disable/rename an add-on, change a port, change Java, regenerate keys;
- explicit destructive confirmation: delete a mod/plugin/world/config;
- manual-only: account login, author-blocked download, ambiguous runtime crash, external router/firewall changes.

### Safe disable versus delete

Recommended direction: disable should be a reversible rename/quarantine operation with a reason and timestamp. Delete should be a separate, less prominent action and should require a backup or explicit confirmation.

### “Best action” selection

Recommended ranking inputs:

- hard loader constraints;
- provider compatibility metadata;
- exact Minecraft/loader/server version;
- whether the file is managed by a modpack;
- whether the action is reversible;
- whether the user changed this item recently;
- whether the problem is confirmed or only inferred;
- whether a compatible artifact can be verified.

“Latest” should not outrank “compatible and verified.”

### Timeouts and readiness

Recommended direction: retain lifecycle states and make the timeout explain what was observed. “No ready marker within 120 seconds; process still running” is different from “process exited after 2 seconds.” Offer “continue waiting,” “stop,” and “inspect logs” where appropriate.

### Unknown failures

Recommended direction: unknown must still be useful. It should show:

- process state;
- exit code/signal;
- last meaningful log lines;
- files collected;
- checks already performed;
- safe next actions: retry, open logs, collect diagnostic bundle, check Java, check ports, or contact support/community.

Never manufacture a mod culprit from the last line of a stack trace.

---

## 13. Source notes and reading list

The sources below are intentionally mixed: official documentation for formats/contracts, and community reports for the failures users actually encounter. Community reports are useful for discovering cases and wording, but should not be treated as authoritative product contracts without verification.

### Fabric

- [Fabric mod metadata (`fabric.mod.json`)](https://docs.fabricmc.net/develop/loader/fabric-mod-json) — IDs, versions, dependencies, `depends`, `recommends`, `breaks`, `conflicts`, environment, version ranges, entrypoints.
- [Fabric Loader overview](https://docs.fabricmc.net/develop/loader/) — dependency resolution, launch behavior, nested JARs, `mods` directory.
- [Fabric crash reports](https://docs.fabricmc.net/players/troubleshooting/crash-reports) — crash-report location and useful summary/system/mod details.
- [Fabric dependency overrides](https://docs.fabricmc.net/players/troubleshooting/dependency-overrides) — powerful but risky temporary override; not a normal repair recommendation.
- [Fabric community: version/Java and missing API examples](https://www.reddit.com/r/fabricmc/comments/wkgqvt/fabric_mods_error/), [missing Fabric API](https://www.reddit.com/r/fabricmc/comments/1242voe/i_cannot_figure_out_how_to_fix_this_error/), [loader/version conflict example](https://www.reddit.com/r/fabricmc/comments/zlb8c7/help_incompatible_mods/).

### Quilt

- [Quilt server installation](https://quiltmc.org/en/install/server/) — server launcher and mod placement.
- [Quilt troubleshooting](https://quiltmc.org/en/usage/troubleshooting/) — “Incompatible mod set,” dependency and compatibility guidance.
- [Quilt update: human-readable dependency errors](https://quiltmc.org/en/blog/2024-04-08-quilt-update/) — error presentation and future repair direction.
- [Quilt update: Cozy/error analysis](https://quiltmc.org/en/blog/2023-09-28-quilt-update/) — parsing, solver errors, sidedness, overrides.
- [Quilt non-obfuscated updates](https://quiltmc.org/en/blog/2026-02-03-non-obfuscated-updates/) — update/crash assistance goals and limits of automation.

### Forge and NeoForge

- [Forge mod files](https://docs.minecraftforge.net/en/latest/gettingstarted/modfiles/) — `mods.toml`, dependencies, sides, mandatory/version range/order.
- [Forge dependency management](https://docs.minecraftforge.net/en/1.13.x/gettingstarted/dependencymanagement/) — embedded/extracted dependency concepts.
- [NeoForge mod files](https://docs.neoforged.net/docs/1.21.5/gettingstarted/modfiles/) — required/optional/incompatible/discouraged dependency semantics and sides.
- [NeoForge project structure](https://docs.neoforged.net/docs/1.21.1/gettingstarted/structuring/) — duplicate packages and client-only code on dedicated servers.
- [NeoForge dependencies](https://docs.neoforged.net/toolchain/docs/dependencies/) — dependency and repository concepts.
- [Forge support forum: missing crash report/log diagnosis](https://forums.minecraftforge.net/topic/122507/).
- [Forge support forum: Java runtime configuration](https://forums.minecraftforge.net/topic/110678/).
- [Forge support forum: client-only/missing dependency examples](https://forums.minecraftforge.net/profile/201837-warjort/content/page/113/).

### Paper and plugin servers

- [Paper getting started](https://docs.papermc.io/paper/getting-started/) — Java requirements and launch command.
- [Paper basic troubleshooting](https://docs.papermc.io/paper/basic-troubleshooting/) — `latest.log`, bind failures, Java, worlds, plugins, watchdog, binary search.
- [Paper Java installation](https://docs.papermc.io/misc/java-install/) — supported JDKs and verifying `java -version`.
- [Paper adding plugins](https://docs.papermc.io/paper/adding-plugins/) — plugin folder, JAR validity, dependencies, logs, `UnknownDependencyException`.
- [Paper FAQ](https://docs.papermc.io/paper/faq/) — supported Java and early-access/internal runtime warning.
- [Paper system properties](https://docs.papermc.io/paper/reference/system-properties/) — JVM/Paper flags can change behavior and should be handled carefully.
- [Community failed-bind example](https://www.reddit.com/r/admincraft/comments/1jfsxiw/).
- [Community Java/version example](https://www.reddit.com/r/admincraft/comments/1ev1p/).
- [Community world downgrade example](https://www.reddit.com/r/admincraft/comments/1ajoqkv/).
- [Paper forum failed-bind thread](https://forums.papermc.io/threads/failed-to-bind-port.261/).
- [Paper forum Geyser plugin initialization thread](https://forums.papermc.io/threads/geyser-plugin-not-running.1648/).

### Java runtime

- [Oracle `UnsupportedClassVersionError`](https://docs.oracle.com/en/java/javase/26/docs/api/java.base/java/lang/UnsupportedClassVersionError.html) — exact meaning of class-file/runtime mismatch.
- [Java Virtual Machine Specification](https://docs.oracle.com/en/java/javase/26/docs/specs/jvms26.pdf) — class-file version loading rules.

### Bedrock and cross-play

- [Minecraft Bedrock Dedicated Server download/requirements](https://www.minecraft.net/en-us/download/server/bedrock) — supported Windows/Ubuntu platforms and requirements.
- [Microsoft Bedrock Dedicated Server getting started](https://learn.microsoft.com/en-us/minecraft/creator/documents/bedrockserver/getting-started?view=minecraft-bedrock-stable) — start commands, properties, ports, firewall, version/allow-list notes.
- [Geyser common issues](https://geysermc.org/wiki/geyser/common-issues/) — bind, Java, connection, Floodgate, IP/SRV, UDP/MTU cases.
- [Geyser unable to connect to world](https://geysermc.org/wiki/geyser/fixing-unable-to-connect-to-world/) — connection-test workflow and update/restart guidance.
- [Floodgate issues](https://geysermc.org/wiki/floodgate/issues/) — key mismatch, AEAD/authentication, forwarding, Xbox account issues.

### Voice chat

- [Simple Voice Chat troubleshooting](https://modrepo.de/minecraft/voicechat/wiki/troubleshooting) — UDP, bind, port collision, loader/API, corrupted JAR, mixin, client permissions.
- [Simple Voice Chat self-hosted setup](https://modrepo.de/minecraft/voicechat/wiki/server_setup_self_hosted) — UDP firewall/router and Docker mapping.
- [Simple Voice Chat server setup](https://modrepo.de/minecraft/voicechat/wiki/server_setup) — port config and voice-specific test.
- [Simple Voice Chat installation](https://modrepo.de/minecraft/voicechat/wiki/installation) — server/client component and loader/plugin variants.
- [Simple Voice Chat FAQ](https://modrepo.de/minecraft/voicechat/faq) — UDP and proxy limitations.
- [Issue: voice unavailable/timeouts](https://github.com/henkelmax/simple-voice-chat/issues/287).
- [Issue: cannot assign requested address](https://github.com/henkelmax/simple-voice-chat/issues/170).
- [Issue: voice unavailable despite port](https://github.com/henkelmax/simple-voice-chat/issues/695).

### Playit

- [Playit agent issues](https://github.com/playit-cloud/playit-agent/issues) — helper/control/network issue patterns.
- [Playit support: tunnel path and latency](https://playit.gg/support/how-to-lower-ping/).
- [Playit community: agent not connecting](https://discuss.playit.gg/t/playit-not-connecting-to-agent/5640).
- [Playit community: agent unable to connect to tunnel](https://discuss.playit.gg/t/agent-is-unable-to-connect-to-tunnel/1101).
- [Playit community: connection refused](https://discuss.playit.gg/t/connection-refused-getsockopt/3745/3).
- [Reddit: tunnel/local versus external connection and CGNAT](https://www.reddit.com/r/admincraft/comments/1slz73i/cant_connect_to_mc_server_via_tunnel_or_ip/).
- [Reddit: Playit ISP/VPN-specific issues](https://www.reddit.com/r/admincraft/comments/1utyknk/playitgg_issues_for_specfic_players/).
- [Reddit: Bedrock UDP tunneling](https://www.reddit.com/r/admincraft/comments/18hmlga/bedrock_server_tunnelling).

### Xbox Broadcast

- [MCXboxBroadcast official README](https://github.com/MCXboxBroadcast/Broadcaster/blob/master/README.md) — standalone JAR, device-code login, Geyser/Bedrock target configuration, commands, risks, and Docker.

### DuckDNS

- [DuckDNS HTTP API specification](https://www.duckdns.org/spec.jsp) — parameters and `OK`/`KO` responses.
- [DuckDNS installation/update examples](https://www.duckdns.org/install.jsp) — cron, logs, token/domain troubleshooting, operating-system/router patterns.
- [Community DuckDNS IPv6 issue](https://www.reddit.com/r/selfhosted/comments/1cpvrje/duckdns_isnt_updating_ipv6_adresses/).
- [Community token/update URL issue](https://www.reddit.com/r/TPLink_Omada/comments/1bxh0c2/working_ddns_with_duckdns_standalonehttpsuntruncated/).

### Tailscale

- [Tailscale Minecraft/private game server guide](https://tailscale.com/docs/solutions/set-up-minecraft).
- [Tailscale private game server sharing](https://tailscale.com/docs/use-cases/personal-or-at-home-use/share-private-game-server).
- [Tailscale subnet routers](https://tailscale.com/kb/1104/enable-ip-forwarding) — route advertisement, approval, forwarding, firewall/SNAT.
- [Tailscale connectivity troubleshooting](https://tailscale.com/kb/1463/troubleshoot-connectivity).
- [Tailscale no DERP home relay](https://tailscale.com/kb/1561/messages-client-no-derp-home).
- [Tailscale CLI/netcheck](https://tailscale.com/kb/1080/cli) — UDP, NAT, DERP and port-mapping signals.

### Mod providers

- [Modrinth API](https://docs.modrinth.com/api/) — IDs, rate limits, authentication, API behavior.
- [Modrinth get-version-dependencies](https://docs.modrinth.com/api/operations/getdependencies/) — game versions, loaders, dependencies, files, hashes.
- [Modrinth search projects](https://docs.modrinth.com/api/operations/searchprojects/) — catalog filtering.
- [CurseForge API overview](https://support.curseforge.com/support/solutions/articles/9000208346-about-the-curseforge-api).
- [CurseForge API-key download authentication](https://blog.curseforge.com/introducing-api-key-authentication-for-curseforge-file-downloads/) — `x-api-key` and 401 behavior.
- [CurseForge dependency data warning](https://docs.curseforge.com/docs/game-integration/unreal/mod-dependencies/) — dependency data is not the same as automatic resolution.
- [Reddit: author restrictions on third-party downloads](https://www.reddit.com/r/feedthebeast/comments/1souu7g/psa_curseforge_has_started_enforcing_restrictions_on_mod_downloads/).

---

## 14. Working principles to carry into implementation discussion

1. Diagnose the lifecycle phase before choosing the error class.
2. Keep server-start failures separate from reachability/helper failures.
3. Parse structured metadata and crash reports before relying on prose heuristics.
4. Use console parsing plus process, filesystem, port, provider, and helper evidence.
5. Represent known findings with stable codes, but keep an honest unknown path.
6. Rank recommended actions using compatibility metadata and reversibility.
7. Prefer stage/validate/repair/re-scan/retry over directly mutating a live server.
8. Make disable reversible; make delete explicit and backed up.
9. Preserve the original operation so initiate and normal start can both be retried.
10. Persist the incident across client/app restarts.
11. Show confidence and evidence when diagnosis is inferred.
12. Redact secrets, tokens, keys, passwords, and authorization responses.
13. Treat warnings as warnings unless evidence shows they blocked startup.
14. When a repair fixes one issue but reveals another, explain the sequence.
15. Keep raw logs available for users who want them, but never make raw logs the only path to understanding the problem.

