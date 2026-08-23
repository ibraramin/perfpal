# Perfpal — Lessons Learned Log

- **Purpose:** durable record of mistakes, missteps, and issues encountered during development — so future work falls back on them instead of re-learning them.
- **Formal term:** *Lessons Learned Log/Register* (agile cousins: retrospective notes; ITIL cousin: Known Error Database).
- **Rules:** append-only (never delete history); one entry per distinct lesson; **add an entry whenever a mistake costs more than a few minutes or reveals a systemic gap**; consult this log before starting work in a tagged category.
- **Entry format:** ID · date/session/commit · category · what happened · root cause · fix applied · **fallback** (what to do next time) · tags.

---

### LL-001 — Structural edit landed at wrong nesting level
- **When:** session 5 · pre-commit `7ebb81c` (step 3 schema work)
- **Category:** tooling / editing discipline
- **What happened:** Adding an `allOf` block into `$defs.finding` of `metrics.schema.v1.json`, the edit anchored on a generic `"additionalProperties": false` + `"properties": {` pair — which also existed at the document root. Whitespace-flexible matching placed the block at root, where its `if` conditions could never fire (silent dead constraint).
- **Root cause:** Anchor string was structurally common; no post-edit placement check.
- **Fix applied:** Removed misplaced block; re-inserted anchored on unique context (the finding-specific `required` array); added negative tests that would have caught it immediately.
- **Fallback:** When editing structured files (JSON/YAML/code), anchor edits on **multi-line context containing distinctive content**, not generic key pairs. After any structural edit, grep/read to confirm placement, and prefer executable checks over eyeballing.
- **Tags:** editing, json-schema, verification

### LL-002 — Validating a `$def` standalone breaks internal `$ref`s
- **When:** session 5 · step 3 validation
- **Category:** schema-design / testing
- **What happened:** Extracting `$defs.finding` and validating the worked example against it directly threw `PointerToNowhere: '/$defs/span'` — the def's `$ref`s resolve from the document root, which doesn't exist in isolation.
- **Root cause:** JSON Schema `$ref` resolution is document-relative; a subtree is not a self-contained schema.
- **Fix applied:** Validate examples embedded in a minimal full document (or against the whole schema).
- **Fallback:** Any time a sub-schema is tested/reused alone, wrap it or resolve refs explicitly first. Same principle applies to partial configs/modules with internal references.
- **Tags:** json-schema, testing, refs

### LL-003 — Compressed notation defeats machine consumption
- **When:** session 5 · step 3 review fixes
- **Category:** schema-design / documentation
- **What happened:** Rule-ID list written as `PERF-C-ALLOC-001/002/003 … LOOP-001..008` read fine for humans, but individual IDs (`PERF-C-ALLOC-002`) were not literally present — a mechanical assertion failed and LLM/tooling greppability was degraded.
- **Root cause:** Optimizing for human brevity in a field explicitly meant for machine consumption.
- **Fix applied:** Expanded to the full explicit 23-ID enumeration.
- **Fallback:** Anything consumed mechanically (schema descriptions, enums, manifests) gets **explicit, enumerated values**; keep compression for prose only. Write the mechanical check first and let it force explicitness.
- **Tags:** documentation, llm-facing, testing

### LL-004 — Closed enum contradicted forward-compatibility promise
- **When:** session 5 · reviewer MAJOR finding
- **Category:** schema-design / API evolution
- **What happened:** `rule_id` was a closed enum while the versioning policy promised consumers would "tolerate unknown rule_ids" across minor bumps — a v1.1 report with a new ID would fail strict v1.0 validation, breaking the promise.
- **Root cause:** Versioning semantics decided after the shape was fixed.
- **Fix applied:** Switched to an open `pattern` + explicit known-values list in the description; documented strictness caveat.
- **Fallback:** Decide **evolution semantics before freezing shapes**: anything claiming forward-compatibility uses open patterns/optional fields; closed enums are reserved for values guaranteed frozen (confidence/severity/tier).
- **Tags:** versioning, api-design, json-schema

### LL-005 — Long-running phase timed out; retry needed clean seams
- **When:** session 1 · project bootstrap
- **Category:** process / session management
- **What happened:** First bootstrap attempt timed out mid-phase; recovery required redoing discovery before producing artifacts.
- **Root cause:** One monolithic phase with no intermediate checkpoints.
- **Fix applied:** Retry deliberately scoped to research + clarifying questions only, persisting results to state files before proceeding.
- **Fallback:** Split long phases into resumable chunks with state persisted at each seam; on timeout, resume from the last persisted chunk rather than restarting blind.
- **Tags:** process, sessions, resilience

### LL-006 — WATCH: tree-sitter-c C99 grammar vs C11/C17 constructs
- **When:** standing item (opened session 2; carried through steps 2–3)
- **Category:** parsing / monitoring
- **Status:** Not yet a mistake — a tracked watch item. AI-generated C frequently uses `_Generic`, `_Static_assert`, `_Atomic`; the C99-based grammar may raise ERROR-node rates on them.
- **Mitigation so far:** ParseHealth/error-region machinery is first-class in IR v1 and metrics.json v1; findings overlapping error regions are capped at MEDIUM confidence by schema `if/then`.
- **Fallback:** During implementation, measure ERROR-node rates on real AI-generated corpora early; if high, evaluate grammar upgrade (pinned-version policy, ADR-0001 guardrail 5) before blaming heuristics.
- **Tags:** parsing, tree-sitter, monitoring

### LL-007 — Whole-file write targeted the wrong path (CHANGELOG overwritten)
- **When:** session 7 · EOL ritual
- **Category:** process / file operations
- **What happened:** While batching end-of-session writes, a full-file `write` carrying activeContext content was aimed at `CHANGELOG.md`, silently replacing 24 lines of changelog history. Caught on the next tool result and restored verbatim from an earlier read plus the intended additions; net damage zero.
- **Root cause:** Multiple whole-file writes prepared in one batch → path/content pairing not re-checked; overwrite semantics mean a wrong path destroys history with no warning.
- **Fix applied:** Restored CHANGELOG from the session's earlier read + appended new entries; verified via `git diff --stat` (exactly the intended insertions).
- **Fallback:** Never batch multiple whole-file writes; one write per call with an explicit path↔content check. Prefer `edit` (anchored) over `write` (clobbering) for existing files. Before any whole-file overwrite of a tracked file, confirm its current content was read this session so recovery is possible.
- **Tags:** process, file-operations, data-loss
