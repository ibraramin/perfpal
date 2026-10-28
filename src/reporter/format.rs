//! Shared formatting primitives for the reporter.
//!
//! One home for the Markdown/HTML escapers (agent-report.md + report.md
//! contract), the warm-paper palette, the shared page shell, and the
//! key-view CSS shared by every report-family page.

// ---------------------------------------------------------------------------
// Markdown escaping
// ---------------------------------------------------------------------------

/// Pinned Markdown metacharacter escape set (output-contract §2 paths rule).
/// Every char here is backslash-escaped wherever document-derived text is
/// embedded. `|` inclusion makes table cells safe without a second pass.
///
/// `pub` so the integration-test harness imports this single source of truth
/// instead of hand-mirroring it.
pub const MD_ESCAPE_CHARS: &[char] = &[
    '\\', '`', '*', '_', '{', '}', '[', ']', '(', ')', '#', '+', '-', '.', '!', '|', '<', '>', '&',
];

/// Escape Markdown metacharacters with the PINNED set [`MD_ESCAPE_CHARS`],
/// and flatten control characters (`\r`/`\n`/`\t` → two-character escapes)
/// so an embedded path/title/message can never break out of its line or inject
/// a heading. Applied to every embedded path, title, message, id, symbol, and
/// caption in `report.md` / `profile-queue.md`.
#[must_use]
pub fn escape_md(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => {
                if MD_ESCAPE_CHARS.contains(&c) {
                    out.push('\\');
                }
                out.push(c);
            }
        }
    }
    out
}

/// Single-line, structure-safe rendering: CR/LF/TAB become two-character
/// escapes and every backtick becomes U+02CB (a backslash does not escape a
/// backtick inside a CommonMark code span). Lossy by design; JSON keeps the
/// original bytes.
pub(crate) fn md_single_line(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '`' => out.push('\u{02cb}'),
            _ => out.push(ch),
        }
    }
    out
}

/// Code-span value (path, id, name, evidence): single-line. The HTML tags are
/// inert inside a code span, but `<script` sequences are still neutralized so
/// a hostile embedded value can never contribute a raw script token to the
/// Markdown artifact.
pub(crate) fn md_code(text: &str) -> String {
    md_single_line(text)
        .replace("<script", "&lt;script")
        .replace("</script", "&lt;/script")
}

/// Prose value (`title`, `why`): single-line plus HTML neutralization.
pub(crate) fn md_text(text: &str) -> String {
    md_single_line(text)
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Table cell: prose rendering plus column-separator escaping.
pub(crate) fn md_cell(text: &str) -> String {
    // Backslash FIRST: otherwise an input `\|` becomes `\\|`, leaving the
    // pipe live as a table-column separator.
    md_text(text).replace('\\', "\\\\").replace('|', "\\|")
}

// ---------------------------------------------------------------------------
// HTML escaping
// ---------------------------------------------------------------------------

/// Escape HTML special characters (`& < > " '`) for safe embedding in
/// element text and double-quoted attribute values (output-contract §2
/// paths rule, HTML counterpart of [`escape_md`]).
///
/// Single pass over chars — `&` is escaped first by construction (match arms
/// emit `&amp;` literally, never re-scanned), so `&` cannot be re-escaped.
/// Every interpolated path/title/message/id/symbol in the HTML renderer goes
/// through this function; chart filenames and captions are charset-pinned
/// static text (output-contract §3 kebab-case) and bypass it by construction.
#[must_use]
pub fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Warm-paper palette (single source for every report-family page)
// ---------------------------------------------------------------------------

/// Page paper background.
pub(crate) const PAPER: &str = "#f4f1ea";
/// Primary ink.
pub(crate) const INK: &str = "#202b35";
/// Muted secondary text.
pub(crate) const MUTED: &str = "#6f6a61";
/// Hairline rule / border.
pub(crate) const RULE: &str = "#d6d0c4";
/// Card / figure background.
pub(crate) const CARD: &str = "#fffdf8";
/// Accent (links).
pub(crate) const ACCENT: &str = "#7c2d12";
/// Table header / chrome background.
pub(crate) const HEAD: &str = "#ece8df";
/// Zebra row background.
pub(crate) const ZEBRA: &str = "#fbf9f4";

/// Shared palette custom properties. Every page consumes these tokens; the
/// literal values are declared exactly once (the constants above), so the
/// report-family shell and the Atlas drill pages can never drift apart.
fn tokens_css() -> String {
    format!(
        ":root{{--paper:{paper};--ink:{ink};--muted:{muted};--rule:{rule};--card:{card};--accent:{accent};--head:{head};--zebra:{zebra}}}\n",
        paper = PAPER,
        ink = INK,
        muted = MUTED,
        rule = RULE,
        card = CARD,
        accent = ACCENT,
        head = HEAD,
        zebra = ZEBRA,
    )
}

/// Shared base CSS: warm paper body, one heading scale, accent links,
/// tabular tables, card figures, flex nav.
const BASE_CSS: &str = "\
*{box-sizing:border-box}\n\
body{background:var(--paper);color:var(--ink);font-family:ui-sans-serif,system-ui,-apple-system,\"Segoe UI\",Roboto,Helvetica,Arial,sans-serif;font-size:13px;line-height:1.5;margin:0}\n\
h1,h2,h3,h4{margin:1rem 0 .5rem;line-height:1.25}\n\
h1{font-size:1.6rem;font-weight:800}\n\
h2{font-size:1.15rem;font-weight:800;padding-top:.75rem;border-top:1px solid var(--rule)}\n\
h3{font-size:1rem;font-weight:800}\n\
h4{font-size:.85rem;font-weight:800;text-transform:uppercase;color:var(--muted);letter-spacing:.04em}\n\
a{color:var(--accent);text-decoration:none}\n\
a:hover{text-decoration:underline}\n\
code,pre,kbd,samp,.mono{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace}\n\
pre{background:var(--head);border:1px solid var(--rule);padding:.5rem;overflow-x:auto}\n\
.table-scroll{overflow-x:auto;max-width:100%}\n\
table{border-collapse:collapse;width:100%;margin:1rem 0;font-variant-numeric:tabular-nums}\n\
th,td{border:1px solid var(--rule);padding:.25rem .5rem;text-align:left;vertical-align:top}\n\
th{background:var(--head)}\n\
tbody tr:nth-child(even){background:var(--zebra)}\n\
figure{background:var(--card);border:1px solid var(--rule);padding:.5rem;margin:1rem 0;max-width:100%;overflow-x:auto}\n\
figcaption{margin-top:.5rem;color:var(--muted)}\n\
img{max-width:100%;height:auto}\n\
nav ul{list-style:none;padding:0;margin:.5rem 0;display:flex;gap:1rem;flex-wrap:wrap}\n\
.sr-only{position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;border:0}\n";

/// Scoped key-view CSS (map-intelligibility V4). Shared by the report page
/// and the detail/profile pages. No `text-overflow: ellipsis` anywhere on
/// key-view text: labels wrap instead of truncating.
pub(crate) const KEY_VIEW_CSS: &str = "\
         .kv{margin:1.25rem 0 1.75rem}\n\
         .kv h3{margin:.25rem 0 .35rem;font-size:1.05rem}\n\
         .kv-action{margin:.15rem 0 .75rem;color:#444}\n\
         .kv-rows{list-style:none;margin:0;padding:0;border-bottom:1px solid #999}\n\
         .kv-row{display:grid;grid-template-columns:3.5rem 1fr;gap:.75rem;padding:.55rem 0;border-top:1px solid #ccc}\n\
         .kv-rank{font-size:1.5rem;font-weight:700;line-height:1}\n\
         .kv-line1{display:flex;flex-wrap:wrap;align-items:baseline;gap:.4rem}\n\
         .kv-loc,.kv-fn{font-family:monospace,monospace;overflow-wrap:anywhere;word-break:break-word;white-space:normal}\n\
         .kv-loc{font-weight:700;color:#111;text-decoration:none}\n\
         .kv-loc:hover{text-decoration:underline}\n\
         .kv-title{font-weight:600}\n\
         .kv-key{margin:.1rem 0 .6rem;color:#333;font-size:.85rem}\n\
         .kv-note{margin-left:.4rem;font-size:.78rem;color:#4a5568}\n\
         .kv-caption{margin:.1rem 0 .6rem;color:#333;font-size:.85rem}\n\
         .kv-badge,.kv-class,.kv-dem{display:inline-block;font-size:.72rem;font-weight:700;text-transform:uppercase;letter-spacing:.03em;padding:0 .3rem;border:1px solid #999;border-radius:.2rem;white-space:normal;overflow-wrap:anywhere}\n\
         .kv-class.nonprod{background:#f3e8ff;border-color:#9f7aea}\n\
         .sev-hot{background:#fff5f5;border-color:#e53e3e;color:#9b2c2c}\n\
         .sev-warn{background:#fffaf0;border-color:#dd6b20;color:#975a16}\n\
         .sev-info{background:#f7fafc;border-color:#a0aec0;color:#2d3748}\n\
         .kv-dem{background:#edf2f7;border-color:#a0aec0;color:#2d3748;margin-left:.35rem}\n\
         .kv-anchor,.kv-value{font-family:monospace,monospace;font-size:.8rem;color:#333;overflow-wrap:anywhere;white-space:normal;line-height:1.35}\n\
         .kv-why{margin-top:.2rem;overflow-wrap:anywhere;white-space:normal}\n\
         .kv-counts{margin:.45rem 0 0;color:#444;font-size:.85rem}\n\
         .kv-more{margin:.5rem 0 .2rem;font-weight:600}\n\
         .kv-bars{max-width:100%}\n\
         .kv-brow{display:grid;grid-template-columns:minmax(14rem,26rem) minmax(4rem,1fr) minmax(12rem,24rem);gap:.7rem;align-items:center;padding:.35rem 0;border-top:1px solid #ccc}\n\
         .kv-brow:last-child{border-bottom:1px solid #999}\n\
         .kv-blabel{overflow-wrap:anywhere;word-break:break-word;white-space:normal;min-width:0}\n\
         .kv-track{background:#edf2f7;border-radius:.15rem;height:1.1rem;width:100%;max-width:100%}\n\
         .kv-fill{background:#2b6cb0;height:1.1rem;border-radius:.15rem;min-width:0;max-width:100%}\n\
         .kv-fill.demoted{background:#a0aec0}\n\
         .kv-demoted .kv-blabel,.kv-demoted .kv-value{opacity:.6}\n\
         .kv-badge.trip{background:#fffaf0;border-color:#dd6b20;color:#975a16;margin-top:.2rem}\n\
         .kv-badge.known{background:#f0fff4;border-color:#38a169;color:#22543d;margin-top:.2rem}\n\
         @media print{.kv-row,.kv-brow{break-inside:avoid}}\n";

// ---------------------------------------------------------------------------
// Shared page shell
// ---------------------------------------------------------------------------

/// Emit the shared report-family document head: doctype, charset, viewport,
/// empty favicon, title, and one inline `<style>` block composed of the
/// shared tokens + base CSS, the caller's page CSS, and [`KEY_VIEW_CSS`].
///
/// `title` is escaped here; `body_attrs` is spliced verbatim into
/// `<body{attrs}>` (callers pass only static attributes).
pub(crate) fn html_page_head(out: &mut String, title: &str, page_css: &str, body_attrs: &str) {
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    // Inline empty data-URI favicon: kills the /favicon.ico 404 console error
    // with zero bytes fetched (no network, fully offline).
    out.push_str("<link rel=\"icon\" href=\"data:,\">\n");
    out.push_str(&format!("<title>{}</title>\n", escape_html(title)));
    out.push_str("<style>\n");
    out.push_str(&tokens_css());
    out.push_str(BASE_CSS);
    out.push_str(page_css);
    out.push_str(KEY_VIEW_CSS);
    out.push_str(&format!("</style>\n</head>\n<body{}>\n", body_attrs));
}
