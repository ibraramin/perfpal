#!/usr/bin/env python3
"""Build the Atlas Story data files from Perfpal core artifacts.

Inputs, both emitted by the Perfpal scan itself (never scraped from HTML):

    source/out-<case>/atlas-layout.json   geometry + story projection (`--story`)
    source/out-<case>/metrics.json        findings (the machine source of truth)

Output: ``sim/data-<case>.json`` in the sim's internal shape, plus a
``source`` block carrying the SHA-256 of both input buffers (the exact bytes
that were parsed) so the provenance can be re-checked.

Deterministic: identical inputs produce byte-identical output (sorted keys,
sorted arrays, no timestamps).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path

CASES = (("hyperfine", "out-hyperfine"), ("cjson", "out-cjson"))

# S2 edge classification is computed by Perfpal core; the sim only projects it.
EDGE_LABELS = ("type-only", "literal-observed", "unresolved")


def content_sha256(value: object) -> str:
    """Hash canonical JSON with scan provenance removed.

    ``metrics.json`` carries ``scan.started_at``, ``scan.duration_ms`` and
    ``scan.root_path`` (an absolute path), which change per run/machine. The
    story data must stay byte-identical for the same tree anywhere, so that
    provenance block is stripped before hashing; the field is named
    ``metricsContentSha256`` rather than the file bytes' hash.
    """
    probe = json.loads(json.dumps(value))
    scan = probe.get("scan")
    if isinstance(scan, dict):
        for key in ("started_at", "duration_ms", "root_path"):
            scan.pop(key, None)
    payload = json.dumps(probe, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(payload).hexdigest()


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def function_row(function: dict) -> dict:
    return {
        "id": function["id"],
        "name": function["name"],
        "line": function["line"],
        "cost": function["cost"],
        "maxLoopDepth": function["loop_depth"],
        "tripSource": function["trip_source"],
        "unknownTripLoops": function["unknown_trip_loops"],
        "fanIn": function["fan_in"],
        "fanOut": function["fan_out"],
        "recursive": function["recursive"],
    }


def node_row(node: dict) -> dict:
    return {
        "id": node["path"],
        "name": node["name"],
        "x": node["x"],
        "y": node["y"],
        "w": node["w"],
        "h": node["h"],
        "column": node["column"],
        "entry": node["entry"],
        "hottest": node["hottest"],
        "unreached": node["unreached"],
        "nonproduction": not node["production"],
        "pathStep": node["path_step"],
        "partialParse": node["partial_parse"],
        "fill": node["fill"],
        "stroke": node["stroke"],
        "strokeWidth": node["stroke_width"],
        "dash": node["dash"],
        "badge": node["badge"],
        "cost": node["cost"],
        "costText": node["cost_text"],
        "ratioText": node["ratio_text"],
        "topText": node["top_text"],
        "title": f"{node['path']} · {node['cost_text']}",
        "findings": node["findings"],
        "functions": [function_row(function) for function in node["functions"]],
    }


def first_typed_arg(call: dict) -> dict | None:
    """The first written argument that carries a type or a resolved name."""
    for arg in call["args"]:
        if arg.get("type") is None and arg.get("named") is None:
            continue
        return {
            "expr": arg["expr"],
            "type": arg["type"],
            "named": arg["named"],
            "dir": arg["dir"],
            "literal": arg["literal"],
        }
    return None


def calls_by_pair(calls: list[dict]) -> dict[tuple[str, str], list[dict]]:
    """Resolved calls grouped by their drawn ``(from, to)`` file pair."""
    grouped: dict[tuple[str, str], list[dict]] = {}
    for call in calls:
        to = call.get("to")
        if call.get("unresolved") or not to:
            continue
        grouped.setdefault((call["from"], to), []).append(call)
    for rows in grouped.values():
        rows.sort(key=lambda call: (call["line"], call["callee"], call["caller"]))
    return grouped


def edge_row(edge: dict, order: int, calls: dict[tuple[str, str], list[dict]]) -> dict:
    label = edge.get("label")
    payload = edge.get("payload")
    pair = (edge["from"], edge["to"])
    top_calls = [
        {
            "callee": call["callee"],
            "caller": call["caller"],
            "line": call["line"],
            "arg": first_typed_arg(call),
        }
        for call in calls.get(pair, [])[:2]
    ]
    return {
        "from": edge["from"],
        "to": edge["to"],
        "d": edge["d"],
        "count": edge["count"],
        "back": edge["back"],
        "onPath": edge["on_path"],
        "label": label,
        "payload": payload,
        "labelClass": "unresolved" if label == "unresolved" else "resolved",
        "topCalls": top_calls,
        "className": "back" if edge["back"] else "forward",
        "order": order,
    }


def is_test_function(path: str, name: str) -> bool:
    """Test/bench rows are excluded from the profile queue."""
    return "tests" in Path(path).parts or name.startswith("test")


def profile_rows(nodes: list[dict]) -> list[dict]:
    """Top 8 drawn functions by static cost, non-test only, deduplicated by id."""
    rows: dict[str, dict] = {}
    for node in nodes:
        for function in node["functions"]:
            if is_test_function(node["path"], function["name"]):
                continue
            rows[function["id"]] = {
                "id": function["id"],
                "name": function["name"],
                "path": node["path"],
                "line": function["line"],
                "cost": function["cost"],
                "fanIn": function["fan_in"],
                "fanOut": function["fan_out"],
                "loopDepth": function["loop_depth"],
                "tripSource": function["trip_source"],
                "unknownTripLoops": function["unknown_trip_loops"],
                "recursive": function["recursive"],
            }
    ordered = sorted(rows.values(), key=lambda row: (-row["cost"], row["line"], row["name"]))
    return ordered[:8]


def unknown_trip_functions(nodes: list[dict]) -> int:
    """Drawn functions whose loop trip count is unknown (the estimate floor)."""
    seen: dict[str, dict] = {}
    for node in nodes:
        for function in node["functions"]:
            seen[function["id"]] = function
    return sum(1 for function in seen.values() if function["unknown_trip_loops"] > 0)


def anchor_path(anchor: str) -> str:
    """``path:line`` -> ``path`` (locations are rendered with a trailing line)."""
    head, separator, tail = anchor.rpartition(":")
    return head if separator and tail.isdigit() else anchor


def findings_for(layout: dict, metrics: dict) -> list[dict]:
    """Findings on drawn files only, in a stable order.

    ``meta.findings_included`` is defined as the drawn-slice count, so this
    filter must agree with it; the converter asserts that below.
    """
    drawn = {node["path"] for node in layout["nodes"]}
    rows = []
    for finding in metrics["findings"]:
        if finding["path"] not in drawn:
            continue
        rows.append(
            {
                "path": finding["path"],
                "line": finding["span"]["start_line"],
                "ruleId": finding["rule_id"],
                "severity": finding["severity"],
                "symbol": finding.get("symbol", ""),
                "title": finding["title"],
            }
        )
    rows.sort(key=lambda row: (row["path"], row["line"], row["ruleId"]))
    return rows


def build(case: str, out_dir: Path) -> dict:
    layout_path = out_dir / "atlas-layout.json"
    metrics_path = out_dir / "metrics.json"
    layout_bytes = layout_path.read_bytes()
    metrics_bytes = metrics_path.read_bytes()
    layout = json.loads(layout_bytes)
    metrics = json.loads(metrics_bytes)

    if layout.get("layout_version") != 1:
        raise SystemExit(f"{layout_path}: unsupported layout_version {layout.get('layout_version')!r}")
    canvas = layout.get("canvas") or {}
    height = canvas.get("h")
    if (
        canvas.get("w") != 1400
        or not isinstance(height, int)
        or not 480 <= height <= 20_000
    ):
        raise SystemExit(
            f"{layout_path}: unexpected canvas {canvas} "
            "(expected width 1400 and height 480..20000; the approved fixtures are 718)"
        )

    for node in layout["nodes"]:
        if (
            node["x"] < 0
            or node["y"] < 0
            or node["x"] + node["w"] > canvas["w"]
            or node["y"] + node["h"] > canvas["h"]
        ):
            raise SystemExit(
                f"{case}: node {node['path']} outside canvas {canvas}"
            )

    state = layout["state"]
    meta = layout["meta"]
    if meta["findings_total"] != len(metrics["findings"]):
        raise SystemExit(
            f"{case}: meta.findings_total {meta['findings_total']} != "
            f"metrics.findings {len(metrics['findings'])}"
        )
    findings = findings_for(layout, metrics)
    included = meta["findings_included"]
    if len(findings) != included:
        raise SystemExit(
            f"{case}: drawn findings {len(findings)} != meta.findings_included {included}"
        )

    flow = layout.get("flow") or {}
    calls = calls_by_pair(flow.get("calls") or [])
    for edge in layout["edges"]:
        label = edge.get("label")
        if label not in EDGE_LABELS:
            raise SystemExit(
                f"{case}: edge {edge['from']} -> {edge['to']} has no S2 label "
                f"(got {label!r}; allowed: {', '.join(EDGE_LABELS)})"
            )
        if label != "unresolved" and not (isinstance(edge.get("payload"), str) and edge["payload"].strip()):
            raise SystemExit(
                f"{case}: edge {edge['from']} -> {edge['to']} is {label} without a payload"
            )

    drawn = {node["path"] for node in layout["nodes"]}
    unknown_trips = unknown_trip_functions(layout["nodes"])

    return {
        "case": case,
        "synthetic": False,
        "stage": layout["canvas"],
        "source": {
            "artifact": "atlas-layout.json",
            "layoutVersion": layout["layout_version"],
            "sha256": sha256(layout_bytes),
            "metricsContentSha256": content_sha256(metrics),
        },
        "meta": {
            "entry": meta["entry_text"],
            "hottest": meta["hottest_text"],
            "path": meta["path_text"],
            "caps": meta["caps_text"],
            "footer": meta["footer_text"],
            "findingsIncluded": meta["findings_included"],
            "findingsTotal": meta["findings_total"],
            "actions": [
                {
                    "anchor": action["location"],
                    "title": action["rule"],
                    "onCanvas": anchor_path(action["location"]) in drawn,
                }
                for action in layout["actions"]
            ],
        },
        "trust": {
            "caps": meta["caps_text"],
            "partialParse": state["partial_parse"],
            "nonProductionNodes": sum(1 for node in layout["nodes"] if not node["production"]),
            "drawnNodes": len(layout["nodes"]),
            "unknownTripFunctions": unknown_trips,
            "findingsIncluded": meta["findings_included"],
            "findingsTotal": meta["findings_total"],
        },
        "profile": profile_rows(layout["nodes"]),
        "state": {
            "empty": state["empty"],
            "noEntry": state["no_entry"],
            "library": state["library"],
            "allTest": state["all_test"],
            "capped": state["capped"],
            "partialParse": state["partial_parse"],
            "singleFile": state["single_file"],
        },
        "columns": [
            {
                "depth": column["depth"],
                "x": column["x"],
                "order": index,
                "unreached": column["unreached"],
                "label": column["label"],
                "countText": column["count_text"],
            }
            for index, column in enumerate(layout["columns"])
        ],
        "hulls": [
            {"x": hull["x"], "y": hull["y"], "w": hull["w"], "h": hull["h"], "label": hull["label"]}
            for hull in layout["hulls"]
        ],
        "nodes": [node_row(node) for node in layout["nodes"]],
        "edges": [edge_row(edge, order, calls) for order, edge in enumerate(layout["edges"])],
        "trace": [
            {"path": stage["files"][0], "step": stage["order"] + 1}
            for stage in layout["stages"]
            if stage["files"]
        ],
        "findings": findings,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", default="sim/source", help="scan output root (default: sim/source)")
    parser.add_argument("--sim", default="sim", help="sim directory to write data files into")
    args = parser.parse_args()

    source = Path(args.source)
    sim = Path(args.sim)
    for case, directory in CASES:
        out_dir = source / directory
        if not (out_dir / "atlas-layout.json").is_file():
            print(
                f"missing {out_dir / 'atlas-layout.json'}; run the scan with --story first",
                file=sys.stderr,
            )
            return 1
        payload = build(case, out_dir)
        target = sim / f"data-{case}.json"
        target.write_text(json.dumps(payload, indent=1, sort_keys=True) + "\n")
        print(f"{target}: {len(payload['nodes'])} nodes, {len(payload['edges'])} edges, "
              f"{len(payload['findings'])} findings, layout sha {payload['source']['sha256'][:12]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
