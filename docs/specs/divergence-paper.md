# Spec: divergence-paper

Scope: feature

# Divergence Paper — Feature Spec

Instantiation of divergence-study (repo) for MSR/ESEM first submission.

## RQs
- RQ1: Along which static dims do AI-era snapshots diverge (loop-nesting rate, alloc-in-loop, idiom ratios, call-shape, coupling-shape)?
- RQ2: Which divergences survive churn/size/author controls?
- RQ3: Do divergences predict triaged candidacy (GENUINE calibration)?

## Corpus (paired + matched)
- Longitudinal: same repo pre-2022 SHA vs post-2024 SHA.
- Cross-sectional: AI-native vs human-mature matched on language x size x domain.
- N: 8-12 repos x2 min, C/Rust/Python, >=100k LOC per arm where possible.
- Anchors: Aider a4be6cc, chunkhound a8553ae, ripgrep 4649aa9, curl 75a2079d.
- Packet per cell: URL+SHA+license+scan cmd+expected counts+double-scan proof. Exclude .ipynb, content-gated prod filter.

## Stats
- Unit repo-snapshot, hierarchical/cluster-robust. Pre-reg 4-5 primary dims.
- Mann-Whitney + Cliff delta + bootstrap CIs, Holm/BH, power calc (use info-tier).
- Blinded dual-triage + kappa. Sensitivity: with/without error-path strata.

## Replication
- Zenodo: corpus list, frozen Perfpal binary, metrics.json set, triage.json, scripts, claim-to-evidence README.

## Tool ablation (usefulness)
- Agent fix-rate with vs without JSON+arch-map on AI-heavy sample. Same IR v1.4 build serves both tracks.