//! Atlas page integration locks: A10 (state-unconditional chart stack) and the
//! step-12 amended A8 first-screen order (strip -> canvas -> progressive charts).
//!
//! The v2 two-layer contract retired the seven-block text wall: the first page
//! is the masthead + key-metrics strip (entry/HOTTEST/path/3 START actions/one
//! structural caveat) and the canvas/rail stage; below it only the four
//! designed charts and the coverage/links line render.

mod atlas_fixtures;

use atlas_fixtures::states;
use perfpal::reporter::render_html;

/// Slice `#map` up to the Explorer section (the canvas nests its own
/// `</section>`, so the first close tag is not the map boundary).
fn map_body(html: &str) -> &str {
    let start = html.find("<section id=\"map\">").expect("#map");
    let end = html
        .find("<section id=\"explorer\">")
        .expect("#explorer after map");
    assert!(start < end, "#map must precede #explorer");
    &html[start..end]
}

fn block_order(map: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut rest = &map[map.find("id=\"atlas-charts\"").expect("charts")..];
    while let Some(index) = rest.find("data-atlas-block=\"") {
        let tail = &rest[index + "data-atlas-block=\"".len()..];
        let end = tail.find('"').expect("block marker close");
        blocks.push(tail[..end].to_string());
        rest = &tail[end..];
    }
    blocks
}

fn chart<'a>(map: &'a str, name: &str) -> &'a str {
    let body = &map[map.find("id=\"atlas-charts\"").expect("charts")..];
    let marker = format!("data-atlas-block=\"{name}\"");
    let start = body.find(&marker).expect("chart marker");
    let tail = &body[start + marker.len()..];
    let end = tail.find("data-atlas-block=\"").unwrap_or(tail.len());
    &tail[..end]
}

#[test]
fn atlas_states_render_same_stack() {
    let expected = ["weight-chart", "blast-chart", "file-chart", "coverage"];
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        let map = map_body(&html);
        assert!(
            map.starts_with("<section id=\"map\">\n<div class=\"atlas-page-chrome\""),
            "{name}: #map must open with the first-page chrome"
        );
        assert_eq!(block_order(map), expected, "{name}: chart block stack");
        for marker in [
            "id=\"atlas-canvas\"",
            "id=\"atlas-brief\"",
            "id=\"atlas-charts\"",
        ] {
            assert!(map.contains(marker), "{name}: missing {marker}");
        }
        assert!(
            !map.contains("id=\"atlas-answer\""),
            "{name}: the retired answer block must not return"
        );
        assert!(
            !map.contains("id=\"atlas-drill-"),
            "{name}: drills moved to standalone pages"
        );
        // The canvas precedes the charts, and every chart follows it.
        let canvas = map.find("id=\"atlas-canvas\"").expect("canvas");
        let charts = map.find("id=\"atlas-charts\"").expect("charts");
        assert!(canvas < charts, "{name}: canvas must precede the charts");
        // The retired wall's block names must not appear.
        for retired in ["answer", "actions", "cards", "noise", "links"] {
            assert!(
                !map.contains(&format!("data-atlas-block=\"{retired}\"")),
                "{name}: retired block {retired} must not return"
            );
        }
        // One structural caveat and one unit definition on the whole map.
        assert_eq!(
            map.matches("class=\"atlas-structural-caption\"").count(),
            1,
            "{name}: exactly one structural caveat"
        );
        assert_eq!(
            map.matches("class=\"atlas-unit\"").count(),
            1,
            "{name}: exactly one unit definition"
        );
    }
}

#[test]
fn atlas_first_screen_strip_then_canvas_then_charts() {
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        let map = map_body(&html);
        let chrome_start = map
            .find("<div class=\"atlas-page-chrome\"")
            .expect("page chrome");
        let canvas = map.find("id=\"atlas-canvas\"").expect("canvas");
        let charts = map.find("id=\"atlas-charts\"").expect("charts");
        assert!(
            chrome_start < canvas && canvas < charts,
            "{name}: strip -> canvas -> charts order"
        );

        let chrome = &map[chrome_start..canvas];
        assert!(
            chrome.contains("atlas-masthead"),
            "{name}: the masthead is part of the first-page chrome"
        );
        assert!(
            chrome.contains("class=\"atlas-keymetrics\""),
            "{name}: the key-metrics strip must sit above the canvas"
        );
        // A8 (v2 + step 12): the strip is the first answer-bearing surface.
        assert!(
            chrome.contains("production") || chrome.contains("no entry"),
            "{name}: the strip must carry the entry badge"
        );
        assert!(
            chrome.contains("HOTTEST:"),
            "{name}: the strip must carry the HOTTEST label"
        );
        assert!(
            chrome.contains("PATH ·"),
            "{name}: the strip must carry the path summary"
        );
        assert!(
            chrome.contains("<b>START</b>"),
            "{name}: the strip must carry the START actions"
        );
        assert!(
            chrome.contains("atlas-structural-caption"),
            "{name}: the one caveat lives in the strip"
        );
        // The canvas itself still carries the graph + its one footer, not chrome.
        let canvas_body = &map[canvas..charts];
        assert!(
            canvas_body.contains("atlas-graph") && canvas_body.contains("atlas-footer"),
            "{name}: the canvas must carry the graph and its one footer"
        );
        assert!(
            !canvas_body.contains("atlas-keymetrics") && !canvas_body.contains("atlas-masthead"),
            "{name}: the chrome must not live inside the canvas"
        );
        // The first below-canvas block is the weight chart, not an answer line.
        assert!(
            map[charts..].starts_with("id=\"atlas-charts\""),
            "{name}: the chart section is the first below-canvas block"
        );
        assert!(
            chart(map, "weight-chart").contains("Where is the mass?"),
            "{name}: weight chart heading"
        );

        // The one path:line action is reachable in the first scroll (strip).
        let actionable = doc
            .findings
            .iter()
            .any(|finding| !finding.rule_id.ends_with("-ARCH-002"));
        if actionable {
            assert!(
                chrome.contains("findings-detail.html#finding-") && chrome.contains(':'),
                "{name}: the strip must carry a path:line action"
            );
        } else {
            assert!(
                chrome.contains("no ranked action recorded"),
                "{name}: no-action strip must use the named stub"
            );
        }
    }
}
