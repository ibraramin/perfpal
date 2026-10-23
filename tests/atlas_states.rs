//! Atlas page integration locks: A10 (state-unconditional stack) and the
//! amended A8 first-screen order (canvas → answer line → seven blocks).

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
    let mut rest = map;
    while let Some(index) = rest.find("data-atlas-block=\"") {
        let tail = &rest[index + "data-atlas-block=\"".len()..];
        let end = tail.find('"').expect("block marker close");
        blocks.push(tail[..end].to_string());
        rest = &tail[end..];
    }
    blocks
}

fn answer_block(map: &str) -> &str {
    map.split("id=\"atlas-answer\"")
        .nth(1)
        .expect("answer line")
        .split("data-atlas-block=\"actions\"")
        .next()
        .expect("answer close")
}

fn actions_block(map: &str) -> &str {
    map.split("id=\"atlas-actions\"")
        .nth(1)
        .expect("actions block")
        .split("data-atlas-block=\"weight\"")
        .next()
        .expect("actions close")
}

#[test]
fn atlas_states_render_same_stack() {
    let expected = [
        "answer", "actions", "weight", "blast", "cards", "noise", "links",
    ];
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        let map = map_body(&html);
        assert!(
            map.starts_with("<section id=\"map\">\n<div class=\"atlas-page-chrome\""),
            "{name}: #map must open with the first-page chrome"
        );
        assert_eq!(block_order(map), expected, "{name}: brief block stack");
        for marker in [
            "id=\"atlas-canvas\"",
            "id=\"atlas-brief\"",
            "id=\"atlas-drill-q2\"",
            "id=\"atlas-drill-weight\"",
        ] {
            assert!(map.contains(marker), "{name}: missing {marker}");
        }
        // The canvas precedes the brief, and every block follows the canvas.
        let canvas = map.find("id=\"atlas-canvas\"").expect("canvas");
        let brief = map.find("id=\"atlas-brief\"").expect("brief");
        assert!(canvas < brief, "{name}: canvas must precede the brief");
    }
}

#[test]
fn atlas_first_screen_canvas_then_answer_line() {
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        let map = map_body(&html);
        let canvas = map.find("id=\"atlas-canvas\"").expect("canvas");
        let answer = map.find("id=\"atlas-answer\"").expect("answer line");
        assert!(
            canvas < answer,
            "{name}: the answer line must sit below the canvas"
        );
        let first_block = map.find("data-atlas-block=\"").expect("first block");
        assert!(
            canvas < first_block && map[first_block..].starts_with("data-atlas-block=\"answer\""),
            "{name}: the answer line must be the first answer-bearing block"
        );
        // The key-metrics strip is the first answer-bearing block above the
        // canvas (the founder's 2026-10-23 v2 amendment).
        let chrome_start = map
            .find("<div class=\"atlas-page-chrome\"")
            .expect("page chrome");
        let chrome = &map[chrome_start..canvas];
        assert!(
            map.find("class=\"atlas-keymetrics\"")
                .is_some_and(|strip| chrome_start < strip && strip < canvas),
            "{name}: the key-metrics strip must sit above the canvas"
        );
        assert!(
            chrome.contains("atlas-masthead"),
            "{name}: the masthead is part of the first-page chrome"
        );
        // A8 (v2): the answer-bearing strip carries entry, HOTTEST and path.
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
        // The canvas itself still carries the graph, not the chrome.
        let canvas_body = map
            .split("id=\"atlas-canvas\"")
            .nth(1)
            .expect("canvas body")
            .split("id=\"atlas-brief\"")
            .next()
            .expect("canvas close");
        assert!(
            canvas_body.contains("atlas-graph") && canvas_body.contains("atlas-footer"),
            "{name}: the canvas must carry the graph and its one footer"
        );
        assert!(
            !canvas_body.contains("atlas-keymetrics") && !canvas_body.contains("atlas-masthead"),
            "{name}: the chrome must not live inside the canvas"
        );
        let answer = answer_block(map);
        assert!(
            answer.contains("ENTRY ·"),
            "{name}: answer line must name the entry"
        );
        if doc.files.is_empty() {
            assert!(answer.contains("No file with functions was recorded."));
        } else {
            assert!(
                answer.contains("The largest static cost is"),
                "{name}: answer line must name the largest static cost"
            );
        }
        assert!(answer.contains("Caveat:"), "{name}: answer caveat");

        let actionable = doc
            .findings
            .iter()
            .any(|finding| !finding.rule_id.ends_with("-ARCH-002"));
        if actionable {
            assert!(
                answer.contains("Open first:") && answer.contains("<code>"),
                "{name}: the one path:line action must be in the answer line"
            );
            let location = answer
                .split("<code>")
                .nth(1)
                .expect("answer location")
                .split("</code>")
                .next()
                .expect("answer location close");
            assert!(
                location.contains(':'),
                "{name}: answer action must be path:line"
            );
            assert!(
                actions_block(map).contains(&format!("<code>{location}</code>")),
                "{name}: the same action must open KV1"
            );
        } else {
            assert!(
                answer.contains("no findings"),
                "{name}: no-action answer must use the named stub"
            );
        }
    }
}
