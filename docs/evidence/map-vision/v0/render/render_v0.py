import json, os
from playwright.sync_api import sync_playwright

OUT = "/home/shafin/Opencode/Perfpal/docs/evidence/map-vision/v0"
REPOS = ["tmux", "hyperfine", "cJSON"]
STEMS = ["include-graph", "layer-file", "layer-entry", "component-coupling",
         "hot-components", "layer-workspace", "layer-bands", "layer-findings"]
EXE = "/home/shafin/.cache/ms-playwright/chromium-1243/chrome-linux64/chrome"

manifest = []
with sync_playwright() as p:
    browser = p.chromium.launch(executable_path=EXE)
    ctx = browser.new_context(viewport={"width": 1440, "height": 900}, device_scale_factor=1)
    page = ctx.new_page()
    page.set_default_timeout(60000)
    for repo in REPOS:
        base = f"/home/shafin/Opencode/perfpal-test/out-i5-{repo}"
        os.makedirs(f"{OUT}/{repo}", exist_ok=True)
        page.set_viewport_size({"width": 1440, "height": 900})
        page.goto(f"file://{base}/report.html", wait_until="load")
        page.screenshot(path=f"{OUT}/{repo}/report-first-screen.png")
        page.goto(f"file://{base}/report.html#map", wait_until="load")
        page.wait_for_timeout(250)
        page.screenshot(path=f"{OUT}/{repo}/report-map.png")
        for stem in STEMS:
            page.goto(f"file://{base}/charts/{stem}.svg", wait_until="load")
            page.wait_for_timeout(120)
            dims = page.evaluate("""() => { const el = document.querySelector('svg'); const r = el ? el.getBoundingClientRect() : null; return {w: r?Math.round(r.width):null, h: r?Math.round(r.height):null, docW: document.documentElement.scrollWidth, docH: document.documentElement.scrollHeight, sx: window.scrollX, sy: window.scrollY}; }""")
            vh = min(max(dims["docH"] + 20, 300), 4000)
            page.set_viewport_size({"width": 1440, "height": vh})
            page.wait_for_timeout(120)
            page.screenshot(path=f"{OUT}/{repo}/{stem}.png")
            manifest.append({"repo": repo, "stem": stem, "svg": [dims["w"], dims["h"]],
                             "doc": [dims["docW"], dims["docH"]], "png": [1440, vh]})
            page.set_viewport_size({"width": 1440, "height": 900})
    browser.close()

print(json.dumps(manifest, indent=1))
