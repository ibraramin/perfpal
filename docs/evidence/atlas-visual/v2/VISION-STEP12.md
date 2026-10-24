# Step 12 — vision read (step-13 proxy; not the founder gate)

**Status:** **PASS** on the final 1440×900 crops (hyperfine + cJSON). This is a `@vision` proxy for
step 13, not the founder sign-off. Two independent reads per repo: the initial full-page/charts read
and a final crop re-read after the fixes.

## Final reads

### hyperfine

- **First viewport (canonical `#map`):** `ENTRY production main · src/main.rs:88 main · confidence
  HIGH · 0 alternatives`, `HOTTEST: src/command.rs · 611 static cost units (v2) · 87× median
  function`, and the yellow `START src/output/report.rs:144 — Heap allocation inside loop (Rust) · …`
  strip are all visible without scrolling; the full map, 12-card rail and footer fit ≤900, with
  `Where is the mass?` peeking below the fold.
- **Weight:** the production/non-production ribbon reads clearly; the mini-bars are visibly filled
  and proportional (command 100%, metric 70.5%, options 45.2%, report 34.9%, shell 34.4% of the
  largest shown); component bars fill (src 100%, export 30.6%, output 16.5%); the labelled band is
  present.
- **Blast:** target `src/main.rs`, `no resolved dependents`, include coverage `205 of 238 include
  edges`, `0 caller files · 5 callee files (structural)`; legible.
- **File detail:** the exact cost anchor sits on its own line and is fully readable
  (`file total 611 static cost units (v2) · 87× median function · top 2% of files`); role tags and
  function/finding links are complete; bars are proportional to the 611/431/276/213/210/110 costs.
- **Residuals (step-11, disclosed):** canvas node basenames mid-elide (`macos_c_.ters.rs`,
  `executi_.ets.rs`, `windows_.iner.rs`); one rail card (`src/shell_syntax.rs`) clips a long function
  row at `:1` and wraps `…without_p / anicking:247`; the UNREACHED column is a tall empty band.

### cJSON

- **First viewport:** `ENTRY production main · fuzzing/afl.c:85 main · confidence HIGH · 1
  alternative`, `HOTTEST: tests/unity/test/tests/testunity.c · 2,000,468 static cost units (v2) ·
  1,000,234× median function`, and `START cJSON_Utils.c:1188 — Expensive setup or I/O call inside
  loop` are visible without scrolling.
- **Weight:** the 0.2% / 99.8% split is honest and clear; the dashed callout prints
  `non-production (test/bench/other) = 99.8% of scanned static cost (75 files)`; production rows are
  readable; the mini-bars are visibly filled because they are scaled to the largest shown item, with
  the exact share printed beside each row.
- **File detail:** anchors are complete (`file total 2,605 static cost units (v2) · 1,303× median
  function · top 4% of files`), role tags complete, bars proportional. The old 0.1%/0.1%
  bar-vs-label ambiguity is gone.
- **Residuals (step-11, disclosed):** the rail truncates the HOTTEST path mid-string
  (`tests/unity/te.../testunity.c`) and a second path (`tests/unity/ex...ity_fixture.c`); the summary
  line glues `1 alternative HOTTEST:` and `…median function PATH`; the sparse graph leaves the
  UNREACHED column largely empty while the rail is full.

## Crop-vs-DOM note

The charts element crops are 1,393px wide (the true panel width at a 1,440px viewport with a
scrollbar). The reads flagged the rightmost labels as flush/clipped in those crops. DOM measurement
on the live page shows the labels and links end **inside** the panel (`right 1396/1398 < panel right
1409`), so this is a crop artifact, not a page overflow. The ribbon and its dashed band now print the same paired values from one integer-tenths split
(`0.2% / 99.8%` on cJSON, `61.9% / 38.1%` on hyperfine), so the pair sums to exactly 100.0%.

## Gate status

- **A9/A12/A4/A10:** green (`atlas_render`, `atlas_charts`, `atlas_states`).
- **Founder visual gate (step 13): PENDING.** The founder must judge the 1440×900 composition, the
  G-baseline diff and the V8 A1/strip rerun on this reworked page. This vision read does not replace
  that gate.
