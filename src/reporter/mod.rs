//! `reporter` — human-readable rendering (modules.md §3.6).
//!
//! Consumes: the final document. Produces: report.md bytes.
//!
//! Responsibilities: render the fixed section template (output-contract §5);
//! cite findings by exact finding_id; escape all embedded paths/titles/
//! messages (Markdown-injection rule); apply optional-field n/a-or-omit rules;
//! Appendix A descriptions from the built-in catalog (frozen v1).
//!
//! Determinism: double-render byte-equality modulo provenance fields
//! (`started_at`, `duration_ms`) is a standing test obligation.
//!
//! Non-goals: reading source, IR, or scanner state; inventing aggregations.
//!
//! P0 status: scaffold only. Template lands in P4.
