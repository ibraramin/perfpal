//! Independent Rust reference parser for the Perfpal 0.5.0 ICC study.
//!
//! This binary is deliberately *not* built on Perfpal's tree-sitter backend:
//! it uses `syn` to parse each `.rs` file under `examples-rust/` and reports
//! the same function-level loop metrics as the scanner:
//!
//! * `loop_count`       — `for` / `while` / `loop` expressions in the body
//! * `loop_body_loc`    — size of the union of inclusive loop-body line spans
//! * `max_loop_depth`   — syntactic loop nesting depth
//! * `param_count`      — typed parameters (receivers are counted separately)
//! * `alloc_sites_in_loops` — allocation call/macro sites at loop depth >= 1
//!   (independent, name-based allocation family; no type resolution)
//!
//! Scope rules (matching Perfpal's Rust backend):
//! * inline `mod` bodies are walked and their `fn` items are emitted;
//! * loops inside closures are counted;
//! * a nested `fn` is a scope boundary — its loops are *not* counted in the
//!   enclosing function, and the nested `fn` is not emitted as a record;
//! * macro invocations are token trees and therefore opaque (documented
//!   limitation: loops/allocations written inside a macro expansion other than
//!   the invocation itself are invisible).
//!
//! Inputs: `examples-rust/` and `tools/icc/extra/` (`.rs` files).
//!
//! Output: `tools/icc/ref/rust.json`, deterministic and sorted by
//! `(path, start_line, start_col, name)`.  Paths are relative to the repo root,
//! `start_line` is 1-based and `start_col` is 0-based.
//!
//! Usage:
//!     cargo run --release --manifest-path tools/icc/rust_ref/Cargo.toml -- \
//!         --repo /home/shafin/Opencode/Perfpal

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Block, FnArg, Item, ItemFn};

#[derive(Debug, Serialize)]
struct FunctionRecord {
    path: String,
    name: String,
    start_line: usize,
    start_col: usize,
    param_count: usize,
    loop_count: usize,
    loop_body_loc: usize,
    max_loop_depth: usize,
    alloc_sites_in_loops: usize,
}

#[derive(Debug, Serialize)]
struct Output {
    language: &'static str,
    functions: Vec<FunctionRecord>,
}

#[derive(Default)]
struct LoopCollector {
    depth: usize,
    count: usize,
    max_depth: usize,
    spans: Vec<(usize, usize)>,
    alloc_sites: usize,
}

impl LoopCollector {
    fn enter(&mut self, span: (usize, usize)) {
        self.count += 1;
        self.depth += 1;
        if self.depth > self.max_depth {
            self.max_depth = self.depth;
        }
        self.spans.push(span);
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }
}

/// Inclusive line span of a loop body block, using the brace tokens so the
/// span matches Perfpal's convention (the `{` and `}` lines are included).
fn block_span(block: &Block) -> (usize, usize) {
    let open = block.brace_token.span.open().start();
    let close = block.brace_token.span.close().start();
    (open.line, close.line)
}

// Independent allocation-site vocabulary for `alloc_sites_in_loops`.
//
// This list was written from the Rust allocation idioms, deliberately not
// imported from Perfpal's `RUST_ALLOC_FAMILY` table. It is **name-based only**:
// there is no type resolution (HIR/MIR), so a user-defined `Vec::new` or a
// same-named method counts the same as the stdlib item. `clone` is excluded on
// purpose (ubiquitous, not necessarily heap-allocating).
const ALLOC_QUALIFIED_CALLS: &[&str] = &[
    "Vec::new",
    "Vec::with_capacity",
    "VecDeque::new",
    "VecDeque::with_capacity",
    "Box::new",
    "Rc::new",
    "Arc::new",
    "String::new",
    "String::from",
    "String::with_capacity",
    "HashMap::new",
    "HashMap::with_capacity",
    "HashSet::new",
    "HashSet::with_capacity",
    "BTreeMap::new",
    "BTreeSet::new",
    "CString::new",
    "OsString::new",
    "PathBuf::new",
];

const ALLOC_METHODS: &[&str] = &[
    "to_vec",
    "to_owned",
    "to_string",
    "collect",
    "with_capacity",
];

const ALLOC_MACROS: &[&str] = &["vec", "format"];

fn path_segments(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect()
}

fn qualified_alloc_call(path: &syn::Path) -> bool {
    let segments = path_segments(path);
    if segments.len() < 2 {
        return false;
    }
    let last_two = format!(
        "{}::{}",
        segments[segments.len() - 2],
        segments[segments.len() - 1]
    );
    ALLOC_QUALIFIED_CALLS.contains(&last_two.as_str())
}

impl<'ast> Visit<'ast> for LoopCollector {
    /// Stop at every nested item. A nested `fn` (also inside an `impl`, a
    /// trait, or a nested module) is a separate Perfpal scope; descending
    /// would wrongly attribute its loops to the enclosing function.
    fn visit_item(&mut self, _item: &'ast Item) {}

    // Loop headers are evaluated once, not per iteration: visit them at the
    // current depth, then enter the body. This keeps `alloc_sites_in_loops`
    // faithful to "inside the loop body" (and matches how nested loops in a
    // header should be counted).
    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.visit_expr(&node.expr);
        self.enter(block_span(&node.body));
        self.visit_block(&node.body);
        self.leave();
    }

    fn visit_expr_while(&mut self, node: &'ast syn::ExprWhile) {
        self.visit_expr(&node.cond);
        self.enter(block_span(&node.body));
        self.visit_block(&node.body);
        self.leave();
    }

    fn visit_expr_loop(&mut self, node: &'ast syn::ExprLoop) {
        self.enter(block_span(&node.body));
        self.visit_block(&node.body);
        self.leave();
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if self.depth >= 1 {
            if let syn::Expr::Path(function) = &*node.func {
                if qualified_alloc_call(&function.path) {
                    self.alloc_sites += 1;
                }
            }
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if self.depth >= 1 {
            let method = node.method.to_string();
            if ALLOC_METHODS.contains(&method.as_str()) {
                // `"literal".to_string()` / `.to_owned()` is foldable and
                // deliberately excluded (same accepted-FN class as Perfpal).
                let literal_receiver = matches!(
                    &*node.receiver,
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(_),
                        ..
                    })
                );
                let suppressed =
                    literal_receiver && (method == "to_string" || method == "to_owned");
                if !suppressed {
                    self.alloc_sites += 1;
                }
            }
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        if self.depth >= 1 {
            if let Some(segment) = node.mac.path.segments.last() {
                let name = segment.ident.to_string();
                if ALLOC_MACROS.contains(&name.as_str()) {
                    self.alloc_sites += 1;
                }
            }
        }
        visit::visit_expr_macro(self, node);
    }
}

fn analyze_function(item: &ItemFn, rel_path: &str) -> (FunctionRecord, bool) {
    let mut collector = LoopCollector::default();
    collector.visit_block(&item.block);

    let mut unique_lines: BTreeSet<usize> = BTreeSet::new();
    for (start, end) in &collector.spans {
        for line in *start..=*end {
            unique_lines.insert(line);
        }
    }

    let mut param_count = 0usize;
    let mut has_receiver = false;
    for arg in &item.sig.inputs {
        match arg {
            FnArg::Typed(_) => param_count += 1,
            FnArg::Receiver(_) => has_receiver = true,
        }
    }

    // `ItemFn::span()` includes leading doc-comment attributes, which Perfpal's
    // tree-sitter span does not. Anchor at the earliest item token instead:
    // the visibility modifier when present, otherwise the signature keyword
    // (`fn`, or `async`/`unsafe`/`const`/`extern` before it).
    let signature_start = item.sig.span().start();
    let start = match &item.vis {
        syn::Visibility::Inherited => signature_start,
        visibility => {
            let visibility_start = visibility.span().start();
            if (visibility_start.line, visibility_start.column)
                <= (signature_start.line, signature_start.column)
            {
                visibility_start
            } else {
                signature_start
            }
        }
    };
    let record = FunctionRecord {
        path: rel_path.to_string(),
        name: item.sig.ident.to_string(),
        start_line: start.line,
        start_col: start.column,
        param_count,
        loop_count: collector.count,
        loop_body_loc: unique_lines.len(),
        max_loop_depth: collector.max_depth,
        alloc_sites_in_loops: collector.alloc_sites,
    };
    (record, has_receiver)
}

fn walk_items(
    items: &[Item],
    rel_path: &str,
    out: &mut Vec<FunctionRecord>,
    receiver_functions: &mut Vec<String>,
) {
    for item in items {
        match item {
            Item::Fn(function) => {
                let (record, has_receiver) = analyze_function(function, rel_path);
                if has_receiver {
                    receiver_functions.push(format!("{}::{}", rel_path, record.name));
                }
                out.push(record);
            }
            Item::Mod(module) => {
                if let Some((_, nested)) = &module.content {
                    walk_items(nested, rel_path, out, receiver_functions);
                }
            }
            _ => {}
        }
    }
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                eprintln!("warning: cannot read {}: {error}", dir.display());
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn parse_args() -> (PathBuf, PathBuf) {
    let mut repo = env::current_dir().expect("current directory");
    let mut out: Option<PathBuf> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repo" => {
                repo = PathBuf::from(args.next().expect("--repo needs a value"));
            }
            "--out" => {
                out = Some(PathBuf::from(args.next().expect("--out needs a value")));
            }
            "-h" | "--help" => {
                println!(
                    "usage: rust_ref [--repo ROOT] [--out TOOLS/ICC/REF/RUST.JSON]\n\
                     Parses examples-rust/ and tools/icc/extra/ (*.rs) and writes the Rust reference metrics."
                );
                std::process::exit(0);
            }
            other => {
                eprintln!("error: unknown argument {other:?}");
                std::process::exit(2);
            }
        }
    }
    let repo = repo.canonicalize().unwrap_or(repo);
    let out = out.unwrap_or_else(|| repo.join("tools/icc/ref/rust.json"));
    (repo, out)
}

fn main() {
    let (repo, out_path) = parse_args();
    let corpus_roots = [repo.join("examples-rust"), repo.join("tools/icc/extra")];
    if !corpus_roots.iter().any(|root| root.is_dir()) {
        eprintln!(
            "error: no Rust corpus directory found (expected {} or {})",
            corpus_roots[0].display(),
            corpus_roots[1].display()
        );
        std::process::exit(2);
    }

    let mut functions = Vec::new();
    let mut receiver_functions = Vec::new();
    let mut parse_errors = Vec::new();
    let mut files = Vec::new();
    for root in &corpus_roots {
        if root.is_dir() {
            files.extend(rust_files(root));
        }
    }
    files.sort();

    for path in files {
        let rel_path = relative_path(&repo, &path);
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                parse_errors.push(format!("{rel_path}: {error}"));
                continue;
            }
        };
        match syn::parse_file(&source) {
            Ok(file) => walk_items(&file.items, &rel_path, &mut functions, &mut receiver_functions),
            Err(error) => parse_errors.push(format!("{rel_path}: {error}")),
        }
    }

    functions.sort_by(|a, b| {
        (a.path.as_str(), a.start_line, a.start_col, a.name.as_str()).cmp(&(
            b.path.as_str(),
            b.start_line,
            b.start_col,
            b.name.as_str(),
        ))
    });

    if let Some(parent) = out_path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!("error: cannot create {}: {error}", parent.display());
            std::process::exit(2);
        }
    }

    let output = Output {
        language: "rust",
        functions,
    };
    let mut json = serde_json::to_string_pretty(&output).expect("serialize output");
    json.push('\n');
    if let Err(error) = fs::write(&out_path, json) {
        eprintln!("error: cannot write {}: {error}", out_path.display());
        std::process::exit(2);
    }

    println!(
        "rust reference: {} functions, {} alloc sites in loops, {} parse errors -> {}",
        output.functions.len(),
        output
            .functions
            .iter()
            .map(|function| function.alloc_sites_in_loops)
            .sum::<usize>(),
        parse_errors.len(),
        out_path.display()
    );
    if !parse_errors.is_empty() {
        for error in &parse_errors {
            eprintln!("parse error: {error}");
        }
        std::process::exit(1);
    }
    if !receiver_functions.is_empty() {
        println!(
            "note: {} function(s) have an explicit receiver: {}",
            receiver_functions.len(),
            receiver_functions.join(", ")
        );
    }
}
