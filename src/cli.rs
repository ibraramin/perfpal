use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct DiagnoseArgs {
    /// The binary to profile. Anything after `--` is forwarded as its argv.
    #[arg(long, value_name = "PATH")]
    pub binary: PathBuf,

    /// Args to pass to the binary.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,

    /// Profile duration in seconds (ignored if the binary exits sooner).
    #[arg(long, default_value_t = 10)]
    pub duration: u64,

    /// Sampling frequency in Hz.
    #[arg(long, default_value_t = 999)]
    pub sample_rate: u64,

    /// Minimum self-time % for a function to be considered a bottleneck.
    #[arg(long, default_value_t = 2.0)]
    pub min_self_pct: f64,

    /// Output directory for the report.
    #[arg(long, default_value = "./.perfpal")]
    pub output_dir: PathBuf,

    /// Path prefix to exclude (comma-separated). Default: std::,__libc_,...
    #[arg(long, value_delimiter = ',', default_value = "std::,__libc_,__gxx_personality,_init,_start,__libc_start_main,_")]
    pub exclude_prefixes: Vec<String>,

    /// Path to a C++ demangler binary. Default: c++filt from PATH.
    #[arg(long)]
    pub demangler: Option<PathBuf>,

    /// Record a second event for memory-pressure analysis (cycles,cache-misses).
    /// Doubles the per-sample cost and enables heuristic 5.
    #[arg(long, default_value_t = false)]
    pub memory_pressure: bool,
}
