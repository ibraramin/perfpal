//! `examples-js/` 11/11 rule-coverage gate (plan `js-ts`, B2-W2 / W3a).
//!
//! Runs the REAL full pipeline (`cli::run_with` = scanner::scan →
//! analyzer::analyze → aggregator → writer/reporter) over the COMMITTED
//! `examples-js/` package and locks the JavaScript-tier dogfood evidence
//! (mirrors `tests/examples_python_coverage.rs` for the Python tier):
//!
//! - the emitted `PERF-JS-*` rule-id set is EXACTLY the frozen 11-rule
//!   JavaScript catalog (set equality — no missing rule, no unexpected rule);
//! - the scan emits ZERO `PERF-C-*`, `PERF-PY-*`, `PERF-RS-*` and `PERF-TS-*`
//!   findings (the corpus is pure JS/ESM + one CJS module);
//! - `scan.languages == ["js"]` and every scanned file is `language == "js"`
//!   (14 `.js` sources + one `.cjs` module; `package.json` is not a source);
//! - `report.md` §4 catalogs every emitted `PERF-JS-*` rule id;
//! - the total finding count is pinned to the measured 11 (5 warn / 6 info),
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
    JS_ALLOC_001_RULE_ID, JS_ALLOC_003_RULE_ID, JS_ARCH_001_RULE_ID, JS_ASYNC_001_RULE_ID,
    JS_ASYNC_002_RULE_ID, JS_CALL_001_RULE_ID, JS_LOOP_001_RULE_ID, JS_LOOP_002_RULE_ID,
    JS_LOOP_007_RULE_ID, JS_STR_001_RULE_ID, JS_TYPE_001_RULE_ID,
};
use perfpal::cli::run_with;

mod common;

/// The committed corpus root (`<repo>/examples-js`).
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples-js")
}

/// A fresh unique tempdir OUTSIDE the scanned tree.
fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-examples-js-{tag}-{}-{nanos}",
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

/// The frozen 11-rule JavaScript catalog (plan `js-ts` B2-W0 disposition).
fn catalog_11() -> BTreeSet<&'static str> {
    [
        JS_ALLOC_001_RULE_ID,
        JS_ALLOC_003_RULE_ID,
        JS_ARCH_001_RULE_ID,
        JS_ASYNC_001_RULE_ID,
        JS_ASYNC_002_RULE_ID,
        JS_CALL_001_RULE_ID,
        JS_LOOP_001_RULE_ID,
        JS_LOOP_002_RULE_ID,
        JS_LOOP_007_RULE_ID,
        JS_STR_001_RULE_ID,
        JS_TYPE_001_RULE_ID,
    ]
    .into_iter()
    .collect()
}

/// One documented silent control: the `(file, line)` construct that must stay
/// silent for `rule`, with a distinctive `needle` so a drifted line number
/// cannot make the check pass.
struct Control {
    /// Snapshot-relative corpus path (e.g. `01_alloc_churn.js`).
    file: &'static str,
    /// 1-based line of the CONTROL construct — the line a false positive of
    /// `rule` would be anchored on.
    line: usize,
    /// The target rule id that must NOT fire at `file:line`.
    rule: &'static str,
    /// Distinctive substring of the control line (anti-typo guard).
    needle: &'static str,
}

/// The 18 silent controls: ALLOC-001 induction-variable + mutation variants,
/// ALLOC-003 snapshot + out-of-loop, STR-001 tiny loop + out-of-loop + numeric
/// `+=`, LOOP-001
/// depth-1, LOOP-002 out-of-loop, LOOP-007 out-of-loop + small collection,
/// ASYNC-001 out-of-loop, ASYNC-002 non-async, CALL-001 guarded, TYPE-001
/// subscript, and the cross-cutting acyclic-import / slice / concat controls.
const SILENT_CONTROLS: [Control; 18] = [
    Control {
        file: "01_alloc_churn.js",
        line: 17,
        rule: JS_ALLOC_001_RULE_ID,
        needle: "points.push({ index: i });",
    },
    Control {
        file: "01_alloc_churn.js",
        line: 27,
        rule: JS_ALLOC_001_RULE_ID,
        needle: "out.push({ label });",
    },
    Control {
        file: "02_array_accumulation.js",
        line: 16,
        rule: JS_ALLOC_003_RULE_ID,
        needle: "const view = rows.slice();",
    },
    Control {
        file: "02_array_accumulation.js",
        line: 23,
        rule: JS_ALLOC_003_RULE_ID,
        needle: "return a.concat(b);",
    },
    Control {
        file: "03_string_build.js",
        line: 16,
        rule: JS_STR_001_RULE_ID,
        needle: "out += `item:${items[i]}`;",
    },
    Control {
        file: "03_string_build.js",
        line: 22,
        rule: JS_STR_001_RULE_ID,
        needle: "s += suffix;",
    },
    Control {
        file: "03_string_build.js",
        line: 29,
        rule: JS_STR_001_RULE_ID,
        needle: "total += value;",
    },
    Control {
        file: "04_deep_nesting.js",
        line: 17,
        rule: JS_LOOP_001_RULE_ID,
        needle: "total = total + values[i];",
    },
    Control {
        file: "05_sync_io_loop.js",
        line: 15,
        rule: JS_LOOP_002_RULE_ID,
        needle: "return readFileSync(file, \"utf8\");",
    },
    Control {
        file: "06_membership_scan.js",
        line: 16,
        rule: JS_LOOP_007_RULE_ID,
        needle: "return seen.includes(ids[0]);",
    },
    Control {
        file: "06_membership_scan.js",
        line: 22,
        rule: JS_LOOP_007_RULE_ID,
        needle: "if (seen.includes(ids[i])) {",
    },
    Control {
        file: "07_async_await.js",
        line: 17,
        rule: JS_ASYNC_001_RULE_ID,
        needle: "const payload = await fetchJson(url);",
    },
    Control {
        file: "08_async_blocking.js",
        line: 11,
        rule: JS_ASYNC_002_RULE_ID,
        needle: "return readFileSync(path, \"utf8\");",
    },
    Control {
        file: "09_recursion.js",
        line: 13,
        rule: JS_CALL_001_RULE_ID,
        needle: "return countdown(n - 1);",
    },
    Control {
        file: "12_delete.js",
        line: 10,
        rule: JS_TYPE_001_RULE_ID,
        needle: "delete record[key];",
    },
    Control {
        file: "13_controls.js",
        line: 5,
        rule: JS_ARCH_001_RULE_ID,
        needle: "import { score } from \"./04_deep_nesting.js\";",
    },
    Control {
        file: "13_controls.js",
        line: 10,
        rule: JS_ALLOC_003_RULE_ID,
        needle: "chunks.push(items.slice(i, i + size));",
    },
    Control {
        file: "13_controls.js",
        line: 20,
        rule: JS_ALLOC_003_RULE_ID,
        needle: "return base.concat(patch);",
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
fn examples_js_corpus_fires_exactly_the_11_js_rules() {
    let committed = corpus_root();
    assert!(
        committed.join("package.json").is_file() && committed.join("index.js").is_file(),
        "committed corpus missing at {}",
        committed.display()
    );

    // Hermetic snapshot. `package.json` is copied too: the entry ladder needs
    // it, and it is not part of the `.js` file count.
    let root = temp_root("coverage");
    let target = root.join("corpus");
    common::copy_tree_excluding(&committed, &target, "node_modules");

    let out = root.join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        target.to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "examples-js pipeline must exit 0");

    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("metrics.json parses");

    // --- 1. exactly the 11-rule id set (set equality) -----------------------
    let findings = doc["findings"].as_array().expect("findings[] present");
    let seen: BTreeSet<&str> = findings
        .iter()
        .map(|f| f["rule_id"].as_str().expect("rule_id token"))
        .collect();
    let expected = catalog_11();
    let expected_ref: BTreeSet<&str> = expected.iter().copied().collect();
    assert_eq!(
        seen, expected_ref,
        "examples-js must fire EXACTLY the 11 PERF-JS-* ids \
         (missing or unexpected rule ids present)"
    );
    assert_eq!(expected.len(), 11, "catalog constant set must be 11 rules");

    // --- 2. zero cross-language findings -----------------------------------
    for f in findings {
        let rule = f["rule_id"].as_str().expect("rule_id token");
        assert!(
            rule.starts_with("PERF-JS-"),
            "pure-JS corpus emitted a non-JS rule id: {rule}"
        );
    }

    // --- 3. language derivation --------------------------------------------
    assert_eq!(
        doc["scan"]["languages"],
        serde_json::json!(["js"]),
        "examples-js must scan as JS only"
    );
    let files = doc["files"].as_array().expect("files[] present");
    assert_eq!(files.len(), 15, "corpus is 14 .js + 1 .cjs sources");
    let mut js = 0usize;
    let mut cjs = 0usize;
    for f in files {
        let path = f["path"].as_str().expect("file path");
        if path.ends_with(".js") {
            js += 1;
        } else if path.ends_with(".cjs") {
            cjs += 1;
        } else {
            panic!("non-JS source admitted: {path}");
        }
        assert_eq!(
            f["language"].as_str(),
            Some("js"),
            "scanned file {path} must be language == \"js\""
        );
    }
    assert_eq!((js, cjs), (14, 1), "corpus .js/.cjs file split drifted");

    // --- 4. report.md §4 catalogs every emitted JS id -----------------------
    let report = fs::read_to_string(out.join("report.md")).expect("report.md present");
    // §4 embeds ids through the reporter's Markdown escaper (`-` → `\-`),
    // so compare against the DE-ESCAPED section.
    let sec4 = section(&report, "## 4. Findings", "## 5. Explorer").replace('\\', "");
    for rule in &seen {
        assert!(
            sec4.contains(rule),
            "report.md §4 must catalog emitted JS rule {rule}"
        );
    }

    // --- 5. pinned total + severity split + per-rule counts -----------------
    assert_eq!(
        findings.len(),
        11,
        "pinned examples-js total drifted from 11"
    );
    let count = |sev: &str| findings.iter().filter(|f| f["severity"] == sev).count();
    assert_eq!(
        (count("hot"), count("warn"), count("info")),
        (0, 5, 6),
        "pinned examples-js severity split drifted from 0/5/6"
    );
    let per_rule: &[(&str, usize)] = &[
        (JS_ALLOC_001_RULE_ID, 1),
        (JS_ALLOC_003_RULE_ID, 1),
        (JS_ARCH_001_RULE_ID, 1),
        (JS_ASYNC_001_RULE_ID, 1),
        (JS_ASYNC_002_RULE_ID, 1),
        (JS_CALL_001_RULE_ID, 1),
        (JS_LOOP_001_RULE_ID, 1),
        (JS_LOOP_002_RULE_ID, 1),
        (JS_LOOP_007_RULE_ID, 1),
        (JS_STR_001_RULE_ID, 1),
        (JS_TYPE_001_RULE_ID, 1),
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
