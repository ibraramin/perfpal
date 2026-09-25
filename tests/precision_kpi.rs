//! M1 per-rule PRECISION KPI ratchet (plan `evidence-beta`).
//!
//! Runs the REAL full pipeline (`cli::run_with` = scanner::scan →
//! analyzer::analyze → aggregator → writer) over the COMMITTED labeled
//! mutation corpus `tests/fixtures/precision/` and MEASURES per-rule
//! precision/recall against the intended ground truth in
//! `tests/fixtures/precision/labels.json`.
//!
//! Contract (the M1 gate):
//! - a `fire` label (rule_id + path + `start_line == line`) matched by a
//!   finding ⇒ true positive; an unmatched `fire` label ⇒ false negative;
//! - ANY finding of a scoped rule that matches no `fire` label ⇒ false
//!   positive (this subsumes every finding on a `silent` (rule, path));
//! - `precision = TP / (TP + FP)` (1.0 when TP + FP == 0);
//!   `recall = TP / (TP + FN)` (1.0 when TP + FN == 0);
//! - per rule, `precision >= baseline.precision` AND
//!   `recall >= baseline.recall` must hold. The baseline is the INTENDED
//!   1.0/1.0 target: if the measurement falls short, the failure is evidence
//!   of a real analyzer gap or a corpus/label bug — never a reason to lower
//!   the baseline.
//!
//! Findings are matched by `span.start_line` (1-based). The corpus is scanned
//! in place and stays read-only; the report is written to a fresh tempdir
//! outside it, so the committed fixture tree is never mutated.
//!
//! Run with `cargo test --test precision_kpi -- --nocapture` to print the
//! per-rule markdown table and the exact mismatches on failure.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;

/// The committed labeled corpus root (`<repo>/tests/fixtures/precision`).
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/precision")
}

/// A fresh unique tempdir OUTSIDE the scanned tree.
fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-precision-{tag}-{}-{nanos}",
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

/// One measured per-rule confusion cell set.
#[derive(Default, Clone, Copy)]
struct Kpi {
    tp: usize,
    fp: usize,
    r#fn: usize,
}

impl Kpi {
    fn precision(self) -> f64 {
        let denom = self.tp + self.fp;
        if denom == 0 {
            1.0
        } else {
            self.tp as f64 / denom as f64
        }
    }

    fn recall(self) -> f64 {
        let denom = self.tp + self.r#fn;
        if denom == 0 {
            1.0
        } else {
            self.tp as f64 / denom as f64
        }
    }
}

/// `rule_id -> [(path, line)]` fire labels.
type FireLabels = BTreeMap<String, Vec<(String, u32)>>;
/// `rule_id -> [path]` silent labels.
type SilentLabels = BTreeMap<String, Vec<String>>;

/// Decode `labels.json` into `(fire labels, silent labels)`, requiring a
/// `line` on every `fire` entry.
fn decode_cases(cases: &[serde_json::Value]) -> (FireLabels, SilentLabels) {
    let mut fire: FireLabels = BTreeMap::new();
    let mut silent: SilentLabels = BTreeMap::new();
    for case in cases {
        let rule_id = case["rule_id"]
            .as_str()
            .expect("case.rule_id is a string")
            .to_string();
        let path = case["path"]
            .as_str()
            .expect("case.path is a string")
            .to_string();
        match case["expect"].as_str().expect("case.expect is a string") {
            "fire" => {
                let line = case["line"]
                    .as_u64()
                    .expect("fire case requires an integer line") as u32;
                assert!(
                    line != 0,
                    "fire case for {rule_id} at {path} has line 0 — 1-based spans required"
                );
                fire.entry(rule_id).or_default().push((path, line));
            }
            "silent" => {
                silent.entry(rule_id).or_default().push(path);
            }
            other => panic!("unknown expect value: {other}"),
        }
    }
    (fire, silent)
}

#[test]
fn precision_kpi_meets_baseline() {
    let root = corpus_root();
    let labels_path = root.join("labels.json");
    assert!(
        labels_path.is_file(),
        "labeled corpus missing at {}",
        labels_path.display()
    );

    // --- run the REAL pipeline, report OUTSIDE the read-only corpus ---------
    let out = temp_root("kpi").join("perfpal-report");
    let out_s = out.to_string_lossy().into_owned();
    let root_s = root.to_string_lossy().into_owned();
    let code = run_with(argv(&["--output", &out_s, &root_s]));
    assert_eq!(code, 0, "precision corpus pipeline must exit 0");

    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    // The report is in memory now: clean up the temp report dir BEFORE any
    // assertion below can panic, so a failure never leaks it.
    fs::remove_dir_all(out.parent().expect("report parent")).ok();
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("metrics.json parses");
    let findings = doc["findings"].as_array().expect("findings[] present");

    let labels_raw = fs::read_to_string(&labels_path).expect("labels.json readable");
    let labels: serde_json::Value = serde_json::from_str(&labels_raw).expect("labels.json parses");
    let baseline = labels["baseline"]
        .as_object()
        .expect("baseline object present");
    let cases = labels["cases"].as_array().expect("cases[] present");

    let rules: BTreeSet<String> = baseline.keys().cloned().collect();
    let (fire, silent) = decode_cases(cases);

    // m7: every baseline rule must carry at least one labeled case. A
    // baseline-only rule (no `cases[]` entry) would measure 0 TP/0 FP/0 FN,
    // and Kpi::precision()/recall() return 1.0 on an empty denominator —
    // letting the ratchet pass vacuously. Refuse that corpus shape outright.
    let case_rules: BTreeSet<&String> = fire.keys().chain(silent.keys()).collect();
    for rule in &rules {
        assert!(
            case_rules.contains(rule),
            "baseline rule {rule} has no entry in cases[] — a 0/0/0 cell would \
             ratchet vacuously to precision/recall 1.0"
        );
    }

    // --- actual findings, restricted to the scoped rules --------------------
    let mut actual: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
    for finding in findings {
        let rule = finding["rule_id"].as_str().expect("rule_id token");
        if !rules.contains(rule) {
            continue; // out-of-scope rules are not part of the M1 KPI
        }
        let path = finding["path"].as_str().expect("finding path").to_string();
        let line = finding["span"]["start_line"]
            .as_u64()
            .expect("finding span.start_line") as u32;
        actual
            .entry(rule.to_string())
            .or_default()
            .push((path, line));
    }

    // --- per-rule confusion cells -------------------------------------------
    let mut kpis: BTreeMap<String, Kpi> = BTreeMap::new();
    let mut mismatches: Vec<String> = Vec::new();
    for rule in &rules {
        let fires = fire.get(rule).cloned().unwrap_or_default();
        let findings_of_rule = actual.get(rule).cloned().unwrap_or_default();

        let fire_set: BTreeSet<(String, u32)> = fires.iter().cloned().collect();
        let actual_set: BTreeSet<(String, u32)> = findings_of_rule.iter().cloned().collect();

        let tp = fire_set.intersection(&actual_set).count();
        let r#fn = fire_set.difference(&actual_set).count();
        let fp = findings_of_rule
            .iter()
            .filter(|(path, line)| !fire_set.contains(&(path.clone(), *line)))
            .count();

        for (path, line) in fire_set.difference(&actual_set) {
            mismatches.push(format!(
                "FN  {rule} {path}:{line} — expected fire, got no finding"
            ));
        }
        for (path, line) in findings_of_rule
            .iter()
            .filter(|p| !fire_set.contains(&(p.0.clone(), p.1)))
        {
            mismatches.push(format!(
                "FP  {rule} {path}:{line} — finding has no matching fire label"
            ));
        }
        // A silent-path finding that matches no `fire` label was already
        // counted (and messaged) as an FP above — do not double-report it as
        // SILENT. Only a silent finding that IS also a fire label (a
        // contradictory corpus entry) gets the extra context message.
        for path in silent.get(rule).cloned().unwrap_or_default() {
            for (fpath, fline) in findings_of_rule
                .iter()
                .filter(|(p, _)| *p == path)
                .filter(|(p, l)| fire_set.contains(&(p.clone(), *l)))
            {
                mismatches.push(format!(
                    "SILENT {rule} {fpath}:{fline} — silent control fired"
                ));
            }
        }

        kpis.insert(rule.clone(), Kpi { tp, fp, r#fn });
    }

    // --- markdown table (visible under --nocapture) -------------------------
    println!("\n### M1 per-rule precision KPI — `tests/fixtures/precision/`\n");
    println!("| rule | TP | FP | FN | precision | recall |");
    println!("|------|----|----|----|-----------|--------|");
    for rule in &rules {
        let kpi = kpis.get(rule).copied().unwrap_or_default();
        println!(
            "| {rule} | {} | {} | {} | {:.2} | {:.2} |",
            kpi.tp,
            kpi.fp,
            kpi.r#fn,
            kpi.precision(),
            kpi.recall()
        );
    }
    println!();

    // --- ratchet: per-rule measured >= intended baseline ---------------------
    let mut violations: Vec<String> = Vec::new();
    for rule in &rules {
        let kpi = kpis.get(rule).copied().unwrap_or_default();
        let base = &baseline[rule];
        let base_precision = base["precision"].as_f64().expect("baseline.precision");
        let base_recall = base["recall"].as_f64().expect("baseline.recall");
        if kpi.precision() < base_precision {
            violations.push(format!(
                "PRECISION {rule}: measured {:.4} < baseline {:.4}",
                kpi.precision(),
                base_precision
            ));
        }
        if kpi.recall() < base_recall {
            violations.push(format!(
                "RECALL    {rule}: measured {:.4} < baseline {:.4}",
                kpi.recall(),
                base_recall
            ));
        }
    }

    if !mismatches.is_empty() {
        eprintln!("\nprecision KPI label mismatches:");
        for line in &mismatches {
            eprintln!("  {line}");
        }
    }
    if !violations.is_empty() {
        eprintln!("\nprecision KPI baseline violations:");
        for line in &violations {
            eprintln!("  {line}");
        }
    }
    assert!(
        mismatches.is_empty() && violations.is_empty(),
        "M1 precision KPI ratchet failed ({} mismatch(es), {} violation(s)); \
         a shortfall is a REAL analyzer gap or a corpus/label bug — do NOT \
         lower the baseline",
        mismatches.len(),
        violations.len()
    );
}
