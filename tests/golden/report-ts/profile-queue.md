# Perfpal Profile Queue

54 functions ranked by static cost — candidates, confirm with a profiler.

| rank | function | location | path class | cost | notes |
|---:|---|---|---|---|---|
| 1 | `smallWindow` | `06\_membership\_scan\.ts:19` | production | 19 static cost units (v2) · 19× median · top 2% | known trip ×4 — verify it actually runs |
| 2 | `score` | `04\_deep\_nesting\.ts:7` | production | 18 static cost units (v2) · 18× median · top 4% | unknown\-trip ×2 — estimate incomplete |
| 3 | `tinyPrefix` | `03\_string\_build\.ts:13` | production | 15 static cost units (v2) · 15× median · top 6% | known trip ×5 — verify it actually runs |
| 4 | `buildEnvelopes` | `01\_alloc\_churn\.ts:11` | production | 13 static cost units (v2) · 13× median · top 7% | unknown\-trip ×1 — estimate incomplete |
| 5 | `buildIndexed` | `01\_alloc\_churn\.ts:19` | production | 13 static cost units (v2) · 13× median · top 9% | unknown\-trip ×1 — estimate incomplete |
| 6 | `buildLabelled` | `01\_alloc\_churn\.ts:27` | production | 13 static cost units (v2) · 13× median · top 11% | unknown\-trip ×1 — estimate incomplete |
| 7 | `flatten` | `02\_array\_accumulation\.ts:5` | production | 13 static cost units (v2) · 13× median · top 13% | unknown\-trip ×1 — estimate incomplete |
| 8 | `intersect` | `06\_membership\_scan\.ts:5` | production | 13 static cost units (v2) · 13× median · top 15% | unknown\-trip ×1 — estimate incomplete |
| 9 | `loadAll` | `07\_async\_await\.ts:9` | production | 11 static cost units (v2) · 11× median · top 17% | unknown\-trip ×1 — estimate incomplete |
| 10 | `snapshotRows` | `02\_array\_accumulation\.ts:13` | production | 10 static cost units (v2) · 10× median · top 19% | unknown\-trip ×1 — estimate incomplete |
| 11 | `loadAll` | `05\_sync\_io\_loop\.ts:6` | production | 10 static cost units (v2) · 10× median · top 20% | unknown\-trip ×1 — estimate incomplete |
| 12 | `chunk` | `13\_controls\.ts:7` | production | 10 static cost units (v2) · 10× median · top 22% | unknown\-trip ×1 — estimate incomplete |
| 13 | `render` | `03\_string\_build\.ts:5` | production | 7 static cost units (v2) · 7.0× median · top 24% | unknown\-trip ×1 — estimate incomplete |
| 14 | `sumValues` | `03\_string\_build\.ts:26` | production | 7 static cost units (v2) · 7.0× median · top 26% | unknown\-trip ×1 — estimate incomplete |
| 15 | `scoreFlat` | `04\_deep\_nesting\.ts:17` | production | 7 static cost units (v2) · 7.0× median · top 28% | unknown\-trip ×1 — estimate incomplete |
| 16 | `countdown` | `09\_recursion\.ts:13` | production | 7 static cost units (v2) · 7.0× median · top 30% | no\-loop — demoted |
| 17 | `formatFromB` | `11\_cycle\_b\.ts:5` | production | 6 static cost units (v2) · 6.0× median · top 31% | no\-loop — demoted |
| 18 | `descend` | `09\_recursion\.ts:9` | production | 5 static cost units (v2) · 5.0× median · top 33% | no\-loop — demoted |
| 19 | `format` | `10\_cycle\_a\.ts:6` | production | 5 static cost units (v2) · 5.0× median · top 35% | no\-loop — demoted |
| 20 | `\<module\>` | `14\_types\_only\.ts:1` | production | 5 static cost units (v2) · 5.0× median · top 37% | no\-loop — demoted |
| 21 | `fetchJson` | `07\_async\_await\.ts:5` | production | 4 static cost units (v2) · 4.0× median · top 39% | no\-loop — demoted |
| 22 | `summarize` | `13\_controls\.ts:15` | production | 4 static cost units (v2) · 4.0× median · top 41% | no\-loop — demoted |
| 23 | `Badge` | `15\_component\.tsx:9` | production | 3 static cost units (v2) · 3.0× median · top 43% | no\-loop — demoted |
| 24 | `loadOne` | `07\_async\_await\.ts:17` | production | 2 static cost units (v2) · 2.0× median · top 44% | no\-loop — demoted |
| 25 | `boot` | `index\.ts:6` | production | 2 static cost units (v2) · 2.0× median · top 46% | no\-loop — demoted |
| 26 | `\<module\>` | `01\_alloc\_churn\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 48% | no\-loop — demoted |
| 27 | `\<module\>` | `02\_array\_accumulation\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 50% | no\-loop — demoted |
| 28 | `merge` | `02\_array\_accumulation\.ts:22` | production | 1 static cost units (v2) · 1.0× median · top 52% | no\-loop — demoted |
| 29 | `\<module\>` | `03\_string\_build\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 54% | no\-loop — demoted |
| 30 | `appendOnce` | `03\_string\_build\.ts:21` | production | 1 static cost units (v2) · 1.0× median · top 56% | no\-loop — demoted |
| 31 | `\<module\>` | `04\_deep\_nesting\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 57% | no\-loop — demoted |
| 32 | `\<module\>` | `05\_sync\_io\_loop\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 59% | no\-loop — demoted |
| 33 | `loadOnce` | `05\_sync\_io\_loop\.ts:14` | production | 1 static cost units (v2) · 1.0× median · top 61% | no\-loop — demoted |
| 34 | `\<module\>` | `06\_membership\_scan\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 63% | no\-loop — demoted |
| 35 | `containsOnce` | `06\_membership\_scan\.ts:15` | production | 1 static cost units (v2) · 1.0× median · top 65% | no\-loop — demoted |
| 36 | `\<module\>` | `07\_async\_await\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 67% | no\-loop — demoted |
| 37 | `\<module\>` | `08\_async\_blocking\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 69% | no\-loop — demoted |
| 38 | `readConfig` | `08\_async\_blocking\.ts:6` | production | 1 static cost units (v2) · 1.0× median · top 70% | no\-loop — demoted |
| 39 | `readConfigSync` | `08\_async\_blocking\.ts:10` | production | 1 static cost units (v2) · 1.0× median · top 72% | no\-loop — demoted |
| 40 | `\<module\>` | `09\_recursion\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 74% | no\-loop — demoted |
| 41 | `\<module\>` | `10\_cycle\_a\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 76% | no\-loop — demoted |
| 42 | `\<module\>` | `11\_cycle\_b\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 78% | no\-loop — demoted |
| 43 | `\<module\>` | `12\_delete\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 80% | no\-loop — demoted |
| 44 | `dropField` | `12\_delete\.ts:5` | production | 1 static cost units (v2) · 1.0× median · top 81% | no\-loop — demoted |
| 45 | `dropKey` | `12\_delete\.ts:9` | production | 1 static cost units (v2) · 1.0× median · top 83% | no\-loop — demoted |
| 46 | `\<module\>` | `13\_controls\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 85% | no\-loop — demoted |
| 47 | `mergeDefaults` | `13\_controls\.ts:19` | production | 1 static cost units (v2) · 1.0× median · top 87% | no\-loop — demoted |
| 48 | `\<anonymous\>` | `14\_types\_only\.ts:45` | production | 1 static cost units (v2) · 1.0× median · top 89% | no\-loop — demoted |
| 49 | `parse` | `14\_types\_only\.ts:26` | production | 1 static cost units (v2) · 1.0× median · top 91% | no\-loop — demoted |
| 50 | `\<module\>` | `15\_component\.tsx:1` | production | 1 static cost units (v2) · 1.0× median · top 93% | no\-loop — demoted |
| 51 | `\<module\>` | `16\_generic\_arrow\.tsx:1` | production | 1 static cost units (v2) · 1.0× median · top 94% | no\-loop — demoted |
| 52 | `identity` | `16\_generic\_arrow\.tsx:4` | production | 1 static cost units (v2) · 1.0× median · top 96% | no\-loop — demoted |
| 53 | `identityBare` | `16\_generic\_arrow\.tsx:6` | production | 1 static cost units (v2) · 1.0× median · top 98% | no\-loop — demoted |
| 54 | `\<module\>` | `index\.ts:1` | production | 1 static cost units (v2) · 1.0× median · top 100% | startup \+ no\-loop — demoted |
