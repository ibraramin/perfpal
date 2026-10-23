#!/usr/bin/env python3
"""Atlas visual-regression audit: render the real report.html#map and measure the layout.

Reads the current release binary's output for hyperfine/cJSON (out-atlas-*), captures the
first viewport + full page at 1440x900 with reduced motion, and records the DOM geometry,
typography and canvas density. No source changes; audit evidence only.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

from playwright.sync_api import sync_playwright

EXE = "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
ROOT = Path("/home/shafin/Opencode/perfpal-test")
OUT = Path(__file__).resolve().parent
REPOS = ["hyperfine", "cJSON"]
VIEWPORT = {"width": 1440, "height": 900}

DOM = """
() => {
  const rect = (sel) => {
    const e = document.querySelector(sel);
    if (!e) return null;
    const r = e.getBoundingClientRect();
    return {x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height), bottom: Math.round(r.bottom)};
  };
  const style = (sel) => {
    const e = document.querySelector(sel);
    if (!e) return null;
    const s = getComputedStyle(e);
    return {fontFamily: s.fontFamily, fontSize: s.fontSize, background: s.backgroundColor, color: s.color};
  };
  const canvas = document.querySelector('#atlas-canvas');
  const small = canvas ? [...canvas.querySelectorAll('*')].filter((e) => {
    const s = getComputedStyle(e);
    const fs = parseFloat(s.fontSize);
    return fs > 0 && fs < 10;
  }).length : null;
  const svg = document.querySelector('#atlas-canvas svg');
  return {
    body: {scrollW: document.body.scrollWidth, scrollH: document.body.scrollHeight,
           clientW: document.documentElement.clientWidth, clientH: document.documentElement.clientHeight},
    canvas: rect('#atlas-canvas'),
    brief: rect('#atlas-brief'),
    map: rect('#map'),
    header: rect('header'),
    nav: rect('nav'),
    canvasStyle: style('#atlas-canvas'),
    bodyStyle: style('body'),
    nodes: document.querySelectorAll('#atlas-canvas .atlas-node').length,
    edges: document.querySelectorAll('#atlas-canvas .atlas-edge').length,
    edgeLabels: document.querySelectorAll('#atlas-canvas .atlas-edge-label').length,
    columns: document.querySelectorAll('#atlas-canvas .atlas-column').length,
    nodesInFirstFold: [...document.querySelectorAll('#atlas-canvas .atlas-node')]
      .filter((e) => e.getBoundingClientRect().bottom <= 900).length,
    smallCanvasTextElements: small,
    svg: svg ? {viewBox: svg.getAttribute('viewBox'),
                rect: (() => { const r = svg.getBoundingClientRect(); return {w: Math.round(r.width), h: Math.round(r.height)}; })()} : null,
    firstScreen: {canvasBottom: rect('#atlas-canvas')?.bottom, briefTop: rect('#atlas-brief')?.y, fold: 900},
  };
}
"""


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    audit: dict[str, object] = {}
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=EXE)
        for repo in REPOS:
            report = ROOT / f"out-atlas-{repo}" / "report.html"
            if not report.exists():
                raise SystemExit(f"FATAL: missing {report}")
            page = browser.new_page(viewport={"width": 1440, "height": 900}, device_scale_factor=1)
            page.emulate_media(reduced_motion="reduce")
            page.goto(report.as_uri() + "#map")
            page.wait_for_load_state("load")
            page.set_viewport_size({"width": 1440, "height": 900})
            first = OUT / f"atlas-{repo}-final.png"
            full = OUT / f"atlas-{repo}-full.png"
            page.screenshot(path=str(first))
            page.screenshot(path=str(full), full_page=True)
            dom = page.evaluate(DOM)
            audit[repo] = {
                "report": str(report),
                "report_sha256": sha256(report),
                "first_png": first.name,
                "first_png_sha256": sha256(first),
                "full_png": full.name,
                "full_png_sha256": sha256(full),
                "dom": dom,
            }
            page.close()
        browser.close()
    payload = {
        "_comment": "Atlas visual-regression audit of the real report.html#map; 1440x900, reduced motion.",
        "repos": audit,
        "viewport": VIEWPORT,
    }
    (OUT / "audit.json").write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(payload, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
