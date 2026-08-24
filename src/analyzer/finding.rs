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

/// rule_ids of the Tier 1 heuristics (catalog-v1 §3), one const each so
/// heuristics and tests share a single spelling across the seam.
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
