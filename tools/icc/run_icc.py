#!/usr/bin/env python3
"""Single entry point for the Perfpal 0.5.0 ICC reliability harness.

Steps
-----
1. Reuse or create Perfpal 0.5.0 scans under
   ``/var/tmp/opencode/icc/perfpal/<language>/metrics.json``.
2. Build the deterministic sample (``sample.json``).
3. Run the Python and C reference tools.
4. Consume Rust / JavaScript / TypeScript reference outputs produced by the
   parallel agent if ``tools/icc/ref/{rust,js,ts}.json`` exist.
5. Compute ICCs and gates; exit non-zero if a gate fails.

Usage:
    python3 tools/icc/run_icc.py [--repo ROOT] [--force-scans]
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

LANGUAGES = {
    "c": "examples",
    "rust": "examples-rust",
    "python": "examples-python",
    "js": "examples-js",
    "ts": "examples-ts",
    "extra": "tools/icc/extra",
}


def default_repo() -> Path:
    return Path(__file__).resolve().parents[2]


def run(cmd: list[str], cwd: Path) -> int:
    print("+ " + " ".join(cmd))
    completed = subprocess.run(cmd, cwd=cwd)
    return completed.returncode


def ensure_scans(
    repo: Path,
    binary: Path,
    metrics_root: Path,
    force_scans: bool,
) -> bool:
    ok = True
    for language, tree in LANGUAGES.items():
        out_dir = metrics_root / language
        metrics = out_dir / "metrics.json"
        if metrics.is_file() and not force_scans:
            print(f"reuse {metrics}")
            continue
        if not binary.is_file():
            print(f"missing Perfpal binary: {binary}", file=sys.stderr)
            return False
        out_dir.mkdir(parents=True, exist_ok=True)
        cmd = [str(binary), tree, "-o", str(out_dir), "--force"]
        code = run(cmd, repo)
        if code != 0 or not metrics.is_file():
            print(f"scan failed for {language} (exit {code})", file=sys.stderr)
            ok = False
    return ok


def main(argv: list[str] | None = None) -> int:
    repo = default_repo()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=repo)
    parser.add_argument(
        "--metrics-root",
        type=Path,
        default=Path("/var/tmp/opencode/icc/perfpal"),
    )
    parser.add_argument(
        "--binary",
        type=Path,
        default=repo / "target" / "release" / "perfpal",
    )
    parser.add_argument(
        "--force-scans",
        action="store_true",
        help="re-run the Perfpal binary even when metrics.json exists",
    )
    parser.add_argument(
        "--skip-scans",
        action="store_true",
        help="fail instead of running the binary when metrics are missing",
    )
    args = parser.parse_args(argv)

    repo = args.repo.resolve()
    icc_dir = repo / "tools" / "icc"
    python = sys.executable or "python3"

    if args.skip_scans:
        missing = [
            language
            for language in LANGUAGES
            if not (args.metrics_root / language / "metrics.json").is_file()
        ]
        if missing:
            print(
                f"missing Perfpal metrics for: {', '.join(missing)}", file=sys.stderr
            )
            return 2
    else:
        if not ensure_scans(repo, args.binary, args.metrics_root, args.force_scans):
            return 2

    prep_steps = [
        [python, str(icc_dir / "sample.py"), "--repo", str(repo),
         "--metrics-root", str(args.metrics_root)],
        [python, str(icc_dir / "make_labels.py"), "--repo", str(repo),
         "--metrics-root", str(args.metrics_root)],
        [python, str(icc_dir / "python_ref.py"), "--repo", str(repo)],
        [python, str(icc_dir / "c_ref.py"), "--repo", str(repo)],
    ]
    for cmd in prep_steps:
        code = run(cmd, repo)
        if code != 0:
            return code

    missing_refs = [
        language
        for language in ("rust", "js", "ts")
        if not (icc_dir / "ref" / f"{language}.json").is_file()
    ]
    if missing_refs:
        print(
            "missing reference outputs (owned by the parallel Rust/JS/TS agent): "
            + ", ".join(f"tools/icc/ref/{l}.json" for l in missing_refs),
            file=sys.stderr,
        )

    compute_code = run(
        [python, str(icc_dir / "compute_icc.py"), "--repo", str(repo)], repo
    )

    results_path = icc_dir / "results.json"
    if results_path.is_file():
        results = json.loads(results_path.read_text(encoding="utf-8"))
        freeze_ok = results.get("freeze_ok")
        if freeze_ok is True:
            return 0
        if freeze_ok is False:
            return 1
        # Labels absent: fall back to the independent-parser gates.
        gate_pass = all(
            gate.get("pass") is not False for gate in results["gate"].values()
        )
        if compute_code != 0 or not gate_pass:
            return 1
    return compute_code


if __name__ == "__main__":
    raise SystemExit(main())
