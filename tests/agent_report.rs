//! Track C integration suite (plan `agent-report`, DoD 13–20).
//!
//! Locks the CLI wiring and the contract in `docs/specs/agent-report.md`:
//! - double run of the same command ⇒ byte-identical artifacts (§12);
//! - the JSON completeness arithmetic (§6.1–§6.2);
//! - the eleven-row state matrix (§11) renders the fixed seven-section
//!   skeleton with named stubs;
//! - every `detail_anchor` resolves inside `findings-detail.html`;
//! - no `<script>`/network token in either artifact;
//! - the emitted artifact set (nine fixed files including `charts/graph.dot`);
//! - the committed C1 sample is reproduced by the wired CLI;
//! - A's step-12 human-page link resolves to both artifacts;
//! - the ≤ 80 KB budget on the committed goldens (the three external
//!   `perfpal-test` scans only with `PERFPAL_EXTERNAL_SCANS=1`).

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::aggregator::{Provenance, assemble};
use perfpal::cli::run_with;
use perfpal::reporter::agent_report;
use perfpal::schema::MetricsDocument;

/// Spec §7: the fixed seven-section skeleton. A wording change is a contract
/// edit — these strings are asserted, not the rendered body.
const SECTION_HEADINGS: [&str; 7] = [
    "# Perfpal Agent Report",
    "## How to use this report",
    "## Data quality",
    "## Start here — ranked actions",
    "## Action index by rule",
    "## Profile queue",
    "## What this scan cannot see",
];

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn manifest(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-agent-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// A not-yet-existing output directory: the writer refuses to replace an
/// existing tree that carries no `metrics.json`, and these tests never pass
/// `--force` (the normal-path contract).
fn out_dir(tag: &str) -> PathBuf {
    let parent = temp_dir(tag);
    parent.join("out")
}

fn write_source(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create source parent");
    }
    fs::write(path, body).expect("write source");
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

fn run_scan(fixture: &Path, out: &Path) -> i32 {
    let out_arg = out.to_string_lossy().into_owned();
    let fixture_arg = fixture.to_string_lossy().into_owned();
    run_with(argv(&["--output", &out_arg, &fixture_arg]))
}

fn read_artifact(out: &Path, name: &str) -> String {
    fs::read_to_string(out.join(name)).unwrap_or_else(|error| panic!("read {name}: {error}"))
}

fn read_json(out: &Path) -> serde_json::Value {
    serde_json::from_str(&read_artifact(out, "agent-actions.json")).expect("agent-actions.json")
}

/// Build the same `MetricsDocument` the cli aggregates, without the writer —
/// the state matrix needs documents the committed fixtures do not cover.
fn assemble_doc(root: &Path) -> MetricsDocument {
    let root = root.canonicalize().expect("canonicalize fixture root");
    let mut units = perfpal::scanner::scan(&root).expect("scan fixture tree");
    perfpal::resolver::resolve(&mut units, &root);
    let findings = perfpal::analyzer::analyze(&units);
    let provenance = Provenance {
        root_path: root.display().to_string(),
        started_at: "2026-01-01T00:00:00Z".to_string(),
        duration_ms: 0,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    assemble(&units, &findings, &provenance)
}

fn render_pair(doc: &MetricsDocument) -> (String, String) {
    (
        agent_report::render_actions_json(doc),
        agent_report::render_markdown(doc),
    )
}

fn assert_skeleton(doc: &MetricsDocument, state: &str) {
    let (json, markdown) = render_pair(doc);
    for heading in SECTION_HEADINGS {
        assert!(
            markdown.contains(heading),
            "[{state}] missing section heading {heading:?}"
        );
    }
    let value: serde_json::Value = serde_json::from_str(&json).expect("state JSON parses");
    assert_eq!(value["contract_version"], 1, "[{state}] contract_version");
    assert_eq!(
        value["artifact"], "agent-actions",
        "[{state}] artifact name"
    );
}

/// A function-granularity, non-component finding — the unit the ranked action
/// list is built from.
fn cloneable_action(doc: &MetricsDocument) -> perfpal::schema::FindingEntry {
    doc.findings
        .iter()
        .find(|finding| finding.symbol.is_some() && !finding.rule_id.ends_with("-ARCH-002"))
        .unwrap_or_else(|| panic!("fixture has no function-granularity finding"))
        .clone()
}

// ---------------------------------------------------------------------------
// 1. Determinism — the same command twice on the same tree (§12)
// ---------------------------------------------------------------------------

#[test]
fn agent_report_double_run_bytes_identical() {
    let fixture = manifest("tests/fixtures/e2e-mixed");
    let first = out_dir("det-a");
    let second = out_dir("det-b");
    assert_eq!(run_scan(&fixture, &first), 0, "first run");
    assert_eq!(run_scan(&fixture, &second), 0, "second run");

    for name in ["agent-report.md", "agent-actions.json"] {
        let a = fs::read(first.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        let b = fs::read(second.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(a, b, "{name} drifted between two runs of the same command");
    }
}

// ---------------------------------------------------------------------------
// 2. Completeness arithmetic (§6.1–§6.2)
// ---------------------------------------------------------------------------

#[test]
fn agent_report_completeness_arithmetic() {
    let out = out_dir("complete");
    assert_eq!(run_scan(&manifest("tests/fixtures/e2e-mixed"), &out), 0);
    let doc = read_json(&out);

    let actions = doc["actions"].as_array().expect("actions array");
    let complete = doc["complete_action_count"]
        .as_u64()
        .expect("complete_action_count") as usize;
    assert_eq!(actions.len(), complete, "JSON must carry every action");
    assert_eq!(
        actions.len(),
        actions.iter().filter_map(|a| a["rank"].as_u64()).count(),
        "every action carries a rank"
    );
    for (index, action) in actions.iter().enumerate() {
        assert_eq!(
            action["rank"].as_u64(),
            Some(index as u64 + 1),
            "ranks are contiguous from 1"
        );
        assert!(
            action["fix_hint"].is_string(),
            "every action carries a fix hint"
        );
        assert!(
            action["detail_anchor"]
                .as_str()
                .is_some_and(|a| !a.is_empty()),
            "every action carries a detail anchor"
        );
    }

    let queue = doc["profile_queue"].as_array().expect("profile queue");
    let cap = doc["profile_queue_cap"].as_u64().expect("cap") as usize;
    let profile_complete = doc["complete_profile_count"]
        .as_u64()
        .expect("complete_profile_count") as usize;
    assert_eq!(
        queue.len(),
        cap.min(profile_complete),
        "profile cap arithmetic"
    );
}

// ---------------------------------------------------------------------------
// 3. The eleven-row state matrix (§11)
// ---------------------------------------------------------------------------

#[test]
fn agent_report_state_matrix_renders_fixed_skeleton() {
    // Row 1 — application scan (the committed mixed fixture).
    let application = assemble_doc(&manifest("tests/fixtures/e2e-mixed"));
    assert_skeleton(&application, "application scan");

    // Row 2 — library-root: production code, no `main`.
    let library_root = temp_dir("state-library");
    write_source(
        &library_root,
        "src/lib.c",
        "int add(int a, int b) { return a + b; }\n",
    );
    let library_doc = assemble_doc(&library_root);
    assert_skeleton(&library_doc, "library-root");

    // Row 3 — no-entry scan: the same document must make no entry claim.
    let library_md = agent_report::render_markdown(&library_doc);
    assert!(
        !library_md.to_lowercase().contains("entry"),
        "a no-entry scan must not claim an entry point"
    );

    // Row 4 — empty scan (0 files).
    let empty_root = temp_dir("state-empty");
    let empty_doc = assemble_doc(&empty_root);
    assert_skeleton(&empty_doc, "empty scan");
    let empty_md = agent_report::render_markdown(&empty_doc);
    for stub in ["no findings", "no functions", "no rules fired"] {
        assert!(empty_md.contains(stub), "empty scan missing stub {stub:?}");
    }
    assert!(
        empty_md.contains("parse: 0 ok / 0 partial / 0 failed"),
        "an empty scan reports real parse counts, never a silent drop"
    );
    // The `parse: not recorded` stub is the absent-summary path (§11); the
    // aggregator always emits a summary, so exercise it by direct render.
    let mut no_summary = empty_doc.clone();
    no_summary.parse_health_summary = None;
    assert!(
        agent_report::render_markdown(&no_summary).contains("parse: not recorded"),
        "absent parse summary renders the named stub"
    );
    assert!(
        empty_md.contains("languages: c, js, python, rust, ts, tsx"),
        "empty scan prints the producer's supported set verbatim"
    );
    let empty_json: serde_json::Value =
        serde_json::from_str(&agent_report::render_actions_json(&empty_doc)).expect("empty JSON");
    assert!(
        empty_json["scan"]["loc_total"].is_null(),
        "empty scan serializes loc_total as null"
    );

    // Row 5 — all-test/bench scan: non-production rows are labelled, not dropped.
    let tests_root = temp_dir("state-tests");
    write_source(
        &tests_root,
        "tests/bench.c",
        "void work(int n) {\n  for (int i = 0; i < n; i++) {\n    char *p = malloc(64);\n    if (p) { p[0] = (char)i; free(p); }\n  }\n}\n",
    );
    let tests_doc = assemble_doc(&tests_root);
    assert_skeleton(&tests_doc, "all-test scan");
    let tests_json: serde_json::Value =
        serde_json::from_str(&agent_report::render_actions_json(&tests_doc)).expect("tests JSON");
    let tests_actions = tests_json["actions"].as_array().expect("actions");
    assert!(
        !tests_actions.is_empty(),
        "alloc-in-loop must fire in tests/"
    );
    assert!(
        tests_actions
            .iter()
            .any(|action| action["path_class"] != "production"),
        "non-production actions keep their path class"
    );
    assert!(
        agent_report::render_markdown(&tests_doc).contains("demotions:"),
        "the demotions line prints the non-production/startup/no-loop counts"
    );

    // Row 6 — capped/large scan: top 25 in detail, complete JSON.
    let mut capped = application.clone();
    let base = cloneable_action(&application);
    capped.findings = (0..30)
        .map(|index| {
            let mut finding = base.clone();
            finding.finding_id = format!("{}:{}", base.finding_id, index);
            finding
        })
        .collect();
    assert_skeleton(&capped, "capped scan");
    let capped_json: serde_json::Value =
        serde_json::from_str(&agent_report::render_actions_json(&capped)).expect("capped JSON");
    assert_eq!(capped_json["complete_action_count"], 30);
    assert_eq!(
        capped_json["actions"].as_array().expect("actions").len(),
        30,
        "the JSON is complete beyond the inline cap"
    );
    let capped_md = agent_report::render_markdown(&capped);
    assert!(
        capped_md.contains("showing top 25 of 30 actions"),
        "the inline cap is printed with N of M"
    );
    assert_eq!(
        capped_md.matches("\n### ").count(),
        25,
        "the inline detail renders exactly the top-25 cap"
    );

    // Row 7 — partial-parse scan: counts are reported, never silently dropped.
    let broken_root = temp_dir("state-partial");
    write_source(&broken_root, "src/broken.c", "void broken( { \n");
    let broken_doc = assemble_doc(&broken_root);
    assert_skeleton(&broken_doc, "partial-parse scan");
    let broken_md = agent_report::render_markdown(&broken_doc);
    assert!(
        broken_md.contains("parse: 0 ok / 1 partial / 0 failed"),
        "a broken file is counted as partial, never silently dropped"
    );

    // Row 8 — single-file scan.
    let single_root = temp_dir("state-single");
    write_source(&single_root, "only.c", "int main(void) { return 0; }\n");
    assert_skeleton(&assemble_doc(&single_root), "single-file scan");

    // Row 9 — v1-only cost scan: v1 anchors, no v2 share on a row.
    let mut v1 = application.clone();
    for function in &mut v1.stats.functions {
        function.function_cost_v2 = None;
    }
    assert_skeleton(&v1, "v1-only cost scan");
    let v1_json: serde_json::Value =
        serde_json::from_str(&agent_report::render_actions_json(&v1)).expect("v1 JSON");
    let v1_actions = v1_json["actions"].as_array().expect("actions");
    assert!(
        v1_actions
            .iter()
            .any(|action| action["function"].is_object()),
        "the v1-only fixture still carries function sub-objects"
    );
    for action in v1_actions {
        if action["function"].is_object() {
            assert_eq!(
                action["function"]["cost_scale"], "v1",
                "v1-only rows read v1, never a v2 share"
            );
        }
    }
    assert!(
        agent_report::render_markdown(&v1).contains("static cost units (v1)"),
        "v1-only anchors are labelled"
    );

    // Row 10 — semantic-clang scan: the tier caveat switches wording.
    let mut semantic = application.clone();
    for finding in &mut semantic.findings {
        finding.resolver_tier = perfpal::ir::ResolverTier::Semantic;
    }
    assert_skeleton(&semantic, "semantic scan");
    let semantic_md = agent_report::render_markdown(&semantic);
    assert!(
        semantic_md.contains("**Resolver tiers** (`resolver_tier: semantic`)"),
        "semantic scans print the resolver-tier caveat"
    );
    assert!(
        !semantic_md.contains("Syntactic tier only"),
        "semantic scans make no syntactic-only claim"
    );

    // Row 11 — all-component scan: every finding is ARCH-002.
    let mut components = application.clone();
    let archived = components.findings[0].clone();
    components.findings = (0..2)
        .map(|index| {
            let mut finding = archived.clone();
            finding.rule_id = "-ARCH-002".to_string();
            finding.finding_id = format!("PERF-C-ARCH-002:src:{index}");
            finding
        })
        .collect();
    assert_skeleton(&components, "all-component scan");
    let components_json: serde_json::Value =
        serde_json::from_str(&agent_report::render_actions_json(&components))
            .expect("components JSON");
    assert_eq!(components_json["complete_action_count"], 0);
    assert_eq!(
        components_json["data_quality"]["component_findings_excluded"], 2,
        "the excluded component aggregate is disclosed, never dropped"
    );
    let components_md = agent_report::render_markdown(&components);
    assert!(
        components_md.contains(agent_report::STUB_NO_FINDINGS),
        "an empty action list renders the fixed stub"
    );
}

// ---------------------------------------------------------------------------
// 4. A's step-12 link slot (§14.15)
// ---------------------------------------------------------------------------

#[test]
fn agent_report_human_page_links_to_agent_report() {
    let out = out_dir("links");
    assert_eq!(run_scan(&manifest("tests/fixtures/e2e-mixed"), &out), 0);
    let html = read_artifact(&out, "report.html");
    for href in ["agent-report.md", "agent-actions.json"] {
        assert!(
            html.contains(&format!("href=\"{href}\"")),
            "report.html must link {href} (A's step-12 slot)"
        );
        assert!(
            out.join(href).is_file(),
            "the linked artifact {href} must exist in the same directory"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. detail_anchor resolves in findings-detail.html
// ---------------------------------------------------------------------------

#[test]
fn agent_report_detail_anchor_resolves_in_findings_detail() {
    let out = out_dir("anchors");
    assert_eq!(run_scan(&manifest("tests/fixtures/e2e-mixed"), &out), 0);
    let doc = read_json(&out);
    let detail = read_artifact(&out, "findings-detail.html");
    let mut checked = 0;
    for action in doc["actions"].as_array().expect("actions") {
        let anchor = action["detail_anchor"].as_str().expect("detail_anchor");
        assert!(
            detail.contains(&format!("id=\"{anchor}\"")),
            "detail_anchor {anchor:?} does not resolve in findings-detail.html"
        );
        checked += 1;
    }
    assert!(checked > 0, "the fixture emits at least one action");
}

// ---------------------------------------------------------------------------
// 6. No script, no network (§14.14, §15 non-goals)
// ---------------------------------------------------------------------------

#[test]
fn agent_report_artifacts_carry_no_script_or_network() {
    let out = out_dir("inert");
    assert_eq!(run_scan(&manifest("tests/fixtures/e2e-mixed"), &out), 0);
    for name in ["agent-report.md", "agent-actions.json"] {
        let body = read_artifact(&out, name);
        for token in [
            "<script",
            "</script",
            "javascript:",
            "http://",
            "https://",
            "fetch(",
            "XMLHttpRequest",
            "Date.now",
        ] {
            assert!(
                !body.contains(token),
                "{name} contains forbidden token {token:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 7. Emitted artifact set (§14.14)
// ---------------------------------------------------------------------------

#[test]
fn agent_report_emitted_artifact_set() {
    let out = out_dir("artifacts");
    assert_eq!(run_scan(&manifest("tests/fixtures/e2e-mixed"), &out), 0);
    for name in [
        "metrics.json",
        "metrics.schema.v1.9.json",
        "report.md",
        "report.html",
        "findings-detail.html",
        "profile-queue.md",
        "profile-queue.html",
        "agent-report.md",
        "agent-actions.json",
        "charts/graph.dot",
    ] {
        assert!(out.join(name).is_file(), "{name} missing from the scan");
    }
}

// ---------------------------------------------------------------------------
// 8. The committed C1 sample is reproduced by the wired CLI (§14.12)
// ---------------------------------------------------------------------------

// Default build only, matching the committed-golden precedent in
// `tests/p6_hardening.rs`: the C1 sample was derived from the syntactic
// goldens, while the `semantic-clang` tier legitimately changes resolver
// tiers, ARCH-003 confidence and cost topology. The semantic tier is covered
// by the state-matrix row and the semantic-feature test run.
#[cfg(not(feature = "semantic-clang"))]
#[test]
fn agent_report_sample_matches_committed_contract() {
    let fixture = manifest("tests/fixtures/e2e-mixed");
    let out = out_dir("sample");
    assert_eq!(run_scan(&fixture, &out), 0);

    // `root_path` is verbatim CLI provenance by design (§12): the committed
    // goldens were written with the fixture root masked to `<ROOT>`, so the
    // parity assertion normalizes exactly that one token — nothing else.
    let root_token = fixture.to_string_lossy().into_owned();
    // JSON escapes path separators (Windows), so the machine contract needs its
    // own escaped form. Both tokens are asserted to occur exactly once, so the
    // normalization can never hide a second (leaked) absolute path.
    let json_token = serde_json::to_string(&root_token).expect("json-encode root path");
    let json_token = &json_token[1..json_token.len() - 1];
    for (emitted, committed, token) in [
        (
            "agent-report.md",
            "agent-report.sample.md",
            root_token.as_str(),
        ),
        (
            "agent-actions.json",
            "agent-actions.sample.json",
            json_token,
        ),
    ] {
        let raw = read_artifact(&out, emitted);
        assert_eq!(
            raw.matches(token).count(),
            1,
            "{emitted} must carry the scan root exactly once (provenance)"
        );
        let actual = raw.replace(token, "<ROOT>");
        let expected = read_artifact(&manifest("docs/evidence/agent-report/sample"), committed);
        assert_eq!(
            actual, expected,
            "{emitted} drifted from the committed C1 sample ({committed})"
        );
    }

    let doc = read_json(&out);
    assert!(
        doc["scan"].get("started_at").is_none() && doc["scan"].get("duration_ms").is_none(),
        "the machine contract never carries clock provenance"
    );
}

// ---------------------------------------------------------------------------
// 9. Size budget on the committed goldens (§12, §14.6)
// ---------------------------------------------------------------------------

#[test]
fn agent_report_size_budget_golden_fixtures() {
    let budget = agent_report::MD_SIZE_BUDGET;
    for (tag, fixture) in [
        ("report", "tests/fixtures/e2e"),
        ("report-mixed", "tests/fixtures/e2e-mixed"),
        ("report-python", "examples-python"),
        ("report-rust", "examples-rust"),
    ] {
        let doc = assemble_doc(&manifest(fixture));
        let (_, markdown) = render_pair(&doc);
        assert!(
            markdown.len() <= agent_report::MD_SIZE_BUDGET,
            "[{tag}] agent-report.md is {} B (budget {budget} B)",
            markdown.len()
        );
    }

    // Optional manual check (spec §12): the three external scans, never part
    // of the required gate — a fresh checkout has no `~/Opencode/perfpal-test`.
    if std::env::var_os("PERFPAL_EXTERNAL_SCANS").is_none() {
        return;
    }
    let Ok(home) = std::env::var("HOME") else {
        return;
    };
    for repo in ["cJSON", "hyperfine", "bat"] {
        let path = Path::new(&home).join("Opencode/perfpal-test").join(repo);
        if !path.is_dir() {
            continue;
        }
        let doc = assemble_doc(&path);
        let (_, markdown) = render_pair(&doc);
        assert!(
            markdown.len() <= agent_report::MD_SIZE_BUDGET,
            "[{repo}] agent-report.md is {} B (budget {budget} B)",
            markdown.len()
        );
    }
}
