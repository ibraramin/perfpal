//! Rust backend R3 workstream A unit tests (plan `rust-backend`).
//!
//! Locks the additive IR + scanner behavior for: `CallEvent.callee_path` /
//! `is_macro`, the macro-as-call surface, `RUST_ALLOC_FAMILY`
//! `alloc_site_count`, the alloc-wrapper `RecognizedPattern` recognizer,
//! `macro_origin` wiring, and `StructKind` / member-flag derivation for
//! structs, unions and enums. Exercises the public backend seam
//! (`tree_sitter_rust::extract`) through the real grammar.

use perfpal::ir::{ALLOC_WRAPPER_PATTERN_ID, CallEvent, MacroEventKind, StructDef, StructKind};
use perfpal::scanner::backends::tree_sitter_rust;

const SRC: &str = r#"
use std::collections::HashMap;

struct Point {
    x: i64,
    y: i64,
}

struct Holder(&'static str, *const u8, [u8; 16]);

union U {
    a: i32,
    b: f32,
}

enum E {
    A,
    B(i32),
}

macro_rules! alloc_wrap {
    ($x:expr) => { Box::new($x) };
}

macro_rules! twice {
    ($x:expr) => { $x * 2 };
}

fn build(n: i32, items: &[i32]) -> i32 {
    let b = Box::new(n);
    let v = Vec::with_capacity(n as usize);
    let owned = n.to_string();
    let copied = items.to_vec();
    let collected: Vec<i32> = items.iter().collect();
    let fresh = String::new();
    let size = std::mem::size_of::<i64>();
    helper(n);
    let mac = vec![1, 2, 3];
    let fmt = format!("{}", n);
    let wrapped = alloc_wrap!(n);
    let doubled = twice!(n);
    println!("{}", wrapped);
    (b, v, owned, copied, collected, fresh, size, mac, fmt, doubled);
    mac.len() as i32
}
"#;

fn build_calls() -> Vec<CallEvent> {
    let ex = tree_sitter_rust::extract("src/r3.rs", SRC.as_bytes());
    let build = ex
        .functions
        .iter()
        .find(|f| f.record.name == "build")
        .expect("build function");
    build.calls.clone()
}

fn call_with_path<'a>(calls: &'a [CallEvent], path: &str) -> &'a CallEvent {
    calls
        .iter()
        .find(|c| c.callee_path.as_deref() == Some(path))
        .unwrap_or_else(|| panic!("no call with callee_path {path}"))
}

#[test]
fn callee_path_is_populated_for_scoped_method_and_generic_callees() {
    let calls = build_calls();
    // scoped_identifier qualified constructor.
    let string_new = call_with_path(&calls, "String::new");
    assert_eq!(string_new.callee_name, "new");
    // method (field_expression) callee.
    let to_vec = call_with_path(&calls, "items.to_vec");
    assert_eq!(to_vec.callee_name, "to_vec");
    // generic_function: the turbofish is stripped from the path text.
    let size_of = call_with_path(&calls, "std::mem::size_of");
    assert_eq!(size_of.callee_name, "size_of");
    // method chain off a call expression.
    let collect = call_with_path(&calls, "items.iter().collect");
    assert_eq!(collect.callee_name, "collect");
    // plain identifier path equals the name.
    let helper = call_with_path(&calls, "helper");
    assert_eq!(helper.callee_name, "helper");
    // Real (non-macro) calls are never flagged as macros.
    for real in [string_new, to_vec, size_of, collect, helper] {
        assert!(!real.is_macro);
    }
}

#[test]
fn macro_invocations_emit_macro_as_call_events() {
    let calls = build_calls();
    let println = call_with_path(&calls, "println");
    assert!(println.is_macro);
    assert_eq!(println.callee_name, "println");
    assert!(println.args.is_empty(), "token_tree args are opaque");
    let vec_call = call_with_path(&calls, "vec");
    assert!(vec_call.is_macro);
    let format = call_with_path(&calls, "format");
    assert!(format.is_macro);
    // The D7 MacroEvent `Use` emission is retained alongside the call event.
    let ex = tree_sitter_rust::extract("src/r3.rs", SRC.as_bytes());
    assert!(
        ex.macros
            .iter()
            .any(|m| m.kind == MacroEventKind::Use && m.name == "vec")
    );
}

#[test]
fn alloc_site_count_counts_qualified_method_and_macro_sites() {
    let ex = tree_sitter_rust::extract("src/r3.rs", SRC.as_bytes());
    let build = ex
        .functions
        .iter()
        .find(|f| f.record.name == "build")
        .unwrap();
    // Box::new, Vec::with_capacity, to_string, to_vec, collect, String::new,
    // vec!, format! = 8. `alloc_wrap!`/`twice!`/`println!`/size_of/helper are
    // NOT in the family.
    assert_eq!(build.record.alloc_site_count, 8);
}

#[test]
fn wrapper_recognizer_emits_recognized_pattern_for_alloc_body() {
    let ex = tree_sitter_rust::extract("src/r3.rs", SRC.as_bytes());
    let wrap = ex
        .macros
        .iter()
        .find(|m| m.kind == MacroEventKind::RecognizedPattern && m.name == "alloc_wrap")
        .expect("alloc_wrap recognized");
    assert_eq!(wrap.pattern_id.as_deref(), Some(ALLOC_WRAPPER_PATTERN_ID));
    // `twice` has no alloc-family call in its body: definition only.
    assert!(
        !ex.macros
            .iter()
            .any(|m| m.kind == MacroEventKind::RecognizedPattern && m.name == "twice")
    );
}

#[test]
fn macro_origin_true_via_local_macro_name_and_span_intersection() {
    // (a) callee-name branch: the macro-as-call event for a LOCAL macro
    // definition name (`alloc_wrap`) is macro-origin.
    let calls = build_calls();
    let wrapped = call_with_path(&calls, "alloc_wrap");
    assert!(
        wrapped.meta.macro_origin,
        "call to a local macro_definition name is macro-origin"
    );
    assert!(wrapped.is_macro);
    // A std macro invocation with no local definition is NOT macro-origin.
    let println = call_with_path(&calls, "println");
    assert!(!println.meta.macro_origin);

    // (b) span-intersection branch: a recognized wrapper macro_definition
    // nested inside a loop makes the enclosing LoopNode macro-origin (its
    // span contains the recognized-pattern span).
    let src = concat!(
        "fn f() {\n",
        "    for i in 0..3 {\n",
        "        macro_rules! inner_wrap { ($x:expr) => { Vec::new() }; }\n",
        "        let _ = i;\n",
        "    }\n",
        "}\n",
    );
    let ex = tree_sitter_rust::extract("span_origin.rs", src.as_bytes());
    assert!(
        ex.macros
            .iter()
            .any(|m| m.kind == MacroEventKind::RecognizedPattern && m.name == "inner_wrap"),
        "nested alloc wrapper is recognized"
    );
    let f = &ex.functions[0];
    assert_eq!(f.loops.len(), 1);
    assert!(
        f.loops[0].macro_origin,
        "loop span intersects a recognized wrapper span"
    );
}

#[test]
fn struct_union_enum_emit_kind_and_member_flags() {
    let ex = tree_sitter_rust::extract("src/r3.rs", SRC.as_bytes());
    let point = ex.structs.iter().find(|s| s.name == "Point").unwrap();
    assert_eq!(point.kind, StructKind::Struct);
    assert_eq!(point.members.len(), 2);

    // Tuple struct member flags: `&str`, `*const u8`, `[u8; 16]`.
    let holder = ex.structs.iter().find(|s| s.name == "Holder").unwrap();
    assert_eq!(holder.kind, StructKind::Struct);
    assert!(holder.members[0].is_pointer, "&'static str is a reference");
    assert!(holder.members[1].is_pointer, "*const u8 is a pointer");
    assert_eq!(holder.members[2].array_dim_literal, Some(16));

    let union = ex.structs.iter().find(|s| s.name == "U").unwrap();
    assert_eq!(union.kind, StructKind::Union);
    assert_eq!(union.members.len(), 2, "union members from its field list");

    let enumeration = ex.structs.iter().find(|s| s.name == "E").unwrap();
    assert_eq!(enumeration.kind, StructKind::Enum);
    assert!(enumeration.members.is_empty());
}

#[test]
fn named_member_flags_derive_from_type_shapes() {
    let src = "struct S { a: &str, b: *const u8, c: [u8; 16], d: [u8; N] }\n";
    let ex = tree_sitter_rust::extract("flags.rs", src.as_bytes());
    let s = ex.structs.iter().find(|s| s.name == "S").unwrap();
    assert!(s.members[0].is_pointer);
    assert!(s.members[1].is_pointer);
    assert_eq!(s.members[2].array_dim_literal, Some(16));
    assert_eq!(s.members[3].array_dim_literal, None);
}

/// Additive-field backwards compatibility: documents written before R3
/// deserialize with serde defaults (no schema change required).
#[test]
fn additive_ir_fields_deserialize_with_defaults() {
    // StructDef without `kind` → Struct.
    let def: StructDef = serde_json::from_str(
        r#"{"name":"S","path":"p.c","span":{"start_line":1,"start_col":1,"end_line":1,"end_col":2},"members":[]}"#,
    )
    .expect("pre-R3 StructDef deserializes");
    assert_eq!(def.kind, StructKind::Struct);

    // CallEvent without `callee_path`/`is_macro` → None / false.
    let call: CallEvent = serde_json::from_str(
        r#"{"span":{"start_line":1,"start_col":1,"end_line":1,"end_col":2},
            "in_loop_depth":0,"is_innermost":false,"resolver_tier":"syntactic",
            "macro_origin":false,"callee_name":"malloc","callee_kind":"direct",
            "args":[],"statement_role":"unknown","conditional_guarded":false}"#,
    )
    .expect("pre-R3 CallEvent deserializes");
    assert_eq!(call.callee_path, None);
    assert!(!call.is_macro);

    // Enum serialization uses snake_case.
    assert_eq!(
        serde_json::to_value(StructKind::Union).unwrap(),
        serde_json::json!("union")
    );
    assert_eq!(
        serde_json::to_value(StructKind::Enum).unwrap(),
        serde_json::json!("enum")
    );
    assert_eq!(
        serde_json::to_value(StructKind::Struct).unwrap(),
        serde_json::json!("struct")
    );
}
