//! P3 — signature summaries (`docs/atlas/core-probes.md` P3, Atlas Story S2).
//!
//! Story-only probe: for every drawn file the scan already rendered, parse the
//! file once more and emit the function signatures **as written**, so the
//! story can join `flow.signatures[].id` to `metrics.json`
//! `stats.functions[].id` (the canonical IR identity `path#name#line:col`).
//!
//! Known limits (deliberate, part of the probe contract):
//!
//! - Types are the source text: macros are never expanded, and generics /
//!   `impl Trait` are recorded as written, not resolved. Opaque constructs are
//!   listed in `unresolved` (`"impl Trait"`, `"variadic"`, `"fn-pointer"`)
//!   rather than guessed.
//! - `dir` only distinguishes by-value / const-pointer (`in`) from mutable
//!   pointers (`inout`). `out` is never inferred in S2.
//! - This is static inference: a signature says what the source *writes*, not
//!   what the program does at runtime.
//!
//! Determinism: only the caller-supplied drawn paths are parsed (C/Rust
//! extensions), and the result is sorted `(path, line, name, id)` with
//! `BTreeSet`-backed `unresolved` lists. Unreadable or unparsable files are
//! skipped silently; their call edges stay unresolved in later slices.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde::Serialize;
use tree_sitter::{Node, Parser, Tree};

/// `flow_version` — bumped only when the shape of the flow block changes.
pub(crate) const FLOW_VERSION: u8 = 1;

/// The story-only `flow` block as emitted into `atlas-layout.json`.
#[derive(Debug, Serialize)]
pub(crate) struct Flow {
    pub(crate) flow_version: u8,
    pub(crate) fidelity: &'static str,
    pub(crate) signatures: Vec<Signature>,
}

/// One function signature, keyed by the canonical IR function id.
#[derive(Debug, Serialize)]
pub(crate) struct Signature {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) line: u32,
    pub(crate) params: Vec<Param>,
    pub(crate) ret: Option<String>,
    pub(crate) unresolved: Vec<String>,
}

/// One written parameter: source text, never a resolved type.
#[derive(Debug, Serialize)]
pub(crate) struct Param {
    pub(crate) name: Option<String>,
    #[serde(rename = "type")]
    pub(crate) type_text: String,
    /// `in` or `inout`; `out` is unused in S2.
    pub(crate) dir: &'static str,
}

/// Build the signature list for the drawn files under `root`.
///
/// `root` is the scan root verbatim (`MetricsDocument::scan.root_path`): a
/// directory joins it with each drawn repo-relative path; a single-file scan
/// reads that file directly. Paths with other extensions are ignored.
#[must_use]
pub(crate) fn build(root: &Path, drawn: impl IntoIterator<Item = String>) -> Flow {
    let mut signatures = Vec::new();
    let mut seen = BTreeSet::new();
    for path in drawn {
        if !seen.insert(path.clone()) {
            continue;
        }
        let Some(language) = language_of(&path) else {
            continue;
        };
        let source_path = if root.is_file() {
            root.to_path_buf()
        } else {
            root.join(&path)
        };
        let Ok(source) = fs::read(&source_path) else {
            continue;
        };
        let Some(tree) = parse(language, &source) else {
            continue;
        };
        let mut found = Vec::new();
        collect_functions(tree.root_node(), language, &path, &source, &mut found);
        signatures.extend(found);
    }
    signatures.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.cmp(&b.line))
            .then(a.name.cmp(&b.name))
            .then(a.id.cmp(&b.id))
    });
    Flow {
        flow_version: FLOW_VERSION,
        fidelity: "syntactic",
        signatures,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Language {
    C,
    Rust,
}

fn language_of(path: &str) -> Option<Language> {
    let extension = path.rsplit_once('.')?.1;
    match extension {
        "c" | "h" => Some(Language::C),
        "rs" => Some(Language::Rust),
        _ => None,
    }
}

fn parse(language: Language, source: &[u8]) -> Option<Tree> {
    let mut parser = Parser::new();
    let grammar = match language {
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
    };
    parser.set_language(&grammar).ok()?;
    parser.parse(source, None)
}

/// Walk file-level items, stopping at each function body — the same boundary
/// the scanner backends use, so nested function-local items never produce an
/// id the metrics document does not carry.
fn collect_functions(
    node: Node<'_>,
    language: Language,
    path: &str,
    source: &[u8],
    out: &mut Vec<Signature>,
) {
    let function_kind = match language {
        Language::C => "function_definition",
        Language::Rust => "function_item",
    };
    if node.kind() == function_kind {
        if let Some(signature) = signature(node, language, path, source) {
            out.push(signature);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions(child, language, path, source, out);
    }
}

fn signature(node: Node<'_>, language: Language, path: &str, source: &[u8]) -> Option<Signature> {
    let (name, params, ret, unresolved) = match language {
        Language::C => c_signature(node, source),
        Language::Rust => rust_signature(node, source),
    };
    if name.is_empty() {
        return None;
    }
    let start = node.start_position();
    let line = u32::try_from(start.row + 1).ok()?;
    let column = u32::try_from(start.column + 1).ok()?;
    Some(Signature {
        id: format!("{path}#{name}#{line}:{column}"),
        name,
        path: path.to_string(),
        line,
        params,
        ret,
        unresolved: unresolved.into_iter().collect(),
    })
}

// ---------------------------------------------------------------------------
// C
// ---------------------------------------------------------------------------

fn c_signature(
    fdef: Node<'_>,
    source: &[u8],
) -> (String, Vec<Param>, Option<String>, BTreeSet<String>) {
    let name = c_function_name(fdef, source);
    let ret = fdef
        .child_by_field_name("type")
        .map(|node| collapse(text(node, source)));
    let (params, unresolved) = c_params(fdef, source);
    (name, params, ret, unresolved)
}

fn c_function_name(fdef: Node<'_>, source: &[u8]) -> String {
    let Some(declarator) = fdef.child_by_field_name("declarator") else {
        return String::new();
    };
    let func = find_descendant(declarator, "function_declarator").unwrap_or(declarator);
    func.child_by_field_name("declarator")
        .and_then(leftmost_identifier)
        .map(|node| text(node, source).to_string())
        .unwrap_or_default()
}

fn c_params(fdef: Node<'_>, source: &[u8]) -> (Vec<Param>, BTreeSet<String>) {
    let mut params = Vec::new();
    let mut unresolved = BTreeSet::new();
    let Some(declarator) = fdef.child_by_field_name("declarator") else {
        return (params, unresolved);
    };
    let func = find_descendant(declarator, "function_declarator").unwrap_or(declarator);
    let Some(list) = func.child_by_field_name("parameters") else {
        return (params, unresolved);
    };
    let mut cursor = list.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "parameter_declaration" {
                if let Some(param) = c_param(child, source, &mut unresolved) {
                    params.push(param);
                }
            } else if child.kind() == "..." || child.kind().contains("variadic") {
                unresolved.insert("variadic".to_string());
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    (params, unresolved)
}

fn c_param(param: Node, source: &[u8], unresolved: &mut BTreeSet<String>) -> Option<Param> {
    let declarator = param.child_by_field_name("declarator");
    let name_node = declarator.and_then(leftmost_identifier);
    let full = text(param, source);
    // `(void)` is the C spelling of an empty parameter list.
    if name_node.is_none() && collapse(full) == "void" {
        return None;
    }
    let type_text = match name_node {
        Some(node) => collapse(&splice_without(param, node, source)),
        None => collapse(full),
    };
    if full.contains("(*") {
        unresolved.insert("fn-pointer".to_string());
    }
    let is_pointer = declarator.is_some_and(|node| contains_kind(node, "pointer_declarator"));
    let dir = if c_is_const_qualified(&type_text) && is_pointer {
        "in"
    } else if is_pointer {
        "inout"
    } else {
        "in"
    };
    Some(Param {
        name: name_node.map(|node| text(node, source).to_string()),
        type_text,
        dir,
    })
}

/// `const T *` (const before the first `*`) means read-only data; a
/// pointer-to-const (`char * const p`) is not the same thing.
fn c_is_const_qualified(type_text: &str) -> bool {
    let before_pointer = type_text
        .split_once('*')
        .map_or(type_text, |(head, _)| head);
    before_pointer
        .split_whitespace()
        .any(|token| token == "const")
}

// ---------------------------------------------------------------------------
// Rust
// ---------------------------------------------------------------------------

fn rust_signature(
    node: Node<'_>,
    source: &[u8],
) -> (String, Vec<Param>, Option<String>, BTreeSet<String>) {
    let name = node
        .child_by_field_name("name")
        .map(|node| text(node, source).to_string())
        .unwrap_or_default();
    let mut unresolved = BTreeSet::new();
    let ret = node.child_by_field_name("return_type").map(|node| {
        let written = collapse(text(node, source));
        written
            .strip_prefix("->")
            .unwrap_or(&written)
            .trim()
            .to_string()
    });
    if ret.as_deref().is_some_and(|value| value.contains("impl ")) {
        unresolved.insert("impl Trait".to_string());
    }

    let mut params = Vec::new();
    if let Some(list) = node.child_by_field_name("parameters") {
        let mut cursor = list.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                match child.kind() {
                    "parameter" => {
                        let type_text = child
                            .child_by_field_name("type")
                            .map(|node| collapse(text(node, source)))
                            .unwrap_or_default();
                        if type_text.contains("impl ") {
                            unresolved.insert("impl Trait".to_string());
                        }
                        params.push(Param {
                            name: child
                                .child_by_field_name("pattern")
                                .and_then(leftmost_identifier)
                                .map(|node| text(node, source).to_string()),
                            dir: rust_dir(&type_text),
                            type_text,
                        });
                    }
                    "self_parameter" => {
                        let type_text = collapse(text(child, source));
                        params.push(Param {
                            name: Some("self".to_string()),
                            dir: rust_dir(&type_text),
                            type_text,
                        });
                    }
                    "variadic_parameter" => {
                        unresolved.insert("variadic".to_string());
                    }
                    _ => {}
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    (name, params, ret, unresolved)
}

fn rust_dir(type_text: &str) -> &'static str {
    if type_text.contains("*mut")
        || type_text.contains("&mut")
        || (type_text.contains('&') && type_text.split_whitespace().any(|token| token == "mut"))
    {
        "inout"
    } else {
        "in"
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

fn text<'a>(node: Node, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

/// Collapse every run of whitespace to one space and trim the ends.
fn collapse(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Source text of `node` with `removed`'s bytes cut out (declarator name).
fn splice_without(node: Node<'_>, removed: Node<'_>, source: &[u8]) -> String {
    let mut bytes = Vec::with_capacity(node.end_byte() - node.start_byte());
    bytes.extend_from_slice(&source[node.start_byte()..removed.start_byte()]);
    bytes.extend_from_slice(&source[removed.end_byte()..node.end_byte()]);
    String::from_utf8_lossy(&bytes).into_owned()
}

fn leftmost_identifier(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "identifier" {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = leftmost_identifier(child) {
            return Some(found);
        }
    }
    None
}

fn find_descendant<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_descendant(child, kind) {
            return Some(found);
        }
    }
    None
}

fn contains_kind(node: Node<'_>, kind: &str) -> bool {
    node.kind() == kind || find_descendant(node, kind).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, source: &str) {
        fs::write(dir.join(name), source).expect("write fixture");
    }

    fn flow_for(dir: &Path, paths: &[&str]) -> Flow {
        build(dir, paths.iter().map(|path| (*path).to_string()))
    }

    #[test]
    fn c_params_dirs_and_ret_as_written() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-c-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "a.c",
            "int parse(Config *cfg, const char *path, int flags) { return 0; }\n",
        );

        let flow = flow_for(&dir, &["a.c"]);
        assert_eq!(flow.flow_version, 1);
        assert_eq!(flow.fidelity, "syntactic");
        assert_eq!(flow.signatures.len(), 1);
        let signature = &flow.signatures[0];
        assert_eq!(signature.name, "parse");
        assert_eq!(signature.path, "a.c");
        assert_eq!(signature.id, "a.c#parse#1:1");
        assert_eq!(signature.line, 1);
        assert_eq!(signature.ret.as_deref(), Some("int"));
        assert_eq!(signature.unresolved, Vec::<String>::new());

        let params: Vec<(&str, &str, &str)> = signature
            .params
            .iter()
            .map(|param| {
                (
                    param.name.as_deref().unwrap_or(""),
                    param.type_text.as_str(),
                    param.dir,
                )
            })
            .collect();
        assert_eq!(
            params,
            vec![
                ("cfg", "Config *", "inout"),
                ("path", "const char *", "in"),
                ("flags", "int", "in"),
            ]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rust_params_dirs_and_ret_as_written() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-r-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "a.rs",
            "fn run(cfg: &mut Config, path: &str) -> Result<(), Error> { Ok(()) }\n",
        );

        let flow = flow_for(&dir, &["a.rs"]);
        assert_eq!(flow.signatures.len(), 1);
        let signature = &flow.signatures[0];
        assert_eq!(signature.id, "a.rs#run#1:1");
        assert_eq!(signature.ret.as_deref(), Some("Result<(), Error>"));
        assert_eq!(signature.unresolved, Vec::<String>::new());
        let params: Vec<(&str, &str, &str)> = signature
            .params
            .iter()
            .map(|param| {
                (
                    param.name.as_deref().unwrap_or(""),
                    param.type_text.as_str(),
                    param.dir,
                )
            })
            .collect();
        assert_eq!(
            params,
            vec![("cfg", "&mut Config", "inout"), ("path", "&str", "in")]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn opaque_constructs_are_unresolved_not_guessed() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-u-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "opaque.c",
            "void apply(void (*cb)(int), ...) { (void)cb; }\n",
        );
        write(
            &dir,
            "opaque.rs",
            "fn run(iter: impl Iterator<Item = u8>) -> impl Fn() { || () }\n",
        );

        let flow = flow_for(&dir, &["opaque.c", "opaque.rs"]);
        assert_eq!(flow.signatures.len(), 2);
        let c = &flow.signatures[0];
        assert_eq!(c.path, "opaque.c");
        assert_eq!(c.unresolved, vec!["fn-pointer", "variadic"]);
        assert_eq!(c.params[0].name.as_deref(), Some("cb"));
        assert_eq!(c.params[0].type_text, "void (*)(int)");

        let rust = &flow.signatures[1];
        assert_eq!(rust.path, "opaque.rs");
        assert_eq!(rust.unresolved, vec!["impl Trait"]);
        assert_eq!(rust.params[0].type_text, "impl Iterator<Item = u8>");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn files_outside_the_drawn_set_are_not_parsed() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-f-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(&dir, "drawn.c", "int drawn(void) { return 1; }\n");
        write(&dir, "hidden.c", "int hidden(void) { return 2; }\n");
        write(&dir, "script.py", "def skipped():\n    pass\n");

        let flow = flow_for(&dir, &["drawn.c", "script.py"]);
        let paths: Vec<&str> = flow.signatures.iter().map(|s| s.path.as_str()).collect();
        assert_eq!(paths, vec!["drawn.c"]);
        assert_eq!(flow.signatures[0].name, "drawn");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn single_file_root_is_read_directly() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-s-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let file = dir.join("solo.c");
        write(
            &dir,
            "solo.c",
            "long compute(long value) { return value; }\n",
        );

        let flow = build(&file, ["solo.c".to_string()]);
        assert_eq!(flow.signatures.len(), 1);
        assert_eq!(flow.signatures[0].id, "solo.c#compute#1:1");
        let _ = fs::remove_dir_all(&dir);
    }
}
