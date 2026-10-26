#!/usr/bin/env python3
"""Generate the deterministic 3,000-source-file Atlas S4 / F7 scale fixture.

The fixture is intentionally synthetic and static-only: every file parses as
plain C or Rust, but nothing is compiled. It exercises Perfpal's directory
scenes, the 20-node Atlas canvas cap, cross-file reachability, unresolved
indirection (C function pointer + Rust ``dyn``) and the flow I/O table.

Usage:
    python3 sim/fixtures/gen-3k.py --out /var/tmp/opencode/atlas-3k-fixture [--force]

Layout (exactly 3,000 scanned .c/.rs files):
    1  src/main.c                    (entry; calls every module entry)
    30 src/modNN/entry.c             (one per module; calls every module file)
    2,700 src/modNN/subMM/fNN.*      (9 subdirs x 10 files x 30 modules)
    269 tests/test_NNN.c             (flat, non-production)
    ---
    3,000 scanned source files, plus a non-scanned README.md and manifest.json.

Neither timestamps nor randomness enter any file or the manifest, and every
iteration order is fixed. Re-running against a complete fixture is a no-op
unless ``--force`` is passed.
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from pathlib import Path
from typing import Any

DEFAULT_OUT = Path("/var/tmp/opencode/atlas-3k-fixture")

MODULE_COUNT = 30
SUBDIRS_PER_MODULE = 9
FILES_PER_SUBDIR = 10
MODULE_FILE_COUNT = MODULE_COUNT * SUBDIRS_PER_MODULE * FILES_PER_SUBDIR
TEST_FILE_COUNT = 269
SOURCE_FILE_COUNT = 1 + MODULE_COUNT + MODULE_FILE_COUNT + TEST_FILE_COUNT

# The two engineered indirection files must stay inside the drawn slice.
C_FN_POINTER_MODULE = 0
C_FN_POINTER_SUB = 0
C_FN_POINTER_FILE = 0
RUST_DYN_MODULE = MODULE_COUNT - 1
RUST_DYN_SUB = SUBDIRS_PER_MODULE - 1
RUST_DYN_FILE = FILES_PER_SUBDIR - 1

C_FN_POINTER_PATH = (
    f"src/mod{C_FN_POINTER_MODULE:02d}/sub{C_FN_POINTER_SUB:02d}/"
    f"f{C_FN_POINTER_FILE:02d}.c"
)
RUST_DYN_PATH = (
    f"src/mod{RUST_DYN_MODULE:02d}/sub{RUST_DYN_SUB:02d}/"
    f"f{RUST_DYN_FILE:02d}.rs"
)

HEADER = "#include <stdio.h>\n\n"


def module_entry_name(module: int) -> str:
    return f"mod{module:02d}_entry"


def module_file_name(module: int, sub: int, file_index: int) -> str:
    return f"mod{module:02d}_sub{sub:02d}_f{file_index:02d}_work"


def module_file_path(module: int, sub: int, file_index: int) -> str:
    extension = "rs" if (module, sub, file_index) == (
        RUST_DYN_MODULE,
        RUST_DYN_SUB,
        RUST_DYN_FILE,
    ) else "c"
    return f"src/mod{module:02d}/sub{sub:02d}/f{file_index:02d}.{extension}"


def c_io_block(unique_stem: str, indent: str = "    ") -> str:
    return (
        f'{indent}FILE *handle = fopen("/tmp/atlas-synth-{unique_stem}", "r");\n'
        f"{indent}if (handle != NULL) {{\n"
        f"{indent}    fclose(handle);\n"
        f"{indent}}}\n"
    )


def c_loop(iterations: int, indent: str = "    ") -> str:
    return (
        f"{indent}for (int i = 0; i < {iterations}; i += 1) {{\n"
        f"{indent}    total += i;\n"
        f"{indent}}}\n"
    )


def main_source() -> str:
    calls = "\n".join(
        f"    total += {module_entry_name(module)}(total);"
        for module in range(MODULE_COUNT)
    )
    prototypes = "\n".join(
        f"int {module_entry_name(module)}(int x);"
        for module in range(MODULE_COUNT)
    )
    return (
        HEADER
        + prototypes
        + "\n\nint main(void) {\n"
        + "    int total = 0;\n"
        + calls
        + "\n"
        + c_loop(1000)
        + c_io_block("main")
        + "    return total > 0 ? 0 : 0;\n"
        + "}\n"
    )


def module_entry_source(module: int) -> str:
    calls = "\n".join(
        "    total += "
        f"{module_file_name(module, sub, file_index)}(total);"
        for sub in range(SUBDIRS_PER_MODULE)
        for file_index in range(FILES_PER_SUBDIR)
    )
    prototypes = "\n".join(
        f"int {module_file_name(module, sub, file_index)}(int x);"
        for sub in range(SUBDIRS_PER_MODULE)
        for file_index in range(FILES_PER_SUBDIR)
    )
    return (
        HEADER
        + prototypes
        + f"\n\nint {module_entry_name(module)}(int x) {{\n"
        + "    int total = x;\n"
        + calls
        + "\n"
        + c_loop(96)
        + c_io_block(f"mod{module:02d}-entry")
        + "    return total;\n"
        + "}\n"
    )


def module_c_source(module: int, sub: int, file_index: int) -> str:
    if (module, sub, file_index) == (
        C_FN_POINTER_MODULE,
        C_FN_POINTER_SUB,
        C_FN_POINTER_FILE,
    ):
        return c_function_pointer_source()

    name = module_file_name(module, sub, file_index)
    return (
        HEADER
        + f"int {name}(int x) {{\n"
        + "    int total = x;\n"
        + c_loop(64)
        + c_io_block(f"mod{module:02d}-sub{sub:02d}-f{file_index:02d}")
        + "    return total;\n"
        + "}\n"
    )


def c_function_pointer_source() -> str:
    """High-cost reachable C file with a local function-pointer call.

    The 40 literal-trip loops put this file near the top of the reachable cost
    ranking, and the ``fp(x)`` call is the F3 C indirection probe.
    """
    name = module_file_name(
        C_FN_POINTER_MODULE, C_FN_POINTER_SUB, C_FN_POINTER_FILE
    )
    helper = f"{name}_helper"
    loops = "".join(c_loop(1_000_000) for _ in range(40))
    return (
        HEADER
        + f"static int {helper}(int value) {{\n"
        + "    return value + 3;\n"
        + "}\n\n"
        + f"int {name}(int x) {{\n"
        + "    int total = x;\n"
        + f"    int (*fp)(int) = &{helper};\n"
        + loops
        + "    total += fp(x);\n"
        + "    total += "
        + helper
        + "(1);\n"
        + c_io_block("mod00-sub00-f00-fnptr")
        + "    return total;\n"
        + "}\n"
    )


def rust_dyn_source() -> str:
    """High-cost Rust file with a ``dyn Fn`` call (the F3 Rust probe).

    The Rust file cannot be structurally reached from a C ``main`` because the
    call-graph producer is language-scoped; it is made the global hottest file
    so the Atlas always draws it. The unresolved ``f(x)`` row is the proof.
    """
    name = module_file_name(RUST_DYN_MODULE, RUST_DYN_SUB, RUST_DYN_FILE)
    loops = "".join(
        "    for i in 0..1000000 {\n"
        "        total += i;\n"
        "    }\n"
        for _ in range(50)
    )
    return (
        f"fn {name}(x: i32) -> i32 {{\n"
        "    let f: &dyn Fn(i32) -> i32 = &|v| v + 1;\n"
        "    let mut total = f(x);\n"
        + loops
        + '    let _probe = std::fs::read_to_string("/tmp/atlas-synth-rust-dyn-probe");\n'
        + "    total\n"
        + "}\n"
    )


def test_source(index: int) -> str:
    name = f"test_{index:03d}_case"
    return (
        HEADER
        + f"int {name}(int x) {{\n"
        + "    int total = x;\n"
        + c_loop(8)
        + c_io_block(f"test-{index:03d}")
        + "    return total;\n"
        + "}\n"
    )


def build_files() -> tuple[dict[str, str], dict[str, str]]:
    """Return (scanned sources, non-scanned extras)."""
    sources: dict[str, str] = {}
    sources["src/main.c"] = main_source()

    for module in range(MODULE_COUNT):
        sources[f"src/mod{module:02d}/entry.c"] = module_entry_source(module)
        for sub in range(SUBDIRS_PER_MODULE):
            for file_index in range(FILES_PER_SUBDIR):
                path = module_file_path(module, sub, file_index)
                if path == RUST_DYN_PATH:
                    sources[path] = rust_dyn_source()
                else:
                    sources[path] = module_c_source(module, sub, file_index)

    for index in range(TEST_FILE_COUNT):
        sources[f"tests/test_{index:03d}.c"] = test_source(index)

    extras = {
        "README.md": (
            "# Synth Atlas 3K\n\n"
            "A synthetic three-thousand-file C and Rust repository used to test "
            "Atlas scene aggregation at scale.\n"
        )
    }
    return sources, extras


def manifest_for(sources: dict[str, str]) -> dict[str, Any]:
    c_files = sum(1 for path in sources if path.endswith(".c"))
    rust_files = sum(1 for path in sources if path.endswith(".rs"))
    test_files = sum(1 for path in sources if path.startswith("tests/"))
    return {
        "fixture": "synth-atlas-3k",
        "manifest_version": 1,
        "source_files": len(sources),
        "c_files": c_files,
        "rust_files": rust_files,
        "production_files": len(sources) - test_files,
        "test_files": test_files,
        "modules": MODULE_COUNT,
        "module_files": MODULE_FILE_COUNT,
        "engineered": {
            "c_function_pointer": C_FN_POINTER_PATH,
            "rust_dyn": RUST_DYN_PATH,
        },
        "readme": "README.md",
    }


def is_owned_fixture(out: Path) -> bool:
    """Only an empty directory or a previous synth-atlas-3k fixture may be replaced."""
    if not out.exists():
        return True
    if not out.is_dir():
        return False
    manifest_path = out / "manifest.json"
    if not manifest_path.is_file():
        return not any(out.iterdir())
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return False
    return manifest.get("fixture") == "synth-atlas-3k"


def is_complete(out: Path) -> bool:
    manifest_path = out / "manifest.json"
    if not manifest_path.is_file() or not (out / "README.md").is_file():
        return False
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return False
    if manifest.get("source_files") != SOURCE_FILE_COUNT:
        return False
    actual = sum(
        1
        for path in out.rglob("*")
        if path.is_file() and path.suffix in {".c", ".rs"}
    )
    return actual == SOURCE_FILE_COUNT


def print_summary(out: Path, manifest: dict[str, Any], reused: bool) -> None:
    state = "reused" if reused else "generated"
    print(f"fixture={out}")
    print(f"status={state}")
    print(f"scanned_source_files={manifest['source_files']}")
    print(
        "source_mix="
        f"c:{manifest['c_files']} rust:{manifest['rust_files']} "
        f"production:{manifest['production_files']} tests:{manifest['test_files']}"
    )
    print(
        f"modules={manifest['modules']} module_files={manifest['module_files']} "
        "plus src/main.c and tests/"
    )
    engineered = manifest["engineered"]
    print(
        "engineered="
        f"c_fn_pointer:{engineered['c_function_pointer']} "
        f"rust_dyn:{engineered['rust_dyn']}"
    )
    print(f"manifest={out / 'manifest.json'}")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Generate the deterministic 3,000-source-file Atlas S4 fixture."
    )
    parser.add_argument("--out", type=Path, default=DEFAULT_OUT)
    parser.add_argument(
        "--force",
        action="store_true",
        help="remove an existing fixture directory and recreate it",
    )
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    out = args.out

    resolved = out.resolve()
    if resolved == Path("/") or resolved == Path.home():
        print(f"refusing unsafe --out path: {out}", file=sys.stderr)
        return 2

    if out.exists():
        if not args.force and is_complete(out):
            manifest = json.loads(
                (out / "manifest.json").read_text(encoding="utf-8")
            )
            print_summary(out, manifest, reused=True)
            return 0
        if not args.force:
            print(
                f"incomplete or missing fixture at {out}; pass --force to recreate",
                file=sys.stderr,
            )
            return 1
        if not is_owned_fixture(out):
            print(
                f"refusing to delete a non-fixture directory at {out}; "
                "choose a new --out or remove it yourself",
                file=sys.stderr,
            )
            return 2
        shutil.rmtree(out)
    out.mkdir(parents=True, exist_ok=True)

    sources, extras = build_files()

    # Gate: the fixture contract is exact, checked before anything is written.
    if len(sources) != SOURCE_FILE_COUNT:
        print(
            f"internal error: built {len(sources)} sources, expected {SOURCE_FILE_COUNT}",
            file=sys.stderr,
        )
        return 1

    for relative, content in sources.items():
        target = out / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8", newline="\n")

    for relative, content in extras.items():
        (out / relative).write_text(content, encoding="utf-8", newline="\n")

    manifest = manifest_for(sources)
    (out / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print_summary(out, manifest, reused=False)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
