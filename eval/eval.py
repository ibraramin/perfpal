#!/usr/bin/env python3
"""
W2 eval harness.

For each case in cases.json:
  1. Build the source into a binary (with debug info + -O2).
  2. Run `perfpald diagnose` and capture the hotspots + bottlenecks.
  3. Run `perf report` and capture the ground-truth top functions.
  4. Compare (set-based, not rank-paired):
     (a) every expected user function appears in perfpal's hotspots,
     (b) self_pct for each expected function is within +/- tolerance
         of perf's value (paired by demangled-name match, not by rank),
     (c) all expected bottleneck kinds appear in the report.

Set-based comparison is critical: at high sample rates, two functions
within ~3% of each other can swap ranks between two independent `perf
record` runs. Rank-paired comparison would produce false failures.
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


def run_perfpald(binary: Path, duration: int, out_dir: Path,
                 memory_pressure: bool = False) -> dict | None:
    """Run perfpald diagnose on the binary, return parsed report.json or None on failure."""
    out_dir.mkdir(parents=True, exist_ok=True)
    for f in out_dir.glob("report-*.json"):
        f.unlink()
    cmd = [
        str(PERFPALD), "diagnose",
        "--binary", str(binary),
        "--duration", str(duration),
        "--output-dir", str(out_dir),
    ]
    if memory_pressure:
        cmd += ["--memory-pressure"]
    proc = subprocess.run(
        cmd,
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
    """Filter kernel/loader addresses and noisy kernel-module entries."""
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
            if len(parts) < 5:
                continue
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


def normalize(s: str) -> str:
    return s.split("+")[0].strip()


def names_match(a: str, b: str) -> bool:
    """Set-based name match tolerating clone suffixes and demangle drift."""
    an, bn = normalize(a), normalize(b)
    if an == bn or an.startswith(bn) or bn.startswith(an):
        return True
    if len(an) > 8 and (an in bn or bn in an):
        return True
    return False


def check_case(case: dict) -> tuple[bool, list[str]]:
    """Run a single case end-to-end. Return (passed, list of failure notes)."""
    name = case["name"]
    failures: list[str] = []
    duration = case.get("duration", 5)
    expected_funcs = case.get("expected_functions", [])
    expected_kinds = case.get("expected_kinds", [])
    memory_pressure = case.get("memory_pressure", False)

    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        try:
            binary = build_case(case, td)
        except Exception as e:
            return False, [f"build failed: {e}"]

        gt = run_perf_ground_truth(binary, duration)
        report = run_perfpald(binary, duration, BUILD, memory_pressure=memory_pressure)
        if not report:
            return False, ["perfpald returned no report"]

        perfpal_user = [
            (h["function"], h["self_pct"])
            for h in report["hotspots"]
            if _is_user_symbol(h["function"])
        ]
        gt_user = [(s, p) for s, p in gt if _is_user_symbol(s)]
        print(f"  ground truth (user): {[(s[:40], p) for s, p in gt_user[:6]]}")
        print(f"  perfpal     (user): {[(s[:40], p) for s, p in perfpal_user[:6]]}")

        # Set-based comparison: for each expected function, find it in both
        # ground truth and perfpal and check the delta.
        for ef in expected_funcs:
            gt_match = next(((s, p) for s, p in gt_user if names_match(s, ef)), None)
            pp_match = next(((s, p) for s, p in perfpal_user if names_match(s, ef)), None)
            if gt_match is None:
                failures.append(f"expected function {ef!r} not in perf ground truth")
                continue
            if pp_match is None:
                failures.append(f"expected function {ef!r} not in perfpal hotspots")
                continue
            delta = abs(gt_match[1] - pp_match[1])
            if delta > TOLERANCE_PCT:
                failures.append(
                    f"function {ef!r}: pct delta {delta:.2f} > {TOLERANCE_PCT}: gt={gt_match[1]:.2f} pp={pp_match[1]:.2f}"
                )

        # Bottleneck kind coverage.
        bottleneck_kinds = {b["kind"] for b in report.get("bottlenecks", [])}
        for ek in expected_kinds:
            if ek not in bottleneck_kinds:
                failures.append(f"expected bottleneck kind {ek!r} not emitted (got {sorted(bottleneck_kinds)})")

    return len(failures) == 0, failures


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
        ok, notes = check_case(case)
        if ok:
            print("  PASS")
            passed += 1
        else:
            print("  FAIL")
            for n in notes:
                print(f"    - {n}")
            failed += 1
    print(f"\n{passed} passed, {failed} failed (out of {len(cases)})")
    sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    main()
