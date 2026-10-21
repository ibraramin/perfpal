import json, os, math
from playwright.sync_api import sync_playwright

OUT = "/home/shafin/Opencode/Perfpal/docs/evidence/map-vision/v0"
REPOS = ["tmux", "hyperfine", "cJSON"]
STEMS = ["include-graph", "layer-file", "layer-entry", "component-coupling",
         "hot-components", "layer-workspace", "layer-bands", "layer-findings"]
EXE = "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"

manifest = {"generated": "2026-10-21", "viewport": "1440x900 @ 100% zoom (device_scale_factor=1)",
            "note": "standalone stem PNGs use natural width when the SVG exceeds 1440px so the whole chart is visible; report img sizes are the actual rendered CSS size inside report.html",
            "report_rendered": [], "stems": []}

with sync_playwright() as p:
    browser = p.chromium.launch(executable_path=EXE)
    ctx = browser.new_context(viewport={"width": 1440, "height": 900}, device_scale_factor=1)
    page = ctx.new_page()
    page.set_default_timeout(60000)
    for repo in REPOS:
        base = f"/home/shafin/Opencode/perfpal-test/out-i5-{repo}"
        page.set_viewport_size({"width": 1440, "height": 900})
        page.goto(f"file://{base}/report.html", wait_until="load")
        rendered = page.evaluate("""(stems) => stems.map(s => {
            const img = [...document.querySelectorAll('img')].find(i => (i.getAttribute('src')||'').includes('/'+s+'.svg'));
            if (!img) return {stem: s, missing: true};
            const r = img.getBoundingClientRect();
            return {stem: s, css_w: Math.round(r.width), css_h: Math.round(r.height),
                    natural_w: img.naturalWidth, natural_h: img.naturalHeight,
                    scale: +(r.width / img.naturalWidth).toFixed(3)};
        })""", STEMS)
        for row in rendered:
            row["repo"] = repo
            manifest["report_rendered"].append(row)
        for stem in STEMS:
            page.goto(f"file://{base}/charts/{stem}.svg", wait_until="load")
            page.wait_for_timeout(120)
            dims = page.evaluate("""() => { const el = document.querySelector('svg'); const r = el.getBoundingClientRect(); return {w: Math.ceil(r.width), h: Math.ceil(r.height)}; }""")
            vw = max(1440, dims["w"] + 20)
            vh = max(900, dims["h"] + 20)
            page.set_viewport_size({"width": vw, "height": vh})
            page.wait_for_timeout(120)
            page.screenshot(path=f"{OUT}/{repo}/{stem}.png")
            manifest["stems"].append({"repo": repo, "stem": stem, "svg": [dims["w"], dims["h"]],
                                       "png": [vw, vh], "file": f"{repo}/{stem}.png"})
    browser.close()

with open(f"{OUT}/manifest.json", "w") as f:
    json.dump(manifest, f, indent=1)
print(json.dumps(manifest, indent=1))
