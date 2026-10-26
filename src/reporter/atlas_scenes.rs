//! Story-only directory-scene aggregation (P8, `docs/plans/atlas-s4.md`).
//!
//! Partitions the scanned source files into scenes for 1k–10k-file repos:
//! root files form `(root)`, every top-level directory becomes a scene, and a
//! top-level directory is split into its immediate children when it spans more
//! than one child directory. Files at any deeper depth stay in their depth-2
//! scene. Costs come from the shared [`crate::reporter::atlas::file_cost`] rule
//! (the same one the drawn `AtlasNode.cost` uses), so scenes can never drift
//! from the Canvas.
//!
//! Story-only: [`crate::reporter::atlas_layout_json`] is the only caller, so
//! normal scans keep byte-identical output.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::reporter::atlas::{AtlasView, file_cost};
use crate::reporter::views::CostScale;
use crate::schema::{FileEntry, MetricsDocument};

/// `scene_version` — bumped only when the shape of the scenes block changes.
pub(crate) const SCENE_VERSION: u8 = 1;

/// Maximum number of scenes carried into the story.
const SCENE_CAP: usize = 24;

/// Maximum number of top files carried per scene.
const FILE_CAP: usize = 5;

/// The story-only `scenes` block as emitted into `atlas-layout.json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct SceneBlock {
    pub(crate) scene_version: u8,
    pub(crate) scene_cap: usize,
    pub(crate) file_cap: usize,
    pub(crate) total_scenes: usize,
    pub(crate) shown_scenes: usize,
    pub(crate) total_files: usize,
    pub(crate) shown_files: usize,
    pub(crate) rows: Vec<SceneRow>,
    pub(crate) omitted: SceneOmitted,
    /// Source path → scene key (never serialized; used to annotate actions).
    #[serde(skip)]
    pub(crate) scene_of: BTreeMap<String, String>,
}

/// One directory scene.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct SceneRow {
    pub(crate) path: String,
    pub(crate) depth: u8,
    pub(crate) files: usize,
    pub(crate) production_files: usize,
    pub(crate) findings: usize,
    pub(crate) cost: u64,
    pub(crate) production_cost: u64,
    /// `"v2"`, `"v1"` (all v1) or `"mixed"`.
    pub(crate) scale: String,
    pub(crate) entry: bool,
    pub(crate) hottest: bool,
    pub(crate) top_files: Vec<SceneFile>,
}

/// One file inside a scene's bounded top list.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct SceneFile {
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) cost: u64,
    pub(crate) scale: String,
    pub(crate) production: bool,
    pub(crate) findings: usize,
    pub(crate) function_count: u32,
    pub(crate) entry: bool,
    pub(crate) hottest: bool,
}

/// Totals over the scenes excluded by [`SCENE_CAP`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SceneOmitted {
    pub(crate) scenes: usize,
    pub(crate) files: usize,
    pub(crate) cost: u64,
    pub(crate) production_cost: u64,
    pub(crate) findings: usize,
}

struct SceneAccum {
    depth: u8,
    files: Vec<SceneFile>,
    cost: u64,
    production_cost: u64,
    production_files: usize,
    findings: usize,
}

impl SceneBlock {
    /// Scene key for `path` (source paths use the partition map; unknown
    /// paths fall back to the same top-level / depth-2 rule).
    #[must_use]
    pub(crate) fn scene_for(&self, path: &str) -> String {
        self.scene_of
            .get(path)
            .cloned()
            .unwrap_or_else(|| best_effort_scene(path).0)
    }
}

/// Build the scene block from the document plus the already-built Atlas view.
#[must_use]
pub(crate) fn build(doc: &MetricsDocument, view: &AtlasView) -> SceneBlock {
    let mut indices_by_path: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, function) in doc.stats.functions.iter().enumerate() {
        indices_by_path
            .entry(function.path.as_str())
            .or_default()
            .push(index);
    }

    let source_files: Vec<&FileEntry> = doc
        .files
        .iter()
        .filter(|file| file.function_count > 0)
        .collect();

    // `d1` → distinct immediate child directories (`d1/d2/...`).
    let mut child_dirs: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for file in &source_files {
        if let (Some(first), Some(second), Some(_)) = (
            file.path.split('/').next(),
            file.path.split('/').nth(1),
            file.path.split('/').nth(2),
        ) {
            child_dirs.entry(first).or_default().insert(second);
        }
    }

    let mut findings_by_path: BTreeMap<&str, usize> = BTreeMap::new();
    for finding in &doc.findings {
        *findings_by_path.entry(finding.path.as_str()).or_insert(0) += 1;
    }

    let entry_path = view.entry.point.as_ref().map(|point| point.path.as_str());
    let hottest_path = view.hottest.as_deref();

    let mut scenes: BTreeMap<String, SceneAccum> = BTreeMap::new();
    let mut scene_of: BTreeMap<String, String> = BTreeMap::new();

    for file in &source_files {
        let (key, depth) = partition(&file.path, &child_dirs);
        let indices: &[usize] = indices_by_path
            .get(file.path.as_str())
            .map_or(&[], Vec::as_slice);
        let (cost, _exact, scale) = file_cost(doc, indices);
        let production = file.path_class.as_deref() == Some("production");
        let findings = findings_by_path
            .get(file.path.as_str())
            .copied()
            .unwrap_or(0);
        let scene_file = SceneFile {
            path: file.path.clone(),
            name: basename(&file.path).to_string(),
            cost,
            scale: scale_token(scale).to_string(),
            production,
            findings,
            function_count: file.function_count,
            entry: entry_path == Some(file.path.as_str()),
            hottest: hottest_path == Some(file.path.as_str()),
        };
        let accum = scenes.entry(key.clone()).or_insert_with(|| SceneAccum {
            depth,
            files: Vec::new(),
            cost: 0,
            production_cost: 0,
            production_files: 0,
            findings: 0,
        });
        accum.cost = accum.cost.saturating_add(cost);
        if production {
            accum.production_cost = accum.production_cost.saturating_add(cost);
            accum.production_files += 1;
        }
        accum.findings = accum.findings.saturating_add(findings);
        accum.files.push(scene_file);
        scene_of.insert(file.path.clone(), key);
    }

    let entry_scene = entry_path.and_then(|path| scene_of.get(path));
    let hottest_scene = hottest_path.and_then(|path| scene_of.get(path));

    let mut rows: Vec<SceneRow> = scenes
        .into_iter()
        .map(|(path, mut accum)| {
            let scale = scene_scale(&accum.files);
            let total_cost = accum.cost;
            let total_production_cost = accum.production_cost;
            let total_findings = accum.findings;
            let files = accum.files.len();
            let production_files = accum.production_files;
            accum
                .files
                .sort_by(|a, b| b.cost.cmp(&a.cost).then_with(|| a.path.cmp(&b.path)));
            accum.files.truncate(FILE_CAP);
            SceneRow {
                entry: entry_scene.is_some_and(|key| key == &path),
                hottest: hottest_scene.is_some_and(|key| key == &path),
                path,
                depth: accum.depth,
                files,
                production_files,
                findings: total_findings,
                cost: total_cost,
                production_cost: total_production_cost,
                scale,
                top_files: accum.files,
            }
        })
        .collect();

    let total_scenes = rows.len();
    let total_files = source_files.len();
    let total_cost = rows
        .iter()
        .fold(0_u64, |sum, row| sum.saturating_add(row.cost));
    let total_production_cost = rows
        .iter()
        .fold(0_u64, |sum, row| sum.saturating_add(row.production_cost));
    let total_findings = rows
        .iter()
        .fold(0_usize, |sum, row| sum.saturating_add(row.findings));

    rows.sort_by(|a, b| {
        b.production_cost
            .cmp(&a.production_cost)
            .then_with(|| b.cost.cmp(&a.cost))
            .then_with(|| a.path.cmp(&b.path))
    });
    rows.truncate(SCENE_CAP);

    let shown_scenes = rows.len();
    let shown_files = rows.iter().map(|row| row.files).sum::<usize>();
    let shown_cost = rows
        .iter()
        .fold(0_u64, |sum, row| sum.saturating_add(row.cost));
    let shown_production_cost = rows
        .iter()
        .fold(0_u64, |sum, row| sum.saturating_add(row.production_cost));
    let shown_findings = rows
        .iter()
        .fold(0_usize, |sum, row| sum.saturating_add(row.findings));

    SceneBlock {
        scene_version: SCENE_VERSION,
        scene_cap: SCENE_CAP,
        file_cap: FILE_CAP,
        total_scenes,
        shown_scenes,
        total_files,
        shown_files,
        rows,
        omitted: SceneOmitted {
            scenes: total_scenes - shown_scenes,
            files: total_files - shown_files,
            cost: total_cost - shown_cost,
            production_cost: total_production_cost - shown_production_cost,
            findings: total_findings - shown_findings,
        },
        scene_of,
    }
}

/// Assign one source path to `(scene key, depth)`.
fn partition(path: &str, child_dirs: &BTreeMap<&str, BTreeSet<&str>>) -> (String, u8) {
    let mut parts = path.split('/');
    let Some(first) = parts.next() else {
        return ("(root)".to_string(), 0);
    };
    let Some(second) = parts.next() else {
        return ("(root)".to_string(), 0);
    };
    let deeper = parts.next().is_some();
    let children = child_dirs.get(first).map_or(0, BTreeSet::len);
    if children > 1 && deeper {
        (format!("{first}/{second}"), 2)
    } else {
        (first.to_string(), 1)
    }
}

fn best_effort_scene(path: &str) -> (String, u8) {
    let mut parts = path.split('/');
    let Some(first) = parts.next() else {
        return ("(root)".to_string(), 0);
    };
    let Some(second) = parts.next() else {
        return ("(root)".to_string(), 0);
    };
    if parts.next().is_some() {
        (format!("{first}/{second}"), 2)
    } else {
        (first.to_string(), 1)
    }
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn scale_token(scale: CostScale) -> &'static str {
    match scale {
        CostScale::V2 => "v2",
        CostScale::V1 => "v1",
    }
}

fn scene_scale(files: &[SceneFile]) -> String {
    let all_v2 = files.iter().all(|file| file.scale == "v2");
    let none_v2 = files.iter().all(|file| file.scale == "v1");
    if all_v2 {
        "v2".to_string()
    } else if none_v2 {
        "v1".to_string()
    } else {
        "mixed".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::Severity;
    use crate::reporter::atlas::build_atlas_view;
    use crate::reporter::atlas::test_support::{document, file, finding, function};
    use crate::schema::MetricsDocument;

    fn paths_fixture(paths: &[(&str, u64)]) -> MetricsDocument {
        let files = paths
            .iter()
            .map(|(path, _)| file(path, "production", 1))
            .collect();
        let functions = paths
            .iter()
            .map(|(path, cost)| function(path, "run", 1, Some(*cost), 0, None))
            .collect();
        document(files, functions, Vec::new(), Vec::new(), Vec::new())
    }

    fn partition_fixture() -> MetricsDocument {
        paths_fixture(&[
            ("main.c", 1),
            ("src/one.c", 2),
            ("src/two.c", 3),
            ("tools/direct.c", 4),
            ("tools/a/x.c", 5),
            ("tools/a/y.c", 6),
            ("tools/a/nested/deep.c", 7),
            ("tools/b/z.c", 8),
            ("lib/one/deep.c", 9),
            ("lib/direct.c", 10),
        ])
    }

    fn row<'a>(block: &'a SceneBlock, path: &str) -> &'a SceneRow {
        block
            .rows
            .iter()
            .find(|row| row.path == path)
            .unwrap_or_else(|| panic!("scene {path} not found in {:?}", block.rows))
    }

    fn cost_of(doc: &MetricsDocument, path: &str) -> u64 {
        let indices: Vec<usize> = doc
            .stats
            .functions
            .iter()
            .enumerate()
            .filter(|(_, function)| function.path == path)
            .map(|(index, _)| index)
            .collect();
        file_cost(doc, &indices).0
    }

    #[test]
    fn root_files_form_the_root_scene() {
        let doc = partition_fixture();
        let view = build_atlas_view(&doc);
        let block = build(&doc, &view);
        assert_eq!(block.scene_for("main.c"), "(root)");
        let root = row(&block, "(root)");
        assert_eq!(root.depth, 0);
        assert_eq!(root.files, 1);
        assert_eq!(root.production_files, 1);
        assert_eq!(root.top_files[0].path, "main.c");
        assert_eq!(root.top_files[0].name, "main.c");
    }

    #[test]
    fn top_level_dir_with_one_child_stays_one_scene() {
        let doc = partition_fixture();
        let view = build_atlas_view(&doc);
        let block = build(&doc, &view);
        // `lib` has exactly one immediate child (`one`) → all files stay in
        // the depth-1 `lib` scene.
        assert_eq!(block.scene_for("lib/one/deep.c"), "lib");
        assert_eq!(block.scene_for("lib/direct.c"), "lib");
        let lib = row(&block, "lib");
        assert_eq!(lib.depth, 1);
        assert_eq!(lib.files, 2);
    }

    #[test]
    fn two_child_dirs_split_into_parent_and_child_scenes() {
        let doc = partition_fixture();
        let view = build_atlas_view(&doc);
        let block = build(&doc, &view);
        assert_eq!(block.scene_for("tools/direct.c"), "tools");
        assert_eq!(block.scene_for("tools/a/x.c"), "tools/a");
        assert_eq!(block.scene_for("tools/b/z.c"), "tools/b");

        let parent = row(&block, "tools");
        assert_eq!(parent.depth, 1);
        assert_eq!(parent.files, 1);
        let a = row(&block, "tools/a");
        assert_eq!(a.depth, 2);
        assert_eq!(a.files, 3);
        let b = row(&block, "tools/b");
        assert_eq!(b.depth, 2);
        assert_eq!(b.files, 1);
    }

    #[test]
    fn deeper_files_stay_in_the_depth_two_scene() {
        let doc = partition_fixture();
        let view = build_atlas_view(&doc);
        let block = build(&doc, &view);
        let a = row(&block, "tools/a");
        assert!(
            a.top_files
                .iter()
                .any(|file| file.path == "tools/a/nested/deep.c"),
            "depth-3 file must stay in tools/a"
        );
        assert_eq!(block.scene_for("tools/a/nested/deep.c"), "tools/a");
    }

    #[test]
    fn scene_for_handles_actions_and_unknown_paths() {
        let doc = partition_fixture();
        let view = build_atlas_view(&doc);
        let block = build(&doc, &view);
        assert_eq!(block.scene_for("src/one.c"), "src");
        // Unknown path: same top-level / depth-2 fallback.
        assert_eq!(block.scene_for("src/missing.c"), "src");
        assert_eq!(block.scene_for("src/deep/missing.c"), "src/deep");
        assert_eq!(block.scene_for("missing.c"), "(root)");
    }

    #[test]
    fn file_cost_matches_every_drawn_node() {
        let doc = crate::reporter::atlas::test_support::application_fixture();
        let view = build_atlas_view(&doc);
        assert!(!view.nodes.is_empty(), "fixture must draw nodes");
        for node in &view.nodes {
            assert_eq!(
                node.cost,
                cost_of(&doc, &node.path),
                "node cost drifted for {}",
                node.path
            );
        }
    }

    #[test]
    fn v1_file_cost_matches_the_node_rule() {
        let doc = document(
            vec![file("src/v1.c", "production", 3)],
            vec![
                function("src/v1.c", "main", 1, None, 0, Some("main")),
                function("src/v1.c", "a", 5, Some(10), 0, None),
                function("src/v1.c", "b", 9, None, 0, None),
            ],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        let node = view
            .nodes
            .iter()
            .find(|node| node.path == "src/v1.c")
            .expect("v1 node must be drawn");
        let indices: Vec<usize> = (0..doc.stats.functions.len()).collect();
        let (cost, _, scale) = file_cost(&doc, &indices);
        assert_eq!(scale, CostScale::V1);
        assert_eq!(node.cost, cost);
        assert_eq!(node.cost, cost_of(&doc, "src/v1.c"));
    }

    #[test]
    fn findings_are_summed_per_scene() {
        let doc = document(
            vec![
                file("src/one.c", "production", 1),
                file("src/two.c", "production", 1),
                file("other/three.c", "production", 1),
            ],
            vec![
                function("src/one.c", "a", 1, Some(10), 0, None),
                function("src/two.c", "b", 1, Some(20), 0, None),
                function("other/three.c", "c", 1, Some(30), 0, None),
            ],
            vec![
                finding("PERF-C-LOOP-001", "src/one.c", 1, Severity::Hot, "one"),
                finding("PERF-C-LOOP-001", "src/one.c", 2, Severity::Warn, "one"),
                finding("PERF-C-LOOP-001", "src/two.c", 1, Severity::Info, "two"),
                finding(
                    "PERF-C-LOOP-001",
                    "other/three.c",
                    1,
                    Severity::Info,
                    "three",
                ),
            ],
            Vec::new(),
            Vec::new(),
        );
        let view = build_atlas_view(&doc);
        let block = build(&doc, &view);
        assert_eq!(row(&block, "src").findings, 3);
        assert_eq!(row(&block, "other").findings, 1);
        let one = row(&block, "src")
            .top_files
            .iter()
            .find(|file| file.path == "src/one.c")
            .expect("src/one.c in top files");
        assert_eq!(one.findings, 2);
    }

    #[test]
    fn scene_cap_and_file_cap_are_enforced() {
        let mut paths: Vec<(String, u64)> = (0..30)
            .map(|index| (format!("d{index:02}/f.c"), 1_000 - index as u64))
            .collect();
        // One extra scene with six files to exercise the per-scene top cap
        // (costs above every d* scene so it survives the 24-scene cut).
        for index in 0..6 {
            paths.push((format!("big/f{index}.c"), 2_000 + index as u64));
        }
        let refs: Vec<(&str, u64)> = paths
            .iter()
            .map(|(path, cost)| (path.as_str(), *cost))
            .collect();
        let doc = paths_fixture(&refs);
        let view = build_atlas_view(&doc);
        let block = build(&doc, &view);

        assert_eq!(block.total_scenes, 31);
        assert_eq!(block.shown_scenes, 24);
        assert_eq!(block.rows.len(), 24);
        assert_eq!(block.omitted.scenes, 7);
        assert!(block.rows.iter().all(|row| row.top_files.len() <= 5));
        let big = row(&block, "big");
        assert_eq!(big.files, 6);
        assert_eq!(big.top_files.len(), 5);
    }

    #[test]
    fn omitted_totals_reconcile() {
        let paths: Vec<(String, u64)> = (0..30)
            .map(|index| (format!("d{index:02}/f{index}.c"), 1_000 - index as u64))
            .collect();
        let refs: Vec<(&str, u64)> = paths
            .iter()
            .map(|(path, cost)| (path.as_str(), *cost))
            .collect();
        let doc = paths_fixture(&refs);
        let view = build_atlas_view(&doc);
        let block = build(&doc, &view);

        let shown_files: usize = block.rows.iter().map(|row| row.files).sum();
        let shown_cost: u64 = block.rows.iter().map(|row| row.cost).sum();
        assert_eq!(block.total_files, 30);
        assert_eq!(shown_files + block.omitted.files, block.total_files);
        assert_eq!(
            block.shown_scenes + block.omitted.scenes,
            block.total_scenes
        );
        assert_eq!(shown_cost + block.omitted.cost, 1_000 * 30 - 29 * 30 / 2);
        assert_eq!(block.shown_files, shown_files);
    }

    #[test]
    fn scene_build_is_deterministic() {
        let doc = partition_fixture();
        let view = build_atlas_view(&doc);
        let first = build(&doc, &view);
        let second = build(&doc, &view);
        assert_eq!(first, second);
        assert_eq!(
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        );
        assert!(!serde_json::to_string(&first).unwrap().contains("scene_of"));
    }
}
