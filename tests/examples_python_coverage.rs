//! `examples-python/` 22/22 rule-coverage gate (plan `python-backend`, W4).
//!
//! Runs the REAL full pipeline (`cli::run_with` = scanner::scan →
//! analyzer::analyze → aggregator → writer/reporter) over the COMMITTED
//! `examples-python/` package and locks the Python-tier dogfood evidence
//! (mirrors `tests/examples_rust_coverage.rs` for the Rust tier):
//!
//! - the emitted `PERF-PY-*` rule-id set is EXACTLY the 22-rule Python catalog
//!   (set equality — no missing rule, no unexpected rule);
//! - the scan emits ZERO `PERF-C-*` and ZERO `PERF-RS-*` findings (the corpus
//!   is pure Python);
//! - `scan.languages == ["python"]` and every scanned file is `language ==
//!   "python"` (20 `.py` files, including the `pkg/` subpackage);
//! - `report.md` §4 catalogs every emitted `PERF-PY-*` rule id;
//! - the total finding count is pinned to the measured 29 (12 warn / 17 info)
//!   and each rule's count is pinned, so a silent drop or duplication fails
//!   loudly;
//! - the documented silent controls stay silent — asserted DIRECTLY (anti-typo
//!   needle + span-overlap check + non-vacuity), never inferred.
//!
//! The scan target is the committed tree (read-only) and the report is written
//! to a fresh tempdir OUTSIDE it, so the scan root stays clean.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::finding::{
    PY_ALLOC_001_RULE_ID, PY_ALLOC_003_RULE_ID, PY_ALLOC_004_RULE_ID, PY_ARCH_001_RULE_ID,
    PY_ARCH_002_RULE_ID, PY_ARCH_003_RULE_ID, PY_ASYNC_001_RULE_ID, PY_ASYNC_002_RULE_ID,
    PY_CALL_001_RULE_ID, PY_CALL_002_RULE_ID, PY_COMPREHENSION_001_RULE_ID,
    PY_EXECUTOR_001_RULE_ID, PY_LOOP_001_RULE_ID, PY_LOOP_002_RULE_ID, PY_LOOP_003_RULE_ID,
    PY_LOOP_007_RULE_ID, PY_LOOP_009_RULE_ID, PY_MEM_002_RULE_ID, PY_N1_001_RULE_ID,
    PY_PANDAS_001_RULE_ID, PY_PANDAS_002_RULE_ID, PY_STR_001_RULE_ID,
};
use perfpal::cli::run_with;

mod common;

/// The committed corpus root (`<repo>/examples-python`).
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples-python")
}

/// A fresh unique tempdir OUTSIDE the scanned tree.
fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-examples-python-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp root");
    dir
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

/// Slice `text` from the END of the `start` marker up to the `end` marker.
fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let s = text.find(start).expect("section start marker") + start.len();
    let e = s + text[s..].find(end).expect("section end marker");
    &text[s..e]
}

/// The documented 22-rule Python catalog (plan `python-backend` W3).
fn catalog_22() -> BTreeSet<&'static str> {
    [
        PY_ALLOC_001_RULE_ID,
        PY_ALLOC_003_RULE_ID,
        PY_ALLOC_004_RULE_ID,
        PY_ARCH_001_RULE_ID,
        PY_ARCH_002_RULE_ID,
        PY_ARCH_003_RULE_ID,
        PY_ASYNC_001_RULE_ID,
        PY_ASYNC_002_RULE_ID,
        PY_CALL_001_RULE_ID,
        PY_CALL_002_RULE_ID,
        PY_COMPREHENSION_001_RULE_ID,
        PY_EXECUTOR_001_RULE_ID,
        PY_LOOP_001_RULE_ID,
        PY_LOOP_002_RULE_ID,
        PY_LOOP_003_RULE_ID,
        PY_LOOP_007_RULE_ID,
        PY_LOOP_009_RULE_ID,
        PY_MEM_002_RULE_ID,
        PY_N1_001_RULE_ID,
        PY_PANDAS_001_RULE_ID,
        PY_PANDAS_002_RULE_ID,
        PY_STR_001_RULE_ID,
    ]
    .into_iter()
    .collect()
}

/// One documented silent control: the `(file, line)` construct that must stay
/// silent for `rule`, with a distinctive `needle` so a drifted line number
/// cannot make the check pass.
struct Control {
    /// Snapshot-relative corpus path (e.g. `01_alloc_churn.py`).
    file: &'static str,
    /// 1-based line of the CONTROL construct — the line a false positive of
    /// `rule` would be anchored on.
    line: usize,
    /// The target rule id that must NOT fire at `file:line`.
    rule: &'static str,
    /// Distinctive substring of the control line (anti-typo guard).
    needle: &'static str,
}

/// The 19 silent controls (one per rule that has a clean benign variant; the
/// remaining three rules — ARCH-001, ARCH-002, ARCH-003 — are structure-level
/// rules asserted by non-vacuity + the pinned counts instead).
const SILENT_CONTROLS: [Control; 19] = [
    Control {
        file: "01_alloc_churn.py",
        line: 15,
        rule: PY_ALLOC_001_RULE_ID,
        needle: "return dict()",
    },
    Control {
        file: "01_alloc_churn.py",
        line: 33,
        rule: PY_ALLOC_003_RULE_ID,
        needle: "tmp.append(row)",
    },
    Control {
        file: "01_alloc_churn.py",
        line: 48,
        rule: PY_ALLOC_004_RULE_ID,
        needle: "return list(rows[0])",
    },
    Control {
        file: "02_string_building.py",
        line: 17,
        rule: PY_STR_001_RULE_ID,
        needle: "last = prefix + piece",
    },
    Control {
        file: "03_deep_nesting.py",
        line: 17,
        rule: PY_LOOP_001_RULE_ID,
        needle: "for x in a:",
    },
    Control {
        file: "04_stdio_churn.py",
        line: 18,
        rule: PY_LOOP_002_RULE_ID,
        needle: "print(row)",
    },
    Control {
        file: "05_recursion_complexity.py",
        line: 13,
        rule: PY_CALL_001_RULE_ID,
        needle: "return n - 1",
    },
    Control {
        file: "05_recursion_complexity.py",
        line: 56,
        rule: PY_CALL_002_RULE_ID,
        needle: "return value + 1",
    },
    Control {
        file: "06_memory_ops.py",
        line: 16,
        rule: PY_MEM_002_RULE_ID,
        needle: "return copy.deepcopy(rows)",
    },
    Control {
        file: "07_membership_scan.py",
        line: 36,
        rule: PY_LOOP_007_RULE_ID,
        needle: "if r in seen:",
    },
    Control {
        file: "08_range_len.py",
        line: 16,
        rule: PY_LOOP_003_RULE_ID,
        needle: "for i in range(n):",
    },
    Control {
        file: "09_try_in_loop.py",
        line: 17,
        rule: PY_LOOP_009_RULE_ID,
        needle: "try:",
    },
    Control {
        file: "10_comprehension.py",
        line: 20,
        rule: PY_COMPREHENSION_001_RULE_ID,
        needle: "value = transform(row)",
    },
    Control {
        file: "11_executors.py",
        line: 15,
        rule: PY_EXECUTOR_001_RULE_ID,
        needle: "ThreadPoolExecutor() as pool:",
    },
    Control {
        file: "12_io_n1.py",
        line: 14,
        rule: PY_N1_001_RULE_ID,
        needle: "conn.execute(\"select 1\")",
    },
    Control {
        file: "13_pandas.py",
        line: 24,
        rule: PY_PANDAS_001_RULE_ID,
        needle: "df[\"x\"].mean()",
    },
    Control {
        file: "14_async_blocking.py",
        line: 20,
        rule: PY_ASYNC_001_RULE_ID,
        needle: "requests.get(url)",
    },
    Control {
        file: "14_async_blocking.py",
        line: 33,
        rule: PY_ASYNC_002_RULE_ID,
        needle: "await fetch_blocking(url)",
    },
    Control {
        file: "15_cross_file_and_arch.py",
        line: 13,
        rule: PY_ARCH_003_RULE_ID,
        needle: "local_only(row)",
    },
];

/// True when a finding's span covers `line` (1-based, inclusive).
fn span_covers(finding: &serde_json::Value, line: usize) -> bool {
    let span = &finding["span"];
    let start = span["start_line"].as_u64().unwrap_or(0) as usize;
    let end = span["end_line"].as_u64().unwrap_or(u64::MAX) as usize;
    start <= line && line <= end
}

#[test]
fn examples_python_corpus_fires_exactly_the_22_python_rules() {
    let committed = corpus_root();
    assert!(
        committed.join("__init__.py").is_file(),
        "committed corpus missing at {}",
        committed.display()
    );

    // Hermetic snapshot (there is no build/dependency dir to exclude; the
    // exclusion name is a no-op safety net).
    let root = temp_root("coverage");
    let target = root.join("corpus");
    common::copy_tree_excluding(&committed, &target, "target");

    let out = root.join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        target.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "examples-python pipeline must exit 0");

    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("metrics.json parses");

    // --- 1. exactly the 22-rule id set (set equality) -----------------------
    let findings = doc["findings"].as_array().expect("findings[] present");
    let seen: BTreeSet<&str> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().expect("rule_id token"))
        .collect();
    let expected = catalog_22();
    let expected_ref: BTreeSet<&str> = expected.iter().copied().collect();
    assert_eq!(
        seen, expected_ref,
        "examples-python must fire EXACTLY the 22 PERF-PY-* ids \
         (missing or unexpected rule ids present)"
    );
    assert_eq!(expected.len(), 22, "catalog constant set must be 22 rules");

    // --- 2. zero cross-language findings -----------------------------------
    for f in findings {
        let rule = f["rule_id"].as_str().expect("rule_id token");
        assert!(
            rule.starts_with("PERF-PY-"),
            "pure-Python corpus emitted a non-Python rule id: {rule}"
        );
    }

    // --- 3. language derivation --------------------------------------------
    assert_eq!(
        doc["scan"]["languages"],
        serde_json::json!(["python"]),
        "examples-python must scan as Python only"
    );
    let files = doc["files"].as_array().expect("files[] present");
    assert_eq!(files.len(), 20, "corpus is 20 .py sources");
    for f in files {
        let path = f["path"].as_str().expect("file path");
        assert!(path.ends_with(".py"), "non-Python source admitted: {path}");
        assert_eq!(
            f["language"].as_str(),
            Some("python"),
            "scanned file {path} must be language == \"python\""
        );
    }

    // --- 4. report.md §4 catalogs every emitted PY id -----------------------
    let report = fs::read_to_string(out.join("report.md")).expect("report.md present");
    // §4 embeds ids through the reporter's Markdown escaper (`-` → `\-`),
    // so compare against the DE-ESCAPED section.
    let sec4 = section(&report, "## 4. Findings", "## 5. Architecture").replace('\\', "");
    for rule in &seen {
        assert!(
            sec4.contains(rule),
            "report.md §4 must catalog emitted Python rule {rule}"
        );
    }

    // --- 5. pinned total + severity split + per-rule counts -----------------
    assert_eq!(
        findings.len(),
        29,
        "pinned examples-python total drifted from 29"
    );
    let count = |sev: &str| findings.iter().filter(|f| f["severity"] == sev).count();
    assert_eq!(
        (count("hot"), count("warn"), count("info")),
        (0, 12, 17),
        "pinned examples-python severity split drifted from 0/12/17"
    );
    let per_rule: &[(&str, usize)] = &[
        (PY_ALLOC_001_RULE_ID, 1),
        (PY_ALLOC_003_RULE_ID, 3),
        (PY_ALLOC_004_RULE_ID, 1),
        (PY_ARCH_001_RULE_ID, 1),
        (PY_ARCH_002_RULE_ID, 1),
        (PY_ARCH_003_RULE_ID, 3),
        (PY_ASYNC_001_RULE_ID, 2),
        (PY_ASYNC_002_RULE_ID, 1),
        (PY_CALL_001_RULE_ID, 1),
        (PY_CALL_002_RULE_ID, 1),
        (PY_COMPREHENSION_001_RULE_ID, 1),
        (PY_EXECUTOR_001_RULE_ID, 1),
        (PY_LOOP_001_RULE_ID, 1),
        (PY_LOOP_002_RULE_ID, 1),
        (PY_LOOP_003_RULE_ID, 1),
        (PY_LOOP_007_RULE_ID, 3),
        (PY_LOOP_009_RULE_ID, 1),
        (PY_MEM_002_RULE_ID, 1),
        (PY_N1_001_RULE_ID, 1),
        (PY_PANDAS_001_RULE_ID, 1),
        (PY_PANDAS_002_RULE_ID, 1),
        (PY_STR_001_RULE_ID, 1),
    ];
    for (rule, want) in per_rule {
        let got = findings.iter().filter(|f| f["rule_id"] == *rule).count();
        assert_eq!(got, *want, "pinned count for {rule} drifted");
    }

    // --- 6. the silent controls stay silent ---------------------------------
    for control in SILENT_CONTROLS {
        // (a) anti-typo: read the scanned snapshot file and pin the line.
        let source = fs::read_to_string(target.join(control.file))
            .unwrap_or_else(|e| panic!("control file {} unreadable: {e}", control.file));
        let line_text = source.lines().nth(control.line - 1).unwrap_or_else(|| {
            panic!(
                "control line {}:{} out of range (drifted line number)",
                control.file, control.line
            )
        });
        assert!(
            line_text.contains(control.needle),
            "control {}:{} drifted: expected {:?} on the control line, found {line_text:?}",
            control.file,
            control.line,
            control.needle
        );

        // (b) the target rule must NOT fire on the control construct.
        let offenders: Vec<&serde_json::Value> = findings
            .iter()
            .filter(|f| {
                f["rule_id"] == control.rule
                    && f["path"] == control.file
                    && span_covers(f, control.line)
            })
            .collect();
        assert!(
            offenders.is_empty(),
            "silent control {}:{} fired {} ({} finding(s)): {offenders:?}",
            control.file,
            control.line,
            control.rule,
            offenders.len()
        );

        // (c) non-vacuity: the rule must fire somewhere else in the corpus.
        let rule_total = findings
            .iter()
            .filter(|f| f["rule_id"] == control.rule)
            .count();
        assert!(
            rule_total > 0,
            "silent control {}:{} is vacuous: {} never fires anywhere in the corpus",
            control.file,
            control.line,
            control.rule
        );
    }

    fs::remove_dir_all(&root).ok();
}
