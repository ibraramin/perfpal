#!/usr/bin/env python3
"""Runnable acceptance check for the in-report Atlas page (ADR-0014 H1/H2).

Usage:
    python3 sim/report_check.py perfpal-report/report.html

Checks, on a real rendered report, for every (viewport width, device scale
factor) pair in WIDTHS x DPIS:
- no horizontal scroll;
- the Atlas canvas SVG fills its #atlas-canvas container;
- no visible SVG label is clipped by the #atlas-canvas bounds;
- exactly one static <svg> (the Atlas canvas) and one story section;
- exactly three script elements;
- the story player boots, renders nodes, and reaches the summary beat;
- no console errors and no fetch/XHR/websocket/beacon traffic (offline report).

With JavaScript disabled the <noscript> narrative renders, the static canvas,
all static canvas nodes remain visible, the dead player shell is hidden, and
there is no horizontal scroll.

The browser executable defaults to /usr/bin/brave; override with
PERFPAL_BROWSER (for a Playwright-installed Chromium, use its executable path).
Exit code 0 = every assertion passed.
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

from playwright.sync_api import sync_playwright

WIDTHS = (1280, 1440, 1920, 2560, 3072)
DPIS = (1, 2)
VIEWPORT_HEIGHT = 900

STATE_JS = """() => {
  const de = document.documentElement;
  const story = document.querySelector('#atlas-story');
  const stage = document.querySelector('#atlas-story #stage');
  const canvas = document.querySelector('#atlas-canvas');
  const svg = document.querySelector('#atlas-canvas svg');
  const canvasRect = canvas ? canvas.getBoundingClientRect() : null;
  const svgRect = svg ? svg.getBoundingClientRect() : null;
  const isVisible = (el) => {
    const rect = el.getBoundingClientRect();
    const style = getComputedStyle(el);
    return rect.width > 0 && rect.height > 0 &&
      style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
  };
  const canvasNodes = [...document.querySelectorAll('#atlas-canvas .atlas-node')];
  const clipped = [];
  if (canvasRect) {
    for (const text of document.querySelectorAll('#atlas-canvas svg text')) {
      const style = getComputedStyle(text);
      if (style.display === 'none' || style.visibility === 'hidden' || style.opacity === '0') continue;
      const rect = text.getBoundingClientRect();
      if (rect.width === 0 || rect.height === 0) continue;
      const outside =
        rect.left < canvasRect.left - 2 ||
        rect.right > canvasRect.right + 2 ||
        rect.top < canvasRect.top - 2 ||
        rect.bottom > canvasRect.bottom + 2;
      if (outside) {
        clipped.push((text.textContent || '').trim().replace(/\\s+/g, ' ').slice(0, 60));
      }
    }
  }
  const shell = story ? story.querySelector('.canvas-shell') : null;
  return {
    viewport: de.clientWidth,
    scrollWidth: de.scrollWidth,
    canvas_svgs: document.querySelectorAll('#atlas-canvas svg').length,
    story_stage: !!stage,
    stories: document.querySelectorAll('#atlas-story').length,
    scripts: document.querySelectorAll('script').length,
    ready: !!(window.__sim && window.__sim.ready),
    nodes: stage ? stage.querySelectorAll('.node').length : 0,
    canvas_nodes: canvasNodes.length,
    canvas_nodes_visible: canvasNodes.filter(isVisible).length,
    canvas_width: canvasRect ? Math.round(canvasRect.width * 100) / 100 : 0,
    svg_width: svgRect ? Math.round(svgRect.width * 100) / 100 : 0,
    clipped_labels: clipped,
    canvas_height: shell ? Math.round(shell.getBoundingClientRect().height) : 0,
  };
}"""

JS_OFF_STATE_JS = """() => {
  const de = document.documentElement;
  const text = document.body.innerText || '';
  const nodes = [...document.querySelectorAll('#atlas-canvas .atlas-node')];
  const isVisible = (el) => {
    const rect = el.getBoundingClientRect();
    const style = getComputedStyle(el);
    return rect.width > 0 && rect.height > 0 &&
      style.display !== 'none' && style.visibility !== 'hidden' && style.opacity !== '0';
  };
  const shell = document.querySelector('#atlas-story .page');
  return {
    viewport: de.clientWidth,
    scrollWidth: de.scrollWidth,
    svgs: document.querySelectorAll('svg').length,
    noscript_visible: text.includes('The interactive run story needs JavaScript'),
    static_canvas: !!document.querySelector('#atlas-canvas svg'),
    shell_display: shell ? getComputedStyle(shell).display : null,
    canvas_nodes: nodes.length,
    canvas_nodes_visible: nodes.filter(isVisible).length,
  };
}"""


def fail(message: str) -> None:
    raise SystemExit(f"FAIL: {message}")


def key_for(width: int, dpr: int) -> str:
    return f"{width}@{dpr}"


def check_interactive(browser, url: str) -> dict:
    out: dict[str, dict] = {}
    for width in WIDTHS:
        for dpr in DPIS:
            key = key_for(width, dpr)
            page = browser.new_page(
                viewport={"width": width, "height": VIEWPORT_HEIGHT},
                device_scale_factor=dpr,
            )
            errors: list[str] = []
            network: list[str] = []
            page.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
            page.on(
                "request",
                lambda r: network.append(r.url)
                if r.resource_type in ("fetch", "xhr", "websocket", "beacon")
                else None,
            )
            page.goto(url, wait_until="load")
            page.wait_for_timeout(400)
            state = page.evaluate(STATE_JS)
            state["console_errors"] = errors
            state["dynamic_requests"] = network

            if state["scrollWidth"] > state["viewport"]:
                fail(
                    f"{key}: horizontal scroll "
                    f"({state['scrollWidth']} > {state['viewport']})"
                )
            if state["canvas_svgs"] != 1 or not state["story_stage"]:
                fail(f"{key}: expected one Atlas canvas SVG + runtime story stage: {state}")
            if state["stories"] != 1 or state["scripts"] != 3:
                fail(f"{key}: story/script count wrong: {state}")
            if not state["ready"] or state["nodes"] == 0:
                fail(f"{key}: story did not boot/render: {state}")
            if state["canvas_height"] < 300:
                fail(f"{key}: story canvas collapsed ({state['canvas_height']}px)")
            if state["canvas_width"] <= 0 or state["svg_width"] <= 0:
                fail(f"{key}: Atlas canvas/SVG has no width: {state}")
            if state["svg_width"] < state["canvas_width"] - 2:
                fail(
                    f"{key}: Atlas SVG does not fill its container "
                    f"({state['svg_width']} < {state['canvas_width']} - 2)"
                )
            if state["clipped_labels"]:
                fail(f"{key}: clipped Atlas SVG labels: {state['clipped_labels']}")
            if errors:
                fail(f"{key}: console errors: {errors}")
            if network:
                fail(f"{key}: dynamic requests in an offline report: {network}")

            page.evaluate("() => window.__sim.applyFinal()")
            final = page.evaluate("() => window.__sim.state()")
            state["final"] = final
            if final["index"] != final["beats"] - 1:
                fail(f"{key}: final beat not reached: {final}")
            if final["kind"] != "summary":
                fail(f"{key}: final beat is not the summary: {final}")

            # Classic-scrollbar stress: shrink the root by a nominal 15 px
            # scrollbar and prove the full-bleed band follows the body, not
            # the viewport (the old `50vw` breakout overflowed here).
            page.add_style_tag(content="html{width:calc(100% - 15px)}")
            page.wait_for_timeout(100)
            stress = page.evaluate(
                """() => ({
                  body_width: document.body.getBoundingClientRect().width,
                  map_width: document.querySelector('#map').getBoundingClientRect().width,
                  scroll_width: document.documentElement.scrollWidth,
                  client_width: document.documentElement.clientWidth,
                })"""
            )
            state["scrollbar_stress"] = stress
            if stress["map_width"] > stress["body_width"] + 1:
                fail(
                    f"{key}: full-bleed band ignores the body width under a classic scrollbar "
                    f"(map {stress['map_width']} > body {stress['body_width']})"
                )

            out[key] = state
            page.close()
    return out


def check_js_off(browser, url: str) -> dict:
    out: dict[str, dict] = {}
    # The full width × DPR matrix (vector output is DPR-independent, but the
    # acceptance mirrors the interactive matrix exactly).
    targets = [(width, dpr) for width in WIDTHS for dpr in DPIS]
    for width, dpr in targets:
        key = key_for(width, dpr)
        context = browser.new_context(
            viewport={"width": width, "height": VIEWPORT_HEIGHT},
            device_scale_factor=dpr,
            java_script_enabled=False,
            reduced_motion="reduce",
        )
        page = context.new_page()
        page.goto(url, wait_until="load")
        page.wait_for_timeout(200)
        state = page.evaluate(JS_OFF_STATE_JS)
        if not state["noscript_visible"]:
            fail(f"{key} (JS off): noscript narrative is missing")
        if not state["static_canvas"]:
            fail(f"{key} (JS off): the static Atlas canvas is missing")
        if state["shell_display"] != "none":
            fail(f"{key} (JS off): the dead player shell is still visible")
        if state["canvas_nodes"] == 0:
            fail(f"{key} (JS off): the static canvas final frame has no nodes")
        if state["canvas_nodes_visible"] != state["canvas_nodes"]:
            fail(
                f"{key} (JS off): not all static canvas nodes are visible "
                f"({state['canvas_nodes_visible']}/{state['canvas_nodes']})"
            )
        if state["scrollWidth"] > state["viewport"]:
            fail(
                f"{key} (JS off): horizontal scroll "
                f"({state['scrollWidth']} > {state['viewport']})"
            )
        out[key] = state
        context.close()
    return out


def check(url: str) -> dict:
    with sync_playwright() as p:
        browser = p.chromium.launch(
            executable_path=os.environ.get("PERFPAL_BROWSER", "/usr/bin/brave"),
            headless=True,
        )
        try:
            widths = check_interactive(browser, url)
            js_off = check_js_off(browser, url)
        finally:
            browser.close()
    return {"widths": widths, "js_off": js_off}


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: python3 sim/report_check.py <report.html>")
    path = Path(sys.argv[1]).resolve()
    if not path.is_file():
        raise SystemExit(f"missing report: {path}")
    html_text = path.read_text(encoding="utf-8")
    static_svgs = html_text.count("<svg")
    if static_svgs != 1:
        fail(f"static HTML must carry exactly one SVG (the Atlas canvas), got {static_svgs}")
    result = check(path.as_uri())
    result["static_svgs"] = static_svgs
    result["widths_checked"] = [key_for(w, d) for w in WIDTHS for d in DPIS]
    print(json.dumps(result, indent=2))
    print("PASS: report acceptance checks")


if __name__ == "__main__":
    main()
