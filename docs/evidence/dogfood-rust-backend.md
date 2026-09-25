# Dogfood + `examples-rust/` — evidence (plan `rust-backend`, phase R4)

> **Superseded R4 snapshot (M1 note, append-only).** Every count in this document is the
> `rust-backend` R4 self-scan and **predates the M1 Precision Gate** (`evidence-beta`), which
> disposed 5 rules (`ALLOC-001`, `ARCH-003`, `STR-001`, `MEM-002`, `LOOP-008`). `perfpal src`
> now reports **304 findings (76 hot / 97 warn / 131 info)**, and those 5 rules are down to
> **168** (`ARCH-003` 90, `ALLOC-001` 74, `LOOP-008` 4, `STR-001` 0, `MEM-002` 0). History
> below is retained **unchanged**. The full before/after baseline and the per-rule
> precision KPI are [precision-baseline.md](./precision-baseline.md); see also the
> [updated Rust catalog §3–4](../heuristics/rust-catalog.md) and [roadmap M1](../roadmap.md).

- **Status:** LANDED — plan `rust-backend` R4. This document is the published dogfood
  report and the corpus coverage record for the Rust language tier.
- **Reproduce (dogfood):** `perfpal src -o <out>` from the repo root (33 `.rs` files).
- **Reproduce (corpus):** `perfpal examples-rust -o <out>` (14 `.rs` files).
- **Build:** local debug binary of this commit; `metrics.json` schema **1.2**.
- **Artifacts are NOT committed** — the full run is 1.5 MB `metrics.json` / 640 KB
  `report.md`; this doc is the curated, durable evidence (founder decision,
  session 28). Regenerate with the two commands above (byte-deterministic modulo
  `started_at`/`duration_ms`/`root_path`).

---

## 1. Dogfood — Perfpal scans its own `src/`

> **Final (post-fix) run:** `scanned 33 files (33 ok, 0 partial, 0 failed) · 1219 findings (658 hot, 214 warn, 347 info)`.

17 of the 23 `PERF-RS-*` rules fire on Perfpal's own source (§1.1); the other 6
are listed in §1.4 (five constructs genuinely absent + one rule made silent by
the §1.3 fix). The headline conclusion: **there are no clear-cut source-level
hotspots** — the volume is dominated by constructs that are *documented accepted
FP classes* of the syntactic tier, by intentional design, or by rule-accuracy
defects (one class of which was found and fixed here; two deferred).

### 1.1 Per-rule counts and triage verdict (firing rules)

| Rule | n | Triage verdict |
|---|---|---|
| `PERF-RS-ALLOC-001` | 511 | **NOISE / accepted-FP** — per-finding report construction (`format!`, `"x".to_string()` on literals) inside builders; catalog §3 class (tier cannot prove buffer reuse). 0 real. |
| `PERF-RS-ARCH-003` | 204 | **Accepted-FP** — 110/204 match the ubiquitous callee name `new`; name-based LOW coupling class (catalog §3). 0 real. |
| `PERF-RS-STR-001` | 145 | **Accepted-FP** — `findings.push`/`evidence.push` are `Vec::push`, colliding with the string-push family (documented §3 `push` collision). 0 real. |
| `PERF-RS-MEM-002` | 128 | **Accepted-FP** — `unit.file.path.clone`-style copies on owned output records; idiomatic-copy class (§4). 0 real. |
| `PERF-RS-LOOP-008` | 96 | **Accepted-FP** — `unit.file.path` member chains; field-access/idiom class (§4). 0 real. |
| `PERF-RS-LOOP-001` | 39 | **Accepted-FP** — depth-3 nests over tiny fixed collections in rule code. 0 real. |
| `PERF-RS-STR-002` | 38 | **Accepted-FP** — `.len()`/`.is_empty()` on O(1) `BTreeSet`/`Vec` (§3). 0 real. |
| `PERF-RS-CALL-001` | 23 | **Real but intentional** — AST recursion (`walk`, `shape_of`, `access_chain`, `flatten_use`) in both backends; iterative rewrite not low-risk. Documented, not fixed. |
| `PERF-RS-TYPE-002` | 7 | **Accepted-FP** — no `#[repr(C)]` in the repo; catalog §4 mandatory `repr(Rust)` auto-reorder caveat. 0 real. |
| `PERF-RS-CALL-002` | 6 | **Real, deferred** — see §1.3 follow-ups (`apply_resolutions` 22, `extract` 21, `mem_001` 17, `cross_file_call_edges` 16, `flatten_use` 16, `resolve_use` 15). |
| `PERF-RS-LOOP-002` | 5 | **Accepted-FP** — `fs::write`/`fs::read`/`eprintln` where the loop *is* the per-artifact/per-file batch. |
| `PERF-RS-ARCH-001` | 5 | **Real (structural)** — hubs: `ir/mod.rs` fan-in 26, `scanner/mod.rs` 21, `analyzer/mod.rs` 17, `analyzer/finding.rs` 14, `analyzer/heuristics/mod.rs` 12. Splitting is architectural; documented, not fixed in R4. |
| `PERF-RS-LOOP-004` | 5 | **Mixed: 3 defect + 2 real.** `charts/mod.rs:266,505,570` are `as f64 / as f64` float division (out of the rule's *integer* div/mod scope — deferred defect, §1.3); `analyzer/heuristics/type.rs:218` (×2) is `(align - offset % align) % align` — a genuine integer `%` with a variable divisor in an in-loop body, **in scope and correctly reported**. |
| `PERF-RS-TYPE-001` | 4 | **Real, LOW** — 2 are test helpers; 2 are genuine by-value passes (`Span` at `resolver/mod.rs:245`, `CrossFileCallEdge` at `aggregator/mod.rs:265`). Deferred (signature ripple). |
| `PERF-RS-ARCH-002` | 1 | **Real (aggregate)** — `analyzer/heuristics` component, complexity_sum 297 over 50 fns. Informational. |
| `PERF-RS-LOOP-006` | 1 | **Real but benign** — bounded `REGISTRY` dispatch (`(entry.f)(slice)`). |
| `PERF-RS-LOOP-007` | 1 | **Rule defect (deferred)** — fires on a `BTreeMap` index (`graph.rs:139`, `layer[node.as_str()]`); catalog says *linear* membership scan. See §1.3. |

### 1.2 Top files by findings (final, post-fix run)

| # | File (`src/`-relative) | total (hot/warn/info) |
|---|---|---|
| 1 | `analyzer/heuristics/rust/loop.rs` | 132 (77/23/32) |
| 2 | `analyzer/heuristics/loop.rs` | 131 (77/26/28) |
| 3 | `analyzer/heuristics/arch.rs` | 104 (64/22/18) |
| 4 | `analyzer/heuristics/rust/arch.rs` | 69 (48/11/10) |
| 5 | `scanner/backends/tree_sitter_rust/mod.rs` | 58 (18/6/34) |
| 6 | `resolver/mod.rs` | 55 (4/18/33) |
| 7 | `analyzer/heuristics/alloc.rs` | 53 (39/4/10) |
| 8 | `analyzer/heuristics/rust/alloc.rs` | 48 (29/6/13) |
| 9 | `reporter/mod.rs` | 48 (42/2/4) |
| 10 | `scanner/backends/tree_sitter_c/mod.rs` | 48 (17/3/28) |
| 11 | `scanner/mod.rs` | 47 (13/22/12) |
| 12 | `analyzer/heuristics/rust/str.rs` | 46 (25/6/15) |

### 1.3 Real issues

**Fixed in R4 — `PERF-RS-LOOP-003` `for`-header false positive (and the
`is_innermost`/`statement_role` half of the same class).**
Dogfood + a direct probe showed `for c in text.chars()` filed `LOOP-003`, even
though `docs/heuristics/rust-catalog.md` §3 states such headers *correctly never
fire*. Root cause: the Rust backend set `loop_depth` **before** visiting the
`for_expression`'s iterable (`value` field), so the one-time iterable expression
was treated as per-iteration body work. Unlike C's per-iteration
increment/condition, Rust's `for` iterable is evaluated **once**.

Fixed in `src/scanner/backends/tree_sitter_rust/mod.rs`:
- `enter_loop` visits the iterable at the **outer** depth and skips it in the
  body walk (new `visit_children_except`);
- a new `in_for_iterable(node)` guard makes `meta_for` force
  `is_innermost = false` and `call_position` return
  `(StatementRole::Unknown, false)` for iterable nodes — otherwise an event could
  report the self-contradictory `in_loop_depth = 0` **and** `is_innermost = true`
  (contract `ir-v1.md` §3.3), and `is_innermost`-keyed rules (RS-LOOP-004) would
  still false-fire on `for i in 0..(n / d)`;
- `while`/`loop` conditions and bodies keep the inner (per-iteration) depth (the
  `while let` condition contract is unchanged).

Six backend tests lock it (`for_iterable_is_outside_loop_body_depth`,
`charwise_call_in_loop_body_stays_in_loop`,
`for_iterable_call_does_not_shadow_body_depth`,
`for_iterable_arithmetic_is_not_innermost`,
`while_inside_for_iterable_stays_in_loop`,
`loop_in_for_iterable_does_not_demote_body_innermost`). The last two complete the
class: the exclusion stops at the **nearest loop of any kind** (a `while`/`loop`
nested in the iterable stays correctly in-loop), and `is_innermost_event` now
skips the iterable when deciding whether the `for` nests a loop, so a loop
*inside* the iterable no longer demotes the `for` body's events from
`is_innermost` (this last part fixed a pre-existing FN surfaced by review).

Effect on the self-scan — **6 findings removed, 0 added** (1225 → 1219), all
`for`-iterable events that are evaluated once and must not be in-loop:

| Removed finding | Construct |
|---|---|
| `PERF-RS-LOOP-003` ×2 (`charts/mod.rs:133`, `reporter/mod.rs:731`) | `for c in …chars()` header |
| `PERF-RS-STR-002` ×3 (`graph.rs:127`, `scanner/mod.rs:469`, `scanner/mod.rs:573`) | `.len()` in a `for` iterable |
| `PERF-RS-LOOP-008` ×1 (`resolver/mod.rs:1545`) | member chain in a `for` iterable |

**Deferred follow-ups (rule-accuracy, surfaced by dogfood — NOT fixed in R4):**

1. `PERF-RS-LOOP-004` fires on float division (`charts/mod.rs:266,505,570`:
   `sum as f64 / preds.len() as f64`). The catalog scope is *integer* div/mod;
   reliably distinguishing float operands needs IR support the syntactic tier
   does not expose today. (The two `type.rs:218` hits are correct — see §1.1.)
2. `PERF-RS-LOOP-007` fires on a map index (`graph.rs:139`, `layer[node.as_str()]`
   where `layer` is a `BTreeMap`). The catalog scope is a *linear* membership
   scan; gating on a sequence root is not syntactically available.

**Deferred source-level smells (documented, not fixed):** the 6 `CALL-002`
high-complexity functions and 2 real `TYPE-001` by-value passes (§1.1). Both are
low-severity and non-trivial diffs; they belong to a dedicated refactor task, not
an evidence session.

### 1.4 Rules that do not fire on own code

- **Construct genuinely absent (expected; no detection gap):**
  `PERF-RS-ALLOC-002`, `PERF-RS-ALLOC-003`, `PERF-RS-STR-003`,
  `PERF-RS-LOOP-005`, `PERF-RS-MEM-001` — grep confirms `src/` contains no
  `drop(` in a loop, no `.reserve(`, no `unsafe` raw-parts ops, no `static mut`
  writes, and no `with_capacity(size_of::<…>())`. All five fire in the corpus
  (§2).
- **Made silent by the §1.3 fix:** `PERF-RS-LOOP-003` — its only self-scan hits
  were `for`-header `.chars()` iterables, now correctly excluded.

17 (firing) + 6 (non-firing) = the full 23-rule catalog. ✔

### 1.5 Noise drivers → tightening candidates (follow-up, not R4)

- `PERF-RS-ALLOC-001` (511): suppressing `"literal".to_string()` would remove a
  large, provably-empty slice of the volume.
- `PERF-RS-ARCH-003` (204): suppressing ubiquitous constructor names
  (`new`, `default`, `clone`) or requiring receiver evidence would cut ~half.

---

## 2. `examples-rust/` corpus — all 23 rules fire

A mirror of `examples/` (the C corpus): a 15-file mini-crate (14 `.rs` files /
547 lines + a 17-line `Cargo.toml`) — 10
numbered issue-dense programs (`src/01_…` … `src/10_…`), a shared `src/common.rs`,
a `src/cycle_a.rs`/`src/cycle_b.rs` `use`-cycle pair, `src/lib.rs`, and a minimal
`Cargo.toml`. (Line density vs `examples/` — 899 lines — is lower, but the
intent, "one issue-dense program per family with silent controls", is met: 10
programs, 23/23 rules, 13 controls.)

- **Why a mini-crate, not a flat dir:** an internal `use`-cycle only resolves
  with a `Cargo.toml` + `src/` layout (crate-root detection). Verified: the same
  two files scanned manifest-free yield **0** findings; with the manifest the
  cycle fires.
- Numbered files are intentionally **not** `mod`-declared (Rust identifiers cannot
  start with a digit); the scanner discovers them by extension — parity with
  `examples/` naming. Documented in `src/lib.rs`. The manifest sets
  `publish = false` and is not a member of any workspace, so root `cargo`
  ignores it entirely.
- Every program opens with a header comment listing its exact expected findings,
  and each rule family carries a **silent control** (13 controls, all confirmed
  to produce zero findings of their target rule).

> **Final run:** `scanned 14 files (14 ok, 0 partial, 0 failed) · 41 findings (17 hot, 11 warn, 13 info)` — **23/23 `PERF-RS-*` rules fire.**

| Rule | n | Tripped by (non-exhaustive) |
|---|---|---|
| `PERF-RS-ALLOC-001` | 6 | `01_alloc_churn.rs` (`build_row_labels`, `grid_reset`), `02_string_building.rs:41`, `06_memory_ops.rs:29` |
| `PERF-RS-ALLOC-002` | 1 | `01_alloc_churn.rs::build_row_labels` (`drop`, alloc-cooccurrence → hot) |
| `PERF-RS-ALLOC-003` | 1 | `01_alloc_churn.rs::grow_one_at_a_time` (`reserve`) |
| `PERF-RS-STR-001` | 5 | `02_string_building.rs::accumulate` |
| `PERF-RS-STR-002` | 2 | `02_string_building.rs::scan_lengths` |
| `PERF-RS-STR-003` | 1 | `02_string_building.rs::unsafe_slice` (`from_raw_parts`) |
| `PERF-RS-LOOP-001` | 1 | `03_deep_nesting.rs::grid_checksum` (depth 3) |
| `PERF-RS-LOOP-002` | 3 | `04_stdio_churn.rs::emit_rows`, `01_alloc_churn.rs:40` |
| `PERF-RS-LOOP-003` | 1 | `04_stdio_churn.rs::count_chars_per_iteration` (body-primary `chars`) |
| `PERF-RS-LOOP-004` | 1 | `03_deep_nesting.rs::grid_checksum` (`acc % divisor`, innermost) |
| `PERF-RS-LOOP-005` | 1 | `08_globals_and_chains.rs::run_handlers` (`static mut` write) |
| `PERF-RS-LOOP-006` | 1 | `08_globals_and_chains.rs::run_handlers` (`handlers[i]()`) |
| `PERF-RS-LOOP-007` | 1 | `09_membership_scan.rs::contains_nested` |
| `PERF-RS-LOOP-008` | 1 | `08_globals_and_chains.rs::read_depth` (`cfg.region.zone.depth`) |
| `PERF-RS-CALL-001` | 1 | `05_recursion_complexity.rs::fib` |
| `PERF-RS-CALL-002` | 1 | `05_recursion_complexity.rs::classify` (complexity 17) |
| `PERF-RS-ARCH-001` | 1 | cycle `cycle_b.rs ⇄ cycle_a.rs` (`kind=cycle`) |
| `PERF-RS-ARCH-002` | 1 | component `src` (complexity_sum 84) |
| `PERF-RS-ARCH-003` | 5 | `10_cross_file_and_arch.rs::orchestrate` → `01`/`03`/`common`; cycle pair |
| `PERF-RS-TYPE-001` | 1 | `07_struct_by_value.rs::consume_blob(BigBlob)` (32 B) |
| `PERF-RS-TYPE-002` | 1 | `07_struct_by_value.rs::PaddedLayout` (padding 14 B) |
| `PERF-RS-MEM-001` | 1 | `06_memory_ops.rs::over_reserve` (`with_capacity(size_of::<u64>())`) |
| `PERF-RS-MEM-002` | 3 | `06_memory_ops.rs::duplicate_rows`; `01_alloc_churn.rs` (`copy_from_slice`) |

### Silent controls (13, each verified to fire **0** findings)

| Control | Construct | Target rule (must not fire) |
|---|---|---|
| `01_alloc_churn.rs::make_scratch` | alloc + reserve outside any loop | ALLOC-001 / ALLOC-003 |
| `01_alloc_churn.rs::drop_outside_loop` | `drop` at depth 0 | ALLOC-002 |
| `02_string_building.rs::append_via_add_assign` | `s += part` (compound assign) | STR-001 |
| `02_string_building.rs::len_with_reassignment` | `.len()` after reassignment in loop | STR-002 (suppression) |
| `02_string_building.rs::bounded_copy` | `copy_from_slice` (checked) | STR-003 |
| `04_stdio_churn.rs::consume_all` | nested non-primary `chars()` | LOOP-003 |
| `06_memory_ops.rs::capacity_from_count` | `with_capacity(count)` | MEM-001 |
| `07_struct_by_value.rs::consume_blob_ref` | `&BigBlob` param | TYPE-001 |
| `07_struct_by_value.rs::PackedTight` | padding-free struct | TYPE-002 |
| `08_globals_and_chains.rs::call_directly` | `Direct` callee | LOOP-006 |
| `08_globals_and_chains.rs::read_zone_ref` | chain depth 2 | LOOP-008 |
| `09_membership_scan.rs::first_match_shallow` | subscript compare at depth 1 | LOOP-007 |
| `10_cross_file_and_arch.rs::clamp_all` | file-local callee | ARCH-003 |

---

## 3. Takeaways

- Dogfooding paid off: it found a real **rule-accuracy defect** (`LOOP-003`
  `for`-header + the matching `is_innermost`/`statement_role` inconsistency) that
  unit fixtures never exercised; the fix removed exactly 6 once-evaluated-iterable
  findings from the self-scan and is locked by 6 backend tests.
- The Rust syntactic tier is **honest about its noise**: the self-scan volume is
  an FP-class catalogue, not a defect list — exactly the confidence framing the
  catalog promises.
- Deferred, tracked follow-ups: `LOOP-004` (float-operand scope), `LOOP-007`
  (sequence-root gating), `CALL-002` complexity splits, `TYPE-001` by-value
  passes, and the two ALLOC-001/ARCH-003 tightening candidates.
