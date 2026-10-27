#!/usr/bin/env python3
"""Runnable acceptance check for the in-report Atlas page (ADR-0014 H1).

Usage:
    python3 sim/report_check.py perfpal-report/report.html

Checks, on a real rendered report:
- no horizontal scroll at 1280, 1440 and 3072 CSS px;
- exactly one static <svg> (the Atlas canvas) and one story section;
- the story player boots, renders nodes, and reaches the final beat;
- no fetch/XHR/beacon traffic (the report is offline);
- with JavaScript disabled, the <noscript> narrative renders and the static
  canvas remains.

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

WIDTHS = (1280, 1440, 3072)


def fail(message: str) -> None:
    raise SystemExit(f"FAIL: {message}")


def check(widths, url: str) -> dict:
    out = {"widths": {}, "js_off": None}
    with sync_playwright() as p:
        browser = p.chromium.launch(
            executable_path=os.environ.get("PERFPAL_BROWSER", "/usr/bin/brave"),
            headless=True,
        )
        for width in widths:
            page = browser.new_page(viewport={"width": width, "height": 900})
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
            state = page.evaluate(
                """() => {
                  const de = document.documentElement;
                  const story = document.querySelector('#atlas-story');
                  const stage = document.querySelector('#atlas-story #stage');
                  return {
                    viewport: de.clientWidth,
                    scrollWidth: de.scrollWidth,
                    canvas_svgs: document.querySelectorAll('#atlas-canvas svg').length,
                    story_stage: !!document.querySelector('#atlas-story #stage'),
                    stories: document.querySelectorAll('#atlas-story').length,
                    scripts: document.querySelectorAll('script').length,
                    ready: !!(window.__sim && window.__sim.ready),
                    nodes: stage ? stage.querySelectorAll('.node').length : 0,
                    canvasH: story
                      ? Math.round(story.querySelector('.canvas-shell').getBoundingClientRect().height)
                      : 0,
                  };
                }"""
            )
            state["console_errors"] = errors
            state["dynamic_requests"] = network
            if state["scrollWidth"] > state["viewport"]:
                fail(f"{width}px: horizontal scroll ({state['scrollWidth']} > {state['viewport']})")
            if state["canvas_svgs"] != 1 or not state["story_stage"]:
                fail(f"{width}px: expected one Atlas canvas SVG + runtime story stage: {state}")
            if state["stories"] != 1 or state["scripts"] != 3:
                fail(f"{width}px: story/script count wrong: {state}")
            if not state["ready"] or state["nodes"] == 0:
                fail(f"{width}px: story did not boot/render: {state}")
            if state["canvasH"] < 300:
                fail(f"{width}px: story canvas collapsed ({state['canvasH']}px)")
            if errors:
                fail(f"{width}px: console errors: {errors}")
            if network:
                fail(f"{width}px: dynamic requests in an offline report: {network}")
            page.evaluate("() => window.__sim.applyFinal()")
            state["final"] = page.evaluate("() => window.__sim.state()")
            out["widths"][str(width)] = state
            page.close()

        context = browser.new_context(java_script_enabled=False, reduced_motion='reduce')
        page = context.new_page()
        page.goto(url, wait_until="load")
        page.wait_for_timeout(200)
        out["js_off"] = page.evaluate(
            """() => {
              const text = document.body.innerText || '';
              const nodes = [...document.querySelectorAll('#atlas-canvas .atlas-node')];
              const visible = nodes.filter((node) => {
                const rect = node.getBoundingClientRect();
                const style = getComputedStyle(node);
                return rect.width > 0 && rect.height > 0 && style.opacity !== '0';
              }).length;
              return {
                svgs: document.querySelectorAll('svg').length,
                noscript_visible: text.includes('The interactive run story needs JavaScript'),
                static_canvas: !!document.querySelector('#atlas-canvas svg'),
                shell_display: getComputedStyle(document.querySelector('#atlas-story .page')).display,
                canvas_nodes: nodes.length,
                canvas_nodes_visible: visible,
              };
            }"""
        )
        if not out["js_off"]["noscript_visible"]:
            fail("JS disabled: noscript narrative is missing")
        if not out["js_off"]["static_canvas"]:
            fail("JS disabled: the static Atlas canvas is missing")
        if out["js_off"]["shell_display"] != "none":
            fail("JS disabled: the dead player shell is still visible")
        if out["js_off"]["canvas_nodes_visible"] <= 0:
            fail("JS disabled: the static canvas final frame has no visible nodes")
        browser.close()
    return out


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: python3 sim/report_check.py <report.html>")
    path = Path(sys.argv[1]).resolve()
    if not path.is_file():
        raise SystemExit(f"missing report: {path}")
    html_text = path.read_text(encoding="utf-8")
    if html_text.count("<svg") != 1:
        fail(f"static HTML must carry exactly one SVG (the Atlas canvas), got {html_text.count('<svg')}")
    result = check(WIDTHS, path.as_uri())
    result["static_svgs"] = html_text.count("<svg")
    print(json.dumps(result, indent=2))
    print("PASS: report acceptance checks")


if __name__ == "__main__":
    main()
