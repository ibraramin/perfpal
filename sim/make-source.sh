#!/usr/bin/env bash
# Regenerate the Atlas sim's source renders from THIS worktree.
#
# The sim data is extracted from the product's own `report.html` + `metrics.json`
# (see `extract.py`), so the extraction is only reproducible if the render comes
# from a known revision. This script builds the local binary and scans the two
# external fixture repos into `sim/source/` (gitignored).
#
# Usage: bash sim/make-source.sh && python3 sim/extract.py
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
FIXTURES="${FIXTURES:-/home/shafin/Opencode/perfpal-test}"
OUT="$ROOT/sim/source"

if [[ ! -d "$FIXTURES/hyperfine" || ! -d "$FIXTURES/cJSON" ]]; then
  echo "missing fixture repos under $FIXTURES (expected hyperfine/ and cJSON/)" >&2
  exit 1
fi

cd "$ROOT"
"$CARGO" build --locked --offline
BIN="$ROOT/target/debug/perfpal"

rm -rf "$OUT"
mkdir -p "$OUT"
"$BIN" "$FIXTURES/hyperfine" -o "$OUT/out-hyperfine"
"$BIN" "$FIXTURES/cJSON" -o "$OUT/out-cjson"

for case in hyperfine cjson; do
  report="$OUT/out-$case/report.html"
  [[ -f "$report" ]] || { echo "missing $report" >&2; exit 1; }
  if ! grep -q 'viewBox="0 0 1400 718"' "$report"; then
    echo "$report: unexpected layout (expected the 1400x718 full-bleed canvas)" >&2
    exit 1
  fi
done

echo "source renders ready in $OUT (1400x718 full-bleed)"
