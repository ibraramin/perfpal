//! `reporter` — human-readable rendering (modules.md §3.6).
//!
//! Consumes: the final document. Produces: report.md bytes.
//!
//! Responsibilities: render the fixed section template (output-contract §5);
//! cite findings by exact finding_id; escape all embedded paths/titles/
//! messages (Markdown-injection rule); apply optional-field n/a-or-omit rules;
//! Appendix A descriptions from the built-in catalog (frozen v1).
//!
//! Determinism: ordering depends ONLY on document content (files[] are
//! re-sorted here for the §3 table; findings keep the analyzer's total order,
//! partitioned hot→warn→info WITHOUT re-sorting within a group). Double-render
//! byte-equality modulo provenance fields (`started_at`, `duration_ms`) is a
//! standing test obligation ([`tests/reporter_render.rs`]).
//!
//! Markdown-injection escaping: every embedded path/title/message/id goes
//! through [`escape_md`] with the PINNED metacharacter set
//! [`MD_ESCAPE_CHARS`] (backslash, backtick, asterisk, underscore, braces,
//! brackets, parentheses, hash, plus, hyphen, period, bang, pipe, angle
//! brackets, ampersand). Because `|` is in the set, table cells are covered
//! automatically. Inside code spans the backslash renders literally — that is
//! the accepted cost of never letting a hostile filename break out.
//!
//! Non-goals: reading source, IR, or scanner state; inventing aggregations
//! beyond the output-contract §5 template.

use std::collections::BTreeSet;

use crate::analyzer::{Confidence, Severity};
use crate::ir::ResolverTier;
use crate::schema::{DistributionEntry, FindingEntry, MetricsDocument, format_fixed_point};

// ---------------------------------------------------------------------------
// Pinned constants
// ---------------------------------------------------------------------------

/// Pinned Markdown metacharacter escape set (output-contract §2 paths rule).
/// Every char here is backslash-escaped wherever document-derived text is
/// embedded. `|` inclusion makes table cells safe without a second pass.
const MD_ESCAPE_CHARS: &[char] = &[
    '\\', '`', '*', '_', '{', '}', '[', ']', '(', ')', '#', '+', '-', '.', '!', '|', '<', '>', '&',
];

/// Frozen v1 heuristic catalog (read-code-first transcription of
/// docs/heuristics/catalog-v1.md §3 + §4 titles and one-line "Detects"
/// summaries). FROZEN for v1: supplies Appendix A descriptions ONLY —
/// never invent wording here; catalog changes flow from the doc, not the
/// reverse.
const HEURISTIC_CATALOG_V1: &[(&str, &str, &str)] = &[
    (
        "PERF-C-ALLOC-001",
        "Heap allocation inside loop",
        "Detects malloc/calloc/realloc/aligned_alloc/strdup calls nested anywhere inside a loop body.",
    ),
    (
        "PERF-C-ALLOC-002",
        "`free()` inside loop",
        "Detects free() directly inside a loop body (paired with ALLOC-001 it indicates allocator churn).",
    ),
    (
        "PERF-C-ALLOC-003",
        "`realloc()` linear growth in loop",
        "Detects realloc(p, n+k) additive-growth shapes nested in loops — quadratic memmove behavior.",
    ),
    (
        "PERF-C-STR-001",
        "String accumulation in loop (`strcat` family)",
        "Detects strcat/strncat with a destination reused across loop iterations (O(n²) rescan).",
    ),
    (
        "PERF-C-STR-002",
        "`strlen()` inside loop condition/body",
        "Detects strlen(s) in a loop condition or body where s is not reassigned in the loop.",
    ),
    (
        "PERF-C-STR-003",
        "Unbounded copy family (`strcpy`/`sprintf`/`gets`)",
        "Detects unbounded copy calls (strcpy, strcat, sprintf, gets, scanf %s) — unbounded-latency hotspots.",
    ),
    (
        "PERF-C-LOOP-001",
        "Deep loop nesting (≥3)",
        "Detects loop nesting depth ≥ 3 — O(n^k) candidate zones and primary input to hotspot maps.",
    ),
    (
        "PERF-C-LOOP-002",
        "Expensive setup / I/O call inside loop",
        "Detects printf-family, file-I/O, getenv, system, and regex-compilation calls nested in loops.",
    ),
    (
        "PERF-C-LOOP-003",
        "Char-wise stdio loop (`fgetc`/`fputc` family)",
        "Detects fgetc/getc/getchar/fputc/putc as the primary statement of a loop body.",
    ),
    (
        "PERF-C-LOOP-004",
        "Integer div/mod with variable divisor in innermost loop",
        "Detects / or % with a non-literal divisor inside the innermost loop body (literal divisors skipped by design).",
    ),
    (
        "PERF-C-CALL-001",
        "Direct recursion",
        "Detects a function body containing a call to its own name (presence-only; depth analysis is post-MVP).",
    ),
    (
        "PERF-C-CALL-002",
        "High cyclomatic complexity",
        "Detects branch-count complexity above threshold (default ≥ 15), reported per function.",
    ),
    (
        "PERF-C-ARCH-001",
        "Include-graph fan-out / cycles",
        "Detects include fan-out/fan-in, deep include chains, and cyclic includes from #include directives.",
    ),
    (
        "PERF-C-ARCH-002",
        "Hot-component aggregation",
        "Ranks components by aggregated metrics (complexity sum, alloc-site density, max loop depth, finding density).",
    ),
    (
        "PERF-C-ARCH-003",
        "Cross-file call coupling (name-based)",
        "Detects cross-file call edges via name matching — LOW confidence until a semantic resolver tier exists.",
    ),
    (
        "PERF-C-TYPE-001",
        "Large struct passed by value",
        "Detects struct-typed by-value parameters whose TU-defined struct reaches the approximate-size floor (LOW confidence — no real type layout at MVP).",
    ),
    (
        "PERF-C-TYPE-002",
        "Struct member ordering (padding candidate)",
        "Detects struct definitions whose naive aligned layout wastes padding versus the packed member sum beyond the threshold.",
    ),
    (
        "PERF-C-MEM-001",
        "sizeof-misuse shapes in allocation args",
        "Detects malloc(x * sizeof(ptr)) and sizeof misplacement shapes in allocation arguments — wrong-size allocations.",
    ),
    (
        "PERF-C-MEM-002",
        "Block copy/set family in loop (`memcpy`/`memmove`/`memset`)",
        "Detects memcpy/memmove/memset calls with non-literal size arguments inside loop bodies; warn when innermost.",
    ),
    (
        "PERF-C-LOOP-005",
        "Global variable written inside loop",
        "Detects assignment to a file-scope/global identifier inside a loop body (name-based scope classification, MEDIUM confidence).",
    ),
    (
        "PERF-C-LOOP-006",
        "Indirect call via function pointer in innermost loop",
        "Detects calls through function-pointer variables inside innermost loop bodies (LOW confidence without type info).",
    ),
    (
        "PERF-C-LOOP-007",
        "Linear membership scan nested in loop",
        "Detects comparison-against-array-element-indexed-by-outer-counter shapes — hand-rolled O(n²) lookup.",
    ),
    (
        "PERF-C-LOOP-008",
        "Pointer-chasing chain in innermost loop",
        "Detects member-access chains of depth ≥ 3 inside innermost loop bodies — pointer-chasing cache-miss candidates.",
    ),
];

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Render the full `report.md` content (output-contract §5 template).
///
/// `emitted_charts` holds the chart names/filenames the charts module
/// produced (e.g. `"include-graph"` or `"include-graph.svg"`); §5 renders
/// links ONLY for names in this list. Pure function of `(doc,
/// emitted_charts)` — LF newlines, trailing newline, byte-deterministic
/// modulo `scan.started_at` / `scan.duration_ms`.
#[must_use]
pub fn render_report(doc: &MetricsDocument, emitted_charts: &[String]) -> String {
    let mut out = String::new();
    render_header(&mut out, doc);
    render_executive_summary(&mut out, doc);
    render_provenance_section(&mut out, doc);
    render_codebase_statistics(&mut out, doc);
    render_findings(&mut out, doc);
    render_architecture(&mut out, doc, emitted_charts);
    render_appendix_a(&mut out, doc);
    render_appendix_b(&mut out);
    // Exactly one trailing newline (output-contract §2 encoding rule).
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

// ---------------------------------------------------------------------------
// Section renderers (fixed order, output-contract §5)
// ---------------------------------------------------------------------------

fn render_header(out: &mut String, doc: &MetricsDocument) {
    out.push_str("# Perfpal Report\n");
    // Blockquote provenance line; duration_ms renders `n/a` inline when None.
    let duration = match doc.scan.duration_ms {
        Some(ms) => ms.to_string(),
        None => "n/a".to_string(),
    };
    out.push_str(&format!(
        "> schema_version {} · perfpal {} · scanned {}\n",
        escape_md(&doc.schema_version),
        escape_md(&doc.tool.version),
        escape_md(&doc.scan.root_path),
    ));
    out.push_str(&format!(
        "> started {} · duration {duration} ms · {} files\n\n",
        escape_md(&doc.scan.started_at),
        doc.scan.file_count,
    ));
}

fn render_executive_summary(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## 1. Executive summary\n");
    let hot = findings_of_severity(doc, Severity::Hot);
    let warn_count = doc
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warn)
        .count();
    let info_count = doc
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Info)
        .count();
    let health = match &doc.parse_health_summary {
        Some(s) => format!(
            "Parse health: {} ok, {} partial, {} failed.",
            s.ok, s.partial, s.failed
        ),
        None => "Parse health: n/a.".to_string(),
    };
    out.push_str(&format!(
        "{} findings: {} hot, {} warn, {} info. {health}\n",
        doc.findings.len(),
        hot.len(),
        warn_count,
        info_count,
    ));
    // First 3 HOT findings in §4 order (all if fewer than 3).
    for f in hot.iter().take(3) {
        out.push_str(&format!(
            "- {} ({})\n",
            escape_md(&f.title),
            escape_md(&f.finding_id),
        ));
    }
    out.push('\n');
}

fn render_provenance_section(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## 2. Scan provenance & parse health\n");
    out.push_str(&format!(
        "- Tool: {} {}\n",
        escape_md(&doc.tool.name),
        escape_md(&doc.tool.version),
    ));
    out.push_str(&format!(
        "- Scan root: `{}`\n",
        escape_md(&doc.scan.root_path),
    ));
    out.push_str(&format!("- Files scanned: {}\n", doc.scan.file_count));
    // Per-language totals (languages, loc_total) belong to §3 Codebase
    // statistics per output-contract §5 — rendered there, not here.

    // Parse-health clause — omitted ENTIRELY when the summary is absent.
    if let Some(summary) = &doc.parse_health_summary {
        out.push_str("\n### Parse health summary\n\n");
        out.push_str("| status | files |\n|---|---:|\n");
        out.push_str(&format!("| ok | {} |\n", summary.ok));
        out.push_str(&format!("| partial | {} |\n", summary.partial));
        out.push_str(&format!("| failed | {} |\n", summary.failed));

        let unhealthy: Vec<&crate::schema::FileEntry> = doc
            .files
            .iter()
            .filter(|f| {
                matches!(
                    f.parse_health.status,
                    crate::ir::ParseStatus::Partial | crate::ir::ParseStatus::Failed
                )
            })
            .collect();
        out.push('\n');
        if unhealthy.is_empty() {
            out.push_str("No failed or partially parsed files.\n");
        } else {
            out.push_str("| path | status | error nodes |\n|---|---|---:|\n");
            for f in unhealthy {
                out.push_str(&format!(
                    "| {} | {} | {} |\n",
                    escape_md(&f.path),
                    parse_status_token(f.parse_health.status),
                    f.parse_health.error_node_count,
                ));
            }
        }
        out.push_str(
            "\nFindings overlapping parse-error regions are flagged \
             (`error_node_overlap`) and confidence-capped — never silently dropped.\n",
        );
    }
    out.push('\n');
}

fn render_codebase_statistics(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## 3. Codebase statistics\n");
    // Per-language totals clause (output-contract §5 §3): languages +
    // loc_total, rendered FIRST so the section leads with scan-wide totals.
    out.push_str(&format!(
        "- Languages: {}\n",
        escape_md(&doc.scan.languages.join(", ")),
    ));
    match &doc.scan.loc_total {
        Some(loc) => out.push_str(&format!(
            "- Total LOC: {} code / {} comment / {} blank\n",
            loc.code, loc.comment, loc.blank,
        )),
        None => out.push_str("- Total LOC: n/a\n"),
    }
    out.push('\n');
    if doc.files.is_empty() {
        out.push_str("No files scanned.\n\n");
        return;
    }
    // Top 10 by loc.code DESC, ties broken by path ASC — depends only on
    // document content (defensive re-sort; scanner order is not trusted here).
    let mut ranked: Vec<&crate::schema::FileEntry> = doc.files.iter().collect();
    ranked.sort_by(|a, b| {
        b.loc
            .code
            .cmp(&a.loc.code)
            .then_with(|| a.path.cmp(&b.path))
    });
    out.push_str("Top files by code LOC:\n\n");
    out.push_str("| path | loc.code | loc.comment | loc.blank | functions |\n");
    out.push_str("|---|---:|---:|---:|---:|\n");
    for f in ranked.iter().take(10) {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            escape_md(&f.path),
            f.loc.code,
            f.loc.comment,
            f.loc.blank,
            f.function_count,
        ));
    }
    out.push('\n');

    // Distributions tables — omitted entirely while stats.distributions is
    // None (the canonical form this phase). When populated, entries arrive
    // pre-sorted by (metric, bucket); document order is preserved.
    if let Some(distributions) = &doc.stats.distributions {
        for (metric, entries) in group_by_metric(distributions) {
            out.push_str(&format!(
                "Distribution — {}:\n\n| bucket | count |\n|---|---:|\n",
                escape_md(metric),
            ));
            for entry in entries {
                out.push_str(&format!(
                    "| {} | {} |\n",
                    escape_md(&entry.bucket),
                    entry.count,
                ));
            }
            out.push('\n');
        }
    }
}

fn render_findings(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## 4. Findings\n");
    if doc.findings.is_empty() {
        out.push('\n');
        out.push_str("No findings.\n\n");
        return;
    }
    // Stable severity partition hot→warn→info; WITHIN a group document order
    // is preserved EXACTLY (the analyzer already emits the total order
    // severity→confidence→rule_id→path→start_line→start_col — do NOT re-sort).
    let ordered = [
        findings_of_severity(doc, Severity::Hot),
        findings_of_severity(doc, Severity::Warn),
        findings_of_severity(doc, Severity::Info),
    ]
    .concat();

    for f in ordered {
        out.push_str(&format!(
            "#### {}  (`{}`)\n",
            escape_md(&f.title),
            escape_md(&f.finding_id),
        ));
        match &f.symbol {
            Some(symbol) => out.push_str(&format!(
                "- Location: `{}:{}` ({})\n",
                escape_md(&f.path),
                f.span.start_line,
                escape_md(symbol),
            )),
            None => out.push_str(&format!(
                "- Location: `{}:{}`\n",
                escape_md(&f.path),
                f.span.start_line,
            )),
        }
        out.push_str(&format!("- Message: {}\n", escape_md(&f.message)));
        out.push_str(&format!(
            "- Confidence: {} · Severity: {} · Resolver tier: {}\n",
            confidence_token(f.confidence),
            severity_token(f.severity),
            resolver_tier_token(f.resolver_tier),
        ));
        let mut flags: Vec<&str> = Vec::new();
        if f.macro_suspect {
            flags.push("macro_suspect");
        }
        if f.error_node_overlap {
            flags.push("error_node_overlap");
        }
        if !flags.is_empty() {
            out.push_str(&format!("- Flags: {}\n", flags.join(", ")));
        }
        if !f.evidence.is_empty() {
            let pairs: Vec<String> = f
                .evidence
                .iter()
                .map(|(k, v)| format!("{}={}", escape_md(k), escape_md(v)))
                .collect();
            out.push_str(&format!("- Evidence: {}\n", pairs.join(", ")));
        }
        out.push('\n');
    }
}

fn render_architecture(out: &mut String, doc: &MetricsDocument, emitted_charts: &[String]) {
    out.push_str("## 5. Architecture\n");
    if emitted_charts.is_empty() {
        out.push_str("No charts were emitted for this scan.\n\n");
    } else {
        for name in emitted_charts {
            let stem = name.strip_suffix(".svg").unwrap_or(name);
            // Chart names are NOT escape_md'd: output-contract §3 pins them
            // to kebab-case ASCII ([a-z0-9]+(-[a-z0-9]+)*), so they carry no
            // Markdown metacharacters by construction.
            out.push_str(&format!("[{}](charts/{stem}.svg)\n", chart_caption(stem)));
        }
        out.push('\n');
    }

    if let Some(components) = &doc.components
        && !components.is_empty()
    {
        out.push_str("Component ranking (hottest first):\n\n");
        out.push_str("| component | path prefix | rank | finding density (per KLOC) |\n");
        out.push_str("|---|---|---:|---:|\n");
        // Document order IS the ranked emission order (rank, path_prefix)
        // per the aggregator contract — preserved verbatim, no re-sort.
        for c in components {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                escape_md(&c.component_id),
                escape_md(&c.path_prefix),
                c.rank,
                // Shared HALF-UP fixed-point formatter (scale 2) — same
                // numeric text the charts module emits for the same density.
                format_fixed_point(c.metrics.finding_density, 2),
            ));
        }
        out.push('\n');
    }

    out.push_str(
        "The include graph reflects resolved internal `#include` edges between \
         scanned files; unresolved quoted includes are retained as data, and \
         external (system) headers are excluded from coupling edges.\n",
    );
    out.push_str(
        "Standing caveat: cross-file call edges are LOW confidence until a \
         semantic resolver tier exists (ADR-0001).\n\n",
    );
}

fn render_appendix_a(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## Appendix A. Heuristic reference\n");
    if doc.findings.is_empty() {
        out.push_str("No findings — no heuristics to reference.\n\n");
        return;
    }
    // EXACTLY the rule_ids present in this report's findings, sorted for
    // determinism (ordering must depend only on document content).
    let rule_ids: BTreeSet<&str> = doc.findings.iter().map(|f| f.rule_id.as_str()).collect();
    out.push_str("| rule_id | title | description |\n|---|---|---|\n");
    for rule_id in rule_ids {
        let (_, title, description) = HEURISTIC_CATALOG_V1
            .iter()
            .find(|(id, _, _)| *id == rule_id)
            .unwrap_or_else(|| panic!("uncataloged rule_id {rule_id} reached Appendix A"));
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            escape_md(rule_id),
            escape_md(title),
            escape_md(description),
        ));
    }
    out.push('\n');
}

fn render_appendix_b(out: &mut String) {
    out.push_str("## Appendix B. How to read this report\n");
    out.push_str(
        "**Severity** bands (`hot`, `warn`, `info`) rank expected runtime or \
         architecture impact: review `hot` findings first, schedule `warn`, \
         and treat `info` as context signal feeding aggregation.\n",
    );
    out.push_str(
        "**Confidence** tiers (`HIGH`, `MEDIUM`, `LOW`) express the \
         false-positive risk of the detection shape — pure-syntax detections \
         are HIGH, shape heuristics with documented FP classes are MEDIUM, and \
         approximations (size models, cross-file name matching) are LOW. \
         Downgrade flags (`macro_suspect`, `error_node_overlap`) mark findings \
         whose evidence quality is reduced; their confidence is capped at \
         MEDIUM.\n",
    );
    out.push_str(
        "Every finding is an **optimization candidate, never a certainty**: a \
         candidate for human review, not a defect claim. Perfpal v1 runs on a \
         purely syntactic tier (`resolver_tier: syntactic`) — no macro \
         expansion, no type resolution.\n",
    );
    out.push_str(&format!(
        "For programmatic consumption, read `metrics.json` — the source of \
         truth — and validate it against the bundled schema \
         `{}` shipped alongside it. Where this report and \
         `metrics.json` ever disagree, `metrics.json` wins.\n\n",
        crate::schema::SCHEMA_ARTIFACT_NAME
    ));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Findings of one severity, in DOCUMENT order (never re-sorted here).
fn findings_of_severity(doc: &MetricsDocument, severity: Severity) -> Vec<&FindingEntry> {
    doc.findings
        .iter()
        .filter(|f| f.severity == severity)
        .collect()
}

/// Group distribution entries by metric, preserving document order within
/// and across groups (entries arrive pre-sorted by `(metric, bucket)`).
fn group_by_metric(entries: &[DistributionEntry]) -> Vec<(&str, Vec<&DistributionEntry>)> {
    let mut groups: Vec<(&str, Vec<&DistributionEntry>)> = Vec::new();
    for entry in entries {
        match groups.last_mut() {
            Some((metric, list)) if *metric == entry.metric.as_str() => list.push(entry),
            _ => groups.push((entry.metric.as_str(), vec![entry])),
        }
    }
    groups
}

/// Fixed caption per known chart-set-v1 name (ADR-0002); unknown names get a
/// generic caption (forward compatibility: consumers tolerate new charts).
/// Captions are reporter-owned static text — safe by construction, never
/// document-derived (the dynamic name is charset-pinned per output-contract
/// §3), so they bypass [`escape_md`].
fn chart_caption(stem: &str) -> String {
    match stem {
        "include-graph" => {
            "Include graph — internal #include dependencies between scanned files.".to_string()
        }
        "component-coupling" => {
            "Component coupling — cross-file call edges between components.".to_string()
        }
        "hot-components" => {
            "Hot components — ranking by aggregated bottleneck metrics.".to_string()
        }
        other => format!("Chart: {other}."),
    }
}

/// Escape Markdown metacharacters with the PINNED set [`MD_ESCAPE_CHARS`].
/// Applied to every embedded path, title, message, id, symbol, and caption.
fn escape_md(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if MD_ESCAPE_CHARS.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn severity_token(severity: Severity) -> &'static str {
    match severity {
        Severity::Hot => "hot",
        Severity::Warn => "warn",
        Severity::Info => "info",
    }
}

fn confidence_token(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::High => "HIGH",
        Confidence::Medium => "MEDIUM",
        Confidence::Low => "LOW",
    }
}

fn resolver_tier_token(tier: ResolverTier) -> &'static str {
    match tier {
        ResolverTier::Syntactic => "syntactic",
        ResolverTier::Semantic => "semantic",
        ResolverTier::Dynamic => "dynamic",
    }
}

fn parse_status_token(status: crate::ir::ParseStatus) -> &'static str {
    match status {
        crate::ir::ParseStatus::Ok => "ok",
        crate::ir::ParseStatus::Partial => "partial",
        crate::ir::ParseStatus::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    //! Unit locks on the two tiny pure helpers; full-template obligations live
    //! in tests/reporter_render.rs.

    use super::*;

    #[test]
    fn escape_md_pins_exact_set() {
        assert_eq!(escape_md("plain"), "plain");
        assert_eq!(
            escape_md("#incl<ude>&\"x\".c"),
            "\\#incl\\<ude\\>\\&\"x\"\\.c"
        );
        assert_eq!(escape_md("a|b"), "a\\|b");
        assert_eq!(
            escape_md("*_{}[]()+-!`\\"),
            "\\*\\_\\{\\}\\[\\]\\(\\)\\+\\-\\!\\`\\\\"
        );
    }

    #[test]
    fn round_half_up_is_fixed_point_two_half_up() {
        // Locks the SHARED formatter (crate::schema::format_fixed_point) at
        // the reporter's scale-2 call shape.
        assert_eq!(format_fixed_point(0.0, 2), "0.00");
        assert_eq!(format_fixed_point(8.25, 2), "8.25");
        assert_eq!(format_fixed_point(2.675, 2), "2.68"); // ties-to-even would say 2.67
        assert_eq!(format_fixed_point(1.005, 2), "1.01");
        assert_eq!(format_fixed_point(12.5, 2), "12.50");
        assert_eq!(format_fixed_point(0.999, 2), "1.00"); // carry into integer part
        assert_eq!(format_fixed_point(-2.675, 2), "-2.68");
        // Negative-zero guarantee: "-0.00" must never leak.
        assert_eq!(format_fixed_point(-0.001, 2), "0.00");
    }
}
