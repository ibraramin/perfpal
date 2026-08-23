<!-- stm:v1 -->
## Session Memory

### User Instructions
- …

### Long Horizon Context
- …

### Decisions
- …

### Conclusions
- …

### Active References
- …

<existing_memory>
## Session Memory

### User Instructions
- None captured yet.

### Long Horizon Context
- None captured yet.

### Decisions
- None captured yet.

### Conclusions
- None captured yet.

### Active References
- None captured yet.

</existing_memory>

<conversation_update>
USER:
READ-ONLY code/document review — do not edit files. Review /home/ibrar/Perfpal/docs/heuristics/catalog-v1.md (C performance heuristic catalog v1, design-phase artifact, no implementation code exists yet).

Context for consistency checks:
- Spec: /home/ibrar/Perfpal/docs/specs/perfpal-core-spec.md (MVP = C, static-only, pillar 1a stats + 1b architecture-level visualization + LLM-facing output with confidence/severity mandatory)
- ADR: /home/ibrar/Perfpal/docs/adr/ADR-0001-parser-strategy.md (tree-sitter-first syntactic tier; parser-agnostic IR keyed (path,name,span); macro-suspect downgrades; file-local name matching at MVP)
- Plan: /home/ibrar/Perfpal/docs/plans/mvp-architecture.md (step 2 defines this deliverable)

Review across your standard categories as applicable to a design document: correctness/technical accuracy of the heuristic definitions and AST-feasibility claims; internal consistency (IDs, tables vs detail blocks, counts, tier assignments); completeness vs spec pillars (1a stats, 1b architecture signals incl. dependency structure/coupling/hot components, LLM output readiness); constraint compliance (static-only, tree-sitter syntactic feasibility per entry, confidence/severity present everywhere required); clarity/actionability for founder sign-off and for downstream metrics.json schema work (step 3); risks/gaps not documented.

Output: severity-ranked findings (CRITICAL/HIGH/MEDIUM/LOW/NIT) with file line references where possible, then a final one-line verdict SHIP / SHIP-WITH-FIXES / HOLD. Max ~30 lines.

---

ASSISTANT:
# Review: `docs/heuristics/catalog-v1.md`

**Verified:** full read of catalog + spec + ADR-0001 + plan; cross-checked IDs, counts, tier assignments, pillar coverage, ADR guardrail compliance. **Not verified:** the "mainstream tools don't check this" differentiation claims (lines 69, 86, 106) — no tool-source audit performed; treat as unverified assertions.

## Findings

**MEDIUM — LOOP-004 rationale contradicts its own detection shape** (lines 114–117). The heuristic flags `/` or `%` with an *integer-literal* operand, but the rationale claims "compilers can't strength-reduce non-constant divisors." Compilers *do* strength-reduce constant divisors (magic-number multiply-shift), including non-power-of-two literals — so the detected shape is exactly what GCC/Clang already optimize. Either retarget to *variable* divisors (the genuinely slow case) or rewrite the rationale; as written, expected signal is near zero.

**MEDIUM — ADR guardrail 3 not met: no per-entry resolver-tier tag** (catalog line 5 vs ADR lines 24–25, 73). ADR requires "Every heuristic tagged `syntactic` or `semantic` in catalog metadata" and per-finding tier recording in confidence. The catalog has a blanket statement only; no per-ID tier tag, and the confidence model (lines 20–27) never records *which tier served each finding*. Add a tier column or per-entry tag before step 3 schema work.

**MEDIUM — Tier 2 detail blocks missing for 2 of 8 entries** (table lines 154–155 vs detail bullets 159–163). LOOP-005 (global var written in loop) and LOOP-006 (indirect call via function pointer) have no detection sketch, FP documentation, or confidence justification — unlike every other entry. Incomplete for v1.1 scoping decisions.

**LOW — `macro_suspect` detection mechanism unspecified** (lines 25, 72). Without macro expansion, how does the tool know a call "originates from a macro"? ADR-0001 references a curated macro-pattern recognizer library (ADR line 64); the catalog should cite it, or the downgrade rule is unimplementable as written.

**LOW — differentiation claims unverified** (lines 69, 86, 106). "Only PVS-Studio V819 has an analogue," "mainstream tools don't flag it," "cppcheck/clang-tidy have no C check" — strong claims; cppcheck does ship loop/allocation-adjacent checks. Soften to "not found in surveyed tools" or add a verification pass before founder sign-off (they're marketing claims in a sign-off doc).

**NIT — pervasive markdown backtick errors**: unclosed leading backticks throughout (lines 50, 52–54, 57–58, 60, 75, 85, 90, 95, 105, 110, 115, 130, 153, 160, 171, 204); line 201 missing closing `**`. Renders as broken inline code.

**NIT — line 41 accounting note omits two plan-enumerated candidates**: loop-nesting → LOOP-001 and memcpy-family → MEM-002 are covered elsewhere but absent from the "all accounted for" list.

## What's solid
Counts check out (15/15 Tier 1 table↔details; 8/8 Tier 2 table); all three pillar-1b architecture signals present (ARCH-001 dependency structure, ARCH-002 hot components, ARCH-003 coupling); confidence/severity mandatory everywhere required; §5 exclusions all reasoned; §7 sign-off is actionable; LOOP-004's power-of-two suppression and ARCH-001's angle-vs-quoted separation correctly mirror ADR consequences.

**Verdict: SHIP-WITH-FIXES** — no blockers to founder sign-off, but fix LOOP-004's rationale, add per-entry tier tags (ADR guardrail 3), and complete the two missing Tier 2 detail blocks before step 3.
</conversation_update>
