"""Regression checks for the ICC harness exit-code resolution.

Run with:
    python3 -m unittest discover -s tools/icc -p 'test_*.py'
"""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from run_icc import resolve_exit  # noqa: E402


class ResolveExitTests(unittest.TestCase):
    def test_failed_compute_beats_stale_freeze_ok(self) -> None:
        # Regression: a stale results.json with freeze_ok=true used to turn a
        # failed recomputation into exit 0.
        self.assertEqual(resolve_exit(1, {"freeze_ok": True}), 1)
        self.assertEqual(resolve_exit(2, {"freeze_ok": True}), 2)

    def test_successful_compute_trusts_freeze_ok(self) -> None:
        self.assertEqual(resolve_exit(0, {"freeze_ok": True}), 0)
        self.assertEqual(resolve_exit(0, {"freeze_ok": False}), 1)

    def test_missing_results_falls_back_to_compute_code(self) -> None:
        self.assertEqual(resolve_exit(0, None), 0)
        self.assertEqual(resolve_exit(1, None), 1)

    def test_missing_freeze_uses_gate_fallback(self) -> None:
        results = {"gate": {"loop_body_loc": {"pass": True}}}
        self.assertEqual(resolve_exit(0, results), 0)
        failed = {"gate": {"loop_body_loc": {"pass": False}}}
        self.assertEqual(resolve_exit(0, failed), 1)


if __name__ == "__main__":
    unittest.main()
