# Spec: evidence-beta-spec

Scope: repo

# Perfpal — Evidence-First AI-Code Beta (repository scope spec)

> Formalizes the founder direction recorded in session 34 (`docs/roadmap.md` `## Direction`,
> the session-34 direction entries in `state/session-state.json`). Complements — does not replace —
> `perfpal-core-spec` (product vision, pillars, MVP definition, license constraint). Where the
> two touch the post-MVP roadmap, THIS spec governs.

## 1. Positioning (the pitch)
Perfpal's first *evidence-backed* release is pitched as **"Perfpal audits what AI wrote"**:
find performance landmines in large **AI-assisted** codebases. The prior "100%-AI-written
C/Rust" framing is **dropped** — it is unsubstantiable (no famous large mostly-AI-generated
C/Rust OSS exists).

Cornerstones (all three required before any release):
- **Verifiable truth** per finding (reproducible, hand-checkable).
- **Benchmark evidence** (the suite is built AND run, not merely designed).
- **Verified showcase** on large AI-assisted codebases.

## 2. Publish gate (hard)
**NO release** — including the already-code-complete `v0.2.0-beta.1` — until BOTH hold:
1. The **benchmark suite is built AND run** (results recorded as evidence).
2. An **AI-code credibility showcase is verified** (findings hand-checked on real repos).

The existing `beta-release` plan (B0–B6 code complete, publish parked) becomes the **M5
release node** under this gate. Its residual blockers remain: the EULA/privacy **contact
address** and the **`MINISIGN_SECRET_KEY` / `MINISIGN_PASSWORD`** repo secrets.

## 3. Sequence to first release
| # | Node | Purpose |
|---|---|---|
| M1 | Precision Gate (C+Rust) | kill the 5 noise rules (≈89% of self-scan volume) + per-rule precision KPI |
| M2 | Benchmark suite — build + RUN | mutation ground-truth + scale ladder + determinism proof |
| M3 | Python backend | unlocks the most common AI-generated language |
| M4 | AI-code showcase | verified findings on Aider (Apache-2.0, 88% self-written) + chunkhound (MIT, 100% AI) + a famous human-written C/Rust repo for scale |
| M5 | First release | gated on M2 run + M4 verified; un-parks `beta-release` |
| M6+ | JS → TS → C++ ; then cross-language linking | fast-follow reach |

## 4. Language order (supersedes the prior order)
**C → Rust → Python → JS → TS → C++** (Python promoted, C++ demoted). Rationale is
data-driven: AI-generated-code share Python 41.7%, JS 15.4%, Java 8.6%, C++ 7.8%, TS 7.8%,
C 3.7%, Rust not in top-8 (arXiv 2603.27130). **JS and TS are separate milestones.**
C already ships; Rust is implemented (`rust-backend` R0–R6 complete).

## 5. Cross-language linking (deferred)
Mixed C/Rust (and later cross-language) coupling/ARCH analysis is a **separate milestone
AFTER** the language sequence — **not** part of M1–M5. Language-tagged analysis (no
cross-language call edges; the `(language, name)` definition-map key) stays the behavior
until that milestone explicitly changes it.

## 6. Evidence discipline ("verifiable truth")
- **Repro packet** per showcased result: pinned repo URL + commit + license, a one-command
  scan invocation, expected finding counts, and a determinism proof.
- **Hand-verified true positives**: claims rest on findings a human checked, not raw counts.
- **Accepted-limitations page**: an explicit, honest list of the tool's approximate/FP
  classes (the cranky truth — e.g. `format!`/`to_string` ALLOC-001 volume, name-based
  ARCH-003 coupling).
- **Showcase licensing**: only **MIT / Apache-2.0 / BSD** repos, with minimal attributed spans.

## 7. Shipping scope
- **T0 + T2 shipping.** T0 = GitHub Releases with the existing **6 unsigned binaries**
  (linux musl x86_64/aarch64, linux gnu, macOS x86_64/aarch64, windows msvc) + minisign
  `SHA256SUMS` + SBOM + legal docs. T2 = packaging: winget (`portable`), own Scoop bucket,
  own Homebrew tap (macOS+Linux), AUR `-bin`, cargo-binstall.
- **T1 (code signing / notarization) DEFERRED to GA.** Rationale: Azure Artifact Signing
  likely ineligible (Individual Public Trust = US/Canada only), signing does not silence
  SmartScreen (reputation accrues over weeks), macOS notarization needs $99/yr + a
  `.pkg`/`.dmg`.
- The release is a clearly-labelled GitHub **pre-release**, never displacing `v0.1.0` as *Latest*.

## 8. Constraints (carried from perfpal-core-spec)
- **No copyleft (GPL-family) dependencies** anywhere in the core — `cargo-deny` allow-list
  stays the enforcement gate.
- **Source-available proprietary** (PolyForm Strict + EULA); repo stays **private** at beta.
- Strictly **static analysis**; outputs framed as optimization *candidates*; runtime-profile
  ingestion is post-MVP.

## 9. Non-goals
Public/GA product, paid licenses/activation, deep obfuscation, cross-language linking
(pre-M6), C++/JS/TS backends (pre-M6), T1 signing (pre-GA), any release before the gate clears.