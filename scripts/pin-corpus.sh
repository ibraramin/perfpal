#!/usr/bin/env bash
# Pin + scan loop for the divergence corpus (plan divergence-revamp bullet 4).
# Reads cells from docs/evidence/corpus.json. Writes NOTHING inside the repo
# except this script's own stdout (checkouts + scan outputs go to WORKDIR).
#
#   scripts/pin-corpus.sh [--corpus PATH] [--workdir DIR] [--cell ID] --mode verify|scan
#
# verify (default): clone (skip if checkout exists) -> rev-parse -> date gate.
# scan:             verify, then perfpal scan TWICE into separate out dirs,
#                   mask provenance fields, print counts + sha256 proof lines
#                   that docs/evidence/corpus-packet.md records per cell.
set -euo pipefail

CORPUS="docs/evidence/corpus.json"
WORKDIR="/var/tmp/opencode/div-corpus"
CELL=""
MODE="verify"
PERFPAL="${PERFPAL:-perfpal}"

while [ $# -gt 0 ]; do
  case "$1" in
    --corpus)  CORPUS="$2"; shift 2 ;;
    --workdir) WORKDIR="$2"; shift 2 ;;
    --cell)    CELL="$2"; shift 2 ;;
    --mode)    MODE="$2"; shift 2 ;;
    *) echo "usage: $0 [--corpus PATH] [--workdir DIR] [--cell ID] --mode verify|scan" >&2; exit 1 ;;
  esac
done

[ "$MODE" = verify ] || [ "$MODE" = scan ] || { echo "mode must be verify|scan" >&2; exit 1; }
command -v jq >/dev/null || { echo "jq required" >&2; exit 1; }
command -v git >/dev/null || { echo "git required" >&2; exit 1; }

mkdir -p "$WORKDIR"

cell_ids() {
  if [ -n "$CELL" ]; then echo "$CELL";
  else jq -r '.cells[] | select(.status=="pinned") | .id' "$CORPUS"; fi
}

cell_field() { jq -r --arg id "$1" '.cells[] | select(.id==$id) | .[$f] // empty' --arg f "$2" "$CORPUS"; }

fail=0
for id in $(cell_ids); do
  url="$(cell_field "$id" url)"
  tag="$(cell_field "$id" tag)"
  want="$(cell_field "$id" commit)"
  role="$(cell_field "$id" role)"
  co="$WORKDIR/$(cell_field "$id" checkout)"

  if [ -z "$want" ]; then echo "[$id] SKIP (proposed, no pinned SHA)"; continue; fi

  if [ -d "$co/.git" ]; then
    echo "[$id] checkout exists, reusing $co"
  else
    echo "[$id] cloning --depth 1 --branch $tag $url"
    git clone --depth 1 --branch "$tag" "$url" "$co"
  fi

  got="$(git -C "$co" rev-parse HEAD)"
  dat="$(git -C "$co" log -1 --format=%ci)"
  echo "[$id] rev-parse: $got"
  echo "[$id] date:      $dat"
  if [ "$got" != "$want" ]; then
    echo "[$id] MISMATCH: want $want" >&2; fail=1; continue
  fi

  case "$role" in
    longitudinal-pre)
      [ "$dat" '<' "2022-01-01" ] || { echo "[$id] DATE-GATE FAIL: pre cell must be < 2022-01-01" >&2; fail=1; continue; } ;;
    longitudinal-post)
      [ "$dat" '>' "2024-01-01" ] || [ "$dat" = "2024-01-01" ] || { echo "[$id] DATE-GATE FAIL: post cell must be >= 2024-01-01" >&2; fail=1; continue; } ;;
    *) echo "[$id] role $role: date recorded, no gate" ;;
  esac
  echo "[$id] VERIFIED ($role)"

  if [ "$MODE" = scan ]; then
    command -v "$PERFPAL" >/dev/null || { echo "[$id] PERFPAL binary '$PERFPAL' not found" >&2; fail=1; continue; }
    for run in 1 2; do
      out="$WORKDIR/out-div-$id-r$run"
      rm -rf "$out"
      "$PERFPAL" "$co" -o "$out"
    done
    for run in 1 2; do
      out="$WORKDIR/out-div-$id-r$run"
      sum="$(python3 - "$out/metrics.json" <<'EOF'
import json,sys,hashlib
m=json.load(open(sys.argv[1]))
m.get("scan",{}).pop("started_at",None)
m.get("scan",{}).pop("duration_ms",None)
print(hashlib.sha256(json.dumps(m).encode()).hexdigest())
EOF
)"
      files="$(jq -r '.scan.files_scanned // "?"' "$out/metrics.json" 2>/dev/null || echo ?)"
      echo "[$id] run$run masked metrics.json sha256: $sum (files: $files)"
    done
    echo "[$id] counts slot: PENDING (paste the 'scanned N files ... findings' summary + per-rule table)"
    echo "[$id] proof slot:  PENDING (runs IDENTICAL iff the two sha256 lines match)"
  fi
done

[ "$fail" -eq 0 ] || { echo "FAILURES present" >&2; exit 1; }
echo "OK ($MODE)"
