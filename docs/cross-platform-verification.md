# Cross-platform verification (Windows + Linux) — runbook

> **Cadence: periodic, not per session.** Run this after any *major* change that could affect
> portability (see **When to run**). Routine feature work that doesn't touch OS-facing code does
> **not** need it. The cheap half is always-on in CI (`.github/workflows/ci.yml` job
> `check-windows`); this runbook is the heavier **runtime** half, run at milestones / before a tag.

## Why this exists

Perfpal is portable Rust, but the Windows path was never exercised. A cross-platform audit
(2026-10-12) found that, because **no CI job ran on Windows**, two regressions slipped through:

- **M1** — an ungated `std::os::unix` test made `cargo test` fail to compile on Windows;
- **M2** — the semantic-resolver re-read joined a POSIX repo-relative path onto the Windows
  `\\?\`-prefixed canonical root, so the semantic tier was dead on Windows.

Both are fixed (`cccd91c`) and the Windows leg is now guarded by CI + Wine (below). This runbook
keeps that from silently rotting.

## When to run (triggers)

- **Before a release tag / the publish gate (M5)**, and **at each milestone** (M4, M5, …).
- After changes touching OS-facing seams:
  `src/scanner/mod.rs` (discovery + path handling), `src/resolver/mod.rs`,
  `src/cli/mod.rs` (root canonicalization / provenance), `src/writer/mod.rs` (output-dir swap),
  `src/reporter/mod.rs` + `src/schema/` (path emission).
- After a dependency/grammar bump (`tree-sitter*`, `clap`, `serde*`, `clang-sys`) or a
  `rust-toolchain.toml` bump.
- After any edit to `.github/workflows/{ci,release}.yml`.
- **Quarterly, at minimum.**

## Prerequisites (install once)

```sh
rustup target add x86_64-pc-windows-gnu          # cross target (rustup only, no system pkgs)
sudo pacman -S mingw-w64-gcc wine                # C toolchain for the grammars + Wine (Arch/CachyOS)
```

`wine` is only needed for the runtime half (step 3); steps 1–2 need just the mingw toolchain.

## Steps

### 1. Windows type-check — every target (incl. tests)

```sh
cargo check --locked --all-targets --target x86_64-pc-windows-gnu
```

This is the step that catches the **M1 class** (unix-only APIs in `src/` *or* tests).
**Expected:** finishes clean (no `error[E…]`).

### 2. Windows release build

```sh
cargo build --release --locked --target x86_64-pc-windows-gnu
file target/x86_64-pc-windows-gnu/release/perfpal.exe     # expect: PE32+ … x86-64
```

### 3. Wine runtime check + Linux-equivalence diff

```sh
scripts/verify-windows-compat.sh        # automates steps 1–3 + the equivalence diff
```

Or by hand:

```sh
mkdir -p /var/tmp/pf-wine && cp target/x86_64-pc-windows-gnu/release/perfpal.exe /var/tmp/pf-wine/
cp /usr/x86_64-w64-mingw32/bin/libgcc_s_seh-1.dll /var/tmp/pf-wine/ 2>/dev/null   # if the exe needs it
export WINEPREFIX=/var/tmp/pf-wine/prefix WINEDEBUG=-all WINEDLLOVERRIDES="mscoree,mshtml=" DISPLAY=
cd /var/tmp/pf-wine
wine perfpal.exe --version                                    # expect: perfpal <ver> (metrics schema 1.3)
wine perfpal.exe --output 'Z:\var\tmp\pf\out' 'Z:\no\such'; echo "exit=$?"   # expect 1 (usage error)
wine perfpal.exe --bogus-flag;                               echo "exit=$?"   # expect 1
wine perfpal.exe --output 'Z:\var\tmp\pf\src' 'Z:\path\to\repo\src'          # same counts as Linux
```

**Pass criteria:** `--version` prints the current schema (1.3); usage errors exit `1`; a scan of a
known corpus produces the **same file/finding counts** as the Linux binary, ideally with a
**byte-identical `metrics.json`** once provenance (`root_path`/`started_at`/`duration_ms`) is
stripped. Wine maps Linux `/` to `Z:\`, so scan targets are `Z:\…` Windows paths.

## What this covers / doesn't

| ✅ Covered (Wine ≈ Windows) | ❌ Not covered — needs a real Windows host or extra assets |
|---|---|
| Build + link for Windows (`windows-gnu`) | **MSVC-ABI** build (this is `*-gnu`; the official artifact is `*-msvc`, built only in CI `build-windows`) |
| All-target type-check incl. tests | **`semantic-clang` tier** (needs a Windows `libclang.dll`) — the M2 fix is code-reviewed only |
| CLI parse, exit codes, scan → analyze → report | True-Windows kernel behaviors: AV/indexer **rename locks**, real NTFS **junctions** |
| Path handling: internal paths stay POSIX; `root_path` takes the Windows form | Legacy-console non-ASCII rendering (cosmetic) |
| Deterministic output equivalence vs Linux | |

## Residuals

Tracked as **`R-pf-095`** in `state/session-state.json`. The only functional gap is the
`semantic-clang` tier on Windows: to close it, drop a Windows `libclang.dll` (LLVM / llvm-mingw
release) beside the exe, build with `--features semantic-clang`, and re-run step 3.

## Always-on CI guard

`.github/workflows/ci.yml` job **`check-windows`** runs `cargo check --locked --all-targets` on
`windows-latest` on every push/PR — that is the cheap, continuous half (it catches the M1 class
before a tag). This runbook adds the periodic **runtime** half.

## Run log

| Date | Commit | Result | Notes |
|---|---|---|---|
| 2026-10-12 | `cccd91c` | ✅ PASS | exe built (PE32+); `--all-targets` green after M1; Wine 11.19: `src` 49 files/463 findings, `examples` 13/83, `examples-python` 20/29 — `metrics.json` byte-identical to Linux modulo provenance; usage exits `1`/`1`; output-dir swap OK. Not covered: `semantic-clang` tier. |
