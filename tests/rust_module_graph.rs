//! Rust module-graph resolution integration tests (plan `rust-backend`, phase
//! R3 workstream B). Drives the real scanner (`perfpal::scanner::scan`) over
//! on-disk fixture trees in a tempdir and asserts FILE-LEVEL resolution of
//! internal `use` edges against a module-path → file map built from resolved
//! `mod` edges, plus the Cargo.toml-aware crate-root fix and an ARCH-001
//! end-to-end cycle.
//!
//! Every fixture writes a `Cargo.toml` where package-aware crate-root
//! detection is under test; the manifest-free fallback is exercised by the
//! existing `tests/rust_backend.rs` crate-root tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::analyze;
use perfpal::analyzer::finding::RS_ARCH_001_RULE_ID;
use perfpal::ir::IncludeForm;
use perfpal::scanner::{FileUnit, scan};

fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-rust-mg-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp root");
    dir
}

/// Write `contents` to `root/rel`, creating parent directories.
fn write_file(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write fixture file");
}

fn manifest(root: &Path) {
    write_file(
        root,
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    );
}

fn unit<'a>(units: &'a [FileUnit], path: &str) -> &'a FileUnit {
    units
        .iter()
        .find(|u| u.file.path == path)
        .unwrap_or_else(|| panic!("unit {path} present"))
}

/// (1) `use crate::a::b::Thing` resolves to the file owning module `a::b`;
/// `use crate::a::Thing` resolves to the module file `a.rs`. Resolution takes
/// the LONGEST module prefix (the last segment is an imported item).
#[test]
fn crate_use_resolves_to_longest_module_prefix_file() {
    let root = temp_root("crate-use");
    manifest(&root);
    write_file(&root, "src/lib.rs", "mod a;\nmod c;\n");
    write_file(&root, "src/a.rs", "mod b;\npub struct Other;\n");
    write_file(&root, "src/a/b.rs", "pub struct Thing;\n");
    write_file(
        &root,
        "src/c.rs",
        "use crate::a::b::Thing;\nuse crate::a::Other;\n",
    );

    let units = scan(&root).expect("scan must succeed");
    let c = unit(&units, "src/c.rs");

    // Deep path → the nested module FILE a/b.rs.
    let deep = c
        .includes
        .iter()
        .find(|e| e.resolved && e.to_path.as_deref() == Some("src/a/b.rs"))
        .expect("crate::a::b::Thing resolves to src/a/b.rs");
    assert_eq!(deep.form, IncludeForm::Quoted);

    // Shallow path → the owning module file a.rs (the item lives there).
    assert!(
        c.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("src/a.rs")),
        "crate::a::Other resolves to src/a.rs"
    );

    fs::remove_dir_all(&root).ok();
}

/// (2) `self::` resolves relative to the source module and `super::` pops one
/// module (repeated `super`s pop repeatedly). `src/a/b.rs` is module `a::b`, so
/// `self::c` → `a::b::c` and `super::d` → `a::d`.
#[test]
fn self_and_super_use_resolve_relative_to_source_module() {
    let root = temp_root("self-super");
    manifest(&root);
    write_file(&root, "src/lib.rs", "mod a;\n");
    write_file(&root, "src/a/mod.rs", "mod b;\nmod d;\n");
    write_file(&root, "src/a/b.rs", "mod c;\nuse self::c;\nuse super::d;\n");
    write_file(&root, "src/a/b/c.rs", "pub fn c() {}\n");
    write_file(&root, "src/a/d.rs", "pub fn d() {}\n");

    let units = scan(&root).expect("scan must succeed");
    let b = unit(&units, "src/a/b.rs");

    // `self::c` resolves to a/b/c.rs. That file also has the `mod c;` edge, so
    // exactly TWO edges to it proves the `self::` use resolved as well.
    let self_hits = b
        .includes
        .iter()
        .filter(|e| e.resolved && e.to_path.as_deref() == Some("src/a/b/c.rs"))
        .count();
    assert_eq!(
        self_hits, 2,
        "mod c + `use self::c` both resolve to src/a/b/c.rs"
    );

    // `super::d` resolves to the parent module's file a/d.rs.
    assert!(
        b.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("src/a/d.rs")),
        "use super::d resolves to src/a/d.rs"
    );

    fs::remove_dir_all(&root).ok();
}

/// (3) `use crate::a::{B, C};` fans out into two resolved edges (one per leaf
/// path), both pointing at the file owning module `a` (`a/mod.rs`).
///
/// Note: a BARE `use a::{B, C}` is classified `Angle` (external crate) by the
/// backend's D1 form rule, so it is deliberately NOT resolved here; the
/// crate-qualified form is the internal one.
#[test]
fn use_list_fan_out_produces_multiple_resolved_edges() {
    let root = temp_root("fan-out");
    manifest(&root);
    write_file(&root, "src/lib.rs", "mod a;\nmod user;\n");
    write_file(&root, "src/a/mod.rs", "pub struct B;\npub struct C;\n");
    write_file(&root, "src/user.rs", "use crate::a::{B, C};\n");

    let units = scan(&root).expect("scan must succeed");
    let user = unit(&units, "src/user.rs");

    let resolved: Vec<&str> = user
        .includes
        .iter()
        .filter(|e| e.resolved)
        .filter_map(|e| e.to_path.as_deref())
        .collect();
    assert_eq!(
        resolved,
        ["src/a/mod.rs", "src/a/mod.rs"],
        "both use-list leaves resolve to the owning module file"
    );

    fs::remove_dir_all(&root).ok();
}

/// (4) An unresolvable internal `use crate::nope;` normalizes to
/// `to_path=None, resolved=false` (the unresolved-`Quoted` IR contract; no raw
/// `::` retention).
#[test]
fn unresolvable_internal_use_normalizes_to_null() {
    let root = temp_root("unresolved");
    manifest(&root);
    write_file(&root, "src/lib.rs", "mod user;\n");
    write_file(&root, "src/user.rs", "use crate::nope;\n");

    let units = scan(&root).expect("scan must succeed");
    let user = unit(&units, "src/user.rs");

    let edge = user
        .includes
        .iter()
        .find(|e| e.form == IncludeForm::Quoted)
        .expect("internal use edge");
    assert_eq!(edge.to_path, None, "unresolved internal use is normalized");
    assert!(!edge.resolved);

    fs::remove_dir_all(&root).ok();
}

/// (5) An inline `mod outer { mod inner; }` keeps its `/`-joined raw and
/// resolves `src/outer/inner.rs`; a `use` through that inline path also lands
/// on the backing file (inline modules have no file of their own).
#[test]
fn inline_module_prefix_resolves_and_use_through_it_lands_on_the_file() {
    let root = temp_root("inline");
    manifest(&root);
    write_file(
        &root,
        "src/lib.rs",
        "mod outer { mod inner; }\nuse crate::outer::inner::Thing;\n",
    );
    write_file(&root, "src/outer/inner.rs", "pub struct Thing;\n");

    let units = scan(&root).expect("scan must succeed");
    let lib = unit(&units, "src/lib.rs");

    let hits = lib
        .includes
        .iter()
        .filter(|e| e.resolved && e.to_path.as_deref() == Some("src/outer/inner.rs"))
        .count();
    assert_eq!(
        hits, 2,
        "the `/`-raw `mod outer {{ mod inner; }}` and the use both resolve there"
    );

    fs::remove_dir_all(&root).ok();
}

/// (6) Cargo.toml-aware crate roots: a module directory literally named
/// `src/tests/` is NOT a crate root (so `src/tests/unit.rs` owns the
/// `src/tests/unit/` subdirectory), while a real `<pkg>/tests/foo.rs`
/// integration-test target IS a crate root.
#[test]
fn package_aware_crate_root_ignores_module_dir_named_tests() {
    let root = temp_root("crate-root");
    manifest(&root);
    // Module tree under src/tests/ that must NOT be treated as a test target
    // dir: `src/tests/unit.rs` is module `tests::unit`.
    write_file(&root, "src/lib.rs", "mod tests;\n");
    write_file(&root, "src/tests/mod.rs", "mod unit;\n");
    write_file(&root, "src/tests/unit.rs", "mod inner;\n");
    write_file(&root, "src/tests/unit/inner.rs", "pub fn i() {}\n");
    // A REAL integration-test target: tests/foo.rs is a crate root, so its
    // `mod bar;` resolves tests/bar.rs.
    write_file(&root, "tests/foo.rs", "mod bar;\n");
    write_file(&root, "tests/bar.rs", "pub fn b() {}\n");

    let units = scan(&root).expect("scan must succeed");

    let unit_file = unit(&units, "src/tests/unit.rs");
    assert!(
        unit_file
            .includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("src/tests/unit/inner.rs")),
        "src/tests/unit.rs resolves mod inner to its OWN subdirectory"
    );
    assert!(
        !unit_file
            .includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("src/tests/inner.rs")),
        "src/tests is not a crate root, so src/tests/inner.rs is not a candidate"
    );

    let foo = unit(&units, "tests/foo.rs");
    assert!(
        foo.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("tests/bar.rs")),
        "the real tests/ target dir is still a crate root"
    );

    fs::remove_dir_all(&root).ok();
}

/// (7) RS-ARCH-001 end-to-end: a two-file internal `use` cycle
/// (`src/a.rs` ↔ `src/b.rs`) yields a cycle finding. C rules are
/// language-gated off Rust files, so this is the Rust rule id
/// (`PERF-RS-ARCH-001`).
#[test]
fn two_file_use_cycle_yields_arch_001_finding() {
    let root = temp_root("cycle");
    manifest(&root);
    write_file(&root, "src/lib.rs", "mod a;\nmod b;\n");
    write_file(&root, "src/a.rs", "use crate::b::B;\npub struct A;\n");
    write_file(&root, "src/b.rs", "use crate::a::A;\npub struct B;\n");

    let units = scan(&root).expect("scan must succeed");
    let a = unit(&units, "src/a.rs");
    let b = unit(&units, "src/b.rs");
    assert!(
        a.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("src/b.rs")),
        "a.rs -> b.rs resolved internal use edge"
    );
    assert!(
        b.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("src/a.rs")),
        "b.rs -> a.rs resolved internal use edge"
    );

    let findings = analyze(&units);
    assert!(
        findings.iter().any(|f| {
            f.rule_id == RS_ARCH_001_RULE_ID
                && f.evidence.get("kind").map(String::as_str) == Some("cycle")
        }),
        "the resolved two-file use cycle fires an RS-ARCH-001 cycle finding"
    );

    fs::remove_dir_all(&root).ok();
}

/// (8) Determinism: scanning the same tree twice yields identical units
/// (BTree-ordered map construction must not leak iteration order).
#[test]
fn resolution_is_deterministic() {
    let root = temp_root("determinism");
    manifest(&root);
    write_file(&root, "src/lib.rs", "mod a;\nmod c;\n");
    write_file(&root, "src/a.rs", "mod b;\npub struct Other;\n");
    write_file(&root, "src/a/b.rs", "pub struct Thing;\n");
    write_file(&root, "src/c.rs", "use crate::a::b::Thing;\n");

    let first = scan(&root).expect("scan must succeed");
    let second = scan(&root).expect("scan must succeed");
    assert_eq!(first, second);

    fs::remove_dir_all(&root).ok();
}

/// (9) C1 regression: a raw-identifier `mod r#type;` must strip the `r#`, so
/// the module resolves to `src/type.rs`, enters `RustModuleMap`, and its own
/// internal `use` resolves too. Before the fix the raw `r#type` probed
/// `src/r#type.rs`, missed, and poisoned the graph (spurious RS-ARCH-001
/// unresolved findings).
#[test]
fn raw_identifier_mod_resolves_and_its_use_resolves() {
    let root = temp_root("raw-ident");
    manifest(&root);
    write_file(&root, "src/lib.rs", "mod r#type;\nmod helper;\n");
    write_file(
        &root,
        "src/type.rs",
        "use crate::helper::H;\npub struct T;\n",
    );
    write_file(&root, "src/helper.rs", "pub struct H;\n");

    let units = scan(&root).expect("scan must succeed");
    let lib = unit(&units, "src/lib.rs");
    assert!(
        lib.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("src/type.rs")),
        "`mod r#type;` must resolve to src/type.rs (raw ident stripped)"
    );
    let ty = unit(&units, "src/type.rs");
    assert!(
        ty.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("src/helper.rs")),
        "`use crate::helper::H` inside the raw-ident module must resolve"
    );
    let findings = analyze(&units);
    assert!(
        !findings.iter().any(|f| f.rule_id == RS_ARCH_001_RULE_ID),
        "a fully resolvable raw-ident crate emits no RS-ARCH-001 finding, got {:?}",
        findings
            .iter()
            .map(|f| f.rule_id.as_str())
            .collect::<Vec<_>>()
    );

    fs::remove_dir_all(&root).ok();
}

/// (10) M3 regression: a single-FILE scan target anchors module resolution at
/// the file's PARENT directory (`include_root`), not the file itself. Before
/// the fix `root` was the canonicalized file, so `mod bar;` probed
/// `<file>/bar.rs` and always missed.
#[test]
fn single_file_target_resolves_sibling_mod() {
    let root = temp_root("single-file");
    manifest(&root);
    write_file(&root, "src/lib.rs", "mod bar;\n");
    write_file(&root, "src/bar.rs", "pub struct B;\n");

    // Target is the FILE `src/lib.rs`; its rel path is just `lib.rs`.
    let units = scan(&root.join("src/lib.rs")).expect("scan must succeed");
    let lib = unit(&units, "lib.rs");
    assert!(
        lib.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("bar.rs")),
        "single-file scan: `mod bar;` resolves to its sibling bar.rs, got {:?}",
        lib.includes
    );

    fs::remove_dir_all(&root).ok();
}

/// (11) N1 regression: a bare `mod child;` declaration in the parent plus an
/// ordinary `use super::…` in the child is parent/child structure, NOT a
/// module cycle. `PERF-RS-ARCH-001` is the `use` import graph (`mod`
/// declaration edges excluded), so it must emit ZERO findings here while the
/// `mod` edge remains resolved in the IR/`include_graph` data.
#[test]
fn parent_child_mod_plus_super_use_is_not_a_cycle() {
    let root = temp_root("mod-parent-child");
    manifest(&root);
    write_file(&root, "src/lib.rs", "mod parent;\n");
    write_file(&root, "src/parent/mod.rs", "mod child;\npub struct P;\n");
    write_file(
        &root,
        "src/parent/child.rs",
        "use super::P;\npub struct C;\n",
    );

    let units = scan(&root).expect("scan must succeed");
    let parent = unit(&units, "src/parent/mod.rs");
    let child = unit(&units, "src/parent/child.rs");

    // The `mod` declaration edge is still emitted, resolved, and flagged as a
    // declaration (it drives module resolution; only ARCH-001 filters it).
    let mod_edge = parent
        .includes
        .iter()
        .find(|e| e.resolved && e.to_path.as_deref() == Some("src/parent/child.rs"))
        .expect("`mod child;` resolves to src/parent/child.rs");
    assert!(mod_edge.is_mod, "the `mod child;` edge is flagged is_mod");

    // The child's `use super::P` is the ordinary import edge (is_mod = false)
    // and resolves back to the parent — the pair that used to look cyclic.
    let use_edge = child
        .includes
        .iter()
        .find(|e| e.resolved && e.to_path.as_deref() == Some("src/parent/mod.rs"))
        .expect("`use super::P` resolves to src/parent/mod.rs");
    assert!(
        !use_edge.is_mod,
        "the `use super::P` edge is not a mod edge"
    );

    let findings = analyze(&units);
    assert!(
        !findings.iter().any(|f| {
            f.rule_id == RS_ARCH_001_RULE_ID
                && f.evidence.get("kind").map(String::as_str) == Some("cycle")
        }),
        "parent/child `mod` + `use super` must not be reported as a cycle, got {:?}",
        findings
            .iter()
            .filter(|f| f.rule_id == RS_ARCH_001_RULE_ID)
            .map(|f| (
                f.path.clone(),
                f.evidence.get("kind").cloned(),
                f.evidence.get("coupled_with").cloned()
            ))
            .collect::<Vec<_>>()
    );

    fs::remove_dir_all(&root).ok();
}

/// (12) N1 hub regression: `mod` declaration fan-out alone must not inflate an
/// ARCH-001 hub. A crate root declaring 13 child modules (≥ the hub
/// threshold) but importing nothing has an EMPTY `use` graph, so no hub fires.
#[test]
fn mod_declaration_fan_out_alone_is_not_a_hub() {
    let root = temp_root("mod-fanout");
    manifest(&root);
    let mods: String = (0..13).map(|i| format!("mod m{i:02};\n")).collect();
    write_file(&root, "src/lib.rs", &mods);
    for i in 0..13 {
        write_file(&root, &format!("src/m{i:02}.rs"), "pub struct S;\n");
    }

    let units = scan(&root).expect("scan must succeed");
    let lib = unit(&units, "src/lib.rs");
    let mod_edges = lib
        .includes
        .iter()
        .filter(|e| e.is_mod && e.resolved)
        .count();
    assert_eq!(mod_edges, 13, "all 13 `mod` declarations still resolve");

    let findings = analyze(&units);
    assert!(
        !findings.iter().any(|f| {
            f.rule_id == RS_ARCH_001_RULE_ID
                && f.evidence.get("kind").map(String::as_str) == Some("hub")
        }),
        "`mod` declaration fan-out alone must not fire an ARCH-001 hub"
    );

    fs::remove_dir_all(&root).ok();
}
