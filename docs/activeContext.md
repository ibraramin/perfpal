# Active Context

## Current Task
Session 23: **roadmap re-plan (planning-only — no product code).** Founder reshaped the post-MVP direction:

1. **`rust-backend`** (next implementation milestone) — a Rust language backend (official `tree-sitter-rust =0.24.2`, MIT, ABI 15) with the **full catalog mirrored** as `PERF-RS-*`, so Perfpal can **analyze its own codebase** (dogfooding). Schema minor bump 1.1 → 1.2; crate version 0.1.0 → 0.2.0.
2. **`beta-release`** — official proprietary beta **`0.2.0-beta.1`** containing **C + Rust**: hardening (strip/LTO/`panic=abort`/`--remap-path-prefix`), minisign + GitHub attestations, SBOM + NOTICES, EULA + PolyForm Strict; **deep obfuscation and license-key/activation deliberately out** at beta.
3. **Website** — separate repo `~/Perfpal-Website`, separate session family; contract = `docs/website/scope-and-content.md`.

**C++/Python delayed.**

## Recent Decisions
- Ordering: Rust backend **before** the beta (dogfooding = quality signal; beta then ships C+Rust).
- Beta research verdict: skip deep obfuscation + activation; ship hardening/signing/EULA/SBOM.
- Rust `#[cfg]` is a normal sibling node (our C guarded-content bug does **not** recur); `cfg_if!`/`macro_rules!` bodies are opaque `token_tree`s → documented blind spot. Rust is syntactic-tier only.

## In Progress
- Nothing — planning boundary reached; commit pending founder approval.

## Next
- **Next session (`/new`) = plan `rust-backend` R0** (pin grammar, link test, backend doc). Read state files + `readPlan rust-backend` FIRST.
- Then `beta-release`. Website runs independently in `~/Perfpal-Website`.

## Blockers
- None.
