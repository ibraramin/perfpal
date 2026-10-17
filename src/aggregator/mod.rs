//! `aggregator` — architecture views & document assembly (modules.md §3.4).
//!
//! Consumes: IR stream ([`FileUnit`]s) + analyzer `findings[]` + provenance
//! from cli. Produces: the complete metrics.json document as schema-typed
//! data (all nine sections; P1 emitted the required set plus
//! `parse_health_summary` and `graphs.include_graph`; P4 Wave 2b adds
//! `graphs.call_graph` and `components[]`; v1.4 step 1 (plan
//! `divergence-revamp`) populates `stats.distributions` — see
//! [`schema::MetricsDocument`]).
//!
//! Sort orders ENFORCED here (modules.md §1 Principle 2 ownership table):
//! - `stats.functions[]`: `(path, span.start_line, span.start_col)`;
//! - `graphs.include_graph.nodes`: sorted by path;
//! - `graphs.include_graph.edges`: sorted by `(from, to)` ascending.
//!   Admission (m6): resolved internal edges (quoted or angle-in-repo) PLUS
//!   unresolved QUOTED edges (`to: null`, data-not-error); unresolved ANGLE
//!   edges stay excluded (external bucket). Rust's `Option<String>` Ord
//!   places `None` BEFORE every `Some(_)`, so an includer's unresolved-quoted
//!   edges sort ahead of its resolved edges sharing the same `from` — pinned
//!   by test;
//! - `graphs.call_graph.edges`: aggregator OWNS document order — re-sorted
//!   `(from_function, to_name, from_path)` ascending regardless of producer
//!   order;
//! - `components[]`: ranked by finding_density ↓ → complexity_sum ↓ →
//!   max_loop_depth ↓ → alloc_site_density ↓ → path_prefix ↑ (1-based rank),
//!   emitted in `(rank, path_prefix)` order (path_prefix is the unique
//!   grouping key, so the two orders coincide);
//! - `files[]`: scanner-guaranteed sorted-by-path; re-sorted defensively so
//!   the aggregator's output is correct even if a future scanner regression
//!   slips past its own tests.
//!
//! ARCH-002 asymmetry note (founder-approved): `components[].rank` uses
//! finding_density; the ARCH-002 heuristic ranks IR-only (no density). They
//! may disagree; that is accepted and documented — do not unify.
//!
//! `findings[]` pass through in the ANALYZER's total order untouched —
//! document order equals report order (modules.md §3.3), re-sorting here
//! would risk diverging from the report §4 comparator.
//!
//! Non-goals: rendering; interpretation; chart capping decisions.

use std::collections::{BTreeMap, BTreeSet};

use crate::analyzer::{CrossFileCallEdge, Finding, cross_file_call_edges};
use crate::graph::{break_cycles, topo_layers};
use crate::ir::{CalleeKind, FunctionRecord, IncludeForm};
use crate::scanner::{FileUnit, SUPPORTED_LANGUAGES};
use crate::schema::{
    self, CallEdge, CallGraphSection, ComponentEntry, ComponentMetrics, DistributionEntry,
    FileEntry, FindingEntry, GraphNode, GraphsSection, IncludeEdgeDto, IncludeGraph,
    MetricsDocument, ParseHealthEntry, ParseHealthSummary, ScanInfo, StatsSection, ToolInfo,
};

/// Provenance values captured by cli (modules.md §3.1 owns the clock).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// Scanned root VERBATIM as given to the CLI (output-contract §2).
    pub root_path: String,
    /// UTC RFC3339 scan-start instant.
    pub started_at: String,
    /// Wall-clock scan duration in milliseconds.
    pub duration_ms: u64,
    /// Version of the producing binary (`tool.version`).
    pub tool_version: String,
}

/// Assemble the complete metrics.json document from the IR stream, findings,
/// and cli-captured provenance.
///
/// Deterministic: same inputs ⇒ same document (provenance values are the
/// caller's clock-derived data, not generated here).
#[must_use]
pub fn assemble(
    units: &[FileUnit],
    findings: &[Finding],
    provenance: &Provenance,
) -> MetricsDocument {
    // files[] — defensive re-sort of the scanner-guaranteed order.
    let mut units_sorted: Vec<&FileUnit> = units.iter().collect();
    units_sorted.sort_by(|a, b| a.file.path.cmp(&b.file.path));

    let files: Vec<FileEntry> = units_sorted
        .iter()
        .map(|u| FileEntry {
            path: u.file.path.clone(),
            language: u.file.language.clone(),
            loc: u.file.loc,
            function_count: u.file.function_count,
            global_var_count: u.file.global_var_count,
            include_counts: u.file.include_counts,
            parse_health: parse_health_entry(&u.parse_health),
        })
        .collect();

    let loc_total = if units.is_empty() {
        None
    } else {
        Some(
            files
                .iter()
                .fold(crate::ir::LocCounts::default(), |acc, f| {
                    crate::ir::LocCounts {
                        code: acc.code + f.loc.code,
                        comment: acc.comment + f.loc.comment,
                        blank: acc.blank + f.loc.blank,
                    }
                }),
        )
    };

    let mut ok = 0u32;
    let mut partial = 0u32;
    let mut failed = 0u32;
    for u in units {
        match u.parse_health.status {
            crate::ir::ParseStatus::Ok => ok += 1,
            crate::ir::ParseStatus::Partial => partial += 1,
            crate::ir::ParseStatus::Failed => failed += 1,
        }
    }

    let shape = call_shape(units);
    // v1.5 I1a+I1b (plan `map-build`): C + Rust entry + reachability
    // passes (per-language gates inside; Python units never contribute —
    // I1c owns them).
    let c_entries = detect_c_entries(units);
    let rust_entries = detect_rust_entries(units);
    let mut entries = c_entries.clone();
    entries.extend(rust_entries.clone());
    let c_edges = resolved_c_edges(units);
    let rust_edges = resolved_rust_edges(units);
    // Union of the per-language BFS closures == the combined BFS: edge
    // sets are language-disjoint (each resolver gates both ends to its own
    // language ids), so no cross-language path exists to merge.
    let mut reachable = c_reachable(&c_entries, &c_edges);
    reachable.extend(rust_reachable(&rust_entries, &rust_edges));
    let functions = function_stats(units, &shape, &entries, &reachable);
    let distributions = distributions(&functions);
    let graphs = GraphsSection {
        include_graph: include_graph(units),
        // ALWAYS Some — even with zero edges — so consumers can rely on the
        // ["edges"] shape (mirrors graphs-always-Some precedent).
        call_graph: Some(call_graph(units)),
    };
    let components = components(units, findings);

    MetricsDocument {
        schema_version: schema::SCHEMA_VERSION.to_string(),
        tool: ToolInfo {
            name: "perfpal".to_string(),
            version: provenance.tool_version.clone(),
        },
        scan: ScanInfo {
            root_path: provenance.root_path.clone(),
            started_at: provenance.started_at.clone(),
            duration_ms: Some(provenance.duration_ms),
            languages: scan_languages(units),
            file_count: units.len().min(u32::MAX as usize) as u32,
            loc_total,
        },
        parse_health_summary: Some(ParseHealthSummary {
            ok,
            partial,
            failed,
        }),
        files,
        stats: StatsSection {
            functions,
            // v1.4 step 1: populated histograms (fixed buckets, full label
            // sets); `None` only on a zero-function scan.
            distributions,
        },
        findings: findings_with_symbols(units_sorted, findings),
        graphs: Some(graphs),
        components: Some(components),
    }
}

/// Per-file parse-health entry; empty error regions are omitted
/// (schema-optional field, omit-don't-null policy).
fn parse_health_entry(health: &crate::ir::ParseHealth) -> ParseHealthEntry {
    ParseHealthEntry {
        status: health.status,
        error_regions: if health.error_regions.is_empty() {
            None
        } else {
            Some(health.error_regions.clone())
        },
        error_node_count: health.error_node_count,
        resolver: health.resolver,
    }
}

/// `scan.languages`: sorted-unique languages of the scanned files (BTreeSet =>
/// deterministic; owner: aggregator per modules.md §1), or SUPPORTED_LANGUAGES
/// on a zero-file scan (schema minItems:1 — "languages scanned for").
fn scan_languages(units: &[FileUnit]) -> Vec<String> {
    let set: std::collections::BTreeSet<&str> =
        units.iter().map(|u| u.file.language.as_str()).collect();
    if set.is_empty() {
        SUPPORTED_LANGUAGES
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    } else {
        set.into_iter().map(str::to_string).collect()
    }
}

/// `stats.functions[]` across ALL files, sorted
/// `(path, span.start_line, span.start_col)` (modules.md §1 aggregator row).
/// The v1.4 call-shape fields come from the whole-program [`call_shape`] pass
/// below (every function id is covered, so every DTO emits `Some`); the three
/// source-local fields (`idiom_hash`, `error_path`, `alloc_family`) lift
/// verbatim from the scanner post-pass (`None`/omitted for parser-free or
/// pre-step-2 inputs). The v1.5 entry/reachability fields come from the
/// whole-program [`detect_c_entries`] + [`detect_rust_entries`] and
/// [`c_reachable`] + [`rust_reachable`] passes — `Some` for every C and Rust
/// function (C+Rust gate since I1b; Python stays `None`/omitted — I1c).
fn function_stats(
    units: &[FileUnit],
    shape: &BTreeMap<String, CallShape>,
    entries: &BTreeMap<String, (String, String)>,
    reachable: &BTreeSet<String>,
) -> Vec<schema::FunctionStats> {
    let mut records: Vec<(bool, &FunctionRecord)> = units
        .iter()
        .flat_map(|u| {
            let entry_scoped = matches!(u.file.language.as_str(), "c" | "rust");
            u.functions.iter().map(move |f| (entry_scoped, &f.record))
        })
        .collect();
    records.sort_by(|a, b| {
        a.1.path
            .cmp(&b.1.path)
            .then(a.1.span.start_line.cmp(&b.1.span.start_line))
            .then(a.1.span.start_col.cmp(&b.1.span.start_col))
    });
    records
        .into_iter()
        .map(|(entry_scoped, r)| {
            let call = shape.get(&r.id).copied().unwrap_or_default();
            let (entry_kind, entry_confidence) = if entry_scoped {
                entries
                    .get(&r.id)
                    .map(|(kind, confidence)| (Some(kind.clone()), Some(confidence.clone())))
                    .unwrap_or((None, None))
            } else {
                (None, None)
            };
            schema::FunctionStats {
                id: r.id.clone(),
                name: r.name.clone(),
                path: r.path.clone(),
                span: r.span,
                param_count: r.param_count,
                cyclomatic_complexity: r.cyclomatic_complexity,
                cognitive_complexity: r.cognitive_complexity,
                max_loop_depth: r.max_loop_depth,
                alloc_site_count: r.alloc_site_count,
                length_lines: r.length_lines,
                // v1.4 step 2: whole-program call-shape (computed above —
                // always `Some`); source-local fields lift verbatim.
                call_fanout: Some(call.fanout),
                call_fanin: Some(call.fanin),
                call_depth: Some(call.depth),
                is_recursive: Some(call.recursive),
                idiom_hash: r.idiom_hash.clone(),
                error_path: r.error_path,
                alloc_family: r.alloc_family.clone(),
                cfg_gated: r.cfg_gated,
                // v1.5 I1a+I1b (plan `map-build`): whole-program entry
                // passes — `Some` for entries, `None` (omitted) for C/Rust
                // non-entries and for every Python function (I1c owns it).
                entry_kind,
                entry_confidence,
                // v1.5 I1a+I1b: BFS membership over resolved edges — `Some`
                // for every C/Rust function, `None` (omitted) for Python.
                reachable_from_entry: if entry_scoped {
                    Some(reachable.contains(&r.id))
                } else {
                    None
                },
            }
        })
        .collect()
}

/// Per-function call-shape instrument (IR v1.4 STEP 2, plan
/// `divergence-revamp`, specs `divergence-study`/`divergence-paper`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct CallShape {
    /// Distinct direct-callee count (resolved function ids).
    fanout: u32,
    /// Distinct caller count (resolved function ids).
    fanin: u32,
    /// Longest-path layer index in the call DAG (0 = no in-graph callers;
    /// cycle-broken edges excluded — see below).
    depth: u32,
    /// True when the function can reach itself statically (self-loop or
    /// directed cycle on the FULL edge set, before cycle-breaking).
    recursive: bool,
}

/// Whole-program call-shape pass (v1.4 step 2, slice A).
///
/// Edge resolution (deterministic; `BTree*` throughout):
/// 1. semantic facts first — a `Direct` call carrying
///    `semantic.resolved_definition` resolves to the function owning
///    `(site.path, site.span)` (span intersection), even same-function
///    (self-loop ⇒ recursive);
/// 2. name-based fallback — the caller's OWN file defining the name exactly
///    once wins (covers self-recursion and sibling calls — file-local-first,
///    mirroring `cross_file_call_edges`); else the language-scoped global
///    set must contain exactly one function of that name (the M1 uniqueness
///    gate); zero or many ⇒ ambiguous, no edge;
/// 3. non-`Direct` calls never resolve (the name is a variable, not a target).
///
/// Unlike `graphs.call_graph` (cross-file only), intra-file edges COUNT here
/// (spec: "intra-file resolved edges emitted as stats, not findings").
///
/// `depth` is the [`topo_layers`] index over the [`break_cycles`] DAG —
/// i.e. the longest caller-chain length above the function, NOT the callee
/// subtree below it (documented reading of "depth = topo layer").
/// `recursive` is computed on the FULL pre-break edge set (self-loop or
/// directed reachability back to self), so cycle-breaking never hides it.
///
/// Sorted/deterministic: node ids sorted, edges deduped in a `BTreeSet`,
/// counts from ordered maps — same units ⇒ same map on every run.
fn call_shape(units: &[FileUnit]) -> BTreeMap<String, CallShape> {
    // Indexes: id → (language, name); (language, name) → sorted ids;
    // path → sorted ids (for own-file-first + semantic site lookup).
    let mut id_lang: BTreeMap<String, &str> = BTreeMap::new();
    let mut by_name: BTreeMap<(&str, &str), Vec<String>> = BTreeMap::new();
    let mut by_path: BTreeMap<&str, Vec<&FunctionRecord>> = BTreeMap::new();
    for unit in units {
        for func in &unit.functions {
            let record = &func.record;
            id_lang.insert(record.id.clone(), unit.file.language.as_str());
            by_name
                .entry((unit.file.language.as_str(), record.name.as_str()))
                .or_default()
                .push(record.id.clone());
            by_path
                .entry(record.path.as_str())
                .or_default()
                .push(record);
        }
    }
    for ids in by_name.values_mut() {
        ids.sort();
    }

    // Resolve every Direct call to a target id (deduped per caller).
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for unit in units {
        for func in &unit.functions {
            let caller = &func.record.id;
            let language = unit.file.language.as_str();
            let mut calls: Vec<&crate::ir::CallEvent> = func.calls.iter().collect();
            calls.sort_by_key(|c| (c.meta.span.start_line, c.meta.span.start_col));
            for call in calls {
                if call.callee_kind != CalleeKind::Direct {
                    continue;
                }
                // (1) Semantic fact: (path, span) → owning function.
                if !call.meta.macro_origin
                    && let Some(site) = call
                        .semantic
                        .as_ref()
                        .and_then(|s| s.resolved_definition.as_ref())
                    && let Some(target) = by_path.get(site.path.as_str()).and_then(|defs| {
                        defs.iter()
                            .find(|r| r.span.intersects(&site.span))
                            .map(|r| &r.id)
                    })
                {
                    edges.insert((caller.clone(), target.clone()));
                    continue;
                }
                // (2a) Own-file-first: exactly one same-file definition wins.
                let own: Vec<&String> = by_path
                    .get(func.record.path.as_str())
                    .map(|defs| {
                        defs.iter()
                            .filter(|r| r.name == call.callee_name)
                            .map(|r| &r.id)
                            .collect()
                    })
                    .unwrap_or_default();
                if own.len() == 1 {
                    edges.insert((caller.clone(), (*own[0]).clone()));
                    continue;
                }
                if !own.is_empty() {
                    continue; // same-file overloads: ambiguous, no edge
                }
                // (2b) Language-scoped uniqueness gate.
                if let Some(ids) = by_name.get(&(language, call.callee_name.as_str()))
                    && ids.len() == 1
                {
                    edges.insert((caller.clone(), ids[0].clone()));
                }
            }
        }
    }

    // Fanout / fanin over the deduped resolved set.
    let mut fanout: BTreeMap<&str, u32> = BTreeMap::new();
    let mut fanin: BTreeMap<&str, u32> = BTreeMap::new();
    for id in id_lang.keys() {
        fanout.insert(id.as_str(), 0);
        fanin.insert(id.as_str(), 0);
    }
    for (from, to) in &edges {
        *fanout.get_mut(from.as_str()).expect("caller indexed") += 1;
        *fanin.get_mut(to.as_str()).expect("callee indexed") += 1;
    }

    // Depth: longest-path layering over the cycle-broken DAG.
    let pairs: Vec<(String, String)> = edges.iter().cloned().collect();
    let broken = break_cycles(&pairs);
    let mut nodes: Vec<String> = id_lang.keys().cloned().collect();
    nodes.sort();
    let layers = topo_layers(&nodes, &broken.kept);
    let mut depth: BTreeMap<&str, u32> = BTreeMap::new();
    for (layer_idx, members) in layers.iter().enumerate() {
        for member in members {
            depth.insert(member.as_str(), layer_idx as u32);
        }
    }

    // Recursion on the FULL edge set: self-loop or directed path back to self.
    let mut adj: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for id in id_lang.keys() {
        adj.entry(id.as_str()).or_default();
    }
    for (from, to) in &edges {
        adj.entry(from.as_str()).or_default().insert(to.as_str());
    }
    let mut out = BTreeMap::new();
    for id in id_lang.keys() {
        let recursive = reaches_self(&adj, id.as_str());
        out.insert(
            id.clone(),
            CallShape {
                fanout: fanout[id.as_str()],
                fanin: fanin[id.as_str()],
                depth: depth.get(id.as_str()).copied().unwrap_or(0),
                recursive,
            },
        );
    }
    out
}

/// True when `start` reaches itself over `adj` in ≥1 step (self-loop or
/// directed cycle). Iterative DFS over sorted sets — deterministic.
fn reaches_self(adj: &BTreeMap<&str, BTreeSet<&str>>, start: &str) -> bool {
    let mut stack: Vec<&str> = adj
        .get(start)
        .map(|next| next.iter().copied().collect())
        .unwrap_or_default();
    let mut visited: BTreeSet<&str> = BTreeSet::new();
    while let Some(node) = stack.pop() {
        if node == start {
            return true;
        }
        if visited.insert(node)
            && let Some(next) = adj.get(node)
        {
            stack.extend(next.iter().copied());
        }
    }
    false
}

/// Entry-kind tokens (plan `map-build` I1a, C only) — the closed
/// `stats.functions[].entry_kind` value set, mirrored in
/// `docs/schema/metrics.schema.v1.5.json`.
const ENTRY_MAIN: &str = "main";
const ENTRY_HANDLER: &str = "handler";
const ENTRY_LIB_ROOT: &str = "lib-root";
const ENTRY_UNKNOWN: &str = "unknown-entry";

/// Entry-confidence tokens — closed `entry_confidence` value set.
const ENTRY_HIGH: &str = "HIGH";
const ENTRY_MEDIUM: &str = "MEDIUM";
const ENTRY_LOW: &str = "LOW";

/// Handler-heuristic name fragments (plan `map-build` I1a): matched
/// case-insensitively against the function name (birdseye-map.md F1 —
/// `*handler*/*callback*/*dispatch*`).
const HANDLER_FRAGMENTS: [&str; 3] = ["handler", "callback", "dispatch"];

/// C entry-point detection (plan `map-build` I1a; Rust has its own
/// [`detect_rust_entries`] since I1b, Python units are never consulted by
/// either detector — their DTO fields stay `None`/omitted until I1c).
///
/// Returns `function id → (entry_kind, entry_confidence)`, deterministic
/// (`BTreeMap`, path-asc tie-break by construction — ids embed
/// `path#line:col`):
/// 1. `main` — exact name match (any file, any linkage): `(main, HIGH)`;
/// 2. handler heuristic — case-insensitive `*handler*`/`*callback*`/
///    `*dispatch*` substring: `(handler, MEDIUM)`. Precedence: `main` wins
///    (checked first);
/// 3. lib-roots — exported (non-`static`, i.e. `is_static != Some(true)`)
///    functions in MAIN-LESS files (no `main`-named definition in that
///    path): `(lib-root, MEDIUM)`. Files WITH a `main` contribute no
///    lib-roots — their surface is reachable from `main` instead;
/// 4. honest `unknown-entry` stub — when steps 1–3 yield ZERO entries but C
///    functions exist, EVERY C function becomes `(unknown-entry, LOW)` so
///    reachability stays total instead of silently empty (never a blank —
///    maintainer-map honesty contract).
///
/// C non-entries (e.g. `static` helpers, callees) are ABSENT from the map
/// (`None`/omitted on the wire) — absence means "not an entry", not
/// "unknown".
fn detect_c_entries(units: &[FileUnit]) -> BTreeMap<String, (String, String)> {
    let mut c_functions: Vec<&FunctionRecord> = units
        .iter()
        .filter(|u| u.file.language.as_str() == "c")
        .flat_map(|u| u.functions.iter().map(|f| &f.record))
        .collect();
    // Defensive input sort (rev-1 nit from I1a): output is a `BTreeMap`
    // either way, but sorted input keeps precedence claims and stub
    // insertion order input-independent.
    c_functions.sort_by(|a, b| a.id.cmp(&b.id));
    // Main-less check is per PATH: a file contributes lib-roots only when no
    // `main`-named C definition lives in that path.
    let files_with_main: BTreeSet<&str> = c_functions
        .iter()
        .filter(|r| r.name == "main")
        .map(|r| r.path.as_str())
        .collect();

    let mut entries: BTreeMap<String, (String, String)> = BTreeMap::new();
    for record in &c_functions {
        if record.name == "main" {
            entries.insert(
                record.id.clone(),
                (ENTRY_MAIN.to_string(), ENTRY_HIGH.to_string()),
            );
            continue;
        }
        let lowered = record.name.to_lowercase();
        if HANDLER_FRAGMENTS.iter().any(|frag| lowered.contains(frag)) {
            entries.insert(
                record.id.clone(),
                (ENTRY_HANDLER.to_string(), ENTRY_MEDIUM.to_string()),
            );
        }
    }
    for record in &c_functions {
        if entries.contains_key(&record.id) {
            continue; // main/handler precedence already claimed it
        }
        // Lib-root = exported (non-static) in a main-less file. `is_static`
        // is `None` only for pre-I1a inputs — treated as exported (the C
        // default linkage) rather than silently dropped.
        if !files_with_main.contains(record.path.as_str()) && record.is_static != Some(true) {
            entries.insert(
                record.id.clone(),
                (ENTRY_LIB_ROOT.to_string(), ENTRY_MEDIUM.to_string()),
            );
        }
    }
    if entries.is_empty() && !c_functions.is_empty() {
        // Honest stub (birdseye-map.md F1): no heuristic fired anywhere in
        // the C corpus — every C function is an unknown entry (LOW) so the
        // reachability BFS below stays total. Captioned, never silent.
        for record in &c_functions {
            entries.insert(
                record.id.clone(),
                (ENTRY_UNKNOWN.to_string(), ENTRY_LOW.to_string()),
            );
        }
    }
    entries
}

/// RESOLVED C call edges as `(caller id, callee id)` (plan `map-build` I1a).
///
/// Mirrors [`call_shape`] resolution step (1) ONLY — semantic-fact-backed
/// (`semantic.resolved_definition` → span-intersecting definition, either
/// file). The name-based fallbacks (call_shape steps 2a/2b — own-file-single
/// and language-unique matching, the LOW-confidence tier) are DELIBERATELY
/// EXCLUDED: reachability rides MEDIUM+ edges only (birdseye-map.md F1,
/// ADR-0008 third layer). C-only gate: both ends must be C function ids.
///
/// Sorted/deterministic: `BTreeSet` over `(caller, callee)` id pairs.
fn resolved_c_edges(units: &[FileUnit]) -> BTreeSet<(String, String)> {
    let c_ids: BTreeSet<&str> = units
        .iter()
        .filter(|u| u.file.language.as_str() == "c")
        .flat_map(|u| u.functions.iter().map(|f| f.record.id.as_str()))
        .collect();
    let mut by_path: BTreeMap<&str, Vec<&FunctionRecord>> = BTreeMap::new();
    for unit in units.iter().filter(|u| u.file.language.as_str() == "c") {
        for func in &unit.functions {
            by_path
                .entry(func.record.path.as_str())
                .or_default()
                .push(&func.record);
        }
    }

    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for unit in units.iter().filter(|u| u.file.language.as_str() == "c") {
        for func in &unit.functions {
            let mut calls: Vec<&crate::ir::CallEvent> = func.calls.iter().collect();
            calls.sort_by_key(|c| (c.meta.span.start_line, c.meta.span.start_col));
            for call in calls {
                if call.callee_kind != CalleeKind::Direct || call.meta.macro_origin {
                    continue;
                }
                let Some(site) = call
                    .semantic
                    .as_ref()
                    .and_then(|s| s.resolved_definition.as_ref())
                else {
                    continue; // no semantic fact ⇒ LOW-tier name match at
                    // best — excluded from reachability
                };
                let Some(target) = by_path.get(site.path.as_str()).and_then(|defs| {
                    defs.iter()
                        .find(|r| r.span.intersects(&site.span))
                        .map(|r| &r.id)
                }) else {
                    continue;
                };
                if c_ids.contains(func.record.id.as_str()) && c_ids.contains(target.as_str()) {
                    edges.insert((func.record.id.clone(), (*target).clone()));
                }
            }
        }
    }
    edges
}

/// Shared entry-reachability BFS core (plans `map-build` I1a+I1b): the
/// reachable function-id set over the given RESOLVED edges — entries
/// themselves are included (distance 0). Iterative DFS over sorted
/// adjacency — deterministic; same inputs ⇒ same set on every run.
fn entry_reachable(
    entries: &BTreeMap<String, (String, String)>,
    edges: &BTreeSet<(String, String)>,
) -> BTreeSet<String> {
    let mut adj: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (from, to) in edges {
        adj.entry(from.as_str()).or_default().insert(to.as_str());
    }
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut stack: Vec<&str> = entries.keys().map(String::as_str).collect();
    while let Some(node) = stack.pop() {
        if !seen.insert(node.to_string()) {
            continue;
        }
        if let Some(next) = adj.get(node) {
            stack.extend(next.iter().copied());
        }
    }
    seen
}

/// Reachability BFS from C entries over RESOLVED edges only (plan
/// `map-build` I1a). Delegates to [`entry_reachable`]; I1a behavior
/// unchanged — the C-only edge gate lives in [`resolved_c_edges`].
fn c_reachable(
    entries: &BTreeMap<String, (String, String)>,
    edges: &BTreeSet<(String, String)>,
) -> BTreeSet<String> {
    entry_reachable(entries, edges)
}

/// Rust bin-target test (plan `map-build` I1b): `true` when `path` is a
/// binary crate root — a bare `main.rs` single-file target or any
/// `<pkg>/src/main.rs` (Cargo default layout AND workspace layouts like
/// `crates/foo/src/main.rs`, via the trailing `/main.rs` suffix).
/// Bin-owned files never contribute lib-roots — their surface is reachable
/// from the bin `main` instead (the C with-main-file rule, mirrored). The
/// path-suffix approximation in `src/scanner/mod.rs`
/// (`is_crate_root_dir_fallback`) is deliberately NOT reused: entry
/// detection needs the coarser bin-vs-lib file split, not
/// module-resolution roots. Revisit only if golden tests prove this
/// suffix rule wrong.
fn is_rust_bin_path(path: &str) -> bool {
    path == "main.rs" || path.ends_with("/main.rs")
}

/// Rust entry-point detection (plan `map-build` I1b — founder-approved
/// option 1: entries + honest-empty reachability).
///
/// Mirrors [`detect_c_entries`] with the Rust bin/lib split:
/// 1. `main` — exact name match (any file): `(main, HIGH)`;
/// 2. handler heuristic — case-insensitive `*handler*`/`*callback*`/
///    `*dispatch*` substring: `(handler, MEDIUM)`. Precedence: `main` wins
///    (checked first);
/// 3. lib-roots — exported-looking (`is_static != Some(true)` — the Rust
///    backend always leaves `is_static` as `None`, so every Rust function
///    counts as exported-looking) functions in MAIN-LESS, non-bin files
///    (`src/lib.rs` or any other non-`main.rs` file with no `main`-named
///    definition in that path): `(lib-root, MEDIUM)`. Bin-owned files
///    ([`is_rust_bin_path`]) contribute no lib-roots — their surface is
///    reachable from the bin `main` instead;
/// 4. honest `unknown-entry` stub — when steps 1–3 yield ZERO entries but
///    Rust functions exist, EVERY Rust function becomes
///    `(unknown-entry, LOW)` so reachability stays total instead of
///    silently empty (never a blank — maintainer-map honesty contract).
///
/// Rust non-entries are ABSENT from the map (`None`/omitted on the wire) —
/// absence means "not an entry", not "unknown".
fn detect_rust_entries(units: &[FileUnit]) -> BTreeMap<String, (String, String)> {
    let mut rust_functions: Vec<&FunctionRecord> = units
        .iter()
        .filter(|u| u.file.language.as_str() == "rust")
        .flat_map(|u| u.functions.iter().map(|f| &f.record))
        .collect();
    // Defensive input sort (rev-1 nit from I1a, applied to both detectors):
    // output is a `BTreeMap` either way, but sorted input keeps precedence
    // claims and stub insertion order input-independent.
    rust_functions.sort_by(|a, b| a.id.cmp(&b.id));
    // Main-less check is per PATH (mirrors C): a file contributes
    // lib-roots only when no `main`-named Rust definition lives in it.
    let files_with_main: BTreeSet<&str> = rust_functions
        .iter()
        .filter(|r| r.name == "main")
        .map(|r| r.path.as_str())
        .collect();

    let mut entries: BTreeMap<String, (String, String)> = BTreeMap::new();
    for record in &rust_functions {
        if record.name == "main" {
            entries.insert(
                record.id.clone(),
                (ENTRY_MAIN.to_string(), ENTRY_HIGH.to_string()),
            );
            continue;
        }
        let lowered = record.name.to_lowercase();
        if HANDLER_FRAGMENTS.iter().any(|frag| lowered.contains(frag)) {
            entries.insert(
                record.id.clone(),
                (ENTRY_HANDLER.to_string(), ENTRY_MEDIUM.to_string()),
            );
        }
    }
    for record in &rust_functions {
        if entries.contains_key(&record.id) {
            continue; // main/handler precedence already claimed it
        }
        // Lib-root = exported-looking in a main-less, non-bin file.
        // `is_static` is `None` for every Rust function (the backend never
        // populates it — no `static`-linkage concept is surfaced), so the
        // `!= Some(true)` test treats all of them as exported.
        if !files_with_main.contains(record.path.as_str())
            && !is_rust_bin_path(&record.path)
            && record.is_static != Some(true)
        {
            entries.insert(
                record.id.clone(),
                (ENTRY_LIB_ROOT.to_string(), ENTRY_MEDIUM.to_string()),
            );
        }
    }
    if entries.is_empty() && !rust_functions.is_empty() {
        // Honest stub (birdseye-map.md F1): no heuristic fired anywhere in
        // the Rust corpus — every Rust function is an unknown entry (LOW)
        // so the reachability BFS below stays total. Captioned, never
        // silent.
        for record in &rust_functions {
            entries.insert(
                record.id.clone(),
                (ENTRY_UNKNOWN.to_string(), ENTRY_LOW.to_string()),
            );
        }
    }
    entries
}

/// RESOLVED Rust call edges as `(caller id, callee id)` (plan `map-build`
/// I1b).
///
/// Mirrors [`resolved_c_edges`] resolution step (1) ONLY —
/// semantic-fact-backed (`semantic.resolved_definition` → span-intersecting
/// definition, intra-file INCLUDED — this set is never built from
/// `cross_file_call_edges`, which excludes same-file pairs). The
/// name-based fallbacks are DELIBERATELY EXCLUDED: reachability rides
/// MEDIUM+ edges only (birdseye-map.md F1, ADR-0008 third layer).
/// Rust-only gate: both ends must be Rust function ids.
///
/// Honestly EMPTY in this milestone: Rust is syntactic-tier only
/// (`src/analyzer/heuristics/rust/arch.rs:490-493` — every Rust edge is
/// the LOW name-based fallback), so a default build carries no semantic
/// facts and this set is empty; reachability is then entries-only BY
/// CONSTRUCTION, never by fallback to LOW/name-based edges (option 3 was
/// explicitly rejected).
///
/// Sorted/deterministic: `BTreeSet` over `(caller, callee)` id pairs.
fn resolved_rust_edges(units: &[FileUnit]) -> BTreeSet<(String, String)> {
    let rust_ids: BTreeSet<&str> = units
        .iter()
        .filter(|u| u.file.language.as_str() == "rust")
        .flat_map(|u| u.functions.iter().map(|f| f.record.id.as_str()))
        .collect();
    let mut by_path: BTreeMap<&str, Vec<&FunctionRecord>> = BTreeMap::new();
    for unit in units.iter().filter(|u| u.file.language.as_str() == "rust") {
        for func in &unit.functions {
            by_path
                .entry(func.record.path.as_str())
                .or_default()
                .push(&func.record);
        }
    }

    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for unit in units.iter().filter(|u| u.file.language.as_str() == "rust") {
        for func in &unit.functions {
            let mut calls: Vec<&crate::ir::CallEvent> = func.calls.iter().collect();
            calls.sort_by_key(|c| (c.meta.span.start_line, c.meta.span.start_col));
            for call in calls {
                if call.callee_kind != CalleeKind::Direct || call.meta.macro_origin {
                    continue;
                }
                let Some(site) = call
                    .semantic
                    .as_ref()
                    .and_then(|s| s.resolved_definition.as_ref())
                else {
                    continue; // no semantic fact ⇒ LOW-tier name match at
                    // best — excluded from reachability
                };
                let Some(target) = by_path.get(site.path.as_str()).and_then(|defs| {
                    defs.iter()
                        .find(|r| r.span.intersects(&site.span))
                        .map(|r| &r.id)
                }) else {
                    continue;
                };
                if rust_ids.contains(func.record.id.as_str()) && rust_ids.contains(target.as_str())
                {
                    edges.insert((func.record.id.clone(), (*target).clone()));
                }
            }
        }
    }
    edges
}

/// Reachability BFS from Rust entries over RESOLVED edges only (plan
/// `map-build` I1b). Mirrors [`c_reachable`] via the shared
/// [`entry_reachable`] core. In this milestone the edge set is honestly
/// empty (see [`resolved_rust_edges`]), so the result is entries at
/// distance 0 only — total, never silently empty.
fn rust_reachable(
    entries: &BTreeMap<String, (String, String)>,
    edges: &BTreeSet<(String, String)>,
) -> BTreeSet<String> {
    entry_reachable(entries, edges)
}

/// `graphs.include_graph`: nodes = every scanned file (sorted by path); edges
/// sorted `(from, to)` ascending. Admission (m6): resolved internal edges
/// (quoted or angle-in-repo) PLUS unresolved QUOTED edges (`to: null` — data,
/// not error, modules.md Principle 4); unresolved ANGLE edges stay excluded
/// (external bucket). Duplicates are preserved, not deduped.
///
/// Sort note: Rust's `Option<String>` Ord places `None` BEFORE every
/// `Some(_)`, so an includer's unresolved-quoted edges sort ahead of its
/// resolved edges sharing the same `from`. Pinned by test.
fn include_graph(units: &[FileUnit]) -> IncludeGraph {
    let mut nodes: Vec<GraphNode> = units
        .iter()
        .map(|u| GraphNode {
            path: u.file.path.clone(),
        })
        .collect();
    nodes.sort_by(|a, b| a.path.cmp(&b.path));

    let mut edges: Vec<IncludeEdgeDto> = units
        .iter()
        .flat_map(|u| u.includes.iter().map(move |e| (u.file.path.as_str(), e)))
        .filter(|(_, e)| e.resolved || e.form == IncludeForm::Quoted)
        .map(|(from, e)| IncludeEdgeDto {
            from: from.to_string(),
            to: e.to_path.clone(),
            form: e.form,
            resolved: e.resolved,
            span: e.span,
        })
        .collect();
    // Option<String> Ord: None < Some(_) — unresolved-quoted edges of one
    // includer precede its resolved edges with the same `from`.
    edges.sort_by(|a, b| a.from.cmp(&b.from).then(a.to.cmp(&b.to)));

    IncludeGraph { nodes, edges }
}

/// One [`CrossFileCallEdge`] lifted into its schema shape. The confidence is
/// DYNAMIC as of the semantic-resolver Wave B1: MEDIUM for resolved (USR-
/// backed) edges, LOW for the name-based fallback — the v1.1 schema widened
/// `call_graph.edges[].confidence` to oneOf LOW/MEDIUM/HIGH exactly for this
/// (metrics-json-v1.md §5; v1.0 all-LOW documents still validate).
fn call_edge_from(edge: CrossFileCallEdge) -> CallEdge {
    CallEdge::with_confidence(
        edge.from_function_id,
        edge.callee_name,
        edge.from_path,
        Some(edge.to_path),
        edge.confidence,
    )
}

/// `graphs.call_graph`: cross-file call edges only (intra-file omitted in
/// v1), re-sorted by the AGGREGATOR's document order
/// `(from_function, to_name, from_path)` ascending — producer order is never
/// trusted (modules.md §1 Principle 2).
fn call_graph(units: &[FileUnit]) -> CallGraphSection {
    let mut edges: Vec<CallEdge> = cross_file_call_edges(units)
        .into_iter()
        .map(call_edge_from)
        .collect();
    edges.sort_by(|a, b| {
        a.from_function
            .cmp(&b.from_function)
            .then(a.to_name.cmp(&b.to_name))
            .then(a.from_path.cmp(&b.from_path))
    });
    CallGraphSection { edges }
}

/// Per-directory accumulator feeding [`components`].
struct ComponentAcc {
    complexity_sum: f64,
    max_loop_depth: u32,
    alloc_site_count: u64,
    kloc: f64,
    finding_count: u32,
}

impl ComponentAcc {
    fn new() -> Self {
        Self {
            complexity_sum: 0.0,
            max_loop_depth: 0,
            alloc_site_count: 0,
            kloc: 0.0,
            finding_count: 0,
        }
    }

    /// Densities are per KLOC; a zero-KLOC component (all-comment/blank
    /// files) yields 0.0 rather than ±inf/NaN.
    fn alloc_site_density(&self) -> f64 {
        if self.kloc == 0.0 {
            0.0
        } else {
            self.alloc_site_count as f64 / self.kloc
        }
    }

    fn finding_density(&self) -> f64 {
        if self.kloc == 0.0 {
            0.0
        } else {
            f64::from(self.finding_count) / self.kloc
        }
    }
}

/// Parent directory of a repo-relative POSIX path; root-level files map to
/// `"."`. Scanner paths are already clean POSIX (join_normalized resolves
/// `.`/`..` and collapses separators), so lexical splitting suffices — no
/// extra normalization here.
fn parent_dir(path: &str) -> String {
    match path.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => ".".to_string(),
    }
}

/// `components[]`: ONE component per parent directory of every scanned
/// FileRecord path (repo-relative POSIX; root-level files → prefix `"."`).
///
/// Metrics per component:
/// - `complexity_sum`: Σ cyclomatic_complexity over FunctionRecords in the dir;
/// - `max_loop_depth`: max over those records (0 when none);
/// - `alloc_site_density` / `finding_density`: per KLOC (Σ loc.code / 1000),
///   guarded to 0.0 when KLOC == 0;
/// - `finding_count`: findings whose path's parent dir equals the prefix —
///   ALWAYS populated (Some(n), even 0).
///
/// Rank composite: finding_density ↓ → complexity_sum ↓ → max_loop_depth ↓ →
/// alloc_site_density ↓ → path_prefix ↑; rank is the 1-based position in that
/// order. Emission order = `(rank, path_prefix)` (path_prefix unique ⇒
/// identical). Float comparisons use `total_cmp` for total determinism.
///
/// ARCH-002 asymmetry note: this ranking uses finding_density; the ARCH-002
/// heuristic ranks IR-only (no density). They may disagree by design.
fn components(units: &[FileUnit], findings: &[Finding]) -> Vec<ComponentEntry> {
    let mut acc: BTreeMap<String, ComponentAcc> = BTreeMap::new();

    for unit in units {
        let entry = acc
            .entry(parent_dir(&unit.file.path))
            .or_insert_with(ComponentAcc::new);
        entry.kloc += f64::from(unit.file.loc.code) / 1000.0;

        for func in &unit.functions {
            let record = &func.record;
            let entry = acc
                .entry(parent_dir(&record.path))
                .or_insert_with(ComponentAcc::new);
            entry.complexity_sum += f64::from(record.cyclomatic_complexity);
            entry.max_loop_depth = entry.max_loop_depth.max(record.max_loop_depth);
            entry.alloc_site_count += u64::from(record.alloc_site_count);
        }
    }

    // Findings count into their directory ONLY when that directory exists as
    // a component (components come from FileRecords alone).
    for finding in findings {
        if let Some(entry) = acc.get_mut(&parent_dir(&finding.path)) {
            entry.finding_count += 1;
        }
    }

    let mut entries: Vec<ComponentEntry> = acc
        .into_iter()
        .map(|(prefix, a)| ComponentEntry {
            component_id: prefix.clone(),
            path_prefix: prefix,
            rank: 0, // assigned after sorting below
            metrics: ComponentMetrics {
                complexity_sum: a.complexity_sum,
                alloc_site_density: a.alloc_site_density(),
                max_loop_depth: a.max_loop_depth,
                finding_density: a.finding_density(),
            },
            finding_count: Some(a.finding_count),
        })
        .collect();

    entries.sort_by(|x, y| {
        y.metrics
            .finding_density
            .total_cmp(&x.metrics.finding_density)
            .then(
                y.metrics
                    .complexity_sum
                    .total_cmp(&x.metrics.complexity_sum),
            )
            .then(y.metrics.max_loop_depth.cmp(&x.metrics.max_loop_depth))
            .then(
                y.metrics
                    .alloc_site_density
                    .total_cmp(&x.metrics.alloc_site_density),
            )
            .then(x.path_prefix.cmp(&y.path_prefix))
    });
    for (position, entry) in entries.iter_mut().enumerate() {
        entry.rank = position as u32 + 1;
    }
    entries
}

/// Fixed `cyclomatic_complexity` bucket labels (v1.4 step 1, per
/// `docs/schema/stats-distributions-proposal.md` §3–§4): edges
/// `[1, 5, 10, 15, 25, 50, ∞)` — 15 is CALL-002's promotion threshold, so the
/// last three buckets are exactly the CALL-002-qualifying population.
/// Zero-padded so LEXICOGRAPHIC order equals numeric order under the
/// unit-locked [`sort_distributions`] comparator.
const COMPLEXITY_BUCKETS: [&str; 6] = ["01-04", "05-09", "10-14", "15-24", "25-49", "50+"];

/// Fixed `max_loop_depth` bucket labels (proposal §3.2): one bucket per
/// integer — 3 is LOOP-001's threshold — with `6+` as the open top.
const DEPTH_BUCKETS: [&str; 7] = ["0", "1", "2", "3", "4", "5", "6+"];

/// Bucketize one cyclomatic-complexity value (schema minimum 1; sub-minimum
/// values clamp into the first bucket).
fn complexity_bucket(value: u32) -> &'static str {
    if value < 5 {
        "01-04"
    } else if value < 10 {
        "05-09"
    } else if value < 15 {
        "10-14"
    } else if value < 25 {
        "15-24"
    } else if value < 50 {
        "25-49"
    } else {
        "50+"
    }
}

/// Bucketize one max-loop-depth value (open top at 6).
fn depth_bucket(value: u32) -> &'static str {
    match value {
        0 => "0",
        1 => "1",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        _ => "6+",
    }
}

/// `stats.distributions[]` producer (v1.4 step 1): exactly two histograms
/// over `stats.functions[]` — `cyclomatic_complexity` and `max_loop_depth`
/// — with the FULL fixed label set emitted per metric INCLUDING zero-count
/// buckets (fixed buckets keep report tables diffable across runs; the
/// per-metric count sum equals `len(functions)`). Output is sorted by
/// `(metric, bucket)` via [`sort_distributions`]. Returns `None` on a
/// zero-function scan so the canonical absent form is preserved there.
///
/// Determinism: counting goes through a `BTreeMap` keyed `(metric, bucket)`
/// and the only ordering obligation is the aggregator-owned sort — no
/// timestamps, floats, or environment-dependent values.
fn distributions(functions: &[schema::FunctionStats]) -> Option<Vec<DistributionEntry>> {
    if functions.is_empty() {
        return None;
    }
    let mut counts: BTreeMap<(&str, &str), u32> = BTreeMap::new();
    for bucket in COMPLEXITY_BUCKETS {
        counts.insert(("cyclomatic_complexity", bucket), 0);
    }
    for bucket in DEPTH_BUCKETS {
        counts.insert(("max_loop_depth", bucket), 0);
    }
    for f in functions {
        if let Some(count) = counts.get_mut(&(
            "cyclomatic_complexity",
            complexity_bucket(f.cyclomatic_complexity),
        )) {
            *count += 1;
        }
        if let Some(count) = counts.get_mut(&("max_loop_depth", depth_bucket(f.max_loop_depth))) {
            *count += 1;
        }
    }
    let mut entries: Vec<DistributionEntry> = counts
        .into_iter()
        .map(|((metric, bucket), count)| DistributionEntry {
            metric: metric.to_string(),
            bucket: bucket.to_string(),
            count,
        })
        .collect();
    sort_distributions(&mut entries);
    Some(entries)
}

/// Sort `stats.distributions[]` by `(metric, bucket)` ascending — the
/// aggregator-owned contract per modules.md §1. Live since v1.4 step 1 (the
/// [`distributions`] producer calls it during assembly); unit-tested below.
#[allow(clippy::ptr_arg)] // Vec is the ownership-contract shape here, not a slice view
pub(crate) fn sort_distributions(entries: &mut Vec<DistributionEntry>) {
    entries.sort_by(|a, b| a.metric.cmp(&b.metric).then(a.bucket.cmp(&b.bucket)));
}

/// `findings[]` passthrough in the analyzer's total order, each lifted into
/// its schema shape with the enclosing-function symbol resolved when one
/// contains the finding span (schema-optional `symbol`; resolution is
/// FIRST-HIT in backend emission order over the sorted units — deterministic
/// per input, not a `(path, line, col)` ranking of candidates).
fn findings_with_symbols(units_sorted: Vec<&FileUnit>, findings: &[Finding]) -> Vec<FindingEntry> {
    findings
        .iter()
        .map(|finding| {
            let symbol = units_sorted
                .iter()
                .filter(|u| u.file.path == finding.path)
                .flat_map(|u| u.functions.iter())
                .find(|f| f.record.span.intersects(&finding.span))
                .map(|f| f.record.name.clone());
            schema::finding_entry(finding, symbol)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    //! Sort-order locks (modules.md §1 rows owned by the aggregator),
    //! fixture-driven per the P1 test plan; m6 include-graph admission;
    //! call_graph + components assembly per P4 Wave 2b.

    use super::*;
    use crate::analyzer::{Confidence, Severity};
    use crate::ir::{
        CallEvent, CalleeKind, EventMeta, IncludeEdge, LocCounts, Span, StatementRole,
    };
    use crate::scanner::FunctionUnit;

    fn unit(path: &str) -> FileUnit {
        FileUnit {
            file: crate::ir::FileRecord {
                path: path.to_string(),
                language: "c".to_string(),
                loc: LocCounts::default(),
                function_count: 0,
                global_var_count: 0,
                include_counts: Default::default(),
            },
            ..FileUnit::default()
        }
    }

    fn func(path: &str, name: &str, line: u32, col: u32) -> FunctionUnit {
        FunctionUnit {
            record: FunctionRecord {
                id: format!("{path}#{name}#{line}:{col}"),
                name: name.to_string(),
                path: path.to_string(),
                // A body-extending span so enclosing-function lookups
                // (symbol resolution) can hit findings inside it.
                span: crate::ir::Span {
                    start_line: line,
                    start_col: col,
                    end_line: line + 20,
                    end_col: 1,
                },
                cyclomatic_complexity: 1,
                ..FunctionRecord::default()
            },
            ..FunctionUnit::default()
        }
    }

    fn quoted_edge(from: &str, to: Option<&str>, resolved: bool) -> IncludeEdge {
        IncludeEdge {
            from_path: from.to_string(),
            to_path: to.map(str::to_string),
            form: IncludeForm::Quoted,
            resolved,
            is_mod: false,
            span: Span::point(1, 1),
        }
    }

    fn edge(from: &str, to: &str, resolved: bool) -> IncludeEdge {
        quoted_edge(from, Some(to), resolved)
    }

    /// Unresolved ANGLE edge — raw target text stays in `to_path`
    /// (ir-v1.md §3.1 lifecycle), `resolved=false`: the external bucket.
    fn angle_edge(from: &str, raw_target: &str) -> IncludeEdge {
        IncludeEdge {
            from_path: from.to_string(),
            to_path: Some(raw_target.to_string()),
            form: IncludeForm::Angle,
            resolved: false,
            is_mod: false,
            span: Span::point(1, 1),
        }
    }

    fn direct_call(callee: &str, line: u32, col: u32) -> CallEvent {
        CallEvent {
            meta: EventMeta {
                span: Span::point(line, col),
                ..EventMeta::default()
            },
            callee_name: callee.to_string(),
            callee_kind: CalleeKind::Direct,
            callee_path: None,
            is_macro: false,
            args: vec![],
            statement_role: StatementRole::Unknown,
            conditional_guarded: false,
            semantic: None,
            receiver_root: None,
        }
    }

    fn finding(path: &str, line: u32, col: u32) -> Finding {
        Finding {
            threshold: None,
            rule_id: "PERF-C-ALLOC-001".to_string(),
            title: "Heap allocation inside loop".to_string(),
            message: "candidate".to_string(),
            severity: Severity::Hot,
            confidence: Confidence::High,
            resolver_tier: crate::ir::ResolverTier::Syntactic,
            span: Span::point(line, col),
            path: path.to_string(),
            evidence: Default::default(),
            macro_suspect: false,
            error_node_overlap: false,
        }
    }

    #[test]
    fn functions_sorted_by_path_then_line_then_col() {
        let mut b = unit("b.c");
        b.functions = vec![func("b.c", "second", 10, 1), func("b.c", "first", 2, 5)];
        let mut a = unit("a.c");
        a.functions = vec![func("a.c", "late", 9, 3), func("a.c", "early", 9, 1)];

        let doc = assemble(&[b, a], &[], &prov());
        let keys: Vec<(String, u32, u32)> = doc
            .stats
            .functions
            .iter()
            .map(|f| (f.path.clone(), f.span.start_line, f.span.start_col))
            .collect();
        assert_eq!(
            keys,
            [
                ("a.c".to_string(), 9, 1),
                ("a.c".to_string(), 9, 3),
                ("b.c".to_string(), 2, 5),
                ("b.c".to_string(), 10, 1),
            ]
        );
    }

    /// m6 admission semantics (Wave 2b — deliberately replaces the old
    /// unresolved-exclusion contract): unresolved QUOTED edges PRESENT
    /// (`to: null`, `resolved: false`); unresolved ANGLE edges ABSENT;
    /// resolved internal edges unchanged; `(from, to)` sort with
    /// None-before-Some pinned; duplicates preserved.
    #[test]
    fn include_graph_m6_admission_sort_and_duplicates() {
        let mut z = unit("z.c");
        z.includes = vec![
            edge("z.c", "shared.h", true),
            // Unresolved QUOTED: stays in the graph, to_path null.
            quoted_edge("z.c", None, false),
            // Duplicate of the same miss at a different directive site:
            // preserved, distinguishable by span.
            {
                let mut e = quoted_edge("z.c", None, false);
                e.span = Span::point(7, 3);
                e
            },
            // Unresolved ANGLE (external bucket): excluded.
            angle_edge("z.c", "<stdio.h>"),
        ];
        let mut a = unit("a.c");
        a.includes = vec![edge("a.c", "z.c", true)];
        let mut m = unit("m.c");
        m.includes = vec![edge("m.c", "shared.h", true)];

        let doc = assemble(&[z, m, a], &[], &prov());
        let graph = doc
            .graphs
            .as_ref()
            .expect("graphs present")
            .include_graph
            .clone();
        let node_paths: Vec<&str> = graph.nodes.iter().map(|n| n.path.as_str()).collect();
        assert_eq!(node_paths, ["a.c", "m.c", "z.c"]);

        // Angle-unresolved absent everywhere.
        assert!(
            graph
                .edges
                .iter()
                .all(|e| e.form != IncludeForm::Angle || e.resolved),
            "unresolved angle edges must stay excluded"
        );
        // Resolved edges unchanged.
        assert!(
            graph
                .edges
                .iter()
                .filter(|e| e.resolved)
                .all(|e| e.to.is_some()),
            "resolved edges keep their to_path"
        );

        // The two z.c → null duplicates survive with distinct spans.
        let misses: Vec<&IncludeEdgeDto> = graph
            .edges
            .iter()
            .filter(|e| e.from == "z.c" && !e.resolved)
            .collect();
        assert_eq!(misses.len(), 2, "duplicates must be preserved");
        assert!(misses.iter().all(|e| e.to.is_none()));
        let mut miss_spans: Vec<_> = misses.iter().map(|e| e.span).collect();
        miss_spans.sort();
        assert_ne!(miss_spans[0], miss_spans[1], "distinguishable by span");

        // Sort pin: (from, to) ascending with None BEFORE Some(_) among the
        // same from. z.c's edges must therefore be [null-miss, null-miss,
        // "shared.h"].
        let z_edges: Vec<Option<&str>> = graph
            .edges
            .iter()
            .filter(|e| e.from == "z.c")
            .map(|e| e.to.as_deref())
            .collect();
        assert_eq!(
            z_edges,
            [None, None, Some("shared.h")],
            "Option<String> Ord puts None before every Some(_)"
        );
        // Whole-edge-list non-decreasing under the same comparator.
        let keys: Vec<(&str, Option<&str>)> = graph
            .edges
            .iter()
            .map(|e| (e.from.as_str(), e.to.as_deref()))
            .collect();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(keys, sorted_keys);
    }

    #[test]
    fn findings_pass_through_in_analyzer_order_with_symbols() {
        // Analyzer already ordered these (hot→…); aggregator must NOT reorder.
        let ordered = vec![finding("b.c", 20, 1), finding("a.c", 5, 2)];
        let mut b = unit("b.c");
        b.functions = vec![func("b.c", "enclosing", 18, 1)];
        let a = unit("a.c");

        let doc = assemble(&[b, a], &ordered, &prov());
        let ids: Vec<String> = doc.findings.iter().map(|f| f.finding_id.clone()).collect();
        assert_eq!(
            ids,
            ["PERF-C-ALLOC-001:b.c:20:1", "PERF-C-ALLOC-001:a.c:5:2",]
        );
        assert_eq!(doc.findings[0].symbol.as_deref(), Some("enclosing"));
        assert_eq!(
            doc.findings[1].symbol, None,
            "no enclosing function ⇒ omitted"
        );
    }

    #[test]
    fn zero_file_scan_yields_valid_empty_document() {
        let doc = assemble(&[], &[], &prov());
        assert_eq!(doc.scan.languages, ["c", "python", "rust"]);
        assert_eq!(doc.scan.file_count, 0);
        assert!(doc.files.is_empty());
        assert!(doc.findings.is_empty());
        assert_eq!(
            doc.parse_health_summary,
            Some(ParseHealthSummary {
                ok: 0,
                partial: 0,
                failed: 0
            })
        );
        assert!(doc.scan.loc_total.is_none());
        let graphs = doc.graphs.expect("graphs present");
        assert!(graphs.include_graph.nodes.is_empty() && graphs.include_graph.edges.is_empty());
        // call_graph ALWAYS Some — empty vec on a zero-edge scan.
        assert!(
            graphs
                .call_graph
                .as_ref()
                .is_some_and(|cg| cg.edges.is_empty()),
            "call_graph must be Some(empty), got {graphs:?}"
        );
        // components ALWAYS Some — empty vec on a zero-file scan.
        assert!(
            doc.components.as_ref().is_some_and(Vec::is_empty),
            "components must be Some(empty), got {:?}",
            doc.components
        );
    }

    /// `scan.languages` derives from the scanned files: C-only → `["c"]`,
    /// Rust-only → `["rust"]`, Python-only → `["python"]` (D-A, plan
    /// `rust-backend` R2; extended by plan `python-backend` W3).
    #[test]
    fn scan_languages_derive_from_single_language_units() {
        let c_doc = assemble(&[unit("a.c")], &[], &prov());
        assert_eq!(c_doc.scan.languages, ["c"]);

        let mut rust = unit("a.rs");
        rust.file.language = "rust".to_string();
        let rust_doc = assemble(&[rust], &[], &prov());
        assert_eq!(rust_doc.scan.languages, ["rust"]);

        let mut python = unit("a.py");
        python.file.language = "python".to_string();
        let python_doc = assemble(&[python], &[], &prov());
        assert_eq!(python_doc.scan.languages, ["python"]);
    }

    /// Mixed C+Python+Rust → sorted-unique `["c","python","rust"]`,
    /// deterministic regardless of unit order (fed in REVERSE to prove the
    /// BTreeSet sorts).
    #[test]
    fn scan_languages_mixed_c_and_rust_are_sorted_unique() {
        let mut rust = unit("a.rs");
        rust.file.language = "rust".to_string();
        let mut python = unit("a.py");
        python.file.language = "python".to_string();
        let doc = assemble(&[rust, python, unit("a.c")], &[], &prov());
        assert_eq!(doc.scan.languages, ["c", "python", "rust"]);
    }

    /// Zero-file scan → the supported set (schema `minItems: 1` preserved;
    /// semantics = "languages scanned for").
    #[test]
    fn scan_languages_empty_scan_uses_supported_set() {
        let doc = assemble(&[], &[], &prov());
        assert_eq!(doc.scan.languages, ["c", "python", "rust"]);
    }

    /// call_graph assembly: a genuine cross-file Direct call maps field-for-
    /// field onto CallEdge (confidence locked LOW via the constructor), and
    /// the aggregator re-sorts by `(from_function, to_name, from_path)`.
    #[test]
    fn call_graph_maps_cross_file_edges_and_reorders() {
        // a.c: two callers; b.c defines both callees. Producer emits per
        // unit/function walk; aggregator owns final document order.
        let mut caller_a = func("a.c", "zcaller", 4, 1);
        caller_a.calls = vec![direct_call("helper", 6, 5)];
        let mut caller_b = func("a.c", "acaller", 30, 1);
        caller_b.calls = vec![direct_call("util", 32, 5)];
        let mut a = unit("a.c");
        a.functions = vec![caller_b, caller_a]; // deliberately unsorted

        let mut helper_def = func("b.c", "helper", 2, 1);
        helper_def.record.cyclomatic_complexity = 3;
        let mut util_def = func("b.c", "util", 10, 1);
        util_def.record.cyclomatic_complexity = 3;
        let mut b = unit("b.c");
        b.functions = vec![helper_def, util_def];

        // Intra-file call (a.c defines its own local_fn): must NOT appear.
        let mut self_contained = func("c.c", "lonely", 1, 1);
        self_contained.calls = vec![direct_call("local_fn", 3, 5)];
        let mut local = func("c.c", "local_fn", 8, 1);
        local.record.cyclomatic_complexity = 3;
        let mut c = unit("c.c");
        c.functions = vec![self_contained, local];

        let doc = assemble(&[a, b, c], &[], &prov());
        let call_graph = doc
            .graphs
            .as_ref()
            .expect("graphs present")
            .call_graph
            .clone()
            .expect("call_graph always Some");

        let keys: Vec<(String, String, String)> = call_graph
            .edges
            .iter()
            .map(|e| {
                (
                    e.from_function.clone(),
                    e.to_name.clone(),
                    e.from_path.clone(),
                )
            })
            .collect();
        assert_eq!(
            keys,
            [
                (
                    "a.c#acaller#30:1".to_string(),
                    "util".to_string(),
                    "a.c".to_string()
                ),
                (
                    "a.c#zcaller#4:1".to_string(),
                    "helper".to_string(),
                    "a.c".to_string()
                ),
            ],
            "(from_function, to_name, from_path) ascending"
        );

        let edge = &call_graph.edges[0];
        assert_eq!(edge.confidence, "LOW");
        assert_eq!(edge.to_path.as_deref(), Some("b.c"));
        // Intra-file edge excluded entirely.
        assert!(
            call_graph.edges.iter().all(|e| e.to_name != "local_fn"),
            "file-local calls are omitted in v1"
        );

        // Unit with no cross-file calls ⇒ still Some(empty).
        let solo_doc = assemble(&[unit("solo.c")], &[], &prov());
        let solo_cg = solo_doc.graphs.expect("graphs").call_graph.expect("Some");
        assert!(solo_cg.edges.is_empty());
    }

    /// v1.4 step 2 slice A locks: whole-program call-shape over resolved
    /// edges — fanout/fanin counts, topo-layer depth, self + cyclic
    /// recursion, own-file-first, uniqueness gate, semantic precedence.
    #[test]
    fn call_shape_counts_depth_and_recursion() {
        // b.c: helper (leaf) + util calling helper (own-file-first edge).
        let helper_def = func("b.c", "helper", 2, 1);
        let mut util_def = func("b.c", "util", 10, 1);
        util_def.calls = vec![direct_call("helper", 12, 5)];
        let mut b = unit("b.c");
        b.functions = vec![helper_def, util_def];

        // a.c: main calls helper (global-unique ⇒ cross-file edge) +
        // missing (no definition ⇒ no edge).
        let mut main = func("a.c", "main", 4, 1);
        main.calls = vec![direct_call("helper", 6, 5), direct_call("missing", 7, 5)];
        let mut a = unit("a.c");
        a.functions = vec![main];

        // c.c: self-recursive.
        let mut recurse = func("c.c", "recurse", 1, 1);
        recurse.calls = vec![direct_call("recurse", 3, 5)];
        let mut c = unit("c.c");
        c.functions = vec![recurse];

        // d.c ↔ e.c: two-cycle.
        let mut cyc_a = func("d.c", "cyc_a", 1, 1);
        cyc_a.calls = vec![direct_call("cyc_b", 2, 5)];
        let mut d = unit("d.c");
        d.functions = vec![cyc_a];
        let mut cyc_b = func("e.c", "cyc_b", 1, 1);
        cyc_b.calls = vec![direct_call("cyc_a", 2, 5)];
        let mut e = unit("e.c");
        e.functions = vec![cyc_b];

        let doc = assemble(&[a, b, c, d, e], &[], &prov());
        let shape = |id: &str| {
            doc.stats
                .functions
                .iter()
                .find(|f| f.id == id)
                .unwrap_or_else(|| panic!("function {id} present"))
        };

        let main = shape("a.c#main#4:1");
        assert_eq!(main.call_fanout, Some(1), "helper only; missing unresolved");
        assert_eq!(main.call_fanin, Some(0));
        let helper = shape("b.c#helper#2:1");
        assert_eq!(helper.call_fanin, Some(2), "main + util");
        assert_eq!(helper.call_fanout, Some(0));
        assert_eq!(helper.call_depth, Some(1), "callee below two callers");
        let util = shape("b.c#util#10:1");
        assert_eq!(util.call_fanout, Some(1), "own-file-first edge to helper");
        assert_eq!(util.call_depth, Some(0), "entries sit at layer 0");
        assert_eq!(main.call_depth, Some(0));

        let recurse = shape("c.c#recurse#1:1");
        assert_eq!(recurse.is_recursive, Some(true), "self-loop");
        assert_eq!(recurse.call_fanout, Some(1));
        assert_eq!(recurse.call_fanin, Some(1));

        for id in ["d.c#cyc_a#1:1", "e.c#cyc_b#1:1"] {
            let f = shape(id);
            assert_eq!(f.is_recursive, Some(true), "{id} is in a 2-cycle");
            assert_eq!(f.call_fanout, Some(1));
            assert_eq!(f.call_fanin, Some(1));
        }
        assert_eq!(helper.is_recursive, Some(false));
        assert_eq!(main.is_recursive, Some(false));
    }

    /// Ambiguous names (two definitions) and undefined names yield NO edge —
    /// the M1 uniqueness gate applies to stats as well as findings.
    #[test]
    fn call_shape_skips_ambiguous_and_undefined_callees() {
        let mut d1 = unit("d1.c");
        d1.functions = vec![func("d1.c", "dup", 1, 1)];
        let mut d2 = unit("d2.c");
        d2.functions = vec![func("d2.c", "dup", 1, 1)];
        let mut caller = func("q.c", "caller", 1, 1);
        caller.calls = vec![direct_call("dup", 2, 5), direct_call("undefined_fn", 3, 5)];
        let mut q = unit("q.c");
        q.functions = vec![caller];

        let doc = assemble(&[d1, d2, q], &[], &prov());
        let get = |id: &str| {
            doc.stats
                .functions
                .iter()
                .find(|f| f.id == id)
                .unwrap_or_else(|| panic!("{id} present"))
        };
        assert_eq!(get("q.c#caller#1:1").call_fanout, Some(0));
        assert_eq!(get("d1.c#dup#1:1").call_fanin, Some(0));
        assert_eq!(get("d2.c#dup#1:1").call_fanin, Some(0));
    }

    /// Semantic facts take precedence over own-file-first: the caller file
    /// defines `helper` locally, but the resolved definition points at y.c.
    #[test]
    fn call_shape_prefers_semantic_facts_over_name_matching() {
        use crate::ir::{CallSemantic, ResolvedSite};

        let mut caller = func("x.c", "caller", 1, 1);
        let mut resolved = direct_call("helper", 2, 5);
        resolved.semantic = Some(CallSemantic {
            resolved_definition: Some(ResolvedSite {
                path: "y.c".to_string(),
                span: Span::point(1, 1),
            }),
            fopened_handle: false,
            literal_args: Vec::new(),
        });
        caller.calls = vec![resolved];
        let mut x = unit("x.c");
        x.functions = vec![
            caller,
            func("x.c", "helper", 20, 1), // local shadow, must NOT win
        ];
        let mut y = unit("y.c");
        // Span must intersect the resolved site span (point 1:1).
        y.functions = vec![func("y.c", "helper", 1, 1)];

        let doc = assemble(&[x, y], &[], &prov());
        let get = |id: &str| {
            doc.stats
                .functions
                .iter()
                .find(|f| f.id == id)
                .unwrap_or_else(|| panic!("{id} present"))
        };
        assert_eq!(get("y.c#helper#1:1").call_fanin, Some(1));
        assert_eq!(get("x.c#helper#20:1").call_fanin, Some(0));
        assert_eq!(get("x.c#caller#1:1").call_fanout, Some(1));
    }

    /// Same units in shuffled order ⇒ identical `stats.functions[]`
    /// (byte-determinism of the whole-program pass).
    #[test]
    fn call_shape_is_deterministic_under_shuffled_input() {
        let mut caller = func("q.c", "caller", 1, 1);
        caller.calls = vec![direct_call("helper", 2, 5)];
        let mut q = unit("q.c");
        q.functions = vec![caller];
        let mut h = unit("h.c");
        h.functions = vec![func("h.c", "helper", 5, 1)];

        let fwd = assemble(&[q.clone(), h.clone()], &[], &prov());
        let rev = assemble(&[h, q], &[], &prov());
        assert_eq!(fwd.stats.functions, rev.stats.functions);
    }

    #[test]
    fn parse_health_summary_counts_statuses() {
        let mut ok_u = unit("ok.c");
        ok_u.parse_health.status = crate::ir::ParseStatus::Ok;
        let mut part = unit("part.c");
        part.parse_health.status = crate::ir::ParseStatus::Partial;
        let mut bad = unit("bad.c");
        bad.parse_health.status = crate::ir::ParseStatus::Failed;

        let doc = assemble(&[bad, ok_u, part], &[], &prov());
        assert_eq!(
            doc.parse_health_summary,
            Some(ParseHealthSummary {
                ok: 1,
                partial: 1,
                failed: 1
            })
        );
    }

    /// components[]: directory grouping (incl. root "."), density math with
    /// the KLOC==0 guard, finding_count population, rank under density
    /// inversion (higher density beats higher complexity), emission order
    /// == (rank, path_prefix).
    #[test]
    fn components_group_rank_and_density_math() {
        // app/main.c: 1000 code LOC ⇒ kloc 1.0; one modest function.
        let mut app_unit = unit("app/main.c");
        app_unit.file.loc = LocCounts {
            code: 1000,
            comment: 0,
            blank: 0,
        };
        let mut app_fn = func("app/main.c", "run", 4, 1);
        app_fn.record.cyclomatic_complexity = 5;
        app_fn.record.max_loop_depth = 2;
        app_fn.record.alloc_site_count = 3;
        app_unit.functions = vec![app_fn];

        // lib/util.c: 2000 code LOC ⇒ kloc 2.0; HIGH complexity but LOW
        // density inputs.
        let mut lib_unit = unit("lib/util.c");
        lib_unit.file.loc = LocCounts {
            code: 2000,
            comment: 0,
            blank: 0,
        };
        let mut lib_f1 = func("lib/util.c", "big_one", 2, 1);
        lib_f1.record.cyclomatic_complexity = 10;
        lib_f1.record.max_loop_depth = 1;
        let mut lib_f2 = func("lib/util.c", "big_two", 40, 1);
        lib_f2.record.cyclomatic_complexity = 10;
        lib_f2.record.alloc_site_count = 2;
        lib_unit.functions = vec![lib_f1, lib_f2];

        // Root file: zero code LOC ⇒ KLOC==0 guard must zero the densities.
        let mut root_unit = unit("top.c");
        let mut root_fn = func("top.c", "tiny", 1, 1);
        root_fn.record.cyclomatic_complexity = 1;
        root_unit.functions = vec![root_fn];

        // Findings: 2 in app/, 1 in lib/, 0 at root.
        let findings = vec![
            finding("app/main.c", 6, 5),
            finding("app/main.c", 8, 5),
            finding("lib/util.c", 44, 5),
        ];

        let doc = assemble(&[root_unit, lib_unit, app_unit], &findings, &prov());
        let comps = doc.components.expect("components always Some");

        let prefixes: Vec<&str> = comps.iter().map(|c| c.path_prefix.as_str()).collect();
        assert_eq!(
            prefixes,
            ["app", "lib", "."],
            "emission order (rank, path_prefix)"
        );
        let ranks: Vec<u32> = comps.iter().map(|c| c.rank).collect();
        assert_eq!(ranks, [1, 2, 3]);
        // component_id mirrors path_prefix verbatim.
        assert!(
            comps.iter().all(|c| c.component_id == c.path_prefix),
            "component_id == path_prefix"
        );

        // Rank-1 "app": density inversion — complexity_sum 5 < lib's 20, yet
        // finding_density 2.0 > lib's 0.5 wins rank 1.
        let app = &comps[0];
        assert_eq!(app.finding_count, Some(2));
        assert_eq!(app.metrics.complexity_sum, 5.0);
        assert_eq!(app.metrics.max_loop_depth, 2);
        assert_eq!(app.metrics.alloc_site_density, 3.0); // 3 sites / 1.0 kloc
        assert_eq!(app.metrics.finding_density, 2.0); // 2 findings / 1.0 kloc

        let lib = &comps[1];
        assert_eq!(lib.finding_count, Some(1));
        assert_eq!(lib.metrics.complexity_sum, 20.0);
        assert_eq!(lib.metrics.max_loop_depth, 1);
        assert_eq!(lib.metrics.alloc_site_density, 1.0); // 2 / 2.0
        assert_eq!(lib.metrics.finding_density, 0.5); // 1 / 2.0

        // Root ".": KLOC==0 ⇒ densities guarded to 0.0 (never NaN/inf);
        // finding_count still populated as Some(0).
        let root = &comps[2];
        assert_eq!(root.path_prefix, ".");
        assert_eq!(root.finding_count, Some(0));
        assert_eq!(root.metrics.complexity_sum, 1.0);
        assert_eq!(root.metrics.max_loop_depth, 0);
        assert_eq!(root.metrics.alloc_site_density, 0.0);
        assert_eq!(root.metrics.finding_density, 0.0);
    }

    /// §1 ownership row for `stats.distributions`: the `(metric, bucket)`
    /// comparator is proven on synthetic entries even though no producer
    /// populates the field yet (reserved-for-future).
    #[test]
    fn sort_distributions_orders_by_metric_then_bucket() {
        let mut entries = vec![
            DistributionEntry {
                metric: "length_lines".to_string(),
                bucket: "11-20".to_string(),
                count: 3,
            },
            DistributionEntry {
                metric: "cyclomatic_complexity".to_string(),
                bucket: "6-10".to_string(),
                count: 2,
            },
            DistributionEntry {
                metric: "length_lines".to_string(),
                bucket: "1-10".to_string(),
                count: 1,
            },
            DistributionEntry {
                metric: "cyclomatic_complexity".to_string(),
                bucket: "1-5".to_string(),
                count: 5,
            },
        ];
        sort_distributions(&mut entries);
        let keys: Vec<(String, String)> = entries
            .iter()
            .map(|e| (e.metric.clone(), e.bucket.clone()))
            .collect();
        assert_eq!(
            keys,
            [
                ("cyclomatic_complexity".to_string(), "1-5".to_string()),
                ("cyclomatic_complexity".to_string(), "6-10".to_string()),
                ("length_lines".to_string(), "1-10".to_string()),
                ("length_lines".to_string(), "11-20".to_string()),
            ]
        );
        // Counts travel with their buckets.
        assert_eq!(entries[0].count, 5);
        assert_eq!(entries[3].count, 3);
    }

    fn prov() -> Provenance {
        Provenance {
            root_path: ".".to_string(),
            started_at: "2026-08-24T00:00:00Z".to_string(),
            duration_ms: 7,
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// v1.4 step 1 producer lock: bucket edges (incl. the CALL-002 ≥15 and
    /// LOOP-001 ≥3 promotion zones), the full fixed label set with
    /// zero-count buckets present, `(metric, bucket)` order, and per-metric
    /// sums equal to the function count.
    #[test]
    fn distributions_populates_fixed_buckets_in_sort_order() {
        use crate::schema::FunctionStats;
        let stat = |complexity: u32, depth: u32| FunctionStats {
            id: "a.c#f#1:1".to_string(),
            name: "f".to_string(),
            path: "a.c".to_string(),
            span: Span::point(1, 1),
            param_count: 0,
            cyclomatic_complexity: complexity,
            cognitive_complexity: 0,
            max_loop_depth: depth,
            alloc_site_count: 0,
            length_lines: 3,
            call_fanout: None,
            call_fanin: None,
            call_depth: None,
            is_recursive: None,
            idiom_hash: None,
            error_path: None,
            alloc_family: None,
            cfg_gated: None,
            // v1.5: entry/reachability play no role in distributions.
            entry_kind: None,
            entry_confidence: None,
            reachable_from_entry: None,
        };
        let functions = vec![
            stat(2, 0),   // 01-04, 0
            stat(7, 3),   // 05-09, 3
            stat(15, 6),  // 15-24, 6+
            stat(60, 10), // 50+, 6+
        ];
        let entries = distributions(&functions).expect("non-empty functions ⇒ Some");
        // Full fixed label sets: 6 complexity + 7 depth buckets.
        assert_eq!(entries.len(), 13);
        let keys: Vec<(String, String)> = entries
            .iter()
            .map(|e| (e.metric.clone(), e.bucket.clone()))
            .collect();
        let mut sorted_keys = keys.clone();
        sorted_keys.sort();
        assert_eq!(keys, sorted_keys, "(metric, bucket) ascending");
        let count_of = |metric: &str, bucket: &str| {
            entries
                .iter()
                .find(|e| e.metric == metric && e.bucket == bucket)
                .unwrap_or_else(|| panic!("bucket {metric}/{bucket} missing"))
                .count
        };
        assert_eq!(count_of("cyclomatic_complexity", "01-04"), 1);
        assert_eq!(count_of("cyclomatic_complexity", "05-09"), 1);
        assert_eq!(count_of("cyclomatic_complexity", "10-14"), 0);
        assert_eq!(count_of("cyclomatic_complexity", "15-24"), 1);
        assert_eq!(count_of("cyclomatic_complexity", "25-49"), 0);
        assert_eq!(count_of("cyclomatic_complexity", "50+"), 1);
        assert_eq!(count_of("max_loop_depth", "0"), 1);
        assert_eq!(count_of("max_loop_depth", "3"), 1);
        assert_eq!(count_of("max_loop_depth", "6+"), 2);
        // Per-metric sums equal the function count.
        for metric in ["cyclomatic_complexity", "max_loop_depth"] {
            let sum: u32 = entries
                .iter()
                .filter(|e| e.metric == metric)
                .map(|e| e.count)
                .sum();
            assert_eq!(sum, 4, "per-metric sum for {metric}");
        }
    }

    /// Zero-function scan ⇒ `None` (canonical absent form preserved).
    #[test]
    fn distributions_absent_on_zero_functions() {
        assert_eq!(distributions(&[]), None);
    }

    /// Assembly wires the producer: a unit with functions yields
    /// `Some(distributions)`; an empty scan stays `None`.
    #[test]
    fn assemble_populates_distributions_for_nonempty_scans() {
        let mut u = unit("a.c");
        u.functions = vec![func("a.c", "f", 1, 1)];
        let doc = assemble(&[u], &[], &prov());
        let entries = doc
            .stats
            .distributions
            .as_ref()
            .expect("distributions populated");
        assert_eq!(entries.len(), 13);
        assert!(doc.stats.distributions.is_some());

        let empty = assemble(&[], &[], &prov());
        assert_eq!(empty.stats.distributions, None);
    }
}
