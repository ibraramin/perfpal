//! H3 artifact-consolidation matrix: one row per emitted artifact, one owner
//! per (uniqueness class, format), explicit cap disclosure, and link
//! resolution across the whole report directory.
//!
//! Runs the REAL pipeline on `tests/fixtures/e2e` into a temp dir, then
//! asserts the static matrix below matches the emitted set exactly.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;

// ---------------------------------------------------------------------------
// Static matrix
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Exact relative filename.
    Exact,
    /// `charts/*.svg` overview / map chart (not a per-component page).
    OverviewChart,
    /// `charts/component-*.svg` per-component detail page.
    ComponentChart,
}

#[derive(Clone, Copy)]
struct Row {
    file: &'static str,
    format: &'static str,
    audience: &'static str,
    consumer: &'static str,
    action: &'static str,
    role: &'static str,
    uniqueness: &'static str,
    canonical: bool,
    opt_in: bool,
    kind: Kind,
}

const MATRIX: &[Row] = &[
    Row {
        file: "metrics.json",
        format: "json",
        audience: "machine",
        consumer: "downstream tools / LLM planners",
        action: "validate, diff, or feed downstream analysis",
        opt_in: false,
        role: "canonical metrics and IR (source of truth)",
        uniqueness: "canonical-metrics",
        canonical: true,
        kind: Kind::Exact,
    },
    Row {
        file: "metrics.schema.v1.8.json",
        format: "json",
        audience: "tooling",
        consumer: "JSON-schema validators",
        action: "validate metrics.json against the bundled schema",
        opt_in: false,
        role: "bundled schema mirror",
        uniqueness: "schema-mirror",
        canonical: true,
        kind: Kind::Exact,
    },
    Row {
        file: "report.md",
        format: "md",
        audience: "human",
        consumer: "terminal / Markdown viewer",
        action: "read the ranked summary in a terminal/editor",
        opt_in: false,
        role: "bounded narrative report and rankings",
        uniqueness: "human-report",
        canonical: false,
        kind: Kind::Exact,
    },
    Row {
        file: "report.html",
        format: "html",
        audience: "human",
        consumer: "browser",
        action: "open the visual Atlas map and story in a browser",
        opt_in: false,
        role: "Atlas map, story, and bounded findings",
        uniqueness: "human-report-html",
        canonical: false,
        kind: Kind::Exact,
    },
    Row {
        file: "findings-detail.html",
        format: "html",
        audience: "human",
        consumer: "browser drill-down",
        action: "inspect every finding row and its anchors",
        opt_in: false,
        role: "COMPLETE per-finding detail (canonical human detail)",
        uniqueness: "complete-findings",
        canonical: true,
        kind: Kind::Exact,
    },
    Row {
        file: "profile-queue.md",
        format: "md",
        audience: "human",
        consumer: "terminal / Markdown viewer",
        action: "read the complete cost-ranked queue",
        opt_in: false,
        role: "complete cost-ranked profile queue",
        uniqueness: "complete-profile",
        canonical: false,
        kind: Kind::Exact,
    },
    Row {
        file: "profile-queue.html",
        format: "html",
        audience: "human",
        consumer: "browser",
        action: "browse the complete cost-ranked queue",
        opt_in: false,
        role: "complete cost-ranked profile queue",
        uniqueness: "complete-profile",
        canonical: false,
        kind: Kind::Exact,
    },
    Row {
        file: "atlas-weight.html",
        format: "html",
        audience: "human",
        consumer: "browser",
        action: "drill into per-file weight and cost",
        opt_in: false,
        role: "Atlas weight drill page",
        uniqueness: "atlas-drill-weight",
        canonical: false,
        kind: Kind::Exact,
    },
    Row {
        file: "atlas-q2.html",
        format: "html",
        audience: "human",
        consumer: "browser",
        action: "drill into reverse dependencies (Q2)",
        opt_in: false,
        role: "Atlas reverse (Q2) drill page",
        uniqueness: "atlas-drill-q2",
        canonical: false,
        kind: Kind::Exact,
    },
    Row {
        file: "agent-report.md",
        format: "md",
        audience: "LLM / agent",
        consumer: "LLM triage",
        action: "triage quickly as an LLM or agent",
        opt_in: false,
        role: "bounded agent digest",
        uniqueness: "digest",
        canonical: false,
        kind: Kind::Exact,
    },
    Row {
        file: "agent-actions.json",
        format: "json",
        audience: "LLM / agent",
        consumer: "agent action executor",
        action: "execute or iterate the complete action list",
        opt_in: false,
        role: "COMPLETE machine action list",
        uniqueness: "complete-actions",
        canonical: true,
        kind: Kind::Exact,
    },
    Row {
        file: "atlas-layout.json",
        format: "json",
        audience: "tooling / story consumer",
        consumer: "sim reference player + external story consumers",
        action: "overlay/project the run story",
        role: "story canvas geometry projection",
        uniqueness: "story-geometry",
        canonical: false,
        opt_in: true,
        kind: Kind::Exact,
    },
    Row {
        file: "charts/graph.dot",
        format: "dot",
        audience: "tooling",
        consumer: "Graphviz / graph tools",
        action: "render or analyse the graph externally",
        opt_in: false,
        role: "deterministic graph export",
        uniqueness: "graph-export",
        canonical: false,
        kind: Kind::Exact,
    },
    Row {
        file: "charts/*.svg",
        format: "svg",
        audience: "human",
        consumer: "browser",
        action: "view a chart layer in a browser",
        opt_in: false,
        role: "overview charts and Atlas map layers",
        uniqueness: "chart-overview",
        canonical: false,
        kind: Kind::OverviewChart,
    },
    Row {
        file: "charts/component-*.svg",
        format: "svg",
        audience: "human",
        consumer: "browser",
        action: "drill into one component page",
        opt_in: false,
        role: "per-component drill detail",
        uniqueness: "chart-component",
        canonical: false,
        kind: Kind::ComponentChart,
    },
];

/// Fixed (non-chart) emitted files, H3 set. `charts/graph.dot` is fixed; the
/// SVG charts are covered by the two pattern rows.
const FIXED_FILES: &[&str] = &[
    "metrics.json",
    "metrics.schema.v1.8.json",
    "report.md",
    "report.html",
    "findings-detail.html",
    "profile-queue.md",
    "profile-queue.html",
    "atlas-weight.html",
    "atlas-q2.html",
    "agent-report.md",
    "agent-actions.json",
    "charts/graph.dot",
];

// ---------------------------------------------------------------------------
// Pipeline harness
// ---------------------------------------------------------------------------

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/e2e")
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-matrix-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn run_pipeline(tag: &str) -> PathBuf {
    let root = temp_dir(tag);
    let out = root.join("perfpal-report");
    let argv: Vec<OsString> = [
        OsString::from("perfpal"),
        OsString::from("--output"),
        out.clone().into_os_string(),
        fixture_root().into_os_string(),
    ]
    .into_iter()
    .collect();
    assert_eq!(run_with(argv), 0, "pipeline run must succeed");
    out
}

fn run_pipeline_story(tag: &str) -> PathBuf {
    let root = temp_dir(tag);
    let out = root.join("perfpal-report");
    let argv: Vec<OsString> = [
        OsString::from("perfpal"),
        OsString::from("--story"),
        OsString::from("--output"),
        out.clone().into_os_string(),
        fixture_root().into_os_string(),
    ]
    .into_iter()
    .collect();
    assert_eq!(run_with(argv), 0, "story pipeline run must succeed");
    out
}

fn emitted_files(root: &Path) -> BTreeSet<String> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeSet<String>) {
        for entry in fs::read_dir(dir).expect("artifact directory readable") {
            let path = entry.expect("artifact entry readable").path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(base)
                        .expect("artifact under root")
                        .to_string_lossy()
                        .replace('\\', "/"),
                );
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(root, root, &mut out);
    out
}

fn is_overview_chart(name: &str) -> bool {
    name.starts_with("charts/") && name.ends_with(".svg") && !name.starts_with("charts/component-")
}

fn is_component_chart(name: &str) -> bool {
    name.starts_with("charts/component-") && name.ends_with(".svg")
}

fn exact_rows(file: &str) -> usize {
    MATRIX
        .iter()
        .filter(|r| r.kind == Kind::Exact && r.file == file)
        .count()
}

// ---------------------------------------------------------------------------
// 1. Matrix ↔ emitted set
// ---------------------------------------------------------------------------

#[test]
fn matrix_matches_emitted_artifact_set_exactly() {
    let out = run_pipeline("set");
    let emitted = emitted_files(&out);

    // Every emitted fixed file has exactly one exact matrix row.
    for name in FIXED_FILES {
        assert!(emitted.contains(*name), "fixed artifact {name} not emitted");
        assert_eq!(exact_rows(name), 1, "matrix must row {name} exactly once");
    }
    // No emitted file outside the fixed set except SVG charts.
    for name in &emitted {
        if FIXED_FILES.contains(&name.as_str()) {
            continue;
        }
        assert!(
            is_overview_chart(name) || is_component_chart(name),
            "emitted artifact {name} has no matrix class"
        );
    }
    // Every non-opt-in matrix row is emitted (patterns must match at least
    // one file); opt-in rows are covered by the --story test below.
    for row in MATRIX.iter().filter(|r| !r.opt_in) {
        let present = match row.kind {
            Kind::Exact => emitted.contains(row.file),
            Kind::OverviewChart => emitted.iter().any(|n| is_overview_chart(n)),
            Kind::ComponentChart => emitted.iter().any(|n| is_component_chart(n)),
        };
        assert!(present, "matrix row {} is not emitted", row.file);
    }
    // Default run excludes every opt-in row.
    for row in MATRIX.iter().filter(|r| r.opt_in) {
        assert!(
            !emitted.contains(row.file),
            "opt-in row {} emitted without its flag",
            row.file
        );
    }
    // H3: the Markdown detail artifact is gone and has no matrix row.
    assert!(!emitted.contains("findings-detail.md"));
    assert!(
        MATRIX.iter().all(|r| r.file != "findings-detail.md"),
        "findings-detail.md must not appear in the artifact matrix"
    );
    // Canonical ownership is explicit and complete.
    let canonical: Vec<&str> = MATRIX
        .iter()
        .filter(|r| r.canonical)
        .map(|r| r.file)
        .collect();
    assert_eq!(
        canonical,
        [
            "metrics.json",
            "metrics.schema.v1.8.json",
            "findings-detail.html",
            "agent-actions.json",
        ]
    );

    // Metadata is complete.
    for row in MATRIX {
        assert!(!row.audience.is_empty(), "{} needs an audience", row.file);
        assert!(!row.consumer.is_empty(), "{} needs a consumer", row.file);
        assert!(!row.action.is_empty(), "{} needs an action", row.file);
        assert!(!row.role.is_empty());
        assert!(!row.uniqueness.is_empty());
        assert!(!row.format.is_empty());
    }
}

#[test]
fn story_geometry_is_opt_in_only() {
    let default_out = run_pipeline("no-story");
    let default_set = emitted_files(&default_out);
    assert!(!default_set.contains("atlas-layout.json"));

    let story_out = run_pipeline_story("story");
    let story_set = emitted_files(&story_out);
    assert!(story_set.contains("atlas-layout.json"));

    // --story adds exactly the layout artifact (plus nothing else).
    let mut expected = default_set;
    expected.insert("atlas-layout.json".to_string());
    assert_eq!(
        story_set, expected,
        "--story must add only atlas-layout.json"
    );

    let row = MATRIX
        .iter()
        .find(|r| r.file == "atlas-layout.json")
        .expect("atlas-layout.json matrix row");
    assert!(row.opt_in, "layout row must be opt-in");
    assert_eq!(row.uniqueness, "story-geometry");
    assert!(!row.canonical);
    assert!(!row.action.is_empty());
    assert_eq!(exact_rows("atlas-layout.json"), 1);
}

#[test]
fn invalid_report_width_exits_one() {
    let root = temp_dir("bad-width");
    let out = root.join("perfpal-report");
    let argv: Vec<OsString> = [
        OsString::from("perfpal"),
        OsString::from("--report-width"),
        OsString::from("9999"),
        OsString::from("--output"),
        out.into_os_string(),
        fixture_root().into_os_string(),
    ]
    .into_iter()
    .collect();
    assert_eq!(run_with(argv), 1, "invalid --report-width must exit 1");
}

// ---------------------------------------------------------------------------
// 2. One owner per (uniqueness class, format)
// ---------------------------------------------------------------------------

fn exact_owners(class: &str, format: &str) -> Vec<&'static str> {
    MATRIX
        .iter()
        .filter(|r| r.kind == Kind::Exact && r.uniqueness == class && r.format == format)
        .map(|r| r.file)
        .collect()
}

#[test]
fn one_owner_per_uniqueness_class_and_format() {
    assert_eq!(
        exact_owners("complete-findings", "html"),
        ["findings-detail.html"]
    );
    assert_eq!(
        exact_owners("complete-actions", "json"),
        ["agent-actions.json"]
    );
    assert_eq!(exact_owners("complete-profile", "md"), ["profile-queue.md"]);
    assert_eq!(
        exact_owners("complete-profile", "html"),
        ["profile-queue.html"]
    );
    assert_eq!(exact_owners("canonical-metrics", "json"), ["metrics.json"]);
    assert_eq!(exact_owners("digest", "md"), ["agent-report.md"]);
    assert_eq!(
        exact_owners("schema-mirror", "json"),
        ["metrics.schema.v1.8.json"]
    );
}

// ---------------------------------------------------------------------------
// 3. Cap disclosure: no silent caps
// ---------------------------------------------------------------------------

#[test]
fn caps_are_disclosed_in_every_consumer() {
    let out = run_pipeline("caps");

    let metrics: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("metrics.json")).unwrap())
            .expect("metrics.json parses");
    let finding_count = metrics["findings"].as_array().expect("findings[]").len();

    // report.md §4 always prints the honest `N of M` disclosure.
    let report = fs::read_to_string(out.join("report.md")).unwrap();
    assert!(
        report.contains(&format!(
            "{} of {} total findings",
            finding_count, finding_count
        )),
        "report.md §4 must print the N-of-M disclosure"
    );

    let actions: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("agent-actions.json")).unwrap())
            .expect("agent-actions.json parses");
    let caps = &actions["data_quality"]["caps"];
    assert_eq!(caps["actions_detail"], 25);
    assert_eq!(caps["profile_inline"], 25);
    let action_count = actions["complete_action_count"]
        .as_u64()
        .expect("complete_action_count") as usize;
    let profile_count = actions["complete_profile_count"]
        .as_u64()
        .expect("complete_profile_count") as usize;

    let agent_md = fs::read_to_string(out.join("agent-report.md")).unwrap();
    if action_count > 25 {
        assert!(
            agent_md.contains(&format!(
                "showing the top 25 of {action_count} actions; the complete list is `agent-actions.json`."
            )),
            "agent-report.md must disclose the 25-action cap"
        );
    }
    if profile_count > 25 {
        assert!(
            agent_md.contains(&format!("showing the top 25 of {profile_count} functions")),
            "agent-report.md must disclose the 25-profile cap"
        );
    }
    // Silent caps are forbidden: the machine envelope always names both caps.
    assert!(caps.get("actions_detail").is_some());
    assert!(caps.get("profile_inline").is_some());
}

// ---------------------------------------------------------------------------
// 4. Link resolution + no stale Markdown-detail references
// ---------------------------------------------------------------------------

fn html_ids(html: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for needle in ["id=\"", "<a id=\""] {
        let mut rest = html;
        while let Some(i) = rest.find(needle) {
            let from = i + needle.len();
            if let Some(j) = rest[from..].find('"') {
                ids.insert(rest[from..from + j].to_string());
                rest = &rest[from + j + 1..];
            } else {
                break;
            }
        }
    }
    ids
}

fn md_links(md: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = md;
    while let Some(i) = rest.find("](") {
        let from = i + 2;
        if let Some(j) = rest[from..].find(')') {
            out.push(rest[from..from + j].to_string());
            rest = &rest[from + j + 1..];
        } else {
            break;
        }
    }
    out
}

fn html_refs(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    for needle in ["href=\"", "src=\""] {
        let mut rest = html;
        while let Some(i) = rest.find(needle) {
            let from = i + needle.len();
            if let Some(j) = rest[from..].find('"') {
                out.push(rest[from..from + j].to_string());
                rest = &rest[from + j + 1..];
            } else {
                break;
            }
        }
    }
    out
}

fn target_ids(emitted: &BTreeSet<String>, target: &str, root: &Path) -> BTreeSet<String> {
    assert!(emitted.contains(target), "link target {target} not emitted");
    let text = fs::read_to_string(root.join(target)).expect("target is UTF-8");
    if target.ends_with(".html") {
        html_ids(&text)
    } else {
        let mut ids = BTreeSet::new();
        for line in text.lines() {
            if let Some(i) = line.find("<a id=\"") {
                let from = i + 7;
                if let Some(j) = line[from..].find('"') {
                    ids.insert(line[from..from + j].to_string());
                }
            }
        }
        ids
    }
}

#[test]
fn every_relative_link_resolves_and_no_md_detail_reference_survives() {
    let root = run_pipeline("links");
    let emitted = emitted_files(&root);
    let pages = [
        "report.md",
        "report.html",
        "findings-detail.html",
        "profile-queue.md",
        "profile-queue.html",
        "agent-report.md",
        "atlas-weight.html",
        "atlas-q2.html",
    ];
    for page in pages {
        let text = fs::read_to_string(root.join(page)).expect("page is UTF-8");
        let refs = if page.ends_with(".md") {
            md_links(&text)
        } else {
            html_refs(&text)
        };
        let own_ids = if page.ends_with(".html") {
            html_ids(&text)
        } else {
            BTreeSet::new()
        };
        for reference in refs {
            if reference.starts_with("data:") || reference.starts_with("mailto:") {
                continue;
            }
            if let Some(frag) = reference.strip_prefix('#') {
                if page.ends_with(".html") {
                    assert!(
                        own_ids.contains(frag),
                        "{page}: in-page link {reference} has no id"
                    );
                }
                continue;
            }
            assert!(
                !reference.contains("://"),
                "{page}: external link {reference} is not allowed"
            );
            let (target, fragment) = match reference.split_once('#') {
                Some((t, f)) => (t, Some(f)),
                None => (reference.as_str(), None),
            };
            assert!(!target.is_empty(), "{page}: empty link target");
            if let Some(fragment) = fragment {
                let ids = target_ids(&emitted, target, &root);
                assert!(
                    ids.contains(fragment),
                    "{page}: link {reference} has no matching fragment in {target}"
                );
            } else {
                assert!(
                    emitted.contains(target),
                    "{page}: link target {target} not emitted"
                );
            }
        }
    }

    // No emitted text artifact may mention the removed Markdown detail.
    for name in &emitted {
        let text_ext = [".md", ".html", ".json", ".dot", ".svg"]
            .iter()
            .any(|ext| name.ends_with(ext));
        if !text_ext {
            continue;
        }
        let text = fs::read_to_string(root.join(name)).expect("text artifact");
        assert!(
            !text.contains("findings-detail.md"),
            "{name} still references the removed findings-detail.md"
        );
    }
}
