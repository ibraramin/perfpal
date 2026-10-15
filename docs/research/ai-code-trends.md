# AI-Code Trends Study — Reading List & Positioning

> Status: **positioning notes only** — literature review not yet done, study not yet started.
> Purpose: (1) the pre-empting work this study must cite and position against; (2) a draft intro "delta statement"; (3) venues; (4) open risks.
> Instrument: Perfpal (static-analysis CLI for AI-generated codebases — C/Rust/Python done, JS/TS pending).

---

## 1. Reading list — pre-empting & adjacent work (2023–2026)

| # | Work | Venue / Affiliation | Why it matters (one line) | Link |
|---|---|---|---|---|
| 1 | Agarwal, He & Vasilescu — *AI IDEs or Autonomous Agents? Measuring the Impact of Coding Agents on Software Development* | arXiv 2601.13597 · **CMU** | **Most dangerous precedent** — longitudinal, causally-identified (staggered diff-in-diff on AIDev); +~18% static-analysis warnings, +~39% cognitive complexity. | https://arxiv.org/abs/2601.13597 |
| 2 | Coppola, Esposito, Kazman & Lenarduzzi — *AI Writes Code, Humans Pay the Debt* | **ESEM 2026 registered report** · arXiv 2609.04208 · SDU / U. Oulu / U. Hawaii | Pre-commits the "agent-generated vs human commits via static metrics over time" frame (SQuaD, 628k issues). Cite + differentiate. | https://arxiv.org/abs/2609.04208 |
| 3 | GitClear — *AI Copilot Code Quality: 2025 Look Back* | Industry report (US vendor, Bill Harding) | The industry longitudinal benchmark: 211M changed lines; refactor share 25%→<10%, copy/paste 8.3%→12.3%. **Proprietary / unauditable** — cite but don't rely on. | https://www.gitclear.com/ai_assistant_code_quality_2025_research |
| 4 | Xiao, Fan, Calefato, Treude, Kula, Hata & Baltes — *Self-Admitted GenAI Usage in Open-Source Software* | **IEEE TSE** · arXiv 2507.10422 · Kyushu / NAIST / Bari / SMU / Osaka / Shinshu / Heidelberg | Longitudinal churn in 151 repos finds **no general increase** — contradicts the churn narrative; supplies the declared-provenance method. | https://arxiv.org/abs/2507.10422 |
| 5 | Cynthia, Muttakin & Roy — *Beyond Bug Fixes: Post-Merge Code Quality in Agent-Generated PRs* | **MSR 2026** · arXiv 2601.20109 · U. Saskatchewan | Agent-vs-human differences **vanish after churn normalisation** — adopt the normalisation control. | https://arxiv.org/abs/2601.20109 |
| 6 | Li, Zhang & Hassan — *AIDev: Studying AI Coding Agents on GitHub* | arXiv 2602.09185 · **Queen's SAIL** | 932,791 agentic PRs — the de-facto **provenance dataset**; Hassan founded the MSR conference. | https://arxiv.org/abs/2602.09185 |
| 7 | Peng, Kalliamvakou, Cihon & Demirer — *The Impact of AI on Developer Productivity (GitHub Copilot RCT)* | arXiv 2302.06590 | The productivity baseline (55.8% faster) — the "known" result everyone cites; productivity ≠ quality. | https://arxiv.org/abs/2302.06590 |
| 8 | Ben Sghaier, Li, Adams & Hassan — *Don't Blame the LLM: How Agent Harness Evolution Shapes Coding Agent Quality* | **ACM TOSEM** · arXiv 2607.03691 · Queen's SAIL | Controlled 35-release study isolating the **harness** confound — directly relevant to the agentic/harness era. | https://arxiv.org/abs/2607.03691 |
| 9 | Dihan & Muhtasim — *LLMSniffer* (AI-code detector) | arXiv 2604.16058 | AI-code detectors top out at **70–94% accuracy** and generalise poorly → do NOT use a classifier for provenance. | https://arxiv.org/abs/2604.16058 |
| 10 | Chakraborty & Steghöfer — ESEM 2026 registered report | arXiv 2609.04209 | Optional — parallel registered-report in the space. | https://arxiv.org/abs/2609.04209 |
| 11 | Deaconu et al. — EMSE 2026 | arXiv 2608.11513 | Optional — recent EMSE entry in the space. | https://arxiv.org/abs/2608.11513 |

**Reading-list bottom line:** the *frame* ("static analysis of AI vs human code over time") is **derivative**. Novelty survives only on the intersection: era-stratification × performance-pattern lens × multi-language (C/Rust/Python) × provenance rigor.

---

## 2. Delta statement (draft intro paragraph, ~300 words)

The 2025–2026 literature has established that AI-generated code can be measured with static analysis over time, and that in some settings it carries more warnings and cognitive complexity (Agarwal et al., 2026) and a declining refactor share (GitClear, 2025) — while other work finds no general increase in churn once results are normalised (Xiao et al., TSE; Cynthia et al., MSR 2026). This study does not re-litigate that binary. It contributes a narrower, complementary measurement:

1. **Era stratification by model-capability class**, not a binary AI/human split. We treat the last four years as distinct treatment eras — pre-2022 human baseline; first-generation completion (GPT-3-class); conversational code generation (GPT-3.5/4-class); mass adoption (2024–2025); and the agentic/harness era — and measure structural patterns *within* each era. No prior work operationalises model-capability generations as separate strata.

2. **A performance-pattern taxonomy**, not generic smells. Where prior studies use SonarQube-style bug/duplication metrics, we measure performance-relevant structures — allocation inside hot loops, I/O inside loops, pointer-chasing chains, include/coupling fan-out, and cross-file call edges — extracted by a single instrument (Perfpal) held constant across all eras.

3. **Same-repository pre-2022 controls** in **C, Rust, and Python** — systems languages largely absent from the current (Python/JS-heavy) corpus.

4. **A descriptive, no-superiority stance:** we report what patterns emerge, not that AI code is worse.

5. **Honest instrumentation:** because Perfpal's output is "candidates, not certainties", we validate its precision/recall against hand-labelled ground truth before use, and we derive provenance by triangulating *declared* channels (AI git trailers incl. `Assisted-by:`, agent service identities, `refs/notes/ai` attestations, self-admitted usage, AIDev labels) — reporting channel disagreement and carrying a selection-bias limitation rather than relying on an unreliable classifier.

The intended contribution is a rigorous, era-aware, performance-pattern view that **extends AIDev and complements the ESEM 2026 registered report** — not a standalone claim about AI code quality.

---

## 3. Provenance plan (the weak point — declare it, don't hide it)

Converging practice is **declared provenance, triangulated across channels** (not a classifier):

1. **Git trailers** — `Co-authored-by:` (Claude/Copilot/Cursor) and the `Assisted-by:` convention. ⚠ Opt-in and often stripped/banned (e.g. gradle, libkrun reject them) → **systematically under-counts**.
2. **Agent service identity** — bot committer/author identities (`cursoragent@cursor.com`, `noreply@anthropic.com`, `*claude[bot]@users.noreply.github.com`).
3. **Line-level attestation** — `git-ai` authorship notes (`refs/notes/ai`); strongest but rarest signal.
4. **Self-admitted GenAI usage** — explicit mentions in commits/comments/docs (Xiao et al., TSE).
5. **Curated corpora** — AIDev (932k agent PRs), and labelled sets (SWE-bench/SQuaD-style).

**Rule:** triangulate channels, **report overlap + disagreement**, and state the **selection-bias limitation** (declared provenance correlates with project policy, team size, and date — all confounding the era strata).

---

## 4. Venues

- **Conferences:** **MSR** (best fit — data/provenance-centric), **ESEM** (empirical measurement + registered reports), then ICSE / FSE / ASE.
- **Journals:** **EMSE** (canonical fit), **TSE**, **JSS**, **IST**, **JSEP**.
- ⚠ **There is no software-engineering journal abbreviated "JEE"** — verify the intended target before submission.

---

## 5. Open risks / prerequisites

1. **Literature review first** — mandatory before writing a line; the space moved fast in 2025–2026 and this file is only a starting map.
2. **Validate the instrument** — Perfpal's heuristics are "candidates, not certainties"; a hand-labelled precision/recall study is itself a contribution and is required for construct validity.
3. **Provenance is unfalsifiable at the edges** — no reliable AI-code detector exists; lean on declared provenance + triangulation.
4. **Do NOT frame as "static analysis of AI vs human code"** — that is taken (Agarwal et al.; ESEM 2026 RR).
5. **Sequencing / money** — this is a multi-month project (dataset → labelling → validation → statistics → writing); it sits **behind** the cash work (tuition, first Automation-App user, JS/TS completion).

---

*Positioning line to keep in view: for a solo author, the bar is not "beat CMU" — it is "does a reviewer say this adds a useful angle nobody had?" The angle is **era × performance-pattern × C/Rust**, positioned as an extension, not a standalone claim.*
