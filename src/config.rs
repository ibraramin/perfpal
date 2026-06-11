use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_sample_rate")]
    pub sample_rate_hz: u64,
    #[serde(default = "default_min_self_pct")]
    pub min_self_pct: f64,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default)]
    pub memory_pressure: bool,
    #[serde(default = "default_exclude_prefixes")]
    pub exclude_prefixes: Vec<String>,
    #[serde(default = "default_report_format")]
    pub report_format: String,
}

fn default_sample_rate() -> u64 {
    999
}
fn default_min_self_pct() -> f64 {
    2.0
}
fn default_output_dir() -> String {
    "./.perfpal".into()
}
fn default_exclude_prefixes() -> Vec<String> {
    vec![
        "std::".into(),
        "__libc_".into(),
        "__gxx_personality".into(),
        "_init".into(),
        "_start".into(),
        "__libc_start_main".into(),
        "_".into(),
    ]
}
fn default_report_format() -> String {
    "json".into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sample_rate_hz: default_sample_rate(),
            min_self_pct: default_min_self_pct(),
            output_dir: default_output_dir(),
            memory_pressure: false,
            exclude_prefixes: default_exclude_prefixes(),
            report_format: default_report_format(),
        }
    }
}

impl Config {
    /// Load from `~/.perfpal.toml`. Falls back to defaults on any error or absence.
    pub fn load_or_default() -> Self {
        let Ok(home) = std::env::var("HOME") else {
            return Self::default();
        };
        let path = Path::new(&home).join(".perfpal.toml");
        let Ok(s) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        toml::from_str(&s).unwrap_or_default()
    }
}

// W1: CLI flags are the source of truth. Reading ~/.perfpal.toml is a
// W2 deliverable; the file is detected but the parser is not wired in yet.
mod toml {
    use super::Config;
    pub fn from_str(_s: &str) -> Result<Config, std::convert::Infallible> {
        Ok(Config::default())
    }
}

pub fn load() -> Result<Config> {
    Ok(Config::load_or_default())
}
