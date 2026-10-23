//! Atlas integration locks: A9 canvas purity and A12 no-new-JS.

mod atlas_fixtures;

use atlas_fixtures::states;
use perfpal::reporter::{render_html, report_script};

fn map_body(html: &str) -> &str {
    let start = html.find("<section id=\"map\">").expect("#map");
    let end = html
        .find("<section id=\"explorer\">")
        .expect("#explorer after map");
    &html[start..end]
}

fn canvas_body(html: &str) -> &str {
    // The brief is the canvas boundary (the canvas nests its own `</section>`).
    html.split("<section id=\"atlas-canvas\"")
        .nth(1)
        .expect("atlas canvas")
        .split("id=\"atlas-brief\"")
        .next()
        .expect("canvas close")
}

fn brief_body(html: &str) -> &str {
    html.split("<section id=\"atlas-brief\"")
        .nth(1)
        .expect("atlas brief")
        .split("</section>")
        .next()
        .expect("brief close")
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
            if previous_ok && cursor > index + 2 && cursor < bytes.len() && bytes[cursor] == b'=' {
                return true;
            }
        }
        index += 1;
    }
    false
}

#[test]
fn atlas_canvas_has_no_metric_panels() {
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        let canvas = canvas_body(&html);
        for needle in [
            "<table",
            "<select",
            "<ul>",
            "<ul ",
            "<li>",
            "<li ",
            "<figure",
            "data-chart=",
            "charts/",
            "data-atlas-block=",
            "class=\"bar\"",
            "class=\"card\"",
            "role=\"table\"",
        ] {
            assert!(
                !canvas.contains(needle),
                "{name}: forbidden canvas markup `{needle}`"
            );
        }
        // The canvas is the graph plus its chrome, not nothing.
        assert!(canvas.contains("atlas-graph"), "{name}: graph field");
        assert!(canvas.contains("atlas-legend"), "{name}: legend");
        assert!(canvas.contains("atlas-footer"), "{name}: footer");
    }
}

#[test]
fn atlas_adds_no_new_script() {
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        assert_eq!(
            html.to_ascii_lowercase().matches("<script").count(),
            1,
            "{name}: report.html must keep exactly one script block"
        );
        assert!(
            html.contains(report_script()),
            "{name}: the one script must be the fixed REPORT_SCRIPT"
        );
        let map = map_body(&html);
        assert!(!map.contains("<script"), "{name}: Atlas adds a script");
        assert!(
            !has_event_handler(map),
            "{name}: Atlas adds an event handler"
        );
        assert!(!map.contains("javascript:"), "{name}: javascript: URL");
        assert!(
            !map.contains("href=\"http") && !map.contains("src=\"http"),
            "{name}: Atlas adds a network reference"
        );
        assert!(!map.contains("@import"), "{name}: Atlas adds @import");
        // Canvas and brief fragments specifically carry no script either.
        assert!(!canvas_body(&html).contains("<script"));
        assert!(!brief_body(&html).contains("<script"));
    }
}

#[test]
fn atlas_double_render_is_byte_identical() {
    for (name, doc) in states() {
        let first = render_html(&doc, &[]);
        let second = render_html(&doc, &[]);
        assert_eq!(first, second, "{name}: double render diverged");
    }
}
