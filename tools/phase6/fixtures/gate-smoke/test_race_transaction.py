#!/usr/bin/env python3
"""Focused tests for the restart-race subprocess evidence capture."""

import importlib.util
from pathlib import Path
import sys
import unittest


MODULE_PATH = Path(__file__).with_name("race_transaction.py")
SPEC = importlib.util.spec_from_file_location("race_transaction", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
RACE_TRANSACTION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RACE_TRANSACTION)


class RaceTransactionOutputTests(unittest.TestCase):
    def test_operation_id_survives_cli_timeout(self) -> None:
        stdout = RACE_TRANSACTION.run_cli_capture_stdout(
            [
                sys.executable,
                "-c",
                (
                    "import time; "
                    "print('operation id: op-timeout-proof', flush=True); "
                    "time.sleep(10)"
                ),
            ],
            timeout=0.1,
        )

        self.assertEqual(
            RACE_TRANSACTION.extract_operation_id(stdout),
            "op-timeout-proof",
        )

    def test_operation_id_is_captured_after_normal_exit(self) -> None:
        stdout = RACE_TRANSACTION.run_cli_capture_stdout(
            [sys.executable, "-c", "print('operation id: op-normal-proof')"],
            timeout=5,
        )

        self.assertEqual(
            RACE_TRANSACTION.extract_operation_id(stdout),
            "op-normal-proof",
        )


if __name__ == "__main__":
    unittest.main()
