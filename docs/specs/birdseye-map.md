# Spec: birdseye-map

Scope: feature

# Birdseye Map — Feature Spec (implements `maintainer-map`, plan `map-build`)

Concrete build contract for the maintainer birdseye map. The repo spec states the durable
rules; this spec states exactly what gets built, in what order, and how each piece is accepted.

## Feature deliverables
- **F1 entry-points + reachability (sequenced C → Rust → Python, one burst per language):**
  per-language entry detection (`main`, handlers, lib roots; heuristic + honest `unknown-entry`
  stub), reachability sets over resolved call edges only (LOW-confidence edges excluded,
  captioned). Additive IR/schema fields, serde-defaulted. C first (resolver + `main`
  conventions strongest), then Rust (`main` + bin/lib targets), then Python (`if __name__` +
  entry scripts); each language commits independently with `cargo test --locked` green.
- **F2 cost-v2:** trip-count gating + per-iteration weights; `function_cost` successor versioned
  alongside (old formula kept until cutover); reachability-aware cap election preserving
  entry-reachable tails past the top-50 cutoff.
- **F3 map layer:** entry-point-stratified cost map as third layer stem (workspace/file pattern),
  plus the deferred middle-bands layout; reuses `resolved_pairs`, break-cycles/topo-layers/
  barycenter pipeline, `(rank,prefix)` cap pattern, symmetric stubs.
- **F4 annotations:** finding dots on nodes (severity+confidence glued, never bare red);
  `report.html` drill path header → workspace → file/component → `findings-detail` rows;
  output-contract registry +1 stem; every heat legend reads candidate-density/cost, profile-to-confirm.
- **F5 proof + demo:** double-scan byte-identical goldens, full gate exit 0, dual review APPROVED,
  9-repo demo eyeball.

## Acceptance criteria (all must hold)
- A1: `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`,
  `cargo test --locked`, `cargo deny check licenses` — all exit 0.
- A2: 67-rule catalog intact; per-rule ratchets never lowered; `metrics.json` findings
  byte-identical on all four golden trees (map artifacts are additive-only).
- A3: `graph.dot` + all SVG stems byte-equal across double-scan UNMASKED; `report.md`
  masked-only (provenance fields).
- A4: every truncation marked (`showing top S of M`, `…+N more`); every empty scope a stub,
  never a blank; LOW-confidence edges labeled until semantic tier.
- A5 (demo test): a maintainer unfamiliar with the repo names 3 concrete actions
  (where to look / what not to touch / what to profile) from the map alone in under
  10 minutes — else the idiom is judged wrong, not the data.

## Explicit non-deliverables
Force-directed full-corpus layout; flame graphs; history/diff views; JS layout engines;
runtime truth claims; cross-component file edges beyond aggregates (stays captioned).

## Research dependency
R1–R4 (`map-research`) + ADR gate must land before F1 starts; if R4 rejects the third-layer
idiom, this spec is amended (new ADR), not silently reinterpreted.