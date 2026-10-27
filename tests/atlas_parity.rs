//! Structural parity lock for plan `polish-report` H1 (ADR-0014).
//!
//! The pre-merge `feat/atlas-sim` report exposed seven `data-atlas-block`
//! surfaces. H1 restores all seven additively, keeps main's four charts, and
//! adds the always-on story. This test pins the complete ordered set and the
//! sibling placement so a later composition pass cannot silently drop one.

mod atlas_fixtures;

use atlas_fixtures::application;
use perfpal::reporter::render_html;

const BLOCKS: [&str; 12] = [
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
    "story",
];

#[test]
fn report_contains_every_branch_block_and_the_story_in_order() {
    let doc = application();
    let html = render_html(&doc, &[]);

    let mut previous = 0;
    for block in BLOCKS {
        let marker = format!("data-atlas-block=\"{block}\"");
        assert_eq!(
            html.matches(&marker).count(),
            1,
            "exactly one {block} block"
        );
        let position = html.find(&marker).unwrap_or_else(|| panic!("{block} block"));
        assert!(
            position > previous,
            "{block} block is out of composition order"
        );
        previous = position;
    }

    // The story is a sibling after the map and before the legacy Explorer
    // (H4 moves Explorer out of the report).
    let map = html.find("<section id=\"map\">").expect("#map");
    let story = html
        .find("<section id=\"atlas-story\"")
        .expect("#atlas-story");
    let explorer = html
        .find("<section id=\"explorer\">")
        .expect("#explorer");
    assert!(map < story && story < explorer, "#map < #atlas-story < #explorer");

    // Exactly the seven branch surfaces plus the story, and the four charts.
    assert_eq!(html.matches("data-atlas-block=\"").count(), BLOCKS.len());
    for chart in ["weight-chart", "blast-chart", "file-chart", "coverage"] {
        assert!(
            html.contains(&format!("data-atlas-block=\"{chart}\"")),
            "chart {chart} missing"
        );
    }

    // The drills stay standalone pages, linked from the blocks.
    assert!(!html.contains("id=\"atlas-drill-"));
    for href in [
        "atlas-weight.html",
        "atlas-q2.html",
        "findings-detail.html#ranked-actions",
        "profile-queue.html",
    ] {
        assert!(html.contains(href), "drill link {href} missing");
    }
}
