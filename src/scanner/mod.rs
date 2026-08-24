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
//! - Language detection by extension: `.c`/`.h` are C in v1; others skipped
//!   silently. `scan.file_count` = `len(files[])`; zero-C-file scans still
//!   record `scan.languages: ["c"]`.
//! - Parse via the bound backend; capture ParseHealth (status, error regions,
//!   error node count).
//! - Include resolution against the filesystem (includer-relative first,
//!   then repo-root-relative) for quoted AND angle-bracket edges (ADR-0002:
//!   angle hits on in-repo files are internal coupling); unresolvable quoted
//!   includes emit `IncludeEdge{resolved:false, to_path:null}` — ARCH-001
//!   signal, not an error; unresolvable angle includes keep the raw include
//!   text in `to_path` with `resolved:false` (external bucket).
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

/// Scan a target path (directory or single `.c`/`.h` file) into the IR stream.
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
        let unit = match fs::read(abs) {
            Ok(source) => {
                let mut unit = extract_unit(rel, &source);
                resolve_includes(&mut unit, &include_root, rel);
                unit
            }
            Err(_) => failed_unit(rel),
        };
        units.push(unit);
    }
    Ok(units)
}

/// Recursively discover C sources under `root`. Skips dot-prefixed
/// directories, does not follow symlinks, keeps only `.c`/`.h` files
/// (modules.md §3.2); other extensions are skipped silently.
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
            } else if is_c_source(name) {
                out.push(path);
            }
        }
    }
    out
}

/// Language detection by extension: `.c` and `.h` are C in v1.
fn is_c_source(file_name: &str) -> bool {
    Path::new(file_name)
        .extension()
        .is_some_and(|ext| ext == "c" || ext == "h")
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

/// Extract one file's IR via the bound backend (tree-sitter-c in v1).
fn extract_unit(rel_path: &str, source: &[u8]) -> FileUnit {
    let ex = backends::tree_sitter_c::extract(rel_path, source);
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
fn failed_unit(rel_path: &str) -> FileUnit {
    FileUnit {
        file: FileRecord {
            path: rel_path.to_string(),
            language: "c".to_string(),
            loc: Default::default(),
            function_count: 0,
            global_var_count: 0,
            include_counts: Default::default(),
        },
        parse_health: ParseHealth {
            status: ParseStatus::Failed,
            error_regions: Vec::new(),
            error_node_count: 0,
        },
        ..FileUnit::default()
    }
}

/// Include-edge resolution against the filesystem (modules.md §3.2; ADR-0002
/// chart policy): BOTH quoted and angle-bracket edges are resolved with the
/// same two candidates — includer-relative first, then repo-root-relative.
/// A hit rewrites `to_path` to the repo-relative POSIX path and sets
/// `resolved=true` (angle hits count as internal coupling per ADR-0002). A
/// miss keeps the edge as data, not error: quoted ⇒ `to_path=null`
/// (`resolved=false`, ARCH-001 signal); angle ⇒ raw include text preserved in
/// `to_path` with `resolved=false` (external bucket).
///
/// The backend leaves raw include text in `to_path`; this pass replaces it
/// with the resolved repo-relative POSIX path or `None`.
fn resolve_includes(unit: &mut FileUnit, root: &Path, rel_path: &str) {
    let includer_dir = match rel_path.rsplit_once('/') {
        Some((dir, _)) => dir.to_string(),
        None => String::new(),
    };
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
