# Perfpal Profile Queue

48 functions ranked by static cost — candidates, confirm with a profiler.

| rank | function | location | path class | cost | notes |
|---:|---|---|---|---|---|
| 1 | `smallWindow` | `06\_membership\_scan\.js:19` | production | 19 static cost units (v2) · 9.5× median · top 2% | known trip ×4 — verify it actually runs |
| 2 | `score` | `04\_deep\_nesting\.js:4` | production | 18 static cost units (v2) · 9.0× median · top 4% | unknown\-trip ×2 — estimate incomplete |
| 3 | `tinyPrefix` | `03\_string\_build\.js:13` | production | 15 static cost units (v2) · 7.5× median · top 6% | known trip ×5 — verify it actually runs |
| 4 | `buildEnvelopes` | `01\_alloc\_churn\.js:6` | production | 13 static cost units (v2) · 6.5× median · top 8% | unknown\-trip ×1 — estimate incomplete |
| 5 | `buildIndexed` | `01\_alloc\_churn\.js:14` | production | 13 static cost units (v2) · 6.5× median · top 10% | unknown\-trip ×1 — estimate incomplete |
| 6 | `buildLabelled` | `01\_alloc\_churn\.js:22` | production | 13 static cost units (v2) · 6.5× median · top 13% | unknown\-trip ×1 — estimate incomplete |
| 7 | `flatten` | `02\_array\_accumulation\.js:5` | production | 13 static cost units (v2) · 6.5× median · top 15% | unknown\-trip ×1 — estimate incomplete |
| 8 | `intersect` | `06\_membership\_scan\.js:5` | production | 13 static cost units (v2) · 6.5× median · top 17% | unknown\-trip ×1 — estimate incomplete |
| 9 | `loadAll` | `07\_async\_await\.js:8` | production | 11 static cost units (v2) · 5.5× median · top 19% | unknown\-trip ×1 — estimate incomplete |
| 10 | `snapshotRows` | `02\_array\_accumulation\.js:13` | production | 10 static cost units (v2) · 5.0× median · top 21% | unknown\-trip ×1 — estimate incomplete |
| 11 | `loadAll` | `05\_sync\_io\_loop\.js:6` | production | 10 static cost units (v2) · 5.0× median · top 23% | unknown\-trip ×1 — estimate incomplete |
| 12 | `chunk` | `13\_controls\.js:7` | production | 10 static cost units (v2) · 5.0× median · top 25% | unknown\-trip ×1 — estimate incomplete |
| 13 | `render` | `03\_string\_build\.js:5` | production | 7 static cost units (v2) · 3.5× median · top 27% | unknown\-trip ×1 — estimate incomplete |
| 14 | `sumValues` | `03\_string\_build\.js:26` | production | 7 static cost units (v2) · 3.5× median · top 29% | unknown\-trip ×1 — estimate incomplete |
| 15 | `scoreFlat` | `04\_deep\_nesting\.js:14` | production | 7 static cost units (v2) · 3.5× median · top 31% | unknown\-trip ×1 — estimate incomplete |
| 16 | `countdown` | `09\_recursion\.js:9` | production | 7 static cost units (v2) · 3.5× median · top 33% | no\-loop — demoted |
| 17 | `formatFromB` | `11\_cycle\_b\.js:5` | production | 6 static cost units (v2) · 3.0× median · top 35% | no\-loop — demoted |
| 18 | `descend` | `09\_recursion\.js:5` | production | 5 static cost units (v2) · 2.5× median · top 38% | no\-loop — demoted |
| 19 | `format` | `10\_cycle\_a\.js:6` | production | 5 static cost units (v2) · 2.5× median · top 40% | no\-loop — demoted |
| 20 | `basename` | `14\_commonjs\.cjs:5` | production | 5 static cost units (v2) · 2.5× median · top 42% | no\-loop — demoted |
| 21 | `fetchJson` | `07\_async\_await\.js:4` | production | 4 static cost units (v2) · 2.0× median · top 44% | no\-loop — demoted |
| 22 | `summarize` | `13\_controls\.js:15` | production | 4 static cost units (v2) · 2.0× median · top 46% | no\-loop — demoted |
| 23 | `\<module\>` | `14\_commonjs\.cjs:1` | production | 4 static cost units (v2) · 2.0× median · top 48% | no\-loop — demoted |
| 24 | `loadOne` | `07\_async\_await\.js:16` | production | 2 static cost units (v2) · 1.0× median · top 50% | no\-loop — demoted |
| 25 | `boot` | `index\.js:5` | production | 2 static cost units (v2) · 1.0× median · top 52% | no\-loop — demoted |
| 26 | `\<module\>` | `01\_alloc\_churn\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 54% | no\-loop — demoted |
| 27 | `\<module\>` | `02\_array\_accumulation\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 56% | no\-loop — demoted |
| 28 | `merge` | `02\_array\_accumulation\.js:22` | production | 1 static cost units (v2) · 0.5× median · top 58% | no\-loop — demoted |
| 29 | `\<module\>` | `03\_string\_build\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 60% | no\-loop — demoted |
| 30 | `appendOnce` | `03\_string\_build\.js:21` | production | 1 static cost units (v2) · 0.5× median · top 63% | no\-loop — demoted |
| 31 | `\<module\>` | `04\_deep\_nesting\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 65% | no\-loop — demoted |
| 32 | `\<module\>` | `05\_sync\_io\_loop\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 67% | no\-loop — demoted |
| 33 | `loadOnce` | `05\_sync\_io\_loop\.js:14` | production | 1 static cost units (v2) · 0.5× median · top 69% | no\-loop — demoted |
| 34 | `\<module\>` | `06\_membership\_scan\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 71% | no\-loop — demoted |
| 35 | `containsOnce` | `06\_membership\_scan\.js:15` | production | 1 static cost units (v2) · 0.5× median · top 73% | no\-loop — demoted |
| 36 | `\<module\>` | `07\_async\_await\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 75% | no\-loop — demoted |
| 37 | `\<module\>` | `08\_async\_blocking\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 77% | no\-loop — demoted |
| 38 | `readConfig` | `08\_async\_blocking\.js:6` | production | 1 static cost units (v2) · 0.5× median · top 79% | no\-loop — demoted |
| 39 | `readConfigSync` | `08\_async\_blocking\.js:10` | production | 1 static cost units (v2) · 0.5× median · top 81% | no\-loop — demoted |
| 40 | `\<module\>` | `09\_recursion\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 83% | no\-loop — demoted |
| 41 | `\<module\>` | `10\_cycle\_a\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 85% | no\-loop — demoted |
| 42 | `\<module\>` | `11\_cycle\_b\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 88% | no\-loop — demoted |
| 43 | `\<module\>` | `12\_delete\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 90% | no\-loop — demoted |
| 44 | `dropField` | `12\_delete\.js:5` | production | 1 static cost units (v2) · 0.5× median · top 92% | no\-loop — demoted |
| 45 | `dropKey` | `12\_delete\.js:9` | production | 1 static cost units (v2) · 0.5× median · top 94% | no\-loop — demoted |
| 46 | `\<module\>` | `13\_controls\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 96% | no\-loop — demoted |
| 47 | `mergeDefaults` | `13\_controls\.js:19` | production | 1 static cost units (v2) · 0.5× median · top 98% | no\-loop — demoted |
| 48 | `\<module\>` | `index\.js:1` | production | 1 static cost units (v2) · 0.5× median · top 100% | startup \+ no\-loop — demoted |
