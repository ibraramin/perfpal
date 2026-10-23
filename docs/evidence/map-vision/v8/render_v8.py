#!/usr/bin/env python3
"""V8 vision-matrix renderer.

Screenshots the eight synthetic Atlas fixture states from ``html/`` at a
1440x900 viewport, writes one final frame and one weight-strip crop per
state, and records a deterministic manifest with hashes and dimensions.

Run from anywhere::

    python3 docs/evidence/map-vision/v8/render_v8.py

Requires ``playwright`` (``pip install playwright`` plus ``playwright install
chromium``). No network access is performed by this script.
"""

from __future__ import annotations

import hashlib
import json
import os
import struct
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parent
HTML_DIR = ROOT / "html"
W, H = 1440, 900
STATES = [
    "application",
    "library",
    "no-entry",
    "empty",
    "all-test",
    "capped",
    "partial-parse",
    "single-file",
]
PNG_MAGIC = b"\x89PNG\r\n\x1a\n"
STRUCTURAL_CAPTION = (
    "structural (LOW name-based + MEDIUM semantic), may over/under-approximate"
)
# Rendered-size floors at the canonical 1440x900 viewport (SVG text includes
# the viewBox/CSS scale). Keep these in sync with the canvas font sizes.
RENDERED_FLOORS = {
    "node name": 9.9,
    "node meta": 8.4,
    "node badge": 7.9,
    "column header": 10.4,
    "aggregate": 8.4,
    "hull label": 9.0,
}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def png_dimensions(path: Path) -> tuple[int, int]:
    data = path.read_bytes()
    if len(data) < 24 or data[:8] != PNG_MAGIC:
        raise SystemExit(f"not a PNG file: {path}")
    if data[12:16] != b"IHDR":
        raise SystemExit(f"PNG does not start with IHDR: {path}")
    width, height = struct.unpack(">II", data[16:24])
    return width, height


def latest_cached_chromium() -> Path | None:
    cache = Path.home() / ".cache" / "ms-playwright"
    candidates = []
    for chrome in cache.glob("chromium-*/chrome-linux64/chrome"):
        version = chrome.parent.parent.name.removeprefix("chromium-")
        try:
            key = int(version.split(".")[0])
        except (ValueError, IndexError):
            key = 0
        candidates.append((key, chrome))
    if not candidates:
        return None
    candidates.sort(key=lambda item: item[0])
    return candidates[-1][1]


def launch_browser(pw):
    override = os.environ.get("PERFPAL_CHROMIUM", "").strip()
    if override:
        executable = Path(override).expanduser()
        if not executable.is_file():
            raise SystemExit(f"PERFPAL_CHROMIUM does not exist: {executable}")
        browser = pw.chromium.launch(executable_path=str(executable))
        print(f"chromium executable: {executable} (PERFPAL_CHROMIUM)")
        return browser

    try:
        browser = pw.chromium.launch()
    except Exception:
        pinned = Path.home() / ".cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
        executable = pinned if pinned.is_file() else latest_cached_chromium()
        if executable is None:
            raise SystemExit(
                "could not launch chromium from Playwright and no cached executable "
                "was found under ~/.cache/ms-playwright"
            )
        browser = pw.chromium.launch(executable_path=str(executable))
        print(f"chromium executable: {executable} (cached fallback)")
        return browser

    print("chromium executable: playwright default")
    return browser


def measure_state(page) -> dict:
    """Measure the legibility invariants in a loaded fixture page."""
    return page.evaluate(
        """
        () => {
          const head = document.querySelector('.atlas-head');
          const keys = document.querySelector('.atlas-keys');
          const canvas = document.querySelector('#atlas-canvas');
          const firstNode = document.querySelector('.atlas-node');
          const keysBox = keys ? keys.getBoundingClientRect() : null;
          const firstNodeBox = firstNode ? firstNode.getBoundingClientRect() : null;
          // Rendered font size: SVG text is scaled by the SVG viewBox/CSS
          // transform, so computed font-size alone misses the canvas scale.
          const renderedSize = (el) => {
            const size = parseFloat(getComputedStyle(el).fontSize);
            if (!el.ownerSVGElement) return size;
            const ctm = el.getScreenCTM();
            return ctm ? size * ctm.a : size;
          };
          const minOf = (nodes) =>
            nodes.reduce((acc, el) => {
              const size = renderedSize(el);
              return acc === null ? size : Math.min(acc, size);
            }, null);
          const texts = [...document.querySelectorAll('.atlas-node text')];
          let minName = null;
          let minMeta = null;
          let minBadge = null;
          const overflows = [];
          for (const el of texts) {
            const size = renderedSize(el);
            const cls = el.getAttribute('class') || '';
            if (cls.includes('atlas-node-name')) {
              minName = minName === null ? size : Math.min(minName, size);
            }
            if (cls.includes('atlas-node-meta')) {
              minMeta = minMeta === null ? size : Math.min(minMeta, size);
            }
            if (cls.includes('atlas-node-badge')) {
              minBadge = minBadge === null ? size : Math.min(minBadge, size);
            }
            const node = el.closest('.atlas-node');
            const rectEl = node ? node.querySelector('rect') : null;
            if (!node || !rectEl) continue;
            const textBox = el.getBoundingClientRect();
            const rectBox = rectEl.getBoundingClientRect();
            if (textBox.left < rectBox.left - 2 || textBox.right > rectBox.right + 2) {
              overflows.push({
                text: (el.textContent || '').slice(0, 80),
                left: Math.round(textBox.left * 10) / 10,
                right: Math.round(textBox.right * 10) / 10,
                rectLeft: Math.round(rectBox.left * 10) / 10,
                rectRight: Math.round(rectBox.right * 10) / 10,
              });
            }
          }
          const minColumn = minOf([...document.querySelectorAll('.atlas-column-header')]);
          const minAggregate = minOf([...document.querySelectorAll('.atlas-aggregate text')]);
          const minHull = minOf([...document.querySelectorAll('.atlas-hull text')]);
          const graph = document.querySelector('.atlas-graph');
          const svg = document.querySelector('.atlas-svg');
          const corner = document.elementFromPoint(1, 1);
          const edgeLabelWidths = [...document.querySelectorAll('.atlas-edge-label')]
            .map((el) => Math.round(el.getBoundingClientRect().width * 10) / 10);
          return {
            head: head ? (head.innerText || head.textContent || '') : '',
            keys: keys ? (keys.innerText || keys.textContent || '') : '',
            keysTop: keysBox ? Math.round(keysBox.top * 10) / 10 : null,
            keysBottom: keysBox ? Math.round(keysBox.bottom * 10) / 10 : null,
            firstNodeTop: firstNodeBox
              ? Math.round(firstNodeBox.top * 10) / 10
              : null,
            canvasTop: canvas
              ? Math.round(canvas.getBoundingClientRect().top * 10) / 10
              : null,
            minName,
            minMeta,
            minBadge,
            minColumn,
            minAggregate,
            minHull,
            overflows,
            canvasWidth: graph ? graph.getBoundingClientRect().width : null,
            svgHeight: svg ? svg.getBoundingClientRect().height : null,
            viewBox: svg ? svg.getAttribute('viewBox') : null,
            cornerTag: corner ? corner.tagName.toLowerCase() : null,
            cornerInCanvas: !!(corner && corner.closest('#atlas-canvas')),
            edgeLabelWidths,
          };
        }
        """
    )


def preflight_state(page, state: str) -> tuple[dict, list[str]]:
    """Return the measurements plus every failed legibility invariant."""
    # Park the pointer in the page's top-left corner before measuring so no
    # node's native <title> tooltip can be open (and so screenshots below are
    # clean). Native tooltips are not inspectable via the DOM, so we assert the
    # hit-test target at (1, 1) is chrome, not the canvas.
    page.mouse.move(1, 1)
    page.wait_for_timeout(50)
    data = measure_state(page)
    errors: list[str] = []
    head = data["head"]
    for needle in ("static cost units (v2)", "median function", STRUCTURAL_CAPTION):
        if needle not in head:
            errors.append(f"canvas header is missing `{needle}`")
    keys = data["keys"]
    for needle in ("edge ×N", "numbered badges", "relative scale", STRUCTURAL_CAPTION):
        if needle not in keys:
            errors.append(f"first-screen key line is missing `{needle}`")
    if (
        data["keysBottom"] is not None
        and data["firstNodeTop"] is not None
        and data["keysBottom"] >= H
    ):
        errors.append(
            f"first-screen key line ends at {data['keysBottom']}px, below the {H}px fold"
        )
    if data["firstNodeTop"] is not None and data["firstNodeTop"] >= H:
        errors.append(
            f"first node starts at {data['firstNodeTop']}px, below the {H}px fold"
        )
    if data["minName"] is not None and data["minName"] < RENDERED_FLOORS["node name"]:
        errors.append(
            f"node name rendered size {data['minName']}px < "
            f"{RENDERED_FLOORS['node name']}px"
        )
    if data["minMeta"] is not None and data["minMeta"] < RENDERED_FLOORS["node meta"]:
        errors.append(
            f"node meta rendered size {data['minMeta']}px < "
            f"{RENDERED_FLOORS['node meta']}px"
        )
    if data["minBadge"] is not None and data["minBadge"] < RENDERED_FLOORS["node badge"]:
        errors.append(
            f"node badge rendered size {data['minBadge']}px < "
            f"{RENDERED_FLOORS['node badge']}px"
        )
    if (
        data["minColumn"] is not None
        and data["minColumn"] < RENDERED_FLOORS["column header"]
    ):
        errors.append(
            f"column header rendered size {data['minColumn']}px < "
            f"{RENDERED_FLOORS['column header']}px"
        )
    if (
        data["minAggregate"] is not None
        and data["minAggregate"] < RENDERED_FLOORS["aggregate"]
    ):
        errors.append(
            f"aggregate text rendered size {data['minAggregate']}px < "
            f"{RENDERED_FLOORS['aggregate']}px"
        )
    if data["minHull"] is not None and data["minHull"] < RENDERED_FLOORS["hull label"]:
        errors.append(
            f"hull label rendered size {data['minHull']}px < "
            f"{RENDERED_FLOORS['hull label']}px"
        )
    for overflow in data["overflows"]:
        errors.append(
            "node text overflows its rect horizontally: "
            f"`{overflow['text']}` "
            f"[{overflow['left']}, {overflow['right']}] vs rect "
            f"[{overflow['rectLeft']}, {overflow['rectRight']}]"
        )
    if data["cornerInCanvas"]:
        errors.append(
            "mouse corner (1, 1) hit-tests inside #atlas-canvas: native node "
            "tooltip risk"
        )
    if data["cornerTag"] not in (None, "body", "html"):
        errors.append(
            f"mouse corner (1, 1) hit-tests `{data['cornerTag']}`, expected body/html"
        )
    for width in data["edgeLabelWidths"]:
        if width > 32:
            errors.append(
                f"edge label is {width}px wide (> 32px); the compact pill regressed"
            )
    return data, errors


def format_min(value) -> str:
    return "n/a" if value is None else f"{value:.1f}px"


def main() -> int:
    missing = [state for state in STATES if not (HTML_DIR / f"{state}.html").is_file()]
    if missing:
        print(
            "missing fixture HTML in "
            f"{HTML_DIR}: {', '.join(missing)}\n"
            "write them first with:\n"
            "  PERFPAL_ATLAS_VISION_DIR=docs/evidence/map-vision/v8/html "
            "cargo test --locked --test atlas_vision -- --ignored",
            file=sys.stderr,
        )
        return 1

    # Preflight every state first: if any invariant fails we write nothing.
    measured = []
    failures = []
    with sync_playwright() as pw:
        browser = launch_browser(pw)
        try:
            for state in STATES:
                html_path = HTML_DIR / f"{state}.html"
                context = browser.new_context(
                    viewport={"width": W, "height": H},
                    device_scale_factor=1,
                )
                try:
                    page = context.new_page()
                    page.emulate_media(reduced_motion="reduce")
                    page.goto(html_path.resolve().as_uri())
                    page.wait_for_selector("#map")
                    page.wait_for_selector("#atlas-weight")
                    page.wait_for_timeout(150)
                    data, errors = preflight_state(page, state)
                finally:
                    context.close()
                measured.append(data)
                canvas = (
                    "n/a"
                    if data["canvasWidth"] is None
                    else f"{data['canvasWidth']:.0f}px"
                )
                height = (
                    "n/a" if data["svgHeight"] is None else f"{data['svgHeight']:.0f}px"
                )
                labels = data["edgeLabelWidths"]
                max_label = "n/a" if not labels else f"{max(labels):.1f}px"
                corner = data["cornerTag"] or "none"
                hit = "canvas" if data["cornerInCanvas"] else "clear"
                keys_top = (
                    "n/a"
                    if data["keysTop"] is None
                    else f"{data['keysTop']:.0f}px"
                )
                canvas_top = (
                    "n/a"
                    if data["canvasTop"] is None
                    else f"{data['canvasTop']:.0f}px"
                )
                keys_bottom = (
                    "n/a"
                    if data["keysBottom"] is None
                    else f"{data['keysBottom']:.0f}px"
                )
                first_row = (
                    "n/a"
                    if data["firstNodeTop"] is None
                    else f"{data['firstNodeTop']:.0f}px"
                )
                print(
                    f"preflight {state:<14} canvas {canvas}  svg {height}  "
                    f"name>={format_min(data['minName'])} "
                    f"(floor {RENDERED_FLOORS['node name']}px)  "
                    f"meta>={format_min(data['minMeta'])} "
                    f"(floor {RENDERED_FLOORS['node meta']}px)  "
                    f"badge>={format_min(data['minBadge'])} "
                    f"(floor {RENDERED_FLOORS['node badge']}px)  "
                    f"column>={format_min(data['minColumn'])} "
                    f"(floor {RENDERED_FLOORS['column header']}px)  "
                    f"aggregate>={format_min(data['minAggregate'])} "
                    f"(floor {RENDERED_FLOORS['aggregate']}px)  "
                    f"hull>={format_min(data['minHull'])} "
                    f"(floor {RENDERED_FLOORS['hull label']}px)  "
                    f"edge-label<={max_label} (max 32px)  "
                    f"canvas-top {canvas_top} keys {keys_top}-{keys_bottom} "
                    f"first-row {first_row}  "
                    f"mouse(1,1)->{corner} ({hit})",
                    flush=True,
                )
                failures.extend(f"{state}: {error}" for error in errors)
        finally:
            browser.close()

    names = [item["minName"] for item in measured if item["minName"] is not None]
    badges = [item["minBadge"] for item in measured if item["minBadge"] is not None]
    metas = [item["minMeta"] for item in measured if item["minMeta"] is not None]
    columns = [item["minColumn"] for item in measured if item["minColumn"] is not None]
    aggregates = [
        item["minAggregate"] for item in measured if item["minAggregate"] is not None
    ]
    hulls = [item["minHull"] for item in measured if item["minHull"] is not None]
    print(
        "measured minima (rendered, 1440x900): "
        f"name {format_min(min(names) if names else None)} "
        f"(floor {RENDERED_FLOORS['node name']}px) · "
        f"meta {format_min(min(metas) if metas else None)} "
        f"(floor {RENDERED_FLOORS['node meta']}px) · "
        f"badge {format_min(min(badges) if badges else None)} "
        f"(floor {RENDERED_FLOORS['node badge']}px) · "
        f"column {format_min(min(columns) if columns else None)} "
        f"(floor {RENDERED_FLOORS['column header']}px) · "
        f"aggregate {format_min(min(aggregates) if aggregates else None)} "
        f"(floor {RENDERED_FLOORS['aggregate']}px) · "
        f"hull {format_min(min(hulls) if hulls else None)} "
        f"(floor {RENDERED_FLOORS['hull label']}px)",
        flush=True,
    )
    if failures:
        print("\nDOM preflight FAILED; no screenshots written:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    records = []
    with sync_playwright() as pw:
        browser = launch_browser(pw)
        try:
            for state in STATES:
                html_path = HTML_DIR / f"{state}.html"
                final_path = ROOT / f"{state}-final.png"
                strip_path = ROOT / f"{state}-weight-strip.png"

                context = browser.new_context(
                    viewport={"width": W, "height": H},
                    device_scale_factor=1,
                )
                try:
                    page = context.new_page()
                    page.emulate_media(reduced_motion="reduce")
                    page.goto(html_path.resolve().as_uri())
                    page.wait_for_selector("#map")
                    page.wait_for_selector("#atlas-weight")
                    page.wait_for_timeout(150)

                    # Park the pointer away from nodes and reset the scroll so
                    # Chromium's native <title> tooltip cannot appear in the
                    # final frame.
                    page.mouse.move(1, 1)
                    page.evaluate("window.scrollTo(0, 0)")
                    page.wait_for_timeout(100)

                    page.screenshot(
                        path=final_path,
                        full_page=False,
                        animations="disabled",
                    )

                    locator = page.locator("#atlas-weight")
                    locator.scroll_into_view_if_needed()
                    page.mouse.move(1, 1)
                    page.wait_for_timeout(50)
                    locator.screenshot(path=strip_path, animations="disabled")
                finally:
                    context.close()

                final_w, final_h = png_dimensions(final_path)
                if (final_w, final_h) != (W, H):
                    raise SystemExit(
                        f"{state}: final frame is {final_w}x{final_h}, expected {W}x{H}"
                    )
                strip_w, strip_h = png_dimensions(strip_path)

                records.append(
                    {
                        "state": state,
                        "html": f"html/{html_path.name}",
                        "html_sha256": sha256_file(html_path),
                        "final": {
                            "file": final_path.name,
                            "width": final_w,
                            "height": final_h,
                            "sha256": sha256_file(final_path),
                        },
                        "strip": {
                            "file": strip_path.name,
                            "width": strip_w,
                            "height": strip_h,
                            "sha256": sha256_file(strip_path),
                        },
                    }
                )
                print(
                    f"{state:<14} final {final_w}x{final_h}  strip {strip_w}x{strip_h}",
                    flush=True,
                )
        finally:
            browser.close()

    manifest = {
        "_comment": (
            "Synthetic 8-state fixture matrix; no holdout read."
        ),
        "protocol": (
            "Two blind reads per state on the 1440x900 final frame; one image per "
            "task; no source/siblings/expected answer; final frame captured with "
            "prefers-reduced-motion + animations disabled."
        ),
        "viewport": {"width": W, "height": H},
        "device_scale_factor": 1,
        "animation": "CSS/SMIL declarative only; final frame; zero script added by Atlas",
        "holdout": "django/postgres and the memchr A2 payload deliberately absent",
        "states": records,
    }
    manifest_path = ROOT / "manifest.json"
    with manifest_path.open("w", encoding="utf-8") as handle:
        json.dump(manifest, handle, indent=2)
        handle.write("\n")

    print(f"\nmanifest: {manifest_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
