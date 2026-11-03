#!/usr/bin/env python3
"""Independent C reference loop metrics using clang's JSON AST.

Uses ``/usr/bin/clang -Xclang -ast-dump=json -fsyntax-only``.  For every
``FunctionDecl`` with a body that belongs to the target translation unit, count
``ForStmt`` / ``WhileStmt`` / ``DoStmt`` nodes and measure the loop body span
from the body child's source range.  Clang omits redundant ``line`` fields and,
for the main file, the ``file`` field; offsets are resolved through a byte
offset -> line map.

Known divergence: clang drops ``#if 0`` regions while Perfpal counts them
syntactically.  Any function whose source range contains ``#if 0`` / ``#if (0)``
is marked ``dead_code_excluded: true`` so the statistics step can skip it
instead of silently comparing incomparable values.

Usage:
    python3 tools/icc/c_ref.py [--repo ROOT] [--sample FILE] [--out FILE]
                               [--clang /usr/bin/clang]
"""

from __future__ import annotations

import argparse
import bisect
import json
import re
import subprocess
import sys
from pathlib import Path

LOOP_KINDS = {"ForStmt", "WhileStmt", "DoStmt"}
# Independent C allocation vocabulary for alloc_sites_in_loops.  This list is
# written from the C allocation API and deliberately not imported from
# Perfpal's allocation-family table.
ALLOC_FUNCTIONS = {"malloc", "calloc", "realloc", "aligned_alloc", "strdup"}
DEAD_CODE_RE = re.compile(r"^\s*#\s*if\s+\(?\s*0\s*\)?\s*(?://|/\*|$)")
INCLUDE_RE = re.compile(r'^\s*#\s*include\s*"([^"]+)"')


def default_repo() -> Path:
    return Path(__file__).resolve().parents[2]


def line_starts(data: bytes) -> list[int]:
    starts = [0]
    for index, byte in enumerate(data):
        if byte == 0x0A:  # '\n'
            starts.append(index + 1)
    return starts


def make_line_of(starts: list[int]):
    def line_of(offset: int | None) -> int | None:
        if offset is None:
            return None
        return bisect.bisect_right(starts, offset)

    return line_of


def point_line(point: dict, line_of) -> int | None:
    if not isinstance(point, dict):
        return None
    if point.get("line") is not None:
        return int(point["line"])
    return line_of(point.get("offset"))


def range_lines(node: dict, line_of) -> tuple[int, int] | None:
    rng = node.get("range") if isinstance(node, dict) else None
    if not isinstance(rng, dict):
        return None
    start = point_line(rng.get("begin") or {}, line_of)
    end = point_line(rng.get("end") or {}, line_of)
    if start is None:
        start = end
    if end is None:
        end = start
    if start is None or end is None:
        return None
    return (start, max(start, end))


def node_line(node: dict, line_of) -> int | None:
    loc = node.get("loc") or {}
    line = point_line(loc, line_of)
    if line is not None:
        return line
    rng = range_lines(node, line_of)
    return rng[0] if rng else None


def node_col(node: dict) -> int | None:
    loc = node.get("loc") or {}
    if loc.get("col") is not None:
        return int(loc["col"])
    rng = node.get("range") or {}
    begin = rng.get("begin") or {}
    if begin.get("col") is not None:
        return int(begin["col"])
    return None


def loc_file(node: dict) -> str | None:
    loc = node.get("loc") or {}
    for key in ("file",):
        if loc.get(key):
            return loc[key]
    for key in ("expansionLoc", "spellingLoc"):
        nested = loc.get(key) or {}
        if nested.get("file"):
            return nested["file"]
    rng = node.get("range") or {}
    for point in (rng.get("begin") or {}, rng.get("end") or {}):
        if point.get("file"):
            return point["file"]
    return None


def is_target_file(node: dict, target: Path) -> bool:
    loc = node.get("loc") or {}
    rng = node.get("range") or {}
    points = [loc, rng.get("begin") or {}, rng.get("end") or {}]
    if any(point.get("includedFrom") for point in points):
        # A node from an included header, even when clang elides "file".
        return False
    filename = loc_file(node)
    if not filename:
        # clang omits the file for nodes in the main input file.
        return True
    try:
        return Path(filename).resolve() == target.resolve()
    except OSError:
        return False


def loop_body(node: dict) -> dict | None:
    inner = [child for child in node.get("inner", []) if isinstance(child, dict)]
    if not inner:
        return None
    kind = node.get("kind")
    if kind == "DoStmt":
        # DoStmt: body, condition
        return inner[0]
    # ForStmt: init, (null), condition, increment, body
    # WhileStmt: condition, body
    return inner[-1]


def collect_loops(node: dict, depth: int, line_of, out: list[tuple[int, int, int]]) -> None:
    if not isinstance(node, dict):
        return
    kind = node.get("kind")
    if kind in LOOP_KINDS:
        loop_depth = depth + 1
        body = loop_body(node)
        if body is not None:
            span = range_lines(body, line_of)
            if span is not None:
                out.append((span[0], span[1], loop_depth))
        for child in node.get("inner", []):
            if not isinstance(child, dict):
                continue
            collect_loops(child, loop_depth if child is body else depth, line_of, out)
        return
    for child in node.get("inner", []):
        if isinstance(child, dict):
            collect_loops(child, depth, line_of, out)


UNWRAP_KINDS = {
    "ImplicitCastExpr",
    "ParenExpr",
    "ExprWithCleanups",
    "CStyleCastExpr",
    "ConstantExpr",
}


def callee_name(call: dict) -> str | None:
    """Resolve a direct C call's callee name, if it is a plain function ref."""
    inner = [child for child in call.get("inner", []) if isinstance(child, dict)]
    if not inner:
        return None
    node = inner[0]
    while isinstance(node, dict) and node.get("kind") in UNWRAP_KINDS:
        nested = [child for child in node.get("inner", []) if isinstance(child, dict)]
        if not nested:
            return None
        node = nested[0]
    if not isinstance(node, dict) or node.get("kind") != "DeclRefExpr":
        return None
    referenced = node.get("referencedDecl") or {}
    return referenced.get("name") or node.get("name")


def count_alloc_sites(node: dict, depth: int) -> int:
    """Count direct alloc calls whose syntactic loop depth is at least 1."""
    if not isinstance(node, dict):
        return 0
    total = 0
    if node.get("kind") == "CallExpr" and depth >= 1:
        if callee_name(node) in ALLOC_FUNCTIONS:
            total += 1
    kind = node.get("kind")
    if kind in LOOP_KINDS:
        body = loop_body(node)
        for child in node.get("inner", []):
            if not isinstance(child, dict):
                continue
            total += count_alloc_sites(child, depth + 1 if child is body else depth)
        return total
    for child in node.get("inner", []):
        if isinstance(child, dict):
            total += count_alloc_sites(child, depth)
    return total


def analyze_function(body: dict, line_of) -> dict:
    loops: list[tuple[int, int, int]] = []
    collect_loops(body, 0, line_of, loops)
    lines: set[int] = set()
    for start, end, _depth in loops:
        lines.update(range(start, end + 1))
    return {
        "loop_count": len(loops),
        "loop_body_loc": len(lines),
        "max_loop_depth": max((d for _, _, d in loops), default=0),
        "alloc_sites_in_loops": count_alloc_sites(body, 0),
    }


def walk_function_decls(node, out: list[dict]) -> None:
    if not isinstance(node, dict):
        return
    if node.get("kind") == "FunctionDecl":
        out.append(node)
    for child in node.get("inner", []):
        if isinstance(child, dict):
            walk_function_decls(child, out)


ZERO_IF_RE = re.compile(r"^\s*#\s*if\s+\(?\s*0\s*\)?\s*(?://|/\*|$)")
DIRECTIVE_RE = re.compile(r"^\s*#\s*(if|ifdef|ifndef|elif|else|endif)\b(.*)$")


def disabled_lines(source_lines: list[str]) -> set[int]:
    """Return 1-based line numbers inside an ``#if 0`` branch.

    A small preprocessor-lite pass: only ``#if 0`` / ``#if (0)`` branches are
    treated as disabled.  ``#else`` / non-zero ``#elif`` re-enable the branch;
    nested conditionals inherit the outer disabled state.
    """
    disabled: set[int] = set()
    stack: list[dict] = []
    for number, line in enumerate(source_lines, 1):
        match = DIRECTIVE_RE.match(line)
        if match:
            directive, rest = match.group(1), match.group(2)
            if directive in ("if", "ifdef", "ifndef"):
                is_zero = directive == "if" and bool(ZERO_IF_RE.match(line))
                stack.append({"zero": is_zero})
            elif directive == "elif":
                if stack:
                    stack[-1]["zero"] = bool(ZERO_IF_RE.match(line))
            elif directive == "else":
                if stack:
                    stack[-1]["zero"] = False
            elif directive == "endif":
                if stack:
                    stack.pop()
            if any(frame["zero"] for frame in stack):
                disabled.add(number)
            continue
        if any(frame["zero"] for frame in stack):
            disabled.add(number)
    return disabled


def ensure_stub_includes(repo: Path, rel_path: str, stub_dir: Path) -> None:
    """Create empty stubs for deliberately unresolved quoted includes.

    A few examples reference headers that do not exist on purpose (unresolved
    include diagnostics).  Clang stops at a fatal missing include, so the
    reference compilation supplies empty stubs; Perfpal's syntactic scan is
    unaffected by the header contents.
    """
    target = repo / rel_path
    for line in target.read_text(encoding="utf-8", errors="replace").splitlines():
        match = INCLUDE_RE.match(line)
        if not match:
            continue
        name = match.group(1)
        if (target.parent / name).exists():
            continue
        stub = stub_dir / Path(name).name
        stub_dir.mkdir(parents=True, exist_ok=True)
        if not stub.exists():
            stub.write_text("", encoding="utf-8")


def parse_file(
    clang: str, repo: Path, rel_path: str, stub_dir: Path
) -> tuple[list[dict], int]:
    target = (repo / rel_path).resolve()
    ensure_stub_includes(repo, rel_path, stub_dir)
    proc = subprocess.run(
        [
            clang,
            "-Wno-implicit-function-declaration",
            "-I",
            str(stub_dir),
            "-Xclang",
            "-ast-dump=json",
            "-fsyntax-only",
            str(target),
        ],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        raise SystemExit(
            f"clang failed for {rel_path} (exit {proc.returncode}):\n{proc.stderr}"
        )
    ast = json.loads(proc.stdout)

    data = target.read_bytes()
    starts = line_starts(data)
    line_of = make_line_of(starts)
    source_lines = data.decode("utf-8", errors="replace").splitlines()
    total_lines = len(source_lines)

    decls: list[dict] = []
    walk_function_decls(ast, decls)

    functions: list[dict] = []
    dead_code_excluded = 0
    for decl in decls:
        if not is_target_file(decl, target):
            continue
        inner = [child for child in decl.get("inner", []) if isinstance(child, dict)]
        body = next((child for child in inner if child.get("kind") == "CompoundStmt"), None)
        if body is None:
            continue
        start_line = node_line(decl, line_of)
        if start_line is None or start_line < 1 or start_line > total_lines:
            continue
        metrics = analyze_function(body, line_of)
        end_span = range_lines(decl, line_of)
        end_line = end_span[1] if end_span else start_line
        snippet = source_lines[start_line - 1 : end_line]
        dead = any(DEAD_CODE_RE.match(line) for line in snippet)
        if dead:
            dead_code_excluded += 1
        functions.append(
            {
                "path": rel_path,
                "name": decl.get("name"),
                "start_line": start_line,
                "start_col": node_col(decl),
                "param_count": sum(
                    1 for child in inner if child.get("kind") == "ParmVarDecl"
                ),
                **metrics,
                "dead_code_excluded": dead,
            }
        )
    functions.sort(key=lambda f: (f["start_line"], f["name"] or ""))
    return functions, dead_code_excluded


def build(repo: Path, sample: dict, clang: str) -> dict:
    paths = sorted(
        {fn["path"] for fn in sample["functions"] if fn.get("language") == "c"}
    )
    stub_dir = Path("/var/tmp/opencode/icc/clang-stubs")
    functions: list[dict] = []
    dead_total = 0
    for rel_path in paths:
        parsed, dead = parse_file(clang, repo, rel_path, stub_dir)
        functions.extend(parsed)
        dead_total += dead

    # Whole functions inside `#if 0` are absent from clang's AST but present in
    # Perfpal's syntactic scan.  Emit an explicit dead-code placeholder so the
    # comparison records why the subject was excluded instead of silently
    # treating it as an unmatched function.
    emitted = {
        (f["path"], f["name"], f["start_line"]) for f in functions
    }
    sample_c = [f for f in sample["functions"] if f.get("language") == "c"]
    disabled_cache: dict[str, set[int]] = {}
    for fn in sample_c:
        key = (fn["path"], fn["name"], fn["start_line"])
        if key in emitted:
            continue
        rel_path = fn["path"]
        if rel_path not in disabled_cache:
            source = (repo / rel_path).read_text(encoding="utf-8", errors="replace")
            disabled_cache[rel_path] = disabled_lines(source.splitlines())
        if fn["start_line"] in disabled_cache[rel_path]:
            functions.append(
                {
                    "path": rel_path,
                    "name": fn["name"],
                    "start_line": fn["start_line"],
                    "start_col": fn.get("start_col"),
                    "param_count": fn.get("param_count"),
                    "loop_count": None,
                    "loop_body_loc": None,
                    "max_loop_depth": None,
                    "alloc_sites_in_loops": None,
                    "dead_code_excluded": True,
                }
            )
            dead_total += 1

    functions.sort(key=lambda f: (f["path"], f["start_line"], f["name"] or ""))
    return {
        "language": "c",
        "functions": functions,
        "dead_code_excluded_count": dead_total,
    }


def main(argv: list[str] | None = None) -> int:
    repo = default_repo()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=repo)
    parser.add_argument("--sample", type=Path, default=repo / "tools" / "icc" / "sample.json")
    parser.add_argument("--out", type=Path, default=repo / "tools" / "icc" / "ref" / "c.json")
    parser.add_argument("--clang", default="/usr/bin/clang")
    args = parser.parse_args(argv)

    sample = json.loads(args.sample.read_text(encoding="utf-8"))
    result = build(args.repo.resolve(), sample, args.clang)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as fh:
        json.dump(result, fh, indent=2)
        fh.write("\n")
    print(
        f"c reference: {len(result['functions'])} functions, "
        f"{result['dead_code_excluded_count']} dead-code-only -> {args.out}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
