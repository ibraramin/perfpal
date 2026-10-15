# Spec: divergence-study

Scope: repo

# Divergence Study Spec (repo scope)

Perfpal is a deterministic measurement instrument for human-vs-AI perf-pattern divergence, not a bug-finder for mature code.

## Claim
AI-era code diverges structurally from human code (call-shape, idiom/clone rate, error-vs-hot stratification, alloc-family histogram). Report rates and distributions, never raw counts. Never claim AI is worse.

## Instrument revamp (IR v1.4 additive only)
- Per-function call fanout/fanin/depth/recursion + intra-file resolved edges emitted as stats, not findings.
- Token-normalized idiom hash + clone buckets seeded by deep-dive families (to_string-per-row, collect-per-embedding, range(len()) vs enumerate).
- error_path flag: try/except, ?/Err-return, do-while(0), #if 0 / cfg(test) gating, error-branch free.
- Alloc-family histogram: real_alloc vs empty_ctor (String::new/vec![]) vs serialize/format + invariant/variant split.
- LOOP-002 message split: hoistable vs intrinsic streaming I/O.
- Populate stats.distributions[] (today sanctioned-but-empty) + DOT export + cost overlay. Charts stay pure-fn-of-JSON. Byte-determinism preserved (sorted traversals, preserve_order OFF).

## Corpus
- Longitudinal: same repo pre-2022 SHA vs post-2024 SHA.
- Cross-sectional: AI-native vs human-mature matched on language x size x domain, 8-12 repos x2 min.
- Every cell: URL+SHA+license+scan command+expected counts+double-scan byte-identical proof.
- Reuse M4 pins as anchors (Aider a4be6cc, chunkhound a8553ae, ripgrep 4649aa9, curl 75a2079d).
- Stratify per-language (Python retires 10/23 premises; no cross-language pooling).

## Stats bar (MSR/JSS)
- Unit = repo-snapshot, cluster-robust/hierarchical, never pooled t-tests.
- Pre-register 4-5 primary dims + FDR over 67-rule exploratory set.
- Mann-Whitney + Cliff delta + bootstrap CIs, Holm/BH, power calc (hot-only underpowered, use info-tier).
- Blinded dual-triage + Cohen kappa. Threats: construct (9/181 genuine), internal (triage subjectivity), external (4 repos, label uncertainty), reliability (determinism proof + triage.json packet).

## Venues
1. MSR/ESEM empirical first. 2. ASE/ICSE-SEIP tool+benchmark with ablation (fix-rate with/without JSON+arch-map). 3. TOSEM/JSS longitudinal capstone. Zenodo replication from day one.

## Non-goals
No pure tool-only paper. No breaking schema change. No GPL deps. No claim of ground-truth bottlenecks.