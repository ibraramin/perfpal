# G — entry-flow map prototype (cJSON, hyperfine, tmux)

Static-first, zero-JS entry-flow map. Forward call direction only. Call edges are
structural and LOW confidence in the default tier: the view says **estimated static
flow** and never claims runtime truth.

## Entry policy

- Prefer a **production main** (`entry_kind == main`). Build scripts (`build.rs`),
  `tests/`, `fuzzing/`, `benches/`, `examples/` and `scripts/` are not program entries.
- If no production main exists (libraries), use the **highest fan-in production
  `lib-root` function** and label it `library root (no production main)`.
- Ties: cost desc, fan-in desc, path asc. Alternatives are shown in the header and
  recorded in `manifest.json`.

## Reading the view

- Header summary: `N files shown · R reachable · A reachable aggregated · U unreached production`.
  cJSON is `6 files shown · 1 reachable (entry) · 0 reachable aggregated · 5 unreached production (grey)`;
  the 5 grey nodes are callers into the library root, not extra reachable files.
- Small numbers on call edges are `edge label = distinct call count` (call-graph edges
  aggregated per caller→callee file pair).
- Near the file cards: `× median function = multiple of the repo-wide median function cost`.

## Units and anchors

- Every visible cost number is in **static cost units (v2)**.
- File/node anchor = file cost / **median function cost**. The median is over every
  function in the repo's v2 scale, zeros included; the ratio is shown to one decimal
  below 100× and rounded to an integer at/above 100×.
- Function lines show **top P%**, where P = ceil(100 × rank / N) over all functions
  ordered by cost desc, fan-in desc, path, name, id (zeros included). Rank 1 = top 1%.
- Cards show 1-3 **full** function names (never ellipsised); long names wrap. Function
  lines carry the rank anchor instead of a second cost figure, so the file line is the
  single magnitude carrier. The SVG `<title>` keeps `path:function` for top functions.

## cJSON

- Entry: `cJSON.c:253` · `cJSON_Delete` · library root (no production main) · confidence MEDIUM.
- Alternatives: 2. cJSON.c:2609 cJSON_CreateObject (fan-in 40); cJSON.c:1227 cJSON_Parse (fan-in 22)
- Files: 6 nodes shown; 1 reachable, 0 aggregated, 5 production unreached (5 drawn grey).
- Edges: 5 of 5 shown file pairs drawn (cap 55); 5 back-edges dashed amber.
- Path trace: cJSON.c.
- Q2 drill target: `cJSON.h` (5 direct reverse includes).

### cJSON specifics

- No production `main` (29 mains are tests/fuzzing). Highest fan-in production
  `lib-root` is `cJSON_Delete` (fan-in 105), so the entry is labelled
  `library root (no production main)`.
- `cJSON.c` has **no outgoing file-level call edges**: the library root is a sink.
  The forward map therefore has one reachable file. The 5 unreached production
  sources are drawn grey in a `UNREACHED` column; they call **into** the entry.
  Reverse context is deliberately left to `g-cJSON-drill.html` (Q2).
- This is the honest structural shape of a single-file library, not a layout bug.

## hyperfine

- Entry: `src/main.rs:88` · `main` · production main · confidence HIGH.
- Alternatives: 1. excluded build-script main build.rs:7
- Files: 19 nodes shown; 19 reachable, 0 aggregated, 21 production unreached (0 drawn grey).
- Edges: 49 of 49 shown file pairs drawn (cap 55); 26 back-edges dashed amber.
- Path trace: src/main.rs → src/command.rs → src/metric.rs.
- Q2 drill target: `src/quantity/mod.rs` (12 direct reverse includes).

### hyperfine specifics

- Production `main` is `src/main.rs`; `build.rs` is a build-script main and is
  excluded (recorded as one alternative).
- Tests/benches reached by call edges remain visible but are demoted in colour
  and labelled `test / bench`.
- 19 reachable files fit under the 20-card budget; no reachable aggregation.

## tmux

- Entry: `tmux.c:434` · `main` · production main · confidence HIGH.
- Alternatives: 0. no plausible alternative entry.
- Files: 20 nodes shown; 106 reachable, 86 aggregated, 99 production unreached (0 drawn grey).
- Edges: 57 of 145 shown file pairs drawn (cap 55); 49 back-edges dashed amber.
- Path trace: tmux.c → options.c → format.c → grid.c.
- Q2 drill target: `tmux.h` (160 direct reverse includes).

### tmux specifics (density stress case)

- Production `main` is `tmux.c:434`; no alternative entry.
- 106 reachable files but only 20 nodes/cards fit: the remaining 86 reachable
  files are aggregated into five `…+N more` depth nodes (depth 1: 11 files,
  depth 2: 28, depth 3: 39, depth 4: 7, depth 5: 1), with incoming-call counts.
- 145 caller→callee file pairs exist among the 20 shown nodes; the view draws
  the top 55 by call count plus path edges (57 total). 49 are dashed amber
  back-edges. The omitted count is in the header and manifest.
- The source `components[]` list is root-only (`fuzz`, `.`, `regress`, `compat`),
  so hulls use filename families (`window`, `tty`, `screen`) as a production-first
  approximation; only families with ≥2 visible files get a hull.
- Path trace: tmux.c → options.c → format.c → grid.c.

## Density / legibility blockers

- Cap: hard ceiling 50 nodes (plan), effective 20 nodes because the 1440×900
  first screen must also carry 20 per-file cards at readable type. Nodes beyond the cap
  and reachable files are aggregated into honest `…+N more` nodes with incoming-call counts.
- Edge drawing is capped at 55 file pairs by call count; path edges are forced.
  tmux has 145 pairs among 20 shown nodes and draws 57 (55 + path); the rest are counted.
- Component hulls: source `components[]` are test-dominated for cJSON and root-only for
  tmux; the prototype derives production-first path/filename domains. Hulls are drawn
  only for domains with ≥2 visible files; single-file domains are named in cards.
- Per-file cards truncate long **paths** only (full path in the title); function names
  are shown in full and the card shows 1-3 functions depending on wrapping.
- cJSON's one-node flow is a genuine data limitation; the drill, not the flow map,
  answers its reverse/dependency question.

## Verification

- Console/page errors observed: 0.
- Each HTML has no `<script` and no `onclick`/`onload`; animation is CSS keyframes only.
- Final PNGs are captured with `prefers-reduced-motion: reduce`, which is the complete
  static frame; start/mid are captured with `document.getAnimations()` paused at 0%/50%.
- Determinism: no clock, no randomness, sorted traversal; HTML bytes are compared across
  two runs with `cmp` (manifest image hashes are run outputs, not HTML input).

