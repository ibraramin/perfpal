//! Perfpal — static-analysis CLI extracting performance-relevant heuristics
//! from C and Rust codebases; emits LLM-consumable metrics (`metrics.json` schema v1),
//! a human report, and deterministic SVG charts.
//!
//! Module layout is contractual: `docs/architecture/modules.md` §7.
//! Pipeline order: cli → scanner → analyzer → aggregator → reporter/charts → writer.

pub mod aggregator;
mod analysis;
pub mod analyzer;
pub mod charts;
pub mod cli;
pub mod graph;
pub mod ir;
pub mod reporter;
pub mod resolver;
pub mod scanner;
pub mod schema;
pub mod writer;
