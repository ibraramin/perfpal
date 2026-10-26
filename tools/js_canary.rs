//! W0 pre-freeze frequency canary for the 12 candidate `PERF-JS-*` rules.
//!
//! Invocation:
//!
//! ```text
//! cargo run --locked --quiet --example js_canary -- <ROOT_DIR>
//! ```
//!
//! Emits one deterministic JSON document to stdout. The only non-zero exit is an
//! unreadable root directory (exit code 2). No timestamps, no absolute paths.
//!
//! This is a *shape counter*, not the real analyzer: it answers "how often does
//! this construct occur in real JS/TS?" so implausible rules die before
//! implementation. See `docs/evidence/js-canary.md` for the method, corpus,
//! limitations and per-rule dispositions.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use tree_sitter::{Language, Node, Parser};

const RULE_IDS: [&str; 12] = [
    "PERF-JS-ALLOC-001",
    "PERF-JS-ALLOC-003",
    "PERF-JS-STR-001",
    "PERF-JS-LOOP-001",
    "PERF-JS-LOOP-002",
    "PERF-JS-LOOP-003",
    "PERF-JS-LOOP-007",
    "PERF-JS-ASYNC-001",
    "PERF-JS-ASYNC-002",
    "PERF-JS-CALL-001",
    "PERF-JS-ARCH-001",
    "PERF-JS-TYPE-001",
];

const CONTROL_IDS: [&str; 12] = [
    "CTL-ALLOC-NOLOOP",
    "CTL-ALLOC-NOLOOP-MUT",
    "CTL-INT-ACC",
    "CTL-STR-NOLOOP",
    "CTL-DEPTH1",
    "CTL-IO-NOLOOP",
    "CTL-FORIN-OBJ",
    "CTL-MEMBERSHIP-NOLOOP",
    "CTL-AWAIT-NOLOOP",
    "CTL-SYNC-SYNC-FN",
    "CTL-ACYCLIC",
    "CTL-DELETE-MAP",
];

const BREAKDOWN_IDS: [&str; 10] = [
    "arch_internal_edges",
    "arch_modules",
    "arch_modules_acyclic",
    "arch_modules_in_cycle",
    "await_for_of",
    "call001_guarded",
    "call001_unguarded",
    "forin_array",
    "forin_object",
    "forin_other",
];

const SKIP_DIRS: [&str; 11] = [
    "node_modules",
    "bower_components",
    "dist",
    "build",
    "out",
    "coverage",
    ".git",
    ".next",
    ".nuxt",
    ".svelte-kit",
    ".cache",
];

const SKIP_FILE_SUFFIXES: [&str; 7] = [
    ".min.js",
    ".min.mjs",
    ".min.cjs",
    ".bundle.js",
    ".d.ts",
    ".d.mts",
    ".d.cts",
];

const JS_EXTS: [&str; 4] = ["js", "jsx", "mjs", "cjs"];
const TS_EXTS: [&str; 3] = ["ts", "mts", "cts"];

const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

const RESOLVE_EXTS: [&str; 8] = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts"];

const SYNC_CALLS: [&str; 15] = [
    "readFileSync",
    "writeFileSync",
    "appendFileSync",
    "readdirSync",
    "statSync",
    "lstatSync",
    "existsSync",
    "mkdirSync",
    "rmSync",
    "unlinkSync",
    "openSync",
    "closeSync",
    "execSync",
    "execFileSync",
    "spawnSync",
];

const MEMBERSHIP_METHODS: [&str; 5] = ["indexOf", "includes", "find", "findIndex", "lastIndexOf"];

type KeyMap = BTreeMap<&'static str, u64>;

#[derive(Clone, Copy, Default, Serialize)]
struct Pair {
    prod: u64,
    test: u64,
}

type PairMap = BTreeMap<&'static str, Pair>;

fn seed(ids: &[&'static str]) -> KeyMap {
    ids.iter().map(|&id| (id, 0)).collect()
}

fn seed_pairs(ids: &[&'static str]) -> PairMap {
    ids.iter().map(|&id| (id, Pair::default())).collect()
}

fn bump(map: &mut KeyMap, key: &'static str) {
    *map.get_mut(key).expect("id is pre-seeded") += 1;
}

fn bump_pair(map: &mut PairMap, key: &'static str, is_test: bool) {
    let pair = map.get_mut(key).expect("id is pre-seeded");
    if is_test {
        pair.test += 1;
    } else {
        pair.prod += 1;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoopKind {
    For,
    ForOf,
    ForIn,
    While,
    Do,
}

#[derive(Clone, Copy)]
struct Ctx {
    loop_depth: u32,
    innermost: Option<LoopKind>,
    in_function: bool,
    async_fn: bool,
}

impl Ctx {
    const TOP: Ctx = Ctx {
        loop_depth: 0,
        innermost: None,
        in_function: false,
        async_fn: false,
    };
}

struct FileCounts {
    rules: KeyMap,
    controls: KeyMap,
    breakdown: KeyMap,
    imports: Vec<String>,
}

struct Analyzer<'a> {
    src: &'a [u8],
    rules: KeyMap,
    controls: KeyMap,
    breakdown: KeyMap,
    imports: Vec<String>,
}

impl<'a> Analyzer<'a> {
    fn new(src: &'a [u8]) -> Self {
        Self {
            src,
            rules: seed(&RULE_IDS),
            controls: seed(&CONTROL_IDS),
            breakdown: seed(&BREAKDOWN_IDS),
            imports: Vec::new(),
        }
    }

    fn finish(self) -> FileCounts {
        FileCounts {
            rules: self.rules,
            controls: self.controls,
            breakdown: self.breakdown,
            imports: self.imports,
        }
    }

    fn run(mut self, root: Node<'_>) -> FileCounts {
        self.walk(root, Ctx::TOP);
        self.scan_functions(root);
        self.finish()
    }

    /// Main walk: rules that depend on loop/function context.
    fn walk(&mut self, node: Node<'_>, ctx: Ctx) {
        if is_function(node) {
            // A nested function body executes on its own schedule, not once per
            // enclosing iteration: reset loop context (documented limitation).
            let inner = Ctx {
                loop_depth: 0,
                innermost: None,
                in_function: true,
                async_fn: has_async_token(node),
            };
            for i in 0..node.child_count() as u32 {
                if let Some(child) = node.child(i) {
                    self.walk(child, inner);
                }
            }
            return;
        }

        if is_loop(node) {
            self.eval_loop(node);
            let body_range = node
                .child_by_field_name("body")
                .map(|b| (b.start_byte(), b.end_byte()));
            for i in 0..node.child_count() as u32 {
                let Some(child) = node.child(i) else { continue };
                if Some((child.start_byte(), child.end_byte())) == body_range {
                    continue;
                }
                // Loop headers run in the *enclosing* context.
                self.walk(child, ctx);
            }
            if let Some(body) = node.child_by_field_name("body") {
                let kind = loop_kind(node);
                self.walk(
                    body,
                    Ctx {
                        loop_depth: ctx.loop_depth + 1,
                        innermost: Some(kind),
                        ..ctx
                    },
                );
            }
            return;
        }

        self.eval(node, ctx);
        for i in 0..node.child_count() as u32 {
            if let Some(child) = node.child(i) {
                self.walk(child, ctx);
            }
        }
    }

    fn eval(&mut self, node: Node<'_>, ctx: Ctx) {
        let src = self.src;
        match node.kind() {
            "object" | "array" => {
                if ctx.loop_depth > 0 {
                    bump(&mut self.rules, "PERF-JS-ALLOC-001");
                } else {
                    bump(&mut self.controls, "CTL-ALLOC-NOLOOP");
                }
            }
            "assignment_expression" => {
                self.check_alloc_003(node, ctx);
                self.check_str_assignment(node, ctx);
            }
            "augmented_assignment_expression" => self.check_str_augmented(node, ctx),
            "call_expression" => {
                self.check_sync_call(node, ctx);
                self.check_membership(node, ctx);
                self.check_delete_map(node);
                self.collect_require(node);
            }
            "await_expression" => {
                if ctx.loop_depth > 0 {
                    bump(&mut self.rules, "PERF-JS-ASYNC-001");
                    if ctx.innermost == Some(LoopKind::ForOf) {
                        bump(&mut self.breakdown, "await_for_of");
                    }
                } else {
                    bump(&mut self.controls, "CTL-AWAIT-NOLOOP");
                }
            }
            "unary_expression" => {
                if text(src, node.child_by_field_name("operator").unwrap_or(node)) == "delete"
                    && node
                        .child_by_field_name("argument")
                        .is_some_and(|a| a.kind() == "member_expression")
                {
                    bump(&mut self.rules, "PERF-JS-TYPE-001");
                }
            }
            "import_statement" => {
                if let Some(source) = node.child_by_field_name("source") {
                    self.push_spec(text(src, source));
                }
            }
            "export_statement" => {
                if let Some(source) = node.child_by_field_name("source") {
                    self.push_spec(text(src, source));
                }
            }
            _ => {}
        }
    }

    /// `PERF-JS-LOOP-003` / `CTL-FORIN-OBJ`: only `for...in` (`of` is a different
    /// loop). RHS shape is split into array / object / identifier-or-other; an
    /// object literal is the idiomatic, silent control.
    fn eval_loop(&mut self, node: Node<'_>) {
        if node.kind() != "for_in_statement" {
            return;
        }
        let src = self.src;
        let operator = node
            .child_by_field_name("operator")
            .map(|n| text(src, n))
            .unwrap_or("");
        if operator != "in" {
            return;
        }
        let right = node.child_by_field_name("right");
        let shape = right.map(|r| r.kind()).unwrap_or("");
        match shape {
            "array" => {
                bump(&mut self.breakdown, "forin_array");
                bump(&mut self.rules, "PERF-JS-LOOP-003");
            }
            "object" => {
                bump(&mut self.breakdown, "forin_object");
                bump(&mut self.controls, "CTL-FORIN-OBJ");
            }
            _ => {
                bump(&mut self.breakdown, "forin_other");
                bump(&mut self.rules, "PERF-JS-LOOP-003");
            }
        }
    }

    fn check_alloc_003(&mut self, node: Node<'_>, ctx: Ctx) {
        let src = self.src;
        let (Some(left), Some(right)) = (
            node.child_by_field_name("left"),
            node.child_by_field_name("right"),
        ) else {
            return;
        };
        let left_text = text(src, left);
        let mut matched = false;

        if right.kind() == "call_expression"
            && let Some(function) = right.child_by_field_name("function")
            && function.kind() == "member_expression"
            && let Some(property) = function.child_by_field_name("property")
        {
            let method = text(src, property);
            if (method == "concat" || method == "slice")
                && function
                    .child_by_field_name("object")
                    .is_some_and(|o| text(src, o) == left_text)
            {
                matched = true;
            }
        }

        if right.kind() == "array"
            && (0..right.named_child_count() as u32).any(|i| {
                let Some(element) = right.named_child(i) else {
                    return false;
                };
                element.kind() == "spread_element"
                    && element
                        .named_child(0)
                        .is_some_and(|inner| text(src, inner) == left_text)
            })
        {
            matched = true;
        }

        if matched {
            if ctx.loop_depth > 0 {
                bump(&mut self.rules, "PERF-JS-ALLOC-003");
            } else {
                bump(&mut self.controls, "CTL-ALLOC-NOLOOP-MUT");
            }
        }
    }

    fn check_str_augmented(&mut self, node: Node<'_>, ctx: Ctx) {
        let src = self.src;
        let operator = node
            .child_by_field_name("operator")
            .map(|n| text(src, n))
            .unwrap_or("");
        if operator != "+=" {
            return;
        }
        let Some(right) = node.child_by_field_name("right") else {
            return;
        };
        match right.kind() {
            "string" | "template_string" => {
                if ctx.loop_depth > 0 {
                    bump(&mut self.rules, "PERF-JS-STR-001");
                } else {
                    bump(&mut self.controls, "CTL-STR-NOLOOP");
                }
            }
            "number" => {
                if ctx.loop_depth > 0 {
                    bump(&mut self.controls, "CTL-INT-ACC");
                }
            }
            _ => {}
        }
    }

    /// `x = x + <string>` (the non-`+=` accumulation shape).
    fn check_str_assignment(&mut self, node: Node<'_>, ctx: Ctx) {
        let src = self.src;
        let (Some(left), Some(right)) = (
            node.child_by_field_name("left"),
            node.child_by_field_name("right"),
        ) else {
            return;
        };
        if right.kind() != "binary_expression" {
            return;
        }
        let operator = right
            .child_by_field_name("operator")
            .map(|n| text(src, n))
            .unwrap_or("");
        if operator != "+" {
            return;
        }
        let (Some(bin_left), Some(bin_right)) = (
            right.child_by_field_name("left"),
            right.child_by_field_name("right"),
        ) else {
            return;
        };
        let right_is_str = matches!(bin_right.kind(), "string" | "template_string");
        if !right_is_str || text(src, bin_left) != text(src, left) {
            return;
        }
        if ctx.loop_depth > 0 {
            bump(&mut self.rules, "PERF-JS-STR-001");
        } else {
            bump(&mut self.controls, "CTL-STR-NOLOOP");
        }
    }

    fn check_sync_call(&mut self, node: Node<'_>, ctx: Ctx) {
        let src = self.src;
        let Some(function) = node.child_by_field_name("function") else {
            return;
        };
        let sync = match function.kind() {
            "identifier" => SYNC_CALLS.contains(&text(src, function)),
            "member_expression" => {
                let Some(property) = function.child_by_field_name("property") else {
                    return;
                };
                let name = text(src, property);
                if !name.ends_with("Sync") {
                    return;
                }
                let Some(object) = function.child_by_field_name("object") else {
                    return;
                };
                matches!(
                    root_name(src, object).as_deref(),
                    Some("fs") | Some("crypto") | Some("child_process")
                )
            }
            _ => false,
        };
        if !sync {
            return;
        }

        if ctx.loop_depth > 0 {
            bump(&mut self.rules, "PERF-JS-LOOP-002");
        } else {
            bump(&mut self.controls, "CTL-IO-NOLOOP");
        }
        if ctx.in_function {
            if ctx.async_fn {
                bump(&mut self.rules, "PERF-JS-ASYNC-002");
            } else {
                bump(&mut self.controls, "CTL-SYNC-SYNC-FN");
            }
        }
    }

    fn check_membership(&mut self, node: Node<'_>, ctx: Ctx) {
        let src = self.src;
        let Some(function) = node.child_by_field_name("function") else {
            return;
        };
        if function.kind() != "member_expression" {
            return;
        }
        let Some(property) = function.child_by_field_name("property") else {
            return;
        };
        if !MEMBERSHIP_METHODS.contains(&text(src, property)) {
            return;
        }
        if ctx.loop_depth > 0 {
            bump(&mut self.rules, "PERF-JS-LOOP-007");
        } else {
            bump(&mut self.controls, "CTL-MEMBERSHIP-NOLOOP");
        }
    }

    fn check_delete_map(&mut self, node: Node<'_>) {
        let src = self.src;
        let Some(function) = node.child_by_field_name("function") else {
            return;
        };
        if function.kind() != "member_expression" {
            return;
        }
        if function
            .child_by_field_name("property")
            .is_some_and(|p| text(src, p) == "delete")
        {
            bump(&mut self.controls, "CTL-DELETE-MAP");
        }
    }

    fn collect_require(&mut self, node: Node<'_>) {
        let src = self.src;
        let Some(function) = node.child_by_field_name("function") else {
            return;
        };
        if function.kind() != "identifier" || text(src, function) != "require" {
            return;
        }
        let Some(arguments) = node.child_by_field_name("arguments") else {
            return;
        };
        if arguments.named_child_count() != 1 {
            return;
        }
        if let Some(argument) = arguments.named_child(0)
            && argument.kind() == "string"
        {
            self.push_spec(text(src, argument));
        }
    }

    fn push_spec(&mut self, literal: &str) {
        if let Some(spec) = unquote(literal)
            && (spec.starts_with("./") || spec.starts_with("../"))
        {
            self.imports.push(spec.to_string());
        }
    }

    /// Per-function rules (`LOOP-001`, `CALL-001`) need their own scope: a
    /// function's loop nest depth and self-calls ignore nested function bodies.
    fn scan_functions(&mut self, node: Node<'_>) {
        if is_function(node) {
            self.function_rules(node);
        }
        for i in 0..node.child_count() as u32 {
            if let Some(child) = node.child(i) {
                self.scan_functions(child);
            }
        }
    }

    fn function_rules(&mut self, node: Node<'_>) {
        let Some(body) = node.child_by_field_name("body") else {
            return;
        };
        let mut loops = 0u32;
        let mut max_depth = 0u32;
        count_loops(body, 0, &mut loops, &mut max_depth);
        if max_depth >= 2 {
            bump(&mut self.rules, "PERF-JS-LOOP-001");
        } else if loops > 0 {
            bump(&mut self.controls, "CTL-DEPTH1");
        }

        let Some(name) = derive_name(node, self.src) else {
            return;
        };
        let is_method = node.kind() == "method_definition";
        let guarded = body_has_guard(body);
        let mut calls = 0u64;
        count_self_calls(body, &name, is_method, self.src, &mut calls);
        if calls == 0 {
            return;
        }
        *self.rules.get_mut("PERF-JS-CALL-001").expect("seeded") += calls;
        let key = if guarded {
            "call001_guarded"
        } else {
            "call001_unguarded"
        };
        *self.breakdown.get_mut(key).expect("seeded") += calls;
    }
}

fn is_function(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "function_declaration"
            | "function_expression"
            | "arrow_function"
            | "method_definition"
            | "generator_function_declaration"
            | "generator_function"
    )
}

fn is_loop(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "for_statement" | "for_in_statement" | "while_statement" | "do_statement"
    )
}

fn loop_kind(node: Node<'_>) -> LoopKind {
    match node.kind() {
        "for_statement" => LoopKind::For,
        "for_in_statement" => {
            // `for...of` and `for...in` share one node kind; the `of`/`in`
            // operator token disambiguates them.
            match node.child_by_field_name("operator").map(|n| n.kind()) {
                Some("of") => LoopKind::ForOf,
                _ => LoopKind::ForIn,
            }
        }
        "while_statement" => LoopKind::While,
        _ => LoopKind::Do,
    }
}

fn has_async_token(node: Node<'_>) -> bool {
    (0..node.child_count() as u32).any(|i| node.child(i).is_some_and(|c| c.kind() == "async"))
}

fn text<'a>(src: &'a [u8], node: Node<'_>) -> &'a str {
    node.utf8_text(src).unwrap_or("")
}

fn unquote(literal: &str) -> Option<&str> {
    let bytes = literal.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' || first == b'\'') && first == last {
            return Some(&literal[1..literal.len() - 1]);
        }
    }
    None
}

fn root_name(src: &[u8], node: Node<'_>) -> Option<String> {
    match node.kind() {
        "identifier" => Some(text(src, node).to_string()),
        "member_expression" => root_name(src, node.child_by_field_name("object")?),
        _ => None,
    }
}

fn body_has_guard(body: Node<'_>) -> bool {
    if matches!(body.kind(), "if_statement" | "return_statement") {
        return true;
    }
    for i in 0..body.child_count() as u32 {
        let Some(child) = body.child(i) else { continue };
        if is_function(child) {
            continue;
        }
        if body_has_guard(child) {
            return true;
        }
    }
    false
}

fn count_loops(node: Node<'_>, depth: u32, loops: &mut u32, max_depth: &mut u32) {
    for i in 0..node.child_count() as u32 {
        let Some(child) = node.child(i) else { continue };
        if is_function(child) {
            continue;
        }
        if is_loop(child) {
            *loops += 1;
            let next = depth + 1;
            *max_depth = (*max_depth).max(next);
            count_loops(child, next, loops, max_depth);
        } else {
            count_loops(child, depth, loops, max_depth);
        }
    }
}

fn count_self_calls(node: Node<'_>, name: &str, is_method: bool, src: &[u8], count: &mut u64) {
    for i in 0..node.child_count() as u32 {
        let Some(child) = node.child(i) else { continue };
        if is_function(child) {
            continue;
        }
        if child.kind() == "call_expression" && callee_is_self(child, name, is_method, src) {
            *count += 1;
        }
        count_self_calls(child, name, is_method, src, count);
    }
}

fn callee_is_self(call: Node<'_>, name: &str, is_method: bool, src: &[u8]) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    match function.kind() {
        "identifier" => text(src, function) == name,
        "member_expression" if is_method => {
            let this_receiver = function
                .child_by_field_name("object")
                .is_some_and(|o| o.kind() == "this");
            let property_matches = function
                .child_by_field_name("property")
                .is_some_and(|p| text(src, p) == name);
            this_receiver && property_matches
        }
        _ => false,
    }
}

fn derive_name(node: Node<'_>, src: &[u8]) -> Option<String> {
    if let Some(name) = node.child_by_field_name("name")
        && matches!(
            name.kind(),
            "identifier" | "property_identifier" | "private_property_identifier"
        )
    {
        let value = text(src, name);
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    let parent = node.parent()?;
    match parent.kind() {
        "variable_declarator" | "public_field_definition" => {
            let name = parent.child_by_field_name("name")?;
            if name.kind() == "identifier" {
                return Some(text(src, name).to_string());
            }
        }
        "assignment_expression" => {
            let left = parent.child_by_field_name("left")?;
            if left.kind() == "identifier" {
                return Some(text(src, left).to_string());
            }
        }
        "pair" => {
            let key = parent.child_by_field_name("key")?;
            if matches!(key.kind(), "identifier" | "property_identifier") {
                return Some(text(src, key).to_string());
            }
        }
        _ => {}
    }
    None
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_dir(root, &mut files);
    files.sort();
    files
}

fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<fs::DirEntry> = read_dir.filter_map(Result::ok).collect();
    // Deterministic mutation-free ordering: sort raw file names.
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if file_type.is_dir() {
            if name.starts_with('.') || SKIP_DIRS.contains(&name.as_ref()) {
                continue;
            }
            walk_dir(&entry.path(), files);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !JS_EXTS.contains(&ext) && !TS_EXTS.contains(&ext) && ext != "tsx" {
            continue;
        }
        if SKIP_FILE_SUFFIXES
            .iter()
            .any(|suffix| name.ends_with(*suffix))
        {
            continue;
        }
        if entry.metadata().is_ok_and(|m| m.len() > MAX_FILE_BYTES) {
            continue;
        }
        files.push(path);
    }
}

fn is_test_path(relative: &str) -> bool {
    let mut segments = relative.split('/').peekable();
    while let Some(segment) = segments.next() {
        let is_last = segments.peek().is_none();
        if !is_last
            && matches!(
                segment,
                "tests" | "test" | "__tests__" | "__mocks__" | "spec" | "e2e"
            )
        {
            return true;
        }
        if is_last && (segment.contains(".test.") || segment.contains(".spec.")) {
            return true;
        }
    }
    false
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn count_lines(bytes: &[u8]) -> u64 {
    if bytes.is_empty() {
        return 0;
    }
    let newlines = bytes.iter().filter(|&&b| b == b'\n').count() as u64;
    if bytes.ends_with(b"\n") {
        newlines
    } else {
        newlines + 1
    }
}

// ---------------------------------------------------------------------------
// Import graph / SCC
// ---------------------------------------------------------------------------

fn resolve_specifier(spec: &str, importer: &str, files: &BTreeSet<String>) -> Option<String> {
    let mut components: Vec<&str> = importer
        .rsplit_once('/')
        .map(|(dir, _)| dir.split('/').filter(|s| !s.is_empty()).collect())
        .unwrap_or_default();
    for part in spec.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                components.pop()?;
            }
            other => components.push(other),
        }
    }
    let base = components.join("/");
    if base.is_empty() {
        return None;
    }
    if files.contains(&base) {
        return Some(base);
    }
    for ext in RESOLVE_EXTS {
        let candidate = format!("{base}{ext}");
        if files.contains(&candidate) {
            return Some(candidate);
        }
    }
    for ext in RESOLVE_EXTS {
        let candidate = format!("{base}/index{ext}");
        if files.contains(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// SCC component sizes via iterative Kosaraju. Nodes and edges are visited in
/// sorted order, so component ids are deterministic.
fn cyclic_modules(nodes: &[String], edges: &BTreeSet<(String, String)>) -> BTreeSet<String> {
    let index: BTreeMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.as_str(), i))
        .collect();
    let mut adjacency = vec![Vec::new(); nodes.len()];
    let mut reverse = vec![Vec::new(); nodes.len()];
    for (from, to) in edges {
        if let (Some(&a), Some(&b)) = (index.get(from.as_str()), index.get(to.as_str())) {
            adjacency[a].push(b);
            reverse[b].push(a);
        }
    }

    let mut visited = vec![false; nodes.len()];
    let mut order = Vec::with_capacity(nodes.len());
    for start in 0..nodes.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut stack = vec![(start, 0usize)];
        while let Some((node, next_child)) = stack.pop() {
            if next_child < adjacency[node].len() {
                stack.push((node, next_child + 1));
                let child = adjacency[node][next_child];
                if !visited[child] {
                    visited[child] = true;
                    stack.push((child, 0));
                }
            } else {
                order.push(node);
            }
        }
    }

    let mut component = vec![usize::MAX; nodes.len()];
    let mut sizes: Vec<usize> = Vec::new();
    for &start in order.iter().rev() {
        if component[start] != usize::MAX {
            continue;
        }
        let id = sizes.len();
        let mut size = 0usize;
        component[start] = id;
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            size += 1;
            for &child in &reverse[node] {
                if component[child] == usize::MAX {
                    component[child] = id;
                    stack.push(child);
                }
            }
        }
        sizes.push(size);
    }

    nodes
        .iter()
        .enumerate()
        .filter(|(i, _)| sizes[component[*i]] > 1)
        .map(|(_, node)| node.clone())
        .collect()
}

// ---------------------------------------------------------------------------
// Per-repo accumulation
// ---------------------------------------------------------------------------

#[derive(Default)]
struct RepoAccum {
    prod_files: u64,
    test_files: u64,
    prod_lines: u64,
    test_lines: u64,
    parse_failures: u64,
    rules_prod: KeyMap,
    rules_test: KeyMap,
    controls_prod: KeyMap,
    controls_test: KeyMap,
    breakdown: PairMap,
}

impl RepoAccum {
    fn seeded() -> Self {
        Self {
            rules_prod: seed(&RULE_IDS),
            rules_test: seed(&RULE_IDS),
            controls_prod: seed(&CONTROL_IDS),
            controls_test: seed(&CONTROL_IDS),
            breakdown: seed_pairs(&BREAKDOWN_IDS),
            ..Self::default()
        }
    }

    fn merge_file(&mut self, counts: FileCounts, is_test: bool) {
        let (rules, controls) = if is_test {
            (&mut self.rules_test, &mut self.controls_test)
        } else {
            (&mut self.rules_prod, &mut self.controls_prod)
        };
        for (id, value) in counts.rules {
            *rules.get_mut(id).expect("seeded") += value;
        }
        for (id, value) in counts.controls {
            *controls.get_mut(id).expect("seeded") += value;
        }
        for (id, value) in counts.breakdown {
            if value > 0 {
                let pair = self.breakdown.get_mut(id).expect("seeded");
                if is_test {
                    pair.test += value;
                } else {
                    pair.prod += value;
                }
            }
        }
    }

    fn merge_graph(&mut self, modules: &[ModuleInfo], edges: &BTreeSet<(String, String)>) {
        let nodes: Vec<String> = modules.iter().map(|m| m.rel.clone()).collect();
        let cyclic = cyclic_modules(&nodes, edges);

        for module in modules {
            bump_pair(&mut self.breakdown, "arch_modules", module.is_test);
            if cyclic.contains(&module.rel) {
                bump_pair(&mut self.breakdown, "arch_modules_in_cycle", module.is_test);
                let rules = if module.is_test {
                    &mut self.rules_test
                } else {
                    &mut self.rules_prod
                };
                *rules.get_mut("PERF-JS-ARCH-001").expect("seeded") += 1;
            } else {
                bump_pair(&mut self.breakdown, "arch_modules_acyclic", module.is_test);
                let controls = if module.is_test {
                    &mut self.controls_test
                } else {
                    &mut self.controls_prod
                };
                *controls.get_mut("CTL-ACYCLIC").expect("seeded") += 1;
            }
        }

        let test_set: BTreeSet<&str> = modules
            .iter()
            .filter(|m| m.is_test)
            .map(|m| m.rel.as_str())
            .collect();
        for (from, _) in edges {
            let is_test = test_set.contains(from.as_str());
            bump_pair(&mut self.breakdown, "arch_internal_edges", is_test);
        }
    }

    fn add(&mut self, other: &RepoAccum) {
        self.prod_files += other.prod_files;
        self.test_files += other.test_files;
        self.prod_lines += other.prod_lines;
        self.test_lines += other.test_lines;
        self.parse_failures += other.parse_failures;
        add_map(&mut self.rules_prod, &other.rules_prod);
        add_map(&mut self.rules_test, &other.rules_test);
        add_map(&mut self.controls_prod, &other.controls_prod);
        add_map(&mut self.controls_test, &other.controls_test);
        for (id, pair) in &other.breakdown {
            let target = self.breakdown.get_mut(id).expect("seeded");
            target.prod += pair.prod;
            target.test += pair.test;
        }
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "prod_files": self.prod_files,
            "test_files": self.test_files,
            "prod_lines": self.prod_lines,
            "test_lines": self.test_lines,
            "parse_failures": self.parse_failures,
            "rules_prod": self.rules_prod,
            "rules_test": self.rules_test,
            "controls_prod": self.controls_prod,
            "controls_test": self.controls_test,
            "breakdown": self.breakdown,
        })
    }
}

fn add_map(target: &mut KeyMap, other: &KeyMap) {
    for (id, value) in other {
        *target.get_mut(id).expect("seeded") += value;
    }
}

struct ModuleInfo {
    rel: String,
    is_test: bool,
}

// ---------------------------------------------------------------------------
// Driver
// ---------------------------------------------------------------------------

fn git_field(root: &Path, args: &[&str]) -> String {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn parser_for(language: Language) -> Parser {
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        eprintln!("js_canary: pinned grammar rejected by the parser runtime");
        exit(2);
    }
    parser
}

fn analyze_root(root: &Path, parsers: &mut Parsers) -> RepoAccum {
    let canonical = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let files = collect_files(&canonical);
    let mut accum = RepoAccum::seeded();
    let mut modules: Vec<ModuleInfo> = Vec::new();
    let mut module_imports: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for path in files {
        let relative = relative_path(&canonical, &path);
        let is_test = is_test_path(&relative);
        let Ok(source) = fs::read(&path) else {
            continue;
        };
        let lines = count_lines(&source);

        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let parser = if JS_EXTS.contains(&extension) {
            &mut parsers.js
        } else if TS_EXTS.contains(&extension) {
            &mut parsers.ts
        } else {
            &mut parsers.tsx
        };

        if is_test {
            accum.test_files += 1;
            accum.test_lines += lines;
        } else {
            accum.prod_files += 1;
            accum.prod_lines += lines;
        }

        let Some(tree) = parser.parse(&source, None) else {
            accum.parse_failures += 1;
            continue;
        };
        if tree.root_node().has_error() {
            accum.parse_failures += 1;
        }

        let counts = Analyzer::new(&source).run(tree.root_node());
        for spec in &counts.imports {
            module_imports
                .entry(relative.clone())
                .or_default()
                .push(spec.clone());
        }
        accum.merge_file(counts, is_test);
        modules.push(ModuleInfo {
            rel: relative,
            is_test,
        });
    }

    // Resolve the import graph over the scanned module set only.
    let file_set: BTreeSet<String> = modules.iter().map(|m| m.rel.clone()).collect();
    let mut edges: BTreeSet<(String, String)> = BTreeSet::new();
    for (importer, specs) in &module_imports {
        for spec in specs {
            if let Some(target) = resolve_specifier(spec, importer, &file_set)
                && target != *importer
            {
                edges.insert((importer.clone(), target));
            }
        }
    }
    accum.merge_graph(&modules, &edges);
    accum
}

struct Parsers {
    js: Parser,
    ts: Parser,
    tsx: Parser,
}

fn main() {
    let root_args: Vec<String> = std::env::args().skip(1).collect();
    if root_args.is_empty() {
        eprintln!("usage: js_canary <ROOT_DIR>...");
        exit(2);
    }

    // Fail before doing work if any root is unreadable (the only non-zero exit).
    let mut roots = Vec::with_capacity(root_args.len());
    for argument in &root_args {
        let root = PathBuf::from(argument);
        if fs::read_dir(&root).is_err() {
            eprintln!("js_canary: root is not a readable directory: {argument}");
            exit(2);
        }
        roots.push(root);
    }

    let mut parsers = Parsers {
        js: parser_for(tree_sitter_javascript::LANGUAGE.into()),
        ts: parser_for(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        tsx: parser_for(tree_sitter_typescript::LANGUAGE_TSX.into()),
    };

    let mut corpus = serde_json::Map::new();
    let mut repos = serde_json::Map::new();
    let mut totals = RepoAccum::seeded();

    for root in &roots {
        let canonical = root.canonicalize().unwrap_or_else(|_| root.clone());
        let repo_name = canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "corpus".to_string());
        let accum = analyze_root(root, &mut parsers);

        corpus.insert(
            repo_name.clone(),
            serde_json::json!({
                "tag": git_field(&canonical, &["describe", "--tags", "--exact-match", "HEAD"]),
                "commit": git_field(&canonical, &["rev-parse", "HEAD"]),
                "prod_files": accum.prod_files,
                "test_files": accum.test_files,
                "prod_lines": accum.prod_lines,
                "test_lines": accum.test_lines,
                "parse_failures": accum.parse_failures,
            }),
        );
        repos.insert(repo_name, accum.to_json());
        totals.add(&accum);
    }

    let document = serde_json::json!({
        "corpus": corpus,
        "repos": repos,
        "totals": totals.to_json(),
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&document).expect("JSON serialization")
    );
}
