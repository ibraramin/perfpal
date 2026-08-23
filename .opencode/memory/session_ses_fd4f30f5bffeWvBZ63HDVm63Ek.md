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
READ-ONLY requirements verification — do not edit files. Compare the delivered artifact against the ORIGINAL request, verbatim.

ORIGINAL REQUEST (plan `mvp-architecture`, step 2, from /home/ibrar/Perfpal/docs/plans/mvp-architecture.md): "Draft heuristic catalog v1 for C: enumerate candidate static performance heuristics (cyclomatic/cognitive complexity, function length, malloc/free density, pointer-chasing patterns, loop nesting depth, recursion depth, memcpy/strcpy-family hotspots, missed-inline candidates, etc.), rank by signal-value vs implementation cost, present ranked list to founder for sign-off."

DELIVERED ARTIFACT: /home/ibrar/Perfpal/docs/heuristics/catalog-v1.md

Check verbatim:
1. Is it a heuristic catalog v1 for C? 
2. Does it enumerate the named candidates: cyclomatic complexity, cognitive complexity, function length, malloc/free density, pointer-chasing patterns, loop nesting depth, recursion depth, memcpy/strcpy-family hotspots, missed-inline candidates? For each: state PRESENT (where) or MISSING.
3. Are heuristics ranked by signal-value vs implementation cost?
4. Is it structured as a ranked list suitable for founder sign-off (clear decision points)?
5. Any scope additions beyond the request that were not justified/documented?

Return verdict APPROVED / CHANGES_REQUESTED / BLOCKED with a short itemized rationale (max 20 lines).

---

ASSISTANT:
## Requirements Review — Heuristic Catalog v1

**Verdict: CHANGES_REQUESTED**

### Checklist (verbatim vs. plan step 2)

1. **Heuristic catalog v1 for C** — ✅ PRESENT. `docs/heuristics/catalog-v1.md:1` title "C Performance Heuristic Catalog v1"; scope = C, static analysis only (line 4).

2. **Named candidates:**
   - **Cyclomatic complexity** — ✅ PRESENT. `PERF-C-CALL-002` (line 60, detail 124–126); also baseline stat (line 37).
   - **Cognitive complexity** — ✅ PRESENT. Baseline stat, nesting-weighted variant (line 37).
   - **Function length** — ✅ PRESENT. Baseline stat (line 36) with explicit rationale for stat-not-finding (line 41).
   - **malloc/free density** — ✅ PRESENT. `ALLOC-001/002/003` (lines 49–51, 67–82); alloc-site count stat (line 38).
   - **Pointer-chasing patterns** — ✅ PRESENT. `PERF-C-LOOP-008` (line 155, detail 160).
   - **Loop nesting depth** — ✅ PRESENT. `PERF-C-LOOP-001` (line 55, detail 99–102).
   - **Recursion depth** — ⚠️ PARTIAL. Enumerated as `PERF-C-CALL-001` "Direct recursion" (line 59, 119–122) — detects recursion *presence*, not *depth*; depth itself is not addressed (documented as stack-depth risk). Defensible but a deviation from the named candidate.
   - **memcpy/strcpy-family hotspots** — ❌ **MISSING (memcpy half).** strcpy-family covered (`STR-001` line 52, `STR-003` line 54). **`memcpy` appears nowhere** — not in Tier 1, Tier 2, or §5 deferred/excluded (grep confirms only `memmove` in a realloc rationale and `memset` in a zeroing note). The request explicitly named "memcpy/strcpy-family hotspots."
   - **Missed-inline candidates** — ✅ PRESENT. §5 deferred/excluded with reason (line 173); cross-referenced at line 41.

3. **Ranked by signal-value vs implementation cost** — ✅ PRESENT. §1 model (S1–S3 × C1–C3, lines 13–18); S/C columns on every entry.

4. **Ranked list suitable for founder sign-off** — ✅ PRESENT. §7 with three explicit decision points (lines 195–199).

5. **Scope additions** — ✅ None unjustified. Extra heuristics (ARCH/TYPE/MEM/LOOP extras) fall under the request's "etc." and are each documented + grounded (clang-tidy/cppcheck/PVS-Studio, line 6); ARCH items serve plan pillar 1b.

### Required fix
Add a memcpy-family hotspot candidate (e.g., memcpy/memmove in loops, or a deferred/excluded entry with reason) so every named candidate is accounted for. Optionally note recursion-depth treatment explicitly.
</conversation_update>
