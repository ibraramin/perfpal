# Perfpal Website — Scope Brief & Content Spec

> **Status:** v1 draft (session 23). Source of truth for the website build.
> **Build repo:** `~/Perfpal-Website` — a **separate repository**, versioned and
> released independently, developed by its own session family. THIS document is
> the handoff input; product code stays in the Perfpal repo.

## 1. Purpose & audience

**Purpose:** an attractive, eye-catching marketing/showcase site that communicates the
**entire scope** of Perfpal — what is implemented today **and** what is planned — so it
works as a product page, a technical showcase, and a founder-funding artifact.

**Audiences (in priority order):**
1. **Funders / investors / grant reviewers** — needs credibility, scope, differentiation, roadmap, founder story.
2. **Developers** — needs what-it-is, how-it-works, heuristics, quickstart, download.
3. **Employers / general** — needs polish and a clear one-line value proposition.

**Non-goals:** an application UI, auth, accounts, dashboards, a hosted service, docs-hostile CMS, or any feature that implies a SaaS product.

## 2. Scope rule: shipped vs planned

The site presents the full vision but **never blurs shipped and planned**:
- Anything implemented is marked **Shipped** (C backend; 23 heuristics; charts; metrics.json v1.1).
- Anything planned is marked **Planned** with its milestone (Rust backend → `0.2.0`; proprietary beta → `0.2.0-beta.1`; C++/Python → later).
- A single visible legend/badge system (`Shipped` / `Planned`) is used site-wide; no claim may appear as shipped when it is planned.

## 3. Sitemap

| # | Page | Route | Priority |
|---|---|---|---|
| 1 | Home | `/` | P0 |
| 2 | How it works | `/how-it-works` | P0 |
| 3 | Heuristics catalog | `/heuristics` | P0 |
| 4 | Visual output | `/visualization` | P1 |
| 5 | Dogfooding | `/dogfooding` | P1 |
| 6 | Roadmap | `/roadmap` | P1 |
| 7 | Quickstart / Download | `/download` | P0 |
| 8 | About / Founder | `/about` | P0 |
| 9 | Contact | `/contact` | P1 |

Nav: logo → Home; `Product` dropdown (How it works, Heuristics, Visualization, Dogfooding); `Roadmap`; `Download`; `About`; primary CTA button = **Download beta** (disabled/"coming soon" until the beta exists — must never link to a non-existent asset).

## 4. Page content spec

### 4.1 Home `/`
- **Hero:** one-line value prop + sub-line; primary CTA (Download), secondary CTA (See how it works). Suggested headline angle: *"See the bottlenecks your AI wrote."* (final copy TBD).
- **Hero visual:** the `hot-components.svg` and/or `include-graph.svg` rendered from the `examples/` corpus (real tool output — not mockups).
- **Problem strip:** AI-generated code at volume; manual review impossible; static analysis ≠ profiling but is cheap and complete.
- **What it does (3–4 cards):** static C (and planned Rust) analysis · architecture-level visual bottleneck maps · LLM-consumable `metrics.json` · deterministic reports.
- **Proof strip:** "23 heuristics · 82→83 findings on the 13-file demo corpus · byte-deterministic charts".
- **Shipped vs Planned teaser** linking to `/roadmap`.
- **Footer:** license (PolyForm Strict, non-commercial), repo link (when public), contact.

### 4.2 How it works `/how-it-works`
- Pipeline diagram `cli → scanner → analyzer → aggregator → reporter/charts → writer` (from `docs/architecture/modules.md`).
- Tier explainer: syntactic (tree-sitter) → semantic resolver (libclang, C only) → dynamic (planned).
- Confidence/severity model: every finding carries `confidence`, `severity`, `resolver_tier`, `macro_suspect`, `error_node_overlap`; framing = *optimization candidates, never bugs*.
- Determinism: charts/metrics byte-identical for identical inputs; provenance-only exception.
- Deep links to `docs/architecture/ir-v1.md`, `docs/schema/metrics-json-v1.md`, `docs/report/output-contract.md`.

### 4.3 Heuristics catalog `/heuristics`
- Grouped tables by family: **ALLOC · ARCH · CALL · LOOP · MEM · STR · TYPE**.
- Columns per rule: id, title, what it detects, the false-positive guard (silent control), status (Shipped for `PERF-C-*` / Planned for `PERF-RS-*`).
- **Content source:** `docs/heuristics/catalog-v1.md` (C) and the Rust catalog companion produced by the `rust-backend` milestone.
- A short "why these matter for AI-generated C/Rust" intro; the differentiation claim must stay hedged ("not found in surveyed tools", not "unique").

### 4.4 Visual output `/visualization`
- The overview chart set: `include-graph.svg`, `layer-file.svg`, `layer-entry.svg`, `component-coupling.svg`, `hot-components.svg`, `layer-workspace.svg`, `layer-bands.svg`, `layer-findings.svg`, plus per-component detail pages and `graph.dot`.
- The map is the core view: workspace → file → entry-stratified → component bands → findings annotations, with honest caps (`showing top S of M`) and finding dots (severity fill + confidence ring).
- Explain the deterministic hand-rolled SVG emitter (ADR-0002) and the node/component caps.
- Interactive gallery optional (static images are sufficient for v1).

### 4.5 Dogfooding `/dogfooding`
- "Perfpal analyzed itself" — the real report from the `rust-backend`/`beta-release` milestones.
- Show headline stats and a few representative findings; be honest about accepted false-positive classes.
- **Depends on:** the dogfood report artifact from `rust-backend` (R4).

### 4.6 Roadmap `/roadmap`
- Rendered from `docs/roadmap.md`: v0.1.0 shipped → Rust backend (0.2.0) → beta (0.2.0-beta.1) → C++/Python later.
- Consistent `Shipped`/`Planned` badges.

### 4.7 Quickstart / Download `/download`
- CLI install: download the release asset for the platform, verify the `.sha256`, run `perfpal <path>`.
- Exact `perfpal-report/` tree; exit-code table; `--help`/`--version` output.
- Beta download CTA + "beta, non-commercial evaluation" notice + EULA link.
- **Depends on:** the beta release assets (binaries, checksums, SBOM, EULA) from `beta-release`; until then show a "coming soon" state fed by the roadmap.

### 4.8 About `/about`
- Founder story, why Perfpal exists (AI-generated codebases), the product's positioning, license/usage terms, contact for funding/permission inquiries.

### 4.9 Contact `/contact`
- Form (or mailto) + links to repo/Release/socials; a permission-request note (modification/redistribution + commercial use require permission, per PolyForm Strict).

## 5. Content inputs the site pulls from Perfpal

| Asset | Source |
|---|---|
| Value prop + feature list | `README.md` |
| Pipeline / architecture | `docs/architecture/modules.md`, `docs/architecture/ir-v1.md` |
| Heuristic tables | `docs/heuristics/catalog-v1.md` (+ Rust companion) |
| Charts (real) | run the tool over `examples/` (+ `examples-rust/`) and copy the SVGs |
| Sample report | `examples/` run → `perfpal-report/` |
| Roadmap | `docs/roadmap.md` |
| License + beta terms | `LICENSE`, beta `EULA` |
| Dogfood report | `rust-backend` R4 artifact |
| Release assets | `beta-release` GitHub pre-release |

## 6. Technical direction (to be decided in the website repo)

- **Stack:** static site generator or static-export framework (e.g. Astro / Next static export / Vite + a component lib). Decide in the website sessions; requirement = fast, static, deployable to any static host.
- **Constraints:** no backend, no accounts, no tracking beyond optional privacy-respecting analytics; assets self-hosted; charts inlined as SVG; a11y (contrast, focus, reduced-motion) and responsive down to mobile.
- **Branding:** logo/wordmark + palette + typography are a website-repo deliverable (a lightweight brand kit may later be referenced by the product README).
- **Deployment:** any static host (GitHub Pages is a fine default for a showcase; custom domain can be added later).

## 7. Acceptance criteria (for the website milestone)

1. All P0 pages exist and render on mobile + desktop with no layout breakage.
2. Every feature/status claim is correctly badged `Shipped` or `Planned` (no ambiguity), and matches `docs/roadmap.md`.
3. Real artifacts used: real chart SVGs, real sample report, real release links (or a correct "coming soon" state before the beta).
4. Lighthouse/a11y baseline met (performance + accessibility ≥ a chosen floor) and no console errors.
5. Download CTA resolves to a real asset once the beta exists; before that it clearly signals "coming soon".
6. License/EULA/usage terms are visible and correct.

## 8. Handoff to the website session family

- New repo: `~/Perfpal-Website` (own git history, own plans/state, own release cadence).
- **First website session should:** read this document, choose the stack, propose the brand kit + repo scaffold, and create its own plan (planning-first session).
- Keep this doc as the contract; if the product roadmap changes, update `docs/roadmap.md` here and re-point the website sessions at it.

### 8.1 Release asset inventory (input for `/download`)

The `beta-release` milestone publishes **`v0.2.0-beta.1`** as a GitHub
**pre-release** (never marked *Latest*). The release bundle is **19 assets** —
hand this table to the `/download` page:

| Group | Assets | Count |
|---|---|---|
| Binaries | `perfpal-x86_64-unknown-linux-musl`, `perfpal-aarch64-unknown-linux-musl`, `perfpal-x86_64-unknown-linux-gnu`, `perfpal-x86_64-apple-darwin`, `perfpal-aarch64-apple-darwin`, `perfpal-x86_64-pc-windows-msvc.exe` | 6 |
| Per-file checksums | `<each binary>.sha256` | 6 |
| Aggregate integrity | `SHA256SUMS`, `SHA256SUMS.minisig` | 2 |
| Supply chain | `perfpal-0.2.0-beta.1.spdx.json` (SPDX 2.3), `THIRD-PARTY-NOTICES.md` | 2 |
| Legal | `EULA.md`, `PRIVACY.md`, `LICENSE` (PolyForm Strict) | 3 |

Notes the page must reflect:

- **Verify authenticity:** `minisign -Vm SHA256SUMS -p minisign.pub` then
  `sha256sum -c SHA256SUMS` (the public key `minisign.pub` ships in the repo).
  Only the **6 binaries** are covered by `SHA256SUMS`; the other 13 assets are
  the per-file checksums, the `SHA256SUMS.minisig` signature, the SBOM, the
  notices, and the legal documents.
- **Binaries are unsigned** at beta (no Windows Authenticode / macOS
  notarization) — the minisign-signed manifest *is* the authenticity story.
- **Feature split:** the two **musl** binaries are syntactic-only (static musl
  cannot `dlopen` libclang); the `-gnu`, darwin, and windows binaries carry the
  optional semantic (`semantic-clang`) tier with graceful degradation when
  libclang is absent.
- **Corpus smoke test (evidence):** the shipped musl binary scans `examples/`
  (13 C files) → **83 findings** (16 hot / 25 warn / 42 info, 23 rules) and
  `examples-rust/` (14 Rust files) → **41 findings** (17 hot / 11 warn / 13 info,
  23 rules), exit 0, full `perfpal-report/` tree each. The real chart SVGs for
  `/visualization` and `/` are regenerated by running the tool over these
  corpora (`--force`, then copy `charts/*.svg`).
