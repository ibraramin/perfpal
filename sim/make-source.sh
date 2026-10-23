#!/usr/bin/env bash
# Regenerate the Atlas Story data from THIS worktree's Perfpal binary.
#
# The data comes from Perfpal's own artifacts - `atlas-layout.json` emitted by
# `--story` plus `metrics.json` - never from the rendered HTML. This script
# builds the local binary, scans the two external fixture repos into
# `sim/source/` (gitignored), and regenerates `sim/data-<case>.json`.
#
# Usage: bash sim/make-source.sh
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
"$BIN" "$FIXTURES/hyperfine" -o "$OUT/out-hyperfine" --story
"$BIN" "$FIXTURES/cJSON" -o "$OUT/out-cjson" --story

for case in hyperfine cjson; do
  report="$OUT/out-$case/report.html"
  layout="$OUT/out-$case/atlas-layout.json"
  [[ -f "$report" ]] || { echo "missing $report" >&2; exit 1; }
  [[ -f "$layout" ]] || { echo "missing $layout" >&2; exit 1; }
  if ! grep -q 'viewBox="0 0 1400 718"' "$report"; then
    echo "$report: unexpected layout (expected the 1400x718 full-bleed canvas)" >&2
    exit 1
  fi
done

python3 "$ROOT/sim/from-layout.py" --source "$OUT" --sim "$ROOT/sim"

echo "source renders + story data ready in $OUT (1400x718 full-bleed)"
