//! `writer` — atomic output management (modules.md §3.8; output-contract §3).
//!
//! Consumes: staged artifact bytes (metrics.json, bundled schema copy,
//! report.md, charts/*.svg). Produces: the report directory per
//! output-contract §2.
//!
//! Responsibilities: build in a temporary sibling directory;
//! rename-aside-then-swap (POSIX rename(2) cannot overwrite a non-empty dir);
//! safety guard — refuse replacing a directory that lacks metrics.json unless
//! `--force`; remove stale temp directories from crashed runs on the next
//! successful run; enforce encoding (UTF-8, LF, trailing newline).
//!
//! Failure behavior: any upstream failure aborts before the swap — the
//! previous complete report survives intact; interrupted runs never leave
//! half-written reports.
//!
//! P0 status: scaffold only. Atomic swap + safety guard land in P1.
