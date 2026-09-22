# Active Context

## Current Task
Session 27: plan `rust-backend` **R3** — full `PERF-RS-*` heuristic catalog (23 rules, language-tagged registry), Rust `use`-graph resolution + Cargo.toml-aware crate-root detection, Rust allocation family + `macro_origin` recognizer, and enum/union/tuple-struct IR decisions. **Complete**; dual review passed (final round 3/3 **APPROVE** after the N1 fix; `@requirements-reviewer` **APPROVED**); gates **361 default / 366 semantic** green; commit pending founder approval.

## Recent Decisions
- **Founder (R3 gate):** all-of-R3 staged waves · all 23 families with honest reframing · language-tagged registry (C-only/Rust-only) · macro-as-call surface · additive `StructDef.kind` + union/enum + pointer/array dims AND `use`-edge resolution + Cargo.toml crate-root detection.
- Additive IR (all IR-only — **no metrics.json schema bump**, stays v1.2): `StructKind`/`StructDef.kind`, `CallEvent.callee_path`/`is_macro`, `IncludeEdge.is_mod`, shared `RUST_ALLOC_FAMILY`.
- `analyze()` now groups units once by `file.language` and passes each registry entry only its language slice; `cross_file_call_edges` is language-scoped.
- **N1 fix:** `PERF-RS-ARCH-001` operates on the `use` **import graph only** — `mod` declaration edges (`is_mod`) are excluded (they were producing parent↔child false cycles); `mod` edges still drive resolution and remain in `include_graph` data.
- Rust is **syntactic-tier only**; `token_tree` macro bodies remain a documented blind spot.

## In Progress
- Nothing — R3 boundary reached; commit pending.

## Next
- **Next session (`/new`) = plan `rust-backend` R4** — dogfood (run the built binary over Perfpal's own `src/`, publish the report, triage findings vs accepted FP classes) + `examples-rust/` corpus (mirror `examples/` density + silent controls; verify every `PERF-RS-*` rule fires against real output). Read state files + `readPlan rust-backend` FIRST.
- Then R5 (tests/gates + crate version `0.1.0 → 0.2.0`) → R6 (README `PERF-RS-*` table, modules.md §4.2/§5.2, CHANGELOG, EOL) → plan `beta-release` (`0.2.0-beta.1`).

## Blockers
- None.
