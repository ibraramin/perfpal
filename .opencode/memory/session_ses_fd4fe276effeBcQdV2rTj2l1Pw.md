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
plan mvp-architecture, commit 6b5dda4 → step 2: heuristic catalog v1 for C.

---

ASSISTANT:
Resuming with signposts: plan `mvp-architecture`, commit `6b5dda4`, task = step 2 (heuristic catalog v1 for C). Loading context first.

---

ASSISTANT:
State loaded. Verifying the signpost commit and handoff before starting step 2.

---

ASSISTANT:
Signposts verified: `6b5dda4` = ADR-0001 commit, plan active, step 2 = **heuristic catalog v1 for C, ranked by signal-value vs cost, for founder sign-off**. Your message is the explicit go-ahead for this pre-approved plan step, so I'm executing it now.

**Approach (lightest fitting workflow):** this is a design-doc authoring task grounded in ADR-0001 constraints (tree-sitter syntactic tier, macro-suspect downgrades, confidence/severity fields, pillar-1b architecture-level signals). I'll: (1) one `@librarian pass to ground the catalog against established analyzer checklists (clang-tidy/cppcheck), (2) author `docs/heuristics/catalog-v1.md directly, (3) dual review (@reviewer + @requirements-reviewer), (4) EOL ritual + present ranked tiers for your sign-off.

---

ASSISTANT:
Dispatching `@librarian` for grounding research.

---

ASSISTANT:
Research grounded (clang-tidy/cppcheck/PVS-Studio themes + whitespace anti-patterns mainstream tools miss — that's our differentiation space). Now authoring the catalog.

---

ASSISTANT:
Job board reconciled (lib-1 consumed). Writing the catalog now.

---

ASSISTANT:
Job board clean (lib-1 reconciled). Before dispatching reviewers, I'm closing two gaps I can already see against the plan's verbatim enumeration ("pointer-chasing patterns", "cognitive complexity") — better than letting review catch them:

---

ASSISTANT:
Job board clean (lib-1 reconciled). Continuing the gap-closure edits: LOOP-008 detail block, cognitive complexity stat, function-length clarification.

---

ASSISTANT:
Edit failed on exact-match — re-reading the Tier 2 section to get precise text.

---

ASSISTANT:
Job board clean. Applying the LOOP-008 detail bullet with exact text:

---

ASSISTANT:
Job board clean. Final gap-closure edit: cognitive complexity + function-length note in baseline stats.

---

ASSISTANT:
Catalog authored with plan-enumerated candidates all accounted for. Dispatching dual review in parallel.

---

ASSISTANT:
req-1 result consumed (reconciled): CHANGES_REQUESTED — memcpy half genuinely missing, recursion-depth treatment needs an explicit note. Applying both fixes now, then dispatching @reviewer for the second verdict.

---

ASSISTANT:
Job board clean. Applying remaining two fixes:

---

ASSISTANT:
Job board clean. Final fix — explicit recursion-depth note in CALL-001:

---

ASSISTANT:
Job board clean. Both fixes applied. Dispatching @reviewer (quality review) + requirements re-check in parallel for both verdicts.

---

ASSISTANT:
req-2 consumed: **APPROVED** with one cosmetic flag (Tier 2 header says "6 heuristics", table has 8 rows). Fixing that, then dispatching @reviewer for the second verdict.

---

ASSISTANT:
Job board clean. Dispatching @reviewer for the quality verdict (second required review).

---

ASSISTANT:
rev-1 consumed (reconciled): **SHIP-WITH-FIXES** — no disagreements with req-2's APPROVED. Applying all 7 findings (3 MEDIUM, 2 LOW, 2 NIT). Re-reading the full file first, then rewriting once with every fix incorporated.

---

ASSISTANT:
Job board clean — all four sessions reconciled. Rewriting the catalog once with all 7 rev-1 findings applied:

---

ASSISTANT:
Job board clean. All review fixes applied; both verdicts secured (req-2 APPROVED, rev-1 SHIP-WITH-FIXES→fixed). Running the EOL ritual now.

---

ASSISTANT:
Job board clean. Executing EOL ritual — timestamp first, then all four state files.

---

ASSISTANT:
Job board clean. Applying all four ritual updates in parallel:

---

ASSISTANT:
**Step 2 complete.** `docs/heuristics/catalog-v1.md` drafted, grounded against clang-tidy/cppcheck/PVS-Studio (@librarian), dual review passed — @requirements-reviewer **APPROVED**, @reviewer SHIP-WITH-FIXES with all 7 findings applied (notably: LOOP-004 retargeted to *variable* divisors, since compilers already strength-reduce constant ones). EOL ritual done (state/handoff/activeContext/CHANGELOG).

## Ranked catalog summary

**Ranking model:** Signal S1–S3 × Cost C1–C3 → tiers; every finding carries confidence (HIGH/MED/LOW) + severity (hot/warn/info), all tagged `(syntactic)` per ADR-0001.

**Tier 1 — MVP must-have (15):**

| Cluster | Heuristics |
|---|---|
| Memory | heap-alloc-in-loop ⭐, free-in-loop, realloc linear growth |
| Strings | strcat accumulation (O(n²)), strlen-in-loop, unbounded copy family |
| Loops | nesting ≥3, expensive setup/I-O in loop, char-wise stdio, variable-divisor div/mod |
| Calls | direct recursion, high complexity |
| Architecture (pillar 1b) | include-graph fan-out/cycles, hot-component aggregation, cross-file coupling |

⭐ = differentiation space — alloc/string/I-O-in-loop anti-patterns that surveyed mainstream tools don't flag; ideal for AI-generated codebases.

**Tier 2 — should-have (8):** large struct by-value, padding candidate, sizeof-misuse, memcpy-family in loop, global writes in loop, fn-ptr indirect calls, membership scan, pointer-chasing chains.

**Deferred/excluded (10):** loop-invariant hoisting, dead stores, vectorization, missed-inline, etc. — each with a reason (semantic-tier/compiler-territory/scope).

## Your sign-off decisions (catalog §7)

1. **Approve Tier 1 (15 heuristics) as the MVP set?**
2. **Tier 2: include any/all in MVP scope, or hold for v1.1?**
3. **Accept §5 deferred/excluded list as out-of-scope?**

## Commit approval

Pending changes ready to commit on `main`:
- `docs/heuristics/catalog-v1.md` (new)
- `state/session-state.json`, `state/handoff/latest.md`, `docs/activeContext.md`, `CHANGELOG.md` (EOL ritual)
- `.opencode/memory/*` (session artifacts)

Proposed message: `docs: heuristic catalog v1 for C — 15 Tier-1 + 8 Tier-2, ranked for sign-off`

**On your approvals:** I'll commit, then this session is at its boundary — start `/new` and resume with: *plan mvp-architecture, commit \<new-hash\> → step 3: IR + metrics.json schema v1*.
</conversation_update>
