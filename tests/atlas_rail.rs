//! Atlas v2 step 11: the first-page per-file detail rail.

mod atlas_fixtures;

use atlas_fixtures::{roles_demo, states};
use perfpal::reporter::atlas::build_atlas_view;
use perfpal::reporter::atlas_brief::RAIL_FILE_CAP;
use perfpal::reporter::{render_findings_detail_html, render_html, render_profile_queue_html};

fn rail_body(html: &str) -> &str {
    html.split("<aside id=\"atlas-rail\"")
        .nth(1)
        .expect("atlas rail")
        .split("</aside>")
        .next()
        .expect("rail close")
}

fn cards(rail: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = rail;
    while let Some(index) = rest.find("<article class=\"atlas-rail-card") {
        let tail = &rest[index..];
        let end = tail.find("</article>").expect("card close");
        out.push(&tail[..end]);
        rest = &tail[end..];
    }
    out
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

fn anchors<'a>(haystack: &'a str, prefix: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut rest = haystack;
    while let Some(index) = rest.find(prefix) {
        let tail = &rest[index + prefix.len()..];
        let end = tail
            .find(|c: char| c == '"' || c == '<' || c.is_whitespace())
            .unwrap_or(tail.len());
        out.push(&tail[..end]);
        rest = &tail[end..];
    }
    out
}

#[test]
fn rail_sits_between_canvas_and_brief_in_every_state() {
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        assert_eq!(
            html.matches("<aside id=\"atlas-rail\"").count(),
            1,
            "{name}: exactly one rail"
        );
        let map_start = html.find("<section id=\"map\">").expect("#map");
        let map_end = html.find("<section id=\"explorer\">").expect("#explorer");
        assert!(
            map_start < html.find("<aside id=\"atlas-rail\"").expect("rail")
                && html.find("<aside id=\"atlas-rail\"").expect("rail") < map_end,
            "{name}: rail must live inside #map"
        );
        let canvas_start = html.find("id=\"atlas-canvas\"").expect("canvas");
        let canvas_close = canvas_start
            + html[canvas_start..]
                .find("</section>")
                .expect("canvas close");
        let rail = html.find("<aside id=\"atlas-rail\"").expect("rail");
        let brief = html.find("id=\"atlas-brief\"").expect("brief");
        assert!(canvas_close < rail, "{name}: rail must follow the canvas");
        assert!(rail < brief, "{name}: rail must precede the brief");
    }
}

#[test]
fn rail_cards_carry_roles_anchors_and_links() {
    for (name, doc) in states() {
        if doc.files.is_empty() {
            continue;
        }
        let html = render_html(&doc, &[]);
        let rail = rail_body(&html);
        let cards = cards(rail);
        assert!(
            (1..=RAIL_FILE_CAP).contains(&cards.len()),
            "{name}: expected 1..={RAIL_FILE_CAP} rail cards, got {}",
            cards.len()
        );
        for card in &cards {
            assert!(
                card.contains("class=\"atlas-rail-role\""),
                "{name}: card missing role label: {card}"
            );
            assert!(
                card.contains("median function") && card.contains("top "),
                "{name}: card missing cost anchor: {card}"
            );
            assert!(
                card.contains("profile-queue.html#fn-"),
                "{name}: card missing profile-queue anchor: {card}"
            );
            assert!(
                card.contains("findings-detail.html#") || card.contains("no findings"),
                "{name}: card missing finding link/stub: {card}"
            );
        }
    }
}

#[test]
fn rail_links_resolve_on_every_page() {
    for (name, doc) in states() {
        let html = render_html(&doc, &[]);
        let rail = rail_body(&html);
        let profile = render_profile_queue_html(&doc);
        let findings = render_findings_detail_html(&doc);

        for target in anchors(rail, "href=\"#atlas-") {
            assert!(
                html.contains(&format!("id=\"atlas-{target}\"")),
                "{name}: dead weight-drill link #atlas-{target}"
            );
        }
        for target in anchors(rail, "href=\"profile-queue.html#") {
            assert!(
                profile.contains(&format!("id=\"{target}\"")),
                "{name}: dead profile-queue link {target}"
            );
        }
        for target in anchors(rail, "href=\"findings-detail.html#") {
            assert!(
                findings.contains(&format!("id=\"{target}\"")),
                "{name}: dead findings link {target}"
            );
        }
    }
}

#[test]
fn rail_empty_state_renders_named_stub() {
    let doc = states()
        .into_iter()
        .find(|(name, _)| *name == "empty")
        .map(|(_, doc)| doc)
        .expect("empty state");
    let html = render_html(&doc, &[]);
    let rail = rail_body(&html);
    assert!(rail.contains("top 0 of 0 source files"), "{rail}");
    assert_eq!(cards(rail).len(), 0, "empty state must show no cards");
    assert!(
        rail.contains("class=\"atlas-rail-stub\"") && rail.contains("no source functions recorded"),
        "{rail}"
    );
}

#[test]
fn rail_application_entry_first_deterministic_and_script_free() {
    let doc = states()
        .into_iter()
        .find(|(name, _)| *name == "application")
        .map(|(_, doc)| doc)
        .expect("application state");
    let html = render_html(&doc, &[]);
    let rail = rail_body(&html);
    let cards = cards(rail);
    assert!(!cards.is_empty(), "application must have rail cards");
    assert!(
        cards[0].contains("title=\"src/main.c\"") && cards[0].contains("ENTRY"),
        "first card must be the elected entry: {}",
        cards[0]
    );

    let first = render_html(&doc, &[]);
    let second = render_html(&doc, &[]);
    assert_eq!(
        rail_body(&first),
        rail_body(&second),
        "rail must be byte-identical across renders"
    );
    assert!(!rail.contains("<script"), "rail must carry no script");
    assert!(!has_event_handler(rail), "rail must carry no event handler");
}

/// Card `title` attributes carry the full path; the first title in a card is
/// the file link (function rows follow).
fn card_paths(rail: &str) -> Vec<String> {
    cards(rail)
        .iter()
        .map(|card| {
            let start = card.find("title=\"").expect("card title") + "title=\"".len();
            let end = start + card[start..].find('"').expect("title close");
            card[start..end].to_string()
        })
        .collect()
}

#[test]
fn rail_order_is_entry_first_then_shipped_production_first() {
    let doc = atlas_fixtures::capped();
    let view = build_atlas_view(&doc);
    let html = render_html(&doc, &[]);
    let rail = rail_body(&html);
    let mut expected: Vec<String> = Vec::new();
    if let Some(entry) = view.entry.point.as_ref() {
        expected.push(entry.path.clone());
    }
    for file in &view.files {
        if expected.len() >= RAIL_FILE_CAP {
            break;
        }
        if !expected.contains(&file.path) {
            expected.push(file.path.clone());
        }
    }
    assert_eq!(card_paths(rail), expected, "rail order must match the view");
}

#[test]
fn rail_capped_state_shows_exactly_the_cap_and_the_remainder() {
    let doc = atlas_fixtures::capped();
    let html = render_html(&doc, &[]);
    let rail = rail_body(&html);
    assert_eq!(
        cards(rail).len(),
        RAIL_FILE_CAP,
        "capped state must show exactly the rail cap"
    );
    assert!(
        rail.contains(&format!(
            "+{} more source files in the weight detail below",
            doc.files.len() - RAIL_FILE_CAP
        )),
        "capped state must disclose the remainder: {rail}"
    );
    assert!(
        rail.contains("href=\"#atlas-drill-weight\""),
        "the remainder note must link to the weight detail"
    );
}

#[test]
fn rail_roles_cover_entry_production_test_bench_unreached() {
    let doc = roles_demo();
    let html = render_html(&doc, &[]);
    let rail = rail_body(&html);
    for role in ["ENTRY", "PRODUCTION", "TEST", "BENCH", "UNREACHED"] {
        assert!(
            rail.contains(&format!("class=\"atlas-rail-role\">{role}<")),
            "missing rail role {role}: {rail}"
        );
    }
}

#[test]
fn rail_function_and_finding_links_target_their_own_row() {
    let doc = atlas_fixtures::application();
    let html = render_html(&doc, &[]);
    let rail = rail_body(&html);
    let profile = render_profile_queue_html(&doc);
    let findings = render_findings_detail_html(&doc);
    let card = cards(rail)
        .into_iter()
        .find(|card| card.contains("profile-queue.html#fn-"))
        .expect("a card with a function link");
    let path_start = card.find("title=\"").expect("file title") + "title=\"".len();
    let path_end = path_start + card[path_start..].find('"').expect("title close");
    let path = &card[path_start..path_end];

    let href_start = card
        .find("href=\"profile-queue.html#")
        .expect("function href")
        + "href=\"profile-queue.html#".len();
    let href_end = href_start + card[href_start..].find('"').expect("href close");
    let anchor = &card[href_start..href_end];
    let row_start = profile
        .find(&format!("id=\"{anchor}\""))
        .unwrap_or_else(|| panic!("profile row {anchor}"));
    let row = &profile[row_start..];
    let row = &row[..row.find("</tr>").expect("row close")];
    assert!(
        row.contains(&format!("{path}:")),
        "profile row {anchor} must be the card's file ({path}): {row}"
    );

    if let Some(finding_href) = card.find("href=\"findings-detail.html#") {
        let href = &card[finding_href..];
        let start = "href=\"findings-detail.html#".len();
        let end = start + href[start..].find('"').expect("finding href close");
        let anchor = &href[start..end];
        assert!(
            findings.contains(&format!("id=\"{anchor}\"")),
            "finding anchor {anchor} must exist"
        );
    }
}
