//! Deterministic `files[].path_class` classifier (plan
//! `map-intelligibility` V4, spec §"Field decision").
//!
//! Pure function of the repo-relative POSIX path: case-insensitive, first
//! match wins in the documented order (`test` → `example` → `bench` →
//! `fixture` → `generated` → `vendor` → `production`). The classifier never
//! emits `unknown` — that enum value is reserved for absent/hand-written
//! documents; every computed file gets a concrete class.
//!
//! `test` covers exact test segments (`tests`, `spec`, …) plus common
//! JS/TS naming conventions: a segment that starts with `test-` or ends with
//! `-test`/`-tests`/`-spec`/`-specs` (`test-utils/`, `e2e-tests/`,
//! `foo-specs/`), and basenames shaped `test_*`/`test-*`, `*_test.*`,
//! `*.test.*`, `*_spec.*`, `*.spec.*`, or `*-spec.*` (`fetch.spec.ts`).
//! Production lookalikes (`latest.ts`, `contest.ts`, `attestation/`,
//! `spectrum/`) deliberately stay `production`.
//!
//! `bench` deliberately also covers repo tooling/harness paths
//! (`scripts/`, `tools/`, `ci/`, `.github/`): hyperfine's `scripts/plot_*.py`
//! must not rank as production source. `compat/` is compiled source, not
//! scaffolding, and stays `production` (explicit regression test below).

/// `production` — compiled/interpreted source that ships (the default).
pub(crate) const PATH_PRODUCTION: &str = "production";
/// `test` — unit/integration test files.
pub(crate) const PATH_TEST: &str = "test";
/// `example` — examples/samples.
pub(crate) const PATH_EXAMPLE: &str = "example";
/// `bench` — benchmarks **and** repo tooling/harness paths.
pub(crate) const PATH_BENCH: &str = "bench";
/// `fixture` — test fixtures/test data.
pub(crate) const PATH_FIXTURE: &str = "fixture";
/// `generated` — generated/vendored-build output.
pub(crate) const PATH_GENERATED: &str = "generated";
/// `vendor` — third-party dependencies vendored into the tree.
pub(crate) const PATH_VENDOR: &str = "vendor";

/// Path segments that classify as `test` (any segment, case-insensitive).
const TEST_SEGMENTS: [&str; 5] = ["test", "tests", "testing", "spec", "specs"];
/// Path segments that classify as `example`.
const EXAMPLE_SEGMENTS: [&str; 4] = ["example", "examples", "sample", "samples"];
/// Path segments that classify as `bench` (benchmarks + repo tooling/harness).
const BENCH_SEGMENTS: [&str; 8] = [
    "bench",
    "benches",
    "benchmark",
    "benchmarks",
    "scripts",
    "tools",
    "ci",
    ".github",
];
/// Path segments that classify as `fixture`.
const FIXTURE_SEGMENTS: [&str; 5] = ["fixture", "fixtures", "testdata", "test-data", "test_data"];
/// Path segments that classify as `generated`.
const GENERATED_SEGMENTS: [&str; 6] = ["generated", "gen", "dist", "build", "out", "target"];
/// Path segments that classify as `vendor`.
const VENDOR_SEGMENTS: [&str; 6] = [
    "vendor",
    "third_party",
    "third-party",
    "external",
    "deps",
    "node_modules",
];

/// Classify a repo-relative POSIX path into the closed `path_class` set.
///
/// Never returns `"unknown"`: absence (not this function) means unknown.
#[must_use]
pub(crate) fn classify(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    let segments: Vec<&str> = lower.split('/').filter(|s| !s.is_empty()).collect();
    let basename = segments.last().copied().unwrap_or("");

    if segments.iter().any(|s| is_test_segment(s)) || is_test_basename(basename) {
        return PATH_TEST;
    }
    if segments.iter().any(|s| EXAMPLE_SEGMENTS.contains(s)) {
        return PATH_EXAMPLE;
    }
    if segments.iter().any(|s| BENCH_SEGMENTS.contains(s)) {
        return PATH_BENCH;
    }
    if segments.iter().any(|s| FIXTURE_SEGMENTS.contains(s)) {
        return PATH_FIXTURE;
    }
    if segments.iter().any(|s| GENERATED_SEGMENTS.contains(s)) || is_generated_basename(basename) {
        return PATH_GENERATED;
    }
    if segments.iter().any(|s| VENDOR_SEGMENTS.contains(s)) {
        return PATH_VENDOR;
    }
    PATH_PRODUCTION
}

/// True when a single path segment classifies as `test`: an exact
/// [`TEST_SEGMENTS`] member, a `test-` prefix, or a `-test`/`-tests`/
/// `-spec`/`-specs` suffix. Applied to every segment (including the
/// basename), so `test-utils/`, `e2e-tests/`, and `test-setup.ts` all match
/// while `spectrum/`, `attestation/`, and `contest.ts` do not.
///
/// One carve-out: the `test-` prefix does NOT claim an exact
/// [`FIXTURE_SEGMENTS`] structural name (`test-data`). That directory holds
/// test DATA, not test code, so it keeps the more specific `fixture` class
/// (test still outranks fixture for every other segment).
fn is_test_segment(segment: &str) -> bool {
    TEST_SEGMENTS.contains(&segment)
        || (segment.starts_with("test-") && !FIXTURE_SEGMENTS.contains(&segment))
        || segment.ends_with("-test")
        || segment.ends_with("-tests")
        || segment.ends_with("-spec")
        || segment.ends_with("-specs")
}

/// `test_*` | `test-*` | `*_test.*` | `*.test.*` | `*_spec.*` | `*.spec.*` |
/// `*-spec.*` basename shapes.
fn is_test_basename(basename: &str) -> bool {
    basename.starts_with("test_")
        || basename.starts_with("test-")
        || basename.contains("_test.")
        || basename.contains(".test.")
        || basename.contains("_spec.")
        || basename.contains(".spec.")
        || basename.contains("-spec.")
}

/// `*.generated.*` | `*.pb.*` | `*_pb2.py` basename shapes.
fn is_generated_basename(basename: &str) -> bool {
    basename.contains(".generated.") || basename.contains(".pb.") || basename.ends_with("_pb2.py")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths_classify_as_test() {
        for path in [
            "tests/foo.c",
            "src/test_x.c",
            "src/x_test.rs",
            "tests/helpers/main.rs",
            "spec/thing.rb",
            "src/x.test.ts",
            "src/x_spec.rb",
            "TESTING/Foo.C",
        ] {
            assert_eq!(classify(path), PATH_TEST, "{path}");
        }
    }

    #[test]
    fn js_ts_test_naming_and_production_controls() {
        // New (R-tb-020): JS/TS conventions join the `test` class.
        for path in [
            "packages/core/src/utils/fetch.spec.ts",
            "packages/cli/test-setup.ts",
            "packages/cli/test-utils/src/test-rig.ts",
            "e2e-tests/a.ts",
            "src/spec/thing.ts",
            "src/foo-spec.ts",
            "src/test_helper.ts",
        ] {
            assert_eq!(classify(path), PATH_TEST, "{path}");
        }

        // Production lookalikes must NOT flip.
        for path in [
            "src/latest.ts",
            "src/contest.ts",
            "src/attestation/x.ts",
            "src/spectrum/a.ts",
            "src/specification/a.ts",
            "src/prospect.ts",
        ] {
            assert_eq!(classify(path), PATH_PRODUCTION, "{path}");
        }
    }

    #[test]
    fn example_bench_fixture_paths_classify() {
        assert_eq!(classify("examples/a.c"), PATH_EXAMPLE);
        assert_eq!(classify("sample/a.c"), PATH_EXAMPLE);
        assert_eq!(classify("benches/b.rs"), PATH_BENCH);
        assert_eq!(classify("benchmark/b.rs"), PATH_BENCH);
        // Tooling/harness rule: repo scripts are benchmark-class, not production.
        assert_eq!(classify("scripts/plot.py"), PATH_BENCH);
        assert_eq!(classify("tools/gen.py"), PATH_BENCH);
        assert_eq!(classify("ci/run.sh"), PATH_BENCH);
        assert_eq!(classify(".github/x.yml"), PATH_BENCH);
        assert_eq!(classify("tests/fixtures/a.c"), PATH_TEST);
        assert_eq!(classify("src/testdata/a.c"), PATH_FIXTURE);
        assert_eq!(classify("src/test-data/a.c"), PATH_FIXTURE);
    }

    #[test]
    fn generated_and_vendor_paths_classify() {
        assert_eq!(classify("generated/g.c"), PATH_GENERATED);
        assert_eq!(classify("src/x.generated.rs"), PATH_GENERATED);
        assert_eq!(classify("proto/x.pb.cc"), PATH_GENERATED);
        assert_eq!(classify("src/x_pb2.py"), PATH_GENERATED);
        assert_eq!(classify("build/out.c"), PATH_GENERATED);
        assert_eq!(classify("vendor/lib.c"), PATH_VENDOR);
        assert_eq!(classify("third_party/z.c"), PATH_VENDOR);
        assert_eq!(classify("node_modules/pkg/a.js"), PATH_VENDOR);
    }

    #[test]
    fn production_paths_and_compat_regression() {
        // `compat/` is compiled source, not scaffolding — stays production.
        assert_eq!(classify("compat/c.c"), PATH_PRODUCTION);
        assert_eq!(classify("src/main.rs"), PATH_PRODUCTION);
        assert_eq!(classify("lib/curl.c"), PATH_PRODUCTION);
        assert_eq!(classify("README.md"), PATH_PRODUCTION);
    }

    #[test]
    fn classifier_is_case_insensitive_and_first_match_wins() {
        assert_eq!(classify("Tests/Foo.C"), PATH_TEST);
        assert_eq!(classify("Vendor/Lib.C"), PATH_VENDOR);
        // A test segment outranks a vendor/bench segment later in the path.
        assert_eq!(classify("vendor/tests/lib.c"), PATH_TEST);
        // Empty/degenerate paths stay production, never unknown.
        assert_eq!(classify(""), PATH_PRODUCTION);
    }
}
