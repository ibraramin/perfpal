//! Atlas below-graph brief and drills (plan `atlas-build` step 5).
//!
//! The fixed seven-block brief — answer line, KV1 actions, weight strip,
//! blast radius, file cards, noise/confidence footer, drill links — plus the
//! Q2 reverse drill and the weight detail. Every block renders in every state
//! (A10); a named stub replaces absent content. No schema field is added or
//! changed, and the module emits **no** script, event handler or network
//! reference: the canvas/brief bytes are static.
//!
//! Metric contract: every measured number carries its unit/anchor via
//! [`crate::reporter::atlas_metrics`]; M-08's percentile is
//! `blast_radius.dependent_top_pct` verbatim (never recomputed here).

use std::collections::{BTreeMap, BTreeSet};

use crate::reporter::atlas::{AtlasFile, AtlasFunction, AtlasView, EntryPolicy, function_cost_u64};
use crate::reporter::atlas_metrics::{
    NO_FINDINGS, NO_INCLUDE_EDGES, NO_PRODUCTION_COST, NO_RESOLVED_DEPENDENTS, NO_SCANNED_COST,
    NO_SOURCE_FUNCTIONS, format_call_degree, format_caps, format_component_share, format_count,
    format_demoted_files, format_dependents, format_edge_mix, format_entry, format_file_cost,
    format_finding, format_function_cost, format_nonproduction, format_parse,
    format_severity_confidence, format_share, format_trip_provenance, format_unresolved_includes,
};
use crate::reporter::views::{self, CostScale, ScaleOrdering, scale_of};
use crate::reporter::{escape_html, file_anchor_map, finding_anchor_assignments};
use crate::schema::{FindingEntry, FunctionStats, MetricsDocument};

/// One component-page link resolved against `emitted_charts`.
struct ComponentLink<'a> {
    prefix: &'a str,
    share: String,
    href: String,
}

/// Render the fixed seven-block brief plus its two drill sections.
///
/// Returns `#atlas-brief` followed by `#atlas-drill-q2` and
/// `#atlas-drill-weight`; `#map` places the canvas before this fragment.
#[must_use]
pub fn render_brief(view: &AtlasView, doc: &MetricsDocument, emitted_charts: &[String]) -> String {
    let (_, finding_anchors) = finding_anchor_assignments(doc);
    let file_anchors = file_anchor_map(doc);
    let kv1 = views::kv1(doc);
    let function_anchors = FunctionAnchors::new(doc);
    let component_links = component_links(view, doc, emitted_charts);

    let mut out = String::new();
    out.push_str("<section id=\"atlas-brief\" class=\"atlas-brief\">\n");
    out.push_str("<style>");
    out.push_str(BRIEF_CSS);
    out.push_str("</style>\n");
    render_answer(&mut out, view, doc, &kv1, &finding_anchors);
    render_actions(&mut out, doc, &kv1, &finding_anchors, &function_anchors);
    render_weight(&mut out, view);
    render_blast(&mut out, view);
    render_cards(&mut out, view, &file_anchors, &function_anchors);
    render_noise(&mut out, view);
    render_links(&mut out, view, &component_links, kv1.total_actions);
    out.push_str("</section>\n");
    render_q2_drill(&mut out, view, doc);
    render_weight_drill(&mut out, view, &file_anchors);
    out
}

// ---------------------------------------------------------------------------
// Seven-block brief
// ---------------------------------------------------------------------------

fn render_answer(
    out: &mut String,
    view: &AtlasView,
    doc: &MetricsDocument,
    kv1: &views::Kv1View,
    finding_anchors: &[String],
) {
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
            escape_html(&finding.title),
            escape_html(&format_severity_confidence(
                finding.severity,
                finding.confidence
            )),
        ));
    } else {
        let text = no_action_text(kv1);
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
}

fn render_actions(
    out: &mut String,
    doc: &MetricsDocument,
    kv1: &views::Kv1View,
    finding_anchors: &[String],
    function_anchors: &FunctionAnchors,
) {
    out.push_str("<div class=\"atlas-block\" data-atlas-block=\"actions\" id=\"atlas-actions\">\n");
    out.push_str("<h3>Start here — ranked actions (production first)</h3>\n");
    if kv1.rows.is_empty() {
        out.push_str(&format!("<p>{}</p>\n", escape_html(&no_action_text(kv1))));
    } else {
        out.push_str("<ol class=\"atlas-action-list\">\n");
        let mut seen_locations: BTreeMap<(String, u32), usize> = BTreeMap::new();
        for row in &kv1.rows {
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
                escape_html(&finding.title),
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
                escape_html(&sanitize_rationale(&finding.message))
            ));
        }
        out.push_str("</ol>\n");
        out.push_str(&format!(
            "<p class=\"atlas-counts\">{} · {} in the full list</p>\n",
            escape_html(&if kv1.nonprod_file_count == 1 {
                format!("{} non-production file demoted", kv1.nonprod_file_count)
            } else {
                format!("{} non-production files demoted", kv1.nonprod_file_count)
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
}

fn render_weight(out: &mut String, view: &AtlasView) {
    out.push_str("<div class=\"atlas-block\" data-atlas-block=\"weight\" id=\"atlas-weight\">\n");
    out.push_str("<h3>Where is the mass? — share of scanned static cost</h3>\n");
    out.push_str(
        "<p class=\"atlas-caption\">Production files first; non-production cost is counted in the labelled band, never dropped.</p>\n",
    );
    let total = view.totals.total_cost;
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
            "<p class=\"atlas-caption\">Component shares include every member file; component groups are approximate prefix groups.</p>\n",
        );
    }
    let production_cost = view
        .files
        .iter()
        .filter(|file| file.production)
        .map(|file| file.cost)
        .fold(0_u64, u64::saturating_add);
    let nonproduction_cost = total.saturating_sub(production_cost);
    out.push_str(&format!(
        "<p class=\"atlas-nonprod\">{}</p>\n",
        escape_html(&format_nonproduction(
            nonproduction_cost,
            total,
            view.coverage.demoted_files
        ))
    ));
    out.push_str("</div>\n");
}

fn render_blast(out: &mut String, view: &AtlasView) {
    out.push_str("<div class=\"atlas-block\" data-atlas-block=\"blast\" id=\"atlas-blast\">\n");
    out.push_str("<h3>What breaks if I touch it? — blast radius</h3>\n");
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
                "<p><a href=\"#atlas-drill-q2\">Full dependent list and Q2 reverse drill</a></p>\n",
            );
            out.push_str(
                "<p class=\"atlas-caption\">Callers and callees are structural (LOW name-based + MEDIUM semantic) and may over/under-approximate.</p>\n",
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
}

fn render_cards(
    out: &mut String,
    view: &AtlasView,
    file_anchors: &BTreeMap<String, String>,
    function_anchors: &FunctionAnchors,
) {
    out.push_str("<div class=\"atlas-block\" data-atlas-block=\"cards\" id=\"atlas-cards\">\n");
    out.push_str("<h3>File cards — what is inside the top files</h3>\n");
    if view.files.is_empty() {
        out.push_str(&format!("<p>{NO_SOURCE_FUNCTIONS}</p>\n"));
    } else {
        out.push_str("<div class=\"atlas-cards\">\n");
        for file in view.files.iter().take(6) {
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
                    escape_html(&finding.title),
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
                        escape_html(&function.name),
                        escape_html(&function_anchor_for(function, function_anchors)),
                    ));
                }
                out.push_str("</ol>\n");
            }
            let anchor = file_anchors
                .get(&file.path)
                .cloned()
                .unwrap_or_else(|| format!("file-{}", slug(&file.path)));
            out.push_str(&format!(
                "<p><a href=\"#atlas-{anchor}\">Weight detail for this file</a> · <a href=\"findings-detail.html#ranked-actions\">complete action list</a></p>\n",
                anchor = escape_html(&anchor),
            ));
            out.push_str("</article>\n");
        }
        out.push_str("</div>\n");
        out.push_str(&format!(
            "<p class=\"atlas-counts\">showing top {} of {} source files (production-first)</p>\n",
            view.files.len().min(6),
            format_count(view.files.len() as u64),
        ));
    }
    out.push_str("</div>\n");
}

fn render_noise(out: &mut String, view: &AtlasView) {
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
        "<p class=\"atlas-caption\">Structural edges are LOW name-based + MEDIUM semantic and may over/under-approximate; static cost is not ground truth.</p>\n",
    );
    out.push_str("</div>\n");
}

fn render_links(
    out: &mut String,
    view: &AtlasView,
    components: &[ComponentLink<'_>],
    total_actions: usize,
) {
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
        "<li><a href=\"#atlas-drill-q2\">Q2 reverse drill</a> — dependents and structural callers/callees for {}</li>\n",
        escape_html(focus_path(view).unwrap_or("no entry")),
    ));
    out.push_str(&format!(
        "<li><a href=\"#atlas-drill-weight\">Weight detail</a> — {} and {} by share</li>\n",
        escape_html(&views::plural(view.files.len(), "source file")),
        escape_html(&views::plural(view.components.len(), "component")),
    ));
    if components.is_empty() {
        out.push_str("<li>Component pages: no component page emitted</li>\n");
    } else {
        out.push_str("<li>Component pages: ");
        for (index, component) in components.iter().enumerate() {
            if index > 0 {
                out.push_str(" · ");
            }
            out.push_str(&format!(
                "<a href=\"{}\">{}</a> ({})",
                escape_html(&component.href),
                escape_html(component.prefix),
                escape_html(&component.share),
            ));
        }
        out.push_str("</li>\n");
    }
    out.push_str("<li><a href=\"#explorer\">Explorer</a> — legacy static chart stems</li>\n");
    out.push_str("</ul>\n</div>\n");
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
        "<p class=\"atlas-caption\">Callers and callees are structural (LOW name-based + MEDIUM semantic) and may over/under-approximate.</p>\n</section>\n",
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
        let anchor = file_anchors
            .get(&file.path)
            .cloned()
            .unwrap_or_else(|| format!("file-{}", slug(&file.path)));
        out.push_str(&format!(
            "<tr id=\"atlas-{anchor}\"><td><span class=\"atlas-path\">{}</span></td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            escape_html(&file.path),
            escape_html(&file.path_class),
            escape_html(&file_cost_anchor(file)),
            escape_html(&format_share(file.cost, total)),
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
        "<p class=\"atlas-caption\">Component rows are approximate prefix groups; the non-production band in the brief counts the rest.</p>\n",
    );
    out.push_str("</section>\n");
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Per-scale function anchors (M-05): unit + `× median function` + `top P%`.
struct FunctionAnchors {
    v2: ScaleOrdering,
    v1: ScaleOrdering,
}

impl FunctionAnchors {
    fn new(doc: &MetricsDocument) -> Self {
        Self {
            v2: ScaleOrdering::build(&doc.stats.functions, CostScale::V2),
            v1: ScaleOrdering::build(&doc.stats.functions, CostScale::V1),
        }
    }

    fn format(&self, function: &FunctionStats) -> String {
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

fn function_anchor_for(function: &AtlasFunction, _anchors: &FunctionAnchors) -> String {
    format_function_cost(
        function.cost,
        function.scale,
        positive_median(function.cost, function.median),
        function.rank,
        function.population,
    )
}

/// A zero-cost row must not print `0×` (metrics spec §5 missing values).
fn positive_median(cost: u64, median: Option<f64>) -> Option<f64> {
    if cost == 0 { None } else { median }
}

fn file_cost_anchor(file: &AtlasFile) -> String {
    format_file_cost(
        file.cost,
        file.scale,
        positive_median(file.cost, file.median),
        file.file_rank,
        file.file_population,
    )
}

fn hottest_file(view: &AtlasView) -> Option<&AtlasFile> {
    let path = view.hottest.as_deref()?;
    view.files.iter().find(|file| file.path == path)
}

fn focus_path(view: &AtlasView) -> Option<&str> {
    view.entry
        .point
        .as_ref()
        .map(|point| point.path.as_str())
        .or(view.hottest.as_deref())
}

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

fn caveat(view: &AtlasView) -> String {
    let coverage = &view.coverage;
    if view.state.empty {
        return "no source functions recorded — every block shows a named stub.".to_string();
    }
    if view.state.no_entry {
        return "no entry was elected; the cost ranking and the action list still stand."
            .to_string();
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
    if view.state.library {
        return "there is no forward path from the library root; callers are listed in the Q2 drill."
            .to_string();
    }
    if view.state.all_test {
        return "all source cost is non-production (test/bench/other); production rankings are empty."
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
    "the drawn edges are structural (LOW name-based + MEDIUM semantic) and may over/under-approximate."
        .to_string()
}

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

fn location(finding: &FindingEntry) -> String {
    format!("{}:{}", finding.path, finding.span.start_line)
}

fn slug(path: &str) -> String {
    path.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn component_links<'a>(
    view: &'a AtlasView,
    doc: &MetricsDocument,
    emitted_charts: &[String],
) -> Vec<ComponentLink<'a>> {
    let emitted: BTreeSet<&str> = emitted_charts
        .iter()
        .map(|name| name.strip_suffix(".svg").unwrap_or(name))
        .collect();
    let total = view.totals.total_cost;
    view.components
        .iter()
        .filter(|component| component.production_file_count > 0)
        .take(3)
        .map(|component| {
            let href = doc
                .components
                .iter()
                .flatten()
                .find(|entry| entry.path_prefix == component.prefix)
                .map(|entry| {
                    let filename = crate::charts::component::component_filename(
                        entry.rank,
                        &entry.path_prefix,
                    );
                    let stem = filename.strip_suffix(".svg").unwrap_or(&filename);
                    if emitted.contains(stem) {
                        format!("charts/{filename}")
                    } else {
                        "#components".to_string()
                    }
                })
                .unwrap_or_else(|| "#components".to_string());
            ComponentLink {
                prefix: &component.prefix,
                share: format_component_share(component.cost, total, component.file_count),
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

/// Rule messages are analyzer-authored and can contain words the Atlas
/// contract forbids as claims (`runtime`, `latency`, …). The Atlas surface
/// substitutes neutral static wording for those whole words so the emitted
/// page never makes a time claim; every other byte is preserved verbatim.
fn sanitize_rationale(message: &str) -> String {
    let mut text = message.to_string();
    for (needle, replacement) in [
        ("runtime", "static cost"),
        ("latency", "static cost"),
        ("throughput", "static cost"),
        ("speedup", "static improvement"),
        ("hot path", "hot function"),
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
        let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
        let after_ok = end == bytes.len() || !bytes[end].is_ascii_alphanumeric();
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

// ---------------------------------------------------------------------------
// CSS (scoped, deterministic, print-safe; no script, no network)
// ---------------------------------------------------------------------------

const BRIEF_CSS: &str = "\
#atlas-brief,#atlas-drill-q2,#atlas-drill-weight{font-family:monospace,monospace;color:#0f172a;max-width:72rem;margin:0 auto;padding:0 1rem}\
#atlas-brief .atlas-block{border-top:1px solid #cbd5e1;padding:.9rem 0}\
#atlas-brief h3,#atlas-drill-q2 h3,#atlas-drill-weight h3{font-size:1rem;margin:.2rem 0 .5rem}\
#atlas-brief h4,#atlas-drill-q2 h4,#atlas-drill-weight h4{font-size:.95rem;margin:.6rem 0 .3rem}\
.atlas-answer-line{font-size:1.02rem;line-height:1.5}\
.atlas-caveat{display:block;margin-top:.35rem;color:#334155}\
.atlas-rank{font-weight:bold;margin-right:.4rem}\
.atlas-action{margin:.6rem 0}\
.atlas-anchor,.atlas-share,.atlas-caption,.atlas-counts,.atlas-trip,.atlas-note{color:#334155;font-size:.9rem}\
.atlas-badge{display:inline-block;border:1px solid #334155;border-radius:3px;padding:0 .25rem;margin-left:.25rem;font-size:.85rem}\
.atlas-class{display:inline-block;border:1px solid #64748b;border-radius:3px;padding:0 .25rem;font-size:.85rem}\
.atlas-class.nonprod{color:#7c2d12;border:1px dashed #9a3412}\
.atlas-action-list,.atlas-weight-list,.atlas-dependent-list,.atlas-fn-list,.atlas-drill-links{padding-left:1.2rem}\
.atlas-why{margin:.2rem 0}\
.atlas-cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(20rem,1fr));gap:.75rem}\
.atlas-card{border:1px solid #cbd5e1;border-radius:6px;padding:.6rem}\
.atlas-card h4{margin:.1rem 0 .35rem;font-size:.95rem}\
.atlas-nonprod{border-left:4px solid #9a3412;padding-left:.5rem}\
.atlas-drill{border-top:2px solid #334155;padding:.9rem 0}\
.atlas-table-scroll{overflow-x:auto;max-width:100%}\
.atlas-drill table{border-collapse:collapse;width:100%;margin:.5rem 0}\
.atlas-drill th,.atlas-drill td{border:1px solid #999;padding:.2rem .4rem;text-align:left;vertical-align:top}\
.atlas-drill th{background:#eee}\
@media print{.atlas-drill{display:none}}\
";

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::{Confidence, Severity};
    use crate::reporter::atlas::build_atlas_view;
    use crate::reporter::atlas::test_support::*;

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
            let boundary =
                |byte: Option<u8>| byte.is_none_or(|value| !value.is_ascii_alphanumeric());
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

    const BLOCKS: [&str; 7] = [
        "answer", "actions", "weight", "blast", "cards", "noise", "links",
    ];

    #[test]
    fn every_state_renders_the_same_seven_blocks() {
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
            assert!(html.contains("id=\"atlas-drill-q2\""));
            assert!(html.contains("id=\"atlas-drill-weight\""));
            assert!(html.contains("id=\"atlas-answer\""));
        }
    }

    #[test]
    fn answer_line_names_entry_hottest_and_one_action() {
        let builder = DocBuilder::new()
            .file(file("src/main.c", "production", 1))
            .file(file("src/a.c", "production", 1))
            .function(function(
                "src/main.c",
                "main",
                1,
                Some(100),
                0,
                Some("main"),
            ))
            .function(function("src/a.c", "work", 10, Some(900), 1, None))
            .include_edge(include_edge("src/a.c", Some("src/main.c"), true))
            .finding(finding(
                "PERF-C-LOOP-001",
                "src/a.c",
                12,
                Severity::Hot,
                "Loop with a large trip count",
            ));
        let doc = builder.build();
        let html = render(&doc);
        let answer = html
            .split("id=\"atlas-answer\"")
            .nth(1)
            .expect("answer block")
            .split("</div>")
            .next()
            .expect("answer close");
        assert!(answer.contains("ENTRY · production main"));
        assert!(answer.contains("The largest static cost is src/a.c"));
        assert!(answer.contains("Open first:"));
        assert!(answer.contains("<code>src/a.c:12</code>"));
        assert!(answer.contains("HOT · HIGH"));
        assert!(answer.contains("Caveat:"));
        assert_eq!(answer.matches("<code>").count(), 1);
    }

    #[test]
    fn empty_state_uses_named_stubs_in_every_block() {
        let html = render(&empty_fixture());
        for stub in [NO_SOURCE_FUNCTIONS, NO_FINDINGS, NO_ENTRY_TOKEN] {
            assert!(html.contains(stub), "missing stub {stub}");
        }
        let all_test = render(&all_test_fixture());
        assert!(all_test.contains(NO_PRODUCTION_COST));
    }

    const NO_ENTRY_TOKEN: &str = "no entry";

    #[test]
    fn kv1_actions_print_m05_anchor_and_trip_provenance() {
        let mut function = function("src/a.c", "work", 10, Some(900), 1, None);
        function.known_trip_value = Some(10);
        function.unknown_trip_loops = 1;
        let doc = DocBuilder::new()
            .file(file("src/a.c", "production", 1))
            .function(function)
            .finding(finding(
                "PERF-C-LOOP-001",
                "src/a.c",
                12,
                Severity::Hot,
                "Loop with a large trip count",
            ))
            .build();
        let html = render(&doc);
        assert!(html.contains("× median function"));
        assert!(html.contains("of functions"));
        assert!(html.contains("known trip ×10 — verify it actually runs"));
        assert!(html.contains("unknown-trip ×1 — estimate incomplete"));
        assert!(html.contains(">production<"));
    }

    #[test]
    fn weight_strip_is_production_first_with_labelled_nonproduction_band() {
        let doc = DocBuilder::new()
            .file(file("src/a.c", "production", 1))
            .file(file("tests/t.c", "test", 1))
            .function(function("src/a.c", "work", 1, Some(100), 0, None))
            .function(function("tests/t.c", "test_work", 1, Some(900), 0, None))
            .build();
        let html = render(&doc);
        let weight = html
            .split("id=\"atlas-weight\"")
            .nth(1)
            .expect("weight block")
            .split("id=\"atlas-blast\"")
            .next()
            .expect("weight close");
        assert!(weight.contains("src/a.c"));
        assert!(
            weight.find("src/a.c").unwrap()
                < weight.find("non-production (test/bench/other)").unwrap()
        );
        assert!(weight.contains("non-production (test/bench/other) = 90% of scanned static cost"));
        assert!(weight.contains("% of scanned static cost"));
    }

    #[test]
    fn blast_radius_uses_dependent_top_pct_verbatim() {
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
        let html = render(&doc);
        let drill = html
            .split("id=\"atlas-drill-q2\"")
            .nth(1)
            .expect("q2 drill");
        assert!(drill.contains("<code>src/main.c</code>"));
        assert!(drill.contains("src/a.c"));
        assert!(drill.contains("src/b.c"));
        assert!(drill.contains("Component DSM"));
        assert!(drill.contains("<td>src</td><td>src</td>"));
        assert!(drill.contains("1 caller files · 0 callee files (structural)"));
    }

    #[test]
    fn weight_drill_lists_every_source_file_and_component() {
        let doc = DocBuilder::new()
            .file(file("src/a.c", "production", 1))
            .file(file("tests/t.c", "test", 1))
            .function(function("src/a.c", "work", 1, Some(100), 0, None))
            .function(function("tests/t.c", "test_work", 1, Some(900), 0, None))
            .build();
        let html = render(&doc);
        let drill = html
            .split("id=\"atlas-drill-weight\"")
            .nth(1)
            .expect("weight drill");
        assert!(drill.contains("src/a.c"));
        assert!(drill.contains("tests/t.c"));
        assert!(drill.contains("% of scanned static cost"));
        assert!(drill.contains("<td>src</td>"));
    }

    #[test]
    fn brief_adds_no_script_handler_or_network() {
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
        let html = render(&application_fixture());
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
    }

    #[test]
    fn rationale_words_forbidden_as_claims_are_neutralized() {
        let mut f = finding(
            "PERF-C-LOOP-001",
            "src/a.c",
            3,
            Severity::Hot,
            "Loop with a large trip count",
        );
        f.message =
            "allocator churn dominates runtime; latency and throughput; speedup 2×; 12 ms; 3 µs; hot path"
                .to_string();
        let doc = DocBuilder::new()
            .file(file("src/a.c", "production", 1))
            .function(function("src/a.c", "work", 1, Some(10), 0, None))
            .finding(f)
            .build();
        let html = render(&doc);
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
        assert!(html.contains("static cost"));
    }

    #[test]
    fn no_bare_ellipsis_is_emitted() {
        assert!(!render(&application_fixture()).contains('…'));
    }
}
