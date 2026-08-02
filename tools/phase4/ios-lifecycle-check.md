# Phase 4 iOS lifecycle check

Purpose: prove the existing iOS app can drive the same imported-Paper lifecycle slice as the CLI and API.

Use this after the Phase 4 agent is running and already has one imported Paper server available.

## Preconditions

1. Build and launch the iOS app:
   `xcodebuild -project clients/ios/MSCRemoteiOS.xcodeproj -scheme MSCRemoteiOS build`
2. Start the Phase 4 agent and pair the app with a valid bearer token.
3. Confirm the imported Paper server exists in the agent already. This checklist does not import from iOS.

## Checklist

1. Pair
   Open Settings in the iOS app.
   Enter or scan the agent base URL and bearer token.
   Tap Save.
   Expected: the pairing card changes to `Paired`, and the role pill appears if the token is valid.

2. See the imported server
   Open the Dashboard.
   Expected: the imported Paper server appears in the server picker/list, and it can be set as the active server if it is not active already.

3. Start
   With the imported Paper server active, tap Start.
   Expected: the dashboard status changes from stopped/starting to running.

4. Watch status become running
   Stay on Dashboard until the running state settles.
   Expected: running indicators update from the live `/v1/status` response rather than remaining in a loading or stale state.

5. Send a command
   Open Commands or Console.
   Send `say ios lifecycle check`.
   Expected: the command request succeeds without an auth or routing error.

6. See console output
   Stay in Console with live streaming enabled if available.
   Expected: recent output includes the Paper ready line and a line reflecting the sent `say ios lifecycle check` command.

7. Stop
   Return to Dashboard and tap Stop.
   Expected: the server transitions to stopping, then stopped.

8. Restart
   Tap Restart from the stopped or running lifecycle controls, whichever the UI exposes for the current state.
   Expected: the server returns to running, and Console shows a fresh startup sequence after the restart.

## Record

When you run this, add a short result note to `docs/msc2/rolling-plan.md` under `P4.20`:

- device or simulator used
- whether pair, start, command, console, stop, and restart all passed
- any bug found, with the exact screen and action that exposed it
