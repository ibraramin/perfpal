//! `aggregator` — architecture views & document assembly (modules.md §3.4).
//!
//! Consumes: IR stream + findings[] + provenance from cli. Produces: the
//! complete metrics.json document as schema-typed data (all nine sections:
//! schema_version, tool, scan, parse_health_summary, files, stats, findings,
//! graphs, components).
//!
//! Responsibilities:
//! - graphs.include_graph: file nodes + resolved internal edges.
//! - graphs.call_graph: cross-file name-matched edges, confidence locked LOW
//!   (ARCH-003).
//! - components[]: aggregation by path_prefix incl. finding_density and rank;
//!   sorted (rank, path_prefix).
//! - stats.*: distributions and totals from FileRecord/FunctionRecord fields.
//! - Assemble the document; enforce every sort order in its scope
//!   (Principle 2 ownership table) with determinism tests per row.
//!
//! Non-goals: rendering; interpretation; chart capping decisions.
//!
//! P0 status: scaffold only. Minimal assembly lands in P1; full set in P4.
