//! H3 style + contract locks: heading hierarchy, the shared warm-paper shell,
//! hostile-input escaping, empty states, and Markdown table/encoding
//! conventions. Driven by a REAL pipeline run on `tests/fixtures/e2e`.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::analyzer::{Confidence, Severity};
use perfpal::cli::run_with;
use perfpal::ir::{IncludeCounts, LocCounts, ParseStatus, ResolverTier, Span};
use perfpal::reporter::agent_report::{render_actions_json, render_markdown};
use perfpal::reporter::report_script;
use perfpal::reporter::{
    render_findings_detail_html, render_html, render_profile_queue, render_profile_queue_html,
    render_report,
};
use perfpal::schema::{
    FileEntry, FindingEntry, FunctionStats, MetricsDocument, ParseHealthEntry, ScanInfo,
    StatsSection, ToolInfo,
};

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
        "perfpal-style-{tag}-{}-{nanos}",
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

// ---------------------------------------------------------------------------
// Heading helpers
// ---------------------------------------------------------------------------

/// `(level, line)` for every Markdown ATX heading.
fn md_headings(md: &str) -> Vec<(usize, &str)> {
    md.lines()
        .filter_map(|line| {
            let level = line.chars().take_while(|c| *c == '#').count();
            if level == 0 || level > 6 {
                return None;
            }
            let rest = &line[level..];
            if !rest.starts_with(' ') {
                return None;
            }
            Some((level, line))
        })
        .collect()
}

fn h2_lines(md: &str) -> Vec<&str> {
    md_headings(md)
        .into_iter()
        .filter(|(level, _)| *level == 2)
        .map(|(_, line)| line)
        .collect()
}

fn assert_no_level_skips(headings: &[(usize, &str)], artifact: &str) {
    assert!(!headings.is_empty(), "{artifact}: no headings");
    assert_eq!(headings[0].0, 1, "{artifact}: must start with an H1");
    for pair in headings.windows(2) {
        assert!(
            pair[1].0 <= pair[0].0 + 1,
            "{artifact}: heading level skip {} → {} ({})",
            pair[0].0,
            pair[1].0,
            pair[1].1
        );
    }
}

/// Heading levels in an HTML page, in document order.
fn html_heading_levels(html: &str) -> Vec<usize> {
    let mut levels = Vec::new();
    for n in 1..=4 {
        let pat = format!("<h{n}");
        let mut from = 0;
        while let Some(i) = html[from..].find(&pat) {
            levels.push((from + i, n));
            from += i + pat.len();
        }
    }
    levels.sort_unstable();
    levels.into_iter().map(|(_, n)| n).collect()
}

fn assert_html_hierarchy(html: &str, name: &str) {
    let levels = html_heading_levels(html);
    assert!(!levels.is_empty(), "{name}: no headings");
    assert_eq!(levels[0], 1, "{name}: must start with an H1");
    assert_eq!(
        levels[1], 2,
        "{name}: first heading after H1 must be H2 (found H{})",
        levels[1]
    );
    for pair in levels.windows(2) {
        assert!(
            pair[1] <= pair[0] + 1,
            "{name}: heading level skip H{} → H{}",
            pair[0],
            pair[1]
        );
    }
}

// ---------------------------------------------------------------------------
// 1. Heading order
// ---------------------------------------------------------------------------

#[test]
fn heading_order_report_md() {
    let out = run_pipeline("headings-md");
    let report = fs::read_to_string(out.join("report.md")).expect("report.md");
    let headings = md_headings(&report);
    assert_no_level_skips(&headings, "report.md");
    assert_eq!(
        h2_lines(&report),
        [
            "## 1. Executive summary",
            "## 2. Scan provenance & parse health",
            "## 3. Codebase statistics",
            "## 4. Findings",
            "## 5. Explorer",
            "## Appendix A. Heuristic reference",
            "## Appendix B. How to read this report",
        ]
    );
    assert!(
        !report.contains("## Appendix C."),
        "Appendix C must be gone"
    );
    assert!(
        !headings.iter().any(|(_, line)| line.starts_with("#### ")),
        "report.md must not carry per-finding detail rows"
    );
}

#[test]
fn heading_order_profile_and_agent_markdown() {
    let out = run_pipeline("headings-other");
    let profile = fs::read_to_string(out.join("profile-queue.md")).expect("profile md");
    let profile_heads = md_headings(&profile);
    assert_no_level_skips(&profile_heads, "profile-queue.md");
    assert_eq!(profile_heads[0].1, "# Perfpal Profile Queue");
    assert_eq!(h2_lines(&profile), ["## Complete cost-ranked list"]);

    let agent = fs::read_to_string(out.join("agent-report.md")).expect("agent report");
    let agent_heads = md_headings(&agent);
    assert_no_level_skips(&agent_heads, "agent-report.md");
    assert_eq!(agent_heads[0].1, "# Perfpal Agent Report");
    assert_eq!(
        h2_lines(&agent),
        [
            "## How to use this report",
            "## Data quality",
            "## Start here — ranked actions",
            "## Action index by rule",
            "## Profile queue",
            "## What this scan cannot see",
        ]
    );
}

#[test]
fn heading_order_html_pages() {
    let out = run_pipeline("headings-html");
    for name in ["report.html", "findings-detail.html", "profile-queue.html"] {
        let html = fs::read_to_string(out.join(name)).expect("page reads");
        assert_html_hierarchy(&html, name);
    }
}

// ---------------------------------------------------------------------------
// 2. Shared warm-paper shell
// ---------------------------------------------------------------------------

const PALETTE: [&str; 3] = ["#f4f1ea", "#202b35", "#7c2d12"];

fn head_html(html: &str) -> &str {
    let end = html.find("</head>").expect("head close");
    &html[..end]
}

fn head_css(html: &str) -> &str {
    let head = head_html(html);
    let start = head.find("<style>").expect("head style") + "<style>".len();
    let end = head[start..].find("</style>").expect("style close") + start;
    &head[start..end]
}

/// Selectors allowed to use monospace: code/pre plus path/label classes.
fn monospace_selector_allowed(selector: &str) -> bool {
    [
        "code",
        "pre",
        "kbd",
        "samp",
        ".mono",
        ".kv-loc",
        ".kv-fn",
        ".kv-anchor",
        ".kv-value",
    ]
    .iter()
    .any(|token| selector.contains(token))
}

#[test]
fn shared_warm_paper_shell_and_no_external_assets() {
    let out = run_pipeline("shell");
    for name in ["report.html", "findings-detail.html", "profile-queue.html"] {
        let html = fs::read_to_string(out.join(name)).expect("page reads");
        for color in PALETTE {
            assert!(html.contains(color), "{name}: missing palette {color}");
        }
        assert!(
            html.contains("font-family:ui-sans-serif,system-ui"),
            "{name}: missing sans body stack"
        );
        assert!(
            !html.contains("rel=\"stylesheet\""),
            "{name}: external stylesheet found"
        );
        let head = head_html(&html);
        assert_eq!(
            head.matches("<style").count(),
            1,
            "{name}: exactly one head <style>"
        );
        // Monospace is reserved for code/pre/path classes.
        for rule in head_css(&html).split('}') {
            let Some((selector, decls)) = rule.split_once('{') else {
                continue;
            };
            if decls.contains("font-family") && decls.contains("monospace") {
                assert!(
                    monospace_selector_allowed(selector),
                    "{name}: monospace leaked into selector {selector:?}"
                );
            }
        }
    }

    // Script policy: report keeps the A12 three; detail + profile-queue keep
    // exactly one fixed enhancement block (filterable contract); drill pages
    // are script-free. No page may load an external script.
    let report_html = fs::read_to_string(out.join("report.html")).expect("report.html");
    assert_eq!(report_html.matches("<script").count(), 3);
    assert_eq!(report_html.matches("</script>").count(), 3);
    for name in ["findings-detail.html", "profile-queue.html"] {
        let html = fs::read_to_string(out.join(name)).expect("page reads");
        let blocks = script_blocks(&html);
        assert_eq!(blocks.len(), 1, "{name}: one fixed script block");
        assert_eq!(
            blocks[0].1.trim_matches('\n'),
            report_script().trim_matches('\n'),
            "{name}: script body must be the fixed REPORT_SCRIPT"
        );
        assert!(
            !blocks[0].0.contains("src="),
            "{name}: script must be inline"
        );
    }
    let detail_html = fs::read_to_string(out.join("findings-detail.html")).expect("detail");
    for id in ["id=\"filter-bar\"", "id=\"filter-clear\""] {
        assert!(
            detail_html.contains(id),
            "findings-detail.html: missing {id} (filterable contract)"
        );
    }
    for name in ["atlas-weight.html", "atlas-q2.html"] {
        let html = fs::read_to_string(out.join(name)).expect("drill page");
        assert_eq!(html.matches("<script").count(), 0, "{name}: script-free");
    }
    for name in [
        "report.html",
        "findings-detail.html",
        "profile-queue.html",
        "atlas-weight.html",
        "atlas-q2.html",
    ] {
        let html = fs::read_to_string(out.join(name)).expect("page");
        assert!(
            !html.contains("<script src=") && !html.contains("<script src=\""),
            "{name}: external/script-src found"
        );
        let lower = html.to_ascii_lowercase();
        assert!(
            !lower.contains("src=\"http://") && !lower.contains("src=\"https://"),
            "{name}: remote script/image found"
        );
    }
}

/// `(opening_tag, body)` for each inline script element.
fn script_blocks(html: &str) -> Vec<(&str, &str)> {
    let mut blocks = Vec::new();
    let mut rest = html;
    while let Some(i) = rest.find("<script") {
        let tail = &rest[i..];
        let tag_end = tail.find('>').expect("script tag close") + 1;
        let body_start = i + tag_end;
        let body_end = html[body_start..]
            .find("</script>")
            .expect("script body close")
            + body_start;
        blocks.push((&html[i..body_start], &html[body_start..body_end]));
        rest = &html[body_end + "</script>".len()..];
    }
    blocks
}

// ---------------------------------------------------------------------------
// 3. Hostile-input escaping
// ---------------------------------------------------------------------------

fn loc(code: u32) -> LocCounts {
    LocCounts {
        code,
        comment: code / 10,
        blank: code / 20,
    }
}

fn ok_health() -> ParseHealthEntry {
    ParseHealthEntry {
        status: ParseStatus::Ok,
        error_regions: None,
        error_node_count: 0,
        resolver: None,
    }
}

fn file_entry(path: &str) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        language: "c".to_string(),
        loc: loc(10),
        function_count: 1,
        global_var_count: 0,
        include_counts: IncludeCounts {
            quoted: 0,
            angle: 0,
        },
        parse_health: ok_health(),
        path_class: None,
    }
}

fn function_stats(path: &str, name: &str) -> FunctionStats {
    FunctionStats {
        id: format!("{path}#{name}#1:1"),
        name: name.to_string(),
        path: path.to_string(),
        span: Span::point(1, 1),
        param_count: 0,
        cyclomatic_complexity: 1,
        cognitive_complexity: 0,
        max_loop_depth: 0,
        alloc_site_count: 0,
        length_lines: 1,
        call_fanout: None,
        call_fanin: None,
        call_depth: None,
        is_recursive: None,
        idiom_hash: None,
        error_path: None,
        alloc_family: None,
        cfg_gated: None,
        entry_kind: None,
        entry_confidence: None,
        reachable_from_entry: None,
        function_cost_v2: None,
        unknown_trip_loops: 0,
        trip_count_source: None,
        known_trip_value: None,
    }
}

const HOSTILE: &str = "evil\n# INJECT `|` <script>alert(1)</script>.c";
const HOSTILE_TITLE: &str = "Title\n# HEADING `|` <script>alert(1)</script>";
const HOSTILE_MSG: &str = "Message\n## BOOM | <script>alert(1)</script>";
const HOSTILE_SYMBOL: &str = "sym\n`| <script>alert(1)</script>";

fn hostile_doc() -> MetricsDocument {
    // A registered rule id is required (Appendix A looks the id up in the
    // frozen catalog); the hostile bytes ride in path/title/message/symbol.
    let rule_id = "PERF-C-ALLOC-001";
    let path = HOSTILE.to_string();
    MetricsDocument {
        schema_version: "1.8".to_string(),
        tool: ToolInfo {
            name: "perfpal".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        scan: ScanInfo {
            root_path: ".".to_string(),
            started_at: "2026-08-24T00:00:00Z".to_string(),
            duration_ms: Some(1),
            languages: vec!["c".to_string()],
            file_count: 1,
            loc_total: Some(loc(10)),
        },
        parse_health_summary: None,
        files: vec![file_entry(&path)],
        stats: StatsSection {
            functions: vec![function_stats(&path, HOSTILE_SYMBOL)],
            distributions: None,
        },
        findings: vec![FindingEntry {
            finding_id: format!("{rule_id}:{path}:1:1"),
            rule_id: rule_id.to_string(),
            title: HOSTILE_TITLE.to_string(),
            path,
            symbol: Some(HOSTILE_SYMBOL.to_string()),
            span: Span::point(1, 1),
            confidence: Confidence::High,
            severity: Severity::Hot,
            resolver_tier: ResolverTier::Syntactic,
            macro_suspect: true,
            error_node_overlap: true,
            threshold: None,
            message: HOSTILE_MSG.to_string(),
            evidence: {
                let mut m = BTreeMap::new();
                m.insert("key\n#".to_string(), "val|ue".to_string());
                m
            },
        }],
        graphs: None,
        components: None,
    }
}

/// Count Markdown table cells, ignoring `\|` escapes.
fn table_cells(line: &str) -> usize {
    let mut cells: usize = 0;
    let mut escaped = false;
    for c in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '|' => cells += 1,
            _ => {}
        }
    }
    cells.saturating_sub(1)
}

#[test]
fn hostile_input_is_neutralized_everywhere() {
    // Direct escaper checks (rule_id cannot be hostile in a full document:
    // Appendix A resolves the frozen catalog and panics on an unknown id).
    let rule = "PERF-C-X\n#|<script>/`";
    let escaped_rule = perfpal::reporter::escape_md(rule);
    assert!(!escaped_rule.contains('\n'));
    assert!(!escaped_rule.contains("<script>"));
    assert!(escaped_rule.contains("\\#"));
    assert!(escaped_rule.contains("\\|"));
    assert!(perfpal::reporter::escape_html(rule).contains("&lt;script&gt;"));

    let doc = hostile_doc();
    let report = render_report(&doc, &[]);
    assert!(!report.contains("<script>"), "report.md leaked raw script");
    assert!(!report.contains("\n# INJECT"), "report.md line injection");
    assert!(
        !report.contains("\n# HEADING"),
        "report.md heading injection"
    );
    assert!(report.contains("\\n"), "report.md must flatten newlines");
    assert!(
        !md_headings(&report)
            .iter()
            .any(|(_, l)| l.contains("INJECT"))
    );
    assert_eq!(report.matches("## Appendix C.").count(), 0);

    let profile = render_profile_queue(&doc);
    assert!(
        !profile.contains("<script>"),
        "profile-queue.md leaked raw script"
    );
    assert!(
        !profile.contains("\n## BOOM"),
        "profile-queue.md line injection"
    );

    let agent = render_markdown(&doc);
    assert!(
        !agent.contains("<script>"),
        "agent-report.md leaked raw script"
    );

    let report_html = render_html(&doc, &[]);
    let detail = render_findings_detail_html(&doc);
    let profile_html = render_profile_queue_html(&doc);
    for (name, html) in [
        ("report.html", &report_html),
        ("findings-detail.html", &detail),
        ("profile-queue.html", &profile_html),
    ] {
        assert!(
            html.contains("&lt;script&gt;"),
            "{name}: hostile script must be HTML-escaped"
        );
        assert!(
            !html.contains("<script>alert(1)"),
            "{name}: executable script injection"
        );
    }
    // The hostile finding row keeps its 11 columns.
    let row_start = detail.find("<tr id=\"finding-").expect("finding row");
    let row = &detail[row_start..];
    let row = &row[..row.find("</tr>").expect("row close")];
    assert_eq!(row.matches("<td").count(), 11);

    // Metrics JSON keeps the original bytes (values round-trip exactly).
    let json = serde_json::to_string(&doc).expect("doc serializes");
    let value: serde_json::Value = serde_json::from_str(&json).expect("json parses");
    assert_eq!(value["findings"][0]["path"], HOSTILE);
    assert_eq!(value["findings"][0]["title"], HOSTILE_TITLE);
    assert_eq!(value["findings"][0]["message"], HOSTILE_MSG);
    assert_eq!(value["findings"][0]["symbol"], HOSTILE_SYMBOL);
    assert!(json.contains("<script>alert(1)</script>"));

    // Markdown §4 table stays structurally intact (6 columns).
    let sec4 = report
        .split("## 4. Findings")
        .nth(1)
        .and_then(|rest| rest.split("## 5. Explorer").next())
        .expect("§4 present");
    let table_lines: Vec<&str> = sec4.lines().filter(|l| l.starts_with('|')).collect();
    assert!(table_lines.len() >= 3);
    let expected = table_cells(table_lines[0]);
    assert_eq!(expected, 6);
    for line in &table_lines {
        assert_eq!(
            table_cells(line),
            expected,
            "ragged Markdown table row: {line}"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. Empty states
// ---------------------------------------------------------------------------

fn empty_doc() -> MetricsDocument {
    MetricsDocument {
        schema_version: "1.8".to_string(),
        tool: ToolInfo {
            name: "perfpal".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        scan: ScanInfo {
            root_path: ".".to_string(),
            started_at: "2026-08-24T00:00:00Z".to_string(),
            duration_ms: Some(1),
            languages: vec!["c".to_string()],
            file_count: 0,
            loc_total: Some(loc(0)),
        },
        parse_health_summary: None,
        files: vec![],
        stats: StatsSection {
            functions: vec![],
            distributions: None,
        },
        findings: vec![],
        graphs: None,
        components: None,
    }
}

#[test]
fn empty_states_are_named_stubs() {
    let doc = empty_doc();
    let report = render_report(&doc, &[]);
    assert!(report.contains("No findings."));
    assert!(
        !report.contains("findings-detail.html"),
        "empty report must not point at detail"
    );

    let report_html = render_html(&doc, &[]);
    assert!(report_html.contains("No findings."));
    let detail = render_findings_detail_html(&doc);
    assert!(detail.contains("No findings."));

    let profile_md = render_profile_queue(&doc);
    assert!(profile_md.contains("No functions recorded."));
    assert!(profile_md.contains("# Perfpal Profile Queue"));
    let profile_html = render_profile_queue_html(&doc);
    assert!(profile_html.contains("No functions recorded."));

    let agent = render_markdown(&doc);
    use perfpal::reporter::agent_report::{STUB_NO_FINDINGS, STUB_NO_FUNCTIONS, STUB_NO_RULES};
    assert!(agent.contains(STUB_NO_FINDINGS));
    assert!(agent.contains(STUB_NO_FUNCTIONS));
    assert!(agent.contains(STUB_NO_RULES));
    let actions: serde_json::Value =
        serde_json::from_str(&render_actions_json(&doc)).expect("actions json");
    assert_eq!(actions["complete_action_count"], 0);
}

// ---------------------------------------------------------------------------
// 5. Markdown table + encoding conventions
// ---------------------------------------------------------------------------

#[test]
fn tables_are_rectangular_and_all_text_is_lf_with_trailing_newline() {
    let out = run_pipeline("tables");
    let mut names: Vec<String> = fs::read_dir(&out)
        .unwrap()
        .flatten()
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.extend(
        fs::read_dir(out.join("charts"))
            .unwrap()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .map(|n| format!("charts/{n}")),
    );
    names.sort();

    for name in &names {
        if !matches!(
            name.rsplit('.').next(),
            Some("md" | "html" | "json" | "dot" | "svg")
        ) {
            continue;
        }
        let bytes = fs::read(out.join(name)).expect("artifact reads");
        let text = std::str::from_utf8(&bytes).expect("text artifact is UTF-8");
        assert!(bytes.ends_with(b"\n"), "{name}: missing trailing newline");
        assert!(!text.contains('\r'), "{name}: CR found (must be LF-only)");
        assert!(!text.contains('\u{0}'), "{name}: NUL found");
        if name.ends_with(".md") {
            let lines: Vec<&str> = text.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if !line.starts_with('|') {
                    continue;
                }
                // Separator row: all cells are `---`-style.
                let separator = line
                    .split('|')
                    .filter(|c| !c.trim().is_empty())
                    .all(|c| c.trim().trim_matches(':').chars().all(|ch| ch == '-'));
                if !separator || i == 0 {
                    continue;
                }
                let header = lines[i - 1];
                assert!(header.starts_with('|'), "{name}: separator without header");
                let cols = table_cells(header);
                assert_eq!(
                    table_cells(line),
                    cols,
                    "{name}:{} separator/header column mismatch",
                    i + 1
                );
                for row in lines.iter().skip(i + 1) {
                    if !row.starts_with('|') {
                        break;
                    }
                    assert_eq!(
                        table_cells(row),
                        cols,
                        "{name}: ragged table row at {}",
                        i + 1
                    );
                }
            }
        }
    }
}
