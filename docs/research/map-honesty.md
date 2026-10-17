# Map Honesty Research (R3)

Scope: cost-without-lying idioms, cap/omission markers, and maintainer
information-needs literature for the maintainer birdseye map. Answers how
Perfpal shows static cost hints without claiming runtime truth, how every cut
is marked, and what maintainers actually ask. Feeds R4
(`docs/research/map-decision.md` + ADR).

Provenance: 3 parallel lanes, all pages fetched 2026-10-17 (LL-012
fetch→quote→cite discipline). This doc uses ONLY the fetched packets listed
under References. Every external claim below carries [full URL + date].
Unopenable pages are listed under Dead ends and support nothing. Remainder
conventions (`showing top S of M`, `…+N more`, aggregated-edge counts) are
novel synthesis where marked — no fetched graph-viz source showed them.

## 1. Cost-without-lying idioms (static ≠ runtime truth)

Static estimators must label themselves as estimates; uncertainty is encoded,
not hidden; severity and relevance are glued, never shown alone.

- **llvm-mca is static measurement against a model, not silicon truth:** "llvm-mca is a performance analysis tool that uses information available in LLVM (e.g. scheduling models) to statically measure the performance of machine code in a specific CPU." [https://www.llvm.org/docs/CommandGuide/llvm-mca.html + 2026-10-17]
- **llvm-mca quality is bounded by model quality:** "By design, the quality of the analysis conducted by llvm-mca is inevitably affected by the quality of the scheduling models in LLVM." [https://www.llvm.org/docs/CommandGuide/llvm-mca.html + 2026-10-17]
- **llvm-mca omits whole pipeline stages by design:** "llvm-mca assumes that instructions have all been decoded and placed into a queue before the simulation start. Therefore, the instruction fetch and decode stages are not modeled. Performance bottlenecks in the frontend are not diagnosed. Also, llvm-mca does not model branch prediction." [https://www.llvm.org/docs/CommandGuide/llvm-mca.html + 2026-10-17]
- **code2flow labels its output an estimate, not a graph:** "Code2flow provides a pretty good estimate of your project's structure. No algorithm can generate a perfect call graph for a dynamic language – even less so if that language is duck-typed." [https://github.com/scottrogowski/code2flow/blob/master/README.md + 2026-10-17]
- **code2flow restates the impossibility:** "Code2flow approximates the structure of projects in dynamic languages. It is not possible to generate a perfect callgraph for a dynamic language." [https://github.com/scottrogowski/code2flow/blob/master/README.md + 2026-10-17]
- **code2flow names the runtime-unknowable case:** "We have no way of knowing whether `func` will point to `func_a` or `func_b` until runtime." [https://github.com/scottrogowski/code2flow/blob/master/README.md + 2026-10-17]
- **code2flow enumerates skip rules instead of silent drops:** "All functions without definitions are skipped… Functions with identical names in different namespaces are (loudly) skipped… Anonymous or generated functions are skipped. This includes lambdas and factories… If a function is renamed, either explicitly or by being passed around as a parameter, it will be skipped." [https://github.com/scottrogowski/code2flow/blob/master/README.md + 2026-10-17]
- **Flame graphs are sampled coarseness, honestly labelled:** "Profiling by sampling at a fixed rate is a coarse but effective way to see which code-paths are hot (busy on-CPU)." [https://www.brendangregg.com/FlameGraphs/cpuflamegraphs.html + 2026-10-17]
- **Flame-graph x-axis is not time — alphabetical merge order:** "The x-axis spans the sample population. It does not show the passing of time from left to right, as most graphs do. The left to right ordering has no meaning (it's sorted alphabetically to maximize frame merging)." [https://www.brendangregg.com/FlameGraphs/cpuflamegraphs.html + 2026-10-17]
- **Flame-graph width is sample-count ancestry, call count unknown:** "The width of the box shows the total time it was on-CPU or part of an ancestry that was on-CPU (based on sample count). Functions with wide boxes may consume more CPU per execution than those with narrow boxes, or, they may simply be called more often. The call count is not shown (or known via sampling)." [https://www.brendangregg.com/FlameGraphs/cpuflamegraphs.html + 2026-10-17]
- **Flame-graph axis restatement:** "The x-axis shows the stack profile population, sorted alphabetically (it is not the passage of time), and the y-axis shows stack depth" [https://www.brendangregg.com/flamegraphs.html + 2026-10-17]
- **Uncertainty idiom — error matters as much as the mean:** "When making an inference or comparison with uncertain, noisy, or incomplete data, measurement error and confidence intervals can be as important for judgment as the actual mean values of different groups." [https://graphics.cs.wisc.edu/Papers/2014/CG14/ + 2026-10-17]
- **Uncertainty idiom — encoding changes decisions:** "We present a series of crowd-sourced experiments that confirm that the encoding of mean and error significantly changes how viewers make decisions about uncertain data." [https://graphics.cs.wisc.edu/Papers/2014/CG14/ + 2026-10-17]
- **Uncertainty idiom — transparency for confidence, width for distribution:** "We suggest the use of gradient plots (which use transparency to encode uncertainty) and violin plots (which use width) as better alternatives for inferential tasks than bar charts with error bars." [https://graphics.cs.wisc.edu/Papers/2014/CG14/ + 2026-10-17]
- **Severity×relevance gluing — Hotspots are priority (relevance):** "Hotspots: This is the relevance of the findings – the priority. The metric is calculated from the development activity in the code." [https://docs.enterprise.codescene.io/versions/6.2.5/guides/technical/hotspots.html + 2026-10-17]
- **Severity×relevance gluing — Code Health is severity:** "Low Code Health: This is the severity of any hotspot. The lower the code health, the more red the corresponding circle that represents the hotspot." [https://docs.enterprise.codescene.io/versions/6.2.5/guides/technical/hotspots.html + 2026-10-17]
- **Severity×relevance gluing — prioritize the intersection:** "To prioritize hotspots, CodeScene employs algorithms that look at deeper change patterns in the analysis data. CodeScene starts by identifying files with high change frequency and low Code Health." [https://docs.enterprise.codescene.io/versions/6.2.5/guides/technical/hotspots.html + 2026-10-17]
- **Severity×relevance gluing — cheap-bad vs expensive-bad:** "Low code health in a development hotspot is expensive… Low code health in stable parts of the codebase, i.e. non-hotspots, has lower priority." [https://docs.enterprise.codescene.io/versions/6.2.5/guides/technical/hotspots.html + 2026-10-17]
- **Severity×relevance gluing — health is a 25+ factor aggregate:** "Code Health is an aggregated metric based on 25+ factors scanned from the source code. The code health factors correlate with increased maintenance costs and an increased risk for defects." [https://docs.enterprise.codescene.io/latest/guides/technical/code-health.html + 2026-10-17]
- **Severity×relevance gluing — weighted hotspot health is the critical metric:** "Hotspot Code Health: A weighted average of the code health in your hotspots. Generally, this is the most critical metric since low code health in a hotspot will be expensive." [https://docs.enterprise.codescene.io/latest/guides/technical/code-health.html + 2026-10-17]

## 2. Cap/omission markers (top-N, aggregation, stubs, truncation)

Every cut has a named control and a visible marker; empty states are stubs with
a next step, never blanks; truncated labels keep a readable stem plus a
full-string fallback.

- **madge depth caps dependency levels pre-output:** "a depth limit feature to control how many dependency levels are included in the output graph. When the `depth` option is specified, only modules within that number of steps from the input modules are included, while maintaining proper connections by bypassing deep dependencies that reference back to non-deep modules." [https://github.com/pahen/madge/pull/318 + 2026-10-17]
- **dependency-cruiser maxDepth keeps visualization manageable:** "Only cruise the specified depth, counting from the specified root-module(s). This command is mostly useful in combination with visualization output like *dot* to keep the generated output to a manageable size." [https://github.com/sverweij/dependency-cruiser/blob/main/doc/options-reference.md + 2026-10-17]
- **dependency-cruiser warns caps are for pictures, not validation:** "The maxDepth option is there to help with visualizing. If your goal is to *validate* this option is best left alone as you'll miss a dependency or two otherwise." [https://github.com/sverweij/dependency-cruiser/blob/main/doc/options-reference.md + 2026-10-17]
- **dependency-cruiser focus shows matches plus neighbours:** "Just like the includeOnly option, focus takes a regular expressions you want dependency-cruiser to show in its output. In addition dependency-cruiser will include all neighbours of those modules. By default these will be direct dependencies and direct dependents - if you want more, you can use the `depth` attribute." [https://github.com/sverweij/dependency-cruiser/blob/main/doc/options-reference.md + 2026-10-17]
- **dependency-cruiser collapsePattern folds matches to one node:** "The dot reporter also supports the collapsePattern option originally created for the archi reporter, which summarizes modules matching the pattern to one node in the output." [https://github.com/sverweij/dependency-cruiser/blob/main/doc/options-reference.md + 2026-10-17]
- **dependency-cruiser archi reporter summarizes to folders:** "The 'customizable dot' (cdot) or 'archi' reporter exists to make high level dependency overviews. Out of the box it recognises structures that summarise to folders directly under *packages*, *src*, *lib*, and *node_modules*." [https://github.com/sverweij/dependency-cruiser/blob/main/doc/options-reference.md + 2026-10-17]
- **Graphviz sfdp is the large-graph layout escape hatch:** "`sfdp` is a fast, multilevel, force-directed algorithm that efficiently layouts large graphs" [https://www.graphviz.org/docs/layouts/sfdp/ + 2026-10-17]
- **Cytoscape LOD drops labels above a node threshold for speed:** "Cytoscape is able to display large networks (> 10,000 nodes) while maintaining interactive speed. To accomplish this goal the **level of detail (LOD)** used to display the network is chosen based on the number of nodes and edges currently being displayed. For example, by default, node labels (if present) are only rendered when less than 200 nodes are visible because drawing text is a relatively expensive operation." [https://manual.cytoscape.org/en/latest/Rendering_Engine.html + 2026-10-17]
- **Cytoscape LOD flip-flops visibly at the threshold:** "This can create some unusual behavior. If the screen currently contains 198 nodes, node labels will be displayed. If you pan across the network, such that now 201 nodes are displayed, the node labels will disappear." [https://manual.cytoscape.org/en/latest/Rendering_Engine.html + 2026-10-17]
- **cytoscape.js labels-are-expensive rule:** "**Labels** : Drawing labels is expensive. If you can go without them or show them on tap/mouseover, you'll get better performance. Consider not having labels for edges. Consider setting `min-zoomed-font-size` in your style so that when labels are small — and hard to read anyway — they are not rendered." [https://github.com/cytoscape/cytoscape.js/blob/master/documentation/md/performance.md + 2026-10-17]
- **SAP empty-state minimum — headline plus description:** "Empty states must have a message (headline with supporting description). An illustration and call-to-action are optional." [https://www.sap.com/design-system/fiori-design-web/v1-145/foundations/best-practices/global-patterns/designing-for-empty-states + 2026-10-17]
- **SAP empty-state must explain and offer a next step:** "Empty states should never feel empty or negative… It is important to tell the user what the empty state is for, why the user is seeing it, and what the user can do next." [https://www.sap.com/design-system/fiori-design-web/v1-145/foundations/best-practices/global-patterns/designing-for-empty-states + 2026-10-17]
- **SAP small-surface rule — text only when space is limited:** "Do not use a UX illustration for UI elements that are smaller than a medium size card, such as tiles, message toasts, message strips, or other smaller UI elements. Use a headline and supporting message only." [https://www.sap.com/design-system/fiori-design-web/v1-145/foundations/best-practices/global-patterns/designing-for-empty-states + 2026-10-17]
- **SAP space-limited fallback:** "When space is limited, use text only." [https://www.sap.com/design-system/fiori-design-web/v1-145/foundations/best-practices/global-patterns/designing-for-empty-states + 2026-10-17]
- **Carbon truncation contract — ellipsis, stem, tooltip:** "Truncated items are represented by an ellipsis `...` and should represent three or more truncated characters in a text string. There must be at least four characters of non-truncated content in a truncated string. Truncated items always include a browser tooltip on hover to show the entire string, name, or phrase that the ellipsis is representing. The only time a browser tooltip does not need to be used is at the end of a truncated paragraph." [https://www.carbondesignsystem.com/patterns/overflow-content/ + 2026-10-17]
- **Carbon Show-more vs Load-more for overflow:** "The 'Show more' button is used when there is a significant amount of overflow content… 'Show more' can also be presented as 'Load more' in cases where performance is a concern." [https://www.carbondesignsystem.com/patterns/overflow-content/ + 2026-10-17]
- **cytoscape.js ellipsis truncation primitive:** "`text-wrap` : A wrapping style to apply to the label text; may be `none` for no wrapping (including manual newlines: `\n`), `wrap` for manual and/or autowrapping or `ellipsis` to truncate the string and append '...' based on `text-max-width`." [https://github.com/cytoscape/cytoscape.js/commit/306198a7e0fc248de8ae32f9d90153da9bb95397 + 2026-10-17]
- **code2flow group-vs-node folding:** "Recursively separate groups and nodes. Groups are files, modules, or classes. More precisely, groups are namespaces where functions live. Nodes are the functions themselves." [https://github.com/scottrogowski/code2flow/blob/master/README.md + 2026-10-17]
- **code2flow orphan trimming:** "Trim orphaned nodes and groups." [https://github.com/scottrogowski/code2flow/blob/master/README.md + 2026-10-17]

## 3. Maintainer-needs literature (7 venue papers)

Each paper: finding, supporting quote, full dated cite. All quotes from pages
actually fetched 2026-10-17.

### P1 — Ko, DeLine, Venolia (ICSE 2007): information needs at Microsoft

- **Finding:** the most-sought needs are artifact/coworker awareness; the most-deferred are design rationale and program behavior, blocked on unavailable coworkers. [https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/icse07_ko.pdf + 2026-10-17]
- **Quote:** "We identified twenty-one information types and cataloged the outcome and source when each type of information was sought. The most frequently sought information included awareness about artifacts and coworkers. The most often deferred searches included knowledge about design and program behavior, such as why code was written a particular way, what a program was supposed to do, and the cause of a program state. Developers often had to defer tasks because the only source of knowledge was unavailable coworkers." [https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/icse07_ko.pdf + 2026-10-17]
- **Quote (hardest unsatisfied needs with cost):** "The most frequently unsatisfied information needs were the following… 1 What code caused this program state? 61% 21 min; 2 Why was the code implemented this way? 44% 21 min; 3 In what situations does this failure occur? 41% 49 min" [https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/icse07_ko.pdf + 2026-10-17]
- **Quote (coworkers as top source):** "Coworkers were the most frequent source of information, accessed at least once for 13 of the 21 information needs and in 83 of the 334 instances of information seeking." [https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/icse07_ko.pdf + 2026-10-17]

### P2 — Sillito, Murphy, De Volder (FSE 2006): questions programmers ask

- **Finding:** 44 question kinds; answers routinely require combining multiple tool result sets; developers lose calling context and crave an architecture overview document. [https://www.cs.ubc.ca/~murphy/papers/other/asking-answering-fse06.pdf + 2026-10-17]
- **Quote:** "Based on this analysis we catalog and categorize 44 different kinds of questions asked by our participants." [https://www.cs.ubc.ca/~murphy/papers/other/asking-answering-fse06.pdf + 2026-10-17]
- **Quote (multi-tool correlation burden):** "Participants, at times, needed to combine result sets or other information from multiple tools to answer their questions." [https://www.cs.ubc.ca/~murphy/papers/other/asking-answering-fse06.pdf + 2026-10-17]
- **Quote (lost context N6 + overview hunger N9):** "N6 "I was starting to forget who was calling what, especially because there is only one search panel at a time that I can see" + N9 "I think I would need some kind of overview document that says… this is the architecture of how the thing works and the main classes involved"" [https://www.cs.ubc.ca/~murphy/papers/other/asking-answering-fse06.pdf + 2026-10-17]

### P3 — Fritz, Murphy (ICSE 2010 NIER / InfoFragments): compose the fragments

- **Finding:** 78 wanted-but-unsupported questions; single-chain tools force developers to mentally correlate across chains; a composed-fragment prototype answered 94% in 2.3 min mean. [https://www.cs.ubc.ca/labs/spl/papers/2010/icse10-infofrags.pdf + 2026-10-17]
- **Quote:** "Through interviews with eleven professional developers, we identified 78 questions developers want to ask, but for which support is lacking." [https://www.cs.ubc.ca/labs/spl/papers/2010/icse10-infofrags.pdf + 2026-10-17]
- **Quote (single-chain bottleneck):** "only one chain of linked information can be followed at a time; if multiple chains are needed to answer the question… the burden is placed on the developer to mentally and manually correlate the information reached." [https://www.cs.ubc.ca/labs/spl/papers/2010/icse10-infofrags.pdf + 2026-10-17]
- **Quote (prototype payoff):** "All developers were able to easily use the prototype to successfully answer 94% of questions in a mean time of 2.3 minutes per question." [https://www.cs.ubc.ca/labs/spl/papers/2010/icse10-infofrags.pdf + 2026-10-17]

### P4 — LaToza, Myers (ICSE 2010): reachability questions dominate bugs

- **Finding:** reachability (feasible-path search to matching statements) tracks half of inserted bugs and 9-of-10 longest activities; practitioners ask them 9+ times daily and rate them hard. [https://pldb.kirancodes.me/p/conf/icse/LaTozaM10 + 2026-10-17]
- **Quote (definition):** "A reachability question is a search across feasible paths through a program for target statements matching search criteria." [https://pldb.kirancodes.me/p/conf/icse/LaTozaM10 + 2026-10-17]
- **Quote (frequency + difficulty + bug share):** "half of the bugs developers inserted were associated with reachability questions" + "9 of the 10 longest activities were associated with reachability questions" + "460 professional software developers reported asking questions that may be answered using reachability questions more than 9 times a day, and 82% rated one or more as at least somewhat hard to answer." [https://www.sciweavers.org/publications/developers-ask-reachability-questions + 2026-10-17]

### P5 — Bacchelli, Bird (ICSE 2013): review is understanding, defects are 4th

- **Finding:** code/change understanding is the key review activity and tools miss it; the hardest part is the reason for the change; defect comments rank only fourth at 14%. [https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/ICSE202013-codereview.pdf + 2026-10-17]
- **Quote:** "we find that code and change understanding is the key aspect of code reviewing and that developers employ a wide range of mechanisms to meet their understanding needs, most of which are not met by current tools." [https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/ICSE202013-codereview.pdf + 2026-10-17]
- **Quote (rationale pain):** "'the most difficult thing when doing a code review is understanding the reason of the change'" [https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/ICSE202013-codereview.pdf + 2026-10-17]
- **Quote (defects are not the bulk):** "the category 'defect' is only the fourth most frequent… with 78 (14%) comments" [https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/ICSE202013-codereview.pdf + 2026-10-17]

### P6 — Begel, Zimmermann (ICSE 2014): usage and impact top the wish-list

- **Finding:** practitioners rank customer-usage questions highest (80.0% Essential), with change-impact (52.1%) and risk-measurement (47.8%) close behind — Q1/Q2-shaped demand. [https://andrewbegel.com/papers/begel-icse-2014.pdf + 2026-10-17]
- **Quote:** "Respondents favored questions that focus on how customers typically use their applications." [https://andrewbegel.com/papers/begel-icse-2014.pdf + 2026-10-17]
- **Quote (three ranked Essentials):** "Q27 How do users typically use my application? 80.0% Essential"; "Q40 What is the impact of a code change or requirements change to the project and tests? 52.1% Essential"; "Q42 What tools can help us measure and estimate the risk associated with code changes? 47.8% Essential" [https://andrewbegel.com/papers/begel-icse-2014.pdf + 2026-10-17]

### P7 — Cornelissen et al. (ICPC 2007, EXTRAVIS): mental map + phase view

- **Finding:** maintainers build a mental map from functionality to code; up to 50% of maintenance time is comprehension; a phase view (input→calculation→output) correctly revealed architecture. [https://azaidman.github.io/publications/cornelissenICPC2007.pdf + 2026-10-17]
- **Quote:** "the software engineer attempts to build a mental map that relates the system's functionality and concepts to its source code" [https://azaidman.github.io/publications/cornelissenICPC2007.pdf + 2026-10-17]
- **Quote (comprehension cost):** "Corbi reports that up to 50% of the time allocated for a maintenance task is spent on gaining knowledge of the software system at hand." [https://azaidman.github.io/publications/cornelissenICPC2007.pdf + 2026-10-17]
- **Quote (phase-view payoff):** "The massive sequence view indicates that there are three major 'phases'… (1) an input phase, (2) a calculation phase, and (3) an output phase. This proved to be correct" [https://azaidman.github.io/publications/cornelissenICPC2007.pdf + 2026-10-17]

## 4. Implications for Perfpal's 3 questions

Q1 look-first, Q2 blast-radius, Q3 cost-hints-to-profile; plus the
cross-cutting rule that findings ride on the map, never as bare lists.

- **Q1 look-first = severity×relevance, not raw counts:** rank hot-spots per KLOC by intersecting finding density (severity) with change frequency (relevance), because hotspot priority is defined as relevance from development activity while low health is defined as severity, and the method starts by identifying high-change low-health files. [https://docs.enterprise.codescene.io/versions/6.2.5/guides/technical/hotspots.html + 2026-10-17]
- **Q1 look-first demotes stable-but-ugly code:** low health in stable non-hotspots gets lower priority than the same health in a hotspot, because low health in a hotspot is expensive while low health in stable code has lower priority. [https://docs.enterprise.codescene.io/versions/6.2.5/guides/technical/hotspots.html + 2026-10-17]
- **Q1 look-first answers the top-ranked practitioner need:** usage-weighted ranking speaks to the 80.0% Essential usage question, since respondents favored questions about how customers typically use applications. [https://andrewbegel.com/papers/begel-icse-2014.pdf + 2026-10-17]
- **Q1 look-first needs an overview document, not a search panel:** the birdseye map is that overview, because one participant forgot who-called-what with a single search panel while another explicitly wanted an overview document of architecture and main classes. [https://www.cs.ubc.ca/~murphy/papers/other/asking-answering-fse06.pdf + 2026-10-17]
- **Q2 blast-radius = bounded upstream/downstream + aggregated coupling:** show focus-matches plus direct neighbours with a depth control, because focus includes all neighbours by default (direct dependencies and dependents) with an optional deeper depth attribute. [https://github.com/sverweij/dependency-cruiser/blob/main/doc/options-reference.md + 2026-10-17]
- **Q2 blast-radius caps depth pre-layout and says so:** apply a madge-style depth limit that includes only modules within N steps while bypassing deep back-references, because that is the documented depth-limit behavior. [https://github.com/pahen/madge/pull/318 + 2026-10-17]
- **Q2 blast-radius folds instead of hairballing:** summarize matched modules to one node / folder-level overviews, because collapsePattern summarizes matches to one node and the archi reporter summarizes to folders under packages/src/lib/node_modules. [https://github.com/sverweij/dependency-cruiser/blob/main/doc/options-reference.md + 2026-10-17]
- **Q2 blast-radius directly serves reachability demand:** feasible-path upstream/downstream views answer the daily-hard reachability class, because reachability is defined as search across feasible paths and practitioners report asking such questions 9+ times daily with 82% rating them hard. [https://www.sciweavers.org/publications/developers-ask-reachability-questions + 2026-10-17]
- **Q3 cost-hints = static candidates with llvm-mca-style disclaimers:** label every heat as candidate density/cost to profile, because llvm-mca quality is inevitably affected by scheduling-model quality and it omits fetch/decode plus branch prediction by design. [https://www.llvm.org/docs/CommandGuide/llvm-mca.html + 2026-10-17]
- **Q3 cost-hints never imply time on the x-axis or precision on width:** axis/width legends must state the sampling-style caveat, because the x-axis spans the sample population rather than time and width reflects sample-count ancestry with unknown call counts. [https://www.brendangregg.com/FlameGraphs/cpuflamegraphs.html + 2026-10-17]
- **Q3 cost-hints encode confidence with transparency/width, not hidden:** use gradient transparency for uncertainty and width for distributions per the crowd-sourced result, because encoding of mean and error significantly changes decisions and gradient/violin plots beat bar-with-error-bars for inference. [https://graphics.cs.wisc.edu/Papers/2014/CG14/ + 2026-10-17]
- **Cross-cutting lists-fail — findings ride on the map:** glue severity+confidence to each annotation in place, because participants needed to combine multiple tool result sets, single-chain tools push correlation burden onto developers, and a composed prototype answered 94% in 2.3 min mean. [https://www.cs.ubc.ca/labs/spl/papers/2010/icse10-infofrags.pdf + 2026-10-17]
- **Cross-cutting lists-fail — rationale is the gap:** annotate why-suspect reasons alongside what/where, because the most-deferred searches were why-code and program-behavior knowledge blocked on unavailable coworkers. [https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/icse07_ko.pdf + 2026-10-17]
- **Cross-cutting lists-fail — review needs understanding, not defect lists:** the map carries change-reason context, because understanding is the key review aspect tools miss and defect comments were only fourth at 14%. [https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/ICSE202013-codereview.pdf + 2026-10-17]

## 5. Stealable legend wordings (exact strings + source)

Verbatim source strings Perfpal can paste into legends/captions/stubs. Each is
an exact quote; adaptation notes are synthesis, citations are the source.

- **Estimate banner:** "Code2flow provides a pretty good estimate of your project's structure." — steal as `pretty good estimate — static, profile to confirm` [https://github.com/scottrogowski/code2flow/blob/master/README.md + 2026-10-17]
- **Impossibility footnote:** "No algorithm can generate a perfect call graph for a dynamic language" — steal for LOW-confidence edge captions [https://github.com/scottrogowski/code2flow/blob/master/README.md + 2026-10-17]
- **Runtime-unknown edge label:** "We have no way of knowing whether `func` will point to `func_a` or `func_b` until runtime." — steal for unresolved-call captions [https://github.com/scottrogowski/code2flow/blob/master/README.md + 2026-10-17]
- **Skip-rule caption:** "Functions with identical names in different namespaces are (loudly) skipped" — steal as loud-skip marker wording [https://github.com/scottrogowski/code2flow/blob/master/README.md + 2026-10-17]
- **Model-quality disclaimer:** "the quality of the analysis conducted by llvm-mca is inevitably affected by the quality of the scheduling models in LLVM" — steal as `cost hints reflect the static model, not silicon` [https://www.llvm.org/docs/CommandGuide/llvm-mca.html + 2026-10-17]
- **Axis honesty:** "The x-axis spans the sample population. It does not show the passing of time from left to right" — steal pattern for any sorted axis (`sorted alphabetically to maximize frame merging` analogue: `sorted by degree, not importance`) [https://www.brendangregg.com/FlameGraphs/cpuflamegraphs.html + 2026-10-17]
- **Width honesty:** "The call count is not shown (or known via sampling)" — steal as `call counts unknown statically` [https://www.brendangregg.com/FlameGraphs/cpuflamegraphs.html + 2026-10-17]
- **Relevance legend:** "Hotspots: This is the relevance of the findings – the priority. The metric is calculated from the development activity in the code." — steal verbatim for hotspot heat [https://docs.enterprise.codescene.io/versions/6.2.5/guides/technical/hotspots.html + 2026-10-17]
- **Severity legend:** "Low Code Health: This is the severity of any hotspot." — steal verbatim for severity heat [https://docs.enterprise.codescene.io/versions/6.2.5/guides/technical/hotspots.html + 2026-10-17]
- **Validation-vs-pictures rule:** "The maxDepth option is there to help with visualizing. If your goal is to *validate* this option is best left alone" — steal as `caps apply to pictures; counts/validation use full data` [https://github.com/sverweij/dependency-cruiser/blob/main/doc/options-reference.md + 2026-10-17]
- **Truncation marker:** "Truncated items are represented by an ellipsis `...`" + "Truncated items always include a browser tooltip on hover to show the entire string" — steal as `… + <title> full string` (SVG `<title>` = tooltip) [https://www.carbondesignsystem.com/patterns/overflow-content/ + 2026-10-17]
- **Empty-state stub:** "Empty states must have a message (headline with supporting description)." + "tell the user what the empty state is for, why the user is seeing it, and what the user can do next" — steal as `no files/components recorded — <why> — <next>` [https://www.sap.com/design-system/fiori-design-web/v1-145/foundations/best-practices/global-patterns/designing-for-empty-states + 2026-10-17]

## 6. Dead ends (support nothing)

Every page below failed to open or yielded snippets only. Nothing here backs
any claim above.

- CSE442 PDF — download-limit, body not persisted.
- Gregg JavaOne PDF — unopened (companion to flame-graph pages).
- Gregg flamescope — unopened.
- SBLP venue page — snippet-only, no fetchable body.
- arXiv 2411.13854 — snippet-only, no fetchable body.
- code2flow.com/docs — snippet-only, README used instead.
- flint-chart — JS-rendered, chrome-only, no readable content.
- js.cytoscape.org labels anchor — truncated, cytoscape.js performance doc used instead.
- Carbon non-www host — 301, canonical carbondesignsystem.com used instead.
- Gephi tutorial — redirect, unexamined.
- D3 websearch — provider 429, unexamined.
- ACM dl.acm.org Fritz (1806828) — paywall, open UBC-hosted PDF used instead.
- ACM dl.acm.org LaToza (1806829) — paywall, sciweavers + pldb mirrors used instead.
- IEEE 4497212 — paywall, no open mirror fetched.
- GMU tlatoza PDF (TLS cert fail) — unopened, mirrors used instead.

## 7. Gaps (explicit)

- NO fetched graph-viz source showed an in-chart `…+N more` remainder-node convention (novel synthesis for R4).
- NO fetched graph-viz source showed a `showing top S of M` caption convention (novel synthesis for R4).
- NO fetched source captions aggregated edges with counts (e.g. `12 files × 40 includes`) — novel synthesis for R4.
- D3 gallery/examples unexamined (provider 429); Gephi tutorials unexamined (redirects) — layout idioms from those families are uncovered.
- No fetched source quantifies the readability threshold for label hiding in static SVG (Cytoscape's 200-node LOD is interactive-only); Perfpal's static label policy (tail-kept 32ch + tooltip) is synthesis.
- Uncertainty-encoding evidence is screen/interactive (gradient/violin plots); no fetched source tests transparency/width encodings inside static SVG architecture maps.

## 8. References (every URL fetched 2026-10-17)

- llvm-mca static model — https://www.llvm.org/docs/CommandGuide/llvm-mca.html + 2026-10-17.
- code2flow README (estimate + skip rules + grouping + orphan trim) — https://github.com/scottrogowski/code2flow/blob/master/README.md + 2026-10-17.
- Gregg CPU flame graphs — https://www.brendangregg.com/FlameGraphs/cpuflamegraphs.html + 2026-10-17.
- Gregg flame graphs hub — https://www.brendangregg.com/flamegraphs.html + 2026-10-17.
- Correll & Gleicher uncertainty (CG14) — https://graphics.cs.wisc.edu/Papers/2014/CG14/ + 2026-10-17.
- CodeScene hotspots — https://docs.enterprise.codescene.io/versions/6.2.5/guides/technical/hotspots.html + 2026-10-17.
- CodeScene code health — https://docs.enterprise.codescene.io/latest/guides/technical/code-health.html + 2026-10-17.
- madge depth PR — https://github.com/pahen/madge/pull/318 + 2026-10-17.
- dependency-cruiser options (maxDepth, focus/depth, collapsePattern, archi) — https://github.com/sverweij/dependency-cruiser/blob/main/doc/options-reference.md + 2026-10-17.
- Graphviz sfdp — https://www.graphviz.org/docs/layouts/sfdp/ + 2026-10-17.
- Cytoscape rendering engine / LOD — https://manual.cytoscape.org/en/latest/Rendering_Engine.html + 2026-10-17.
- cytoscape.js performance (labels) — https://github.com/cytoscape/cytoscape.js/blob/master/documentation/md/performance.md + 2026-10-17.
- SAP empty states — https://www.sap.com/design-system/fiori-design-web/v1-145/foundations/best-practices/global-patterns/designing-for-empty-states + 2026-10-17.
- Carbon overflow content — https://www.carbondesignsystem.com/patterns/overflow-content/ + 2026-10-17.
- cytoscape.js ellipsis commit — https://github.com/cytoscape/cytoscape.js/commit/306198a7e0fc248de8ae32f9d90153da9bb95397 + 2026-10-17.
- Ko DeLine Venolia ICSE07 — https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/icse07_ko.pdf + 2026-10-17.
- Sillito Murphy De Volder FSE06 — https://www.cs.ubc.ca/~murphy/papers/other/asking-answering-fse06.pdf + 2026-10-17.
- Fritz Murphy ICSE10 InfoFragments — https://www.cs.ubc.ca/labs/spl/papers/2010/icse10-infofrags.pdf + 2026-10-17.
- LaToza Myers ICSE10 definition mirror — https://pldb.kirancodes.me/p/conf/icse/LaTozaM10 + 2026-10-17.
- LaToza Myers reachability stats mirror — https://www.sciweavers.org/publications/developers-ask-reachability-questions + 2026-10-17.
- Bacchelli Bird ICSE13 — https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/ICSE202013-codereview.pdf + 2026-10-17.
- Begel Zimmermann ICSE14 — https://andrewbegel.com/papers/begel-icse-2014.pdf + 2026-10-17.
- Cornelissen et al. ICPC07 EXTRAVIS — https://azaidman.github.io/publications/cornelissenICPC2007.pdf + 2026-10-17.
