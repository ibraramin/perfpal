//! Atlas step-12 progressive-chart locks.
//!
//! The below-graph surface is four designed blocks (weight, blast, file,
//! coverage/links) inside `#atlas-brief`. It must be table-free, script-free,
//! deterministic and state-unconditional, and its drill links must resolve on
//! the standalone `atlas-weight.html` / `atlas-q2.html` pages.

mod atlas_fixtures;

use atlas_fixtures::{application, states};
use perfpal::reporter::{render_atlas_q2_html, render_atlas_weight_html, render_html};

fn map_body(html: &str) -> &str {
    let start = html.find("<section id=\"map\">").expect("#map");
    let end = html
        .find("<section id=\"explorer\">")
        .expect("#explorer after map");
    &html[start..end]
}

fn charts_start(map: &str) -> usize {
    map.find("id=\"atlas-charts\"").expect("charts")
}

fn chart<'a>(map: &'a str, name: &str) -> &'a str {
    let body = &map[charts_start(map)..];
    let marker = format!("data-atlas-block=\"{name}\"");
    let start = body.find(&marker).expect("chart marker");
    let tail = &body[start + marker.len()..];
    let end = tail.find("data-atlas-block=\"").unwrap_or(tail.len());
    &tail[..end]
}

fn chart_blocks(map: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut rest = &map[charts_start(map)..];
    while let Some(index) = rest.find("data-atlas-block=\"") {
        let tail = &rest[index + "data-atlas-block=\"".len()..];
        let end = tail.find('"').expect("block marker close");
        blocks.push(&tail[..end]);
        rest = &tail[end..];
    }
    blocks
}

#[test]
fn charts_are_state_unconditional_and_table_free() {
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        let map = map_body(&html);
        assert_eq!(
            chart_blocks(map),
            [
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
            ],
            "{name}: block stack"
        );
        let brief = &map[charts_start(map)..];
        assert!(
            !brief.contains("<table"),
            "{name}: designed charts must not be text tables"
        );
        for restored in ["actions", "cards", "noise", "links"] {
            assert!(
                brief.contains(&format!("data-atlas-block=\"{restored}\"")),
                "{name}: restored block {restored} missing"
            );
        }
        assert!(
            map.contains("data-atlas-block=\"answer\"") && map.contains("id=\"atlas-answer\""),
            "{name}: the restored answer block must render"
        );
        assert_eq!(
            brief.matches("id=\"atlas-charts\"").count(),
            1,
            "{name}: exactly one chart grid"
        );
    }
}

#[test]
fn charts_emit_no_script_handler_or_network() {
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        let map = map_body(&html);
        let brief = &map[map.find("id=\"atlas-charts\"").expect("charts")..];
        assert!(!brief.contains("<script"), "{name}: chart script");
        for handler in [
            "onclick=",
            "onload=",
            "onerror=",
            "onmouseover=",
            "onchange=",
        ] {
            assert!(!brief.contains(handler), "{name}: event handler {handler}");
        }
        assert!(!brief.contains("http://"), "{name}: network URL");
        assert!(!brief.contains("https://"), "{name}: network URL");
    }
}

#[test]
fn weight_chart_carries_share_band_and_component_caption() {
    let html = render_html(&application(), &[]);
    let map = map_body(&html);
    let weight = chart(map, "weight-chart");
    assert!(weight.contains("% of scanned static cost"), "file share");
    assert!(
        weight.contains("non-production (test/bench/other)"),
        "labelled non-production band"
    );
    assert!(
        weight.contains("Component shares include every member file"),
        "component disclosure"
    );
    assert!(
        weight.contains("atlas-share-ribbon"),
        "production/non-production share ribbon"
    );
    assert_eq!(
        weight.matches("atlas-share-ribbon").count(),
        1,
        "one share ribbon"
    );
}

#[test]
fn weight_ribbon_carries_the_denominator_and_sums_to_100() {
    let html = render_html(&application(), &[]);
    let map = map_body(&html);
    let weight = chart(map, "weight-chart");
    let labels = weight
        .split("atlas-ribbon-labels\">")
        .nth(1)
        .expect("ribbon labels")
        .split("</p>")
        .next()
        .expect("ribbon labels close");
    assert_eq!(
        labels.matches("of scanned static cost").count(),
        2,
        "both ribbon labels must carry the denominator: {labels}"
    );
    let values: Vec<f64> = labels
        .split(|c: char| !(c.is_ascii_digit() || c == '.'))
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect();
    assert_eq!(values.len(), 2, "two shares in the ribbon labels: {labels}");
    assert!(
        (values[0] + values[1] - 100.0).abs() < 0.001,
        "ribbon shares must sum to 100.0: {values:?}"
    );
    assert!(
        weight.contains("aria-label=\"production") && weight.contains("of scanned static cost"),
        "the accessible name must carry the denominator"
    );
}

#[test]
fn weight_band_matches_the_ribbon_pair() {
    let html = render_html(&application(), &[]);
    let map = map_body(&html);
    let weight = chart(map, "weight-chart");
    let labels = weight
        .split("atlas-ribbon-labels\">")
        .nth(1)
        .expect("ribbon labels")
        .split("</p>")
        .next()
        .expect("ribbon labels close");
    let nonproduction = labels
        .split("non-production ")
        .nth(1)
        .expect("non-production label")
        .split(" of scanned")
        .next()
        .expect("non-production value");
    assert!(
        weight.contains(&format!(
            "non-production (test/bench/other) = {nonproduction} of scanned static cost"
        )),
        "the band must print the same non-production share as the ribbon: {labels}"
    );
}

#[test]
fn file_chart_marks_the_entry_and_never_prints_a_zero_multiple() {
    let html = render_html(&application(), &[]);
    let map = map_body(&html);
    let files = chart(map, "file-chart");
    assert!(
        files.contains("src/main.c") && files.contains("atlas-file-role\">ENTRY<"),
        "the elected entry must carry the ENTRY role in the file chart"
    );
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        let map = map_body(&html);
        let files = chart(map, "file-chart");
        assert!(
            !files.contains("0.0× median function"),
            "{name}: zero-cost rows must suppress the multiple"
        );
    }
}

#[test]
fn file_chart_reaches_functions_and_findings() {
    let html = render_html(&application(), &[]);
    let map = map_body(&html);
    let files = chart(map, "file-chart");
    assert!(
        files.contains("profile-queue.html#fn-"),
        "function links must reach the profile queue"
    );
    assert!(
        files.contains("findings-detail.html#finding-"),
        "dominant finding links must reach findings detail"
    );
    assert!(
        files.contains("atlas-weight.html#atlas-file-"),
        "each file bar must link to its weight-detail row"
    );
    assert!(
        files.contains("bar = file cost relative to the largest file shown"),
        "the bar denominator must be printed"
    );
    assert!(
        files.contains("file total ") && files.contains("static cost units (v2)"),
        "each row must print the exact cost and its unit"
    );
    assert!(
        !files.contains("atlas-bar-share"),
        "the long cost anchor moved to its own line; no clipped right column"
    );
}

#[test]
fn coverage_block_links_the_artifacts() {
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        let map = map_body(&html);
        let coverage = chart(map, "coverage");
        for href in [
            "report.md",
            "metrics.json",
            "atlas-weight.html",
            "atlas-q2.html",
            "findings-detail.html#ranked-actions",
            "profile-queue.html",
            "agent-report.md",
            "agent-actions.json",
        ] {
            assert!(
                coverage.contains(&format!("href=\"{href}\"")),
                "{name}: missing {href}"
            );
        }
    }
}

#[test]
fn standalone_drill_pages_are_script_free_and_link_back() {
    let doc = application();
    for (page, title) in [
        (render_atlas_weight_html(&doc), "weight detail"),
        (render_atlas_q2_html(&doc), "Q2 reverse drill"),
    ] {
        assert!(page.starts_with("<!DOCTYPE html>"), "{title}: page shell");
        assert!(
            page.contains("href=\"report.html#map\""),
            "{title}: back link"
        );
        assert!(!page.contains("<script"), "{title}: no script");
        assert!(!page.contains("http://"), "{title}: no network");
        assert!(!page.contains("https://"), "{title}: no network");
    }
    let weight = render_atlas_weight_html(&doc);
    assert!(
        weight.contains("id=\"atlas-drill-weight\""),
        "weight drill id"
    );
    assert!(weight.contains("atlas-file-"), "weight drill file anchors");
}

#[test]
fn coverage_does_not_duplicate_the_canvas_footer() {
    let html = render_html(&application(), &[]);
    let map = map_body(&html);
    let coverage = chart(map, "coverage");
    // The canvas footer already prints parse/unresolved/demoted; the chart
    // coverage line carries links only, so the page has one copy of each.
    assert!(
        !coverage.contains("unresolved includes"),
        "coverage line must not repeat the footer metric"
    );
    assert!(
        !coverage.contains("parse "),
        "coverage line must not repeat the parse metric"
    );
    assert!(
        !coverage.contains("demoted"),
        "coverage line must not repeat the demoted metric"
    );
}
