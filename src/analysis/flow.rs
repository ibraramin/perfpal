//! P3 signatures + P1 call payloads (`docs/atlas/core-probes.md`, Atlas Story
//! S2).
//!
//! Story-only probe: for every drawn file the scan already rendered, parse the
//! file once more and emit function signatures **as written** plus call sites
//! with their written arguments, so the story can join
//! `flow.signatures[].id` / `flow.calls[].caller` to `metrics.json`
//! `stats.functions[].id` (the canonical IR identity `path#name#line:col`).
//!
//! S3 adds the P2 I/O effects (`flow.io[]`): a fixed, versioned call-name table
//! ([`IO_TABLE_VERSION`] = 1) classifies call sites in the drawn files as a
//! `read`/`write` effect on `file`/`env`/`net`/`stdio`. A `literal` target is
//! recorded only when the designated argument is a **plain string literal**
//! (`string_literal` / `raw_string_literal`); values built by formatting,
//! macro concatenation or C adjacent-string concatenation are never targets.
//! This is a name match, not a runtime path: an I/O wrapper defined outside the
//! drawn set can still be misclassified, which is why the row carries no
//! observed-path claim and the story renders a missing target as `unknown`.
//!
//! Known limits (deliberate, part of the probe contract):
//!
//! - Types are the source text: macros are never expanded, and generics /
//!   `impl Trait` are recorded as written, not resolved. Opaque constructs are
//!   listed in `unresolved` (`"impl Trait"`, `"variadic"`, `"fn-pointer"`,
//!   `"fn-pointer-return"`) rather than guessed. C return types include the
//!   declarator prefix (`char *dup` ⇒ `char *`); function-pointer returns are
//!   best-effort written text and marked `"fn-pointer-return"`.
//! - `dir` is expression/type based: C address-of (`&x`) and non-const pointer
//!   locals/params, Rust `&mut`/`*mut` expressions and `&mut T`/`*mut T`
//!   locals are `inout`; `const T*`, `&T` and values are `in`. `out` is never
//!   inferred in S2.
//! - A callee resolves only when it is a direct name defined exactly once
//!   across the parsed drawn functions: a bare identifier, or a Rust
//!   `scoped_identifier` whose root is `crate`/`self`/`super`. Methods, field
//!   calls, function pointers, dynamic dispatch, macros (collected across all
//!   parsed drawn files), external scoped paths (`std::…`), ambiguous names
//!   and unknown names stay unresolved — never guessed.
//! - A call's `unresolved` flag means "callee unresolved OR every argument is
//!   untyped/non-literal" (zero-arg resolved calls are resolved). It is not a
//!   statement about indirection alone.
//! - Argument types come from the caller's same-function local table only
//!   (parameters plus explicit Rust `let x: T` / C `T x` declarations), with
//!   no cross-file inference. C decimal integer constants are `int` by the
//!   language. Rust reference/pointer types expose their pointee. `named` is
//!   the bare identifier or field-access root only when it resolves; borrowed
//!   expressions such as `&cfg` have `named: null`.
//! - Recursion is capped at [`MAX_DEPTH`]: a pathologically deep expression
//!   yields fewer rows rather than a stack overflow.
//! - I/O rows are call sites inside drawn functions only. A bare C name is
//!   skipped when any drawn file defines a function with that name (a wrapper
//!   must never be classified by name alone); a wrapper defined outside the
//!   drawn set is the documented prototype limit. `fopen`-style calls with a
//!   non-literal mode, and `open`-style calls with non-literal flags, emit no
//!   row because the direction is unknown — never guessed. `read`/`write` are
//!   fd-based: their `file` target is the call class and may be a pipe/socket.
//!   Stdio rows never carry a literal (a format string is not a target).
//! - `literal` is the source text of a plain string literal with the outer
//!   quotes removed; escape sequences are **not** decoded and values are
//!   display-capped. It is a written literal, not an observed runtime value.
//! - This is static inference: the flow says what the source *writes*, not
//!   what the program does at runtime.
//!
//! Determinism: only the caller-supplied drawn paths are parsed (C/Rust
//! extensions), and output is sorted `(path, line, name, id)` for signatures,
//! `(from, line, callee, to, caller)` for calls and
//! `(file, line, callee, op, target, literal)` for I/O, with `BTreeSet`-backed
//! `unresolved` lists and `BTreeMap`-backed edge classification. Unreadable or
//! unparsable files are skipped silently.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Serialize;
use tree_sitter::{Node, Parser, Tree};

/// `flow_version` — bumped only when the shape of the flow block changes.
pub(crate) const FLOW_VERSION: u8 = 1;

/// Recursion guard for pathological syntax trees. Past this depth the probe
/// stops descending: a pathologically deep expression yields fewer rows rather
/// than a stack overflow. No panic path exists behind it.
const MAX_DEPTH: usize = 256;

/// The story-only `flow` block as emitted into `atlas-layout.json`.
#[derive(Debug, Serialize)]
pub(crate) struct Flow {
    pub(crate) flow_version: u8,
    pub(crate) fidelity: &'static str,
    pub(crate) signatures: Vec<Signature>,
    pub(crate) calls: Vec<Call>,
    pub(crate) io: Vec<Io>,
}

/// One I/O effect at a call site in a drawn file (P2, Atlas Story S3).
#[derive(Debug, Serialize)]
pub(crate) struct Io {
    /// Call-site file (a drawn repo-relative path), never the target path.
    pub(crate) file: String,
    pub(crate) line: u32,
    pub(crate) callee: String,
    /// `read` or `write`.
    pub(crate) op: &'static str,
    /// `file`, `env`, `net` or `stdio`.
    pub(crate) target: &'static str,
    /// Plain string-literal target as written (quotes removed, escapes kept);
    /// `None` when the argument is not a plain literal — never guessed.
    pub(crate) literal: Option<String>,
}

/// One call site in a drawn file, with resolved target only when the callee is
/// a direct name that is unique across the parsed drawn functions.
#[derive(Debug, Serialize)]
pub(crate) struct Call {
    pub(crate) from: String,
    pub(crate) to: Option<String>,
    pub(crate) to_id: Option<String>,
    pub(crate) caller: String,
    pub(crate) callee: String,
    pub(crate) line: u32,
    pub(crate) args: Vec<CallArg>,
    pub(crate) unresolved: bool,
}

/// One written argument expression, typed only from the caller's local table.
#[derive(Debug, Serialize)]
pub(crate) struct CallArg {
    pub(crate) expr: String,
    #[serde(rename = "type")]
    pub(crate) type_text: Option<String>,
    /// `in` or `inout`; `out` is unused in S2.
    pub(crate) dir: &'static str,
    /// Bare identifier (or root of a field access) only when it resolves.
    pub(crate) named: Option<String>,
    /// True only for string literals (path/URL observability).
    pub(crate) literal: bool,
}

/// Edge label derived from the calls that cross the edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EdgeLabel {
    pub(crate) payload: Option<String>,
    pub(crate) label: &'static str,
}

/// A call before whole-set name resolution.
struct RawCall {
    from: String,
    language: Language,
    caller: String,
    callee: String,
    direct_name: Option<String>,
    /// True for a C macro invocation or a local function-pointer value: never
    /// table-matched as I/O.
    blocked: bool,
    /// Per-argument plain string-literal value (quotes removed), aligned with
    /// `args`; `None` for non-literals such as `format!(...)`.
    literals: Vec<Option<String>>,
    /// Per-argument collapsed source text, aligned with `args`. Used to read
    /// `open`-style flag expressions (`O_WRONLY` is an identifier, not a
    /// string literal).
    arg_texts: Vec<String>,
    line: u32,
    args: Vec<CallArg>,
}

/// One successfully parsed drawn file, retained for the whole-set passes.
struct ParsedFile {
    path: String,
    language: Language,
    source: Vec<u8>,
    tree: Tree,
}

/// Declared local/parameter type used for argument typing (never inferred).
struct LocalType {
    type_text: String,
    dir: &'static str,
}

/// Version of the fixed I/O call-name table below. Bump when a name, target or
/// direction rule changes; the emitted `flow` shape does not change. The story
/// artifact does not carry the number, so the constant is the merge-contract
/// marker (tests assert it).
#[allow(dead_code)]
pub(crate) const IO_TABLE_VERSION: u8 = 1;

/// Longest recorded literal target (characters). Longer values are display-
/// capped with `...`; this is a display bound, not a runtime value.
const LITERAL_LIMIT: usize = 160;

/// How a table row derives its operation(s).
#[derive(Clone, Copy)]
enum OpRule {
    Read,
    Write,
    /// A call that both reads a source and writes a destination
    /// (`std::fs::copy`).
    Both,
    /// `fopen`-style mode argument: `+` modes are read+write, otherwise the
    /// first character decides (`r` ⇒ read, `w`/`a` ⇒ write). A non-literal
    /// mode yields no row (direction unknown, never guessed).
    Mode(usize),
    /// `open`-style flags argument: `O_WRONLY` ⇒ write, `O_RDWR` ⇒ read+write,
    /// `O_RDONLY`/`0` ⇒ read. A variable, a non-zero numeric literal (the bits
    /// are platform-specific) or a flags expression without an access mode
    /// yields no row (direction unknown, never guessed).
    Flags(usize),
}

/// One row of the fixed v1 I/O table: written callee, effect class and the
/// argument that may carry a path/name/URL literal.
struct IoRule {
    /// The complete written callee: a bare C name (`fopen`) or an exact Rust
    /// path (`File::open`, `std::env::var_os`). Never a suffix match.
    name: &'static str,
    op: OpRule,
    /// `file`, `env`, `net` or `stdio`.
    target: &'static str,
    /// Argument index for a path/name/URL literal; `None` for fd/stream calls
    /// and for stdio (a format string is not a target).
    literal_arg: Option<usize>,
    /// True for bare C names: skip when a drawn file defines the same name
    /// (a user wrapper must never be classified by name alone).
    bare: bool,
}

impl OpRule {
    fn resolve(self, literals: &[Option<String>], texts: &[String]) -> Vec<&'static str> {
        match self {
            OpRule::Read => vec!["read"],
            OpRule::Write => vec!["write"],
            OpRule::Both => vec!["read", "write"],
            OpRule::Mode(index) => {
                let Some(mode) = literals.get(index).and_then(Option::as_deref) else {
                    return Vec::new();
                };
                let both = mode.contains('+');
                match mode.chars().next() {
                    Some('r') if both => vec!["read", "write"],
                    Some('r') => vec!["read"],
                    Some('w' | 'a') if both => vec!["read", "write"],
                    Some('w' | 'a') => vec!["write"],
                    _ => Vec::new(),
                }
            }
            OpRule::Flags(index) => {
                let Some(flags) = texts.get(index) else {
                    return Vec::new();
                };
                if !written_flag_expression(flags) {
                    return Vec::new();
                }
                if flags.contains("O_RDWR") {
                    vec!["read", "write"]
                } else if flags.contains("O_WRONLY") {
                    vec!["write"]
                } else if flags.contains("O_RDONLY") || flags.trim() == "0" {
                    vec!["read"]
                } else {
                    // A non-zero numeric literal (platform-specific bits) or a
                    // flags expression with no access mode: direction unknown.
                    Vec::new()
                }
            }
        }
    }
}

/// Whether an `open`-style flags argument is a written flag expression
/// (`0`, `O_WRONLY`, `O_CREAT | O_WRONLY`) rather than a variable or call.
/// Only a written expression can state the access mode; a bare `flags`
/// identifier (or a lone `x`) yields no row, and non-zero numeric literals are
/// not decoded because their bits differ per platform.
fn written_flag_expression(text: &str) -> bool {
    let mut saw_token = false;
    for part in text.split('|') {
        let token = part.trim();
        if token.is_empty() {
            return false;
        }
        if token.starts_with("O_") {
            saw_token = true;
            continue;
        }
        if numeric_flag_token(token) {
            saw_token = true;
            continue;
        }
        return false;
    }
    saw_token
}

/// A decimal/hex integer token (with optional C suffix). Requiring a leading
/// digit and a digit in the text rejects identifiers such as `x`/`flags`.
fn numeric_flag_token(token: &str) -> bool {
    if let Some(rest) = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
    {
        return !rest.is_empty()
            && rest.chars().all(|character| {
                character.is_ascii_hexdigit() || matches!(character, 'u' | 'U' | 'l' | 'L')
            });
    }
    !token.is_empty()
        && token.chars().all(|character| {
            character.is_ascii_digit() || matches!(character, 'u' | 'U' | 'l' | 'L')
        })
        && token.chars().any(|character| character.is_ascii_digit())
}

/// Fixed, versioned call-name table (v1). Every entry is an explicit API name;
/// there is no fuzzy matching. New names require a table bump and a test.
#[rustfmt::skip]
const IO_RULES: &[IoRule] = &[
    // --- C: files (paths / fd) -------------------------------------------
    IoRule { name: "fopen", op: OpRule::Mode(1), target: "file", literal_arg: Some(0), bare: true },
    IoRule { name: "freopen", op: OpRule::Mode(1), target: "file", literal_arg: Some(0), bare: true },
    IoRule { name: "open", op: OpRule::Flags(1), target: "file", literal_arg: Some(0), bare: true },
    IoRule { name: "openat", op: OpRule::Flags(2), target: "file", literal_arg: Some(1), bare: true },
    IoRule { name: "creat", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: true },
    IoRule { name: "read", op: OpRule::Read, target: "file", literal_arg: None, bare: true },
    IoRule { name: "pread", op: OpRule::Read, target: "file", literal_arg: None, bare: true },
    IoRule { name: "write", op: OpRule::Write, target: "file", literal_arg: None, bare: true },
    IoRule { name: "pwrite", op: OpRule::Write, target: "file", literal_arg: None, bare: true },
    IoRule { name: "stat", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: true },
    IoRule { name: "lstat", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: true },
    IoRule { name: "access", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: true },
    IoRule { name: "unlink", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: true },
    IoRule { name: "remove", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: true },
    IoRule { name: "rename", op: OpRule::Write, target: "file", literal_arg: None, bare: true },
    // --- C: stdio streams (no path literal) ------------------------------
    IoRule { name: "fread", op: OpRule::Read, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "fwrite", op: OpRule::Write, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "fgets", op: OpRule::Read, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "fputs", op: OpRule::Write, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "puts", op: OpRule::Write, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "printf", op: OpRule::Write, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "fprintf", op: OpRule::Write, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "vprintf", op: OpRule::Write, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "vfprintf", op: OpRule::Write, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "putchar", op: OpRule::Write, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "scanf", op: OpRule::Read, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "fscanf", op: OpRule::Read, target: "stdio", literal_arg: None, bare: true },
    IoRule { name: "getchar", op: OpRule::Read, target: "stdio", literal_arg: None, bare: true },
    // --- C: environment and network --------------------------------------
    IoRule { name: "getenv", op: OpRule::Read, target: "env", literal_arg: Some(0), bare: true },
    IoRule { name: "setenv", op: OpRule::Write, target: "env", literal_arg: Some(0), bare: true },
    IoRule { name: "putenv", op: OpRule::Write, target: "env", literal_arg: Some(0), bare: true },
    IoRule { name: "unsetenv", op: OpRule::Write, target: "env", literal_arg: Some(0), bare: true },
    IoRule { name: "connect", op: OpRule::Write, target: "net", literal_arg: None, bare: true },
    IoRule { name: "send", op: OpRule::Write, target: "net", literal_arg: None, bare: true },
    IoRule { name: "sendto", op: OpRule::Write, target: "net", literal_arg: None, bare: true },
    IoRule { name: "recv", op: OpRule::Read, target: "net", literal_arg: None, bare: true },
    IoRule { name: "recvfrom", op: OpRule::Read, target: "net", literal_arg: None, bare: true },
    // --- Rust: files (exact written paths; never a bare `read`) ----------
    IoRule { name: "File::open", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::File::open", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::File::open", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "File::create", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::File::create", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::File::create", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::read", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::read", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::read_to_string", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::read_to_string", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::read_dir", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::read_dir", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::metadata", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::metadata", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::symlink_metadata", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::symlink_metadata", op: OpRule::Read, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::write", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::write", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::create_dir", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::create_dir", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::create_dir_all", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::create_dir_all", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::remove_file", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::remove_file", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::remove_dir", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::remove_dir", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::remove_dir_all", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::remove_dir_all", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::rename", op: OpRule::Write, target: "file", literal_arg: None, bare: false },
    IoRule { name: "fs::rename", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "std::fs::copy", op: OpRule::Both, target: "file", literal_arg: None, bare: false },
    IoRule { name: "fs::copy", op: OpRule::Both, target: "file", literal_arg: None, bare: false },
    IoRule { name: "std::fs::hard_link", op: OpRule::Write, target: "file", literal_arg: None, bare: false },
    IoRule { name: "fs::hard_link", op: OpRule::Write, target: "file", literal_arg: None, bare: false },
    IoRule { name: "std::fs::set_permissions", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    IoRule { name: "fs::set_permissions", op: OpRule::Write, target: "file", literal_arg: Some(0), bare: false },
    // --- Rust: environment and network -----------------------------------
    IoRule { name: "std::env::var", op: OpRule::Read, target: "env", literal_arg: Some(0), bare: false },
    IoRule { name: "env::var", op: OpRule::Read, target: "env", literal_arg: Some(0), bare: false },
    IoRule { name: "std::env::var_os", op: OpRule::Read, target: "env", literal_arg: Some(0), bare: false },
    IoRule { name: "env::var_os", op: OpRule::Read, target: "env", literal_arg: Some(0), bare: false },
    IoRule { name: "std::env::vars", op: OpRule::Read, target: "env", literal_arg: None, bare: false },
    IoRule { name: "env::vars", op: OpRule::Read, target: "env", literal_arg: None, bare: false },
    IoRule { name: "std::env::args", op: OpRule::Read, target: "env", literal_arg: None, bare: false },
    IoRule { name: "env::args", op: OpRule::Read, target: "env", literal_arg: None, bare: false },
    IoRule { name: "std::env::args_os", op: OpRule::Read, target: "env", literal_arg: None, bare: false },
    IoRule { name: "env::args_os", op: OpRule::Read, target: "env", literal_arg: None, bare: false },
    IoRule { name: "std::env::current_dir", op: OpRule::Read, target: "env", literal_arg: None, bare: false },
    IoRule { name: "env::current_dir", op: OpRule::Read, target: "env", literal_arg: None, bare: false },
    IoRule { name: "std::env::set_var", op: OpRule::Write, target: "env", literal_arg: Some(0), bare: false },
    IoRule { name: "env::set_var", op: OpRule::Write, target: "env", literal_arg: Some(0), bare: false },
    IoRule { name: "std::env::remove_var", op: OpRule::Write, target: "env", literal_arg: Some(0), bare: false },
    IoRule { name: "env::remove_var", op: OpRule::Write, target: "env", literal_arg: Some(0), bare: false },
    IoRule { name: "TcpStream::connect", op: OpRule::Write, target: "net", literal_arg: Some(0), bare: false },
    IoRule { name: "std::net::TcpStream::connect", op: OpRule::Write, target: "net", literal_arg: Some(0), bare: false },
    IoRule { name: "reqwest::get", op: OpRule::Read, target: "net", literal_arg: Some(0), bare: false },
    IoRule { name: "reqwest::blocking::get", op: OpRule::Read, target: "net", literal_arg: Some(0), bare: false },
];

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

/// Build the story-only call/signature projection for the drawn files under
/// `root`.
///
/// `root` is the scan root verbatim (`MetricsDocument::scan.root_path`): a
/// directory joins it with each drawn repo-relative path; a single-file scan
/// reads that file directly. Paths with other extensions are ignored.
#[must_use]
pub(crate) fn build(root: &Path, drawn: impl IntoIterator<Item = String>) -> Flow {
    let mut files = Vec::new();
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
        files.push(ParsedFile {
            path,
            language,
            source,
            tree,
        });
    }

    // Macro names are collected across ALL parsed drawn files before any call
    // is extracted, so a macro defined in another drawn file still excludes a
    // same-named call site from direct resolution.
    let mut macro_names = BTreeSet::new();
    for file in &files {
        collect_macro_names(
            file.tree.root_node(),
            file.language,
            &file.source,
            &mut macro_names,
            0,
        );
    }

    let mut signatures = Vec::new();
    let mut raw_calls = Vec::new();
    for file in &files {
        let context = FileContext {
            language: file.language,
            path: &file.path,
            source: &file.source,
            macro_names: &macro_names,
        };
        collect_functions(
            file.tree.root_node(),
            &context,
            &mut signatures,
            &mut raw_calls,
            0,
        );
    }

    // Direct-name resolution is whole-set only: a name is resolvable when
    // exactly one parsed drawn function defines it. Everything else stays
    // unresolved (never guessed).
    let mut by_name: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for signature in &signatures {
        by_name
            .entry(signature.name.clone())
            .or_default()
            .push((signature.id.clone(), signature.path.clone()));
    }
    let mut calls = Vec::with_capacity(raw_calls.len());
    let mut io = Vec::new();
    for raw in raw_calls {
        let resolved = raw
            .direct_name
            .as_ref()
            .and_then(|name| by_name.get(name))
            .and_then(|entries| (entries.len() == 1).then(|| entries[0].clone()));
        let (to, to_id) = match resolved {
            Some((id, path)) => (Some(path), Some(id)),
            None => (None, None),
        };
        let unresolved = to_id.is_none()
            || (!raw.args.is_empty()
                && !raw
                    .args
                    .iter()
                    .any(|arg| arg.type_text.is_some() || arg.literal));

        // P2: fixed-table I/O effects. A bare name defined by any drawn
        // function is a local wrapper, not the library call; macros and
        // function-pointer values are blocked. No rule ⇒ no row.
        for rule in IO_RULES {
            if raw.blocked || raw.callee != rule.name {
                continue;
            }
            if rule.bare && (raw.language != Language::C || by_name.contains_key(rule.name)) {
                continue;
            }
            let literal = rule
                .literal_arg
                .and_then(|index| raw.literals.get(index).cloned().flatten());
            for op in rule.op.resolve(&raw.literals, &raw.arg_texts) {
                io.push(Io {
                    file: raw.from.clone(),
                    line: raw.line,
                    callee: raw.callee.clone(),
                    op,
                    target: rule.target,
                    literal: literal.clone(),
                });
            }
        }

        calls.push(Call {
            from: raw.from,
            to,
            to_id,
            caller: raw.caller,
            callee: raw.callee,
            line: raw.line,
            args: raw.args,
            unresolved,
        });
    }

    signatures.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then(a.line.cmp(&b.line))
            .then(a.name.cmp(&b.name))
            .then(a.id.cmp(&b.id))
    });
    calls.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then(a.line.cmp(&b.line))
            .then(a.callee.cmp(&b.callee))
            .then(a.to.cmp(&b.to))
            .then(a.caller.cmp(&b.caller))
    });
    io.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.line.cmp(&b.line))
            .then(a.callee.cmp(&b.callee))
            .then(a.op.cmp(b.op))
            .then(a.target.cmp(b.target))
            .then(a.literal.cmp(&b.literal))
    });
    Flow {
        flow_version: FLOW_VERSION,
        fidelity: "syntactic",
        signatures,
        calls,
        io,
    }
}

impl Flow {
    /// Classify an edge `(from, to)` from its matching calls.
    ///
    /// Any string-literal argument wins (`literal-observed`); otherwise the
    /// most frequent written type (`type-only`, ties lexicographically
    /// smallest); otherwise `unresolved`. No matching call is unresolved.
    #[must_use]
    pub(crate) fn edge_label(&self, from: &str, to: &str) -> EdgeLabel {
        let mut matching = self
            .calls
            .iter()
            .filter(|call| call.from == from && call.to.as_deref() == Some(to));
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for call in matching.by_ref() {
            for arg in &call.args {
                if arg.literal {
                    return EdgeLabel {
                        payload: Some(literal_payload(&arg.expr)),
                        label: "literal-observed",
                    };
                }
                if let Some(type_text) = arg.type_text.as_deref() {
                    *counts.entry(type_text).or_insert(0) += 1;
                }
            }
        }
        let mut best: Option<(&str, usize)> = None;
        for (type_text, count) in counts {
            if best.is_none_or(|(_, best_count)| count > best_count) {
                best = Some((type_text, count));
            }
        }
        if let Some((type_text, _)) = best {
            return EdgeLabel {
                payload: Some(type_text.to_string()),
                label: "type-only",
            };
        }
        EdgeLabel {
            payload: None,
            label: "unresolved",
        }
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

struct FileContext<'a> {
    language: Language,
    path: &'a str,
    source: &'a [u8],
    macro_names: &'a BTreeSet<String>,
}

/// Walk file-level items, stopping at each function body — the same boundary
/// the scanner backends use, so nested function-local items never produce an
/// id the metrics document does not carry.
fn collect_functions(
    node: Node<'_>,
    context: &FileContext<'_>,
    signatures: &mut Vec<Signature>,
    calls: &mut Vec<RawCall>,
    depth: usize,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let function_kind = match context.language {
        Language::C => "function_definition",
        Language::Rust => "function_item",
    };
    if node.kind() == function_kind {
        if let Some(signature) = signature(node, context.language, context.path, context.source) {
            let mut table = BTreeMap::new();
            for param in &signature.params {
                if let Some(name) = &param.name {
                    table.insert(
                        name.clone(),
                        LocalType {
                            type_text: param.type_text.clone(),
                            dir: param.dir,
                        },
                    );
                }
            }
            if let Some(body) = node.child_by_field_name("body") {
                match context.language {
                    Language::C => collect_c_locals(body, context.source, &mut table, 0),
                    Language::Rust => collect_rust_locals(body, context.source, &mut table, 0),
                }
                let call_context = CallContext {
                    language: context.language,
                    source: context.source,
                    table: &table,
                    macro_names: context.macro_names,
                    from: context.path,
                    caller: &signature.id,
                };
                collect_calls(body, &call_context, calls, 0);
            }
            signatures.push(signature);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions(child, context, signatures, calls, depth + 1);
    }
}

/// Macro names are excluded from direct resolution: a C function-like macro
/// invocation is syntactically a `call_expression`, and must never become a
/// resolved function call. Rust macro invocations are a distinct node kind and
/// are simply not walked as calls.
fn collect_macro_names(
    node: Node<'_>,
    language: Language,
    source: &[u8],
    out: &mut BTreeSet<String>,
    depth: usize,
) {
    if depth > MAX_DEPTH {
        return;
    }
    if language == Language::C
        && matches!(node.kind(), "preproc_def" | "preproc_function_def")
        && let Some(name) = node.child_by_field_name("name")
    {
        out.insert(text(name, source).to_string());
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_macro_names(child, language, source, out, depth + 1);
    }
}

// ---------------------------------------------------------------------------
// Call sites
// ---------------------------------------------------------------------------

struct CallContext<'a> {
    language: Language,
    source: &'a [u8],
    table: &'a BTreeMap<String, LocalType>,
    macro_names: &'a BTreeSet<String>,
    from: &'a str,
    caller: &'a str,
}

fn collect_calls(node: Node<'_>, context: &CallContext<'_>, out: &mut Vec<RawCall>, depth: usize) {
    if depth > MAX_DEPTH {
        return;
    }
    let function_kind = match context.language {
        Language::C => "function_definition",
        Language::Rust => "function_item",
    };
    if node.kind() == function_kind {
        return;
    }
    if node.kind() == "call_expression"
        && let Some(call) = build_call(node, context)
    {
        out.push(call);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls(child, context, out, depth + 1);
    }
}

fn build_call(node: Node<'_>, context: &CallContext<'_>) -> Option<RawCall> {
    let function = node.child_by_field_name("function")?;
    let arguments = node.child_by_field_name("arguments")?;
    let (callee, direct_name, blocked) = callee_of(function, context);
    let mut args = Vec::new();
    let mut literals = Vec::new();
    let mut arg_texts = Vec::new();
    let mut cursor = arguments.walk();
    for child in arguments.named_children(&mut cursor) {
        args.push(build_arg(child, context));
        literals.push(plain_string_literal(child, context.source));
        arg_texts.push(collapse(text(child, context.source)));
    }
    let line = u32::try_from(node.start_position().row + 1).ok()?;
    Some(RawCall {
        from: context.from.to_string(),
        language: context.language,
        caller: context.caller.to_string(),
        callee,
        direct_name,
        blocked,
        literals,
        arg_texts,
        line,
        args,
    })
}

/// `callee` is the call target as written; `direct_name` is `Some` only for a
/// plain identifier or a Rust scoped identifier's final segment, and only when
/// the target is not a macro and not a local/parameter value (this catches
/// function-pointer locals, including typedef'd pointers and closures that a
/// textual type check would miss). `blocked` marks such a target so the I/O
/// table cannot match it.
fn callee_of(function: Node<'_>, context: &CallContext<'_>) -> (String, Option<String>, bool) {
    let written = collapse(text(function, context.source));
    match (context.language, function.kind()) {
        (_, "identifier") => {
            let name = text(function, context.source).to_string();
            // Any local/parameter value is not a direct library call: a local
            // shadow, a function-pointer (including a typedef'd one) or a
            // closure. Macros are not calls at all. Both are blocked.
            let local_value = context.table.contains_key(&name);
            if context.macro_names.contains(&name) || local_value {
                (written, None, true)
            } else {
                (written, Some(name), false)
            }
        }
        (Language::Rust, "scoped_identifier") => {
            // Only crate-internal paths are eligible. `std::fs::read` must not
            // resolve to a unique drawn `fn read`.
            let root = function
                .child_by_field_name("path")
                .and_then(|path| rust_path_root(path, context.source));
            let name = if matches!(root, Some("crate" | "self" | "super")) {
                function
                    .child_by_field_name("name")
                    .map(|node| text(node, context.source).to_string())
            } else {
                None
            };
            (written, name, false)
        }
        _ => (written, None, false),
    }
}

/// Value of a **plain** string literal with the outer quotes removed (escapes
/// left as written). `None` for anything built at runtime: `format!`/`concat!`,
/// C adjacent-string concatenation (`concatenated_string`), identifiers and
/// expressions. This is the one gate that keeps a formatted value from being
/// shown as an observed target.
fn plain_string_literal(node: Node<'_>, source: &[u8]) -> Option<String> {
    if !matches!(node.kind(), "string_literal" | "raw_string_literal") {
        return None;
    }
    let written = text(node, source);
    let start = written.find('"')?;
    let end = written.rfind('"')?;
    if end <= start {
        return None;
    }
    Some(truncate(&written[start + 1..end], LITERAL_LIMIT))
}

/// First segment of a Rust scoped path (`crate`, `self`, `super`, or a crate
/// name). Only the first three make the path crate-internal.
fn rust_path_root<'a>(node: Node<'a>, source: &'a [u8]) -> Option<&'a str> {
    match node.kind() {
        "identifier" | "crate" | "self" | "super" => Some(text(node, source)),
        "scoped_identifier" => node
            .child_by_field_name("path")
            .and_then(|path| rust_path_root(path, source)),
        _ => None,
    }
}

fn build_arg(arg: Node<'_>, context: &CallContext<'_>) -> CallArg {
    let source = context.source;
    let raw_expr = collapse(text(arg, source));
    let lookup = expression_identifier(arg, context.language).and_then(|node| {
        context
            .table
            .get(text(node, source))
            .map(|local| (node, local))
    });
    let literal = matches!(
        arg.kind(),
        "string_literal" | "raw_string_literal" | "concatenated_string"
    );
    let type_text = if literal {
        Some(
            match context.language {
                Language::C => "char*",
                Language::Rust => "str",
            }
            .to_string(),
        )
    } else if let Some((_, local)) = lookup {
        match context.language {
            Language::C => Some(local.type_text.clone()),
            Language::Rust => Some(rust_pointee(&local.type_text)),
        }
    } else if context.language == Language::C && is_c_integer_literal(arg, source) {
        // A C decimal integer constant is `int` by the language, not a guess.
        Some("int".to_string())
    } else {
        None
    };
    let named = if matches!(arg.kind(), "identifier" | "field_expression") {
        lookup.map(|(node, _)| text(node, source).to_string())
    } else {
        None
    };
    let dir = match context.language {
        Language::C => {
            let address_of = arg.kind() == "pointer_expression"
                && text(arg, source).trim_start().starts_with('&');
            let mutable_pointer =
                arg.kind() == "identifier" && lookup.is_some_and(|(_, local)| local.dir == "inout");
            if address_of || mutable_pointer {
                "inout"
            } else {
                "in"
            }
        }
        Language::Rust => {
            let mutable_expression = raw_expr.contains("&mut") || raw_expr.contains("*mut");
            let mutable_local = arg.kind() == "identifier"
                && lookup.is_some_and(|(_, local)| is_mutable_rust_type(&local.type_text));
            if mutable_expression || mutable_local {
                "inout"
            } else {
                "in"
            }
        }
    };
    CallArg {
        expr: truncate(&raw_expr, 48),
        type_text,
        dir,
        named,
        literal,
    }
}

/// Root identifier of an expression: bare identifier, through `&`/`*` wrappers,
/// or the base of a field access. Anything else has no local identity.
fn expression_identifier(node: Node<'_>, language: Language) -> Option<Node<'_>> {
    expression_identifier_at(node, language, 0)
}

fn expression_identifier_at(node: Node<'_>, language: Language, depth: usize) -> Option<Node<'_>> {
    if depth > MAX_DEPTH {
        return None;
    }
    match node.kind() {
        "identifier" => Some(node),
        "reference_expression" => node
            .child_by_field_name("value")
            .and_then(|child| expression_identifier_at(child, language, depth + 1)),
        "unary_expression" | "pointer_expression" => node
            .child_by_field_name("argument")
            .or_else(|| node.named_child(0))
            .and_then(|child| expression_identifier_at(child, language, depth + 1)),
        "parenthesized_expression" => node
            .named_child(0)
            .and_then(|child| expression_identifier_at(child, language, depth + 1)),
        "field_expression" => {
            let base = match language {
                Language::C => node.child_by_field_name("argument"),
                Language::Rust => node.child_by_field_name("value"),
            };
            base.and_then(|child| expression_identifier_at(child, language, depth + 1))
        }
        "scoped_identifier" => node
            .child_by_field_name("name")
            .and_then(|child| expression_identifier_at(child, language, depth + 1)),
        _ => None,
    }
}

/// Whether a written Rust type is a mutable reference/pointer (`&mut T`,
/// `&'a mut T`, `*mut T`). Immutable `&T`/`*const T` stay `in`.
fn is_mutable_rust_type(type_text: &str) -> bool {
    let mut value = type_text.trim();
    if let Some(rest) = value.strip_prefix('&') {
        value = rest.trim_start();
        if value.starts_with('\'') {
            value = value
                .split_once(char::is_whitespace)
                .map_or(value, |(_, rest)| rest.trim_start());
        }
        return value.starts_with("mut ");
    }
    value.starts_with("*mut ")
}

/// Rust references/pointers expose their pointee for argument typing.
fn rust_pointee(type_text: &str) -> String {
    let mut value = type_text.trim();
    if let Some(rest) = value.strip_prefix('&') {
        value = rest.trim_start();
        if value.starts_with('\'') {
            value = value
                .split_once(char::is_whitespace)
                .map_or(value, |(_, rest)| rest.trim_start());
        }
        if let Some(rest) = value.strip_prefix("mut ") {
            value = rest.trim_start();
        }
    } else if let Some(rest) = value
        .strip_prefix("*const ")
        .or_else(|| value.strip_prefix("*mut "))
    {
        value = rest.trim_start();
    }
    collapse(value)
}

fn is_c_integer_literal(node: Node<'_>, source: &[u8]) -> bool {
    node.kind() == "number_literal"
        && text(node, source).chars().all(|character| {
            character.is_ascii_digit() || matches!(character, 'u' | 'U' | 'l' | 'L')
        })
}

fn truncate(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        value.to_string()
    } else {
        let mut truncated: String = value.chars().take(limit).collect();
        truncated.push_str("...");
        truncated
    }
}

/// Edge payload for a string literal: the source text, kept quoted and capped
/// at 24 characters (the closing quote is preserved when truncating).
fn literal_payload(value: &str) -> String {
    const LIMIT: usize = 24;
    let characters: Vec<char> = value.chars().collect();
    if characters.len() <= LIMIT {
        return value.to_string();
    }
    match characters.iter().position(|character| *character == '"') {
        Some(start) => {
            let inner = LIMIT.saturating_sub(start + 2);
            let mut payload: String = characters[..=start].iter().collect();
            payload.extend(characters[start + 1..].iter().take(inner));
            payload.push('"');
            payload
        }
        None => characters[..LIMIT].iter().collect(),
    }
}

// ---------------------------------------------------------------------------
// Local declarations (narrow, same-file only)
// ---------------------------------------------------------------------------

fn collect_c_locals(
    node: Node<'_>,
    source: &[u8],
    table: &mut BTreeMap<String, LocalType>,
    depth: usize,
) {
    if depth > MAX_DEPTH || node.kind() == "function_definition" {
        return;
    }
    if node.kind() == "declaration" {
        collect_c_declaration(node, source, table);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_c_locals(child, source, table, depth + 1);
    }
}

fn collect_c_declaration(
    declaration: Node<'_>,
    source: &[u8],
    table: &mut BTreeMap<String, LocalType>,
) {
    let mut declarators = Vec::new();
    let mut first_start = None;
    let mut cursor = declaration.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            let field_declarator = cursor.field_name() == Some("declarator");
            if field_declarator || child.kind() == "init_declarator" {
                let declarator = match child.kind() {
                    "init_declarator" => child.child_by_field_name("declarator"),
                    _ => Some(child),
                };
                if let Some(declarator) = declarator {
                    first_start.get_or_insert(declarator.start_byte());
                    declarators.push(declarator);
                }
            }
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
    let Some(prefix_start) = first_start else {
        return;
    };
    let prefix =
        String::from_utf8_lossy(&source[declaration.start_byte()..prefix_start]).into_owned();
    for declarator in declarators {
        let Some(name) = leftmost_identifier(declarator) else {
            continue;
        };
        let decoration = splice_without(declarator, name, source);
        let type_text = collapse(&format!("{prefix}{decoration}"));
        let is_pointer = contains_kind(declarator, "pointer_declarator");
        let dir = if is_pointer && !c_is_const_qualified(&type_text) {
            "inout"
        } else {
            "in"
        };
        table.insert(text(name, source).to_string(), LocalType { type_text, dir });
    }
}

fn collect_rust_locals(
    node: Node<'_>,
    source: &[u8],
    table: &mut BTreeMap<String, LocalType>,
    depth: usize,
) {
    if depth > MAX_DEPTH || node.kind() == "function_item" {
        return;
    }
    if node.kind() == "let_declaration"
        && let (Some(pattern), Some(type_node)) = (
            node.child_by_field_name("pattern"),
            node.child_by_field_name("type"),
        )
        && let Some(name) = leftmost_identifier(pattern)
    {
        let type_text = collapse(text(type_node, source));
        table.insert(
            text(name, source).to_string(),
            LocalType {
                dir: rust_dir(&type_text),
                type_text,
            },
        );
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_rust_locals(child, source, table, depth + 1);
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
    let (ret, function_pointer_return) = c_return(fdef, source);
    let (params, mut unresolved) = c_params(fdef, source);
    if function_pointer_return {
        unresolved.insert("fn-pointer-return".to_string());
    }
    (name, params, ret, unresolved)
}

/// Return type as written: the declaration specifiers plus the declarator
/// prefix before the function name (`char *dup` ⇒ `char *`,
/// `const char *name` ⇒ `const char *`, `int **grid` ⇒ `int **`).
///
/// The pointer/qualifier text lives in the declarator, not the `type` field
/// (`src/scanner/backends/tree_sitter_c/mod.rs:459-461`). A function returning
/// a function pointer is recorded best-effort (`int (*f(void))(int)` ⇒
/// `int (*)(int)`) and flagged `fn-pointer-return`; it is never guessed.
fn c_return(fdef: Node<'_>, source: &[u8]) -> (Option<String>, bool) {
    let Some(declarator) = fdef.child_by_field_name("declarator") else {
        return (None, false);
    };
    let func = find_descendant(declarator, "function_declarator").unwrap_or(declarator);
    let Some(name) = func
        .child_by_field_name("declarator")
        .and_then(leftmost_identifier)
    else {
        return (
            fdef.child_by_field_name("type")
                .map(|node| collapse(text(node, source))),
            false,
        );
    };
    let start = c_type_start(fdef);
    let written = collapse(&String::from_utf8_lossy(&source[start..name.start_byte()]));
    let function_pointer_return = func.kind() == "function_declarator"
        && written.contains("(*")
        && func
            .child_by_field_name("declarator")
            .is_some_and(|inner| inner.kind() != "identifier");
    if function_pointer_return {
        let parameters = func
            .child_by_field_name("parameters")
            .map(|node| collapse(text(node, source)))
            .unwrap_or_default();
        let written = if written.ends_with('*') {
            format!("{written}){parameters}")
        } else {
            format!("{written} (*){parameters}")
        };
        return (Some(written), true);
    }
    if written.is_empty() {
        return (
            fdef.child_by_field_name("type")
                .map(|node| collapse(text(node, source))),
            false,
        );
    }
    (Some(written), false)
}

/// First byte of the return type: the earliest declaration specifier before
/// the declarator, excluding storage classes (`static`, `extern`, `inline`).
fn c_type_start(fdef: Node<'_>) -> usize {
    let declarator_start = fdef
        .child_by_field_name("declarator")
        .map_or(fdef.end_byte(), |node| node.start_byte());
    let mut start = fdef
        .child_by_field_name("type")
        .map_or(fdef.start_byte(), |node| node.start_byte());
    let mut cursor = fdef.walk();
    for child in fdef.named_children(&mut cursor) {
        if child.start_byte() >= declarator_start {
            break;
        }
        if matches!(
            child.kind(),
            "storage_class_specifier" | "attribute_specifier" | "attribute_declaration"
        ) {
            continue;
        }
        start = child.start_byte();
        break;
    }
    start
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

/// The `function_declarator` that owns the function's **name** and its own
/// parameter list. A function returning a function pointer
/// (`int (*f(void))(int)`) has an outer declarator whose parameters belong to
/// the returned function; choose the deepest name-owning declarator and never
/// descend into a parameter list.
fn deepest_name_declarator<'a>(
    node: Node<'a>,
    name: &str,
    source: &[u8],
    depth: usize,
    best: &mut Option<Node<'a>>,
) {
    if depth > MAX_DEPTH || matches!(node.kind(), "parameter_list" | "parameters") {
        return;
    }
    let owns_name = node.kind() == "function_declarator"
        && node
            .child_by_field_name("declarator")
            .and_then(leftmost_identifier)
            .is_some_and(|identifier| text(identifier, source) == name);
    if owns_name && best.is_none_or(|current| node.start_byte() >= current.start_byte()) {
        *best = Some(node);
    }
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            deepest_name_declarator(cursor.node(), name, source, depth + 1, best);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

fn c_params(fdef: Node<'_>, source: &[u8]) -> (Vec<Param>, BTreeSet<String>) {
    let mut params = Vec::new();
    let mut unresolved = BTreeSet::new();
    let name = c_function_name(fdef, source);
    let Some(declarator) = fdef.child_by_field_name("declarator") else {
        return (params, unresolved);
    };
    let mut func = None;
    deepest_name_declarator(declarator, &name, source, 0, &mut func);
    let Some(func) = func else {
        return (params, unresolved);
    };
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
    leftmost_identifier_at(node, 0)
}

fn leftmost_identifier_at(node: Node<'_>, depth: usize) -> Option<Node<'_>> {
    if depth > MAX_DEPTH {
        return None;
    }
    if node.kind() == "identifier" {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = leftmost_identifier_at(child, depth + 1) {
            return Some(found);
        }
    }
    None
}

fn find_descendant<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    find_descendant_at(node, kind, 0)
}

fn find_descendant_at<'a>(node: Node<'a>, kind: &str, depth: usize) -> Option<Node<'a>> {
    if depth > MAX_DEPTH {
        return None;
    }
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_descendant_at(child, kind, depth + 1) {
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
    #[test]
    fn c_calls_carry_local_types_and_a_literal_edge_label() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-call-c-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "callee.c",
            "int parse(Config *cfg, const char *path, int flags) { return 0; }\n\
             void g(Config *cfg) { (void)cfg; }\n",
        );
        write(
            &dir,
            "caller.c",
            "int caller(void) {\n    Config cfg;\n    parse(&cfg, \"config.toml\", 1);\n    g(&cfg);\n    return 0;\n}\n",
        );

        let flow = flow_for(&dir, &["callee.c", "caller.c"]);
        assert_eq!(flow.calls.len(), 2);
        let call = &flow.calls[0];
        assert_eq!(call.from, "caller.c");
        assert_eq!(call.to.as_deref(), Some("callee.c"));
        assert_eq!(call.to_id.as_deref(), Some("callee.c#parse#1:1"));
        assert_eq!(call.caller, "caller.c#caller#1:1");
        assert_eq!(call.callee, "parse");
        assert_eq!(call.line, 3);
        assert!(!call.unresolved);

        let types: Vec<Option<&str>> = call
            .args
            .iter()
            .map(|arg| arg.type_text.as_deref())
            .collect();
        assert_eq!(types, vec![Some("Config"), Some("char*"), Some("int")]);
        let literals: Vec<bool> = call.args.iter().map(|arg| arg.literal).collect();
        assert_eq!(literals, vec![false, true, false]);
        let dirs: Vec<&str> = call.args.iter().map(|arg| arg.dir).collect();
        assert_eq!(dirs, vec!["inout", "in", "in"], "address-of is inout");

        let g = flow
            .calls
            .iter()
            .find(|call| call.callee == "g")
            .expect("g call");
        assert_eq!(g.args[0].dir, "inout", "C address-of is inout");

        let label = flow.edge_label("caller.c", "callee.c");
        assert_eq!(label.label, "literal-observed");
        assert_eq!(label.payload.as_deref(), Some("\"config.toml\""));
        assert_eq!(flow.edge_label("caller.c", "callee.c"), label);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rust_calls_type_only_and_opaque_calls_stay_unresolved() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-call-r-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "a.rs",
            "struct Config;\n\
             struct Thing;\n\
             fn run(cfg: &Config) { let _ = cfg; }\n\
             fn mutate(cfg: &mut Config) { let _ = cfg; }\n\
             fn ping() {}\n\
             fn caller(cfg: Config) {\n\
                 let mut local: Config = cfg;\n\
                 run(&local);\n\
                 mutate(&mut local);\n\
                 ping();\n\
             }\n\
             fn pointer_call(f: fn(&Config), cfg: Config) { f(&cfg); }\n\
             fn method_call(obj: Thing) { obj.method(); }\n\
             fn call_parse() { parse(); }\n",
        );
        write(&dir, "parse_a.rs", "fn parse() {}\n");
        write(&dir, "parse_b.rs", "fn parse() {}\n");

        let flow = flow_for(&dir, &["a.rs", "parse_a.rs", "parse_b.rs"]);
        assert_eq!(flow.calls.len(), 6);

        let run = flow
            .calls
            .iter()
            .find(|call| call.callee == "run")
            .expect("run call");
        assert_eq!(run.to.as_deref(), Some("a.rs"));
        assert_eq!(run.to_id.as_deref(), Some("a.rs#run#3:1"));
        assert!(!run.unresolved);
        assert_eq!(run.args[0].type_text.as_deref(), Some("Config"));
        assert!(!run.args[0].literal);
        assert_eq!(run.args[0].dir, "in", "immutable borrow stays in");

        let mutate = flow
            .calls
            .iter()
            .find(|call| call.callee == "mutate")
            .expect("mutate call");
        assert_eq!(mutate.args[0].dir, "inout", "&mut is inout");
        assert_eq!(mutate.args[0].type_text.as_deref(), Some("Config"));

        let ping = flow
            .calls
            .iter()
            .find(|call| call.callee == "ping")
            .expect("ping call");
        assert!(!ping.unresolved, "zero-arg resolved calls are resolved");
        assert!(ping.args.is_empty());

        let function_pointer = flow
            .calls
            .iter()
            .find(|call| call.callee == "f")
            .expect("function pointer call");
        assert!(function_pointer.unresolved);
        assert!(function_pointer.to.is_none());

        let method = flow
            .calls
            .iter()
            .find(|call| call.callee == "obj.method")
            .expect("method call");
        assert!(method.unresolved);
        assert!(method.to.is_none());

        let ambiguous = flow
            .calls
            .iter()
            .find(|call| call.callee == "parse")
            .expect("ambiguous call");
        assert!(
            ambiguous.unresolved,
            "duplicate definition names are ambiguous"
        );
        assert!(ambiguous.to.is_none());

        let label = flow.edge_label("a.rs", "a.rs");
        assert_eq!(label.label, "type-only");
        assert_eq!(label.payload.as_deref(), Some("Config"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn c_return_types_keep_pointer_and_qualifiers() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-ret-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "ret.c",
            "char *dup(const char *s) { return 0; }\n\
             const char *name(void) { return 0; }\n\
             int **grid(void) { return 0; }\n\
             int plain(void) { return 0; }\n\
             int (*factory(void))(int) { return 0; }\n",
        );

        let flow = flow_for(&dir, &["ret.c"]);
        let by_name: BTreeMap<&str, &Signature> = flow
            .signatures
            .iter()
            .map(|signature| (signature.name.as_str(), signature))
            .collect();
        assert_eq!(by_name["dup"].ret.as_deref(), Some("char *"));
        assert_eq!(by_name["name"].ret.as_deref(), Some("const char *"));
        assert_eq!(by_name["grid"].ret.as_deref(), Some("int **"));
        assert_eq!(by_name["plain"].ret.as_deref(), Some("int"));
        assert!(
            by_name["factory"]
                .ret
                .as_deref()
                .is_some_and(|ret| { ret.starts_with("int (") && ret.ends_with(")(int)") }),
            "best-effort function-pointer return text: {:?}",
            by_name["factory"].ret
        );
        assert_eq!(
            by_name["factory"].unresolved,
            vec!["fn-pointer-return".to_string()]
        );
        assert!(
            by_name["factory"].params.is_empty(),
            "fn-pointer-returning function must not inherit the returned function's parameters: {:?}",
            by_name["factory"].params
        );
        assert!(by_name["dup"].unresolved.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn macros_defined_in_another_drawn_file_never_resolve() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-macro-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(&dir, "macro.c", "#define PARSE(x) x\n");
        write(&dir, "parse_fn.c", "int PARSE(int x) { return x; }\n");
        write(&dir, "caller.c", "int caller(void) { return PARSE(1); }\n");

        let flow = flow_for(&dir, &["macro.c", "parse_fn.c", "caller.c"]);
        let call = flow
            .calls
            .iter()
            .find(|call| call.callee == "PARSE")
            .expect("macro call site");
        assert!(call.unresolved);
        assert!(call.to.is_none(), "a cross-file macro must not resolve");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rust_scoped_paths_resolve_only_crate_internal_roots() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-scope-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "a.rs",
            "fn read(p: &str) { let _ = p; }\n\
             fn parse() {}\n\
             fn caller() {\n\
                 std::fs::read(\"x\");\n\
                 crate::util::parse();\n\
             }\n",
        );

        let flow = flow_for(&dir, &["a.rs"]);
        let external = flow
            .calls
            .iter()
            .find(|call| call.callee == "std::fs::read")
            .expect("external scoped call");
        assert!(external.unresolved, "std::fs::read must not resolve");
        assert!(external.to.is_none());

        let internal = flow
            .calls
            .iter()
            .find(|call| call.callee == "crate::util::parse")
            .expect("crate scoped call");
        assert!(!internal.unresolved, "crate-internal scoped call resolves");
        assert_eq!(internal.to_id.as_deref(), Some("a.rs#parse#2:1"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn edge_without_matching_calls_is_unresolved() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-call-e-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(&dir, "a.c", "int a(void) { return 0; }\n");
        let flow = flow_for(&dir, &["a.c"]);
        let label = flow.edge_label("a.c", "b.c");
        assert_eq!(label.label, "unresolved");
        assert!(label.payload.is_none());
        let _ = fs::remove_dir_all(&dir);
    }
    #[test]
    fn deeply_nested_expressions_do_not_panic() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-depth-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let nested = format!("{}1{}", "(".repeat(400), ")".repeat(400));
        write(
            &dir,
            "a.rs",
            &format!("fn sink(x: i32) {{ let _ = x; }}\nfn caller() {{ sink({nested}); }}\n"),
        );
        let flow = flow_for(&dir, &["a.rs"]);
        assert!(
            flow.signatures
                .iter()
                .any(|signature| signature.name == "caller")
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn c_io_rows_carry_ops_targets_and_literals() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-io-c-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "io.c",
            "int main(void) {\n\
                 FILE *in = fopen(\"cfg.ini\", \"r\");\n\
                 FILE *out = fopen(\"out.log\", \"w\");\n\
                 FILE *both = fopen(\"log.txt\", \"r+\");\n\
                 FILE *unknown = fopen(path, mode);\n\
                 FILE *re = freopen(\"freopen.log\", \"w\", stdout);\n\
                 int n = read(fd, buf, 4);\n\
                 FILE *wonly = open(\"w.log\", O_WRONLY);\n\
                 FILE *rw = open(\"rw.log\", O_RDWR);\n\
                 open(path, flags);\n\
                 printf(\"%d\\n\", n);\n\
                 sprintf(buf, \"%d\", n);\n\
                 const char *home = getenv(\"HOME\");\n\
                 setenv(\"MODE\", \"1\", 1);\n\
                 connect(fd, addr, len);\n\
                 return 0;\n\
             }\n",
        );

        let flow = flow_for(&dir, &["io.c"]);
        let rows: Vec<(&str, &str, &str, Option<&str>)> = flow
            .io
            .iter()
            .map(|row| {
                (
                    row.callee.as_str(),
                    row.op,
                    row.target,
                    row.literal.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            vec![
                ("fopen", "read", "file", Some("cfg.ini")),
                ("fopen", "write", "file", Some("out.log")),
                ("fopen", "read", "file", Some("log.txt")),
                ("fopen", "write", "file", Some("log.txt")),
                // freopen(path, mode, stream): mode is argument 1.
                ("freopen", "write", "file", Some("freopen.log")),
                ("read", "read", "file", None),
                ("open", "write", "file", Some("w.log")),
                ("open", "read", "file", Some("rw.log")),
                ("open", "write", "file", Some("rw.log")),
                // stdio: a format string is never a target.
                ("printf", "write", "stdio", None),
                ("getenv", "read", "env", Some("HOME")),
                ("setenv", "write", "env", Some("MODE")),
                ("connect", "write", "net", None),
            ],
            "non-literal modes/flags emit no row; `sprintf` is not I/O"
        );
        assert_eq!(flow.io[0].line, 2);
        assert_eq!(flow.io.last().map(|row| row.line), Some(15));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn c_open_numeric_flags_are_not_guessed() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-io-f-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "flags.c",
            "int main(void) {\n\
                 open(\"zero\", 0);\n\
                 open(\"one\", 1);\n\
                 open(\"two\", 2);\n\
                 open(\"hex\", 0x241);\n\
                 open(\"sym\", O_WRONLY | O_CREAT);\n\
                 open(\"rd\", O_RDONLY);\n\
                 open(\"rw\", O_RDWR);\n\
                 open(\"var\", flags);\n\
                 return 0;\n\
             }\n",
        );

        let flow = flow_for(&dir, &["flags.c"]);
        let rows: Vec<(&str, &str, Option<&str>)> = flow
            .io
            .iter()
            .map(|row| (row.callee.as_str(), row.op, row.literal.as_deref()))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("open", "read", Some("zero")),
                ("open", "write", Some("sym")),
                ("open", "read", Some("rd")),
                ("open", "read", Some("rw")),
                ("open", "write", Some("rw")),
            ],
            "only 0 and symbolic access flags are decoded; non-zero numeric bits and variables emit no row"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn c_io_never_classifies_wrappers_macros_or_formatted_targets() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-io-w-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "wrap.c",
            "#include <stdio.h>\n\
             #define fopen(path, mode) my_open(path, mode)\n\
             int read(int fd) { return fd; }\n\
             int main(void) {\n\
                 read(0);\n\
                 fopen(\"x\", \"r\");\n\
                 open(concat_path(), 0);\n\
                 return 0;\n\
             }\n",
        );

        let flow = flow_for(&dir, &["wrap.c"]);
        let rows: Vec<(&str, &str, Option<&str>)> = flow
            .io
            .iter()
            .map(|row| (row.callee.as_str(), row.op, row.literal.as_deref()))
            .collect();
        assert_eq!(
            rows,
            vec![("open", "read", None)],
            "a drawn `read` wrapper and a macro `fopen` are not I/O; a non-literal path stays unknown"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn c_typedef_function_pointer_is_not_io() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-io-fp-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "fp.c",
            "typedef int (*read_fn)(int);\n\
             int main(void) { read_fn read = 0; return read(0); }\n",
        );

        let flow = flow_for(&dir, &["fp.c"]);
        assert!(
            flow.io.is_empty(),
            "a typedef'd function-pointer local is not the library call: {:?}",
            flow.io
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rust_bare_names_are_not_c_io() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-io-rb-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(&dir, "bare.rs", "fn main() { read(0); write(1, 0, 1); }\n");

        let flow = flow_for(&dir, &["bare.rs"]);
        assert!(
            flow.io.is_empty(),
            "C bare-name rules are language-gated (Rust paths are exact): {:?}",
            flow.io
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rust_io_matches_exact_paths_and_rejects_formatted_targets() {
        let dir = std::env::temp_dir().join(format!("perfpal-flow-io-r-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        write(
            &dir,
            "io.rs",
            "use std::fs::File;\n\
             use std::env;\n\
             fn main() {\n\
                 let f = File::open(\"cfg.toml\");\n\
                 let out = File::create(\"/dev/null\");\n\
                 let home = env::var_os(\"HOME\");\n\
                 let text = std::fs::read_to_string(path);\n\
                 let _ = std::fs::write(\"out.txt\", b\"x\");\n\
                 let _ = File::open(format!(\"{}/x\", dir));\n\
                 let _ = File::open(concat!(\"a\", \"b\"));\n\
                 let _ = File::open(r#\"raw/place\"#);\n\
                 let _ = std::fs::copy(\"src\", \"dst\");\n\
             }\n",
        );

        let flow = flow_for(&dir, &["io.rs"]);
        let rows: Vec<(&str, &str, &str, Option<&str>)> = flow
            .io
            .iter()
            .map(|row| {
                (
                    row.callee.as_str(),
                    row.op,
                    row.target,
                    row.literal.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            vec![
                ("File::open", "read", "file", Some("cfg.toml")),
                ("File::create", "write", "file", Some("/dev/null")),
                ("env::var_os", "read", "env", Some("HOME")),
                ("std::fs::read_to_string", "read", "file", None),
                ("std::fs::write", "write", "file", Some("out.txt")),
                // `format!`/`concat!` are built values, never literal targets.
                ("File::open", "read", "file", None),
                ("File::open", "read", "file", None),
                // A raw string is still a plain written literal.
                ("File::open", "read", "file", Some("raw/place")),
                // copy reads the source and writes the destination; no single
                // literal is picked from the two path arguments.
                ("std::fs::copy", "read", "file", None),
                ("std::fs::copy", "write", "file", None),
            ]
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn io_table_names_are_unique() {
        let mut names: Vec<&str> = IO_RULES.iter().map(|rule| rule.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), IO_RULES.len());
        assert_eq!(IO_TABLE_VERSION, 1);
    }

    #[test]
    fn io_table_pins_direction_arguments() {
        let find = |name: &str| {
            IO_RULES
                .iter()
                .find(|rule| rule.name == name)
                .unwrap_or_else(|| panic!("missing rule {name}"))
        };
        assert!(matches!(find("fopen").op, OpRule::Mode(1)));
        assert!(matches!(find("freopen").op, OpRule::Mode(1)));
        assert!(matches!(find("open").op, OpRule::Flags(1)));
        assert!(matches!(find("openat").op, OpRule::Flags(2)));
        assert!(matches!(find("std::fs::copy").op, OpRule::Both));
        assert_eq!(find("rename").literal_arg, None);
        assert_eq!(find("std::fs::rename").literal_arg, None);
        assert_eq!(find("std::fs::copy").literal_arg, None);
    }
}
