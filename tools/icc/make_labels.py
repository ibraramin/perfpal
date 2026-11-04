#!/usr/bin/env python3
"""Generate blind manual-annotation packets for the ICC study.

The packets deliberately contain **no Perfpal labels**.  ``loop_packet.json``
contains 100 loop-bearing sampled functions with source lines; ``site_packet``
contains 100 call-like candidates inside loop bodies.  ``site_comparison.json``
is a separate, non-blind file used only by ``compute_icc.py`` to score the
annotations later.

The dead-C fixture ``tools/icc/extra/dead.c`` is scanned separately so both the
live and ``#if 0`` call sites are represented for ``cfg_gated`` validation.

Usage:
    python3 tools/icc/make_labels.py [--repo ROOT] [--sample FILE]
                                     [--metrics-root DIR] [--out DIR]
"""

from __future__ import annotations

import argparse
import ast
import json
import re
import sys
from pathlib import Path

sys.dont_write_bytecode = True

LANGUAGES = ("c", "rust", "python", "js", "ts")
TREE_BY_LANGUAGE = {
    "c": "examples",
    "rust": "examples-rust",
    "python": "examples-python",
    "js": "examples-js",
    "ts": "examples-ts",
}
EXTRA_TREE = "tools/icc/extra"
LOOP_PACKET_SIZE = 100
SITE_PACKET_SIZE = 100
DEAD_PATH = f"{EXTRA_TREE}/dead.c"

PY_CALL_RE = re.compile(r"\b([A-Za-z_]\w*(?:\s*\.\s*[A-Za-z_]\w*)*)\s*\(")
C_CALL_RE = re.compile(
    r"\b([A-Za-z_]\w*(?:\s*(?:\.|->|::)\s*[A-Za-z_]\w*)*)\s*\("
)
PY_EXCLUDE = {
    "if", "for", "while", "return", "def", "class", "with", "elif", "in",
    "not", "and", "or", "lambda", "assert", "raise", "except", "from",
}
C_EXCLUDE = {
    "if", "for", "while", "switch", "catch", "return", "sizeof", "typeof",
    "function", "def", "fn", "match", "else", "do", "new", "await", "async",
    "constructor",
}


def default_repo() -> Path:
    return Path(__file__).resolve().parents[2]


def read_lines(repo: Path, rel_path: str) -> list[str]:
    return (repo / rel_path).read_text(encoding="utf-8", errors="replace").splitlines()


def numbered_source(lines: list[str], start: int, end: int) -> str:
    out = []
    for number in range(start, min(end, len(lines)) + 1):
        out.append(f"{number:>4}  {lines[number - 1]}")
    return "\n".join(out)


# ---------------------------------------------------------------------------
# Loop-body line discovery
# ---------------------------------------------------------------------------

def python_loop_lines(repo: Path, rel_path: str, start_line: int) -> set[int]:
    """Loop body lines for a module-level Python function."""
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    import python_ref  # local reference helper

    tree = ast.parse((repo / rel_path).read_text(encoding="utf-8"))
    function = next(
        (
            node
            for node in tree.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
            and node.lineno == start_line
        ),
        None,
    )
    if function is None:
        return set()
    collector = python_ref.LoopCollector()
    for statement in function.body:
        collector.walk_stmt(statement, 0)
    lines: set[int] = set()
    for body_start, body_end, _depth in collector.loops:
        lines.update(range(body_start, body_end + 1))
    return lines


def _blank_strings_and_comments(line: str, in_block: bool) -> tuple[str, bool]:
    """Return code text with string/comment contents blanked, plus block state."""
    out = []
    index = 0
    quote = None
    while index < len(line):
        char = line[index]
        pair = line[index : index + 2]
        if in_block:
            if pair == "*/":
                in_block = False
                out.extend("  ")
                index += 2
            else:
                out.append(" ")
                index += 1
            continue
        if quote is not None:
            if char == "\\":
                out.extend("  ")
                index += 2
                continue
            if char == quote:
                quote = None
                out.append(" ")
                index += 1
                continue
            out.append(" ")
            index += 1
            continue
        if pair == "/*":
            in_block = True
            out.extend("  ")
            index += 2
            continue
        if pair == "//":
            out.extend(" " * (len(line) - index))
            break
        if char in ('"', "'", "`"):
            quote = char
            out.append(" ")
            index += 1
            continue
        out.append(char)
        index += 1
    return "".join(out), in_block


def _matching_brace(code_lines: list[str], start_line0: int, start_col: int) -> int | None:
    """Return the 0-based line index of the matching ``}`` (or None)."""
    depth = 0
    for line_index in range(start_line0, len(code_lines)):
        text = code_lines[line_index]
        begin = start_col if line_index == start_line0 else 0
        for column in range(begin, len(text)):
            char = text[column]
            if char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    return line_index
    return None


def _matching_paren(code_lines: list[str], start_line0: int, start_col: int) -> tuple[int, int] | None:
    depth = 0
    started = False
    for line_index in range(start_line0, len(code_lines)):
        text = code_lines[line_index]
        begin = start_col if line_index == start_line0 else 0
        for column in range(begin, len(text)):
            char = text[column]
            if char == "(":
                depth += 1
                started = True
            elif char == ")":
                depth -= 1
                if started and depth == 0:
                    return (line_index, column)
    return None


def _first_code_position(
    code_lines: list[str], start_line0: int, start_col: int
) -> tuple[int, int] | None:
    for line_index in range(start_line0, len(code_lines)):
        text = code_lines[line_index]
        begin = start_col if line_index == start_line0 else 0
        for column in range(begin, len(text)):
            if not text[column].isspace():
                return (line_index, column)
    return None


def _statement_end(code_lines: list[str], start_line0: int, start_col: int) -> int:
    """0-based line index of the semicolon ending a single-statement body."""
    paren_depth = 0
    brace_depth = 0
    for line_index in range(start_line0, len(code_lines)):
        text = code_lines[line_index]
        begin = start_col if line_index == start_line0 else 0
        for column in range(begin, len(text)):
            char = text[column]
            if char == "(":
                paren_depth += 1
            elif char == ")":
                paren_depth -= 1
            elif char == "{":
                brace_depth += 1
            elif char == "}":
                brace_depth -= 1
            elif char == ";" and paren_depth == 0 and brace_depth == 0:
                return line_index
    return start_line0


def _loop_body_range(
    code_lines: list[str], line0: int, after_col: int, is_do: bool
) -> tuple[int, int] | None:
    """Return (first_line0, last_line0) for a loop body."""
    search_line, search_col = line0, after_col
    if not is_do:
        paren = _first_code_position(code_lines, line0, after_col)
        if paren is None:
            return None
        if code_lines[paren[0]][paren[1]] == "(":
            closing = _matching_paren(code_lines, paren[0], paren[1])
            if closing is None:
                return None
            search_line, search_col = closing[0], closing[1] + 1
        else:
            # Rust-style header without parentheses: find the opening brace.
            for line_index in range(paren[0], len(code_lines)):
                text = code_lines[line_index]
                begin = paren[1] if line_index == paren[0] else 0
                for column in range(begin, len(text)):
                    if text[column] == "{":
                        closing = _matching_brace(code_lines, line_index, column)
                        if closing is None:
                            return None
                        return (line_index, closing)
                    if text[column] == ";":
                        return (line_index, line_index)
            return None
    first = _first_code_position(code_lines, search_line, search_col)
    if first is None:
        return None
    if code_lines[first[0]][first[1]] == "{":
        closing = _matching_brace(code_lines, first[0], first[1])
        if closing is None:
            return None
        return (first[0], closing)
    end = _statement_end(code_lines, first[0], first[1])
    return (first[0], end)


def brace_loop_lines(lines: list[str], start: int, end: int) -> set[int]:
    """Conservative brace-scanner loop-body ranges for C-like languages."""
    code_lines = []
    in_block = False
    for line in lines:
        code, in_block = _blank_strings_and_comments(line, in_block)
        code_lines.append(code)

    result: set[int] = set()
    loop_re = re.compile(r"\b(for|while|do)\b")
    limit = min(end, len(code_lines))
    index = start - 1
    while index < limit:
        code = code_lines[index]
        for match in loop_re.finditer(code):
            keyword = match.group(1)
            body = _loop_body_range(code_lines, index, match.end(), keyword == "do")
            if body is None:
                continue
            first, last = body
            result.update(range(first + 1, last + 2))
        index += 1
    return result


def loop_lines(repo: Path, language: str, rel_path: str, start: int, end: int) -> set[int]:
    if language == "python":
        return python_loop_lines(repo, rel_path, start)
    lines = read_lines(repo, rel_path)
    return brace_loop_lines(lines, start, end)


# ---------------------------------------------------------------------------
# Call candidates
# ---------------------------------------------------------------------------

def call_on_line(text: str, language: str) -> list[tuple[str, int]]:
    """Return (callee, column) candidates on one code line."""
    regex = PY_CALL_RE if language == "python" else C_CALL_RE
    exclude = PY_EXCLUDE if language == "python" else C_EXCLUDE
    return [
        (match.group(1), match.start(1))
        for match in regex.finditer(text)
        if match.group(1) not in exclude
    ]


def candidates(
    repo: Path, language: str, rel_path: str, function: str, start: int, end: int
) -> list[dict]:
    lines = read_lines(repo, rel_path)
    body_lines = loop_lines(repo, language, rel_path, start, end)
    found = []
    seen = set()
    for line_number in sorted(body_lines):
        if line_number < 1 or line_number > len(lines):
            continue
        code, _ = _blank_strings_and_comments(lines[line_number - 1], False)
        for name, column in call_on_line(code, language):
            key = (line_number, column, name)
            if key in seen:
                continue
            seen.add(key)
            context_start = max(1, line_number - 1)
            context_end = min(len(lines), line_number + 1)
            found.append(
                {
                    "language": language,
                    "path": rel_path,
                    "function": function,
                    "line": line_number,
                    # 0-based byte column, matching metrics.json
                    # resource_sites[].column.
                    "column": column,
                    "candidate": name,
                    "source_line": lines[line_number - 1].strip(),
                    "context": numbered_source(lines, context_start, context_end),
                }
            )
    return found


def dead_function_span(repo: Path) -> tuple[int, int]:
    lines = read_lines(repo, DEAD_PATH)
    code_lines = []
    in_block = False
    for line in lines:
        code, in_block = _blank_strings_and_comments(line, in_block)
        code_lines.append(code)
    for index, code in enumerate(code_lines):
        if "dead_sites" in code and "(" in code:
            start = index + 1
            body_end = _matching_brace(code_lines, index, 0)
            if body_end is None:
                body_end = start
            return start, body_end
    return 1, len(lines)


# ---------------------------------------------------------------------------
# Perfpal resource-site index (comparison only, never in the packet)
# ---------------------------------------------------------------------------

def metrics_path_for(sample_path: str) -> str:
    if sample_path.startswith(EXTRA_TREE + "/"):
        return sample_path[len(EXTRA_TREE) + 1 :]
    for tree in TREE_BY_LANGUAGE.values():
        if sample_path.startswith(tree + "/"):
            return sample_path[len(tree) + 1 :]
    return sample_path


def perfpal_site_index(metrics_root: Path) -> dict[tuple[str, int, int], dict]:
    """Index Perfpal resource sites by ``(path, line, 0-based column)``.

    Column is part of the key so two calls on the same source line cannot
    alias each other (the old ``(path, line)`` key produced false positives
    such as site-083/084).
    """
    index: dict[tuple[str, int, int], dict] = {}
    docs = []
    for language in LANGUAGES:
        path = metrics_root / language / "metrics.json"
        if path.is_file():
            docs.append((language, json.loads(path.read_text(encoding="utf-8"))))
    extra = metrics_root / "extra" / "metrics.json"
    if extra.is_file():
        docs.append(("extra", json.loads(extra.read_text(encoding="utf-8"))))
    for language, doc in docs:
        for function in doc.get("stats", {}).get("functions", []):
            flat = function.get("path", "")
            if language == "extra":
                rel = f"{EXTRA_TREE}/{flat}"
            else:
                rel = f"{TREE_BY_LANGUAGE[language]}/{flat}"
            for site in function.get("resource_sites", []) or []:
                key = (rel, site.get("line"), site.get("column"))
                current = index.get(key)
                if current is None or site.get("kind") == "alloc":
                    index[key] = {
                        "kind": site.get("kind"),
                        "cfg_gated": site.get("cfg_gated"),
                    }
    return index


def build(repo: Path, sample: dict, metrics_root: Path, out_dir: Path) -> None:
    functions = sample["functions"]
    by_id = {fn["id"]: fn for fn in functions}

    # -- loop packet ----------------------------------------------------
    loop_functions = [
        fn for fn in functions if (fn.get("loop_count") or 0) > 0
    ]
    loop_functions.sort(
        key=lambda fn: (fn["path"], fn["start_line"], fn["name"])
    )
    loop_selected = loop_functions[:LOOP_PACKET_SIZE]
    loop_items = []
    for number, fn in enumerate(loop_selected, 1):
        lines = read_lines(repo, fn["path"])
        loop_items.append(
            {
                "id": f"loop-{number:03d}",
                "language": fn["language"],
                "path": fn["path"],
                "name": fn["name"],
                "start_line": fn["start_line"],
                "source": numbered_source(lines, fn["start_line"], fn["end_line"]),
            }
        )

    # -- site candidates ------------------------------------------------
    dead_start, dead_end = dead_function_span(repo)
    dead_sites = candidates(
        repo, "c", DEAD_PATH, "dead_sites", dead_start, dead_end
    )

    sample_sites: dict[str, list[dict]] = {language: [] for language in LANGUAGES}
    for fn in functions:
        if (fn.get("loop_count") or 0) <= 0:
            continue
        for item in candidates(
            repo,
            fn["language"],
            fn["path"],
            fn["name"],
            fn["start_line"],
            fn["end_line"],
        ):
            sample_sites[fn["language"]].append(item)
    for language in LANGUAGES:
        sample_sites[language].sort(
            key=lambda item: (
                item["path"],
                item["line"],
                item.get("column", 0),
                item["function"],
            )
        )

    remaining = SITE_PACKET_SIZE - len(dead_sites)
    selected_sample: list[dict] = []
    cursors = {language: 0 for language in LANGUAGES}
    while len(selected_sample) < remaining:
        progressed = False
        for language in LANGUAGES:
            if len(selected_sample) >= remaining:
                break
            bucket = sample_sites[language]
            cursor = cursors[language]
            if cursor < len(bucket):
                selected_sample.append(bucket[cursor])
                cursors[language] = cursor + 1
                progressed = True
        if not progressed:
            break

    site_items = dead_sites + selected_sample
    site_items.sort(
        key=lambda item: (
            item["language"],
            item["path"],
            item["line"],
            item.get("column", 0),
            item["function"],
        )
    )
    site_items = site_items[:SITE_PACKET_SIZE]
    for number, item in enumerate(site_items, 1):
        item["id"] = f"site-{number:03d}"

    index = perfpal_site_index(metrics_root)
    comparison_items = []
    for item in site_items:
        ref = index.get((item["path"], item["line"], item["column"]))
        comparison_items.append(
            {
                "id": item["id"],
                "language": item["language"],
                "path": item["path"],
                "function": item["function"],
                "line": item["line"],
                "column": item["column"],
                "perfpal_kind": ref["kind"] if ref else "neither",
                "perfpal_cfg_gated": ref["cfg_gated"] if ref else None,
            }
        )

    out_dir.mkdir(parents=True, exist_ok=True)
    write_json(
        out_dir / "loop_packet.json",
        {
            "count": len(loop_items),
            "instruction": "label each function; see labels/README.md",
            "items": loop_items,
        },
    )
    write_json(
        out_dir / "site_packet.json",
        {
            "count": len(site_items),
            "instruction": "label each call-site candidate; see labels/README.md",
            "items": site_items,
        },
    )
    write_json(
        out_dir / "site_comparison.json",
        {
            "note": "not blind: Perfpal reference for scoring only",
            "dead_sites_included": len(dead_sites),
            "items": comparison_items,
        },
    )
    print(
        f"labels: {len(loop_items)} loop functions, {len(site_items)} sites "
        f"({len(dead_sites)} from dead.c) -> {out_dir}"
    )


def write_json(path: Path, doc: dict) -> None:
    with path.open("w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=2)
        fh.write("\n")


def main(argv: list[str] | None = None) -> int:
    repo = default_repo()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=repo)
    parser.add_argument("--sample", type=Path, default=repo / "tools" / "icc" / "sample.json")
    parser.add_argument(
        "--metrics-root",
        type=Path,
        default=Path("/var/tmp/opencode/icc/perfpal"),
    )
    parser.add_argument(
        "--out", type=Path, default=repo / "tools" / "icc" / "labels"
    )
    args = parser.parse_args(argv)

    sample = json.loads(args.sample.read_text(encoding="utf-8"))
    build(args.repo.resolve(), sample, args.metrics_root, args.out.resolve())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
