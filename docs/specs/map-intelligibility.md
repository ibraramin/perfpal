# Spec: map-intelligibility

Scope: feature

# Map Intelligibility — feature spec (successor to `birdseye-map` F5)

## Why (founder verdict, 2026-10-21)

The I5 human A5 test **failed**. After the 9-repo demo the founder said, verbatim:

> "Honestly, the visuals are all posished and nice, but I can't understand anything, I doubt
> anyone will undersand anything. Now, there can be 2 reasons, 1. We simply don't have enogh
> useful information extracted via perfpal. or 2. We have enough information to construct an
> intuituve and easy to understandable birds-eye-view map of the codebase, it's just that the
> graphs and figures we are constructing aren't doing the data extracted by perfpal justice.
> Use a methodology of going back and forth with the vision model to see what's the issue, and
> from now on, while maijng the graph and charts and anything visual, always consult the
> vision model to make use the visual elemtn adheres to our project methodology (not all
> graphs has to be completely simple and understandable, there has to be ones that are simply
> info-dumps but some of the key ones has to be easily understandablem, do you get me >)"

Per `birdseye-map` A5, **the idiom is judged wrong, not the data**. The I5 proof (determinism,
gate, semantic tier, artifact contract) stands; the visual language and the ranking it
expresses do not.

## The two hypotheses (must be separated by evidence, not opinion)

- **H1 — data insufficiency:** Perfpal does not extract the facts needed to answer the three
  maintainer questions (e.g. no runtime weights, no per-function caller/callee chains at a
  useful granularity, no change/churn signal, no build-flag/gating signal).
- **H2 — visualisation insufficiency:** the facts exist in `metrics.json` (findings,
  `function_cost_v2`, entry/reachability, resolved call edges, include edges, components) but
  the current stems (DAGs, bands, cost heat) do not turn them into a maintainer-readable
  answer.

The first deliverable is a **vision-model diagnostic** that decides, per view and per
question, which hypothesis dominates. Neither hypothesis may be assumed.

## Visual methodology (durable rule)

- **Two tiers, explicitly labelled.**
  - **Answer-first key views (must be legible):** one view answers one question in plain
    language; a reader with no repo knowledge must name a concrete action from it. These are
    the views the founder/vision test judges.
  - **Explorer / info-dump stems (allowed):** full-fidelity charts for power users and
    LLM consumption; they may be dense, but they must be labelled as explorers and must not
    be the only path to an answer.
- **Vision gate is mandatory.** Every chart/visual change is reviewed by `@vision` against
  the methodology before it is claimed done: "what does this communicate?", "what can a
  reader not answer?", "which question does it serve?". Transcripts are committed as
  evidence.
- **No orphan views.** Every key view names the maintainer question it answers and the
  action it is supposed to enable. A view that answers nothing is removed or demoted to an
  explorer stem.
- **Plain language first.** Jargon (`DAG`, `barycenter`, `stratum`, `resolved pair`) stays in
  the explorer tier; key views use maintainer words ("what breaks if I change this", "where
  is time likely going", "what is safe to ignore").
- **Honesty unchanged.** Candidates, not certainties; caps, stubs and LOW-confidence labels
  survive any redesign.

## Acceptance (all must hold)

- **A1 — vision-read pass:** for each answer-first view, a fresh `@vision` read (no source,
  no other view) states the question it answers and one concrete action; recorded verbatim.
- **A2 — founder timed test:** a maintainer unfamiliar with the repo names 3 actions from the
  key views alone in **under 10 minutes**, and says the map is understandable. This is the
  same A5 bar that failed; no agent-only substitute closes it.
- **A3 — data decision:** for every maintainer question, the plan records either "existing
  field X answers it" (with `file:line`/schema proof) or "missing field Y is added
  additively" (schema note + tests). No question is left implicitly unsupported.
- **A4 — determinism/contract:** double-scan byte-identical; goldens regenerated; output
  contract updated; no new dependency; no copyleft.
- **A5 — dual review:** `@reviewer` + `@requirements-reviewer` APPROVED on the final tree.

## Explicit non-goals

Force-directed full-corpus layout; flame graphs; history/diff views; JS layout engines;
runtime truth claims; new languages; any change that makes key views denser than today.

## Relationship to the roadmap

I5 was the last node of `map-build` (I1–I5; there is no I6). This spec governs the next
map node, **before** JS/TS backends and the paper queue: a core-feature that nobody can read
is not a shippable core feature. M5 remains gated by `B-pf-001`/`B-pf-002`.