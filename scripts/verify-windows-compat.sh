#!/usr/bin/env bash
# Periodic Windows-compatibility verification for Perfpal.
# Runbook: docs/cross-platform-verification.md
#
# Steps: (1) cross type-check ALL targets, (2) windows-gnu release build,
#        (3) Wine runtime smoke + Linux <-> Wine output-equivalence diff.
#
# Usage:  scripts/verify-windows-compat.sh [corpus-dir]   (default: examples-python)
# Exit 0 = PASS. Requires: rustup target x86_64-pc-windows-gnu, mingw-w64-gcc, wine.
set -euo pipefail

TARGET=x86_64-pc-windows-gnu
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

WORK="${TMPDIR:-/var/tmp}/perfpal-wine-compat"
EXE="target/$TARGET/release/perfpal.exe"
CORPUS="${1:-examples-python}"

# Linux path -> Wine Z: drive path (Wine maps / to Z:\).
winpath() { printf 'Z:%s' "$1" | tr '/' '\\'; }

echo "== [1/4] cargo check --all-targets ($TARGET)  [catches unix-only APIs in src/ + tests] =="
cargo check --locked --all-targets --target "$TARGET"

echo "== [2/4] release build ($TARGET) =="
cargo build --release --locked --target "$TARGET"
file "$EXE"

if ! command -v wine >/dev/null 2>&1; then
  echo "!! wine not found — the BUILD half passed; install wine for the runtime half (sudo pacman -S wine)."
  exit 0
fi

echo "== [3/4] Wine staging =="
mkdir -p "$WORK"
cp -f "$EXE" "$WORK/perfpal.exe"
for dll in libgcc_s_seh-1.dll libwinpthread-1.dll; do
  src="/usr/x86_64-w64-mingw32/bin/$dll"
  [ -f "$src" ] && cp -f "$src" "$WORK/" || true
done
export WINEPREFIX="$WORK/prefix" WINEDEBUG=-all WINEDLLOVERRIDES="mscoree,mshtml=" DISPLAY=
wine_run() { ( cd "$WORK" && wine perfpal.exe "$@" 2>/dev/null ); }

echo "== [4/4] Wine runtime + Linux-equivalence =="
wine_run --version

set +e
wine_run --output "$(winpath "$WORK/bad")" 'Z:\no\such\path' >/dev/null; ec1=$?
wine_run --bogus-flag >/dev/null; ec2=$?
set -e
[ "$ec1" = 1 ] && echo "  usage-error (bad path) exit=$ec1 OK" || { echo "  FAIL: bad-path exit=$ec1 (expect 1)"; exit 1; }
[ "$ec2" = 1 ] && echo "  usage-error (bad flag) exit=$ec2 OK" || { echo "  FAIL: bad-flag exit=$ec2 (expect 1)"; exit 1; }

echo "  scanning '$CORPUS' under Wine and on Linux…"
wine_run --output "$(winpath "$WORK/wine-out")" "$(winpath "$REPO_ROOT/$CORPUS")" >/dev/null
cargo build --release --locked >/dev/null
"target/release/perfpal" --output "$WORK/lin-out" "$CORPUS" >/dev/null

python3 - "$WORK/lin-out/metrics.json" "$WORK/wine-out/metrics.json" <<'PY'
import json, sys
def norm(p):
    d = json.load(open(p))
    for k in ("root_path", "started_at", "duration_ms"):
        d.get("scan", {}).pop(k, None)
    return json.dumps(d, sort_keys=True)
a, b = (norm(p) for p in sys.argv[1:3])
print("  metrics.json identical to Linux (modulo provenance):", a == b)
sys.exit(0 if a == b else 1)
PY

echo "PASS — Windows build + Wine runtime verified. (semantic-clang tier NOT covered; see docs/cross-platform-verification.md)"
