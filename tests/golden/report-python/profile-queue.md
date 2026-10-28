# Perfpal Profile Queue

68 functions ranked by static cost — candidates, confirm with a profiler.

## Complete cost-ranked list

| rank | function | location | path class | cost | notes |
|---:|---|---|---|---|---|
| 1 | `triple\_nested` | `03\_deep\_nesting\.py:4` | production | 25 static cost units (v2) · 6.3× median · top 3% | unknown\-trip ×3 — estimate incomplete |
| 2 | `double\_nested` | `03\_deep\_nesting\.py:14` | production | 15 static cost units (v2) · 3.8× median · top 4% | unknown\-trip ×2 — estimate incomplete |
| 3 | `positional\_sum` | `08\_range\_len\.py:4` | production | 15 static cost units (v2) · 3.8× median · top 6% | unknown\-trip ×2 — estimate incomplete |
| 4 | `eager\_materialise` | `01\_alloc\_churn\.py:38` | production | 11 static cost units (v2) · 2.8× median · top 7% | unknown\-trip ×1 — estimate incomplete |
| 5 | `loop\_invariant\_construction` | `01\_alloc\_churn\.py:5` | production | 11 static cost units (v2) · 2.8× median · top 9% | unknown\-trip ×1 — estimate incomplete |
| 6 | `read\_each` | `04\_stdio\_churn\.py:4` | production | 11 static cost units (v2) · 2.8× median · top 10% | unknown\-trip ×1 — estimate incomplete |
| 7 | `index\_scan` | `07\_membership\_scan\.py:14` | production | 11 static cost units (v2) · 2.8× median · top 12% | known trip ×3 — verify it actually runs |
| 8 | `swallow` | `09\_try\_in\_loop\.py:4` | production | 11 static cost units (v2) · 2.8× median · top 13% | unknown\-trip ×1 — estimate incomplete |
| 9 | `contains\_any` | `07\_membership\_scan\.py:4` | production | 10 static cost units (v2) · 2.5× median · top 15% | unknown\-trip ×1 — estimate incomplete |
| 10 | `set\_membership` | `07\_membership\_scan\.py:32` | production | 10 static cost units (v2) · 2.5× median · top 16% | unknown\-trip ×1 — estimate incomplete |
| 11 | `accumulate\_multi` | `10\_comprehension\.py:16` | production | 8 static cost units (v2) · 2.0× median · top 18% | unknown\-trip ×1 — estimate incomplete |
| 12 | `transform\_all` | `10\_comprehension\.py:8` | production | 8 static cost units (v2) · 2.0× median · top 19% | unknown\-trip ×1 — estimate incomplete |
| 13 | `await\_each` | `14\_async\_blocking\.py:23` | production | 8 static cost units (v2) · 2.0× median · top 21% | unknown\-trip ×1 — estimate incomplete |
| 14 | `local\_rebuild\_is\_benign` | `01\_alloc\_churn\.py:27` | production | 7 static cost units (v2) · 1.8× median · top 25% | unknown\-trip ×1 — estimate incomplete |
| 15 | `superlinear\_concat` | `01\_alloc\_churn\.py:18` | production | 7 static cost units (v2) · 1.8× median · top 26% | unknown\-trip ×1 — estimate incomplete |
| 16 | `build\_string` | `02\_string\_building\.py:4` | production | 7 static cost units (v2) · 1.8× median · top 28% | unknown\-trip ×1 — estimate incomplete |
| 17 | `loop\_invariant\_prefix` | `02\_string\_building\.py:12` | production | 7 static cost units (v2) · 1.8× median · top 29% | unknown\-trip ×1 — estimate incomplete |
| 18 | `log\_each` | `04\_stdio\_churn\.py:14` | production | 7 static cost units (v2) · 1.8× median · top 31% | unknown\-trip ×1 — estimate incomplete |
| 19 | `copy\_each` | `06\_memory\_ops\.py:6` | production | 7 static cost units (v2) · 1.8× median · top 34% | unknown\-trip ×1 — estimate incomplete |
| 20 | `position\_of` | `07\_membership\_scan\.py:23` | production | 7 static cost units (v2) · 1.8× median · top 35% | unknown\-trip ×1 — estimate incomplete |
| 21 | `counter\_sum` | `08\_range\_len\.py:13` | production | 7 static cost units (v2) · 1.8× median · top 37% | unknown\-trip ×1 — estimate incomplete |
| 22 | `fan\_out` | `11\_executors\.py:6` | production | 7 static cost units (v2) · 1.8× median · top 38% | unknown\-trip ×1 — estimate incomplete |
| 23 | `fan\_out\_once` | `11\_executors\.py:13` | production | 7 static cost units (v2) · 1.8× median · top 40% | unknown\-trip ×1 — estimate incomplete |
| 24 | `fetch\_each` | `12\_io\_n1\.py:4` | production | 7 static cost units (v2) · 1.8× median · top 41% | unknown\-trip ×1 — estimate incomplete |
| 25 | `append\_frames` | `13\_pandas\.py:14` | production | 7 static cost units (v2) · 1.8× median · top 43% | unknown\-trip ×1 — estimate incomplete |
| 26 | `row\_wise` | `13\_pandas\.py:6` | production | 7 static cost units (v2) · 1.8× median · top 44% | unknown\-trip ×1 — estimate incomplete |
| 27 | `branchy` | `05\_recursion\_complexity\.py:16` | production | 33 static cost units (v2) · 8.3× median · top 1% | no\-loop — demoted |
| 28 | `clamp\_value` | `pkg/arch\_helpers\.py:9` | production | 8 static cost units (v2) · 2.0× median · top 22% | no\-loop — demoted |
| 29 | `ping\_b` | `pkg/cycle\_b\.py:6` | production | 8 static cost units (v2) · 2.0× median · top 24% | no\-loop — demoted |
| 30 | `walk\_down` | `05\_recursion\_complexity\.py:4` | production | 7 static cost units (v2) · 1.8× median · top 32% | no\-loop — demoted |
| 31 | `hoisted\_construction` | `01\_alloc\_churn\.py:13` | production | 5 static cost units (v2) · 1.3× median · top 46% | no\-loop — demoted |
| 32 | `materialise\_outside\_loop` | `01\_alloc\_churn\.py:46` | production | 5 static cost units (v2) · 1.3× median · top 47% | no\-loop — demoted |
| 33 | `walk\_cycle` | `pkg/cycle\_a\.py:6` | production | 5 static cost units (v2) · 1.3× median · top 49% | no\-loop — demoted |
| 34 | `guarded` | `09\_try\_in\_loop\.py:15` | production | 4 static cost units (v2) · 1.0× median · top 50% | no\-loop — demoted |
| 35 | `transform` | `10\_comprehension\.py:4` | production | 4 static cost units (v2) · 1.0× median · top 51% | no\-loop — demoted |
| 36 | `fetch\_blocking` | `14\_async\_blocking\.py:8` | production | 4 static cost units (v2) · 1.0× median · top 53% | no\-loop — demoted |
| 37 | `local\_only` | `15\_cross\_file\_and\_arch\.py:16` | production | 3 static cost units (v2) · 0.8× median · top 54% | no\-loop — demoted |
| 38 | `await\_once` | `14\_async\_blocking\.py:31` | production | 2 static cost units (v2) · 0.5× median · top 56% | no\-loop — demoted |
| 39 | `local\_helper` | `15\_cross\_file\_and\_arch\.py:11` | production | 2 static cost units (v2) · 0.5× median · top 57% | no\-loop — demoted |
| 40 | `use\_cross\_file` | `15\_cross\_file\_and\_arch\.py:6` | production | 2 static cost units (v2) · 0.5× median · top 59% | no\-loop — demoted |
| 41 | `normalize` | `pkg/arch\_helpers\.py:17` | production | 2 static cost units (v2) · 0.5× median · top 60% | no\-loop — demoted |
| 42 | `\<module\>` | `01\_alloc\_churn\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 62% | no\-loop — demoted |
| 43 | `\<module\>` | `02\_string\_building\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 63% | no\-loop — demoted |
| 44 | `\<module\>` | `03\_deep\_nesting\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 65% | no\-loop — demoted |
| 45 | `\<module\>` | `04\_stdio\_churn\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 66% | no\-loop — demoted |
| 46 | `\<module\>` | `05\_recursion\_complexity\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 68% | no\-loop — demoted |
| 47 | `simple` | `05\_recursion\_complexity\.py:54` | production | 1 static cost units (v2) · 0.3× median · top 69% | no\-loop — demoted |
| 48 | `step\_once` | `05\_recursion\_complexity\.py:11` | production | 1 static cost units (v2) · 0.3× median · top 71% | no\-loop — demoted |
| 49 | `\<module\>` | `06\_memory\_ops\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 72% | no\-loop — demoted |
| 50 | `copy\_once` | `06\_memory\_ops\.py:14` | production | 1 static cost units (v2) · 0.3× median · top 74% | no\-loop — demoted |
| 51 | `\<module\>` | `07\_membership\_scan\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 75% | no\-loop — demoted |
| 52 | `\<module\>` | `08\_range\_len\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 76% | no\-loop — demoted |
| 53 | `\<module\>` | `09\_try\_in\_loop\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 78% | no\-loop — demoted |
| 54 | `\<module\>` | `10\_comprehension\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 79% | no\-loop — demoted |
| 55 | `\<module\>` | `11\_executors\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 81% | no\-loop — demoted |
| 56 | `\<module\>` | `12\_io\_n1\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 82% | no\-loop — demoted |
| 57 | `fetch\_once` | `12\_io\_n1\.py:12` | production | 1 static cost units (v2) · 0.3× median · top 84% | no\-loop — demoted |
| 58 | `\<module\>` | `13\_pandas\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 85% | no\-loop — demoted |
| 59 | `vectorised` | `13\_pandas\.py:22` | production | 1 static cost units (v2) · 0.3× median · top 87% | no\-loop — demoted |
| 60 | `\<module\>` | `14\_async\_blocking\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 88% | no\-loop — demoted |
| 61 | `fetch\_sync` | `14\_async\_blocking\.py:18` | production | 1 static cost units (v2) · 0.3× median · top 90% | no\-loop — demoted |
| 62 | `pause` | `14\_async\_blocking\.py:13` | production | 1 static cost units (v2) · 0.3× median · top 91% | no\-loop — demoted |
| 63 | `\<module\>` | `15\_cross\_file\_and\_arch\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 93% | no\-loop — demoted |
| 64 | `\<module\>` | `\_\_init\_\_\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 94% | no\-loop — demoted |
| 65 | `\<module\>` | `pkg/\_\_init\_\_\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 96% | no\-loop — demoted |
| 66 | `\<module\>` | `pkg/arch\_helpers\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 97% | no\-loop — demoted |
| 67 | `\<module\>` | `pkg/cycle\_a\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 99% | no\-loop — demoted |
| 68 | `\<module\>` | `pkg/cycle\_b\.py:1` | production | 1 static cost units (v2) · 0.3× median · top 100% | no\-loop — demoted |
