# Intel Mac XboxBroadcast correction

P16.89 corrects Bedrock helper message ordering, prevents a recoverable event
error from stopping the live server's helpers, and moves broadcast acquisition
off the async executor and outside the broadcast state lock. RakNet selection
is unchanged. The October 16 process sample captured the Playit background
status check entering Bedrock event handling and stopping broadcast while
broadcast startup held its state lock during an HTTP artifact download.
The sample identifies the stop path and contention; it does not contain the
specific rejected Bedrock frame. Interleaved event/reply handling is a confirmed
source defect that can produce that path.

## Fast delivery

Only the Rust agent needs recompilation. On an Intel Mac with the checkout:

```sh
cargo build --locked -p msc-agent --bin msc
bash tools/release/prepare-macos-agent-fix.sh target/debug/msc 0.1.29 target/broadcast-agent-fix
```

The preparation script requires the existing installed desktop app to be
version 0.1.29. It copies that app, replaces its bundled agent, and signs the
replacement agent and outer app locally. The Swift helper, VM appliance,
desktop executable and map helpers are reused. It verifies the resulting
bundle. It changes no installed app, service, settings or world data.

For the prepared full-app delivery, the published v0.1.29 Intel DMG is checked
against its published SHA256SUMS and used as the base instead of an older
locally installed app. AirDrop `msc2-broadcast-fix-intel.zip` to the affected
Mac and extract it. It contains the prepared `MSC 2.app`, installation notes,
and BUILD.txt identifying the source commit and development agent checksum.
No Rust toolchain or build is required on the receiving Mac.

Stop Minecraft through MSC and wait until it reports Stopped. Quit MSC, then
copy the prepared `patched/MSC 2.app` (or the checkout's output app) into
Applications and replace the old app. Open the new app and use **Repair
agent** before starting Minecraft. Repair stages the changed bundled binary
under its correct content digest and updates the desktop bootstrap identity.
Do not overwrite a cached `agent/builds/<digest>/msc` directly: desktop startup
checks those bytes against its bundled payload and will reject a mismatch.

This is a temporary local build with the existing version number, not a
published release. A future normal app update replaces it. Keep the original
0.1.29 installer for rollback; reinstall it and Repair agent while Minecraft
is stopped. The fast agent is a development build, so its performance and
size differ from a release build. No release workflow or tag is required.

## Owner verification

With the corrected app installed and its agent repaired, retain RakNet,
start the affected Bedrock server and enable broadcast. Require Microsoft
sign-in or helper readiness output, running broadcast, and continued running
status through routine time/status polling. Then stop broadcast during
preparation and require that a late download completion does not launch it.
World data, ports and transport settings must remain unchanged.

```sh
/usr/local/bin/msc broadcast start
/usr/local/bin/msc --json broadcast status
/usr/local/bin/msc console tail -n 100
```

If an event is still rejected, its error is now written to both the server
console and `~/Library/Application Support/MSC 2/logs/agent.log`. Build and
type checks do not establish acceptance on the affected Mac.
