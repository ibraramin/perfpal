#!/usr/bin/env python3
"""Atlas v2 step-12 evidence harness — measured, not claimed.

Adapts `docs/evidence/atlas-visual/v2/render_step11.py`. Uses the existing real
outputs only (no rescan, no build):

  /home/shafin/Opencode/perfpal-test/out-atlas-v2-hyperfine
  /home/shafin/Opencode/perfpal-test/out-atlas-v2-cJSON

For each repo it opens `report.html#map` at 1440x900 with reduced motion,
scrolls `#map` to the top of the viewport (the canonical Atlas capture; the
global report header sits above and is outside the 900px page), measures the
step-12 layout/charts/links/text floors, saves screenshots, hashes the three
requested HTML artifacts, and writes `step12.json`.

Screenshots (under this directory):
  atlas-v2-step12-<repo>-final.png    1440x900 viewport, #map at top
  atlas-v2-step12-<repo>-full.png     full page at 1440
  atlas-v2-step12-<repo>-charts.png   #atlas-charts element at 1440
  atlas-v2-step12-<repo>-1280x800.png 1280x800 viewport, #map at top
Comparison (under step12/):
  W1-vs-F-<repo>.png                  real live weight panel next to the real F candidate

This harness only reads existing artifacts; step 12 itself added
`src/reporter/atlas_charts.rs` and `tests/atlas_charts.rs`. No commit. No new
dependency; Pillow is used only if already available for the comparison image.
"""

from __future__ import annotations

import hashlib
import json
import re
import struct
from pathlib import Path
from typing import Any

from playwright.sync_api import sync_playwright

REPO = Path(__file__).resolve().parents[4]
TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
OUT = Path(__file__).resolve().parent
F_DIR = OUT / "step12"
REPOS = ["hyperfine", "cJSON"]
VIEWPORT = {"width": 1440, "height": 900}
VIEWPORT_1280 = {"width": 1280, "height": 800}
PINNED_CHROME = Path(
    "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
)
EXPECTED_BLOCKS = ["weight-chart", "blast-chart", "file-chart", "coverage"]
EXPECTED_ATLAS_LINKS = {
    "report.md",
    "metrics.json",
    "atlas-weight.html",
    "atlas-q2.html",
    "findings-detail.html#ranked-actions",
    "profile-queue.html",
}
FORBIDDEN_BLOCKS = {"answer", "actions", "cards", "noise", "links"}
TEXT_FLOOR = 8.5
TARGET_MAP_HEIGHT = 5000

# DOM measurements are taken after #map is scrolled to the viewport top. All
# rects are viewport-relative; the global report header is deliberately above
# the canonical Atlas capture.
DOM = r"""
() => {
  const rect = (sel) => {
    const el = document.querySelector(sel);
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return {x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width),
            h: Math.round(r.height), bottom: Math.round(r.bottom)};
  };
  const map = document.querySelector('#map');
  const chart = document.querySelector('#atlas-charts');
  const chartBlocks = chart ? [...chart.querySelectorAll('[data-atlas-block]')] : [];
  const blocks = chartBlocks.map((el) => {
    const r = el.getBoundingClientRect();
    return {block: el.getAttribute('data-atlas-block'),
            height: Math.round(r.height), top: Math.round(r.top)};
  });
  const weight = chart ? chart.querySelector('[data-atlas-block="weight-chart"]') : null;
  const weightRows = weight ? [...weight.querySelectorAll('.atlas-bar-row')].map((row) => {
    const fill = row.querySelector('.atlas-bar-fill');
    const label = row.querySelector('.atlas-bar-label');
    return {
      label: (label?.textContent || '').trim(),
      width: fill ? Math.round(fill.getBoundingClientRect().width * 10) / 10 : null,
      nonproduction: !!(fill && fill.classList.contains('is-nonprod')),
      style: fill ? fill.getAttribute('style') : null,
    };
  }) : [];
  const weightBars = weight
    ? [...weight.querySelectorAll('.atlas-bar-fill')]
        .map((fill) => Math.round(fill.getBoundingClientRect().width * 10) / 10)
    : [];
  const fileChart = chart ? chart.querySelector('[data-atlas-block="file-chart"]') : null;
  const fileText = fileChart
    ? (fileChart.textContent || '').replace(/\s+/g, ' ').trim()
    : '';
  const visibleLeaves = [];
  if (chart) {
    const walker = document.createTreeWalker(chart, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const text = (walker.currentNode.nodeValue || '').trim();
      if (!text) continue;
      const el = walker.currentNode.parentElement;
      if (!el) continue;
      const tag = el.tagName;
      if (tag === 'SCRIPT' || tag === 'STYLE' || tag === 'NOSCRIPT') continue;
      const cs = getComputedStyle(el);
      if (cs.display === 'none' || cs.visibility === 'hidden'
          || cs.visibility === 'collapse' || parseFloat(cs.opacity) === 0) continue;
      if (el.getClientRects().length === 0) continue;
      visibleLeaves.push({
        text: text.slice(0, 48),
        size: parseFloat(cs.fontSize),
        tag,
        cls: String(el.className?.baseVal ?? el.className ?? ''),
      });
    }
  }
  visibleLeaves.sort((a, b) => a.size - b.size);
  const railLinks = [...document.querySelectorAll('#atlas-rail a[href]')]
    .map((a) => a.getAttribute('href')).filter(Boolean);
  const atlasLinks = [...document.querySelectorAll('.atlas-links a[href]')]
    .map((a) => a.getAttribute('href')).filter(Boolean);
  const mapBlocks = map
    ? [...map.querySelectorAll('[data-atlas-block]')]
        .map((el) => el.getAttribute('data-atlas-block'))
    : [];
  const page = document.scrollingElement || document.documentElement;
  return {
    map: rect('#map'),
    canvas: rect('#atlas-canvas'),
    chart: rect('#atlas-charts'),
    brief: rect('#atlas-brief'),
    rail: rect('#atlas-rail'),
    page: {
      clientWidth: document.documentElement.clientWidth,
      scrollWidth: page.scrollWidth,
      scrollHeight: page.scrollHeight,
      scrollY: Math.round(window.scrollY),
    },
    blocks,
    blockOrder: blocks.map((item) => item.block),
    chartHeights: Object.fromEntries(blocks.map((item) => [item.block, item.height])),
    chartTotalHeight: rect('#atlas-charts')?.h ?? null,
    counts: {
      atlasCharts: document.querySelectorAll('#atlas-charts').length,
      chartTables: chart ? chart.querySelectorAll('table').length : -1,
      chartScripts: chart ? chart.querySelectorAll('script').length : -1,
      structuralCaptions: document.querySelectorAll('.atlas-structural-caption').length,
      units: document.querySelectorAll('.atlas-unit').length,
      atlasLinks: document.querySelectorAll('.atlas-links').length,
      railCards: document.querySelectorAll('#atlas-rail .atlas-rail-card').length,
      answerInMap: map ? map.querySelectorAll('#atlas-answer').length : -1,
      drillInMap: map ? map.querySelectorAll('[id^="atlas-drill-"]').length : -1,
      forbiddenBlocksInMap: mapBlocks.filter((value) => FORBIDDEN_BLOCKS_JS.includes(value)).length,
    },
    mapBlocks,
    atlasLinks: [...new Set(atlasLinks)],
    railLinks: [...new Set(railLinks)],
    textLeaves: {
      count: visibleLeaves.length,
      min: visibleLeaves.length ? visibleLeaves[0].size : null,
      offenders: visibleLeaves.filter((item) => item.size < 8.5).slice(0, 12),
    },
    weightBars: {
      count: weightBars.length,
      max: weightBars.length ? Math.max(...weightBars) : 0,
      positive: weightBars.filter((width) => width > 0).length,
      minPositive: weightBars.filter((width) => width > 0).length
        ? Math.min(...weightBars.filter((width) => width > 0))
        : 0,
      rows: weightRows,
    },
    fileChart: {
      hasFileTotal: fileText.includes('file total'),
      hasUnit: fileText.includes('static cost units (v2)'),
      text: fileText.slice(0, 220),
    },
  };
}
""".replace(
    "FORBIDDEN_BLOCKS_JS",
    json.dumps(sorted(FORBIDDEN_BLOCKS)),
)



def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def png_size(path: Path) -> tuple[int, int]:
    head = path.read_bytes()[:24]
    if head[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"FATAL: {path} is not a PNG")
    return struct.unpack(">II", head[16:24])


def anchors_in(path: Path) -> set[str]:
    text = path.read_text(encoding="utf-8")
    return set(re.findall(r'id="([^"]+)"', text))


def scroll_map_to_top(page) -> None:
    page.evaluate(
        "() => { const map = document.querySelector('#map');"
        " if (map) map.scrollIntoView({block: 'start'}); }"
    )
    page.evaluate("() => document.fonts.ready.then(() => true)")
    page.wait_for_timeout(80)


def collect_1440(browser, repo: str, out_dir: Path) -> tuple[dict, dict[str, Path]]:
    report = out_dir / "report.html"
    if not report.exists():
        raise SystemExit(f"FATAL: missing {report}")
    page = browser.new_page(viewport=VIEWPORT, device_scale_factor=1)
    page.emulate_media(reduced_motion="reduce")
    page.goto(report.as_uri() + "#map")
    page.wait_for_load_state("load")
    scroll_map_to_top(page)
    dom = page.evaluate(DOM)

    final = OUT / f"atlas-v2-step12-{repo}-final.png"
    full = OUT / f"atlas-v2-step12-{repo}-full.png"
    charts = OUT / f"atlas-v2-step12-{repo}-charts.png"
    page.screenshot(path=str(final))
    page.screenshot(path=str(full), full_page=True)
    page.locator("#atlas-charts").screenshot(path=str(charts))

    weight_tmp = OUT / f".atlas-v2-step12-{repo}-weight.png"
    page.locator('#atlas-charts [data-atlas-block="weight-chart"]').screenshot(
        path=str(weight_tmp)
    )
    page.close()
    return dom, {"final": final, "full": full, "charts": charts, "weight": weight_tmp}


def collect_1280(browser, repo: str, out_dir: Path) -> tuple[dict, Path]:
    report = out_dir / "report.html"
    page = browser.new_page(viewport=VIEWPORT_1280, device_scale_factor=1)
    page.emulate_media(reduced_motion="reduce")
    page.goto(report.as_uri() + "#map")
    page.wait_for_load_state("load")
    scroll_map_to_top(page)
    dom = page.evaluate(DOM)
    shot = OUT / f"atlas-v2-step12-{repo}-1280x800.png"
    page.screenshot(path=str(shot))
    page.close()
    return dom, shot


def make_comparison(repo: str, weight_png: Path, f_png: Path) -> dict[str, Any]:
    """Put the real live weight panel next to the real F candidate.

    This is a labelled evidence composite, not synthetic chart content. Pillow
    is optional, but a missing Pillow/source image fails the run via
    `w1_vs_f_written` (the comparison is part of the evidence contract).
    """
    try:
        from PIL import Image, ImageDraw, ImageFont
    except Exception as exc:  # pragma: no cover - environment-dependent
        return {"status": "skipped", "reason": f"Pillow unavailable: {exc}"}
    if not weight_png.exists():
        return {"status": "skipped", "reason": f"missing weight panel: {weight_png.name}"}
    if not f_png.exists():
        return {"status": "skipped", "reason": f"missing F candidate: {f_png.name}"}

    def load_font(size: int, bold: bool = False):
        candidates = [
            Path(
                "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
                if bold
                else "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"
            ),
            Path(
                "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf"
                if bold
                else "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf"
            ),
        ]
        for candidate in candidates:
            if candidate.exists():
                return ImageFont.truetype(str(candidate), size)
        return ImageFont.load_default()

    target_w = 680
    margin = 40
    gutter = 40
    header_h = 88
    footer_h = 46
    paper = (244, 241, 234)
    ink = (32, 43, 53)
    muted = (111, 106, 97)
    accent = (194, 65, 12)
    hairline = (214, 208, 196)
    panel = (251, 249, 244)

    w1 = Image.open(weight_png).convert("RGB")
    f_img = Image.open(f_png).convert("RGB")

    def scale(image):
        height = max(1, round(image.height * target_w / image.width))
        return image.resize((target_w, height), Image.BICUBIC)

    w1s = scale(w1)
    fs = scale(f_img)
    body_h = max(w1s.height, fs.height)
    canvas_w = margin * 2 + target_w * 2 + gutter
    canvas_h = margin + header_h + body_h + footer_h + margin
    canvas = Image.new("RGB", (canvas_w, canvas_h), paper)
    draw = ImageDraw.Draw(canvas)
    title_font = load_font(19, bold=True)
    sub_font = load_font(11)
    footer_font = load_font(10)

    left_x = margin
    right_x = margin + target_w + gutter
    top = margin
    draw.text((left_x, top), "W1 · shipped weight panel", font=title_font, fill=ink)
    draw.text(
        (left_x, top + 26),
        "live report.html · production-first bars + non-production ribbon",
        font=sub_font,
        fill=muted,
    )
    draw.text((right_x, top), "F · candidate prototype, not shipped", font=title_font, fill=accent)
    draw.text(
        (right_x, top + 26),
        "260px whole-repo component treemap · production-first top-50",
        font=sub_font,
        fill=muted,
    )
    draw.line(
        [(left_x, top + 48), (right_x + target_w, top + 48)],
        fill=hairline,
        width=1,
    )

    image_top = margin + header_h
    canvas.paste(w1s, (left_x, image_top))
    canvas.paste(fs, (right_x, image_top))
    draw.rectangle(
        [left_x - 1, image_top - 1, left_x + target_w, image_top + w1s.height],
        outline=hairline,
        width=1,
    )
    draw.rectangle(
        [right_x - 1, image_top - 1, right_x + target_w, image_top + fs.height],
        outline=hairline,
        width=1,
    )
    draw.text(
        (left_x, image_top + w1s.height + 10),
        "Live W1 panel is one weight block inside the four-chart brief.",
        font=sub_font,
        fill=muted,
    )
    draw.text(
        (right_x, image_top + fs.height + 10),
        "F panel includes its own heading, caption, legend and status line.",
        font=sub_font,
        fill=muted,
    )
    footer = (
        "Step-12 evidence · same repo, 1440px captures · no founder decision · F remains undecided"
    )
    draw.line(
        [(left_x, canvas_h - footer_h), (canvas_w - margin, canvas_h - footer_h)],
        fill=hairline,
        width=1,
    )
    draw.text(
        (left_x, canvas_h - footer_h + 14),
        footer,
        font=footer_font,
        fill=muted,
    )

    out_path = F_DIR / f"W1-vs-F-{repo}.png"
    canvas.save(out_path)
    return {
        "status": "written",
        "file": out_path.name,
        "width": canvas.width,
        "height": canvas.height,
        "sources": [weight_png.name, f_png.name],
    }


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    F_DIR.mkdir(parents=True, exist_ok=True)
    payload: dict[str, Any] = {
        "_comment": (
            "Atlas v2 step-12 evidence: existing real outputs only, no rescan; "
            "canonical #map capture at 1440x900 + 1280x800; no founder gate claim; "
            "F remains a candidate."
        ),
        "renderer": "docs/evidence/atlas-visual/v2/render_step12.py",
        "viewport": {"1440x900": VIEWPORT, "1280x800": VIEWPORT_1280},
        "no_rescan": True,
        "repos": {},
    }
    anchor_cache: dict[Path, set[str]] = {}

    def anchors(path: Path) -> set[str]:
        path = path.resolve()
        if path not in anchor_cache:
            anchor_cache[path] = anchors_in(path)
        return anchor_cache[path]

    all_pass = True
    with sync_playwright() as playwright:
        if PINNED_CHROME.exists():
            browser = playwright.chromium.launch(executable_path=str(PINNED_CHROME))
        else:
            browser = playwright.chromium.launch()
        for repo in REPOS:
            out_dir = TEST_ROOT / f"out-atlas-v2-{repo}"
            if not out_dir.exists():
                raise SystemExit(f"FATAL: missing output dir {out_dir}")
            dom, shots = collect_1440(browser, repo, out_dir)
            dom1280, shot1280 = collect_1280(browser, repo, out_dir)
            report = out_dir / "report.html"
            atlas_weight = out_dir / "atlas-weight.html"
            atlas_q2 = out_dir / "atlas-q2.html"
            for required in (report, atlas_weight, atlas_q2):
                if not required.exists():
                    raise SystemExit(f"FATAL: missing {required}")

            # 6. .atlas-links files AND anchors resolve.
            atlas_links = set(dom["atlasLinks"])
            missing_atlas_links = []
            for href in atlas_links:
                target_name, _, target_anchor = href.partition("#")
                target_path = out_dir / target_name
                if not target_path.exists():
                    missing_atlas_links.append(href)
                    continue
                if target_anchor and target_anchor not in anchors(target_path):
                    missing_atlas_links.append(href)
            missing_atlas_links = sorted(missing_atlas_links)
            expected_missing = sorted(
                name
                for name in EXPECTED_ATLAS_LINKS
                if not (out_dir / name.split("#", 1)[0]).exists()
            )
            expected_present = EXPECTED_ATLAS_LINKS.issubset(atlas_links)

            # 7. Rail anchors resolve in their target pages.
            rail_links = dom["railLinks"]
            report_ids = anchors(report)
            missing_rail_links: list[dict[str, str]] = []
            rail_type_counts = {
                "atlas-weight.html#atlas-": 0,
                "profile-queue.html#fn-": 0,
                "findings-detail.html#finding-": 0,
                "other": 0,
            }
            for href in rail_links:
                if href.startswith("#"):
                    target_name = "report.html"
                    target_anchor = href[1:]
                    target_ids = report_ids
                elif "#" in href:
                    target_name, target_anchor = href.split("#", 1)
                    target_path = out_dir / target_name
                    if not target_path.exists():
                        missing_rail_links.append({"href": href, "reason": "missing page"})
                        continue
                    target_ids = anchors(target_path)
                else:
                    target_name = href
                    target_path = out_dir / target_name
                    if not target_path.exists():
                        missing_rail_links.append({"href": href, "reason": "missing file"})
                    continue
                if target_anchor not in target_ids:
                    missing_rail_links.append(
                        {"href": href, "reason": f"missing #{target_anchor} in {target_name}"}
                    )
                if href.startswith("atlas-weight.html#atlas-"):
                    rail_type_counts["atlas-weight.html#atlas-"] += 1
                elif href.startswith("profile-queue.html#fn-"):
                    rail_type_counts["profile-queue.html#fn-"] += 1
                elif href.startswith("findings-detail.html#finding-"):
                    rail_type_counts["findings-detail.html#finding-"] += 1
                else:
                    rail_type_counts["other"] += 1

            weight_bars = dom["weightBars"]
            production_rows = [
                row
                for row in weight_bars["rows"]
                if not row["nonproduction"] and row["width"] is not None and row["width"] > 0
            ]
            text_min = dom["textLeaves"]["min"]
            blocks = dom["blockOrder"]
            chart_heights = dom["chartHeights"]
            booleans = {
                "map_scrolled_to_top": abs(dom["map"]["y"]) <= 1,
                "map_height_le_5000": dom["map"]["h"] <= TARGET_MAP_HEIGHT,
                "canvas_bottom_le_900": dom["canvas"]["bottom"] <= 900,
                "brief_top_ge_canvas_bottom": dom["brief"]["y"] >= dom["canvas"]["bottom"],
                "noHorizontalScroll_1440": dom["page"]["scrollWidth"] <= 1440,
                "atlas_charts_once": dom["counts"]["atlasCharts"] == 1,
                "chart_blocks_exact": blocks == EXPECTED_BLOCKS,
                "chart_blocks_nonzero": bool(blocks)
                and all(chart_heights.get(block, 0) > 0 for block in EXPECTED_BLOCKS),
                "no_table_or_script_in_charts": dom["counts"]["chartTables"] == 0
                and dom["counts"]["chartScripts"] == 0,
                "no_forbidden_map_content": dom["counts"]["answerInMap"] == 0
                and dom["counts"]["drillInMap"] == 0
                and dom["counts"]["forbiddenBlocksInMap"] == 0,
                "structural_caption_once": dom["counts"]["structuralCaptions"] == 1,
                "unit_once": dom["counts"]["units"] == 1,
                "atlas_links_resolve": not missing_atlas_links
                and not expected_missing
                and expected_present,
                "rail_links_present": bool(rail_links),
                "rail_cards_present": dom["counts"]["railCards"] > 0,
                "rail_links_resolve": not missing_rail_links,
                "text_floor_ok": text_min is not None and text_min >= TEXT_FLOOR,
                "weight_bar_visible": weight_bars["max"] > 0,
                "production_weight_rows_visible": bool(production_rows),
                "file_chart_prints": dom["fileChart"]["hasFileTotal"]
                and dom["fileChart"]["hasUnit"],
                "noHorizontalScroll_1280": dom1280["page"]["scrollWidth"] <= 1280,
                "rail_stacks_below_1280": dom1280["rail"]["y"]
                >= dom1280["canvas"]["bottom"] - 1,
                "final_png_1440x900": png_size(shots["final"]) == (1440, 900),
                "shot_1280x800": png_size(shot1280) == (1280, 800),
            }
            failed = [key for key, value in booleans.items() if not value]
            all_pass = all_pass and not failed

            comparison = make_comparison(repo, shots["weight"], F_DIR / f"f-treemap-{repo}.png")
            shots["weight"].unlink(missing_ok=True)
            booleans["w1_vs_f_written"] = comparison["status"] == "written"
            if comparison["status"] != "written":
                failed.append("w1_vs_f_written")
                all_pass = False
                print(f"FAIL {repo}: W1-vs-F comparison not written — {comparison.get('reason', comparison['status'])}")

            screenshot_meta = {}
            for role, path in (
                ("final", shots["final"]),
                ("full", shots["full"]),
                ("charts", shots["charts"]),
                ("1280x800", shot1280),
            ):
                width, height = png_size(path)
                screenshot_meta[role] = {
                    "file": path.name,
                    "width": width,
                    "height": height,
                    "sha256": sha256(path),
                }

            payload["repos"][repo] = {
                "report": str(report),
                "hashes": {
                    "report.html": sha256(report),
                    "atlas-weight.html": sha256(atlas_weight),
                    "atlas-q2.html": sha256(atlas_q2),
                },
                "screenshots": screenshot_meta,
                "comparison": comparison,
                "measurements": {
                    "map": dom["map"],
                    "canvas": dom["canvas"],
                    "brief": dom["brief"],
                    "rail": dom["rail"],
                    "chart": dom["chart"],
                    "page": dom["page"],
                    "blocks": dom["blocks"],
                    "chartHeights": chart_heights,
                    "chartTotalHeight": dom["chartTotalHeight"],
                    "minChartTextPx": text_min,
                    "chartTextOffenders": dom["textLeaves"]["offenders"],
                    "weightBars": weight_bars,
                    "productionWeightRows": [
                        {"label": row["label"], "width": row["width"]}
                        for row in production_rows
                    ],
                    "fileChart": dom["fileChart"],
                    "counts": dom["counts"],
                    "mapBlocks": dom["mapBlocks"],
                },
                "map_height": dom["map"]["h"],
                "page_height": dom["page"]["scrollHeight"],
                "min_chart_text_px": text_min,
                "chart_heights": chart_heights,
                "hashes_note": "existing artifacts; no rescan",
                "atlas_links": {
                    "hrefs": sorted(atlas_links),
                    "missing_files": missing_atlas_links,
                    "expected_missing": expected_missing,
                    "expected_present": expected_present,
                },
                "rail_links": {
                    "count": len(rail_links),
                    "counts": rail_type_counts,
                    "missing": missing_rail_links,
                },
                "viewport_1280": {
                    "map": dom1280["map"],
                    "canvas": dom1280["canvas"],
                    "brief": dom1280["brief"],
                    "rail": dom1280["rail"],
                    "chart": dom1280["chart"],
                    "page": dom1280["page"],
                    "chartHeights": dom1280["chartHeights"],
                    "minChartTextPx": dom1280["textLeaves"]["min"],
                    "railStacked": dom1280["rail"]["y"]
                    >= dom1280["canvas"]["bottom"] - 1,
                },
                "booleans": booleans,
                "failed": failed,
            }
        browser.close()

    payload["all_pass"] = all_pass
    text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    (OUT / "step12.json").write_text(text, encoding="utf-8")

    print("step-12 evidence (existing outputs; no rescan)")
    for repo in REPOS:
        entry = payload["repos"][repo]
        measures = entry["measurements"]
        bars = measures["weightBars"]
        print(
            f"{'PASS' if not entry['failed'] else 'FAIL'} {repo}: "
            f"map={entry['map_height']}px (target <= {TARGET_MAP_HEIGHT}) · "
            f"canvas_bottom={measures['canvas']['bottom']} · "
            f"brief_top={measures['brief']['y']} · "
            f"charts={measures['chartTotalHeight']}px {list(entry['chart_heights'].values())} · "
            f"min_text={entry['min_chart_text_px']}px · "
            f"weight_bars={bars['positive']}/{bars['count']} positive "
            f"(max {bars['max']}px) · "
            f"atlas_links={len(entry['atlas_links']['hrefs'])} · "
            f"rail_links={entry['rail_links']['count']}"
        )
        if entry["failed"]:
            print(f"  failed: {', '.join(entry['failed'])}")
    print(f"{'PASS' if all_pass else 'FAIL'}: all step-12 assertions")
    print(f"wrote {OUT / 'step12.json'}")
    return 0 if all_pass else 1


if __name__ == "__main__":
    raise SystemExit(main())
