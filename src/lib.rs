//! Perfpal — static-analysis CLI extracting performance-relevant heuristics
//! from C codebases; emits LLM-consumable metrics (`metrics.json` schema v1),
//! a human report, and deterministic SVG charts.
//!
//! Module layout is contractual: `docs/architecture/modules.md` §7.
//! Pipeline order: cli → scanner → analyzer → aggregator → reporter/charts → writer.

pub mod aggregator;
pub mod analyzer;
pub mod charts;
pub mod cli;
pub mod ir;
pub mod reporter;
pub mod scanner;
pub mod schema;
pub mod writer;
