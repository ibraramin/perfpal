#!/usr/bin/env python3
"""Measure the in-report H1 size/FCP budget (ADR-0014 Decision 8).

Usage:
    python3 sim/report_budget.py <report.html> --ceiling-bytes N --ceiling-fcp-ms N

Prints the report path, HEAD, bytes, FCP median (1440x900, headless Brave,
file://, 3 runs), the ceilings, and PASS/FAIL. Exit 0 only when both are inside
the ceiling. This is the runnable budget gate; the report is otherwise offline.
"""
from __future__ import annotations

import argparse
import json
import os
import statistics
import subprocess
from pathlib import Path

from playwright.sync_api import sync_playwright


def fcp_ms(url: str) -> list[float]:
    values: list[float] = []
    with sync_playwright() as p:
        browser = p.chromium.launch(
            executable_path=os.environ.get("PERFPAL_BROWSER", "/usr/bin/brave"),
            headless=True,
        )
        for _ in range(3):
            page = browser.new_page(viewport={"width": 1440, "height": 900})
            page.goto(url, wait_until="load")
            page.wait_for_timeout(150)
            value = page.evaluate(
                """() => {
                  const p = performance.getEntriesByType('paint');
                  const f = p.find(e => e.name === 'first-contentful-paint');
                  return f ? f.startTime : null;
                }"""
            )
            if value is not None:
                values.append(float(value))
            page.close()
        browser.close()
    return values


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("report")
    parser.add_argument("--ceiling-bytes", type=int, required=True)
    parser.add_argument("--ceiling-fcp-ms", type=int, required=True)
    args = parser.parse_args()
    path = Path(args.report).resolve()
    if not path.is_file():
        raise SystemExit(f"missing report: {path}")
    repo = Path(__file__).resolve().parent.parent
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=repo, capture_output=True, text=True
    ).stdout.strip()
    runs = fcp_ms(path.as_uri())
    median = round(statistics.median(runs), 1) if runs else None
    size = path.stat().st_size
    ok = size <= args.ceiling_bytes and median is not None and median <= args.ceiling_fcp_ms
    result = {
        "commit": commit or None,
        "report": str(path),
        "bytes": size,
        "ceiling_bytes": args.ceiling_bytes,
        "fcp_ms_runs": runs,
        "fcp_ms_median": median,
        "ceiling_fcp_ms": args.ceiling_fcp_ms,
        "verdict": "PASS" if ok else "FAIL",
    }
    print(json.dumps(result, indent=2))
    if not ok:
        raise SystemExit("budget FAIL")


if __name__ == "__main__":
    main()
