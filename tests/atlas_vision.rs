//! V8 vision-matrix writer: renders the eight state fixtures to disk on demand.

mod atlas_fixtures;

use std::fs;
use std::path::PathBuf;

use atlas_fixtures::states;
use perfpal::reporter::render_html;

const EXPECTED: [&str; 8] = [
    "application",
    "library",
    "no-entry",
    "empty",
    "all-test",
    "capped",
    "partial-parse",
    "single-file",
];

#[test]
#[ignore = "writes the V8 vision matrix; run with PERFPAL_ATLAS_VISION_DIR=<dir>"]
fn write_vision_matrix() {
    let dir = std::env::var("PERFPAL_ATLAS_VISION_DIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .expect(
            "set PERFPAL_ATLAS_VISION_DIR to the output directory for the V8 vision matrix \
             (the test is #[ignore]d and only runs on demand)",
        );

    fs::create_dir_all(&dir).expect("create the V8 vision output directory");

    let mut written: Vec<(&'static str, usize)> = Vec::new();
    for (name, doc) in states() {
        let first = render_html(&doc, &[]);
        let second = render_html(&doc, &[]);
        assert_eq!(
            first, second,
            "{name}: full-page render must be byte-identical across runs (A4)"
        );
        assert!(
            first.contains("<section id=\"map\">"),
            "{name}: report page must contain the Atlas map section"
        );
        assert!(
            first.contains("<div class=\"atlas-block\" data-atlas-block=\"weight\""),
            "{name}: report page must contain the Atlas weight block"
        );

        let path = dir.join(format!("{name}.html"));
        fs::write(&path, first.as_bytes())
            .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
        written.push((name, first.len()));
    }

    assert_eq!(
        written.len(),
        EXPECTED.len(),
        "the V8 matrix must render exactly eight states"
    );
    let names: Vec<&str> = written.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        names.as_slice(),
        EXPECTED.as_slice(),
        "the V8 matrix state order must match the canonical order"
    );

    for (name, bytes) in &written {
        println!("wrote {name}.html ({bytes} bytes)");
    }
    println!("output directory: {}", dir.display());
}
