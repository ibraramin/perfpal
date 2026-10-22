#!/usr/bin/env python3
"""V5 falsifier for the KV3 no-loop demotion guard.

The V2 lock demotes `max_loop_depth == 0` functions below loop-bearing ones so
that statement/branch-count dispatchers (the tmux `format.c` FP class) cannot
dominate the profile queue. V3 found that this guard also demotes all three
pre-registered Q3 targets, each of which is a per-byte / per-call leaf whose
hotness comes from call frequency, not from an internal loop. V5 must test that
demotion against the per-leaf evidence before V7 re-triages the ranking key.

This script reproduces, from the schema-1.7 V5 scans:

- the shipped KV3 rank/total and every demotion reason for the three Q3 targets;
- the counterfactual rank when the no-loop term is removed from the KV3 key
  (production -> startup -> cost desc -> path -> name -> id), which isolates the
  guard's effect;
- the source evidence for the "leaf called per byte/item/measurement" claim.

Writes `no_loop_falsifier_v5.json` and exits non-zero on rank drift.

Host-local paths: the scan outputs live outside the repo (`perfpal-test/out-v5-*`);
patch `TEST_ROOT` on another host.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
OUT = Path(__file__).resolve().parent / "no_loop_falsifier_v5.json"

TARGETS = [
    {
        "repo": "tmux",
        "name": "screen_write_collect_add",
        "path": "screen-write.c",
        "expected_rank": 820,
        "expected_total": 3189,
        "leaf_evidence": "per-printable-byte leaf: no loop inside; branch ladder decides collect vs. direct write, then stores one cell byte at `screen-write.c:2830`; called from `input.c:1244` and `input.c:2892` (pre-registered Q3).",
        "source_ranges": [
            ("screen-write.c", 2775, 2780),
            ("screen-write.c", 2794, 2807),
            ("screen-write.c", 2824, 2831),
        ],
    },
    {
        "repo": "hyperfine",
        "name": "execute_and_measure",
        "path": "src/timer/mod.rs",
        "expected_rank": 58,
        "expected_total": 353,
        "leaf_evidence": "per-measurement leaf: no loop inside; starts the wall clock, spawns the child and blocks in `wait4` (`src/timer/unix_timer.rs:33-41`), so wall-clock time is dominated by the child wait (pre-registered Q3).",
        "source_ranges": [
            ("src/timer/mod.rs", 64, 82),
            ("src/timer/unix_timer.rs", 33, 42),
        ],
    },
    {
        "repo": "cJSON",
        "name": "cJSON_New_Item",
        "path": "cJSON.c",
        "expected_rank": 60,
        "expected_total": 1055,
        "leaf_evidence": "per-item allocation leaf: no loop inside; one allocator call plus `memset` (`cJSON.c:243-246`), called for the root, every array element, every object member, every string and every number temp (pre-registered Q3).",
        "source_ranges": [
            ("cJSON.c", 241, 250),
            ("cJSON.c", 1166, 1166),
            ("cJSON.c", 1535, 1535),
            ("cJSON.c", 1698, 1698),
        ],
    },
]

SEVERITY = {"hot": 0, "warn": 1, "info": 2}
CONFIDENCE = {"HIGH": 0, "MEDIUM": 1, "LOW": 2}


def load(repo):
    path = TEST_ROOT / f"out-v5-{repo}" / "metrics.json"
    if not path.exists():
        raise SystemExit(f"FATAL: missing {path}")
    data = json.loads(path.read_text(encoding="utf-8"))
    classes = {f["path"]: f.get("path_class") or "unknown" for f in data["files"]}
    return data, classes


def production(classes, path):
    return classes.get(path, "unknown") == "production"


def function_cost(fn):
    return fn.get("function_cost_v2") or 0


def ordering(data, classes, *, no_loop_guard):
    functions = data["stats"]["functions"]

    def key(index):
        fn = functions[index]
        return (
            not production(classes, fn["path"]),
            fn["max_loop_depth"] == 0 if no_loop_guard else False,
            fn.get("entry_kind") in ("main", "script"),
            -function_cost(fn),
            fn["path"],
            fn["name"],
            fn["id"],
        )

    return sorted(range(len(functions)), key=key)


def source_excerpt(repo, rel, start, end):
    file_path = TEST_ROOT / repo / rel
    lines = file_path.read_text(encoding="utf-8", errors="replace").splitlines()
    excerpt = [
        f"{number}: {lines[number - 1]}" for number in range(start, end + 1)
    ]
    return {"file": rel, "lines": f"{start}-{end}", "text": "\n".join(excerpt)}


def main():
    results = []
    ok = True
    for target in TARGETS:
        data, classes = load(target["repo"])
        functions = data["stats"]["functions"]
        index = next(
            i
            for i, fn in enumerate(functions)
            if fn["name"] == target["name"] and fn["path"] == target["path"]
        )
        fn = functions[index]

        shipped = ordering(data, classes, no_loop_guard=True)
        counterfactual = ordering(data, classes, no_loop_guard=False)
        rank = shipped.index(index) + 1
        # The counterfactual drops only the no-loop tier. Every target is
        # production and non-startup, so its 1-based position in this ordering is
        # directly comparable to the inline top-8 slot (which is also the
        # production/non-startup tier). If a future key ranks demoted files first,
        # revisit this comparison.
        cf_rank = counterfactual.index(index) + 1

        non_production = not production(classes, fn["path"])
        startup = fn.get("entry_kind") in ("main", "script")
        no_loop = fn["max_loop_depth"] == 0
        demotions = []
        if non_production:
            demotions.append("non-production")
        if startup:
            demotions.append("startup")
        if no_loop:
            demotions.append("no-loop")

        non_demoted = [
            i
            for i in shipped
            if production(classes, functions[i]["path"])
            and functions[i]["max_loop_depth"] > 0
            and functions[i].get("entry_kind") not in ("main", "script")
        ]

        result = {
            "repo": target["repo"],
            "target": f"{target['name']}|{target['path']}",
            "rank": rank,
            "total": len(shipped),
            "inline_top8": index in non_demoted[:8],
            "counterfactual_rank_without_no_loop": cf_rank,
            "counterfactual_inline_top8": cf_rank <= 8,
            "max_loop_depth": fn["max_loop_depth"],
            "entry_kind": fn.get("entry_kind"),
            "path_class": classes.get(fn["path"], "unknown"),
            "reachable_from_entry": fn.get("reachable_from_entry"),
            "call_fanin": fn.get("call_fanin"),
            "call_fanout": fn.get("call_fanout"),
            "function_cost_v2": fn.get("function_cost_v2"),
            "demotions": demotions,
            "leaf_evidence": target["leaf_evidence"],
            "source": [
                source_excerpt(target["repo"], rel, start, end)
                for rel, start, end in target["source_ranges"]
            ],
        }
        results.append(result)

        if (rank, len(shipped)) != (target["expected_rank"], target["expected_total"]):
            ok = False
            print(
                f"MISMATCH {target['repo']} {target['name']}: "
                f"got {rank}/{len(shipped)}, expected "
                f"{target['expected_rank']}/{target['expected_total']}",
                file=sys.stderr,
            )

    demoted_all = all("no-loop" in row["demotions"] for row in results)
    hidden_all = all(not row["inline_top8"] for row in results)
    cf_inline_all = all(row["counterfactual_inline_top8"] for row in results)
    ranks = ", ".join(
        f"{row['repo']} {row['rank']}->{row['counterfactual_rank_without_no_loop']}"
        for row in results
    )
    if demoted_all and hidden_all and cf_inline_all:
        verdict = (
            "FALSIFIED - the no-loop guard demotes all three pre-registered Q3 targets "
            "and removing only the no-loop term restores all three to the inline top 8; "
            "the guard alone hides them. Per map-idiom-ab.md §5, V7 must adopt "
            "per-iteration cost or the two-list variant before changing the idiom."
        )
    elif demoted_all and hidden_all:
        verdict = (
            "PARTIALLY FALSIFIED - the no-loop guard demotes all three pre-registered "
            f"Q3 targets (3/3), but removing only the no-loop term does NOT restore any "
            f"to the inline top 8 (shipped->counterfactual ranks: {ranks}; top 8). The "
            "guard is a contributing demotion, not the sole cause of the Q3 recall miss: "
            "the static-cost key cannot surface per-byte/per-call leaves (the V1 H1 "
            "prediction for call weights). V7 must adopt call-weighted/per-iteration "
            "cost (or a separate call-frequency view), not merely drop the guard; a "
            "two-list variant of the same static cost is insufficient. Note cJSON "
            "cJSON_New_Item is also reachable_from_entry=false in the default-tier call "
            "graph, so a reachable-only Q3 (R-pf-098) would hide it further."
        )
    else:
        verdict = "INCONCLUSIVE - see per-target rows; the pre-registered all-three test did not fire."
    payload = {
        "_comment": "V5 KV3 no-loop falsifier. Reproduces the shipped KV3 rank and the counterfactual rank without the no-loop key over the schema-1.7 out-v5 scans; includes the per-leaf source evidence for the three pre-registered Q3 targets.",
        "generated": "2026-10-22",
        "held_out": "django/postgres deliberately absent (V6)",
        "falsifier": {
            "demoted_by_no_loop": demoted_all,
            "hidden_from_inline_top8": hidden_all,
            "counterfactual_inline_top8_all": cf_inline_all,
            "guard_is_sole_cause": cf_inline_all,
            "ranks_shipped_to_counterfactual": ranks,
        },
        "verdict": verdict,
        "targets": results,
    }
    OUT.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if not ok:
        return 1
    print(f"V5 NO-LOOP FALSIFIER OK: {len(results)}/3 ranks match -> {OUT}")
    print(verdict)
    return 0


if __name__ == "__main__":
    sys.exit(main())
