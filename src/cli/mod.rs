//! `cli` — entry point & orchestration (modules.md §3.1).
//!
//! Consumes: argv. Produces: validated invocation options; scan provenance
//! (`started_at`, `root_path` verbatim, tool version); process exit code;
//! stdout/stderr text (§8).
//!
//! Responsibilities: parse/validate arguments; run stages in pipeline order
//! (scanner → analyzer → aggregator → serialize → writer); collect wall-clock
//! duration; map stage failures to exit codes (§8.2: 0 success / 1 usage /
//! 2 output incl. safety-guard refusal / 3 internal bug); print the stdout
//! summary (explicitly NOT a stability contract). Findings NEVER affect the
//! exit code — they are data (modules.md Principle 4); a zero-C-file scan
//! succeeds with a valid empty report (`languages: ["c"]`).
//!
//! Panic policy: the pipeline runs under [`std::panic::catch_unwind`]; an
//! unexpected panic maps to exit 3 with a plain diagnostic — no Rust
//! backtrace is part of normal operation (§8.2 internal-error row).
//!
//! Provenance clock: std-only ([`SystemTime`] + hand-rolled RFC3339
//! rendering in [`rfc3339_utc`]) — Cargo.toml is dependency-frozen and v1
//! needs exactly one timestamp format.
//!
//! Non-goals: analysis logic; config files; filesystem inspection beyond
//! what scanner/writer already own.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;

use crate::aggregator::Provenance;
use crate::writer::Artifact;

/// `--version` payload (modules.md §8.1): `perfpal <version> (metrics schema
/// <schema_version>)`. clap renders `{name} {version}`, so this constant
/// carries everything after the binary name. The suffix literal is locked to
/// [`crate::schema::SCHEMA_VERSION`] by a unit test below.
const VERSION_SUFFIX: &str = concat!(env!("CARGO_PKG_VERSION"), " (metrics schema ", "1.1", ")");

/// Validated invocation options (modules.md §8.1 flag table).
#[derive(Debug, Parser)]
#[command(
    name = "perfpal",
    about = "Static-analysis scan of a C codebase emitting an LLM-consumable metrics report.",
    version = VERSION_SUFFIX
)]
pub struct Args {
    /// Scan target: directory or single `.c`/`.h` file.
    #[arg(value_name = "PATH", default_value = ".")]
    pub path: PathBuf,
    /// Report directory; internal layout fixed by output-contract §2.
    #[arg(
        short = 'o',
        long,
        value_name = "DIR",
        default_value = "perfpal-report"
    )]
    pub output: PathBuf,
    /// Allow replacing an existing directory that contains no metrics.json
    /// (safety-guard override).
    #[arg(long)]
    pub force: bool,
}

/// Stage failures mapped to exit codes (modules.md §8.2). Constructed only
/// at the cli boundary; every variant renders a single-line stderr diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PipelineError {
    /// Exit 1: bad invocation — unknown flag, missing/unreadable target.
    #[error("usage error: {0}")]
    Usage(String),
    /// Exit 2: output problem — cannot create output dir, safety-guard
    /// refusal, write failure.
    #[error("output error: {0}")]
    Output(String),
    /// Exit 3: unexpected panic/crash — always a bug.
    #[error("internal error: {0} (this is a bug in perfpal)")]
    Internal(String),
}

/// Exit-code mapping, exposed standalone so the 3-internal path is directly
/// unit-testable (it cannot be triggered naturally through the pipeline).
#[must_use]
pub fn exit_code(err: &PipelineError) -> i32 {
    match err {
        PipelineError::Usage(_) => 1,
        PipelineError::Output(_) => 2,
        PipelineError::Internal(_) => 3,
    }
}

/// Counts backing the stdout summary line (non-normative, §8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunSummary {
    pub files: u32,
    pub ok: u32,
    pub partial: u32,
    pub failed: u32,
    pub findings: u32,
    pub hot: u32,
    pub warn: u32,
    pub info: u32,
}

impl RunSummary {
    fn render(&self, output_dir: &str) -> String {
        format!(
            "scanned {} files ({} ok, {} partial, {} failed) · {} findings \
             ({} hot, {} warn, {} info)\nreport: {}\n",
            self.files,
            self.ok,
            self.partial,
            self.failed,
            self.findings,
            self.hot,
            self.warn,
            self.info,
            output_dir,
        )
    }
}

/// Binary entry: parse argv, run the pipeline under catch_unwind, emit the
/// stdout summary / stderr diagnostics, return the process exit code.
#[must_use]
pub fn run() -> i32 {
    run_with(std::env::args_os())
}

/// [`run`] over an explicit argv (injection seam for integration tests).
#[must_use]
pub fn run_with<I: IntoIterator<Item = OsString>>(argv: I) -> i32 {
    let args = match Args::try_parse_from(argv) {
        Ok(args) => args,
        Err(err) => {
            // clap handles -h/--help/--version rendering; we own the EXIT
            // CODES (§8.2): help/version ⇒ 0, parse failure ⇒ 1 usage —
            // NOT clap's default 2, which collides with output-error.
            let _ = err.print();
            return if err.use_stderr() { 1 } else { 0 };
        }
    };

    guarded(&args.output.display().to_string(), || run_pipeline(&args))
}

/// Panic-catch seam around one pipeline pass (testable extraction of the
/// exact behavior [`run_with`] had inline): `Ok(summary)` prints the stdout
/// summary and exits 0; `Err(err)` prints the diagnostic and maps through
/// [`exit_code`]; an unwound panic prints the internal-error line and exits
/// 3 (§8.2). Private — the exit-3 path is locked by a `#[cfg(test)]` unit
/// test below, the least-surface option (integration tests cover 0/1/2
/// through [`run_with`], where panics cannot be triggered naturally).
fn guarded(output_dir: &str, f: impl FnOnce() -> Result<RunSummary, PipelineError>) -> i32 {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    match result {
        Ok(Ok(summary)) => {
            print!("{}", summary.render(output_dir));
            0
        }
        Ok(Err(err)) => {
            eprintln!("perfpal: {err}");
            exit_code(&err)
        }
        Err(_panic) => {
            eprintln!("perfpal: internal error: unexpected panic (this is a bug in perfpal)");
            3
        }
    }
}

/// One full pipeline pass (modules.md §2 stage order). Pure orchestration —
/// all decisions live in the stage modules.
fn run_pipeline(args: &Args) -> Result<RunSummary, PipelineError> {
    let started = SystemTime::now();

    // scanner: discovery + IR stream (usage error only if target unreadable).
    let mut units =
        crate::scanner::scan(&args.path).map_err(|err| PipelineError::Usage(err.to_string()))?;

    // resolver: semantic enrichment seam (modules.md §4.3) — Wave A records
    // only ParseHealth.resolver; identity/heuristic-visible state untouched.
    // Explicit-root contract: the resolver re-reads sources from disk, so the
    // caller threads the canonicalized scan target (the same value the
    // scanner anchors discovery on — the scan above already proved it
    // canonicalizes, so this cannot fail here).
    let scan_root = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| args.path.clone());
    crate::resolver::resolve(&mut units, &scan_root);

    // analyzer: heuristics in report §4 total order.
    let findings = crate::analyzer::analyze(&units);

    // Provenance capture — cli owns the clock (modules.md §3.1).
    let duration_ms =
        u64::try_from(started.elapsed().unwrap_or_default().as_millis()).unwrap_or(u64::MAX);
    let provenance = Provenance {
        root_path: args.path.display().to_string(),
        started_at: rfc3339_utc(started),
        duration_ms,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
    };

    // aggregator: complete schema-typed document.
    let document = crate::aggregator::assemble(&units, &findings, &provenance);

    // serializer: canonical bytes (a serialization failure is a bug).
    let json = crate::schema::to_json(&document).map_err(|err| {
        PipelineError::Internal(format!("metrics.json serialization failed: {err}"))
    })?;

    // Derived artifacts (modules.md Principle 5): report.md and charts/ are
    // pure functions of the final document. charts::render applies the
    // emission rule itself (a chart appears iff its source section is
    // present and non-empty), so an empty scan yields an empty chart set —
    // the writer still creates charts/ (output-contract §3).
    let charts = crate::charts::render(&document);
    let chart_names: Vec<String> = charts.iter().map(|(name, _)| (*name).to_string()).collect();
    let report = crate::reporter::render_report(&document, &chart_names);

    // writer: atomic directory swap (all writer failures are output errors).
    // rel paths use forward slashes; nested entries land in subdirectories.
    let mut artifacts = vec![
        Artifact::text("metrics.json", json),
        Artifact::text(
            crate::schema::SCHEMA_ARTIFACT_NAME,
            crate::schema::EMBEDDED_SCHEMA,
        ),
        Artifact::text("report.md", report),
    ];
    for (name, bytes) in &charts {
        artifacts.push(Artifact {
            rel_path: format!("charts/{name}"),
            bytes: bytes.clone(),
        });
    }
    crate::writer::write_report(&args.output, &artifacts, args.force)
        .map_err(|err| PipelineError::Output(err.to_string()))?;

    Ok(summarize(&document))
}

/// Derive the stdout-summary counts from the final document (single source:
/// the same numbers a consumer reads out of metrics.json).
fn summarize(document: &crate::schema::MetricsDocument) -> RunSummary {
    let health = document.parse_health_summary.unwrap_or_default();
    let (mut hot, mut warn, mut info) = (0u32, 0u32, 0u32);
    for finding in &document.findings {
        match finding.severity {
            crate::analyzer::Severity::Hot => hot += 1,
            crate::analyzer::Severity::Warn => warn += 1,
            crate::analyzer::Severity::Info => info += 1,
        }
    }
    RunSummary {
        files: document.scan.file_count,
        ok: health.ok,
        partial: health.partial,
        failed: health.failed,
        findings: u32::try_from(document.findings.len()).unwrap_or(u32::MAX),
        hot,
        warn,
        info,
    }
}

/// Format a [`SystemTime`] as UTC RFC3339 (`YYYY-MM-DDTHH:MM:SSZ`) using the
/// proleptic Gregorian civil-from-days algorithm (Hinnant). Pre-epoch instants
/// clamp to the epoch rather than panicking — provenance is never worth a
/// crash.
#[must_use]
pub fn rfc3339_utc(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = i64::try_from(secs / 86_400).unwrap_or(i64::MAX);
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600,
        (rem % 3_600) / 60,
        rem % 60
    )
}

/// Days since 1970-01-01 → `(year, month, day)` (Howard Hinnant's
/// `civil_from_days`; public-domain formulation).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (
        if m <= 2 { y + 1 } else { y },
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    //! Exit-code mapping locks (§8.2) + RFC3339 provenance formatting.

    use super::*;

    #[test]
    fn exit_codes_map_per_contract() {
        assert_eq!(exit_code(&PipelineError::Usage("x".into())), 1);
        assert_eq!(exit_code(&PipelineError::Output("x".into())), 2);
        assert_eq!(exit_code(&PipelineError::Internal("x".into())), 3);
    }

    #[test]
    fn guarded_maps_ok_err_and_panic_to_contract_codes() {
        // The panic→3 path cannot occur naturally through run_with, so the
        // guarded seam is exercised directly (§8.2 internal-error row).
        let empty = RunSummary {
            files: 0,
            ok: 0,
            partial: 0,
            failed: 0,
            findings: 0,
            hot: 0,
            warn: 0,
            info: 0,
        };
        assert_eq!(guarded("perfpal-report", || Ok(empty)), 0);
        assert_eq!(
            guarded("perfpal-report", || Err(PipelineError::Usage("x".into()))),
            1
        );
        assert_eq!(
            guarded("perfpal-report", || Err(PipelineError::Output("x".into()))),
            2
        );
        assert_eq!(
            guarded("perfpal-report", || -> Result<RunSummary, PipelineError> {
                panic!("boom")
            }),
            3,
            "an unwound panic must map to the internal-error exit code"
        );
    }

    #[test]
    fn version_suffix_carries_schema_version() {
        assert!(
            VERSION_SUFFIX.contains("(metrics schema 1.1)"),
            "--version payload must name the metrics schema version"
        );
        assert_eq!(
            format!("perfpal {VERSION_SUFFIX}"),
            format!(
                "perfpal {} (metrics schema {})",
                env!("CARGO_PKG_VERSION"),
                crate::schema::SCHEMA_VERSION
            )
        );
    }

    #[test]
    fn rfc3339_formats_known_instants() {
        assert_eq!(rfc3339_utc(UNIX_EPOCH), "1970-01-01T00:00:00Z");
        // Leap day: 2000-02-29T00:00:00Z == 951782400s.
        assert_eq!(
            rfc3339_utc(UNIX_EPOCH + std::time::Duration::from_secs(951_782_400)),
            "2000-02-29T00:00:00Z"
        );
        // Year boundary rollover: 2026-01-01T00:00:00Z == 1767225600s.
        assert_eq!(
            rfc3339_utc(UNIX_EPOCH + std::time::Duration::from_secs(1_767_225_600)),
            "2026-01-01T00:00:00Z"
        );
        // Intra-day components.
        assert_eq!(
            rfc3339_utc(UNIX_EPOCH + std::time::Duration::from_secs(86_400 + 7_325)),
            "1970-01-02T02:02:05Z"
        );
    }

    #[test]
    fn pre_epoch_clamps_to_epoch_instead_of_panicking() {
        assert_eq!(
            rfc3339_utc(UNIX_EPOCH - std::time::Duration::from_secs(100)),
            "1970-01-01T00:00:00Z"
        );
    }

    #[test]
    fn summary_line_matches_stdout_shape() {
        let s = RunSummary {
            files: 214,
            ok: 210,
            partial: 3,
            failed: 1,
            findings: 57,
            hot: 3,
            warn: 21,
            info: 33,
        };
        assert_eq!(
            s.render("perfpal-report"),
            "scanned 214 files (210 ok, 3 partial, 1 failed) · 57 findings \
             (3 hot, 21 warn, 33 info)\nreport: perfpal-report\n"
        );
    }
}
