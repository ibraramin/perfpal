use crate::perf_runner::FuncRow;
use std::path::PathBuf;

/// Build the report's `hotspots` array from `perf report`-derived function rows.
pub fn build_hotspots(funcs: &[FuncRow]) -> Vec<crate::report::Hotspot> {
    funcs
        .iter()
        .enumerate()
        .take(50)
        .map(|(i, f)| crate::report::Hotspot {
            rank: (i + 1) as u32,
            function: f.symbol.clone(),
            file: f.file.as_ref().map(PathBuf::from),
            line: f.line,
            self_pct: f.self_pct,
            // Cumulative pct is not available from `perf report --no-children`.
            cumulative_pct: f.self_pct,
            samples: f.samples,
        })
        .collect()
}
