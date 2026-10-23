#!/usr/bin/env bash
# Regenerate docs/evidence/atlas-sim/GATE.txt from the current worktree.
#
# Every line is produced by a command in this script; nothing is hand-edited.
# Usage: bash sim/gate.sh
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EVIDENCE="$ROOT/docs/evidence/atlas-sim"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
FIXTURES="${FIXTURES:-/home/shafin/Opencode/perfpal-test}"
PORT="${SIM_PORT:-8770}"
OUT="$ROOT/sim/source"

cd "$ROOT"
export PATH="$HOME/.cargo/bin:$PATH"
{
  echo "# Atlas Story — gate transcript"
  echo
  echo "Date: $(date -I)"
  echo "Worktree: $ROOT"
  echo "Branch: $(git rev-parse --abbrev-ref HEAD)"
  echo "Commit: $(git rev-parse --short HEAD) ($(git log -1 --format=%s))"
  echo

  echo "## 1. Format, lint, tests"
  echo '$ cargo fmt --all -- --check'
  "$CARGO" fmt --all -- --check 2>&1; echo "exit=$?"
  echo
  echo '$ cargo clippy --locked --all-targets -- -D warnings'
  "$CARGO" clippy --locked --all-targets -- -D warnings 2>&1 | tail -3; echo "exit=${PIPESTATUS[0]}"
  echo
  echo '$ cargo test --locked'
  "$CARGO" test --locked 2>&1 | grep -E "^test result|^running" ; echo "exit=${PIPESTATUS[0]}"
  echo
  echo '$ cargo deny check licenses'
  if command -v "$HOME/.cargo/bin/cargo-deny" >/dev/null 2>&1; then
    "$HOME/.cargo/bin/cargo-deny" check licenses 2>&1 | tail -5; echo "exit=${PIPESTATUS[0]}"
  elif "$CARGO" deny --version >/dev/null 2>&1; then
    "$CARGO" deny check licenses 2>&1 | tail -5; echo "exit=${PIPESTATUS[0]}"
  else
    echo "skipped: cargo-deny not installed"
  fi
  echo

  echo "## 2. Additive proof — scan without and with --story"
  rm -rf /var/tmp/opencode/gate-off /var/tmp/opencode/gate-on
  for case in hyperfine cJSON; do
    echo "\$ perfpal $case  (story off)"
    "$ROOT/target/debug/perfpal" "$FIXTURES/$case" -o "/var/tmp/opencode/gate-off/$case" >/dev/null 2>&1
    echo "exit=$?"
    echo "\$ perfpal $case --story"
    "$ROOT/target/debug/perfpal" "$FIXTURES/$case" -o "/var/tmp/opencode/gate-on/$case" --story >/dev/null 2>&1
    echo "exit=$?"
    echo "report.html:"
    sha256sum "/var/tmp/opencode/gate-off/$case/report.html" "/var/tmp/opencode/gate-on/$case/report.html"
    echo "metrics.json (story off vs on; only scan provenance may differ):"
    python3 - "$case" <<'PY'
import json, sys
case = sys.argv[1]
a = json.load(open(f"/var/tmp/opencode/gate-off/{case}/metrics.json"))
b = json.load(open(f"/var/tmp/opencode/gate-on/{case}/metrics.json"))
for doc in (a, b):
    doc.get("scan", {}).pop("duration_ms", None)
    doc.get("scan", {}).pop("started_at", None)
print("content_equal:", a == b)
PY
    echo "atlas-layout.json:"
    ls -l "/var/tmp/opencode/gate-on/$case/atlas-layout.json"
    python3 - "$case" <<'PY'
import json, sys
case = sys.argv[1]
d = json.load(open(f"/var/tmp/opencode/gate-on/{case}/atlas-layout.json"))
print("canvas", d["canvas"], "nodes", len(d["nodes"]), "edges", len(d["edges"]),
      "stages", len(d["stages"]), "findings", f'{d["meta"]["findings_included"]}/{d["meta"]["findings_total"]}')
print("columns", [(c["depth"], c["x"], c["label"], c["count_text"]) for c in d["columns"]][:4])
PY
    echo
  done

  echo "## 3. Story data from core artifacts"
  echo '$ bash sim/make-source.sh'
  bash sim/make-source.sh 2>&1 | tail -4; echo "exit=${PIPESTATUS[0]}"
  echo
  echo '$ python3 sim/from-layout.py (twice; hashes must match)'
  python3 sim/from-layout.py 2>&1 | tail -2
  sha256sum sim/data-hyperfine.json sim/data-cjson.json > /var/tmp/opencode/gate-data.sha
  python3 sim/from-layout.py >/dev/null 2>&1
  sha256sum -c /var/tmp/opencode/gate-data.sha 2>&1; echo "exit=$?"
  echo
  echo "provenance (data file source.sha256 vs the layout artifact it read):"
  python3 - <<'PY'
import hashlib, json
for case, out in (("hyperfine", "out-hyperfine"), ("cjson", "out-cjson")):
    artifact = open(f"sim/source/{out}/atlas-layout.json", "rb").read()
    data = json.load(open(f"sim/data-{case}.json"))
    print(case, data["source"]["sha256"] == hashlib.sha256(artifact).hexdigest(),
          data["source"]["sha256"][:12])
PY
  echo

  echo "## 4. Story gate"
  echo "\$ SIM_PORT=$PORT python3 sim/verify.py"
  SIM_PORT="$PORT" python3 sim/verify.py 2>&1 | tail -3; echo "exit=${PIPESTATUS[0]}"
  echo
  echo "## 5. No production scope leak"
  echo '$ git status --porcelain src/ docs/schema/ Cargo.toml Cargo.lock'
  git status --porcelain src/ docs/schema/ Cargo.toml Cargo.lock
  echo "lines=$(git status --porcelain src/ docs/schema/ Cargo.toml Cargo.lock | wc -l)"
} > "$EVIDENCE/GATE.txt" 2>&1

echo "wrote $EVIDENCE/GATE.txt"
