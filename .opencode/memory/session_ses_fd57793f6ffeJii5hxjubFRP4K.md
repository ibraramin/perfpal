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
Read the rules and begin executing plan mvp-architecture, starting with ADR-0001.

---

ASSISTANT:
Context loaded. Session 2 begins: design gate is OPEN, specs approved, plan `mvp-architecture` active. Task = **Step 1: ADR-0001 (parser choice)** — evaluate tree-sitter vs libclang vs hybrid for C scanning, with a license audit against the no-copyleft constraint.

Quick local discovery first (ADR conventions, repo state):

---

ASSISTANT:
State confirmed. Note: session-1 EOL updates are still **uncommitted** (pending founder approval — will fold into this session's commit request). Now dispatching the two research lanes for ADR-0001 in parallel:

1. **@librarian** — license audit + capability verification (facts)
2. **@oracle** — strategic second opinion on the parser fork (judgment)

---

ASSISTANT:
Dispatching both lanes now — independent, so they run in parallel as background tasks. I'll reconcile when both report terminal status.

---

ASSISTANT:
Both lanes are live on the job board:

| Lane | Agent | Objective | Task ID |
|---|---|---|---|
| lib-1 | @librarian | License audit (tree-sitter / libclang SPDX verification) + capability facts + precedent | `ses_fd574aa61ffePyjLkwe3PU2z5V` |
| ora-1 | @oracle | Strategic verdict on tree-sitter-first vs libclang-first vs hybrid | `ses_fd5748215ffed4RGksEYKQ38Rf` |

**Next:** when both report terminal status, I'll reconcile findings → draft `docs/adr/ADR-0001` (decision + license audit table + consequences) → present the recommendation here for your sign-off before any commit.

Waiting on hook-driven completion — no polling.
</conversation_update>

</existing_memory>

<conversation_update>
ASSISTANT:
**ora-1 reconciled** — @oracle verdict: **tree-sitter-first**, with the key architectural commitment being a **parser-agnostic IR** (`(path, name, span)` identity, tiered `syntactic`/`semantic` resolver interface) so libclang bolts on later exactly like runtime profiles will. Hybrid-now rejected as YAGNI. Strong rationale tied to our non-compiling-AI-codebase audience.

ADR drafting stays blocked on **lib-1** (@librarian license audit) — the license-audit table is a required section of ADR-0001 and must use verified SPDX facts, not memory. Not polling; will reconcile when it reports terminal status.

---

ASSISTANT:
`docs/adr/` confirmed empty (`.gitkeep` only) — ADR-0001 will be the first record there.

**Status:** ora-1 ✅ reconciled. lib-1 (@librarian license audit) still running. ADR drafting resumes the moment it lands — no polling.
</conversation_update>

</existing_memory>

<conversation_update>
ASSISTANT:
Both lanes complete and **converging**: @librarian confirms **both ecosystems are license-clean** (MIT / Apache-2.0+LLVM-exception — the no-copyleft constraint does *not* decide this), while @oracle's capability-based verdict is **tree-sitter-first with a parser-agnostic IR as the real commitment**. Drafting ADR-0001 now from the reconciled evidence.
</conversation_update>

</existing_memory>

<conversation_update>
USER:
continue

---

ASSISTANT:
Reconciling **lib-1** ✅ (license audit received — both options permissive, landmines documented). Both lanes terminal and reconciled; converging evidence supports drafting now. Writing `docs/adr/ADR-0001-parser-strategy.md`:

---

USER:
continue
</conversation_update>

</existing_memory>

<conversation_update>
USER:
continue
</conversation_update>

</existing_memory>

<conversation_update>
USER:
continue

---

ASSISTANT:
Reconciling **lib-1** ✅ (full license audit received and folded into evidence base). Both lanes terminal. Writing ADR-0001 now:

---

USER:
continue
</conversation_update>

</existing_memory>

<conversation_update>
USER:
Read the rules and begin executing plan mvp-architecture, starting with ADR-0001.

---

ASSISTANT:
Context loaded. Session 2 begins: design gate is OPEN, specs approved, plan `mvp-architecture` active. Task = **Step 1: ADR-0001 (parser choice)** — evaluate tree-sitter vs libclang vs hybrid for C scanning, with a license audit against the no-copyleft constraint.

Quick local discovery first (ADR conventions, repo state):

---

ASSISTANT:
State confirmed. Note: session-1 EOL updates are still **uncommitted** (pending founder approval — will fold into this session's commit request). Now dispatching the two research lanes for ADR-0001 in parallel:

1. **@librarian** — license audit + capability verification (facts)
2. **@oracle** — strategic second opinion on the parser fork (judgment)

---

ASSISTANT:
Dispatching both lanes now — independent, so they run in parallel as background tasks. I'll reconcile when both report terminal status.

---

ASSISTANT:
Both lanes are live on the job board:

| Lane | Agent | Objective | Task ID |
|---|---|---|---|
| lib-1 | @librarian | License audit (tree-sitter / libclang SPDX verification) + capability facts + precedent | `ses_fd574aa61ffePyjLkwe3PU2z5V` |
| ora-1 | @oracle | Strategic verdict on tree-sitter-first vs libclang-first vs hybrid | `ses_fd5748215ffed4RGksEYKQ38Rf` |

**Next:** when both report terminal status, I'll reconcile findings → draft `docs/adr/ADR-0001` (decision + license audit table + consequences) → present the recommendation here for your sign-off before any commit.

Waiting on hook-driven completion — no polling.

---

ASSISTANT:
**ora-1 reconciled** — @oracle verdict: **tree-sitter-first**, with the key architectural commitment being a **parser-agnostic IR** (`(path, name, span)` identity, tiered `syntactic`/`semantic` resolver interface) so libclang bolts on later exactly like runtime profiles will. Hybrid-now rejected as YAGNI. Strong rationale tied to our non-compiling-AI-codebase audience.

ADR drafting stays blocked on **lib-1** (@librarian license audit) — the license-audit table is a required section of ADR-0001 and must use verified SPDX facts, not memory. Not polling; will reconcile when it reports terminal status.

---

ASSISTANT:
`docs/adr/` confirmed empty (`.gitkeep` only) — ADR-0001 will be the first record there.

**Status:** ora-1 ✅ reconciled. lib-1 (@librarian license audit) still running. ADR drafting resumes the moment it lands — no polling.

---

USER:
Continue.

---

ASSISTANT:
lib-1 died before returning a result. Recovering it by alias — one retry on the same session:

---

ASSISTANT:
lib-1's session was not retained — unrecoverable. Re-dispatching the license audit fresh, **foreground** this time (single dependent lane, nothing to parallelize — lightest reliable path):

---

ASSISTANT:
Both lanes terminal and **converging**: @librarian confirms **both ecosystems are license-clean** (MIT / Apache-2.0+LLVM-exception — the no-copyleft constraint does *not* decide this), while @oracle's capability-based verdict is **tree-sitter-first with a parser-agnostic IR as the real commitment**. Drafting ADR-0001 now from the reconciled evidence.

---

ASSISTANT:
**ADR-0001 drafted** → `docs/adr/ADR-0001-parser-strategy.md`. Recommendation for your sign-off:

## Verdict: tree-sitter-first, behind a parser-agnostic IR

**The license audit settles nothing — capability decides everything:**

| | tree-sitter | libclang |
|---|---|---|
| License | MIT ✅ | Apache-2.0 + LLVM-exception ✅ (not copyleft) |
| Broken/non-compiling C | Best-effort CST + `ERROR` nodes, never aborts | Declarations silently vanish when headers are missing |
| Footprint | <5 MB source, zero deps | ~25 MB compressed / 100+ MB unpacked + version churn |
| Fits MVP heuristics? | Fully — catalog is syntactic | Overkill; needs compile DB that AI-gen repos rarely ship |

**Why this wins for us:** our core input is AI-generated codebases that often don't compile. libclang degrades exactly where our market lives; tree-sitter degrades gracefully everywhere. GitHub code-nav, Semgrep, and ast-grep all validate this class of tooling on tree-sitter.

**The real architectural commitment** (per @oracle): not the parser but the **IR seam** — entities keyed `(path, name, span)`, heuristics consume only the IR, resolver tiers `syntactic → semantic → dynamic`. libclang and perf/valgrind both bolt onto the same seam later without rewrite. Hybrid-now rejected as YAGNI.
