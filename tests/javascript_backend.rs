//! JavaScript backend + wiring integration test (plan `js-ts`, phase B2-W1).
//!
//! Exercises the real tree-sitter-javascript grammar through the public seam
//! (`scanner::scan`) and the real aggregator (`aggregator::assemble`) over
//! temp trees, plus full-pipeline `cli::run_with` runs for the emitted
//! document and the double-run determinism lock.
//!
//! Scope (B2-W1): detection/tokens, discovery skips, parse health, the entry
//! ladder, loop/trip + cost provenance, allocation/error/idiom facts, the
//! relative import graph, "no JS findings yet" (rules are B2-W2), and
//! byte-determinism modulo the two contract-permitted provenance fields.
//!
//! Deliberately does NOT assert `PERF-JS-*` rule ids: the rule catalog and
//! schema v1.8 are B2-W2.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::aggregator::{Provenance, assemble};
use perfpal::cli::run_with;
use perfpal::ir::{AllocKind, IncludeForm, ParseStatus};
use perfpal::scanner::{FileUnit, scan};
use perfpal::schema::{FunctionStats, MetricsDocument};

mod common;

// ---------------------------------------------------------------------------
// Temp-tree + document helpers
// ---------------------------------------------------------------------------

fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-js-itest-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp root");
    dir
}

/// Write `(relative path, contents)` pairs under a fresh temp root.
fn write_tree(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = temp_root(tag);
    for (path, contents) in files {
        let full = root.join(path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).expect("create fixture parent");
        }
        fs::write(&full, contents).unwrap_or_else(|e| panic!("write {path}: {e}"));
    }
    root
}

fn scan_tree(tag: &str, files: &[(&str, &str)]) -> (PathBuf, Vec<FileUnit>) {
    let root = write_tree(tag, files);
    let units = scan(&root).expect("scan must succeed");
    (root, units)
}

fn prov() -> Provenance {
    Provenance {
        root_path: ".".to_string(),
        started_at: "2026-01-01T00:00:00Z".to_string(),
        duration_ms: 0,
        tool_version: "test".to_string(),
    }
}

fn doc_for(units: &[FileUnit]) -> MetricsDocument {
    assemble(units, &[], &prov())
}

fn unit<'a>(units: &'a [FileUnit], path: &str) -> &'a FileUnit {
    units
        .iter()
        .find(|u| u.file.path == path)
        .unwrap_or_else(|| panic!("missing unit {path}"))
}

fn stat<'a>(doc: &'a MetricsDocument, path: &str, name: &str) -> &'a FunctionStats {
    doc.stats
        .functions
        .iter()
        .find(|f| f.path == path && f.name == name)
        .unwrap_or_else(|| panic!("missing stats entry {path}#{name}"))
}

fn js_function<'a>(unit: &'a FileUnit, name: &str) -> &'a perfpal::scanner::FunctionUnit {
    unit.functions
        .iter()
        .find(|f| f.record.name == name)
        .unwrap_or_else(|| panic!("missing function {name} in {}", unit.file.path))
}

fn argv(args: &[&str]) -> Vec<OsString> {
    std::iter::once(OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

/// Run the pipeline once against `root`, returning the exit code and the
/// output directory (`<fresh temp dir>/perfpal-report`).
fn run_pipeline(tag: &str, root: &Path) -> (i32, PathBuf) {
    let out = temp_root(tag).join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        root.to_string_lossy().as_ref(),
    ]));
    (code, out)
}

fn read_json(path: &Path) -> serde_json::Value {
    let raw = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn json_stat<'a>(doc: &'a serde_json::Value, path: &str, name: &str) -> &'a serde_json::Value {
    doc["stats"]["functions"]
        .as_array()
        .expect("stats.functions[]")
        .iter()
        .find(|f| f["path"] == path && f["name"] == name)
        .unwrap_or_else(|| panic!("missing JSON stats entry {path}#{name}"))
}

// ---------------------------------------------------------------------------
// (1) Detection + tokens
// ---------------------------------------------------------------------------

#[test]
fn detection_selects_js_and_typescript_suffixes_and_skips_declarations() {
    let (root, units) = scan_tree(
        "detect",
        &[
            ("a.js", "const a = 1;\n"),
            ("b.jsx", "const b = <div />;\n"),
            ("c.mjs", "export const c = 1;\n"),
            ("d.cjs", "module.exports = 1;\n"),
            // TypeScript landed in B3: value files are discovered with their
            // grammar-boundary tokens; declaration files stay invisible.
            ("e.ts", "export const e: number = 1;\n"),
            ("f.tsx", "export const f = <div />;\n"),
            ("g.d.ts", "declare const g: number;\n"),
            ("h.d.mts", "export type H = number;\n"),
            ("i.d.cts", "export type I = number;\n"),
        ],
    );

    let paths: Vec<&str> = units.iter().map(|u| u.file.path.as_str()).collect();
    assert_eq!(
        paths,
        ["a.js", "b.jsx", "c.mjs", "d.cjs", "e.ts", "f.tsx"],
        "js/ts value files are discovered; .d.* are not; sorted"
    );
    assert!(paths.windows(2).all(|w| w[0] <= w[1]), "sorted emission");

    let tokens: Vec<&str> = units.iter().map(|u| u.file.language.as_str()).collect();
    assert_eq!(tokens, ["js", "js", "js", "js", "ts", "tsx"]);

    let doc = doc_for(&units);
    assert_eq!(
        doc.scan.languages,
        ["js", "ts", "tsx"],
        "grammar-boundary tokens are reported sorted"
    );

    fs::remove_dir_all(&root).ok();
}

#[test]
fn mixed_c_and_js_tree_reports_both_languages() {
    let (root, units) = scan_tree(
        "mixed",
        &[
            ("a.c", "int main(void) { return 0; }\n"),
            ("b.js", "const b = 1;\n"),
        ],
    );
    let doc = doc_for(&units);
    assert_eq!(
        doc.scan.languages,
        ["c", "js"],
        "mixed C+JS tree reports both sorted tokens"
    );
    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (2) Discovery skips
// ---------------------------------------------------------------------------

#[test]
fn discovery_skips_noise_directories_minified_names_and_oversize_js() {
    // > 2 MiB of valid-ish source: JS must be skipped by the size cap, C must
    // still be scanned (the cap is JS-only).
    let big_js = format!("var x = 1;\n//{}", "x".repeat(2 * 1024 * 1024));
    let big_c = format!(
        "int main(void) {{ return 0; }}\n/*{}*/",
        "c".repeat(2 * 1024 * 1024)
    );

    let (root, units) = scan_tree(
        "skips",
        &[
            ("normal.js", "const n = 1;\n"),
            ("a.min.js", "var a=1;\n"),
            ("b.min.mjs", "export var b=1;\n"),
            ("c.min.cjs", "module.exports=1;\n"),
            ("d.bundle.js", "var d=1;\n"),
            ("big.js", &big_js),
            ("big.c", &big_c),
            ("vendor/mod.js", "export const v = 1;\n"),
            ("node_modules/dep.js", "module.exports = 1;\n"),
            ("bower_components/dep.js", "module.exports = 2;\n"),
            ("dist/out.js", "export const d = 1;\n"),
            ("build/out.js", "export const b = 2;\n"),
            ("out/out.js", "export const o = 3;\n"),
            ("coverage/out.js", "export const c = 4;\n"),
        ],
    );

    let paths: Vec<&str> = units.iter().map(|u| u.file.path.as_str()).collect();
    assert_eq!(
        paths,
        ["big.c", "normal.js", "vendor/mod.js"],
        "only the normal JS, the vendored JS, and the oversized C file survive"
    );

    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (3) Parse health
// ---------------------------------------------------------------------------

#[test]
fn malformed_js_is_partial_and_clean_js_is_ok() {
    let (root, units) = scan_tree(
        "health",
        &[
            ("clean.js", "function f() { return 1; }\n"),
            ("broken.js", "function f( {\n  return 1;\n"),
        ],
    );

    let broken = unit(&units, "broken.js");
    assert_eq!(broken.parse_health.status, ParseStatus::Partial);
    assert!(
        broken.parse_health.error_node_count > 0,
        "partial parse records error nodes"
    );
    assert!(
        !broken.parse_health.error_regions.is_empty(),
        "partial parse records error regions"
    );

    let clean = unit(&units, "clean.js");
    assert_eq!(clean.parse_health.status, ParseStatus::Ok);
    assert_eq!(clean.parse_health.error_node_count, 0);

    let doc = doc_for(&units);
    let summary = doc.parse_health_summary.expect("parse_health_summary");
    assert_eq!(summary.ok, 1, "one clean file");
    assert_eq!(summary.partial, 1, "one malformed file");
    assert_eq!(summary.failed, 0, "no failed files");

    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (4) Entry ladder
// ---------------------------------------------------------------------------

#[test]
fn package_json_main_declares_a_high_confidence_script_entry() {
    let (root, units) = scan_tree(
        "entry-main",
        &[
            ("package.json", r#"{"main":"src/entry.js"}"#),
            ("src/entry.js", "export function start() { return 1; }\n"),
        ],
    );
    let doc = doc_for(&units);
    let module = stat(&doc, "src/entry.js", "<module>");
    assert_eq!(module.entry_kind.as_deref(), Some("script"));
    assert_eq!(module.entry_confidence.as_deref(), Some("HIGH"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn package_json_bin_string_object_and_array_forms_are_declared_high() {
    let cases: [(&str, &str, &[&str]); 3] = [
        ("bin-string", r#"{"bin":"bin/cli.js"}"#, &["bin/cli.js"]),
        (
            "bin-object",
            r#"{"bin":{"tool":"bin/tool.js"}}"#,
            &["bin/tool.js"],
        ),
        (
            "bin-array",
            r#"{"bin":["bin/a.js","bin/b.js"]}"#,
            &["bin/a.js", "bin/b.js"],
        ),
    ];

    for (tag, manifest, entry_paths) in cases {
        let mut files: Vec<(&str, &str)> = vec![("package.json", manifest)];
        for path in entry_paths {
            files.push((path, "export function start() { return 1; }\n"));
        }
        let (root, units) = scan_tree(tag, &files);
        let doc = doc_for(&units);
        for path in entry_paths {
            let module = stat(&doc, path, "<module>");
            assert_eq!(
                module.entry_kind.as_deref(),
                Some("script"),
                "{tag}: {path} is declared"
            );
            assert_eq!(
                module.entry_confidence.as_deref(),
                Some("HIGH"),
                "{tag}: {path} is HIGH"
            );
        }
        fs::remove_dir_all(&root).ok();
    }
}

#[test]
fn conventional_module_roots_are_medium_scripts() {
    let (root, units) = scan_tree(
        "entry-conventional",
        &[
            ("index.js", "export function a() { return 1; }\n"),
            ("main.js", "export function b() { return 2; }\n"),
            ("server.js", "export function c() { return 3; }\n"),
            ("app.js", "export function d() { return 4; }\n"),
        ],
    );
    let doc = doc_for(&units);
    for path in ["index.js", "main.js", "server.js", "app.js"] {
        let module = stat(&doc, path, "<module>");
        assert_eq!(module.entry_kind.as_deref(), Some("script"), "{path}");
        assert_eq!(
            module.entry_confidence.as_deref(),
            Some("MEDIUM"),
            "{path}: conventional rung is MEDIUM"
        );
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn in_source_require_main_and_import_meta_guards_are_high() {
    let (root, units) = scan_tree(
        "entry-guards",
        &[
            (
                "cjs.js",
                "function run() { return 1; }\nif (require.main === module) {\n  run();\n}\n",
            ),
            (
                "esm.js",
                "function start() { return 2; }\nif (import.meta.url === \"file://\" + process.argv[1]) {\n  start();\n}\n",
            ),
        ],
    );
    let doc = doc_for(&units);
    for path in ["cjs.js", "esm.js"] {
        let module = stat(&doc, path, "<module>");
        assert_eq!(module.entry_kind.as_deref(), Some("script"), "{path}");
        assert_eq!(
            module.entry_confidence.as_deref(),
            Some("HIGH"),
            "{path}: in-source guard rung is HIGH"
        );
    }
    fs::remove_dir_all(&root).ok();
}

#[test]
fn malformed_package_json_is_ignored_and_conventional_rung_still_fires() {
    let root = write_tree(
        "entry-malformed",
        &[
            ("package.json", "{\"main\": "),
            ("index.js", "export function start() { return 1; }\n"),
        ],
    );
    let (code, out) = run_pipeline("entry-malformed-out", &root);
    assert_eq!(code, 0, "a malformed manifest must not abort the scan");

    let doc = read_json(&out.join("metrics.json"));
    let module = json_stat(&doc, "index.js", "<module>");
    assert_eq!(
        module["entry_kind"], "script",
        "falls back to the conventional rung"
    );
    assert_eq!(module["entry_confidence"], "MEDIUM");

    fs::remove_dir_all(&root).ok();
    fs::remove_dir_all(out.parent().expect("output parent")).ok();
}

#[test]
fn no_entry_candidate_yields_unknown_entry_low_on_every_js_function() {
    let (root, units) = scan_tree(
        "entry-unknown",
        &[(
            "lib/data.js",
            "function _hidden() { return 1; }\nexport const value = 2;\n",
        )],
    );
    let doc = doc_for(&units);
    let js_stats: Vec<&FunctionStats> = doc
        .stats
        .functions
        .iter()
        .filter(|f| f.path == "lib/data.js")
        .collect();
    assert_eq!(js_stats.len(), 2, "module + _hidden");
    for f in js_stats {
        assert_eq!(
            f.entry_kind.as_deref(),
            Some("unknown-entry"),
            "{} must be the honest stub",
            f.name
        );
        assert_eq!(f.entry_confidence.as_deref(), Some("LOW"), "{}", f.name);
    }
    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (5) Loops + cost
// ---------------------------------------------------------------------------

#[test]
fn loop_trip_counts_and_cost_provenance() {
    let (root, units) = scan_tree(
        "loops",
        &[(
            "loops.js",
            "function counted() {\n  for (let i = 0; i < 10; i++) {}\n}\n\
             function inclusive() {\n  for (let i = 0; i <= 3; i++) {}\n}\n\
             function unknownLoop(xs) {\n  while (xs.length) {}\n}\n\
             function mixed(xs) {\n  for (let i = 0; i < 10; i++) {}\n  while (xs.length) {}\n}\n\
             function nested(n) {\n  for (let i = 0; i < n; i++) {\n    for (let j = 0; j < n; j++) {}\n  }\n}\n\
             function noLoops() {\n  return 1;\n}\n",
        )],
    );
    let file = unit(&units, "loops.js");

    let counted = js_function(file, "counted");
    assert_eq!(counted.loops.len(), 1);
    assert_eq!(counted.loops[0].trip_count, Some(10));

    let inclusive = js_function(file, "inclusive");
    assert_eq!(
        inclusive.loops[0].trip_count,
        Some(4),
        "`i <= 3` executes 4 times"
    );

    assert_eq!(
        js_function(file, "unknownLoop").loops[0].trip_count,
        None,
        "while is genuinely unknown"
    );

    let nested = js_function(file, "nested");
    assert_eq!(nested.record.max_loop_depth, 2);
    let depths: Vec<u32> = nested.loops.iter().map(|l| l.depth).collect();
    assert_eq!(depths, [1, 2], "nested loop depth reaches 2");

    let doc = doc_for(&units);

    let c = stat(&doc, "loops.js", "counted");
    assert_eq!(c.trip_count_source.as_deref(), Some("literal"));
    assert_eq!(c.known_trip_value, Some(10));

    let i = stat(&doc, "loops.js", "inclusive");
    assert_eq!(i.trip_count_source.as_deref(), Some("literal"));
    assert_eq!(i.known_trip_value, Some(4));

    let u = stat(&doc, "loops.js", "unknownLoop");
    assert_eq!(u.trip_count_source.as_deref(), Some("unknown"));
    assert_eq!(u.known_trip_value, None);
    assert_eq!(u.unknown_trip_loops, 1);

    let m = stat(&doc, "loops.js", "mixed");
    assert_eq!(m.trip_count_source.as_deref(), Some("partial"));
    assert_eq!(m.known_trip_value, Some(10));
    assert_eq!(m.unknown_trip_loops, 1);

    let n = stat(&doc, "loops.js", "nested");
    assert_eq!(n.max_loop_depth, 2, "metrics expose the nesting depth");
    assert!(n.function_cost_v2.is_some(), "cost v2 is always computed");

    let none = stat(&doc, "loops.js", "noLoops");
    assert_eq!(
        none.trip_count_source, None,
        "no loops ⇒ the field is omitted"
    );
    assert_eq!(none.known_trip_value, None, "no literal trip exists");

    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (6) Events / instrument
// ---------------------------------------------------------------------------

#[test]
fn allocation_error_path_and_idiom_facts_reach_the_metrics() {
    let (root, units) = scan_tree(
        "events",
        &[
            (
                "events.js",
                "function events() {\n  const a = new Foo();\n  const o = { x: 1 };\n  const arr = [1, 2];\n  return a;\n}\n\
                 function guarded() {\n  try {\n    throw new Error(\"boom\");\n  } catch (e) {\n    return e;\n  }\n}\n\
                 function family() {\n  const a = new Array(3);\n  const b = Buffer.alloc(4);\n  return JSON.stringify(a);\n}\n\
                 const s = \"\";\nfunction accumulate() {\n  let out = \"\";\n  for (let i = 0; i < 3; i++) {\n    out += \"x\";\n  }\n  return out;\n}\n",
            ),
            (
                "private_a.js",
                "class A {\n  #alpha = 1;\n  read() { return this.#alpha; }\n}\n",
            ),
            (
                "private_b.js",
                "class B {\n  #beta = 2;\n  read() { return this.#beta; }\n}\n",
            ),
        ],
    );

    let file = unit(&units, "events.js");

    let events = js_function(file, "events");
    let kinds: Vec<AllocKind> = events.allocations.iter().map(|a| a.kind).collect();
    assert_eq!(
        kinds,
        [AllocKind::New, AllocKind::Object, AllocKind::Array],
        "new + object literal + array literal"
    );
    assert_eq!(events.record.alloc_site_count, 3);

    let guarded = js_function(file, "guarded");
    assert_eq!(guarded.try_blocks.len(), 1, "try_statement event");

    let doc = doc_for(&units);

    let e = stat(&doc, "events.js", "events");
    assert_eq!(e.alloc_site_count, 3, "alloc_site_count sums all sites");
    assert_eq!(e.error_path, Some(false));
    assert!(
        e.idiom_hash.as_deref().is_some_and(|h| !h.is_empty()),
        "idiom_hash is present"
    );
    assert!(e.function_cost_v2.is_some(), "function_cost_v2 present");

    let g = stat(&doc, "events.js", "guarded");
    assert_eq!(g.error_path, Some(true), "throw/try marks the error path");
    assert!(
        g.idiom_hash.as_deref().is_some_and(|h| !h.is_empty()),
        "guarded idiom_hash present"
    );

    let f = stat(&doc, "events.js", "family");
    assert_eq!(
        f.alloc_family.as_deref(),
        Some("real_alloc"),
        "new Array(len) + Buffer.alloc dominate JSON.stringify"
    );
    assert_eq!(f.error_path, Some(false));

    let a = stat(&doc, "events.js", "accumulate");
    assert_eq!(
        a.error_path,
        Some(false),
        "`+=` is a B2-W2 rule input, not an error path"
    );
    assert!(a.function_cost_v2.is_some());

    // JS private fields (`#name`) must survive idiom normalization. Renaming
    // only the private field changes the hash; if `#` were stripped as a
    // Python-style comment, both bodies would normalize identically.
    let read_a = stat(&doc, "private_a.js", "read");
    let read_b = stat(&doc, "private_b.js", "read");
    let hash_a = read_a.idiom_hash.as_deref().expect("private_a hash");
    let hash_b = read_b.idiom_hash.as_deref().expect("private_b hash");
    assert_ne!(
        hash_a, hash_b,
        "private-field identifiers participate in the idiom hash"
    );

    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (7) Import graph
// ---------------------------------------------------------------------------

#[test]
fn relative_imports_resolve_and_the_include_graph_has_the_exact_edges() {
    let (root, units) = scan_tree(
        "imports",
        &[
            (
                "main.js",
                "import { u } from \"./util.js\";\n\
                 import missing from \"./missing.js\";\n\
                 import React from \"react\";\n\
                 export { x } from \"./x.js\";\n\
                 const y = require(\"./y.js\");\n",
            ),
            ("util.js", "export const u = 1;\n"),
            ("x.js", "export const x = 2;\n"),
            ("y.js", "module.exports = 3;\n"),
        ],
    );

    let main = unit(&units, "main.js");
    // The external bare specifier is kept as an Angle (unresolved) edge in the
    // IR; unresolved internal edges are normalized to `to_path == None`.
    let react = main
        .includes
        .iter()
        .find(|e| e.to_path.as_deref() == Some("react"))
        .expect("external bare specifier is retained as data");
    assert_eq!(react.form, IncludeForm::Angle);
    assert!(!react.resolved);

    let missing = main
        .includes
        .iter()
        .find(|e| e.form == IncludeForm::Quoted && !e.resolved)
        .expect("unresolved internal edge");
    assert_eq!(missing.to_path, None, "internal miss is normalized to null");
    assert!(
        main.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("util.js")),
        "`./util.js` resolves"
    );
    assert!(
        main.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("x.js")),
        "`export … from \"./x.js\"` resolves"
    );
    assert!(
        main.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("y.js")),
        "static `require(\"./y.js\")` resolves"
    );

    let doc = doc_for(&units);
    let graph = &doc.graphs.expect("graphs").include_graph;
    let nodes: Vec<&str> = graph.nodes.iter().map(|n| n.path.as_str()).collect();
    assert_eq!(nodes, ["main.js", "util.js", "x.js", "y.js"]);

    let edges: Vec<(&str, Option<&str>, IncludeForm, bool)> = graph
        .edges
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_deref(), e.form, e.resolved))
        .collect();
    assert_eq!(
        edges,
        [
            ("main.js", None, IncludeForm::Quoted, false),
            ("main.js", Some("util.js"), IncludeForm::Quoted, true),
            ("main.js", Some("x.js"), IncludeForm::Quoted, true),
            ("main.js", Some("y.js"), IncludeForm::Quoted, true),
        ],
        "exact admitted edge set; the external `react` edge is excluded"
    );
    assert!(
        !graph.edges.iter().any(|e| e.to.as_deref() == Some("react")),
        "bare specifiers are external and never enter the graph"
    );

    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (7b) Specifier probe order (extensionless + directory)
// ---------------------------------------------------------------------------

#[test]
fn extensionless_and_directory_imports_resolve_through_the_probe_order() {
    let (root, units) = scan_tree(
        "probe-order",
        &[
            (
                "main.js",
                "import { u } from \"./util\";\n\
                 import { l } from \"./lib\";\n",
            ),
            ("util.js", "export const u = 1;\n"),
            ("lib/index.js", "export const l = 2;\n"),
        ],
    );

    let main = unit(&units, "main.js");
    assert!(
        main.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("util.js")),
        "`./util` resolves to util.js: {:?}",
        main.includes
    );
    assert!(
        main.includes
            .iter()
            .any(|e| e.resolved && e.to_path.as_deref() == Some("lib/index.js")),
        "`./lib` resolves to lib/index.js: {:?}",
        main.includes
    );

    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (7c) Single-file policy + entry-candidate root escape
// ---------------------------------------------------------------------------

#[test]
fn single_file_minified_and_declaration_targets_are_skipped_silently() {
    for name in ["a.min.js", "b.min.mjs", "c.min.cjs", "d.bundle.js"] {
        let root = write_tree("single-min", &[(name, "var x=1;\n")]);
        let units = scan(&root.join(name)).expect("single-file scan");
        assert!(units.is_empty(), "{name} is a JS skip name");
        fs::remove_dir_all(&root).ok();
    }

    // Declaration-only files are silent; value TS files are scanned with the
    // grammar-boundary token instead of falling back to C.
    for name in ["i.d.ts", "j.d.mts", "k.d.cts"] {
        let root = write_tree("single-ts-decl", &[(name, "export type X = number;\n")]);
        let units = scan(&root.join(name)).expect("single-file scan");
        assert!(
            units.is_empty(),
            "{name} is a declaration-only file (must not fall back to C)"
        );
        fs::remove_dir_all(&root).ok();
    }
    for (name, token) in [
        ("e.ts", "ts"),
        ("f.mts", "ts"),
        ("g.cts", "ts"),
        ("h.tsx", "tsx"),
    ] {
        let root = write_tree("single-ts", &[(name, "export const x = 1;\n")]);
        let units = scan(&root.join(name)).expect("single-file scan");
        assert_eq!(units.len(), 1, "{name} must be scanned");
        assert_eq!(units[0].file.language, token, "{name} token");
        fs::remove_dir_all(&root).ok();
    }

    // Unknown extensions keep the documented C fallback.
    let root = write_tree(
        "single-unknown",
        &[("weird.xyz", "int main(void) { return 0; }\n")],
    );
    let units = scan(&root.join("weird.xyz")).expect("single-file scan");
    assert_eq!(units.len(), 1, "unknown extension is scanned as C");
    assert_eq!(units[0].file.language, "c");
    fs::remove_dir_all(&root).ok();
}

#[test]
fn single_file_js_target_reports_one_js_file() {
    let (root, _) = scan_tree("single-js", &[("src/util.js", "export const u = 1;\n")]);
    let units = scan(&root.join("src/util.js")).expect("single-file scan");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].file.path, "util.js");
    let doc = doc_for(&units);
    assert_eq!(doc.scan.languages, ["js"]);
    assert_eq!(doc.files.len(), 1);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn nested_package_json_main_cannot_escape_the_scan_root() {
    let (root, units) = scan_tree(
        "entry-escape",
        &[
            ("nested/package.json", r#"{"main":"../../outside.js"}"#),
            ("outside.js", "export function outside() { return 1; }\n"),
        ],
    );
    let doc = doc_for(&units);
    let module = stat(&doc, "outside.js", "<module>");
    assert_ne!(
        module.entry_confidence.as_deref(),
        Some("HIGH"),
        "an escaping main must not declare an out-of-scan-root file"
    );
    assert_ne!(
        module.entry_kind.as_deref(),
        Some("script"),
        "no declared-script rung may fire from an escaping main"
    );
    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (8) JS-only corpus is scored only by JS rules
// ---------------------------------------------------------------------------

#[test]
fn js_only_corpus_emits_only_js_findings() {
    let root = write_tree(
        "no-findings",
        &[
            (
                "app.js",
                "function run(xs) {\n  const out = [];\n  for (let i = 0; i < xs.length; i++) {\n    out.push(new Item(xs[i]));\n  }\n  return out;\n}\n",
            ),
            ("util.js", "export function helper(v) { return v + 1; }\n"),
        ],
    );
    let (code, out) = run_pipeline("no-findings-out", &root);
    assert_eq!(code, 0, "findings never affect the exit code");
    let doc = read_json(&out.join("metrics.json"));
    // Since W2 the JS family ships rules, so findings may exist; the intent of
    // this test is that JS units are scored ONLY by the shared PERF-JS-* family
    // — never by a same-named C/Python/Rust heuristic.
    let findings = doc["findings"].as_array().expect("findings[]");
    for finding in findings {
        let rule_id = finding["rule_id"].as_str().expect("rule_id");
        assert!(
            rule_id.starts_with("PERF-JS-"),
            "JS-only corpus produced a non-JS rule id: {rule_id}"
        );
        for other in ["PERF-C-", "PERF-PY-", "PERF-RS-", "PERF-TS-"] {
            assert!(
                !rule_id.starts_with(other),
                "JS-only corpus must not be scored by {other}: {rule_id}"
            );
        }
    }
    assert_eq!(doc["scan"]["languages"], serde_json::json!(["js"]));

    fs::remove_dir_all(&root).ok();
    fs::remove_dir_all(out.parent().expect("output parent")).ok();
}

// ---------------------------------------------------------------------------
// (8b) IR-internal facts never reach the wire
// ---------------------------------------------------------------------------

#[test]
fn metrics_json_does_not_leak_ir_internal_event_fields() {
    let root = write_tree(
        "no-wire-leak",
        &[
            ("package.json", r#"{"main":"app.js"}"#),
            (
                "app.js",
                "function f(a) {\n\
                   const o = { x: 1 };\n\
                   const arr = [1];\n\
                   delete a.prop;\n\
                   return new Foo(o, arr);\n\
                 }\n",
            ),
        ],
    );
    let (code, out) = run_pipeline("no-wire-leak-out", &root);
    assert_eq!(code, 0);
    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json");
    for needle in ["allocations", "unaries", "entry_declared"] {
        assert!(
            !raw.contains(needle),
            "`{needle}` is IR-internal and must not appear in metrics.json"
        );
    }
    fs::remove_dir_all(&root).ok();
    fs::remove_dir_all(out.parent().expect("output parent")).ok();
}

// ---------------------------------------------------------------------------
// (9) Determinism
// ---------------------------------------------------------------------------

fn collect_artifacts(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
            let entry = entry.expect("dir entry readable");
            let name = entry
                .file_name()
                .into_string()
                .expect("UTF-8 artifact name");
            let path = entry.path();
            let rel = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            if path.is_dir() {
                walk(&path, &rel, out);
            } else {
                out.insert(rel, fs::read(&path).expect("artifact readable"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, "", &mut out);
    out
}

#[test]
fn two_js_scans_are_byte_identical_modulo_provenance() {
    let root = write_tree(
        "determinism",
        &[
            (
                "main.js",
                "import { helper } from \"./helper.js\";\n\
                 function run(n) {\n  const acc = [];\n  for (let i = 0; i < n; i++) {\n    acc.push(helper(i));\n  }\n  return acc;\n}\n\
                 run(3);\n",
            ),
            (
                "helper.js",
                "export function helper(v) {\n  return v + 1;\n}\n",
            ),
        ],
    );

    let (code1, out1) = run_pipeline("determinism-a", &root);
    assert_eq!(code1, 0);
    let (code2, out2) = run_pipeline("determinism-b", &root);
    assert_eq!(code2, 0);

    let run1 = collect_artifacts(&out1);
    let run2 = collect_artifacts(&out2);

    let names1: Vec<&String> = run1.keys().collect();
    let names2: Vec<&String> = run2.keys().collect();
    assert_eq!(names1, names2, "artifact sets must match");

    for (name, bytes1) in &run1 {
        let bytes2 = &run2[name];
        match name.as_str() {
            "metrics.json" => {
                let text1 = String::from_utf8(bytes1.clone()).expect("metrics UTF-8");
                let text2 = String::from_utf8(bytes2.clone()).expect("metrics UTF-8");
                assert_eq!(
                    common::mask_metrics_provenance(&text1),
                    common::mask_metrics_provenance(&text2),
                    "metrics.json diverged beyond scan.started_at/duration_ms"
                );
            }
            "report.md" => {
                let text1 = String::from_utf8(bytes1.clone()).expect("report UTF-8");
                let text2 = String::from_utf8(bytes2.clone()).expect("report UTF-8");
                assert_eq!(
                    common::mask_report_provenance(&text1),
                    common::mask_report_provenance(&text2),
                    "report.md diverged beyond the provenance header"
                );
            }
            _ => assert_eq!(bytes1, bytes2, "{name} must be byte-identical across runs"),
        }
    }

    // Non-vacuity: the JS corpus must produce the full artifact set.
    for required in [
        "metrics.json",
        "report.md",
        "report.html",
        "findings-detail.md",
        "findings-detail.html",
        "profile-queue.md",
        "profile-queue.html",
    ] {
        assert!(run1.contains_key(required), "missing artifact {required}");
    }
    assert!(
        run1.keys().any(|n| n.starts_with("charts/")),
        "charts must be emitted"
    );

    fs::remove_dir_all(&root).ok();
    fs::remove_dir_all(out1.parent().expect("output parent")).ok();
    fs::remove_dir_all(out2.parent().expect("output parent")).ok();
}

// ---------------------------------------------------------------------------
// (10) STR-001 real-parse negative: a `+` nested in a call is not an
// accumulator assignment
// ---------------------------------------------------------------------------

#[test]
fn str_001_real_parse_ignores_a_string_plus_nested_in_a_call() {
    // Both loops use `i < xs.length` (unknown trip count) so the tiny-loop
    // floor never suppresses; both functions are top-level so the loop gate
    // applies. `s = f(s + "x")` must stay SILENT (the `+` is contained in the
    // call, not the whole RHS); `s = s + "x"` must FIRE once.
    let root = write_tree(
        "str001-nested",
        &[(
            "str.js",
            "function run(xs) {\n\
               for (let i = 0; i < xs.length; i++) {\n\
                 s = f(s + \"x\");\n\
               }\n\
               return s;\n\
             }\n\
             function runDirect(xs) {\n\
               for (let i = 0; i < xs.length; i++) {\n\
                 s = s + \"x\";\n\
               }\n\
               return s;\n\
             }\n",
        )],
    );

    let (code, out) = run_pipeline("str001-nested-out", &root);
    assert_eq!(code, 0, "findings never affect the exit code");

    let doc = read_json(&out.join("metrics.json"));
    let findings: Vec<&serde_json::Value> = doc["findings"]
        .as_array()
        .expect("findings[]")
        .iter()
        .filter(|f| f["rule_id"] == "PERF-JS-STR-001")
        .collect();

    assert_eq!(
        findings.len(),
        1,
        "only the direct accumulator fires: {findings:?}"
    );
    // The finding is the direct accumulator at line 9, inside `runDirect`
    // (the nested `s = f(s + "x")` at line 3 is silent).
    assert_eq!(findings[0]["span"]["start_line"], 9);

    // `metrics.json` `findings[].symbol` uses the aggregator's documented
    // FIRST-HIT resolution over `FunctionUnit`s; every JS file emits the
    // synthetic `<module>` record (spanning the whole file) before the real
    // functions, so that field reports `<module>` for every JS finding. The
    // ranked action list resolves the narrowest enclosing function; the full
    // detail rows still inherit `<module>` until the aggregator fix. Assert the
    // ranked-list attribution to `runDirect` here.
    assert_eq!(
        doc["parse_health_summary"]["ok"], 1,
        "fixture must parse cleanly (no dropped nested binary ops): {doc:?}"
    );
    assert_eq!(
        doc["parse_health_summary"]["partial"], 0,
        "fixture must not be partial: {doc:?}"
    );
    assert_eq!(
        doc["files"][0]["parse_health"]["error_node_count"], 0,
        "fixture must have zero error nodes: {doc:?}"
    );
    let detail = fs::read_to_string(out.join("findings-detail.md"))
        .expect("findings-detail.md must be emitted");
    assert!(
        detail.contains("| `runDirect` |"),
        "finding must be attributed to runDirect:\n{detail}"
    );

    fs::remove_dir_all(&root).ok();
    fs::remove_dir_all(out.parent().expect("output parent")).ok();
}
