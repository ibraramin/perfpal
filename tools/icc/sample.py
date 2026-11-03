#!/usr/bin/env python3
"""Build the deterministic ICC function sample from Perfpal 0.5.0 metrics.

The sample is drawn from Perfpal's own function records so that the reference
tools and the statistics step have a stable, reproducible set of subjects.
Synthetic ``<module>`` records are excluded.

Selection rule
--------------
Base corpus: for each language take *all* functions with ``loop_count > 0`` and
then fill the remainder with no-loop functions in ``(path, span.start_line)``
order until the base target is reached: 188 functions.

Extra corpus: ``tools/icc/extra`` contributes all 12 loop-bearing real
functions (C 4, Rust 2, Python 2, JavaScript 2, TypeScript 2); ``dead.c`` is
excluded from the loop/alloc sample because clang drops its ``#if 0`` branch.
The final sample is exactly 200 functions.  No randomness is used
(``seed`` is null).

Usage:
    python3 tools/icc/sample.py [--repo ROOT] [--metrics-root DIR] [--out FILE]
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

# language -> (tree relative to repo root, base sample size)
LANGUAGES = {
    "c": ("examples", 38),
    "rust": ("examples-rust", 40),
    "python": ("examples-python", 40),
    "js": ("examples-js", 40),
    "ts": ("examples-ts", 42),
}

EXTRA_TREE = "tools/icc/extra"
EXTRA_EXCLUDE = {"dead.c"}
EXTRA_TARGET = 12
TOTAL_TARGET = 200
LANGUAGE_BY_SUFFIX = {
    ".c": "c",
    ".rs": "rust",
    ".py": "python",
    ".js": "js",
    ".ts": "ts",
}

FIELDS = (
    "loop_count",
    "loop_body_loc",
    "max_loop_depth",
    "alloc_sites_in_loops",
    "io_site_count",
    "io_sites_in_loops",
)


def default_repo() -> Path:
    return Path(__file__).resolve().parents[2]


def load_metrics(metrics_root: Path, language: str) -> dict:
    path = metrics_root / language / "metrics.json"
    if not path.is_file():
        raise SystemExit(
            f"missing Perfpal metrics for {language}: {path}\n"
            "run tools/icc/run_icc.py (or the 0.5.0 binary) first"
        )
    with path.open(encoding="utf-8") as fh:
        return json.load(fh)


def real_functions(doc: dict, tree: str) -> list[dict]:
    """Return non-synthetic function records with repo-relative paths."""
    out = []
    for fn in doc.get("stats", {}).get("functions", []):
        if fn.get("name") == "<module>":
            continue
        span = fn.get("span") or {}
        out.append(
            {
                "id": fn.get("id"),
                "path": f"{tree}/{fn['path']}",
                "name": fn["name"],
                "start_line": span.get("start_line"),
                "start_col": span.get("start_col"),
                "end_line": span.get("end_line"),
                "param_count": fn.get("param_count"),
                **{field: fn.get(field) for field in FIELDS},
            }
        )
    return out


def extra_functions(metrics_root: Path) -> list[dict]:
    """Map the flat extra metrics paths to ``tools/icc/extra/<file>``."""
    doc = load_metrics(metrics_root, "extra")
    out = []
    for fn in doc.get("stats", {}).get("functions", []):
        if fn.get("name") == "<module>":
            continue
        flat = fn.get("path") or ""
        if Path(flat).name in EXTRA_EXCLUDE:
            continue
        language = LANGUAGE_BY_SUFFIX.get(Path(flat).suffix)
        if language is None:
            continue
        span = fn.get("span") or {}
        out.append(
            {
                "id": fn.get("id"),
                "path": f"{EXTRA_TREE}/{flat}",
                "name": fn["name"],
                "start_line": span.get("start_line"),
                "start_col": span.get("start_col"),
                "end_line": span.get("end_line"),
                "param_count": fn.get("param_count"),
                "language": language,
                **{field: fn.get(field) for field in FIELDS},
            }
        )
    return out


def select(functions: list[dict], target: int) -> tuple[list[dict], dict]:
    ordered = sorted(functions, key=lambda f: (f["path"], f["start_line"] or 0, f["name"]))
    loops = [f for f in ordered if (f.get("loop_count") or 0) > 0]
    no_loops = [f for f in ordered if (f.get("loop_count") or 0) == 0]
    take_no_loops = max(0, target - len(loops))
    chosen_no_loops = no_loops[:take_no_loops]
    selected = sorted(
        loops + chosen_no_loops,
        key=lambda f: (f["path"], f["start_line"] or 0, f["name"]),
    )
    info = {
        "base_target": target,
        "base_selected": len(selected),
        "selected": len(selected),
        "loop_functions_selected": len(loops),
        "no_loop_functions_selected": len(chosen_no_loops),
        "available_real_functions": len(functions),
    }
    return selected, info


def build(repo: Path, metrics_root: Path) -> dict:
    all_functions: list[dict] = []
    counts: dict[str, dict] = {}
    for language, (tree, target) in LANGUAGES.items():
        doc = load_metrics(metrics_root, language)
        funcs = real_functions(doc, tree)
        selected, info = select(funcs, target)
        for fn in selected:
            fn["language"] = language
        all_functions.extend(selected)
        counts[language] = info

    extras = extra_functions(metrics_root)
    extra_loops = [f for f in extras if (f.get("loop_count") or 0) > 0]
    if len(extra_loops) != EXTRA_TARGET:
        raise SystemExit(
            f"expected {EXTRA_TARGET} loop-bearing extra functions, "
            f"found {len(extra_loops)}: "
            + ", ".join(f"{f['path']}:{f['name']}" for f in extra_loops)
        )
    for fn in extra_loops:
        counts[fn["language"]]["extra_selected"] = (
            counts[fn["language"]].get("extra_selected", 0) + 1
        )
        counts[fn["language"]]["selected"] += 1
        counts[fn["language"]]["loop_functions_selected"] += 1
        counts[fn["language"]]["available_real_functions"] += 1
    all_functions.extend(extra_loops)

    for language, info in counts.items():
        info.setdefault("extra_selected", 0)
        # The final sample is exactly the target now; no shortfall.
        info["requested"] = info["selected"]
        info["shortfall"] = 0

    all_functions.sort(
        key=lambda f: (f["language"], f["path"], f["start_line"] or 0, f["name"])
    )
    if len(all_functions) != TOTAL_TARGET:
        raise SystemExit(
            f"expected {TOTAL_TARGET} sampled functions, got {len(all_functions)}"
        )

    functions_by_language = {
        language: {
            "language": language,
            "functions": [f for f in all_functions if f["language"] == language],
        }
        for language in LANGUAGES
    }
    return {
        "seed": None,
        "languages": list(LANGUAGES),
        "selection": (
            "base: all loop-bearing non-<module> functions plus no-loop "
            "functions in deterministic (path, start_line) order up to the "
            "base target (188); extra: all 12 loop-bearing functions under "
            "tools/icc/extra, excluding dead.c"
        ),
        "corpus": {lang: tree for lang, (tree, _) in LANGUAGES.items()},
        "targets": {
            language: counts[language]["selected"] for language in LANGUAGES
        },
        "extra": {
            "tree": EXTRA_TREE,
            "excluded_files": [f"{EXTRA_TREE}/{name}" for name in sorted(EXTRA_EXCLUDE)],
            "selected": len(extra_loops),
        },
        "counts": counts,
        "total_functions": len(all_functions),
        "by_language": functions_by_language,
        "functions": all_functions,
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=default_repo())
    parser.add_argument(
        "--metrics-root",
        type=Path,
        default=Path("/var/tmp/opencode/icc/perfpal"),
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="output path (default: <repo>/tools/icc/sample.json)",
    )
    args = parser.parse_args(argv)

    repo = args.repo.resolve()
    out = args.out or repo / "tools" / "icc" / "sample.json"
    sample = build(repo, args.metrics_root)

    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8") as fh:
        json.dump(sample, fh, indent=2, sort_keys=False)
        fh.write("\n")

    for language, info in sample["counts"].items():
        print(
            f"{language:6s} selected {info['selected']:3d}/{info['requested']:3d} "
            f"(base {info['base_selected']:3d} + extra {info['extra_selected']:2d}; "
            f"loop {info['loop_functions_selected']:3d}, "
            f"no-loop {info['no_loop_functions_selected']:3d})"
        )
    print(f"wrote {out} ({sample['total_functions']} functions, seed=null)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
