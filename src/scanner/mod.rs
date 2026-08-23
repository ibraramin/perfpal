//! `scanner` — discovery, parsing, IR emission (modules.md §3.2).
//!
//! Consumes: target path; invocation options. Produces: the complete IR stream
//! (file-level records, FunctionRecords, events — ir-v1.md §3) in deterministic
//! order.
//!
//! Responsibilities:
//! - Recursive discovery, lexicographically sorted emission; dot-dirs skipped;
//!   symlinks not followed (v1); per-file failures become `ParseHealth{failed}`
//!   records, never run aborts (total target failure = usage error, exit 1).
//! - Language detection by extension: `.c`/`.h` are C in v1; others skipped
//!   silently. `scan.file_count` = `len(files[])`; zero-C-file scans still
//!   record `scan.languages: ["c"]`.
//! - Parse via the bound backend; capture ParseHealth (status, error regions,
//!   error node count).
//! - Quoted-include resolution against the filesystem (includer-relative
//!   first); angle-bracket includes bucketed external (ADR-0001); unresolvable
//!   quoted includes emit `IncludeEdge{resolved:false}` — ARCH-001 signal.
//! - Curated macro-pattern recognizer pass → MacroEvents.
//! - Engine-native span → IR span conversion happens at this boundary
//!   exclusively (+1 on entry, −1 on exit; ir-v1.md §2).
//!
//! Non-goals: semantic resolution; macro expansion; build-flag awareness;
//! scoring.

pub mod backends;
