//! `charts` — deterministic SVG emitter (modules.md §3.7; ADR-0002).
//!
//! Consumes: the final document — graphs.* and components[] ONLY (ADR-0002
//! guardrail 1). Produces: SVG bytes for chart-set v1 (include-graph.svg,
//! component-coupling.svg, hot-components.svg).
//!
//! Responsibilities: byte-deterministic rendering by construction (ADR-0002
//! guardrail 2); XML-escape all label/path text; deterministic node caps with
//! in-chart truncation markers; presence rule — a chart is emitted iff its
//! source section is present and non-empty, otherwise omitted (charts/ may be
//! empty).
//!
//! Non-goals: layout quality beyond "legible" (MVP bar); external
//! charting/layout libraries are banned by ADR-0002.
//!
//! P0 status: scaffold only. Emitter lands in P4.
