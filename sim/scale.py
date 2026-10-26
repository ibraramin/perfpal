#!/usr/bin/env python3
"""3k scale gate (S4 F2-F7): run the deterministic 3k fixture end to end.

Checks the story projection and page at scale: F6 (story off vs on), F5
(determinism), F7 (page bytes, first frame, no horizontal scroll), F2 (edge
labels), F3 (unresolved indirection), F4 (honesty) and the served-page DOM
counts. Everything is written under /var/tmp/opencode/ and
docs/evidence/atlas-sim/; no repo data file is created.

Usage:
    python3 sim/scale.py              # reuse the fixture when complete
    ATLAS_3K_FORCE=1 python3 sim/scale.py   # regenerate the fixture
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path
from typing import Any
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]
SIM = ROOT / "sim"
sys.path.insert(0, str(SIM))
import verify as sim_verify  # noqa: E402  (path setup must run first)

from playwright.sync_api import sync_playwright  # noqa: E402

FIXTURE = Path("/var/tmp/opencode/atlas-3k-fixture")
WORK = Path("/var/tmp/opencode/atlas-s4-scale")
SITE = WORK / "site"
EVIDENCE = ROOT / "docs" / "evidence" / "atlas-sim"
BIN = ROOT / "target" / "debug" / "perfpal"
CARGO = Path(os.environ.get("CARGO", str(Path.home() / ".cargo" / "bin" / "cargo")))
PINNED_CHROME = Path(
    "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
)
PAGE_BUDGET = 5_000_000
FIRST_FRAME_BUDGET_MS = 2_000
EDGE_LABELS = {"type-only", "literal-observed", "unresolved"}
REAL_CASE = {"file": "data-3k.json", "id": "3k",
             "label": "3k synthetic · C + Rust · scale", "synthetic": False}

CHECKS: list[tuple[str, bool, str]] = []
PAGE_BYTES = 0
FIRST_FRAME_MAX = 0.0
SCAN_SECONDS: dict[str, float] = {}


def check(name: str, ok: object, detail: str = "") -> bool:
    passed = bool(ok)
    CHECKS.append((name, passed, str(detail)[:900]))
    print(f"assert={'pass' if passed else 'FAIL'} {name}")
    if detail:
        print(f"  {detail}")
    return passed


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_path(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def run(cmd: list[object], timeout: int = 300) -> subprocess.CompletedProcess:
    return subprocess.run(
        [str(part) for part in cmd],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        check=False,
    )


def ensure_fixture() -> bool:
    cmd: list[object] = [sys.executable, SIM / "fixtures" / "gen-3k.py",
                         "--out", FIXTURE]
    if os.environ.get("ATLAS_3K_FORCE") == "1":
        cmd.append("--force")
    proc = run(cmd)
    lines = [line for line in proc.stdout.strip().splitlines() if line]
    ok = proc.returncode == 0 and (FIXTURE / "manifest.json").is_file()
    return check("fixture", ok,
                 " · ".join(lines[-4:]) if lines else proc.stderr[-300:])


def ensure_binary() -> bool:
    if BIN.is_file() and os.access(BIN, os.X_OK):
        return check("binary_present", True, str(BIN))
    proc = run([CARGO, "build", "--locked", "--offline"], timeout=900)
    ok = proc.returncode == 0 and BIN.is_file()
    return check("binary_present", ok,
                 f"built missing binary · exit {proc.returncode}" if ok
                 else proc.stderr[-400:])


def scan(name: str, story: bool) -> tuple[Path, float, bool]:
    out = WORK / name
    shutil.rmtree(out, ignore_errors=True)
    cmd: list[object] = [BIN, FIXTURE, "-o", out]
    if story:
        cmd.append("--story")
    start = time.perf_counter()
    proc = run(cmd, timeout=900)
    elapsed = time.perf_counter() - start
    required = ["report.html", "metrics.json"] + (["atlas-layout.json"] if story else [])
    ok = proc.returncode == 0 and all((out / item).is_file() for item in required)
    check(f"scan_{name}", ok,
          f"{elapsed:.2f}s · exit {proc.returncode}"
          + ("" if ok else f" · {proc.stderr[-300:]}"))
    return out, elapsed, ok


def mask_metrics(doc: dict) -> dict:
    probe = json.loads(json.dumps(doc))
    scan_info = probe.get("scan")
    if isinstance(scan_info, dict):
        for key in ("duration_ms", "started_at", "root_path"):
            scan_info.pop(key, None)
    return probe


def project(layout: Path, metrics: Path, out: Path) -> bool:
    proc = run([sys.executable, SIM / "from-layout.py",
                "--layout", layout, "--metrics", metrics,
                "--out", out, "--case", "3k"])
    ok = proc.returncode == 0 and out.is_file()
    return ok if ok else bool(check(f"project_{out.name}", False, proc.stderr[-400:]))


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def wait_for_server(base: str, timeout: float = 15.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with urlopen(base + "index.html", timeout=0.5) as response:
                if response.status == 200:
                    return True
        except OSError:
            time.sleep(0.15)
    return False


def warm_server(base: str) -> None:
    """Prime the HTTP server + OS file cache before timing the page frame.

    Browser contexts stay fresh for every measured load; this only removes the
    one-time server/disk cost of serving the 3k data file.
    """
    for name in ("index.html", "sim.css", "sim.js", "data-3k.json", "data-index.json"):
        try:
            with urlopen(base + name, timeout=2) as response:
                response.read()
        except OSError:
            pass


def first_paragraph(markdown: str) -> str:
    lines: list[str] = []
    for raw in markdown.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            if lines:
                break
            continue
        lines.append(line)
    return " ".join(lines)


def honesty_hits(text: str) -> list[str]:
    hits = [pattern for pattern in sim_verify.FORBIDDEN_CLAIMS
            if re.search(pattern, text, re.I)]
    if re.search(sim_verify.TIME_UNITS, text, re.I):
        hits.append("time-unit")
    hits.extend(match.group(0) for match in sim_verify.NEGATED_RUNTIME.finditer(text))
    hits.extend(sim_verify.unnegated_hits(text, sim_verify.NEGATABLE_CLAIMS))
    return hits


def scroll_info(page) -> dict:
    return page.evaluate(
        """() => ({
            doc: document.documentElement.scrollWidth,
            body: document.body.scrollWidth,
            inner: window.innerWidth,
        })"""
    )


def s4_info(page) -> dict:
    return page.evaluate(
        """() => {
            const section = document.querySelector('#s4-detail');
            const visible = !!section && !section.hidden
                && getComputedStyle(section).display !== 'none';
            return {
                visible,
                scenes: document.querySelectorAll('#s4-scene-grid [data-scene]').length,
                queue: document.querySelectorAll('#s4-queue [data-profile-row]').length,
            };
        }"""
    )


def inspect_indirection(page, path: str, data_node: dict, label: str) -> None:
    page.evaluate(
        """(path) => {
            const node = document.querySelector(`#stage g[role="button"][data-node="${path}"]`);
            if (node) node.dispatchEvent(new MouseEvent('click', {bubbles: true}));
        }""",
        path,
    )
    page.wait_for_timeout(80)
    selected = page.evaluate("() => window.__sim.state().selected")
    inspector = page.inner_text("#inspector")
    rows = page.evaluate(
        """() => [...document.querySelectorAll('#inspector [data-unresolved]')]
            .map((li) => ({
                line: li.getAttribute('data-unresolved-line'),
                text: li.textContent,
            }))"""
    )
    count = int(data_node.get("unresolvedCalls", {}).get("count", 0))
    check(f"F3_{label}_selectable", selected == path, f"selected={selected}")
    check(f"F3_{label}_inspector_rows",
          "UNRESOLVED CALLS" in inspector and 0 < len(rows) <= 6,
          f"{len(rows)}/{count} rows")
    check(f"F3_{label}_no_guess_note",
          "never guessed" in inspector and all("→" not in row["text"] for row in rows),
          inspector[-180:])
    if count > len(rows):
        check(f"F3_{label}_cap_disclosed",
              f"showing {len(rows)} of {count:,}" in inspector,
              f"{len(rows)}/{count}")
    page.keyboard.press("Escape")


def main() -> int:
    global PAGE_BYTES, FIRST_FRAME_MAX
    WORK.mkdir(parents=True, exist_ok=True)
    EVIDENCE.mkdir(parents=True, exist_ok=True)
    print("=== 3k scale gate (F2-F7) ===")

    if not ensure_fixture():
        return finish()
    if not ensure_binary():
        return finish()

    shutil.rmtree(WORK / "off", ignore_errors=True)
    shutil.rmtree(WORK / "on", ignore_errors=True)
    shutil.rmtree(WORK / "on2", ignore_errors=True)
    shutil.rmtree(SITE, ignore_errors=True)

    off, off_s, off_ok = scan("off", story=False)
    on, on_s, on_ok = scan("on", story=True)
    on2, on2_s, on2_ok = scan("on2", story=True)
    SCAN_SECONDS.update({"off": off_s, "on": on_s, "on2": on2_s})
    check("scan_times_recorded", off_ok and on_ok and on2_ok,
          f"off={off_s:.2f}s on={on_s:.2f}s on2={on2_s:.2f}s "
          f"story_delta={on_s - off_s:+.2f}s")

    if not (off_ok and on_ok and on2_ok):
        return finish()

    # ---- F6: story output is additive --------------------------------
    off_report = sha256_path(off / "report.html")
    on_report = sha256_path(on / "report.html")
    check("F6_report_identical", off_report == on_report,
          f"off={off_report[:16]} on={on_report[:16]}")
    off_metrics = mask_metrics(json.loads((off / "metrics.json").read_text()))
    on_metrics = mask_metrics(json.loads((on / "metrics.json").read_text()))
    check("F6_metrics_content_equal", off_metrics == on_metrics,
          "masked scan.duration_ms/started_at/root_path")

    # ---- F5: layout + projection determinism -------------------------
    layout_hash = sha256_path(on / "atlas-layout.json")
    layout2_hash = sha256_path(on2 / "atlas-layout.json")
    check("F5_layout_identical", layout_hash == layout2_hash,
          f"on={layout_hash[:16]} on2={layout2_hash[:16]}")
    data_path = WORK / "data-3k.json"
    data2_path = WORK / "data-3k2.json"
    projected = project(on / "atlas-layout.json", on / "metrics.json", data_path)
    projected = project(on2 / "atlas-layout.json", on2 / "metrics.json", data2_path) and projected
    check("F5_projection_written", projected, f"{data_path} {data2_path}")
    if projected:
        data_hash = sha256_path(data_path)
        data2_hash = sha256_path(data2_path)
        check("F5_projection_identical", data_hash == data2_hash,
              f"data={data_hash[:16]} data2={data2_hash[:16]}")
    else:
        return finish()

    data = json.loads(data_path.read_text())
    layout = json.loads((on / "atlas-layout.json").read_text())
    manifest = json.loads((FIXTURE / "manifest.json").read_text())
    readme = (FIXTURE / "README.md").read_text(encoding="utf-8")

    # ---- temp served page --------------------------------------------
    SITE.mkdir(parents=True, exist_ok=True)
    for name in ("index.html", "sim.css", "sim.js"):
        shutil.copy2(SIM / name, SITE / name)
    shutil.copy2(data_path, SITE / "data-3k.json")
    (SITE / "data-index.json").write_text(
        json.dumps({"cases": [REAL_CASE]}, indent=1) + "\n", encoding="utf-8"
    )
    page_files = [SITE / "index.html", SITE / "sim.css", SITE / "sim.js",
                  SITE / "data-3k.json"]
    page_bytes = sum(path.stat().st_size for path in page_files)
    PAGE_BYTES = page_bytes
    check("F7_page_bytes", page_bytes <= PAGE_BUDGET,
          f"{page_bytes:,} bytes (budget {PAGE_BUDGET:,}) · "
          + " · ".join(f"{path.name}={path.stat().st_size:,}" for path in page_files))

    port = free_port()
    base = f"http://127.0.0.1:{port}/"
    server = subprocess.Popen(
        [sys.executable, "-m", "http.server", str(port), "--bind", "127.0.0.1",
         "--directory", str(SITE)],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    browser = None
    contexts: list[Any] = []
    console_errors: list[str] = []
    page_errors: list[str] = []
    requests: list[str] = []

    def attach(page) -> None:
        page.on("console", lambda msg: console_errors.append(f"{msg.type}: {msg.text}")
                if msg.type == "error" else None)
        page.on("pageerror", lambda err: page_errors.append(str(err)))
        page.on("request", lambda req: requests.append(req.url))

    try:
        if not check("server_ready", wait_for_server(base), base):
            return finish()
        warm_server(base)

        with sync_playwright() as playwright:
            launch_kwargs: dict[str, Any] = {"args": ["--no-sandbox"]}
            if PINNED_CHROME.exists():
                launch_kwargs["executable_path"] = str(PINNED_CHROME)
            browser = playwright.chromium.launch(**launch_kwargs)

            # F7 cold first frame: 3 independent loads.
            cold = []
            for index in range(3):
                context = browser.new_context(
                    viewport={"width": 1440, "height": 900}, device_scale_factor=1)
                contexts.append(context)
                page = context.new_page()
                attach(page)
                start = time.perf_counter()
                page.goto(base, wait_until="domcontentloaded")
                page.wait_for_function(
                    "() => window.__sim && window.__sim.ready === true", timeout=15000)
                page.evaluate(
                    "() => new Promise((resolve) => requestAnimationFrame("
                    "() => requestAnimationFrame(resolve)))")
                elapsed_ms = (time.perf_counter() - start) * 1000
                cold.append((page, elapsed_ms))
                print(f"first_frame[{index + 1}]={elapsed_ms:.0f}ms")
            max_ms = max(ms for _, ms in cold)
            FIRST_FRAME_MAX = max_ms
            check("F7_first_frame", max_ms < FIRST_FRAME_BUDGET_MS,
                  f"loads={[round(ms) for _, ms in cold]}ms · max={max_ms:.0f}ms "
                  f"(budget {FIRST_FRAME_BUDGET_MS}ms)")

            evidence_page = cold[2][0]
            check("ready_true",
                  evidence_page.evaluate("() => window.__sim.ready === true"))

            # F5 page: two independent loads, final-frame DOM hash.
            dom_hashes = []
            for page, _ in cold[1:]:
                page.evaluate("() => window.__sim.applyFinal()")
                page.wait_for_timeout(250)
                dom_hashes.append(
                    hashlib.sha256(sim_verify.stage_dom(page).encode()).hexdigest())
            check("F5_page_dom_identical", dom_hashes[0] == dom_hashes[1],
                  f"hash={dom_hashes[0][:16]}")

            # F7 viewport / S4 checks at 1440.
            info_1440 = scroll_info(evidence_page)
            check("F7_no_hscroll_1440",
                  info_1440["doc"] <= info_1440["inner"] + 1
                  and info_1440["body"] <= info_1440["inner"] + 1,
                  str(info_1440))
            detail_1440 = s4_info(evidence_page)
            check("F7_s4_detail_1440",
                  detail_1440["visible"]
                  and detail_1440["scenes"] == 24
                  and detail_1440["queue"] == 12,
                  str(detail_1440))

            # F2: data edges + DOM labels.
            data_edges = data["edges"]
            layout_edges = layout["edges"]
            layout_pairs = {(edge["from"], edge["to"]) for edge in layout_edges}
            data_pairs = {(edge["from"], edge["to"]) for edge in data_edges}
            unresolved_data = [edge for edge in data_edges
                               if edge.get("label") == "unresolved"]
            unresolved_layout = [edge for edge in layout_edges
                                 if edge.get("label") == "unresolved"]
            check("F2_edge_labels",
                  all(edge.get("label") in EDGE_LABELS for edge in data_edges),
                  f"{len(data_edges)} edges")
            check("F2_layout_edges_represented",
                  layout_pairs == data_pairs and len(layout_edges) == len(data_edges),
                  f"layout={len(layout_edges)} data={len(data_edges)}")
            check("F2_unresolved_payloads_null",
                  all(edge.get("payload") is None for edge in unresolved_data)
                  and all(edge.get("payload") is None for edge in unresolved_layout),
                  f"{len(unresolved_data)} unresolved data edges")
            dom_edges = evidence_page.evaluate(
                """() => {
                    const labels = [...document.querySelectorAll('#stage text.edge-label')];
                    return {
                        paths: document.querySelectorAll('#stage path.edge').length,
                        labels: labels.length,
                        bad: labels.filter((el) => !['type-only', 'literal-observed',
                            'unresolved'].includes(el.getAttribute('data-label'))).length,
                        unresolvedPaths: document.querySelectorAll(
                            '#stage path.edge.edge-unresolved').length,
                        unresolvedLabels: labels.filter(
                            (el) => el.getAttribute('data-label') === 'unresolved').length,
                    };
                }"""
            )
            check("F2_dom_labels",
                  dom_edges["paths"] == len(data_edges)
                  and dom_edges["labels"] == len(data_edges)
                  and dom_edges["bad"] == 0
                  and dom_edges["unresolvedPaths"] == len(unresolved_data)
                  and dom_edges["unresolvedLabels"] == len(unresolved_data),
                  str(dom_edges))

            # F3: engineered indirection in the inspector.
            c_path = manifest["engineered"]["c_function_pointer"]
            rust_path = manifest["engineered"]["rust_dyn"]
            drawn = {node["path"] for node in layout["nodes"]}
            check("F3_engineered_drawn",
                  c_path in drawn and rust_path in drawn,
                  f"c={c_path} rust={rust_path} drawn={len(drawn)}")
            for path, label in ((c_path, "c"), (rust_path, "rust")):
                calls = [call for call in layout["flow"]["calls"]
                         if call.get("from") == path]
                unresolved_calls = [call for call in calls if call.get("unresolved")]
                check(f"F3_{label}_core_unresolved",
                      bool(unresolved_calls)
                      and all(call.get("to") is None and call.get("to_id") is None
                              for call in unresolved_calls),
                      f"{len(unresolved_calls)} unresolved / {len(calls)} calls; "
                      f"callees={sorted({call.get('callee') for call in unresolved_calls})}")
                data_node = next(node for node in data["nodes"] if node["id"] == path)
                inspect_indirection(evidence_page, path, data_node, label)

            # F1/F7 evidence frames (inspector closed, final frame).
            evidence_page.evaluate("() => document.activeElement && document.activeElement.blur()")
            evidence_page.wait_for_timeout(300)
            final_shot = EVIDENCE / "scale-final.png"
            full_shot = EVIDENCE / "scale-full.png"
            evidence_page.screenshot(path=str(final_shot), full_page=False)
            evidence_page.screenshot(path=str(full_shot), full_page=True)
            check("F7_scale_final_png",
                  final_shot.exists() and final_shot.stat().st_size > 20_000,
                  f"{final_shot.stat().st_size if final_shot.exists() else 0} bytes")
            check("F7_scale_full_png",
                  full_shot.exists() and full_shot.stat().st_size > 20_000,
                  f"{full_shot.stat().st_size if full_shot.exists() else 0} bytes")

            # 1280x800 viewport: no horizontal scroll, S4 visible, final frame.
            narrow_context = browser.new_context(
                viewport={"width": 1280, "height": 800}, device_scale_factor=1)
            contexts.append(narrow_context)
            narrow_page = narrow_context.new_page()
            attach(narrow_page)
            narrow_page.goto(base, wait_until="load")
            narrow_page.wait_for_function(
                "() => window.__sim && window.__sim.ready === true", timeout=15000)
            narrow_page.evaluate("() => window.__sim.applyFinal()")
            narrow_page.wait_for_timeout(250)
            info_1280 = scroll_info(narrow_page)
            check("F7_no_hscroll_1280",
                  info_1280["doc"] <= info_1280["inner"] + 1
                  and info_1280["body"] <= info_1280["inner"] + 1,
                  str(info_1280))
            detail_1280 = s4_info(narrow_page)
            check("F7_s4_detail_1280",
                  detail_1280["visible"]
                  and detail_1280["scenes"] == 24
                  and detail_1280["queue"] == 12,
                  str(detail_1280))
            narrow_page.evaluate("() => document.activeElement && document.activeElement.blur()")
            narrow_page.wait_for_timeout(250)
            narrow_shot = EVIDENCE / "scale-1280.png"
            narrow_page.screenshot(path=str(narrow_shot), full_page=False)
            check("F7_scale_1280_png",
                  narrow_shot.exists() and narrow_shot.stat().st_size > 20_000,
                  f"{narrow_shot.stat().st_size if narrow_shot.exists() else 0} bytes")

            # F4: honesty in the data file and the final rendered frame.
            final_text = evidence_page.inner_text("body")
            data_text = data_path.read_text()
            data_hits = honesty_hits(data_text)
            final_hits = honesty_hits(final_text)
            check("F4_data_honesty", not data_hits, ", ".join(sorted(set(data_hits))[:6]))
            check("F4_final_honesty", not final_hits, ", ".join(sorted(set(final_hits))[:6]))
            purpose = data.get("purpose") or {}
            expected_purpose = first_paragraph(readme)
            expected_forbidden = bool(honesty_hits(expected_purpose))
            if expected_forbidden or purpose.get("kind") == "withheld":
                purpose_ok = purpose.get("kind") == "withheld"
            else:
                purpose_ok = (
                    purpose.get("kind") == "readme"
                    and purpose.get("source") == "README.md"
                    and purpose.get("text") == expected_purpose
                )
            check("F4_purpose_readme", purpose_ok,
                  f"kind={purpose.get('kind')} source={purpose.get('source')} "
                  f"text={str(purpose.get('text'))[:100]!r}")

            # ready + transport sanity.
            check("ready_after_load",
                  evidence_page.evaluate("() => window.__sim.ready === true"))
            beats = evidence_page.evaluate("() => window.__sim.state().beats")
            check("beats_present", beats >= 6, f"{beats} beats")

            # Network + console cleanliness (all pages).
            external = [url for url in requests
                        if not (url.startswith(base) or url.startswith("data:")
                                or url.startswith("blob:"))]
            check("no_external_requests", not external, str(external[:3]))
            check("no_console_errors", not console_errors, "; ".join(console_errors[:3]))
            check("no_page_errors", not page_errors, "; ".join(page_errors[:3]))

            for context in contexts:
                context.close()
            browser.close()
            browser = None
    finally:
        if browser is not None:
            try:
                browser.close()
            except Exception:  # pragma: no cover - best-effort teardown
                pass
        server.terminate()
        try:
            server.wait(timeout=5)
        except subprocess.TimeoutExpired:
            server.kill()

    return finish()


def finish() -> int:
    passed = sum(1 for _, ok, _ in CHECKS if ok)
    failed = len(CHECKS) - passed
    groups = {}
    for name, ok, _ in CHECKS:
        if len(name) >= 2 and name[:2] in {"F2", "F3", "F4", "F5", "F6"}:
            group = groups.setdefault(name[:2], [0, 0])
            group[0 if ok else 1] += 1
    f26 = ", ".join(f"{key}={'pass' if value[1] == 0 else 'FAIL'}"
                    for key, value in sorted(groups.items()))
    scans = " ".join(f"{name}={value:.2f}s" for name, value in SCAN_SECONDS.items())
    try:
        projected = json.loads((WORK / "data-3k.json").read_text())
        scene_meta = projected.get("sceneMeta") or {}
        profile_meta = projected.get("profileMeta") or {}
        omitted = scene_meta.get("omitted") or {}
        print(
            "detail: "
            f"scenes={scene_meta.get('shownScenes')}/{scene_meta.get('totalScenes')} "
            f"omitted={omitted.get('scenes')}/{omitted.get('files')} "
            f"source_files={scene_meta.get('totalFiles')} "
            f"queue={profile_meta.get('shown')}/{profile_meta.get('candidateTotal')} "
            f"ranked={profile_meta.get('totalRanked')}"
        )
    except (OSError, ValueError, TypeError):
        pass
    print(f"summary: page_bytes={PAGE_BYTES:,} first_frame_max_ms={FIRST_FRAME_MAX:.0f} "
          f"scans={scans or 'n/a'} F2-F6=[{f26 or 'n/a'}] failed={failed}")
    print("F8 deferred: founder A2 session; not claimed")
    print(f"\nasserts={passed}/{len(CHECKS)} passed · failed={failed}")
    print("3k scale gate: " + ("PASS" if failed == 0 else "FAIL"))
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
