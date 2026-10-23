#!/usr/bin/env python3
"""Deterministic Atlas run-story extractor (Track D, Session D1).

Reads two already-rendered Perfpal Atlas reports plus their metrics.json and
emits compact JSON that the sim UI can replay with exact drawn geometry.

Stdlib only. No network. Byte-stable output:
  json.dumps(..., sort_keys=True, indent=2, ensure_ascii=False) + "\n"
"""

import hashlib
import html
import json
import os
import re
import sys
from pathlib import Path
from typing import NoReturn

OUT_DIR = Path(__file__).resolve().parent
BASE = OUT_DIR / "source"

REAL_CASES = [
    {
        "case": "hyperfine",
        "report": BASE / "out-hyperfine" / "report.html",
        "metrics": BASE / "out-hyperfine" / "metrics.json",
        "expected": {"nodes": 19, "edges": 49, "hulls": 5, "columns": 6, "trace": 3},
    },
    {
        "case": "cjson",
        "report": BASE / "out-cjson" / "report.html",
        "metrics": BASE / "out-cjson" / "metrics.json",
        "expected": {"nodes": 7, "edges": 5, "hulls": 2, "columns": 3, "trace": 2},
    },
]

KICKER = "PERFPAL · ATLAS"
WORDMARK = "Atlas"
SUBLINE = "entry-flow map · static, profile to confirm"

FINDINGS_CAP = 400
FUNCTIONS_CAP = 8


# ---------------------------------------------------------------------------
# fail-closed helpers
# ---------------------------------------------------------------------------

def fail(message: str) -> NoReturn:
    raise SystemExit("extract: FAIL: " + message)


def require(condition: object, message: str) -> None:
    if not condition:
        fail(message)


def first_group(
    match: "re.Match[str] | None", message: str, group: int = 1
) -> str:
    """Return a capture group, failing closed when the match is absent."""
    if match is None:
        fail(message)
    value = match.group(group)
    if value is None:
        fail(message)
    return value


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        fail("cannot read %s: %s" % (path, exc))


def read_json(path: Path):
    try:
        return json.loads(read_text(path))
    except json.JSONDecodeError as exc:
        fail("invalid JSON in %s: %s" % (path, exc))


# ---------------------------------------------------------------------------
# tiny HTML/XML helpers (the report is machine-rendered, fixed structure)
# ---------------------------------------------------------------------------

def strip_tags(value):
    return html.unescape(re.sub(r"<[^>]*>", "", value))


def norm(value):
    return re.sub(r"\s+", " ", strip_tags(value)).strip()


def attr(attrs, name):
    match = re.search(r'\b' + re.escape(name) + r'="([^"]*)"', attrs)
    return match.group(1) if match else None


def required_attr(attrs: str, name: str, message: str) -> str:
    value = attr(attrs, name)
    if value is None:
        fail(message)
    return value


def num(value):
    number = float(value)
    return int(number) if number.is_integer() else round(number, 3)


_G_TAG = re.compile(r"<(/?)g(?=[\s>])[^>]*>")


def group_end(source, start):
    """Return the index just past the </g> matching the <g> at `start`."""
    depth = 0
    pos = start
    while True:
        match = _G_TAG.search(source, pos)
        if not match:
            fail("unbalanced <g> group at offset %d" % start)
        if match.group(1) == "/":
            depth -= 1
            if depth == 0:
                return match.end()
        else:
            depth += 1
        pos = match.end()


def classes(value):
    return (value or "").split()


def text_nodes(body, class_name):
    """[(attrs, text)] for <text> elements carrying `class_name`."""
    found = []
    for match in re.finditer(r"<text\b([^>]*)>(.*?)</text>", body, re.S):
        if class_name in classes(attr(match.group(1), "class")):
            found.append((match.group(1), norm(match.group(2))))
    return found


# ---------------------------------------------------------------------------
# report parsing
# ---------------------------------------------------------------------------

def parse_report(report_path):
    source = read_text(report_path)

    map_start = source.find('<section id="map">')
    require(map_start >= 0, "%s: missing <section id=\"map\">" % report_path.name)
    canvas_start = source.find('<section id="atlas-canvas"', map_start)
    require(canvas_start >= 0, "%s: missing atlas-canvas" % report_path.name)

    masthead = source[map_start:canvas_start]
    canvas = source[canvas_start:]
    svg_end = canvas.find("</svg>")
    require(svg_end >= 0, "%s: missing </svg>" % report_path.name)
    svg = canvas[: svg_end + len("</svg>")]

    view = re.search(r'<svg[^>]*viewBox="0 0 ([0-9.]+) ([0-9.]+)"', svg)
    view_message = "%s: missing svg viewBox" % report_path.name
    stage = {
        "w": num(first_group(view, view_message, 1)),
        "h": num(first_group(view, view_message, 2)),
    }

    footer_match = re.search(
        r'<footer class="atlas-footer">(.*?)</footer>', canvas, re.S
    )
    footer = norm(
        first_group(footer_match, "%s: missing atlas footer" % report_path.name)
    )

    meta = parse_masthead(masthead)
    meta["footer"] = footer

    hulls = parse_hulls(svg)
    columns = parse_columns(svg)
    nodes = parse_nodes(svg)
    edges = parse_edges(svg)

    trace_nodes = sorted(
        ((node["pathStep"], node["id"]) for node in nodes if node["pathStep"] is not None),
        key=lambda item: item[0],
    )
    trace = [{"step": step, "path": path} for step, path in trace_nodes]

    return {
        "stage": stage,
        "hulls": hulls,
        "columns": columns,
        "nodes": nodes,
        "edges": edges,
        "trace": trace,
        "meta": meta,
    }


def parse_masthead(masthead):
    def div_text(class_name):
        match = re.search(
            r'<div class="' + re.escape(class_name) + r'">(.*?)</div>',
            masthead,
            re.S,
        )
        return norm(match.group(1)) if match else ""

    entry_match = re.search(
        r'<b>ENTRY</b>\s*<span class="atlas-mono atlas-entry-value">(.*?)</span>',
        masthead,
        re.S,
    )
    entry = norm(first_group(entry_match, "masthead: missing ENTRY value"))

    hottest_match = re.search(
        r'<span class="atlas-hot">HOTTEST:\s*<span class="atlas-mono">(.*?)</span>',
        masthead,
        re.S,
    )
    hottest = norm(first_group(hottest_match, "masthead: missing HOTTEST value"))

    path_match = re.search(
        r'<span class="atlas-mono">PATH · ([^<]*)</span>', masthead
    )
    path_text = re.sub(
        r"\s+", " ", first_group(path_match, "masthead: missing PATH value")
    ).strip()
    path = path_text.split("·")[0].strip()

    caps_match = re.search(
        r'<span class="atlas-mono">([^<]*aggregated[^<]*)</span>', masthead
    )
    caps = re.sub(
        r"\s+", " ", first_group(caps_match, "masthead: missing caps value")
    ).strip()

    actions = []
    for match in re.finditer(
        r'<a class="atlas-chip"[^>]*>(.*?)</a>', masthead, re.S
    ):
        label = norm(match.group(1))
        anchor, separator, title = label.partition(" — ")
        if not separator:
            anchor, title = label, ""
        actions.append({"anchor": anchor.strip(), "title": title.strip()})

    return {
        "kicker": div_text("atlas-kicker"),
        "wordmark": div_text("atlas-wordmark"),
        "subline": div_text("atlas-subline"),
        "entry": entry,
        "hottest": hottest,
        "path": path,
        "caps": caps,
        "actions": actions,
    }


def parse_hulls(svg):
    hulls = []
    for match in re.finditer(r'<g class="atlas-hull">', svg):
        end = group_end(svg, match.start())
        body = svg[match.start():end]
        rect = re.search(r"<rect\b([^>]*)>", body)
        attrs = first_group(rect, "hull without rect")
        label_match = re.search(r"<text\b[^>]*>(.*?)</text>", body, re.S)
        hulls.append(
            {
                "x": num(attr(attrs, "x")),
                "y": num(attr(attrs, "y")),
                "w": num(attr(attrs, "width")),
                "h": num(attr(attrs, "height")),
                "label": norm(first_group(label_match, "hull without label")),
            }
        )
    return hulls


def parse_columns(svg):
    columns = []
    for match in re.finditer(r'<g class="(atlas-column[^"]*)"', svg):
        tokens = classes(match.group(1))
        if "atlas-column" not in tokens:
            continue  # atlas-column-header-group
        depth_match = re.search(r"--atlas-col:(\d+)", svg[match.start():match.end() + 200])
        depth = int(first_group(depth_match, "column without --atlas-col"))
        end = group_end(svg, match.start())
        body = svg[match.start():end]
        headers = text_nodes(body, "atlas-column-header")
        require(len(headers) >= 2, "column without two headers")
        x = num(attr(headers[0][0], "x"))
        # The renderer labels the right-most lane UNREACHED. Keep it: the sim
        # draws its header so unreached nodes never float without a lane.
        columns.append(
            {
                "depth": depth,
                "x": x,
                "label": headers[0][1],
                "countText": headers[1][1],
                "unreached": "is-unreached" in tokens,
                "order": len(columns),
            }
        )
    return columns


def parse_nodes(svg):
    nodes = []
    for match in re.finditer(
        r'<g class="(atlas-node[^"]*)" data-node="([^"]*)">', svg
    ):
        tokens = classes(match.group(1))
        node_id = html.unescape(match.group(2))
        end = group_end(svg, match.start())
        body = svg[match.start():end]

        title_match = re.search(r"<title>(.*?)</title>", body, re.S)
        title = norm(first_group(title_match, "node %s without title" % node_id))

        rect = re.search(r"<rect\b([^>]*?)/?>", body)
        rect_attrs = first_group(rect, "node %s without rect" % node_id)

        badges = text_nodes(body, "atlas-node-badge")
        badge = badges[0][1] if badges else ""

        costs = text_nodes(body, "atlas-node-cost")
        require(costs, "node %s without cost text" % node_id)
        cost_text = costs[0][1]
        cost_match = re.search(r"([0-9][0-9,]*)", cost_text)
        cost = int(
            first_group(cost_match, "node %s has no numeric cost" % node_id).replace(
                ",", ""
            )
        )

        ranks = [text for _, text in text_nodes(body, "atlas-node-rank")]
        require(len(ranks) >= 2, "node %s without two rank texts" % node_id)

        step = None
        path_badge = re.search(
            r'<g class="atlas-path-badge"><title>(.*?)</title>', body, re.S
        )
        if path_badge is not None:
            step_match = re.search(r"step (\d+) of (\d+)", path_badge.group(1))
            step = int(
                first_group(
                    step_match, "node %s has unparsable path badge" % node_id
                )
            )

        dash = attr(rect_attrs, "stroke-dasharray")
        nodes.append(
            {
                "id": node_id,
                "name": node_id,
                "x": num(attr(rect_attrs, "x")),
                "y": num(attr(rect_attrs, "y")),
                "w": num(attr(rect_attrs, "width")),
                "h": num(attr(rect_attrs, "height")),
                "order": len(nodes),
                "fill": attr(rect_attrs, "fill"),
                "stroke": attr(rect_attrs, "stroke"),
                "strokeWidth": num(attr(rect_attrs, "stroke-width")),
                "dash": dash,
                "badge": badge,
                "cost": cost,
                "costText": cost_text,
                "ratioText": ranks[0],
                "topText": ranks[1],
                "entry": "is-entry" in tokens,
                "hottest": "is-hottest" in tokens,
                "nonproduction": "is-nonproduction" in tokens,
                "pathStep": step,
                "title": title,
                "findings": finding_counts(badge),
            }
        )
    return nodes


def finding_counts(badge):
    counts = {"hot": 0, "warn": 0, "info": 0}
    for severity, amount in re.findall(r"(HOT|WARN|INFO)\s*×\s*([0-9,]+)", badge):
        counts[severity.lower()] = int(amount.replace(",", ""))
    return counts


def parse_edges(svg):
    edges = []
    pattern = re.compile(
        r'<path class="atlas-edge atlas-edge--(forward|back)"([^>]*)>(.*?)</path>',
        re.S,
    )
    for match in pattern.finditer(svg):
        kind = match.group(1)
        attrs = match.group(2)
        data_edge = html.unescape(
            required_attr(attrs, "data-edge", "edge without data-edge")
        )
        require("->" in data_edge, "edge data-edge is not FROM->TO: %r" % data_edge)
        source, target = data_edge.split("->", 1)

        title = norm(match.group(3))
        count_match = re.search(r"(\d+)\s+distinct structural edge", title)
        count = int(
            first_group(count_match, "edge %s has no count title" % data_edge)
        )
        d = required_attr(attrs, "d", "edge %s has no path data" % data_edge)

        edges.append(
            {
                "from": source,
                "to": target,
                "count": count,
                "back": kind == "back",
                "onPath": False,
                "d": d,
                "className": kind,
                "order": len(edges),
            }
        )
    return edges


# ---------------------------------------------------------------------------
# metrics join
# ---------------------------------------------------------------------------

def functions_by_path(metrics):
    grouped = {}
    for function in metrics["stats"]["functions"]:
        grouped.setdefault(function["path"], []).append(function)
    for items in grouped.values():
        items.sort(
            key=lambda f: (
                -int(f["function_cost_v2"]),
                int(f["span"]["start_line"]),
                f["id"],
            )
        )
    return grouped


def join_functions(nodes, metrics):
    grouped = functions_by_path(metrics)
    for node in nodes:
        joined = []
        for function in grouped.get(node["id"], [])[:FUNCTIONS_CAP]:
            item = {
                "name": function["name"],
                "cost": int(function["function_cost_v2"]),
                "line": int(function["span"]["start_line"]),
                "fanIn": int(function.get("call_fanin", 0)),
                "fanOut": int(function.get("call_fanout", 0)),
                "maxLoopDepth": int(function.get("max_loop_depth", 0)),
                "unknownTripLoops": int(function.get("unknown_trip_loops", 0)),
                "tripSource": function.get("trip_count_source") or "unknown",
                "recursive": bool(function.get("is_recursive", False)),
                "id": function["id"],
            }
            if function["name"] == "<module>":
                item["module"] = True
            joined.append(item)
        node["functions"] = joined


def global_findings(metrics, wanted_paths=None):
    items = []
    for finding in metrics["findings"]:
        if wanted_paths is not None and finding["path"] not in wanted_paths:
            continue
        items.append(
            {
                "path": finding["path"],
                "line": int(finding["span"]["start_line"]),
                "severity": str(finding["severity"]).lower(),
                "ruleId": finding["rule_id"],
                "title": finding["title"],
                "symbol": finding.get("symbol", ""),
            }
        )
    items.sort(key=lambda f: (f["path"], f["line"], f["ruleId"]))
    return items[:FINDINGS_CAP]


def derive_state(nodes, meta):
    caps_count = re.search(r"(\d[\d,]*)\s+aggregated", meta["caps"])
    partial_match = re.search(r"(\d[\d,]*)\s+partial", meta["footer"])
    return {
        "empty": False,
        "noEntry": False,
        "library": "library root" in meta["entry"].lower(),
        "allTest": bool(nodes) and all(node["nonproduction"] for node in nodes),
        "capped": bool(caps_count) and int(caps_count.group(1).replace(",", "")) > 0,
        "partialParse": bool(partial_match)
        and int(partial_match.group(1).replace(",", "")) > 0,
        "singleFile": len(nodes) == 1,
    }


# ---------------------------------------------------------------------------
# assembly + assertions
# ---------------------------------------------------------------------------

def build_real_case(spec):
    report_path = spec["report"]
    metrics_path = spec["metrics"]
    parsed = parse_report(report_path)
    metrics = read_json(metrics_path)

    nodes = parsed["nodes"]
    edges = parsed["edges"]
    trace = parsed["trace"]
    meta = parsed["meta"]

    scan = metrics["scan"]
    meta["repo"] = os.path.basename(str(scan["root_path"]).rstrip("/\\"))
    meta["fileCount"] = int(scan["file_count"])
    meta["locCode"] = int(scan["loc_total"]["code"])
    meta["languages"] = list(scan["languages"])
    meta["stub"] = None

    join_functions(nodes, metrics)
    # The sim only renders findings for drawn file nodes; filtering before the
    # cap keeps every drawn node's findings (an alphabetical slice dropped the
    # cJSON unity tests entirely).
    findings = global_findings(metrics, {node["id"] for node in nodes})

    node_ids = {node["id"] for node in nodes}
    require(len(node_ids) == len(nodes), "duplicate node ids")

    # trace -> onPath edges (direction strictly from -> to)
    consecutive = [
        (trace[i]["path"], trace[i + 1]["path"]) for i in range(len(trace) - 1)
    ]
    pairs = set(consecutive)
    for edge in edges:
        if (edge["from"], edge["to"]) in pairs:
            edge["onPath"] = True

    expected = spec["expected"]
    require(
        parsed["stage"] == {"w": 1400, "h": 718},
        "%s: unexpected stage %r (the sim is built for 1400x718)" % (spec["case"], parsed["stage"]),
    )
    require(len(nodes) > 0, "no nodes for %s" % spec["case"])
    require(len(edges) > 0, "no edges for %s" % spec["case"])
    require(len(parsed["hulls"]) > 0, "no hulls for %s" % spec["case"])
    require(len(parsed["columns"]) > 0, "no columns for %s" % spec["case"])

    require(
        len(nodes) == expected["nodes"],
        "%s: expected %d nodes, got %d" % (spec["case"], expected["nodes"], len(nodes)),
    )
    require(
        len(edges) == expected["edges"],
        "%s: expected %d edges, got %d" % (spec["case"], expected["edges"], len(edges)),
    )
    require(
        len(parsed["hulls"]) == expected["hulls"],
        "%s: expected %d hulls, got %d"
        % (spec["case"], expected["hulls"], len(parsed["hulls"])),
    )
    require(
        len(parsed["columns"]) == expected["columns"],
        "%s: expected %d columns, got %d"
        % (spec["case"], expected["columns"], len(parsed["columns"])),
    )
    require(
        len(trace) == expected["trace"],
        "%s: expected %d trace steps, got %d"
        % (spec["case"], expected["trace"], len(trace)),
    )

    # every edge endpoint exists
    for edge in edges:
        require(
            edge["from"] in node_ids,
            "%s: edge source missing node %r" % (spec["case"], edge["from"]),
        )
        require(
            edge["to"] in node_ids,
            "%s: edge target missing node %r" % (spec["case"], edge["to"]),
        )

    # every trace path exists and the badge agrees with the step number
    traced = {item["path"]: item["step"] for item in trace}
    require(len(traced) == len(trace), "%s: duplicate trace paths" % spec["case"])
    require(
        sorted(traced.values()) == list(range(1, len(trace) + 1)),
        "%s: trace steps are not 1..%d" % (spec["case"], len(trace)),
    )
    for node in nodes:
        if node["pathStep"] is not None:
            require(
                node["id"] in traced,
                "%s: pathStep node %r absent from trace" % (spec["case"], node["id"]),
            )
            require(
                traced[node["id"]] == node["pathStep"],
                "%s: pathStep mismatch for %r" % (spec["case"], node["id"]),
            )
    for path, step in traced.items():
        require(path in node_ids, "%s: trace node missing %r" % (spec["case"], path))
        node = next(item for item in nodes if item["id"] == path)
        require(
            node["pathStep"] == step,
            "%s: trace step %d node %r has no matching badge"
            % (spec["case"], step, path),
        )

    # one drawn edge per consecutive trace pair, marked onPath
    for source, target in consecutive:
        matches = [e for e in edges if e["from"] == source and e["to"] == target]
        require(
            matches,
            "%s: trace edge %s -> %s is not drawn" % (spec["case"], source, target),
        )
        require(
            all(edge["onPath"] for edge in matches),
            "%s: trace edge %s -> %s not onPath" % (spec["case"], source, target),
        )
    on_path = [edge for edge in edges if edge["onPath"]]
    require(
        len(on_path) == len(consecutive),
        "%s: expected %d onPath edges, got %d"
        % (spec["case"], len(consecutive), len(on_path)),
    )

    require(sum(node["cost"] for node in nodes) > 0, "%s: node cost sum is 0" % spec["case"])
    require(bool(meta["entry"].strip()), "%s: empty entry" % spec["case"])
    require(bool(meta["hottest"].strip()), "%s: empty hottest" % spec["case"])
    require(bool(meta["footer"].strip()), "%s: empty footer" % spec["case"])

    return {
        "case": spec["case"],
        "synthetic": False,
        "source": {
            "renderDir": report_path.parent.name,
            "report": report_path.name,
            "metrics": metrics_path.name,
            "schemaVersion": str(metrics["schema_version"]),
        },
        "meta": {
            "repo": meta["repo"],
            "kicker": meta["kicker"],
            "wordmark": meta["wordmark"],
            "subline": meta["subline"],
            "entry": meta["entry"],
            "hottest": meta["hottest"],
            "path": meta["path"],
            "caps": meta["caps"],
            "footer": meta["footer"],
            "fileCount": meta["fileCount"],
            "locCode": meta["locCode"],
            "languages": meta["languages"],
            "actions": meta["actions"],
            "findingsTotal": len(metrics["findings"]),
            "findingsIncluded": len(findings),
            "stub": None,
        },
        "stage": parsed["stage"],
        "hulls": parsed["hulls"],
        "columns": parsed["columns"],
        "nodes": nodes,
        "edges": edges,
        "trace": trace,
        "state": derive_state(nodes, meta),
        "findings": findings,
    }


def build_synthetic(case_id, stub, state):
    return {
        "case": case_id,
        "synthetic": True,
        "source": {
            "renderDir": None,
            "report": None,
            "metrics": None,
            "schemaVersion": None,
        },
        "meta": {
            "repo": "state demo",
            "kicker": KICKER,
            "wordmark": WORDMARK,
            "subline": SUBLINE,
            "entry": stub["entry"],
            "hottest": "no hotspot recorded",
            "path": "0 steps",
            "caps": "state demo · no canvas nodes",
            "footer": "state demo · synthetic state, not a scan",
            "fileCount": 0,
            "locCode": 0,
            "languages": [],
            "actions": [],
            "findingsTotal": 0,
            "findingsIncluded": 0,
            "stub": stub,
        },
        "stage": {"w": 1400, "h": 718},
        "hulls": [],
        "columns": [],
        "nodes": [],
        "edges": [],
        "trace": [],
        "state": state,
        "findings": [],
    }


def no_forward_path_case():
    stub = {
        "title": "no forward path",
        "body": (
            "Library root elected (no production main). No forward edges exist; "
            "callers appear as reverse context."
        ),
        "entry": "library root (no production main) · state demo",
        "path": "0 steps",
        "caps": "state demo · no canvas nodes",
        "footer": "state demo · synthetic state, not a scan",
    }
    state = {
        "empty": False,
        "noEntry": False,
        "library": True,
        "allTest": False,
        "capped": False,
        "partialParse": False,
        "singleFile": False,
    }
    return build_synthetic("no-forward-path", stub, state)


def empty_case():
    stub = {
        "title": "no source functions recorded",
        "body": (
            "The scan produced no source functions, so the canvas is a named stub. "
            "This is a state demo, not a scan."
        ),
        "entry": "no entry · state demo",
        "path": "0 steps",
        "caps": "state demo · no canvas nodes",
        "footer": "state demo · synthetic state, not a scan",
    }
    state = {
        "empty": True,
        "noEntry": True,
        "library": False,
        "allTest": False,
        "capped": False,
        "partialParse": False,
        "singleFile": False,
    }
    return build_synthetic("empty", stub, state)


def capped_case():
    stub = {
        "title": "capped canvas",
        "body": (
            "In a capped scan, the canvas draws the top-cost files per depth plus aggregated "
            "boxes. This state demo carries no fabricated nodes or counts; the story stops at "
            "the cap. It is a state demo, not a scan."
        ),
        "entry": "capped · state demo",
        "path": "0 steps",
        "caps": "state demo · capped, no canvas nodes",
        "footer": "state demo · synthetic state, not a scan",
    }
    state = {
        "empty": False,
        "noEntry": False,
        "library": False,
        "allTest": False,
        "capped": True,
        "partialParse": False,
        "singleFile": False,
    }
    return build_synthetic("capped", stub, state)


INDEX = {
    "cases": [
        {
            "id": "hyperfine",
            "label": "hyperfine · Rust + Python · application",
            "file": "data-hyperfine.json",
            "synthetic": False,
        },
        {
            "id": "cjson",
            "label": "cJSON · C · test-dominated",
            "file": "data-cjson.json",
            "synthetic": False,
        },
        {
            "id": "no-forward-path",
            "label": "state demo · no forward path",
            "file": "data-no-forward-path.json",
            "synthetic": True,
        },
        {
            "id": "empty",
            "label": "state demo · empty scan",
            "file": "data-empty.json",
            "synthetic": True,
        },
        {
            "id": "capped",
            "label": "state demo · capped canvas",
            "file": "data-capped.json",
            "synthetic": True,
        },
    ]
}


def serialize(payload):
    return json.dumps(payload, sort_keys=True, indent=2, ensure_ascii=False) + "\n"


def write_output(name, payload):
    path = OUT_DIR / name
    data = serialize(payload).encode("utf-8")
    path.write_bytes(data)
    return path, data


def main():
    payloads = []
    for spec in REAL_CASES:
        payloads.append((spec["case"], build_real_case(spec)))
    payloads.append(("no-forward-path", no_forward_path_case()))
    payloads.append(("empty", empty_case()))
    payloads.append(("capped", capped_case()))

    filenames = {
        "hyperfine": "data-hyperfine.json",
        "cjson": "data-cjson.json",
        "no-forward-path": "data-no-forward-path.json",
        "empty": "data-empty.json",
        "capped": "data-capped.json",
    }

    written = []
    for case_id, payload in payloads:
        filename = filenames[case_id]
        path, data = write_output(filename, payload)
        written.append((filename, data))
        print(
            "%s: %d nodes · %d edges · %d hulls · %d columns · %d trace steps · "
            "%d findings -> %s"
            % (
                case_id,
                len(payload["nodes"]),
                len(payload["edges"]),
                len(payload["hulls"]),
                len(payload["columns"]),
                len(payload["trace"]),
                len(payload["findings"]),
                filename,
            )
        )

    index_path, index_data = write_output("data-index.json", INDEX)
    written.append(("data-index.json", index_data))
    print(
        "index: %d cases -> data-index.json" % len(INDEX["cases"])
    )

    for filename, data in written:
        print("sha256 %s %s" % (hashlib.sha256(data).hexdigest(), filename))


if __name__ == "__main__":
    main()
