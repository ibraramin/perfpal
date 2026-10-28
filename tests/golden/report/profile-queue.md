# Perfpal Profile Queue

12 functions ranked by static cost — candidates, confirm with a profiler.

## Complete cost-ranked list

| rank | function | location | path class | cost | notes |
|---:|---|---|---|---|---|
| 1 | `dispatch` | `src/main\.c:84` | production | 57 static cost units (v2) · 8.1× median · top 8% | unknown\-trip ×2 — estimate incomplete |
| 2 | `deep\_sum` | `src/main\.c:57` | production | 27 static cost units (v2) · 3.9× median · top 17% | unknown\-trip ×3 — estimate incomplete |
| 3 | `process` | `src/main\.c:31` | production | 14 static cost units (v2) · 2.0× median · top 25% | unknown\-trip ×1 — estimate incomplete |
| 4 | `grow` | `src/main\.c:69` | production | 13 static cost units (v2) · 1.9× median · top 33% | unknown\-trip ×1 — estimate incomplete |
| 5 | `report` | `src/main\.c:78` | production | 9 static cost units (v2) · 1.3× median · top 50% | unknown\-trip ×1 — estimate incomplete |
| 6 | `drain` | `src/main\.c:42` | production | 7 static cost units (v2) · 1.0× median · top 58% | unknown\-trip ×1 — estimate incomplete |
| 7 | `join` | `src/main\.c:48` | production | 7 static cost units (v2) · 1.0× median · top 67% | unknown\-trip ×1 — estimate incomplete |
| 8 | `clone\_row` | `src/main\.c:117` | production | 5 static cost units (v2) · 0.7× median · top 75% | no\-loop — demoted |
| 9 | `helper` | `src/main\.c:27` | production | 4 static cost units (v2) · 0.6× median · top 83% | no\-loop — demoted |
| 10 | `util\_init` | `src/util\.c:8` | production | 3 static cost units (v2) · 0.4× median · top 92% | no\-loop — demoted |
| 11 | `weigh` | `src/main\.c:113` | production | 1 static cost units (v2) · 0.1× median · top 100% | no\-loop — demoted |
| 12 | `main` | `src/main\.c:121` | production | 11 static cost units (v2) · 1.6× median · top 42% | startup \+ no\-loop — demoted |
