# H2 page height and A8 re-baseline — 2026-10-28

Report: `report.html` generated from each fixture at the final H2 code (`4d401db`, `--story`),
opened over `file://` in headless Brave at DPR 1, viewport height 900. Method:
Playwright `document.documentElement.scrollHeight` and `getBoundingClientRect()`
after a 500 ms settle; all sections are fully laid out. Raw JSON:
`/var/tmp/opencode/h2/{hyperfine,cjson,3k}-height5.json` (final run5 artifacts).

## Hyperfine (A8 comparator fixture)

| Width | Document px | `#map` px | Story px | Story canvas px | Answer bottom px |
|---:|---:|---:|---:|---:|---:|
| 1280 | 17,879 | 4,524 | 993 | 635 | 427 |
| 1440 | 16,927 | 3,692 | 1,029 | 715 | 427 |
| 1920 | 18,752 | 4,990 | 1,274 | 953 | 428 |
| 2560 | 19,946 | 5,536 | 1,604 | 1,272 | 424 |
| 3072 | 21,241 | 6,165 | 1,875 | 1,534 | 436 |

H1 comparator (ADR-0014): 18,747 px at 1440, 20,646 px at 3072, story 1,032 px.
H2 is **−1,820 px at 1440** because the legacy Explorer pan/zoom figures now
downscale to their figure instead of overflowing it (no internal scroll), and
**+595 px at 3072** because the story canvas is full-width (1,534 px vs the H1
718 px capped canvas) and the Atlas charts fill the band. The legacy Explorer
SVGs are **not upscaled** past their intrinsic width; an earlier candidate that
stretched them to 2,974 px made the page 30,470 px and was rejected.

## cJSON and 3k scale fixture

| Fixture | 1280 doc | 1440 doc | 1920 doc | 2560 doc | 3072 doc | 3072 `#map` | 3072 story |
|---|---:|---:|---:|---:|---:|---:|---:|
| cJSON | 25,063 | 24,092 | 25,658 | 26,559 | 27,379 | 6,104 | 1,875 |
| 3k | 29,809 | 29,511 | 32,489 | 33,736 | 34,882 | 7,405 | 1,875 |

## A8 decision (recorded in report-format §6/§9)

- **First-page fold (unchanged):** the answer line must end above the fold at
  every supported width. Measured bottoms: 427 px at 1280/1440, 428 at 1920,
  424 at 2560, 436 at 3072 — all ≤ 436 px, so the A8 fold target is unchanged.
- **Document height (re-baselined with evidence):** 16,927 px at 1440 and
  21,241 px at 3072 on hyperfine; cJSON 24,092/27,379; 3k 29,511/34,882. The
  budget is per-fixture and content-driven; the layout must not stretch legacy
  figures, and no width introduces horizontal overflow.
