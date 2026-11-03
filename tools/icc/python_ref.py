#!/usr/bin/env python3
"""Independent Python reference loop metrics using the stdlib ``ast`` module.

Counts ``for``/``while`` statements plus comprehension generators, mirroring
Perfpal 0.5.0's function-level loop metrics without using Perfpal's parser.

Policy notes
------------
* Only module-level ``FunctionDef`` / ``AsyncFunctionDef`` records are emitted;
  Perfpal does not emit nested function records.  Nested defs and lambdas are
  skipped while counting the enclosing function.
* ``loop_body_loc`` is the size (in lines) of the union of inclusive body line
  intervals.  For ``for``/``while`` the body is the first-to-last statement;
  for a comprehension it is the element expression (``elt``, or key/value for
  a dict comprehension).
* ``max_loop_depth`` is the syntactic nesting depth of counted loops.

Usage:
    python3 tools/icc/python_ref.py [--repo ROOT] [--sample FILE] [--out FILE]
"""

from __future__ import annotations

import argparse
import ast
import json
from pathlib import Path


def default_repo() -> Path:
    return Path(__file__).resolve().parents[2]


def node_end(node: ast.AST) -> int:
    end = getattr(node, "end_lineno", None)
    return end if end is not None else getattr(node, "lineno", 0)


def stmt_span(stmt: ast.AST) -> tuple[int, int]:
    return (getattr(stmt, "lineno", 0), node_end(stmt))


def comprehension_body_span(node: ast.AST) -> tuple[int, int]:
    if isinstance(node, ast.DictComp):
        spans = [stmt_span(node.key), stmt_span(node.value)]
        return (min(s[0] for s in spans), max(s[1] for s in spans))
    elt = getattr(node, "elt", None)
    if elt is None:  # defensive: unknown comprehension shape
        return (getattr(node, "lineno", 0), node_end(node))
    return stmt_span(elt)


class LoopCollector:
    """Collect ``(start_line, end_line, depth)`` triples for one function."""

    def __init__(self) -> None:
        self.loops: list[tuple[int, int, int]] = []

    def add(self, start: int, end: int, depth: int) -> None:
        if start and end:
            self.loops.append((start, max(start, end), depth))

    # -- statements -----------------------------------------------------
    def walk_stmt(self, stmt: ast.AST, depth: int) -> None:
        if isinstance(stmt, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
            # Nested definitions are separate functions/classes in Perfpal's
            # model (and nested functions are not emitted at all).
            return
        if isinstance(stmt, (ast.For, ast.AsyncFor, ast.While)):
            d = depth + 1
            if stmt.body:
                start = getattr(stmt.body[0], "lineno", 0)
                end = max(node_end(s) for s in stmt.body)
                self.add(start, end, d)
                for child in stmt.body:
                    self.walk_stmt(child, d)
            if isinstance(stmt, (ast.For, ast.AsyncFor)):
                self.walk_expr(stmt.iter, depth)
            else:
                self.walk_expr(stmt.test, depth)
            for child in stmt.orelse:
                self.walk_stmt(child, d)
            return
        if isinstance(stmt, (ast.ListComp, ast.SetComp, ast.DictComp, ast.GeneratorExp)):
            self.walk_expr(stmt, depth)
            return
        for child in ast.iter_child_nodes(stmt):
            if isinstance(child, ast.stmt):
                self.walk_stmt(child, depth)
            elif isinstance(child, ast.expr):
                self.walk_expr(child, depth)

    # -- expressions ----------------------------------------------------
    def walk_expr(self, expr: ast.AST, depth: int) -> None:
        if isinstance(expr, ast.Lambda):
            return
        if isinstance(expr, (ast.ListComp, ast.SetComp, ast.DictComp, ast.GeneratorExp)):
            start, end = comprehension_body_span(expr)
            generators = expr.generators
            for index, gen in enumerate(generators):
                d = depth + index + 1
                self.add(start, end, d)
                # The iterable is evaluated before/around this generator; a
                # comprehension nested there is one level shallower.
                self.walk_expr(gen.iter, depth + index)
                for cond in gen.ifs:
                    self.walk_expr(cond, d)
            elt_depth = depth + len(generators)
            if isinstance(expr, ast.DictComp):
                self.walk_expr(expr.key, elt_depth)
                self.walk_expr(expr.value, elt_depth)
            else:
                self.walk_expr(expr.elt, elt_depth)
            return
        for child in ast.iter_child_nodes(expr):
            if isinstance(child, ast.expr):
                self.walk_expr(child, depth)


def analyze_function(node: ast.AST) -> dict:
    collector = LoopCollector()
    for stmt in node.body:
        collector.walk_stmt(stmt, 0)

    lines: set[int] = set()
    for start, end, _depth in collector.loops:
        lines.update(range(start, end + 1))
    return {
        "loop_count": len(collector.loops),
        "loop_body_loc": len(lines),
        "max_loop_depth": max((d for _, _, d in collector.loops), default=0),
    }


def param_count(node: ast.AST) -> int:
    args = node.args
    count = (
        len(getattr(args, "posonlyargs", []))
        + len(args.args)
        + len(args.kwonlyargs)
    )
    if args.vararg is not None:
        count += 1
    if args.kwarg is not None:
        count += 1
    return count


def functions_for_file(repo: Path, rel_path: str) -> list[dict]:
    source = (repo / rel_path).read_text(encoding="utf-8")
    tree = ast.parse(source, filename=rel_path)
    out = []
    for node in tree.body:
        if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            continue
        metrics = analyze_function(node)
        out.append(
            {
                "path": rel_path,
                "name": node.name,
                "start_line": node.lineno,
                "start_col": node.col_offset,
                "param_count": param_count(node),
                **metrics,
            }
        )
    return out


def build(repo: Path, sample: dict) -> dict:
    paths = sorted(
        {
            fn["path"]
            for fn in sample["functions"]
            if fn.get("language") == "python"
        }
    )
    functions: list[dict] = []
    for rel_path in paths:
        functions.extend(functions_for_file(repo, rel_path))
    functions.sort(key=lambda f: (f["path"], f["start_line"], f["name"]))
    return {"language": "python", "functions": functions}


def main(argv: list[str] | None = None) -> int:
    repo = default_repo()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=repo)
    parser.add_argument("--sample", type=Path, default=repo / "tools" / "icc" / "sample.json")
    parser.add_argument("--out", type=Path, default=repo / "tools" / "icc" / "ref" / "python.json")
    args = parser.parse_args(argv)

    sample = json.loads(args.sample.read_text(encoding="utf-8"))
    result = build(args.repo.resolve(), sample)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    with args.out.open("w", encoding="utf-8") as fh:
        json.dump(result, fh, indent=2)
        fh.write("\n")
    print(f"python reference: {len(result['functions'])} functions -> {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
