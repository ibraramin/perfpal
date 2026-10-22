# V6 — founder timed eye-test protocol (A2 / A2b)

**Purpose.** The same A5 bar that failed at I5, run by the founder on the untouched
django/postgres holdout: from the key views alone, name **3 concrete actions in under
10 minutes** and state whether the map is understandable. No agent-only substitute closes
this gate.

**Build under test.** HEAD after the V6a KV2 production-first amendment (commit recorded in
`result.json`). Binary: `target/release/perfpal`. Report hashes + parse health:
`preflight.json`.

**Holdout release.** This is the first and only release of the holdout for the map loop.
It is used **only** for this test: not for design/tuning, not for agent reads, not for
`@vision` screenshots. The timed repo is **postgres**; **django** is an untimed smoke read
afterward.

## Viewing conditions (fixed, or the test is meaningless)

- Open `perfpal-test/out-v6-postgres/report.html#map` in a browser, window maximized,
  **100% zoom** (`Ctrl+0`). Canonical viewport 1440×900.
- Use **only the three key views** on `#map`: *Where do I look first?* / *What breaks if I
  touch it?* / *Where might time plausibly go?*
- Do **not** open the Explorer stems, per-component pages, or drill artifacts for the three
  actions; you may note them after the timer stops.
- Do **not** read the source, use an LLM/chat, or ask an agent for help. The test is the
  map alone.
- Start the clock when the report is open and you begin reading; stop at 10:00 even if you
  have fewer than 3 actions (that counts as FAIL on the time bar).

## Task

1. Name **3 concrete actions** you would take, in your own words, each tied to what the map
   shows (a `path:line`, a file, or a function, plus why).
2. Say whether the map is **understandable** (yes/no) and give your words.
3. Note any blocker that stopped you.

Record: start time, end time, elapsed, the 3 actions **verbatim**, the understandable
verdict + verbatim words, and any protocol deviation. Fill `result.json` from
`result.template.json`; the elapsed time is authoritative (self-reported plus wall-clock
cross-check).

## Verdict rule

**PASS** iff `elapsed_seconds < 600` AND exactly 3 concrete actions are named AND
`understandable = true`. Anything else is **FAIL**. A FAIL re-opens V3 with a **distinct
idiom**; two distinct idiom failures stop the polishing loop and escalate to an ADR-0008
amendment.

## Guardrails

- No re-litigation of the V5 blind reads. If the run exposes a defect, record it and re-test
  after a fix; do not tune against the holdout.
- Do not mark `map-build` done; do not claim A1b closed. This test closes A2/A2b only.
