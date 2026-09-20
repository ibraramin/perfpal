//! Guarded-header integration locks (recursive file-level scanner walk).
//!
//! These mirror the session-21 bisects end-to-end through the real
//! `perfpal::scanner::scan` + `perfpal::analyzer::analyze` pipeline:
//! preprocessor-guarded content (`#ifndef`/`#define`/`#endif` — the everyday
//! header shape) must be visible to both extraction and the heuristics that
//! consume it.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::{Confidence, Severity, analyze};
use perfpal::ir::IncludeForm;
use perfpal::scanner::{FileUnit, scan};

fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-guarded-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp root");
    dir
}

fn unit<'a>(units: &'a [FileUnit], path: &str) -> &'a FileUnit {
    units
        .iter()
        .find(|u| u.file.path == path)
        .unwrap_or_else(|| panic!("unit {path} present"))
}

/// bxA vs bxB parity: a guarded `#include` inside `t.h` is collected,
/// counted, and resolved to its in-repo target.
#[test]
fn guarded_include_resolves_and_counts() {
    let root = temp_root("include");
    fs::write(
        root.join("main.c"),
        "#include \"t.h\"\nint main(void){return 0;}\n",
    )
    .unwrap();
    fs::write(
        root.join("t.h"),
        "#ifndef T_H\n#define T_H\n#include \"u.h\"\n#endif\n",
    )
    .unwrap();
    fs::write(root.join("u.h"), "int u;\n").unwrap();

    let units = scan(&root).expect("scan must succeed");
    let t = unit(&units, "t.h");
    assert_eq!(t.file.include_counts.quoted, 1);
    let edge = t
        .includes
        .iter()
        .find(|e| e.from_path == "t.h" && e.to_path.as_deref() == Some("u.h"))
        .expect("guarded include edge from t.h to u.h");
    assert!(edge.resolved, "in-repo target must resolve");
    assert_eq!(edge.form, IncludeForm::Quoted);

    fs::remove_dir_all(&root).ok();
}

/// g1/g2 parity: a guarded `struct Padded` yields the TYPE-002 padding
/// finding anchored at `g.h`.
#[test]
fn guarded_struct_yields_type_002() {
    let root = temp_root("struct");
    fs::write(
        root.join("c.c"),
        "#include \"g.h\"\nint main(void){return 0;}\n",
    )
    .unwrap();
    fs::write(
        root.join("g.h"),
        "#ifndef G_H\n#define G_H\nstruct Padded { char tag; double value; int count; };\nvoid use(struct Padded p);\n#endif\n",
    )
    .unwrap();

    let units = scan(&root).expect("scan must succeed");
    let findings = analyze(&units);
    let type_002 = findings
        .iter()
        .find(|f| f.rule_id == "PERF-C-TYPE-002")
        .expect("TYPE-002 through guarded struct");
    assert_eq!(type_002.path, "g.h");

    fs::remove_dir_all(&root).ok();
}

/// A guarded include cycle is still detected: exactly one ARCH-001 cycle
/// finding, anchored at the broken edge's `from` (the higher-sorted
/// `cycle_b.h -> cycle_a.h`).
#[test]
fn guarded_include_cycle_reports_arch_001() {
    let root = temp_root("cycle");
    fs::write(
        root.join("cycle_a.h"),
        "#ifndef CYCLE_A_H\n#define CYCLE_A_H\n#include \"cycle_b.h\"\n#endif\n",
    )
    .unwrap();
    fs::write(
        root.join("cycle_b.h"),
        "#ifndef CYCLE_B_H\n#define CYCLE_B_H\n#include \"cycle_a.h\"\n#endif\n",
    )
    .unwrap();

    let units = scan(&root).expect("scan must succeed");
    let findings = analyze(&units);
    let cycles: Vec<_> = findings
        .iter()
        .filter(|f| {
            f.rule_id == "PERF-C-ARCH-001"
                && f.evidence.get("kind").map(String::as_str) == Some("cycle")
        })
        .collect();
    assert_eq!(cycles.len(), 1, "exactly one broken cycle edge");
    let cycle = cycles[0];
    assert_eq!(cycle.severity, Severity::Warn);
    assert_eq!(cycle.confidence, Confidence::High);
    assert_eq!(cycle.path, "cycle_b.h");
    assert_eq!(
        cycle.evidence.get("coupled_with").map(String::as_str),
        Some("cycle_a.h"),
        "cycle finding names its partner"
    );
    // Anchored at the broken edge's directive span: `#include "cycle_a.h"`
    // is line 3 of the guarded cycle_b.h fixture above.
    assert_eq!(cycle.span.start_line, 3);

    fs::remove_dir_all(&root).ok();
}

/// A guarded global written inside a guarded function's loop yields
/// LOOP-005. This also exercises the Pass 2 guarded-function fix: without
/// it there would be no function to analyze at all.
#[test]
fn guarded_global_write_in_loop_reports_loop_005() {
    let root = temp_root("loop");
    fs::write(
        root.join("g.h"),
        "#ifndef GLO_H\n#define GLO_H\nstatic long acc;\nlong sum(int n){ for(int i=0;i<n;i++){ acc += i; } return acc; }\n#endif\n",
    )
    .unwrap();

    let units = scan(&root).expect("scan must succeed");
    // The guarded function is extracted at all.
    let g = unit(&units, "g.h");
    assert_eq!(g.file.function_count, 1);

    let findings = analyze(&units);
    let loop_005 = findings
        .iter()
        .find(|f| f.rule_id == "PERF-C-LOOP-005")
        .expect("LOOP-005 through guarded global write");
    assert_eq!(loop_005.path, "g.h");

    fs::remove_dir_all(&root).ok();
}
