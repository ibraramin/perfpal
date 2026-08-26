# Perfpal — Module Architecture

- **Status:** v1 draft — plan `mvp-architecture`, step 6. Defines the module decomposition, plugin boundaries, extension points, and CLI surface for the MVP implementation.
- **Grounding:** [ADR-0001](../adr/ADR-0001-parser-strategy.md) (parser strategy + guardrails), [ADR-0002](../adr/ADR-0002-visualization.md) (chart emitter + guardrails), [ir-v1.md](ir-v1.md) (the IR seam), [output-contract.md](../report/output-contract.md) (owns everything written to disk; §6 delegates the CLI surface here), [metrics-json-v1.md](../schema/metrics-json-v1.md) (external data contract + versioning policy), [catalog-v1.md](../heuristics/catalog-v1.md) (heuristic set + tier/confidence model), spec ["Product Form (decided)"](../specs/perfpal-core-spec.md).
- **Audience:** Perfpal implementers (implementation planning and beyond). This doc constrains *structure and contracts*, not syntax — it is deliberately language-neutral (see §6).

---

## 1. Principles

1. **Seam discipline — two hard import rules.** Heuristics import only the IR module, never parser internals (ADR-0001 guardrail 1). The chart emitter consumes only schema types (`graphs.*`, `components[]`), never source, IR, or scanner state (ADR-0002 guardrail 1). Everything else in this doc exists to keep those two seams clean.
2. **Determinism is owned, not hoped for.** Every ordering obligation has exactly one owner:

   | Obligation | Owner | Rule |
   |---|---|---|
   | File discovery order | scanner | lexicographic sort of discovered paths |
   | `files[]` sorted by path | scanner | output-contract §5 determinism presupposition |
   | Span fidelity | scanner | engine-native → IR conversion locked by unit tests (IR §7) |
   | `finding_id` construction & uniqueness | analyzer | `rule:path:line:col`, column disambiguates collisions |
   | Graph/component iteration order | aggregator | edges by `(from, to)`; components by `(rank, path_prefix)` |
   | `graphs.include_graph.nodes` order | aggregator | nodes sorted by path |
   | `findings[]` array order | analyzer | report §4 total order: severity → confidence → rule_id → path → start_line → start_col |
   | `stats.functions[]` order | aggregator | `(path, span.start_line, span.start_col)` |
   | `stats.distributions[]` order | aggregator | `(metric, bucket)` |
   | Chart bytes | charts module | ADR-0002 guardrail 2 (sorted orders, fixed-point, C locale) |
   | Report sections & sorts | reporter | output-contract §5 total order |
   | Artifact names & layout | writer | output-contract §3 |

3. **Parse honesty.** `ParseHealth` is consulted before any scoring; findings overlapping error regions are downgraded and flagged (`error_node_overlap`), never silently dropped or silently trusted (IR §5, ADR-0001).
4. **Findings, not failures.** Unresolvable includes, include cycles, partial parses, and zero-C-file scans are *data* — they produce findings, flags, or a valid empty report. They never fail the run (exit-code contract, §8).
5. **Derived artifacts are pure functions.** `report.md` and `charts/` are pure functions of the final `metrics.json` document (+ the reporter's frozen built-in catalog for Appendix A descriptions). If any derived artifact disagrees with `metrics.json`, `metrics.json` wins (output-contract Principle 2).
6. **Language-neutral boundaries.** Module boundaries are defined by data contracts, not by an implementation language. Any language satisfying §6's pre-committed criteria can implement every module as specified.
7. **Optimization-candidate framing.** All human-facing text (report prose, stdout summary) frames outputs as candidates for review, never certainties.

## 2. Pipeline overview

Single process, single pass per scan; stages communicate in memory; nothing persists between stages except the final report directory.

```
                ┌─────────────────────────────────────────────────┐
                │                       cli                       │
                │   args · provenance · orchestration · exit codes│
                └───────┬─────────────────────────────────────────┘
                        │ target path, options, provenance
                        ▼
                ┌───────────────┐    IR stream     ┌───────────────┐
                │    scanner    │ ───────────────▶ │   analyzer    │
                │ (backend:     │                  │   heuristic   │
                │ tree-sitter-c)│                  │   registry    │
                └───────┬───────┘                  └──────┬────────┘
                        │ IR stream                       │ findings[]
                        ▼                                 ▼
                ┌─────────────────────────────────────────────────┐
                │                    aggregator                   │
                │      graphs · components · stats · assemble     │
                └──────────────────────┬──────────────────────────┘
                                       │ metrics.json document (schema types)
                       ┌───────────────┼────────────────┐
                       ▼               ▼                ▼
                ┌───────────┐   ┌───────────┐   ┌─────────────┐
                │ reporter  │   │  charts   │   │   writer    │
                │ report.md │   │  *.svg    │   │ atomic dir  │
                └─────┬─────┘   └─────┬─────┘   │  swap       │
                      │ bytes         │ bytes   └────▲────────┘
                      └───────────────┴──────────────┘
```

Stage order is fixed: scanner → analyzer → aggregator → render (reporter ∥ charts) → writer. The renderer fan-out is conceptually parallel; sequential execution is equally correct (determinism makes concurrency optional, never required).

## 3. Module contracts

Each module lists Consumes / Produces / Responsibilities / Non-goals.

### 3.1 `cli` — entry point & orchestration

- **Consumes:** argv (env: none in v1).
- **Produces:** validated invocation options; scan provenance (`started_at`, `root_path` verbatim, tool version); process exit code; stdout/stderr text (§8).
- **Responsibilities:** parse and validate arguments; run stages in pipeline order; collect wall-clock duration; map stage failures to exit codes; print the stdout summary.
- **Non-goals:** analysis logic; config files; direct filesystem inspection beyond existence/readability checks needed for argument validation.

### 3.2 `scanner` — discovery, parsing, IR emission

- **Consumes:** target path; invocation options.
- **Produces:** the complete IR stream (file-level records, FunctionRecords, events — IR §3) in deterministic order.
- **Responsibilities:**
  - Recursive file discovery with lexicographically sorted emission; hidden directories (dot-prefixed) are skipped and symlinks are not followed in v1; per-file read/parse failures become `ParseHealth{failed}` records, never run aborts (total discovery failure of the target itself is a usage error, exit 1).
  - Language detection by extension: `.c` and `.h` are C in v1; other extensions are skipped silently (the v1 schema has no field for skipped-file counts). `scan.file_count` = number of scanned C files = `len(files[])`; a zero-C-file scan still records `scan.languages: ["c"]` (schema `minItems: 1`).
  - Parse via the bound backend; capture `ParseHealth` (status, error regions, error node count).
  - Quoted-include resolution against the filesystem (includer-relative first); angle-bracket includes bucketed external (ADR-0001); unresolvable quoted includes emit `IncludeEdge{resolved: false}` — ARCH-001 signal, not an error.
  - Curated macro-pattern recognizer pass → `MacroEvent`s.
  - Engine-native span → IR span conversion at this boundary exclusively (+1 on entry, −1 on exit; IR §2).
- **Non-goals:** semantic resolution; macro expansion; build-flag/compile-db awareness; scoring.

### 3.3 `analyzer` — heuristic registry execution

- **Consumes:** IR stream (read-only).
- **Produces:** `findings[]` in the schema's shape, emitted in the report §4 total order (severity → confidence → rule_id → path → start_line → start_col) so document order equals report order.
- **Responsibilities:** execute every registered heuristic over the IR; enforce ParseHealth gating and all three downgrade hooks (macro origin, cross-file name-based edges LOW, error-region overlap — IR §5); construct `finding_id` values; enforce uniqueness — two findings with identical `(rule_id, path, start_line, start_col)` MUST NOT exist; the registry collapses such duplicates deterministically (first in execution order; heuristics run in ascending `rule_id`) and CI asserts uniqueness.
- **Non-goals:** graph/component building; stats assembly; rendering; runtime threshold configuration (thresholds are named constants documented in the catalog).

### 3.4 `aggregator` — architecture views & document assembly

- **Consumes:** IR stream + `findings[]` + provenance from `cli`.
- **Produces:** the complete `metrics.json` document as schema-typed data (all nine sections: `schema_version`, `tool`, `scan`, `parse_health_summary`, `files`, `stats`, `findings`, `graphs`, `components`).
- **Responsibilities:**
  - `graphs.include_graph`: file nodes + resolved internal edges (quoted includes resolving in-repo; angle-bracket includes resolving to in-repo files per ADR-0002 chart policy). Founder decision (P4): unresolved QUOTED edges are INCLUDED in metrics.json (`to: null`, `resolved: false` — schema-sanctioned "a finding, not a failure"); angle-unresolved edges stay excluded; the SVG chart remains resolved-internal-only per ADR-0002.
  - `graphs.call_graph`: cross-file name-matched edges, confidence locked LOW (ARCH-003).
  - `components[]`: aggregation by `path_prefix` including `finding_density` and `rank`; sorted `(rank, path_prefix)`.
  - `stats.*`: distributions and totals from FileRecord/FunctionRecord fields.
  - Assemble the document; enforce every sort order in its scope (Principle 2 table).
- **Non-goals:** rendering; interpretation; chart capping decisions (owned by charts).

### 3.5 `schema` — metrics.json model & serializer

- **Owns:** the metrics.json document types; the serializer; the embedded canonical JSON Schema artifact (byte-copy source for the bundled `metrics.schema.v<MAJOR>.<MINOR>.json`); the `SCHEMA_VERSION` constant.
- **Contract:** other modules depend on these types only — never on ad-hoc JSON maps. Version bumps follow the metrics-json-v1 policy; minor = additive, major = breaking.

### 3.6 `reporter` — human-readable rendering

- **Consumes:** the final document.
- **Produces:** `report.md` bytes.
- **Responsibilities:** render the fixed section template (output-contract §5); cite findings by exact `finding_id`; escape all embedded paths/titles/messages (Markdown-injection rule); apply optional-field `n/a`/omit rules; Appendix A descriptions from the built-in catalog (frozen v1).
- **Non-goals:** reading source, IR, or scanner state; inventing aggregations beyond the template.

### 3.7 `charts` — SVG emitter

- **Consumes:** the final document — `graphs.*` and `components[]` only (ADR-0002 guardrail 1).
- **Produces:** SVG bytes for chart-set v1 (`include-graph.svg`, `component-coupling.svg`, `hot-components.svg`).
- **Responsibilities:** byte-deterministic rendering (ADR-0002 guardrail 2); XML-escape all label/path text; deterministic node caps with in-chart truncation markers; presence rule — a chart is emitted iff its source section is present and non-empty, otherwise omitted (`charts/` may be empty).
- **Non-goals:** layout quality beyond "legible" (MVP bar); consuming anything but schema types.

### 3.8 `writer` — atomic output management

- **Consumes:** staged artifact bytes (`metrics.json`, bundled schema copy, `report.md`, `charts/*.svg`).
- **Produces:** the report directory, per output-contract §2.
- **Responsibilities:** build in a temporary sibling directory; swap via rename-aside-then-swap; safety guard (refuse replacing a directory that lacks `metrics.json` unless forced); remove stale temp directories from crashed runs on the next successful run; enforce encoding (UTF-8, LF, trailing newline).
- **Failure behavior:** any upstream failure aborts before the swap — the previous complete report survives intact; interrupted runs never leave half-written reports.

## 4. Plugin boundaries

Three registries make the MVP extensible without touching pipeline contracts.

### 4.1 Heuristic registry

Each heuristic is a pure function over IR views:

- **Input:** read-only access to FunctionRecords + their events, file-level records, and ParseHealth — nothing else (ADR-0001 guardrail 1).
- **Output:** zero or more candidate findings carrying `rule_id`, span, evidence, proposed severity/confidence.
- **Metadata:** stable catalog ID (`rule_id`), resolver-tier tag, named threshold constants (documented in catalog-v1).
- **Purity rules:** no I/O, no clock, no global mutable state, no dependence on sibling-heuristic execution order; MUST consult ParseHealth before scoring; MUST apply the downgrade hooks (IR §5).
- **Registration:** additive. Adding a heuristic introduces a new `rule_id` (open pattern per the schema versioning policy) — at most a minor bump. CI must run the full suite against fixture IR with no parser bound (ADR-0001 guardrail 2).

### 4.2 Scanner backends (language frontends)

A backend implements one interface: *given (path, source bytes), emit that language's IR records/events.*

- v1 ships exactly one backend: `tree-sitter-c` (official grammar, MIT, pinned version per ADR-0001 guardrail 5).
- Backends convert spans at the boundary and never leak node kinds past it.
- New languages = new backends + grammar vendoring per the ADR-0001 license policy (official `tree-sitter-*` repos unless individually audited). No analyzer, aggregator, or reporter changes required — see §5.2 for the schema caveat.

### 4.3 Resolvers (future)

Resolvers sit between scanner and analyzer: they consume the IR stream and enrich it (types, true scopes, resolved callees), raising tier tags and confidence. They never change identity keys `(path, name, span)` or record shapes (IR §6). The semantic (libclang) and dynamic (runtime-profile) tiers both plug in here — one seam, three sources (ADR-0001).

## 5. Extension points (explicitly not built in v1)

### 5.1 Runtime-profile ingestion (dynamic tier)

Profile adapters (perf / valgrind / cProfile) translate tool-specific output into dynamic-tier events keyed by the same `(path, name, span)` identity. Merge happens at the IR level; the analyzer treats dynamic evidence as additional input to severity/confidence (upgrade semantics deferred to the ingestion design). Because identity is parser-agnostic (ADR-0001), ingestion bolts on without rewriting scanner, analyzer, or schema — the commitment CONFIRMED-1 requires.

### 5.2 Additional languages (C++, Python next)

Add a scanner backend + vendored grammar. Both extension costs are already priced into the schema: the `rule_id` pattern is open (future-language IDs validate without breaking strict v1.x validation), and the language constants widen as a **minor-bump schema re-emission** — see `scan.languages` / `files[].language` in [metrics.schema.v1.json](../schema/metrics.schema.v1.json) and the minor-bump policy in [metrics-json-v1.md §5](../schema/metrics-json-v1.md). No major bump is required merely to add a language.

### 5.3 Semantic resolver

libclang joins as an additive resolver per ADR-0001 (deferred, license-clean, viable). Enrichment raises confidence/tier; identity and schema unchanged.

## 6. Implementation-language fork — RESOLVED: Rust

**Decision (founder, 2026-08-23): Rust.** The fork was deferred to implementation planning (superseding ADR-0002's pointer) and is now closed there against the pre-committed criteria below. All modules above remain specified language-neutrally; the choice constrains syntax and tooling only.

Pre-committed evaluation criteria (any candidate must satisfy all of them; recording them up front made the decision mechanical rather than re-litigated):

1. **Self-contained distribution** — single static binary; end users install nothing else (spec Product Form; ADR-0002 constraint 3). Interpreted runtimes qualify only if frozen/embedded into the shipped binary.
2. **First-class tree-sitter integration** — official or ecosystem-standard bindings; hand-rolled FFI is a disqualifier for MVP velocity.
3. **Byte-determinism suitability** — no framework-level nondeterminism leaking into output bytes (unseeded map iteration, default timestamps/uuids in serializers).
4. **No copyleft achievable** — full transitive closure auditable against the license policy.
5. **Cross-platform builds** — Linux/macOS/Windows from one toolchain.
6. **Dev velocity on string/graph/JSON-heavy code** — Perfpal is a text-munging tool, not a kernel; heavy manual memory management is a real cost.

### Outcome (fold-in, founder-approved 2026-08-23)

| Criterion | Verdict | Basis |
|---|---|---|
| 1. Self-contained distribution | PASS | musl fully-static release builds; no runtime install for end users |
| 2. First-class tree-sitter | PASS | official org-published crates: `tree-sitter` v0.26.x + `tree-sitter-c` v0.24.x (both MIT) — the reference binding |
| 3. Byte-determinism suitability | PASS | `serde_json` object keys serialize sorted by default (`BTreeMap` backing; `preserve_order` feature must remain OFF — pinned at scaffold and guarded in CI); struct fields in declaration order |
| 4. No copyleft achievable | PASS | core crates audited `MIT OR Apache-2.0` (serde, serde_json, petgraph, clap, anyhow, thiserror); tree-sitter runtime + grammar MIT |
| 5. Cross-platform builds | PASS | cargo + `cc`-crate vendored C compiles per-target; cargo-zigbuild covers linux-x86_64/aarch64-musl, macOS x86_64/aarch64, Windows MSVC; existence proof: ast-grep ships this exact matrix on Rust+tree-sitter |
| 6. Dev velocity (string/graph/JSON) | PASS | petgraph / serde_json / clap cover the text-munging workload |

**Rejected:** Go — official `go-tree-sitter` is cgo-bound (CGO_ENABLED=0 impossible → per-target C cross-toolchain matrix), failing criteria 1+5 via criterion 2's interaction; community smacker binding stale >2 yrs. C — fails criterion 6 outright (manual memory management for string/graph/JSON is the named cost) and makes 1+5 manual labor. Python — remains ruled out under criterion 1; no frozen-binary exception sought.

**Consequences:** repository layout §7 maps to a single Cargo crate with module-per-directory under `src/`; toolchain = stable Rust pinned in CI; CI adds a license audit of the full dependency tree (criterion 4 is a standing obligation, not a one-time check).

## 7. Repository layout (Rust)

Directory names are contractual; nesting below them is not.

```
perfpal/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── cli/            # §3.1
│   ├── scanner/        # §3.2 — discovery, resolution, recognizer pass
│   │   └── backends/tree-sitter-c/
│   ├── ir/             # the seam: record/event/shape types (IR v1)
│   ├── analyzer/       # §3.3 — registry + the 23 catalog heuristics
│   ├── aggregator/     # §3.4
│   ├── schema/         # §3.5 — document model, serializer, embedded JSON Schema
│   ├── reporter/       # §3.6
│   ├── charts/         # §3.7 — SVG emitter
│   └── writer/         # §3.8
├── tests/
│   ├── fixtures/ir/        # parser-free fixture corpus (ADR-0001 guardrail 2)
│   ├── fixtures/metrics/   # documents for byte-determinism tests (ADR-0002 guardrail 6)
│   └── golden/report/      # golden report-directory comparisons
├── docs/                   # existing design artifacts (this tree)
├── state/                  # session memory (never shipped)
└── CHANGELOG.md
```

Required test structure follows from the guardrails: fixture-IR heuristic suite (no parser bound), span round-trip unit tests, ParseHealth gating tests (IR §7), double-render byte-equality for SVGs, golden-directory tests for the writer's swap behavior, plus: `finding_id` uniqueness/collision tests, aggregator sort-order determinism tests (Principle 2 table rows), `report.md` double-render byte-equality modulo provenance fields (`started_at`, `duration_ms`), CLI exit-code mapping tests (0/1/2/3), and the writer safety-guard refusal path.

## 8. CLI surface

Owned here per output-contract §6. `metrics.json` remains the machine contract; stdout is explicitly **not** a stability contract — only the report directory is.

### 8.1 Flags (MVP)

| Flag | Argument | Default | Meaning |
|---|---|---|---|
| `<path>` | positional | `.` | scan target (directory or single file) |
| `-o`, `--output` | directory | `perfpal-report/` | report directory; internal layout fixed by output-contract §2 |
| `--force` | — | off | allow replacing an existing directory that contains no `metrics.json` (safety-guard override) |
| `--version` | — | — | print `perfpal <version> (metrics schema <schema_version>)`, exit 0 |
| `-h`, `--help` | — | — | usage text, exit 0 |

Explicitly out of MVP (post-MVP list): include/exclude path filters, config files, parallelism knobs, `--json` stdout mode, watch mode, diff mode.

### 8.2 Exit codes

| Code | Meaning | Examples |
|---|---|---|
| `0` | Success — report written. Findings never affect the exit code (they are data, Principle 4). Zero-C-file scans succeed with a valid empty report. | |
| `1` | Usage error | unknown flag; missing/unreadable target path |
| `2` | Output error | cannot create output dir; safety-guard refusal without `--force`; disk/write failure |
| `3` | Internal error | unexpected panic/crash — always a bug |

### 8.3 stdout / stderr

stdout carries a short plain-text human summary (illustrative, non-normative):

```
scanned 214 files (210 ok, 3 partial, 1 failed) · 57 findings (3 hot, 21 warn, 33 info)
report: perfpal-report/
```

All diagnostics, warnings, and progress go to stderr. No color/cursor tricks in v1 — output stays pipe-clean. Machine consumption starts at `metrics.json`, never stdout parsing.
