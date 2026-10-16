//! M2 benchmark harness core (plan `evidence-beta` M2).
//!
//! Builds the labeled mutation corpus from [`corpus`], materializes each rung
//! into a fresh tempdir OUTSIDE the scanned tree, drives the REAL pipeline
//! (`perfpal::cli::run_with`) over it, and MEASURES per-rule
//! precision/recall against the planted `fire` labels. Every rung is run
//! TWICE into fresh output dirs and the artifacts are compared byte-for-byte
//! after provenance masking — a run-level determinism proof.
//!
//! Scoring contract (identical to the M1 KPI gate):
//! - a `fire` label (rule_id, path, `start_line == line`) matched by a finding
//!   ⇒ true positive; an unmatched `fire` label ⇒ false negative;
//! - ANY finding of a scoped rule with no matching `fire` label ⇒ false
//!   positive (this subsumes every finding on a `silent` control);
//! - `precision = TP / (TP + FP)`, `recall = TP / (TP + FN)` (1.0 on an empty
//!   denominator);
//! - measured precision/recall MUST be ≥ the baseline in
//!   `tests/fixtures/benchmark/baseline.json`. The baseline is the intended
//!   1.0/1.0 target — a shortfall is a REAL analyzer gap or a corpus bug,
//!   never a reason to lower the baseline.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;
use perfpal::schema::SCHEMA_ARTIFACT_NAME;
use serde_json::Value;

use crate::common;

pub mod corpus;

/// The one retired rule id (M1 Precision Gate): a reserved const with no
/// registry emitter. It is silent-only by design, so the no-vacuous-cell guard
/// exempts it and it is excluded from the 67-rule active catalog.
const RETIRED_RULE_ID: &str = "PERF-RS-STR-001";

/// The 67 active rule ids, derived from the PRODUCT's public rule-id
/// constants (`perfpal::analyzer::finding`) — NOT from the corpus or the
/// baseline — so deleting a corpus unit can never silently shrink the scored
/// set: the M2 assertion `fire_set(small) == active_rule_ids()` fails loudly
/// instead.
///
/// The product registry exposes 67 registered ids (23 `PERF-C-*` + 22
/// `PERF-PY-*` + 22 `PERF-RS-*`); `PERF-RS-STR-001` is a reserved const with no
/// registry entry, so it is deliberately absent here. (`perfpal::analyzer::
/// registered_rule_ids()` would be the direct source, but it is `pub(crate)`
/// and this task forbids `src/` changes; the public consts are the same
/// authoritative ids. The in-crate test
/// `reporter::tests::catalog_covers_every_registry_rule_id` locks the
/// registry itself to exactly 67 ids, so a new registry rule still fails the
/// suite.)
fn active_rule_ids() -> BTreeSet<&'static str> {
    use perfpal::analyzer::finding::*;
    [
        ALLOC_001_RULE_ID,
        ALLOC_002_RULE_ID,
        ALLOC_003_RULE_ID,
        ARCH_001_RULE_ID,
        ARCH_002_RULE_ID,
        ARCH_003_RULE_ID,
        CALL_001_RULE_ID,
        CALL_002_RULE_ID,
        LOOP_001_RULE_ID,
        LOOP_002_RULE_ID,
        LOOP_003_RULE_ID,
        LOOP_004_RULE_ID,
        LOOP_005_RULE_ID,
        LOOP_006_RULE_ID,
        LOOP_007_RULE_ID,
        LOOP_008_RULE_ID,
        MEM_001_RULE_ID,
        MEM_002_RULE_ID,
        STR_001_RULE_ID,
        STR_002_RULE_ID,
        STR_003_RULE_ID,
        TYPE_001_RULE_ID,
        TYPE_002_RULE_ID,
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
        RS_STR_002_RULE_ID,
        RS_STR_003_RULE_ID,
        RS_TYPE_001_RULE_ID,
        RS_TYPE_002_RULE_ID,
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

/// Whether a label expects the rule to fire or stay silent at its anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expect {
    Fire,
    Silent,
}

/// One generated source file (relative POSIX path + exact contents).
#[derive(Debug, Clone)]
pub struct GenFile {
    pub rel_path: String,
    pub contents: String,
}

/// One planted ground-truth label.
#[derive(Debug, Clone)]
pub struct Label {
    pub rule_id: &'static str,
    pub rel_path: String,
    /// 1-based anchor line for `Fire` labels; ignored (0) for `Silent`.
    pub line: u32,
    pub expect: Expect,
}

/// One labeled benchmark rung.
#[derive(Debug, Clone)]
pub struct Rung {
    pub name: &'static str,
    pub files: Vec<GenFile>,
    pub labels: Vec<Label>,
}

// ---------------------------------------------------------------------------
// Temp tree (RAII): every rung gets a unique root cleaned up on drop — even
// on panic — so no generated corpus or report ever leaks or mutates the repo.
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempTree {
    root: PathBuf,
}

impl TempTree {
    fn new(tag: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "perfpal-bench-{tag}-{}-{nanos}-{seq}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create benchmark temp root");
        Self { root }
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

// ---------------------------------------------------------------------------
// Per-rule confusion cell.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Public entry point.
// ---------------------------------------------------------------------------

/// Run the whole always-on benchmark suite: every labeled rung (scale ladder)
/// plus the unlabeled mixed real-code determinism rung. Panics on the first
/// measured shortfall, parse-health failure, or nondeterminism.
pub fn run_suite() {
    let baseline = load_baseline();
    assert!(!baseline.is_empty(), "benchmark baseline is empty");

    println!("\n# M2 benchmark suite — measured per-rule precision/recall\n");
    println!("Baseline: `tests/fixtures/benchmark/baseline.json` (intended 1.0/1.0 ratchet).");
    println!(
        "Scoped rules: {} (67 active rules + retired PERF-RS-STR-001); the `small` rung \
         plants all of them, while `medium`/`large` replicate the nine gate rules only.",
        baseline.len()
    );
    println!("Corpus: deterministic in-code mutation catalog (`tests/benchmark/corpus.rs`).\n");

    let rungs = corpus::catalog();
    let small = rungs
        .iter()
        .find(|r| r.name == "small")
        .expect("catalog must contain the small rung");
    assert_small_anchors_match_m1(small);
    assert_small_covers_catalog(small);
    assert_no_stale_baseline(&rungs, &baseline);

    for rung in &rungs {
        run_labeled_rung(rung, &baseline);
    }
    run_real_code_rung();

    println!("\n## Determinism verdict\n");
    println!("PASS — every labeled rung and the mixed real-code rung produced byte-identical");
    println!("artifacts across two runs. Masking is provenance-only: metrics.json masks ONLY");
    println!("`scan.started_at`, `scan.duration_ms`, `scan.root_path` by textual value");
    println!("substitution (no JSON re-serialization; the producer's exact key order and byte");
    println!("layout are preserved); report.md masks ONLY the header `started`/`duration`");
    println!("instants and the absolute scan root (findings-detail.md masks the same way).");
    println!("The detail findings blocks, report.html, charts/*.svg");
    println!("and the bundled schema are compared byte-exact (nothing stripped or reordered).");
    println!("\n## Overall verdict\n");
    println!("PASS — all measured per-rule precision/recall meet or exceed the baseline,");
    println!(
        "all generated corpora parse cleanly (zero partial/failed), and artifacts are byte-stable.\n"
    );
}

// ---------------------------------------------------------------------------
// Baseline.
// ---------------------------------------------------------------------------

fn load_baseline() -> BTreeMap<String, (f64, f64)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/benchmark/baseline.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("benchmark baseline unreadable: {}: {e}", path.display()));
    let value: Value = serde_json::from_str(&raw).expect("baseline.json parses");
    let obj = value.as_object().expect("baseline.json is a rule object");
    obj.iter()
        .map(|(rule, cell)| {
            let precision = cell["precision"].as_f64().expect("baseline.precision");
            let recall = cell["recall"].as_f64().expect("baseline.recall");
            assert!(
                precision.is_finite() && (0.0..=1.0).contains(&precision),
                "baseline {rule} precision {precision} is not a finite value in [0, 1]"
            );
            assert!(
                recall.is_finite() && (0.0..=1.0).contains(&recall),
                "baseline {rule} recall {recall} is not a finite value in [0, 1]"
            );
            (rule.clone(), (precision, recall))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// M1 fidelity guard: the `small` rung must reproduce the committed M1
// corpus's anchor lines exactly (the mission lifts those constructs verbatim).
// ---------------------------------------------------------------------------

fn assert_small_anchors_match_m1(small: &Rung) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/precision/labels.json");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("M1 labels unreadable: {}: {e}", path.display()));
    let value: Value = serde_json::from_str(&raw).expect("M1 labels parse");
    let mut m1: BTreeSet<(String, String, u32)> = BTreeSet::new();
    for case in value["cases"].as_array().expect("M1 cases[]") {
        if case["expect"] == "fire" {
            m1.insert((
                case["rule_id"].as_str().expect("M1 rule_id").to_string(),
                case["path"].as_str().expect("M1 path").to_string(),
                case["line"].as_u64().expect("M1 line") as u32,
            ));
        }
    }
    let generated: BTreeSet<(String, String, u32)> = small
        .labels
        .iter()
        .filter(|l| l.expect == Expect::Fire)
        .map(|l| (l.rule_id.to_string(), l.rel_path.clone(), l.line))
        .collect();
    // The small rung now plants the full 67-rule catalog, so it is a SUPERSET
    // of the M1 corpus: every committed M1 fire anchor must still be present
    // verbatim (the nine gate constructs are lifted unmodified), while the
    // added coverage contributes extra anchors that M1 never had.
    let missing: Vec<_> = m1.difference(&generated).cloned().collect();
    assert!(
        missing.is_empty(),
        "small rung lost committed M1 fire anchors: {missing:?}"
    );
}

/// M2: the `small` rung MUST carry exactly one `Fire` label for every one of
/// the 67 active rules (and no other rule), so an extension unit deleted from
/// the catalog can never silently drop a rule from scoring. M4: it must also
/// carry an explicit `Silent` control for every active rule (plus the retired
/// always-silent `PERF-RS-STR-001`).
fn assert_small_covers_catalog(small: &Rung) {
    let active = active_rule_ids();
    assert_eq!(active.len(), 67, "active catalog must be 67 rule ids");

    let fired: BTreeSet<&str> = small
        .labels
        .iter()
        .filter(|l| l.expect == Expect::Fire)
        .map(|l| l.rule_id)
        .collect();
    let missing_fire: Vec<&str> = active.difference(&fired).copied().collect();
    let extra_fire: Vec<&str> = fired.difference(&active).copied().collect();
    assert!(
        missing_fire.is_empty(),
        "small rung has NO Fire label for active rule(s): {missing_fire:?}"
    );
    assert!(
        extra_fire.is_empty(),
        "small rung fires rule(s) outside the 67-rule active catalog: {extra_fire:?}"
    );

    let silenced: BTreeSet<&str> = small
        .labels
        .iter()
        .filter(|l| l.expect == Expect::Silent)
        .map(|l| l.rule_id)
        .collect();
    let expected_silent: BTreeSet<&str> = active
        .iter()
        .copied()
        .chain(std::iter::once(RETIRED_RULE_ID))
        .collect();
    let missing_silent: Vec<&str> = expected_silent.difference(&silenced).copied().collect();
    let extra_silent: Vec<&str> = silenced.difference(&expected_silent).copied().collect();
    assert!(
        missing_silent.is_empty(),
        "small rung has NO Silent control for active rule(s): {missing_silent:?}"
    );
    assert!(
        extra_silent.is_empty(),
        "small rung has Silent control(s) for unknown rule(s): {extra_silent:?}"
    );
}

/// n2: every `baseline.json` key MUST be planted by at least one rung, so a
/// stale baseline entry can never sit in the file unscored.
fn assert_no_stale_baseline(rungs: &[Rung], baseline: &BTreeMap<String, (f64, f64)>) {
    let planted: BTreeSet<&str> = rungs
        .iter()
        .flat_map(|rung| rung.labels.iter().map(|l| l.rule_id))
        .collect();
    let stale: Vec<&str> = baseline
        .keys()
        .map(String::as_str)
        .filter(|rule| !planted.contains(rule))
        .collect();
    assert!(
        stale.is_empty(),
        "baseline.json rule(s) are planted by NO rung (stale entries): {stale:?}"
    );
}

// ---------------------------------------------------------------------------
// Materialization + pipeline run.
// ---------------------------------------------------------------------------
/// Write every file of `rung` under `dir`, creating parent directories.
fn materialize(rung: &Rung, dir: &Path) {
    for file in &rung.files {
        let path = dir.join(&file.rel_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create generated parent dir");
        }
        fs::write(&path, &file.contents).expect("write generated file");
    }
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

/// Run one pipeline pass, assert exit 0, and return the parsed metrics.json.
fn run_pipeline(scan: &Path, out: &Path) -> Value {
    let out_s = out.to_string_lossy().into_owned();
    let scan_s = scan.to_string_lossy().into_owned();
    let code = run_with(argv(&["--output", &out_s, &scan_s]));
    assert_eq!(code, 0, "benchmark pipeline must exit 0 (scan {scan_s})");
    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    serde_json::from_str(&raw).expect("metrics.json parses")
}

// ---------------------------------------------------------------------------
// Per-rung execution.
// ---------------------------------------------------------------------------

fn run_labeled_rung(rung: &Rung, baseline: &BTreeMap<String, (f64, f64)>) {
    // Every rule this rung plants must be covered by the baseline, or a
    // finding would be silently unscored. Conversely the ratchet is scoped to
    // the rules the rung actually plants: `medium`/`large` replicate only the
    // nine gate rules (plus the retired always-silent control), so their
    // KPI stays the committed M1 set, while `small` ratchets all 67.
    let planted: BTreeSet<&str> = rung.labels.iter().map(|l| l.rule_id).collect();
    for rule in &planted {
        assert!(
            baseline.contains_key(*rule),
            "rung {}: planted rule {rule} is absent from the baseline",
            rung.name
        );
    }
    let effective: BTreeMap<String, (f64, f64)> = baseline
        .iter()
        .filter(|(rule, _)| planted.contains(rule.as_str()))
        .map(|(rule, cell)| (rule.clone(), *cell))
        .collect();
    assert!(
        !effective.is_empty(),
        "rung {}: no baseline rule is planted by this rung",
        rung.name
    );

    let tree = TempTree::new(rung.name);
    let scan = tree.root.join("scan");
    materialize(rung, &scan);
    let out1 = tree.root.join("out1");
    let out2 = tree.root.join("out2");

    let doc = run_pipeline(&scan, &out1);
    let _second = run_pipeline(&scan, &out2);
    assert_parse_health(rung, &doc);

    let determinism_ok = assert_byte_stable(&out1, &out2, &scan);

    let loc: usize = rung.files.iter().map(|f| f.contents.lines().count()).sum();
    let findings = doc["findings"].as_array().map_or(0, Vec::len);
    println!(
        "### Rung `{}` — {} files, {} LOC, {} findings, {} scoped rule(s), determinism {}\n",
        rung.name,
        rung.files.len(),
        loc,
        findings,
        effective.len(),
        if determinism_ok { "PASS" } else { "FAIL" }
    );

    score(rung, &doc, &effective);

    // `tree` drops here (and on panic): the generated corpus + both report
    // dirs are removed, never touching the committed tree.
}

fn assert_parse_health(rung: &Rung, doc: &Value) {
    let files = doc["files"].as_array().expect("files[] present");
    assert_eq!(
        files.len(),
        rung.files.len(),
        "rung {}: files[] count differs from generated file count",
        rung.name
    );
    for file in files {
        let path = file["path"].as_str().unwrap_or("<unknown>");
        let status = file["parse_health"]["status"]
            .as_str()
            .unwrap_or("<missing>");
        assert_eq!(
            status, "ok",
            "rung {}: {path} parse status {status}",
            rung.name
        );
    }
    let summary = &doc["parse_health_summary"];
    assert_eq!(summary["failed"], 0, "rung {}: parse failures", rung.name);
    assert_eq!(summary["partial"], 0, "rung {}: partial parses", rung.name);
    let file_count = doc["scan"]["file_count"].as_u64().expect("scan.file_count");
    assert_eq!(
        file_count as usize,
        rung.files.len(),
        "rung {}: scan.file_count differs",
        rung.name
    );
}

/// Compute + print the per-rule table and enforce the ratchet.
fn score(rung: &Rung, doc: &Value, baseline: &BTreeMap<String, (f64, f64)>) {
    let rules: BTreeSet<&str> = baseline.keys().map(String::as_str).collect();

    let mut actual: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
    for finding in doc["findings"].as_array().expect("findings[] present") {
        let rule = finding["rule_id"].as_str().expect("finding rule_id");
        if !rules.contains(rule) {
            continue; // out-of-scope rules are not part of the measured KPI
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

    let mut fire: BTreeMap<String, Vec<(String, u32)>> = BTreeMap::new();
    let mut silent: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for label in &rung.labels {
        match label.expect {
            Expect::Fire => fire
                .entry(label.rule_id.to_string())
                .or_default()
                .push((label.rel_path.clone(), label.line)),
            Expect::Silent => silent
                .entry(label.rule_id.to_string())
                .or_default()
                .push(label.rel_path.clone()),
        }
    }

    // m1/M4: the `silent` map must be meaningful, not dead weight — every
    // `Silent` rule must be inside the scored (effective) set, so a silent
    // control can never sit outside what the rung ratchets.
    for rule in silent.keys() {
        assert!(
            rules.contains(rule.as_str()),
            "rung {}: Silent control for rule {rule} is outside the scored/effective set",
            rung.name
        );
    }

    let mut kpis: BTreeMap<String, Kpi> = BTreeMap::new();
    let mut mismatches: Vec<String> = Vec::new();
    for rule in &rules {
        // M1 no-vacuous-cell guard (mirrors tests/precision_kpi.rs): a scored
        // rule with no `Fire` label measures 0/0/0 and Kpi::precision()/
        // recall() return 1.0 on an empty denominator — a vacuous pass. The
        // only exemption is the explicitly retired always-silent control.
        if *rule != RETIRED_RULE_ID {
            assert!(
                fire.contains_key(*rule),
                "rung {}: scored rule {rule} has no Fire label — a 0/0/0 cell \
                 would ratchet vacuously to precision/recall 1.0",
                rung.name
            );
        }

        let fires = fire.get(*rule).cloned().unwrap_or_default();
        let found = actual.get(*rule).cloned().unwrap_or_default();
        let fire_set: BTreeSet<(String, u32)> = fires.iter().cloned().collect();
        let found_set: BTreeSet<(String, u32)> = found.iter().cloned().collect();

        // m3: TP/FN are set-based (a duplicate finding at a fire coordinate is
        // not double-counted as a true positive), matching
        // tests/precision_kpi.rs; any finding at a non-fire coordinate —
        // including a duplicate — is an FP.
        let tp = fire_set.intersection(&found_set).count();
        let r#fn = fire_set.difference(&found_set).count();
        let fp = found
            .iter()
            .filter(|(path, line)| !fire_set.contains(&(path.clone(), *line)))
            .count();

        for (path, line) in fire_set.difference(&found_set) {
            mismatches.push(format!(
                "FN  {rule} {path}:{line} — expected fire, got no finding"
            ));
        }
        for (path, line) in &found {
            if !fire_set.contains(&(path.clone(), *line)) {
                mismatches.push(format!(
                    "FP  {rule} {path}:{line} — finding has no matching fire label"
                ));
            }
        }
        kpis.insert((*rule).to_string(), Kpi { tp, fp, r#fn });
    }

    println!("| rule | TP | FP | FN | precision | recall |");
    println!("|------|----|----|----|-----------|--------|");
    for rule in &rules {
        let kpi = kpis.get(*rule).copied().unwrap_or_default();
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

    let mut violations: Vec<String> = Vec::new();
    for (rule, (base_p, base_r)) in baseline {
        let kpi = kpis.get(rule).copied().unwrap_or_default();
        if kpi.precision() < *base_p {
            violations.push(format!(
                "PRECISION {rule}: measured {:.4} < baseline {:.4}",
                kpi.precision(),
                base_p
            ));
        }
        if kpi.recall() < *base_r {
            violations.push(format!(
                "RECALL    {rule}: measured {:.4} < baseline {:.4}",
                kpi.recall(),
                base_r
            ));
        }
    }

    if !mismatches.is_empty() {
        eprintln!("\nbenchmark rung {} label mismatches:", rung.name);
        for line in &mismatches {
            eprintln!("  {line}");
        }
    }
    if !violations.is_empty() {
        eprintln!("\nbenchmark rung {} baseline violations:", rung.name);
        for line in &violations {
            eprintln!("  {line}");
        }
    }
    assert!(
        mismatches.is_empty() && violations.is_empty(),
        "rung {}: {} label mismatch(es), {} baseline violation(s); a shortfall is a \
         REAL analyzer gap or a corpus bug — do NOT lower the baseline",
        rung.name,
        mismatches.len(),
        violations.len()
    );
}

// ---------------------------------------------------------------------------
// Determinism proof.
// ---------------------------------------------------------------------------

fn collect_artifacts(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
            let entry = entry.expect("dir entry readable");
            let name = entry
                .file_name()
                .into_string()
                .expect("UTF-8 artifact name");
            let path = entry.path();
            let rel = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if path.is_dir() {
                walk(&path, &rel, out);
            } else {
                out.insert(rel, fs::read(&path).expect("artifact readable"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, "", &mut out);
    out
}

/// Compare two report trees under a STRICT provenance-only mask: metrics.json
/// neutralizes only `scan.started_at` / `scan.duration_ms` / `scan.root_path`
/// by textual value substitution (no JSON re-serialization — the producer's
/// exact key order and byte layout survive), report.md masks only the
/// provenance instants + absolute scan root, and the §4 findings block,
/// charts/*.svg and the bundled schema are compared byte-exact. Panics with
/// the normalized diff copies written under a `TempTree`-owned dir whose path
/// is named in the panic (and which is cleaned up on unwind).
fn assert_byte_stable(out1: &Path, out2: &Path, scan: &Path) -> bool {
    let a = collect_artifacts(out1);
    let b = collect_artifacts(out2);
    assert_eq!(
        a.keys().collect::<Vec<_>>(),
        b.keys().collect::<Vec<_>>(),
        "artifact set diverged between runs"
    );

    // The comparison must not be vacuous: the canonical v1 artifact set is
    // always present (metrics.json + bundled schema + report.md + 3 overview
    // charts + 2 layered overviews + graph.dot since v1.4 step 2 + one
    // per-component detail page per capped component).
    // m6: the schema artifact name comes from the product, so an additive
    // schema bump does not break the benchmark.
    for required in [
        "metrics.json",
        "report.md",
        "report.html",
        "findings-detail.md",
        "findings-detail.html",
        SCHEMA_ARTIFACT_NAME,
    ] {
        assert!(
            a.contains_key(required),
            "determinism proof: expected artifact {required} missing"
        );
    }
    let component_pages = a
        .keys()
        .filter(|n| {
            n.starts_with("charts/component-")
                && n.ends_with(".svg")
                && *n != "charts/component-coupling.svg"
        })
        .count();
    assert_eq!(
        a.keys().filter(|n| n.starts_with("charts/")).count(),
        6 + component_pages,
        "determinism proof: expected all three v1 charts + both layered overviews + graph.dot + one page per component"
    );
    assert!(
        component_pages >= 1,
        "determinism proof: expected at least one per-component detail page"
    );

    for (name, bytes1) in &a {
        let bytes2 = &b[name];
        let normalized = match name.as_str() {
            "metrics.json" => {
                let t1 = String::from_utf8_lossy(bytes1).into_owned();
                let t2 = String::from_utf8_lossy(bytes2).into_owned();
                (
                    mask_metrics_strict(&t1).into_bytes(),
                    mask_metrics_strict(&t2).into_bytes(),
                )
            }
            "report.md" => {
                let t1 = String::from_utf8_lossy(bytes1).into_owned();
                let t2 = String::from_utf8_lossy(bytes2).into_owned();
                let root = scan.to_string_lossy();
                (
                    mask_report_strict(&t1, &root).into_bytes(),
                    mask_report_strict(&t2, &root).into_bytes(),
                )
            }
            // C-split: findings-detail.md masks like report.md (provenance
            // no-op — no header surfaces — plus absolute-root substitution).
            "findings-detail.md" => {
                let t1 = String::from_utf8_lossy(bytes1).into_owned();
                let t2 = String::from_utf8_lossy(bytes2).into_owned();
                let root = scan.to_string_lossy();
                (
                    mask_report_strict(&t1, &root).into_bytes(),
                    mask_report_strict(&t2, &root).into_bytes(),
                )
            }
            // report.html + findings-detail.html + charts/*.svg +
            // charts/graph.dot + the bundled schema carry no provenance.
            _ => (bytes1.clone(), bytes2.clone()),
        };
        if normalized.0 != normalized.1 {
            // m5/N1: persist the diff copies. `mem::forget` deliberately leaks
            // the one small TempTree dir so the paths named below still exist
            // when CI reads the panic. This only happens on a real determinism
            // failure; the leak is bounded to that failure path.
            let diff_tree = TempTree::new("diff");
            let diff = write_diff(&diff_tree.root, name, &normalized.0, &normalized.1);
            std::mem::forget(diff_tree);
            panic!(
                "determinism FAIL: artifact `{name}` diverged between two runs; \
                 normalized copies were written to {diff} (kept for inspection)"
            );
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Benchmark-local STRICT masks (deliberately NOT the shared `common::*`
// helpers: `common::mask_golden_metrics` serde-round-trips — canonicalizing
// key order — and `common::mask_golden_report` deletes the whole §4
// ARCH-003 block. Both are too weak for a "byte-stable after provenance
// masking" claim.)
// ---------------------------------------------------------------------------

/// Neutralize ONLY `scan.started_at`, `scan.duration_ms` and
/// `scan.root_path` in a raw metrics.json by textual value substitution. No
/// serde round-trip: the producer's exact key order, indentation and every
/// other byte are preserved.
#[must_use]
fn mask_metrics_strict(raw: &str) -> String {
    let s = replace_json_value(raw, "started_at", "null");
    let s = replace_json_value(&s, "duration_ms", "null");
    replace_json_value(&s, "root_path", "\"<ROOT>\"")
}

/// Provenance/root-only report mask: [`common::mask_report_provenance`] (the
/// header `started` instant and `duration` figure) plus every literal /
/// Markdown-escaped occurrence of the absolute scan root. Unlike
/// `common::mask_golden_report` it does NOT strip the §4 findings block — the
/// two benchmark runs are the same build over identical bytes, so that block
/// must be byte-identical.
#[must_use]
fn mask_report_strict(raw: &str, root: &str) -> String {
    let forward = root.replace('\\', "/");
    let escaped = common::markdown_escaped(root);
    let escaped_forward = common::markdown_escaped(&forward);
    let mut masked = common::mask_report_provenance(raw);
    for needle in [
        root,
        forward.as_str(),
        escaped.as_str(),
        escaped_forward.as_str(),
    ] {
        if !needle.is_empty() {
            masked = masked.replace(needle, "<ROOT>");
        }
    }
    masked
}

/// Replace the value token following `"<key>":` with `replacement`, leaving
/// every other byte (key order, separators, whitespace) untouched. The three
/// provenance keys occur exactly once each in the v1 metrics.json — asserted,
/// so a second same-named key (e.g. a future nested `root_path`) fails loudly
/// instead of masking only the first occurrence.
fn replace_json_value(raw: &str, key: &str, replacement: &str) -> String {
    let needle = format!("\"{key}\":");
    assert_eq!(
        raw.matches(&needle).count(),
        1,
        "provenance key {key} must occur exactly once in metrics.json"
    );
    let Some(colon) = raw.find(&needle) else {
        return raw.to_string();
    };
    let mut start = colon + needle.len();
    while raw
        .as_bytes()
        .get(start)
        .is_some_and(u8::is_ascii_whitespace)
    {
        start += 1;
    }
    let end = json_value_end(raw, start);
    let mut out = String::with_capacity(raw.len() + replacement.len());
    out.push_str(&raw[..start]);
    out.push_str(replacement);
    out.push_str(&raw[end..]);
    out
}

/// Byte offset just past the JSON value starting at `start` (string, object,
/// array or scalar) — string/escape aware, so a quote inside a value cannot
/// truncate the scan.
fn json_value_end(raw: &str, start: usize) -> usize {
    let bytes = raw.as_bytes();
    match bytes.get(start) {
        Some(b'"') => {
            let mut i = start + 1;
            while i < bytes.len() {
                match bytes[i] {
                    b'\\' => i += 2,
                    b'"' => return i + 1,
                    _ => i += 1,
                }
            }
            bytes.len()
        }
        Some(&open @ (b'{' | b'[')) => {
            let close = if open == b'{' { b'}' } else { b']' };
            let mut depth = 0usize;
            let mut i = start;
            let mut in_str = false;
            while i < bytes.len() {
                let b = bytes[i];
                if in_str {
                    match b {
                        b'\\' => i += 2,
                        b'"' => {
                            in_str = false;
                            i += 1;
                        }
                        _ => i += 1,
                    }
                    continue;
                }
                match b {
                    b'"' => {
                        in_str = true;
                        i += 1;
                    }
                    _ if b == open => {
                        depth += 1;
                        i += 1;
                    }
                    _ if b == close => {
                        depth -= 1;
                        i += 1;
                        if depth == 0 {
                            return i;
                        }
                    }
                    _ => i += 1,
                }
            }
            bytes.len()
        }
        _ => {
            let mut i = start;
            while i < bytes.len()
                && !matches!(bytes[i], b',' | b'}' | b']')
                && !bytes[i].is_ascii_whitespace()
            {
                i += 1;
            }
            i
        }
    }
}

/// Persist both normalized artifacts into `dir` (a `TempTree` root, so they
/// are cleaned up on drop) and return a message naming both paths.
fn write_diff(dir: &Path, name: &str, run1: &[u8], run2: &[u8]) -> String {
    let sanitized = name.replace(['/', '\\'], "_");
    let base = dir.join(format!("{sanitized}.run"));
    let p1 = PathBuf::from(format!("{}.1", base.display()));
    let p2 = PathBuf::from(format!("{}.2", base.display()));
    fs::write(&p1, run1).expect("write diff run1");
    fs::write(&p2, run2).expect("write diff run2");
    format!("{} / {}", p1.display(), p2.display())
}

// ---------------------------------------------------------------------------
// Unlabeled real-code determinism rung.
// ---------------------------------------------------------------------------

/// One mixed-language scan over the repo's OWN committed corpora (`src/` Rust,
/// `examples/` C, `examples-rust/` Rust, `examples-python/` Python), snapshotted
/// read-only into a tempdir (so `target/` never inflates it) and run twice.
/// Counts only — no labels.
fn run_real_code_rung() {
    let tree = TempTree::new("real-code");
    let scan = tree.root.join("scan");
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    common::copy_tree_excluding(&repo.join("src"), &scan.join("src"), "target");
    common::copy_tree_excluding(&repo.join("examples"), &scan.join("examples"), "target");
    common::copy_tree_excluding(
        &repo.join("examples-rust"),
        &scan.join("examples-rust"),
        "target",
    );
    common::copy_tree_excluding(
        &repo.join("examples-python"),
        &scan.join("examples-python"),
        "target",
    );

    let out1 = tree.root.join("out1");
    let out2 = tree.root.join("out2");
    let doc = run_pipeline(&scan, &out1);
    let _second = run_pipeline(&scan, &out2);
    let determinism_ok = assert_byte_stable(&out1, &out2, &scan);

    let files = doc["scan"]["file_count"].as_u64().unwrap_or(0);
    let findings = doc["findings"].as_array().map_or(0, Vec::len);
    let summary = &doc["parse_health_summary"];
    println!("### Rung `real-code` (unlabeled, counts only)\n");
    println!(
        "Committed mixed scan: `src/` (Rust) + `examples/` (C) + `examples-rust/` (Rust) + `examples-python/` (Python).\n"
    );
    println!("- files scanned: {files}");
    println!("- findings: {findings}");
    println!(
        "- parse health: {} ok, {} partial, {} failed",
        summary["ok"].as_u64().unwrap_or(0),
        summary["partial"].as_u64().unwrap_or(0),
        summary["failed"].as_u64().unwrap_or(0)
    );
    println!(
        "- determinism: {}\n",
        if determinism_ok { "PASS" } else { "FAIL" }
    );

    // m4: a broken committed `src/`/`examples*` file must fail the suite, not
    // merely print. These corpora are committed-clean, so BOTH zero failures
    // AND zero partial parses are required (partial is NOT tolerated here).
    assert_eq!(
        summary["failed"].as_u64(),
        Some(0),
        "real-code rung: committed src/examples corpora have parse failures"
    );
    assert_eq!(
        summary["partial"].as_u64(),
        Some(0),
        "real-code rung: committed src/examples corpora have partial parses"
    );
}
