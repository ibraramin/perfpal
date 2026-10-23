#!/usr/bin/env python3
"""Atlas v2 step 10 evidence: render the real report.html#map and measure fit.

Adapts `docs/evidence/atlas-audit/audit_render.py`. Builds/uses the release
binary, scans perfpal-test/{hyperfine,cJSON} into fresh `out-atlas-v2-*` dirs,
opens `report.html#map` at 1440x900 with reduced motion, captures
`atlas-v2-{repo}-final.png` / `-full.png`, and writes `step10.json` with the
first-page geometry, chrome-hygiene counts and the acceptance booleans.

No source changes; evidence only.
"""

from __future__ import annotations

import hashlib
import json
import shutil
import subprocess
from pathlib import Path
from typing import Any

from playwright.sync_api import sync_playwright

REPO = Path(__file__).resolve().parents[4]
TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
OUT = Path(__file__).resolve().parent
BIN = REPO / "target" / "release" / "perfpal"
CARGO = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo")
REPOS = ["hyperfine", "cJSON"]
VIEWPORT = {"width": 1440, "height": 900}
VIEWPORT_1280 = {"width": 1280, "height": 800}
# Shipped Atlas v1 `#map` heights from docs/evidence/atlas-audit/audit.json.
AUDIT_BASELINE = {"hyperfine": 8982, "cJSON": 14685}
# The chip and the footer must agree on the global unreached totals.
UNREACHED_EXPECTED = {
    "hyperfine": {
        "chip": ["production unreached: 21", "non-production unreached: 11"],
        "footer": ["unreached production: 21", "non-production: 11"],
    },
    "cJSON": {
        "chip": ["production unreached: 4", "non-production unreached: 75"],
        "footer": ["unreached production: 4", "non-production: 75"],
    },
}
# Prefer the pinned playwright chromium, fall back to the bundled default.
PINNED_CHROME = Path(
    "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
)

DOM = """
() => {
  const rect = (sel) => {
    const e = document.querySelector(sel);
    if (!e) return null;
    const r = e.getBoundingClientRect();
    return {x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width),
            h: Math.round(r.height), bottom: Math.round(r.bottom)};
  };
  const map = document.querySelector('#map');
  const text = map ? map.textContent || '' : '';
  const count = (needle) => text.split(needle).length - 1;
  const minFont = (sel) => {
    const values = [...document.querySelectorAll(sel)]
      .map((e) => parseFloat(getComputedStyle(e).fontSize))
      .filter((n) => n > 0);
    return values.length ? Math.min(...values) : null;
  };
  const textOf = (sel) => document.querySelector(sel)?.textContent || null;
  let leadingDot = 0;
  if (map) {
    const walker = document.createTreeWalker(map, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const value = (walker.currentNode.nodeValue || '').trim();
      if (value.startsWith('…') || value.startsWith('..')) leadingDot += 1;
    }
  }
  const noClip = (sel) => [...document.querySelectorAll(sel)].every(
    (e) => e.scrollHeight <= e.clientHeight + 1 && e.scrollWidth <= e.clientWidth + 1
  );
  const chipsTitled = [...document.querySelectorAll('.atlas-chip')].every(
    (e) => ((e.getAttribute('title') || '').trim().length > 0)
  );
  const scrolling = document.scrollingElement || document.documentElement;
  const canvas = rect('#atlas-canvas');
  const brief = rect('#atlas-brief');
  const mapRect = rect('#map');
  return {
    map: mapRect,
    chrome: rect('.atlas-page-chrome'),
    masthead: rect('.atlas-masthead'),
    keymetrics: rect('.atlas-keymetrics'),
    canvas: canvas,
    graph: rect('.atlas-graph'),
    svg: rect('#atlas-canvas svg'),
    footer: rect('.atlas-footer'),
    brief: brief,
    body: {
      clientWidth: document.documentElement.clientWidth,
      scrollWidth: scrolling.scrollWidth,
      scrollHeight: scrolling.scrollHeight,
    },
    counts: {
      legend: document.querySelectorAll('.atlas-legend').length,
      footer: document.querySelectorAll('.atlas-footer').length,
      structuralCaveat: count('LOW name-based + MEDIUM semantic'),
      medianDefinition: count('× median function = all functions in the scan'),
      leadingDotTruncation: leadingDot,
      visibleEdgeLabels: document.querySelectorAll('#atlas-canvas .atlas-edge-label').length,
      edges: document.querySelectorAll('#atlas-canvas .atlas-edge').length,
      canvasPanels: document.querySelectorAll(
        '#atlas-canvas table, #atlas-canvas ul, #atlas-canvas li, #atlas-canvas select, '
        + '#atlas-canvas .card, #atlas-canvas .bar, #atlas-canvas [data-atlas-block]'
      ).length,
      reportScript: document.querySelectorAll('script').length,
      nodes: document.querySelectorAll('#atlas-canvas .atlas-node').length,
    },
    textFloors: {
      nodeName: minFont('#atlas-canvas .atlas-node-name'),
      nodeMeta: minFont('#atlas-canvas .atlas-node-meta'),
      columnHeader: minFont('#atlas-canvas .atlas-column-header'),
      nodeBadge: minFont('#atlas-canvas .atlas-node-badge'),
      strip: minFont('.atlas-keymetrics'),
      chip: minFont('.atlas-chip'),
      edgePill: minFont('#atlas-canvas .atlas-edge-label text'),
      entryValue: minFont('.atlas-entry-value'),
    },
    noClippedChrome: {
      keymetrics: noClip('.atlas-keymetrics'),
      footer: noClip('.atlas-footer'),
      chipTitles: chipsTitled,
    },
    sublineText: textOf('.atlas-subline'),
    unreachedChipText: textOf('#atlas-canvas .atlas-unreached-chip'),
    footerText: textOf('#atlas-canvas .atlas-footer'),
    firstScreen: {
      canvasBottom: canvas ? canvas.bottom : null,
      briefTop: brief ? brief.y : null,
      fold: 900,
    },
  };
}
"""


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def build_and_scan() -> None:
    subprocess.run(
        [CARGO, "build", "--release", "--locked"],
        cwd=REPO,
        check=True,
    )
    for repo in REPOS:
        source = TEST_ROOT / repo
        if not source.exists():
            raise SystemExit(f"FATAL: missing source repo {source}")
        out_dir = TEST_ROOT / f"out-atlas-v2-{repo}"
        subprocess.run(
            [str(BIN), str(source), "-o", str(out_dir)],
            check=True,
        )


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    build_and_scan()
    payload: dict[str, Any] = {
        "_comment": "Atlas v2 step 10 first-page fit + chrome hygiene; report.html#map, 1440x900 + 1280x800, reduced motion.",
        "viewport": VIEWPORT,
        "repos": {},
    }
    with sync_playwright() as playwright:
        if PINNED_CHROME.exists():
            browser = playwright.chromium.launch(executable_path=str(PINNED_CHROME))
        else:
            browser = playwright.chromium.launch()
        for repo in REPOS:
            report = TEST_ROOT / f"out-atlas-v2-{repo}" / "report.html"
            if not report.exists():
                raise SystemExit(f"FATAL: missing {report}")
            page = browser.new_page(viewport=VIEWPORT, device_scale_factor=1)
            page.emulate_media(reduced_motion="reduce")
            page.goto(report.as_uri() + "#map")
            page.wait_for_load_state("load")
            page.set_viewport_size(VIEWPORT)
            final = OUT / f"atlas-v2-{repo}-final.png"
            full = OUT / f"atlas-v2-{repo}-full.png"
            page.screenshot(path=str(final))
            page.screenshot(path=str(full), full_page=True)
            dom = page.evaluate(DOM)
            page.close()

            page1280 = browser.new_page(viewport=VIEWPORT_1280, device_scale_factor=1)
            page1280.emulate_media(reduced_motion="reduce")
            page1280.goto(report.as_uri() + "#map")
            page1280.wait_for_load_state("load")
            dom1280 = page1280.evaluate(DOM)
            page1280.close()

            baseline = AUDIT_BASELINE[repo]
            canvas_bottom = dom["canvas"]["bottom"]
            client_width = dom["body"]["clientWidth"]
            scroll_width = dom["body"]["scrollWidth"]
            brief_top = dom["brief"]["y"]
            map_height = dom["map"]["h"]
            floors = dom["textFloors"]
            floors_ok = all(
                [
                    (floors.get("nodeName") or 0) >= 10,
                    (floors.get("nodeMeta") or 0) >= 8.5,
                    (floors.get("columnHeader") or 0) >= 9,
                    (floors.get("nodeBadge") or 0) >= 8.5,
                    (floors.get("strip") or 0) >= 9,
                    (floors.get("chip") or 0) >= 8.5,
                    (floors.get("edgePill") or 0) >= 8.5,
                ]
            )
            no_clipped = all(dom["noClippedChrome"].values())
            viewport_1280 = {
                "clientWidth": dom1280["body"]["clientWidth"],
                "scrollWidth": dom1280["body"]["scrollWidth"],
                "canvasBottom": dom1280["canvas"]["bottom"],
                "noHorizontalScroll": dom1280["body"]["scrollWidth"]
                <= dom1280["body"]["clientWidth"] + 1,
            }
            text_checks: dict[str, bool] = {}
            expected = UNREACHED_EXPECTED.get(repo)
            if expected:
                chip = dom.get("unreachedChipText") or ""
                footer = dom.get("footerText") or ""
                for needle in expected["chip"]:
                    text_checks[f"chip contains {needle}"] = needle in chip
                for needle in expected["footer"]:
                    text_checks[f"footer contains {needle}"] = needle in footer
            booleans = {
                "canvasBottom_le_900": canvas_bottom <= 900,
                "noHorizontalScroll": scroll_width <= client_width + 1,
                "briefTop_ge_canvasBottom": brief_top >= canvas_bottom,
                "mapHeight_le_auditBaseline": map_height <= baseline,
                "noClippedChrome": no_clipped,
                "textFloors_ok": floors_ok,
                "noHorizontalScroll_1280": viewport_1280["noHorizontalScroll"],
            }
            failed = [key for key, value in booleans.items() if not value]
            failed.extend(key for key, value in text_checks.items() if not value)
            if failed:
                raise SystemExit(f"FATAL: {repo} checks failed: {failed}")
            payload["repos"][repo] = {
                "report": str(report),
                "report_sha256": sha256(report),
                "final_png": final.name,
                "final_png_sha256": sha256(final),
                "full_png": full.name,
                "full_png_sha256": sha256(full),
                "dom": dom,
                "viewport_1280": viewport_1280,
                "audit_baseline_map_height": baseline,
                "text_checks": text_checks,
                "booleans": booleans,
            }
        browser.close()

    text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    (OUT / "step10.json").write_text(text, encoding="utf-8")
    print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
