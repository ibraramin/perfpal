#!/usr/bin/env python3
"""Measure the in-report H1/H2 size/FCP budget (ADR-0014 Decision 8).

Usage:
    python3 sim/report_budget.py <report.html> \
        --ceiling-bytes N --ceiling-fcp-ms N [--widths 1440,3072]

Prints HEAD, report path, bytes, and the FCP median of three runs per viewport
width (headless Brave, file://, 150ms settle), plus the ceilings and PASS/FAIL.
Exit 0 only when the byte count and every width's FCP median are inside the
ceiling. This is the runnable budget gate; the report is otherwise offline.
"""
from __future__ import annotations

import argparse
import json
import os
import statistics
import subprocess
from pathlib import Path

from playwright.sync_api import sync_playwright

RUNS_PER_WIDTH = 3
VIEWPORT_HEIGHT = 900
SETTLE_MS = 150


def parse_widths(raw: str) -> list[int]:
    widths: list[int] = []
    for part in raw.split(","):
        part = part.strip()
        if not part:
            continue
        try:
            width = int(part)
        except ValueError:
            raise SystemExit(f"invalid --widths value: {part!r}") from None
        if width <= 0:
            raise SystemExit(f"--widths values must be positive CSS px, got {width}")
        if width not in widths:
            widths.append(width)
    if not widths:
        raise SystemExit("--widths must contain at least one width")
    return widths


def measure_fcp(browser, url: str, widths: list[int]) -> dict[str, dict]:
    out: dict[str, dict] = {}
    for width in widths:
        runs: list[float | None] = []
        for _ in range(RUNS_PER_WIDTH):
            page = browser.new_page(viewport={"width": width, "height": VIEWPORT_HEIGHT})
            try:
                page.goto(url, wait_until="load")
                page.wait_for_timeout(SETTLE_MS)
                value = page.evaluate(
                    """() => {
                      const paints = performance.getEntriesByType('paint');
                      const fcp = paints.find((entry) => entry.name === 'first-contentful-paint');
                      return fcp ? fcp.startTime : null;
                    }"""
                )
            finally:
                page.close()
            runs.append(float(value) if value is not None else None)
        measured = [value for value in runs if value is not None]
        median = round(statistics.median(measured), 1) if measured else None
        out[str(width)] = {"runs": runs, "median": median}
    return out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("report")
    parser.add_argument("--ceiling-bytes", type=int, required=True)
    parser.add_argument("--ceiling-fcp-ms", type=int, required=True)
    parser.add_argument("--widths", default="1440,3072")
    args = parser.parse_args()

    widths = parse_widths(args.widths)
    path = Path(args.report).resolve()
    if not path.is_file():
        raise SystemExit(f"missing report: {path}")
    repo = Path(__file__).resolve().parent.parent
    commit = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=repo, capture_output=True, text=True
    ).stdout.strip()

    size = path.stat().st_size
    url = path.as_uri()
    with sync_playwright() as p:
        browser = p.chromium.launch(
            executable_path=os.environ.get("PERFPAL_BROWSER", "/usr/bin/brave"),
            headless=True,
        )
        try:
            per_width = measure_fcp(browser, url, widths)
        finally:
            browser.close()

    for entry in per_width.values():
        entry["ceiling_fcp_ms"] = args.ceiling_fcp_ms
        entry["pass"] = entry["median"] is not None and entry["median"] <= args.ceiling_fcp_ms

    bytes_ok = size <= args.ceiling_bytes
    widths_ok = all(entry["pass"] for entry in per_width.values())
    ok = bytes_ok and widths_ok
    result = {
        "commit": commit or None,
        "report": str(path),
        "bytes": size,
        "ceiling_bytes": args.ceiling_bytes,
        "ceiling_fcp_ms": args.ceiling_fcp_ms,
        "widths": per_width,
        "verdict": "PASS" if ok else "FAIL",
    }
    print(json.dumps(result, indent=2))
    if not ok:
        raise SystemExit("budget FAIL")


if __name__ == "__main__":
    main()
