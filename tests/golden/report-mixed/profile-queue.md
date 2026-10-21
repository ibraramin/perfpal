# Perfpal Profile Queue

4 functions ranked by static cost — candidates, confirm with a profiler.

| rank | function | location | path class | cost | notes |
|---:|---|---|---|---|---|
| 1 | `build` | `src/lib\.rs:15` | production | 20 static cost units (v2) · 6.7× median · top 25% | unknown\-trip ×1 — estimate incomplete |
| 2 | `c\_loop` | `src/main\.c:23` | production | 15 static cost units (v2) · 5.0× median · top 50% | unknown\-trip ×1 — estimate incomplete |
| 3 | `util\_init` | `src/util\.c:6` | production | 3 static cost units (v2) · 1.0× median · top 75% | no\-loop — demoted |
| 4 | `rust\_surface` | `src/lib\.rs:33` | production | 1 static cost units (v2) · 0.3× median · top 100% | no\-loop — demoted |
