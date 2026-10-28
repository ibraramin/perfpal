#!/usr/bin/env python3
"""Derive the C1 contract sample from a real metrics.json + the spec's hint table.

EVIDENCE-ONLY, NON-NORMATIVE tooling (founder-ratified 2026-10-24, spec §2 D8).
It is not production code, is never shipped, and does not prove renderer parity:
C2/C3 replace it with `src/reporter/agent_report.rs` and diff the result.

Mirrors the shipped key-view model (`src/reporter/views.rs`):
  * action rank = production-first, severity, confidence, containing-function
    cost_v2 desc, finding_id asc (KV1; `-ARCH-002` excluded)
  * profile rank = production-first, has-loop, non-startup, cost desc, path,
    name, id (KV3)
  * cost anchor = "<cost> static cost units (v2|v1) · <ratio>× median · top P%"
  * median = lower-middle for even N (nearest-rank, zeros included)
  * top P% = max(1, round(100*rank/N))
  * fix hints come from the spec table (single source) — never invented here

Fail-closed rules (exit non-zero):
  * hint table is not exactly 78 rows, has a duplicate id, or a hint > 20 words;
  * an action's rule_id has no hint;
  * a v1-only function is present (the chart-side v1 cost is not reproduced here);
  * a forbidden time claim appears in C-authored text;
  * complete_action_count != actions.len();
  * the rendered MD exceeds 64 KB.

Usage: derive_sample.py <metrics.json> <out-dir> [spec.md]
"""
import json
import re
import sys
from pathlib import Path

SEV = {"hot": 0, "warn": 1, "info": 2}
CONF = {"HIGH": 0, "MEDIUM": 1, "LOW": 2}
ACTIONS_DETAIL_CAP = 25
PROFILE_INLINE_CAP = 25
PROFILE_JSON_CAP = 100
SIZE_BUDGET = 65536
FORBIDDEN = re.compile(r"\b(ms|µs|seconds|speedup|latency|throughput|runtime)\b", re.IGNORECASE)
# Analyser provenance (D7): quoted finding text is exempt from the claim scan.
PROVENANCE_PREFIXES = (
    "- why:",
    "- what:",
    "- function:",
    "- evidence:",
    "- anchor:",
    "### ",
    "| PERF-",
    "| rule",
    "|---",
)
PROFILE_ROW = re.compile(r"^\d+\. `")


def _single_line(text):
    """Newlines forge headings; a backtick closes a code span (CommonMark ignores backslash)."""
    return (
        str(text)
        .replace("\r", "\\r")
        .replace("\n", "\\n")
        .replace("\t", "\\t")
        .replace("`", "\u02cb")  # MODIFIER LETTER GRAVE ACCENT: never closes a span
    )


def md_code(text):
    """Inside a code span: single-line + closed backticks; `<`/`>` are literal and safe."""
    return _single_line(text)


def md_text(text):
    """Prose (outside a code span): also neutralise raw HTML."""
    return _single_line(text).replace("<", "&lt;").replace(">", "&gt;")


def md_cell(text):
    """Table cell: prose rendering plus column-separator escaping."""
    return md_text(text).replace("|", "\\|")


def load_hints(spec_path):
    """Parse `| PERF-... | hint |` rows from the spec's fix-hint table."""
    hints = {}
    for line in spec_path.read_text().splitlines():
        match = re.match(r"^\| (PERF-[A-Z0-9-]+) \| (.+?) \|$", line)
        if match:
            rule_id, hint = match.group(1), match.group(2).strip()
            if rule_id in hints:
                raise SystemExit(f"duplicate hint row for {rule_id}")
            if len(hint.split()) > 20:
                raise SystemExit(f"hint for {rule_id} exceeds 20 words")
            hints[rule_id] = hint
    if len(hints) != 78:
        raise SystemExit(f"expected 78 hint rows in {spec_path}, found {len(hints)}")
    return hints


def containing_function(functions_by_path, finding):
    line = finding["span"]["start_line"]
    best = None
    for fn in functions_by_path.get(finding["path"], []):
        span = fn["span"]
        if span["start_line"] <= line <= span["end_line"]:
            key = (span["end_line"] - span["start_line"], fn["id"])
            if best is None or key < best[0]:
                best = (key, fn)
    return best[1] if best else None


def median(values):
    if not values:
        return None
    ordered = sorted(values)
    return ordered[(len(ordered) - 1) // 2]


def json_cost(value):
    """Integral costs serialize as JSON integers (spec §6.1)."""
    return int(value) if float(value).is_integer() else round(value, 1)


def ratio_label(r):
    if r < 10.0:
        return f"{r:.1f}×"
    return f"{round(r)}×"


def top_label(k, n):
    if n == 0:
        return "top 1%"
    pct = max(1, min(100, (100 * k + n // 2) // n))
    return f"top {pct}%"


class Scale:
    def __init__(self, functions, scale):
        if scale == "v2":
            members = [f for f in functions if f.get("function_cost_v2") is not None]
            value = lambda f: float(f["function_cost_v2"])
        else:
            members = [f for f in functions if f.get("function_cost_v2") is None]
            if members:
                raise SystemExit(
                    "v1-only functions present: the chart-side v1 cost is not reproduced "
                    "by this evidence script (spec §5.2 item 4)"
                )
            value = lambda f: 0.0
        members.sort(key=lambda f: (-value(f), f["path"], f["name"], f["id"]))
        self.unit = f"static cost units ({scale})"
        self.scale = scale
        self.value = value
        self.median = median([value(f) for f in members])
        self.rank = {f["id"]: i + 1 for i, f in enumerate(members)}
        self.total = len(members)

    def anchor(self, fn):
        parts = [f"{self.value(fn):.0f} {self.unit}"]
        if self.median and self.median > 0:
            parts.append(f"{ratio_label(self.value(fn) / self.median)} median")
        rank = self.rank.get(fn["id"])
        if rank is not None:
            parts.append(top_label(rank, self.total))
        return " · ".join(parts)


def span_obj(span):
    """Emit span keys in the schema/declaration order, not the file's key order."""
    return {
        "start_line": span["start_line"],
        "start_col": span["start_col"],
        "end_line": span["end_line"],
        "end_col": span["end_col"],
    }


def finding_anchor(finding_id):
    return "finding-" + "".join(
        ch if (ch.isalnum() and ch.isascii()) else "-" for ch in finding_id
    )


def build(doc, hints):
    functions = doc["stats"]["functions"]
    findings = doc["findings"]
    path_class = {f["path"]: f.get("path_class", "unknown") for f in doc["files"]}
    fby = {}
    for fn in functions:
        fby.setdefault(fn["path"], []).append(fn)

    v2 = Scale(functions, "v2")
    v1 = Scale(functions, "v1")

    def scale_of(fn):
        return v2 if fn.get("function_cost_v2") is not None else v1

    actions = []
    for index, finding in enumerate(findings):
        if finding["rule_id"].endswith("-ARCH-002"):
            continue
        container = containing_function(fby, finding)
        production = path_class.get(finding["path"], "unknown") == "production"
        cost_v2 = (container or {}).get("function_cost_v2") or 0
        actions.append((index, finding, container, production, cost_v2))
    actions.sort(
        key=lambda row: (
            not row[3],
            SEV[row[1]["severity"]],
            CONF[row[1]["confidence"]],
            -row[4],
            row[1]["finding_id"],
        )
    )

    # Anchor assignment: sorted finding_id order, duplicate bases suffixed.
    order = sorted(range(len(findings)), key=lambda i: findings[i]["finding_id"])
    bases = [finding_anchor(findings[i]["finding_id"]) for i in order]
    seen, unique = {}, []
    for base in bases:
        seen[base] = seen.get(base, 0) + 1
        unique.append(base if seen[base] == 1 else f"{base}-{seen[base]}")
    anchors = {index: unique[pos] for pos, index in enumerate(order)}

    out_actions = []
    for rank, (index, finding, container, _production, _cost) in enumerate(actions, 1):
        fn_obj = None
        if container is not None:
            scale = scale_of(container)
            fn_obj = {
                "id": container["id"],
                "name": container["name"],
                "path": container["path"],
                "span": span_obj(container["span"]),
                "cost": json_cost(scale.value(container)),
                "cost_scale": scale.scale,
                "cost_anchor": scale.anchor(container),
            }
        out_actions.append(
            {
                "action_id": finding["finding_id"],
                "rank": rank,
                "rule_id": finding["rule_id"],
                "title": finding["title"],
                "path": finding["path"],
                "symbol": finding.get("symbol"),
                "span": span_obj(finding["span"]),
                "severity": finding["severity"],
                "confidence": finding["confidence"],
                "resolver_tier": finding["resolver_tier"],
                "macro_suspect": finding["macro_suspect"],
                "error_node_overlap": finding["error_node_overlap"],
                "threshold": finding.get("threshold"),
                "path_class": path_class.get(finding["path"], "unknown"),
                "function": fn_obj,
                "why": finding["message"],
                "evidence": finding.get("evidence", {}),
                "fix_hint": hints.get(finding["rule_id"]),
                "detail_anchor": anchors[index],
            }
        )

    def kv3_key(fn):
        cls = path_class.get(fn["path"], "unknown")
        return (
            cls != "production",
            fn["max_loop_depth"] == 0,
            fn.get("entry_kind") in ("main", "script"),
            -scale_of(fn).value(fn),
            fn["path"],
            fn["name"],
            fn["id"],
        )

    profile = []
    for rank, fn in enumerate(sorted(functions, key=kv3_key), 1):
        scale = scale_of(fn)
        profile.append(
            {
                "rank": rank,
                "function_id": fn["id"],
                "path": fn["path"],
                "function_name": fn["name"],
                "path_class": path_class.get(fn["path"], "unknown"),
                "cost": json_cost(scale.value(fn)),
                "cost_scale": scale.scale,
                "cost_anchor": scale.anchor(fn),
                "unknown_trip_loops": fn.get("unknown_trip_loops", 0),
                "known_trip_value": fn.get("known_trip_value"),
            }
        )

    call_edges = doc.get("graphs", {}).get("call_graph", {}).get("edges", [])
    include_edges = doc.get("graphs", {}).get("include_graph", {}).get("edges", [])
    mix = {"HIGH": 0, "MEDIUM": 0, "LOW": 0}
    for edge in call_edges:
        confidence = edge.get("confidence", "LOW")
        mix[confidence] = mix.get(confidence, 0) + 1
    health = doc.get("parse_health_summary")
    scan = doc["scan"]
    data_quality = {
        "parse_health": (
            {
                "ok": health["ok"],
                "partial": health["partial"],
                "failed": health["failed"],
            }
            if health is not None
            else None
        ),
        "unresolved_includes": {
            "unresolved": sum(1 for e in include_edges if not e.get("resolved")),
            "total": len(include_edges),
        },
        "structural_call_edges": {
            "total": len(call_edges),
            "high": mix["HIGH"],
            "medium": mix["MEDIUM"],
            "low": mix["LOW"],
        },
        "non_production_files": sum(1 for v in path_class.values() if v != "production"),
        "startup_functions": sum(
            1 for fn in functions if fn.get("entry_kind") in ("main", "script")
        ),
        "no_loop_functions": sum(1 for fn in functions if fn["max_loop_depth"] == 0),
        "cost_scale": {"v2_functions": v2.total, "v1_functions": v1.total},
        "unknown_trip_functions": sum(
            1 for fn in functions if fn.get("unknown_trip_loops", 0) > 0
        ),
        "component_findings_excluded": sum(
            1 for f in findings if f["rule_id"].endswith("-ARCH-002")
        ),
        "resolver_tiers": sorted({f["resolver_tier"] for f in findings}),
        "caps": {"actions_detail": ACTIONS_DETAIL_CAP, "profile_inline": PROFILE_INLINE_CAP},
    }

    envelope = {
        "artifact": "agent-actions",
        "contract_version": 1,
        "schema_version": doc["schema_version"],
        "tool": doc["tool"],
        "scan": {
            "root_path": scan["root_path"],
            "languages": scan["languages"],
            "file_count": scan["file_count"],
            "loc_total": (
                {
                    "code": scan["loc_total"]["code"],
                    "comment": scan["loc_total"]["comment"],
                    "blank": scan["loc_total"]["blank"],
                }
                if scan.get("loc_total") is not None
                else None
            ),
        },
        "rank_key": {
            "actions": [
                "path_class:production-first",
                "severity:hot>warn>info",
                "confidence:HIGH>MEDIUM>LOW",
                "function_cost_v2:desc",
                "finding_id:asc",
            ],
            "profile_queue": [
                "path_class:production-first",
                "max_loop_depth>0:first",
                "entry_kind:non-startup-first",
                "cost:desc",
                "path:asc",
                "function_name:asc",
                "function_id:asc",
            ],
        },
        "data_quality": data_quality,
        "actions": out_actions,
        "complete_action_count": len(out_actions),
        "profile_queue": profile[:PROFILE_JSON_CAP],
        "profile_queue_cap": PROFILE_JSON_CAP,
        "complete_profile_count": len(profile),
    }
    if envelope["complete_action_count"] != len(envelope["actions"]):
        raise SystemExit("complete_action_count != actions.len()")
    if any(action["fix_hint"] is None for action in out_actions):
        raise SystemExit("an action's rule_id has no curated hint in the spec table")
    for action in out_actions:
        if FORBIDDEN.search(action["fix_hint"]):
            raise SystemExit(f"forbidden time claim in hint for {action['rule_id']}")
    return envelope, doc


def md_report(env, doc):
    dq = env["data_quality"]
    scan = env["scan"]
    complete = env["complete_action_count"]
    profile_total = env["complete_profile_count"]
    total_findings = len(doc["findings"])
    tiers = ", ".join(dq["resolver_tiers"])
    health = dq["parse_health"]
    parse_line = (
        f"parse: {health['ok']} ok / {health['partial']} partial / {health['failed']} failed"
        if health is not None
        else "parse: not recorded"
    )
    languages = ", ".join(scan["languages"])
    machine_cap = env["profile_queue_cap"]
    machine_rows = min(machine_cap, profile_total)
    profile_md_cap = dq["caps"]["profile_inline"]
    profile_md_rows = min(profile_md_cap, profile_total)
    lines = [
        "# Perfpal Agent Report",
        "",
        f"> contract {env['contract_version']} · schema {env['schema_version']} · "
        f"{env['tool']['name']} {env['tool']['version']} · scan root: `{md_code(scan['root_path'])}`",
        f"> {scan['file_count']} files · {len(doc['stats']['functions'])} functions · "
        f"{total_findings} findings · languages: {md_code(languages)}",
        "",
        "## How to use this report",
        "",
        "- Every row is an **optimization candidate**, not a defect verdict; confirm it with a profiler.",
        "- Cost is **static cost units**, a heuristic; it is never a duration.",
        "- Rank key: production code first, then severity (hot > warn > info), then confidence "
        "(HIGH > MEDIUM > LOW), then containing-function static cost descending, then finding_id ascending.",
        f"- Machine contract: `agent-actions.json` holds all {complete} actions and the top "
        f"{machine_rows} of {profile_total} profile rows.",
        "",
        "## Data quality",
        "",
        f"- {parse_line}",
        f"- unresolved includes: {dq['unresolved_includes']['unresolved']} of "
        f"{dq['unresolved_includes']['total']} · structural call edges: "
        f"{dq['structural_call_edges']['total']} ({dq['structural_call_edges']['high']} HIGH / "
        f"{dq['structural_call_edges']['medium']} MEDIUM / {dq['structural_call_edges']['low']} LOW)",
        f"- demotions: {dq['non_production_files']} non-production files · "
        f"{dq['startup_functions']} startup functions · {dq['no_loop_functions']} no-loop functions "
        "(counted, never dropped)",
        f"- cost scale: {dq['cost_scale']['v2_functions']} v2 / {dq['cost_scale']['v1_functions']} v1 functions",
        f"- resolver tier(s): {md_code(tiers)}",
        f"- functions with unknown trip counts: {dq['unknown_trip_functions']} — their cost estimate is incomplete",
        f"- component-level findings excluded from actions: {dq['component_findings_excluded']} "
        "(see the human Explorer)",
        f"- caps: top {dq['caps']['actions_detail']} actions and {profile_md_cap} profile rows here; "
        f"all actions + top {machine_cap} profile rows in `agent-actions.json`",
        "",
        "## Start here — ranked actions",
        "",
        f"showing top {min(ACTIONS_DETAIL_CAP, complete)} of {complete} actions",
        "",
    ]
    if not env["actions"]:
        lines += ["no findings — nothing to rank on this scan.", ""]
    for action in env["actions"][:ACTIONS_DETAIL_CAP]:
        lines.append(
            f"### {action['rank']}. {md_code(action['rule_id'])} · "
            f"`{md_code(action['path'])}:{action['span']['start_line']}` · "
            f"{action['severity']}/{action['confidence']} · {action['path_class']}"
        )
        lines.append(f"- what: {md_text(action['title'])}")
        if action["function"]:
            fn = action["function"]
            lines.append(
                f"- function: `{md_code(fn['name'])}` "
                f"(`{md_code(fn['path'])}:{fn['span']['start_line']}`) · {fn['cost_anchor']}"
            )
        else:
            lines.append("- function: no enclosing function recorded for this location")
        lines.append(f"- why: {md_text(action['why'])}")
        if action["evidence"]:
            ev = " · ".join(
                f"{md_code(k)}={md_code(action['evidence'][k])}"
                for k in sorted(action["evidence"])
            )
            lines.append(f"- evidence: `{ev}`")
        lines.append(f"- fix: {action['fix_hint'] or 'no curated hint for this rule yet'}")
        lines.append(
            f"- anchor: `findings-detail.html#{action['detail_anchor']}` · "
            f"`{md_code(action['action_id'])}`"
        )
        lines.append("")
    if complete > ACTIONS_DETAIL_CAP:
        lines += [
            f"showing the top {ACTIONS_DETAIL_CAP} of {complete} actions; the complete list is "
            "`agent-actions.json`.",
            "",
        ]
    lines += ["## Action index by rule", ""]
    rules = {}
    for action in env["actions"]:
        entry = rules.setdefault(
            action["rule_id"],
            {"count": 0, "hot": 0, "warn": 0, "info": 0, "title": action["title"], "top": action["rank"]},
        )
        entry["count"] += 1
        entry[action["severity"]] += 1
        entry["top"] = min(entry["top"], action["rank"])
    if not rules:
        lines += ["no rules fired — nothing to index.", ""]
    else:
        lines += [
            "| rule | title | actions | hot | warn | info | top rank |",
            "|---|---|---:|---:|---:|---:|---:|",
        ]
        for rule_id in sorted(rules):
            row = rules[rule_id]
            lines.append(
                f"| {rule_id} | {md_cell(row['title'])} | {row['count']} | {row['hot']} | "
                f"{row['warn']} | {row['info']} | {row['top']} |"
            )
        lines.append("")
    lines += ["## Profile queue", ""]
    lines.append(f"showing top {profile_md_rows} of {profile_total} functions")
    lines.append("")
    if not env["profile_queue"]:
        lines += ["no functions recorded — nothing to queue.", ""]
    for row in env["profile_queue"][:profile_md_cap]:
        suffix = ""
        if row["unknown_trip_loops"]:
            suffix = f" · unknown-trip ×{row['unknown_trip_loops']} — estimate incomplete"
        lines.append(
            f"{row['rank']}. `{md_code(row['function_name'])}` — `{md_code(row['path'])}` — "
            f"{row['cost_anchor']} · {row['path_class']}{suffix}"
        )
    if profile_total > profile_md_cap:
        lines += [
            "",
            f"showing the top {profile_md_cap} of {profile_total} functions; complete human list "
            "in `profile-queue.md`, machine rows in `agent-actions.json`.",
        ]
    lines += [
        "",
        "## What this scan cannot see",
        "",
        f"- **Syntactic tier only** (`resolver_tier: {md_code(tiers)}`): no macro expansion, no type resolution.",
        "- **Structural call edges** are name-based/semantic and may over/under-approximate; not resolved.",
        "- **Static cost is a heuristic**, not a duration; a high rank is a hypothesis to profile.",
        "- **Partial or failed parses** and unresolved includes reduce coverage; the counts above say by how much.",
        "- **Findings are candidates** for human or agent review, never defect claims.",
        "",
    ]
    md = "\n".join(lines)
    for line in md.splitlines():
        if line.startswith(PROVENANCE_PREFIXES) or PROFILE_ROW.match(line):
            continue
        if FORBIDDEN.search(line):
            raise SystemExit(f"forbidden time claim in C-authored text: {line!r}")
    size = len(md.encode())
    if size > SIZE_BUDGET:
        raise SystemExit(f"agent-report.md is {size} B, over the {SIZE_BUDGET} B budget")
    return md


def main():
    metrics_path = Path(sys.argv[1])
    out_dir = Path(sys.argv[2])
    spec_path = Path(sys.argv[3]) if len(sys.argv) > 3 else Path("docs/specs/agent-report.md")
    hints = load_hints(spec_path)
    doc = json.loads(metrics_path.read_text())
    env, doc = build(doc, hints)
    out_dir.mkdir(parents=True, exist_ok=True)
    (out_dir / "agent-actions.sample.json").write_text(
        json.dumps(env, indent=2, ensure_ascii=False) + "\n"
    )
    (out_dir / "agent-report.sample.md").write_text(md_report(env, doc))
    print(
        f"actions={env['complete_action_count']} profile={env['complete_profile_count']} "
        f"hints={len(hints)} bytes={len(md_report(env, doc).encode())} ok"
    )


if __name__ == "__main__":
    main()
