# Showcase repro packet — curl `curl-8_11_1` (C + Python (tests), curl (MIT-like))

> Part of plan `evidence-beta` **M4** (AI-code credibility showcase), the ADR-0005 publish-gate
> precondition. Umbrella record: `docs/evidence/ai-code-showcase.md`. Raw counts:
> `docs/evidence/showcase/curl/counts.json`; full verdict rows:
> `docs/evidence/showcase/curl/verified.json`.

## 1. Corpus (pinned)
- **Repo:** https://github.com/curl/curl
- **Tag → commit:** `curl-8_11_1` → `75a2079d5c28debb2eaa848ca9430f1fe0d7844c`
- **License:** curl (MIT-like)
- **Cloned outside the product repo** at `curl/` (CWD-relative) via:
  ```bash
  git clone --depth 1 --branch curl-8_11_1 https://github.com/curl/curl.git curl
  git -C curl rev-parse HEAD   # 75a2079d5c28debb2eaa848ca9430f1fe0d7844c
  ```

## 2. One-command scan
```bash
perfpal curl -o out-curl
```
**Expected summary (exit 0):** `scanned 923 files (687 ok, 236 partial, 0 failed) · 11004 findings (225 hot, 1308 warn, 9471 info)`

## 3. Per-rule counts (32 distinct rules)
| rule | total | hot | warn | info |
|---|---:|---:|---:|---:|
| `PERF-C-ALLOC-001` | 68 | 68 | 0 | 0 |
| `PERF-C-ALLOC-002` | 107 | 37 | 70 | 0 |
| `PERF-C-ALLOC-003` | 2 | 2 | 0 | 0 |
| `PERF-C-ARCH-001` | 1254 | 0 | 189 | 1065 |
| `PERF-C-ARCH-002` | 1 | 1 | 0 | 0 |
| `PERF-C-ARCH-003` | 6958 | 0 | 0 | 6958 |
| `PERF-C-CALL-001` | 16 | 0 | 0 | 16 |
| `PERF-C-CALL-002` | 502 | 0 | 502 | 0 |
| `PERF-C-LOOP-001` | 34 | 0 | 34 | 0 |
| `PERF-C-LOOP-002` | 392 | 117 | 275 | 0 |
| `PERF-C-LOOP-003` | 11 | 0 | 0 | 11 |
| `PERF-C-LOOP-004` | 37 | 0 | 0 | 37 |
| `PERF-C-LOOP-005` | 145 | 0 | 0 | 145 |
| `PERF-C-LOOP-006` | 83 | 0 | 0 | 83 |
| `PERF-C-LOOP-007` | 36 | 0 | 0 | 36 |
| `PERF-C-LOOP-008` | 191 | 0 | 0 | 191 |
| `PERF-C-MEM-002` | 78 | 0 | 63 | 15 |
| `PERF-C-STR-002` | 32 | 0 | 32 | 0 |
| `PERF-C-STR-003` | 55 | 0 | 55 | 0 |
| `PERF-C-TYPE-002` | 46 | 0 | 0 | 46 |
| `PERF-PY-ALLOC-003` | 47 | 0 | 47 | 0 |
| `PERF-PY-ARCH-002` | 1 | 0 | 0 | 1 |
| `PERF-PY-ARCH-003` | 827 | 0 | 0 | 827 |
| `PERF-PY-ASYNC-002` | 1 | 0 | 1 | 0 |
| `PERF-PY-CALL-001` | 21 | 0 | 0 | 21 |
| `PERF-PY-CALL-002` | 7 | 0 | 0 | 7 |
| `PERF-PY-COMPREHENSION-001` | 1 | 0 | 0 | 1 |
| `PERF-PY-LOOP-001` | 3 | 0 | 0 | 3 |
| `PERF-PY-LOOP-002` | 28 | 0 | 28 | 0 |
| `PERF-PY-LOOP-007` | 11 | 0 | 11 | 0 |
| `PERF-PY-LOOP-009` | 8 | 0 | 0 | 8 |
| `PERF-PY-STR-001` | 1 | 0 | 1 | 0 |

## 4. Determinism proof
Two scans into separate output dirs; `metrics.json` compared **byte-identical after masking the
documented provenance fields** (`scan.started_at`, `scan.duration_ms`).
The two runs differ only in the masked provenance fields (`started_at` and/or `duration_ms`); the masked copies are byte-identical.
- Run-1 masked `metrics.json` sha256: `302b53cb5cf035455575b314741bf18785a9bbaa7ece9f5857e42a072177a55d`
- Result: **IDENTICAL** across both runs.

## 5. Hand-verified findings (stratified sample, n=60)
Sample = up to 3 findings for the dominant rule + up to 2 per other rule, preferring
`hot` → `warn` → `info`. Verdicts: **TP** = claim holds in the real code; **FP** = claim is
wrong; **AMBIG** = technically present but harmless/context-dependent.
**Tally: TP=51 · FP=8 · AMBIG=1** (85% TP of sample).

### Per-rule tally
| rule | TP | FP | AMBIG | n |
|---|---:|---:|---:|---:|
| `PERF-C-ALLOC-001` | 2 | 0 | 0 | 2 |
| `PERF-C-ALLOC-002` | 2 | 0 | 0 | 2 |
| `PERF-C-ALLOC-003` | 2 | 0 | 0 | 2 |
| `PERF-C-ARCH-001` | 2 | 0 | 0 | 2 |
| `PERF-C-ARCH-002` | 1 | 0 | 0 | 1 |
| `PERF-C-ARCH-003` | 2 | 1 | 0 | 3 |
| `PERF-C-CALL-001` | 2 | 0 | 0 | 2 |
| `PERF-C-CALL-002` | 2 | 0 | 0 | 2 |
| `PERF-C-LOOP-001` | 2 | 0 | 0 | 2 |
| `PERF-C-LOOP-002` | 2 | 0 | 0 | 2 |
| `PERF-C-LOOP-003` | 2 | 0 | 0 | 2 |
| `PERF-C-LOOP-004` | 0 | 2 | 0 | 2 |
| `PERF-C-LOOP-005` | 2 | 0 | 0 | 2 |
| `PERF-C-LOOP-006` | 1 | 1 | 0 | 2 |
| `PERF-C-LOOP-007` | 2 | 0 | 0 | 2 |
| `PERF-C-LOOP-008` | 2 | 0 | 0 | 2 |
| `PERF-C-MEM-002` | 2 | 0 | 0 | 2 |
| `PERF-C-STR-002` | 1 | 1 | 0 | 2 |
| `PERF-C-STR-003` | 2 | 0 | 0 | 2 |
| `PERF-C-TYPE-002` | 1 | 0 | 1 | 2 |
| `PERF-PY-ALLOC-003` | 2 | 0 | 0 | 2 |
| `PERF-PY-ARCH-002` | 1 | 0 | 0 | 1 |
| `PERF-PY-ARCH-003` | 0 | 2 | 0 | 2 |
| `PERF-PY-ASYNC-002` | 1 | 0 | 0 | 1 |
| `PERF-PY-CALL-001` | 1 | 1 | 0 | 2 |
| `PERF-PY-CALL-002` | 2 | 0 | 0 | 2 |
| `PERF-PY-COMPREHENSION-001` | 1 | 0 | 0 | 1 |
| `PERF-PY-LOOP-001` | 2 | 0 | 0 | 2 |
| `PERF-PY-LOOP-002` | 2 | 0 | 0 | 2 |
| `PERF-PY-LOOP-007` | 2 | 0 | 0 | 2 |
| `PERF-PY-LOOP-009` | 2 | 0 | 0 | 2 |
| `PERF-PY-STR-001` | 1 | 0 | 0 | 1 |

### Full verdict rows
| rule | location | verdict | decisive span | note |
|---|---|---|---|---|
| `PERF-C-ARCH-003` | `curl/lib/asyn-thread.c:191` | **TP** | `Curl_freeaddrinfo(tsd->res);` | name match real; defined in curl_addrinfo.c:78 - genuine cross-file call |
| `PERF-C-ARCH-003` | `curl/lib/asyn-thread.c:248` | **FP** | `tsd->hostname = strdup(hostname);` | strdup is libc; src/tool_strdup.c is the curl-tool binary, unrelated to libcurl |
| `PERF-C-ARCH-003` | `curl/lib/asyn-thread.c:270` | **TP** | `Curl_addrinfo_callback(data, tsd->sock_error, tsd->res);` | name match real; defined in hostasyn.c:66 |
| `PERF-C-ARCH-001` | `curl/include/curl/mprintf.h:29` | **TP** | `#include "curl.h"` | curl.h:3246 includes mprintf.h -> real include cycle |
| `PERF-C-ARCH-001` | `curl/include/curl/multi.h:51` | **TP** | `#include "curl.h"` | curl.h:3240 includes multi.h -> real include cycle |
| `PERF-PY-ARCH-003` | `curl/tests/dictserver.py:63` | **FP** | `log.info("[DICT] Listening on %s", local_bind)` | logging.Logger.info; scorecard.py's `info` method is unrelated |
| `PERF-PY-ARCH-003` | `curl/tests/dictserver.py:84` | **FP** | `data = self.request.recv(1024).strip()` | socket.recv; name-only match to negtelnetserver's recv |
| `PERF-C-CALL-002` | `curl/docs/examples/chkspeed.c:64` | **TP** | `int main(int argc, char *argv[])` | CLI parser: 2 switches (13+8 cases) + many ifs, >15 branches |
| `PERF-C-CALL-002` | `curl/docs/examples/debug.c:35` | **TP** | `void dump(const char *text,` | hex-dump with many compound `&&` conditions ~18 branch points |
| `PERF-C-LOOP-002` | `curl/docs/examples/certinfo.c:74` | **TP** | `printf("%s\n", slist->data);` | printf executed per inner-loop iteration (depth 2), stdio I/O |
| `PERF-C-LOOP-002` | `curl/docs/examples/cookie_interface.c:55` | **TP** | `printf("[%d]: %s\n", i, nc->data);` | printf in while-loop, depth 1 |
| `PERF-C-LOOP-008` | `curl/docs/examples/crawler.c:143` | **TP** | `nodeset->nodeTab[x]->xmlChildrenNode` | 3-deep deref chain in innermost loop |
| `PERF-C-LOOP-008` | `curl/docs/examples/ephiperfifo.c:529` | **TP** | `events[idx].data.fd` | 3-deep member chain inside inner for loop |
| `PERF-C-LOOP-005` | `curl/docs/examples/synctime.c:277` | **TP** | `ShowAllHeader = 1;` | global assigned inside while(argv) loop |
| `PERF-C-LOOP-005` | `curl/docs/examples/synctime.c:280` | **TP** | `AutoSyncTime = 1;` | global assigned inside while(argv) loop |
| `PERF-C-ALLOC-002` | `curl/lib/formdata.c:369` | **TP** | `free(fname);` | free inside FormAdd `while(return_value==OK)` loop |
| `PERF-C-ALLOC-002` | `curl/lib/formdata.c:458` | **TP** | `free(type);` | free inside same FormAdd while loop |
| `PERF-C-LOOP-006` | `curl/lib/multi.c:2958` | **TP** | `data->set.fmultidone(data, result);` | indirect call inside do-while loop |
| `PERF-C-LOOP-006` | `curl/lib/socketpair.c:224` | **FP** | `#ifdef WSAEWOULDBLOCK` | line is a preprocessor directive; no indirect call - macro parse artifact |
| `PERF-C-MEM-002` | `curl/lib/asyn-ares.c:712` | **TP** | `memcpy(ca->ai_addr, ai->ai_addr, ss_size);` | variable-size copy every iteration of addrinfo loop |
| `PERF-C-MEM-002` | `curl/lib/cfilters.c:931` | **TP** | `memmove(&ps->sockets[i], &ps->sockets[i + 1],` | array-shift memmove inside for loop |
| `PERF-C-ALLOC-001` | `curl/lib/asyn-ares.c:693` | **TP** | `ca = malloc(sizeof(struct Curl_addrinfo) + ss_size);` | malloc nested in addrinfo loop body |
| `PERF-C-ALLOC-001` | `curl/lib/c-hyper.c:456` | **TP** | `data->req.newurl = strdup(data->state.url);` | strdup nested inside while(1) loop |
| `PERF-C-STR-003` | `curl/docs/examples/hsts-preload.c:60` | **TP** | `strcpy(e->name, host);` | unbounded strcpy present |
| `PERF-C-STR-003` | `curl/docs/examples/hsts-preload.c:62` | **TP** | `strcpy(e->expire, expire);` | unbounded strcpy present |
| `PERF-PY-ALLOC-003` | `curl/tests/http/conftest.py:67` | **TP** | `report.extend([line])` | list grown inside `for line in file_in` loop |
| `PERF-PY-ALLOC-003` | `curl/tests/http/scorecard.py:150` | **TP** | `errors.append(err)` | list append inside for loop |
| `PERF-C-TYPE-002` | `curl/lib/socks.c:82` | **AMBIG** | `struct socks_state {` | ~8B padding exists, but the claim's '64 vs 53 / ~11B' numbers are wrong |
| `PERF-C-TYPE-002` | `curl/docs/examples/ephiperfifo.c:82` | **TP** | `typedef struct _GlobalInfo` | 40 aligned vs 32 packed confirmed (~8B padding) |
| `PERF-C-LOOP-004` | `curl/docs/examples/crawler.c:142` | **FP** | `int x = r * nodeset->nodeNr / RAND_MAX;` | divisor RAND_MAX is a constant, not non-constant as claimed |
| `PERF-C-LOOP-004` | `curl/lib/cf-https-connect.c:361` | **FP** | `for(i = 0; i < sizeof(ballers)/sizeof(ballers[0]); i++)` | divisor is a sizeof constant, compile-time folded |
| `PERF-C-LOOP-007` | `curl/lib/multi.c:3234` | **TP** | `if(s == last_ps->sockets[j])` | linear socket scan in nested loop |
| `PERF-C-LOOP-007` | `curl/lib/multi.c:3313` | **TP** | `if(s == ps->sockets[j])` | linear socket scan in nested loop |
| `PERF-C-LOOP-001` | `curl/lib/altsvc.c:585` | **TP** | `while(ISBLANK(*p))` | while subset for(;;) subset do -> depth 3 |
| `PERF-C-LOOP-001` | `curl/lib/altsvc.c:597` | **TP** | `while(*p && ISBLANK(*p))` | same depth-3 nest |
| `PERF-C-STR-002` | `curl/docs/examples/crawler.c:151` | **FP** | `strlen(link) < 20` | `link` is reassigned each iteration (line 150); hoist invalid |
| `PERF-C-STR-002` | `curl/lib/altsvc.c:561` | **TP** | `dstlen = strlen(srchost);` | srchost is invariant param over do-while loop |
| `PERF-PY-LOOP-002` | `curl/tests/http/test_02_download.py:539` | **TP** | `open(srcfile).readlines()` | file I/O per loop iteration |
| `PERF-PY-LOOP-002` | `curl/tests/http/test_02_download.py:540` | **TP** | `open(dfile).readlines()` | file I/O per loop iteration |
| `PERF-PY-CALL-001` | `curl/tests/ech_combos.py:70` | **TP** | `CombinationRepetitionUtil(chosen, arr, badarr, index + 1,` | direct recursion, 2 call sites |
| `PERF-PY-CALL-001` | `curl/tests/http/test_11_unix.py:64` | **FP** | `self._thread.start()` | calls Thread.start, not method `start` itself - name collision |
| `PERF-C-CALL-001` | `curl/docs/examples/htmltidy.c:72` | **TP** | `dumpNode(doc, child, indent + 4);` | direct recursion |
| `PERF-C-CALL-001` | `curl/lib/curl_fnmatch.c:285` | **TP** | `if(loop(p, s, maxstars) == CURL_FNMATCH_MATCH)` | `loop` directly recursive |
| `PERF-PY-LOOP-007` | `curl/tests/http/scorecard.py:530` | **TP** | `if sskey not in sizes:` | list membership scan inside loop |
| `PERF-PY-LOOP-007` | `curl/tests/http/scorecard.py:533` | **TP** | `if mkey not in measures:` | list membership scan in nested loop |
| `PERF-C-LOOP-003` | `curl/docs/examples/debug.c:81` | **TP** | `fputc('\n', stream);` | char-wise I/O in loop body |
| `PERF-C-LOOP-003` | `curl/docs/examples/http2-download.c:97` | **TP** | `fputc('\n', stderr);` | char-wise I/O in loop body |
| `PERF-PY-LOOP-009` | `curl/tests/http/test_11_unix.py:72` | **TP** | `try:` | try/except inside `while self._done is False` loop |
| `PERF-PY-LOOP-009` | `curl/tests/http/test_11_unix.py:74` | **TP** | `try:` | nested try inside loop |
| `PERF-PY-CALL-002` | `curl/tests/http/scorecard.py:431` | **TP** | `def score(self,` | ~19 branch points > 15 |
| `PERF-PY-CALL-002` | `curl/tests/http/scorecard.py:507` | **TP** | `def print_score(self, score):` | large function, ~43 branches > 15 |
| `PERF-PY-LOOP-001` | `curl/tests/http/scorecard.py:83` | **TP** | `for _ in range(sample_size):` | 3-deep loop nest |
| `PERF-PY-LOOP-001` | `curl/tests/http/test_17_ssl_use.py:194` | **TP** | `for [ciphers12, succeed12] in tls12_tests:` | 3-deep loop nest |
| `PERF-C-ALLOC-003` | `curl/src/tool_urlglob.c:133` | **TP** | `char **new_arr = realloc(pat->content.Set.elements,` | additive (+1 element) realloc inside while loop |
| `PERF-C-ALLOC-003` | `curl/tests/server/rtspd.c:380` | **TP** | `req->rtp_buffer = realloc(req->rtp_buffer,` | additive realloc inside do-while loop |
| `PERF-C-ARCH-002` | `curl/lib/setopt.c:258` | **TP** | `static CURLcode setopt_long(` | lib/ is the largest component; aggregate claim plausible |
| `PERF-PY-ASYNC-002` | `curl/tests/http/testenv/ws_echo_server.py:38` | **TP** | `await websocket.send(message)` | await inside `async for` loop |
| `PERF-PY-STR-001` | `curl/tests/http/scorecard.py:258` | **TP** | `error += f'[{f["response_code"]}]'` | string `+=` accumulated in loop (O(n^2)) |
| `PERF-PY-ARCH-002` | `curl/tests/http/scorecard.py:507` | **TP** | `def print_score(self, score):` | tests/http is largest Python dir; aggregate plausible |
| `PERF-PY-COMPREHENSION-001` | `curl/tests/http/test_17_ssl_use.py:195` | **TP** | `ret.append([tls_proto, ciphers13, ciphers12, succeed13, succeed12])` | only statement in innermost loop; comprehension candidate |

## 6. Limitations observed on this repo
- The strongest repo in the sample (**51 TP / 8 FP / 1 AMBIG**), but note the composition: many TPs are in `docs/examples/` (example programs), and the repo is **macro-heavy** — 236 of 923 files parsed **partially**.
- **63% of volume is `PERF-C-ARCH-003`** (6958 of 11004) + a further `PERF-C-ARCH-001` 11%.
- `PERF-C-ARCH-003` is mixed: genuine cross-file calls (`lib/asyn-thread.c:191/270`) are TPs, but libc-name collisions like `strdup` ↔ `src/tool_strdup.c` (`:248`) are FPs.
- `PERF-C-LOOP-004` fired **0/2 TP** — the divisors are constants (`RAND_MAX`, `sizeof(...)`).
- `PERF-C-LOOP-006` FP on a raw `#ifdef` line (macro parse artifact) — a symptom of the 236 partial parses.
- `PERF-C-STR-002` FP when the tested string is reassigned each iteration (`crawler.c:151`).

## 7. Provenance
- Plan `evidence-beta` M4; spec `evidence-beta-spec` §6; ADR-0005.
- Binary: `perfpal 0.2.0-beta.1 (metrics schema 1.3)`, release build (`--locked`).
- Scan tier: **syntactic** (default). Python/Rust are syntactic-only; C here is also syntactic (no libclang).
- Generated from `counts.json` + `verified.json`.
