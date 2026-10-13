# Showcase repro packet — Aider `v0.86.0` (Python, Apache-2.0)

> Part of plan `evidence-beta` **M4** (AI-code credibility showcase), the ADR-0005 publish-gate
> precondition. Umbrella record: `docs/evidence/ai-code-showcase.md`. Raw counts:
> `docs/evidence/showcase/aider/counts.json`; full verdict rows:
> `docs/evidence/showcase/aider/verified.json`.

## 1. Corpus (pinned)
- **Repo:** https://github.com/Aider-AI/aider
- **Tag → commit:** `v0.86.0` → `a4be6ccd87ebaa59b361f3f028d116ce1761b626`
- **License:** Apache-2.0
- **Cloned outside the product repo** at `aider/` (CWD-relative) via:
  ```bash
  git clone --depth 1 --branch v0.86.0 https://github.com/Aider-AI/aider.git aider
  git -C aider rev-parse HEAD   # a4be6ccd87ebaa59b361f3f028d116ce1761b626
  ```

## 2. One-command scan
```bash
perfpal aider -o out-aider
```
**Expected summary (exit 0):** `scanned 148 files (148 ok, 0 partial, 0 failed) · 1567 findings (0 hot, 256 warn, 1311 info)`

## 3. Per-rule counts (15 distinct rules)
| rule | total | hot | warn | info |
|---|---:|---:|---:|---:|
| `PERF-PY-ALLOC-001` | 7 | 0 | 0 | 7 |
| `PERF-PY-ALLOC-003` | 128 | 0 | 128 | 0 |
| `PERF-PY-ALLOC-004` | 38 | 0 | 0 | 38 |
| `PERF-PY-ARCH-002` | 1 | 0 | 0 | 1 |
| `PERF-PY-ARCH-003` | 1072 | 0 | 0 | 1072 |
| `PERF-PY-CALL-001` | 48 | 0 | 0 | 48 |
| `PERF-PY-CALL-002` | 57 | 0 | 0 | 57 |
| `PERF-PY-COMPREHENSION-001` | 16 | 0 | 0 | 16 |
| `PERF-PY-LOOP-001` | 11 | 0 | 0 | 11 |
| `PERF-PY-LOOP-002` | 28 | 0 | 28 | 0 |
| `PERF-PY-LOOP-007` | 18 | 0 | 18 | 0 |
| `PERF-PY-LOOP-009` | 56 | 0 | 0 | 56 |
| `PERF-PY-MEM-002` | 5 | 0 | 0 | 5 |
| `PERF-PY-N1-001` | 14 | 0 | 14 | 0 |
| `PERF-PY-STR-001` | 68 | 0 | 68 | 0 |

## 4. Determinism proof
Two scans into separate output dirs; `metrics.json` compared **byte-identical after masking the
documented provenance fields** (`scan.started_at`, `scan.duration_ms`).
The two runs differ only in the masked provenance fields (`started_at` and/or `duration_ms`); the masked copies are byte-identical.
- Run-1 masked `metrics.json` sha256: `cd1af6ec5d7883ade8e2a752992c828a32a768ce5d075c6050a7559a415fa8a9`
- Result: **IDENTICAL** across both runs.

## 5. Hand-verified findings (stratified sample, n=30)
Sample = up to 3 findings for the dominant rule + up to 2 per other rule, preferring
`hot` → `warn` → `info`. Verdicts: **TP** = claim holds in the real code; **FP** = claim is
wrong; **AMBIG** = technically present but harmless/context-dependent.
**Tally: TP=20 · FP=6 · AMBIG=4** (67% TP of sample).

### Per-rule tally
| rule | TP | FP | AMBIG | n |
|---|---:|---:|---:|---:|
| `PERF-PY-ALLOC-001` | 1 | 0 | 1 | 2 |
| `PERF-PY-ALLOC-003` | 2 | 0 | 0 | 2 |
| `PERF-PY-ALLOC-004` | 1 | 0 | 1 | 2 |
| `PERF-PY-ARCH-002` | 1 | 0 | 0 | 1 |
| `PERF-PY-ARCH-003` | 1 | 2 | 0 | 3 |
| `PERF-PY-CALL-001` | 0 | 2 | 0 | 2 |
| `PERF-PY-CALL-002` | 2 | 0 | 0 | 2 |
| `PERF-PY-COMPREHENSION-001` | 2 | 0 | 0 | 2 |
| `PERF-PY-LOOP-001` | 2 | 0 | 0 | 2 |
| `PERF-PY-LOOP-002` | 1 | 0 | 1 | 2 |
| `PERF-PY-LOOP-007` | 2 | 0 | 0 | 2 |
| `PERF-PY-LOOP-009` | 2 | 0 | 0 | 2 |
| `PERF-PY-MEM-002` | 0 | 2 | 0 | 2 |
| `PERF-PY-N1-001` | 1 | 0 | 1 | 2 |
| `PERF-PY-STR-001` | 2 | 0 | 0 | 2 |

### Full verdict rows
| rule | location | verdict | decisive span | note |
|---|---|---|---|---|
| `PERF-PY-ARCH-003` | `aider/analytics.py:181` | **FP** | `data_file.write_text(json.dumps(data, indent=4))` | pathlib write_text (1-arg); io.py:478 is a different method (filename,content) - duck-typing name-only match |
| `PERF-PY-ARCH-003` | `aider/analytics.py:199` | **TP** | `model_info_manager.get_model_from_cached_json_db(model.name)` | agent imports model_info_manager; method defined at models.py:204 - real cross-file coupling |
| `PERF-PY-ARCH-003` | `aider/analytics.py:208` | **FP** | `print("X" * 100)` | builtin print, not InputOutput.print (io.py:1047) - name-only false positive |
| `PERF-PY-ALLOC-003` | `aider/args_formatter.py:218` | **TP** | `parts.append(f"  - `{switch} {metavar}`")` | append inside `for switch in action.option_strings`; parts built before loop, accumulates |
| `PERF-PY-ALLOC-003` | `aider/args_formatter.py:220` | **TP** | `parts.append(f"  - `{switch}`")` | same loop else-branch; list accumulates across iterations |
| `PERF-PY-STR-001` | `aider/coders/base_coder.py:612` | **TP** | `all_content += content + "\n"` | += on str inside for-loop over generator - immutable-copy O(n^2) |
| `PERF-PY-STR-001` | `aider/coders/base_coder.py:616` | **TP** | `all_content += content + "\n"` | += on str inside second for-loop; genuinely accumulates |
| `PERF-PY-CALL-002` | `aider/args_formatter.py:105` | **TP** | `elif action.nargs in ("*", "+") or isinstance(action, argparse._AppendAction):` | _format_action has ~16 branch points; high-complexity confirmed |
| `PERF-PY-CALL-002` | `aider/coders/base_coder.py:125` | **TP** | `if edit_format != from_coder.edit_format and done_messages` | create() has many conditionals/branches (>15) - genuinely complex flow |
| `PERF-PY-LOOP-009` | `aider/coders/base_coder.py:691` | **TP** | `for fname in self.get_all_relative_files():` | try/except (691-699) inside the for-loop body |
| `PERF-PY-LOOP-009` | `aider/coders/base_coder.py:883` | **TP** | `try:` | try/except at 883 nested inside `while True:` (882) |
| `PERF-PY-CALL-001` | `aider/args_formatter.py:13` | **FP** | `super().start_section(res)` | calls PARENT via super(), not itself - not direct recursion |
| `PERF-PY-CALL-001` | `aider/args_formatter.py:86` | **FP** | `super().start_section(res)` | same super() delegation in YamlHelpFormatter - not self-recursion |
| `PERF-PY-ALLOC-004` | `aider/coders/base_coder.py:193` | **AMBIG** | `res.original_kwargs = dict(kwargs)` | dict(non-literal) in loop body, but returns immediately after match -> runs <=1x |
| `PERF-PY-ALLOC-004` | `aider/coders/base_coder.py:934` | **TP** | `list(self.send_message(message))` | list(...) wraps generator (non-literal) inside `while message:` loop |
| `PERF-PY-LOOP-002` | `aider/coders/base_coder.py:838` | **TP** | `with open(fname, "rb") as image_file:` | blocking open inside `for fname in fnames` - real per-file I/O |
| `PERF-PY-LOOP-002` | `aider/coders/base_coder.py:1459` | **AMBIG** | `yield from self.send(messages, functions=self.functions)` | inside retry `while True`/except; deliberate retry-until-success, not per-element I/O |
| `PERF-PY-LOOP-007` | `aider/coders/editblock_coder.py:578` | **TP** | `if fname in valid_fnames:` | valid_fnames is a list (default []); membership scan in loop |
| `PERF-PY-LOOP-007` | `aider/coders/search_replace.py:715` | **TP** | `if method not in methods:` | `methods = []` list; O(n) `not in` scan inside loop (dedup pattern) |
| `PERF-PY-COMPREHENSION-001` | `aider/coders/base_coder.py:283` | **TP** | `lines.append(f"Added {fname} to the chat.")` | loop body's only statement is a list append |
| `PERF-PY-COMPREHENSION-001` | `aider/coders/base_coder.py:1243` | **TP** | `example_messages.append(` | sole loop-body statement is append (multiline call) |
| `PERF-PY-N1-001` | `aider/scripts/issues.py:89` | **AMBIG** | `response = requests.get(` | per-page fetch in `while True` pagination (page+=1); inherent, not per-element N+1 |
| `PERF-PY-N1-001` | `aider/scripts/issues.py:164` | **TP** | `response = requests.get(comments_url, headers=headers)` | one HTTP request per issue inside `for issue in issues` - genuine N+1 |
| `PERF-PY-LOOP-001` | `aider/coders/patch_coder.py:451` | **TP** | `for i, scope in enumerate(scope_lines):` | while(421)>while(448)>for(451) = depth 3 |
| `PERF-PY-LOOP-001` | `aider/coders/patch_coder.py:469` | **TP** | `for i, scope in enumerate(scope_lines):` | while(421)>while(467)>for(469) = depth 3 |
| `PERF-PY-ALLOC-001` | `aider/commands.py:406` | **TP** | `lint_coder.abs_fnames = set()` | set() constructed inside `for fname in fnames` (377) each pass |
| `PERF-PY-ALLOC-001` | `aider/linter.py:283` | **AMBIG** | `result[fname] = set()` | inside loop but guarded by `if fname not in result`; one-time init per new key |
| `PERF-PY-MEM-002` | `aider/benchmark/benchmark.py:768` | **FP** | `shutil.copy(original_fname, src)` | shutil.copy is a FILESYSTEM copy, not container copy.copy/deepcopy - rule misidentified |
| `PERF-PY-MEM-002` | `aider/benchmark/benchmark.py:1014` | **FP** | `shutil.copy(src, dst)` | filesystem copy in loop, not a container copy |
| `PERF-PY-ARCH-002` | `aider/main.py:451` | **TP** | `def main(argv=None, input=None, output=None` | directory-level `aider` complexity aggregate anchored at entry point |

## 6. Limitations observed on this repo
- The scan reports **1567 findings, of which 1072 (68%) are the single `PERF-PY-ARCH-003`** (name-based cross-file coupling, all `info`, `LOW` confidence). This rule is the volume driver.
- `ARCH-003` is imperfect by construction (name-only): sample verdicts `analytics.py:181` (pathlib `write_text` vs a same-named method) and `analytics.py:208` (builtin `print`) are **FPs**; a genuine coupling (`analytics.py:199`) is a TP.
- `PERF-PY-CALL-001` false-positives on `super()` delegation (`args_formatter.py:13/86`) — parent-class calls are not recursion.
- `PERF-PY-MEM-002` false-positives on `shutil.copy` (`benchmark.py:768/1014`) — a filesystem copy, not a container copy.

## 7. Provenance
- Plan `evidence-beta` M4; spec `evidence-beta-spec` §6; ADR-0005.
- Binary: `perfpal 0.2.0-beta.1 (metrics schema 1.3)`, release build (`--locked`).
- Scan tier: **syntactic** (default). Python/Rust are syntactic-only; C here is also syntactic (no libclang).
- Generated from `counts.json` + `verified.json`.
