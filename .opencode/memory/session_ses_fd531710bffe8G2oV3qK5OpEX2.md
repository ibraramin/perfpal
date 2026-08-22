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
READ-ONLY REVIEW — no edits. Review `/home/ibrar/Perfpal/docs/adr/ADR-0001-parser-strategy.md` (a decision record, not code — weight categories accordingly).

Context: Perfpal MVP = static-analysis CLI for C; founder-approved spec at `/home/ibrar/Perfpal/docs/specs/perfpal-core-spec.md`; plan at `/home/ibrar/Perfpal/docs/plans/mvp-architecture.md`. The ADR was drafted from two verified research reports (license audit + strategic advisory); your job is to catch errors BEFORE it steers implementation.

Check proportionately across: factual accuracy of license claims (MIT / Apache-2.0 WITH LLVM-exception / LGPL Semgrep flag), internal consistency (decision vs consequences vs guardrails vs reversal triggers), consistency with the approved spec (no contradiction with pillars/constraints), completeness as a decision record, clarity, risks not covered, assumption validity.

Respond in ≤10 bullets, severity-ranked (CRITICAL/MAJOR/MINOR/NIT), each with file line reference where possible. End with verdict: SHIP or FIX-FIRST (with the minimal fix list).

---

ASSISTANT:
All three files read; license claims verified against primary sources (semgrep/semgrep LICENSE = LGPL-2.1 confirmed; tree-sitter-c = MIT confirmed; LLVM Apache-2.0 WITH LLVM-exception is stable well-known fact). Review below.

---

**Review of ADR-0001 (decision record — weighted for docs, not code)**

1. **MINOR — internal inconsistency: macro-recognizer mitigation vs Guardrail 1.** Line 64 promises "small curated library of common macro patterns as cheap syntactic recognizers," but the IR is defined (line 22) as a "normalized stream of functions / calls / includes" — no preprocessor/macro events. Macro recognizers therefore need either parser access (violating Guardrail 1, line 71) or an IR extension that isn't specified. Fix: one line extending the IR definition to include preproc/macro events, or drop the mitigation.

2. **MINOR — missing risk: tree-sitter version churn.** tree-sitter has shipped breaking API changes across 0.20→0.24, and grammar node-types shift between grammar releases. The ADR pins "official tree-sitter-c grammar" (line 18) but no version-pinning policy; Guardrail 4 (line 74) covers licensing only. Since this ADR gates all implementation, a pin-and-upgrade-policy line would prevent churn pain.

3. **MINOR — missing risk: tree-sitter-c is a C99-based grammar** (repo README: "Adapted from this C99 grammar"). AI-generated C frequently uses C11/C17 constructs (`_Generic`, `_Static_assert`, `_Atomic`); expect more `ERROR` nodes than the "localized ERROR" framing (line 46) implies. Worth one sentence in Consequences.

4. **NIT — "~2 orders of magnitude" (line 48) is overstated.** 5 MB vs 100+ MB is ~20× (≈1.3 orders). The asymmetry argument survives; the number doesn't.

5. **NIT — "declarations silently vanish" (lines 46, 53) is slightly overstated.** libclang does emit diagnostics for missing headers; the truncation is silent *to AST consumers*, not literally silent. Harmless, but "silently" invites a pedantic challenge.

6. **NIT — "The fork:" (line 14)** is ambiguous wording; "The options:" or "The fork in the road:" reads cleaner.

7. **NIT — "Zero compile databases will exist in the MVP funnel" (line 54)** is an absolute claim; fine as YAGNI rationale, but "essentially none" is more defensible.

8. **praise: — License audit is factually accurate.** All four rows verified: tree-sitter core MIT, tree-sitter-c MIT (generated parser inherits grammar license — correct), LLVM `Apache-2.0 WITH LLVM-exception` (linking does not taint — correct), Semgrep LGPL-2.1 (correctly flagged weak copyleft, reference-only, never-copy). The UIUC/NCSA legacy-LLVM note (line 41) is also correct. No CRITICAL/MAJOR factual errors found.

9. **praise: — Fully consistent with the approved spec.** No contradiction with pillar 1b (IR carries includes/calls → coupling), pillar 2 (confidence field, line 25), the no-copyleft hard constraint (line 12 vs spec line 35), C-only MVP, or the static-only-with-runtime-bolt-on architecture (resolver tier "dynamic", line 24). Reversal triggers (lines 78–81) are measurable and consistent with the rationale.

10. **praise: — Complete as a decision record.** Context / Decision / License Audit / Rationale / Alternatives / Consequences / Guardrails / Reversal Triggers / References all present; guardrails and reversal triggers are mutually consistent.

**Assumptions checked:** AI repos rarely ship compile DBs → reasonable; non-compiling input is core case → reasonable (matches spec audience); precision buys ~nothing for syntactic MVP heuristics → reasonable; tree-sitter best-effort CST with ERROR recovery → correct; libclang AST truncation on missing headers → correct behavior.

**Not verified:** the "verified examples" of GPL/AGPL/MPL community grammars (line 40) — plausible and policy is sound regardless; the <5 MB / 25 MB footprint figures (line 48) — directionally correct, not independently measured.

---

**Verdict: SHIP**

Minimal fix list (non-blocking, all optional):
- Line 22/64: add one clause extending the IR to carry preproc/macro events (resolves the only real inconsistency).
- Line 18/74: add a version-pinning note for tree-sitter core + grammar.
- Line 48: change "~2 orders of magnitude" → "~20×".
</conversation_update>
