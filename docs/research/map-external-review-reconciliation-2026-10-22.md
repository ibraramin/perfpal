# Reconciliation — external map review vs `map-intelligibility` + ADR-0008

**Date:** 2026-10-22 · **Status:** analysis only; no implementation, no spec/ADR edit, no code.
**Input:** `docs/research/map-external-review-2026-10-22.md` (external, not a decision).
**Founder signal:** review §0:14 quotes *"very convoluted, not easy to understand at all."* The
founder's 2026-10-22 instruction is **"Scrap V6"** (timed test waived) plus a request to tighten
Perfpal's product definition. This document records the signal; it does not ratify it as a formal
V6 verdict (question 1 below).

## Bottom line

The review is directionally right, and it does **not** contradict ADR-0008's rejected idioms. It
exposes a conflict that predates it: the north star says **one visual**, with findings riding as
annotations on the map, **"never as a list to cross-reference by hand"** (`docs/NORTH-STAR.md:17-20`).
The V2 lock says **three co-equal answer-first slots** (`docs/specs/maintainer-map.md:55-57`); V3
chose a **text-first brief** as KV1's idiom (`maintainer-map.md:96-98`); V4 shipped KV1 first, two
bar dashboards second, and the Explorer stems by default. The product now *says* map-first but
*ships* list-first. The review's F1/F2 are symptoms of that incoherence, not a new idiom problem.

## Finding-by-finding

| Review | Anchor in this repo | Reconciliation |
|---|---|---|
| **F1** three dashboards, not one object | `NORTH-STAR.md:17-20`; `maintainer-map.md:55-57`; `views.rs:497-507,529-637,753-782` | **Real conflict.** The north star's "one visual / never a list" is not satisfied by V4. Resolving it is a founder decision (Q2/Q3), not another view. |
| **F2** KV2 cannot answer its own question (count, not *which*) | `maintainer-map.md:62,77` (bar = dependent count; one encoding) | **Valid.** The spec deliberately chose one encoding (count). The question is graph-shaped. Either keep bars and add a text dependents drill (focus + 1-hop, capped) or replace KV2 with a DSM. Both are spec amendments, not silent fixes. |
| **F3** colour triple-booked | `maintainer-map.md:65,77` ("one primary encoding") | **Valid defect**, no conflict. P0 fix, vision-gated. |
| **F4** Explorer heavier than the product | `maintainer-map.md:40-42`; `ADR-0008:37-50` | **Valid scope issue, not an ADR contradiction.** ADR-0008 chose the stems; V4 added key views on top; both ship by default. Resolution is a default/opt-in split, not a new layout family. |
| **F5** KV1 is the part that works | `maintainer-map.md:96-98`; `v5/reads.json` 18/18 | **Agrees with the evidence.** But "keep KV1" must be reconciled with `NORTH-STAR.md:20` ("never as a list"): keep it as an **annotation strip**, not necessarily the hero. |
| **§3** prior art | `ADR-0008:57-83` | **Agrees** with the ADR rejections; no new external claim. DSM stays component-level (ADR-0008 L0), file DSM only on drill. |
| **§4** options A/B/C | `maintainer-map.md:55-57` | A/B strain the three-slot lock; C is zero-churn but does not deliver a spatial object. The review itself names its weakest assumption (`review:136-139`) and the condition that would change its mind. |
| **§5** P0 defects | — | **Valid; #3 is partly stale.** V6a (`0e24c30`) added a per-row non-production badge, grey rows and a demoted-count caption, so KV2 demotion is no longer colour-only. The remaining gap is a per-row *reason*, not disclosure. |
| **§6** aggregator duplication before JS/TS | `docs/roadmap.md:27-29`; `src/aggregator/mod.rs:628,822,995,701,909,1084,778,964,1146` | **Valid sequencing finding**, independent of the map idiom. Record as a pre-JS/TS follow-up. |

## Stop criterion

`ADR-0008:107-110` and `map-intelligibility.md:17`: **two distinct V3 idioms failing the founder
test → ADR-0008 amendment, no endless polishing.** On the record:

1. The I5 8-stem/Explorer idiom failed A5 (*"I can't understand anything"*, `map-intelligibility.md:9-20`).
2. The V4 three-slot idiom now has the founder signal *"very convoluted, not easy to understand at all."*

That is **two distinct idioms**. Recommendation: treat the stop criterion as triggered; do not
start a fifth idiom inside the three-slot frame. The review's options A/B/C are inputs to an
**ADR-0008 amendment**, not a new V-series increment.

## What is not in conflict

- Static-only, candidates-not-certainties, byte-determinism, caps, zero-JS, no new dependencies:
  untouched by the review.
- KV1's *content* (`path:line`, severity, confidence, why) is the part that works; its *rank as
  hero* is the open question.
- ADR-0008's rejected families (3D, force, Voronoi, flame, history) remain rejected; the review
  adds no new claim.

## Four open questions (review §7) — returned to the founder

1. **Formal V6 FAIL or timed test first?** Recommendation: formal V6 FAIL, timed test waived per
   "Scrap V6"; the founder signal is the A2 verdict. This triggers the stop criterion above.
2. **First-screen object: treemap (A), DSM (B), or brief (C)?** Recommendation: A with a brief
   strip and a component-DSM drill; B if "what breaks" is the primary pain; C only if the
   founder rejects the spatial-object goal. This is the product-definition decision.
3. **If A/B: amend the one-question/one-encoding rule?** Recommendation: amend to "one object, one
   **active** metric, one legend; the brief is the only multi-question surface; Q2 is a drill or
   overlay, never a second colour channel."
4. **File-level DSM?** Recommendation: full-corpus file DSM permanently out (quadratic);
   focus + 1-hop file DSM on drill is in scope behind the 50-node cap. Component DSM stays the
   default.

## Guardrails

- No implementation before the founder answers Q1–Q4 and the product tightening.
- V6a (`0e24c30`) is committed but may be superseded by the KV2 decision (Q2/B); do not revert
  without instruction.
- `map-build` remains NOT done; A1b remains open; the holdout was scanned but never read (V6
  timed test scrapped before execution).
