# Showcase repro packet — chunkhound `v6.0.0` (Python + Rust, MIT)

> Part of plan `evidence-beta` **M4** (AI-code credibility showcase), the ADR-0005 publish-gate
> precondition. Umbrella record: `docs/evidence/ai-code-showcase.md`. Raw counts:
> `docs/evidence/showcase/chunkhound/counts.json`; full verdict rows:
> `docs/evidence/showcase/chunkhound/verified.json`.

## 1. Corpus (pinned)
- **Repo:** https://github.com/chunkhound/chunkhound
- **Tag → commit:** `v6.0.0` → `a8553ae644686d8ee55b94ee74ff560a38e6cc7d`
- **License:** MIT
- **Cloned outside the product repo** at `chunkhound/` (CWD-relative) via:
  ```bash
  git clone --depth 1 --branch v6.0.0 https://github.com/chunkhound/chunkhound.git chunkhound
  git -C chunkhound rev-parse HEAD   # a8553ae644686d8ee55b94ee74ff560a38e6cc7d
  ```

## 2. One-command scan
```bash
perfpal chunkhound -o out-chunkhound
```
**Expected summary (exit 0):** `scanned 887 files (887 ok, 0 partial, 0 failed) · 8124 findings (26 hot, 1125 warn, 6973 info)`

## 3. Per-rule counts (31 distinct rules)
| rule | total | hot | warn | info |
|---|---:|---:|---:|---:|
| `PERF-PY-ALLOC-001` | 9 | 0 | 0 | 9 |
| `PERF-PY-ALLOC-003` | 718 | 0 | 718 | 0 |
| `PERF-PY-ALLOC-004` | 36 | 0 | 0 | 36 |
| `PERF-PY-ARCH-001` | 1 | 0 | 0 | 1 |
| `PERF-PY-ARCH-002` | 1 | 0 | 0 | 1 |
| `PERF-PY-ARCH-003` | 6047 | 0 | 0 | 6047 |
| `PERF-PY-ASYNC-001` | 17 | 0 | 17 | 0 |
| `PERF-PY-ASYNC-002` | 251 | 0 | 251 | 0 |
| `PERF-PY-CALL-001` | 228 | 0 | 0 | 228 |
| `PERF-PY-CALL-002` | 242 | 0 | 0 | 242 |
| `PERF-PY-COMPREHENSION-001` | 60 | 0 | 0 | 60 |
| `PERF-PY-LOOP-001` | 22 | 0 | 0 | 22 |
| `PERF-PY-LOOP-002` | 16 | 0 | 16 | 0 |
| `PERF-PY-LOOP-007` | 30 | 0 | 30 | 0 |
| `PERF-PY-LOOP-009` | 200 | 0 | 0 | 200 |
| `PERF-PY-MEM-002` | 11 | 0 | 0 | 11 |
| `PERF-PY-N1-001` | 42 | 0 | 42 | 0 |
| `PERF-PY-PANDAS-001` | 4 | 0 | 0 | 4 |
| `PERF-PY-STR-001` | 10 | 0 | 10 | 0 |
| `PERF-RS-ALLOC-001` | 22 | 22 | 0 | 0 |
| `PERF-RS-ARCH-002` | 1 | 1 | 0 | 0 |
| `PERF-RS-ARCH-003` | 97 | 0 | 0 | 97 |
| `PERF-RS-CALL-001` | 10 | 0 | 0 | 10 |
| `PERF-RS-CALL-002` | 7 | 0 | 7 | 0 |
| `PERF-RS-LOOP-001` | 2 | 0 | 2 | 0 |
| `PERF-RS-LOOP-002` | 4 | 3 | 1 | 0 |
| `PERF-RS-LOOP-004` | 1 | 0 | 0 | 1 |
| `PERF-RS-LOOP-008` | 3 | 0 | 0 | 3 |
| `PERF-RS-STR-002` | 28 | 0 | 28 | 0 |
| `PERF-RS-TYPE-001` | 3 | 0 | 3 | 0 |
| `PERF-RS-TYPE-002` | 1 | 0 | 0 | 1 |

## 4. Determinism proof
Two scans into separate output dirs; `metrics.json` compared **byte-identical after masking the
documented provenance fields** (`scan.started_at`, `scan.duration_ms`).
The two runs differ only in the masked provenance fields (`started_at` and/or `duration_ms`); the masked copies are byte-identical.
- Run-1 masked `metrics.json` sha256: `693ccbd380480e7ef7d72e5f8f5506a364bc31bd9b7541037735800583f6f3b8`
- Result: **IDENTICAL** across both runs.

## 5. Hand-verified findings (stratified sample, n=58)
Sample = up to 3 findings for the dominant rule + up to 2 per other rule, preferring
`hot` → `warn` → `info`. Verdicts: **TP** = claim holds in the real code; **FP** = claim is
wrong; **AMBIG** = technically present but harmless/context-dependent.
**Tally: TP=40 · FP=17 · AMBIG=1** (69% TP of sample).

### Per-rule tally
| rule | TP | FP | AMBIG | n |
|---|---:|---:|---:|---:|
| `PERF-PY-ALLOC-001` | 0 | 2 | 0 | 2 |
| `PERF-PY-ALLOC-003` | 2 | 0 | 0 | 2 |
| `PERF-PY-ALLOC-004` | 2 | 0 | 0 | 2 |
| `PERF-PY-ARCH-001` | 0 | 1 | 0 | 1 |
| `PERF-PY-ARCH-002` | 1 | 0 | 0 | 1 |
| `PERF-PY-ARCH-003` | 2 | 1 | 0 | 3 |
| `PERF-PY-ASYNC-001` | 2 | 0 | 0 | 2 |
| `PERF-PY-ASYNC-002` | 2 | 0 | 0 | 2 |
| `PERF-PY-CALL-001` | 0 | 2 | 0 | 2 |
| `PERF-PY-CALL-002` | 2 | 0 | 0 | 2 |
| `PERF-PY-COMPREHENSION-001` | 1 | 1 | 0 | 2 |
| `PERF-PY-LOOP-001` | 2 | 0 | 0 | 2 |
| `PERF-PY-LOOP-002` | 1 | 1 | 0 | 2 |
| `PERF-PY-LOOP-007` | 2 | 0 | 0 | 2 |
| `PERF-PY-LOOP-009` | 2 | 0 | 0 | 2 |
| `PERF-PY-MEM-002` | 2 | 0 | 0 | 2 |
| `PERF-PY-N1-001` | 2 | 0 | 0 | 2 |
| `PERF-PY-PANDAS-001` | 2 | 0 | 0 | 2 |
| `PERF-PY-STR-001` | 1 | 0 | 1 | 2 |
| `PERF-RS-ALLOC-001` | 2 | 0 | 0 | 2 |
| `PERF-RS-ARCH-002` | 1 | 0 | 0 | 1 |
| `PERF-RS-ARCH-003` | 1 | 1 | 0 | 2 |
| `PERF-RS-CALL-001` | 0 | 2 | 0 | 2 |
| `PERF-RS-CALL-002` | 2 | 0 | 0 | 2 |
| `PERF-RS-LOOP-001` | 2 | 0 | 0 | 2 |
| `PERF-RS-LOOP-002` | 2 | 0 | 0 | 2 |
| `PERF-RS-LOOP-004` | 0 | 1 | 0 | 1 |
| `PERF-RS-LOOP-008` | 2 | 0 | 0 | 2 |
| `PERF-RS-STR-002` | 0 | 2 | 0 | 2 |
| `PERF-RS-TYPE-001` | 0 | 2 | 0 | 2 |
| `PERF-RS-TYPE-002` | 0 | 1 | 0 | 1 |

### Full verdict rows
| rule | location | verdict | decisive span | note |
|---|---|---|---|---|
| `PERF-PY-ARCH-003` | `chunkhound/api/cli/commands/autodoc.py:28` | **TP** | `formatter.error(message)` | `error` genuinely defined at rich_output.py:173 on the formatter class |
| `PERF-PY-ARCH-003` | `chunkhound/api/cli/commands/autodoc.py:34` | **FP** | `Path(getattr(args, "out_dir")).resolve()` | `resolve` is a pathlib method, unrelated to llm_provider.py:162 |
| `PERF-PY-ARCH-003` | `chunkhound/api/cli/commands/autodoc.py:37` | **TP** | `git_utils.maybe_warn_git_output_dir(` | `git_utils`=&autodoc_git; real def at autodoc_git.py:49 |
| `PERF-PY-ALLOC-003` | `chunkhound/api/cli/commands/autodoc_autorun.py:37` | **TP** | `output.append(item)` | list grows via append across `for item in items` loop |
| `PERF-PY-ALLOC-003` | `chunkhound/api/cli/commands/autodoc_autorun.py:220` | **TP** | `details.append(f"- Missing LLM configuration: {item}.")` | append accumulates across loop iterations |
| `PERF-PY-ASYNC-002` | `chunkhound/api/cli/commands/autodoc_generate.py:66` | **TP** | `return await call_generate_docsite(` | await inside `for attempt in range(2)` loop |
| `PERF-PY-ASYNC-002` | `chunkhound/api/cli/commands/autodoc_generate.py:94` | **TP** | `current_map_dir = await autorun.autorun_code_mapper_for_autodoc(` | await inside for loop |
| `PERF-PY-CALL-002` | `chunkhound/api/cli/commands/code_mapper.py:89` | **TP** | `async def code_mapper_command(args: argparse.Namespace, config: Config)` | ~32 branch keywords in body, >15 |
| `PERF-PY-CALL-002` | `chunkhound/api/cli/commands/mcp.py:127` | **TP** | `def _show_mcp_setup_instructions(` | ~23 branches in body, >15 |
| `PERF-PY-CALL-001` | `chunkhound/api/cli/commands/code_mapper.py:57` | **FP** | `return await code_mapper_pipeline.run_code_mapper_overview_hyde(` | calls another module's fn, not self-recursion |
| `PERF-PY-CALL-001` | `chunkhound/api/cli/utils/rich_output.py:593` | **FP** | `task_id = self.progress.add_task(` | rich Progress method; not self-recursion |
| `PERF-PY-LOOP-009` | `chunkhound/api/cli/commands/autodoc_generate.py:65` | **TP** | `try:` | try/except inside `for attempt` loop body |
| `PERF-PY-LOOP-009` | `chunkhound/api/cli/commands/run.py:427` | **TP** | `try:` | try/except inside `for p in skipped_timeouts` loop |
| `PERF-RS-ARCH-003` | `chunkhound/src/db/duckdb_backend/compaction.rs:33` | **FP** | `PathBuf::from(&self.config.db_path)` | std PathBuf::from, unrelated to error.rs `fn from` |
| `PERF-RS-ARCH-003` | `chunkhound/src/db/duckdb_backend/compaction.rs:39` | **TP** | `let conn = self.conn_or_err()?;` | method really defined mod.rs:50 |
| `PERF-PY-COMPREHENSION-001` | `chunkhound/api/cli/commands/autodoc_autorun.py:220` | **TP** | `details.append(f"- Missing LLM configuration: {item}.")` | loop body's only statement is the append |
| `PERF-PY-COMPREHENSION-001` | `chunkhound/autodoc/ia.py:328` | **FP** | `collected.append(line)` | body also has `if ...: break`; not 'only' append |
| `PERF-PY-N1-001` | `chunkhound/providers/database/duckdb/embedding_repository.py:218` | **TP** | `conn.execute(` | DB round-trip per `for index_info` iteration |
| `PERF-PY-N1-001` | `chunkhound/providers/database/duckdb/embedding_repository.py:236` | **TP** | `conn.execute(create_sql)` | DB call per loop iteration |
| `PERF-PY-ALLOC-004` | `chunkhound/daemon/client_manager.py:77` | **TP** | `for cid, sess in list(self._sessions.items())` | list() over non-literal iterable inside `while True` |
| `PERF-PY-ALLOC-004` | `chunkhound/parsers/mappings/go.py:554` | **TP** | `go_files = list(pkg_dir.glob("*.go"))` | list() over generator inside `for prefix` loop |
| `PERF-PY-LOOP-007` | `chunkhound/core/config/indexing_config.py:568` | **TP** | `if pat not in out:` | list membership scan inside for loop |
| `PERF-PY-LOOP-007` | `chunkhound/parsers/mappings/groovy.py:328` | **TP** | `if annotation_text and annotation_text not in annotations:` | list `in` test inside loop |
| `PERF-RS-STR-002` | `chunkhound/src/db/duckdb_backend/write.rs:29` | **FP** | `if !e.is_empty() {` | is_empty() is O(1) and `e` is loop-varying; no string build |
| `PERF-RS-STR-002` | `chunkhound/src/db/duckdb_backend/write.rs:30` | **FP** | `dims.insert(e.len() as u32);` | len() O(1), loop-varying; not a string build |
| `PERF-RS-ALLOC-001` | `chunkhound/src/db/duckdb_backend/compaction.rs:47` | **TP** | `let _ = conn.execute_batch(&format!("DROP TABLE IF EXISTS \"{}\"", name));` | format! allocates per iteration |
| `PERF-RS-ALLOC-001` | `chunkhound/src/db/duckdb_backend/compaction.rs:99` | **TP** | `&format!("SELECT COALESCE(MAX(id), 0) FROM src.\"{}\"", tname)` | format! allocates per iteration |
| `PERF-PY-LOOP-001` | `chunkhound/parsers/mappings/css.py:99` | **TP** | `for prop_child in block_child.children:` | loops nest 3 deep (children->block_child->prop_child) |
| `PERF-PY-LOOP-001` | `chunkhound/parsers/mappings/lua.py:183` | **TP** | `for var in subchild.children:` | 3-deep nest (children->subchild->var) |
| `PERF-PY-ASYNC-001` | `chunkhound/services/embedding_service.py:161` | **TP** | `with open(debug_file, "a") as f:` | blocking open inside async method (await at line 172) |
| `PERF-PY-ASYNC-001` | `chunkhound/services/embedding_service.py:200` | **TP** | `with open(debug_file, "a") as f:` | blocking open inside same async method |
| `PERF-PY-LOOP-002` | `chunkhound/code_mapper/hyde.py:233` | **TP** | `with path.open("rb") as f:` | file open per loop iteration |
| `PERF-PY-LOOP-002` | `chunkhound/daemon/client_proxy.py:300` | **FP** | `line = await stdin.readline()` | async non-blocking readline, not a blocking I/O call |
| `PERF-PY-MEM-002` | `chunkhound/mcp_server/base.py:1321` | **TP** | `tool_params = copy.deepcopy(tool.parameters)` | deepcopy per iteration of TOOL_REGISTRY loop |
| `PERF-PY-MEM-002` | `chunkhound/parsers/svelte_parser.py:134` | **TP** | `chunk.metadata.copy()` | dict copy at loop depth 2 |
| `PERF-PY-STR-001` | `chunkhound/api/cli/commands/search.py:235` | **AMBIG** | `line_info += f"-{end_line}"` | `+=` in loop, but line_info reset each pass so no O(n^2) growth |
| `PERF-PY-STR-001` | `chunkhound/parsers/mappings/markdown.py:203` | **TP** | `heading_text += text_content + " "` | accumulates across loop -> O(n^2) |
| `PERF-RS-CALL-001` | `chunkhound/src/db/duckdb_backend/mod.rs:43` | **FP** | `known_dims: HashSet::new(),` | HashSet::new is not self-recursion on `new` |
| `PERF-RS-CALL-001` | `chunkhound/src/db/duckdb_backend/mod.rs:69` | **FP** | `let conn = Connection::open(&self.config.db_path)?;` | Connection::open, not the method `open` recursing |
| `PERF-PY-ALLOC-001` | `chunkhound/code_mapper/hyde.py:233` | **FP** | `with path.open("rb") as f:` | open() is not a container literal (set/list/dict) |
| `PERF-PY-ALLOC-001` | `chunkhound/daemon/client_proxy.py:180` | **FP** | `self._shutdown_event.set()` | Event.set() method, not constructing a set |
| `PERF-RS-CALL-002` | `chunkhound/src/db/duckdb_backend/compaction.rs:32` | **TP** | `fn run_attach_copy_compaction(&mut self) -> Result<(), DbError> {` | ~37 branch/`?` points >15 |
| `PERF-RS-CALL-002` | `chunkhound/src/db/duckdb_backend/write.rs:367` | **TP** | `fn pre_delete_for_upsert(` | ~23 branch/`?` points >15 |
| `PERF-RS-LOOP-002` | `chunkhound/src/pipeline/pipeline.rs:707` | **TP** | `let recv = store_rx.recv();` | blocking channel recv in loop at depth 2 |
| `PERF-RS-LOOP-002` | `chunkhound/src/pipeline/pipeline.rs:902` | **TP** | `let recv = parse_rx.recv();` | blocking channel recv in loop |
| `PERF-PY-PANDAS-001` | `chunkhound/providers/database/lancedb_provider.py:1278` | **TP** | `for _, row in existing_data_df.iterrows():` | iterrows row-by-row iteration |
| `PERF-PY-PANDAS-001` | `chunkhound/providers/database/lancedb_provider.py:1427` | **TP** | `for _, row in existing_df.iterrows():` | iterrows row-by-row iteration |
| `PERF-RS-TYPE-001` | `chunkhound/src/db/duckdb_backend/mod.rs:37` | **FP** | `pub fn new(config: DbConfig) -> Self {` | DbConfig is Clone (moved, not Copy); constructor called once |
| `PERF-RS-TYPE-001` | `chunkhound/src/db/mod.rs:81` | **FP** | `pub fn create_backend(cfg: DbConfig) -> Box<dyn DbBackend> {` | arg moved once at startup; not copied per call |
| `PERF-RS-LOOP-008` | `chunkhound/src/db/duckdb_backend/write.rs:206` | **TP** | `let chunk = &batch.files[file_idx].chunks[chunk_idx];` | depth-4 index chain inside loop |
| `PERF-RS-LOOP-008` | `chunkhound/src/db/duckdb_backend/write.rs:532` | **TP** | `if file.chunks[chunk_idx]` | depth-4 field/index chain in innermost loop |
| `PERF-RS-LOOP-001` | `chunkhound/src/db/duckdb_backend/write.rs:239` | **TP** | `for (chunk_id, chunk) in chunk_slice.iter() {` | 3-deep nest (by_dims->chunks->iter) |
| `PERF-RS-LOOP-001` | `chunkhound/src/pipeline/pipeline.rs:922` | **TP** | `for (ci, ch) in pf.chunks.iter().enumerate() {` | 3-deep nest (loop->fi->ci) |
| `PERF-RS-ARCH-002` | `chunkhound/src/db/duckdb_backend/compaction.rs:32` | **TP** | `fn run_attach_copy_compaction(&mut self) -> Result<(), DbError> {` | plausible directory-aggregate complexity claim |
| `PERF-PY-ARCH-001` | `chunkhound/services/indexing_coordinator.py:27` | **FP** | `if TYPE_CHECKING:` | import is TYPE_CHECKING-only; no runtime import cycle |
| `PERF-PY-ARCH-002` | `chunkhound/tests/test_qa_deterministic.py:350` | **TP** | `tests/ dir aggregate` | plausible for a 2465-function tests dir |
| `PERF-RS-LOOP-004` | `chunkhound/src/pipeline/differ.rs:122` | **FP** | `if files_scanned % DIFF_TICK_INTERVAL == 0 {` | divisor is `const DIFF_TICK_INTERVAL: usize = 200` (differ.rs:45) |
| `PERF-RS-TYPE-002` | `chunkhound/src/pipeline/config.rs:49` | **FP** | `pub(crate) struct PipelineConfig {` | not `#[repr(C)]`/FFI; Rust auto-reorders fields, padding N/A |

## 6. Limitations observed on this repo
- **74% of dry-run volume is `PERF-PY-ARCH-003`** (6047 of 8124), the name-based coupling rule.
- `PERF-RS-CALL-001` fires on `HashSet::new` / `Connection::open` (name collisions) — **0/2 TP** here.
- `PERF-PY-ALLOC-001` fires on `open()` and `Event.set()` (`0/2 TP` in sample) — the rule's name heuristic over-matches.
- `PERF-RS-STR-002` fires on O(1) `.is_empty()`/`.len()` — not an in-loop string build.
- `PERF-RS-TYPE-001` treats a by-value **move** as a copy; `PERF-RS-TYPE-002` claims padding on non-`repr(C)` structs.
- `PERF-PY-ARCH-001` fired on a `TYPE_CHECKING`-only import (no runtime cycle).
- Positive: the Python loop/allocation/async core (`ALLOC-003`, `ASYNC-001/002`, `LOOP-001/007/009`, `CALL-002`, `PANDAS-001`) is uniformly TP in the sample.

## 7. Provenance
- Plan `evidence-beta` M4; spec `evidence-beta-spec` §6; ADR-0005.
- Binary: `perfpal 0.2.0-beta.1 (metrics schema 1.3)`, release build (`--locked`).
- Scan tier: **syntactic** (default). Python/Rust are syntactic-only; C here is also syntactic (no libclang).
- Generated from `counts.json` + `verified.json`.
