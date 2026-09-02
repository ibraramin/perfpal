# Perfpal — `stats.distributions` Population Proposal

- **Status:** **Proposed — not accepted.** This document is a design proposal only. It changes no schema file, no code, and no shipped behavior in v1.0.
- **Grounding:** [metrics.schema.v1.json](metrics.schema.v1.json) (`stats.distributions` shape), [metrics-json-v1.md](metrics-json-v1.md) §2/§5 (document structure, versioning policy), [modules.md](../architecture/modules.md) §1 (determinism ownership table), [catalog-v1.md](../heuristics/catalog-v1.md) (LOOP-001, CALL-002 threshold constants), [output-contract.md](../report/output-contract.md) §5 (report template), `src/schema/mod.rs` (`DistributionEntry`), `src/aggregator/mod.rs` (`sort_distributions`, reserved-for-future), `src/reporter/mod.rs` (distribution tables).
- **Audience:** Perfpal implementers and the founder (acceptance decision).

---

## 1. Purpose

`stats.distributions` is schema-sanctioned but unpopulated. The v1.0 schema defines it as an optional array of `{metric, bucket, count}` histogram entries over function-level stats; the typed model (`DistributionEntry`) exists, the aggregator-owned `(metric, bucket)` sort comparator is implemented and unit-tested but deliberately never called ("RESERVED-FOR-FUTURE: no producer populates distributions yet"), and the reporter renders per-metric tables whenever entries are present. The golden report's `metrics.json` contains no `distributions` key; absent is the canonical form. ADR-0002 notes that no v1 chart consumes the section.

This proposal fixes a concrete, deterministic population contract for two function-metric histograms, so that a future producer can land as a single reviewable change with no open design questions.

## 2. Proposed populations

Exactly two populations are proposed:

| `metric` | Source field | Notes |
|---|---|---|
| `cyclomatic_complexity` | `stats.functions[].cyclomatic_complexity` | Required on every entry; integer ≥ 1 (schema minimum). |
| `max_loop_depth` | `stats.functions[].max_loop_depth` | Required on every entry; integer ≥ 0 (schema minimum). |

Rationale:

- Both source fields are **schema-required** on every `stats.functions[]` item, so there are no availability gaps: every function contributes to both histograms.
- Both feed the ARCH-002 component composite (`complexity_sum`, `max_loop_depth`); the histograms expose the raw distribution behind the ranking.
- Both are named in the report template's §3 distributions clause ("complexity buckets, loop depth"), so the existing reporter path renders them without template changes.

Function length — the third family named in output-contract §5 — is deliberately **not** proposed here: its dynamic range is larger and deserves its own edge study (see §8).

## 3. Bucket edges (fixed constants)

### 3.1 `cyclomatic_complexity`

Edges: `[1, 5, 10, 15, 25, 50, ∞)` — buckets `[1,5)`, `[5,10)`, `[10,15)`, `[15,25)`, `[25,50)`, `[50,∞)`.

- **15** is `CALL_002_MIN_COMPLEXITY` (catalog-v1, CALL-002): the `15-24` and upper buckets are exactly the CALL-002-qualifying population, so the histogram directly answers "how many functions already sit at or above the promotion threshold".
- **1** is the schema minimum for the field (every function has complexity ≥ 1).
- **5 / 10 / 25 / 50** give coarse grain below and above the promotion threshold without exploding the bucket count.

### 3.2 `max_loop_depth`

Edges: `[0, 1, 2, 3, 4, 5, ∞)` — one bucket per integer.

- **3** is LOOP-001's threshold (`min_nesting_depth = 3`): the `3`, `4`, `5`, `6+` buckets are exactly the LOOP-001-qualifying zone.
- **0** is the schema minimum (functions containing no loops).
- Per-integer grain because loop depth is small-range in practice; wider buckets would merge the signal the report wants to expose.

## 4. Bucket labeling convention

- Labels are hyphenated inclusive integer ranges — the `"1-10"` style of the schema description's example — with a `+` suffix marking the open-top bucket.
- Lower bounds are zero-padded to a fixed width where a metric's labels mix digit widths. Proposed labels:
  - `cyclomatic_complexity`: `01-04`, `05-09`, `10-14`, `15-24`, `25-49`, `50+`
  - `max_loop_depth`: `0`, `1`, `2`, `3`, `4`, `5`, `6+`
- **Why the padding is load-bearing:** modules.md §1 assigns `stats.distributions[]` ordering to the aggregator as `(metric, bucket)`, and the shipped comparator (`sort_distributions`, `src/aggregator/mod.rs`) orders bucket **strings lexicographically**. Labels must therefore be chosen so lexicographic order equals numeric order. Unpadded labels such as `1-4` / `10-14` would be reordered incorrectly by the unit-locked comparator (`"10-14" < "5-9"` as strings). The proposed padded labels sort correctly under the existing comparator with no code change; the single-digit depth labels need no padding.
- Alternative considered: unpadded human-readable labels plus a numeric-aware comparator. Rejected for this proposal — it requires changing a unit-tested comparator, which this document deliberately does not propose (§8).
- With ascending metric names, the emitted array will list `cyclomatic_complexity` entries before `max_loop_depth` entries (`"c" < "m"` lexicographically), as fixed by the aggregator-owned sort.

## 5. Count semantics

- `count` is a non-negative integer (schema `minimum: 0`).
- Each `stats.functions[]` entry contributes to **exactly one** bucket per metric. Because both source fields are schema-required, for these two metrics the per-metric sum of counts equals `len(stats.functions[])`.
- The **full fixed label set is emitted** for each metric, including zero-count buckets. Fixed buckets keep report tables diffable across runs and make the ordering contract trivially stable; no bucket is ever silently missing.

## 6. Determinism

Histogram construction iterates `stats.functions[]` in the aggregator-owned `(path, span.start_line, span.start_col)` order; counting is order-independent regardless. The only ordering obligation is the existing `stats.distributions[]` → `(metric, bucket)` row in the modules.md §1 determinism table, already owned by the aggregator and proven by unit test. No timestamps, floats, or environment-dependent values are involved; the section introduces no new nondeterminism.

## 7. Versioning & activation

**Activation = MINOR schema bump per docs/schema/metrics-json-v1.md §5 versioning policy; until then `stats.distributions` stays absent/reserved.**

Notes:

- The object shape (`{metric, bucket, count}`) already exists in schema v1.0 as an optional field, so the bump is **procedural** — it formalizes the first producer and this bucket contract — not structural: no fields are added, removed, or retyped.
- Per metrics-json-v1.md §5, the minor bump re-emits the schema file as `metrics.schema.v1.1.json`. Consumers pinning the exact minor schema keep validating: the v1.0 field set is unchanged (the property is already declared), so the §5 strictness caveat about newly added fields does not bite here.
- `report.md` §3 renders the tables automatically through the existing reporter path once entries are present; no report-template change is needed or proposed.
- ADR-0002 records that no v1 chart consumes `stats.distributions`; chart consumption remains out of scope (§8).

## 8. Out of scope

- **No code changes are proposed in this document.** The producer, the named bucket-edge constants, and their tests are deferred to an accepted-implementation plan.
- Function-length distribution — the third family named in output-contract §5 — is left to a separate proposal.
- Chart consumption of distributions (ADR-0002 reserves the section for future minor-bump additions).
- Runtime configuration of bucket edges: if accepted, edges become named constants, consistent with the catalog's threshold-constant policy and the analyzer's "no runtime threshold configuration" non-goal.
