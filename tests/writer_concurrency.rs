//! m7 concurrent-run staging guard (writer level): foreign-pid staging
//! directories belong to a possibly still-running perfpal and must survive
//! a concurrent successful run until the grace period lapses; own-pid
//! leftovers are always swept immediately.
//!
//! Thread-safety note: cargo runs this binary's tests in parallel threads
//! and `PERFPAL_STALE_GRACE_SECS` is process-global, so every env write/read
//! AND every `write_report` call here happens while a guard holds the shared
//! env-mutex from tests/common (a test may hold successive guards; the
//! between-guards window touches nothing env-dependent).
//! The e2e/pipeline test binaries are unaffected: they never touch the
//! variable and only ever see own-pid staging dirs inside their own unique
//! temp parents (the own-pid always-sweep rule is grace-independent).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use perfpal::writer::{Artifact, write_report};

mod common;

fn temp_dir(tag: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "perfpal-writer-conc-{tag}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn artifacts() -> Vec<Artifact> {
    vec![
        Artifact::text("metrics.json", "{\n  \"schema_version\": \"1.0\"\n}"),
        Artifact::text("metrics.schema.v1.0.json", "{ \"schema\": true }"),
    ]
}

/// A pid that is definitely not ours for this test process.
const FOREIGN_PID: u32 = 4_000_000_000;

#[test]
fn foreign_fresh_staging_dir_survives_then_swept_at_zero_grace() {
    let root = temp_dir("foreign");
    let target = root.join("report");

    // A foreign-pid staging dir created "just now" — a concurrently running
    // perfpal mid-write, from this directory's point of view.
    let foreign = root.join(format!(".perfpal-report.tmp-{FOREIGN_PID}"));
    fs::create_dir_all(foreign.join("charts")).expect("create foreign staging dir");

    // Default grace (env unset): the foreign dir MUST survive the run.
    let _default = common::set_stale_grace_env(None);
    write_report(&target, &artifacts(), false).expect("first write succeeds");
    assert!(
        foreign.exists(),
        "fresh foreign-pid staging dir must survive the default grace — \
         a concurrent perfpal must not be nuked mid-write"
    );
    drop(_default);

    // Grace 0: the foreign dir is older than grace ⇒ swept by the next run.
    {
        let _zero = common::set_stale_grace_env(Some("0"));
        write_report(&target, &artifacts(), false).expect("second write succeeds");
        assert!(
            !foreign.exists(),
            "grace-0 sweep must take the foreign staging dir"
        );
    }
    cleanup(&root);
}

#[test]
fn own_pid_leftover_staging_dir_is_swept_immediately() {
    let root = temp_dir("own");
    let target = root.join("report");

    // Own-pid leftover = crashed run of THIS process — always stale,
    // regardless of the grace value (even at an explicit huge grace).
    let own = root.join(format!(".perfpal-report.tmp-{}", std::process::id()));
    fs::create_dir_all(own.join("junk")).expect("create own-pid leftover");

    let _huge = common::set_stale_grace_env(Some("86400"));
    write_report(&target, &artifacts(), false).expect("write succeeds");
    assert!(
        !own.exists(),
        "own-pid leftovers must be swept immediately, even with a huge grace"
    );
    drop(_huge);
    cleanup(&root);
}

fn cleanup(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}
