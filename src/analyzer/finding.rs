//! Analyzer-local finding type (modules.md §3.3 output shape).
//!
//! Wave 2 note: this type is deliberately schema-shaped so the aggregator can
//! lift it into `findings[]` document fields without transformation beyond
//! serialization — `evidence` is a `BTreeMap` (sorted keys, byte-determinism
//! per modules.md §6 row 3), enums serialize to their schema tokens
//! (`hot|warn|info`, `HIGH|MEDIUM|LOW`).

use std::collections::BTreeMap;

use crate::ir::{ResolverTier, Span};
use serde::{Deserialize, Serialize};

/// rule_ids of the catalog heuristics (catalog-v1 §3 Tier 1 + §4 Tier 2),
/// one const each so heuristics and tests share a single spelling across
/// the seam.
pub const ALLOC_001_RULE_ID: &str = "PERF-C-ALLOC-001";
pub const ALLOC_002_RULE_ID: &str = "PERF-C-ALLOC-002";
pub const ALLOC_003_RULE_ID: &str = "PERF-C-ALLOC-003";
pub const ARCH_001_RULE_ID: &str = "PERF-C-ARCH-001";
pub const ARCH_002_RULE_ID: &str = "PERF-C-ARCH-002";
pub const ARCH_003_RULE_ID: &str = "PERF-C-ARCH-003";
pub const STR_001_RULE_ID: &str = "PERF-C-STR-001";
pub const STR_002_RULE_ID: &str = "PERF-C-STR-002";
pub const STR_003_RULE_ID: &str = "PERF-C-STR-003";
pub const LOOP_001_RULE_ID: &str = "PERF-C-LOOP-001";
pub const LOOP_002_RULE_ID: &str = "PERF-C-LOOP-002";
pub const LOOP_003_RULE_ID: &str = "PERF-C-LOOP-003";
pub const LOOP_004_RULE_ID: &str = "PERF-C-LOOP-004";
pub const CALL_001_RULE_ID: &str = "PERF-C-CALL-001";
pub const CALL_002_RULE_ID: &str = "PERF-C-CALL-002";
/// Tier 2 rule ids (catalog-v1 §4).
pub const MEM_001_RULE_ID: &str = "PERF-C-MEM-001";
pub const MEM_002_RULE_ID: &str = "PERF-C-MEM-002";
pub const TYPE_001_RULE_ID: &str = "PERF-C-TYPE-001";
pub const TYPE_002_RULE_ID: &str = "PERF-C-TYPE-002";
pub const LOOP_005_RULE_ID: &str = "PERF-C-LOOP-005";
pub const LOOP_006_RULE_ID: &str = "PERF-C-LOOP-006";
pub const LOOP_007_RULE_ID: &str = "PERF-C-LOOP-007";
pub const LOOP_008_RULE_ID: &str = "PERF-C-LOOP-008";

/// Rust (`PERF-RS-*`) rule ids (plan `rust-backend` R3 workstreams C1 + C2),
/// mirroring the C families' intent on Rust idioms. Each const is the single
/// spelling shared by the heuristic and its tests across the seam.
///
/// Coverage is now the 22-rule Rust catalog (STR-001 retired by the M1
/// Precision Gate): the structural subset (LOOP-001/004/005/007/008,
/// CALL-001/002, ARCH-001/002/003, TYPE-001/002, wave C1) plus the API-name
/// families (ALLOC-001/002/003, STR-002/003, LOOP-002/003/006, MEM-001/002,
/// wave C2).
pub const RS_ALLOC_001_RULE_ID: &str = "PERF-RS-ALLOC-001";
pub const RS_ALLOC_002_RULE_ID: &str = "PERF-RS-ALLOC-002";
pub const RS_ALLOC_003_RULE_ID: &str = "PERF-RS-ALLOC-003";
pub const RS_ARCH_001_RULE_ID: &str = "PERF-RS-ARCH-001";
pub const RS_ARCH_002_RULE_ID: &str = "PERF-RS-ARCH-002";
pub const RS_ARCH_003_RULE_ID: &str = "PERF-RS-ARCH-003";
pub const RS_CALL_001_RULE_ID: &str = "PERF-RS-CALL-001";
pub const RS_CALL_002_RULE_ID: &str = "PERF-RS-CALL-002";
pub const RS_LOOP_001_RULE_ID: &str = "PERF-RS-LOOP-001";
pub const RS_LOOP_002_RULE_ID: &str = "PERF-RS-LOOP-002";
pub const RS_LOOP_003_RULE_ID: &str = "PERF-RS-LOOP-003";
pub const RS_LOOP_004_RULE_ID: &str = "PERF-RS-LOOP-004";
pub const RS_LOOP_005_RULE_ID: &str = "PERF-RS-LOOP-005";
pub const RS_LOOP_006_RULE_ID: &str = "PERF-RS-LOOP-006";
pub const RS_LOOP_007_RULE_ID: &str = "PERF-RS-LOOP-007";
pub const RS_LOOP_008_RULE_ID: &str = "PERF-RS-LOOP-008";
pub const RS_MEM_001_RULE_ID: &str = "PERF-RS-MEM-001";
pub const RS_MEM_002_RULE_ID: &str = "PERF-RS-MEM-002";
/// RETIRED by the M1 Precision Gate (plan `evidence-beta`): no emitter is
/// registered and no rule emits this id (Rust tier 23 → 22). Kept as public
/// API — the id stays **reserved** so it is never reused for a different rule
/// — so it is not dead code.
pub const RS_STR_001_RULE_ID: &str = "PERF-RS-STR-001";
pub const RS_STR_002_RULE_ID: &str = "PERF-RS-STR-002";
pub const RS_STR_003_RULE_ID: &str = "PERF-RS-STR-003";
pub const RS_TYPE_001_RULE_ID: &str = "PERF-RS-TYPE-001";
pub const RS_TYPE_002_RULE_ID: &str = "PERF-RS-TYPE-002";

/// Python (`PERF-PY-*`) rule ids (plan `python-backend` W3), the durable
/// disposition table in `docs/architecture/python-backend.md` §9: 13
/// transferred/reframed mirrors of the C/Rust catalog plus 9 Python-native
/// additions. Ten C/Rust ids are RETIRED for Python and deliberately have NO
/// const here (TYPE-001/002, MEM-001, ALLOC-002, STR-002/003, LOOP-004/005/006/
/// 008 — their premises do not transfer to CPython). Each const is the single
/// spelling shared by the heuristic and its tests across the seam.
pub const PY_ALLOC_001_RULE_ID: &str = "PERF-PY-ALLOC-001";
pub const PY_ALLOC_003_RULE_ID: &str = "PERF-PY-ALLOC-003";
pub const PY_ALLOC_004_RULE_ID: &str = "PERF-PY-ALLOC-004";
pub const PY_ARCH_001_RULE_ID: &str = "PERF-PY-ARCH-001";
pub const PY_ARCH_002_RULE_ID: &str = "PERF-PY-ARCH-002";
pub const PY_ARCH_003_RULE_ID: &str = "PERF-PY-ARCH-003";
pub const PY_ASYNC_001_RULE_ID: &str = "PERF-PY-ASYNC-001";
pub const PY_ASYNC_002_RULE_ID: &str = "PERF-PY-ASYNC-002";
pub const PY_CALL_001_RULE_ID: &str = "PERF-PY-CALL-001";
pub const PY_CALL_002_RULE_ID: &str = "PERF-PY-CALL-002";
pub const PY_COMPREHENSION_001_RULE_ID: &str = "PERF-PY-COMPREHENSION-001";
pub const PY_EXECUTOR_001_RULE_ID: &str = "PERF-PY-EXECUTOR-001";
pub const PY_LOOP_001_RULE_ID: &str = "PERF-PY-LOOP-001";
pub const PY_LOOP_002_RULE_ID: &str = "PERF-PY-LOOP-002";
pub const PY_LOOP_003_RULE_ID: &str = "PERF-PY-LOOP-003";
pub const PY_LOOP_007_RULE_ID: &str = "PERF-PY-LOOP-007";
pub const PY_LOOP_009_RULE_ID: &str = "PERF-PY-LOOP-009";
pub const PY_MEM_002_RULE_ID: &str = "PERF-PY-MEM-002";
pub const PY_N1_001_RULE_ID: &str = "PERF-PY-N1-001";
pub const PY_PANDAS_001_RULE_ID: &str = "PERF-PY-PANDAS-001";
pub const PY_PANDAS_002_RULE_ID: &str = "PERF-PY-PANDAS-002";
pub const PY_STR_001_RULE_ID: &str = "PERF-PY-STR-001";

/// JavaScript/TypeScript (`PERF-JS-*`) rule ids (plan `js-ts` B2-W2), the
/// frozen 11-rule disposition table in `docs/architecture/js-ts.md` §8. JS and
/// TS/TSX share one family because the runtime mechanism (V8 / Node) is
/// identical; `PERF-JS-LOOP-003` is CUT for v1 and deliberately has NO const
/// here, and v1 ships **zero** `PERF-TS-*` rules (the B2-W0 canary proved none).
/// Each const is the single spelling shared by the heuristic and its tests
/// across the seam.
pub const JS_ALLOC_001_RULE_ID: &str = "PERF-JS-ALLOC-001";
pub const JS_ALLOC_003_RULE_ID: &str = "PERF-JS-ALLOC-003";
pub const JS_ARCH_001_RULE_ID: &str = "PERF-JS-ARCH-001";
pub const JS_ASYNC_001_RULE_ID: &str = "PERF-JS-ASYNC-001";
pub const JS_ASYNC_002_RULE_ID: &str = "PERF-JS-ASYNC-002";
pub const JS_CALL_001_RULE_ID: &str = "PERF-JS-CALL-001";
pub const JS_LOOP_001_RULE_ID: &str = "PERF-JS-LOOP-001";
pub const JS_LOOP_002_RULE_ID: &str = "PERF-JS-LOOP-002";
pub const JS_LOOP_007_RULE_ID: &str = "PERF-JS-LOOP-007";
pub const JS_STR_001_RULE_ID: &str = "PERF-JS-STR-001";
pub const JS_TYPE_001_RULE_ID: &str = "PERF-JS-TYPE-001";

/// Severity band (report §4 order: hot → warn → info).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Hot,
    Warn,
    Info,
}

/// Confidence tier (report §4 order: HIGH → MEDIUM → LOW).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// One candidate issue. `finding_id` is derived, not stored:
/// `rule:path:start_line:start_col` (modules.md §1 analyzer row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub rule_id: String,
    pub title: String,
    pub message: String,
    pub severity: Severity,
    pub confidence: Confidence,
    /// v1 emits only [`ResolverTier::Syntactic`] (ir-v1.md §2).
    pub resolver_tier: ResolverTier,
    pub span: Span,
    /// Repo-relative POSIX path.
    pub path: String,
    /// Key/value evidence; BTreeMap keeps serialized keys sorted.
    pub evidence: BTreeMap<String, String>,
    /// ir-v1.md §5 hook 1: macro-origin downgrade applied.
    pub macro_suspect: bool,
    /// ir-v1.md §5 hook 3: error-region overlap downgrade applied.
    pub error_node_overlap: bool,
    /// Catalog threshold that fired, as `(name, value)` — e.g.
    /// `("min_nesting_depth", 3)` for LOOP-001. `None` for rules with no
    /// configurable threshold (the schema field is omitted on lift).
    /// Analyzer-internal: never serialized on `Finding` itself — the
    /// aggregator lifts it into `schema::Threshold`.
    #[serde(skip)]
    pub threshold: Option<(&'static str, u32)>,
}

impl Finding {
    /// `finding_id = "rule:path:line:col"` with 1-based start line/col
    /// (modules.md §1: construction & uniqueness owned by the analyzer;
    /// the column disambiguates same-line collisions).
    #[must_use]
    pub fn finding_id(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.rule_id, self.path, self.span.start_line, self.span.start_col
        )
    }
}
