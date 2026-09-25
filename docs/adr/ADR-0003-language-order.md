# ADR-0003: Language Order — C → Rust → Python → JS → TS → C++

- **Status:** Accepted (founder direction recorded session 34; formalized 2026-09-25)
- **Date:** 2026-09-25
- **Deciders:** Founder + orchestrator (evidence: @librarian AI-code language-share research, session 34)
- **Gates:** The post-MVP language sequence in `docs/roadmap.md` and the M1–M6+ milestones of plan `evidence-beta`.
- **Supersedes:** The "Post-MVP Roadmap (direction only)" language ordering in `perfpal-core-spec` (which listed C++ and Python next) and the session-23 order (`rust-backend` → `beta-release`, C++/Python delayed) where it named a *relative* language order.

## Context

Perfpal ships **C** (v0.1.0) and **Rust** (`rust-backend` R0–R6, code-complete in `v0.2.0-beta.1`). The remaining-language order had been informal: `perfpal-core-spec` said "C++ and Python next"; session 23 delayed both while Rust landed.

Session 34 reset the product story to **"Perfpal audits what AI wrote"** (AI-assisted large codebases — see [ADR-0005](ADR-0005-evidence-publish-gate.md)). That story makes **reach into the languages AI actually writes** the sequencing criterion, not linguistic proximity to C.

Evidence (arXiv 2603.27130 — AI-generated-code share): **Python 41.7%**, JS 15.4%, Java 8.6%, **C++ 7.8%**, TS 7.8%, C# 4.8%, Swift 3.8%, **C 3.7%**, Rust not in the top-8. Python dominates; C++ ranks below Python and JS and ties TS at 7.8%; Rust is a strategically necessary backend (dogfooding + systems credibility) but is not a top-share AI language.

## Decision

**Adopt the language order: C → Rust → Python → JS → TS → C++.**

- **C** — shipped (v0.1.0).
- **Rust** — landed (`rust-backend` R0–R6); retained at position 2 for dogfooding + systems credibility, ahead of Python on that non-share rationale.
- **Python** — promoted (M3), the highest AI-generated-code share and therefore the best unlock for the AI-code showcase.
- **JS** and **TS** — **separate** milestones, in that order.
- **C++** — demoted to last (after TS), despite sharing foundations with C: the data does not justify prioritizing it for an AI-code audience.

## Rationale

1. **The audience picks the order.** For an "audits what AI wrote" product, language reach must track AI-authored volume, and Python leads by a wide margin.
2. **Rust stays early for dogfooding, not share.** Analyzing Perfpal's own Rust tree is the best quality signal and part of the product story; the share data does not contradict this (Rust is simply not a top-share AI language, not a bad target).
3. **C++ delayed is cheap.** Its shared foundations with C lower the marginal cost of adding it later; there is no urgency that beats Python's reach.
4. **JS/TS are genuinely different.** Keeping them separate avoids a false "JS/TS backend" monolith and lets each ship honestly.

## Alternatives Considered

- **Keep C → Rust → C++ → Python (session-23 order) — rejected.** C++ sits below Python and JS (and ties TS at 7.8%) in AI-authored share; prioritizing it does not serve the new story.
- **Rust → Python immediately (drop C++ entirely) — rejected as an ADR, retained as a non-goal.** C++ stays on the roadmap (position last), just deprioritized; nothing is cancelled.
- **One combined JS/TS milestone — rejected.** They diverge enough (types, tooling, tree-sitter grammars) to warrant separate milestones.

## Consequences

**Positive**
- The sequence directly follows the AI-code-share data the pitch rests on; Python unlocks the flagship showcase repos.
- Already-spent Rust effort stays valuable (dogfood + credibility), so the change is cheap.
- C++'s late slot is explicit and reversible without re-litigating the whole order.

**Negative / mitigated**
- **Roadmap churn**: `perfpal-core-spec`'s "C++/Python next" line is now stale → this ADR + the rewritten `docs/roadmap.md` are the authoritative ordering; the spec is left as a frozen MVP-era record.
- **Python adds a new grammar/dependency surface** → the no-copyleft constraint and exact-pin discipline (`ADR-0001` G5) apply unchanged to the tree-sitter-python pin.

## Reversal Triggers

1. New, credible language-share data materially re-orders the sequence.
2. A high-value, AI-heavy C++ target emerges that outweighs Python reach.
3. The Python backend proves disproportionately costly (grammar/ABI/semantic-tier friction) versus a JS/TS first move.

## References

- arXiv 2603.27130 — AI-generated-code language share (survey data).
- `docs/specs/evidence-beta-spec.md` §4 (language order), §3 (sequence).
- `docs/roadmap.md` (post-rewrite: authoritative language/milestone order).
- Plan `evidence-beta` (M1–M6+); plan `rust-backend` (R0–R6, complete).
