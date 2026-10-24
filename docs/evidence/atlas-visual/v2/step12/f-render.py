#!/usr/bin/env python3
"""F treemap candidate prototype (Atlas v2, step 12) — evidence only.

Status: candidate prototype, not shipped. F remains undecided.

Reads exactly two real scans:
  /home/shafin/Opencode/perfpal-test/out-atlas-v2-hyperfine/metrics.json
  /home/shafin/Opencode/perfpal-test/out-atlas-v2-cJSON/metrics.json

Writes, in this directory:
  f-treemap-hyperfine.html
  f-treemap-cJSON.html
  f-treemap-hyperfine.png   (only when Playwright + chromium are available)
  f-treemap-cJSON.png       (only when Playwright + chromium are available)

No src/, tests/, schema or Cargo changes. No JavaScript in the artifacts, no
event handlers, no links or URLs, no network at render time, no new dependency.
The django/postgres holdout is never read or referenced as input.

Run from the repo root:
    python3 docs/evidence/atlas-visual/v2/step12/f-render.py
"""

from __future__ import annotations

import html
import json
import math
import statistics
import struct
from pathlib import Path

HERE = Path(__file__).resolve().parent
TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
INPUTS = {
    "hyperfine": TEST_ROOT / "out-atlas-v2-hyperfine" / "metrics.json",
    "cJSON": TEST_ROOT / "out-atlas-v2-cJSON" / "metrics.json",
}
ALLOWED = {path.resolve() for path in INPUTS.values()}

PAGE_W = 1440
GUTTER = 16
CONTENT_W = PAGE_W - 2 * GUTTER  # 1408, the Atlas full-bleed band width
PANEL_H = 260
TOP_CELLS = 50
UNIT = "static cost units (v2)"
HOLDOUT_MARKERS = ("django", "postgres")

# Warm Atlas tokens; the production ramp is a single-hue teal scale.
RAMP = [
    (0.00, "#e9f0ed"),
    (0.20, "#d3e4df"),
    (0.40, "#b0cec7"),
    (0.62, "#86b0a7"),
    (0.82, "#5e9087"),
    (1.00, "#3f6d6a"),
]


def esc(value: object) -> str:
    return html.escape(str(value), quote=True)


def guard_input(path: Path) -> Path:
    text = str(path).lower()
    if any(marker in text for marker in HOLDOUT_MARKERS):
        raise SystemExit("FATAL: holdout input is forbidden (django/postgres)")
    if path.resolve() not in ALLOWED:
        raise SystemExit(f"FATAL: input is not one of the two allowed scans: {path}")
    return path


def fmt_int(value: float) -> str:
    return f"{int(round(value)):,}"


def fmt_share_spec(share: float) -> str:
    """Atlas-metrics section 5: <1% one decimal; 1-9.9% one decimal; >=10% integer."""
    pct = share * 100.0
    if pct >= 10:
        return f"{pct:.0f}%"
    return f"{pct:.1f}%"


def fmt_share_exact(share: float) -> str:
    pct = share * 100.0
    if pct == 0:
        return "0%"
    if pct < 0.01:
        return "less than 0.01%"
    return f"{pct:.2f}%"


def fmt_mult(value: float, median: float) -> str:
    if median <= 0:
        return "n/a"
    ratio = value / median
    if ratio < 0.05:
        return "<0.1\u00d7"
    if ratio < 10:
        return f"{ratio:.1f}\u00d7"
    return f"{int(math.floor(ratio + 0.5)):,}\u00d7"


def top_pct(rank: int, population: int) -> int:
    if population <= 0:
        return 100
    value = int(math.floor(100.0 * rank / population + 0.5))
    return min(100, max(1, value))


def trunc(text: str, limit: int) -> str:
    text = " ".join(str(text).split())
    if len(text) <= limit:
        return text
    return text[: max(0, limit - 1)].rstrip() + "\u2026"


def ramp(t: float) -> str:
    t = max(0.0, min(1.0, t))
    for i in range(1, len(RAMP)):
        t0, c0 = RAMP[i - 1]
        t1, c1 = RAMP[i]
        if t <= t1:
            fraction = (t - t0) / (t1 - t0) if t1 > t0 else 0.0
            a = [int(c0[j:j + 2], 16) for j in (1, 3, 5)]
            b = [int(c1[j:j + 2], 16) for j in (1, 3, 5)]
            return "#" + "".join(
                f"{round(a[k] + (b[k] - a[k]) * fraction):02x}" for k in range(3)
            )
    return RAMP[-1][1]


def dark_text(hex_color: str) -> bool:
    r = int(hex_color[1:3], 16) / 255.0
    g = int(hex_color[3:5], 16) / 255.0
    b = int(hex_color[5:7], 16) / 255.0
    return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.56


def squarify(
    values: list[float], x: float, y: float, w: float, h: float
) -> list[tuple[float, float, float, float]]:
    """Deterministic squarified treemap, identical to the V7 F renderer."""
    if not values:
        return []
    scale = (w * h) / sum(values)
    vals = [value * scale for value in values]
    rects: list[tuple[float, float, float, float]] = []

    def worst(row: list[float], side: float) -> float:
        total = sum(row)
        if total <= 0 or side <= 0:
            return math.inf
        biggest, smallest = max(row), min(row)
        return max(
            (side * side * biggest) / (total * total),
            (total * total) / (side * side * smallest),
        )

    i = 0
    while i < len(vals):
        row = [vals[i]]
        i += 1
        side = min(w, h)
        while i < len(vals) and worst(row + [vals[i]], side) <= worst(row, side):
            row.append(vals[i])
            i += 1
        total = sum(row)
        if w >= h:
            row_w = total / h if h else 0.0
            yy = y
            for area in row:
                row_h = area / row_w if row_w else 0.0
                rects.append((x, yy, row_w, row_h))
                yy += row_h
            x += row_w
            w -= row_w
        else:
            row_h = total / w if w else 0.0
            xx = x
            for area in row:
                col_w = area / row_h if row_h else 0.0
                rects.append((xx, y, col_w, row_h))
                xx += col_w
            y += row_h
            h -= row_h
    return rects


def build_model(repo: str) -> dict:
    path = guard_input(INPUTS[repo])
    data = json.loads(path.read_text(encoding="utf-8"))
    files = {item["path"]: item for item in data.get("files", [])}
    prefixes = sorted(
        {component["path_prefix"] for component in data.get("components", [])} | {"."}
    )

    cost_by_file: dict[str, int] = {}
    functions = data.get("stats", {}).get("functions", [])
    for function in functions:
        cost_by_file[function["path"]] = cost_by_file.get(function["path"], 0) + int(
            function.get("function_cost_v2", 0) or 0
        )
    total = sum(cost_by_file.values())
    if total <= 0:
        raise SystemExit(f"FATAL: {repo} has no function_cost_v2 weight")
    prod_total = sum(
        value
        for file_path, value in cost_by_file.items()
        if files.get(file_path, {}).get("path_class") == "production"
    )

    def component_of(file_path: str) -> str:
        best = "."
        for prefix in prefixes:
            if prefix == ".":
                continue
            if file_path == prefix or file_path.startswith(prefix + "/"):
                if best == "." or len(prefix) > len(best):
                    best = prefix
        return best

    base = {
        prefix: {"id": prefix, "cost": 0, "prod_cost": 0, "files": []}
        for prefix in prefixes
    }
    for file_path in files:
        base[component_of(file_path)]["files"].append(file_path)
    for file_path, value in cost_by_file.items():
        component = base[component_of(file_path)]
        component["cost"] += value
        if files.get(file_path, {}).get("path_class") == "production":
            component["prod_cost"] += value

    for component in base.values():
        component["files"].sort()
        component["members"] = len(component["files"])
        classes = [
            files.get(file_path, {}).get("path_class", "test")
            for file_path in component["files"]
        ]
        if component["cost"] <= 0:
            klass = "test"
        elif component["prod_cost"] * 2 >= component["cost"]:
            klass = "production"
        elif classes.count("bench") > classes.count("test"):
            klass = "bench"
        else:
            klass = "test"
        component["class"] = klass

    components = sorted(
        [component for component in base.values() if component["cost"] > 0],
        key=lambda component: (-component["cost"], component["id"]),
    )
    if not components:
        raise SystemExit(f"FATAL: {repo} has no component cost")
    median_component = float(statistics.median([c["cost"] for c in components]))
    for rank, component in enumerate(components, 1):
        component["rank"] = rank
        component["top_pct"] = top_pct(rank, len(components))
        component["share"] = component["cost"] / total
        component["mult"] = fmt_mult(component["cost"], median_component)
        component["label"] = "core (root)" if component["id"] == "." else component["id"]

    picked = sorted(
        components,
        key=lambda component: (
            0 if component["class"] == "production" else 1,
            -component["cost"],
            component["id"],
        ),
    )[:TOP_CELLS]
    shown_cost = sum(component["cost"] for component in picked)
    residual = total - shown_cost
    cells = [
        {**component, "kind": "component"}
        for component in sorted(picked, key=lambda item: (-item["cost"], item["id"]))
    ]
    picked_ids = {component["id"] for component in picked}
    omitted_components = [
        component for component in components if component["id"] not in picked_ids
    ]
    omitted_files = sum(component["members"] for component in omitted_components)
    if residual > 0:
        cells.append(
            {
                "id": "other",
                "label": f"other ({len(omitted_components)} components)",
                "kind": "other",
                "cost": residual,
                "prod_cost": 0,
                "class": "other",
                "members": omitted_files,
                "files": [],
                "share": residual / total,
                "mult": "residual",
                "top_pct": None,
                "rank": None,
            }
        )

    ordered = sorted(cells, key=lambda cell: (-cell["cost"], cell["id"]))
    rects = squarify([cell["cost"] for cell in ordered], 0, 0, CONTENT_W, PANEL_H)
    laid_out: list[dict] = []
    for cell, (x0, y0, width, height) in zip(ordered, rects):
        x0i = min(CONTENT_W - 1, max(0, int(round(x0))))
        y0i = min(PANEL_H - 1, max(0, int(round(y0))))
        x1i = min(CONTENT_W, max(x0i + 1, int(round(x0 + width))))
        y1i = min(PANEL_H, max(y0i + 1, int(round(y0 + height))))
        laid_out.append(
            {**cell, "x": x0i, "y": y0i, "w": x1i - x0i, "h": y1i - y0i}
        )

    ranked = sorted(
        [cell for cell in laid_out if cell["kind"] != "other"],
        key=lambda cell: (-cell["cost"], cell["id"]),
    )
    for rank, cell in enumerate(ranked, 1):
        cell["rank"] = rank
        cell["top_pct"] = top_pct(rank, len(ranked))
    for rank, cell in enumerate(ranked[:3], 1):
        cell["hot_rank"] = rank
    for i, cell in enumerate(
        sorted(laid_out, key=lambda item: (-item["cost"], item["id"]))
    ):
        cell["reveal"] = i

    parse = data.get("parse_health_summary", {})
    scan = data.get("scan", {})
    top_six_share = sum(component["share"] for component in components[:6])
    return {
        "repo": repo,
        "data": data,
        "files": files,
        "total": total,
        "prod_total": prod_total,
        "prod_share": prod_total / total,
        "components": components,
        "components_total": len(data.get("components", [])),
        "components_with_cost": len(components),
        "median_component": median_component,
        "cells": laid_out,
        "omitted_components": len(omitted_components),
        "omitted_files": omitted_files,
        "residual": residual,
        "n_files": int(scan.get("file_count", len(files))),
        "n_functions": len(functions),
        "n_findings": len(data.get("findings", [])),
        "parse": parse,
        "top_six_share": top_six_share,
    }


def read_line(model: dict) -> str:
    top = model["components"][0]
    if model["prod_share"] < 0.02:
        return (
            f"One {esc(top['class'])} component <code>{esc(top['label'])}</code> holds "
            f"<strong>{fmt_share_exact(top['share'])}</strong> of scanned static cost; "
            f"all production is <strong>{fmt_share_exact(model['prod_share'])}</strong> of the map "
            f"({fmt_int(model['prod_total'])} {UNIT}). The map is test-dominated by construction."
        )
    return (
        f"Production is <strong>{fmt_share_exact(model['prod_share'])}</strong> of scanned static "
        f"cost; <code>{esc(top['label'])}</code> alone is "
        f"<strong>{fmt_share_exact(top['share'])}</strong> and the top six components hold "
        f"<strong>{fmt_share_exact(model['top_six_share'])}</strong>."
    )


def cell_title(cell: dict, model: dict) -> str:
    if cell["kind"] == "other":
        return (
            f"{cell['label']} \u2014 residual \u2014 {fmt_int(cell['cost'])} {UNIT} \u00b7 "
            f"{fmt_share_exact(cell['share'])} of scanned static cost \u00b7 "
            f"{cell['members']} file{'s' if cell['members'] != 1 else ''} \u00b7 "
            f"omitted by the top-{TOP_CELLS} cell cap"
        )
    return (
        f"{cell['label']} \u2014 {cell['class']} \u2014 {fmt_int(cell['cost'])} {UNIT} \u00b7 "
        f"{fmt_share_exact(cell['share'])} of scanned static cost \u00b7 "
        f"{cell['members']} file{'s' if cell['members'] != 1 else ''} \u00b7 "
        f"{cell['mult']} median component \u00b7 top {cell['top_pct']}% of components"
    )


def cell_text(cell: dict) -> list[tuple[str, str, float]]:
    """Return (kind, text, font_size) lines for a cell, if the area allows."""
    width, height, area = cell["w"], cell["h"], cell["w"] * cell["h"]
    hero = area >= 80000 and width >= 300
    path_size = 13.5 if hero else 11.5
    meta_size = 10.0 if hero else 9.5
    lines: list[tuple[str, str, float]] = []
    if area >= 3200 and height >= 24 and width >= 64:
        if width >= 110:
            path = trunc(cell["label"], max(6, int((width - 18) / 6.9)))
            lines.append(("path", path, path_size))
        else:
            short = cell["label"].rsplit("/", 1)[-1]
            lines.append(("path", trunc(short, max(4, int((width - 14) / 5.3))), 9.5))
        if width >= 200:
            lines.append(
                ("share", f"{fmt_share_spec(cell['share'])} of scanned static cost", meta_size)
            )
        else:
            lines.append(("share", f"{fmt_share_spec(cell['share'])} of map", meta_size))
        if area >= 12000 and width >= 150 and height >= 70:
            lines.append(("cost", f"{fmt_int(cell['cost'])} {UNIT}", meta_size))
    elif width >= 40 and height >= 10:
        short = "core" if cell["label"].startswith("core (root)") else cell["label"].rsplit("/", 1)[-1]
        lines.append(("compact", trunc(short, max(3, int((width - 12) / 5.2))), 8.5))
    return lines


def render_html(model: dict) -> str:
    cells = model["cells"]
    max_share = max((cell["share"] for cell in cells), default=1.0)
    svg: list[str] = [
        f'<svg class="treemap" viewBox="0 0 {CONTENT_W} {PANEL_H}" width="{CONTENT_W}" '
        f'height="{PANEL_H}" role="img" aria-label="{esc("Component weight treemap for " + model["repo"] + ": area equals static cost units (v2); colour uses the same scale; test and bench components are hatched. Candidate prototype, not shipped.")}">',
        '<defs><pattern id="hatch" width="8" height="8" patternUnits="userSpaceOnUse" '
        'patternTransform="rotate(45)"><rect width="8" height="8" fill="#ebe7df"/>'
        '<line x1="0" y1="0" x2="0" y2="8" stroke="#8d8577" stroke-width="1.1" '
        'opacity=".30"/></pattern></defs>',
        f'<rect x="0" y="0" width="{CONTENT_W}" height="{PANEL_H}" fill="#fbf9f4"/>',
    ]

    for cell in cells:
        x, y, w, h = cell["x"], cell["y"], cell["w"], cell["h"]
        production = cell["class"] == "production"
        fill = ramp(math.sqrt(cell["share"] / max_share)) if production else "url(#hatch)"
        text_color = "#fbf9f4" if production and not dark_text(fill) else "#202b35"
        stroke_width = 1 if w < 12 or h < 12 else 2
        cell_class = f"cell class-{cell['class']}"
        if cell.get("hot_rank"):
            cell_class += f" hot hot-{cell['hot_rank']}"
        title = cell_title(cell, model)
        svg.append(
            f'<g class="{cell_class}" style="--i:{cell["reveal"]}"><title>{esc(title)}</title>'
        )
        svg.append(
            f'<rect class="face" x="{x}" y="{y}" width="{w}" height="{h}" fill="{fill}" '
            f'stroke="#fbf9f4" stroke-width="{stroke_width}" shape-rendering="crispEdges"/>'
        )
        non_production = not production
        if non_production:
            halo = ' paint-order="stroke" stroke="#fbf9f4" stroke-width="2.4" stroke-linejoin="round"'
        else:
            halo = ""
        lines = cell_text(cell)
        if lines and lines[0][0] == "compact":
            ty = y + (13 if h >= 16 else max(9, h - 2))
        elif lines and lines[0][2] >= 13:
            ty = y + 24
        else:
            ty = y + 21
        for kind, text, size in lines:
            weight = 700 if kind in ("path", "compact") else 500
            opacity = ".94" if kind != "compact" else ".90"
            svg.append(
                f'<text x="{x + 10}" y="{ty}" font-size="{size}" font-weight="{weight}" '
                f'fill="{text_color}" opacity="{opacity}"'
                f'{halo if non_production else ""}>{esc(text)}</text>'
            )
            if kind == "path":
                ty += 18 if size >= 13 else (15 if size == 11.5 else 14)
            elif kind in ("share", "cost"):
                ty += 15 if size >= 10 else 14
        if non_production and cell["kind"] != "other" and w >= 80 and h >= 90 and w * h >= 6000:
            pill_w = len(cell["class"]) * 7 + 12
            pill_y = y + h - 20
            svg.append(
                f'<rect x="{x + 9}" y="{pill_y}" width="{pill_w}" height="13" rx="2" '
                f'fill="#fbf9f4" fill-opacity=".88" stroke="#d6d0c4" stroke-width=".6"/>'
                f'<text x="{x + 15}" y="{pill_y + 9.6}" font-size="8.5" font-weight="800" '
                f'letter-spacing=".08em" fill="#6f6a61">{esc(cell["class"].upper())}</text>'
            )
        if cell.get("hot_rank"):
            sw = {1: 2.5, 2: 1.6, 3: 1.3}[cell["hot_rank"]]
            ring_color = "#c2410c" if cell["hot_rank"] == 1 else "#202b35"
            if w >= 40 and h >= 30:
                svg.append(
                    f'<rect x="{x + sw / 2}" y="{y + sw / 2}" width="{max(1, w - sw)}" '
                    f'height="{max(1, h - sw)}" fill="none" stroke="{ring_color}" '
                    f'stroke-width="{sw}" stroke-opacity="{0.92 if cell["hot_rank"] == 1 else 0.52}"/>'
                )
            badge_fill = "#c2410c" if cell["hot_rank"] == 1 else "#202b35"
            if w >= 26 and h >= 24:
                cx, cy = x + w - 13, y + 13
                svg.append(f'<circle cx="{cx}" cy="{cy}" r="9" fill="{badge_fill}"/>')
                svg.append(
                    f'<text x="{cx}" y="{cy + 3.4}" text-anchor="middle" font-size="9" '
                    f'font-weight="800" fill="#fbf9f4">{cell["hot_rank"]}</text>'
                )
            elif w >= 14 and h >= 10:
                bx, by = x + w - 14, y + 1
                svg.append(
                    f'<rect x="{bx}" y="{by}" width="12" height="9" rx="2" fill="{badge_fill}"/>'
                )
                svg.append(
                    f'<text x="{bx + 6}" y="{by + 6.9}" text-anchor="middle" font-size="8.5" '
                    f'font-weight="800" fill="#fbf9f4">{cell["hot_rank"]}</text>'
                )
        svg.append("</g>")
    svg.append("</svg>")

    swatches = "".join(
        f'<i class="sw" style="background:{ramp(t)}"></i>' for t in (0.0, 0.25, 0.5, 0.75, 1.0)
    )
    parse = model["parse"]
    if model["residual"] > 0:
        cap_note = f"top-50 cap: {len(cells) - 1} cells + residual other"
    else:
        cap_note = f"top-50 cap not reached: {len(cells)} of {model['components_with_cost']} components shown"
    production_note = (
        f"production {fmt_int(model['prod_total'])} of {fmt_int(model['total'])} {UNIT}"
    )
    return f'''<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>F candidate \u00b7 component weight treemap \u00b7 {esc(model['repo'])} \u2014 candidate prototype, not shipped</title>
<style>{CSS}</style>
</head>
<body>
<main class="sheet">
  <header class="head">
    <div class="kicker"><span class="dot"></span>Atlas \u00b7 below-graph weight slot <span class="slash">/</span> step 12 <span class="slash">/</span> F</div>
    <div class="status-chip">candidate prototype, not shipped</div>
    <div class="title-row">
      <h1>Component weight map</h1>
      <span class="repo">{esc(model['repo'])}</span>
    </div>
    <p class="deck"><span class="read">READ</span>{read_line(model)}</p>
  </header>

  <section class="panel" aria-label="F treemap candidate panel">
    {''.join(svg)}
  </section>

  <div class="cap">
    <p class="cap-main">area = static cost units (v2); colour = the same scale</p>
    <p class="cap-anchor">anchor = \u00d7 median component (<span class="fig">{fmt_int(model['median_component'])} {UNIT}</span>) \u00b7 top P% of components \u00b7 cell title carries the full identifier, class, share, file count and rank</p>
  </div>

  <div class="legend" aria-label="Legend">
    <span class="legend-label">production</span>
    {swatches}
    <span class="legend-note">lighter \u2192 heavier</span>
    <span class="legend-sep"></span>
    <span class="hatchbox"></span>
    <span class="legend-note">test / bench / residual = neutral hatch, class-labelled</span>
    <span class="legend-sep"></span>
    <span class="badge-demo">1</span>
    <span class="legend-note">top-3 cost rank</span>
  </div>

  <p class="meta">
    <span class="fig">{model['n_files']}</span> files \u00b7
    <span class="fig">{fmt_int(model['n_functions'])}</span> functions \u00b7
    <span class="fig">{fmt_int(model['n_findings'])}</span> findings \u00b7
    <span class="fig">{model['components_with_cost']}</span> components with cost ({model['components_total']} in the scan) \u00b7
    {esc(cap_note)} \u00b7 {esc(production_note)} \u00b7
    parse <span class="fig">{parse.get('ok', 0)} ok / {parse.get('partial', 0)} partial / {parse.get('failed', 0)} failed</span>
  </p>

  <p class="status">candidate prototype, not shipped \u00b7 F remains undecided \u00b7 the Explorer treemap is untouched \u00b7 static heuristic, not a measurement</p>
</main>
</body>
</html>
'''


CSS = """
:root{
  --paper:#f4f1ea;
  --panel:#fbf9f4;
  --ink:#202b35;
  --entry:#1f2d3a;
  --muted:#6f6a61;
  --hairline:#d6d0c4;
  --production:#3f6d6a;
  --hot:#c2410c;
  --neutral:#8d8577;
  --sans:ui-sans-serif,-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;
  --mono:ui-monospace,"SFMono-Regular",Menlo,Consolas,"Liberation Mono",monospace;
}
*{box-sizing:border-box}
html,body{margin:0;padding:0}
body{
  background:var(--paper);
  color:var(--ink);
  font-family:var(--sans);
  -webkit-font-smoothing:antialiased;
  text-rendering:optimizeLegibility;
}
.sheet{
  width:1440px;
  min-height:900px;
  margin:0 auto;
  padding:30px 16px 0;
  background:
    radial-gradient(1100px 460px at 50% -14%, rgba(255,255,255,.50), rgba(255,255,255,0) 72%),
    var(--paper);
}
.head{
  display:grid;
  grid-template-columns:1fr auto;
  column-gap:24px;
  row-gap:10px;
  border-bottom:1px solid var(--hairline);
  padding-bottom:15px;
}
.kicker{
  display:flex;
  align-items:center;
  gap:8px;
  min-width:0;
  font-size:9px;
  font-weight:800;
  letter-spacing:.18em;
  text-transform:uppercase;
  color:var(--muted);
}
.kicker .dot{width:6px;height:6px;border-radius:50%;background:var(--hot);flex:0 0 auto}
.kicker .slash{color:#a79f92;font-weight:650;letter-spacing:.08em}
.status-chip{
  align-self:start;
  white-space:nowrap;
  padding:5px 9px;
  border:1px solid #e5c9b9;
  border-radius:2px;
  background:#fbefe8;
  color:#a63c0c;
  font-size:9px;
  font-weight:800;
  letter-spacing:.11em;
  text-transform:uppercase;
}
.title-row{
  grid-column:1 / -1;
  display:flex;
  align-items:baseline;
  gap:14px;
  min-width:0;
}
h1{
  margin:0;
  font-size:32px;
  line-height:1.04;
  font-weight:800;
  letter-spacing:-.62px;
  color:var(--ink);
}
.repo{
  border-left:1px solid var(--hairline);
  padding-left:14px;
  font-family:var(--mono);
  font-size:20px;
  font-weight:600;
  letter-spacing:-.2px;
  color:var(--production);
}
.deck{
  grid-column:1 / -1;
  margin:0;
  max-width:1190px;
  font-size:13.5px;
  line-height:1.5;
  color:var(--muted);
}
.deck strong{color:var(--ink);font-family:var(--mono);font-size:.94em;font-weight:750}
.deck code{
  font-family:var(--mono);
  font-size:12px;
  color:var(--entry);
  background:#ece7dd;
  border-radius:2px;
  padding:1px 4px;
}
.read{
  display:inline-block;
  margin-right:8px;
  padding:2px 5px;
  border:1px solid #e5c9b9;
  border-radius:2px;
  background:#fbefe8;
  color:var(--hot);
  font-size:8.5px;
  font-weight:800;
  letter-spacing:.14em;
  vertical-align:1px;
}
.panel{
  margin-top:14px;
  background:var(--panel);
  border-radius:3px;
  box-shadow:
    inset 0 0 0 1px var(--hairline),
    0 1px 0 rgba(32,43,53,.035),
    0 18px 36px -30px rgba(32,43,53,.42);
  overflow:hidden;
}
.treemap{display:block;width:1408px;height:260px;background:var(--panel)}
.treemap text{
  font-family:var(--mono);
  font-variant-numeric:tabular-nums;
  font-kerning:none;
}
.cell{animation:cellIn .30s ease-out both;animation-delay:calc(60ms + var(--i) * 22ms)}
.cell .face{transition:filter .16s ease}
.cell:hover .face{filter:brightness(1.06)}
.cap{
  display:flex;
  flex-wrap:wrap;
  align-items:baseline;
  gap:4px 16px;
  margin-top:13px;
}
.cap-main{
  margin:0;
  font-size:12px;
  font-weight:750;
  color:var(--ink);
  letter-spacing:.005em;
}
.cap-anchor{
  margin:0;
  font-size:10.5px;
  line-height:1.45;
  color:var(--muted);
}
.fig{font-family:var(--mono);font-variant-numeric:tabular-nums;font-size:.96em}
.legend{
  display:flex;
  align-items:center;
  flex-wrap:wrap;
  gap:6px;
  margin-top:10px;
  font-size:10px;
  color:var(--muted);
}
.legend-label{
  font-size:9px;
  font-weight:800;
  letter-spacing:.13em;
  text-transform:uppercase;
  color:var(--ink);
}
.sw{
  display:inline-block;
  width:15px;
  height:9px;
  border-radius:1px;
  border:1px solid rgba(32,43,53,.13);
}
.legend-note{line-height:1.35}
.legend-sep{width:1px;height:12px;background:var(--hairline);margin:0 5px}
.hatchbox{
  display:inline-block;
  width:15px;
  height:9px;
  border-radius:1px;
  border:1px solid rgba(32,43,53,.13);
  background:#ebe7df;
  background-image:repeating-linear-gradient(45deg,rgba(141,133,119,0) 0 3px,rgba(141,133,119,.55) 3px 4px);
}
.badge-demo{
  display:inline-flex;
  align-items:center;
  justify-content:center;
  width:16px;
  height:16px;
  border-radius:50%;
  background:var(--hot);
  color:var(--panel);
  font-family:var(--mono);
  font-size:9px;
  font-weight:800;
}
.meta{
  margin:11px 0 0;
  padding-top:8px;
  border-top:1px solid var(--hairline);
  font-size:10.5px;
  line-height:1.5;
  color:var(--muted);
}
.status{
  margin:8px 0 0;
  font-size:9.5px;
  line-height:1.4;
  color:#857d70;
  letter-spacing:.01em;
}
@keyframes cellIn{from{opacity:0}to{opacity:1}}
@keyframes rise{from{opacity:0;transform:translateY(5px)}to{opacity:1;transform:none}}
.head,.cap,.legend,.meta,.status{animation:rise .42s ease-out both}
.cap{animation-delay:280ms}
.legend{animation-delay:340ms}
.meta{animation-delay:400ms}
.status{animation-delay:440ms}
@media (prefers-reduced-motion:reduce){
  *,*::before,*::after{animation:none!important;transition:none!important}
}
@media print{
  body{background:#fff}
  .sheet{width:auto;min-height:0;padding:0}
  .treemap{width:100%;height:auto}
}
"""


def png_size(path: Path) -> tuple[int, int]:
    head = path.read_bytes()[:24]
    if head[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"FATAL: {path} is not a PNG")
    return struct.unpack(">II", head[16:24])


def render_screenshots() -> None:
    try:
        from playwright.sync_api import sync_playwright
    except Exception as exc:  # pragma: no cover - optional evidence path
        print(f"skip screenshots: Playwright unavailable ({exc})")
        return

    pinned = Path("/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome")
    try:
        with sync_playwright() as playwright:
            if pinned.exists():
                browser = playwright.chromium.launch(executable_path=str(pinned))
            else:
                browser = playwright.chromium.launch()
            page = browser.new_page(
                viewport={"width": PAGE_W, "height": 900}, device_scale_factor=1
            )
            page.emulate_media(reduced_motion="reduce")
            for repo in INPUTS:
                target = HERE / f"f-treemap-{repo}.html"
                page.goto(target.as_uri())
                page.wait_for_load_state("load")
                page.evaluate("() => document.fonts.ready.then(() => true)")
                page.wait_for_timeout(80)
                image = HERE / f"f-treemap-{repo}.png"
                page.screenshot(
                    path=str(image),
                    clip={"x": 0, "y": 0, "width": PAGE_W, "height": 900},
                )
                width, height = png_size(image)
                if (width, height) != (PAGE_W, 900):
                    raise SystemExit(
                        f"FATAL: {image.name} is {width}x{height}, expected {PAGE_W}x900"
                    )
                print(f"wrote {image.name} ({width}x{height})")
            browser.close()
    except SystemExit:
        raise
    except Exception as exc:  # pragma: no cover - optional evidence path
        print(f"skip screenshots: chromium unavailable ({exc})")


def main() -> int:
    models = {repo: build_model(repo) for repo in INPUTS}
    for repo, model in models.items():
        output = HERE / f"f-treemap-{repo}.html"
        with output.open("w", encoding="utf-8", newline="\n") as handle:
            handle.write(render_html(model))
        print(
            f"wrote {output.name}: {model['n_files']} files, "
            f"{model['components_with_cost']} components, {len(model['cells'])} cells, "
            f"production {fmt_int(model['prod_total'])}/{fmt_int(model['total'])} {UNIT}"
        )
    render_screenshots()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
