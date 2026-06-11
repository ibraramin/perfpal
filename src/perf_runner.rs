use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// Output of `perf report --stdio` parsed into a list of functions with self-pct.
#[derive(Debug, Clone)]
pub struct FuncRow {
    pub self_pct: f64,
    pub symbol: String,
    pub dso: String,
    /// Optional resolved location from `addr2line`. W1 best-effort.
    pub file: Option<String>,
    pub line: Option<u32>,
    /// Per-event sample count (from `perf report` with `-n`). Always set for
    /// cycles and cache-misses blocks in W2.
    pub samples: u64,
}

/// One event's function table from `perf report --stdio`.
///
/// `perf report` with grouped events (e.g. `cycles,cache-misses`) emits one
/// block per event. Each block has its own sample count and a function
/// table. We capture both so the heuristics can pick the right one:
/// heuristics 1-4 use the cycles block, heuristic 5 uses the cache-misses
/// block.
#[derive(Debug, Clone)]
pub struct EventBlock {
    pub event: String,
    pub total_samples: u64,
    pub total_count_approx: u64,
    pub functions: Vec<FuncRow>,
}

pub struct RawProfile {
    pub _data_dir: TempDir,
    /// First (primary) event block — typically cycles:u. Used by heuristics 1-4.
    pub functions: Vec<FuncRow>,
    /// Per-event blocks. The first entry is the primary cycles block; further
    /// entries are opt-in extras (cache-misses in W2).
    pub events: Vec<EventBlock>,
    pub total_samples: u64,
    pub duration_s: f64,
    pub perf_version: String,
}

/// Run the workload under `perf record`, then read the function-level table
/// out of `perf report --stdio`. This avoids the `perf script` symbol-resolution
/// path, which is unreliable on some perf/kernel combos.
///
/// W1 caveat: we get *function-level* self-pct, not per-sample stacks.
/// Stack-level heuristics (tail, dominator) are W2 deliverables and may
/// require us to revisit the data source.
pub fn record_and_parse(
    binary: &Path,
    args: &[String],
    sample_rate_hz: u64,
    duration_s: u64,
    event_spec: &str,
) -> Result<RawProfile> {
    anyhow::ensure!(binary.exists(), "binary does not exist: {}", binary.display());
    anyhow::ensure!(
        which("perf").is_some(),
        "`perf` not found on PATH. Install linux-tools."
    );

    let tmp = TempDir::new().context("create tempdir for perf data")?;
    let data_path = tmp.path().join("perf.data");

    // Step 1: perf record with user-space cycles (or grouped events).
    let mut child = Command::new("perf")
        .args([
            "record",
            "-F",
            &sample_rate_hz.to_string(),
            "-g",
            "-e",
        ])
        .arg(event_spec)
        .args(["--output"])
        .arg(&data_path)
        .arg("--")
        .arg(binary)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn `perf record`")?;

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(duration_s);
    let status = loop {
        if let Some(s) = child.try_wait().context("try_wait perf record")? {
            break s;
        }
        if start.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            break child.wait().context("wait perf record after kill")?;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    };
    let elapsed = start.elapsed().as_secs_f64();

    if !data_path.exists() && !status.success() {
        let mut stderr = String::new();
        if let Some(mut s) = child.stderr.take() {
            use std::io::Read;
            let _ = s.read_to_string(&mut stderr);
        }
        anyhow::bail!("`perf record` failed: {stderr}");
    }

    // Step 2: perf report --stdio -n (no children) -g none (no call graph dump) for parseability.
    let output = Command::new("perf")
        .args([
            "report",
            "-i",
        ])
        .arg(&data_path)
        .args([
            "--stdio",
            "-n",
            "--no-children",
            "-g",
            "none",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("run `perf report`")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("`perf report` failed: {stderr}");
    }
    let report = String::from_utf8_lossy(&output.stdout);

    let (mut events, primary_samples) = parse_perf_report_multi(&report);
    // Enrich every event block with file/line so all heuristics can use it.
    for b in events.iter_mut() {
        b.functions = enrich_with_addr2line(binary, std::mem::take(&mut b.functions));
    }
    let primary = events.first().map(|e| e.functions.clone()).unwrap_or_default();

    let perf_version = Command::new("perf")
        .arg("--version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    Ok(RawProfile {
        _data_dir: tmp,
        functions: primary,
        events,
        total_samples: primary_samples,
        duration_s: elapsed,
        perf_version,
    })
}

/// Parse `perf report --stdio -n --no-children -g none` output.
///
/// `perf report` with grouped events emits one function table per event.
/// Each table is preceded by a header like:
///     # Samples: 196  of event 'cycles:u'
///     # Event count (approx.): 576723222
///
/// We return one `EventBlock` per event. The first event is treated as
/// "primary" (typically cycles:u) and its total sample count is the
/// canonical total for the run.
///
/// Each data row looks like:
///     50.00%      81  hot_loop  hot_loop  [.] hot_loop(std::vector<int, std::allocator<int> >&) [clone .isra.0]
///
/// - Column 1: self-pct
/// - Column 2: sample count (only with `-n` — we always pass `-n`)
/// - Column 3: comm (process name)
/// - Column 4: shared object (DSO)
/// - Column 5: `[k]` for kernel, `[.]` for user, `[u]` for unknown; rest is the symbol
pub fn parse_perf_report_multi(s: &str) -> (Vec<EventBlock>, u64) {
    let mut blocks: Vec<EventBlock> = Vec::new();
    let mut current: Option<EventBlock> = None;
    let mut primary_total: u64 = 0;

    for line in s.lines() {
        // Event header detection: "# Samples: NNN  of event 'NAME'"
        // Also capture the "Event count (approx.):" line if it follows.
        if let Some(rest) = line.strip_prefix("# Samples:") {
            // Flush previous block.
            if let Some(b) = current.take() {
                primary_total = primary_total.max(b.total_samples);
                blocks.push(b);
            }
            let mut block = EventBlock {
                event: String::new(),
                total_samples: 0,
                total_count_approx: 0,
                functions: Vec::new(),
            };
            // Parse "Samples: N  of event 'NAME'"
            let after = rest.trim_start();
            let mut parts = after.split_whitespace();
            if let Some(n_str) = parts.next() {
                block.total_samples = n_str.parse().unwrap_or(0);
            }
            // Find "of event '...'"
            if let Some(idx) = after.find("of event '") {
                let tail = &after[idx + "of event '".len()..];
                if let Some(end) = tail.find('\'') {
                    block.event = tail[..end].to_string();
                }
            }
            current = Some(block);
            continue;
        }
        if let Some(rest) = line.strip_prefix("# Event count (approx.):") {
            if let Some(b) = current.as_mut() {
                if let Some(n) = rest.split_whitespace().next().and_then(|x| x.parse().ok()) {
                    b.total_count_approx = n;
                }
            }
            continue;
        }
        // Skip blanks and comments.
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        // Data row.
        if current.is_none() {
            // No event header seen yet — start a synthetic one so we still
            // capture rows from perf variants that omit the header.
            current = Some(EventBlock {
                event: "cycles:u".to_string(),
                total_samples: 0,
                total_count_approx: 0,
                functions: Vec::new(),
            });
        }
        // Parse the data line: "PERCENT  SAMPLES  COMM  DSO  [k|u|.]  SYMBOL..."
        // (With -n, second column is the per-function sample count.)
        let mut parts = line.split_whitespace();
        let pct_str = match parts.next() {
            Some(p) => p,
            None => continue,
        };
        if !pct_str.ends_with('%') {
            continue;
        }
        let pct: f64 = match pct_str.trim_end_matches('%').parse() {
            Ok(n) => n,
            Err(_) => continue,
        };
        // Sample count column.
        let samples = parts
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let comm = parts.next().unwrap_or("");
        let dso = parts.next().unwrap_or("");
        let bracket = parts.next().unwrap_or("");
        if !bracket.starts_with('[') {
            continue;
        }
        let symbol = parts.collect::<Vec<_>>().join(" ");
        if symbol.is_empty() {
            continue;
        }
        let _ = comm;
        if let Some(b) = current.as_mut() {
            b.functions.push(FuncRow {
                self_pct: pct,
                symbol,
                dso: dso.to_string(),
                file: None,
                line: None,
                samples,
            });
        }
    }
    if let Some(b) = current.take() {
        primary_total = primary_total.max(b.total_samples);
        blocks.push(b);
    }
    for b in blocks.iter_mut() {
        b.functions.sort_by(|a, c| {
            c.self_pct
                .partial_cmp(&a.self_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.symbol.cmp(&c.symbol))
        });
    }
    (blocks, primary_total)
}

/// For each function, attempt to find its file/line by:
/// 1. Reading the symbol table with `nm` to get the function's address.
/// 2. Asking `addr2line` for the file/line at that address.
///
/// Best-effort: if `nm` or `addr2line` is missing, or the binary is stripped,
/// we silently leave file/line as None. The LLM still gets a useful symbol name.
fn enrich_with_addr2line(binary: &Path, mut funcs: Vec<FuncRow>) -> Vec<FuncRow> {
    if funcs.is_empty() {
        return funcs;
    }
    // For each top function, ask addr2line at a representative address and
    // pick the first user-code frame.
    let mut cache: std::collections::HashMap<String, (Option<String>, Option<u32>)> =
        std::collections::HashMap::new();

    for f in funcs.iter_mut() {
        if f.dso != binary
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default()
        {
            // DSO doesn't match the binary itself (e.g. it's a shared lib or [kernel]).
            // Skip enrichment.
            continue;
        }
        // perf prints symbols with an optional `+0xN` offset suffix. Use it
        // directly if present; otherwise sample several addresses across the
        // function body and pick the first one that resolves to user code.
        let (sym_name, offset) = parse_symbol_with_offset(&f.symbol);
        let (base, size) = find_function_with_size(binary, &sym_name);
        let Some(base) = base else { continue };
        let key = if offset != 0 {
            format!("{:x}", base.wrapping_add(offset))
        } else {
            // No offset: try a few points across the function body.
            let step = (size.unwrap_or(64) / 8).max(1);
            let mut target = None;
            for i in 0..8 {
                let addr = base.wrapping_add(step * i);
                if let Some((file, _)) = addr2line_user_frame(binary, addr) {
                    if let Some(ref f_path) = file {
                        if !is_system_header(f_path) {
                            target = Some(addr);
                            break;
                        }
                    }
                }
            }
            match target {
                Some(a) => format!("{:x}", a),
                None => format!("{:x}", base),
            }
        };
        if let std::collections::hash_map::Entry::Vacant(e) = cache.entry(key.clone()) {
            let result = addr2line_user_frame(binary, u64::from_str_radix(&key, 16).unwrap_or(base))
                .unwrap_or((None, None));
            e.insert(result);
        }
        if let Some((file, line)) = cache.get(&key) {
            f.file = file.clone();
            f.line = *line;
        }
    }
    funcs
}

/// Parse "name+0x24" -> ("name", 0x24). If no offset, returns (name, 0).
fn parse_symbol_with_offset(sym: &str) -> (String, u64) {
    if let Some(idx) = sym.rfind('+') {
        let (name, tail) = sym.split_at(idx);
        let tail = &tail[1..]; // skip '+'
        if let Some(hex) = tail.strip_prefix("0x").or(Some(tail)) {
            if let Ok(n) = u64::from_str_radix(hex, 16) {
                return (name.trim().to_string(), n);
            }
        }
    }
    (sym.trim().to_string(), 0)
}

fn find_function_with_size(binary: &Path, symbol: &str) -> (Option<u64>, Option<u64>) {
    // `nm -C --size-sort -S` would sort by size; we use `nm -C --defined-only`
    // and a substring match to find the function and its size.
    let out = match Command::new("nm")
        .args(["-C", "-S", "--defined-only"])
        .arg(binary)
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return (None, None),
    };
    let s = String::from_utf8_lossy(&out.stdout);
    // With -S: "<addr> <size> <type> <name>". Without -S (older binutils):
    // "<addr> <type> <size> <name>". Probe both formats.
    let needle = symbol.split('+').next().unwrap_or(symbol).trim();
    for line in s.lines() {
        let mut parts = line.split_whitespace();
        let addr_s = match parts.next() {
            Some(a) => a,
            None => continue,
        };
        let second = match parts.next() {
            Some(s) => s,
            None => continue,
        };
        // Try format "<addr> <size> <type> <name>"
        if let (Ok(addr), Ok(size)) = (u64::from_str_radix(addr_s, 16), u64::from_str_radix(second, 16)) {
            let type_or_name = parts.next().unwrap_or("");
            let name: String = parts.collect::<Vec<_>>().join(" ");
            // If type_or_name is a single char (T/W/etc), use name; else the
            // second field was the type, not the size.
            let real_name = if type_or_name.len() == 1 {
                name
            } else {
                format!("{} {}", type_or_name, name)
            };
            if real_name == needle
                || real_name.starts_with(needle)
                || (needle.len() >= 16 && real_name.contains(&needle[..needle.len().min(40)]))
            {
                return (Some(addr), Some(size));
            }
        }
    }
    (None, None)
}

fn find_function_addr(binary: &Path, symbol: &str) -> Option<u64> {
    find_function_with_size(binary, symbol).0
}

fn addr2line_user_frame(binary: &Path, addr: u64) -> Option<(Option<String>, Option<u32>)> {
    // `addr2line -f -e <bin> <addr>` emits, per inline frame:
    //   <function-name>
    //   <file>:<line>
    // in caller -> callee order. With `-O2` and header-inlined code, the
    // first frame may be the user function and later frames may be header
    // stdlib internals. We want the FIRST frame in the user's project, not
    // a system header.
    let out = Command::new("addr2line")
        .args(["-f", "-e"])
        .arg(binary)
        .arg(format!("0x{:x}", addr))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let mut lines = s.lines();
    let mut user_frame: Option<(Option<String>, Option<u32>)> = None;
    let mut any_frame: Option<(Option<String>, Option<u32>)> = None;
    loop {
        let Some(_func) = lines.next() else { break };
        let Some(loc) = lines.next() else { break };
        let (file, line_no) = parse_file_line(loc.trim());
        if file.is_none() && line_no.is_none() {
            continue;
        }
        any_frame = Some((file.clone(), line_no));
        if let Some(ref f) = file {
            if !is_system_header(f) && user_frame.is_none() {
                user_frame = Some((file, line_no));
            }
        }
    }
    Some(user_frame.or(any_frame).unwrap_or((None, None)))
}

fn is_system_header(path: &str) -> bool {
    path.starts_with("/usr/include")
        || path.starts_with("/usr/lib/gcc")
        || path.starts_with("/usr/lib/clang")
        || path.starts_with("/opt/")
        || path.starts_with("/include/")
        || path.contains("/bits/") && path.contains("/c++/")
}

fn parse_file_line(s: &str) -> (Option<String>, Option<u32>) {
    if s.is_empty() || s == "??" || s.starts_with("??:") {
        return (None, None);
    }
    if let Some(colon) = s.rfind(':') {
        let (file_part, line_part) = s.split_at(colon);
        // addr2line appends " (discriminator N)" to repeated line numbers.
        // Strip it before parsing.
        let line_part = line_part[1..]
            .split_whitespace()
            .next()
            .unwrap_or("");
        if let Ok(n) = line_part.parse::<u32>() {
            let file = if file_part.is_empty() || file_part == "?" {
                None
            } else {
                Some(file_part.to_string())
            };
            return (file, Some(n));
        }
    }
    (None, None)
}

fn which(cmd: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(cmd);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
