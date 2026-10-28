//! TypeScript/TSX backend + wiring integration test (plan `js-ts`, phase B3).
//!
//! Exercises the real `tree-sitter-typescript` grammars through the public seam
//! (`scanner::scan`) and the real aggregator (`aggregator::assemble`) over temp
//! trees, plus full-pipeline `cli::run_with` runs for emitted documents and the
//! double-run determinism lock.
//!
//! Scope (B3-WP1): detection/tokens, discovery skips, parse health, entry
//! declarations, relative import resolution across TS/TSX probes, type-only
//! erasure, single-file policy, parameter annotations, generic-arrow parse
//! health, "only the shared PERF-JS-* family scores TS", IR-internal field
//! containment, and byte-determinism modulo the two contract-permitted
//! provenance fields.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::aggregator::{Provenance, assemble};
use perfpal::analyzer::analyze;
use perfpal::cli::run_with;
use perfpal::ir::ParseStatus;
use perfpal::scanner::{FileUnit, scan};
use perfpal::schema::{FunctionStats, MetricsDocument};

mod common;

// ---------------------------------------------------------------------------
// Temp-tree + document helpers (mirror tests/javascript_backend.rs)
// ---------------------------------------------------------------------------

fn temp_root(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-ts-itest-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp root");
    dir
}

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

fn ts_function<'a>(unit: &'a FileUnit, name: &str) -> &'a perfpal::scanner::FunctionUnit {
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

/// A committed TypeScript negative fixture (`tests/fixtures/typescript/`).
fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/typescript")
        .join(name)
}

// ---------------------------------------------------------------------------
// (1) Detection + tokens
// ---------------------------------------------------------------------------

#[test]
fn detection_selects_typescript_suffixes_with_grammar_boundary_tokens() {
    let (root, units) = scan_tree(
        "detect",
        &[
            ("a.ts", "export const a: number = 1;\n"),
            ("b.mts", "export const b: number = 1;\n"),
            ("c.cts", "module.exports = 1;\n"),
            ("d.tsx", "export const d = <div />;\n"),
            ("e.d.ts", "declare const e: number;\n"),
            ("f.d.mts", "export type F = number;\n"),
            ("g.d.cts", "export type G = number;\n"),
        ],
    );

    let paths: Vec<&str> = units.iter().map(|u| u.file.path.as_str()).collect();
    assert_eq!(
        paths,
        ["a.ts", "b.mts", "c.cts", "d.tsx"],
        "value TS files are discovered; declaration files are invisible; sorted"
    );
    assert!(paths.windows(2).all(|w| w[0] <= w[1]), "sorted emission");

    let tokens: Vec<&str> = units.iter().map(|u| u.file.language.as_str()).collect();
    assert_eq!(tokens, ["ts", "ts", "ts", "tsx"]);

    fs::remove_dir_all(&root).ok();
}

#[test]
fn mixed_c_typescript_and_tsx_tree_reports_all_tokens() {
    let (root, units) = scan_tree(
        "mixed",
        &[
            ("a.c", "int main(void) { return 0; }\n"),
            ("b.ts", "export const b: number = 1;\n"),
            ("c.tsx", "export const c = <div />;\n"),
        ],
    );
    let doc = doc_for(&units);
    assert_eq!(doc.scan.languages, ["c", "ts", "tsx"]);

    let langs: Vec<&str> = doc.files.iter().map(|f| f.language.as_str()).collect();
    assert_eq!(langs, ["c", "ts", "tsx"]);
    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (2) Discovery skips
// ---------------------------------------------------------------------------

#[test]
fn discovery_skips_apply_to_typescript_and_tsx() {
    let big_ts = format!(
        "export const x: number = 1;\n//{}",
        "x".repeat(2 * 1024 * 1024)
    );
    let big_c = format!(
        "int main(void) {{ return 0; }}\n/*{}*/",
        "c".repeat(2 * 1024 * 1024)
    );

    let (root, units) = scan_tree(
        "skips",
        &[
            ("normal.ts", "export const n: number = 1;\n"),
            ("view.tsx", "export const v = <div />;\n"),
            ("a.min.js", "var a=1;\n"),
            ("big.ts", &big_ts),
            ("big.c", &big_c),
            ("node_modules/dep.ts", "export const d: number = 1;\n"),
            ("dist/out.tsx", "export const o = <div />;\n"),
            ("build/out.ts", "export const b: number = 2;\n"),
            ("out/out.ts", "export const o2: number = 3;\n"),
            ("coverage/out.ts", "export const c2: number = 4;\n"),
            ("bower_components/dep.tsx", "export const b2 = <div />;\n"),
        ],
    );

    let paths: Vec<&str> = units.iter().map(|u| u.file.path.as_str()).collect();
    assert_eq!(
        paths,
        ["big.c", "normal.ts", "view.tsx"],
        "TS minified/oversized and skipped-directory files do not survive"
    );
    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (3) Parse health
// ---------------------------------------------------------------------------

#[test]
fn malformed_typescript_is_partial_and_clean_typescript_is_ok() {
    let (root, units) = scan_tree(
        "health",
        &[
            ("clean.ts", "export function f(): number { return 1; }\n"),
            ("broken.ts", "interface Broken {\n  foo: ;\n}\n"),
        ],
    );

    let broken = unit(&units, "broken.ts");
    assert_eq!(broken.parse_health.status, ParseStatus::Partial);
    assert!(broken.parse_health.error_node_count > 0);
    assert!(!broken.parse_health.error_regions.is_empty());

    let clean = unit(&units, "clean.ts");
    assert_eq!(clean.parse_health.status, ParseStatus::Ok);
    assert_eq!(clean.parse_health.error_node_count, 0);

    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (4) Entry declarations
// ---------------------------------------------------------------------------

#[test]
fn package_json_main_index_ts_is_a_high_confidence_script_entry() {
    let (root, units) = scan_tree(
        "entry-ts",
        &[
            ("package.json", r#"{"main":"src/index.ts"}"#),
            (
                "src/index.ts",
                "export function start(): number { return 1; }\n",
            ),
        ],
    );
    let doc = doc_for(&units);
    let module = stat(&doc, "src/index.ts", "<module>");
    assert_eq!(module.entry_kind.as_deref(), Some("script"));
    assert_eq!(module.entry_confidence.as_deref(), Some("HIGH"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn package_json_main_entry_mts_is_a_high_confidence_script_entry() {
    let (root, units) = scan_tree(
        "entry-mts",
        &[
            ("package.json", r#"{"main":"src/cli.mts"}"#),
            (
                "src/cli.mts",
                "export function start(): number { return 1; }\n",
            ),
        ],
    );
    let doc = doc_for(&units);
    let module = stat(&doc, "src/cli.mts", "<module>");
    assert_eq!(module.entry_kind.as_deref(), Some("script"));
    assert_eq!(module.entry_confidence.as_deref(), Some("HIGH"));
    fs::remove_dir_all(&root).ok();
}

#[test]
fn conventional_tsx_root_is_a_medium_script_entry() {
    let (root, units) = scan_tree(
        "entry-conventional-tsx",
        &[("app.tsx", "export const App = () => <div />;\n")],
    );
    let doc = doc_for(&units);
    let module = stat(&doc, "app.tsx", "<module>");
    assert_eq!(module.entry_kind.as_deref(), Some("script"));
    assert_eq!(module.entry_confidence.as_deref(), Some("MEDIUM"));
    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (5) Import graph
// ---------------------------------------------------------------------------

#[test]
fn relative_imports_resolve_across_ts_tsx_and_index_probes() {
    let (root, units) = scan_tree(
        "imports",
        &[
            (
                "main.ts",
                "import { u } from \"./util\";\n\
                 import { C } from \"./comp\";\n\
                 import { l } from \"./lib\";\n\
                 import type { T } from \"./types\";\n",
            ),
            ("util.ts", "export const u: number = 1;\n"),
            ("comp.tsx", "export const C = () => <div />;\n"),
            ("lib/index.ts", "export const l: number = 2;\n"),
            ("types.ts", "export type T = number;\n"),
        ],
    );

    let main = unit(&units, "main.ts");
    let resolved: Vec<&str> = main
        .includes
        .iter()
        .filter(|e| e.resolved)
        .filter_map(|e| e.to_path.as_deref())
        .collect();
    assert_eq!(
        resolved,
        ["util.ts", "comp.tsx", "lib/index.ts"],
        "`.ts` then `.tsx` then `/index.ts` probes resolve: {:?}",
        main.includes
    );
    assert!(
        !main
            .includes
            .iter()
            .any(|e| e.to_path.as_deref() == Some("types.ts")),
        "`import type` must not produce a resolved edge"
    );

    let doc = doc_for(&units);
    let graph = &doc.graphs.expect("graphs").include_graph;
    assert!(
        !graph
            .edges
            .iter()
            .any(|e| e.to.as_deref() == Some("types.ts")),
        "type-only imports are erased from the include graph"
    );
    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (6) Single-file policy
// ---------------------------------------------------------------------------

#[test]
fn single_file_typescript_is_scanned_and_declarations_are_skipped() {
    let root = write_tree(
        "single-ts",
        &[("src/app.ts", "export const x: number = 1;\n")],
    );
    let units = scan(&root.join("src/app.ts")).expect("single-file scan");
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].file.path, "app.ts");
    assert_eq!(units[0].file.language, "ts");
    fs::remove_dir_all(&root).ok();

    let root = write_tree("single-decl", &[("app.d.ts", "declare const x: number;\n")]);
    let units = scan(&root.join("app.d.ts")).expect("single-file scan");
    assert!(units.is_empty(), "declaration files are skipped silently");
    fs::remove_dir_all(&root).ok();

    let root = write_tree(
        "single-unknown",
        &[("weird.xyz", "int main(void) { return 0; }\n")],
    );
    let units = scan(&root.join("weird.xyz")).expect("single-file scan");
    assert_eq!(units.len(), 1, "unknown extensions keep the C fallback");
    assert_eq!(units[0].file.language, "c");
    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (7) Parameter annotations reach the IR/metrics
// ---------------------------------------------------------------------------

#[test]
fn typescript_param_types_and_count_reach_the_metrics() {
    let (root, units) = scan_tree(
        "params",
        &[(
            "params.ts",
            "export function f(this: Foo, a: string, b?: number, ...rest: any[]) {\n  return a;\n}\n",
        )],
    );
    let file = unit(&units, "params.ts");
    let f = ts_function(file, "f");
    assert_eq!(f.record.param_count, 3, "{:?}", f.record.param_types);
    assert_eq!(f.record.param_types, ["string", "number", "any[]"]);

    let doc = doc_for(&units);
    let metrics = stat(&doc, "params.ts", "f");
    assert_eq!(metrics.param_count, 3);
    fs::remove_dir_all(&root).ok();
}

// ---------------------------------------------------------------------------
// (8) TSX generic-arrow parse health
// ---------------------------------------------------------------------------

#[test]
fn tsx_generic_arrow_disambiguation_parse_status_is_recorded_honestly() {
    let (root, units) = scan_tree(
        "generic-arrow",
        &[
            ("comma.tsx", "const id = <T,>(x: T) => x;\n"),
            ("bare.tsx", "const id = <T>(x: T) => x;\n"),
        ],
    );

    let comma = unit(&units, "comma.tsx");
    assert_eq!(
        comma.parse_health.status,
        ParseStatus::Ok,
        "`<T,>` is the canonical TSX disambiguation"
    );

    // Observed on the pinned TSX grammar: BOTH forms parse cleanly at
    // expression position — the grammar resolves `<T>` after `=` as a
    // type-parameter list (the canonical portable form stays `<T,>`). This is
    // recorded, not assumed: if a grammar upgrade regresses it, this test
    // fails and the accepted limitation (R-tb-010) is re-worded.
    let bare = unit(&units, "bare.tsx");
    assert_eq!(
        bare.parse_health.status,
        ParseStatus::Ok,
        "pinned grammar resolves `<T>(x: T) => x` at expression position"
    );
    assert_eq!(bare.parse_health.error_node_count, 0);
    for path in ["comma.tsx", "bare.tsx"] {
        let arrow = ts_function(unit(&units, path), "id");
        assert_eq!(arrow.record.param_count, 1, "{path}");
        assert_eq!(arrow.record.param_types, ["T"], "{path}");
    }

    let (code, out) = run_pipeline("generic-arrow-out", &root);
    assert_eq!(code, 0);
    let doc = read_json(&out.join("metrics.json"));
    let findings = doc["findings"].as_array().expect("findings[]");
    assert!(
        findings.is_empty(),
        "generic arrow identities must not fire a rule: {findings:?}"
    );

    fs::remove_dir_all(&root).ok();
    fs::remove_dir_all(out.parent().expect("output parent")).ok();
}

// ---------------------------------------------------------------------------
// (9) TS corpus is scored only by the shared JS family
// ---------------------------------------------------------------------------

#[test]
fn typescript_corpus_is_scored_only_by_the_js_family() {
    // Non-vacuous: the corpus contains one KNOWN finding (LOOP-007) and one
    // silent JSX module, so an empty `findings[]` fails the pin below.
    let root = write_tree(
        "family-scoring",
        &[
            (
                "app.ts",
                "export function hits(ids: string[], seen: string[]): string[] {\n\
                   const out: string[] = [];\n\
                   for (const id of ids) {\n\
                     if (seen.includes(id)) {\n\
                       out.push(id);\n\
                     }\n\
                   }\n\
                   return out;\n\
                 }\n",
            ),
            (
                "view.tsx",
                "export function View(props: { label: string }) {\n  return <div>{props.label}</div>;\n}\n",
            ),
        ],
    );
    let (code, out) = run_pipeline("family-scoring-out", &root);
    assert_eq!(code, 0, "findings never affect the exit code");
    let doc = read_json(&out.join("metrics.json"));
    let findings = doc["findings"].as_array().expect("findings[]");
    assert_eq!(findings.len(), 1, "the known LOOP-007 hit must appear");
    assert_eq!(findings[0]["rule_id"], "PERF-JS-LOOP-007");
    for finding in findings {
        let rule_id = finding["rule_id"].as_str().expect("rule_id");
        assert!(
            rule_id.starts_with("PERF-JS-"),
            "TS corpus produced a non-JS rule id: {rule_id}"
        );
        for other in ["PERF-C-", "PERF-PY-", "PERF-RS-", "PERF-TS-"] {
            assert!(
                !rule_id.starts_with(other),
                "TS corpus must not be scored by {other}: {rule_id}"
            );
        }
    }
    assert_eq!(doc["scan"]["languages"], serde_json::json!(["ts", "tsx"]));

    fs::remove_dir_all(&root).ok();
    fs::remove_dir_all(out.parent().expect("output parent")).ok();
}

// ---------------------------------------------------------------------------
// (10) IR-internal facts never reach the wire
// ---------------------------------------------------------------------------

#[test]
fn metrics_json_does_not_leak_ir_internal_event_fields_for_ts() {
    let root = write_tree(
        "no-wire-leak",
        &[
            ("package.json", r#"{"main":"app.ts"}"#),
            (
                "app.ts",
                "export function f(a: any): unknown {\n\
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
// (11) Determinism
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
fn two_typescript_scans_are_byte_identical_modulo_provenance() {
    let root = write_tree(
        "determinism",
        &[
            (
                "main.ts",
                "import { helper } from \"./helper\";\n\
                 export function run(n: number): number[] {\n\
                   const acc: number[] = [];\n\
                   for (let i = 0; i < n; i++) {\n\
                     acc.push(helper(i));\n\
                   }\n\
                   return acc;\n\
                 }\n\
                 run(3);\n",
            ),
            (
                "helper.ts",
                "export function helper(v: number): number {\n  return v + 1;\n}\n",
            ),
            (
                "view.tsx",
                "export const View = (props: { label: string }) => <div>{props.label}</div>;\n",
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

    for required in [
        "metrics.json",
        "report.md",
        "report.html",
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
// (12) Type-land-only fixture (committed negative corpus)
// ---------------------------------------------------------------------------

#[test]
fn type_only_file_has_no_runtime_events_or_function_records() {
    let units = scan(&fixture_path("type_only.ts")).expect("scan type_only.ts");
    assert_eq!(units.len(), 1, "single-file scan yields one unit");
    let unit = &units[0];
    assert_eq!(unit.file.language, "ts");
    assert_eq!(
        unit.functions.len(),
        1,
        "interfaces/types/`declare` produce no function record: {:?}",
        unit.functions
            .iter()
            .map(|f| f.record.name.as_str())
            .collect::<Vec<_>>()
    );
    assert_eq!(unit.functions[0].record.name, "<module>");
    assert_eq!(unit.file.function_count, 1);

    let module = &unit.functions[0];
    assert!(module.calls.is_empty(), "type land has no calls");
    assert!(
        module.allocations.is_empty(),
        "type land has no allocations"
    );
    assert!(module.loops.is_empty(), "type land has no loops");
    assert!(
        module.assignments.is_empty(),
        "type land has no assignments"
    );

    assert!(
        unit.structs.is_empty(),
        "a `declare class` must not become a StructDef: {:?}",
        unit.structs
    );
    assert!(unit.includes.is_empty());
    assert!(
        analyze(&units).is_empty(),
        "a type-land-only module must produce zero findings"
    );
}

// ---------------------------------------------------------------------------
// (13) `import type` / `export type` erase to zero IncludeEdges
// ---------------------------------------------------------------------------

#[test]
fn import_type_and_export_type_produce_no_include_edges() {
    let units = scan(&fixture_path("type_edges.ts")).expect("scan type_edges.ts");
    assert_eq!(units.len(), 1);
    let unit = &units[0];
    assert!(
        unit.includes.is_empty(),
        "`import type`/`export type` must not mint edges: {:?}",
        unit.includes
    );
    assert_eq!(unit.file.include_counts.quoted, 0);
    assert_eq!(unit.file.include_counts.angle, 0);
}

// ---------------------------------------------------------------------------
// (14) `typeof import("./x")` is a type position, not a dynamic import
// ---------------------------------------------------------------------------

#[test]
fn typeof_import_type_position_produces_no_finding_even_when_partial() {
    let units = scan(&fixture_path("typeof_import.ts")).expect("scan typeof_import.ts");
    assert_eq!(units.len(), 1);
    let unit = &units[0];
    assert!(
        unit.includes.is_empty(),
        "a `typeof import(...)` type query must not mint an IncludeEdge: {:?}",
        unit.includes
    );
    // Parse health is recorded honestly rather than forced: if a grammar
    // upgrade degrades this type position, the file may be `Partial` — the
    // zero-findings assertion below still holds.
    assert!(
        matches!(
            unit.parse_health.status,
            ParseStatus::Ok | ParseStatus::Partial
        ),
        "unexpected parse status {:?}",
        unit.parse_health.status
    );
    assert_eq!(
        unit.parse_health.error_node_count == 0,
        unit.parse_health.status == ParseStatus::Ok,
        "parse status must agree with the error-node count"
    );
    assert_eq!(unit.functions.len(), 1);
    assert_eq!(unit.functions[0].record.name, "<module>");
    assert!(
        unit.functions[0].calls.is_empty(),
        "a type-position import must not become a CallEvent: {:?}",
        unit.functions[0].calls
    );
    assert!(unit.functions[0].allocations.is_empty());
    assert!(unit.functions[0].loops.is_empty());
    assert!(
        analyze(&units).is_empty(),
        "the `typeof import` type position must produce no finding"
    );
}

// ---------------------------------------------------------------------------
// (10) CALL-001 real-parse negative: a TS-wrapped member self-call is not a
// bare direct recursion
// ---------------------------------------------------------------------------

#[test]
fn call_001_real_parse_excludes_wrapped_member_receivers() {
    let (root, units) = scan_tree(
        "call001-ts",
        &[(
            "service.ts",
            "export function dispose(store) {\n  return (this.storage as any).dispose(store);\n}\n",
        )],
    );

    let file = unit(&units, "service.ts");
    assert_eq!(file.parse_health.status, ParseStatus::Ok);
    assert_eq!(file.parse_health.error_node_count, 0);

    // `callee_name` is `dispose` (== the enclosing function name) and the
    // receiver root is unresolvable, but the callee path is the TS-wrapped
    // member chain `(this.storage as any).dispose`, not a bare `dispose`.
    let findings: Vec<_> = analyze(&units)
        .into_iter()
        .filter(|f| f.rule_id == "PERF-JS-CALL-001")
        .collect();
    assert!(
        findings.is_empty(),
        "wrapped member self-call must stay silent: {findings:?}"
    );

    let doc = doc_for(&units);
    let summary = doc.parse_health_summary.expect("parse_health_summary");
    assert_eq!(summary.ok, 1, "fixture must parse cleanly");
    assert_eq!(summary.partial, 0, "fixture must not be partial");

    fs::remove_dir_all(&root).ok();
}

/// B5 review note R1: the fresh-object gate must see through TS transparent
/// wrappers around the initializer (`const x = { … } as T` is a fresh literal).
#[test]
fn type_001_fresh_object_gate_strips_ts_wrappers() {
    let (root, units) = scan_tree(
        "type001-ts-fresh",
        &[(
            "fresh.ts",
            "export function sanitize(config: Record<string, string>) {\n  const merged = { ...config } as Record<string, string>;\n  delete merged.unsafe;\n}\n",
        )],
    );

    let file = unit(&units, "fresh.ts");
    assert_eq!(file.parse_health.status, ParseStatus::Ok);
    assert_eq!(file.parse_health.error_node_count, 0);

    let findings: Vec<_> = analyze(&units)
        .into_iter()
        .filter(|f| f.rule_id == "PERF-JS-TYPE-001")
        .collect();
    assert!(
        findings.is_empty(),
        "a fresh TS-wrapped object literal must stay silent: {findings:?}"
    );

    fs::remove_dir_all(&root).ok();
}
