//! Atlas below-graph progressive charts (plan `atlas-build` step 12).
//!
//! Four designed blocks replace the retired seven-block text wall:
//! weight (production-first share), blast radius, file detail, and one
//! coverage/links line. The canvas + detail rail above stay untouched; every
//! chart is state-unconditional (a named stub replaces absent data), emits
//! **no** script/handler/network reference, and uses the canonical
//! [`crate::reporter::atlas_metrics`] formatters for every measured string.
//!
//! Determinism: bars use tenths geometry (`format_fixed_point`/`round_half_up`),
//! every traversal follows the shipped `AtlasView` order, and no locale/clock/
//! randomness enters the output. Bars are `div`/inline styles, never `<table>`.

use std::collections::BTreeSet;

use crate::reporter::atlas::{AtlasFile, AtlasView};
use crate::reporter::atlas_brief::{
    FunctionAnchors, dominant_finding_anchor, file_cost_anchor, file_link_anchor,
    function_display_name, function_link_anchor, sanitize_atlas_text,
};
use crate::reporter::atlas_metrics::{
    NO_ENTRY, NO_PRODUCTION_COST, NO_SCANNED_COST, NO_SOURCE_FUNCTIONS, format_call_degree,
    format_component_share, format_count, format_dependents, format_finding, format_nonproduction,
    format_share,
};
use crate::reporter::{
    escape_html, file_anchor_map, finding_anchor_assignments, function_anchor_map,
};
use crate::schema::{MetricsDocument, format_fixed_point, round_half_up};

/// Render `#atlas-brief`: the four designed below-graph blocks.
///
/// `doc` is only used for the shared deterministic anchor maps (file /
/// function / finding) so chart links resolve to the same rows as the rail
/// and the drill pages — the view model carries every measured value.
#[must_use]
pub fn render_charts(view: &AtlasView, doc: &MetricsDocument) -> String {
    let mut out = String::new();
    out.push_str("<section id=\"atlas-brief\" class=\"atlas-brief\">\n");
    out.push_str("<div id=\"atlas-charts\" class=\"atlas-charts\">\n");
    render_weight_chart(&mut out, view);
    render_blast_chart(&mut out, view);
    render_file_chart(&mut out, view, doc);
    render_coverage(&mut out);
    out.push_str("</div>\n</section>\n");
    out
}

// ---------------------------------------------------------------------------
// Weight — "Where is the mass?"
// ---------------------------------------------------------------------------

fn render_weight_chart(out: &mut String, view: &AtlasView) {
    out.push_str("<div class=\"atlas-chart\" data-atlas-block=\"weight-chart\">\n");
    out.push_str("<h3>Where is the mass?</h3>\n");
    out.push_str(
        "<p class=\"atlas-chart-caption\">share of scanned static cost · production first · bar = cost relative to the largest item in each column · share printed</p>\n",
    );

    let total = view.totals.total_cost;
    let production_cost = view
        .files
        .iter()
        .filter(|file| file.production)
        .map(|file| file.cost)
        .fold(0_u64, u64::saturating_add);
    // The weight ribbon and its non-production band are one paired encoding:
    // both values come from the SAME integer-tenths split, so the pair sums to
    // exactly 100.0% (the step-12 paired-share exception in atlas-metrics §5).
    let production_tenths = if total == 0 {
        0
    } else {
        round_half_up(production_cost as f64 * 1000.0 / total as f64, 0)
    };
    let production_pct = tenths_percent(production_tenths);
    let nonproduction_pct = tenths_percent(1000 - production_tenths);
    let band = if total == 0 {
        format_nonproduction(0, 0, view.coverage.demoted_files)
    } else {
        format!(
            "non-production (test/bench/other) = {} of scanned static cost ({} files)",
            nonproduction_pct,
            format_count(view.coverage.demoted_files as u64)
        )
    };

    // Layout §5: an all-test chart leads with the non-production band.
    if view.state.all_test {
        out.push_str(&format!(
            "<p class=\"atlas-band\">{}</p>\n",
            escape_html(&band)
        ));
    }

    if total == 0 {
        out.push_str(&format!("<p class=\"atlas-stub\">{NO_SCANNED_COST}</p>\n"));
    } else {
        render_share_ribbon(out, production_tenths, &production_pct, &nonproduction_pct);
        let production: Vec<&AtlasFile> = view
            .files
            .iter()
            .filter(|file| file.production)
            .take(5)
            .collect();
        if production.is_empty() {
            out.push_str(&format!(
                "<p class=\"atlas-stub\">{NO_PRODUCTION_COST}</p>\n"
            ));
        } else {
            let file_max = production.iter().map(|file| file.cost).max().unwrap_or(0);
            out.push_str("<div class=\"atlas-bar-columns\">\n");
            out.push_str("<div class=\"atlas-bar-col\">\n<h4>top production-class files</h4>\n");
            for file in &production {
                push_bar_row(
                    out,
                    &file.path,
                    &file.path,
                    &bar_percent(file.cost, file_max),
                    &format_share(file.cost, total),
                );
            }
            out.push_str("</div>\n<div class=\"atlas-bar-col\">\n");
            out.push_str("<h4>top production components</h4>\n");
            let components: Vec<_> = view
                .components
                .iter()
                .filter(|component| component.production_file_count > 0)
                .take(3)
                .collect();
            if components.is_empty() {
                out.push_str("<p class=\"atlas-stub\">no production component recorded</p>\n");
            } else {
                let component_max = components
                    .iter()
                    .map(|component| component.cost)
                    .max()
                    .unwrap_or(0);
                for component in &components {
                    push_bar_row(
                        out,
                        &component.prefix,
                        &component.prefix,
                        &bar_percent(component.cost, component_max),
                        &format_component_share(component.cost, total, component.file_count),
                    );
                }
            }
            out.push_str("</div>\n</div>\n");
            out.push_str(
                "<p class=\"atlas-chart-caption\">Production class includes unreached production files (flagged in the file chart below). Component shares include every member file (including non-production members); component groups are approximate prefix groups.</p>\n",
            );
        }
    }

    if !view.state.all_test {
        out.push_str(&format!(
            "<p class=\"atlas-band\">{}</p>\n",
            escape_html(&band)
        ));
    }
    out.push_str("</div>\n");
}

/// 100% production / non-production ribbon. Segment widths are integer tenths
/// of a percent and sum to exactly 1000 tenths, so the bar never leaves a gap.
fn render_share_ribbon(
    out: &mut String,
    production_tenths: i64,
    production_pct: &str,
    nonproduction_pct: &str,
) {
    // The caller derives both labels from the SAME integer tenths split, so the
    // pair sums to exactly 100.0% and matches the drawn geometry. The
    // denominator is part of every emitted text run (visible, title, aria).
    let nonproduction_tenths = 1000 - production_tenths;
    let denominator = " of scanned static cost";

    out.push_str("<div class=\"atlas-share-ribbon\" role=\"img\" aria-label=\"");
    out.push_str(&escape_html(&format!(
        "production {production_pct}{denominator} · non-production {nonproduction_pct}{denominator}"
    )));
    out.push_str("\">\n");
    out.push_str(&format!(
        "<span class=\"atlas-share-seg is-prod\" style=\"width:{}%\" title=\"production = {}{}\"></span>\n",
        tenths_text(production_tenths),
        escape_html(production_pct),
        denominator,
    ));
    out.push_str(&format!(
        "<span class=\"atlas-share-seg is-nonprod\" style=\"width:{}%\" title=\"non-production = {}{}\"></span>\n",
        tenths_text(nonproduction_tenths),
        escape_html(nonproduction_pct),
        denominator,
    ));
    out.push_str("</div>\n");
    out.push_str(&format!(
        "<p class=\"atlas-ribbon-labels\"><span>production {}{}</span><span>non-production {}{}</span></p>\n",
        escape_html(production_pct),
        denominator,
        escape_html(nonproduction_pct),
        denominator,
    ));
}

/// `360` tenths -> `36.0`.
fn tenths_text(tenths: i64) -> String {
    format!("{}.{}", tenths / 10, tenths % 10)
}

/// Contract-style rounded share from tenths: whole tenths print an integer
/// (`360` -> `36%`), otherwise one decimal (`2` -> `0.2%`).
fn tenths_percent(tenths: i64) -> String {
    if tenths % 10 == 0 {
        format!("{}%", tenths / 10)
    } else {
        format!("{}.{}%", tenths / 10, tenths % 10)
    }
}

// ---------------------------------------------------------------------------
// Blast radius — "What breaks if I touch it?"
// ---------------------------------------------------------------------------

fn render_blast_chart(out: &mut String, view: &AtlasView) {
    out.push_str("<div class=\"atlas-chart\" data-atlas-block=\"blast-chart\">\n");
    out.push_str("<h3>What breaks if I touch it?</h3>\n");
    let blast = &view.blast_radius;
    match blast.target.as_deref() {
        Some(target) => {
            out.push_str(&format!(
                "<p class=\"atlas-chart-caption\">target (elected entry file) <span class=\"atlas-mono atlas-path\">{}</span></p>\n",
                escape_html(target),
            ));
            out.push_str(&format!(
                "<p class=\"atlas-blast-line\">{}</p>\n",
                escape_html(&format_dependents(
                    blast.dependents,
                    view.reachability.source_files,
                    blast.dependent_multiple,
                    blast.dependent_top_pct,
                )),
            ));
            out.push_str(&format!(
                "<div class=\"atlas-bar-row\"><span class=\"atlas-bar-label\">include coverage</span><span class=\"atlas-bar-track\"><span class=\"atlas-bar-fill\" style=\"width:{}%\"></span></span><span class=\"atlas-bar-share\">{} of {} include edges</span></div>\n",
                bar_percent(
                    blast.include_edges_resolved as u64,
                    blast.include_edges_total as u64,
                ),
                format_count(blast.include_edges_resolved as u64),
                format_count(blast.include_edges_total as u64),
            ));
            if !blast.top_dependents.is_empty() {
                let max = blast
                    .top_dependents
                    .iter()
                    .map(|(_, edges)| *edges)
                    .max()
                    .unwrap_or(0);
                out.push_str("<h4>top dependent files</h4>\n");
                for (path, edges) in &blast.top_dependents {
                    push_bar_row(
                        out,
                        path,
                        path,
                        &bar_percent(*edges as u64, max as u64),
                        &format!("{} include edges", format_count(*edges as u64)),
                    );
                }
            }
            out.push_str(&format!(
                "<p class=\"atlas-call-degree\">{}</p>\n",
                escape_html(&format_call_degree(blast.caller_files, blast.callee_files,)),
            ));
        }
        None => {
            out.push_str(&format!(
                "<p class=\"atlas-stub\">{NO_ENTRY} — blast radius unavailable</p>\n"
            ));
        }
    }
    out.push_str("</div>\n");
}

// ---------------------------------------------------------------------------
// File detail — "Which file carries the cost?"
// ---------------------------------------------------------------------------

fn render_file_chart(out: &mut String, view: &AtlasView, doc: &MetricsDocument) {
    out.push_str("<div class=\"atlas-chart\" data-atlas-block=\"file-chart\">\n");
    out.push_str("<h3>Which file carries the cost?</h3>\n");
    out.push_str(
        "<p class=\"atlas-chart-caption\">bar = file cost relative to the largest file shown · exact file cost and median anchor printed · role = ENTRY / production class (PRODUCTION, TEST, BENCH, or another path class) / UNREACHED</p>\n",
    );

    let mut files: Vec<&AtlasFile> = view
        .files
        .iter()
        .filter(|file| file.production)
        .take(6)
        .collect();
    if files.is_empty() {
        files = view.files.iter().take(6).collect();
    }
    if files.is_empty() {
        out.push_str(&format!(
            "<p class=\"atlas-stub\">{NO_SOURCE_FUNCTIONS}</p>\n</div>\n"
        ));
        return;
    }

    let file_anchors = file_anchor_map(doc);
    let function_anchors = function_anchor_map(doc);
    let function_costs = FunctionAnchors::new(doc);
    let (_, finding_anchors) = finding_anchor_assignments(doc);
    let unreached: BTreeSet<&str> = view
        .unreachable_production
        .iter()
        .chain(view.unreachable_nonproduction.iter())
        .map(String::as_str)
        .collect();
    let max = files.iter().map(|file| file.cost).max().unwrap_or(0);
    let shown = files.len();
    let entry_path = view.entry.point.as_ref().map(|point| point.path.as_str());

    for file in files {
        let role = chart_role(file, &unreached, entry_path);
        out.push_str("<div class=\"atlas-file-row\">\n");
        out.push_str(&format!(
            "<div class=\"atlas-file-head\"><span class=\"atlas-file-path atlas-mono\" title=\"{}\">{}</span><span class=\"atlas-file-role\">{}</span></div>\n",
            escape_html(&file.path),
            escape_html(&file.path),
            escape_html(&role),
        ));
        let file_anchor = file_link_anchor(&file_anchors, &file.path);
        out.push_str(&format!(
            "<div class=\"atlas-file-bar\"><a class=\"atlas-bar-track\" href=\"atlas-weight.html#atlas-{anchor}\" title=\"weight detail for {path}\"><span class=\"atlas-bar-fill\" style=\"width:{}%\"></span></a></div>\n",
            bar_percent(file.cost, max),
            anchor = escape_html(&file_anchor),
            path = escape_html(&file.path),
        ));
        out.push_str(&format!(
            "<div class=\"atlas-file-cost\">{}</div>\n",
            escape_html(&file_cost_anchor(file)),
        ));
        if file.top_functions.is_empty() {
            out.push_str("<div class=\"atlas-file-fns\">no functions recorded</div>\n");
        } else {
            let mut links = Vec::new();
            for function in file.top_functions.iter().take(3) {
                let anchor = function_link_anchor(&function_anchors, &function.id);
                let name = function_display_name(&function.name);
                let cost_anchor = doc
                    .stats
                    .functions
                    .iter()
                    .find(|stats| stats.id == function.id)
                    .map(|stats| function_costs.format(stats));
                let title = match cost_anchor {
                    Some(anchor) => {
                        format!("{}:{} {} · {}", file.path, function.line, name, anchor)
                    }
                    None => format!("{}:{} {}", file.path, function.line, name),
                };
                links.push(format!(
                    "<a class=\"atlas-mono\" href=\"profile-queue.html#{anchor}\" title=\"{title}\">{name}:{line}</a>",
                    anchor = escape_html(&anchor),
                    title = escape_html(&title),
                    name = escape_html(&name),
                    line = function.line,
                ));
            }
            out.push_str(&format!(
                "<div class=\"atlas-file-fns\">functions: {}</div>\n",
                links.join(" · ")
            ));
        }
        match &file.finding {
            Some(dominant) => {
                let anchor = dominant_finding_anchor(doc, file, dominant, &finding_anchors);
                out.push_str(&format!(
                    "<div class=\"atlas-file-find\">FIND <a class=\"atlas-mono\" href=\"findings-detail.html#{anchor}\">{rule}</a> · {finding} — {title}</div>\n",
                    anchor = escape_html(&anchor),
                    rule = escape_html(&dominant.rule_id),
                    finding = escape_html(&format_finding(dominant.severity, dominant.count)),
                    title = escape_html(&sanitize_atlas_text(&dominant.title)),
                ));
            }
            None => out.push_str("<div class=\"atlas-file-find\">no findings on this file</div>\n"),
        }
        out.push_str("</div>\n");
    }
    out.push_str(&format!(
        "<p class=\"atlas-chart-caption\">showing top {} of {} source files (production first)</p>\n",
        shown,
        format_count(view.files.len() as u64),
    ));
    out.push_str("</div>\n");
}

fn chart_role(file: &AtlasFile, unreached: &BTreeSet<&str>, entry_path: Option<&str>) -> String {
    if entry_path == Some(file.path.as_str()) {
        return "ENTRY".to_string();
    }
    if unreached.contains(file.path.as_str()) {
        return "UNREACHED".to_string();
    }
    if !file.production {
        let class = file.path_class.to_ascii_uppercase();
        return if class.is_empty() {
            "OTHER".to_string()
        } else {
            class
        };
    }
    "PRODUCTION".to_string()
}

// ---------------------------------------------------------------------------
// Coverage + links
// ---------------------------------------------------------------------------

fn render_coverage(out: &mut String) {
    out.push_str("<div class=\"atlas-chart atlas-coverage\" data-atlas-block=\"coverage\">\n");
    out.push_str("<p class=\"atlas-coverage\">Full scan:</p>\n");
    out.push_str("<nav class=\"atlas-links\" aria-label=\"Atlas artifacts\">\n");
    for (href, label) in [
        ("report.md", "report.md"),
        ("metrics.json", "metrics.json"),
        ("atlas-weight.html", "weight detail"),
        ("atlas-q2.html", "blast drill"),
        ("findings-detail.html#ranked-actions", "complete actions"),
        ("profile-queue.html", "profile queue"),
    ] {
        out.push_str(&format!(
            "<a href=\"{}\">{}</a>\n",
            escape_html(href),
            escape_html(label)
        ));
    }
    out.push_str("</nav>\n</div>\n");
}

// ---------------------------------------------------------------------------
// Shared bar helpers
// ---------------------------------------------------------------------------

/// Bar width as a percentage string (one decimal, deterministic).
fn bar_percent(value: u64, max: u64) -> String {
    if max == 0 {
        return "0.0".to_string();
    }
    format_fixed_point(value as f64 * 100.0 / max as f64, 1)
}

fn push_bar_row(out: &mut String, label: &str, title: &str, width: &str, share: &str) {
    out.push_str(&format!(
        "<div class=\"atlas-bar-row\"><span class=\"atlas-bar-label\" title=\"{title}\">{label}</span><span class=\"atlas-bar-track\"><span class=\"atlas-bar-fill\" style=\"width:{width}%\"></span></span><span class=\"atlas-bar-share\">{share}</span></div>\n",
        title = escape_html(title),
        label = escape_html(label),
        width = width,
        share = escape_html(share),
    ));
}
