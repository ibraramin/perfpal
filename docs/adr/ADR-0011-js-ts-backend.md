# ADR-0011: JS/TS backends — tree-sitter grammars, grammar-boundary language tokens, one minor schema bump

- **Status:** Accepted (founder-approved 2026-10-24, Track B Session B1)
- **Date:** 2026-10-24
- **Deciders:** Founder + orchestrator (evidence: @librarian parser/licence audit, @explorer Phase 0 inventory)
- **Gates:** All Track B work (B2 JS, B3 TS/TSX, B4 docs/goldens). Extends ADR-0001 (parser strategy) and ADR-0003 (language order).

## Context

Track B adds JavaScript, then TypeScript/TSX, as backends in the existing Rust core. The repo invariants
that bound the choice:

- **Exact pins** for the parser runtime and every grammar (ADR-0001 G5); upgrades are deliberate events
  with a node-types diff review.
- **Rust floor 1.92.0**, edition 2024, pinned by `rust-toolchain.toml`.
- **No copyleft** anywhere in the dependency closure, enforced by `cargo deny check licenses` against
  `deny.toml` (allowlist already includes `MIT`).
- **Byte-determinism**: no `preserve_order`, no set/HashMap iteration in metric output.
- The IR is parser-agnostic (ADR-0001): heuristics never import node kinds, so a second grammar family is
  an additive backend, not a rewrite.
- `tree-sitter = "=0.26.13"` accepts language ABI **13..=15**.

The fork: **(a)** tree-sitter JS + TS grammars, **(b)** `oxc_parser`, **(c)** `swc_ecma_parser`,
**(d)** an out-of-process Node/tsc parser.

## Decision

**Adopt the official tree-sitter grammars, exact-pinned:**

| Crate | Pin | ABI | Licence | Serves |
|---|---|---|---|---|
| `tree-sitter` | `=0.26.13` (unchanged) | 13–15 accepted | MIT | runtime |
| `tree-sitter-javascript` | `=0.25.0` | 15 | MIT | `.js`, `.jsx`, `.mjs`, `.cjs` |
| `tree-sitter-typescript` | `=0.23.2` | 14 | MIT | `LANGUAGE_TYPESCRIPT` for `.ts`/`.mts`/`.cts`; `LANGUAGE_TSX` for `.tsx` |

Both grammar crates are official `tree-sitter` org repositories, compile pre-generated C parsers with
**C11 only** (verified in each `bindings/rust/build.rs` — no C++ standard library is required), and add
`tree-sitter-language` plus build-only `cc` as transitives. No runtime dependency beyond the two grammars
is added.

**Language tokens:** `js`, `ts`, `tsx` — following the **grammar boundary**, not cosmetics.
`.jsx` parses with the JS grammar (JSX is embedded in it) and therefore emits `js`; `.ts` and `.tsx` use
two distinct grammars and therefore emit two tokens. Declaration files (`.d.ts`, `.d.mts`, `.d.cts`) are
**skipped**: they contain no runtime code, so every rule hit on them would be a false positive.

**Schema:** one **MINOR** bump `v1.7 → v1.8`, landing in B2, declaring **all three** tokens up front
(`["c", "js", "python", "rust", "ts", "tsx"]` in `scan.languages` and `files[].language`). B3 then emits
`ts`/`tsx` without a second artifact re-emission or a second full golden regeneration; the alternative
(v1.8 `js`, v1.9 `ts`/`tsx`) was rejected as pure churn: two artifacts, two golden passes, no consumer
benefit. Declaring a token before a backend emits it is additive and harmless (the enum is closed only for
validation; no producer emits unsupported values). The **empty-scan fallback is staged separately**:
`SUPPORTED_LANGUAGES` is `["c", "js", "python", "rust"]` in B2 and adds `"ts"`/`"tsx"` in B3, so it never
advertises a backend that does not exist. The re-emitted artifact also rewrites `$id`, `title`,
`description`, `schema_version.const` and both enum descriptions (they name the old set today).

**Analyzer family:** JS and TS/TSX share the `PERF-JS-*` catalog because the runtime mechanism is
identical (V8 / Node). `RuleLanguage::Js` matches `js|ts|tsx`; `RuleLanguage::Ts` is reserved for
constructs that exist only in TypeScript, and v1 ships none unless the B2 canary proves one. This avoids
duplicating ~12 implementations and two diverging catalogs.

**Tier:** `syntactic` throughout. No `semantic` claim, no libclang involvement, no `semantic-clang` change.

**Discovery:** directory skips are **global** because discovery is language-blind at directory level — skip
`node_modules`, `bower_components`, `dist`, `build`, `out`, `coverage` (dot-directories such as `.next`,
`.nuxt`, `.svelte-kit`, `.cache` are already skipped today). File-level skips are **JS/TS-only**:
`*.min.js`, `*.min.mjs`, `*.min.cjs`, `*.bundle.js`, and files larger than **2 MiB** (minified bundles).
Without this, vendored JS dominates every finding and multi-megabyte minified files stall the scan. The
existing C/Rust/Python corpora contain none of the skipped directories, so their finding sets and goldens do
not move; the behavior change is documented in the output contract.

## License audit (no-copyleft constraint)

| Component | SPDX | Copyleft? | Notes |
|---|---|---|---|
| `tree-sitter-javascript` + generated `parser.c` | `MIT` | No | Official org; vendoring permitted with notice; already in `deny.toml` allowlist. |
| `tree-sitter-typescript` (both parsers) + generated `parser.c` | `MIT` | No | Same posture; one crate compiles four C files (`typescript/{parser,scanner}.c`, `tsx/{parser,scanner}.c`). |
| `oxc_parser` (rejected) | `MIT` | No | Licence-clean but MSRV 1.97 > repo floor; see alternatives. |
| `swc_ecma_parser` (rejected) | `Apache-2.0` | No | Licence-clean; MSRV unverified; major-version churn. |

ADR-0001's landmine still applies: **community** grammars can be GPL/AGPL/MPL. Only official
`tree-sitter` org grammars are eligible; both chosen crates are MIT.

## Rationale

1. **Invariant fit.** The grammars load inside the pinned runtime's ABI window (15 and 14 ∈ 13..=15), add
   only MIT code, and keep the single-binary, no-runtime-dependency model.
2. **Architecture fit.** A tree-sitter backend emits the same `Extraction` records as C/Rust/Python, so the
   analyzer, aggregator and reporter stay untouched apart from language tokens. `oxc`/`swc` expose
   Rust-internal arena ASTs, requiring a second translation layer and a different traversal model.
3. **Cost.** Two C parsers compile at build time (already true for three grammars today). `oxc`/`swc`
   would add dozens of crates to the lockfile and a fast-moving upgrade surface.
4. **Recovery.** tree-sitter always returns a best-effort tree with localized `ERROR` nodes, which matches
   Perfpal's non-compiling-input-first philosophy (ADR-0001 rationale 1).
5. **Evidence discipline.** The rule catalog is frozen by a canary, not by intuition: JS performance
   folklore is dense (`arr.length` hoisting, `new RegExp` caching), and the M1 lesson showed five rules can
   produce 89% of self-scan noise.

## Alternatives considered

- **`oxc_parser` — rejected.** Latest (0.154.x) declares **MSRV 1.97.0**, above the founder-locked 1.92
  floor; the last 1.92-compatible line is ~0.126. Weekly releases and an arena-tied AST conflict with
  ADR-0001 G5 deliberate upgrades and with the IR seam. Revisit only if oxc's MSRV drops to ≤1.92 **and** a
  stable serializable AST appears.
- **`swc_ecma_parser` — rejected.** MSRV not published (unverified against 1.92); major-version churn
  (v24 → v46 in ~2 years); per-parse `SourceMap`/`Handler` scaffolding and a `tsrx` feature surface.
- **Out-of-process Node/tsc — rejected.** Breaks the single-binary, zero-runtime-dependency model and
  introduces a second toolchain to the funnel.
- **Community JS grammars — rejected.** ADR-0001 landmine: unvetted licences.

## Consequences

**Positive**
- JS/JSX/TS/TSX become additive backends with no analyzer/reporter rewrite.
- ABI compatibility is provable in-tree via grammar-link tests.
- One schema bump covers the whole track; the golden regeneration happens once.

**Negative / mitigated**
- **MSRV not declared** by either grammar crate. Mitigation: B2-W0 compiles both on 1.92 before any
  implementation; if a crate fails, fall back to the previous tag (`tree-sitter-javascript` 0.23.1) and
  record the changed pin in the track handoff.
- **ABI skew** (JS 15, TS 14) is inside the window but the TS grammar is one generation behind. Mitigation:
  exact pin + node-types diff review on every upgrade (ADR-0001 G5).
- **Build time / binary size** grows by two C parsers. Accepted; no feature gate (a gated backend would
  fragment the language matrix and the goldens).
- **Minified/vendored JS** is skipped, not analysed. Published as an accepted limitation.
- **No type information.** Type-aware rules are out of scope; `syntactic` tier is stated honestly in every
  finding.

## Guardrails

1. Exact pins for both grammar crates; `Cargo.lock` tracked; upgrades are deliberate with a node-types diff.
2. `tests/javascript_grammar_link.rs` and `tests/typescript_grammar_link.rs` assert ABI-in-window,
   `set_language` success, zero `ERROR`/missing nodes on representative sources, and non-vacuous degradation
   on malformed input.
3. Backends never leak node kinds past the scanner (`Extraction` only); heuristics import only the IR.
4. The `PERF-JS-*` catalog is frozen only after the W0 canary; every entry records mechanism, fix, and
   accepted FP class.
5. Double-run byte-determinism on the JS and TS corpora.
6. JS/TS discovery skips are path/size-based and deterministic; no content sniffing.
7. No merge to `main` before A step 12 and explicit founder approval; beta is Track A and is not delayed.
8. **Reporter catalog ownership:** `HEURISTIC_CATALOG_V1` lives in `src/reporter/mod.rs` (A-owned until A step
   12) and panics on uncatalogued rule ids. The catalog entries and B2's registry block must land in the
   **same change** — a catalog-only landing ahead of B2 fails the `catalog == registry` / count assertions
   immediately. Track A supplies the entries for that change, or the founder approves a catalog-only B edit
   inside B2-W2. Blocker `B-tb-003`.

## Reversal triggers

1. The TS grammar stalls (no ABI-15 regeneration) while `tree-sitter` moves past ABI 15.
2. Measured tree-sitter recovery failure leaves JS/TS function boundaries unrecoverable on real corpora.
3. The catalog requires type-aware facts to reach the precision ratchet.
4. `oxc` lowers its MSRV to ≤1.92 and ships a stable serializable AST.

## References

- tree-sitter ABI versions: https://tree-sitter.github.io/tree-sitter/using-parsers/7-abi-versions.html
- `tree-sitter-javascript` (MIT, ABI 15): https://github.com/tree-sitter/tree-sitter-javascript
- `tree-sitter-typescript` (MIT, ABI 14, two parsers): https://github.com/tree-sitter/tree-sitter-typescript
- `oxc_parser` (MIT, MSRV 1.97 at latest): https://crates.io/crates/oxc_parser
- `swc_ecma_parser` (Apache-2.0): https://crates.io/crates/swc_ecma_parser
- ADR-0001 parser strategy · ADR-0003 language order · ADR-0007 Python tier strategy
