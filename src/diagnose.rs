use crate::demangle;
use crate::perf_runner;
use crate::report::{
    Binary, Bottleneck, BottleneckKind, Environment, Hotspot, Report, Sampling, WeightedStack,
    Workload, SCHEMA_VERSION,
};
use crate::stacks;
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;
use uuid::Uuid;

pub fn run(args: crate::cli::DiagnoseArgs) -> Result<()> {
    let (report, path) = run_report(args)?;
    println!("Top 5 hotspots (self-time):");
    for h in report.hotspots.iter().take(5) {
        let loc = match (&h.file, h.line) {
            (Some(f), Some(l)) => format!("{}:{}", f.display(), l),
            _ => "<no location>".to_string(),
        };
        println!(
            "  #{:<2} {:6.2}%  {}  ({})",
            h.rank, h.self_pct, h.function, loc
        );
    }
    if !report.bottlenecks.is_empty() {
        println!("\nBottlenecks ({}):", report.bottlenecks.len());
        for b in &report.bottlenecks {
            let loc = match (&b.file, b.line) {
                (Some(f), Some(l)) => format!("{}:{}", f.display(), l),
                _ => "<no location>".to_string(),
            };
            println!(
                "  [{:?}] {} {}  metric={} value={:.2}  {}",
                b.kind, loc, b.function, b.metric, b.value, b.note
            );
        }
    }
    eprintln!("perfpal: wrote {}", path.display());
    Ok(())
}

/// Run the full pipeline and return the `Report` along with the on-disk path
/// where it was written. The CLI and the MCP tool both call this.
pub fn run_report(args: crate::cli::DiagnoseArgs) -> Result<(Report, std::path::PathBuf)> {
    let event_spec = if args.memory_pressure {
        "cycles,cache-misses"
    } else {
        "cycles:u"
    };
    eprintln!(
        "perfpal: profiling {} ({} sample/s, events={}) for up to {}s",
        args.binary.display(),
        args.sample_rate,
        event_spec,
        args.duration
    );

    let raw = perf_runner::record_and_parse(
        &args.binary,
        &args.args,
        args.sample_rate,
        args.duration,
        event_spec,
    )?;
    eprintln!(
        "perfpal: captured {} events, primary {} functions, total samples={} in {:.2}s",
        raw.events.len(),
        raw.functions.len(),
        raw.total_samples,
        raw.duration_s
    );

    // Demangle function names. perf --stdio may or may not emit demangled
    // names depending on locale/options; we demangle to be safe.
    let demangler = args.demangler.as_deref();
    let mut funcs = raw.functions.clone();
    for f in funcs.iter_mut() {
        f.symbol = demangle::demangle(&f.symbol, demangler);
    }
    // Demangle the other event blocks too (cache-misses) for the memory-pressure heuristic.
    let mut events = raw.events.clone();
    for b in events.iter_mut() {
        for f in b.functions.iter_mut() {
            f.symbol = demangle::demangle(&f.symbol, demangler);
        }
    }

    let env = Environment {
        kernel: read_kernel_release(),
        perf_paranoid: read_perf_paranoid(),
        cpu: read_cpu_model(),
        perf_version: raw.perf_version.clone(),
    };
    let sampling = Sampling {
        rate_hz: args.sample_rate,
        samples: raw.total_samples,
        duration_s: raw.duration_s,
        counters: events.iter().map(|b| b.event.clone()).collect(),
    };
    let workload = Workload {
        name: workload_name(&args.binary, &args.args),
        args: args.args.clone(),
        duration_s: raw.duration_s,
    };
    let binary = Binary {
        path: args.binary.clone(),
        build_id: read_build_id(&args.binary),
        demangled: true,
    };

    let hotspots: Vec<Hotspot> = stacks::build_hotspots(&funcs);
    // W1: stacks is empty. Stack-level data is W2.
    let stacks: Vec<WeightedStack> = Vec::new();

    let bottlenecks = apply_heuristics(&funcs, &events, &args.exclude_prefixes, args.min_self_pct);

    let report = Report {
        schema_version: SCHEMA_VERSION,
        session_id: Uuid::new_v4().to_string(),
        binary,
        workload,
        environment: env,
        sampling,
        hotspots,
        stacks,
        bottlenecks,
    };

    let path = report.write_to(&args.output_dir)?;
    Ok((report, path))
}

fn is_user_code(symbol: &str, excludes: &[String]) -> bool {
    !excludes.iter().any(|p| symbol.starts_with(p))
        && !symbol.starts_with("0x")
        && !symbol.starts_with("[k]")
        && !symbol.is_empty()
}

/// Heuristics 1-5 — the bottleneck detection core.
///
/// All heuristics read from the function-level table produced by
/// `perf report --stdio -n --no-children -g none`. We do NOT have per-sample
/// stacks on this kernel+perf combo, so the heuristics operate on the
/// collapsed function table.
///
/// 1. HotFunction     — top self_pct functions over `min_self_pct` threshold.
/// 2. TailFunction    — top self_pct user-code functions with resolved
///                      file/line. This is a "user tail" — the leaf where
///                      the work actually happens, not a library leaf.
/// 3. Dominator       — per user file, the top self_pct function. The
///                      function that "dominates" its file's hotness.
/// 4. HotFile         — aggregated self_pct by file path. Top files by
///                      total self_pct.
/// 5. MemoryPressure  — when a cache-misses event block is present, top
///                      functions by cache_miss_pct.
fn apply_heuristics(
    funcs: &[perf_runner::FuncRow],
    events: &[perf_runner::EventBlock],
    excludes: &[String],
    min_pct: f64,
) -> Vec<Bottleneck> {
    let mut out: Vec<Bottleneck> = Vec::new();

    // 1. HotFunction — self_pct >= min_pct, user code only.
    let mut hot_count = 0;
    for f in funcs {
        if f.self_pct < min_pct {
            continue;
        }
        if !is_user_code(&f.symbol, excludes) {
            continue;
        }
        out.push(Bottleneck {
            kind: BottleneckKind::HotFunction,
            file: f.file.as_ref().map(std::path::PathBuf::from),
            line: f.line,
            function: f.symbol.clone(),
            metric: "self_pct".into(),
            value: f.self_pct,
            note: "self-time >= threshold".into(),
        });
        hot_count += 1;
        if hot_count >= 20 {
            break;
        }
    }

    // 2. TailFunction — user-code top self_pct that we managed to attribute.
    // A "tail" is a leaf in a hot call chain. We don't have stack data, so
    // the proxy is: top self_pct functions with a resolved user-code
    // file/line. The LLM uses this to know "the actual work is here, not
    // in a library leaf."
    let mut tail_count = 0;
    for f in funcs {
        if f.self_pct < min_pct {
            continue;
        }
        if !is_user_code(&f.symbol, excludes) {
            continue;
        }
        if f.file.is_none() || f.line.is_none() {
            continue;
        }
        // Skip if already emitted as a HotFunction for the same symbol+line.
        let dup = out.iter().any(|b| {
            b.kind == BottleneckKind::TailFunction
                && b.function == f.symbol
                && b.file.as_ref().map(std::path::PathBuf::as_path) == f.file.as_deref().map(std::path::Path::new)
                && b.line == f.line
        });
        if dup {
            continue;
        }
        out.push(Bottleneck {
            kind: BottleneckKind::TailFunction,
            file: f.file.as_ref().map(std::path::PathBuf::from),
            line: f.line,
            function: f.symbol.clone(),
            metric: "self_pct".into(),
            value: f.self_pct,
            note: "user-code leaf with file/line attribution".into(),
        });
        tail_count += 1;
        if tail_count >= 5 {
            break;
        }
    }

    // 3. Dominator — per file, the top self_pct function. The function
    // "dominates" its file's hotness. Skips files without attribution.
    let mut by_file: BTreeMap<String, &perf_runner::FuncRow> = BTreeMap::new();
    for f in funcs {
        if !is_user_code(&f.symbol, excludes) {
            continue;
        }
        let Some(file) = f.file.as_ref() else { continue };
        match by_file.get(file) {
            Some(existing) if existing.self_pct >= f.self_pct => {}
            _ => {
                by_file.insert(file.clone(), f);
            }
        }
    }
    let mut dom_entries: Vec<&perf_runner::FuncRow> = by_file.values().copied().collect();
    dom_entries.sort_by(|a, b| {
        b.self_pct
            .partial_cmp(&a.self_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for f in dom_entries.iter().take(10) {
        if f.self_pct < min_pct {
            break;
        }
        out.push(Bottleneck {
            kind: BottleneckKind::Dominator,
            file: f.file.as_ref().map(std::path::PathBuf::from),
            line: f.line,
            function: f.symbol.clone(),
            metric: "self_pct".into(),
            value: f.self_pct,
            note: "top self_pct in its source file".into(),
        });
    }

    // 4. HotFile — aggregate self_pct by file. Top files by total self_pct.
    let mut file_totals: BTreeMap<String, f64> = BTreeMap::new();
    for f in funcs {
        if !is_user_code(&f.symbol, excludes) {
            continue;
        }
        let Some(file) = f.file.as_ref() else { continue };
        *file_totals.entry(file.clone()).or_insert(0.0) += f.self_pct;
    }
    let mut file_ranked: Vec<(String, f64)> = file_totals.into_iter().collect();
    file_ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (file, total) in file_ranked.iter().take(10) {
        if *total < min_pct {
            break;
        }
        out.push(Bottleneck {
            kind: BottleneckKind::HotFile,
            file: Some(std::path::PathBuf::from(file)),
            line: None,
            function: String::new(),
            metric: "self_pct_sum".into(),
            value: *total,
            note: format!("aggregate self_pct across all functions in {}", file),
        });
    }

    // 5. MemoryPressure — when cache-misses event block is present.
    // Each entry is a top function by cache_miss_pct.
    let cache_block = events
        .iter()
        .find(|b| b.event.contains("cache-misses"));
    if let Some(b) = cache_block {
        let mut count = 0;
        for f in &b.functions {
            if f.self_pct < min_pct {
                continue;
            }
            if !is_user_code(&f.symbol, excludes) {
                continue;
            }
            out.push(Bottleneck {
                kind: BottleneckKind::MemoryPressure,
                file: f.file.as_ref().map(std::path::PathBuf::from),
                line: f.line,
                function: f.symbol.clone(),
                metric: "cache_miss_pct".into(),
                value: f.self_pct,
                note: format!(
                    "cache-misses: {} samples, ~{} misses",
                    f.samples, b.total_count_approx
                ),
            });
            count += 1;
            if count >= 10 {
                break;
            }
        }
    }

    out
}

fn workload_name(binary: &Path, args: &[String]) -> String {
    if args.is_empty() {
        binary
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "binary".into())
    } else {
        format!("{} {}", binary.display(), args.join(" "))
    }
}

fn read_kernel_release() -> String {
    std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

fn read_perf_paranoid() -> i32 {
    std::fs::read_to_string("/proc/sys/kernel/perf_event_paranoid")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(-1)
}

fn read_cpu_model() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_default()
}

fn read_build_id(binary: &Path) -> Option<String> {
    let out = std::process::Command::new("readelf")
        .args(["-n", &binary.to_string_lossy()])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    for line in s.lines() {
        if line.contains("Build ID:") {
            return line.split(':').nth(1).map(|s| s.trim().to_string());
        }
    }
    None
}
