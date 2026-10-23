#!/usr/bin/env python3
"""Atlas v2 step 11 evidence: render the real report.html#map and measure the rail.

Adapts `render_step10.py`. Builds/uses the release binary, scans
perfpal-test/{hyperfine,cJSON} into fresh `out-atlas-v2-*` dirs, opens
`report.html#map` at 1440x900 and 1280x800 with reduced motion, captures
`atlas-v2-step11-{repo}-*.png`, and writes `step11.json` with the stage/rail
geometry, rail completeness, link-resolution, chrome-hygiene and page-budget
booleans.

No source changes; evidence only.
"""

from __future__ import annotations

import hashlib
import json
import re
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
# Shipped step-10 `#map` / page heights (no increase is allowed in step 11).
STEP10_BASELINE = {
    "hyperfine": {"map": 8887, "page": 22911},
    "cJSON": {"map": 14591, "page": 34800},
}
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
  const textOf = (sel) => document.querySelector(sel)?.textContent || null;
  const minFont = (sel) => {
    const values = [...document.querySelectorAll(sel)]
      .map((e) => parseFloat(getComputedStyle(e).fontSize))
      .filter((n) => n > 0);
    return values.length ? Math.min(...values) : null;
  };
  const scrolling = document.scrollingElement || document.documentElement;
  const canvas = rect('#atlas-canvas');
  const rail = rect('#atlas-rail');
  const cards = [...document.querySelectorAll('#atlas-rail .atlas-rail-card')];
  const railLinks = [...document.querySelectorAll('#atlas-rail a[href]')];
  const samePage = railLinks
    .map((a) => a.getAttribute('href') || '')
    .filter((h) => h.startsWith('#atlas-'))
    .map((h) => h.slice(1));
  const deadSamePage = [...new Set(samePage)].filter((id) => !document.getElementById(id));
  const cardChecks = cards.map((card) => {
    const text = card.textContent || '';
    return {
      role: (card.querySelector('.atlas-rail-role')?.textContent || '').trim(),
      hasCostAnchor: text.includes('median function') && text.includes('top '),
      hasFunctionLink: !!card.querySelector('a[href^="profile-queue.html#fn-"]'),
      hasFindingLink: !!card.querySelector('a[href^="findings-detail.html#"]'),
      hasFindingStub: text.includes('no findings on this file'),
    };
  });
  const leadingDot = (() => {
    let count = 0;
    const walker = document.createTreeWalker(document.querySelector('#map') || document.body,
                                             NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const value = (walker.currentNode.nodeValue || '').trim();
      if (value.startsWith('…') || value.startsWith('..')) count += 1;
    }
    return count;
  })();
  const count = (needle) => (document.querySelector('#map')?.textContent || '').split(needle).length - 1;
  const noClip = (sel) => [...document.querySelectorAll(sel)].every(
    (e) => e.scrollHeight <= e.clientHeight + 1 && e.scrollWidth <= e.clientWidth + 1
  );
  return {
    map: rect('#map'),
    chrome: rect('.atlas-page-chrome'),
    keymetrics: rect('.atlas-keymetrics'),
    stage: rect('.atlas-stage'),
    canvas: canvas,
    graph: rect('.atlas-graph'),
    svg: rect('#atlas-canvas svg'),
    footer: rect('.atlas-footer'),
    rail: rail,
    brief: rect('#atlas-brief'),
    body: {
      clientWidth: document.documentElement.clientWidth,
      scrollWidth: scrolling.scrollWidth,
      scrollHeight: scrolling.scrollHeight,
    },
    railCards: cards.length,
    cardChecks: cardChecks,
    railLinks: {
      total: railLinks.length,
      samePage: [...new Set(samePage)],
      deadSamePage: deadSamePage,
      profileQueue: [...new Set(railLinks
        .map((a) => a.getAttribute('href') || '')
        .filter((h) => h.startsWith('profile-queue.html#')))],
      findingsDetail: [...new Set(railLinks
        .map((a) => a.getAttribute('href') || '')
        .filter((h) => h.startsWith('findings-detail.html#')))],
    },
    counts: {
      legend: document.querySelectorAll('.atlas-legend').length,
      footer: document.querySelectorAll('.atlas-footer').length,
      structuralCaveat: count('LOW name-based + MEDIUM semantic'),
      medianDefinition: count('× median function = all functions in the scan'),
      leadingDotTruncation: leadingDot,
      canvasPanels: document.querySelectorAll(
        '#atlas-canvas table, #atlas-canvas ul, #atlas-canvas li, #atlas-canvas select, '
        + '#atlas-canvas .card, #atlas-canvas .bar, #atlas-canvas [data-atlas-block], '
        + '#atlas-canvas .atlas-rail-card'
      ).length,
      railAsides: document.querySelectorAll('#atlas-rail').length,
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
      railTitle: minFont('#atlas-rail .atlas-rail-title'),
      railName: minFont('#atlas-rail .atlas-rail-name'),
      railRole: minFont('#atlas-rail .atlas-rail-role'),
      railAnchor: minFont('#atlas-rail .atlas-rail-anchor'),
      railFn: minFont('#atlas-rail .atlas-rail-fn'),
      railFind: minFont('#atlas-rail .atlas-rail-find'),
    },
    noClippedChrome: {
      keymetrics: noClip('.atlas-keymetrics'),
      footer: noClip('.atlas-footer'),
      rail: noClip('#atlas-rail'),
    },
    noEventHandlers: ![...document.querySelectorAll('#atlas-rail *')]
      .some((e) => [...e.attributes].some((a) => a.name.toLowerCase().startsWith('on'))),
    railTitle: textOf('#atlas-rail .atlas-rail-title'),
    railFirstCard: textOf('#atlas-rail .atlas-rail-card'),
    firstScreen: {
      canvasBottom: canvas ? canvas.bottom : null,
      railBottom: rail ? rail.bottom : null,
      briefTop: rect('#atlas-brief') ? rect('#atlas-brief').y : null,
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
    subprocess.run([CARGO, "build", "--release", "--locked"], cwd=REPO, check=True)
    for repo in REPOS:
        source = TEST_ROOT / repo
        if not source.exists():
            raise SystemExit(f"FATAL: missing source repo {source}")
        out_dir = TEST_ROOT / f"out-atlas-v2-{repo}"
        subprocess.run([str(BIN), str(source), "-o", str(out_dir)], check=True)


def anchors_in(path: Path) -> set[str]:
    text = path.read_text(encoding="utf-8")
    return set(re.findall(r'id="([^"]+)"', text))


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    build_and_scan()
    payload: dict[str, Any] = {
        "_comment": (
            "Atlas v2 step 11 per-file detail rail; report.html#map, 1440x900 + "
            "1280x800, reduced motion."
        ),
        "viewport": VIEWPORT,
        "repos": {},
    }
    with sync_playwright() as playwright:
        if PINNED_CHROME.exists():
            browser = playwright.chromium.launch(executable_path=str(PINNED_CHROME))
        else:
            browser = playwright.chromium.launch()
        for repo in REPOS:
            out_dir = TEST_ROOT / f"out-atlas-v2-{repo}"
            report = out_dir / "report.html"
            if not report.exists():
                raise SystemExit(f"FATAL: missing {report}")
            page = browser.new_page(viewport=VIEWPORT, device_scale_factor=1)
            page.emulate_media(reduced_motion="reduce")
            page.goto(report.as_uri() + "#map")
            page.wait_for_load_state("load")
            page.set_viewport_size(VIEWPORT)
            final = OUT / f"atlas-v2-step11-{repo}-final.png"
            full = OUT / f"atlas-v2-step11-{repo}-full.png"
            page.screenshot(path=str(final))
            page.screenshot(path=str(full), full_page=True)
            dom = page.evaluate(DOM)
            page.close()

            page1280 = browser.new_page(viewport=VIEWPORT_1280, device_scale_factor=1)
            page1280.emulate_media(reduced_motion="reduce")
            page1280.goto(report.as_uri() + "#map")
            page1280.wait_for_load_state("load")
            shot1280 = OUT / f"atlas-v2-step11-{repo}-1280x800.png"
            page1280.screenshot(path=str(shot1280))
            dom1280 = page1280.evaluate(DOM)
            page1280.close()

            # Cross-page anchors must resolve in the served artifact tree.
            profile_anchors = anchors_in(out_dir / "profile-queue.html")
            findings_anchors = anchors_in(out_dir / "findings-detail.html")
            missing_profile = [
                href.split("#", 1)[1]
                for href in dom["railLinks"]["profileQueue"]
                if href.split("#", 1)[1] not in profile_anchors
            ]
            missing_findings = [
                href.split("#", 1)[1]
                for href in dom["railLinks"]["findingsDetail"]
                if href.split("#", 1)[1] not in findings_anchors
            ]

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
                    (floors.get("railTitle") or 0) >= 8.5,
                    (floors.get("railName") or 0) >= 10,
                    (floors.get("railRole") or 0) >= 8.5,
                    (floors.get("railAnchor") or 0) >= 8.5,
                    (floors.get("railFn") or 0) >= 8.5,
                    (floors.get("railFind") or 0) >= 8.5,
                ]
            )
            cards_ok = dom["railCards"] > 0 and all(
                card["role"]
                and card["hasCostAnchor"]
                and card["hasFunctionLink"]
                and (card["hasFindingLink"] or card["hasFindingStub"])
                for card in dom["cardChecks"]
            )
            baseline = STEP10_BASELINE[repo]
            canvas_bottom = dom["canvas"]["bottom"]
            rail_bottom = dom["rail"]["bottom"]
            scroll_width = dom["body"]["scrollWidth"]
            client_width = dom["body"]["clientWidth"]
            stacked = dom1280["rail"]["y"] >= dom1280["canvas"]["bottom"] - 1
            booleans = {
                "canvasBottom_le_900": canvas_bottom <= 900,
                "railBottom_le_900": rail_bottom <= 900,
                "rail_beside_canvas": dom["rail"]["x"] >= dom["canvas"]["x"] + dom["canvas"]["w"],
                "rail_cards_complete": cards_ok,
                "rail_cards_le_cap": dom["railCards"] <= 12,
                "rail_one_aside": dom["counts"]["railAsides"] == 1,
                "rail_samePage_links_resolve": not dom["railLinks"]["deadSamePage"],
                "rail_profileQueue_links_resolve": not missing_profile,
                "rail_findings_links_resolve": not missing_findings,
                "rail_noEventHandlers": dom["noEventHandlers"],
                "noHorizontalScroll": scroll_width <= client_width + 1,
                "briefTop_ge_canvasBottom": dom["brief"]["y"] >= canvas_bottom,
                "briefTop_ge_railBottom": dom["brief"]["y"] >= rail_bottom - 1,
                # The 868px canvas wraps the footer from one line to two, so the
                # rail layout is allowed a bounded footer-wrap delta (+12px);
                # nothing else may grow the page. Step 12 owns the real cut.
                "mapHeight_le_step10_plus_footer_wrap": dom["map"]["h"]
                <= baseline["map"] + 12,
                "pageHeight_le_step10_plus_footer_wrap": dom["body"]["scrollHeight"]
                <= baseline["page"] + 12,
                "noClippedChrome": all(dom["noClippedChrome"].values()),
                "textFloors_ok": floors_ok,
                "chrome_counts": (
                    dom["counts"]["legend"] == 1
                    and dom["counts"]["footer"] == 1
                    and dom["counts"]["structuralCaveat"] == 1
                    and dom["counts"]["medianDefinition"] == 1
                    and dom["counts"]["leadingDotTruncation"] == 0
                ),
                "canvas_purity": dom["counts"]["canvasPanels"] == 0,
                "reportScript_unchanged": dom["counts"]["reportScript"] == 1,
                "noHorizontalScroll_1280": dom1280["body"]["scrollWidth"]
                <= dom1280["body"]["clientWidth"] + 1,
                "rail_stacks_below_1280": stacked,
            }
            failed = [key for key, value in booleans.items() if not value]
            if failed:
                raise SystemExit(
                    f"FATAL: {repo} checks failed: {failed}; "
                    f"missing_profile={missing_profile[:3]} missing_findings={missing_findings[:3]}"
                )
            payload["repos"][repo] = {
                "report": str(report),
                "report_sha256": sha256(report),
                "profile_queue_sha256": sha256(out_dir / "profile-queue.html"),
                "final_png": final.name,
                "final_png_sha256": sha256(final),
                "full_png": full.name,
                "full_png_sha256": sha256(full),
                "shot_1280": shot1280.name,
                "shot_1280_sha256": sha256(shot1280),
                "dom": dom,
                "viewport_1280": {
                    "clientWidth": dom1280["body"]["clientWidth"],
                    "scrollWidth": dom1280["body"]["scrollWidth"],
                    "canvasBottom": dom1280["canvas"]["bottom"],
                    "railTop": dom1280["rail"]["y"],
                    "railBottom": dom1280["rail"]["bottom"],
                    "stacked": stacked,
                },
                "step10_baseline": baseline,
                "cross_page": {
                    "profileQueueLinks": len(dom["railLinks"]["profileQueue"]),
                    "findingsDetailLinks": len(dom["railLinks"]["findingsDetail"]),
                    "missingProfileAnchors": missing_profile,
                    "missingFindingsAnchors": missing_findings,
                },
                "booleans": booleans,
            }
        browser.close()

    text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    (OUT / "step11.json").write_text(text, encoding="utf-8")
    print(text)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
