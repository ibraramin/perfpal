//! Python backend + import-resolution integration test (plan `python-backend`,
//! phase W2). Exercises the real tree-sitter-python grammar through the public
//! seam (`scanner::scan`) over a temp package tree, plus ONE full-pipeline
//! smoke through `cli::run_with`.
//!
//! Deliberately does NOT validate the emitted document against the embedded
//! v1.2 JSON schema and does NOT assert `scan.languages`: the Python schema
//! token lands with the v1.3 bump in W3. Per-file `files[].language` is
//! already `"python"` (FileRecord contract), which is all this milestone locks.

use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;
use perfpal::ir::{IncludeForm, LoopKind, ParseStatus, StatementRole};
use perfpal::scanner::{FileUnit, scan};

/// `pkg/__init__.py` — re-export hub + a module-level global.
const INIT_PY: &str = "from .mod_b import Helper\n\nCOUNT = 0\n";

/// `pkg/mod_a.py` — every import form the resolver must handle, a top-level
/// `for` loop, a comprehension-bearing function, an `async def`, and a method
/// call.
const MOD_A_PY: &str = "\
import os
import pkg.mod_b
import pkg.doesnotexist
import pkg.stub
from . import mod_b
from .mod_b import helper
from pkg.mod_b import helper as h

for item in items_source():
    helper()


def compute(items):
    helper()
    return [transform(x) for x in build_items()]


def run_query(conn, q):
    return conn.execute(q)


async def fetch(url):
    return url
";

/// `pkg/mod_b.py` — the resolution target: a function + a class with a method.
const MOD_B_PY: &str = "\
def helper():
    return 1


class Helper:
    def method(self):
        return 2
";

/// `pkg/stub.pyi` — a stub that must still be discovered as `"python"`.
const STUB_PYI: &str = "def stubbed(x: int) -> int: ...\n";

/// A minimal notebook — must be IGNORED by discovery.
const NOTEBOOK: &str = r#"{"cells": [], "metadata": {}, "nbformat": 4, "nbformat_minor": 5}"#;

/// Malformed Python — forces `ParseStatus::Partial` with error regions.
const BROKEN_PY: &str = "def f(:\n  pass\n";

fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-python-itest-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp root");
    dir
}

/// Write the fixture package tree and return its root.
fn build_pkg_tree(tag: &str) -> PathBuf {
    let root = temp_root(tag);
    fs::create_dir_all(root.join("pkg")).expect("create pkg dir");
    fs::write(root.join("pkg/__init__.py"), INIT_PY).expect("write __init__.py");
    fs::write(root.join("pkg/mod_a.py"), MOD_A_PY).expect("write mod_a.py");
    fs::write(root.join("pkg/mod_b.py"), MOD_B_PY).expect("write mod_b.py");
    fs::write(root.join("pkg/stub.pyi"), STUB_PYI).expect("write stub.pyi");
    fs::write(root.join("notebook.ipynb"), NOTEBOOK).expect("write notebook.ipynb");
    fs::write(root.join("broken.py"), BROKEN_PY).expect("write broken.py");
    root
}

fn unit<'a>(units: &'a [FileUnit], path: &str) -> &'a FileUnit {
    units
        .iter()
        .find(|u| u.file.path == path)
        .unwrap_or_else(|| panic!("missing unit {path}"))
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

/// (1) Discovery: every `.py`/`.pyi` file present as a `"python"` unit, the
/// notebook excluded, emitted in sorted repo-relative order.
#[test]
fn discovery_selects_python_files_and_ignores_notebook() {
    let root = build_pkg_tree("discovery");
    let units = scan(&root).expect("scan must succeed");

    let paths: Vec<&str> = units.iter().map(|u| u.file.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "broken.py",
            "pkg/__init__.py",
            "pkg/mod_a.py",
            "pkg/mod_b.py",
            "pkg/stub.pyi",
        ],
        "only .py/.pyi discovered; .ipynb ignored; sorted emission"
    );
    assert!(
        paths.windows(2).all(|w| w[0] <= w[1]),
        "emission must be lexicographically sorted"
    );
    assert!(
        units.iter().all(|u| u.file.language == "python"),
        "every discovered unit is language \"python\""
    );

    fs::remove_dir_all(&root).ok();
}

/// (2) The synthetic `<module>` record is functions[0] for EVERY Python unit
/// (`is_module_scope = true`), and a module-level `for` loop lands in its
/// `loops` at depth ≥ 1.
#[test]
fn synthetic_module_record_captures_top_level_loop() {
    let root = build_pkg_tree("module-record");
    let units = scan(&root).expect("scan must succeed");

    for u in &units {
        let first = u.functions.first().expect("every unit emits a record");
        assert_eq!(first.record.name, "<module>", "on {}", u.file.path);
        assert!(first.record.is_module_scope, "on {}", u.file.path);
    }

    let a = unit(&units, "pkg/mod_a.py");
    let module = &a.functions[0];
    assert!(
        module
            .loops
            .iter()
            .any(|l| l.kind == LoopKind::For && l.depth >= 1),
        "top-level `for item in items_source():` must be a depth≥1 loop in <module>"
    );

    fs::remove_dir_all(&root).ok();
}

/// (3) A list comprehension is a real `LoopNode` in its enclosing function, and
/// its iterable is evaluated ONCE (outer depth, not in-loop) while the
/// comprehension body IS in-loop.
#[test]
fn comprehension_is_a_loop_and_iterable_evaluated_once() {
    let root = build_pkg_tree("comprehension");
    let units = scan(&root).expect("scan must succeed");
    let a = unit(&units, "pkg/mod_a.py");
    let compute = a
        .functions
        .iter()
        .find(|f| f.record.name == "compute")
        .expect("compute unit");

    assert_eq!(
        compute.loops.len(),
        1,
        "the list comprehension is the function's only loop"
    );
    let comp = &compute.loops[0];
    assert_eq!(comp.kind, LoopKind::For);
    assert_eq!(comp.depth, 1);

    // Iterable `build_items()`: evaluated at the clause's OUTER depth (0) and
    // not a loop-body statement.
    let build = compute
        .calls
        .iter()
        .find(|c| c.callee_name == "build_items")
        .expect("build_items call");
    assert_eq!(build.meta.in_loop_depth, 0, "iterable once-evaluated");
    assert_eq!(build.statement_role, StatementRole::Unknown);

    // Body `transform(x)`: inside the comprehension loop.
    let transform = compute
        .calls
        .iter()
        .find(|c| c.callee_name == "transform")
        .expect("transform call");
    assert_eq!(transform.meta.in_loop_depth, 1, "body is in-loop");

    fs::remove_dir_all(&root).ok();
}

/// (4) Python import resolution over `pkg/mod_a.py` (W2 rules): external
/// absolute kept raw, absolute internal hit resolved, relative hit resolved,
/// internal miss normalized to null; all Python edges are `Quoted`.
#[test]
fn python_import_edges_resolve() {
    let root = build_pkg_tree("imports");
    let units = scan(&root).expect("scan must succeed");

    // Every Python edge is Quoted (there is no angle form in Python).
    for u in &units {
        assert!(
            u.includes.iter().all(|e| e.form == IncludeForm::Quoted),
            "all edges Quoted on {}",
            u.file.path
        );
    }

    let a = unit(&units, "pkg/mod_a.py");

    // External absolute: `import os` → raw text kept, unresolved.
    let os = a
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("os"))
        .expect("external os edge keeps raw text");
    assert!(
        !os.resolved,
        "unknown top-level package stays in the external bucket"
    );

    // Absolute internal hit: `import pkg.mod_b` (and the aliased from-import)
    // resolve to the owning file.
    assert!(
        a.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("pkg/mod_b.py")),
        "pkg.mod_b resolves to pkg/mod_b.py"
    );

    // BOTH relative forms (`.mod_b`) plus BOTH absolute forms resolve to the
    // same file → exactly 4 resolved edges. Relative-only resolution would
    // leave this at 2, so the count locks the relative path too.
    let resolved_b = a
        .includes
        .iter()
        .filter(|e| e.resolved && e.to_path.as_deref() == Some("pkg/mod_b.py"))
        .count();
    assert_eq!(
        resolved_b, 4,
        "2 absolute (`import pkg.mod_b`, `from pkg.mod_b import helper as h`) \
         + 2 relative (`from . import mod_b`, `from .mod_b import helper`)"
    );

    // Internal absolute miss: `import pkg.doesnotexist` → internal unresolved,
    // normalized to null (no raw text).
    let misses: Vec<_> = a
        .includes
        .iter()
        .filter(|e| !e.resolved && e.to_path.is_none())
        .collect();
    assert_eq!(
        misses.len(),
        1,
        "only pkg.doesnotexist is an internal unresolved edge"
    );

    // Relative resolution is also exercised from the re-export hub.
    let init = unit(&units, "pkg/__init__.py");
    assert!(
        init.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("pkg/mod_b.py")),
        "from .mod_b import Helper resolves in pkg/__init__.py"
    );

    // `.pyi` resolution alignment (FIX 5): `import pkg.stub` resolves to the
    // stub-only file `pkg/stub.pyi` (module index includes `.pyi` units).
    assert!(
        a.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("pkg/stub.pyi")),
        "pkg.stub resolves to the stub-only pkg/stub.pyi"
    );

    fs::remove_dir_all(&root).ok();
}

/// The method call `conn.execute(q)` is a Direct callee with receiver root
/// `conn` (documented Python IR shape).
#[test]
fn method_call_records_receiver_root() {
    let root = build_pkg_tree("method-call");
    let units = scan(&root).expect("scan must succeed");
    let a = unit(&units, "pkg/mod_a.py");
    let run_query = a
        .functions
        .iter()
        .find(|f| f.record.name == "run_query")
        .expect("run_query unit");
    let exec = run_query
        .calls
        .iter()
        .find(|c| c.callee_name == "execute")
        .expect("conn.execute call");
    assert_eq!(exec.receiver_root.as_deref(), Some("conn"));

    fs::remove_dir_all(&root).ok();
}

/// (5) Malformed source → `Partial` health with non-empty error regions.
#[test]
fn malformed_python_yields_partial_health() {
    let root = build_pkg_tree("partial");
    let units = scan(&root).expect("scan must succeed");
    let broken = unit(&units, "broken.py");
    assert_eq!(broken.parse_health.status, ParseStatus::Partial);
    assert!(
        !broken.parse_health.error_regions.is_empty(),
        "partial parse must record error regions"
    );

    fs::remove_dir_all(&root).ok();
}

/// (6) A `.pyi` stub is discovered as a `"python"` unit.
#[test]
fn stub_file_is_discovered_as_python() {
    let root = build_pkg_tree("stub");
    let units = scan(&root).expect("scan must succeed");
    let stub = unit(&units, "pkg/stub.pyi");
    assert_eq!(stub.file.language, "python");

    fs::remove_dir_all(&root).ok();
}

/// (7) Full-pipeline smoke: `cli::run_with` exits 0 over a `.py` tree and the
/// emitted metrics.json carries a `"python"` file. NOT schema-validated (the
/// v1.3 Python token lands in W3) and `scan.languages` is intentionally not
/// asserted here.
#[test]
fn full_pipeline_smoke_emits_python_file() {
    let root = temp_root("pipeline");
    fs::write(
        root.join("app.py"),
        "import os\n\nfor i in data():\n    consume(i)\n",
    )
    .expect("write app.py");
    let out = temp_root("pipeline-out").join("perfpal-report");

    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        root.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "pipeline over a .py tree must exit 0");

    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("metrics.json parses");

    let files = doc["files"].as_array().expect("files[] present");
    assert!(
        files.iter().any(|f| f["language"] == "python"),
        "at least one files[].language == \"python\""
    );

    fs::remove_dir_all(&root).ok();
    fs::remove_dir_all(out.parent().expect("output parent")).ok();
}
