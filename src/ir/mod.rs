//! `ir` — the seam (ir-v1.md): parser-agnostic record/event/shape types.
//!
//! File-level records ([`FileRecord`], [`ParseHealth`], [`IncludeEdge`],
//! [`GlobalVar`], [`StructDef`], [`MacroEvent`]), [`FunctionRecord`], and typed
//! events ([`CallEvent`], [`AssignmentEvent`], [`BinaryOpEvent`],
//! [`ComparisonEvent`], [`AccessChainEvent`], [`LoopNode`]) sharing the
//! [`Shape`] descriptor.
//!
//! Conventions (ir-v1.md §2): spans are 1-based lines and byte-columns with
//! INCLUSIVE ends; engine-native conversion (+1/−1) happens only at the
//! scanner boundary via [`Span::from_native`] / [`Span::to_native`]. Identity
//! keys are `(path, name, span)` — resolvers never change them.
//!
//! Confidence downgrade hooks (ir-v1.md §5): `macro_origin`, cross-file edges
//! (analyzer-owned, ARCH-003), error-region overlap (`ParseHealth`).
//!
//! Hard rule (ADR-0001 guardrail 1): this module references NO parser engine
//! types. The engine-native point representation ([`NativePoint`]) is a plain
//! struct so heuristics can depend on `ir` without transitively importing a
//! parser crate.

use serde::{Deserialize, Serialize};

/// 1-based line number (ir-v1.md §2).
pub type Line = u32;
/// 1-based byte column (ir-v1.md §2).
pub type Col = u32;

/// A half-open-in-source, closed-in-IR source range (ir-v1.md §2).
///
/// Lines are 1-based; columns are 1-based byte offsets; `end_line`/`end_col`
/// point AT the last byte of the match (inclusive).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct Span {
    pub start_line: Line,
    pub start_col: Col,
    pub end_line: Line,
    pub end_col: Col,
}

impl Span {
    /// A span covering a single byte position.
    #[must_use]
    pub fn point(line: Line, col: Col) -> Self {
        Self {
            start_line: line,
            start_col: col,
            end_line: line,
            end_col: col,
        }
    }

    /// Engine-native (0-based rows/columns, END-exclusive) → IR span
    /// (ir-v1.md §2: "+1 on entry, −1 on exit").
    ///
    /// The source bytes are required because the −1 side of the conversion
    /// must locate the last covered byte even when the exclusive end sits at
    /// column 0 of the following line (e.g. nodes that swallow their trailing
    /// newline). This is exact, not approximate.
    ///
    /// Degenerate empty ranges (`start == end`) produce a point span.
    #[must_use]
    pub fn from_native(source: &[u8], start: NativePoint, end: NativePoint) -> Self {
        let start_line = start.row as Line + 1;
        let start_col = start.column as Col + 1;
        if end.row == start.row && end.column <= start.column {
            return Self::point(start_line, start_col);
        }
        let (end_line, end_col) = if end.column > 0 {
            (end.row as Line + 1, end.column as Col)
        } else {
            // Exclusive end at column 0 of row `end.row`: the last covered
            // byte is the newline terminating 0-based row `end.row - 1`.
            // Its 1-based column is that line's byte length + 1.
            let len = line_byte_len(source, end.row - 1);
            (end.row as Line, len as Col + 1)
        };
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }

    /// Inverse of [`Span::from_native`] (ir-v1.md §7 round-trip contract):
    /// returns `(start, end_exclusive)` engine-native points.
    ///
    /// Canonicalization rule keeping the mapping injective (hence round-trip
    /// safe): the exclusive end is `(end_line − 1, end_col)` in 0-based terms
    /// — i.e. it stays on the SAME row as the last covered byte even when
    /// that byte ends its line. Engines that report a next-row/column-0 end
    /// (nodes swallowing their trailing newline) convert through
    /// [`Span::from_native`]'s source-aware branch instead.
    #[must_use]
    pub fn to_native(&self, _source: &[u8]) -> (NativePoint, NativePoint) {
        let start = NativePoint {
            row: self.start_line as usize - 1,
            column: self.start_col as usize - 1,
        };
        let end = NativePoint {
            row: self.end_line as usize - 1,
            column: self.end_col as usize,
        };
        (start, end)
    }

    /// Inclusive-span overlap check used for error-region and macro-origin
    /// downgrade hooks (ir-v1.md §5). Two spans intersect iff each starts at
    /// or before the other's end, comparing (line, col) lexicographically.
    #[must_use]
    pub fn intersects(&self, other: &Span) -> bool {
        let self_start = (self.start_line, self.start_col);
        let self_end = (self.end_line, self.end_col);
        let other_start = (other.start_line, other.start_col);
        let other_end = (other.end_line, other.end_col);
        self_start <= other_end && other_start <= self_end
    }
}

/// Engine-native point: 0-based row, 0-based BYTE column within the row.
///
/// Deliberately a plain struct (no engine crate types cross the seam,
/// ADR-0001 guardrail 1); scanner backends copy their engine's point fields
/// into this at the boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePoint {
    pub row: usize,
    pub column: usize,
}

/// Byte length of 0-based row `row`, excluding its terminating `\n`. A
/// trailing `\r` (CRLF lines) is a covered byte and therefore counts: the
/// `\n` of a CRLF line sits at 0-based column `len` (not `len - 1`), so the
/// inclusive 1-based end column of a newline-swallowing span is `len + 1`
/// either way.
fn line_byte_len(source: &[u8], row: usize) -> usize {
    let mut iter = source.split(|&b| b == b'\n');
    for _ in 0..row {
        iter.next();
    }
    iter.next().map_or(0, <[u8]>::len)
}

/// LOC breakdown (ir-v1.md §3.1). Counting rules live with the scanner
/// backend; the IR only carries the numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LocCounts {
    pub code: u32,
    pub comment: u32,
    pub blank: u32,
}

/// Include counts by form (ir-v1.md §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct IncludeCounts {
    pub quoted: u32,
    pub angle: u32,
}

/// Per-file structural summary (ir-v1.md §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FileRecord {
    /// Repo-relative POSIX path.
    pub path: String,
    /// `"c"` in v1.
    pub language: String,
    pub loc: LocCounts,
    pub function_count: u32,
    pub global_var_count: u32,
    pub include_counts: IncludeCounts,
}

/// Parse honesty record — consulted before ANY scoring (ir-v1.md §3.1,
/// modules.md Principle 3).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ParseHealth {
    pub status: ParseStatus,
    /// Spans of unrecovered/ERROR regions (outermost extents).
    pub error_regions: Vec<Span>,
    /// Total count of ERROR/missing nodes, including nested ones.
    pub error_node_count: u32,
}

/// Parse outcome (ir-v1.md §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParseStatus {
    #[default]
    Ok,
    Partial,
    Failed,
}

/// Include dependency edge (ir-v1.md §3.1).
///
/// Lifecycle: the backend fills `to_path` with the RAW include text and
/// `resolved=false`; the scanner's resolution pass then tries the same two
/// candidates (includer-relative, then repo-root-relative) for BOTH forms:
/// - Hit (either form): `to_path` rewritten to the repo-relative normalized
///   path, `resolved=true` — an internal coupling edge. Per ADR-0002 chart
///   policy, angle-bracket includes resolving to in-repo files count as
///   internal too.
/// - Miss on a QUOTED edge: `to_path=null`, `resolved=false` — data, not
///   error per modules.md Principle 4 (ARCH-001 signal). `to_path: None`
///   occurs ONLY here.
/// - Miss on an ANGLE edge: raw include text stays in `to_path`,
///   `resolved=false` — the external bucket (system headers etc.). Form is
///   authoritative for bucketing: form=angle ⇒ external EXCEPT when the
///   target resolves to an in-repo file; form=quoted ⇒ resolved
///   includer-relative-then-root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncludeEdge {
    pub from_path: String,
    pub to_path: Option<String>,
    pub form: IncludeForm,
    pub resolved: bool,
    pub span: Span,
}

/// Include directive form (ir-v1.md §3.1): `quoted` vs `angle`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IncludeForm {
    Quoted,
    Angle,
}

/// File-scope variable (ir-v1.md §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalVar {
    pub name: String,
    pub path: String,
    pub span: Span,
    pub storage: StorageClass,
}

/// Storage class of a global (ir-v1.md §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageClass {
    #[default]
    None,
    Static,
    Extern,
}

/// One struct member (ir-v1.md §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructMember {
    pub name: String,
    pub type_text: String,
    /// Best-effort integer array dimension (`int name[16]` ⇒ `Some(16)`),
    /// feeding the approximate size model (TYPE-001/002).
    pub array_dim_literal: Option<u64>,
    /// Declarator carries at least one pointer star (`char *p`) — recorded
    /// explicitly because declarator stars never reach `type_text`
    /// (additive v1 field; `#[serde(default)]` keeps older documents
    /// deserializing unchanged). Feeds the LP64 size model (TYPE-001/002).
    #[serde(default)]
    pub is_pointer: bool,
}

/// Struct definition (ir-v1.md §3.1). `name` is the tag name, `""` if anonymous.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructDef {
    pub name: String,
    pub path: String,
    pub span: Span,
    pub members: Vec<StructMember>,
}

/// Macro recognizer event kind (ir-v1.md §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MacroEventKind {
    Definition,
    Use,
    RecognizedPattern,
}

/// Curated macro-recognizer id for `#define` wrappers whose bodies call the
/// alloc family (catalog PERF-C-ALLOC-001 macro policy). Lives in `ir` so
/// the scanner backend (producer) and the analyzer heuristics (consumer)
/// share one constant across the seam; infrastructure is extensible — new
/// patterns append new ids, never rename existing ones.
pub const ALLOC_WRAPPER_PATTERN_ID: &str = "alloc-wrapper";

/// Macro event (ir-v1.md §3.1). `pattern_id` is set when
/// `kind = recognized_pattern` (curated recognizer id, e.g.
/// [`ALLOC_WRAPPER_PATTERN_ID`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroEvent {
    pub kind: MacroEventKind,
    pub name: String,
    pub path: String,
    pub span: Span,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern_id: Option<String>,
}

/// Function record — structural facts = baseline stats (ir-v1.md §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FunctionRecord {
    /// Identity rendering `path#name#start_line:start_col` (ir-v1.md §2).
    pub id: String,
    pub name: String,
    pub path: String,
    pub span: Span,
    pub param_count: u32,
    /// Raw type text per parameter (approximate; TYPE-001 input).
    pub param_types: Vec<String>,
    /// Branch-count approximation on the AST (catalog §2).
    pub cyclomatic_complexity: u32,
    /// Nesting-weighted variant of cyclomatic complexity.
    pub cognitive_complexity: u32,
    pub max_loop_depth: u32,
    /// malloc-family call sites in the body.
    pub alloc_site_count: u32,
    pub length_lines: u32,
}

/// Resolver tier tag carried by every event (ir-v1.md §2). v1 emits only
/// [`ResolverTier::Syntactic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResolverTier {
    #[default]
    Syntactic,
    Semantic,
    Dynamic,
}

/// Callee shape classification (ir-v1.md §3.3 CallEvent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CalleeKind {
    Direct,
    Indirect,
    #[default]
    Unknown,
}

/// Position of a call within its enclosing loop body (ir-v1.md §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatementRole {
    Primary,
    Nested,
    #[default]
    Unknown,
}

/// Shared expression vocabulary (ir-v1.md §3.4) — keeps events stream-shaped,
/// not AST-shaped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    Literal,
    Identifier,
    Call,
    Subscript,
    MemberAccess,
    Unary,
    Binary,
    Sizeof,
    #[default]
    Other,
}

/// Shape descriptor (ir-v1.md §3.4). Optional fields are omitted when absent.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Shape {
    pub kind: ShapeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_ident: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<u32>,
    /// Raw source text of a `sizeof` operand, parens stripped as written
    /// (`sizeof(p)` ⇒ `"p"`, `sizeof(struct S)` ⇒ `"struct S"`; additive v1
    /// field — `Option` keeps older documents deserializing unchanged).
    /// Feeds MEM-001 sizeof-misuse classification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizeof_operand: Option<String>,
}

impl Shape {
    /// A plain identifier shape.
    #[must_use]
    pub fn identifier(name: impl Into<String>) -> Self {
        Self {
            kind: ShapeKind::Identifier,
            name: Some(name.into()),
            ..Self::default()
        }
    }

    /// A literal shape carrying its source text.
    #[must_use]
    pub fn literal(value: impl Into<String>) -> Self {
        Self {
            kind: ShapeKind::Literal,
            value: Some(value.into()),
            ..Self::default()
        }
    }
}

/// Common event header fields (ir-v1.md §3.3). Flattened into every event so
/// serialized shapes stay schema-friendly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EventMeta {
    pub span: Span,
    /// 0 = not inside any loop.
    pub in_loop_depth: u32,
    /// Inside the deepest loop of its own ancestor chain (operational rule:
    /// no further loop nested below the event).
    pub is_innermost: bool,
    pub resolver_tier: ResolverTier,
    /// Span originates from a macro expansion/wrapper (ir-v1.md §2) — caps
    /// finding confidence at MEDIUM downstream.
    pub macro_origin: bool,
}

/// Call event (ir-v1.md §3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallEvent {
    #[serde(flatten)]
    pub meta: EventMeta,
    pub callee_name: String,
    pub callee_kind: CalleeKind,
    pub args: Vec<Shape>,
    pub statement_role: StatementRole,
    /// Call sits inside a conditional branch within the loop (LOOP-002 hint).
    pub conditional_guarded: bool,
}

/// Assignment event (ir-v1.md §3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentEvent {
    #[serde(flatten)]
    pub meta: EventMeta,
    pub target: Shape,
    /// Name-based classification against GlobalVar records (LOOP-005, MEDIUM).
    pub scope_class: ScopeClass,
}

/// Assignment-target scope class (ir-v1.md §3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScopeClass {
    Local,
    Param,
    Global,
    #[default]
    Unknown,
}

/// Binary operator event (ir-v1.md §3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryOpEvent {
    #[serde(flatten)]
    pub meta: EventMeta,
    /// Operator text, e.g. `"/"`, `"%"` (LOOP-004).
    pub op: String,
    pub left: Shape,
    pub right: Shape,
}

/// Comparison event (ir-v1.md §3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComparisonEvent {
    #[serde(flatten)]
    pub meta: EventMeta,
    pub left: Shape,
    pub right: Shape,
}

/// Member/subscript access chain event (ir-v1.md §3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessChainEvent {
    #[serde(flatten)]
    pub meta: EventMeta,
    /// Base identifier.
    pub root: String,
    /// Member/subscript sequence; chain depth ≥ 3 ⇒ LOOP-008 territory.
    pub elements: Vec<Shape>,
}

/// Loop kind (ir-v1.md §3.3 LoopNode).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoopKind {
    For,
    While,
    Do,
}

/// Loop record — loops anchor nesting chains (ir-v1.md §3.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoopNode {
    pub kind: LoopKind,
    /// Nesting depth (LOOP-001 threshold, default ≥ 3).
    pub depth: u32,
    /// Syntactic best-effort for-loop induction identifiers (feeds LOOP-007).
    pub induction_vars: Vec<String>,
    pub span: Span,
    pub resolver_tier: ResolverTier,
    pub macro_origin: bool,
}

#[cfg(test)]
mod tests {
    //! Span-convention locks (ir-v1.md §2, §7): engine-native ↔ IR
    //! conversions round-trip BOTH directions, including multi-line nodes and
    //! nodes whose engine end sits at column 0 of the next line.

    use super::*;

    const SRC: &[u8] = b"#include <stdio.h>\nint main(void) {\n    return 0;\n}\n";

    #[test]
    fn native_to_ir_adds_one_on_entry() {
        // tree-sitter point (row 1, col 0) → IR line 2, col 1.
        let span = Span::from_native(
            SRC,
            NativePoint { row: 1, column: 0 },
            NativePoint { row: 1, column: 16 },
        );
        assert_eq!(span.start_line, 2);
        assert_eq!(span.start_col, 1);
        assert_eq!(span.end_line, 2);
        assert_eq!(span.end_col, 16, "exclusive col N ⇒ inclusive col N");
    }

    #[test]
    fn newline_swallowing_node_ends_at_line_length_plus_one() {
        // `#include <stdio.h>\n` ends exclusively at (row 1, col 0): last
        // covered byte is the newline at 0-based (0, 18) — 18-byte line.
        let span = Span::from_native(
            SRC,
            NativePoint { row: 0, column: 0 },
            NativePoint { row: 1, column: 0 },
        );
        assert_eq!((span.start_line, span.start_col), (1, 1));
        assert_eq!((span.end_line, span.end_col), (1, 19));
    }

    /// m4 regression lock: on a CRLF line the `\r` is a covered byte and the
    /// `\n` sits one column further, so a newline-swallowing node's inclusive
    /// end column is content_len + 2 (`#define X 1\r\n` ⇒ end col 13).
    #[test]
    fn crlf_newline_swallowing_node_ends_past_the_carriage_return() {
        let crlf_src: &[u8] = b"#define X 1\r\nint main(void) {\n    return 0;\n}\n";
        let span = Span::from_native(
            crlf_src,
            NativePoint { row: 0, column: 0 },
            NativePoint { row: 1, column: 0 },
        );
        assert_eq!((span.start_line, span.start_col), (1, 1));
        assert_eq!(
            (span.end_line, span.end_col),
            (1, 13),
            "11 content bytes + \\r + \\n ⇒ inclusive end col 13"
        );
        // Round-trip both directions must hold for the CRLF source too.
        let (start, end_exclusive) = span.to_native(crlf_src);
        assert_eq!(Span::from_native(crlf_src, start, end_exclusive), span);
    }

    #[test]
    fn round_trip_holds_both_directions_for_every_span_shape() {
        let cases = [
            ((1u32, 1u32), (1, 1)), // single byte
            ((1, 1), (1, 19)),      // include line incl. newline byte
            ((2, 1), (4, 1)),       // multi-line function
            ((3, 5), (3, 13)),      // mid-line statement
            ((2, 17), (2, 17)),     // degenerate point
        ];
        for &(start, end) in &cases {
            let span = Span {
                start_line: start.0,
                start_col: start.1,
                end_line: end.0,
                end_col: end.1,
            };
            let (native_start, native_end) = span.to_native(SRC);
            assert_eq!(native_start.row, (span.start_line - 1) as usize);
            assert_eq!(native_start.column, (span.start_col - 1) as usize);
            let back = Span::from_native(SRC, native_start, native_end);
            assert_eq!(back, span, "IR → native → IR diverged for {span:?}");
        }
    }

    #[test]
    fn inclusive_span_intersection_semantics() {
        let a = Span {
            start_line: 5,
            start_col: 1,
            end_line: 7,
            end_col: 3,
        };
        assert!(a.intersects(&Span::point(7, 3)), "touching end counts");
        assert!(a.intersects(&Span::point(5, 1)), "touching start counts");
        assert!(a.intersects(&Span {
            start_line: 6,
            start_col: 9,
            end_line: 9,
            end_col: 1
        }));
        assert!(!a.intersects(&Span::point(7, 4)), "just past end does not");
        assert!(!a.intersects(&Span::point(4, 99)));
    }

    #[test]
    fn spans_order_lexicographically_for_determinism_sorts() {
        let early = Span::point(3, 2);
        let late = Span {
            start_line: 3,
            start_col: 10,
            end_line: 3,
            end_col: 12,
        };
        assert!(early < late);
    }
}
