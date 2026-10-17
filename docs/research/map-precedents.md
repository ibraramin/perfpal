# Map Precedents — Tool Teardown (R2)

Scope: teardown of 10 dependency/architecture visualization tools, each judged
maintainer-helpful vs eye-candy against Perfpal's three questions (Q1 where to
look first · Q2 what breaks if touched · Q3 where time might plausibly go,
quantified for LLM consumption). Feeds R4 (`map-decision.md`).

Provenance: 4 parallel lanes, all pages fetched 2026-10-17 (LL-012
fetch→quote→cite discipline). Every external claim below carries [URL + date].
Unopenable pages are listed under Dead ends and support nothing.

## 1. Sourcetrail — maintainer-helpful (archived)

- **Renders.** Interactive source explorer, not a whole-program map: "three
  interactive views" — "Search… Graph… Code"
  [github.com/CoatiSoftware/Sourcetrail/DOCUMENTATION.md + 2026-10-17]. "The
  graph displays the structure of your source code. It focuses on the currently
  selected symbol and directly shows all incoming and outgoing dependencies to
  other symbols" [same docs + 2026-10-17]. Whole call graphs / inheritance
  chains / include trees available via toolbar (fetched page truncated the
  graph-view section; claim limited to what was opened).
- **Scale.** No throughput numbers. Release history: archived "Dec 14, 2021",
  "9" releases ending "2021.4.19" [github.com/CoatiSoftware/Sourcetrail/releases
  + 2026-10-17]. Platforms C/C++/Java/Python via heavy indexers: "C support is
  powered by Clang 11.0.0" / "Java 12 and lower … powered by Eclipse JDT" [same
  docs + 2026-10-17].
- **Honesty.** Best-in-class incapability admission: "If a viewed file had
  errors during indexing, its file icon will contain an `x`… Show Errors will
  show you only the errors that cause this specific file to be incomplete"
  [same docs + 2026-10-17]. Setup demands a compilation database
  (`compile_commands.json` via `CMAKE_EXPORT_COMPILE_COMMANDS`, Bear, Qt
  Creator) — honest signal that C++ maps without build info lie.
- **Status: archived, verified.** "This project was archived by the original
  autors and maintainers of Sourcetrail by the end of 2021" [repo page +
  2026-10-17]. "We decided to discontinue maintaining Sourcetrail… archived
  after publishing a last release version this Fall"
  [web.archive.org Sourcetrail discontinue post + 2026-10-17]. Cause:
  "ambitious software with many tech dependencies, which require constant
  attention … multiple platforms… languages and build systems" plus "lost
  interest in working in the field of language analysis and software
  visualization" [same post + 2026-10-17].
- **Verdict: helpful.** Q1: search autocomplete "instantly provide[s] an
  overview of all matching results throughout your codebase" [same docs +
  2026-10-17]. Q2: per-symbol incoming/outgoing ego-graph is the interaction
  Perfpal's static map should steal, minus Qt GUI and Clang-weight indexer. Q3:
  nothing — no timing data. Lesson: cap scope (tree-sitter, offline, CLI) or
  inherit Sourcetrail's maintenance death.

## 2. code2flow — maintainer-helpful (labelled uncertainty)

- **Renders.** Whole-project call graph as DOT/PNG/JSON: "Code2flow generates
  call graphs for dynamic programming language… Python, JavaScript, Ruby, PHP"
  [github.com/scottrogowski/code2flow/README.md + 2026-10-17]. "Groups are
  files, modules, or classes… Nodes are the functions themselves" [same README
  + 2026-10-17]. Subset navigation: `--target-function my_func
  --upstream-depth=1 --downstream-depth=1` [same README + 2026-10-17]. Claimed
  uses: "Untangling spaghetti code. Identifying orphaned functions. Getting new
  developers up to speed" [raw README + 2026-10-17].
- **Scale.** No LOC caps. Signals: badges "Version 2.5.1", "Coverage 100%"
  [repo page + 2026-10-17]; pipeline step "7. Trim orphaned nodes and groups"
  [same README + 2026-10-17]. Self-hosted demo renders only "a subset of
  itself" (`--downstream-depth=3`) — implicit admission full graphs overwhelm.
- **Honesty: the template to steal.** "Code2flow provides a *pretty good
  estimate* of your project's structure. No algorithm can generate a perfect
  call graph for a dynamic language" [same README + 2026-10-17]. Enumerated
  failures: "Functions with identical names in different namespaces are
  (loudly) skipped"; "imported functions … may not be handled correctly…
  code2flow will link to your local functions"; "Anonymous or generated
  functions are skipped. This includes lambdas and factories"; "If a function
  is renamed … it will be skipped" [same README + 2026-10-17]. MIT licensed
  [raw README + 2026-10-17]. Status: alive, low-churn, no archive banner.
- **History (3rd source).** CHANGELOG lineage: "subsets" in 2.4.0 (2021-12-26);
  async/await + `--include-only` in 2.5.0 (2022-03-25); Ruby+PHP in 2.2.0
  (2021-06-21); AST rewrite in 2.0.0 (2021-04-28); lineage to "0.1 -
  2013-05-23 Initial release"; latest entry 2.5.1 (2023-01-08)
  [https://github.com/scottrogowski/code2flow/blob/master/CHANGELOG.md +
  2026-10-17]. Corroborates low-churn maturity over a decade.
- **Verdict: helpful.** Q1: orphan-trimmed graph + target-function subsets
  answer entry points. Q2: upstream/downstream depth flags are the minimal
  viable blast-radius view to copy. Q3: no weights. Steal the honesty template:
  "pretty good estimate" + enumerated skips + subset flags + caps up front
  ["pretty good estimate" wording per same README + 2026-10-17].

## 3. Gource — eye-candy (negative precedent)

- **Renders.** Animated directory-tree history, not code structure: "animated
  tree with the root directory of the project at its centre. Directories appear
  as branches with files as leaves. Developers can be seen working on the tree"
  [gource.io + 2026-10-17]. Inputs are commit logs (Git/Hg/Bazaar/SVN) [same +
  2026-10-17]. Status: active, "0.56… 6 March 2026" [same + 2026-10-17].
- **Scale.** Via sampling/expiry knobs: "Seconds per day… 10 seconds by
  default"; files "expire" after 60 untouched seconds
  [github.com/acaudwell/Gource/wiki/Controls + 2026-10-17].
- **Honesty: weak.** Bloom/elasticity/colour-seed controls ("Elasticity is a
  fun if somewhat pointless effect" [same wiki + 2026-10-17]) signal
  presentation over fidelity. No semantic claims — and none disclaimed.
- **Verdict: eye-candy.** Q1: no — motion shows recency, not importance. Q2:
  no dependency edges, only tree co-location. Q3: dangerously misleading —
  seconds-per-day playback speed ["Seconds per day… 10 seconds by default",
  same Controls wiki + 2026-10-17] is not CPU/IO cost. Negative precedent
  only: Perfpal maps stay static, quantified, capped — never animated.

## 4. NDepend — maintainer-helpful (.NET, commercial)

- **Renders.** DSM + dependency graph + treemap + dashboard. "Matrix headers'
  elements represent graph boxes" / "Matrix non-empty cells correspond to graph
  arrows" [ndepend.com/docs/dependency-structure-matrix-dsm + 2026-10-17].
  Graph "can scale up to thousands of elements" post-v2020.1 rebuild; before,
  "not able to show more than a few dozens boxes"
  [same DSM docs + 2026-10-17]. Treemap correlates two metrics: "A second code
  metric can be represented by coloring code elements rectangle" (e.g. LOC
  size + coverage color)
  [ndepend.com/docs/treemap-visualization-of-code-metrics + 2026-10-17]. Edge
  drill-down: "Any edge can be double-clicked to obtain a Coupling Graph"
  [ndepend.com/docs/project-dependency-diagram + 2026-10-17].
- **Layering/cycles.** Layered = triangular: "all blue cells in the lower-left
  triangle and all green cells in the upper-right… perfectly layered"; cycles =
  "red square on the DSM" + "unique option Indirect Dependency" where cycle
  members "are indirectly and mutually dependent" [same DSM docs + 2026-10-17].
  Rules as queries: `// <Name>UI layer shouldn't use directly DB types</Name>`
  with `warnif count > 0` [ndepend.com/features/cqlinq + 2026-10-17] — R1
  Family-6 intent via text CQLinq instead of DSM triangles.
- **Scale.** "Application map heuristic actually attempts to show around 1.000
  elements"; "Clustering… even on largest Visual Studio solution with thousands
  of projects", demoed on "176 .NET Core assemblies"; "DSM scales better than
  graph" [project-dependency-diagram + DSM docs + 2026-10-17].
- **Status: commercial.** Per-seat; Developer edition "only runs on Windows";
  14-day trial [ndepend.com/purchase + features/cqlinq + 2026-10-17]. Not OSS:
  copy DSM-pattern + queryable-rules ideas, not UX.
- **Verdict: helpful.** Q1: treemap "spotting too large and too complex methods
  is easy" (size=LOC, color=complexity) [same treemap docs + 2026-10-17]. Q2:
  red-square + indirect-dependency "exhibit properly the 3 cycles, like the
  matrix does" [same DSM docs + 2026-10-17]. Q3: CQLinq debt formula
  "estimates a technical debt proportional to the coupling" [same CQLinq
  features + 2026-10-17] — closest to Perfpal JSON metrics.

## 5. Structure101 — maintainer-helpful, acquired (precedent discounted)

- **Renders.** Directed-graph diagrams + DSM for large graphs + Slice + XS
  metrics + trending: "shows you dependency graphs… either diagrams… or
  dependency matrices"; v2 added "dependency structure matrix ('dsm')
  representation for large dependency graphs"; Slice "see your whole code-base
  at any level of composition" [infoq.com/news/structure101-v2 + 2026-10-17].
- **Layering/cycles.** Tangles-first: "straight to 'tangles' (cyclic
  dependencies) at any level of composition (class, design, jar)"; drill from
  package-depends to "class extends class or method calls method"; architecture
  control via "transformation capability" + per-build "snapshot 'comparison'"
  [same InfoQ + 2026-10-17].
- **Honesty (XS thresholds).** "Excess complexity… Fat" + "Design Tangles"
  [sonarsource Structure101 XS docs + 2026-10-17]; "threshold for Fat at method
  level" = 15, "design tangles… 0% — package-level dependency tangles are
  always considered undesirable" [XS excess-complexity docs + 2026-10-17] —
  the threshold+overage pattern Perfpal should steal for Q3.
- **Scale (vendor-claimed).** "Majority of customers… 25-75k lines of code";
  users "Expedia, European commission, Credit Suisse, Delta airlines, the US
  Airforce and Navy" [same InfoQ + 2026-10-17].
- **Status: acquired, no longer sold.** "Acquired by… Sonar" on "October 15,
  2024"; "Will Structure101 products still be available for sale? No"
  [sonarsource.com/structure101 + press release + 2026-10-17]. Precedent value
  lowered — cannot trial; look to SonarQube architecture analysis instead.
- **Verdict: helpful with Q3 caveat.** Q1: "drill down on the size and
  complexity… prioritize problems in terms of how much they are likely costing
  you" [same InfoQ + 2026-10-17]. Q2: tangles-first beats generic graphs
  (matches R1 DSM partitioning) [same InfoQ + 2026-10-17]. Q3 mixed: trending
  useful, but Fat=15 defaults quote no cost model [same InfoQ + 2026-10-17] —
  Perfpal must attach minutes/dollars, not scores.

## 6. Lattix — maintainer-helpful for Q2 (commercial, DSM-centric)

- **Renders.** DSM primary + Conceptual Architecture Diagrams + LattixWeb
  dashboard/heatmap [lattix.com/products + 2026-10-17]. Canonical rule overlay:
  "Lattix Web now has option to display CAN-USE/CANNOT-USE rule indicator
  triangles" [docs.lattix.com release 2024 + 2026-10-17]; semantics "S2
  can-use S1" vs "S1 cannot-use S2", "evaluated in sequence and can be
  over-ridden" with exceptions + rationale [Lattix DesignRules PDF +
  2026-10-17] — direct template for R1 Family 6.
- **Scale.** Teradyne: "reduce dependencies… move from sequential to parallel
  builds… drop in build times of approximately 15 percent" [lattix.com +
  2026-10-17]. Logo wall (ABB/Boeing/NASA/Siemens/Toyota/USAF) is marketing,
  not numbers [lattix.com/products + 2026-10-17]. Covers "C/C++, Java, .NET,
  Python, UML/SysML" [same + 2026-10-17] — only precedent here touching C/C++.
- **Status: commercial**, Team vs Enterprise, sales-led, no public price.
  Precedent value = interaction design only.
- **Verdict.** Q1 eye-candy risk (heatmap/trending without cost model — don't
  mimic color-only views; judgment on the fetched product pages
  [lattix.com/products + 2026-10-17]). Q2 helpful — partitioned DSM +
  CAN-USE triangles is the clearest layering-violation affordance surveyed;
  adopt [DesignRules semantics above + 2026-10-17]. Q3 helpful-if-quantified
  — pair every violation with seconds/bytes like the Teradyne 15% quote above
  [lattix.com + 2026-10-17].

## 7. CodeSee — eye-candy (vitamin, shut down)

- **Rendered.** "Automatically generating visual maps of code architecture,
  dependencies, and data flows… reduce onboarding time from weeks to days"
  [loot-drop.io/startup/2500-codesee + 2026-10-17].
- **Scale/fate.** "$10M from… Matrix Partners and Boldstart" yet "struggled to
  convert free users to paid… shut down in 2024 after four years" [same +
  2026-10-17]; "Founded → Closed: 2020 → 2024", "$10M", "Developer Tools"
  [ideaproof.io/failure/codesee + 2026-10-17].
- **Honesty: the cautionary quote.** "While code visualization was a useful
  'vitamin'… it failed to address a critical 'painkiller' problem… The market
  prioritizes solutions that directly impact engineering productivity, reduce
  technical debt, or provide critical architectural governance" [same IdeaProof
  + 2026-10-17]. Mechanism: "Modern LLMs like Claude 3.5 Sonnet and GPT-4 can
  parse codebases… GitHub… Copilot" compress the standalone-map niche
  [loot-drop + 2026-10-17].
- **Timeline verified.** "May 14, 2024 — GitKraken… acquired… CodeSee"
  [gitkraken.com press + 2026-10-17]; standalone removed: "On Feb 22nd we'll be
  removing the CodeSee app from the GitHub app store… Delete
  `.github/workflows/codesee-arch-diagram.yml`"
  [docs.codesee.io uninstall + 2026-10-17].
- **Verdict: eye-candy.** Pretty auto-updating diagrams, no quantified
  governance gate [vitamin-vs-painkiller quote per same IdeaProof +
  2026-10-17]. Lesson: maps must carry numbers and CI gates, not pictures.

## 8. madge — maintainer-helpful, scoped (JS)

- **Renders.** "Visual graph of your module dependencies, finding circular
  dependencies, and giving you other useful info" [github.com/pahen/madge +
  2026-10-17]. Colors explicit: "Blue has dependencies, green has no
  dependencies, and red has circular dependencies" [same + 2026-10-17]. Scope:
  JS (AMD/CommonJS/ES6) + CSS preprocessors; "NPM installed dependencies are
  excluded by default"; Node core excluded [npmjs.com/package/madge +
  2026-10-17].
- **Scale.** Solo-maintained since ~2019 (changelog v3.x→v8.0.0)
  [madge CHANGELOG.md + 2026-10-17]. Programmatic API: `.circular()`,
  `.orphans()`, `.depends()`, `.leaves()`, `.dot()`, `.svg()` [repo +
  2026-10-17].
- **Honesty.** Graphviz only needed for pictures — "only required if you want
  to generate visual graphs" [npm README + 2026-10-17]; text/JSON answers work
  headless. No health scores, no AI claims; fails loudly on cycles.
- **Verdict: helpful, scoped.** One painful thing (cycle/orphan detection) with
  machine-readable CI-gatable output [API list per same repo + 2026-10-17].
  Perfpal must exceed: JS-only, file-level, no cost weighting (judgment: graphs
  degrade past ~100 nodes; no vendor number published).

## 9. dependency-cruiser — maintainer-helpful, the model (JS/TS)

- **Renders + validates.** "Runs through the dependencies in any JavaScript,
  TypeScript, LiveScript or CoffeeScript project and … **validates** them
  against (your own) rules … **reports** violated rules — in text (for your
  builds), in graphics (for your eyeballs). As a side effect it can generate
  dependency graphs" [github.com/sverweij/dependency-cruiser + 2026-10-17].
- **Scale: proven, not asserted.** "When your app becomes big, dependency
  graphs on module level will become impractical… can consolidate modules (…+
  dependencies) and colour them" — demoed on yarn berry and facebook/react
  with per-repo howtos [doc/real-world-samples.md + 2026-10-17]; also
  commander/chalk/yargs/tslint/CoffeeScript itself [same + 2026-10-17].
- **Honesty: enforceable architecture.** Config holds "`forbidden`, `allowed`,
  `required` and `options`"; "`forbidden`… dependency-cruiser will emit a
  separate error (warning/ informational) message for each violated rule"
  [doc/rules-reference.md + 2026-10-17]. `--init` ships cycle/orphan/missing/
  prod-on-dev defaults [repo + 2026-10-17]. Self-hosting: "uses itself to
  check on itself in its own build process" [repo + 2026-10-17].
- **Verdict: helpful — the model.** Validation-first ("for your builds"),
  visualization second ("for your eyeballs") [validation-first wording per
  same repo README + 2026-10-17]. Copy: rule names + severities, `err`
  reporter for CI, consolidation (`archi`/`ddot`) for large graphs.

## 10. CodeScene — maintainer-helpful (quantified debt; sourced 7th+)

- **Renders.** File Green/Yellow/Red overlaid with effort: "Green… healthy…
  Yellow… problematic… Red… unhealthy… high maintenance risks" plus
  "Identify Hotspots™… modules with high development activity"
  [codescene.com/improve-code-health + 2026-10-17].
- **Honesty: quantified.** "Code Health… 25+ factors… correlate with increased
  maintenance costs and… defects"; "Moving from 6.0 to 8.0… iterate 30%
  faster"; "15x fewer bugs… 2x faster feature delivery… 9x more likely to
  deliver on time"; "6x more accurate [than SonarQube]… 1.4 million lines of
  manually reviewed code" [codescene.com/product/code-health + 2026-10-17].
  Prioritization: "Not all code is equally important, and not all technical
  debt is bad… pinpoints… where poor code quality actively slows your team
  down"; "Hotspot code health… Average… Worst performer… Trends are
  actionable" [same + 2026-10-17].
- **Methodology (3rd source).** "File-level Hotspots analysis… hotspot maps…
  Combined with CodeHealth™ metrics, CodeScene pinpoints where poor code
  quality slows teams down"; "X-Ray analysis works at the function level…
  prioritized refactoring list"; "Quality gates enforce code standards in
  CI/PRs" [https://codescene.com/pricing + 2026-10-17]. Reinforces the
  hotspot×severity precedent with CI-enforcement teeth.
- **Verdict: helpful.** The precedent Perfpal's cost-weighted map should cite:
  hotspot × severity with PR gates ("Detect code quality issues via automated
  code reviews… Set Quality Gates" [same use-cases page + 2026-10-17]).

## Real-world usage (gh_grep — CI, not demos; 2026-10-17)

- madge `--circular` as gate: `dendronhq/dendron hooks/pre-push.js`
  (`madge --circular --extensions ts .` + `process.exit(1)`);
  `getsentry/sentry-javascript` (`circularDepCheck` script);
  `livestorejs/livestore` (`bunx madge --circular` in lint).
- depcruiser configs in repos: `infinitered/ignite` (`depcruise` + graph
  scripts); `langfuse/langfuse web/.dependency-cruiser.js` consumed
  programmatically in `stats.mjs`.
- code2flow as library: `open-thoughts/open-thoughts` (`import code2flow` →
  call-graph JSON); `mkdocstrings/griffe` (`code2flow(…)` → `dot -Tsvg`);
  `allenai/SERA` vendored.

## Overlap vs what Perfpal already ships (local inventory, exp-2)

| Artifact | Answers | Cap / honesty marker |
|---|---|---|
| `include-graph.svg` | file DAG | top-50 degree/path-asc; `…N more`; cyclic→ARCH-001; 32ch labels |
| `layer-file.svg` | hot-path files | top-50; heat=Σ function_cost; single outline "among shown"; stubs |
| `layer-workspace.svg` | component coupling | CAP=50; heat=density; inter-only (intra on detail pages, captioned) |
| `component-coupling.svg` | blast radius | 50×50; heat=cost edges; LOW-confidence caption; `…more omitted` |
| `hot-components.svg` | look-first ranking | top-50 density bars; zero→2px stubs; max subtitle |
| `component-<rank>-<slug>.svg` | component innards | intra-only; page-local heat; `no files recorded` stub |
| `graph.dot` | machine-readable graphs | sorted/deduped; unresolved excluded; LOW edges dashed |
| `report.md §5` / `report.html` | drill path | HUMAN_CAP=10; `top S of M` / `…+N more`; heat = candidate-cost |

Per-tool overlap-or-gap: Sourcetrail gap (no interactive def-use; static DAGs
only) · code2flow overlap lineage, gap LOW-confidence dashed edges ·
NDepend overlap matrix+ranking, gap no query engine · Structure101 overlap
layering+cycles, gap no enforcement gates · Lattix overlap (DSM already our
most-honest fit) · CodeSee gap (no PR overlays; offline batch) · Gource gap by
design (history = spec non-goal) · madge overlap cycles, gap byte-identical
stubs/caps are ours · depcruise overlap surfacing, gap no rule-config
enforcement.

## Verdict matrix (helpful vs eye-candy)

| Tool | Q1 look-first | Q2 blast-radius | Q3 quantified | Overall |
|---|---|---|---|---|
| Sourcetrail | helpful (search overview) | helpful (ego-graph) | — (no timing) | **helpful** |
| code2flow | helpful (orphans+subsets) | helpful (up/down depth) | — (no weights) | **helpful** |
| Gource | — (recency≠importance) | — (no edges) | misleading (playback≠cost) | **eye-candy** |
| NDepend | helpful (treemap LOC×cx) | helpful (red-square+indirect) | helpful (CQLinq debt) | **helpful** |
| Structure101 | helpful (cost-ranked drill) | helpful (tangles-first) | mixed (thresholds w/o cost) | **helpful*** |
| Lattix | eye-candy risk (color-only) | helpful (DSM+triangles) | helpful-if-quantified | **helpful*** |
| CodeSee | — (no numbers) | — (no gates) | — (vitamin) | **eye-candy** |
| madge | scoped (cycles/orphans) | scoped (file-level) | — (no weights) | **helpful** |
| depcruiser | helpful (consolidation) | helpful (forbidden rules) | helpful (err/CI) | **helpful** |
| CodeScene | helpful (hotspots) | helpful (health×effort) | helpful (gates+debt) | **helpful** |

\* discounted: Structure101 unsellable since Sonar acquisition; Lattix
commercial-only, design-pattern value only.

Net for R4: steal code2flow's honesty template ("pretty good estimate" +
enumerated skips), depcruiser's validation-first posture (text for builds,
pictures for eyeballs), Lattix/NDepend rule overlays (triangles/CQLinq),
CodeScene hotspot×severity gates, Sourcetrail ego-graph interaction (as static
subset views), Structure101 tangles-first navigation; reject Gource motion and
CodeSee picture-without-numbers as anti-patterns.

## Dead ends (support nothing)

- Ubuntu gource manpage → 404; ACM 10.1145/1869542.1869554 → 403;
  pypi.org/project/code2flow → JS-shell failure; code2flow engine.py blob →
  chrome only (lane 1).
- stackoverflow 797303 → 403; Structure101 diagram-spec → 404;
  ndepend.com/pricing → 404 (canonical /purchase) (lane 2).
- None from lane 3 (13/13 URLs opened); CB Insights cited as snippet-only,
  supporting nothing standalone.
- CodeScene docs (`docs.codescene.com/hotspots/hotspots` unreachable;
  `codescene.com/product/hotspots` → 404) — methodology claims use the /pricing
  page instead.

## References (all fetched 2026-10-17)

- Sourcetrail: DOCUMENTATION.md, releases page, repo page, archived
  discontinue blog post (web.archive.org).
- code2flow: README.md (github + raw), repo page, CHANGELOG.md.
- Gource: gource.io, repo page, Controls wiki.
- NDepend: DSM docs, treemap docs, project-dependency-diagram docs, CQLinq
  features, purchase page.
- Structure101: InfoQ v2 news; Sonar Structure101 page + press release; XS
  measuring-complexity + excess-complexity docs.
- Lattix: products page, homepage (Teradyne quote), 2024 release notes,
  DesignRules PDF.
- CodeSee: loot-drop + ideaproof post-mortems, GitKraken acquisition press,
  codesee.io uninstall docs.
- madge: repo README, npm page, CHANGELOG.md.
- dependency-cruiser: repo README, real-world-samples.md, rules-reference.md.
- CodeScene: code-health product page, improve-code-health use cases, pricing page (hotspot/X-Ray/gates methodology).
- Real-world: dendronhq/dendron, getsentry/sentry-javascript,
  livestorejs/livestore, infinitered/ignite, langfuse/langfuse,
  open-thoughts/open-thoughts, mkdocstrings/griffe, allenai/SERA.
