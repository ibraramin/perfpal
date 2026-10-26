# Spec — Atlas Story (temporal + data-flow showcase)

- **Status:** **SEALED** (founder, 2026-10-24). Supersedes the D1 framing in
  `docs/research/atlas-run.md` §1–§3 where they conflict; D1's honesty contract (§4) and D3
  appendix (§6) still apply.
- **Track:** Atlas Run · **Worktree:** `Perfpal-atlas-sim` (`feat/atlas-sim`)
- **Owner:** orchestrator · **Decision:** founder
- **Depends on:** Perfpal core only. No separate pipeline, no HTML scraping, no new dependency.

## 1. Product claim

A **temporal showcase of a codebase you did not write**: press play on a repo and watch the inferred
run — entry → stages → loops → I/O → stop — with **the payload that crosses each file boundary
labelled**. One artifact serves two readers: a vibecoder gets "what does this app do?" in plain
language within 60 seconds; a senior dev gets `path:line`, function and type on demand. Everything is
static inference, labelled as such, never runtime truth.

Source of the requirement: `Perfpal/docs/tracks/atlas-run.md:9` ("feel like the program is running —
startup, flow, loops, work, end/stop — playable, scrubbable, speedable"); `docs/research/atlas-run.md:13-16`
("the map answers *where to look* … it does not answer **order**").

**The gap this spec closes.** D1 animates order but carries **zero data**: `AtlasEdge` is
`{from, to, count, back, on_path}` (`src/reporter/atlas.rs:163-169`) and `AtlasFunction` has no
signature (`atlas.rs:96-111`). The audit's verdict still holds: *"The canvas answers `src/command.rs`
is hot; it does not answer **which function, which finding?**"* (`docs/evidence/atlas-audit/AUDIT.md:51-56`).

## 2. Non-goals

- **No execution.** No runtime trace, values, wall-clock time, sampling or instrumentation. That is D3
  (`docs/research/atlas-run.md` §6, proposal only).
- **No time units.** No `ms`/`µs`/`s`/throughput/latency/speedup claims (`docs/specs/atlas-metrics.md:29-31`).
- **No alias/points-to claims.** C function pointers, `dlopen`, Rust `dyn`/closures/generic dispatch are
  **unresolved**, rendered as such, never guessed.
- **No new dependency, no network, no schema version bump.** The addition is an optional `flow` block.
- **Not a dashboard.** One story surface; no mode switch, no second metric encoding on the canvas.

## 3. Audience contract (first 60 seconds)

| Reader | Must get in 60s | Where it lives |
|---|---|---|
| Vibecoder | 3–5 plain-language stages: what each consumes, produces, and touches (files/env/net) | subtitle line + scene titles |
| Senior dev | entry, hottest **function** (not just file), one `path:line` action | inspector / drill, one click |
| Both | what is inferred vs unresolved; where the story is capped | persistent honesty strip |

A1/A2 remain the acceptance bar: a blind read names one concrete `path:line`/file action per state,
and the founder names 3 actions in under 10 minutes (`docs/specs/atlas-layout.md:189-192`). KV1/KV3
define what an action is (`docs/specs/map-intelligibility.md:93-95`).

## 4. Data contract (Perfpal core owns it)

All story data comes from the Perfpal scan. The story renders from core-emitted JSON:
`metrics.json` (existing `AtlasView`), **`atlas-layout.json`** (`layout_version: 1`; geometry, the
presentation strings the SVG renders, state flags, top functions, derived `stages`, KV1 actions and
drawn/scan findings counts — `--story` only) and, from S2, an optional **`flow`** block for data
movement. `sim/extract.py`'s HTML scraping is deleted.

```
"flow": {
  "flow_version": 1,
  "fidelity": "syntactic",                  // never "runtime"
  "calls":   [ {from, to, callee, line, args:[{expr, type, dir, named}], unresolved:bool} ],
  "io":      [ {file, line, callee, op:"read"|"write", target:"file"|"env"|"net"|"stdio", literal} ],
  "signatures": [ {id, params:[{name, type, dir}], ret, unresolved:[...]} ]
}
```

Rules: flat arrays, sorted deterministically, no maps in output, omitted entirely when the story output
is not requested (existing scans stay byte-identical). Every new field is additive; nothing existing is
renamed, reordered or repurposed. Probe registry: `docs/atlas/core-probes.md`.

## 5. Honesty contract (extends `docs/research/atlas-run.md` §4)

| Surface | Allowed | Forbidden |
|---|---|---|
| Story frame | "candidate path", "static slice", "inferred order" | "execution", "executed", "the route taken", "call stack" |
| Data edge | `type-only`, `literal-observed` (path/URL from source), `unresolved` (greyed) | "flows at runtime", observed values, % of traffic |
| Quantity | call **sites**, loop depth, static cost units (v2) | widths/areas from call counts, ms, flame widths |
| Loop | `unknown-trip ×N — estimate incomplete` | "runs N times", "iterations" |
| Stop | caps + state flags | "program exited" |

Every beat states its inference class. Motion may not promote an inference to a fact. The prior-art
review is explicit that static "execution order" without a trace is the classic dishonesty
(see `docs/atlas/core-probes.md` → Evidence).

## 6. Story model

- **Scenes** = directories (top-level, then nested while >1 file); files are characters inside a scene.
- **Stages** = entry → path steps → leaf, derived from the existing depth/path data (probe P4).
- **Beats** (deterministic, fixed dwell, play/pause/step/scrub/speed/select/reset):
  1. `boot` — entry + election confidence + alternatives
  2. `enter` ×N — the path step, with **payload on the edge** (`Config → command.rs`)
  3. `loop` ×N — function, loop depth, trip provenance (`unknown ×1` floor)
  4. `branch` ×N — off-path edges named as omission
  5. `hottest` — file **and** hottest function with `L<line>` and ratio
  6. `profile` — KV3 queue: top non-test functions, "static candidate, verify"
  7. `start-here` — KV1's 3 actions verbatim, each `on-canvas` / `off-canvas`
  8. `stop` + `trust` — caps, partial parse, non-production share, unknown-trip floor
- Final frame is complete: every scene, every label, every number, the full trust line.

## 7. Scale

Target 1k–10k files. Scenes keep the view bounded; per scene show top-N by cost + counts; caps printed.
Story geometry must render at 1440×900 with no horizontal scroll at 1280×800. Scan + story generation
must stay inside the existing scan budget (no second parse pass over unchanged files).

## 8. Acceptance (runnable)

| ID | Test | Command / read |
|---|---|---|
| F1 | Final frame states the app's purpose in one plain sentence + entry + one `path:line` | blind read (`@vision`), recorded in evidence |
| F2 | Every drawn data edge carries a label from `type-only` / `literal-observed` / `unresolved` | DOM assertion in `sim/verify.py` |
| F3 | Unresolved indirection is visibly distinct and never guessed | fixture with a C function pointer + Rust `dyn` call |
| F4 | No forbidden claim (`execution`, `runs N times`, `ms`, `µs`, `runtime`, `speedup`) | grep test over story JSON + rendered DOM |
| F5 | Double scan → byte-identical `flow` block, `atlas-layout.json` and final frame | `sha256sum` diff, exit 0 |
| F6 | Existing scan output unchanged when story output is not requested | `report.html` byte-identical; `metrics.json` content-equal after removing scan provenance |
| F7 | 3,000-file synthetic repo: page ≤ 5 MB, first frame < 2 s local, no horizontal scroll | timed script |
| F8 | Founder A2: 3 actions in < 10 min on a holdout with animation on | founder session, recorded |

DoD gate: `cargo fmt --all -- --check` && `cargo clippy --locked --all-targets -- -D warnings` &&
`cargo test --locked` && `cargo deny check licenses` && `python3 sim/verify.py` → exit 0, plus F5/F6.

## 9. Build slices

- **S1 — pipeline truth (done 2026-10-24).** P4 (stages) + P5 (layout JSON); story consumes core
  JSON; `extract.py` deleted. No visuals change. *Proves Atlas reads Perfpal, not HTML.*
- **S2 — data on edges (done 2026-10-24).** P3 (signatures) + P1 (call payloads) → edge labels +
  inspector; grey unresolved. First slice that answers "what moves".
- **S3 — effects (done 2026-10-26).** P2 (I/O read/write with literal targets) → data stores on
  scenes; trust line. The core emits `flow.io[]` from a fixed v1 call-name table; a literal is only
  a plain source string (never a formatted value), and a non-literal target stays `null`/unknown.
- **S4 — scale + payoff (done 2026-10-26).** Directory scenes/aggregation at 3k files (P8), the
  whole-repo KV3 queue (P7) and the README purpose line (P6) feed the `profile` + `start-here` +
  `trust` beats; the 3k scale gate covers F2–F7. F8 (founder A2) is **deferred**, not claimed.

Each slice = one commit per probe + evidence + probe-registry row status flip.

## 10. Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Type resolution gaps in C (macros, implicit `int`, typedefs) | high | `unresolved` label; never infer a type from a name |
| Indirection (fn-ptrs, `dyn`) hides real data movement | high | grey + count them; F3 fixture; disclose N unresolved calls |
| 3k-file scenes still unreadable | medium | top-N per scene + counts; F7 budget |
| "Film" tempts a runtime claim | high | honesty table in §5 is a gate (F4) |
| Story scope creep into the static report | medium | companion first; report only after S2 proves value (ADR + A12 if interactive) |

## 11. Decisions (founder, 2026-10-24)

1. **`flow` emission: story-mode only.** Existing scans stay byte-identical (§4).
2. **Slice order: pipeline first.** S1 (P4 stages + P5 layout) shipped before data movement (S2/S3).
3. **Companion vs in-report: deferred to after S2.** If in-report with controls → new ADR + A12
   amendment (A12 already permits CSS/SMIL, not a second script).
   **Resolved 2026-10-26 (ADR-0011):** the story **remains a standalone companion**; it does not
   graduate into the static report, and A12 is unchanged. The founder-ratified graduation
   conditions (D2 session, step-13 report gate, deterministic player, explicit A12 amendment) are
   recorded in `docs/adr/ADR-0011-atlas-story-companion.md` Decision 4.
4. **CLI shape:** S1 ships `--story` on the existing command; the target entry point remains
   `perfpal story <repo>` (S4). **S4 status (2026-10-26):** S4 shipped the `--story` flag only; the
   `perfpal story <repo>` subcommand is **deferred**, not delivered, and is recorded as a deviation
   in `docs/atlas/core-probes.md`. The S4 acceptance does not claim the subcommand.
5. **`stages` placement:** derived stages ship inside `atlas-layout.json` (they are a projection of the
   same layout), not in the `flow` block; §4 was corrected 2026-10-24.
