//! Shared synthetic documents for the Atlas integration locks (A8/A9/A10/A12).
//!
//! Mirrors the in-crate `atlas::test_support` state matrix without exporting
//! test-only internals across the crate boundary.

use std::collections::BTreeMap;

use perfpal::analyzer::{Confidence, Severity};
use perfpal::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverTier, Span};
use perfpal::schema::{
    CallEdge, CallGraphSection, FileEntry, FindingEntry, FunctionStats, GraphsSection,
    IncludeEdgeDto, IncludeGraph, MetricsDocument, ParseHealthEntry, ScanInfo, StatsSection,
    ToolInfo,
};

fn health(status: ParseStatus) -> ParseHealthEntry {
    ParseHealthEntry {
        status,
        error_regions: None,
        error_node_count: 0,
        resolver: None,
    }
}

fn file(path: &str, class: &str, function_count: u32) -> FileEntry {
    file_with_status(path, class, function_count, ParseStatus::Ok)
}

fn file_with_status(
    path: &str,
    class: &str,
    function_count: u32,
    status: ParseStatus,
) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        language: "c".to_string(),
        loc: LocCounts {
            code: 10,
            comment: 0,
            blank: 0,
        },
        function_count,
        global_var_count: 0,
        include_counts: IncludeCounts {
            quoted: 0,
            angle: 0,
        },
        parse_health: health(status),
        path_class: Some(class.to_string()),
    }
}

fn function(
    path: &str,
    name: &str,
    line: u32,
    cost: Option<u64>,
    fan_in: u32,
    entry: Option<&str>,
) -> FunctionStats {
    let entry_confidence = match entry {
        Some("main" | "script") => Some("HIGH".to_string()),
        Some("lib-root" | "handler") => Some("MEDIUM".to_string()),
        Some("unknown-entry") => Some("LOW".to_string()),
        _ => None,
    };
    FunctionStats {
        id: format!("{path}#{name}#{line}:1"),
        name: name.to_string(),
        path: path.to_string(),
        span: Span {
            start_line: line,
            start_col: 1,
            end_line: line + 8,
            end_col: 1,
        },
        param_count: 0,
        cyclomatic_complexity: 1,
        cognitive_complexity: 0,
        max_loop_depth: 1,
        alloc_site_count: 1,
        length_lines: 9,
        call_fanout: Some(0),
        call_fanin: Some(fan_in),
        call_depth: Some(0),
        is_recursive: Some(false),
        idiom_hash: None,
        error_path: Some(false),
        alloc_family: None,
        cfg_gated: None,
        entry_kind: entry.map(str::to_string),
        entry_confidence,
        reachable_from_entry: None,
        function_cost_v2: cost,
        unknown_trip_loops: 0,
        trip_count_source: None,
        known_trip_value: None,
    }
}

fn finding(rule_id: &str, path: &str, line: u32, severity: Severity, title: &str) -> FindingEntry {
    FindingEntry {
        finding_id: format!("{rule_id}:{path}:{line}:1"),
        rule_id: rule_id.to_string(),
        title: title.to_string(),
        path: path.to_string(),
        symbol: None,
        span: Span::point(line, 1),
        confidence: Confidence::High,
        severity,
        resolver_tier: ResolverTier::Syntactic,
        macro_suspect: false,
        error_node_overlap: false,
        threshold: None,
        message: format!("Rationale for {rule_id} at {path}:{line}."),
        evidence: BTreeMap::new(),
    }
}

fn call_edge(
    from_function: &str,
    to_name: &str,
    from_path: &str,
    to_path: Option<&str>,
    confidence: Confidence,
) -> CallEdge {
    CallEdge::with_confidence(
        from_function,
        to_name,
        from_path,
        to_path.map(str::to_string),
        confidence,
    )
}

fn include_edge(from: &str, to: Option<&str>, resolved: bool) -> IncludeEdgeDto {
    IncludeEdgeDto {
        from: from.to_string(),
        to: to.map(str::to_string),
        form: perfpal::ir::IncludeForm::Quoted,
        resolved,
        span: Span::point(1, 1),
    }
}

fn document(
    files: Vec<FileEntry>,
    functions: Vec<FunctionStats>,
    findings: Vec<FindingEntry>,
    call_edges: Vec<CallEdge>,
    include_edges: Vec<IncludeEdgeDto>,
) -> MetricsDocument {
    MetricsDocument {
        schema_version: "1.7".to_string(),
        tool: ToolInfo {
            name: "perfpal".to_string(),
            version: "test".to_string(),
        },
        scan: ScanInfo {
            root_path: ".".to_string(),
            started_at: "2026-10-23T00:00:00Z".to_string(),
            duration_ms: None,
            languages: vec!["c".to_string()],
            file_count: files.len() as u32,
            loc_total: None,
        },
        parse_health_summary: None,
        files,
        stats: StatsSection {
            functions,
            distributions: None,
        },
        findings,
        graphs: Some(GraphsSection {
            include_graph: IncludeGraph {
                nodes: Vec::new(),
                edges: include_edges,
            },
            call_graph: Some(CallGraphSection { edges: call_edges }),
        }),
        components: None,
    }
}

/// Application: production main, forward hops, reachable/unreached mix.
pub fn application() -> MetricsDocument {
    document(
        vec![
            file("src/main.c", "production", 2),
            file("src/a.c", "production", 1),
            file("src/b.c", "production", 1),
            file("tests/t.c", "test", 1),
        ],
        vec![
            function("src/main.c", "main", 1, Some(500), 0, Some("main")),
            function("src/main.c", "init", 20, Some(50), 1, None),
            function("src/a.c", "work", 1, Some(300), 1, None),
            function("src/b.c", "leaf", 1, Some(100), 1, None),
            function("tests/t.c", "test_work", 1, Some(900), 0, None),
        ],
        vec![
            finding(
                "PERF-C-LOOP-001",
                "src/a.c",
                3,
                Severity::Hot,
                "Loop with a large trip count",
            ),
            finding(
                "PERF-C-ALLOC-001",
                "src/main.c",
                4,
                Severity::Warn,
                "Allocation inside a loop",
            ),
        ],
        vec![
            call_edge(
                "src/main.c#main#1:1",
                "work",
                "src/main.c",
                Some("src/a.c"),
                Confidence::Low,
            ),
            call_edge(
                "src/a.c#work#1:1",
                "leaf",
                "src/a.c",
                Some("src/b.c"),
                Confidence::Low,
            ),
        ],
        vec![
            include_edge("src/a.c", Some("src/main.c"), true),
            include_edge("tests/t.c", Some("src/main.c"), true),
        ],
    )
}

/// Library: highest-fan-in production lib-root with reverse callers.
pub fn library() -> MetricsDocument {
    document(
        vec![
            file("src/lib.c", "production", 1),
            file("src/caller1.c", "production", 1),
            file("src/caller2.c", "production", 1),
        ],
        vec![
            function("src/lib.c", "lib_root", 1, Some(100), 10, Some("lib-root")),
            function("src/caller1.c", "caller_one", 1, Some(200), 0, None),
            function("src/caller2.c", "caller_two", 1, Some(50), 0, None),
        ],
        vec![finding(
            "PERF-C-STR-001",
            "src/lib.c",
            3,
            Severity::Warn,
            "String copy in a loop",
        )],
        vec![
            call_edge(
                "src/caller1.c#caller_one#1:1",
                "lib_root",
                "src/caller1.c",
                Some("src/lib.c"),
                Confidence::Low,
            ),
            call_edge(
                "src/caller2.c#caller_two#1:1",
                "lib_root",
                "src/caller2.c",
                Some("src/lib.c"),
                Confidence::Low,
            ),
        ],
        vec![include_edge("src/caller1.c", Some("src/lib.c"), true)],
    )
}

/// Production handler only: no electable entry.
pub fn no_entry() -> MetricsDocument {
    document(
        vec![file("src/handler.c", "production", 1)],
        vec![function(
            "src/handler.c",
            "handle",
            1,
            Some(100),
            0,
            Some("handler"),
        )],
        vec![finding(
            "PERF-C-LOOP-001",
            "src/handler.c",
            3,
            Severity::Info,
            "Nested loop",
        )],
        Vec::new(),
        Vec::new(),
    )
}

/// Empty scan: no files at all.
pub fn empty() -> MetricsDocument {
    document(Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new())
}

/// Test-only corpus (no production source files).
pub fn all_test() -> MetricsDocument {
    document(
        vec![file("tests/test_main.c", "test", 1)],
        vec![function(
            "tests/test_main.c",
            "test_main",
            1,
            Some(100),
            0,
            Some("main"),
        )],
        vec![finding(
            "PERF-C-ALLOC-001",
            "tests/test_main.c",
            3,
            Severity::Info,
            "Allocation in a test",
        )],
        Vec::new(),
        Vec::new(),
    )
}

/// 26 reachable files with one depth-2 file: the node cap bites.
pub fn capped() -> MetricsDocument {
    let mut files = vec![file("src/entry.c", "production", 1)];
    let mut functions = vec![function(
        "src/entry.c",
        "main",
        1,
        Some(10),
        0,
        Some("main"),
    )];
    let mut edges = Vec::new();
    for index in 0..24_u64 {
        let path = format!("src/f{index}.c");
        files.push(file(&path, "production", 1));
        functions.push(function(
            &path,
            &format!("f{index}"),
            1,
            Some(1000 - index),
            0,
            None,
        ));
        edges.push(call_edge(
            "src/entry.c#main#1:1",
            &format!("f{index}"),
            "src/entry.c",
            Some(&path),
            Confidence::Low,
        ));
    }
    files.push(file("src/deep.c", "production", 1));
    functions.push(function("src/deep.c", "deep", 1, Some(5), 0, None));
    edges.push(call_edge(
        "src/f0.c#f0#1:1",
        "deep",
        "src/f0.c",
        Some("src/deep.c"),
        Confidence::Low,
    ));
    document(
        files,
        functions,
        vec![finding(
            "PERF-C-LOOP-001",
            "src/f0.c",
            3,
            Severity::Hot,
            "Loop with a large trip count",
        )],
        edges,
        Vec::new(),
    )
}

/// Application shape with one partially parsed source file.
pub fn partial_parse() -> MetricsDocument {
    document(
        vec![
            file("src/main.c", "production", 2),
            file_with_status("src/broken.c", "production", 1, ParseStatus::Partial),
        ],
        vec![
            function("src/main.c", "main", 1, Some(100), 0, Some("main")),
            function("src/main.c", "helper", 20, Some(20), 1, None),
            function("src/broken.c", "broken", 1, Some(50), 0, None),
        ],
        vec![finding(
            "PERF-C-LOOP-001",
            "src/broken.c",
            3,
            Severity::Warn,
            "Loop in a partially parsed file",
        )],
        vec![call_edge(
            "src/main.c#main#1:1",
            "broken",
            "src/main.c",
            Some("src/broken.c"),
            Confidence::Low,
        )],
        Vec::new(),
    )
}

/// One production file with a main function and no edges.
pub fn single_file() -> MetricsDocument {
    document(
        vec![file("src/solo.c", "production", 1)],
        vec![function("src/solo.c", "main", 1, Some(42), 0, Some("main"))],
        vec![finding(
            "PERF-C-ALLOC-001",
            "src/solo.c",
            3,
            Severity::Hot,
            "Allocation in main",
        )],
        Vec::new(),
        Vec::new(),
    )
}

/// Every rail role in one page: entry, production, test, bench, unreached.
/// Only `atlas_rail` consumes this fixture; the other test crates include this
/// module wholesale.
#[allow(dead_code)]
pub fn roles_demo() -> MetricsDocument {
    document(
        vec![
            file("src/main.c", "production", 1),
            file("src/util.c", "production", 1),
            file("tests/t.c", "test", 1),
            file("benches/b.c", "bench", 1),
            file("src/dead.c", "production", 1),
        ],
        vec![
            function("src/main.c", "main", 1, Some(100), 0, Some("main")),
            function("src/util.c", "util", 1, Some(40), 1, None),
            function("tests/t.c", "t_helper", 1, Some(30), 1, None),
            function("benches/b.c", "b_helper", 1, Some(20), 1, None),
            function("src/dead.c", "dead", 1, Some(10), 0, None),
        ],
        Vec::new(),
        vec![
            call_edge(
                "src/main.c#main#1:1",
                "util",
                "src/main.c",
                Some("src/util.c"),
                Confidence::Low,
            ),
            call_edge(
                "src/main.c#main#1:1",
                "t_helper",
                "src/main.c",
                Some("tests/t.c"),
                Confidence::Low,
            ),
            call_edge(
                "src/main.c#main#1:1",
                "b_helper",
                "src/main.c",
                Some("benches/b.c"),
                Confidence::Low,
            ),
        ],
        Vec::new(),
    )
}

/// The eight-state A10 matrix.
pub fn states() -> Vec<(&'static str, MetricsDocument)> {
    vec![
        ("application", application()),
        ("library", library()),
        ("no-entry", no_entry()),
        ("empty", empty()),
        ("all-test", all_test()),
        ("capped", capped()),
        ("partial-parse", partial_parse()),
        ("single-file", single_file()),
    ]
}
