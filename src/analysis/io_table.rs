//! Structural-suite v1 external-I/O classifier (metrics schema 1.9).
//!
//! One shared construct across the five languages: a call that performs
//! external I/O against a **file**, **stdio**, or **network** target.
//! Environment access (`getenv`, `std::env::*`), process spawning (`system`),
//! regex compilation (`regcomp`), and in-memory formatting
//! (`sprintf`/`snprintf`) are deliberately excluded.
//!
//! C and Rust reuse the fixed, versioned table in [`crate::analysis::flow`]
//! (no duplicated names). Python and JavaScript/TypeScript use the tables
//! below; TypeScript/TSX inherit the JavaScript table. Each table is
//! versioned and membership changes require a version bump plus a fixture.
//!
//! Method-receiver heuristics (`read`/`write`/…) are intentionally narrow:
//! a method is classified only when its receiver root is one of the
//! documented file/stream roots. A differently named receiver is a documented
//! false negative, never a guessed positive.

use std::collections::BTreeSet;

use crate::analysis::flow;
use crate::ir::{CallEvent, CalleeKind};

/// Version of the Python external-I/O table. Bump on any membership change.
#[allow(dead_code)]
pub(crate) const PY_IO_TABLE_VERSION: u8 = 1;

/// Version of the JavaScript/TypeScript external-I/O table. Bump on any
/// membership change.
#[allow(dead_code)]
pub(crate) const JS_IO_TABLE_VERSION: u8 = 1;

/// Python bare-name file constructors.
const PY_BARE_FILE: [&str; 1] = ["open"];
/// Python bare-name stdio constructors.
const PY_BARE_STDIO: [&str; 1] = ["print"];
/// Python exact written paths for file I/O.
const PY_EXACT_FILE: [&str; 2] = ["os.read", "os.write"];
/// Python exact written paths for network I/O.
const PY_EXACT_NET: [&str; 1] = ["urllib.request.urlopen"];
/// Python dotted prefixes classified as network I/O.
const PY_NET_PREFIXES: [&str; 2] = ["socket.", "requests."];
/// Python `sys` stdio stream prefixes.
const PY_SYS_STDIO_PREFIXES: [&str; 3] = ["sys.stdin", "sys.stdout", "sys.stderr"];
/// Documented Python file/stream receiver roots for method I/O.
const PY_STREAM_ROOTS: [&str; 10] = [
    "f", "fp", "fh", "file", "stream", "sock", "conn", "stdin", "stdout", "stderr",
];
/// Python method names classified only on a documented stream receiver.
const PY_STREAM_METHODS: [&str; 4] = ["read", "write", "readline", "writelines"];

/// JavaScript/TypeScript file-I/O dotted prefixes.
const JS_FILE_PREFIXES: [&str; 1] = ["fs."];
/// JavaScript/TypeScript stdio dotted prefixes.
const JS_STDIO_PREFIXES: [&str; 3] = ["console.", "process.stdout.", "process.stderr."];
/// JavaScript/TypeScript network dotted prefixes.
const JS_NET_PREFIXES: [&str; 3] = ["net.", "http.", "https."];
/// JavaScript/TypeScript global network constructors (bare names).
const JS_NET_BARE: [&str; 2] = ["fetch", "XMLHttpRequest"];
/// Documented JavaScript/TypeScript stream receiver roots for method I/O.
const JS_STREAM_ROOTS: [&str; 6] = ["stdout", "stderr", "stream", "socket", "sock", "conn"];
/// JavaScript/TypeScript method names classified only on a stream receiver.
const JS_STREAM_METHODS: [&str; 2] = ["read", "write"];

/// Whether one call is an external-I/O site under the language's fixed table.
///
/// `c_defined_names` is the whole-set set of C function names: a bare C I/O
/// name shadowed by a drawn definition is a user wrapper and is never
/// classified (the same guard the flow projection applies).
#[must_use]
pub(crate) fn is_external_io(
    language: &str,
    call: &CallEvent,
    c_defined_names: &BTreeSet<String>,
) -> bool {
    match language {
        "c" => {
            call.callee_kind == CalleeKind::Direct
                && !c_defined_names.contains(&call.callee_name)
                && flow::matches_external_io_rule(&call.callee_name, false, true)
        }
        "rust" => {
            if call.is_macro {
                flow::matches_external_io_rule(&call.callee_name, true, false)
            } else {
                call.callee_path
                    .as_deref()
                    .is_some_and(|path| flow::matches_external_io_rule(path, false, false))
            }
        }
        "python" => python_external_io(call),
        "js" | "ts" | "tsx" => javascript_external_io(call),
        _ => false,
    }
}

fn python_external_io(call: &CallEvent) -> bool {
    let name = call.callee_name.as_str();
    let path = call.callee_path.as_deref().unwrap_or("");
    if PY_BARE_FILE.contains(&name) || PY_BARE_STDIO.contains(&name) {
        return true;
    }
    if PY_EXACT_FILE.contains(&path) || PY_EXACT_NET.contains(&path) {
        return true;
    }
    if PY_NET_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        return true;
    }
    if PY_SYS_STDIO_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        return true;
    }
    if PY_STREAM_METHODS.contains(&name) {
        return call
            .receiver_root
            .as_deref()
            .is_some_and(|root| PY_STREAM_ROOTS.contains(&root));
    }
    false
}

fn javascript_external_io(call: &CallEvent) -> bool {
    let name = call.callee_name.as_str();
    let path = call.callee_path.as_deref().unwrap_or("");
    if JS_FILE_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        return true;
    }
    if JS_STDIO_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
    {
        return true;
    }
    if JS_NET_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
        || JS_NET_BARE.contains(&name)
    {
        return true;
    }
    if JS_STREAM_METHODS.contains(&name) {
        return call
            .receiver_root
            .as_deref()
            .is_some_and(|root| JS_STREAM_ROOTS.contains(&root));
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{EventMeta, ResolverTier, Shape, Span, StatementRole};

    fn call(name: &str, path: Option<&str>, receiver: Option<&str>) -> CallEvent {
        CallEvent {
            meta: EventMeta {
                span: Span::point(1, 1),
                in_loop_depth: 1,
                is_innermost: true,
                resolver_tier: ResolverTier::Syntactic,
                macro_origin: false,
            },
            callee_name: name.to_string(),
            callee_kind: CalleeKind::Direct,
            callee_path: path.map(str::to_string),
            is_macro: false,
            args: Vec::<Shape>::new(),
            statement_role: StatementRole::Unknown,
            conditional_guarded: false,
            semantic: None,
            receiver_root: receiver.map(str::to_string),
        }
    }

    #[test]
    fn python_table_is_narrow_on_method_receivers() {
        let none = BTreeSet::new();
        assert!(is_external_io(
            "python",
            &call("open", Some("open"), None),
            &none
        ));
        assert!(is_external_io(
            "python",
            &call("print", Some("print"), None),
            &none
        ));
        assert!(is_external_io(
            "python",
            &call("read", Some("os.read"), None),
            &none
        ));
        assert!(is_external_io(
            "python",
            &call("urlopen", Some("urllib.request.urlopen"), None),
            &none
        ));
        assert!(is_external_io(
            "python",
            &call("write", Some("sys.stdout.write"), Some("sys")),
            &none
        ));
        assert!(is_external_io(
            "python",
            &call("read", Some("f.read"), Some("f")),
            &none
        ));
        assert!(!is_external_io(
            "python",
            &call("read", Some("obj.read"), Some("obj")),
            &none
        ));
        assert!(!is_external_io(
            "python",
            &call("getenv", Some("os.getenv"), None),
            &none
        ));
    }

    #[test]
    fn javascript_table_covers_fs_stdio_net_and_narrow_streams() {
        let none = BTreeSet::new();
        assert!(is_external_io(
            "js",
            &call("readFileSync", Some("fs.readFileSync"), Some("fs")),
            &none
        ));
        assert!(is_external_io(
            "ts",
            &call("log", Some("console.log"), Some("console")),
            &none
        ));
        assert!(is_external_io(
            "js",
            &call("write", Some("process.stdout.write"), Some("process")),
            &none
        ));
        assert!(is_external_io(
            "js",
            &call("fetch", Some("fetch"), None),
            &none
        ));
        assert!(is_external_io(
            "js",
            &call("write", Some("stream.write"), Some("stream")),
            &none
        ));
        assert!(!is_external_io(
            "js",
            &call("write", Some("obj.write"), Some("obj")),
            &none
        ));
        assert!(!is_external_io(
            "js",
            &call("system", Some("system"), None),
            &none
        ));
    }

    #[test]
    fn c_and_rust_reuse_the_flow_table() {
        let defined: BTreeSet<String> = ["printf".to_string()].into_iter().collect();
        assert!(!is_external_io("c", &call("printf", None, None), &defined));
        assert!(is_external_io("c", &call("fprintf", None, None), &defined));
        assert!(!is_external_io("c", &call("getenv", None, None), &defined));
        assert!(!is_external_io(
            "c",
            &call("system", None, None),
            &BTreeSet::new()
        ));
        assert!(is_external_io(
            "rust",
            &call("read_to_string", Some("std::fs::read_to_string"), None),
            &BTreeSet::new()
        ));
        let mut macro_call = call("println", Some("println"), None);
        macro_call.is_macro = true;
        assert!(is_external_io("rust", &macro_call, &BTreeSet::new()));
        let mut plain_write = call("write", Some("write"), None);
        plain_write.is_macro = false;
        assert!(!is_external_io("rust", &plain_write, &BTreeSet::new()));
    }
}
