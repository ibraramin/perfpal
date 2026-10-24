#!/usr/bin/env python3
"""Atlas v2 step 13 — V8 A1/strip rerun on the reworked page.

Renders the eight synthetic fixture states from `step13/html/` at 1440x900,
asserts the step-13 visual-gate DOM invariants, and writes the final frames +
weight-panel crops for the blind reads.

Run from the repo root:
    PERFPAL_ATLAS_VISION_DIR=docs/evidence/atlas-visual/v2/step13/html \
        cargo test --locked --test atlas_vision -- --ignored
    python3 docs/evidence/atlas-visual/v2/render_step13_states.py
"""

from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

OUT = Path(__file__).resolve().parent
ROOT = OUT / "step13"
HTML_DIR = ROOT / "html"
sys.path.insert(0, str(OUT))
import render_step13 as gate  # noqa: E402

W, H = 1440, 900
STATES = [
    "application",
    "library",
    "no-entry",
    "empty",
    "all-test",
    "capped",
    "partial-parse",
    "single-file",
]
RENDERED_FLOORS = {
    "node name": 9.9,
    "node meta": 8.4,
    "node badge": 7.9,
    "column header": 10.4,
    "aggregate": 8.4,
    "hull label": 9.0,
}
# The capped fixture is the V8-disclosed framing exception: its depth-1
# column can exceed the first screen by design (plan step 10 note). It is
# measured and reported, not tuned in this gate.
CAPPED_EXEMPT = {"capped"}

CANVAS = r"""
() => {
  const renderedSize = (el) => {
    const size = parseFloat(getComputedStyle(el).fontSize);
    if (!el.ownerSVGElement) return size;
    const ctm = el.getScreenCTM();
    return ctm ? size * ctm.a : size;
  };
  const minOf = (nodes) => nodes.reduce((acc, el) => {
    const size = renderedSize(el);
    return acc === null ? size : Math.min(acc, size);
  }, null);
  return {
    minName: minOf([...document.querySelectorAll('.atlas-node .atlas-node-name')]),
    minMeta: minOf([...document.querySelectorAll('.atlas-node .atlas-node-meta')]),
    minBadge: minOf([...document.querySelectorAll('.atlas-node .atlas-node-badge')]),
    minColumn: minOf([...document.querySelectorAll('.atlas-column-header')]),
    minAggregate: minOf([...document.querySelectorAll('.atlas-aggregate text')]),
    minHull: minOf([...document.querySelectorAll('.atlas-hull text')]),
    maxEdgeLabel: [...document.querySelectorAll('.atlas-edge-label')]
      .reduce((acc, el) => Math.max(acc,
        Math.round(el.getBoundingClientRect().width * 10) / 10), 0),
  };
}
"""


def png_size(path: Path) -> tuple[int, int]:
    head = path.read_bytes()[:24]
    if head[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"FATAL: {path} is not a PNG")
    return struct.unpack(">II", head[16:24])


def main() -> int:
    missing = [s for s in STATES if not (HTML_DIR / f"{s}.html").is_file()]
    if missing:
        failures = [f"missing fixture HTML: {', '.join(missing)}"]
        (ROOT / "states.json").write_text(
            json.dumps({"status": "FAIL", "failures": failures}, indent=2) + "\n",
            encoding="utf-8",
        )
        print(f"FATAL: {failures[0]}", file=sys.stderr)
        return 1

    measured: dict[str, dict] = {}
    failures: list[str] = []
    with sync_playwright() as pw:
        browser = gate.launch_browser(pw)
        try:
            for state in STATES:
                context = browser.new_context(
                    viewport={"width": W, "height": H}, device_scale_factor=1
                )
                try:
                    page = context.new_page()
                    page.emulate_media(reduced_motion="reduce")
                    page.goto((HTML_DIR / f"{state}.html").as_uri() + "#map")
                    page.wait_for_selector("#map")
                    page.wait_for_selector('[data-atlas-block="weight-chart"]')
                    page.wait_for_load_state("load")
                    gate.scroll_map_to_top(page)
                    data = page.evaluate(gate.DOM)
                    canvas_text = page.evaluate(CANVAS)
                finally:
                    context.close()
                data["canvasText"] = canvas_text
                data["state"] = state
                measured[state] = data

                state_failures = gate.check_1440(
                    state, data, state not in CAPPED_EXEMPT
                )
                for metric, floor in RENDERED_FLOORS.items():
                    key = {
                        "node name": "minName",
                        "node meta": "minMeta",
                        "node badge": "minBadge",
                        "column header": "minColumn",
                        "aggregate": "minAggregate",
                        "hull label": "minHull",
                    }[metric]
                    value = canvas_text[key]
                    if value is None:
                        # Only the node name is mandatory when the state draws
                        # nodes; aggregates/hulls/columns legitimately vary.
                        if metric == "node name" and state not in {
                            "empty", "no-entry", "all-test"
                        }:
                            state_failures.append(
                                f"{state}: node name missing although the state draws nodes"
                            )
                    elif value < floor:
                        state_failures.append(
                            f"{state}: {metric} rendered size {value:.1f}px < {floor}px"
                        )
                if canvas_text["maxEdgeLabel"] > 32:
                    state_failures.append(
                        f"{state}: edge label {canvas_text['maxEdgeLabel']}px wide (>32px)"
                    )
                failures.extend(state_failures)
                print(
                    f"{state:<14} canvas {data['canvas']['y']:.0f}-"
                    f"{data['canvas']['bottom']:.0f}  map {data['map']['h']:.0f}  "
                    f"rail-cards {data['counts']['railCards']:>2}  "
                    f"name {canvas_text['minName']}  meta {canvas_text['minMeta']}  "
                    f"badge {canvas_text['minBadge']}  "
                    f"defs c{data['counts']['structuralCaptions']}/u{data['counts']['units']}"
                    f"/l{data['counts']['legends']}/f{data['counts']['footers']}  "
                    f"lead {len(data['leadingDot'])}  clip {len(data['clippedContainers'])}"
                    f"  over {len(data['nodeTextOverflow'])}"
                )
        finally:
            browser.close()

    states_record = {
        "status": "PASS" if not failures else "FAIL",
        "failures": failures,
        "states": {
            state: {
                "canvas_bottom": data["canvas"]["bottom"],
                "canvas_footer_bottom": data["canvasFooter"]["bottom"],
                "map_height": data["map"]["h"],
                "rail_cards": data["counts"]["railCards"],
                "definitions": data["definitions"],
                "canvas_text": data["canvasText"],
                "node_text_overflow": data["nodeTextOverflow"],
                "clipped": data["clippedContainers"],
                "leading_dot": data["leadingDot"],
            }
            for state, data in measured.items()
        },
    }
    if failures:
        states_record["measured_full"] = measured
    (ROOT / "states.json").write_text(
        json.dumps(states_record, indent=2) + "\n", encoding="utf-8"
    )
    if failures:
        print("\nSTEP-13 STATE PREFLIGHT FAILED; no screenshots written:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    records = []
    with sync_playwright() as pw:
        browser = gate.launch_browser(pw)
        try:
            for state in STATES:
                context = browser.new_context(
                    viewport={"width": W, "height": H}, device_scale_factor=1
                )
                try:
                    page = context.new_page()
                    page.emulate_media(reduced_motion="reduce")
                    page.goto((HTML_DIR / f"{state}.html").as_uri() + "#map")
                    page.wait_for_selector("#map")
                    page.wait_for_selector('[data-atlas-block="weight-chart"]')
                    page.wait_for_load_state("load")
                    gate.scroll_map_to_top(page)
                    final = ROOT / f"{state}-final.png"
                    weight = ROOT / f"{state}-weight.png"
                    page.screenshot(path=str(final), animations="disabled")
                    page.locator(
                        '[data-atlas-block="weight-chart"]'
                    ).screenshot(path=str(weight), animations="disabled")
                finally:
                    context.close()
                fw, fh = png_size(final)
                if (fw, fh) != (W, H):
                    raise SystemExit(
                        f"{state}: final frame {fw}x{fh}, expected {W}x{H}"
                    )
                ww, wh = png_size(weight)
                data = measured[state]
                records.append(
                    {
                        "state": state,
                        "html": f"html/{state}.html",
                        "html_sha256": gate.sha256(HTML_DIR / f"{state}.html"),
                        "final": {"file": final.name, "width": fw, "height": fh,
                                  "sha256": gate.sha256(final)},
                        "weight": {"file": weight.name, "width": ww, "height": wh,
                                   "sha256": gate.sha256(weight)},
                        "measurements": {
                            "map_height": data["map"]["h"],
                            "canvas_bottom": data["canvas"]["bottom"],
                            "canvas_footer_bottom": data["canvasFooter"]["bottom"],
                            "page_scroll_width": data["page"]["scrollWidth"],
                            "page_client_width": data["page"]["clientWidth"],
                            "rail_cards": data["counts"]["railCards"],
                            "definitions": data["definitions"],
                            "counts": data["counts"],
                            "min_text": data["textLeaves"]["minSize"],
                            "leading_dot": data["leadingDot"],
                            "clipped": data["clippedContainers"],
                            "node_overflow": data["nodeTextOverflow"],
                            "duplicate_ids": data["duplicateIds"],
                            "canvas_text": data["canvasText"],
                            "keymetrics": data["keymetrics"],
                            "footer": data["footerText"],
                            "header": data["header"],
                        },
                    }
                )
                print(f"{state:<14} final {fw}x{fh}  weight {ww}x{wh}")
        finally:
            browser.close()

    manifest = {
        "_comment": "Step-13 rerun of the V8 8-state matrix on the reworked page.",
        "protocol": (
            "Final frames captured at 1440x900 with #map at the viewport top, "
            "prefers-reduced-motion + animations disabled. Two blind A1 reads "
            "per state on the final frame; two blind strip reads per state on "
            "the weight-chart crop; one image per task; no source/siblings/"
            "expected answer."
        ),
        "viewport": {"width": W, "height": H},
        "device_scale_factor": 1,
        "capped_exemption": (
            "capped is the V8-disclosed framing exception: the canvas/footer "
            "fold is measured and disclosed, not asserted; its A1 reads still "
            "must pass."
        ),
        "states": records,
    }
    (ROOT / "manifest.json").write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )
    print(f"\nPASS — manifest {ROOT / 'manifest.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
