//! `cli` — entry point & orchestration (modules.md §3.1).
//!
//! Consumes: argv. Produces: validated invocation options; scan provenance
//! (`started_at`, `root_path` verbatim, tool version); process exit code;
//! stdout/stderr text (§8).
//!
//! Responsibilities: parse/validate arguments; run stages in pipeline order;
//! collect wall-clock duration; map stage failures to exit codes (0 success /
//! 1 usage / 2 output incl. safety-guard refusal / 3 internal bug); print the
//! stdout summary (explicitly NOT a stability contract).
//!
//! Non-goals: analysis logic; config files; filesystem inspection beyond
//! existence/readability checks needed for argument validation.
//!
//! P0 status: scaffold only. Flag parsing and pipeline wiring land in P1.

/// Process exit code mapping lives here so `main` stays trivial.
/// Returns 0 (success) for now; real mapping arrives with the P1 CLI surface.
pub fn run() -> i32 {
    0
}
