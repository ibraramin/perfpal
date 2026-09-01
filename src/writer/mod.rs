//! `writer` — atomic output management (modules.md §3.8; output-contract §2–3).
//!
//! Consumes: staged artifact bytes as a generic [`Artifact`] map — metrics.json,
//! bundled schema copy today; report.md and charts/*.svg join in P4 WITHOUT
//! redesign (they are just more entries). Produces: the report directory per
//! output-contract §2.
//!
//! Responsibilities:
//! - Build in a temporary SIBLING directory of the target
//!   (`.perfpal-report.tmp-<pid>`); temp names may vary freely — they never
//!   survive a run, and FINAL artifact names carry no timestamps/counters
//!   (output-contract §3 stability rule).
//! - Swap via rename-aside-then-swap: POSIX rename(2) cannot overwrite a
//!   non-empty directory, so an existing target is renamed aside first, the
//!   new directory renamed into place, then the aside removed. Any failure
//!   BEFORE the swap aborts with the previous complete report intact; a
//!   failure DURING the swap rolls the aside back best-effort
//!   (output-contract §2 atomic regeneration).
//! - Safety guard: refuse replacing an existing directory that lacks
//!   `metrics.json` unless forced — user data is never silently destroyed
//!   ([`WriterError::SafetyGuard`], cli maps to exit 2).
//! - Stale-temp cleanup: temp/aside directories from crashed runs are swept
//!   on the next successful run (output-contract §2). Concurrency rule (m7):
//!   staging dirs whose pid suffix is OUR process id are crashed-run
//!   leftovers and are ALWAYS swept; foreign-pid staging dirs belong to a
//!   possibly still-running perfpal and are swept only after the grace
//!   period (`PERFPAL_STALE_GRACE_SECS`, default 3600 s) has elapsed since
//!   their last modification — a concurrent run is never nuked mid-write.
//!   Entries whose metadata cannot be read are skipped with a warning
//!   rather than treated as stale: destroying a live directory on a
//!   metadata hiccup is worse than leaving one extra temp dir behind.
//! - Encoding enforcement: every text artifact UTF-8, LF line endings,
//!   trailing newline ([`WriterError::Encoding`] otherwise).
//! - `charts/` created empty from day one (output-contract §3: the reporter
//!   always creates it, possibly empty on trivial scans).
//!
//! Failure behavior: any upstream failure aborts before the swap — the
//! previous complete report survives intact; interrupted runs never leave
//! half-written reports.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// One staged artifact: bytes plus their path INSIDE the report directory
/// (`"metrics.json"`, `"metrics.schema.v1.0.json"`, `"charts/x.svg"` in P4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub rel_path: String,
    pub bytes: Vec<u8>,
}

impl Artifact {
    /// A text artifact from UTF-8 content.
    #[must_use]
    pub fn text(rel_path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            rel_path: rel_path.into(),
            bytes: content.into().into_bytes(),
        }
    }
}

/// Writer failures. ALL variants are output errors (cli exit 2, modules.md
/// §8.2) — nothing here is a usage error or an internal bug.
#[derive(Debug, thiserror::Error)]
pub enum WriterError {
    /// Safety-guard refusal (output-contract §2): target exists without
    /// metrics.json and `--force` was not given.
    #[error(
        "refusing to replace '{dir}': directory exists but contains no \
         metrics.json — pass --force to replace it"
    )]
    SafetyGuard { dir: PathBuf },
    /// Text-encoding contract violation (output-contract §2 encoding rules).
    #[error("artifact '{artifact}' violates encoding contract: {reason}")]
    Encoding { artifact: String, reason: String },
    #[error("{context}: {source}")]
    Io {
        context: String,
        source: std::io::Error,
    },
}

/// Prefix shared by all temporary directories this writer creates. Stale
/// entries matching either prefix are swept on the next successful run.
const TEMP_PREFIX: &str = ".perfpal-report.tmp-";
const ASIDE_PREFIX: &str = ".perfpal-report.aside-";

/// Environment knob for the concurrent-run sweep grace period (m7): a
/// foreign-pid staging dir is swept only once it is at least this many
/// seconds old. Absent or unparseable value ⇒ [`DEFAULT_STALE_GRACE_SECS`].
const STALE_GRACE_ENV: &str = "PERFPAL_STALE_GRACE_SECS";
/// Default grace: one hour. A concurrently running perfpal that has been
/// writing within the last hour is never treated as a crashed leftover.
const DEFAULT_STALE_GRACE_SECS: u64 = 3600;

/// Grace period for the stale sweep, from the environment.
fn stale_grace() -> Duration {
    Duration::from_secs(
        std::env::var(STALE_GRACE_ENV)
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_STALE_GRACE_SECS),
    )
}

/// Write the staged artifacts into `target` atomically (see module docs).
///
/// # Crash window (inherent limitation)
///
/// The swap is `rename(target → aside)` followed by `rename(temp → target)`.
/// POSIX gives no atomic two-directory swap, so a process crash (or power
/// loss) BETWEEN the two renames leaves the previous report parked at the
/// aside path and no report at `target` until the next successful run
/// regenerates it. This window is inherent to rename-aside-then-swap and
/// accepted for v1; every HANDLED failure path rolls the aside back (below),
/// and stale aside/temp directories from crashed runs are swept on the next
/// successful run ([`sweep_stale`]).
///
/// # Errors
/// [`WriterError`] — safety-guard refusal, encoding violation, or I/O
/// failure. On any error the previous report (if any) is left intact.
pub fn write_report(target: &Path, artifacts: &[Artifact], force: bool) -> Result<(), WriterError> {
    let normalized = normalize_all(artifacts)?;
    let parent = parent_of(target);
    let pid = std::process::id();

    // Safety guard BEFORE any filesystem mutation (output-contract §2).
    if target.exists() {
        if !target.is_dir() {
            return Err(io_err(
                format!(
                    "output target '{}' exists and is not a directory",
                    target.display()
                ),
                std::io::ErrorKind::AlreadyExists,
            ));
        }
        if !target.join("metrics.json").is_file() && !force {
            return Err(WriterError::SafetyGuard {
                dir: target.to_path_buf(),
            });
        }
    }

    let temp = parent.join(format!("{TEMP_PREFIX}{pid}"));
    let aside = parent.join(format!("{ASIDE_PREFIX}{pid}"));

    // Fresh staging area (a leftover with our exact pid name would be stale).
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp)
        .map_err(|e| io_ctx(format!("creating staging dir {}", temp.display()), e))?;

    if let Err(err) = stage(&temp, &normalized) {
        let _ = fs::remove_dir_all(&temp);
        return Err(err);
    }

    // charts/ always exists, possibly empty (output-contract §3 chart-set
    // rules); P4 fills it via additional Artifact entries.
    if let Err(err) = fs::create_dir_all(temp.join("charts")) {
        let _ = fs::remove_dir_all(&temp);
        return Err(io_ctx("creating charts/ directory".to_string(), err));
    }

    // rename-aside-then-swap. Crash window: a crash between the two renames
    // loses the previous report (parked at `aside`) until the next run —
    // see the doc comment on [`write_report`]. Handled failures roll back.
    // An own-pid aside leftover from an earlier crash in this same process
    // (or after pid reuse) would block `rename(target → aside)`, so it is
    // removed first — own-pid leftovers are always stale (m7).
    let aside_taken = if target.exists() {
        let _ = fs::remove_dir_all(&aside);
        if let Err(e) = fs::rename(target, &aside) {
            let _ = fs::remove_dir_all(&temp);
            return Err(io_ctx(format!("setting aside old {}", target.display()), e));
        }
        true
    } else {
        false
    };

    if let Err(e) = fs::rename(&temp, target) {
        // Roll back so the previous report stays in place.
        if aside_taken {
            let _ = fs::rename(&aside, target);
        }
        let _ = fs::remove_dir_all(&temp);
        return Err(io_ctx(
            format!("activating new report at {}", target.display()),
            e,
        ));
    }

    if aside_taken && let Err(e) = fs::remove_dir_all(&aside) {
        return Err(io_ctx(format!("removing aside dir {}", aside.display()), e));
    }

    sweep_stale(&parent);
    Ok(())
}

/// Enforce the text contract on every artifact up front — before any
/// filesystem mutation, so an encoding violation can never leave a partial
/// staging area behind (UTF-8, LF, trailing newline; output-contract §2).
fn normalize_all(artifacts: &[Artifact]) -> Result<Vec<Artifact>, WriterError> {
    artifacts
        .iter()
        .map(|a| {
            let text = std::str::from_utf8(&a.bytes).map_err(|_| WriterError::Encoding {
                artifact: a.rel_path.clone(),
                reason: "bytes are not valid UTF-8".to_string(),
            })?;
            let lf = text.replace("\r\n", "\n");
            let mut out = lf.into_bytes();
            if out.last() != Some(&b'\n') {
                out.push(b'\n');
            }
            Ok(Artifact {
                rel_path: a.rel_path.clone(),
                bytes: out,
            })
        })
        .collect()
}

/// Write all artifacts into the staging directory (parents created for
/// nested rel paths). Rejects absolute or parent-escaping paths — a staged
/// artifact must land inside the report directory.
fn stage(temp: &Path, artifacts: &[Artifact]) -> Result<(), WriterError> {
    for artifact in artifacts {
        let rel = Path::new(&artifact.rel_path);
        if rel.is_absolute()
            || rel
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(io_err(
                format!("artifact path escapes report dir: '{}'", artifact.rel_path),
                std::io::ErrorKind::InvalidInput,
            ));
        }
        let dest = temp.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| io_ctx(format!("creating {}", parent.display()), e))?;
        }
        fs::write(&dest, &artifact.bytes)
            .map_err(|e| io_ctx(format!("writing {}", dest.display()), e))?;
    }
    Ok(())
}

/// Remove temp/aside directories left by crashed runs (next-successful-run
/// cleanup, output-contract §2), with the m7 concurrent-run guard:
///
/// - own-pid staging dirs are crashed leftovers of THIS process — always
///   swept immediately;
/// - foreign-pid staging dirs belong to a possibly still-running perfpal —
///   swept only when `now - modified >= grace` (grace from
///   `PERFPAL_STALE_GRACE_SECS`, default 3600 s), so a concurrently running
///   process is never nuked mid-write;
/// - entries whose metadata (existence or mtime) cannot be read are SKIPPED
///   with a stderr warning, never treated as stale — a metadata hiccup must
///   not destroy a live directory.
///
/// Best-effort overall: a stuck entry is reported to stderr but never fails
/// the run that just succeeded.
fn sweep_stale(parent: &Path) {
    let now = SystemTime::now();
    let self_pid = std::process::id();
    let grace = stale_grace();
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !(name.starts_with(TEMP_PREFIX) || name.starts_with(ASIDE_PREFIX)) {
            continue;
        }
        let modified = match entry.metadata().and_then(|meta| meta.modified()) {
            Ok(mtime) => mtime,
            Err(err) => {
                eprintln!(
                    "perfpal: warning: skipping staging dir {} in stale sweep \
                     (metadata unavailable: {err})",
                    entry.path().display()
                );
                continue;
            }
        };
        if should_sweep(name, modified, now, self_pid, grace)
            && let Err(err) = fs::remove_dir_all(entry.path())
        {
            eprintln!(
                "perfpal: warning: could not remove stale temp dir {}: {err}",
                entry.path().display()
            );
        }
    }
}

/// Pure sweep decision for one staging-dir entry (m7 concurrent-run guard).
///
/// Semantics:
/// - `name` must carry a staging prefix; anything else is never swept.
/// - a suffix that parses to `self_pid` is an OWN leftover → ALWAYS swept.
/// - any other suffix — including unparseable/non-numeric ones — is treated
///   as FOREIGN → swept iff `now - modified >= grace`. A `modified` in the
///   future (clock skew) counts as age zero, i.e. fresh.
///
/// Metadata READ failures are not decided here: [`sweep_stale`] skips those
/// entries with a warning before ever calling this function.
fn should_sweep(
    name: &str,
    modified: SystemTime,
    now: SystemTime,
    self_pid: u32,
    grace: Duration,
) -> bool {
    let suffix = if let Some(s) = name.strip_prefix(TEMP_PREFIX) {
        s
    } else if let Some(s) = name.strip_prefix(ASIDE_PREFIX) {
        s
    } else {
        return false;
    };
    if suffix.parse::<u32>() == Ok(self_pid) {
        return true;
    }
    now.duration_since(modified).unwrap_or_default() >= grace
}

fn parent_of(target: &Path) -> PathBuf {
    match target.parent() {
        Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

fn io_ctx(context: String, source: std::io::Error) -> WriterError {
    WriterError::Io { context, source }
}

fn io_err(message: String, kind: std::io::ErrorKind) -> WriterError {
    WriterError::Io {
        context: message.clone(),
        source: std::io::Error::new(kind, message),
    }
}

#[cfg(test)]
mod tests {
    //! Golden-directory behavior locks (modules.md §7 test obligations):
    //! swap completeness, safety guard, stale cleanup, crash-intact previous
    //! report, encoding enforcement.

    use super::*;

    fn tmp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "perfpal-writer-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
        ));
        fs::create_dir_all(&dir).expect("create temp root");
        dir
    }

    fn artifacts() -> Vec<Artifact> {
        vec![
            Artifact::text("metrics.json", "{\n  \"schema_version\": \"1.0\"\n}"),
            Artifact::text("metrics.schema.v1.0.json", "{ \"schema\": true }"),
        ]
    }

    #[test]
    fn write_creates_report_with_layout_and_charts_dir() {
        let root = tmp_root("fresh");
        let target = root.join("perfpal-report");

        write_report(&target, &artifacts(), false).expect("first write succeeds");

        assert!(target.join("metrics.json").is_file());
        assert!(target.join("metrics.schema.v1.0.json").is_file());
        assert!(
            target.join("charts").is_dir(),
            "charts/ always created (possibly empty)"
        );
        assert_eq!(fs::read_dir(target.join("charts")).unwrap().count(), 0);
    }

    #[test]
    fn swap_replaces_old_report_completely_no_stale_files() {
        let root = tmp_root("swap");
        let target = root.join("report");
        fs::create_dir_all(target.join("charts")).unwrap();
        fs::write(target.join("metrics.json"), "old\n").unwrap();
        fs::write(target.join("stale-artifact.txt"), "must not survive\n").unwrap();

        write_report(&target, &artifacts(), false).expect("swap succeeds");

        assert!(
            !target.join("stale-artifact.txt").exists(),
            "stale file survived swap"
        );
        assert_eq!(
            fs::read_to_string(target.join("metrics.json")).unwrap(),
            "{\n  \"schema_version\": \"1.0\"\n}\n"
        );
        // No temp/aside leftovers in the sibling namespace.
        let siblings: Vec<String> = fs::read_dir(&root)
            .unwrap()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        assert!(
            siblings
                .iter()
                .all(|n| !n.starts_with(TEMP_PREFIX) && !n.starts_with(ASIDE_PREFIX)),
            "leftover temp dirs: {siblings:?}"
        );
    }

    #[test]
    fn safety_guard_refuses_without_force_and_allows_with_force() {
        let root = tmp_root("guard");
        let target = root.join("not-a-report");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("user-data.txt"), "precious\n").unwrap();

        let refusal = write_report(&target, &artifacts(), false)
            .expect_err("guard must refuse without force");
        assert!(
            matches!(refusal, WriterError::SafetyGuard { .. }),
            "expected SafetyGuard, got {refusal:?}"
        );
        assert_eq!(
            fs::read_to_string(target.join("user-data.txt")).unwrap(),
            "precious\n",
            "guarded directory must be untouched"
        );

        write_report(&target, &artifacts(), true).expect("force overrides the guard");
        assert!(!target.join("user-data.txt").exists());
        assert!(target.join("metrics.json").is_file());
    }

    #[test]
    fn failure_before_swap_leaves_previous_report_intact() {
        let root = tmp_root("abort");
        let target = root.join("report");
        write_report(&target, &artifacts(), false).expect("seed a good report");
        let good = fs::read_to_string(target.join("metrics.json")).unwrap();

        // Non-UTF-8 artifact ⇒ encoding violation during normalization,
        // BEFORE any filesystem mutation.
        let bad = [Artifact {
            rel_path: "metrics.json".to_string(),
            bytes: vec![0xff, 0xfe, b'{', b'\n'],
        }];
        let err = write_report(&target, &bad, true).expect_err("encoding violation");
        assert!(matches!(err, WriterError::Encoding { .. }), "got {err:?}");

        assert_eq!(
            fs::read_to_string(target.join("metrics.json")).unwrap(),
            good,
            "previous report must survive an aborted run"
        );
        let siblings: Vec<String> = fs::read_dir(&root)
            .unwrap()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        assert!(
            siblings.iter().all(|n| !n.starts_with(TEMP_PREFIX)),
            "aborted staging dir must be cleaned: {siblings:?}"
        );
    }

    #[test]
    fn stale_temp_dirs_are_swept_on_next_successful_run() {
        // Env-free by design (P6 review fix): mutating
        // PERFPAL_STALE_GRACE_SECS here would race sibling tests' concurrent
        // write_report → stale_grace() env reads (setenv/getenv is UB under
        // concurrency on glibc). Own-pid rules are grace-independent, and the
        // grace-0 foreign sweep is locked end-to-end in
        // tests/writer_concurrency.rs — nothing is lost by dropping the
        // mutation.
        let root = tmp_root("sweep");
        let target = root.join("report");

        // Own-pid leftover: ALWAYS swept, regardless of grace (m7).
        let own_temp = root.join(format!("{TEMP_PREFIX}{}", std::process::id()));
        fs::create_dir_all(own_temp.join("junk")).unwrap();

        // Foreign-pid leftovers with fresh mtimes: SURVIVE under the default
        // grace — a concurrently running perfpal is not nuked mid-write.
        let foreign_temp = root.join(format!("{TEMP_PREFIX}999999"));
        let foreign_aside = root.join(format!("{ASIDE_PREFIX}999999"));
        fs::create_dir_all(foreign_temp.join("junk")).unwrap();
        fs::create_dir_all(&foreign_aside).unwrap();

        write_report(&target, &artifacts(), false).expect("write succeeds");

        assert!(!own_temp.exists(), "own-pid leftover must be swept at once");
        assert!(
            foreign_temp.exists(),
            "fresh foreign-pid temp must survive the default grace"
        );
        assert!(
            foreign_aside.exists(),
            "fresh foreign-pid aside must survive the default grace"
        );
    }

    #[test]
    fn should_sweep_decision_matrix() {
        let now = SystemTime::now();
        let grace = Duration::from_secs(3600);
        let pid = std::process::id();
        let fresh = now - Duration::from_secs(1);
        let old = now - Duration::from_secs(2 * 3600);
        let exact = now - grace;

        // Own pid: ALWAYS swept, even fresh, either prefix.
        assert!(should_sweep(
            &format!("{TEMP_PREFIX}{pid}"),
            fresh,
            now,
            pid,
            grace
        ));
        assert!(should_sweep(
            &format!("{ASIDE_PREFIX}{pid}"),
            fresh,
            now,
            pid,
            grace
        ));
        // Foreign pid: fresh survives, old is swept.
        assert!(!should_sweep(
            &format!("{TEMP_PREFIX}12345"),
            fresh,
            now,
            pid,
            grace
        ));
        assert!(should_sweep(
            &format!("{TEMP_PREFIX}12345"),
            old,
            now,
            pid,
            grace
        ));
        // Unparseable suffix: treated as foreign (fresh ⇒ survive).
        assert!(!should_sweep(
            &format!("{TEMP_PREFIX}junk"),
            fresh,
            now,
            pid,
            grace
        ));
        assert!(should_sweep(
            &format!("{ASIDE_PREFIX}junk"),
            old,
            now,
            pid,
            grace
        ));
        // Grace boundary: age exactly == grace ⇒ swept (>=, not >).
        assert!(should_sweep(
            &format!("{TEMP_PREFIX}12345"),
            exact,
            now,
            pid,
            grace
        ));
        // Zero grace sweeps every foreign dir regardless of mtime.
        assert!(should_sweep(
            &format!("{TEMP_PREFIX}12345"),
            fresh,
            now,
            pid,
            Duration::ZERO
        ));
        // Clock skew (mtime in the future) counts as age zero ⇒ survive.
        assert!(!should_sweep(
            &format!("{TEMP_PREFIX}12345"),
            now + Duration::from_secs(60),
            now,
            pid,
            grace
        ));
        // Non-staging names are never swept by the decision function.
        assert!(!should_sweep("perfpal-report", old, now, pid, grace));
        assert!(!should_sweep(
            ".perfpal-report.other-12345",
            old,
            now,
            pid,
            grace
        ));
    }

    #[test]
    fn regeneration_over_valid_report_swaps_cleanly() {
        // Golden-flow swap lock (modules.md §7): a pre-existing VALID report
        // is replaced in place, with no aside dirs left behind and staging
        // dirs swept. Mid-failure rollback stays documented-only (no fault
        // injection here).
        let root = tmp_root("regen");
        let target = root.join("report");
        write_report(&target, &artifacts(), false).expect("seed a valid report");

        // Own-pid crashed staging leftover from a previous run.
        let leftover = root.join(format!("{TEMP_PREFIX}{}", std::process::id()));
        fs::create_dir_all(leftover.join("junk")).unwrap();

        let v2 = vec![Artifact::text("metrics.json", "{\n  \"v\": 2\n}")];
        write_report(&target, &v2, false).expect("rewrite succeeds");

        assert_eq!(
            fs::read_to_string(target.join("metrics.json")).unwrap(),
            "{\n  \"v\": 2\n}\n",
            "target must hold the NEW content after the swap"
        );
        assert!(!leftover.exists(), "own-pid staging leftover must be swept");
        let siblings: Vec<String> = fs::read_dir(&root)
            .unwrap()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        assert!(
            siblings
                .iter()
                .all(|n| !n.starts_with(TEMP_PREFIX) && !n.starts_with(ASIDE_PREFIX)),
            "no aside/temp dirs may remain after a clean swap: {siblings:?}"
        );
    }

    #[test]
    fn encoding_contract_lf_and_trailing_newline_enforced() {
        let root = tmp_root("enc");
        let target = root.join("report");
        let crlf = [Artifact::text("metrics.json", "{\"a\": 1}\r\n{\"b\": 2}")];

        write_report(&target, &crlf, false).expect("write succeeds");
        let raw = fs::read(target.join("metrics.json")).unwrap();
        assert!(!raw.windows(2).any(|w| w == b"\r\n"), "CRLF leaked through");
        assert_eq!(raw.last(), Some(&b'\n'), "trailing newline missing");
        let text = String::from_utf8(raw).expect("valid UTF-8");
        assert!(text.ends_with("{\"b\": 2}\n"));
    }

    #[test]
    fn existing_file_at_target_is_an_output_error() {
        let root = tmp_root("file");
        let target = root.join("occupied");
        fs::write(&target, "i am a file\n").unwrap();

        let err = write_report(&target, &artifacts(), true).expect_err("file target must fail");
        assert!(matches!(err, WriterError::Io { .. }), "got {err:?}");
        assert_eq!(fs::read_to_string(&target).unwrap(), "i am a file\n");
    }

    #[test]
    fn nested_artifact_paths_land_in_subdirectories() {
        let root = tmp_root("nested");
        let target = root.join("report");
        let arts = vec![
            Artifact::text("metrics.json", "{}"),
            Artifact::text("charts/include-graph.svg", "<svg/>"),
        ];
        write_report(&target, &arts, false).expect("write succeeds");
        assert_eq!(
            fs::read_to_string(target.join("charts/include-graph.svg")).unwrap(),
            "<svg/>\n"
        );
    }
}
