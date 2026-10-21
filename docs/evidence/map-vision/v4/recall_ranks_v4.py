#!/usr/bin/env python3
"""Reproduce the V4 shipped-key recall/rank table.

The V3 prototype ranks (`recall_ranks.json`) measure the prototype ordering.
This script measures the **shipped** ordering: the same nine pre-registered
targets, ranked by the V4 implementation (`src/reporter/views.rs`) over the
schema-1.7 scans in `/home/shafin/Opencode/perfpal-test/out-v4-<repo>`.

Shipped keys reproduced here:
- KV1: production first -> severity (hot<warn<info) -> confidence
  (HIGH<MEDIUM<LOW) -> containing-function function_cost_v2 desc ->
  finding_id asc; `-ARCH-002` component aggregates excluded.
- KV2: reverse include in-degree desc (resolved edges, distinct from) ->
  path asc; no production-first demotion.
- KV3: production first -> loop-bearing before no-loop -> non-startup before
  startup -> cost desc -> path -> name -> id (v1 fallback rows unchanged in
  these scans; every function has function_cost_v2).

Writes `recall_ranks_v4.json` and exits non-zero on drift.
"""

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]  # .../Perfpal/docs/evidence/map-vision/v4
TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
OUT = Path(__file__).resolve().parent / "recall_ranks_v4.json"

# repo, question, kind, selector, V4 (rank, total, inline), V3 (rank, total, inline)
TARGETS = [
    ("tmux", "Q1", "finding", "PERF-C-CALL-002:input.c:969:1", (209, 7543, False), (210, 7545, False)),
    ("tmux", "Q2", "file", "tmux.h", (1, 216, True), (1, 216, True)),
    ("tmux", "Q3", "func", "screen_write_collect_add|screen-write.c", (820, 3189, False), (811, 3189, False)),
    ("hyperfine", "Q1", "finding", "PERF-RS-ALLOC-001:src/benchmark/mod.rs:333:9", (172, 346, False), (197, 348, False)),
    ("hyperfine", "Q2", "file", "src/command.rs", (6, 53, True), (4, 53, True)),
    ("hyperfine", "Q3", "func", "execute_and_measure|src/timer/mod.rs", (58, 353, False), (66, 353, False)),
    ("cJSON", "Q1", "finding", "PERF-C-CALL-002:cJSON.c:307:1", (37, 835, False), (35, 837, False)),
    ("cJSON", "Q2", "file", "cJSON.h", (4, 101, True), (1, 101, True)),
    ("cJSON", "Q3", "func", "cJSON_New_Item|cJSON.c", (60, 1055, False), (58, 1055, False)),
]

SEVERITY = {"hot": 0, "warn": 1, "info": 2}
CONFIDENCE = {"HIGH": 0, "MEDIUM": 1, "LOW": 2}
COMPONENT_SUFFIX = "-ARCH-002"


def load(repo):
    path = TEST_ROOT / f"out-v4-{repo}" / "metrics.json"
    if not path.exists():
        raise SystemExit(f"FATAL: missing {path}")
    data = json.loads(path.read_text(encoding="utf-8"))
    classes = {f["path"]: f.get("path_class") or "unknown" for f in data["files"]}
    return data, classes


def production(classes, path):
    return classes.get(path, "unknown") == "production"


def function_v1(fn):
    return (
        fn["cyclomatic_complexity"]
        + fn["cognitive_complexity"]
        + 2 * fn["max_loop_depth"]
        + 3 * fn["alloc_site_count"]
        + fn.get("call_fanout", 0)
        + fn.get("call_fanin", 0)
        + fn.get("call_depth", 0)
        + (2 if fn.get("is_recursive") else 0)
        + (1 if fn.get("error_path") else 0)
        + (1 if fn.get("alloc_family") else 0)
    )


def function_cost(fn):
    value = fn.get("function_cost_v2")
    return value if value is not None else function_v1(fn)


def kv1_order(data, classes):
    functions_by_path = {}
    for index, fn in enumerate(data["stats"]["functions"]):
        functions_by_path.setdefault(fn["path"], []).append(index)

    def containing_cost(finding_index):
        finding = data["findings"][finding_index]
        line = finding["span"]["start_line"]
        candidates = [
            index
            for index in functions_by_path.get(finding["path"], [])
            if data["stats"]["functions"][index]["span"]["start_line"]
            <= line
            <= data["stats"]["functions"][index]["span"]["end_line"]
        ]
        if not candidates:
            return 0
        candidates.sort(
            key=lambda index: (
                data["stats"]["functions"][index]["span"]["end_line"]
                - data["stats"]["functions"][index]["span"]["start_line"],
                data["stats"]["functions"][index]["id"],
            )
        )
        return data["stats"]["functions"][candidates[0]].get("function_cost_v2") or 0

    rows = [
        (index, containing_cost(index))
        for index, finding in enumerate(data["findings"])
        if not finding["rule_id"].endswith(COMPONENT_SUFFIX)
    ]
    rows.sort(
        key=lambda pair: (
            not production(classes, data["findings"][pair[0]]["path"]),
            SEVERITY[data["findings"][pair[0]]["severity"]],
            CONFIDENCE[data["findings"][pair[0]]["confidence"]],
            -pair[1],
            data["findings"][pair[0]]["finding_id"],
        )
    )
    return [data["findings"][index]["finding_id"] for index, _ in rows]


def kv2_order(data):
    indegree = {}
    for edge in data.get("graphs", {}).get("include_graph", {}).get("edges", []):
        if edge.get("resolved") and edge.get("to"):
            indegree.setdefault(edge["to"], set()).add(edge["from"])
    ordering = sorted(
        [(f["path"], len(indegree.get(f["path"], ()))) for f in data["files"]],
        key=lambda pair: (-pair[1], pair[0]),
    )
    return [path for path, _ in ordering], indegree


def kv3_order(data, classes):
    functions = data["stats"]["functions"]
    ordering = sorted(
        range(len(functions)),
        key=lambda index: (
            not production(classes, functions[index]["path"]),
            functions[index]["max_loop_depth"] == 0,
            functions[index].get("entry_kind") in ("main", "script"),
            -function_cost(functions[index]),
            functions[index]["path"],
            functions[index]["name"],
            functions[index]["id"],
        ),
    )
    return ordering, functions


def main():
    models = {}
    results = []
    ok = True
    for repo, question, kind, selector, expected, v3 in TARGETS:
        if repo not in models:
            models[repo] = load(repo)
        data, classes = models[repo]

        if kind == "finding":
            ordering = kv1_order(data, classes)
            rank = ordering.index(selector) + 1
            total = len(ordering)
            inline = rank <= 5
        elif kind == "file":
            ordering, indegree = kv2_order(data)
            rank = ordering.index(selector) + 1
            total = len(ordering)
            inline = rank <= 8 and len(indegree.get(selector, ())) > 0
        else:
            name, path = selector.split("|", 1)
            ordering, functions = kv3_order(data, classes)
            index = next(
                i
                for i in ordering
                if functions[i]["name"] == name and functions[i]["path"] == path
            )
            rank = ordering.index(index) + 1
            total = len(ordering)
            non_demoted = [
                i
                for i in ordering
                if production(classes, functions[i]["path"])
                and functions[i]["max_loop_depth"] > 0
                and functions[i].get("entry_kind") not in ("main", "script")
            ]
            inline = index in non_demoted[:8]

        actual = (rank, total, inline)
        results.append(
            {
                "repo": repo,
                "question": question,
                "kind": kind,
                "target": selector,
                "rank": rank,
                "total": total,
                "inline": inline,
                "v3_rank": v3[0],
                "v3_total": v3[1],
                "v3_inline": v3[2],
            }
        )
        if actual != expected:
            ok = False
            print(
                f"MISMATCH {repo} {question}: got {actual}, expected {expected}",
                file=sys.stderr,
            )

    payload = {
        "_comment": "V4 shipped-key recall/rank table. Generated by recall_ranks_v4.py over the schema-1.7 out-v4 scans using the keys implemented in src/reporter/views.rs. The v3_* columns are the prototype measurements from v3/recall_ranks.json (kept for the V7 re-triage, not as the shipped claim).",
        "generated": "2026-10-22",
        "ranks": results,
    }
    OUT.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if not ok:
        return 1
    print(f"V4 RECALL RANKS OK: {len(results)}/9 match the shipped ordering -> {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
