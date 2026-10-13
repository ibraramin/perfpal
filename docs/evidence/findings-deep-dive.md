# Perfpal — Findings Deep Dive (production-code `hot` tier)

> **Scope (locked, session 45 → session 46).** Triage **every** production-code `hot` finding emitted by
> Perfpal on the four pinned M4 showcase corpora, and hunt for a standing-out performance bug evidenced at
> **L3 (quantified impact) + L4 (upstream corroboration)**. Deliverable = this dossier + machine-readable
> per-repo verdicts (`docs/evidence/showcase/<repo>/triage.json`).
> **No product code was changed by this milestone.**

---

## 1. Method

### 1.1 Corpora (pinned, reused from M4)
| Repo | Tag | Commit | License | Language |
|---|---|---|---|---|
| Aider | `v0.86.0` | `a4be6ccd87ebaa59b361f3f028d116ce1761b626` | Apache-2.0 | Python |
| chunkhound | `v6.0.0` | `a8553ae644686d8ee55b94ee74ff560a38e6cc7d` | MIT | Python + Rust |
| ripgrep | `14.1.1` | `4649aa9700619f94cf9c66876e9549d83420e16c` | MIT/Unlicense | Rust |
| curl | `curl-8_11_1` | `75a2079d5c28debb2eaa848ca9430f1fe0d7844c` | curl (MIT-like) | C |

Checkouts live at `/var/tmp/opencode/m4-showcase/<repo>/` (un-shallowed to full history for the L4 search).

### 1.2 Candidate selection
"Production code" = **exclude** any finding whose path contains a `tests/`, `testing/`, `testdata/`,
`benches/`, `examples/`, or `fixtures/` path segment, or whose file name matches `test_*`, `*_test`,
`testutil.*`, or `util.rs`. Everything else (including `lib/`, `src/`, `crates/*/src`) counts.

### 1.3 Triage protocol
Each candidate was **individually hand-read** at its `path:span` in the pinned checkout and given one verdict:
- **GENUINE** — the rule's claim holds *and* the containing function is on a reachable path where the loop
  iterates many times with meaningful per-iteration cost (a real optimization opportunity).
- **REAL_BUT_MINOR** — the claim holds but the site is a one-shot / config-parse / init / teardown /
  diagnostic-print path, or the loop is trivially short.
- **FP** — the claim does not actually hold in this code (allocation outside the loop; the loop runs 0/1×;
  `do{...}while(0)` single pass; `#if 0`/`#ifdef`-out code; a different symbol).

### 1.4 Evidence ladder
| Level | Meaning |
|---|---|
| **L1** | The tool emitted the finding (raw signal). |
| **L2** | Hand-confirmed real **and reachable on a non-test path**. |
| **L3** | **Impact quantified** (loop trip count / per-iteration cost / allocation volume). |
| **L4** | **Upstream corroboration** — a commit/PR/issue on the *same spot* confirming it is a real perf concern. |

---

## 2. Scope reconciliation — the locked number was ~259; the reproducible number is **181**

The session-45 handoff enumerated "~259" prod-`hot` findings (chunkhound 26, ripgrep 34, **curl 199**).
Two of those three match this milestone's filter exactly; **curl does not**:

| Repo | raw `hot` (incl. tests) | **prod-`hot` (this filter)** | handoff figure |
|---|---|---|---|
| Aider | 0 | **0** | 0 ✅ |
| chunkhound | 26 | **26** | 26 ✅ |
| ripgrep | 40 | **34** | 34 ✅ |
| curl | 225 | **121** | 199 ❌ |
| **Total** | **291** | **181** | ~259 |

Raw `hot` totals are anchored in the M4 packets (`docs/evidence/showcase-{aider,chunkhound,ripgrep,curl}.md:22`:
0 / 26 / 40 / 225). The chunkhound and ripgrep figures reproduce exactly. The curl figure does not: since
chunkhound = 26 and curl's `hot` total = 225, the handoff's `199 = 225 − 26` is almost certainly a copy of
chunkhound's 26 subtracted from curl's total. curl's raw→prod drop of 104 is `tests/*` (81) +
`docs/examples/*` (23), computed from the M4 scan outputs. **The reproducible production-`hot` set is 181**
(curl 121). This dossier triages all 181; the discrepancy is recorded rather than silently adopted.

---

## 3. Results

### 3.1 Aggregate verdicts
| Repo | candidates | GENUINE | REAL_BUT_MINOR | FP | FP rate |
|---|---|---|---|---|---|
| chunkhound | 26 | **6** | 16 | 4 | 15% |
| ripgrep | 34 | **1** | 24 | 9 | 26% |
| curl | 121 | **2** | 84 | 35 | 29% |
| **Total** | **181** | **9** | **124** | **48** | **27%** |

### 3.2 Per rule
| Rule | n | GENUINE | REAL_BUT_MINOR | FP |
|---|---|---|---|---|
| `PERF-C-ALLOC-001` (C: alloc in loop) | 61 | 0 | 52 | 9 |
| `PERF-C-ALLOC-002` (C: free in loop) | 30 | 0 | 16 | 14 |
| `PERF-C-ALLOC-003` (C: additive realloc) | 1 | 0 | 1 | 0 |
| `PERF-C-LOOP-002` (C: expensive call in loop) | 28 | 2 | 14 | 12 |
| `PERF-C-ARCH-002` (C: hottest dir) | 1 | 0 | 1 | 0 |
| `PERF-RS-ALLOC-001` (Rust: alloc in loop) | 45 | 6 | 30 | 9 |
| `PERF-RS-LOOP-002` (Rust: call in loop) | 13 | 0 | 9 | 4 |
| `PERF-RS-ARCH-002` (Rust: hottest dir) | 2 | 1 | 1 | 0 |

### 3.3 Reachability
- **chunkhound:** 18/26 on a hot path (pipeline / DB write), 8 one-shot.
- **ripgrep:** 3/34 on a hot path, 31 one-shot (help/man/doc generation, startup flag parse).
- **curl:** 84 reachable, 35 not (dead/`#if`-out/platform-gated), 2 platform-dependent.

**Reading:** the `hot` tier is dominated by two things — (a) **one-shot** paths the coarse loop-depth
signal cannot distinguish from hot loops, and (b) **platform-/build-gated or dead** code. Only
**9 of 181 (5%)** are genuine candidates by this rubric.

---

## 4. The 9 GENUINE findings

| # | Repo | Finding | Reach | Why genuine |
|---|---|---|---|---|
| 1 | chunkhound | `PERF-RS-ALLOC-001:src/db/duckdb_backend/write.rs:239` | hot | Per embedding row: `serde_json::to_string` of the **whole vector** + 2 `to_string`, at **loop depth 3**, on the embedding-write path. |
| 2 | chunkhound | `PERF-RS-ALLOC-001:src/pipeline/pipeline.rs:1493` | hot | 2 `to_string` per chunk for **loop-invariant** `provider`/`model` — classic hoist. |
| 3 | chunkhound | `PERF-RS-ALLOC-001:src/db/duckdb_backend/write.rs:133` | hot | Inner per-chunk loop (depth 2): 3 `to_string` (symbol/language/metadata) per chunk on the chunk-insert path. |
| 4 | chunkhound | `PERF-RS-ALLOC-001:src/pipeline/pipeline.rs:922` | hot | Depth 3 (batch>file>chunk): `to_string` copies each chunk's `embed_text`/code per chunk. |
| 5 | chunkhound | `PERF-RS-ALLOC-001:src/pipeline/pipeline.rs:1431` | hot | Per embedding: `collect::<Vec<f32>>` allocates a dims-length vector converting f64→f32, for every embedded chunk. |
| 6 | chunkhound | `PERF-RS-ARCH-002:src/db/duckdb_backend/compaction.rs:32` | hot | Aggregation correct — `src/db/duckdb_backend` (the write path) is genuinely the hottest component. |
| 7 | ripgrep | `PERF-RS-ALLOC-001:crates/globset/src/lib.rs:424` | hot | `to_owned`/`to_string` per glob pattern while building a `GlobSet` (walk path, once per ignore-file compile). |
| 8 | curl | `PERF-C-LOOP-002:lib/file.c:582` | yes | `read()` is the per-chunk body of the `file://` download loop — many iterations, real syscall cost. |
| 9 | curl | `PERF-C-LOOP-002:lib/file.c:387` | yes | `write()` is the per-chunk body of the `file://` upload loop — many iterations, real syscall cost. |

**Orchestrator disposition of the 9:**
- **#1, #2 are the strongest** (see §6). #3–#6 are the same "per-chunk `to_string`/`collect` on the
  chunkhound pipeline/DB path" family — real but individually small.
- **#7 is refuted by upstream** — the ripgrep maintainer explicitly treats `GlobSet` construction
  allocations as negligible (§6.4).
- **#8, #9** are detections that hold, but the rule's advice ("hoist/batch") is **inapplicable** — the call
  *is* the streaming loop; you cannot hoist a `read()` out of a file-transfer loop. Detection TP, recommendation FP.

---

## 5. False-positive classes (the honest engine analysis)

48/181 (27%) are FP. Ordered by consequence:

1. **One-shot paths the loop signal can't see** (~dominant). `PERF-C-ALLOC-001`/`-002` fire on config /
   cookie / header / netrc / auth / DNS parsing that runs **once per invocation** — e.g.
   `lib/cookie.c:850` `parse_netscape` (`strdup` per TAB field, but the field loop is ≤6 iterations).
   Same shape in `lib/formdata.c`, `lib/http.c`, `lib/vauth/digest.c`.
2. **`do{...}while(0)` mis-read as a loop** — `lib/vtls/gtls.c:1255`, `lib/vtls/openssl.c:4409`,
   `lib/vtls/sectransp.c:1777`. The `while(0)` body runs exactly once. (Note: `error_node_overlap` was
   correctly flagged `MEDIUM` on some of these — the signal exists but does not gate severity.)
3. **`#if 0` / `#ifdef`-out / debug-only code** — `lib/conncache.c:1357-1365` (whole function under `#if 0`),
   `lib/easy.c:572` (`#if DEBUG_EV_POLL`), `lib/curl_addrinfo.c:287` (`#if !HAVE_GETADDRINFO`),
   `lib/getenv.c:48` (`#elif defined(_WIN32)`), `packages/OS400/*` (non-standard build).
4. **`String::new()` / `vec![]` are not heap allocations** — `PERF-RS-ALLOC-001` counts them;
   `crates/core/flags/doc/help.rs:28,113`, `crates/core/flags/doc/man.rs:29`,
   `crates/globset/src/glob.rs:712`. An empty `String`/`Vec` has no buffer.
5. **Error-path `free` / allocation inside a closure** — the call executes once on an error branch
   (`lib/socks_sspi.c:*` all 8 sites; `lib/formdata.c:369,458`; lazy `with_context`/`map_err` closures in
   `crates/core/flags/parse.rs:227`, `crates/ignore/src/types.rs:337`).
6. **The loop body is the loop's purpose (blocking/channel/streaming I/O)** — `src/pipeline/pipeline.rs`
   `recv`/`send` (#`PERF-RS-LOOP-002`), and `lib/file.c` `read`/`write`. These are correctly *detected*
   but the "hoist/batch" recommendation is incoherent for them.
7. **`#[cfg(test)]` code inside a production-named file** — `src/db/duckdb_backend/hnsw.rs:90` sits under
   a `#[cfg(test)]` gate (line 89); `crates/ignore/src/lib.rs:529` sits inside `mod tests` (`#[cfg(test)]`,
   line 488). (The path filter removes files/dirs named `tests/`, `testutil.*`, etc., but a `#[cfg(test)]`
   gate **inside** a production-named file is a *content* condition the path filter cannot see.)

---

## 6. Hero-bug hunt (L2 → L3 → L4)

### 6.1 The standout (chunkhound `insert_embeddings_txn`)
**Spot:** `src/db/duckdb_backend/write.rs:239-243` (`PERF-RS-ALLOC-001`).
```rust
for (chunk_id, chunk) in chunk_slice.iter() {                       // depth 3: by_dims > chunks > rows
    let emb = chunk.embedding.as_ref().expect(...);
    let emb_json = serde_json::to_string(emb).map_err(DbError::Json)?;   // ← whole vector → JSON text
    params.push(duckdb::types::Value::Text(emb_json));                   // bound as ?::FLOAT[dims]
    ...
}
```
- **L2 (reachable):** yes — reached from `write_batch_incremental` (`:615`) and `write_batches_in_one_txn`
  (`:684`), driven by the pipeline store thread on **every `chunkhound index` run**.
- **L3 (impact):** the loop runs **once per stored embedding**. Each iteration JSON-serializes a
  `dims`-length `f32` vector (typically 768–1536 dims ≈ **10–30 KB of text**), then DuckDB parses that
  JSON back into `FLOAT[dims]`. This is a full serialize→parse round-trip per embedding, and the
  surrounding `row_ph` SQL is rebuilt per batch (`format!` + `join` + `replace`).
- **L4 (upstream):** **NOT met at the exact spot.** chunkhound `main` (tip `a131f5b`) **still contains**
  `serde_json::to_string(emb)` unchanged since the initial Rust port `5f9ddfd "Main rust flow (#380)"`
  (merged; the only commit that ever touched `write.rs`). No issue/PR names this line.
  - *Adjacent (partial) corroboration:* upstream's own **PR #375** ("feat(native): Phase 0 Rust DB writer",
    merged — https://github.com/chunkhound/chunkhound/pull/375) targeted the embedding-insert path for
    performance; and the **Python predecessor** bound embeddings natively via `executemany` (`embedding_repository.py:438-447`,
    a list, **no JSON**). So the subsystem is demonstrably perf-sensitive and the JSON round-trip is a real
    cost — but #375's removed round-trip was the **PyO3 extraction** path, *not* this DuckDB binding, so it
    is corroboration of the *mechanism class*, not of *this line*.

### 6.2 Secondary (chunkhound `embed_batch_parallel`)
`src/pipeline/pipeline.rs:1493-1497` — `chunk.provider = Some(provider.to_string())` /
`chunk.model = Some(model.to_string())` per chunk, where `provider`/`model` are `&str` **parameters**
(`:1387-1388`), i.e. constant across the entire call. A textbook loop-invariant hoist (2 allocations →
2 allocations total). **L2 yes, L3 small, L4 none** (upstream never touched it).

### 6.3 curl (`lib/file.c` read/write)
`lib/file.c:582` (`read`) and `:387` (`write`) are the per-chunk bodies of the `file://` transfer loops.
Real many-iteration reachable sites — but the allocation/call **is** the streaming mechanism, so the
rule's "hoist or batch" recommendation is wrong. Detection L2, no optimization.

### 6.4 L4-negative evidence (documented honestly)
- **ripgrep `GlobSet::new`** (`crates/globset/src/lib.rs:424`) — upstream **PR #1915** (closed) asked to
  preallocate; BurntSushi: *"allocating memory is a pittance compared to what a globset builder is actually
  doing."* The triage marked this `GENUINE` (it runs once per ignore-file compiled on the walk path), but
  the maintainer treats globset-construction allocations as negligible **by design** — so the concern is
  **L4-refuted**: a true detection whose optimization is not worth making.
- **curl `parse_netscape`** (`lib/cookie.c:850`) — the **exact** function was rewritten **16 days after our
  snapshot** (commit `39e21794a7`, PR **#15826**, 2024-12-27), but the motive was a **const-input crash**
  ("Parse the input string without modifying it"), **not** performance. Related cookie-perf work exists
  (PR #19864 → `c6e5dfa2` "allocate the main struct once"; #15824 bagder on per-line alloc/free), but none
  targets our spot for perf.

### 6.5 Verdict
**No candidate satisfies the strict L4 bar** — there is **no perf-motivated upstream commit/PR/issue on any
exact flagged spot**. The strongest outcome is **L2/L3** (`write.rs:239`, with subsystem-level corroboration).
This is the honest result of the hunt and is consistent with the north-star's "verifiable truth per finding":
the `hot` tier on real code is **architecturally sound but conservative** (5% GENUINE), and its precision is
gated by the same name-/context-blind rules M4 already named.

---

## 7. Limitations & follow-ups
- **L4 negative, not L4 failure of the tool.** No upstream perf confirmation exists for these spots; that is
  a fact about upstream history, not evidence the findings are wrong. Perfpal never claimed ground truth.
- **Filter excludes by path, not by `#[cfg(test)]`/feature gate.** `hnsw.rs:90` (test-only) and several
  `#if`-gated curl sites reached the candidate set. A *reachability* signal (cfg/ifdef awareness) would lift
  precision on the `hot` tier — deferred (not this evidence task).
- **`String::new()`/`vec![]` zero-alloc forms** are counted by `PERF-RS-ALLOC-001`; a cheap constant-folding
  fix would remove a whole FP class (#4, §5).
- **Rule recommendation ≠ detection.** `PERF-C-LOOP-002`/`RS-LOOP-002` correctly detect many-iteration I/O
  but their "hoist/batch" advice is incoherent for streaming/channel loops; message or severity could reflect
  "intrinsic per-iteration I/O".
- All of the above are **precision follow-ups** (M1 KPI + M2 benchmark measured) — **not** gate-blocking, and
  **no product code changed here**.

---

## 8. Reproduce
```bash
# 1. Corpora (pinned) — the M4 packet §1 commands; e.g. curl
git clone https://github.com/curl/curl /var/tmp/opencode/m4-showcase/curl
git -C /var/tmp/opencode/m4-showcase/curl checkout 75a2079d5c28debb2eaa848ca9430f1fe0d7844c

# 2. Scan
cargo build --release --locked
./target/release/perfpal /var/tmp/opencode/m4-showcase/curl -o /tmp/out-curl   # exit 0

# 3. Production-hot candidate set  (expected: aider 0, chunkhound 26, ripgrep 34, curl 121 → 181)
python3 - <<'PY'
import json, re
seg = re.compile(r'(^|/)(tests?|testing|testdata|benches?|examples?|fixtures?)(/)', re.I)
fname = re.compile(r'(^|/)test[_-]|_test\.|testutil\.|(^|/)util\.rs$', re.I)
d = json.load(open('/tmp/out-curl/metrics.json'))
hot = [f for f in d['findings'] if f['severity'] == 'hot'
       and not seg.search('/' + f['path'] + '/') and not fname.search(f['path'])]
print(len(hot))            # curl → 121
PY

# 4. Verdicts: docs/evidence/showcase/<repo>/triage.json  (finding_id → GENUINE|REAL_BUT_MINOR|FP)
```

## 9. Change set (this milestone)
- **Added:** `docs/evidence/findings-deep-dive.md`; `docs/evidence/showcase/{chunkhound,ripgrep,curl}/triage.json`.
- **No product code changed.** (`src/`, `Cargo.toml`, schema, goldens untouched.)
