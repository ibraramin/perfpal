#!/usr/bin/env python3
"""Generate the V3 idiom prototypes (18 self-contained HTML pages).

Contract:
  docs/evidence/map-vision/v3/prereg.md  (section 3 is normative)
  docs/specs/map-intelligibility.md      (locked slot contract)

Prototype-only tooling: reads pre-existing scan output, writes only under
perfpal-test/v3-prototypes and docs/evidence/map-vision/v3. No product code.
"""

import datetime
import hashlib
import html
import json
import re
import statistics
from pathlib import Path

TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
V3_DIR = Path("/home/shafin/Opencode/Perfpal/docs/evidence/map-vision/v3")
PROTO_DIR = TEST_ROOT / "v3-prototypes"

REPOS = ["tmux", "hyperfine", "cJSON"]
SLOT_IDIOMS = [
    ("kv1", "brief"),
    ("kv1", "cards"),
    ("kv2", "bars"),
    ("kv2", "matrix"),
    ("kv3", "bars"),
    ("kv3", "treemap"),
]

# prereg.md header data SHAs (sha256 prefix as recorded)
EXPECTED_SHA_PREFIX = {
    "tmux": "62fd2e23",
    "hyperfine": "5735135b",
    "cJSON": "99736e9f",
}

QUESTIONS = {
    "kv1": "Where do I look first?",
    "kv2": "What breaks if I touch it?",
    "kv3": "Where might time plausibly go?",
}
ACTIONS = {
    "kv1": "Open the first path:line and profile it — static candidates, confirm with a profiler.",
    "kv2": "Changing a file here forces re-import of its dependents — do not edit directly.",
    "kv3": "Profile the named function first — static candidate, confirm with a profiler.",
}
CAPTIONS = {
    "kv2": "Changing a file here forces re-import of its dependents — do not edit directly.",
    "kv3": "Profile the named function first — static candidate, confirm with a profiler.",
}
HONESTY = "Perfpal static analysis — optimization candidates, not runtime truth."
SLOT_LABEL = {"kv1": "KV1", "kv2": "KV2", "kv3": "KV3"}

NONPROD_DIRS = re.compile(
    r"(^|/)(tests?|testing|specs?|examples?|fuzz(ing)?|benches?|benchmarks?|vendor|third_party|generated)(/|$)"
)
NONPROD_BASE = re.compile(r"^(test_.*|.*_test\..*|.*\.test\..*)$")

SEVERITY_RANK = {"hot": 0, "warn": 1, "info": 2}
CONFIDENCE_RANK = {"HIGH": 0, "MEDIUM": 1, "LOW": 2}


# --------------------------------------------------------------------------
# helpers
# --------------------------------------------------------------------------
def esc(value):
    return html.escape(str(value), quote=True)


def is_nonprod(path):
    """prereg section 3 path_class proxy. compat/ stays production."""
    low = path.lower()
    base = low.rsplit("/", 1)[-1]
    return bool(NONPROD_DIRS.search(low) or NONPROD_BASE.match(base))


def p95(values):
    """Nearest-rank p95 over all values (0s included)."""
    if not values:
        return 0
    ordered = sorted(values)
    n = len(ordered)
    idx = (19 * n + 19) // 20 - 1  # ceil(0.95 * n) - 1
    return ordered[max(0, min(n - 1, idx))]


def top_label(rank, total):
    """1-based rank of total -> top 1% / 5% / 10% / 25% label."""
    pct = max(1, int(100.0 * rank / total + 0.5))
    if pct <= 1:
        return "top 1%"
    if pct <= 5:
        return "top 5%"
    if pct <= 10:
        return "top 10%"
    return "top 25%"


def ratio_label(value, median):
    if not median:
        return None
    r = value / median
    return f"{r:.1f}\u00d7" if r < 10 else f"{r:.0f}\u00d7"


def first_sentence(message):
    msg = (message or "").strip()
    match = re.search(r"\.(\s|$)", msg)
    text = msg[: match.start() + 1] if match else msg
    if len(text) > 140:
        text = text[:137].rstrip() + "..."
    return text


def fmt_int(value):
    return str(int(value))


def sha256_of(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_repo(repo):
    path = TEST_ROOT / f"out-i5b-{repo}" / "metrics.json"
    if not path.exists():
        raise SystemExit(f"FATAL: missing input {path}")
    digest = sha256_of(path)
    expected = EXPECTED_SHA_PREFIX[repo]
    if not digest.startswith(expected):
        raise SystemExit(
            f"FATAL: sha256 mismatch for {path}\n"
            f"  expected prefix {expected}\n  actual          {digest}"
        )
    with open(path, "r", encoding="utf-8") as handle:
        data = json.load(handle)
    if str(data.get("schema_version")) != "1.6":
        raise SystemExit(
            f"FATAL: {path} schema_version is {data.get('schema_version')!r}, expected '1.6'"
        )
    return data, digest


# --------------------------------------------------------------------------
# model
# --------------------------------------------------------------------------
class RepoModel:
    def __init__(self, repo, data, sha):
        self.repo = repo
        self.data = data
        self.sha = sha
        self.file_paths = [f["path"] for f in data["files"]]
        self.nonprod = {p: is_nonprod(p) for p in self.file_paths}
        self.funcs = data["stats"]["functions"]
        self.findings = data["findings"]
        self.funcs_by_path = {}
        for f in self.funcs:
            self.funcs_by_path.setdefault(f["path"], []).append(f)
        for bucket in self.funcs_by_path.values():
            bucket.sort(key=lambda f: (f["span"]["start_line"], f["span"]["end_line"], f["id"]))

        self.costs = [f["function_cost_v2"] for f in self.funcs]
        self.median_cost = statistics.median(self.costs) if self.costs else 0
        self.p95_cost = p95(self.costs)

        edges = data["graphs"]["include_graph"]["edges"]
        self.edge_total = len(edges)
        self.edge_resolved = 0
        self.indeg = {}
        self.outdeg = {}
        self.pairs = set()
        for edge in edges:
            if not edge.get("resolved"):
                continue
            self.edge_resolved += 1
            src, dst = edge.get("from"), edge.get("to")
            self.indeg.setdefault(dst, set()).add(src)
            self.outdeg.setdefault(src, set()).add(dst)
            self.pairs.add((src, dst))

        self.indeg_values = [len(self.indeg.get(p, ())) for p in self.file_paths]
        self.median_indeg = (
            statistics.median(self.indeg_values) if self.indeg_values else 0
        )
        self.nonprod_file_count = sum(1 for p in self.file_paths if self.nonprod[p])
        self.startup_count = sum(
            1 for f in self.funcs if f.get("entry_kind") in ("main", "script")
        )
        self.noloop_count = sum(1 for f in self.funcs if f["max_loop_depth"] == 0)
        self._source_cache = {}

        self.kv1_order = self._kv1_order()
        self.kv2_order = sorted(
            self.file_paths,
            key=lambda p: (self.nonprod[p], -len(self.indeg.get(p, ())), p),
        )
        # prereg section 3 as corrected by docs/specs/map-intelligibility.md:
        # production first, then startup + no-loop demotion score ascending,
        # then cost desc, then path.
        self.kv3_order = sorted(
            self.funcs,
            key=lambda f: (
                self.nonprod.get(f["path"], is_nonprod(f["path"])),
                self.demote_score(f),
                -f["function_cost_v2"],
                f["path"],
                f["name"],
                f["span"]["start_line"],
            ),
        )

    # ---- KV1 -----------------------------------------------------------
    def containing_function(self, finding):
        path = finding["path"]
        line = finding["span"]["start_line"]
        candidates = [
            f
            for f in self.funcs_by_path.get(path, ())
            if f["span"]["start_line"] <= line <= f["span"]["end_line"]
        ]
        if candidates:
            return max(
                candidates,
                key=lambda f: (f["span"]["start_line"], -f["span"]["end_line"], f["id"]),
            )
        symbol = finding.get("symbol")
        if symbol:
            named = [f for f in self.funcs_by_path.get(path, ()) if f["name"] == symbol]
            if named:
                return min(
                    named,
                    key=lambda f: (f["span"]["start_line"], f["span"]["end_line"], f["id"]),
                )
        return None

    def _kv1_order(self):
        enriched = []
        for finding in self.findings:
            container = self.containing_function(finding)
            cost = container["function_cost_v2"] if container else 0
            enriched.append((finding, container, cost))
        enriched.sort(
            key=lambda item: (
                self.nonprod.get(item[0]["path"], is_nonprod(item[0]["path"])),
                SEVERITY_RANK.get(item[0]["severity"], 3),
                CONFIDENCE_RANK.get(item[0]["confidence"], 3),
                -item[2],
                item[0]["finding_id"],
                item[0]["path"],
                item[0]["span"]["start_line"],
            )
        )
        return enriched

    def kv1_inline(self):
        total = len(self.kv1_order)
        rows = []
        for idx, (finding, container, cost) in enumerate(self.kv1_order[:5]):
            anchor = None
            if container is not None:
                parts = [f"{fmt_int(cost)} static cost units (v2)"]
                ratio = ratio_label(cost, self.median_cost)
                if ratio:
                    parts.append(f"{ratio} median")
                parts.append(top_label(idx + 1, total))
                anchor = " \u00b7 ".join(parts)
            rows.append(
                {
                    "rank": idx + 1,
                    "finding": finding,
                    "container": container,
                    "anchor": anchor,
                    "why": first_sentence(finding.get("message")),
                    "nonprod": self.nonprod.get(
                        finding["path"], is_nonprod(finding["path"])
                    ),
                }
            )
        return rows

    # ---- KV2 -----------------------------------------------------------
    def kv2_inline(self):
        production = [p for p in self.file_paths if not self.nonprod[p]]
        production.sort(key=lambda p: (-len(self.indeg.get(p, ())), p))
        rows = production[:8]
        max_deg = max((len(self.indeg.get(p, ())) for p in rows), default=0)
        total = len(self.file_paths)
        result = []
        for p in rows:
            degree = len(self.indeg.get(p, ()))
            parts = [
                f"{degree} dependents",
                f"{int(100.0 * degree / total + 0.5)}% of scanned files",
                top_label(self.kv2_order.index(p) + 1, total),
            ]
            ratio = ratio_label(degree, self.median_indeg)
            if ratio:
                parts.append(f"{ratio} median")
            result.append(
                {
                    "path": p,
                    "degree": degree,
                    "width": (100.0 * degree / max_deg) if max_deg else 0.0,
                    "value": " \u00b7 ".join(parts),
                }
            )
        return result

    def kv2_matrix(self):
        production = [p for p in self.file_paths if not self.nonprod[p]]
        production.sort(key=lambda p: (-len(self.indeg.get(p, ())), p))
        others = [p for p in self.file_paths if self.nonprod[p]]
        others.sort(key=lambda p: (-len(self.indeg.get(p, ())), p))
        columns = (production + others)[:10]
        # Row rule: rows are production includers first. (a) the top 8 production
        # files by distinct resolved out-degree plus (b) every production file
        # that includes one of the top 3 columns. If that union has fewer than
        # 12 rows, fill with the top non-production files by out-degree; those
        # demoted rows carry a "test" chip. Ordering: production (out-degree
        # desc, path asc) then non-production (out-degree desc, path asc).
        degree = lambda p: len(self.outdeg.get(p, ()))
        top_prod = sorted(production, key=lambda p: (-degree(p), p))[:8]
        includers = [
            p
            for p in production
            if any((p, c) in self.pairs for c in columns[:3])
        ]
        production_rows = sorted(
            set(top_prod) | set(includers), key=lambda p: (-degree(p), p)
        )[:12]
        row_paths = [(p, False) for p in production_rows]
        if len(row_paths) < 12:
            for p in sorted(others, key=lambda p: (-degree(p), p)):
                row_paths.append((p, True))
                if len(row_paths) == 12:
                    break
        return {
            "columns": [
                {
                    "path": p,
                    "basename": p.rsplit("/", 1)[-1],
                    "total": len(self.indeg.get(p, ())),
                }
                for p in columns
            ],
            "rows": [
                {
                    "path": p,
                    "demoted": demoted,
                    "out_degree": degree(p),
                    "cells": [1 if (p, c) in self.pairs else 0 for c in columns],
                }
                for p, demoted in row_paths
            ],
        }

    # ---- KV3 -----------------------------------------------------------
    def demote_score(self, func):
        """Startup (main/script) + no-loop demotion score; 0 = never demoted."""
        return (1 if func.get("entry_kind") in ("main", "script") else 0) + (
            1 if func["max_loop_depth"] == 0 else 0
        )

    def _is_nonprod(self, func):
        return self.nonprod.get(func["path"], is_nonprod(func["path"]))

    def kv3_non_demoted(self):
        return [
            f
            for f in self.kv3_order
            if not self._is_nonprod(f) and self.demote_score(f) == 0
        ]

    def demote_badge(self, func):
        reasons = []
        if self._is_nonprod(func):
            reasons.append("non-production")
        if func.get("entry_kind") in ("main", "script"):
            reasons.append("startup")
        if func["max_loop_depth"] == 0:
            reasons.append("no-loop")
        if not reasons:
            return "demoted"
        if set(reasons) == {"startup", "no-loop"}:
            stem = "startup + no-loop"
        else:
            stem = " + ".join(reasons)
        return f"{stem} \u2014 demoted"

    def kv3_selection(self):
        """Single selection shared by kv3-bars and kv3-treemap.

        8 non-demoted rows (production, demote_score == 0) + the first two
        rows with demote_score > 0 in the corrected ranking.
        """
        total = len(self.kv3_order)
        non_demoted = self.kv3_non_demoted()[:8]
        rows = [
            {
                "func": f,
                "rank": idx + 1,
                "anchor": self._cost_anchor(f, idx + 1, total),
                "demoted": None,
            }
            for idx, f in enumerate(non_demoted)
        ]

        demoted = [f for f in self.kv3_order if self.demote_score(f) > 0][:2]
        if len(demoted) < 2:
            seen = {id(f) for f in demoted} | {id(f) for f in non_demoted}
            for f in self.kv3_order:
                if id(f) in seen:
                    continue
                demoted.append(f)
                seen.add(id(f))
                if len(demoted) == 2:
                    break

        for f in demoted:
            rows.append(
                {
                    "func": f,
                    "rank": None,
                    "anchor": self._cost_anchor(
                        f, self.kv3_order.index(f) + 1, total
                    ),
                    "demoted": self.demote_badge(f),
                }
            )
        return rows

    def _cost_anchor(self, func, rank, total):
        parts = [f"{fmt_int(func['function_cost_v2'])} static cost units (v2)"]
        ratio = ratio_label(func["function_cost_v2"], self.median_cost)
        if ratio:
            parts.append(f"{ratio} median")
        parts.append(top_label(rank, total))
        return " \u00b7 ".join(parts)

    # ---- source callout ------------------------------------------------
    def source_lines(self, path):
        if path not in self._source_cache:
            full = TEST_ROOT / self.repo / path
            try:
                text = full.read_text(encoding="utf-8", errors="replace")
            except OSError:
                self._source_cache[path] = None
            else:
                self._source_cache[path] = text.splitlines()
        return self._source_cache[path]


# --------------------------------------------------------------------------
# CSS + page chrome
# --------------------------------------------------------------------------
CSS = """
*{box-sizing:border-box}
html,body{background:#fff;color:#1a202c}
body{
  margin:0;padding:20px;
  font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,"Helvetica Neue",Arial,sans-serif;
  font-size:14px;line-height:1.45;
}
.page{max-width:1152px;margin:0 auto}
.kicker{font-size:11px;letter-spacing:.12em;text-transform:uppercase;color:#4a5568;margin-bottom:6px}
h1{font-size:25px;line-height:1.2;margin:0 0 6px;letter-spacing:-.01em}
.action{margin:0 0 18px;color:#4a5568;font-size:13px}
.caption{margin:10px 0 0;color:#4a5568;font-size:12px}
.mono,pre,.loc,.anchor,.value,.bval,.vval{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,"Liberation Mono",monospace}
.anchor{font-size:11.5px;color:#4a5568;font-variant-numeric:tabular-nums}
.why{font-size:12.5px;color:#2d3748;margin-top:3px}
.badge{display:inline-block;font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:.04em;padding:1px 6px;border-radius:3px;border:1px solid #cbd5e0;color:#4a5568;vertical-align:1px;white-space:nowrap}
.sev-hot{background:#fff5f5;border-color:#feb2b2;color:#9b2c2c}
.sev-warn{background:#fffaf0;border-color:#fbd38d;color:#975a16}
.sev-info{background:#f7fafc;border-color:#cbd5e0;color:#4a5568}
.chip{background:#f0fff4;border:1px solid #9ae6b4;color:#22543d;font-size:10px;font-weight:600;padding:1px 6px;border-radius:10px;white-space:nowrap}
.chip.nonprod{background:#faf5ff;border-color:#d6bcfa;color:#553c9a}
.sym{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:12.5px;color:#2d3748}
.rtitle{font-weight:600}
.rows{border-bottom:1px solid #e2e8f0}
.row{display:grid;grid-template-columns:64px 1fr;gap:14px;padding:11px 0;border-top:1px solid #e2e8f0}
.rank{font-size:28px;font-weight:700;line-height:1;color:#2b6cb0;font-variant-numeric:tabular-nums}
.line1{display:flex;flex-wrap:wrap;align-items:baseline;gap:7px}
.line1 .loc{font-size:12.5px;color:#1a202c;font-weight:600}
.card{border:1px solid #e2e8f0;border-radius:6px;padding:13px 15px;margin-bottom:11px}
.card-head{display:grid;grid-template-columns:64px 1fr;gap:14px;align-items:start}
.code{background:#f7fafc;border-left:3px solid #2b6cb0;border-radius:3px;margin:10px 0 0;padding:7px 10px;font-size:11px;line-height:1.55;overflow-x:auto}
.code .cl{display:block;white-space:pre}
.code .ln{display:inline-block;width:44px;color:#a0aec0;user-select:none;text-align:right;margin-right:12px}
.code .hl{background:#fefcbf}
.foot{margin-top:16px;padding-top:12px;border-top:1px solid #e2e8f0}
.foot .counts{display:flex;flex-wrap:wrap;gap:18px;font-size:12px;color:#2d3748}
.foot .more{display:block;font-size:12.5px;color:#2b6cb0;margin-top:7px;font-weight:600}
.foot .legend{font-size:11.5px;color:#718096;margin-top:5px}
.honesty{margin-top:14px;padding-top:10px;border-top:1px solid #edf2f7;font-size:11px;color:#718096}
/* KV2 horizontal dependents rows */
.brow{display:grid;grid-template-columns:330px 1fr 340px;gap:16px;align-items:center;padding:7px 0;border-top:1px solid #e2e8f0}
.brow:last-child{border-bottom:1px solid #e2e8f0}
.blabel{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:12px;color:#1a202c;overflow-wrap:anywhere}
.btrack{background:#edf2f7;border-radius:2px;height:18px;width:100%}
.bfill{background:#2b6cb0;height:18px;border-radius:2px;min-width:0}
.bval{font-size:11.5px;color:#2d3748;font-variant-numeric:tabular-nums;line-height:1.35}
/* KV2 grid */
table.mx{border-collapse:collapse;width:100%;table-layout:fixed}
table.mx th,table.mx td{border:1px solid #e2e8f0;padding:5px 4px;font-size:10.5px;text-align:center;vertical-align:middle}
table.mx thead th{background:#f7fafc;font-weight:600}
table.mx thead th .base{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:10px;overflow-wrap:anywhere;display:block;line-height:1.25}
table.mx tbody th{text-align:left;font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-weight:500;font-size:10px;overflow-wrap:anywhere;background:#fff}
table.mx tbody th .chip.rowwide{margin-left:5px;font-size:9px;padding:0 4px}
table.mx td.dot{color:#2b6cb0;font-size:14px;line-height:1}
table.mx tfoot th,table.mx tfoot td{background:#f7fafc;font-weight:600;font-variant-numeric:tabular-nums}
table.mx .corner{width:220px;text-align:left;color:#718096;font-weight:400;font-size:9.5px}
.mx-wrap{overflow-x:auto}
/* KV3 rows */
.vrow{display:grid;grid-template-columns:330px 1fr 360px;gap:16px;align-items:center;padding:7px 0;border-top:1px solid #e2e8f0}
.vrow:last-child{border-bottom:1px solid #e2e8f0}
.vlabel .loc{font-size:12px;color:#1a202c}
.vlabel .fn{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace;font-size:11.5px;color:#4a5568}
.vval{font-size:11.5px;color:#2d3748;font-variant-numeric:tabular-nums;line-height:1.35}
.vrow.dim .vlabel,.vrow.dim .vval{opacity:.55}
.vrow.dim .bfill{background:#a0aec0}
.bfill.stub{width:6px}
.vrow.strip{grid-template-columns:330px 1fr}
.tstrip{margin-top:8px}
.dembadge{display:inline-block;font-size:10px;font-weight:700;text-transform:uppercase;letter-spacing:.04em;padding:1px 6px;border-radius:3px;background:#edf2f7;border:1px solid #cbd5e0;color:#4a5568;margin-left:6px}
.tripbadge{display:inline-block;font-size:10px;font-weight:700;padding:1px 6px;border-radius:3px;background:#fffaf0;border:1px solid #fbd38d;color:#975a16;margin-top:3px}
svg.tmap{display:block;width:100%;height:auto;background:#fff}
"""


def page_start(kicker, question, action):
    return (
        "<!DOCTYPE html>\n"
        '<html lang="en"><head><meta charset="utf-8">\n'
        '<meta name="viewport" content="width=device-width, initial-scale=1">\n'
        f"<title>{esc(kicker)} — Perfpal</title>\n"
        f"<style>{CSS}</style></head>\n"
        '<body><div class="page">\n'
        f'<div class="kicker">{esc(kicker)}</div>\n'
        f"<h1>{esc(question)}</h1>\n"
        f'<p class="action">{esc(action)}</p>\n'
    )


def page_end(footer_html):
    return (
        f'{footer_html}\n'
        f'<div class="honesty">{esc(HONESTY)}</div>\n'
        "</div></body></html>\n"
    )


def chips(row):
    finding = row["finding"]
    sev = finding.get("severity", "info")
    conf = finding.get("confidence", "LOW")
    cls = "chip" if not row["nonprod"] else "chip nonprod"
    label = "production" if not row["nonprod"] else "non-production"
    return (
        f'<span class="badge sev-{esc(sev)}">{esc(sev)}</span>'
        f'<span class="badge conf">{esc(conf)}</span>'
        f'<span class="{cls}">{esc(label)}</span>'
    )


def kv1_meta_line(row):
    finding = row["finding"]
    loc = f'{finding["path"]}:{finding["span"]["start_line"]}'
    symbol = finding.get("symbol") or "\u2014"
    return (
        f'<span class="loc mono">{esc(loc)}</span>'
        f'<span class="sym">{esc(symbol)}</span>'
        f'<span class="rtitle">{esc(finding.get("title", ""))}</span>'
        f"{chips(row)}"
    )


def code_callout(model, finding):
    lines = model.source_lines(finding["path"])
    if lines is None:
        return ""
    start = finding["span"]["start_line"]
    if start < 1 or start > len(lines):
        return ""
    lo = max(1, start - 2)
    hi = min(len(lines), start + 2)
    out = []
    for n in range(lo, hi + 1):
        content = lines[n - 1].expandtabs(4)
        cls = "cl hl" if n == start else "cl"
        out.append(f'<span class="{cls}"><span class="ln">{n}</span>{esc(content)}</span>')
    return '<pre class="code">' + "\n".join(out) + "</pre>"


def kv1_footer(model):
    n = len(model.findings)
    return (
        f'<div class="foot">'
        f'<span class="more">Complete ranked list — all {n} findings (production-first) \u2192</span>'
        f'<div class="counts"><span>{model.nonprod_file_count} non-production files demoted</span></div>'
        f"</div>"
    )


def render_kv1_brief(model):
    rows = model.kv1_inline()
    kicker = f"Key view \u00b7 {SLOT_LABEL['kv1']} \u00b7 {model.repo}"
    parts = [page_start(kicker, QUESTIONS["kv1"], ACTIONS["kv1"]), '<section class="rows">']
    for row in rows:
        anchor = (
            f'<div class="anchor">{esc(row["anchor"])}</div>' if row["anchor"] else ""
        )
        parts.append(
            '<div class="row">'
            f'<div class="rank">#{row["rank"]}</div>'
            '<div class="body">'
            f'<div class="line1">{kv1_meta_line(row)}</div>'
            f"{anchor}"
            f'<div class="why">{esc(row["why"])}</div>'
            "</div></div>"
        )
    parts.append("</section>")
    parts.append(page_end(kv1_footer(model)))
    return "".join(parts)


def render_kv1_cards(model):
    rows = model.kv1_inline()
    kicker = f"Key view \u00b7 {SLOT_LABEL['kv1']} \u00b7 {model.repo}"
    parts = [page_start(kicker, QUESTIONS["kv1"], ACTIONS["kv1"])]
    for row in rows:
        anchor = (
            f'<div class="anchor">{esc(row["anchor"])}</div>' if row["anchor"] else ""
        )
        parts.append(
            '<article class="card">'
            '<div class="card-head">'
            f'<div class="rank">#{row["rank"]}</div>'
            "<div>"
            f'<div class="line1">{kv1_meta_line(row)}</div>'
            f"{anchor}"
            f'<div class="why">{esc(row["why"])}</div>'
            "</div></div>"
            f'{code_callout(model, row["finding"])}'
            "</article>"
        )
    parts.append(page_end(kv1_footer(model)))
    return "".join(parts)


def kv2_footer(model):
    return (
        f'<div class="foot">'
        f'<div class="counts"><span>{model.nonprod_file_count} non-production files demoted '
        f'\u00b7 resolved {model.edge_resolved} of {model.edge_total} include edges</span></div>'
        f"</div>"
    )


def render_kv2_bars(model):
    rows = model.kv2_inline()
    kicker = f"Key view \u00b7 {SLOT_LABEL['kv2']} \u00b7 {model.repo}"
    parts = [page_start(kicker, QUESTIONS["kv2"], ACTIONS["kv2"])]
    for row in rows:
        parts.append(
            '<div class="brow">'
            f'<div class="blabel">{esc(row["path"])}</div>'
            f'<div class="btrack"><div class="bfill" style="width:{row["width"]:.3f}%"></div></div>'
            f'<div class="bval">{esc(row["value"])}</div>'
            "</div>"
        )
    parts.append(f'<p class="caption">{esc(CAPTIONS["kv2"])}</p>')
    parts.append(page_end(kv2_footer(model)))
    return "".join(parts)


def render_kv2_matrix(model):
    matrix = model.kv2_matrix()
    kicker = f"Key view \u00b7 {SLOT_LABEL['kv2']} \u00b7 {model.repo}"
    columns = matrix["columns"]
    header = ['<th class="corner">row includes \u2192</th>']
    for col in columns:
        header.append(
            "<th>"
            f'<span class="base">{esc(col["basename"])}</span>'
            "</th>"
        )
    body = []
    for row in matrix["rows"]:
        cells = "".join(
            f'<td class="dot">{"\u25cf" if hit else ""}</td>' for hit in row["cells"]
        )
        body.append(
            f'<tr><th>{esc(row["path"])}'
            + ('<span class="chip nonprod rowwide">test</span>' if row["demoted"] else "")
            + f"</th>{cells}</tr>"
        )
    footer_cells = "".join(f"<td>{col['total']}</td>" for col in columns)
    parts = [
        page_start(kicker, QUESTIONS["kv2"], ACTIONS["kv2"]),
        '<div class="mx-wrap"><table class="mx"><thead><tr>',
        "".join(header),
        "</tr></thead><tbody>",
        "".join(body),
        '</tbody><tfoot><tr><th>dependent files</th>',
        footer_cells,
        "</tr></tfoot></table></div>",
        '<p class="caption">\u25cf = row file includes column file; column total = files that include it (all scanned)</p>',
        '<p class="caption">rows: production includers first; demoted rows carry a test chip</p>',
        f'<p class="caption">{esc(CAPTIONS["kv2"])}</p>',
    ]
    parts.append(page_end(kv2_footer(model)))
    return "".join(parts)


def kv3_badge(func):
    unknown = func.get("unknown_trip_loops") or 0
    if unknown > 0:
        return f'<span class="tripbadge">unknown-trip \u00d7{unknown} — estimate incomplete</span>'
    return ""


def render_kv3_bars(model):
    rows = model.kv3_selection()
    kicker = f"Key view \u00b7 {SLOT_LABEL['kv3']} \u00b7 {model.repo}"
    # Scale bars over the non-demoted rows only; demoted rows get a grey stub.
    max_cost = max(
        (r["func"]["function_cost_v2"] for r in rows if not r["demoted"]), default=0
    )
    parts = [
        page_start(kicker, QUESTIONS["kv3"], ACTIONS["kv3"]),
        '<p class="caption">bars scaled to non-demoted maximum; '
        "demoted rows shown off-scale</p>",
    ]
    for row in rows:
        func = row["func"]
        if row["demoted"]:
            bar = '<div class="bfill stub"></div>'
        else:
            width = (100.0 * func["function_cost_v2"] / max_cost) if max_cost else 0.0
            bar = f'<div class="bfill" style="width:{width:.3f}%"></div>'
        loc = f'{func["path"]}:{func["span"]["start_line"]}'
        dem = (
            f'<span class="dembadge">{esc(row["demoted"])}</span>'
            if row["demoted"]
            else ""
        )
        cls = "vrow dim" if row["demoted"] else "vrow"
        parts.append(
            f'<div class="{cls}">'
            '<div class="vlabel">'
            f'<div class="loc mono">{esc(loc)}</div>'
            f'<div class="fn">{esc(func["name"])}{dem}</div>'
            "</div>"
            f'<div class="btrack">{bar}</div>'
            f'<div class="vval">{esc(row["anchor"])}{kv3_badge(func)}</div>'
            "</div>"
        )
    parts.append(f'<p class="caption">{esc(CAPTIONS["kv3"])}</p>')
    parts.append(page_end(kv3_footer(model)))
    return "".join(parts)


# ---------- treemap ----------
def _worst(row, length):
    total = sum(v for v, _ in row)
    if total <= 0 or length <= 0:
        return float("inf")
    high = max(v for v, _ in row)
    low = min(v for v, _ in row)
    if low <= 0:
        return float("inf")
    return max(
        (length * length * high) / (total * total),
        (total * total) / (length * length * low),
    )


def squarify(items, x, y, w, h):
    """Squarified treemap. items = [(value, payload)] sorted descending."""
    rects = []
    values = list(items)

    def place(row, x, y, w, h, horizontal):
        total = sum(v for v, _ in row)
        if horizontal:
            depth = total / w if w else 0
            cursor = x
            for value, payload in row:
                width = (value / depth) if depth else 0
                rects.append((cursor, y, width, depth, payload))
                cursor += width
            return x, y + depth, w, h - depth
        depth = total / h if h else 0
        cursor = y
        for value, payload in row:
            height = (value / depth) if depth else 0
            rects.append((x, cursor, depth, height, payload))
            cursor += height
        return x + depth, y, w - depth, h

    while values and w > 0.01 and h > 0.01:
        horizontal = w >= h
        length = w if horizontal else h
        row = [values[0]]
        index = 1
        while index < len(values):
            if _worst(row + [values[index]], length) <= _worst(row, length):
                row.append(values[index])
                index += 1
            else:
                break
        x, y, w, h = place(row, x, y, w, h, horizontal)
        values = values[index:]
    return rects


def truncate_label(text, max_chars):
    if max_chars <= 0:
        return ""
    if len(text) <= max_chars:
        return text
    if max_chars <= 1:
        return "\u2026"
    return text[: max_chars - 1] + "\u2026"


def render_kv3_treemap(model):
    rows = model.kv3_selection()
    funcs = [r["func"] for r in rows if not r["demoted"]]
    kicker = f"Key view \u00b7 {SLOT_LABEL['kv3']} \u00b7 {model.repo}"
    width, height = 1120, 420
    raw = [(f["function_cost_v2"], f) for f in funcs]
    raw.sort(key=lambda item: (-item[0], item[1]["path"], item[1]["name"]))
    total = sum(value for value, _ in raw)
    # Squarified treemaps operate on area-normalised values (sum == w*h).
    items = [
        ((value * width * height / total) if total else 0.0, func)
        for value, func in raw
    ]
    rects = squarify(items, 0, 0, width, height)
    svg = [
        f'<svg class="tmap" viewBox="0 0 {width} {height}" role="img" '
        f'aria-label="Function cost shares">'
    ]
    for rx, ry, rw, rh, func in rects:
        svg.append(
            f'<rect x="{rx:.2f}" y="{ry:.2f}" width="{rw:.2f}" height="{rh:.2f}" '
            'fill="#2b6cb0" stroke="#ffffff" stroke-width="1.5"/>'
        )
        name = func["name"]
        cost = f"{func['function_cost_v2']}"
        max_chars = max(1, int((rw - 10) / 6.1))
        if rh >= 32:
            svg.append(
                f'<text x="{rx + 5:.2f}" y="{ry + 15:.2f}" fill="#ffffff" '
                'font-family="ui-monospace,Menlo,Consolas,monospace" font-size="11">'
                f"{esc(truncate_label(name, max_chars))}</text>"
            )
            svg.append(
                f'<text x="{rx + 5:.2f}" y="{ry + 27:.2f}" fill="#e6f0fa" '
                'font-family="ui-monospace,Menlo,Consolas,monospace" font-size="10">'
                f"{esc(truncate_label(cost, max_chars))}</text>"
            )
        elif rh >= 20:
            combined = f"{name} \u00b7 {cost}"
            svg.append(
                f'<text x="{rx + 5:.2f}" y="{ry + rh / 2 + 3:.2f}" fill="#ffffff" '
                'font-family="ui-monospace,Menlo,Consolas,monospace" font-size="9.5">'
                f"{esc(truncate_label(combined, max_chars))}</text>"
            )
        else:
            svg.append(
                f'<text x="{rx + 3:.2f}" y="{ry + rh / 2 + 3:.2f}" fill="#ffffff" '
                'font-family="ui-monospace,Menlo,Consolas,monospace" font-size="9">'
                f"{esc(truncate_label(cost, max(1, int((rw - 6) / 5.4))))}</text>"
            )
    svg.append("</svg>")

    strip = []
    for row in rows:
        if not row["demoted"]:
            continue
        func = row["func"]
        loc = f'{func["path"]}:{func["span"]["start_line"]}'
        strip.append(
            '<div class="vrow dim strip">'
            '<div class="vlabel">'
            f'<div class="loc mono">{esc(loc)}</div>'
            f'<div class="fn">{esc(func["name"])}'
            f'<span class="dembadge">{esc(row["demoted"])}</span></div>'
            "</div>"
            f'<div class="vval">{esc(row["anchor"])}{kv3_badge(func)}</div>'
            "</div>"
        )

    parts = [
        page_start(kicker, QUESTIONS["kv3"], ACTIONS["kv3"]),
        "".join(svg),
        '<p class="caption">treemap scaled to non-demoted maximum; '
        "demoted rows shown off-scale</p>",
        f'<div class="tstrip">{"".join(strip)}</div>',
        f'<p class="caption">{esc(CAPTIONS["kv3"])}</p>',
        page_end(kv3_footer(model)),
    ]
    return "".join(parts)


def kv3_footer(model):
    return (
        f'<div class="foot">'
        f'<div class="counts">'
        f'<span>{model.startup_count} startup functions demoted</span>'
        f'<span>{model.noloop_count} no-loop functions demoted</span>'
        f'<span>{model.nonprod_file_count} non-production files demoted</span>'
        f"</div>"
        f'<span class="more">Complete cost-ranked list ({len(model.funcs)} functions) \u2192</span>'
        f'<div class="legend">known-literal-trip flag: V4 field (not computable in this prototype)</div>'
        f"</div>"
    )


RENDERERS = {
    ("kv1", "brief"): render_kv1_brief,
    ("kv1", "cards"): render_kv1_cards,
    ("kv2", "bars"): render_kv2_bars,
    ("kv2", "matrix"): render_kv2_matrix,
    ("kv3", "bars"): render_kv3_bars,
    ("kv3", "treemap"): render_kv3_treemap,
}


# --------------------------------------------------------------------------
# main
# --------------------------------------------------------------------------
def main():
    PROTO_DIR.mkdir(parents=True, exist_ok=True)
    V3_DIR.mkdir(parents=True, exist_ok=True)

    models = {}
    manifest = {
        "generated": datetime.datetime.now(datetime.timezone.utc)
        .isoformat(timespec="seconds")
        .replace("+00:00", "Z"),
        "data": {},
        "renders": [],
        "holdout": {"django": "not-rendered", "postgres": "not-rendered"},
    }

    for repo in REPOS:
        data, digest = load_repo(repo)
        model = RepoModel(repo, data, digest)
        models[repo] = model
        manifest["data"][repo] = {
            "path": str(TEST_ROOT / f"out-i5b-{repo}" / "metrics.json"),
            "sha256": digest,
            "schema_version": str(data["schema_version"]),
            "files": len(data["files"]),
            "findings": len(data["findings"]),
            "functions": len(data["stats"]["functions"]),
        }

        out_dir = PROTO_DIR / repo
        out_dir.mkdir(parents=True, exist_ok=True)
        for slot, idiom in SLOT_IDIOMS:
            page = RENDERERS[(slot, idiom)](model)
            html_path = out_dir / f"{slot}-{idiom}.html"
            html_path.write_text(page, encoding="utf-8")
            manifest["renders"].append(
                {
                    "repo": repo,
                    "slot": slot,
                    "idiom": idiom,
                    "html": str(html_path),
                    "png": str(V3_DIR / repo / f"{slot}-{idiom}.png"),
                    "png_dims": None,
                }
            )

    manifest_path = V3_DIR / "manifest.json"
    with open(manifest_path, "w", encoding="utf-8") as handle:
        json.dump(manifest, handle, indent=2, sort_keys=True)
        handle.write("\n")

    print(f"wrote {len(manifest['renders'])} pages to {PROTO_DIR}")
    print(f"wrote {manifest_path}")
    for repo in REPOS:
        m = models[repo]
        print(
            f"{repo}: files={len(m.file_paths)} findings={len(m.findings)} "
            f"functions={len(m.funcs)} median_cost={m.median_cost} "
            f"p95_cost={m.p95_cost} median_indeg={m.median_indeg} "
            f"edges={m.edge_resolved}/{m.edge_total} "
            f"nonprod_files={m.nonprod_file_count} startup={m.startup_count} "
            f"noloop={m.noloop_count}"
        )


if __name__ == "__main__":
    main()
