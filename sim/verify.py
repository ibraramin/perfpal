#!/usr/bin/env python3
"""D1 gate — verify the Atlas run-story prototype and capture evidence.

Serves `sim/` on 127.0.0.1:8768, drives the prototype with Playwright, asserts
the D1 contract (data fidelity, controls, honesty, determinism, no network),
writes 1440x900 keyframes plus `checks.json`, and exits non-zero on any failure.

Validates core-fed sim data (from `atlas-layout.json` + `metrics.json`); it
makes no source changes and captures evidence only.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import socket
import subprocess
import sys
import time
from datetime import date
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[1]
SIM = ROOT / "sim"
EVIDENCE = Path(os.environ.get("SIM_EVIDENCE", ROOT / "docs" / "evidence" / "atlas-sim"))
PORT = int(os.environ.get("SIM_PORT", "8768"))
BASE = f"http://127.0.0.1:{PORT}/"
PINNED_CHROME = Path("/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome")
REAL_CASES = ["hyperfine", "cjson"]

CHECKS: list[dict[str, object]] = []


def check(name: str, ok: object, detail: str = "") -> None:
    passed = bool(ok)
    CHECKS.append({"check": name, "ok": passed, "detail": str(detail)[:500]})
    print(f"{'PASS' if passed else 'FAIL'}  {name}" + (f"  · {detail}" if detail else ""))


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(65536), b""):
            digest.update(block)
    return digest.hexdigest()


def port_free() -> bool:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        return sock.connect_ex(("127.0.0.1", PORT)) != 0


def wait_for_server(timeout: float = 10.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            if sock.connect_ex(("127.0.0.1", PORT)) == 0:
                return True
        time.sleep(0.1)
    return False


def main() -> int:
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    (EVIDENCE / "checks.json").unlink(missing_ok=True)

    # ---- static preflight -------------------------------------------------
    data_files = sorted(SIM.glob("data-*.json"))
    before = {path.name: sha256(path) for path in data_files}
    check("data files present", len(data_files) == 6, ", ".join(before))

    index = json.loads((SIM / "data-index.json").read_text())
    cases = {case["id"]: case for case in index["cases"]}
    check("index has the five D1 cases",
          set(cases) == {"hyperfine", "cjson", "no-forward-path", "empty", "capped"},
          ", ".join(cases))

    for name in ("index.html", "sim.css", "sim.js"):
        text = (SIM / name).read_text()
        external = [
            url for url in re.findall(r"https?://[^\s\"'<>)]*", text)
            if url != "http://www.w3.org/2000/svg"
        ]
        check(f"{name}: no external/network reference", not external, str(external[:3]))
        check(f"{name}: no fetch of a remote origin",
              not re.search(r"fetch\(\s*[\"'](?:https?:)?//", text),
              "local data only")

    payloads = {}
    for case_id in REAL_CASES:
        payload = json.loads((SIM / cases[case_id]["file"]).read_text())
        payloads[case_id] = payload
        check(f"{case_id}: non-empty graph",
              payload["nodes"] and payload["edges"] and payload["trace"])
        check(f"{case_id}: on-path edges match trace",
              sum(1 for e in payload["edges"] if e["onPath"]) == len(payload["trace"]) - 1,
              f'{sum(1 for e in payload["edges"] if e["onPath"])} edges')
        check(f"{case_id}: findings accounting present",
              "findingsTotal" in payload["meta"] and "findingsIncluded" in payload["meta"],
              f'{payload["meta"].get("findingsIncluded")}/{payload["meta"].get("findingsTotal")}')
        check(f"{case_id}: no absolute paths in data",
              "/home/" not in (SIM / cases[case_id]["file"]).read_text())

    if not port_free():
        check(f"port {PORT} is free", False, "another process is listening; stop it and rerun")
        return finish()

    server = subprocess.Popen(
        [sys.executable, "-m", "http.server", str(PORT), "--bind", "127.0.0.1",
         "--directory", str(SIM)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    try:
        check(f"server answers on :{PORT}", wait_for_server())
        if not CHECKS[-1]["ok"]:
            return finish()

        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(
                executable_path=str(PINNED_CHROME) if PINNED_CHROME.exists() else None,
                args=["--no-sandbox"],
            )
            context = browser.new_context(viewport={"width": 1440, "height": 900},
                                          device_scale_factor=1)
            page = context.new_page()
            console_errors: list[str] = []
            page_errors: list[str] = []
            requests: list[str] = []
            page.on("console", lambda msg: console_errors.append(f"{msg.type}: {msg.text}")
                    if msg.type == "error" else None)
            page.on("pageerror", lambda err: page_errors.append(str(err)))
            page.on("request", lambda req: requests.append(req.url))

            response = page.goto(BASE, wait_until="networkidle")
            check("index.html returns 200", response is not None and response.status == 200,
                  str(response.status if response else None))
            page.wait_for_function("() => window.__sim && window.__sim.state().beats > 0",
                                   timeout=10000)

            for case_id in REAL_CASES:
                expected = payloads[case_id]
                page.select_option("#case-select", value=case_id)
                page.wait_for_function(
                    "id => window.__sim.state().caseId === id", arg=case_id, timeout=10000)
                state = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: beats derived", state["beats"] >= 6, f'{state["beats"]} beats')
                check(f"{case_id}: case is not synthetic", state["synthetic"] is False)

                rendered = page.evaluate(
                    "() => document.querySelectorAll('#stage g[role=\"button\"]').length")
                check(f"{case_id}: every node rendered", rendered == len(expected["nodes"]),
                      f"{rendered}/{len(expected['nodes'])}")

                footer = page.inner_text("#canvas-footer")
                check(f"{case_id}: footer matches the render", footer.strip() == expected["meta"]["footer"].strip())

                honesty = page.inner_text("#honesty").lower()
                check(f"{case_id}: honesty label present",
                      "static, inferred" in honesty and "not a runtime trace" in honesty
                      and "profile to confirm" in honesty)

                text = page.inner_text("body")
                time_claim = re.search(r"\b\d+(?:\.\d+)?\s?(?:ms|µs|μs|milliseconds?|seconds?)\b", text, re.I)
                check(f"{case_id}: no time units rendered", time_claim is None,
                      time_claim.group(0) if time_claim else "none")

                kinds = page.evaluate("() => window.__sim.state().kinds")
                check(f"{case_id}: beats include loop and summary",
                      "loop" in kinds and "summary" in kinds, ",".join(sorted(set(kinds))))
                loop_index = kinds.index("loop")
                page.evaluate("(i) => window.__sim.seekIndex(i)", loop_index)
                page.wait_for_timeout(80)
                loop_text = page.inner_text("#narration-body")
                check(f"{case_id}: loop honesty caveat",
                      "inferred trip weight" in loop_text.lower(), loop_text[:90])
                focus = page.evaluate("() => window.__sim.state().focus")
                fn_label = page.evaluate(
                    "(sel) => { const el = document.querySelector(sel); return el ? el.textContent : ''; }",
                    f'#stage g[data-node="{focus["node"]}"] .node-top',
                )
                check(f"{case_id}: function highlight rendered",
                      bool(focus.get("fn")) and fn_label.startswith("▶") and "· L" in fn_label,
                      f'{focus.get("fn")} -> {fn_label}')
                page.evaluate("() => document.activeElement && document.activeElement.blur()")
                page.wait_for_timeout(200)
                loop_shot = EVIDENCE / f"{case_id}-loop.png"
                page.screenshot(path=str(loop_shot), full_page=False)
                check(f"{case_id}: loop keyframe", loop_shot.exists() and loop_shot.stat().st_size > 20000,
                      f"{loop_shot.stat().st_size if loop_shot.exists() else 0} bytes")
                page.evaluate("() => window.__sim.seekIndex(window.__sim.state().beats - 1)")
                page.wait_for_timeout(80)
                final_text = page.inner_text("body")
                final_claim = re.search(
                    r"\b\d+(?:\.\d+)?\s?(?:ms|µs|μs|milliseconds?|seconds?)\b", final_text, re.I)
                check(f"{case_id}: no time units at the final beat", final_claim is None,
                      final_claim.group(0) if final_claim else "none")
                if case_id == "cjson":
                    meta = expected["meta"]
                    check("cjson: findings cap disclosed",
                          f'{meta["findingsTotal"]} findings in the scan' in final_text
                          and f'{meta["findingsIncluded"]} included in this data slice' in final_text,
                          final_text[-180:])

                # ---- transport ------------------------------------------
                page.evaluate("() => window.__sim.seekIndex(0)")
                page.click("#step-forward")
                forward = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: step forward advances", forward["index"] == 1, forward["index"])
                page.click("#step-back")
                back = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: step back returns", back["index"] == 0, back["index"])

                page.click("#play-pause")
                page.wait_for_timeout(350)
                playing = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: play advances the clock",
                      playing["playing"] is True and playing["ms"] > 0, playing["ms"])
                page.click("#play-pause")
                paused = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: pause freezes", paused["playing"] is False)

                page.select_option("#speed", value="4")
                speed = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: speed control", speed["speed"] == 4, speed["speed"])
                page.select_option("#speed", value="1")

                page.evaluate(
                    """() => {
                        const scrub = document.querySelector('#scrub');
                        scrub.value = scrub.max;
                        scrub.dispatchEvent(new Event('input', {bubbles: true}));
                    }"""
                )
                end = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: scrub reaches the final beat",
                      end["index"] == end["beats"] - 1, f'{end["index"]}/{end["beats"] - 1}')

                page.click("#reset")
                reset = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: reset returns to beat 0",
                      reset["index"] == 0 and reset["ms"] == 0, f'{reset["index"]} @ {reset["ms"]}')

                # ---- inspector ------------------------------------------
                target = expected["trace"][0]["path"]
                page.evaluate(
                    """(path) => {
                        const node = document.querySelector(`#stage g[role="button"][data-node="${path}"]`)
                            || document.querySelectorAll('#stage g[role="button"]')[0];
                        node.dispatchEvent(new MouseEvent('click', {bubbles: true}));
                    }""",
                    target,
                )
                selected = page.evaluate("() => window.__sim.state().selected")
                check(f"{case_id}: node select opens the inspector", bool(selected), str(selected))
                check(f"{case_id}: inspector shows the path",
                      expected["trace"][0]["path"] in page.inner_text("#inspector"))
                page.keyboard.press("Escape")
                check(f"{case_id}: Escape closes the inspector",
                      not page.evaluate("() => window.__sim.state().selected"))

                # ---- keyframes ------------------------------------------
                last = page.evaluate("() => window.__sim.state().beats") - 1
                for label, index in (("start", 0), ("mid", last // 2), ("end", last)):
                    page.evaluate("(i) => window.__sim.seekIndex(i)", index)
                    page.evaluate("() => document.activeElement && document.activeElement.blur()")
                    page.wait_for_timeout(250)
                    shot = EVIDENCE / f"{case_id}-{label}.png"
                    page.screenshot(path=str(shot), full_page=False)
                    check(f"{case_id}: {label} keyframe", shot.exists() and shot.stat().st_size > 20000,
                          f"{shot.stat().st_size if shot.exists() else 0} bytes")

                # ---- final frame completeness ---------------------------
                page.evaluate("(i) => window.__sim.seekIndex(i)", last)
                visible = page.evaluate(
                    """() => {
                        const effective = (el) => {
                            let opacity = 1;
                            let node = el;
                            while (node && node !== document.documentElement) {
                                opacity *= parseFloat(getComputedStyle(node).opacity || '1');
                                node = node.parentElement;
                            }
                            return opacity;
                        };
                        return [...document.querySelectorAll('#stage g[role="button"]')]
                            .every(node => effective(node) >= 0.9);
                    }"""
                )
                check(f"{case_id}: final frame shows every node", visible)
                check(f"{case_id}: final frame keeps the honesty label",
                      "STATIC, INFERRED" in page.inner_text("#honesty"))

            # ---- synthetic state demos ----------------------------------
            for case_id in ("no-forward-path", "empty", "capped"):
                page.select_option("#case-select", value=case_id)
                page.wait_for_function(
                    "id => window.__sim.state().caseId === id", arg=case_id, timeout=10000)
                demo = page.evaluate("() => window.__sim.state()")
                check(f"{case_id}: labelled synthetic", demo["synthetic"] is True)
                check(f"{case_id}: stub panel visible", page.is_visible("#stub"))
                check(f"{case_id}: no fabricated nodes",
                      page.evaluate("() => document.querySelectorAll('#stage g[role=\"button\"]').length") == 0)
                page.evaluate("() => window.__sim.seekIndex(window.__sim.state().beats - 2)")
                page.evaluate("() => document.activeElement && document.activeElement.blur()")
                page.wait_for_timeout(200)
                expected_chip = {"no-forward-path": "no forward path", "empty": "empty", "capped": "capped"}
                title = page.inner_text("#narration-title").lower()
                check(f"{case_id}: stop title names the state",
                      expected_chip[case_id] in title, title[:80])
                page.evaluate("() => window.__sim.seekIndex(window.__sim.state().beats - 1)")
                summary = page.inner_text("#narration-body").lower()
                check(f"{case_id}: summary does not claim a scan",
                      "no findings in this state demo" in summary, summary[:90])
                page.evaluate("() => window.__sim.seekIndex(window.__sim.state().beats - 2)")
                shot = EVIDENCE / f"{case_id}.png"
                page.screenshot(path=str(shot), full_page=False)
                check(f"{case_id}: state keyframe", shot.exists() and shot.stat().st_size > 10000,
                      f"{shot.stat().st_size if shot.exists() else 0} bytes")

            # ---- layout fit ---------------------------------------------
            page.select_option("#case-select", value="hyperfine")
            page.wait_for_function("() => window.__sim.state().caseId === 'hyperfine'")
            fit = page.evaluate(
                """() => {
                    const footer = document.querySelector('#canvas-footer').getBoundingClientRect();
                    const scrolling = document.scrollingElement || document.documentElement;
                    return {footerBottom: Math.round(footer.bottom),
                            scrollWidth: scrolling.scrollWidth,
                            clientWidth: scrolling.clientWidth,
                            bodyScrollWidth: document.body.scrollWidth,
                            innerWidth: window.innerWidth};
                }"""
            )
            check("first page fits 1440x900", fit["footerBottom"] <= 900, f'footer {fit["footerBottom"]}')
            check("no horizontal scroll at 1440", fit["scrollWidth"] <= fit["clientWidth"] + 1, str(fit))
            check("no horizontal overflow (body at 1440)",
                  fit["bodyScrollWidth"] <= fit["innerWidth"] + 1, str(fit))
            check("dominant-badge definition present",
                  "dominant rule annotation" in page.inner_text("#badge-def"))
            page.set_viewport_size({"width": 1280, "height": 800})
            page.wait_for_timeout(200)
            narrow = page.evaluate(
                """() => {
                    const s = document.scrollingElement || document.documentElement;
                    return {scrollWidth: s.scrollWidth, clientWidth: s.clientWidth};
                }"""
            )
            check("no horizontal scroll at 1280x800",
                  narrow["scrollWidth"] <= narrow["clientWidth"] + 1, str(narrow))

            # ---- reduced motion ----------------------------------------
            reduced = browser.new_context(viewport={"width": 1440, "height": 900},
                                          reduced_motion="reduce")
            reduced_page = reduced.new_page()
            reduced_requests = []
            reduced_console = []
            reduced_page.on("request", lambda req: reduced_requests.append(req.url))
            reduced_page.on("console", lambda msg: reduced_console.append(msg.text)
                            if msg.type == "error" else None)
            reduced_page.goto(BASE, wait_until="networkidle")
            reduced_page.wait_for_function("() => window.__sim && window.__sim.state().beats > 0")
            reduced_state = reduced_page.evaluate("() => window.__sim.state()")
            check("reduced motion starts at the final frame",
                  reduced_state["index"] == reduced_state["beats"] - 1,
                  f'{reduced_state["index"]}/{reduced_state["beats"] - 1}')
            reduced_external = [url for url in reduced_requests
                                if not (url.startswith(BASE) or url.startswith("data:")
                                        or url.startswith("blob:"))]
            check("reduced motion: no non-local requests", not reduced_external,
                  str(reduced_external[:3]))
            check("reduced motion: no console errors", not reduced_console,
                  "; ".join(reduced_console[:3]))
            reduced.close()

            # ---- no network, no console errors --------------------------
            external = [url for url in requests
                        if not (url.startswith(BASE) or url.startswith("data:")
                                or url.startswith("blob:"))]
            check("no non-local network requests", not external, str(external[:3]))
            check("no console errors", not console_errors, "; ".join(console_errors[:3]))
            check("no page errors", not page_errors, "; ".join(page_errors[:3]))

            context.close()
            browser.close()
    finally:
        server.terminate()
        try:
            server.wait(timeout=5)
        except subprocess.TimeoutExpired:
            server.kill()

    # ---- data must be read-only ----------------------------------------
    after = {path.name: sha256(path) for path in sorted(SIM.glob("data-*.json"))}
    check("data files unchanged by the gate", before == after)

    return finish()


def finish() -> int:
    failed = [entry for entry in CHECKS if not entry["ok"]]
    payload = {
        "date": date.today().isoformat(),
        "viewport": "1440x900",
        "total": len(CHECKS),
        "passed": len(CHECKS) - len(failed),
        "failed": len(failed),
        "checks": CHECKS,
    }
    (EVIDENCE / "checks.json").write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    print(f"\n{payload['passed']}/{payload['total']} checks passed"
          f" -> {EVIDENCE / 'checks.json'}")
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main())
