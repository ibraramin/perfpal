//! Atlas report blocks: the restored seven-block brief, the first-page detail
//! rail and the standalone drill pages (plan `atlas-build` steps 5/11/12;
//! restored additively by plan `polish-report` H1).
//!
//! The pre-merge `feat/atlas-sim` report exposed seven `data-atlas-block`
//! surfaces — `answer`, `actions`, `weight`, `blast`, `cards`, `noise`,
//! `links`. They are restored around main's rail and four designed charts
//! ([`crate::reporter::atlas_charts`]): `weight` and `blast` render as prose
//! immediately before their charts, the rest sit after the chart grid, and
//! `links` is last. Every state renders every block (A10); a named stub
//! replaces absent content. No schema field is added or changed, and the
//! module emits **no** script, event handler or network reference.
//!
//! Metric contract: every measured number carries its unit/anchor via
//! [`crate::reporter::atlas_metrics`]; M-08's percentile is
//! `blast_radius.dependent_top_pct` verbatim (never recomputed here).

use std::collections::{BTreeMap, BTreeSet};

use crate::reporter::atlas::{
    AtlasFile, AtlasFinding, AtlasFunction, AtlasView, EntryPolicy, function_cost_u64,
};
use crate::reporter::atlas_metrics::{
    NO_FINDINGS, NO_INCLUDE_EDGES, NO_PRODUCTION_COST, NO_RESOLVED_DEPENDENTS, NO_SCANNED_COST,
    NO_SOURCE_FUNCTIONS, format_call_degree, format_caps, format_component_share, format_count,
    format_demoted_files, format_dependents, format_edge_mix, format_entry, format_file_cost,
    format_finding, format_function_cost, format_nonproduction, format_parse,
    format_severity_confidence, format_share, format_trip_provenance, format_unresolved_includes,
};
use crate::reporter::atlas_render::mid_ellipsis;
use crate::reporter::format::{ACCENT, HEAD, INK, MUTED, PAPER, RULE, ZEBRA};
use crate::reporter::views::{self, CostScale, ScaleOrdering, scale_of};
use crate::reporter::{
    escape_html, file_anchor_map, finding_anchor_assignments, function_anchor_map,
};
use crate::schema::{FindingEntry, FunctionStats, MetricsDocument};

/// One schema-component page link resolved against `emitted_charts`.
struct ComponentPage {
    prefix: String,
    rank: u32,
    href: String,
}

/// Maximum number of file cards shown in the first-page detail rail.
pub const RAIL_FILE_CAP: usize = 12;

/// One compact start-here chip for the Atlas key-metrics strip. The ranking
/// comes from the shipped [`views::kv1`] view (never re-derived here).
#[derive(Debug, Clone, PartialEq)]
pub struct StartAction {
    /// `path:line` visible label.
    pub location: String,
    /// Sanitized rule title (visible label).
    pub rule: String,
    pub function: Option<String>,
    /// `severity · confidence` badge text.
    pub severity_confidence: String,
    pub path_class: String,
    pub production: bool,
    /// Rendered cost anchor (M-05) when a containing function exists.
    pub cost_anchor: Option<String>,
    /// Existing `findings-detail.html#...` anchor target.
    pub href: String,
}

impl StartAction {
    /// Full tooltip: path/function/rule/severity/confidence/cost anchor.
    #[must_use]
    pub fn title(&self) -> String {
        let mut parts = vec![self.location.clone()];
        if let Some(function) = &self.function {
            parts.push(function.clone());
        }
        parts.push(self.rule.clone());
        parts.push(self.severity_confidence.clone());
        parts.push(if self.production {
            "production".to_string()
        } else {
            format!("non-production ({})", self.path_class)
        });
        if let Some(anchor) = &self.cost_anchor {
            parts.push(anchor.clone());
        }
        parts.join(" · ")
    }
}

/// Top three KV1 actions for the first-page strip. Reuses the shipped
/// deterministic ranking and the existing finding-anchor/location formatting.
#[must_use]
pub fn start_actions(doc: &MetricsDocument) -> Vec<StartAction> {
    let (_, finding_anchors) = finding_anchor_assignments(doc);
    let kv1 = views::kv1(doc);
    kv1.rows
        .iter()
        .take(3)
        .map(|row| {
            let finding = &doc.findings[row.finding_index];
            StartAction {
                location: location(finding),
                rule: sanitize_atlas_text(&finding.title),
                function: row.function.clone(),
                severity_confidence: format_severity_confidence(
                    finding.severity,
                    finding.confidence,
                ),
                path_class: row.path_class.clone(),
                production: row.production,
                cost_anchor: row.cost_anchor.clone(),
                href: format!(
                    "findings-detail.html#{}",
                    finding_anchors[row.finding_index]
                ),
            }
        })
        .collect()
}

/// Render the restored `answer` block: one sentence naming the elected entry
/// and why, the largest static-cost file, the first ranked action and the
/// structural caveat. It is placed after the page chrome and before the
/// canvas/rail stage (A8), so the answer survives the first scroll.
#[must_use]
pub fn render_answer(view: &AtlasView, doc: &MetricsDocument) -> String {
    let (_, finding_anchors) = finding_anchor_assignments(doc);
    let kv1 = views::kv1(doc);
    let mut out = String::new();
    out.push_str("<div class=\"atlas-block\" data-atlas-block=\"answer\" id=\"atlas-answer\">\n");
    out.push_str("<h3>What to open first</h3>\n<p class=\"atlas-answer-line\">");
    out.push_str(&format!(
        "<strong>{}</strong>. {}",
        escape_html(&format_entry(
            view.entry.policy.label(),
            &view.entry.confidence,
            view.entry.alternatives.len(),
        )),
        escape_html(entry_why(view.entry.policy)),
    ));
    out.push(' ');
    match hottest_file(view) {
        Some(file) => out.push_str(&format!(
            "The largest static cost is {} ({}).",
            escape_html(&file.path),
            escape_html(&file_cost_anchor(file)),
        )),
        None => out.push_str(&escape_html("No file with functions was recorded.")),
    }
    out.push(' ');
    if let Some(row) = kv1.rows.first() {
        let finding = &doc.findings[row.finding_index];
        out.push_str("Open first: <a href=\"findings-detail.html#");
        out.push_str(&escape_html(&finding_anchors[row.finding_index]));
        out.push_str("\"><code>");
        out.push_str(&escape_html(&location(finding)));
        out.push_str("</code></a>");
        if let Some(function) = &row.function {
            out.push_str(" <span class=\"atlas-fn\">");
            out.push_str(&escape_html(function));
            out.push_str("</span>");
        }
        out.push_str(&format!(
            " — {} ({}).",
            escape_html(&sanitize_atlas_text(&finding.title)),
            escape_html(&format_severity_confidence(
                finding.severity,
                finding.confidence
            )),
        ));
    } else {
        let text = no_action_text(&kv1);
        out.push_str(&escape_html(&text));
        if !text.ends_with('.') {
            out.push('.');
        }
    }
    out.push_str(&format!(
        " <span class=\"atlas-caveat\">Caveat: {}</span>",
        escape_html(&caveat(view))
    ));
    out.push_str("</p>\n</div>\n");
    out
}

/// Render the restored `actions` block: the shipped KV1 order with
/// severity/confidence, path class, `path:line` anchor, why/trip provenance,
/// same-location dedupe, and an honest `N of M` pointer to the complete list.
#[must_use]
pub fn render_actions(view: &AtlasView, doc: &MetricsDocument) -> String {
    let (_, finding_anchors) = finding_anchor_assignments(doc);
    let kv1 = views::kv1(doc);
    let function_anchors = FunctionAnchors::new(doc);
    let mut out = String::new();
    out.push_str("<div class=\"atlas-block\" data-atlas-block=\"actions\" id=\"atlas-actions\">\n");
    out.push_str("<h3>Start here — ranked actions (production first)</h3>\n");
    if kv1.rows.is_empty() {
        out.push_str(&format!("<p>{}</p>\n", escape_html(&no_action_text(&kv1))));
    } else {
        out.push_str("<ol class=\"atlas-action-list\">\n");
        let shown = kv1.rows.len().min(3);
        let mut seen_locations: BTreeMap<(String, u32), usize> = BTreeMap::new();
        for row in kv1.rows.iter().take(shown) {
            let finding = &doc.findings[row.finding_index];
            let same_as = seen_locations
                .get(&(finding.path.clone(), finding.span.start_line))
                .copied();
            out.push_str(&format!(
                "<li class=\"atlas-action\"><span class=\"atlas-rank\">#{rank}</span>\
                 <a class=\"atlas-loc\" href=\"findings-detail.html#{anchor}\"><code>{location}</code></a>",
                rank = row.rank,
                anchor = escape_html(&finding_anchors[row.finding_index]),
                location = escape_html(&location(finding)),
            ));
            if let Some(function) = &row.function {
                out.push_str(&format!(
                    " <span class=\"atlas-fn\">{}</span>",
                    escape_html(function)
                ));
            }
            out.push_str(&format!(
                " <span class=\"atlas-rule\">{}</span>\
                 <span class=\"atlas-badge\">{}</span>\
                 <span class=\"atlas-class{}\">{}</span>",
                escape_html(&sanitize_atlas_text(&finding.title)),
                escape_html(&format_severity_confidence(
                    finding.severity,
                    finding.confidence
                )),
                if row.production { "" } else { " nonprod" },
                escape_html(&row.path_class),
            ));
            if let Some(prior) = same_as {
                out.push_str(&format!(
                    " <span class=\"atlas-note\">same location as #{prior}</span>"
                ));
            }
            seen_locations
                .entry((finding.path.clone(), finding.span.start_line))
                .or_insert(row.rank);
            out.push_str(&format!(
                "<div class=\"atlas-anchor\">{}</div>\n",
                escape_html(&match row.function_index {
                    Some(index) => function_anchors.format(&doc.stats.functions[index]),
                    None => "no enclosing function recorded".to_string(),
                })
            ));
            if let Some(index) = row.function_index {
                let function = &doc.stats.functions[index];
                if let Some(trip) =
                    format_trip_provenance(function.known_trip_value, function.unknown_trip_loops)
                {
                    out.push_str(&format!(
                        "<div class=\"atlas-trip\">{}</div>\n",
                        escape_html(&trip)
                    ));
                }
            }
            out.push_str(&format!(
                "<div class=\"atlas-why\">{}</div></li>\n",
                escape_html(&sanitize_atlas_text(&finding.message))
            ));
        }
        out.push_str("</ol>\n");
        out.push_str(&format!(
            "<p class=\"atlas-counts\">showing {} of {} ranked actions (production-first) · {} · {} in the full list</p>\n",
            shown,
            kv1.total_actions,
            escape_html(&if view.coverage.demoted_files == 1 {
                format!(
                    "{} non-production file demoted",
                    view.coverage.demoted_files
                )
            } else {
                format!(
                    "{} non-production files demoted",
                    view.coverage.demoted_files
                )
            }),
            escape_html(&views::plural(
                kv1.component_findings,
                "component-level finding"
            )),
        ));
    }
    out.push_str(&format!(
        "<p class=\"atlas-more\"><a href=\"findings-detail.html#ranked-actions\">Complete ranked action list — {} (production-first)</a></p>\n",
        escape_html(&views::plural(kv1.total_actions, "action")),
    ));
    out.push_str("</div>\n");
    out
}

/// Render the restored `weight` prose: the production-first share list, the
/// component shares and the labelled non-production band. The designed weight
/// chart follows immediately after this block.
#[must_use]
pub fn render_weight(view: &AtlasView) -> String {
    let mut out = String::new();
    out.push_str("<div class=\"atlas-block\" data-atlas-block=\"weight\" id=\"atlas-weight\">\n");
    out.push_str("<h3>Weight — the share in words</h3>\n");
    out.push_str(
        "<p class=\"atlas-caption\">Production files first; non-production cost is counted in the labelled band, never dropped.</p>\n",
    );
    let total = view.totals.total_cost;
    let production_cost = view
        .files
        .iter()
        .filter(|file| file.production)
        .map(|file| file.cost)
        .fold(0_u64, u64::saturating_add);
    let nonproduction_cost = total.saturating_sub(production_cost);
    let band = format_nonproduction(nonproduction_cost, total, view.coverage.demoted_files);
    // Layout §5: an all-test strip leads with the non-production band.
    if view.state.all_test {
        out.push_str(&format!(
            "<p class=\"atlas-nonprod\">{}</p>\n",
            escape_html(&band)
        ));
    }
    if total == 0 {
        out.push_str(&format!("<p>{NO_SCANNED_COST}</p>\n"));
    } else {
        let production: Vec<&AtlasFile> = view
            .files
            .iter()
            .filter(|file| file.production)
            .take(5)
            .collect();
        if production.is_empty() {
            out.push_str(&format!("<p>{NO_PRODUCTION_COST}</p>\n"));
        } else {
            out.push_str("<ol class=\"atlas-weight-list\">\n");
            for file in production {
                out.push_str(&format!(
                    "<li><span class=\"atlas-path\">{}</span> <span class=\"atlas-share\">{}</span></li>\n",
                    escape_html(&file.path),
                    escape_html(&format_share(file.cost, total)),
                ));
            }
            out.push_str("</ol>\n");
        }
        let production_components: Vec<_> = view
            .components
            .iter()
            .filter(|component| component.production_file_count > 0)
            .take(3)
            .collect();
        if production_components.is_empty() {
            out.push_str("<p>no production component recorded</p>\n");
        } else {
            out.push_str("<ul class=\"atlas-weight-list\">\n");
            for component in production_components {
                out.push_str(&format!(
                    "<li><span class=\"atlas-path\">{}</span> <span class=\"atlas-share\">{}</span></li>\n",
                    escape_html(&component.prefix),
                    escape_html(&format_component_share(
                        component.cost,
                        total,
                        component.file_count
                    )),
                ));
            }
            out.push_str("</ul>\n");
        }
        out.push_str(
            "<p class=\"atlas-caption\">Component shares include every member file (including non-production members); component groups are approximate prefix groups.</p>\n",
        );
    }
    if !view.state.all_test {
        out.push_str(&format!(
            "<p class=\"atlas-nonprod\">{}</p>\n",
            escape_html(&band)
        ));
    }
    out.push_str("</div>\n");
    out
}

/// Render the restored `blast` prose: dependents (M-08 verbatim), structural
/// call degree, include coverage and the top dependent files. The designed
/// blast chart follows immediately after this block; the complete drill lives
/// in `atlas-q2.html` (no inline drill section).
#[must_use]
pub fn render_blast(view: &AtlasView) -> String {
    let mut out = String::new();
    out.push_str("<div class=\"atlas-block\" data-atlas-block=\"blast\" id=\"atlas-blast\">\n");
    out.push_str("<h3>Blast radius — the dependents in words</h3>\n");
    let blast = &view.blast_radius;
    match &blast.target {
        Some(target) => {
            out.push_str(&format!(
                "<p class=\"atlas-blast-line\">{} · target <span class=\"atlas-path\">{}</span></p>\n",
                escape_html(&format_dependents(
                    blast.dependents,
                    view.reachability.source_files,
                    blast.dependent_multiple,
                    // M-08 contract: the brief prints the view's percentile
                    // verbatim instead of recomputing it.
                    blast.dependent_top_pct,
                )),
                escape_html(target),
            ));
            out.push_str(&format!(
                "<p>{}</p>\n",
                escape_html(&format_call_degree(blast.caller_files, blast.callee_files))
            ));
            out.push_str(&format!(
                "<p class=\"atlas-caption\">{} of {} include edges resolved · {}</p>\n",
                format_count(blast.include_edges_resolved as u64),
                format_count(blast.include_edges_total as u64),
                escape_html(&format_unresolved_includes(
                    view.coverage.unresolved_includes,
                    blast.include_edges_total,
                )),
            ));
            if !blast.top_dependents.is_empty() {
                out.push_str("<ul class=\"atlas-dependent-list\">\n");
                for (path, edges) in &blast.top_dependents {
                    out.push_str(&format!(
                        "<li><span class=\"atlas-path\">{}</span> · {} include edges</li>\n",
                        escape_html(path),
                        format_count(*edges as u64),
                    ));
                }
                out.push_str("</ul>\n");
            }
            out.push_str(
                "<p><a href=\"atlas-q2.html\">Full dependent list and Q2 reverse drill</a></p>\n",
            );
            out.push_str(
                "<p class=\"atlas-caption\">Callers and callees are structural; see the Atlas legend for edge confidence.</p>\n",
            );
        }
        None => {
            out.push_str("<p>no entry — blast radius unavailable</p>\n");
            out.push_str(&format!(
                "<p class=\"atlas-caption\">{}</p>\n",
                escape_html(&format_unresolved_includes(
                    view.coverage.unresolved_includes,
                    blast.include_edges_total,
                ))
            ));
        }
    }
    out.push_str("</div>\n");
    out
}

/// File cards: production-first top files; a capped canvas shows only the
/// drawn set (layout §5) while an uncapped scan cards the cost-ranked set.
fn card_files(view: &AtlasView) -> Vec<&AtlasFile> {
    if view.state.capped {
        let drawn: BTreeSet<&str> = view.nodes.iter().map(|node| node.path.as_str()).collect();
        view.files
            .iter()
            .filter(|file| drawn.contains(file.path.as_str()))
            .take(6)
            .collect()
    } else {
        view.files.iter().take(6).collect()
    }
}

/// Render the restored `cards` block: per-file identity, cost/share, every
/// `top_functions` row and the dominant finding. The rail stays as the summary.
#[must_use]
pub fn render_cards(view: &AtlasView, doc: &MetricsDocument) -> String {
    let file_anchors = file_anchor_map(doc);
    let mut out = String::new();
    out.push_str("<div class=\"atlas-block\" data-atlas-block=\"cards\" id=\"atlas-cards\">\n");
    out.push_str("<h3>File cards — what is inside the top files</h3>\n");
    let cards = card_files(view);
    if cards.is_empty() {
        out.push_str(&format!("<p>{NO_SOURCE_FUNCTIONS}</p>\n"));
    } else {
        out.push_str("<div class=\"atlas-cards\">\n");
        for file in cards {
            out.push_str("<article class=\"atlas-card\">\n<h4>");
            out.push_str(&format!(
                "<span class=\"atlas-path\">{}</span> <span class=\"atlas-class{}\">{}</span>",
                escape_html(&file.path),
                if file.production { "" } else { " nonprod" },
                escape_html(&file.path_class),
            ));
            out.push_str("</h4>\n");
            out.push_str(&format!(
                "<p class=\"atlas-anchor\">{}</p>\n",
                escape_html(&file_cost_anchor(file))
            ));
            out.push_str(&format!(
                "<p class=\"atlas-share\">{}</p>\n",
                escape_html(&format_share(file.cost, view.totals.total_cost))
            ));
            match &file.finding {
                Some(finding) => out.push_str(&format!(
                    "<p class=\"atlas-dominant\">Dominant finding: <code>{}</code> {} — {}</p>\n",
                    escape_html(&finding.rule_id),
                    escape_html(&format_finding(finding.severity, finding.count)),
                    escape_html(&sanitize_atlas_text(&finding.title)),
                )),
                None => out.push_str("<p class=\"atlas-dominant\">no findings on this file</p>\n"),
            }
            if file.top_functions.is_empty() {
                out.push_str("<p>no functions recorded</p>\n");
            } else {
                out.push_str("<ol class=\"atlas-fn-list\">\n");
                for function in &file.top_functions {
                    out.push_str(&format!(
                        "<li><code>{}:{}</code> <span class=\"atlas-fn\">{}</span> <span class=\"atlas-anchor\">{}</span></li>\n",
                        escape_html(&file.path),
                        function.line,
                        escape_html(&function_display_name(&function.name)),
                        escape_html(&function_anchor_for(function)),
                    ));
                }
                out.push_str("</ol>\n");
            }
            let anchor = file_link_anchor(&file_anchors, &file.path);
            out.push_str(&format!(
                "<p><a href=\"atlas-weight.html#atlas-{anchor}\">Weight detail for this file</a> · <a href=\"findings-detail.html#ranked-actions\">complete action list</a></p>\n",
                anchor = escape_html(&anchor),
            ));
            out.push_str("</article>\n");
        }
        out.push_str("</div>\n");
        out.push_str(&format!(
            "<p class=\"atlas-counts\">showing top {} of {} source files (production-first)</p>\n",
            card_files(view).len(),
            format_count(view.files.len() as u64),
        ));
    }
    out.push_str("</div>\n");
    out
}

/// Render the restored `noise` block: parse health, include coverage, edge
/// confidence mix, caps/demotion, unreached counts, and the explicit
/// `files_without_functions` / `partial_parse_files` disclosure.
#[must_use]
pub fn render_noise(view: &AtlasView) -> String {
    let mut out = String::new();
    out.push_str("<div class=\"atlas-block\" data-atlas-block=\"noise\" id=\"atlas-noise\">\n");
    out.push_str("<h3>Noise and confidence</h3>\n");
    let coverage = &view.coverage;
    out.push_str(&format!(
        "<p>{}</p>\n",
        escape_html(&format_parse(
            coverage.parse_ok,
            coverage.parse_partial,
            coverage.parse_failed,
        ))
    ));
    out.push_str(&format!(
        "<p>{} · {} of {} include edges resolved</p>\n",
        escape_html(&format_unresolved_includes(
            coverage.unresolved_includes,
            view.blast_radius.include_edges_total,
        )),
        format_count(view.blast_radius.include_edges_resolved as u64),
        format_count(view.blast_radius.include_edges_total as u64),
    ));
    out.push_str(&format!(
        "<p>{}</p>\n",
        escape_html(&format_edge_mix(
            view.edge_mix.medium,
            view.edge_mix.low,
            view.edge_mix.high,
        ))
    ));
    out.push_str(&format!(
        "<p>{} · {}</p>\n",
        escape_html(&format_demoted_files(coverage.demoted_files)),
        escape_html(&format_caps(
            view.caps.reachable_shown,
            view.caps.reachable_files,
            view.caps.aggregated_files,
            view.caps.edges_drawn,
            view.caps.edge_pairs_total,
        )),
    ));
    out.push_str(&format!(
        "<p>{} production + {} non-production of {} source files unreached</p>\n",
        format_count(coverage.unreached_production as u64),
        format_count(coverage.unreached_nonproduction as u64),
        format_count(view.reachability.source_files as u64),
    ));
    out.push_str(&format!(
        "<p>{} of {} scanned files have no functions · {} source files partial or failed</p>\n",
        format_count(coverage.files_without_functions as u64),
        format_count(view.totals.file_count as u64),
        format_count(coverage.partial_parse_files as u64),
    ));
    out.push_str(
        "<p class=\"atlas-caption\">Edge confidence and the cost unit are defined once, in the Atlas legend; static cost is not ground truth.</p>\n",
    );
    out.push_str("</div>\n");
    out
}

/// Render the restored `links` block last: the complete lists, the two
/// standalone drill pages, the emitted component pages and the Explorer.
#[must_use]
pub fn render_links(view: &AtlasView, doc: &MetricsDocument, emitted_charts: &[String]) -> String {
    let components = component_pages(doc, emitted_charts);
    let total_actions = views::kv1(doc).total_actions;
    let mut out = String::new();
    out.push_str("<div class=\"atlas-block\" data-atlas-block=\"links\" id=\"atlas-links\">\n");
    out.push_str("<h3>Drills</h3>\n<ul class=\"atlas-drill-links\">\n");
    out.push_str(&format!(
        "<li><a href=\"findings-detail.html#ranked-actions\">Complete ranked action list</a> — {} (production-first)</li>\n",
        escape_html(&views::plural(total_actions, "action")),
    ));
    out.push_str(&format!(
        "<li><a href=\"profile-queue.html\">Complete profile queue</a> — {} ranked by static cost</li>\n",
        escape_html(&views::plural(view.totals.function_count, "function")),
    ));
    out.push_str(&format!(
        "<li><a href=\"atlas-q2.html\">Q2 reverse drill</a> — dependents and structural callers/callees for {}</li>\n",
        escape_html(focus_path(view).unwrap_or("no entry")),
    ));
    out.push_str(&format!(
        "<li><a href=\"atlas-weight.html\">Weight detail</a> — {} and {} by share</li>\n",
        escape_html(&views::plural(view.files.len(), "source file")),
        escape_html(&views::plural(view.components.len(), "component")),
    ));
    if components.is_empty() {
        out.push_str("<li>Component pages: no component page emitted</li>\n");
    } else {
        out.push_str("<li>Component pages (schema component index): ");
        for (index, component) in components.iter().enumerate() {
            if index > 0 {
                out.push_str(" · ");
            }
            out.push_str(&format!(
                "<a href=\"{}\">{}</a> (#{})",
                escape_html(&component.href),
                escape_html(&component.prefix),
                component.rank,
            ));
        }
        out.push_str("</li>\n");
    }
    out.push_str("<li><a href=\"#explorer\">Explorer</a> — legacy static chart stems</li>\n");
    out.push_str("</ul>\n</div>\n");
    out
}

/// Render the first-page per-file detail rail (`#atlas-rail`).
///
/// Selection: the elected entry file first, then `view.files` in shipped
/// (production-first) order, deduplicated by path and capped at
/// [`RAIL_FILE_CAP`]. File names link to the weight-drill row; function rows
/// link to their exact profile-queue row and finding rows to their exact
/// findings-detail occurrence, so no link is dead. Emits exactly one `<aside>`
/// in every state, including the empty one (a named stub, no cards).
#[must_use]
pub fn render_rail(view: &AtlasView, doc: &MetricsDocument) -> String {
    let file_anchors = file_anchor_map(doc);
    let function_anchors = function_anchor_map(doc);
    let (_, finding_anchors) = finding_anchor_assignments(doc);
    let unreached: BTreeSet<&str> = view
        .unreachable_production
        .iter()
        .chain(view.unreachable_nonproduction.iter())
        .map(String::as_str)
        .collect();

    let entry_path = view.entry.point.as_ref().map(|point| point.path.as_str());
    let mut selected: Vec<&AtlasFile> = Vec::new();
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    if let Some(path) = entry_path
        && let Some(file) = view.files.iter().find(|file| file.path == path)
    {
        selected.push(file);
        seen.insert(file.path.as_str());
    }
    for file in &view.files {
        if selected.len() >= RAIL_FILE_CAP {
            break;
        }
        if seen.insert(file.path.as_str()) {
            selected.push(file);
        }
    }

    let mut out = String::new();
    out.push_str("<aside id=\"atlas-rail\" class=\"atlas-rail\" aria-label=\"File detail\">\n");
    out.push_str(&format!(
        "<div class=\"atlas-rail-title\">File detail<span>top {} of {} source files · production first</span></div>\n",
        selected.len(),
        format_count(view.files.len() as u64),
    ));
    if selected.is_empty() {
        out.push_str(&format!(
            "<p class=\"atlas-rail-stub\">{NO_SOURCE_FUNCTIONS}</p>\n"
        ));
        out.push_str("</aside>\n");
        return out;
    }

    out.push_str("<div class=\"atlas-rail-cards\">\n");
    for file in &selected {
        let is_entry = entry_path == Some(file.path.as_str());
        let role = rail_role(file, is_entry, unreached.contains(file.path.as_str()));
        let mut classes = String::from("atlas-rail-card");
        if is_entry {
            classes.push_str(" is-entry");
        }
        if unreached.contains(file.path.as_str()) {
            classes.push_str(" is-unreached");
        }
        if file.path_class == "test" {
            classes.push_str(" is-test");
        }
        if file.path_class == "bench" {
            classes.push_str(" is-bench");
        }
        if file.production {
            classes.push_str(" is-production");
        }
        if view.hottest.as_deref() == Some(file.path.as_str()) {
            classes.push_str(" is-hot");
        }

        let file_anchor = file_link_anchor(&file_anchors, &file.path);
        let mut card = String::new();
        card.push_str(&format!("<article class=\"{classes}\">\n"));
        // The rail is the identity layer: show the path (mid-ellipsis, never
        // leading-dot) so same-named files stay distinguishable; the graph
        // node carries the short file name.
        let path_label = mid_ellipsis(&file.path, 28);
        card.push_str(&format!(
            "<div class=\"atlas-rail-head\"><a class=\"atlas-rail-name atlas-mono\" href=\"atlas-weight.html#atlas-{anchor}\" title=\"{path}\">{name}</a><span class=\"atlas-rail-role\">{role}</span></div>\n",
            anchor = escape_html(&file_anchor),
            path = escape_html(&file.path),
            name = escape_html(&path_label),
            role = escape_html(&role),
        ));
        card.push_str(&format!(
            "<div class=\"atlas-rail-anchor\">{}</div>\n",
            escape_html(&file_cost_anchor(file)),
        ));
        if file.top_functions.is_empty() {
            card.push_str("<div class=\"atlas-rail-fn\">no functions recorded</div>\n");
        } else {
            for function in file.top_functions.iter().take(3) {
                let anchor = function_anchors
                    .get(&function.id)
                    .cloned()
                    .unwrap_or_else(|| format!("fn-{}", super::slugify(&function.id)));
                let name = function_display_name(&function.name);
                card.push_str(&format!(
                    "<div class=\"atlas-rail-fn\"><a class=\"atlas-mono\" href=\"profile-queue.html#{anchor}\" title=\"{path}:{line} {name}\">{name}:{line}</a></div>\n",
                    anchor = escape_html(&anchor),
                    path = escape_html(&file.path),
                    line = function.line,
                    name = escape_html(&name),
                ));
            }
        }
        match &file.finding {
            Some(dominant) => {
                // Prefer the occurrence with the same rule AND severity (the
                // dominant finding is severity-ranked); fall back to the first
                // rule occurrence, then the ranked-actions anchor.
                let finding_anchor = dominant_finding_anchor(doc, file, dominant, &finding_anchors);
                card.push_str(&format!(
                    "<div class=\"atlas-rail-find\">FIND <a class=\"atlas-mono\" href=\"findings-detail.html#{anchor}\">{rule}</a> · {finding}</div>\n",
                    anchor = escape_html(&finding_anchor),
                    rule = escape_html(&dominant.rule_id),
                    finding = escape_html(&format_finding(dominant.severity, dominant.count)),
                ));
            }
            None => {
                card.push_str("<div class=\"atlas-rail-find\">no findings on this file</div>\n");
            }
        }
        card.push_str("</article>\n");
        out.push_str(&card);
    }
    out.push_str("</div>\n");
    let hidden = view.files.len().saturating_sub(selected.len());
    if hidden > 0 {
        out.push_str(&format!(
            "<p class=\"atlas-rail-more\"><a href=\"atlas-weight.html\">+{} more {} in the weight detail</a></p>\n",
            format_count(hidden as u64),
            views::plural_word(hidden, "source file", "source files"),
        ));
    }
    out.push_str("</aside>\n");
    out
}

/// Visible function name; anonymous functions (an empty schema name) get a
/// named stub instead of a bare `:line` label.
pub(crate) fn function_display_name(name: &str) -> String {
    if name.trim().is_empty() {
        "(anonymous)".to_string()
    } else {
        name.to_string()
    }
}

/// Rail role label; entry and unreached take precedence over path class.
fn rail_role(file: &AtlasFile, is_entry: bool, unreached: bool) -> String {
    if is_entry {
        return "ENTRY".to_string();
    }
    if unreached {
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

/// Render the complete restored brief: `answer`, the chart grid (with `weight`
/// and `blast` prose immediately before their charts), then `actions`, `cards`,
/// `noise` and `links` (last). Used by the unit/API surface; the report page
/// splits these blocks around the canvas stage so the answer stays in the first
/// scroll. Emits exactly one `#atlas-brief` and no inline drill sections — the
/// drills stay standalone `atlas-q2.html` / `atlas-weight.html` pages.
#[must_use]
pub fn render_brief(view: &AtlasView, doc: &MetricsDocument, emitted_charts: &[String]) -> String {
    let mut out = String::new();
    out.push_str("<section id=\"atlas-brief\" class=\"atlas-brief\">\n");
    out.push_str(&render_answer(view, doc));
    out.push_str(&crate::reporter::atlas_charts::render_chart_grid(view, doc));
    out.push_str(&render_actions(view, doc));
    out.push_str(&render_cards(view, doc));
    out.push_str(&render_noise(view));
    out.push_str(&render_links(view, doc, emitted_charts));
    out.push_str("</section>\n");
    out
}

// ---------------------------------------------------------------------------
// Drills
// ---------------------------------------------------------------------------

fn render_q2_drill(out: &mut String, view: &AtlasView, doc: &MetricsDocument) {
    out.push_str(
        "<section id=\"atlas-drill-q2\" class=\"atlas-drill\" data-atlas-drill=\"q2\">\n\
         <h3>Q2 reverse drill — what breaks if I touch the focus file</h3>\n",
    );
    let Some(focus) = focus_path(view) else {
        out.push_str("<p>no entry or file to drill</p>\n</section>\n");
        return;
    };
    out.push_str(&format!(
        "<p>focus: <code>{}</code> · include coverage: {} of {} include edges resolved</p>\n",
        escape_html(focus),
        format_count(view.blast_radius.include_edges_resolved as u64),
        format_count(view.blast_radius.include_edges_total as u64),
    ));
    let reverse = include_reverse(doc, focus);
    out.push_str("<h4>1-hop include dependents</h4>\n");
    if reverse.dependents.is_empty() {
        out.push_str(&format!("<p>{NO_RESOLVED_DEPENDENTS}</p>\n"));
    } else {
        out.push_str("<div class=\"atlas-table-scroll\"><table>\n<thead><tr><th>dependent file</th><th>include edges into focus</th></tr></thead>\n<tbody>\n");
        for (path, count) in &reverse.dependents {
            out.push_str(&format!(
                "<tr><td><span class=\"atlas-path\">{}</span></td><td>{} include edges</td></tr>\n",
                escape_html(path),
                format_count(*count as u64),
            ));
        }
        out.push_str("</tbody>\n</table>\n</div>\n");
    }
    out.push_str("<h4>Component DSM — approximate prefix groups (resolved include edges)</h4>\n");
    let mut dsm: Vec<(&(String, String), &usize)> = reverse.dsm.iter().collect();
    dsm.sort_by(|a, b| {
        b.1.cmp(a.1)
            .then_with(|| a.0.0.cmp(&b.0.0))
            .then_with(|| a.0.1.cmp(&b.0.1))
    });
    if dsm.is_empty() {
        out.push_str(&format!("<p>{NO_INCLUDE_EDGES}</p>\n"));
    } else {
        out.push_str("<div class=\"atlas-table-scroll\"><table>\n<thead><tr><th>from component</th><th>to component</th><th>include edges</th></tr></thead>\n<tbody>\n");
        for ((from, to), count) in dsm {
            out.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                escape_html(from),
                escape_html(to),
                format_count(*count as u64),
            ));
        }
        out.push_str("</tbody>\n</table>\n</div>\n");
    }
    out.push_str("<h4>Structural callers and callees</h4>\n");
    let call = call_degree(doc, focus);
    out.push_str(&format!(
        "<p>{}</p>\n",
        escape_html(&format_call_degree(call.callers.len(), call.callees.len()))
    ));
    out.push_str(&format!(
        "<p>caller files: {}</p>\n",
        if call.callers.is_empty() {
            "no structural call edges recorded".to_string()
        } else {
            call.callers
                .iter()
                .map(|path| format!("<span class=\"atlas-path\">{}</span>", escape_html(path)))
                .collect::<Vec<_>>()
                .join(" · ")
        }
    ));
    out.push_str(&format!(
        "<p>callee files: {}</p>\n",
        if call.callees.is_empty() {
            "no structural call edges recorded".to_string()
        } else {
            call.callees
                .iter()
                .map(|path| format!("<span class=\"atlas-path\">{}</span>", escape_html(path)))
                .collect::<Vec<_>>()
                .join(" · ")
        }
    ));
    out.push_str(
        "<p class=\"atlas-caption\">Callers and callees are structural; see the Atlas legend for edge confidence.</p>\n</section>\n",
    );
}

fn render_weight_drill(
    out: &mut String,
    view: &AtlasView,
    file_anchors: &BTreeMap<String, String>,
) {
    out.push_str(
        "<section id=\"atlas-drill-weight\" class=\"atlas-drill\" data-atlas-drill=\"weight\">\n\
         <h3>Weight detail — every source file and component by share</h3>\n",
    );
    let total = view.totals.total_cost;
    if view.files.is_empty() {
        out.push_str(&format!("<p>{NO_SOURCE_FUNCTIONS}</p>\n</section>\n"));
        return;
    }
    out.push_str(&format!(
        "<p class=\"atlas-caption\">{} source files · every share is relative to total scanned static cost.</p>\n",
        format_count(view.files.len() as u64),
    ));
    out.push_str("<div class=\"atlas-table-scroll\"><table>\n<thead><tr><th>file</th><th>class</th><th>cost</th><th>share</th></tr></thead>\n<tbody>\n");
    for file in &view.files {
        let anchor = file_link_anchor(file_anchors, &file.path);
        out.push_str(&format!(
            "<tr id=\"atlas-{anchor}\"><td><span class=\"atlas-path\">{}</span></td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            escape_html(&file.path),
            escape_html(&file.path_class),
            escape_html(&file_cost_anchor(file)),
            escape_html(&format_share(file.cost, total)),
            anchor = escape_html(&anchor),
        ));
    }
    out.push_str("</tbody>\n</table>\n</div>\n");
    out.push_str("<h4>Components</h4>\n");
    if view.components.is_empty() {
        out.push_str("<p>no component recorded</p>\n");
    } else {
        out.push_str("<div class=\"atlas-table-scroll\"><table>\n<thead><tr><th>component</th><th>files</th><th>production files</th><th>share</th></tr></thead>\n<tbody>\n");
        for component in &view.components {
            out.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                escape_html(&component.prefix),
                format_count(component.file_count as u64),
                format_count(component.production_file_count as u64),
                escape_html(&format_component_share(
                    component.cost,
                    total,
                    component.file_count
                )),
            ));
        }
        out.push_str("</tbody>\n</table>\n</div>\n");
    }
    out.push_str(
        "<p class=\"atlas-caption\">Component rows are approximate prefix groups; each share includes every member file, including non-production members.</p>\n",
    );
    out.push_str(
        "<p><a href=\"report.html#atlas-story\">Atlas sim — the inferred run</a> (the deferred treemap would live here).</p>\n",
    );
    out.push_str("</section>\n");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Per-scale function anchors (M-05): unit + `× median function` + `top P%`.
pub(crate) struct FunctionAnchors {
    v2: ScaleOrdering,
    v1: ScaleOrdering,
}

impl FunctionAnchors {
    pub(crate) fn new(doc: &MetricsDocument) -> Self {
        Self {
            v2: ScaleOrdering::build(&doc.stats.functions, CostScale::V2),
            v1: ScaleOrdering::build(&doc.stats.functions, CostScale::V1),
        }
    }

    pub(crate) fn format(&self, function: &FunctionStats) -> String {
        let scale = scale_of(function);
        let ordering = match scale {
            CostScale::V2 => &self.v2,
            CostScale::V1 => &self.v1,
        };
        let cost = function_cost_u64(function);
        format_function_cost(
            cost,
            scale,
            positive_median(cost, ordering.median()),
            ordering.rank(&function.id).unwrap_or(1),
            ordering.total(),
        )
    }
}

/// A zero-cost row must not print `0×` (metrics spec §5 missing values).
fn positive_median(cost: u64, median: Option<f64>) -> Option<f64> {
    if cost == 0 { None } else { median }
}

pub(crate) fn file_cost_anchor(file: &AtlasFile) -> String {
    format_file_cost(
        file.cost,
        file.scale,
        positive_median(file.cost, file.median),
        file.file_rank,
        file.file_population,
    )
}

/// Rendered cost anchor (M-05) for one `top_functions` row.
fn function_anchor_for(function: &AtlasFunction) -> String {
    format_function_cost(
        function.cost,
        function.scale,
        positive_median(function.cost, function.median),
        function.rank,
        function.population,
    )
}

/// The file carrying the largest static cost (the hottest file), when drawn.
fn hottest_file(view: &AtlasView) -> Option<&AtlasFile> {
    let path = view.hottest.as_deref()?;
    view.files.iter().find(|file| file.path == path)
}

/// Why the entry was elected (M-01 policy wording, verbatim per policy).
fn entry_why(policy: EntryPolicy) -> &'static str {
    match policy {
        EntryPolicy::ProductionMain => "Elected because the scan found a production main.",
        EntryPolicy::ProductionScript => {
            "Elected because the scan found a production script entry (Python __main__)."
        }
        EntryPolicy::LibraryRoot => {
            "Elected as the highest fan-in production library root; this scan has no production main."
        }
        EntryPolicy::NoEntry => {
            "No production main, script, or library root was found, so no entry is shown."
        }
    }
}

/// One structural caveat, chosen deterministically from the scan state.
fn caveat(view: &AtlasView) -> String {
    let coverage = &view.coverage;
    if view.state.empty {
        return "no source functions recorded — every block shows a named stub.".to_string();
    }
    if coverage.parse_failed > 0 {
        return format!(
            "{} files failed to parse, so their cost and edges are missing.",
            format_count(coverage.parse_failed as u64)
        );
    }
    if coverage.parse_partial > 0 {
        return format!(
            "{} files parsed partially, so their cost and edges are under-counted.",
            format_count(coverage.parse_partial as u64)
        );
    }
    if view.state.all_test {
        return "all source cost is non-production (test/bench/other); production rankings are empty."
            .to_string();
    }
    if view.state.library {
        // A library root may call helpers; only claim "no forward path" when
        // the canvas truly has no forward edge (every drawn edge is a back edge).
        if view.edges.iter().all(|edge| edge.back) {
            return "there is no forward path from the library root; callers are listed in the Q2 drill."
                .to_string();
        }
        return "the library root has forward calls; callers are listed in the Q2 drill."
            .to_string();
    }
    if view.state.no_entry {
        return "no entry was elected; the cost ranking and the action list still stand."
            .to_string();
    }
    if view.state.capped {
        return format!(
            "{} reachable files are aggregated, so the canvas shows a capped set.",
            format_count(view.caps.aggregated_files as u64)
        );
    }
    "the drawn edges are structural; see the Atlas legend for units and edge confidence."
        .to_string()
}

/// Named stub for the no-action case; component aggregates are named honestly.
fn no_action_text(kv1: &views::Kv1View) -> String {
    if kv1.component_findings > 0 {
        format!(
            "No function-level action was recorded; {} component-level findings are in the full list.",
            format_count(kv1.component_findings as u64)
        )
    } else {
        NO_FINDINGS.to_string()
    }
}

/// Deterministic weight-drill anchor for a file, shared by the rail and the
/// file chart so both link to the exact same row.
pub(crate) fn file_link_anchor(anchors: &BTreeMap<String, String>, path: &str) -> String {
    anchors
        .get(path)
        .cloned()
        .unwrap_or_else(|| format!("file-{}", super::slugify(path)))
}

/// Deterministic profile-queue anchor for a function, shared by the rail and
/// the file chart.
pub(crate) fn function_link_anchor(anchors: &BTreeMap<String, String>, id: &str) -> String {
    anchors
        .get(id)
        .cloned()
        .unwrap_or_else(|| format!("fn-{}", super::slugify(id)))
}

/// Dominant-finding link target: prefer the occurrence with the same rule AND
/// severity, fall back to the first rule occurrence, then the ranked-actions
/// anchor. Shared by the rail and the file chart.
pub(crate) fn dominant_finding_anchor(
    doc: &MetricsDocument,
    file: &AtlasFile,
    dominant: &AtlasFinding,
    finding_anchors: &[String],
) -> String {
    doc.findings
        .iter()
        .position(|finding| {
            finding.path == file.path
                && finding.rule_id == dominant.rule_id
                && finding.severity == dominant.severity
        })
        .or_else(|| {
            doc.findings.iter().position(|finding| {
                finding.path == file.path && finding.rule_id == dominant.rule_id
            })
        })
        .map(|index| finding_anchors[index].clone())
        .unwrap_or_else(|| "ranked-actions".to_string())
}

fn focus_path(view: &AtlasView) -> Option<&str> {
    view.entry
        .point
        .as_ref()
        .map(|point| point.path.as_str())
        .or(view.hottest.as_deref())
}

fn location(finding: &FindingEntry) -> String {
    format!("{}:{}", finding.path, finding.span.start_line)
}

/// Top schema components resolved to emitted chart pages. The link is the
/// emitted `charts/component-<rank>-<slug>.svg` when present, otherwise the
/// component-index row anchor, otherwise the index section — never a dead link.
fn component_pages(doc: &MetricsDocument, emitted_charts: &[String]) -> Vec<ComponentPage> {
    let emitted: BTreeSet<&str> = emitted_charts
        .iter()
        .map(|name| name.strip_suffix(".svg").unwrap_or(name))
        .collect();
    let mut sorted: Vec<&crate::schema::ComponentEntry> = doc.components.iter().flatten().collect();
    sorted.sort_by(|a, b| {
        a.rank
            .cmp(&b.rank)
            .then_with(|| a.path_prefix.cmp(&b.path_prefix))
    });
    let rendered: BTreeSet<&str> = sorted
        .iter()
        .take(10)
        .map(|entry| entry.path_prefix.as_str())
        .collect();
    sorted
        .into_iter()
        .take(3)
        .map(|entry| {
            let filename =
                crate::charts::component::component_filename(entry.rank, &entry.path_prefix);
            let stem = filename.strip_suffix(".svg").unwrap_or(&filename);
            let href = if emitted.contains(stem) {
                format!("charts/{filename}")
            } else if rendered.contains(entry.path_prefix.as_str()) {
                // The component index row carries this exact id.
                format!("#{stem}")
            } else {
                "#components".to_string()
            };
            ComponentPage {
                prefix: if entry.path_prefix == "." {
                    "repository root (.)".to_string()
                } else {
                    entry.path_prefix.clone()
                },
                rank: entry.rank,
                href,
            }
        })
        .collect()
}

struct IncludeReverse {
    dependents: Vec<(String, usize)>,
    dsm: BTreeMap<(String, String), usize>,
}

fn include_reverse(doc: &MetricsDocument, focus: &str) -> IncludeReverse {
    // The DSM covers every scanned file (headers included): a `.c -> .h`
    // include is real coupling and must show up in the reverse view. The
    // focus file is a source file by construction; edges into it from any
    // scanned file count as dependents.
    let scanned: BTreeSet<&str> = doc.files.iter().map(|file| file.path.as_str()).collect();
    let mut dependents: BTreeMap<String, usize> = BTreeMap::new();
    let mut dsm: BTreeMap<(String, String), usize> = BTreeMap::new();
    if let Some(graphs) = &doc.graphs {
        for edge in &graphs.include_graph.edges {
            if !edge.resolved {
                continue;
            }
            let Some(to) = edge.to.as_deref() else {
                continue;
            };
            if edge.from == to || !scanned.contains(edge.from.as_str()) || !scanned.contains(to) {
                continue;
            }
            *dsm.entry((
                crate::reporter::atlas::domain_of(&edge.from),
                crate::reporter::atlas::domain_of(to),
            ))
            .or_insert(0) += 1;
            if to == focus {
                *dependents.entry(edge.from.clone()).or_insert(0) += 1;
            }
        }
    }
    let mut dependents: Vec<(String, usize)> = dependents.into_iter().collect();
    dependents.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    IncludeReverse { dependents, dsm }
}

struct CallDegree {
    callers: Vec<String>,
    callees: Vec<String>,
}

fn call_degree(doc: &MetricsDocument, focus: &str) -> CallDegree {
    let source: BTreeSet<&str> = doc
        .files
        .iter()
        .filter(|file| file.function_count > 0)
        .map(|file| file.path.as_str())
        .collect();
    let mut callers: BTreeSet<String> = BTreeSet::new();
    let mut callees: BTreeSet<String> = BTreeSet::new();
    if let Some(graphs) = &doc.graphs
        && let Some(call_graph) = &graphs.call_graph
    {
        for edge in &call_graph.edges {
            let Some(to) = edge.to_path.as_deref() else {
                continue;
            };
            if edge.from_path == to
                || !source.contains(edge.from_path.as_str())
                || !source.contains(to)
            {
                continue;
            }
            if to == focus {
                callers.insert(edge.from_path.clone());
            }
            if edge.from_path == focus {
                callees.insert(to.to_string());
            }
        }
    }
    CallDegree {
        callers: callers.into_iter().collect(),
        callees: callees.into_iter().collect(),
    }
}

/// Rule messages and titles are analyzer-authored and can contain words the
/// Atlas contract forbids as claims (`runtime`, `hot paths`, …). The Atlas
/// surface substitutes neutral static wording for those whole words so the
/// emitted page never makes a time claim. Identifiers are preserved: `_` is a
/// word character, so `calc_ms` / `runtime_cost` are left untouched.
pub(crate) fn sanitize_atlas_text(message: &str) -> String {
    let mut text = message.to_string();
    for (needle, replacement) in [
        ("hot paths", "hot functions"),
        ("hot-paths", "hot-functions"),
        ("hot path", "hot function"),
        ("hot-path", "hot-function"),
        ("runtimes", "static costs"),
        ("runtime", "static cost"),
        ("latencies", "static costs"),
        ("latency", "static cost"),
        ("throughputs", "static costs"),
        ("throughput", "static cost"),
        ("speedups", "static improvements"),
        ("speedup", "static improvement"),
        ("ms", "static cost units"),
        ("µs", "static cost units"),
    ] {
        text = replace_whole_word(&text, needle, replacement);
    }
    text.replace("static cost cost", "static cost")
}

fn replace_whole_word(text: &str, needle: &str, replacement: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0;
    let mut search = 0;
    while let Some(offset) = lower[search..].find(needle) {
        let start = search + offset;
        let end = start + needle.len();
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after_ok = end == bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            out.push_str(&text[cursor..start]);
            out.push_str(replacement);
            cursor = end;
            search = end;
        } else {
            search = start + 1;
        }
    }
    out.push_str(&text[cursor..]);
    out
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

// ---------------------------------------------------------------------------
// Standalone drill pages (weight detail / Q2 reverse)
// ---------------------------------------------------------------------------

/// Standalone `atlas-weight.html`: the complete weight detail with a warm
/// Atlas shell and a breadcrumb back to `report.html#atlas-story`.
#[must_use]
pub fn render_atlas_weight_html(doc: &MetricsDocument) -> String {
    let view = crate::reporter::atlas::build_atlas_view(doc);
    let mut body = String::new();
    render_weight_drill(&mut body, &view, &file_anchor_map(doc));
    drill_page("Perfpal Atlas — weight detail", "Weight detail", &body)
}

/// Standalone `atlas-q2.html`: the Q2 reverse drill (dependents, DSM,
/// structural callers/callees) with the same warm Atlas shell.
#[must_use]
pub fn render_atlas_q2_html(doc: &MetricsDocument) -> String {
    let view = crate::reporter::atlas::build_atlas_view(doc);
    let mut body = String::new();
    render_q2_drill(&mut body, &view, doc);
    drill_page("Perfpal Atlas — Q2 reverse drill", "Blast drill", &body)
}

fn drill_page(title: &str, crumb: &str, body: &str) -> String {
    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    out.push_str(&format!("<title>{}</title>\n", escape_html(title)));
    out.push_str("<style>");
    out.push_str(&drill_page_css());
    out.push_str("</style>\n</head>\n<body>\n");
    out.push_str(&format!(
        "<header class=\"atlas-drill-header\"><h1>Perfpal Atlas</h1><h2>{}</h2></header>\n",
        escape_html(crumb)
    ));
    out.push_str("<nav class=\"atlas-breadcrumb\" aria-label=\"Breadcrumb\">");
    out.push_str("<a href=\"report.html#atlas-story\">Atlas sim</a>");
    out.push_str("<span aria-hidden=\"true\"> / </span>");
    out.push_str(&format!("<span>{}</span></nav>\n", escape_html(crumb)));
    out.push_str("<main class=\"atlas-drill-page\">\n");
    out.push_str(body);
    out.push_str("</main>\n</body>\n</html>\n");
    out
}

// ---------------------------------------------------------------------------
// Drill CSS (warm Atlas tokens; deterministic, print-safe; no script/network)
// ---------------------------------------------------------------------------

/// Warm Atlas palette applied to the standalone drill pages. Every color
/// comes from the shared palette constants, so the warm paper look is
/// defined exactly once (report-family shell + drill pages).
fn drill_page_css() -> String {
    format!(
        "*{{box-sizing:border-box}}\
body{{margin:0;background:{paper};color:{ink};font-family:ui-sans-serif,system-ui,-apple-system,\"Segoe UI\",Roboto,Helvetica,Arial,sans-serif;font-size:10px;line-height:1.45}}\
a{{color:{accent}}}\
.atlas-breadcrumb{{padding:10px 16px;border-bottom:1px solid {rule};font-size:9px;font-weight:650;color:{muted}}}\
.atlas-breadcrumb a{{color:{ink};text-decoration:none}}\
.atlas-breadcrumb a:hover{{text-decoration:underline}}\
.atlas-drill-header{{max-width:72rem;margin:0 auto;padding:10px 16px 0}}\
.atlas-drill-header h1{{font-size:1.6rem;font-weight:800;margin:0 0 4px;color:{ink}}}\n\
.atlas-drill-header h2{{font-size:1.15rem;font-weight:800;margin:0 0 8px;padding-top:6px;border-top:1px solid {rule};color:{ink}}}\n\
.atlas-drill-page{{max-width:72rem;margin:0 auto;padding:12px 16px 28px}}\
.atlas-drill{{border-top:1px solid {rule};padding:10px 0 4px}}\
.atlas-drill h3{{font-size:14px;font-weight:800;margin:0 0 6px}}\
.atlas-drill h4{{font-size:10px;font-weight:800;letter-spacing:.08em;text-transform:uppercase;color:{muted};margin:12px 0 4px}}\
.atlas-mono,.atlas-path,.atlas-drill code{{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}}\
.atlas-caption{{color:{muted};font-size:9px}}\
.atlas-table-scroll{{overflow-x:auto;max-width:100%}}\
.atlas-drill table{{border-collapse:collapse;width:100%;margin:6px 0;font-size:9px}}\
.atlas-drill th,.atlas-drill td{{border:1px solid {rule};padding:3px 6px;text-align:left;vertical-align:top}}\
.atlas-drill th{{background:{head};font-size:9px;font-weight:800}}\
.atlas-drill tbody tr:nth-child(even){{background:{zebra}}}\
.atlas-drill p{{margin:4px 0}}\
",
        paper = PAPER,
        ink = INK,
        accent = ACCENT,
        rule = RULE,
        muted = MUTED,
        head = HEAD,
        zebra = ZEBRA,
    )
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{Confidence, Severity};
    use crate::reporter::atlas::build_atlas_view;
    use crate::reporter::atlas::test_support::*;
    use crate::reporter::atlas_metrics::{NO_PRODUCTION_COST, NO_SCANNED_COST, format_dependents};

    fn render(doc: &MetricsDocument) -> String {
        render_brief(&build_atlas_view(doc), doc, &[])
    }

    fn contains_word(haystack: &str, needle: &str) -> bool {
        let haystack = haystack.to_ascii_lowercase();
        let bytes = haystack.as_bytes();
        let mut start = 0;
        while let Some(offset) = haystack[start..].find(needle) {
            let index = start + offset;
            let before = index.checked_sub(1).map(|i| bytes[i]);
            let after = bytes.get(index + needle.len()).copied();
            let boundary = |byte: Option<u8>| {
                byte.is_none_or(|value| !value.is_ascii_alphanumeric() && value != b'_')
            };
            if boundary(before) && boundary(after) {
                return true;
            }
            start = index + needle.len().max(1);
            if start >= haystack.len() {
                break;
            }
        }
        false
    }

    fn has_event_handler(text: &str) -> bool {
        let bytes = text.as_bytes();
        let mut index = 0;
        while index + 2 < bytes.len() {
            if bytes[index] == b'o' && bytes[index + 1] == b'n' {
                let previous_ok = index == 0
                    || matches!(
                        bytes[index - 1],
                        b' ' | b'\n' | b'\t' | b'\r' | b'<' | b'"' | b'\''
                    );
                let mut cursor = index + 2;
                while cursor < bytes.len() && bytes[cursor].is_ascii_lowercase() {
                    cursor += 1;
                }
                if previous_ok
                    && cursor > index + 2
                    && cursor < bytes.len()
                    && bytes[cursor] == b'='
                {
                    return true;
                }
            }
            index += 1;
        }
        false
    }

    fn block_positions(html: &str) -> Vec<&str> {
        let mut positions = Vec::new();
        let mut rest = html;
        while let Some(index) = rest.find("data-atlas-block=\"") {
            let tail = &rest[index + "data-atlas-block=\"".len()..];
            let end = tail.find('"').expect("block marker close");
            positions.push(&tail[..end]);
            rest = &tail[end..];
        }
        positions
    }

    const BLOCKS: [&str; 11] = [
        "answer",
        "weight",
        "weight-chart",
        "blast",
        "blast-chart",
        "file-chart",
        "coverage",
        "actions",
        "cards",
        "noise",
        "links",
    ];

    #[test]
    fn every_state_renders_the_same_chart_stack() {
        for doc in [
            application_fixture(),
            library_fixture(),
            no_entry_fixture(),
            empty_fixture(),
            all_test_fixture(),
            capped_fixture(),
            partial_parse_fixture(),
            single_file_fixture(),
        ] {
            let html = render(&doc);
            assert_eq!(block_positions(&html), BLOCKS);
            assert_eq!(html.matches("id=\"atlas-charts\"").count(), 1);
            assert!(html.contains("id=\"atlas-answer\""));
            assert!(!html.contains("id=\"atlas-drill-"));
            assert!(!html.contains("<table"));
        }
    }

    #[test]
    fn empty_state_uses_named_stubs_in_every_chart() {
        let html = render(&empty_fixture());
        for stub in [
            NO_SOURCE_FUNCTIONS,
            NO_SCANNED_COST,
            "no entry — blast radius unavailable",
        ] {
            assert!(html.contains(stub), "missing stub {stub}");
        }
        let all_test = render(&all_test_fixture());
        assert!(all_test.contains(NO_PRODUCTION_COST));
    }

    #[test]
    fn weight_chart_is_production_first_with_labelled_nonproduction_band() {
        let doc = DocBuilder::new()
            .file(file("src/a.c", "production", 1))
            .file(file("tests/t.c", "test", 1))
            .function(function("src/a.c", "work", 1, Some(100), 0, None))
            .function(function("tests/t.c", "test_work", 1, Some(900), 0, None))
            .build();
        let html = render(&doc);
        let weight = html
            .split("data-atlas-block=\"weight-chart\"")
            .nth(1)
            .expect("weight chart")
            .split("data-atlas-block=\"blast-chart\"")
            .next()
            .expect("weight close");
        assert!(weight.contains("src/a.c"));
        assert!(
            weight.find("src/a.c").unwrap()
                < weight.find("non-production (test/bench/other)").unwrap()
        );
        assert!(weight.contains("non-production (test/bench/other) = 90% of scanned static cost"));
        assert!(weight.contains("% of scanned static cost"));
        assert!(weight.contains("production first"));
    }

    #[test]
    fn blast_chart_uses_dependent_top_pct_verbatim() {
        let doc = DocBuilder::new()
            .file(file("src/main.c", "production", 1))
            .file(file("src/a.c", "production", 1))
            .file(file("src/b.c", "production", 1))
            .file(file("src/c.c", "production", 1))
            .function(function(
                "src/main.c",
                "main",
                1,
                Some(100),
                0,
                Some("main"),
            ))
            .function(function("src/a.c", "a", 1, Some(10), 0, None))
            .function(function("src/b.c", "b", 1, Some(10), 0, None))
            .function(function("src/c.c", "c", 1, Some(10), 0, None))
            .include_edge(include_edge("src/a.c", Some("src/main.c"), true))
            .build();
        let view = build_atlas_view(&doc);
        let expected = format_dependents(
            view.blast_radius.dependents,
            view.reachability.source_files,
            view.blast_radius.dependent_multiple,
            view.blast_radius.dependent_top_pct,
        );
        let html = render_brief(&view, &doc, &[]);
        assert!(html.contains(&escape_html(&expected)));
        assert!(view.blast_radius.dependent_top_pct.is_some());
    }

    #[test]
    fn q2_drill_lists_dependents_dsm_and_structural_call_degree() {
        let doc = DocBuilder::new()
            .file(file("src/main.c", "production", 1))
            .file(file("src/a.c", "production", 1))
            .file(file("src/b.c", "production", 1))
            .function(function(
                "src/main.c",
                "main",
                1,
                Some(100),
                0,
                Some("main"),
            ))
            .function(function("src/a.c", "a", 1, Some(10), 0, None))
            .function(function("src/b.c", "b", 1, Some(10), 0, None))
            .include_edge(include_edge("src/a.c", Some("src/main.c"), true))
            .include_edge(include_edge("src/b.c", Some("src/main.c"), true))
            .call_edge(call_edge(
                "src/a.c#a#1:1",
                "main",
                "src/a.c",
                Some("src/main.c"),
                Confidence::Low,
            ))
            .build();
        let html = render_atlas_q2_html(&doc);
        assert!(html.contains("<code>src/main.c</code>"));
        assert!(html.contains("src/a.c"));
        assert!(html.contains("src/b.c"));
        assert!(html.contains("Component DSM"));
        assert!(html.contains("<td>src</td><td>src</td>"));
        assert!(html.contains("1 caller files · 0 callee files (structural)"));
        assert!(html.contains("href=\"report.html#atlas-story\""));
    }

    #[test]
    fn weight_drill_lists_every_source_file_and_component() {
        let doc = DocBuilder::new()
            .file(file("src/a.c", "production", 1))
            .file(file("tests/t.c", "test", 1))
            .function(function("src/a.c", "work", 1, Some(100), 0, None))
            .function(function("tests/t.c", "test_work", 1, Some(900), 0, None))
            .build();
        let html = render_atlas_weight_html(&doc);
        assert!(html.contains("src/a.c"));
        assert!(html.contains("tests/t.c"));
        assert!(html.contains("% of scanned static cost"));
        assert!(html.contains("<td>src</td>"));
        assert!(html.contains("href=\"report.html#atlas-story\""));
    }

    #[test]
    fn charts_add_no_script_handler_or_network() {
        let html = render(&application_fixture());
        assert!(!html.contains("<script"));
        assert!(!has_event_handler(&html));
        assert!(!html.contains("href=\"http"));
        assert!(!html.contains("src=\"http"));
        assert!(!html.contains("url(http"));
        assert!(!html.contains("@import"));
    }

    #[test]
    fn emitted_surfaces_never_claim_time_or_runtime() {
        let doc = application_fixture();
        let mut html = render(&doc);
        html.push_str(&render_atlas_weight_html(&doc));
        html.push_str(&render_atlas_q2_html(&doc));
        for token in [
            "ms",
            "µs",
            "runtime",
            "speedup",
            "latency",
            "throughput",
            "hot path",
        ] {
            assert!(!contains_word(&html, token), "forbidden token {token}");
        }
    }

    #[test]
    fn double_render_is_byte_identical() {
        let doc = application_fixture();
        let view = build_atlas_view(&doc);
        assert_eq!(
            render_brief(&view, &doc, &[]),
            render_brief(&view, &doc, &[])
        );
        assert_eq!(
            render_atlas_weight_html(&doc),
            render_atlas_weight_html(&doc)
        );
        assert_eq!(render_atlas_q2_html(&doc), render_atlas_q2_html(&doc));
    }

    #[test]
    fn rationale_words_forbidden_as_claims_are_neutralized() {
        let sanitized = sanitize_atlas_text(
            "allocator churn dominates runtime; latency and throughput; speedup 2×; 12 ms; 3 µs; hot paths; calc_ms and runtime_cost stay",
        );
        for token in [
            "ms",
            "µs",
            "runtime",
            "speedup",
            "latency",
            "throughput",
            "hot path",
            "hot paths",
        ] {
            assert!(!contains_word(&sanitized, token), "forbidden token {token}");
        }
        assert!(sanitized.contains("static cost"));
        assert!(sanitized.contains("hot functions"));
        assert!(sanitized.contains("calc_ms"));
        assert!(sanitized.contains("runtime_cost"));
    }

    #[test]
    fn all_test_weight_chart_leads_with_the_band() {
        let html = render(&all_test_fixture());
        let weight = html
            .split("data-atlas-block=\"weight-chart\"")
            .nth(1)
            .expect("weight chart")
            .split("data-atlas-block=\"blast-chart\"")
            .next()
            .expect("weight close");
        let band = weight
            .find("non-production (test/bench/other)")
            .expect("non-production band");
        let stub = weight.find(NO_PRODUCTION_COST).expect("production stub");
        assert!(band < stub, "all-test must lead with the band");
    }

    #[test]
    fn no_bare_ellipsis_is_emitted() {
        assert!(!render(&application_fixture()).contains('…'));
    }

    #[test]
    fn rail_finding_link_prefers_the_dominant_severity_occurrence() {
        let doc = document(
            vec![file("src/a.c", "production", 1)],
            vec![function("src/a.c", "main", 1, Some(100), 0, Some("main"))],
            vec![
                finding("PERF-C-X", "src/a.c", 3, Severity::Info, "info occurrence"),
                finding("PERF-C-X", "src/a.c", 9, Severity::Hot, "hot occurrence"),
            ],
            Vec::new(),
            Vec::new(),
        );
        let (_, anchors) = finding_anchor_assignments(&doc);
        let hot_anchor = anchors[1].clone();
        let html = render_rail(&build_atlas_view(&doc), &doc);
        assert!(
            html.contains(&format!("findings-detail.html#{hot_anchor}")),
            "the dominant HOT occurrence must be linked: {html}"
        );
        assert!(
            !html.contains(&format!("findings-detail.html#{}", anchors[0])),
            "the INFO occurrence must not be linked as the dominant finding: {html}"
        );
    }
}
