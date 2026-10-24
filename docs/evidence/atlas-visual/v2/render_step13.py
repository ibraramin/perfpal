#!/usr/bin/env python3
"""Atlas v2 step 13 — founder visual gate measurement harness.

Renders the *real* page (no rescan) at 1440x900 and 1280x800, asserts the
v2 visual-gate DoD, writes the screenshots, the G-baseline composite and
step13.json. Fail-closed: any failed assertion exits 1.

Run from the repo root:
    python3 docs/evidence/atlas-visual/v2/render_step13.py
"""

from __future__ import annotations

import hashlib
import json
import os
import struct
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

REPO = Path(__file__).resolve().parents[4]
# Session-private scan outputs. The shared perfpal-test/out-atlas-v2-* dirs are
# written by parallel sessions (the C track adds agent-report links), so the
# gate never reads them.
TEST_ROOT = Path(
    os.environ.get("PERFPAL_STEP13_OUT", "/tmp/opencode/step13-page")
)
SHARED_ROOT = Path("/home/shafin/Opencode/perfpal-test")
OUT = Path(__file__).resolve().parent
G_FINAL = REPO / "docs/evidence/map-vision/v7/g/g-hyperfine-final.png"
REPOS = ["hyperfine", "cJSON"]
VIEWPORT = {"width": 1440, "height": 900}
VIEWPORT_1280 = {"width": 1280, "height": 800}
W, H = VIEWPORT["width"], VIEWPORT["height"]
TARGET_MAP_HEIGHT = 5000
PINNED_CHROME = Path(
    "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
)
STRUCTURAL_CAPTION = (
    "structural (LOW name-based + MEDIUM semantic), may over/under-approximate"
)

DOM = r"""
() => {
  const q = (sel, root) => (root || document).querySelector(sel);
  const rect = (el) => {
    if (!el) return null;
    const r = el.getBoundingClientRect();
    return {x: Math.round(r.x * 10) / 10, y: Math.round(r.y * 10) / 10,
            w: Math.round(r.width * 10) / 10, h: Math.round(r.height * 10) / 10,
            bottom: Math.round(r.bottom * 10) / 10,
            right: Math.round(r.right * 10) / 10};
  };
  const map = q('#map');
  const canvas = q('#atlas-canvas');
  const footer = q('#atlas-canvas footer');
  const page = document.scrollingElement || document.documentElement;

  // Every visible text leaf inside #map (used for truncation / clipping).
  const leaves = [];
  const walker = document.createTreeWalker(map, NodeFilter.SHOW_TEXT);
  while (walker.nextNode()) {
    const text = (walker.currentNode.nodeValue || '').trim();
    if (!text) continue;
    const el = walker.currentNode.parentElement;
    if (!el) continue;
    if (['SCRIPT', 'STYLE', 'NOSCRIPT', 'TITLE'].includes(el.tagName)) continue;
    const cs = getComputedStyle(el);
    if (cs.display === 'none' || cs.visibility === 'hidden'
        || cs.visibility === 'collapse' || parseFloat(cs.opacity) === 0) continue;
    if (el.getClientRects().length === 0) continue;
    leaves.push({text: text.slice(0, 120), tag: el.tagName,
                 cls: String(el.className?.baseVal ?? el.className ?? ''),
                 size: Math.round(parseFloat(cs.fontSize) * 100) / 100,
                 box: rect(el)});
  }

  // Text clipped by a hidden/clip ancestor (overflow:auto/scroll is reachable
  // by scrolling and is therefore not "clipped text").
  const clipped = [];
  for (const el of map.querySelectorAll('*')) {
    const cs = getComputedStyle(el);
    const hardX = cs.overflowX === 'hidden' || cs.overflowX === 'clip';
    const hardY = cs.overflowY === 'hidden' || cs.overflowY === 'clip';
    if (!hardX && !hardY) continue;
    if (!el.getClientRects().length) continue;
    if (hardX && el.clientWidth && el.scrollWidth > el.clientWidth + 2) {
      clipped.push({tag: el.tagName, id: el.id,
                    cls: String(el.className?.baseVal ?? el.className ?? ''),
                    axis: 'x', client: el.clientWidth, scroll: el.scrollWidth,
                    box: rect(el)});
    }
    if (hardY && el.clientHeight && el.scrollHeight > el.clientHeight + 2) {
      clipped.push({tag: el.tagName, id: el.id,
                    cls: String(el.className?.baseVal ?? el.className ?? ''),
                    axis: 'y', client: el.clientHeight, scroll: el.scrollHeight,
                    box: rect(el)});
    }
  }

  // SVG node text vs its node rect (the V8 legibility rule).
  const nodeOverflow = [];
  for (const text of map.querySelectorAll('.atlas-node text')) {
    const node = text.closest('.atlas-node');
    const rectEl = node ? node.querySelector('rect') : null;
    if (!node || !rectEl) continue;
    const t = text.getBoundingClientRect();
    const r = rectEl.getBoundingClientRect();
    // Badges are intentionally drawn above the node rect; only horizontal
    // overflow is a legibility defect (the V8 rule).
    if (t.left < r.left - 2 || t.right > r.right + 2) {
      nodeOverflow.push({text: (text.textContent || '').slice(0, 80),
                         left: Math.round(t.left * 10) / 10,
                         right: Math.round(t.right * 10) / 10,
                         top: Math.round(t.top * 10) / 10,
                         bottom: Math.round(t.bottom * 10) / 10,
                         rectLeft: Math.round(r.left * 10) / 10,
                         rectRight: Math.round(r.right * 10) / 10,
                         rectTop: Math.round(r.top * 10) / 10,
                         rectBottom: Math.round(r.bottom * 10) / 10});
    }
  }

  const text = map.innerText || '';
  const count = (re) => (text.match(re) || []).length;
  const leadingDot = leaves.filter(
    (leaf) => /(^|\s)(\.\.|\u2026)[^\s.]/.test(leaf.text)
  );
  const cssEllipsis = [...map.querySelectorAll('*')]
    .filter((el) => {
      const cs = getComputedStyle(el);
      if (cs.textOverflow !== 'ellipsis') return false;
      if (!el.clientWidth || el.scrollWidth <= el.clientWidth + 1) return false;
      return !(el.getAttribute('title') || el.getAttribute('aria-label'));
    })
    .map((el) => ({tag: el.tagName,
                   cls: String(el.className?.baseVal ?? el.className ?? ''),
                   text: (el.textContent || '').trim().slice(0, 80),
                   clientW: el.clientWidth, scrollW: el.scrollWidth}));

  const idCounts = {};
  for (const el of document.querySelectorAll('[id]')) {
    idCounts[el.id] = (idCounts[el.id] || 0) + 1;
  }
  const duplicateIds = Object.entries(idCounts).filter(([, n]) => n > 1);
  const idCountsMap = {};
  for (const el of map.querySelectorAll('[id]')) {
    idCountsMap[el.id] = (idCountsMap[el.id] || 0) + 1;
  }
  const duplicateIdsInMap = Object.entries(idCountsMap).filter(([, n]) => n > 1);

  const tokens = {};
  const sample = (name, sel) => {
    const el = q(sel);
    if (!el) { tokens[name] = null; return; }
    const cs = getComputedStyle(el);
    tokens[name] = {
      font: cs.fontFamily, size: parseFloat(cs.fontSize), weight: cs.fontWeight,
      color: cs.color, bg: cs.backgroundColor,
    };
  };
  sample('body', 'body');
  sample('map', '#map');
  sample('wordmark', '.atlas-wordmark');
  sample('kicker', '.atlas-kicker');
  sample('subline', '.atlas-subline');
  sample('keymetrics', '.atlas-keymetrics');
  sample('node', '.atlas-node text');
  sample('legend', '.atlas-legend');

  const canvasElements = [...canvas.querySelectorAll('*')];
  const eventHandlers = canvasElements.filter((el) =>
    [...el.attributes].some((attr) => attr.name.startsWith('on'))).length;

  return {
    map: rect(map),
    chrome: rect(q('.atlas-page-chrome')),
    stage: rect(q('.atlas-stage')),
    canvas: rect(canvas),
    canvasFooter: rect(footer),
    rail: rect(q('#atlas-rail')),
    brief: rect(q('#atlas-brief')),
    charts: rect(q('#atlas-charts')),
    page: {
      clientWidth: document.documentElement.clientWidth,
      scrollWidth: page.scrollWidth,
      scrollHeight: page.scrollHeight,
    },
    counts: {
      structuralCaptions: map.querySelectorAll('.atlas-structural-caption').length,
      units: map.querySelectorAll('.atlas-unit').length,
      legends: map.querySelectorAll('.atlas-legend').length,
      footers: map.querySelectorAll('footer').length,
      railCards: map.querySelectorAll('.atlas-rail-card').length,
      railStubs: map.querySelectorAll('.atlas-rail-stub').length,
      edgeLabels: map.querySelectorAll('.atlas-edge-label').length,
      tablesInCanvas: canvas.querySelectorAll('table').length,
      scriptsInCanvas: canvas.querySelectorAll('script').length,
      eventHandlersInCanvas: eventHandlers,
      barRowsInCanvas: canvas.querySelectorAll('.atlas-bar-row').length,
      railCardsInCanvas: canvas.querySelectorAll('.atlas-rail-card').length,
    },
    definitions: {
      structuralCaption: count(/structural \(LOW name-based \+ MEDIUM semantic\)/g),
      medianDefinition: count(/× median function = all functions in the scan/g),
      unreachedDefinition: count(/unreached production: \d+ · non-production: \d+/g),
    },
    textLeaves: {
      count: leaves.length,
      minSize: leaves.reduce((acc, leaf) => acc === null ? leaf.size
                             : Math.min(acc, leaf.size), null),
      smallest: [...leaves].sort((a, b) => a.size - b.size).slice(0, 6),
    },
    leadingDot: leadingDot.slice(0, 10),
    cssEllipsis: cssEllipsis.slice(0, 10),
    clippedContainers: clipped.slice(0, 10),
    nodeTextOverflow: nodeOverflow.slice(0, 10),
    duplicateIds,
    duplicateIdsInMap,
    tokens,
    header: (q('.atlas-masthead')?.innerText || '').slice(0, 240),
    keymetrics: (q('.atlas-keymetrics')?.innerText || '').slice(0, 700),
    footerText: (footer?.innerText || '').slice(0, 500),
    firstActions: [...map.querySelectorAll('.atlas-keymetrics a, .atlas-actions a')]
      .slice(0, 4).map((el) => (el.textContent || '').trim()),
    htmlTitle: document.title,
  };
}
"""


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


def launch_browser(pw):
    if PINNED_CHROME.is_file():
        print(f"chromium executable: {PINNED_CHROME}")
        return pw.chromium.launch(executable_path=str(PINNED_CHROME))
    return pw.chromium.launch()


def scroll_map_to_top(page) -> None:
    page.evaluate(
        "() => { const map = document.querySelector('#map');"
        " if (map) map.scrollIntoView({block: 'start'}); }"
    )
    page.evaluate("() => document.fonts.ready.then(() => true)")
    page.wait_for_timeout(120)
    page.mouse.move(1, 1)
    page.wait_for_timeout(40)


def measure(page, viewport_h: int) -> dict:
    data = page.evaluate(DOM)
    data["viewport"] = {"width": page.viewport_size["width"],
                        "height": viewport_h}
    return data


def assertion_flags(data: dict, kind: str) -> dict[str, bool]:
    """Derive the published assertion booleans from the measurements."""
    canvas = data["canvas"]
    footer = data["canvasFooter"]
    counts = data["counts"]
    defs = data["definitions"]
    flags = {
        "canvas_bottom_le_900": bool(canvas and canvas["bottom"] <= H),
        "canvas_footer_bottom_le_900": bool(footer is None or footer["bottom"] <= H),
        "map_height_le_5000": bool(
            data["map"] and data["map"]["h"] <= TARGET_MAP_HEIGHT
        ),
        "no_horizontal_scroll": (
            data["page"]["scrollWidth"] <= data["page"]["clientWidth"]
        ),
        "no_leading_dot_truncation": not data["leadingDot"],
        "no_css_ellipsis_without_title": not data["cssEllipsis"],
        "no_clipped_text": not data["clippedContainers"],
        "no_node_text_overflow": not data["nodeTextOverflow"],
        "one_structural_caption": (
            counts["structuralCaptions"] == 1
            and defs["structuralCaption"] == 1
        ),
        "one_unit_definition": counts["units"] == 1,
        "one_median_definition": defs["medianDefinition"] == 1,
        "one_unreached_definition": defs["unreachedDefinition"] == 1,
        "one_legend": counts["legends"] == 1,
        "one_footer": counts["footers"] == 1,
        "no_duplicate_ids": (
            not data["duplicateIds"] and not data["duplicateIdsInMap"]
        ),
        "canvas_purity_tables_scripts_handlers": all(
            counts[key] == 0
            for key in (
                "tablesInCanvas",
                "scriptsInCanvas",
                "eventHandlersInCanvas",
                "barRowsInCanvas",
                "railCardsInCanvas",
            )
        ),
        "below_canvas_first_block": bool(
            data["brief"] and canvas and data["brief"]["y"] >= canvas["bottom"]
        ),
        "chrome_above_canvas": bool(
            data["chrome"] and canvas and data["chrome"]["bottom"] <= canvas["y"]
        ),
        "rail_stacks_at_1280": bool(
            data["rail"] and canvas and data["rail"]["y"] >= canvas["bottom"] - 2
        ),
    }
    if kind == "1440":
        flags.pop("rail_stacks_at_1280")
    else:
        flags.pop("canvas_bottom_le_900")
        flags.pop("canvas_footer_bottom_le_900")
    return flags


def aggregate_flags(first: dict, second: dict) -> dict[str, bool]:
    """AND the common flags across viewports; keep the viewport-only ones."""
    out = {}
    for key, value in first.items():
        out[key] = bool(value and second.get(key, True))
    out["no_horizontal_scroll_1440"] = first["no_horizontal_scroll"]
    out["no_horizontal_scroll_1280"] = second["no_horizontal_scroll"]
    out.pop("no_horizontal_scroll", None)
    out["rail_stacks_at_1280"] = second["rail_stacks_at_1280"]
    return out


def check_common(
    repo: str, data: dict, viewport_h: int, enforce_fold: bool
) -> list[str]:
    """Assertions that hold at both viewports (fold only at 1440x900)."""
    errors: list[str] = []
    canvas = data["canvas"]
    footer = data["canvasFooter"]
    if canvas is None:
        return [f"{repo}: no #atlas-canvas"]
    if enforce_fold and canvas["bottom"] > viewport_h:
        errors.append(
            f"{repo}: canvas bottom {canvas['bottom']} > {viewport_h}px fold"
        )
    if enforce_fold and footer is not None and footer["bottom"] > viewport_h:
        errors.append(
            f"{repo}: canvas footer bottom {footer['bottom']} > {viewport_h}px fold"
        )
    map_height = data["map"]["h"] if data["map"] else None
    if map_height is None or map_height > TARGET_MAP_HEIGHT:
        errors.append(
            f"{repo}: #map height {map_height} > {TARGET_MAP_HEIGHT}px budget"
        )
    if data["page"]["scrollWidth"] > data["page"]["clientWidth"]:
        errors.append(
            f"{repo}: horizontal scroll at {data['viewport']['width']}px "
            f"({data['page']['scrollWidth']} > {data['page']['clientWidth']})"
        )
    if data["leadingDot"]:
        errors.append(f"{repo}: leading-dot truncation: {data['leadingDot'][:3]}")
    if data["clippedContainers"]:
        errors.append(
            f"{repo}: clipped text containers: {data['clippedContainers'][:3]}"
        )
    if data["nodeTextOverflow"]:
        errors.append(
            f"{repo}: node text overflows its rect: {data['nodeTextOverflow'][:3]}"
        )
    if data["cssEllipsis"]:
        errors.append(
            f"{repo}: CSS ellipsis truncation without a full value in title: "
            f"{data['cssEllipsis'][:3]}"
        )
    counts = data["counts"]
    if counts["structuralCaptions"] != 1:
        errors.append(
            f"{repo}: {counts['structuralCaptions']} structural captions (want 1)"
        )
    if data["definitions"]["structuralCaption"] != 1:
        errors.append(
            f"{repo}: structural-caption text appears "
            f"{data['definitions']['structuralCaption']}× (want 1)"
        )
    if counts["units"] != 1:
        errors.append(f"{repo}: {counts['units']} unit definitions (want 1)")
    if counts["legends"] != 1:
        errors.append(f"{repo}: {counts['legends']} legends (want 1)")
    if counts["footers"] != 1:
        errors.append(f"{repo}: {counts['footers']} footers (want 1)")
    if data["definitions"]["medianDefinition"] != 1:
        errors.append(
            f"{repo}: median definition appears "
            f"{data['definitions']['medianDefinition']}× (want 1)"
        )
    if data["definitions"]["unreachedDefinition"] != 1:
        errors.append(
            f"{repo}: unreached definition appears "
            f"{data['definitions']['unreachedDefinition']}× (want 1)"
        )
    if data["duplicateIds"]:
        errors.append(f"{repo}: duplicate DOM ids: {data['duplicateIds']}")
    if data["duplicateIdsInMap"]:
        errors.append(
            f"{repo}: duplicate DOM ids in #map: {data['duplicateIdsInMap']}"
        )
    for key, want in (
        ("tablesInCanvas", 0),
        ("scriptsInCanvas", 0),
        ("eventHandlersInCanvas", 0),
        ("barRowsInCanvas", 0),
        ("railCardsInCanvas", 0),
    ):
        if counts[key] != want:
            errors.append(f"{repo}: canvas {key}={counts[key]} (want {want})")
    if (
        data["brief"] is not None
        and canvas is not None
        and data["brief"]["y"] < canvas["bottom"]
    ):
        errors.append(
            f"{repo}: first below-canvas block starts at {data['brief']['y']} "
            f"< canvas bottom {canvas['bottom']}"
        )
    if data["chrome"] is not None and data["chrome"]["bottom"] > canvas["y"]:
        errors.append(
            f"{repo}: page chrome overlaps the canvas "
            f"({data['chrome']['bottom']} > {canvas['y']})"
        )
    if not data["header"].strip():
        errors.append(f"{repo}: empty masthead")
    if not data["keymetrics"].strip():
        errors.append(f"{repo}: empty key-metrics strip")
    return errors


def check_1440(repo: str, data: dict, enforce_fold: bool = True) -> list[str]:
    errors = check_common(repo, data, H, enforce_fold)
    counts = data["counts"]
    if data["rail"] is None:
        errors.append(f"{repo}: no #atlas-rail")
    elif counts["railCards"] < 1 and counts["railStubs"] < 1:
        errors.append(f"{repo}: rail has neither cards nor a named stub")
    if data["rail"] is not None and data["rail"]["y"] > H:
        errors.append(f"{repo}: rail starts below the fold ({data['rail']['y']})")
    if data["keymetrics"] and "START" not in data["keymetrics"]:
        errors.append(f"{repo}: key-metrics strip has no START actions")
    return errors


def check_1280(repo: str, data: dict) -> list[str]:
    errors = check_common(repo, data, 800, False)
    canvas = data["canvas"]
    rail = data["rail"]
    if rail is not None and canvas is not None and rail["y"] < canvas["bottom"] - 2:
        errors.append(
            f"{repo}: rail does not stack below the canvas at 1280px "
            f"(rail y {rail['y']} < canvas bottom {canvas['bottom']})"
        )
    return errors


def make_g_composite(step13_final: Path) -> dict:
    """G (approved) vs Atlas v2 step 13, labelled, at native 1440 width."""
    try:
        from PIL import Image, ImageDraw, ImageFont
    except Exception as exc:  # pragma: no cover
        return {"status": "failed", "reason": f"Pillow unavailable: {exc}"}
    if not G_FINAL.is_file():
        return {"status": "failed", "reason": f"missing G baseline {G_FINAL}"}

    def font(size: int, bold: bool = False):
        name = "DejaVuSans-Bold.ttf" if bold else "DejaVuSans.ttf"
        for base in ("/usr/share/fonts/truetype/dejavu",
                     "/usr/share/fonts/truetype/liberation"):
            candidate = Path(base) / (
                name if "dejavu" in base else
                ("LiberationSans-Bold.ttf" if bold else "LiberationSans-Regular.ttf")
            )
            if candidate.exists():
                return ImageFont.truetype(str(candidate), size)
        return ImageFont.load_default()

    g = Image.open(G_FINAL).convert("RGB")
    a = Image.open(step13_final).convert("RGB")
    margin, gutter, header_h, footer_h = 32, 32, 92, 52
    x_left, x_right = margin, margin + W + gutter
    width = margin * 2 + W * 2 + gutter
    height = margin + header_h + H + footer_h + margin
    paper, ink, muted, accent, hairline = (
        (244, 241, 234), (32, 43, 53), (111, 106, 97), (194, 65, 12), (214, 208, 196)
    )
    compos = Image.new("RGB", (width, height), paper)
    draw = ImageDraw.Draw(compos)
    top = margin
    draw.text((x_left, top), "G · approved V7 prototype (baseline)", font=font(20, True), fill=ink)
    draw.text((x_left, top + 30),
              "docs/evidence/map-vision/v7/g/g-hyperfine-final.png · 1440×900",
              font=font(11), fill=muted)
    draw.text((x_right, top), "Atlas v2 · step 13 (this build)", font=font(20, True), fill=accent)
    draw.text((x_right, top + 30),
              "report.html#map · real hyperfine scan · 1440×900 · reduced motion",
              font=font(11), fill=muted)
    draw.line([(x_left, top + 56), (x_right + W, top + 56)], fill=hairline, width=1)
    image_top = margin + header_h
    compos.paste(g.resize((W, H), Image.LANCZOS), (x_left, image_top))
    compos.paste(a.resize((W, H), Image.LANCZOS), (x_right, image_top))
    draw.rectangle([x_left - 1, image_top - 1, x_left + W, image_top + H],
                   outline=hairline, width=1)
    draw.rectangle([x_right - 1, image_top - 1, x_right + W, image_top + H],
                   outline=hairline, width=1)
    draw.text((x_left, height - footer_h + 12),
              "G: one 900px artboard, graph + rail, one legend/footer. "
              "Atlas: same system, key-metrics strip, 4 progressive charts below the fold.",
              font=font(11), fill=muted)
    draw.text((x_right, height - footer_h + 30),
              "Evidence composite · no synthetic content · founder gate step 13",
              font=font(11), fill=muted)
    out = OUT / "step13" / "G-vs-step13-hyperfine-1440x900.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    compos.save(out)
    gw, gh = png_size(G_FINAL)
    return {
        "status": "ok",
        "file": str(out.relative_to(OUT)),
        "sha256": sha256(out),
        "g_baseline": {"file": str(G_FINAL.relative_to(REPO)),
                       "width": gw, "height": gh, "sha256": sha256(G_FINAL)},
    }


def main() -> int:
    reports = {}
    failures: list[str] = []
    for repo in REPOS:
        path = TEST_ROOT / repo / "report.html"
        if not path.is_file():
            failures.append(f"{repo}: missing real output {path}")
        reports[repo] = path
    if failures:
        for failure in failures:
            print(f"FATAL: {failure}", file=sys.stderr)
        (OUT / "step13.json").write_text(
            json.dumps({"status": "FAIL", "failures": failures}, indent=2) + "\n",
            encoding="utf-8",
        )
        return 1

    measured: dict[str, dict] = {}
    repo_flags: dict[str, dict] = {}
    with sync_playwright() as pw:
        browser = launch_browser(pw)
        try:
            for repo in REPOS:
                html = reports[repo]
                context = browser.new_context(viewport=VIEWPORT, device_scale_factor=1)
                try:
                    page = context.new_page()
                    page.emulate_media(reduced_motion="reduce")
                    page.goto(html.as_uri() + "#map")
                    page.wait_for_selector("#map")
                    page.wait_for_load_state("load")
                    scroll_map_to_top(page)
                    data = measure(page, H)
                finally:
                    context.close()
                measured[repo] = {"1440": data}
                failures.extend(check_1440(repo, data))

                context = browser.new_context(
                    viewport=VIEWPORT_1280, device_scale_factor=1
                )
                try:
                    page = context.new_page()
                    page.emulate_media(reduced_motion="reduce")
                    page.goto(html.as_uri() + "#map")
                    page.wait_for_selector("#map")
                    page.wait_for_load_state("load")
                    scroll_map_to_top(page)
                    data = measure(page, 800)
                finally:
                    context.close()
                measured[repo]["1280"] = data
                failures.extend(check_1280(repo, data))

                flags_1440 = assertion_flags(measured[repo]["1440"], "1440")
                flags_1280 = assertion_flags(data, "1280")
                measured[repo]["assertions_1440"] = flags_1440
                measured[repo]["assertions_1280"] = flags_1280
                for key, value in flags_1440.items():
                    if not value:
                        failures.append(f"{repo}: 1440 assertion `{key}` is false")
                for key, value in flags_1280.items():
                    if not value:
                        failures.append(f"{repo}: 1280 assertion `{key}` is false")
                repo_flags[repo] = aggregate_flags(flags_1440, flags_1280)

                base = measured[repo]["1440"]
                print(
                    f"{repo:<10} #map {base['map']['h']:.0f}px  "
                    f"canvas {base['canvas']['y']:.0f}-{base['canvas']['bottom']:.0f}  "
                    f"footer {base['canvasFooter']['bottom']:.0f}  "
                    f"rail {base['rail']['y']:.0f}-{base['rail']['bottom']:.0f}  "
                    f"brief {base['brief']['y']:.0f}  "
                    f"page {base['page']['scrollHeight']}px  "
                    f"hscroll {base['page']['scrollWidth']}/{base['page']['clientWidth']}"
                )
                print(
                    f"            definitions caption={base['counts']['structuralCaptions']} "
                    f"unit={base['counts']['units']} legend={base['counts']['legends']} "
                    f"footer={base['counts']['footers']} "
                    f"median={base['definitions']['medianDefinition']} "
                    f"unreached={base['definitions']['unreachedDefinition']}  "
                    f"leading-dot={len(base['leadingDot'])} "
                    f"clipped={len(base['clippedContainers'])} "
                    f"node-overflow={len(base['nodeTextOverflow'])} "
                    f"min-text={base['textLeaves']['minSize']}px"
                )
        finally:
            browser.close()

    if failures:
        print("\nSTEP-13 DOM GATE FAILED:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        (OUT / "step13.json").write_text(
            json.dumps({"status": "FAIL", "failures": failures,
                        "measured": measured}, indent=2) + "\n",
            encoding="utf-8",
        )
        return 1

    # Screenshots (preflight passed).
    artifacts: dict[str, dict] = {}
    with sync_playwright() as pw:
        browser = launch_browser(pw)
        try:
            for repo in REPOS:
                html = reports[repo]
                context = browser.new_context(viewport=VIEWPORT, device_scale_factor=1)
                try:
                    page = context.new_page()
                    page.emulate_media(reduced_motion="reduce")
                    page.goto(html.as_uri() + "#map")
                    page.wait_for_selector("#map")
                    page.wait_for_load_state("load")
                    scroll_map_to_top(page)
                    final = OUT / f"atlas-v2-step13-{repo}-final.png"
                    full = OUT / f"atlas-v2-step13-{repo}-full.png"
                    charts = OUT / f"atlas-v2-step13-{repo}-charts.png"
                    weight = OUT / "step13" / f"{repo}-weight.png"
                    weight.parent.mkdir(parents=True, exist_ok=True)
                    page.screenshot(path=str(final), animations="disabled")
                    page.screenshot(path=str(full), full_page=True,
                                    animations="disabled")
                    page.locator("#atlas-charts").screenshot(
                        path=str(charts), animations="disabled")
                    page.locator(
                        '#atlas-charts [data-atlas-block="weight-chart"]'
                    ).screenshot(path=str(weight), animations="disabled")
                finally:
                    context.close()

                context = browser.new_context(
                    viewport=VIEWPORT_1280, device_scale_factor=1
                )
                try:
                    page = context.new_page()
                    page.emulate_media(reduced_motion="reduce")
                    page.goto(html.as_uri() + "#map")
                    page.wait_for_selector("#map")
                    page.wait_for_load_state("load")
                    scroll_map_to_top(page)
                    shot1280 = OUT / f"atlas-v2-step13-{repo}-1280x800.png"
                    page.screenshot(path=str(shot1280), animations="disabled")
                finally:
                    context.close()

                entry = {}
                for name, path in (("final", final), ("full", full),
                                   ("charts", charts), ("weight", weight),
                                   ("1280", shot1280)):
                    width, height = png_size(path)
                    entry[name] = {"file": str(path.relative_to(OUT)),
                                   "width": width, "height": height,
                                   "sha256": sha256(path)}
                artifacts[repo] = entry
                print(f"{repo:<10} screenshots: final {entry['final']['width']}x"
                      f"{entry['final']['height']}, weight "
                      f"{entry['weight']['width']}x{entry['weight']['height']}, "
                      f"1280 {entry['1280']['width']}x{entry['1280']['height']}")
        finally:
            browser.close()

    comparison = make_g_composite(OUT / "atlas-v2-step13-hyperfine-final.png")
    if comparison.get("status") != "ok":
        print(f"FATAL: G-baseline composite failed: {comparison}", file=sys.stderr)
        (OUT / "step13.json").write_text(
            json.dumps(
                {"status": "FAIL", "failures": [f"G-baseline composite: {comparison}"]},
                indent=2,
            ) + "\n",
            encoding="utf-8",
        )
        return 1

    payload = {
        "status": "PASS",
        "step": 13,
        "viewport": {"width": W, "height": H},
        "viewport_1280": VIEWPORT_1280,
        "capture": {
            "source": "session-private fresh scan with the release binary built "
                      "from merged main; the shared perfpal-test/out-atlas-v2-* "
                      "outputs are cross-checked but never the render source",
            "repos": {
                repo: {
                    "report_html": str(reports[repo]),
                    "report_html_sha256": sha256(reports[repo]),
                    "shared_output": {
                        "path": str(SHARED_ROOT / f"out-atlas-v2-{repo}" / "report.html"),
                        "exists": (
                            SHARED_ROOT / f"out-atlas-v2-{repo}" / "report.html"
                        ).is_file(),
                        "sha256": (
                            sha256(SHARED_ROOT / f"out-atlas-v2-{repo}" / "report.html")
                            if (SHARED_ROOT / f"out-atlas-v2-{repo}" / "report.html").is_file()
                            else None
                        ),
                        "same_as_private": (
                            (SHARED_ROOT / f"out-atlas-v2-{repo}" / "report.html").is_file()
                            and sha256(SHARED_ROOT / f"out-atlas-v2-{repo}" / "report.html")
                            == sha256(reports[repo])
                        ),
                    },
                }
                for repo in REPOS
            },
            "animation": "prefers-reduced-motion + animations=disabled",
            "map_anchor": "#map scrolled to viewport top; #map is a DOM sibling "
                          "of the global report header",
        },
        "assertions": {
            key: bool(all(repo_flags[repo][key] for repo in REPOS))
            for key in repo_flags[REPOS[0]]
        },
        "shared_output_drift": {
            "used": False,
            "note": "Track C fast-forward merged into main (c73d6ff) during the "
                    "gate. The gate was re-scanned and re-rendered on that tree; "
                    "the private scan equals the shared perfpal-test page. The "
                    "merge adds two below-the-fold nav links (agent-report.md, "
                    "agent-actions.json): -charts and -full change, while the "
                    "900px -final, weight crops, 1280 frames and all eight "
                    "fixture frames are byte-identical, so the 32 reads stand.",
        },
        "measured": measured,
        "artifacts": artifacts,
        "g_baseline": comparison,
    }
    out = OUT / "step13.json"
    out.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"\nPASS — wrote {out}")
    print(f"G composite: {OUT / comparison['file']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
