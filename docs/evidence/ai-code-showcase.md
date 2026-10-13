# M4 — AI-code credibility showcase (plan `evidence-beta`)

> **Scope.** `evidence-beta` **M4** is the ADR-0005 publish-gate precondition: *an AI-code credibility
> showcase is verified* — findings hand-checked on real AI-authored repos. This is the umbrella record.
> Per-repo repro packets: `docs/evidence/showcase-aider.md`, `showcase-chunkhound.md`,
> `showcase-ripgrep.md`, `showcase-curl.md`. The honest defect catalogue: `docs/evidence/accepted-limitations.md`.
> Machine-readable counts + verdicts live beside each packet under `docs/evidence/showcase/<repo>/`.
>
> **Pitch (spec §1).** *"Perfpal audits what AI wrote"* — find performance landmines in large
> **AI-assisted** codebases. A pitch about *verifiable* findings cannot ship on unverified ones.
>
> **Reproduce:** §7. **No product code changed** — this milestone produces evidence only.

---

## 1. What the gate requires (ADR-0005 / spec §6)

No release until **both** hold: (a) the benchmark suite is built AND run (M2 ✅, `docs/evidence/benchmark-suite.md`),
and (b) an AI-code credibility showcase is **verified**. Evidence discipline for (b):

- a **repro packet per repo** — pinned URL + commit + license, a one-command scan, expected finding
  counts, a determinism proof;
- **hand-verified true positives** — claims rest on findings a human checked, not raw counts;
- an **accepted-limitations page** — an explicit, honest list of approximate/FP classes;
- **showcase licensing** — MIT / Apache-2.0 / BSD targets only, minimal attributed spans.

## 2. Corpora (all pinned)

| Repo | Purpose | License | Tag → pinned commit | Language | Files scanned |
|---|---|---|---|---|---|
| Aider | flagship AI-authored (≈88% self-written) | Apache-2.0 | `v0.86.0` → `a4be6ccd…b626` | Python | 148 |
| chunkhound | 100% AI-authored | MIT | `v6.0.0` → `a8553ae6…cc7d` | Python + Rust | 887 |
| ripgrep | famous **human**-written, for scale | MIT / Unlicense | `14.1.1` → `4649aa97…e16c` | Rust | 98 |
| curl | famous **human**-written, for scale | curl (MIT-like) | `curl-8_11_1` → `75a2079d…7844c` | C + Python (tests) | 923 (236 partial) |

Total: **2056 files / 22010 findings**.

## 3. Dry-run output (syntactic tier, `perfpal <repo> -o out`, exit 0)

| Repo | Findings | hot | warn | info | distinct rules | parse health |
|---|---:|---:|---:|---:|---:|---|
| Aider | 1567 | 0 | 256 | 1311 | 15 | 148 ok |
| chunkhound | 8124 | 26 | 1125 | 6973 | 31 | 887 ok |
| ripgrep | 1315 | 40 | 90 | 1185 | 13 | 98 ok |
| curl | 11004 | 225 | 1308 | 9471 | 32 | 687 ok / 236 partial |
| **Total** | **22010** | **291** | **2779** | **18940** | — | — |

No run produced a `hot` finding on Aider — expected: the Python tier is conservative and Aider is
well-factored code. Findings never affect the exit code (they are data); every run exited **0**.

## 4. Hand-verification

**Method.** For each repo a **stratified sample** was drawn from run 1: up to 3 findings for the
dominant rule plus up to 2 per other rule, preferring `hot` → `warn` → `info`. Each finding was opened
at its span in the pinned checkout and judged against the rule's stated semantics:

- **TP** — the claimed pattern actually holds in the code;
- **FP** — the claim is wrong (misread / name collision / wrong construct);
- **AMBIG** — the pattern is technically present but harmless or context-dependent.

The sample is **stratified, not uniform**, so the aggregate is a per-rule quality signal, **not** a
whole-corpus precision estimate. Raw verdict rows: `docs/evidence/showcase/<repo>/verified.json`.

| Repo | Sample n | TP | FP | AMBIG | TP of sample |
|---|---:|---:|---:|---:|---:|
| Aider | 30 | 20 | 6 | 4 | 67% |
| chunkhound | 58 | 40 | 17 | 1 | 69% |
| ripgrep | 26 | 11 | 14 | 1 | 42% |
| curl | 60 | 51 | 8 | 1 | 85% |
| **Aggregate** | **174** | **122** | **45** | **7** | **70%** |

**70% TP / 26% FP / 4% AMBIG** across 174 hand-checked findings (TP/(TP+FP) = 73%). Every FP and AMBIG
row is enumerated with a decisive span in the per-repo packets — the cranky truth, not a sales pitch.

## 5. Honest findings (the point of the gate)

### 5.1 Volume is dominated by one accepted-FP-class rule
Across all four repos, **15,850 of 22,010 findings (72.0%) are the single `ARCH-003` rule** (name-based
cross-file coupling, always `info`, `LOW` confidence). All-language, it is 68% of Aider, 76% of
chunkhound, 65% of ripgrep, and 71% of curl. Adding `ARCH-001`/`ARCH-002` pushes the architecture family
past 82% on curl. **Any unread scan therefore reads as noise even when the loop/allocation core is clean.**
This is the same class M1 identified on the self-scan; on real code it is now *quantified*.

### 5.2 Name-based rules are the FP engine
The deterministic, name-matching rules produce most of the observed FPs:

- `ARCH-003` (PY/C/RS) — real cross-file calls are TPs (`aider/analytics.py:199`,
  `curl/lib/asyn-thread.c:191/270`), but duck-typing / libc collisions are FPs (`print` vs
  `InputOutput.print`; `strdup` ↔ `src/tool_strdup.c`).
- `CALL-001` (recursion) — `0/4` TP on Rust and `1/6` on Python: it fires on `super()`, `HashSet::new`,
  `Connection::open`, `self.progress.add_task` — **method-name collisions, not recursion**.
- `MEM-002` (PY) — fires on `shutil.copy`, a **filesystem** copy, not `copy.copy`/`deepcopy`.

### 5.3 The Rust backend needs precision work
ripgrep is the weakest target (42% TP). Systematic Rust FPs: `CALL-001` (name collisions),
`STR-002` (flags O(1) `len()`/`is_empty()`), `TYPE-001` (treats a by-value **move** as a copy),
`TYPE-002` (claims field padding on structs that are not `#[repr(C)]`/FFI), `STR-003` (`File::set_len`
read as a string conversion). These are concrete follow-up targets, not hidden.

### 5.4 What is strong
The **allocation / loop / string / async core is uniformly true-positive** in the sample: Python
`ALLOC-003` (6/6), `LOOP-001` (6/6), `LOOP-007` (6/6), `LOOP-009` (6/6), `CALL-002` (6/6), `ASYNC-001/002`
(5/5), `PANDAS-001` (2/2); Rust `CALL-002` (4/4), `LOOP-001` (4/4), `LOOP-008` (4/4), `ALLOC-001` (4/4);
the C backend's `ALLOC-001/002/003`, `LOOP-001/002/003/005/007/008`, `MEM-002`, `STR-003`,
`CALL-001/002` are TP across the board (the one exception among these core rules is C `STR-002`,
**1 TP / 1 FP** — `crawler.c:151` tests a string reassigned each iteration; C `LOOP-004`/`LOOP-006` are
separately disclosed in `accepted-limitations.md`). The high-confidence, severity-ranked signal a user acts on first
is the signal that holds up.

## 6. Verdict

The ADR-0005 showcase precondition is **satisfied as an evidence record**: four permissively licensed
repos scanned at pinned commits, deterministic outputs, and findings hand-checked with every FP
disclosed. The record is deliberately un-flattering where the tool is weak (Rust name-based rules;
architecture volume) — that honesty is the deliverable the gate asks for, and it seeds the follow-ups
in `docs/evidence/accepted-limitations.md`.

**Not** done here (out of M4 scope): fixing the FP classes (they become follow-up work), and any
`cargo`-gate re-run (this milestone changes no product code).

## 7. Reproduce

```bash
# 1. Fetch the pinned corpora (outside the product repo)
git clone --depth 1 --branch v0.86.0    https://github.com/Aider-AI/aider.git      aider
git clone --depth 1 --branch v6.0.0     https://github.com/chunkhound/chunkhound.git chunkhound
git clone --depth 1 --branch 14.1.1     https://github.com/BurntSushi/ripgrep.git  ripgrep
git clone --depth 1 --branch curl-8_11_1 https://github.com/curl/curl.git         curl
git -C aider rev-parse HEAD        # a4be6ccd87ebaa59b361f3f028d116ce1761b626
git -C chunkhound rev-parse HEAD   # a8553ae644686d8ee55b94ee74ff560a38e6cc7d
git -C ripgrep rev-parse HEAD      # 4649aa9700619f94cf9c66876e9549d83420e16c
git -C curl rev-parse HEAD         # 75a2079d5c28debb2eaa848ca9430f1fe0d7844c

# 2. Build + scan (release, locked)
cargo build --release --locked
./target/release/perfpal aider      -o out-aider
./target/release/perfpal chunkhound -o out-chunkhound
./target/release/perfpal ripgrep    -o out-ripgrep
./target/release/perfpal curl       -o out-curl
```

**Determinism.** Each repo was scanned **twice** into separate output dirs; `metrics.json` is
byte-identical **after masking the documented provenance fields** `scan.started_at` and
`scan.duration_ms` (the same contract the golden tests assert). Per-repo masked `sha256` is recorded in
packet §4. The corpora are pinned by commit, so counts reproduce without a lockfile.

## 8. Provenance

- **Plan:** `evidence-beta` M4 · **Spec:** `evidence-beta-spec` §1/§6 · **ADR:** ADR-0005.
- **Binary:** `perfpal 0.2.0-beta.1 (metrics schema 1.3)`, `cargo build --release --locked`.
- **Tier:** syntactic (default). Python/Rust are syntactic-only; C here is also syntactic (no libclang).
- **Artifacts:** `docs/evidence/showcase-{aider,chunkhound,ripgrep,curl}.md`;
  `docs/evidence/showcase/<repo>/{counts.json,verified.json}`; this umbrella; `accepted-limitations.md`.
- **No product code changed.** No `src/`, `Cargo.toml`, schema, or golden touched.
