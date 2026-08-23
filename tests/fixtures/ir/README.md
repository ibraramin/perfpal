# Fixture-IR corpus (parser-free)

Per ADR-0001 guardrail 2, heuristics are tested against serialized IR
documents in this directory — **no tree-sitter binding is required** to run
those tests. This keeps the analyzer test suite fast, deterministic, and
independent of grammar version drift.

## Format

Each file is one serialized IR v1 document (shapes per `docs/architecture/ir-v1.md` §3).
Strict typing arrives with the IR types in P1; until then the harness
(`tests/common/mod.rs`) treats fixtures as `serde_json::Value`.

Naming: `<RULE-ID>__<scenario>.json` for heuristic fixtures
(e.g. `ALLOC-001__malloc-in-for-loop.json`); free-form names for pipeline
fixtures.

## Conventions

- Spans are 1-based lines/byte-columns, ends INCLUSIVE (ir-v1.md §2).
- Fixtures carry ParseHealth fields so gating and downgrade hooks
  (macro_origin, error_node_overlap) are exercisable without a parser.
- Byte-determinism applies here too: any fixture re-emitted by the tool must
  round-trip byte-identically (`tests/harness_smoke.rs` guards the mechanics).
