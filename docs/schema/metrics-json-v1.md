# Perfpal — metrics.json v1: Design & Stability Contract

- **Status:** v1 draft — plan `mvp-architecture`, step 3. Formal schema: [`metrics.schema.v1.json`](./metrics.schema.v1.json) (JSON Schema Draft 2020-12). Internal representation: [`docs/architecture/ir-v1.md`](../architecture/ir-v1.md). Heuristic IDs: [`docs/heuristics/catalog-v1.md`](../heuristics/catalog-v1.md).
- **Audience:** consumers of Perfpal output — humans, tooling, and any LLM. The schema is deliberately **LLM-agnostic**: no vendor fields, no prompt-specific tuning (spec CONFIRMED-4).

---

## 1. Design goals

1. **Self-describing.** Every field carries a `description` in the schema; enums are closed and named. An LLM given only this document + a report can interpret it correctly.
2. **Honest uncertainty.** Confidence and severity are mandatory on every finding (catalog downstream contract); findings are framed as *optimization candidates*, never certainties.
3. **Stable identity.** Heuristic IDs (`PERF-C-*`) are frozen by the signed-off catalog; entity identity follows the ADR-0001 `(path, name, span)` keying — never engine-specific keys.
4. **Stats ≠ findings.** Baseline measurements (catalog §2) carry no severity/confidence and live in `stats`/`files`; only genuine heuristic detections appear in `findings`.

## 2. Document structure

| Section | Purpose | Feeds |
|---|---|---|
| `schema_version`, `tool`, `scan` | Provenance: which schema/binary scanned what, when | Everything |
| `parse_health_summary` *(optional)* + per-file `parse_health` | Parse quality; ERROR regions reported, never analyzed as fiction | Confidence downgrades |
| `files[]` | Per-file baseline stats (LOC, counts) | Pillar 1a |
| `stats.functions[]`, `stats.distributions[]` | Per-function measurements (complexity, loop depth, alloc sites, length) | Pillar 1a, ARCH-002 |
| `findings[]` | Heuristic detections with confidence/severity/evidence | Human report, LLM raw data |
| `graphs.include_graph`, `graphs.call_graph` *(optional)* | Architecture-level structure (quoted vs angle includes; cross-file name-based call edges) | Pillar 1b visualization, ARCH-001/003 |
| `components[]` *(optional)* | Hot-component ranking (ARCH-002 aggregation) | Bottleneck maps |

Only `schema_version`, `tool`, `scan`, `files`, `stats`, `findings` are schema-required; the optional sections may be empty objects/arrays on trivial scans but the fields exist so consumers can rely on their shape when present.

## 3. Enums and their semantics

- **`confidence`: HIGH / MEDIUM / LOW** — catalog §1 model. Mandatory downgrades encoded as flags:
  - `macro_suspect: true` ⇒ confidence capped at MEDIUM (span originates from macro expansion).
  - `error_node_overlap: true` ⇒ confidence capped at MEDIUM (C11/C17 grammar-gap monitor).
  - Cross-file name-based edges (`call_graph`, ARCH-003) are LOW until a semantic resolver exists.
- **`severity`: hot / warn / info** — prioritization hint only: hot = review first; warn = schedule; info = context/aggregation feed. Never a certainty claim.
- **`resolver_tier`: syntactic / semantic / dynamic** — which resolver tier served the record (ADR-0001). MVP emits `syntactic` exclusively; the field exists so tier upgrades never change the schema.

## 4. Finding identity

`finding_id = "{rule_id}:{path}:{start_line}:{start_col}"` (e.g. `PERF-C-ALLOC-001:src/net/conn.c:142:9`). The column disambiguates two findings sharing a line; parse by splitting from the RIGHT, since POSIX paths may legally contain colons. Deterministic within a report for dedupe and cross-referencing from `report.md` (step 4). IDs are **not** stable across scans of changed code — re-running on modified sources legitimately yields different locations.

## 5. Versioning & stability policy

Semver-inspired `MAJOR.MINOR`:

- **Minor bumps (v1.x): additive only.** New optional fields, new `rule_id`s (the schema uses an open `pattern`, not a closed enum — new heuristics/languages never break strict validation), new evidence keys, new optional graph/component fields. The schema file is re-emitted per minor version (`metrics.schema.v1.1.json`) so validation stays exact.
- **Major bumps (v2+): anything breaking**, explicitly including:
  - removing or renaming fields;
  - changing the **closed enums** (`confidence`, `severity`, `resolver_tier`) in either direction;
  - adding **required** fields anywhere;
  - widening the `threshold.value` type set;
  - changing span conventions or identity formats.
- **Consumer obligations:** tolerate unknown `rule_id`s and unknown `evidence` keys gracefully (forward compatibility within v1); pin the exact minor schema they validate against.
- **Rule of thumb:** if an old consumer reading a new report can misinterpret data → major bump; if it can only miss new information → minor bump.
- **Strictness caveat:** strict validation against an *older* minor schema is guaranteed only until that minor's field set changes — i.e., a v1.0 validator may reject a v1.1 report containing newly added fields. Consumers wanting forward tolerance should validate structurally (required fields + types) rather than with `additionalProperties: false`.

**Minor bump v1.0 → v1.1 (2026-09-02, plan `semantic-resolver` Wave A).** Two additive changes, both backward-compatible (v1.0 documents validate unchanged under the v1.1 schema):

1. **`files[].parse_health.resolver`** — new OPTIONAL field, enum `absent | ok | failed`, recording the semantic (libclang) resolver's per-file outcome (modules.md §4.3). Omitted entirely by producers without a resolver outcome (so v1.0-style `parse_health` objects validate unchanged); `absent` = resolver disabled/unavailable; `ok` = libclang parsed the translation unit; `failed` = libclang reported a TU-level failure. Never deletes or degrades syntactic results.
2. **`graphs.call_graph.edges[].confidence`** — widened from `const "LOW"` to oneOf `["LOW", "MEDIUM", "HIGH"]`. Widening is additive: every v1.0 document (all-LOW) still validates. Wave A producers still emit only `"LOW"`; the semantic tier's unambiguous-resolution upgrades consume the widened set in Wave B (ADR-0001 conservative-upgrade rule).

The schema artifact was re-emitted as `metrics.schema.v1.1.json` (the v1.0 copy, `metrics.schema.v1.json`, is kept as history); the bundled artifact shipped in `perfpal-report/` carries the full v1.1 name.

## 6. Worked example

```json
{
  "finding_id": "PERF-C-ALLOC-001:src/net/conn.c:142:9",
  "rule_id": "PERF-C-ALLOC-001",
  "title": "Heap allocation inside loop",
  "path": "src/net/conn.c",
  "symbol": "process_packets",
  "span": { "start_line": 142, "start_col": 9, "end_line": 142, "end_col": 41 },
  "confidence": "MEDIUM",
  "severity": "hot",
  "resolver_tier": "syntactic",
  "macro_suspect": true,
  "error_node_overlap": false,
  "message": "malloc is called inside the loop beginning at line 138; per-iteration allocation is an optimization candidate (allocator churn dominates runtime).",
  "evidence": { "callee": "malloc", "loop_depth": 2 }
}
```

Note the downgrade in action: allocation call reached through a wrapper macro ⇒ `macro_suspect=true`, confidence MEDIUM instead of the rule's HIGH default. Optional fields (`symbol`, `threshold`, `evidence`) are simply **omitted** when they don't apply — explicit `null`s are not valid for object-typed optionals like `threshold`.

## 7. LLM-consumption guidance (embedded intent)

- Sort/triage by `severity` then `confidence`; treat LOW-confidence architecture edges (ARCH-003) as hypotheses to verify, not facts.
- Use `evidence` for programmatic reasoning; use `message` for human-facing explanation.
- Respect parse health: findings overlapping error regions are already flagged; do not invent findings for unparseable code.
- Aggregate before acting: single `info` findings are weak; density patterns across `components[]` are strong.

## 8. Deliberate non-goals in v1

- No per-finding fix suggestions (LLM's job, not ours).
- No intra-file call graph (noise without semantic resolution).
- No diffing/history semantics (post-MVP concern).
- No vendor/model-specific output variants (CONFIRMED-4).
