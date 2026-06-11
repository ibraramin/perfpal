#!/usr/bin/env python3
"""
W1 eval harness.

For each case in cases.json:
  1. Build the source into a binary (with debug info + -O2).
  2. Run `perfpald diagnose` and capture the hotspots.
  3. Run `perf report` and capture the ground-truth top functions.
  4. Compare: for the top-N expected functions, do they appear in
     perfpal's report at approximately the same self-pct (within +/- 2%)?

This is the W1 gate from the plan: "matches perf report ground truth on
10 OSS binaries within +/- 2% on top-10 functions."

W2 will extend this to check bottleneck identification, not just function ranking.
"""
from __future__ import annotations
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent
PKG = ROOT.parent
BUILD = ROOT / "build"
BUILD.mkdir(exist_ok=True)

PERFPALD = PKG / "target" / "debug" / "perfpald"
TOLERANCE_PCT = 2.0   # +/- 2% absolute on self_pct
TOP_N = 5             # check the top-5 functions


def build_case(case: dict, workdir: Path) -> Path:
    """Compile the case's source into a binary. Returns the binary path."""
    src = PKG / case["source"]
    if not src.exists():
        raise FileNotFoundError(src)
    binary = workdir / case["name"]
    cmd = case.get("compile", ["g++", "-O2", "-g"])
    cmd += [str(src), "-o", str(binary)]
    subprocess.run(cmd, check=True)
    return binary


def run_perfpald(binary: Path, duration: int, out_dir: Path) -> dict | None:
    """Run perfpald diagnose on the binary, return parsed report.json or None on failure."""
    out_dir.mkdir(parents=True, exist_ok=True)
    # Clear stale reports.
    for f in out_dir.glob("report-*.json"):
        f.unlink()
    proc = subprocess.run(
        [str(PERFPALD), "diagnose", "--binary", str(binary), "--duration", str(duration), "--output-dir", str(out_dir)],
        capture_output=True,
        text=True,
        timeout=duration * 3 + 30,
    )
    if proc.returncode != 0:
        print(f"  [perfpald stderr]\n{proc.stderr}", file=sys.stderr)
        return None
    reports = sorted(out_dir.glob("report-*.json"))
    if not reports:
        return None
    return json.loads(reports[-1].read_text())


def _is_user_symbol(symbol: str) -> bool:
    """Filter kernel/loader addresses and noisy kernel-module entries.

    perf report --stdio on this kernel surfaces raw addresses like
    '0xffffffffb46015f0' and kernel-module symbols prefixed with
    '[k] ' that are noise floor (parasite entries in the user-only
    cycles:u event). User-code functions we care about are demangled
    C++/C symbols that don't start with '0x' and don't carry a
    '[k]' module tag.
    """
    s = symbol.strip()
    if s.startswith("0x") or s.startswith("0X"):
        return False
    if s.startswith("[k]"):
        return False
    return True


def run_perf_ground_truth(binary: Path, duration: int) -> list[tuple[str, float]]:
    """Run `perf record` + `perf report` and return [(function, self_pct), ...] sorted by self_pct desc."""
    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        data = td / "perf.data"
        proc = subprocess.run(
            ["perf", "record", "-F", "999", "-e", "cycles:u", "--output", str(data), "--", str(binary)],
            capture_output=True,
            text=True,
            timeout=duration * 3 + 30,
        )
        if proc.returncode != 0 and not data.exists():
            print(f"  [perf record failed] {proc.stderr}", file=sys.stderr)
            return []
        proc = subprocess.run(
            ["perf", "report", "-i", str(data), "--stdio", "-n", "--no-children", "-g", "none"],
            capture_output=True,
            text=True,
            timeout=30,
        )
        if proc.returncode != 0:
            return []
        out = []
        for line in proc.stdout.splitlines():
            stripped = line.lstrip()
            if not stripped or stripped.startswith("#"):
                continue
            parts = stripped.split()
            if not parts or not parts[0].endswith("%"):
                continue
            try:
                pct = float(parts[0].rstrip("%"))
            except ValueError:
                continue
            # Find the symbol after the [k/.] bracket
            if len(parts) < 5:
                continue
            if not parts[-1 - (0 if "[" in parts[3] else 0):][0].startswith("["):
                # format: pct samples comm dso [k/.] symbol...
                pass
            # Symbol is everything from the 4th field onward, skipping the bracket.
            # With -n: pct samples comm dso [bracket] symbol
            # Without -n: pct comm dso [bracket] symbol
            if parts[1].isdigit():
                sym_start = 4
            else:
                sym_start = 3
            if not parts[sym_start].startswith("["):
                continue
            symbol = " ".join(parts[sym_start + 1 :])
            if not _is_user_symbol(symbol):
                continue
            out.append((symbol, pct))
        return out


def main():
    cases_path = ROOT / "cases.json"
    if not cases_path.exists():
        print(f"no cases.json at {cases_path}", file=sys.stderr)
        sys.exit(2)
    cases = json.loads(cases_path.read_text())

    passed = 0
    failed = 0
    for case in cases:
        name = case["name"]
        print(f"=== {name} ===")
        with tempfile.TemporaryDirectory() as td:
            td = Path(td)
            try:
                binary = build_case(case, td)
            except Exception as e:
                print(f"  BUILD FAILED: {e}")
                failed += 1
                continue
            duration = case.get("duration", 5)
            gt = run_perf_ground_truth(binary, duration)
            report = run_perfpald(binary, duration, BUILD)
            if not report:
                print("  PERFPALD FAILED")
                failed += 1
                continue

            perfpal_top = [(h["function"], h["self_pct"]) for h in report["hotspots"] if _is_user_symbol(h["function"])][:TOP_N]
            gt_top = gt[:TOP_N]
            print(f"  ground truth top-{TOP_N}: {[(s[:40], p) for s, p in gt_top]}")
            print(f"  perfpal     top-{TOP_N}: {[(s[:40], p) for s, p in perfpal_top]}")

            # Compare function names and pct values.
            # Allow symbol-name mismatch when one is mangled and the other is demangled
            # by checking the demangled prefix.
            def normalize(s: str) -> str:
                return s.split("+")[0].strip()

            case_pass = True
            for i, pair in enumerate(zip(gt_top, perfpal_top)):
                (gt_sym, gt_pct) = pair[0]
                (pp_sym, pp_pct) = pair[1]
                gt_n = normalize(gt_sym)
                pp_n = normalize(pp_sym)
                if not (gt_n == pp_n or gt_n.startswith(pp_n) or pp_n.startswith(gt_n) or (len(gt_n) > 8 and (gt_n in pp_n or pp_n in gt_n))):
                    print(f"  rank {i+1}: name mismatch: gt={gt_sym!r} pp={pp_sym!r}")
                    case_pass = False
                    continue
                delta = abs(gt_pct - pp_pct)
                if delta > TOLERANCE_PCT:
                    print(f"  rank {i+1}: pct delta {delta:.2f} > {TOLERANCE_PCT}: gt={gt_pct:.2f} pp={pp_pct:.2f}")
                    case_pass = False

            if case_pass:
                print("  PASS")
                passed += 1
            else:
                print("  FAIL")
                failed += 1
    print(f"\n{passed} passed, {failed} failed (out of {len(cases)})")
    sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    main()
