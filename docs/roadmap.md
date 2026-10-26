# Perfpal — Roadmap (post-v0.1.0)

> Authoritative post-MVP roadmap. Supersedes the "Post-MVP Roadmap (direction only)"
> section of `docs/specs/perfpal-core-spec.md` (which listed C++/Python next) and the
> session-23 milestone sequence.
>
> **Re-based in session 35:** the founder direction recorded in session 34 is now formalized
> (spec `evidence-beta-spec`, plan `evidence-beta`, ADR-0003…0006). The pre-reset M1/M2/M3
> labels (`rust-backend` / `beta-release` / website) **no longer** correspond to the M1–M6+
> numbering below.

## Direction (formalized — session 35)

> Source of truth: `docs/specs/evidence-beta-spec.md` (repo scope). Decisions recorded as
> ADR-0003 (language order), ADR-0004 (cross-language deferral), ADR-0005 (evidence/publish
> gate), ADR-0006 (shipping scope). Execution plan: `docs/plans/evidence-beta.md` (M1–M6+).

**Goal:** ship Perfpal's first *evidence-backed* release — a polished beta whose pitch is
**"Perfpal audits what AI wrote"**: **verifiable truth** per finding, **benchmark evidence**,
and a **verified showcase** on large **AI-assisted** codebases. (The "100%-AI-written
C/Rust" framing is dropped — it is unsubstantiable: no famous large mostly-AI-generated
C/Rust OSS exists.)

**Publish gate (ADR-0005):** **NO release** until (a) the benchmark suite is built **and run**,
and (b) the AI-code credibility showcase is **verified**.

**Language order (ADR-0003):** **C → Rust → Python → JS → TS → C++** (Python promoted, C++
demoted — data: AI-generated code share Python 41.7%, JS 15.4%, C++ 7.8%, TS 7.8%, C 3.7%, Rust not
top-8; arXiv 2603.27130). JS and TS are **separate** milestones.

**Cross-language linking (ADR-0004):** mixed-language coupling is a **separate milestone
AFTER** the language sequence — **not** in M1–M5; language-tagged analysis (no cross-language
edges) stays as-is until then.

**Sequence to first release:**

| # | Node | Purpose |
|---|---|---|
| M1 | Precision Gate (C+Rust) | kill the 5 noise rules (≈89% of self-scan volume) + per-rule precision KPI |
| M2 | Benchmark suite — **build + RUN** | mutation ground-truth + scale ladder + determinism proof |
| M3 | Python backend | unlocks the popular AI-generated repos |
| M4 | AI-code showcase | verified findings on **Aider** (Apache-2.0, 88% self-written) + **chunkhound** (MIT, 100% AI) + a famous human-written C/Rust repo for scale |
| M5 | **First release** | gated on M2 run + M4 verified; un-parks `beta-release` |
| M6+ | JS → TS → C++ ; then cross-language linking | fast-follow reach |

**Evidence discipline ("verifiable truth"):** a repro packet (pinned URL+commit+license,
one-command scan, expected counts, determinism proof) + hand-verified true positives +
an explicit accepted-limitations page. Showcase only **MIT/Apache-2.0/BSD** with minimal
attributed spans.

**Shipping scope (T0 + T2 — ADR-0006):** GitHub Releases with the existing **6 unsigned
binaries** (linux musl x86_64/aarch64, linux gnu, macOS x86_64/aarch64, windows msvc) +
minisign `SHA256SUMS` + SBOM + legal docs; **packaging** = winget (`portable`), own Scoop
bucket, own Homebrew tap (macOS+Linux), AUR `-bin`, cargo-binstall. **Signing/notarization
(T1) DEFERRED to GA** — Azure Artifact Signing is likely ineligible (Bangladesh is not a
listed country; Individual Public Trust is US/Canada only), signing does **not** silence
SmartScreen (reputation accrues over weeks), and macOS notarization needs $99/yr + a
`.pkg`/`.dmg`.

## Status snapshot (session 45)

| Item | Value |
|---|---|
| Published | **`v0.1.0`** (tag `v0.1.0`, commit `f368133`) — the only published release |
| Parked by founder | **`v0.2.0-beta.1`** — code complete (`beta-release` B0–B6, commit `b3a1105`); **publishing is held** until the benchmark is run + the AI-code showcase is verified (see Direction). Still needs the EULA/privacy **contact address** + **`MINISIGN_SECRET_KEY`**/**`MINISIGN_PASSWORD`** secrets at tag time |
| `Cargo.toml` version | `0.2.0-beta.1` (unreleased) |
| `metrics.json` schema | **1.8** |
| Languages supported | **C** + **Rust** + **Python** + **JavaScript** + **TypeScript/TSX** (`.c`/`.h`/`.rs`/`.py`/`.pyi`/`.js`/`.jsx`/`.mjs`/`.cjs`/`.ts`/`.mts`/`.cts`/`.tsx`; `.ipynb` and declaration-only `.d.*` excluded) |
| Specs | `perfpal-core-spec` (core product) · **`evidence-beta-spec`** (direction; post-MVP roadmap) |
| ADRs | `0001` parser · `0002` visualization · **`0003` language order** · **`0004` cross-language deferral** · **`0005` evidence/publish gate** · **`0006` shipping scope** |
| Plans | `mvp-implementation` ✅ · `housekeeping-batch` ✅ · `semantic-resolver` ✅ · `guarded-scan-fix` ✅ · `rust-backend` ✅ · `beta-release` ✅ code (B0–B6; publish PARKED) · **`evidence-beta`** (active — M1 ✅ · M2 ✅ · M3 ✅ · **M4 ✅ record**) · **`python-backend`** ✅ COMPLETE (W0–W4) · **`js-ts`** (active — B2 ✅ · B3 ✅ · B4 docs/canary; merge pending founder approval) |

## Milestone sequence

Ordered. One milestone = one or more focused sessions; one task per session. The milestone
steps are owned by plan `evidence-beta` (`docs/plans/evidence-beta.md`).

### M1 — Precision Gate (C+Rust) *(dispositions + KPI harness landed)*
The existential milestone. The self-scan is ~89% noise (`docs/evidence/dogfood-rust-backend.md`:
1219 findings, 5 rules dominate). Triage and disposition the 5 rules — ALLOC-001 per-finding
`format!`/`to_string`, ARCH-003 name-based `new` coupling, STR-001 `Vec::push` collision,
MEM-002 `.clone` copies, LOOP-008 member chains — deciding per rule to **suppress**, **reframe**
(severity/confidence), or **fix**.
- **Status — 5 dispositions LANDED.** `ALLOC-001` now aggregates one finding per innermost loop (with literal-receiver suppression), `ARCH-003` name-based edges require a single-definition-file match, `MEM-002` dropped `clone`/`to_vec`, `LOOP-008` requires a `Subscript` element, and `STR-001` is retired for Rust (Rust tier = 22 rules). Self-scan `perfpal src` (33 files): **1219 → 304 findings** (658 hot / 214 warn / 347 info → 76 / 97 / 131); the 5 rules **1084 → 168** (`ALLOC-001` 511→74, `ARCH-003` 204→90, `STR-001` 145→0, `MEM-002` 128→0, `LOOP-008` 96→4).
- **Precision KPI harness LANDED.** Labeled ground-truth (mutation) corpus (`tests/fixtures/precision/`, C+Rust) + `tests/precision_kpi.rs` runs the REAL full pipeline and ratchets per-rule precision/recall against a committed baseline; all 9 C+Rust M1 gate rules measure **1.0/1.0** under both the default and `semantic-clang` builds. Evidence: `docs/evidence/precision-baseline.md`. Goldens regenerated (`report-rust`, `report-mixed`; C `report` unchanged) and byte-idempotent.
- **Close-out** — dual review (@reviewer + @requirements-reviewer) + founder-approved commit.

### M2 — Benchmark suite (build + **RUN**)
Ground-truth (mutation) fixtures + a **scale ladder** (small/medium/large corpora) + a
**determinism proof** (byte-stable artifacts across runs). **Run it** and record results as
evidence in `docs/evidence/`. Hard precondition for the publish gate (ADR-0005): the metric is
measured precision/recall against planted ground truth — never designed-on-paper.
- **Status — BUILT and RUN.** A deterministic in-code mutation catalog (`tests/benchmark/corpus.rs`) + harness (`tests/benchmark/mod.rs`, entry `tests/benchmark.rs`) run the **REAL** pipeline (`cli::run_with`) over a three-rung ladder: `small` **142 files** plants a TP + silent control for **all 78 active rules** (C + Python + Rust + JS/TS) (+ the retired `PERF-RS-STR-001` always-silent control); `medium` **132 files** and `large` **418 files** replicate the **nine** existential M1 gate rules with per-replica unique names. Per-rule TP/FP/FN → precision/recall **ratchets** against `tests/fixtures/benchmark/baseline.json` (intended 1.0/1.0, never lowered), with a no-vacuous-cell guard, a 78-id coverage lock, and a stale-baseline check. All rules measure **1.00/1.00** on every rung. A **byte-determinism proof** runs every rung (plus an unlabeled mixed `src/`+`examples/`+`examples-rust/`+`examples-python/` real-code rung) twice under a **provenance-only** mask (metrics.json: only `scan.started_at`/`duration_ms`/`root_path` textually neutralized; report.md: provenance/root only, the §4 block NOT stripped; charts + bundled schema byte-exact) → **PASS**. All corpora parse cleanly (0 partial/failed); the ladder is always-on at **~4.3 s**. Evidence: `docs/evidence/benchmark-suite.md`. IR-only semantic sub-cases (proven-`fclose`, resolved-format, resolved-edge; ARCH-001 deep-chain/hub) are covered by the `tests/fixtures/ir/` fixtures + `semantic-clang`, not plain source.
- **Close-out** — dual review (@reviewer + @requirements-reviewer; reviewer CHANGES_REQUESTED → fix wave → re-review **APPROVED**) + founder-approved commit.

### M3 — Python backend
Add a Python scanner backend over the official **tree-sitter-python** grammar (MIT, exact pin
per ADR-0001 G5), mirroring the **full catalog** with **`PERF-PY-*`** rule ids and honest
reframing to Python idioms. Schema **MINOR bump** admitting the `"python"` language token
(`rule_id` pattern is already open, modules.md §5.2). Add an `examples-python/` corpus. The
`semantic-clang` resolver tier stays **C-only**.
- **Status — COMPLETE (`python-backend` W0–W4).** W0 canary (rule-frequency evidence) → W1
  grammar pin + design record → W2 scanner backend + binding/alias inference + import
  resolution → W3 the **22-rule `PERF-PY-*` catalog** (13 reframed mirrors + 9 Python-native;
  10 C/Rust rules retired) + schema **1.2 → 1.3** → W4 the proof package: `examples-python/`
  (20-file real package) fires **all 22** rules exactly (29 findings, 0 hot / 12 warn / 17 info;
  `tests/examples_python_coverage.rs`), a 6-rule Python precision KPI at 1.0/1.0
  (`tests/fixtures/precision/python/`), the benchmark `small` rung wide to 67 rules (at Python W4; later
  78 with the JS/TS track), and a 4th
  golden tree `report-python`. Python is syntactic-tier only.

### M4 — AI-code credibility showcase
Scan the flagship AI-authored repos — **Aider** (Apache-2.0, ~88% self-written) and
**chunkhound** (MIT, 100% AI) — plus a famous human-written C/Rust repo for scale. **Hand-verify**
true positives; produce a **repro packet** per repo (pinned URL+commit+license, one-command
scan, expected counts, determinism proof) and an **accepted-limitations page**. Verification of
this showcase is a hard precondition for the publish gate (ADR-0005).
- **Status — EVIDENCE RECORD COMPLETE.** Four pinned repos scanned — Aider `v0.86.0`
  (`a4be6ccd…`), chunkhound `v6.0.0` (`a8553ae6…`), ripgrep `14.1.1` (`4649aa97…`), curl
  `curl-8_11_1` (`75a2079d…`) — **2056 files / 22010 findings**, all runs exit 0 and
  `metrics.json` byte-identical **modulo provenance** (`started_at`/`duration_ms`). Hand-verified
  **stratified sample (174 findings): 70% TP / 26% FP / 4% AMBIG**. Deliverables:
  `docs/evidence/ai-code-showcase.md` (umbrella), `docs/evidence/showcase-{aider,chunkhound,ripgrep,curl}.md`
  (repro packets), `docs/evidence/showcase/<repo>/{counts.json,verified.json}`, and
  `docs/evidence/accepted-limitations.md`. Honest headline: **72% of volume is the single
  `ARCH-003`** name-based coupling rule; name-matching rules (`ARCH-003`/`CALL-001`/`MEM-002`) are
  the FP engine; the **Rust backend is weakest** (ripgrep 42% TP). Evidence only — no product code.

### M5 — First release — un-parks `beta-release`
With M2 run **and** M4 verified, clear the residual beta blockers (EULA/privacy **contact
address** + **`MINISIGN_SECRET_KEY`**/**`MINISIGN_PASSWORD`** secrets) and publish
**`v0.2.0-beta.1`** as a clearly-labelled GitHub **pre-release** (`--prerelease --latest=false`,
never displacing `v0.1.0` as *Latest*), then execute **T2 packaging** (ADR-0006). The release
candidate itself is already built by plan `beta-release`:
- Hardening: `[profile.release]` `opt-level=3`, fat LTO, `codegen-units=1`, `strip="symbols"`,
  `panic="unwind"` kept deliberately (the CLI maps a caught panic to exit 3 via
  `std::panic::catch_unwind`, modules.md §8.2); `--remap-path-prefix` in every release build.
  `strip` + path-remapping are size/path hygiene, **not** obfuscation; `obfstr` **not adopted**
  (no sensitive literals compiled in).
- Integrity: per-target `.sha256` + aggregate **minisign-signed** `SHA256SUMS`
  (`SHA256SUMS.minisig`; public key `minisign.pub` committed). GitHub **artifact attestations
  deferred** (need GitHub Enterprise Cloud on private repos; the beta repo stays private).
- **SBOM** (`cargo-sbom`, SPDX 2.3) + **THIRD-PARTY-NOTICES** (`cargo-about`) attached by the
  tag-time pipeline; `cargo-deny` GPL deny-list remains the standing enforcement gate.
- **Legal bundle** (`LICENSE` + `EULA.md` + `PRIVACY.md`) attached; noncommercial evaluation
  notice surfaced in `--help` only (`--version` byte-exact); `docs/release-notes-header.md`
  prepended to generated notes.
- **Tag discipline:** GitHub runs the workflow file from the tagged commit, so the tag must point
  at the commit containing the current `.github/workflows/release.yml`; re-point a not-yet-validated
  tag after any workflow edit, and for an **already-published** tag delete/replace the release
  assets first (the publish step never rewrites a published release's assets).
- **Out of scope at beta:** deep obfuscation (no viable license-clean Rust tooling) and
  license-key/activation (deferred to paid GA).
- Plan: `docs/plans/beta-release.md` (linked `perfpal-core-spec`).

### M6+ — Fast-follow reach *(JS/TS started; C++ deferred)*
**JS → TS → C++** backends as separate milestones (ADR-0003), then **cross-language linking** as
its own milestone **after** the language sequence (ADR-0004). Status: **JS + TS/TSX are implemented**
on branch `feat/js-ts` (plan `js-ts` B2–B4: 11 shared `PERF-JS-*` rules, six language tokens,
schema v1.8, `examples-js`/`examples-ts` + `report-js`/`report-ts` goldens); the merge is a later
release, non-blocking for the beta and gated on founder approval. **C++ remains deferred.** Each
remaining language becomes its own plan/session family when reached; recorded here so scope cannot
creep into M1–M5.

### Website (separate repo, separate session family)
Attractive showcase site covering the **full scope** (shipped + planned). **Different repo:**
`~/Perfpal-Website`, versioned independently. Source of truth: `docs/website/scope-and-content.md`
(brief + sitemap + page content spec) in THIS repo. Not tracked by a plan here.

## Map-first track (post-I5, founder-directed 2026-10-21)

`map-build` ended at **I5** (I1–I5; there is no I6). The I5 human A5 test **failed** —
the founder judged the visuals unintelligible — so I5 is not closed and the map work moves to
the successor plan/spec `map-intelligibility`:

1. **Diagnose** with a mandatory `@vision` loop: is the gap data (H1) or visualisation (H2)?
2. **Redesign the key views** as answer-first (one question each, plain language); keep dense
   stems only as labelled explorers.
3. **Vision-gate and founder-test** every key view (unfamiliar repo, 3 actions, <10 min).
4. **Re-triage** R-pf-098 (entry strata), R-pf-099 (production/literal-trip ranking),
   R-pf-100 (tiny-scope density) against the new visual/data decisions.

This track runs **before** the JS/TS backends and the paper queue (a core feature nobody can
read is not shippable). M5 remains gated by B-pf-001/B-pf-002 and is otherwise unaffected.

## Sequencing rationale
- **Precision before exposure** (M1) — a noisy first impression on a famous repo is
  irrecoverable (ADR-0005).
- **Benchmark before release** (M2) — the pitch is a truth claim; measured precision is the
  evidence.
- **Python at M3** — the highest AI-generated-code share unlocks the flagship showcase (ADR-0003).
- **Release at M5** — the already-built beta simply waits behind the gate; nothing is discarded.
- **Website is orthogonal** — its own repo, parallelizable without touching the product.

## Deferred (unchanged priority, not cancelled)
- **C++ / JS / TS backends** — sequenced M6+ (ADR-0003).
- **Cross-language linking** — its own milestone after the language sequence (ADR-0004).
- **T1 code signing / notarization** — deferred to GA (ADR-0006).
- **LOOP-004 resolver-constness** follow-up (semantic tier).
- **Runtime-profile ingestion** (CONFIRMED-1).
- Release follow-ups: Node-24 action pins, `/Brepro` + darwin-x64 binary reproducibility,
  toolchain-pin drift guard.
- `stats.distributions` activation (MINOR bump on proposal acceptance).
- Housekeeping: Cargo.toml PolyForm SPDX alignment; `modules.md` §8 CLI doc drift.
- R5 deferred test-hygiene nits (mirror const import, control-needle uniqueness,
  `copy_tree_excluding` symlink skip, panic-path temp-dir RAII, `materialize_fixture` descriptor).
- Optional: `examples/README.md`; corpus extension to fire the ARCH-001 hub-fanout sub-case.

## Versioning
- `0.1.0` released → `0.2.0` (Rust backend, unreleased; the beta is its prerelease) →
  **`0.2.0-beta.1`** (official beta, prerelease, publish parked) → later betas/GA as scoped.
- `metrics.json` schema follows its own `MAJOR.MINOR` (currently **1.8** — the Python backend
  landed the 1.2 → 1.3 MINOR that admits the `"python"` token, and the JS/TS track landed the additive
  1.7 → 1.8 MINOR that admits `"js"`/`"ts"`/`"tsx"`; see `docs/schema/metrics-json-v1.md` §5).
