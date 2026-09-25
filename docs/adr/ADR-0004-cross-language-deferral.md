# ADR-0004: Cross-Language Linking — a separate milestone after the language sequence

- **Status:** Accepted (founder direction recorded session 34; formalized 2026-09-25)
- **Date:** 2026-09-25
- **Deciders:** Founder + orchestrator
- **Gates:** M1–M5 of plan `evidence-beta`; the language-tagged analysis behavior shipped by `rust-backend` R3/R5.
- **Resolves:** The carried open risk "R5 cross-language ARCH-003 substitution still needs founder sign-off" (`state/session-state.json`).

## Context

Perfpal analyzes each file in **one** language tier. Rust landed in `rust-backend` R3 with a **language-tagged registry** (`PERF-C-*` fires only on C files, `PERF-RS-*` only on Rust files) and a definition map keyed on **`(language, name)`** (`docs/architecture/rust-backend.md` §9.3). As a direct consequence, the aggregator's whole-program call graph **can never emit a cross-language edge**.

R5 made this concrete: the mixed C+Rust corpus (`tests/fixtures/e2e-mixed/`) was required to exercise cross-language ARCH-003 coupling, but that is **impossible without a production `src/` change** under the language-tagged design. The substitution made knowingly at R5 (founder-approved commit `2cde8eb`) asserts the gate — exactly one *same-language* edge, zero cross-language edges — plus a non-vacuity IR check, rather than fabricating coupling the tool does not compute. That substitution satisfied the requirement's intent but left the *question of when real cross-language linking lands* formally open.

At the same time, the session-34 story ("audits what AI wrote") is strongest on **polyglot** AI-assisted codebases, so cross-language insight (e.g. C↔Rust coupling, mixed-language architecture) is genuinely desirable — just not on the critical path to the first release.

## Decision

**Cross-language linking is its own milestone, scheduled AFTER the language sequence (i.e. after JS → TS → C++). It is explicitly OUT of M1–M5.**

- The language-tagged behavior (no cross-language call/architecture edges; `(language, name)` definition-map key; per-language rule gating) **stays as-is** through M1–M5. It is not a defect to fix en route.
- The R5 mixed-corpus substitution is thereby **ratified as the accepted state**, not a temporary hack pending immediate rework.
- Cross-language work will define its own semantics (edge identity across languages, confidence/severity framing, schema implications) when it becomes a milestone — and may require a schema MINOR bump.

## Rationale

1. **Priority is reach, not linking.** The publish gate (ADR-0005) is cleared by precision + benchmark + single-language showcase, none of which need cross-language edges.
2. **Language-tagged analysis is correct per-language.** Mixing languages inside one rule/edge model without an explicit design would risk false coupling (same name, different language/ABI) — exactly what the `(language, name)` key prevents.
3. **Nice-to-have, not existential.** Polyglot coupling is valuable for a later, more mature release; it does not gate credibility of the first one.
4. **It closes an open risk honestly.** Ratifying the R5 substitution removes ambiguity without pretending the tool computes something it does not.

## Alternatives Considered

- **Do cross-language linking now (before the sequence) — rejected.** Highest cost (new semantics, schema, edge identity) for zero publish-gate value, and it would delay M1–M4 which are existential.
- **Promote it to M5 scope — rejected.** The first release should ship a smaller, honest surface; cross-language is post-sequence.
- **Leave the substitution undocumented until later — rejected.** Explicit deferral is the point of this ADR; it converts an accepted deviation into a recorded decision.

## Consequences

**Positive**
- M1–M5 stay focused; no cross-language semantics block the first release.
- The R5 deviation is formally ratified, closing the carried risk.
- Cross-language gets a clean slate when it lands (own milestone, own design, own schema bump if needed).

**Negative / mitigated**
- **Mixed-language codebases show per-language silos** (no C↔Rust edges) through M6 — mitigated by documenting this as an explicit, dated limitation (accepted-limitations page, ADR-0005) rather than a hidden gap.
- **A later schema MINOR bump is likely** for cross-language edges — accepted; the schema versioning policy already anticipates additive/loosening changes.

## Reversal Triggers

1. A showcase or benchmark result shows cross-language coupling is the decisive credibility signal.
2. A high-value target repo is genuinely polyglot and its headline finding is cross-language.
3. The language sequence completes early, freeing the milestone earlier than planned.

## References

- `docs/architecture/rust-backend.md` §9.3 (language-tagged registry / `(language, name)` key), §9.7 (R5 mixed-corpus substitution note).
- `tests/mixed_language_e2e.rs`, `tests/fixtures/e2e-mixed/` (the gate + non-vacuity check).
- `docs/specs/evidence-beta-spec.md` §5 (cross-language deferral).
- ADR-0001 (parser-agnostic IR / language-addition seam), `docs/architecture/modules.md` §5.2.
