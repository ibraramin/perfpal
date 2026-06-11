use crate::demangle;
use crate::perf_runner;
use crate::report::{
    Binary, Bottleneck, BottleneckKind, Environment, Hotspot, Report, Sampling, WeightedStack,
    Workload, SCHEMA_VERSION,
};
use crate::stacks;
use anyhow::Result;
use std::path::Path;
use uuid::Uuid;

pub fn run(args: crate::cli::DiagnoseArgs) -> Result<()> {
    eprintln!(
        "perfpal: profiling {} ({} sample/s) for up to {}s",
        args.binary.display(),
        args.sample_rate,
        args.duration
    );

    let raw = perf_runner::record_and_parse(
        &args.binary,
        &args.args,
        args.sample_rate,
        args.duration,
    )?;
    eprintln!(
        "perfpal: captured {} functions, total samples={} in {:.2}s",
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
        counters: Vec::new(),
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

    let report = Report {
        schema_version: SCHEMA_VERSION,
        session_id: Uuid::new_v4().to_string(),
        binary,
        workload,
        environment: env,
        sampling,
        hotspots,
        stacks,
        bottlenecks: apply_heuristics_w1(&funcs, &args.exclude_prefixes, args.min_self_pct),
    };

    let path = report.write_to(&args.output_dir)?;
    eprintln!("perfpal: wrote {}", path.display());

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
        println!(
            "\nBottlenecks (heuristic 1: self-time >= {:.2}%):",
            args.min_self_pct
        );
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
    Ok(())
}

fn apply_heuristics_w1(
    funcs: &[perf_runner::FuncRow],
    excludes: &[String],
    min_pct: f64,
) -> Vec<Bottleneck> {
    // W1: only heuristic 1 (self-time threshold).
    let mut out = Vec::new();
    for f in funcs {
        if f.self_pct < min_pct {
            continue;
        }
        if excludes.iter().any(|p| f.symbol.starts_with(p)) {
            continue;
        }
        out.push(Bottleneck {
            kind: BottleneckKind::HotFunction,
            file: f.file.as_ref().map(std::path::PathBuf::from),
            line: f.line,
            function: f.symbol.clone(),
            metric: "self_pct".into(),
            value: f.self_pct,
            note: "largest self-time in workload".into(),
        });
        if out.len() >= 20 {
            break;
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
