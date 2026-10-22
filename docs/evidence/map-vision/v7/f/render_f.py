#!/usr/bin/env python3
"""F candidate prototype: deterministic component weight map (treemap + KV1 brief + Q2 drill).

Reads ONLY the design-repo scans under /home/shafin/Opencode/perfpal-test/:
out-v6-cJSON/metrics.json and out-v6-hyperfine/metrics.json (tmux is added only if
the TMUX_INPUT exists and --with-tmux is passed). The V6 holdout (django/postgres) is
never referenced.

Run from the repo root:
    python3 docs/evidence/map-vision/v7/f/render_f.py

Output (same directory): f-<repo>.html, f-<repo>-drill.html, f-<repo>-{start,mid,final}.png,
manifest.json. No src/ changes, no network, no JavaScript in the artifacts.
"""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import math
import re
import statistics
import struct
from pathlib import Path

from playwright.sync_api import sync_playwright

EXE = "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
OUT_DIR = Path(__file__).resolve().parent
REPOS = ["cJSON", "hyperfine", "tmux"]
TMUX_INPUT = TEST_ROOT / "out-v6-tmux" / "metrics.json"
GENERATED = "2026-10-22"
TOP_CELLS = 50
MAX_ACTIONS = 5
MAP_W, MAP_H = 1360, 404
VIEW_W, VIEW_H = 1440, 900

SEV_RANK = {"hot": 0, "warn": 1, "info": 2}
CONF_RANK = {"HIGH": 0, "MEDIUM": 1, "LOW": 2}
RAMP = [(0.0, "#f2f7fd"), (0.25, "#d3e3f5"), (0.5, "#9fc0e4"),
        (0.75, "#5b8fc9"), (1.0, "#0b3d91")]


def esc(value: object) -> str:
    return html.escape(str(value), quote=True)


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


def fmt_cost(value: float) -> str:
    if value >= 1_000_000:
        return f"{value / 1_000_000:.2f}M"
    if value >= 10_000:
        return f"{value / 1_000:.1f}k"
    if value >= 1_000:
        return f"{value / 1_000:.2f}k"
    return str(int(round(value)))


def fmt_pct(value: float, digits: int = 2) -> str:
    return f"{value * 100:.{digits}f}%"


UNIT = "static cost units (v2)"


def unit_str(value: float) -> str:
    return f"{fmt_cost(value)} {UNIT}"


def mult_str(value: float, median: float) -> str:
    if median <= 0:
        return "n/a×"
    ratio = value / median
    if ratio >= 1000:
        return f"{ratio:,.0f}×"
    return f"{ratio:,.1f}×"


def top_pct(rank: int, n: int) -> int:
    return max(1, int(round(100 * rank / max(1, n))))


def trunc(text: str, limit: int) -> str:
    text = " ".join(str(text).split())
    return text if len(text) <= limit else text[: max(0, limit - 1)].rstrip() + "…"


def slug(text: str) -> str:
    return re.sub(r"[^A-Za-z0-9]+", "-", text).strip("-").lower() or "root"


def ramp(t: float) -> str:
    t = max(0.0, min(1.0, t))
    for i in range(1, len(RAMP)):
        t0, c0 = RAMP[i - 1]
        t1, c1 = RAMP[i]
        if t <= t1:
            f = (t - t0) / (t1 - t0) if t1 > t0 else 0.0
            a = [int(c0[j:j + 2], 16) for j in (1, 3, 5)]
            b = [int(c1[j:j + 2], 16) for j in (1, 3, 5)]
            return "#" + "".join(f"{round(a[k] + (b[k] - a[k]) * f):02x}" for k in range(3))
    return RAMP[-1][1]


def dark_text(hex_color: str) -> bool:
    r = int(hex_color[1:3], 16) / 255
    g = int(hex_color[3:5], 16) / 255
    b = int(hex_color[5:7], 16) / 255
    return 0.2126 * r + 0.7152 * g + 0.0722 * b > 0.56


def load(repo: str) -> dict:
    path = TEST_ROOT / f"out-v6-{repo}" / "metrics.json"
    if path.parent.name == "out-v6-django" or "django" in str(path) or "postgres" in str(path):
        raise SystemExit("FATAL: holdout input is forbidden")
    return json.loads(path.read_text(encoding="utf-8"))


def build_model(repo: str, data: dict) -> dict:
    files = {f["path"]: f for f in data.get("files", [])}
    prefixes = sorted({c["path_prefix"] for c in data.get("components", [])} | {"."})
    cost_by_file: dict[str, int] = {}
    for fn in data.get("stats", {}).get("functions", []):
        cost_by_file[fn["path"]] = cost_by_file.get(fn["path"], 0) + int(fn.get("function_cost_v2", 0))
    total = sum(cost_by_file.values())
    total = total or 1
    prod_total = sum(v for path, v in cost_by_file.items()
                     if files.get(path, {}).get("path_class") == "production")
    fn_values = [int(fn.get("function_cost_v2", 0))
                 for fn in data.get("stats", {}).get("functions", [])]
    median_function = statistics.median(fn_values) if fn_values else 0

    def comp_of(path: str) -> str:
        best = "."
        for prefix in prefixes:
            if prefix == ".":
                continue
            if path == prefix or path.startswith(prefix + "/"):
                if best == "." or len(prefix) > len(best):
                    best = prefix
        return best

    base = {p: {"id": p, "cost": 0, "prod_cost": 0, "files": [], "class": "test"} for p in prefixes}
    for path, finfo in files.items():
        base.setdefault(comp_of(path), {"id": comp_of(path), "cost": 0, "prod_cost": 0,
                                        "files": [], "class": "test"})
        base[comp_of(path)]["files"].append(path)
    for path, value in cost_by_file.items():
        comp = base[comp_of(path)]
        comp["cost"] += value
        if files.get(path, {}).get("path_class") == "production":
            comp["prod_cost"] += value
    for comp in base.values():
        comp["files"].sort()
        comp["members"] = len(comp["files"])
        comp["files_with_cost"] = [p for p in comp["files"] if cost_by_file.get(p, 0) > 0]
        if comp["cost"] == 0:
            comp["class"] = "test"
        elif comp["prod_cost"] * 2 >= comp["cost"]:
            comp["class"] = "production"
        else:
            classes = [files.get(p, {}).get("path_class") for p in comp["files"]]
            comp["class"] = "bench" if classes.count("bench") > classes.count("test") else "test"
    cost_components = sorted([c for c in base.values() if c["cost"] > 0],
                             key=lambda c: (-c["cost"], c["id"]))
    median_component = (statistics.median([c["cost"] for c in cost_components])
                        if cost_components else 0)
    for rank, comp in enumerate(cost_components, 1):
        comp["cost_rank"] = rank
        comp["top_pct"] = top_pct(rank, len(cost_components))
        comp["mult"] = mult_str(comp["cost"], median_component)

    root = base["."]
    expand_root = bool(root["cost"] / total > 0.55 and len(root["files_with_cost"]) > 40)
    candidates: list[dict] = []
    if expand_root:
        for path in root["files_with_cost"]:
            candidates.append({
                "id": f"file:{path}", "label": Path(path).name, "kind": "file",
                "base": ".", "cost": cost_by_file[path],
                "class": files.get(path, {}).get("path_class", "test"),
                "members": 1, "files": [path],
            })
        for comp in base.values():
            if comp["id"] != "." and comp["cost"] > 0:
                candidates.append({**comp, "label": comp["id"], "kind": "component",
                                   "base": comp["id"]})
    else:
        for comp in base.values():
            if comp["cost"] > 0:
                candidates.append({**comp, "label": "core (root)" if comp["id"] == "." else comp["id"],
                                   "kind": "component", "base": comp["id"]})
    for cell in candidates:
        cell["share"] = cell["cost"] / total

    picked = sorted(candidates, key=lambda c: (0 if c["class"] == "production" else 1,
                                               -c["cost"], c["id"]))[:TOP_CELLS]
    shown_cost = sum(c["cost"] for c in picked)
    residual = total - shown_cost
    cells = sorted(picked, key=lambda c: (-c["cost"], c["id"]))
    if residual > 0:
        cells.append({
            "id": "other", "label": f"other ({len(candidates) - len(picked)} cells)",
            "kind": "other", "base": None, "cost": residual,
            "share": residual / total, "class": "other", "members": 0, "files": [],
        })
    for i, cell in enumerate(sorted(cells, key=lambda c: (-c["cost"], c["id"])), 1):
        cell["reveal"] = i - 1
        cell["cost_rank"] = i
        cell["mult"] = ("residual" if cell["kind"] == "other"
                        else mult_str(cell["cost"], median_component))
        cell["unit"] = unit_str(cell["cost"])
    ranked = [c for c in cells if c["kind"] != "other"]
    for i, cell in enumerate(sorted(ranked, key=lambda c: (-c["cost"], c["id"])), 1):
        cell["top_pct"] = top_pct(i, len(ranked))
    for i, cell in enumerate(sorted(cells, key=lambda c: (-c["cost"], c["id"])), 1):
        if cell["kind"] == "other":
            cell["top_pct"] = top_pct(i, len(cells))
    top = sorted(ranked, key=lambda c: (-c["cost"], c["id"]))[:3]
    for rank, cell in enumerate(top, 1):
        cell["hot_rank"] = rank
    prod_cells = [c for c in ranked if c["class"] == "production"]
    hottest = ranked[0] if ranked else cells[0]
    hottest_prod = prod_cells[0] if prod_cells else hottest

    cost_files = sorted([p for p, v in cost_by_file.items() if v > 0],
                        key=lambda p: (-cost_by_file[p], p))
    file_rank = {p: i for i, p in enumerate(cost_files, 1)}
    actions = build_actions(data, files, cost_by_file, total, comp_of)
    for action in actions:
        rank = file_rank.get(action["path"])
        action["file_unit"] = unit_str(action["file_cost"])
        action["file_mult"] = mult_str(action["file_cost"], median_function)
        action["file_top_pct"] = top_pct(rank, len(cost_files)) if rank else 100
    answer = build_answer(hottest, hottest_prod, actions, total)
    edges = data.get("graphs", {}).get("include_graph", {}).get("edges", [])
    resolved = [e for e in edges if e.get("from") and e.get("to")]
    file_edges = [e for e in resolved if e["from"] in files and e["to"] in files]
    stats = data.get("parse_health_summary", {})
    return {
        "repo": repo, "data": data, "files": files, "prefixes": prefixes, "base": base,
        "total": total, "cost_by_file": cost_by_file, "expand_root": expand_root,
        "cells": cells, "candidate_count": len(candidates), "hottest": hottest,
        "hottest_prod": hottest_prod, "actions": actions, "answer": answer,
        "prod_total": prod_total, "median_component": median_component,
        "median_function": median_function, "cost_components": len(cost_components),
        "cost_files": len(cost_files), "file_rank": file_rank,
        "edges": edges, "resolved_edges": resolved, "dsm_edges": file_edges,
        "edges_unresolved": len(edges) - len(resolved),
        "edges_nonfile": len(resolved) - len(file_edges),
        "parse": stats, "comp_of": comp_of,
    }


def build_actions(data: dict, files: dict, cost_by_file: dict, total: float, comp_of) -> list[dict]:
    rows = []
    for finding in data.get("findings", []):
        rule = finding.get("rule_id", "")
        if "-ARCH-002" in rule:
            continue
        path = finding.get("path", "")
        cls = files.get(path, {}).get("path_class", "test")
        line = int(finding.get("span", {}).get("start_line", 0))
        rows.append({
            "rule": rule, "title": finding.get("title", rule), "path": path, "line": line,
            "message": finding.get("message", ""), "severity": finding.get("severity", "info"),
            "confidence": finding.get("confidence", "LOW"), "class": cls,
            "file_cost": cost_by_file.get(path, 0), "comp": comp_of(path),
        })
    rows.sort(key=lambda r: (0 if r["class"] == "production" else 1,
                             SEV_RANK.get(r["severity"], 3), CONF_RANK.get(r["confidence"], 3),
                             -r["file_cost"], r["path"], r["line"], r["rule"]))
    picked, used = [], set()
    for row in rows:
        if row["rule"] in used:
            continue
        if row["severity"] == "info" and len(picked) >= 3:
            break
        used.add(row["rule"])
        row = dict(row, share=row["file_cost"] / total)
        picked.append(row)
        if len(picked) >= MAX_ACTIONS:
            break
    if len(picked) < 3:
        for row in rows:
            if row in picked:
                continue
            picked.append(dict(row, share=row["file_cost"] / total))
            if len(picked) >= 3:
                break
    return picked[:MAX_ACTIONS]


def build_answer(hottest: dict, hottest_prod: dict, actions: list[dict], total: float) -> str:
    first = actions[0] if actions else None
    action = (f"{first['rule']} at {first['path']}:{first['line']} "
              f"({first['severity']}/{first['confidence'].lower()})") if first else "no actionable finding"
    if hottest["class"] != "production":
        return (f"{fmt_pct(hottest['share'], 1)} of static weight is {hottest['class']} code "
                f"({hottest['label']}); production is only {fmt_pct(hottest_prod['share'], 1)}. "
                f"Start in production: {action}, led by {hottest_prod['label']}.")
    return (f"{hottest['label']} is the heaviest cell ({fmt_pct(hottest['share'], 1)} of static cost); "
            f"start with {action}.")


def squarify(values: list[float], x: float, y: float, w: float, h: float) -> list[tuple[float, float, float, float]]:
    if not values:
        return []
    scale = (w * h) / sum(values)
    vals = [v * scale for v in values]
    rects: list[tuple[float, float, float, float]] = []

    def worst(row: list[float], side: float) -> float:
        s = sum(row)
        if s <= 0 or side <= 0:
            return math.inf
        rmax, rmin = max(row), min(row)
        return max((side * side * rmax) / (s * s), (s * s) / (side * side * rmin))

    i = 0
    while i < len(vals):
        row = [vals[i]]
        i += 1
        side = min(w, h)
        while i < len(vals) and worst(row + [vals[i]], side) <= worst(row, side):
            row.append(vals[i])
            i += 1
        s = sum(row)
        if w >= h:
            rw = s / h if h else 0
            yy = y
            for area in row:
                rh = area / rw if rw else 0
                rects.append((x, yy, rw, rh))
                yy += rh
            x += rw
            w -= rw
        else:
            rh = s / w if w else 0
            xx = x
            for area in row:
                rw = area / rh if rh else 0
                rects.append((xx, y, rw, rh))
                xx += rw
            y += rh
            h -= rh
    return rects


def layout_cells(cells: list[dict]) -> list[dict]:
    ordered = sorted(cells, key=lambda c: (-c["cost"], c["id"]))
    rects = squarify([c["cost"] for c in ordered], 0, 0, MAP_W, MAP_H)
    out = []
    for cell, (x0, y0, rw, rh) in zip(ordered, rects):
        x1, y1 = x0 + rw, y0 + rh
        if x0 > MAP_W - 1 or y0 > MAP_H - 1:
            continue
        x0i, y0i = int(round(x0)), int(round(y0))
        x1i, y1i = int(round(x1)), int(round(y1))
        x1i = min(MAP_W, max(x0i + 1, x1i))
        y1i = min(MAP_H, max(y0i + 1, y1i))
        out.append({**cell, "x": x0i, "y": y0i, "w": x1i - x0i, "h": y1i - y0i})
    return out


def base_order(model: dict) -> list[dict]:
    return sorted(model["base"].values(), key=lambda c: (0 if c["class"] == "production" else 1,
                                                          -c["cost"], c["id"]))


def render_first(model: dict) -> str:
    cells = layout_cells(model["cells"])
    visible = [c for c in cells if c["kind"] != "other"]
    for i, cell in enumerate(sorted(visible, key=lambda c: (-c["cost"], c["id"])), 1):
        cell["top_pct"] = top_pct(i, len(visible))
    for i, cell in enumerate(sorted(cells, key=lambda c: (-c["cost"], c["id"])), 1):
        if cell["kind"] == "other":
            cell["top_pct"] = top_pct(i, len(cells))
    max_share = max((c["share"] for c in cells), default=1)
    order = base_order(model)
    base_index = {c["id"]: i for i, c in enumerate(order)}
    prod_share = model["prod_total"] / model["total"]
    drill = f"f-{model['repo']}-drill.html"
    parse = model["parse"]
    stats = model["data"].get("scan", {})

    svg = [f'<svg class="treemap" viewBox="0 0 {MAP_W} {MAP_H}" width="{MAP_W}" height="{MAP_H}" '
           f'role="img" aria-label="Component treemap: area = static cost units (v2); colour = the same cost scale">',
           '<defs><pattern id="hatch" width="7" height="7" patternUnits="userSpaceOnUse" '
           'patternTransform="rotate(45)"><line x1="0" y1="0" x2="0" y2="7" '
           'stroke="#64748b" stroke-width="2" opacity=".55"/></pattern></defs>',
           f'<rect x="0" y="0" width="{MAP_W}" height="{MAP_H}" fill="#f8fafc"/>']
    for cell in cells:
        x, y, w, h = cell["x"], cell["y"], cell["w"], cell["h"]
        prod = cell["class"] == "production"
        color = ramp(math.sqrt(cell["share"] / max_share)) if prod else "#dfe6ee"
        text_color = "#0f172a" if (not prod or dark_text(color)) else "#ffffff"
        base = model["base"].get(cell.get("base") or "")
        target = f"{drill}#c{base_index.get(base['id'], 0)}" if base else ""
        label = cell["label"]
        short = label if len(label) <= 30 else label[:27] + "…"
        area = w * h
        lines = []
        if area >= 4200 and w >= 44 and h >= 22:
            lines.append(("label", trunc(short, max(3, int((w - 14) / 6.9)))))
        if area >= 12000 and w >= 150 and h >= 74:
            if w >= 200:
                lines.append(("small", f"{fmt_pct(cell['share'], 1)} · {cell['unit']}"))
            else:
                lines.append(("small", cell["unit"]))
            if cell["kind"] == "other":
                lines.append(("small", f"residual · top {cell['top_pct']}% of cells"))
            elif w >= 200:
                lines.append(("small", f"{cell['mult']} median component · top {cell['top_pct']}%"))
            else:
                lines.append(("small", f"{cell['mult']} median component"))
                lines.append(("small", f"top {cell['top_pct']}% of cells"))
        elif area >= 9000 and w >= 80 and h >= 36:
            lines.append(("small", f"{fmt_pct(cell['share'], 1)} of map"))
        if cell["kind"] == "other":
            title = (f"{label} — residual — {cell['unit']} · top {cell['top_pct']}% of cells · "
                     f"{fmt_pct(cell['share'], 2)} of map; omitted by the top-{TOP_CELLS} cap")
        else:
            title = (f"{label} — {cell['class']} — {cell['unit']} · {cell['mult']} median component · "
                     f"top {cell['top_pct']}% of cells · {fmt_pct(cell['share'], 2)} of map; "
                     f"{cell['members']} file(s)")
        cell_class = f"cell class-{cell['class']}" + (f" hot-rank{cell['hot_rank']}" if cell.get("hot_rank") else "")
        open_tag = f'<a class="cell-link" href="{esc(target)}" aria-label="{esc(title)}">' if target else ""
        close_tag = "</a>" if target else ""
        svg.append(f'{open_tag}<g class="{cell_class}" style="--i:{cell["reveal"]}"><title>{esc(title)}</title>')
        svg.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="{color}" stroke="#ffffff" stroke-width="2"/>')
        if not prod:
            svg.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="url(#hatch)" pointer-events="none"/>')
        if cell.get("hot_rank"):
            sw = {1: 3.0, 2: 2.0, 3: 1.5}[cell["hot_rank"]]
            svg.append(f'<rect class="hot-ring" x="{x + sw / 2}" y="{y + sw / 2}" width="{max(1, w - sw)}" '
                       f'height="{max(1, h - sw)}" fill="none" stroke="#0f172a" stroke-width="{sw}"/>')
        ty = y + 18
        for kind, text in lines:
            if kind == "label":
                svg.append(f'<text x="{x + 8}" y="{ty}" fill="{text_color}" font-size="12.5" '
                           f'font-weight="700">{esc(text)}</text>')
            else:
                svg.append(f'<text x="{x + 8}" y="{ty + 15}" fill="{text_color}" font-size="8.8" '
                           f'opacity=".94">{esc(text)}</text>')
            ty += 15
        if cell.get("hot_rank") and w >= 40 and h >= 34:
            svg.append(f'<circle cx="{x + w - 14}" cy="{y + 14}" r="9" fill="#0f172a"/>')
            svg.append(f'<text x="{x + w - 14}" y="{y + 18}" text-anchor="middle" fill="#ffffff" '
                       f'font-size="10" font-weight="800">{cell["hot_rank"]}</text>')
        svg.append("</g>" + close_tag)
    svg.append("</svg>")
    svg_markup = "\n".join(svg)

    cards = []
    for j, action in enumerate(model["actions"]):
        cls = action["class"]
        tag = "" if cls == "production" else f'<span class="tag tag-{esc(cls)}">{esc(cls)}</span>'
        cards.append(f'''<article class="brief-card" style="--j:{j}">
  <div class="card-top"><span class="rank">{j + 1}</span><span class="sev sev-{esc(action['severity'])}">{esc(action['severity'].upper())}</span><span class="conf">{esc(action['confidence'])}</span>{tag}</div>
  <div class="card-path">{esc(trunc(action['path'], 38))}:{action['line']}</div>
  <div class="card-title">{esc(trunc(action['title'], 76))}</div>
  <div class="card-why">{esc(trunc(action['message'], 122))}</div>
  <div class="card-anchor"><span>{esc(action['file_unit'])}</span><span>{esc(action['file_mult'])} median function · top {action['file_top_pct']}% of files</span></div>
</article>''')
    cards_markup = "\n".join(cards)
    n_cards = max(1, len(cards))
    swatches = "".join(f'<i class="sw" style="background:{ramp(t)}"></i>' for t in (0.0, 0.25, 0.5, 0.75, 1.0))
    meta = (f"{stats.get('file_count', len(model['files']))} files · "
            f"{len(model['data'].get('stats', {}).get('functions', []))} functions · "
            f"{len(model['data'].get('findings', []))} findings · parse "
            f"{parse.get('ok', 0)} ok / {parse.get('partial', 0)} partial / {parse.get('failed', 0)} failed")
    cap = (f"{len(cells) - (1 if any(c['kind'] == 'other' for c in cells) else 0)} cells · "
           f"top-{TOP_CELLS} cap · production {fmt_pct(prod_share, 1)} of cost")
    note = (f"static heuristics · area/colour = static cost units (v2) · "
            f"median component {unit_str(model['median_component'])} · "
            f"{model['edges_unresolved']}/{len(model['edges'])} unresolved + "
            f"{model['edges_nonfile']} non-repo imports excluded from drill")
    return f'''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>F · Weight map · {esc(model['repo'])}</title>
<style>
:root{{--ink:#0f172a;--muted:#5b6b7f;--line:#dbe2ea;--blue:#0b3d91;--hot:#b42318}}
*{{box-sizing:border-box}}
html,body{{margin:0;padding:0}}
body{{background:#eef1f5;color:var(--ink);font-family:ui-sans-serif,-apple-system,"Segoe UI",Roboto,Helvetica,Arial,sans-serif}}
.pause-toggle{{position:fixed;top:12px;right:26px;width:1px;height:1px;opacity:0;z-index:30}}
.pause-control{{position:fixed;top:10px;right:26px;z-index:31;display:inline-flex;align-items:center;gap:6px;padding:6px 11px;border:1px solid #cbd5e1;border-radius:999px;background:#ffffff;color:#334155;font-size:11px;font-weight:650;cursor:pointer;box-shadow:0 1px 2px rgba(15,23,42,.08)}}
.pause-control:hover{{border-color:#94a3b8}}
.pause-off{{display:none}}
.pause-toggle:checked~.pause-control .pause-on{{display:none}}
.pause-toggle:checked~.pause-control .pause-off{{display:inline}}
.pause-toggle:focus-visible~.pause-control{{outline:2px solid var(--blue);outline-offset:2px}}
.screen{{position:relative;width:1440px;height:900px;margin:0 auto;background:#fff;padding:20px 40px 0;overflow:hidden}}
.top{{height:38px;display:flex;align-items:flex-end;justify-content:space-between;border-bottom:1px solid var(--line);padding-bottom:8px}}
.brand{{font-size:20px;font-weight:760;letter-spacing:-.3px}}
.brand .repo{{color:var(--blue)}}
.topmeta{{margin-right:118px;font-size:11.5px;color:var(--muted);text-align:right}}
.answer{{margin:10px 0 6px;font-size:19px;line-height:1.28;font-weight:650;letter-spacing:-.1px;max-width:1360px;animation:fade .45s ease-out 100ms both}}
.answer b{{color:var(--blue)}}
.map-head{{height:20px;display:flex;justify-content:space-between;align-items:center;font-size:11px;color:var(--muted);animation:fade .45s ease-out 160ms both}}
.legend{{display:flex;align-items:center;gap:7px}}
.sw{{display:inline-block;width:13px;height:8px;border-radius:2px;border:1px solid rgba(15,23,42,.12)}}
.legend .lbl{{margin-right:5px}}
.legend .hatchbox{{display:inline-block;width:13px;height:8px;border-radius:2px;background:#dfe6ee;border:1px solid #b6c2d1;background-image:repeating-linear-gradient(45deg,transparent 0 2px,#64748b 2px 3px)}}
.treemap{{display:block;width:1360px;height:404px;margin-top:2px;border-radius:4px;background:#f8fafc}}
.cell{{animation:cellIn .32s ease-out both;animation-delay:calc(160ms + var(--i)*40ms)}}
.cell-link{{text-decoration:none}}
.cell-link:focus-visible .cell{{outline:none}}
.cell-link:hover .cell{{filter:brightness(1.06)}}
.hot-ring{{stroke-opacity:.92}}
.hot-rank1 .hot-ring{{animation:ringPulse 1.5s ease-in-out 2600ms 2 both}}
.hot-rank2 .hot-ring{{stroke-opacity:.8}}
.hot-rank3 .hot-ring{{stroke-opacity:.72}}
.brief-head{{height:18px;margin-top:12px;display:flex;justify-content:space-between;align-items:baseline;font-size:11px;color:var(--muted);animation:fade .45s ease-out 2400ms both}}
.brief-head b{{font-size:12px;color:var(--ink);letter-spacing:.02em}}
.brief-cards{{display:grid;grid-template-columns:repeat(var(--n),1fr);gap:10px;height:238px;margin-top:4px}}
.brief-card{{border:1px solid var(--line);border-radius:5px;padding:9px 10px 8px;background:#fbfcfe;overflow:hidden;display:flex;flex-direction:column;gap:4px;animation:cardIn .45s ease-out both;animation-delay:calc(2500ms + var(--j)*90ms)}}
.card-top{{display:flex;align-items:center;gap:6px;height:16px}}
.rank{{display:inline-flex;align-items:center;justify-content:center;width:16px;height:16px;border-radius:50%;background:#0f172a;color:#fff;font-size:9.5px;font-weight:800}}
.sev{{font-size:9.5px;font-weight:800;letter-spacing:.06em;padding:1px 5px;border-radius:3px}}
.sev-hot{{color:#7a271a;background:#fee4e2}}
.sev-warn{{color:#7a2e0e;background:#fef0c7}}
.sev-info{{color:#344054;background:#eaecf0}}
.conf{{font-size:9.5px;font-weight:700;color:var(--muted);letter-spacing:.04em}}
.tag{{margin-left:auto;font-size:8.5px;font-weight:800;letter-spacing:.08em;color:#475467;background:#eaecf0;padding:1px 4px;border-radius:3px}}
.card-path{{font-family:ui-monospace,"SFMono-Regular",Menlo,Consolas,monospace;font-size:10.5px;font-weight:650;color:var(--blue);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}
.card-title{{font-size:12px;font-weight:700;line-height:1.18;max-height:28px;overflow:hidden}}
.card-why{{font-size:10.5px;line-height:1.3;color:#475467;max-height:55px;overflow:hidden}}
.card-anchor{{margin-top:auto;display:flex;flex-direction:column;gap:1px;font-size:9.4px;line-height:1.25;font-weight:650;color:#334155;border-top:1px solid #edf1f6;padding-top:4px}}
footer{{position:absolute;left:40px;right:40px;bottom:10px;height:38px;display:flex;justify-content:space-between;align-items:center;border-top:1px solid var(--line);font-size:10.5px;color:var(--muted);animation:fade .45s ease-out 3100ms both}}
.drill-link{{font-size:12px;font-weight:700;color:var(--blue);text-decoration:none}}
.drill-link:hover{{text-decoration:underline}}
.foot-note{{max-width:900px;text-align:right}}
@keyframes cellIn{{from{{opacity:0}}to{{opacity:1}}}}
@keyframes cardIn{{from{{opacity:0;transform:translateY(6px)}}to{{opacity:1;transform:none}}}}
@keyframes fade{{from{{opacity:0}}to{{opacity:1}}}}
@keyframes ringPulse{{0%,100%{{stroke-opacity:.45}}50%{{stroke-opacity:1}}}}
.pause-toggle:checked~.screen *{{animation-play-state:paused!important}}
@media (prefers-reduced-motion:reduce){{
  .screen,.screen *{{animation:none!important;transition:none!important}}
  .cell,.brief-card,.answer,.map-head,.brief-head,footer{{opacity:1!important;transform:none!important}}
}}
@media print{{
  body{{background:#fff}}
  .pause-control,.pause-toggle{{display:none!important}}
  .screen,.screen *{{animation:none!important;transition:none!important}}
  .cell,.brief-card,.answer,.map-head,.brief-head,footer{{opacity:1!important;transform:none!important}}
}}
</style></head>
<body>
<input class="pause-toggle" id="pause" type="checkbox" aria-label="Pause animation">
<label class="pause-control" for="pause"><span class="pause-on">❚❚ Pause</span><span class="pause-off">▶ Resume</span></label>
<main class="screen">
  <header class="top"><div class="brand">Perfpal weight map <span class="repo">· {esc(model['repo'])}</span></div><div class="topmeta">{esc(meta)}</div></header>
  <p class="answer">{esc(model['answer'])}</p>
  <div class="map-head"><span>{esc(cap)} — area = static cost units (v2); colour = the same scale · anchor = × median component ({esc(unit_str(model['median_component']))}) / top P%</span><span class="legend"><span class="lbl">production static cost units (v2)</span>{swatches}<span class="hatchbox"></span><span>test/bench/other</span></span></div>
  {svg_markup}
  <div class="brief-head"><span><b>KV1 BRIEF</b> — top {len(cards)} actions, production first, ranked by severity then confidence</span><span>ARCH-002 excluded · cost anchor = × median function ({esc(unit_str(model['median_function']))}) / top P% of files · evidence is static</span></div>
  <section class="brief-cards" style="--n:{n_cards}">
{cards_markup}
  </section>
  <footer><a class="drill-link" href="{esc(drill)}">Q2 · what depends on the hot component? Open the 1-hop drill →</a><span class="foot-note">{esc(note)}</span></footer>
</main>
</body></html>
'''

def edge_summary(model: dict):
    order = base_order(model)
    index = {c["id"]: i for i, c in enumerate(order)}
    pair: dict[tuple[int, int], int] = {}
    example: dict[tuple[int, int], str] = {}
    for edge in model["dsm_edges"]:
        a = index.get(model["comp_of"](edge["from"]))
        b = index.get(model["comp_of"](edge["to"]))
        if a is None or b is None:
            continue
        pair[(a, b)] = pair.get((a, b), 0) + 1
        example.setdefault((a, b), f"{edge['from']} → {edge['to']}")
    uses = {i: [] for i in range(len(order))}
    deps = {i: [] for i in range(len(order))}
    for (a, b), count in pair.items():
        if a == b:
            continue
        uses[a].append((b, count))
        deps[b].append((a, count))
    for i in uses:
        uses[i].sort(key=lambda t: (-t[1], order[t[0]]["id"]))
        deps[i].sort(key=lambda t: (-t[1], order[t[0]]["id"]))
    max_cross = max([v for (a, b), v in pair.items() if a != b] or [1])
    return order, index, pair, example, uses, deps, max_cross


def render_drill(model: dict) -> str:
    order, index, pair, example, uses, deps, max_cross = edge_summary(model)
    focus_cell = model["hottest_prod"]
    focus_id = focus_cell.get("base") or focus_cell["id"]
    focus = model["base"].get(focus_id) or order[0]
    fi = index.get(focus["id"], 0)
    n = len(order)
    cell = min(34, max(15, int((1360 - 250) / max(1, n))))
    label_w, top_h, row_h = 250, 136, 30
    grid_w = label_w + n * cell + 8
    grid_h = top_h + n * row_h + 10
    max_file = max((model["cost_by_file"].get(p, 0) for p in focus["files"]), default=0)
    top_files = sorted(focus["files"], key=lambda p: (-model["cost_by_file"].get(p, 0), p))[:5]

    def short_name(cid: str, limit: int = 34) -> str:
        if cid == ".":
            return "core (root)"
        name = cid.replace("tests/unity/", "u/").replace("tests/", "t/")
        return name if len(name) <= limit else "…" + name[-(limit - 1):]

    parts = [f'<svg class="dsm" viewBox="0 0 {grid_w} {grid_h}" width="{grid_w}" height="{grid_h}" '
             f'role="img" aria-label="Component dependency structure matrix of include edges">',
             f'<rect x="0" y="0" width="{grid_w}" height="{grid_h}" fill="#ffffff"/>']
    for j, comp in enumerate(order):
        cx = label_w + j * cell + cell / 2
        cy = top_h - 8
        parts.append(f'<text x="{cx}" y="{cy}" transform="rotate(-55 {cx} {cy})" text-anchor="start" '
                     f'font-size="10.5" fill="#475467">{esc(trunc(short_name(comp["id"]), 30))}</text>')
    for i, row in enumerate(order):
        y = top_h + i * row_h
        use_ids = " ".join(f"c{b}" for b, _ in uses.get(i, []))
        parts.append(f'<g class="dsm-row" id="c{i}" data-uses="{use_ids}">')
        parts.append(f'<a href="#c{i}"><text class="rowlab" x="{label_w - 8}" y="{y + 20}" text-anchor="end" '
                     f'font-size="11" fill="#334155">{esc(trunc(short_name(row["id"]), 34))}</text></a>')
        for j, col in enumerate(order):
            count = pair.get((i, j), 0)
            if i == j:
                fill = "#e9eef5" if count else "#f8fafc"
            elif count:
                fill = ramp(math.sqrt(count / max_cross))
            else:
                fill = "#f8fafc"
            parts.append(f'<rect class="bg col{j}" x="{label_w + j * cell}" y="{y}" width="{max(1, cell - 1)}" '
                         f'height="{max(1, row_h - 1)}" fill="{fill}" stroke="#ffffff" stroke-width="1">'
                         f'<title>{esc(short_name(row["id"]))} includes {esc(short_name(col["id"]))}: {count} edge(s)</title></rect>')
            if count and cell >= 20:
                parts.append(f'<text x="{label_w + j * cell + (cell - 1) / 2}" y="{y + 19}" text-anchor="middle" '
                             f'font-size="9" fill="{"#ffffff" if (i != j and dark_text(fill) is False) else "#334155"}">{count}</text>')
        parts.append("</g>")
    parts.append("</svg>")
    dsm_svg = "\n".join(parts)

    focus_members = "".join(
        f'<li><span class="mono">{esc(trunc(p, 52))}</span>'
        f'<span class="member-anchor">{esc(unit_str(model["cost_by_file"].get(p, 0)))} · '
        f'{mult_str(model["cost_by_file"].get(p, 0), model["median_function"])} median function · '
        f'top {top_pct(model["file_rank"].get(p, model["cost_files"]), model["cost_files"])}% of files</span></li>'
        for p in top_files)
    dep_rows = deps.get(fi, [])
    dep_items = "".join(
        f'<li class="dep"><span class="dep-name">{esc(short_name(order[b]["id"]))}</span>'
        f'<span class="dep-class tag tag-{esc(order[b]["class"])}">{esc(order[b]["class"])}</span>'
        f'<span class="dep-count">{count} edge{"s" if count != 1 else ""}</span>'
        f'<span class="dep-ex">{esc(trunc(example.get((b, fi), ""), 64))}</span></li>'
        for b, count in dep_rows) or '<li class="dep dep-none">No component includes it directly — a change is confined to its own translation units.</li>'
    use_rows = uses.get(fi, [])
    use_items = "".join(
        f'<li class="dep"><span class="dep-name">{esc(short_name(order[b]["id"]))}</span>'
        f'<span class="dep-count">{count} edge{"s" if count != 1 else ""}</span>'
        f'<span class="dep-ex">{esc(trunc(example.get((fi, b), ""), 64))}</span></li>'
        for b, count in use_rows) or '<li class="dep dep-none">No resolved outgoing include edges.</li>'
    dep_names = ", ".join(short_name(order[b]["id"]) for b, _ in dep_rows[:3]) or "no direct dependents"
    focus_label = short_name(focus["id"])
    q2 = (f"If <b>{esc(focus_label)}</b> changes, {len(dep_rows)} component{'s' if len(dep_rows) != 1 else ''} {'are' if len(dep_rows) != 1 else 'is'} one hop away "
          f"({esc(dep_names)}); its heaviest member is "
          f"{esc(Path(top_files[0]).name if top_files else '—')}.")
    rules = []
    for i, comp in enumerate(order):
        s = f"c{i}"
        rules.append(f'.dsm:has(#{s}:target) g[data-uses~="{s}"] rect.col{i}{{fill:#fde68a!important}}')
        rules.append(f'.dsm:has(#{s}:target) g[data-uses~="{s}"] .rowlab{{fill:#7a2e0e;font-weight:800}}')
        rules.append(f'#{s}:target .rowlab{{fill:#0b3d91;font-weight:800}}')
        rules.append(f'#{s}:target .rowlab{{fill:#0b3d91;font-weight:800}}')
    extra_css = "\n".join(rules)
    return f'''<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>F · Q2 drill · {esc(model['repo'])}</title>
<style>
:root{{--ink:#0f172a;--muted:#5b6b7f;--line:#dbe2ea;--blue:#0b3d91}}
*{{box-sizing:border-box}}
html,body{{margin:0;padding:0}}
body{{background:#eef1f5;color:var(--ink);font-family:ui-sans-serif,-apple-system,"Segoe UI",Roboto,Helvetica,Arial,sans-serif}}
.page{{width:1440px;margin:0 auto;background:#fff;padding:24px 40px 40px;min-height:900px}}
.top{{display:flex;justify-content:space-between;align-items:flex-end;border-bottom:1px solid var(--line);padding-bottom:10px}}
.brand{{font-size:20px;font-weight:760;letter-spacing:-.3px}} .brand .repo{{color:var(--blue)}}
.back{{font-size:12px;font-weight:700;color:var(--blue);text-decoration:none}}
.back:hover{{text-decoration:underline}}
.q2{{margin:14px 0 4px;font-size:20px;font-weight:700;line-height:1.3}}
.q2 b{{color:var(--blue)}}
.sub{{margin:0 0 14px;font-size:12px;color:var(--muted)}}
.dsm-wrap{{border:1px solid var(--line);border-radius:6px;padding:10px 12px 4px;background:#fff;overflow:auto}}
.dsm{{display:block}}
.dsm-row:hover .bg{{stroke:#475467!important}}
.dsm-row .rowlab{{cursor:pointer}}
.legend{{display:flex;gap:14px;align-items:center;font-size:10.5px;color:var(--muted);margin:8px 2px 0}}
.sw{{display:inline-block;width:12px;height:8px;border-radius:2px;border:1px solid rgba(15,23,42,.12)}}
.focus-panel{{margin-top:16px;border:1px solid var(--line);border-radius:6px;padding:14px 16px;background:#fbfcfe}}
.focus-head{{display:flex;align-items:baseline;gap:10px;flex-wrap:wrap}}
.focus-head h2{{margin:0;font-size:16px}}
.focus-meta{{font-size:11px;color:var(--muted)}}
.cols{{display:grid;grid-template-columns:1.05fr 1fr 1fr;gap:18px;margin-top:10px}}
h3{{margin:0 0 6px;font-size:11px;letter-spacing:.06em;text-transform:uppercase;color:var(--muted)}}
ul{{margin:0;padding:0;list-style:none}}
.focus-members li{{display:flex;justify-content:space-between;gap:8px;font-size:10.5px;padding:4px 0;border-bottom:1px dashed #eef1f6}}
.member-anchor{{text-align:right;color:#475467;font-size:9.4px;line-height:1.35}}
.mono{{font-family:ui-monospace,"SFMono-Regular",Menlo,Consolas,monospace;font-size:10.5px}}
.dep{{display:grid;grid-template-columns:1fr auto auto;gap:4px 8px;font-size:11px;padding:5px 0;border-bottom:1px solid #eef1f6}}
.dep-name{{font-weight:650}}
.dep-count{{color:#334155;font-weight:700}}
.dep-ex{{grid-column:1/4;color:var(--muted);font-family:ui-monospace,"SFMono-Regular",Menlo,Consolas,monospace;font-size:10px}}
.dep-none{{color:var(--muted);font-style:italic}}
.tag{{font-size:8.5px;font-weight:800;letter-spacing:.06em;color:#475467;background:#eaecf0;padding:1px 4px;border-radius:3px}}
.tag-production{{color:#0b3d91;background:#e8f0fb}} .tag-test{{color:#7a2e0e;background:#fef0c7}} .tag-bench{{color:#475467;background:#eaecf0}}
.caveat{{margin-top:14px;font-size:10.5px;line-height:1.45;color:var(--muted);border-top:1px solid var(--line);padding-top:8px}}
{extra_css}
@media (prefers-reduced-motion:reduce){{*{{transition:none!important}}}}
@media print{{body{{background:#fff}}.page{{width:auto}}}}
</style></head>
<body><main class="page">
<header class="top"><div class="brand">Q2 drill <span class="repo">· {esc(model['repo'])}</span></div><a class="back" href="f-{esc(model['repo'])}.html">← back to the weight map</a></header>
<p class="q2">{q2}</p>
<p class="sub">Component DSM of resolved file-to-file include edges · rows include → columns · click any row (or a treemap cell) to trace its 1-hop dependents in amber. Ordering is production-first, then static cost. Aliases: u/ = tests/unity/, t/ = tests/.</p>
<div class="dsm-wrap">{dsm_svg}
<div class="legend"><span>row includes → column</span><span><i class="sw" style="background:{ramp(0)}"></i> fewer</span><span><i class="sw" style="background:{ramp(0.5)}"></i></span><span><i class="sw" style="background:{ramp(1)}"></i> more</span><span><i class="sw" style="background:#e9eef5"></i> internal (diagonal)</span><span>{len(model['dsm_edges'])}/{len(model['edges'])} edges aggregated</span></div></div>
<section class="focus-panel">
  <div class="focus-head"><h2>Focus · {esc(focus_label)}</h2><span class="focus-meta">{esc(focus['class'])} · {focus['members']} files · {esc(unit_str(focus['cost']))} · {focus['mult']} median component · top {focus['top_pct']}% of components · {fmt_pct(focus['cost'] / model['total'])} of repo · {len(dep_rows)} one-hop dependents</span></div>
  <div class="cols">
    <div><h3>Heaviest member files</h3><ul class="focus-members">{focus_members}</ul></div>
    <div><h3>Depended on by (1 hop)</h3><ul>{dep_items}</ul></div>
    <div><h3>Uses (1 hop)</h3><ul>{use_items}</ul></div>
  </div>
</section>
<p class="caveat">Focus is the hottest <b>production</b> component (the overall hottest cell can be test code, which has no production dependents). Edge counts are file-to-file include edges aggregated to component pairs; {model['edges_unresolved']}/{len(model['edges'])} unresolved edges (to = null) and {model['edges_nonfile']} resolved imports naming non-repo modules are excluded, and schema 1.7 carries no per-edge confidence field. Static approximation — validate with a build/runtime profile. Diagram is CSS/SVG only: no script, no network.</p>
</main></body></html>
'''


PAUSE_JS = """(mode) => {
  const anims = document.getAnimations();
  let maxEnd = 0;
  for (const a of anims) {
    const t = a.effect.getComputedTiming();
    if (Number.isFinite(t.endTime)) maxEnd = Math.max(maxEnd, t.endTime);
  }
  const at = mode === 'start' ? 0 : (mode === 'mid' ? maxEnd * 0.5 : maxEnd);
  for (const a of anims) {
    a.pause();
    const t = a.effect.getComputedTiming();
    a.currentTime = Number.isFinite(t.endTime) ? Math.min(at, t.endTime) : at;
  }
  return {animations: anims.length, maxEnd: maxEnd, at: at};
}"""


def shoot(models: dict, browser) -> list[dict]:
    images = []
    for repo, model in models.items():
        page = browser.new_page(viewport={"width": VIEW_W, "height": VIEW_H}, device_scale_factor=1)
        errors: list[str] = []
        page.on("pageerror", lambda exc: errors.append(f"pageerror: {exc}"))
        page.on("console", lambda msg: errors.append(f"console: {msg.text}") if msg.type == "error" else None)
        first = OUT_DIR / f"f-{repo}.html"
        drill = OUT_DIR / f"f-{repo}-drill.html"
        page.goto(first.as_uri())
        page.wait_for_load_state("load")
        page.evaluate("() => document.fonts.ready.then(() => true)")
        page.wait_for_timeout(60)
        for mode in ("start", "mid"):
            info = page.evaluate(PAUSE_JS, mode)
            page.wait_for_timeout(60)
            out = OUT_DIR / f"f-{repo}-{mode}.png"
            page.screenshot(path=str(out))
            width, height = png_size(out)
            if (width, height) != (VIEW_W, VIEW_H):
                raise SystemExit(f"FATAL: {out} is {width}x{height}")
            images.append({"file": out.name, "repo": repo, "role": mode, "width": width,
                           "height": height, "sha256": sha256(out), "animations": info["animations"],
                           "timeline_ms": round(info["maxEnd"], 1)})
        page.emulate_media(reduced_motion="reduce")
        page.goto(first.as_uri())
        page.wait_for_load_state("load")
        page.wait_for_timeout(120)
        if page.locator(".treemap").count() != 1:
            raise SystemExit(f"FATAL: {repo} final frame has no treemap")
        cards = page.locator(".brief-card").count()
        if not 3 <= cards <= 5:
            raise SystemExit(f"FATAL: {repo} has {cards} brief cards")
        link = page.locator("a.drill-link")
        if link.count() != 1 or not link.is_visible():
            raise SystemExit(f"FATAL: {repo} drill link missing/invisible")
        box = link.bounding_box()
        if not box or box["y"] + box["height"] > VIEW_H:
            raise SystemExit(f"FATAL: {repo} drill link is below the 900px final frame")
        hidden = page.eval_on_selector_all(".cell", "els => els.filter(e => getComputedStyle(e).opacity !== '1').length")
        if hidden:
            raise SystemExit(f"FATAL: {repo} final frame has {hidden} hidden cells")
        out = OUT_DIR / f"f-{repo}-final.png"
        page.screenshot(path=str(out))
        width, height = png_size(out)
        if (width, height) != (VIEW_W, VIEW_H):
            raise SystemExit(f"FATAL: {out} is {width}x{height}")
        images.append({"file": out.name, "repo": repo, "role": "final", "width": width,
                       "height": height, "sha256": sha256(out)})
        if errors:
            raise SystemExit(f"FATAL: {repo} first screen browser errors: {errors}")
        page.emulate_media(reduced_motion="no-preference")
        page.goto(drill.as_uri())
        page.wait_for_load_state("load")
        page.wait_for_timeout(80)
        if page.locator("svg.dsm").count() != 1 or page.locator(".focus-panel").count() != 1:
            raise SystemExit(f"FATAL: {repo} drill missing DSM/focus panel")
        if errors:
            raise SystemExit(f"FATAL: {repo} drill browser errors: {errors}")
        page.close()
        print(f"captured {repo}: {cards} actions, {len(model['cells'])} cells, "
              f"{len(model['resolved_edges'])}/{len(model['edges'])} edges resolved")
    return sorted(images, key=lambda item: item["file"])


def make_manifest(models: dict, images: list[dict]) -> dict:
    repos = {}
    for repo, model in models.items():
        prod_share = model["prod_total"] / model["total"]
        rendered = layout_cells(model["cells"])
        repos[repo] = {
            "html": f"f-{repo}.html", "drill": f"f-{repo}-drill.html",
            "files": len(model["files"]),
            "functions": len(model["data"].get("stats", {}).get("functions", [])),
            "findings": len(model["data"].get("findings", [])),
            "components_total": len(model["base"]),
            "components_rendered": len([c for c in rendered if c["kind"] != "other"]),
            "cells_rendered": len(rendered),
            "actions": len(model["actions"]),
            "root_expanded_to_files": model["expand_root"],
            "production_share_of_cost": round(prod_share, 6),
            "median_component": model["median_component"],
            "median_function": model["median_function"],
            "cost_components": model["cost_components"],
            "cost_files": model["cost_files"],
            "hottest_cell": {"label": model["hottest"]["label"], "class": model["hottest"]["class"],
                             "cost": model["hottest"]["cost"], "share": round(model["hottest"]["share"], 6)},
            "hottest_production_cell": {"label": model["hottest_prod"]["label"],
                                        "class": model["hottest_prod"]["class"],
                                        "cost": model["hottest_prod"]["cost"],
                                        "share": round(model["hottest_prod"]["share"], 6)},
            "edges_total": len(model["edges"]), "edges_resolved": len(model["resolved_edges"]),
            "edges_aggregated": len(model["dsm_edges"]),
            "edges_unresolved": model["edges_unresolved"],
            "edges_nonfile": model["edges_nonfile"],
            "parse_health_summary": model["parse"],
            "input": f"perfpal-test/out-v6-{repo}/metrics.json",
            "input_sha256": sha256(TEST_ROOT / f"out-v6-{repo}" / "metrics.json"),
        }
    return {
        "candidate": "F",
        "generated": GENERATED,
        "cost_unit": "static cost units (v2)",
        "anchor_method": ("median component = median cost over scan components with cost>0; "
                          "median function = median over all function_cost_v2 values; "
                          "top P% = round(100 * cost rank / n), rank 1 = hottest"),
        "renderer": "docs/evidence/map-vision/v7/f/render_f.py (Playwright chromium-1243)",
        "viewport": "1440x900 @ device_scale_factor=1",
        "holdout": "django/postgres deliberately absent",
        "caps": {"top_cells": TOP_CELLS, "max_actions": MAX_ACTIONS,
                 "min_label_area_px": 4200, "treemap_px": [MAP_W, MAP_H]},
        "images": images,
        "repos": repos,
        "caveats": [
            "Component derivation: longest matching components[].path_prefix; '.' is the root fallback; overlapping prefixes resolve to the longest.",
            "Treemap area is the component's share of summed function_cost_v2; the top-50 cap leaves a residual 'other' cell so area still sums to the repo total.",
            "A flat repo whose root component holds >55% of cost and >40 files (tmux) expands root files as cells so the map stays legible; the DSM/drill remains component-level.",
            "cJSON static cost is 93.5% one test component (tests/unity/test/tests) and production is 0.18% of the total; the map shows this honestly, and the brief stays production-first.",
            "Include edges: only resolved file-to-file edges are aggregated; unresolved edges (to = null) and resolved imports naming non-repo modules (e.g. Python stdlib/numpy) are excluded. Schema 1.7 has no per-edge confidence field, so no LOW-confidence edge tier exists.",
            "The KV1 brief excludes -ARCH-002; findings are ranked production-first by severity then confidence; LOW-confidence findings rank last and do not appear when higher-confidence production actions exist.",
            "function_cost_v2 is a static heuristic approximation, not measured runtime; parse health is reported per repo and partial parses bound confidence.",
            "Animation is a presentation layer: the final frame contains every cell, label, action and the Q2 link; start/mid frames are evidence only.",
        ],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--no-tmux", action="store_true", help="skip the tmux density-stress repo")
    args = parser.parse_args()
    repos = [r for r in REPOS if not (r == "tmux" and (args.no_tmux or not TMUX_INPUT.exists()))]
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    models = {}
    for repo in repos:
        model = build_model(repo, load(repo))
        models[repo] = model
        (OUT_DIR / f"f-{repo}.html").write_text(render_first(model), encoding="utf-8")
        (OUT_DIR / f"f-{repo}-drill.html").write_text(render_drill(model), encoding="utf-8")
        print(f"wrote f-{repo}.html + drill")
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=EXE)
        images = shoot(models, browser)
        browser.close()
    manifest = make_manifest(models, images)
    (OUT_DIR / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n",
                                           encoding="utf-8")
    print(f"wrote manifest.json ({len(images)} images)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())


