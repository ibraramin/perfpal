//! `examples-rust/` 23/23 rule-coverage gate (plan `rust-backend`, R5 lane B).
//!
//! Runs the REAL full pipeline (`cli::run_with` = scanner::scan →
//! analyzer::analyze → aggregator → writer/reporter) over the COMMITTED
//! `examples-rust/` mini-crate and locks the documented R4 evidence
//! (`docs/evidence/dogfood-rust-backend.md` §2):
//!
//! - the emitted `PERF-RS-*` rule-id set is EXACTLY the 23-rule Rust catalog
//!   (set equality — no missing rule, no unexpected rule);
//! - the scan emits ZERO `PERF-C-*` findings (the corpus is pure Rust);
//! - `scan.languages == ["rust"]` and every scanned file is `language ==
//!   "rust"`; `Cargo.toml` (the crate manifest that exists only for module
//!   resolution) is NOT admitted as a source file;
//! - `report.md` §4 catalogs every emitted `PERF-RS-*` rule id;
//! - the total finding count is pinned to the documented 41
//!   (17 hot / 11 warn / 13 info over 14 files) — the count is byte-stable
//!   across runs modulo provenance and is part of the R4 evidence, so it is
//!   asserted explicitly rather than left floating.
//!
//! The scan target is the committed tree (read-only) and the report is written
//! to a fresh tempdir OUTSIDE it, so the scan root stays clean and the gate
//! exercises the exact corpus that ships in the repo.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::finding::{
    RS_ALLOC_001_RULE_ID, RS_ALLOC_002_RULE_ID, RS_ALLOC_003_RULE_ID, RS_ARCH_001_RULE_ID,
    RS_ARCH_002_RULE_ID, RS_ARCH_003_RULE_ID, RS_CALL_001_RULE_ID, RS_CALL_002_RULE_ID,
    RS_LOOP_001_RULE_ID, RS_LOOP_002_RULE_ID, RS_LOOP_003_RULE_ID, RS_LOOP_004_RULE_ID,
    RS_LOOP_005_RULE_ID, RS_LOOP_006_RULE_ID, RS_LOOP_007_RULE_ID, RS_LOOP_008_RULE_ID,
    RS_MEM_001_RULE_ID, RS_MEM_002_RULE_ID, RS_STR_001_RULE_ID, RS_STR_002_RULE_ID,
    RS_STR_003_RULE_ID, RS_TYPE_001_RULE_ID, RS_TYPE_002_RULE_ID,
};
use perfpal::cli::run_with;

mod common;

/// The committed corpus root (`<repo>/examples-rust`).
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples-rust")
}

/// A fresh unique tempdir OUTSIDE the scanned tree.
fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-examples-rust-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp root");
    dir
}

fn argv(args: &[&str]) -> Vec<std::ffi::OsString> {
    std::iter::once(std::ffi::OsString::from("perfpal"))
        .chain(args.iter().map(std::ffi::OsString::from))
        .collect()
}

/// Slice `text` from the END of the `start` marker up to the `end` marker.
fn section<'a>(text: &'a str, start: &str, end: &str) -> &'a str {
    let s = text.find(start).expect("section start marker") + start.len();
    let e = s + text[s..].find(end).expect("section end marker");
    &text[s..e]
}

/// The documented 23-rule Rust catalog, reusing the canonical constants and
/// naming style of `tests/rust_wave_c2.rs`.
fn catalog_23() -> BTreeSet<&'static str> {
    [
        RS_ALLOC_001_RULE_ID,
        RS_ALLOC_002_RULE_ID,
        RS_ALLOC_003_RULE_ID,
        RS_ARCH_001_RULE_ID,
        RS_ARCH_002_RULE_ID,
        RS_ARCH_003_RULE_ID,
        RS_CALL_001_RULE_ID,
        RS_CALL_002_RULE_ID,
        RS_LOOP_001_RULE_ID,
        RS_LOOP_002_RULE_ID,
        RS_LOOP_003_RULE_ID,
        RS_LOOP_004_RULE_ID,
        RS_LOOP_005_RULE_ID,
        RS_LOOP_006_RULE_ID,
        RS_LOOP_007_RULE_ID,
        RS_LOOP_008_RULE_ID,
        RS_MEM_001_RULE_ID,
        RS_MEM_002_RULE_ID,
        RS_STR_001_RULE_ID,
        RS_STR_002_RULE_ID,
        RS_STR_003_RULE_ID,
        RS_TYPE_001_RULE_ID,
        RS_TYPE_002_RULE_ID,
    ]
    .into_iter()
    .collect()
}

/// One documented silent control from
/// `docs/evidence/dogfood-rust-backend.md` §"Silent controls (13, each
/// verified to fire **0** findings)" + the inline corpus markers.
struct Control {
    /// Snapshot-relative corpus path (e.g. `src/01_alloc_churn.rs`).
    file: &'static str,
    /// 1-based line of the CONTROL CONSTRUCT — the line a false-positive
    /// finding of `rule` would be anchored on.
    line: usize,
    /// The target rule id that must NOT fire at `file:line`.
    rule: &'static str,
    /// Distinctive substring of the control line, so a line-number typo that
    /// silently points at some other line cannot make the check pass.
    needle: &'static str,
}

/// All 13 documented silent controls, expanded to their 14 (file, line, rule)
/// targets (`make_scratch` covers BOTH ALLOC-001 and ALLOC-003).
///
/// Re-verified against the evidence doc + the corpus comments; two line
/// drifts corrected vs the review table and one omitted control restored:
/// - `07_struct_by_value.rs` TYPE-001 is anchored at the `consume_blob_ref`
///   DEFINITION (line 45); line 44 is that function's doc comment.
/// - `09_membership_scan.rs` LOOP-007 is anchored at the comparison `if`
///   (line 28); line 29 is the comment inside its body.
/// - the evidence doc's 13th control `PackedTight → TYPE-002`
///   (`07_struct_by_value.rs`, struct def line 34) was absent from the review
///   table and is asserted here so all 13 documented controls are covered.
const SILENT_CONTROLS: [Control; 14] = [
    Control {
        file: "src/01_alloc_churn.rs",
        line: 21,
        rule: RS_ALLOC_001_RULE_ID,
        needle: "Vec::new()",
    },
    Control {
        file: "src/01_alloc_churn.rs",
        line: 22,
        rule: RS_ALLOC_003_RULE_ID,
        needle: "reserve(8)",
    },
    Control {
        file: "src/01_alloc_churn.rs",
        line: 28,
        rule: RS_ALLOC_002_RULE_ID,
        needle: "drop(value)",
    },
    Control {
        file: "src/02_string_building.rs",
        line: 42,
        rule: RS_STR_002_RULE_ID,
        needle: "s.len()",
    },
    Control {
        file: "src/02_string_building.rs",
        line: 51,
        rule: RS_STR_001_RULE_ID,
        needle: "s += part",
    },
    Control {
        file: "src/02_string_building.rs",
        line: 66,
        rule: RS_STR_003_RULE_ID,
        needle: "copy_from_slice(src)",
    },
    Control {
        file: "src/04_stdio_churn.rs",
        line: 37,
        rule: RS_LOOP_003_RULE_ID,
        needle: "out.extend(text.chars())",
    },
    Control {
        file: "src/06_memory_ops.rs",
        line: 20,
        rule: RS_MEM_001_RULE_ID,
        needle: "with_capacity(count)",
    },
    Control {
        file: "src/07_struct_by_value.rs",
        line: 45,
        rule: RS_TYPE_001_RULE_ID,
        needle: "consume_blob_ref(blob: &BigBlob)",
    },
    Control {
        file: "src/07_struct_by_value.rs",
        line: 34,
        rule: RS_TYPE_002_RULE_ID,
        needle: "pub struct PackedTight",
    },
    Control {
        file: "src/08_globals_and_chains.rs",
        line: 49,
        rule: RS_LOOP_006_RULE_ID,
        needle: "direct_handler()",
    },
    Control {
        file: "src/08_globals_and_chains.rs",
        line: 67,
        rule: RS_LOOP_008_RULE_ID,
        needle: "&cfg.region.zone",
    },
    Control {
        file: "src/09_membership_scan.rs",
        line: 28,
        rule: RS_LOOP_007_RULE_ID,
        needle: "table[j] == key",
    },
    Control {
        file: "src/10_cross_file_and_arch.rs",
        line: 43,
        rule: RS_ARCH_003_RULE_ID,
        needle: "clamp_int(*value, lo, hi)",
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
fn examples_rust_corpus_fires_exactly_the_23_rust_rules() {
    let committed = corpus_root();
    assert!(
        committed.join("src").is_dir(),
        "committed corpus missing at {}",
        committed.display()
    );

    // Hermetic snapshot: copy the crate EXCLUDING any `target/` directory, so
    // a stray build tree cannot inflate the scan (`scanner::discover` does not
    // skip it). Relative paths stay identical, keeping the scan
    // byte-equivalent to the committed tree.
    let root = temp_root("coverage");
    let target = root.join("corpus");
    common::copy_tree_excluding(&committed, &target, "target");

    let out = root.join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        target.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "examples-rust pipeline must exit 0");

    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("metrics.json parses");

    // --- 1. exactly the 23-rule id set (set equality) -----------------------
    let findings = doc["findings"].as_array().expect("findings[] present");
    let seen: BTreeSet<&str> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().expect("rule_id token"))
        .collect();
    let expected = catalog_23();
    let expected_ref: BTreeSet<&str> = expected.iter().copied().collect();
    assert_eq!(
        seen, expected_ref,
        "examples-rust must fire EXACTLY the 23 PERF-RS-* ids \
         (missing or unexpected rule ids present)"
    );
    assert_eq!(expected.len(), 23, "catalog constant set must be 23 rules");

    // --- 2. zero PERF-C-* findings ------------------------------------------
    let c_findings: Vec<&str> = findings
        .iter()
        .filter_map(|f| f["rule_id"].as_str())
        .filter(|r| r.starts_with("PERF-C-"))
        .collect();
    assert!(
        c_findings.is_empty(),
        "Rust-only corpus emitted PERF-C-* findings: {c_findings:?}"
    );
    for f in findings {
        let rule = f["rule_id"].as_str().expect("rule_id token");
        assert!(
            rule.starts_with("PERF-RS-"),
            "every corpus finding must be a PERF-RS-* id, got {rule}"
        );
    }

    // --- 3. language derivation & Cargo.toml exclusion ----------------------
    assert_eq!(
        doc["scan"]["languages"],
        serde_json::json!(["rust"]),
        "examples-rust must scan as Rust only"
    );
    let files = doc["files"].as_array().expect("files[] present");
    assert_eq!(
        files.len(),
        14,
        "corpus is 14 .rs sources; Cargo.toml must not be admitted"
    );
    for f in files {
        let path = f["path"].as_str().expect("file path");
        assert!(
            path.ends_with(".rs"),
            "non-Rust source admitted to the scan: {path}"
        );
        assert_ne!(path, "Cargo.toml", "Cargo.toml scanned as a source");
        assert_eq!(
            f["language"].as_str(),
            Some("rust"),
            "scanned file {path} must be language == \"rust\""
        );
    }

    // --- 4. report.md §4 catalogs every emitted RS id -----------------------
    let report = fs::read_to_string(out.join("report.md")).expect("report.md present");
    // §4 embeds ids through the reporter's Markdown escaper (`-` → `\-`),
    // so compare against the DE-ESCAPED section.
    let sec4 = section(&report, "## 4. Findings", "## 5. Architecture").replace('\\', "");
    for rule in &seen {
        assert!(
            sec4.contains(rule),
            "report.md §4 must catalog emitted Rust rule {rule}"
        );
    }

    // --- 5. pinned total (documented 41 = 17 hot / 11 warn / 13 info) -------
    assert_eq!(
        findings.len(),
        41,
        "documented examples-rust total drifted from 41"
    );
    let count = |sev: &str| findings.iter().filter(|f| f["severity"] == sev).count();
    assert_eq!(
        (count("hot"), count("warn"), count("info")),
        (17, 11, 13),
        "documented examples-rust severity split drifted from 17/11/13"
    );

    // --- 6. the 13 silent controls stay silent (14 target rules) ------------
    // Requirement: "the 13 silent controls stay silent" must be asserted
    // DIRECTLY, not inferred from the 23-rule set equality or the pinned
    // totals. For each control: (a) the control line must exist in the
    // snapshot AND carry the construct (anti-typo), (b) no finding of the
    // target rule may overlap that line, and (c) the target rule must fire
    // ELSEWHERE in the corpus, so (b) cannot pass vacuously because the rule
    // never ran at all.
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
