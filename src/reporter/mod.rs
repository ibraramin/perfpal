//! `reporter` — human-readable rendering (modules.md §3.6).
//!
//! Consumes: the final document. Produces: report.md bytes.
//!
//! Responsibilities: render the fixed section template (output-contract §5);
//! cite findings by exact finding_id; escape all embedded paths/titles/
//! messages (Markdown-injection rule); apply optional-field n/a-or-omit rules;
//! Appendix A descriptions from the built-in catalog (frozen v1).
//!
//! Determinism: ordering depends ONLY on document content (files[] are
//! re-sorted here for the §3 table; findings keep the analyzer's total order,
//! partitioned hot→warn→info WITHOUT re-sorting within a group). Double-render
//! byte-equality modulo provenance fields (`started_at`, `duration_ms`) is a
//! standing test obligation ([`tests/reporter_render.rs`]).
//!
//! Markdown-injection escaping: every embedded path/title/message/id goes
//! through [`escape_md`] with the PINNED metacharacter set
//! [`MD_ESCAPE_CHARS`] (backslash, backtick, asterisk, underscore, braces,
//! brackets, parentheses, hash, plus, hyphen, period, bang, pipe, angle
//! brackets, ampersand). Because `|` is in the set, table cells are covered
//! automatically. Inside code spans the backslash renders literally — that is
//! the accepted cost of never letting a hostile filename break out.
//!
//! Non-goals: reading source, IR, or scanner state; inventing aggregations
//! beyond the output-contract §5 template.

use std::collections::BTreeSet;

use crate::analyzer::{Confidence, Severity};
use crate::ir::ResolverTier;
use crate::schema::{DistributionEntry, FindingEntry, MetricsDocument, format_fixed_point};

// ---------------------------------------------------------------------------
// Pinned constants
// ---------------------------------------------------------------------------

/// Pinned Markdown metacharacter escape set (output-contract §2 paths rule).
/// Every char here is backslash-escaped wherever document-derived text is
/// embedded. `|` inclusion makes table cells safe without a second pass.
const MD_ESCAPE_CHARS: &[char] = &[
    '\\', '`', '*', '_', '{', '}', '[', ']', '(', ')', '#', '+', '-', '.', '!', '|', '<', '>', '&',
];

/// Heuristic catalog (read-code-first transcription of
/// docs/heuristics/catalog-v1.md §3 + §4 titles and one-line "Detects"
/// summaries, EXTENDED for the Rust `PERF-RS-*` families added by plan
/// `rust-backend` R3 from their implemented rule functions and rustdocs).
/// Supplies Appendix A descriptions ONLY — never invent wording here; the
/// catalog follows the rules/doc, not the reverse.
///
/// The C (`PERF-C-*`) entries are FROZEN v1 wording (unchanged). The Rust
/// (`PERF-RS-*`) entries cover BOTH languages' rule ids and transcribe each
/// rule's ACTUAL implemented detection, carrying an explicit "REFRAMED for
/// Rust" clause wherever the rule is an honest re-mapping of the C intent
/// (no `realloc`/char-stdio, `unsafe`-only unbounded ops, `size_of` as an
/// element count, default `repr(Rust)` field reordering). The Python
/// (`PERF-PY-*`) entries transcribe the durable disposition table
/// (docs/architecture/python-backend.md §9): 13 reframed mirrors + 9 native,
/// with the CPython mechanism and the missing-signal limitations stated. A
/// completeness lock in this module's tests asserts the catalog covers
/// exactly the 67 rule ids the analyzer registry can emit.
const HEURISTIC_CATALOG_V1: &[(&str, &str, &str)] = &[
    (
        "PERF-C-ALLOC-001",
        "Heap allocation inside loop",
        "Detects malloc/calloc/realloc/aligned_alloc/strdup calls nested anywhere inside a loop body.",
    ),
    (
        "PERF-C-ALLOC-002",
        "`free()` inside loop",
        "Detects free() directly inside a loop body (paired with ALLOC-001 it indicates allocator churn).",
    ),
    (
        "PERF-C-ALLOC-003",
        "`realloc()` linear growth in loop",
        "Detects realloc(p, n+k) additive-growth shapes nested in loops — quadratic memmove behavior.",
    ),
    (
        "PERF-C-STR-001",
        "String accumulation in loop (`strcat` family)",
        "Detects strcat/strncat with a destination reused across loop iterations (O(n²) rescan).",
    ),
    (
        "PERF-C-STR-002",
        "`strlen()` inside loop condition/body",
        "Detects strlen(s) in a loop condition or body where s is not reassigned in the loop.",
    ),
    (
        "PERF-C-STR-003",
        "Unbounded copy family (`strcpy`/`sprintf`/`gets`)",
        "Detects unbounded copy calls (strcpy, strcat, sprintf, gets, scanf %s) — unbounded-latency hotspots.",
    ),
    (
        "PERF-C-LOOP-001",
        "Deep loop nesting (≥3)",
        "Detects loop nesting depth ≥ 3 — O(n^k) candidate zones and primary input to hotspot maps.",
    ),
    (
        "PERF-C-LOOP-002",
        "Expensive setup / I/O call inside loop",
        "Detects printf-family, file-I/O, getenv, system, and regex-compilation calls nested in loops. The message splits hoistable setup (hoisting or batching is the candidate) from intrinsic streaming I/O (read/write — batching into block transfers is the candidate); evidence carries io_kind. Calls in do-while(0) wrapper bodies stay silent.",
    ),
    (
        "PERF-C-LOOP-003",
        "Char-wise stdio loop (`fgetc`/`fputc` family)",
        "Detects fgetc/getc/getchar/fputc/putc as the primary statement of a loop body.",
    ),
    (
        "PERF-C-LOOP-004",
        "Integer div/mod with variable divisor in innermost loop",
        "Detects / or % with a non-literal divisor inside the innermost loop body (literal divisors skipped by design).",
    ),
    (
        "PERF-C-CALL-001",
        "Direct recursion",
        "Detects a function body containing a call to its own name (presence-only; depth analysis is post-MVP).",
    ),
    (
        "PERF-C-CALL-002",
        "High cyclomatic complexity",
        "Detects branch-count complexity above threshold (default ≥ 15), reported per function.",
    ),
    (
        "PERF-C-ARCH-001",
        "Include-graph fan-out / cycles",
        "Detects include fan-out/fan-in, deep include chains, and cyclic includes from #include directives.",
    ),
    (
        "PERF-C-ARCH-002",
        "Hot-component aggregation",
        "Ranks components by aggregated metrics (complexity sum, alloc-site density, max loop depth, finding density).",
    ),
    (
        "PERF-C-ARCH-003",
        "Cross-file call coupling (name-based)",
        "Detects cross-file call edges via name matching — LOW confidence until a semantic resolver tier exists.",
    ),
    (
        "PERF-C-TYPE-001",
        "Large struct passed by value",
        "Detects struct-typed by-value parameters whose TU-defined struct reaches the approximate-size floor (LOW confidence — no real type layout at MVP).",
    ),
    (
        "PERF-C-TYPE-002",
        "Struct member ordering (padding candidate)",
        "Detects struct definitions whose naive aligned layout wastes padding versus the packed member sum beyond the threshold.",
    ),
    (
        "PERF-C-MEM-001",
        "sizeof-misuse shapes in allocation args",
        "Detects malloc(x * sizeof(ptr)) and sizeof misplacement shapes in allocation arguments — wrong-size allocations.",
    ),
    (
        "PERF-C-MEM-002",
        "Block copy/set family in loop (`memcpy`/`memmove`/`memset`)",
        "Detects memcpy/memmove/memset calls with non-literal size arguments inside loop bodies; warn when innermost.",
    ),
    (
        "PERF-C-LOOP-005",
        "Global variable written inside loop",
        "Detects assignment to a file-scope/global identifier inside a loop body (name-based scope classification, MEDIUM confidence).",
    ),
    (
        "PERF-C-LOOP-006",
        "Indirect call via function pointer in innermost loop",
        "Detects calls through function-pointer variables inside innermost loop bodies (LOW confidence without type info).",
    ),
    (
        "PERF-C-LOOP-007",
        "Linear membership scan nested in loop",
        "Detects comparison-against-array-element-indexed-by-outer-counter shapes — hand-rolled O(n²) lookup.",
    ),
    (
        "PERF-C-LOOP-008",
        "Pointer-chasing chain in innermost loop",
        "Detects member-access chains of depth ≥ 3 inside innermost loop bodies — pointer-chasing cache-miss candidates.",
    ),
    // -----------------------------------------------------------------------
    // Python (`PERF-PY-*`) entries (plan `python-backend` W3), ascending
    // rule_id. The durable disposition table is docs/architecture/
    // python-backend.md §9 (13 reframed mirrors + 9 native).
    (
        "PERF-PY-ALLOC-001",
        "Loop-invariant construction inside loop",
        "Detects a container/object constructor (list/dict/set/tuple/str/bytes/open/DataFrame/Series/array/ndarray) called inside a loop with only literal (or no) arguments — a loop-invariant value rebuilt each pass; f-string-in-loop is NOT a finding (98–99.6% of raw C hits were f-strings).",
    ),
    (
        "PERF-PY-ALLOC-003",
        "Superlinear list accumulation in loop",
        "Detects non-in-place `acc = acc + [...]` concatenation of a list-bound name reassigned inside the loop, and append/extend onto a list-bound receiver NOT rebuilt inside any enclosing loop (the accumulator carried across iterations) — CPython rebuilds and copies the list each pass (O(n²)). Silent when the receiver is reset inside the loop (the benign CTL-APPEND-LOCAL control). Flagship.",
    ),
    (
        "PERF-PY-ALLOC-004",
        "Eager iterable materialisation inside loop",
        "Detects `list()`/`dict()`/`set()`/`tuple()` with a non-literal iterable argument inside a loop — forces an eager O(n) allocation and copy, defeating lazy iteration.",
    ),
    (
        "PERF-PY-ARCH-001",
        "Import cycle",
        "Detects cycles among RESOLVED internal Python import edges (via break_cycles); unresolved internal imports stay data-only with no finding. Info, conservative — thresholds are not decision-grade on Python.",
    ),
    (
        "PERF-PY-ARCH-002",
        "Hot component",
        "Ranks directories by aggregated cyclomatic complexity and reports only the hottest component above a conservative floor. Info — thresholds are not measured on Python in W0 (measured in W4).",
    ),
    (
        "PERF-PY-ARCH-003",
        "Cross-file call coupling",
        "Detects name-based cross-file call edges (a callee name with exactly one other-file definition); LOW confidence because Python duck typing makes name coupling noisy.",
    ),
    (
        "PERF-PY-ASYNC-001",
        "Blocking call inside `async def`",
        "Detects `time.sleep`, blocking HTTP clients (requests/httpx/urllib), synchronous DB driver methods and `open` inside an `async def` — the blocking call stalls the single event-loop thread. The `async def` gate is required (166 production blocking calls in sync functions vs 0 real hits).",
    ),
    (
        "PERF-PY-ASYNC-002",
        "`await` inside a loop",
        "Detects an `await` whose event metadata reports `in_loop_depth ≥ 1` (additive AwaitEvent stream) — sequential awaits serialise latency that `asyncio.gather` could overlap.",
    ),
    (
        "PERF-PY-CALL-001",
        "Direct recursion",
        "Detects a function body containing a direct call to its own name; framed as CPython call cost and `RecursionError` depth, not C stack cost.",
    ),
    (
        "PERF-PY-CALL-002",
        "High cyclomatic complexity",
        "Detects branch-count complexity above threshold (default ≥ 15), reported per function.",
    ),
    (
        "PERF-PY-COMPREHENSION-001",
        "Manual loop collapsible to a comprehension",
        "Detects a loop whose body is a single `list.append` statement (single-statement gate) appending to a list-bound receiver — a list comprehension compiles to a specialised `LIST_APPEND` with inline iteration.",
    ),
    (
        "PERF-PY-EXECUTOR-001",
        "Executor/event-loop creation per iteration",
        "Detects `ThreadPoolExecutor(...)`, `ProcessPoolExecutor(...)` and `asyncio.new_event_loop()` constructed inside a loop — worker/selector setup is re-paid every iteration and never amortises.",
    ),
    (
        "PERF-PY-LOOP-001",
        "Deep loop nesting",
        "Detects loop nesting depth exactly 3 (one finding per maximal nest entry) — an O(n^k) candidate zone under interpreted iteration.",
    ),
    (
        "PERF-PY-LOOP-002",
        "Expensive I/O call inside loop",
        "Detects file/network/subprocess/socket I/O inside a loop — direct I/O names, network-rooted methods, and filehandle read/write methods. `print`/`input` are deliberately excluded (72–83% of the C rule's raw hits were logging). The message splits hoistable handle setup (open/urlopen/Popen/...) from element-wise streaming transfer (batching is the candidate); evidence carries io_kind.",
    ),
    (
        "PERF-PY-LOOP-003",
        "`range(len(seq))` iteration",
        "Detects `range(len(seq))` inside a loop (the `len()` gate keeps it false-positive-free) — prefer direct iteration or `enumerate`.",
    ),
    (
        "PERF-PY-LOOP-007",
        "Linear index scan of a list inside loop",
        "Detects (a) membership `x in list` / `x not in list` where the container is a list-bound name (additive `ComparisonEvent.operator`), and (b) a list-bound name subscripted by an enclosing loop's induction variable — including a lone `seq[i]` and deeper `seq[i].field` chains. The kind gate excludes set/dict (kills the CTL-IN-SET false-positive class). Flagship.",
    ),
    (
        "PERF-PY-LOOP-009",
        "`try`/`except` inside a loop",
        "Detects a `try_statement` whose event metadata reports `in_loop_depth ≥ 1` (additive TryEvent stream) — CPython sets up per-iteration exception handling. Honest that the zero-cost-`try` claim is version-dependent (CPython ≥ 3.11). Info.",
    ),
    (
        "PERF-PY-MEM-002",
        "Container copy inside loop",
        "Detects `copy`/`deepcopy` (module function or `.copy()` method) inside a loop — a whole new container is materialised per iteration (`deepcopy` also walks the graph with a memo dict).",
    ),
    (
        "PERF-PY-N1-001",
        "DB/HTTP call inside loop (N+1)",
        "Detects a DB/HTTP round-trip per iteration (receiver-root and call-name family gated) — the N+1 pattern; batch into `IN (...)`, a JOIN, or a bulk request. Flagship.",
    ),
    (
        "PERF-PY-PANDAS-001",
        "Row-wise pandas iteration",
        "Detects `iterrows`/`itertuples` or `apply(axis=1)` (receiver-root gated) — pandas materialises a Python object per row, dwarfing the vectorised work. Info; frequency caveat (4 hits on the W0 corpora).",
    ),
    (
        "PERF-PY-PANDAS-002",
        "`concat`/`append` in loop",
        "Detects `pd.concat`/`DataFrame.append` inside a loop — the whole frame is copied per iteration (O(n²)). `.append` is syntactically indistinguishable from `list.append`, so the receiver must be a proven DataFrame. Info; 0 hits on the W0 corpora.",
    ),
    (
        "PERF-PY-STR-001",
        "String accumulation in loop",
        "Detects string accumulation across iterations on a `str`-bound identifier inside a loop: both the explicit `s = s + piece` form and the in-place `s += piece` form (additive augmented-assignment BinaryOpEvent). Gated on the name being reassigned within the loop (accumulation, not a loop-invariant read) — CPython strings are immutable, so each pass allocates and copies (O(n²)); `str.join` is the fix.",
    ),
    // -----------------------------------------------------------------------
    // Rust (`PERF-RS-*`) entries (plan `rust-backend` R3 workstream D1),
    // ascending rule_id. Titles match each rule's emitted finding title;
    // descriptions transcribe the implemented detection, with explicit
    // "REFRAMED for Rust" clauses for the honest re-mappings.
    // -----------------------------------------------------------------------
    (
        "PERF-RS-ALLOC-001",
        "Heap allocation inside loop (Rust)",
        "Detects Rust allocation sites — qualified constructors (`Box::new`/`Rc::new`/`Arc::new`/`Vec::new`/`String::new`/collection `new`/`with_capacity`), methods (`to_vec`/`to_owned`/`to_string`/`collect`/`with_capacity`), and the `vec!`/`format!` macros — at `in_loop_depth ≥ 1` AND `is_innermost`; surviving sites are aggregated into ONE finding per innermost enclosing loop (`alloc_count` plus sorted-unique short callee tokens capped at 8 with a `+N more` suffix). Literal-receiver `\"…\".to_string()`/`\"…\".to_owned()` (including raw/byte-string prefixes) are suppressed.",
    ),
    (
        "PERF-RS-ALLOC-002",
        "Deallocation inside loop (Rust)",
        "Detects a direct `drop` call inside a loop body (allocator churn pairing; escalated to hot when the same function also allocates in a loop).",
    ),
    (
        "PERF-RS-ALLOC-003",
        "Growable-buffer additive growth in loop (Rust)",
        "REFRAMED for Rust (no `realloc`): detects additive `reserve`/`reserve_exact` capacity growth inside a loop — the `realloc(p, n+k)` analogue; multiplicative capacity growth is the fix, not the finding.",
    ),
    (
        "PERF-RS-ARCH-001",
        "Module-graph fan-out / cycles (Rust)",
        "Detects module fan-out/fan-in, deep module chains, and cyclic module edges from resolved Rust module edges; unresolved internal `mod`/`use` edges are retained as graph data but intentionally emit no finding (their resolver cannot see crate-root or inline-module items).",
    ),
    (
        "PERF-RS-ARCH-002",
        "Hot-component aggregation (Rust)",
        "Ranks Rust component directories by aggregated metrics (complexity sum, alloc-site density, max loop depth) — the hottest component only.",
    ),
    (
        "PERF-RS-ARCH-003",
        "Cross-module call coupling (Rust)",
        "Detects cross-file/module call edges via name matching — LOW confidence, upgraded to MEDIUM when a semantic resolver fact resolves the definition.",
    ),
    (
        "PERF-RS-CALL-001",
        "Direct recursion (Rust)",
        "Detects a function body containing a direct call to its own name (presence-only; recursion depth is post-MVP).",
    ),
    (
        "PERF-RS-CALL-002",
        "High cyclomatic complexity (Rust)",
        "Detects branch-count complexity above threshold (default ≥ 15), reported per function.",
    ),
    (
        "PERF-RS-LOOP-001",
        "Deeply nested loops (Rust)",
        "Detects loop nesting depth ≥ 3 — O(n^k) candidate zones and primary input to hotspot maps.",
    ),
    (
        "PERF-RS-LOOP-002",
        "Expensive setup or I/O call inside loop (Rust)",
        "Detects Rust I/O/setup work nested in loops — `println!`/`print!`-family macros, reader/writer/socket methods (`read_to_string`/`write_all`/`flush`/`recv`/`send`), and `File`/`fs`/`Command`/`TcpStream`/`env` setup calls; lowered to warn when conditionally guarded. The message splits hoistable setup from intrinsic streaming transfer (recv/send/read_line — batching is the candidate); evidence carries io_kind.",
    ),
    (
        "PERF-RS-LOOP-003",
        "Character-wise processing in loop (Rust)",
        "REFRAMED for Rust (no char stdio): detects char/byte-at-a-time iterator or reader methods (`chars`/`bytes`/`char_indices`/`byte_indices`/`read_byte`) as the primary statement of a loop body.",
    ),
    (
        "PERF-RS-LOOP-004",
        "Variable-divisor division in innermost loop (Rust)",
        "Detects / or % with a non-literal divisor inside the innermost loop body (literal divisors skipped by design).",
    ),
    (
        "PERF-RS-LOOP-005",
        "Global variable written inside loop (Rust)",
        "Detects assignment to a Rust `static`/`const`-backed identifier inside a loop body (name-based scope classification, MEDIUM confidence).",
    ),
    (
        "PERF-RS-LOOP-006",
        "Indirect call in innermost loop (Rust)",
        "Detects indirect (function-pointer/trait-object) calls inside innermost loop bodies (LOW confidence; field-method/closure dispatch is classified direct and not seen).",
    ),
    (
        "PERF-RS-LOOP-007",
        "Linear membership scan nested in loop (Rust)",
        "Detects a comparison against a subscript indexed by an enclosing loop's induction variable — hand-rolled O(n²) membership scan.",
    ),
    (
        "PERF-RS-LOOP-008",
        "Member-chain dereference in innermost loop (Rust)",
        "Detects field/index access chains of depth ≥ 3 inside innermost loop bodies — pointer/lifetime-chasing cache-miss candidates.",
    ),
    (
        "PERF-RS-MEM-001",
        "size_of passed as an element count (Rust)",
        "REFRAMED for Rust: detects `Vec::with_capacity`/`String::with_capacity` receiving a `size_of`/`size_of_val` byte count where an element count is expected (over-reserves by the element size).",
    ),
    (
        "PERF-RS-MEM-002",
        "Block copy/clone in loop (Rust)",
        "REFRAMED for Rust (no explicit size argument): detects block-copy methods (`copy_from_slice`/`copy_nonoverlapping`/`extend_from_slice`) inside loops — warn in the innermost loop, info otherwise. (`clone`/`to_vec` were removed by the M1 Precision Gate: borrow-checker clones and `Copy` bit-copies are not block copies.)",
    ),
    (
        "PERF-RS-STR-002",
        "Length query recomputed in loop (Rust)",
        "Detects `len`/`is_empty`/`capacity` recomputed inside a loop where the receiver is not reassigned in the innermost enclosing loop.",
    ),
    (
        "PERF-RS-STR-003",
        "Unbounded unsafe memory operation (Rust)",
        "REFRAMED for Rust (memory-safe by default): detects the unsafe unbounded-op family (`copy_nonoverlapping`/`copy`/`from_raw_parts`/`from_utf8_unchecked`/`set_len`) anywhere — unchecked-length surfaces.",
    ),
    (
        "PERF-RS-TYPE-001",
        "Large struct passed by value (Rust)",
        "Detects a bare-named struct parameter whose LP64 estimate reaches the approximate-size floor (LOW confidence — no real type layout); one finding per function aggregating qualifying parameters.",
    ),
    (
        "PERF-RS-TYPE-002",
        "Struct field ordering (padding candidate, Rust)",
        "Detects struct definitions whose declared-order aligned layout wastes padding versus the packed member sum beyond the threshold — but default `repr(Rust)` may reorder fields automatically, so this is a lower-confidence candidate (check `#[repr(C)]`/FFI).",
    ),
];

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Render the full `report.md` content (output-contract §5 template).
///
/// `emitted_charts` holds the chart names/filenames the charts module
/// produced (e.g. `"include-graph"` or `"include-graph.svg"`); §5 renders
/// links ONLY for names in this list. Pure function of `(doc,
/// emitted_charts)` — LF newlines, trailing newline, byte-deterministic
/// modulo `scan.started_at` / `scan.duration_ms`.
#[must_use]
pub fn render_report(doc: &MetricsDocument, emitted_charts: &[String]) -> String {
    let mut out = String::new();
    render_header(&mut out, doc);
    render_executive_summary(&mut out, doc);
    render_provenance_section(&mut out, doc);
    render_codebase_statistics(&mut out, doc);
    render_findings(&mut out, doc);
    render_architecture(&mut out, doc, emitted_charts);
    render_appendix_a(&mut out, doc);
    render_appendix_b(&mut out);
    // Exactly one trailing newline (output-contract §2 encoding rule).
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

// ---------------------------------------------------------------------------
// Section renderers (fixed order, output-contract §5)
// ---------------------------------------------------------------------------

fn render_header(out: &mut String, doc: &MetricsDocument) {
    out.push_str("# Perfpal Report\n");
    // Blockquote provenance line; duration_ms renders `n/a` inline when None.
    let duration = match doc.scan.duration_ms {
        Some(ms) => ms.to_string(),
        None => "n/a".to_string(),
    };
    out.push_str(&format!(
        "> schema_version {} · perfpal {} · scanned {}\n",
        escape_md(&doc.schema_version),
        escape_md(&doc.tool.version),
        escape_md(&doc.scan.root_path),
    ));
    out.push_str(&format!(
        "> started {} · duration {duration} ms · {} files\n\n",
        escape_md(&doc.scan.started_at),
        doc.scan.file_count,
    ));
}

fn render_executive_summary(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## 1. Executive summary\n");
    let hot = findings_of_severity(doc, Severity::Hot);
    let warn_count = doc
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warn)
        .count();
    let info_count = doc
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Info)
        .count();
    let health = match &doc.parse_health_summary {
        Some(s) => format!(
            "Parse health: {} ok, {} partial, {} failed.",
            s.ok, s.partial, s.failed
        ),
        None => "Parse health: n/a.".to_string(),
    };
    out.push_str(&format!(
        "{} findings: {} hot, {} warn, {} info. {health}\n",
        doc.findings.len(),
        hot.len(),
        warn_count,
        info_count,
    ));
    // First 3 HOT findings in §4 order (all if fewer than 3).
    for f in hot.iter().take(3) {
        out.push_str(&format!(
            "- {} ({})\n",
            escape_md(&f.title),
            escape_md(&f.finding_id),
        ));
    }
    out.push('\n');
}

fn render_provenance_section(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## 2. Scan provenance & parse health\n");
    out.push_str(&format!(
        "- Tool: {} {}\n",
        escape_md(&doc.tool.name),
        escape_md(&doc.tool.version),
    ));
    out.push_str(&format!(
        "- Scan root: `{}`\n",
        escape_md(&doc.scan.root_path),
    ));
    out.push_str(&format!("- Files scanned: {}\n", doc.scan.file_count));
    // Per-language totals (languages, loc_total) belong to §3 Codebase
    // statistics per output-contract §5 — rendered there, not here.

    // Parse-health clause — omitted ENTIRELY when the summary is absent.
    if let Some(summary) = &doc.parse_health_summary {
        out.push_str("\n### Parse health summary\n\n");
        out.push_str("| status | files |\n|---|---:|\n");
        out.push_str(&format!("| ok | {} |\n", summary.ok));
        out.push_str(&format!("| partial | {} |\n", summary.partial));
        out.push_str(&format!("| failed | {} |\n", summary.failed));

        let unhealthy: Vec<&crate::schema::FileEntry> = doc
            .files
            .iter()
            .filter(|f| {
                matches!(
                    f.parse_health.status,
                    crate::ir::ParseStatus::Partial | crate::ir::ParseStatus::Failed
                )
            })
            .collect();
        out.push('\n');
        if unhealthy.is_empty() {
            out.push_str("No failed or partially parsed files.\n");
        } else {
            out.push_str("| path | status | error nodes |\n|---|---|---:|\n");
            for f in unhealthy {
                out.push_str(&format!(
                    "| {} | {} | {} |\n",
                    escape_md(&f.path),
                    parse_status_token(f.parse_health.status),
                    f.parse_health.error_node_count,
                ));
            }
        }
        out.push_str(
            "\nFindings overlapping parse-error regions are flagged \
             (`error_node_overlap`) and confidence-capped — never silently dropped.\n",
        );
    }
    out.push('\n');
}

fn render_codebase_statistics(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## 3. Codebase statistics\n");
    // Per-language totals clause (output-contract §5 §3): languages +
    // loc_total, rendered FIRST so the section leads with scan-wide totals.
    out.push_str(&format!(
        "- Languages: {}\n",
        escape_md(&doc.scan.languages.join(", ")),
    ));
    match &doc.scan.loc_total {
        Some(loc) => out.push_str(&format!(
            "- Total LOC: {} code / {} comment / {} blank\n",
            loc.code, loc.comment, loc.blank,
        )),
        None => out.push_str("- Total LOC: n/a\n"),
    }
    out.push('\n');
    if doc.files.is_empty() {
        out.push_str("No files scanned.\n\n");
        return;
    }
    // Top 10 by loc.code DESC, ties broken by path ASC — depends only on
    // document content (defensive re-sort; scanner order is not trusted here).
    let mut ranked: Vec<&crate::schema::FileEntry> = doc.files.iter().collect();
    ranked.sort_by(|a, b| {
        b.loc
            .code
            .cmp(&a.loc.code)
            .then_with(|| a.path.cmp(&b.path))
    });
    out.push_str("Top files by code LOC:\n\n");
    out.push_str("| path | loc.code | loc.comment | loc.blank | functions |\n");
    out.push_str("|---|---:|---:|---:|---:|\n");
    for f in ranked.iter().take(10) {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            escape_md(&f.path),
            f.loc.code,
            f.loc.comment,
            f.loc.blank,
            f.function_count,
        ));
    }
    out.push('\n');

    // Distributions tables — omitted entirely while stats.distributions is
    // None (the canonical form this phase). When populated, entries arrive
    // pre-sorted by (metric, bucket); document order is preserved.
    if let Some(distributions) = &doc.stats.distributions {
        for (metric, entries) in group_by_metric(distributions) {
            out.push_str(&format!(
                "Distribution — {}:\n\n| bucket | count |\n|---|---:|\n",
                escape_md(metric),
            ));
            for entry in entries {
                out.push_str(&format!(
                    "| {} | {} |\n",
                    escape_md(&entry.bucket),
                    entry.count,
                ));
            }
            out.push('\n');
        }
    }
}

fn render_findings(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## 4. Findings\n");
    if doc.findings.is_empty() {
        out.push('\n');
        out.push_str("No findings.\n\n");
        return;
    }
    // Stable severity partition hot→warn→info; WITHIN a group document order
    // is preserved EXACTLY (the analyzer already emits the total order
    // severity→confidence→rule_id→path→start_line→start_col — do NOT re-sort).
    let ordered = [
        findings_of_severity(doc, Severity::Hot),
        findings_of_severity(doc, Severity::Warn),
        findings_of_severity(doc, Severity::Info),
    ]
    .concat();

    for f in ordered {
        out.push_str(&format!(
            "#### {}  (`{}`)\n",
            escape_md(&f.title),
            escape_md(&f.finding_id),
        ));
        match &f.symbol {
            Some(symbol) => out.push_str(&format!(
                "- Location: `{}:{}` ({})\n",
                escape_md(&f.path),
                f.span.start_line,
                escape_md(symbol),
            )),
            None => out.push_str(&format!(
                "- Location: `{}:{}`\n",
                escape_md(&f.path),
                f.span.start_line,
            )),
        }
        out.push_str(&format!("- Message: {}\n", escape_md(&f.message)));
        out.push_str(&format!(
            "- Confidence: {} · Severity: {} · Resolver tier: {}\n",
            confidence_token(f.confidence),
            severity_token(f.severity),
            resolver_tier_token(f.resolver_tier),
        ));
        let mut flags: Vec<&str> = Vec::new();
        if f.macro_suspect {
            flags.push("macro_suspect");
        }
        if f.error_node_overlap {
            flags.push("error_node_overlap");
        }
        if !flags.is_empty() {
            out.push_str(&format!("- Flags: {}\n", flags.join(", ")));
        }
        if !f.evidence.is_empty() {
            let pairs: Vec<String> = f
                .evidence
                .iter()
                .map(|(k, v)| format!("{}={}", escape_md(k), escape_md(v)))
                .collect();
            out.push_str(&format!("- Evidence: {}\n", pairs.join(", ")));
        }
        out.push('\n');
    }
}

fn render_architecture(out: &mut String, doc: &MetricsDocument, emitted_charts: &[String]) {
    out.push_str("## 5. Architecture\n");
    if emitted_charts.is_empty() {
        out.push_str("No charts were emitted for this scan.\n\n");
    } else {
        for name in emitted_charts {
            let stem = name.strip_suffix(".svg").unwrap_or(name);
            // Chart names are NOT escape_md'd: output-contract §3 pins them
            // to kebab-case ASCII ([a-z0-9]+(-[a-z0-9]+)*), so they carry no
            // Markdown metacharacters by construction.
            out.push_str(&format!("[{}](charts/{stem}.svg)\n", chart_caption(stem)));
        }
        out.push('\n');
    }

    if let Some(components) = &doc.components
        && !components.is_empty()
    {
        out.push_str("Component ranking (hottest first):\n\n");
        out.push_str("| component | path prefix | rank | finding density (per KLOC) |\n");
        out.push_str("|---|---|---:|---:|\n");
        // Document order IS the ranked emission order (rank, path_prefix)
        // per the aggregator contract — preserved verbatim, no re-sort.
        for c in components {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                escape_md(&c.component_id),
                escape_md(&c.path_prefix),
                c.rank,
                // Shared HALF-UP fixed-point formatter (scale 2) — same
                // numeric text the charts module emits for the same density.
                format_fixed_point(c.metrics.finding_density, 2),
            ));
        }
        out.push('\n');
    }

    out.push_str(
        "The dependency graph reflects resolved internal include/module edges \
         between scanned files — C `#include` directives and Rust `mod`/`use` \
         module paths; unresolved internal edges are retained as data, and \
         external (system headers/crates) dependencies are excluded from \
         coupling edges.\n",
    );
    out.push_str(
        "Coupling heat blends edge counts with per-function cost \
         (cost = cyclomatic + cognitive + 2·max_loop_depth + 3·alloc_site_count \
         + call fanout/fanin/depth, +2 if recursive, +1 each for error_path / \
         alloc_family; each edge weighs 1 + mean endpoint cost normalized by \
         the scan maximum) — charts remain pure functions of metrics.json.\n",
    );
    out.push_str(
        "Machine-readable graph: [graph.dot](charts/graph.dot) — deterministic \
         DOT export of the include/call/component graphs (same source data as \
         the SVGs above).\n",
    );
    out.push_str(
        "Standing caveat: cross-file call edges are LOW confidence until a \
         semantic resolver tier exists (ADR-0001).\n\n",
    );
}

fn render_appendix_a(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## Appendix A. Heuristic reference\n");
    if doc.findings.is_empty() {
        out.push_str("No findings — no heuristics to reference.\n\n");
        return;
    }
    // EXACTLY the rule_ids present in this report's findings, sorted for
    // determinism (ordering must depend only on document content).
    let rule_ids: BTreeSet<&str> = doc.findings.iter().map(|f| f.rule_id.as_str()).collect();
    out.push_str("| rule_id | title | description |\n|---|---|---|\n");
    for rule_id in rule_ids {
        let (_, title, description) = HEURISTIC_CATALOG_V1
            .iter()
            .find(|(id, _, _)| *id == rule_id)
            .unwrap_or_else(|| panic!("uncataloged rule_id {rule_id} reached Appendix A"));
        out.push_str(&format!(
            "| `{}` | {} | {} |\n",
            escape_md(rule_id),
            escape_md(title),
            escape_md(description),
        ));
    }
    out.push('\n');
}

fn render_appendix_b(out: &mut String) {
    out.push_str("## Appendix B. How to read this report\n");
    out.push_str(
        "**Severity** bands (`hot`, `warn`, `info`) rank expected runtime or \
         architecture impact: review `hot` findings first, schedule `warn`, \
         and treat `info` as context signal feeding aggregation.\n",
    );
    out.push_str(
        "**Confidence** tiers (`HIGH`, `MEDIUM`, `LOW`) express the \
         false-positive risk of the detection shape — pure-syntax detections \
         are HIGH, shape heuristics with documented FP classes are MEDIUM, and \
         approximations (size models, cross-file name matching) are LOW. \
         Downgrade flags (`macro_suspect`, `error_node_overlap`) mark findings \
         whose evidence quality is reduced; their confidence is capped at \
         MEDIUM.\n",
    );
    out.push_str(
        "Every finding is an **optimization candidate, never a certainty**: a \
         candidate for human review, not a defect claim. Perfpal v1 runs on a \
         purely syntactic tier (`resolver_tier: syntactic`) — no macro \
         expansion, no type resolution.\n",
    );
    out.push_str(&format!(
        "For programmatic consumption, read `metrics.json` — the source of \
         truth — and validate it against the bundled schema \
         `{}` shipped alongside it. Where this report and \
         `metrics.json` ever disagree, `metrics.json` wins.\n\n",
        crate::schema::SCHEMA_ARTIFACT_NAME
    ));
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Findings of one severity, in DOCUMENT order (never re-sorted here).
fn findings_of_severity(doc: &MetricsDocument, severity: Severity) -> Vec<&FindingEntry> {
    doc.findings
        .iter()
        .filter(|f| f.severity == severity)
        .collect()
}

/// Group distribution entries by metric, preserving document order within
/// and across groups (entries arrive pre-sorted by `(metric, bucket)`).
fn group_by_metric(entries: &[DistributionEntry]) -> Vec<(&str, Vec<&DistributionEntry>)> {
    let mut groups: Vec<(&str, Vec<&DistributionEntry>)> = Vec::new();
    for entry in entries {
        match groups.last_mut() {
            Some((metric, list)) if *metric == entry.metric.as_str() => list.push(entry),
            _ => groups.push((entry.metric.as_str(), vec![entry])),
        }
    }
    groups
}

/// Fixed caption per known chart-set-v1 name (ADR-0002); unknown names get a
/// generic caption (forward compatibility: consumers tolerate new charts).
/// Captions are reporter-owned static text — safe by construction, never
/// document-derived (the dynamic name is charset-pinned per output-contract
/// §3), so they bypass [`escape_md`].
fn chart_caption(stem: &str) -> String {
    match stem {
        "include-graph" => {
            "Dependency graph — internal #include (C) and mod/use (Rust) dependencies between scanned files."
                .to_string()
        }
        "component-coupling" => {
            "Component coupling — cross-file call edges between components.".to_string()
        }
        "hot-components" => {
            "Hot components — ranking by aggregated bottleneck metrics.".to_string()
        }
        other => format!("Chart: {other}."),
    }
}

/// Escape Markdown metacharacters with the PINNED set [`MD_ESCAPE_CHARS`].
/// Applied to every embedded path, title, message, id, symbol, and caption.
fn escape_md(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if MD_ESCAPE_CHARS.contains(&c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn severity_token(severity: Severity) -> &'static str {
    match severity {
        Severity::Hot => "hot",
        Severity::Warn => "warn",
        Severity::Info => "info",
    }
}

fn confidence_token(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::High => "HIGH",
        Confidence::Medium => "MEDIUM",
        Confidence::Low => "LOW",
    }
}

fn resolver_tier_token(tier: ResolverTier) -> &'static str {
    match tier {
        ResolverTier::Syntactic => "syntactic",
        ResolverTier::Semantic => "semantic",
        ResolverTier::Dynamic => "dynamic",
    }
}

fn parse_status_token(status: crate::ir::ParseStatus) -> &'static str {
    match status {
        crate::ir::ParseStatus::Ok => "ok",
        crate::ir::ParseStatus::Partial => "partial",
        crate::ir::ParseStatus::Failed => "failed",
    }
}

#[cfg(test)]
mod tests {
    //! Unit locks on the two tiny pure helpers; full-template obligations live
    //! in tests/reporter_render.rs.

    use super::*;

    #[test]
    fn escape_md_pins_exact_set() {
        assert_eq!(escape_md("plain"), "plain");
        assert_eq!(
            escape_md("#incl<ude>&\"x\".c"),
            "\\#incl\\<ude\\>\\&\"x\"\\.c"
        );
        assert_eq!(escape_md("a|b"), "a\\|b");
        assert_eq!(
            escape_md("*_{}[]()+-!`\\"),
            "\\*\\_\\{\\}\\[\\]\\(\\)\\+\\-\\!\\`\\\\"
        );
    }

    #[test]
    fn round_half_up_is_fixed_point_two_half_up() {
        // Locks the SHARED formatter (crate::schema::format_fixed_point) at
        // the reporter's scale-2 call shape.
        assert_eq!(format_fixed_point(0.0, 2), "0.00");
        assert_eq!(format_fixed_point(8.25, 2), "8.25");
        assert_eq!(format_fixed_point(2.675, 2), "2.68"); // ties-to-even would say 2.67
        assert_eq!(format_fixed_point(1.005, 2), "1.01");
        assert_eq!(format_fixed_point(12.5, 2), "12.50");
        assert_eq!(format_fixed_point(0.999, 2), "1.00"); // carry into integer part
        assert_eq!(format_fixed_point(-2.675, 2), "-2.68");
        // Negative-zero guarantee: "-0.00" must never leak.
        assert_eq!(format_fixed_point(-0.001, 2), "0.00");
    }

    /// Completeness lock (plans `rust-backend` R3 D1 + `python-backend` W3):
    /// the catalog must cover EXACTLY the rule ids the analyzer registry can
    /// emit (23 `PERF-C-*` + 22 `PERF-PY-*` + 22 `PERF-RS-*`; C/Rust STR-001
    /// and the 10 Python-retired ids never registered), so a newly added
    /// heuristic cannot silently reintroduce the Appendix A `panic!` on an
    /// uncataloged rule_id. The expected set is derived FROM the registry
    /// ([`crate::analyzer::registered_rule_ids`]) — an id added to `REGISTRY`
    /// without a catalog entry now fails THIS test instead of passing a
    /// hand-maintained duplicate. Every catalog entry also has a non-empty
    /// title/description.
    #[test]
    fn catalog_covers_every_registry_rule_id() {
        // Authoritative: the registry's own rule ids, in declaration order.
        let registry_ids: Vec<&'static str> = crate::analyzer::registered_rule_ids().collect();
        assert_eq!(
            registry_ids.len(),
            67,
            "expected 23 C + 22 Python + 22 Rust ids"
        );

        // The registry declares its entries in ascending rule_id order
        // (the C block, then the `PERF-RS-*` block) — declaration order is
        // the load-bearing first-wins duplicate-collapse contract
        // (modules.md §3.3), so lock it globally with one comparison.
        let mut globally_sorted = registry_ids.clone();
        globally_sorted.sort_unstable();
        assert_eq!(
            registry_ids, globally_sorted,
            "REGISTRY must declare entries in ascending rule_id order"
        );

        // No duplicate registry rule ids.
        let mut deduped = registry_ids.clone();
        deduped.sort_unstable();
        deduped.dedup();
        assert_eq!(
            deduped.len(),
            registry_ids.len(),
            "duplicate registry rule_id"
        );

        let catalog_ids: Vec<&str> = HEURISTIC_CATALOG_V1.iter().map(|(id, _, _)| *id).collect();
        assert_eq!(catalog_ids.len(), 67, "catalog entry count drifted");

        // Every catalog entry carries a non-empty title and description.
        for (id, title, description) in HEURISTIC_CATALOG_V1 {
            assert!(!title.is_empty(), "empty title for {id}");
            assert!(!description.is_empty(), "empty description for {id}");
        }

        // No duplicate rule ids in the catalog.
        let mut deduped_catalog = catalog_ids.clone();
        deduped_catalog.sort_unstable();
        deduped_catalog.dedup();
        assert_eq!(
            deduped_catalog.len(),
            catalog_ids.len(),
            "duplicate catalog rule_id"
        );

        let expected: BTreeSet<&str> = registry_ids.iter().copied().collect();
        let actual: BTreeSet<&str> = catalog_ids.iter().copied().collect();
        assert_eq!(
            actual, expected,
            "catalog must cover exactly the registered rule ids"
        );

        // The Python and Rust blocks are appended in ascending rule_id order
        // (the C block keeps its frozen catalog-v1 doc order).
        for prefix in ["PERF-PY-", "PERF-RS-"] {
            let ids: Vec<&str> = catalog_ids
                .iter()
                .copied()
                .filter(|id| id.starts_with(prefix))
                .collect();
            let mut sorted = ids.clone();
            sorted.sort_unstable();
            assert_eq!(ids, sorted, "{prefix}* catalog entries not ascending");
        }

        // The 10 Python-retired C/Rust ids must never be registered for
        // Python (docs/architecture/python-backend.md §9.1).
        const RETIRED_PYTHON_IDS: [&str; 10] = [
            "PERF-PY-TYPE-001",
            "PERF-PY-TYPE-002",
            "PERF-PY-MEM-001",
            "PERF-PY-ALLOC-002",
            "PERF-PY-STR-002",
            "PERF-PY-STR-003",
            "PERF-PY-LOOP-004",
            "PERF-PY-LOOP-005",
            "PERF-PY-LOOP-006",
            "PERF-PY-LOOP-008",
        ];
        for retired in RETIRED_PYTHON_IDS {
            assert!(
                !actual.contains(retired),
                "retired Python rule id must not be registered/cataloged: {retired}"
            );
        }
    }
}
