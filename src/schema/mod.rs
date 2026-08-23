//! `schema` — metrics.json model & serializer (modules.md §3.5).
//!
//! Owns: the metrics.json document types; the serializer; the embedded
//! canonical JSON Schema artifact (byte-copy source for the bundled
//! `metrics.schema.v<MAJOR>.<MINOR>.json`); the SCHEMA_VERSION constant.
//!
//! Contract: other modules depend on these types only — never on ad-hoc JSON
//! maps. Version bumps follow metrics-json-v1 policy: minor = additive only,
//! major = breaking.
//!
//! Determinism guard: serde_json runs with default features — the
//! `preserve_order` feature stays OFF so object keys serialize in sorted
//! (BTreeMap) order (modules.md §6 row 3). Enforced by exact pin in
//! Cargo.toml, a CI grep, and the behavioral test in tests/determinism_guard.rs.
//!
//! P0 status: scaffold only. Document types + serializer land in P1.
