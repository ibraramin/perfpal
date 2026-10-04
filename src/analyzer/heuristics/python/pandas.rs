//! Python pandas heuristics (plan `python-backend` W3):
//! PY-PANDAS-001/002 (Python-native additions).
//!
//! Matching is name-based (receiver-root gated), so a non-pandas object with
//! the same method names can fire — an accepted FP class documented in the
//! catalog. Frequency on the W0 corpora was near-zero (4 and 0 hits); these
//! ship Info with that caveat.

use crate::analyzer::finding::{PY_PANDAS_001_RULE_ID, PY_PANDAS_002_RULE_ID};
use crate::analyzer::{Confidence, Finding, Severity};
use crate::ir::{BindingKind, CalleeKind};
use crate::scanner::FileUnit;

use super::{binding_kinds, sort_findings};

/// Row-wise iteration methods (`iterrows`/`itertuples`).
const ROWWISE_METHODS: [&str; 2] = ["iterrows", "itertuples"];

/// Receiver roots that are the pandas module itself.
const PANDAS_ROOTS: [&str; 2] = ["pd", "pandas"];

/// PY-PANDAS-001 — Row-wise pandas iteration (Python-native), Info.
///
/// `iterrows`/`itertuples` (receiver-root gated) or `apply(axis=1)`: each row
/// is materialised as a Python object (a `Series` with per-row dtype coercion
/// for `iterrows`), so the pure-Python per-row work dominates. Evidence
/// `callee`, `receiver_root`, `loop_depth`. Info/MEDIUM.
pub(crate) fn python_pandas_001(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            for call in &func.calls {
                if call.callee_kind != CalleeKind::Direct {
                    continue;
                }
                let Some(root) = call.receiver_root.as_deref() else {
                    continue;
                };
                let row_method = ROWWISE_METHODS.contains(&call.callee_name.as_str());
                let axis_one_apply = call.callee_name == "apply"
                    && call.args.iter().any(|a| {
                        a.name.as_deref() == Some("axis") && a.value.as_deref() == Some("1")
                    });
                if !(row_method || axis_one_apply) {
                    continue;
                }
                let callee = call
                    .callee_path
                    .clone()
                    .unwrap_or_else(|| call.callee_name.clone());
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_PANDAS_001_RULE_ID.to_string(),
                    title: "Row-wise pandas iteration".to_string(),
                    message: format!(
                        "'{callee}' iterates row by row — pandas materialises a \
                         Python object per row (`iterrows` builds a `Series` \
                         with dtype coercion), so the per-row overhead dwarfs \
                         the vectorised work; column arithmetic or `groupby`/\
                         `itertuples`-free vectorisation is a candidate \
                         optimization"
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), callee),
                        ("receiver_root".to_string(), root.to_string()),
                        (
                            "loop_depth".to_string(),
                            call.meta.in_loop_depth.to_string(),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    macro_suspect: false,
                    error_node_overlap: false,
                });
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

/// PY-PANDAS-002 — `concat`/`DataFrame.append` in a loop (Python-native),
/// Info.
///
/// `pd.concat([...])` or `df.append(row)` at `in_loop_depth ≥ 1` copies the
/// whole frame every iteration (O(n²)). The `.append` name is syntactically
/// indistinguishable from `list.append`, so the receiver must be proven a
/// DataFrame by the binding lattice; `concat` is gated on the `pd`/`pandas`
/// receiver root or a DataFrame receiver. Evidence `callee`, `receiver_root`,
/// `loop_depth`. Info/MEDIUM.
pub(crate) fn python_pandas_002(units: &[FileUnit]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for unit in units {
        for func in &unit.functions {
            let kinds = binding_kinds(func);
            for call in &func.calls {
                if call.meta.in_loop_depth == 0 || call.callee_kind != CalleeKind::Direct {
                    continue;
                }
                let root = call.receiver_root.as_deref();
                let root_is_df = root.is_some_and(|r| {
                    kinds.get(r) == Some(&BindingKind::Dataframe) || PANDAS_ROOTS.contains(&r)
                });
                let is_concat = call.callee_name == "concat" && root_is_df;
                let is_append = call.callee_name == "append"
                    && root.is_some_and(|r| kinds.get(r) == Some(&BindingKind::Dataframe));
                if !(is_concat || is_append) {
                    continue;
                }
                let callee = call
                    .callee_path
                    .clone()
                    .unwrap_or_else(|| call.callee_name.clone());
                findings.push(Finding {
                    threshold: None,
                    rule_id: PY_PANDAS_002_RULE_ID.to_string(),
                    title: "`concat`/`append` in loop".to_string(),
                    message: format!(
                        "'{callee}' rebuilds the whole DataFrame every iteration \
                         at loop depth {} — pandas copies the full frame on each \
                         `concat`/`append` (O(n²)); collecting rows in a list \
                         and concatenating once after the loop is a candidate \
                         optimization",
                        call.meta.in_loop_depth
                    ),
                    severity: Severity::Info,
                    confidence: Confidence::Medium,
                    resolver_tier: call.meta.resolver_tier,
                    span: call.meta.span,
                    path: unit.file.path.clone(),
                    evidence: [
                        ("callee".to_string(), callee),
                        ("receiver_root".to_string(), root.unwrap_or("").to_string()),
                        (
                            "loop_depth".to_string(),
                            call.meta.in_loop_depth.to_string(),
                        ),
                    ]
                    .into_iter()
                    .collect(),
                    macro_suspect: false,
                    error_node_overlap: false,
                });
            }
        }
    }
    sort_findings(&mut findings);
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::heuristics::python::test_support::{
        assign, call, func, point, unit, with_keyword_arg,
    };
    use crate::ir::{BindingKind, Span};

    #[test]
    fn pandas_001_fires_on_iterrows_and_axis_one_apply() {
        let span = point(4);
        let rows = func(
            "f",
            Span::default(),
            vec![call("iterrows", Some("df.iterrows"), Some("df"), span, 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert_eq!(python_pandas_001(&[unit(vec![rows])]).len(), 1);

        let axis_one = func(
            "f",
            Span::default(),
            vec![with_keyword_arg(
                call("apply", Some("df.apply"), Some("df"), span, 0),
                "axis",
                "1",
            )],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert_eq!(python_pandas_001(&[unit(vec![axis_one])]).len(), 1);

        // `apply(1)` (positional, no axis keyword) is not row-wise.
        let plain = func(
            "f",
            Span::default(),
            vec![call("apply", Some("df.apply"), Some("df"), span, 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_pandas_001(&[unit(vec![plain])]).is_empty());
    }

    #[test]
    fn pandas_002_fires_on_concat_in_loop_silent_outside() {
        let span = point(6);
        let in_loop = func(
            "f",
            Span::default(),
            vec![call("concat", Some("pd.concat"), Some("pd"), span, 1)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert_eq!(python_pandas_002(&[unit(vec![in_loop])]).len(), 1);

        let outside = func(
            "f",
            Span::default(),
            vec![call("concat", Some("pd.concat"), Some("pd"), span, 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_pandas_002(&[unit(vec![outside])]).is_empty());

        // `.append` on a non-DataFrame (list) receiver stays silent.
        let list_append = func(
            "f",
            Span {
                start_line: 1,
                start_col: 1,
                end_line: 9,
                end_col: 1,
            },
            vec![call("append", Some("acc.append"), Some("acc"), span, 1)],
            vec![assign("acc", BindingKind::List, point(2), 0)],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(python_pandas_002(&[unit(vec![list_append])]).is_empty());
    }
}
