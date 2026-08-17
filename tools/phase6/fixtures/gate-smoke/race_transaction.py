#!/usr/bin/env python3
"""Kill a real msc-agent process mid world-activation/backup-restore
transaction, to prove `reconcile_interrupted_activation`/
`reconcile_interrupted_restore` (P6.13/P6.18) actually recover a real
on-disk `.activation/`-or-`.restore/` transaction, not just a
fixture-shaped one.

Both transactions share one on-disk shape (`worlds.rs`'s own section
doc): `<marker-dir>/prior/` appears once the live world folders have
been moved aside, and `<marker-dir>/staged/` is removed once the
replacement has been moved into place. So "prior/ exists" is the
window where the live server directory has *no* complete world at all
-- the dangerous case `fixtures/world-mutations/
activate-extraction-failure-leaves-partial-state-for-safety-backup-recovery.json`
names, and the one worth actually catching with a real SIGKILL rather
than only asserting against a hand-built fixture.

The window is typically a handful of `rename()` syscalls wide (low
double-digit microseconds on a local SSD) -- too narrow to hit with a
fixed sleep, and (discovered running this on real Windows CI) too
narrow to reliably even *observe* by busy-polling there: GitHub's
Windows runners have enough per-syscall filesystem overhead that the
whole transaction can complete between one poll and the next,
regardless of how fast the kill itself lands afterward. `msc-agent`
(`worlds::test_pause_after_world_move`/`backups`'s call to it) closes
that gap by durably blocking in the real window when
`MSC2_TEST_PAUSE_AFTER_WORLD_MOVE` is set -- the caller
(`phase6-gate-smoke.sh`) sets it only for an agent instance it starts
dedicated to one racy call. That turns the window from microseconds
into "however long it takes this poller to notice", so the busy-poll
below still does real work (there is no other way to learn *when* to
kill) but no longer needs to win a timing race to succeed.

This script still busy-polls for `prior/`'s appearance concurrently
with a *blocking* CLI call (no `--no-wait`): the CLI process only
returns once the operation reaches a terminal state, so "the CLI call
returned and the poller never saw prior/" is a genuine, race-free
signal that the whole transaction completed normally -- letting the
driver alternate targets and try again without needing to guess at
timing anywhere. With the pause in place this should always catch on
the first attempt; the retry loop is left as a harmless fallback
rather than removed, in case a target's own call fails validation
before ever reaching the pause point.

P6.35 adds one more thing worth catching from the caught attempt: the
real operation id the killed CLI call was driving, scraped from its
own captured stdout (`extract_operation_id`) rather than invented --
`finish_operation`'s "operation id: <id>" line prints as soon as the
agent admits the operation, well before the on-disk work this script
races to interrupt. The caller uses it to fetch the operation's
durable record (`GET /v1/operations/{id}`) after the killed agent is
restarted, and confirm the record itself explains what happened
(`agent restarted mid-operation`) rather than only checking that the
recovered folders/markers look right.
"""
import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time


def hard_kill(pid: int) -> None:
    # `signal.SIGKILL` does not exist in Python's `signal` module on
    # Windows, and POSIX `os.kill` semantics (a real SIGKILL) have no
    # Windows equivalent via `os.kill` at all. Spawning `taskkill.exe`
    # (a whole new process, routinely tens of milliseconds) was the
    # first fix here (P6.27), but the real interruption window this
    # script targets is a handful of `rename()` syscalls wide -- low
    # double-digit microseconds, per this file's own module doc -- so
    # that spawn latency is enough for the agent to race past the
    # window and finish the whole transaction before the kill lands.
    # `TerminateProcess` via `ctypes` is a single direct WinAPI call,
    # not a process spawn, and lands at comparable latency to POSIX
    # `SIGKILL` below.
    if os.name == "nt":
        import ctypes

        PROCESS_TERMINATE = 0x0001
        kernel32 = ctypes.windll.kernel32
        handle = kernel32.OpenProcess(PROCESS_TERMINATE, False, pid)
        if handle:
            kernel32.TerminateProcess(handle, 1)
            kernel32.CloseHandle(handle)
    else:
        import signal

        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass


def process_alive(pid: int) -> bool:
    # `os.kill(pid, 0)` (POSIX's "is it there" probe) is not portable:
    # on Windows, CPython's `os.kill` has no null-signal case and would
    # call `TerminateProcess(handle, 0)` instead of merely checking.
    if os.name == "nt":
        result = subprocess.run(
            ["tasklist", "/FI", f"PID eq {pid}"],
            capture_output=True,
            text=True,
        )
        return str(pid) in result.stdout
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def extract_operation_id(stdout: str) -> str | None:
    # `finish_operation`'s own non-JSON print (`cli/mod.rs`), the first
    # thing it writes once the agent has journaled the operation and
    # admitted it for its target -- present in the captured stdout even
    # when this same process is killed later, mid-transaction, since the
    # print happens well before the on-disk work this script is racing
    # to catch.
    match = re.search(r"operation id:\s*(\S+)", stdout)
    return match.group(1) if match else None


def attempt(msc, base_url, token, argv_tail, pid, prior_dir, staged_dir):
    argv = [msc, "--base-url", base_url, "--token", token] + argv_tail
    result = {"caught": False}
    caught_event = threading.Event()

    def poller():
        while not caught_event.is_set():
            if os.path.isdir(prior_dir):
                result["caught"] = True
                result["phase"] = (
                    "prior_moved" if os.path.isdir(staged_dir) else "installed"
                )
                hard_kill(pid)
                caught_event.set()
                return

    poller_thread = threading.Thread(target=poller, daemon=True)
    poller_thread.start()
    stdout = ""
    try:
        proc = subprocess.run(argv, capture_output=True, timeout=30, text=True)
        stdout = proc.stdout
    except Exception:
        pass
    caught_event.set()
    poller_thread.join(timeout=2.0)
    result["operation_id"] = extract_operation_id(stdout)
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--msc", required=True)
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--token", required=True)
    parser.add_argument("--pid", type=int, required=True)
    parser.add_argument("--marker-dir", required=True)
    parser.add_argument("--cmd-a", required=True, help="comma-separated argv tail")
    parser.add_argument("--cmd-b", required=True, help="comma-separated argv tail")
    parser.add_argument("--start-with", choices=["a", "b"], default="a")
    parser.add_argument("--max-attempts", type=int, default=400)
    parser.add_argument("--max-seconds", type=float, default=60.0)
    args = parser.parse_args()

    prior_dir = os.path.join(args.marker_dir, "prior")
    staged_dir = os.path.join(args.marker_dir, "staged")
    cmds = {"a": args.cmd_a.split(","), "b": args.cmd_b.split(",")}

    target = args.start_with
    start = time.time()
    attempts = 0

    while attempts < args.max_attempts and (time.time() - start) < args.max_seconds:
        attempts += 1
        result = attempt(
            args.msc,
            args.base_url,
            args.token,
            cmds[target],
            args.pid,
            prior_dir,
            staged_dir,
        )
        if result["caught"]:
            deadline = time.time() + 5.0
            while time.time() < deadline and process_alive(args.pid):
                time.sleep(0.02)
            print(
                json.dumps(
                    {
                        "caught": True,
                        "winning_target": target,
                        "phase": result["phase"],
                        "attempts": attempts,
                        "elapsed": time.time() - start,
                        "operation_id": result.get("operation_id"),
                    }
                )
            )
            return 0
        # This attempt's CLI call returned only once the operation
        # reached a terminal state and the poller never saw prior/, so
        # the transaction genuinely completed -- `target` is now live.
        target = "b" if target == "a" else "a"

    print(json.dumps({"caught": False, "attempts": attempts, "elapsed": time.time() - start}))
    return 3


if __name__ == "__main__":
    sys.exit(main())
