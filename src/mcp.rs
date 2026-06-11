//! MCP server: stdio JSON-RPC 2.0, exposing a single `diagnose` tool.
//!
//! Transport: `Content-Length: N\r\n\r\n<body>` framing (per MCP spec).
//! All logging goes to stderr. stdout is reserved for JSON-RPC frames.
//!
//! Tools:
//!   - `diagnose` — profile a C/C++ binary and return a markdown summary plus
//!     the on-disk report path. The full report is `report-<uuid>.json` and
//!     the LLM can `read_file` it for the structured payload.

use crate::cli::DiagnoseArgs;
use crate::diagnose;
use crate::report::{Bottleneck, BottleneckKind, Hotspot, Report};
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "perfpal";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run the MCP server. Reads JSON-RPC 2.0 messages from stdin and writes
/// responses to stdout. Blocks until stdin EOF.
pub fn run() -> Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    loop {
        let msg = match read_framed_message(&mut reader) {
            Ok(Some(m)) => m,
            Ok(None) => break, // clean EOF
            Err(e) => {
                eprintln!("perfpal-mcp: read error: {e}");
                break;
            }
        };
        let response = handle_request(&msg);
        if let Some(resp) = response {
            write_framed_message(&mut stdout, &resp)?;
        }
    }
    Ok(())
}

/// Read one MCP-framed message. Returns Ok(None) on clean EOF.
fn read_framed_message<R: BufRead>(reader: &mut R) -> Result<Option<Value>> {
    let mut content_length: usize = 0;
    let mut header = String::new();
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().unwrap_or(0);
        }
        // Ignore Content-Type and other headers.
        let _ = header.clear();
    }
    if content_length == 0 {
        // Empty body; spec says server SHOULD tolerate it.
        return Ok(Some(Value::Null));
    }
    let mut buf = vec![0u8; content_length];
    reader.read_exact(&mut buf)?;
    let v: Value = serde_json::from_slice(&buf)?;
    Ok(Some(v))
}

/// Write one MCP-framed message to stdout.
fn write_framed_message<W: Write>(w: &mut W, msg: &Value) -> Result<()> {
    let body = serde_json::to_vec(msg)?;
    write!(w, "Content-Length: {}\r\n\r\n", body.len())?;
    w.write_all(&body)?;
    w.flush()?;
    Ok(())
}

fn handle_request(msg: &Value) -> Option<Value> {
    let obj = msg.as_object()?;
    let method = obj.get("method")?.as_str()?;
    let id = obj.get("id").cloned();
    let params = obj.get("params").cloned().unwrap_or(Value::Null);

    match method {
        "initialize" => Some(ok(id, initialize_result())),
        "notifications/initialized" => None, // notification, no response
        "ping" => Some(ok(id, json!({}))),
        "tools/list" => Some(ok(id, json!({ "tools": tools_list() }))),
        "tools/call" => match tools_call(&params) {
            Ok(content) => Some(ok(id, json!({ "content": content, "isError": false }))),
            Err(e) => {
                let msg = format!("{e:#}");
                eprintln!("perfpal-mcp: tools/call failed: {msg}");
                Some(ok(
                    id,
                    json!({
                        "content": [{"type": "text", "text": format!("error: {msg}")}],
                        "isError": true,
                    }),
                ))
            }
        },
        // Spec: server MUST respond with method-not-found for unknown methods.
        other => Some(err(
            id,
            -32601,
            format!("method not found: {other}"),
        )),
    }
}

fn ok(id: Option<Value>, result: Value) -> Value {
    match id {
        Some(id) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        None => result, // shouldn't happen for ok()-wrapped responses
    }
}

fn err(id: Option<Value>, code: i32, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
        "capabilities": { "tools": {} },
        "instructions":
            "perfpal: profile a C/C++ binary with Linux perf and report bottlenecks. \
             Use the 'diagnose' tool with the path to a built binary. The tool returns \
             a markdown summary and the on-disk report path; read the JSON report for \
             structured hotspots, stacks, and bottlenecks."
                .to_string(),
    })
}

fn tools_list() -> Vec<Value> {
    vec![json!({
        "name": "diagnose",
        "description": "Profile a C/C++ binary under Linux perf, demangle symbols, attribute to file:line, and write a bottleneck report. Returns a markdown summary of hotspots and bottlenecks plus the path to the structured report.json on disk.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "binary": {
                    "type": "string",
                    "description": "Absolute path to the binary to profile."
                },
                "args": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Args to forward to the binary."
                },
                "duration": {
                    "type": "integer",
                    "description": "Profile duration cap in seconds. Default 10. Ignored if the binary exits sooner.",
                    "default": 10
                },
                "sample_rate": {
                    "type": "integer",
                    "description": "Sampling frequency in Hz. Default 999.",
                    "default": 999
                },
                "min_self_pct": {
                    "type": "number",
                    "description": "Minimum self-time % for a function to be flagged. Default 2.0.",
                    "default": 2.0
                },
                "memory_pressure": {
                    "type": "boolean",
                    "description": "Opt-in: record a second event (cache-misses) to enable the MemoryPressure heuristic.",
                    "default": false
                },
                "output_dir": {
                    "type": "string",
                    "description": "Directory where report.json is written. Default './.perfpal'."
                }
            },
            "required": ["binary"],
            "additionalProperties": false
        }
    })]
}

fn tools_call(params: &Value) -> Result<Vec<Value>> {
    let obj = params
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("params must be an object"))?;
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("params.name is required"))?;
    if name != "diagnose" {
        anyhow::bail!("unknown tool: {name}");
    }
    let arguments = obj
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let binary = require_str(&arguments, "binary")?;
    let args = arguments
        .get("args")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .map(|x| x.as_str().unwrap_or("").to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let duration = arguments
        .get("duration")
        .and_then(|v| v.as_u64())
        .unwrap_or(10);
    let sample_rate = arguments
        .get("sample_rate")
        .and_then(|v| v.as_u64())
        .unwrap_or(999);
    let min_self_pct = arguments
        .get("min_self_pct")
        .and_then(|v| v.as_f64())
        .unwrap_or(2.0);
    let memory_pressure = arguments
        .get("memory_pressure")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let output_dir = arguments
        .get("output_dir")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./.perfpal"));

    let diagnose_args = DiagnoseArgs {
        binary: PathBuf::from(binary),
        args,
        duration,
        sample_rate,
        min_self_pct,
        output_dir,
        exclude_prefixes: vec![
            "std::".into(),
            "__libc_".into(),
            "__gxx_personality".into(),
            "_init".into(),
            "_start".into(),
            "__libc_start_main".into(),
            "_".into(),
        ],
        demangler: None,
        memory_pressure,
    };

    let (report, path) = diagnose::run_report(diagnose_args)?;
    let summary = render_summary_markdown(&report, &path);
    Ok(vec![json!({
        "type": "text",
        "text": summary,
    })])
}

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing required argument: {key}"))
}

fn render_summary_markdown(report: &Report, path: &PathBuf) -> String {
    use std::fmt::Write as _;
    let mut s = String::new();
    let _ = writeln!(s, "# perfpal diagnose — {}", report.binary.path.display());
    let _ = writeln!(s, "");
    let _ = writeln!(s, "- **session**: {}", report.session_id);
    let _ = writeln!(s, "- **build_id**: {}", report.binary.build_id.as_deref().unwrap_or("?"));
    let _ = writeln!(s, "- **kernel**: {}", report.environment.kernel);
    let _ = writeln!(s, "- **perf**: {}", report.environment.perf_version);
    let _ = writeln!(
        s,
        "- **samples**: {} (counters: {})",
        report.sampling.samples,
        if report.sampling.counters.is_empty() {
            "?".to_string()
        } else {
            report.sampling.counters.join(", ")
        }
    );
    let _ = writeln!(s, "- **duration_s**: {:.2}", report.sampling.duration_s);
    let _ = writeln!(s, "");

    let _ = writeln!(s, "## Top 10 hotspots (self-time)");
    for h in report.hotspots.iter().take(10) {
        let loc = match (&h.file, h.line) {
            (Some(f), Some(l)) => format!("`{}:{}`", f.display(), l),
            _ => "<no location>".to_string(),
        };
        let _ = writeln!(
            s,
            "  {}. **{:.2}%** `{}` ({}) — {} samples",
            h.rank, h.self_pct, h.function, loc, h.samples
        );
    }
    let _ = writeln!(s, "");

    if !report.bottlenecks.is_empty() {
        let _ = writeln!(s, "## Bottlenecks ({} total)", report.bottlenecks.len());
        let _ = writeln!(s, "");
        let _ = writeln!(s, "| kind | function | location | metric | value | note |");
        let _ = writeln!(s, "|------|----------|----------|--------|-------|------|");
        for b in &report.bottlenecks {
            let loc = match (&b.file, b.line) {
                (Some(f), Some(l)) => format!("`{}:{}`", f.display(), l),
                _ => "—".to_string(),
            };
            let func = if b.function.is_empty() { "—" } else { b.function.as_str() };
            let kind = bottleneck_kind_name(&b.kind);
            let note = b.note.replace('|', "\\|");
            let _ = writeln!(
                s,
                "| {kind} | `{func}` | {loc} | {} | {:.2} | {note} |",
                b.metric, b.value
            );
        }
        let _ = writeln!(s, "");
    }

    let _ = writeln!(s, "## Full report");
    let _ = writeln!(s, "Structured JSON written to: `{}`", path.display());
    let _ = writeln!(
        s,
        "Use your file-read tool to load it for the complete hotspots/stacks/bottlenecks payload (schema_version {}).",
        report.schema_version
    );

    s
}

fn bottleneck_kind_name(k: &BottleneckKind) -> &'static str {
    match k {
        BottleneckKind::HotFunction => "hot_function",
        BottleneckKind::TailFunction => "tail_function",
        BottleneckKind::Dominator => "dominator",
        BottleneckKind::HotFile => "hot_file",
        BottleneckKind::MemoryPressure => "memory_pressure",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framing_roundtrip() {
        let payload = json!({"jsonrpc": "2.0", "id": 1, "method": "ping"});
        let mut buf: Vec<u8> = Vec::new();
        write_framed_message(&mut buf, &payload).unwrap();
        let mut cur = std::io::Cursor::new(buf);
        let read = read_framed_message(&mut std::io::BufReader::new(&mut cur))
            .unwrap()
            .unwrap();
        assert_eq!(read, payload);
    }

    #[test]
    fn test_initialize_result_shape() {
        let v = initialize_result();
        assert_eq!(v["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(v["serverInfo"]["name"], SERVER_NAME);
        assert!(v["capabilities"]["tools"].is_object());
    }

    #[test]
    fn test_tools_list_has_diagnose() {
        let tools = tools_list();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "diagnose");
    }

    #[test]
    fn test_handle_unknown_method() {
        let req = json!({"jsonrpc": "2.0", "id": 1, "method": "nope"});
        let resp = handle_request(&req).unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn test_handle_ping() {
        let req = json!({"jsonrpc": "2.0", "id": 7, "method": "ping"});
        let resp = handle_request(&req).unwrap();
        assert_eq!(resp["id"], 7);
        assert!(resp["result"].is_object());
    }

    #[test]
    fn test_initialized_is_notification() {
        let req = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
        let resp = handle_request(&req);
        assert!(resp.is_none());
    }

    #[test]
    fn test_tools_call_diagnose_shape() {
        // Just the argument-parsing path. Don't actually run perf in unit tests.
        let req = json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "unknown_tool"
            }
        });
        let resp = handle_request(&req).unwrap();
        assert_eq!(resp["id"], 9);
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn test_summary_renders_path() {
        // Build a minimal Report and ensure the path appears in the markdown.
        let report = Report {
            schema_version: 1,
            session_id: "test".into(),
            binary: crate::report::Binary {
                path: PathBuf::from("/tmp/foo"),
                build_id: Some("deadbeef".into()),
                demangled: true,
            },
            workload: crate::report::Workload {
                name: "foo".into(),
                args: vec![],
                duration_s: 1.0,
            },
            environment: crate::report::Environment {
                kernel: "test".into(),
                perf_paranoid: 2,
                cpu: "test".into(),
                perf_version: "test".into(),
            },
            sampling: crate::report::Sampling {
                rate_hz: 999,
                samples: 100,
                duration_s: 1.0,
                counters: vec!["cycles:u".into()],
            },
            hotspots: vec![Hotspot {
                rank: 1,
                function: "hot".into(),
                file: Some(PathBuf::from("/tmp/foo.cpp")),
                line: Some(10),
                self_pct: 90.0,
                cumulative_pct: 90.0,
                samples: 90,
            }],
            stacks: vec![],
            bottlenecks: vec![],
        };
        let md = render_summary_markdown(&report, &PathBuf::from("/tmp/foo/report.json"));
        assert!(md.contains("/tmp/foo/report.json"));
        assert!(md.contains("hot"));
    }
}
