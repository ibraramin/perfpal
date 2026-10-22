#!/usr/bin/env python3
"""Render map-intelligibility V5 key views from real scan output.

Reads only the three design repos (tmux, hyperfine, cJSON) from
`/home/shafin/Opencode/perfpal-test/out-v5-<repo>` (scanned with the HEAD
binary at 758f492). The V6 holdout (django/postgres) is deliberately absent.

Viewport: 1440x900, device_scale_factor=1 (same canonical path as V0/V3/V4).
Outputs: one element screenshot per key view (`<repo>-kv{1,2,3}.png`) plus a
`manifest.json` carrying dimensions and sha256 per image.
"""

from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path

from playwright.sync_api import sync_playwright

EXE = "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
OUT_DIR = Path(__file__).resolve().parent
REPOS = ["tmux", "hyperfine", "cJSON"]
SLOTS = ("kv1", "kv2", "kv3")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def png_size(path: Path) -> tuple[int, int]:
    header = path.read_bytes()[:24]
    if header[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"FATAL: {path} is not a PNG")
    width, height = struct.unpack(">II", header[16:24])
    return width, height


def main() -> int:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    images: list[dict[str, object]] = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=EXE)
        page = browser.new_page(
            viewport={"width": 1440, "height": 900}, device_scale_factor=1
        )
        for repo in REPOS:
            report = TEST_ROOT / f"out-v5-{repo}" / "report.html"
            if not report.exists():
                raise SystemExit(f"FATAL: missing {report}")
            page.goto(report.as_uri())
            page.wait_for_load_state("load")
            page.set_viewport_size({"width": 1440, "height": 900})
            for slot in SLOTS:
                element = page.locator(f".{slot}")
                if element.count() != 1:
                    raise SystemExit(f"FATAL: {repo} has {element.count()} .{slot}")
                out = OUT_DIR / f"{repo}-{slot}.png"
                element.screenshot(path=str(out))
                width, height = png_size(out)
                images.append(
                    {
                        "file": out.name,
                        "height": height,
                        "sha256": sha256(out),
                        "width": width,
                    }
                )
            print(f"rendered {repo}")
        browser.close()

    manifest = {
        "_comment": (
            "V5 key-view render manifest. 9 element screenshots (one per key view) "
            "for the three design repos. The V6 holdout is deliberately absent."
        ),
        "generated": "2026-10-22",
        "holdout": "django/postgres deliberately absent (V6)",
        "images": sorted(images, key=lambda item: str(item["file"])),
        "renderer": "Playwright chromium-1243",
        "repo": "/home/shafin/Opencode/Perfpal",
        "scan_inputs": {
            repo: f"perfpal-test/out-v5-{repo} (schema 1.7)" for repo in REPOS
        },
        "viewport": "1440x900 @ device_scale_factor=1; element screenshots at 1152 CSS px",
    }
    (OUT_DIR / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote manifest with {len(images)} images")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
