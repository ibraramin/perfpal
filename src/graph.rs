//! Deterministic graph utilities shared across seams (ADR-0002).
//!
//! Consumers: PERF-C-ARCH-001 (include-graph cycle breaking and depth
//! layering, now) and the chart emitters (later wave). Everything here is a
//! pure function of its inputs — no I/O, no clock, no global state — and
//! byte-deterministic: identical inputs yield identical outputs on every run
//! (modules.md §6 row 3).
//!
//! Cycle policy (ADR-0002): sort edges by `(from, to)`, then drop the
//! lowest-sorted edge that closes a cycle; self-loops are dropped too.
//! Broken edges remain VISIBLE — they surface as ARCH-001 findings, never
//! silently discarded.

use std::collections::{BTreeMap, BTreeSet};

/// Outcome of [`break_cycles`]: the acyclic edge set plus the edges dropped
/// to make it acyclic (kept in drop order — lowest-sorted closing edge
/// first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleBreakResult {
    /// Edges forming a DAG (deduped, sorted by `(from, to)`).
    pub kept: Vec<(String, String)>,
    /// Edges dropped because they closed a cycle or were self-loops,
    /// in drop order.
    pub broken: Vec<(String, String)>,
}

/// Break cycles in an edge set per ADR-0002: sort by `(from, to)`, walk the
/// sorted order, and drop any edge that CLOSES A DIRECTED CYCLE — i.e. an
/// edge `(u, v)` whose target `v` can already reach its source `u` through
/// previously kept edges — plus every self-loop.
///
/// Note on the mechanism: ADR-0002's sketch mentions union-find, but
/// union-find tracks UNDIRECTED connectivity and would wrongly drop DAG
/// edges (a diamond's second arm shares both endpoints' component without
/// any directed cycle existing). Directed reachability implements the
/// stated POLICY exactly — "drop the lowest-sorted edge that closes a
/// cycle" where a cycle is a directed one — and keeps diamonds intact.
///
/// Contract: identical `(from, to)` pairs are DEDUPED before processing,
/// keeping the FIRST occurrence — callers anchor findings at the earliest
/// span of each pair, so first-occurrence retention keeps anchors stable.
///
/// Determinism: the result depends only on the input pair set, never on
/// iteration order of the caller's collection.
#[must_use]
pub fn break_cycles(edges: &[(String, String)]) -> CycleBreakResult {
    // Dedupe identical pairs, keeping the first occurrence (the pair itself
    // is identical, so only cardinality changes).
    let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut deduped: Vec<&(String, String)> = Vec::with_capacity(edges.len());
    for edge in edges {
        if seen.insert((edge.0.as_str(), edge.1.as_str())) {
            deduped.push(edge);
        }
    }

    // Sorted order is load-bearing: the LOWEST-sorted edge closing a cycle
    // is the one dropped (ADR-0002).
    let mut sorted = deduped;
    sorted.sort();

    let mut kept: Vec<(String, String)> = Vec::new();
    let mut broken: Vec<(String, String)> = Vec::new();
    // Adjacency of the KEPT set, grown incrementally; used for the
    // "can v already reach u?" directed-cycle probe below.
    let mut adj: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for &(from, to) in &sorted {
        if from == to || reaches(&adj, to, from) {
            // Self-loop or directed cycle closed: drop (stays visible via
            // `broken`).
            broken.push((from.clone(), to.clone()));
            continue;
        }
        adj.entry(from).or_default().push(to);
        kept.push((from.clone(), to.clone()));
    }

    CycleBreakResult { kept, broken }
}

/// True when `from` can reach `to` following `adj` edges (BFS).
fn reaches(adj: &BTreeMap<&str, Vec<&str>>, from: &str, to: &str) -> bool {
    let mut frontier = vec![from];
    let mut visited: BTreeSet<&str> = BTreeSet::new();
    while let Some(node) = frontier.pop() {
        if node == to {
            return true;
        }
        if visited.insert(node)
            && let Some(next) = adj.get(node)
        {
            frontier.extend(next.iter().copied());
        }
    }
    false
}

/// Longest-path layering over a DAG (ADR-0002 chart layout input):
/// `layer(n) = 0` when `n` has no predecessors in the edge set, otherwise
/// `1 + max(layer(pred))`. Members within each layer are sorted ascending;
/// layers themselves ascend from 0.
///
/// Edges referencing nodes absent from `nodes` are ignored (the node list is
/// authoritative). The input MUST be acyclic — feed it [`break_cycles`]'s
/// `kept` set; a cyclic input yields a best-effort partial layering bounded
/// by `nodes.len()` passes rather than a panic.
#[must_use]
pub fn topo_layers(nodes: &[String], dag_edges: &[(String, String)]) -> Vec<Vec<String>> {
    let node_set: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
    // Predecessor lists restricted to known nodes.
    let mut preds: BTreeMap<&str, Vec<&str>> =
        nodes.iter().map(|n| (n.as_str(), Vec::new())).collect();
    for (from, to) in dag_edges {
        if let Some(list) = preds.get_mut(to.as_str())
            && node_set.contains(from.as_str())
            && !list.contains(&from.as_str())
        {
            list.push(from.as_str());
        }
    }

    // Relax to fixpoint, bounded by |nodes| passes (longest path visits a
    // node at most once; deterministic because predecessor lists are keyed
    // by BTreeMap and each pass only raises layers monotonically).
    let mut layer: BTreeMap<&str, u32> = nodes.iter().map(|n| (n.as_str(), 0)).collect();
    for _ in 0..nodes.len() {
        let mut changed = false;
        for node in nodes {
            let next = preds
                .get(node.as_str())
                .and_then(|ps| {
                    ps.iter()
                        .map(|p| layer[p])
                        .max()
                        .map(|max_pred| max_pred + 1)
                })
                .unwrap_or(0); // no predecessors ⇒ layer 0
            if next > layer[node.as_str()] {
                layer.insert(node.as_str(), next);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let depth = layer.values().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<String>> = vec![Vec::new(); depth as usize + 1];
    for node in nodes {
        layers[layer[node.as_str()] as usize].push(node.clone());
    }
    for members in &mut layers {
        members.sort();
    }
    layers
}

#[cfg(test)]
mod tests {
    //! In-module locks: cycle breaking picks the lowest-sorted closing edge,
    //! self-loops land in `broken`, DAGs survive intact, layering follows
    //! longest paths, and everything is deterministic across runs.

    use super::*;

    fn edges(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(f, t)| ((*f).to_string(), (*t).to_string()))
            .collect()
    }

    #[test]
    fn two_node_cycle_breaks_lowest_sorted_edge() {
        // Sorted: ("a","b") < ("b","a"); ("a","b") unions, ("b","a") closes.
        let result = break_cycles(&edges(&[("b", "a"), ("a", "b")]));
        assert_eq!(result.kept, [("a".to_string(), "b".to_string())]);
        assert_eq!(result.broken, [("b".to_string(), "a".to_string())]);
    }

    #[test]
    fn self_loop_lands_in_broken() {
        let result = break_cycles(&edges(&[("a", "a")]));
        assert!(result.kept.is_empty());
        assert_eq!(result.broken, [("a".to_string(), "a".to_string())]);
    }

    #[test]
    fn duplicate_pairs_collapse_to_first() {
        let result = break_cycles(&edges(&[("a", "b"), ("a", "b")]));
        assert_eq!(
            result.kept,
            [("a".to_string(), "b".to_string())],
            "identical pairs dedupe keeping one instance"
        );
        assert!(result.broken.is_empty());
    }

    #[test]
    fn diamond_dag_keeps_all_edges() {
        let result = break_cycles(&edges(&[
            ("top", "left"),
            ("top", "right"),
            ("left", "bottom"),
            ("right", "bottom"),
        ]));
        assert_eq!(result.broken, Vec::new(), "diamond has no cycle");
        assert_eq!(
            result.kept,
            // `kept` is emitted in (from, to) sorted order.
            edges(&[
                ("left", "bottom"),
                ("right", "bottom"),
                ("top", "left"),
                ("top", "right"),
            ])
        );
    }

    #[test]
    fn three_node_cycle_drops_exactly_one_edge() {
        // Sorted: (a,b),(a,c),(b,c),(c,a); (c,a) is the only closing edge.
        let result = break_cycles(&edges(&[("c", "a"), ("b", "c"), ("a", "b"), ("a", "c")]));
        assert_eq!(result.kept.len(), 3);
        assert_eq!(result.broken, [("c".to_string(), "a".to_string())]);
    }

    #[test]
    fn longest_path_layering_matches_definition() {
        // top → left → bottom, top → right → bottom: left/right at layer 1,
        // bottom at 2 despite also being reachable in 2 hops via either side.
        let nodes: Vec<String> = ["bottom", "left", "right", "top"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let dag = edges(&[
            ("top", "left"),
            ("top", "right"),
            ("left", "bottom"),
            ("right", "bottom"),
        ]);
        let layers = topo_layers(&nodes, &dag);
        assert_eq!(
            layers,
            [
                vec!["top".to_string()],
                vec!["left".to_string(), "right".to_string()],
                vec!["bottom".to_string()],
            ]
        );
    }

    #[test]
    fn isolated_nodes_sit_at_layer_zero() {
        let nodes = vec!["lone".to_string()];
        let layers = topo_layers(&nodes, &[]);
        assert_eq!(layers, [vec!["lone".to_string()]]);
    }

    #[test]
    fn same_input_twice_yields_equal_output() {
        let input = edges(&[("b", "a"), ("a", "b"), ("a", "b"), ("c", "a")]);
        let first = break_cycles(&input);
        let second = break_cycles(&input);
        assert_eq!(
            first, second,
            "determinism: identical input ⇒ identical output"
        );
        let dag = first.kept.clone();
        let nodes: Vec<String> = ["a".to_string(), "b".to_string(), "c".to_string()].to_vec();
        assert_eq!(topo_layers(&nodes, &dag), topo_layers(&nodes, &dag));
    }
}
