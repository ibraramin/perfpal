# Divergence-corpus pinning packet (plan `divergence-revamp` bullet 4)

> Machine-readable cell list: `docs/evidence/corpus.json`. Pin/scan loop:
> `scripts/pin-corpus.sh`. Specs: `divergence-study:20-25`, `divergence-paper:14-19`.
> Conventions follow the M4 packet pattern (`showcase-aider.md` §1/§2/§4; umbrella
> `ai-code-showcase.md`; schema exemplar `showcase/aider/counts.json`).
> Checkouts live at `/var/tmp/opencode/div-corpus/<cell>/` — **outside** the product
> repo, never committed. Full 40-char SHAs everywhere (per `findings-deep-dive.md:14-21`).

## 1. Design (paired + matched)

- **Arm A — longitudinal** (same repo, pre-2022 vs post-2024/AI-era): `ripgrep`, `curl`,
  `requests`. Pre = commit date `< 2022-01-01` (pre-Copilot/ChatGPT); post = AI-era
  `>= 2024-01-01` with exact dates recorded, so a strict 2025+ cut can be applied
  as a sensitivity analysis. `aider`/`chunkhound` have **no pre arm** (repos postdate 2022).
- **Arm B — cross-sectional** (AI-native vs human-mature, matched on language × size ×
  domain): `aider-ai` vs `requests-post` (Python), `chunkhound-ai` vs `ripgrep-m4`
  (Rust side; chunkhound findings stratified per-language at analysis), `curl-m4`
  (C human; AI-native C mate PROPOSED). M4 anchors reused; re-scanned under IR v1.4.
- **N:** 8 repos (5 pinned + 3 proposed) × 1–2 snapshots = **10 pinned + 4 proposed**
  cells. Divergence-paper target is 8–12 repos ×2; the 4 PROPOSED cells close the gap
  after founder ratification.
- **Filters:** permissive licenses only (record license file + SPDX per cell); `.ipynb`
  excluded; production filter = `findings-deep-dive.md` §1.2 path rules.
- **Stratification:** per-language analysis only. chunkhound (`python+rust`) is never
  pooled — Python findings compare with Python cells, Rust with Rust cells (§7).

## 2. Longitudinal cells (pinned)

| Cell | Tag → commit | Date | License (file) | Scan command |
|---|---|---|---|---|
| `ripgrep-pre` | `12.1.1` → `7cb211378a2ac6d421c5f6f3f71411937af23136` | 2020-05-29 | MIT/Unlicense (`LICENSE-MIT`, `COPYING`) | `perfpal /var/tmp/opencode/div-corpus/ripgrep-pre -o out-div-ripgrep-pre` |
| `ripgrep-post` | `15.0.0` → `3a612f88b805e14aef45bfa43e25a54abc6297fc` | 2025-10-15 | MIT/Unlicense (`LICENSE-MIT`, `COPYING`) | `perfpal /var/tmp/opencode/div-corpus/ripgrep-post -o out-div-ripgrep-post` |
| `curl-pre` | `curl-7_80_0` → `9e560d11aad028de74addc0d1edfefa5667884f4` | 2021-11-10 | curl MIT-like (`COPYING`) | `perfpal /var/tmp/opencode/div-corpus/curl-pre -o out-div-curl-pre` |
| `curl-post` | `curl-8_12_0` → `34cf9d54a46598c44938aa7598820484d7af7133` | 2025-02-05 | curl MIT-like (`COPYING`) | `perfpal /var/tmp/opencode/div-corpus/curl-post -o out-div-curl-post` |
| `requests-pre` | `v2.26.0` → `a1a6a549a0143d9b32717dbe3d75cd543ae5a4f6` | 2021-07-13 | Apache-2.0 (`LICENSE`) | `perfpal /var/tmp/opencode/div-corpus/requests-pre -o out-div-requests-pre` |
| `requests-post` | `v2.32.4` → `021dc729f0b71a3030cefdbec7fb57a0e80a6cfd` | 2025-06-09 | Apache-2.0 (`LICENSE`) | `perfpal /var/tmp/opencode/div-corpus/requests-post -o out-div-requests-post` |

Expected counts per cell: **PENDING** (fill from `pin-corpus.sh --mode scan` run;
M4-era schema-1.3 counts are orientation only — re-scan under IR v1.4 required).
Double-scan proof per cell: **PENDING** (masked-`metrics.json` sha256 pair; §6).

## 3. Cross-sectional cells (pinned)

| Cell | Role | Tag → commit | Date | License (file) | Matched against |
|---|---|---|---|---|---|
| `aider-ai` | ai-native (Python) | `v0.86.0` → `a4be6ccd87ebaa59b361f3f028d116ce1761b626` | 2025-08-09 | Apache-2.0 (`LICENSE.txt`) | `requests-post` (Python, http-library vs coding-agent — domain approximate) |
| `chunkhound-ai` | ai-native (Python+Rust) | `v6.0.0` → `a8553ae644686d8ee55b94ee74ff560a38e6cc7d` | 2026-09-14 | MIT (`LICENSE`) | `ripgrep-m4` Rust side (code-indexer vs search-tool — domain approximate) |
| `ripgrep-m4` | human-mature (Rust) | `14.1.1` → `4649aa9700619f94cf9c66876e9549d83420e16c` | 2024-09-08 | MIT/Unlicense (`LICENSE-MIT`, `COPYING`) | M4 anchor; schema-1.3 counts in `showcase-ripgrep.md` |
| `curl-m4` | human-mature (C) | `curl-8_11_1` → `75a2079d5c28debb2eaa848ca9430f1fe0d7844c` | 2024-12-11 | curl MIT-like (`COPYING`) | M4 anchor; schema-1.3 counts in `showcase-curl.md` |

`requests-post` doubles as the human-mature Python mate (Arm B) and the longitudinal
post snapshot (Arm A). `aider-ai` M4 orientation counts: `showcase-aider.md`
(148 files / 1567 findings under schema 1.3 — informational, NOT the expected value).
Expected counts: **PENDING**. Double-scan proof: **PENDING**.

## 4. Proposed cells (founder ratifies; NO SHA claimed)

| Cell | Role | URL | License | Purpose |
|---|---|---|---|---|
| `flask-pre` / `flask-post` | longitudinal pair (Python) | `https://github.com/pallets/flask` | BSD-3-Clause (TBD — verify) | second Python longitudinal repo (web-framework) |
| `fd-human` | human-mature (Rust) | `https://github.com/sharkdp/fd` | MIT/Apache-2.0 (TBD — verify) | domain-matched Rust mate (search-tool, pairs ripgrep arm) |
| `swe-agent-ai` | ai-native (Python) | `https://github.com/SWE-bench/SWE-agent` | TBD — verify permissive-only | second AI-native anchor (coding-agent, pairs aider arm; UNVETTED) |

Pinning a proposal = founder picks tags → `pin-corpus.sh --mode verify --cell <id>`
after adding tag+SHA to `corpus.json` (status → `pinned`) → scan step fills counts/proof.

## 5. Scan protocol (per cell; later step — NOT run here)

```bash
# 1. Pin + date-gate every pinned cell (idempotent; skips existing checkouts)
scripts/pin-corpus.sh --mode verify
# 2. Scan twice (needs PERFPAL=perfpal on PATH or ./target/release/perfpal;
#    invocation follows the showcase §2 pattern: perfpal <repo> -o <out>)
PERFPAL=./target/release/perfpal scripts/pin-corpus.sh --mode scan
# equivalent single-cell form (mirrors showcase-aider.md:18-22):
# cargo run --locked -- /var/tmp/opencode/div-corpus/<cell> -o out-div-<cell>-r1
```

Record per cell: the `scanned N files (… ok, … partial, … failed) · M findings
(h hot, w warn, i info)` summary line, the per-rule table (counts.json convention),
and the two masked sha256 lines from §6. Exit code must be 0 (findings are data).

## 6. Determinism proof protocol (per cell; slots PENDING)

Two scans into separate output dirs; `metrics.json` compared byte-identical **after
masking the documented provenance fields** (`scan.started_at`, `scan.duration_ms`) —
the same contract as showcase-aider.md §4. Record:

- Run-1 masked `metrics.json` sha256: **PENDING**
- Run-2 masked `metrics.json` sha256: **PENDING**
- Result: **PENDING** (must read IDENTICAL before the cell counts as pinned-scanned)

## 7. Analysis rules (binding on the stats step)

- Stratify per-language (Python retires 10/23 premises per divergence-study:25 —
  no cross-language pooling of rates or distributions).
- Unit = repo-snapshot; cluster-robust/hierarchical stats per divergence-study:27-31.
- Rates and distributions, never raw counts; never claim AI is worse (spec claim §9-10).

### 7.1 Arm-membership matrix (which cells serve which RQs)

| Cell | Arm A longitudinal | Arm B cross-sectional | Language stratum |
|---|---|---|---|
| `ripgrep-pre` / `ripgrep-post` | Rust pair | (post: domain ref for `fd-human`) | rust |
| `curl-pre` / `curl-post` | C pair | `curl-m4` + PROPOSED AI-native C mate | c |
| `requests-pre` / `requests-post` | Python pair | `requests-post` = human-mature mate vs `aider-ai` | python |
| `aider-ai` | — (no pre-2022 history) | ai-native vs `requests-post` | python |
| `chunkhound-ai` | — (no pre-2022 history) | ai-native vs `ripgrep-m4` (Rust stratum) | python AND rust (split) |
| `ripgrep-m4` / `curl-m4` | — (single-snapshot anchors) | human-mature anchors | rust / c |

Size note (divergence-paper:17 asks ≥100k LOC/arm where possible): per-cell `loc_code`
is recorded at the scan step (counts.json convention) — no LOC claimed here. curl,
ripgrep-post, and aider are known ≥100k from M4-era orientation; requests-pre/post
and the PROPOSED cells confirm size at scan time; shortfalls are disclosed, not patched.

## 8. SHA verification log (all dates via `git log -1 --format=%ci`, never tag-name guesses)

- `aider-ai` `a4be6ccd…b626`: `git ls-remote https://github.com/Aider-AI/aider.git refs/tags/v0.86.0`
  (lightweight tag, direct hit) + local `git -C /var/tmp/opencode/m4-showcase/aider log -1` → 2025-08-09.
- `chunkhound-ai` `a8553ae6…cc7d`: `git ls-remote …/chunkhound/chunkhound.git refs/tags/v6.0.0`
  (direct hit) + local m4-showcase log → 2026-09-14.
- `ripgrep-m4` `4649aa97…e16c`: `git ls-remote …/ripgrep.git 'refs/tags/14.1.1^{}'` (peeled) +
  local m4-showcase log → 2024-09-08.
- `curl-m4` `75a2079d…7844c`: `git ls-remote …/curl/curl.git 'refs/tags/curl-8_11_1^{}'` (peeled) +
  local m4-showcase log → 2024-12-11.
- `ripgrep-pre` `7cb21137…3136`: `git ls-remote …/ripgrep.git 'refs/tags/12.1.1^{}'` (peeled) +
  shallow clone `git clone --depth 1 --branch 12.1.1 … /var/tmp/opencode/div-verify/ripgrep-pre`, log → 2020-05-29 (< 2022-01-01 ✓).
- `ripgrep-post` `3a612f88…97fc`: `git ls-remote …/ripgrep.git 'refs/tags/15.0.0^{}'` (peeled) +
  shallow clone `--branch 15.0.0`, log → 2025-10-15 (≥ 2024-01-01 ✓).
- `curl-pre` `9e560d11…884f`: `git ls-remote …/curl/curl.git 'refs/tags/curl-7_80_0^{}'` (peeled) +
  shallow clone `--branch curl-7_80_0`, log → 2021-11-10 (< 2022-01-01 ✓).
- `curl-post` `34cf9d54…7133`: `git ls-remote …/curl/curl.git 'refs/tags/curl-8_12_0^{}'` (peeled) +
  shallow clone `--branch curl-8_12_0`, log → 2025-02-05 (≥ 2024-01-01 ✓).
- `requests-pre` `a1a6a549…5a4f6`: `git ls-remote …/psf/requests.git` (lightweight `refs/tags/v2.26.0`,
  direct hit) + shallow clone `--branch v2.26.0`, log → 2021-07-13 (< 2022-01-01 ✓).
- `requests-post` `021dc729…c20d`: `git ls-remote …/psf/requests.git 'refs/tags/v2.32.4^{}'` (peeled) +
  shallow clone `--branch v2.32.4`, log → 2025-06-09 (≥ 2024-01-01 ✓).

Annotated tags resolve via the `^{}` peel; lightweight tags match directly. Verification
clones went to `/var/tmp/opencode/div-verify/` (scratch, deletable); the canonical
checkouts for the scan step go to `/var/tmp/opencode/div-corpus/` via the script.

## 9. Provenance

- Plan `divergence-revamp` bullet 4; specs `divergence-study` §Corpus, `divergence-paper` §Corpus.
- No product code changed: `src/`, schema, and tests untouched (docs + `scripts/` only).
- No counts or hashes fabricated: every scan/proof slot reads PENDING until the scan step runs.
