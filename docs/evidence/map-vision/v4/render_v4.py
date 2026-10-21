#!/usr/bin/env python3
"""Render map-intelligibility V4 key views from real scan output.

Reads only the three design repos (tmux, hyperfine, cJSON) from
`/home/shafin/Opencode/perfpal-test/out-v4-<repo>` (refreshed with the V4
binary). The V6 holdout (django/postgres) is deliberately absent.

Viewport: 1440x900, device_scale_factor=1 (same canonical path as V0/V3).
Outputs: `<repo>-first-screen.png` (viewport, no scroll) and one element
screenshot per key view (`<repo>-kv{1,2,3}.png`).
"""

from pathlib import Path

from playwright.sync_api import sync_playwright

EXE = "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
OUT_DIR = Path(__file__).resolve().parent
REPOS = ["tmux", "hyperfine", "cJSON"]


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=EXE)
        page = browser.new_page(
            viewport={"width": 1440, "height": 900}, device_scale_factor=1
        )
        for repo in REPOS:
            report = TEST_ROOT / f"out-v4-{repo}" / "report.html"
            if not report.exists():
                raise SystemExit(f"FATAL: missing {report}")
            page.goto(report.as_uri())
            page.wait_for_load_state("load")
            page.set_viewport_size({"width": 1440, "height": 900})
            page.evaluate("window.scrollTo(0, 0)")
            page.screenshot(path=str(OUT_DIR / f"{repo}-first-screen.png"))
            for slot in ("kv1", "kv2", "kv3"):
                element = page.locator(f".{slot}")
                if element.count() != 1:
                    raise SystemExit(f"FATAL: {repo} has {element.count()} .{slot}")
                element.screenshot(path=str(OUT_DIR / f"{repo}-{slot}.png"))
            print(f"rendered {repo}")
        browser.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
