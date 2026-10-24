//! `scanner` — discovery, parsing, IR emission (modules.md §3.2).
//!
//! Consumes: target path; invocation options. Produces: the complete IR stream
//! ([`FileUnit`] records — file-level records, FunctionRecords, events per
//! ir-v1.md §3) in deterministic order.
//!
//! Responsibilities:
//! - Recursive discovery, lexicographically sorted emission; dot-dirs skipped;
//!   symlinks not followed (v1); per-file failures become `ParseHealth{failed}`
//!   records, never run aborts (total target failure = [`ScanError`], which
//!   cli maps to exit 1).
//! - Language detection by extension: `.c`/`.h` are C, `.rs` is Rust and
//!   `.py`/`.pyi` are Python; others skipped silently. `scan.file_count` =
//!   `len(files[])`.
//! - Parse via the bound backend; capture ParseHealth (status, error regions,
//!   error node count).
//! - **C** include resolution against the filesystem (includer-relative first,
//!   then repo-root-relative) for quoted AND angle-bracket edges (ADR-0002:
//!   angle hits on in-repo files are internal coupling); unresolvable quoted
//!   includes emit `IncludeEdge{resolved:false, to_path:null}` — ARCH-001
//!   signal, not an error; unresolvable angle includes keep the raw include
//!   text in `to_path` with `resolved:false` (external bucket).
//! - **Rust** module-edge resolution is MODULE-dir-relative (reviews N1 +
//!   round-4), not file-dir-relative: `src/a.rs` + `mod b;` looks under
//!   `src/a/`, while a crate-root file (`lib.rs`/`main.rs`, `mod.rs`, or a
//!   direct child of a Cargo crate-root directory) uses its own directory.
//!   Crate-root directories are package-aware (`Cargo.toml` discovery + exact
//!   default target layouts), with the R1 path-suffix heuristic as the
//!   documented fallback when no manifest is discoverable. `mod` declarations
//!   resolve to `.rs`/`mod.rs` files; internal `use` edges resolve to the file
//!   owning the longest module prefix (module-graph semantics, R3) and an
//!   unresolved internal `use` normalizes to `to_path=null, resolved=false`;
//!   angle (external) edges stay raw and unresolved.
//! - Curated macro-pattern recognizer pass → MacroEvents (implemented inside
//!   the backend, which owns the source text; see `backends::tree_sitter_c`).
//! - Engine-native span → IR span conversion happens at this boundary
//!   exclusively (+1 on entry, −1 on exit; ir-v1.md §2).
//!
//! Non-goals: semantic resolution; macro expansion; build-flag awareness;
//! scoring.

pub mod backends;
pub(crate) mod instrument;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::ir::{
    FileRecord, GlobalVar, IncludeEdge, IncludeForm, MacroEvent, ParseHealth, ParseStatus,
    StructDef,
};

/// One scanned file: everything the analyzer and aggregator consume for it.
///
/// This is the unit of the "IR stream" handed to the analyzer (modules.md §2
/// pipeline). Function-level payloads live in [`FunctionUnit`]. Serde derives
/// support the parser-free fixture corpus (ADR-0001 guardrail 2).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct FileUnit {
    pub file: FileRecord,
    pub parse_health: ParseHealth,
    pub includes: Vec<IncludeEdge>,
    pub global_vars: Vec<GlobalVar>,
    pub structs: Vec<StructDef>,
    pub macros: Vec<MacroEvent>,
    pub functions: Vec<FunctionUnit>,
}

/// A function's record plus its attached event streams (ir-v1.md §3.3:
/// events are attached to their enclosing FunctionRecord).
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct FunctionUnit {
    pub record: crate::ir::FunctionRecord,
    pub calls: Vec<crate::ir::CallEvent>,
    pub assignments: Vec<crate::ir::AssignmentEvent>,
    pub binary_ops: Vec<crate::ir::BinaryOpEvent>,
    pub comparisons: Vec<crate::ir::ComparisonEvent>,
    pub access_chains: Vec<crate::ir::AccessChainEvent>,
    pub loops: Vec<crate::ir::LoopNode>,
    /// `await` occurrences in the function body (plan `python-backend` W2;
    /// additive v1 field with `#[serde(default)]`, so documents written before
    /// it deserialize unchanged as an empty stream). The C and Rust backends
    /// always leave it empty — only the Python backend emits `AwaitEvent`s.
    #[serde(default)]
    pub awaits: Vec<crate::ir::AwaitEvent>,
    /// `try_statement` occurrences in the function body (plan `python-backend`
    /// W3; additive v1 field with `#[serde(default)]`, empty for C/Rust).
    #[serde(default)]
    pub try_blocks: Vec<crate::ir::TryEvent>,
    /// Allocation-site occurrences in the function body (plan `js-ts`;
    /// additive v1 field with `#[serde(default)]`, so documents written before
    /// it deserialize unchanged as an empty stream). Only the JavaScript
    /// backend emits these today.
    #[serde(default)]
    pub allocations: Vec<crate::ir::AllocEvent>,
    /// Unary-operator occurrences in the function body (plan `js-ts`;
    /// additive v1 field with `#[serde(default)]`, empty for every non-JS
    /// backend). v1 carries `delete obj.prop`.
    #[serde(default)]
    pub unaries: Vec<crate::ir::UnaryEvent>,
}

/// Scanner failure that is a USAGE error rather than scan data (modules.md
/// Principle 4 / §8.2): only the total unreadability of the target itself.
/// Everything else (unreadable individual files, partial parses, unresolvable
/// includes) is data carried in the IR stream.
#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("scan target is missing or unreadable: {}", .0.display())]
    UnreadableTarget(PathBuf),
}

/// Supported language set, in canonical order, used for the empty-scan
/// `scan.languages` fallback (aggregator). Per-file language tokens are
/// emitted by each backend (and `failed_unit`); a new language must update
/// those backends AND this const — nothing enforces that in lockstep.
pub const SUPPORTED_LANGUAGES: &[&str] = &["c", "js", "python", "rust"];
// "python" landed with the 1.3 schema bump (W3); .py files emit "python" via FileRecord.
// "js" landed with the `js-ts` wiring wave (B2-W1); .js/.jsx/.mjs/.cjs emit "js".

/// Source language selected by extension (plan `rust-backend` R1; JS added by
/// plan `js-ts` B3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LanguageKind {
    C,
    Rust,
    Python,
    Js,
}

/// Global directory names skipped during discovery, for EVERY language
/// (plan `js-ts` §4). Deliberately excluded: `vendor` (C/Rust corpora keep
/// their vendored sources).
const SKIP_DIRS: [&str; 6] = [
    "node_modules",
    "bower_components",
    "dist",
    "build",
    "out",
    "coverage",
];

/// JS/TS-only file-name patterns skipped during discovery (minified/bundled
/// output). Applied only when [`language_of`] already selected JS.
const JS_SKIP_FILE_SUFFIXES: [&str; 4] = [".min.js", ".min.mjs", ".min.cjs", ".bundle.js"];

/// TypeScript/declaration suffixes this wave deliberately does not parse
/// (B3 owns the TS backend). A single-file target with one of these names is
/// skipped silently instead of falling through to the C parser.
const UNSUPPORTED_TS_SUFFIXES: [&str; 7] =
    [".ts", ".mts", ".cts", ".tsx", ".d.ts", ".d.mts", ".d.cts"];

/// Whether a file name is a JS skip pattern (minified/bundled output).
fn has_js_skip_suffix(name: &str) -> bool {
    JS_SKIP_FILE_SUFFIXES
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// Whether a file name carries an unsupported TypeScript/declaration suffix.
fn has_unsupported_ts_suffix(name: &str) -> bool {
    UNSUPPORTED_TS_SUFFIXES
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

/// JS-only per-file size cap: 2 MiB of source. Larger files are minified
/// bundles; they are skipped silently (plan `js-ts` §4).
const JS_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

/// Scan a target path (directory or single `.c`/`.h`/`.rs`/`.py`/`.pyi`/JS file)
/// into the IR stream.
///
/// Single-file policy: a JS/TS-named target that discovery would skip is skipped
/// here too — `*.min.js`/`*.min.mjs`/`*.min.cjs`/`*.bundle.js`, and every
/// unsupported TypeScript/declaration suffix (`.ts`, `.mts`, `.cts`, `.tsx`,
/// `.d.ts`, `.d.mts`, `.d.cts`). A genuinely unknown extension keeps the
/// documented C fallback.
///
/// Determinism (modules.md §1): discovered files are emitted in lexicographic
/// order of their repo-relative POSIX paths; dot-prefixed directories are
/// skipped; symlinks are never followed (v1).
pub fn scan(target: &Path) -> Result<Vec<FileUnit>, ScanError> {
    let root = target
        .canonicalize()
        .map_err(|_| ScanError::UnreadableTarget(target.into()))?;
    let single_file = root.is_file();
    // (repo-relative POSIX path, absolute path) pairs, sorted by rel path
    // (modules.md §1 row 1). A single-file target keeps its own file name as
    // the rel path — joining it onto itself would produce a trailing-slash
    // path that fs::read rejects with ENOTDIR.
    let mut entries: Vec<(String, PathBuf)> = if single_file {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        vec![(name, root.clone())]
    } else {
        // The ROOT target's readability is a usage error (modules.md §8.2):
        // a directory we cannot even list yields an empty scan and a silent
        // exit 0, so it is rejected up front. Failures of NESTED subdirs stay
        // silent-continue inside [`discover`] — partial discovery is data,
        // not an abort (v1 has no counter for them).
        if fs::read_dir(&root).is_err() {
            return Err(ScanError::UnreadableTarget(target.into()));
        }
        discover(&root)
            .into_iter()
            .map(|p| (rel_posix(&root, &p), p))
            .collect()
    };
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // Include resolution anchors at the repo root; for a single-file target
    // that root is the file's parent directory.
    let include_root = if single_file {
        root.parent().unwrap_or(&root).to_path_buf()
    } else {
        root.clone()
    };

    let mut units = Vec::with_capacity(entries.len());
    for (rel, abs) in &entries {
        // Directory discovery admits only known extensions, so `language_of`
        // is always `Some` there. A single-file target is taken as-is, except
        // for the JS/TS suffix policy above; a genuinely unknown extension
        // keeps the pre-R1 default of C.
        let lang = if single_file {
            if has_unsupported_ts_suffix(rel) {
                continue;
            }
            let lang = language_of(rel).unwrap_or(LanguageKind::C);
            if lang == LanguageKind::Js && has_js_skip_suffix(rel) {
                continue;
            }
            lang
        } else {
            language_of(rel).unwrap_or(LanguageKind::C)
        };
        // JS-only 2 MiB cap: minified bundles are skipped silently. Metadata
        // is consulted only for JS; C/Rust/Python read paths are unchanged.
        if lang == LanguageKind::Js
            && let Ok(metadata) = fs::metadata(abs)
            && metadata.len() > JS_MAX_FILE_BYTES
        {
            continue;
        }
        let unit = match fs::read(abs) {
            Ok(source) => {
                let mut unit = extract_unit(rel, &source, lang);
                // C include resolution is per-file and stateless.
                if lang == LanguageKind::C {
                    resolve_c_edges(&mut unit, &include_root, rel);
                }
                unit
            }
            Err(_) => failed_unit(rel, lang),
        };
        units.push(unit);
    }

    // Rust module-graph resolution (R3 workstream B) runs over the WHOLE unit
    // set once every file is extracted, because `use` edges resolve against a
    // module-path → file map assembled from all files' resolved `mod` edges
    // ([`RustModuleMap`]). Three ordered passes keep this deterministic:
    //   1. resolve `mod` edges (module-dir-relative; R1 semantics preserved);
    //   2. build the module map from those resolved edges + crate roots;
    //   3. resolve `use` edges (longest-module-prefix) against the map.
    if units.iter().any(|u| u.file.language == "rust") {
        let ctx = RustModuleContext::discover(&root);
        for unit in units.iter_mut().filter(|u| u.file.language == "rust") {
            // M3: `include_root` (the file's parent for a single-file target,
            // else `root`) is the correct anchor — passing `root` directly made
            // `<file>/foo.rs` the candidate and every `mod` in a single-file
            // scan miss.
            resolve_rust_mod_edges(unit, &ctx, &include_root);
        }
        let map = RustModuleMap::build(&units, &ctx);
        for unit in units.iter_mut().filter(|u| u.file.language == "rust") {
            resolve_rust_use_edges(unit, &map);
        }
    }

    // Python import resolution (plan `python-backend` W2) runs over the WHOLE
    // unit set once every file is extracted, because absolute `import a.b`
    // edges resolve against a dotted-module → file map assembled from all
    // Python units. The backend emits RAW module text in `to_path`
    // (`IncludeForm::Quoted`, `resolved=false`); this pass rewrites hits to the
    // owning repo-relative file or normalizes internal misses to `to_path=None`
    // (external `Angle`-style misses keep the raw text). Data only — no
    // findings are emitted here.
    if units.iter().any(|u| u.file.language == "python") {
        resolve_python_edges(&mut units);
    }

    // JavaScript import resolution + package.json entry declarations (plan
    // `js-ts` §8) run over the WHOLE unit set after extraction, in the same
    // pipeline position as the Python pass. Relative-specifier edges are
    // resolved against the scanned-path set; bare/absolute/URL specifiers were
    // already classified `Angle` by the backend. Data only — no findings are
    // emitted here.
    if units.iter().any(|u| u.file.language == "js") {
        resolve_js_edges(&mut units);
        mark_js_entry_declarations(&mut units, &root);
    }
    Ok(units)
}

/// Recursively discover sources under `root`. Skips dot-prefixed directories,
/// does not follow symlinks, keeps only known-language files
/// (`.c`/`.h`/`.rs`/`.py`/`.pyi`, modules.md §3.2); other extensions are skipped
/// silently.
fn discover(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    // Cycle guard (cross-platform hardening): follow nothing twice. `is_symlink`
    // below skips POSIX symlinks, but a Windows directory *junction* reports
    // `is_dir() == true` and is NOT `is_symlink()`, so without this a junction
    // cycle would re-enter without bound. Canonicalize each popped directory and
    // visit each real directory at most once (deterministic BTreeSet).
    let mut visited: std::collections::BTreeSet<PathBuf> = std::collections::BTreeSet::new();
    while let Some(dir) = stack.pop() {
        let real = dir.canonicalize().unwrap_or_else(|_| dir.clone());
        if !visited.insert(real) {
            continue;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue, // unreadable subdir: skip silently, v1 has no counter for it
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue; // symlinks not followed in v1 (modules.md §3.2)
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let path = entry.path();
            if file_type.is_dir() {
                // Dot-directories stay skipped; the global skip set silences
                // JS-ecosystem/build output for every language (plan `js-ts` §4).
                if !name.starts_with('.') && !SKIP_DIRS.contains(&name) {
                    stack.push(path);
                }
            } else if let Some(lang) = language_of(name) {
                // Minified/bundled JS is noise: skip by file name only (never
                // content sniffing), and only for JS-selected files.
                if lang == LanguageKind::Js && has_js_skip_suffix(name) {
                    continue;
                }
                out.push(path);
            }
        }
    }
    out
}

/// Language detection by file NAME, longest suffix first. `.c`/`.h` are C,
/// `.rs` is Rust, `.py`/`.pyi` are Python, and `.js`/`.jsx`/`.mjs`/`.cjs` are
/// JS. Declaration-only TypeScript (`.d.ts`/`.d.mts`/`.d.cts`) is skipped, and
/// every other TS suffix (`.ts`/`.mts`/`.cts`/`.tsx`) stays `None` in this wave
/// (B2-W1; the TS backend is B3). Matching is case-sensitive.
fn language_of(file_name: &str) -> Option<LanguageKind> {
    // Declaration-only files carry no runtime code.
    if file_name.ends_with(".d.ts")
        || file_name.ends_with(".d.mts")
        || file_name.ends_with(".d.cts")
    {
        return None;
    }
    if file_name.ends_with(".js")
        || file_name.ends_with(".jsx")
        || file_name.ends_with(".mjs")
        || file_name.ends_with(".cjs")
    {
        return Some(LanguageKind::Js);
    }
    if file_name.ends_with(".ts")
        || file_name.ends_with(".mts")
        || file_name.ends_with(".cts")
        || file_name.ends_with(".tsx")
    {
        return None;
    }
    let ext = Path::new(file_name).extension()?;
    if ext == "rs" {
        Some(LanguageKind::Rust)
    } else if ext == "py" || ext == "pyi" {
        Some(LanguageKind::Python)
    } else if ext == "c" || ext == "h" {
        Some(LanguageKind::C)
    } else {
        None
    }
}

/// Repo-relative POSIX path string.
fn rel_posix(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Extract one file's IR via the backend bound to its language.
fn extract_unit(rel_path: &str, source: &[u8], lang: LanguageKind) -> FileUnit {
    let ex = match lang {
        LanguageKind::C => backends::tree_sitter_c::extract(rel_path, source),
        LanguageKind::Rust => backends::tree_sitter_rust::extract(rel_path, source),
        LanguageKind::Python => backends::tree_sitter_python::extract(rel_path, source),
        LanguageKind::Js => backends::tree_sitter_javascript::extract(rel_path, source),
    };
    let mut unit = FileUnit {
        file: ex.file,
        parse_health: ex.parse_health,
        includes: ex.includes,
        global_vars: ex.global_vars,
        structs: ex.structs,
        macros: ex.macros,
        functions: ex.functions,
    };
    // IR v1.4 STEP 2 (plan `divergence-revamp`): populate the three
    // source-local instrument fields (`idiom_hash`, `alloc_family`,
    // `error_path`) per function. This is the only stage holding both the
    // record and its source text; call-shape fields stay `None` here for the
    // aggregator's whole-program pass.
    let language = match lang {
        LanguageKind::C => "c",
        LanguageKind::Rust => "rust",
        LanguageKind::Python => "python",
        LanguageKind::Js => "js",
    };
    for func in &mut unit.functions {
        let body = instrument::body_text(source, &func.record.span);
        instrument::populate(func, &body, language);
    }
    unit
}

/// Per-file read failure → `ParseHealth{failed}` record, never an abort
/// (modules.md §3.2). Structural fields stay empty: there is nothing honest
/// to report about an unreadable file.
fn failed_unit(rel_path: &str, lang: LanguageKind) -> FileUnit {
    FileUnit {
        file: FileRecord {
            path: rel_path.to_string(),
            language: match lang {
                LanguageKind::C => "c".to_string(),
                LanguageKind::Rust => "rust".to_string(),
                LanguageKind::Python => "python".to_string(),
                LanguageKind::Js => "js".to_string(),
            },
            loc: Default::default(),
            function_count: 0,
            global_var_count: 0,
            include_counts: Default::default(),
        },
        parse_health: ParseHealth {
            status: ParseStatus::Failed,
            error_regions: Vec::new(),
            error_node_count: 0,
            resolver: None,
        },
        ..FileUnit::default()
    }
}

/// C include resolution: BOTH quoted and angle-bracket edges are resolved with
/// the same two candidates — includer-relative first, then repo-root-relative.
/// A hit rewrites `to_path` to the repo-relative POSIX path and sets
/// `resolved=true` (angle hits count as internal coupling per ADR-0002). A
/// miss keeps the edge as data, not error: quoted ⇒ `to_path=null`
/// (`resolved=false`, ARCH-001 signal); angle ⇒ raw include text preserved in
/// `to_path` with `resolved=false` (external bucket).
///
/// The backend leaves raw include text in `to_path`; this pass replaces it
/// with the resolved repo-relative POSIX path or `None`.
fn resolve_c_edges(unit: &mut FileUnit, root: &Path, rel_path: &str) {
    let includer_dir = includer_dir(rel_path);
    for edge in &mut unit.includes {
        let Some(raw) = edge.to_path.clone() else {
            continue;
        };
        let candidates = [
            join_normalized(&includer_dir, &raw),
            join_normalized("", &raw),
        ];
        let resolved_rel = candidates.into_iter().find_map(|candidate| {
            let abs = root.join(candidate.replace('/', std::path::MAIN_SEPARATOR_STR));
            if abs.is_file() { Some(candidate) } else { None }
        });
        match resolved_rel {
            Some(rel) => {
                // In-repo target (quoted OR angle): internal coupling edge.
                edge.to_path = Some(rel);
                edge.resolved = true;
            }
            None => {
                if edge.form == IncludeForm::Quoted {
                    // Unresolvable quoted: ARCH-001 signal, not an error.
                    edge.to_path = None;
                    edge.resolved = false;
                }
                // Unresolvable angle: external bucket — keep the raw include
                // text in `to_path`, `resolved=false` (unchanged semantics).
            }
        }
    }
}

/// Python import resolution over the whole unit set (plan `python-backend`
/// W2). The Python backend emits [`IncludeEdge`]s with the RAW module text in
/// `to_path`, [`IncludeForm::Quoted`], `resolved=false`, `is_mod=false`. This
/// pass resolves exactly that edge encoding:
///
/// - `import a.b.c` → raw `"a.b.c"`; `import a.b as x` → raw `"a.b"`;
///   `import a, b` → raw `"a"`, `"b"`.
/// - `from a.b import c, d` → ONE raw `"a.b"` (module dependency; imported
///   names are not separate edges).
/// - `from .mod import z` → raw `".mod"` (leading dots preserved).
/// - `from . import x, y` → raw `".x"`, `".y"`.
/// - `from x import *` → raw `"x"`.
///
/// Determinism: `module_map` is a `BTreeMap`, `top_level` a `BTreeSet`, unit
/// order is the scanner's sorted emission, and edges retain source order.
/// Relative imports are always INTERNAL: a miss normalizes to `to_path=None`.
/// An absolute miss whose first segment names a top-level scanned module is
/// internal (also `to_path=None`); any other absolute miss is the external
/// bucket and keeps the raw text. `__init__.py` one-level re-exports fall out
/// of the hub's own relative edges; deep name-level re-export tracing is OUT OF
/// SCOPE (T2 `python-resolver`) and is deliberately NOT guessed here.
fn resolve_python_edges(units: &mut [FileUnit]) {
    // Dotted module path → repo-relative POSIX file, from every `.py` and
    // `.pyi` unit (so a `.py` importing a stub-only module resolves to the
    // `.pyi`; `python_dotted_module` applies `__init__.pyi` like
    // `__init__.py`).
    let mut module_map: BTreeMap<String, String> = BTreeMap::new();
    for unit in units.iter().filter(|u| u.file.language == "python") {
        let path = &unit.file.path;
        if !(path.ends_with(".py") || path.ends_with(".pyi")) {
            continue;
        }
        if let Some(module) = python_dotted_module(path) {
            module_map.entry(module).or_insert_with(|| path.clone());
        }
    }
    let top_level: BTreeSet<String> = module_map
        .keys()
        .filter_map(|module| module.split('.').next())
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect();

    for unit in units.iter_mut().filter(|u| u.file.language == "python") {
        let package = python_package(&unit.file.path);
        let base: Vec<&str> = if package.is_empty() {
            Vec::new()
        } else {
            package.split('.').collect()
        };
        for edge in &mut unit.includes {
            let Some(raw) = edge.to_path.clone() else {
                continue;
            };
            if raw.starts_with('.') {
                // Relative: leading dots are preserved in `raw`.
                let dots = raw.chars().take_while(|&c| c == '.').count();
                let rest = &raw[dots..];
                // `dots == 1` anchors at the includer package; each extra dot
                // climbs one parent package.
                let drop = dots.saturating_sub(1);
                let keep = base.len().saturating_sub(drop);
                let base_str = base[..keep].join(".");
                let candidate = if rest.is_empty() {
                    base_str
                } else {
                    format!("{base_str}.{rest}")
                };
                match module_map.get(&candidate) {
                    Some(file) => {
                        edge.to_path = Some(file.clone());
                        edge.resolved = true;
                    }
                    None => {
                        // Relative miss is always internal (data, not error).
                        edge.to_path = None;
                        edge.resolved = false;
                    }
                }
            } else if let Some(file) = module_map.get(&raw) {
                edge.to_path = Some(file.clone());
                edge.resolved = true;
            } else {
                let first = raw.split('.').next().unwrap_or("");
                if top_level.contains(first) {
                    // The top-level package is in-repo but the full path is
                    // not a module file: internal unresolved.
                    edge.to_path = None;
                    edge.resolved = false;
                }
                // Absolute miss on an unknown top-level package: external
                // bucket — keep the raw text in `to_path`, `resolved=false`.
            }
        }
    }
}

/// Dotted Python module path for a repo-relative path (plan `python-backend`
/// W2). `pkg/__init__.py` → `"pkg"`, `pkg/sub/mod.py` → `"pkg.sub.mod"`,
/// repo-root `foo.py` → `"foo"`, root `__init__.py` → `""`. `.pyi` stubs use
/// the same rule; non-Python extensions return `None`.
fn python_dotted_module(path: &str) -> Option<String> {
    let (dir, file) = split_rel(path);
    let stem = file
        .strip_suffix(".py")
        .or_else(|| file.strip_suffix(".pyi"))?;
    if stem == "__init__" {
        Some(dir.replace('/', "."))
    } else if dir.is_empty() {
        Some(stem.to_string())
    } else {
        Some(format!("{}.{}", dir.replace('/', "."), stem))
    }
}

/// Package containing a Python unit (plan `python-backend` W2). For an
/// `__init__.py`/`__init__.pyi` hub the package IS its module; for an ordinary
/// module it is the module minus its final segment; a top-level module (or a
/// non-Python path) yields `""`.
fn python_package(path: &str) -> String {
    let (dir, file) = split_rel(path);
    if file == "__init__.py" || file == "__init__.pyi" {
        return dir.replace('/', ".");
    }
    python_dotted_module(path)
        .and_then(|module| {
            module
                .rsplit_once('.')
                .map(|(package, _)| package.to_string())
        })
        .unwrap_or_default()
}

/// JS probe extensions in deterministic order (plan `js-ts` §8). This is the
/// documented v1 order; TypeScript suffixes are listed first but only JS
/// targets exist in the scanned set this wave (B3 makes the TS entries
/// resolvable).
const JS_RESOLVE_EXTS: [&str; 8] = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".mts", ".cts"];

/// Resolve JavaScript relative-specifier imports over the whole unit set (plan
/// `js-ts` §8). The backend emits RAW specifier text in `to_path`,
/// [`IncludeForm::Quoted`] for `./`/`../` and `Angle` for bare/absolute/URL/
/// `node:` specifiers. This pass handles `Quoted` edges only:
///
/// - normalize the specifier against the includer's directory (POSIX join; an
///   escape above the scan root is treated as unresolved);
/// - probe the scanned-path set deterministically: exact path, then each
///   [`JS_RESOLVE_EXTS`] extension, then `<path>/index.<ext>`;
/// - a hit sets `to_path`/`resolved=true`; a miss normalizes to
///   `to_path=None`/`resolved=false` (unresolved-internal, data only).
///
/// `Angle` edges are never touched, and no findings are created here.
/// Determinism: `BTreeSet` path set, source-order edge iteration, fixed probe
/// order.
fn resolve_js_edges(units: &mut [FileUnit]) {
    let scanned: BTreeSet<String> = units
        .iter()
        .filter(|u| u.file.language == "js")
        .map(|u| u.file.path.clone())
        .collect();

    for unit in units.iter_mut().filter(|u| u.file.language == "js") {
        let includer = includer_dir(&unit.file.path);
        for edge in &mut unit.includes {
            if edge.form != IncludeForm::Quoted {
                continue;
            }
            let Some(raw) = edge.to_path.clone() else {
                continue;
            };
            match resolve_js_specifier(&includer, &raw, &scanned) {
                Some(path) => {
                    edge.to_path = Some(path);
                    edge.resolved = true;
                }
                None => {
                    // Unresolved relative import is internal data, not error.
                    edge.to_path = None;
                    edge.resolved = false;
                }
            }
        }
    }
}

/// Resolve one relative JS specifier against the scanned-path set. Returns
/// `None` for non-relative specifiers, root escapes, and misses.
fn resolve_js_specifier(
    includer_dir: &str,
    raw: &str,
    scanned: &BTreeSet<String>,
) -> Option<String> {
    if !(raw.starts_with("./") || raw.starts_with("../")) {
        return None;
    }
    let base = join_within_root(includer_dir, raw)?;
    if scanned.contains(&base) {
        return Some(base);
    }
    for ext in JS_RESOLVE_EXTS {
        let candidate = format!("{base}{ext}");
        if scanned.contains(&candidate) {
            return Some(candidate);
        }
    }
    for ext in JS_RESOLVE_EXTS {
        let candidate = join_dir(&base, &format!("index{ext}"));
        if scanned.contains(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Join a JS specifier onto a base directory and lexically normalize it,
/// returning `None` when a `..` component would escape the scan root. This is
/// the JS resolver's guard on top of [`join_normalized`] (whose C/Rust callers
/// deliberately clamp instead).
fn join_within_root(base_dir: &str, target: &str) -> Option<String> {
    if target.starts_with('/') {
        return None;
    }
    let joined = if base_dir.is_empty() {
        target.to_string()
    } else {
        format!("{base_dir}/{target}")
    };
    let mut depth: i32 = 0;
    for component in Path::new(&joined).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            Component::Normal(_) => depth += 1,
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(join_normalized(base_dir, target))
}

/// Max directory depth probed for `package.json` (bounded walk).
const PACKAGE_JSON_WALK_MAX_DEPTH: usize = 24;

/// Mark the synthetic `<module>` record of every JS file named by a
/// `package.json` `main`/`bin` field (`entry_declared = Some(true)`), plan
/// `js-ts` §8.
///
/// A deterministic bounded walk finds `package.json` files, skipping dot-dirs,
/// the global [`SKIP_DIRS`], and symlinks; unreadable/empty dirs and malformed
/// manifests are per-file data (ignored, scan continues — never panic/exit 3).
/// `main` (string), `bin` (string | object of strings | array of strings) are
/// normalised by stripping a leading `./`, joining with the manifest's
/// directory, and probing the exact path, `.js/.jsx/.mjs/.cjs` (plus
/// `.ts/.tsx`), then `<dir>/index.*`.
///
/// Accepted limitations: `exports` maps, `"type"`, and `tsconfig` `paths` are
/// NOT consulted.
fn mark_js_entry_declarations(units: &mut [FileUnit], root: &Path) {
    let scanned: BTreeSet<String> = units
        .iter()
        .filter(|u| u.file.language == "js")
        .map(|u| u.file.path.clone())
        .collect();
    if scanned.is_empty() {
        return;
    }

    let search = if root.is_file() {
        root.parent().unwrap_or(root)
    } else {
        root
    };
    let mut entry_paths: BTreeSet<String> = BTreeSet::new();
    for manifest in discover_package_json(search) {
        let absolute = search.join(manifest.replace('/', std::path::MAIN_SEPARATOR_STR));
        let Ok(bytes) = fs::read(&absolute) else {
            continue; // unreadable manifest: per-file data, keep scanning
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue; // malformed manifest: ignored, never panic/exit 3
        };
        let package_dir = includer_dir(&manifest);
        let mut raw_entries: Vec<String> = Vec::new();
        if let Some(main) = value.get("main").and_then(|v| v.as_str()) {
            raw_entries.push(main.to_string());
        }
        match value.get("bin") {
            Some(serde_json::Value::String(bin)) => raw_entries.push(bin.clone()),
            Some(serde_json::Value::Array(items)) => {
                for item in items {
                    if let Some(bin) = item.as_str() {
                        raw_entries.push(bin.to_string());
                    }
                }
            }
            Some(serde_json::Value::Object(map)) => {
                for bin in map.values() {
                    if let Some(bin) = bin.as_str() {
                        raw_entries.push(bin.to_string());
                    }
                }
            }
            _ => {}
        }
        for raw in raw_entries {
            for candidate in js_entry_candidates(&package_dir, &raw) {
                if scanned.contains(&candidate) {
                    entry_paths.insert(candidate);
                }
            }
        }
    }

    if entry_paths.is_empty() {
        return;
    }
    for unit in units.iter_mut().filter(|u| u.file.language == "js") {
        if !entry_paths.contains(&unit.file.path) {
            continue;
        }
        if let Some(module) = unit.functions.iter_mut().find(|f| f.record.is_module_scope) {
            module.record.entry_declared = Some(true);
        }
    }
}

/// Deterministic bounded walk collecting repo-relative `package.json` paths
/// under `root`. Skips dot-dirs, the global [`SKIP_DIRS`], and symlinks; a
/// canonical-path visited set guards junctions. Output is sorted/deduped.
fn discover_package_json(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    let mut visited: BTreeSet<PathBuf> = BTreeSet::new();
    while let Some((dir, depth)) = stack.pop() {
        let real = dir.canonicalize().unwrap_or_else(|_| dir.clone());
        if !visited.insert(real) {
            continue;
        }
        let manifest = dir.join("package.json");
        if manifest.is_file() {
            out.push(rel_posix(root, &manifest));
        }
        if depth >= PACKAGE_JSON_WALK_MAX_DEPTH {
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue; // unreadable dir: skip silently
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() || !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with('.') || SKIP_DIRS.contains(&name) {
                continue;
            }
            stack.push((entry.path(), depth + 1));
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Deterministic candidate paths for one `package.json` `main`/`bin` value:
/// the exact normalized path, extension probes, then `<dir>/index.*`.
///
/// A value that escapes the SCAN ROOT (`"../../outside.js"` from a root-level
/// package) yields NO candidates: [`join_within_root`] is the same guard the
/// resolver uses. It is scan-root confinement, not package-directory
/// confinement — a nested `packages/a` manifest may point at `../shared.js`
/// when the result remains in-tree.
fn js_entry_candidates(package_dir: &str, raw: &str) -> Vec<String> {
    let trimmed = raw.strip_prefix("./").unwrap_or(raw);
    let Some(base) = join_within_root(package_dir, trimmed) else {
        return Vec::new();
    };
    let mut candidates: BTreeSet<String> = BTreeSet::new();
    candidates.insert(base.clone());
    for ext in [".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx"] {
        candidates.insert(format!("{base}{ext}"));
    }
    for ext in [".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx"] {
        candidates.insert(join_dir(&base, &format!("index{ext}")));
    }
    candidates.into_iter().collect()
}

/// Discovered Cargo package layout for Rust module resolution (R3 workstream
/// B). When at least one `Cargo.toml` is discoverable under the scan root,
/// crate-root directories are matched EXACTLY against Cargo's default target
/// layouts (`<pkg>/src/bin`, `<pkg>/examples`, `<pkg>/tests`,
/// `<pkg>/benches`); `lib.rs`/`main.rs` under `<pkg>/src` are crate roots by
/// the file-stem rule. This supersedes R1's path-suffix approximation, which
/// mis-classified a module directory literally named `tests`/`examples`/… as a
/// crate-root directory (e.g. `src/tests/`).
///
/// `src` itself is intentionally NOT in `crate_root_dirs`: only `lib.rs` /
/// `main.rs` directly under it are crate roots (matching rustc — `src/a.rs`
/// is a module, not a crate root), which the stem rule already covers.
///
/// Fallback: when NO `Cargo.toml` is discoverable (the scan target is an
/// isolated subdirectory), the R1 path-suffix heuristic is retained so
/// workspace layouts without discoverable manifests keep resolving.
#[derive(Debug, Default)]
struct RustModuleContext {
    has_packages: bool,
    crate_root_dirs: BTreeSet<String>,
}

impl RustModuleContext {
    /// Bounded walk for `Cargo.toml` manifests under `scan_root`, mapping each
    /// manifest's directory to the default target-layout crate-root dirs.
    /// Custom `[lib] path` / `[[bin]] path` overrides are OUT OF SCOPE
    /// (accepted limitation).
    fn discover(scan_root: &Path) -> Self {
        let search = if scan_root.is_file() {
            scan_root.parent().unwrap_or(scan_root)
        } else {
            scan_root
        };
        let packages = discover_packages(search);
        let mut crate_root_dirs = BTreeSet::new();
        for pkg in &packages {
            for suffix in ["src/bin", "examples", "tests", "benches"] {
                crate_root_dirs.insert(join_dir(pkg, suffix));
            }
        }
        Self {
            has_packages: !packages.is_empty(),
            crate_root_dirs,
        }
    }

    /// Whether `dir` is a crate-root directory. Package-aware exact match when
    /// manifests are known; R1 path-suffix fallback otherwise.
    fn is_crate_root_dir(&self, dir: &str) -> bool {
        if self.has_packages {
            self.crate_root_dirs.contains(dir)
        } else {
            is_crate_root_dir_fallback(dir)
        }
    }

    /// Whether `rel_path` is a crate-root FILE: stem `lib`/`main`, or a direct
    /// child of a crate-root directory.
    fn is_crate_root_file(&self, rel_path: &str) -> bool {
        let (dir, file) = split_rel(rel_path);
        let stem = file.strip_suffix(".rs").unwrap_or(file);
        matches!(stem, "lib" | "main") || self.is_crate_root_dir(dir)
    }
}

/// Max directory depth probed for `Cargo.toml` manifests (bounded walk).
const CARGO_WALK_MAX_DEPTH: usize = 12;

/// Bounded walk collecting the repo-relative directories that contain a
/// `Cargo.toml` (package / workspace-member roots). Skips dot-directories,
/// symlinks and build/vendor noise (`target`, `node_modules`); depth-bounded so
/// discovery stays cheap on large trees.
fn discover_packages(root: &Path) -> Vec<String> {
    let mut packages = Vec::new();
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if dir.join("Cargo.toml").is_file() {
            packages.push(rel_posix(root, &dir));
        }
        if depth >= CARGO_WALK_MAX_DEPTH {
            continue;
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            // `file_type` does not follow symlinks: symlinked dirs are skipped.
            if !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            stack.push((entry.path(), depth + 1));
        }
    }
    packages.sort();
    packages.dedup();
    packages
}

/// Module-path → file map built from resolved `mod` edges (R3 workstream B).
///
/// Keyed per CRATE ROOT FILE (not per package) so a package's lib, each bin,
/// each test and each example — every one an independent crate — keep separate
/// `crate::` namespaces. `by_path` maps `(crate_root, module_path)` →
/// repo-relative file; `file_module` maps a file → its
/// `(crate_root, module_path)`.
#[derive(Debug, Default)]
struct RustModuleMap {
    by_path: BTreeMap<(String, Vec<String>), String>,
    file_module: BTreeMap<String, (String, Vec<String>)>,
}

impl RustModuleMap {
    /// Seed every crate-root file with the empty module path, then propagate
    /// module paths down resolved `mod` edges to a fixpoint. Determinism:
    /// units arrive sorted by path, edges keep source order, and every
    /// structure is a `BTree*`.
    fn build(units: &[FileUnit], ctx: &RustModuleContext) -> Self {
        let mut map = Self::default();
        for unit in units.iter().filter(|u| u.file.language == "rust") {
            let rel = &unit.file.path;
            if ctx.is_crate_root_file(rel) {
                map.file_module
                    .insert(rel.clone(), (rel.clone(), Vec::new()));
                map.by_path.insert((rel.clone(), Vec::new()), rel.clone());
            }
        }
        // At most one new file mapping is added per pass; `units.len()` passes
        // is a safe upper bound (the `mod` graph is a forest in valid Rust).
        for _ in 0..=units.len() {
            let mut changed = false;
            for unit in units.iter().filter(|u| u.file.language == "rust") {
                let source = &unit.file.path;
                let Some((crate_key, module_path)) = map.file_module.get(source).cloned() else {
                    continue;
                };
                for edge in &unit.includes {
                    // Only RESOLVED `mod` edges propagate here: `use` edges are
                    // still raw/unresolved at this point and `mod` misses have
                    // `to_path=None`.
                    if edge.form != IncludeForm::Quoted || !edge.resolved {
                        continue;
                    }
                    let Some(target) = &edge.to_path else {
                        continue;
                    };
                    let Some(segments) = mod_target_segments(source, target, ctx) else {
                        continue;
                    };
                    let mut child = module_path.clone();
                    child.extend(segments);
                    if !map.file_module.contains_key(target) {
                        map.file_module
                            .insert(target.clone(), (crate_key.clone(), child.clone()));
                        changed = true;
                    }
                    map.by_path
                        .entry((crate_key.clone(), child))
                        .or_insert_with(|| target.clone());
                }
            }
            if !changed {
                break;
            }
        }
        map
    }

    /// Resolve one internal `use` raw path (e.g. `crate::a::b::Thing`) against
    /// the map, returning the owning repo-relative file or `None`.
    ///
    /// Path semantics: `crate` → absolute from the crate root; `self` → the
    /// source module; each leading `super` → one parent (underflow fails).
    /// Raw identifiers (`r#type`) are normalized to their bare names (C1).
    /// The LAST segment is usually an imported ITEM, so the LONGEST module
    /// prefix that maps to a file wins (`crate::util::helper` → `util.rs`;
    /// `crate::a::b::Thing` → whichever of `a::b`, `a` is a file).
    ///
    /// Inline modules (`mod foo { … }`) have no backing file, so a path that
    /// only matches an inline module resolves to the file containing it (the
    /// longest FILE-backed prefix) — a documented approximation.
    ///
    /// Only module prefixes of length ≥ 1 are considered: the empty path is
    /// the crate-root FILE itself, and using it as a catch-all fallback would
    /// resolve every unknown `crate::missing` to the crate root. Importing an
    /// item defined directly at the crate root therefore stays unresolved — a
    /// documented false-negative class.
    ///
    /// Returns `None` for a self-import (target == source): an intra-file
    /// import is not coupling and would otherwise manufacture a self-loop
    /// ARCH-001 cycle.
    fn resolve_use(&self, source: &str, raw: &str) -> Option<String> {
        let (crate_key, module_path) = match self.file_module.get(source) {
            Some(entry) => entry.clone(),
            // Unmapped file (repo-root single-file target, or a module not
            // reachable from any crate root): treat it as its own implicit
            // crate root. Documented fallback.
            None => (source.to_string(), Vec::new()),
        };
        let parts: Vec<&str> = raw
            .split("::")
            // C1 defensive normalization: the backend already strips `r#`, but
            // a raw identifier arriving here must still map to its bare name.
            .map(|segment| segment.strip_prefix("r#").unwrap_or(segment))
            .collect();
        let mut segments: Vec<String> = Vec::new();
        let rest_start = match parts.first().copied() {
            Some("crate") => 1,
            Some("self") => {
                segments = module_path;
                1
            }
            Some("super") => {
                segments = module_path;
                let mut i = 0;
                while i < parts.len() && parts[i] == "super" {
                    segments.pop()?; // `super` above the crate root
                    i += 1;
                }
                i
            }
            // Defensive default: a bare leading identifier CANNOT reach here —
            // `resolve_rust_use_edges` only calls this for `Quoted` edges, and
            // the backend classifies a bare `use` path (no `::`, not
            // `crate`/`self`/`super`) as `Angle` (external). Kept as a total
            // fallback so this function never panics on unexpected input.
            _ => 0,
        };
        for segment in &parts[rest_start..] {
            if !segment.is_empty() {
                segments.push((*segment).to_string());
            }
        }
        for len in (1..=segments.len()).rev() {
            if let Some(target) = self
                .by_path
                .get(&(crate_key.clone(), segments[..len].to_vec()))
            {
                return if target == source {
                    None
                } else {
                    Some(target.clone())
                };
            }
        }
        None
    }
}

/// Resolve `mod` DECLARATIONS for one unit (R1 semantics, reviews m2 + N1 +
/// round-4; R3 threads the package-aware [`RustModuleContext`]). Module-dir-
/// relative per [`module_dir`] — `src/a.rs` + `mod b;` → `src/a/b.rs` /
/// `src/a/b/mod.rs` (a bare `src/b.rs` is rustc error[E0583]). Inline-module
/// nesting arrives as a `/`-joined raw prefix (`mod outer { mod inner; }` →
/// `outer/inner`), so candidates are `<module_dir>/outer/inner.rs` /
/// `<module_dir>/outer/inner/mod.rs`. `use` edges are left for
/// [`resolve_rust_use_edges`]; a miss yields `to_path=null`, `resolved=false`.
fn resolve_rust_mod_edges(unit: &mut FileUnit, ctx: &RustModuleContext, root: &Path) {
    let module_dir = module_dir(&unit.file.path, ctx);
    for edge in &mut unit.includes {
        if edge.form != IncludeForm::Quoted {
            continue;
        }
        let Some(raw) = edge.to_path.clone() else {
            continue;
        };
        // A `use` path (not a `mod` declaration): resolved in pass 3.
        if is_use_edge(&raw) {
            continue;
        }
        let candidates = [
            join_normalized(&module_dir, &format!("{raw}.rs")),
            join_normalized(&module_dir, &format!("{raw}/mod.rs")),
        ];
        let resolved_rel = candidates.into_iter().find(|candidate| {
            root.join(candidate.replace('/', std::path::MAIN_SEPARATOR_STR))
                .is_file()
        });
        match resolved_rel {
            Some(rel) => {
                edge.to_path = Some(rel);
                edge.resolved = true;
            }
            None => {
                edge.to_path = None;
                edge.resolved = false;
            }
        }
    }
}

/// Resolve internal `use` edges for one unit against the whole-scan module map
/// (R3 workstream B). Supersedes the R1 founder decision that internal `use`
/// paths retain their raw `::` text with `resolved=false`: resolution now
/// exists, so a hit rewrites `to_path` to the owning repo-relative file
/// (`resolved=true`), and a miss NORMALIZES to `to_path=null`,
/// `resolved=false` — the unresolved-`Quoted` IR contract (an unresolved
/// internal edge carries no raw text). `Angle` (external) edges are untouched.
fn resolve_rust_use_edges(unit: &mut FileUnit, map: &RustModuleMap) {
    for edge in &mut unit.includes {
        if edge.form != IncludeForm::Quoted {
            continue;
        }
        let Some(raw) = edge.to_path.clone() else {
            continue;
        };
        if !is_use_edge(&raw) {
            continue;
        }
        match map.resolve_use(&unit.file.path, &raw) {
            Some(target) => {
                edge.to_path = Some(target);
                edge.resolved = true;
            }
            None => {
                edge.to_path = None;
                edge.resolved = false;
            }
        }
    }
}

/// Distinguish a `use` raw path from a `mod` declaration raw. The backend joins
/// `use` segments with `::` and inline `mod` prefixes with `/`; the only
/// single-segment internal `use` raws are the keywords `crate`/`self`/`super`
/// (a bare identifier is classified `Angle`/external by the backend).
fn is_use_edge(raw: &str) -> bool {
    raw.contains("::") || matches!(raw, "crate" | "self" | "super")
}

/// Recover the `mod`-raw segments for a resolved `mod` edge by relating the
/// resolved target path back to the source file's module directory.
/// `mod outer { mod inner; }` (target `…/outer/inner.rs`) → `["outer",
/// "inner"]`; `mod pkg;` hitting `…/pkg/mod.rs` → `["pkg"]`.
fn mod_target_segments(source: &str, target: &str, ctx: &RustModuleContext) -> Option<Vec<String>> {
    let base = module_dir(source, ctx);
    let prefix = if base.is_empty() {
        String::new()
    } else {
        format!("{base}/")
    };
    let suffix = target.strip_prefix(&prefix)?;
    let stem = suffix.strip_suffix(".rs")?;
    let trimmed = stem.strip_suffix("/mod").unwrap_or(stem);
    if trimmed.is_empty() {
        Some(Vec::new())
    } else {
        // C1 defensive: a resolved target filename never carries `r#`, but
        // strip defensively so a raw-identifier segment can never leak into
        // the module map.
        Some(
            trimmed
                .split('/')
                .map(|segment| segment.strip_prefix("r#").unwrap_or(segment).to_string())
                .collect(),
        )
    }
}

/// Repo-relative directory of `rel_path` (empty for a root-level file).
fn includer_dir(rel_path: &str) -> String {
    split_rel(rel_path).0.to_string()
}

/// Split a repo-relative path into `(dir, file)`; `dir` is empty at the root.
fn split_rel(rel_path: &str) -> (&str, &str) {
    match rel_path.rsplit_once('/') {
        Some((dir, file)) => (dir, file),
        None => ("", rel_path),
    }
}

/// Join a package root (possibly empty) with a POSIX subdirectory.
fn join_dir(root: &str, sub: &str) -> String {
    if root.is_empty() {
        sub.to_string()
    } else {
        format!("{root}/{sub}")
    }
}

/// Rust module directory for `rel_path` (reviews N1 + round-3 + round-4; R3
/// package-aware crate roots). Matches rustc: a CRATE-ROOT file resolves its
/// submodules in its OWN (containing) directory, while a submodule file
/// `<dir>/<stem>.rs` owns the `<dir>/<stem>/` subdirectory.
///
/// A file behaves as a crate root when its stem is `lib`/`main` or it is a
/// direct child of a crate-root directory
/// ([`RustModuleContext::is_crate_root_dir`]); `mod.rs` is not a crate root but
/// resolves submodules in its own directory.
///
/// Examples: `src/lib.rs` → `src`; `src/a/mod.rs` → `src/a`; `src/a.rs` →
/// `src/a`; `src/bin/foo.rs` → `src/bin`; `tests/x.rs` → `tests`;
/// `tests/common/mod.rs` → `tests/common`; `src/bin/foo/bar.rs` →
/// `src/bin/foo/bar`. A repo-root `foo.rs` (single-file target) → `foo`.
fn module_dir(rel_path: &str, ctx: &RustModuleContext) -> String {
    let (dir, file) = split_rel(rel_path);
    let stem = file.strip_suffix(".rs").unwrap_or(file);
    if matches!(stem, "lib" | "main" | "mod") || ctx.is_crate_root_dir(dir) {
        dir.to_string()
    } else if dir.is_empty() {
        stem.to_string()
    } else {
        format!("{dir}/{stem}")
    }
}

/// R1 fallback crate-root-directory test, used only when NO `Cargo.toml` is
/// discoverable: `dir` is, or ends with, a Cargo default target directory.
/// Segment-based so workspace layouts (`crates/foo/src/bin`) are recognized.
/// Superseded by exact package-aware matching when manifests are known.
fn is_crate_root_dir_fallback(dir: &str) -> bool {
    const CRATE_ROOT_DIRS: [&str; 4] = ["src/bin", "examples", "tests", "benches"];
    CRATE_ROOT_DIRS
        .iter()
        .any(|root| dir == *root || dir.ends_with(&format!("/{root}")))
}

/// Join an include target onto a base directory and lexically normalize the
/// result to a clean repo-relative POSIX path (`.`/`..` resolved without
/// touching the filesystem).
fn join_normalized(base_dir: &str, target: &str) -> String {
    let joined = if target.starts_with('/') || base_dir.is_empty() {
        target.to_string()
    } else {
        format!("{base_dir}/{target}")
    };
    let mut parts: Vec<String> = Vec::new();
    for component in Path::new(&joined).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop(); // above-repo escapes clamp at the root
            }
            Component::Normal(seg) => parts.push(seg.to_string_lossy().into_owned()),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    parts.join("/")
}
