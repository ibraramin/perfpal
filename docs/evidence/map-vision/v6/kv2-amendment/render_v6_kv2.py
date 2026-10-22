#!/usr/bin/env python3
"""Render the V6 KV2 production-first amendment views (vision gate).

Reads only the three design repos (tmux, hyperfine, cJSON) from
`/home/shafin/Opencode/perfpal-test/out-v6-<repo>` (scanned with the amended
HEAD binary). The V6 holdout (django/postgres) is deliberately absent.

Images are written under neutral blind names (`01.png`..`03.png`); the
repo->image mapping + seed live in `manifest.json` and are revealed only
after the blind reads are recorded. Viewport: 1440x900, device_scale_factor=1
(same canonical path as V0/V3/V4/V5).
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
# Blind order fixed by the recorded seed; do not reorder without a new seed.
REPOS = ["tmux", "hyperfine", "cJSON"]
BLIND_SEED = 20261022


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
    mapping: dict[str, str] = {}
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=EXE)
        page = browser.new_page(
            viewport={"width": 1440, "height": 900}, device_scale_factor=1
        )
        for index, repo in enumerate(REPOS, start=1):
            report = TEST_ROOT / f"out-v6-{repo}" / "report.html"
            if not report.exists():
                raise SystemExit(f"FATAL: missing {report}")
            page.goto(report.as_uri())
            page.wait_for_load_state("load")
            page.set_viewport_size({"width": 1440, "height": 900})
            element = page.locator(".kv2")
            if element.count() != 1:
                raise SystemExit(f"FATAL: {repo} has {element.count()} .kv2")
            name = f"{index:02d}.png"
            out = OUT_DIR / name
            element.screenshot(path=str(out))
            width, height = png_size(out)
            images.append(
                {
                    "file": name,
                    "height": height,
                    "sha256": sha256(out),
                    "width": width,
                }
            )
            mapping[name] = repo
            print(f"rendered {name} ({repo})")
        browser.close()

    manifest = {
        "_comment": (
            "V6 KV2 production-first amendment render manifest. 3 blind element "
            "screenshots, one per design repo. Holdout deliberately absent. "
            "Mapping revealed after the blind reads were recorded."
        ),
        "blind_mapping": mapping,
        "blind_mapping_seed": BLIND_SEED,
        "generated": "2026-10-22",
        "holdout": "django/postgres deliberately absent (V6 timed test only)",
        "images": sorted(images, key=lambda item: str(item["file"])),
        "renderer": "Playwright chromium-1243",
        "scan_inputs": {
            repo: f"perfpal-test/out-v6-{repo} (schema 1.7)" for repo in REPOS
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
