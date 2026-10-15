#!/usr/bin/env bash
# run-ablation.sh — ablation harness, Phase 1 (plan `divergence-revamp`).
#
# For each task: materialize the fixture to a tempdir OUTSIDE the repo, scan it
# with a frozen perfpal binary, emit WITH and WITHOUT prompt packets, and score
# a fixed tree per docs/evidence/benchmark-suite.md §3 (TP = rule_id+path+
# start_line match; success = target gone + no new scoped FP + parses).
#
# Modes:
#   --mode dry-run   CI-safe, no model/network: materialize + scan + emit both
#                    packets, assert packet integrity, execute the scoring code
#                    path on the UNFIXED tree (expected: target-present).
#   --mode manual    Emit packets for a human/model run; prints the workdir and
#                    the follow-up --mode score invocation. No model is called.
#   --mode score     Rescan --workdir <dir> (a hand/model-fixed tree) and score
#                    it against the task target. Appends one JSONL row per task.
#
# Writes only to tempdirs and the results file. Never touches the repo fixtures.
# Frozen binary: $PERFPAL_BIN, else the cargo toolchain ($CARGO, then
# $HOME/.cargo/bin/cargo, then PATH cargo) running
# `cargo run --locked --` at the current commit. The $PERFPAL_BIN path needs
# NO cargo present. Results accumulate: every invocation appends/merges one
# JSONL row per task+condition (upsert on re-run of the same task+condition);
# pass --fresh to truncate the results file first (fresh campaign start).
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TASKS="$REPO/docs/evidence/ablation-tasks.json"
LABELS="$REPO/tests/fixtures/precision/labels.json"
CORPUS="$REPO/tests/fixtures/precision"
MODE="dry-run"
OUT=""
WORKDIR=""
ONLY=""
FRESH="0"

usage() {
  cat <<'EOF'
Usage: scripts/run-ablation.sh [--mode dry-run|manual|score] [--task ABL-001[,ABL-002...]]
       [--out DIR] [--workdir DIR] [--tasks FILE] [--binary PATH] [--fresh]

  dry-run: packets + integrity asserts + scoring path on unfixed tree (CI-safe).
  manual:  emit packets for hand/model fixing; prints the score follow-up.
  score:   rescan --workdir (fixed tree, e.g. from a manual run) and score it.
  --fresh: truncate the results file before running (fresh campaign start).
           Default appends/merges: one row per task+condition, upserting any
           prior row with the same task_id+condition (separately-scored tasks
           never discard each other's rows).
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode) MODE="$2"; shift 2 ;;
    --task) ONLY="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    --workdir) WORKDIR="$2"; shift 2 ;;
    --tasks) TASKS="$2"; shift 2 ;;
    --binary) PERFPAL_BIN="$2"; shift 2 ;;
    --fresh) FRESH="1"; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown arg: $1" >&2; usage; exit 2 ;;
  esac
done

case "$MODE" in dry-run|manual|score) ;; *) echo "bad --mode: $MODE" >&2; exit 2 ;; esac
if [[ "$MODE" == "score" && -z "$WORKDIR" ]]; then echo "--mode score needs --workdir" >&2; exit 2; fi
if [[ -z "$OUT" ]]; then OUT="$(mktemp -d "${TMPDIR:-/tmp}/perfpal-ablation-XXXXXX")"; else mkdir -p "$OUT"; fi

resolve_cargo() { # echo a usable cargo path, or fail
  if [[ -n "${CARGO:-}" ]]; then echo "$CARGO"; return 0; fi
  if [[ -x "$HOME/.cargo/bin/cargo" ]]; then echo "$HOME/.cargo/bin/cargo"; return 0; fi
  if command -v cargo >/dev/null 2>&1; then echo "cargo"; return 0; fi
  return 1
}

if [[ -n "${PERFPAL_BIN:-}" ]]; then
  # shellcheck disable=SC2206
  SCAN=( "$PERFPAL_BIN" )
elif CARGO_BIN="$(resolve_cargo)"; then
  # shellcheck disable=SC2206
  SCAN=( "$CARGO_BIN" run --locked --manifest-path "$REPO/Cargo.toml" -- )
else
  echo "error: no Rust toolchain found (honored \$CARGO, then \$HOME/.cargo/bin/cargo, then PATH cargo) and \$PERFPAL_BIN is unset." >&2
  echo "Set --binary PATH to a frozen perfpal binary (works with no cargo present) or install the toolchain." >&2
  exit 2
fi

task_ids() {
  if [[ -n "$ONLY" ]]; then tr ',' ' ' <<<"$ONLY";
  else python3 -c "import json;print(' '.join(t['task_id'] for t in json.load(open('$TASKS'))['tasks']))"; fi
}

task_field() { # <id> <field>
  python3 -c "import json;ts=json.load(open('$TASKS'))['tasks'];print(next(t['$2'] for t in ts if t['task_id']=='$1'))"
}

# Emit WITH + WITHOUT packets for one task. Sets globals PACK_WITH / PACK_WITHOUT.
emit_packets() { # <task_id> <scan_outdir> <packet_dir> <source_file>
  local id="$1" scan="$2" pkt="$3" src="$4"
  local rule path line prompt
  rule="$(task_field "$id" rule_id)"; path="$(task_field "$id" path)"
  line="$(task_field "$id" line)"; prompt="$(task_field "$id" rule_prompt)"
  mkdir -p "$pkt"
  python3 - "$scan" "$pkt" "$rule" "$path" "$line" <<'EOF'
import json, os, sys
scan, pkt, rule, path, line = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5])
doc = json.load(open(f"{scan}/metrics.json"))
subset = [f for f in doc.get("findings", [])
          if f.get("rule_id") == rule and f.get("span", {}).get("start_line") == line]
json.dump(subset, open(f"{pkt}/with_findings.json", "w"), indent=2, sort_keys=True)
report = open(f"{scan}/report.md").read()
sec4, sec5 = "", ""
if "## 4. Findings" in report:
    sec4 = report.split("## 4. Findings", 1)[1]
    if "## 5. Architecture" in sec4:
        sec5 = sec4.split("## 5. Architecture", 1)[1]
        sec4 = sec4.split("## 5. Architecture", 1)[0]
        for marker in ("\n## ", "\n# "):
            if marker in sec5:
                sec5 = sec5.split(marker, 1)[0]
target = [b for b in sec4.split("#### ") if rule.replace("-", "\\-") in b or rule in b]
# §5 excerpt, TARGET FILE ONLY (design §4): keep table structure rows plus
# lines naming the target file (full rel path or basename) or its parent dir
# as a pipe cell (component-table rows carry the dir, not the file). Chart
# index links are dropped — the full arch-map already ships as with_graph.dot.
base = os.path.basename(path)
parent = os.path.dirname(path)
kept = []
for ln in sec5.strip().splitlines():
    s = ln.strip()
    if s.startswith("[") and "](charts/" in s:
        continue  # arch-map link; graph.dot covers it
    if "Component detail pages" in s:
        continue  # its links are all dropped above
    if s.startswith("|") and ("component" in s.lower() or set(s) <= set("|-: ")):
        kept.append(ln)  # ranking-table header/separator
        continue
    if path in ln or (base and base in ln):
        kept.append(ln)
        continue
    if parent and (f"| {parent} |" in ln or f"| {parent} " in ln or f" {parent} |" in ln):
        kept.append(ln)
if not kept:
    kept = [f"(no target-file block in §5 for {path})"]
open(f"{pkt}/with_report_excerpt.md", "w").write(
    "## 4. Findings (target-rule blocks)\n\n#### " + "\n#### ".join(target)
    + "\n\n## 5. Architecture (excerpt — target file only)\n" + "\n".join(kept) + "\n")
with open(f"{scan}/charts/graph.dot") as fh:
    open(f"{pkt}/with_graph.dot", "w").write(fh.read())
EOF
  PACK_WITH="$pkt/WITH.txt"
  {
    echo "CONDITION: WITH (perfpal artifacts for $rule at $path:$line)"
    echo "--- findings subset (rule/message/span/evidence) ---"
    cat "$pkt/with_findings.json"
    echo; echo "--- charts/graph.dot ---"
    cat "$pkt/with_graph.dot"
    echo; echo "--- report.md §4 target blocks + §5 excerpt ---"
    cat "$pkt/with_report_excerpt.md"
  } > "$PACK_WITH"
  PACK_WITHOUT="$pkt/WITHOUT.txt"
  {
    echo "CONDITION: WITHOUT (bare source + one-line rule prompt)"
    echo "RULE: $prompt"
    echo "(No location hint beyond what the compiler gives. Same rule name, same attempt budget.)"
    echo "--- source: $path ---"
    cat "$src"
  } > "$PACK_WITHOUT"
  echo "$rule $path $line" > "$pkt/target.txt"
}

# Score a scanned tree. Prints "target_gone|new_scoped_fp|parse_ok|total|non_vacuous".
# non_vacuous is the anti-degenerate guard (design §5 criterion 4): the
# target file must still be present in files[] (FileEntry.path per
# src/schema/mod.rs, with the same path||file||span.file fallback as
# findings), still parse ok, AND still define ≥1 function — so deleting or
# emptying the work tree (0 findings, parses trivially) scores failure, never
# success.
score_tree() { # <task_id> <scan_outdir>
  local id="$1" scan="$2"
  local rule path line
  rule="$(task_field "$id" rule_id)"; path="$(task_field "$id" path)"; line="$(task_field "$id" line)"
  python3 - "$scan" "$LABELS" "$rule" "$path" "$line" <<'EOF'
import json, sys
scan, labels, rule, path, line = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], int(sys.argv[5])
doc = json.load(open(f"{scan}/metrics.json"))
findings = doc.get("findings", [])
def fpath(f):
    return f.get("path", "") or f.get("file", "") or f.get("span", {}).get("file", "") or ""
def same_file(p, q):
    return (q == p or q.endswith("/" + p) or p.endswith("/" + q)
            or (q and p and q.split("/")[-1] == p.split("/")[-1] and q.split("/")[-1] != ""))
def match_target(f):
    return (f.get("rule_id") == rule and f.get("span", {}).get("start_line") == line
            and same_file(fpath(f), path))
target_gone = not any(match_target(f) for f in findings)
fires = {(c["rule_id"], c["path"], c.get("line")) for c in json.load(open(labels))["cases"] if c.get("expect") == "fire"}
def match_any_fire(f):
    sl = f.get("span", {}).get("start_line")
    p = fpath(f)
    base = p.split("/")[-1]
    return any(f.get("rule_id") == r and (q == p or q.endswith("/" + p) or p.endswith("/" + q) or q.split("/")[-1] == base) and s == sl for (r, q, s) in fires)
scoped_rules = {c["rule_id"] for c in json.load(open(labels))["cases"]}
new_fp = sum(1 for f in findings if f.get("rule_id") in scoped_rules and not match_any_fire(f))
files = doc.get("files", [])
def epath(e):
    return e.get("path", "") or e.get("file", "") or ""
statuses = [e.get("parse_health", {}).get("status", "ok") for e in files]
parse_ok = all(s == "ok" for s in statuses) if files else True
# Non-vacuity guard: target file present + parses ok + still defines ≥1
# function (FileEntry.function_count; falls back to stats.functions when the
# key is absent). Missing/empty/zero-function trees are vacuous by construction.
tentries = [e for e in files if same_file(epath(e), path)]
funcs = doc.get("stats", {}).get("functions", [])
if tentries:
    t = tentries[0]
    file_ok = t.get("parse_health", {}).get("status", "ok") == "ok"
    if "function_count" in t:
        file_funcs = t["function_count"]
    else:
        file_funcs = sum(1 for fn in funcs if same_file(fn.get("path", ""), path))
    non_vacuous = file_ok and file_funcs >= 1
else:
    non_vacuous = False
print(f"{str(target_gone).lower()}|{new_fp}|{str(parse_ok).lower()}|{len(findings)}|{str(non_vacuous).lower()}")
EOF
}

RESULTS="$OUT/results.jsonl"
if [[ "$FRESH" == "1" ]]; then
  : > "$RESULTS"
else
  touch "$RESULTS"
fi

# Append/merge one result row: upsert on (task_id, condition) so re-running
# one task never discards other tasks' rows (paired-campaign requirement:
# 52 rows accumulate across separate invocations). Keys sorted for determinism.
record_row() { # <json-object>
  python3 - "$RESULTS" "$1" <<'EOF'
import json, sys
path, row = sys.argv[1], sys.argv[2]
rows = []
try:
    with open(path) as fh:
        for ln in fh:
            ln = ln.strip()
            if ln:
                rows.append(json.loads(ln))
except FileNotFoundError:
    pass
new = json.loads(row)
rows = [r for r in rows
        if not (r.get("task_id") == new.get("task_id")
                and r.get("condition") == new.get("condition"))]
rows.append(new)
with open(path, "w") as fh:
    for r in rows:
        fh.write(json.dumps(r, sort_keys=True) + "\n")
EOF
}

for id in $(task_ids); do
  rule="$(task_field "$id" rule_id)"; rel="$(task_field "$id" path)"; line="$(task_field "$id" line)"
  echo "=== $id $rule $rel:$line [$MODE] ==="
  if [[ "$MODE" == "score" ]]; then
    tree="$WORKDIR"
  else
    tree="$(mktemp -d "${TMPDIR:-/tmp}/perfpal-abl-task-XXXXXX")"
    mkdir -p "$tree/$(dirname "$rel")"
    cp "$CORPUS/$rel" "$tree/$rel"
  fi
  scan="$OUT/scans/$id"; rm -rf "$scan"
  "${SCAN[@]}" "$tree" -o "$scan" >/dev/null
  pkt="$OUT/packets/$id"
  srcfile="$tree/$rel"
  if [[ ! -f "$srcfile" ]]; then
    # Manual workdirs may hold the file by basename (single-file tasks).
    srcfile="$(find "$tree" -name "$(basename "$rel")" -type f | head -1)"
  fi
  [[ -n "${srcfile:-}" && -f "$srcfile" ]] || { echo "FAIL: source for $rel not found under $tree" >&2; exit 1; }
  if [[ "$MODE" == "score" && ! -s "$srcfile" ]]; then
    echo "FAIL: source for $rel is empty under $tree (degenerate tree — see design §5 criterion 4)" >&2; exit 1
  fi
  emit_packets "$id" "$scan" "$pkt" "$srcfile"

  # Packet integrity asserts (all modes).
  [[ -s "$PACK_WITH" ]] || { echo "FAIL: empty WITH packet $id" >&2; exit 1; }
  [[ -s "$PACK_WITHOUT" ]] || { echo "FAIL: empty WITHOUT packet $id" >&2; exit 1; }
  grep -q "$rule" "$PACK_WITH" || { echo "FAIL: WITH packet lacks $rule" >&2; exit 1; }
  grep -q "$rule" "$PACK_WITHOUT" || { echo "FAIL: WITHOUT packet lacks rule name" >&2; exit 1; }
  grep -q "digraph perfpal" "$pkt/with_graph.dot" || { echo "FAIL: graph.dot missing" >&2; exit 1; }
  python3 -c "import json;assert isinstance(json.load(open('$pkt/with_findings.json')),list)"
  echo "packets OK: $pkt"

  scored="$(score_tree "$id" "$scan")"
  target_gone="${scored%%|*}"; rest="${scored#*|}"; new_fp="${rest%%|*}"; rest="${rest#*|}"
  parse_ok="${rest%%|*}"; rest="${rest#*|}"; total="${rest%%|*}"; non_vacuous="${rest##*|}"

  if [[ "$MODE" == "dry-run" ]]; then
    # Unfixed tree: target MUST still be present (scoring path executes, negative control).
    [[ "$target_gone" == "false" ]] || { echo "FAIL: target already gone pre-fix ($id) — fixture drift?" >&2; exit 1; }
    # Unfixed tree MUST be non-vacuous (guard sanity: the fixture parses with code in it).
    [[ "$non_vacuous" == "true" ]] || { echo "FAIL: unfixed tree vacuous ($id) — fixture drift?" >&2; exit 1; }
    echo "dry-run OK: target present (total=$total), scoring path executes (new_fp=$new_fp, parse_ok=$parse_ok, non_vacuous=$non_vacuous)"
    record_row "{\"task_id\":\"$id\",\"condition\":\"dry-run\",\"attempts\":0,\"outcome\":\"dry-run-ok\",\"target_gone\":false,\"new_scoped_fp\":$new_fp,\"parse_ok\":$([ "$parse_ok" = true ] && echo true || echo false),\"non_vacuous\":$([ "$non_vacuous" = true ] && echo true || echo false),\"rescan_findings\":$total}"
  elif [[ "$MODE" == "manual" ]]; then
    echo "WITH packet:    $PACK_WITH"
    echo "WITHOUT packet: $PACK_WITHOUT"
    echo "Work tree (edit a COPY here, never the repo): $tree"
    echo "When fixed, score with:"
    echo "  scripts/run-ablation.sh --mode score --workdir $tree --task $id --out $OUT"
    record_row "{\"task_id\":\"$id\",\"condition\":\"manual-pending\",\"attempts\":0,\"outcome\":\"manual-pending\",\"workdir\":\"$tree\"}"
  else
    # Success = target gone + no new scoped FP + parses + NON-VACUOUS (an
    # emptied/deleted tree scores failure via the guard, never success).
    if [[ "$target_gone" == "true" && "$new_fp" == "0" && "$parse_ok" == "true" && "$non_vacuous" == "true" ]]; then outcome="success"; else outcome="failure"; fi
    echo "score: target_gone=$target_gone new_scoped_fp=$new_fp parse_ok=$parse_ok total=$total non_vacuous=$non_vacuous -> $outcome"
    record_row "{\"task_id\":\"$id\",\"condition\":\"manual\",\"attempts\":1,\"outcome\":\"$outcome\",\"target_gone\":$([ "$target_gone" = true ] && echo true || echo false),\"new_scoped_fp\":$new_fp,\"parse_ok\":$([ "$parse_ok" = true ] && echo true || echo false),\"non_vacuous\":$([ "$non_vacuous" = true ] && echo true || echo false),\"rescan_findings\":$total}"
  fi
done

echo "results: $RESULTS"
