//! Parser-free `PERF-JS-*` IR fixture tests (plan `js-ts`, B2-W2 / W3a;
//! ADR-0001 guardrail 2).
//!
//! Each fixture under `tests/fixtures/ir/PERF-JS-*__*.json` is a serialized IR
//! v1 document deserialized as typed `Vec<FileUnit>` through
//! [`common::load_units_fixture`] — **no tree-sitter binding** — and run
//! through the REAL analyzer registry. One positive fixture per rule proves the
//! mechanism fires; the paired control proves the mandatory gate stays silent
//! (loop-invariance, accumulator shape, tiny trip counts, the async gate, the
//! visible-base-case predicate, the subscript exclusion, acyclic imports).
//!
//! Cross-language isolation is asserted directly: every finding in this suite
//! must carry the owning `PERF-JS-*` id, and a control must emit no finding of
//! ANY language.

mod common;

use perfpal::analyzer::finding::{
    JS_ALLOC_001_RULE_ID, JS_ALLOC_003_RULE_ID, JS_ARCH_001_RULE_ID, JS_ASYNC_001_RULE_ID,
    JS_ASYNC_002_RULE_ID, JS_CALL_001_RULE_ID, JS_LOOP_001_RULE_ID, JS_LOOP_002_RULE_ID,
    JS_LOOP_007_RULE_ID, JS_STR_001_RULE_ID, JS_TYPE_001_RULE_ID,
};
use perfpal::analyzer::{Confidence, Severity, analyze};

/// One fixture under test: `positive` fixtures must fire `rule_id`, controls
/// must stay silent.
struct Case {
    fixture: &'static str,
    rule_id: &'static str,
    positive: bool,
    /// Expected finding count for a positive fixture.
    expected: usize,
    /// Expected severity for every positive finding.
    severity: Severity,
}

/// The 22 fixtures: one positive + one control per frozen 11-rule catalog.
const CASES: [Case; 22] = [
    Case {
        fixture: "PERF-JS-ALLOC-001__literal-in-loop.json",
        rule_id: JS_ALLOC_001_RULE_ID,
        positive: true,
        expected: 1,
        severity: Severity::Info,
    },
    Case {
        fixture: "PERF-JS-ALLOC-001__loop-variant.json",
        rule_id: JS_ALLOC_001_RULE_ID,
        positive: false,
        expected: 0,
        severity: Severity::Info,
    },
    Case {
        // One fixture carries BOTH frozen accumulation shapes: the concat
        // call (`acc = acc.concat(x)`) and the spread literal
        // (`acc = [...acc, x]` via `has_spread`).
        fixture: "PERF-JS-ALLOC-003__concat-accumulator.json",
        rule_id: JS_ALLOC_003_RULE_ID,
        positive: true,
        expected: 2,
        severity: Severity::Info,
    },
    Case {
        fixture: "PERF-JS-ALLOC-003__snapshot-receiver.json",
        rule_id: JS_ALLOC_003_RULE_ID,
        positive: false,
        expected: 0,
        severity: Severity::Info,
    },
    Case {
        fixture: "PERF-JS-ARCH-001__import-cycle.json",
        rule_id: JS_ARCH_001_RULE_ID,
        positive: true,
        expected: 1,
        severity: Severity::Info,
    },
    Case {
        fixture: "PERF-JS-ARCH-001__acyclic-import.json",
        rule_id: JS_ARCH_001_RULE_ID,
        positive: false,
        expected: 0,
        severity: Severity::Info,
    },
    Case {
        fixture: "PERF-JS-ASYNC-001__await-in-loop.json",
        rule_id: JS_ASYNC_001_RULE_ID,
        positive: true,
        expected: 1,
        severity: Severity::Warn,
    },
    Case {
        fixture: "PERF-JS-ASYNC-001__await-outside-loop.json",
        rule_id: JS_ASYNC_001_RULE_ID,
        positive: false,
        expected: 0,
        severity: Severity::Warn,
    },
    Case {
        fixture: "PERF-JS-ASYNC-002__sync-in-async.json",
        rule_id: JS_ASYNC_002_RULE_ID,
        positive: true,
        expected: 1,
        severity: Severity::Warn,
    },
    Case {
        fixture: "PERF-JS-ASYNC-002__sync-in-non-async.json",
        rule_id: JS_ASYNC_002_RULE_ID,
        positive: false,
        expected: 0,
        severity: Severity::Warn,
    },
    Case {
        fixture: "PERF-JS-CALL-001__unguarded-recursion.json",
        rule_id: JS_CALL_001_RULE_ID,
        positive: true,
        expected: 1,
        severity: Severity::Info,
    },
    Case {
        fixture: "PERF-JS-CALL-001__guarded-recursion.json",
        rule_id: JS_CALL_001_RULE_ID,
        positive: false,
        expected: 0,
        severity: Severity::Info,
    },
    Case {
        fixture: "PERF-JS-LOOP-001__depth-two.json",
        rule_id: JS_LOOP_001_RULE_ID,
        positive: true,
        expected: 1,
        severity: Severity::Info,
    },
    Case {
        fixture: "PERF-JS-LOOP-001__depth-one.json",
        rule_id: JS_LOOP_001_RULE_ID,
        positive: false,
        expected: 0,
        severity: Severity::Info,
    },
    Case {
        fixture: "PERF-JS-LOOP-002__sync-io-in-loop.json",
        rule_id: JS_LOOP_002_RULE_ID,
        positive: true,
        expected: 1,
        severity: Severity::Warn,
    },
    Case {
        fixture: "PERF-JS-LOOP-002__sync-io-outside-loop.json",
        rule_id: JS_LOOP_002_RULE_ID,
        positive: false,
        expected: 0,
        severity: Severity::Warn,
    },
    Case {
        fixture: "PERF-JS-LOOP-007__membership-in-loop.json",
        rule_id: JS_LOOP_007_RULE_ID,
        positive: true,
        expected: 1,
        severity: Severity::Warn,
    },
    Case {
        fixture: "PERF-JS-LOOP-007__membership-small-loop.json",
        rule_id: JS_LOOP_007_RULE_ID,
        positive: false,
        expected: 0,
        severity: Severity::Warn,
    },
    Case {
        fixture: "PERF-JS-STR-001__accumulate-in-loop.json",
        rule_id: JS_STR_001_RULE_ID,
        positive: true,
        expected: 1,
        severity: Severity::Warn,
    },
    Case {
        fixture: "PERF-JS-STR-001__tiny-bounded-loop.json",
        rule_id: JS_STR_001_RULE_ID,
        positive: false,
        expected: 0,
        severity: Severity::Warn,
    },
    Case {
        fixture: "PERF-JS-TYPE-001__delete-property.json",
        rule_id: JS_TYPE_001_RULE_ID,
        positive: true,
        expected: 1,
        severity: Severity::Info,
    },
    Case {
        fixture: "PERF-JS-TYPE-001__delete-subscript.json",
        rule_id: JS_TYPE_001_RULE_ID,
        positive: false,
        expected: 0,
        severity: Severity::Info,
    },
];

#[test]
fn js_ir_fixtures_fire_their_owning_rule_or_stay_silent() {
    for case in CASES {
        let units = common::load_units_fixture(case.fixture);
        assert!(
            !units.is_empty(),
            "{}: fixture must contain at least one FileUnit",
            case.fixture
        );
        for unit in &units {
            assert_eq!(
                unit.file.language, "js",
                "{}: fixture unit {} must be language \"js\" (no parser bound)",
                case.fixture, unit.file.path
            );
        }

        let findings = analyze(&units);

        if !case.positive {
            assert!(
                findings.is_empty(),
                "{}: silent control must emit no finding of any language, got {:?}",
                case.fixture,
                findings
                    .iter()
                    .map(|f| (f.rule_id.as_str(), f.path.as_str(), f.span.start_line))
                    .collect::<Vec<_>>()
            );
            continue;
        }

        // Positive: the owning rule fires, and NOTHING else does (no
        // cross-language and no cross-rule leakage from the fixture).
        let matching: Vec<_> = findings
            .iter()
            .filter(|f| f.rule_id == case.rule_id)
            .collect();
        assert_eq!(
            matching.len(),
            case.expected,
            "{}: expected {} finding(s) of {}, got {:?}",
            case.fixture,
            case.expected,
            case.rule_id,
            findings
                .iter()
                .map(|f| (f.rule_id.as_str(), f.path.as_str(), f.span.start_line))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            findings.len(),
            case.expected,
            "{}: fixture emitted unexpected extra rules: {:?}",
            case.fixture,
            findings
                .iter()
                .map(|f| (f.rule_id.as_str(), f.span.start_line))
                .collect::<Vec<_>>()
        );
        for finding in &findings {
            assert!(
                finding.rule_id.starts_with("PERF-JS-"),
                "{}: cross-language rule id leaked: {}",
                case.fixture,
                finding.rule_id
            );
            assert_eq!(
                finding.severity, case.severity,
                "{}: severity drifted for {}",
                case.fixture, finding.rule_id
            );
            if case.rule_id == JS_TYPE_001_RULE_ID {
                assert_eq!(
                    finding.evidence.get("property").map(String::as_str),
                    Some("prop"),
                    "{}: TYPE-001 evidence must carry the deleted property name",
                    case.fixture
                );
            }
            assert_eq!(
                finding.confidence,
                Confidence::Medium,
                "{}: JS rules cap at MEDIUM (syntactic inference)",
                case.fixture
            );
            assert!(
                finding.message.contains("candidate optimization"),
                "{}: message must keep the optimization-candidate framing: {}",
                case.fixture,
                finding.message
            );
        }
    }
}
