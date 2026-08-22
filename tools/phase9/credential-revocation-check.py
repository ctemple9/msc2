#!/usr/bin/env python3
"""Run the durable named-token revocation proof without production data."""

from __future__ import annotations

import platform
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


def main() -> int:
    if platform.system() != "Darwin":
        # The live service test uses macOS's production Keychain adapter.  The
        # other platforms still get repository persistence coverage from the
        # synthetic smoke; claiming a live SecretStore restart proof here
        # would be dishonest until their platform adapters have this test.
        print(
            f"credential revocation live evidence unavailable on {platform.system()}; "
            "production Keychain restart proof is macOS-only"
        )
        return 0

    command = [
        "cargo",
        "nextest",
        "run",
        "-p",
        "msc-agent",
        "--test",
        "user_routes",
        "--no-fail-fast",
    ]
    print("== running isolated production SecretStore revocation tests ==")
    result = subprocess.run(command, cwd=ROOT)
    if result.returncode:
        print("FAIL: named-token CRUD/revocation restart proof failed", file=sys.stderr)
        return result.returncode
    print("credential revocation: create/list/update/revoke and post-restart rejection passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
