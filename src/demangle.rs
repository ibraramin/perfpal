use anyhow::Result;
use std::process::{Command, Stdio};

/// Demangle an Itanium-ABI C++ symbol (e.g. `_ZNSt6vector...`) to its source form.
/// Also handles Rust symbols via rustc-demangle as a fast path.
///
/// Falls back to the input on any failure: better to show mangled than to fail the run.
pub fn demangle(sym: &str, demangler: Option<&std::path::Path>) -> String {
    // Rust fast path.
    if let Ok(d) = rustc_demangle::try_demangle(sym) {
        return format!("{:#}", d);
    }
    // Itanium C++ ABI symbols start with "_Z".
    if !sym.starts_with("_Z") {
        return sym.to_string();
    }
    let prog = demangler
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("c++filt"));
    let out = Command::new(&prog)
        .arg(sym)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => sym.to_string(),
    }
}

#[allow(dead_code)]
pub fn demangle_bulk(symbols: &[String], demangler: Option<&std::path::Path>) -> Result<Vec<String>> {
    if symbols.is_empty() {
        return Ok(Vec::new());
    }
    let prog = demangler
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("c++filt"));
    let mut child = Command::new(&prog)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            for s in symbols {
                writeln!(stdin, "{}", s)?;
            }
        }
    }
    let out = child.wait_with_output()?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect())
}
