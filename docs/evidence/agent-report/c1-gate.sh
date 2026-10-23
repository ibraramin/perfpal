#!/usr/bin/env bash
# Track C — C1 contract gate. Evidence-only tooling (spec §2 D8); never shipped.
# Usage: bash docs/evidence/agent-report/c1-gate.sh   (exit 0 = gate green)
# The transcript committed as c1-gate.txt is this script's own stdout.
set -u
cd "$(git rev-parse --show-toplevel)"

fail=0
tmp=$(mktemp -d "${TMPDIR:-/var/tmp}/c1-gate.XXXXXX")
say() { printf '%s\n' "$*"; }
check() { # check <label> <cmd...>
  local label=$1; shift
  if "$@" >/dev/null 2>&1; then say "ok   $label"; else say "FAIL $label"; fail=1; fi
}

say "Perfpal Track C — C1 contract gate"
say "run: $(date -u +%Y-%m-%dT%H:%M:%SZ) · branch $(git branch --show-current) · HEAD $(git rev-parse --short HEAD)"
say

say "== 0. branch state =="
say "branch=$(git branch --show-current) head=$(git rev-parse --short HEAD) main=$(git rev-parse --short main)"
say "uncommitted (expected: docs/state only):"
git status --short | sed 's/^/  /'
say

say "== 1. deliverables exist =="
for f in \
  docs/specs/agent-report.md \
  docs/plans/agent-report.md \
  state/tracks/agent-report/handoff.md \
  docs/evidence/agent-report/README.md \
  docs/evidence/agent-report/sample/agent-report.sample.md \
  docs/evidence/agent-report/sample/agent-actions.sample.json \
  docs/evidence/agent-report/sample/derive_sample.py; do
  check "$f" test -f "$f"
done
say

say "== 2. sample reproduces byte-identically =="
if python3 docs/evidence/agent-report/sample/derive_sample.py \
  tests/golden/report-mixed/metrics.json "$tmp" docs/specs/agent-report.md; then
  say "ok   derive_sample.py"
else
  say "FAIL derive_sample.py"; fail=1
fi
for f in agent-report.sample.md agent-actions.sample.json; do
  if diff -q "$tmp/$f" "docs/evidence/agent-report/sample/$f" >/dev/null 2>&1; then
    say "ok   $f identical"
  else
    say "FAIL $f differs"; diff -u "$tmp/$f" "docs/evidence/agent-report/sample/$f" | head -20; fail=1
  fi
done
say

say "== 3. JSON contract invariants =="
if jq -e '
  (.artifact == "agent-actions")
  and (.contract_version == 1)
  and ((.actions | length) == .complete_action_count)
  and ((.profile_queue | length) == ([.profile_queue_cap, .complete_profile_count] | min))
  and ([.actions[].rank] == [range(1; (.complete_action_count + 1))])
  and (([.actions[] | select(.fix_hint == null)] | length) == 0)
  and (([.actions[] | select(.function == null)] | length) == 0)
  and (([.profile_queue[] | select(.function_name == null or (.function_name | type) != "string")] | length) == 0)
  and (.scan | has("started_at") | not)
  and (.scan | has("duration_ms") | not)
' docs/evidence/agent-report/sample/agent-actions.sample.json >/dev/null; then
  say "ok   jq -e fixture invariants (counts, ranks, non-null hints/functions here, profile schema, no clock)"
else
  say "FAIL jq fixture invariants"; fail=1
fi
say

say "== 4. hint table: 67 rows + set equality with HEURISTIC_CATALOG_V1 =="
if python3 - <<'PY'
import re, sys
from pathlib import Path

spec_ids = [
    line.split("|")[1].strip()
    for line in Path("docs/specs/agent-report.md").read_text().splitlines()
    if line.startswith("| PERF-")
]
src = Path("src/reporter/mod.rs").read_text()
start = src.index("const HEURISTIC_CATALOG_V1")
block = src[start:src.index("\n];", start)]
catalog = re.findall(r'^\s*"(PERF-[A-Z0-9-]+)",\s*$', block, re.M)
print(f"spec rows={len(spec_ids)} catalog ids={len(catalog)}")
if len(spec_ids) != 67 or len(set(spec_ids)) != 67:
    print("FAIL spec hint rows are not 67 unique ids"); sys.exit(1)
if set(spec_ids) != set(catalog):
    missing = sorted(set(catalog) - set(spec_ids))
    extra = sorted(set(spec_ids) - set(catalog))
    print(f"FAIL hint set != catalog set; missing={missing} extra={extra}"); sys.exit(1)
print("ok   hint id set == catalog id set")
PY
then :; else fail=1; fi
say

say "== 5. spec worked example == committed sample action 1 =="
if python3 - <<'PY'
import json, re, sys
from pathlib import Path
spec = Path("docs/specs/agent-report.md").read_text()
blocks = re.findall(r"```json\n(.*?)\n```", spec, re.S)
ex = json.loads(blocks[0])
act = json.loads(Path("docs/evidence/agent-report/sample/agent-actions.sample.json").read_text())["actions"][0]
if list(ex) != list(act) or ex != act:
    print("FAIL spec example != sample action 1"); sys.exit(1)
print("ok   example == sample action 1")
PY
then :; else fail=1; fi
say

say "== 6. fixed MD skeleton headings (exact, in order) =="
if python3 - <<'PY'
import re, sys
from pathlib import Path
heads = [
    "# Perfpal Agent Report",
    "## How to use this report",
    "## Data quality",
    "## Start here — ranked actions",
    "## Action index by rule",
    "## Profile queue",
    "## What this scan cannot see",
]
lines = Path("docs/evidence/agent-report/sample/agent-report.sample.md").read_text().splitlines()
found = [l for l in lines if l.startswith("# ") or l.startswith("## ")]
if found != heads:
    print("FAIL headings/order differ"); print(found); sys.exit(1)
rogue = [l for l in lines if l.startswith("### ") and not re.match(r"^### \d+\. ", l)]
if rogue:
    print("FAIL rogue ### heading(s):", rogue[:3]); sys.exit(1)
print("ok   7 headings, exact order, no extras; every ### is a ranked action block")
PY
then :; else fail=1; fi
say

say "== 7. no production code / schema / dependency changes =="
if [ -z "$(git status --short src/ Cargo.toml Cargo.lock deny.toml docs/schema/)" ]; then
  say "ok   clean"
else
  say "FAIL dirty"; git status --short src/ Cargo.toml Cargo.lock deny.toml docs/schema/; fail=1
fi
say

say "== 8. sample size assertion (64 KB hard budget) =="
bytes=$(wc -c < docs/evidence/agent-report/sample/agent-report.sample.md)
if [ "$bytes" -le 65536 ]; then say "ok   $bytes <= 65536"; else say "FAIL $bytes > 65536"; fail=1; fi
say

say "GATE_EXIT=$fail"
exit "$fail"
