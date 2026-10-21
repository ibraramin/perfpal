#!/usr/bin/env python3
"""Render the 18 V3 prototype pages to PNG and update manifest dimensions.

Copies the V0 Playwright pattern (chromium 1243, 1440x900, device_scale_factor=1).
PNG dimensions are read from the IHDR header with struct (no PIL).
"""

import json
import struct
from pathlib import Path

from playwright.sync_api import sync_playwright

V3_DIR = Path("/home/shafin/Opencode/Perfpal/docs/evidence/map-vision/v3")
MANIFEST = V3_DIR / "manifest.json"
EXE = "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"


def png_dims(path):
    with open(path, "rb") as handle:
        header = handle.read(24)
    if len(header) < 24 or header[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"FATAL: not a PNG: {path}")
    width, height = struct.unpack(">II", header[16:24])
    return [width, height]


def main():
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    renders = manifest["renders"]
    rows = []

    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=EXE)
        context = browser.new_context(
            viewport={"width": 1440, "height": 900}, device_scale_factor=1
        )
        page = context.new_page()
        page.set_default_timeout(60000)

        for render in renders:
            html_path = Path(render["html"])
            png_path = Path(render["png"])
            png_path.parent.mkdir(parents=True, exist_ok=True)

            page.set_viewport_size({"width": 1440, "height": 900})
            page.goto(html_path.as_uri(), wait_until="load")
            page.wait_for_timeout(150)
            doc_h = page.evaluate("document.documentElement.scrollHeight")
            viewport_h = max(900, int(doc_h) + 20)
            page.set_viewport_size({"width": 1440, "height": viewport_h})
            page.wait_for_timeout(150)
            page.screenshot(path=str(png_path))

            dims = png_dims(png_path)
            render["png_dims"] = dims
            render["fits_900"] = bool(doc_h <= 900)
            rows.append(
                (
                    render["repo"],
                    f'{render["slot"]}-{render["idiom"]}',
                    dims,
                    render["fits_900"],
                    int(doc_h),
                )
            )

        browser.close()

    MANIFEST.write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )

    print(f"rendered {len(rows)} PNGs")
    print(f"{'repo':10} {'slot-idiom':14} {'png':12} fits_900  docH")
    for repo, name, dims, fits, doc_h in rows:
        print(f"{repo:10} {name:14} {str(dims):12} {str(fits):8} {doc_h}")

    bad = [row for row in rows if row[2][0] != 1440]
    if bad:
        raise SystemExit(f"FATAL: non-1440 PNG width: {bad}")
    print(f"manifest: {MANIFEST}")


if __name__ == "__main__":
    main()
