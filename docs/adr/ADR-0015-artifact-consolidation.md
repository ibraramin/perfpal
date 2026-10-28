# ADR-0015: Consolidate the report artifacts and unify their style

- **Status:** Accepted (polish-report H3, 2026-10-28)
- **Date:** 2026-10-28
- **Plan:** `docs/plans/polish-report.md` (H3)
- **Related:** `docs/report/output-contract.md` §2/§3/§4/§5; `docs/specs/report-format.md` §2/§4/§8/§9;
  `docs/specs/agent-report.md`; `ADR-0014` (the warm-paper Atlas page this decision extends).
- **Supersedes:** none. Amends the pre-beta artifact set and `report.md` section order.

## Context

- **H3 is an artifact-role pass, not a merge pass.** `metrics.json` is the canonical data source;
  `agent-actions.json` is the complete machine action contract; `profile-queue.{md,html}` is the
  distinct KV3 function list; `report.{md,html}` is the narrative + Atlas; `agent-report.md` is the
  bounded LLM digest. The remit is to name a consumer per file, delete anything without one, and
  never leave two full renderings of the same list.
- **`findings-detail.md` duplicated `findings-detail.html`.** The two artifacts carried the same
  ranked-action list and the same per-finding rows in two formats. No named consumer existed for the
  Markdown mirror (no text-only diffing workflow, no tool that reads it), so it failed the
  "one audience per artifact" principle.
- **`report.md` Appendix C duplicated the detail artifact.** Appendix C was already a link stub plus
  a pointer, but it still re-rendered the per-finding section shape and a second "N of M" header.
  The detail artifact is the single owner of that content; the report needs a link, not a stub
  section.
- **The human pages had drifted in style.** `report.html`, `findings-detail.html`,
  `profile-queue.html` and the Atlas drill pages reused a head/style block, but the Markdown
  artifacts had different escaping, heading and table conventions, and `profile-queue.md` had no
  named section for its complete list. Formatting drift is a maintenance cost and a source of
  consumer confusion; a single contract plus shared helpers prevents it recurring.
- **The schema copy is contractually required.** It is not a convenience duplicate: an offline
  consumer validates `metrics.json` without the Perfpal repo, and discovering it requires mapping
  `schema_version` to the filename (`"1.8"` ⇒ `metrics.schema.v1.8.json`).
- **Pre-beta status.** No published v1.x consumer exists yet, so a file removal is permitted before
  the beta tag. After a published release the output-contract versioning rule would force a major
  bump for the same change.

## Decision

1. **Remove `findings-detail.md`.** `findings-detail.html` is the canonical complete human detail
   (ranked actions + every finding, complete and static — no script, no filter bar). The fixed
   artifact set becomes **9 files**:
   `metrics.json`, `metrics.schema.v1.8.json`, `report.md`, `report.html`, `findings-detail.html`,
   `profile-queue.md`, `profile-queue.html`, `agent-report.md`, `agent-actions.json` — plus the two
   Atlas drill pages, `charts/graph.dot` and the chart set.
2. **Remove `report.md` Appendix C.** Section order is 1..5, Appendix A, Appendix B. §4 carries the
   complete-list pointer instead:
   `All <N> of <M> findings: [findings-detail.html](findings-detail.html) — complete ranked action
   list and per-finding detail (<R> rules).`
3. **One warm-paper style contract.** All human artifacts share the Atlas palette (`#f4f1ea` paper,
   `#202b35` ink, `#6f6a61` muted, `#d6d0c4` rule, `#fffdf8` card, `#7c2d12` accent, `#ece8df` table
   head, `#fbf9f4` zebra), sans-serif body, monospace only for code/paths, one heading scale,
   tabular numerals, and unified table/link/empty-state markup. `report.html`, `findings-detail.html`
   and `profile-queue.html` share one report-family shell; the Atlas drill pages reuse the same
   palette constants but keep drill-specific markup. Both formats use the shared
   `escape_md`/`escape_html` helpers. `profile-queue.md` gains the `## Complete cost-ranked list`
   section and the `No functions recorded.` stub.
4. **Keep the rest unchanged.** `agent-actions.json` stays complete (uncapped actions, profile cap
   100 named); `profile-queue.{md,html}` stays the distinct complete KV3 list; the schema copy stays;
   every 25-cap and every `N of M` disclosure stays.
5. **Lock the contract with tests.** `tests/artifact_matrix.rs` asserts the role matrix, one owner
   per content class/format, cap disclosures and link consumers; `tests/style_contract.rs` asserts
   heading order per artifact, hostile-input escaping, and the table/link/empty-state contract.

## Consequences

- **Positive.** The fixed set is smaller and honest; no content class has two complete renderings;
  every emitted file names its consumer; the `.md`/`.html` pages cannot style-drift independently;
  `report.md` is shorter and links the complete detail.
- **Breaking pre-beta change.** Directories that previously contained `findings-detail.md` or a
  `## Appendix C` section no longer do. Goldens, artifact-set assertions and link assertions must be
  regenerated/updated (including `.md` anchor references in `agent-report.md`).
- **Downstream links move.** Any consumer that linked `findings-detail.md#…` must link
  `findings-detail.html#…`; anchors themselves are unchanged.
- **No schema bump.** `metrics.json` and `metrics.schema.v1.8.json` are untouched; `schema_version`
  stays `1.8`.
- **Determinism unchanged.** The remaining artifacts keep the same sorted traversals and
  provenance-free detail surfaces; byte-identity across runs still holds.
- **Style unification is a rendering change.** It does not change data, rank keys, caps or
  `metrics.json`; it does require a formatting snapshot test so future edits cannot fork the shell.

## Amendment (2026-10-28, H3 close-out)

- **ADR-0010's click-to-filter description is superseded.** The shipped `REPORT_SCRIPT` is
  pan/zoom only; after the C-split `report.html` has no per-finding rows and
  `findings-detail.html` has no file/component rows, so the filter UI was removed. `report.html`
  keeps pan/zoom + story (three scripts); `findings-detail.html`, `profile-queue.html` and the
  Atlas drills are script-free. ADR-0010 is left unchanged as the decision record.
- **Atlas drill headings join the style contract.** `atlas-weight.html` and `atlas-q2.html` now
  carry `<h1>Perfpal Atlas</h1>` + `<h2>Weight detail</h2>` / `<h2>Blast drill</h2>` on the shared
  heading scale (h1 1.6rem, h2 1.15rem with top rule) and palette; their 10px drill body density
  remains a documented exception.

## Weakest assumption

**The removal is safe because the project is pre-beta and no published consumer exists.** The
decision also assumes no internal/private consumer relied on the Markdown mirror. If a text-only
consumer is later named, restoring `findings-detail.md` is a new contract change and, after a
published release, a major bump — it should not be added back silently.
