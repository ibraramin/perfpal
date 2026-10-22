//! Atlas formatting contract (plan `atlas-build` step 3).
//!
//! Pure, deterministic rendering for every measured value on the Atlas page
//! and its drills (`docs/specs/atlas-metrics.md`). No I/O, no clock, no
//! locale-dependent formatting, no hash-map iteration. `views` owns the
//! median/rank arithmetic; this module owns the emitted label.
//!
//! Honesty rules enforced here: cost is a static heuristic (never a time
//! unit or a runtime claim), structural edges are called structural (never
//! `resolved`), a multiple/share always names its denominator, and every
//! missing value becomes a named stub rather than a blank or `0×`.

use crate::analyzer::{Confidence, Severity};
use crate::reporter::views::{self, CostScale};

/// Stub: no electable entry.
pub const NO_ENTRY: &str = "no entry";
/// Stub: no findings.
pub const NO_FINDINGS: &str = "no findings";
/// Stub: no resolved include dependents.
pub const NO_RESOLVED_DEPENDENTS: &str = "no resolved dependents";
/// Stub: no source files/functions in the scan.
pub const NO_SOURCE_FUNCTIONS: &str = "no source functions recorded";
/// Stub: zero scanned cost (a share has no denominator).
pub const NO_SCANNED_COST: &str = "no scanned static cost recorded";
/// Stub: no production cost in the weight strip.
pub const NO_PRODUCTION_COST: &str = "no production cost recorded";
/// Stub: a loop trip count that is not a known literal.
pub const ESTIMATE_INCOMPLETE: &str = "estimate incomplete";
/// Stub: no include edges at all (unresolved ratio has no denominator).
pub const NO_INCLUDE_EDGES: &str = "no include edges recorded";

/// Insert thousands separators into a decimal string, leaving any sign and
/// fractional part untouched (manual grouping; no locale).
fn group_thousands(raw: &str) -> String {
    let (sign, body) = match raw.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", raw),
    };
    let (integer, fraction) = match body.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (body, None),
    };
    let length = integer.len();
    let mut grouped = String::with_capacity(raw.len() + length / 3 + 1);
    grouped.push_str(sign);
    for (index, character) in integer.chars().enumerate() {
        if index > 0 && (length - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(character);
    }
    if let Some(fraction) = fraction {
        grouped.push('.');
        grouped.push_str(fraction);
    }
    grouped
}

/// Display any non-negative counter with thousands separators.
fn grouped(value: impl std::fmt::Display) -> String {
    group_thousands(&value.to_string())
}

/// `6,030` — integer count with thousands separators.
#[must_use]
pub fn format_count(value: u64) -> String {
    grouped(value)
}

/// `611 static cost units (v2)`.
#[must_use]
pub fn format_cost(cost: u64, scale: CostScale) -> String {
    format!("{} {}", grouped(cost), scale.unit())
}

/// `2.6×` below 10 (one decimal), `12×` / `83,354×` at or above 10.
///
/// The threshold is applied to the value rounded HALF-UP to one decimal, so
/// `9.96` renders `10×` (never `10.0×`) while `9.94` renders `9.9×`.
#[must_use]
pub fn format_multiple(value: f64) -> String {
    let tenths = crate::schema::round_half_up(value, 1);
    if tenths >= 100 {
        let integer = crate::schema::round_half_up(value, 0);
        format!("{}×", grouped(integer))
    } else {
        format!("{}×", grouped(crate::schema::format_fixed_point(value, 1)))
    }
}

/// `top 4%` — exact scan-relative percentile via [`views::top_label`].
#[must_use]
pub fn format_top(rank: usize, population: usize) -> String {
    views::top_label(rank, population)
}

/// `0.2%` / `6.2%` / `36%`; `None` when the denominator is zero.
#[must_use]
pub fn format_percent(numerator: u64, denominator: u64) -> Option<String> {
    if denominator == 0 {
        return None;
    }
    let ratio = (numerator as f64) * 100.0 / (denominator as f64);
    let tenths = crate::schema::round_half_up(ratio, 1);
    if tenths >= 100 {
        let integer = crate::schema::round_half_up(ratio, 0);
        Some(format!("{}%", grouped(integer)))
    } else {
        Some(format!(
            "{}%",
            grouped(crate::schema::format_fixed_point(ratio, 1))
        ))
    }
}

/// `36% of scanned static cost`; a zero total prints [`NO_SCANNED_COST`].
#[must_use]
pub fn format_share(cost: u64, total: u64) -> String {
    match format_percent(cost, total) {
        Some(percent) => format!("{percent} of scanned static cost"),
        None => NO_SCANNED_COST.to_string(),
    }
}

/// `12% of scanned static cost · 7 files` (M-07 component share).
#[must_use]
pub fn format_component_share(cost: u64, total: u64, files: usize) -> String {
    let noun = if files == 1 { "file" } else { "files" };
    let files = grouped(files);
    match format_percent(cost, total) {
        Some(percent) => format!("{percent} of scanned static cost · {files} {noun}"),
        None => format!("{NO_SCANNED_COST} · {files} {noun}"),
    }
}

fn format_cost_anchor(
    cost: u64,
    scale: CostScale,
    median: Option<f64>,
    rank: usize,
    population: usize,
    noun: &str,
) -> String {
    let mut parts = vec![format_cost(cost, scale)];
    if let Some(median) = median
        && median > 0.0
    {
        parts.push(format!(
            "{} median function",
            format_multiple(cost as f64 / median)
        ));
    }
    if population > 0 {
        parts.push(format!("{} of {noun}", format_top(rank, population)));
    }
    parts.join(" · ")
}

/// `file total 611 static cost units (v2) · 87× median function · top 4% of files`.
#[must_use]
pub fn format_file_cost(
    cost: u64,
    scale: CostScale,
    median: Option<f64>,
    rank: usize,
    population: usize,
) -> String {
    format!(
        "file total {}",
        format_cost_anchor(cost, scale, median, rank, population, "files")
    )
}

/// Same anchor as [`format_file_cost`] with the `functions` denominator.
#[must_use]
pub fn format_function_cost(
    cost: u64,
    scale: CostScale,
    median: Option<f64>,
    rank: usize,
    population: usize,
) -> String {
    format_cost_anchor(cost, scale, median, rank, population, "functions")
}

/// `19 of 53 source files reachable (36%)`.
#[must_use]
pub fn format_reachable(reachable: usize, source_files: usize) -> String {
    if source_files == 0 {
        return NO_SOURCE_FUNCTIONS.to_string();
    }
    let percent =
        format_percent(reachable as u64, source_files as u64).unwrap_or_else(|| "0%".to_string());
    format!(
        "{} of {} source files reachable ({percent})",
        grouped(reachable),
        grouped(source_files)
    )
}

/// `22 dependent files · 10% of source files [· 2.0× median dependent count] [· top 2%]`.
#[must_use]
pub fn format_dependents(
    dependents: usize,
    source_files: usize,
    median_multiple: Option<f64>,
    top_pct: Option<usize>,
) -> String {
    if dependents == 0 || source_files == 0 {
        return NO_RESOLVED_DEPENDENTS.to_string();
    }
    let percent =
        format_percent(dependents as u64, source_files as u64).unwrap_or_else(|| "0%".to_string());
    let noun = if dependents == 1 {
        "dependent file"
    } else {
        "dependent files"
    };
    let mut label = format!("{} {noun} · {percent} of source files", grouped(dependents));
    if let Some(median) = median_multiple
        && median > 0.0
    {
        label.push_str(&format!(
            " · {} median dependent count",
            format_multiple(median)
        ));
    }
    if let Some(top_pct) = top_pct {
        label.push_str(&format!(" · top {top_pct}%"));
    }
    label
}

/// `N caller files · M callee files (structural)`.
#[must_use]
pub fn format_call_degree(callers: usize, callees: usize) -> String {
    format!(
        "{} caller files · {} callee files (structural)",
        grouped(callers),
        grouped(callees)
    )
}

/// `1 distinct structural edge` / `56 distinct structural edges`.
#[must_use]
pub fn format_distinct_edges(edges: usize) -> String {
    let noun = if edges == 1 { "edge" } else { "edges" };
    format!("{} distinct structural {noun}", grouped(edges))
}

/// `57 MEDIUM / 88 LOW structural edges`, plus ` / K HIGH` when any exist.
#[must_use]
pub fn format_edge_mix(medium: usize, low: usize, high: usize) -> String {
    let mut label = format!("{} MEDIUM / {} LOW", grouped(medium), grouped(low));
    if high > 0 {
        label.push_str(&format!(" / {} HIGH", grouped(high)));
    }
    label.push_str(" structural edges");
    label
}

/// `20 of 106 reachable files shown · 86 aggregated · 57 of 145 file pairs drawn`.
#[must_use]
pub fn format_caps(
    shown: usize,
    reachable: usize,
    aggregated: usize,
    edges_drawn: usize,
    edge_pairs: usize,
) -> String {
    format!(
        "{} of {} reachable files shown · {} aggregated · {} of {} file pairs drawn",
        grouped(shown),
        grouped(reachable),
        grouped(aggregated),
        grouped(edges_drawn),
        grouped(edge_pairs)
    )
}

/// `non-production (test/bench/other) = 17% of scanned static cost (42 files)`.
#[must_use]
pub fn format_nonproduction(cost: u64, total: u64, files: usize) -> String {
    let files = grouped(files);
    match format_percent(cost, total) {
        Some(percent) => format!(
            "non-production (test/bench/other) = {percent} of scanned static cost ({files} files)"
        ),
        None => format!("non-production (test/bench/other) = {NO_SCANNED_COST} ({files} files)"),
    }
}

/// `parse 100 ok / 116 partial / 0 failed` (M-13).
#[must_use]
pub fn format_parse(ok: u32, partial: u32, failed: u32) -> String {
    format!(
        "parse {} ok / {} partial / {} failed",
        grouped(ok),
        grouped(partial),
        grouped(failed)
    )
}

/// `0/234 unresolved includes` (M-13).
#[must_use]
pub fn format_unresolved_includes(unresolved: usize, total: usize) -> String {
    if total == 0 {
        return NO_INCLUDE_EDGES.to_string();
    }
    format!(
        "{}/{} unresolved includes",
        grouped(unresolved),
        grouped(total)
    )
}

/// `42 files demoted` (M-13).
#[must_use]
pub fn format_demoted_files(files: usize) -> String {
    let noun = if files == 1 { "file" } else { "files" };
    format!("{} {noun} demoted", grouped(files))
}

/// `representative path (static) · 4 steps` (zero steps still prints).
#[must_use]
pub fn format_path_steps(steps: usize) -> String {
    format!("representative path (static) · {} steps", grouped(steps))
}

/// `ENTRY · production main · confidence HIGH · 0 alternatives`.
#[must_use]
pub fn format_entry(policy_label: &str, confidence: &str, alternatives: usize) -> String {
    let confidence = confidence.trim();
    let confidence = if confidence.is_empty() {
        "UNKNOWN".to_string()
    } else {
        confidence.to_ascii_uppercase()
    };
    let noun = if alternatives == 1 {
        "alternative"
    } else {
        "alternatives"
    };
    format!(
        "ENTRY · {policy_label} · confidence {confidence} · {} {noun}",
        grouped(alternatives)
    )
}

/// `depth 3` (ordinal, no anchor).
#[must_use]
pub fn format_depth(depth: u32) -> String {
    format!("depth {depth}")
}

/// `216 files · 3,189 functions · 7,545 findings`.
#[must_use]
pub fn format_scope(files: usize, functions: usize, findings: usize) -> String {
    format!(
        "{} files · {} functions · {} findings",
        grouped(files),
        grouped(functions),
        grouped(findings)
    )
}

/// Uppercase severity label (`HOT` / `WARN` / `INFO`).
#[must_use]
pub fn format_severity(severity: Severity) -> &'static str {
    match severity {
        Severity::Hot => "HOT",
        Severity::Warn => "WARN",
        Severity::Info => "INFO",
    }
}

/// Uppercase confidence label (`HIGH` / `MEDIUM` / `LOW`).
#[must_use]
pub fn format_confidence(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::High => "HIGH",
        Confidence::Medium => "MEDIUM",
        Confidence::Low => "LOW",
    }
}

/// `HOT ×3` — the M-18 node finding annotation.
#[must_use]
pub fn format_finding(severity: Severity, count: usize) -> String {
    format!("{} ×{}", format_severity(severity), grouped(count))
}

/// `HOT · MEDIUM` — severity + confidence text chip.
#[must_use]
pub fn format_severity_confidence(severity: Severity, confidence: Confidence) -> String {
    format!(
        "{} · {}",
        format_severity(severity),
        format_confidence(confidence)
    )
}

/// Known/unknown trip provenance (M-12), both joined in that order.
#[must_use]
pub fn format_trip_provenance(known_trip: Option<u64>, unknown_loops: u32) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(trip) = known_trip {
        parts.push(format!(
            "known trip ×{} — verify it actually runs",
            grouped(trip)
        ));
    }
    if unknown_loops > 0 {
        parts.push(format!(
            "unknown-trip ×{} — {ESTIMATE_INCOMPLETE}",
            grouped(unknown_loops)
        ));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" · "))
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Word-boundary, case-insensitive token scan (so `units` never trips
    /// the `us` unit check).
    fn contains_word(haystack: &str, needle: &str) -> bool {
        let haystack = haystack.to_ascii_lowercase();
        let bytes = haystack.as_bytes();
        let mut start = 0;
        while let Some(offset) = haystack[start..].find(needle) {
            let index = start + offset;
            let before = index.checked_sub(1).map(|i| bytes[i]);
            let after = bytes.get(index + needle.len()).copied();
            let boundary =
                |byte: Option<u8>| byte.is_none_or(|value| !value.is_ascii_alphanumeric());
            if boundary(before) && boundary(after) {
                return true;
            }
            start = index + needle.len().max(1);
            if start >= haystack.len() {
                break;
            }
        }
        false
    }

    #[test]
    fn counts_use_thousands_separators() {
        assert_eq!(format_count(0), "0");
        assert_eq!(format_count(999), "999");
        assert_eq!(format_count(1_000), "1,000");
        assert_eq!(format_count(6_030), "6,030");
        assert_eq!(format_count(7_543), "7,543");
        assert_eq!(format_count(83_354), "83,354");
        assert_eq!(format_count(1_234_567), "1,234,567");
    }

    #[test]
    fn multiples_switch_scale_on_the_rounded_value() {
        assert_eq!(format_multiple(2.6), "2.6×");
        assert_eq!(format_multiple(0.04), "0.0×");
        assert_eq!(format_multiple(9.94), "9.9×");
        assert_eq!(format_multiple(9.96), "10×");
        assert_eq!(format_multiple(9.95), "10×");
        assert_eq!(format_multiple(9.999), "10×");
        assert_eq!(format_multiple(12.0), "12×");
        assert_eq!(format_multiple(12.4), "12×");
        assert_eq!(format_multiple(83_354.0), "83,354×");
    }

    #[test]
    fn percentages_use_the_pinned_bands() {
        assert_eq!(format_percent(1, 500).as_deref(), Some("0.2%"));
        assert_eq!(format_percent(31, 500).as_deref(), Some("6.2%"));
        assert_eq!(format_percent(36, 100).as_deref(), Some("36%"));
        assert_eq!(format_percent(996, 10_000).as_deref(), Some("10%"));
        assert_eq!(format_percent(9999, 100_000).as_deref(), Some("10%"));
        assert_eq!(format_percent(1, 3).as_deref(), Some("33%"));
        assert_eq!(format_percent(1, 1_000).as_deref(), Some("0.1%"));
        assert_eq!(format_percent(0, 53).as_deref(), Some("0.0%"));
        assert_eq!(format_percent(1, 0), None);
        assert_eq!(format_percent(0, 0), None);
    }

    #[test]
    fn top_percentiles_are_integer_and_tie_safe() {
        assert_eq!(format_top(1, 200), "top 1%");
        assert_eq!(format_top(1, 100), "top 1%");
        assert_eq!(format_top(4, 100), "top 4%");
        assert_eq!(format_top(9, 100), "top 9%");
        assert_eq!(format_top(50, 100), "top 50%");
        assert_eq!(format_top(3, 3), "top 100%");
        assert_eq!(format_top(100, 100), "top 100%");
        assert_eq!(format_top(1, 0), "top 1%");
    }

    #[test]
    fn file_and_function_cost_anchors() {
        assert_eq!(
            format_file_cost(611, CostScale::V2, Some(7.0), 4, 100),
            "file total 611 static cost units (v2) · 87× median function · top 4% of files"
        );
        assert_eq!(
            format_function_cost(611, CostScale::V2, Some(7.0), 4, 100),
            "611 static cost units (v2) · 87× median function · top 4% of functions"
        );
        assert_eq!(
            format_file_cost(100, CostScale::V2, None, 1, 10),
            "file total 100 static cost units (v2) · top 10% of files"
        );
        assert_eq!(
            format_file_cost(100, CostScale::V2, Some(0.0), 1, 10),
            "file total 100 static cost units (v2) · top 10% of files"
        );
        assert_eq!(
            format_file_cost(6, CostScale::V1, Some(2.5), 0, 0),
            "file total 6 static cost units (v1) · 2.4× median function"
        );
        assert_eq!(
            format_cost(6_030, CostScale::V1),
            "6,030 static cost units (v1)"
        );
    }

    #[test]
    fn shares_use_the_scanned_cost_denominator_or_a_stub() {
        assert_eq!(format_share(36, 100), "36% of scanned static cost");
        assert_eq!(format_share(1, 500), "0.2% of scanned static cost");
        assert_eq!(format_share(0, 100), "0.0% of scanned static cost");
        assert_eq!(format_share(0, 0), NO_SCANNED_COST);
    }

    #[test]
    fn component_share_has_its_denominator_and_file_count() {
        assert_eq!(
            format_component_share(12, 100, 7),
            "12% of scanned static cost · 7 files"
        );
        assert_eq!(
            format_component_share(1, 500, 1),
            "0.2% of scanned static cost · 1 file"
        );
        assert_eq!(
            format_component_share(0, 0, 7),
            "no scanned static cost recorded · 7 files"
        );
    }

    #[test]
    fn coverage_formatters_print_counts_and_denominators() {
        assert_eq!(
            format_parse(100, 116, 0),
            "parse 100 ok / 116 partial / 0 failed"
        );
        assert_eq!(format_parse(0, 0, 0), "parse 0 ok / 0 partial / 0 failed");
        assert_eq!(
            format_unresolved_includes(0, 234),
            "0/234 unresolved includes"
        );
        assert_eq!(
            format_unresolved_includes(7, 1_234),
            "7/1,234 unresolved includes"
        );
        assert_eq!(format_unresolved_includes(0, 0), NO_INCLUDE_EDGES);
        assert_eq!(format_demoted_files(42), "42 files demoted");
        assert_eq!(format_demoted_files(1), "1 file demoted");
        assert_eq!(format_demoted_files(0), "0 files demoted");
    }

    #[test]
    fn reachable_counts_handle_zero_and_partial_cases() {
        assert_eq!(
            format_reachable(19, 53),
            "19 of 53 source files reachable (36%)"
        );
        assert_eq!(
            format_reachable(0, 53),
            "0 of 53 source files reachable (0.0%)"
        );
        assert_eq!(
            format_reachable(2, 1_000),
            "2 of 1,000 source files reachable (0.2%)"
        );
        assert_eq!(format_reachable(0, 0), NO_SOURCE_FUNCTIONS);
    }

    #[test]
    fn dependents_include_share_and_optional_top() {
        assert_eq!(
            format_dependents(22, 220, Some(2.0), Some(2)),
            "22 dependent files · 10% of source files · 2.0× median dependent count · top 2%"
        );
        assert_eq!(
            format_dependents(5, 10, None, None),
            "5 dependent files · 50% of source files"
        );
        assert_eq!(
            format_dependents(5, 10, Some(1.5), Some(50)),
            "5 dependent files · 50% of source files · 1.5× median dependent count · top 50%"
        );
        assert_eq!(
            format_dependents(1, 10, Some(2.0), Some(10)),
            "1 dependent file · 10% of source files · 2.0× median dependent count · top 10%"
        );
        assert_eq!(
            format_dependents(5, 10, Some(0.0), None),
            "5 dependent files · 50% of source files"
        );
        assert_eq!(
            format_dependents(5, 10, Some(-1.0), None),
            "5 dependent files · 50% of source files"
        );
        assert_eq!(
            format_dependents(0, 10, Some(2.0), Some(1)),
            NO_RESOLVED_DEPENDENTS
        );
        assert_eq!(
            format_dependents(3, 0, Some(2.0), Some(1)),
            NO_RESOLVED_DEPENDENTS
        );
    }

    #[test]
    fn structural_edge_labels() {
        assert_eq!(
            format_call_degree(3, 4),
            "3 caller files · 4 callee files (structural)"
        );
        assert_eq!(format_distinct_edges(56), "56 distinct structural edges");
        assert_eq!(
            format_edge_mix(57, 88, 0),
            "57 MEDIUM / 88 LOW structural edges"
        );
        assert_eq!(
            format_edge_mix(57, 88, 4),
            "57 MEDIUM / 88 LOW / 4 HIGH structural edges"
        );
        assert_eq!(
            format_edge_mix(1_234, 5_678, 90),
            "1,234 MEDIUM / 5,678 LOW / 90 HIGH structural edges"
        );
        // `resolved` is forbidden on the structural set.
        assert!(!format_distinct_edges(56).contains("resolved"));
        assert!(!format_edge_mix(1, 2, 3).contains("resolved"));
        assert!(!format_call_degree(1, 2).contains("resolved"));
    }

    #[test]
    fn caps_print_both_denominators() {
        assert_eq!(
            format_caps(20, 106, 86, 57, 145),
            "20 of 106 reachable files shown · 86 aggregated · 57 of 145 file pairs drawn"
        );
        assert_eq!(
            format_caps(1_234, 5_678, 90, 12, 3_456),
            "1,234 of 5,678 reachable files shown · 90 aggregated · 12 of 3,456 file pairs drawn"
        );
    }

    #[test]
    fn nonproduction_mass_is_labelled_with_its_denominator() {
        assert_eq!(
            format_nonproduction(17, 100, 42),
            "non-production (test/bench/other) = 17% of scanned static cost (42 files)"
        );
        assert_eq!(
            format_nonproduction(50, 100, 3),
            "non-production (test/bench/other) = 50% of scanned static cost (3 files)"
        );
        assert_eq!(
            format_nonproduction(1, 1_000, 42),
            "non-production (test/bench/other) = 0.1% of scanned static cost (42 files)"
        );
        assert_eq!(
            format_nonproduction(0, 0, 42),
            "non-production (test/bench/other) = no scanned static cost recorded (42 files)"
        );
    }

    #[test]
    fn path_and_scope_labels() {
        assert_eq!(
            format_path_steps(0),
            "representative path (static) · 0 steps"
        );
        assert_eq!(
            format_path_steps(4),
            "representative path (static) · 4 steps"
        );
        assert_eq!(
            format_path_steps(1_234),
            "representative path (static) · 1,234 steps"
        );
        assert_eq!(
            format_scope(216, 3_189, 7_545),
            "216 files · 3,189 functions · 7,545 findings"
        );
        assert_eq!(format_depth(3), "depth 3");
    }

    #[test]
    fn entry_badge_shape_and_confidence_normalization() {
        assert_eq!(
            format_entry("production main", "HIGH", 0),
            "ENTRY · production main · confidence HIGH · 0 alternatives"
        );
        assert_eq!(
            format_entry("library root (no production main)", "medium", 2),
            "ENTRY · library root (no production main) · confidence MEDIUM · 2 alternatives"
        );
        assert_eq!(
            format_entry("library root (no production main)", "medium", 1),
            "ENTRY · library root (no production main) · confidence MEDIUM · 1 alternative"
        );
        assert_eq!(
            format_entry("no entry", "", 0),
            "ENTRY · no entry · confidence UNKNOWN · 0 alternatives"
        );
        assert_eq!(
            format_entry("production script (Python __main__)", "high", 1_234),
            "ENTRY · production script (Python __main__) · confidence HIGH · 1,234 alternatives"
        );
    }

    #[test]
    fn severity_confidence_and_finding_annotations() {
        assert_eq!(format_severity(Severity::Hot), "HOT");
        assert_eq!(format_severity(Severity::Warn), "WARN");
        assert_eq!(format_severity(Severity::Info), "INFO");
        assert_eq!(format_confidence(Confidence::High), "HIGH");
        assert_eq!(format_confidence(Confidence::Medium), "MEDIUM");
        assert_eq!(format_confidence(Confidence::Low), "LOW");
        assert_eq!(format_finding(Severity::Hot, 3), "HOT ×3");
        assert_eq!(format_finding(Severity::Warn, 12_345), "WARN ×12,345");
        assert_eq!(
            format_severity_confidence(Severity::Hot, Confidence::Medium),
            "HOT · MEDIUM"
        );
        assert_eq!(
            format_severity_confidence(Severity::Info, Confidence::Low),
            "INFO · LOW"
        );
    }

    #[test]
    fn trip_provenance_known_unknown_both_and_neither() {
        assert_eq!(
            format_trip_provenance(Some(10_000), 0).as_deref(),
            Some("known trip ×10,000 — verify it actually runs")
        );
        assert_eq!(
            format_trip_provenance(None, 3).as_deref(),
            Some("unknown-trip ×3 — estimate incomplete")
        );
        assert_eq!(
            format_trip_provenance(Some(10_000), 2).as_deref(),
            Some(
                "known trip ×10,000 — verify it actually runs · \
                 unknown-trip ×2 — estimate incomplete"
            )
        );
        assert_eq!(format_trip_provenance(None, 0), None);
    }

    #[test]
    fn named_stubs_are_pinned() {
        assert_eq!(NO_ENTRY, "no entry");
        assert_eq!(NO_FINDINGS, "no findings");
        assert_eq!(NO_RESOLVED_DEPENDENTS, "no resolved dependents");
        assert_eq!(NO_SOURCE_FUNCTIONS, "no source functions recorded");
        assert_eq!(NO_SCANNED_COST, "no scanned static cost recorded");
        assert_eq!(NO_PRODUCTION_COST, "no production cost recorded");
        assert_eq!(ESTIMATE_INCOMPLETE, "estimate incomplete");
        assert_eq!(NO_INCLUDE_EDGES, "no include edges recorded");
        assert_eq!(format_unresolved_includes(0, 0), NO_INCLUDE_EDGES);
        assert_eq!(format_reachable(0, 0), NO_SOURCE_FUNCTIONS);
        assert_eq!(format_dependents(0, 0, None, None), NO_RESOLVED_DEPENDENTS);
        assert_eq!(format_share(0, 0), NO_SCANNED_COST);
        assert!(format_nonproduction(0, 0, 0).contains(NO_SCANNED_COST));
    }

    fn representative_outputs() -> Vec<String> {
        vec![
            format_count(83_354),
            format_cost(611, CostScale::V2),
            format_cost(6_030, CostScale::V1),
            format_multiple(2.6),
            format_multiple(83_354.0),
            format_top(4, 100),
            format_percent(1, 500).unwrap(),
            format_share(36, 100),
            format_file_cost(611, CostScale::V2, Some(7.0), 4, 100),
            format_function_cost(611, CostScale::V1, None, 1, 1),
            format_reachable(19, 53),
            format_dependents(22, 220, Some(2.0), Some(2)),
            format_call_degree(3, 4),
            format_distinct_edges(56),
            format_distinct_edges(1),
            format_edge_mix(57, 88, 4),
            format_caps(20, 106, 86, 57, 145),
            format_nonproduction(17, 100, 42),
            format_component_share(12, 100, 7),
            format_parse(100, 116, 0),
            format_unresolved_includes(0, 234),
            format_unresolved_includes(0, 0),
            NO_INCLUDE_EDGES.to_string(),
            format_demoted_files(42),
            format_nonproduction(0, 0, 42),
            format_path_steps(4),
            format_entry("production main", "high", 0),
            format_depth(3),
            format_scope(216, 3_189, 7_545),
            format_finding(Severity::Hot, 3),
            format_severity_confidence(Severity::Warn, Confidence::Low),
            format_trip_provenance(Some(10_000), 2).unwrap(),
            format_trip_provenance(None, 1).unwrap(),
            NO_ENTRY.to_string(),
            NO_FINDINGS.to_string(),
            NO_SOURCE_FUNCTIONS.to_string(),
            NO_SCANNED_COST.to_string(),
            NO_PRODUCTION_COST.to_string(),
            ESTIMATE_INCOMPLETE.to_string(),
        ]
    }

    #[test]
    fn formatters_are_deterministic() {
        assert_eq!(representative_outputs(), representative_outputs());
    }

    #[test]
    fn emitted_labels_never_claim_time_or_runtime() {
        let haystack = representative_outputs().join(" | ");
        for token in ["ms", "µs", "us"] {
            assert!(
                !contains_word(&haystack, token),
                "forbidden unit token `{token}` emitted: {haystack}"
            );
        }
        let lowered = haystack.to_ascii_lowercase();
        for token in ["runtime", "speedup", "latency", "throughput", "hot path"] {
            assert!(
                !lowered.contains(token),
                "forbidden runtime claim `{token}` emitted: {haystack}"
            );
        }
    }
}
