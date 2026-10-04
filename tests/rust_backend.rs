//! Rust backend + language-dispatch integration test (plan `rust-backend`,
//! phase R1). Exercises the real grammar through the public seam
//! (`scanner::backends::tree_sitter_rust::extract`) and the extension-based
//! dispatch of `scanner::scan` over a temp directory.
//!
//! Deliberately does NOT run the schema/writer path over Rust: R1 stopped at
//! the scanner seam. That restriction is historical — R2 has since widened
//! the schema (`scan.languages` / `files[].language` admit `"rust"`), and the
//! schema/writer path over Rust is locked by `tests/schema_1_3.rs`.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::ir::{CalleeKind, IncludeForm, MacroEventKind, ParseStatus};
use perfpal::scanner::backends::tree_sitter_rust;
use perfpal::scanner::scan;

const SAMPLE: &str = r#"
use std::collections::HashMap;
use crate::util::{helper, Config};

mod inline_mod {
    pub fn nested() -> i32 { 1 }
}

mod external_mod;

const LIMIT: usize = 16;
static mut COUNTER: i64 = 0;

struct Point {
    x: i64,
    y: i64,
}

struct Pair(u32, i64);

macro_rules! twice {
    ($x:expr) => { $x * 2 };
}

impl Point {
    fn sum(&self) -> i64 {
        self.x + self.y
    }
}

trait Shape {
    fn area(&self) -> i64;
    fn label(&self) -> &'static str { "shape" }
}

fn process(p: &Point, n: i32, items: &[i32]) -> i32 {
    let mut total = 0;
    for value in items {
        total += value;
    }
    while total < n {
        total = total + 1;
    }
    loop {
        total += 1;
        if total > 100 { break; }
    }
    let doubled = twice!(total);
    let s = p.sum();
    log_value(doubled, s);
    total
}
"#;

fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-rust-itest-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp root");
    dir
}

/// (1) file-level records + function/method extraction through the real
/// grammar.
#[test]
fn extracts_file_level_records_and_functions() {
    let ex = tree_sitter_rust::extract("src/sample.rs", SAMPLE.as_bytes());
    assert_eq!(ex.file.language, "rust");
    assert_eq!(ex.parse_health.status, ParseStatus::Ok);

    // Functions: nested (inline mod), sum (impl), label (trait default),
    // process. The body-less `area` signature item is skipped (D5).
    let names: Vec<&str> = ex
        .functions
        .iter()
        .map(|f| f.record.name.as_str())
        .collect();
    assert_eq!(names, ["nested", "sum", "label", "process"]);
    assert_eq!(ex.file.function_count, 4);

    // Globals.
    let globals: Vec<&str> = ex.global_vars.iter().map(|g| g.name.as_str()).collect();
    assert_eq!(globals, ["LIMIT", "COUNTER"]);

    // Include edges (D1): internal module paths quoted, extern/std angle.
    let mangle = ex
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("std::collections::HashMap"))
        .expect("angle std use");
    assert_eq!(mangle.form, IncludeForm::Angle);
    let internal = ex
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("crate::util::helper"))
        .expect("quoted crate use");
    assert_eq!(internal.form, IncludeForm::Quoted);
    assert_eq!(ex.file.include_counts.quoted, 3);
    assert_eq!(ex.file.include_counts.angle, 1);

    // Structs: named + tuple positional.
    let pair = ex.structs.iter().find(|s| s.name == "Pair").unwrap();
    assert_eq!(pair.members[0].name, "0");
    assert_eq!(pair.members[0].type_text, "u32");

    // Macros: definition + use.
    let twice_use = ex
        .macros
        .iter()
        .find(|m| m.name == "twice" && m.kind == MacroEventKind::Use)
        .expect("macro use event");
    assert!(twice_use.span.start_line > 0);

    // Function metrics + events.
    let process = ex
        .functions
        .iter()
        .find(|f| f.record.name == "process")
        .unwrap();
    assert_eq!(process.record.param_count, 3);
    assert_eq!(process.record.cyclomatic_complexity, 5);
    assert_eq!(process.record.max_loop_depth, 1);
    assert_eq!(process.record.alloc_site_count, 0);
    assert_eq!(process.loops.len(), 3);
    assert!(
        process
            .calls
            .iter()
            .any(|c| c.callee_name == "sum" && c.callee_kind == CalleeKind::Direct)
    );
}

/// (2) attribute-sibling lock: `#[cfg]`/`#[inline]` are ordinary siblings, so
/// the annotated items still extract (the C guarded-content defect does not
/// recur in Rust).
#[test]
fn attribute_siblings_do_not_suppress_annotated_items() {
    let src = concat!(
        "#[cfg(unix)]\n",
        "#[inline]\n",
        "fn guarded(n: i32) -> i32 { n + 1 }\n",
        "\n",
        "#[cfg(not(unix))]\n",
        "fn other() -> i32 { 0 }\n",
    );
    let ex = tree_sitter_rust::extract("attrs.rs", src.as_bytes());
    let names: Vec<&str> = ex
        .functions
        .iter()
        .map(|f| f.record.name.as_str())
        .collect();
    assert_eq!(names, ["guarded", "other"]);
    assert_eq!(ex.parse_health.status, ParseStatus::Ok);
}

/// (3) ParseHealth partial on malformed source, with error regions.
#[test]
fn malformed_source_yields_partial_health() {
    let ex = tree_sitter_rust::extract("bad.rs", b"fn broken( {");
    assert_eq!(ex.parse_health.status, ParseStatus::Partial);
    assert!(ex.parse_health.error_node_count >= 1);
    assert!(!ex.parse_health.error_regions.is_empty());
    assert_eq!(ex.file.language, "rust");
}

/// (5) Determinism: the same bytes yield an identical `Extraction`.
#[test]
fn extraction_is_deterministic_for_identical_bytes() {
    let a = tree_sitter_rust::extract("d.rs", SAMPLE.as_bytes());
    let b = tree_sitter_rust::extract("d.rs", SAMPLE.as_bytes());
    assert_eq!(a, b);
}

/// (4) Extension-based dispatch: discovery keeps `.c`/`.h`/`.rs`, skips `.txt`,
/// records the right `FileRecord.language`, and emits lexicographically sorted.
#[test]
fn dispatch_selects_language_by_extension_and_sorts() {
    let root = temp_root("dispatch");
    fs::write(root.join("a.c"), "int a(void) { return 0; }\n").unwrap();
    fs::write(root.join("b.h"), "int b(int);\n").unwrap();
    fs::write(root.join("c.rs"), "fn c() {}\n").unwrap();
    fs::write(root.join("d.txt"), "not source\n").unwrap();

    let units = scan(&root).expect("scan must succeed");
    let paths: Vec<&str> = units.iter().map(|u| u.file.path.as_str()).collect();
    assert_eq!(
        paths,
        ["a.c", "b.h", "c.rs"],
        ".txt skipped, sorted emission"
    );
    assert!(paths.windows(2).all(|w| w[0] <= w[1]));

    let lang_of = |p: &str| {
        units
            .iter()
            .find(|u| u.file.path == p)
            .map(|u| u.file.language.as_str())
            .unwrap()
    };
    assert_eq!(lang_of("a.c"), "c");
    assert_eq!(lang_of("b.h"), "c");
    assert_eq!(lang_of("c.rs"), "rust");

    fs::remove_dir_all(&root).ok();
}

/// Rust module-edge resolution (R1 `resolve_rust_edges`, reviews m2 + N1):
/// module-dir-relative. For a crate root (`src/lib.rs`) the module dir is the
/// containing dir, so `mod util;` resolves to `src/util.rs` / `src/util/mod.rs`.
/// A miss yields `to_path=None`, `resolved=false`. `use` paths (containing
/// `::`) and extern/angle edges stay unresolved with their raw text kept.
#[test]
fn rust_module_edges_resolve_module_dir_relative_and_misses_are_null() {
    let root = temp_root("resolve");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "mod util;\nmod pkg;\nmod missing;\nuse std::collections;\nfn lib_fn() {}\n",
    )
    .unwrap();
    fs::write(root.join("src/util.rs"), "pub fn u() {}\n").unwrap();
    fs::create_dir_all(root.join("src/pkg")).unwrap();
    fs::write(root.join("src/pkg/mod.rs"), "pub fn p() {}\n").unwrap();

    let units = scan(&root).expect("scan must succeed");
    let lib = units
        .iter()
        .find(|u| u.file.path == "src/lib.rs")
        .expect("lib unit");

    let util = lib
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("src/util.rs"))
        .expect("mod util resolved relative to the crate-root module dir");
    assert!(util.resolved);

    let pkg = lib
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("src/pkg/mod.rs"))
        .expect("mod pkg resolved to src/pkg/mod.rs");
    assert!(pkg.resolved);

    // Miss: `mod missing;` has no file → null + unresolved (data, not error).
    let missing = lib
        .includes
        .iter()
        .find(|e| e.form == IncludeForm::Quoted && e.to_path.is_none())
        .expect("unresolvable quoted mod edge is null");
    assert!(!missing.resolved);

    // `use std::…` is an Angle edge: raw text kept, never resolved.
    let angle = lib
        .includes
        .iter()
        .find(|e| e.form == IncludeForm::Angle)
        .expect("angle use edge");
    assert_eq!(angle.to_path.as_deref(), Some("std::collections"));
    assert!(!angle.resolved);

    fs::remove_dir_all(&root).ok();
}

/// N1/N2 discriminator: module resolution is module-DIR-relative, not
/// file-dir-relative. `src/a.rs` + `mod b;` must resolve `src/a/b.rs` (a bare
/// `src/b.rs` is NOT a valid module location — rustc error[E0583]) and must NOT
/// false-resolve to `src/b.rs`.
#[test]
fn rust_module_resolution_is_module_dir_relative() {
    // Hit: src/a.rs + mod b; with src/a/b.rs.
    let root = temp_root("moddir-hit");
    fs::create_dir_all(root.join("src/a")).unwrap();
    fs::write(root.join("src/a.rs"), "mod b;\nfn a() {}\n").unwrap();
    fs::write(root.join("src/a/b.rs"), "pub fn b() {}\n").unwrap();

    let units = scan(&root).expect("scan must succeed");
    let a = units
        .iter()
        .find(|u| u.file.path == "src/a.rs")
        .expect("a.rs unit");
    let hit = a
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("src/a/b.rs"))
        .expect("mod b resolves to the module dir src/a/b.rs");
    assert!(hit.resolved);
    fs::remove_dir_all(&root).ok();

    // Miss: src/a.rs + mod b; with ONLY src/b.rs (invalid location) → not resolved.
    let root2 = temp_root("moddir-miss");
    fs::create_dir_all(root2.join("src")).unwrap();
    fs::write(root2.join("src/a.rs"), "mod b;\nfn a() {}\n").unwrap();
    fs::write(root2.join("src/b.rs"), "pub fn b() {}\n").unwrap();

    let units2 = scan(&root2).expect("scan must succeed");
    let a2 = units2
        .iter()
        .find(|u| u.file.path == "src/a.rs")
        .expect("a.rs unit");
    let edge = a2
        .includes
        .iter()
        .find(|e| e.form == IncludeForm::Quoted)
        .expect("mod b edge");
    assert_eq!(
        edge.to_path, None,
        "src/b.rs is not a valid module location for src/a.rs"
    );
    assert!(!edge.resolved);
    fs::remove_dir_all(&root2).ok();
}

/// Round-3: Cargo crate-root files resolve submodules in their CONTAINING dir,
/// not `<dir>/<stem>/`. `tests/x.rs` + `mod common;` lives in `tests/common/`
/// (rustc-verified: `tests/x.rs` + `tests/common/mod.rs` compiles).
#[test]
fn crate_root_in_tests_dir_resolves_submodule_in_containing_dir() {
    let root = temp_root("crate-root-tests");
    fs::create_dir_all(root.join("tests/common")).unwrap();
    fs::write(root.join("tests/x.rs"), "mod common;\nfn x() {}\n").unwrap();
    fs::write(root.join("tests/common/mod.rs"), "pub fn c() {}\n").unwrap();

    let units = scan(&root).expect("scan must succeed");
    let x = units
        .iter()
        .find(|u| u.file.path == "tests/x.rs")
        .expect("tests/x.rs unit");
    let common = x
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("tests/common/mod.rs"))
        .expect("crate-root tests/x.rs resolves mod common to tests/common/mod.rs");
    assert!(common.resolved);

    fs::remove_dir_all(&root).ok();
}

/// Round-3: `src/bin` is a Cargo crate-root directory — `src/bin/foo.rs` +
/// `mod bar;` resolves `src/bin/bar.rs` (NOT `src/bin/foo/bar.rs`; rustc
/// verified `src/bin/foo.rs` + `src/bin/bar.rs` compiles, `src/bin/foo/bar.rs`
/// is error[E0583]).
#[test]
fn crate_root_in_src_bin_resolves_submodule_in_containing_dir() {
    let root = temp_root("crate-root-src-bin");
    fs::create_dir_all(root.join("src/bin")).unwrap();
    fs::write(root.join("src/bin/foo.rs"), "mod bar;\nfn foo() {}\n").unwrap();
    fs::write(root.join("src/bin/bar.rs"), "pub fn b() {}\n").unwrap();

    let units = scan(&root).expect("scan must succeed");
    let foo = units
        .iter()
        .find(|u| u.file.path == "src/bin/foo.rs")
        .expect("src/bin/foo.rs unit");
    let bar = foo
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("src/bin/bar.rs"))
        .expect("src/bin/foo.rs resolves mod bar to src/bin/bar.rs");
    assert!(bar.resolved);

    fs::remove_dir_all(&root).ok();
}

/// Round-4 Major 1: crate-root dirs are matched by TRAILING SEGMENT, so nested
/// workspace packages (`crates/foo/src/bin`, `crates/foo/tests`) resolve too.
#[test]
fn crate_root_dirs_match_nested_workspace_packages() {
    let root = temp_root("nested-pkg");
    fs::create_dir_all(root.join("crates/foo/tests/common")).unwrap();
    fs::write(
        root.join("crates/foo/tests/x.rs"),
        "mod common;\nfn x() {}\n",
    )
    .unwrap();
    fs::write(
        root.join("crates/foo/tests/common/mod.rs"),
        "pub fn c() {}\n",
    )
    .unwrap();

    fs::create_dir_all(root.join("crates/foo/src/bin")).unwrap();
    fs::write(
        root.join("crates/foo/src/bin/bar.rs"),
        "mod baz;\nfn bar() {}\n",
    )
    .unwrap();
    fs::write(root.join("crates/foo/src/bin/baz.rs"), "pub fn z() {}\n").unwrap();

    let units = scan(&root).expect("scan must succeed");

    let x = units
        .iter()
        .find(|u| u.file.path == "crates/foo/tests/x.rs")
        .expect("tests/x.rs unit");
    let common = x
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("crates/foo/tests/common/mod.rs"))
        .expect("nested-package tests crate root resolves mod common");
    assert!(common.resolved);

    let bar = units
        .iter()
        .find(|u| u.file.path == "crates/foo/src/bin/bar.rs")
        .expect("src/bin/bar.rs unit");
    let baz = bar
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("crates/foo/src/bin/baz.rs"))
        .expect("nested-package src/bin crate root resolves mod baz");
    assert!(baz.resolved);

    fs::remove_dir_all(&root).ok();
}

/// Round-4 Major 1 negative discriminator: `src/bin/foo.rs` + `mod bar;` in
/// `src/bin/foo/bar.rs` must stay UNRESOLVED (rustc error[E0583] — that is not
/// a valid module location for a crate-root binary file).
#[test]
fn crate_root_src_bin_does_not_resolve_under_stem_dir() {
    let root = temp_root("src-bin-neg");
    fs::create_dir_all(root.join("src/bin/foo")).unwrap();
    fs::write(root.join("src/bin/foo.rs"), "mod bar;\nfn foo() {}\n").unwrap();
    fs::write(root.join("src/bin/foo/bar.rs"), "pub fn b() {}\n").unwrap();

    let units = scan(&root).expect("scan must succeed");
    let foo = units
        .iter()
        .find(|u| u.file.path == "src/bin/foo.rs")
        .expect("src/bin/foo.rs unit");
    let edge = foo
        .includes
        .iter()
        .find(|e| e.form == IncludeForm::Quoted)
        .expect("mod bar edge");
    assert_eq!(
        edge.to_path, None,
        "src/bin/foo/bar.rs is E0583; src/bin/bar.rs is the only valid location"
    );
    assert!(!edge.resolved);

    fs::remove_dir_all(&root).ok();
}

/// R3 supersedes the Round-4 Major 2 founder decision: internal `use` edges now
/// resolve to the file owning the longest module prefix (`crate::util::helper`
/// → `src/util.rs`), and an unresolvable internal `use` normalizes to
/// `to_path=None, resolved=false` (no raw `::` retention).
#[test]
fn internal_use_edges_resolve_to_owning_module_file() {
    let root = temp_root("use-resolve");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "mod util;\nuse crate::util::helper;\nuse crate::nope::missing;\nfn lib_fn() {}\n",
    )
    .unwrap();
    fs::write(root.join("src/util.rs"), "pub fn helper() {}\n").unwrap();

    let units = scan(&root).expect("scan must succeed");
    let lib = units
        .iter()
        .find(|u| u.file.path == "src/lib.rs")
        .expect("lib unit");

    // `use crate::util::helper` → the file owning module `util`.
    let util = lib
        .includes
        .iter()
        .find(|e| e.resolved && e.to_path.as_deref() == Some("src/util.rs"))
        .expect("crate::util::helper resolves to src/util.rs");
    assert_eq!(util.form, IncludeForm::Quoted);

    // Unresolvable internal use normalizes to null + unresolved.
    assert!(
        lib.includes
            .iter()
            .any(|e| e.form == IncludeForm::Quoted && !e.resolved && e.to_path.is_none()),
        "crate::nope::missing normalizes to to_path=None, resolved=false"
    );

    fs::remove_dir_all(&root).ok();
}

/// Round-4 Minor 1: an inline module carries a `/`-joined prefix, so
/// `mod outer { mod inner; }` in `src/lib.rs` resolves `src/outer/inner.rs`
/// (and a bare `src/inner.rs` does NOT satisfy it).
#[test]
fn inline_module_path_prefix_composes_for_mod_edges() {
    let root = temp_root("inline-prefix");
    fs::create_dir_all(root.join("src/outer")).unwrap();
    fs::write(root.join("src/lib.rs"), "mod outer { mod inner; }\n").unwrap();
    fs::write(root.join("src/outer/inner.rs"), "pub fn i() {}\n").unwrap();

    let units = scan(&root).expect("scan must succeed");
    let lib = units
        .iter()
        .find(|u| u.file.path == "src/lib.rs")
        .expect("lib unit");
    let inner = lib
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("src/outer/inner.rs"))
        .expect("inline module prefix resolves src/outer/inner.rs");
    assert!(inner.resolved);

    // Negative: `src/inner.rs` alone is not where `mod outer { mod inner; }` lives.
    let root2 = temp_root("inline-prefix-miss");
    fs::create_dir_all(root2.join("src")).unwrap();
    fs::write(root2.join("src/lib.rs"), "mod outer { mod inner; }\n").unwrap();
    fs::write(root2.join("src/inner.rs"), "pub fn i() {}\n").unwrap();

    let units2 = scan(&root2).expect("scan must succeed");
    let lib2 = units2
        .iter()
        .find(|u| u.file.path == "src/lib.rs")
        .expect("lib unit");
    let edge2 = lib2
        .includes
        .iter()
        .find(|e| e.form == IncludeForm::Quoted)
        .expect("mod inner edge");
    assert_eq!(
        edge2.to_path, None,
        "src/inner.rs is not a valid location for mod outer {{ mod inner; }}"
    );
    assert!(!edge2.resolved);

    fs::remove_dir_all(&root).ok();
    fs::remove_dir_all(&root2).ok();
}
