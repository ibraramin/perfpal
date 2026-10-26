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
import re
import sys
from pathlib import Path

CASES = (("hyperfine", "out-hyperfine"), ("cjson", "out-cjson"))

# S2 edge classification is computed by Perfpal core; the sim only projects it.
EDGE_LABELS = ("type-only", "literal-observed", "unresolved")
# S3 I/O effects are core-classified too (fixed call-name table v1).
IO_OPS = ("read", "write")
IO_TARGETS = ("file", "env", "net", "stdio")
IO_ROWS_PER_FILE = 6

# S4 story-only blocks are projected, never re-ranked or recomputed.
PROFILE_ROWS = 12
SCENE_ROWS = 24
SCENE_TOP_FILES = 5
UNRESOLVED_ROWS = 6
PURPOSE_MAX_CHARS = 201  # core caps at 200 chars plus a trailing ellipsis
WITHHELD_PURPOSE = {
    "kind": "withheld",
    "source": None,
    "text": "No static purpose sentence.",
    "version": 1,
}

# F4 honesty filter. These mirror `sim/verify.py`; keep the two in sync so a
# claim can never be rejected by the gate after entering a data file.
FORBIDDEN_CLAIMS = (
    r"speedup",
    r"call stack",
    r"execution order",
    r"runs \d+ times",
    r"% of traffic",
)
TIME_UNITS = r"\b\d+(?:\.\d+)?\s?(?:ms|µs|μs|milliseconds?|seconds?)\b"
# "runtime" is allowed only inside the honesty strip's negation; bare
# execution/iteration claims are banned unless the same clause negates them.
# These mirror sim/verify.py's F4 constants exactly — keep them in sync.
NEGATED_RUNTIME = re.compile(r"(?<!not a )runtime", re.I)
NEGATABLE_CLAIMS = (r"\bexecution\b", r"\bexecuted\b", r"\biterations\b")
NEGATION = re.compile(
    r"\b(?:not|no|nothing|never|without|none|isn't|aren't|doesn't|don't|cannot|can't)\b",
    re.I,
)


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


def unnegated_hits(text: str, patterns: tuple[str, ...]) -> list[str]:
    """Matches not preceded by a negation in the same 40-char window.

    Mirrors ``sim/verify.py``'s F4 helper: "nothing is executed" and "not
    measured iterations" pass; bare claims do not.
    """
    hits = []
    for pattern in patterns:
        for match in re.finditer(pattern, text, re.I):
            window = text[max(0, match.start() - 40):match.start()]
            if NEGATION.search(window):
                continue
            hits.append(match.group(0))
    return hits


def forbidden_purpose(text: str) -> list[str]:
    """Full F4 hits in a core purpose sentence (mirrors sim/verify.py)."""
    hits = [pattern for pattern in FORBIDDEN_CLAIMS if re.search(pattern, text, re.I)]
    if re.search(TIME_UNITS, text, re.I):
        hits.append("time-unit")
    if NEGATED_RUNTIME.search(text):
        hits.append("runtime")
    hits.extend(unnegated_hits(text, NEGATABLE_CLAIMS))
    return hits


def purpose_projection(layout: dict) -> dict:
    """Project the core purpose block, applying the story honesty filter."""
    block = layout.get("purpose")
    if not isinstance(block, dict):
        raise SystemExit("layout has no purpose block (run the scan with --story)")
    kind = block.get("kind")
    source = block.get("source")
    text = block.get("text")
    version = block.get("purpose_version")
    if kind not in ("readme", "structural"):
        raise SystemExit(f"purpose kind outside readme/structural: {kind!r}")
    if source is not None and not isinstance(source, str):
        raise SystemExit(f"purpose source is not a string/null: {source!r}")
    if not isinstance(text, str) or not text.strip():
        raise SystemExit(f"purpose text is empty: {text!r}")
    if len(text) > PURPOSE_MAX_CHARS:
        raise SystemExit(f"purpose text exceeds {PURPOSE_MAX_CHARS} chars: {len(text)}")
    if version != 1:
        raise SystemExit(f"unsupported purpose_version: {version!r}")
    if forbidden_purpose(text):
        return dict(WITHHELD_PURPOSE)
    return {"kind": kind, "source": source, "text": text, "version": version}


def profile_projection(layout: dict, metrics: dict) -> tuple[list[dict], dict]:
    """Project the authoritative whole-repo queue in core order."""
    block = layout.get("profile")
    if not isinstance(block, dict):
        raise SystemExit("layout has no profile block (run the scan with --story)")
    rows = block.get("rows")
    if not isinstance(rows, list):
        raise SystemExit("layout.profile.rows is not a list")
    shown = block.get("shown")
    candidate_total = block.get("candidate_total")
    production_ranked = block.get("production_ranked")
    total_ranked = block.get("total_ranked")
    demoted_production = block.get("demoted_production")
    version = block.get("queue_version")
    if shown != len(rows):
        raise SystemExit(f"profile.shown {shown} != rows {len(rows)}")
    if not isinstance(shown, int) or shown > PROFILE_ROWS:
        raise SystemExit(f"profile.shown outside 0..{PROFILE_ROWS}: {shown!r}")
    for name, value in (
        ("candidate_total", candidate_total),
        ("production_ranked", production_ranked),
        ("total_ranked", total_ranked),
        ("demoted_production", demoted_production),
    ):
        if not isinstance(value, int) or value < 0:
            raise SystemExit(f"profile.{name} is not a non-negative integer: {value!r}")
    assert isinstance(shown, int)
    assert isinstance(candidate_total, int)
    assert isinstance(production_ranked, int)
    assert isinstance(total_ranked, int)
    if candidate_total < shown:
        raise SystemExit(f"profile.candidate_total {candidate_total} < shown {shown}")
    if production_ranked < candidate_total:
        raise SystemExit(
            f"profile.production_ranked {production_ranked} < candidate_total {candidate_total}"
        )
    if total_ranked < production_ranked:
        raise SystemExit(
            f"profile.total_ranked {total_ranked} < production_ranked {production_ranked}"
        )
    if version != 1:
        raise SystemExit(f"unsupported queue_version: {version!r}")

    functions = {function["id"]: function for function in metrics["stats"]["functions"]}
    projected: list[dict] = []
    ranks: list[int] = []
    for row in rows:
        if row.get("production") is not True:
            raise SystemExit(f"profile row is not production: {row.get('id')!r}")
        rank = row.get("rank")
        if not isinstance(rank, int) or rank < 1:
            raise SystemExit(f"profile row rank is invalid: {rank!r}")
        ranks.append(rank)
        function_id = row.get("id")
        if function_id not in functions:
            raise SystemExit(f"profile row id missing from metrics.stats.functions: {function_id!r}")
        function = functions[function_id]
        projected.append(
            {
                "id": function_id,
                "name": row["name"],
                "path": row["path"],
                "line": row["line"],
                "cost": row["cost"],
                "fanIn": function.get("call_fanin"),
                "fanOut": function.get("call_fanout"),
                "loopDepth": function.get("max_loop_depth"),
                "tripSource": function.get("trip_count_source"),
                "unknownTripLoops": row["unknown_trip_loops"],
                "recursive": function.get("is_recursive"),
                "production": row["production"],
                "rank": rank,
                "costText": row["cost_text"],
                "pathClass": row["path_class"],
                "knownTripValue": row["known_trip_value"],
                "demotions": row["demotions"],
            }
        )
    if ranks != sorted(ranks) or len(set(ranks)) != len(ranks):
        raise SystemExit(f"profile ranks are not ascending in core order: {ranks}")

    meta = {
        "shown": shown,
        "candidateTotal": candidate_total,
        "productionRanked": production_ranked,
        "totalRanked": total_ranked,
        "demotedProduction": demoted_production,
        "queueVersion": version,
    }
    return projected, meta


def scenes_projection(layout: dict, metrics: dict) -> tuple[list[dict], dict]:
    """Project the directory scenes and their bounded top-file lists."""
    block = layout.get("scenes")
    if not isinstance(block, dict):
        raise SystemExit("layout has no scenes block (run the scan with --story)")
    rows = block.get("rows")
    if not isinstance(rows, list):
        raise SystemExit("layout.scenes.rows is not a list")
    scene_cap = block.get("scene_cap")
    file_cap = block.get("file_cap")
    total_scenes = block.get("total_scenes")
    shown_scenes = block.get("shown_scenes")
    total_files = block.get("total_files")
    shown_files = block.get("shown_files")
    omitted = block.get("omitted")
    version = block.get("scene_version")
    if not isinstance(omitted, dict):
        raise SystemExit("layout.scenes.omitted is missing")
    for name, value in (
        ("scene_cap", scene_cap),
        ("file_cap", file_cap),
        ("total_scenes", total_scenes),
        ("shown_scenes", shown_scenes),
        ("total_files", total_files),
        ("shown_files", shown_files),
    ):
        if not isinstance(value, int) or value < 0:
            raise SystemExit(f"scenes.{name} is not a non-negative integer: {value!r}")
    assert isinstance(scene_cap, int)
    assert isinstance(file_cap, int)
    assert isinstance(total_scenes, int)
    assert isinstance(shown_scenes, int)
    assert isinstance(total_files, int)
    assert isinstance(shown_files, int)
    if shown_scenes != len(rows):
        raise SystemExit(f"scenes.shown_scenes {shown_scenes} != rows {len(rows)}")
    if shown_scenes > SCENE_ROWS:
        raise SystemExit(f"scenes.shown_scenes exceeds {SCENE_ROWS}: {shown_scenes}")
    if file_cap > SCENE_TOP_FILES:
        raise SystemExit(f"scenes.file_cap exceeds {SCENE_TOP_FILES}: {file_cap}")
    metrics_files = {file["path"]: file for file in metrics["files"]}
    source_files = [file for file in metrics["files"] if file["function_count"] > 0]
    expected_total_files = len(source_files)
    production_files_total = sum(
        1 for file in source_files if file.get("path_class") == "production"
    )
    non_production_files_total = expected_total_files - production_files_total
    if total_files != expected_total_files:
        raise SystemExit(
            f"scenes.total_files {total_files} != metrics files with functions "
            f"{expected_total_files}"
        )
    row_files = sum(row["files"] for row in rows)
    if shown_files != row_files:
        raise SystemExit(f"scenes.shown_files {shown_files} != sum(row.files) {row_files}")
    omitted_files = omitted.get("files")
    if not isinstance(omitted_files, int) or omitted_files < 0:
        raise SystemExit(f"scenes.omitted.files is invalid: {omitted_files!r}")
    if row_files + omitted_files != total_files:
        raise SystemExit(
            f"scene files {row_files} + omitted {omitted_files} != total {total_files}"
        )
    if omitted.get("scenes") != total_scenes - shown_scenes:
        raise SystemExit(
            f"scenes.omitted.scenes {omitted.get('scenes')} != "
            f"total_scenes - shown_scenes {total_scenes - shown_scenes}"
        )
    for name in ("cost", "production_cost", "findings"):
        value = omitted.get(name)
        if not isinstance(value, int) or value < 0:
            raise SystemExit(f"scenes.omitted.{name} is invalid: {value!r}")

    projected: list[dict] = []
    for row in rows:
        path = row["path"]
        files = row["files"]
        top_files = row["top_files"]
        if not isinstance(path, str) or not path:
            raise SystemExit(f"scene path is invalid: {path!r}")
        if not isinstance(files, int) or files < 1:
            raise SystemExit(f"scene {path} has no files")
        if not isinstance(top_files, list) or not top_files:
            raise SystemExit(f"scene {path} has no top files")
        if len(top_files) > SCENE_TOP_FILES:
            raise SystemExit(f"scene {path} has {len(top_files)} top files > {SCENE_TOP_FILES}")
        for name in ("cost", "production_cost", "findings", "production_files"):
            value = row.get(name)
            if not isinstance(value, int) or value < 0:
                raise SystemExit(f"scene {path}.{name} is invalid: {value!r}")
        if top_files != sorted(top_files, key=lambda file: (-file["cost"], file["path"])):
            raise SystemExit(f"scene {path} top files are not (cost desc, path asc)")
        top_rows = []
        for file in top_files:
            file_path = file["path"]
            if file_path not in metrics_files:
                raise SystemExit(f"scene {path} top file missing from metrics: {file_path!r}")
            if not isinstance(file["cost"], int) or file["cost"] < 0:
                raise SystemExit(f"scene {path} file {file_path} has an invalid cost")
            if path == "(root)":
                contained = "/" not in file_path
            else:
                contained = file_path.startswith(path + "/")
            if not contained:
                raise SystemExit(f"scene {path} is not a prefix of its top file {file_path!r}")
            top_rows.append(
                {
                    "path": file_path,
                    "name": file["name"],
                    "cost": file["cost"],
                    "scale": file["scale"],
                    "production": file["production"],
                    "findings": file["findings"],
                    "functionCount": file["function_count"],
                    "entry": file["entry"],
                    "hottest": file["hottest"],
                }
            )
        projected.append(
            {
                "path": path,
                "depth": row["depth"],
                "files": files,
                "productionFiles": row["production_files"],
                "findings": row["findings"],
                "cost": row["cost"],
                "productionCost": row["production_cost"],
                "scale": row["scale"],
                "entry": row["entry"],
                "hottest": row["hottest"],
                "topFiles": top_rows,
            }
        )
    if version != 1:
        raise SystemExit(f"unsupported scene_version: {version!r}")

    meta = {
        "sceneCap": scene_cap,
        "fileCap": file_cap,
        "totalScenes": total_scenes,
        "shownScenes": shown_scenes,
        "totalFiles": total_files,
        "shownFiles": shown_files,
        "productionFiles": production_files_total,
        "nonProductionFiles": non_production_files_total,
        "omitted": {
            "scenes": omitted["scenes"],
            "files": omitted_files,
            "cost": omitted["cost"],
            "productionCost": omitted["production_cost"],
            "findings": omitted["findings"],
        },
        "sceneVersion": version,
    }
    return projected, meta


def unresolved_by_path(rows: list[dict], drawn: set[str]) -> dict[str, dict]:
    """Core unresolved-call rows grouped by their drawn call-site file."""
    grouped: dict[str, list[dict]] = {}
    for row in rows:
        if not row.get("unresolved"):
            continue
        path = row.get("from")
        if path not in drawn:
            raise SystemExit(f"unresolved call on a non-drawn file: {row}")
        line = row.get("line")
        callee = row.get("callee")
        caller = row.get("caller")
        if not isinstance(line, int) or line < 1:
            raise SystemExit(f"unresolved call line is not a positive integer: {row}")
        if not isinstance(callee, str) or not callee:
            raise SystemExit(f"unresolved call callee is invalid: {row}")
        if not isinstance(caller, str) or not caller:
            raise SystemExit(f"unresolved call caller is invalid: {row}")
        grouped.setdefault(path, []).append(
            {"line": line, "callee": callee, "caller": caller}
        )
    result: dict[str, dict] = {}
    for path, grouped_rows in grouped.items():
        grouped_rows.sort(key=lambda row: (row["line"], row["callee"], row["caller"]))
        result[path] = {
            "count": len(grouped_rows),
            "rows": grouped_rows[:UNRESOLVED_ROWS],
        }
    return result


def node_row(node: dict, io: dict | None = None, unresolved: dict | None = None) -> dict:
    row = {
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
        "scene": node["scene"],
        "functions": [function_row(function) for function in node["functions"]],
    }
    if io:
        row["io"] = io
    if unresolved:
        row["unresolvedCalls"] = unresolved
    return row


def io_rows_by_path(rows: list[dict], drawn: set[str]) -> dict[str, list[dict]]:
    """Core `flow.io[]` rows grouped by their drawn call-site file.

    The row's ``file`` is the call site, never the target path. Rows are
    validated against the spec enums here so a core regression cannot reach the
    story as an invented class; stdio rows never carry a path literal.
    """
    grouped: dict[str, list[dict]] = {}
    for row in rows:
        path = row.get("file")
        if path not in drawn:
            raise SystemExit(f"io row on a non-drawn file: {row}")
        op = row.get("op")
        target = row.get("target")
        literal = row.get("literal")
        if op not in IO_OPS:
            raise SystemExit(f"io op outside read/write: {row}")
        if target not in IO_TARGETS:
            raise SystemExit(f"io target outside the contract: {row}")
        if literal is not None and not isinstance(literal, str):
            raise SystemExit(f"io literal is not a string/null: {row}")
        if target == "stdio" and literal is not None:
            raise SystemExit(f"stdio io row carries a path literal: {row}")
        line = row.get("line")
        if not isinstance(line, int) or line < 1:
            raise SystemExit(f"io line is not a positive integer: {row}")
        grouped.setdefault(path, []).append(
            {
                "line": line,
                "callee": row["callee"],
                "op": op,
                "target": target,
                "literal": literal,
            }
        )
    for grouped_rows in grouped.values():
        grouped_rows.sort(
            key=lambda row: (
                row["line"],
                row["callee"],
                row["op"],
                row["target"],
                row["literal"] or "",
            )
        )
    return grouped


def io_summary(rows: list[dict] | None) -> dict | None:
    """Per-file I/O summary: totals by op/target, literal/unknown counts, rows."""
    if not rows:
        return None
    return {
        "total": len(rows),
        "read": sum(1 for row in rows if row["op"] == "read"),
        "write": sum(1 for row in rows if row["op"] == "write"),
        "file": sum(1 for row in rows if row["target"] == "file"),
        "env": sum(1 for row in rows if row["target"] == "env"),
        "net": sum(1 for row in rows if row["target"] == "net"),
        "stdio": sum(1 for row in rows if row["target"] == "stdio"),
        "literal": sum(1 for row in rows if row["literal"] is not None),
        # stdio has no path target by design; only path/name/URL rows can be
        # "unknown" when the argument is not a plain literal.
        "unknownTarget": sum(
            1 for row in rows if row["literal"] is None and row["target"] != "stdio"
        ),
        "rows": rows[:IO_ROWS_PER_FILE],
    }


def io_trust(grouped: dict[str, list[dict]]) -> dict:
    """Aggregate I/O coverage for the trust beat."""
    rows = [row for grouped_rows in grouped.values() for row in grouped_rows]
    literals = sorted(
        {(row["target"], row["literal"]) for row in rows if row["literal"] is not None}
    )
    return {
        "rows": len(rows),
        "files": sum(1 for grouped_rows in grouped.values() if grouped_rows),
        "read": sum(1 for row in rows if row["op"] == "read"),
        "write": sum(1 for row in rows if row["op"] == "write"),
        "stdio": sum(1 for row in rows if row["target"] == "stdio"),
        "literal": sum(1 for row in rows if row["literal"] is not None),
        "unknownTarget": sum(
            1 for row in rows if row["literal"] is None and row["target"] != "stdio"
        ),
        # A few written literal examples for the trust line (sorted, capped);
        # full per-file details stay in the inspector.
        "literals": [
            {"target": target, "literal": literal}
            for target, literal in literals[:4]
        ],
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


def projected_functions(nodes: list[dict], production_only: bool) -> list[dict]:
    """Drawn-node functions in the canonical cost rank.

    The core emits up to three candidates per drawn file. ``production_only``
    keeps the drawn-slice hottest comparison honest; the whole-repo profile
    queue is projected separately from ``layout.profile``.
    """
    rows: dict[str, dict] = {}
    for node in nodes:
        production = bool(node["production"])
        if production_only and not production:
            continue
        for function in node["functions"]:
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
                "production": production,
            }
    return sorted(rows.values(), key=lambda row: (-row["cost"], row["line"], row["name"]))


def hottest_function(nodes: list[dict]) -> dict | None:
    """Globally highest-cost drawn function (test/bench rows included)."""
    rows = projected_functions(nodes, production_only=False)
    return rows[0] if rows else None


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


def build(case: str, layout_path: Path, metrics_path: Path) -> dict:
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
    io_grouped = io_rows_by_path(flow.get("io") or [], drawn)
    io_totals = io_trust(io_grouped)
    unknown_trips = unknown_trip_functions(layout["nodes"])

    purpose = purpose_projection(layout)
    profile, profile_meta = profile_projection(layout, metrics)
    scenes, scene_meta = scenes_projection(layout, metrics)
    unresolved = unresolved_by_path(flow.get("calls") or [], drawn)
    unresolved_total = sum(entry["count"] for entry in unresolved.values())
    unresolved_files = len(unresolved)

    for node in layout["nodes"]:
        if not isinstance(node.get("scene"), str):
            raise SystemExit(f"{case}: node {node['path']} has no string scene")
    actions = []
    for action in layout["actions"]:
        if not isinstance(action.get("scene"), str):
            raise SystemExit(f"{case}: action {action['location']} has no string scene")
        actions.append(
            {
                "anchor": action["location"],
                "title": action["rule"],
                "function": action["function"],
                "onCanvas": anchor_path(action["location"]) in drawn,
                "scene": action["scene"],
            }
        )

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
            "actions": actions,
        },
        "trust": {
            "caps": meta["caps_text"],
            "partialParse": state["partial_parse"],
            "nonProductionNodes": sum(1 for node in layout["nodes"] if not node["production"]),
            "drawnNodes": len(layout["nodes"]),
            "unknownTripFunctions": unknown_trips,
            "io": io_totals,
            "findingsIncluded": meta["findings_included"],
            "findingsTotal": meta["findings_total"],
            "purpose": {"kind": purpose["kind"], "source": purpose["source"]},
            "profile": {
                "shown": profile_meta["shown"],
                "candidateTotal": profile_meta["candidateTotal"],
                "productionRanked": profile_meta["productionRanked"],
                "totalRanked": profile_meta["totalRanked"],
                "demotedProduction": profile_meta["demotedProduction"],
            },
            "scenes": {
                "total": scene_meta["totalScenes"],
                "shown": scene_meta["shownScenes"],
                "files": scene_meta["totalFiles"],
                "shownFiles": scene_meta["shownFiles"],
                "productionFiles": scene_meta["productionFiles"],
                "nonProductionFiles": scene_meta["nonProductionFiles"],
                "omitted": scene_meta["omitted"]["scenes"],
            },
            "unresolvedCalls": unresolved_total,
            "unresolvedFiles": unresolved_files,
        },
        "purpose": purpose,
        "profile": profile,
        "profileMeta": profile_meta,
        "scenes": scenes,
        "sceneMeta": scene_meta,
        "hottestFunction": hottest_function(layout["nodes"]),
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
        "nodes": [
            node_row(node, io_summary(io_grouped.get(node["path"])), unresolved.get(node["path"]))
            for node in layout["nodes"]
        ],
        "edges": [edge_row(edge, order, calls) for order, edge in enumerate(layout["edges"])],
        "trace": [
            {"path": stage["files"][0], "step": stage["order"] + 1}
            for stage in layout["stages"]
            if stage["files"]
        ],
        "findings": findings,
    }


def write_payload(target: Path, payload: dict) -> None:
    target.write_text(json.dumps(payload, indent=1, sort_keys=True) + "\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", default="sim/source", help="scan output root (default: sim/source)")
    parser.add_argument("--sim", default="sim", help="sim directory to write data files into")
    parser.add_argument("--layout", help="one atlas-layout.json (with --metrics, --out, --case)")
    parser.add_argument("--metrics", help="one metrics.json (with --layout, --out, --case)")
    parser.add_argument("--out", help="one data JSON target (with --layout, --metrics, --case)")
    parser.add_argument("--case", help="one case name (with --layout, --metrics, --out)")
    args = parser.parse_args()

    single = (args.layout, args.metrics, args.out, args.case)
    if any(single):
        if not all(single):
            parser.error("--layout, --metrics, --out and --case must be given together")
        payload = build(args.case, Path(args.layout), Path(args.metrics))
        target = Path(args.out)
        write_payload(target, payload)
        print(
            f"{target}: {len(payload['nodes'])} nodes, {len(payload['edges'])} edges, "
            f"{len(payload['findings'])} findings, layout sha {payload['source']['sha256'][:12]}"
        )
        return 0

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
        payload = build(case, out_dir / "atlas-layout.json", out_dir / "metrics.json")
        target = sim / f"data-{case}.json"
        write_payload(target, payload)
        print(f"{target}: {len(payload['nodes'])} nodes, {len(payload['edges'])} edges, "
              f"{len(payload['findings'])} findings, layout sha {payload['source']['sha256'][:12]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
