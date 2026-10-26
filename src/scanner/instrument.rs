//! `scanner::instrument` — IR v1.4 STEP 2 producers (plan `divergence-revamp`).
//!
//! Populates the three source-local instrument fields on every
//! [`FunctionRecord`](crate::ir::FunctionRecord):
//! [`idiom_hash`](crate::ir::FunctionRecord::idiom_hash),
//! [`error_path`](crate::ir::FunctionRecord::error_path), and
//! [`alloc_family`](crate::ir::FunctionRecord::alloc_family).
//! The four call-shape fields (`call_fanout/fanin/depth/is_recursive`) need a
//! whole-program view and are computed by the aggregator instead.
//!
//! Runs as a post-pass inside [`extract_unit`](super::extract_unit), where the
//! file source is still available — body text is sliced line-granularly from
//! `record.span` (approximation, documented on [`body_text`]).
//!
//! Determinism: every rule here is a pure function of (language, body text,
//! event streams) over sorted structures — no I/O, no clock, no hash-map
//! iteration.

use crate::ir::{CalleeKind, LoopKind, RUST_ALLOC_FAMILY};
use crate::scanner::FunctionUnit;
use crate::scanner::backends::tree_sitter_c::ALLOC_FAMILY;
use crate::scanner::backends::tree_sitter_python::PY_ALLOC_FAMILY;

/// Dominant allocation family tag (divergence-study spec §"Instrument
/// revamp": `real_alloc` vs `empty_ctor` vs `serialize/format`). The enum is
/// deliberately small — three values — and serializes as its spec string via
/// [`AllocFamily::as_str`]. The closed value set is ALSO documented on the IR
/// field ([`FunctionRecord::alloc_family`](crate::ir::FunctionRecord::alloc_family))
/// and in the schema JSON description; the wire type stays `string` (no
/// closed-enum validation change — additive-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AllocFamily {
    /// True heap allocation: C `malloc`-family, Rust `Box`/`Rc`/`Arc`/
    /// `with_capacity`/`to_vec`/`to_owned`, Python sized/container
    /// constructors and `open`/`DataFrame`-family.
    RealAlloc,
    /// Allocation of an EMPTY container: Rust `String::new`/`Vec::new`/
    /// map-set `::new` + empty `vec![]`, Python zero-arg container ctors.
    EmptyCtor,
    /// Formatting/serialization-shaped construction: Rust `format!`/
    /// `to_string`/`collect`, Python `str(x)`.
    Serialize,
}

impl AllocFamily {
    /// Spec tag string (`real_alloc` | `empty_ctor` | `serialize`).
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            AllocFamily::RealAlloc => "real_alloc",
            AllocFamily::EmptyCtor => "empty_ctor",
            AllocFamily::Serialize => "serialize",
        }
    }
}

/// Classify ONE direct call site into the alloc family, or `None` when the
/// call is not an allocation-family site.
///
/// Language matchers (lifted from the existing ALLOC matchers, same names):
/// - C: [`ALLOC_FAMILY`] (`malloc`, `calloc`, `realloc`, `aligned_alloc`,
///   `strdup`) ⇒ [`AllocFamily::RealAlloc`]. `free` is NOT allocation.
/// - Rust: [`RUST_ALLOC_FAMILY`] split by group — qualified `*::new`
///   zero-arg ctors (`String`/`Vec`/`VecDeque`/`HashMap`/`HashSet`/
///   `BTreeMap`/`BTreeSet`) ⇒ [`AllocFamily::EmptyCtor`]; qualified
///   `Box`/`Rc`/`Arc` `::new` + `with_capacity` ⇒ [`AllocFamily::RealAlloc`];
///   methods `to_string`/`collect` ⇒ [`AllocFamily::Serialize`],
///   `to_vec`/`to_owned`/`with_capacity` ⇒ [`AllocFamily::RealAlloc`];
///   macros `format` ⇒ [`AllocFamily::Serialize`], `vec` ⇒
///   [`AllocFamily::EmptyCtor`] iff the (whitespace-collapsed) body shows the
///   empty form `vec![]`, else [`AllocFamily::RealAlloc`] (macro args are an
///   opaque `token_tree`, so emptiness is read textually — documented).
/// - Python: [`PY_ALLOC_FAMILY`] on the callee name —
///   `list`/`dict`/`set`/`tuple`/`frozenset`/`bytes`/`bytearray` ⇒
///   [`AllocFamily::EmptyCtor`] iff zero args else [`AllocFamily::RealAlloc`];
///   `str` ⇒ [`AllocFamily::EmptyCtor`] iff zero args else
///   [`AllocFamily::Serialize`]; `open`/`DataFrame`/`Series`/`array`/
///   `ndarray` ⇒ [`AllocFamily::RealAlloc`].
/// - JavaScript/TypeScript/TSX (same V8/Node mechanism): qualified `Buffer.alloc`/`Buffer.from`/`Array.from`/
///   `Object.assign` and bare `structuredClone` ⇒ [`AllocFamily::RealAlloc`];
///   bare `Array`/`Object`/`Map`/`Set`/`WeakMap`/`WeakSet` ⇒
///   [`AllocFamily::EmptyCtor`] iff zero args else
///   [`AllocFamily::RealAlloc`]; `JSON.stringify`/`JSON.parse`, `String` with
///   args, and methods `toString`/`toJSON` ⇒ [`AllocFamily::Serialize`].
///
/// Non-`Direct` calls (indirect/unknown — the callee name is a variable, not
/// a target) never classify. Unknown languages return `None`.
pub(crate) fn classify_alloc_call(
    language: &str,
    callee_path: Option<&str>,
    callee_name: &str,
    callee_kind: CalleeKind,
    is_macro: bool,
    arg_count: usize,
    collapsed_body: &str,
) -> Option<AllocFamily> {
    if callee_kind != CalleeKind::Direct {
        return None;
    }
    match language {
        "c" => {
            if !is_macro && ALLOC_FAMILY.contains(&callee_name) {
                Some(AllocFamily::RealAlloc)
            } else {
                None
            }
        }
        "rust" => {
            if is_macro {
                return match callee_name {
                    "format" => Some(AllocFamily::Serialize),
                    "vec" => {
                        if collapsed_body.contains("vec![]") {
                            Some(AllocFamily::EmptyCtor)
                        } else {
                            Some(AllocFamily::RealAlloc)
                        }
                    }
                    _ => None,
                };
            }
            if let Some(path) = callee_path {
                match path {
                    "String::new" | "Vec::new" | "VecDeque::new" | "HashMap::new"
                    | "HashSet::new" | "BTreeMap::new" | "BTreeSet::new" => {
                        return Some(AllocFamily::EmptyCtor);
                    }
                    "Box::new"
                    | "Rc::new"
                    | "Arc::new"
                    | "Vec::with_capacity"
                    | "String::with_capacity" => return Some(AllocFamily::RealAlloc),
                    _ => {}
                }
            }
            // Method-name fallback mirrors
            // `RUST_ALLOC_FAMILY::matches_site` group semantics; `clone` is
            // excluded upstream (noise control) and stays excluded here.
            debug_assert!(
                RUST_ALLOC_FAMILY.matches_site(callee_path, callee_name, false)
                    || !RUST_ALLOC_FAMILY.methods.contains(&callee_name),
                "classifier must stay consistent with RUST_ALLOC_FAMILY membership"
            );
            match callee_name {
                "to_string" | "collect" => Some(AllocFamily::Serialize),
                "to_vec" | "to_owned" | "with_capacity" => Some(AllocFamily::RealAlloc),
                _ => None,
            }
        }
        "python" => {
            if is_macro || !PY_ALLOC_FAMILY.contains(&callee_name) {
                return None;
            }
            match callee_name {
                "open" | "DataFrame" | "Series" | "array" | "ndarray" => {
                    Some(AllocFamily::RealAlloc)
                }
                "str" => {
                    if arg_count == 0 {
                        Some(AllocFamily::EmptyCtor)
                    } else {
                        Some(AllocFamily::Serialize)
                    }
                }
                _ => {
                    if arg_count == 0 {
                        Some(AllocFamily::EmptyCtor)
                    } else {
                        Some(AllocFamily::RealAlloc)
                    }
                }
            }
        }
        "js" | "ts" | "tsx" => {
            if is_macro {
                return None;
            }
            // Qualified call paths carry the namespace (the JS backend sets
            // `callee_path` for member calls). `Buffer.alloc`/`Buffer.from`
            // are explicit copies; `Array.from`/`Object.assign` allocate a
            // new result; `JSON.*` are parse/serialize-shaped.
            if let Some(path) = callee_path {
                match path {
                    "Buffer.alloc" | "Buffer.from" | "Array.from" | "Object.assign" => {
                        return Some(AllocFamily::RealAlloc);
                    }
                    "JSON.stringify" | "JSON.parse" => {
                        return Some(AllocFamily::Serialize);
                    }
                    _ => {}
                }
            }
            match callee_name {
                "structuredClone" => Some(AllocFamily::RealAlloc),
                // Bare container constructors: allocation family depends on
                // arity, mirroring the Python/`str` split.
                "Array" | "Object" | "Map" | "Set" | "WeakMap" | "WeakSet" => {
                    if arg_count == 0 {
                        Some(AllocFamily::EmptyCtor)
                    } else {
                        Some(AllocFamily::RealAlloc)
                    }
                }
                // `String(x)` formats; zero-arg `String()` is not a site.
                "String" => (arg_count > 0).then_some(AllocFamily::Serialize),
                "toString" | "toJSON" => Some(AllocFamily::Serialize),
                _ => None,
            }
        }
        _ => None,
    }
}

/// Dominant family over a function's alloc sites: majority wins; ties break
/// toward the lexicographically smallest tag (`empty_ctor` < `real_alloc` <
/// `serialize`) — deterministic and documented. Empty input ⇒ `None` (the
/// field is omitted on the wire when the function has no alloc sites).
pub(crate) fn dominant_alloc_family(sites: &[AllocFamily]) -> Option<&'static str> {
    if sites.is_empty() {
        return None;
    }
    let mut counts = [0u32; 3];
    for site in sites {
        counts[*site as usize] += 1;
    }
    // Index order EmptyCtor(1) < RealAlloc(0) < Serialize(2) is NOT
    // lexicographic, so compare explicitly: max count, tie → smallest tag.
    let mut best: AllocFamily = AllocFamily::EmptyCtor;
    let mut best_count = 0u32;
    for family in [
        AllocFamily::EmptyCtor,
        AllocFamily::RealAlloc,
        AllocFamily::Serialize,
    ] {
        let count = counts[family as usize];
        if count > best_count {
            best = family;
            best_count = count;
        }
    }
    (best_count > 0).then(|| best.as_str())
}

/// Whitespace-collapse + comment-strip normalization feeding BOTH
/// [`idiom_hash`] and the textual [`detect_error_path`] checks.
///
/// Rules (documented simplicity over fidelity — stability is what matters):
/// - `/* … */` block comments removed (unclosed ⇒ dropped to end);
/// - `// …` line comments removed (C/Rust);
/// - `# …` line comments removed ONLY for Python (C/Rust `#` lines are
///   preprocessor directives — kept as code; Python `#` inside strings is a
///   documented over-strip);
/// - NO string-literal awareness anywhere (a `//` inside `"http://…"` is cut
///   — documented limitation, stable across runs regardless);
/// - remaining text split on whitespace and re-joined with single spaces.
pub(crate) fn normalize_idiom(body: &str, language: &str) -> String {
    let strip_hash = language == "python";
    let mut out = String::with_capacity(body.len());
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &bytes[i..];
        if rest.starts_with(b"/*") {
            match body[i + 2..].find("*/") {
                Some(end) => i += 2 + end + 2,
                None => break, // unclosed block: drop to end
            }
        } else if rest.starts_with(b"//") || (strip_hash && rest.starts_with(b"#")) {
            match body[i..].find('\n') {
                Some(end) => i += end,
                None => break,
            }
        } else {
            // Char-boundary-safe advance (bodies are lossy-UTF8 text).
            let ch_len = body[i..].chars().next().map_or(1, |c| c.len_utf8());
            out.push_str(&body[i..i + ch_len]);
            i += ch_len;
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// FNV-1a 64-bit over UTF-8 bytes, hex-encoded (16 lowercase chars).
///
/// Hand-rolled (10 lines, zero deps) so the digest is pinned FOREVER —
/// `std`'s `DefaultHasher` (SipHash) is explicitly unstable across rustc
/// versions and must never feed a persisted hash. Empty input hashes to the
/// FNV offset basis `cbf29ce484222325` (locked by test).
fn fnv1a_hex(text: &str) -> String {
    const OFFSET: u64 = 14_695_981_039_346_656_037;
    const PRIME: u64 = 1_099_511_628_211;
    let mut hash = OFFSET;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{hash:016x}")
}

/// Token-normalized idiom hash of a function body: [`normalize_idiom`] then
/// [`fnv1a_hex`]. Equal code modulo comments/whitespace ⇒ equal hash (clone/
/// idiom-rate bucketing input for the divergence-study instrument).
pub(crate) fn idiom_hash(body: &str, language: &str) -> String {
    fnv1a_hex(&normalize_idiom(body, language))
}

/// Minimal error-path detector (divergence-study spec list). Returns true
/// when the body contains a CLEAR error-handling signal:
///
/// - `try_blocks > 0` — Python `try`/`except` (the backend's `TryEvent`
///   stream; empty for C/Rust);
/// - C `do-while(0)` macro-wrapper idiom — a `Do` loop AND the collapsed body
///   shows `while(0)`/`while (0)` (a bare `Do` loop with a real condition is
///   NOT flagged — conservative);
/// - `#if 0` gating in the collapsed body (any language; C in practice);
/// - Rust: `?` operator (collapsed body minus `?Sized` bounds still contains
///   `?` — `?` is unambiguous in Rust outside `?Sized`), `Err(` construction,
///   `cfg(test)`/`cfg (test)` gating (attributes live inside the function
///   span, so the slice sees them);
/// - C error-branch `free`: a `free(` call AND error vocabulary (`goto`,
///   `err`, `fail`, `cleanup`, `stderr` — substring match, so `errno`/
///   `perror`/`stderr` all hit; case-sensitive by C convention);
/// - Python: `raise ` / `raise(` (explicit raise; `except` is covered by
///   `try_blocks`);
/// - JavaScript/TypeScript/TSX: `throw ` / `throw(` (explicit throw; `try`/`catch`
///   is covered by `try_blocks`).
///
/// Heuristic limits (documented, NOT branch-sensitive): the checks are
/// textual over the normalized body — a `?` in dead code still flags, and
/// `free` + `err` in unrelated statements still flags. Conservative in the
/// spec's sense: each rule fires only on its listed clear-case token.
/// `body` MUST be the [`normalize_idiom`] output (comment-stripped,
/// whitespace-collapsed) so comment text can never flag.
pub(crate) fn detect_error_path(
    language: &str,
    collapsed_body: &str,
    try_blocks: usize,
    has_do_loop: bool,
    has_free_call: bool,
) -> bool {
    if try_blocks > 0 {
        return true;
    }
    if collapsed_body.contains("#if 0") {
        return true;
    }
    if has_do_loop && (collapsed_body.contains("while(0)") || collapsed_body.contains("while (0)"))
    {
        return true;
    }
    match language {
        "rust" => {
            if collapsed_body.contains("Err(") {
                return true;
            }
            if collapsed_body.contains("cfg(test)") || collapsed_body.contains("cfg (test)") {
                return true;
            }
            collapsed_body.replace("?Sized", "").contains('?')
        }
        "c" => {
            has_free_call
                && ["goto", "err", "fail", "cleanup", "stderr"]
                    .iter()
                    .any(|token| collapsed_body.contains(token))
        }
        "python" => collapsed_body.contains("raise ") || collapsed_body.contains("raise("),
        "js" | "ts" | "tsx" => {
            collapsed_body.contains("throw ") || collapsed_body.contains("throw(")
        }
        _ => false,
    }
}

/// Slice a function body from `source` by IR `span`, LINE-granular
/// (1-based inclusive lines ⇒ byte lines `[start-1, end)` clamped).
/// Columns are ignored — first/last lines may carry bytes outside the exact
/// span (documented approximation; stability is unaffected). Non-UTF8 bytes
/// become U+FFFD (lossy) — deterministic for a fixed input.
pub(crate) fn body_text(source: &[u8], span: &crate::ir::Span) -> String {
    let text = String::from_utf8_lossy(source);
    let lines: Vec<&str> = text.split('\n').collect();
    let start = span.start_line.saturating_sub(1) as usize;
    let end = (span.end_line as usize).min(lines.len());
    if start >= lines.len() || start >= end {
        return String::new();
    }
    lines[start..end].join("\n")
}

/// Populate the three source-local v1.4 fields on `func.record` from its
/// event streams + body text. Call-shape fields are left for the aggregator.
/// Runs inside [`extract_unit`](super::extract_unit) — the only stage holding
/// both the record and its source.
pub(crate) fn populate(func: &mut FunctionUnit, body: &str, language: &str) {
    let collapsed = normalize_idiom(body, language);
    let record = &mut func.record;
    record.idiom_hash = Some(idiom_hash(body, language));
    let sites: Vec<AllocFamily> = func
        .calls
        .iter()
        .filter_map(|call| {
            classify_alloc_call(
                language,
                call.callee_path.as_deref(),
                &call.callee_name,
                call.callee_kind,
                call.is_macro,
                call.args.len(),
                &collapsed,
            )
        })
        .collect();
    record.alloc_family = dominant_alloc_family(&sites).map(str::to_string);
    record.error_path = Some(detect_error_path(
        language,
        &collapsed,
        func.try_blocks.len(),
        func.loops.iter().any(|l| l.kind == LoopKind::Do),
        language == "c" && func.calls.iter().any(|c| c.callee_name == "free"),
    ));
}

#[cfg(test)]
mod tests {
    //! v1.4 step-2 producer locks: normalization stability, pinned digest,
    //! per-language alloc classification, dominant tie-breaks, error-path
    //! detectors (incl. negative controls).

    use super::*;

    #[test]
    fn fnv_digest_pins_empty_and_known_vector() {
        assert_eq!(fnv1a_hex(""), "cbf29ce484222325", "FNV offset basis");
        // Pinned: normalize("int f(void) { return 0; }") digest. Recompute by
        // hand on any change — a drift means every committed idiom_hash moved.
        assert_eq!(
            idiom_hash("int f(void) { return 0; }", "c"),
            fnv1a_hex("int f(void) { return 0; }")
        );
        assert_eq!(idiom_hash("", "c"), "cbf29ce484222325");
    }

    #[test]
    fn normalization_is_stable_over_comments_and_whitespace() {
        let a =
            "int f(void) {\n  // allocate\n  void *p = malloc(n); /* trailing */\n  return p;\n}";
        let b = "int\tf(void)\n{\nvoid *p = malloc(n);\nreturn p;\n}";
        assert_eq!(normalize_idiom(a, "c"), normalize_idiom(b, "c"));
        assert_eq!(idiom_hash(a, "c"), idiom_hash(b, "c"));
    }

    #[test]
    fn normalization_differs_on_real_code_change() {
        assert_ne!(
            idiom_hash("int f(void) { return 0; }", "c"),
            idiom_hash("int f(void) { return 1; }", "c")
        );
    }

    #[test]
    fn hash_comments_strip_per_language() {
        // Python `#` comments strip; C/Rust `#` lines (directives) survive.
        assert_eq!(normalize_idiom("x = 1 # comment", "python"), "x = 1");
        assert_eq!(
            normalize_idiom("#if 0\nx = 1", "c"),
            "#if 0 x = 1",
            "C directives are code for hashing"
        );
    }

    #[test]
    fn c_alloc_classifies_malloc_family_only() {
        for name in ["malloc", "calloc", "realloc", "aligned_alloc", "strdup"] {
            assert_eq!(
                classify_alloc_call("c", None, name, CalleeKind::Direct, false, 1, ""),
                Some(AllocFamily::RealAlloc),
                "{name}"
            );
        }
        assert_eq!(
            classify_alloc_call("c", None, "free", CalleeKind::Direct, false, 1, ""),
            None,
            "free is not allocation"
        );
        assert_eq!(
            classify_alloc_call("c", None, "malloc", CalleeKind::Indirect, false, 1, ""),
            None,
            "indirect calls never classify"
        );
    }

    #[test]
    fn rust_alloc_splits_empty_ctor_serialize_and_real() {
        let direct = CalleeKind::Direct;
        // Empty ctors.
        for path in [
            "String::new",
            "Vec::new",
            "VecDeque::new",
            "HashMap::new",
            "HashSet::new",
            "BTreeMap::new",
            "BTreeSet::new",
        ] {
            assert_eq!(
                classify_alloc_call("rust", Some(path), "new", direct, false, 0, ""),
                Some(AllocFamily::EmptyCtor),
                "{path}"
            );
        }
        // Sized/boxing ctors are real.
        for path in [
            "Box::new",
            "Rc::new",
            "Arc::new",
            "Vec::with_capacity",
            "String::with_capacity",
        ] {
            assert_eq!(
                classify_alloc_call("rust", Some(path), "new", direct, false, 1, ""),
                Some(AllocFamily::RealAlloc),
                "{path}"
            );
        }
        // Methods: serialize vs real.
        assert_eq!(
            classify_alloc_call("rust", None, "to_string", direct, false, 1, ""),
            Some(AllocFamily::Serialize)
        );
        assert_eq!(
            classify_alloc_call("rust", None, "collect", direct, false, 0, ""),
            Some(AllocFamily::Serialize)
        );
        assert_eq!(
            classify_alloc_call("rust", None, "to_vec", direct, false, 1, ""),
            Some(AllocFamily::RealAlloc)
        );
        assert_eq!(
            classify_alloc_call("rust", None, "clone", direct, false, 0, ""),
            None,
            "clone stays excluded (noise control)"
        );
        // Macros.
        assert_eq!(
            classify_alloc_call("rust", None, "format", direct, true, 1, ""),
            Some(AllocFamily::Serialize)
        );
        assert_eq!(
            classify_alloc_call("rust", None, "vec", direct, true, 0, "let v = vec![];"),
            Some(AllocFamily::EmptyCtor)
        );
        assert_eq!(
            classify_alloc_call("rust", None, "vec", direct, true, 0, "let v = vec![1];"),
            Some(AllocFamily::RealAlloc)
        );
    }

    #[test]
    fn python_alloc_splits_by_arity() {
        let direct = CalleeKind::Direct;
        assert_eq!(
            classify_alloc_call("python", None, "list", direct, false, 0, ""),
            Some(AllocFamily::EmptyCtor)
        );
        assert_eq!(
            classify_alloc_call("python", None, "list", direct, false, 1, ""),
            Some(AllocFamily::RealAlloc)
        );
        assert_eq!(
            classify_alloc_call("python", None, "str", direct, false, 1, ""),
            Some(AllocFamily::Serialize)
        );
        assert_eq!(
            classify_alloc_call("python", None, "DataFrame", direct, false, 1, ""),
            Some(AllocFamily::RealAlloc)
        );
        assert_eq!(
            classify_alloc_call("python", None, "print", direct, false, 1, ""),
            None
        );
    }

    #[test]
    fn js_alloc_classifies_qualified_and_container_forms() {
        let direct = CalleeKind::Direct;
        // Qualified real allocations.
        for (path, name) in [
            ("Buffer.alloc", "alloc"),
            ("Buffer.from", "from"),
            ("Array.from", "from"),
            ("Object.assign", "assign"),
        ] {
            assert_eq!(
                classify_alloc_call("js", Some(path), name, direct, false, 1, ""),
                Some(AllocFamily::RealAlloc),
                "{path}"
            );
        }
        assert_eq!(
            classify_alloc_call("js", None, "structuredClone", direct, false, 1, ""),
            Some(AllocFamily::RealAlloc)
        );
        // Bare container constructors: arity decides.
        for name in ["Array", "Object", "Map", "Set", "WeakMap", "WeakSet"] {
            assert_eq!(
                classify_alloc_call("js", None, name, direct, false, 0, ""),
                Some(AllocFamily::EmptyCtor),
                "{name}()"
            );
            assert_eq!(
                classify_alloc_call("js", None, name, direct, false, 1, ""),
                Some(AllocFamily::RealAlloc),
                "{name}(1)"
            );
        }
        // Serialize-shaped.
        for path in ["JSON.stringify", "JSON.parse"] {
            let name = path.rsplit('.').next().unwrap_or("");
            assert_eq!(
                classify_alloc_call("js", Some(path), name, direct, false, 1, ""),
                Some(AllocFamily::Serialize),
                "{path}"
            );
        }
        assert_eq!(
            classify_alloc_call("js", None, "String", direct, false, 1, ""),
            Some(AllocFamily::Serialize)
        );
        assert_eq!(
            classify_alloc_call("js", None, "String", direct, false, 0, ""),
            None,
            "zero-arg String() is not an allocation site"
        );
        for name in ["toString", "toJSON"] {
            assert_eq!(
                classify_alloc_call("js", None, name, direct, false, 0, ""),
                Some(AllocFamily::Serialize),
                "{name}"
            );
        }
        // Non-sites.
        assert_eq!(
            classify_alloc_call("js", None, "parseInt", direct, false, 1, ""),
            None
        );
        assert_eq!(
            classify_alloc_call("js", None, "Buffer", CalleeKind::Indirect, false, 1, ""),
            None,
            "indirect calls never classify"
        );
    }

    #[test]
    fn js_private_fields_survive_normalization() {
        let body = "class C {\n  #cache = new Map(); // drop this\n  #count = 0; /* drop block */\n  read() { return this.#cache.get(this.#count); }\n}";
        let normalized = normalize_idiom(body, "js");
        assert!(
            normalized.contains("#cache"),
            "JS private field must survive"
        );
        assert!(
            normalized.contains("#count"),
            "JS private field must survive"
        );
        assert!(!normalized.contains("drop this"), "// comment stripped");
        assert!(!normalized.contains("drop block"), "/* */ comment stripped");
    }

    #[test]
    fn js_error_path_flags_explicit_throw() {
        assert!(detect_error_path(
            "js",
            "function f() { throw new Error('x'); }",
            0,
            false,
            false
        ));
        assert!(detect_error_path(
            "js",
            "throw(new Error('x'))",
            0,
            false,
            false
        ));
        assert!(!detect_error_path(
            "js",
            "const x = 1; return x;",
            0,
            false,
            false
        ));
        // try/catch is covered independently by `try_blocks`.
        assert!(detect_error_path(
            "js",
            "try { f(); } catch (e) {}",
            1,
            false,
            false
        ));
    }

    #[test]
    fn ts_and_tsx_mirror_js_alloc_and_error_path() {
        let direct = CalleeKind::Direct;
        let body = "const a = Buffer.alloc(4);\nthrow new Error('x');";
        for lang in ["js", "ts", "tsx"] {
            assert_eq!(
                classify_alloc_call(lang, Some("Buffer.alloc"), "alloc", direct, false, 1, ""),
                Some(AllocFamily::RealAlloc),
                "{lang}: qualified allocation"
            );
            assert_eq!(
                classify_alloc_call(lang, None, "Map", direct, false, 0, ""),
                Some(AllocFamily::EmptyCtor),
                "{lang}: zero-arg container ctor"
            );
            assert_eq!(
                classify_alloc_call(
                    lang,
                    Some("JSON.stringify"),
                    "stringify",
                    direct,
                    false,
                    1,
                    ""
                ),
                Some(AllocFamily::Serialize),
                "{lang}: serialize-shaped"
            );
            assert!(
                detect_error_path(
                    lang,
                    "function f() { throw new Error('x'); }",
                    0,
                    false,
                    false
                ),
                "{lang}: throw is an error path"
            );
            assert!(
                !detect_error_path(lang, "const x = 1; return x;", 0, false, false),
                "{lang}: quiet body is not an error path"
            );
            // Same runtime mechanism ⇒ same token normalization/hash.
            assert_eq!(
                idiom_hash(body, lang),
                idiom_hash(body, "js"),
                "{lang}: idiom hash matches js"
            );
        }
    }

    #[test]
    fn dominant_majority_wins_ties_go_lexicographic() {
        assert_eq!(dominant_alloc_family(&[]), None);
        assert_eq!(
            dominant_alloc_family(&[AllocFamily::RealAlloc]),
            Some("real_alloc")
        );
        assert_eq!(
            dominant_alloc_family(&[
                AllocFamily::Serialize,
                AllocFamily::RealAlloc,
                AllocFamily::RealAlloc,
            ]),
            Some("real_alloc")
        );
        // 1-1-1 tie ⇒ lexicographically smallest tag.
        assert_eq!(
            dominant_alloc_family(&[
                AllocFamily::Serialize,
                AllocFamily::RealAlloc,
                AllocFamily::EmptyCtor,
            ]),
            Some("empty_ctor")
        );
    }

    #[test]
    fn error_path_flags_clear_cases_per_language() {
        // Python try/except.
        assert!(detect_error_path(
            "python",
            "def f(): pass",
            1,
            false,
            false
        ));
        // Python raise.
        assert!(detect_error_path(
            "python",
            "def f(): raise ValueError()",
            0,
            false,
            false
        ));
        // C do-while(0) with the macro idiom text.
        assert!(detect_error_path("c", "do { x; } while(0)", 0, true, false));
        // A Do loop with a REAL condition is not the idiom.
        assert!(!detect_error_path(
            "c",
            "do { x; } while (n > 0)",
            0,
            true,
            false
        ));
        // #if 0 gating.
        assert!(detect_error_path("c", "#if 0 int x;", 0, false, false));
        // Rust ? / Err / cfg(test).
        assert!(detect_error_path(
            "rust",
            "let v = foo()?;",
            0,
            false,
            false
        ));
        assert!(!detect_error_path(
            "rust",
            "fn f<T: ?Sized>() {}",
            0,
            false,
            false
        ));
        assert!(detect_error_path(
            "rust",
            "fn f() -> Result<T> { Err(e) }",
            0,
            false,
            false
        ));
        assert!(detect_error_path(
            "rust",
            "#[cfg(test)] fn t() {}",
            0,
            false,
            false
        ));
        // C error-branch free.
        assert!(detect_error_path(
            "c",
            "if (rc) { free(buf); goto err; } err: return rc;",
            0,
            false,
            true
        ));
        // free WITHOUT error vocabulary is not an error path.
        assert!(!detect_error_path(
            "c",
            "free(buf); return 0;",
            0,
            false,
            true
        ));
        // Quiet bodies stay false everywhere.
        for lang in ["c", "rust", "python"] {
            assert!(
                !detect_error_path(lang, "int x = 1; return x;", 0, false, false),
                "{lang} quiet body must not flag"
            );
        }
    }

    #[test]
    fn body_text_slices_line_granular_and_clamps() {
        let src = b"line1\nline2\nline3\n";
        let span = crate::ir::Span {
            start_line: 2,
            start_col: 3,
            end_line: 3,
            end_col: 2,
        };
        assert_eq!(body_text(src, &span), "line2\nline3");
        let oob = crate::ir::Span {
            start_line: 9,
            start_col: 1,
            end_line: 12,
            end_col: 1,
        };
        assert_eq!(body_text(src, &oob), "");
    }
}
