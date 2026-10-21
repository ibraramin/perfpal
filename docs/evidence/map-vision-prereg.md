# Map-Intelligibility V0 — Pre-registered answer key

> **Written before any chart, SVG, `report.html`, `report.md`, or `out-i5-*` artifact was
> opened.** Inputs: repo source at the pinned commits + `docs/evidence/i5-map-triage.json`
> (source-level triage). `docs/evidence/i5-map-proof.md` §7.2/§7.3 (map-derived blind answers
> and founder verdict) was deliberately **not** used. The pre-registration is committed first
> so the V0/V1 vision read cannot be reverse-fitted to the expected answers.
>
> Date: 2026-10-21 · plan `map-intelligibility` V0 · tree `337ed93`.
> Sample repos: tmux `c251547e`, hyperfine `c516152`, cJSON `6d9f244`
> (checkouts at `/home/shafin/Opencode/perfpal-test/<repo>`).

## Protocol this key is used for

1. Render `report.html` first screen, `report.html#map`, and each of the 8 overview stems at
   **1440×900, 100% zoom** (Playwright, PNG) — `docs/evidence/map-vision/`.
2. Give each PNG to `@vision` **blind**: no source, no sibling views, no expected answer.
3. Recall test (V1): are these actions surfaced by any view — at what rank, under what label?
   Data exists but never surfaces → **H2 (visualisation)**; the field needed is absent →
   **H1 (data)**.

## tmux (C, 216 files)

### 1. Where to look first
- **Path:** `input.c:970` `input_parse` (per-byte PTY parser; called from
  `input.c:1043-1076` `input_parse_buffer` ← `input.c:1029-1035` `input_parse_pane` ←
  `window.c:1688-1712` read callback).
- **Action:** profile bulk pane output, then try a printable-ASCII run fast path (one
  `screen_write_collect_add` per byte today, `input.c:1244`).
- **Why:** steady-state shipped path for every byte every pane emits; not startup, tests or
  teardown. **Confidence: high.**
- **Expected map evidence:** a production hot path, visible if the map ranks real steady-state
  work — not necessarily reachable from `format.c`.

### 2. What breaks if touched / do not touch
- **Path:** `tmux.h` (4274 lines; `struct grid_cell:874`, `struct screen:1071`,
  `struct window_pane:1300`, `struct window:1439`).
- **Blast radius:** **160 of 203** `.c` files include `tmux.h` (the other 43 are
  `compat/`/`osdep-*`); `grid_default_cell` is referenced 99× in 23 files; the structs are
  written directly across `input.c`, `screen-write.c`, `tty.c`, `grid.c`, `window.c`.
- **Action:** treat the header/ABI as frozen during a performance pass.
- **Confidence: high.**

### 3. Where time plausibly goes / profile
- **Path:** `screen-write.c:2776` `screen_write_collect_add` (per-printable-byte leaf; branch
  ladder `:2794-2807`, byte store `:2830`; called from `input.c:1244`, `input.c:2892`).
- **Action:** `perf record -g` a pane running `seq`/`yes`; attribute self time here and in
  `input_print` before changing anything.
- **Confidence: medium** (static inference).

### Noise / safe to ignore
`format.c:6053` `format_replace` (I5 top cost) — dispatcher, cost is branch/statement
inflation; callers exist at `format.c:6839,6909`. `window.c:882-940`
`window_get_active_at` — pane-count TAILQ scans on mouse events.

## hyperfine (Rust, 53 scanned files)

### 1. Where to look first
- **Path:** `src/benchmark/mod.rs:346` `Benchmark::run` loop (`run_command_and_measure`;
  loop `:333-359`, validation `:352`, accumulation `:354`).
- **Action:** instrument/profile the per-iteration executor call and measure hyperfine's own
  overhead per timed run before optimizing anything.
- **Why:** the only production code repeated once per timed run; warmups (`:234`) and the
  initial run (`:279`) share the executor. **Confidence: high.**

### 2. What breaks if touched / do not touch
- **Path:** `src/command.rs` (`Command:24`, `Commands:173`).
- **Blast radius:** imported by `src/benchmark/executor.rs:7`, `src/benchmark/mod.rs:13`,
  `src/benchmark/scheduler.rs:5`, `src/options.rs:10`, `src/main.rs:9`; owns substitution,
  quoting, shell-word splitting and environment merge (`:95-170`). A change silently alters
  every benchmark command line and the names shown in reports.
- **Confidence: high.**

### 3. Where time plausibly goes / profile
- **Path:** `src/timer/mod.rs:82` `execute_and_measure` (wall-clock start `:81`, spawn `:82`,
  blocking `wait4` `src/timer/unix_timer.rs:33-41`).
- **Action:** off-CPU/wall-clock profile (`perf sched`, `strace -c`, samply); expect the child
  wait to dominate; do not optimize statistics/formatting first.
- **Confidence: medium.**

### Noise / safe to ignore
`tests/execution_order_tests.rs:31-34` `arg` helper (TEST_NOISE, fn_cost artifact). Triage
`src/command.rs` / `src/metric.rs` nodes are startup/one-shot, not the hot path.

## cJSON (C, 101 scanned files)

### 1. Where to look first
- **Path:** `cJSON.c:357` `parse_number` (allocate `:357`, `memcpy` `:363`, `strtod` `:378`,
  `deallocate` `:406`; dispatched from `parse_value` `:1404-1406`).
- **Action:** use a bounded stack buffer for the literal and allocate only on overflow;
  benchmark a number-heavy parse before/after.
- **Why:** every numeric token pays a heap round-trip solely to append `NUL`/normalize the
  locale decimal. Local, ABI-neutral. **Confidence: medium.**

### 2. What breaks if touched / do not touch
- **Path:** `cJSON.h` (`struct cJSON:103-123`, `cJSON_Hooks:125-130`, field macros
  `:281,284,289,296`; 83 `CJSON_PUBLIC` declarations).
- **Blast radius:** public ABI; `cJSON_Utils` writes `next/prev/child` directly
  (`cJSON_Utils.c:409-424`); layout/offset change breaks every prebuilt consumer, including
  `cJSON_Utils`, with no ABI version check.
- **Confidence: high.**

### 3. Where time plausibly goes / profile
- **Path:** `cJSON.c:243` `cJSON_New_Item` (`allocate` `:243`, `memset` `:246`; root `:1166`,
  array elements `:1535`, object members `:1698`; string `:863`; number temp `:357`).
- **Action:** profile allocator share (perf/massif) on parse+print; if allocation dominates,
  batch/arena internally **without** changing `cJSON.h`.
- **Confidence: medium.**

### Noise / safe to ignore
`tests/unity/test/tests/testunity.c:4571` million-iteration float-print sweep (body disabled
by `TEST_IGNORE` at `:4559-4560`). `tests/parse_hex4.c` exhaustive 16-bit test.

## How this key is scored (V0/V1)

Each repo contributes 9 expected answers (3 per repo). For each, V1 records:

| Field | Values |
|---|---|
| surfaced? | `yes` / `partial` / `no` |
| view | stem + anchor/row |
| rank | e.g. `#1`, `top 5`, `below fold`, `not listed` |
| label fidelity | does the view say what the action is, in maintainer words? |

Any expected answer whose data is present in `metrics.json` but absent from every view is an
**H2** data point for that question. Any expected answer with **no supporting field at all**
is **H1**, and the missing field is listed with an additive-schema note.

**Prediction registered now (falsifiable):** the ranking defects already documented
(`R-pf-098` entry strata, `R-pf-099` test/literal-bound cost rank, `R-pf-100` tiny-scope
density) predict that the **cost** answer is *surfaced but mis-ranked* (H2-by-ranking), while
the **blast-radius** answer may be H1 if include edges are not exposed per-file in a key view.
The vision read must be able to contradict this.
