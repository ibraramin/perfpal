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
//! - Language detection by extension: `.c`/`.h` are C and `.rs` is Rust in v1;
//!   others skipped silently. `scan.file_count` = `len(files[])`.
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
//!   direct child of `src/bin`/`examples`/`tests`/`benches`, matched by
//!   trailing segment for workspace packages) uses its own directory. Only
//!   `mod` declarations are resolved; `use` edges keep their raw `::` path
//!   with `resolved=false` (founder decision), and angle edges stay raw.
//! - Curated macro-pattern recognizer pass → MacroEvents (implemented inside
//!   the backend, which owns the source text; see `backends::tree_sitter_c`).
//! - Engine-native span → IR span conversion happens at this boundary
//!   exclusively (+1 on entry, −1 on exit; ir-v1.md §2).
//!
//! Non-goals: semantic resolution; macro expansion; build-flag awareness;
//! scoring.

pub mod backends;

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
pub const SUPPORTED_LANGUAGES: &[&str] = &["c", "rust"];

/// Source language selected by extension (plan `rust-backend` R1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LanguageKind {
    C,
    Rust,
}

/// Scan a target path (directory or single `.c`/`.h`/`.rs` file) into the IR stream.
///
/// Determinism (modules.md §1): discovered files are emitted in lexicographic
/// order of their repo-relative POSIX paths; dot-prefixed directories are
/// skipped; symlinks are never followed (v1).
pub fn scan(target: &Path) -> Result<Vec<FileUnit>, ScanError> {
    let root = target
        .canonicalize()
        .map_err(|_| ScanError::UnreadableTarget(target.into()))?;
    // (repo-relative POSIX path, absolute path) pairs, sorted by rel path
    // (modules.md §1 row 1). A single-file target keeps its own file name as
    // the rel path — joining it onto itself would produce a trailing-slash
    // path that fs::read rejects with ENOTDIR.
    let mut entries: Vec<(String, PathBuf)> = if root.is_file() {
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
    let include_root = if root.is_file() {
        root.parent().unwrap_or(&root).to_path_buf()
    } else {
        root.clone()
    };

    let mut units = Vec::with_capacity(entries.len());
    for (rel, abs) in &entries {
        // Directory discovery admits only known extensions, so `language_of`
        // is always `Some` there. A single-file target is taken as-is, and an
        // unknown extension keeps the pre-R1 default of C.
        let lang = language_of(rel).unwrap_or(LanguageKind::C);
        let unit = match fs::read(abs) {
            Ok(source) => {
                let mut unit = extract_unit(rel, &source, lang);
                resolve_includes(&mut unit, &include_root, rel, lang);
                unit
            }
            Err(_) => failed_unit(rel, lang),
        };
        units.push(unit);
    }
    Ok(units)
}

/// Recursively discover sources under `root`. Skips dot-prefixed directories,
/// does not follow symlinks, keeps only known-language files (`.c`/`.h`/`.rs`,
/// modules.md §3.2); other extensions are skipped silently.
fn discover(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
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
                if !name.starts_with('.') {
                    stack.push(path);
                }
            } else if language_of(name).is_some() {
                out.push(path);
            }
        }
    }
    out
}

/// Language detection by extension: `.c`/`.h` are C, `.rs` is Rust.
fn language_of(file_name: &str) -> Option<LanguageKind> {
    let ext = Path::new(file_name).extension()?;
    if ext == "rs" {
        Some(LanguageKind::Rust)
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
    };
    FileUnit {
        file: ex.file,
        parse_health: ex.parse_health,
        includes: ex.includes,
        global_vars: ex.global_vars,
        structs: ex.structs,
        macros: ex.macros,
        functions: ex.functions,
    }
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

/// Include-edge resolution against the filesystem (modules.md §3.2; ADR-0002
/// chart policy). C keeps its shipped behavior ([`resolve_c_edges`]); Rust
/// resolves `Quoted` module edges to `.rs`/`mod.rs` files
/// ([`resolve_rust_edges`]).
fn resolve_includes(unit: &mut FileUnit, root: &Path, rel_path: &str, lang: LanguageKind) {
    match lang {
        LanguageKind::C => resolve_c_edges(unit, root, rel_path),
        LanguageKind::Rust => resolve_rust_edges(unit, root, rel_path),
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

/// Rust module-edge resolution (plan `rust-backend` R1; reviews m2 + N1 +
/// round-4): resolves `mod` DECLARATIONS only, module-dir-relative (see
/// [`module_dir`]) — `src/a.rs` + `mod b;` → `src/a/b.rs` / `src/a/b/mod.rs`
/// (rustc: a bare `src/b.rs` is error[E0583]). Inline-module nesting is carried
/// as a `/`-joined path prefix (`mod outer { mod inner; }` → raw
/// `outer/inner`), so its candidates are `<module_dir>/outer/inner.rs` /
/// `<module_dir>/outer/inner/mod.rs`.
///
/// Internal `use` edges (raw contains `::`) are CLASSIFIED but NOT resolved
/// (founder decision, round-4): they keep their raw `::` path with
/// `resolved=false` (they never match a file; R3 owns module-graph semantics).
/// A `mod` miss yields `to_path=null`, `resolved=false`. `Angle` edges
/// (external crates) are left untouched (raw name, unresolved).
fn resolve_rust_edges(unit: &mut FileUnit, root: &Path, rel_path: &str) {
    let module_dir = module_dir(rel_path);
    for edge in &mut unit.includes {
        if edge.form != IncludeForm::Quoted {
            continue;
        }
        let Some(raw) = edge.to_path.clone() else {
            continue;
        };
        // `use` path (not a `mod` declaration): preserved raw + unresolved.
        if raw.contains("::") {
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

/// Repo-relative directory of `rel_path` (empty for a root-level file).
fn includer_dir(rel_path: &str) -> String {
    match rel_path.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => String::new(),
    }
}

/// Rust module directory for `rel_path` (reviews N1 + round-3 + round-4).
/// Matches rustc: a CRATE-ROOT file resolves its submodules in its OWN
/// (containing) directory, while a submodule file `<dir>/<stem>.rs` owns the
/// `<dir>/<stem>/` subdirectory.
///
/// A path is a crate root when either:
/// 1. its file stem is `lib`/`main` (crate roots) or `mod` (a directory module
///    file, `mod.rs`), or
/// 2. it is a DIRECT child of a Cargo crate-root directory — `src/bin`,
///    `examples`, `tests`, `benches`. This is matched by TRAILING SEGMENT so
///    nested/workspace packages work too (`crates/foo/src/bin/bar.rs`,
///    `crates/foo/tests/x.rs`; see `is_crate_root_dir`).
///
/// Examples: `src/lib.rs` → `src`; `src/a/mod.rs` → `src/a`; `src/a.rs` →
/// `src/a`; `src/bin/foo.rs` → `src/bin`; `tests/x.rs` → `tests`;
/// `tests/common/mod.rs` → `tests/common`; `crates/foo/src/bin/bar.rs` →
/// `crates/foo/src/bin`; `src/bin/foo/bar.rs` → `src/bin/foo/bar`. A repo-root
/// `foo.rs` (single-file target) → `foo`.
fn module_dir(rel_path: &str) -> String {
    let (dir, file) = match rel_path.rsplit_once('/') {
        Some((dir, file)) => (dir, file),
        None => ("", rel_path),
    };
    let stem = file.strip_suffix(".rs").unwrap_or(file);
    if matches!(stem, "lib" | "main" | "mod") || is_crate_root_dir(dir) {
        dir.to_string()
    } else if dir.is_empty() {
        stem.to_string()
    } else {
        format!("{dir}/{stem}")
    }
}

/// Whether `dir` is (or ends with) a Cargo crate-root directory. Segment-based
/// so workspace layouts (`crates/foo/src/bin`) are recognized.
fn is_crate_root_dir(dir: &str) -> bool {
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
