//! I2 cost-model v2 locks (plan `map-build`, spec `birdseye-map` F2).
//!
//! (a) canonical C trip-count extraction + clamp; (b) v1 formula unchanged
//! and v2 math pinned exactly; (c) unknown-trip honesty for Rust/Python;
//! (d) reachability-aware cap election + honest `top S of M` caption;
//! (e) byte-determinism under shuffled input + real double scan;
//! (f) the explicitly-named GENUINE PROXY (never ground truth).

mod common;

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::aggregator::{Provenance, assemble};
use perfpal::charts::layers::render_layer_file;
use perfpal::cli::run_with;
use perfpal::ir::{
    FileRecord, FunctionRecord, IncludeForm, LoopKind, LoopNode, ParseHealth, ParseStatus,
    ResolverTier, Span,
};
use perfpal::scanner::{FileUnit, FunctionUnit, scan};
use perfpal::schema::{
    FunctionStats, GraphsSection, IncludeEdgeDto, IncludeGraph, SCHEMA_VERSION, to_json,
};

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("perfpal-i2-{tag}-{}-{nanos}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

fn prov() -> Provenance {
    Provenance {
        root_path: ".".to_string(),
        started_at: "2026-10-21T00:00:00Z".to_string(),
        duration_ms: 0,
        tool_version: "test".to_string(),
    }
}

fn loop_node(kind: LoopKind, depth: u32, trip_count: Option<u64>) -> LoopNode {
    LoopNode {
        kind,
        depth,
        induction_vars: vec!["i".to_string()],
        span: Span::point(1, 1),
        resolver_tier: ResolverTier::Syntactic,
        macro_origin: false,
        trip_count,
        condition_is_zero: false,
    }
}

fn record(path: &str, name: &str, line: u32) -> FunctionRecord {
    FunctionRecord {
        id: format!("{path}#{name}#{line}:1"),
        name: name.to_string(),
        path: path.to_string(),
        span: Span::point(line, 1),
        ..FunctionRecord::default()
    }
}

fn unit(path: &str, language: &str, functions: Vec<FunctionUnit>) -> FileUnit {
    FileUnit {
        file: FileRecord {
            path: path.to_string(),
            language: language.to_string(),
            ..Default::default()
        },
        parse_health: ParseHealth {
            status: ParseStatus::Ok,
            ..Default::default()
        },
        functions,
        ..Default::default()
    }
}

fn stat(path: &str, reachable: bool, cost: u64) -> FunctionStats {
    FunctionStats {
        id: format!("{path}#f#1:1"),
        name: "f".to_string(),
        path: path.to_string(),
        span: Span::point(1, 1),
        param_count: 0,
        cyclomatic_complexity: 1,
        cognitive_complexity: 0,
        max_loop_depth: 0,
        alloc_site_count: 0,
        length_lines: 1,
        call_fanout: Some(0),
        call_fanin: Some(0),
        call_depth: Some(0),
        is_recursive: Some(false),
        idiom_hash: None,
        error_path: Some(false),
        alloc_family: None,
        cfg_gated: None,
        entry_kind: None,
        entry_confidence: None,
        reachable_from_entry: Some(reachable),
        function_cost_v2: Some(cost),
        unknown_trip_loops: 0,
    }
}

/// v1 cost replicated from the documented formula — used ONLY to prove v2 is
/// non-decreasing and to pin that the v1 coefficients did not drift.
fn v1_cost(f: &FunctionStats) -> u64 {
    u64::from(f.cyclomatic_complexity)
        + u64::from(f.cognitive_complexity)
        + 2 * u64::from(f.max_loop_depth)
        + 3 * u64::from(f.alloc_site_count)
        + u64::from(f.call_fanout.unwrap_or(0))
        + u64::from(f.call_fanin.unwrap_or(0))
        + u64::from(f.call_depth.unwrap_or(0))
        + if f.is_recursive.unwrap_or(false) {
            2
        } else {
            0
        }
        + if f.error_path.unwrap_or(false) { 1 } else { 0 }
        + if f.alloc_family.is_some() { 1 } else { 0 }
}

// ---------------------------------------------------------------------------
// (a) C trip extraction
// ---------------------------------------------------------------------------

fn scan_c_source(tag: &str, source: &str) -> Vec<FileUnit> {
    let root = temp_dir(tag);
    fs::write(root.join("main.c"), source).expect("write C fixture");
    let units = scan(&root).expect("scan C fixture");
    fs::remove_dir_all(&root).ok();
    units
}

#[test]
fn c_trip_count_extraction_is_canonical_only() {
    let source = r#"
void f(void) {
    for (int i = 0; i < 10; i++) {}
    for (int i = 0; i <= 10; i++) {}
    while (1) {}
    for (int i = 0; i < LIMIT; i++) {}
    for (int i = 0; i < n; i++) {}
    for (int i = 10; i > 0; i--) {}
    for (int i = 0; i < 10; i -= 1) {}
    for (int i = 0; i < 10; i += 2) {}
    for (int i = 0; i < 1000001; i++) {}
    for (int i = 0; i < 18446744073709551615u; i++) {}
}
"#;
    let units = scan_c_source("trip", source);
    let loops: Vec<Option<u64>> = units[0].functions[0]
        .loops
        .iter()
        .map(|l| l.trip_count)
        .collect();
    assert_eq!(
        loops,
        vec![
            Some(10),        // < 10
            Some(11),        // <= 10
            None,            // while
            None,            // macro/identifier bound
            None,            // variable bound
            None,            // descending
            None,            // i -= 1 is not a unit step
            None,            // i += 2 is not a unit step
            Some(1_000_000), // 1000001 clamps at MAX_TRIP
            Some(1_000_000), // absurd literal clamps too
        ],
        "trip extraction must admit only the canonical counted shape"
    );
}

// ---------------------------------------------------------------------------
// (b) v1 unchanged + v2 exact formula through the real aggregator
// ---------------------------------------------------------------------------

#[test]
fn v1_is_unchanged_and_v2_math_is_pinned() {
    // v1 pin (same numbers as charts::tests::function_cost_sums_documented_terms):
    // 3 + 2 + 2*1 + 3*2 + 4 + 1 + 2 + 2 + 1 + 1 = 24.
    let costly = FunctionStats {
        cyclomatic_complexity: 3,
        cognitive_complexity: 2,
        max_loop_depth: 1,
        alloc_site_count: 2,
        call_fanout: Some(4),
        call_fanin: Some(1),
        call_depth: Some(2),
        is_recursive: Some(true),
        error_path: Some(true),
        alloc_family: Some("real_alloc".to_string()),
        ..stat("a.c", false, 0)
    };
    assert_eq!(v1_cost(&costly), 24, "v1 coefficients must not drift");

    // v2 through the aggregator: base = 3 + 2 + 2*2 + 3*2 + 0 + 0 + 0
    // (call shape is empty; recursive false) + 1 (error_path) + 1 (alloc)
    // = 17. Loops add 10*(1+1) + 5*(1+2) + 1*(1+3) = 20 + 15 + 4.
    // Expected cost_v2 = 56, unknown_trip_loops = 1.
    let function = FunctionUnit {
        record: FunctionRecord {
            cyclomatic_complexity: 3,
            cognitive_complexity: 2,
            max_loop_depth: 2,
            alloc_site_count: 2,
            error_path: Some(true),
            alloc_family: Some("real_alloc".to_string()),
            ..record("a.c", "f", 1)
        },
        loops: vec![
            loop_node(LoopKind::For, 1, Some(10)),
            loop_node(LoopKind::For, 2, Some(5)),
            loop_node(LoopKind::While, 3, None),
        ],
        ..Default::default()
    };
    let doc = assemble(&[unit("a.c", "c", vec![function])], &[], &prov());
    let f = &doc.stats.functions[0];
    assert_eq!(f.function_cost_v2, Some(56));
    assert_eq!(f.unknown_trip_loops, 1);
    assert_eq!(
        v1_cost(f),
        17,
        "v1 base still computed from the same fields"
    );
    assert!(
        f.function_cost_v2.unwrap() >= v1_cost(f),
        "v2 must never be cheaper than v1"
    );
}

// ---------------------------------------------------------------------------
// (c) unknown-trip honesty: non-canonical Rust/Python stay unknown; C canonical is known
// ---------------------------------------------------------------------------

#[test]
fn unknown_trip_loops_are_honest_for_rust_and_python() {
    // Real backends, not fabricated IR: Rust/Python must leave every loop
    // trip count `None` in this increment.
    // I2b: Rust literal ranges are now KNOWN, so this honesty fixture uses a
    // non-literal bound (`parts.len()`) to stay honestly unknown.
    let rust_root = temp_dir("honest-rust");
    fs::write(
        rust_root.join("lib.rs"),
        "pub fn run(parts: &[i32]) { for i in 0..parts.len() { let _ = parts[i]; } }\n",
    )
    .expect("write Rust fixture");
    // I2c: Python literal `range(10)` is now KNOWN, so the honesty fixture
    // uses a non-literal bound (`range(len(items))`) to stay honestly unknown.
    let python_root = temp_dir("honest-python");
    fs::write(
        python_root.join("app.py"),
        "def run(items):\n    for i in range(len(items)):\n        print(items[i])\n",
    )
    .expect("write Python fixture");

    let rust_units = scan(&rust_root).expect("scan Rust fixture");
    let python_units = scan(&python_root).expect("scan Python fixture");
    for u in rust_units.iter().chain(python_units.iter()) {
        for f in &u.functions {
            for l in &f.loops {
                assert_eq!(
                    l.trip_count, None,
                    "{}: non-canonical trip counts stay honest-unknown",
                    f.record.id
                );
            }
        }
    }

    let c_units = scan_c_source(
        "honest",
        "void f(void) { for (int i = 0; i < 10; i++) { } }\n",
    );
    let doc = assemble(
        &[
            c_units[0].clone(),
            rust_units[0].clone(),
            python_units[0].clone(),
        ],
        &[],
        &prov(),
    );

    let c = doc.stats.functions.iter().find(|f| f.name == "f").unwrap();
    assert_eq!(c.unknown_trip_loops, 0);
    assert!(c.function_cost_v2.is_some());
    assert_eq!(c_units[0].functions[0].loops[0].trip_count, Some(10));

    let rust = doc
        .stats
        .functions
        .iter()
        .find(|f| f.path == "lib.rs" && f.name == "run")
        .unwrap();
    assert!(rust.unknown_trip_loops > 0);
    assert!(rust.function_cost_v2.is_some());
    let python = doc
        .stats
        .functions
        .iter()
        .find(|f| f.path == "app.py" && f.name == "run")
        .unwrap();
    assert!(python.unknown_trip_loops > 0);
    assert!(python.function_cost_v2.is_some());
    fs::remove_dir_all(&rust_root).ok();
    fs::remove_dir_all(&python_root).ok();
}

// ---------------------------------------------------------------------------
// (d) reachability-aware cap election
// ---------------------------------------------------------------------------

fn cap_fixture(
    reachable_extra: usize,
    non_reachable_extra: usize,
) -> (GraphsSection, Vec<FunctionStats>) {
    // 49 high-degree chain files + one degree-0 path leader = exactly the
    // 50 base slots. Every extra file has degree 0 and sorts after them.
    let mut paths: Vec<String> = (0..49).map(|i| format!("f{i:02}.c")).collect();
    paths.push("a0.c".to_string());
    let mut edges = Vec::new();
    for pair in paths[..49].windows(2) {
        edges.push(IncludeEdgeDto {
            from: pair[0].clone(),
            to: Some(pair[1].clone()),
            form: IncludeForm::Quoted,
            resolved: true,
            span: Span::point(1, 1),
        });
    }
    let reachable: Vec<String> = (0..reachable_extra).map(|i| format!("r{i:02}.c")).collect();
    let cut: Vec<String> = (0..non_reachable_extra)
        .map(|i| format!("z{i:02}.c"))
        .collect();
    paths.extend(reachable.iter().cloned());
    paths.extend(cut.iter().cloned());

    let graphs = GraphsSection {
        include_graph: IncludeGraph {
            nodes: paths
                .iter()
                .map(|path| perfpal::schema::GraphNode { path: path.clone() })
                .collect(),
            edges,
        },
        call_graph: None,
    };
    let mut functions: Vec<FunctionStats> =
        reachable.iter().map(|path| stat(path, true, 50)).collect();
    functions.extend(cut.iter().map(|path| stat(path, false, 1)));
    // Costs for base nodes keep the reserve election deterministic.
    functions.push(stat("a0.c", false, 1));
    (graphs, functions)
}

#[test]
fn reachable_low_degree_node_survives_the_top_50_cut() {
    let (graphs, functions) = cap_fixture(9, 1);
    let svg = String::from_utf8(render_layer_file(&graphs, &functions)).expect("SVG UTF-8");
    // 50 base + 9 reserved = 59 drawn of 60 total.
    assert_eq!(svg.matches("width=\"170\"").count(), 59, "shown node count");
    for i in 0..9 {
        assert!(
            svg.contains(&format!(">r{i:02}.c<")),
            "entry-reachable low-degree r{i:02}.c must survive the cut"
        );
    }
    assert!(
        !svg.contains(">z00.c<"),
        "non-reachable 51st node must be cut"
    );
    assert!(
        svg.contains("showing top 59 of 60 files"),
        "honest S/M caption missing:\n{svg}"
    );
}

#[test]
fn reachable_overflow_is_bounded_at_ten() {
    let (graphs, functions) = cap_fixture(15, 10);
    let svg = String::from_utf8(render_layer_file(&graphs, &functions)).expect("SVG UTF-8");
    // 50 base + at most REACHABLE_OVERFLOW (10) = 60 drawn of 75 total.
    assert_eq!(svg.matches("width=\"170\"").count(), 60, "overflow bound");
    for i in 0..10 {
        assert!(svg.contains(&format!(">r{i:02}.c<")), "reserved r{i:02}");
    }
    for i in 10..15 {
        assert!(
            !svg.contains(&format!(">r{i:02}.c<")),
            "overflow beyond 10 must stay cut"
        );
    }
    assert!(
        svg.contains("showing top 60 of 75 files"),
        "honest S/M caption"
    );
}

// ---------------------------------------------------------------------------
// (e) byte-determinism
// ---------------------------------------------------------------------------

#[test]
fn shuffled_input_order_produces_identical_output() {
    let a = unit(
        "a.c",
        "c",
        vec![FunctionUnit {
            record: record("a.c", "a", 1),
            loops: vec![loop_node(LoopKind::For, 1, Some(3))],
            ..Default::default()
        }],
    );
    let b = unit(
        "b.rs",
        "rust",
        vec![FunctionUnit {
            record: record("b.rs", "b", 1),
            loops: vec![loop_node(LoopKind::For, 1, None)],
            ..Default::default()
        }],
    );
    let forward = to_json(&assemble(&[a.clone(), b.clone()], &[], &prov())).unwrap();
    let reverse = to_json(&assemble(&[b, a], &[], &prov())).unwrap();
    assert_eq!(forward, reverse, "input order must not affect output bytes");
}

fn list_chart_files(out: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(out.join("charts"))
        .expect("charts dir")
        .flatten()
    {
        let name = entry.file_name().into_string().expect("utf8 name");
        files.insert(name, fs::read(entry.path()).expect("read chart"));
    }
    files
}

#[test]
fn real_pipeline_double_scan_is_byte_identical() {
    let root = temp_dir("double");
    fs::write(
        root.join("main.c"),
        "void sink(int);\nint main(void) { for (int i = 0; i < 10; i++) { sink(i); } return 0; }\n",
    )
    .expect("write fixture");

    let run = |tag: &str| {
        let out = temp_dir(tag).join("report");
        let code = run_with(argv(&[
            "--output",
            out.to_string_lossy().as_ref(),
            root.to_string_lossy().as_ref(),
        ]));
        assert_eq!(code, 0, "double-scan run must exit 0");
        out
    };
    let first = run("double-a");
    let second = run("double-b");

    let schema_name = format!("metrics.schema.v{SCHEMA_VERSION}.json");
    assert_eq!(
        fs::read(first.join(&schema_name)).unwrap(),
        fs::read(second.join(&schema_name)).unwrap(),
        "bundled schema must be byte-identical"
    );
    assert_eq!(
        list_chart_files(&first),
        list_chart_files(&second),
        "every chart stem must be byte-identical"
    );
    let masked_first = common::mask_golden_metrics(
        &fs::read_to_string(first.join("metrics.json")).expect("metrics 1"),
    );
    let masked_second = common::mask_golden_metrics(
        &fs::read_to_string(second.join("metrics.json")).expect("metrics 2"),
    );
    assert_eq!(masked_first, masked_second, "masked metrics must match");
    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (f) GENUINE PROXY — explicitly a proxy, never ground truth
// ---------------------------------------------------------------------------

/// Compute the PROXY predicate `cost_vX > 0 ∧ has_loop ∧ trip_known ∧
/// reachable_from_entry`. This is NOT the manual GENUINE verdict from
/// docs/evidence/findings-deep-dive.md:29-36 — true GENUINE requires human
/// triage of reachability, trip magnitude and per-iteration cost. The proxy
/// exists only to prove cost-v2 does not shrink the structurally-plausible
/// candidate share.
fn proxy_share(units: &[FileUnit], functions: &[FunctionStats], v2: bool) -> usize {
    let mut loops_by_id: BTreeMap<&str, (bool, bool)> = BTreeMap::new();
    for u in units {
        for f in &u.functions {
            let has_loop = !f.loops.is_empty();
            let trip_known = f.loops.iter().any(|l| l.trip_count.is_some());
            loops_by_id.insert(f.record.id.as_str(), (has_loop, trip_known));
        }
    }
    functions
        .iter()
        .filter(|f| {
            let cost = if v2 {
                f.function_cost_v2.unwrap_or(0)
            } else {
                v1_cost(f)
            };
            let (has_loop, trip_known) = loops_by_id
                .get(f.id.as_str())
                .copied()
                .unwrap_or((false, false));
            cost > 0 && has_loop && trip_known && f.reachable_from_entry == Some(true)
        })
        .count()
}

/// Non-regression across the shipped corpora: cost-v2 must never shrink the
/// structurally-plausible candidate set. On corpora with no canonical
/// trip-count loops the predicate is version-independent, so this is a
/// NON-REGRESSION lock only; the "measured up" proof is
/// `genuine_proxy_uplift_is_measured_on_c` below.
#[test]
fn genuine_proxy_share_not_lowered() {
    // The four golden/fixture corpora (same trees the p6 goldens derive
    // from). GENUINE itself stays manual triage (see doc comment above).
    let roots = [
        "tests/fixtures/e2e",
        "tests/fixtures/e2e-mixed",
        "examples-rust",
        "examples-python",
    ];
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0usize;
    for rel in roots {
        let units = scan(&manifest.join(rel)).expect("scan proxy corpus");
        let doc = assemble(&units, &[], &prov());
        let total = doc.stats.functions.len();
        if total == 0 {
            continue;
        }
        let v1 = proxy_share(&units, &doc.stats.functions, false);
        let v2 = proxy_share(&units, &doc.stats.functions, true);
        assert!(
            v2 >= v1,
            "{rel}: v2 proxy share ({v2}/{total}) must not be lower than v1 ({v1}/{total})"
        );
        // Honest unknown: a corpus with no canonical trip count must yield
        // an empty candidate share, never a fabricated one.
        let any_known_trip = units
            .iter()
            .flat_map(|u| u.functions.iter())
            .any(|f| f.loops.iter().any(|l| l.trip_count.is_some()));
        if !any_known_trip {
            assert_eq!(
                v2, 0,
                "{rel}: no known trip counts => proxy share must be 0"
            );
        }
        for f in &doc.stats.functions {
            let cost_v2 = f.function_cost_v2.expect("v2 cost populated");
            assert!(
                cost_v2 >= v1_cost(f),
                "{}: v2 {cost_v2} < v1 {}",
                f.id,
                v1_cost(f)
            );
        }
        checked += 1;
    }
    assert!(checked >= 4, "all four corpora must be exercised");
}

/// The "measured up" half of the GENUINE clause: on a real C scan, canonical
/// literal-bound loops must actually move the heat (v2 > v1), and at least
/// one entry-REACHABLE function with a >=16-trip loop must enter the
/// candidate set. Still a structural proxy — a true GENUINE verdict requires
/// manual triage of reachability, trip magnitude and per-iteration cost
/// (docs/evidence/findings-deep-dive.md:29-36); that measurement lands in I5.
///
/// The loops live in `main` on purpose: default C scans are syntactic-only,
/// so only entry functions are reachable (resolved edges need the semantic
/// tier). Putting loops in the entry keeps the reachability clause honest
/// without fabricating call facts.
#[test]
fn genuine_proxy_uplift_is_measured_on_c() {
    let source = r#"
int main(void) {
    int s = 0;
    for (int i = 0; i < 32; i++) { s += i; }
    for (int j = 0; j < 64; j++) { s += j; }
    return s;
}
"#;
    let units = scan_c_source("uplift", source);
    let doc = assemble(&units, &[], &prov());

    let main = doc
        .stats
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("main present");
    let v1 = v1_cost(main);
    let v2 = main.function_cost_v2.expect("v2 populated");
    assert!(v2 > v1, "trip weighting must lift cost: v2 {v2} <= v1 {v1}");
    assert_eq!(
        main.unknown_trip_loops, 0,
        "both loops have literal bounds, so none may be unknown"
    );
    assert_eq!(
        main.reachable_from_entry,
        Some(true),
        "an entry function is reachable at distance 0"
    );

    let lifted = doc
        .stats
        .functions
        .iter()
        .filter(|f| f.function_cost_v2.is_some_and(|c| c > v1_cost(f)))
        .count();
    assert!(
        lifted > 0,
        "at least one function must show trip-weighted uplift"
    );

    // Candidate share: reachable ∧ cost_v2 > 0 ∧ a >=16-trip canonical loop.
    let mut big_trip: BTreeMap<&str, bool> = BTreeMap::new();
    for u in &units {
        for f in &u.functions {
            big_trip.insert(
                f.record.id.as_str(),
                f.loops
                    .iter()
                    .any(|l| l.trip_count.is_some_and(|t| t >= 16)),
            );
        }
    }
    let candidates = doc
        .stats
        .functions
        .iter()
        .filter(|f| {
            f.function_cost_v2.unwrap_or(0) > 0
                && f.reachable_from_entry == Some(true)
                && big_trip.get(f.id.as_str()).copied().unwrap_or(false)
        })
        .count();
    assert!(
        candidates > 0,
        "C fixture must yield >=1 reachable function with a >=16-trip loop"
    );
}
