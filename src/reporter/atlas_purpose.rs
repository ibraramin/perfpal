//! Story-only purpose probe (P6, `docs/plans/atlas-s4.md`).
//!
//! Reads the scan-root README on demand and extracts a single plain-language
//! purpose sentence. Only the first README in [`README_CANDIDATES`] order at
//! the scan root is considered; a scan whose root is a file (or whose README
//! has no usable prose paragraph) falls back to the Atlas entry line. The
//! probe is called only from [`crate::reporter::atlas_layout_json`], so normal
//! scans never touch the filesystem here and stay byte-identical.
//!
//! Honesty rules: text is copied from the README or built from the existing
//! entry helper — never invented. The story's F4 forbidden-claim filter is
//! deliberately NOT applied here; presentation honesty belongs to the consumer.
//!
//! Bounds: at most [`MAX_README_BYTES`] bytes are read, lossy UTF-8, and the
//! emitted sentence is capped at [`MAX_PURPOSE_CHARS`] on a word boundary.
//! Every failure path (missing file, unreadable file, no prose) degrades to the
//! structural fallback instead of failing the scan.

use std::fs;
use std::io::Read;
use std::path::Path;

use serde::Serialize;

/// `purpose_version` — bumped only when the shape of the purpose block changes.
pub(crate) const PURPOSE_VERSION: u8 = 1;

/// Hard cap on bytes read from the root README (64 KiB).
const MAX_README_BYTES: u64 = 65_536;

/// Hard cap on the emitted purpose sentence.
const MAX_PURPOSE_CHARS: usize = 200;

/// README discovery order at the scan root (first match wins).
const README_CANDIDATES: [(&str, ReadmeKind); 4] = [
    ("README.md", ReadmeKind::Markdown),
    ("README.rst", ReadmeKind::Plain),
    ("README.txt", ReadmeKind::Plain),
    ("README", ReadmeKind::Plain),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadmeKind {
    Markdown,
    Plain,
}

/// The story-only `purpose` block as emitted into `atlas-layout.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct Purpose {
    pub(crate) purpose_version: u8,
    pub(crate) text: String,
    /// The README filename when the text came from one; `null` for the
    /// structural fallback.
    pub(crate) source: Option<String>,
    /// `"readme"` or `"structural"`.
    pub(crate) kind: &'static str,
}

/// Read the scan-root README and build the purpose block.
///
/// `entry_text` is the already-formatted Atlas entry line
/// ([`crate::reporter::atlas_render::strip_entry_line`]), used verbatim in the
/// structural fallback so this module never reformats the entry.
#[must_use]
pub(crate) fn read(root: &Path, entry_text: &str) -> Purpose {
    if root.is_dir() {
        for (name, kind) in README_CANDIDATES {
            let path = root.join(name);
            if !path.is_file() {
                continue;
            }
            let text = read_capped(&path).and_then(|raw| match kind {
                ReadmeKind::Markdown => extract_markdown(&raw),
                ReadmeKind::Plain => extract_plain(&raw),
            });
            return match text {
                Some(text) => Purpose {
                    purpose_version: PURPOSE_VERSION,
                    text,
                    source: Some(name.to_string()),
                    kind: "readme",
                },
                None => structural(entry_text),
            };
        }
    }
    structural(entry_text)
}

fn structural(entry_text: &str) -> Purpose {
    Purpose {
        purpose_version: PURPOSE_VERSION,
        text: cap_text(&format!(
            "No README at the scan root; inferred entry: {entry_text}."
        )),
        source: None,
        kind: "structural",
    }
}

/// Read at most [`MAX_README_BYTES`] from `path`, decoded lossily.
fn read_capped(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    let mut handle = file.take(MAX_README_BYTES);
    handle.read_to_end(&mut bytes).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

// ---------------------------------------------------------------------------
// Markdown
// ---------------------------------------------------------------------------

/// Extract the first prose paragraph from Markdown, after skipping front
/// matter, headings, badges/images, TOC/link-only blocks, HTML, fences and
/// tables.
fn extract_markdown(raw: &str) -> Option<String> {
    let body = strip_front_matter(raw);
    let body = strip_html_comments(body);
    let mut lines = body.lines().peekable();
    let mut paragraph = String::new();
    let mut in_fence = false;

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        if in_fence {
            if is_fence(trimmed) {
                in_fence = false;
            }
            continue;
        }
        if is_fence(trimmed) {
            in_fence = true;
            continue;
        }

        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }

        // Setext heading: the following underline belongs to this line.
        if lines
            .peek()
            .is_some_and(|next| is_setext_underline(next.trim()))
        {
            if !paragraph.is_empty() {
                break;
            }
            lines.next();
            continue;
        }

        if is_md_heading(trimmed)
            || is_setext_underline(trimmed)
            || is_horizontal_rule(trimmed)
            || is_md_skippable(trimmed)
        {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }

        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(trimmed);
    }

    finish_paragraph(&paragraph)
}

fn strip_front_matter(input: &str) -> &str {
    let raw = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut lines = raw.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return raw;
    };
    if first.trim() != "---" {
        return raw;
    }
    let mut offset = first.len();
    for line in lines {
        offset += line.len();
        let trimmed = line.trim();
        if trimmed == "---" || trimmed == "..." {
            return &raw[offset..];
        }
    }
    // Unterminated front matter is treated as ordinary content: the leading
    // `---` is skipped later as a horizontal rule.
    raw
}

fn strip_html_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

fn is_fence(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}

fn is_md_heading(line: &str) -> bool {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    (1..=6).contains(&hashes) && line.get(hashes..).is_some_and(|rest| rest.starts_with(' '))
}

fn is_setext_underline(line: &str) -> bool {
    let mut chars = line.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    matches!(first, '=' | '-') && chars.all(|c| c == first)
}

fn is_horizontal_rule(line: &str) -> bool {
    let compact: String = line.chars().filter(|c| *c != ' ').collect();
    let mut chars = compact.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    compact.len() >= 3 && matches!(first, '*' | '-' | '_') && chars.all(|c| c == first)
}

fn is_md_skippable(line: &str) -> bool {
    if line.starts_with("![") || line.starts_with("[![") {
        return true;
    }
    if line.starts_with('<') || line.starts_with('>') || line.starts_with('|') {
        return true;
    }
    if is_bullet_link(line) || is_numbered_link(line) {
        return true;
    }
    // Link reference definitions (`[id]: https://…`).
    line.starts_with('[') && line.contains("]:")
}

fn is_bullet_link(line: &str) -> bool {
    ["* ", "- ", "+ "].into_iter().any(|marker| {
        line.strip_prefix(marker)
            .is_some_and(|rest| rest.trim_start().starts_with('['))
    })
}

fn is_numbered_link(line: &str) -> bool {
    let rest = line.trim_start();
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return false;
    }
    let after = &rest[digits..];
    let Some(after) = after.strip_prefix('.').or_else(|| after.strip_prefix(')')) else {
        return false;
    };
    after.trim_start().starts_with('[')
}

// ---------------------------------------------------------------------------
// RST / plain text
// ---------------------------------------------------------------------------

/// Extract the first prose paragraph from RST/plain text, skipping title
/// underlines, directives (`..`) and list blocks.
fn extract_plain(raw: &str) -> Option<String> {
    let mut lines = raw.lines().peekable();
    let mut paragraph = String::new();

    while let Some(line) = lines.next() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with("..") {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if is_rst_underline(trimmed) {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        // RST title: the following adornment underlines this line.
        if lines
            .peek()
            .is_some_and(|next| is_rst_underline(next.trim()))
        {
            if !paragraph.is_empty() {
                break;
            }
            lines.next();
            continue;
        }
        if is_plain_skippable(trimmed) {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }

        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(trimmed);
    }

    finish_paragraph(&paragraph)
}

fn is_rst_underline(line: &str) -> bool {
    let mut chars = line.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    matches!(
        first,
        '=' | '-' | '~' | '^' | '"' | '\'' | '*' | '#' | '+' | '_' | '.' | ':' | '<' | '>'
    ) && line.chars().count() >= 3
        && chars.all(|c| c == first)
}

fn is_plain_skippable(line: &str) -> bool {
    if line.starts_with('<') || line.starts_with('>') {
        return true;
    }
    if is_bullet_link(line) || is_numbered_link(line) {
        return true;
    }
    if ["* ", "- ", "+ ", "• ", "#."]
        .into_iter()
        .any(|marker| line.starts_with(marker))
    {
        return true;
    }
    // RST field lists / directives-like rows.
    line.starts_with(':')
}

// ---------------------------------------------------------------------------
// Shared cleanup
// ---------------------------------------------------------------------------

fn finish_paragraph(raw: &str) -> Option<String> {
    let cleaned = clean_inline(raw);
    let cleaned = cleaned.trim();
    if cleaned.is_empty() || !cleaned.chars().any(char::is_alphanumeric) {
        return None;
    }
    Some(cap_text(cleaned))
}

fn clean_inline(input: &str) -> String {
    let text = strip_markdown_images(input);
    let text = strip_markdown_links(&text);
    // Code spans run BEFORE HTML-tag stripping: an RST inline link carries its
    // target in angle brackets, which must be consumed as a link label rather
    // than mistaken for a tag.
    let text = strip_code_spans(&text);
    let text = strip_html_tags(&text);
    let text = strip_emphasis(&text);
    collapse_whitespace(&text)
}

fn strip_markdown_images(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    loop {
        let Some(open) = rest.find("![") else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let Some(close) = after.find(']') else {
            out.push_str(&rest[open..]);
            break;
        };
        let tail = &after[close + 1..];
        if let Some(inner) = tail.strip_prefix('(')
            && let Some(end) = inner.find(')')
        {
            rest = &inner[end + 1..];
            continue;
        }
        // Not an image after all: keep the marker and continue after `]`.
        out.push_str("![");
        out.push_str(&after[..close]);
        out.push(']');
        rest = tail;
    }
    out
}

fn strip_markdown_links(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    loop {
        let Some(open) = rest.find('[') else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find(']') else {
            out.push_str(&rest[open..]);
            break;
        };
        let text = &after[..close];
        let tail = &after[close + 1..];
        if let Some(inner) = tail.strip_prefix('(')
            && let Some(end) = inner.find(')')
        {
            out.push_str(text);
            rest = &inner[end + 1..];
            continue;
        }
        if let Some(inner) = tail.strip_prefix('[')
            && let Some(end) = inner.find(']')
        {
            out.push_str(text);
            rest = &inner[end + 1..];
            continue;
        }
        out.push('[');
        out.push_str(text);
        out.push(']');
        rest = tail;
    }
    out
}

fn strip_html_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        match rest[open + 1..].find('>') {
            Some(end) => rest = &rest[open + 1 + end + 1..],
            None => {
                out.push_str(&rest[open..]);
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

/// Strip Markdown code ticks and RST inline links to their label.
fn strip_code_spans(input: &str) -> String {
    let input = input.replace("``", "");
    let mut out = String::with_capacity(input.len());
    let mut rest = input.as_str();
    loop {
        let Some(open) = rest.find('`') else {
            out.push_str(rest);
            break;
        };
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('`') else {
            out.push_str(&rest[open..]);
            break;
        };
        let mut tail = &after[close + 1..];
        if let Some(stripped) = tail.strip_prefix('_') {
            tail = stripped;
        }
        let text = &after[..close];
        if let Some(label) = rst_link_label(text) {
            out.push_str(label);
        } else {
            out.push_str(text);
        }
        rest = tail;
    }
    out
}

/// ``label <https://…>`` ⇒ `label`; every other span is kept verbatim.
fn rst_link_label(text: &str) -> Option<&str> {
    if !text.ends_with('>') {
        return None;
    }
    let start = text.rfind(" <")?;
    Some(text[..start].trim_end())
}

fn strip_emphasis(input: &str) -> String {
    let input = input.replace("~~", "");
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    for (index, c) in chars.iter().enumerate() {
        match c {
            '*' => {}
            '_' => {
                let prev = index.checked_sub(1).map_or(' ', |i| chars[i]);
                let next = chars.get(index + 1).copied().unwrap_or(' ');
                if prev.is_alphanumeric() && next.is_alphanumeric() {
                    out.push('_');
                }
            }
            _ => out.push(*c),
        }
    }
    out
}

fn collapse_whitespace(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for word in input.split_whitespace() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    }
    out
}

fn cap_text(text: &str) -> String {
    if text.chars().count() <= MAX_PURPOSE_CHARS {
        return text.to_string();
    }
    let mut truncated: String = text.chars().take(MAX_PURPOSE_CHARS).collect();
    if let Some(boundary) = truncated.rfind(char::is_whitespace)
        && boundary > 0
    {
        truncated.truncate(boundary);
    }
    let truncated = truncated.trim_end();
    if truncated.is_empty() {
        let mut hard: String = text.chars().take(MAX_PURPOSE_CHARS).collect();
        hard.push('…');
        return hard;
    }
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "perfpal-purpose-{tag}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn read_readme(tag: &str, name: &str, content: &str) -> (PathBuf, Purpose) {
        let root = temp_root(tag);
        fs::write(root.join(name), content).expect("write README");
        let purpose = read(&root, "ENTRY");
        (root, purpose)
    }

    #[test]
    fn markdown_title_badge_and_first_paragraph() {
        let (root, purpose) = read_readme(
            "md-basic",
            "README.md",
            "# Perfpal\n\n\
             [![CI](https://img.shields.io/ci)](https://ci.example)\n\
             ![logo](logo.png)\n\n\
             Fast `static` **analysis** for C and [Rust](https://rust-lang.org).\n\n\
             A second paragraph is ignored.\n",
        );
        assert_eq!(purpose.kind, "readme");
        assert_eq!(purpose.source.as_deref(), Some("README.md"));
        assert_eq!(purpose.purpose_version, 1);
        assert_eq!(purpose.text, "Fast static analysis for C and Rust.");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn markdown_skips_headings_and_toc_blocks() {
        let (root, purpose) = read_readme(
            "md-toc",
            "README.md",
            "# Title\n\n\
             ## Contents\n\n\
             - [Overview](#overview)\n\
             - [Install](#install)\n\
             1. [Usage](#usage)\n\n\
             Actual purpose sentence.\n",
        );
        assert_eq!(purpose.text, "Actual purpose sentence.");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn markdown_strips_inline_links_and_emphasis() {
        let (root, purpose) = read_readme(
            "md-inline",
            "README.md",
            "# T\n\n\
             A [fast](https://example.com/fast) _static_ analyzer for `C` and ~~old~~ Rust.\n",
        );
        assert_eq!(purpose.text, "A fast static analyzer for C and old Rust.");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn purpose_is_capped_at_200_chars_on_a_word_boundary() {
        let paragraph = format!("{}\n", "word ".repeat(60));
        let (root, purpose) = read_readme("md-cap", "README.md", &format!("# T\n\n{paragraph}"));
        assert!(
            purpose.text.chars().count() <= MAX_PURPOSE_CHARS + 1,
            "cap exceeded: {}",
            purpose.text.chars().count()
        );
        assert!(purpose.text.ends_with('…'), "truncation marker missing");
        assert!(
            !purpose.text.trim_end_matches('…').ends_with(' '),
            "cap must fall on a word boundary"
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn readme_is_capped_at_64_kib() {
        // The first paragraph lives beyond the 64 KiB read window, so the
        // capped read sees only the unterminated code fence.
        let beyond_cap = format!(
            "```\n{}\n```\n\nPurpose after the cap.\n",
            "x".repeat(70_000)
        );
        let root = temp_root("md-64k");
        fs::write(root.join("README.md"), beyond_cap).expect("write README");
        let purpose = read(&root, "ENTRY");
        assert_eq!(purpose.kind, "structural");
        assert_eq!(purpose.source, None);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn missing_readme_uses_the_structural_fallback() {
        let root = temp_root("missing");
        let purpose = read(&root, "HEURISTIC · src/main.c:1 main · confidence HIGH");
        assert_eq!(purpose.kind, "structural");
        assert_eq!(purpose.source, None);
        assert_eq!(purpose.purpose_version, 1);
        assert_eq!(
            purpose.text,
            "No README at the scan root; inferred entry: \
             HEURISTIC · src/main.c:1 main · confidence HIGH."
        );
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn rst_title_and_directives_are_skipped() {
        let (root, purpose) = read_readme(
            "rst",
            "README.rst",
            "Perfpal\n========\n\n\
             .. _docs: https://example.com\n\n\
             A *static* analyzer for ``C`` and `Rust <https://rust-lang.org>`_.\n\n\
             Ignored.\n",
        );
        assert_eq!(purpose.kind, "readme");
        assert_eq!(purpose.source.as_deref(), Some("README.rst"));
        assert_eq!(purpose.text, "A static analyzer for C and Rust.");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn purpose_is_deterministic() {
        let (root, first) = read_readme(
            "determinism",
            "README.md",
            "# T\n\nA deterministic purpose line.\n",
        );
        let second = read(&root, "ENTRY");
        assert_eq!(first, second);
        fs::remove_dir_all(root).ok();
    }
}
