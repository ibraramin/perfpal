# V0 render reproduction

The committed PNGs under `docs/evidence/map-vision/v0/<repo>/` were produced with Playwright
(chromium 1243), viewport **1440x900**, `device_scale_factor=1` (100% zoom).

```bash
python3 render_v0.py        # 6 report captures: report.html first screen + report.html#map
python3 render_v0_stems.py  # 24 stem captures (natural width when the SVG exceeds 1440px)
```

- `render_v0.py` hardcodes the checkout paths (`/home/shafin/Opencode/perfpal-test/...`); adjust
  before running elsewhere.
- `render_v0_stems.py` writes `manifest_stems.json` (stem dimensions only). The curated
  `../manifest.json` is **not** overwritten: it also carries `report_pngs`, `off_viewport_reads`,
  `read_coverage`, and the canonical note.
- The 3 `report-first-screen.png` captures are artifacts; only `#map` + the 8 stems were sent to
  `@vision` (28 reads in `../reads.json`).
