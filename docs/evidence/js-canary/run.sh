#!/usr/bin/env bash
# W0 js-ts canary driver: fetch the pinned corpus, run the counter twice, prove
# determinism, and publish the raw counts.
#
# Idempotent: reuses an existing checkout, re-verifies the pin AND that the
# checkout is clean, re-runs the counter. Runnable from the repository root (or
# anywhere).
#
#   docs/evidence/js-canary/run.sh
#
# Requires bash >= 4 (associative arrays), git, and sha256sum or shasum. python3
# is used only for the optional human-readable summary; the artifact is published
# even when python3 is absent.
#
# Env knobs: WORKDIR (default /var/tmp/opencode/js-canary),
#            CARGO_TARGET_DIR (default /var/tmp/perfpal-w0-b), CARGO (cargo path).

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd -P)"
workdir="${WORKDIR:-/var/tmp/opencode/js-canary}"
target_dir="${CARGO_TARGET_DIR:-/var/tmp/perfpal-w0-b}"
out_dir="$repo_root/docs/evidence/js-canary"

cargo_cmd="${CARGO:-}"
if [[ -z "$cargo_cmd" ]]; then
    if [[ -x "$HOME/.cargo/bin/cargo" ]]; then
        cargo_cmd="$HOME/.cargo/bin/cargo"
    else
        cargo_cmd="cargo"
    fi
fi

# id | git URL | tag | pinned commit
declare -A urls=(
    [express]="https://github.com/expressjs/express.git"
    [execa]="https://github.com/sindresorhus/execa.git"
    [gemini-cli]="https://github.com/google-gemini/gemini-cli.git"
)
declare -A tags=(
    [express]="v5.1.0"
    [execa]="v9.6.1"
    [gemini-cli]="v0.65.0"
)
declare -A commits=(
    [express]="cd7d4397c398a3f3ecadeaf9ef6ac1377bd414c4"
    [execa]="84e1f36b2c31a463e3efbd37c7505da9438a05c9"
    [gemini-cli]="6006c14a4decfe4b891c6263ad6ad5992e404b5b"
)
corpus_order=(express execa gemini-cli)

mkdir -p "$workdir" "$out_dir"
# Normalise so ".", trailing slashes, and symlinks cannot bypass the guard.
workdir="$(cd "$workdir" && pwd -P)"

case "$workdir" in
    "/"|"$repo_root")
        echo "ERROR: refusing unsafe WORKDIR '$workdir'" >&2
        exit 2
        ;;
esac

roots=()
for id in "${corpus_order[@]}"; do
    dir="$workdir/$id"
    if [[ -e "$dir" && ! -d "$dir/.git" ]]; then
        rm -rf -- "$dir"
    fi
    if [[ ! -d "$dir/.git" ]]; then
        echo "==> cloning $id at ${tags[$id]}" >&2
        git clone --depth 1 --branch "${tags[$id]}" "${urls[$id]}" "$dir"
    fi
    actual="$(git -C "$dir" rev-parse HEAD)"
    if [[ "$actual" != "${commits[$id]}" ]]; then
        echo "ERROR: $id pin mismatch: expected ${commits[$id]}, got $actual" >&2
        exit 1
    fi
    if [[ -n "$(git -C "$dir" status --porcelain)" ]]; then
        echo "ERROR: $id checkout is dirty (measurements would not match the pin): $dir" >&2
        exit 1
    fi
    roots+=("$dir")
done

tmp1="$(mktemp)"
tmp2="$(mktemp)"
trap 'rm -f "$tmp1" "$tmp2"' EXIT

run_counter() {
    CARGO_TARGET_DIR="$target_dir" \
        "$cargo_cmd" run --manifest-path "$repo_root/Cargo.toml" \
        --locked --quiet --example js_canary -- "${roots[@]}"
}

echo "==> counter run 1" >&2
run_counter > "$tmp1"
echo "==> counter run 2 (determinism check)" >&2
run_counter > "$tmp2"

if ! cmp -s "$tmp1" "$tmp2"; then
    echo "ERROR: counter output is not byte-identical across runs" >&2
    exit 1
fi
echo "==> determinism: OK (byte-identical)" >&2

if command -v python3 >/dev/null 2>&1; then
    python3 - "$tmp1" <<'PYSUMMARY'
import json
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    doc = json.load(handle)

print("\ncorpus")
print(f"  {'repo':<12} {'tag':<10} {'prod files':>10} {'test files':>10} "
      f"{'prod lines':>11} {'test lines':>11} {'parse fail':>10}")
for name, entry in doc["corpus"].items():
    print(f"  {name:<12} {entry['tag']:<10} {entry['prod_files']:>10} "
          f"{entry['test_files']:>10} {entry['prod_lines']:>11} "
          f"{entry['test_lines']:>11} {entry['parse_failures']:>10}")

totals = doc["totals"]
print("\nrules (totals: prod / test)")
for rule, prod_hits in totals["rules_prod"].items():
    print(f"  {rule:<22} {prod_hits:>7} / {totals['rules_test'][rule]:>7}")

print("\ncontrols (totals: prod / test)")
for control, prod_hits in totals["controls_prod"].items():
    print(f"  {control:<24} {prod_hits:>7} / {totals['controls_test'][control]:>7}")
PYSUMMARY
else
    echo "python3 not found; skipping the human-readable summary" >&2
fi

# Publish only after the determinism proof and the summary step succeeded.
cp -- "$tmp1" "$out_dir/canary_results.json"
echo
echo "wrote $out_dir/canary_results.json"

if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$repo_root/tools/js_canary.rs" "$out_dir/canary_results.json"
else
    shasum -a 256 "$repo_root/tools/js_canary.rs" "$out_dir/canary_results.json"
fi
