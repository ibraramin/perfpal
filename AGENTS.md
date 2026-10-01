# Perfpal — project rules (auto-loaded)

Project-specific invariants + verify commands. The global constitution lives in
`~/.config/opencode/AGENTS.md` (session lifecycle, delegation, EOL contract, token/request discipline) and is
**not** repeated here — this file holds deltas only.

**Product:** a static-analysis CLI (Rust) that scans C and Rust codebases and produces three complementary
outputs — language-specific performance heuristics, architecture-level bottleneck visualizations, and
LLM-consumable quantified metrics (structured JSON + a human-readable Markdown report).

## Invariants (founder-locked)
- **License:** source-available **proprietary** (`LicenseRef-Proprietary`; see `LICENSE`, `EULA.md`,
  `PRIVACY.md`). Free to use; modification and redistribution require explicit founder permission.
- **No copyleft:** no GPL-family dependency anywhere in the closure (direct or transitive) — permissive or
  proprietary only; enforced by `cargo deny check licenses` against the `deny.toml` allowlist.
- **Exact pins** for the parser runtime + grammars (`tree-sitter`, `tree-sitter-c`, `tree-sitter-rust`) —
  ADR-0001 G5; `Cargo.lock` is intentionally tracked (reproducible builds).
- **Byte-determinism guard:** serde_json `preserve_order` MUST stay OFF (checked in CI); no set-iteration in
  metric output — sort every traversal so runs are byte-identical.
- **Exit-code contract:** a caught panic maps to **exit code 3** via `std::panic::catch_unwind`; `panic =
  "abort"` is deliberately NOT set (it would break the contract in release builds).
- **Rust floor 1.92.0** (edition 2024), pinned via `rust-toolchain.toml`; CI and release use it exactly.
- **`semantic-clang` feature default OFF** everywhere (static musl cannot `dlopen`); graceful absence of
  libclang is a first-class path, and real-parse tests skip — never fail — without it.
- **Static analysis is approximate, not ground truth:** heuristics flag "optimization candidates", not
  guaranteed bottlenecks; framing must stay careful (an LLM/human validates with runtime profiling).

## Verify (runnable commands)
- Format: `cargo fmt --all -- --check`
- Lint: `cargo clippy --locked --all-targets -- -D warnings`
- Tests: `cargo test --locked`
- Licenses: `cargo deny check licenses`
- Byte-determinism: no `preserve_order` present in `Cargo.lock`
- Feature gate: `cargo clippy --locked --all-targets --features semantic-clang -- -D warnings` and
  `PERFPAL_SEMANTIC_TESTS=1 cargo test --locked --features semantic-clang`
- Full gate = CI `.github/workflows/ci.yml` (jobs: `check`, `semantic-features`, `license-audit`).

## Doc pointers
- Architecture: `docs/architecture/` (`modules.md`, `ir-v1.md`, `rust-backend.md`)
- Specs: `docs/specs/` (`perfpal-core-spec.md`, `evidence-beta-spec.md`) · Plans: `docs/plans/` · ADRs: `docs/adr/`
- Heuristics: `docs/heuristics/` · Schema: `docs/schema/` · Lessons: `docs/lessons-learned.md`
- **North-star (pinned, founder-edited):** `docs/NORTH-STAR.md` — goal / non-goals / stage / metrics.
- **State (hot/cold):** hot `state/session-state.json` (compact JSON, ≤ 8 KB; schema `~/.config/opencode/docs/STATE-SCHEMA.md`;
  view via `jq .`) · cold `state/archive/ledger.jsonl` (greppable, never auto-read). Session start reads hot +
  `state/handoff/latest.md` + `docs/activeContext.md` only.
