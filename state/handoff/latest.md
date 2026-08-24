# Handoff — Latest

## Objective
Execute plan `mvp-implementation` phase P2 Tier 1 wave A: ALLOC-002, STR-001, STR-002, STR-003, LOOP-001, LOOP-003, CALL-001 as pure IR-view functions, parser-free fixture tests, ParseHealth gating + downgrade hooks applied, ascending rule_id execution order. **DONE — dual-review passed (requirements items resolved; reviewer SHIP-WITH-FIXES with zero Critical/MAJOR, both MINORs fixed), 89 tests green.**

## Phase
executing mvp-implementation — P2 Tier 1 wave A COMPLETE; next session starts **P3 Tier 1 wave B**

## Key Decisions
- **Zero extraction growth:** P1 backend already emitted every event type the 7 rules consume (calls+args+roles, loops+depth, assignments, Shape literals) — all rules are analyzer-side only.
- **REGISTRY reordered to true lexicographic ascending rule_id** `[ALLOC-001, ALLOC-002, CALL-001, LOOP-001, LOOP-003, STR-001, STR-002, STR-003]` — declaration order is load-bearing (first-wins dedup); doc comment lists entries.
- **Heuristics module split:** inline mod → `src/analyzer/heuristics/{mod,alloc,str,loop,call}.rs` (`loop.rs` declared `mod r#loop`); heuristic fns `pub(crate)` + re-exported `pub(super)` (Rust visibility necessity).
- **ALLOC-002 = free-only** (`fclose` deferred to semantic tier — heap-backedness not syntactically decidable; documented in alloc.rs rustdoc + catalog-v1.md implementation-note line). Co-occurrence escalation Warn→Hot when same function has an alloc-in-loop site (+`alloc_cooccurrence` evidence).
- **STR-002 suppression:** innermost enclosing LoopNode = max depth among containing loops, earliest-span tie-break; assignment to the identifier inside that span suppresses; function-span fallback.
- **LOOP-001 fires only at depth == exactly 3** — one finding per maximal nest entry, `max_depth` evidence over contained loops (avoids 3 findings for one depth-5 chain).
- **CALL-001 presence-only:** one finding per function at earliest Direct self-call site; Indirect excluded.
- **STR-003 not loop-gated** (unbounded op anywhere); scanf family matched via `%s` in any quote-stripped string-literal arg; messages framed "optimization candidate", never "bug".
- **strcat-in-loop intentionally double-fires STR-001 + STR-003** (distinct signals; dedup key permits coexistence) — founder-approved.
- **Downgrade hooks stayed central** — new rules inherit macro_origin/error-region handling free; fixtures prove hooks fire on str/loop/call families (`PERF-C-CALL-001__downgrade-hooks.json` exercises both hook paths independently).

## Reviews
- @requirements-reviewer: **CHANGES_REQUESTED → resolved.** All verbatim clauses PRESENT with evidence, zero scope creep; two founder-approved items were incomplete and got fixed: (B2) fclose deferral undocumented durably → rustdoc + catalog annotation added; (B5) call family lacked downgrade-hook proof → new fixture + test added.
- @reviewer: **SHIP-WITH-FIXES, zero Critical/MAJOR.** Independently verified gates incl. stronger `clippy --all-targets`; confirmed both B-fixes sound with arithmetic checks. 2 MINORs fixed: vacuous negative assertion (range guard couldn't fail for its stated target) replaced with explicit line-34 LOOP-003 absence check; kitchen-sink synthetic unit `in_loop_depth` 1→3 consistency. Deferred nit-grade: loop_003 defensive in_loop_depth guard; alloc_001 hardcoded Syntactic tier vs siblings copying event tier; str_003 scanf scans all args for %s instead of format position; is_strlen_in_loop duplication; rustdoc backtick nesting; README stale wording.

## Risks
- cargo NOT on PATH in fresh shells (`rustup --no-modify-path`): prefix commands with `export PATH="$HOME/.cargo/bin:$PATH"`.
- STR-002 suppression relies on plain AssignmentEvent capture — compound ops / pointer writes invisible to syntactic tier (mutated-in-loop strlen may still fire); catalog footnote candidate.
- tree-sitter-c C99 grammar vs C11/C17 ERROR-rate risk unchanged; ParseHealth downgrade hook covers it.
- Writer crash window + concurrent staging-dir sweep risks unchanged (P6 hardening).
- Two transient provider network errors hit reviewer dispatch this session (rev-1/rev-2); third attempt succeeded — retry-on-network-error is the working pattern.

## Blockers
- None.

## Next Actions
1. Founder approves commit → fresh session (`/new`) starts `mvp-implementation` **P3 Tier 1 wave B**: ALLOC-003 (realloc linear growth), LOOP-002 (setup/I-O in loop, severity lowered when conditional_guarded), LOOP-004 (variable-divisor div/mod innermost — uses FIXED is_innermost), CALL-002 (cyclomatic complexity ≥15 promoted from stat). Resume with: plan mvp-implementation, commit \<P2 hash\>.
2. P3 implementer notes: BinaryOpEvent already carries op/left/right (LOOP-004 data ready); CallEvent.conditional_guarded already extracted (LOOP-002 lowering hook); FunctionRecord.cyclomatic_complexity already computed (CALL-002 is stat-promotion). Consider unifying AllocCandidate resolver_tier while touching the alloc family.

## Uncommitted Files (pending founder-approved commit)
- New: `src/analyzer/heuristics/{mod,alloc,str,loop,call}.rs`, `tests/wave_a_fixtures.rs`, `tests/fixtures/ir/PERF-C-{ALLOC-002__free-in-loop,ALLOC-002__free-not-in-loop,STR-001__strcat-in-loop,STR-001__strcat-not-in-loop,STR-002__strlen-in-loop,STR-003__unbounded-family,STR-003__macro-origin,LOOP-001__deep-nest,LOOP-001__error-region-overlap,LOOP-003__char-io-primary,CALL-001__direct-recursion,CALL-001__downgrade-hooks}.json`
- Modified: `src/analyzer/{mod,finding}.rs`, `tests/fixtures/e2e/src/main.c`, `tests/pipeline_e2e.rs`, `docs/heuristics/catalog-v1.md` (ALLOC-002 fclose deferral note)
- EOL ritual updates: `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`, `CHANGELOG.md`
