---
plan name: divergence-revamp
plan description: Instrument plus paper track
plan status: active
---

## Idea
Revamp Perfpal IR to v1.4 additive stats (call-shape, idiom/clone rate, error_path flag, alloc histogram, LOOP-002 split, populate distributions + DOT/cost viz) while preserving byte-determinism and 67-rule ratchets, and run it as divergence instrument on paired longitudinal + matched cross-sectional corpus for MSR/ESEM paper with Zenodo replication plus agent-loop ablation for tool usefulness.

## Implementation
- Land IR v1.4 additive fields with serde defaults + determinism proof (double-scan byte-identical)
- Populate stats.distributions + DOT export + cost overlay; charts stay pure-fn-of-JSON
- Precision-measure top FP-class fixes (do-while0, cfg-gated, empty_ctor, realloc guard) without lowering ratchets
- Pin corpus: pre-2022 vs post-2024 SHAs + AI-native vs human-mature mates with URL+SHA+license+counts packet → pinning packet landed: `docs/evidence/corpus-packet.md` (10 pinned + 4 proposed cells, 8 repos, all scan/proof slots PENDING) + machine-readable `docs/evidence/corpus.json` (counts.json key convention + role/status) + `scripts/pin-corpus.sh --mode verify|scan` (clone→rev-parse→date-gate→double-scan loop; scans NOT yet run)
- Pre-register 4-5 dims + blinded dual-triage kappa + cluster-robust stats with effect sizes and FDR
- Publish Zenodo replication + MSR/ESEM draft; run agent fix-rate ablation with/without JSON+arch-map

## Required Specs
<!-- SPECS_START -->
- divergence-study
- evidence-beta-spec
- perfpal-core-spec
- divergence-paper
<!-- SPECS_END -->