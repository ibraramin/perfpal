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
//! re-sorted here for the §3 table; findings are grouped by rule_id in the
//! pinned group order severity-rank → count-desc → rule_id-asc, with members
//! in `finding_id`-ascending order regardless of input order). Double-render
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

pub mod atlas;
pub mod atlas_brief;
pub mod atlas_layout;
pub mod atlas_metrics;
pub mod atlas_render;
pub mod views;

use std::collections::{BTreeMap, BTreeSet};

use crate::analyzer::{Confidence, Severity};
use crate::ir::ResolverTier;
use crate::schema::{DistributionEntry, FindingEntry, MetricsDocument, format_fixed_point};

// ---------------------------------------------------------------------------
// Pinned constants
// ---------------------------------------------------------------------------

/// Human-report cap (output-contract §4 bounded-human rule, C-full-split
/// follow-up): the §5 component table / Component index and the per-file
/// finding-link lists stay bounded at any scale. Same `take(10)` precedent
/// as the §3 top-files table and the `#files` drill-down. Full component and
/// finding data stays in `metrics.json` (machine source of truth) and the
/// uncapped `findings-detail.*` artifacts — the caps only bound the human
/// pages, with honest `showing top … of M` / `…+N more` counts so a cap can
/// never silently drop rows without breaking the header.
const HUMAN_CAP: usize = 10;

/// Pinned Markdown metacharacter escape set (output-contract §2 paths rule).
/// Every char here is backslash-escaped wherever document-derived text is
/// embedded. `|` inclusion makes table cells safe without a second pass.
///
/// `pub` (not private) so the integration-test harness imports this single
/// source of truth instead of hand-mirroring it.
pub const MD_ESCAPE_CHARS: &[char] = &[
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

/// Escape HTML special characters (`& < > " '`) for safe embedding in
/// element text and double-quoted attribute values (output-contract §2
/// paths rule, HTML counterpart of [`escape_md`]).
///
/// Single pass over chars — `&` is escaped first by construction (match arms
/// emit `&amp;` literally, never re-scanned), so `&` cannot be re-escaped.
/// Every interpolated path/title/message/id/symbol in [`render_html`] goes
/// through this function; chart filenames and captions are charset-pinned
/// static text (output-contract §3 kebab-case) and bypass it by construction,
/// exactly like [`escape_md`] bypasses chart names in [`render_report`].
#[must_use]
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Render the full `report.html` content (Option B slice 2: slice-1 LINKED
/// page plus a small fixed enhancement script).
///
/// A static self-contained page with ONE inline `<script>` block
/// ([`REPORT_SCRIPT`], fixed source, no libraries): it references
/// `charts/*.svg` via relative `<img src>`/`<a href>` links, so the HTML
/// stays small on big repos while remaining fully offline within the report
/// directory. Pure function of `(doc, emitted_charts)` — LF newlines,
/// trailing newline, byte-deterministic with NO provenance surfaces at all
/// (no `started_at`, no `duration_ms`, no `root_path`; header carries only
/// `schema_version`, tool version, file/finding counts).
///
/// Contents: header, the Perfpal Atlas `#map` page (100%-graph canvas +
/// fixed seven-block brief + drills, ADR-0009), the Explorer chart links,
/// component index (top-10 rank, prefix, metrics + detail-page links, with an
/// honest `showing top … of M` note), file drill-down (top files by
/// degree/cost + their top-10 finding_id lists linking into
/// `findings-detail.html`), findings section (C-split: per-rule
/// `<details><summary>` groups ONLY, each linking to the detail page
/// anchors — zero per-finding rows here). Drill-down anchors link
/// component → file → finding via deterministic `id` anchors derived from
/// `(rank, slug)`, file slugs, and sanitized finding ids (finding-row ids
/// live in the detail artifact).
#[must_use]
pub fn render_html(doc: &MetricsDocument, emitted_charts: &[String]) -> String {
    let mut out = String::new();
    html_head(&mut out, doc);
    // Report-only breadcrumb (inside the shared header, right after the h1):
    // findings-detail.html calls the same header WITHOUT it, keeping that
    // artifact's bytes a hard contract.
    html_header(&mut out, doc, true);
    html_nav(&mut out);
    html_map(&mut out, doc, emitted_charts);
    html_architecture(&mut out, doc, emitted_charts);
    html_components(&mut out, doc, emitted_charts);
    html_files(&mut out, doc, emitted_charts);
    html_findings(&mut out, doc);
    html_appendix_a(&mut out, doc);
    html_footer(&mut out);
    // Exactly one trailing newline (output-contract §2 encoding rule; the
    // writer normalizes anyway).
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

/// Render the full `report.md` content (output-contract §5 template).
///
/// `emitted_charts` holds the chart names/filenames the charts module
/// produced (e.g. `"include-graph"` or `"include-graph.svg"`); §5 renders
/// links ONLY for names in this list. Pure function of `(doc,
/// emitted_charts)` — LF newlines, trailing newline, byte-deterministic
/// modulo `scan.started_at` / `scan.duration_ms`.
///
/// Human-scale bound (C-split): `report.md` carries NO per-finding rows —
/// the uncapped detail lives in the `findings-detail.md` artifact
/// ([`render_findings_detail`]); Appendix C is a link stub only.
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
    render_appendix_b(&mut out, semantic_tier_active(doc));
    render_appendix_c(&mut out, doc);
    // Exactly one trailing newline (output-contract §2 encoding rule).
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

/// Render the standalone `findings-detail.md` artifact (C-split promotion
/// of [`render_findings_full_detail` — called VERBATIM, never reimplemented).
///
/// Bounded-human companion to [`render_report`]: the full uncapped
/// per-finding detail in the legacy total order (stable severity partition
/// hot→warn→info; document order within a group). Pure function of `doc` —
/// LF newlines, trailing newline, byte-deterministic with NO provenance
/// surfaces at all (no `started_at`, no `duration_ms`, no `root_path`), so
/// double-render output is byte-identical UNMASKED.
#[must_use]
pub fn render_findings_detail(doc: &MetricsDocument) -> String {
    let mut out = String::new();
    out.push_str("# Perfpal Findings Detail\n");
    if doc.findings.is_empty() {
        out.push_str("\nNo findings.\n");
    } else {
        out.push_str(&format!(
            "\n{} of {} total {}.\n\n",
            doc.findings.len(),
            doc.findings.len(),
            views::plural_word(doc.findings.len(), "finding", "findings"),
        ));
        render_ranked_actions_md(&mut out, doc);
        render_findings_full_detail(&mut out, doc);
    }
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

/// Render the standalone `findings-detail.html` artifact (C-split promotion
/// of [`html_findings_table`]/[`html_finding_row`] — called VERBATIM, never
/// reimplemented — inside a standalone page reusing the existing
/// head/style/table markup and the fixed [`REPORT_SCRIPT`] enhancement).
///
/// Per-rule sections in the pinned [`group_findings_by_rule`] order, each
/// with a `rule-<rule_id>` anchor plus that rule's rows in the shared full
/// table markup; finding-row anchors come from the SAME
/// [`finding_anchor_assignments`] map as `report.html`, so every
/// `findings-detail.html#…` link in the human page resolves here. Pure
/// function of `doc` — LF newlines, trailing newline, byte-deterministic
/// with NO provenance surfaces at all.
#[must_use]
pub fn render_findings_detail_html(doc: &MetricsDocument) -> String {
    let mut out = String::new();
    html_detail_head(&mut out, doc);
    html_header(&mut out, doc, false);
    html_detail_nav(&mut out);
    html_detail_findings(&mut out, doc);
    html_footer(&mut out);
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

/// New top section of `findings-detail.md`: every function-granularity
/// finding in KV1 order. The legacy full-detail sections below are kept
/// verbatim; the explicit `<a id="ranked-actions">` anchor is the stable
/// cross-artifact target.
fn render_ranked_actions_md(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## Complete ranked action list (KV1 order)\n\n");
    out.push_str("<a id=\"ranked-actions\"></a>\n\n");
    let actions = views::complete_actions(doc);
    if actions.is_empty() {
        out.push_str("No function-granularity actions.\n\n");
        return;
    }
    let anchors = finding_anchor_assignments(doc).1;
    out.push_str("| rank | finding | title | location | function | severity | confidence | path class | cost anchor |\n");
    out.push_str("|---:|---|---|---|---|---|---|---|---|\n");
    for row in actions {
        let finding = &doc.findings[row.finding_index];
        let location = format!("{}:{}", finding.path, finding.span.start_line);
        let function = row
            .function
            .as_deref()
            .map_or_else(|| "—".to_string(), |name| format!("`{}`", escape_md(name)));
        out.push_str(&format!(
            "| {} | [`{}`](#{}) | {} | `{}` | {} | {} | {} | {} | {} |\n",
            row.rank,
            escape_md(&finding.finding_id),
            &anchors[row.finding_index],
            escape_md(&finding.title),
            escape_md(&location),
            function,
            severity_token(finding.severity),
            confidence_token(finding.confidence),
            escape_md(&row.path_class),
            row.cost_anchor
                .as_deref()
                .map_or_else(|| "—".to_string(), escape_md),
        ));
    }
    out.push('\n');
}

/// Ranked-action table emitted at the top of `findings-detail.html`.
fn html_ranked_actions(out: &mut String, doc: &MetricsDocument, anchors: &[String]) {
    out.push_str(
        "<section id=\"ranked-actions\">\n<h2>Complete ranked action list (KV1 order)</h2>\n",
    );
    let actions = views::complete_actions(doc);
    if actions.is_empty() {
        out.push_str("<p>No function-granularity actions.</p>\n</section>\n");
        return;
    }
    out.push_str(
        "<div class=\"table-scroll\">\n<table>\n<thead><tr><th>rank</th><th>finding_id</th>\
         <th>title</th><th>location</th><th>function</th><th>severity</th>\
         <th>confidence</th><th>path class</th><th>cost anchor</th></tr></thead>\n<tbody>\n",
    );
    for row in actions {
        let finding = &doc.findings[row.finding_index];
        let location = format!("{}:{}", finding.path, finding.span.start_line);
        out.push_str(&format!(
            "<tr><td>{rank}</td><td><a href=\"#{anchor}\">{id}</a></td>\
             <td>{title}</td><td>{location}</td><td>{function}</td>\
             <td>{severity}</td><td>{confidence}</td><td>{class}</td>\
             <td class=\"wrap\">{cost}</td></tr>\n",
            rank = row.rank,
            anchor = escape_html(&anchors[row.finding_index]),
            id = escape_html(&finding.finding_id),
            title = escape_html(&finding.title),
            location = escape_html(&location),
            function = escape_html(row.function.as_deref().unwrap_or("—")),
            severity = severity_token(finding.severity),
            confidence = confidence_token(finding.confidence),
            class = escape_html(&row.path_class),
            cost = escape_html(row.cost_anchor.as_deref().unwrap_or("—")),
        ));
    }
    out.push_str("</tbody>\n</table>\n</div>\n</section>\n");
}

/// Render the standalone `profile-queue.md` artifact: the complete
/// cost-ranked function list (demoted rows labelled, v1 rows marked `(v1)`).
#[must_use]
pub fn render_profile_queue(doc: &MetricsDocument) -> String {
    let mut out = String::new();
    out.push_str("# Perfpal Profile Queue\n\n");
    let rows = views::complete_profile_queue(doc);
    out.push_str(&format!(
        "{} ranked by static cost — candidates, confirm with a profiler.\n\n",
        views::plural(rows.len(), "function"),
    ));
    if rows.is_empty() {
        while out.ends_with('\n') {
            out.pop();
        }
        out.push('\n');
        return out;
    }
    out.push_str("| rank | function | location | path class | cost | notes |\n");
    out.push_str("|---:|---|---|---|---|---|\n");
    for row in &rows {
        let function = &doc.stats.functions[row.function_index];
        let location = format!("{}:{}", row.path, function.span.start_line);
        let mut notes: Vec<String> = Vec::new();
        if let Some(label) = row.demotion_label() {
            notes.push(label);
        }
        if let Some(badge) = row.unknown_trip_badge() {
            notes.push(badge);
        }
        if let Some(badge) = row.known_trip_badge() {
            notes.push(badge);
        }
        let notes = if notes.is_empty() {
            "—".to_string()
        } else {
            notes.join("; ")
        };
        out.push_str(&format!(
            "| {} | `{}` | `{}` | {} | {} | {} |\n",
            row.rank,
            escape_md(&row.function),
            escape_md(&location),
            escape_md(&row.path_class),
            row.cost_anchor,
            escape_md(&notes),
        ));
    }
    out.push('\n');
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

/// Render the standalone `profile-queue.html` artifact (mirrors the
/// findings-detail page structure: shared head/style, fixed script, one
/// deterministic table).
#[must_use]
pub fn render_profile_queue_html(doc: &MetricsDocument) -> String {
    let mut out = String::new();
    html_profile_head(&mut out, doc);
    let rows = views::complete_profile_queue(doc);
    out.push_str("<header>\n<h1>Perfpal Profile Queue</h1>\n");
    out.push_str(&format!(
        "<p>{}</p>\n</header>\n",
        escape_html(&format!(
            "{} ranked by static cost — candidates, confirm with a profiler",
            views::plural(rows.len(), "function")
        )),
    ));
    out.push_str(
        "<nav>\n<ul>\n\
         <li><a href=\"report.html\">Report</a></li>\n\
         <li><a href=\"#profile-queue\">Profile queue</a></li>\n\
         </ul>\n</nav>\n",
    );
    out.push_str("<section id=\"profile-queue\">\n<h2>Complete cost-ranked list</h2>\n");
    if rows.is_empty() {
        out.push_str("<p>No functions recorded.</p>\n");
    } else {
        out.push_str(
            "<div class=\"table-scroll\">\n<table>\n<thead><tr><th>rank</th><th>function</th>\
             <th>location</th><th>path class</th><th>cost</th><th>notes</th></tr></thead>\n<tbody>\n",
        );
        for row in &rows {
            let function = &doc.stats.functions[row.function_index];
            let location = format!("{}:{}", row.path, function.span.start_line);
            let mut notes: Vec<String> = Vec::new();
            if let Some(label) = row.demotion_label() {
                notes.push(format!(
                    "<span class=\"kv-dem\">{}</span>",
                    escape_html(&label)
                ));
            }
            if let Some(badge) = row.unknown_trip_badge() {
                notes.push(format!(
                    "<span class=\"kv-badge trip\">{}</span>",
                    escape_html(&badge)
                ));
            }
            if let Some(badge) = row.known_trip_badge() {
                notes.push(format!(
                    "<span class=\"kv-badge known\">{}</span>",
                    escape_html(&badge)
                ));
            }
            let notes = if notes.is_empty() {
                "—".to_string()
            } else {
                notes.join(" ")
            };
            out.push_str(&format!(
                "<tr><td>{rank}</td><td class=\"wrap\">{function}</td>\
                 <td class=\"wrap\">{location}</td><td>{class}</td>\
                 <td class=\"wrap\">{cost}</td><td class=\"wrap\">{notes}</td></tr>\n",
                rank = row.rank,
                function = escape_html(&row.function),
                location = escape_html(&location),
                class = escape_html(&row.path_class),
                cost = escape_html(&row.cost_anchor),
                notes = notes,
            ));
        }
        out.push_str("</tbody>\n</table>\n</div>\n");
    }
    out.push_str("</section>\n");
    html_footer(&mut out);
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

/// Profile-queue head: findings-detail markup/title with the key-view CSS.
fn html_profile_head(out: &mut String, doc: &MetricsDocument) {
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    out.push_str("<link rel=\"icon\" href=\"data:,\">\n");
    out.push_str(&format!(
        "<title>Perfpal Profile Queue — schema {}</title>\n",
        escape_html(&doc.schema_version),
    ));
    out.push_str(
        "<style>\n\
         body{font-family:monospace,monospace;max-width:72rem;margin:2rem auto;padding:0 1rem;color:#111;background:#fff}\n\
         .table-scroll{overflow-x:auto;max-width:100%}\n\
         table{border-collapse:collapse;width:100%;margin:1rem 0}\n\
         th,td{border:1px solid #999;padding:.25rem .5rem;text-align:left;vertical-align:top}\n\
         th{background:#eee}\n\
         #profile-queue tbody td{white-space:nowrap}\n\
         #profile-queue td.wrap,#profile-queue th{white-space:normal;overflow-wrap:anywhere}\n\
         nav ul{list-style:none;padding:0;display:flex;gap:1rem;flex-wrap:wrap}\n",
    );
    out.push_str(KEY_VIEW_CSS);
    out.push_str("         </style>\n</head>\n<body>\n");
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
        "{} findings: {} hot, {} warn, {} info. {health}\n\n",
        doc.findings.len(),
        hot.len(),
        warn_count,
        info_count,
    ));

    let kv1 = views::kv1(doc);
    let anchors = finding_anchor_assignments(doc).1;
    out.push_str("### Where do I look first?\n\n");
    out.push_str(
        "Ranked: production code first, then severity, then confidence, then static cost.\n\n",
    );
    if kv1.rows.is_empty() {
        out.push_str("No function-granularity actions to rank.\n\n");
    } else {
        let mut seen_locations: BTreeMap<(String, u32), usize> = BTreeMap::new();
        for row in &kv1.rows {
            let finding = &doc.findings[row.finding_index];
            let location = format!("{}:{}", finding.path, finding.span.start_line);
            let same_as = seen_locations
                .get(&(finding.path.clone(), finding.span.start_line))
                .copied();
            out.push_str(&format!(
                "{}. **{}**{} ([`{}`](findings-detail.md#{})) — `{}`",
                row.rank,
                escape_md(&finding.title),
                same_as.map_or_else(String::new, |prior| format!(" (same location as #{prior})")),
                escape_md(&finding.finding_id),
                &anchors[row.finding_index],
                escape_md(&location),
            ));
            seen_locations
                .entry((finding.path.clone(), finding.span.start_line))
                .or_insert(row.rank);
            if let Some(function) = &row.function {
                out.push_str(&format!(" · function `{}`", escape_md(function)));
            }
            out.push_str(&format!(
                " · {} · {} · {}\n",
                severity_token(finding.severity),
                confidence_token(finding.confidence),
                escape_md(&row.path_class),
            ));
            if let Some(anchor) = &row.cost_anchor {
                out.push_str(&format!("   - Cost: `{anchor}`\n"));
            }
            out.push_str(&format!("   - Why: {}\n", escape_md(&finding.message)));
        }
        out.push_str(&format!(
            "\nshowing top {} of {} actions\n\n[Complete ranked list](findings-detail.md#ranked-actions) — all {} findings (production-first).\n\n",
            kv1.rows.len(),
            kv1.total_actions,
            kv1.total_actions,
        ));
        out.push_str(&format!(
            "{} demoted · {} in Explorer.\n\n",
            views::plural(kv1.nonprod_file_count, "non-production file"),
            views::plural(kv1.component_findings, "component-level finding"),
        ));
    }

    let kv2 = views::kv2(doc);
    out.push_str("### What breaks if I touch it?\n\n");
    if kv2.rows.is_empty() {
        out.push_str("No resolved include dependents to rank.\n\n");
    } else {
        for row in kv2.rows.iter().take(5) {
            let class = if row.production {
                String::new()
            } else {
                format!(" · {}", escape_md(&row.path_class))
            };
            let demoted = if row.demoted { " · demoted" } else { "" };
            out.push_str(&format!(
                "- `{}` — {}{class}{demoted}\n",
                escape_md(&row.path),
                row.value_label
            ));
        }
        out.push_str(&format!(
            "\nshowing top {} of {} files with resolved dependents · {} demoted\n\nBlast-radius bars: [report.html#map](report.html#map). resolved {} of {} include edges.\n\n",
            kv2.rows.len().min(5),
            kv2.total_with_dependents,
            views::plural(kv2.nonprod_file_count, "non-production file"),
            kv2.resolved_edges,
            kv2.total_edges,
        ));
    }

    let kv3 = views::kv3(doc);
    out.push_str("### Where might time plausibly go?\n\n");
    out.push_str(
        "Bars scale within each block: blue = ranked functions, grey = demoted functions.\n\n",
    );
    if kv3.rows.is_empty() {
        out.push_str("No functions to rank.\n\n");
    } else {
        let shown = kv3.rows.len().min(5);
        for row in kv3.rows.iter().take(5) {
            let line = doc.stats.functions[row.function_index].span.start_line;
            let mut extras: Vec<String> = Vec::new();
            if let Some(label) = row.demotion_label() {
                extras.push(label);
            }
            if let Some(badge) = row.unknown_trip_badge() {
                extras.push(badge);
            }
            if let Some(badge) = row.known_trip_badge() {
                extras.push(badge);
            }
            let suffix = if extras.is_empty() {
                String::new()
            } else {
                format!(" · {}", extras.join(" · "))
            };
            out.push_str(&format!(
                "- `{}:{}` `{}` — {}{}\n",
                escape_md(&row.path),
                line,
                escape_md(&row.function),
                row.cost_anchor,
                suffix,
            ));
        }
        out.push_str(&format!(
            "\nshowing top {shown} of {} functions\n\n[Complete cost-ranked list ({})](profile-queue.md).\n\n",
            kv3.total_functions,
            views::plural(kv3.total_functions, "function"),
        ));
        out.push_str(&format!(
            "{} demoted · {} demoted · {} demoted.\n\n",
            views::plural(kv3.startup_count, "startup function"),
            views::plural(kv3.no_loop_count, "no-loop function"),
            views::plural(kv3.nonprod_file_count, "non-production file"),
        ));
    }
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
    // Group-and-fold summary (output-contract §5 §4): ONE row per firing
    // rule with an honest "R rules: N of M total" header (N always equals M
    // today — every finding is represented via its rule group — so a future
    // cap cannot silently drop rows without breaking the header). The
    // uncapped per-finding detail lives in the `findings-detail.md`
    // artifact ([`render_findings_detail`] → [`render_findings_full_detail`]);
    // the exemplar cites the exact finding_id so it cross-references the
    // detail artifact mechanically.
    let groups = group_findings_by_rule(doc);
    let (hot_count, warn_count, info_count) = severity_counts(doc);
    out.push_str(&format!(
        "{} rules: {} of {} total findings ({} hot, {} warn, {} info).\n\n",
        groups.len(),
        doc.findings.len(),
        doc.findings.len(),
        hot_count,
        warn_count,
        info_count,
    ));
    out.push_str("| rule_id | findings | hot | warn | info | top exemplar |\n");
    out.push_str("|---|---:|---:|---:|---:|---|\n");
    for group in &groups {
        // Exemplar: first member in finding_id order (deterministic); cited
        // by exact finding_id so it cross-references Appendix C mechanically.
        // The trailing location is the unambiguous three-part
        // `path:start_line:start_col` (matching the HTML row location cell) —
        // never a range: the second number is a column, not an end line.
        let exemplar = &doc.findings[group.members[0]];
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} | {} (`{}`) — `{}:{}:{}` |\n",
            escape_md(group.rule_id),
            group.members.len(),
            group.hot,
            group.warn,
            group.info,
            escape_md(&exemplar.title),
            escape_md(&exemplar.finding_id),
            escape_md(&exemplar.path),
            exemplar.span.start_line,
            exemplar.span.start_col,
        ));
    }
    out.push('\n');
}

/// The UNCAPPED per-finding detail renderer, kept as a SEPARATE function by
/// design (group-and-fold seam): §4 above summarizes, this renders every
/// finding in full. The future option-C artifact split promotes THIS function
/// into its own artifact — do not delete it, inline it into §4, or render a
/// second copy of its rows anywhere else in `report.md`.
///
/// Ordering (legacy total order, preserved verbatim): stable severity
/// partition hot→warn→info; WITHIN a group document order is preserved
/// EXACTLY (the analyzer already emits the total order
/// severity→confidence→rule_id→path→start_line→start_col — do NOT re-sort).
fn render_findings_full_detail(out: &mut String, doc: &MetricsDocument) {
    // Stable severity partition hot→warn→info; WITHIN a group document order
    // is preserved EXACTLY (the analyzer already emits the total order
    // severity→confidence→rule_id→path→start_line→start_col — do NOT re-sort).
    let ordered: Vec<usize> = [Severity::Hot, Severity::Warn, Severity::Info]
        .into_iter()
        .flat_map(|severity| {
            doc.findings
                .iter()
                .enumerate()
                .filter(move |(_, finding)| finding.severity == severity)
                .map(|(index, _)| index)
        })
        .collect();
    let anchors = finding_anchor_assignments(doc).1;

    for index in ordered {
        let f = &doc.findings[index];
        out.push_str(&format!(
            "<a id=\"{}\"></a>\n#### {}  (`{}`)\n",
            &anchors[index],
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
    out.push_str("## 5. Explorer\n");
    // Emitted stems (`.svg`-suffixed or bare — both forms accepted, mirroring
    // the pipe contract in `crate::cli`); per-component detail pages are
    // indexed under the component table below, not in the overview link list.
    // NOTE: `component-coupling.svg` is an OVERVIEW chart — only stems in
    // the exact `detail_stems` registry below count as detail pages.
    let emitted_stems: BTreeSet<String> = emitted_charts
        .iter()
        .map(|name| name.strip_suffix(".svg").unwrap_or(name).to_string())
        .collect();
    // Exact detail-page registry (B4 pin): reconstructed via the
    // charts-owned pinning function from THIS document's components —
    // never guessed, never a name heuristic. A stem routes to the detail
    // path ONLY when it names a page emitted for a component in this
    // document; every other stem — known overviews AND unknown/colliding
    // names like `component-5-notes` — takes the overview path.
    let detail_stems: BTreeSet<String> = doc
        .components
        .iter()
        .flatten()
        .map(|c| {
            let filename = crate::charts::component::component_filename(c.rank, &c.path_prefix);
            filename
                .strip_suffix(".svg")
                .unwrap_or(&filename)
                .to_string()
        })
        .collect();
    let mut component_stems: Vec<String> = Vec::new();
    if emitted_charts.is_empty() {
        out.push_str("No charts were emitted for this scan.\n\n");
    } else {
        let mut any_overview = false;
        // Emission order is preserved verbatim (the charts module owns it).
        for name in emitted_charts {
            let stem = name.strip_suffix(".svg").unwrap_or(name);
            if detail_stems.contains(stem) {
                if !component_stems.iter().any(|s| s == stem) {
                    component_stems.push(stem.to_string());
                }
                continue;
            }
            // Chart names are NOT escape_md'd: output-contract §3 pins them
            // to kebab-case ASCII ([a-z0-9]+(-[a-z0-9]+)*), so they carry no
            // Markdown metacharacters by construction.
            out.push_str(&format!(
                "[{}](charts/{stem}.svg)\n",
                chart_caption(stem, &doc.scan.languages)
            ));
            any_overview = true;
        }
        if !any_overview && component_stems.is_empty() {
            out.push_str("No charts were emitted for this scan.\n\n");
        } else {
            out.push('\n');
        }
    }

    if let Some(components) = &doc.components
        && !components.is_empty()
    {
        // Human-scale bound (C-full-split follow-up): top-10 in document
        // (rank) order with an honest `showing top … of M` header — same
        // take(10) precedent as §3. Full component data stays in
        // metrics.json; the detail-page hook below links only shown rows.
        let total = components.len();
        let shown = total.min(HUMAN_CAP);
        out.push_str(&format!(
            "Component ranking (hottest first — showing top {shown} of {total} components):\n\n"
        ));
        out.push_str("| component | path prefix | rank | finding density (per KLOC) |\n");
        out.push_str("|---|---|---:|---:|\n");
        // Document order IS the ranked emission order (rank, path_prefix)
        // per the aggregator contract — preserved verbatim, no re-sort.
        for c in components.iter().take(HUMAN_CAP) {
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

        // Per-component detail index hook (output-contract §5): one link per
        // SHOWN (top-10 capped, same order as the table above) component
        // whose page the charts module emitted, in document (rank) order.
        // Filenames are reconstructed via the charts-owned pinning function
        // — never guessed — and linked ONLY when present in
        // `emitted_charts`.
        let mut linked: BTreeSet<String> = BTreeSet::new();
        let mut hook: Vec<String> = Vec::new();
        for c in components.iter().take(HUMAN_CAP) {
            let filename = crate::charts::component::component_filename(c.rank, &c.path_prefix);
            let stem = filename
                .strip_suffix(".svg")
                .unwrap_or(&filename)
                .to_string();
            if emitted_stems.contains(&stem) {
                linked.insert(stem);
                hook.push(format!(
                    "[Component {} — per-file hot-path map.](charts/{filename})\n",
                    escape_md(&c.path_prefix),
                ));
            }
        }
        if !hook.is_empty() {
            out.push_str("Component detail pages (one per component above):\n\n");
            for line in hook {
                out.push_str(&line);
            }
            out.push('\n');
        }
        // Orphan fallback: an emitted registry detail page with no matching
        // hook entry (unreachable via the pipeline — defensive only for
        // direct `render_report` callers) still links via the detail caption.
        for stem in &component_stems {
            if !linked.contains(stem) {
                out.push_str(&format!("[{}](charts/{stem}.svg)\n", detail_caption()));
            }
        }
        if !component_stems.is_empty() && linked.len() != component_stems.len() {
            out.push('\n');
        }
    } else {
        // No component table to hook under: without components the detail
        // registry is empty, so every emitted stem took the overview path
        // above (generic caption for unknown names). This loop only fires
        // for registry detail stems, which require components — kept for
        // shape symmetry with the branch above.
        for stem in &component_stems {
            out.push_str(&format!("[{}](charts/{stem}.svg)\n", detail_caption()));
        }
        if !component_stems.is_empty() {
            out.push('\n');
        }
    }

    out.push_str(&dependency_paragraph(&doc.scan.languages, escape_md));
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
    if semantic_tier_active(doc) {
        out.push_str(
            "Standing caveat: resolved cross-file call edges are MEDIUM \
             confidence; name-based edges the semantic tier could not resolve \
             remain LOW (ADR-0001).\n\n",
        );
    } else {
        out.push_str(
            "Standing caveat: cross-file call edges are LOW confidence until a \
             semantic resolver tier exists (ADR-0001).\n\n",
        );
    }
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

fn render_appendix_b(out: &mut String, semantic: bool) {
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
    if semantic {
        out.push_str(
            "Every finding is an **optimization candidate, never a certainty**: a \
             candidate for human review, not a defect claim. Perfpal ran with the \
             semantic resolver tier active for C — `resolver_tier: semantic` marks \
             findings upgraded by a libclang fact (resolved cross-file edges are \
             MEDIUM confidence); findings without a resolved fact stay syntactic. \
             Macro expansion and full type resolution remain out of scope.\n",
        );
    } else {
        out.push_str(
            "Every finding is an **optimization candidate, never a certainty**: a \
             candidate for human review, not a defect claim. Perfpal v1 runs on a \
             purely syntactic tier (`resolver_tier: syntactic`) — no macro \
             expansion, no type resolution.\n",
        );
    }
    out.push_str(&format!(
        "For programmatic consumption, read `metrics.json` — the source of \
         truth — and validate it against the bundled schema \
         `{}` shipped alongside it. Where this report and \
         `metrics.json` ever disagree, `metrics.json` wins.\n\n",
        crate::schema::SCHEMA_ARTIFACT_NAME
    ));
}

/// Appendix C (C-split): link stub ONLY — the UNCAPPED per-finding detail
/// lives in the `findings-detail.md` artifact
/// ([`render_findings_detail`] → [`render_findings_full_detail`]). The
/// honest header ties the count back to §4 (`N of M total` — both sides
/// always agree today) so a future row cap cannot silently drop rows
/// without breaking the header. `report.md` carries zero per-finding rows
/// by construction (locked by `tests/findings_detail_split.rs`).
fn render_appendix_c(out: &mut String, doc: &MetricsDocument) {
    out.push_str("## Appendix C. All findings (full detail)\n");
    if doc.findings.is_empty() {
        out.push_str("No findings.\n\n");
        return;
    }
    let groups = group_findings_by_rule(doc);
    out.push_str(&format!(
        "{} of {} total findings.\n\nFull per-finding detail has moved to \
         [findings-detail.md](findings-detail.md) ({} rules, same {} of {} total).\n\n",
        doc.findings.len(),
        doc.findings.len(),
        groups.len(),
        doc.findings.len(),
        doc.findings.len(),
    ));
}

// ---------------------------------------------------------------------------
// HTML renderer helpers (Option B slice 2: LINKED variant + fixed script)
// ---------------------------------------------------------------------------

/// Emit the document head: doctype, charset, title, and a minimal inline
/// `<style>` block (static text — no external resources; the only script is
/// the fixed [`REPORT_SCRIPT`] block appended in `html_footer`).
/// Scoped key-view CSS (map-intelligibility V4). Shared by the report page
/// and the detail/profile pages. No `text-overflow: ellipsis` anywhere on
/// key-view text: labels wrap instead of truncating.
const KEY_VIEW_CSS: &str = "\
         .kv{margin:1.25rem 0 1.75rem}\n\
         .kv h3{margin:.25rem 0 .35rem;font-size:1.05rem}\n\
         .kv-action{margin:.15rem 0 .75rem;color:#444}\n\
         .kv-rows{list-style:none;margin:0;padding:0;border-bottom:1px solid #999}\n\
         .kv-row{display:grid;grid-template-columns:3.5rem 1fr;gap:.75rem;padding:.55rem 0;border-top:1px solid #ccc}\n\
         .kv-rank{font-size:1.5rem;font-weight:700;line-height:1}\n\
         .kv-line1{display:flex;flex-wrap:wrap;align-items:baseline;gap:.4rem}\n\
         .kv-loc,.kv-fn{font-family:monospace,monospace;overflow-wrap:anywhere;word-break:break-word;white-space:normal}\n\
         .kv-loc{font-weight:700;color:#111;text-decoration:none}\n\
         .kv-loc:hover{text-decoration:underline}\n\
         .kv-title{font-weight:600}\n\
         .kv-key{margin:.1rem 0 .6rem;color:#333;font-size:.85rem}\n\
         .kv-note{margin-left:.4rem;font-size:.78rem;color:#4a5568}\n\
         .kv-caption{margin:.1rem 0 .6rem;color:#333;font-size:.85rem}\n\
         .kv-badge,.kv-class,.kv-dem{display:inline-block;font-size:.72rem;font-weight:700;text-transform:uppercase;letter-spacing:.03em;padding:0 .3rem;border:1px solid #999;border-radius:.2rem;white-space:normal;overflow-wrap:anywhere}\n\
         .kv-class.nonprod{background:#f3e8ff;border-color:#9f7aea}\n\
         .sev-hot{background:#fff5f5;border-color:#e53e3e;color:#9b2c2c}\n\
         .sev-warn{background:#fffaf0;border-color:#dd6b20;color:#975a16}\n\
         .sev-info{background:#f7fafc;border-color:#a0aec0;color:#2d3748}\n\
         .kv-dem{background:#edf2f7;border-color:#a0aec0;color:#2d3748;margin-left:.35rem}\n\
         .kv-anchor,.kv-value{font-family:monospace,monospace;font-size:.8rem;color:#333;overflow-wrap:anywhere;white-space:normal;line-height:1.35}\n\
         .kv-why{margin-top:.2rem;overflow-wrap:anywhere;white-space:normal}\n\
         .kv-counts{margin:.45rem 0 0;color:#444;font-size:.85rem}\n\
         .kv-more{margin:.5rem 0 .2rem;font-weight:600}\n\
         .kv-bars{max-width:100%}\n\
         .kv-brow{display:grid;grid-template-columns:minmax(14rem,26rem) minmax(4rem,1fr) minmax(12rem,24rem);gap:.7rem;align-items:center;padding:.35rem 0;border-top:1px solid #ccc}\n\
         .kv-brow:last-child{border-bottom:1px solid #999}\n\
         .kv-blabel{overflow-wrap:anywhere;word-break:break-word;white-space:normal;min-width:0}\n\
         .kv-track{background:#edf2f7;border-radius:.15rem;height:1.1rem;width:100%;max-width:100%}\n\
         .kv-fill{background:#2b6cb0;height:1.1rem;border-radius:.15rem;min-width:0;max-width:100%}\n\
         .kv-fill.demoted{background:#a0aec0}\n\
         .kv-demoted .kv-blabel,.kv-demoted .kv-value{opacity:.6}\n\
         .kv-badge.trip{background:#fffaf0;border-color:#dd6b20;color:#975a16;margin-top:.2rem}\n\
         .kv-badge.known{background:#f0fff4;border-color:#38a169;color:#22543d;margin-top:.2rem}\n\
         @media print{.kv-row,.kv-brow{break-inside:avoid}}\n";

fn html_head(out: &mut String, doc: &MetricsDocument) {
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    // Inline empty data-URI favicon: kills the /favicon.ico 404 console error
    // with zero bytes fetched (no network, fully offline).
    out.push_str("<link rel=\"icon\" href=\"data:,\">\n");
    out.push_str(&format!(
        "<title>Perfpal Report — schema {}</title>\n",
        escape_html(&doc.schema_version),
    ));
    out.push_str(
        "<style>\n\
         body{font-family:monospace,monospace;max-width:90rem;margin:2rem auto;padding:0 1rem;color:#111;background:#fff}\n\
         .table-scroll{overflow-x:auto;max-width:100%}\n\
         table{border-collapse:collapse;width:100%;margin:1rem 0}\n\
         th,td{border:1px solid #999;padding:.25rem .5rem;text-align:left;vertical-align:top}\n\
         th{background:#eee}\n\
         #findings table{min-width:72rem}\n\
         #findings tbody td{white-space:nowrap}\n\
         #findings td.wrap{white-space:normal;overflow-wrap:anywhere;min-width:22rem;max-width:38rem}\n\
         figure{border:1px solid #999;padding:.5rem;margin:1rem 0;max-width:100%;overflow-x:auto}\n\
         figcaption{margin-top:.5rem}\n\
         img{max-width:100%;height:auto}\n\
         figure.pz img{max-width:none;touch-action:none}\n\
         #files tbody tr[data-path],#components tbody tr[data-prefix]{cursor:pointer}\n\
         #filter-bar{margin:.5rem 0}\n\
         nav ul{list-style:none;padding:0;display:flex;gap:1rem;flex-wrap:wrap}\n\
         #findings summary{cursor:pointer;list-style:none}\n\
         #findings summary::-webkit-details-marker{display:none}\n\
         #findings summary::before{content:\"▸\";display:inline-block;margin-right:.35em}\n\
         #findings details[open]>summary::before{transform:rotate(90deg)}\n",
    );
    out.push_str(KEY_VIEW_CSS);
    out.push_str("         </style>\n</head>\n<body>\n");
}

/// Header: schema version + tool version + file/finding counts ONLY — never
/// `started_at`, `duration_ms`, or `root_path` (no provenance surfaces, so
/// double-scan output is byte-identical UNMASKED).
fn html_header(out: &mut String, doc: &MetricsDocument, breadcrumb: bool) {
    let hot = findings_of_severity(doc, Severity::Hot).len();
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
    out.push_str("<header>\n<h1>Perfpal Report</h1>\n");
    if breadcrumb {
        html_breadcrumb(out);
    }
    out.push_str("<p>");
    out.push_str(&format!(
        "schema_version {} · perfpal {} {} · {} files · {} findings ({} hot, {} warn, {} info)",
        escape_html(&doc.schema_version),
        escape_html(&doc.tool.name),
        escape_html(&doc.tool.version),
        doc.scan.file_count,
        doc.findings.len(),
        hot,
        warn_count,
        info_count,
    ));
    if !doc.scan.languages.is_empty() {
        // Sorted-unique by aggregator contract; re-sorted here so ordering
        // depends only on document content.
        let mut languages = doc.scan.languages.clone();
        languages.sort();
        languages.dedup();
        out.push_str(&format!(
            " · languages: {}",
            escape_html(&languages.join(", "))
        ));
    }
    out.push_str("</p>\n</header>\n");
}

/// Report-only breadcrumb line (after `<h1>`): the overview → drill path.
/// Every label is escaped at the interpolation site (static text today, but
/// the escaper keeps the contract uniform); no JavaScript.
fn html_breadcrumb(out: &mut String) {
    out.push_str(&format!(
        "<p class=\"breadcrumb\"><a href=\"#map\">{}</a> › <a href=\"#components\">{}</a> › <a href=\"#files\">{}</a> › <a href=\"findings-detail.html\">{}</a></p>\n",
        escape_html("Atlas"),
        escape_html("Components"),
        escape_html("Files"),
        escape_html("Findings detail"),
    ));
}

fn html_nav(out: &mut String) {
    out.push_str(
        "<nav>\n<ul>\n\
         <li><a href=\"#map\">Atlas</a></li>\n\
         <li><a href=\"#explorer\">Explorer</a></li>\n\
         <li><a href=\"#components\">Components</a></li>\n\
         <li><a href=\"#files\">Files</a></li>\n\
         <li><a href=\"#findings\">Findings</a></li>\n\
         <li><a href=\"findings-detail.html\">Findings detail</a></li>\n\
         <li><a href=\"#appendix-a\">Heuristic reference</a></li>\n\
         </ul>\n</nav>\n",
    );
}

/// Detail-page nav: link back to the human page plus in-page anchors.
fn html_detail_nav(out: &mut String) {
    out.push_str(
        "<nav>\n<ul>\n\
         <li><a href=\"report.html\">Report</a></li>\n\
         <li><a href=\"#findings\">Findings detail</a></li>\n\
         </ul>\n</nav>\n",
    );
}

/// Fixed inline enhancement script (Option B slice 2): hand-rolled
/// pan/zoom for `figure.pz` chart figures (drag + wheel + buttons via CSS
/// transforms over the existing linked `<img>` — the SVGs are never inlined
/// or duplicated) and click-to-filter for the findings table (pure DOM hide
/// via `hidden` over already-rendered rows, no re-fetch, no network).
///
/// Byte-determinism: this source is a FIXED string constant — no timestamps,
/// ids, or counters in the generated code. Every interpolated document value
/// is escaped at RENDER time via [`escape_html`]; the script itself only
/// reads `data-*` attributes and writes `textContent`/`hidden`/style (never
/// `innerHTML`/`outerHTML`, `insertAdjacentHTML`, `document.write`, `eval` /
/// `Function(`, string-arg timers, `fetch`/`XHR`, `Date`, or `random`). Row
/// click-to-filter ignores clicks originating inside an inner `a[href]`
/// (bubbling guard — following a drill-down/detail link never also filters).
/// All handlers bind via `addEventListener` (no inline `on*=` attributes),
/// and the page is fully usable with JS disabled (all rows present; the
/// script only enhances).
const REPORT_SCRIPT: &str = r##"(function(){
"use strict";
function apply(st,img){img.style.transform="translate("+st.x+"px, "+st.y+"px) scale("+st.k+")";}
function zoom(st,img,f){st.k=Math.min(8,Math.max(1,st.k*f));if(st.k===1){st.x=0;st.y=0;}apply(st,img);}
function btn(label,name,fn){var b=document.createElement("button");b.textContent=label;b.setAttribute("type","button");b.setAttribute("aria-label",name);b.addEventListener("click",fn);return b;}
function enhance(fig){
var img=fig.querySelector("img");if(!img){return;}
var st={x:0,y:0,k:1};fig.style.overflow="hidden";img.style.cursor="grab";img.setAttribute("draggable","false");
var drag=null;
img.addEventListener("pointerdown",function(e){drag={a:e.clientX-st.x,b:e.clientY-st.y};img.setPointerCapture(e.pointerId);});
img.addEventListener("pointermove",function(e){if(!drag){return;}st.x=e.clientX-drag.a;st.y=e.clientY-drag.b;apply(st,img);});
img.addEventListener("pointerup",function(){drag=null;});
img.addEventListener("wheel",function(e){e.preventDefault();zoom(st,img,e.deltaY<0?1.15:0.87);},{passive:false});
var bar=document.createElement("div");
bar.appendChild(btn("+","zoom in",function(){zoom(st,img,1.25);}));
bar.appendChild(btn("-","zoom out",function(){zoom(st,img,0.8);}));
bar.appendChild(btn("reset","reset view",function(){st.x=0;st.y=0;st.k=1;apply(st,img);}));
fig.appendChild(bar);
}
function boot(){
var figs=document.querySelectorAll("figure.pz");for(var i=0;i<figs.length;i++){enhance(figs[i]);}
var rows=Array.prototype.slice.call(document.querySelectorAll("#findings tbody tr"));
var label=document.getElementById("filter-label");var clear=document.getElementById("filter-clear");
if(!label||!clear||rows.length===0){return;}
function reset(){for(var i=0;i<rows.length;i++){rows[i].hidden=false;}label.textContent="all findings";clear.hidden=true;}
function show(kind,value,match){var n=0;for(var i=0;i<rows.length;i++){var ok=match(rows[i]);rows[i].hidden=!ok;if(ok){n++;}}label.textContent="filter: "+kind+" "+value+" ("+n+" of "+rows.length+")";clear.hidden=false;}
clear.addEventListener("click",reset);
function wire(list,kind,match){for(var i=0;i<list.length;i++){(function(tr){tr.addEventListener("click",function(e){if(e&&e.target&&e.target.closest&&e.target.closest("a[href]")){return;}var v=tr.getAttribute(kind)||"";show(kind==="data-path"?"file":"component",v,function(r){return match(r,v);});});})(list[i]);}}
wire(document.querySelectorAll("#files tbody tr[data-path]"),"data-path",function(r,v){return r.getAttribute("data-path")===v;});
wire(document.querySelectorAll("#components tbody tr[data-prefix]"),"data-prefix",function(r,v){var p=r.getAttribute("data-path")||"";return p===v||v==="."||p.indexOf(v+"/")===0;});
}
if(document.readyState==="loading"){document.addEventListener("DOMContentLoaded",boot);}else{boot();}
})"##;

/// Byte-exact accessor for the fixed enhancement script (slice-2 golden and
/// byte-identity tests compare the emitted `<script>` block against this).
#[must_use]
pub fn report_script() -> &'static str {
    REPORT_SCRIPT
}

/// Known overview chart stems (ADR-0002 chart-set-v1 + Option A slice 2
/// layers). Overview links are allowlisted to this set: an emitted stem that
/// is neither a known overview nor in the exact per-component detail registry
/// is NEVER turned into an `<a href>`/`<img src>` link (unknown stems must
/// not become links — a hostile `emitted_charts` entry cannot break out of
/// an attribute it never enters). Every stem that DOES link is still passed
/// through [`escape_html`] at the interpolation site.
const KNOWN_OVERVIEW_STEMS: &[&str] = &[
    "include-graph",
    "component-coupling",
    "hot-components",
    "layer-workspace",
    "layer-file",
    "layer-entry",
    "layer-bands",
    "layer-findings",
];

/// Split `emitted_charts` into overview stems (emission order, linked as
/// figures) versus per-component detail stems (exact registry, like §5).
/// Returns `(overviews, details)`; both derived from document content +
/// emission order only.
///
/// Allowlist gate (FIX 2): overviews keep ONLY [`KNOWN_OVERVIEW_STEMS`]
/// entries — unknown stems (including hostile `"><img …>` payloads) are
/// dropped before they can reach any `href`/`src` interpolation. Detail
/// stems still route by the exact registry.
fn html_chart_split(
    doc: &MetricsDocument,
    emitted_charts: &[String],
) -> (Vec<String>, Vec<String>) {
    let detail_stems: BTreeSet<String> = doc
        .components
        .iter()
        .flatten()
        .map(|c| {
            let filename = crate::charts::component::component_filename(c.rank, &c.path_prefix);
            filename
                .strip_suffix(".svg")
                .unwrap_or(&filename)
                .to_string()
        })
        .collect();
    let mut overviews: Vec<String> = Vec::new();
    let mut details: Vec<String> = Vec::new();
    for name in emitted_charts {
        let stem = name.strip_suffix(".svg").unwrap_or(name).to_string();
        if detail_stems.contains(&stem) {
            if !details.iter().any(|s| s == &stem) {
                details.push(stem);
            }
        } else if KNOWN_OVERVIEW_STEMS.contains(&stem.as_str())
            && !overviews.iter().any(|s| s == &stem)
        {
            overviews.push(stem);
        }
    }
    (overviews, details)
}

/// Explorer section: the §5 chart links mirrored as LINKED figures
/// (`<a href>` + `<img src>`, relative, offline) plus the standing caveat
/// notes (static text) and the DOT export link.
fn html_architecture(out: &mut String, doc: &MetricsDocument, emitted_charts: &[String]) {
    out.push_str("<section id=\"explorer\">\n<h2>Explorer</h2>\n");
    let (overviews, details) = html_chart_split(doc, emitted_charts);
    if overviews.is_empty() && details.is_empty() {
        out.push_str("<p>No charts were emitted for this scan.</p>\n");
    } else {
        for stem in &overviews {
            // Stems reaching this point are allowlisted (known overviews)
            // yet still escaped at the interpolation site — belt and
            // suspenders. Captions are reporter-owned static text, except the
            // include-graph arm which reflects the scanned languages.
            let caption = chart_caption(stem, &doc.scan.languages);
            push_chart_figure(out, stem, &caption);
        }
        if !details.is_empty() {
            out.push_str("<p>Component detail pages (one per component below):</p>\n<ul>\n");
            for stem in &details {
                // Registry-derived (charts-owned filenames), escaped anyway.
                let safe_stem = escape_html(stem);
                out.push_str(&format!(
                    "<li><a href=\"charts/{stem}.svg\">{caption}</a></li>\n",
                    stem = safe_stem,
                    caption = escape_html(&detail_caption()),
                ));
            }
            out.push_str("</ul>\n");
        }
    }
    // Known-language fragments are pinned static text (safe verbatim in
    // HTML); unknown language names are escaped inside `dependency_clause`.
    out.push_str(&format!(
        "<p>{}</p>\n",
        dependency_paragraph(&doc.scan.languages, escape_html)
    ));
    out.push_str(
        "<p>Machine-readable graph: <a href=\"charts/graph.dot\">graph.dot</a> — deterministic \
         DOT export of the include/call/component graphs (same source data as \
         the figures above).</p>\n",
    );
    if semantic_tier_active(doc) {
        out.push_str(
            "<p>Semantic resolver tier active for C — resolved cross-file call \
             edges are MEDIUM confidence; name-based edges the semantic tier \
             could not resolve remain LOW (ADR-0001).</p>\n",
        );
    } else {
        out.push_str(
            "<p>Standing caveat: cross-file call edges are LOW confidence until a \
             semantic resolver tier exists (ADR-0001).</p>\n",
        );
    }
    out.push_str("</section>\n");
}

/// Shared linked-chart figure (used by the Explorer section): one
/// `figure.pz` hooking the fixed pan/zoom enhancement,
/// a relative `<a href>`/`<img src>` pair, and an escaped caption.
fn push_chart_figure(out: &mut String, stem: &str, caption: &str) {
    let safe_stem = escape_html(stem);
    out.push_str(&format!(
        "<figure class=\"pz\" data-chart=\"{stem}\">\n<a href=\"charts/{stem}.svg\"><img src=\"charts/{stem}.svg\" alt=\"{caption}\"></a>\n<figcaption>{caption}</figcaption>\n</figure>\n",
        stem = safe_stem,
        caption = escape_html(caption),
    ));
}

/// `#map` — the Perfpal Atlas page (ADR-0009; `map-intelligibility` A8
/// amended for this page).
///
/// The first content block is the 100%-graph canvas; the fixed seven-block
/// brief follows it, then the Q2/weight drills. Every block renders in every
/// scan state. The legacy V4 KV1/KV2/KV3 list-first markup no longer lives
/// here: KV1 actions, the weight mass and the blast radius ride in the brief,
/// and the complete lists stay in `findings-detail.html` / `profile-queue.html`.
fn html_map(out: &mut String, doc: &MetricsDocument, emitted_charts: &[String]) {
    let view = atlas::build_atlas_view(doc);
    let start_actions = atlas_brief::start_actions(doc);
    out.push_str("<section id=\"map\">\n");
    out.push_str(&atlas_render::render_canvas_html_with_actions(
        &view,
        &start_actions,
    ));
    out.push_str(&atlas_brief::render_brief(&view, doc, emitted_charts));
    out.push_str("</section>\n");
}

/// JSON companion of the `#map` canvas (Atlas Story S1): the exact story geometry
/// the SVG draws, reusing the same view, start actions and findings counts as
/// [`html_map`]. Only emitted when `--story` is passed.
#[must_use]
pub fn atlas_layout_json(doc: &MetricsDocument) -> String {
    let view = atlas::build_atlas_view(doc);
    let start_actions = atlas_brief::start_actions(doc);
    // The story covers only the drawn file slice: findings_included is the sum
    // of the per-node counts (many findings live on undrawn files), while
    // findings_total stays the full scan total.
    let findings_included = view
        .nodes
        .iter()
        .map(|node| node.findings.hot + node.findings.warn + node.findings.info)
        .sum();
    let findings_total = doc.findings.len();
    atlas_layout::to_json(&view, &start_actions, findings_included, findings_total)
}

/// Component index: rank, prefix, metrics + link to the
/// `charts/component-<rank>-<slug>.svg` detail page when emitted.
/// Document (rank) order preserved verbatim, no re-sort beyond the pinned
/// `(rank, path_prefix)` defensive sort. Human-scale bound (C-full-split
/// follow-up): top-10 rows with an honest `showing top … of M` note — same
/// take(10) precedent as the `#files` drill-down; full component data stays
/// in `metrics.json`.
fn html_components(out: &mut String, doc: &MetricsDocument, emitted_charts: &[String]) {
    out.push_str("<section id=\"components\">\n<h2>Component index</h2>\n");
    let Some(components) = &doc.components else {
        out.push_str("<p>No components recorded.</p>\n</section>\n");
        return;
    };
    if components.is_empty() {
        out.push_str("<p>No components recorded.</p>\n</section>\n");
        return;
    }
    let total = components.len();
    let shown = total.min(HUMAN_CAP);
    out.push_str(&format!(
        "<p>showing top {shown} of {total} components</p>\n"
    ));
    let emitted_stems: BTreeSet<String> = emitted_charts
        .iter()
        .map(|name| name.strip_suffix(".svg").unwrap_or(name).to_string())
        .collect();
    let mut sorted: Vec<&crate::schema::ComponentEntry> = components.iter().collect();
    sorted.sort_by(|a, b| a.rank.cmp(&b.rank).then(a.path_prefix.cmp(&b.path_prefix)));
    out.push_str(
        "<div class=\"table-scroll\">\n<table>\n<thead><tr><th>rank</th><th>prefix</th>\
         <th>finding density (per KLOC)</th><th>max loop depth</th>\
         <th>finding count</th><th>detail</th></tr></thead>\n<tbody>\n",
    );
    for c in sorted.into_iter().take(HUMAN_CAP) {
        let filename = crate::charts::component::component_filename(c.rank, &c.path_prefix);
        let stem = filename.strip_suffix(".svg").unwrap_or(&filename);
        let anchor = escape_html(stem);
        let detail = if emitted_stems.contains(stem) {
            // Charts-owned pinned filename — escaped at interpolation anyway.
            format!(
                "<a href=\"charts/{filename}\">detail</a>",
                filename = escape_html(&filename)
            )
        } else {
            "n/a".to_string()
        };
        let finding_count = match c.finding_count {
            Some(n) => n.to_string(),
            None => "n/a".to_string(),
        };
        out.push_str(&format!(
            "<tr id=\"{anchor}\" data-prefix=\"{prefix_attr}\"><td>{rank}</td><td>{prefix}</td><td>{density}</td>\
             <td>{depth}</td><td>{count}</td><td>{detail}</td></tr>\n",
            anchor = anchor,
            prefix_attr = escape_html(&c.path_prefix),
            rank = c.rank,
            prefix = escape_html(&c.path_prefix),
            density = escape_html(&format_fixed_point(c.metrics.finding_density, 2)),
            depth = c.metrics.max_loop_depth,
            count = finding_count,
            detail = detail,
        ));
    }
    out.push_str("</tbody>\n</table>\n</div>\n</section>\n");
}

/// File drill-down: top files by (degree desc, cost desc, path asc) with
/// their finding_id lists linking into the findings table. Each file's
/// link list is capped at the top 10 by severity-rank → finding_id (with
/// an honest `…+N more` marker); the full per-file lists stay in
/// `metrics.json`.
fn html_files(out: &mut String, doc: &MetricsDocument, emitted_charts: &[String]) {
    out.push_str("<section id=\"files\">\n<h2>File drill-down</h2>\n");
    if doc.files.is_empty() {
        out.push_str("<p>No files scanned.</p>\n</section>\n");
        return;
    }
    // Per-file cost lookup (BTreeMap ⇒ deterministic) from the shared heat
    // selector: v2 when present, v1 fallback (I2, same selector as charts).
    let mut file_cost_sum: std::collections::BTreeMap<&str, f64> =
        std::collections::BTreeMap::new();
    for f in &doc.stats.functions {
        *file_cost_sum.entry(f.path.as_str()).or_insert(0.0) +=
            crate::charts::function_heat_cost(f);
    }
    let mut ranked: Vec<(&str, usize, f64)> = doc
        .files
        .iter()
        .map(|f| {
            (
                f.path.as_str(),
                file_degree(&f.path, doc),
                file_cost_sum.get(f.path.as_str()).copied().unwrap_or(0.0),
            )
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal))
            .then(a.0.cmp(b.0))
    });
    // Findings grouped by path: index-keyed (into `doc.findings`), each
    // group sorted by severity-rank ascending then finding_id ascending
    // (total order, deterministic) so the per-file top-10 cap below keeps
    // the hottest findings.
    let mut by_path: std::collections::BTreeMap<&str, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, f) in doc.findings.iter().enumerate() {
        by_path.entry(f.path.as_str()).or_default().push(i);
    }
    for list in by_path.values_mut() {
        list.sort_by(|&a, &b| {
            severity_rank(doc.findings[a].severity)
                .cmp(&severity_rank(doc.findings[b].severity))
                .then_with(|| doc.findings[a].finding_id.cmp(&doc.findings[b].finding_id))
        });
    }
    out.push_str(
        "<div class=\"table-scroll\">\n<table>\n<thead><tr><th>file</th><th>degree</th><th>cost</th>\
         <th>findings</th><th>component</th></tr></thead>\n<tbody>\n",
    );
    // Component-page membership for the per-file drill link: exact
    // `parent_dir` prefix → pinned detail stem, restricted to emitted pages.
    let emitted_stems: BTreeSet<String> = emitted_charts
        .iter()
        .map(|name| name.strip_suffix(".svg").unwrap_or(name).to_string())
        .collect();
    let component_stems: BTreeMap<&str, String> = doc
        .components
        .iter()
        .flatten()
        .map(|c| {
            let filename = crate::charts::component::component_filename(c.rank, &c.path_prefix);
            let stem = filename
                .strip_suffix(".svg")
                .unwrap_or(&filename)
                .to_string();
            (c.path_prefix.as_str(), stem)
        })
        .collect();
    // Collision-free anchors (FIX 1, corrected): anchors are pre-assigned
    // per finding OCCURRENCE in finding_id order (index-keyed map), and BOTH
    // the drill-down below and the findings table look up from that map —
    // render order (degree/cost/path here) never decides which row owns the
    // bare slug, so a slug-colliding pair links row↔row correctly no matter
    // which file ranks first.
    let file_anchors = file_anchor_map(doc);
    let (_, finding_anchors) = finding_anchor_assignments(doc);
    for (path, degree, cost) in ranked.iter().take(10) {
        let anchor = escape_html(
            file_anchors
                .get(*path)
                .unwrap_or(&format!("file-{}", slugify(path))),
        );
        let finding_cells = match by_path.get(*path) {
            Some(list) if !list.is_empty() => {
                // C-split: per-finding rows live in findings-detail.html, so
                // drill-down links point at the detail page anchors (same
                // anchor map as the detail page — every href resolves).
                // Human-scale bound (C-full-split follow-up): top-10 links
                // per file (severity-rank order, assigned above) with an
                // honest `…+N more` marker reusing the slice-1 pattern —
                // the full per-file list stays in metrics.json.
                let links: Vec<String> = list
                    .iter()
                    .take(HUMAN_CAP)
                    .map(|&i| {
                        let f = &doc.findings[i];
                        let unique = finding_anchors[i].clone();
                        format!(
                            "<a href=\"findings-detail.html#{}\">{}</a>",
                            escape_html(&unique),
                            escape_html(&f.finding_id),
                        )
                    })
                    .collect();
                let mut cell = links.join("<br>\n");
                if list.len() > HUMAN_CAP {
                    cell.push_str(&format!("<br>\n…+{} more", list.len() - HUMAN_CAP));
                }
                cell
            }
            _ => "none".to_string(),
        };
        let component_cell = component_stems
            .get(crate::charts::parent_dir(path))
            .filter(|stem| emitted_stems.contains(stem.as_str()))
            .map_or_else(
                || "n/a".to_string(),
                |stem| format!("<a href=\"charts/{}.svg\">component</a>", escape_html(stem)),
            );
        out.push_str(&format!(
            "<tr id=\"{anchor}\" data-path=\"{data_path}\"><td>{path}</td><td>{degree}</td><td>{cost}</td><td>{links}</td><td>{component_cell}</td></tr>\n",
            anchor = anchor,
            data_path = escape_html(path),
            path = escape_html(path),
            degree = degree,
            cost = escape_html(&format_fixed_point(*cost, 0)),
            links = finding_cells,
        ));
    }
    if ranked.len() > 10 {
        out.push_str(&format!(
            "<p>…+{} more (see findings table)</p>\n",
            ranked.len() - 10
        ));
    }
    out.push_str("</tbody>\n</table>\n</div>\n</section>\n");
}

/// Findings section (C-split): per-rule `<details><summary>` groups ONLY —
/// zero per-finding rows in `report.html` by construction. The summary
/// carries rule + count + severity split + top-exemplar link into the
/// `findings-detail.html` artifact, plus a per-group "full detail" link.
/// Group/summary order is the pinned [`group_findings_by_rule`] order.
/// Finding-row anchors come from the SAME [`finding_anchor_assignments`]
/// map the detail page uses, so every exemplar href resolves there.
fn html_findings(out: &mut String, doc: &MetricsDocument) {
    out.push_str("<section id=\"findings\">\n<h2>Findings</h2>\n");
    out.push_str(
        "<div id=\"filter-bar\"><span id=\"filter-label\">all findings</span> \
         <button id=\"filter-clear\" type=\"button\" hidden>clear</button></div>\n",
    );
    if doc.findings.is_empty() {
        out.push_str("<p>No findings.</p>\n</section>\n");
        return;
    }
    // Honest header (mirrors the §4 "R rules: N of M total" line).
    let groups = group_findings_by_rule(doc);
    let (hot_count, warn_count, info_count) = severity_counts(doc);
    out.push_str(&format!(
        "<p>{} rules: {} of {} total findings ({} hot, {} warn, {} info).</p>\n",
        groups.len(),
        doc.findings.len(),
        doc.findings.len(),
        hot_count,
        warn_count,
        info_count,
    ));
    out.push_str("<p>Full per-finding tables: <a href=\"findings-detail.html\">findings-detail.html</a>.</p>\n");
    // `anchors[i]` is the collision-free anchor for `doc.findings[i]`
    // (finding_id-order assignment shared with the file drill-down AND the
    // detail page).
    let (_, anchors) = finding_anchor_assignments(doc);
    for group in &groups {
        // Exemplar: first member in finding_id order (deterministic), linked
        // to its DETAIL-page row anchor — every exemplar href resolves by
        // construction (locked by `tests/findings_detail_split.rs`).
        let exemplar = &doc.findings[group.members[0]];
        let exemplar_anchor = &anchors[group.members[0]];
        let group_anchor = format!("rule-{}", group.rule_id);
        out.push_str(&format!(
            "<details>\n<summary><code>{rule}</code> — {n} findings ({h} hot, {w} warn, {i} info); \
             top exemplar: <a href=\"findings-detail.html#{anchor}\">{id}</a> — \
             <a href=\"findings-detail.html#{group}\">full detail</a></summary>\n\
             <p><a href=\"findings-detail.html#{group}\">View full detail table for {rule} ({n} findings)</a></p>\n\
             </details>\n",
            rule = escape_html(group.rule_id),
            n = group.members.len(),
            h = group.hot,
            w = group.warn,
            i = group.info,
            anchor = escape_html(exemplar_anchor),
            id = escape_html(&exemplar.finding_id),
            group = escape_html(&group_anchor),
        ));
    }
    out.push_str("</section>\n");
}

/// Detail-page findings: per-rule sections in the pinned
/// [`group_findings_by_rule`] order, each with a `rule-<rule_id>` anchor
/// plus that rule's rows in the shared [`html_findings_table`] markup
/// (promoted VERBATIM — this is the C-split seam). Finding-row anchors come
/// from the SAME [`finding_anchor_assignments`] map as `report.html`, so
/// every `findings-detail.html#…` link resolves here. The `#findings`
/// section id + `filter-bar` controls are reused verbatim so the fixed
/// [`REPORT_SCRIPT`] click-to-filter keeps working on this page.
fn html_detail_findings(out: &mut String, doc: &MetricsDocument) {
    out.push_str("<section id=\"findings\">\n<h2>Findings detail</h2>\n");
    out.push_str(
        "<div id=\"filter-bar\"><span id=\"filter-label\">all findings</span> \
         <button id=\"filter-clear\" type=\"button\" hidden>clear</button></div>\n",
    );
    if doc.findings.is_empty() {
        out.push_str("<p>No findings.</p>\n</section>\n");
        return;
    }
    let groups = group_findings_by_rule(doc);
    let (hot_count, warn_count, info_count) = severity_counts(doc);
    out.push_str(&format!(
        "<p>{} rules: {} of {} total findings ({} hot, {} warn, {} info).</p>\n",
        groups.len(),
        doc.findings.len(),
        doc.findings.len(),
        hot_count,
        warn_count,
        info_count,
    ));
    out.push_str("<p><a href=\"report.html\">Back to report.html</a>.</p>\n");
    let (_, anchors) = finding_anchor_assignments(doc);
    html_ranked_actions(out, doc, &anchors);
    for group in &groups {
        let group_anchor = format!("rule-{}", group.rule_id);
        out.push_str(&format!(
            "<section id=\"{group}\">\n<h3><code>{rule}</code> — {n} findings ({h} hot, {w} warn, {i} info)</h3>\n",
            group = escape_html(&group_anchor),
            rule = escape_html(group.rule_id),
            n = group.members.len(),
            h = group.hot,
            w = group.warn,
            i = group.info,
        ));
        html_findings_table(out, doc, &group.members, &anchors);
        out.push_str("</section>\n");
    }
    out.push_str("</section>\n");
}

/// Detail-page head: same markup as [`html_head`] (doctype, charset,
/// viewport, empty favicon, inline `<style>`), titled for the findings
/// detail artifact. It keeps the 72rem body width: the 90rem width on the
/// report page exists for the Atlas canvas, while this page and the
/// profile-queue page intentionally stay at 72rem for dense tables.
fn html_detail_head(out: &mut String, doc: &MetricsDocument) {
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    out.push_str("<link rel=\"icon\" href=\"data:,\">\n");
    out.push_str(&format!(
        "<title>Perfpal Findings Detail — schema {}</title>\n",
        escape_html(&doc.schema_version),
    ));
    out.push_str(
        "<style>\n\
         body{font-family:monospace,monospace;max-width:72rem;margin:2rem auto;padding:0 1rem;color:#111;background:#fff}\n\
         .table-scroll{overflow-x:auto;max-width:100%}\n\
         table{border-collapse:collapse;width:100%;margin:1rem 0}\n\
         th,td{border:1px solid #999;padding:.25rem .5rem;text-align:left;vertical-align:top}\n\
         th{background:#eee}\n\
         #findings table{min-width:72rem}\n\
         #findings tbody td{white-space:nowrap}\n\
         #findings td.wrap{white-space:normal;overflow-wrap:anywhere;min-width:22rem;max-width:38rem}\n\
         figure{border:1px solid #999;padding:.5rem;margin:1rem 0;max-width:100%;overflow-x:auto}\n\
         figcaption{margin-top:.5rem}\n\
         img{max-width:100%;height:auto}\n\
         figure.pz img{max-width:none;touch-action:none}\n\
         #files tbody tr[data-path],#components tbody tr[data-prefix]{cursor:pointer}\n\
         #filter-bar{margin:.5rem 0}\n\
         nav ul{list-style:none;padding:0;display:flex;gap:1rem;flex-wrap:wrap}\n\
         #findings summary{cursor:pointer;list-style:none}\n\
         #findings summary::-webkit-details-marker{display:none}\n\
         #findings summary::before{content:\"▸\";display:inline-block;margin-right:.35em}\n\
         #findings details[open]>summary::before{transform:rotate(90deg)}\n",
    );
    out.push_str(KEY_VIEW_CSS);
    out.push_str("         </style>\n</head>\n<body>\n");
}

/// Full findings-table renderer, kept as a SEPARATE function by design
/// (C-split seam): the complete `<table>` (shared head + one
/// [`html_finding_row`] per index) for EXACTLY the findings in `indices`.
/// The `findings-detail.html` artifact calls it once per rule group, reusing
/// the existing table markup verbatim — do not delete it or inline its
/// markup into the section wrapper.
fn html_findings_table(
    out: &mut String,
    doc: &MetricsDocument,
    indices: &[usize],
    anchors: &[String],
) {
    out.push_str(
        "<div class=\"table-scroll\">\n<table>\n<thead><tr><th>finding_id</th><th>rule</th><th>severity</th>\
         <th>confidence</th><th>resolver</th><th>location</th><th>symbol</th>\
         <th>message</th><th>evidence</th><th>threshold</th><th>flags</th></tr></thead>\n<tbody>\n",
    );
    for &i in indices {
        html_finding_row(out, &doc.findings[i], &anchors[i]);
    }
    out.push_str("</tbody>\n</table>\n</div>\n");
}

/// One findings-table row: full §4 parity (FIX 3) — symbol, resolver tier,
/// flags, threshold, and evidence (values escaped — hostile evidence cannot
/// break out) alongside the finding_id/rule/severity/confidence/location/
/// message core.
fn html_finding_row(out: &mut String, f: &FindingEntry, anchor: &str) {
    let mut flags: Vec<&str> = Vec::new();
    if f.macro_suspect {
        flags.push("macro_suspect");
    }
    if f.error_node_overlap {
        flags.push("error_node_overlap");
    }
    let flags_text = if flags.is_empty() {
        "none".to_string()
    } else {
        escape_html(&flags.join(", "))
    };
    let evidence_text = if f.evidence.is_empty() {
        "none".to_string()
    } else {
        // BTreeMap ⇒ keys already sorted; keys AND values escaped (FIX 3:
        // hostile evidence values stay inert — this also closes the
        // hostile-evidence review bypass).
        f.evidence
            .iter()
            .map(|(k, v)| format!("{}={}", escape_html(k), escape_html(v)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    out.push_str(&format!(
        "<tr id=\"{anchor}\" data-path=\"{data_path}\"><td>{id}</td><td>{rule}</td><td>{severity}</td>\
         <td>{confidence}</td><td>{resolver}</td>\
         <td>{path}:{line}:{col}</td><td>{symbol}</td><td class=\"wrap\">{message}</td>\
         <td class=\"wrap\">{evidence}</td><td>{threshold}</td><td>{flags}</td></tr>\n",
        anchor = escape_html(anchor),
        data_path = escape_html(&f.path),
        id = escape_html(&f.finding_id),
        rule = escape_html(&f.rule_id),
        severity = severity_token(f.severity),
        confidence = confidence_token(f.confidence),
        resolver = resolver_tier_token(f.resolver_tier),
        path = escape_html(&f.path),
        line = f.span.start_line,
        col = f.span.start_col,
        symbol = escape_html(f.symbol.as_deref().unwrap_or("n/a")),
        message = escape_html(&f.message),
        evidence = evidence_text,
        threshold = escape_html(&threshold_text(&f.threshold)),
        flags = flags_text,
    ));
}

/// Render an optional threshold as `name=value` (`n/a` when absent).
/// Text values are returned raw — the caller passes the result through
/// [`escape_html`].
fn threshold_text(threshold: &Option<crate::schema::Threshold>) -> String {
    match threshold {
        None => "n/a".to_string(),
        Some(t) => {
            let value = match &t.value {
                crate::schema::ThresholdValue::Number(n) => {
                    if n.fract() == 0.0 && n.is_finite() {
                        format!("{}", *n as i64)
                    } else {
                        n.to_string()
                    }
                }
                crate::schema::ThresholdValue::Text(s) => s.clone(),
            };
            format!("{}={}", t.name, value)
        }
    }
}

/// Appendix A: heuristic reference for exactly the rule_ids present in this
/// report's findings, sorted for determinism (mirrors §5 Appendix A).
fn html_appendix_a(out: &mut String, doc: &MetricsDocument) {
    out.push_str("<section id=\"appendix-a\">\n<h2>Appendix A. Heuristic reference</h2>\n");
    if doc.findings.is_empty() {
        out.push_str("<p>No findings — no heuristics to reference.</p>\n</section>\n");
        return;
    }
    let rule_ids: BTreeSet<&str> = doc.findings.iter().map(|f| f.rule_id.as_str()).collect();
    out.push_str(
        "<div class=\"table-scroll\">\n<table>\n<thead><tr><th>rule_id</th><th>title</th><th>description</th></tr></thead>\n<tbody>\n",
    );
    for rule_id in rule_ids {
        let (_, title, description) = HEURISTIC_CATALOG_V1
            .iter()
            .find(|(id, _, _)| *id == rule_id)
            .unwrap_or_else(|| panic!("uncataloged rule_id {rule_id} reached Appendix A"));
        out.push_str(&format!(
            "<tr><td>{id}</td><td>{title}</td><td>{description}</td></tr>\n",
            id = escape_html(rule_id),
            title = escape_html(title),
            description = escape_html(description),
        ));
    }
    out.push_str("</tbody>\n</table>\n</div>\n</section>\n");
}

fn html_footer(out: &mut String) {
    out.push_str(
        "<footer>\n<p>Generated offline — no network access. Interactive \
         enhancements (pan/zoom, click-to-filter) run locally with no network. \
         Where this page and `metrics.json` ever disagree, `metrics.json` wins.</p>\n</footer>\n",
    );
    // Slice-2 enhancement script: fixed source (REPORT_SCRIPT), exactly one
    // block, bound via addEventListener only. Content is static text — no
    // document data is interpolated into the script.
    out.push_str("<script>\n");
    out.push_str(REPORT_SCRIPT);
    out.push_str("\n</script>\n</body>\n</html>\n");
}

/// Incident-edge degree of one file: resolved include edges touching `path`
/// (either endpoint) plus call-graph edges touching `path` (resolved
/// `to_path` only — unresolved hypotheses have no target file and never
/// enter the ranking).
fn file_degree(path: &str, doc: &MetricsDocument) -> usize {
    let mut degree: usize = 0;
    if let Some(graphs) = &doc.graphs {
        for edge in &graphs.include_graph.edges {
            if !edge.resolved {
                continue;
            }
            let to = edge.to.as_deref().unwrap_or("");
            if edge.from == path || to == path {
                degree += 1;
            }
        }
        if let Some(call_graph) = &graphs.call_graph {
            for edge in &call_graph.edges {
                if edge.from_path == path || edge.to_path.as_deref() == Some(path) {
                    degree += 1;
                }
            }
        }
    }
    degree
}

/// Slugify a file path into an `id`-anchor fragment: ASCII alphanumerics
/// kept verbatim, every other char (or run) → a single `-`, trimmed;
/// empty ⇒ `"root"`. Mirrors the charts-owned [`crate::charts::component`]
/// slug for components; local for files so the reporter owns the mapping.
fn slugify(path: &str) -> String {
    let mut slug = String::with_capacity(path.len());
    let mut last_dash = true; // suppress leading runs
    for c in path.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() {
        "root".to_string()
    } else {
        slug
    }
}

/// Sanitize a `finding_id` into an `id`-anchor fragment: ASCII alphanumerics
/// kept, every other char → `-` (no collapsing across the fixed
/// `rule:path:line:col` shape — each separator maps to exactly one dash, so
/// distinct finding ids stay distinct).
fn finding_anchor(finding_id: &str) -> String {
    let mut anchor = String::with_capacity(finding_id.len() + 8);
    anchor.push_str("finding-");
    for c in finding_id.chars() {
        if c.is_ascii_alphanumeric() {
            anchor.push(c);
        } else {
            anchor.push('-');
        }
    }
    anchor
}

/// Collision-free disambiguation (FIX 1): `bases` MUST arrive in sorted
/// order (deterministic across runs); the first use of a base keeps the
/// bare human-readable slug, subsequent colliding uses get `-2`, `-3`, …
/// suffixes. The common (collision-free) case stays clean while uniqueness
/// is guaranteed. Pure function of its input — byte-deterministic.
fn disambiguate_sorted_bases(bases: &[String]) -> Vec<String> {
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    bases
        .iter()
        .map(|base| {
            let n = counts.entry(base.as_str()).or_insert(0);
            *n += 1;
            if *n == 1 {
                base.clone()
            } else {
                format!("{base}-{}", *n)
            }
        })
        .collect()
}

/// File drill-down anchors keyed by path: bases built over the SORTED
/// deduplicated path set, so `a-b.c` vs `a/b.c` (same slug) get
/// `file-a-b-c` and `file-a-b-c-2` deterministically, independent of the
/// degree/cost render order.
fn file_anchor_map(doc: &MetricsDocument) -> std::collections::BTreeMap<String, String> {
    let mut paths: Vec<String> = doc.files.iter().map(|f| f.path.clone()).collect();
    paths.sort();
    paths.dedup();
    let bases: Vec<String> = paths
        .iter()
        .map(|p| format!("file-{}", slugify(p)))
        .collect();
    let unique = disambiguate_sorted_bases(&bases);
    paths.into_iter().zip(unique).collect()
}

/// Per-occurrence finding anchors (FIX 1, corrected): pre-assigned ONCE over
/// the stable `finding_id`-ascending occurrence order (position in that order
/// ⇒ anchor: first use of a slug base keeps it bare, later colliding uses
/// get `-2`, `-3`, …), keyed by the finding's index in `doc.findings`.
/// Returns the render order alongside the index-keyed anchor map so BOTH the
/// findings table and the file drill-down consume a single assignment —
/// ranked-file order (degree/cost/path) can no longer steal the bare anchor
/// from the finding_id-first colliding row. (The previous per-`finding_id`
/// FIFO queues were popped in ranked-file order by the drill-down but
/// assigned in `finding_id` order by the table — with two distinct ids that
/// slug-collide, the file rendered first grabbed the bare anchor regardless
/// of `finding_id` order, mislinking rows.) Stable sort keeps duplicate
/// finding_ids in document order; pure function of document content —
/// byte-deterministic.
fn finding_anchor_assignments(doc: &MetricsDocument) -> (Vec<usize>, Vec<String>) {
    let mut order: Vec<usize> = (0..doc.findings.len()).collect();
    order.sort_by(|&a, &b| doc.findings[a].finding_id.cmp(&doc.findings[b].finding_id));
    let bases: Vec<String> = order
        .iter()
        .map(|&i| finding_anchor(&doc.findings[i].finding_id))
        .collect();
    let unique = disambiguate_sorted_bases(&bases);
    let mut anchors = vec![String::new(); doc.findings.len()];
    for (pos, &i) in order.iter().enumerate() {
        anchors[i] = unique[pos].clone();
    }
    (order, anchors)
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

/// `(hot, warn, info)` finding counts for the honest §4 / findings-section
/// headers.
fn severity_counts(doc: &MetricsDocument) -> (usize, usize, usize) {
    let mut hot = 0;
    let mut warn = 0;
    let mut info = 0;
    for f in &doc.findings {
        match f.severity {
            Severity::Hot => hot += 1,
            Severity::Warn => warn += 1,
            Severity::Info => info += 1,
        }
    }
    (hot, warn, info)
}

/// Severity rank for finding-group ordering (output-contract §5 §4):
/// `hot` (0) → `warn` (1) → `info` (2).
fn severity_rank(severity: Severity) -> u8 {
    match severity {
        Severity::Hot => 0,
        Severity::Warn => 1,
        Severity::Info => 2,
    }
}

/// One firing rule's findings (group-and-fold unit shared by `report.md` §4
/// and `report.html` `#findings`).
struct RuleGroup<'a> {
    rule_id: &'a str,
    /// Document indices of the member findings in `finding_id`-ascending
    /// order (input order is never trusted); index 0 is the group's
    /// deterministic top exemplar.
    members: Vec<usize>,
    /// Hottest severity present (minimum [`severity_rank`]) — the primary
    /// group sort key.
    rank: u8,
    hot: usize,
    warn: usize,
    info: usize,
}

/// Group findings by `rule_id` in the pinned group order (output-contract §5
/// §4): severity-rank ascending (hottest group first), then member count
/// descending, then `rule_id` ascending. Members within a group are in
/// `finding_id`-ascending order, so exemplars are deterministic regardless of
/// analyzer emission order. Pure function of document content —
/// byte-deterministic.
fn group_findings_by_rule(doc: &MetricsDocument) -> Vec<RuleGroup<'_>> {
    // BTreeMap ⇒ rule iteration in rule_id-ascending order (the final sort
    // re-establishes the full order anyway; this keeps construction tidy).
    let mut by_rule: std::collections::BTreeMap<&str, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (index, finding) in doc.findings.iter().enumerate() {
        by_rule
            .entry(finding.rule_id.as_str())
            .or_default()
            .push(index);
    }
    let mut groups: Vec<RuleGroup<'_>> = by_rule
        .into_iter()
        .map(|(rule_id, mut members)| {
            members.sort_by(|&a, &b| doc.findings[a].finding_id.cmp(&doc.findings[b].finding_id));
            let (mut rank, mut hot, mut warn, mut info) = (2u8, 0usize, 0usize, 0usize);
            for &index in &members {
                let finding = &doc.findings[index];
                rank = rank.min(severity_rank(finding.severity));
                match finding.severity {
                    Severity::Hot => hot += 1,
                    Severity::Warn => warn += 1,
                    Severity::Info => info += 1,
                }
            }
            RuleGroup {
                rule_id,
                members,
                rank,
                hot,
                warn,
                info,
            }
        })
        .collect();
    groups.sort_by(|a, b| {
        a.rank
            .cmp(&b.rank)
            .then_with(|| b.members.len().cmp(&a.members.len()))
            .then_with(|| a.rule_id.cmp(b.rule_id))
    });
    groups
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
/// Captions are reporter-owned static text EXCEPT the include-graph arm,
/// which names the dependency mechanisms of the ACTUAL scanned languages
/// ([`include_graph_caption`]) — safe by construction (pinned per-language
/// fragments; anything else is escaped by the caller), never document-derived
/// (the dynamic name is charset-pinned per output-contract §3), so they
/// bypass [`escape_md`].
fn chart_caption(stem: &str, languages: &[String]) -> String {
    match stem {
        "include-graph" => include_graph_caption(languages),
        "component-coupling" => {
            "Component coupling — cross-file call edges between components.".to_string()
        }
        "hot-components" => {
            "Hot components — ranking by candidate finding density.".to_string()
        }
        "layer-workspace" => {
            "Workspace map — components as nodes over inter-component call/include edges, heat by candidate finding density.".to_string()
        }
        "layer-file" => {
            "File hot-path map — top files by degree with reachable reserve, heat by candidate function cost (v2 where available; v1 fallback); highest-cost shown file outlined (ties → path-asc first).".to_string()
        }
        "layer-entry" => {
            "Entry-stratified cost map — files stratified by resolved entry reachability, heat by candidate function cost; LOW-confidence call edges excluded.".to_string()
        }
        "layer-bands" => {
            "Component bands — files grouped under capped components, cross-component file edges aggregated as counted bands.".to_string()
        }
        "layer-findings" => {
            "Findings map — severity-weighted finding density (hot 3 / warn 2 / info 1 per candidate), dots = severity fill + confidence ring; profile to confirm.".to_string()
        }
        // Detail pages never reach this caption: routing is by the exact
        // `detail_stems` registry in `render_architecture`, and orphan
        // registry stems link via `detail_caption` below. Unknown stems —
        // including colliding `component-<digits>-*` names with no matching
        // component — take the generic overview path.
        other => format!("Chart: {other}."),
    }
}

/// Dependency mechanisms of the scanned languages in PINNED order
/// (c → rust → python, then any other language sorted ascending by name).
/// Pure function of the language list — deterministic regardless of document
/// order. Unknown languages pass through as bare names (escaped by the
/// caller — see [`dependency_clause`]).
fn dependency_mechanisms(languages: &[String]) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    if languages.iter().any(|l| l == "c") {
        parts.push("#include".to_string());
    }
    if languages.iter().any(|l| l == "rust") {
        parts.push("mod/use".to_string());
    }
    if languages.iter().any(|l| l == "python") {
        parts.push("import".to_string());
    }
    let mut others: Vec<&String> = languages
        .iter()
        .filter(|l| l.as_str() != "c" && l.as_str() != "rust" && l.as_str() != "python")
        .collect();
    others.sort();
    others.dedup();
    for lang in others {
        parts.push(lang.clone());
    }
    parts
}

/// Caption for the include-graph overview, conditioned on the ACTUAL scanned
/// languages: a Python-only scan never claims "internal #include (C)".
/// `c + rust` renders "internal #include, mod/use dependencies …" (the old
/// hardcoded "(C) … (Rust)" parentheticals are gone everywhere).
#[must_use]
fn include_graph_caption(languages: &[String]) -> String {
    let parts = dependency_mechanisms(languages);
    if parts.is_empty() {
        "Dependency graph — internal dependencies between scanned files.".to_string()
    } else {
        format!(
            "Dependency graph — internal {} dependencies between scanned files.",
            parts.join(", ")
        )
    }
}

/// Oxford-joined clause over 1–3+ parts ("A", "A and B", "A, B, and C").
/// Pure function — byte-deterministic.
fn oxford_join(mut parts: Vec<String>) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts.remove(0),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let last = parts.pop().unwrap_or_default();
            format!("{}, and {}", parts.join(", "), last)
        }
    }
}

/// Edge-kind clause for the architecture paragraph ("C `#include`
/// directives and Rust `mod`/`use` module paths"), conditioned on the ACTUAL
/// scanned languages — same registry as [`dependency_mechanisms`]. `None`
/// when no language names an edge kind (the paragraph then falls back to the
/// generic sentence). Known-language fragments are pinned static text (they
/// carry backticks that must render as Markdown code spans / literal HTML);
/// unknown language names are document-derived and go through `escape`.
#[must_use]
fn dependency_clause(languages: &[String], escape: impl Fn(&str) -> String) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if languages.iter().any(|l| l == "c") {
        parts.push("C `#include` directives".to_string());
    }
    if languages.iter().any(|l| l == "rust") {
        parts.push("Rust `mod`/`use` module paths".to_string());
    }
    if languages.iter().any(|l| l == "python") {
        parts.push("Python `import` bindings".to_string());
    }
    let mut others: Vec<&String> = languages
        .iter()
        .filter(|l| l.as_str() != "c" && l.as_str() != "rust" && l.as_str() != "python")
        .collect();
    others.sort();
    others.dedup();
    for lang in others {
        parts.push(format!("{} module edges", escape(lang)));
    }
    if parts.is_empty() {
        None
    } else {
        Some(oxford_join(parts))
    }
}

/// Full architecture dependency sentence for the scanned languages (shared
/// by the Markdown §5 and the HTML page): the edge-kind clause names ONLY the
/// mechanisms of the actual scanned languages ([`dependency_clause`]); with
/// no clause the generic sentence stands alone. `c + rust` renders
/// byte-identical to the historical hardcoded sentence.
#[must_use]
fn dependency_paragraph(languages: &[String], escape: impl Fn(&str) -> String) -> String {
    let middle = match dependency_clause(languages, escape) {
        Some(clause) => format!(" — {clause}"),
        None => String::new(),
    };
    format!(
        "The dependency graph reflects resolved internal include/module edges \
         between scanned files{middle}; unresolved internal edges are retained \
         as data, and external (system headers/crates) dependencies are \
         excluded from coupling edges.\n",
    )
}

/// Caption for registry-confirmed per-component detail pages (orphan links
/// only — the hooked index builds its own link text).
fn detail_caption() -> String {
    "Component detail — per-file hot-path map.".to_string()
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

/// True when the run used the semantic resolver tier. Two independent
/// signals, because neither alone is sufficient: a semantic-tier finding
/// proves the tier produced a fact, while a file with
/// `parse_health.resolver == Ok` proves libclang actually parsed a TU even
/// when no emitted finding needed a semantic upgrade (e.g. a single-file C
/// project with no cross-file edge). The default build records `Absent`
/// (or omits the field), so the default report stays on the syntactic text.
fn semantic_tier_active(doc: &MetricsDocument) -> bool {
    doc.findings
        .iter()
        .any(|f| matches!(f.resolver_tier, ResolverTier::Semantic))
        || doc.files.iter().any(|f| {
            // The semantic tier is C-only (ADR-0001); a non-C unit that
            // happens to parse is never evidence of a C semantic run.
            f.language == "c"
                && matches!(f.parse_health.resolver, Some(crate::ir::ResolverHealth::Ok))
        })
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
