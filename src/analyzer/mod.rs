//! `analyzer` — heuristic registry execution (modules.md §3.3).
//!
//! Consumes: IR stream (read-only). Produces: `findings[]` in the report §4
//! total order (severity → confidence → rule_id → path → start_line →
//! start_col) so document order equals report order.
//!
//! Responsibilities: execute every registered heuristic over the IR; enforce
//! ParseHealth gating and all three downgrade hooks (macro origin, cross-file
//! name-based edges LOW, error-region overlap); construct finding_id values;
//! enforce uniqueness — identical `(rule_id, path, start_line, start_col)`
//! MUST NOT exist; registry collapses duplicates deterministically (first in
//! execution order; heuristics run in ascending rule_id).
//!
//! Purity rules (modules.md §4): heuristics are pure IR-view functions — no
//! I/O, no clock, no global state, no order dependence. Parser-free testing
//! against tests/fixtures/ir/ per ADR-0001 guardrail 2.
//!
//! Non-goals: graph/component building; stats assembly; rendering; runtime
//! threshold configuration.
//!
//! P0 status: scaffold only. Registry + ALLOC-001 land in P1–P2.
