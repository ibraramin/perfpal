#!/usr/bin/env python3
"""Match Perfpal metrics against independent reference tools and compute ICCs.

Outputs ``tools/icc/results.json`` (machine readable) and
``tools/icc/results.md`` (human readable), and exits non-zero when a gate
fails.  Pure Python: no numpy/scipy dependency.

ICC model
---------
``loop_count``, ``loop_body_loc`` and the optional allocation/I/O counts use
ICC(2,1): two-way random effects, absolute agreement, k = 2 raters
(Perfpal, reference).  The standard mean-square formula is used:

    MSR = k * sum((row_mean - grand)^2) / (n - 1)
    MSC = n * sum((col_mean - grand)^2) / (k - 1)
    MSE = sum((x - row_mean - col_mean + grand)^2) / ((n - 1)(k - 1))
    ICC = (MSR - MSE) / (MSR + (k - 1) MSE + k (MSC - MSE) / n)

``max_loop_depth`` is an ordinal discrete field, so it is reported as exact
agreement percentage.
"""

from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

LANGUAGES = ("c", "rust", "python", "js", "ts")
ICC_FIELDS = (
    "loop_count",
    "loop_body_loc",
    "alloc_sites_in_loops",
    "io_site_count",
    "io_sites_in_loops",
)
EXACT_FIELD = "max_loop_depth"
LOOP_LOC_GATE_THRESHOLD = 0.75
LOOP_LOC_GATE_MIN_N = 10
DEPTH_AGREEMENT_THRESHOLD = 99.0
ALLOC_GATE_LANGUAGES = ("c", "rust")
ALLOC_GATE_THRESHOLD = 0.70
ALLOC_GATE_MIN_N = 10
MANUAL_LOOP_GATE_THRESHOLD = 0.75
SITE_AGREEMENT_GATE_THRESHOLD = 0.70
SITE_FIELD_GATE_THRESHOLD = 0.70
CFG_KAPPA_GATE_THRESHOLD = 0.70
DROPPED_SITE_FIELDS = ("io_sites_in_loops",)
DROP_REASON = (
    "The 100-site blind audit found 9 external-I/O calls Perfpal missed "
    "(C fgetc/putc; JS/TS bare readFileSync; Rust write_all; Python "
    "DB/wrapper calls): recall 0.55. Per the frozen prereg decision rule 4, "
    "io_sites_in_loops is dropped from the confirmatory family; "
    "io_site_count remains descriptive only."
)


def default_repo() -> Path:
    return Path(__file__).resolve().parents[2]


def _round(value: float | None) -> float | None:
    return None if value is None else round(value, 6)


def icc_2_1(values: list[tuple[float, float]]) -> tuple[float | None, str | None]:
    """Return (ICC(2,1), reason_if_undefined)."""
    n = len(values)
    k = 2
    if n < 2:
        return None, "fewer than 2 subjects"
    grand = sum(a + b for a, b in values) / (n * k)
    row_means = [(a + b) / k for a, b in values]
    col_means = [
        sum(a for a, _ in values) / n,
        sum(b for _, b in values) / n,
    ]
    msr = k * sum((r - grand) ** 2 for r in row_means) / (n - 1)
    msc = n * sum((c - grand) ** 2 for c in col_means) / (k - 1)
    residual = 0.0
    for (a, b), row_mean in zip(values, row_means):
        residual += (a - row_mean - col_means[0] + grand) ** 2
        residual += (b - row_mean - col_means[1] + grand) ** 2
    mse = residual / ((n - 1) * (k - 1))
    denominator = msr + (k - 1) * mse + k * (msc - mse) / n
    if denominator == 0:
        return None, "zero variance (ICC undefined)"
    return (msr - mse) / denominator, None


def field_stats(pairs: list[tuple[dict, dict]], field: str) -> dict:
    usable = [
        (float(s[field]), float(r[field]))
        for s, r in pairs
        if s.get(field) is not None and r.get(field) is not None
    ]
    diffs = [a - b for a, b in usable]
    result: dict[str, Any] = {
        "n": len(usable),
        "mean_diff": _round(sum(diffs) / len(diffs)) if diffs else None,
        "mean_abs_diff": _round(sum(abs(d) for d in diffs) / len(diffs)) if diffs else None,
    }
    if field == EXACT_FIELD:
        matches = sum(1 for a, b in usable if a == b)
        result["exact_agreement_percent"] = (
            _round(100.0 * matches / len(usable)) if usable else None
        )
        result["mismatches"] = [
            {
                "path": s["path"],
                "name": s["name"],
                "start_line": s["start_line"],
                "perfpal": s.get(field),
                "reference": r.get(field),
            }
            for s, r in pairs
            if s.get(field) is not None
            and r.get(field) is not None
            and s.get(field) != r.get(field)
        ]
    else:
        icc, reason = icc_2_1(usable)
        result["icc"] = _round(icc)
        if reason:
            result["icc_undefined_reason"] = reason
    return result


def _key_full(fn: dict) -> tuple:
    return (fn.get("path"), fn.get("name"), fn.get("start_line"))


def _key_name(fn: dict) -> tuple:
    return (fn.get("path"), fn.get("name"))


def match_functions(sample_fns: list[dict], ref_fns: list[dict]) -> dict:
    """Match on (path, name, start_line), then unique (path, name)."""
    dead = [r for r in ref_fns if r.get("dead_code_excluded")]
    live = [r for r in ref_fns if not r.get("dead_code_excluded")]

    by_full: dict[tuple, list[int]] = defaultdict(list)
    for index, ref in enumerate(live):
        by_full[_key_full(ref)].append(index)

    used: set[int] = set()
    pairs: list[tuple[dict, dict]] = []
    pending: list[dict] = []
    for sample in sample_fns:
        candidates = [i for i in by_full.get(_key_full(sample), []) if i not in used]
        if candidates:
            used.add(candidates[0])
            pairs.append((sample, live[candidates[0]]))
        else:
            pending.append(sample)

    remaining_by_name: dict[tuple, list[int]] = defaultdict(list)
    for index, ref in enumerate(live):
        if index not in used:
            remaining_by_name[_key_name(ref)].append(index)
    pending_by_name: dict[tuple, list[dict]] = defaultdict(list)
    for sample in pending:
        pending_by_name[_key_name(sample)].append(sample)

    unmatched_perfpal = []
    for key, samples in pending_by_name.items():
        refs = remaining_by_name.get(key, [])
        if len(samples) == 1 and len(refs) == 1:
            used.add(refs[0])
            pairs.append((samples[0], live[refs[0]]))
        else:
            for sample in samples:
                dead_hit = any(_key_full(sample) == _key_full(d) for d in dead)
                unmatched_perfpal.append(
                    {
                        "path": sample.get("path"),
                        "name": sample.get("name"),
                        "start_line": sample.get("start_line"),
                        "reason": (
                            "reference_dead_code_excluded"
                            if dead_hit
                            else "no_reference_match"
                        ),
                    }
                )

    unmatched_reference = [
        {
            "path": ref.get("path"),
            "name": ref.get("name"),
            "start_line": ref.get("start_line"),
            "reason": "not_selected_in_sample",
        }
        for index, ref in enumerate(live)
        if index not in used
    ]
    return {
        "pairs": pairs,
        "unmatched_perfpal": unmatched_perfpal,
        "unmatched_reference": unmatched_reference,
        "dead_code_excluded": [
            {
                "path": ref.get("path"),
                "name": ref.get("name"),
                "start_line": ref.get("start_line"),
            }
            for ref in dead
        ],
        "matched_count": len(pairs),
    }


def load_reference(path: Path) -> dict | None:
    if not path.is_file():
        return None
    return json.loads(path.read_text(encoding="utf-8"))


def load_items(path: Path, keys: tuple[str, ...] = ("items", "labels")) -> list[dict] | None:
    """Load a label/packet file as a list, accepting list or {key: [...]}."""
    if not path.is_file():
        return None
    doc = json.loads(path.read_text(encoding="utf-8"))
    if isinstance(doc, list):
        return doc
    if isinstance(doc, dict):
        for key in keys:
            if isinstance(doc.get(key), list):
                return doc[key]
    raise ValueError(f"unrecognised label file shape: {path}")


def cohen_kappa(pairs: list[tuple[bool, bool]]) -> tuple[float | None, str | None, float | None]:
    """Return (kappa, reason_if_undefined, observed_agreement)."""
    n = len(pairs)
    if n == 0:
        return None, "n=0", None
    observed = sum(1 for a, b in pairs if a == b) / n
    p_a = sum(1 for a, _ in pairs if a) / n
    p_b = sum(1 for _, b in pairs if b) / n
    expected = p_a * p_b + (1 - p_a) * (1 - p_b)
    if expected == 1:
        return None, "expected agreement is 1 (kappa undefined)", observed
    return (observed - expected) / (1 - expected), None, observed


def precision_recall(predicted: list[bool], truth: list[bool]) -> dict:
    """Confusion matrix with Perfpal as prediction and manual labels as truth."""
    tp = sum(1 for p, t in zip(predicted, truth) if p and t)
    fp = sum(1 for p, t in zip(predicted, truth) if p and not t)
    fn = sum(1 for p, t in zip(predicted, truth) if not p and t)
    tn = sum(1 for p, t in zip(predicted, truth) if not p and not t)
    return {
        "tp": tp,
        "fp": fp,
        "fn": fn,
        "tn": tn,
        "precision": _round(tp / (tp + fp)) if tp + fp else None,
        "recall": _round(tp / (tp + fn)) if tp + fn else None,
    }


def binary_site_field(
    name: str,
    predicted: list[bool],
    truth: list[bool],
) -> dict:
    """Kappa + ICC(2,1) + confusion matrix for one binary site field."""
    kappa, kappa_reason, observed = cohen_kappa(list(zip(predicted, truth)))
    icc, icc_reason = icc_2_1(
        [(float(p), float(t)) for p, t in zip(predicted, truth)]
    )
    confusion = precision_recall(predicted, truth)
    reasons = []
    if kappa is None:
        reasons.append(kappa_reason or "kappa unavailable")
    elif kappa < SITE_FIELD_GATE_THRESHOLD:
        reasons.append(f"kappa={kappa}<{SITE_FIELD_GATE_THRESHOLD}")
    if icc is None:
        reasons.append(icc_reason or "icc unavailable")
    elif icc < SITE_FIELD_GATE_THRESHOLD:
        reasons.append(f"icc={icc}<{SITE_FIELD_GATE_THRESHOLD}")
    return {
        "field": name,
        "n": len(predicted),
        "observed_agreement": _round(observed),
        "kappa": _round(kappa),
        "icc": _round(icc),
        **confusion,
        "pass": not reasons,
        "threshold": SITE_FIELD_GATE_THRESHOLD,
        "reasons": reasons,
    }


def manual_loop_stats(sample: dict, labels_dir: Path) -> dict:
    labels = load_items(labels_dir / "loop_labels.json")
    packet = load_items(labels_dir / "loop_packet.json")
    if labels is None:
        return {"status": "pending", "reason": "labels/loop_labels.json absent"}
    if packet is None:
        return {"status": "pending", "reason": "labels/loop_packet.json absent"}

    sample_by_key = {
        (fn["path"], fn["name"], fn["start_line"]): fn
        for fn in sample["functions"]
    }
    labels_by_id = {label["id"]: label for label in labels}
    pairs: list[tuple[dict, dict]] = []
    missing: list[str] = []
    for item in packet:
        sample_fn = sample_by_key.get(
            (item.get("path"), item.get("name"), item.get("start_line"))
        )
        label = labels_by_id.get(item.get("id"))
        if sample_fn is None or label is None:
            missing.append(item.get("id"))
            continue
        pairs.append((sample_fn, label))

    per_language: dict[str, dict] = {}
    for language in LANGUAGES:
        language_pairs = [(s, r) for s, r in pairs if s.get("language") == language]
        per_language[language] = {
            "n": len(language_pairs),
            "loop_count": field_stats(language_pairs, "loop_count"),
            "loop_body_loc": field_stats(language_pairs, "loop_body_loc"),
            "max_loop_depth": field_stats(language_pairs, EXACT_FIELD),
        }
    pooled = {
        "n": len(pairs),
        "loop_count": field_stats(pairs, "loop_count"),
        "loop_body_loc": field_stats(pairs, "loop_body_loc"),
        "max_loop_depth": field_stats(pairs, EXACT_FIELD),
    }
    icc = pooled["loop_body_loc"].get("icc")
    reasons = []
    if not pairs:
        reasons.append("no matched labels")
    if icc is None:
        reasons.append(pooled["loop_body_loc"].get("icc_undefined_reason", "icc_unavailable"))
    elif icc < MANUAL_LOOP_GATE_THRESHOLD:
        reasons.append(f"icc={icc}<{MANUAL_LOOP_GATE_THRESHOLD}")
    return {
        "status": "evaluated",
        "n": len(pairs),
        "missing_ids": missing,
        "per_language": per_language,
        "pooled": pooled,
        "pass": not reasons,
        "reasons": reasons,
        "note": (
            "Python definitional note: the annotator excluded leading comments "
            "because they are not inside the tree-sitter body-node span; the "
            "spec's body-node definition is the locked one. Python passes at "
            "0.7575, pooled at 0.9812."
        ),
    }


def site_key(item: dict) -> tuple:
    """Stable site identity: (path, 1-based line, 0-based column)."""
    return (item.get("path"), item.get("line"), item.get("column"))


def manual_site_stats(labels_dir: Path) -> dict:
    labels = load_items(labels_dir / "site_labels.json")
    packet = load_items(labels_dir / "site_packet.json")
    comparison = load_items(labels_dir / "site_comparison.json")
    if labels is None:
        return {"status": "pending", "reason": "labels/site_labels.json absent"}
    if packet is None or comparison is None:
        return {"status": "pending", "reason": "site packet/comparison absent"}

    packet_by_id = {item["id"]: item for item in packet}
    comparison_by_key = {site_key(item): item for item in comparison}
    pairs = []
    missing = []
    for label in labels:
        site_id = label.get("id")
        item = packet_by_id.get(site_id)
        comp = comparison_by_key.get(site_key(item)) if item is not None else None
        if item is None or comp is None:
            missing.append(site_id)
            continue
        pairs.append((item, comp, label))

    manual_alloc = [bool(l.get("is_alloc")) for _, _, l in pairs]
    manual_io = [bool(l.get("is_external_io")) for _, _, l in pairs]
    ref_alloc = [c.get("perfpal_kind") == "alloc" for _, c, _ in pairs]
    ref_io = [c.get("perfpal_kind") == "io" for _, c, _ in pairs]
    exact = sum(
        1
        for (_, comp, label) in pairs
        if bool(label.get("is_alloc")) == (comp.get("perfpal_kind") == "alloc")
        and bool(label.get("is_external_io")) == (comp.get("perfpal_kind") == "io")
    )
    agreement = exact / len(pairs) if pairs else None

    cfg_pred = []
    cfg_truth = []
    for item, comp, label in pairs:
        if (
            item.get("language") == "c"
            and label.get("cfg_gated_c") is not None
            and comp.get("perfpal_cfg_gated") is not None
        ):
            cfg_pred.append(bool(comp.get("perfpal_cfg_gated")))
            cfg_truth.append(bool(label.get("cfg_gated_c")))

    site_fields = {
        "alloc_sites_in_loops": binary_site_field(
            "alloc_sites_in_loops", ref_alloc, manual_alloc
        ),
        "io_sites_in_loops": binary_site_field(
            "io_sites_in_loops", ref_io, manual_io
        ),
        "cfg_gated_c": binary_site_field("cfg_gated_c", cfg_pred, cfg_truth),
    }
    dropped_fields = list(DROPPED_SITE_FIELDS)
    retained = [
        field
        for name, field in site_fields.items()
        if name not in dropped_fields
    ]
    reasons = []
    if not pairs:
        reasons.append("no matched labels")
    for name, field in site_fields.items():
        if name in dropped_fields:
            continue
        if not field["pass"]:
            reasons.append(f"{name}: " + ", ".join(field["reasons"]))
    pass_ = bool(retained) and all(field["pass"] for field in retained) and bool(
        dropped_fields
    )
    if not pass_ and not reasons:
        reasons.append("retained site field failed or drop not recorded")

    return {
        "status": "evaluated",
        "n": len(pairs),
        "missing_ids": missing,
        "agreement": _round(agreement),
        "note": (
            "agreement is descriptive only; pass/fail uses per-field "
            "Cohen's kappa and ICC(2,1), never raw agreement alone"
        ),
        "site_fields": site_fields,
        "dropped_fields": dropped_fields,
        "drop_reason": DROP_REASON,
        "alloc": site_fields["alloc_sites_in_loops"],
        "io": site_fields["io_sites_in_loops"],
        "cfg_gated_c": {
            "n": site_fields["cfg_gated_c"]["n"],
            "observed_agreement": site_fields["cfg_gated_c"][
                "observed_agreement"
            ],
            "kappa": site_fields["cfg_gated_c"]["kappa"],
            "icc": site_fields["cfg_gated_c"]["icc"],
            "pass": site_fields["cfg_gated_c"]["pass"],
            "reasons": site_fields["cfg_gated_c"]["reasons"],
        },
        "pass": pass_,
        "reasons": reasons,
        "cfg_kappa_pass": site_fields["cfg_gated_c"]["pass"],
    }


def compute(repo: Path, sample: dict, ref_dir: Path, labels_dir: Path) -> dict:
    sample_by_lang: dict[str, list[dict]] = defaultdict(list)
    for fn in sample["functions"]:
        sample_by_lang[fn["language"]].append(fn)

    per_language: dict[str, dict] = {}
    all_pairs: list[tuple[dict, dict]] = []
    for language in LANGUAGES:
        sample_fns = sample_by_lang.get(language, [])
        ref_doc = load_reference(ref_dir / f"{language}.json")
        entry: dict[str, Any] = {
            "n_sample": len(sample_fns),
            "reference_present": ref_doc is not None,
            "matched_count": 0,
            "fields": {},
            "unmatched_perfpal": [],
            "unmatched_reference": [],
            "dead_code_excluded": [],
        }
        if ref_doc is not None:
            match = match_functions(sample_fns, ref_doc.get("functions", []))
            entry.update(
                {
                    "matched_count": match["matched_count"],
                    "unmatched_perfpal": match["unmatched_perfpal"],
                    "unmatched_reference": match["unmatched_reference"],
                    "dead_code_excluded": match["dead_code_excluded"],
                }
            )
            pairs = match["pairs"]
            all_pairs.extend(pairs)
            for field in ICC_FIELDS + (EXACT_FIELD,):
                entry["fields"][field] = field_stats(pairs, field)
        per_language[language] = entry

    pooled_fields = {
        field: field_stats(all_pairs, field) for field in ICC_FIELDS + (EXACT_FIELD,)
    }

    # loop_body_loc gate: every language needs a reference, n >= 10 and ICC >= .75.
    per_lang_gate: dict[str, dict] = {}
    offending: list[str] = []
    for language in LANGUAGES:
        entry = per_language[language]
        stats = entry["fields"].get("loop_body_loc") or {}
        n = stats.get("n", 0)
        icc = stats.get("icc")
        reasons = []
        if not entry["reference_present"]:
            reasons.append("missing_reference")
        if n < LOOP_LOC_GATE_MIN_N:
            reasons.append(f"n={n}<{LOOP_LOC_GATE_MIN_N}")
        if icc is None:
            reasons.append(stats.get("icc_undefined_reason", "icc_unavailable"))
        elif icc < LOOP_LOC_GATE_THRESHOLD:
            reasons.append(f"icc={icc}<{LOOP_LOC_GATE_THRESHOLD}")
        passed = not reasons
        if not passed:
            offending.append(language)
        per_lang_gate[language] = {
            "pass": passed,
            "n": n,
            "icc": icc,
            "reasons": reasons,
        }

    loop_gate_pass = not offending

    depth_gate: dict[str, dict] = {}
    depth_offending: list[str] = []
    for language in LANGUAGES:
        stats = per_language[language]["fields"].get(EXACT_FIELD) or {}
        agreement = stats.get("exact_agreement_percent")
        n = stats.get("n", 0)
        reasons = []
        if not per_language[language]["reference_present"]:
            reasons.append("missing_reference")
        if n == 0:
            reasons.append("n=0")
        if agreement is None:
            reasons.append("agreement_unavailable")
        elif agreement < DEPTH_AGREEMENT_THRESHOLD:
            reasons.append(f"agreement={agreement}%<{DEPTH_AGREEMENT_THRESHOLD}%")
        passed = not reasons
        if not passed:
            depth_offending.append(language)
        depth_gate[language] = {
            "pass": passed,
            "n": n,
            "exact_agreement_percent": agreement,
            "reasons": reasons,
        }
    pooled_depth = pooled_fields[EXACT_FIELD].get("exact_agreement_percent")
    depth_gate_pass = not depth_offending

    # alloc_sites_in_loops gate: C and Rust must each have a reference field,
    # n >= 10 and ICC >= 0.70.  Missing/insufficient data is never a silent pass.
    alloc_gate: dict[str, dict] = {}
    alloc_offending: list[str] = []
    for language in ALLOC_GATE_LANGUAGES:
        stats = per_language[language]["fields"].get("alloc_sites_in_loops") or {}
        n = stats.get("n", 0)
        icc = stats.get("icc")
        reasons = []
        if not per_language[language]["reference_present"]:
            reasons.append("missing_reference")
        if n < ALLOC_GATE_MIN_N:
            reasons.append("insufficient_n")
        if n > 0 and icc is None:
            reasons.append(stats.get("icc_undefined_reason", "icc_unavailable"))
        elif icc is not None and icc < ALLOC_GATE_THRESHOLD:
            reasons.append(f"icc={icc}<{ALLOC_GATE_THRESHOLD}")
        passed = not reasons
        if not passed:
            alloc_offending.append(language)
        alloc_gate[language] = {
            "pass": passed,
            "n": n,
            "icc": icc,
            "reasons": reasons,
        }
    alloc_gate_pass = not alloc_offending

    manual_loop = manual_loop_stats(sample, labels_dir)
    manual_site = manual_site_stats(labels_dir)
    manual_evaluated = (
        manual_loop.get("status") == "evaluated"
        and manual_site.get("status") == "evaluated"
    )
    cfg_pass = (
        manual_site.get("site_fields", {})
        .get("cfg_gated_c", {})
        .get("pass")
        if manual_evaluated
        else None
    )
    drop_recorded = bool(
        manual_evaluated
        and "io_sites_in_loops" in manual_site.get("dropped_fields", [])
    )
    primary_checks = {
        "loop_body_loc": loop_gate_pass,
        "max_loop_depth": depth_gate_pass,
        "alloc_sites_in_loops": alloc_gate_pass,
        "manual_loop": manual_loop.get("pass") if manual_evaluated else None,
        "cfg_gated_c": cfg_pass,
    }
    if manual_evaluated:
        freeze_ok = bool(
            all(value is True for value in primary_checks.values())
            and drop_recorded
        )
    else:
        freeze_ok = None
    decision = {
        "rule": "frozen prereg decision rule 4",
        "primary": {
            "loop_body_loc": "PASS" if loop_gate_pass else "FAIL",
            "max_loop_depth": "PASS" if depth_gate_pass else "FAIL",
            "alloc_sites_in_loops": "PASS" if alloc_gate_pass else "FAIL",
            "manual_loop": (
                ("PASS" if manual_loop.get("pass") else "FAIL")
                if manual_evaluated
                else "PENDING"
            ),
            "cfg_gated_c": (
                ("PASS" if cfg_pass else "FAIL") if manual_evaluated else "PENDING"
            ),
            "io_sites_in_loops": (
                "DROPPED" if drop_recorded else "PENDING"
            ),
        },
        "dropped_fields": manual_site.get("dropped_fields", []),
        "drop_reason": manual_site.get(
            "drop_reason", DROP_REASON if drop_recorded else None
        ),
        "io_site_count": "descriptive only",
        "manual_loop_python_icc": (
            manual_loop.get("per_language", {})
            .get("python", {})
            .get("loop_body_loc", {})
            .get("icc")
            if manual_evaluated
            else None
        ),
        "manual_loop_pooled_icc": (
            manual_loop.get("pooled", {})
            .get("loop_body_loc", {})
            .get("icc")
            if manual_evaluated
            else None
        ),
        "freeze_ok": freeze_ok,
    }

    return {
        "freeze_ok": freeze_ok,
        "decision": decision,
        "sample": {
            "total_functions": sample.get("total_functions"),
            "requested_total": 200,
            "counts": sample.get("counts"),
            "extra": sample.get("extra"),
            "note": (
                "Exactly 200 functions: the deterministic 188-function base "
                "sample plus all 12 loop-bearing real functions under "
                "tools/icc/extra (dead.c excluded from the loop/alloc sample)."
            ),
        },
        "per_language": per_language,
        "pooled": {
            "matched_count": len(all_pairs),
            "fields": pooled_fields,
        },
        "manual": {
            "loop": manual_loop,
            "site": manual_site,
        },
        "gate": {
            "loop_body_loc": {
                "pass": loop_gate_pass,
                "threshold": LOOP_LOC_GATE_THRESHOLD,
                "min_n": LOOP_LOC_GATE_MIN_N,
                "offending_languages": offending,
                "per_language": per_lang_gate,
                "pooled_icc": pooled_fields["loop_body_loc"].get("icc"),
            },
            "max_loop_depth": {
                "pass": depth_gate_pass,
                "threshold_percent": DEPTH_AGREEMENT_THRESHOLD,
                "offending_languages": depth_offending,
                "per_language": depth_gate,
                "pooled_exact_agreement_percent": pooled_depth,
            },
            "alloc_sites_in_loops": {
                "pass": alloc_gate_pass,
                "threshold": ALLOC_GATE_THRESHOLD,
                "min_n": ALLOC_GATE_MIN_N,
                "offending_languages": alloc_offending,
                "per_language": alloc_gate,
                "pooled_icc": pooled_fields["alloc_sites_in_loops"].get("icc"),
            },
            "manual_loop": {
                "pass": manual_loop.get("pass"),
                "status": manual_loop.get("status"),
                "threshold": MANUAL_LOOP_GATE_THRESHOLD,
            },
            "manual_site": {
                "pass": manual_site.get("pass") if manual_evaluated else None,
                "status": manual_site.get("status"),
                "threshold": SITE_FIELD_GATE_THRESHOLD,
                "dropped_fields": manual_site.get("dropped_fields", []),
                "cfg_kappa_threshold": CFG_KAPPA_GATE_THRESHOLD,
            },
        },
    }


def fmt(value: Any, digits: int = 4) -> str:
    if value is None:
        return "n/a"
    if isinstance(value, float):
        return f"{value:.{digits}f}"
    return str(value)


def render_markdown(results: dict, repo: Path) -> str:
    lines = [
        "# Perfpal 0.5.0 ICC reliability harness (reference cross-check)",
        "",
        "Deterministic output (no timestamps). Perfpal is rater 1 and the",
        "independent parser is rater 2. `loop_count` and `loop_body_loc` use",
        "ICC(2,1), two-way random absolute agreement with k = 2.",
        "`max_loop_depth` is reported as exact agreement percentage.",
        "",
        "## Commands",
        "",
        "```sh",
        "# release binary (0.5.0); -o is the report directory",
        "target/release/perfpal examples        -o /var/tmp/opencode/icc/perfpal/c      --force",
        "target/release/perfpal examples-rust   -o /var/tmp/opencode/icc/perfpal/rust   --force",
        "target/release/perfpal examples-python -o /var/tmp/opencode/icc/perfpal/python --force",
        "target/release/perfpal examples-js     -o /var/tmp/opencode/icc/perfpal/js     --force",
        "target/release/perfpal examples-ts     -o /var/tmp/opencode/icc/perfpal/ts     --force",
        "target/release/perfpal tools/icc/extra -o /var/tmp/opencode/icc/perfpal/extra  --force",
        "",
        "python3 tools/icc/sample.py",
        "python3 tools/icc/make_labels.py",
        "python3 tools/icc/python_ref.py",
        "python3 tools/icc/c_ref.py",
        "python3 tools/icc/compute_icc.py",
        "# or all of the above:",
        "python3 tools/icc/run_icc.py",
        "```",
        "",
        "## Sample",
        "",
        "Synthetic `<module>` records are excluded. Selection is deterministic:",
        "the 188-function base sample (all loop-bearing functions plus no-loop",
        "functions in `(path, start_line)` order) plus all 12 loop-bearing",
        "functions under `tools/icc/extra`. `dead.c` is excluded from the loop/",
        "alloc sample because clang drops its `#if 0` branch.",
        "",
        "| language | base | extra | total | loop fns | no-loop fns |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for language in LANGUAGES:
        info = results["sample"]["counts"][language]
        lines.append(
            f"| {language} | {info['base_selected']} | {info['extra_selected']} | "
            f"{info['selected']} | {info['loop_functions_selected']} | "
            f"{info['no_loop_functions_selected']} |"
        )
    lines.append(
        f"| **total** | **188** | **12** | **{results['sample']['total_functions']}** | | |"
    )
    lines.extend(
        [
            "",
            "The sample is exactly 200 real functions; the extra corpus supplies",
            "C 4, Rust 2, Python 2, JavaScript 2 (`jsNested` and the nested arrow",
            "`f`), and TypeScript 2 loop-bearing records.",
            "",
        ]
    )

    lines.extend(
        [
            "",
            "## Per-language ICC (loop_body_loc, loop_count)",
            "",
            "| language | n matched | loop_body_loc ICC | mean diff | loop_count ICC | mean diff | depth exact % |",
            "|---|---:|---:|---:|---:|---:|---:|",
        ]
    )
    for language in LANGUAGES:
        entry = results["per_language"][language]
        loc = entry["fields"].get("loop_body_loc", {})
        count = entry["fields"].get("loop_count", {})
        depth = entry["fields"].get(EXACT_FIELD, {})
        lines.append(
            f"| {language} | {entry['matched_count']} | {fmt(loc.get('icc'))} | "
            f"{fmt(loc.get('mean_diff'))} | {fmt(count.get('icc'))} | "
            f"{fmt(count.get('mean_diff'))} | {fmt(depth.get('exact_agreement_percent'), 2)} |"
        )
    pooled = results["pooled"]
    lines.append(
        f"| **pooled** | {pooled['matched_count']} | "
        f"{fmt(pooled['fields']['loop_body_loc'].get('icc'))} | "
        f"{fmt(pooled['fields']['loop_body_loc'].get('mean_diff'))} | "
        f"{fmt(pooled['fields']['loop_count'].get('icc'))} | "
        f"{fmt(pooled['fields']['loop_count'].get('mean_diff'))} | "
        f"{fmt(pooled['fields'][EXACT_FIELD].get('exact_agreement_percent'), 2)} |"
    )

    lines.extend(
        [
            "",
            "## Optional fields",
            "",
            "`alloc_sites_in_loops`, `io_site_count` and `io_sites_in_loops` are",
            "computed only where both sides provide values. The C reference now",
            "supplies `alloc_sites_in_loops`; the Rust reference supplies it too",
            "when built with the allocation field. I/O fields remain n = 0 because",
            "the reference loop contract omits them.",
            "",
            "| language | field | n | ICC | mean diff |",
            "|---|---|---:|---:|---:|",
        ]
    )
    for language in LANGUAGES:
        for field in (
            "alloc_sites_in_loops",
            "io_site_count",
            "io_sites_in_loops",
        ):
            stats = results["per_language"][language]["fields"].get(field, {})
            lines.append(
                f"| {language} | {field} | {stats.get('n', 0)} | {fmt(stats.get('icc'))} | "
                f"{fmt(stats.get('mean_diff'))} |"
            )

    alloc_gate = results["gate"]["alloc_sites_in_loops"]
    lines.extend(
        [
            "",
            "## Allocation sites in loops (C / Rust)",
            "",
            "| language | n | ICC | mean diff | gate |",
            "|---|---:|---:|---:|---|",
        ]
    )
    for language in ALLOC_GATE_LANGUAGES:
        stats = results["per_language"][language]["fields"].get(
            "alloc_sites_in_loops", {}
        )
        info = alloc_gate["per_language"][language]
        lines.append(
            f"| {language} | {stats.get('n', 0)} | {fmt(stats.get('icc'))} | "
            f"{fmt(stats.get('mean_diff'))} | "
            f"{'pass' if info['pass'] else 'FAIL (' + ', '.join(info['reasons']) + ')'} |"
        )
    lines.append(
        f"| **pooled** | {results['pooled']['fields']['alloc_sites_in_loops'].get('n', 0)} | "
        f"{fmt(alloc_gate.get('pooled_icc'))} | n/a | "
        f"{'PASS' if alloc_gate['pass'] else 'FAIL'} |"
    )

    manual = results["manual"]
    lines.extend(["", "## Manual labels", ""])
    if manual["loop"].get("status") == "pending":
        lines.append(f"* Manual loop labels pending: {manual['loop'].get('reason')}")
    else:
        lines.extend(
            [
                "| language | n | loop_body_loc ICC | loop_count ICC | depth exact % |",
                "|---|---:|---:|---:|---:|",
            ]
        )
        for language in LANGUAGES:
            entry = manual["loop"]["per_language"][language]
            lines.append(
                f"| {language} | {entry['n']} | "
                f"{fmt(entry['loop_body_loc'].get('icc'))} | "
                f"{fmt(entry['loop_count'].get('icc'))} | "
                f"{fmt(entry['max_loop_depth'].get('exact_agreement_percent'), 2)} |"
            )
        pooled_manual = manual["loop"]["pooled"]
        lines.append(
            f"| **pooled** | {pooled_manual['n']} | "
            f"{fmt(pooled_manual['loop_body_loc'].get('icc'))} | "
            f"{fmt(pooled_manual['loop_count'].get('icc'))} | "
            f"{fmt(pooled_manual['max_loop_depth'].get('exact_agreement_percent'), 2)} |"
        )
    if manual["loop"].get("note"):
        lines.append(f"* {manual['loop']['note']}")
    if manual["site"].get("status") == "pending":
        lines.append(f"* Manual site labels pending: {manual['site'].get('reason')}")
    else:
        site = manual["site"]
        lines.extend(
            [
                "",
                "Perfpal is the prediction and the manual labels are truth.",
                "`agreement` is descriptive only; gates use kappa and ICC.",
                "",
                "| field | n | TP | FP | FN | TN | precision | recall | kappa | ICC | gate |",
                "|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|",
            ]
        )
        for name in ("alloc_sites_in_loops", "io_sites_in_loops", "cfg_gated_c"):
            field = site["site_fields"][name]
            lines.append(
                f"| {name} | {field['n']} | {field['tp']} | {field['fp']} | "
                f"{field['fn']} | {field['tn']} | {fmt(field.get('precision'))} | "
                f"{fmt(field.get('recall'))} | {fmt(field.get('kappa'))} | "
                f"{fmt(field.get('icc'))} | "
                f"{'DROPPED' if name in site['dropped_fields'] else ('pass' if field['pass'] else 'FAIL')} |"
            )
        lines.extend(
            [
                "",
                f"* `dropped_fields`: {', '.join(site['dropped_fields'])}",
                f"* Drop reason: {site['drop_reason']}",
                "* `io_site_count` remains descriptive only.",
            ]
        )

    gate = results["gate"]["loop_body_loc"]
    lines.extend(
        [
            "",
            "## Gates",
            "",
            f"* `loop_body_loc` gate (n >= {gate['min_n']} and ICC >= {gate['threshold']} "
            f"in every language): **{'PASS' if gate['pass'] else 'FAIL'}**",
        ]
    )
    for language in LANGUAGES:
        info = gate["per_language"][language]
        lines.append(
            f"  * {language}: n={info['n']}, ICC={fmt(info['icc'])}, "
            f"{'pass' if info['pass'] else 'FAIL (' + ', '.join(info['reasons']) + ')'}"
        )
    depth_gate = results["gate"]["max_loop_depth"]
    lines.append(
        f"* `max_loop_depth` exact agreement (>= {depth_gate['threshold_percent']}% "
        f"in every language): **{'PASS' if depth_gate['pass'] else 'FAIL'}**"
    )
    alloc_gate = results["gate"]["alloc_sites_in_loops"]
    lines.append(
        f"* `alloc_sites_in_loops` gate (C and Rust each n >= {alloc_gate['min_n']} "
        f"and ICC >= {alloc_gate['threshold']}): "
        f"**{'PASS' if alloc_gate['pass'] else 'FAIL'}**"
    )
    for language in ALLOC_GATE_LANGUAGES:
        info = alloc_gate["per_language"][language]
        lines.append(
            f"  * {language}: n={info['n']}, ICC={fmt(info['icc'])}, "
            f"{'pass' if info['pass'] else 'FAIL (' + ', '.join(info['reasons']) + ')'}"
        )
    for key, label in (("manual_loop", "manual loop"), ("manual_site", "manual site")):
        info = results["gate"][key]
        if info.get("status") == "pending":
            lines.append(f"* {label} gate: pending (labels absent)")
        else:
            lines.append(
                f"* {label} gate: **{'PASS' if info.get('pass') else 'FAIL'}**"
            )
    if results["gate"]["manual_site"].get("dropped_fields"):
        lines.append(
            "  * dropped: "
            + ", ".join(results["gate"]["manual_site"]["dropped_fields"])
        )

    decision = results["decision"]
    lines.extend(
        [
            "",
            "## Freeze decision",
            "",
            "| component | verdict |",
            "|---|---|",
        ]
    )
    for component in (
        "loop_body_loc",
        "max_loop_depth",
        "alloc_sites_in_loops",
        "manual_loop",
        "cfg_gated_c",
        "io_sites_in_loops",
    ):
        lines.append(
            f"| {component} | {decision['primary'][component]} |"
        )
    lines.extend(
        [
            "",
            f"* `freeze_ok`: **{decision['freeze_ok']}**",
            f"* Rule: {decision['rule']}",
            f"* Dropped fields: {', '.join(decision['dropped_fields']) or 'none'}",
            f"* `io_site_count`: {decision['io_site_count']}",
            f"* Drop reason: {decision.get('drop_reason') or 'n/a'}",
        ]
    )

    lines.extend(
        [
            "",
            "## Matching and unmatched counts",
            "",
            "Match order: `(path, name, start_line)` first, then `(path, name)` when",
            "unique on both sides. Nothing is silently dropped.",
            "",
            "| language | matched | unmatched Perfpal | unmatched reference | dead-code excluded |",
            "|---|---:|---:|---:|---:|",
        ]
    )
    for language in LANGUAGES:
        entry = results["per_language"][language]
        lines.append(
            f"| {language} | {entry['matched_count']} | "
            f"{len(entry['unmatched_perfpal'])} | "
            f"{len(entry['unmatched_reference'])} | "
            f"{len(entry['dead_code_excluded'])} |"
        )

    detail_lines: list[str] = []
    for language in LANGUAGES:
        entry = results["per_language"][language]
        for item in entry["unmatched_perfpal"]:
            detail_lines.append(
                f"* {language} unmatched Perfpal: `{item['path']}` "
                f"`{item['name']}` line {item['start_line']} "
                f"({item['reason']})"
            )
        for item in entry["unmatched_reference"]:
            detail_lines.append(
                f"* {language} unmatched reference: `{item['path']}` "
                f"`{item['name']}` line {item['start_line']} "
                f"({item.get('reason', 'no_match')})"
            )
        for item in entry["dead_code_excluded"]:
            detail_lines.append(
                f"* {language} dead-code excluded: `{item['path']}` "
                f"`{item['name']}` line {item['start_line']}"
            )
    if detail_lines:
        lines.extend(["", "### Unmatched details", ""])
        lines.extend(detail_lines)

    lines.extend(
        [
            "",
            "## Limitations",
            "",
            "* The example corpus is human-written. AI-arm stratification and",
            "  per-author grouping are deferred to a later phase.",
            "* C loop bodies are measured from clang's body range, which includes",
            "  braces for compound bodies (matching Perfpal's span convention).",
            "  Clang removes `#if 0` / `#if (0)` code while Perfpal counts it",
            "  syntactically; any such sampled function is flagged",
            "  `dead_code_excluded` and excluded from the comparison rather than",
            "  compared silently.",
            "* Python reference emits module-level `def` / `async def` only.",
            "  Perfpal does not emit nested function records, so nested defs and",
            "  lambdas are skipped when counting an outer function; loops inside",
            "  a nested def are not attributed to the outer function.",
            "* Comprehension generators count as loops; their body span is the",
            "  element expression. The extra corpus (`extra.py` `p_comp`) now",
            "  exercises this path (Perfpal and the Python reference agree).",
            "* The extra corpus adds C 4, Rust 2, Python 2, JavaScript 2 and",
            "  TypeScript 2 loop-bearing records, making the sample exactly 200.",
            "  `dead.c` is excluded from the loop/alloc comparison because clang",
            "  drops its `#if 0` branch; it is used only in the site packet.",
            "* The C reference counts direct calls to `malloc`, `calloc`,",
            "  `realloc`, `aligned_alloc`, `strdup` at loop depth >= 1. It uses",
            "  the clang AST and does not import Perfpal's allocation table.",
            "* Perfpal emits `resource_sites` for every language: allocation",
            "  sites for C/Rust and external-I/O sites for all five. Site",
            "  comparison entries are matched by `(path, line, 0-based column)`,",
            "  so two calls on one line are scored independently; a packet",
            "  candidate with no matching in-loop Perfpal site is `neither`.",
            "  `cfg_gated` remains C-only (`null` for non-C sites).",
            "* Manual-label packets are blind (no Perfpal values). The dead.c",
            "  fixture contains only four call sites (two live, two `#if 0`), so",
            "  the site packet includes four rather than ten dead.c candidates;",
            "  cfg_gated is still exercised in both states.",
            "* Reference outputs for Rust/JavaScript/TypeScript are produced by a",
            "  separate agent; this harness consumes them when present and fails",
            "  the gate when they are missing.",
            "",
        ]
    )
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    repo = default_repo()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=repo)
    parser.add_argument("--sample", type=Path, default=repo / "tools" / "icc" / "sample.json")
    parser.add_argument("--ref-dir", type=Path, default=repo / "tools" / "icc" / "ref")
    parser.add_argument(
        "--labels-dir", type=Path, default=repo / "tools" / "icc" / "labels"
    )
    parser.add_argument("--results-json", type=Path, default=repo / "tools" / "icc" / "results.json")
    parser.add_argument("--results-md", type=Path, default=repo / "tools" / "icc" / "results.md")
    parser.add_argument("--quiet", action="store_true")
    args = parser.parse_args(argv)

    sample = json.loads(args.sample.read_text(encoding="utf-8"))
    results = compute(args.repo.resolve(), sample, args.ref_dir, args.labels_dir)

    args.results_json.parent.mkdir(parents=True, exist_ok=True)
    with args.results_json.open("w", encoding="utf-8") as fh:
        json.dump(results, fh, indent=2, sort_keys=False)
        fh.write("\n")
    args.results_md.write_text(
        render_markdown(results, args.repo.resolve()), encoding="utf-8"
    )

    if not args.quiet:
        print("per-language loop_body_loc ICC / n / depth exact agreement:")
        for language in LANGUAGES:
            entry = results["per_language"][language]
            loc = entry["fields"].get("loop_body_loc", {})
            depth = entry["fields"].get(EXACT_FIELD, {})
            print(
                f"  {language:6s} n={entry['matched_count']:3d} "
                f"loc_icc={fmt(loc.get('icc')):>8s} "
                f"loc_mean_diff={fmt(loc.get('mean_diff')):>8s} "
                f"depth_exact={fmt(depth.get('exact_agreement_percent'), 2):>7s}% "
                f"unmatched={len(entry['unmatched_perfpal'])}"
            )
        gate = results["gate"]["loop_body_loc"]
        print(f"loop_body_loc gate: {'PASS' if gate['pass'] else 'FAIL'}")
        if gate["offending_languages"]:
            print(f"  offending: {', '.join(gate['offending_languages'])}")
        depth_gate = results["gate"]["max_loop_depth"]
        print(f"max_loop_depth gate: {'PASS' if depth_gate['pass'] else 'FAIL'}")
        alloc_gate = results["gate"]["alloc_sites_in_loops"]
        print(
            f"alloc_sites_in_loops gate: "
            f"{'PASS' if alloc_gate['pass'] else 'FAIL'}"
        )
        for language in ALLOC_GATE_LANGUAGES:
            info = alloc_gate["per_language"][language]
            print(
                f"  {language:6s} alloc n={info['n']:3d} "
                f"icc={fmt(info['icc'])}"
            )
        manual = results["manual"]
        if manual["loop"].get("status") == "pending":
            print("manual labels pending (loop_labels.json absent)")
        else:
            print(
                "manual loop gate: "
                f"{'PASS' if manual['loop'].get('pass') else 'FAIL'}"
            )
        if manual["site"].get("status") == "pending":
            print("manual site labels pending (site_labels.json absent)")
        else:
            print("manual site fields (Perfpal prediction vs manual truth):")
            for name in (
                "alloc_sites_in_loops",
                "io_sites_in_loops",
                "cfg_gated_c",
            ):
                field = manual["site"]["site_fields"][name]
                verdict = (
                    "DROPPED"
                    if name in manual["site"]["dropped_fields"]
                    else ("PASS" if field["pass"] else "FAIL")
                )
                print(
                    f"  {name:22s} n={field['n']:3d} "
                    f"TP={field['tp']} FP={field['fp']} "
                    f"FN={field['fn']} TN={field['tn']} "
                    f"kappa={fmt(field['kappa'])} icc={fmt(field['icc'])} "
                    f"precision={fmt(field['precision'])} "
                    f"recall={fmt(field['recall'])} -> {verdict}"
                )
            print(
                "manual site gate: "
                f"{'PASS' if manual['site'].get('pass') else 'FAIL'} "
                f"(dropped: {', '.join(manual['site']['dropped_fields'])})"
            )
        print(
            "freeze_ok: "
            f"{results['freeze_ok']} "
            f"(rule: {results['decision']['rule']})"
        )
        print(f"wrote {args.results_json}")
        print(f"wrote {args.results_md}")

    if results["freeze_ok"] is True:
        return 0
    if results["freeze_ok"] is False:
        return 1
    gates = results["gate"].values()
    return 0 if all(gate.get("pass") is not False for gate in gates) else 1


if __name__ == "__main__":
    raise SystemExit(main())
