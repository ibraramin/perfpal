//! In-report Atlas story embedding (ADR-0014, plan `polish-report` H1/2B2).
//!
//! Every `report.html` carries exactly one `#atlas-story` section:
//!
//! 1. the player shell ported from `sim/index.html` (no external `<script src>`,
//!    no load-error panel, and a `#story-stage-host` div in which the player
//!    creates its stage SVG at runtime);
//! 2. a `<noscript>` narrative built from the same payload;
//! 3. one inline `<script type="application/json">` data block whose every
//!    `<` is written as `\u003c`;
//! 4. one fixed `<script id="atlas-story-script">` with [`STORY_SCRIPT`].
//!
//! The standalone sim shell is also used by the v0.5.1 CLI's sim-only
//! `report.html`, which carries exactly two script elements (the inline data
//! and the fixed player — no pan/zoom script). The full library composition
//! additionally carries the unchanged [`crate::reporter::report_script`] block,
//! for three scripts there (ADR-0014 as amended). Both variants are offline:
//! no fetch/HTTP, no eval, `file://`-safe.
//!
//! The section is scoped under `#atlas-story` by `sim/sim.report.css`, which is
//! generated from `sim/sim.css` by `sim/scope-story-css.py`. The stylesheet is
//! a fixed repository constant, never generated per scan.

use std::path::Path;

use serde_json::Value;

use crate::reporter::atlas_story::StoryBundle;
use crate::reporter::escape_html;
use crate::schema::MetricsDocument;

/// The fixed player source (repository constant, not generated per scan).
pub const STORY_SCRIPT: &str = include_str!("../../sim/sim.js");

/// The scoped story stylesheet (generated from `sim/sim.css`).
const STORY_CSS: &str = include_str!("../../sim/sim.report.css");

/// Report-only fluid overrides layered AFTER [`STORY_CSS`] (same `<style>`
/// element — no extra script). `sim/sim.css` / `sim/sim.report.css` are
/// generated sources and stay untouched.
///
/// Type sizes use the same anchored `clamp()` ramp as the map, keyed on
/// `--report-max` (not the viewport): at a 1440px design width each value
/// equals the stylesheet's current px size, and the ceiling is 1.5× at 3072px.
/// A pinned `--report-width` therefore keeps the pinned page's type sizes even
/// on a wider monitor. The story stage's SVG text is untouched — SVG user
/// units already scale with the viewBox.
const STORY_FLUID_CSS: &str = r#"#atlas-story{--story-fs-8:clamp(8px,calc(4.4704px + .0024512*var(--report-max,100vw)),12px);--story-fs-8-2:clamp(8.2px,calc(4.58216px + .00251248*var(--report-max,100vw)),12.3px);--story-fs-8-5:clamp(8.5px,calc(4.7498px + .0026044*var(--report-max,100vw)),12.75px);--story-fs-9:clamp(9px,calc(5.0292px + .0027576*var(--report-max,100vw)),13.5px);--story-fs-9-5:clamp(9.5px,calc(5.3086px + .0029108*var(--report-max,100vw)),14.25px);--story-fs-12:clamp(12px,calc(6.7056px + .0036768*var(--report-max,100vw)),18px);--story-fs-16:clamp(16px,calc(8.9408px + .0049024*var(--report-max,100vw)),24px)}
#atlas-story .page{max-width:min(100%,var(--report-max,100vw));height:auto;min-height:0}
#atlas-story .canvas-shell{max-width:100%;flex:0 0 auto;aspect-ratio:1400/718;height:auto;min-height:0}
#atlas-story .story-head{max-width:min(100%,var(--report-max,100vw))}
#atlas-story .kicker,#atlas-story .story-kicker{font-size:var(--story-fs-8)}
#atlas-story .subline{font-size:var(--story-fs-8-5)}
#atlas-story .story-sub{font-size:var(--story-fs-9)}
#atlas-story .transport-btn,#atlas-story #speed,#atlas-story #beat-label{font-size:var(--story-fs-9-5)}
#atlas-story #case-select{font-size:var(--story-fs-9)}
#atlas-story .key-label{font-size:var(--story-fs-8)}
#atlas-story .key-defs,#atlas-story .legend{font-size:var(--story-fs-8-2)}
#atlas-story .narration-title{font-size:var(--story-fs-12)}
#atlas-story .story-head h2{font-size:var(--story-fs-16)}
#atlas-story details.s4-detail>summary.s4-summary{font-size:var(--story-fs-9)}"#;

/// The inline data block's `id`.
pub const STORY_DATA_TAG: &str = "perfpal-atlas-story-data";

/// The fixed player script's `id`.
pub const STORY_SCRIPT_ID: &str = "atlas-story-script";

/// Render the complete, always-on `#atlas-story` section.
///
/// `doc` is only consulted for a safe narrative fallback when the payload has
/// no entry line; `bundle` supplies the already-built layout/payload pair.
#[must_use]
pub fn render_story_section(doc: &MetricsDocument, bundle: &StoryBundle) -> String {
    debug_assert_story_script_safe();

    let payload: Value = serde_json::from_str(&bundle.payload_json).unwrap_or(Value::Null);
    let narrative = narrative(&payload, doc);
    let inline = inline_story_json(&bundle.payload_json);

    let mut out = String::with_capacity(STORY_CSS.len() + STORY_SCRIPT.len() + inline.len() + 4096);
    out.push_str("<section id=\"atlas-story\" class=\"atlas-story\" data-atlas-block=\"story\" aria-labelledby=\"atlas-story-title\">\n");
    // Visible story heading; the player's duplicate brand is hidden in-report.
    out.push_str("<div class=\"story-head\"><div class=\"story-kicker\">ATLAS STORY</div><h2 id=\"atlas-story-title\">The inferred run, beat by beat</h2><p class=\"story-sub\">Static inference · profile to confirm · entry → stages → loops → I/O → stop</p></div>\n");
    out.push_str("<noscript><style>#atlas-story .page,#atlas-story #s4-detail{display:none!important}</style></noscript>\n");
    out.push_str("<style>\n");
    out.push_str(STORY_CSS);
    out.push_str(STORY_FLUID_CSS);
    out.push_str("</style>\n");
    out.push_str(STORY_SHELL);
    out.push_str(&narrative);
    out.push_str(&format!(
        "<script type=\"application/json\" id=\"{STORY_DATA_TAG}\">{inline}</script>\n"
    ));
    out.push_str(&format!("<script id=\"{STORY_SCRIPT_ID}\">\n"));
    out.push_str(STORY_SCRIPT.trim_end_matches('\n'));
    out.push_str("\n</script>\n</section>\n");
    out
}

/// Build the inline data block body and neutralize every `<`.
///
/// Replacing `<` with `\u003c` (still the same JSON string after `JSON.parse`)
/// makes `</script>` breakout impossible regardless of scanned path content.
fn inline_story_json(payload_json: &str) -> String {
    let body = format!(
        "{{\"index\":{{\"cases\":[{{\"id\":\"scan\",\"label\":\"this scan\",\"file\":\"inline\"}}]}},\"cases\":{{\"scan\":{payload_json}}}}}"
    );
    body.replace('<', "\\u003c")
}

/// `<noscript>` narrative from the payload: entry, stage path, stop state,
/// first action anchor, and the drill links. Every interpolated value is
/// escaped exactly like the rest of the report.
fn narrative(payload: &Value, doc: &MetricsDocument) -> String {
    let entry = payload
        .pointer("/meta/entry")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty() && *text != "—")
        .map(str::to_string)
        .unwrap_or_else(|| fallback_entry(doc));
    let path = payload
        .pointer("/meta/path")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .unwrap_or("not drawn");
    let stages = payload
        .get("trace")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let stage_phrase = format!("{stages} stage{}", if stages == 1 { "" } else { "s" });
    let stop = stop_state(payload.get("state").unwrap_or(&Value::Null));
    let (loops, io) = summary_counts(payload);
    let loop_phrase = format!(
        "{loops} loop-bearing function{}",
        if loops == 1 { "" } else { "s" }
    );
    let io_phrase = format!("{io} I/O site{}", if io == 1 { "" } else { "s" });
    let action = payload
        .pointer("/meta/actions/0/anchor")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .unwrap_or("no ranked action on the drawn slice");

    format!(
        "<noscript>\n<p class=\"atlas-story-noscript\">The interactive run story needs JavaScript. Inferred run: entry <strong>{entry}</strong>, {stage_phrase} along <code>{path}</code>; {loop_phrase}; {io_phrase}; stop reason: {stop}; first action: <code>{action}</code>. Continue in <a href=\"atlas-weight.html\">the weight drill</a>, <a href=\"atlas-q2.html\">the Q2 reverse drill</a>, <a href=\"findings-detail.html#ranked-actions\">ranked actions</a>, or <a href=\"profile-queue.html\">the profile queue</a>.</p>\n</noscript>\n",
        entry = escape_html(&entry),
        stage_phrase = escape_html(&stage_phrase),
        path = escape_html(path),
        loop_phrase = escape_html(&loop_phrase),
        io_phrase = escape_html(&io_phrase),
        stop = escape_html(&stop),
        action = escape_html(action),
    )
}

/// Counts used by the JS-off narrative: every node function with a loop
/// (`maxLoopDepth > 0`) or an unknown trip count, and the emitted I/O rows.
/// The player's `trust.io.rows` is the canonical count (there is no `total`).
fn summary_counts(payload: &Value) -> (u64, u64) {
    let io = payload
        .pointer("/trust/io/rows")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let loops = payload
        .pointer("/trust/loopBearingFunctions")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    (loops, io)
}

/// Readable stop reason for the payload's state flags.
///
/// Mirrors the player's `stopText`: the path/state reason comes first and the
/// parse/test/file caveats are appended as notes. `partialParse` is not a
/// primary stop reason (the cJSON fixture is partial but still ends at a leaf).
fn stop_state(state: &Value) -> String {
    let on = |key: &str| state.get(key).and_then(Value::as_bool) == Some(true);
    let primary = if on("empty") {
        "empty scan (no source files)"
    } else if on("noEntry") {
        "no production entry"
    } else if on("library") {
        "no forward path: the elected entry is a library root"
    } else if on("capped") {
        "capped at the render budget"
    } else {
        "static leaf (the representative path ends at a static leaf)"
    };
    let mut parts = vec![primary.to_string()];
    if on("partialParse") {
        parts.push("partial parse".to_string());
    }
    if on("allTest") {
        parts.push("all-test scan".to_string());
    }
    if on("singleFile") {
        parts.push("single-file scan".to_string());
    }
    parts.join("; ")
}

/// Safe entry fallback when the payload carries none (e.g. an empty scan).
fn fallback_entry(doc: &MetricsDocument) -> String {
    Path::new(&doc.scan.root_path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("not determined")
        .to_string()
}

/// The player shell ported from `sim/index.html:11-137` with the ADR-0014
/// changes: no load-error panel, no static stage `<svg>`, and a
/// `#story-stage-host` container the fixed script fills at runtime.
const STORY_SHELL: &str = r##"<div class="page">
  <header class="masthead">
    <div class="brand">
      <div class="kicker" id="brand-kicker">PERFPAL · ATLAS</div>
      <div class="wordmark" id="brand-wordmark">Atlas</div>
      <div class="subline" id="brand-subline">entry-flow map · static, profile to confirm</div>
    </div>

    <div class="transport" role="group" aria-label="Run story transport">
      <button id="play-pause" class="transport-btn play" type="button" aria-label="Play story">▶ <span>Play</span></button>
      <button id="step-back" class="transport-btn" type="button" aria-label="Previous beat" title="Previous beat (←)">◀ step</button>
      <button id="step-forward" class="transport-btn" type="button" aria-label="Next beat" title="Next beat (→)">step ▶</button>
      <input id="scrub" type="range" min="0" max="1" step="10" value="0" aria-label="Scrub the static run story">
      <output id="beat-label" aria-live="off">beat 1/1</output>
      <select id="speed" aria-label="Playback speed">
        <option value="0.5">0.5×</option>
        <option value="1" selected>1×</option>
        <option value="2">2×</option>
        <option value="4">4×</option>
      </select>
      <button id="reset" class="transport-btn" type="button" aria-label="Reset story to the beginning" title="Reset (Home)">Reset</button>
    </div>
  </header>

  <div class="honesty" id="honesty" role="note">
    <span class="honesty-label">STATIC, INFERRED · not a runtime trace · profile to confirm</span>
    <label class="case-wrap" for="case-select">case
      <select id="case-select" aria-label="Choose a case"></select>
    </label>
    <span id="synthetic-chip" class="chip chip-synth" hidden>synthetic state demo</span>
  </div>

  <section class="key-strip" aria-label="Atlas key facts">
    <div class="key-row key-top">
      <div class="key key-entry">
        <span class="key-label">ENTRY</span>
        <span id="key-entry" class="key-value mono">—</span>
      </div>
      <div class="key key-hottest">
        <span class="key-label">HOTTEST</span>
        <span id="key-hottest" class="key-value mono"><span id="key-hottest-path"></span><span id="key-hottest-meta"></span></span>
      </div>
      <div class="key key-path">
        <span class="key-label">PATH</span>
        <span id="key-path" class="key-value mono">0 steps</span>
      </div>
      <div class="key key-caps-key">
        <span class="key-label">CAPS</span>
        <span id="key-caps" class="key-caps"></span>
      </div>
    </div>
    <div class="key-row key-mid">
      <div class="starts">
        <span class="key-label">START</span>
        <span id="start-actions" class="start-actions"></span>
      </div>
      <div class="legend" id="legend" aria-label="Map legend">
        <span class="legend-item"><span class="sw sw-prod" aria-hidden="true"></span>production</span>
        <span class="legend-item"><span class="sw sw-nonprod" aria-hidden="true"></span>test/bench</span>
        <span class="legend-item"><span class="sw sw-unreached" aria-hidden="true"></span>unreached</span>
        <span class="legend-item"><span class="sw sw-forward" aria-hidden="true"></span>forward</span>
        <span class="legend-item"><span class="sw sw-back" aria-hidden="true"></span>back</span>
        <span class="legend-item"><span class="sw sw-path" aria-hidden="true"></span>path</span>
        <span class="legend-item legend-labels" title="edge label classes">
          <span class="legend-sub">label</span>
          <span class="lg lg-type">type-only</span>
          <span class="lg lg-literal">literal-observed</span>
          <span class="lg lg-unresolved">unresolved</span>
        </span>
      </div>
    </div>
    <div class="key-row key-bottom">
      <div class="key-defs">
        <span class="definition" id="median-def">ratio × = file cost ÷ repo-wide median function cost</span>
        <span class="definition" id="badge-def">badge ×N = dominant rule annotation, not a file total</span>
        <span class="caveat" id="structural-caveat">Structural inference only — no edge, cost or trip is measured; confirm with a profiler.</span>
      </div>
    </div>
  </section>

  <div class="canvas-shell" id="canvas-shell">
    <div id="story-stage-host"></div>

    <div id="stub" class="stub" hidden>
      <div class="stub-card">
        <div class="stub-kicker">SYNTHETIC STATE DEMO</div>
        <h2 id="stub-title"></h2>
        <p id="stub-body"></p>
        <p class="stub-note">No nodes, edges, costs, or findings are fabricated for this state.</p>
      </div>
    </div>

    <aside id="inspector" class="inspector" hidden aria-label="File inspector">
      <div class="inspector-head">
        <div class="inspector-kicker">FILE INSPECTOR</div>
        <button id="inspector-close" class="inspector-close" type="button" aria-label="Close file inspector">×</button>
      </div>
      <div id="inspector-body"></div>
    </aside>
  </div>

  <section class="narration" aria-live="polite" aria-atomic="true">
    <div id="narration-title" class="narration-title">Boot</div>
    <div id="narration-body" class="narration-body">Loading the static inferred run story…</div>
    <div id="narration-chips" class="narration-chips"></div>
  </section>

  <div class="canvas-footer" id="canvas-footer"></div>
</div>

<details id="s4-detail" class="s4-detail" aria-label="Whole-repo detail" hidden>
  <summary class="s4-summary">Whole-repo detail — scenes, profile queue and trust accounting (open to expand)</summary>
  <div class="s4-panel" id="s4-scenes-panel">
    <h2>SCENES</h2>
    <p class="s4-caps" id="s4-scenes-caps"></p>
    <div class="s4-scene-grid" id="s4-scene-grid"></div>
  </div>
  <div class="s4-panel" id="s4-profile-panel">
    <h2>PROFILE QUEUE</h2>
    <p class="s4-caps" id="s4-profile-caps"></p>
    <ol class="s4-queue" id="s4-queue"></ol>
  </div>
  <div class="s4-panel" id="s4-trust-panel">
    <h2>TRUST ACCOUNTING</h2>
    <dl class="s4-trust" id="s4-trust-list"></dl>
    <p class="s4-caveat">Static inference only — costs, scenes and call edges are candidates, not a runtime measurement. Validate with a profiler.</p>
  </div>
</details>
"##;

/// Hard offline/safety locks for the fixed script. Called by the renderer in
/// debug builds and asserted by the unit test below.
fn debug_assert_story_script_safe() {
    debug_assert!(
        !STORY_SCRIPT.contains("</script"),
        "story script closes its tag"
    );
    debug_assert!(!STORY_SCRIPT.contains("fetch("), "story script fetches");
    debug_assert!(
        !STORY_SCRIPT.contains("http://"),
        "story script has an HTTP URL"
    );
    debug_assert!(
        !STORY_SCRIPT.contains("https://"),
        "story script has an HTTPS URL"
    );
    debug_assert!(!STORY_SCRIPT.contains("eval("), "story script evals");
    debug_assert!(!STORY_CSS.contains("</style"), "story CSS closes its tag");
    debug_assert!(
        !STORY_FLUID_CSS.contains("</style"),
        "story override CSS closes its tag"
    );
    debug_assert!(!STORY_CSS.contains("http://"), "story CSS has an HTTP URL");
    debug_assert!(
        !STORY_CSS.contains("https://"),
        "story CSS has an HTTPS URL"
    );
    debug_assert!(!STORY_CSS.contains("@import"), "story CSS imports");
    debug_assert!(
        !STORY_SCRIPT.contains("Function("),
        "story script builds functions"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn story_script_is_offline_and_non_empty() {
        assert!(!STORY_SCRIPT.is_empty());
        for banned in [
            "</script",
            "fetch(",
            "http://",
            "https://",
            "eval(",
            "Function(",
        ] {
            assert!(
                !STORY_SCRIPT.contains(banned),
                "banned string {banned:?} in the fixed story script"
            );
        }
        debug_assert_story_script_safe();
    }

    #[test]
    fn summary_counts_reads_io_rows_and_all_loop_bearing_functions() {
        let payload = serde_json::json!({
            "trust": {"io": {"rows": 4}, "loopBearingFunctions": 2}
        });
        assert_eq!(summary_counts(&payload), (2, 4));
    }

    #[test]
    fn stop_state_names_the_reason() {
        assert_eq!(
            stop_state(&serde_json::json!({})),
            "static leaf (the representative path ends at a static leaf)"
        );
        assert_eq!(
            stop_state(&serde_json::json!({"capped": true})),
            "capped at the render budget"
        );
        assert_eq!(
            stop_state(&serde_json::json!({"library": true})),
            "no forward path: the elected entry is a library root"
        );
        assert_eq!(
            stop_state(&serde_json::json!({"partialParse": true})),
            "static leaf (the representative path ends at a static leaf); partial parse"
        );
        assert_eq!(
            stop_state(&serde_json::json!({"allTest": true})),
            "static leaf (the representative path ends at a static leaf); all-test scan"
        );
        assert_eq!(
            stop_state(&serde_json::json!({"singleFile": true})),
            "static leaf (the representative path ends at a static leaf); single-file scan"
        );
    }

    #[test]
    fn story_css_is_scoped_and_offline() {
        assert!(STORY_CSS.contains("#atlas-story"));
        for banned in ["</style", "http://", "https://", "@import"] {
            assert!(
                !STORY_CSS.contains(banned),
                "banned string {banned:?} in the scoped story CSS"
            );
            assert!(
                !STORY_FLUID_CSS.contains(banned),
                "banned string {banned:?} in the fluid override CSS"
            );
        }
        // The report-only override keeps the story page and canvas fluid and
        // leaves the generated stylesheet alone.
        assert!(STORY_FLUID_CSS.contains("#atlas-story .page{max-width:min(100%,var(--report-max"));
        assert!(STORY_FLUID_CSS.contains("#atlas-story .canvas-shell{max-width:100%"));
        assert!(!STORY_FLUID_CSS.contains("<script"));
    }

    #[test]
    fn inline_data_never_contains_a_raw_less_than() {
        let hostile = "{\"path\":\"</script><img src=x onerror=alert(1)>.c\"}";
        let body = inline_story_json(hostile);
        assert!(!body.contains('<'), "raw `<` survived: {body}");
        assert!(!body.contains("</script"), "script breakout survived");
        assert_eq!(
            serde_json::from_str::<Value>(&body).expect("inline JSON parses")["cases"]["scan"]["path"],
            "</script><img src=x onerror=alert(1)>.c"
        );
    }
}
