# Pre-registration — library-shaped A2 holdout (A-pf-116)

**Date:** 2026-10-23. **Status:** pre-registered, vaulted. **Owner:** orchestrator.
**Purpose:** the second A2 class required by ADR-0009 and `docs/specs/atlas-feature.md` §4:
a library-shaped **crate payload** where the entry-flow canvas may have no production `main` and no
forward path, so the below-graph brief must carry the action. The A2 scan unit is the **published
`memchr` crate payload at the pin** (`src/` + `Cargo.toml` + license/readme), not the repository's
dev-only benchmark/fuzz tooling. The payload is not a design repo and must not be scanned, rendered
or read by any agent before the founder A2 test.

## Registration

| Field | Value |
|---|---|
| Repository | `BurntSushi/memchr` — <https://github.com/BurntSushi/memchr> |
| Pinned commit | `bd6068c30e9074a90c285e47912fa0b047d07597` (2026-08-10) |
| **A2 scan unit** | **Published `memchr` crate payload at the pin:** `src/`, `Cargo.toml`, `README.md`, `UNLICENSE`, `LICENSE-MIT` (plus `COPYING`; the crate license is `Unlicense OR MIT`). `benchmarks/` and `fuzz/` are **excluded** (dev-only). |
| Crate | `memchr` 2.8.3, `edition = "2021"` |
| License | `Unlicense OR MIT` (permissive; scan-only, not a dependency) |
| Shape (crate payload) | Rust **library only** — `[lib]` present, no `[[bin]]`, no `src/bin/`, no `build.rs`, no `[workspace]` |
| Dev subtrees (excluded) | The repository also ships `fuzz/` and `benchmarks/`: **16 `[[bin]]` targets** across 9 Cargo.tomls, **10 `[workspace]` tables**, and `benchmarks/engines/rust-std/build.rs`. None is part of the A2 scan unit. |
| Size at pin | 184 tracked files. **Crate payload:** 45 `.rs` under `src/` + `Cargo.toml`. Repo total: 64 `.rs` (45 `src/` + 8 `fuzz/` + 11 `benchmarks/`). |
| Design use | **None.** Before this file was written (2026-10-23), `rg -ni "memchr" docs/ state/handoff state/session-state.json` returned 0 matches. Later ratification docs necessarily name the holdout. |
| Vault | No Perfpal scan, no chart render, no agent read of scan output/PNG until the founder A2 test |

## Why this crate payload satisfies the library class

- The **crate payload** is a real library with **no application entry point**, so the Atlas entry
  policy must fall through production `main` and production `script` to the highest-fan-in
  production `lib-root` (expected label: `library root (no production main)`). If the election
  instead finds `NO ENTRY`, that is a legitimate observed state and is recorded as such.
- The excluded dev subtrees are not production code: the benchmark mains are `bench`-class and the
  fuzz targets are fuzz-only. Excluding them removes the 16 dev bins without removing library source.
- It has enough production code (vector/SIMD search paths, loops, allocations) that a complete KV1
  should be able to state 3 actions even if the canvas has no forward path — the exact library
  falsifier in `docs/research/atlas-developer-needs.md` §9.2.
- It is not any design repo (cJSON, hyperfine, tmux, curl, ripgrep, django, postgres, requests,
  flask, serde, lua are all excluded) and it was never used in V0–V7.

## Structural preflight (verification only — not an Atlas scan)

Run against a throwaway clone at the pinned commit; no Perfpal binary was run.

| Check | Command | Result |
|---|---|---|
| Pin resolves | `git rev-parse HEAD` | `bd6068c30e9074a90c285e47912fa0b047d07597` |
| Crate payload root shape | `! test -d src/bin && ! rg -q '^\[\[bin\]\]' Cargo.toml && ! test -f build.rs && ! rg -q '^\[workspace\]' Cargo.toml` | pass (no bin / no build.rs / no workspace) |
| Library target | `rg -q '^\[lib\]' Cargo.toml` | pass (`name = "memchr"`) |
| Crate payload source count | `git ls-files 'src/*.rs' \| wc -l` | 45 |
| Dev bin paths (excluded) | `rg -l '^\[\[bin\]\]' --glob '**/Cargo.toml' .` | 9 files: `fuzz/Cargo.toml` + 8 `benchmarks/engines/*/Cargo.toml` (16 bins) — all under `fuzz/` or `benchmarks/` |
| Dev workspace paths (excluded) | `rg -l '^\[workspace\]' --glob '**/Cargo.toml' .` | the same 9 + `benchmarks/shared/Cargo.toml` (10 tables) — all under `fuzz/` or `benchmarks/` |
| Dev build script (excluded) | `find . -name build.rs` | `benchmarks/engines/rust-std/build.rs` |
| License files at pin | `git ls-tree --name-only HEAD` | root includes `COPYING`, `LICENSE-MIT`, `UNLICENSE`; no `LICENSE-APACHE` |
| No design use | `rg -ni "memchr" docs/ state/handoff state/session-state.json` | 0 matches before this file was written |
| License | `rg -n '^license' Cargo.toml` | `Unlicense OR MIT` |

## A2 protocol (to run only at the A2 gate)

1. Fresh clone at the pinned commit into `<scratch>/memchr`; create a **separate clean payload
   directory** and extract only the crate payload:
   ```bash
   mkdir -p <scratch>/memchr-payload
   git -C <scratch>/memchr archive <pin> src Cargo.toml README.md UNLICENSE LICENSE-MIT COPYING \
     | tar -x -C <scratch>/memchr-payload
   ```
   Do **not** copy or scan `benchmarks/` or `fuzz/`; the scan target is
   `<scratch>/memchr-payload` (not the clone).
2. Build the shipped Perfpal binary; scan the extracted crate-payload directory with animation
   enabled; capture the final frames of the Atlas page at 1440×900.
3. Founder timed test: name **3 actions in under 10 minutes**; "understandable" yes/no; verify the
   CSS-only pause, `prefers-reduced-motion` final frame and print final frame.
4. Record the verbatim transcript, the 3 actions, timing and verdict in
   `docs/evidence/map-vision/v8/a2.md`; commit the evidence.
5. The django/postgres holdout remains vaulted and is tested in the same A2 session on the
   application class.

## Invalidation / what would change this pre-registration

- The pinned commit cannot be reproduced from the public repo (registration void; pick a new pin
  before any read).
- The A2 scan target is the clone or otherwise includes `benchmarks/`/`fuzz/` (scope violation:
  scan only `<scratch>/memchr-payload` and rerun before recording a verdict).
- The crate payload is discovered to have been used in design or scanned early (contaminated;
  replace it and record the incident).
- A2 on the application class already fails (the feature is superseded per ADR-0009 §Gates; the
  library run cannot rescue it).
- The founder explicitly wants a **single-file C lib** instead; a replacement must be registered
  here with the same preflight and scope statement before the A2 session.
