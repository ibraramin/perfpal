//! Semantic-resolver Wave A locks (plan `semantic-resolver`, modules.md
//! §4.3): honest tier reporting in ALL builds, identity-key stability, the
//! schema v1.1 additive surface, and — behind `#[cfg(feature =
//! "semantic-clang")]` + `PERFPAL_SEMANTIC_TESTS=1` — real-libclang parses.
//!
//! Default gate (no feature, no libclang needed): everything in this file
//! outside the `semantic` module runs, including the golden-schema negative
//! test.
//!
//! Feature gate: the `semantic` module's tests run ONLY when BOTH the
//! `semantic-clang` cargo feature is on AND `PERFPAL_SEMANTIC_TESTS=1` (set by
//! the CI `semantic-features` job with `libclang-dev` installed). Locally,
//! without libclang, they SKIP with an eprintln note — never fail
//! (graceful absence is a first-class path).
//!
//! Concurrency note: scan+resolve carry no process-global state (the
//! resolver takes an explicit root — no scan anchor — and libclang handles
//! are thread-local with a fresh `CXIndex` per translation unit), so tests
//! in this binary run unserialized.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::cli::run_with;
use perfpal::ir::{ResolverHealth, ResolverTier};
use perfpal::{resolver, scanner};

/// The committed fixture repo (shared with the P6 goldens).
fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/e2e")
}

/// Canonicalized scan root threaded explicitly into every [`resolver::resolve`]
/// call in this file (the resolver's explicit-root contract: no globals).
fn canonical_root(path: &Path) -> PathBuf {
    path.canonicalize()
        .unwrap_or_else(|e| panic!("scan root must canonicalize: {}: {e}", path.display()))
}

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-resolver-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn argv(args: &[&str]) -> Vec<std::ffi::OsString> {
    std::iter::once(std::ffi::OsString::from("perfpal"))
        .chain(args.iter().map(OsString::from))
        .collect()
}

/// HARD identity guard (modules.md §4.3): the resolver must never change any
/// `(path, name, span)` identity key, and may only write the sanctioned
/// enrichment surface — `parse_health.resolver` (not compared here),
/// `CallEvent.semantic`, `FunctionRecord.resolver_tier` — which this helper
/// normalizes away before comparing. Everything else (event tiers, loop
/// tiers, spans, counts, edges) must be byte-stable. Every pre/post
/// assertion in this file goes through this helper so the contract has
/// exactly one enforcement point; fact PRESENCE is locked separately (the
/// strict default-only test below + the resolved-edge feature test).
fn assert_identity_stable(before: &[scanner::FileUnit], after: &[scanner::FileUnit]) {
    assert_eq!(before.len(), after.len(), "file count changed");
    for (b, a) in before.iter().zip(after.iter()) {
        assert_eq!(b.file, a.file, "FileRecord changed for {}", b.file.path);
        assert_eq!(
            b.parse_health.status, a.parse_health.status,
            "parse status changed for {}",
            b.file.path
        );
        assert_eq!(
            b.parse_health.error_regions, a.parse_health.error_regions,
            "error regions changed for {}",
            b.file.path
        );
        assert_eq!(
            b.parse_health.error_node_count, a.parse_health.error_node_count,
            "error node count changed for {}",
            b.file.path
        );
        assert_eq!(b.includes, a.includes, "include edges changed");
        assert_eq!(b.global_vars, a.global_vars, "global vars changed");
        assert_eq!(b.structs, a.structs, "structs changed");
        assert_eq!(b.macros, a.macros, "macros changed");
        assert_eq!(
            b.functions.len(),
            a.functions.len(),
            "function count changed"
        );
        for (fb, fa) in b.functions.iter().zip(a.functions.iter()) {
            assert_eq!(
                (
                    fb.record.path.clone(),
                    fb.record.name.clone(),
                    fb.record.span
                ),
                (
                    fa.record.path.clone(),
                    fa.record.name.clone(),
                    fa.record.span
                ),
                "identity key (path,name,span) changed for {}",
                fb.record.name
            );
            // Sanctioned enrichment only: a resolved definition record may
            // carry resolver_tier Semantic; normalize before comparing.
            let mut fa_record = fa.record.clone();
            fa_record.resolver_tier = ResolverTier::Syntactic;
            assert_eq!(
                fb.record, fa_record,
                "FunctionRecord payload changed beyond resolver_tier for {}",
                fb.record.name
            );
            // Sanctioned enrichment only: resolved calls may carry semantic
            // facts; event tiers must never move (covered by this compare).
            let mut fa_calls = fa.calls.clone();
            for call in &mut fa_calls {
                call.semantic = None;
            }
            assert_eq!(
                fb.calls, fa_calls,
                "call events changed beyond semantic facts"
            );
            assert_eq!(fb.assignments, fa.assignments, "assignments changed");
            assert_eq!(fb.binary_ops, fa.binary_ops, "binary ops changed");
            assert_eq!(fb.comparisons, fa.comparisons, "comparisons changed");
            assert_eq!(fb.access_chains, fa.access_chains, "access chains changed");
            assert_eq!(fb.loops, fa.loops, "loops changed");
        }
    }
}

// ===========================================================================
// Default gate — no libclang, no feature
// ===========================================================================

/// Honest tier reporting in all builds: WITHOUT the `semantic-clang` feature
/// every file reports `absent`. (The feature-enabled counterpart lives in the
/// env-gated `semantic` module below.)
#[test]
#[cfg(not(feature = "semantic-clang"))]
fn resolver_reports_absent_without_feature() {
    let fixture = fixture_root();
    let mut units = scanner::scan(&fixture).expect("e2e fixture scans");
    assert!(!units.is_empty());
    resolver::resolve(&mut units, &canonical_root(&fixture));
    for unit in &units {
        assert_eq!(
            unit.parse_health.resolver,
            Some(ResolverHealth::Absent),
            "{} must report honest absence without the semantic-clang feature",
            unit.file.path
        );
    }
}

#[test]
fn resolve_never_touches_identity_or_heuristic_state() {
    let fixture = fixture_root();
    let units = scanner::scan(&fixture).expect("e2e fixture scans");
    let mut resolved = units.clone();
    resolver::resolve(&mut resolved, &canonical_root(&fixture));
    assert_identity_stable(&units, &resolved);
}

/// STRICT default-build contract: without the `semantic-clang` feature the
/// resolver must set NO enrichment facts anywhere (the normalizing helper
/// above would mask a spurious write, so this test pins the absence).
#[test]
#[cfg(not(feature = "semantic-clang"))]
fn no_enrichment_facts_without_feature() {
    let fixture = fixture_root();
    let mut units = scanner::scan(&fixture).expect("e2e fixture scans");
    resolver::resolve(&mut units, &canonical_root(&fixture));
    for unit in &units {
        for func in &unit.functions {
            assert_eq!(
                func.record.resolver_tier,
                ResolverTier::Syntactic,
                "record tier moved without the feature: {}",
                func.record.name
            );
            for call in &func.calls {
                assert!(
                    call.semantic.is_none(),
                    "semantic fact set without the feature: {}",
                    call.callee_name
                );
            }
        }
    }
}

/// Full-pipeline proof on the DEFAULT build (validation item 8): metrics.json
/// carries schema_version 1.2 and `files[].parse_health.resolver = "absent"`,
/// and the bundled schema uses the full v1.2 artifact name.
#[test]
#[cfg(not(feature = "semantic-clang"))]
fn e2e_metrics_show_schema_1_2_and_absent_resolver() {
    let out = temp_dir("e2e11").join("perfpal-report");
    let code = run_with(argv(&[
        "--output",
        out.to_string_lossy().as_ref(),
        fixture_root().to_string_lossy().as_ref(),
    ]));
    assert_eq!(code, 0, "pipeline over the e2e fixture must succeed");
    let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
    let doc: serde_json::Value = serde_json::from_str(&raw).expect("metrics.json parses");
    assert_eq!(doc["schema_version"], "1.2");
    let files = doc["files"].as_array().expect("files[] present");
    assert!(!files.is_empty());
    for file in files {
        assert_eq!(
            file["parse_health"]["resolver"], "absent",
            "default build must emit honest resolver absence"
        );
    }
    assert!(out.join("metrics.schema.v1.2.json").is_file());
    assert!(!out.join("metrics.schema.v1.1.json").exists());
}

/// NEGATIVE TEST (metrics-json-v1.md §5 minor-bump policy): a v1.0-style
/// document — `parse_health` WITHOUT the `resolver` field — must still
/// validate under the v1.2 schema. The repo carries no runtime JSON-Schema
/// validator dependency, so the validation contract is asserted
/// structurally against the embedded canonical v1.2 schema: `resolver` is
/// optional (not in `required`), its enum is exactly the documented set, the
/// v1.0-shaped object's keys all exist as schema properties (no
/// `additionalProperties` violation possible), and the widened call-edge
/// confidence still accepts every v1.0 value.
#[test]
fn v1_0_style_parse_health_still_validates_under_v1_2_schema() {
    let schema: serde_json::Value =
        serde_json::from_str(perfpal::schema::EMBEDDED_SCHEMA).expect("embedded schema parses");

    // (1) resolver is additive-optional: NOT in parse_health.required.
    let required = schema["properties"]["files"]["items"]["properties"]["parse_health"]["required"]
        .as_array()
        .expect("parse_health.required present");
    assert!(!required.iter().any(|v| v == "resolver"));

    // (2) resolver's enum is exactly the documented set.
    let resolver_schema = &schema["properties"]["files"]["items"]["properties"]["parse_health"]["properties"]
        ["resolver"];
    let tokens: Vec<&str> = resolver_schema["enum"]
        .as_array()
        .expect("resolver.enum present")
        .iter()
        .map(|v| v.as_str().expect("enum token"))
        .collect();
    assert_eq!(tokens, ["absent", "ok", "failed"]);

    // (3) a v1.0-style parse_health object (no resolver) only uses keys the
    // v1.2 schema declares — so `additionalProperties: false` cannot reject
    // it; and omitting the optional `resolver` cannot violate `required`.
    let v1_style = serde_json::json!({
        "status": "ok",
        "error_node_count": 0
    });
    let properties =
        schema["properties"]["files"]["items"]["properties"]["parse_health"]["properties"]
            .as_object()
            .expect("parse_health.properties present");
    for key in v1_style.as_object().expect("object").keys() {
        assert!(properties.contains_key(key), "undeclared key {key}");
    }

    // (4) the call-edge confidence widening keeps every v1.0 value valid:
    // oneOf must admit "LOW" (the only value v1.0 could emit).
    let confidence =
        schema["properties"]["graphs"]["properties"]["call_graph"]["properties"]["edges"]["items"]
            ["properties"]["confidence"]
            .clone();
    let admits = |token: &str| {
        confidence["oneOf"]
            .as_array()
            .expect("confidence.oneOf present")
            .iter()
            .any(|v| v["const"] == token)
    };
    for token in ["LOW", "MEDIUM", "HIGH"] {
        assert!(admits(token), "confidence oneOf must admit {token}");
    }
    assert!(
        confidence.get("const").is_none(),
        "must no longer be const LOW"
    );
}

// ===========================================================================
// Feature gate — real libclang parses, only under BOTH conditions:
//   1. cargo feature `semantic-clang`
//   2. env PERFPAL_SEMANTIC_TESTS=1
// Any other combination SKIPS with a note (never fails).
// ===========================================================================

#[cfg(feature = "semantic-clang")]
mod semantic {
    use super::*;

    /// Skip gate: PERFPAL_SEMANTIC_TESTS=1 AND a loadable libclang. Every
    /// test in this module calls this first and returns early on skip.
    fn semantic_ready() -> bool {
        if std::env::var("PERFPAL_SEMANTIC_TESTS").ok().as_deref() != Some("1") {
            eprintln!(
                "perfpal: skipping semantic-resolver real-parse tests \
                 (PERFPAL_SEMANTIC_TESTS != 1)"
            );
            return false;
        }
        if let Err(err) = clang_sys::load() {
            eprintln!(
                "perfpal: skipping semantic-resolver real-parse tests \
                 (libclang unavailable: {err})"
            );
            return false;
        }
        true
    }

    #[test]
    fn real_parse_reports_ok_on_valid_c_and_keeps_identity() {
        if !semantic_ready() {
            return;
        }
        let fixture = fixture_root();
        let units = scanner::scan(&fixture).expect("e2e fixture scans");
        let mut resolved = units.clone();
        resolver::resolve(&mut resolved, &canonical_root(&fixture));
        assert_identity_stable(&units, &resolved);
        for unit in &resolved {
            assert_eq!(
                unit.parse_health.resolver,
                Some(ResolverHealth::Ok),
                "{} parsed by libclang must report Ok",
                unit.file.path
            );
        }
    }

    #[test]
    fn broken_c_reports_failed_honestly_and_keeps_syntactic_results() {
        if !semantic_ready() {
            return;
        }
        let root = temp_dir("broken");
        fs::write(
            root.join("broken.c"),
            "int main( { this is $ not C <<\nint half(;\n",
        )
        .expect("write broken fixture");
        let units = scanner::scan(&root).expect("broken fixture scans (tree-sitter best effort)");
        let mut resolved = units.clone();
        resolver::resolve(&mut resolved, &canonical_root(&root));
        assert_identity_stable(&units, &resolved);

        let unit = &resolved[0];
        // Honest tier reporting per CXErrorCode: a deliberately broken C
        // file yields Ok (KeepGoing best-effort parse with in-AST errors) or
        // Failed (TU-level error code) — NEVER None/Absent, and the
        // resolver's verdict never erases the tree-sitter truth below.
        let health = unit.parse_health.resolver;
        assert!(
            health == Some(ResolverHealth::Ok) || health == Some(ResolverHealth::Failed),
            "broken TU must report Ok-or-Failed honestly, got {health:?}"
        );
        // Syntactic results are NEVER deleted/degraded by the resolver.
        assert_eq!(
            unit.parse_health.status, units[0].parse_health.status,
            "tree-sitter status must survive untouched"
        );
    }

    #[test]
    fn maps_build_deterministically_over_the_same_tu() {
        if !semantic_ready() {
            return;
        }
        let fixture = fixture_root();
        let units = scanner::scan(&fixture).expect("e2e fixture scans");
        // The resolver's explicit root is this test's own canonicalized
        // fixture root — no process-global anchor involved.
        let root = canonical_root(&fixture);
        let main = units
            .iter()
            .find(|u| u.file.path.ends_with("main.c"))
            .expect("e2e fixture has main.c");
        let abs = root.join(&main.file.path);
        let source = fs::read(&abs).expect("re-read main.c");

        let rel = main.file.path.clone();
        let maps1 = resolver::parse_unit_maps(&rel, &abs, &source, root.as_path())
            .expect("TU parses into maps");
        let maps2 = resolver::parse_unit_maps(&rel, &abs, &source, root.as_path())
            .expect("TU parses into maps (repeat run)");
        let (defs1, refs1) = (maps1.definitions, maps1.call_refs);
        let (defs2, refs2) = (maps2.definitions, maps2.call_refs);
        assert_eq!(
            maps1.function_facts, maps2.function_facts,
            "function-facts map must be byte-stable across runs"
        );
        assert_eq!(
            defs1, defs2,
            "definition map must be byte-stable across runs"
        );
        assert_eq!(
            refs1, refs2,
            "call-reference map must be byte-stable across runs"
        );

        // Non-empty and key-ordered (determinism contract).
        assert!(!defs1.is_empty(), "main.c defines functions");
        assert!(!refs1.is_empty(), "main.c makes calls");
        assert!(defs1.keys().is_sorted());
        assert!(refs1.keys().is_sorted());

        // Path sanity: the main TU's own definitions are repo-relative; the
        // include closure may ALSO contribute system-header definitions
        // (absolute /usr/include paths from the default libclang search) —
        // both are legitimate map data; USRs never appear as paths
        // (ADR-0001 identity rule).
        assert!(
            defs1.values().any(|site| site.path == rel),
            "main TU's own definitions must be mapped repo-relative"
        );
        for site in defs1.values() {
            assert!(!site.path.contains("USR"));
            assert!(
                site.path.starts_with("src/") || site.path.starts_with('/'),
                "unexpected path {}",
                site.path
            );
        }

        // Cross-file pairing NOTE (Wave B): util.c is a separate TU — its
        // definitions map under their own parse, not this one. This test
        // only locks per-TU map determinism.
    }

    /// Wave B1 end-to-end lock: the e2e main.c→util.c edge resolves via USR    /// pairing across TUs ⇒ MEDIUM-confidence semantic ARCH-003 finding +
    /// MEDIUM call-graph edge. This is exactly the behavior the P6 golden
    /// comparator neutralizes (comparison-time only) — locked for real here.
    #[test]
    fn resolved_cross_file_edge_upgrades_arch003_to_medium() {
        if !semantic_ready() {
            return;
        }
        // Mutation determinism first: two resolves of the same scan must
        // agree on every fact, not just the maps.
        let fixture = fixture_root();
        let units = scanner::scan(&fixture).expect("e2e fixture scans");
        let root = canonical_root(&fixture);
        let mut first = units.clone();
        let mut second = units.clone();
        resolver::resolve(&mut first, &root);
        resolver::resolve(&mut second, &root);
        for (f1, f2) in first
            .iter()
            .flat_map(|u| u.functions.iter())
            .zip(second.iter().flat_map(|u| u.functions.iter()))
        {
            assert_eq!(f1.calls, f2.calls, "call facts must be deterministic");
            assert_eq!(
                f1.record.resolver_tier, f2.record.resolver_tier,
                "record tiers must be deterministic"
            );
        }
        // Full pipeline: the resolved edge surfaces upgraded.
        let out = temp_dir("resolved-arch003").join("perfpal-report");
        let code = run_with(argv(&[
            "--output",
            out.to_string_lossy().as_ref(),
            fixture_root().to_string_lossy().as_ref(),
        ]));
        assert_eq!(code, 0, "pipeline over the e2e fixture must succeed");
        let raw = fs::read_to_string(out.join("metrics.json")).expect("metrics.json present");
        let doc: serde_json::Value = serde_json::from_str(&raw).expect("metrics.json parses");
        let findings = doc["findings"].as_array().expect("findings[] present");
        let arch003: Vec<_> = findings
            .iter()
            .filter(|f| f["rule_id"] == "PERF-C-ARCH-003")
            .collect();
        assert_eq!(
            arch003.len(),
            1,
            "exactly one ARCH-003 edge on the e2e fixture"
        );
        let f = arch003[0];
        assert_eq!(f["confidence"], "MEDIUM");
        assert_eq!(f["resolver_tier"], "semantic");
        assert_eq!(f["evidence"]["callee_name"], "util_init");
        assert_eq!(f["evidence"]["matched_path"], "src/util.c");
        assert_eq!(f["evidence"]["resolved"], "true");
        let edges = doc["graphs"]["call_graph"]["edges"]
            .as_array()
            .expect("call_graph.edges present");
        assert!(
            edges
                .iter()
                .any(|e| e["to_path"] == "src/util.c" && e["confidence"] == "MEDIUM"),
            "resolved call-graph edge must be MEDIUM: {edges:?}"
        );
    }

    // =======================================================================
    // Wave B2 visitor locks (plan `semantic-resolver`): FOPEN SET + LITERAL
    // MAP collection on inline temp C sources, plus one scan+resolve
    // end-to-end proving the facts land on the IR stream. Same skip gate —
    // never fail without libclang.
    // =======================================================================

    /// Parse an inline C source as `src/<name>` under a fresh temp root and
    /// return its [`resolver::ParsedMaps`].
    fn parse_inline(name: &str, source: &str) -> (PathBuf, resolver::ParsedMaps) {
        let root = temp_dir(name);
        fs::create_dir_all(root.join("src")).expect("create src dir");
        let rel = format!("src/{name}");
        let abs = root.join(&rel);
        fs::write(&abs, source).expect("write inline fixture");
        let bytes = fs::read(&abs).expect("re-read inline fixture");
        let maps = resolver::parse_unit_maps(&rel, &abs, &bytes, &root)
            .expect("inline TU parses into maps");
        (root, maps)
    }

    #[test]
    fn fopen_set_collects_fopen_assigned_locals() {
        if !semantic_ready() {
            return;
        }
        let (_root, maps) = parse_inline(
            "work.c",
            r#"
#define _GNU_SOURCE
#include <stdio.h>
void work(void) {
    FILE *fp = fopen("a.txt", "r");
    FILE *g;
    g = fopen("b.txt", "r");
    FILE *h = fdopen(3, "r");
    int n = 5;
    char *p = "x";
    (void)n; (void)p;
}
"#,
        );
        let facts = maps
            .function_facts
            .get(&("src/work.c".to_string(), "work".to_string()))
            .expect("work() proves facts");
        assert_eq!(
            facts.fopen_vars,
            ["fp".to_string(), "g".to_string(), "h".to_string()]
                .into_iter()
                .collect(),
            "decl-init, single assignment, and fdopen all prove membership"
        );
        assert_eq!(
            facts.literals.get("p").map(String::as_str),
            Some("x"),
            "single-literal locals ride along in the same scope"
        );
        assert!(
            !facts.literals.contains_key("n"),
            "non-string initializers prove nothing"
        );
    }

    #[test]
    fn literal_map_enforces_single_token_and_single_write() {
        if !semantic_ready() {
            return;
        }
        let (_root, maps) = parse_inline(
            "fmt.c",
            r#"
void fmt_work(void) {
    const char *ok = "%s";
    const char *cat = "%s" "%d";
    const char *re = "%s";
    re = "%d";
    const char *late;
    late = "%s";
    int n = 0;
    n++;
    (void)cat; (void)re; (void)late; (void)n;
}
"#,
        );
        let facts = maps
            .function_facts
            .get(&("src/fmt.c".to_string(), "fmt_work".to_string()))
            .expect("fmt_work() proves facts");
        assert_eq!(
            facts.literals,
            [
                ("ok".to_string(), "%s".to_string()),
                ("late".to_string(), "%s".to_string())
            ]
            .into_iter()
            .collect(),
            "single init proves; concatenated (2 tokens), reassigned (2 \
             writes) stay silent; decl-then-single-assign proves"
        );
        assert!(
            facts.fopen_vars.is_empty(),
            "no fopen shapes here — FOPEN SET stays empty"
        );
    }

    #[test]
    fn fopen_set_applies_the_file_type_veto() {
        if !semantic_ready() {
            return;
        }
        let (_root, maps) = parse_inline(
            "types.c",
            r#"
#include <stdio.h>
FILE *G;
void *grab(void) {
    void *v = fopen("x", "r");
    return v;
}
void use_global(void) {
    G = fopen("y", "r");
    fclose(G);
}
"#,
        );
        // Declared non-FILE type vetoes the fopen shape (belt-and-braces
        // precedence); the scope proves nothing at all, so no entry.
        assert!(
            !maps
                .function_facts
                .contains_key(&("src/types.c".to_string(), "grab".to_string())),
            "void*-typed fopen assignment proves nothing"
        );
        // No in-scope declaration (global) ⇒ type unobtainable ⇒
        // assignment-from-fopen alone suffices.
        let facts = maps
            .function_facts
            .get(&("src/types.c".to_string(), "use_global".to_string()))
            .expect("use_global() proves the global write");
        assert_eq!(facts.fopen_vars, ["G".to_string()].into_iter().collect());
    }

    #[test]
    fn scan_and_resolve_lands_b2_facts_on_the_ir_stream() {
        if !semantic_ready() {
            return;
        }
        let root = temp_dir("b2-e2e");
        fs::create_dir_all(root.join("src")).expect("create src dir");
        fs::write(
            root.join("src/work.c"),
            r#"
#include <stdio.h>
void drain(int n) {
    FILE *fp = fopen("data.txt", "r");
    const char *fmt = "%s";
    char buf[64];
    for (int i = 0; i < n; i++) {
        fclose(fp);
    }
    scanf(fmt, buf);
}
"#,
        )
        .expect("write inline fixture");
        let mut units = scanner::scan(&root).expect("inline fixture scans");
        // Mutation determinism on real facts: two resolves agree.
        let mut second = units.clone();
        let scan_root = canonical_root(&root);
        resolver::resolve(&mut units, &scan_root);
        resolver::resolve(&mut second, &scan_root);
        let func = units
            .iter()
            .find(|u| u.file.path == "src/work.c")
            .and_then(|u| u.functions.iter().find(|f| f.record.name == "drain"))
            .expect("drain() scanned");
        let func2 = second
            .iter()
            .find(|u| u.file.path == "src/work.c")
            .and_then(|u| u.functions.iter().find(|f| f.record.name == "drain"))
            .expect("drain() scanned (repeat)");
        assert_eq!(func.calls, func2.calls, "call facts deterministic");

        let fclose = func
            .calls
            .iter()
            .find(|c| c.callee_name == "fclose")
            .expect("fclose call scanned");
        assert!(
            fclose.semantic.as_ref().is_some_and(|s| s.fopened_handle),
            "fopened fclose in loop carries the proof: {fclose:?}"
        );
        let scanf = func
            .calls
            .iter()
            .find(|c| c.callee_name == "scanf")
            .expect("scanf call scanned");
        assert_eq!(
            scanf.semantic.as_ref().map(|s| &s.literal_args),
            Some(&vec![(0, "%s".to_string())]),
            "identifier format resolves to its literal value"
        );
        // The fopen call itself carries no facts (it is neither a fclose
        // nor a scanf-family call) — facts attach only where proven.
        let fopen = func
            .calls
            .iter()
            .find(|c| c.callee_name == "fopen")
            .expect("fopen call scanned");
        assert!(
            fopen.semantic.is_none(),
            "no proof about the fopen call itself: {fopen:?}"
        );
    }
}
