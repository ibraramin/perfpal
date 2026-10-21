# Perfpal Profile Queue

40 functions ranked by static cost — candidates, confirm with a profiler.

| rank | function | location | path class | cost | notes |
|---:|---|---|---|---|---|
| 1 | `grid\_checksum` | `src/03\_deep\_nesting\.rs:15` | production | 25 static cost units (v2) · 3.6× median · top 5% | unknown\-trip ×3 — estimate incomplete |
| 2 | `build\_row\_labels` | `src/01\_alloc\_churn\.rs:32` | production | 22 static cost units (v2) · 3.1× median · top 8% | unknown\-trip ×1 — estimate incomplete |
| 3 | `duplicate\_rows` | `src/06\_memory\_ops\.rs:24` | production | 22 static cost units (v2) · 3.1× median · top 10% | unknown\-trip ×2 — estimate incomplete |
| 4 | `grid\_reset` | `src/01\_alloc\_churn\.rs:56` | production | 19 static cost units (v2) · 2.7× median · top 13% | unknown\-trip ×2 — estimate incomplete |
| 5 | `contains\_nested` | `src/09\_membership\_scan\.rs:13` | production | 19 static cost units (v2) · 2.7× median · top 15% | unknown\-trip ×2 — estimate incomplete |
| 6 | `len\_with\_reassignment` | `src/02\_string\_building\.rs:38` | production | 14 static cost units (v2) · 2.0× median · top 18% | unknown\-trip ×1 — estimate incomplete |
| 7 | `grow\_one\_at\_a\_time` | `src/01\_alloc\_churn\.rs:46` | production | 11 static cost units (v2) · 1.6× median · top 20% | unknown\-trip ×1 — estimate incomplete |
| 8 | `append\_via\_add\_assign` | `src/02\_string\_building\.rs:49` | production | 11 static cost units (v2) · 1.6× median · top 23% | unknown\-trip ×1 — estimate incomplete |
| 9 | `first\_match\_shallow` | `src/09\_membership\_scan\.rs:26` | production | 10 static cost units (v2) · 1.4× median · top 25% | unknown\-trip ×1 — estimate incomplete |
| 10 | `call\_directly` | `src/08\_globals\_and\_chains\.rs:49` | production | 8 static cost units (v2) · 1.1× median · top 28% | unknown\-trip ×1 — estimate incomplete |
| 11 | `clamp\_all` | `src/10\_cross\_file\_and\_arch\.rs:40` | production | 8 static cost units (v2) · 1.1× median · top 30% | unknown\-trip ×1 — estimate incomplete |
| 12 | `accumulate` | `src/02\_string\_building\.rs:20` | production | 7 static cost units (v2) · 1.0× median · top 33% | unknown\-trip ×1 — estimate incomplete |
| 13 | `scan\_lengths` | `src/02\_string\_building\.rs:28` | production | 7 static cost units (v2) · 1.0× median · top 35% | unknown\-trip ×1 — estimate incomplete |
| 14 | `consume\_all` | `src/04\_stdio\_churn\.rs:35` | production | 7 static cost units (v2) · 1.0× median · top 38% | known trip ×1 — verify it actually runs |
| 15 | `count\_chars\_per\_iteration` | `src/04\_stdio\_churn\.rs:16` | production | 7 static cost units (v2) · 1.0× median · top 40% | unknown\-trip ×1 — estimate incomplete |
| 16 | `emit\_rows` | `src/04\_stdio\_churn\.rs:26` | production | 7 static cost units (v2) · 1.0× median · top 43% | unknown\-trip ×1 — estimate incomplete |
| 17 | `read\_depth` | `src/08\_globals\_and\_chains\.rs:56` | production | 7 static cost units (v2) · 1.0× median · top 48% | unknown\-trip ×1 — estimate incomplete |
| 18 | `read\_depth\_member` | `src/08\_globals\_and\_chains\.rs:66` | production | 7 static cost units (v2) · 1.0× median · top 50% | unknown\-trip ×1 — estimate incomplete |
| 19 | `read\_zone\_ref` | `src/08\_globals\_and\_chains\.rs:76` | production | 7 static cost units (v2) · 1.0× median · top 53% | unknown\-trip ×1 — estimate incomplete |
| 20 | `run\_handlers` | `src/08\_globals\_and\_chains\.rs:37` | production | 7 static cost units (v2) · 1.0× median · top 55% | unknown\-trip ×1 — estimate incomplete |
| 21 | `classify` | `src/05\_recursion\_complexity\.rs:20` | production | 33 static cost units (v2) · 4.7× median · top 3% | no\-loop — demoted |
| 22 | `fib` | `src/05\_recursion\_complexity\.rs:10` | production | 7 static cost units (v2) · 1.0× median · top 45% | no\-loop — demoted |
| 23 | `clamp\_int` | `src/10\_cross\_file\_and\_arch\.rs:29` | production | 7 static cost units (v2) · 1.0× median · top 58% | no\-loop — demoted |
| 24 | `make\_scratch` | `src/01\_alloc\_churn\.rs:20` | production | 5 static cost units (v2) · 0.7× median · top 60% | no\-loop — demoted |
| 25 | `capacity\_from\_count` | `src/06\_memory\_ops\.rs:19` | production | 5 static cost units (v2) · 0.7× median · top 63% | no\-loop — demoted |
| 26 | `over\_reserve` | `src/06\_memory\_ops\.rs:14` | production | 5 static cost units (v2) · 0.7× median · top 65% | no\-loop — demoted |
| 27 | `orchestrate` | `src/10\_cross\_file\_and\_arch\.rs:20` | production | 4 static cost units (v2) · 0.6× median · top 68% | no\-loop — demoted |
| 28 | `scale\_checksum` | `src/03\_deep\_nesting\.rs:30` | production | 3 static cost units (v2) · 0.4× median · top 70% | no\-loop — demoted |
| 29 | `direct\_handler` | `src/08\_globals\_and\_chains\.rs:22` | production | 3 static cost units (v2) · 0.4× median · top 73% | no\-loop — demoted |
| 30 | `shared\_checksum` | `src/common\.rs:15` | production | 3 static cost units (v2) · 0.4× median · top 75% | no\-loop — demoted |
| 31 | `cycle\_a\_root` | `src/cycle\_a\.rs:12` | production | 3 static cost units (v2) · 0.4× median · top 78% | no\-loop — demoted |
| 32 | `cycle\_b\_step` | `src/cycle\_b\.rs:9` | production | 3 static cost units (v2) · 0.4× median · top 80% | no\-loop — demoted |
| 33 | `cycle\_a\_step` | `src/cycle\_a\.rs:17` | production | 2 static cost units (v2) · 0.3× median · top 83% | no\-loop — demoted |
| 34 | `cycle\_b\_root` | `src/cycle\_b\.rs:14` | production | 2 static cost units (v2) · 0.3× median · top 85% | no\-loop — demoted |
| 35 | `drop\_outside\_loop` | `src/01\_alloc\_churn\.rs:27` | production | 1 static cost units (v2) · 0.1× median · top 88% | no\-loop — demoted |
| 36 | `bounded\_copy` | `src/02\_string\_building\.rs:66` | production | 1 static cost units (v2) · 0.1× median · top 90% | no\-loop — demoted |
| 37 | `unsafe\_slice` | `src/02\_string\_building\.rs:61` | production | 1 static cost units (v2) · 0.1× median · top 93% | no\-loop — demoted |
| 38 | `consume\_blob` | `src/07\_struct\_by\_value\.rs:40` | production | 1 static cost units (v2) · 0.1× median · top 95% | no\-loop — demoted |
| 39 | `consume\_blob\_ref` | `src/07\_struct\_by\_value\.rs:45` | production | 1 static cost units (v2) · 0.1× median · top 98% | no\-loop — demoted |
| 40 | `layout\_total` | `src/07\_struct\_by\_value\.rs:50` | production | 1 static cost units (v2) · 0.1× median · top 100% | no\-loop — demoted |
