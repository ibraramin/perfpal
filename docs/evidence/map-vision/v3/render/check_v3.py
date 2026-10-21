#!/usr/bin/env python3
"""V3 idiom-A/B evidence gate.

Verifies the committed V3 artifacts are complete, consistent and holdout-clean.
Exit 0 = pass; any assertion failure = non-zero. No product code is touched.
"""

import hashlib
import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[5]  # .../docs/evidence/map-vision/v3/render -> repo root
V3 = ROOT / "docs" / "evidence" / "map-vision" / "v3"
TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
REPOS = ["tmux", "hyperfine", "cJSON"]
RENDER_SLOTS = {
    "kv1": ["brief", "cards"],
    "kv2": ["bars", "matrix"],
    "kv3": ["bars", "treemap"],
}
READ_SLOTS = {
    "KV1": ["brief", "cards"],
    "KV2": ["bars", "matrix"],
    "KV3": ["bars", "treemap"],
}
EXPECTED_SHA_PREFIX = {
    "tmux": "62fd2e23",
    "hyperfine": "5735135b",
    "cJSON": "99736e9f",
}
EXPECTED_RANKS = {
    ("tmux", "Q1"): (210, 7545, False),
    ("tmux", "Q2"): (1, 216, True),
    ("tmux", "Q3"): (811, 3189, False),
    ("hyperfine", "Q1"): (197, 348, False),
    ("hyperfine", "Q2"): (4, 53, True),
    ("hyperfine", "Q3"): (66, 353, False),
    ("cJSON", "Q1"): (35, 837, False),
    ("cJSON", "Q2"): (1, 101, True),
    ("cJSON", "Q3"): (58, 1055, False),
}

failures = []


def check(condition, message):
    if not condition:
        failures.append(message)


def png_size(path):
    raw = path.read_bytes()[:24]
    check(raw[:8] == b"\x89PNG\r\n\x1a\n", f"not a PNG: {path}")
    return struct.unpack(">II", raw[16:24])


def main():
    check((V3 / "prereg.md").is_file(), "missing prereg.md")
    manifest = json.loads((V3 / "manifest.json").read_text(encoding="utf-8"))
    reads = json.loads((V3 / "reads.json").read_text(encoding="utf-8"))
    evidence = (ROOT / "docs" / "evidence" / "map-idiom-ab.md").read_text(encoding="utf-8")

    # --- manifest <-> reads binding ---
    manifest_sha = hashlib.sha256((V3 / "manifest.json").read_bytes()).hexdigest()
    check(reads["protocol"]["manifest_sha256"] == manifest_sha,
          "reads.protocol.manifest_sha256 does not match the committed manifest.json")

    # --- manifest: data SHAs + schema ---
    for repo in REPOS:
        entry = manifest["data"][repo]
        actual = hashlib.sha256(
            (TEST_ROOT / f"out-i5b-{repo}" / "metrics.json").read_bytes()
        ).hexdigest()
        check(actual == entry["sha256"], f"{repo}: manifest data sha mismatch")
        check(actual.startswith(EXPECTED_SHA_PREFIX[repo]), f"{repo}: unexpected data sha")
        check(str(entry["schema_version"]) == "1.6", f"{repo}: schema != 1.6")

    # --- exactly 18 renders, expected grid, 1440 wide, dims match ---
    renders = manifest["renders"]
    check(len(renders) == 18, f"expected 18 renders, found {len(renders)}")
    seen = set()
    for render in renders:
        repo, slot, idiom = render["repo"], render["slot"], render["idiom"]
        seen.add((repo, slot, idiom))
        check(repo in REPOS, f"unexpected repo in render: {repo}")
        check(slot in RENDER_SLOTS and idiom in RENDER_SLOTS[slot],
              f"unexpected slot/idiom: {slot}/{idiom}")
        png = Path(render["png"])
        check(png.is_file(), f"missing render: {png}")
        if png.is_file():
            width, height = png_size(png)
            check(width == 1440, f"{png}: width {width} != 1440")
            check(list(render["png_dims"]) == [width, height], f"{png}: manifest dims mismatch")
            check(bool(render["fits_900"]) == (height <= 920),
                  f"{png}: fits_900 flag inconsistent with {height}px")
        check(not ({"django", "postgres"} & set(png.parts)), f"holdout leak: {png}")
    expected_renders = {
        (r, s, i) for r in REPOS for s, idioms in RENDER_SLOTS.items() for i in idioms
    }
    check(seen == expected_renders,
          f"render grid mismatch: missing {expected_renders - seen}, extra {seen - expected_renders}")
    fits = sum(1 for render in renders if render["fits_900"])
    check(fits == 15, f"expected 15/18 renders to fit 900px, found {fits}")

    # --- reads: 18 singles + 9 pairs, full coverage, required fields ---
    singles = reads["singles"]
    pairs = reads["pairs"]
    check(len(singles) == 18, f"expected 18 singles, found {len(singles)}")
    check(len(pairs) == 9, f"expected 9 pairs, found {len(pairs)}")
    single_keys = {(s["repo"], s["slot"], s["idiom"]) for s in singles}
    expected_singles = {
        (r, s, i) for r in REPOS for s, idioms in READ_SLOTS.items() for i in idioms
    }
    check(single_keys == expected_singles, "singles do not cover the 6 candidates x 3 repos")
    for single in singles:
        for key in ("question_answered", "action", "blockers", "confidence", "desc"):
            check(key in single, f"single {single.get('id')} missing {key}")
        check(single.get("question_match") is True,
              f"single {single.get('id')} did not match the question")
    pair_keys = {(p["repo"], p["slot"]) for p in pairs}
    check(pair_keys == {(r, s) for r in REPOS for s in READ_SLOTS},
          "pairs do not cover 3 slots x 3 repos")
    for pair in pairs:
        check(pair["winner"] in {"brief", "cards", "bars", "matrix", "treemap", "tie"},
              f"pair {pair.get('id')} bad winner {pair['winner']!r}")
        check(set(pair["order"]) == {"A", "B"}, f"pair {pair.get('id')} missing A/B order")
        check(pair["winner"] in pair["order"].values(),
              f"pair {pair.get('id')} winner not in the shown order")

    # --- decisions recorded, one per slot, each with a change trigger ---
    for slot in READ_SLOTS:
        check(f"### {slot} —" in evidence, f"evidence missing decision for {slot}")
    check(evidence.count("What would change it:") == 3,
          "expected 3 'What would change it' triggers")
    for winner in ("text-first ranked brief", "horizontal dependents bars", "horizontal cost bars"):
        check(winner in evidence, f"evidence missing winner: {winner}")
    check("Proposed V2 amendment" in evidence and "production-first demotion to KV2" in evidence,
          "evidence missing the KV2 amendment proposal")
    check("django/postgres were not rendered" in evidence,
          "evidence missing holdout statement")

    # --- specs mirrored ---
    feature = (ROOT / "docs" / "specs" / "map-intelligibility.md").read_text(encoding="utf-8")
    mirror = (ROOT / "docs" / "specs" / "maintainer-map.md").read_text(encoding="utf-8")
    for doc, name in ((feature, "map-intelligibility"), (mirror, "maintainer-map")):
        check("V3 idiom decision" in doc, f"{name} missing V3 decision")
        check("text-first ranked" in doc, f"{name} missing KV1 decision")
        check("pending founder ratification" in doc, f"{name} missing KV2 amendment status")

    # --- recall ranks reproducer artifact ---
    recall_path = V3 / "recall_ranks.json"
    check(recall_path.is_file(), "missing recall_ranks.json")
    if recall_path.is_file():
        ranks = json.loads(recall_path.read_text(encoding="utf-8"))["ranks"]
        check(len(ranks) == 9, f"expected 9 recall ranks, found {len(ranks)}")
        got = {(r["repo"], r["question"]): (r["rank"], r["total"], r["inline"]) for r in ranks}
        check(got == EXPECTED_RANKS,
              f"recall ranks drifted from the evidence doc: {got}")

    # --- holdout: no scan input, render, or read references django/postgres ---
    holdout_hits = []
    for path in list(V3.rglob("*.png")) + list(V3.rglob("reads.json")) + list(V3.rglob("manifest.json")):
        if "django" in path.name.lower() or "postgres" in path.name.lower():
            holdout_hits.append(str(path))
    check(not holdout_hits, f"holdout leak in V3 artifacts: {holdout_hits}")
    check(manifest.get("holdout") == {"django": "not-rendered", "postgres": "not-rendered"},
          "manifest holdout block missing/altered")

    if failures:
        print("V3 GATE FAILED:")
        for failure in failures:
            print(f"  - {failure}")
        return 1
    print("V3 GATE PASSED: 18 renders (1440px), 18 singles + 9 pairs, 3 decisions + triggers, specs mirrored, no holdout leak")
    return 0


if __name__ == "__main__":
    sys.exit(main())
