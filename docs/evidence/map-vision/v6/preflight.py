#!/usr/bin/env python3
"""V6 holdout preflight — STRUCTURAL ONLY.

Checks that each holdout report loads and that the three key views exist with
at least one row each. It deliberately does NOT screenshot, extract row labels,
or interpret any content: the holdout is released only for the founder's timed
test. Output: preflight.json (hashes, scan duration, DOM counts, parse health).
"""

from __future__ import annotations

import hashlib
import json
import time
from pathlib import Path

from playwright.sync_api import sync_playwright

EXE = "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"
TEST_ROOT = Path("/home/shafin/Opencode/perfpal-test")
OUT = Path(__file__).resolve().parent / "preflight.json"
REPOS = ["django", "postgres"]  # timed test = postgres; django = untimed smoke
VIEWPORT = {"width": 1440, "height": 900}

DOM_CHECK = """
() => {
  const q = (sel) => document.querySelectorAll(sel).length;
  const first = document.querySelector('#map .kv');
  const caption = document.querySelector('.kv2 .kv-counts');
  return {
    kv1: q('.kv1'),
    kv2: q('.kv2'),
    kv3: q('.kv3'),
    kv1_rows: q('.kv1 li'),
    kv2_rows: q('.kv2 .kv-brow'),
    kv2_demoted_rows: q('.kv2 .kv-demoted'),
    kv3_rows: q('.kv3 .kv-brow'),
    first_kv_class: first ? String(first.className) : '',
    kv2_caption_has_demoted: caption ? /demoted/.test(caption.textContent || '') : false,
  };
}
"""


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    results: dict[str, object] = {}
    failures: list[str] = []
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(executable_path=EXE)
        for repo in REPOS:
            out_dir = TEST_ROOT / f"out-v6-{repo}"
            report = out_dir / "report.html"
            metrics = out_dir / "metrics.json"
            if not report.exists() or not metrics.exists():
                raise SystemExit(f"FATAL: missing report/metrics for {repo}")
            page = browser.new_page(
                viewport={"width": VIEWPORT["width"], "height": VIEWPORT["height"]},
                device_scale_factor=1,
            )
            console_errors: list[str] = []
            page.on("pageerror", lambda error: console_errors.append(str(error)))
            started = time.time()
            page.goto(report.as_uri())
            page.wait_for_load_state("load")
            dom = page.evaluate(DOM_CHECK)
            load_ms = round((time.time() - started) * 1000)
            page.close()

            doc = json.loads(metrics.read_text())
            parse = doc.get("parse_health_summary") or {}
            files = doc.get("files") or []
            findings = doc.get("findings") or []
            checks = {
                "one_kv1": dom["kv1"] == 1,
                "one_kv2": dom["kv2"] == 1,
                "one_kv3": dom["kv3"] == 1,
                "kv1_has_rows": dom["kv1_rows"] > 0,
                "kv2_has_rows": dom["kv2_rows"] > 0,
                "kv3_has_rows": dom["kv3_rows"] > 0,
                "first_kv_is_kv1": "kv1" in dom["first_kv_class"],
                "kv2_caption_discloses_demotion": dom["kv2_caption_has_demoted"],
                "no_page_errors": not console_errors,
            }
            if not all(checks.values()):
                failures.append(repo)
            results[repo] = {
                "checks": checks,
                "dom": dom,
                "findings": len(findings),
                "files": len(files),
                "page_errors": console_errors,
                "load_ms": load_ms,
                "metrics_sha256": sha256(metrics),
                "parse_health_summary": parse,
                "report_sha256": sha256(report),
                "repo_commit": {
                    "django": "d430084",
                    "postgres": "856a659",
                }[repo],
            }
        browser.close()

    payload = {
        "_comment": (
            "V6 holdout preflight — structural only; no screenshots, no key-view "
            "content extraction, no interpretation. Holdout released for the "
            "founder timed test only."
        ),
        "generated": "2026-10-22",
        "repos": results,
        "timed_repo": "postgres",
        "verdict": "PASS" if not failures else f"FAIL: {', '.join(failures)}",
        "viewport": VIEWPORT,
    }
    OUT.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"verdict": payload["verdict"], "repos": results}, indent=2))
    return 0 if not failures else 1


if __name__ == "__main__":
    raise SystemExit(main())
