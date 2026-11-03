//! Story-only probes (Atlas Story, `docs/specs/atlas-story.md` §4).
//!
//! These run **only** when the caller asks for the story projection
//! (`--story` → `atlas-layout.json`); normal scans never reach this module.

pub(crate) mod flow;
pub(crate) mod io_table;
