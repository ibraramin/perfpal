#!/usr/bin/env python3
"""A2 verification — animation enabled, CSS-only pause, reduced motion, print.

Runs against the two A2 holdout pages served on 127.0.0.1 (postgres: application
class; memchr crate payload: library class). It does not rescan and does not read
source: it measures the rendered page at 1440x900 and writes PNGs + a2-verify.json
under ``a2/``. Fail-closed: any failed check exits 1.

Run from the repo root with the two servers up:
    python3 docs/evidence/map-vision/v8/a2_verify.py
"""

from __future__ import annotations

import hashlib
import json
import struct
import sys
from datetime import datetime, timezone
from pathlib import Path

from playwright.sync_api import sync_playwright

OUT = Path(__file__).resolve().parent / "a2"
PINNED = Path(
    "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
)
VIEWPORT = {"width": 1440, "height": 900}
CLASSES = {
    "postgres": {
        "url": "http://127.0.0.1:8771/report.html#map",
        "scan": Path("/tmp/opencode/a2-postgres"),
        "log": Path("/tmp/opencode/a2-postgres-scan.log"),
    },
    "memchr": {
        "url": "http://127.0.0.1:8772/report.html#map",
        "scan": Path("/tmp/opencode/a2-memchr"),
        "log": Path("/tmp/opencode/a2-memchr-scan.log"),
    },
}

STATE = r"""
() => {
  const canvas = document.querySelector('#atlas-canvas');
  const label = document.querySelector('label[for="atlas-pause"]');
  const input = document.querySelector('#atlas-pause');
  const anims = document.getAnimations();
  const cssAnimated = canvas
    ? [...canvas.querySelectorAll('*')].filter(
        (el) => getComputedStyle(el).animationName !== 'none'
      )
    : [];
  const column = canvas ? canvas.querySelector('.atlas-column') : null;
  const map = document.querySelector('#map');
  const r = (el) => {
    if (!el) return null;
    const b = el.getBoundingClientRect();
    return {
      top: Math.round(b.top * 10) / 10,
      bottom: Math.round(b.bottom * 10) / 10,
      height: Math.round(b.height * 10) / 10,
    };
  };
  return {
    animations_total: anims.length,
    animations_running: anims.filter((a) => a.playState === 'running').length,
    animations_paused: anims.filter((a) => a.playState === 'paused').length,
    animations_finished: anims.filter((a) => a.playState === 'finished').length,
    max_current_time_ms: anims.reduce(
      (m, a) => Math.max(m, Number(a.currentTime) || 0), 0
    ),
    column_opacity: column ? getComputedStyle(column).opacity : null,
    css_animated: cssAnimated.length,
    css_animated_names: [...new Set(cssAnimated.map(
      (el) => getComputedStyle(el).animationName
    ))],
    css_play_states: [...new Set(cssAnimated.map(
      (el) => getComputedStyle(el).animationPlayState
    ))],
    nodes: canvas ? canvas.querySelectorAll('.atlas-node').length : 0,
    edges: canvas ? canvas.querySelectorAll('.atlas-edge').length : 0,
    traces: canvas ? canvas.querySelectorAll('.atlas-trace').length : 0,
    hidden_by_opacity: canvas
      ? [...canvas.querySelectorAll('.atlas-node, .atlas-edge, .atlas-trace')]
          .filter((el) => Number(getComputedStyle(el).opacity) === 0).length
      : 0,
    pause_input: Boolean(input),
    pause_label: label ? (label.textContent || '').trim() : null,
    pause_checked: input ? input.checked : null,
    pause_label_display: label ? getComputedStyle(label).display : null,
    canvas_rect: r(canvas),
    map_rect: r(map),
    map_background: map ? getComputedStyle(map).backgroundColor : null,
    horizontal_scroll:
      document.documentElement.scrollWidth > document.documentElement.clientWidth,
    scroll_width: document.documentElement.scrollWidth,
    client_width: document.documentElement.clientWidth,
    url: location.href,
  };
}
"""


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def png_size(path: Path) -> tuple[int, int]:
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        raise SystemExit(f"not a PNG: {path}")
    return struct.unpack(">II", data[16:24])


def launch(pw):
    if PINNED.is_file():
        return pw.chromium.launch(executable_path=str(PINNED))
    return pw.chromium.launch()


def capture(page, path: Path) -> dict:
    page.screenshot(path=str(path), animations="allow")
    return {"path": path.name, "sha256": sha256(path), "size": png_size(path)}


def run_class(pw, name: str, spec: dict) -> dict:
    browser = launch(pw)
    report = spec["scan"] / "report.html"
    result: dict = {
        "url": spec["url"],
        "report": str(report),
        "report_sha256": sha256(report) if report.is_file() else None,
        "checks": {},
        "states": {},
    }

    # --- 1. Animation enabled (default browser settings) -------------------
    page = browser.new_page(viewport=VIEWPORT)
    page.goto(spec["url"], wait_until="load")
    page.wait_for_timeout(250)
    initial = page.evaluate(STATE)
    first = capture(page, OUT / f"{name}-initial.png")
    page.wait_for_timeout(1800)
    mid = page.evaluate(STATE)
    second = capture(page, OUT / f"{name}-animating.png")
    page.wait_for_timeout(6500)
    final = page.evaluate(STATE)
    final_frame = capture(page, OUT / f"{name}-final.png")
    page.close()
    result["states"]["initial"] = initial
    result["states"]["mid_animation"] = mid
    result["states"]["final"] = final
    result["frames"] = {
        "initial": first,
        "animating": second,
        "final": final_frame,
    }
    result["checks"]["animation_enabled"] = (
        initial["css_animated"] > 0
        and initial["animations_total"] > 0
        and mid["max_current_time_ms"] > initial["max_current_time_ms"]
        and float(mid["column_opacity"] or 0)
        > float(initial["column_opacity"] or 0)
    )
    result["checks"]["final_frame_complete"] = (
        final["animations_running"] == 0
        and final["hidden_by_opacity"] == 0
        and final["nodes"] > 0
        and final["edges"] > 0
    )

    # --- 2. CSS-only pause freezes the timeline (fresh page, mid-animation) --
    page = browser.new_page(viewport=VIEWPORT)
    page.goto(spec["url"], wait_until="load")
    page.wait_for_timeout(2900)
    page.locator("#atlas-pause").check(force=True)
    page.wait_for_timeout(350)
    paused = page.evaluate(STATE)
    pause_one = capture(page, OUT / f"{name}-pause-1.png")
    page.wait_for_timeout(900)
    pause_two = capture(page, OUT / f"{name}-pause-2.png")
    page.locator("#atlas-pause").focus()
    focus_id = page.evaluate(
        "() => document.activeElement && document.activeElement.id"
    )
    result["checks"]["pause_focusable"] = focus_id == "atlas-pause"
    result["states"]["paused"] = paused
    result["frames"]["pause_1"] = pause_one
    result["frames"]["pause_2"] = pause_two
    result["checks"]["pause_control_present"] = bool(
        paused["pause_input"] and paused["pause_label"]
    )
    result["checks"]["pause_freezes"] = (
        paused["pause_checked"] is True
        and paused["css_animated"] > 0
        and all(state == "paused" for state in paused["css_play_states"])
        and pause_one["sha256"] == pause_two["sha256"]
    )
    page.locator("#atlas-pause").uncheck(force=True)
    page.wait_for_timeout(200)
    resumed = page.evaluate(STATE)
    result["states"]["resumed"] = resumed
    result["checks"]["pause_resumes"] = all(
        state == "running" for state in resumed["css_play_states"]
    )
    page.close()

    # --- 3. Reduced motion: final frame, no animation ---------------------
    reduced = browser.new_page(viewport=VIEWPORT, reduced_motion="reduce")
    reduced.goto(spec["url"], wait_until="load")
    reduced.wait_for_timeout(500)
    reduced_state = reduced.evaluate(STATE)
    reduced_frame = capture(reduced, OUT / f"{name}-reduced-motion.png")
    result["states"]["reduced_motion"] = reduced_state
    result["frames"]["reduced_motion"] = reduced_frame
    result["checks"]["reduced_motion_final"] = (
        reduced_state["css_animated"] == 0
        and reduced_state["nodes"] > 0
        and reduced_state["edges"] > 0
        and reduced_state["hidden_by_opacity"] == 0
    )
    reduced.close()

    # --- 4. Print: final frame, no animation, pause hidden ----------------
    printed = browser.new_page(viewport=VIEWPORT)
    printed.goto(spec["url"], wait_until="load")
    printed.emulate_media(media="print")
    printed.wait_for_timeout(500)
    print_state = printed.evaluate(STATE)
    print_frame = capture(printed, OUT / f"{name}-print.png")
    result["states"]["print"] = print_state
    result["frames"]["print"] = print_frame
    result["checks"]["print_final"] = (
        print_state["css_animated"] == 0
        and print_state["nodes"] > 0
        and print_state["edges"] > 0
        and print_state["hidden_by_opacity"] == 0
        and print_state["pause_label_display"] == "none"
        and print_state["map_background"] in ("rgb(255, 255, 255)", "rgba(0, 0, 0, 0)")
    )
    printed.close()

    # --- 5. 1280x800 geometry (layout contract; recorded, not asserted) ---
    narrow = browser.new_page(viewport={"width": 1280, "height": 800})
    narrow.goto(spec["url"], wait_until="load")
    narrow.wait_for_timeout(500)
    narrow_state = narrow.evaluate(STATE)
    narrow_frame = capture(narrow, OUT / f"{name}-1280x800.png")
    narrow.close()
    result["states"]["1280x800"] = narrow_state
    result["frames"]["1280x800"] = narrow_frame

    browser.close()

    result["checks"]["geometry"] = {
        "canvas_bottom": initial["canvas_rect"]["bottom"],
        "canvas_height": initial["canvas_rect"]["height"],
        "map_height": initial["map_rect"]["height"],
        "horizontal_scroll": initial["horizontal_scroll"],
        "note": "recorded, not asserted: the capped-canvas residual is A2-owned",
    }
    result["checks"]["geometry_1280"] = {
        "canvas_bottom": narrow_state["canvas_rect"]["bottom"],
        "map_height": narrow_state["map_rect"]["height"],
        "horizontal_scroll": narrow_state["horizontal_scroll"],
        "scroll_width": narrow_state["scroll_width"],
        "client_width": narrow_state["client_width"],
    }
    result["pass"] = all(
        value
        for key, value in result["checks"].items()
        if not key.startswith("geometry")
    )
    return result


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    results = {
        "generated": datetime.now(timezone.utc).isoformat(),
        "viewport": f"{VIEWPORT['width']}x{VIEWPORT['height']}",
        "chromium": str(PINNED),
        "classes": {},
    }
    with sync_playwright() as pw:
        for name, spec in CLASSES.items():
            print(f"== {name} :: {spec['url']}")
            result = run_class(pw, name, spec)
            results["classes"][name] = result
            for check, value in result["checks"].items():
                if isinstance(value, bool):
                    print(f"  {'PASS' if value else 'FAIL'}  {check}")
                else:
                    print(f"  info  {check}: {value}")
            print(f"  class pass: {result['pass']}")

    results["pass"] = all(item["pass"] for item in results["classes"].values())
    (OUT / "a2-verify.json").write_text(json.dumps(results, indent=2) + "\n")
    print(f"\nverdict: {'PASS' if results['pass'] else 'FAIL'}")
    print(f"wrote {OUT / 'a2-verify.json'}")
    return 0 if results["pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
