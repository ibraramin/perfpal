//! Story bundle projection (plan `polish-report` H1 / step 2A).
//!
//! One deterministic [`StoryBundle`] from a finished [`MetricsDocument`]:
//!
//! - `layout_json` is the `atlas-layout.json` payload produced by
//!   [`crate::reporter::atlas_layout::to_json`] — the Rust side never
//!   re-derives geometry here;
//! - `payload_json` is the camelCase projection consumed by `sim/sim.js`. It
//!   is the Rust replacement for `sim/from-layout.py`'s `build()` and keeps
//!   that file's ordering, caps and honesty rules.
//!
//! ## Filesystem probes
//!
//! [`build`] takes an optional scan root. `Some(root)` runs the same two
//! story-only probes as [`crate::reporter::atlas_layout_json`]: the P3
//! signature/I/O parse over the drawn files and the P6 README purpose read.
//! `None` substitutes named empty stubs, so tests and renderer goldens can
//! build a bundle without touching the filesystem.
//!
//! ## Fingerprints (no new dependency)
//!
//! The Python converter used SHA-256 from the standard library. This crate
//! deliberately keeps its dependency closure pinned, so integrity fields use a
//! deterministic FNV-1a 64-bit hash rendered as 16 lowercase hex characters.
//! The names say what they are: `layoutFingerprint` hashes the emitted file
//! bytes (the writer-normalised `layout_json` + trailing LF);
//! `metricsContentFingerprint` hashes the canonical
//! metrics JSON with `scan.started_at`, `scan.duration_ms` and
//! `scan.root_path` removed (those change per run/machine). They are
//! change-detection tokens, not cryptographic digests.
//!
//! ## Determinism
//!
//! No set or hash-map iteration reaches the output: groups are `BTreeMap`s,
//! literal pairs are a `BTreeSet`, and every array gets an explicit total
//! order. `serde_json` runs without `preserve_order`, so object keys are
//! sorted. The same document and root therefore produce byte-identical
//! `layout_json` and `payload_json`.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::{Value, json};

use crate::analysis::flow::{FLOW_VERSION, Flow};
use crate::reporter::atlas;
use crate::reporter::atlas_brief;
use crate::reporter::atlas_layout;
use crate::reporter::atlas_purpose::{self, Purpose};
use crate::reporter::atlas_queue;
use crate::reporter::atlas_render;
use crate::reporter::atlas_scenes;
use crate::schema::{FunctionStats, MetricsDocument};

/// Per-file I/O rows carried into the story inspector.
const IO_ROWS_PER_FILE: usize = 6;
/// Unresolved-call rows carried per file.
const UNRESOLVED_ROWS: usize = 6;
/// Literal examples carried into the trust beat.
const IO_TRUST_LITERALS: usize = 4;
/// Purpose sentence limit mirrored from `atlas_purpose` (200 + ellipsis).
const PURPOSE_MAX_CHARS: usize = 201;

/// The two story artifacts built from one scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoryBundle {
    /// Exact `atlas-layout.json` bytes.
    pub layout_json: String,
    /// CamelCase payload consumed by `sim/sim.js`.
    pub payload_json: String,
}

/// Build the layout and payload projection for `doc`.
///
/// `root = Some(path)` runs the story-only flow + README probes exactly like
/// [`crate::reporter::atlas_layout_json`]; `None` uses empty stubs and performs
/// no filesystem access.
#[must_use]
pub fn build(doc: &MetricsDocument, root: Option<&Path>) -> StoryBundle {
    let view = atlas::build_atlas_view(doc);
    let actions = atlas_brief::start_actions(doc);
    let findings_included = view
        .nodes
        .iter()
        .map(|node| node.findings.hot + node.findings.warn + node.findings.info)
        .sum();
    let findings_total = doc.findings.len();

    let empty_flow = Flow {
        flow_version: FLOW_VERSION,
        fidelity: "syntactic",
        signatures: Vec::new(),
        calls: Vec::new(),
        io: Vec::new(),
    };
    let empty_purpose = Purpose {
        purpose_version: atlas_purpose::PURPOSE_VERSION,
        text: "No static purpose sentence.".to_string(),
        source: None,
        kind: "structural",
    };
    let (flow, purpose) = match root {
        Some(root) => (
            crate::analysis::flow::build(root, view.nodes.iter().map(|node| node.path.clone())),
            atlas_purpose::read(root, &atlas_render::strip_entry_line(&view)),
        ),
        None => (empty_flow, empty_purpose),
    };

    let profile = atlas_queue::build(doc);
    let scenes = atlas_scenes::build(doc, &view);
    let layout_json = atlas_layout::to_json(
        &view,
        &actions,
        findings_included,
        findings_total,
        Some(&flow),
        Some(&purpose),
        Some(&profile),
        Some(&scenes),
    );
    let layout: Value =
        serde_json::from_str(&layout_json).expect("atlas layout JSON parsed back as a value");
    let payload = payload_value(doc, &layout, &layout_json);

    StoryBundle {
        layout_json,
        payload_json: serde_json::to_string(&payload)
            .expect("story payload is built from serializable primitives"),
    }
}

/// Project the Python converter's `build()` return shape.
/// Fingerprint of the emitted `atlas-layout.json` bytes.
///
/// The writer enforces one trailing LF on every text artifact; hash the same
/// normalised bytes so a consumer can verify the file against the inline
/// `source.layoutFingerprint` (ADR-0014 F6c).
#[must_use]
pub fn layout_fingerprint(layout_json: &str) -> String {
    fnv1a64_hex(format!("{layout_json}\n").as_bytes())
}

fn payload_value(doc: &MetricsDocument, layout: &Value, layout_json: &str) -> Value {
    let nodes = array_at(layout, "nodes");
    let edges = array_at(layout, "edges");
    let actions = array_at(layout, "actions");
    let stages = array_at(layout, "stages");
    let drawn: BTreeSet<String> = nodes
        .iter()
        .filter_map(|node| node.get("path").and_then(Value::as_str))
        .map(str::to_string)
        .collect();

    let flow = layout.get("flow").cloned().unwrap_or(Value::Null);
    let io_grouped = io_rows_by_path(&flow, &drawn);
    let unresolved = unresolved_by_path(&flow, &drawn);
    let calls = calls_by_pair(&flow);
    let purpose = purpose_projection(layout);
    let (profile, profile_meta) = profile_projection(layout, doc);
    let (scenes, scene_meta) = scenes_projection(layout, doc);
    let unknown_trips = unknown_trip_functions(nodes);
    let non_production_nodes = nodes
        .iter()
        .filter(|node| node.get("production").and_then(Value::as_bool) != Some(true))
        .count();
    let unresolved_total: u64 = unresolved
        .values()
        .map(|entry| entry.get("count").and_then(Value::as_u64).unwrap_or(0))
        .sum();
    let unresolved_files = unresolved.len();
    let io_trust = io_trust(&io_grouped);

    let meta = layout.get("meta").cloned().unwrap_or(Value::Null);
    let state = layout.get("state").cloned().unwrap_or(Value::Null);
    let findings_included = meta
        .get("findings_included")
        .cloned()
        .unwrap_or(Value::from(0));
    let findings_total = meta
        .get("findings_total")
        .cloned()
        .unwrap_or(Value::from(0));

    let actions: Vec<Value> = actions
        .iter()
        .map(|action| {
            let anchor = action.get("location").and_then(Value::as_str).unwrap_or("");
            json!({
                "anchor": anchor,
                "title": field(action, "rule"),
                "function": field(action, "function"),
                "onCanvas": drawn.contains(anchor_path(anchor)),
                "scene": field(action, "scene"),
            })
        })
        .collect();

    let scene_omitted = scene_meta.get("omitted").cloned().unwrap_or(Value::Null);
    let scene_omitted_scenes = field(&scene_omitted, "scenes");
    // The complete drawn-slice count, not the per-file top-3 projection
    // carried on `nodes[].functions` (which is a sample).
    let loop_bearing_functions = doc
        .stats
        .functions
        .iter()
        .filter(|function| {
            drawn.contains(&function.path)
                && (function.max_loop_depth > 0 || function.unknown_trip_loops > 0)
        })
        .count();
    let trust = json!({
        "caps": field(&meta, "caps_text"),
        "partialParse": field(&state, "partial_parse"),
        "nonProductionNodes": non_production_nodes,
        "drawnNodes": nodes.len(),
        "unknownTripFunctions": unknown_trips,
        "loopBearingFunctions": loop_bearing_functions,
        "io": io_trust,
        "findingsIncluded": findings_included,
        "findingsTotal": findings_total,
        "purpose": {
            "kind": field(&purpose, "kind"),
            "source": field(&purpose, "source"),
        },
        "profile": {
            "shown": field(&profile_meta, "shown"),
            "candidateTotal": field(&profile_meta, "candidateTotal"),
            "productionRanked": field(&profile_meta, "productionRanked"),
            "totalRanked": field(&profile_meta, "totalRanked"),
            "demotedProduction": field(&profile_meta, "demotedProduction"),
        },
        "scenes": {
            "total": field(&scene_meta, "totalScenes"),
            "shown": field(&scene_meta, "shownScenes"),
            "files": field(&scene_meta, "totalFiles"),
            "shownFiles": field(&scene_meta, "shownFiles"),
            "productionFiles": field(&scene_meta, "productionFiles"),
            "nonProductionFiles": field(&scene_meta, "nonProductionFiles"),
            "omitted": scene_omitted_scenes,
        },
        "unresolvedCalls": unresolved_total,
        "unresolvedFiles": unresolved_files,
    });

    let node_rows: Vec<Value> = nodes
        .iter()
        .map(|node| {
            let path = node.get("path").and_then(Value::as_str).unwrap_or("");
            node_row(
                node,
                io_grouped.get(path).and_then(|rows| io_summary(rows)),
                unresolved.get(path),
            )
        })
        .collect();

    let edge_rows: Vec<Value> = edges
        .iter()
        .enumerate()
        .map(|(order, edge)| edge_row(edge, order, &calls))
        .collect();

    let columns: Vec<Value> = array_at(layout, "columns")
        .iter()
        .enumerate()
        .map(|(order, column)| {
            json!({
                "depth": field(column, "depth"),
                "x": field(column, "x"),
                "order": order,
                "unreached": field(column, "unreached"),
                "label": field(column, "label"),
                "countText": field(column, "count_text"),
            })
        })
        .collect();

    let hulls: Vec<Value> = array_at(layout, "hulls")
        .iter()
        .map(|hull| {
            json!({
                "x": field(hull, "x"),
                "y": field(hull, "y"),
                "w": field(hull, "w"),
                "h": field(hull, "h"),
                "label": field(hull, "label"),
            })
        })
        .collect();

    let trace: Vec<Value> = stages
        .iter()
        .filter_map(|stage| {
            let files = stage.get("files").and_then(Value::as_array)?;
            let first = files.first()?;
            let step = stage.get("order").and_then(Value::as_u64).unwrap_or(0) + 1;
            Some(json!({"path": first, "step": step}))
        })
        .collect();

    let findings = findings_for(doc, &drawn);

    json!({
        "case": "scan",
        "synthetic": false,
        "stage": layout.get("canvas").cloned().unwrap_or(Value::Null),
        "source": {
            "artifact": "inline",
            "layoutVersion": field(layout, "layout_version"),
            "layoutFingerprint": layout_fingerprint(layout_json),
            "metricsContentFingerprint": metrics_content_fingerprint(doc),
        },
        "meta": {
            "entry": field(&meta, "entry_text"),
            "hottest": field(&meta, "hottest_text"),
            "path": field(&meta, "path_text"),
            "caps": field(&meta, "caps_text"),
            "footer": field(&meta, "footer_text"),
            "findingsIncluded": findings_included,
            "findingsTotal": findings_total,
            "actions": actions,
        },
        "trust": trust,
        "purpose": purpose,
        "profile": profile,
        "profileMeta": profile_meta,
        "scenes": scenes,
        "sceneMeta": scene_meta,
        "hottestFunction": hottest_function(nodes),
        "state": state_projection(&state),
        "columns": columns,
        "hulls": hulls,
        "nodes": node_rows,
        "edges": edge_rows,
        "trace": trace,
        "findings": findings,
    })
}

// ---------------------------------------------------------------------------
// Generic JSON access
// ---------------------------------------------------------------------------

fn array_at<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn field(value: &Value, key: &str) -> Value {
    value.get(key).cloned().unwrap_or(Value::Null)
}

/// `path:line` → `path`; locations always render with a numeric tail.
fn anchor_path(anchor: &str) -> &str {
    match anchor.rsplit_once(':') {
        Some((path, line))
            if !path.is_empty() && !line.is_empty() && line.bytes().all(|b| b.is_ascii_digit()) =>
        {
            path
        }
        _ => anchor,
    }
}

// ---------------------------------------------------------------------------
// Fingerprints
// ---------------------------------------------------------------------------

/// FNV-1a 64-bit over `bytes`, rendered as 16 lowercase hex characters.
fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// Canonical metrics hash with the per-run provenance fields removed.
///
/// `findings[]` is sorted by `finding_id` before hashing: the report is
/// input-order independent (the grouped findings test shuffles the same
/// findings and requires byte identity), so a content fingerprint must not
/// change when only the array order does.
fn metrics_content_fingerprint(doc: &MetricsDocument) -> String {
    let mut value = serde_json::to_value(doc).expect("metrics document serializes to a value");
    if let Value::Object(root) = &mut value {
        if let Some(Value::Object(scan)) = root.get_mut("scan") {
            scan.remove("started_at");
            scan.remove("duration_ms");
            scan.remove("root_path");
        }
        if let Some(Value::Array(findings)) = root.get_mut("findings") {
            findings.sort_by(|left, right| {
                left.get("finding_id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .cmp(
                        right
                            .get("finding_id")
                            .and_then(Value::as_str)
                            .unwrap_or(""),
                    )
            });
        }
    }
    let canonical =
        serde_json::to_string(&value).expect("metrics value serializes to canonical JSON");
    fnv1a64_hex(canonical.as_bytes())
}

// ---------------------------------------------------------------------------
// Story blocks
// ---------------------------------------------------------------------------

/// Project the purpose block, applying the F4 honesty filter from
/// `sim/from-layout.py` (`forbidden_purpose`): a performance claim in README
/// prose becomes a labelled withheld stub instead of reaching the story.
fn purpose_projection(layout: &Value) -> Value {
    let block = layout.get("purpose").cloned().unwrap_or(Value::Null);
    let kind = block
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("structural");
    let source = block.get("source").cloned().unwrap_or(Value::Null);
    let text = block
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("No static purpose sentence.");
    let version = block
        .get("purpose_version")
        .and_then(Value::as_u64)
        .unwrap_or(1);
    if forbidden_purpose(text) || text.trim().is_empty() || text.chars().count() > PURPOSE_MAX_CHARS
    {
        json!({
            "kind": "withheld",
            "source": source,
            "text": "No static purpose sentence.",
            "version": 1,
        })
    } else {
        json!({
            "kind": kind,
            "source": source,
            "text": text,
            "version": version,
        })
    }
}

/// Mirror `sim/from-layout.py`'s F4 `forbidden_purpose` check.
///
/// The converter used Python's `re`; no regex dependency is added here, so the
/// same patterns are expressed as small scans. The purpose sentence is capped
/// at 201 characters, so this stays cheap.
fn forbidden_purpose(text: &str) -> bool {
    let lower = text.to_lowercase();
    if lower.contains("speedup")
        || lower.contains("call stack")
        || lower.contains("execution order")
        || lower.contains("% of traffic")
        || runs_n_times(&lower)
    {
        return true;
    }
    if contains_time_unit(&lower) || unnegated_runtime(&lower) {
        return true;
    }
    ["execution", "executed", "iterations"]
        .iter()
        .any(|claim| has_unnegated_word(&lower, claim))
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// `runs \d+ times` from the F4 forbidden-claims list.
fn runs_n_times(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut cursor = 0;
    while let Some(relative) = text[cursor..].find("runs ") {
        let start = cursor + relative + "runs ".len();
        let digits = bytes[start..]
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digits > 0 && text[start + digits..].starts_with(" times") {
            return true;
        }
        cursor = start;
    }
    false
}

/// `\b\d+(?:\.\d+)?\s?(?:ms|µs|μs|milliseconds?|seconds?)\b`.
fn contains_time_unit(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if !bytes[index].is_ascii_digit() || (index > 0 && is_word_byte(bytes[index - 1])) {
            index += 1;
            continue;
        }
        let mut cursor = index;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor + 1 < bytes.len() && bytes[cursor] == b'.' && bytes[cursor + 1].is_ascii_digit() {
            cursor += 1;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
        }
        if cursor < bytes.len() && bytes[cursor] == b' ' {
            cursor += 1;
        }
        let rest = &text[cursor..];
        for unit in [
            "milliseconds",
            "millisecond",
            "seconds",
            "second",
            "ms",
            "µs",
            "μs",
        ] {
            if let Some(tail) = rest.strip_prefix(unit)
                && tail
                    .as_bytes()
                    .first()
                    .is_none_or(|byte| !is_word_byte(*byte))
            {
                return true;
            }
        }
        index = cursor.max(index + 1);
    }
    false
}

/// `(?<!not a )runtime` — any `runtime` not immediately preceded by `not a `.
fn unnegated_runtime(text: &str) -> bool {
    let mut cursor = 0;
    while let Some(relative) = text[cursor..].find("runtime") {
        let start = cursor + relative;
        let negated = start >= 6 && text.get(start - 6..start) == Some("not a ");
        if !negated {
            return true;
        }
        cursor = start + "runtime".len();
    }
    false
}

/// A claim word outside the 40-character negation window that mirrors the
/// Python helper's `NEGATION` pattern.
fn has_unnegated_word(text: &str, word: &str) -> bool {
    let mut cursor = 0;
    while let Some(relative) = text[cursor..].find(word) {
        let start = cursor + relative;
        let end = start + word.len();
        let bounded_left = start == 0 || !is_word_byte(text.as_bytes()[start - 1]);
        let bounded_right = end == text.len() || !is_word_byte(text.as_bytes()[end]);
        if bounded_left && bounded_right {
            let window: String = text[..start]
                .chars()
                .rev()
                .take(40)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if !contains_negation(&window) {
                return true;
            }
        }
        cursor = end;
    }
    false
}

fn contains_negation(window: &str) -> bool {
    [
        "not", "no", "nothing", "never", "without", "none", "isn't", "aren't", "doesn't", "don't",
        "cannot", "can't",
    ]
    .iter()
    .any(|word| has_word(window, word))
}

fn has_word(text: &str, word: &str) -> bool {
    let mut cursor = 0;
    while let Some(relative) = text[cursor..].find(word) {
        let start = cursor + relative;
        let end = start + word.len();
        let left = start == 0 || !is_word_byte(text.as_bytes()[start - 1]);
        let right = end == text.len() || !is_word_byte(text.as_bytes()[end]);
        if left && right {
            return true;
        }
        cursor = end;
    }
    false
}

fn profile_projection(layout: &Value, doc: &MetricsDocument) -> (Vec<Value>, Value) {
    let block = layout.get("profile").cloned().unwrap_or(Value::Null);
    let functions: BTreeMap<&str, &FunctionStats> = doc
        .stats
        .functions
        .iter()
        .map(|function| (function.id.as_str(), function))
        .collect();
    let rows: Vec<Value> = array_at(&block, "rows")
        .iter()
        .map(|row| {
            let id = row.get("id").and_then(Value::as_str).unwrap_or("");
            let function = functions.get(id).copied();
            json!({
                "id": id,
                "name": field(row, "name"),
                "path": field(row, "path"),
                "line": field(row, "line"),
                "cost": field(row, "cost"),
                "fanIn": function.and_then(|f| f.call_fanin).map_or(Value::Null, Value::from),
                "fanOut": function.and_then(|f| f.call_fanout).map_or(Value::Null, Value::from),
                "loopDepth": function.map_or(Value::Null, |f| Value::from(f.max_loop_depth)),
                "tripSource": function
                    .and_then(|f| f.trip_count_source.clone())
                    .map_or(Value::Null, Value::String),
                "unknownTripLoops": field(row, "unknown_trip_loops"),
                "recursive": function
                    .and_then(|f| f.is_recursive)
                    .map_or(Value::Null, Value::from),
                "production": field(row, "production"),
                "rank": field(row, "rank"),
                "costText": field(row, "cost_text"),
                "pathClass": field(row, "path_class"),
                "knownTripValue": field(row, "known_trip_value"),
                "demotions": field(row, "demotions"),
            })
        })
        .collect();
    let meta = json!({
        "shown": field(&block, "shown"),
        "candidateTotal": field(&block, "candidate_total"),
        "productionRanked": field(&block, "production_ranked"),
        "totalRanked": field(&block, "total_ranked"),
        "demotedProduction": field(&block, "demoted_production"),
        "queueVersion": field(&block, "queue_version"),
    });
    (rows, meta)
}

fn scenes_projection(layout: &Value, doc: &MetricsDocument) -> (Vec<Value>, Value) {
    let block = layout.get("scenes").cloned().unwrap_or(Value::Null);
    let rows: Vec<Value> = array_at(&block, "rows")
        .iter()
        .map(|row| {
            let top_files: Vec<Value> = array_at(row, "top_files")
                .iter()
                .map(|file| {
                    json!({
                        "path": field(file, "path"),
                        "name": field(file, "name"),
                        "cost": field(file, "cost"),
                        "scale": field(file, "scale"),
                        "production": field(file, "production"),
                        "findings": field(file, "findings"),
                        "functionCount": field(file, "function_count"),
                        "entry": field(file, "entry"),
                        "hottest": field(file, "hottest"),
                    })
                })
                .collect();
            json!({
                "path": field(row, "path"),
                "depth": field(row, "depth"),
                "files": field(row, "files"),
                "productionFiles": field(row, "production_files"),
                "findings": field(row, "findings"),
                "cost": field(row, "cost"),
                "productionCost": field(row, "production_cost"),
                "scale": field(row, "scale"),
                "entry": field(row, "entry"),
                "hottest": field(row, "hottest"),
                "topFiles": top_files,
            })
        })
        .collect();

    let source_files: Vec<&crate::schema::FileEntry> = doc
        .files
        .iter()
        .filter(|file| file.function_count > 0)
        .collect();
    let production_files = source_files
        .iter()
        .filter(|file| file.path_class.as_deref() == Some("production"))
        .count();
    let non_production_files = source_files.len() - production_files;
    let omitted = block.get("omitted").cloned().unwrap_or(Value::Null);
    let meta = json!({
        "sceneCap": field(&block, "scene_cap"),
        "fileCap": field(&block, "file_cap"),
        "totalScenes": field(&block, "total_scenes"),
        "shownScenes": field(&block, "shown_scenes"),
        "totalFiles": field(&block, "total_files"),
        "shownFiles": field(&block, "shown_files"),
        "productionFiles": production_files,
        "nonProductionFiles": non_production_files,
        "omitted": {
            "scenes": field(&omitted, "scenes"),
            "files": field(&omitted, "files"),
            "cost": field(&omitted, "cost"),
            "productionCost": field(&omitted, "production_cost"),
            "findings": field(&omitted, "findings"),
        },
        "sceneVersion": field(&block, "scene_version"),
    });
    (rows, meta)
}

fn state_projection(state: &Value) -> Value {
    json!({
        "empty": field(state, "empty"),
        "noEntry": field(state, "no_entry"),
        "library": field(state, "library"),
        "allTest": field(state, "all_test"),
        "capped": field(state, "capped"),
        "partialParse": field(state, "partial_parse"),
        "singleFile": field(state, "single_file"),
    })
}

// ---------------------------------------------------------------------------
// I/O rows
// ---------------------------------------------------------------------------

/// Group `flow.io[]` by its drawn call-site file and sort each list.
fn io_rows_by_path(flow: &Value, drawn: &BTreeSet<String>) -> BTreeMap<String, Vec<Value>> {
    let mut grouped: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for row in array_at(flow, "io") {
        let Some(path) = row.get("file").and_then(Value::as_str) else {
            continue;
        };
        if !drawn.contains(path) {
            continue;
        }
        grouped.entry(path.to_string()).or_default().push(json!({
            "line": field(row, "line"),
            "callee": field(row, "callee"),
            "op": field(row, "op"),
            "target": field(row, "target"),
            "literal": field(row, "literal"),
        }));
    }
    for rows in grouped.values_mut() {
        rows.sort_by(io_row_order);
    }
    grouped
}

fn io_row_order(left: &Value, right: &Value) -> Ordering {
    left.get("line")
        .and_then(Value::as_u64)
        .cmp(&right.get("line").and_then(Value::as_u64))
        .then_with(|| text(left, "callee").cmp(&text(right, "callee")))
        .then_with(|| text(left, "op").cmp(&text(right, "op")))
        .then_with(|| text(left, "target").cmp(&text(right, "target")))
        .then_with(|| {
            text(left, "literal")
                .unwrap_or_default()
                .cmp(text(right, "literal").unwrap_or_default())
        })
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

/// Per-file I/O totals plus the capped row list (`None` when the file has none).
fn io_summary(rows: &[Value]) -> Option<Value> {
    if rows.is_empty() {
        return None;
    }
    Some(json!({
        "total": rows.len(),
        "read": count_where(rows, "op", "read"),
        "write": count_where(rows, "op", "write"),
        "file": count_where(rows, "target", "file"),
        "env": count_where(rows, "target", "env"),
        "net": count_where(rows, "target", "net"),
        "stdio": count_where(rows, "target", "stdio"),
        "literal": rows
            .iter()
            .filter(|row| !row.get("literal").is_none_or(Value::is_null))
            .count(),
        "unknownTarget": rows
            .iter()
            .filter(|row| {
                row.get("literal").is_none_or(Value::is_null)
                    && row.get("target").and_then(Value::as_str) != Some("stdio")
            })
            .count(),
        "rows": rows[..rows.len().min(IO_ROWS_PER_FILE)].to_vec(),
    }))
}

fn count_where(rows: &[Value], key: &str, expected: &str) -> usize {
    rows.iter()
        .filter(|row| row.get(key).and_then(Value::as_str) == Some(expected))
        .count()
}

fn io_trust(grouped: &BTreeMap<String, Vec<Value>>) -> Value {
    let rows: Vec<&Value> = grouped.values().flatten().collect();
    let mut literals: BTreeSet<(String, String)> = BTreeSet::new();
    for row in &rows {
        if let Some(literal) = row.get("literal").and_then(Value::as_str) {
            let target = row
                .get("target")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            literals.insert((target, literal.to_string()));
        }
    }
    let literals: Vec<Value> = literals
        .into_iter()
        .take(IO_TRUST_LITERALS)
        .map(|(target, literal)| json!({"target": target, "literal": literal}))
        .collect();
    json!({
        "rows": rows.len(),
        "files": grouped.values().filter(|rows| !rows.is_empty()).count(),
        "read": rows
            .iter()
            .filter(|row| text(row, "op") == Some("read"))
            .count(),
        "write": rows
            .iter()
            .filter(|row| text(row, "op") == Some("write"))
            .count(),
        "stdio": rows
            .iter()
            .filter(|row| text(row, "target") == Some("stdio"))
            .count(),
        "literal": rows
            .iter()
            .filter(|row| !row.get("literal").is_none_or(Value::is_null))
            .count(),
        "unknownTarget": rows
            .iter()
            .filter(|row| {
                row.get("literal").is_none_or(Value::is_null)
                    && text(row, "target") != Some("stdio")
            })
            .count(),
        "literals": literals,
    })
}

// ---------------------------------------------------------------------------
// Calls
// ---------------------------------------------------------------------------

/// Group resolved calls by their drawn `(from, to)` file pair.
fn calls_by_pair(flow: &Value) -> BTreeMap<(String, String), Vec<&Value>> {
    let mut grouped: BTreeMap<(String, String), Vec<&Value>> = BTreeMap::new();
    for call in array_at(flow, "calls") {
        if call.get("unresolved").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let Some(to) = call.get("to").and_then(Value::as_str) else {
            continue;
        };
        let from = call.get("from").and_then(Value::as_str).unwrap_or("");
        grouped
            .entry((from.to_string(), to.to_string()))
            .or_default()
            .push(call);
    }
    for rows in grouped.values_mut() {
        rows.sort_by(|left, right| {
            left.get("line")
                .and_then(Value::as_u64)
                .cmp(&right.get("line").and_then(Value::as_u64))
                .then_with(|| text(left, "callee").cmp(&text(right, "callee")))
                .then_with(|| text(left, "caller").cmp(&text(right, "caller")))
        });
    }
    grouped
}

fn first_typed_arg(call: &Value) -> Value {
    let Some(args) = call.get("args").and_then(Value::as_array) else {
        return Value::Null;
    };
    for arg in args {
        let has_type = arg.get("type").is_some_and(|value| !value.is_null());
        let has_named = arg.get("named").is_some_and(|value| !value.is_null());
        if !has_type && !has_named {
            continue;
        }
        return json!({
            "expr": field(arg, "expr"),
            "type": field(arg, "type"),
            "named": field(arg, "named"),
            "dir": field(arg, "dir"),
            "literal": field(arg, "literal"),
        });
    }
    Value::Null
}

fn edge_row(edge: &Value, order: usize, calls: &BTreeMap<(String, String), Vec<&Value>>) -> Value {
    let from = edge.get("from").and_then(Value::as_str).unwrap_or("");
    let to = edge.get("to").and_then(Value::as_str).unwrap_or("");
    let label = field(edge, "label");
    let pair = (from.to_string(), to.to_string());
    let top_calls: Vec<Value> = calls
        .get(&pair)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .take(2)
        .map(|call| {
            json!({
                "callee": field(call, "callee"),
                "caller": field(call, "caller"),
                "line": field(call, "line"),
                "arg": first_typed_arg(call),
            })
        })
        .collect();
    let back = edge.get("back").and_then(Value::as_bool) == Some(true);
    json!({
        "from": from,
        "to": to,
        "d": field(edge, "d"),
        "count": field(edge, "count"),
        "back": back,
        "onPath": field(edge, "on_path"),
        "label": label,
        "payload": field(edge, "payload"),
        "labelClass": if label.as_str() == Some("unresolved") { "unresolved" } else { "resolved" },
        "topCalls": top_calls,
        "className": if back { "back" } else { "forward" },
        "order": order,
    })
}

/// Core unresolved-call rows grouped by drawn call-site file.
fn unresolved_by_path(flow: &Value, drawn: &BTreeSet<String>) -> BTreeMap<String, Value> {
    let mut grouped: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for row in array_at(flow, "calls") {
        if row.get("unresolved").and_then(Value::as_bool) != Some(true) {
            continue;
        }
        let Some(path) = row.get("from").and_then(Value::as_str) else {
            continue;
        };
        if !drawn.contains(path) {
            continue;
        }
        grouped.entry(path.to_string()).or_default().push(json!({
            "line": field(row, "line"),
            "callee": field(row, "callee"),
            "caller": field(row, "caller"),
        }));
    }
    grouped
        .into_iter()
        .map(|(path, mut rows)| {
            rows.sort_by(|left, right| {
                left.get("line")
                    .and_then(Value::as_u64)
                    .cmp(&right.get("line").and_then(Value::as_u64))
                    .then_with(|| text(left, "callee").cmp(&text(right, "callee")))
                    .then_with(|| text(left, "caller").cmp(&text(right, "caller")))
            });
            let count = rows.len();
            rows.truncate(UNRESOLVED_ROWS);
            let value = json!({"count": count, "rows": rows});
            (path, value)
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Node / function rows
// ---------------------------------------------------------------------------

fn node_row(node: &Value, io: Option<Value>, unresolved: Option<&Value>) -> Value {
    let path = node.get("path").and_then(Value::as_str).unwrap_or("");
    let cost_text = text(node, "cost_text").unwrap_or("");
    let functions: Vec<Value> = array_at(node, "functions")
        .iter()
        .map(function_row)
        .collect();
    let mut row = json!({
        "id": path,
        "name": field(node, "name"),
        "x": field(node, "x"),
        "y": field(node, "y"),
        "w": field(node, "w"),
        "h": field(node, "h"),
        "column": field(node, "column"),
        "entry": field(node, "entry"),
        "hottest": field(node, "hottest"),
        "unreached": field(node, "unreached"),
        "nonproduction": node.get("production").and_then(Value::as_bool) != Some(true),
        "pathStep": field(node, "path_step"),
        "partialParse": field(node, "partial_parse"),
        "fill": field(node, "fill"),
        "stroke": field(node, "stroke"),
        "strokeWidth": field(node, "stroke_width"),
        "dash": field(node, "dash"),
        "badge": field(node, "badge"),
        "cost": field(node, "cost"),
        "costText": field(node, "cost_text"),
        "ratioText": field(node, "ratio_text"),
        "topText": field(node, "top_text"),
        "title": format!("{path} · {cost_text}"),
        "findings": field(node, "findings"),
        "scene": field(node, "scene"),
        "functions": functions,
    });
    if let Some(io) = io
        && let Some(object) = row.as_object_mut()
    {
        object.insert("io".to_string(), io);
    }
    if let Some(unresolved) = unresolved
        && let Some(object) = row.as_object_mut()
    {
        object.insert("unresolvedCalls".to_string(), unresolved.clone());
    }
    row
}

fn function_row(function: &Value) -> Value {
    json!({
        "id": field(function, "id"),
        "name": field(function, "name"),
        "line": field(function, "line"),
        "cost": field(function, "cost"),
        "maxLoopDepth": field(function, "loop_depth"),
        "tripSource": field(function, "trip_source"),
        "unknownTripLoops": field(function, "unknown_trip_loops"),
        "fanIn": field(function, "fan_in"),
        "fanOut": field(function, "fan_out"),
        "recursive": field(function, "recursive"),
    })
}

/// Drawn functions deduped by id, sorted `(cost desc, line, name)`.
fn hottest_function(nodes: &[Value]) -> Value {
    let mut rows: BTreeMap<String, Value> = BTreeMap::new();
    for node in nodes {
        let path = node.get("path").cloned().unwrap_or(Value::Null);
        let production = node
            .get("production")
            .cloned()
            .unwrap_or(Value::Bool(false));
        for function in array_at(node, "functions") {
            let id = function
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            rows.insert(
                id.clone(),
                json!({
                    "id": id,
                    "name": field(function, "name"),
                    "path": path,
                    "line": field(function, "line"),
                    "cost": field(function, "cost"),
                    "fanIn": field(function, "fan_in"),
                    "fanOut": field(function, "fan_out"),
                    "loopDepth": field(function, "loop_depth"),
                    "tripSource": field(function, "trip_source"),
                    "unknownTripLoops": field(function, "unknown_trip_loops"),
                    "recursive": field(function, "recursive"),
                    "production": production,
                }),
            );
        }
    }
    let mut rows: Vec<Value> = rows.into_values().collect();
    rows.sort_by(|left, right| {
        let cost = right
            .get("cost")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .partial_cmp(&left.get("cost").and_then(Value::as_f64).unwrap_or(0.0))
            .unwrap_or(Ordering::Equal);
        cost.then_with(|| {
            left.get("line")
                .and_then(Value::as_u64)
                .cmp(&right.get("line").and_then(Value::as_u64))
        })
        .then_with(|| text(left, "name").cmp(&text(right, "name")))
    });
    rows.into_iter().next().unwrap_or(Value::Null)
}

/// Drawn functions whose loop trip count is unknown (deduped by id).
fn unknown_trip_functions(nodes: &[Value]) -> usize {
    let mut seen: BTreeMap<String, u64> = BTreeMap::new();
    for node in nodes {
        for function in array_at(node, "functions") {
            let id = function
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let unknown = function
                .get("unknown_trip_loops")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            seen.insert(id, unknown);
        }
    }
    seen.values().filter(|unknown| **unknown > 0).count()
}

// ---------------------------------------------------------------------------
// Findings
// ---------------------------------------------------------------------------

fn findings_for(doc: &MetricsDocument, drawn: &BTreeSet<String>) -> Vec<Value> {
    let mut rows: Vec<(String, u32, String, Value)> = doc
        .findings
        .iter()
        .filter(|finding| drawn.contains(&finding.path))
        .map(|finding| {
            let value = json!({
                "path": finding.path,
                "line": finding.span.start_line,
                "ruleId": finding.rule_id,
                "severity": finding.severity,
                "symbol": finding.symbol.clone().unwrap_or_default(),
                "title": finding.title,
            });
            (
                finding.path.clone(),
                finding.span.start_line,
                finding.rule_id.clone(),
                value,
            )
        })
        .collect();
    rows.sort_by(|left, right| {
        left.0
            .cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    rows.into_iter().map(|(_, _, _, value)| value).collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use serde_json::Value;

    use super::*;
    use crate::analyzer::{Confidence, Severity};
    use crate::ir::{ResolverTier, Span};
    use crate::reporter::atlas::test_support::application_fixture;
    use crate::schema::FindingEntry;

    fn finding(path: &str, line: u32, message: &str) -> FindingEntry {
        FindingEntry {
            finding_id: format!("PERF-TEST-001:{path}:{line}:1"),
            rule_id: "PERF-TEST-001".to_string(),
            title: "Test finding".to_string(),
            path: path.to_string(),
            symbol: Some("work".to_string()),
            span: Span::point(line, 1),
            confidence: Confidence::High,
            severity: Severity::Warn,
            resolver_tier: ResolverTier::Syntactic,
            macro_suspect: false,
            error_node_overlap: false,
            threshold: None,
            message: message.to_string(),
            evidence: BTreeMap::new(),
        }
    }

    fn fixture() -> MetricsDocument {
        let mut doc = application_fixture();
        doc.findings.push(finding("src/main.c", 1, "drawn finding"));
        // A finding outside the drawn slice must count toward findingsTotal but
        // not toward the per-node findingsIncluded total.
        doc.findings
            .push(finding("src/not-drawn.c", 2, "undrawn finding"));
        doc
    }

    fn parse(bundle: &StoryBundle) -> Value {
        serde_json::from_str(&bundle.payload_json).expect("story payload parses")
    }

    #[test]
    fn payload_has_the_sim_key_set() {
        let bundle = build(&fixture(), None);
        let payload = parse(&bundle);
        let object = payload.as_object().expect("payload object");
        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "case",
                "columns",
                "edges",
                "findings",
                "hottestFunction",
                "hulls",
                "meta",
                "nodes",
                "profile",
                "profileMeta",
                "purpose",
                "sceneMeta",
                "scenes",
                "source",
                "stage",
                "state",
                "synthetic",
                "trace",
                "trust",
            ]
        );
        assert_eq!(payload["case"], "scan");
        assert_eq!(payload["synthetic"], false);
        assert!(payload["stage"]["w"].is_number());
        assert!(payload["source"]["layoutFingerprint"].is_string());
        assert!(payload["source"]["metricsContentFingerprint"].is_string());
    }

    #[test]
    fn payload_is_deterministic() {
        let doc = fixture();
        let first = build(&doc, None);
        let second = build(&doc, None);
        assert_eq!(first.layout_json, second.layout_json);
        assert_eq!(first.payload_json, second.payload_json);
        assert_eq!(first.layout_json, build(&doc, None).layout_json);
    }

    #[test]
    fn payload_matches_layout_totals() {
        let doc = fixture();
        let bundle = build(&doc, None);
        let payload = parse(&bundle);
        let nodes = payload["nodes"].as_array().expect("nodes array");
        let edges = payload["edges"].as_array().expect("edges array");

        assert_eq!(
            payload["meta"]["findingsTotal"].as_u64().unwrap() as usize,
            doc.findings.len()
        );
        let included: u64 = nodes
            .iter()
            .map(|node| {
                node["findings"]["hot"].as_u64().unwrap()
                    + node["findings"]["warn"].as_u64().unwrap()
                    + node["findings"]["info"].as_u64().unwrap()
            })
            .sum();
        assert_eq!(
            payload["meta"]["findingsIncluded"].as_u64().unwrap(),
            included
        );
        assert_eq!(
            payload["findings"].as_array().unwrap().len() as u64,
            included
        );

        let steps: Vec<u64> = payload["trace"]
            .as_array()
            .expect("trace array")
            .iter()
            .map(|step| step["step"].as_u64().expect("trace step"))
            .collect();
        assert!(!steps.is_empty());
        assert_eq!(steps[0], 1);
        assert!(steps.windows(2).all(|pair| pair[0] + 1 == pair[1]));

        for node in nodes {
            assert!(node["title"].as_str().unwrap().contains(" · "));
            // The Python converter omits these keys when the file has no rows;
            // when present they are always objects (never null).
            for key in ["io", "unresolvedCalls"] {
                assert!(
                    node.get(key).is_none_or(Value::is_object),
                    "node {key} must be an object when present: {node}"
                );
            }
        }
        for edge in edges {
            assert!(edge["topCalls"].is_array());
            assert!(edge["labelClass"].is_string());
            assert!(edge["className"].is_string());
        }
    }

    #[test]
    fn fingerprints_are_stable() {
        let doc = fixture();
        let first = build(&doc, None);
        let second = build(&doc, None);
        let first_payload = parse(&first);
        let second_payload = parse(&second);
        for key in ["layoutFingerprint", "metricsContentFingerprint"] {
            let value = first_payload["source"][key]
                .as_str()
                .expect("fingerprint string");
            assert_eq!(value.len(), 16);
            assert!(value.bytes().all(|byte| byte.is_ascii_hexdigit()));
            assert_eq!(value, second_payload["source"][key].as_str().unwrap());
        }
        assert_eq!(
            first_payload["source"]["layoutFingerprint"],
            layout_fingerprint(&first.layout_json),
            "layout fingerprint must cover the emitted trailing newline"
        );
        // A changed finding message is provenance, not geometry: the metrics
        // fingerprint moves while the layout fingerprint stays put.
        let mut changed = doc.clone();
        changed.findings[0].message = "different rationale".to_string();
        let changed_payload = parse(&build(&changed, None));
        assert_eq!(
            first_payload["source"]["layoutFingerprint"],
            changed_payload["source"]["layoutFingerprint"]
        );
        assert_ne!(
            first_payload["source"]["metricsContentFingerprint"],
            changed_payload["source"]["metricsContentFingerprint"]
        );
    }

    #[test]
    fn atlas_layout_json_delegates_to_the_bundle() {
        let doc = fixture();
        let root = Path::new(&doc.scan.root_path);
        assert_eq!(
            crate::reporter::atlas_layout_json(&doc),
            build(&doc, Some(root)).layout_json
        );
    }

    #[test]
    fn synthetic_fixture_carries_whole_repo_blocks() {
        let doc = fixture();
        let payload = parse(&build(&doc, None));
        assert!(payload["profile"].is_array());
        assert!(payload["scenes"].is_array());
        assert!(payload["profileMeta"]["queueVersion"].is_number());
        assert!(payload["sceneMeta"]["sceneVersion"].is_number());
    }
}
