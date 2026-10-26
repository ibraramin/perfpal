#!/usr/bin/env python3
"""D1 gate — verify the Atlas run-story prototype and capture evidence.

Serves `sim/` on 127.0.0.1:8768, drives the prototype with Playwright, asserts
the D1 contract (data fidelity, controls, honesty, determinism, no network),
writes 1440x900 keyframes plus `checks.json`, and exits non-zero on any failure.

Validates core-fed sim data (from `atlas-layout.json` + `metrics.json`); it
makes no source changes and captures evidence only.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import socket
import subprocess
import sys
import time
from datetime import date
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
SIM = ROOT / "sim"
EVIDENCE = Path(os.environ.get("SIM_EVIDENCE", ROOT / "docs" / "evidence" / "atlas-sim"))
PORT = int(os.environ.get("SIM_PORT", "8768"))
BASE = f"http://127.0.0.1:{PORT}/"
PINNED_CHROME = Path("/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome")
REAL_CASES = ["hyperfine", "cjson"]
EDGE_LABELS = {"type-only", "literal-observed", "unresolved"}
PAYLOAD_LABELS = {"type-only", "literal-observed"}
# S3: P2 I/O effects are core-classified; the sim only projects and renders.
IO_OPS = {"read", "write"}
IO_TARGETS = {"file", "env", "net", "stdio"}
# F4: claims the static inference may never make (S2 data + final frame).
FORBIDDEN_CLAIMS = [
    r"speedup",
    r"call stack",
    r"execution order",
    r"runs \d+ times",
    r"% of traffic",
]
TIME_UNITS = r"\b\d+(?:\.\d+)?\s?(?:ms|µs|μs|milliseconds?|seconds?)\b"
# "runtime" is allowed only inside the honesty strip's negation.
NEGATED_RUNTIME = re.compile(r"(?<!not a )runtime", re.I)
# Bare execution/iteration claims are banned unless the same clause negates it.
NEGATABLE_CLAIMS = [r"\bexecution\b", r"\bexecuted\b", r"\biterations\b"]
NEGATION = re.compile(
    r"\b(?:not|no|nothing|never|without|none|isn't|aren't|doesn't|don't|cannot|can't)\b",
    re.I,
)


def unnegated_hits(text: str, patterns: list[str]) -> list[str]:
    """Matches not preceded by a negation in the same clause.

    ``execution_order_tests.rs`` is an identifier, so the word-boundary patterns
    do not match it. "nothing is executed" and "not measured iterations" pass.
    """
    hits = []
    for pattern in patterns:
        for match in re.finditer(pattern, text, re.I):
            window = text[max(0, match.start() - 40):match.start()]
            if NEGATION.search(window):
                continue
            hits.append(match.group(0))
    return hits


def compact_number(value: float) -> str:
    """Mirror sim.js compact() for the key-strip ratio assertion."""
    if value >= 1e6:
        return f"{value / 1e6:.1f}M"
    if value >= 1e4:
        return f"{value / 1e3:.0f}k"
    if value >= 1e3:
        return f"{value / 1e3:.1f}k"
    return f"{value:g}"


def stage_dom(page) -> str:
    """Canonical render DOM: #stage geometry plus the below-fold S4 panels."""
    return page.evaluate(
        """() => {
            const parts = [];
            for (const root of [document.querySelector('#stage'),
                                document.querySelector('#s4-detail')]) {
                if (!root) continue;
                for (const el of [root, ...root.querySelectorAll('*')]) {
                    const attrs = [...el.attributes]
                        .map((attr) => `${attr.name}=${attr.value}`)
                        .sort()
                        .join(';');
                    const text = el.children.length === 0 ? el.textContent : '';
                    parts.push(`${el.tagName}|${attrs}|${text}`);
                }
            }
            return parts.join('\\n');
        }"""
    )


CHECKS: list[dict[str, object]] = []


def check(name: str, ok: object, detail: str = "") -> None:
    passed = bool(ok)
    CHECKS.append({"check": name, "ok": passed, "detail": str(detail)[:500]})
    print(f"{'PASS' if passed else 'FAIL'}  {name}" + (f"  · {detail}" if detail else ""))


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(65536), b""):
            digest.update(block)
    return digest.hexdigest()


def port_free() -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        return sock.connect_ex(("127.0.0.1", PORT)) != 0


def wait_for_server(timeout: float = 10.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            if sock.connect_ex(("127.0.0.1", PORT)) == 0:
                return True
        time.sleep(0.1)
    return False


def main() -> int:
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / "checks.json").unlink(missing_ok=True)

    # ---- static preflight -------------------------------------------------
    data_files = sorted(SIM.glob("data-*.json"))
    before = {path.name: sha256(path) for path in data_files}
    check("data files present", len(data_files) == 6, ", ".join(before))

    index = json.loads((SIM / "data-index.json").read_text())
    cases = {case["id"]: case for case in index["cases"]}
    check("index has the five D1 cases",
          set(cases) == {"hyperfine", "cjson", "no-forward-path", "empty", "capped"},
          ", ".join(cases))

    for name in ("index.html", "sim.css", "sim.js"):
        text = (SIM / name).read_text()
        external = [
            url for url in re.findall(r"https?://[^\s\"'<>)]*", text)
            if url != "http://www.w3.org/2000/svg"
        ]
        check(f"{name}: no external/network reference", not external, str(external[:3]))
        check(f"{name}: no fetch of a remote origin",
              not re.search(r"fetch\(\s*[\"'](?:https?:)?//", text),
              "local data only")

    payloads = {}
    for case_id in REAL_CASES:
        payload = json.loads((SIM / cases[case_id]["file"]).read_text())
        payloads[case_id] = payload
        check(f"{case_id}: non-empty graph",
              payload["nodes"] and payload["edges"] and payload["trace"])
        check(f"{case_id}: on-path edges match trace",
              sum(1 for e in payload["edges"] if e["onPath"]) == len(payload["trace"]) - 1,
              f'{sum(1 for e in payload["edges"] if e["onPath"])} edges')
        check(f"{case_id}: findings accounting present",
              "findingsTotal" in payload["meta"] and "findingsIncluded" in payload["meta"],
              f'{payload["meta"].get("findingsIncluded")}/{payload["meta"].get("findingsTotal")}')
        check(f"{case_id}: no absolute paths in data",
              "/home/" not in (SIM / cases[case_id]["file"]).read_text())

    # ---- S4: whole-repo projection (data-level, real cases only) -------
    for case_id, payload in payloads.items():
        case_dir = SIM / "source" / f"out-{case_id}"
        if not (case_dir / "atlas-layout.json").is_file() or not (case_dir / "metrics.json").is_file():
            check(f"{case_id}: S4 core artifacts present", False,
                  f"missing {case_dir}; run bash sim/make-source.sh first")
            continue
        layout = json.loads((case_dir / "atlas-layout.json").read_text())
        metrics = json.loads((case_dir / "metrics.json").read_text())
        functions = {function["id"]: function for function in metrics["stats"]["functions"]}
        metrics_files = {file["path"]: file for file in metrics["files"]}
        trust = payload.get("trust") or {}

        # purpose: projected envelope, honesty filter, core passthrough
        core_purpose = layout.get("purpose") or {}
        purpose = payload.get("purpose") or {}
        purpose_text: str = "" if purpose.get("text") is None else str(purpose.get("text"))
        purpose_hits = [pattern for pattern in FORBIDDEN_CLAIMS
                        if re.search(pattern, purpose_text, re.I)]
        if re.search(TIME_UNITS, purpose_text, re.I):
            purpose_hits.append("time-unit")
        if NEGATED_RUNTIME.search(purpose_text):
            purpose_hits.append("runtime")
        purpose_hits.extend(unnegated_hits(purpose_text, NEGATABLE_CLAIMS))
        check(f"{case_id}: S4 purpose envelope is well formed",
              set(purpose) == {"kind", "source", "text", "version"}
              and purpose.get("kind") in {"readme", "structural", "withheld"}
              and (purpose.get("source") is None or isinstance(purpose.get("source"), str))
              and bool(purpose_text.strip())
              and len(purpose_text) <= 201
              and purpose.get("version") == 1,
              str(purpose))
        check(f"{case_id}: S4 purpose makes no forbidden claim", not purpose_hits,
              ", ".join(purpose_hits))
        core_text: str = "" if core_purpose.get("text") is None else str(core_purpose.get("text"))
        core_forbidden = any(re.search(pattern, core_text, re.I) for pattern in FORBIDDEN_CLAIMS) \
            or bool(re.search(TIME_UNITS, core_text, re.I)) \
            or bool(NEGATED_RUNTIME.search(core_text)) \
            or bool(unnegated_hits(core_text, NEGATABLE_CLAIMS))
        if core_forbidden:
            withheld = {"kind": "withheld", "source": core_purpose.get("source"),
                        "text": "No static purpose sentence.", "version": 1}
            check(f"{case_id}: S4 forbidden core purpose is withheld", purpose == withheld,
                  str(purpose))
        else:
            check(f"{case_id}: S4 purpose is the core sentence",
                  purpose.get("kind") == core_purpose.get("kind")
                  and purpose.get("source") == core_purpose.get("source")
                  and purpose.get("text") == core_text
                  and purpose.get("version") == core_purpose.get("purpose_version"),
                  str(purpose))
        check(f"{case_id}: S4 purpose trust matches",
              trust.get("purpose") == {"kind": purpose.get("kind"),
                                       "source": purpose.get("source")},
              str(trust.get("purpose")))

        # profile: whole-repo queue projected in authoritative order
        profile = payload.get("profile") or []
        profile_meta = payload.get("profileMeta") or {}
        core_profile = layout.get("profile") or {}
        core_rows = core_profile.get("rows") or []
        trust_profile = trust.get("profile") or {}
        ranks = [row.get("rank") for row in profile]
        shown = profile_meta.get("shown")
        candidate = profile_meta.get("candidateTotal")
        production_ranked = profile_meta.get("productionRanked")
        total_ranked = profile_meta.get("totalRanked")
        check(f"{case_id}: S4 profile rows are production",
              bool(profile) and all(row.get("production") is True for row in profile),
              f'{sum(1 for row in profile if row.get("production"))}/{len(profile)}')
        check(f"{case_id}: S4 profile ranks ascend in core order",
              bool(ranks) and ranks == sorted(ranks) and len(set(ranks)) == len(ranks),
              str(ranks))
        check(f"{case_id}: S4 profile top row is the core top row",
              bool(profile) and bool(core_rows) and profile[0].get("id") == core_rows[0].get("id"),
              f'{profile[0].get("id") if profile else None} vs {core_rows[0].get("id") if core_rows else None}')
        check(f"{case_id}: S4 profile costs match metrics",
              all(row.get("id") in functions
                  and (functions[row["id"]].get("function_cost_v2") is None
                       or row.get("cost") == functions[row["id"]]["function_cost_v2"])
                  and isinstance(row.get("cost"), (int, float)) and row["cost"] >= 0
                  for row in profile),
              str([row.get("id") for row in profile if row.get("id") not in functions][:3]))
        check(f"{case_id}: S4 profile counts reconcile with profileMeta/trust",
              isinstance(shown, int) and isinstance(candidate, int)
              and isinstance(production_ranked, int) and isinstance(total_ranked, int)
              and shown == len(profile) and 0 <= shown <= 12
              and candidate >= shown and production_ranked >= candidate
              and total_ranked >= production_ranked
              and isinstance(profile_meta.get("demotedProduction"), int)
              and profile_meta["demotedProduction"] >= 0
              and trust_profile == {key: profile_meta.get(key) for key in
                                    ("shown", "candidateTotal", "productionRanked",
                                     "totalRanked", "demotedProduction")},
              f"{profile_meta} · {trust_profile}")
        check(f"{case_id}: S4 profile meta matches the core block",
              shown == core_profile.get("shown")
              and candidate == core_profile.get("candidate_total")
              and production_ranked == core_profile.get("production_ranked")
              and total_ranked == core_profile.get("total_ranked")
              and profile_meta.get("queueVersion") == core_profile.get("queue_version"),
              str(core_profile))

        # scenes: counts, caps, containment, metrics membership, ordering
        scenes = payload.get("scenes") or []
        scene_meta = payload.get("sceneMeta") or {}
        core_scenes = layout.get("scenes") or {}
        core_scene_rows = core_scenes.get("rows") or []
        trust_scenes = trust.get("scenes") or {}
        omitted = scene_meta.get("omitted") or {}
        scene_shown = scene_meta.get("shownScenes")
        scene_total = scene_meta.get("totalScenes")
        scene_files = scene_meta.get("totalFiles")
        scene_shown_files = scene_meta.get("shownFiles")
        metrics_production_files = sum(
            1 for file in metrics["files"]
            if file["function_count"] > 0 and file.get("path_class") == "production")
        metrics_non_production_files = sum(
            1 for file in metrics["files"]
            if file["function_count"] > 0 and file.get("path_class") != "production")
        check(f"{case_id}: S4 scene counts and caps reconcile",
              isinstance(scene_shown, int) and isinstance(scene_total, int)
              and isinstance(scene_files, int) and isinstance(scene_shown_files, int)
              and scene_shown == len(scenes) and 0 <= scene_shown <= 24
              and isinstance(scene_meta.get("fileCap"), int) and 0 <= scene_meta["fileCap"] <= 5
              and scene_shown_files == sum(row["files"] for row in scenes)
              and sum(row["files"] for row in scenes) + omitted.get("files", -1) == scene_files
              and scene_files == sum(1 for file in metrics["files"] if file["function_count"] > 0)
              and omitted.get("scenes") == scene_total - scene_shown,
              str(scene_meta))
        check(f"{case_id}: S4 scene production split reconciles with metrics",
              scene_meta.get("productionFiles") == metrics_production_files
              and scene_meta.get("nonProductionFiles") == metrics_non_production_files
              and scene_meta["productionFiles"] + scene_meta["nonProductionFiles"] == scene_files
              and trust_scenes.get("productionFiles") == metrics_production_files
              and trust_scenes.get("nonProductionFiles") == metrics_non_production_files,
              f"production={scene_meta.get('productionFiles')}/{metrics_production_files} "
              f"non-production={scene_meta.get('nonProductionFiles')}/{metrics_non_production_files} "
              f"trust={trust_scenes.get('productionFiles')}/{trust_scenes.get('nonProductionFiles')}")
        check(f"{case_id}: S4 scene top files are capped and sorted",
              all(len(row.get("topFiles") or []) <= 5 for row in scenes)
              and all(row["topFiles"] == sorted(
                  row["topFiles"], key=lambda file: (-file["cost"], file["path"]))
                  for row in scenes),
              str([len(row.get("topFiles") or []) for row in scenes]))
        check(f"{case_id}: S4 scene scales and flags are valid",
              all(row.get("scale") in {"v2", "v1", "mixed"}
                  and row.get("depth") in (0, 1, 2)
                  and isinstance(row.get("entry"), bool)
                  and isinstance(row.get("hottest"), bool)
                  and all(file.get("scale") in {"v2", "v1"}
                          and isinstance(file.get("production"), bool)
                          and isinstance(file.get("entry"), bool)
                          and isinstance(file.get("hottest"), bool)
                          for file in row.get("topFiles") or [])
                  for row in scenes),
              str([(row.get("path"), row.get("scale"), row.get("depth")) for row in scenes][:3]))
        check(f"{case_id}: S4 scene paths contain their top files",
              all(
                  (row["path"] == "(root)"
                   and all("/" not in file["path"] for file in row["topFiles"]))
                  or (row["path"] != "(root)"
                      and all(file["path"].startswith(row["path"] + "/")
                              for file in row["topFiles"]))
                  for row in scenes
              ),
              str([row["path"] for row in scenes
                   if not (row["path"] == "(root)" and all(
                       "/" not in file["path"] for file in row["topFiles"]))
                   and not (row["path"] != "(root)" and all(
                       file["path"].startswith(row["path"] + "/") for file in row["topFiles"]))][:3]))
        check(f"{case_id}: S4 scene top files exist in metrics",
              all(file["path"] in metrics_files
                  for row in scenes for file in row["topFiles"]),
              str([file["path"] for row in scenes for file in row["topFiles"]
                   if file["path"] not in metrics_files][:3]))
        check(f"{case_id}: S4 scene totals are non-negative",
              all(row["cost"] >= 0 and row["productionCost"] >= 0
                  and row["findings"] >= 0 and row["files"] >= 1 for row in scenes)
              and all(isinstance(omitted.get(key), int) and omitted[key] >= 0
                      for key in ("scenes", "files", "cost", "productionCost", "findings")),
              str(omitted))
        check(f"{case_id}: S4 scene trust reconciles",
              trust_scenes == {"total": scene_total, "shown": scene_shown,
                               "files": scene_files, "shownFiles": scene_shown_files,
                               "productionFiles": scene_meta.get("productionFiles"),
                               "nonProductionFiles": scene_meta.get("nonProductionFiles"),
                               "omitted": omitted.get("scenes")},
              f"{trust_scenes} vs {scene_meta}")
        check(f"{case_id}: S4 scenes keep core order",
              [row["path"] for row in scenes] == [row["path"] for row in core_scene_rows],
              str([row["path"] for row in scenes][:3]))
        core_scene_of_node = {node["path"]: node.get("scene") for node in layout["nodes"]}
        core_scene_of_action = {
            action["location"]: action.get("scene") for action in layout["actions"]}
        check(f"{case_id}: S4 node scenes are core strings",
              all(isinstance(node.get("scene"), str)
                  and node["scene"] == core_scene_of_node.get(node["id"])
                  for node in payload["nodes"]),
              str([node["id"] for node in payload["nodes"]
                   if not isinstance(node.get("scene"), str)
                   or node["scene"] != core_scene_of_node.get(node["id"])][:3]))
        check(f"{case_id}: S4 action scenes are core strings",
              all(isinstance(action.get("scene"), str)
                  and action["scene"] == core_scene_of_action.get(action["anchor"])
                  for action in payload["meta"]["actions"]),
              str([action["anchor"] for action in payload["meta"]["actions"]
                   if not isinstance(action.get("scene"), str)
                   or action["scene"] != core_scene_of_action.get(action["anchor"])][:3]))
        check(f"{case_id}: S4 action functions are carried",
              all(isinstance(action.get("function"), str) and action["function"]
                  for action in payload["meta"]["actions"]),
              str([action.get("anchor") for action in payload["meta"]["actions"]
                   if not isinstance(action.get("function"), str) or not action["function"]][:3]))

        # unresolved calls: grouped by drawn call-site file, sorted, capped
        unresolved_total = 0
        unresolved_files = 0
        unresolved_shape = True
        unresolved_sorted = True
        for node in payload["nodes"]:
            block = node.get("unresolvedCalls")
            if not block:
                continue
            unresolved_files += 1
            count = block.get("count")
            rows = block.get("rows") or []
            unresolved_total += count if isinstance(count, int) else 0
            if not isinstance(count, int) or count < 1 or len(rows) != min(count, 6):
                unresolved_shape = False
            if rows != sorted(rows, key=lambda row: (row["line"], row["callee"], row["caller"])):
                unresolved_sorted = False
        core_calls = (layout.get("flow") or {}).get("calls") or []
        core_unresolved = [call for call in core_calls if call.get("unresolved")]
        drawn = {node["path"] for node in layout["nodes"]}
        check(f"{case_id}: S4 unresolved calls are grouped on drawn files",
              unresolved_shape
              and all(call.get("from") in drawn for call in core_unresolved)
              and unresolved_total == len(core_unresolved)
              and trust.get("unresolvedCalls") == unresolved_total
              and trust.get("unresolvedFiles") == unresolved_files,
              f"{unresolved_total} rows on {unresolved_files} files")
        check(f"{case_id}: S4 unresolved-call rows are sorted", unresolved_sorted)

    # ---- F2: every drawn edge carries a core-classified label ----------
    for case_id, payload in payloads.items():
        edges = payload["edges"]
        check(f"{case_id}: F2 every edge has an allowed label",
              all(edge.get("label") in EDGE_LABELS for edge in edges),
              str(sorted({edge.get("label") for edge in edges if edge.get("label") not in EDGE_LABELS})))
        check(f"{case_id}: F2 typed/literal labels carry a payload",
              all(edge.get("payload") for edge in edges if edge.get("label") in PAYLOAD_LABELS))
        unresolved_ok = all(
            edge.get("payload") is None
            for edge in edges
            if edge.get("label") == "unresolved"
        )
        check(f"{case_id}: F2 unresolved labels are never guessed", unresolved_ok)
        check(f"{case_id}: F2 label class matches the label",
              all(edge.get("labelClass") == ("unresolved" if edge["label"] == "unresolved" else "resolved")
                  for edge in edges))
        top_rows = [call for edge in edges for call in edge.get("topCalls", [])]
        check(f"{case_id}: F2 topCalls rows carry callee and line",
              all(isinstance(call.get("callee"), str) and call.get("callee")
                  and isinstance(call.get("line"), int)
                  for call in top_rows),
              f"{len(top_rows)} rows")
        resolved_edges = [edge for edge in edges if edge.get("label") in PAYLOAD_LABELS]
        with_calls = [edge for edge in resolved_edges if edge.get("topCalls")]
        if with_calls:
            check(f"{case_id}: F2 resolved-label edges carry top calls", True,
                  f"{len(with_calls)}/{len(resolved_edges)} resolved-label edges")
        else:
            check(f"{case_id}: F2 topCalls projection shape only", True,
                  f"no resolved-label edge had a top call in this data ({len(resolved_edges)} edges)")

    # ---- S3: P2 I/O effects are projected honestly ---------------------
    for case_id, payload in payloads.items():
        nodes = payload["nodes"]
        io_nodes = [node for node in nodes if node.get("io")]
        rows = [row for node in io_nodes for row in node["io"]["rows"]]
        check(f"{case_id}: S3 io rows are projected", bool(rows),
              f"{len(rows)} rows on {len(io_nodes)} drawn files")
        check(f"{case_id}: S3 io enums are respected",
              all(row["op"] in IO_OPS and row["target"] in IO_TARGETS for row in rows),
              str([(row["op"], row["target"]) for row in rows
                   if row["op"] not in IO_OPS or row["target"] not in IO_TARGETS]))
        check(f"{case_id}: S3 stdio rows never carry a path literal",
              all(row["literal"] is None for row in rows if row["target"] == "stdio"))
        check(f"{case_id}: S3 literals are strings or null",
              all(row["literal"] is None or isinstance(row["literal"], str) for row in rows))

        def counts_ok(node_io: dict) -> bool:
            total = node_io["total"]
            if total != node_io["read"] + node_io["write"]:
                return False
            if total != node_io["file"] + node_io["env"] + node_io["net"] + node_io["stdio"]:
                return False
            return len(node_io["rows"]) == min(total, 6)

        check(f"{case_id}: S3 per-file io counts reconcile",
              all(counts_ok(node["io"]) for node in io_nodes))
        trust_io = (payload.get("trust") or {}).get("io") or {}
        total = sum(node["io"]["total"] for node in io_nodes)
        check(f"{case_id}: S3 trust io reconciles with the nodes",
              trust_io.get("rows") == total
              and trust_io.get("files") == len(io_nodes)
              and trust_io.get("read") == sum(node["io"]["read"] for node in io_nodes)
              and trust_io.get("write") == sum(node["io"]["write"] for node in io_nodes)
              and trust_io.get("stdio") == sum(node["io"]["stdio"] for node in io_nodes)
              and trust_io.get("literal") == sum(node["io"]["literal"] for node in io_nodes)
              and trust_io.get("unknownTarget") == sum(
                  node["io"]["unknownTarget"] for node in io_nodes),
              f'{trust_io} vs {total} rows on {len(io_nodes)} files')

        def io_key(row: dict) -> tuple:
            return (row["line"], row["callee"], row["op"], row["target"], row["literal"] or "")

        check(f"{case_id}: S3 node io rows are sorted",
              all([io_key(row) for row in node["io"]["rows"]]
                  == sorted(io_key(row) for row in node["io"]["rows"])
                  for node in io_nodes))

    # ---- hottest function and profile rank ----------------------------
    def function_rows(payload: dict) -> list[dict]:
        rows = []
        for node in payload["nodes"]:
            for function in node["functions"]:
                rows.append({**function, "path": node["id"],
                             "production": not node["nonproduction"]})
        return sorted(rows, key=lambda row: (-row["cost"], row["line"], row["name"]))

    for case_id, payload in payloads.items():
        rows = function_rows(payload)
        hottest = payload.get("hottestFunction") or {}
        check(f"{case_id}: hottestFunction is the global cost leader",
              bool(rows) and hottest.get("id") == rows[0]["id"]
              and hottest.get("cost") == rows[0]["cost"],
              f'{hottest.get("id")} @ {hottest.get("cost")} vs {rows[0]["id"]} @ {rows[0]["cost"]}')
        check(f"{case_id}: hottestFunction carries identity",
              all(hottest.get(key) not in (None, "") for key in ("id", "name", "path", "line", "cost")),
              str({key: hottest.get(key) for key in ("id", "name", "path", "line", "cost")}))

    # ---- F4: no forbidden claim in the story data ----------------------
    story_text = "\n".join(
        (SIM / entry["file"]).read_text() for entry in index["cases"]
    )
    hits = [pattern for pattern in FORBIDDEN_CLAIMS if re.search(pattern, story_text, re.I)]
    check("F4 no forbidden claim in sim/data-*.json", not hits, ", ".join(hits))
    check("F4 no time units in sim/data-*.json", not re.search(TIME_UNITS, story_text, re.I))
    check("F4 runtime only appears negated in sim/data-*.json",
          not NEGATED_RUNTIME.search(story_text))
    bare_claims = unnegated_hits(story_text, NEGATABLE_CLAIMS)
    check("F4 no bare execution/iteration claim in sim/data-*.json",
          not bare_claims, ", ".join(sorted(set(bare_claims))[:5]))

    # ---- the core artifact must declare syntactic fidelity -------------
    for case, directory in (("hyperfine", "out-hyperfine"), ("cjson", "out-cjson")):
        artifact = SIM / "source" / directory / "atlas-layout.json"
        if not artifact.is_file():
            check(f"{case}: core flow fidelity is syntactic", False,
                  f"missing {artifact}; run bash sim/make-source.sh first")
            continue
        layout = json.loads(artifact.read_text())
        flow = layout.get("flow") or {}
        check(f"{case}: core flow fidelity is syntactic",
              flow.get("fidelity") == "syntactic",
              f'{layout.get("layout_version")} · {flow.get("fidelity")!r}')
        io = flow.get("io")
        check(f"{case}: core io is a flat array", isinstance(io, list), type(io).__name__)
        core_io = io if isinstance(io, list) else []
        drawn = {node["path"] for node in layout["nodes"]}
        check(f"{case}: core io rows stay on drawn call-site files",
              all(row.get("file") in drawn for row in core_io),
              str([row.get("file") for row in core_io if row.get("file") not in drawn][:3]))
        check(f"{case}: core io rows respect the enums",
              all(row.get("op") in IO_OPS and row.get("target") in IO_TARGETS for row in core_io))
        check(f"{case}: core io stdio rows carry no literal",
              all(row.get("literal") is None for row in core_io if row.get("target") == "stdio"))
        core_key = lambda row: (row["file"], row["line"], row["callee"],
                                row["op"], row["target"], row["literal"] or "")
        check(f"{case}: core io rows are sorted",
              [core_key(row) for row in core_io] == sorted(core_key(row) for row in core_io))

    if not port_free():
        check(f"port {PORT} is free", False, "another process is listening; stop it and rerun")
        return finish()

    server = subprocess.Popen(
        [sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1",
         "--directory", str(SIM)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    try:
        check(f"server answers on :{PORT}", wait_for_server())
        if not CHECKS[-1]["ok"]:
            return finish()

        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(
                executable_path=str(PINNED_CHROME) if PINNED_CHROME.exists() else None,
                args=["--no-sandbox"],
            )
            context = browser.new_context(viewport={"width": 1440, "height": 900},
                                          device_scale_factor=1)
            page = context.new_page()
            console_errors: list[str] = []
            page_errors: list[str] = []
            requests: list[str] = []
            page.on("console", lambda msg: console_errors.append(f"{msg.type}: {msg.text}")
                    if msg.type == "error" else None)
            page.on("pageerror", lambda err: page_errors.append(str(err)))
            page.on("request", lambda req: requests.append(req.url))

            response = page.goto(BASE, wait_until="networkidle")
            check("index.html returns 200", response is not None and response.status == 200,
                  str(response.status if response else None))
            page.wait_for_function("() => window.__sim && window.__sim.state().beats > 0",
                                   timeout=10000)
            check("window.__sim.ready is true after the first render",
                  page.evaluate("() => Boolean(window.__sim && window.__sim.ready)"))

            for case_id in REAL_CASES:
                expected = payloads[case_id]
                page.select_option("#case-select", value=case_id)
                page.wait_for_function(
                    "id => window.__sim.state().caseId === id", arg=case_id, timeout=10000)
                state = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: beats derived", state["beats"] >= 6, f'{state["beats"]} beats')
                check(f"{case_id}: case is not synthetic", state["synthetic"] is False)

                purpose = expected.get("purpose") or {}
                if purpose.get("kind") == "withheld":
                    purpose_line = (
                        "PURPOSE · Purpose withheld (README claim)."
                        if purpose.get("source")
                        else "PURPOSE · Purpose withheld (claim filter)."
                    )
                elif purpose.get("kind") == "readme":
                    purpose_line = f'PURPOSE · {purpose["text"]} · from {purpose.get("source") or "README"}'
                elif purpose.get("kind") == "structural":
                    purpose_line = f'PURPOSE · {purpose["text"]} · structural fallback'
                else:
                    purpose_line = "entry-flow map · static, profile to confirm"
                subline = page.inner_text("#brand-subline").strip()
                check(f"{case_id}: masthead purpose matches the projection",
                      subline == purpose_line, subline[:140])

                scene_meta = expected["sceneMeta"]
                profile_meta = expected["profileMeta"]
                trust_data = expected["trust"]
                detail = page.evaluate(
                    """() => {
                        const section = document.querySelector('#s4-detail');
                        const cards = [...document.querySelectorAll('#s4-scene-grid [data-scene]')];
                        const queue = [...document.querySelectorAll('#s4-queue [data-profile-row]')];
                        return {
                            visible: !!section && !section.hidden
                                && getComputedStyle(section).display !== 'none',
                            cards: cards.length,
                            cardPaths: cards.map((el) => el.getAttribute('data-scene')),
                            scenesCaps: (document.querySelector('#s4-scenes-caps') || {}).textContent || '',
                            profileCaps: (document.querySelector('#s4-profile-caps') || {}).textContent || '',
                            queue: queue.length,
                            queueFirst: queue.length ? queue[0].textContent : '',
                            trust: (document.querySelector('#s4-trust-list') || {}).textContent || '',
                            topFilesMax: Math.max(0, ...[...document.querySelectorAll(
                                '#s4-scene-grid .s4-files')].map((el) => el.children.length)),
                        };
                    }"""
                )
                check(f"{case_id}: S4 detail visible for the real case", detail["visible"])
                check(f"{case_id}: S4 scene cards match shownScenes",
                      detail["cards"] == scene_meta["shownScenes"]
                      and detail["cardPaths"] == [scene["path"] for scene in expected["scenes"]],
                      f'{detail["cards"]}/{scene_meta["shownScenes"]} · {detail["cardPaths"][:3]}')
                check(f"{case_id}: S4 scene caps disclose shown/total",
                      f'{scene_meta["shownScenes"]} of {scene_meta["totalScenes"]}' in detail["scenesCaps"],
                      detail["scenesCaps"][:160])
                check(f"{case_id}: S4 scene caps disclose test/bench files",
                      f'{scene_meta["nonProductionFiles"]} test/bench files' in detail["scenesCaps"],
                      detail["scenesCaps"][:200])
                check(f"{case_id}: S4 scene top files stay capped",
                      detail["topFilesMax"] <= 5, str(detail["topFilesMax"]))
                check(f"{case_id}: S4 queue shows every carried row",
                      detail["queue"] == profile_meta["shown"]
                      and expected["profile"][0]["name"] in detail["queueFirst"],
                      f'{detail["queue"]}/{profile_meta["shown"]} · {detail["queueFirst"][:90]}')
                check(f"{case_id}: S4 profile caps disclose the candidate set",
                      f'{profile_meta["shown"]} of {profile_meta["candidateTotal"]}' in detail["profileCaps"]
                      and f'{profile_meta["demotedProduction"]:,} production rows demoted' in detail["profileCaps"],
                      detail["profileCaps"][:160])
                trust_panel = detail["trust"]
                check(f"{case_id}: S4 trust panel carries scenes, profile and unresolved totals",
                      f'{scene_meta["shownScenes"]} of {scene_meta["totalScenes"]} shown' in trust_panel
                      and f'{profile_meta["shown"]} shown of {profile_meta["candidateTotal"]} candidates' in trust_panel
                      and f'{trust_data["unresolvedCalls"]:,} on {trust_data["unresolvedFiles"]:,} files' in trust_panel
                      and f'{purpose.get("kind", "not present")} · {purpose.get("source") or "no source"}' in trust_panel,
                      trust_panel[:200])
                check(f"{case_id}: S4 trust panel carries the scan-wide test/bench split",
                      f'{scene_meta["nonProductionFiles"]} of {scene_meta["totalFiles"]} source files (test/bench)'
                      in trust_panel,
                      trust_panel[:260])

                rendered = page.evaluate(
                    "() => document.querySelectorAll('#stage g[role=\"button\"]').length")
                check(f"{case_id}: every node rendered", rendered == len(expected["nodes"]),
                      f"{rendered}/{len(expected['nodes'])}")

                # ---- F2: DOM label coverage -----------------------------
                dom_counts = page.evaluate(
                    """() => {
                        const labels = [...document.querySelectorAll('#stage text.edge-label')];
                        const unresolved = document.querySelectorAll('#stage path.edge.edge-unresolved').length;
                        return {
                            paths: document.querySelectorAll('#stage path.edge').length,
                            labels: labels.length,
                            unresolved,
                            unresolvedLabels: labels.filter(
                                el => el.getAttribute('data-label') === 'unresolved').length,
                            badLabel: labels.filter(
                                el => !['type-only', 'literal-observed', 'unresolved']
                                    .includes(el.getAttribute('data-label'))).length,
                            badEdge: labels.filter(el => !el.getAttribute('data-edge')).length,
                        };
                    }"""
                )
                edge_total = len(expected["edges"])
                unresolved_total = sum(1 for edge in expected["edges"] if edge["label"] == "unresolved")
                check(f"{case_id}: F2 DOM labels every drawn edge",
                      dom_counts["paths"] == edge_total and dom_counts["labels"] == edge_total,
                      str(dom_counts))
                check(f"{case_id}: F2 DOM labels use the allowed set",
                      dom_counts["badLabel"] == 0 and dom_counts["badEdge"] == 0,
                      str(dom_counts))
                check(f"{case_id}: F2 unresolved edges stay visibly distinct",
                      dom_counts["unresolved"] == unresolved_total
                      and dom_counts["unresolvedLabels"] == unresolved_total,
                      str(dom_counts))

                footer = page.inner_text("#canvas-footer")
                check(f"{case_id}: footer matches the render", footer.strip() == expected["meta"]["footer"].strip())
                path_text = page.inner_text("#key-path").strip()
                path_title = page.get_attribute("#key-path", "title") or ""
                check(f"{case_id}: key strip strips the duplicate PATH label",
                      not re.match(r"^PATH\b", path_text, re.I)
                      and path_title == expected["meta"]["path"],
                      f"{path_text[:80]!r} · title={path_title[:80]!r}")

                honesty = page.inner_text("#honesty").lower()
                check(f"{case_id}: honesty label present",
                      "static, inferred" in honesty and "not a runtime trace" in honesty
                      and "profile to confirm" in honesty)

                hottest_node = next(
                    (node for node in expected["nodes"] if node["hottest"]), None)
                if hottest_node:
                    ratio_match = re.match(
                        r"^([\d,]+(?:\.\d+)?)", hottest_node.get("ratioText") or "")
                    if ratio_match:
                        expected_ratio = (
                            f'{compact_number(float(ratio_match.group(1).replace(",", "")))}× median')
                        key_meta = page.inner_text("#key-hottest-meta")
                        check(f"{case_id}: hottest ratio keeps its precision",
                              expected_ratio in key_meta, f"{expected_ratio} · {key_meta}")

                text = page.inner_text("body")
                time_claim = re.search(r"\b\d+(?:\.\d+)?\s?(?:ms|µs|μs|milliseconds?|seconds?)\b", text, re.I)
                check(f"{case_id}: no time units rendered", time_claim is None,
                      time_claim.group(0) if time_claim else "none")

                kinds = page.evaluate("() => window.__sim.state().kinds")
                check(f"{case_id}: beats include loop and summary",
                      "loop" in kinds and "summary" in kinds, ",".join(sorted(set(kinds))))
                payoff = {"hottest", "profile", "start-here", "trust", "summary"}
                check(f"{case_id}: payoff beats present in order",
                      payoff <= set(kinds)
                      and kinds.index("hottest") < kinds.index("profile")
                      < kinds.index("start-here") < kinds.index("trust")
                      < kinds.index("summary"),
                      ",".join(kinds))
                for kind in ("profile", "start-here", "trust"):
                    page.evaluate("(i) => window.__sim.seekIndex(i)", kinds.index(kind))
                    page.wait_for_timeout(50)
                    body = page.inner_text("#narration-body").strip()
                    check(f"{case_id}: {kind} narration is non-empty", len(body) > 12, body[:90])

                enter_index = kinds.index("enter")
                page.evaluate("(i) => window.__sim.seekIndex(i)", enter_index)
                page.wait_for_timeout(50)
                enter_text = page.inner_text("#narration-body")
                check(f"{case_id}: enter beat carries scene context",
                      "Scene " in enter_text,
                      enter_text[-140:])

                # ---- S3: stores on the scene + trust line ---------------
                io_node = next((node for node in expected["nodes"] if node.get("io")), None)
                literal_node = next(
                    (node for node in expected["nodes"]
                     if node.get("io") and any(row["literal"] for row in node["io"]["rows"])),
                    None,
                )
                unknown_node = next(
                    (node for node in expected["nodes"]
                     if node.get("io") and node["io"]["unknownTarget"] > 0),
                    None,
                )
                check(f"{case_id}: S3 the data slice carries io rows", io_node is not None)
                beat_count = page.evaluate("() => window.__sim.state().beats")
                stores = []
                for index in range(beat_count):
                    page.evaluate("(i) => window.__sim.seekIndex(i)", index)
                    text = page.inner_text("#narration-body")
                    if "Stores:" in text:
                        stores.append((index, text))
                check(f"{case_id}: S3 scene beats render stores", bool(stores),
                      f"{len(stores)} stores beats")
                drawn_by_id = {node["id"]: node for node in expected["nodes"]}
                trace_io = sum(
                    1 for step in expected["trace"]
                    if (drawn_by_id.get(step["path"]) or {}).get("io")
                )
                check(f"{case_id}: S3 every trace node with io has a stores beat",
                      len(stores) == trace_io, f"{len(stores)} stores / {trace_io} trace nodes with io")
                literal = None
                if literal_node:
                    literal = next(
                        row["literal"] for row in literal_node["io"]["rows"] if row["literal"])
                page.evaluate("(i) => window.__sim.seekIndex(i)", kinds.index("trust"))
                page.wait_for_timeout(50)
                trust_text = page.inner_text("#narration-body")
                check(f"{case_id}: S3 trust line carries I/O coverage",
                      "I/O:" in trust_text and "call-name match" in trust_text,
                      trust_text[:160])
                if literal:
                    check(f"{case_id}: S3 a written literal target is rendered",
                          literal in trust_text or any(literal in text for _, text in stores),
                          literal)
                if unknown_node:
                    check(f"{case_id}: S3 a non-literal target is marked unknown",
                          "unknown" in trust_text or any("unknown" in text for _, text in stores),
                          unknown_node["id"])
                trust_io_expected = (expected.get("trust") or {}).get("io") or {}
                if trust_io_expected.get("stdio"):
                    check(f"{case_id}: S3 trust line discloses stdio effects",
                          "stdio" in trust_text, trust_text[-120:])
                if trust_io_expected.get("literal", 0) > 2:
                    check(f"{case_id}: S3 trust line marks omitted literal examples",
                          "+" in trust_text, trust_text[-120:])
                trust_clipped = page.evaluate(
                    """() => {
                        const el = document.querySelector('#narration-body');
                        return el ? el.scrollHeight > el.clientHeight + 1 : false;
                    }"""
                )
                check(f"{case_id}: S3 trust line is not clipped", not trust_clipped)
                page.evaluate("() => document.activeElement && document.activeElement.blur()")
                page.wait_for_timeout(150)
                shot = EVIDENCE / f"{case_id}-trust.png"
                page.screenshot(path=str(shot), full_page=False)
                check(f"{case_id}: S3 trust keyframe",
                      shot.exists() and shot.stat().st_size > 20000,
                      f"{shot.stat().st_size if shot.exists() else 0} bytes")
                if stores:
                    shot_index = stores[0][0]
                    if literal:
                        shot_index = next(
                            (index for index, text in stores if literal in text), stores[0][0])
                    page.evaluate("(i) => window.__sim.seekIndex(i)", shot_index)
                    page.wait_for_timeout(50)
                    stores_text = page.inner_text("#narration-body")
                    check(f"{case_id}: S3 stores make no forbidden claim",
                          not any(re.search(pattern, stores_text, re.I)
                                  for pattern in FORBIDDEN_CLAIMS),
                          stores_text[-140:])
                    clipped = page.evaluate(
                        """() => {
                            const el = document.querySelector('#narration-body');
                            return el ? el.scrollHeight > el.clientHeight + 1 : false;
                        }"""
                    )
                    check(f"{case_id}: S3 stores line is not clipped", not clipped)
                    page.evaluate("() => document.activeElement && document.activeElement.blur()")
                    page.wait_for_timeout(150)
                    shot = EVIDENCE / f"{case_id}-stores.png"
                    page.screenshot(path=str(shot), full_page=False)
                    check(f"{case_id}: S3 stores keyframe",
                          shot.exists() and shot.stat().st_size > 20000,
                          f"{shot.stat().st_size if shot.exists() else 0} bytes")
                inspector_node = literal_node or unknown_node or io_node
                if inspector_node:
                    page.evaluate(
                        """(path) => {
                            const node = document.querySelector(`#stage g[role="button"][data-node="${path}"]`);
                            if (node) node.dispatchEvent(new MouseEvent('click', {bubbles: true}));
                        }""",
                        inspector_node["id"],
                    )
                    page.wait_for_timeout(50)
                    insp = page.inner_text("#inspector")
                    check(f"{case_id}: S3 inspector renders data stores",
                          "DATA STORES" in insp
                          and f'L{inspector_node["io"]["rows"][0]["line"]}' in insp,
                          insp[:140])
                    io_rows_dom = page.evaluate(
                        """() => [...document.querySelectorAll('#inspector .insp-io li')]
                            .map((li) => ({
                                target: li.dataset.ioTarget,
                                text: li.textContent,
                                unknown: li.dataset.ioUnknown === 'true',
                            }))"""
                    )
                    if any(row["literal"] is None and row["target"] != "stdio"
                           for row in inspector_node["io"]["rows"]):
                        check(f"{case_id}: S3 inspector marks a non-literal target row",
                              all("target unknown" in row["text"] for row in io_rows_dom
                                  if row["unknown"] and row["target"] != "stdio"),
                              str([row for row in io_rows_dom if row["unknown"]][:2]))
                    if any(row["target"] == "stdio" for row in inspector_node["io"]["rows"]):
                        check(f"{case_id}: S3 inspector marks a stdio row pathless",
                              all("stream · no path" in row["text"] for row in io_rows_dom
                                  if row["target"] == "stdio"),
                              str([row for row in io_rows_dom if row["target"] == "stdio"][:2]))
                    shown_rows = len(inspector_node["io"]["rows"])
                    total_rows = inspector_node["io"]["total"]
                    if total_rows > shown_rows:
                        check(f"{case_id}: S3 inspector discloses the row cap",
                              f"showing {shown_rows} of {total_rows}" in insp,
                              f"{shown_rows}/{total_rows}")
                    page.evaluate("() => document.activeElement && document.activeElement.blur()")
                    page.wait_for_timeout(150)
                    shot = EVIDENCE / f"{case_id}-io-inspector.png"
                    page.screenshot(path=str(shot), full_page=False)
                    check(f"{case_id}: S3 inspector keyframe",
                          shot.exists() and shot.stat().st_size > 20000,
                          f"{shot.stat().st_size if shot.exists() else 0} bytes")
                    page.keyboard.press("Escape")

                page.evaluate("(i) => window.__sim.seekIndex(i)", kinds.index("hottest"))
                page.wait_for_timeout(50)
                hot_text = page.inner_text("#narration-body")
                hot_fn = expected.get("hottestFunction") or {}
                hot_node = next(
                    (node for node in expected["nodes"] if node["hottest"]), None)
                check(f"{case_id}: hottest beat names the global function",
                      bool(hot_fn)
                      and hot_fn.get("name", "") in hot_text
                      and f'(L{hot_fn.get("line")}, {hot_fn.get("path")})' in hot_text
                      and hot_fn.get("cost") is not None
                      and f'{hot_fn.get("cost"):,} units' in hot_text,
                      hot_text[:180])
                check(f"{case_id}: hottest beat keeps the file separate",
                      hot_node is not None
                      and f'Hottest file: {hot_node["name"]}' in hot_text,
                      hot_text[:90])
                old_fn = ((hot_node or {}).get("functions") or [{}])[0].get("name")
                if old_fn and old_fn != hot_fn.get("name"):
                    check(f"{case_id}: hottest beat drops the file-local function",
                          old_fn not in hot_text, old_fn)

                page.evaluate("(i) => window.__sim.seekIndex(i)", kinds.index("profile"))
                page.wait_for_timeout(50)
                profile_text = page.inner_text("#narration-body").lower()
                profile_title = page.inner_text("#narration-title").lower()
                check(f"{case_id}: profile labels the whole-repo basis",
                      "whole-repo" in profile_text
                      and "drawn-slice" not in profile_text
                      and "static candidates, verify" in profile_text
                      and f'{profile_meta["candidateTotal"]:,}' in profile_text
                      and str(profile_meta["demotedProduction"]) in profile_text
                      and "whole-repo" in profile_title,
                      profile_text[:180])
                actions = expected["meta"]["actions"][:3]
                page.evaluate("(i) => window.__sim.seekIndex(i)", kinds.index("start-here"))
                page.wait_for_timeout(50)
                start_text = page.inner_text("#narration-body")
                first_action = actions[0]
                start_title = page.get_attribute("#start-actions", "title") or ""
                check(f"{case_id}: start-here names up to 3 actions",
                      bool(actions)
                      and all(action["anchor"] in start_text for action in actions)
                      and ("on-canvas" in start_text or "off-canvas" in start_text),
                      start_text[:140])
                check(f"{case_id}: start-here carries scene and the first full anchor",
                      first_action["anchor"] in start_text
                      and first_action["scene"] in start_text
                      and first_action["anchor"] in start_title,
                      f'{first_action["anchor"]} · {first_action["scene"]} · title={start_title[:90]}')
                loop_index = kinds.index("loop")
                page.evaluate("(i) => window.__sim.seekIndex(i)", loop_index)
                page.wait_for_timeout(80)
                loop_text = page.inner_text("#narration-body")
                check(f"{case_id}: loop honesty caveat",
                      "inferred trip weight" in loop_text.lower(), loop_text[:90])
                focus = page.evaluate("() => window.__sim.state().focus")
                fn_label = page.evaluate(
                    "(sel) => { const el = document.querySelector(sel); return el ? el.textContent : ''; }",
                    f'#stage g[data-node="{focus["node"]}"] .node-top',
                )
                check(f"{case_id}: function highlight rendered",
                      bool(focus.get("fn")) and fn_label.startswith("▶") and "· L" in fn_label,
                      f'{focus.get("fn")} -> {fn_label}')
                page.evaluate("() => document.activeElement && document.activeElement.blur()")
                page.wait_for_timeout(200)
                loop_shot = EVIDENCE / f"{case_id}-loop.png"
                page.screenshot(path=str(loop_shot), full_page=False)
                check(f"{case_id}: loop keyframe", loop_shot.exists() and loop_shot.stat().st_size > 20000,
                      f"{loop_shot.stat().st_size if loop_shot.exists() else 0} bytes")
                page.evaluate("() => window.__sim.seekIndex(window.__sim.state().beats - 1)")
                page.wait_for_timeout(80)
                final_text = page.inner_text("body")
                summary_text = page.inner_text("#narration-body")
                first_action = expected["meta"]["actions"][0]
                check(f"{case_id}: final summary names the first start action",
                      f'Start: {first_action["anchor"]} — {first_action["title"]}.' in summary_text,
                      summary_text[-140:])
                final_claim = re.search(
                    r"\b\d+(?:\.\d+)?\s?(?:ms|µs|μs|milliseconds?|seconds?)\b", final_text, re.I)
                check(f"{case_id}: no time units at the final beat", final_claim is None,
                      final_claim.group(0) if final_claim else "none")
                final_hits = [pattern for pattern in FORBIDDEN_CLAIMS
                              if re.search(pattern, final_text, re.I)]
                check(f"{case_id}: F4 no forbidden claim at the final frame",
                      not final_hits, ", ".join(final_hits))
                runtime_hits = [match.group(0) for match in NEGATED_RUNTIME.finditer(final_text)]
                check(f"{case_id}: F4 runtime only appears negated",
                      not runtime_hits, str(runtime_hits[:3]))
                final_bare = unnegated_hits(final_text, NEGATABLE_CLAIMS)
                check(f"{case_id}: F4 no bare execution claim at the final frame",
                      not final_bare, ", ".join(sorted(set(final_bare))[:5]))
                if case_id == "cjson":
                    meta = expected["meta"]
                    check("cjson: findings cap disclosed",
                          f'{meta["findingsTotal"]} findings in the scan' in final_text
                          and f'{meta["findingsIncluded"]} included in this data slice' in final_text,
                          final_text[-180:])

                # ---- transport ------------------------------------------
                page.evaluate("() => window.__sim.seekIndex(0)")
                page.click("#step-forward")
                forward = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: step forward advances", forward["index"] == 1, forward["index"])
                page.click("#step-back")
                back = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: step back returns", back["index"] == 0, back["index"])

                page.click("#play-pause")
                page.wait_for_timeout(350)
                playing = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: play advances the clock",
                      playing["playing"] is True and playing["ms"] > 0, playing["ms"])
                page.click("#play-pause")
                paused = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: pause freezes", paused["playing"] is False)

                page.select_option("#speed", value="4")
                speed = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: speed control", speed["speed"] == 4, speed["speed"])
                page.select_option("#speed", value="1")

                page.evaluate(
                    """() => {
                        const scrub = document.querySelector('#scrub');
                        scrub.value = scrub.max;
                        scrub.dispatchEvent(new Event('input', {bubbles: true}));
                    }"""
                )
                end = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: scrub reaches the final beat",
                      end["index"] == end["beats"] - 1, f'{end["index"]}/{end["beats"] - 1}')

                page.click("#reset")
                reset = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: reset returns to beat 0",
                      reset["index"] == 0 and reset["ms"] == 0, f'{reset["index"]} @ {reset["ms"]}')

                # ---- S4: unresolved calls in the inspector --------------
                unresolved_node = next(
                    (node for node in expected["nodes"] if node.get("unresolvedCalls")), None)
                check(f"{case_id}: S4 unresolved-call data is present",
                      unresolved_node is not None)
                if unresolved_node:
                    page.evaluate(
                        """(path) => {
                            const node = document.querySelector(`#stage g[role="button"][data-node="${path}"]`);
                            if (node) node.dispatchEvent(new MouseEvent('click', {bubbles: true}));
                        }""",
                        unresolved_node["id"],
                    )
                    page.wait_for_timeout(50)
                    selected_unresolved = page.evaluate("() => window.__sim.state().selected")
                    insp_unresolved = page.inner_text("#inspector")
                    rows_dom = page.evaluate(
                        """() => [...document.querySelectorAll('#inspector [data-unresolved]')]
                            .map((li) => ({
                                line: li.getAttribute('data-unresolved-line'),
                                text: li.textContent,
                                title: li.title,
                            }))"""
                    )
                    expected_rows = unresolved_node["unresolvedCalls"]["rows"]
                    count = unresolved_node["unresolvedCalls"]["count"]
                    check(f"{case_id}: S4 unresolved node opens the inspector",
                          selected_unresolved == unresolved_node["id"],
                          str(selected_unresolved))
                    check(f"{case_id}: S4 inspector renders unresolved-call rows",
                          "UNRESOLVED CALLS" in insp_unresolved
                          and 0 < len(rows_dom) <= 6
                          and len(rows_dom) == len(expected_rows),
                          f"{len(rows_dom)}/{count} rows")
                    check(f"{case_id}: S4 unresolved rows never guess a target",
                          all(row["text"].startswith(f'L{row["line"]}')
                              and "→" not in row["text"]
                              and "guess" not in row["text"].lower()
                              for row in rows_dom)
                          and all(
                              row["line"] == str(expected_row["line"])
                              and expected_row["callee"] in row["text"]
                              and row["title"] == expected_row["caller"]
                              for row, expected_row in zip(rows_dom, expected_rows)
                          ),
                          str(rows_dom[:2]))
                    if count > len(rows_dom):
                        check(f"{case_id}: S4 unresolved rows disclose the cap",
                              f'showing {len(rows_dom)} of {count:,}' in insp_unresolved,
                              f'{len(rows_dom)}/{count}')
                    page.keyboard.press("Escape")

                # ---- inspector ------------------------------------------
                target = expected["trace"][0]["path"]
                page.evaluate(
                    """(path) => {
                        const node = document.querySelector(`#stage g[role="button"][data-node="${path}"]`)
                            || document.querySelectorAll('#stage g[role="button"]')[0];
                        node.dispatchEvent(new MouseEvent('click', {bubbles: true}));
                    }""",
                    target,
                )
                selected = page.evaluate("() => window.__sim.state().selected")
                check(f"{case_id}: node select opens the inspector", bool(selected), str(selected))
                check(f"{case_id}: inspector shows the path",
                      expected["trace"][0]["path"] in page.inner_text("#inspector"))
                page.keyboard.press("Escape")
                check(f"{case_id}: Escape closes the inspector",
                      not page.evaluate("() => window.__sim.state().selected"))

                # ---- keyframes ------------------------------------------
                last = page.evaluate("() => window.__sim.state().beats") - 1
                for label, index in (("start", 0), ("mid", last // 2), ("end", last)):
                    page.evaluate("(i) => window.__sim.seekIndex(i)", index)
                    page.evaluate("() => document.activeElement && document.activeElement.blur()")
                    page.wait_for_timeout(250)
                    shot = EVIDENCE / f"{case_id}-{label}.png"
                    page.screenshot(path=str(shot), full_page=False)
                    check(f"{case_id}: {label} keyframe", shot.exists() and shot.stat().st_size > 20000,
                          f"{shot.stat().st_size if shot.exists() else 0} bytes")

                # ---- final frame completeness ---------------------------
                page.evaluate("(i) => window.__sim.seekIndex(i)", last)
                visible = page.evaluate(
                    """() => {
                        const effective = (el) => {
                            let opacity = 1;
                            let node = el;
                            while (node && node !== document.documentElement) {
                                opacity *= parseFloat(getComputedStyle(node).opacity || '1');
                                node = node.parentElement;
                            }
                            return opacity;
                        };
                        return [...document.querySelectorAll('#stage g[role="button"]')]
                            .every(node => effective(node) >= 0.9);
                    }"""
                )
                check(f"{case_id}: final frame shows every node", visible)
                check(f"{case_id}: final frame keeps the honesty label",
                      "STATIC, INFERRED" in page.inner_text("#honesty"))

            # ---- F5: final-frame DOM determinism -----------------------
            for case_id in REAL_CASES:
                hashes = []
                for _ in range(2):
                    det_page = context.new_page()
                    det_page.goto(BASE, wait_until="networkidle")
                    det_page.wait_for_function(
                        "() => window.__sim && window.__sim.state().beats > 0")
                    det_page.select_option("#case-select", value=case_id)
                    det_page.wait_for_function(
                        "id => window.__sim.state().caseId === id", arg=case_id)
                    det_page.evaluate("() => window.__sim.applyFinal()")
                    det_page.wait_for_timeout(150)
                    hashes.append(hashlib.sha256(stage_dom(det_page).encode()).hexdigest())
                    det_page.close()
                check(f"{case_id}: F5 final frame DOM is deterministic across loads",
                      hashes[0] == hashes[1],
                      f"{hashes[0][:16]} == {hashes[1][:16]}; data byte-identical "
                      "across scans + DOM identical across loads ⇒ frame deterministic; "
                      "PNGs are rasterization-dependent")

            # ---- synthetic state demos ----------------------------------
            for case_id in ("no-forward-path", "empty", "capped"):
                page.select_option("#case-select", value=case_id)
                page.wait_for_function(
                    "id => window.__sim.state().caseId === id", arg=case_id, timeout=10000)
                demo = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: labelled synthetic", demo["synthetic"] is True)
                check(f"{case_id}: synthetic purpose fallback unchanged",
                      page.inner_text("#brand-subline").strip()
                      == "entry-flow map · static, profile to confirm")
                check(f"{case_id}: S4 detail hidden for the state demo",
                      page.evaluate("""() => {
                          const section = document.querySelector('#s4-detail');
                          return !section || section.hidden;
                      }"""))
                check(f"{case_id}: stub panel visible", page.is_visible("#stub"))
                check(f"{case_id}: no fabricated nodes",
                      page.evaluate("() => document.querySelectorAll('#stage g[role=\"button\"]').length") == 0)
                page.evaluate("() => window.__sim.seekIndex(window.__sim.state().beats - 2)")
                page.evaluate("() => document.activeElement && document.activeElement.blur()")
                page.wait_for_timeout(200)
                expected_chip = {"no-forward-path": "no forward path", "empty": "empty", "capped": "capped"}
                title = page.inner_text("#narration-title").lower()
                check(f"{case_id}: stop title names the state",
                      expected_chip[case_id] in title, title[:80])
                page.evaluate("() => window.__sim.seekIndex(window.__sim.state().beats - 1)")
                summary = page.inner_text("#narration-body").lower()
                check(f"{case_id}: summary does not claim a scan",
                      "no findings in this state demo" in summary, summary[:90])
                page.evaluate("() => window.__sim.seekIndex(window.__sim.state().beats - 2)")
                shot = EVIDENCE / f"{case_id}.png"
                page.screenshot(path=str(shot), full_page=False)
                check(f"{case_id}: state keyframe", shot.exists() and shot.stat().st_size > 10000,
                      f"{shot.stat().st_size if shot.exists() else 0} bytes")

            # ---- layout fit ---------------------------------------------
            page.select_option("#case-select", value="hyperfine")
            page.wait_for_function("() => window.__sim.state().caseId === 'hyperfine'")
            fit = page.evaluate(
                """() => {
                    const footer = document.querySelector('#canvas-footer').getBoundingClientRect();
                    const scrolling = document.scrollingElement || document.documentElement;
                    return {footerBottom: Math.round(footer.bottom),
                            scrollWidth: scrolling.scrollWidth,
                            clientWidth: scrolling.clientWidth,
                            bodyScrollWidth: document.body.scrollWidth,
                            innerWidth: window.innerWidth};
                }"""
            )
            check("first page fits 1440x900", fit["footerBottom"] <= 900, f'footer {fit["footerBottom"]}')
            check("no horizontal scroll at 1440", fit["scrollWidth"] <= fit["clientWidth"] + 1, str(fit))
            check("no horizontal overflow (body at 1440)",
                  fit["bodyScrollWidth"] <= fit["innerWidth"] + 1, str(fit))
            check("dominant-badge definition present",
                  "dominant rule annotation" in page.inner_text("#badge-def"))
            page.set_viewport_size({"width": 1280, "height": 800})
            page.wait_for_timeout(200)
            narrow = page.evaluate(
                """() => {
                    const s = document.scrollingElement || document.documentElement;
                    return {scrollWidth: s.scrollWidth, clientWidth: s.clientWidth};
                }"""
            )
            check("no horizontal scroll at 1280x800",
                  narrow["scrollWidth"] <= narrow["clientWidth"] + 1, str(narrow))

            # ---- reduced motion ----------------------------------------
            reduced = browser.new_context(viewport={"width": 1440, "height": 900},
                                          reduced_motion="reduce")
            reduced_page = reduced.new_page()
            reduced_requests = []
            reduced_console = []
            reduced_page.on("request", lambda req: reduced_requests.append(req.url))
            reduced_page.on("console", lambda msg: reduced_console.append(msg.text)
                            if msg.type == "error" else None)
            reduced_page.goto(BASE, wait_until="networkidle")
            reduced_page.wait_for_function("() => window.__sim && window.__sim.state().beats > 0")
            reduced_state = reduced_page.evaluate("() => window.__sim.state()")
            check("reduced motion starts at the final frame",
                  reduced_state["index"] == reduced_state["beats"] - 1,
                  f'{reduced_state["index"]}/{reduced_state["beats"] - 1}')
            reduced_external = [url for url in reduced_requests
                                if not (url.startswith(BASE) or url.startswith("data:")
                                        or url.startswith("blob:"))]
            check("reduced motion: no non-local requests", not reduced_external,
                  str(reduced_external[:3]))
            check("reduced motion: no console errors", not reduced_console,
                  "; ".join(reduced_console[:3]))
            reduced.close()

            # ---- no network, no console errors --------------------------
            external = [url for url in requests
                        if not (url.startswith(BASE) or url.startswith("data:")
                                or url.startswith("blob:"))]
            check("no non-local network requests", not external, str(external[:3]))
            check("no console errors", not console_errors, "; ".join(console_errors[:3]))
            check("no page errors", not page_errors, "; ".join(page_errors[:3]))

            context.close()
            browser.close()
    finally:
        server.terminate()
        try:
            server.wait(timeout=5)
        except subprocess.TimeoutExpired:
            server.kill()

    # ---- data must be read-only ----------------------------------------
    after = {path.name: sha256(path) for path in sorted(SIM.glob("data-*.json"))}
    check("data files unchanged by the gate", before == after)

    return finish()


def finish() -> int:
    failed = [entry for entry in CHECKS if not entry["ok"]]
    payload = {
        "date": date.today().isoformat(),
        "viewport": "1440x900",
        "total": len(CHECKS),
        "passed": len(CHECKS) - len(failed),
        "failed": len(failed),
        "checks": CHECKS,
    }
    (EVIDENCE / "checks.json").write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(f"\n{payload['passed']}/{payload['total']} checks passed"
          f" -> {EVIDENCE / 'checks.json'}")
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
