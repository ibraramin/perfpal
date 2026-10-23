#!/usr/bin/env python3
"""Build the Atlas Story data files from Perfpal core artifacts.

Inputs, both emitted by the Perfpal scan itself (never scraped from HTML):

    source/out-<case>/atlas-layout.json   geometry + story projection (`--story`)
    source/out-<case>/metrics.json        findings (the machine source of truth)

Output: ``sim/data-<case>.json`` in the sim's internal shape, plus a
``source`` block carrying the layout artifact's SHA-256 so the provenance can
be re-checked.

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


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def column_label(column: dict) -> str:
    if column["unreached"]:
        return "UNREACHED"
    if column["depth"] == 0:
        return "depth 0 · ENTRY"
    return f"depth {column['depth']}"


def column_count(column: dict) -> str:
    if column["unreached"]:
        return "no forward path"
    count = column["file_count"]
    return f"{count} file" if count == 1 else f"{count} files"


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


def edge_row(edge: dict, order: int) -> dict:
    return {
        "from": edge["from"],
        "to": edge["to"],
        "d": edge["d"],
        "count": edge["count"],
        "back": edge["back"],
        "onPath": edge["on_path"],
        "className": "back" if edge["back"] else "forward",
        "order": order,
    }


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
    layout = json.loads(layout_path.read_text())
    metrics = json.loads(metrics_path.read_text())

    if layout.get("layout_version") != 1:
        raise SystemExit(f"{layout_path}: unsupported layout_version {layout.get('layout_version')!r}")
    if layout.get("canvas") != {"w": 1400, "h": 718}:
        raise SystemExit(f"{layout_path}: unexpected canvas {layout.get('canvas')} (expected 1400x718)")

    state = layout["state"]
    meta = layout["meta"]
    findings = findings_for(layout, metrics)
    included = meta["findings_included"]
    if len(findings) != included:
        raise SystemExit(
            f"{case}: drawn findings {len(findings)} != meta.findings_included {included}"
        )

    return {
        "case": case,
        "synthetic": False,
        "stage": layout["canvas"],
        "source": {
            "artifact": "atlas-layout.json",
            "layoutVersion": layout["layout_version"],
            "sha256": sha256(layout_path),
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
                {"anchor": action["location"], "title": action["rule"]}
                for action in layout["actions"]
            ],
        },
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
                "label": column_label(column),
                "countText": column_count(column),
            }
            for index, column in enumerate(layout["columns"])
        ],
        "hulls": [
            {"x": hull["x"], "y": hull["y"], "w": hull["w"], "h": hull["h"], "label": hull["label"]}
            for hull in layout["hulls"]
        ],
        "nodes": [node_row(node) for node in layout["nodes"]],
        "edges": [edge_row(edge, order) for order, edge in enumerate(layout["edges"])],
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
