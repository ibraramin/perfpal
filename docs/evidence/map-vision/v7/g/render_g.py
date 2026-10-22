#!/usr/bin/env python3
"""Map candidate G: entry-flow map (cJSON, hyperfine; tmux when enabled).

Self-contained, deterministic, zero-JS HTML + Playwright screenshots.
Reads only perfpal-test/out-v6-<repo>/metrics.json for the design repos.
The V6 holdout (django/postgres) is never read or rendered.

Run from the repo root:
    python3 docs/evidence/map-vision/v7/g/render_g.py
"""

from __future__ import annotations

import collections
import hashlib
import html
import json
import math
import re
import statistics
import struct
from pathlib import Path

from playwright.sync_api import sync_playwright
from xml.sax.saxutils import escape as xesc

ROOT = Path(__file__).resolve().parent
TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
EXE = "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
REPOS = ["cJSON", "hyperfine", "tmux"]
W, H = 868, 796
NODE_CAP = 20                            # 50-node plan ceiling; 20 is the 1440x900 card budget
EDGE_CAP = 55

PALETTE = ["#2f6f8f", "#8a5a2b", "#5a7d4a", "#7a4f7d", "#3f6d6a", "#8d6a2f",
           "#4c5f8a", "#7d5a4a", "#5c6e3a", "#6a4a6a"]


def esc(value) -> str:
    return html.escape(str(value), quote=True)


def mid_trunc(text: str, limit: int) -> str:
    if len(text) <= limit:
        return text
    keep = max(2, (limit - 1) // 2)
    return text[:keep] + "…" + text[-keep:]


def fmt_int(value) -> str:
    return f"{int(round(value)):,}"


def cost_text(value) -> str:
    return f"{fmt_int(value)} static cost units (v2)"


def ratio_text(value, median) -> str:
    if not median:
        return "median 0"
    ratio = value / median
    if ratio >= 100:
        return f"{int(ratio + 0.5):,}× median"
    return f"{ratio:.1f}× median"


def top_pct(model: dict, fn: dict) -> int:
    return model["function_percentiles"].get(fn.get("id"), 100)


def median_ok(model: dict) -> bool:
    return bool(model["median_function_cost"])


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
    return struct.unpack(">II", header[16:24])


# ---------------------------------------------------------------- data model

EXCLUDED_DIRS = {"tests", "test", "fuzzing", "benches", "bench", "examples", "scripts"}


def is_program_path(path: str) -> bool:
    if path.rsplit("/", 1)[-1] == "build.rs":
        return False
    return path.split("/", 1)[0] not in EXCLUDED_DIRS


def load_repo(repo: str) -> dict:
    metrics = TEST_ROOT / f"out-v6-{repo}" / "metrics.json"
    if not metrics.exists():
        raise SystemExit(f"FATAL: missing {metrics}")
    data = json.loads(metrics.read_text(encoding="utf-8"))
    if data.get("schema_version") != "1.7":
        raise SystemExit(f"FATAL: {repo} schema {data.get('schema_version')} != 1.7")

    files = {f["path"]: f for f in data.get("files", [])}
    funcs = data.get("stats", {}).get("functions", [])
    findings = data.get("findings", [])
    cost = collections.Counter()
    by_file = collections.defaultdict(list)
    for fn in funcs:
        path = fn.get("path")
        if path in files:
            cost[path] += fn.get("function_cost_v2", 0)
            by_file[path].append(fn)
    for path in by_file:
        by_file[path].sort(key=lambda f: (-f.get("function_cost_v2", 0),
                                          -f.get("call_fanin", 0), f.get("name", "")))

    pairs: collections.Counter[tuple[str, str]] = collections.Counter()
    for edge in data.get("graphs", {}).get("call_graph", {}).get("edges", []):
        src, dst = edge.get("from_path"), edge.get("to_path")
        if src in files and dst in files and src != dst:
            pairs[(src, dst)] += 1

    inc: collections.Counter[tuple[str, str]] = collections.Counter()
    rev_inc = collections.defaultdict(list)
    for edge in data.get("graphs", {}).get("include_graph", {}).get("edges", []):
        src, dst = edge.get("from"), edge.get("to")
        if edge.get("resolved") and src in files and dst in files and src != dst:
            inc[(src, dst)] += 1
            rev_inc[dst].append(src)

    findings_by_file = collections.defaultdict(list)
    for finding in findings:
        path = finding.get("path")
        if path in files:
            findings_by_file[path].append(finding)

    return {
        "repo": repo,
        "data": data,
        "files": files,
        "funcs": funcs,
        "by_file": by_file,
        "cost": cost,
        "pairs": pairs,
        "rev_inc": rev_inc,
        "inc": inc,
        "findings": findings_by_file,
        "source_files": {p for p, f in files.items() if f.get("function_count", 0) > 0},
        "pcs": {p: f.get("path_class", "unknown") for p, f in files.items()},
    }


def choose_entry(model: dict) -> dict:
    pcs, funcs = model["pcs"], model["funcs"]
    mains = [f for f in funcs if f.get("entry_kind") == "main"]
    program_mains = [f for f in mains if pcs.get(f["path"]) == "production"
                     and is_program_path(f["path"])]
    alternatives = []
    if program_mains:
        chosen = sorted(program_mains, key=lambda f: (-f.get("function_cost_v2", 0),
                                                      -f.get("call_fanin", 0),
                                                      f["path"], f["name"]))[0]
        for fn in mains:
            if fn is chosen:
                continue
            if fn["path"].rsplit("/", 1)[-1] == "build.rs":
                alternatives.append(f"excluded build-script main {fn['path']}:{fn['span']['start_line']}")
            elif pcs.get(fn["path"]) == "production" and is_program_path(fn["path"]):
                alternatives.append(f"{fn['path']}:{fn['span']['start_line']} {fn['name']}")
        kind, policy = chosen.get("entry_kind"), "production main"
    else:
        roots = [f for f in funcs if f.get("entry_kind") == "lib-root"
                 and pcs.get(f["path"]) == "production" and is_program_path(f["path"])]
        roots.sort(key=lambda f: (-f.get("call_fanin", 0), -f.get("function_cost_v2", 0),
                                  f["path"], f["name"]))
        if not roots:
            raise SystemExit(f"FATAL: {model['repo']} has no eligible entry")
        chosen = roots[0]
        threshold = chosen.get("call_fanin", 0) * 0.20
        alternatives = [f"{f['path']}:{f['span']['start_line']} {f['name']} "
                        f"(fan-in {f['call_fanin']})"
                        for f in roots[1:] if f.get("call_fanin", 0) >= threshold][:5]
        kind, policy = "lib-root", "library root (no production main)"
    return {
        "path": chosen["path"],
        "name": chosen["name"],
        "line": chosen["span"]["start_line"],
        "kind": kind,
        "confidence": chosen.get("entry_confidence", "?"),
        "policy": policy,
        "cost": chosen.get("function_cost_v2", 0),
        "fanin": chosen.get("call_fanin", 0),
        "alternatives": alternatives,
        "label": f"{policy}: {chosen['path']}:{chosen['span']['start_line']} · {chosen['name']}",
    }


def bfs(model: dict, entry: str) -> tuple[dict, dict]:
    cost, pairs = model["cost"], model["pairs"]
    adj = collections.defaultdict(set)
    for (src, dst) in pairs:
        adj[src].add(dst)
    depth, parent = {entry: 0}, {}
    queue = collections.deque([entry])
    while queue:
        node = queue.popleft()
        for nxt in sorted(adj[node], key=lambda p: (-cost[p], p)):
            if nxt not in depth:
                depth[nxt] = depth[node] + 1
                parent[nxt] = node
                queue.append(nxt)
    return depth, parent


def select_path(depth: dict, parent: dict, cost: collections.Counter, entry: str) -> list[str]:
    if len(depth) == 1:
        return [entry]
    # a leaf has no child in the BFS tree
    children = collections.Counter(parent.values())
    leaves = [p for p in depth if p != entry and children[p] == 0]
    if not leaves:
        leaves = [max(depth, key=lambda p: (depth[p], cost[p], p))]
    endpoint = sorted(leaves, key=lambda p: (-cost[p], -depth[p], p))[0]
    path, node = [], endpoint
    while node != entry:
        path.append(node)
        node = parent[node]
    path.append(entry)
    return list(reversed(path))


def domain_of(path: str) -> str:
    if path.startswith("src/"):
        parts = path.split("/")
        return f"{parts[0]}/{parts[1]}" if len(parts) >= 3 else "src"
    if "/" in path:
        return path.split("/", 1)[0]
    stem = Path(path).stem
    return re.split(r"[-_]", stem)[0]


def finding_summary(findings: list[dict]) -> tuple[str, int, str]:
    if not findings:
        return "no findings", 0, "none"
    counts = collections.Counter(f.get("rule_id", "?") for f in findings)
    sev_rank = {"hot": 3, "warn": 2, "info": 1}
    rules = {f.get("rule_id", "?") for f in findings}
    best = max(rules, key=lambda r: (counts[r],
                                     max(sev_rank.get(str(f.get("severity") or "info"), 0)
                                         for f in findings
                                         if f.get("rule_id") == r),
                                     r))
    sev = max((f.get("severity", "info") for f in findings if f.get("rule_id") == best),
              key=lambda s: sev_rank.get(s, 0))
    return best, counts[best], sev


def build_model(repo: str) -> dict:
    model = load_repo(repo)
    entry = choose_entry(model)
    # Scan-relative scale: every function in the repo, zeros included.
    all_funcs = model["funcs"]
    median_cost = statistics.median([f.get("function_cost_v2", 0) for f in all_funcs])
    ordered = sorted(all_funcs, key=lambda f: (-f.get("function_cost_v2", 0),
                                               -f.get("call_fanin", 0),
                                               f.get("path", ""), f.get("name", ""),
                                               f.get("id", "")))
    function_count = len(ordered)
    percentiles = {}
    for rank, fn in enumerate(ordered, start=1):
        percentiles[fn.get("id")] = max(1, math.ceil(100 * rank / max(1, function_count)))
    depth, parent = bfs(model, entry["path"])
    path = select_path(depth, parent, model["cost"], entry["path"])
    reachable = sorted(depth, key=lambda p: (-model["cost"][p], depth[p], p))
    selected = {entry["path"], *path}
    for p in reachable:
        if len(selected) >= NODE_CAP:
            break
        selected.add(p)
    selected = {p for p in selected if p in depth}
    prod_sources = {p for p in model["source_files"] if model["pcs"][p] == "production"}
    unreached = sorted(prod_sources - set(depth), key=lambda p: (-model["cost"][p], p))
    # cJSON degenerates to a single forward node (library root is a sink). Show the
    # production callers as a grey "no forward path" column so the tail is honest.
    show_unreached = len(depth) <= 2
    shown_unreached = unreached if show_unreached else []
    for p in shown_unreached:
        if len(selected) < NODE_CAP:
            selected.add(p)
    omitted = {p for p in depth if p not in selected}
    omitted_by_depth = collections.Counter(depth[p] for p in omitted)
    incoming_calls = collections.Counter()
    for (src, dst), n in model["pairs"].items():
        if src in selected and dst in omitted:
            incoming_calls[depth[dst]] += n

    max_depth = max([depth[p] for p in selected if p in depth] +
                    list(omitted_by_depth.keys()))
    cols = max_depth + 1
    unreached_col = cols if shown_unreached else None
    if shown_unreached:
        cols += 1

    # extra selection: the hottest unreached file is not added (only cJSON tail above)
    hot_pool = [p for p in selected if p in depth and p != entry["path"]]
    hot = sorted(hot_pool, key=lambda p: (-model["cost"][p], p))[0] if hot_pool else entry["path"]

    # drill: interface file with the most reverse include edges (production only)
    candidates = []
    for target, sources in model["rev_inc"].items():
        if target in model["files"] and model["pcs"].get(target) == "production":
            candidates.append((len(set(sources)), target))
    candidates.sort(key=lambda item: (-item[0], item[1]))
    drill_target = candidates[0][1] if candidates else entry["path"]

    return {
        **model,
        "entry": entry,
        "median_function_cost": median_cost,
        "function_count": function_count,
        "function_percentiles": percentiles,
        "depth": depth,
        "parent": parent,
        "path": path,
        "selected": selected,
        "reachable": reachable,
        "omitted": omitted,
        "omitted_by_depth": omitted_by_depth,
        "incoming_calls": incoming_calls,
        "max_depth": max_depth,
        "cols": cols,
        "unreached": unreached,
        "shown_unreached": shown_unreached,
        "unreached_col": unreached_col,
        "hot": hot,
        "drill_target": drill_target,
        "drill_rev": sorted(set(model["rev_inc"].get(drill_target, []))),
    }


# ---------------------------------------------------------------- layout

def layout(model: dict) -> dict:
    selected = model["selected"]
    by_col: dict[int, list] = collections.defaultdict(list)
    for path in sorted(selected, key=lambda p: (model["depth"].get(p, 10 ** 6),
                                                -model["cost"][p], p)):
        if path in model["depth"]:
            by_col[model["depth"][path]].append(("file", path))
    for depth, count in sorted(model["omitted_by_depth"].items()):
        if count:
            by_col[depth].append(("agg", depth, count, model["incoming_calls"].get(depth, 0)))
    if model["unreached_col"] is not None:
        by_col[model["unreached_col"]] = [("file", p) for p in
                                          sorted(model["shown_unreached"],
                                                 key=lambda p: (-model["cost"][p], p))]

    ncols = model["cols"]
    mx, cg = 16, 12
    colw = (W - 2 * mx - (ncols - 1) * cg) / ncols
    node_w = int(min(150, colw - 14))
    avail = H - 54 - 14
    boxes: dict[str, dict] = {}
    aggs: list[dict] = []
    for col in range(ncols):
        items = by_col.get(col, [])
        heights = [40 if item[0] == "file" else 30 for item in items]
        gap = 8.0
        if len(items) > 1:
            gap = min(8.0, max(3.0, (avail - sum(heights)) / (len(items) - 1)))
        total = sum(heights) + gap * max(0, len(items) - 1)
        y = 54 + max(0, (avail - total) / 2)
        cx = mx + col * (colw + cg)
        x = cx + (colw - node_w) / 2
        for item, height in zip(items, heights):
            if item[0] == "file":
                path = item[1]
                boxes[path] = {
                    "path": path, "x": round(x, 1), "y": round(y, 1),
                    "w": node_w, "h": height, "col": col,
                    "cost": model["cost"][path],
                    "unreached": path not in model["depth"],
                    "entry": path == model["entry"]["path"],
                    "hot": path == model["hot"],
                    "depth": model["depth"].get(path),
                }
            else:
                _, dep, count, calls = item
                aggs.append({
                    "x": round(x, 1), "y": round(y, 1), "w": node_w, "h": height,
                    "col": col, "depth": dep, "count": count, "calls": calls,
                })
            y += height + gap
    return {"boxes": boxes, "aggs": aggs, "colw": colw, "node_w": node_w,
            "by_col": by_col, "ncols": ncols, "gap": cg}


def hull_points(box: dict, pad: int = 7) -> list[tuple[float, float]]:
    x, y, w, h = box["x"], box["y"], box["w"], box["h"]
    pts = [(x - pad, y - pad), (x + w + pad, y - pad),
           (x + w + pad, y + h + pad), (x - pad, y + h + pad)]
    pts = sorted(set(pts))
    if len(pts) <= 3:
        return pts

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower = []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    upper = []
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return lower[:-1] + upper[:-1]


def render_hulls(model: dict, boxes: dict[str, dict]) -> str:
    groups = collections.defaultdict(list)
    for path, box in boxes.items():
        groups[domain_of(path)].append(path)
    domains = sorted(groups)
    out = []
    for index, domain in enumerate(domains):
        members = groups[domain]
        if len(members) < 2:
            continue
        pts = []
        for path in members:
            pts.extend(hull_points(boxes[path]))
        pts = sorted(set(pts))
        if len(pts) <= 3:
            continue

        def cross(o, a, b):
            return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

        lower = []
        for p in pts:
            while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
                lower.pop()
            lower.append(p)
        upper = []
        for p in reversed(pts):
            while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
                upper.pop()
            upper.append(p)
        poly = lower[:-1] + upper[:-1]
        color = PALETTE[index % len(PALETTE)]
        d = " ".join(f"{'M' if i == 0 else 'L'}{x:.1f},{y:.1f}"
                     for i, (x, y) in enumerate(poly)) + " Z"
        top = min(p[1] for p in poly)
        left = min(p[0] for p in poly)
        label = f"{domain} · {len(members)}"
        out.append(f'<g class="hull"><path d="{d}" fill="{color}" stroke="{color}"/>'
                   f'<text x="{left + 4:.1f}" y="{max(11, top - 4):.1f}">{xesc(label)}</text></g>')
    return "".join(out)


def render_edges(model: dict, boxes: dict[str, dict]) -> tuple[str, int, int]:
    pairs = model["pairs"]
    selected = model["selected"]
    candidates = [(n, a, b) for (a, b), n in pairs.items() if a in selected and b in selected]
    candidates.sort(key=lambda item: (-item[0], item[1], item[2]))
    path_pairs = {(model["path"][i], model["path"][i + 1])
                  for i in range(len(model["path"]) - 1)}
    drawn = candidates[:EDGE_CAP]
    drawn_keys = {(a, b) for _, a, b in drawn}
    for pair in sorted(path_pairs):
        if pair not in drawn_keys and pair in pairs:
            drawn.append((pairs[pair], pair[0], pair[1]))

    out, labels = [], []
    back_count = 0
    for count, src, dst in sorted(drawn, key=lambda item: (boxes[item[1]]["col"],
                                                          boxes[item[1]]["y"],
                                                          boxes[item[2]]["col"],
                                                          boxes[item[2]]["y"])):
        a, b = boxes[src], boxes[dst]
        x1, y1, x2, y2 = a["x"] + a["w"], a["y"] + a["h"] / 2, b["x"], b["y"] + b["h"] / 2
        back = b["col"] <= a["col"]
        if back:
            back_count += 1
            mx = max(x1, x2) + 46
            d = f"M{x1:.1f},{y1:.1f} C{mx:.1f},{y1:.1f} {mx:.1f},{y2:.1f} {x2:.1f},{y2:.1f}"
            cls = "edge back"
            delay = 1500 + b["col"] * 180
        else:
            dx = max(18, (x2 - x1) * 0.48)
            d = f"M{x1:.1f},{y1:.1f} C{x1 + dx:.1f},{y1:.1f} {x2 - dx:.1f},{y2:.1f} {x2:.1f},{y2:.1f}"
            cls = "edge"
            delay = 1350 + b["col"] * 190
        width = 0.9 + min(1.8, math.log2(count + 1) * 0.28)
        out.append(f'<path class="{cls}" d="{d}" pathLength="1" stroke-width="{width:.2f}" '
                   f'style="animation-delay:{delay}ms"/>')
        mxp, myp = (x1 + x2) / 2, (y1 + y2) / 2
        if count >= 8 and len(labels) < 12:
            labels.append(f'<text class="edge-count" x="{mxp:.1f}" y="{myp - 3:.1f}">{count}</text>')
    return "".join(out) + "".join(labels), back_count, len(drawn)


def render_nodes(model: dict, boxes: dict[str, dict], aggs: list[dict]) -> str:
    out = []
    for path, box in sorted(boxes.items(), key=lambda item: (item[1]["col"], item[1]["y"], item[0])):
        cls = ["node"]
        if box["entry"]:
            cls.append("entry")
        if box["hot"]:
            cls.append("hot")
        if box["unreached"]:
            cls.append("unreached")
        else:
            pcs = model["pcs"].get(path)
            if pcs in {"test", "bench"}:
                cls.append(pcs)
        delay = 320 + box["col"] * 430 + int((box["y"] - 54) / 14) * 18
        cost = model["cost"][path]
        fns = model["by_file"].get(path, [])
        fn_count = len(fns)
        label = mid_trunc(path, max(12, int((box["w"] - 14) / 5.2)))
        meta = cost_text(cost)
        anchor = ratio_text(cost, model["median_function_cost"]) + " function"
        if box["unreached"]:
            anchor += " · unreached"
        titled_fns = ", ".join(f"{path}:{fn.get('name') or '(anonymous)'}" for fn in fns[:4])
        out.append(f'<g class="{" ".join(cls)}" style="animation-delay:{delay}ms">'
                   f'<title>{xesc(path)} — {xesc(model["pcs"].get(path, "?"))}, '
                   f'{xesc(cost_text(cost))}, {xesc(ratio_text(cost, model["median_function_cost"]))}'
                   f'; functions: {xesc(titled_fns)}</title>'
                   f'<rect class="node-box" x="{box["x"]:.1f}" y="{box["y"]:.1f}" '
                   f'width="{box["w"]}" height="{box["h"]}" rx="5"/>'
                   f'<text class="node-path" x="{box["x"] + 7:.1f}" y="{box["y"] + 13:.1f}">'
                   f'{xesc(label)}</text>'
                   f'<text class="node-meta" x="{box["x"] + 7:.1f}" y="{box["y"] + 25:.1f}">'
                   f'{xesc(meta)}</text>'
                   f'<text class="node-anchor" x="{box["x"] + 7:.1f}" y="{box["y"] + 35:.1f}">'
                   f'{xesc(anchor)}</text>')
        if box["entry"]:
            out.append(f'<text class="node-tag entry-tag" x="{box["x"] + box["w"] - 7:.1f}" '
                       f'y="{box["y"] - 5:.1f}" text-anchor="end">ENTRY</text>')
        if box["hot"] and not box["entry"]:
            out.append(f'<text class="node-tag hot-tag" x="{box["x"] + box["w"] - 7:.1f}" '
                       f'y="{box["y"] - 5:.1f}" text-anchor="end">HOTTEST</text>')
        out.append("</g>")
    for box in aggs:
        delay = 320 + box["col"] * 430
        out.append(f'<g class="agg" style="animation-delay:{delay}ms">'
                   f'<title>Aggregated: {box["count"]} reachable files at depth {box["depth"]}, '
                   f'{box["calls"]} calls in from shown files</title>'
                   f'<rect x="{box["x"]:.1f}" y="{box["y"]:.1f}" width="{box["w"]}" '
                   f'height="{box["h"]}" rx="4"/>'
                   f'<text x="{box["x"] + 7:.1f}" y="{box["y"] + 13:.1f}">…+{box["count"]} more</text>'
                   f'<text class="agg-meta" x="{box["x"] + 7:.1f}" y="{box["y"] + 24:.1f}">'
                   f'depth {box["depth"]} · {box["calls"]} calls in</text></g>')
    return "".join(out)


def render_trace(model: dict, boxes: dict[str, dict]) -> str:
    path = model["path"]
    out = []
    if len(path) == 1:
        b = boxes[path[0]]
        return (f'<g class="trace-step" style="animation-delay:5250ms">'
                f'<circle cx="{b["x"] + b["w"] - 2:.1f}" cy="{b["y"] + 2:.1f}" r="8"/>'
                f'<text x="{b["x"] + b["w"] - 2:.1f}" y="{b["y"] + 5.5:.1f}">1</text></g>')
    for i in range(len(path) - 1):
        a, b = boxes[path[i]], boxes[path[i + 1]]
        x1, y1, x2, y2 = a["x"] + a["w"], a["y"] + a["h"] / 2, b["x"], b["y"] + b["h"] / 2
        if b["col"] <= a["col"]:
            mx = max(x1, x2) + 46
            d = f"M{x1:.1f},{y1:.1f} C{mx:.1f},{y1:.1f} {mx:.1f},{y2:.1f} {x2:.1f},{y2:.1f}"
        else:
            dx = max(18, (x2 - x1) * 0.48)
            d = f"M{x1:.1f},{y1:.1f} C{x1 + dx:.1f},{y1:.1f} {x2 - dx:.1f},{y2:.1f} {x2:.1f},{y2:.1f}"
        out.append(f'<path class="trace-edge" d="{d}" pathLength="1" '
                   f'style="animation-delay:{5050 + i * 240}ms"/>')
    for i, path_id in enumerate(path):
        b = boxes[path_id]
        out.append(f'<g class="trace-step" style="animation-delay:{5250 + i * 240}ms">'
                   f'<circle cx="{b["x"] + b["w"] - 2:.1f}" cy="{b["y"] + 2:.1f}" r="8"/>'
                   f'<text x="{b["x"] + b["w"] - 2:.1f}" y="{b["y"] + 5.5:.1f}">{i + 1}</text></g>')
    return "".join(out)


def render_svg(model: dict, lay: dict) -> tuple[str, int, int]:
    boxes, aggs = lay["boxes"], lay["aggs"]
    edge_svg, back_count, drawn_count = render_edges(model, boxes)
    parts = [f'<svg class="map" viewBox="0 0 {W} {H}" width="{W}" height="{H}" '
             f'role="img" aria-label="Entry-flow map for {xesc(model["repo"])}">',
             '<defs>',
             '<marker id="arrow" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" '
             'markerHeight="6" orient="auto"><path d="M0,0 L8,4 L0,8 z" fill="#8d8577"/></marker>',
             '<marker id="arrow-back" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="6" '
             'markerHeight="6" orient="auto"><path d="M0,0 L8,4 L0,8 z" fill="#b45309"/></marker>',
             '<marker id="arrow-trace" viewBox="0 0 8 8" refX="7" refY="4" markerWidth="7" '
             'markerHeight="7" orient="auto"><path d="M0,0 L8,4 L0,8 z" fill="#c2410c"/></marker>',
             '</defs>']
    parts.append(render_hulls(model, boxes))
    parts.append('<g class="edges">' + edge_svg.replace('class="edge back"',
                 'class="edge back" marker-end="url(#arrow-back)"').replace(
                 'class="edge"', 'class="edge" marker-end="url(#arrow)"') + '</g>')
    parts.append(render_trace(model, boxes))
    parts.append(render_nodes(model, boxes, aggs))
    # column headings
    for col in range(model["cols"]):
        x = 16 + col * (lay["colw"] + lay["gap"]) + 4
        if model["unreached_col"] == col:
            label = "UNREACHED · no forward path"
        elif col == 0:
            label = "DEPTH 0 · ENTRY"
        else:
            label = f"DEPTH {col}"
        parts.append(f'<text class="col-label" x="{x:.1f}" y="38" '
                     f'style="animation-delay:{300 + col * 430}ms">{xesc(label)}</text>')
    if not model["shown_unreached"] and model["unreached"]:
        label = f'…+{len(model["unreached"])} unreached production files · no forward path'
        parts.append(f'<g class="unreached-chip"><rect x="16" y="{H - 32}" width="252" '
                     f'height="20" rx="4"/><text x="24" y="{H - 18}">{xesc(label)}</text></g>')
    parts.append(f'<text class="map-note" x="{W - 14}" y="{H - 8}" text-anchor="end">'
                 f'forward callers → callees only · grey = unreached · dashed amber = back-edge'
                 f' · edge label = distinct call count</text>')
    parts.append("</svg>")
    return "".join(parts), back_count, drawn_count


# ---------------------------------------------------------------- HTML

CSS = """
:root{--ink:#202b35;--muted:#6f6a61;--paper:#f4f1ea;--panel:#fbf9f4;--line:#d6d0c4;
--prod:#3f6d6a;--test:#5c7391;--bench:#7a4f7d;--unreached:#b3aca1;--hot:#c2410c;--entry:#1f2d3a}
*{box-sizing:border-box}
html,body{margin:0;width:1440px;height:900px;overflow:hidden}
body{background:var(--paper);color:var(--ink);font-family:ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,Helvetica,Arial,sans-serif}
.page{padding:8px 12px;height:900px;display:flex;flex-direction:column}
.pausebox{position:absolute;opacity:0;width:1px;height:1px}
header{height:58px;display:flex;align-items:center;gap:14px;flex:0 0 58px}
.hleft{width:244px;flex:0 0 244px}
.kicker{font-size:8px;letter-spacing:.16em;font-weight:800;color:#8a837a}
.title{font-size:21px;font-weight:800;letter-spacing:-.015em;line-height:1.05;margin-top:2px}
.title small{font-size:9px;font-weight:600;color:#6f6a61;letter-spacing:.02em;margin-left:6px}
.entry-block{flex:1;border-left:2px solid var(--entry);padding-left:10px;min-width:0}
.entry-label{font-size:8px;font-weight:800;letter-spacing:.13em;color:var(--hot)}
.entry-name{font-size:12.5px;font-weight:750;margin-top:1px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.entry-policy{font-size:8.2px;color:var(--muted);margin-top:1px}
.hright{flex:0 0 auto;display:flex;align-items:center;gap:8px}
.drill-link{display:inline-block;background:#1f2d3a;color:#fff;text-decoration:none;padding:7px 10px;border-radius:4px;font-size:8.5px;font-weight:750;letter-spacing:.02em}
.pause-btn{display:inline-block;border:1px solid #9d9588;background:#fffdf8;color:#3c4650;padding:7px 10px;border-radius:4px;font-size:8.5px;font-weight:700;cursor:pointer;user-select:none}
.pausebox:checked~.page .pause-btn{background:#ece8df}
.pausebox:focus-visible~.page .pause-btn{outline:2px solid var(--hot);outline-offset:2px}
.subhead{height:30px;flex:0 0 30px;display:flex;flex-direction:column;justify-content:center;border-top:1px solid var(--line);border-bottom:1px solid var(--line);font-size:8px;color:#5f5a51;line-height:1.35}
.subhead b{color:#2d3944}
.legend{display:inline-flex;align-items:center;gap:5px;margin-right:10px}
.sw{display:inline-block;width:8px;height:8px;border-radius:2px;vertical-align:-1px}
.sw.prod{background:#fdfbf6;border:1px solid #8b8173}.sw.test{background:#eef2f7;border:1px solid #7f93aa}
.sw.unreached{background:#e8e5df;border:1px solid #b8b2a7}.sw.path{background:#c2410c}
.sw.back{background:transparent;border-top:2px dashed #b45309;height:4px;border-radius:0}
.stage{flex:1;display:flex;gap:10px;min-height:0}
.map{width:868px;height:796px;flex:0 0 868px;background:var(--panel);border:1px solid var(--line);border-radius:6px}
.rail{flex:1;min-width:0;height:796px;display:flex;flex-direction:column}
.rail-title{height:18px;flex:0 0 18px;font-size:7.6px;font-weight:800;letter-spacing:.11em;color:#8a837a;display:flex;align-items:center;justify-content:space-between}
.rail-title span{font-weight:600;letter-spacing:0;color:#6f6a61}
.cards{display:grid;grid-template-columns:1fr 1fr;grid-auto-rows:70px;gap:6px;align-content:start}
.card{background:#fffdf8;border:1px solid #d8d2c6;border-left:3px solid #8b8173;border-radius:4px;padding:3px 7px;overflow:hidden;opacity:0;animation:fadeUp .45s ease-out both}
.card.entry{border-left-color:var(--entry);background:#f3f1eb}
.card.hot{border-left-color:var(--hot)}
.card.test{border-left-color:var(--test);background:#f4f6f9}
.card.bench{border-left-color:var(--bench);background:#f8f3f8}
.card.unreached{border-left-color:var(--unreached);background:#efede8;color:#756f66}
.card-path{font-size:9.4px;font-weight:750;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;line-height:10px}
.card-meta{font-size:7.4px;color:#726c63;line-height:8.5px;white-space:nowrap;overflow:hidden}
.card-anchor{font-size:7.4px;color:#8a5a2b;line-height:8.5px;white-space:nowrap;overflow:hidden;font-weight:650}
.card-funcs{font-size:7.6px;line-height:8.6px;color:#3f4952;height:26px;overflow:hidden}
.cf{display:block;white-space:normal;overflow-wrap:anywhere;text-overflow:clip}
.cf .pct{color:#8a837a;font-weight:600}
.card-find{font-size:7.6px;line-height:8px;color:#8a5a2b;white-space:nowrap;overflow:hidden;font-weight:650}
.card.unreached .card-find{color:#8a837a}
.hello{font-size:8px}
.node{opacity:0;animation:fadeUp .5s ease-out both}
.node-box{fill:#fdfbf6;stroke:#8b8173;stroke-width:1.1}
.node-path{font-size:8.8px;font-weight:700;fill:#23303a}
.node-meta{font-size:7px;fill:#6b7280}
.node-anchor{font-size:6.8px;fill:#8a837a}
.node.test .node-box{fill:#eef2f7;stroke:#7f93aa;stroke-dasharray:3 2}
.node.bench .node-box{fill:#f7f0f7;stroke:#8a6f9e;stroke-dasharray:3 2}
.node.unreached .node-box{fill:#e8e5df;stroke:#b8b2a7}
.node.unreached .node-path{fill:#8a837a}.node.unreached .node-meta,.node.unreached .node-anchor{fill:#9c958a}
.node.entry{opacity:1;animation:none}
.node.entry .node-box{fill:#1f2d3a;stroke:#1f2d3a;stroke-width:2}
.node.entry .node-path{fill:#fff}.node.entry .node-meta{fill:#cbd5e1}.node.entry .node-anchor{fill:#cbd5e1}
.node.hot .node-box{stroke:#c2410c;stroke-width:2.4;animation:hotPulse 2.4s ease-in-out 6200ms infinite}
.node.entry .node-box{animation:entryPulse 2.4s ease-in-out 0ms infinite}
.node-tag{font-size:6.6px;font-weight:800;letter-spacing:.1em;fill:#c2410c}
.entry-tag{fill:#1f2d3a}
.col-label{font-size:7.6px;letter-spacing:.11em;fill:#8a837a;font-weight:800;opacity:0;animation:fadeUp .4s ease-out both}
.map-note{font-size:6.8px;fill:#9a9388;letter-spacing:.01em}
.edge{fill:none;stroke:#8d8577;stroke-opacity:.34;stroke-dasharray:1;stroke-dashoffset:1;animation:draw .55s ease-out both}
.edge.back{stroke:#b45309;stroke-opacity:.52;stroke-dasharray:4 3;stroke-dashoffset:0;animation:fade .4s ease-out both}
.edge-count{font-size:6.2px;fill:#7c7468;paint-order:stroke;stroke:#fbf9f4;stroke-width:2}
.trace-edge{fill:none;stroke:#c2410c;stroke-width:2.3;stroke-opacity:.95;stroke-dasharray:1;stroke-dashoffset:1;animation:draw .5s ease-out both}
.hull{opacity:0;animation:fade .6s ease-out 150ms both}
.hull path{fill-opacity:.06;stroke-opacity:.5;stroke-width:1;stroke-dasharray:5 3}
.hull text{font-size:7.2px;fill:#7b7266;font-weight:750;letter-spacing:.03em;paint-order:stroke;stroke:#fbf9f4;stroke-width:2}
.agg{opacity:0;animation:fadeUp .45s ease-out both}
.agg rect{fill:#f0ede6;stroke:#b8b2a7;stroke-dasharray:3 2}
.agg text{font-size:7.1px;fill:#7b7266;font-weight:650}.agg .agg-meta{font-size:6.3px;font-weight:500}
.unreached-chip{opacity:0;animation:fadeUp .45s ease-out 2500ms both}
.unreached-chip rect{fill:#e8e5df;stroke:#b8b2a7;stroke-dasharray:3 2}
.unreached-chip text{font-size:7.2px;fill:#8a837a;font-weight:650}
.trace-step{opacity:0;animation:pop .35s ease-out both}
.trace-step circle{fill:#c2410c;stroke:#fff;stroke-width:1.2}
.trace-step text{font-size:6.8px;fill:#fff;font-weight:800;text-anchor:middle}
@keyframes fadeUp{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
@keyframes fade{from{opacity:0}to{opacity:1}}
@keyframes draw{to{stroke-dashoffset:0}}
@keyframes pop{from{opacity:0;transform:scale(.4)}to{opacity:1;transform:none}}
@keyframes entryPulse{0%,100%{stroke-width:2}50%{stroke-width:4.4}}
@keyframes hotPulse{0%,100%{stroke-width:2.4}50%{stroke-width:5}}
.pausebox:checked~.page .stage *{animation-play-state:paused!important}
@media (prefers-reduced-motion: reduce){
.stage *,.stage *::before,.stage *::after{animation:none!important;opacity:1!important;transform:none!important;stroke-dashoffset:0!important}
}
@media print{
body{background:#fff}.drill-link,.pause-btn{display:none}
.stage *,.stage *::before,.stage *::after{animation:none!important;opacity:1!important;transform:none!important;stroke-dashoffset:0!important}
}
"""


def card_html(model: dict, path: str, index: int) -> str:
    box = model["layout"]["boxes"][path]
    cls = ["card"]
    if box["entry"]:
        cls.append("entry")
    if box["hot"]:
        cls.append("hot")
    if box["unreached"]:
        cls.append("unreached")
    elif model["pcs"].get(path) in {"test", "bench"}:
        cls.append(model["pcs"][path])
    funcs = model["by_file"].get(path, [])
    # Full names only. The 3-line function budget means a card may show 1-3
    # functions; names are never truncated and may wrap.
    chosen = []
    lines = 0
    for fn in funcs[:5]:
        display_name = fn.get("name") or "(anonymous)"
        estimate = max(1, math.ceil((len(display_name) + 10) / 46))
        if chosen and lines + estimate > 3:
            break
        chosen.append(fn)
        lines += estimate
    if chosen:
        funcs_html = "".join(
            f'<span class="cf" title="{esc(path + ":" + (fn.get("name") or "(anonymous)"))} — '
            f'{esc(cost_text(fn.get("function_cost_v2", 0)))} · top {top_pct(model, fn)}%">'
            f'{esc(fn.get("name") or "(anonymous)")} '
            f'<span class="pct">· top {top_pct(model, fn)}%</span></span>'
            for fn in chosen)
    else:
        funcs_html = '<span class="cf">no functions</span>'
    rule, count, sev = finding_summary(model["findings"].get(path, []))
    rule_text = f"{rule} ×{count} · {sev}" if count else "no findings"
    cost = model["cost"][path]
    prefix = "entry · " if box["entry"] else ("unreached · " if box["unreached"] else "")
    line2 = f'{prefix}{model["pcs"].get(path, "?")} · {cost_text(cost)}'
    anchor = ratio_text(cost, model["median_function_cost"])
    anchor = anchor.replace("× median", "× median function") if median_ok(model) else "median 0 (no anchor)"
    anchor_line = f"{anchor} · {len(funcs)} fns"
    return (f'<div class="{" ".join(cls)}" style="animation-delay:{2700 + index * 45}ms">'
            f'<div class="card-path" title="{esc(path)}">{esc(mid_trunc(path, 40))}</div>'
            f'<div class="card-meta">{esc(line2)}</div>'
            f'<div class="card-anchor">{esc(anchor_line)}</div>'
            f'<div class="card-funcs">{funcs_html}</div>'
            f'<div class="card-find">{esc(rule_text)}</div></div>')


def render_page(model: dict) -> str:
    lay = model["layout"]
    svg, back_count, drawn_count = render_svg(model, lay)
    model["back_edges"] = back_count
    model["drawn_edges"] = drawn_count
    selected_pairs = sum(1 for (a, b) in model["pairs"] if a in model["selected"] and b in model["selected"])
    selected_files = [p for p in sorted(model["selected"],
                                        key=lambda p: (model["depth"].get(p, 99),
                                                       -model["cost"][p], p))]
    cards = "".join(card_html(model, path, i) for i, path in enumerate(selected_files))
    entry = model["entry"]
    alt = len(entry["alternatives"])
    alt_text = f"alternatives: {alt}" if alt else "alternatives: 0"
    if entry["alternatives"]:
        shorts = []
        for alternative in entry["alternatives"][:2]:
            match = re.search(r"([^\s:]+:\d+)", alternative)
            shorts.append(match.group(1) if match else alternative.split(" ", 1)[0])
        alt_text += " (" + "; ".join(shorts) + ")"
    policy_short = ("production main (build scripts, tests, fuzzing excluded)"
                    if entry["kind"] == "main" else
                    "highest fan-in production lib-root (no production main)")
    reachable_label = f"{len(model['reachable'])} reachable"
    if len(model["reachable"]) == 1:
        reachable_label += " (entry)"
    unreached_label = (f"{len(model['unreached'])} unreached production"
                       + (" (grey)" if model["shown_unreached"] else " (counted, not drawn)"))
    file_summary = (f"{len(model['selected'])} files shown · {reachable_label} · "
                    f"{len(model['omitted'])} reachable aggregated · {unreached_label}")
    edge_note = (f"edges: {drawn_count} of {selected_pairs} shown file pairs drawn"
                 f" (top by call count; {back_count} back-edges)")
    if selected_pairs <= EDGE_CAP:
        edge_note = f"edges: all {selected_pairs} shown file pairs drawn ({back_count} back-edges)"
    path_label = " → ".join(f"{i + 1}. {mid_trunc(p, 20)}" for i, p in enumerate(model["path"]))
    if len(model["path"]) == 1:
        path_label = f"1. {model['path'][0]} (single-node: library root has no forward edges)"

    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<meta name="viewport" content="width=1440, initial-scale=1">
<title>G · {esc(model['repo'])} entry-flow map</title>
<style>{CSS}</style></head>
<body>
<input id="pause" class="pausebox" type="checkbox" aria-label="Pause the animation">
<div class="page">
<header>
  <div class="hleft">
    <div class="kicker">PERFPAL · MAP CANDIDATE G · ENTRY-FLOW</div>
    <div class="title">{esc(model['repo'])} <small>estimated static flow</small></div>
  </div>
  <div class="entry-block">
    <div class="entry-label">ENTRY · {esc(entry['policy'])}</div>
    <div class="entry-name">{esc(entry['path'])}:{entry['line']} · {esc(entry['name'])}</div>
    <div class="entry-policy">{esc(policy_short)} · confidence {esc(entry['confidence'])} · {esc(alt_text)}</div>
  </div>
  <div class="hright">
    <a class="drill-link" href="g-{esc(model['repo'])}-drill.html">Q2 reverse drill · {esc(mid_trunc(model['drill_target'], 24))} →</a>
    <label class="pause-btn" for="pause">Pause animation (CSS)</label>
  </div>
</header>
<div class="subhead">
  <div><b>{esc(file_summary)}</b> · {esc(edge_note)} · call edges are structural and LOW confidence</div>
  <div><span class="legend"><span class="sw prod"></span>production</span><span class="legend"><span class="sw test"></span>test / bench (demoted)</span><span class="legend"><span class="sw unreached"></span>unreached</span><span class="legend"><span class="sw back"></span>back-edge</span><span class="legend"><span class="sw path"></span>path trace</span> Entry policy: {esc(policy_short)}. Animation plays once (~6.5 s); pause is CSS-only; reduced-motion/print show this final frame.</div>
</div>
<div class="stage">
  {svg}
  <div class="rail">
    <div class="rail-title">FILE CARDS · {len(selected_files)} shown <span>× median function = multiple of the repo-wide median function cost</span></div>
    <div class="cards">{cards}</div>
  </div>
</div>
</div>
</body></html>
"""


# ---------------------------------------------------------------- drill (Q2)

DRILL_CSS = """
*{box-sizing:border-box}html,body{margin:0;width:1440px;min-height:900px;background:#f4f1ea;color:#202b35}
body{font-family:ui-sans-serif,system-ui,-apple-system,"Segoe UI",Roboto,Helvetica,Arial,sans-serif}
.page{padding:10px 14px;width:1440px}
header{display:flex;justify-content:space-between;align-items:flex-end;border-bottom:2px solid #1f2d3a;padding-bottom:8px}
.kicker{font-size:8px;letter-spacing:.16em;font-weight:800;color:#8a837a}
h1{font-size:22px;margin:2px 0 0;letter-spacing:-.02em}h1 small{font-size:11px;color:#6f6a61;font-weight:600}
.back{font-size:9px;color:#1f2d3a;font-weight:750;text-decoration:none;border:1px solid #9d9588;padding:6px 9px;border-radius:4px;background:#fffdf8}
.summary{display:flex;gap:8px;margin:9px 0}
.chip{border:1px solid #d6d0c4;background:#fffdf8;border-radius:4px;padding:5px 8px;font-size:8px;color:#5f5a51}
.chip b{color:#202b35;font-size:11px;display:block;line-height:1.1}
.grid{display:grid;grid-template-columns:400px 480px 1fr;gap:12px}
.panel{background:#fbf9f4;border:1px solid #d6d0c4;border-radius:5px;padding:9px}
.panel h2{font-size:10px;margin:0 0 2px;letter-spacing:.04em}
.panel p{font-size:7.6px;color:#6f6a61;margin:0 0 7px;line-height:1.35}
table{border-collapse:collapse;width:100%;table-layout:fixed}
th,td{font-size:7.2px;border:1px solid #ddd7cc;padding:2px 3px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
th{background:#efece4;font-weight:750;color:#55504a;text-align:left}
td.n{text-align:center;font-weight:700}
.matrix th{writing-mode:vertical-rl;transform:rotate(180deg);height:70px;text-align:left;font-size:6.4px;padding:2px}
.matrix td{width:26px;height:22px;text-align:center;font-size:6.6px;font-weight:700;color:#1f2d3a}
.axis{font-size:7.2px;color:#6f6a61;margin-bottom:4px}
.count{font-variant-numeric:tabular-nums;color:#3c4650}
.note{font-size:7px;color:#7b7266;margin-top:6px;line-height:1.4}
.legend{display:flex;gap:10px;font-size:7px;color:#6f6a61;margin-top:5px}
.legend i{display:inline-block;width:8px;height:8px;border-radius:2px;background:#e8e5df;border:1px solid #d6d0c4}
footer{margin-top:8px;border-top:1px solid #d6d0c4;padding-top:6px;font-size:7px;color:#7b7266}
"""


def heat(value: int, maximum: int) -> str:
    if not value:
        return "#fbf9f4"
    ratio = value / max(1, maximum)
    if ratio > 0.66:
        return "#c98a5b"
    if ratio > 0.33:
        return "#e3b98e"
    return "#f0dcc4"


def render_drill(model: dict) -> str:
    target = model["drill_target"]
    rev_counts = collections.Counter()
    for src in model["rev_inc"].get(target, []):
        rev_counts[src] += model["inc"].get((src, target), 1)
    deps = sorted(rev_counts, key=lambda p: (-rev_counts[p], p))
    total = len(deps)
    prod = [p for p in deps if model["pcs"].get(p) == "production"]
    path_class = model["pcs"].get(target, "?")
    domain = domain_of(target)

    list_rows = []
    for i, path in enumerate(deps[:15], start=1):
        list_rows.append(f"<tr><td class='n'>{i}</td><td title='{esc(path)}'>{esc(mid_trunc(path, 34))}</td>"
                         f"<td class='n count'>{rev_counts[path]}</td>"
                         f"<td>{esc(model['pcs'].get(path, '?'))}</td></tr>")
    more = max(0, total - 15)
    if more:
        list_rows.append(f"<tr><td colspan='4'>…+{more} more direct dependents</td></tr>")

    focus_nodes = [target] + deps[:10]
    focus_index = {p: i for i, p in enumerate(focus_nodes)}
    focus_max = 1
    focus_cells = {}
    for (src, dst), n in model["inc"].items():
        if src in focus_index and dst in focus_index:
            focus_cells[(src, dst)] = n
            focus_max = max(focus_max, n)
    focus_header = "".join(f"<th title='{esc(p)}'>{esc(mid_trunc(Path(p).name, 12))}</th>"
                           for p in focus_nodes)
    focus_rows = []
    for row in focus_nodes:
        cells = []
        for col in focus_nodes:
            n = focus_cells.get((row, col), 0)
            style = f"background:{heat(n, focus_max)}"
            label = str(n) if n else ""
            cells.append(f"<td style='{style}'>{label}</td>")
        focus_rows.append(f"<tr><th title='{esc(row)}'>{esc(mid_trunc(row, 30))}</th>{''.join(cells)}</tr>")
    focus_table = ("<table class='matrix'><thead><tr><th>from ↓ / to →</th>"
                   + focus_header + "</tr></thead><tbody>" + "".join(focus_rows) + "</tbody></table>")

    dom_edges = collections.Counter()
    dom_files = collections.Counter()
    for path in model["files"]:
        dom_files[domain_of(path)] += 1
    for (src, dst), n in model["inc"].items():
        dom_edges[(domain_of(src), domain_of(dst))] += n
    dom_volume = collections.Counter()
    for (a, b), n in dom_edges.items():
        dom_volume[a] += n
        dom_volume[b] += n
    top_domains = [d for d, _ in sorted(dom_volume.items(), key=lambda kv: (-kv[1], kv[0]))[:9]]
    if len(dom_volume) > 9:
        top_domains.append("other")
    def dom_label(p):
        return "other" if p not in top_domains else p
    comp = collections.Counter()
    comp_max = 1
    for (a, b), n in dom_edges.items():
        comp[(dom_label(a), dom_label(b))] += n
        comp_max = max(comp_max, comp[(dom_label(a), dom_label(b))])
    comp_header = "".join(f"<th title='{esc(d)}'>{esc(mid_trunc(d, 14))}</th>" for d in top_domains)
    comp_rows = []
    for row in top_domains:
        cells = []
        for col in top_domains:
            n = comp.get((row, col), 0)
            cells.append(f"<td style='background:{heat(n, comp_max)}' title='{esc(row)} → {esc(col)}: {n}'>{n if n else ''}</td>")
        comp_rows.append(f"<tr><th title='{esc(row)}'>{esc(mid_trunc(row, 22))}</th>{''.join(cells)}</tr>")
    comp_table = ("<table class='matrix'><thead><tr><th>from ↓ / to →</th>" + comp_header
                  + "</tr></thead><tbody>" + "".join(comp_rows) + "</tbody></table>")

    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Q2 · {esc(model['repo'])} reverse dependents</title>
<style>{DRILL_CSS}</style></head><body><div class="page">
<header><div><div class="kicker">PERFPAL · MAP CANDIDATE G · Q2 DRILL</div>
<h1>{esc(model['repo'])} · what breaks if <span style="color:#c2410c">{esc(mid_trunc(target, 38))}</span> changes?</h1></div>
<a class="back" href="g-{esc(model['repo'])}.html">← back to entry-flow map</a></header>
<div class="summary">
<div class="chip"><b>{total}</b>direct reverse dependents</div>
<div class="chip"><b>{len(prod)}</b>production dependents</div>
<div class="chip"><b>{rev_counts[deps[0]] if deps else 0}</b>edges from hottest dependent</div>
<div class="chip"><b>{esc(path_class)}</b>focus class · domain {esc(domain)}</div>
<div class="chip"><b>1-hop</b>include graph only</div>
</div>
<div class="grid">
<div class="panel"><h2>Direct dependents (list)</h2><p>Files that include <b>{esc(mid_trunc(target, 34))}</b> directly. Changing its interface is what breaks them.</p>
<table><thead><tr><th style="width:18px">#</th><th>dependent path</th><th style="width:44px">edges</th><th style="width:72px">class</th></tr></thead><tbody>{''.join(list_rows)}</tbody></table>
<div class="note">Reverse direction only. A dependent is a file with an include edge INTO the focus; callers are not implied.</div></div>
<div class="panel"><h2>Focus + 1-hop DSM</h2><p>Rows include columns. Diagonal is self; off-diagonal cells are direct include edges among the focus and its 10 largest dependents.</p>
{focus_table}<div class="note">Focus is row/column 1 ({esc(mid_trunc(target, 18))}). Blank = no direct include edge.</div></div>
<div class="panel"><h2>Component / domain DSM</h2><p>Production-first path/filename domains; cell = include edges from row domain to column domain (all repo files).</p>
{comp_table}<div class="legend"><span><i></i>0</span><span><i style="background:#f0dcc4"></i>low</span><span><i style="background:#e3b98e"></i>mid</span><span><i style="background:#c98a5b"></i>high</span></div>
<div class="note">Hulls/domains on the flow map use the same grouping. Test-dominated component prefixes are replaced by path/filename domains; this is an approximation, not a declared architecture.</div></div>
</div>
<footer>Static structural evidence · resolved include edges only · no runtime implication · Q2 stays a reverse drill so the flow map remains forward-only.</footer>
</div></body></html>
"""


# ---------------------------------------------------------------- capture

def pause_script(fraction: float) -> str:
    return f"""() => {{
      const anims = document.getAnimations();
      anims.forEach(a => a.pause());
      const ends = anims.map(a => a.effect.getComputedTiming().endTime).filter(Number.isFinite);
      const total = ends.length ? Math.max(...ends) : 0;
      anims.forEach(a => {{
        const end = a.effect.getComputedTiming().endTime;
        const target = Number.isFinite(end) ? Math.min(total * {fraction}, end) : total * {fraction};
        try {{ a.currentTime = target; }} catch (e) {{}}
      }});
      return anims.length;
    }}"""


def snap(page, out: Path) -> None:
    """Playwright screenshot with a small retry for transient capture failures."""
    last = None
    for attempt in range(3):
        try:
            page.screenshot(path=str(out), full_page=True)
            return
        except Exception as exc:  # transient Page.captureScreenshot failures
            last = exc
            page.wait_for_timeout(120)
    raise last


def capture(repo: str, html_path: Path, out_dir: Path, errors: list[str]) -> list[dict]:
    images = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=EXE)
        page = browser.new_page(viewport={"width": 1440, "height": 900},
                                device_scale_factor=1, reduced_motion="no-preference")
        page.on("console", lambda msg: errors.append(f"{repo}: console {msg.type}: {msg.text}")
                if msg.type == "error" else None)
        page.on("pageerror", lambda exc: errors.append(f"{repo}: pageerror: {exc}"))
        page.goto(html_path.as_uri())
        page.wait_for_load_state("load")
        page.wait_for_timeout(60)
        for frame, fraction in (("start", 0.0), ("mid", 0.5)):
            page.emulate_media(reduced_motion="no-preference")
            count = page.evaluate(pause_script(fraction))
            page.wait_for_timeout(25)
            out = out_dir / f"g-{repo}-{frame}.png"
            snap(page, out)
            width, height = png_size(out)
            images.append({"frame": frame, "file": out.name, "width": width,
                           "height": height, "sha256": sha256(out),
                           "animations_paused": count})
        page.emulate_media(reduced_motion="reduce")
        page.wait_for_timeout(40)
        out = out_dir / f"g-{repo}-final.png"
        snap(page, out)
        width, height = png_size(out)
        images.append({"frame": "final", "file": out.name, "width": width,
                       "height": height, "sha256": sha256(out),
                       "reduced_motion": True})
        fidelity = page.evaluate("""() => ({
          entry: document.querySelector('.entry-name')?.textContent || '',
          nodes: document.querySelectorAll('svg .node').length,
          cards: document.querySelectorAll('.card').length,
          hulls: document.querySelectorAll('svg .hull').length,
          steps: document.querySelectorAll('svg .trace-step').length,
          drill: document.querySelector('a.drill-link')?.getAttribute('href') || '',
          cards_opacity: getComputedStyle(document.querySelector('.card')).opacity,
          nodes_opacity: getComputedStyle(document.querySelector('svg .node')).opacity,
          scrollWidth: document.documentElement.scrollWidth,
          scrollHeight: document.documentElement.scrollHeight,
          handlers: document.querySelectorAll('[onclick],[onload]').length
        })""")
        page.close()
        browser.close()
    return images + [{"fidelity": fidelity}]


# ---------------------------------------------------------------- manifest

def manifest_entry(model: dict) -> dict:
    boxes = model["layout"]["boxes"]
    selected_pairs = sum(1 for (a, b) in model["pairs"]
                         if a in model["selected"] and b in model["selected"])
    hulls = collections.Counter(domain_of(p) for p in model["selected"])
    return {
        "repo": model["repo"],
        "entry": {
            "path": model["entry"]["path"],
            "line": model["entry"]["line"],
            "function": model["entry"]["name"],
            "kind": model["entry"]["kind"],
            "policy": model["entry"]["policy"],
            "confidence": model["entry"]["confidence"],
            "cost": model["entry"]["cost"],
            "cost_text": cost_text(model["entry"]["cost"]),
            "fan_in": model["entry"]["fanin"],
            "alternatives": model["entry"]["alternatives"],
        },
        "scale": {
            "units": "static cost units (v2)",
            "median_function_cost": model["median_function_cost"],
            "function_count": model["function_count"],
            "ratio_anchor": "file/node cost / median function cost (median over all repo functions, zeros included)",
            "top_pct_anchor": "ceil(100 * rank / N) over all functions ordered by cost desc, fan-in desc, path, name, id; zeros included",
        },
        "counts": {
            "files_total": len(model["files"]),
            "source_files": len(model["source_files"]),
            "production_source_files": sum(1 for p in model["source_files"]
                                           if model["pcs"][p] == "production"),
            "reachable_files": len(model["reachable"]),
            "selected_nodes": len(model["selected"]),
            "cards": len(model["selected"]),
            "unreached_production": len(model["unreached"]),
            "unreached_shown": len(model["shown_unreached"]),
            "aggregated_reachable": len(model["omitted"]),
            "aggregate_nodes": len(model["layout"]["aggs"]),
            "call_pairs_total": len(model["pairs"]),
            "call_pairs_selected": selected_pairs,
            "edges_drawn": model.get("drawn_edges", 0),
            "back_edges": model.get("back_edges", 0),
            "hulls": sum(1 for n in hulls.values() if n >= 2),
        },
        "caps": {
            "node_ceiling": 50,
            "effective_node_cap": NODE_CAP,
            "card_budget": 20,
            "edge_draw_cap": EDGE_CAP,
        },
        "path_trace": model["path"],
        "drill": {
            "target": model["drill_target"],
            "direct_reverse_dependents": len(set(model["rev_inc"].get(model["drill_target"], []))),
            "file": f"g-{model['repo']}-drill.html",
        },
        "caveats": [
            "Call edges are structural and LOW confidence in the default tier; the map says estimated static flow and never implies runtime truth.",
            "Unreached = production source files with no forward path from the chosen entry; cJSON's library root is a sink, so its production callers are greyed and point at the Q2 drill.",
            "Edges drawn are capped at the top caller→callee file pairs by call count; the rest are counted, not drawn.",
            "Card/hull text truncates long paths; the HTML title attribute carries the full path in the artifact.",
        ],
    }


def write_notes(models: dict[str, dict], images: list[dict], errors: list[str]) -> None:
    lines = [
        f"# G — entry-flow map prototype ({', '.join(models.keys())})",
        "",
        "Static-first, zero-JS entry-flow map. Forward call direction only. Call edges are",
        "structural and LOW confidence in the default tier: the view says **estimated static",
        "flow** and never claims runtime truth.",
        "",
        "## Entry policy",
        "",
        "- Prefer a **production main** (`entry_kind == main`). Build scripts (`build.rs`),",
        "  `tests/`, `fuzzing/`, `benches/`, `examples/` and `scripts/` are not program entries.",
        "- If no production main exists (libraries), use the **highest fan-in production",
        "  `lib-root` function** and label it `library root (no production main)`.",
        "- Ties: cost desc, fan-in desc, path asc. Alternatives are shown in the header and",
        "  recorded in `manifest.json`.",
        "",
        "## Reading the view",
        "",
        "- Header summary: `N files shown · R reachable · A reachable aggregated · U unreached production`.",
        "  cJSON is `6 files shown · 1 reachable (entry) · 0 reachable aggregated · 5 unreached production (grey)`;",
        "  the 5 grey nodes are callers into the library root, not extra reachable files.",
        "- Small numbers on call edges are `edge label = distinct call count` (call-graph edges",
        "  aggregated per caller→callee file pair).",
        "- Near the file cards: `× median function = multiple of the repo-wide median function cost`.",
        "",
        "## Units and anchors",
        "",
        "- Every visible cost number is in **static cost units (v2)**.",
        "- File/node anchor = file cost / **median function cost**. The median is over every",
        "  function in the repo's v2 scale, zeros included; the ratio is shown to one decimal",
        "  below 100× and rounded to an integer at/above 100×.",
        "- Function lines show **top P%**, where P = ceil(100 × rank / N) over all functions",
        "  ordered by cost desc, fan-in desc, path, name, id (zeros included). Rank 1 = top 1%.",
        "- Cards show 1-3 **full** function names (never ellipsised); long names wrap. Function",
        "  lines carry the rank anchor instead of a second cost figure, so the file line is the",
        "  single magnitude carrier. The SVG `<title>` keeps `path:function` for top functions.",
        "",
    ]
    for repo, model in models.items():
        entry = model["entry"]
        counts = manifest_entry(model)["counts"]
        lines += [
            f"## {repo}",
            "",
            f"- Entry: `{entry['path']}:{entry['line']}` · `{entry['name']}` · {entry['policy']}"
            f" · confidence {entry['confidence']}.",
            f"- Alternatives: {len(entry['alternatives'])}. "
            + ("; ".join(entry["alternatives"]) if entry["alternatives"] else "no plausible alternative entry."),
            f"- Files: {counts['selected_nodes']} nodes shown; "
            f"{counts['reachable_files']} reachable, {counts['aggregated_reachable']} aggregated, "
            f"{counts['unreached_production']} production unreached "
            f"({counts['unreached_shown']} drawn grey).",
            f"- Edges: {counts['edges_drawn']} of {counts['call_pairs_selected']} shown file pairs drawn "
            f"(cap {EDGE_CAP}); {counts['back_edges']} back-edges dashed amber.",
            f"- Path trace: {' → '.join(model['path'])}.",
            f"- Q2 drill target: `{model['drill_target']}` "
            f"({counts['selected_nodes'] and len(set(model['rev_inc'].get(model['drill_target'], [])))}"
            " direct reverse includes).",
            "",
        ]
        if repo == "cJSON":
            lines += [
                "### cJSON specifics",
                "",
                "- No production `main` (29 mains are tests/fuzzing). Highest fan-in production",
                "  `lib-root` is `cJSON_Delete` (fan-in 105), so the entry is labelled",
                "  `library root (no production main)`.",
                "- `cJSON.c` has **no outgoing file-level call edges**: the library root is a sink.",
                "  The forward map therefore has one reachable file. The 5 unreached production",
                "  sources are drawn grey in a `UNREACHED` column; they call **into** the entry.",
                "  Reverse context is deliberately left to `g-cJSON-drill.html` (Q2).",
                "- This is the honest structural shape of a single-file library, not a layout bug.",
                "",
            ]
        if repo == "hyperfine":
            lines += [
                "### hyperfine specifics",
                "",
                "- Production `main` is `src/main.rs`; `build.rs` is a build-script main and is",
                "  excluded (recorded as one alternative).",
                "- Tests/benches reached by call edges remain visible but are demoted in colour",
                "  and labelled `test / bench`.",
                "- 19 reachable files fit under the 20-card budget; no reachable aggregation.",
                "",
            ]
        if repo == "tmux":
            lines += [
                "### tmux specifics (density stress case)",
                "",
                "- Production `main` is `tmux.c:434`; no alternative entry.",
                "- 106 reachable files but only 20 nodes/cards fit: the remaining 86 reachable",
                "  files are aggregated into five `…+N more` depth nodes (depth 1: 11 files,",
                "  depth 2: 28, depth 3: 39, depth 4: 7, depth 5: 1), with incoming-call counts.",
                "- 145 caller→callee file pairs exist among the 20 shown nodes; the view draws",
                "  the top 55 by call count plus path edges (57 total). 49 are dashed amber",
                "  back-edges. The omitted count is in the header and manifest.",
                "- The source `components[]` list is root-only (`fuzz`, `.`, `regress`, `compat`),",
                "  so hulls use filename families (`window`, `tty`, `screen`) as a production-first",
                "  approximation; only families with ≥2 visible files get a hull.",
                "- Path trace: tmux.c → options.c → format.c → grid.c.",
                "",
            ]
    lines += [
        "## Density / legibility blockers",
        "",
        f"- Cap: hard ceiling {50} nodes (plan), effective {NODE_CAP} nodes because the 1440×900",
        "  first screen must also carry 20 per-file cards at readable type. Nodes beyond the cap",
        "  and reachable files are aggregated into honest `…+N more` nodes with incoming-call counts.",
        f"- Edge drawing is capped at {EDGE_CAP} file pairs by call count; path edges are forced.",
        "  tmux has 145 pairs among 20 shown nodes and draws 57 (55 + path); the rest are counted.",
        "- Component hulls: source `components[]` are test-dominated for cJSON and root-only for",
        "  tmux; the prototype derives production-first path/filename domains. Hulls are drawn",
        "  only for domains with ≥2 visible files; single-file domains are named in cards.",
        "- Per-file cards truncate long **paths** only (full path in the title); function names",
        "  are shown in full and the card shows 1-3 functions depending on wrapping.",
        "- cJSON's one-node flow is a genuine data limitation; the drill, not the flow map,",
        "  answers its reverse/dependency question.",
        "",
        "## Verification",
        "",
        f"- Console/page errors observed: {len(errors)}" + ("." if not errors else ": " + "; ".join(errors)),
        "- Each HTML has no `<script` and no `onclick`/`onload`; animation is CSS keyframes only.",
        "- Final PNGs are captured with `prefers-reduced-motion: reduce`, which is the complete",
        "  static frame; start/mid are captured with `document.getAnimations()` paused at 0%/50%.",
        "- Determinism: no clock, no randomness, sorted traversal; HTML bytes are compared across",
        "  two runs with `cmp` (manifest image hashes are run outputs, not HTML input).",
        "",
    ]
    (ROOT / "NOTES.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main() -> int:
    models: dict[str, dict] = {}
    errors: list[str] = []
    for repo in REPOS:
        model = build_model(repo)
        model["layout"] = layout(model)
        html_text = render_page(model)
        (ROOT / f"g-{repo}.html").write_text(html_text, encoding="utf-8")
        (ROOT / f"g-{repo}-drill.html").write_text(render_drill(model), encoding="utf-8")
        for name in (f"g-{repo}.html", f"g-{repo}-drill.html"):
            text = (ROOT / name).read_text(encoding="utf-8")
            if "<script" in text or "onclick" in text or "onload" in text:
                raise SystemExit(f"FATAL: forbidden script/handler in {name}")
        models[repo] = model
        print(f"wrote {repo}: {len(model['selected'])} nodes, "
              f"{model['drawn_edges']} edges drawn, entry {model['entry']['path']}")

    all_images: list[dict] = []
    for repo in REPOS:
        result = capture(repo, ROOT / f"g-{repo}.html", ROOT, errors)
        images = [item for item in result if "frame" in item]
        fidelity = [item for item in result if "fidelity" in item][0]["fidelity"]
        model = models[repo]
        checks = {
            "entry label": bool(fidelity["entry"]),
            "flow nodes": fidelity["nodes"] > 0,
            "cards": fidelity["cards"] == len(model["selected"]),
            "hulls": fidelity["hulls"] >= 1,
            "path trace": fidelity["steps"] >= 1,
            "drill link": fidelity["drill"] == f"g-{repo}-drill.html",
            "no inline handlers": fidelity["handlers"] == 0,
            "final cards visible": fidelity["cards_opacity"] == "1",
            "final nodes visible": fidelity["nodes_opacity"] == "1",
            "fits 1440x900": fidelity["scrollWidth"] <= 1440 and fidelity["scrollHeight"] <= 900,
        }
        failed = [key for key, value in checks.items() if not value]
        print(f"{repo} fidelity: " + ", ".join(f"{k}={'ok' if v else 'FAIL'}"
                                               for k, v in checks.items()))
        if failed:
            raise SystemExit(f"FATAL: {repo} fidelity failed: {failed}; fidelity={fidelity}")
        all_images.extend(images)

    if errors:
        print("ERRORS:")
        for error in errors:
            print(" ", error)
        raise SystemExit("FATAL: console/page errors observed")

    repo_manifest = {}
    for repo, model in models.items():
        html_path = ROOT / f"g-{repo}.html"
        drill_path = ROOT / f"g-{repo}-drill.html"
        entry = manifest_entry(model)
        entry["html_sha256"] = sha256(html_path)
        entry["drill_html_sha256"] = sha256(drill_path)
        entry["images"] = [img for img in all_images if img["file"].startswith(f"g-{repo}-")]
        repo_manifest[repo] = entry

    manifest = {
        "_comment": "Map candidate G entry-flow prototype. Design repos only; django/postgres holdout never read.",
        "view": "G · entry-flow map",
        "schema_version": "1.7",
        "inputs": {repo: f"perfpal-test/out-v6-{repo}/metrics.json" for repo in REPOS},
        "holdout": "django/postgres deliberately absent",
        "viewport": "1440x900 @ device_scale_factor=1",
        "renderer": f"{ROOT.name}/render_g.py (stdlib + Playwright chromium-1243)",
        "determinism": ("No random, no clock, sorted traversal. HTML/drill bytes are stable across runs; "
                        "PNG sha256 are run outputs from Playwright."),
        "legends": {
            "header": "N files shown · R reachable · A reachable aggregated · U unreached production",
            "edge_label": "distinct call count",
            "anchor": "× median function = multiple of the repo-wide median function cost",
        },
        "repos": repo_manifest,
        "images": sorted(all_images, key=lambda item: item["file"]),
        "caveats": [
            "Call edges are structural and LOW confidence; no runtime truth is implied.",
            "Node/edge/card caps are density decisions recorded in NOTES.md.",
            "Component hulls are production-first approximations (path/filename domains).",
            "cJSON has no production main and no forward edges from its library root.",
        ],
    }
    (ROOT / "manifest.json").write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n",
                                       encoding="utf-8")
    write_notes(models, all_images, errors)
    print(f"wrote manifest.json and NOTES.md; {len(all_images)} PNG frames at 1440x900")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
