use clap::{Parser, Subcommand};

mod cli;
mod config;
mod demangle;
mod diagnose;
mod perf_runner;
mod report;
mod stacks;

use anyhow::Result;

#[derive(Parser, Debug)]
#[command(name = "perfpald", about = "Diagnose C/C++ performance bottlenecks", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run a workload under perf and write a diagnostic report.
    Diagnose(cli::DiagnoseArgs),
    /// Print an MCP server config snippet for Claude Code / opencode.
    McpPrintConfig,
    /// Run the MCP server on stdio (placeholder; implemented in W3).
    Mcp,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Diagnose(a) => diagnose::run(a),
        Cmd::McpPrintConfig => mcp_print_config(),
        Cmd::Mcp => {
            eprintln!("MCP server not implemented yet (W3 deliverable).");
            std::process::exit(1);
        }
    }
}

fn mcp_print_config() -> Result<()> {
    let snippet = serde_json::json!({
        "mcpServers": {
            "perfpal": {
                "command": "perfpald",
                "args": ["mcp"]
            }
        }
    });
    println!("{}", serde_json::to_string_pretty(&snippet)?);
    Ok(())
}
