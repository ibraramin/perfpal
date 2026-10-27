//! Atlas integration locks: A9 canvas purity and A12 no-new-JS.

mod atlas_fixtures;

use atlas_fixtures::states;
use perfpal::reporter::{render_html, report_script};

fn map_body(html: &str) -> &str {
    let start = html.find("<section id=\"map\">").expect("#map");
    // The story is a sibling immediately after `#map` (ADR-0014), so the map
    // fragment ends there — not at the later Explorer section.
    let end = html
        .find("<section id=\"atlas-story\"")
        .expect("#atlas-story after map");
    &html[start..end]
}

fn canvas_body(html: &str) -> &str {
    // The canvas closes before the detail rail/brief; the canvas itself has no
    // nested `<section>`, so the first close tag is its boundary.
    html.split("<section id=\"atlas-canvas\"")
        .nth(1)
        .expect("atlas canvas")
        .split("</section>")
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
        // The canvas is the graph plus its one footer, not a chrome panel.
        assert!(canvas.contains("atlas-graph"), "{name}: graph field");
        assert!(canvas.contains("atlas-footer"), "{name}: footer");
        assert!(
            !canvas.contains("atlas-legend"),
            "{name}: the legend moved out of the canvas"
        );
        assert!(
            !canvas.contains("atlas-masthead") && !canvas.contains("atlas-keymetrics"),
            "{name}: the chrome must stay outside the canvas"
        );
        let map = map_body(&html);
        assert!(map.contains("atlas-masthead"), "{name}: masthead");
        assert!(
            map.contains("atlas-keymetrics"),
            "{name}: key-metrics strip"
        );
        assert_eq!(
            map.matches("class=\"atlas-legend\"").count(),
            1,
            "{name}: exactly one legend"
        );
        assert_eq!(
            map.matches("class=\"atlas-footer\"").count(),
            1,
            "{name}: exactly one canvas footer"
        );
    }
}

#[test]
fn atlas_map_has_one_caveat_and_one_median_definition() {
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        let map = map_body(&html);
        assert_eq!(
            map.matches("structural (LOW name-based + MEDIUM semantic)")
                .count(),
            1,
            "{name}: the structural caveat must appear exactly once"
        );
        assert_eq!(
            map.matches("× median function = all functions in the scan")
                .count(),
            1,
            "{name}: the median definition must appear exactly once"
        );
    }
}

#[test]
fn atlas_adds_no_new_script() {
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        // ADR-0014 amended A12: exactly three script elements in the report —
        // the unchanged REPORT_SCRIPT, the fixed story script, and the inline
        // JSON data block. None of them comes from the Atlas map fragments.
        assert_eq!(
            html.to_ascii_lowercase().matches("<script").count(),
            3,
            "{name}: report.html must carry exactly three script elements"
        );
        assert_eq!(
            html.matches("<script>").count(),
            1,
            "{name}: exactly one bare REPORT_SCRIPT element"
        );
        assert!(
            html.contains(report_script()),
            "{name}: the bare script must be the fixed REPORT_SCRIPT"
        );
        assert!(
            html.contains("id=\"perfpal-atlas-story-data\""),
            "{name}: inline story data block missing"
        );
        assert!(
            html.contains("id=\"atlas-story-script\""),
            "{name}: fixed story script missing"
        );
        assert!(
            html.contains("data-atlas-block=\"story\""),
            "{name}: story section missing"
        );
        assert!(
            !html.to_ascii_lowercase().contains("<script src"),
            "{name}: no external script source is allowed"
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

#[test]
fn atlas_unreached_chip_is_non_additive_and_matches_footer() {
    let doc = states()
        .into_iter()
        .find(|(name, _)| *name == "library")
        .map(|(_, doc)| doc)
        .expect("library state");
    let html = render_html(&doc, &[]);
    let map = map_body(&html);

    // The library state draws its unreached production callers as cards…
    assert!(
        map.contains("data-node=\"src/caller1.c\""),
        "library must draw the unreached caller cards"
    );
    assert!(map.contains("data-node=\"src/caller2.c\""));

    // …and the chip still reports the global totals with non-additive wording.
    let chip = map
        .split("class=\"atlas-unreached-chip\"")
        .nth(1)
        .expect("unreached chip")
        .split("</g>")
        .next()
        .expect("chip close");
    assert!(chip.contains("production unreached: 2"), "{chip}");
    assert!(!chip.contains("+2 production unreached"), "{chip}");
    assert!(chip.contains("data-prod-unreached=\"2\""), "{chip}");
    assert!(
        !chip.contains("non-production unreached: 0"),
        "a zero line must be omitted: {chip}"
    );

    // The canvas footer agrees with the chip.
    let footer = map
        .split("class=\"atlas-footer\"")
        .nth(1)
        .expect("canvas footer")
        .split("</footer>")
        .next()
        .expect("footer close");
    assert!(footer.contains("unreached production: 2"), "{footer}");
    assert!(footer.contains("non-production: 0"), "{footer}");
}
