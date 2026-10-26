#!/usr/bin/env bash
# Run the S4 3k scale gate (F2-F7) from the current worktree.
#
# Usage: bash sim/scale.sh
#   ATLAS_3K_FORCE=1 bash sim/scale.sh   # regenerate the deterministic fixture
#
# Writes only /var/tmp/opencode/ and docs/evidence/atlas-sim/; never creates a
# repo-tracked data file. Exits non-zero on any failed assertion.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO="${CARGO:-$HOME/.cargo/bin/cargo}"
BIN="$ROOT/target/debug/perfpal"
FIXTURE_DIR="/var/tmp/opencode/atlas-3k-fixture"

cd "$ROOT"
export PATH="$HOME/.cargo/bin:$PATH"

if [[ ! -f "$FIXTURE_DIR/manifest.json" ]]; then
  python3 sim/fixtures/gen-3k.py --out "$FIXTURE_DIR"
fi

if [[ ! -x "$BIN" ]]; then
  "$CARGO" build --locked --offline
fi

if python3 sim/scale.py; then
  echo "3k scale gate: PASS"
else
  echo "3k scale gate: FAIL"
  exit 1
fi
