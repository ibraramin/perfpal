//! Scanner→analyzer integration test (modules.md §2 pipeline): real
//! discovery + tree-sitter-c extraction over a temporary directory, then the
//! full registry. Locks discovery determinism (sorted emission, dot-dir skip,
//! symlink non-follow), quoted-include resolution, ParseHealth capture, and
//! end-to-end ALLOC-001 firing.

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::{Confidence, Severity, analyze};
use perfpal::ir::{IncludeForm, ParseStatus};
use perfpal::scanner::{ScanError, scan};

const LOOP_MALLOC_C: &str = "\
#include <stdio.h>
#include \"inc/util.h\"
#include \"missing.h\"

void fill(int n) {
    for (int i = 0; i < n; ++i) {
        char *p = malloc(n);
        p[0] = 0;
    }
}
";

const UTIL_H: &str = "int helper(int x);\n";

fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-itest-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(dir.join("inc")).expect("create temp root");
    dir
}

#[test]
fn scan_then_analyze_end_to_end() {
    let root = temp_root("pipeline");
    fs::write(root.join("a.c"), LOOP_MALLOC_C).unwrap();
    fs::write(root.join("inc/util.h"), UTIL_H).unwrap();
    // Skipped silently: dot-prefixed dir, wrong extension.
    fs::create_dir(root.join(".hidden")).unwrap();
    fs::write(root.join(".hidden/skip.c"), "int z(void) { return 0; }\n").unwrap();
    fs::write(root.join("notes.txt"), "not C\n").unwrap();

    let result = scan(&root);
    let units = match result {
        Ok(units) => units,
        Err(err) => panic!("scan failed: {err}"),
    };

    // Deterministic lexicographic emission; only .c/.h survive.
    let paths: Vec<&str> = units.iter().map(|u| u.file.path.as_str()).collect();
    assert_eq!(paths, ["a.c", "inc/util.h"]);

    let a = &units[0];
    // FileRecord stats (modules.md §3.2).
    assert_eq!(a.file.function_count, 1);
    assert_eq!(a.file.include_counts.quoted, 2);
    assert_eq!(a.file.include_counts.angle, 1);
    // Quoted include resolution: includer-relative hit, miss stays data.
    let util = a
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("inc/util.h"));
    assert!(
        util.is_some_and(|e| e.resolved),
        "includer-relative resolution"
    );
    let missing = a.includes.iter().find(|e| e.to_path.is_none());
    assert!(
        missing.is_some_and(|e| !e.resolved),
        "unresolvable quoted ⇒ resolved:false"
    );
    // ParseHealth captured honestly.
    assert_eq!(a.parse_health.status, ParseStatus::Ok);

    // FunctionRecord + events from the backend.
    let func = &a.functions[0].record;
    assert_eq!(func.name, "fill");
    assert_eq!(func.max_loop_depth, 1);
    assert_eq!(func.alloc_site_count, 1);
    let malloc = a.functions[0]
        .calls
        .iter()
        .find(|c| c.callee_name == "malloc")
        .expect("malloc CallEvent");
    assert_eq!(malloc.meta.in_loop_depth, 1);

    // End-to-end: ALLOC-001 fires through scanner → analyzer, and Wave 2a's
    // ARCH-001 flags the unresolvable quoted `missing.h` (Info, data not
    // error) — exactly two findings from this unit set.
    let findings = analyze(&units);
    assert_eq!(findings.len(), 2);
    let alloc001 = findings
        .iter()
        .find(|f| f.rule_id == "PERF-C-ALLOC-001")
        .expect("ALLOC-001 through the real backend");
    assert_eq!(alloc001.severity, Severity::Hot);
    assert_eq!(alloc001.confidence, Confidence::High);
    assert_eq!(alloc001.path, "a.c");
    let arch001 = findings
        .iter()
        .find(|f| f.rule_id == "PERF-C-ARCH-001")
        .expect("unresolved quoted include surfaced by ARCH-001");
    assert_eq!(arch001.severity, Severity::Info);
    assert_eq!(
        arch001.evidence.get("kind").map(String::as_str),
        Some("unresolved_include")
    );

    fs::remove_dir_all(&root).ok();
}

/// P4 Wave 1 (ADR-0002 chart policy): angle-bracket includes get the SAME
/// two-candidate resolution as quoted includes. An angle hit on an in-repo
/// file becomes an internal coupling edge (`resolved:true`, repo-relative
/// path); an angle miss stays external (raw include text kept,
/// `resolved:false`). Quoted behavior unchanged.
#[test]
fn angle_includes_resolve_when_target_is_in_repo() {
    let root = temp_root("angle");
    fs::write(
        root.join("a.c"),
        "#include <util.h>\n#include <stdio.h>\n#include \"inc/util.h\"\n\nvoid f(void) {}\n",
    )
    .unwrap();
    // Root-level util.h: target of the angle-bracket include.
    fs::write(root.join("util.h"), UTIL_H).unwrap();
    fs::write(root.join("inc/util.h"), UTIL_H).unwrap();

    let units = scan(&root).expect("scan must succeed");
    let a = units
        .iter()
        .find(|u| u.file.path == "a.c")
        .expect("a.c unit present");

    // (a) Angle hit: util.h exists in the scanned tree → internal coupling.
    let util = a
        .includes
        .iter()
        .find(|e| e.form == IncludeForm::Angle && e.to_path.as_deref() == Some("util.h"))
        .expect("angle edge with normalized repo-relative to_path");
    assert!(
        util.resolved,
        "in-repo angle target must resolve:\n{util:?}"
    );

    // (b) Angle miss: stdio.h absent from the tree → external bucket, raw
    // text preserved in to_path (never nulled).
    let stdio = a
        .includes
        .iter()
        .find(|e| e.form == IncludeForm::Angle && e.to_path.as_deref() == Some("stdio.h"))
        .expect("unresolved angle edge keeps raw include text");
    assert!(!stdio.resolved, "absent angle target stays external");

    // (c) Quoted behavior unchanged: includer-relative hit resolves.
    let quoted = a
        .includes
        .iter()
        .find(|e| e.form == IncludeForm::Quoted)
        .expect("quoted edge present");
    assert_eq!(quoted.to_path.as_deref(), Some("inc/util.h"));
    assert!(quoted.resolved);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn unreadable_single_file_target_is_usage_error() {
    let err = scan(&std::env::temp_dir().join("perfpal-does-not-exist-xyz"));
    assert!(
        matches!(err, Err(ScanError::UnreadableTarget(_))),
        "total target failure must be distinguishable for cli exit 1"
    );
}

/// M1 regression: a single-file target must scan under its own file name
/// (not a trailing-slash path that fails ENOTDIR and degrades to a failed
/// unit) and feed the analyzer normally.
#[test]
fn single_file_target_scans_under_its_own_name() {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("perfpal-single-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("hot_loop.c");
    fs::write(
        &file,
        "void f(int n) {\n    for (int i = 0; i < n; ++i) {\n        char *p = malloc(n);\n        p[0] = 0;\n    }\n}\n",
    )
    .unwrap();

    let units = scan(&file).expect("single-file scan must succeed");
    assert_eq!(units.len(), 1);
    let unit = &units[0];
    assert_eq!(unit.file.path, "hot_loop.c", "rel path = file name");
    assert_eq!(unit.parse_health.status, ParseStatus::Ok);
    assert_eq!(unit.file.function_count, 1);
    let malloc = unit.functions[0]
        .calls
        .iter()
        .find(|c| c.callee_name == "malloc")
        .expect("malloc CallEvent");
    assert_eq!(malloc.meta.in_loop_depth, 1);

    let findings = analyze(&units);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "PERF-C-ALLOC-001");
    assert_eq!(findings[0].path, "hot_loop.c");

    fs::remove_dir_all(&dir).ok();
}

/// m9 regression: an unreadable TARGET DIRECTORY is a usage error (exit 1),
/// not a silent empty scan. Nested subdirs stay silent-continue (partial
/// discovery is data), but the root itself must be listable.
#[test]
#[cfg(unix)]
fn unreadable_target_directory_is_usage_error() {
    use std::os::unix::fs::PermissionsExt;

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-locked-root-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("a.c"), "int a(void) { return 0; }\n").unwrap();
    // Strip all permission bits so read_dir on the ROOT fails.
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o000)).unwrap();

    // Running as root defeats permission bits; only hold the assertion when
    // the read genuinely fails in this environment.
    if fs::read_dir(&dir).is_err() {
        let err = scan(&dir);
        assert!(
            matches!(err, Err(ScanError::UnreadableTarget(_))),
            "unreadable root directory must be a usage error, got {err:?}"
        );
    }

    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    fs::remove_dir_all(&dir).ok();
}

#[test]
#[cfg(unix)]
fn per_file_read_failure_becomes_failed_health_not_abort() {
    use std::os::unix::fs::PermissionsExt;
    let root = temp_root("unreadable");
    fs::write(root.join("good.c"), "int g(void) { return 0; }\n").unwrap();
    let locked = root.join("locked.c");
    fs::write(&locked, "int l(void) { return 0; }\n").unwrap();
    // Strip all permission bits so fs::read fails for unprivileged users.
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    // Running as root defeats permission bits; the downgrade assertion would
    // be vacuous there, so only hold it when the read actually fails.
    let read_actually_fails = fs::read(&locked).is_err();

    let units = scan(&root).expect("per-file failure must not abort the run");
    assert_eq!(units.len(), 2);
    let bad = units.iter().find(|u| u.file.path == "locked.c").unwrap();
    if read_actually_fails {
        assert_eq!(bad.parse_health.status, ParseStatus::Failed);
    } else {
        assert_eq!(bad.parse_health.status, ParseStatus::Ok);
    }
    let good = units.iter().find(|u| u.file.path == "good.c").unwrap();
    assert_eq!(good.parse_health.status, ParseStatus::Ok);

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
    fs::remove_dir_all(&root).ok();
}
