//! ADR-0014 integration lock: the always-on `#atlas-story` embed.
//!
//! Every rendered `report.html` carries one story section, exactly three
//! script elements (fixed REPORT_SCRIPT + fixed STORY_SCRIPT + inline JSON),
//! a `<noscript>` narrative, and no banned executable APIs.

#[allow(dead_code)]
mod atlas_fixtures;

use atlas_fixtures::application;
use perfpal::reporter::{render_html, report_script};

#[derive(Debug)]
struct ScriptBlock<'a> {
    tag: &'a str,
    body: &'a str,
}

fn scripts(html: &str) -> Vec<ScriptBlock<'_>> {
    let mut blocks = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<script") {
        let element = &rest[start..];
        let open_end = element.find('>').expect("script open tag terminated");
        let body_start = start + open_end + 1;
        let body_end = rest[body_start..]
            .find("</script>")
            .expect("script close tag")
            + body_start;
        blocks.push(ScriptBlock {
            tag: &rest[start..start + open_end + 1],
            body: &rest[body_start..body_end],
        });
        rest = &rest[body_end + "</script>".len()..];
    }
    blocks
}

#[test]
fn report_carries_one_complete_story_embed() {
    let doc = application();
    let html = render_html(&doc, &[]);

    // Exactly one section and one block marker.
    assert_eq!(
        html.matches(
            "<section id=\"atlas-story\" class=\"atlas-story\" data-atlas-block=\"story\">"
        )
        .count(),
        1,
        "exactly one #atlas-story section"
    );
    let story_start = html.find("<section id=\"atlas-story\"").expect("story");
    let story_end = html[story_start..]
        .find("</section>\n<section id=\"explorer\"")
        .map(|offset| story_start + offset)
        .expect("story closes before #explorer");
    let story = &html[story_start..story_end];

    // Exactly three script elements overall; two inside the story section.
    let all = scripts(&html);
    assert_eq!(all.len(), 3, "ADR-0014: exactly three script elements");
    assert_eq!(
        all.iter().filter(|block| block.tag == "<script>").count(),
        1,
        "exactly one bare REPORT_SCRIPT"
    );
    assert!(html.contains(report_script()), "REPORT_SCRIPT changed");
    let story_blocks = scripts(story);
    assert_eq!(
        story_blocks.len(),
        2,
        "story section carries the data block and the fixed script"
    );
    let data_blocks: Vec<&ScriptBlock<'_>> = story_blocks
        .iter()
        .filter(|block| {
            block.tag.contains("type=\"application/json\"")
                && block.tag.contains("id=\"perfpal-atlas-story-data\"")
        })
        .collect();
    assert_eq!(data_blocks.len(), 1, "exactly one inline JSON block");
    let story_scripts: Vec<&ScriptBlock<'_>> = story_blocks
        .iter()
        .filter(|block| block.tag.contains("id=\"atlas-story-script\""))
        .collect();
    assert_eq!(story_scripts.len(), 1, "exactly one story script");
    assert!(
        !story_scripts[0].body.trim().is_empty(),
        "story script must be non-empty"
    );
    assert!(
        story_scripts[0].body.contains("window.__sim"),
        "fixed story player missing its export"
    );

    // No banned APIs in executable code.
    let executable = format!("{}\n{}", report_script(), story_scripts[0].body);
    for banned in [
        "</script",
        "fetch(",
        "XMLHttpRequest",
        "http://",
        "https://",
        "eval(",
        "Function(",
        "setTimeout",
        "setInterval",
        "innerHTML",
        "outerHTML",
        "document.write",
    ] {
        assert!(
            !executable.contains(banned),
            "banned string {banned:?} in an executable script"
        );
    }

    // JS-off fallback narrative.
    assert!(story.contains("<noscript>"), "missing noscript narrative");
    assert!(
        story.contains("The interactive run story needs JavaScript"),
        "noscript must explain the JS requirement"
    );
    for href in [
        "atlas-weight.html",
        "atlas-q2.html",
        "findings-detail.html#ranked-actions",
        "profile-queue.html",
    ] {
        assert!(
            story.contains(&format!("href=\"{href}\"")),
            "noscript drill link {href} missing"
        );
    }

    // Inline data parses and matches the player's top-level shape.
    let data: serde_json::Value =
        serde_json::from_str(data_blocks[0].body).expect("inline story JSON parses");
    assert_eq!(data["index"]["cases"][0]["id"], "scan");
    let payload = &data["cases"]["scan"];
    for key in [
        "case",
        "synthetic",
        "stage",
        "source",
        "meta",
        "trust",
        "purpose",
        "profile",
        "profileMeta",
        "scenes",
        "sceneMeta",
        "hottestFunction",
        "state",
        "columns",
        "hulls",
        "nodes",
        "edges",
        "trace",
        "findings",
    ] {
        assert!(payload.get(key).is_some(), "payload key {key} missing");
    }
    assert!(
        payload["nodes"].as_array().is_some(),
        "payload nodes must be an array"
    );
    for key in ["layoutFingerprint", "metricsContentFingerprint"] {
        let value = payload["source"][key]
            .as_str()
            .unwrap_or_else(|| panic!("{key} string"));
        assert_eq!(value.len(), 16, "{key} length");
        assert!(value.chars().all(|c| c.is_ascii_hexdigit()), "{key} hex");
    }
    assert_eq!(payload["meta"]["findingsTotal"], doc.findings.len() as u64);

    // No static stage SVG: the report keeps exactly one static SVG (the map).
    assert_eq!(
        story.matches("<svg").count(),
        0,
        "the story must not inline a second static SVG"
    );
    assert_eq!(
        html.matches("<svg").count(),
        1,
        "only the Atlas canvas SVG may be static"
    );
    assert!(
        story.contains("id=\"story-stage-host\""),
        "runtime stage host missing"
    );
    assert!(
        !story.contains("<script src"),
        "no external script source is allowed"
    );
}
